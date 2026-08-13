param(
    [string]$ExportPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

$existingProcess = Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue
if ($existingProcess) {
    throw "Close every existing quickpls-desktop.exe instance before packaged acceptance."
}

if ([string]::IsNullOrWhiteSpace($ExportPath)) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
    $ExportPath = Join-Path $repositoryRoot "validation\results\v247-native-pca-$stamp.xlsx"
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
$env:QUICKPLS_CLI_PATH = Join-Path $repositoryRoot "target\release\qpls.exe"
$env:QUICKPLS_PYTHON = "C:\Python313\python.exe"
$env:QUICKPLS_ACCEPTANCE_SCOPE = "pca"
$env:QUICKPLS_PCA_NATIVE_EXPORT_PATH = $ExportPath

$application = Start-Process `
    -FilePath (Join-Path $repositoryRoot "target\release\quickpls-desktop.exe") `
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
        throw "Focused packaged principal component analysis acceptance failed with exit code $LASTEXITCODE."
    }

    Get-Item -LiteralPath $ExportPath | Select-Object FullName, Length, LastWriteTime
} finally {
    if ($application -and -not $application.HasExited) {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "SilentlyContinue"
        & node .\validation\close_tauri_test_window.mjs 2>$null | Out-Null
        $ErrorActionPreference = $previousErrorActionPreference
        $null = $application.WaitForExit(10000)
    }
    Remove-Item Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_PCA_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
}
