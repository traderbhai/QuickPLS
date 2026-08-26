[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Output,

    [string]$WorkRoot,

    [ValidateRange(1, 4)]
    [int]$MaxParallelCells = 4,

    [ValidateRange(60, 1800)]
    [int]$PerCellTimeoutSeconds = 1800,

    [ValidateRange(600, 6480)]
    [int]$ScientificTimeoutSeconds = 6480,

    [ValidateRange(600, 6600)]
    [int]$OverallTimeoutSeconds = 6600
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$resolvedOutput = [System.IO.Path]::GetFullPath($Output)
$resolvedWorkRoot = if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    "$resolvedOutput.metamorphic-cells"
}
else {
    [System.IO.Path]::GetFullPath($WorkRoot)
}
$cellDirectory = Join-Path $resolvedWorkRoot "cells"
$logDirectory = Join-Path $resolvedWorkRoot "logs"
$historyDirectory = Join-Path $resolvedWorkRoot "_attempt_history"
$planPath = Join-Path $resolvedWorkRoot "cell-plan.json"
$statusPath = Join-Path $resolvedWorkRoot "cell-status.json"
$buildReceiptPath = Join-Path $resolvedWorkRoot "single-build-receipt.json"
$executionReceiptPath = "$resolvedOutput.execution-receipt.json"
$checkpointTool = Join-Path $repositoryRoot "validation/multimod/multimod_metamorphic_cells_v1.py"
$verifier = Join-Path $repositoryRoot "validation/multimod/verify_multimod_metamorphic_qualification_v1.py"
$capabilityIndex = Join-Path $repositoryRoot "validation/multimod/multimod_capability_index_v1.json"
$cleanupReserveSeconds = 120
$wrapperClock = [System.Diagnostics.Stopwatch]::StartNew()
$scientificClock = $null
$active = @{}

