param(
    [string]$ExportPath = "",
    [switch]$RemintResourceEvidenceOnly
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

if ($RemintResourceEvidenceOnly) {
    $resourcePolicyReminter = Join-Path $PSScriptRoot "process_v2_resource_policy_v3.py"
    $pythonExecutable = if (Test-Path -LiteralPath "C:\Python313\python.exe" -PathType Leaf) {
        "C:\Python313\python.exe"
    } else {
        "python"
    }
    & $pythonExecutable $resourcePolicyReminter --root $repositoryRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Offline PROCESS v2 resource-policy v3 remint failed with exit code $LASTEXITCODE."
    }
    return
}

function Test-FullyQualifiedWindowsPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or
        -not [System.IO.Path]::IsPathRooted($Path)) {
        return $false
    }
    $driveAbsolute = $Path -match '^[A-Za-z]:[\\/]'
    $uncAbsolute = $Path -match '^[\\/]{2}[^\\/]+[\\/]+[^\\/]+(?:[\\/]|$)'
    return $driveAbsolute -or $uncAbsolute
}

$existingProcess = Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue
if ($existingProcess) {
    throw "Close every existing quickpls-desktop.exe instance before packaged PROCESS v2 acceptance."
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
if ([string]::IsNullOrWhiteSpace($ExportPath)) {
    $ExportPath = Join-Path $repositoryRoot "validation\results\v247-native-process-v2-$stamp.xlsx"
}
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
if (-not (Test-FullyQualifiedWindowsPath -Path $ExportPath)) {
    throw "ExportPath must be an absolute .xlsx path under validation\results."
}
$ExportPath = [System.IO.Path]::GetFullPath($ExportPath)
$exportParent = [System.IO.Path]::GetFullPath((Split-Path -Parent $ExportPath))
if (-not $exportParent.Equals($resultsRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "ExportPath must resolve directly inside validation\results."
}
if ([System.IO.Path]::GetExtension($ExportPath) -ne ".xlsx") {
    throw "ExportPath must use the .xlsx extension."
}
if (Test-Path -LiteralPath $ExportPath) {
    throw "ExportPath must not already exist: $ExportPath"
}

$desktopExecutable = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$cleanupReportPath = Join-Path $repositoryRoot "validation\results\v247_process_v2_process_cleanup.json"
$resourceReportPath = Join-Path $repositoryRoot "validation\results\process_v2_resource_report.json"
$resourceSamplesEvidencePath = Join-Path $repositoryRoot "validation\results\process_v2_resource_samples.jsonl"
$resourcePhasesEvidencePath = Join-Path $repositoryRoot "validation\results\process_v2_resource_phases.json"
$resourceSamplesPath = Join-Path $repositoryRoot "validation\results\process-v2-resource-samples-$stamp.jsonl"
$resourcePhasesPath = Join-Path $repositoryRoot "validation\results\process-v2-resource-phases-$stamp.json"
$resourceStopPath = Join-Path $repositoryRoot "validation\results\process-v2-resource-stop-$stamp.signal"
$resourceMonitorStdoutPath = Join-Path $repositoryRoot "validation\results\process-v2-resource-monitor-$stamp.stdout.txt"
$resourceMonitorStderrPath = Join-Path $repositoryRoot "validation\results\process-v2-resource-monitor-$stamp.stderr.txt"
$resourceMonitorScript = Join-Path $repositoryRoot "validation\monitor_quickpls_process_tree.ps1"
$packagedReportPath = Join-Path $repositoryRoot "validation\results\process_v2_packaged_acceptance.json"
$acceptanceEnvironmentNames = @(
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
    "QUICKPLS_CDP_ENDPOINT",
    "QUICKPLS_CLI_PATH",
    "QUICKPLS_PYTHON",
    "QUICKPLS_ACCEPTANCE_SCOPE",
    "QUICKPLS_PROCESS_V2_EXPORT_PATH",
    "QUICKPLS_DESKTOP_EXE_PATH",
    "QUICKPLS_PROCESS_V2_RESOURCE_PHASES_PATH"
)
$priorAcceptanceEnvironment = @{}
foreach ($name in $acceptanceEnvironmentNames) {
    $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    $priorAcceptanceEnvironment[$name] = [ordered]@{
        present = $null -ne $item
        value = if ($null -ne $item) { [string]$item.Value } else { $null }
    }
}
$application = $null
$resourceMonitor = $null
$resourceMonitorHandle = $null
$resourceMonitorFirstSample = $null

function Get-ExactDescendantProcesses {
    param([int]$RootProcessId)
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId, ParentProcessId, CreationDate, Name, ExecutablePath)
    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($RootProcessId)
    $descendants = [System.Collections.Generic.HashSet[int]]::new()
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($descendants.Add($childId)) {
                $pending.Enqueue($childId)
            }
        }
    }
    return @($rows | Where-Object { $descendants.Contains([int]$_.ProcessId) } | Sort-Object ProcessId)
}

function Get-LiveExactProcessIds {
    param([object[]]$Processes)
    $live = foreach ($descriptor in $Processes) {
        $processId = [int]$descriptor.ProcessId
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
        if ($current -and $current.CreationDate -eq $descriptor.CreationDate -and $current.Name -eq $descriptor.Name) {
            $processId
        }
    }
    return @($live)
}

