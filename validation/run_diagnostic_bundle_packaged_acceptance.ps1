param(
    [string]$ZipPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

function Resolve-NewDiagnosticZipPath {
    param([string]$Value)
    $resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
    $candidate = if ([string]::IsNullOrWhiteSpace($Value)) {
        Join-Path $resultsRoot ("quickpls-diagnostic-packaged-{0}.zip" -f (Get-Date -Format "yyyyMMdd-HHmmssfff"))
    } elseif ([System.IO.Path]::IsPathRooted($Value)) {
        [System.IO.Path]::GetFullPath($Value)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $Value))
    }
    $prefix = $resultsRoot.TrimEnd([char[]]@('\', '/')) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $candidate.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not [System.IO.Path]::GetExtension($candidate).Equals(".zip", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "ZipPath must be a new .zip file under validation/results."
    }
    if (Test-Path -LiteralPath $candidate) { throw "The diagnostic ZIP target already exists: $candidate" }
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
            [string]$current.Name -eq [string]$_.Name) { [int]$_.ProcessId }
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
    param([string]$Path, [object]$Value, [int]$Depth = 24)
    $utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine), $utf8WithoutBom)
}

function Get-ExactCommandLineSwitchCount {
    param([string]$CommandLine, [string]$Switch)
    if ([string]::IsNullOrWhiteSpace($CommandLine) -or [string]::IsNullOrWhiteSpace($Switch)) { return 0 }
    $pattern = "(?i)(?<!\S)$([regex]::Escape($Switch))(?=\s|$)"
    return [regex]::Matches($CommandLine, $pattern).Count
}

function Test-LoopbackAddress {
    param([string]$Address)
    if ([string]::IsNullOrWhiteSpace($Address)) { return $false }
    $normalized = $Address.Trim().TrimStart('[').TrimEnd(']').ToLowerInvariant()
    return $normalized -eq "::1" -or $normalized.StartsWith("127.") -or $normalized.StartsWith("::ffff:127.")
}

function Test-UnspecifiedAddress {
    param([string]$Address)
    if ([string]::IsNullOrWhiteSpace($Address)) { return $true }
    $normalized = $Address.Trim().TrimStart('[').TrimEnd(']').ToLowerInvariant()
    return $normalized -eq "0.0.0.0" -or $normalized -eq "::"
}

