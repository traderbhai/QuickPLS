[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output,

    [ValidateSet("development", "qualification")]
    [string]$Scale = "qualification",

    [UInt64]$Seed = 42,

    [string]$WorkRoot,

    [ValidateRange(1, 4)]
    [int]$MaxParallelShards = 4,

    [ValidateRange(1, 4)]
    [int]$MaxParallelBootstrapShards = 4,

    [ValidateRange(60, 1800)]
    [int]$PerShardTimeoutSeconds = 1800,

    [ValidateRange(600, 6600)]
    [int]$OverallTimeoutSeconds = 6480,

    [ValidateRange(1, 1500)]
    [int]$BootstrapProcessBudgetSeconds = 1500
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$resolvedOutput = [System.IO.Path]::GetFullPath($Output)
$resolvedWorkRoot = if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    "$resolvedOutput.heterogeneity-shards"
}
else {
    [System.IO.Path]::GetFullPath($WorkRoot)
}
$shardDirectory = Join-Path $resolvedWorkRoot "shards"
$logDirectory = Join-Path $resolvedWorkRoot "logs"
$historyDirectory = Join-Path $resolvedWorkRoot "_attempt_history"
$bootstrapDirectory = Join-Path $resolvedWorkRoot "bootstrap"
$planPath = Join-Path $resolvedWorkRoot "shard-plan.json"
$rawAggregate = Join-Path $resolvedWorkRoot "heterogeneity-production-science.raw.json"
$checkpointTool = Join-Path $repositoryRoot "validation/multimod/multimod_heterogeneity_shards_v2.py"
$comparator = Join-Path $repositoryRoot "validation/multimod/compare_multimod_heterogeneity_qualification_v2.py"
$binary = Join-Path $repositoryRoot "target/release/examples/multimod_heterogeneity_qualification_v2.exe"
$campaignStartedAtUtc = [DateTime]::UtcNow
$campaignClock = [System.Diagnostics.Stopwatch]::StartNew()
$workCutoffSeconds = [Math]::Min($OverallTimeoutSeconds, 6480)
$sentinelTimeoutSeconds = [Math]::Min(120, $PerShardTimeoutSeconds)
$active = @{}
$bootstrapChunkCount = 100

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
        throw "Heterogeneity qualification requires baseline metamorphism; found '$metamorphism'."
    }
    if ($null -ne $workers -and $workers -ne "1") {
        throw "Heterogeneity qualification requires exactly one fixture worker; found '$workers'."
    }
    if ($null -ne $compact) {
        throw "Heterogeneity qualification requires QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 to be unset."
    }
    if ($null -ne $signColumns) {
        throw "Heterogeneity qualification requires QPLS_MULTIMOD_SIGN_COLUMNS_V1 to be unset."
    }
    if ($Scale -eq "qualification" -and $Seed -ne 42) {
        throw "Heterogeneity qualification requires the frozen campaign seed 42."
    }
}

