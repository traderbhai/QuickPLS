[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z0-9_.-]+$')]
    [string]$GateId
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$startedAt = (Get-Date).ToUniversalTime()
$gateClock = [Diagnostics.Stopwatch]::StartNew()

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$catalogPath = Join-Path $PSScriptRoot "multimod_gate_bindings_v1.json"
$planPath = Join-Path $PSScriptRoot "v256_multimod_qualification_plan_v1.json"
$catalog = Get-Content -LiteralPath $catalogPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$bindings = @($catalog.gates | Where-Object { $_.gate_id -ceq $GateId })
$gateMaximumSeconds = 7080
if ($bindings.Count -ne 1) {
    throw "Gate $GateId does not have one exact reviewed command binding."
}
$binding = $bindings[0]
if ($catalog.binding_kind -cne "reviewed_executable_coverage_v1" -or $catalog.placeholder_bindings_permitted -ne $false) {
    throw "The gate catalog does not carry the reviewed executable-coverage identity."
}
$plannedGate = @($plan.gates | Where-Object { $_.gate_id -ceq $GateId })
if ($plannedGate.Count -ne 1 -or $plannedGate[0].implementation_status -cne "ready" -or $null -eq $plannedGate[0].command) {
    $reason = if ($binding.pending_reason) { [string]$binding.pending_reason } else { "No reviewed real command is bound." }
    throw "Gate $GateId is pending and cannot emit a qualification receipt. $reason"
}
if (@($binding.steps).Count -eq 0 -or @($binding.covered_evidence_cells).Count -eq 0) {
    throw "Ready gate $GateId has no executable steps or covered evidence cells."
}

function Get-LowerSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-TextSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ([Convert]::ToHexString($sha.ComputeHash($bytes))).ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Write-JsonAtomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )
    Write-TextAtomic -Path $Path -Text (($Value | ConvertTo-Json -Depth 100) + "`n")
}

function Write-TextAtomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text
    )
    $parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $temporary = Join-Path $parent (".{0}.{1}.tmp" -f ([IO.Path]::GetFileName($Path)), [Guid]::NewGuid().ToString("N"))
    try {
        [IO.File]::WriteAllText($temporary, $Text, [Text.UTF8Encoding]::new($false))
        [IO.File]::Move($temporary, $Path, $true)
    }
    finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Resolve-ExactProcessLaunch {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments
    )
    $resolvedExecutable = (Get-Command $Executable -ErrorAction Stop).Source
    $extension = [IO.Path]::GetExtension($resolvedExecutable).ToLowerInvariant()
    if ($extension -notin @(".cmd", ".bat")) {
        return [pscustomobject]@{
            ResolvedExecutable = $resolvedExecutable
            FileName = $resolvedExecutable
            Arguments = @($Arguments)
            LaunchKind = "native_argument_list"
        }
    }
    $shimName = [IO.Path]::GetFileName($resolvedExecutable).ToLowerInvariant()
    $cliName = switch ($shimName) {
        "npm.cmd" { "npm-cli.js" }
        "npx.cmd" { "npx-cli.js" }
        default { throw "Reviewed command-script launch is admitted only for npm.cmd/npx.cmd; found $resolvedExecutable" }
    }
    $shimDirectory = Split-Path -Parent $resolvedExecutable
    $nodeExecutable = Join-Path $shimDirectory "node.exe"
    if (-not (Test-Path -LiteralPath $nodeExecutable -PathType Leaf)) {
        $nodeExecutable = (Get-Command "node.exe" -ErrorAction Stop).Source
    }
    $cliPath = Join-Path $shimDirectory ("node_modules\npm\bin\{0}" -f $cliName)
    if (-not (Test-Path -LiteralPath $cliPath -PathType Leaf)) {
        throw "The exact $shimName Node CLI entry point is missing: $cliPath"
    }
    return [pscustomobject]@{
        ResolvedExecutable = $resolvedExecutable
        FileName = $nodeExecutable
        Arguments = @($cliPath) + @($Arguments)
        LaunchKind = "node_cli_argument_list"
    }
}