function Wait-ForMonitorSample {
    param([System.Diagnostics.Process]$Monitor, [string]$SamplesPath, [string]$Label)
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($Monitor.HasExited) { break }
        if (Test-Path -LiteralPath $SamplesPath -PathType Leaf) {
            $firstLine = Get-Content -LiteralPath $SamplesPath -Encoding UTF8 -TotalCount 1 -ErrorAction SilentlyContinue
            if (-not [string]::IsNullOrWhiteSpace([string]$firstLine)) {
                try {
                    $sample = $firstLine | ConvertFrom-Json
                    if ($sample.root_present -eq $true) { return $sample }
                } catch { }
            }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "$Label did not produce a valid first sample."
}

function Stop-ExactMonitor {
    param(
        [System.Diagnostics.Process]$Monitor,
        [string]$StopSignalPath,
        [string]$StderrPath,
        [string]$Label
    )
    $result = [ordered]@{
        label = $Label
        exit_confirmed = $false
        exit_code = $null
        forced_termination = $false
        stderr = ""
    }
    if ($Monitor -and -not $Monitor.HasExited) {
        New-Item -ItemType File -Path $StopSignalPath -Force | Out-Null
        $result.exit_confirmed = $Monitor.WaitForExit(5000)
        if (-not $result.exit_confirmed -and -not $Monitor.HasExited) {
            Stop-Process -Id $Monitor.Id -Force -ErrorAction SilentlyContinue
            $result.forced_termination = $true
            $result.exit_confirmed = $Monitor.WaitForExit(5000)
        }
    } elseif ($Monitor) {
        $result.exit_confirmed = $true
    }
    if ($Monitor -and $result.exit_confirmed) {
        $Monitor.WaitForExit()
        $Monitor.Refresh()
        $result.exit_code = [int]$Monitor.ExitCode
    }
    if (Test-Path -LiteralPath $StderrPath -PathType Leaf) {
        # Get-Content -Raw returns $null for an empty redirected stderr file in
        # Windows PowerShell. ReadAllText always returns a string, so a clean
        # monitor exit remains distinguishable from a cleanup failure.
        $result.stderr = [System.IO.File]::ReadAllText(
            $StderrPath,
            [System.Text.Encoding]::UTF8
        ).Trim()
    }
    return $result
}

$ZipPath = Resolve-NewDiagnosticZipPath -Value $ZipPath
$desktopExecutable = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$pythonExecutable = if (Test-Path -LiteralPath "C:\Python313\python.exe" -PathType Leaf) { "C:\Python313\python.exe" } else { "python" }
$rawReportPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_packaged_acceptance.raw.json"
$finalReportPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_packaged_acceptance.json"
$processSamplesPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_process_samples.jsonl"
$networkSamplesPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_network_samples.jsonl"
$processReportPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_process_report.json"
$networkReportPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_network_report.json"
$cleanupReportPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_cleanup_report.json"
$buildReceiptPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_build_receipt.json"
$sourceBeforePath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_source_before.json"
$sourceEvidencePath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_source_evidence.json"
$processStopPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_process_monitor.stop"
$networkStopPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_network_monitor.stop"
$processStderrPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_process_monitor.stderr.log"
$networkStderrPath = Join-Path $repositoryRoot "validation\results\diagnostic_bundle_network_monitor.stderr.log"

if (-not (Test-Path -LiteralPath $desktopExecutable -PathType Leaf)) {
    throw "Required frozen release desktop executable is missing: $desktopExecutable"
}
if (Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue) {
    throw "Close every existing quickpls-desktop.exe instance before packaged diagnostic acceptance."
}
if (-not (Test-Path -LiteralPath $buildReceiptPath -PathType Leaf)) {
    throw "The exact frozen-build receipt is missing. Create it with diagnostic_bundle_source_manifest.py build before packaged acceptance."
}
foreach ($transient in @(
    $rawReportPath, $finalReportPath, $processSamplesPath, $networkSamplesPath,
    $processReportPath, $networkReportPath, $cleanupReportPath, $processStopPath,
    $networkStopPath, $processStderrPath, $networkStderrPath, $sourceBeforePath, $sourceEvidencePath
)) {
    Remove-Item -LiteralPath $transient -Force -ErrorAction SilentlyContinue
}
& $pythonExecutable .\validation\diagnostic_bundle_source_manifest.py snapshot --receipt $buildReceiptPath --output $sourceBeforePath | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Frozen source/build receipt verification failed before packaged launch." }

$environmentNames = @(
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "QUICKPLS_CDP_ENDPOINT",
    "QUICKPLS_PYTHON", "QUICKPLS_DIAGNOSTIC_ZIP_PATH", "QUICKPLS_DESKTOP_EXE_PATH"
)
$priorEnvironment = @{}
foreach ($name in $environmentNames) { $priorEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }

$application = $null
$processMonitor = $null
$networkMonitor = $null
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
    process_monitor = $null
    network_monitor = $null
}

