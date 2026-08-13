param(
    [string]$ExportPath = "",
    [string]$NetworkSamplesPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)

function Resolve-NewResultPath {
    param(
        [Parameter(Mandatory = $true)][string]$Value,
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
    if ($parent -ne $resultsRoot) {
        throw "$Label must be a direct child of validation\results: $full"
    }
    if ([System.IO.Path]::GetExtension($full) -ne $Extension) {
        throw "$Label must use the $Extension extension: $full"
    }
    if (Test-Path -LiteralPath $full) {
        throw "$Label already exists; CTA-PLS packaged acceptance never overwrites evidence: $full"
    }
    return $full
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$ExportPath = Resolve-NewResultPath `
    -Value $ExportPath `
    -DefaultName "v247-native-cta-pls-$stamp.xlsx" `
    -Extension ".xlsx" `
    -Label "CTA-PLS XLSX target"
$NetworkSamplesPath = Resolve-NewResultPath `
    -Value $NetworkSamplesPath `
    -DefaultName "v247-native-cta-pls-network-$stamp.jsonl" `
    -Extension ".jsonl" `
    -Label "CTA-PLS network sample target"
$monitorStopPath = [System.IO.Path]::ChangeExtension($NetworkSamplesPath, ".stop")
if (Test-Path -LiteralPath $monitorStopPath) {
    throw "CTA-PLS network monitor stop signal already exists: $monitorStopPath"
}

$desktopExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"))
$cliExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\qpls.exe"))
foreach ($required in @($desktopExecutable, $cliExecutable, (Join-Path $repositoryRoot "validation\monitor_quickpls_network.ps1"))) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "CTA-PLS packaged acceptance requires the frozen release artifact: $required"
    }
}

$existingProcess = @(Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue)
if ($existingProcess.Count -ne 0) {
    throw "Close every existing quickpls-desktop.exe instance before CTA-PLS packaged acceptance."
}
$cdpListeners = @(Get-NetTCPConnection -State Listen -LocalPort 9222 -ErrorAction SilentlyContinue)
if ($cdpListeners.Count -ne 0) {
    throw "TCP port 9222 is already in use; CTA-PLS acceptance will not attach to an ambiguous CDP endpoint."
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = "http://127.0.0.1:9222"
$env:QUICKPLS_CLI_PATH = $cliExecutable
$env:QUICKPLS_PYTHON = if (Test-Path -LiteralPath "C:\Python313\python.exe" -PathType Leaf) { "C:\Python313\python.exe" } else { "python" }
$env:QUICKPLS_ACCEPTANCE_SCOPE = "cta_pls"
$env:QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH = $ExportPath

$application = $null
$monitor = $null
$primaryError = $null
$cleanupErrors = New-Object System.Collections.Generic.List[string]
try {
    $application = Start-Process `
        -FilePath $desktopExecutable `
        -WorkingDirectory $repositoryRoot `
        -WindowStyle Normal `
        -PassThru

    $monitor = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", (Join-Path $repositoryRoot "validation\monitor_quickpls_network.ps1"),
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

    & node .\validation\v247_tauri_native_acceptance.mjs
    if ($LASTEXITCODE -ne 0) {
        throw "Focused packaged CTA-PLS acceptance failed with exit code $LASTEXITCODE."
    }
    foreach ($artifact in @($ExportPath, $NetworkSamplesPath)) {
        if (-not (Test-Path -LiteralPath $artifact -PathType Leaf) -or (Get-Item -LiteralPath $artifact).Length -le 0) {
            throw "CTA-PLS packaged acceptance did not create non-empty evidence: $artifact"
        }
    }
} catch {
    $primaryError = $_
} finally {
    try {
        [System.IO.File]::WriteAllText($monitorStopPath, "stop", $utf8WithoutBom)
        if ($monitor -and -not $monitor.HasExited) {
            if (-not $monitor.WaitForExit(15000)) {
                Stop-Process -Id $monitor.Id -Force -ErrorAction SilentlyContinue
                $cleanupErrors.Add("The exact CTA-PLS network monitor required forced cleanup.")
            }
        }
        if ($monitor -and $monitor.HasExited -and $monitor.ExitCode -ne 0) {
            $cleanupErrors.Add("The CTA-PLS network monitor exited with code $($monitor.ExitCode).")
        }
    } catch {
        $cleanupErrors.Add("CTA-PLS network monitor cleanup failed: $($_.Exception.Message)")
    }
    try {
        if ($application -and -not $application.HasExited) {
            $previousPreference = $ErrorActionPreference
            $ErrorActionPreference = "SilentlyContinue"
            & node .\validation\close_tauri_test_window.mjs 2>$null | Out-Null
            $ErrorActionPreference = $previousPreference
            if (-not $application.WaitForExit(10000)) {
                Stop-Process -Id $application.Id -Force -ErrorAction SilentlyContinue
                $null = $application.WaitForExit(10000)
                $cleanupErrors.Add("The exact CTA-PLS desktop process required forced cleanup.")
            }
        }
        $lingering = @(Get-Process -Name "quickpls-desktop" -ErrorAction SilentlyContinue)
        if ($lingering.Count -ne 0) {
            $cleanupErrors.Add("A quickpls-desktop.exe process remained after CTA-PLS acceptance.")
        }
    } catch {
        $cleanupErrors.Add("CTA-PLS desktop cleanup failed: $($_.Exception.Message)")
    }
    Remove-Item -LiteralPath $monitorStopPath -Force -ErrorAction SilentlyContinue
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,Env:QUICKPLS_CDP_ENDPOINT,Env:QUICKPLS_CLI_PATH,Env:QUICKPLS_PYTHON,Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH -ErrorAction SilentlyContinue
}

if ($primaryError) { throw $primaryError }
if ($cleanupErrors.Count -ne 0) { throw ($cleanupErrors -join " ") }
Get-Item -LiteralPath $ExportPath, $NetworkSamplesPath | Select-Object FullName, Length, LastWriteTimeUtc
