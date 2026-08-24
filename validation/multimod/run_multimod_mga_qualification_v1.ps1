[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output,

    [ValidateSet("qualification")]
    [string]$Scale = "qualification",

    [UInt64]$Seed = 42,

    [string]$WorkRoot,

    [ValidateRange(1, 4)]
    [int]$MaxParallelCells = 4,

    [ValidateRange(60, 1800)]
    [int]$PerCellTimeoutSeconds = 1800,

    [ValidateRange(600, 6600)]
    [int]$OverallTimeoutSeconds = 6600
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$resolvedOutput = [System.IO.Path]::GetFullPath($Output)
$resolvedWorkRoot = if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    "$resolvedOutput.mga-cells"
}
else {
    [System.IO.Path]::GetFullPath($WorkRoot)
}
$cellDirectory = Join-Path $resolvedWorkRoot "cells"
$cacheDirectory = Join-Path $resolvedWorkRoot "production-cache"
$logDirectory = Join-Path $resolvedWorkRoot "logs"
$historyDirectory = Join-Path $resolvedWorkRoot "_attempt_history"
$planPath = Join-Path $resolvedWorkRoot "cell-plan.json"
$rawAggregate = Join-Path $resolvedWorkRoot "mga-production-science.raw.json"
$checkpointTool = Join-Path $repositoryRoot "validation/multimod/multimod_mga_shards_v1.py"
$comparator = Join-Path $repositoryRoot "validation/multimod/compare_multimod_mga_qualification_v1.py"
$binary = Join-Path $repositoryRoot "target/debug/examples/multimod_mga_qualification_v1.exe"
$campaignClock = [System.Diagnostics.Stopwatch]::StartNew()
$sentinelTimeoutSeconds = 120
$cleanupReserveSeconds = 120
$active = @{}
$attempts = @{}

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
        throw "MGA qualification requires baseline metamorphism; found '$metamorphism'."
    }
    if ($null -ne $workers -and $workers -ne "1") {
        throw "MGA qualification requires exactly one producer worker per cell; found '$workers'."
    }
    if ($null -ne $compact) {
        throw "MGA qualification requires QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 to be unset."
    }
    if ($null -ne $signColumns) {
        throw "MGA qualification requires QPLS_MULTIMOD_SIGN_COLUMNS_V1 to be unset."
    }
    if ($Seed -ne 42) {
        throw "MGA qualification requires the frozen campaign seed 42."
    }
}

function Get-RemainingBudgetSeconds {
    $remaining = [Math]::Floor(
        $OverallTimeoutSeconds - $cleanupReserveSeconds - $campaignClock.Elapsed.TotalSeconds
    )
    if ($remaining -lt 1) {
        throw "MGA qualification reached its bounded work budget and reserved $cleanupReserveSeconds seconds for process-tree termination and log preservation inside the $OverallTimeoutSeconds-second wrapper cap. Completed cells and production-shard caches are retained."
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
        LogsSaved = $false
    }
}

function Save-ProcessLogs {
    param([Parameter(Mandatory = $true)]$Job)
    if ($Job.LogsSaved) { return }
    $tasks = [System.Threading.Tasks.Task[]]@($Job.StdoutCopy, $Job.StderrCopy)
    if (-not [System.Threading.Tasks.Task]::WaitAll($tasks, 10000)) {
        throw "Timed out draining redirected output for $($Job.Stage)."
    }
    [System.IO.File]::WriteAllText($Job.StdoutPath, $Job.StdoutCopy.Result)
    [System.IO.File]::WriteAllText($Job.StderrPath, $Job.StderrCopy.Result)
    $Job.LogsSaved = $true
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
        throw "Could not terminate $($Job.Stage) and its process tree after $Reason."
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

function Test-CellCheckpoint {
    param(
        [Parameter(Mandatory = $true)][string]$CellId,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][string]$EnvironmentSha256
    )
    $exitCode = Invoke-CheckpointTool -Stage "checkpoint-verify-$CellId" -Arguments @(
        "verify", "--plan", $planPath, "--cell-dir", $cellDirectory,
        "--cell-id", $CellId, "--executable-sha256", $ExecutableSha256,
        "--source-commit", $SourceCommit, "--environment-sha256", $EnvironmentSha256
    )
    if ($exitCode -eq 0) { return $true }
    if ($exitCode -eq 3) { return $false }
    throw "Existing MGA cell $CellId is stale, malformed, tampered, or identity-mismatched. Use a fresh external WorkRoot after reviewing retained evidence."
}