try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
    $env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
    $env:QUICKPLS_PYTHON = $pythonExecutable
    $env:QUICKPLS_DIAGNOSTIC_ZIP_PATH = $ZipPath
    $env:QUICKPLS_DESKTOP_EXE_PATH = $desktopExecutable

    $application = Start-Process -FilePath $desktopExecutable -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
    $cleanup.launched_pid = $application.Id
    $processMonitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @(
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            (Join-Path $repositoryRoot "validation\monitor_quickpls_process_tree.ps1"),
            "-RootProcessId", [string]$application.Id,
            "-SamplesPath", $processSamplesPath,
            "-StopSignalPath", $processStopPath,
            "-IntervalMilliseconds", "250"
        ) `
        -WorkingDirectory $repositoryRoot `
        -RedirectStandardError $processStderrPath `
        -WindowStyle Hidden `
        -PassThru
    $networkMonitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @(
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            (Join-Path $repositoryRoot "validation\monitor_quickpls_network.ps1"),
            "-RootProcessId", [string]$application.Id,
            "-SamplesPath", $networkSamplesPath,
            "-StopSignalPath", $networkStopPath,
            "-IntervalMilliseconds", "250"
        ) `
        -WorkingDirectory $repositoryRoot `
        -RedirectStandardError $networkStderrPath `
        -WindowStyle Hidden `
        -PassThru

    $null = Wait-ForMonitorSample -Monitor $processMonitor -SamplesPath $processSamplesPath -Label "Exact process-tree monitor"
    $null = Wait-ForMonitorSample -Monitor $networkMonitor -SamplesPath $networkSamplesPath -Label "Exact process-tree TCP monitor"
    $cdpReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Uri $env:QUICKPLS_CDP_ENDPOINT/json/version -TimeoutSec 1
            $cdpReady = $true
            break
        } catch { Start-Sleep -Milliseconds 500 }
    }
    if (-not $cdpReady) { throw "QuickPLS WebView2 CDP did not open on port 9222." }

    & node .\validation\diagnostic_bundle_packaged_acceptance.mjs
    $nodeExitCode = $LASTEXITCODE
    if ($nodeExitCode -ne 0) { $nodeError = "Packaged diagnostic-bundle browser/native acceptance failed with exit code $nodeExitCode." }
} catch {
    $nodeError = $_.Exception.Message
} finally {
    try { $cleanup.process_monitor = Stop-ExactMonitor -Monitor $processMonitor -StopSignalPath $processStopPath -StderrPath $processStderrPath -Label "process_tree" } catch { $cleanup.process_monitor = [ordered]@{ label = "process_tree"; exit_confirmed = $false; exit_code = $null; forced_termination = $false; stderr = $_.Exception.Message } }
    try { $cleanup.network_monitor = Stop-ExactMonitor -Monitor $networkMonitor -StopSignalPath $networkStopPath -StderrPath $networkStderrPath -Label "network_tcp" } catch { $cleanup.network_monitor = [ordered]@{ label = "network_tcp"; exit_confirmed = $false; exit_code = $null; forced_termination = $false; stderr = $_.Exception.Message } }
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
        $cleanup.process_monitor.exit_confirmed -and $cleanup.process_monitor.exit_code -eq 0 -and
        -not $cleanup.process_monitor.forced_termination -and [string]::IsNullOrWhiteSpace([string]$cleanup.process_monitor.stderr) -and
        $cleanup.network_monitor.exit_confirmed -and $cleanup.network_monitor.exit_code -eq 0 -and
        -not $cleanup.network_monitor.forced_termination -and [string]::IsNullOrWhiteSpace([string]$cleanup.network_monitor.stderr)
    Write-Utf8Json -Path $cleanupReportPath -Value $cleanup -Depth 10
    foreach ($name in $environmentNames) { [Environment]::SetEnvironmentVariable($name, $priorEnvironment[$name], "Process") }
}

& $pythonExecutable .\validation\diagnostic_bundle_source_manifest.py finish-gate --receipt $buildReceiptPath --before $sourceBeforePath --output $sourceEvidencePath | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Product/build or gate-only sources changed during packaged diagnostic acceptance." }
$sourceArtifacts = Get-Content -LiteralPath $sourceEvidencePath -Raw -Encoding UTF8 | ConvertFrom-Json
$sourceStableDuringGate = [bool]$sourceArtifacts.source_stable_during_gate
$sourceFreshnessAfter = $sourceArtifacts.freshness

