param(
    [string]$OlsExportPath = "",
    [string]$LogisticExportPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

$existingProcess = Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue
if ($existingProcess) {
    throw "Close every existing quickpls-desktop.exe instance before packaged acceptance."
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
if ([string]::IsNullOrWhiteSpace($OlsExportPath)) {
    $OlsExportPath = Join-Path $repositoryRoot "validation\results\v247-native-regression-bootstrap-ols-$stamp.xlsx"
}
if ([string]::IsNullOrWhiteSpace($LogisticExportPath)) {
    $LogisticExportPath = Join-Path $repositoryRoot "validation\results\v247-native-regression-bootstrap-logistic-$stamp.xlsx"
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
$env:QUICKPLS_CLI_PATH = Join-Path $repositoryRoot "target\release\qpls.exe"
$env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
$env:QUICKPLS_ACCEPTANCE_SCOPE = "regression_bootstrap"
$env:QUICKPLS_REGRESSION_BOOTSTRAP_OLS_EXPORT_PATH = $OlsExportPath
$env:QUICKPLS_REGRESSION_BOOTSTRAP_LOGISTIC_EXPORT_PATH = $LogisticExportPath
$desktopExecutable = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$env:QUICKPLS_DESKTOP_EXE_PATH = $desktopExecutable
$cleanupReportPath = Join-Path $repositoryRoot "validation\results\v247_regression_bootstrap_process_cleanup.json"

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

$application = Start-Process `
    -FilePath $desktopExecutable `
    -WorkingDirectory $repositoryRoot `
    -WindowStyle Normal `
    -PassThru

try {
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
        throw "Focused packaged regression bootstrap acceptance failed with exit code $LASTEXITCODE."
    }

    Get-Item -LiteralPath $OlsExportPath | Select-Object FullName, Length, LastWriteTime
    Get-Item -LiteralPath $LogisticExportPath | Select-Object FullName, Length, LastWriteTime
    Get-Item -LiteralPath (Join-Path $repositoryRoot "validation\results\regression_bootstrap_v1_packaged_acceptance.json") |
        Select-Object FullName, Length, LastWriteTime
} finally {
    $cleanup = [ordered]@{
        launched_pid = if ($application) { $application.Id } else { $null }
        descendants_at_shutdown = @()
        graceful_close_exit_code = $null
        graceful_exit_confirmed = $false
        forced_parent_termination = $false
        forced_descendant_pids = @()
        parent_exit_confirmed = $false
        lingering_descendant_pids = @()
        passed = $false
    }
    if ($application) {
        $cleanup.descendants_at_shutdown = @(Get-ExactDescendantProcesses -RootProcessId $application.Id)
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
    $cleanup.passed = $cleanup.parent_exit_confirmed -and $cleanup.lingering_descendant_pids.Count -eq 0
    $cleanup | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $cleanupReportPath -Encoding UTF8
    Remove-Item `
        Env:QUICKPLS_ACCEPTANCE_SCOPE, `
        Env:QUICKPLS_REGRESSION_BOOTSTRAP_OLS_EXPORT_PATH, `
        Env:QUICKPLS_REGRESSION_BOOTSTRAP_LOGISTIC_EXPORT_PATH, `
        Env:QUICKPLS_DESKTOP_EXE_PATH `
        -ErrorAction SilentlyContinue
    if (-not $cleanup.passed) {
        throw "QuickPLS exact-PID cleanup failed: $($cleanup | ConvertTo-Json -Compress -Depth 5)"
    }
}
