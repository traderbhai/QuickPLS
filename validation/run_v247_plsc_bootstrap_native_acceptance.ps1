param(
    [string]$ExportPath = "",
    [string]$NetworkSamplesPath = "",
    [string]$ReceiptPath = ""
)

$ErrorActionPreference = "Stop"
Import-Module Microsoft.PowerShell.Utility -MaximumVersion 5.1 -ErrorAction Stop
$repositoryRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Set-Location -LiteralPath $repositoryRoot
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$supervisorStartedUtc = [DateTime]::UtcNow
$cdpEndpoint = "http://127.0.0.1:9222"

function Resolve-NewResultPath {
    param(
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value,
        [Parameter(Mandatory = $true)][string]$DefaultName,
        [Parameter(Mandatory = $true)][string]$Extension,
        [Parameter(Mandatory = $true)][string]$Label
    )
    $candidate = if ([string]::IsNullOrWhiteSpace($Value)) {
        Join-Path $resultsRoot $DefaultName
    } else {
        $Value
    }
    $full = [System.IO.Path]::GetFullPath($candidate)
    $parent = [System.IO.Path]::GetFullPath((Split-Path -Parent $full))
    if ($parent -ne $resultsRoot) { throw "$Label must be a direct child of validation\results: $full" }
    if ([System.IO.Path]::GetExtension($full) -ne $Extension) { throw "$Label must use the $Extension extension: $full" }
    if (Test-Path -LiteralPath $full) { throw "$Label already exists; PLSc-bootstrap evidence is append-only: $full" }
    return $full
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$ExportPath = Resolve-NewResultPath `
    -Value $ExportPath `
    -DefaultName "v247-native-plsc-bootstrap-$stamp.xlsx" `
    -Extension ".xlsx" `
    -Label "PLSc-bootstrap XLSX target"
$NetworkSamplesPath = Resolve-NewResultPath `
    -Value $NetworkSamplesPath `
    -DefaultName "v247-native-plsc-bootstrap-network-$stamp.jsonl" `
    -Extension ".jsonl" `
    -Label "PLSc-bootstrap network sample target"
if ([string]::IsNullOrWhiteSpace($ReceiptPath)) {
    $ReceiptPath = Join-Path $resultsRoot "v247_plsc_bootstrap_scoped_native_acceptance_receipt_v1.json"
}
$ReceiptPath = Resolve-NewResultPath `
    -Value $ReceiptPath `
    -DefaultName "v247_plsc_bootstrap_scoped_native_acceptance_receipt_v1.json" `
    -Extension ".json" `
    -Label "PLSc-bootstrap supervisor receipt"

$monitorStopPath = [System.IO.Path]::ChangeExtension($NetworkSamplesPath, ".stop")
if (Test-Path -LiteralPath $monitorStopPath) { throw "PLSc-bootstrap network stop signal already exists: $monitorStopPath" }

$desktopExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"))
$cliExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\qpls.exe"))
$harnessPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\v247_tauri_native_acceptance.mjs"))
$closeHelperPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"))
$networkMonitorPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\monitor_quickpls_network.ps1"))
$reportPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results\v247_tauri_native_acceptance_plsc_bootstrap.json"))

foreach ($required in @($desktopExecutable, $cliExecutable, $harnessPath, $closeHelperPath, $networkMonitorPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "PLSc-bootstrap packaged acceptance requires the frozen release input: $required"
    }
}

function Get-QuickPlsDesktopProcesses {
    return @(Get-CimInstance Win32_Process -Filter "Name = 'quickpls-desktop.exe'" -ErrorAction SilentlyContinue)
}

function Test-CdpReady {
    try {
        $null = Invoke-RestMethod -Uri "$cdpEndpoint/json/version" -TimeoutSec 1
        return $true
    } catch {
        return $false
    }
}

function Wait-CdpClosed {
    param([int]$TimeoutMilliseconds = 5000)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (-not (Test-CdpReady)) { return $true }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return -not (Test-CdpReady)
}

function Get-TrackedProcessTree {
    param([int]$RootProcessId)
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $ids = New-Object 'System.Collections.Generic.HashSet[int]'
    $pending = New-Object System.Collections.Generic.Queue[int]
    $null = $ids.Add($RootProcessId)
    $pending.Enqueue($RootProcessId)
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($ids.Add($childId)) { $pending.Enqueue($childId) }
        }
    }
    return @($rows | Where-Object { $ids.Contains([int]$_.ProcessId) } | ForEach-Object {
        [pscustomobject]@{
            process_id = [int]$_.ProcessId
            parent_process_id = [int]$_.ParentProcessId
            name = [string]$_.Name
            executable_path = [string]$_.ExecutablePath
            creation_date = [string]$_.CreationDate
        }
    })
}