$processSamples = @()
if (Test-Path -LiteralPath $processSamplesPath -PathType Leaf) {
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $processSamplesPath -Encoding UTF8) {
        $lineNumber += 1
        if ([string]::IsNullOrWhiteSpace($line)) { throw "Process sample JSONL contains a blank row at line $lineNumber." }
        try { $processSamples += $line | ConvertFrom-Json } catch { throw "Process sample JSONL is malformed at line $lineNumber`: $($_.Exception.Message)" }
    }
}
$validProcessSamples = @($processSamples | Where-Object {
    $_.root_present -eq $true -and [int]$_.root_pid -eq [int]$cleanup.launched_pid -and
    [long]$_.total_working_set_bytes -gt 0 -and [long]$_.total_private_memory_bytes -gt 0
})
$frozenProductBrowserSwitches = @(
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--metrics-recording-only",
    "--disable-quic",
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    "--proxy-server=http://127.0.0.1:17846"
)
$acceptanceOnlyBrowserSwitches = @("--remote-debugging-port=9222")
$requiredBrowserSwitches = @($frozenProductBrowserSwitches + $acceptanceOnlyBrowserSwitches)
$observedProcessIdentityMap = [ordered]@{}
$processIdentityStable = $true
$allSampledProcesses = @($validProcessSamples | ForEach-Object { @($_.processes) })
$processCommandLinesPersisted = $allSampledProcesses.Count -gt 0 -and @($allSampledProcesses | Where-Object {
    [string]::IsNullOrWhiteSpace([string]$_.executable_path) -or
    [string]::IsNullOrWhiteSpace([string]$_.command_line)
}).Count -eq 0
foreach ($sampledProcess in $allSampledProcesses) {
    $identity = [ordered]@{
        pid = [int]$sampledProcess.pid
        parent_pid = [int]$sampledProcess.parent_pid
        name = [string]$sampledProcess.name
        role = [string]$sampledProcess.role
        creation_date = [string]$sampledProcess.creation_date
        executable_path = [string]$sampledProcess.executable_path
        command_line = [string]$sampledProcess.command_line
    }
    $identityKey = "{0}|{1}|{2}" -f $identity.pid, $identity.creation_date, $identity.name.ToLowerInvariant()
    if ($observedProcessIdentityMap.Contains($identityKey)) {
        if (($observedProcessIdentityMap[$identityKey] | ConvertTo-Json -Compress) -ne ($identity | ConvertTo-Json -Compress)) {
            $processIdentityStable = $false
        }
    } else {
        $observedProcessIdentityMap[$identityKey] = $identity
    }
}
$observedProcesses = @($observedProcessIdentityMap.Values | Sort-Object pid)
$directBrowserRows = @($allSampledProcesses | Where-Object {
    [int]$_.parent_pid -eq [int]$cleanup.launched_pid -and
    [string]$_.name -ieq "msedgewebview2.exe" -and [string]$_.role -eq "webview_browser"
})
$directBrowserIdentityKeys = @($directBrowserRows | ForEach-Object {
    "{0}|{1}|{2}" -f ([int]$_.pid), ([string]$_.creation_date), ([string]$_.name).ToLowerInvariant()
} | Sort-Object -Unique)
$directBrowserObservedInEverySample = $validProcessSamples.Count -gt 0 -and @($validProcessSamples | Where-Object {
    @($_.processes | Where-Object {
        [int]$_.parent_pid -eq [int]$cleanup.launched_pid -and
        [string]$_.name -ieq "msedgewebview2.exe" -and [string]$_.role -eq "webview_browser"
    }).Count -ne 1
}).Count -eq 0
$directBrowser = if ($directBrowserIdentityKeys.Count -eq 1) {
    $browserRow = $directBrowserRows | Select-Object -First 1
    [ordered]@{
        pid = [int]$browserRow.pid
        parent_pid = [int]$browserRow.parent_pid
        name = [string]$browserRow.name
        role = [string]$browserRow.role
        creation_date = [string]$browserRow.creation_date
        executable_path = [string]$browserRow.executable_path
        command_line = [string]$browserRow.command_line
    }
} else { $null }
$browserCommandLine = if ($directBrowser) { [string]$directBrowser.command_line } else { "" }
$missingBrowserSwitches = @($requiredBrowserSwitches | Where-Object {
    (Get-ExactCommandLineSwitchCount -CommandLine $browserCommandLine -Switch $_) -eq 0
})
$duplicateBrowserSwitches = @($requiredBrowserSwitches | Where-Object {
    (Get-ExactCommandLineSwitchCount -CommandLine $browserCommandLine -Switch $_) -ne 1
})
$conflictingBrowserSwitches = @($requiredBrowserSwitches | ForEach-Object {
    $expectedSwitch = $_
    $switchFamily = ($expectedSwitch -split '=', 2)[0]
    $familyMatches = @([regex]::Matches($browserCommandLine, "(?i)(?<!\S)$([regex]::Escape($switchFamily))(?:=\S+)?(?=\s|$)") | ForEach-Object { $_.Value })
    if ($familyMatches.Count -ne 1 -or $familyMatches[0] -ne $expectedSwitch) { @($familyMatches) }
})
$proxySwitches = @([regex]::Matches($browserCommandLine, '(?i)(?<!\S)--proxy-server(?:=\S+|\s+\S+)') | ForEach-Object { $_.Value })
$debuggingSwitches = @([regex]::Matches($browserCommandLine, '(?i)(?<!\S)--remote-debugging-port(?:=\S+|\s+\S+)') | ForEach-Object { $_.Value })
$browserSwitchContractPassed = $missingBrowserSwitches.Count -eq 0 -and $duplicateBrowserSwitches.Count -eq 0 -and $conflictingBrowserSwitches.Count -eq 0 -and
    $proxySwitches.Count -eq 1 -and $proxySwitches[0] -eq "--proxy-server=http://127.0.0.1:17846" -and
    $debuggingSwitches.Count -eq 1 -and $debuggingSwitches[0] -eq "--remote-debugging-port=9222"