try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
    $env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
    $env:QUICKPLS_CLI_PATH = Join-Path $repositoryRoot "target\release\qpls.exe"
    $env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
    $env:QUICKPLS_ACCEPTANCE_SCOPE = "process_v2"
    $env:QUICKPLS_PROCESS_V2_EXPORT_PATH = $ExportPath
    $env:QUICKPLS_DESKTOP_EXE_PATH = $desktopExecutable
    $env:QUICKPLS_PROCESS_V2_RESOURCE_PHASES_PATH = $resourcePhasesPath

    $application = Start-Process `
        -FilePath $desktopExecutable `
        -WorkingDirectory $repositoryRoot `
        -WindowStyle Normal `
        -PassThru

    $resourceMonitorArguments = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", ('"{0}"' -f $resourceMonitorScript),
        "-RootProcessId", [string]$application.Id,
        "-SamplesPath", ('"{0}"' -f $resourceSamplesPath),
        "-StopSignalPath", ('"{0}"' -f $resourceStopPath),
        "-IntervalMilliseconds", "250"
    )
    $resourceMonitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList $resourceMonitorArguments `
        -WorkingDirectory $repositoryRoot `
        -WindowStyle Hidden `
        -RedirectStandardOutput $resourceMonitorStdoutPath `
        -RedirectStandardError $resourceMonitorStderrPath `
        -PassThru
    # Windows PowerShell 5.1 can lose access to ExitCode when Start-Process's
    # lazily opened process handle is first requested only after process exit.
    # Pin the live handle immediately and keep the Process object alive through
    # the bounded wait and redirected-stream join below.
    $resourceMonitorHandle = $resourceMonitor.Handle
    if ($resourceMonitorHandle -eq [System.IntPtr]::Zero) {
        throw "QuickPLS resource monitor did not expose a live process handle."
    }

    $resourceMonitorReady = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($resourceMonitor.HasExited) { break }
        if (Test-Path -LiteralPath $resourceSamplesPath) {
            $firstLine = Get-Content -LiteralPath $resourceSamplesPath -Encoding UTF8 | Select-Object -First 1
            if (-not [string]::IsNullOrWhiteSpace($firstLine)) {
                try {
                    $candidateSample = $firstLine | ConvertFrom-Json
                    if ($candidateSample.root_present -eq $true -and
                        [int]$candidateSample.root_pid -eq $application.Id -and
                        [long]$candidateSample.total_working_set_bytes -gt 0 -and
                        [long]$candidateSample.total_private_memory_bytes -gt 0 -and
                        [int]$candidateSample.total_handle_count -gt 0 -and
                        [int]$candidateSample.total_thread_count -gt 0 -and
                        -not $resourceMonitor.HasExited) {
                        $resourceMonitorFirstSample = $candidateSample
                        $resourceMonitorReady = $true
                        break
                    }
                } catch {
                    # The append may be between bytes; retry until a complete JSON line exists.
                }
            }
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $resourceMonitorReady) {
        $monitorExit = if ($resourceMonitor.HasExited) { [string]$resourceMonitor.ExitCode } else { "still-running" }
        $monitorError = if (Test-Path -LiteralPath $resourceMonitorStderrPath) {
            [string]$monitorStderrText = Get-Content -LiteralPath $resourceMonitorStderrPath -Raw -Encoding UTF8
            if ($null -eq $monitorStderrText) { "" } else { $monitorStderrText.Trim() }
        } else { "no stderr captured" }
        throw "QuickPLS resource monitor did not produce its first sample (exit $monitorExit): $monitorError"
    }

    $cdpReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Uri $env:QUICKPLS_CDP_ENDPOINT/json/version -TimeoutSec 1
            $cdpReady = $true
            break
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }
    if (-not $cdpReady) {
        throw "QuickPLS WebView2 CDP did not open on port 9222."
    }

    node .\validation\v247_tauri_native_acceptance.mjs
    if ($LASTEXITCODE -ne 0) {
        throw "Focused packaged PROCESS v2 acceptance failed with exit code $LASTEXITCODE."
    }

    Get-Item -LiteralPath $ExportPath | Select-Object FullName, Length, LastWriteTime
    Get-Item -LiteralPath (Join-Path $repositoryRoot "validation\results\process_v2_packaged_acceptance.json") |
        Select-Object FullName, Length, LastWriteTime
} finally {
    try {
    $monitorPidAbsent = $false
    $monitorExitConfirmed = $false
    $monitorExitCodeUnavailable = $false
    $cleanup = [ordered]@{
        generated_at_utc = $null
        launched_pid = if ($application) { $application.Id } else { $null }
        descendants_at_shutdown = @()
        graceful_close_exit_code = $null
        graceful_exit_confirmed = $false
        forced_parent_termination = $false
        forced_descendant_pids = @()
        parent_exit_confirmed = $false
        lingering_descendant_pids = @()
        resource_monitor_pid = if ($resourceMonitor) { $resourceMonitor.Id } else { $null }
        resource_monitor_exit_confirmed = $false
        resource_monitor_exit_code = $null
        resource_monitor_stderr = $null
        resource_monitor_first_sample = $resourceMonitorFirstSample
        resource_monitor_terminal_reason = $null
        forced_resource_monitor_termination = $false
        passed = $false
    }
    if ($application) {
        $cleanup.descendants_at_shutdown = @(Get-ExactDescendantProcesses -RootProcessId $application.Id)
    }
    try {
    try {
        [System.IO.File]::WriteAllText($resourceStopPath, "stop", [System.Text.UTF8Encoding]::new($false))
        $monitorProcess = $resourceMonitor
        if ($null -ne $monitorProcess) {
            $monitorExitConfirmed = $monitorProcess.WaitForExit(5000)
            if (-not $monitorExitConfirmed -and -not $monitorProcess.HasExited) {
                Stop-Process -Id $monitorProcess.Id -Force -ErrorAction SilentlyContinue
                $cleanup.forced_resource_monitor_termination = $true
                $monitorExitConfirmed = $monitorProcess.WaitForExit(3000)
            }
            $cleanup.resource_monitor_exit_confirmed = [bool]$monitorExitConfirmed
            if ($monitorExitConfirmed) {
                # Complete redirected-stream processing before reading process
                # metadata. The timed wait is the exit authority; PID absence is
                # checked separately and never substitutes for this result.
                $monitorProcess.WaitForExit()
                $monitorProcess.Refresh()
                try {
                    $capturedMonitorExitCode = $monitorProcess.ExitCode
                    if ($null -eq $capturedMonitorExitCode) {
                        $monitorExitCodeUnavailable = $true
                    } else {
                        $cleanup.resource_monitor_exit_code = [int]$capturedMonitorExitCode
                    }
                } catch {
                    $monitorExitCodeUnavailable = $true
                }
            }
            $monitorPidAbsent = -not [bool](Get-Process -Id $monitorProcess.Id -ErrorAction SilentlyContinue)
            if (Test-Path -LiteralPath $resourceMonitorStderrPath) {
                [string]$monitorStderrText = Get-Content `
                    -LiteralPath $resourceMonitorStderrPath `
                    -Raw `
                    -Encoding UTF8
                $cleanup.resource_monitor_stderr = if ($null -eq $monitorStderrText) {
                    ""
                } else {
                    $monitorStderrText.Trim()
                }
            }
            $cleanup.resource_monitor_terminal_reason = if (-not $cleanup.resource_monitor_exit_confirmed) {
                "monitor_exit_unconfirmed"
            } elseif ($monitorExitCodeUnavailable -or $null -eq $cleanup.resource_monitor_exit_code) {
                "exit_code_unavailable"
            } elseif ($cleanup.resource_monitor_exit_code -ne 0) {
                "monitor_error"
            } elseif (-not $resourceMonitorFirstSample) {
                "no_valid_first_sample"
            } elseif ($cleanup.forced_resource_monitor_termination) {
                "forced_termination"
            } elseif (-not $monitorPidAbsent) {
                "monitor_pid_still_present"
            } else {
                "stop_signal"
            }
        }
    } catch {
        $monitorCleanupError = $_.Exception.Message
        $monitorProcess = $resourceMonitor
        if ($null -ne $monitorProcess -and -not $monitorProcess.HasExited) {
            Stop-Process -Id $monitorProcess.Id -Force -ErrorAction SilentlyContinue
            $cleanup.forced_resource_monitor_termination = $true
            $monitorExitConfirmed = $monitorProcess.WaitForExit(3000)
        }
        if ($null -ne $monitorProcess) {
            if (-not $monitorExitConfirmed) {
                $monitorExitConfirmed = $monitorProcess.WaitForExit(3000)
            }
            $cleanup.resource_monitor_exit_confirmed = [bool]$monitorExitConfirmed
            if ($monitorExitConfirmed) {
                $monitorProcess.WaitForExit()
                $monitorProcess.Refresh()
                try {
                    $capturedMonitorExitCode = $monitorProcess.ExitCode
                    if ($null -eq $capturedMonitorExitCode) {
                        $monitorExitCodeUnavailable = $true
                    } else {
                        $cleanup.resource_monitor_exit_code = [int]$capturedMonitorExitCode
                    }
                } catch {
                    $monitorExitCodeUnavailable = $true
                }
            }
            $monitorPidAbsent = -not [bool](Get-Process -Id $monitorProcess.Id -ErrorAction SilentlyContinue)
        }
        $cleanup.resource_monitor_stderr = "monitor_cleanup_error: $monitorCleanupError"
        $cleanup.resource_monitor_terminal_reason = if ($monitorExitCodeUnavailable) {
            "exit_code_unavailable"
        } else {
            "monitor_error"
        }
    }
    } finally {
    if ($application -and -not $application.HasExited) {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "SilentlyContinue"
        & node .\validation\close_tauri_test_window.mjs 2>$null | Out-Null
        $cleanup.graceful_close_exit_code = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorActionPreference
        $cleanup.graceful_exit_confirmed = $application.WaitForExit(10000)
        if (-not $cleanup.graceful_exit_confirmed -and -not $application.HasExited) {
            Stop-Process -Id $application.Id -Force -ErrorAction SilentlyContinue
            $cleanup.forced_parent_termination = $true
            $null = $application.WaitForExit(5000)
        }
    }
    if ($application) {
        $cleanup.parent_exit_confirmed = -not [bool](Get-Process -Id $application.Id -ErrorAction SilentlyContinue)
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        $liveDescendants = @(Get-LiveExactProcessIds -Processes $cleanup.descendants_at_shutdown)
        while ($liveDescendants.Count -gt 0 -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 250
            $liveDescendants = @(Get-LiveExactProcessIds -Processes $cleanup.descendants_at_shutdown)
        }
        if ($liveDescendants.Count -gt 0) {
            foreach ($childId in $liveDescendants) {
                Stop-Process -Id $childId -Force -ErrorAction SilentlyContinue
            }
            $cleanup.forced_descendant_pids = @($liveDescendants)
            Start-Sleep -Milliseconds 500
        }
        $cleanup.lingering_descendant_pids = @(Get-LiveExactProcessIds -Processes $cleanup.descendants_at_shutdown)
    }
    }
    $cleanup.passed = $cleanup.graceful_close_exit_code -eq 0 -and
        $cleanup.graceful_exit_confirmed -and
        -not $cleanup.forced_parent_termination -and
        $cleanup.forced_descendant_pids.Count -eq 0 -and
        -not $cleanup.forced_resource_monitor_termination -and
        $monitorPidAbsent -and
        $cleanup.resource_monitor_exit_code -eq 0 -and
        [string]::IsNullOrWhiteSpace([string]$cleanup.resource_monitor_stderr) -and
        $cleanup.resource_monitor_terminal_reason -eq "stop_signal" -and
        $cleanup.parent_exit_confirmed -and
        $cleanup.lingering_descendant_pids.Count -eq 0 -and
        $cleanup.resource_monitor_exit_confirmed

    $samples = @()
    if (Test-Path -LiteralPath $resourceSamplesPath) {
        $samples = @(Get-Content -LiteralPath $resourceSamplesPath -Encoding UTF8 |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json })
    }
    $phaseDocument = if (Test-Path -LiteralPath $resourcePhasesPath) {
        Get-Content -LiteralPath $resourcePhasesPath -Raw -Encoding UTF8 | ConvertFrom-Json
    } else {
        $null
    }
    $launchedProcessId = if ($application) { [int]$application.Id } else { 0 }
    $liveSamples = @($samples | Where-Object {
        $_.root_present -eq $true -and $_.root_pid -eq $launchedProcessId -and
        $_.total_working_set_bytes -is [ValueType] -and [long]$_.total_working_set_bytes -gt 0 -and
        $_.total_private_memory_bytes -is [ValueType] -and [long]$_.total_private_memory_bytes -gt 0 -and
        $_.total_handle_count -is [ValueType] -and [int]$_.total_handle_count -gt 0 -and
        $_.total_thread_count -is [ValueType] -and [int]$_.total_thread_count -gt 0
    })

    function Get-MedianLong {
        param([long[]]$Values)
        if (-not $Values -or $Values.Count -eq 0) { return 0L }
        $ordered = @($Values | Sort-Object)
        $middle = [int][Math]::Floor($ordered.Count / 2)
        if ($ordered.Count % 2 -eq 1) { return [long]$ordered[$middle] }
        return [long][Math]::Floor(([double]$ordered[$middle - 1] + [double]$ordered[$middle]) / 2.0)
    }

    function Get-P95Long {
        param([long[]]$Values)
        if (-not $Values -or $Values.Count -eq 0) { return 0L }
        $ordered = @($Values | Sort-Object)
        $index = [Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.95) - 1)
        return [long]$ordered[$index]
    }

    $resourceRoleNames = @(
        "desktop_root", "webview_browser", "webview_renderer", "webview_gpu",
        "webview_utility", "webview_other", "other_descendant"
    )
    function Get-CanonicalRoleCounts {
        param([object]$Sample)
        $counts = [ordered]@{}
        foreach ($role in $resourceRoleNames) {
            $property = $Sample.process_role_counts.PSObject.Properties[$role]
            $counts[$role] = if ($property) { [int]$property.Value } else { 0 }
        }
        return $counts
    }
    if ($resourceMonitorFirstSample) {
        $resourceMonitorFirstSample.process_role_counts = [pscustomobject](
            Get-CanonicalRoleCounts -Sample $resourceMonitorFirstSample
        )
        $cleanup.resource_monitor_first_sample = $resourceMonitorFirstSample
    }

    function Get-ResourceProcessIdentityKey {
        param([object]$Process)
        return "{0}|{1}|{2}|{3}|{4}" -f @(
            [int]$Process.pid,
            [int]$Process.parent_pid,
            ([string]$Process.name).ToLowerInvariant(),
            [string]$Process.role,
            [string]$Process.creation_date
        )
    }

    function Get-ResourceProcessIdentity {
        param([object]$Process)
        return [ordered]@{
            pid = [int]$Process.pid
            parent_pid = [int]$Process.parent_pid
            name = ([string]$Process.name).ToLowerInvariant()
            role = [string]$Process.role
            creation_date = [string]$Process.creation_date
        }
    }

    function Test-ResourceProcessDescendantOf {
        param([object]$Process, [object[]]$Processes, [int]$AncestorPid)
        $parentByPid = @{}
        foreach ($candidate in $Processes) {
            $parentByPid[[int]$candidate.pid] = [int]$candidate.parent_pid
        }
        $visited = [System.Collections.Generic.HashSet[int]]::new()
        $parentId = [int]$Process.parent_pid
        while ($parentId -gt 0 -and $visited.Add($parentId)) {
            if ($parentId -eq $AncestorPid) { return $true }
            if (-not $parentByPid.ContainsKey($parentId)) { return $false }
            $parentId = [int]$parentByPid[$parentId]
        }
        return $false
    }

    function Get-BoundedResourceRoleWindow {
        param([object[]]$Samples)
        $signatures = @()
        foreach ($sample in $Samples) {
            $keys = @($sample.processes | Sort-Object pid | ForEach-Object {
                Get-ResourceProcessIdentityKey -Process $_
            })
            $signatures += ConvertTo-Json -InputObject ([object[]]$keys) -Compress
        }
        $groups = @($signatures | Group-Object | Sort-Object -Property `
            @{ Expression = { $_.Count }; Descending = $true },
            @{ Expression = { $_.Name }; Descending = $false })
        $modalSignature = if ($groups.Count -gt 0) { [string]$groups[0].Name } else { $null }
        $modalSampleCount = if ($groups.Count -gt 0) { [int]$groups[0].Count } else { 0 }
        $uniqueModal = $groups.Count -eq 1 -or ($groups.Count -gt 1 -and $groups[0].Count -gt $groups[1].Count)
        $modalIndex = if ($modalSignature) { [Array]::IndexOf([object[]]$signatures, $modalSignature) } else { -1 }
        $modalSample = if ($modalIndex -ge 0) { $Samples[$modalIndex] } else { $null }
        $modalProcesses = if ($modalSample) { @($modalSample.processes | Sort-Object pid) } else { @() }
        $modalIdentities = @($modalProcesses | ForEach-Object { Get-ResourceProcessIdentity -Process $_ })
        $modalKeys = @($modalProcesses | ForEach-Object { Get-ResourceProcessIdentityKey -Process $_ })
        $modalKeySet = [System.Collections.Generic.HashSet[string]]::new([string[]]$modalKeys)
        $modalRoleCounts = if ($modalSample) { Get-CanonicalRoleCounts -Sample $modalSample } else { $null }
        $deviationIndices = @(for ($index = 0; $index -lt $signatures.Count; $index++) {
            if ($signatures[$index] -ne $modalSignature) { $index }
        })
        $longestDeviationStreak = 0
        $currentDeviationStreak = 0
        for ($index = 0; $index -lt $signatures.Count; $index++) {
            if ($signatures[$index] -ne $modalSignature) {
                $currentDeviationStreak++
                $longestDeviationStreak = [Math]::Max($longestDeviationStreak, $currentDeviationStreak)
            } else {
                $currentDeviationStreak = 0
            }
        }
        $firstThreeExactModal = $signatures.Count -ge 6 -and
            @($signatures | Select-Object -First 3 | Where-Object { $_ -ne $modalSignature }).Count -eq 0
        $lastThreeExactModal = $signatures.Count -ge 6 -and
            @($signatures | Select-Object -Last 3 | Where-Object { $_ -ne $modalSignature }).Count -eq 0
        $modalRoots = @($modalProcesses | Where-Object { $_.role -eq "desktop_root" })
        $modalBrowsers = @($modalProcesses | Where-Object { $_.role -eq "webview_browser" })
        $rootKey = if ($modalRoots.Count -eq 1) { Get-ResourceProcessIdentityKey -Process $modalRoots[0] } else { $null }
        $persistentBrowserPid = if ($modalBrowsers.Count -eq 1) { [int]$modalBrowsers[0].pid } else { 0 }
        $rootIdentityEverySample = $modalRoots.Count -eq 1
        $otherDescendantZeroEverySample = $true
        $reportedRoleCountsMatchProcessesEverySample = $true
        $baselineNeverRemovedOrReplaced = $modalKeys.Count -gt 0
        $transientProcessesAllowed = $true
        $transientProcesses = @()
        $transientIdentityKeys = [System.Collections.Generic.HashSet[string]]::new()
        for ($index = 0; $index -lt $Samples.Count; $index++) {
            $sample = $Samples[$index]
            $sampleProcesses = @($sample.processes)
            $sampleKeys = @($sampleProcesses | ForEach-Object { Get-ResourceProcessIdentityKey -Process $_ })
            $sampleKeySet = [System.Collections.Generic.HashSet[string]]::new([string[]]$sampleKeys)
            $sampleRoots = @($sampleProcesses | Where-Object { $_.role -eq "desktop_root" })
            $modalRootPid = if ($modalRoots.Count -eq 1) { [int]$modalRoots[0].pid } else { -1 }
            if ($sample.root_present -ne $true -or [int]$sample.root_pid -ne $modalRootPid -or
                $sampleRoots.Count -ne 1 -or
                (Get-ResourceProcessIdentityKey -Process $sampleRoots[0]) -ne $rootKey) {
                $rootIdentityEverySample = $false
            }
            if (@($sampleProcesses | Where-Object { $_.role -eq "other_descendant" }).Count -ne 0) {
                $otherDescendantZeroEverySample = $false
            }
            $reportedRoleCounts = Get-CanonicalRoleCounts -Sample $sample
            $computedRoleCounts = [ordered]@{}
            foreach ($role in $resourceRoleNames) {
                $computedRoleCounts[$role] = @($sampleProcesses | Where-Object { $_.role -eq $role }).Count
            }
            if (($reportedRoleCounts | ConvertTo-Json -Compress) -ne
                ($computedRoleCounts | ConvertTo-Json -Compress)) {
                $reportedRoleCountsMatchProcessesEverySample = $false
            }
            foreach ($modalKey in $modalKeys) {
                if (-not $sampleKeySet.Contains($modalKey)) { $baselineNeverRemovedOrReplaced = $false }
            }
            $extras = @($sampleProcesses | Where-Object {
                -not $modalKeySet.Contains((Get-ResourceProcessIdentityKey -Process $_))
            })
            foreach ($extra in $extras) {
                $extraKey = Get-ResourceProcessIdentityKey -Process $extra
                $null = $transientIdentityKeys.Add($extraKey)
                $descendantOfPersistentBrowser = $persistentBrowserPid -gt 0 -and
                    (Test-ResourceProcessDescendantOf -Process $extra -Processes $sampleProcesses -AncestorPid $persistentBrowserPid)
                $allowed = ([string]$extra.name).ToLowerInvariant() -eq "msedgewebview2.exe" -and
                    @("webview_gpu", "webview_utility", "webview_other") -contains [string]$extra.role -and
                    $descendantOfPersistentBrowser
                if (-not $allowed) { $transientProcessesAllowed = $false }
                $transientProcesses += [ordered]@{
                    sample_index = $index
                    recorded_at_utc = (ConvertTo-UtcDateTime $sample.recorded_at_utc).ToString("o")
                    pid = [int]$extra.pid
                    parent_pid = [int]$extra.parent_pid
                    name = ([string]$extra.name).ToLowerInvariant()
                    role = [string]$extra.role
                    creation_date = [string]$extra.creation_date
                    working_set_bytes = [long]$extra.working_set_bytes
                    private_memory_bytes = [long]$extra.private_memory_bytes
                    handle_count = [int]$extra.handle_count
                    thread_count = [int]$extra.thread_count
                    descendant_of_persistent_browser = [bool]$descendantOfPersistentBrowser
                }
            }
        }
        $maximumDeviatingSamples = [int][Math]::Floor($Samples.Count * 0.20)
        $transientsAbsentTerminalThree = $lastThreeExactModal -and @($transientProcesses | Where-Object {
            [int]$_.sample_index -ge $Samples.Count - 3
        }).Count -eq 0
        $passed = $Samples.Count -ge 6 -and $uniqueModal -and $modalSampleCount -ge 6 -and
            $modalSampleCount * 100 -ge $Samples.Count * 80 -and
            $lastThreeExactModal -and
            $deviationIndices.Count -le $maximumDeviatingSamples -and $longestDeviationStreak -le 2 -and
            $rootIdentityEverySample -and $otherDescendantZeroEverySample -and
            $reportedRoleCountsMatchProcessesEverySample -and
            $baselineNeverRemovedOrReplaced -and $transientProcessesAllowed -and
            $transientsAbsentTerminalThree
        return [ordered]@{
            policy = "modal_pid_role_identity_with_bounded_webview_churn_v1"
            passed = $passed
            sample_count = $Samples.Count
            modal_sample_count = $modalSampleCount
            minimum_modal_sample_count = 6
            minimum_modal_percent = 80
            unique_modal_identity = $uniqueModal
            modal_pid_role_identities = $modalIdentities
            modal_role_counts = $modalRoleCounts
            first_three_exact_modal = $firstThreeExactModal
            last_three_exact_modal = $lastThreeExactModal
            deviating_sample_indices = $deviationIndices
            deviating_sample_count = $deviationIndices.Count
            maximum_deviating_sample_count = $maximumDeviatingSamples
            maximum_deviating_percent = 20
            longest_deviation_streak = $longestDeviationStreak
            maximum_deviation_streak = 2
            root_identity_every_sample = $rootIdentityEverySample
            other_descendant_zero_every_sample = $otherDescendantZeroEverySample
            reported_role_counts_match_processes_every_sample = $reportedRoleCountsMatchProcessesEverySample
            baseline_identities_never_removed_or_replaced = $baselineNeverRemovedOrReplaced
            persistent_browser_pid = if ($persistentBrowserPid -gt 0) { $persistentBrowserPid } else { $null }
            transient_identity_count = $transientIdentityKeys.Count
            allowed_transient_roles = @("webview_gpu", "webview_utility", "webview_other")
            transient_processes = $transientProcesses
            transient_processes_allowed = $transientProcessesAllowed
            transients_absent_terminal_three = $transientsAbsentTerminalThree
        }
    }

    $resourceIdleSettleMilliseconds = 5000
    $resourceCaptureDelayMilliseconds = 500
    $resourceSampleWindowMilliseconds = 10000
    $resourceMinimumSamplesPerCheckpoint = 6

    function ConvertTo-UtcDateTime {
        param([object]$Value)
        if ($Value -is [DateTime]) { return ([DateTime]$Value).ToUniversalTime() }
        if ($Value -is [DateTimeOffset]) { return ([DateTimeOffset]$Value).UtcDateTime }
        return [DateTimeOffset]::Parse(
            [string]$Value,
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::RoundtripKind
        ).UtcDateTime
    }

    function Select-ResourceSamplesAtPhase {
        param([object[]]$Rows, [object]$Phase, [object]$NextPhase = $null)
        if (-not $Phase -or -not $Phase.recorded_at_utc -or
            [int]$Phase.capture_delay_milliseconds -ne $resourceCaptureDelayMilliseconds -or
            [int]$Phase.sample_window_milliseconds -ne $resourceSampleWindowMilliseconds) { return @() }
        $phaseTime = ConvertTo-UtcDateTime $Phase.recorded_at_utc
        $windowStart = $phaseTime.AddMilliseconds($resourceCaptureDelayMilliseconds)
        $windowEnd = $windowStart.AddMilliseconds($resourceSampleWindowMilliseconds)
        $nextPhaseTime = if ($NextPhase -and $NextPhase.recorded_at_utc) {
            ConvertTo-UtcDateTime $NextPhase.recorded_at_utc
        } else { $null }
        return @($Rows | Where-Object {
            $sampleTime = ConvertTo-UtcDateTime $_.recorded_at_utc
            $sampleTime -ge $windowStart -and $sampleTime -lt $windowEnd -and
                (-not $nextPhaseTime -or $sampleTime -lt $nextPhaseTime)
        } | Sort-Object { ConvertTo-UtcDateTime $_.recorded_at_utc })
    }

    $initialPhase = if ($phaseDocument) { $phaseDocument.phases.initial_idle } else { $null }
    $cancelledPhase = if ($phaseDocument) { $phaseDocument.phases.post_cancellation_idle } else { $null }
    $completedCycle1Phase = if ($phaseDocument) { $phaseDocument.phases.post_completed_cycle_1_idle } else { $null }
    $history2Phase = if ($phaseDocument) { $phaseDocument.phases.post_completed_history_2_idle } else { $null }
    $completedCycle2Phase = if ($phaseDocument) { $phaseDocument.phases.post_completed_cycle_2_idle } else { $null }
    $checkpointRows = @(
        [ordered]@{
            name = "initial_idle"; phase = $initialPhase; next = $cancelledPhase
            state_kind = "model_free_fixture"; surface = "data"; completed = 0; witnesses = 0; selected = $null
        },
        [ordered]@{
            name = "post_cancellation_idle"; phase = $cancelledPhase; next = $completedCycle1Phase
            state_kind = "cancelled_setup_no_result"; surface = "data"; completed = 0; witnesses = 0; selected = $null
        },
        [ordered]@{
            name = "post_completed_cycle_1_idle"; phase = $completedCycle1Phase; next = $history2Phase
            state_kind = "one_result_reopened_original"; surface = "results"; completed = 1; witnesses = 1; selected = "dynamic"
        },
        [ordered]@{
            name = "post_completed_history_2_idle"; phase = $history2Phase; next = $completedCycle2Phase
            state_kind = "two_results_retained_history"; surface = "results"; completed = 2; witnesses = 2; selected = "dynamic"
        },
        [ordered]@{
            name = "post_completed_cycle_2_idle"; phase = $completedCycle2Phase; next = $null
            state_kind = "one_result_reopened_reset_clone"; surface = "results"; completed = 1; witnesses = 1; selected = "dynamic"
        }
    )
    $phaseSnapshotArtifacts = @()
    $phaseSnapshotContract = $true
    foreach ($checkpoint in $checkpointRows) {
        $phase = $checkpoint.phase
        $effectiveArchive = if ($phase) { $phase.effective_archive } else { $null }
        $snapshotDescriptor = [ordered]@{
            path = if ($effectiveArchive) { [string]$effectiveArchive.path } else { $null }
            size = if ($effectiveArchive) { [long]$effectiveArchive.bytes } else { 0L }
            sha256 = if ($effectiveArchive) { [string]$effectiveArchive.sha256 } else { $null }
        }
        $phaseSnapshotArtifacts += $snapshotDescriptor
        $snapshotAbsolutePath = if ($snapshotDescriptor.path) {
            Join-Path $repositoryRoot ([string]$snapshotDescriptor.path).Replace('/', '\')
        } else { $null }
        $snapshotCurrentExact = $snapshotAbsolutePath -and
            (Test-Path -LiteralPath $snapshotAbsolutePath -PathType Leaf)
        if ($snapshotCurrentExact) {
            $snapshotCurrentExact = (Get-Item -LiteralPath $snapshotAbsolutePath).Length -eq $snapshotDescriptor.size -and
                (Get-FileHash -LiteralPath $snapshotAbsolutePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq
                    $snapshotDescriptor.sha256
        }
        if (-not $effectiveArchive -or
            $snapshotDescriptor.path -notmatch '^validation/results/process-v2-resource-snapshot-[0-9]+-[0-9]+-[a-z0-9_]+\.qpls$' -or
            $snapshotDescriptor.size -le 0 -or $snapshotDescriptor.sha256 -notmatch '^[0-9a-f]{64}$' -or
            -not $snapshotCurrentExact) {
            $phaseSnapshotContract = $false
        }
    }
    $phaseSnapshotContract = $phaseSnapshotContract -and
        $phaseSnapshotArtifacts.Count -eq 5 -and
        @($phaseSnapshotArtifacts.path | Select-Object -Unique).Count -eq 5
    $checkpointEvidence = @()
    $checkpointDiagnostics = @()
    $checkpointSamplesByName = @{}
    $checkpointContract = $true
    $previousPhaseTime = $null
    foreach ($checkpoint in $checkpointRows) {
        $phase = $checkpoint.phase
        $windowSamples = @(Select-ResourceSamplesAtPhase -Rows $liveSamples -Phase $phase -NextPhase $checkpoint.next)
        $checkpointSamplesByName[[string]$checkpoint.name] = $windowSamples
        $failureReasons = [System.Collections.Generic.List[string]]::new()
        if (-not $phase) { $failureReasons.Add("missing_phase") }
        if ($phase -and [int]$phase.idle_settle_milliseconds -ne $resourceIdleSettleMilliseconds) {
            $failureReasons.Add("idle_settle_milliseconds_mismatch")
        }
        if ($phase -and [int]$phase.capture_delay_milliseconds -ne $resourceCaptureDelayMilliseconds) {
            $failureReasons.Add("capture_delay_milliseconds_mismatch")
        }
        if ($phase -and [int]$phase.sample_window_milliseconds -ne $resourceSampleWindowMilliseconds) {
            $failureReasons.Add("sample_window_milliseconds_mismatch")
        }
        if ($windowSamples.Count -lt $resourceMinimumSamplesPerCheckpoint) {
            $failureReasons.Add("insufficient_window_samples")
        }
        $phaseTime = if ($phase -and $phase.recorded_at_utc) {
            ConvertTo-UtcDateTime $phase.recorded_at_utc
        } else { $null }
        $windowStart = if ($phaseTime) {
            $phaseTime.AddMilliseconds($resourceCaptureDelayMilliseconds)
        } else { $null }
        $windowEnd = if ($windowStart) {
            $windowStart.AddMilliseconds($resourceSampleWindowMilliseconds)
        } else { $null }
        $sampleTimes = @($windowSamples | ForEach-Object {
            ConvertTo-UtcDateTime $_.recorded_at_utc
        })
        $checkpointDiagnostic = [ordered]@{
            name = $checkpoint.name
            passed = $false
            phase_present = [bool]$phase
            phase_recorded_at_utc = if ($phaseTime) { $phaseTime.ToString("o") } else { $null }
            window_start_utc = if ($windowStart) { $windowStart.ToString("o") } else { $null }
            window_end_utc = if ($windowEnd) { $windowEnd.ToString("o") } else { $null }
            eligible_sample_recorded_at_utc = @($sampleTimes | ForEach-Object { $_.ToString("o") })
            eligible_sample_count = $windowSamples.Count
            expected_idle_settle_milliseconds = $resourceIdleSettleMilliseconds
            actual_idle_settle_milliseconds = if ($phase) { [int]$phase.idle_settle_milliseconds } else { $null }
            expected_capture_delay_milliseconds = $resourceCaptureDelayMilliseconds
            actual_capture_delay_milliseconds = if ($phase) { [int]$phase.capture_delay_milliseconds } else { $null }
            expected_sample_window_milliseconds = $resourceSampleWindowMilliseconds
            actual_sample_window_milliseconds = if ($phase) { [int]$phase.sample_window_milliseconds } else { $null }
            minimum_samples = $resourceMinimumSamplesPerCheckpoint
            failure_reasons = @($failureReasons)
        }
        if ($failureReasons.Count -gt 0) {
            $checkpointContract = $false
            $checkpointDiagnostics += $checkpointDiagnostic
            continue
        }
        $logical = $phase.logical_state
        $effectiveLogical = $phase.effective_archive.logical_state
        $sourceBefore = $phase.effective_archive.source_before
        $sourceAfter = $phase.effective_archive.source_after
        $roleWindow = Get-BoundedResourceRoleWindow -Samples $windowSamples
        $selectedExact = if ($checkpoint.selected -eq "dynamic") {
            -not [string]::IsNullOrWhiteSpace([string]$logical.selected_run_id)
        } else { $null -eq $logical.selected_run_id }
        $logicalExact = [string]$logical.state_kind -eq $checkpoint.state_kind -and
            [string]$logical.surface -eq $checkpoint.surface -and
            [int]$logical.completed_result_count -eq $checkpoint.completed -and
            [int]$logical.witness_count -eq $checkpoint.witnesses -and $selectedExact
        $archiveLogicalExact = [bool]$effectiveLogical.manifestValid -and
            [int]$effectiveLogical.completedResultCount -eq $checkpoint.completed -and
            [int]$effectiveLogical.witnessCount -eq $checkpoint.witnesses -and
            [string]$effectiveLogical.selectedRunId -eq [string]$logical.selected_run_id
        $snapshotIdentityExact = [bool]$phase.effective_archive.source_stable_during_copy -and
            [bool]$phase.effective_archive.exclusive_atomic_copy -and
            -not [bool]$phase.effective_archive.application_opened -and
            [string]$phase.effective_archive.path -match '^validation/results/process-v2-resource-snapshot-[0-9]+-[0-9]+-[a-z0-9_]+\.qpls$' -and
            -not [string]::IsNullOrWhiteSpace([string]$phase.effective_archive.source_path) -and
            [long]$sourceBefore.bytes -eq [long]$phase.effective_archive.bytes -and
            [long]$sourceAfter.bytes -eq [long]$phase.effective_archive.bytes -and
            [string]$sourceBefore.sha256 -eq [string]$phase.effective_archive.sha256 -and
            [string]$sourceAfter.sha256 -eq [string]$phase.effective_archive.sha256 -and
            [string]$sourceBefore.mtime_ns -match '^[0-9]+$' -and
            [string]$sourceBefore.mtime_ns -eq [string]$sourceAfter.mtime_ns
        $snapshotAbsolutePath = Join-Path $repositoryRoot ([string]$phase.effective_archive.path).Replace('/', '\')
        $snapshotCurrentExact = Test-Path -LiteralPath $snapshotAbsolutePath -PathType Leaf
        if ($snapshotCurrentExact) {
            $snapshotCurrentExact = (Get-Item -LiteralPath $snapshotAbsolutePath).Length -eq [long]$phase.effective_archive.bytes -and
                (Get-FileHash -LiteralPath $snapshotAbsolutePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq
                    [string]$phase.effective_archive.sha256
        }
        if ($previousPhaseTime -and $phaseTime -le $previousPhaseTime) {
            $failureReasons.Add("phase_order_not_strict")
        }
        if ($sampleTimes[0] -lt $windowStart -or $sampleTimes[-1] -ge $windowEnd) {
            $failureReasons.Add("sample_outside_window")
        }
        if (-not $logicalExact) { $failureReasons.Add("logical_state_mismatch") }
        if (-not $archiveLogicalExact) { $failureReasons.Add("effective_archive_logical_state_mismatch") }
        if (-not $snapshotIdentityExact) { $failureReasons.Add("snapshot_copy_identity_mismatch") }
        if (-not $snapshotCurrentExact) { $failureReasons.Add("snapshot_artifact_mismatch") }
        if (-not $roleWindow.passed) {
            $failureReasons.Add("process_roles_not_bounded_or_terminally_stable")
        }
        if ([string]::IsNullOrWhiteSpace([string]$phase.effective_archive.path) -or
            [long]$phase.effective_archive.bytes -le 0 -or
            [string]$phase.effective_archive.sha256 -notmatch '^[0-9a-f]{64}$') {
            $failureReasons.Add("invalid_snapshot_descriptor")
        }
        $checkpointDiagnostic.passed = $failureReasons.Count -eq 0
        $checkpointDiagnostic.failure_reasons = @($failureReasons)
        $checkpointDiagnostics += $checkpointDiagnostic
        if (-not $checkpointDiagnostic.passed) { $checkpointContract = $false }
        $workingSetValues = [long[]]@($windowSamples | ForEach-Object { [long]$_.total_working_set_bytes })
        $privateValues = [long[]]@($windowSamples | ForEach-Object { [long]$_.total_private_memory_bytes })
        $handleValues = [long[]]@($windowSamples | ForEach-Object { [long]$_.total_handle_count })
        $threadValues = [long[]]@($windowSamples | ForEach-Object { [long]$_.total_thread_count })
        $processValues = [long[]]@($windowSamples | ForEach-Object { [long]$_.processes.Count })
        $checkpointEvidence += [ordered]@{
            name = $checkpoint.name
            phase_recorded_at_utc = $phaseTime.ToString("o")
            window_start_utc = $windowStart.ToString("o")
            window_end_utc = $windowEnd.ToString("o")
            sample_recorded_at_utc = @($sampleTimes | ForEach-Object { $_.ToString("o") })
            sample_count = $windowSamples.Count
            median_working_set_bytes = Get-MedianLong -Values $workingSetValues
            p95_working_set_bytes = Get-P95Long -Values $workingSetValues
            median_private_memory_bytes = Get-MedianLong -Values $privateValues
            p95_private_memory_bytes = Get-P95Long -Values $privateValues
            median_handle_count = Get-MedianLong -Values $handleValues
            p95_handle_count = Get-P95Long -Values $handleValues
            median_thread_count = Get-MedianLong -Values $threadValues
            p95_thread_count = Get-P95Long -Values $threadValues
            median_process_count = Get-MedianLong -Values $processValues
            p95_process_count = Get-P95Long -Values $processValues
            process_role_counts = $roleWindow.modal_role_counts
            process_roles_bounded_and_terminally_stable = [bool]$roleWindow.passed
            process_role_window = $roleWindow
            idle_settle_milliseconds = [int]$phase.idle_settle_milliseconds
            capture_delay_milliseconds = [int]$phase.capture_delay_milliseconds
            sample_window_milliseconds = [int]$phase.sample_window_milliseconds
            logical_state = $logical
            effective_archive = $phase.effective_archive
        }
        $previousPhaseTime = $phaseTime
    }
    $checkpointContract = $checkpointContract -and $checkpointEvidence.Count -eq 5 -and
        $checkpointDiagnostics.Count -eq 5 -and
        @($checkpointDiagnostics | Where-Object { -not $_.passed }).Count -eq 0
    $processRolesBoundedAndTerminallyStable = $checkpointEvidence.Count -eq 5 -and
        @($checkpointEvidence | Where-Object {
            -not $_.process_roles_bounded_and_terminally_stable
        }).Count -eq 0
    $checkpointByName = @{}
    foreach ($row in $checkpointEvidence) { $checkpointByName[[string]$row.name] = $row }
    $peakSample = @($liveSamples | Sort-Object total_working_set_bytes -Descending)[0]
    $peakBytes = if ($peakSample) { [long]$peakSample.total_working_set_bytes } else { 0L }
    $peakPrivateBytes = if ($liveSamples.Count -gt 0) {
        [long](@($liveSamples | Sort-Object total_private_memory_bytes -Descending)[0].total_private_memory_bytes)
    } else { 0L }
    $initialCheckpoint = $checkpointByName["initial_idle"]
    $cancelCheckpoint = $checkpointByName["post_cancellation_idle"]
    $cycle1Checkpoint = $checkpointByName["post_completed_cycle_1_idle"]
    $historyCheckpoint = $checkpointByName["post_completed_history_2_idle"]
    $cycle2Checkpoint = $checkpointByName["post_completed_cycle_2_idle"]
    $cancellationTerminalMinimumSamples = 6
    $cancellationWindowSamples = @($checkpointSamplesByName["post_cancellation_idle"])
    $cancellationTerminalSamples = @($cancellationWindowSamples | Select-Object -Last $cancellationTerminalMinimumSamples)
    $cancellationModalIdentities = @($cancelCheckpoint.process_role_window.modal_pid_role_identities)
    $cancellationModalKeys = @($cancellationModalIdentities | Sort-Object pid | ForEach-Object {
        Get-ResourceProcessIdentityKey -Process $_
    })
    $cancellationTerminalSamplesRoleStable = $cancellationTerminalSamples.Count -eq $cancellationTerminalMinimumSamples -and
        $cancellationModalKeys.Count -gt 0
    foreach ($sample in $cancellationTerminalSamples) {
        $sampleKeys = @($sample.processes | Sort-Object pid | ForEach-Object {
            Get-ResourceProcessIdentityKey -Process $_
        })
        $reportedCanonicalCounts = [ordered]@{}
        foreach ($role in $resourceRoleNames) {
            $reportedProperty = $sample.process_role_counts.PSObject.Properties[$role]
            $reportedCanonicalCounts[$role] = if ($null -ne $reportedProperty) {
                [int]$reportedProperty.Value
            } else {
                0
            }
        }
        $computedCounts = [ordered]@{}
        foreach ($role in $resourceRoleNames) {
            $computedCounts[$role] = @($sample.processes | Where-Object { $_.role -eq $role }).Count
        }
        [long]$computedWorkingSet = 0L
        [long]$computedPrivateMemory = 0L
        [long]$computedHandles = 0L
        [long]$computedThreads = 0L
        foreach ($process in @($sample.processes)) {
            $computedWorkingSet += [long]$process.working_set_bytes
            $computedPrivateMemory += [long]$process.private_memory_bytes
            $computedHandles += [long]$process.handle_count
            $computedThreads += [long]$process.thread_count
        }
        $cancellationTerminalSamplesRoleStable = $cancellationTerminalSamplesRoleStable -and
            (($sampleKeys | ConvertTo-Json -Compress) -eq ($cancellationModalKeys | ConvertTo-Json -Compress)) -and
            (($reportedCanonicalCounts | ConvertTo-Json -Compress) -eq
                ($cancelCheckpoint.process_role_counts | ConvertTo-Json -Compress)) -and
            (($reportedCanonicalCounts | ConvertTo-Json -Compress) -eq ($computedCounts | ConvertTo-Json -Compress)) -and
            $computedWorkingSet -eq [long]$sample.total_working_set_bytes -and
            $computedPrivateMemory -eq [long]$sample.total_private_memory_bytes -and
            $computedHandles -eq [long]$sample.total_handle_count -and
            $computedThreads -eq [long]$sample.total_thread_count
    }
    $cancellationTerminalMaxWorkingSet = if ($cancellationTerminalSamples.Count -eq $cancellationTerminalMinimumSamples) {
        [long](@($cancellationTerminalSamples | Sort-Object total_working_set_bytes -Descending)[0].total_working_set_bytes)
    } else { 0L }
    $cancellationTerminalMaxPrivateMemory = if ($cancellationTerminalSamples.Count -eq $cancellationTerminalMinimumSamples) {
        [long](@($cancellationTerminalSamples | Sort-Object total_private_memory_bytes -Descending)[0].total_private_memory_bytes)
    } else { 0L }
    function Get-ResourceRoleMedianDisclosure {
        param([object[]]$BaselineSamples, [object[]]$CancellationSamples)
        $rows = @()
        foreach ($role in $resourceRoleNames) {
            $baselineWorking = @()
            $baselinePrivate = @()
            foreach ($sample in $BaselineSamples) {
                [long]$working = 0L
                [long]$private = 0L
                foreach ($process in @($sample.processes | Where-Object { $_.role -eq $role })) {
                    $working += [long]$process.working_set_bytes
                    $private += [long]$process.private_memory_bytes
                }
                $baselineWorking += $working
                $baselinePrivate += $private
            }
            $cancellationWorking = @()
            $cancellationPrivate = @()
            foreach ($sample in $CancellationSamples) {
                [long]$working = 0L
                [long]$private = 0L
                foreach ($process in @($sample.processes | Where-Object { $_.role -eq $role })) {
                    $working += [long]$process.working_set_bytes
                    $private += [long]$process.private_memory_bytes
                }
                $cancellationWorking += $working
                $cancellationPrivate += $private
            }
            $baselineWorkingMedian = Get-MedianLong -Values ([long[]]$baselineWorking)
            $cancellationWorkingMedian = Get-MedianLong -Values ([long[]]$cancellationWorking)
            $baselinePrivateMedian = Get-MedianLong -Values ([long[]]$baselinePrivate)
            $cancellationPrivateMedian = Get-MedianLong -Values ([long[]]$cancellationPrivate)
            $rows += [ordered]@{
                role = $role
                baseline_median_working_set_bytes = $baselineWorkingMedian
                cancellation_median_working_set_bytes = $cancellationWorkingMedian
                working_set_delta_bytes = $cancellationWorkingMedian - $baselineWorkingMedian
                baseline_median_private_memory_bytes = $baselinePrivateMedian
                cancellation_median_private_memory_bytes = $cancellationPrivateMedian
                private_memory_delta_bytes = $cancellationPrivateMedian - $baselinePrivateMedian
            }
        }
        return $rows
    }
    $fullWindowDisclosure = [ordered]@{
        qualification_role = "disclosure_only_not_a_threshold"
        baseline_checkpoint = "initial_idle"
        cancellation_checkpoint = "post_cancellation_idle"
        baseline_median_working_set_bytes = [long]$initialCheckpoint.median_working_set_bytes
        cancellation_median_working_set_bytes = [long]$cancelCheckpoint.median_working_set_bytes
        working_set_delta_bytes = [long]$cancelCheckpoint.median_working_set_bytes - [long]$initialCheckpoint.median_working_set_bytes
        baseline_median_private_memory_bytes = [long]$initialCheckpoint.median_private_memory_bytes
        cancellation_median_private_memory_bytes = [long]$cancelCheckpoint.median_private_memory_bytes
        private_memory_delta_bytes = [long]$cancelCheckpoint.median_private_memory_bytes - [long]$initialCheckpoint.median_private_memory_bytes
        per_role_deltas = @(Get-ResourceRoleMedianDisclosure `
            -BaselineSamples @($checkpointSamplesByName["initial_idle"]) `
            -CancellationSamples $cancellationWindowSamples)
    }
    $cancelWorkingTolerance = [long][Math]::Max(134217728, [Math]::Ceiling([long]$initialCheckpoint.median_working_set_bytes * 0.35))
    $cancelPrivateTolerance = [long][Math]::Max(134217728, [Math]::Ceiling([long]$initialCheckpoint.median_private_memory_bytes * 0.35))
    $equalWorkingTolerance = [long][Math]::Max(67108864, [Math]::Ceiling([long]$cycle1Checkpoint.median_working_set_bytes * 0.10))
    $equalPrivateTolerance = [long][Math]::Max(67108864, [Math]::Ceiling([long]$cycle1Checkpoint.median_private_memory_bytes * 0.10))
    $cancellationWithin = $cancellationTerminalSamplesRoleStable -and
        $cancellationTerminalMaxWorkingSet -le [long]$initialCheckpoint.median_working_set_bytes + $cancelWorkingTolerance -and
        $cancellationTerminalMaxPrivateMemory -le [long]$initialCheckpoint.median_private_memory_bytes + $cancelPrivateTolerance
    $equalWorkingWithin = [long]$cycle2Checkpoint.median_working_set_bytes -le [long]$cycle1Checkpoint.median_working_set_bytes + $equalWorkingTolerance
    $equalPrivateWithin = [long]$cycle2Checkpoint.median_private_memory_bytes -le [long]$cycle1Checkpoint.median_private_memory_bytes + $equalPrivateTolerance
    $equalHandlesWithin = [long]$cycle2Checkpoint.median_handle_count -le [long]$cycle1Checkpoint.median_handle_count + 64
    $equalThreadsWithin = [long]$cycle2Checkpoint.median_thread_count -le [long]$cycle1Checkpoint.median_thread_count + 16
    $equalRolesExact = ($cycle1Checkpoint.process_role_counts | ConvertTo-Json -Compress) -eq
        ($cycle2Checkpoint.process_role_counts | ConvertTo-Json -Compress)
    $retainedHistoryDisclosure = [ordered]@{
        checkpoint = "post_completed_history_2_idle"
        median_working_set_bytes = [long]$historyCheckpoint.median_working_set_bytes
        median_private_memory_bytes = [long]$historyCheckpoint.median_private_memory_bytes
        completed_result_count = [int]$historyCheckpoint.logical_state.completed_result_count
        witness_count = [int]$historyCheckpoint.logical_state.witness_count
        qualification_role = "disclosure_only_not_a_threshold"
    }
    $peakWithin = $peakBytes -gt 0 -and $peakBytes -lt 2147483648
    $initialArchiveBytes = if ($initialPhase) { [long]$initialPhase.primary_archive.bytes } else { 0L }
    $finalArchiveBytes = if ($completedCycle1Phase) { [long]$completedCycle1Phase.primary_archive.bytes } else { 0L }
    $initialExportBytes = if ($initialPhase) { [long]$initialPhase.export.bytes } else { 0L }
    $finalExportBytes = if ($completedCycle1Phase) { [long]$completedCycle1Phase.export.bytes } else { 0L }
    $archiveDelta = $finalArchiveBytes - $initialArchiveBytes
    $exportDelta = $finalExportBytes - $initialExportBytes
    function Copy-EvidenceFileWithRetry {
        param([string]$Source, [string]$Destination)
        for ($attempt = 0; $attempt -lt 8; $attempt++) {
            try {
                Copy-Item -LiteralPath $Source -Destination $Destination -Force -ErrorAction Stop
                return
            } catch {
                if ($attempt -eq 7) { throw }
                Start-Sleep -Milliseconds (100 * ($attempt + 1))
            }
        }
    }
    function Write-EvidenceTextWithRetry {
        param(
            [string]$Path,
            [string]$Content,
            [System.Text.Encoding]$Encoding
        )
        for ($attempt = 0; $attempt -lt 12; $attempt++) {
            try {
                [System.IO.File]::WriteAllText($Path, $Content, $Encoding)
                return
            } catch {
                if ($attempt -eq 11) { throw }
                Start-Sleep -Milliseconds 250
            }
        }
    }
    Copy-EvidenceFileWithRetry -Source $resourceSamplesPath -Destination $resourceSamplesEvidencePath
    $resourceSamplesHash = (Get-FileHash -LiteralPath $resourceSamplesEvidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $resourceSamplesLength = (Get-Item -LiteralPath $resourceSamplesEvidencePath).Length
    $resourcePhasesDescriptor = [ordered]@{
        path = "validation/results/process_v2_resource_phases.json"
        size = 0L
        sha256 = $null
    }
    $phaseDocumentCopiedExactly = $false
    if (Test-Path -LiteralPath $resourcePhasesPath -PathType Leaf) {
        $resourcePhasesSourceLength = (Get-Item -LiteralPath $resourcePhasesPath).Length
        $resourcePhasesSourceHash = (Get-FileHash -LiteralPath $resourcePhasesPath -Algorithm SHA256).Hash.ToLowerInvariant()
        Copy-EvidenceFileWithRetry -Source $resourcePhasesPath -Destination $resourcePhasesEvidencePath
        $resourcePhasesDescriptor.size = (Get-Item -LiteralPath $resourcePhasesEvidencePath).Length
        $resourcePhasesDescriptor.sha256 = (Get-FileHash -LiteralPath $resourcePhasesEvidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
        $phaseDocumentCopiedExactly = $resourcePhasesDescriptor.size -eq $resourcePhasesSourceLength -and
            $resourcePhasesDescriptor.sha256 -eq $resourcePhasesSourceHash
    }
    $expectedPhaseNames = @(
        "initial_idle", "post_cancellation_idle", "post_completed_cycle_1_idle",
        "post_completed_history_2_idle", "post_completed_cycle_2_idle"
    )
    $phaseDocumentContract = $phaseDocumentCopiedExactly -and $phaseDocument -and
        [int]$phaseDocument.schema_version -eq 2 -and
        [string]$phaseDocument.feature_id -eq "qpls3.standalone.process" -and
        [string]$phaseDocument.method_version -eq "regression_process_v2" -and
        (($phaseDocument.phases.PSObject.Properties.Name | ConvertTo-Json -Compress) -eq
            ($expectedPhaseNames | ConvertTo-Json -Compress))
    $checkpointDiagnosticsAllPassed = $checkpointDiagnostics.Count -eq 5 -and
        @($checkpointDiagnostics | Where-Object { -not $_.passed }).Count -eq 0
    $resourcePassed = $cleanup.passed -and $liveSamples.Count -ge 30 -and $checkpointContract -and
        $checkpointDiagnosticsAllPassed -and $phaseSnapshotContract -and $phaseDocumentContract -and
        $processRolesBoundedAndTerminallyStable -and
        $cancellationWithin -and $equalWorkingWithin -and $equalPrivateWithin -and
        $equalHandlesWithin -and $equalThreadsWithin -and $equalRolesExact -and $peakWithin -and
        $initialExportBytes -eq 0 -and $finalExportBytes -gt 0 -and
        $exportDelta -eq $finalExportBytes -and $initialArchiveBytes -gt 0 -and
        $finalArchiveBytes -gt 0 -and $archiveDelta -gt 0
    $resourceReport = [ordered]@{
        schema_version = 1
        target = "process_v2_packaged_resource_report"
        feature_id = "qpls3.standalone.process"
        method_version = "regression_process_v2"
        generated_at_utc = [DateTime]::UtcNow.ToString("o")
        launched_pid = if ($application) { $application.Id } else { $null }
        sample_interval_milliseconds = 250
        sample_count = $liveSamples.Count
        raw_sample_count = $samples.Count
        first_sample = $resourceMonitorFirstSample
        monitor_terminal_reason = $cleanup.resource_monitor_terminal_reason
        capture_delay_milliseconds = 500
        sample_window_milliseconds = $resourceSampleWindowMilliseconds
        raw_samples = [ordered]@{
            path = "validation/results/process_v2_resource_samples.jsonl"
            size = $resourceSamplesLength
            sha256 = $resourceSamplesHash
        }
        phase_document = $resourcePhasesDescriptor
        phase_snapshots = $phaseSnapshotArtifacts
        idle_checkpoints = $checkpointEvidence
        checkpoint_diagnostics = $checkpointDiagnostics
        memory = [ordered]@{
            policy = "bounded_equal_logical_state_terminal_stable_v3"
            peak_working_set_bytes = $peakBytes
            peak_private_memory_bytes = $peakPrivateBytes
            peak_working_set_under_2_gib = $peakWithin
            cancellation_working_set_tolerance_bytes = $cancelWorkingTolerance
            cancellation_private_memory_tolerance_bytes = $cancelPrivateTolerance
            cancellation_terminal_sample_count = $cancellationTerminalSamples.Count
            cancellation_terminal_minimum_samples = $cancellationTerminalMinimumSamples
            cancellation_terminal_samples_role_stable = $cancellationTerminalSamplesRoleStable
            cancellation_terminal_sample_recorded_at_utc = @($cancellationTerminalSamples | ForEach-Object {
                (ConvertTo-UtcDateTime $_.recorded_at_utc).ToString("o")
            })
            cancellation_terminal_max_working_set_bytes = $cancellationTerminalMaxWorkingSet
            cancellation_terminal_max_private_memory_bytes = $cancellationTerminalMaxPrivateMemory
            cancellation_within_baseline_tolerance = $cancellationWithin
            full_window_disclosure = $fullWindowDisclosure
            equal_state_working_set_tolerance_bytes = $equalWorkingTolerance
            equal_state_private_memory_tolerance_bytes = $equalPrivateTolerance
            equal_state_working_set_within_tolerance = $equalWorkingWithin
            equal_state_private_memory_within_tolerance = $equalPrivateWithin
            equal_state_handle_tolerance = 64
            equal_state_thread_tolerance = 16
            equal_state_handle_count_within_tolerance = $equalHandlesWithin
            equal_state_thread_count_within_tolerance = $equalThreadsWithin
            equal_state_process_roles_exact = $equalRolesExact
            process_roles_bounded_and_terminally_stable = $processRolesBoundedAndTerminallyStable
            retained_history_disclosure = $retainedHistoryDisclosure
            phase_snapshots_attested = $phaseSnapshotContract
            phase_document_attested = $phaseDocumentContract
            conclusion = "bounded_post_replacement_recovery_terminal_stable_v3"
            cancellation_cycle_count = 1
            completed_cycle_count = 2
            idle_checkpoint_count = 5
            idle_settle_milliseconds = 5000
            idle_checkpoints_ordered_and_distinct = $checkpointContract
            capture_delay_milliseconds = 500
            sample_window_milliseconds = $resourceSampleWindowMilliseconds
            minimum_samples_per_checkpoint = 6
            checkpoint_diagnostic_count = $checkpointDiagnostics.Count
            checkpoint_diagnostics_all_passed = $checkpointDiagnosticsAllPassed
        }
        disk = [ordered]@{
            project_archive = [ordered]@{
                path = if ($completedCycle1Phase) { [string]$completedCycle1Phase.primary_archive.path } else { $null }
                initial_bytes = $initialArchiveBytes
                final_bytes = $finalArchiveBytes
                delta_bytes = $archiveDelta
            }
            xlsx_export = [ordered]@{
                path = if ($completedCycle1Phase) { [string]$completedCycle1Phase.export.path } else { $null }
                initial_bytes = $initialExportBytes
                final_bytes = $finalExportBytes
                delta_bytes = $exportDelta
            }
        }
        process_cleanup = [ordered]@{
            graceful_close_exit_code = $cleanup.graceful_close_exit_code
            graceful_exit_confirmed = $cleanup.graceful_exit_confirmed
            forced_parent_termination = $cleanup.forced_parent_termination
            forced_descendant_pids = @($cleanup.forced_descendant_pids)
            forced_resource_monitor_termination = $cleanup.forced_resource_monitor_termination
            parent_exit_confirmed = $cleanup.parent_exit_confirmed
            lingering_descendant_pids = @($cleanup.lingering_descendant_pids)
            resource_monitor_exit_confirmed = $cleanup.resource_monitor_exit_confirmed
            resource_monitor_exit_code = $cleanup.resource_monitor_exit_code
            resource_monitor_stderr = $cleanup.resource_monitor_stderr
            resource_monitor_terminal_reason = $cleanup.resource_monitor_terminal_reason
        }
        passed = $resourcePassed
    }
    $utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
    Write-EvidenceTextWithRetry `
        -Path $resourceReportPath `
        -Content (($resourceReport | ConvertTo-Json -Depth 8) + [Environment]::NewLine) `
        -Encoding $utf8WithoutBom

    if (Test-Path -LiteralPath $packagedReportPath) {
        $packaged = Get-Content -LiteralPath $packagedReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
        $resourceHash = (Get-FileHash -LiteralPath $resourceReportPath -Algorithm SHA256).Hash.ToLowerInvariant()
        $resourceLength = (Get-Item -LiteralPath $resourceReportPath).Length
        $resourceCheck = [ordered]@{
            passed = $resourcePassed
            sample_count = $liveSamples.Count
            raw_sample_count = $samples.Count
            first_sample = $resourceMonitorFirstSample
            monitor_terminal_reason = $cleanup.resource_monitor_terminal_reason
            peak_working_set_bytes = $peakBytes
            peak_private_memory_bytes = $peakPrivateBytes
            peak_working_set_under_2_gib = $peakWithin
            policy = "bounded_equal_logical_state_terminal_stable_v3"
            cancellation_terminal_sample_count = $cancellationTerminalSamples.Count
            cancellation_terminal_minimum_samples = $cancellationTerminalMinimumSamples
            cancellation_terminal_samples_role_stable = $cancellationTerminalSamplesRoleStable
            cancellation_terminal_sample_recorded_at_utc = @($cancellationTerminalSamples | ForEach-Object {
                (ConvertTo-UtcDateTime $_.recorded_at_utc).ToString("o")
            })
            cancellation_terminal_max_working_set_bytes = $cancellationTerminalMaxWorkingSet
            cancellation_terminal_max_private_memory_bytes = $cancellationTerminalMaxPrivateMemory
            cancellation_within_baseline_tolerance = $cancellationWithin
            full_window_disclosure = $fullWindowDisclosure
            equal_state_working_set_within_tolerance = $equalWorkingWithin
            equal_state_private_memory_within_tolerance = $equalPrivateWithin
            equal_state_handle_count_within_tolerance = $equalHandlesWithin
            equal_state_thread_count_within_tolerance = $equalThreadsWithin
            equal_state_process_roles_exact = $equalRolesExact
            process_roles_bounded_and_terminally_stable = $processRolesBoundedAndTerminallyStable
            retained_history_disclosure = $retainedHistoryDisclosure
            phase_snapshots_attested = $phaseSnapshotContract
            phase_document_attested = $phaseDocumentContract
            conclusion = "bounded_post_replacement_recovery_terminal_stable_v3"
            cancellation_cycle_count = 1
            completed_cycle_count = 2
            idle_checkpoint_count = 5
            idle_settle_milliseconds = 5000
            idle_checkpoints_ordered_and_distinct = $checkpointContract
            capture_delay_milliseconds = 500
            sample_window_milliseconds = $resourceSampleWindowMilliseconds
            minimum_samples_per_checkpoint = 6
            checkpoint_diagnostic_count = $checkpointDiagnostics.Count
            checkpoint_diagnostics_all_passed = $checkpointDiagnosticsAllPassed
            artifact_disk_deltas_recorded = $archiveDelta -gt 0 -and $exportDelta -gt 0
            zero_lingering_descendants = $cleanup.lingering_descendant_pids.Count -eq 0
            graceful_exit_confirmed = $cleanup.graceful_exit_confirmed
            parent_absent = $cleanup.parent_exit_confirmed
            forced_parent_termination = $cleanup.forced_parent_termination
            forced_descendant_pids = @($cleanup.forced_descendant_pids)
            forced_resource_monitor_termination = $cleanup.forced_resource_monitor_termination
            source_check = "processV2Resources"
        }
        $packaged.checks | Add-Member -NotePropertyName resources -NotePropertyValue $resourceCheck -Force
        $packaged.artifacts | Add-Member -NotePropertyName resource_report -NotePropertyValue ([ordered]@{
            path = "validation/results/process_v2_resource_report.json"
            size = $resourceLength
            sha256 = $resourceHash
        }) -Force
        $packaged.artifacts | Add-Member -NotePropertyName resource_samples -NotePropertyValue ([ordered]@{
            path = "validation/results/process_v2_resource_samples.jsonl"
            size = $resourceSamplesLength
            sha256 = $resourceSamplesHash
        }) -Force
        $packaged.artifacts | Add-Member -NotePropertyName resource_phases -NotePropertyValue $resourcePhasesDescriptor -Force
        $packaged.artifacts | Add-Member -NotePropertyName resource_phase_snapshots -NotePropertyValue $phaseSnapshotArtifacts -Force
        $packaged.passed = [bool]$packaged.passed -and $resourcePassed
        Write-EvidenceTextWithRetry `
            -Path $packagedReportPath `
            -Content (($packaged | ConvertTo-Json -Depth 100) + [Environment]::NewLine) `
            -Encoding $utf8WithoutBom
    }

    $cleanup.generated_at_utc = [DateTime]::UtcNow.ToString("o")
    Write-EvidenceTextWithRetry `
        -Path $cleanupReportPath `
        -Content (($cleanup | ConvertTo-Json -Depth 8) + [Environment]::NewLine) `
        -Encoding $utf8WithoutBom
    Remove-Item -LiteralPath `
        $resourceSamplesPath, $resourcePhasesPath, $resourceStopPath, `
        $resourceMonitorStdoutPath, $resourceMonitorStderrPath `
        -Force -ErrorAction SilentlyContinue
    if (-not $cleanup.passed) {
        throw "QuickPLS exact-PID cleanup failed: $($cleanup | ConvertTo-Json -Compress -Depth 5)"
    }
    if (-not $resourcePassed) {
        throw "QuickPLS PROCESS v2 resource gate failed: $($resourceReport | ConvertTo-Json -Compress -Depth 8)"
    }
    } finally {
        foreach ($name in $acceptanceEnvironmentNames) {
            $prior = $priorAcceptanceEnvironment[$name]
            if ($prior.present) {
                [Environment]::SetEnvironmentVariable($name, [string]$prior.value, "Process")
            } else {
                [Environment]::SetEnvironmentVariable($name, $null, "Process")
            }
        }
    }
}
