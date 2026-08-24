[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("conditional", "causal")]
    [string]$Family,

    [Parameter(Mandatory = $true)]
    [ValidateSet("development", "qualification")]
    [string]$Scale,

    [Parameter(Mandatory = $true)]
    [string]$Output,

    [string]$WorkRoot,

    [ValidateRange(1, 4)]
    [int]$MaxParallelShards = 4,

    [ValidateRange(1, 2)]
    [int]$MaxParallelHeavyShards = 2,

    [ValidateRange(60, 1800)]
    [int]$PerShardTimeoutSeconds = 1800,

    [ValidateRange(600, 6600)]
    [int]$OverallTimeoutSeconds = 6600
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [System.IO.Path]::GetFullPath($Output)
$outputDirectory = [System.IO.Path]::GetDirectoryName($outputPath)
$outputStem = [System.IO.Path]::GetFileNameWithoutExtension($outputPath)
$producerOutput = Join-Path $outputDirectory "$outputStem.producer.json"
$example = if ($Family -eq "conditional") {
    "multimod_conditional_qualification_v1"
}
else {
    "multimod_causal_qualification_v1"
}
$resolvedWorkRoot = if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    "$producerOutput.$Family-shards"
}
else {
    [System.IO.Path]::GetFullPath($WorkRoot)
}
$shardDirectory = Join-Path $resolvedWorkRoot "shards"
$logDirectory = Join-Path $resolvedWorkRoot "logs"
$historyDirectory = Join-Path $resolvedWorkRoot "_attempt_history"
$planPath = Join-Path $resolvedWorkRoot "shard-plan.json"
$checkpointTool = Join-Path $repositoryRoot "validation/multimod/conditional_causal_shards_v1.py"
$verifier = Join-Path $repositoryRoot "validation/multimod/verify_conditional_causal_raw_qualification_v1.py"
$binary = Join-Path $repositoryRoot "target/debug/examples/$example.exe"
$campaignClock = [System.Diagnostics.Stopwatch]::StartNew()
$sentinelTimeoutSeconds = [Math]::Min(120, $PerShardTimeoutSeconds)
$active = @{}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-BaselineEnvironment {
    $metamorphism = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_METAMORPHISM_V1")
    $workers = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_WORKERS_V1")
    $compact = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1")
    $signColumns = [Environment]::GetEnvironmentVariable("QPLS_MULTIMOD_SIGN_COLUMNS_V1")
    if ($null -ne $metamorphism -and $metamorphism -ne "baseline") {
        throw "$Family qualification requires baseline metamorphism; found '$metamorphism'."
    }
    if ($null -ne $workers -and $workers -ne "1") {
        throw "$Family qualification requires exactly one fixture worker; found '$workers'."
    }
    if ($null -ne $compact) {
        throw "$Family qualification requires QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 to be unset."
    }
    if ($null -ne $signColumns) {
        throw "$Family qualification requires QPLS_MULTIMOD_SIGN_COLUMNS_V1 to be unset."
    }
}

function Get-RemainingBudgetSeconds {
    $remaining = [Math]::Floor($OverallTimeoutSeconds - $campaignClock.Elapsed.TotalSeconds)
    if ($remaining -lt 1) {
        throw "$Family qualification reached its $OverallTimeoutSeconds-second resumable wall-clock cap. Completed shard receipts are retained."
    }
    return [int]$remaining
}

function Start-BoundedChild {
    param(
        [Parameter(Mandatory = $true)][string]$Stage,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )
    $effectiveTimeout = [Math]::Min($TimeoutSeconds, (Get-RemainingBudgetSeconds))
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FileName
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add([string]$argument)
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    return [pscustomobject]@{
        Stage = $Stage
        Process = $process
        Clock = [System.Diagnostics.Stopwatch]::StartNew()
        TimeoutSeconds = $effectiveTimeout
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
        StdoutCopy = $process.StandardOutput.ReadToEndAsync()
        StderrCopy = $process.StandardError.ReadToEndAsync()
    }
}