$peakWorkingSet = if ($validProcessSamples.Count) { [long](@($validProcessSamples | Sort-Object total_working_set_bytes -Descending)[0].total_working_set_bytes) } else { 0L }
$zeroOtherDescendants = $validProcessSamples.Count -gt 0 -and @($validProcessSamples | Where-Object { [int]($_.process_role_counts.other_descendant) -gt 0 }).Count -eq 0
$processPassed = $validProcessSamples.Count -ge 4 -and $peakWorkingSet -lt 2147483648 -and $zeroOtherDescendants -and
    $processCommandLinesPersisted -and $processIdentityStable -and $directBrowserIdentityKeys.Count -eq 1 -and
    $directBrowserObservedInEverySample -and $browserSwitchContractPassed -and $cleanup.passed
$processReport = [ordered]@{
    schema_version = 1
    passed = $processPassed
    root_pid = $cleanup.launched_pid
    sample_count = $validProcessSamples.Count
    peak_total_working_set_bytes = $peakWorkingSet
    peak_total_private_memory_bytes = if ($validProcessSamples.Count) { [long](@($validProcessSamples | Sort-Object total_private_memory_bytes -Descending)[0].total_private_memory_bytes) } else { 0L }
    peak_total_handle_count = if ($validProcessSamples.Count) { [int](@($validProcessSamples | Sort-Object total_handle_count -Descending)[0].total_handle_count) } else { 0 }
    peak_total_thread_count = if ($validProcessSamples.Count) { [int](@($validProcessSamples | Sort-Object total_thread_count -Descending)[0].total_thread_count) } else { 0 }
    peak_process_count = if ($validProcessSamples.Count) { [int](@($validProcessSamples | ForEach-Object { @($_.processes).Count } | Sort-Object -Descending)[0]) } else { 0 }
    peak_working_set_under_2_gib = $peakWorkingSet -gt 0 -and $peakWorkingSet -lt 2147483648
    zero_other_descendants = $zeroOtherDescendants
    process_command_lines_persisted = $processCommandLinesPersisted
    process_identity_stable = $processIdentityStable
    observed_processes = $observedProcesses
    direct_webview_browser_child_count = $directBrowserIdentityKeys.Count
    direct_webview_browser_child = $directBrowser
    direct_webview_browser_observed_in_every_sample = $directBrowserObservedInEverySample
    frozen_product_browser_switches = $frozenProductBrowserSwitches
    acceptance_only_browser_switches = $acceptanceOnlyBrowserSwitches
    missing_browser_switches = $missingBrowserSwitches
    duplicate_browser_switches = $duplicateBrowserSwitches
    conflicting_browser_switches = $conflictingBrowserSwitches
    browser_switch_contract_passed = $browserSwitchContractPassed
    observation = "sampled exact root process identity, executable path, command line, and descendants with a configured 250 ms inter-sample delay; bounded run, not a sustained no-leak claim"
}
Write-Utf8Json -Path $processReportPath -Value $processReport -Depth 10