function Stop-VerifiedProcessTree {
    param(
        [Parameter(Mandatory = $true)][Diagnostics.Process]$Process,
        [Parameter(Mandatory = $true)][string]$Stage
    )
    if ($Process.HasExited) { return }
    $primaryFailure = $null
    try { $Process.Kill($true) } catch { $primaryFailure = $_.Exception.Message }
    if (-not $Process.WaitForExit(10000) -and $IsWindows -and -not $Process.HasExited) {
        $taskkillInfo = [Diagnostics.ProcessStartInfo]::new()
        $taskkillInfo.FileName = (Get-Command "taskkill.exe" -ErrorAction Stop).Source
        $taskkillInfo.UseShellExecute = $false
        $taskkillInfo.CreateNoWindow = $true
        foreach ($argument in @("/PID", $Process.Id.ToString(), "/T", "/F")) {
            [void]$taskkillInfo.ArgumentList.Add($argument)
        }
        $taskkill = [Diagnostics.Process]::new()
        $taskkill.StartInfo = $taskkillInfo
        try {
            [void]$taskkill.Start()
            if (-not $taskkill.WaitForExit(10000)) {
                try { $taskkill.Kill($true) } catch { Write-Warning $_.Exception.Message }
                [void]$taskkill.WaitForExit(5000)
            }
        }
        finally { $taskkill.Dispose() }
    }
    if (-not $Process.HasExited -and -not $Process.WaitForExit(10000)) {
        $detail = if ($primaryFailure) { " Primary termination error: $primaryFailure" } else { "" }
        throw "Gate step $Stage retained a live process tree after verified cleanup.$detail"
    }
}

function Expand-BindingValue {
    param([Parameter(Mandatory = $true)][string]$Value)
    $expanded = $Value
    $replacements = [ordered]@{
        "{repository_root}" = $repositoryRoot
        "{gate_output}" = $gateOutput
        "{campaign_root}" = $campaignRoot
        "{candidate_commit}" = $candidateCommit
        "{plan_sha256}" = $planSha256
        "{seed}" = [string]$seed
        "{gate_id}" = $GateId
    }
    foreach ($entry in $replacements.GetEnumerator()) {
        $expanded = $expanded.Replace([string]$entry.Key, [string]$entry.Value)
    }
    return $expanded
}

$candidateCommit = if ($env:QPLS_MULTIMOD_CANDIDATE_COMMIT) {
    $env:QPLS_MULTIMOD_CANDIDATE_COMMIT
} else {
    (& git -C $repositoryRoot rev-parse HEAD 2>$null).Trim()
}
if ($candidateCommit -cnotmatch '^[a-f0-9]{40}$') {
    throw "QPLS_MULTIMOD_CANDIDATE_COMMIT must be an exact lowercase commit SHA."
}
$candidateVersion = [string]$env:QPLS_MULTIMOD_CANDIDATE_VERSION
if ($candidateVersion -cne [string]$plan.candidate.final_version) {
    throw "QPLS_MULTIMOD_CANDIDATE_VERSION must equal the frozen final candidate version $($plan.candidate.final_version)."
}

$planSha256 = if ($env:QPLS_MULTIMOD_PLAN_SHA256) {
    $env:QPLS_MULTIMOD_PLAN_SHA256
} else {
    Get-LowerSha256 -Path $planPath
}
if ($planSha256 -cnotmatch '^[a-f0-9]{64}$') {
    throw "QPLS_MULTIMOD_PLAN_SHA256 must be a lowercase SHA-256."
}
$actualPlanSha256 = Get-LowerSha256 -Path $planPath
if ($planSha256 -cne $actualPlanSha256) {
    throw "QPLS_MULTIMOD_PLAN_SHA256 does not match the frozen plan bytes."
}
$actualBindingSha256 = Get-LowerSha256 -Path $catalogPath
$bindingSha256 = if ($env:QPLS_MULTIMOD_BINDING_SHA256) {
    $env:QPLS_MULTIMOD_BINDING_SHA256
} else {
    $actualBindingSha256
}
if ($bindingSha256 -cnotmatch '^[a-f0-9]{64}$' -or $bindingSha256 -cne $actualBindingSha256) {
    throw "QPLS_MULTIMOD_BINDING_SHA256 does not match the reviewed gate catalog bytes."
}

