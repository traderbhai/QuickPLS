[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][UInt64]$Seed,
    [string]$WorkRoot,
    [ValidateRange(1, 1800)][int]$PerAtomicProcessTimeoutSeconds = 1800,
    [ValidateRange(1, 6480)][int]$InternalTimeoutSeconds = 6480
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [IO.Path]::GetFullPath($Output)
$outputDirectory = Split-Path -Parent $outputPath
$resolvedWorkRoot = if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    Join-Path $outputDirectory "maximum-profile-performance-v2-work"
} else {
    [IO.Path]::GetFullPath($WorkRoot)
}
$publicationReserveSeconds = 120
$executionDeadlineSeconds = $InternalTimeoutSeconds - $publicationReserveSeconds
if ($executionDeadlineSeconds -lt 1) {
    throw "InternalTimeoutSeconds must leave the frozen $publicationReserveSeconds-second publication reserve."
}
$logDirectory = Join-Path $resolvedWorkRoot "logs"
$buildRoot = Join-Path $resolvedWorkRoot "build"
$buildAttemptPath = Join-Path $buildRoot "build-attempt.json"
$buildCommandIdentity = @(
    "cargo", "build", "--release", "--locked", "-p", "qpls-runner",
    "--example", "multimod_mga_qualification_v1",
    "--example", "multimod_heterogeneity_qualification_v2",
    "--example", "multimod_maximum_profiles_performance_v1"
)
$mgaRoot = Join-Path $resolvedWorkRoot "mga"
$mgaCellDirectory = Join-Path $mgaRoot "cells"
$mgaCacheDirectory = Join-Path $mgaRoot "production-cache"
$mgaPlanPath = Join-Path $mgaRoot "cell-plan.json"
$heterogeneityRoot = Join-Path $resolvedWorkRoot "heterogeneity"
$heterogeneityShardDirectory = Join-Path $heterogeneityRoot "shards"
$heterogeneityPlanPath = Join-Path $heterogeneityRoot "shard-plan.json"
$maximumRoot = Join-Path $resolvedWorkRoot "maximum"
$maximumOutput = Join-Path $maximumRoot "result.json"
$topologyPlanPath = Join-Path $resolvedWorkRoot "performance-topology-plan.json"
$topologyOutput = Join-Path $resolvedWorkRoot "performance-execution-topology.json"
$verificationOutput = Join-Path $resolvedWorkRoot "performance-output-verification.json"
$topologyTool = Join-Path $PSScriptRoot "multimod_performance_topology_v2.py"
$mgaCheckpointTool = Join-Path $PSScriptRoot "multimod_mga_shards_v1.py"
$heterogeneityCheckpointTool = Join-Path $PSScriptRoot "multimod_heterogeneity_shards_v2.py"
$verifier = Join-Path $PSScriptRoot "verify_multimod_performance_profiles_v2.py"
$cargoLockPath = Join-Path $repositoryRoot "Cargo.lock"
$mgaBinary = Join-Path $repositoryRoot "target/release/examples/multimod_mga_qualification_v1.exe"
$heterogeneityBinary = Join-Path $repositoryRoot "target/release/examples/multimod_heterogeneity_qualification_v2.exe"
$maximumBinary = Join-Path $repositoryRoot "target/release/examples/multimod_maximum_profiles_performance_v1.exe"
$campaignClock = [Diagnostics.Stopwatch]::StartNew()
$mgaCellId = "mga-general-20-groups"
$budgets = [ordered]@{
    mga_20_groups_190_pairs = [ordered]@{ maximum_seconds = 1800; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 2GB }
    heterogeneity_locked_p23 = [ordered]@{ maximum_seconds = 3720; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 4GB }
    conditional_sidecar_resume = [ordered]@{ maximum_seconds = 1800; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 2GB }
}

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-JsonAtomic([string]$Path, $Value) {
    $temporary = "$Path.$([Guid]::NewGuid().ToString('N')).tmp"
    [IO.File]::WriteAllText($temporary, (($Value | ConvertTo-Json -Depth 100) + "`n"), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Assert-BaselineEnvironment {
    $metamorphism = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_METAMORPHISM_V1")
    $workers = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_WORKERS_V1")
    $compact = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1")
    $signColumns = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_SIGN_COLUMNS_V1")
    if ($null -ne $metamorphism -and $metamorphism -ne "baseline") { throw "Performance qualification requires baseline metamorphism." }
    if ($null -ne $workers -and $workers -ne "1") { throw "Performance qualification requires the frozen one-worker scientific environment." }
    if ($null -ne $compact -or $null -ne $signColumns) { throw "Performance qualification rejects compact/sign metamorphic overrides." }
    if ($Seed -ne 42) { throw "Qualification-scale maximum-profile performance requires seed 42." }
}

function Get-RemainingSeconds {
    $remaining = [Math]::Floor($executionDeadlineSeconds - $campaignClock.Elapsed.TotalSeconds)
    if ($remaining -lt 1) { throw "Performance orchestration reached its $executionDeadlineSeconds-second execution cap; the publication reserve remains protected and verified checkpoints remain resumable." }
    return [int]$remaining
}

function Assert-PublicationDeadline {
    if ($campaignClock.Elapsed.TotalSeconds -ge $InternalTimeoutSeconds) {
        throw "Performance orchestration reached its $InternalTimeoutSeconds-second publication cutoff; no final report may be published."
    }
}

function Get-DescendantRows([int[]]$RootProcessIds) {
    $rows = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId, ParentProcessId, WorkingSetSize)
    if ($rows.Count -eq 0) { throw "Win32_Process returned no rows; process-tree performance evidence is unavailable." }
    $selected = [Collections.Generic.HashSet[uint32]]::new()
    foreach ($rootProcessId in $RootProcessIds) { [void]$selected.Add([uint32]$rootProcessId) }
    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($row in $rows) {
            if ($selected.Contains([uint32]$row.ParentProcessId) -and $selected.Add([uint32]$row.ProcessId)) { $changed = $true }
        }
    }
    return @($rows | Where-Object { $selected.Contains([uint32]$_.ProcessId) })
}