function Get-CellCacheCheckpointCount {
    param([Parameter(Mandatory = $true)][string]$CellId)
    $directory = Join-Path (Join-Path $cacheDirectory $CellId) $CellId
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) { return 0 }
    return @(
        Get-ChildItem -LiteralPath $directory -File -Filter "checkpoint-*.json" -ErrorAction Stop
    ).Count
}

function Get-VerifiedCellCacheProgress {
    param(
        [Parameter(Mandatory = $true)][string]$CellId,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][string]$EnvironmentSha256,
        [Parameter(Mandatory = $true)][string]$PlanSha256
    )
    $temporaryStatus = Join-Path $resolvedWorkRoot `
        (".{0}.cache-status.{1}.tmp.json" -f $CellId, [Guid]::NewGuid().ToString("N"))
    try {
        $statusExit = Invoke-BoundedStage -Stage "cache-progress-$CellId" `
            -FileName $binary -Arguments @(
                "--output", $temporaryStatus, "--scale", $Scale,
                "--seed", $Seed.ToString(), "--cell", $CellId, "--cache-status",
                "--cache-root", $cacheDirectory, "--plan-path", $planPath,
                "--plan-sha256", $PlanSha256, "--source-commit", $SourceCommit,
                "--executable-sha256", $ExecutableSha256,
                "--environment-sha256", $EnvironmentSha256
            ) -TimeoutSeconds $sentinelTimeoutSeconds
        if ($statusExit -ne 0 -or -not (Test-Path -LiteralPath $temporaryStatus -PathType Leaf)) {
            throw "MGA cell $CellId cache progress could not be validated by the exact production authority."
        }
        $status = Get-Content -LiteralPath $temporaryStatus -Raw | ConvertFrom-Json -Depth 100
        if ([string]$status.suite_id -ne "qpls.multimod.mga.verified-cache-progress.v1" -or
            [string]$status.cell_id -ne $CellId -or
            [string]$status.source_commit -ne $SourceCommit -or
            [string]$status.executable_sha256 -ne $ExecutableSha256 -or
            [string]$status.qualification_plan_sha256 -ne $PlanSha256 -or
            [string]$status.environment_sha256 -ne $EnvironmentSha256 -or
            [string]$status.scale -ne $Scale -or
            [UInt64]$status.seed -ne $Seed -or
            [string]$status.production_plan_sha256 -notmatch '^[0-9a-f]{64}$' -or
            [string]$status.cache_sha256 -notmatch '^[0-9a-f]{64}$' -or
            [int]$status.completed_shards -lt 0 -or
            [int]$status.pending_shards -lt 0 -or
            [int]$status.planned_shards -lt 1 -or
            [int]$status.completed_shards + [int]$status.pending_shards -ne [int]$status.planned_shards) {
            throw "MGA cell $CellId verified-cache progress receipt is incomplete or identity-mismatched."
        }
        return [int]$status.completed_shards
    }
    finally {
        if (Test-Path -LiteralPath $temporaryStatus -PathType Leaf) {
            Remove-Item -LiteralPath $temporaryStatus -Force
        }
    }
}

