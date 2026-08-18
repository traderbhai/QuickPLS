param(
    [switch]$RunQualifiedAcceptance,
    [string]$ExportPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot "validation\methods\cbsem_bootstrap_v2.manifest.json"
$harnessPath = Join-Path $repositoryRoot "validation\v252_cbsem_bootstrap_v2_native_acceptance.mjs"
$desktopPath = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$cliPath = Join-Path $repositoryRoot "target\release\qpls.exe"

if (-not $RunQualifiedAcceptance) {
    throw "Future-only CB-SEM bootstrap v2 packaged acceptance is disabled. Source integration does not authorize GUI execution or promotion."
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$declaredState = [string]$manifest.qualification.declared_state
if ($declaredState -notin @("native_qualified", "release_qualified")) {
    throw "CB-SEM bootstrap v2 remains '$declaredState'; packaged acceptance cannot start before qualification governance explicitly advances it."
}
foreach ($required in @($harnessPath, $desktopPath, $cliPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required future acceptance artifact is absent: $required"
    }
}
if (Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue) {
    throw "Close every existing quickpls-desktop.exe instance before packaged acceptance."
}
if ([string]::IsNullOrWhiteSpace($ExportPath)) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
    $ExportPath = Join-Path $repositoryRoot "validation\results\v252-cbsem-bootstrap-v2-$stamp.xlsx"
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
$env:QUICKPLS_CLI_PATH = $cliPath
$env:QUICKPLS_ACCEPTANCE_SCOPE = "cbsem_bootstrap_v2"
$env:QUICKPLS_CBSEM_BOOTSTRAP_NATIVE_EXPORT_PATH = $ExportPath

$application = Start-Process -FilePath $desktopPath -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
try {
    node $harnessPath
    if ($LASTEXITCODE -ne 0) {
        throw "Future CB-SEM bootstrap v2 packaged acceptance failed with exit code $LASTEXITCODE."
    }
} finally {
    if ($application -and -not $application.HasExited) {
        Stop-Process -Id $application.Id -ErrorAction SilentlyContinue
        $null = $application.WaitForExit(10000)
    }
    Remove-Item Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_CBSEM_BOOTSTRAP_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
}