function Start-ChildProcess {
    param(
        [Parameter(Mandatory = $true)][string]$ProcessIdentity,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath
    )
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FileName
    $startInfo.WorkingDirectory = $repositoryRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) { [void]$startInfo.ArgumentList.Add([string]$argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    return [pscustomobject]@{
        ProcessIdentity = $ProcessIdentity
        Process = $process
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
        StdoutCopy = $process.StandardOutput.ReadToEndAsync()
        StderrCopy = $process.StandardError.ReadToEndAsync()
        LogsSaved = $false
        PeakWorkingSetBytes = [long]0
    }
}

function Update-ChildPeakWorkingSets {
    param([Parameter(Mandatory = $true)][object[]]$Jobs)
    foreach ($job in $Jobs) {
        try {
            $job.Process.Refresh()
            $candidatePeak = [long]$job.Process.PeakWorkingSet64
            if ($candidatePeak -gt [long]$job.PeakWorkingSetBytes) {
                $job.PeakWorkingSetBytes = $candidatePeak
            }
        } catch {
            throw "Could not capture PeakWorkingSet64 for $($job.ProcessIdentity): $($_.Exception.Message)"
        }
    }
}

function Save-ChildLogs {
    param([Parameter(Mandatory = $true)]$Job)
    if ($Job.LogsSaved) { return }
    $tasks = [Threading.Tasks.Task[]]@($Job.StdoutCopy, $Job.StderrCopy)
    if (-not [Threading.Tasks.Task]::WaitAll($tasks, 10000)) { throw "Timed out draining redirected output for $($Job.ProcessIdentity)." }
    [IO.File]::WriteAllText($Job.StdoutPath, $Job.StdoutCopy.Result, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($Job.StderrPath, $Job.StderrCopy.Result, [Text.UTF8Encoding]::new($false))
    $Job.LogsSaved = $true
}

function Stop-ChildGroup {
    param([Parameter(Mandatory = $true)][object[]]$Jobs, [Parameter(Mandatory = $true)][string]$Reason)
    $knownProcessIds = [Collections.Generic.HashSet[int]]::new()
    try {
        $rows = @(Get-DescendantRows -RootProcessIds @($Jobs | ForEach-Object { $_.Process.Id }))
        foreach ($row in $rows) { [void]$knownProcessIds.Add([int]$row.ProcessId) }
    } catch {
        foreach ($job in $Jobs) { [void]$knownProcessIds.Add([int]$job.Process.Id) }
    }
    foreach ($job in $Jobs) {
        if (-not $job.Process.HasExited) {
            try { $job.Process.Kill($true) } catch { & taskkill.exe /PID $job.Process.Id /T /F *> $null }
        }
    }
    foreach ($job in $Jobs) {
        if (-not $job.Process.WaitForExit(10000)) {
            & taskkill.exe /PID $job.Process.Id /T /F *> $null
            if (-not $job.Process.WaitForExit(10000)) { throw "Could not terminate $($job.ProcessIdentity) after $Reason." }
        }
    }
    foreach ($knownProcessId in $knownProcessIds) {
        if ($null -ne (Get-Process -Id $knownProcessId -ErrorAction SilentlyContinue)) { Stop-Process -Id $knownProcessId -Force -ErrorAction Stop }
    }
    $terminationDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $survivors = @($knownProcessIds | Where-Object {
            $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
        })
        if ($survivors.Count -eq 0) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $terminationDeadline)
    if ($survivors.Count -ne 0) {
        throw "Process-tree cleanup left surviving process IDs after $Reason`: $($survivors -join ', ')."
    }
    foreach ($job in $Jobs) { Save-ChildLogs -Job $job }
}

function Invoke-MeasuredProcessGroup {
    param(
        [Parameter(Mandatory = $true)][string]$StageId,
        [Parameter(Mandatory = $true)][object[]]$ProcessSpecs,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [long]$MaximumPeakWorkingSetBytes = 12GB
    )
    $effectiveTimeout = [Math]::Min([Math]::Min($TimeoutSeconds, $PerAtomicProcessTimeoutSeconds), (Get-RemainingSeconds))
    $jobs = [Collections.Generic.List[object]]::new()
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $sampledConcurrentPeak = [long]0
    $samples = 0
    try {
        foreach ($spec in $ProcessSpecs) {
            $jobs.Add((Start-ChildProcess -ProcessIdentity ([string]$spec.ProcessIdentity) -FileName ([string]$spec.FileName) `
                -Arguments @($spec.Arguments) -StdoutPath ([string]$spec.StdoutPath) -StderrPath ([string]$spec.StderrPath)))
        }
        Update-ChildPeakWorkingSets -Jobs @($jobs)
        $rows = @(Get-DescendantRows -RootProcessIds @($jobs | ForEach-Object { $_.Process.Id }))
        $initialWorkingSet = [long]0
        foreach ($row in $rows) {
            if ($null -eq $row.WorkingSetSize) { throw "$StageId has a process-tree row without WorkingSetSize." }
            $initialWorkingSet += [long]$row.WorkingSetSize
        }
        if ($initialWorkingSet -gt $sampledConcurrentPeak) { $sampledConcurrentPeak = $initialWorkingSet }
        $samples += 1
        while (@($jobs | Where-Object { -not $_.Process.HasExited }).Count -gt 0) {
            if ($clock.Elapsed.TotalSeconds -ge $effectiveTimeout -or $campaignClock.Elapsed.TotalSeconds -ge $executionDeadlineSeconds) {
                Stop-ChildGroup -Jobs @($jobs) -Reason "bounded_timeout"
                throw "$StageId exceeded its bounded $effectiveTimeout-second atomic-process window."
            }
            Update-ChildPeakWorkingSets -Jobs @($jobs)
            $rows = @(Get-DescendantRows -RootProcessIds @($jobs | ForEach-Object { $_.Process.Id }))
            $workingSet = [long]0
            foreach ($row in $rows) {
                if ($null -eq $row.WorkingSetSize) { throw "$StageId has a process-tree row without WorkingSetSize." }
                $workingSet += [long]$row.WorkingSetSize
            }
            if ($workingSet -gt $sampledConcurrentPeak) { $sampledConcurrentPeak = $workingSet }
            $samples += 1
            Start-Sleep -Milliseconds 250
            foreach ($job in $jobs) { $job.Process.Refresh() }
        }
        foreach ($job in $jobs) {
            if (-not $job.Process.WaitForExit(10000)) {
                Stop-ChildGroup -Jobs @($jobs) -Reason "exit_wait_timeout"
                throw "$StageId did not finalize after process exit."
            }
            Save-ChildLogs -Job $job
        }
        Update-ChildPeakWorkingSets -Jobs @($jobs)
    } catch {
        if (@($jobs | Where-Object { -not $_.Process.HasExited }).Count -gt 0) { Stop-ChildGroup -Jobs @($jobs) -Reason "exception_cleanup" }
        throw
    } finally { $clock.Stop() }
    $perProcessPeaks = @(
        foreach ($job in $jobs) {
            [ordered]@{
                process_identity = [string]$job.ProcessIdentity
                peak_working_set_bytes = [long]$job.PeakWorkingSetBytes
            }
        }
    )
    if (@($perProcessPeaks | Where-Object { [long]$_.peak_working_set_bytes -le 0 }).Count -ne 0) {
        throw "$StageId produced no usable PeakWorkingSet64 evidence for one or more root processes."
    }
    $perProcessPeakSum = [long](($perProcessPeaks | Measure-Object -Property peak_working_set_bytes -Sum).Sum)
    $peak = [Math]::Max($sampledConcurrentPeak, $perProcessPeakSum)
    if ($samples -lt 1 -or $peak -le 0 -or $peak -gt $MaximumPeakWorkingSetBytes) { throw "$StageId conservative combined peak working set $peak is outside its predeclared bound." }
    $exitCodes = @($jobs | ForEach-Object { [int]$_.Process.ExitCode })
    return [pscustomobject]@{
        StageId = $StageId
        ExitCodes = $exitCodes
        WallTimeMilliseconds = [long]$clock.Elapsed.TotalMilliseconds
        PeakWorkingSetBytes = $peak
        SampledConcurrentPeakWorkingSetBytes = $sampledConcurrentPeak
        PerProcessPeakWorkingSetBytes = $perProcessPeaks
        WorkingSetSampleCount = $samples
        ProcessCount = $jobs.Count
    }
}

function Invoke-BoundedCommand {
    param(
        [Parameter(Mandatory = $true)][string]$StageId,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSeconds = 120
    )
    $identity = [Guid]::NewGuid().ToString("N")
    $measurement = Invoke-MeasuredProcessGroup -StageId $StageId -TimeoutSeconds $TimeoutSeconds -ProcessSpecs @([pscustomobject]@{
        ProcessIdentity = $StageId
        FileName = $FileName
        Arguments = $Arguments
        StdoutPath = Join-Path $logDirectory "_$StageId.$identity.stdout.log"
        StderrPath = Join-Path $logDirectory "_$StageId.$identity.stderr.log"
    })
    return [int]$measurement.ExitCodes[0]
}

function Publish-DeterministicOutput {
    param([string]$TemporaryPath, [string]$DestinationPath, [string]$Identity)
    if (-not (Test-Path -LiteralPath $TemporaryPath -PathType Leaf)) { throw "$Identity did not create its declared output." }
    if (Test-Path -LiteralPath $DestinationPath -PathType Leaf) {
        if ((Get-LowerSha256 $TemporaryPath) -ne (Get-LowerSha256 $DestinationPath)) { throw "$Identity differs from its resumable frozen output." }
        Remove-Item -LiteralPath $TemporaryPath -Force
    } else { Move-Item -LiteralPath $TemporaryPath -Destination $DestinationPath }
}

function Invoke-PythonTool {
    param([string]$StageId, [string]$Tool, [string[]]$Arguments)
    return Invoke-BoundedCommand -StageId $StageId -FileName $script:python -Arguments (@($Tool) + $Arguments) -TimeoutSeconds 120
}

function Test-PerformanceCheckpoint([string]$StageId) {
    $exitCode = Invoke-PythonTool -StageId "verify-performance-$StageId" -Tool $topologyTool -Arguments @(
        "verify", "--plan", $topologyPlanPath, "--work-root", $resolvedWorkRoot,
        "--receipt-root", $resolvedWorkRoot, "--stage-id", $StageId
    )
    if ($exitCode -eq 0) { return $true }
    if ($exitCode -eq 3) { return $false }
    throw "Performance checkpoint $StageId is incomplete, stale, or altered."
}

function Write-And-SealPerformanceMeasurement {
    param([string]$StageId, $Measurement)
    $plan = Get-Content -LiteralPath $topologyPlanPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    $stage = @($plan.stages | Where-Object { [string]$_.stage_id -eq $StageId })
    if ($stage.Count -ne 1) { throw "Topology stage is absent or duplicated: $StageId" }
    $stage = $stage[0]
    if (@($Measurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "$StageId returned a nonzero producer exit code." }
    $temporary = Join-Path $resolvedWorkRoot ".measurement.$StageId.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $executableIdentity = $plan.executables.PSObject.Properties[[string]$stage.executable_id].Value
    Write-JsonAtomic -Path $temporary -Value ([ordered]@{
        schema_version = 2
        suite_id = "qpls.v256.multimod.performance-stage-measurement.v2"
        stage_id = $StageId
        topology_plan_sha256 = Get-LowerSha256 $topologyPlanPath
        source_commit = [string]$plan.source_commit
        executable_sha256 = [string]$executableIdentity.sha256
        command_identities = @($stage.command_identities)
        wall_time_milliseconds = [long]$Measurement.WallTimeMilliseconds
        peak_working_set_bytes = [long]$Measurement.PeakWorkingSetBytes
        sampled_concurrent_peak_working_set_bytes = [long]$Measurement.SampledConcurrentPeakWorkingSetBytes
        per_process_peak_working_set_bytes = @($Measurement.PerProcessPeakWorkingSetBytes)
        working_set_sample_count = [int]$Measurement.WorkingSetSampleCount
        process_count = [int]$Measurement.ProcessCount
        process_exit_codes = @($Measurement.ExitCodes)
        atomic_timeout_seconds = [int]$stage.atomic_timeout_seconds
    })
    $exitCode = Invoke-PythonTool -StageId "seal-performance-$StageId" -Tool $topologyTool -Arguments @(
        "seal", "--plan", $topologyPlanPath, "--work-root", $resolvedWorkRoot,
        "--receipt-root", $resolvedWorkRoot, "--stage-id", $StageId,
        "--temporary-measurement", $temporary
    )
    if ($exitCode -ne 0) { throw "Performance receipt rejected stage $StageId." }
}

function Test-MgaScientificCheckpoint([string]$ExecutableSha256, [string]$SourceCommit, [string]$EnvironmentSha256) {
    $directory = Join-Path $mgaCellDirectory $mgaCellId
    if (-not (Test-Path -LiteralPath $directory)) { return $false }
    $exitCode = Invoke-PythonTool -StageId "verify-mga-science" -Tool $mgaCheckpointTool -Arguments @(
        "verify", "--plan", $mgaPlanPath, "--cell-dir", $mgaCellDirectory,
        "--cell-id", $mgaCellId, "--executable-sha256", $ExecutableSha256,
        "--source-commit", $SourceCommit, "--environment-sha256", $EnvironmentSha256
    )
    if ($exitCode -ne 0) { throw "Existing MGA scientific checkpoint is stale or altered." }
    return $true
}

function Test-HeterogeneityScientificCheckpoint([string]$ShardId, [string]$ExecutableSha256, [string]$SourceCommit) {
    $result = Join-Path $heterogeneityShardDirectory "$ShardId.json"
    $receipt = Join-Path $heterogeneityShardDirectory "$ShardId.receipt.json"
    $resultExists = Test-Path -LiteralPath $result -PathType Leaf
    $receiptExists = Test-Path -LiteralPath $receipt -PathType Leaf
    if (-not $resultExists -and -not $receiptExists) { return $false }
    if ($resultExists -ne $receiptExists) {
        $quarantineRoot = Join-Path $heterogeneityRoot "quarantine"
        $quarantine = Join-Path $quarantineRoot "$ShardId.$([Guid]::NewGuid().ToString('N'))"
        New-Item -ItemType Directory -Path $quarantine -Force | Out-Null
        if ($resultExists) { Move-Item -LiteralPath $result -Destination (Join-Path $quarantine "result.json") }
        if ($receiptExists) { Move-Item -LiteralPath $receipt -Destination (Join-Path $quarantine "receipt.json") }
        Write-JsonAtomic -Path (Join-Path $quarantine "quarantine-receipt.json") -Value ([ordered]@{
            schema_version = 1
            suite_id = "qpls.v256.multimod.performance-incomplete-heterogeneity-quarantine.v1"
            shard_id = $ShardId
            reason = "result_receipt_transaction_incomplete"
            result_was_present = $resultExists
            receipt_was_present = $receiptExists
            quarantined_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        })
        return $false
    }
    $exitCode = Invoke-PythonTool -StageId "verify-heterogeneity-$ShardId" -Tool $heterogeneityCheckpointTool -Arguments @(
        "verify", "--plan", $heterogeneityPlanPath, "--shard-dir", $heterogeneityShardDirectory,
        "--shard-id", $ShardId, "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
    )
    if ($exitCode -ne 0) { throw "Existing heterogeneity checkpoint is stale or altered: $ShardId" }
    return $true
}

function Publish-Or-CompareScientificResult {
    param([string]$TemporaryResult, [string]$PublishedResult, [bool]$AlreadyPublished, [scriptblock]$Seal)
    if (-not (Test-Path -LiteralPath $TemporaryResult -PathType Leaf)) { throw "Producer did not create its declared temporary result: $TemporaryResult" }
    if ($AlreadyPublished) {
        if ((Get-LowerSha256 $TemporaryResult) -ne (Get-LowerSha256 $PublishedResult)) { throw "A resumed producer result differs from its verified scientific checkpoint." }
        Remove-Item -LiteralPath $TemporaryResult -Force
    } else { & $Seal }
}

function Invoke-MgaStage([string]$ExecutableSha256, [string]$SourceCommit, [string]$EnvironmentSha256) {
    $stageId = "mga-general-20-groups"
    if (Test-PerformanceCheckpoint $stageId) { return }
    $alreadyPublished = Test-MgaScientificCheckpoint $ExecutableSha256 $SourceCommit $EnvironmentSha256
    $temporary = Join-Path $mgaRoot ".$mgaCellId.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $planSha256 = Get-LowerSha256 $mgaPlanPath
    $measurement = Invoke-MeasuredProcessGroup -StageId $stageId -TimeoutSeconds $PerAtomicProcessTimeoutSeconds -ProcessSpecs @([pscustomobject]@{
        ProcessIdentity = $mgaCellId
        FileName = $mgaBinary
        Arguments = @(
            "--output", $temporary, "--scale", "qualification", "--seed", $Seed.ToString(),
            "--cell", $mgaCellId, "--cache-root", $mgaCacheDirectory,
            "--plan-path", $mgaPlanPath, "--plan-sha256", $planSha256,
            "--source-commit", $SourceCommit, "--executable-sha256", $ExecutableSha256,
            "--environment-sha256", $EnvironmentSha256
        )
        StdoutPath = Join-Path $logDirectory "$stageId.stdout.log"
        StderrPath = Join-Path $logDirectory "$stageId.stderr.log"
    })
    if (@($measurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "The exact 20-group MGA performance cell failed." }
    $publishedResult = Join-Path (Join-Path $mgaCellDirectory $mgaCellId) "result.json"
    Publish-Or-CompareScientificResult -TemporaryResult $temporary -PublishedResult $publishedResult -AlreadyPublished $alreadyPublished -Seal {
        $exitCode = Invoke-PythonTool -StageId "seal-mga-science" -Tool $mgaCheckpointTool -Arguments @(
            "seal", "--plan", $mgaPlanPath, "--cell-dir", $mgaCellDirectory,
            "--cell-id", $mgaCellId, "--temporary-result", $temporary,
            "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit,
            "--environment-sha256", $EnvironmentSha256
        )
        if ($exitCode -ne 0) { throw "MGA scientific checkpoint sealing failed." }
    }
    Write-And-SealPerformanceMeasurement -StageId $stageId -Measurement $measurement
}

function Get-HeterogeneityArguments([string]$ShardId, [string]$TemporaryResult) {
    $arguments = @("--output", $TemporaryResult, "--scale", "qualification", "--seed", $Seed.ToString(), "--shard", $ShardId)
    if ($ShardId -ne "sentinel") { $arguments += @("--dependency", (Join-Path $heterogeneityShardDirectory "sentinel.json")) }
    if ($ShardId -like "bootstrap-*") {
        $arguments += @("--dependency", (Join-Path $heterogeneityShardDirectory "pos-destination-p23-discovery.json"))
    }
    return $arguments
}

function Seal-HeterogeneityScientificResult {
    param([string]$ShardId, [string]$TemporaryResult, [bool]$AlreadyPublished, [string]$ExecutableSha256, [string]$SourceCommit)
    $publishedResult = Join-Path $heterogeneityShardDirectory "$ShardId.json"
    Publish-Or-CompareScientificResult -TemporaryResult $TemporaryResult -PublishedResult $publishedResult -AlreadyPublished $AlreadyPublished -Seal {
        $exitCode = Invoke-PythonTool -StageId "seal-heterogeneity-$ShardId" -Tool $heterogeneityCheckpointTool -Arguments @(
            "seal", "--plan", $heterogeneityPlanPath, "--shard-dir", $heterogeneityShardDirectory,
            "--shard-id", $ShardId, "--temporary-result", $TemporaryResult,
            "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
        )
        if ($exitCode -ne 0) { throw "Heterogeneity scientific checkpoint sealing failed: $ShardId" }
    }
}

function Invoke-HeterogeneitySingleStage {
    param([string]$StageId, [string]$ShardId, [string]$ExecutableSha256, [string]$SourceCommit, [int]$TimeoutSeconds, [string]$LogStem)
    if (Test-PerformanceCheckpoint $StageId) { return }
    $alreadyPublished = Test-HeterogeneityScientificCheckpoint $ShardId $ExecutableSha256 $SourceCommit
    $temporary = Join-Path $heterogeneityRoot ".$ShardId.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $measurement = Invoke-MeasuredProcessGroup -StageId $StageId -TimeoutSeconds $TimeoutSeconds -ProcessSpecs @([pscustomobject]@{
        ProcessIdentity = $ShardId
        FileName = $heterogeneityBinary
        Arguments = Get-HeterogeneityArguments $ShardId $temporary
        StdoutPath = Join-Path $logDirectory "$LogStem.stdout.log"
        StderrPath = Join-Path $logDirectory "$LogStem.stderr.log"
    })
    if (@($measurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "Heterogeneity performance shard failed: $ShardId" }
    Seal-HeterogeneityScientificResult $ShardId $temporary $alreadyPublished $ExecutableSha256 $SourceCommit
    Write-And-SealPerformanceMeasurement -StageId $StageId -Measurement $measurement
}

function Invoke-HeterogeneityBootstrapPair([string]$ExecutableSha256, [string]$SourceCommit) {
    $stageId = "heterogeneity-p23-bootstrap-pair"
    if (Test-PerformanceCheckpoint $stageId) { return }
    $fimixId = "bootstrap-fimix-p23"
    $posId = "bootstrap-pos-destination-p23"
    $fimixPublished = Test-HeterogeneityScientificCheckpoint $fimixId $ExecutableSha256 $SourceCommit
    $posPublished = Test-HeterogeneityScientificCheckpoint $posId $ExecutableSha256 $SourceCommit
    $fimixTemporary = Join-Path $heterogeneityRoot ".$fimixId.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $posTemporary = Join-Path $heterogeneityRoot ".$posId.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $measurement = Invoke-MeasuredProcessGroup -StageId $stageId -TimeoutSeconds $PerAtomicProcessTimeoutSeconds -ProcessSpecs @(
        [pscustomobject]@{
            ProcessIdentity = $fimixId
            FileName = $heterogeneityBinary
            Arguments = Get-HeterogeneityArguments $fimixId $fimixTemporary
            StdoutPath = Join-Path $logDirectory "heterogeneity-bootstrap-fimix-p23.stdout.log"
            StderrPath = Join-Path $logDirectory "heterogeneity-bootstrap-fimix-p23.stderr.log"
        },
        [pscustomobject]@{
            ProcessIdentity = $posId
            FileName = $heterogeneityBinary
            Arguments = Get-HeterogeneityArguments $posId $posTemporary
            StdoutPath = Join-Path $logDirectory "heterogeneity-bootstrap-pos-p23.stdout.log"
            StderrPath = Join-Path $logDirectory "heterogeneity-bootstrap-pos-p23.stderr.log"
        }
    )
    if (@($measurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "The exact parallel FIMIX/POS P23 bootstrap pair failed." }
    Seal-HeterogeneityScientificResult $fimixId $fimixTemporary $fimixPublished $ExecutableSha256 $SourceCommit
    Seal-HeterogeneityScientificResult $posId $posTemporary $posPublished $ExecutableSha256 $SourceCommit
    Write-And-SealPerformanceMeasurement -StageId $stageId -Measurement $measurement
}

function Invoke-MaximumStage([string]$ExecutableSha256) {
    $stageId = "conditional-sidecar-resume"
    if (Test-PerformanceCheckpoint $stageId) { return }
    $alreadyPublished = Test-Path -LiteralPath $maximumOutput -PathType Leaf
    $temporary = Join-Path $maximumRoot ".maximum.$([Guid]::NewGuid().ToString('N')).tmp.json"
    $measurement = Invoke-MeasuredProcessGroup -StageId $stageId -TimeoutSeconds $PerAtomicProcessTimeoutSeconds -ProcessSpecs @([pscustomobject]@{
        ProcessIdentity = "maximum-profiles"
        FileName = $maximumBinary
        Arguments = @("--output", $temporary, "--seed", $Seed.ToString())
        StdoutPath = Join-Path $logDirectory "$stageId.stdout.log"
        StderrPath = Join-Path $logDirectory "$stageId.stderr.log"
    })
    if (@($measurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "The dedicated conditional/sidecar/resume maximum producer failed." }
    Publish-Or-CompareScientificResult -TemporaryResult $temporary -PublishedResult $maximumOutput -AlreadyPublished $alreadyPublished -Seal {
        Move-Item -LiteralPath $temporary -Destination $maximumOutput
    }
    Write-And-SealPerformanceMeasurement -StageId $stageId -Measurement $measurement
}

if (-not $IsWindows) { throw "Maximum-profile process-tree performance qualification is Windows-only." }
if (Test-Path -LiteralPath $outputPath) { throw "Performance qualification output already exists: $outputPath" }
foreach ($required in @($topologyTool, $mgaCheckpointTool, $heterogeneityCheckpointTool, $verifier)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required performance program is missing: $required" }
}
Assert-BaselineEnvironment
$dirtyState = @(& git -C $repositoryRoot status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) { throw "Could not inspect the candidate worktree." }
if ($dirtyState.Count -ne 0) { throw "Performance qualification requires the frozen clean candidate worktree." }
$sourceCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim().ToLowerInvariant()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') { throw "Could not resolve the exact source commit." }
$script:python = (Get-Command python -ErrorAction Stop).Source
New-Item -ItemType Directory -Path @(
    $outputDirectory, $resolvedWorkRoot, $logDirectory, $buildRoot, $mgaRoot, $mgaCellDirectory, $mgaCacheDirectory,
    $heterogeneityRoot, $heterogeneityShardDirectory, $maximumRoot
) -Force | Out-Null

if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -ne 0) { throw "Another Cargo process is active; the performance gate owns exactly one release build." }
$cargoExecutable = (Get-Command cargo -ErrorAction Stop).Source
$cargoExecutableSha256 = Get-LowerSha256 $cargoExecutable
$cargoLockSha256 = Get-LowerSha256 $cargoLockPath
$buildAttempt = $null
$buildMeasurement = $null
if (Test-Path -LiteralPath $buildAttemptPath -PathType Leaf) {
    $buildAttempt = Get-Content -LiteralPath $buildAttemptPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    if ([string]$buildAttempt.status -cne "completed") {
        throw "A prior release-build attempt did not complete; a second Cargo build is forbidden in the same candidate topology. Retain the marker for diagnosis and use a fresh performance work root."
    }
    if (
        [int]$buildAttempt.schema_version -ne 2 -or
        [string]$buildAttempt.suite_id -cne "qpls.v256.multimod.performance-release-build-attempt.v2" -or
        [int]$buildAttempt.attempt_count -ne 1 -or
        [string]$buildAttempt.source_commit -cne $sourceCommit -or
        [string]$buildAttempt.cargo_executable_sha256 -cne $cargoExecutableSha256 -or
        [string]$buildAttempt.cargo_lock_sha256 -cne $cargoLockSha256 -or
        @($buildAttempt.command_identity).Count -ne $buildCommandIdentity.Count
    ) { throw "The resumable release-build attempt identity is stale or altered." }
    for ($index = 0; $index -lt $buildCommandIdentity.Count; $index += 1) {
        if ([string]$buildAttempt.command_identity[$index] -cne [string]$buildCommandIdentity[$index]) {
            throw "The resumable release-build command identity differs at argument $index."
        }
    }
    $buildMeasurement = [pscustomobject]@{
        ExitCodes = @($buildAttempt.measurement.process_exit_codes)
        WallTimeMilliseconds = [long]$buildAttempt.measurement.wall_time_milliseconds
        PeakWorkingSetBytes = [long]$buildAttempt.measurement.peak_working_set_bytes
        SampledConcurrentPeakWorkingSetBytes = [long]$buildAttempt.measurement.sampled_concurrent_peak_working_set_bytes
        PerProcessPeakWorkingSetBytes = @($buildAttempt.measurement.per_process_peak_working_set_bytes)
        WorkingSetSampleCount = [int]$buildAttempt.measurement.working_set_sample_count
        ProcessCount = [int]$buildAttempt.measurement.process_count
    }
} else {
    $attemptId = [Guid]::NewGuid().ToString("N")
    Write-JsonAtomic -Path $buildAttemptPath -Value ([ordered]@{
        schema_version = 2
        suite_id = "qpls.v256.multimod.performance-release-build-attempt.v2"
        status = "started"
        attempt_id = $attemptId
        attempt_count = 1
        source_commit = $sourceCommit
        cargo_executable_sha256 = $cargoExecutableSha256
        cargo_lock_sha256 = $cargoLockSha256
        command_identity = $buildCommandIdentity
    })
    $buildMeasurement = Invoke-MeasuredProcessGroup -StageId "release-build" -TimeoutSeconds $PerAtomicProcessTimeoutSeconds -ProcessSpecs @([pscustomobject]@{
        ProcessIdentity = "release-build"
        FileName = $cargoExecutable
        Arguments = @($buildCommandIdentity[1..($buildCommandIdentity.Count - 1)])
        StdoutPath = Join-Path $logDirectory "release-build.stdout.log"
        StderrPath = Join-Path $logDirectory "release-build.stderr.log"
    })
    if (@($buildMeasurement.ExitCodes | Where-Object { $_ -ne 0 }).Count -ne 0) { throw "The single three-example release build failed." }
    if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -ne 0) { throw "Cargo remained active after the one allowed build." }
}
foreach ($binary in @($mgaBinary, $heterogeneityBinary, $maximumBinary)) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Built release example is absent: $binary" }
}
$mgaExecutableSha256 = Get-LowerSha256 $mgaBinary
$heterogeneityExecutableSha256 = Get-LowerSha256 $heterogeneityBinary
$maximumExecutableSha256 = Get-LowerSha256 $maximumBinary
$buildOutputs = [ordered]@{
    mga = [ordered]@{ file_name = [IO.Path]::GetFileName($mgaBinary); sha256 = $mgaExecutableSha256 }
    heterogeneity = [ordered]@{ file_name = [IO.Path]::GetFileName($heterogeneityBinary); sha256 = $heterogeneityExecutableSha256 }
    maximum = [ordered]@{ file_name = [IO.Path]::GetFileName($maximumBinary); sha256 = $maximumExecutableSha256 }
}
if ($null -ne $buildAttempt) {
    if (($buildAttempt.output_executables | ConvertTo-Json -Compress -Depth 20) -cne ($buildOutputs | ConvertTo-Json -Compress -Depth 20)) {
        throw "The release executables differ from the completed resumable build attempt."
    }
} else {
    $buildAttempt = [ordered]@{
        schema_version = 2
        suite_id = "qpls.v256.multimod.performance-release-build-attempt.v2"
        status = "completed"
        attempt_id = $attemptId
        attempt_count = 1
        source_commit = $sourceCommit
        cargo_executable_sha256 = $cargoExecutableSha256
        cargo_lock_sha256 = $cargoLockSha256
        command_identity = $buildCommandIdentity
        output_executables = $buildOutputs
        measurement = [ordered]@{
            wall_time_milliseconds = [long]$buildMeasurement.WallTimeMilliseconds
            peak_working_set_bytes = [long]$buildMeasurement.PeakWorkingSetBytes
            sampled_concurrent_peak_working_set_bytes = [long]$buildMeasurement.SampledConcurrentPeakWorkingSetBytes
            per_process_peak_working_set_bytes = @($buildMeasurement.PerProcessPeakWorkingSetBytes)
            working_set_sample_count = [int]$buildMeasurement.WorkingSetSampleCount
            process_count = [int]$buildMeasurement.ProcessCount
            process_exit_codes = @($buildMeasurement.ExitCodes)
        }
    }
    Write-JsonAtomic -Path $buildAttemptPath -Value $buildAttempt
}

$temporaryMgaPlan = Join-Path $mgaRoot ".cell-plan.$([Guid]::NewGuid().ToString('N')).tmp.json"
$planExit = Invoke-BoundedCommand -StageId "generate-mga-plan" -FileName $mgaBinary -Arguments @(
    "--output", $temporaryMgaPlan, "--scale", "qualification", "--seed", $Seed.ToString(), "--plan"
)
if ($planExit -ne 0) { throw "The exact MGA plan producer failed." }
Publish-DeterministicOutput $temporaryMgaPlan $mgaPlanPath "MGA qualification plan"
$mgaPlanExit = Invoke-PythonTool -StageId "validate-mga-plan" -Tool $mgaCheckpointTool -Arguments @("validate-plan", "--plan", $mgaPlanPath)
if ($mgaPlanExit -ne 0) { throw "The MGA plan differs from the frozen exact inventory." }
$mgaPlan = Get-Content -LiteralPath $mgaPlanPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$mgaEnvironmentSha256 = [string]$mgaPlan.baseline_environment_sha256

$temporaryHeterogeneityPlan = Join-Path $heterogeneityRoot ".shard-plan.$([Guid]::NewGuid().ToString('N')).tmp.json"
$planExit = Invoke-BoundedCommand -StageId "generate-heterogeneity-plan" -FileName $heterogeneityBinary -Arguments @(
    "--output", $temporaryHeterogeneityPlan, "--scale", "qualification", "--seed", $Seed.ToString(), "--plan"
)
if ($planExit -ne 0) { throw "The exact heterogeneity plan producer failed." }
Publish-DeterministicOutput $temporaryHeterogeneityPlan $heterogeneityPlanPath "heterogeneity qualification plan"

$temporaryTopologyPlan = Join-Path $resolvedWorkRoot ".performance-topology.$([Guid]::NewGuid().ToString('N')).tmp.json"
$topologyPlanExit = Invoke-PythonTool -StageId "write-performance-topology-plan" -Tool $topologyTool -Arguments @(
    "write-plan", "--output", $temporaryTopologyPlan, "--seed", $Seed.ToString(),
    "--source-commit", $sourceCommit, "--mga-plan", $mgaPlanPath,
    "--heterogeneity-plan", $heterogeneityPlanPath,
    "--cargo-lock", $cargoLockPath, "--build-attempt", $buildAttemptPath,
    "--cargo-executable", $cargoExecutable,
    "--mga-executable", $mgaBinary, "--heterogeneity-executable", $heterogeneityBinary,
    "--maximum-executable", $maximumBinary
)
if ($topologyPlanExit -ne 0) { throw "The bounded performance topology plan was rejected." }
Publish-DeterministicOutput $temporaryTopologyPlan $topologyPlanPath "performance topology plan"

if (-not (Test-PerformanceCheckpoint "release-build")) {
    Write-And-SealPerformanceMeasurement -StageId "release-build" -Measurement $buildMeasurement
}
Invoke-MgaStage $mgaExecutableSha256 $sourceCommit $mgaEnvironmentSha256
Invoke-HeterogeneitySingleStage "heterogeneity-sentinel" "sentinel" $heterogeneityExecutableSha256 $sourceCommit 120 "heterogeneity-sentinel"
Invoke-HeterogeneitySingleStage "heterogeneity-p23-discovery" "pos-destination-p23-discovery" $heterogeneityExecutableSha256 $sourceCommit $PerAtomicProcessTimeoutSeconds "heterogeneity-p23-discovery"
Invoke-HeterogeneityBootstrapPair $heterogeneityExecutableSha256 $sourceCommit
Invoke-MaximumStage $maximumExecutableSha256

$topologyExit = Invoke-PythonTool -StageId "aggregate-performance-topology" -Tool $topologyTool -Arguments @(
    "aggregate", "--plan", $topologyPlanPath, "--work-root", $resolvedWorkRoot,
    "--receipt-root", $resolvedWorkRoot, "--output", $topologyOutput
)
if ($topologyExit -ne 0) { throw "Performance topology aggregation failed." }

$mgaOutput = Join-Path (Join-Path $mgaCellDirectory $mgaCellId) "result.json"
$verificationExit = Invoke-BoundedCommand -StageId "verify-performance-science" -FileName $script:python -Arguments @(
    $verifier,
    "--mga", $mgaOutput,
    "--heterogeneity-sentinel", (Join-Path $heterogeneityShardDirectory "sentinel.json"),
    "--heterogeneity-discovery", (Join-Path $heterogeneityShardDirectory "pos-destination-p23-discovery.json"),
    "--heterogeneity-fimix", (Join-Path $heterogeneityShardDirectory "bootstrap-fimix-p23.json"),
    "--heterogeneity-pos", (Join-Path $heterogeneityShardDirectory "bootstrap-pos-destination-p23.json"),
    "--maximum", $maximumOutput,
    "--execution-topology", $topologyOutput,
    "--output", $verificationOutput
)
if ($verificationExit -ne 0) { throw "Maximum-profile output verification failed." }
$verification = Get-Content -LiteralPath $verificationOutput -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
if ($verification.passed -ne $true -or [int]$verification.schema_version -ne 2 -or [string]$verification.report_id -cne "qpls.v256.multimod.performance-output-verification.v2") {
    throw "Maximum-profile structural verification did not pass."
}
$topology = Get-Content -LiteralPath $topologyOutput -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
$workloads = @(
    foreach ($row in @($topology.workloads)) {
        $budget = $budgets.([string]$row.workload_id)
        if ($null -eq $budget) { throw "Unexpected performance workload: $($row.workload_id)" }
        if ([double]$row.wall_time_milliseconds / 1000.0 -gt [double]$budget.maximum_seconds -or
            [long]$row.peak_working_set_bytes -gt [long]$budget.maximum_peak_working_set_bytes -or
            [long]$row.output_size_bytes -le 0 -or
            [long]$row.output_size_bytes -gt [long]$budget.maximum_output_bytes) {
            throw "Performance workload exceeded its declared wall-time, memory, or output-size budget: $($row.workload_id)"
        }
        [ordered]@{
            workload_id = [string]$row.workload_id
            stage_ids = @($row.stage_ids)
            wall_time_milliseconds = [long]$row.wall_time_milliseconds
            peak_working_set_bytes = [long]$row.peak_working_set_bytes
            working_set_sample_count = [int]$row.working_set_sample_count
            atomic_process_count = [int]$row.atomic_process_count
            output_size_bytes = [long]$row.output_size_bytes
            predeclared_budget = $budget
            budget_passed = $true
        }
    }
)

$sourceFiles = @(
    "Cargo.lock",
    "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/examples/multimod_mga_qualification_v1.rs",
    "crates/qpls-runner/examples/multimod_heterogeneity_qualification_v2.rs",
    "crates/qpls-runner/examples/multimod_maximum_profiles_performance_v1.rs",
    "crates/qpls-runner/src/multimod_execution_v1.rs",
    "crates/qpls-runner/src/multimod_conditional_raw_v2.rs",
    "crates/qpls-estimation/src/heterogeneity_v2.rs",
    "crates/qpls-estimation/src/multigroup_v1.rs",
    "crates/qpls-project/src/multimod_archive_v1.rs",
    "validation/multimod/multimod_mga_shards_v1.py",
    "validation/multimod/multimod_heterogeneity_shards_v2.py",
    "validation/multimod/multimod_performance_topology_v2.py",
    "validation/multimod/test_multimod_performance_topology_v2.py",
    "validation/multimod/multimod_maximum_profile_performance_v2.schema.json",
    "validation/multimod/multimod_performance_output_verification_v2.schema.json",
    "validation/multimod/run_multimod_performance_profiles_v2.ps1",
    "validation/multimod/verify_multimod_performance_profiles_v2.py",
    "validation/multimod/multimod_gate_bindings_v1.json"
)
$sourceDigests = @(
    foreach ($relative in $sourceFiles) {
        $path = Join-Path $repositoryRoot $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Performance source binding is missing: $relative" }
        [ordered]@{ path = $relative.Replace("\", "/"); sha256 = Get-LowerSha256 $path; size = [long](Get-Item -LiteralPath $path).Length }
    }
)
Assert-PublicationDeadline
$report = [ordered]@{
    schema_version = 2
    report_id = "qpls.v256.multimod.maximum-profile-performance.v2"
    passed = $true
    seed = $Seed
    source_commit = $sourceCommit
    measurement_contract = "windows_sampled_concurrent_plus_per_process_peak_working_set_v3"
    metrics_fail_closed_when_unavailable = $true
    one_release_cargo_build = $topology.one_release_cargo_build
    release_build = $topology.release_build
    cargo_run_invocations = 0
    direct_binary_execution_after_build = $true
    per_atomic_process_timeout_seconds = $PerAtomicProcessTimeoutSeconds
    internal_timeout_seconds = $InternalTimeoutSeconds
    execution_deadline_seconds = $executionDeadlineSeconds
    publication_reserve_seconds = $publicationReserveSeconds
    final_publication_cutoff_enforced = $true
    predeclared_budgets = $budgets
    workloads = $workloads
    execution_topology_path = [IO.Path]::GetFullPath($topologyOutput)
    execution_topology_sha256 = Get-LowerSha256 $topologyOutput
    execution_topology = $topology
    production_output_verification = $verification
    production_output_verification_path = [IO.Path]::GetFullPath($verificationOutput)
    production_output_verification_sha256 = Get-LowerSha256 $verificationOutput
    source_digests = $sourceDigests
    sidecar_size_metrics = [ordered]@{
        warning_bytes = [long]$verification.maximum.archive_sidecar_boundaries.warning_bytes
        maximum_bytes = [long]$verification.maximum.archive_sidecar_boundaries.maximum_bytes
        aggregate_maximum_admitted_bytes = [long]$verification.maximum.archive_sidecar_boundaries.aggregate_maximum_admitted_bytes
        maximum_plus_one_rejected = $verification.maximum.archive_sidecar_boundaries.aggregate_maximum_plus_one_rejected
    }
    output_total_bytes = [long](($verification.producer_outputs | Measure-Object -Property size -Sum).Sum)
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
}
Assert-PublicationDeadline
$publicationStagingPath = Join-Path $outputDirectory ".performance-report.$([Guid]::NewGuid().ToString('N')).staged.json"
Write-JsonAtomic $publicationStagingPath $report
Assert-PublicationDeadline
[IO.File]::Move($publicationStagingPath, $outputPath, $false)
if ($campaignClock.Elapsed.TotalSeconds -ge $InternalTimeoutSeconds) {
    try { Remove-Item -LiteralPath $outputPath -Force } catch {
        throw "Performance report crossed its publication cutoff and cleanup of the late report failed: $($_.Exception.Message)"
    }
    throw "Performance report crossed its $InternalTimeoutSeconds-second publication cutoff and was withdrawn."
}
$report | ConvertTo-Json -Depth 100