function Start-Cell {
    param(
        [Parameter(Mandatory = $true)]$Spec,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][string]$EnvironmentSha256,
        [Parameter(Mandatory = $true)][string]$PlanSha256,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )
    $cellId = [string]$Spec.cell_id
    if (-not $attempts.ContainsKey($cellId)) { $attempts[$cellId] = 0 }
    $attempts[$cellId] = [int]$attempts[$cellId] + 1
    $attempt = [int]$attempts[$cellId]
    $attemptIdentity = [Guid]::NewGuid().ToString("N")
    $temporaryResult = Join-Path $resolvedWorkRoot (".{0}.attempt-{1}.{2}.tmp.json" -f $cellId, $attempt, $attemptIdentity)
    $stdout = Join-Path $logDirectory ("{0}.attempt-{1}.{2}.stdout.log" -f $cellId, $attempt, $attemptIdentity)
    $stderr = Join-Path $logDirectory ("{0}.attempt-{1}.{2}.stderr.log" -f $cellId, $attempt, $attemptIdentity)
    $arguments = @(
        "--output", $temporaryResult, "--scale", $Scale, "--seed", $Seed.ToString(),
        "--cell", $cellId, "--cache-root", $cacheDirectory,
        "--plan-path", $planPath, "--plan-sha256", $PlanSha256,
        "--source-commit", $SourceCommit, "--executable-sha256", $ExecutableSha256,
        "--environment-sha256", $EnvironmentSha256
    )
    $cacheCheckpointCountBefore = Get-CellCacheCheckpointCount -CellId $cellId
    $job = Start-BoundedChild -Stage "mga-cell-$cellId" -FileName $binary `
        -Arguments $arguments -StdoutPath $stdout -StderrPath $stderr `
        -TimeoutSeconds $TimeoutSeconds
    $job | Add-Member -NotePropertyName CellId -NotePropertyValue $cellId
    $job | Add-Member -NotePropertyName Spec -NotePropertyValue $Spec
    $job | Add-Member -NotePropertyName TemporaryResult -NotePropertyValue $temporaryResult
    $job | Add-Member -NotePropertyName ExecutableSha256 -NotePropertyValue $ExecutableSha256
    $job | Add-Member -NotePropertyName SourceCommit -NotePropertyValue $SourceCommit
    $job | Add-Member -NotePropertyName EnvironmentSha256 -NotePropertyValue $EnvironmentSha256
    $job | Add-Member -NotePropertyName CacheCheckpointCountBefore `
        -NotePropertyValue $cacheCheckpointCountBefore
    return $job
}

function Preserve-UnpublishedResult {
    param([Parameter(Mandatory = $true)]$Job)
    if (-not (Test-Path -LiteralPath $Job.TemporaryResult -PathType Leaf)) { return }
    $attemptRoot = Join-Path $historyDirectory ("{0}-{1}" -f $Job.CellId, [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssfffZ"))
    New-Item -ItemType Directory -Path $attemptRoot -Force | Out-Null
    Move-Item -LiteralPath $Job.TemporaryResult -Destination (Join-Path $attemptRoot "unpublished-result.json")
}

function Complete-Cell {
    param([Parameter(Mandatory = $true)]$Job)
    if (-not $Job.Process.HasExited) {
        throw "Complete-Cell requires an exited process."
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        Stop-BoundedChild -Job $Job -Reason "exit_wait_timeout"
        throw "MGA cell $($Job.CellId) did not finalize after exit."
    }
    Save-ProcessLogs -Job $Job
    $exitCode = $Job.Process.ExitCode
    if ($exitCode -ne 0 -or -not (Test-Path -LiteralPath $Job.TemporaryResult -PathType Leaf)) {
        Preserve-UnpublishedResult -Job $Job
        $reason = if ($exitCode -ne 0) { "producer_exit_$exitCode" } else { "producer_output_absent" }
        throw "MGA cell $($Job.CellId) failed ($reason). Check $($Job.StderrPath)."
    }
    $sealExit = Invoke-CheckpointTool -Stage "checkpoint-seal-$($Job.CellId)" -Arguments @(
        "seal", "--plan", $planPath, "--cell-dir", $cellDirectory,
        "--cell-id", $Job.CellId, "--temporary-result", $Job.TemporaryResult,
        "--executable-sha256", $Job.ExecutableSha256,
        "--source-commit", $Job.SourceCommit,
        "--environment-sha256", $Job.EnvironmentSha256
    )
    if ($sealExit -ne 0) {
        Preserve-UnpublishedResult -Job $Job
        throw "MGA cell $($Job.CellId) completed but its atomic publication was rejected."
    }
    Write-Host ("[mga] completed cell {0} in {1:n1}s" -f $Job.CellId, $Job.Clock.Elapsed.TotalSeconds)
}

function Stop-ActiveCells {
    foreach ($job in @($active.Values)) {
        try {
            Stop-BoundedChild -Job $job -Reason "wrapper_abort"
            Preserve-UnpublishedResult -Job $job
        }
        catch {
            Write-Warning "Could not terminate cell $($job.CellId): $($_.Exception.Message)"
        }
    }
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
            throw "MGA qualification requires a fully clean commit-bound source tree, including no untracked files."
        }
        $sourceCommit = (& git rev-parse HEAD).Trim().ToLowerInvariant()
        if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
            throw "Unable to resolve the exact source commit."
        }

        New-Item -ItemType Directory -Path $resolvedWorkRoot, $cellDirectory, $cacheDirectory, $logDirectory, $historyDirectory -Force | Out-Null
        Write-Host "[mga] one Cargo build; exact cells then run directly with durable production-shard caches"
        $buildExit = Invoke-BoundedStage -Stage "cargo-build" -FileName "cargo" -Arguments @(
            "build", "--quiet", "--locked", "-p", "qpls-runner", "--example",
            "multimod_mga_qualification_v1"
        ) -TimeoutSeconds ([Math]::Min(1800, (Get-RemainingBudgetSeconds)))
        if ($buildExit -ne 0) { throw "The single MGA producer build failed." }
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "The built MGA producer executable is absent: $binary"
        }
        $executableSha256 = Get-Sha256 -Path $binary

        $temporaryPlan = Join-Path $resolvedWorkRoot (".cell-plan.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $planExit = Invoke-BoundedStage -Stage "plan" -FileName $binary -Arguments @(
            "--output", $temporaryPlan, "--scale", $Scale, "--seed", $Seed.ToString(), "--plan"
        ) -TimeoutSeconds 120
        if ($planExit -ne 0 -or -not (Test-Path -LiteralPath $temporaryPlan -PathType Leaf)) {
            throw "The deterministic MGA cell plan could not be generated."
        }
        Move-Item -LiteralPath $temporaryPlan -Destination $planPath -Force
        $planValidation = Invoke-CheckpointTool -Stage "plan-validation" -Arguments @(
            "validate-plan", "--plan", $planPath
        )
        if ($planValidation -ne 0) { throw "The MGA cell plan failed exact inventory validation." }
        $plan = Get-Content -LiteralPath $planPath -Raw | ConvertFrom-Json -Depth 100
        $specs = @($plan.cells)
        if ($specs.Count -ne 16 -or [string]$plan.root_sentinel_cell_id -ne "mga-general-2-groups") {
            throw "Qualification did not retain the exact 15 production MGA executions plus boundary cell."
        }
        $planSha256 = Get-Sha256 -Path $planPath
        $environmentSha256 = [string]$plan.baseline_environment_sha256

        Write-Host "[mga] running the diagnostic-only g2 compiler/plan sentinel before any scientific cell"
        $temporarySentinel = Join-Path $resolvedWorkRoot (".root-sentinel.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $sentinelExit = Invoke-BoundedStage -Stage "root-sentinel" -FileName $binary -Arguments @(
            "--output", $temporarySentinel, "--scale", $Scale, "--seed", $Seed.ToString(),
            "--sentinel", "--cache-root", $cacheDirectory,
            "--plan-path", $planPath, "--plan-sha256", $planSha256,
            "--source-commit", $sourceCommit, "--executable-sha256", $executableSha256,
            "--environment-sha256", $environmentSha256
        ) -TimeoutSeconds $sentinelTimeoutSeconds
        if ($sentinelExit -ne 0 -or -not (Test-Path -LiteralPath $temporarySentinel -PathType Leaf)) {
            throw "The diagnostic-only MGA root sentinel failed before scientific cells were admitted."
        }
        $sentinelReceipt = Get-Content -LiteralPath $temporarySentinel -Raw | ConvertFrom-Json -Depth 100
        if ([string]$sentinelReceipt.suite_id -ne "qpls.multimod.mga.root-compiler-sentinel.v1" -or
            $sentinelReceipt.diagnostic_only -ne $true -or
            $sentinelReceipt.scientific_result_published -ne $false -or
            [string]$sentinelReceipt.source_commit -ne $sourceCommit -or
            [string]$sentinelReceipt.executable_sha256 -ne $executableSha256 -or
            [string]$sentinelReceipt.qualification_plan_sha256 -ne $planSha256 -or
            [string]$sentinelReceipt.environment_sha256 -ne $environmentSha256 -or
            [string]$sentinelReceipt.scale -ne $Scale -or
            [UInt64]$sentinelReceipt.seed -ne $Seed -or
            [int]$sentinelReceipt.permutation_samples -ne 5000 -or
            [int]$sentinelReceipt.bootstrap_samples -ne 5000 -or
            [int]$sentinelReceipt.planned_production_shards -lt 1 -or
            [int]$sentinelReceipt.pending_production_shards -ne [int]$sentinelReceipt.planned_production_shards) {
            throw "The diagnostic-only MGA root sentinel receipt is incomplete or identity-mismatched."
        }
        Move-Item -LiteralPath $temporarySentinel -Destination (Join-Path $resolvedWorkRoot "root-sentinel.json") -Force

        $completed = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
        foreach ($spec in $specs) {
            $cellId = [string]$spec.cell_id
            if (Test-CellCheckpoint -CellId $cellId -ExecutableSha256 $executableSha256 `
                -SourceCommit $sourceCommit -EnvironmentSha256 $environmentSha256) {
                [void]$completed.Add($cellId)
            }
        }
        if ($completed.Count -gt 0) {
            Write-Host ("[mga] reused {0} exact completed cell(s)" -f $completed.Count)
        }

        while ($completed.Count -lt $specs.Count) {
            [void](Get-RemainingBudgetSeconds)
            foreach ($spec in $specs) {
                if ($active.Count -ge $MaxParallelCells) { break }
                $cellId = [string]$spec.cell_id
                if ($completed.Contains($cellId) -or $active.ContainsKey($cellId)) { continue }
                $job = Start-Cell -Spec $spec -ExecutableSha256 $executableSha256 `
                    -SourceCommit $sourceCommit -EnvironmentSha256 $environmentSha256 `
                    -PlanSha256 $planSha256 -TimeoutSeconds $PerCellTimeoutSeconds
                $active[$cellId] = $job
                Write-Host "[mga] started cell $cellId (attempt $($attempts[$cellId]))"
            }

            $madeProgress = $false
            foreach ($cellId in @($active.Keys)) {
                $job = $active[$cellId]
                if ($job.Process.HasExited) {
                    Complete-Cell -Job $job
                    $active.Remove($cellId)
                    [void]$completed.Add($cellId)
                    $madeProgress = $true
                    continue
                }
                if ($job.Clock.Elapsed.TotalSeconds -ge $job.TimeoutSeconds) {
                    Stop-BoundedChild -Job $job -Reason "resumable_cell_slice_timeout"
                    Preserve-UnpublishedResult -Job $job
                    $active.Remove($cellId)
                    $cacheCheckpointCountAfter = Get-VerifiedCellCacheProgress `
                        -CellId $cellId -ExecutableSha256 $executableSha256 `
                        -SourceCommit $sourceCommit -EnvironmentSha256 $environmentSha256 `
                        -PlanSha256 $planSha256
                    if ($cacheCheckpointCountAfter -le [int]$job.CacheCheckpointCountBefore) {
                        throw "MGA cell $cellId reached its $PerCellTimeoutSeconds-second slice without completing another production shard; refusing a wasteful retry of the same indivisible ledger."
                    }
                    $madeProgress = $true
                    Write-Host "[mga] cell $cellId reached its $PerCellTimeoutSeconds-second slice after advancing from $($job.CacheCheckpointCountBefore) to $cacheCheckpointCountAfter identity-bound production-shard checkpoints; retry will resume"
                }
            }
            if (-not $madeProgress) { Start-Sleep -Milliseconds 250 }
        }

        [void](Get-RemainingBudgetSeconds)
        $aggregateExit = Invoke-CheckpointTool -Stage "aggregate" -TimeoutSeconds $PerCellTimeoutSeconds -Arguments @(
            "aggregate", "--plan", $planPath, "--cell-dir", $cellDirectory,
            "--executable-sha256", $executableSha256, "--source-commit", $sourceCommit,
            "--environment-sha256", $environmentSha256, "--output", $rawAggregate
        )
        if ($aggregateExit -ne 0) { throw "Exact MGA cell aggregation failed." }

        $temporaryComparison = Join-Path $resolvedWorkRoot (".comparison.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $comparisonExit = Invoke-BoundedStage -Stage "comparator" -FileName "python" -Arguments @(
            $comparator, "--input", $rawAggregate, "--output", $temporaryComparison
        ) -TimeoutSeconds $PerCellTimeoutSeconds
        if (Test-Path -LiteralPath $temporaryComparison -PathType Leaf) {
            $outputDirectory = Split-Path -Parent $resolvedOutput
            if (-not [string]::IsNullOrWhiteSpace($outputDirectory)) {
                New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
            }
            Move-Item -LiteralPath $temporaryComparison -Destination $resolvedOutput -Force
        }
        exit $comparisonExit
    }
    finally {
        Pop-Location
    }
}
catch {
    Stop-ActiveCells
    Write-Error $_
    exit 1
}