function Get-RemainingBudgetSeconds {
    $remaining = [Math]::Floor($workCutoffSeconds - $campaignClock.Elapsed.TotalSeconds)
    if ($remaining -lt 1) {
        throw "Heterogeneity qualification reached its $workCutoffSeconds-second resumable work cutoff. Completed shard receipts are retained for cleanup or resume."
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
        StartedAtUtc = $process.StartTime.ToUniversalTime()
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
            $campaignClock.Elapsed.TotalSeconds -ge $workCutoffSeconds) {
            Stop-BoundedChild -Job $Job -Reason "bounded_timeout"
            throw "$($Job.Stage) exceeded its bounded time budget."
        }
        Start-Sleep -Milliseconds 200
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        Stop-BoundedChild -Job $Job -Reason "exit_wait_timeout"
        throw "$($Job.Stage) did not finalize after exit."
    }
    $childElapsedSeconds = ($Job.Process.ExitTime.ToUniversalTime() - $Job.StartedAtUtc).TotalSeconds
    $campaignElapsedAtExitSeconds = ($Job.Process.ExitTime.ToUniversalTime() - $campaignStartedAtUtc).TotalSeconds
    if ($childElapsedSeconds -ge $Job.TimeoutSeconds -or
        $campaignElapsedAtExitSeconds -ge $workCutoffSeconds) {
        Save-ProcessLogs -Job $Job
        throw "$($Job.Stage) exited at or beyond its exact bounded deadline."
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
        "verify", "--plan", $planPath, "--shard-dir", $shardDirectory,
        "--shard-id", $ShardId, "--executable-sha256", $ExecutableSha256,
        "--source-commit", $SourceCommit
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
    $arguments = @("--output", $temporaryResult, "--scale", $Scale, "--seed", $Seed.ToString(), "--shard", $shardId)
    foreach ($dependencyId in @($Spec.dependencies)) {
        $arguments += @("--dependency", (Join-Path $shardDirectory "$dependencyId.json"))
    }
    $job = Start-BoundedChild -Stage "heterogeneity-shard-$shardId" -FileName $binary `
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
        "record-failure", "--plan", $planPath, "--shard-dir", $shardDirectory,
        "--shard-id", $Job.ShardId, "--executable-sha256", $Job.ExecutableSha256,
        "--source-commit", $Job.SourceCommit, "--failure-receipt", $failureReceipt,
        "--exit-code", $ExitCode.ToString(), "--failure-reason", $Reason,
        "--stdout", $Job.StdoutPath, "--stderr", $Job.StderrPath
    ) -Stage "checkpoint-failure-$($Job.ShardId)")
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
        throw "Heterogeneity shard $($Job.ShardId) failed ($reason). Check $($Job.StderrPath)."
    }
    $sealExit = Invoke-CheckpointTool -Arguments @(
        "seal", "--plan", $planPath, "--shard-dir", $shardDirectory,
        "--shard-id", $Job.ShardId, "--temporary-result", $Job.TemporaryResult,
        "--executable-sha256", $Job.ExecutableSha256, "--source-commit", $Job.SourceCommit
    ) -Stage "checkpoint-seal-$($Job.ShardId)"
    if ($sealExit -ne 0) {
        throw "Heterogeneity shard $($Job.ShardId) completed but its atomic receipt was rejected."
    }
    Write-Host ("[heterogeneity] completed shard {0} in {1:n1}s" -f $Job.ShardId, $Job.Clock.Elapsed.TotalSeconds)
}

function Get-BootstrapCellDirectory {
    param([Parameter(Mandatory = $true)][string]$ShardId)
    return Join-Path $bootstrapDirectory $ShardId
}

function Get-BootstrapPreparedPointerPath {
    param([Parameter(Mandatory = $true)][string]$ShardId)
    return Join-Path (Get-BootstrapCellDirectory -ShardId $ShardId) "prepared.current.json"
}

function Get-BootstrapCachePointerPath {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][int]$ChunkIndex
    )
    return Join-Path (Get-BootstrapCellDirectory -ShardId $ShardId) `
        ("cache-{0:D3}-of-{1:D3}.current.json" -f $ChunkIndex, $bootstrapChunkCount)
}

function Resolve-BootstrapGenerationPayload {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][ValidateSet("prepared", "cache")][string]$Kind,
        [Nullable[int]]$ChunkIndex
    )
    $pointerPath = if ($Kind -eq "prepared") {
        Get-BootstrapPreparedPointerPath -ShardId $ShardId
    }
    else {
        Get-BootstrapCachePointerPath -ShardId $ShardId -ChunkIndex ([int]$ChunkIndex)
    }
    if (-not (Test-Path -LiteralPath $pointerPath -PathType Leaf)) {
        throw "Current $Kind generation pointer is absent for $ShardId."
    }
    $pointer = Get-Content -LiteralPath $pointerPath -Raw | ConvertFrom-Json -Depth 20
    $payloadName = [string]$pointer.payload_file
    $chunkMatches = if ($Kind -eq "prepared") {
        $null -eq $pointer.chunk_index
    }
    else {
        $null -ne $pointer.chunk_index -and [int]$pointer.chunk_index -eq [int]$ChunkIndex
    }
    if ([int]$pointer.schema_version -ne 1 -or
        [string]$pointer.suite_id -ne "qpls.multimod.heterogeneity.bootstrap-current-generation.v1" -or
        [string]$pointer.scientific_shard_id -ne $ShardId -or
        [string]$pointer.kind -ne $Kind -or
        -not $chunkMatches -or
        [string]::IsNullOrWhiteSpace($payloadName) -or
        [System.IO.Path]::IsPathRooted($payloadName) -or
        [System.IO.Path]::GetFileName($payloadName) -ne $payloadName) {
        throw "Current $Kind generation pointer is invalid for $ShardId."
    }
    $payloadPath = Join-Path (Get-BootstrapCellDirectory -ShardId $ShardId) $payloadName
    if (-not (Test-Path -LiteralPath $payloadPath -PathType Leaf)) {
        throw "Current $Kind generation payload is absent for $ShardId."
    }
    return $payloadPath
}

function Get-BootstrapPreparedPath {
    param([Parameter(Mandatory = $true)][string]$ShardId)
    return Resolve-BootstrapGenerationPayload -ShardId $ShardId -Kind "prepared"
}

