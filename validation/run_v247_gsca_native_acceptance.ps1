param(
    [string]$ExportPath = "",
    [string]$ReceiptPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Set-Location -LiteralPath $repositoryRoot

$desktopExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"))
$harnessPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\v247_tauri_native_acceptance.mjs"))
$closeHelperPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"))
$reportPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results\v247_tauri_native_acceptance_gsca.json"))
$cdpEndpoint = "http://127.0.0.1:9222"
$supervisorStartedUtc = [DateTime]::UtcNow

if ([string]::IsNullOrWhiteSpace($ExportPath)) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
    $ExportPath = Join-Path $repositoryRoot "validation\results\v247-native-gsca-$stamp.xlsx"
}
$ExportPath = [System.IO.Path]::GetFullPath($ExportPath)
if ([string]::IsNullOrWhiteSpace($ReceiptPath)) {
    $ReceiptPath = Join-Path $repositoryRoot "validation\results\v247_gsca_scoped_native_acceptance_receipt_v2.json"
}
$ReceiptPath = [System.IO.Path]::GetFullPath($ReceiptPath)

foreach ($requiredFile in @($desktopExecutable, $harnessPath, $closeHelperPath)) {
    if (-not (Test-Path -LiteralPath $requiredFile -PathType Leaf)) {
        throw "Required focused GSCA acceptance input is missing: $requiredFile"
    }
}
if (Test-Path -LiteralPath $ReceiptPath) {
    throw "Refusing to overwrite the append-only focused GSCA receipt: $ReceiptPath"
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
    $relative = $file.FullName.Substring($repositoryRoot.Length).TrimStart('\').Replace('\', '/')
    return [pscustomobject]@{
        path = $relative
        size = [int64]$file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

if ((Get-QuickPlsDesktopProcesses).Count -ne 0) {
    throw "Close every existing quickpls-desktop.exe instance before packaged acceptance."
}
if (Test-CdpReady) {
    throw "Focused GSCA acceptance requires the dedicated loopback CDP endpoint to be closed before launch."
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = $cdpEndpoint
$env:QUICKPLS_CLI_PATH = Join-Path $repositoryRoot "target\release\qpls.exe"
$env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
$env:QUICKPLS_ACCEPTANCE_SCOPE = "gsca"
$env:QUICKPLS_GSCA_NATIVE_EXPORT_PATH = $ExportPath

$application = $null
$trackedProcesses = @()
$cleanupVerified = $false
try {
    $application = Start-Process -FilePath $desktopExecutable -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
    $cdpReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if (Test-CdpReady) { $cdpReady = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $cdpReady) {
        throw "QuickPLS WebView2 CDP did not open on the dedicated loopback endpoint."
    }

    node $harnessPath
    if ($LASTEXITCODE -ne 0) {
        throw "Focused packaged GSCA acceptance failed with exit code $LASTEXITCODE."
    }
    $trackedProcesses = @(Get-TrackedProcessTree -RootProcessId $application.Id)
} finally {
    if ($application -and -not $application.HasExited) {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "SilentlyContinue"
        & node $closeHelperPath 2>$null | Out-Null
        $ErrorActionPreference = $previousErrorActionPreference
        $null = $application.WaitForExit(10000)
    }
    $cdpClosed = Wait-CdpClosed
    $remainingDesktop = @(Get-QuickPlsDesktopProcesses)
    $liveIds = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.ProcessId })
    $lingeringTracked = @($trackedProcesses | Where-Object { $liveIds -contains [int]$_.process_id })
    $cleanupVerified = $application -and $application.HasExited -and $cdpClosed -and $remainingDesktop.Count -eq 0 -and $lingeringTracked.Count -eq 0
    Remove-Item Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_GSCA_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
}

if (-not $cleanupVerified) {
    throw "Focused GSCA acceptance did not close its exact process tree and loopback CDP endpoint cleanly."
}
if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
    throw "Focused GSCA acceptance did not publish its scoped report."
}
$report = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
$checkRows = @($report.checks.PSObject.Properties)
$checkNames = @($checkRows | ForEach-Object { [string]$_.Name })
$offline = $report.checks.gscaFunctionalOffline
if ($report.passed -ne $true -or @($report.failures).Count -ne 0 -or @($report.consoleErrors).Count -ne 0 `
    -or [string]$report.runtime -ne "tauri-webview2-cdp" -or [string]$report.focusedRun.scope -ne "gsca" `
    -or $checkNames.Count -ne 15 -or @($checkNames | Sort-Object -Unique).Count -ne 15 `
    -or $offline.passed -ne $true -or [int]$offline.externalRequestCount -ne 0 `
    -or $offline.analyticalWorkflowRequiresInternet -ne $false -or $offline.strictZeroProcessEgressClaimed -ne $false) {
    throw "Focused GSCA scoped report is incomplete, impure, or not offline within the observed browser/app request boundary."
}
if (-not (Test-Path -LiteralPath $ExportPath -PathType Leaf)) {
    throw "Focused GSCA XLSX export is missing: $ExportPath"
}
$projectPath = [string]$report.checks.gscaFixture.projectPath
if (-not (Test-Path -LiteralPath $projectPath -PathType Leaf)) {
    throw "Focused GSCA project archive is missing: $projectPath"
}
$screenshots = @($report.screenshots | ForEach-Object { Get-ArtifactDescriptor -Path ([string]$_) })
if ($screenshots.Count -ne 11) {
    throw "Focused GSCA report must bind exactly 11 screenshot artifacts; found $($screenshots.Count)."
}

$receipt = [pscustomobject]@{
    schema_version = 1
    kind = "quickpls_v247_gsca_scoped_native_acceptance_receipt"
    passed = $true
    supervisor_started_at_utc = $supervisorStartedUtc.ToString("o")
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
    scope = "gsca"
    feature_id = "qpls3.gsca.als"
    method_version = "gsca_als_v2"
    report = Get-ArtifactDescriptor -Path $reportPath
    executable = Get-ArtifactDescriptor -Path $desktopExecutable
    export = Get-ArtifactDescriptor -Path $ExportPath
    project_archive = Get-ArtifactDescriptor -Path $projectPath
    screenshots = $screenshots
    checks = $checkNames.Count
    unique_checks = @($checkNames | Sort-Object -Unique).Count
    check_ids = @($checkNames | Sort-Object)
    failures = 0
    console_errors = 0
    runtime = "tauri-webview2-cdp"
    cdp_endpoint = $cdpEndpoint
    cdp_loopback_only = $true
    functional_offline = $offline
    observed_process_tree = $trackedProcesses
    graceful_process_cleanup_verified = $cleanupVerified
    forced_process_cleanup_used = $false
    orphan_processes = 0
}
$serialized = $receipt | ConvertTo-Json -Depth 12
[System.IO.File]::WriteAllText($ReceiptPath, $serialized + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
$receipt | ConvertTo-Json -Depth 12
