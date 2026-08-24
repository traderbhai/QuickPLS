[CmdletBinding()]
param(
    [switch]$Execute,
    [switch]$Resume,
    [switch]$SchedulerSelfTest,
    [ValidateSet("inventory", "confirmation", "targeted")][string]$CampaignPass = "inventory",
    [string]$PriorIssueInventory = "",
    [string]$CandidateCommit = "",
    [string]$MainRepositoryRoot = "D:\QuickPLS",
    [string]$ExpectedMainCommit = "6ed46cc422917fc9fc9c463302ca4ff1e9ea01a4",
    [string]$OutputRoot = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$planPath = Join-Path $PSScriptRoot "multimod\v256_multimod_qualification_plan_v1.json"
$bindingPath = Join-Path $PSScriptRoot "multimod\multimod_gate_bindings_v1.json"
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$bindings = Get-Content -LiteralPath $bindingPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$planSha256 = (Get-FileHash -LiteralPath $planPath -Algorithm SHA256).Hash.ToLowerInvariant()
$bindingSha256 = (Get-FileHash -LiteralPath $bindingPath -Algorithm SHA256).Hash.ToLowerInvariant()
$dependencyVerifierBasename = "verify_multimod_gate_dependency_v1.py"
$evidenceBarrierGateIds = @(
    "performance.maximum_profiles",
    "manifests.prepackage.authority",
    "package.candidate",
    "manifests.live.derivation",
    "release.acceptance"
)
$runtimeBarrierGateIds = @(
    "performance.maximum_profiles",
    "manifests.prepackage.authority",
    "package.candidate"
)
$targetedDiagnosticBlockedGateIds = @(
    "performance.maximum_profiles",
    "manifests.prepackage.authority",
    "package.candidate",
    "installed.offline.smoke",
    "portable.offline.smoke",
    "manifests.live.derivation",
    "release.acceptance"
)
$artifactProducerDependencies = @(
    [pscustomobject]@{ producer = "manifests.prepackage.authority"; consumer = "package.candidate" },
    [pscustomobject]@{ producer = "package.candidate"; consumer = "installed.offline.smoke" },
    [pscustomobject]@{ producer = "package.candidate"; consumer = "portable.offline.smoke" }
)

function Invoke-GitText {
    param([string]$WorkingDirectory, [string[]]$Arguments)
    $output = & git -C $WorkingDirectory @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "git $($Arguments -join ' ') failed in $WorkingDirectory`: $($output -join [Environment]::NewLine)" }
    return ($output -join "`n").Trim()
}

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-TextSha256([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([Convert]::ToHexString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text)))).ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Get-FreeGiB([string]$DriveName) {
    return [math]::Round(((Get-PSDrive -Name $DriveName -ErrorAction Stop).Free / 1GB), 2)
}

function Assert-DiskThresholds {
    $freeC = Get-FreeGiB "C"
    $freeD = Get-FreeGiB "D"
    if ($freeC -lt [double]$plan.disk_thresholds_gib.C -or $freeD -lt [double]$plan.disk_thresholds_gib.D) {
        throw "Disk guard failed. C: $freeC GiB (minimum $($plan.disk_thresholds_gib.C)); D: $freeD GiB (minimum $($plan.disk_thresholds_gib.D)). Inventory safe cleanup candidates and obtain approval before deleting evidence or shared build output."
    }
    return [pscustomobject]@{ C = $freeC; D = $freeD }
}

function Assert-CandidateUnchanged([string]$ExpectedCandidate) {
    $candidateHead = Invoke-GitText $repositoryRoot @("rev-parse", "HEAD")
    $candidateStatus = Invoke-GitText $repositoryRoot @("status", "--porcelain=v1", "--untracked-files=all")
    $mainHead = Invoke-GitText $MainRepositoryRoot @("rev-parse", "HEAD")
    $mainStatus = Invoke-GitText $MainRepositoryRoot @("status", "--porcelain=v1", "--untracked-files=all")
    if ($candidateHead -cne $ExpectedCandidate -or $candidateStatus) { throw "Candidate source changed while the no-fix campaign was running." }
    if ($mainHead -cne $ExpectedMainCommit -or $mainStatus) { throw "Main changed while the candidate campaign was running." }
}

function Assert-CampaignPreflight {
    $resolvedRepository = (Resolve-Path -LiteralPath $repositoryRoot).Path
    if ((Split-Path -Leaf $resolvedRepository) -cne $plan.candidate.worktree_leaf) { throw "Campaign must run from $($plan.candidate.worktree_leaf)." }
    if ((Invoke-GitText $resolvedRepository @("branch", "--show-current")) -cne $plan.candidate.branch) { throw "Candidate branch differs from $($plan.candidate.branch)." }
    $head = Invoke-GitText $resolvedRepository @("rev-parse", "HEAD")
    if (-not $script:CandidateCommit) { $script:CandidateCommit = $head }
    if ($CandidateCommit -cnotmatch '^[a-f0-9]{40}$' -or $CandidateCommit -cne $head) { throw "CandidateCommit must equal exact lowercase HEAD $head." }
    $null = Resolve-Path -LiteralPath $MainRepositoryRoot
    Assert-CandidateUnchanged $head
    if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -gt 0) { throw "Another Cargo process is active." }
    $disk = Assert-DiskThresholds
    return [pscustomobject]@{ candidate_commit_sha = $head; main_commit_sha = $ExpectedMainCommit; free_gib = $disk }
}