$networkSamples = @()
if (Test-Path -LiteralPath $networkSamplesPath -PathType Leaf) {
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $networkSamplesPath -Encoding UTF8) {
        $lineNumber += 1
        if ([string]::IsNullOrWhiteSpace($line)) { throw "Network sample JSONL contains a blank row at line $lineNumber." }
        try { $networkSamples += $line | ConvertFrom-Json } catch { throw "Network sample JSONL is malformed at line $lineNumber`: $($_.Exception.Message)" }
    }
}
$validNetworkSamples = @($networkSamples | Where-Object { $_.root_present -eq $true -and [int]$_.root_pid -eq [int]$cleanup.launched_pid })
$remoteConnections = @($validNetworkSamples | ForEach-Object { @($_.remote_connections) })
$observedConnections = @($validNetworkSamples | ForEach-Object { @($_.connections) })
$directBrowserPid = if ($directBrowser) { [int]$directBrowser.pid } else { 0 }
$proxyListenerRows = @($observedConnections | Where-Object {
    [int]$_.owning_process -eq [int]$cleanup.launched_pid -and [string]$_.local_address -eq "127.0.0.1" -and
    [int]$_.local_port -eq 17846 -and [string]$_.remote_address -eq "0.0.0.0" -and
    [int]$_.remote_port -eq 0 -and [string]$_.state -eq "Listen" -and $_.remote_access -eq $false
})
$cdpListenerRows = @($observedConnections | Where-Object {
    [int]$_.owning_process -eq $directBrowserPid -and [string]$_.local_address -eq "127.0.0.1" -and
    [int]$_.local_port -eq 9222 -and [string]$_.remote_address -eq "0.0.0.0" -and
    [int]$_.remote_port -eq 0 -and [string]$_.state -eq "Listen" -and $_.remote_access -eq $false
})
$proxyListenerSampleCount = @($validNetworkSamples | Where-Object {
    @($_.connections | Where-Object {
        [int]$_.owning_process -eq [int]$cleanup.launched_pid -and [string]$_.local_address -eq "127.0.0.1" -and
        [int]$_.local_port -eq 17846 -and [string]$_.remote_address -eq "0.0.0.0" -and
        [int]$_.remote_port -eq 0 -and [string]$_.state -eq "Listen" -and $_.remote_access -eq $false
    }).Count -eq 1
}).Count
$cdpListenerSampleCount = @($validNetworkSamples | Where-Object {
    @($_.connections | Where-Object {
        [int]$_.owning_process -eq $directBrowserPid -and [string]$_.local_address -eq "127.0.0.1" -and
        [int]$_.local_port -eq 9222 -and [string]$_.remote_address -eq "0.0.0.0" -and
        [int]$_.remote_port -eq 0 -and [string]$_.state -eq "Listen" -and $_.remote_access -eq $false
    }).Count -eq 1
}).Count
$unexpectedLoopbackConnections = @($observedConnections | Where-Object {
    $connection = $_
    $state = [string]$connection.state
    if ($connection.remote_access -eq $true) { return $false }
    if ($state -in @("Bound", "Closed")) {
        return -not (Test-UnspecifiedAddress -Address ([string]$connection.remote_address)) -or [int]$connection.remote_port -ne 0
    }
    $isProxyListener = [int]$connection.owning_process -eq [int]$cleanup.launched_pid -and
        [string]$connection.local_address -eq "127.0.0.1" -and [int]$connection.local_port -eq 17846 -and
        [string]$connection.remote_address -eq "0.0.0.0" -and [int]$connection.remote_port -eq 0 -and $state -eq "Listen"
    $isCdpListener = [int]$connection.owning_process -eq $directBrowserPid -and
        [string]$connection.local_address -eq "127.0.0.1" -and [int]$connection.local_port -eq 9222 -and
        [string]$connection.remote_address -eq "0.0.0.0" -and [int]$connection.remote_port -eq 0 -and $state -eq "Listen"
    $isAllowedLoopbackFlow = (Test-LoopbackAddress -Address ([string]$connection.local_address)) -and
        (Test-LoopbackAddress -Address ([string]$connection.remote_address)) -and
        (@(9222, 17846) -contains [int]$connection.local_port -or @(9222, 17846) -contains [int]$connection.remote_port)
    return -not ($isProxyListener -or $isCdpListener -or $isAllowedLoopbackFlow)
})
$allowedLoopbackConnections = @($observedConnections | Where-Object {
    $state = [string]$_.state
    if ($state -in @("Bound", "Closed") -or $_.remote_access -eq $true) { return $false }
    return ((Test-LoopbackAddress -Address ([string]$_.local_address)) -and (Test-LoopbackAddress -Address ([string]$_.remote_address)) -and
        (@(9222, 17846) -contains [int]$_.local_port -or @(9222, 17846) -contains [int]$_.remote_port)) -or
        ([int]$_.local_port -in @(9222, 17846) -and [string]$_.state -eq "Listen")
})
$proxyListenerPresentInEverySample = $validNetworkSamples.Count -gt 0 -and $proxyListenerSampleCount -eq $validNetworkSamples.Count
$cdpListenerPresentInEverySample = $validNetworkSamples.Count -gt 0 -and $cdpListenerSampleCount -eq $validNetworkSamples.Count
$networkPassed = $validNetworkSamples.Count -ge 4 -and $remoteConnections.Count -eq 0 -and
    $unexpectedLoopbackConnections.Count -eq 0 -and $proxyListenerPresentInEverySample -and
    $cdpListenerPresentInEverySample -and $browserSwitchContractPassed -and $cleanup.network_monitor.exit_code -eq 0