$seed = if ($env:QPLS_MULTIMOD_SEED) {
    [UInt64]::Parse($env:QPLS_MULTIMOD_SEED, [Globalization.CultureInfo]::InvariantCulture)
} else {
    [UInt64]$catalog.campaign_seed
}
$campaignRoot = if ($env:QPLS_MULTIMOD_CAMPAIGN_ROOT) {
    [IO.Path]::GetFullPath($env:QPLS_MULTIMOD_CAMPAIGN_ROOT)
} else {
    [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) "qpls-multimod-standalone"))
}
$gateOutput = if ($env:QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY) {
    [IO.Path]::GetFullPath($env:QPLS_MULTIMOD_GATE_OUTPUT_DIRECTORY)
} else {
    Join-Path $campaignRoot $GateId
}
New-Item -ItemType Directory -Path $gateOutput -Force | Out-Null

$inputRows = [System.Collections.Generic.List[object]]::new()
foreach ($declared in @($binding.input_artifacts)) {
    $relative = [string]$declared
    $absolute = [IO.Path]::GetFullPath((Join-Path $repositoryRoot $relative))
    $repositoryPrefix = $repositoryRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $absolute.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Gate input escapes the repository: $relative"
    }
    if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) {
        throw "Gate input is missing: $relative"
    }
    $inputRows.Add([ordered]@{ path = $relative.Replace('\', '/'); sha256 = Get-LowerSha256 -Path $absolute })
}
$inputMaterial = [ordered]@{
    schema_version = 1
    gate_id = $GateId
    candidate_commit_sha = $candidateCommit
    candidate_version = $candidateVersion
    plan_sha256 = $planSha256
    binding_sha256 = $bindingSha256
    seed = $seed
    binding = $binding
    artifacts = @($inputRows)
}
$inputDigest = Get-TextSha256 -Text ($inputMaterial | ConvertTo-Json -Depth 100 -Compress)

$stepReceipts = [System.Collections.Generic.List[object]]::new()
$failedStep = $null
$failureSignature = $null

foreach ($step in @($binding.steps)) {
    if ($failedStep) { break }
    $stepId = [string]$step.step_id
    $stdoutPath = Join-Path $gateOutput "$stepId.stdout.log"
    $stderrPath = Join-Path $gateOutput "$stepId.stderr.log"
    $executableName = Expand-BindingValue -Value ([string]$step.executable)
    $arguments = @($step.arguments | ForEach-Object { Expand-BindingValue -Value ([string]$_) })
    $launch = Resolve-ExactProcessLaunch -Executable $executableName -Arguments $arguments
    $resolvedExecutable = [string]$launch.ResolvedExecutable
    $isCargoTest = [bool]$step.uses_cargo -and @($arguments | Where-Object { $_ -ceq "test" }).Count -gt 0
    $requiredRustTests = @()
    if ($step.PSObject.Properties.Name -ccontains "required_test_identities") {
        $requiredRustTests = @($step.required_test_identities | ForEach-Object { [string]$_ })
    }
    $declaredOutputPaths = @(
        $step.expected_outputs |
            ForEach-Object { [IO.Path]::GetFullPath((Expand-BindingValue -Value ([string]$_))) }
    )
    $remainingGateMilliseconds = [long][Math]::Floor(
        ($gateMaximumSeconds - $gateClock.Elapsed.TotalSeconds) * 1000.0
    )
    if ($remainingGateMilliseconds -lt 1) {
        $stepStarted = (Get-Date).ToUniversalTime()
        Write-TextAtomic -Path $stdoutPath -Text ""
        Write-TextAtomic -Path $stderrPath -Text ""
        $prestartExpectedOutputs = [System.Collections.Generic.List[object]]::new()
        $prestartMissingOutputs = [System.Collections.Generic.List[string]]::new()
        foreach ($resolvedOutput in $declaredOutputPaths) {
            if (Test-Path -LiteralPath $resolvedOutput -PathType Leaf) {
                $prestartExpectedOutputs.Add([ordered]@{
                    path = $resolvedOutput
                    sha256 = Get-LowerSha256 -Path $resolvedOutput
                    size = [long](Get-Item -LiteralPath $resolvedOutput).Length
                })
            }
            else {
                $prestartMissingOutputs.Add($resolvedOutput)
            }
        }
        $stepCompleted = (Get-Date).ToUniversalTime()
        $stepReceipts.Add([ordered]@{
            step_id = $stepId
            executable = $resolvedExecutable
            arguments = $arguments
            uses_cargo = [bool]$step.uses_cargo
            exit_code = -1
            started_at_utc = $stepStarted.ToString("o")
            completed_at_utc = $stepCompleted.ToString("o")
            duration_ms = [long][math]::Round(($stepCompleted - $stepStarted).TotalMilliseconds)
            maximum_seconds = [long]$step.maximum_seconds
            effective_maximum_seconds = 0
            gate_budget_limited = $true
            budget_exceeded = $true
            timeout_terminated = $false
            rust_tests_passed = if ($isCargoTest) { 0 } else { $null }
            rust_tests_failed = if ($isCargoTest) { 0 } else { $null }
            rust_tests_executed = if ($isCargoTest) { 0 } else { $null }
            empty_cargo_test_rejected = $false
            required_test_identities = $requiredRustTests
            missing_required_test_identities = $requiredRustTests
            status = "failed"
            stdout_path = $stdoutPath
            stdout_sha256 = Get-LowerSha256 -Path $stdoutPath
            stdout_size = 0
            stderr_path = $stderrPath
            stderr_sha256 = Get-LowerSha256 -Path $stderrPath
            stderr_size = 0
            expected_outputs = @($prestartExpectedOutputs)
            missing_outputs = @($prestartMissingOutputs)
            launch_kind = "not_started_gate_budget_exhausted"
            effective_executable = [string]$launch.FileName
            effective_arguments = @($launch.Arguments)
            gate_deadline_checked_after_evidence_hashing = $true
        })
        $failedStep = $stepId
        $failureSignature = "$stepId`:gate_budget_before_start:$(Get-TextSha256 -Text '')"
        break
    }

    if ([bool]$step.uses_cargo) {
        $otherCargo = @(Get-Process -Name cargo -ErrorAction SilentlyContinue)
        if ($otherCargo.Count -gt 0) {
            throw "Gate $GateId refused to start $stepId because another Cargo process is active."
        }
    }

    $stepStarted = (Get-Date).ToUniversalTime()
    $declaredMaximumMilliseconds = [long]$step.maximum_seconds * 1000L
    $maximumMilliseconds = [Math]::Min($declaredMaximumMilliseconds, $remainingGateMilliseconds)
    $gateBudgetLimited = $maximumMilliseconds -lt $declaredMaximumMilliseconds
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = [string]$launch.FileName
    $startInfo.WorkingDirectory = $repositoryRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in @($launch.Arguments)) {
        [void]$startInfo.ArgumentList.Add([string]$argument)
    }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stdoutCopy = $null
    $stderrCopy = $null
    $processStarted = $false
    $timedOut = $false
    $exitCode = -1
    $stepClock = [Diagnostics.Stopwatch]::StartNew()
    try {
        [void]$process.Start()
        $processStarted = $true
        $stdoutCopy = $process.StandardOutput.ReadToEndAsync()
        $stderrCopy = $process.StandardError.ReadToEndAsync()
        $timedOut = -not $process.WaitForExit([int]$maximumMilliseconds)
        if ($timedOut) {
            Stop-VerifiedProcessTree -Process $process -Stage $stepId
        }
        if (-not $process.WaitForExit(10000)) {
            Stop-VerifiedProcessTree -Process $process -Stage $stepId
            throw "Gate step $stepId did not finalize after exit."
        }
        $exitCode = [int]$process.ExitCode
    }
    catch {
        if ($processStarted -and -not $process.HasExited) {
            Stop-VerifiedProcessTree -Process $process -Stage $stepId
        }
        throw
    }
    finally {
        if ($null -ne $stdoutCopy -and $null -ne $stderrCopy) {
            $tasks = [Threading.Tasks.Task[]]@($stdoutCopy, $stderrCopy)
            if (-not [Threading.Tasks.Task]::WaitAll($tasks, 30000)) {
                if ($processStarted -and -not $process.HasExited) {
                    Stop-VerifiedProcessTree -Process $process -Stage $stepId
                }
                throw "Timed out draining redirected logs for gate step $stepId."
            }
            Write-TextAtomic -Path $stdoutPath -Text ([string]$stdoutCopy.Result)
            Write-TextAtomic -Path $stderrPath -Text ([string]$stderrCopy.Result)
        }
        else {
            Write-TextAtomic -Path $stdoutPath -Text ""
            Write-TextAtomic -Path $stderrPath -Text ""
        }
        $process.Dispose()
    }
    $stepCompleted = (Get-Date).ToUniversalTime()
    $durationMs = [long]$stepClock.ElapsedMilliseconds
    $stdoutText = if (Test-Path -LiteralPath $stdoutPath) { Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 } else { "" }
    $stderrText = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw -Encoding UTF8 } else { "" }
    $rustTestsPassed = 0
    $rustTestsFailed = 0
    if ($isCargoTest) {
        foreach ($match in [regex]::Matches($stdoutText + "`n" + $stderrText, 'test result:\s+(?:ok|FAILED)\.\s+([0-9]+) passed;\s+([0-9]+) failed')) {
            $rustTestsPassed += [int]$match.Groups[1].Value
            $rustTestsFailed += [int]$match.Groups[2].Value
        }
    }
    $rustTestsExecuted = $rustTestsPassed + $rustTestsFailed
    $emptyCargoTest = $isCargoTest -and $exitCode -eq 0 -and $rustTestsExecuted -lt 1
    $executedRustTests = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    if ($isCargoTest) {
        foreach ($match in [regex]::Matches($stdoutText + "`n" + $stderrText, '(?m)^test\s+([^\s]+)\s+\.\.\.\s+(?:ok|FAILED)\s*$')) {
            [void]$executedRustTests.Add($match.Groups[1].Value)
        }
    }
    $missingRequiredRustTests = @(
        $requiredRustTests | Where-Object { -not $executedRustTests.Contains($_) }
    )
    $expectedOutputs = [System.Collections.Generic.List[object]]::new()
    $missingOutputs = [System.Collections.Generic.List[string]]::new()
    foreach ($resolvedOutput in $declaredOutputPaths) {
        if (Test-Path -LiteralPath $resolvedOutput -PathType Leaf) {
            $expectedOutputs.Add([ordered]@{
                path = $resolvedOutput
                sha256 = Get-LowerSha256 -Path $resolvedOutput
                size = [long](Get-Item -LiteralPath $resolvedOutput).Length
            })
        } else {
            $missingOutputs.Add($resolvedOutput)
        }
    }
    # This check intentionally occurs after log reads and output hashing; those
    # are part of the bounded gate invocation, not free post-processing.
    $budgetExceeded = $timedOut -or $durationMs -gt $maximumMilliseconds -or
        $gateClock.Elapsed.TotalSeconds -ge $gateMaximumSeconds
    $passed = $exitCode -eq 0 -and -not $budgetExceeded -and -not $emptyCargoTest -and $missingRequiredRustTests.Count -eq 0 -and $missingOutputs.Count -eq 0
    $stepReceipts.Add([ordered]@{
        step_id = $stepId
        executable = $resolvedExecutable
        arguments = $arguments
        uses_cargo = [bool]$step.uses_cargo
        exit_code = $exitCode
        started_at_utc = $stepStarted.ToString("o")
        completed_at_utc = $stepCompleted.ToString("o")
        duration_ms = $durationMs
        maximum_seconds = [long]$step.maximum_seconds
        effective_maximum_seconds = [math]::Round(($maximumMilliseconds / 1000.0), 3)
        gate_budget_limited = $gateBudgetLimited
        budget_exceeded = $budgetExceeded
        timeout_terminated = $timedOut
        rust_tests_passed = if ($isCargoTest) { $rustTestsPassed } else { $null }
        rust_tests_failed = if ($isCargoTest) { $rustTestsFailed } else { $null }
        rust_tests_executed = if ($isCargoTest) { $rustTestsExecuted } else { $null }
        empty_cargo_test_rejected = $emptyCargoTest
        required_test_identities = $requiredRustTests
        missing_required_test_identities = $missingRequiredRustTests
        status = if ($passed) { "passed" } else { "failed" }
        stdout_path = $stdoutPath
        stdout_sha256 = Get-LowerSha256 -Path $stdoutPath
        stdout_size = [long](Get-Item -LiteralPath $stdoutPath).Length
        stderr_path = $stderrPath
        stderr_sha256 = Get-LowerSha256 -Path $stderrPath
        stderr_size = [long](Get-Item -LiteralPath $stderrPath).Length
        expected_outputs = @($expectedOutputs)
        missing_outputs = @($missingOutputs)
        launch_kind = [string]$launch.LaunchKind
        effective_executable = [string]$launch.FileName
        effective_arguments = @($launch.Arguments)
        gate_deadline_checked_after_evidence_hashing = $true
    })
    if (-not $passed) {
        $failedStep = $stepId
        $tail = ($stderrText + "`n" + $stdoutText)
        if ($tail.Length -gt 131072) { $tail = $tail.Substring($tail.Length - 131072) }
        $failureDigest = Get-TextSha256 -Text $tail
        $reason = if ($timedOut -and $gateBudgetLimited) { "gate_timeout" } elseif ($timedOut) { "timeout" } elseif ($budgetExceeded) { "budget" } elseif ($emptyCargoTest) { "zero_tests" } elseif ($exitCode -ne 0) { "exit_$exitCode" } elseif ($missingRequiredRustTests.Count -gt 0) { "missing_required_test" } elseif ($missingOutputs.Count -gt 0) { "missing_output" } else { "exit_$exitCode" }
        $failureSignature = "$stepId`:$reason`:$failureDigest"
    }
}