$families = @(
    [pscustomobject]@{ Id = "mga"; Example = "multimod_mga_qualification_v1" },
    [pscustomobject]@{ Id = "heterogeneity"; Example = "multimod_heterogeneity_qualification_v2" },
    [pscustomobject]@{ Id = "conditional"; Example = "multimod_conditional_qualification_v1" },
    [pscustomobject]@{ Id = "causal"; Example = "multimod_causal_qualification_v1" }
)
$binaries = [ordered]@{}
foreach ($family in $families) {
    $binaries[$family.Id] = Join-Path $repositoryRoot ("target/release/examples/{0}.exe" -f $family.Example)
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-TextSha256 {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([Convert]::ToHexString($algorithm.ComputeHash($bytes))).ToLowerInvariant()
    }
    finally {
        $algorithm.Dispose()
    }
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
    $temporary = Join-Path $parent (".{0}.{1}.tmp" -f ([System.IO.Path]::GetFileName($Path)), [Guid]::NewGuid().ToString("N"))
    try {
        [System.IO.File]::WriteAllText($temporary, $Text, [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Move($temporary, $Path, $true)
    }
    finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Write-JsonAtomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )
    Write-TextAtomic -Path $Path -Text (($Value | ConvertTo-Json -Depth 100) + "`n")
}

function Get-RemainingWrapperWorkSeconds {
    $remaining = [Math]::Floor(
        $OverallTimeoutSeconds - $cleanupReserveSeconds - $wrapperClock.Elapsed.TotalSeconds
    )
    if ($remaining -lt 1) {
        throw "Metamorphic qualification exhausted its bounded work budget; $cleanupReserveSeconds seconds remain reserved for process-tree termination inside the $OverallTimeoutSeconds-second wrapper cap."
    }
    return [int]$remaining
}

function Get-RemainingScientificWorkSeconds {
    $wrapperRemaining = Get-RemainingWrapperWorkSeconds
    if ($null -eq $scientificClock) {
        return $wrapperRemaining
    }
    $scientificRemaining = [Math]::Floor(
        $ScientificTimeoutSeconds - $scientificClock.Elapsed.TotalSeconds
    )
    if ($scientificRemaining -lt 1) {
        throw "Metamorphic qualification reached its $ScientificTimeoutSeconds-second post-build scientific cap. Exact completed cell receipts are retained."
    }
    return [int][Math]::Min($wrapperRemaining, $scientificRemaining)
}

function Start-BoundedChild {
    param(
        [Parameter(Mandatory = $true)][string]$Stage,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [hashtable]$EnvironmentOverrides = @{},
        [switch]$ScientificBudget
    )
    $remaining = if ($ScientificBudget) {
        Get-RemainingScientificWorkSeconds
    }
    else {
        Get-RemainingWrapperWorkSeconds
    }
    $effectiveTimeout = [int][Math]::Min($TimeoutSeconds, $remaining)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FileName
    $startInfo.WorkingDirectory = $repositoryRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add([string]$argument)
    }
    foreach ($entry in $EnvironmentOverrides.GetEnumerator()) {
        $startInfo.Environment[[string]$entry.Key] = [string]$entry.Value
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    return [pscustomobject]@{
        Stage = $Stage
        Process = $process
        Clock = [System.Diagnostics.Stopwatch]::StartNew()
        TimeoutSeconds = $effectiveTimeout
        ScientificBudget = [bool]$ScientificBudget
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
        StdoutCopy = $process.StandardOutput.ReadToEndAsync()
        StderrCopy = $process.StandardError.ReadToEndAsync()
        LogsSaved = $false
        StdoutText = ""
        StderrText = ""
    }
}

function Save-ProcessLogs {
    param([Parameter(Mandatory = $true)]$Job)
    if ($Job.LogsSaved) { return }
    $tasks = [System.Threading.Tasks.Task[]]@($Job.StdoutCopy, $Job.StderrCopy)
    if (-not [System.Threading.Tasks.Task]::WaitAll($tasks, 10000)) {
        throw "Timed out draining redirected streams for $($Job.Stage)."
    }
    $Job.StdoutText = [string]$Job.StdoutCopy.Result
    $Job.StderrText = [string]$Job.StderrCopy.Result
    Write-TextAtomic -Path $Job.StdoutPath -Text $Job.StdoutText
    Write-TextAtomic -Path $Job.StderrPath -Text $Job.StderrText
    $Job.LogsSaved = $true
}

function Stop-BoundedChild {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    if (-not $Job.Process.HasExited) {
        try {
            # Kill(entireProcessTree: true) prevents a timed-out producer from
            # retaining descendants after the wrapper returns.
            $Job.Process.Kill($true)
        }
        catch {
            Write-Warning "Process-tree termination failed for $($Job.Stage): $($_.Exception.Message)"
        }
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        throw "Could not terminate $($Job.Stage) and its process tree after $Reason."
    }
    Save-ProcessLogs -Job $Job
}

function Test-JobGlobalBudgetExceeded {
    param([Parameter(Mandatory = $true)]$Job)
    if ($wrapperClock.Elapsed.TotalSeconds -ge ($OverallTimeoutSeconds - $cleanupReserveSeconds)) {
        return $true
    }
    if ($Job.ScientificBudget -and $null -ne $scientificClock -and
        $scientificClock.Elapsed.TotalSeconds -ge $ScientificTimeoutSeconds) {
        return $true
    }
    return $false
}

function Get-JobElapsedMilliseconds {
    param([Parameter(Mandatory = $true)]$Job)
    if ($Job.Process.HasExited) {
        try {
            return [long][Math]::Max(
                0,
                ($Job.Process.ExitTime.ToUniversalTime() - $Job.Process.StartTime.ToUniversalTime()).TotalMilliseconds
            )
        }
        catch {
            # The monotonic clock remains a conservative fallback if the OS no
            # longer exposes process timestamps after termination.
        }
    }
    return [long]$Job.Clock.ElapsedMilliseconds
}

function Wait-BoundedChild {
    param([Parameter(Mandatory = $true)]$Job)
    while (-not $Job.Process.HasExited) {
        if ($Job.Clock.Elapsed.TotalSeconds -ge $Job.TimeoutSeconds -or
            (Test-JobGlobalBudgetExceeded -Job $Job)) {
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
    $elapsedMilliseconds = Get-JobElapsedMilliseconds -Job $Job
    if ($elapsedMilliseconds -gt ([long]$Job.TimeoutSeconds * 1000)) {
        throw "$($Job.Stage) completed after its exact $($Job.TimeoutSeconds)-second process cap."
    }
    return [pscustomobject]@{
        ExitCode = [int]$Job.Process.ExitCode
        ElapsedMilliseconds = $elapsedMilliseconds
        StdoutPath = [string]$Job.StdoutPath
        StderrPath = [string]$Job.StderrPath
        Stdout = [string]$Job.StdoutText
        Stderr = [string]$Job.StderrText
    }
}

function Invoke-BoundedStage {
    param(
        [Parameter(Mandatory = $true)][string]$Stage,
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [switch]$ScientificBudget
    )
    $identity = [Guid]::NewGuid().ToString("N")
    $stdout = Join-Path $logDirectory ("_{0}.{1}.stdout.log" -f $Stage, $identity)
    $stderr = Join-Path $logDirectory ("_{0}.{1}.stderr.log" -f $Stage, $identity)
    $job = Start-BoundedChild -Stage $Stage -FileName $FileName -Arguments $Arguments `
        -StdoutPath $stdout -StderrPath $stderr -TimeoutSeconds $TimeoutSeconds `
        -ScientificBudget:$ScientificBudget
    try {
        return Wait-BoundedChild -Job $job
    }
    finally {
        $job.Process.Dispose()
    }
}

function Invoke-CheckpointTool {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments,
        [int]$TimeoutSeconds = 120
    )
    return Invoke-BoundedStage -Stage "checkpoint-$Command" -FileName "python" `
        -Arguments (@($checkpointTool, $Command) + $Arguments) `
        -TimeoutSeconds $TimeoutSeconds -ScientificBudget
}

function Move-StaleCellFiles {
    param([Parameter(Mandatory = $true)][string]$CellId)
    $paths = @(
        @(
            (Join-Path $cellDirectory "$CellId.json"),
            (Join-Path $cellDirectory "$CellId.result.tmp.json"),
            (Join-Path $cellDirectory "$CellId.receipt.json"),
            (Join-Path $cellDirectory "$CellId.failure.json"),
            (Join-Path $logDirectory "$CellId.stdout.log"),
            (Join-Path $logDirectory "$CellId.stderr.log")
        ) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
    )
    if ($paths.Count -eq 0) { return }
    $attemptRoot = Join-Path $historyDirectory (
        "{0}-{1}-{2}" -f $CellId, [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssfffZ"), [Guid]::NewGuid().ToString("N")
    )
    New-Item -ItemType Directory -Path $attemptRoot -Force | Out-Null
    foreach ($path in $paths) {
        Move-Item -LiteralPath $path -Destination (Join-Path $attemptRoot ([System.IO.Path]::GetFileName($path)))
    }
}

function Preserve-UnpublishedCellResult {
    param([Parameter(Mandatory = $true)]$Job)
    if (-not (Test-Path -LiteralPath $Job.TemporaryResult -PathType Leaf)) { return }
    $attemptRoot = Join-Path $historyDirectory (
        "{0}-unpublished-{1}-{2}" -f $Job.CellId, [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssfffZ"), [Guid]::NewGuid().ToString("N")
    )
    New-Item -ItemType Directory -Path $attemptRoot -Force | Out-Null
    Move-Item -LiteralPath $Job.TemporaryResult -Destination (Join-Path $attemptRoot "unpublished-result.json")
}

function Write-CellFailureReceipt {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    $receipt = [ordered]@{
        schema_version = 1
        receipt_id = "qpls.multimod.metamorphic.cell-failure.v1"
        status = "failed"
        cell_id = [string]$Job.CellId
        family = [string]$Job.Family
        axis = [string]$Job.Axis
        source_commit = [string]$Job.SourceCommit
        executable_path = [string]$Job.Executable
        executable_sha256 = [string]$Job.ExecutableSha256
        arguments = @($Job.Arguments)
        reason = $Reason
        elapsed_milliseconds = Get-JobElapsedMilliseconds -Job $Job
        stdout_path = [string]$Job.StdoutPath
        stdout_sha256 = if (Test-Path -LiteralPath $Job.StdoutPath -PathType Leaf) { Get-Sha256 $Job.StdoutPath } else { $null }
        stderr_path = [string]$Job.StderrPath
        stderr_sha256 = if (Test-Path -LiteralPath $Job.StderrPath -PathType Leaf) { Get-Sha256 $Job.StderrPath } else { $null }
    }
    Write-JsonAtomic -Path (Join-Path $cellDirectory "$($Job.CellId).failure.json") -Value $receipt
}

function Start-Cell {
    param(
        [Parameter(Mandatory = $true)]$Spec,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.HashSet[string]]$Completed
    )
    $cellId = [string]$Spec.cell_id
    foreach ($dependency in @($Spec.dependencies)) {
        if (-not $Completed.Contains([string]$dependency)) {
            throw "Metamorphic cell $cellId cannot start before successful root $dependency."
        }
    }
    Move-StaleCellFiles -CellId $cellId
    $executable = [string]$binaries[[string]$Spec.family]
    $executableSha256 = Get-Sha256 -Path $executable
    $temporaryResult = Join-Path $cellDirectory "$cellId.result.tmp.json"
    $stdout = Join-Path $logDirectory "$cellId.stdout.log"
    $stderr = Join-Path $logDirectory "$cellId.stderr.log"
    $arguments = @("--scale", "development", "--output", $temporaryResult)
    $environment = @{
        QPLS_MULTIMOD_METAMORPHISM_V1 = [string]$Spec.axis
        QPLS_MULTIMOD_SIGN_COLUMNS_V1 = [string]$Spec.sign_columns
        QPLS_MULTIMOD_WORKERS_V1 = ([int]$Spec.workers).ToString()
        QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1 = "1"
    }
    $job = Start-BoundedChild -Stage "metamorphic-cell-$cellId" -FileName $executable `
        -Arguments $arguments -StdoutPath $stdout -StderrPath $stderr `
        -TimeoutSeconds $PerCellTimeoutSeconds -EnvironmentOverrides $environment -ScientificBudget
    $job | Add-Member -NotePropertyName CellId -NotePropertyValue $cellId
    $job | Add-Member -NotePropertyName Family -NotePropertyValue ([string]$Spec.family)
    $job | Add-Member -NotePropertyName Axis -NotePropertyValue ([string]$Spec.axis)
    $job | Add-Member -NotePropertyName TemporaryResult -NotePropertyValue $temporaryResult
    $job | Add-Member -NotePropertyName Executable -NotePropertyValue $executable
    $job | Add-Member -NotePropertyName ExecutableSha256 -NotePropertyValue $executableSha256
    $job | Add-Member -NotePropertyName SourceCommit -NotePropertyValue $SourceCommit
    $job | Add-Member -NotePropertyName Arguments -NotePropertyValue $arguments
    return $job
}

function Complete-Cell {
    param([Parameter(Mandatory = $true)]$Job)
    if (-not $Job.Process.HasExited) {
        throw "Complete-Cell requires an exited producer."
    }
    if (-not $Job.Process.WaitForExit(10000)) {
        Stop-BoundedChild -Job $Job -Reason "exit_wait_timeout"
        throw "Metamorphic cell $($Job.CellId) did not finalize after exit."
    }
    Save-ProcessLogs -Job $Job
    if ($Job.Process.ExitCode -ne 0) {
        Write-CellFailureReceipt -Job $Job -Reason "producer_exit_$($Job.Process.ExitCode)"
        Preserve-UnpublishedCellResult -Job $Job
        throw "Metamorphic cell $($Job.CellId) exited with $($Job.Process.ExitCode)."
    }
    if (-not (Test-Path -LiteralPath $Job.TemporaryResult -PathType Leaf)) {
        Write-CellFailureReceipt -Job $Job -Reason "producer_output_absent"
        throw "Metamorphic cell $($Job.CellId) omitted its deterministic output."
    }
    $sealArguments = @(
        "--plan", $planPath,
        "--cell-dir", $cellDirectory,
        "--cell-id", $Job.CellId,
        "--source-commit", $Job.SourceCommit,
        "--executable", $Job.Executable,
        "--executable-sha256", $Job.ExecutableSha256,
        "--temporary-result", $Job.TemporaryResult,
        "--stdout", $Job.StdoutPath,
        "--stderr", $Job.StderrPath,
        "--elapsed-milliseconds", ([string](Get-JobElapsedMilliseconds -Job $Job)),
        "--argument=--scale",
        "--argument=development",
        "--argument=--output",
        ("--argument={0}" -f $Job.TemporaryResult)
    )
    $seal = Invoke-CheckpointTool -Command "seal" -Arguments $sealArguments
    if ($seal.ExitCode -ne 0) {
        Write-CellFailureReceipt -Job $Job -Reason "atomic_seal_rejected"
        Preserve-UnpublishedCellResult -Job $Job
        throw "Metamorphic cell $($Job.CellId) completed but its identity-bound receipt was rejected."
    }
    Write-Host ("[metamorphic] completed {0} in {1:n1}s" -f $Job.CellId, $Job.Clock.Elapsed.TotalSeconds)
}

function Invoke-CellPhase {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$Specs,
        [Parameter(Mandatory = $true)][string]$SourceCommit,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.HashSet[string]]$Completed
    )
    $requiredIds = @($Specs | ForEach-Object { [string]$_.cell_id })
    while (@($requiredIds | Where-Object { -not $Completed.Contains($_) }).Count -gt 0) {
        [void](Get-RemainingScientificWorkSeconds)
        $waveSpecs = @(
            $Specs |
                Where-Object { -not $Completed.Contains([string]$_.cell_id) } |
                Select-Object -First $MaxParallelCells
        )
        if ($waveSpecs.Count -eq 0) {
            throw "Metamorphic $Name phase made no progress; a required root dependency is unavailable."
        }
        $finished = [System.Collections.Generic.List[object]]::new()
        try {
            foreach ($spec in $waveSpecs) {
                $cellId = [string]$spec.cell_id
                $job = Start-Cell -Spec $spec -SourceCommit $SourceCommit -Completed $Completed
                $active[$cellId] = $job
                Write-Host "[metamorphic] started $Name cell $cellId"
            }

            # Keep the supervisor loop free of receipt subprocesses while any
            # scientific producer is live. That guarantees every process tree
            # is observed against its own 1,800-second ceiling. Completed wave
            # members are sealed only after all live members have exited.
            while ($active.Count -gt 0) {
                $madeProgress = $false
                foreach ($cellId in @($active.Keys)) {
                    $job = $active[$cellId]
                    if ($job.Process.HasExited) {
                        $active.Remove($cellId)
                        if (-not $job.Process.WaitForExit(10000)) {
                            throw "Metamorphic cell $cellId did not finalize after exit."
                        }
                        $elapsed = Get-JobElapsedMilliseconds -Job $job
                        $late = $elapsed -gt ([long]$job.TimeoutSeconds * 1000)
                        $globalLate = Test-JobGlobalBudgetExceeded -Job $job
                        if ($late -or $globalLate -or $job.Process.ExitCode -ne 0 -or
                            -not (Test-Path -LiteralPath $job.TemporaryResult -PathType Leaf)) {
                            # End every still-running peer before log/receipt IO
                            # so a failed member cannot push another tree over
                            # its configured ceiling.
                            foreach ($peer in @($active.Values)) {
                                if (-not $peer.Process.HasExited) {
                                    try { $peer.Process.Kill($true) } catch { Write-Warning $_.Exception.Message }
                                }
                            }
                            Save-ProcessLogs -Job $job
                            $reason = if ($late) {
                                "bounded_cell_timeout"
                            }
                            elseif ($globalLate) {
                                "bounded_scientific_timeout"
                            }
                            elseif ($job.Process.ExitCode -ne 0) {
                                "producer_exit_$($job.Process.ExitCode)"
                            }
                            else {
                                "producer_output_absent"
                            }
                            Write-CellFailureReceipt -Job $job -Reason $reason
                            Preserve-UnpublishedCellResult -Job $job
                            $job.Process.Dispose()
                            throw "Metamorphic cell $cellId failed ($reason); completed receipts remain reusable."
                        }
                        $finished.Add($job)
                        $madeProgress = $true
                        continue
                    }
                    if ($job.Clock.Elapsed.TotalSeconds -ge $job.TimeoutSeconds -or
                        (Test-JobGlobalBudgetExceeded -Job $job)) {
                        foreach ($peer in @($active.Values)) {
                            if (-not $peer.Process.HasExited) {
                                try { $peer.Process.Kill($true) } catch { Write-Warning $_.Exception.Message }
                            }
                        }
                        $active.Remove($cellId)
                        try {
                            Stop-BoundedChild -Job $job -Reason "cell_timeout"
                            Write-CellFailureReceipt -Job $job -Reason "bounded_cell_timeout"
                            Preserve-UnpublishedCellResult -Job $job
                        }
                        finally {
                            $job.Process.Dispose()
                        }
                        throw "Metamorphic cell $cellId exceeded its bounded $($job.TimeoutSeconds)-second slice; completed receipts remain reusable."
                    }
                }
                if (-not $madeProgress) { Start-Sleep -Milliseconds 250 }
            }

            foreach ($job in $finished) {
                try {
                    Complete-Cell -Job $job
                    [void]$Completed.Add([string]$job.CellId)
                }
                finally {
                    $job.Process.Dispose()
                }
            }
        }
        finally {
            foreach ($job in $finished) {
                try {
                    $job.Process.Dispose()
                }
                catch {
                    Write-Warning "Could not dispose completed cell $($job.CellId): $($_.Exception.Message)"
                }
            }
        }
    }
}

function Stop-ActiveCells {
    foreach ($cellId in @($active.Keys)) {
        $job = $active[$cellId]
        try {
            Stop-BoundedChild -Job $job -Reason "wrapper_abort"
            Write-CellFailureReceipt -Job $job -Reason "wrapper_abort"
            Preserve-UnpublishedCellResult -Job $job
        }
        catch {
            Write-Warning "Could not completely preserve aborted cell $cellId`: $($_.Exception.Message)"
        }
        finally {
            $job.Process.Dispose()
            $active.Remove($cellId)
        }
    }
}

try {
    New-Item -ItemType Directory -Path $resolvedWorkRoot, $cellDirectory, $logDirectory, $historyDirectory -Force | Out-Null

    $gitStatus = Invoke-BoundedStage -Stage "git-status" -FileName "git" `
        -Arguments @("-C", $repositoryRoot, "status", "--porcelain=v1", "--untracked-files=all") `
        -TimeoutSeconds 120
    if ($gitStatus.ExitCode -ne 0) {
        throw "Unable to inspect the complete tracked and untracked source state."
    }
    if (-not [string]::IsNullOrWhiteSpace($gitStatus.Stdout)) {
        throw "Metamorphic qualification requires a fully clean commit-bound source tree, including no untracked files."
    }
    $gitHead = Invoke-BoundedStage -Stage "git-head" -FileName "git" `
        -Arguments @("-C", $repositoryRoot, "rev-parse", "HEAD") -TimeoutSeconds 120
    $sourceCommit = $gitHead.Stdout.Trim().ToLowerInvariant()
    if ($gitHead.ExitCode -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
        throw "Unable to resolve the exact source commit."
    }
    if (@(Get-Process -Name "cargo" -ErrorAction SilentlyContinue).Count -gt 0) {
        throw "Another Cargo process is active; the one-physical-target rule permits only one Cargo build/test process."
    }

    $buildArguments = @(
        "build", "--release", "--quiet", "--locked", "-p", "qpls-runner",
        "--example", "multimod_mga_qualification_v1",
        "--example", "multimod_heterogeneity_qualification_v2",
        "--example", "multimod_conditional_qualification_v1",
        "--example", "multimod_causal_qualification_v1"
    )
    Write-Host "[metamorphic] one bounded Cargo build for the four exact producers"
    $build = Invoke-BoundedStage -Stage "cargo-build" -FileName "cargo" `
        -Arguments $buildArguments -TimeoutSeconds ([Math]::Min(1800, (Get-RemainingWrapperWorkSeconds)))
    if ($build.ExitCode -ne 0) {
        throw "The single four-producer Cargo build failed."
    }
    $binarySha256 = [ordered]@{}
    foreach ($family in ($families | Sort-Object Id)) {
        $binary = [string]$binaries[$family.Id]
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "The exact $($family.Id) metamorphic producer is absent after the single build: $binary"
        }
        $binarySha256[$family.Id] = Get-Sha256 -Path $binary
    }
    $buildCommand = @("cargo") + $buildArguments
    $buildReceipt = [ordered]@{
        schema_version = 1
        receipt_id = "qpls.multimod.metamorphic.single-build-receipt.v1"
        status = "passed"
        source_commit = $sourceCommit
        cargo_invocation_count = 1
        command = $buildCommand
        command_sha256 = Get-TextSha256 -Text ($buildCommand | ConvertTo-Json -Compress)
        exit_code = 0
        elapsed_milliseconds = [long]$build.ElapsedMilliseconds
        stdout_path = [string]$build.StdoutPath
        stdout_sha256 = Get-Sha256 -Path $build.StdoutPath
        stdout_size = (Get-Item -LiteralPath $build.StdoutPath).Length
        stderr_path = [string]$build.StderrPath
        stderr_sha256 = Get-Sha256 -Path $build.StderrPath
        stderr_size = (Get-Item -LiteralPath $build.StderrPath).Length
        binary_sha256 = $binarySha256
    }
    Write-JsonAtomic -Path $buildReceiptPath -Value $buildReceipt

    $scientificClock = [System.Diagnostics.Stopwatch]::StartNew()
    $temporaryPlan = Join-Path $resolvedWorkRoot (".cell-plan.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
    try {
        $planGeneration = Invoke-CheckpointTool -Command "plan" -Arguments @("--output", $temporaryPlan)
        if ($planGeneration.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $temporaryPlan -PathType Leaf)) {
            throw "The exact 25-cell metamorphic plan could not be generated."
        }
        if (Test-Path -LiteralPath $planPath -PathType Leaf) {
            if ((Get-Sha256 -Path $planPath) -cne (Get-Sha256 -Path $temporaryPlan)) {
                throw "The retained metamorphic plan differs from the frozen 25-cell contract. Use a reviewed fresh WorkRoot."
            }
            Remove-Item -LiteralPath $temporaryPlan -Force
        }
        else {
            [System.IO.File]::Move($temporaryPlan, $planPath)
        }
    }
    finally {
        if (Test-Path -LiteralPath $temporaryPlan -PathType Leaf) {
            Remove-Item -LiteralPath $temporaryPlan -Force
        }
    }
    $plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    $specs = @($plan.cells)
    if ([string]$plan.plan_id -ne "qpls.multimod.metamorphic.cell-plan.v1" -or
        $specs.Count -ne 25 -or
        [int]$plan.maximum_parallel_cells -ne 4 -or
        [int]$plan.maximum_cell_seconds -ne 1800 -or
        [int]$plan.maximum_scientific_seconds -ne 6480 -or
        [int]$plan.maximum_wrapper_seconds -ne 6600) {
        throw "The generated metamorphic plan does not match the frozen execution limits and inventory."
    }

    $bindingArguments = @()
    foreach ($family in $families) {
        $bindingArguments += @("--binary", ("{0}={1}" -f $family.Id, [string]$binaries[$family.Id]))
    }
    $statusArguments = @(
        "--plan", $planPath,
        "--cell-dir", $cellDirectory,
        "--source-commit", $sourceCommit
    ) + $bindingArguments + @("--output", $statusPath)
    $status = Invoke-CheckpointTool -Command "status" -Arguments $statusArguments
    if ($status.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $statusPath -PathType Leaf)) {
        throw "Existing metamorphic checkpoints could not be validated."
    }
    $checkpointStatus = Get-Content -LiteralPath $statusPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    if ([string]$checkpointStatus.status_id -ne "qpls.multimod.metamorphic.cell-status.v1" -or
        [string]$checkpointStatus.source_commit -ne $sourceCommit) {
        throw "Metamorphic checkpoint status is identity-mismatched."
    }
    foreach ($invalid in @($checkpointStatus.invalid_cells)) {
        Move-StaleCellFiles -CellId ([string]$invalid.cell_id)
    }
    $completed = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($cellId in @($checkpointStatus.valid_cell_ids)) {
        [void]$completed.Add([string]$cellId)
    }
    if ($completed.Count -gt 0) {
        Write-Host ("[metamorphic] reused {0} commit/executable/argument/output-bound cell(s)" -f $completed.Count)
    }

    $baselineSpecs = @($specs | Where-Object { [string]$_.axis -eq "baseline" })
    $dependentSpecs = @($specs | Where-Object { [string]$_.axis -ne "baseline" })
    if ($baselineSpecs.Count -ne 4 -or $dependentSpecs.Count -ne 21 -or
        (@($specs | Select-Object -First 4 | Where-Object { [string]$_.axis -ne "baseline" }).Count -ne 0)) {
        throw "Metamorphic family baselines are not the exact four successful roots before 21 dependent axes."
    }
    Invoke-CellPhase -Name "baseline-root" -Specs $baselineSpecs -SourceCommit $sourceCommit -Completed $completed
    Invoke-CellPhase -Name "dependent-axis" -Specs $dependentSpecs -SourceCommit $sourceCommit -Completed $completed
    if ($completed.Count -ne 25) {
        throw "Metamorphic execution ended without all 25 exact cell receipts."
    }

    # Refresh the persisted checkpoint after the final cell is sealed. The
    # pre-execution status above is intentionally retained for exact resume,
    # while this second pass independently revalidates every receipt before
    # any scientific comparison consumes the cell outputs.
    $completedStatus = Invoke-CheckpointTool -Command "status" -Arguments $statusArguments
    if ($completedStatus.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $statusPath -PathType Leaf)) {
        throw "Completed metamorphic checkpoints could not be revalidated."
    }
    $completedCheckpointStatus = Get-Content -LiteralPath $statusPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
    $expectedCellIds = @($specs | ForEach-Object { [string]$_.cell_id })
    $validCellIds = @($completedCheckpointStatus.valid_cell_ids | ForEach-Object { [string]$_ })
    $validCellIdentityMatches = $validCellIds.Count -eq $expectedCellIds.Count
    if ($validCellIdentityMatches) {
        for ($index = 0; $index -lt $expectedCellIds.Count; $index++) {
            if ($validCellIds[$index] -cne $expectedCellIds[$index]) {
                $validCellIdentityMatches = $false
                break
            }
        }
    }
    if ([string]$completedCheckpointStatus.status_id -ne "qpls.multimod.metamorphic.cell-status.v1" -or
        [string]$completedCheckpointStatus.source_commit -ne $sourceCommit -or
        -not [bool]$completedCheckpointStatus.complete -or
        @($completedCheckpointStatus.invalid_cells).Count -ne 0 -or
        $validCellIds.Count -ne 25 -or
        -not $validCellIdentityMatches) {
        throw "Completed metamorphic checkpoint status is not the exact valid 25-cell ledger."
    }

    $temporaryReport = Join-Path $resolvedWorkRoot (".scientific-report.{0}.tmp.json" -f [Guid]::NewGuid().ToString("N"))
    try {
        $verification = Invoke-BoundedStage -Stage "scoped-scientific-verifier" -FileName "python" -Arguments @(
            $verifier,
            "--input-directory", $cellDirectory,
            "--capability-index", $capabilityIndex,
            "--repository-root", $repositoryRoot,
            "--output", $temporaryReport
        ) -TimeoutSeconds ([Math]::Min($PerCellTimeoutSeconds, (Get-RemainingScientificWorkSeconds))) -ScientificBudget
        if ($verification.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $temporaryReport -PathType Leaf)) {
            throw "The scoped scientific metamorphic verifier rejected the completed-result/preparation matrix."
        }
        $report = Get-Content -LiteralPath $temporaryReport -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 100
        if ([string]$report.report_id -ne "qpls.multimod.global-metamorphic-qualification.v1" -or
            [string]$report.status -ne "passed") {
            throw "The scientific metamorphic report is not a passing exact-contract result."
        }
        $outputParent = Split-Path -Parent $resolvedOutput
        if (-not [string]::IsNullOrWhiteSpace($outputParent)) {
            New-Item -ItemType Directory -Path $outputParent -Force | Out-Null
        }
        [System.IO.File]::Move($temporaryReport, $resolvedOutput, $true)
    }
    finally {
        if (Test-Path -LiteralPath $temporaryReport -PathType Leaf) {
            Remove-Item -LiteralPath $temporaryReport -Force
        }
    }

    $aggregateArguments = @(
        "--plan", $planPath,
        "--cell-dir", $cellDirectory,
        "--source-commit", $sourceCommit
    ) + $bindingArguments + @(
        "--build-receipt", $buildReceiptPath,
        "--report", $resolvedOutput,
        "--output", $executionReceiptPath
    )
    $aggregate = Invoke-CheckpointTool -Command "aggregate" -Arguments $aggregateArguments
    if ($aggregate.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $executionReceiptPath -PathType Leaf)) {
        throw "The final metamorphic execution receipt could not be published."
    }
    Write-Host "[metamorphic] passed 25 exact cells; report and execution receipt are atomically hash-bound"
    exit 0
}
catch {
    Stop-ActiveCells
    Write-Error $_
    exit 1
}
