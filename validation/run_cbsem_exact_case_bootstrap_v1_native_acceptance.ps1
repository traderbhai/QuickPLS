param(
    [switch]$RunQualifiedAcceptance,
    [string]$PackagedReportPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
Set-Location -LiteralPath $repositoryRoot
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$cdpEndpoint = "http://127.0.0.1:9222"

if (-not $RunQualifiedAcceptance) {
    throw "Exact-CFA bootstrap packaged qualification is explicit. Pass -RunQualifiedAcceptance to launch the frozen desktop twice, produce the scoped report, and adapt it."
}

if ([string]::IsNullOrWhiteSpace($PackagedReportPath)) {
    $PackagedReportPath = Join-Path $resultsRoot "cbsem_exact_case_bootstrap_v1_packaged_acceptance.json"
}
$PackagedReportPath = [System.IO.Path]::GetFullPath($PackagedReportPath)
$packagedParent = [System.IO.Path]::GetFullPath((Split-Path -Parent $PackagedReportPath))
if ($packagedParent -ne $resultsRoot -or [System.IO.Path]::GetExtension($PackagedReportPath) -ne ".json") {
    throw "PackagedReportPath must be a direct .json child of validation\results: $PackagedReportPath"
}

$desktopPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"))
$cliPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\qpls.exe"))
$harnessPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\v247_tauri_native_acceptance.mjs"))
$closeHelperPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"))
$networkMonitorPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\monitor_quickpls_network.ps1"))
$factoryPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\cbsem_exact_case_bootstrap_v1_factory.py"))
$scopedReportPath = [System.IO.Path]::GetFullPath((Join-Path $resultsRoot "v247_tauri_native_acceptance_cbsem_exact_bootstrap.json"))

foreach ($required in @($desktopPath, $cliPath, $harnessPath, $closeHelperPath, $networkMonitorPath, $factoryPath)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Exact-CFA bootstrap packaged acceptance requires the frozen input: $required"
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
    param([int]$TimeoutMilliseconds = 10000)
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
    $pending = New-Object 'System.Collections.Generic.Queue[int]'
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
    param([Parameter(Mandatory = $true)][string]$Path)
    $file = Get-Item -LiteralPath $Path -ErrorAction Stop
    if (-not $file.PSIsContainer -and $file.Length -le 0) { throw "Evidence artifact is empty: $($file.FullName)" }
    $relative = $file.FullName.Substring($repositoryRoot.Length).TrimStart('\').Replace('\', '/')
    return [pscustomobject]@{
        path = $relative
        size = [int64]$file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Read-NetworkSamples {
    param([Parameter(Mandatory = $true)][string]$Path)
    $rows = @(Get-Content -LiteralPath $Path -Encoding UTF8 | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
    if ($rows.Count -eq 0 `
        -or @($rows | Where-Object {
            $_.root_present -ne $true `
            -or [string]$_.observation -ne "sampled_exact_process_tree_tcp_v1" `
            -or $null -eq $_.PSObject.Properties["remote_connections"] `
            -or $null -eq $_.PSObject.Properties["remote_connections"].Value
        }).Count -ne 0) {
        throw "Exact-CFA bootstrap phase process-tree network observation is empty, malformed, or incomplete: $Path"
    }
    return $rows
}

if ((Get-QuickPlsDesktopProcesses).Count -ne 0) {
    throw "Close every existing quickpls-desktop.exe instance before exact-CFA bootstrap packaged acceptance."
}
if (@(Get-NetTCPConnection -State Listen -LocalPort 9222 -ErrorAction SilentlyContinue).Count -ne 0) {
    throw "Exact-CFA bootstrap acceptance requires TCP port 9222 to be unused before launch."
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$projectPath = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp.qpls"
$schema6Path = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp-v6.qpls"
$checkpointPath = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp.checkpoint.json"
$exportPath = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp.xlsx"
$executeNetworkPath = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp-execute-network.jsonl"
$reopenNetworkPath = Join-Path $resultsRoot "cbsem-exact-bootstrap-$stamp-reopen-network.jsonl"
foreach ($exclusive in @($projectPath, $schema6Path, $checkpointPath, $exportPath, $executeNetworkPath, $reopenNetworkPath)) {
    if (Test-Path -LiteralPath $exclusive) { throw "Exact-CFA bootstrap runtime target must be new: $exclusive" }
}

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
$env:QUICKPLS_CDP_ENDPOINT = $cdpEndpoint
$env:QUICKPLS_CLI_PATH = $cliPath
$env:QUICKPLS_DESKTOP_EXE_PATH = $desktopPath
$env:QUICKPLS_PYTHON = if (Test-Path -LiteralPath "C:\Python313\python.exe" -PathType Leaf) { "C:\Python313\python.exe" } else { "python" }
$env:QUICKPLS_ACCEPTANCE_SCOPE = "cbsem_exact_bootstrap"
$env:QUICKPLS_CBSEM_EXACT_PROJECT_PATH = $projectPath
$env:QUICKPLS_CBSEM_EXACT_SCHEMA6_PATH = $schema6Path
$env:QUICKPLS_CBSEM_EXACT_CHECKPOINT_PATH = $checkpointPath
$env:QUICKPLS_CBSEM_EXACT_EXPORT_PATH = $exportPath
$env:QUICKPLS_CBSEM_EXACT_PACKAGED_REPORT_PATH = $PackagedReportPath

$phaseReceipts = New-Object System.Collections.Generic.List[object]
$forcedCleanupUsed = $false

function Invoke-ExactCbsemPhase {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("execute", "reopen")][string]$Phase,
        [Parameter(Mandatory = $true)][string]$NetworkPath
    )
    if ((Get-QuickPlsDesktopProcesses).Count -ne 0 -or (Test-CdpReady)) {
        throw "Exact-CFA bootstrap $Phase phase did not start from a clean process/CDP boundary."
    }
    $stopPath = [System.IO.Path]::ChangeExtension($NetworkPath, ".stop")
    if (Test-Path -LiteralPath $stopPath) { throw "Exact-CFA bootstrap network stop signal already exists: $stopPath" }
    $application = $null
    $monitor = $null
    $tracked = @()
    $phaseError = $null
    $cleanupErrors = New-Object System.Collections.Generic.List[string]
    $phaseStarted = [DateTime]::UtcNow
    $phaseForcedCleanup = $false
    try {
        $env:QUICKPLS_CBSEM_EXACT_PHASE = $Phase
        $application = Start-Process -FilePath $desktopPath -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
        $monitor = Start-Process `
            -FilePath "powershell.exe" `
            -ArgumentList @(
                "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $networkMonitorPath,
                "-RootProcessId", [string]$application.Id,
                "-SamplesPath", $NetworkPath,
                "-StopSignalPath", $stopPath,
                "-IntervalMilliseconds", "200"
            ) `
            -WorkingDirectory $repositoryRoot `
            -WindowStyle Hidden `
            -PassThru
        $cdpReady = $false
        for ($attempt = 0; $attempt -lt 120; $attempt++) {
            if (Test-CdpReady) { $cdpReady = $true; break }
            Start-Sleep -Milliseconds 250
        }
        if (-not $cdpReady) { throw "QuickPLS WebView2 CDP did not open for exact-CFA bootstrap $Phase." }
        & node $harnessPath
        if ($LASTEXITCODE -ne 0) { throw "Exact-CFA bootstrap $Phase harness failed with exit code $LASTEXITCODE." }
        $tracked = @(Get-TrackedProcessTree -RootProcessId $application.Id)
    } catch {
        $phaseError = $_
    } finally {
        try {
            [System.IO.File]::WriteAllText($stopPath, "stop", $utf8WithoutBom)
            if ($monitor -and -not $monitor.HasExited -and -not $monitor.WaitForExit(15000)) {
                Stop-Process -Id $monitor.Id -Force -ErrorAction SilentlyContinue
                $phaseForcedCleanup = $true
                $script:forcedCleanupUsed = $true
                $cleanupErrors.Add("The exact-CFA bootstrap $Phase network monitor required forced cleanup.")
            }
            if ($monitor -and $monitor.HasExited -and $monitor.ExitCode -ne 0) {
                $cleanupErrors.Add("The exact-CFA bootstrap $Phase network monitor exited with code $($monitor.ExitCode).")
            }
        } catch {
            $cleanupErrors.Add("Exact-CFA bootstrap $Phase network-monitor cleanup failed: $($_.Exception.Message)")
        }
        try {
            if ($application -and -not $application.HasExited) {
                $priorPreference = $ErrorActionPreference
                $ErrorActionPreference = "SilentlyContinue"
                & node $closeHelperPath 2>$null | Out-Null
                $ErrorActionPreference = $priorPreference
                if (-not $application.WaitForExit(15000)) {
                    Stop-Process -Id $application.Id -Force -ErrorAction SilentlyContinue
                    $null = $application.WaitForExit(10000)
                    $phaseForcedCleanup = $true
                    $script:forcedCleanupUsed = $true
                    $cleanupErrors.Add("The exact-CFA bootstrap $Phase desktop process required forced cleanup.")
                }
            }
        } catch {
            $cleanupErrors.Add("Exact-CFA bootstrap $Phase desktop cleanup failed: $($_.Exception.Message)")
        }
        $cdpClosed = Wait-CdpClosed
        $remainingDesktop = @(Get-QuickPlsDesktopProcesses)
        $liveIds = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.ProcessId })
        $lingering = @($tracked | Where-Object { $liveIds -contains [int]$_.process_id })
        if (-not $cdpClosed -or $remainingDesktop.Count -ne 0 -or $lingering.Count -ne 0) {
            $cleanupErrors.Add("Exact-CFA bootstrap $Phase left its exact PID tree or CDP endpoint alive.")
        }
        Remove-Item -LiteralPath $stopPath -Force -ErrorAction SilentlyContinue
        if ($application) {
            $script:phaseReceipts.Add([pscustomobject]@{
                phase = $Phase
                root_pid = [int]$application.Id
                started_at_utc = $phaseStarted.ToString("o")
                completed_at_utc = [DateTime]::UtcNow.ToString("o")
                observed_process_tree = $tracked
                cdp_closed = [bool]$cdpClosed
                forced_cleanup_used = [bool]$phaseForcedCleanup
                orphan_processes = [int]$lingering.Count
                network_samples_path = $NetworkPath
            })
        }
    }
    if ($phaseError) { throw $phaseError }
    if ($cleanupErrors.Count -ne 0) { throw ($cleanupErrors -join " ") }
    if (-not (Test-Path -LiteralPath $NetworkPath -PathType Leaf)) { throw "Exact-CFA bootstrap $Phase emitted no network samples." }
    $null = Read-NetworkSamples -Path $NetworkPath
}

try {
    Invoke-ExactCbsemPhase -Phase "execute" -NetworkPath $executeNetworkPath
    Invoke-ExactCbsemPhase -Phase "reopen" -NetworkPath $reopenNetworkPath
} finally {
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,Env:QUICKPLS_CDP_ENDPOINT,Env:QUICKPLS_CLI_PATH,Env:QUICKPLS_DESKTOP_EXE_PATH,Env:QUICKPLS_PYTHON,Env:QUICKPLS_ACCEPTANCE_SCOPE,Env:QUICKPLS_CBSEM_EXACT_PROJECT_PATH,Env:QUICKPLS_CBSEM_EXACT_SCHEMA6_PATH,Env:QUICKPLS_CBSEM_EXACT_CHECKPOINT_PATH,Env:QUICKPLS_CBSEM_EXACT_EXPORT_PATH,Env:QUICKPLS_CBSEM_EXACT_PACKAGED_REPORT_PATH,Env:QUICKPLS_CBSEM_EXACT_PHASE -ErrorAction SilentlyContinue
}

foreach ($artifact in @($PackagedReportPath, $scopedReportPath, $projectPath, $schema6Path, $checkpointPath, $exportPath, $executeNetworkPath, $reopenNetworkPath)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf) -or (Get-Item -LiteralPath $artifact).Length -le 0) {
        throw "Exact-CFA bootstrap packaged acceptance did not create non-empty evidence: $artifact"
    }
}
if ($forcedCleanupUsed -or $phaseReceipts.Count -ne 2 `
    -or @($phaseReceipts | Where-Object { $_.cdp_closed -ne $true -or $_.orphan_processes -ne 0 }).Count -ne 0 `
    -or (Get-QuickPlsDesktopProcesses).Count -ne 0) {
    throw "Exact-CFA bootstrap did not complete two graceful, orphan-free desktop process lifecycles."
}

$scoped = Get-Content -LiteralPath $scopedReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($scoped.passed -ne $true -or @($scoped.failures).Count -ne 0 -or @($scoped.consoleErrors).Count -ne 0 `
    -or [string]$scoped.acceptance_scope -ne "cbsem_exact_bootstrap" -or [string]$scoped.phase -ne "reopen") {
    throw "Exact-CFA bootstrap scoped Tauri report is failed, impure, or not the fresh-process reopen phase."
}

$raw = Get-Content -LiteralPath $PackagedReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
$expectedChecks = @(
    "setup", "invalid_setup_blocked", "execute_percentile", "execute_studentized", "execute_bca",
    "cancellation_retry", "result_identity", "xlsx_same_run", "save_reopen_same_run", "offline",
    "viewports", "process_cleanup"
) | Sort-Object
$actualChecks = @($raw.checks.PSObject.Properties | ForEach-Object { [string]$_.Name } | Sort-Object)
if ((Compare-Object -ReferenceObject $expectedChecks -DifferenceObject $actualChecks).Count -ne 0) {
    throw "Exact-CFA bootstrap raw report did not contain exactly the factory's 12 checks."
}
$nonCleanupFailures = @($raw.checks.PSObject.Properties | Where-Object { $_.Name -ne "process_cleanup" -and $_.Value.passed -ne $true })
if ($nonCleanupFailures.Count -ne 0) {
    throw "Exact-CFA bootstrap producer left non-cleanup checks failed: $($nonCleanupFailures.Name -join ', ')"
}
$executeNetworkRows = @(Read-NetworkSamples -Path $executeNetworkPath)
$reopenNetworkRows = @(Read-NetworkSamples -Path $reopenNetworkPath)
$allNetworkRows = @($executeNetworkRows) + @($reopenNetworkRows)
$remoteConnections = @($allNetworkRows | ForEach-Object { @($_.remote_connections) })
$platformBackgroundEgressObserved = $remoteConnections.Count -gt 0
$platformBackgroundEgressObservation = [pscustomobject]@{
    passed = $true
    observation_kind = "sampled_exact_process_tree_tcp_v1"
    sample_count = $allNetworkRows.Count
    root_present_every_sample = $true
    platform_background_egress_observed = $platformBackgroundEgressObserved
    commercial_zero_egress_passed = -not $platformBackgroundEgressObserved
    remote_connections = @($remoteConnections)
}
$offlineEvidence = $raw.checks.offline.evidence
if ($raw.checks.offline.passed -ne $true `
    -or $null -eq $offlineEvidence `
    -or $offlineEvidence.passed -ne $true `
    -or [int]$offlineEvidence.externalRequestCount -ne 0 `
    -or @($offlineEvidence.externalRequests).Count -ne 0 `
    -or $offlineEvidence.analyticalWorkflowRequiresInternet -ne $false) {
    throw "Exact-CFA bootstrap page/application functional-offline evidence failed or is incomplete."
}
$offlineEvidence | Add-Member -NotePropertyName strictZeroProcessEgressClaimed -NotePropertyValue $false -Force
$offlineEvidence | Add-Member -NotePropertyName platformBackgroundEgressOutsidePageRequestScope -NotePropertyValue $true -Force
$distinctRootPidCount = ($phaseReceipts | ForEach-Object { [int]$_.root_pid } | Sort-Object -Unique | Measure-Object).Count
$raw.checks.process_cleanup = [pscustomobject]@{
    passed = $true
    exact_root_pid_count = 2
    distinct_root_pids = $distinctRootPidCount -eq 2
    graceful_process_cleanup_verified = $true
    forced_process_cleanup_used = $false
    orphan_processes = 0
    cdp_closed_after_each_phase = $true
    platform_background_egress_observation = $platformBackgroundEgressObservation
    sampled_process_tree_zero_egress = -not $platformBackgroundEgressObserved
    network_sample_count = $allNetworkRows.Count
    phases = @($phaseReceipts)
    network_artifacts = @(
        Get-ArtifactDescriptor -Path $executeNetworkPath
        Get-ArtifactDescriptor -Path $reopenNetworkPath
    )
}
$raw.passed = $true
$raw.generated_at_utc = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
[System.IO.File]::WriteAllText(
    $PackagedReportPath,
    (($raw | ConvertTo-Json -Depth 100) + [Environment]::NewLine),
    $utf8WithoutBom
)

if ((Get-QuickPlsDesktopProcesses).Count -ne 0 -or (Test-CdpReady)) {
    throw "Exact-CFA bootstrap adaptation is forbidden while the desktop or CDP endpoint remains alive."
}
python $factoryPath --adapt-packaged --packaged-report $PackagedReportPath
if ($LASTEXITCODE -ne 0) {
    throw "Exact-CFA bootstrap method-factory adaptation failed with exit code $LASTEXITCODE."
}

$packagedIdentity = Join-Path $resultsRoot "method_factory\cbsem_exact_case_bootstrap_v1\packaged_acceptance.identity.json"
$methodAuditIdentity = Join-Path $resultsRoot "method_factory\cbsem_exact_case_bootstrap_v1\method_audit.identity.json"
foreach ($identity in @($packagedIdentity, $methodAuditIdentity)) {
    if (-not (Test-Path -LiteralPath $identity -PathType Leaf) -or (Get-Item -LiteralPath $identity).Length -le 0) {
        throw "Exact-CFA bootstrap factory did not mint its release identity: $identity"
    }
}

[pscustomobject]@{
    passed = $true
    scope = "cbsem_exact_case_bootstrap_v1"
    raw_packaged_report = Get-ArtifactDescriptor -Path $PackagedReportPath
    packaged_acceptance_identity = Get-ArtifactDescriptor -Path $packagedIdentity
    method_audit_identity = Get-ArtifactDescriptor -Path $methodAuditIdentity
    desktop = Get-ArtifactDescriptor -Path $desktopPath
    cli = Get-ArtifactDescriptor -Path $cliPath
    project = Get-ArtifactDescriptor -Path $projectPath
    schema6_project = Get-ArtifactDescriptor -Path $schema6Path
    xlsx = Get-ArtifactDescriptor -Path $exportPath
    process_phases = @($phaseReceipts)
    platform_background_egress_observation = $platformBackgroundEgressObservation
} | ConvertTo-Json -Depth 16