function Get-BootstrapCachePath {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][int]$ChunkIndex
    )
    return Resolve-BootstrapGenerationPayload -ShardId $ShardId -Kind "cache" `
        -ChunkIndex $ChunkIndex
}

function Get-DependencyArguments {
    param([Parameter(Mandatory = $true)]$Spec)
    $arguments = @()
    foreach ($dependencyId in @($Spec.dependencies)) {
        $arguments += @("--dependency", (Join-Path $shardDirectory "$dependencyId.json"))
    }
    return $arguments
}

function Assert-BootstrapChildBudget {
    param([Parameter(Mandatory = $true)][int]$InternalBudgetSeconds)
    $required = $InternalBudgetSeconds + 30
    $remaining = Get-RemainingBudgetSeconds
    if ($remaining -lt $required) {
        throw "Heterogeneity qualification has $remaining seconds left, below the $required-second safe bootstrap child envelope. Verified caches are retained for resume."
    }
}

function Test-BootstrapPreparedCheckpoint {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $pointer = Get-BootstrapPreparedPointerPath -ShardId $ShardId
    if (-not (Test-Path -LiteralPath $pointer -PathType Leaf)) { return $false }
    $exitCode = Invoke-CheckpointTool -Stage "bootstrap-verify-prepared-$ShardId" -Arguments @(
        "verify-bootstrap-prepared", "--plan", $planPath, "--shard-dir", $shardDirectory,
        "--shard-id", $ShardId, "--executable-sha256", $ExecutableSha256,
        "--source-commit", $SourceCommit
    )
    if ($exitCode -ne 0) {
        throw "Prepared bootstrap checkpoint is stale, altered, or mixed for $ShardId."
    }
    [void](Get-BootstrapPreparedPath -ShardId $ShardId)
    return $true
}

function Test-BootstrapCacheCheckpoint {
    param(
        [Parameter(Mandatory = $true)][string]$ShardId,
        [Parameter(Mandatory = $true)][int]$ChunkIndex,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $pointer = Get-BootstrapCachePointerPath -ShardId $ShardId -ChunkIndex $ChunkIndex
    if (-not (Test-Path -LiteralPath $pointer -PathType Leaf)) { return $false }
    $exitCode = Invoke-CheckpointTool -Stage "bootstrap-verify-cache-$ShardId-$ChunkIndex" -Arguments @(
        "verify-bootstrap-cache", "--plan", $planPath, "--shard-dir", $shardDirectory,
        "--shard-id", $ShardId, "--chunk-index", $ChunkIndex.ToString(),
        "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
    )
    if ($exitCode -ne 0) {
        throw "Bootstrap cache checkpoint is stale, altered, or mixed for $ShardId chunk $ChunkIndex."
    }
    [void](Get-BootstrapCachePath -ShardId $ShardId -ChunkIndex $ChunkIndex)
    return $true
}

function Set-NextBootstrapChunk {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [int]$StartIndex = 0
    )
    for ($chunkIndex = $StartIndex; $chunkIndex -lt $bootstrapChunkCount; $chunkIndex++) {
        $hasCache = Test-BootstrapCacheCheckpoint -ShardId $State.ShardId -ChunkIndex $chunkIndex `
            -ExecutableSha256 $ExecutableSha256 -SourceCommit $SourceCommit
        if (-not $hasCache) {
            $State.Phase = "chunk"
            $State.ChunkIndex = $chunkIndex
            return
        }
        $cachePath = Get-BootstrapCachePath -ShardId $State.ShardId -ChunkIndex $chunkIndex
        $cacheState = Get-Content -LiteralPath $cachePath -Raw | ConvertFrom-Json -Depth 100
        if (-not [bool]$cacheState.completed) {
            $State.Phase = "chunk"
            $State.ChunkIndex = $chunkIndex
            return
        }
    }
    $State.Phase = "finalize"
    $State.ChunkIndex = $bootstrapChunkCount
}

