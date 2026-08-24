[CmdletBinding()]
param(
    [switch]$Execute,
    [switch]$Resume,
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

function Show-CampaignPlan {
    $unbound = @($plan.gates | Where-Object { $_.implementation_status -ne "ready" -or $null -eq $_.command })
    Write-Host "Plan: $($plan.plan_id)"
    Write-Host "Candidate: $($plan.candidate.branch), version $($plan.candidate.final_version)"
    Write-Host "Pass: $CampaignPass"
    Write-Host "Gates: $(@($plan.gates).Count); unbound gates: $($unbound.Count)"
    Write-Host "Live reports are generated only beneath the external campaign root. Tracked manifests remain Labs/absent."
    foreach ($gate in $plan.gates) { Write-Host ("  {0,-42} {1}" -f $gate.gate_id, $gate.implementation_status) }
}

Show-CampaignPlan
if (-not $Execute) { Write-Host "Plan-only mode completed. No campaign output, test, build, package or evidence was created."; return }
$unbound = @($plan.gates | Where-Object { $_.implementation_status -ne "ready" -or $null -eq $_.command })
if ($unbound.Count -gt 0) { throw "Unbound gates: $(($unbound.gate_id) -join ', ')" }
if (@($bindings.gates).Count -ne 32 -or (Compare-Object @($bindings.gates.gate_id) @($plan.gates.gate_id)).Count -ne 0) { throw "Reviewed gate catalog differs from the 32-gate plan." }
if ($bindings.binding_kind -cne "reviewed_executable_coverage_v1" -or $bindings.placeholder_bindings_permitted -ne $false) { throw "Gate catalog does not explicitly reject placeholder coverage." }

$priorInventoryBinding = $null
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
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$campaignId = "v256-multimod-$CampaignPass-$($preflight.candidate_commit_sha.Substring(0, 12))-$stamp"
if (-not $OutputRoot) { $OutputRoot = "D:\QuickPLS-MultiMod-Evidence\v256-multimod" }
$OutputRoot = [IO.Path]::GetFullPath($OutputRoot)
$repoPrefix = [IO.Path]::GetFullPath($repositoryRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (($OutputRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar).StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "Qualification output must be outside tracked source." }
$campaignRoot = Join-Path $OutputRoot $campaignId
if ($Resume) {
    $runs = @(Get-ChildItem -LiteralPath $OutputRoot -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "v256-multimod-$CampaignPass-$($preflight.candidate_commit_sha.Substring(0, 12))-*" } | Sort-Object LastWriteTimeUtc -Descending)
    if ($runs.Count -eq 0) { throw "No resumable exact-candidate campaign exists." }
    $campaignRoot = $runs[0].FullName; $campaignId = $runs[0].Name
} elseif (Test-Path -LiteralPath $campaignRoot) { throw "Campaign output already exists: $campaignRoot" }

New-Item -ItemType Directory -Path $campaignRoot -Force | Out-Null
$statePath = Join-Path $campaignRoot "campaign_state.json"
$issuePath = Join-Path $campaignRoot "issue_inventory.json"
if (Test-Path -LiteralPath $statePath) {
    $state = Get-Content -LiteralPath $statePath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    if ($state.candidate_commit_sha -cne $preflight.candidate_commit_sha -or $state.candidate_version -cne [string]$plan.candidate.final_version -or $state.plan_sha256 -cne $planSha256 -or $state.binding_sha256 -cne $bindingSha256 -or $state.campaign_pass -cne $CampaignPass) { throw "Resume state binding differs." }
} else {
    $state = [ordered]@{ schema_version = 2; campaign_id = $campaignId; campaign_pass = $CampaignPass; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; seed = [UInt64]$bindings.campaign_seed; prior_issue_inventory = $priorInventoryBinding; started_at_utc = (Get-Date).ToUniversalTime().ToString("o"); completed_at_utc = $null; status = "running"; preflight = $preflight; gates = @() }
    foreach ($gate in $plan.gates) {
        $state.gates += [ordered]@{ gate_id = $gate.gate_id; status = "pending"; evidence_valid = $false; invalidated_by = @(); started_at_utc = $null; completed_at_utc = $null; duration_ms = $null; exit_code = $null; seed = [UInt64]$bindings.campaign_seed; input_digest = $null; receipt = $null; receipt_sha256 = $null; stdout = $null; stderr = $null }
    }
    Write-JsonAtomic $statePath $state
}
$inventory = if (Test-Path -LiteralPath $issuePath) { Get-Content -LiteralPath $issuePath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100 } else { [ordered]@{ schema_version = 1; inventory_id = "qpls.multimod.issues.$campaignId"; campaign_id = $campaignId; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; generated_at_utc = (Get-Date).ToUniversalTime().ToString("o"); campaign_status = "running"; issues = @() } }
if ($inventory.candidate_commit_sha -cne $preflight.candidate_commit_sha -or $inventory.candidate_version -cne [string]$plan.candidate.final_version -or $inventory.plan_sha256 -cne $planSha256 -or $inventory.binding_sha256 -cne $bindingSha256) { throw "Issue inventory candidate/version/plan/binding identity is stale." }
Write-JsonAtomic $issuePath $inventory

$environmentNames = @("QPLS_MULTIMOD_CAMPAIGN_ID", "QPLS_MULTIMOD_CAMPAIGN_ROOT", "QPLS_MULTIMOD_CAMPAIGN_PASS", "QPLS_MULTIMOD_CANDIDATE_COMMIT", "QPLS_MULTIMOD_CANDIDATE_VERSION", "QPLS_MULTIMOD_PLAN_SHA256", "QPLS_MULTIMOD_BINDING_SHA256", "QPLS_MULTIMOD_GATE_ID", "QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY", "QPLS_MULTIMOD_SEED")
$savedEnvironment = @{}; foreach ($name in $environmentNames) { $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }
try {
    for ($gateIndex = 0; $gateIndex -lt @($plan.gates).Count; $gateIndex++) {
        $gate = $plan.gates[$gateIndex]
        $gateState = @($state.gates | Where-Object { $_.gate_id -ceq $gate.gate_id })[0]
        # Resume reuses only exact-candidate evidence that is both passed and
        # still valid. A downstream gate that passed while an upstream issue
        # was open is deliberately rerun after that issue clears.
        if ($Resume -and $gateState.status -eq "passed" -and $gateState.evidence_valid -eq $true) { continue }
        Assert-CandidateUnchanged $preflight.candidate_commit_sha
        if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -gt 0) { throw "Another Cargo process is active before $($gate.gate_id)." }
        if ($gate.stage -eq "package") { $null = Assert-DiskThresholds }
        $invalidatedBy = @($inventory.issues | Where-Object { $_.disposition -eq "open" -and @($_.invalidated_downstream_gates) -contains $gate.gate_id } | ForEach-Object gate)
        $gateDirectory = Join-Path $campaignRoot $gate.gate_id
        New-Item -ItemType Directory -Path $gateDirectory -Force | Out-Null
        $stdoutPath = Join-Path $gateDirectory "stdout.log"; $stderrPath = Join-Path $gateDirectory "stderr.log"; $receiptPath = Join-Path $gateDirectory "gate_receipt.json"
        $gateState.status = "running"; $gateState.evidence_valid = $false; $gateState.invalidated_by = $invalidatedBy; $gateState.started_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        Write-JsonAtomic $statePath $state
        $env:QPLS_MULTIMOD_CAMPAIGN_ID = $campaignId; $env:QPLS_MULTIMOD_CAMPAIGN_ROOT = $campaignRoot; $env:QPLS_MULTIMOD_CAMPAIGN_PASS = $CampaignPass
        $env:QPLS_MULTIMOD_CANDIDATE_COMMIT = $preflight.candidate_commit_sha; $env:QPLS_MULTIMOD_CANDIDATE_VERSION = [string]$plan.candidate.final_version
        $env:QPLS_MULTIMOD_PLAN_SHA256 = $planSha256; $env:QPLS_MULTIMOD_BINDING_SHA256 = $bindingSha256; $env:QPLS_MULTIMOD_GATE_ID = $gate.gate_id
        $env:QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY = $gateDirectory; $env:QPLS_MULTIMOD_SEED = [string]$bindings.campaign_seed
        $started = (Get-Date).ToUniversalTime()
        $gateBinding = @($bindings.gates | Where-Object { $_.gate_id -ceq $gate.gate_id })[0]
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
                $receiptValid = $receipt.receipt_kind -ceq "qpls_multimod_gate_receipt_v1" -and $receipt.coverage_binding_state -ceq "executed_real_commands" -and $receipt.gate_id -ceq $gate.gate_id -and $receipt.candidate_commit_sha -ceq $preflight.candidate_commit_sha -and $receipt.candidate_version -ceq [string]$plan.candidate.final_version -and $receipt.plan_sha256 -ceq $planSha256 -and $receipt.binding_sha256 -ceq $bindingSha256 -and [UInt64]$receipt.seed -eq [UInt64]$bindings.campaign_seed -and [string]$receipt.input_digest -cmatch '^[a-f0-9]{64}$'
            } catch { $receiptValid = $false }
        }
        $passed = -not $campaignGateTimedOut -and $processExitCode -eq 0 -and $receiptValid -and $receipt.status -ceq "passed"
        $gateState.exit_code = $processExitCode; $gateState.completed_at_utc = $completed.ToString("o"); $gateState.duration_ms = [long][math]::Round(($completed - $started).TotalMilliseconds)
        $gateState.stdout = Relative-EvidencePath $campaignRoot $stdoutPath; $gateState.stderr = Relative-EvidencePath $campaignRoot $stderrPath
        if ($receiptValid) { $gateState.input_digest = [string]$receipt.input_digest; $gateState.receipt = Relative-EvidencePath $campaignRoot $receiptPath; $gateState.receipt_sha256 = Get-LowerSha256 $receiptPath }
        if ($passed) {
            $gateState.status = "passed"; $gateState.evidence_valid = $invalidatedBy.Count -eq 0
            $inventory.issues = @($inventory.issues | Where-Object { $_.gate -cne $gate.gate_id })
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

$failed = @($state.gates | Where-Object status -eq "failed"); $pending = @($state.gates | Where-Object status -notin @("passed", "failed"))
if ($pending.Count) { $state.status = "running"; $inventory.campaign_status = "running" } elseif ($failed.Count -or @($inventory.issues).Count) { $state.status = "completed_with_issues"; $inventory.campaign_status = "completed_with_issues" } else { $state.status = "completed_clean"; $inventory.campaign_status = "completed_clean" }
$state.completed_at_utc = (Get-Date).ToUniversalTime().ToString("o"); $inventory.generated_at_utc = (Get-Date).ToUniversalTime().ToString("o"); Write-JsonAtomic $statePath $state; Write-JsonAtomic $issuePath $inventory
$evidenceIndex = [ordered]@{ schema_version = 1; campaign_id = $campaignId; campaign_pass = $CampaignPass; candidate_commit_sha = $preflight.candidate_commit_sha; candidate_version = [string]$plan.candidate.final_version; plan_sha256 = $planSha256; binding_sha256 = $bindingSha256; campaign_state_sha256 = Get-LowerSha256 $statePath; issue_inventory_sha256 = Get-LowerSha256 $issuePath; gate_receipts = @($state.gates | Where-Object receipt_sha256 | ForEach-Object { [ordered]@{ gate_id = $_.gate_id; path = $_.receipt; sha256 = $_.receipt_sha256; input_digest = $_.input_digest; seed = $_.seed } }); generated_at_utc = (Get-Date).ToUniversalTime().ToString("o") }
Write-JsonAtomic (Join-Path $campaignRoot "campaign_evidence_index.json") $evidenceIndex
$finalDisk = Assert-DiskThresholds
Write-Host "Campaign status: $($state.status)"; Write-Host "Campaign root: $campaignRoot"; Write-Host "Issue inventory: $issuePath"; Write-Host "Retained free space: C $($finalDisk.C) GiB; D $($finalDisk.D) GiB"
if ($failed.Count -or @($inventory.issues).Count) { exit 1 }