if (-not $failedStep -and $gateClock.Elapsed.TotalSeconds -ge $gateMaximumSeconds) {
    $lastReceipt = $stepReceipts[$stepReceipts.Count - 1]
    $lastReceipt.status = "failed"
    $lastReceipt.budget_exceeded = $true
    $failedStep = [string]$lastReceipt.step_id
    $failureSignature = "$failedStep`:gate_postprocess_budget:$(Get-TextSha256 -Text '')"
}

$completedAt = (Get-Date).ToUniversalTime()
$receipt = [ordered]@{
    schema_version = 1
    receipt_kind = "qpls_multimod_gate_receipt_v1"
    gate_id = $GateId
    candidate_commit_sha = $candidateCommit
    candidate_version = $candidateVersion
    plan_sha256 = $planSha256
    binding_set_id = [string]$catalog.binding_set_id
    binding_sha256 = $bindingSha256
    coverage_binding_state = "executed_real_commands"
    seed = $seed
    input_digest = $inputDigest
    profiles = @($binding.profiles)
    covered_evidence_cells = @($binding.covered_evidence_cells)
    probable_root_component = [string]$binding.probable_root_component
    status = if ($failedStep) { "failed" } else { "passed" }
    failure_step = $failedStep
    failure_signature = $failureSignature
    started_at_utc = $startedAt.ToString("o")
    completed_at_utc = $completedAt.ToString("o")
    duration_ms = [long][math]::Round(($completedAt - $startedAt).TotalMilliseconds)
    gate_maximum_seconds = $gateMaximumSeconds
    cleanup_reserve_before_campaign_timeout_seconds = 120
    steps = @($stepReceipts)
}
$receiptPath = Join-Path $gateOutput "gate_receipt.json"
Write-JsonAtomic -Path $receiptPath -Value $receipt
$receipt | ConvertTo-Json -Depth 100
if ($failedStep) { exit 1 }
