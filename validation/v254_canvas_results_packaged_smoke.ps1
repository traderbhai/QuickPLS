[CmdletBinding()]
param(
    [string]$DesktopPath = "",
    [string]$EvidenceDir = "",
    [string]$PythonPath = "",
    [int]$Port = 9234,
    [double]$MinimumFreeGiB = 20.0
)

$ErrorActionPreference = "Stop"
$repositoryRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Set-Location -LiteralPath $repositoryRoot

if ([string]::IsNullOrWhiteSpace($DesktopPath)) {
    $DesktopPath = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
}
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd_HHmmss")
    $EvidenceDir = Join-Path $repositoryRoot "validation\results\v254_canvas_results_packaged_smoke_$stamp"
}
if ([string]::IsNullOrWhiteSpace($PythonPath)) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}

$desktop = [IO.Path]::GetFullPath($DesktopPath)
$evidence = [IO.Path]::GetFullPath($EvidenceDir)
$python = [IO.Path]::GetFullPath($PythonPath)
$node = (Get-Command node -ErrorAction Stop).Source
$resultsRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$projectPath = Join-Path $evidence "quickpls-v254-canvas-results.qpls"
$driver = Join-Path $repositoryRoot "validation\v254_canvas_results_packaged_smoke.mjs"
$closeDriver = Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"
$endpoint = "http://127.0.0.1:$Port"

if (-not $evidence.StartsWith($resultsRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Evidence directory must remain below validation/results: $evidence"
}
foreach ($required in @($desktop, $driver, $closeDriver, $python, $node)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required packaged-smoke input is missing: $required"
    }
}
if (Test-Path -LiteralPath $evidence) {
    throw "Refusing to reuse packaged-smoke evidence directory: $evidence"
}

function Get-FreeSpaceSnapshot {
    param([Parameter(Mandatory = $true)][string]$Label)
    $snapshot = [ordered]@{ label = $Label; captured_at = [DateTime]::UtcNow.ToString("o"); drives = [ordered]@{} }
    foreach ($driveName in @("C", "D")) {
        $drive = Get-PSDrive -Name $driveName -PSProvider FileSystem -ErrorAction Stop
        $free = [math]::Round($drive.Free / 1GB, 3)
        if ($free -le $MinimumFreeGiB) {
            throw "Drive $driveName must retain more than $MinimumFreeGiB GiB; observed $free GiB."
        }
        $snapshot.drives[$driveName] = $free
    }
    return $snapshot
}

function Test-CdpReady {
    try {
        $null = Invoke-RestMethod -Uri "$endpoint/json/version" -TimeoutSec 1
        return $true
    } catch {
        return $false
    }
}

function Wait-Cdp {
    param([Parameter(Mandatory = $true)][bool]$Open, [int]$Seconds = 45)
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if ((Test-CdpReady) -eq $Open) { return }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "WebView2 CDP endpoint did not become $(if ($Open) { 'ready' } else { 'closed' }): $endpoint"
}

$launchedPids = [System.Collections.Generic.List[int]]::new()

function Start-IsolatedCandidate {
    $process = Start-Process -FilePath $desktop -WorkingDirectory $repositoryRoot -WindowStyle Hidden -PassThru
    $launchedPids.Add($process.Id)
    try {
        Wait-Cdp -Open $true
        return $process
    } catch {
        if (-not $process.HasExited) {
            # Safety boundary: terminate only the exact PID launched above.
            Stop-Process -Id $process.Id -Force
            $null = $process.WaitForExit(5000)
        }
        throw
    }
}

function Stop-IsolatedCandidate {
    param([Diagnostics.Process]$Process)
    if ($Process -and -not $Process.HasExited) {
        # The CDP endpoint was proven unused before launch, so this close request
        # belongs to this isolated instance. The fallback is still PID-scoped.
        & $node $closeDriver *> $null
        $null = $Process.WaitForExit(10000)
    }
    if ($Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force
        $null = $Process.WaitForExit(5000)
    }
    Wait-Cdp -Open $false -Seconds 15
}