function New-BootstrapState {
    param(
        [Parameter(Mandatory = $true)]$Spec,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $shardId = [string]$Spec.shard_id
    $cellDirectory = Get-BootstrapCellDirectory -ShardId $shardId
    New-Item -ItemType Directory -Path $cellDirectory -Force | Out-Null
    $state = [pscustomobject]@{
        Spec = $Spec
        ShardId = $shardId
        Phase = "prepare"
        ChunkIndex = 0
        Job = $null
        TemporaryOutput = $null
        Done = $false
    }
    if (Test-BootstrapPreparedCheckpoint -ShardId $shardId `
        -ExecutableSha256 $ExecutableSha256 -SourceCommit $SourceCommit) {
        Set-NextBootstrapChunk -State $state -ExecutableSha256 $ExecutableSha256 `
            -SourceCommit $SourceCommit
    }
    return $state
}

function Start-BootstrapStateAttempt {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $internalBudgetSeconds = [Math]::Min(
        $BootstrapProcessBudgetSeconds,
        [Math]::Max(1, $PerShardTimeoutSeconds - 30)
    )
    Assert-BootstrapChildBudget -InternalBudgetSeconds $internalBudgetSeconds
    $identity = [Guid]::NewGuid().ToString("N")
    $dependencyArguments = @(Get-DependencyArguments -Spec $State.Spec)
    $stage = "bootstrap-$($State.Phase)-$($State.ShardId)"
    if ($State.Phase -eq "prepare") {
        $temporaryOutput = Join-Path $resolvedWorkRoot `
            (".{0}.prepared.{1}.tmp.json" -f $State.ShardId, $identity)
        $arguments = @(
            "--output", $temporaryOutput, "--scale", $Scale, "--seed", $Seed.ToString(),
            "--bootstrap-prepare", $State.ShardId, "--budget-seconds", $internalBudgetSeconds.ToString()
        ) + $dependencyArguments
    }
    elseif ($State.Phase -eq "chunk") {
        $preparedPath = Get-BootstrapPreparedPath -ShardId $State.ShardId
        $temporaryOutput = Join-Path $resolvedWorkRoot `
            (".{0}.cache-{1:D3}.{2}.tmp.json" -f $State.ShardId, $State.ChunkIndex, $identity)
        $arguments = @(
            "--output", $temporaryOutput, "--scale", $Scale, "--seed", $Seed.ToString(),
            "--bootstrap-chunk", $State.ShardId, "--prepared-execution", $preparedPath,
            "--chunk-index", $State.ChunkIndex.ToString(), "--chunk-count", $bootstrapChunkCount.ToString(),
            "--budget-seconds", $internalBudgetSeconds.ToString()
        ) + $dependencyArguments
        $cachePointer = Get-BootstrapCachePointerPath -ShardId $State.ShardId `
            -ChunkIndex $State.ChunkIndex
        if (Test-Path -LiteralPath $cachePointer -PathType Leaf) {
            $cachePath = Get-BootstrapCachePath -ShardId $State.ShardId `
                -ChunkIndex $State.ChunkIndex
            $arguments += @("--resume-cache", $cachePath)
        }
        $stage += "-$($State.ChunkIndex)"
    }
    elseif ($State.Phase -eq "finalize") {
        $preparedPath = Get-BootstrapPreparedPath -ShardId $State.ShardId
        $cacheInventory = Join-Path (Get-BootstrapCellDirectory -ShardId $State.ShardId) `
            "cache-inventory.json"
        $inventoryExit = Invoke-CheckpointTool -Stage "bootstrap-inventory-$($State.ShardId)" -Arguments @(
            "write-bootstrap-inventory", "--plan", $planPath, "--shard-dir", $shardDirectory,
            "--shard-id", $State.ShardId, "--output", $cacheInventory,
            "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
        )
        if ($inventoryExit -ne 0) {
            throw "Bootstrap finalization inventory is incomplete, altered, or mixed for $($State.ShardId)."
        }
        $temporaryOutput = Join-Path $resolvedWorkRoot `
            (".{0}.finalize.{1}.tmp.json" -f $State.ShardId, $identity)
        $arguments = @(
            "--output", $temporaryOutput, "--scale", $Scale, "--seed", $Seed.ToString(),
            "--bootstrap-finalize", $State.ShardId, "--prepared-execution", $preparedPath,
            "--cache-inventory", $cacheInventory
        ) + $dependencyArguments
    }
    else {
        throw "Bootstrap state $($State.ShardId) has invalid phase $($State.Phase)."
    }
    $stdout = Join-Path $logDirectory "$stage.$identity.stdout.log"
    $stderr = Join-Path $logDirectory "$stage.$identity.stderr.log"
    $job = Start-BoundedChild -Stage $stage -FileName $binary -Arguments $arguments `
        -StdoutPath $stdout -StderrPath $stderr -TimeoutSeconds $PerShardTimeoutSeconds
    $job | Add-Member -NotePropertyName ShardId -NotePropertyValue $State.ShardId
    $job | Add-Member -NotePropertyName ExecutableSha256 -NotePropertyValue $ExecutableSha256
    $job | Add-Member -NotePropertyName SourceCommit -NotePropertyValue $SourceCommit
    $State.TemporaryOutput = $temporaryOutput
    $State.Job = $job
    $active[$State.ShardId] = $job
}

function Complete-BootstrapStateAttempt {
    param(
        [Parameter(Mandatory = $true)]$State,
        [Parameter(Mandatory = $true)][string]$ExecutableSha256,
        [Parameter(Mandatory = $true)][string]$SourceCommit
    )
    $exitCode = Wait-BoundedChild -Job $State.Job
    [void]$active.Remove($State.ShardId)
    if ($exitCode -ne 0 -or -not (Test-Path -LiteralPath $State.TemporaryOutput -PathType Leaf)) {
        $reason = if ($exitCode -ne 0) { "bootstrap_producer_exit_nonzero" } else { "bootstrap_producer_output_absent" }
        Write-ShardFailureReceipt -Job $State.Job -Reason $reason -ExitCode $exitCode
        throw "Bootstrap $($State.Phase) attempt for $($State.ShardId) failed or made zero verified progress."
    }
    if ($State.Phase -eq "prepare") {
        $sealExit = Invoke-CheckpointTool -Stage "bootstrap-seal-prepared-$($State.ShardId)" -Arguments @(
            "seal-bootstrap-prepared", "--plan", $planPath, "--shard-dir", $shardDirectory,
            "--shard-id", $State.ShardId, "--temporary-prepared", $State.TemporaryOutput,
            "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
        )
        if ($sealExit -ne 0) {
            throw "Prepared bootstrap execution failed its atomic receipt for $($State.ShardId)."
        }
        Set-NextBootstrapChunk -State $State -ExecutableSha256 $ExecutableSha256 `
            -SourceCommit $SourceCommit
    }
    elseif ($State.Phase -eq "chunk") {
        $sealExit = Invoke-CheckpointTool -Stage "bootstrap-seal-cache-$($State.ShardId)-$($State.ChunkIndex)" -Arguments @(
            "seal-bootstrap-cache", "--plan", $planPath, "--shard-dir", $shardDirectory,
            "--shard-id", $State.ShardId, "--chunk-index", $State.ChunkIndex.ToString(),
            "--temporary-cache", $State.TemporaryOutput, "--executable-sha256", $ExecutableSha256,
            "--source-commit", $SourceCommit
        )
        if ($sealExit -ne 0) {
            throw "Bootstrap chunk $($State.ChunkIndex) for $($State.ShardId) failed progress, identity, or atomic-cache validation."
        }
        $cachePath = Get-BootstrapCachePath -ShardId $State.ShardId -ChunkIndex $State.ChunkIndex
        $cacheState = Get-Content -LiteralPath $cachePath -Raw | ConvertFrom-Json -Depth 100
        Write-Host ("[heterogeneity] {0} chunk {1:D3}/{2:D3}: {3}/{4} records" -f `
            $State.ShardId, $State.ChunkIndex, $bootstrapChunkCount, $cacheState.record_count, $cacheState.expected_record_count)
        if ([bool]$cacheState.completed) {
            Set-NextBootstrapChunk -State $State -ExecutableSha256 $ExecutableSha256 `
                -SourceCommit $SourceCommit -StartIndex ($State.ChunkIndex + 1)
        }
    }
    elseif ($State.Phase -eq "finalize") {
        $sealExit = Invoke-CheckpointTool -Stage "checkpoint-seal-$($State.ShardId)" -Arguments @(
            "seal", "--plan", $planPath, "--shard-dir", $shardDirectory,
            "--shard-id", $State.ShardId, "--temporary-result", $State.TemporaryOutput,
            "--executable-sha256", $ExecutableSha256, "--source-commit", $SourceCommit
        ) -TimeoutSeconds $PerShardTimeoutSeconds
        if ($sealExit -ne 0) {
            throw "Finalized bootstrap scientific shard $($State.ShardId) failed its exact 100-cache receipt."
        }
        $State.Done = $true
        Write-Host "[heterogeneity] completed resumable bootstrap shard $($State.ShardId)"
    }
    $State.Job = $null
    $State.TemporaryOutput = $null
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
            throw "Heterogeneity qualification requires a fully clean commit-bound source tree, including no untracked files."
        }
        $sourceCommit = (& git rev-parse HEAD).Trim().ToLowerInvariant()
        if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
            throw "Unable to resolve the exact source commit."
        }

        New-Item -ItemType Directory -Path $resolvedWorkRoot, $shardDirectory, $logDirectory, $historyDirectory, $bootstrapDirectory -Force | Out-Null
        Write-Host "[heterogeneity] one Cargo build; point cells use scientific checkpoints and seven typed n=80 dual-outcome bootstrap cells use prepare/100 modulo caches/finalize"
        $buildExit = Invoke-BoundedStage -Stage "cargo-build" -FileName "cargo" -Arguments @(
            "build", "--release", "--quiet", "--locked", "-p", "qpls-runner", "--example",
            "multimod_heterogeneity_qualification_v2"
        ) -TimeoutSeconds (Get-RemainingBudgetSeconds)
        if ($buildExit -ne 0) {
            throw "The single heterogeneity producer build failed."
        }
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "The built heterogeneity producer executable is absent: $binary"
        }
        Assert-TimeBudget
        $executableSha256 = Get-Sha256 -Path $binary

        $temporaryPlan = Join-Path $resolvedWorkRoot (".shard-plan.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $planExit = Invoke-BoundedStage -Stage "plan" -FileName $binary -Arguments @(
            "--output", $temporaryPlan, "--scale", $Scale, "--seed", $Seed.ToString(), "--plan"
        ) -TimeoutSeconds 120
        if ($planExit -ne 0 -or -not (Test-Path -LiteralPath $temporaryPlan -PathType Leaf)) {
            throw "The deterministic heterogeneity shard plan could not be generated."
        }
        Move-Item -LiteralPath $temporaryPlan -Destination $planPath -Force
        $plan = Get-Content -LiteralPath $planPath -Raw | ConvertFrom-Json -Depth 100
        $specs = @($plan.shards)
        if ($specs.Count -eq 0 -or [string]$specs[0].shard_id -ne "sentinel" -or
            [string]$plan.metamorphism -ne "baseline" -or $null -ne $plan.sign_columns -or
            [int]$plan.workers -ne 1 -or [int]$plan.fixture_observations -ne 400) {
            throw "The shard plan did not preserve the baseline 400-row qualification identity with the fast root sentinel first."
        }
        $multiclassPointFixturePlanProperty = $plan.PSObject.Properties["multiclass_point_fixture_plan"]
        if ($null -eq $multiclassPointFixturePlanProperty -or $null -eq $multiclassPointFixturePlanProperty.Value) {
            throw "The shard plan did not declare the required typed per-K multiclass point fixture plan."
        }
        $multiclassPointFixturePlan = $multiclassPointFixturePlanProperty.Value
        $expectedMulticlassPointFixturePlanProperties = @(
            "schema_version",
            "plan_id",
            "purpose",
            "selected_k",
            "fixture_shapes",
            "allocation",
            "bootstrap_evidence"
        ) | Sort-Object
        $actualMulticlassPointFixturePlanProperties = @(
            $multiclassPointFixturePlan.PSObject.Properties.Name | Sort-Object
        )
        $multiclassPointFixturePlanPropertyDifference = @(
            Compare-Object -ReferenceObject $expectedMulticlassPointFixturePlanProperties `
                -DifferenceObject $actualMulticlassPointFixturePlanProperties
        )
        $expectedMulticlassPointFixtureShapes = @(
            [pscustomobject]@{ selected_k = 3; observations_per_fixture = 120; expected_cases_per_true_class = 40 },
            [pscustomobject]@{ selected_k = 4; observations_per_fixture = 120; expected_cases_per_true_class = 30 },
            [pscustomobject]@{ selected_k = 5; observations_per_fixture = 200; expected_cases_per_true_class = 40 }
        )
        $selectedK = @($multiclassPointFixturePlan.selected_k)
        $fixtureShapes = @($multiclassPointFixturePlan.fixture_shapes)
        $multiclassPointFixtureShapesValid = $fixtureShapes.Count -eq 3
        if ($multiclassPointFixtureShapesValid) {
            for ($index = 0; $index -lt $expectedMulticlassPointFixtureShapes.Count; $index++) {
                $actualShape = $fixtureShapes[$index]
                $expectedShape = $expectedMulticlassPointFixtureShapes[$index]
                $expectedShapeProperties = @(
                    "selected_k", "observations_per_fixture", "expected_cases_per_true_class"
                ) | Sort-Object
                $actualShapeProperties = @($actualShape.PSObject.Properties.Name | Sort-Object)
                $shapePropertyDifference = @(
                    Compare-Object -ReferenceObject $expectedShapeProperties `
                        -DifferenceObject $actualShapeProperties
                )
                if ($shapePropertyDifference.Count -ne 0 -or
                    $actualShape.selected_k -isnot [System.Int64] -or
                    [int]$actualShape.selected_k -ne [int]$expectedShape.selected_k -or
                    $actualShape.observations_per_fixture -isnot [System.Int64] -or
                    [int]$actualShape.observations_per_fixture -ne [int]$expectedShape.observations_per_fixture -or
                    $actualShape.expected_cases_per_true_class -isnot [System.Int64] -or
                    [int]$actualShape.expected_cases_per_true_class -ne [int]$expectedShape.expected_cases_per_true_class) {
                    $multiclassPointFixtureShapesValid = $false
                    break
                }
            }
        }
        if ($multiclassPointFixturePlanPropertyDifference.Count -ne 0 -or
            $multiclassPointFixturePlan.schema_version -isnot [System.Int64] -or
            [int]$multiclassPointFixturePlan.schema_version -ne 2 -or
            $multiclassPointFixturePlan.plan_id -isnot [string] -or
            [string]$multiclassPointFixturePlan.plan_id -cne "qpls.multimod.heterogeneity.pos-published-p0-k3-k5-point-discovery.v2" -or
            $multiclassPointFixturePlan.purpose -isnot [string] -or
            [string]$multiclassPointFixturePlan.purpose -cne "published_p0_pos_candidate_point_discovery_only" -or
            $selectedK.Count -ne 3 -or
            $selectedK[0] -isnot [System.Int64] -or [int]$selectedK[0] -ne 3 -or
            $selectedK[1] -isnot [System.Int64] -or [int]$selectedK[1] -ne 4 -or
            $selectedK[2] -isnot [System.Int64] -or [int]$selectedK[2] -ne 5 -or
            -not $multiclassPointFixtureShapesValid -or
            $multiclassPointFixturePlan.allocation -isnot [string] -or
            [string]$multiclassPointFixturePlan.allocation -cne "row_mod_k_exactly_balanced" -or
            $multiclassPointFixturePlan.bootstrap_evidence -isnot [string] -or
            [string]$multiclassPointFixturePlan.bootstrap_evidence -cne "not_requested") {
            throw "The shard plan multiclass_point_fixture_plan differs from the exact typed K3/K4 n=120 and K5 n=200 point-only qualification contract."
        }
        $bootstrapFixturePlanProperty = $plan.PSObject.Properties["bootstrap_fixture_plan"]
        if ($null -eq $bootstrapFixturePlanProperty -or $null -eq $bootstrapFixturePlanProperty.Value) {
            throw "The shard plan did not declare the required typed n=80 dual-outcome fixed-K bootstrap fixture plan."
        }
        $bootstrapFixturePlan = $bootstrapFixturePlanProperty.Value
        $expectedBootstrapFixturePlanProperties = @(
            "schema_version",
            "plan_id",
            "purpose",
            "selected_k",
            "observations_per_fixture",
            "expected_cases_per_true_class",
            "interaction_fixture_design",
            "requested_replicates",
            "performance_scope"
        ) | Sort-Object
        $actualBootstrapFixturePlanProperties = @(
            $bootstrapFixturePlan.PSObject.Properties.Name | Sort-Object
        )
        $bootstrapFixturePlanPropertyDifference = @(
            Compare-Object -ReferenceObject $expectedBootstrapFixturePlanProperties `
                -DifferenceObject $actualBootstrapFixturePlanProperties
        )
        if ($bootstrapFixturePlanPropertyDifference.Count -ne 0 -or
            $bootstrapFixturePlan.schema_version -isnot [System.Int64] -or
            [int]$bootstrapFixturePlan.schema_version -ne 1 -or
            $bootstrapFixturePlan.plan_id -isnot [string] -or
            [string]$bootstrapFixturePlan.plan_id -cne "qpls.multimod.heterogeneity.k2-fixed-bootstrap-n80-dual-outcome.v1" -or
            $bootstrapFixturePlan.purpose -isnot [string] -or
            [string]$bootstrapFixturePlan.purpose -cne "fixed_k_full_pipeline_bootstrap_inference_and_ledger_qualification" -or
            $bootstrapFixturePlan.selected_k -isnot [System.Int64] -or
            [int]$bootstrapFixturePlan.selected_k -ne 2 -or
            $bootstrapFixturePlan.observations_per_fixture -isnot [System.Int64] -or
            [int]$bootstrapFixturePlan.observations_per_fixture -ne 80 -or
            $bootstrapFixturePlan.expected_cases_per_true_class -isnot [System.Int64] -or
            [int]$bootstrapFixturePlan.expected_cases_per_true_class -ne 40 -or
            $bootstrapFixturePlan.interaction_fixture_design -isnot [string] -or
            [string]$bootstrapFixturePlan.interaction_fixture_design -cne "dual_endogenous_anchor_v1" -or
            $bootstrapFixturePlan.requested_replicates -isnot [System.Int64] -or
            [int]$bootstrapFixturePlan.requested_replicates -ne 500 -or
            $bootstrapFixturePlan.performance_scope -isnot [string] -or
            [string]$bootstrapFixturePlan.performance_scope -cne "n80_fixed_k_bootstrap_not_a_500_draw_n400_runtime_claim") {
            throw "The shard plan bootstrap_fixture_plan differs from the exact typed n=80 dual-outcome, K=2, 500-draw qualification contract."
        }

        $completed = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
        $sentinel = $specs[0]
        if (Test-ShardCheckpoint -ShardId "sentinel" -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit) {
            [void]$completed.Add("sentinel")
            Write-Host "[heterogeneity] reused verified sentinel checkpoint"
        }
        else {
            Write-Host "[heterogeneity] running fast root sentinel before expensive cells"
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
            Write-Host ("[heterogeneity] reused {0} verified shard checkpoints" -f ($completed.Count - 1))
        }

        $pointSpecs = @($specs | Where-Object { [string]$_.resource_class -ne "bootstrap" })
        while (@($pointSpecs | Where-Object { -not $completed.Contains([string]$_.shard_id) }).Count -gt 0) {
            Assert-TimeBudget
            $startedAny = $false
            foreach ($spec in $pointSpecs) {
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
                $job = Start-Shard -Spec $spec -ExecutableSha256 $executableSha256 -SourceCommit $sourceCommit
                $active[$shardId] = $job
                $startedAny = $true
                Write-Host "[heterogeneity] started shard $shardId"
            }

            $completedAny = $false
            foreach ($shardId in @($active.Keys)) {
                $job = $active[$shardId]
                if (-not $job.Process.HasExited) {
                    if ($job.Clock.Elapsed.TotalSeconds -ge $job.TimeoutSeconds) {
                        Stop-BoundedChild -Job $job -Reason "shard_timeout"
                        Write-ShardFailureReceipt -Job $job -Reason "shard_timeout" -ExitCode 124
                        $active.Remove($shardId)
                        throw "Heterogeneity shard $shardId exceeded its $PerShardTimeoutSeconds-second timeout; completed checkpoints are retained."
                    }
                    continue
                }
                Complete-Shard -Job $job
                $active.Remove($shardId)
                [void]$completed.Add($shardId)
                $completedAny = $true
            }
            if (-not $startedAny -and -not $completedAny -and $active.Count -eq 0) {
                throw "The heterogeneity shard graph made no progress; a dependency is missing or cyclic."
            }
            if (-not $completedAny) { Start-Sleep -Milliseconds 250 }
        }

        $bootstrapSpecs = @($specs | Where-Object { [string]$_.resource_class -eq "bootstrap" })
        if ($bootstrapSpecs.Count -gt 0) {
            Write-Host ("[heterogeneity] running {0} retained bootstrap profiles on the typed n=80 dual-outcome fixture with exact 500-draw ledgers; up to {1} three-thread scientific cells run concurrently and verified chunks resume without repeating completed draws" -f $bootstrapSpecs.Count, $MaxParallelBootstrapShards)
        }
        $bootstrapStates = [System.Collections.Generic.List[object]]::new()
        foreach ($spec in $bootstrapSpecs) {
            $shardId = [string]$spec.shard_id
            if ($completed.Contains($shardId)) { continue }
            foreach ($dependencyId in @($spec.dependencies)) {
                if (-not $completed.Contains([string]$dependencyId)) {
                    throw "Bootstrap shard $shardId cannot start because dependency $dependencyId is incomplete."
                }
            }
            Move-StaleShardFiles -ShardId $shardId
            $state = New-BootstrapState -Spec $spec -ExecutableSha256 $executableSha256 `
                -SourceCommit $sourceCommit
            [void]$bootstrapStates.Add($state)
        }
        while (@($bootstrapStates | Where-Object { -not $_.Done }).Count -gt 0) {
            Assert-TimeBudget
            $startedAny = $false
            foreach ($state in $bootstrapStates) {
                if ($active.Count -ge $MaxParallelBootstrapShards) { break }
                if ($state.Done -or $null -ne $state.Job) { continue }
                Start-BootstrapStateAttempt -State $state -ExecutableSha256 $executableSha256 `
                    -SourceCommit $sourceCommit
                $startedAny = $true
                Write-Host "[heterogeneity] started $($state.ShardId) phase $($state.Phase)"
            }

            $completedAny = $false
            foreach ($state in $bootstrapStates) {
                if ($null -eq $state.Job) { continue }
                if (-not $state.Job.Process.HasExited) {
                    if ($state.Job.Clock.Elapsed.TotalSeconds -ge $state.Job.TimeoutSeconds) {
                        Stop-BoundedChild -Job $state.Job -Reason "bootstrap_child_timeout"
                        Write-ShardFailureReceipt -Job $state.Job -Reason "bootstrap_child_timeout" -ExitCode 124
                        [void]$active.Remove($state.ShardId)
                        throw "Bootstrap child for $($state.ShardId) exceeded the external $PerShardTimeoutSeconds-second hard bound."
                    }
                    continue
                }
                Complete-BootstrapStateAttempt -State $state -ExecutableSha256 $executableSha256 `
                    -SourceCommit $sourceCommit
                if ($state.Done) {
                    [void]$completed.Add($state.ShardId)
                }
                $completedAny = $true
            }
            if (-not $startedAny -and -not $completedAny -and $active.Count -eq 0) {
                throw "The resumable bootstrap scheduler made no progress."
            }
            if (-not $completedAny) { Start-Sleep -Milliseconds 250 }
        }

        Assert-TimeBudget
        $aggregateExit = Invoke-CheckpointTool -Arguments @(
            "aggregate", "--plan", $planPath, "--shard-dir", $shardDirectory,
            "--executable-sha256", $executableSha256, "--source-commit", $sourceCommit,
            "--output", $rawAggregate
        ) -TimeoutSeconds $PerShardTimeoutSeconds -Stage "aggregate"
        if ($aggregateExit -ne 0) {
            throw "Deterministic heterogeneity shard aggregation failed."
        }

        $temporaryComparison = Join-Path $resolvedWorkRoot (".comparison.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
        $comparisonExit = Invoke-BoundedStage -Stage "comparator" -FileName "python" -Arguments @(
            $comparator, "--input", $rawAggregate, "--output", $temporaryComparison,
            "--require-shard-receipts"
        ) -TimeoutSeconds $PerShardTimeoutSeconds
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
    Stop-ActiveShards
    Write-Error $_
    exit 1
}
