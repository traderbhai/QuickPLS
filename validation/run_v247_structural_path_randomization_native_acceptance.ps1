param(
    [string]$ExportPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

function Resolve-WorkspaceOutputPath {
    param([string]$Value, [string]$DefaultName)
    $candidate = if ([string]::IsNullOrWhiteSpace($Value)) {
        Join-Path $repositoryRoot $DefaultName
    } elseif ([System.IO.Path]::IsPathRooted($Value)) {
        [System.IO.Path]::GetFullPath($Value)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $Value))
    }
    $resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
    $resultsPrefix = $resultsRoot.TrimEnd([char[]]@('\', '/')) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $candidate.StartsWith($resultsPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "ExportPath must be a new .xlsx file under validation/results."
    }
    $relative = $candidate.Substring($resultsPrefix.Length)
    if ([string]::IsNullOrWhiteSpace($relative) -or
        -not [System.IO.Path]::GetExtension($candidate).Equals(".xlsx", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "ExportPath must be a new .xlsx file under validation/results."
    }
    return $candidate
}

function Get-ExactDescendantProcesses {
    param([int]$RootProcessId)
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Select-Object ProcessId, ParentProcessId, CreationDate, Name, ExecutablePath)
    $pending = New-Object 'System.Collections.Generic.Queue[int]'
    $pending.Enqueue($RootProcessId)
    $descendantIds = New-Object 'System.Collections.Generic.HashSet[int]'
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($descendantIds.Add($childId)) { $pending.Enqueue($childId) }
        }
    }
    return @($rows | Where-Object { $descendantIds.Contains([int]$_.ProcessId) } | Sort-Object ProcessId)
}

function Get-LiveExactProcessIds {
    param([object[]]$Processes)
    return @($Processes | ForEach-Object {
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $([int]$_.ProcessId)" -ErrorAction SilentlyContinue
        if ($current -and [string]$current.CreationDate -eq [string]$_.CreationDate -and
            [string]$current.Name -eq [string]$_.Name) {
            [int]$_.ProcessId
        }
    })
}