$diskBefore = Get-FreeSpaceSnapshot -Label "before_packaged_smoke"
if (Test-CdpReady) {
    throw "Port $Port is already serving a CDP endpoint. Close that test instance or choose another isolated port."
}
if (@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals($desktop, [StringComparison]::OrdinalIgnoreCase)
}).Count -ne 0) {
    throw "The exact candidate executable is already running. The harness will not attach to or close it."
}

New-Item -ItemType Directory -Path $evidence | Out-Null
$priorBrowserArgs = [Environment]::GetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "Process")
$priorUserDataFolder = [Environment]::GetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", "Process")
$priorEndpoint = [Environment]::GetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", "Process")
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $evidence "webview2-profile"
$env:QUICKPLS_CDP_ENDPOINT = $endpoint
$application = $null
$runError = $null
try {
    $application = Start-IsolatedCandidate
    & $node $driver --phase execute --endpoint $endpoint --evidence-dir $evidence --project-path $projectPath --python $python
    if ($LASTEXITCODE -ne 0) { throw "Canvas/Results execute phase failed with exit code $LASTEXITCODE." }
    Stop-IsolatedCandidate -Process $application
    $application = $null

    $application = Start-IsolatedCandidate
    & $node $driver --phase reopen --endpoint $endpoint --evidence-dir $evidence --project-path $projectPath --python $python
    if ($LASTEXITCODE -ne 0) { throw "Canvas/Results reopen phase failed with exit code $LASTEXITCODE." }
} catch {
    $runError = $_
} finally {
    if ($application) {
        try { Stop-IsolatedCandidate -Process $application }
        catch { if (-not $runError) { $runError = $_ } }
    }
    [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $priorBrowserArgs, "Process")
    [Environment]::SetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", $priorUserDataFolder, "Process")
    [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $priorEndpoint, "Process")
}

$diskAfter = Get-FreeSpaceSnapshot -Label "after_packaged_smoke"
if ($runError) { throw $runError }

$reportPath = Join-Path $evidence "v254_canvas_results_packaged_smoke.json"
if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
    throw "The packaged-smoke report was not written: $reportPath"
}
$report = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($report.passed -ne $true -or $report.complete -ne $true -or $report.version -ne "2.54.0" -or @($report.failures).Count -ne 0) {
    throw "The Canvas/Results packaged-smoke report is incomplete or failed: $reportPath"
}
$expectedScreens = 10
if (@($report.screenshots).Count -ne $expectedScreens) {
    throw "Expected $expectedScreens packaged-smoke screenshots; found $(@($report.screenshots).Count)."
}
if (@($report.observations).Count -ne $expectedScreens) {
    throw "Expected one machine-readable observation per screenshot; found $(@($report.observations).Count)."
}
foreach ($observation in @($report.observations)) {
    foreach ($field in @("id", "phase", "area", "expected", "observed", "status", "severity", "screenshot")) {
        if ([string]::IsNullOrWhiteSpace([string]$observation.$field)) {
            throw "Observation '$($observation.id)' is missing required field '$field'."
        }
    }
    if ($observation.status -ne "passed") {
        throw "Observation '$($observation.id)' did not pass: $($observation.observed)"
    }
}
foreach ($relative in @($report.screenshots)) {
    $screenshot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot ([string]$relative)))
    if (-not (Test-Path -LiteralPath $screenshot -PathType Leaf)) {
        throw "Packaged-smoke screenshot is missing: $screenshot"
    }
}

[pscustomobject]@{
    passed = $true
    version = "2.54.0"
    executable = $desktop
    executable_sha256 = (Get-FileHash -LiteralPath $desktop -Algorithm SHA256).Hash.ToLowerInvariant()
    launched_pids = @($launchedPids)
    process_safety = "only isolated launched PIDs were eligible for close/termination"
    report = $reportPath
    report_sha256 = (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToLowerInvariant()
    project = $projectPath
    screenshots = @($report.screenshots).Count
    observations = @($report.observations).Count
    free_gib_before = $diskBefore.drives
    free_gib_after = $diskAfter.drives
} | ConvertTo-Json -Depth 8
