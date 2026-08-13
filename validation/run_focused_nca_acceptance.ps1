param(
    [string]$Root = "D:\QuickPLS"
)

$ErrorActionPreference = "Stop"

$existing = Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue
if ($existing) {
    throw "Close existing QuickPLS processes before focused acceptance: $($existing.Id -join ', ')"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$report = Join-Path $Root "validation\results\v247_tauri_native_acceptance.json"
$backup = Join-Path $Root "validation\results\v247_tauri_native_acceptance.pre-nca-$stamp.json"
$exportPath = Join-Path $Root "validation\results\v247-native-nca-$stamp.xlsx"
$desktopPath = Join-Path $Root "target\release\quickpls-desktop.exe"
$cliPath = Join-Path $Root "target\release\qpls.exe"

Copy-Item -LiteralPath $report -Destination $backup

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
$env:QUICKPLS_CLI_PATH = $cliPath
$env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
$env:QUICKPLS_NCA_NATIVE_EXPORT_PATH = $exportPath
$env:QUICKPLS_ACCEPTANCE_SCOPE = "nca"

$app = $null
try {
    $app = Start-Process -FilePath $desktopPath -WorkingDirectory $Root -WindowStyle Normal -PassThru
    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt += 1) {
        try {
            $null = Invoke-RestMethod -Uri $env:QUICKPLS_CDP_ENDPOINT/json/version -TimeoutSec 1
            $ready = $true
            break
        }
        catch {
            Start-Sleep -Milliseconds 500
        }
    }
    if (-not $ready) {
        throw "QuickPLS WebView2 CDP did not open on port 9222."
    }

    & node (Join-Path $Root "validation\v247_tauri_native_acceptance.mjs")
    if ($LASTEXITCODE -ne 0) {
        throw "Focused packaged NCA acceptance failed with exit code $LASTEXITCODE."
    }

    [pscustomobject]@{
        HarnessExitCode = $LASTEXITCODE
        ExportPath = $exportPath
        BackupReport = $backup
        AppPid = $app.Id
    } | Format-List
}
finally {
    if ($app -and -not $app.HasExited) {
        $null = $app.CloseMainWindow()
        if (-not $app.WaitForExit(10000)) {
            Stop-Process -Id $app.Id
            $null = $app.WaitForExit(5000)
        }
    }
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
    Remove-Item Env:QUICKPLS_CDP_ENDPOINT -ErrorAction SilentlyContinue
    Remove-Item Env:QUICKPLS_CLI_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:QUICKPLS_PYTHON -ErrorAction SilentlyContinue
    Remove-Item Env:QUICKPLS_NCA_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:QUICKPLS_ACCEPTANCE_SCOPE -ErrorAction SilentlyContinue
}