function Get-ArtifactDescriptor {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    $item = Get-Item -LiteralPath $Path
    if ($item.Length -le 0) { return $null }
    return [ordered]@{
        path = $item.FullName.Substring($repositoryRoot.TrimEnd([char[]]@('\', '/')).Length).TrimStart([char[]]@('\', '/')).Replace('\', '/')
        size = [long]$item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Write-Utf8Json {
    param([string]$Path, [object]$Value, [int]$Depth = 12)
    $utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine), $utf8WithoutBom)
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$ExportPath = Resolve-WorkspaceOutputPath -Value $ExportPath -DefaultName "validation\results\structural-path-randomization-v1-$stamp.xlsx"
if (Test-Path -LiteralPath $ExportPath) { throw "The Structural Path Randomization XLSX target already exists: $ExportPath" }

$desktopExecutable = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$cliExecutable = Join-Path $repositoryRoot "target\release\qpls.exe"
$rawReportPath = Join-Path $repositoryRoot "validation\results\v247_tauri_native_acceptance_structural_path_randomization.json"
$packagedReportPath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_packaged_acceptance.json"
$samplePath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_process_samples.jsonl"
$resourceReportPath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_resource_report.json"
$cleanupReportPath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_process_cleanup.json"
$stopSignalPath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_process_monitor.stop"
$monitorStderrPath = Join-Path $repositoryRoot "validation\results\structural_path_randomization_v1_process_monitor.stderr.log"

foreach ($required in @($desktopExecutable, $cliExecutable)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required release artifact is missing: $required" }
}
if (Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue) {
    throw "Close every existing quickpls-desktop.exe instance before packaged acceptance."
}
foreach ($transient in @($samplePath, $resourceReportPath, $cleanupReportPath, $stopSignalPath, $monitorStderrPath)) {
    Remove-Item -LiteralPath $transient -Force -ErrorAction SilentlyContinue
}

$environmentNames = @(
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
    "QUICKPLS_CDP_ENDPOINT",
    "QUICKPLS_CLI_PATH",
    "QUICKPLS_PYTHON",
    "QUICKPLS_ACCEPTANCE_SCOPE",
    "QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH",
    "QUICKPLS_DESKTOP_EXE_PATH"
)
$priorEnvironment = @{}
foreach ($name in $environmentNames) { $priorEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }

$application = $null
$resourceMonitor = $null
$resourceMonitorHandle = $null
$resourceMonitorFirstSample = $null
$nodeExitCode = $null
$nodeError = $null
$descendantsAtShutdown = @()
$cleanup = [ordered]@{
    passed = $false
    launched_pid = $null
    descendants_at_shutdown = @()
    graceful_close_exit_code = $null
    graceful_exit_confirmed = $false
    forced_parent_termination = $false
    forced_descendant_pids = @()
    parent_exit_confirmed = $false
    lingering_descendant_pids = @()
    resource_monitor_exit_confirmed = $false
    resource_monitor_exit_code = $null
    forced_resource_monitor_termination = $false
    resource_monitor_stderr = ""
}

try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
    $env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
    $env:QUICKPLS_CLI_PATH = $cliExecutable
    $env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
    $env:QUICKPLS_ACCEPTANCE_SCOPE = "structural_path_randomization"
    $env:QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH = $ExportPath
    $env:QUICKPLS_DESKTOP_EXE_PATH = $desktopExecutable

    $application = Start-Process -FilePath $desktopExecutable -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
    $cleanup.launched_pid = $application.Id
    $resourceMonitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @(
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            (Join-Path $repositoryRoot "validation\monitor_quickpls_process_tree.ps1"),
            "-RootProcessId", [string]$application.Id,
            "-SamplesPath", $samplePath,
            "-StopSignalPath", $stopSignalPath,
            "-IntervalMilliseconds", "250"
        ) `
        -WorkingDirectory $repositoryRoot `
        -RedirectStandardError $monitorStderrPath `
        -WindowStyle Hidden `
        -PassThru
    $resourceMonitorHandle = $resourceMonitor.Handle

    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($resourceMonitor.HasExited) { break }
        if (Test-Path -LiteralPath $samplePath) {
            $firstLine = Get-Content -LiteralPath $samplePath -Encoding UTF8 -TotalCount 1 -ErrorAction SilentlyContinue
            if (-not [string]::IsNullOrWhiteSpace([string]$firstLine)) {
                try {
                    $candidate = $firstLine | ConvertFrom-Json
                    if ($candidate.root_present -eq $true -and [int]$candidate.root_pid -eq $application.Id -and
                        [long]$candidate.total_working_set_bytes -gt 0) {
                        $resourceMonitorFirstSample = $candidate
                        break
                    }
                } catch { }
            }
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $resourceMonitorFirstSample -or $resourceMonitor.HasExited) {
        [string]$monitorStartupError = if (Test-Path -LiteralPath $monitorStderrPath) {
            Get-Content -LiteralPath $monitorStderrPath -Raw -Encoding UTF8
        } else { "" }
        throw "The exact process-tree monitor did not produce a valid first sample: $($monitorStartupError.Trim())"
    }

    $cdpReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Uri $env:QUICKPLS_CDP_ENDPOINT/json/version -TimeoutSec 1
            $cdpReady = $true
            break
        } catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $cdpReady) { throw "QuickPLS WebView2 CDP did not open on port 9222." }

    & node .\validation\v247_tauri_native_acceptance.mjs
    $nodeExitCode = $LASTEXITCODE
    if ($nodeExitCode -ne 0) { $nodeError = "Focused packaged Structural Path Randomization acceptance failed with exit code $nodeExitCode." }
} catch {
    $nodeError = $_.Exception.Message
} finally {
    try {
        if ($resourceMonitor -and -not $resourceMonitor.HasExited) {
            New-Item -ItemType File -Path $stopSignalPath -Force | Out-Null
            $cleanup.resource_monitor_exit_confirmed = $resourceMonitor.WaitForExit(5000)
            if (-not $cleanup.resource_monitor_exit_confirmed -and -not $resourceMonitor.HasExited) {
                Stop-Process -Id $resourceMonitor.Id -Force -ErrorAction SilentlyContinue
                $cleanup.forced_resource_monitor_termination = $true
                $cleanup.resource_monitor_exit_confirmed = $resourceMonitor.WaitForExit(5000)
            }
        } elseif ($resourceMonitor) {
            $cleanup.resource_monitor_exit_confirmed = $true
        }
        if ($resourceMonitor -and $cleanup.resource_monitor_exit_confirmed) {
            $resourceMonitor.WaitForExit()
            $resourceMonitor.Refresh()
            $cleanup.resource_monitor_exit_code = [int]$resourceMonitor.ExitCode
        }
        if (Test-Path -LiteralPath $monitorStderrPath) {
            [string]$monitorStderrText = Get-Content -LiteralPath $monitorStderrPath -Raw -Encoding UTF8
            $cleanup.resource_monitor_stderr = $monitorStderrText.Trim()
        }
    } catch {
        $cleanup.resource_monitor_stderr = "monitor_cleanup_error: $($_.Exception.Message)"
    }

    if ($application) {
        $descendantsAtShutdown = @(Get-ExactDescendantProcesses -RootProcessId $application.Id)
        $cleanup.descendants_at_shutdown = @($descendantsAtShutdown)
    }
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
        $liveDescendants = @(Get-LiveExactProcessIds -Processes $descendantsAtShutdown)
        while ($liveDescendants.Count -gt 0 -and [DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 250
            $liveDescendants = @(Get-LiveExactProcessIds -Processes $descendantsAtShutdown)
        }
        if ($liveDescendants.Count -gt 0) {
            foreach ($processId in $liveDescendants) { Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue }
            $cleanup.forced_descendant_pids = @($liveDescendants)
            Start-Sleep -Milliseconds 500
        }
        $cleanup.lingering_descendant_pids = @(Get-LiveExactProcessIds -Processes $descendantsAtShutdown)
    }
    $cleanup.passed = $cleanup.graceful_close_exit_code -eq 0 -and $cleanup.graceful_exit_confirmed -and
        -not $cleanup.forced_parent_termination -and $cleanup.forced_descendant_pids.Count -eq 0 -and
        $cleanup.parent_exit_confirmed -and $cleanup.lingering_descendant_pids.Count -eq 0 -and
        $cleanup.resource_monitor_exit_confirmed -and $cleanup.resource_monitor_exit_code -eq 0 -and
        -not $cleanup.forced_resource_monitor_termination -and
        [string]::IsNullOrWhiteSpace([string]$cleanup.resource_monitor_stderr)
    Write-Utf8Json -Path $cleanupReportPath -Value $cleanup -Depth 8

    foreach ($name in $environmentNames) {
        [Environment]::SetEnvironmentVariable($name, $priorEnvironment[$name], "Process")
    }
}

$samples = @()
if (Test-Path -LiteralPath $samplePath) {
    foreach ($line in Get-Content -LiteralPath $samplePath -Encoding UTF8) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $samples += $line | ConvertFrom-Json } catch { }
    }
}
$validSamples = @($samples | Where-Object {
    $_.root_present -eq $true -and [int]$_.root_pid -eq [int]$cleanup.launched_pid -and
    [long]$_.total_working_set_bytes -gt 0 -and [long]$_.total_private_memory_bytes -gt 0 -and
    [int]$_.total_handle_count -gt 0 -and [int]$_.total_thread_count -gt 0
})
$peakSample = @($validSamples | Sort-Object total_working_set_bytes -Descending | Select-Object -First 1)
$firstSample = @($validSamples | Select-Object -First 1)
$finalSample = @($validSamples | Select-Object -Last 1)
$zeroOtherDescendants = $validSamples.Count -gt 0 -and @($validSamples | Where-Object {
    [int]($_.process_role_counts.other_descendant) -gt 0
}).Count -eq 0
$resourcePassed = $validSamples.Count -ge 4 -and $firstSample.Count -eq 1 -and $finalSample.Count -eq 1 -and
    $peakSample.Count -eq 1 -and [long]$peakSample[0].total_working_set_bytes -lt 2147483648 -and
    $zeroOtherDescendants -and $cleanup.passed