function Save-ProcessLogs {
    param([Parameter(Mandatory = $true)]$Job)
    $tasks = [System.Threading.Tasks.Task[]]@($Job.StdoutCopy, $Job.StderrCopy)
    if (-not [System.Threading.Tasks.Task]::WaitAll($tasks, 10000)) {
        throw "Timed out draining redirected output for $($Job.Stage)."
    }
    [System.IO.File]::WriteAllText($Job.StdoutPath, $Job.StdoutCopy.Result)
    [System.IO.File]::WriteAllText($Job.StderrPath, $Job.StderrCopy.Result)
}

function Get-CompletedChildRuntimeSeconds {
    param([Parameter(Mandatory = $true)]$Job)
    if (-not $Job.Process.HasExited) {
        throw "Cannot calculate completed runtime for the active stage $($Job.Stage)."
    }
    $started = $Job.Process.StartTime.ToUniversalTime()
    $finished = $Job.Process.ExitTime.ToUniversalTime()
    return ($finished - $started).TotalSeconds
}

function Stop-BoundedChild {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    if (-not $Job.Process.HasExited) {
        try { $Job.Process.Kill($true) } catch { Write-Warning $_.Exception.Message }
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        throw "Could not terminate $($Job.Stage) within 10 seconds after $Reason."
    }
    Save-ProcessLogs -Job $Job
}

function Wait-BoundedChild {
    param([Parameter(Mandatory = $true)]$Job)
    while (-not $Job.Process.HasExited) {
        if ($Job.Clock.Elapsed.TotalSeconds -ge $Job.TimeoutSeconds -or
            $campaignClock.Elapsed.TotalSeconds -ge $OverallTimeoutSeconds) {
            Stop-BoundedChild -Job $Job -Reason "bounded_timeout"
            throw "$($Job.Stage) exceeded its bounded time budget."
        }
        Start-Sleep -Milliseconds 200
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        Stop-BoundedChild -Job $Job -Reason "exit_wait_timeout"
        throw "$($Job.Stage) did not finalize after exit."
    }
    $completedRuntimeSeconds = Get-CompletedChildRuntimeSeconds -Job $Job
    if ($completedRuntimeSeconds -ge $Job.TimeoutSeconds) {
        Save-ProcessLogs -Job $Job
        throw "$($Job.Stage) exited after its bounded time budget ($completedRuntimeSeconds seconds)."
    }
    Save-ProcessLogs -Job $Job
    return $Job.Process.ExitCode
}

function Invoke-BoundedStage {
    param(
        [Parameter(Mandatory = $true)][string]$Stage,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )
    $identity = [Guid]::NewGuid().ToString("N")
    $stdout = Join-Path $logDirectory ("_{0}.{1}.stdout.log" -f $Stage, $identity)
    $stderr = Join-Path $logDirectory ("_{0}.{1}.stderr.log" -f $Stage, $identity)
    $job = Start-BoundedChild -Stage $Stage -FileName $FileName -Arguments $Arguments `
        -StdoutPath $stdout -StderrPath $stderr -TimeoutSeconds $TimeoutSeconds
    return Wait-BoundedChild -Job $job
}

function Invoke-CheckpointTool {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSeconds = 120,
        [string]$Stage = "checkpoint"
    )
    return Invoke-BoundedStage -Stage $Stage -FileName "python" `
        -Arguments (@($checkpointTool) + $Arguments) -TimeoutSeconds $TimeoutSeconds
}

function Test-ShardCheckpoint {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $exitCode = Invoke-CheckpointTool -Stage "checkpoint-verify-$ShardId" -Arguments @(
        "verify", "--family", $Family, "--plan", $planPath,
        "--shard-dir", $shardDirectory, "--shard-id", $ShardId,
        "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
    )
    return $exitCode -eq 0
}

