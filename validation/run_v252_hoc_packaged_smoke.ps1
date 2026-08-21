param(
    [string]$DesktopPath = "",
    [string]$EvidenceDir = "",
    [string]$PythonPath = "",
    [int]$Port = 9222
)

$ErrorActionPreference = "Stop"
$repositoryRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Set-Location -LiteralPath $repositoryRoot

if ([string]::IsNullOrWhiteSpace($DesktopPath)) {
    $DesktopPath = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
}
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $EvidenceDir = Join-Path $repositoryRoot "validation\results\v252_hoc_packaged_smoke_20260821_111034"
}
if ([string]::IsNullOrWhiteSpace($PythonPath)) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}

$desktop = [IO.Path]::GetFullPath($DesktopPath)
$evidence = [IO.Path]::GetFullPath($EvidenceDir)
$resultsRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$projectPath = Join-Path $evidence "quickpls-v252-hoc.qpls"
$driver = Join-Path $repositoryRoot "validation\v252_hoc_packaged_smoke.mjs"
$closeDriver = Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"
$endpoint = "http://127.0.0.1:$Port"

if (-not $evidence.StartsWith($resultsRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Evidence directory must remain below validation/results: $evidence"
}
foreach ($required in @($desktop, $driver, $closeDriver, $PythonPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required packaged-smoke input is missing: $required"
    }
}
if (Test-Path -LiteralPath $evidence) {
    throw "Refusing to reuse packaged-smoke evidence directory: $evidence"
}

function Test-CdpReady {
    try {
        $null = Invoke-RestMethod -Uri "$endpoint/json/version" -TimeoutSec 1
        return $true
    } catch {
        return $false
    }
}

function Wait-Cdp([bool]$Open, [int]$Seconds = 45) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if ((Test-CdpReady) -eq $Open) { return }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "WebView2 CDP endpoint did not become $(if ($Open) { 'ready' } else { 'closed' }): $endpoint"
}

function Start-Candidate {
    $process = Start-Process -FilePath $desktop -WorkingDirectory $repositoryRoot -WindowStyle Hidden -PassThru
    try {
        Wait-Cdp $true
        return $process
    } catch {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
        throw
    }
}

function Stop-Candidate([Diagnostics.Process]$Process) {
    if ($Process -and -not $Process.HasExited) {
        & node $closeDriver *> $null
        $null = $Process.WaitForExit(10000)
    }
    if ($Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force
        $null = $Process.WaitForExit(5000)
    }
    Wait-Cdp $false 15
}

if (Test-CdpReady) {
    throw "Port $Port is already serving a CDP endpoint. Close that test instance first."
}
if (@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals($desktop, [StringComparison]::OrdinalIgnoreCase)
}).Count -ne 0) {
    throw "The exact 2.52 candidate is already running."
}

New-Item -ItemType Directory -Path $evidence | Out-Null
$priorBrowserArgs = [Environment]::GetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "Process")
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
$application = $null
try {
    $application = Start-Candidate
    & node $driver --phase execute --endpoint $endpoint --evidence-dir $evidence --project-path $projectPath --python $PythonPath
    if ($LASTEXITCODE -ne 0) { throw "HOC execute phase failed with exit code $LASTEXITCODE." }
    Stop-Candidate $application
    $application = $null

    $application = Start-Candidate
    & node $driver --phase reopen --endpoint $endpoint --evidence-dir $evidence --project-path $projectPath --python $PythonPath
    if ($LASTEXITCODE -ne 0) { throw "HOC reopen phase failed with exit code $LASTEXITCODE." }
} finally {
    if ($application) { Stop-Candidate $application }
    [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $priorBrowserArgs, "Process")
}

$reportPath = Join-Path $evidence "v252_hoc_packaged_smoke.json"
$report = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($report.passed -ne $true -or $report.complete -ne $true -or @($report.failures).Count -ne 0) {
    throw "The HOC packaged-smoke report is incomplete or failed: $reportPath"
}
if (@($report.screenshots).Count -ne 6) {
    throw "Expected six packaged-smoke screenshots; found $(@($report.screenshots).Count)."
}
foreach ($relative in @($report.screenshots)) {
    $screenshot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot ([string]$relative)))
    if (-not (Test-Path -LiteralPath $screenshot -PathType Leaf)) {
        throw "Packaged-smoke screenshot is missing: $screenshot"
    }
}

[pscustomobject]@{
    passed = $true
    version = "2.52.0"
    executable = $desktop
    executable_sha256 = (Get-FileHash -LiteralPath $desktop -Algorithm SHA256).Hash
    report = $reportPath
    project = $projectPath
    screenshots = @($report.screenshots).Count
    c_free_gib = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    d_free_gib = [math]::Round((Get-PSDrive D).Free / 1GB, 2)
} | ConvertTo-Json -Depth 4