$resourceReport = [ordered]@{
    schema_version = 1
    feature_id = "qpls3.inference.structural_path_randomization"
    method_version = "freedman_lane_permutation_v1"
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    passed = $resourcePassed
    root_pid = $cleanup.launched_pid
    sample_count = $validSamples.Count
    first_sample = if ($firstSample.Count -eq 1) { $firstSample[0] } else { $null }
    final_sample = if ($finalSample.Count -eq 1) { $finalSample[0] } else { $null }
    peak_total_working_set_bytes = if ($peakSample.Count -eq 1) { [long]$peakSample[0].total_working_set_bytes } else { $null }
    peak_total_private_memory_bytes = if ($validSamples.Count) { [long](@($validSamples | Sort-Object total_private_memory_bytes -Descending)[0].total_private_memory_bytes) } else { $null }
    peak_total_handle_count = if ($validSamples.Count) { [int](@($validSamples | Sort-Object total_handle_count -Descending)[0].total_handle_count) } else { $null }
    peak_total_thread_count = if ($validSamples.Count) { [int](@($validSamples | Sort-Object total_thread_count -Descending)[0].total_thread_count) } else { $null }
    peak_process_count = if ($validSamples.Count) { [int](@($validSamples | ForEach-Object { @($_.processes).Count } | Sort-Object -Descending)[0]) } else { $null }
    peak_working_set_under_2_gib = if ($peakSample.Count -eq 1) { [long]$peakSample[0].total_working_set_bytes -lt 2147483648 } else { $false }
    zero_other_descendants = $zeroOtherDescendants
    scope = "Single packaged cancellation/retry/completion/save/reopen run; this is a bounded process footprint report, not a sustained no-leak claim."
    disk = [ordered]@{
        xlsx_bytes = if (Test-Path -LiteralPath $ExportPath) { [long](Get-Item -LiteralPath $ExportPath).Length } else { 0L }
        source_report_bytes = if (Test-Path -LiteralPath $rawReportPath) { [long](Get-Item -LiteralPath $rawReportPath).Length } else { 0L }
    }
}
Write-Utf8Json -Path $resourceReportPath -Value $resourceReport -Depth 10