function Move-StaleShardFiles {
    param([Parameter(Mandatory = $true)][string]$ShardId)
    $paths = @(
        @(
            (Join-Path $shardDirectory "$ShardId.json"),
            (Join-Path $shardDirectory "$ShardId.receipt.json"),
            (Join-Path $shardDirectory "$ShardId.failure.json"),
            (Join-Path $logDirectory "$ShardId.stdout.log"),
            (Join-Path $logDirectory "$ShardId.stderr.log")
        ) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
    )
    if ($paths.Count -eq 0) {
        return
    }
    $attemptRoot = Join-Path $historyDirectory ("{0}-{1}" -f $ShardId, [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssfffZ"))
    New-Item -ItemType Directory -Path $attemptRoot -Force | Out-Null
    foreach ($path in $paths) {
        Move-Item -LiteralPath $path -Destination (Join-Path $attemptRoot ([System.IO.Path]::GetFileName($path)))
    }
}

function Start-Shard {
    param(
        [Parameter(Mandatory = $true)]$Spec,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $shardId = [string]$Spec.shard_id
    Move-StaleShardFiles -ShardId $shardId
    $temporaryResult = Join-Path $resolvedWorkRoot (".{0}.{1}.tmp.json" -f $shardId, [Guid]::NewGuid().ToString("N"))
    $stdout = Join-Path $logDirectory "$shardId.stdout.log"
    $stderr = Join-Path $logDirectory "$shardId.stderr.log"
    $arguments = @("--output", $temporaryResult, "--scale", $Scale, "--shard", $shardId)
    foreach ($dependencyId in @($Spec.dependencies)) {
        $arguments += @("--dependency", (Join-Path $shardDirectory "$dependencyId.json"))
    }
    $job = Start-BoundedChild -Stage "$Family-shard-$shardId" -FileName $binary `
        -Arguments $arguments -StdoutPath $stdout -StderrPath $stderr `
        -TimeoutSeconds $PerShardTimeoutSeconds
    $job | Add-Member -NotePropertyName ShardId -NotePropertyValue $shardId
    $job | Add-Member -NotePropertyName Spec -NotePropertyValue $Spec
    $job | Add-Member -NotePropertyName TemporaryResult -NotePropertyValue $temporaryResult
    $job | Add-Member -NotePropertyName ExecutableSha256 -NotePropertyValue $ExecutableSha256
    $job | Add-Member -NotePropertyName SourceCommit -NotePropertyValue $SourceCommit
    return $job
}

function Write-ShardFailureReceipt {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason,
        [Parameter(Mandatory = $true)][int]$ExitCode
    )
    $failureReceipt = Join-Path $shardDirectory "$($Job.ShardId).failure.json"
    [void](Invoke-CheckpointTool -Arguments @(
        "record-failure", "--family", $Family, "--plan", $planPath,
        "--shard-dir", $shardDirectory, "--shard-id", $Job.ShardId,
        "--executable-sha256", $Job.ExecutableSha256, "--source-commit", $Job.SourceCommit,
        "--failure-receipt", $failureReceipt, "--exit-code", $ExitCode.ToString(),
        "--failure-reason", $Reason, "--stdout", $Job.StdoutPath, "--stderr", $Job.StderrPath
    ) -Stage "checkpoint-failure-$($Job.ShardId)")
}

function Record-ShardFailure {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    Stop-BoundedChild -Job $Job -Reason $Reason
    Write-ShardFailureReceipt -Job $Job -Reason $Reason -ExitCode 124
}

function Complete-Shard {
    param([Parameter(Mandatory = $true)]$Job)
    try {
        $exitCode = Wait-BoundedChild -Job $Job
    }
    catch {
        try { Write-ShardFailureReceipt -Job $Job -Reason "bounded_timeout" -ExitCode 124 } catch { Write-Warning $_.Exception.Message }
        throw
    }
    if ($exitCode -ne 0 -or -not (Test-Path -LiteralPath $Job.TemporaryResult -PathType Leaf)) {
        $reason = if ($exitCode -ne 0) { "producer_exit_nonzero" } else { "producer_output_absent" }
        Write-ShardFailureReceipt -Job $Job -Reason $reason -ExitCode $exitCode
        throw "$Family shard $($Job.ShardId) failed ($reason). Check $($Job.StderrPath)."
    }
    $sealExit = Invoke-CheckpointTool -Arguments @(
        "seal", "--family", $Family, "--plan", $planPath,
        "--shard-dir", $shardDirectory, "--shard-id", $Job.ShardId,
        "--temporary-result", $Job.TemporaryResult, "--executable-sha256", $Job.ExecutableSha256,
        "--source-commit", $Job.SourceCommit
    ) -Stage "checkpoint-seal-$($Job.ShardId)"
    if ($sealExit -ne 0) {
        throw "$Family shard $($Job.ShardId) completed but its atomic receipt was rejected."
    }
    Write-Host ("[{0}] completed shard {1} in {2:n1}s" -f $Family, $Job.ShardId, $Job.Clock.Elapsed.TotalSeconds)
}

function Stop-ActiveShards {
    foreach ($job in @($active.Values)) {
        try {
            Stop-BoundedChild -Job $job -Reason "wrapper_abort"
        }
        catch {
            Write-Warning "Could not terminate shard $($job.ShardId): $($_.Exception.Message)"
        }
    }
}

function Assert-TimeBudget {
    [void](Get-RemainingBudgetSeconds)
}

try {
    Push-Location -LiteralPath $repositoryRoot
    try {
        Assert-BaselineEnvironment
        $dirtyState = @(& git status --porcelain=v1 --untracked-files=all)
        if ($LASTEXITCODE -ne 0) {
            throw "Unable to inspect the complete tracked and untracked source state."
        }
        if (-not [string]::IsNullOrWhiteSpace(($dirtyState -join "`n"))) {
            throw "$Family qualification requires a fully clean commit-bound source tree, including no untracked files."
        }
        $sourceCommit = (& git rev-parse HEAD).Trim().ToLowerInvariant()
        if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
            throw "Unable to resolve the exact source commit."
        }

        New-Item -ItemType Directory -Path $outputDirectory, $resolvedWorkRoot, $shardDirectory, $logDirectory, $historyDirectory -Force | Out-Null
        Write-Host "[$Family] one Cargo build; exact qualification cases run afterward as resumable executable shards"
        $buildExit = Invoke-BoundedStage -Stage "cargo-build" -FileName "cargo" -Arguments @(
            "build", "--quiet", "--locked", "-p", "qpls-runner", "--example", $example
        ) -TimeoutSeconds (Get-RemainingBudgetSeconds)
        if ($buildExit -ne 0) {
            throw "The single $Family producer build failed."
        }
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "The built $Family producer executable is absent: $binary"
        }
        Assert-TimeBudget
        $executableSha256 = Get-Sha256 -Path $binary

        $temporaryPlan = Join-Path $resolvedWorkRoot (".shard-plan.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $planExit = Invoke-BoundedStage -Stage "plan" -FileName $binary -Arguments @(
            "--output", $temporaryPlan, "--scale", $Scale, "--plan"
        ) -TimeoutSeconds 120
        if ($planExit -ne 0 -or -not (Test-Path -LiteralPath $temporaryPlan -PathType Leaf)) {
            throw "The deterministic $Family shard plan could not be generated."
        }
        Move-Item -LiteralPath $temporaryPlan -Destination $planPath -Force
        $plan = Get-Content -LiteralPath $planPath -Raw | ConvertFrom-Json -Depth 100
        $specs = @($plan.shards)
        if ($specs.Count -eq 0 -or [string]$specs[0].shard_id -ne "sentinel" -or
            [string]$plan.metamorphism -ne "baseline" -or $null -ne $plan.sign_columns -or
            [int]$plan.workers -ne 1) {
            throw "The $Family shard plan did not preserve the baseline identity with the fast root sentinel first."
        }

        $completed = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
        $sentinel = $specs[0]
        if (Test-ShardCheckpoint -ShardId "sentinel" -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit) {
            [void]$completed.Add("sentinel")
            Write-Host "[$Family] reused verified sentinel checkpoint"
        }
        else {
            Write-Host "[$Family] running the 2-minute production sentinel before expensive cases"
            $job = Start-Shard -Spec $sentinel -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit
            $job.TimeoutSeconds = [Math]::Min($job.TimeoutSeconds, $sentinelTimeoutSeconds)
            Complete-Shard -Job $job
            [void]$completed.Add("sentinel")
        }

        foreach ($spec in $specs | Select-Object -Skip 1) {
            $shardId = [string]$spec.shard_id
            if (Test-ShardCheckpoint -ShardId $shardId -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit) {
                [void]$completed.Add($shardId)
            }
        }
        if ($completed.Count -gt 1) {
            Write-Host ("[{0}] reused {1} verified case checkpoints" -f $Family, ($completed.Count - 1))
        }

        while ($completed.Count -lt $specs.Count) {
            Assert-TimeBudget
            $startedAny = $false
            $activeHeavy = @($active.Values | Where-Object { [string]$_.Spec.resource_class -eq "heavy" }).Count
            foreach ($spec in $specs) {
                if ($active.Count -ge $MaxParallelShards) { break }
                $shardId = [string]$spec.shard_id
                if ($completed.Contains($shardId) -or $active.ContainsKey($shardId)) { continue }
                $dependenciesReady = $true
                foreach ($dependencyId in @($spec.dependencies)) {
                    if (-not $completed.Contains([string]$dependencyId)) {
                        $dependenciesReady = $false
                        break
                    }
                }
                if (-not $dependenciesReady) { continue }
                if ([string]$spec.resource_class -eq "heavy" -and $activeHeavy -ge $MaxParallelHeavyShards) {
                    continue
                }
                $job = Start-Shard -Spec $spec -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit
                $active[$shardId] = $job
                if ([string]$spec.resource_class -eq "heavy") { $activeHeavy++ }
                $startedAny = $true
                Write-Host "[$Family] started shard $shardId"
            }

            $completedAny = $false
            foreach ($shardId in @($active.Keys)) {
                $job = $active[$shardId]
                if (-not $job.Process.HasExited) {
                    if ($job.Clock.Elapsed.TotalSeconds -ge $job.TimeoutSeconds) {
                        Record-ShardFailure -Job $job -Reason "shard_timeout"
                        $active.Remove($shardId)
                        throw "$Family shard $shardId exceeded its $PerShardTimeoutSeconds-second timeout; completed checkpoints are retained."
                    }
                    continue
                }
                Complete-Shard -Job $job
                $active.Remove($shardId)
                [void]$completed.Add($shardId)
                $completedAny = $true
            }
            if (-not $startedAny -and -not $completedAny -and $active.Count -eq 0) {
                throw "The $Family shard graph made no progress; a dependency is missing or cyclic."
            }
            if (-not $completedAny) { Start-Sleep -Milliseconds 250 }
        }

        Assert-TimeBudget
        $aggregateExit = Invoke-CheckpointTool -Arguments @(
            "aggregate", "--family", $Family, "--plan", $planPath,
            "--shard-dir", $shardDirectory, "--executable-sha256", $executableSha256,
            "--source-commit", $sourceCommit, "--output", $producerOutput
        ) -TimeoutSeconds $PerShardTimeoutSeconds -Stage "aggregate"
        if ($aggregateExit -ne 0) {
            throw "Deterministic $Family shard aggregation failed."
        }

        $temporaryReceipt = Join-Path $resolvedWorkRoot (".verification.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $verificationExit = Invoke-BoundedStage -Stage "verifier" -FileName "python" -Arguments @(
            $verifier, "--family", $Family, "--report", $producerOutput,
            "--expected-scale", $Scale, "--require-shard-receipts",
            "--shard-plan", $planPath, "--shard-dir", $shardDirectory,
            "--producer-executable", $binary, "--expected-source-commit", $sourceCommit,
            "--output", $temporaryReceipt
        ) -TimeoutSeconds $PerShardTimeoutSeconds
        if (Test-Path -LiteralPath $temporaryReceipt -PathType Leaf) {
            Move-Item -LiteralPath $temporaryReceipt -Destination $outputPath -Force
        }
        exit $verificationExit
    }
    finally {
        Pop-Location
    }
}
catch {
    Stop-ActiveShards
    Write-Error $_
    exit 1
}