$networkReport = [ordered]@{
    schema_version = 1
    passed = $networkPassed
    root_pid = $cleanup.launched_pid
    sample_count = $validNetworkSamples.Count
    observed_tcp_connection_rows = $observedConnections.Count
    remote_connection_count = $remoteConnections.Count
    remote_connections = @($remoteConnections)
    allowed_loopback_ports = @(9222, 17846)
    loopback_allowed_for_cdp = $true
    loopback_allowed_for_proxy = $true
    allowed_loopback_connection_rows = $allowedLoopbackConnections.Count
    unexpected_loopback_connection_count = $unexpectedLoopbackConnections.Count
    unexpected_loopback_connections = @($unexpectedLoopbackConnections)
    proxy_listener = @($proxyListenerRows | Select-Object -First 1)[0]
    proxy_listener_sample_count = $proxyListenerSampleCount
    proxy_listener_present_in_every_sample = $proxyListenerPresentInEverySample
    cdp_listener = @($cdpListenerRows | Select-Object -First 1)[0]
    cdp_listener_sample_count = $cdpListenerSampleCount
    cdp_listener_present_in_every_sample = $cdpListenerPresentInEverySample
    exact_loopback_allowances = $unexpectedLoopbackConnections.Count -eq 0
    udp_observation_performed = $false
    packet_capture_performed = $false
    boundary = "sampled_exact_process_tree_tcp_only_no_udp_or_packet_capture"
    observation = "sampled exact-process-tree TCP endpoint snapshots with a configured 250 ms inter-sample delay; only loopback CDP 9222 and QuickPLS proxy 17846 active paths allowed; zero non-loopback rows required; no UDP or packet-capture claim"
}
Write-Utf8Json -Path $networkReportPath -Value $networkReport -Depth 10