$sampleArtifact = Get-ArtifactDescriptor -Path $samplePath
$resourceArtifact = Get-ArtifactDescriptor -Path $resourceReportPath
$cleanupArtifact = Get-ArtifactDescriptor -Path $cleanupReportPath
if (Test-Path -LiteralPath $rawReportPath -PathType Leaf) {
    $raw = Get-Content -LiteralPath $rawReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $raw.checks | Add-Member -NotePropertyName resources -NotePropertyValue ([ordered]@{
        passed = $resourcePassed
        sample_count = $validSamples.Count
        peak_total_working_set_bytes = $resourceReport.peak_total_working_set_bytes
        peak_total_private_memory_bytes = $resourceReport.peak_total_private_memory_bytes
        peak_total_handle_count = $resourceReport.peak_total_handle_count
        peak_total_thread_count = $resourceReport.peak_total_thread_count
        peak_process_count = $resourceReport.peak_process_count
        peak_working_set_under_2_gib = $resourceReport.peak_working_set_under_2_gib
        zero_other_descendants = $resourceReport.zero_other_descendants
        scope = $resourceReport.scope
        disk = $resourceReport.disk
        artifacts = [ordered]@{ samples = $sampleArtifact; report = $resourceArtifact }
    }) -Force
    $raw.checks | Add-Member -NotePropertyName cleanup -NotePropertyValue ([ordered]@{
        passed = $cleanup.passed
        launched_pid = $cleanup.launched_pid
        graceful_close_exit_code = $cleanup.graceful_close_exit_code
        graceful_exit_confirmed = $cleanup.graceful_exit_confirmed
        forced_parent_termination = $cleanup.forced_parent_termination
        forced_descendant_pids = @($cleanup.forced_descendant_pids)
        parent_exit_confirmed = $cleanup.parent_exit_confirmed
        lingering_descendant_pids = @($cleanup.lingering_descendant_pids)
        resource_monitor_exit_confirmed = $cleanup.resource_monitor_exit_confirmed
        resource_monitor_exit_code = $cleanup.resource_monitor_exit_code
        forced_resource_monitor_termination = $cleanup.forced_resource_monitor_termination
        artifact = $cleanupArtifact
    }) -Force
    $raw.passed = [bool]($nodeExitCode -eq 0 -and $cleanup.passed -and $resourcePassed -and
        @($raw.failures).Count -eq 0 -and @($raw.consoleErrors).Count -eq 0)
    Write-Utf8Json -Path $rawReportPath -Value $raw -Depth 20

    $requiredChecks = @(
        "runtimePreflight", "structuralPathRandomizationFixtureProvisioning", "structuralPathRandomizationSetup",
        "structuralPathRandomizationCancellation", "structuralPathRandomizationResults", "structuralPathRandomizationExport",
        "structuralPathRandomizationArchive", "structuralPathRandomizationSaveReopen", "resources", "cleanup"
    )
    $packagedChecks = [ordered]@{}
    foreach ($checkName in $requiredChecks) { $packagedChecks[$checkName] = $raw.checks.$checkName }
    $sourcePackaged = if (Test-Path -LiteralPath $packagedReportPath -PathType Leaf) {
        Get-Content -LiteralPath $packagedReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    } else { $null }
    $allChecksPassed = @($requiredChecks | Where-Object { $packagedChecks[$_].passed -ne $true }).Count -eq 0
    $packaged = [ordered]@{
        schema_version = "quickpls.packaged_acceptance.v1"
        kind = "quickpls3_scoped_tauri_structural_path_randomization_v1_acceptance"
        passed = $false
        generated_at_utc = $raw.generatedAt
        completed_at_utc = $raw.focusedRun.completedAt
        feature_id = "qpls3.inference.structural_path_randomization"
        method_version = "freedman_lane_permutation_v1"
        catalogue_snapshot_date = "2026-08-12"
        target = "windows_10_11_x64_packaged_tauri"
        runtime = $raw.runtime
        endpoint = $raw.endpoint
        generator = "validation/v247_tauri_native_acceptance.mjs"
        acceptance_scope = "structural_path_randomization"
        tested_product = $sourcePackaged.tested_product
        checks = $packagedChecks
        artifacts = [ordered]@{
            xlsx = $sourcePackaged.artifacts.xlsx
            project_archive = $sourcePackaged.artifacts.project_archive
            resource_samples = $sampleArtifact
            resource_report = $resourceArtifact
            cleanup_report = $cleanupArtifact
            cancellation_archive_before = $sourcePackaged.artifacts.cancellation_archive_before
            cancellation_archive_after = $sourcePackaged.artifacts.cancellation_archive_after
            screenshots = $sourcePackaged.artifacts.screenshots
        }
        console_errors = @($raw.consoleErrors)
        failures = @($raw.failures)
        source_report = "validation/results/v247_tauri_native_acceptance_structural_path_randomization.json"
    }
    $packaged.passed = [bool]($raw.passed -and $allChecksPassed -and
        $packaged.artifacts.xlsx -and $packaged.artifacts.project_archive -and
        $packaged.artifacts.cancellation_archive_before -and $packaged.artifacts.cancellation_archive_after -and
        $packaged.tested_product.qpls_cli_exe -and $packaged.tested_product.quickpls_desktop_exe -and
        $packaged.tested_product.dist_bundle -and @($packaged.artifacts.screenshots).Count -ge 6 -and
        @($packaged.console_errors).Count -eq 0 -and @($packaged.failures).Count -eq 0)
    Write-Utf8Json -Path $packagedReportPath -Value $packaged -Depth 24
}

if (-not [string]::IsNullOrWhiteSpace($nodeError)) { throw $nodeError }
if (-not $cleanup.passed) { throw "QuickPLS exact-PID cleanup failed: $($cleanup | ConvertTo-Json -Compress -Depth 6)" }
if (-not $resourcePassed) { throw "QuickPLS bounded Structural Path Randomization resource gate failed: $($resourceReport | ConvertTo-Json -Compress -Depth 6)" }
if (-not (Test-Path -LiteralPath $packagedReportPath)) { throw "The dedicated packaged acceptance report was not produced." }
$finalReport = Get-Content -LiteralPath $packagedReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($finalReport.passed -ne $true) { throw "The dedicated packaged Structural Path Randomization report did not pass." }

Get-Item -LiteralPath $ExportPath, $rawReportPath, $packagedReportPath, $resourceReportPath, $cleanupReportPath |
    Select-Object FullName, Length, LastWriteTime