function Get-ArtifactDescriptor {
    param([string]$Path)
    $file = Get-Item -LiteralPath $Path -ErrorAction Stop
    if ($file.Length -le 0) { throw "Evidence artifact is empty: $($file.FullName)" }
    $relative = $file.FullName.Substring($repositoryRoot.Length).TrimStart('\').Replace('\', '/')
    return [pscustomobject]@{
        path = $relative
        size = [int64]$file.Length
        sha256 = (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

if ((Get-QuickPlsDesktopProcesses).Count -ne 0) {
    throw "Close every existing quickpls-desktop.exe instance before PLSc-bootstrap packaged acceptance."
}
if (@(Get-NetTCPConnection -State Listen -LocalPort 9222 -ErrorAction SilentlyContinue).Count -ne 0) {
    throw "PLSc-bootstrap acceptance requires TCP port 9222 to be unused before launch."
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = $cdpEndpoint
$env:QUICKPLS_CLI_PATH = $cliExecutable
$env:QUICKPLS_PYTHON = if (Test-Path -LiteralPath "C:\Python313\python.exe" -PathType Leaf) { "C:\Python313\python.exe" } else { "python" }
$env:QUICKPLS_ACCEPTANCE_SCOPE = "plsc_bootstrap"
$env:QUICKPLS_PLSC_BOOTSTRAP_NATIVE_EXPORT_PATH = $ExportPath

$application = $null
$monitor = $null
$trackedProcesses = @()
$primaryError = $null
$cleanupErrors = New-Object System.Collections.Generic.List[string]
$forcedProcessCleanupUsed = $false
try {
    $application = Start-Process -FilePath $desktopExecutable -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
    $monitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @(
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $networkMonitorPath,
            "-RootProcessId", [string]$application.Id,
            "-SamplesPath", $NetworkSamplesPath,
            "-StopSignalPath", $monitorStopPath,
            "-IntervalMilliseconds", "200"
        ) `
        -WorkingDirectory $repositoryRoot `
        -WindowStyle Hidden `
        -PassThru

    $cdpReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if (Test-CdpReady) { $cdpReady = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $cdpReady) { throw "QuickPLS WebView2 CDP did not open on the dedicated loopback endpoint." }

    & node $harnessPath
    if ($LASTEXITCODE -ne 0) { throw "Focused packaged PLSc-bootstrap acceptance failed with exit code $LASTEXITCODE." }
    $trackedProcesses = @(Get-TrackedProcessTree -RootProcessId $application.Id)
} catch {
    $primaryError = $_
} finally {
    try {
        [System.IO.File]::WriteAllText($monitorStopPath, "stop", $utf8WithoutBom)
        if ($monitor -and -not $monitor.HasExited -and -not $monitor.WaitForExit(15000)) {
            Stop-Process -Id $monitor.Id -Force -ErrorAction SilentlyContinue
            $forcedProcessCleanupUsed = $true
            $cleanupErrors.Add("The exact PLSc-bootstrap network monitor required forced cleanup.")
        }
        if ($monitor -and $monitor.HasExited -and $monitor.ExitCode -ne 0) {
            $cleanupErrors.Add("The PLSc-bootstrap network monitor exited with code $($monitor.ExitCode).")
        }
    } catch {
        $cleanupErrors.Add("PLSc-bootstrap network monitor cleanup failed: $($_.Exception.Message)")
    }
    try {
        if ($application -and -not $application.HasExited) {
            $previousErrorActionPreference = $ErrorActionPreference
            $ErrorActionPreference = "SilentlyContinue"
            & node $closeHelperPath 2>$null | Out-Null
            $ErrorActionPreference = $previousErrorActionPreference
            if (-not $application.WaitForExit(10000)) {
                Stop-Process -Id $application.Id -Force -ErrorAction SilentlyContinue
                $null = $application.WaitForExit(10000)
                $forcedProcessCleanupUsed = $true
                $cleanupErrors.Add("The exact PLSc-bootstrap desktop process required forced cleanup.")
            }
        }
    } catch {
        $cleanupErrors.Add("PLSc-bootstrap desktop cleanup failed: $($_.Exception.Message)")
    }
    $cdpClosed = Wait-CdpClosed
    $remainingDesktop = @(Get-QuickPlsDesktopProcesses)
    $liveIds = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.ProcessId })
    $lingeringTracked = @($trackedProcesses | Where-Object { $liveIds -contains [int]$_.process_id })
    if (-not $cdpClosed -or $remainingDesktop.Count -ne 0 -or $lingeringTracked.Count -ne 0) {
        $cleanupErrors.Add("PLSc-bootstrap left its CDP endpoint, desktop, or tracked child process alive.")
    }
    Remove-Item -LiteralPath $monitorStopPath -Force -ErrorAction SilentlyContinue
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,Env:QUICKPLS_CDP_ENDPOINT,Env:QUICKPLS_CLI_PATH,Env:QUICKPLS_PYTHON,Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_PLSC_BOOTSTRAP_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
}

if ($primaryError) { throw $primaryError }
if ($cleanupErrors.Count -ne 0) { throw ($cleanupErrors -join " ") }
foreach ($artifact in @($reportPath, $ExportPath, $NetworkSamplesPath)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf) -or (Get-Item -LiteralPath $artifact).Length -le 0) {
        throw "PLSc-bootstrap packaged acceptance did not create non-empty evidence: $artifact"
    }
}

$report = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
$expectedCheckIds = @(
    "plscBootstrapCancellation", "plscBootstrapDialog", "plscBootstrapExport",
    "plscBootstrapFixture", "plscBootstrapFixtureProvisioning", "plscBootstrapFunctionalOffline",
    "plscBootstrapInitialModel", "plscBootstrapInvalidSetup", "plscBootstrapPackagedViewports",
    "plscBootstrapProgress", "plscBootstrapResult", "plscBootstrapSaveReopen",
    "recentProjectsRestored", "runtime", "runtimePreflight"
) | Sort-Object
$checkIds = @($report.checks.PSObject.Properties | ForEach-Object { [string]$_.Name } | Sort-Object)
$offline = $report.checks.plscBootstrapFunctionalOffline
$viewports = @($report.checks.plscBootstrapPackagedViewports.exactViewports | ForEach-Object { [string]$_.id })
if ($report.passed -ne $true -or @($report.failures).Count -ne 0 -or @($report.consoleErrors).Count -ne 0 `
    -or [string]$report.runtime -ne "tauri-webview2-cdp" -or [string]$report.acceptance_scope -ne "plsc_bootstrap" `
    -or [string]$report.focusedRun.scope -ne "plsc_bootstrap" `
    -or (Compare-Object -ReferenceObject $expectedCheckIds -DifferenceObject $checkIds).Count -ne 0 `
    -or $offline.passed -ne $true -or [int]$offline.externalRequestCount -ne 0 `
    -or ($viewports -join ",") -ne "1024x700,1280x720,1440x900") {
    throw "Focused PLSc-bootstrap report is incomplete, impure, non-offline, or missing its exact viewport/check contract."
}

$networkRows = @(Get-Content -LiteralPath $NetworkSamplesPath -Encoding UTF8 | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
$remoteConnections = @($networkRows | ForEach-Object { @($_.remote_connections) })
if ($networkRows.Count -eq 0 `
    -or @($networkRows | Where-Object {
        $_.root_present -ne $true `
        -or [string]$_.observation -ne "sampled_exact_process_tree_tcp_v1" `
        -or $null -eq $_.PSObject.Properties["remote_connections"] `
        -or $null -eq $_.PSObject.Properties["remote_connections"].Value
    }).Count -ne 0) {
    throw "PLSc-bootstrap process-tree network observation is empty, malformed, or incomplete."
}
$platformBackgroundEgressObserved = $remoteConnections.Count -gt 0
$platformBackgroundEgressObservation = [pscustomobject]@{
    passed = $true
    observation_kind = "sampled_exact_process_tree_tcp_v1"
    sample_count = $networkRows.Count
    root_present_every_sample = $true
    platform_background_egress_observed = $platformBackgroundEgressObserved
    commercial_zero_egress_passed = -not $platformBackgroundEgressObserved
    remote_connections = @($remoteConnections)
}

$projectPath = [string]$report.checks.plscBootstrapFixture.projectPath
if (-not (Test-Path -LiteralPath $projectPath -PathType Leaf)) { throw "PLSc-bootstrap project archive is missing: $projectPath" }
$screenshots = @($report.screenshots | ForEach-Object { Get-ArtifactDescriptor -Path ([string]$_) })
if ($screenshots.Count -lt 4) { throw "PLSc-bootstrap report did not bind its lifecycle and three viewport screenshots." }

$receipt = [pscustomobject]@{
    schema_version = 1
    kind = "quickpls_v247_plsc_bootstrap_scoped_native_acceptance_receipt"
    passed = $true
    supervisor_started_at_utc = $supervisorStartedUtc.ToString("o")
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
    scope = "plsc_bootstrap"
    feature_id = "qpls3.inference.consistent_bootstrap"
    method_version = "plsc_bootstrap_v1"
    report = Get-ArtifactDescriptor -Path $reportPath
    executable = Get-ArtifactDescriptor -Path $desktopExecutable
    cli = Get-ArtifactDescriptor -Path $cliExecutable
    export = Get-ArtifactDescriptor -Path $ExportPath
    project_archive = Get-ArtifactDescriptor -Path $projectPath
    network_samples = Get-ArtifactDescriptor -Path $NetworkSamplesPath
    screenshots = $screenshots
    checks = $checkIds.Count
    unique_checks = @($checkIds | Sort-Object -Unique).Count
    check_ids = $checkIds
    failures = 0
    console_errors = 0
    runtime = "tauri-webview2-cdp"
    cdp_endpoint = $cdpEndpoint
    cdp_loopback_only = $true
    functional_offline = $offline
    platform_background_egress_observation = $platformBackgroundEgressObservation
    sampled_process_tree_zero_egress = -not $platformBackgroundEgressObserved
    network_sample_count = $networkRows.Count
    observed_process_tree = $trackedProcesses
    graceful_process_cleanup_verified = $true
    forced_process_cleanup_used = $forcedProcessCleanupUsed
    orphan_processes = 0
}
[System.IO.File]::WriteAllText(
    $ReceiptPath,
    (($receipt | ConvertTo-Json -Depth 14) + [Environment]::NewLine),
    $utf8WithoutBom
)
$receipt | ConvertTo-Json -Depth 14