if (Test-Path -LiteralPath $rawReportPath -PathType Leaf) {
    $raw = Get-Content -LiteralPath $rawReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $checks = [ordered]@{}
    foreach ($property in $raw.checks.PSObject.Properties) { $checks[$property.Name] = $property.Value }
    $checks.process_observation = $processReport
    $checks.network_observation = [ordered]@{
        passed = [bool]($networkReport.passed -and $raw.checks.browser_network_observation.passed)
        tcp_monitor = $networkReport
        browser_request_observation = $raw.checks.browser_network_observation
    }
    $checks.cleanup = $cleanup
    $artifacts = [ordered]@{}
    foreach ($property in $raw.artifacts.PSObject.Properties) { $artifacts[$property.Name] = $property.Value }
    $artifacts.raw_report = Get-ArtifactDescriptor -Path $rawReportPath
    $artifacts.process_samples = Get-ArtifactDescriptor -Path $processSamplesPath
    $artifacts.network_samples = Get-ArtifactDescriptor -Path $networkSamplesPath
    $artifacts.process_report = Get-ArtifactDescriptor -Path $processReportPath
    $artifacts.network_report = Get-ArtifactDescriptor -Path $networkReportPath
    $artifacts.cleanup_report = Get-ArtifactDescriptor -Path $cleanupReportPath
    $artifacts.build_receipt = Get-ArtifactDescriptor -Path $buildReceiptPath
    $artifacts.source_before = Get-ArtifactDescriptor -Path $sourceBeforePath
    $artifacts.source_evidence = Get-ArtifactDescriptor -Path $sourceEvidencePath
    $allChecksPassed = @($checks.GetEnumerator() | Where-Object { $_.Value.passed -ne $true }).Count -eq 0
    $final = [ordered]@{
        schema_version = "quickpls.diagnostic_bundle_packaged_acceptance.v1"
        kind = "quickpls3_packaged_diagnostic_bundle_v1_acceptance"
        passed = [bool]($raw.passed -and $nodeExitCode -eq 0 -and $allChecksPassed -and $cleanup.passed -and
            $sourceStableDuringGate -and $sourceFreshnessAfter.passed)
        generated_at_utc = [DateTime]::UtcNow.ToString("o")
        target = $raw.target
        runtime = $raw.runtime
        endpoint = $raw.endpoint
        generator = "validation/run_diagnostic_bundle_packaged_acceptance.ps1"
        source_generator = $raw.generator
        tested_product = $raw.tested_product
        source_artifacts = $sourceArtifacts
        checks = $checks
        artifacts = $artifacts
        browser_requests = @($raw.browser_requests)
        console_errors = @($raw.console_errors)
        failures = @($raw.failures)
        source_report = "validation/results/diagnostic_bundle_packaged_acceptance.raw.json"
    }
    Write-Utf8Json -Path $finalReportPath -Value $final -Depth 32
}

if (-not [string]::IsNullOrWhiteSpace($nodeError)) { throw $nodeError }
if (-not $sourceStableDuringGate) { throw "A product/build or gate-only source changed while packaged diagnostic acceptance was running." }
if (-not $sourceFreshnessAfter.passed) { throw "The tested desktop became older than a product/build source during packaged diagnostic acceptance." }
if (-not $cleanup.passed) { throw "Exact-PID packaged diagnostic cleanup failed: $($cleanup | ConvertTo-Json -Compress -Depth 8)" }
if (-not $processPassed) { throw "Packaged diagnostic process observation failed: $($processReport | ConvertTo-Json -Compress -Depth 8)" }
if (-not $networkPassed) { throw "Packaged diagnostic TCP observation failed: $($networkReport | ConvertTo-Json -Compress -Depth 8)" }
if (-not (Test-Path -LiteralPath $finalReportPath -PathType Leaf)) { throw "The final packaged diagnostic report was not produced." }
& $pythonExecutable .\validation\diagnostic_bundle_packaged_acceptance.py --report $finalReportPath
if ($LASTEXITCODE -ne 0) { throw "The packaged diagnostic report validator failed with exit code $LASTEXITCODE." }

Get-Item -LiteralPath $ZipPath, $rawReportPath, $finalReportPath, $processReportPath, $networkReportPath, $cleanupReportPath |
    Select-Object FullName, Length, LastWriteTime