function Write-JsonAtomic([string]$Path, $Value) {
    $temporary = "$Path.tmp"
    [IO.File]::WriteAllText($temporary, (($Value | ConvertTo-Json -Depth 100) + "`n"), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Relative-EvidencePath([string]$CampaignRoot, [string]$Path) {
    $root = [IO.Path]::GetFullPath($CampaignRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw "Evidence path escapes campaign root: $resolved" }
    return $resolved.Substring($root.Length).Replace("\", "/")
}

function Resolve-CampaignEvidencePath([string]$CampaignRoot, [string]$Path) {
    if (-not $Path) { throw "Campaign evidence path is empty." }
    $root = [IO.Path]::GetFullPath($CampaignRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $resolved = if ([IO.Path]::IsPathRooted($Path)) {
        [IO.Path]::GetFullPath($Path)
    } else {
        [IO.Path]::GetFullPath((Join-Path $CampaignRoot $Path))
    }
    if (-not $resolved.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Evidence path escapes campaign root: $resolved"
    }
    return $resolved
}

function Resolve-ReviewedEvidenceFilePath {
    param([string]$CampaignRoot, [string]$Path, [string[]]$AllowedExactPaths = @())
    try { return Resolve-CampaignEvidencePath $CampaignRoot $Path } catch {
        $resolved = [IO.Path]::GetFullPath($Path)
        foreach ($allowed in @($AllowedExactPaths)) {
            if ($resolved.Equals([IO.Path]::GetFullPath($allowed), [StringComparison]::OrdinalIgnoreCase)) {
                return $resolved
            }
        }
        throw
    }
}

function Expand-ReviewedExpectedOutputPath {
    param([string]$Template, [string]$CampaignRoot, [string]$GateId)
    $gateOutput = Join-Path $CampaignRoot $GateId
    $expanded = $Template.Replace("{repository_root}", $repositoryRoot)
    $expanded = $expanded.Replace("{campaign_root}", $CampaignRoot)
    $expanded = $expanded.Replace("{gate_output}", $gateOutput)
    if ($expanded.Contains("{")) { throw "Expected-output template contains an unresolved token: $Template" }
    return [IO.Path]::GetFullPath($expanded)
}

function Expand-ReviewedCommandValue {
    param(
        [string]$Value,
        [string]$CampaignRoot,
        [string]$GateId,
        [string]$CandidateCommit,
        [string]$PlanSha256,
        [UInt64]$Seed
    )
    $expanded = $Value
    $replacements = [ordered]@{
        "{repository_root}" = $repositoryRoot
        "{gate_output}" = (Join-Path $CampaignRoot $GateId)
        "{campaign_root}" = $CampaignRoot
        "{candidate_commit}" = $CandidateCommit
        "{plan_sha256}" = $PlanSha256
        "{seed}" = [string]$Seed
        "{gate_id}" = $GateId
    }
    foreach ($entry in $replacements.GetEnumerator()) {
        $expanded = $expanded.Replace([string]$entry.Key, [string]$entry.Value)
    }
    if ($expanded.Contains("{")) { throw "Reviewed command value contains an unresolved token: $Value" }
    return $expanded
}

function Get-ExpectedGateInputDigest {
    param(
        [string]$GateId,
        $GateBinding,
        [string]$CandidateCommit,
        [string]$CandidateVersion,
        [string]$PlanSha256,
        [string]$BindingSha256,
        [UInt64]$Seed
    )
    $repositoryPrefix = [IO.Path]::GetFullPath($repositoryRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $inputRows = [System.Collections.Generic.List[object]]::new()
    foreach ($declared in @($GateBinding.input_artifacts)) {
        $relative = [string]$declared
        $absolute = [IO.Path]::GetFullPath((Join-Path $repositoryRoot $relative))
        if (-not $absolute.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Gate input escapes the repository: $relative"
        }
        if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) { throw "Gate input is missing: $relative" }
        $inputRows.Add([ordered]@{ path = $relative.Replace('\', '/'); sha256 = Get-LowerSha256 $absolute })
    }
    $inputMaterial = [ordered]@{
        schema_version = 1
        gate_id = $GateId
        candidate_commit_sha = $CandidateCommit
        candidate_version = $CandidateVersion
        plan_sha256 = $PlanSha256
        binding_sha256 = $BindingSha256
        seed = $Seed
        binding = $GateBinding
        artifacts = @($inputRows)
    }
    return Get-TextSha256 ($inputMaterial | ConvertTo-Json -Depth 100 -Compress)
}

function Test-ExactStringSequence {
    param(
        [AllowEmptyCollection()][object[]]$Left = @(),
        [AllowEmptyCollection()][object[]]$Right = @(),
        [switch]$IgnoreCase,
        [switch]$AsSet
    )
    $leftValues = @($Left | ForEach-Object { [string]$_ })
    $rightValues = @($Right | ForEach-Object { [string]$_ })
    if ($AsSet) {
        $leftValues = @($leftValues | Sort-Object -Unique)
        $rightValues = @($rightValues | Sort-Object -Unique)
    }
    if ($leftValues.Count -ne $rightValues.Count) { return $false }
    for ($index = 0; $index -lt $leftValues.Count; $index++) {
        $equal = if ($IgnoreCase) {
            $leftValues[$index].Equals($rightValues[$index], [StringComparison]::OrdinalIgnoreCase)
        } else {
            $leftValues[$index] -ceq $rightValues[$index]
        }
        if (-not $equal) { return $false }
    }
    return $true
}

function Test-EvidenceFileBinding {
    param(
        [string]$CampaignRoot,
        [string]$Path,
        [string]$Sha256,
        $Size = $null,
        [string[]]$AllowedExactPaths = @()
    )
    try {
        if ($Sha256 -cnotmatch '^[a-f0-9]{64}$') { return $false }
        $resolved = Resolve-ReviewedEvidenceFilePath $CampaignRoot $Path $AllowedExactPaths
        if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) { return $false }
        if ((Get-LowerSha256 $resolved) -cne $Sha256) { return $false }
        if ($null -ne $Size -and (Get-Item -LiteralPath $resolved).Length -ne [long]$Size) { return $false }
        return $true
    } catch { return $false }
}

$script:lastGateReceiptIntegrityFailure = ""
function Fail-GateReceiptIntegrity([string]$Reason) {
    $script:lastGateReceiptIntegrityFailure = $Reason
    return $false
}

function Test-GateReceiptPayloadIntegrity {
    param(
        $Receipt,
        [string]$CampaignRoot,
        [string]$GateId,
        $GateBinding,
        [string]$ExpectedCandidateCommit,
        [string]$ExpectedCandidateVersion,
        [string]$ExpectedPlanSha256,
        [string]$ExpectedBindingSha256,
        [UInt64]$ExpectedSeed,
        [ValidateSet("passed", "failed", "either")][string]$ExpectedStatus = "passed"
    )
    try {
        $script:lastGateReceiptIntegrityFailure = ""
        $expectedInputDigest = Get-ExpectedGateInputDigest `
            $GateId $GateBinding $ExpectedCandidateCommit $ExpectedCandidateVersion `
            $ExpectedPlanSha256 $ExpectedBindingSha256 $ExpectedSeed
        if (
            [int]$Receipt.schema_version -ne 1 -or
            $Receipt.receipt_kind -cne "qpls_multimod_gate_receipt_v1" -or
            $Receipt.coverage_binding_state -cne "executed_real_commands" -or
            $Receipt.gate_id -cne $GateId -or
            $Receipt.candidate_commit_sha -cne $ExpectedCandidateCommit -or
            $Receipt.candidate_version -cne $ExpectedCandidateVersion -or
            $Receipt.plan_sha256 -cne $ExpectedPlanSha256 -or
            $Receipt.binding_sha256 -cne $ExpectedBindingSha256 -or
            $Receipt.binding_set_id -cne [string]$bindings.binding_set_id -or
            [UInt64]$Receipt.seed -ne $ExpectedSeed -or
            [string]$Receipt.input_digest -cne $expectedInputDigest -or
            $Receipt.probable_root_component -cne [string]$GateBinding.probable_root_component -or
            -not (Test-ExactStringSequence @($Receipt.profiles) @($GateBinding.profiles) -AsSet) -or
            -not (Test-ExactStringSequence @($Receipt.covered_evidence_cells) @($GateBinding.covered_evidence_cells) -AsSet)
        ) { return Fail-GateReceiptIntegrity "top_level_binding" }
        $receiptStatus = [string]$Receipt.status
        if (
            $receiptStatus -notin @("passed", "failed") -or
            ($ExpectedStatus -cne "either" -and $receiptStatus -cne $ExpectedStatus)
        ) { return Fail-GateReceiptIntegrity "top_level_status" }
        $passedReceipt = $receiptStatus -ceq "passed"
        if ($passedReceipt) {
            if ($null -ne $Receipt.failure_step -or $null -ne $Receipt.failure_signature) {
                return Fail-GateReceiptIntegrity "passed_receipt_failure_fields"
            }
        } elseif (
            -not ([string]$Receipt.failure_step) -or
            -not ([string]$Receipt.failure_signature)
        ) { return Fail-GateReceiptIntegrity "failed_receipt_failure_fields" }

        $boundStepIds = @($GateBinding.steps | ForEach-Object { [string]$_.step_id })
        $receiptStepIds = @($Receipt.steps | ForEach-Object { [string]$_.step_id })
        if ($receiptStepIds.Count -lt 1 -or $receiptStepIds.Count -gt $boundStepIds.Count) {
            return Fail-GateReceiptIntegrity "step_identity_count"
        }
        if ($passedReceipt -and $boundStepIds.Count -ne $receiptStepIds.Count) {
            return Fail-GateReceiptIntegrity "passed_step_identity_count"
        }
        for ($stepIndex = 0; $stepIndex -lt $receiptStepIds.Count; $stepIndex++) {
            if ($boundStepIds[$stepIndex] -cne $receiptStepIds[$stepIndex]) {
                return Fail-GateReceiptIntegrity "step_identity_order"
            }
        }
        for ($stepIndex = 0; $stepIndex -lt $receiptStepIds.Count; $stepIndex++) {
            $step = $Receipt.steps[$stepIndex]
            $boundStep = $GateBinding.steps[$stepIndex]
            $failedStep = -not $passedReceipt -and $stepIndex -eq ($receiptStepIds.Count - 1)
            $expectedStepStatus = if ($failedStep) { "failed" } else { "passed" }
            if ($step.status -cne $expectedStepStatus) { return Fail-GateReceiptIntegrity "step_status" }
            if ($failedStep -and $Receipt.failure_step -cne $step.step_id) {
                return Fail-GateReceiptIntegrity "failure_step_identity"
            }
            $expectedExecutableName = Expand-ReviewedCommandValue `
                ([string]$boundStep.executable) $CampaignRoot $GateId `
                $ExpectedCandidateCommit $ExpectedPlanSha256 $ExpectedSeed
            $expectedExecutable = (Get-Command $expectedExecutableName -ErrorAction Stop).Source
            if (-not ([IO.Path]::GetFullPath([string]$step.executable).Equals(
                [IO.Path]::GetFullPath($expectedExecutable),
                [StringComparison]::OrdinalIgnoreCase
            ))) { return Fail-GateReceiptIntegrity "step_executable" }
            $expectedArguments = @(
                $boundStep.arguments |
                    ForEach-Object {
                        Expand-ReviewedCommandValue `
                            ([string]$_) $CampaignRoot $GateId `
                            $ExpectedCandidateCommit $ExpectedPlanSha256 $ExpectedSeed
                    }
            )
            $observedArguments = @($step.arguments | ForEach-Object { [string]$_ })
            if ($expectedArguments.Count -ne $observedArguments.Count) { return Fail-GateReceiptIntegrity "step_argument_count" }
            for ($argumentIndex = 0; $argumentIndex -lt $expectedArguments.Count; $argumentIndex++) {
                if ($expectedArguments[$argumentIndex] -cne $observedArguments[$argumentIndex]) { return Fail-GateReceiptIntegrity "step_argument_value" }
            }
            $requiredTestIdentities = @(
                if ($boundStep.PSObject.Properties.Name -ccontains "required_test_identities") {
                    $boundStep.required_test_identities | ForEach-Object { [string]$_ }
                }
            )
            if ([bool]$step.uses_cargo -ne [bool]$boundStep.uses_cargo) { return Fail-GateReceiptIntegrity "step_uses_cargo" }
            if ([long]$step.maximum_seconds -ne [long]$boundStep.maximum_seconds) { return Fail-GateReceiptIntegrity "step_maximum_seconds" }
            if (-not (Test-ExactStringSequence -Left @($step.required_test_identities) -Right @($requiredTestIdentities))) {
                return Fail-GateReceiptIntegrity "step_required_test_identity"
            }
            if (-not $failedStep) {
                if ([int]$step.exit_code -ne 0) { return Fail-GateReceiptIntegrity "step_exit_code" }
                if ([bool]$step.budget_exceeded) { return Fail-GateReceiptIntegrity "step_budget_exceeded" }
                if ([bool]$step.timeout_terminated) { return Fail-GateReceiptIntegrity "step_timeout_terminated" }
                if ([bool]$step.empty_cargo_test_rejected) { return Fail-GateReceiptIntegrity "step_empty_cargo_test" }
                if (@($step.missing_required_test_identities).Count -ne 0) { return Fail-GateReceiptIntegrity "step_missing_required_tests" }
                if (@($step.missing_outputs).Count -ne 0) { return Fail-GateReceiptIntegrity "step_missing_outputs" }
            } else {
                $coherentFailure =
                    [int]$step.exit_code -ne 0 -or
                    [bool]$step.budget_exceeded -or
                    [bool]$step.timeout_terminated -or
                    [bool]$step.empty_cargo_test_rejected -or
                    @($step.missing_required_test_identities).Count -gt 0 -or
                    @($step.missing_outputs).Count -gt 0
                if (-not $coherentFailure) { return Fail-GateReceiptIntegrity "failed_step_has_no_failure" }
                if (-not ([string]$Receipt.failure_signature).StartsWith("$($step.step_id):", [StringComparison]::Ordinal)) {
                    return Fail-GateReceiptIntegrity "failure_signature_identity"
                }
            }
            $isCargoTest = [bool]$boundStep.uses_cargo -and $expectedArguments -ccontains "test"
            if ($isCargoTest -and -not $failedStep -and (
                [int]$step.rust_tests_executed -lt 1 -or
                [int]$step.rust_tests_failed -ne 0 -or
                [int]$step.rust_tests_passed -ne [int]$step.rust_tests_executed
            )) { return Fail-GateReceiptIntegrity "cargo_test_summary" }
            $reviewedOutputPaths = @(
                $boundStep.expected_outputs |
                    ForEach-Object { Expand-ReviewedExpectedOutputPath ([string]$_) $CampaignRoot $GateId }
            )
            $receiptOutputPaths = @($step.expected_outputs | ForEach-Object { [IO.Path]::GetFullPath([string]$_.path) })
            $missingOutputPaths = @($step.missing_outputs | ForEach-Object { [IO.Path]::GetFullPath([string]$_) })
            $accountedOutputPaths = @($receiptOutputPaths + $missingOutputPaths)
            if (
                $reviewedOutputPaths.Count -ne $accountedOutputPaths.Count -or
                -not (Test-ExactStringSequence $reviewedOutputPaths $accountedOutputPaths -IgnoreCase -AsSet)
            ) { return Fail-GateReceiptIntegrity "expected_output_identity" }
            foreach ($stream in @("stdout", "stderr")) {
                if (-not (Test-EvidenceFileBinding `
                    $CampaignRoot `
                    ([string]$step."${stream}_path") `
                    ([string]$step."${stream}_sha256") `
                    $step."${stream}_size")) { return Fail-GateReceiptIntegrity "step_${stream}_binding" }
            }
            foreach ($output in @($step.expected_outputs)) {
                if (-not (Test-EvidenceFileBinding `
                    $CampaignRoot `
                    ([string]$output.path) `
                    ([string]$output.sha256) `
                    $output.size `
                    $reviewedOutputPaths)) { return Fail-GateReceiptIntegrity "expected_output_file_binding" }
            }
            foreach ($missingOutput in $missingOutputPaths) {
                if (Test-Path -LiteralPath $missingOutput -PathType Leaf) {
                    return Fail-GateReceiptIntegrity "missing_output_now_exists"
                }
            }
        }
        return $true
    } catch { return Fail-GateReceiptIntegrity ("exception:" + $_.Exception.Message) }
}

function Test-PassedGateStateEvidence {
    param(
        $GateState,
        [string]$CampaignRoot,
        $GateBinding,
        [string]$ExpectedCandidateCommit,
        [string]$ExpectedCandidateVersion,
        [string]$ExpectedPlanSha256,
        [string]$ExpectedBindingSha256,
        [UInt64]$ExpectedSeed
    )
    try {
        if (
            $GateState.status -cne "passed" -or
            $GateState.evidence_valid -ne $true -or
            [UInt64]$GateState.seed -ne $ExpectedSeed -or
            [int]$GateState.exit_code -ne 0 -or
            @($GateState.invalidated_by).Count -ne 0
        ) { return $false }
        foreach ($rerunRoot in @($GateState.rerun_forced_by)) {
            if (-not (Test-PlanInvalidationPath ([string]$rerunRoot) ([string]$GateState.gate_id) $plan)) {
                return $false
            }
        }
        $expectedReceipt = [IO.Path]::GetFullPath((Join-Path (Join-Path $CampaignRoot $GateState.gate_id) "gate_receipt.json"))
        $receiptPath = Resolve-CampaignEvidencePath $CampaignRoot ([string]$GateState.receipt)
        if ($receiptPath -cne $expectedReceipt) { return $false }
        if (-not (Test-EvidenceFileBinding $CampaignRoot $receiptPath ([string]$GateState.receipt_sha256))) { return $false }
        $receipt = Get-Content -LiteralPath $receiptPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
        if ([string]$GateState.input_digest -cne [string]$receipt.input_digest) { return $false }
        return Test-GateReceiptPayloadIntegrity `
            $receipt $CampaignRoot $GateState.gate_id $GateBinding `
            $ExpectedCandidateCommit $ExpectedCandidateVersion `
            $ExpectedPlanSha256 $ExpectedBindingSha256 $ExpectedSeed
    } catch { return $false }
}

function Initialize-GateAttemptDirectory {
    param([string]$CampaignRoot, [string]$GateId, $GateBinding)
    $gateDirectory = [IO.Path]::GetFullPath((Join-Path $CampaignRoot $GateId))
    $expected = Resolve-CampaignEvidencePath $CampaignRoot $GateId
    if ($gateDirectory -cne $expected) { throw "Gate attempt path is not the exact planned gate directory: $GateId" }
    $gatePrefix = $gateDirectory.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $sharedOutputs = @(
        $GateBinding.steps |
            ForEach-Object { @($_.expected_outputs) } |
            ForEach-Object { Expand-ReviewedExpectedOutputPath ([string]$_) $CampaignRoot $GateId } |
            Sort-Object -Unique |
            Where-Object {
                try {
                    $resolved = Resolve-CampaignEvidencePath $CampaignRoot ([string]$_)
                    -not $resolved.StartsWith($gatePrefix, [StringComparison]::OrdinalIgnoreCase) -and
                        (Test-Path -LiteralPath $resolved)
                } catch { $false }
            } |
            Sort-Object { $_.Length } -Descending
    )
    if ((Test-Path -LiteralPath $gateDirectory) -or $sharedOutputs.Count -gt 0) {
        $historyRoot = Join-Path $CampaignRoot "_attempt_history"
        New-Item -ItemType Directory -Path $historyRoot -Force | Out-Null
        $attempt = 1
        do {
            $historyPath = Join-Path $historyRoot ("{0}.attempt-{1:D4}" -f $GateId, $attempt)
            $attempt++
        } while (Test-Path -LiteralPath $historyPath)
        $null = Resolve-CampaignEvidencePath $CampaignRoot $historyPath
        New-Item -ItemType Directory -Path $historyPath | Out-Null
        if (Test-Path -LiteralPath $gateDirectory) {
            Move-Item -LiteralPath $gateDirectory -Destination (Join-Path $historyPath "gate")
        }
        foreach ($sharedOutput in $sharedOutputs) {
            $relative = Relative-EvidencePath $CampaignRoot $sharedOutput
            $destination = Join-Path (Join-Path $historyPath "shared") $relative
            $destinationParent = Split-Path -Parent $destination
            New-Item -ItemType Directory -Path $destinationParent -Force | Out-Null
            Move-Item -LiteralPath $sharedOutput -Destination $destination
        }
    }
    New-Item -ItemType Directory -Path $gateDirectory -Force | Out-Null
    return $gateDirectory
}

function Test-DependencyVerifierArguments {
    param([object[]]$CommandArguments)
    foreach ($argument in @($CommandArguments)) {
        $normalized = ([string]$argument).Replace("\", "/")
        if (
            $normalized -ceq $dependencyVerifierBasename -or
            $normalized.EndsWith("/$dependencyVerifierBasename", [StringComparison]::Ordinal)
        ) { return $true }
    }
    return $false
}

function Get-ArgumentValues {
    param([object[]]$CommandArguments, [string]$Flag)
    $values = @()
    for ($index = 0; $index -lt @($CommandArguments).Count - 1; $index++) {
        if ([string]$CommandArguments[$index] -ceq $Flag) {
            $values += [string]$CommandArguments[$index + 1]
        }
    }
    return @($values)
}

function Get-ExplicitProducerDependencies {
    param($GateBindings)
    $dependencies = @()
    foreach ($binding in @($GateBindings.gates)) {
        foreach ($step in @($binding.steps)) {
            $commandArguments = @($step.arguments)
            if (-not (Test-DependencyVerifierArguments $commandArguments)) { continue }
            $producerGateIds = @(Get-ArgumentValues $commandArguments "--producer-gate")
            if ($producerGateIds.Count -ne 1) {
                throw "Gate $($binding.gate_id)/$($step.step_id) must name exactly one --producer-gate."
            }
            $dependencies += [pscustomobject]@{
                producer = [string]$producerGateIds[0]
                consumer = [string]$binding.gate_id
                step_id = [string]$step.step_id
            }
        }
    }
    return @($dependencies)
}

function Get-PlanInvalidationTargets {
    param([string]$GateId, $CampaignPlan)
    $gateRows = @($CampaignPlan.gates | Where-Object { $_.gate_id -ceq $GateId })
    if ($gateRows.Count -ne 1) { return @() }
    return @($gateRows[0].invalidates_on_failure | ForEach-Object { [string]$_ })
}

function Test-PlanInvalidationPath {
    param([string]$ProducerGateId, [string]$ConsumerGateId, $CampaignPlan)
    $frontier = @(Get-PlanInvalidationTargets $ProducerGateId $CampaignPlan)
    $visited = @{}
    while ($frontier.Count -gt 0) {
        $candidate = [string]$frontier[0]
        if ($frontier.Count -eq 1) { $frontier = @() } else { $frontier = @($frontier[1..($frontier.Count - 1)]) }
        if ($candidate -ceq $ConsumerGateId) { return $true }
        if ($visited.ContainsKey($candidate)) { continue }
        $visited[$candidate] = $true
        $frontier += @(Get-PlanInvalidationTargets $candidate $CampaignPlan)
    }
    return $false
}

function Assert-CampaignDependencyGraph {
    param(
        $CampaignPlan,
        $GateBindings,
        [int]$ExpectedGateCount = 32,
        [string[]]$BarrierGateIds = $evidenceBarrierGateIds,
        [object[]]$ArtifactDependencies = $artifactProducerDependencies
    )
    $errors = @()
    $planGateIds = @($CampaignPlan.gates | ForEach-Object { [string]$_.gate_id })
    $bindingGateIds = @($GateBindings.gates | ForEach-Object { [string]$_.gate_id })
    if (
        $planGateIds.Count -ne $ExpectedGateCount -or
        @($planGateIds | Sort-Object -Unique).Count -ne $ExpectedGateCount -or
        $bindingGateIds.Count -ne $ExpectedGateCount -or
        @($bindingGateIds | Sort-Object -Unique).Count -ne $ExpectedGateCount -or
        @(Compare-Object $planGateIds $bindingGateIds).Count -ne 0
    ) { $errors += "plan and binding catalogs must contain the same $ExpectedGateCount unique gates" }

    $planOrder = @{}
    for ($index = 0; $index -lt $planGateIds.Count; $index++) { $planOrder[$planGateIds[$index]] = $index }
    $dependencies = @(Get-ExplicitProducerDependencies $GateBindings)
    foreach ($dependency in $dependencies) {
        if (-not $planOrder.ContainsKey($dependency.producer) -or -not $planOrder.ContainsKey($dependency.consumer)) {
            $errors += "$($dependency.consumer)/$($dependency.step_id): producer dependency names an unknown gate: $($dependency.producer)"
            continue
        }
        if ([int]$planOrder[$dependency.producer] -ge [int]$planOrder[$dependency.consumer]) {
            $errors += "$($dependency.consumer)/$($dependency.step_id): producer does not precede consumer: $($dependency.producer)"
            continue
        }
        if (-not (Test-PlanInvalidationPath $dependency.producer $dependency.consumer $CampaignPlan)) {
            $errors += "$($dependency.consumer)/$($dependency.step_id): producer dependency has no invalidation path: $($dependency.producer) -> $($dependency.consumer)"
        }
    }

    foreach ($barrierGateId in $BarrierGateIds) {
        if (-not $planOrder.ContainsKey($barrierGateId)) {
            $errors += "required evidence barrier is missing: $barrierGateId"
            continue
        }
        $barrierIndex = [int]$planOrder[$barrierGateId]
        for ($index = 0; $index -lt $barrierIndex; $index++) {
            $producerGateId = $planGateIds[$index]
            if (-not (Test-PlanInvalidationPath $producerGateId $barrierGateId $CampaignPlan)) {
                $errors += "$barrierGateId`: prior evidence gate has no invalidation path: $producerGateId -> $barrierGateId"
            }
        }
    }

    foreach ($dependency in $ArtifactDependencies) {
        if (-not $planOrder.ContainsKey($dependency.producer) -or -not $planOrder.ContainsKey($dependency.consumer)) {
            $errors += "artifact producer dependency names an unknown gate: $($dependency.producer) -> $($dependency.consumer)"
        } elseif (-not (Test-PlanInvalidationPath $dependency.producer $dependency.consumer $CampaignPlan)) {
            $errors += "$($dependency.consumer): artifact producer has no invalidation path: $($dependency.producer) -> $($dependency.consumer)"
        }
    }
    if ($errors.Count -gt 0) { throw "Campaign dependency graph is invalid:`n - $($errors -join "`n - ")" }
}

function Get-RequiredProducerGateIds {
    param([string]$TargetGateId, $CampaignPlan, $GateBindings)
    $required = @(
        Get-ExplicitProducerDependencies $GateBindings |
            Where-Object { $_.consumer -ceq $TargetGateId } |
            ForEach-Object { [string]$_.producer }
    )
    $required += @(
        $artifactProducerDependencies |
            Where-Object { $_.consumer -ceq $TargetGateId } |
            ForEach-Object { [string]$_.producer }
    )
    if ($runtimeBarrierGateIds -ccontains $TargetGateId) {
        $targetRows = @(0..(@($CampaignPlan.gates).Count - 1) | Where-Object { $CampaignPlan.gates[$_].gate_id -ceq $TargetGateId })
        if ($targetRows.Count -ne 1) { throw "Runtime evidence barrier is missing or duplicated: $TargetGateId" }
        $targetIndex = [int]$targetRows[0]
        if ($targetIndex -gt 0) {
            $required += @($CampaignPlan.gates[0..($targetIndex - 1)] | ForEach-Object { [string]$_.gate_id })
        }
    }
    return @($required | Sort-Object -Unique)
}

function Get-TargetedDiagnosticGateIds {
    param($PriorInventory, $CampaignPlan, $GateBindings)
    $knownGateIds = @($CampaignPlan.gates | ForEach-Object { [string]$_.gate_id })
    $roots = @(
        $PriorInventory.issues |
            Where-Object { $_.disposition -ceq "open" } |
            ForEach-Object { [string]$_.gate } |
            Sort-Object -Unique
    )
    if ($roots.Count -eq 0) { throw "Targeted diagnostics require at least one open prior issue." }
    $unknownRoots = @($roots | Where-Object { $knownGateIds -cnotcontains $_ })
    if ($unknownRoots.Count -gt 0) { throw "Targeted prior inventory names unknown gates: $($unknownRoots -join ', ')" }

    $selected = @{}
    $frontier = @($roots)
    while ($frontier.Count -gt 0) {
        $candidate = [string]$frontier[0]
        if ($frontier.Count -eq 1) { $frontier = @() } else { $frontier = @($frontier[1..($frontier.Count - 1)]) }
        if ($selected.ContainsKey($candidate)) { continue }
        $selected[$candidate] = $true
        $frontier += @(Get-PlanInvalidationTargets $candidate $CampaignPlan)
    }
    foreach ($blockedGateId in $targetedDiagnosticBlockedGateIds) { $selected.Remove($blockedGateId) }

    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($consumerGateId in @($selected.Keys)) {
            $producers = @(
                Get-ExplicitProducerDependencies $GateBindings |
                    Where-Object { $_.consumer -ceq $consumerGateId } |
                    ForEach-Object { [string]$_.producer }
                $artifactProducerDependencies |
                    Where-Object { $_.consumer -ceq $consumerGateId } |
                    ForEach-Object { [string]$_.producer }
            )
            foreach ($producerGateId in @($producers | Sort-Object -Unique)) {
                if (
                    $targetedDiagnosticBlockedGateIds -cnotcontains $producerGateId -and
                    -not $selected.ContainsKey($producerGateId)
                ) { $selected[$producerGateId] = $true; $changed = $true }
            }
        }
    }
    return @($knownGateIds | Where-Object { $selected.ContainsKey($_) })
}

function Get-UnavailableRequiredProducerRoots {
    param([string]$TargetGateId, $CampaignState, $CampaignPlan, $GateBindings)
    $roots = @()
    foreach ($producerGateId in @(Get-RequiredProducerGateIds $TargetGateId $CampaignPlan $GateBindings)) {
        $producerStates = @($CampaignState.gates | Where-Object { $_.gate_id -ceq $producerGateId })
        if ($producerStates.Count -ne 1) { throw "Required producer state is missing or duplicated: $producerGateId" }
        $producerState = $producerStates[0]
        if ($producerState.status -ceq "passed" -and $producerState.evidence_valid -eq $true) { continue }
        $upstreamRoots = @($producerState.invalidated_by | ForEach-Object { [string]$_ })
        if ($upstreamRoots.Count -gt 0) {
            $roots += $upstreamRoots
        } elseif ($producerState.status -ceq "failed" -or $producerState.status -ceq "passed") {
            $roots += $producerGateId
        } elseif ($producerState.status -ceq "blocked") {
            throw "Blocked producer has no invalidating root: $producerGateId"
        } else {
            throw "Required producer is not terminal before $TargetGateId`: $producerGateId ($($producerState.status))"
        }
    }
    return @($roots | Sort-Object -Unique)
}

function Merge-InvalidatingRootIds {
    param([AllowEmptyCollection()][object[]]$Roots = @())
    # Unary comma prevents PowerShell from unrolling an empty or one-item array
    # at the function boundary. Campaign state must always persist an array.
    return ,@($Roots | ForEach-Object { [string]$_ } | Sort-Object -Unique)
}

function Remove-OpenIssueForVerifiedGate {
    param($Inventory, [string]$GateId)
    $removed = @($Inventory.issues | Where-Object { $_.gate -ceq $GateId }).Count -gt 0
    $Inventory.issues = @($Inventory.issues | Where-Object { $_.gate -cne $GateId })
    return $removed
}

function Set-CampaignDocumentsRunning {
    param($CampaignState, $IssueInventory)
    $CampaignState.status = "running"
    $CampaignState.completed_at_utc = $null
    $IssueInventory.campaign_status = "running"
    $IssueInventory.generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
}

function Test-PriorInventoryBindingMatch {
    param($StoredBinding, $ExpectedBinding)
    if ($null -eq $StoredBinding -or $null -eq $ExpectedBinding) {
        return $null -eq $StoredBinding -and $null -eq $ExpectedBinding
    }
    try {
        if (-not ([IO.Path]::GetFullPath([string]$StoredBinding.path).Equals(
            [IO.Path]::GetFullPath([string]$ExpectedBinding.path),
            [StringComparison]::OrdinalIgnoreCase
        ))) { return $false }
        foreach ($property in @(
            "sha256", "campaign_id", "candidate_commit_sha", "candidate_version",
            "plan_sha256", "binding_sha256", "issue_count"
        )) {
            if ([string]$StoredBinding.$property -cne [string]$ExpectedBinding.$property) { return $false }
        }
        return $true
    } catch { return $false }
}

function Get-InvalidatingRootGates([string]$TargetGateId, $Inventory, $CampaignPlan) {
    $roots = @()
    foreach ($issue in @($Inventory.issues | Where-Object { $_.disposition -eq "open" })) {
        $frontier = @($issue.invalidated_downstream_gates | ForEach-Object { [string]$_ })
        $visited = @{}
        $targetReached = $false
        while ($frontier.Count -gt 0 -and -not $targetReached) {
            $candidate = [string]$frontier[0]
            if ($frontier.Count -eq 1) {
                $frontier = @()
            } else {
                $frontier = @($frontier[1..($frontier.Count - 1)])
            }
            if ($visited.ContainsKey($candidate)) { continue }
            $visited[$candidate] = $true
            if ($candidate -ceq $TargetGateId) {
                $targetReached = $true
                break
            }
            $candidatePlan = @($CampaignPlan.gates | Where-Object { $_.gate_id -ceq $candidate })
            if ($candidatePlan.Count -eq 1) {
                $frontier += @($candidatePlan[0].invalidates_on_failure | ForEach-Object { [string]$_ })
            }
        }
        if ($targetReached) { $roots += [string]$issue.gate }
    }
    return @($roots | Sort-Object -Unique)
}

function Get-RerunInvalidatingRootGates {
    param([string]$TargetGateId, [string[]]$SuccessfulRerunGateIds, $CampaignPlan)
    return @(
        $SuccessfulRerunGateIds |
            Where-Object { Test-PlanInvalidationPath ([string]$_) $TargetGateId $CampaignPlan } |
            Sort-Object -Unique
    )
}

function Get-OutstandingRerunRootGates {
    param(
        [string]$TargetGateId,
        [string[]]$SuccessfulRerunGateIds,
        [string[]]$CoveredRerunGateIds,
        $CampaignPlan
    )
    $forcingRoots = @(Get-RerunInvalidatingRootGates $TargetGateId $SuccessfulRerunGateIds $CampaignPlan)
    return @($forcingRoots | Where-Object { $CoveredRerunGateIds -cnotcontains $_ })
}

function Reset-DownstreamRerunCoverageForGate {
    param([string]$GateId, $CampaignState, $CampaignPlan)
    foreach ($gateState in @($CampaignState.gates)) {
        if (Test-PlanInvalidationPath $GateId ([string]$gateState.gate_id) $CampaignPlan) {
            $gateState.rerun_forced_by = @(
                @($gateState.rerun_forced_by) | Where-Object { [string]$_ -cne $GateId }
            )
        }
    }
}

function Invoke-SchedulerGraphSelfTest {
    function New-SyntheticGate([string]$GateId, [string[]]$Invalidates) {
        return [pscustomobject]@{ gate_id = $GateId; invalidates_on_failure = @($Invalidates) }
    }
    function New-SyntheticBinding([string]$GateId, [object[]]$Steps) {
        return [pscustomobject]@{ gate_id = $GateId; steps = @($Steps) }
    }
    function New-SyntheticState([string]$GateId, [string]$Status, [bool]$EvidenceValid, [string[]]$InvalidatedBy) {
        return [pscustomobject]@{ gate_id = $GateId; status = $Status; evidence_valid = $EvidenceValid; invalidated_by = @($InvalidatedBy) }
    }

    $syntheticPlan = [pscustomobject]@{ gates = @(
        New-SyntheticGate "estimation.point.kernels" @("fimix.recovery")
        New-SyntheticGate "fimix.recovery" @("fimix.collapse.boundaries")
        New-SyntheticGate "fimix.collapse.boundaries" @("performance.maximum_profiles")
        New-SyntheticGate "legacy.continuous_and_serialization" @("performance.maximum_profiles")
        New-SyntheticGate "performance.maximum_profiles" @("manifests.prepackage.authority")
        New-SyntheticGate "manifests.prepackage.authority" @("package.candidate")
        New-SyntheticGate "package.candidate" @(
            "installed.offline.smoke",
            "portable.offline.smoke",
            "manifests.live.derivation"
        )
        New-SyntheticGate "installed.offline.smoke" @("manifests.live.derivation")
        New-SyntheticGate "portable.offline.smoke" @("manifests.live.derivation")
        New-SyntheticGate "manifests.live.derivation" @("release.acceptance")
        New-SyntheticGate "release.acceptance" @()
    ) }
    $dependencyStep = [pscustomobject]@{
        step_id = "fimix_boundary_dependency"
        arguments = @(
            $dependencyVerifierBasename,
            "--producer-gate", "fimix.recovery",
            "--producer-step", "heterogeneity_production_science"
        )
    }
    $syntheticBindings = [pscustomobject]@{ gates = @(
        New-SyntheticBinding "estimation.point.kernels" @()
        New-SyntheticBinding "fimix.recovery" @()
        New-SyntheticBinding "fimix.collapse.boundaries" @($dependencyStep)
        New-SyntheticBinding "legacy.continuous_and_serialization" @()
        New-SyntheticBinding "performance.maximum_profiles" @()
        New-SyntheticBinding "manifests.prepackage.authority" @()
        New-SyntheticBinding "package.candidate" @()
        New-SyntheticBinding "installed.offline.smoke" @()
        New-SyntheticBinding "portable.offline.smoke" @()
        New-SyntheticBinding "manifests.live.derivation" @()
        New-SyntheticBinding "release.acceptance" @()
    ) }
    $syntheticArtifacts = @(
        [pscustomobject]@{ producer = "manifests.prepackage.authority"; consumer = "package.candidate" },
        [pscustomobject]@{ producer = "package.candidate"; consumer = "installed.offline.smoke" },
        [pscustomobject]@{ producer = "package.candidate"; consumer = "portable.offline.smoke" }
    )
    Assert-CampaignDependencyGraph $syntheticPlan $syntheticBindings 11 $evidenceBarrierGateIds $syntheticArtifacts

    $targetedFixtureInventory = [pscustomobject]@{ issues = @(
        [pscustomobject]@{ gate = "estimation.point.kernels"; disposition = "open" }
    ) }
    $targetedFixtureIds = @(Get-TargetedDiagnosticGateIds $targetedFixtureInventory $syntheticPlan $syntheticBindings)
    if (
        @($targetedFixtureIds | Where-Object { $targetedDiagnosticBlockedGateIds -ccontains $_ }).Count -ne 0 -or
        @($targetedFixtureIds | Sort-Object).Count -ne 3 -or
        $targetedFixtureIds -cnotcontains "estimation.point.kernels" -or
        $targetedFixtureIds -cnotcontains "fimix.recovery" -or
        $targetedFixtureIds -cnotcontains "fimix.collapse.boundaries"
    ) { throw "Changed-candidate targeted selection crossed a nonqualifying release barrier." }
    $targetedPrerequisiteInventory = [pscustomobject]@{ issues = @(
        [pscustomobject]@{ gate = "fimix.collapse.boundaries"; disposition = "open" }
    ) }
    $targetedPrerequisiteIds = @(Get-TargetedDiagnosticGateIds $targetedPrerequisiteInventory $syntheticPlan $syntheticBindings)
    if (
        $targetedPrerequisiteIds.Count -ne 2 -or
        $targetedPrerequisiteIds -cnotcontains "fimix.recovery" -or
        $targetedPrerequisiteIds -cnotcontains "fimix.collapse.boundaries"
    ) { throw "Targeted diagnostics omitted an explicit producer prerequisite." }

    $brokenPlan = ($syntheticPlan | ConvertTo-Json -Depth 20 | ConvertFrom-Json -Depth 20)
    @($brokenPlan.gates | Where-Object { $_.gate_id -ceq "fimix.recovery" })[0].invalidates_on_failure = @()
    $missingPathRejected = $false
    try {
        Assert-CampaignDependencyGraph $brokenPlan $syntheticBindings 11 $evidenceBarrierGateIds $syntheticArtifacts
    } catch {
        $missingPathRejected = $_.Exception.Message.Contains("fimix.recovery -> fimix.collapse.boundaries")
    }
    if (-not $missingPathRejected) { throw "Scheduler graph self-test did not reject a missing producer path." }

    $blockedProducerState = [pscustomobject]@{ gates = @(
        New-SyntheticState "estimation.point.kernels" "failed" $false @()
        New-SyntheticState "fimix.recovery" "blocked" $false @("estimation.point.kernels")
        New-SyntheticState "fimix.collapse.boundaries" "blocked" $false @("estimation.point.kernels")
        New-SyntheticState "legacy.continuous_and_serialization" "passed" $true @()
        New-SyntheticState "performance.maximum_profiles" "pending" $false @()
        New-SyntheticState "manifests.prepackage.authority" "pending" $false @()
        New-SyntheticState "package.candidate" "pending" $false @()
        New-SyntheticState "installed.offline.smoke" "pending" $false @()
        New-SyntheticState "portable.offline.smoke" "pending" $false @()
        New-SyntheticState "manifests.live.derivation" "pending" $false @()
        New-SyntheticState "release.acceptance" "pending" $false @()
    ) }
    $fimixRoots = @(Get-UnavailableRequiredProducerRoots "fimix.collapse.boundaries" $blockedProducerState $syntheticPlan $syntheticBindings)
    $performanceRoots = @(Get-UnavailableRequiredProducerRoots "performance.maximum_profiles" $blockedProducerState $syntheticPlan $syntheticBindings)
    if ($fimixRoots.Count -ne 1 -or $fimixRoots[0] -cne "estimation.point.kernels") {
        throw "Blocked FIMIX producer did not preserve its estimation root."
    }
    if ($performanceRoots.Count -ne 1 -or $performanceRoots[0] -cne "estimation.point.kernels") {
        throw "Performance barrier did not preserve the earlier estimation root."
    }

    $barrierCascadeState = ($blockedProducerState | ConvertTo-Json -Depth 20 | ConvertFrom-Json -Depth 20)
    $performanceState = @($barrierCascadeState.gates | Where-Object { $_.gate_id -ceq "performance.maximum_profiles" })[0]
    $performanceState.status = "blocked"; $performanceState.invalidated_by = @("estimation.point.kernels")
    $prepackageRoots = @(Get-UnavailableRequiredProducerRoots "manifests.prepackage.authority" $barrierCascadeState $syntheticPlan $syntheticBindings)
    if ($prepackageRoots.Count -ne 1 -or $prepackageRoots[0] -cne "estimation.point.kernels") {
        throw "Prepackage barrier did not preserve the earlier estimation root."
    }
    $prepackageState = @($barrierCascadeState.gates | Where-Object { $_.gate_id -ceq "manifests.prepackage.authority" })[0]
    $prepackageState.status = "blocked"; $prepackageState.invalidated_by = @("estimation.point.kernels")
    $packageRoots = @(Get-UnavailableRequiredProducerRoots "package.candidate" $barrierCascadeState $syntheticPlan $syntheticBindings)
    if ($packageRoots.Count -ne 1 -or $packageRoots[0] -cne "estimation.point.kernels") {
        throw "Package barrier did not preserve the earlier estimation root."
    }
    $packageState = @($barrierCascadeState.gates | Where-Object { $_.gate_id -ceq "package.candidate" })[0]
    $packageState.status = "blocked"; $packageState.invalidated_by = @("estimation.point.kernels")
    foreach ($smokeGateId in @("installed.offline.smoke", "portable.offline.smoke")) {
        $smokeRoots = @(Get-UnavailableRequiredProducerRoots $smokeGateId $barrierCascadeState $syntheticPlan $syntheticBindings)
        if ($smokeRoots.Count -ne 1 -or $smokeRoots[0] -cne "estimation.point.kernels") {
            throw "$smokeGateId did not preserve the blocked package root."
        }
    }

    $failedPackageState = ($barrierCascadeState | ConvertTo-Json -Depth 20 | ConvertFrom-Json -Depth 20)
    $failedPackage = @($failedPackageState.gates | Where-Object { $_.gate_id -ceq "package.candidate" })[0]
    $failedPackage.status = "failed"; $failedPackage.invalidated_by = @()
    foreach ($smokeGateId in @("installed.offline.smoke", "portable.offline.smoke")) {
        $smokeRoots = @(Get-UnavailableRequiredProducerRoots $smokeGateId $failedPackageState $syntheticPlan $syntheticBindings)
        if ($smokeRoots.Count -ne 1 -or $smokeRoots[0] -cne "package.candidate") {
            throw "$smokeGateId did not block on the failed package state."
        }
    }

    $legacyFailureState = ($blockedProducerState | ConvertTo-Json -Depth 20 | ConvertFrom-Json -Depth 20)
    foreach ($row in @($legacyFailureState.gates)) {
        if ($row.gate_id -in @("estimation.point.kernels", "fimix.recovery", "fimix.collapse.boundaries")) {
            $row.status = "passed"; $row.evidence_valid = $true; $row.invalidated_by = @()
        }
        if ($row.gate_id -ceq "legacy.continuous_and_serialization") {
            $row.status = "failed"; $row.evidence_valid = $false; $row.invalidated_by = @()
        }
    }
    $legacyRoots = @(Get-UnavailableRequiredProducerRoots "performance.maximum_profiles" $legacyFailureState $syntheticPlan $syntheticBindings)
    if ($legacyRoots.Count -ne 1 -or $legacyRoots[0] -cne "legacy.continuous_and_serialization") {
        throw "Performance barrier did not block on an earlier legacy failure."
    }

    $allPassedState = ($legacyFailureState | ConvertTo-Json -Depth 20 | ConvertFrom-Json -Depth 20)
    foreach ($row in @($allPassedState.gates)) {
        if ($row.gate_id -in @(
            "estimation.point.kernels",
            "fimix.recovery",
            "fimix.collapse.boundaries",
            "legacy.continuous_and_serialization"
        )) { $row.status = "passed"; $row.evidence_valid = $true; $row.invalidated_by = @() }
    }
    $allPassedRoots = @(Get-UnavailableRequiredProducerRoots "performance.maximum_profiles" $allPassedState $syntheticPlan $syntheticBindings)
    if ($allPassedRoots.Count -ne 0) { throw "Valid earlier evidence incorrectly blocked the performance barrier." }

    $emptyRootSet = Merge-InvalidatingRootIds -Roots @()
    $singleRootSet = Merge-InvalidatingRootIds -Roots @("estimation.point.kernels")
    $multipleRootSet = Merge-InvalidatingRootIds -Roots @(
        "legacy.continuous_and_serialization",
        "estimation.point.kernels",
        "legacy.continuous_and_serialization"
    )
    if (
        $emptyRootSet -isnot [array] -or $emptyRootSet.Count -ne 0 -or
        $singleRootSet -isnot [array] -or $singleRootSet.Count -ne 1 -or
        $multipleRootSet -isnot [array] -or $multipleRootSet.Count -ne 2
    ) { throw "Invalidating roots do not preserve zero/one/multiple array shape." }

    $transitionState = [pscustomobject]@{ status = "completed_with_issues"; completed_at_utc = "stale" }
    $transitionInventory = [pscustomobject]@{
        campaign_status = "completed_with_issues"
        generated_at_utc = "stale"
        issues = @([pscustomobject]@{ gate = "verified.gate" }, [pscustomobject]@{ gate = "other.gate" })
    }
    $transitionRemovedIssue = Remove-OpenIssueForVerifiedGate $transitionInventory "verified.gate"
    Set-CampaignDocumentsRunning $transitionState $transitionInventory
    if (
        $transitionState.status -cne "running" -or $null -ne $transitionState.completed_at_utc -or
        $transitionInventory.campaign_status -cne "running" -or
        -not $transitionRemovedIssue -or @($transitionInventory.issues).Count -ne 1 -or
        $transitionInventory.issues[0].gate -cne "other.gate"
    ) { throw "Resume transition did not clear stale completion state and verified self-issue." }
    $rerunRoots = @(Get-RerunInvalidatingRootGates "fimix.collapse.boundaries" @("estimation.point.kernels") $syntheticPlan)
    if ($rerunRoots.Count -ne 1 -or $rerunRoots[0] -cne "estimation.point.kernels") {
        throw "Successful repaired root did not force a downstream rerun."
    }
    $outstandingAfterCoverage = @(
        Get-OutstandingRerunRootGates `
            "fimix.collapse.boundaries" `
            @("estimation.point.kernels") `
            @("estimation.point.kernels") `
            $syntheticPlan
    )
    if ($outstandingAfterCoverage.Count -ne 0) {
        throw "Satisfied repaired-root coverage forced an endless downstream rerun."
    }
    $coverageResetState = [pscustomobject]@{ gates = @(
        [pscustomobject]@{ gate_id = "estimation.point.kernels"; rerun_forced_by = @() },
        [pscustomobject]@{ gate_id = "fimix.collapse.boundaries"; rerun_forced_by = @("estimation.point.kernels") }
    ) }
    Reset-DownstreamRerunCoverageForGate "estimation.point.kernels" $coverageResetState $syntheticPlan
    $outstandingAfterNewRootAttempt = @(
        Get-OutstandingRerunRootGates `
            "fimix.collapse.boundaries" `
            @("estimation.point.kernels") `
            @($coverageResetState.gates[1].rerun_forced_by) `
            $syntheticPlan
    )
    if ($outstandingAfterNewRootAttempt.Count -ne 1) {
        throw "A new repaired-root attempt incorrectly reused old descendant coverage."
    }
    $priorFixture = [pscustomobject]@{
        path = (Join-Path $repositoryRoot "validation\fixture.json")
        sha256 = ("1" * 64) -join ""
        campaign_id = "prior-campaign"
        candidate_commit_sha = ("2" * 40) -join ""
        candidate_version = "2.56.0"
        plan_sha256 = ("3" * 64) -join ""
        binding_sha256 = ("4" * 64) -join ""
        issue_count = 1
    }
    $tamperedPriorFixture = ($priorFixture | ConvertTo-Json | ConvertFrom-Json)
    $tamperedPriorFixture.sha256 = ("5" * 64) -join ""
    if (
        -not (Test-PriorInventoryBindingMatch $null $null) -or
        -not (Test-PriorInventoryBindingMatch $priorFixture $priorFixture) -or
        (Test-PriorInventoryBindingMatch $priorFixture $tamperedPriorFixture) -or
        (Test-PriorInventoryBindingMatch $priorFixture $null)
    ) { throw "Prior issue inventory substitution was not rejected." }

    $temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $receiptFixtureRoot = Join-Path $temporaryBase ("qpls-multimod-scheduler-self-test-" + [Guid]::NewGuid().ToString("N"))
    if (-not ([IO.Path]::GetFullPath($receiptFixtureRoot) + [IO.Path]::DirectorySeparatorChar).StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Scheduler receipt fixture escaped the system temporary directory."
    }
    try {
        $fixtureGateId = "receipt.fixture"
        $fixtureGateDirectory = Join-Path $receiptFixtureRoot $fixtureGateId
        $fixtureSharedDirectory = Join-Path $receiptFixtureRoot "shared-fixture"
        New-Item -ItemType Directory -Path $fixtureGateDirectory -Force | Out-Null
        New-Item -ItemType Directory -Path $fixtureSharedDirectory -Force | Out-Null
        $fixtureStdout = Join-Path $fixtureGateDirectory "fixture.stdout.log"
        $fixtureStderr = Join-Path $fixtureGateDirectory "fixture.stderr.log"
        $fixtureGateOutput = Join-Path $fixtureGateDirectory "result.json"
        $fixtureSharedOutput = Join-Path $fixtureSharedDirectory "result.json"
        [IO.File]::WriteAllText($fixtureStdout, "fixture stdout", [Text.UTF8Encoding]::new($false))
        [IO.File]::WriteAllText($fixtureStderr, "", [Text.UTF8Encoding]::new($false))
        [IO.File]::WriteAllText($fixtureGateOutput, '{"passed":true}', [Text.UTF8Encoding]::new($false))
        [IO.File]::WriteAllText($fixtureSharedOutput, '{"shared":true}', [Text.UTF8Encoding]::new($false))
        $fixtureBinding = [pscustomobject]@{
            gate_id = $fixtureGateId
            profiles = @("fixture")
            covered_evidence_cells = @("fixture::receipt")
            probable_root_component = "runner"
            input_artifacts = @("Cargo.toml")
            steps = @(
            [pscustomobject]@{
                step_id = "fixture_step"
                executable = "python"
                arguments = @("--version")
                uses_cargo = $false
                maximum_seconds = 60
                required_test_identities = @()
                expected_outputs = @(
                    "{gate_output}/result.json",
                    "{campaign_root}/shared-fixture/result.json",
                    "{repository_root}/Cargo.toml"
                )
            }
        ) }
        $fixtureCandidate = ("a" * 40) -join ""
        $fixturePlanSha = ("b" * 64) -join ""
        $fixtureBindingSha = ("c" * 64) -join ""
        $fixtureInputDigest = Get-ExpectedGateInputDigest `
            $fixtureGateId $fixtureBinding $fixtureCandidate "2.56.0" `
            $fixturePlanSha $fixtureBindingSha ([UInt64]42)
        $fixtureReceiptPath = Join-Path $fixtureGateDirectory "gate_receipt.json"
        $fixtureReceipt = [ordered]@{
            receipt_kind = "qpls_multimod_gate_receipt_v1"
            schema_version = 1
            coverage_binding_state = "executed_real_commands"
            gate_id = $fixtureGateId
            status = "passed"
            failure_step = $null
            failure_signature = $null
            candidate_commit_sha = $fixtureCandidate
            candidate_version = "2.56.0"
            plan_sha256 = $fixturePlanSha
            binding_sha256 = $fixtureBindingSha
            binding_set_id = [string]$bindings.binding_set_id
            seed = [UInt64]42
            input_digest = $fixtureInputDigest
            profiles = @("fixture")
            covered_evidence_cells = @("fixture::receipt")
            probable_root_component = "runner"
            steps = @([ordered]@{
                step_id = "fixture_step"
                status = "passed"
                executable = (Get-Command python -ErrorAction Stop).Source
                arguments = @("--version")
                uses_cargo = $false
                exit_code = 0
                maximum_seconds = 60
                budget_exceeded = $false
                timeout_terminated = $false
                empty_cargo_test_rejected = $false
                required_test_identities = @()
                missing_required_test_identities = @()
                rust_tests_passed = $null
                rust_tests_failed = $null
                rust_tests_executed = $null
                missing_outputs = @()
                stdout_path = $fixtureStdout
                stdout_sha256 = Get-LowerSha256 $fixtureStdout
                stdout_size = (Get-Item -LiteralPath $fixtureStdout).Length
                stderr_path = $fixtureStderr
                stderr_sha256 = Get-LowerSha256 $fixtureStderr
                stderr_size = (Get-Item -LiteralPath $fixtureStderr).Length
                expected_outputs = @(
                    [ordered]@{ path = $fixtureGateOutput; sha256 = Get-LowerSha256 $fixtureGateOutput; size = (Get-Item -LiteralPath $fixtureGateOutput).Length },
                    [ordered]@{ path = $fixtureSharedOutput; sha256 = Get-LowerSha256 $fixtureSharedOutput; size = (Get-Item -LiteralPath $fixtureSharedOutput).Length },
                    [ordered]@{ path = (Join-Path $repositoryRoot "Cargo.toml"); sha256 = Get-LowerSha256 (Join-Path $repositoryRoot "Cargo.toml"); size = (Get-Item -LiteralPath (Join-Path $repositoryRoot "Cargo.toml")).Length }
                )
            })
        }
        Write-JsonAtomic $fixtureReceiptPath $fixtureReceipt
        $fixtureState = [pscustomobject]@{
            gate_id = $fixtureGateId
            status = "passed"
            evidence_valid = $true
            seed = [UInt64]42
            exit_code = 0
            invalidated_by = @()
            rerun_forced_by = @()
            input_digest = $fixtureInputDigest
            receipt = Relative-EvidencePath $receiptFixtureRoot $fixtureReceiptPath
            receipt_sha256 = Get-LowerSha256 $fixtureReceiptPath
        }
        if (-not (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42))) {
            throw "Valid passed receipt fixture was rejected: $script:lastGateReceiptIntegrityFailure"
        }
        $failedReceipt = ($fixtureReceipt | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100)
        $failedReceipt.status = "failed"
        $failedReceipt.failure_step = "fixture_step"
        $failedReceipt.failure_signature = "fixture_step:exit_7:$(("f" * 64) -join '')"
        $failedReceipt.steps[0].status = "failed"
        $failedReceipt.steps[0].exit_code = 7
        foreach ($expectedFailedStatus in @("failed", "either")) {
            if (-not (Test-GateReceiptPayloadIntegrity `
                $failedReceipt $receiptFixtureRoot $fixtureGateId $fixtureBinding `
                $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42) `
                $expectedFailedStatus)) {
                throw "Coherent failed receipt fixture was rejected under $expectedFailedStatus`: $script:lastGateReceiptIntegrityFailure"
            }
        }
        if (Test-GateReceiptPayloadIntegrity `
            $failedReceipt $receiptFixtureRoot $fixtureGateId $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42) "passed") {
            throw "Failed receipt fixture was accepted as passed evidence."
        }
        $semanticTamper = ($fixtureReceipt | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100)
        $semanticTamper.input_digest = ("e" * 64) -join ""
        $semanticTamper.steps[0].arguments = @("--fabricated")
        $semanticTamper.steps[0].exit_code = 9
        Write-JsonAtomic $fixtureReceiptPath $semanticTamper
        $fixtureState.receipt_sha256 = Get-LowerSha256 $fixtureReceiptPath
        $fixtureState.input_digest = [string]$semanticTamper.input_digest
        if (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42)) {
            throw "Semantically tampered passed receipt fixture was accepted."
        }
        Write-JsonAtomic $fixtureReceiptPath $fixtureReceipt
        $fixtureState.receipt_sha256 = Get-LowerSha256 $fixtureReceiptPath
        $fixtureState.input_digest = $fixtureInputDigest
        $fixtureState.seed = [UInt64]99
        if (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42)) {
            throw "Passed gate state with the wrong seed was accepted."
        }
        $fixtureState.seed = [UInt64]42
        $fixtureState.invalidated_by = @("unknown.root")
        if (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42)) {
            throw "Passed gate state with an invalidating root was accepted."
        }
        $fixtureState.invalidated_by = @()
        $fixtureState.rerun_forced_by = @("unknown.root")
        if (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42)) {
            throw "Passed gate state with an unknown rerun root was accepted."
        }
        $fixtureState.rerun_forced_by = @()
        [IO.File]::AppendAllText($fixtureSharedOutput, "tampered", [Text.UTF8Encoding]::new($false))
        if (Test-PassedGateStateEvidence `
            $fixtureState $receiptFixtureRoot $fixtureBinding `
            $fixtureCandidate "2.56.0" $fixturePlanSha $fixtureBindingSha ([UInt64]42)) {
            throw "Tampered passed receipt fixture was accepted."
        }
        [IO.File]::WriteAllText($fixtureSharedOutput, '{"shared":true}', [Text.UTF8Encoding]::new($false))
        $newFixtureGateDirectory = Initialize-GateAttemptDirectory $receiptFixtureRoot $fixtureGateId $fixtureBinding
        $fixtureHistory = Join-Path $receiptFixtureRoot "_attempt_history\receipt.fixture.attempt-0001"
        if (
            -not (Test-Path -LiteralPath $newFixtureGateDirectory -PathType Container) -or
            @(Get-ChildItem -LiteralPath $newFixtureGateDirectory -Force).Count -ne 0 -or
            -not (Test-Path -LiteralPath (Join-Path $fixtureHistory "gate\gate_receipt.json") -PathType Leaf) -or
            -not (Test-Path -LiteralPath (Join-Path $fixtureHistory "shared\shared-fixture\result.json") -PathType Leaf)
        ) { throw "Gate rerun did not rotate gate-local and shared outputs together." }
        $emptyOutputBinding = [pscustomobject]@{
            steps = @([pscustomobject]@{ expected_outputs = @() })
        }
        $emptyOutputDirectory = Initialize-GateAttemptDirectory `
            $receiptFixtureRoot "empty-output.fixture" $emptyOutputBinding
        if (
            -not (Test-Path -LiteralPath $emptyOutputDirectory -PathType Container) -or
            @(Get-ChildItem -LiteralPath $emptyOutputDirectory -Force).Count -ne 0
        ) { throw "Singleton step with no expected outputs could not initialize its gate directory." }
    } finally {
        if (Test-Path -LiteralPath $receiptFixtureRoot) {
            $resolvedFixture = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $receiptFixtureRoot).Path)
            if (-not ($resolvedFixture + [IO.Path]::DirectorySeparatorChar).StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Refusing to remove scheduler fixture outside the system temporary directory."
            }
            Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
        }
    }

    return [pscustomobject]@{
        schema_version = 1
        report_id = "qpls.multimod.scheduler-graph-self-test.v1"
        passed = $true
        checks = @(
            "valid_transitive_graph_accepted",
            "changed_candidate_targeted_selection_is_diagnostic_only",
            "targeted_selection_includes_explicit_prerequisites",
            "missing_producer_path_rejected",
            "blocked_fimix_producer_preserves_root",
            "performance_blocks_on_scientific_failure",
            "prepackage_blocks_on_earlier_invalid_evidence",
            "package_blocks_on_earlier_invalid_evidence",
            "installed_and_portable_block_on_package_state",
            "performance_blocks_on_legacy_failure",
            "performance_allows_valid_prior_evidence",
            "invalidating_roots_preserve_array_shape",
            "resume_transition_clears_stale_completion_and_self_issue",
            "successful_repair_forces_downstream_rerun",
            "satisfied_repair_coverage_does_not_rerun_forever",
            "new_root_attempt_invalidates_old_descendant_coverage",
            "resume_rejects_prior_inventory_substitution",
            "coherent_failed_receipt_retains_diagnostics_but_not_passed_evidence",
            "passed_gate_state_rejects_seed_and_root_tampering",
            "passed_receipt_integrity_rejects_hash_and_semantic_tampering",
            "rerun_rotates_gate_and_shared_outputs",
            "singleton_empty_expected_outputs_are_supported"
        )
    }
}

function Show-CampaignPlan {
    $unbound = @($plan.gates | Where-Object { $_.implementation_status -ne "ready" -or $null -eq $_.command })
    Write-Host "Plan: $($plan.plan_id)"
    Write-Host "Candidate: $($plan.candidate.branch), version $($plan.candidate.final_version)"
    Write-Host "Pass: $CampaignPass"
    Write-Host "Gates: $(@($plan.gates).Count); unbound gates: $($unbound.Count)"
    Write-Host "Live reports are generated only beneath the external campaign root. Tracked manifests remain Labs/absent."
    foreach ($gate in $plan.gates) { Write-Host ("  {0,-42} {1}" -f $gate.gate_id, $gate.implementation_status) }
}

Assert-CampaignDependencyGraph $plan $bindings
if ($SchedulerSelfTest) {
    Invoke-SchedulerGraphSelfTest | ConvertTo-Json -Depth 20
    return
}
Show-CampaignPlan
if (-not $Execute) { Write-Host "Plan-only mode completed. No campaign output, test, build, package or evidence was created."; return }
$unbound = @($plan.gates | Where-Object { $_.implementation_status -ne "ready" -or $null -eq $_.command })
if ($unbound.Count -gt 0) { throw "Unbound gates: $(($unbound.gate_id) -join ', ')" }
if (@($bindings.gates).Count -ne 32 -or @(Compare-Object @($bindings.gates.gate_id) @($plan.gates.gate_id)).Count -ne 0) { throw "Reviewed gate catalog differs from the 32-gate plan." }
if ($bindings.binding_kind -cne "reviewed_executable_coverage_v1" -or $bindings.placeholder_bindings_permitted -ne $false) { throw "Gate catalog does not explicitly reject placeholder coverage." }

$priorInventoryBinding = $null
$prior = $null
if ($CampaignPass -eq "inventory") {
    if ($PriorIssueInventory) { throw "The inventory pass cannot consume a prior inventory." }
} else {
    if (-not $PriorIssueInventory) { throw "$CampaignPass pass requires -PriorIssueInventory." }
    $resolvedPrior = (Resolve-Path -LiteralPath $PriorIssueInventory).Path
    $prior = Get-Content -LiteralPath $resolvedPrior -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    if ($prior.campaign_status -ne "completed_with_issues" -or @($prior.issues).Count -lt 1) { throw "Prior inventory must be a completed issue-bearing campaign." }
    if ($prior.candidate_version -cne [string]$plan.candidate.final_version -or $prior.plan_sha256 -cne $planSha256 -or $prior.binding_sha256 -cne $bindingSha256) { throw "Prior inventory candidate/version/plan/binding identity is stale." }
    $priorInventoryBinding = [ordered]@{ path = $resolvedPrior; sha256 = Get-LowerSha256 $resolvedPrior; campaign_id = [string]$prior.campaign_id; candidate_commit_sha = [string]$prior.candidate_commit_sha; candidate_version = [string]$prior.candidate_version; plan_sha256 = [string]$prior.plan_sha256; binding_sha256 = [string]$prior.binding_sha256; issue_count = @($prior.issues).Count }
}

$preflight = Assert-CampaignPreflight
$targetedGateIds = @()
if ($CampaignPass -eq "targeted") {
    if ($prior.candidate_commit_sha -ceq $preflight.candidate_commit_sha) {
        throw "Same-candidate targeted recovery must use -Resume; targeted pass is changed-candidate diagnostic only."
    }
    $targetedGateIds = @(Get-TargetedDiagnosticGateIds $prior $plan $bindings)
    if ($targetedGateIds.Count -eq 0) { throw "Targeted diagnostic selection is empty." }
}
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$campaignId = "v256-multimod-$CampaignPass-$($preflight.candidate_commit_sha.Substring(0, 12))-$stamp"
if (-not $OutputRoot) { $OutputRoot = "D:\QuickPLS-MultiMod-Evidence\v256-multimod" }
$OutputRoot = [IO.Path]::GetFullPath($OutputRoot)
$outputPrefix = $OutputRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
foreach ($protectedRepository in @($repositoryRoot, (Resolve-Path -LiteralPath $MainRepositoryRoot).Path)) {
    $repositoryPrefix = [IO.Path]::GetFullPath($protectedRepository).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if ($outputPrefix.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Qualification output must be outside candidate and main repositories: $OutputRoot"
    }
}
$campaignRoot = Join-Path $OutputRoot $campaignId
if ($Resume) {
    $runs = @(Get-ChildItem -LiteralPath $OutputRoot -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "v256-multimod-$CampaignPass-$($preflight.candidate_commit_sha.Substring(0, 12))-*" } | Sort-Object LastWriteTimeUtc -Descending)
    if ($runs.Count -eq 0) { throw "No resumable exact-candidate campaign exists." }
    $campaignRoot = $runs[0].FullName; $campaignId = $runs[0].Name
} elseif (Test-Path -LiteralPath $campaignRoot) { throw "Campaign output already exists: $campaignRoot" }

New-Item -ItemType Directory -Path $campaignRoot -Force | Out-Null
$statePath = Join-Path $campaignRoot "campaign_state.json"
$issuePath = Join-Path $campaignRoot "issue_inventory.json"
if ($Resume -and (
    -not (Test-Path -LiteralPath $statePath -PathType Leaf) -or
    -not (Test-Path -LiteralPath $issuePath -PathType Leaf)
)) { throw "Resume requires both campaign_state.json and issue_inventory.json." }
if (Test-Path -LiteralPath $statePath) {
    $state = Get-Content -LiteralPath $statePath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    if ([int]$state.schema_version -ne 2 -or [UInt64]$state.seed -ne [UInt64]$bindings.campaign_seed -or $state.campaign_id -cne $campaignId -or $state.candidate_commit_sha -cne $preflight.candidate_commit_sha -or $state.candidate_version -cne [string]$plan.candidate.final_version -or $state.plan_sha256 -cne $planSha256 -or $state.binding_sha256 -cne $bindingSha256 -or $state.campaign_pass -cne $CampaignPass) { throw "Resume state binding differs." }
    if (-not (Test-PriorInventoryBindingMatch $state.prior_issue_inventory $priorInventoryBinding)) {
        throw "Resume prior issue inventory binding differs."
    }
    if ($CampaignPass -eq "targeted") {
        if ($state.qualification_eligible -ne $false -or @(Compare-Object @($state.targeted_gate_ids) @($targetedGateIds)).Count -ne 0) {
            throw "Resume targeted diagnostic selection or qualification boundary differs."
        }
    } elseif ($state.qualification_eligible -ne $true) {
        throw "Resume state is not qualification eligible."
    }
} else {
    $state = [ordered]@{ schema_version = 2; campaign_id = $campaignId; campaign_pass = $CampaignPass; qualification_eligible = $CampaignPass -ne "targeted"; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; seed = [UInt64]$bindings.campaign_seed; prior_issue_inventory = $priorInventoryBinding; targeted_gate_ids = @($targetedGateIds); successful_resume_rerun_roots = @(); started_at_utc = (Get-Date).ToUniversalTime().ToString("o"); completed_at_utc = $null; status = "running"; preflight = $preflight; gates = @() }
    foreach ($gate in $plan.gates) {
        $state.gates += [ordered]@{ gate_id = $gate.gate_id; status = "pending"; evidence_valid = $false; invalidated_by = @(); rerun_forced_by = @(); started_at_utc = $null; completed_at_utc = $null; duration_ms = $null; exit_code = $null; seed = [UInt64]$bindings.campaign_seed; input_digest = $null; receipt = $null; receipt_sha256 = $null; stdout = $null; stderr = $null }
    }
    Write-JsonAtomic $statePath $state
}
$inventory = if (Test-Path -LiteralPath $issuePath) { Get-Content -LiteralPath $issuePath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100 } else { [ordered]@{ schema_version = 1; inventory_id = "qpls.multimod.issues.$campaignId"; campaign_id = $campaignId; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; generated_at_utc = (Get-Date).ToUniversalTime().ToString("o"); campaign_status = "running"; issues = @() } }
if ([int]$inventory.schema_version -ne 1 -or $inventory.inventory_id -cne "qpls.multimod.issues.$campaignId" -or $inventory.campaign_id -cne $campaignId -or $inventory.candidate_commit_sha -cne $preflight.candidate_commit_sha -or $inventory.candidate_version -cne [string]$plan.candidate.final_version -or $inventory.plan_sha256 -cne $planSha256 -or $inventory.binding_sha256 -cne $bindingSha256) { throw "Issue inventory candidate/version/plan/binding identity is stale." }
$reconciledSuccessfulRerunRoots = @()
if ($Resume) {
    if ($state.status -notin @("running", "completed_with_issues", "completed_clean", "completed_diagnostic_with_issues", "completed_diagnostic")) {
        throw "Resume state has an unknown campaign status."
    }
    if ($inventory.campaign_status -notin @("running", "completed_with_issues", "completed_clean")) {
        throw "Resume issue inventory has an unknown campaign status."
    }
    $stateGateIds = @($state.gates | ForEach-Object { [string]$_.gate_id })
    $planGateIds = @($plan.gates | ForEach-Object { [string]$_.gate_id })
    if (
        $stateGateIds.Count -ne $planGateIds.Count -or
        @($stateGateIds | Sort-Object -Unique).Count -ne $planGateIds.Count -or
        @(Compare-Object $stateGateIds $planGateIds).Count -ne 0
    ) { throw "Resume state does not contain the exact qualification gate set." }
    $persistedResumeRoots = @($state.successful_resume_rerun_roots | ForEach-Object { [string]$_ })
    if (@($persistedResumeRoots | Where-Object { $planGateIds -cnotcontains $_ }).Count -gt 0) {
        throw "Resume state contains an unknown successful rerun root."
    }
    $allowedResumeStatuses = @("pending", "running", "passed", "failed", "blocked", "not_executed_targeted")
    if (@($state.gates | Where-Object { $allowedResumeStatuses -cnotcontains [string]$_.status }).Count -gt 0) {
        throw "Resume state contains an unknown gate status."
    }
    if (@($state.gates | Where-Object { [UInt64]$_.seed -ne [UInt64]$bindings.campaign_seed }).Count -gt 0) {
        throw "Resume state contains a gate seed that differs from the frozen campaign seed."
    }
    if ($CampaignPass -ne "targeted" -and @($state.gates | Where-Object { $_.status -ceq "not_executed_targeted" }).Count -gt 0) {
        throw "Qualification-eligible resume state contains targeted-only gate statuses."
    }
    foreach ($issue in @($inventory.issues)) {
        $issuePlanRows = @($plan.gates | Where-Object { $_.gate_id -ceq $issue.gate })
        if (
            $issuePlanRows.Count -ne 1 -or
            $issue.disposition -cne "open" -or
            [UInt64]$issue.seed -ne [UInt64]$bindings.campaign_seed -or
            [string]$issue.input_digest -cnotmatch '^[a-f0-9]{64}$' -or
            -not [string]$issue.failure_signature -or
            @(Compare-Object @($issue.invalidated_downstream_gates) @($issuePlanRows[0].invalidates_on_failure)).Count -ne 0
        ) { throw "Resume issue inventory contains an invalid or stale gate issue." }
        if ($CampaignPass -eq "targeted" -and $targetedGateIds -cnotcontains [string]$issue.gate) {
            throw "Targeted resume contains an issue outside its frozen diagnostic gate set."
        }
    }

    foreach ($gateState in @($state.gates | Where-Object { $_.status -ceq "passed" -and $_.evidence_valid -eq $true })) {
        $gateBindingRows = @($bindings.gates | Where-Object { $_.gate_id -ceq $gateState.gate_id })
        if ($gateBindingRows.Count -ne 1 -or -not (Test-PassedGateStateEvidence `
            $gateState $campaignRoot $gateBindingRows[0] `
            $preflight.candidate_commit_sha ([string]$plan.candidate.final_version) `
            $planSha256 $bindingSha256 ([UInt64]$bindings.campaign_seed))) {
            $gateState.evidence_valid = $false
        } else {
            $removedStaleIssue = Remove-OpenIssueForVerifiedGate $inventory $gateState.gate_id
            if ($removedStaleIssue) { $reconciledSuccessfulRerunRoots += [string]$gateState.gate_id }
        }
    }

    for ($gateIndex = 0; $gateIndex -lt @($plan.gates).Count; $gateIndex++) {
        $gate = $plan.gates[$gateIndex]
        $gateState = @($state.gates | Where-Object { $_.gate_id -ceq $gate.gate_id })[0]
        $openIssues = @($inventory.issues | Where-Object { $_.gate -ceq $gate.gate_id -and $_.disposition -ceq "open" })
        if ($openIssues.Count -gt 1) { throw "Resume issue inventory duplicates gate $($gate.gate_id)." }
        if ($gateState.status -ceq "failed" -and $openIssues.Count -eq 0) {
            $recoveryDigest = if ([string]$gateState.input_digest -cmatch '^[a-f0-9]{64}$') {
                [string]$gateState.input_digest
            } else {
                Get-TextSha256 "$campaignId|$($gate.gate_id)|resume-failed-state"
            }
            $inventory.issues += [ordered]@{
                issue_id = "MM-{0:D4}" -f ($gateIndex + 1)
                profile = @($gate.profiles) -join ","
                gate = [string]$gate.gate_id
                seed = [UInt64]$bindings.campaign_seed
                input_digest = $recoveryDigest
                failure_signature = "campaign_resume:reconstructed_failed_state:$recoveryDigest"
                probable_root_component = [string]$gate.probable_root_component
                invalidated_downstream_gates = @($gate.invalidates_on_failure)
                disposition = "open"
                observed_at_utc = (Get-Date).ToUniversalTime().ToString("o")
                evidence_paths = @($gateState.stdout, $gateState.stderr, $gateState.receipt | Where-Object { $_ })
            }
        }
    }
    $state.successful_resume_rerun_roots = @(
        @($persistedResumeRoots + $reconciledSuccessfulRerunRoots) | Sort-Object -Unique
    )
    Set-CampaignDocumentsRunning $state $inventory
    Write-JsonAtomic $statePath $state
}
Write-JsonAtomic $issuePath $inventory

$environmentNames = @("QPLS_MULTIMOD_CAMPAIGN_ID", "QPLS_MULTIMOD_CAMPAIGN_ROOT", "QPLS_MULTIMOD_CAMPAIGN_PASS", "QPLS_MULTIMOD_CANDIDATE_COMMIT", "QPLS_MULTIMOD_CANDIDATE_VERSION", "QPLS_MULTIMOD_PLAN_SHA256", "QPLS_MULTIMOD_BINDING_SHA256", "QPLS_MULTIMOD_GATE_ID", "QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY", "QPLS_MULTIMOD_SEED")
$savedEnvironment = @{}; foreach ($name in $environmentNames) { $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }
$successfulResumeRerunRoots = @($state.successful_resume_rerun_roots | Sort-Object -Unique)
try {
    for ($gateIndex = 0; $gateIndex -lt @($plan.gates).Count; $gateIndex++) {
        $gate = $plan.gates[$gateIndex]
        $gateState = @($state.gates | Where-Object { $_.gate_id -ceq $gate.gate_id })[0]
        if ($CampaignPass -eq "targeted" -and $targetedGateIds -cnotcontains $gate.gate_id) {
            $notExecutedAt = (Get-Date).ToUniversalTime().ToString("o")
            $gateState.status = "not_executed_targeted"
            $gateState.evidence_valid = $false
            $gateState.seed = [UInt64]$bindings.campaign_seed
            $gateState.invalidated_by = @()
            $gateState.rerun_forced_by = @()
            $gateState.started_at_utc = $notExecutedAt
            $gateState.completed_at_utc = $notExecutedAt
            $gateState.duration_ms = 0
            $gateState.exit_code = $null
            $gateState.input_digest = $null
            $gateState.receipt = $null
            $gateState.receipt_sha256 = $null
            $gateState.stdout = $null
            $gateState.stderr = $null
            Write-JsonAtomic $statePath $state
            continue
        }
        # Resume reuses only exact-candidate evidence that is both passed and
        # still valid. A downstream gate that passed while an upstream issue
        # was open is deliberately rerun after that issue clears.
        $invalidatedBy = Merge-InvalidatingRootIds -Roots @(
            Get-InvalidatingRootGates $gate.gate_id $inventory $plan
            Get-UnavailableRequiredProducerRoots $gate.gate_id $state $plan $bindings
        )
        $rerunForcedBy = @(
            Get-RerunInvalidatingRootGates $gate.gate_id $successfulResumeRerunRoots $plan
        )
        $outstandingRerunBy = @(
            $rerunForcedBy | Where-Object { @($gateState.rerun_forced_by) -cnotcontains $_ }
        )
        if (
            $Resume -and
            $gateState.status -eq "passed" -and
            $gateState.evidence_valid -eq $true -and
            $invalidatedBy.Count -eq 0 -and
            $outstandingRerunBy.Count -eq 0
        ) { continue }
        Assert-CandidateUnchanged $preflight.candidate_commit_sha
        if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -gt 0) { throw "Another Cargo process is active before $($gate.gate_id)." }
        if ($gate.stage -eq "package") { $null = Assert-DiskThresholds }
        if ($invalidatedBy.Count -gt 0) {
            $blockedAt = (Get-Date).ToUniversalTime().ToString("o")
            $gateState.status = "blocked"
            $gateState.evidence_valid = $false
            $gateState.seed = [UInt64]$bindings.campaign_seed
            $gateState.invalidated_by = $invalidatedBy
            $gateState.rerun_forced_by = @($rerunForcedBy)
            $gateState.started_at_utc = $blockedAt
            $gateState.completed_at_utc = $blockedAt
            $gateState.duration_ms = 0
            $gateState.exit_code = $null
            $gateState.input_digest = $null
            $gateState.receipt = $null
            $gateState.receipt_sha256 = $null
            $gateState.stdout = $null
            $gateState.stderr = $null
            Write-JsonAtomic $statePath $state
            continue
        }
        if ($Resume) {
            Reset-DownstreamRerunCoverageForGate $gate.gate_id $state $plan
            Write-JsonAtomic $statePath $state
        }
        $gateBinding = @($bindings.gates | Where-Object { $_.gate_id -ceq $gate.gate_id })[0]
        $gateDirectory = Initialize-GateAttemptDirectory $campaignRoot $gate.gate_id $gateBinding
        $stdoutPath = Join-Path $gateDirectory "stdout.log"; $stderrPath = Join-Path $gateDirectory "stderr.log"; $receiptPath = Join-Path $gateDirectory "gate_receipt.json"
        $gateState.status = "running"
        $gateState.evidence_valid = $false
        $gateState.seed = [UInt64]$bindings.campaign_seed
        $gateState.invalidated_by = @($invalidatedBy)
        $gateState.rerun_forced_by = @($rerunForcedBy)
        $gateState.started_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        $gateState.completed_at_utc = $null
        $gateState.duration_ms = $null
        $gateState.exit_code = $null
        $gateState.input_digest = $null
        $gateState.receipt = $null
        $gateState.receipt_sha256 = $null
        $gateState.stdout = $null
        $gateState.stderr = $null
        Write-JsonAtomic $statePath $state
        $env:QPLS_MULTIMOD_CAMPAIGN_ID = $campaignId; $env:QPLS_MULTIMOD_CAMPAIGN_ROOT = $campaignRoot; $env:QPLS_MULTIMOD_CAMPAIGN_PASS = $CampaignPass
        $env:QPLS_MULTIMOD_CANDIDATE_COMMIT = $preflight.candidate_commit_sha; $env:QPLS_MULTIMOD_CANDIDATE_VERSION = [string]$plan.candidate.final_version
        $env:QPLS_MULTIMOD_PLAN_SHA256 = $planSha256; $env:QPLS_MULTIMOD_BINDING_SHA256 = $bindingSha256; $env:QPLS_MULTIMOD_GATE_ID = $gate.gate_id
        $env:QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY = $gateDirectory; $env:QPLS_MULTIMOD_SEED = [string]$bindings.campaign_seed
        $started = (Get-Date).ToUniversalTime()
        $stepBudgetSeconds = [long]0
        foreach ($boundStep in @($gateBinding.steps)) { $stepBudgetSeconds += [long]$boundStep.maximum_seconds }
        $outerBudgetSeconds = $stepBudgetSeconds + [math]::Max(120, 30 * @($gateBinding.steps).Count)
        $outerBudgetMilliseconds = [long]$outerBudgetSeconds * 1000L
        if ($outerBudgetMilliseconds -gt [int]::MaxValue) { throw "Gate $($gate.gate_id) outer timeout exceeds WaitForExit limits." }
        $process = Start-Process -FilePath $gate.command.executable -ArgumentList @($gate.command.arguments) -WorkingDirectory $repositoryRoot -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -NoNewWindow -PassThru
        $campaignGateTimedOut = -not $process.WaitForExit([int]$outerBudgetMilliseconds)
        if ($campaignGateTimedOut) {
            & taskkill.exe /PID $process.Id /T /F *> $null
            if (-not $process.WaitForExit(30000)) { throw "Timed-out gate $($gate.gate_id) did not terminate its exact wrapper-owned process tree." }
        }
        $process.WaitForExit()
        $processExitCode = if ($campaignGateTimedOut) { -1 } else { $process.ExitCode }
        $completed = (Get-Date).ToUniversalTime()
        Assert-CandidateUnchanged $preflight.candidate_commit_sha
        $receipt = $null; $receiptValid = $false
        if (Test-Path -LiteralPath $receiptPath -PathType Leaf) {
            try {
                $receipt = Get-Content -LiteralPath $receiptPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
                $receiptValid = Test-GateReceiptPayloadIntegrity `
                    $receipt $campaignRoot $gate.gate_id $gateBinding `
                    $preflight.candidate_commit_sha ([string]$plan.candidate.final_version) `
                    $planSha256 $bindingSha256 ([UInt64]$bindings.campaign_seed) "either"
            } catch { $receiptValid = $false }
        }
        $passed = -not $campaignGateTimedOut -and $processExitCode -eq 0 -and $receiptValid -and $receipt.status -ceq "passed"
        $gateState.exit_code = $processExitCode; $gateState.completed_at_utc = $completed.ToString("o"); $gateState.duration_ms = [long][math]::Round(($completed - $started).TotalMilliseconds)
        $gateState.stdout = Relative-EvidencePath $campaignRoot $stdoutPath; $gateState.stderr = Relative-EvidencePath $campaignRoot $stderrPath
        if ($receiptValid) { $gateState.input_digest = [string]$receipt.input_digest; $gateState.receipt = Relative-EvidencePath $campaignRoot $receiptPath; $gateState.receipt_sha256 = Get-LowerSha256 $receiptPath }
        if ($passed) {
            $gateState.status = "passed"; $gateState.evidence_valid = $invalidatedBy.Count -eq 0
            $inventory.issues = @($inventory.issues | Where-Object { $_.gate -cne $gate.gate_id })
            if ($Resume) {
                $successfulResumeRerunRoots = @($successfulResumeRerunRoots + [string]$gate.gate_id | Sort-Object -Unique)
                $state.successful_resume_rerun_roots = @($successfulResumeRerunRoots)
            }
        } else {
            $gateState.status = "failed"; $gateState.evidence_valid = $false
            $fallback = [ordered]@{ gate = $gate; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; seed = [UInt64]$bindings.campaign_seed } | ConvertTo-Json -Depth 100 -Compress
            $inputDigest = if ($receiptValid) { [string]$receipt.input_digest } else { Get-TextSha256 $fallback }
            $stdoutText = if (Test-Path $stdoutPath) { Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 } else { "" }; $stderrText = if (Test-Path $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8 } else { "" }
            $tail = $stderrText + "`n" + $stdoutText; if ($tail.Length -gt 131072) { $tail = $tail.Substring($tail.Length - 131072) }
            $signature = if ($campaignGateTimedOut) { "campaign_wrapper:timeout_$outerBudgetSeconds`sec:$(Get-TextSha256 $tail)" } elseif ($receiptValid -and $receipt.failure_signature) { [string]$receipt.failure_signature } else { "campaign_wrapper:exit_$processExitCode`:$(Get-TextSha256 $tail)" }
            $profile = if ($receiptValid -and @($receipt.profiles).Count) { @($receipt.profiles) -join "," } else { @($gate.profiles) -join "," }
            $existing = @($inventory.issues | Where-Object { $_.gate -ceq $gate.gate_id })
            $issue = [ordered]@{ issue_id = "MM-{0:D4}" -f ($gateIndex + 1); profile = $profile; gate = $gate.gate_id; seed = [UInt64]$bindings.campaign_seed; input_digest = $inputDigest; failure_signature = $signature; probable_root_component = $gate.probable_root_component; invalidated_downstream_gates = @($gate.invalidates_on_failure); disposition = "open"; observed_at_utc = (Get-Date).ToUniversalTime().ToString("o"); evidence_paths = @($gateState.stdout, $gateState.stderr) }
            if ($receiptValid) { $issue.evidence_paths += $gateState.receipt }
            if ($existing.Count -eq 0) { $inventory.issues += $issue } else { for ($i = 0; $i -lt @($inventory.issues).Count; $i++) { if ($inventory.issues[$i].gate -ceq $gate.gate_id) { $inventory.issues[$i] = $issue } } }
        }
        $inventory.generated_at_utc = (Get-Date).ToUniversalTime().ToString("o"); Write-JsonAtomic $statePath $state; Write-JsonAtomic $issuePath $inventory
    }
} finally { foreach ($name in $environmentNames) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], "Process") } }

$failed = @($state.gates | Where-Object status -eq "failed"); $blocked = @($state.gates | Where-Object status -eq "blocked"); $pending = @($state.gates | Where-Object status -notin @("passed", "failed", "blocked", "not_executed_targeted"))
if ($pending.Count) {
    $state.status = "running"; $inventory.campaign_status = "running"
} elseif ($CampaignPass -eq "targeted") {
    if ($failed.Count -or $blocked.Count -or @($inventory.issues).Count) {
        $state.status = "completed_diagnostic_with_issues"; $inventory.campaign_status = "completed_with_issues"
    } else {
        $state.status = "completed_diagnostic"; $inventory.campaign_status = "completed_clean"
    }
} elseif ($failed.Count -or $blocked.Count -or @($inventory.issues).Count) {
    $state.status = "completed_with_issues"; $inventory.campaign_status = "completed_with_issues"
} else {
    $state.status = "completed_clean"; $inventory.campaign_status = "completed_clean"
}
if ($state.status -ceq "completed_clean") {
    $state.successful_resume_rerun_roots = @()
    foreach ($gateState in @($state.gates)) { $gateState.rerun_forced_by = @() }
}
$state.completed_at_utc = (Get-Date).ToUniversalTime().ToString("o"); $inventory.generated_at_utc = (Get-Date).ToUniversalTime().ToString("o"); Write-JsonAtomic $statePath $state; Write-JsonAtomic $issuePath $inventory
$evidenceIndex = [ordered]@{ schema_version = 1; campaign_id = $campaignId; campaign_pass = $CampaignPass; qualification_eligible = $CampaignPass -ne "targeted"; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; campaign_state_sha256 = Get-LowerSha256 $statePath; issue_inventory_sha256 = Get-LowerSha256 $issuePath; gate_receipts = @($state.gates | Where-Object receipt_sha256 | ForEach-Object { [ordered]@{ gate_id = $_.gate_id; path = $_.receipt; sha256 = $_.receipt_sha256; input_digest = $_.input_digest; seed = $_.seed } }); generated_at_utc = (Get-Date).ToUniversalTime().ToString("o") }
Write-JsonAtomic (Join-Path $campaignRoot "campaign_evidence_index.json") $evidenceIndex
$finalDisk = Assert-DiskThresholds
Write-Host "Campaign status: $($state.status)"; Write-Host "Campaign root: $campaignRoot"; Write-Host "Issue inventory: $issuePath"; Write-Host "Retained free space: C $($finalDisk.C) GiB; D $($finalDisk.D) GiB"
if ($failed.Count -or $blocked.Count -or @($inventory.issues).Count) { exit 1 }
