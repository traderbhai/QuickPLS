param()

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

$desktopExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"))
$cliExecutable = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "target\release\qpls.exe"))
$harnessPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\v247_tauri_native_acceptance.mjs"))
$closeHelperPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\close_tauri_test_window.mjs"))
$cumulativeAssemblerPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\assemble_v247_cumulative_native_acceptance.py"))
$cumulativeReportPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results\v247_tauri_native_acceptance.json"))
$fullReportPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results\v247_tauri_native_acceptance_full.json"))
$cumulativeReceiptPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results\v247_cumulative_native_acceptance_receipt.json"))
$acceptanceContractPath = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\capabilities\packaged_windows_acceptance_v2.manifest.json"))
$resultsDirectory = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$cdpEndpoint = "http://127.0.0.1:9222"
$supervisorStartedUtc = [DateTime]::UtcNow

foreach ($requiredFile in @($desktopExecutable, $cliExecutable, $harnessPath, $closeHelperPath, $cumulativeAssemblerPath, $acceptanceContractPath)) {
    if (-not (Test-Path -LiteralPath $requiredFile -PathType Leaf)) {
        throw "Required cumulative native acceptance input is missing: $requiredFile"
    }
}

try {
    $acceptanceContract = Get-Content -LiteralPath $acceptanceContractPath -Raw -Encoding UTF8 | ConvertFrom-Json
} catch {
    throw "Packaged Windows acceptance contract is not valid JSON: $acceptanceContractPath ($($_.Exception.Message))"
}
if ($acceptanceContract.schema_version -ne 2 -or [string]($acceptanceContract.contract_id) -ne "quickpls.packaged_windows_acceptance.v2") {
    throw "Packaged Windows acceptance contract identity is invalid: $acceptanceContractPath"
}
$expectedFinalCheckNames = @($acceptanceContract.ordered_check_sets | ForEach-Object { @($_.required_check_ids) })
$expectedFinalCheckCount = $expectedFinalCheckNames.Count
$phase2ReleaseCheckNames = @($acceptanceContract.phase2_release_required_check_ids)
if ($expectedFinalCheckCount -eq 0 -or $phase2ReleaseCheckNames.Count -eq 0) {
    throw "Packaged Windows acceptance contract contains no required check IDs."
}
$contractCheckSet = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
foreach ($checkName in $expectedFinalCheckNames) {
    if ([string]::IsNullOrWhiteSpace([string]$checkName) -or -not $contractCheckSet.Add([string]$checkName)) {
        throw "Packaged Windows acceptance contract contains an empty or duplicate required check ID: '$checkName'."
    }
}
$unknownPhase2Checks = @($phase2ReleaseCheckNames | Where-Object { -not $contractCheckSet.Contains([string]$_) })
if ($unknownPhase2Checks.Count -ne 0) {
    throw "Packaged Windows acceptance contract contains Phase-2 checks outside the full contract: $($unknownPhase2Checks -join ', ')."
}

$nodeExecutable = (Get-Command node.exe -ErrorAction Stop).Source
$powershellExecutable = (Get-Command powershell.exe -ErrorAction Stop).Source
$pythonExecutable = "C:\Python313\python.exe"
if (-not (Test-Path -LiteralPath $pythonExecutable -PathType Leaf)) {
    throw "The packaged native acceptance Python runtime is missing: $pythonExecutable"
}

function ConvertTo-ProcessDescriptor {
    param([object]$Process)
    return [pscustomobject]@{
        ProcessId = [int]$Process.ProcessId
        ParentProcessId = [int]$Process.ParentProcessId
        Name = [string]$Process.Name
        ExecutablePath = [string]$Process.ExecutablePath
        CreationDate = [string]$Process.CreationDate
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

function Assert-CleanLaunchBoundary {
    param([string]$Stage)
    $desktopProcesses = @(Get-QuickPlsDesktopProcesses)
    if ($desktopProcesses.Count -ne 0) {
        $identities = @($desktopProcesses | ForEach-Object { "pid=$($_.ProcessId),path=$($_.ExecutablePath)" })
        throw "$Stage cannot start while a QuickPLS desktop process exists: $($identities -join '; ')"
    }
    if (Test-CdpReady) {
        throw "$Stage cannot start while the dedicated WebView2 CDP endpoint is already open: $cdpEndpoint"
    }
}

function Add-TrackedProcessTree {
    param(
        [int]$RootProcessId,
        [hashtable]$TrackedProcesses
    )
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
    $pending = New-Object System.Collections.Generic.Queue[int]
    $discovered = New-Object 'System.Collections.Generic.HashSet[int]'
    $pending.Enqueue($RootProcessId)
    $null = $discovered.Add($RootProcessId)
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($discovered.Add($childId)) {
                $pending.Enqueue($childId)
            }
        }
    }
    foreach ($row in $rows | Where-Object { $discovered.Contains([int]$_.ProcessId) }) {
        $key = [string][int]$row.ProcessId
        if (-not $TrackedProcesses.ContainsKey($key)) {
            $TrackedProcesses[$key] = ConvertTo-ProcessDescriptor -Process $row
        }
    }
}

function Get-LiveTrackedProcesses {
    param([hashtable]$TrackedProcesses)
    $live = @()
    foreach ($descriptor in $TrackedProcesses.Values) {
        $processId = [int]$descriptor.ProcessId
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
        if ($current -and [string]$current.CreationDate -eq [string]$descriptor.CreationDate -and [string]$current.Name -eq [string]$descriptor.Name) {
            $live += ConvertTo-ProcessDescriptor -Process $current
        }
    }
    return @($live | Sort-Object ProcessId)
}

function Wait-TrackedProcessesExit {
    param(
        [hashtable]$TrackedProcesses,
        [int]$TimeoutMilliseconds = 5000
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    $live = @(Get-LiveTrackedProcesses -TrackedProcesses $TrackedProcesses)
    while ($live.Count -gt 0 -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 250
        $live = @(Get-LiveTrackedProcesses -TrackedProcesses $TrackedProcesses)
    }
    return @($live)
}

function Stop-ExactTrackedProcesses {
    param([object[]]$Processes)
    $stopped = @()
    foreach ($descriptor in $Processes | Sort-Object ProcessId -Descending) {
        $processId = [int]$descriptor.ProcessId
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
        if ($current -and [string]$current.CreationDate -eq [string]$descriptor.CreationDate -and [string]$current.Name -eq [string]$descriptor.Name) {
            Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
            $stopped += $processId
        }
    }
    return @($stopped)
}

function Wait-CdpClosed {
    param([int]$TimeoutMilliseconds = 5000)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    while ((Test-CdpReady) -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 250
    }
    return -not (Test-CdpReady)
}

function Quote-NativeArgument {
    param([string]$Value)
    return '"' + $Value.Replace('"', '\"') + '"'
}

function Start-MonitoredProcess {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$WorkingDirectory,
        [string]$StandardOutputPath,
        [string]$StandardErrorPath,
        [scriptblock]$Observe
    )
    $argumentLine = (($Arguments | ForEach-Object { Quote-NativeArgument -Value $_ }) -join " ")
    $process = Start-Process -FilePath $FilePath -ArgumentList $argumentLine -WorkingDirectory $WorkingDirectory `
        -WindowStyle Hidden -RedirectStandardOutput $StandardOutputPath -RedirectStandardError $StandardErrorPath -PassThru
    while (-not $process.WaitForExit(250)) {
        if ($Observe) { & $Observe }
    }
    if ($Observe) { & $Observe }
    $process.WaitForExit()
    $process.Refresh()
    return [int]$process.ExitCode
}

function Read-LogText {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return "" }
    [string]$text = Get-Content -LiteralPath $Path -Raw -Encoding UTF8
    if ($null -eq $text) { return "" }
    return $text.Trim()
}

function Assert-AcceptanceReport {
    param(
        [string]$Path,
        [DateTime]$NotBeforeUtc,
        [AllowNull()][string]$ExpectedScope,
        [AllowNull()][Nullable[int]]$ExpectedCheckCount
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Acceptance report was not written: $Path"
    }
    $file = Get-Item -LiteralPath $Path
    if ($file.Length -le 0 -or $file.LastWriteTimeUtc -lt $NotBeforeUtc.AddSeconds(-2)) {
        throw "Acceptance report is empty or stale: $Path"
    }
    try {
        $report = Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Acceptance report is not valid JSON: $Path ($($_.Exception.Message))"
    }
    if ($report.passed -ne $true) {
        throw "Acceptance report is red: $Path"
    }
    if (@($report.failures).Count -ne 0 -or @($report.consoleErrors).Count -ne 0) {
        throw "Acceptance report contains failures or console errors: $Path"
    }
    $generatedAt = [DateTimeOffset]::Parse([string]$report.generatedAt).UtcDateTime
    if ($generatedAt -lt $NotBeforeUtc.AddSeconds(-2)) {
        throw "Acceptance report generation time predates this stage: $Path"
    }
    if ($null -eq $report.checks) {
        throw "Acceptance report has no checks object: $Path"
    }
    $checkNames = @($report.checks.PSObject.Properties.Name)
    if ($checkNames.Count -eq 0 -or @($checkNames | Where-Object { [string]::IsNullOrWhiteSpace($_) }).Count -ne 0) {
        throw "Acceptance report has an empty or unnamed check set: $Path"
    }
    $uniqueNames = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
    foreach ($checkName in $checkNames) {
        if (-not $uniqueNames.Add([string]$checkName)) {
            throw "Acceptance report contains a duplicate check name '$checkName': $Path"
        }
        $check = $report.checks.PSObject.Properties[$checkName].Value
        if ($check -and $check.PSObject.Properties.Name -contains "passed" -and $check.passed -ne $true) {
            throw "Acceptance report contains a red check '$checkName': $Path"
        }
    }
    if ($null -ne $ExpectedCheckCount -and $checkNames.Count -ne $ExpectedCheckCount) {
        throw "Acceptance report check count is $($checkNames.Count); expected exactly ${ExpectedCheckCount}: $Path"
    }
    if ($null -eq $ExpectedScope) {
        if ($null -ne $report.focusedRun) {
            throw "Fresh full acceptance unexpectedly reported a focused scope: $Path"
        }
    } else {
        if ([string]$report.focusedRun.scope -ne $ExpectedScope) {
            throw "Focused report scope '$($report.focusedRun.scope)' does not equal '$ExpectedScope': $Path"
        }
    }
    return [pscustomobject]@{
        Report = $report
        CheckNames = $checkNames
        Sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
        Size = [int64]$file.Length
    }
}

function Wait-AcceptanceReportPublished {
    param(
        [string]$Path,
        [DateTime]$NotBeforeUtc,
        [int]$TimeoutMilliseconds = 5000
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (Test-Path -LiteralPath $Path -PathType Leaf) {
            $file = Get-Item -LiteralPath $Path
            if ($file.Length -gt 0 -and $file.LastWriteTimeUtc -ge $NotBeforeUtc.AddSeconds(-2)) {
                return $true
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $false
}

function Assert-ExportArtifacts {
    param([string[]]$Paths)
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Expected native XLSX export is missing: $path"
        }
        if ((Get-Item -LiteralPath $path).Length -le 0) {
            throw "Expected native XLSX export is empty: $path"
        }
    }
}

function Restore-Environment {
    param(
        [hashtable]$PriorValues,
        [string[]]$Names
    )
    foreach ($name in $Names) {
        [Environment]::SetEnvironmentVariable($name, $PriorValues[$name], "Process")
    }
}

function Invoke-FreshFullAcceptance {
    param([hashtable]$ExportPaths)
    Assert-CleanLaunchBoundary -Stage "Fresh full native acceptance"
    $stageStartedUtc = [DateTime]::UtcNow
    $environmentNames = @(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "QUICKPLS_CDP_ENDPOINT", "QUICKPLS_CLI_PATH",
        "QUICKPLS_DESKTOP_EXE_PATH", "QUICKPLS_PYTHON", "QUICKPLS_ACCEPTANCE_SCOPE",
        "QUICKPLS_NATIVE_EXPORT_PATH", "QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH", "QUICKPLS_PLSC_NATIVE_EXPORT_PATH", "QUICKPLS_WPLS_NATIVE_EXPORT_PATH",
        "QUICKPLS_MGA_NATIVE_EXPORT_PATH", "QUICKPLS_CCA_NATIVE_EXPORT_PATH",
        "QUICKPLS_IPMA_NATIVE_EXPORT_PATH", "QUICKPLS_NCA_NATIVE_EXPORT_PATH"
    )
    $priorEnvironment = @{}
    foreach ($name in $environmentNames) {
        $priorEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    $application = $null
    $tracked = @{}
    $stageError = $null
    $cleanupError = $null
    $forcedPids = @()
    $stdoutPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-full-$PID.stdout.txt"
    $stderrPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-full-$PID.stderr.txt"
    $closeStdoutPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-close-$PID.stdout.txt"
    $closeStderrPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-close-$PID.stderr.txt"
    try {
        [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--remote-debugging-port=9222", "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $cdpEndpoint, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_CLI_PATH", $cliExecutable, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_DESKTOP_EXE_PATH", $desktopExecutable, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_PYTHON", $pythonExecutable, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_ACCEPTANCE_SCOPE", "full", "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_NATIVE_EXPORT_PATH", $ExportPaths.generic, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH", $ExportPaths.bootstrap, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_PLSC_NATIVE_EXPORT_PATH", $ExportPaths.plsc, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_WPLS_NATIVE_EXPORT_PATH", $ExportPaths.wpls, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_MGA_NATIVE_EXPORT_PATH", $ExportPaths.mga, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_CCA_NATIVE_EXPORT_PATH", $ExportPaths.cca, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_IPMA_NATIVE_EXPORT_PATH", $ExportPaths.ipma, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_NCA_NATIVE_EXPORT_PATH", $ExportPaths.nca, "Process")

        $application = Start-Process -FilePath $desktopExecutable -WorkingDirectory $repositoryRoot -WindowStyle Normal -PassThru
        Add-TrackedProcessTree -RootProcessId $application.Id -TrackedProcesses $tracked
        $cdpReady = $false
        for ($attempt = 0; $attempt -lt 120; $attempt++) {
            Add-TrackedProcessTree -RootProcessId $application.Id -TrackedProcesses $tracked
            if (Test-CdpReady) {
                $cdpReady = $true
                break
            }
            if ($application.HasExited) { break }
            Start-Sleep -Milliseconds 500
        }
        if (-not $cdpReady) {
            throw "QuickPLS WebView2 CDP did not open for fresh full acceptance."
        }
        $observeDesktop = { Add-TrackedProcessTree -RootProcessId $application.Id -TrackedProcesses $tracked }
        $harnessExitCode = Start-MonitoredProcess -FilePath $nodeExecutable -Arguments @($harnessPath) `
            -WorkingDirectory $repositoryRoot -StandardOutputPath $stdoutPath -StandardErrorPath $stderrPath -Observe $observeDesktop
        if ($harnessExitCode -ne 0) {
            throw "Fresh full packaged acceptance exited $harnessExitCode. stdout=$(Read-LogText $stdoutPath) stderr=$(Read-LogText $stderrPath)"
        }
        $null = Assert-AcceptanceReport -Path $cumulativeReportPath -NotBeforeUtc $stageStartedUtc -ExpectedScope $null -ExpectedCheckCount $null
        Assert-ExportArtifacts -Paths @($ExportPaths.generic, $ExportPaths.bootstrap, $ExportPaths.plsc, $ExportPaths.wpls, $ExportPaths.mga, $ExportPaths.cca, $ExportPaths.ipma, $ExportPaths.nca)
    } catch {
        $stageError = $_.Exception.Message
    } finally {
        try {
            if ($application) {
                Add-TrackedProcessTree -RootProcessId $application.Id -TrackedProcesses $tracked
                if (-not $application.HasExited) {
                    $closeExitCode = Start-MonitoredProcess -FilePath $nodeExecutable -Arguments @($closeHelperPath) `
                        -WorkingDirectory $repositoryRoot -StandardOutputPath $closeStdoutPath -StandardErrorPath $closeStderrPath -Observe $observeDesktop
                    $gracefulExit = $application.WaitForExit(10000)
                    if ($closeExitCode -ne 0 -or -not $gracefulExit) {
                        $cleanupError = "Graceful full-run close failed (helper=$closeExitCode, exited=$gracefulExit)."
                    }
                }
            }
            $live = @(Wait-TrackedProcessesExit -TrackedProcesses $tracked -TimeoutMilliseconds 5000)
            if ($live.Count -gt 0) {
                $forcedPids = @(Stop-ExactTrackedProcesses -Processes $live)
                $null = Wait-TrackedProcessesExit -TrackedProcesses $tracked -TimeoutMilliseconds 5000
                $cleanupError = "Fresh full acceptance required forced exact-PID cleanup: $($forcedPids -join ',')."
            }
            $lingering = @(Get-LiveTrackedProcesses -TrackedProcesses $tracked)
            if ($lingering.Count -gt 0) {
                $cleanupError = "Fresh full acceptance left tracked processes: $(@($lingering.ProcessId) -join ',')."
            }
            if (-not (Wait-CdpClosed -TimeoutMilliseconds 5000)) {
                $cleanupError = "Fresh full acceptance left the dedicated CDP endpoint open."
            }
        } catch {
            $cleanupError = "Fresh full cleanup diagnostics failed: $($_.Exception.Message)"
        }
        Restore-Environment -PriorValues $priorEnvironment -Names $environmentNames
        Remove-Item -LiteralPath $stdoutPath, $stderrPath, $closeStdoutPath, $closeStderrPath -Force -ErrorAction SilentlyContinue
    }
    if ($cleanupError) { throw $cleanupError }
    if ($stageError) { throw $stageError }
    Assert-CleanLaunchBoundary -Stage "Post-full focused chain"
}

function Invoke-FocusedWrapper {
    param(
        [string]$Name,
        [string]$Scope,
        [string]$ScriptPath,
        [string[]]$Arguments,
        [string[]]$ExportPaths,
        [pscustomobject]$FullReportBaseline
    )
    Assert-CleanLaunchBoundary -Stage "Focused $Name acceptance"
    $stageStartedUtc = [DateTime]::UtcNow
    $stdoutPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-$Name-$PID.stdout.txt"
    $stderrPath = Join-Path ([System.IO.Path]::GetTempPath()) "quickpls-cumulative-$Name-$PID.stderr.txt"
    $rootIds = New-Object 'System.Collections.Generic.HashSet[int]'
    $tracked = @{}
    $acceptedDesktopIdentities = @{}
    $unresolvedDesktopIdentities = @{}
    $unexpectedDesktop = @{}
    $observeWrapper = {
        foreach ($desktop in @(Get-QuickPlsDesktopProcesses)) {
            $descriptor = ConvertTo-ProcessDescriptor -Process $desktop
            Add-TrackedProcessTree -RootProcessId ([int]$descriptor.ProcessId) -TrackedProcesses $tracked
            $identityKey = "$($descriptor.ProcessId)|$($descriptor.CreationDate)|$($descriptor.Name)"
            $observedPath = ""
            if (-not [string]::IsNullOrWhiteSpace([string]$descriptor.ExecutablePath)) {
                try {
                    $observedPath = [System.IO.Path]::GetFullPath([string]$descriptor.ExecutablePath)
                } catch {
                    $observedPath = ""
                }
            }
            if ([string]::IsNullOrWhiteSpace($observedPath)) {
                if (-not $acceptedDesktopIdentities.ContainsKey($identityKey)) {
                    $unresolvedDesktopIdentities[$identityKey] = $descriptor
                }
                continue
            }
            $unresolvedDesktopIdentities.Remove($identityKey)
            if (-not [string]::Equals($observedPath, $desktopExecutable, [System.StringComparison]::OrdinalIgnoreCase)) {
                $unexpectedDesktop[$identityKey] = $descriptor
                continue
            }
            $acceptedDesktopIdentities[$identityKey] = $descriptor
            $null = $rootIds.Add([int]$descriptor.ProcessId)
        }
    }
    $wrapperArguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath) + $Arguments
    $wrapperExitCode = $null
    try {
        $wrapperExitCode = Start-MonitoredProcess -FilePath $powershellExecutable -Arguments $wrapperArguments `
            -WorkingDirectory $repositoryRoot -StandardOutputPath $stdoutPath -StandardErrorPath $stderrPath -Observe $observeWrapper
    } finally {
        & $observeWrapper
        $live = @(Wait-TrackedProcessesExit -TrackedProcesses $tracked -TimeoutMilliseconds 5000)
        $forcedPids = @()
        if ($live.Count -gt 0) {
            $forcedPids = @(Stop-ExactTrackedProcesses -Processes $live)
            $null = Wait-TrackedProcessesExit -TrackedProcesses $tracked -TimeoutMilliseconds 5000
        }
        $lingering = @(Get-LiveTrackedProcesses -TrackedProcesses $tracked)
        $cdpClosed = Wait-CdpClosed -TimeoutMilliseconds 5000
    }
    $stdout = Read-LogText -Path $stdoutPath
    $stderr = Read-LogText -Path $stderrPath
    if ($unexpectedDesktop.Count -ne 0) {
        throw "Focused $Name acceptance observed an unexpected QuickPLS executable: $($unexpectedDesktop.Values | ConvertTo-Json -Compress)"
    }
    if ($unresolvedDesktopIdentities.Count -ne 0) {
        throw "Focused $Name acceptance could not authenticate every observed QuickPLS executable: $($unresolvedDesktopIdentities.Values | ConvertTo-Json -Compress)"
    }
    if ($rootIds.Count -ne 1) {
        throw "Focused $Name acceptance observed $($rootIds.Count) release desktop PIDs; expected exactly one."
    }
    if ($forcedPids.Count -ne 0 -or $lingering.Count -ne 0 -or -not $cdpClosed) {
        throw "Focused $Name acceptance cleanup was not graceful and exact (forced=$($forcedPids -join ','), lingering=$(@($lingering.ProcessId) -join ','), cdpClosed=$cdpClosed)."
    }
    if ($wrapperExitCode -ne 0) {
        throw "Focused $Name wrapper exited $wrapperExitCode. stdout=$stdout stderr=$stderr"
    }
    Assert-ExportArtifacts -Paths $ExportPaths
    $scopedReportPath = Join-Path $resultsDirectory "v247_tauri_native_acceptance_$Scope.json"
    $scopedPublished = Wait-AcceptanceReportPublished -Path $scopedReportPath -NotBeforeUtc $stageStartedUtc
    if (-not $scopedPublished) {
        throw "Focused $Name wrapper exited 0 without publishing a fresh scoped report. stdout=$stdout stderr=$stderr"
    }
    $scoped = Assert-AcceptanceReport -Path $scopedReportPath -NotBeforeUtc $stageStartedUtc -ExpectedScope $Scope -ExpectedCheckCount $null
    $preservedFull = Assert-AcceptanceReport -Path $cumulativeReportPath -NotBeforeUtc $supervisorStartedUtc `
        -ExpectedScope $null -ExpectedCheckCount $null
    if ($preservedFull.Size -ne $FullReportBaseline.Size -or $preservedFull.Sha256 -ne $FullReportBaseline.Sha256) {
        throw "Focused $Name acceptance changed the preserved full acceptance report."
    }
    Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
    Assert-CleanLaunchBoundary -Stage "After focused $Name acceptance"
}

New-Item -ItemType Directory -Path $resultsDirectory -Force | Out-Null
Remove-Item -LiteralPath $cumulativeReceiptPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $fullReportPath -Force -ErrorAction SilentlyContinue
$runStamp = "$(Get-Date -Format 'yyyyMMdd-HHmmssfff')-$PID-$([Guid]::NewGuid().ToString('N').Substring(0, 8))"
$exports = [ordered]@{
    generic = Join-Path $resultsDirectory "v247-native-full-$runStamp.xlsx"
    bootstrap = Join-Path $resultsDirectory "v247-native-pls-bootstrap-v4-$runStamp.xlsx"
    plsc = Join-Path $resultsDirectory "v247-native-plsc-$runStamp.xlsx"
    wpls = Join-Path $resultsDirectory "v247-native-wpls-$runStamp.xlsx"
    mga = Join-Path $resultsDirectory "v247-native-mga-$runStamp.xlsx"
    cca = Join-Path $resultsDirectory "v247-native-cca-$runStamp.xlsx"
    ipma = Join-Path $resultsDirectory "v247-native-ipma-$runStamp.xlsx"
    nca = Join-Path $resultsDirectory "v247-native-nca-$runStamp.xlsx"
    prediction = Join-Path $resultsDirectory "v247-native-prediction-$runStamp.xlsx"
    hoc = Join-Path $resultsDirectory "v247-native-hoc-$runStamp.xlsx"
    pca = Join-Path $resultsDirectory "v247-native-pca-$runStamp.xlsx"
    ols = Join-Path $resultsDirectory "v247-native-ols-$runStamp.xlsx"
    cbsem = Join-Path $resultsDirectory "v247-native-cbsem-$runStamp.xlsx"
    gsca = Join-Path $resultsDirectory "v247-native-gsca-$runStamp.xlsx"
    logistic = Join-Path $resultsDirectory "v247-native-logistic-$runStamp.xlsx"
    regression_bootstrap_ols = Join-Path $resultsDirectory "v247-native-regression-bootstrap-ols-$runStamp.xlsx"
    regression_bootstrap_logistic = Join-Path $resultsDirectory "v247-native-regression-bootstrap-logistic-$runStamp.xlsx"
}
$uniqueExportPaths = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
foreach ($exportPath in $exports.Values) {
    $absoluteExportPath = [System.IO.Path]::GetFullPath([string]$exportPath)
    if (-not $uniqueExportPaths.Add($absoluteExportPath)) {
        throw "Cumulative native acceptance generated a duplicate export path: $absoluteExportPath"
    }
    if (Test-Path -LiteralPath $absoluteExportPath) {
        throw "Cumulative native acceptance refuses to overwrite an existing export: $absoluteExportPath"
    }
}

Invoke-FreshFullAcceptance -ExportPaths $exports
$fullReportBaseline = Assert-AcceptanceReport -Path $cumulativeReportPath -NotBeforeUtc $supervisorStartedUtc `
    -ExpectedScope $null -ExpectedCheckCount $null
$primaryCheckNames = @($acceptanceContract.ordered_check_sets | Where-Object { [string]$_.scope -eq "full" } | ForEach-Object { @($_.required_check_ids) })
$missingPrimaryChecks = @($primaryCheckNames | Where-Object { $fullReportBaseline.CheckNames -notcontains $_ })
if ($missingPrimaryChecks.Count -ne 0) {
    throw "Fresh full acceptance omitted required primary checks: $($missingPrimaryChecks -join ', ')."
}
Copy-Item -LiteralPath $cumulativeReportPath -Destination $fullReportPath

$focusedStages = @(
    [pscustomobject]@{ Name = "prediction"; Scope = "prediction"; Script = "run_v247_prediction_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.prediction); Exports = @($exports.prediction) },
    [pscustomobject]@{ Name = "hoc"; Scope = "hoc"; Script = "run_v247_hoc_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.hoc); Exports = @($exports.hoc) },
    [pscustomobject]@{ Name = "pca"; Scope = "pca"; Script = "run_v247_pca_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.pca, "-PreserveMainReport"); Exports = @($exports.pca) },
    [pscustomobject]@{ Name = "ols"; Scope = "ols"; Script = "run_v247_ols_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.ols); Exports = @($exports.ols) },
    [pscustomobject]@{ Name = "cbsem"; Scope = "cbsem"; Script = "run_v247_cbsem_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.cbsem); Exports = @($exports.cbsem) },
    [pscustomobject]@{ Name = "gsca"; Scope = "gsca"; Script = "run_v247_gsca_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.gsca, "-ReceiptPath", (Join-Path $resultsDirectory "v247_gsca_scoped_native_acceptance_receipt_$runStamp.json")); Exports = @($exports.gsca) },
    [pscustomobject]@{ Name = "logistic"; Scope = "logistic"; Script = "run_v247_logistic_native_acceptance.ps1"; Arguments = @("-ExportPath", $exports.logistic); Exports = @($exports.logistic) },
    [pscustomobject]@{ Name = "regression_bootstrap"; Scope = "regression_bootstrap"; Script = "run_v247_regression_bootstrap_native_acceptance.ps1"; Arguments = @("-OlsExportPath", $exports.regression_bootstrap_ols, "-LogisticExportPath", $exports.regression_bootstrap_logistic); Exports = @($exports.regression_bootstrap_ols, $exports.regression_bootstrap_logistic) }
)

foreach ($stage in $focusedStages) {
    $wrapperPath = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot $stage.Script))
    if (-not (Test-Path -LiteralPath $wrapperPath -PathType Leaf)) {
        throw "Focused native acceptance wrapper is missing: $wrapperPath"
    }
    Invoke-FocusedWrapper -Name $stage.Name -Scope $stage.Scope -ScriptPath $wrapperPath -Arguments $stage.Arguments `
        -ExportPaths $stage.Exports -FullReportBaseline $fullReportBaseline
}

$assemblyNotBeforeUtc = $supervisorStartedUtc.ToString("o")
$assemblyOutput = & $pythonExecutable $cumulativeAssemblerPath --not-before-utc $assemblyNotBeforeUtc 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "Cumulative acceptance assembly failed: $($assemblyOutput -join [Environment]::NewLine)"
}
$final = Assert-AcceptanceReport -Path $cumulativeReportPath -NotBeforeUtc $supervisorStartedUtc `
    -ExpectedScope ([string]$acceptanceContract.final_scope) -ExpectedCheckCount $expectedFinalCheckCount
$missingRequiredChecks = @($expectedFinalCheckNames | Where-Object { $final.CheckNames -notcontains $_ })
$unexpectedChecks = @($final.CheckNames | Where-Object { $expectedFinalCheckNames -notcontains $_ })
if ($missingRequiredChecks.Count -ne 0 -or $unexpectedChecks.Count -ne 0) {
    throw "Final cumulative acceptance check IDs differ from the manifest (missing=$($missingRequiredChecks -join ','), unexpected=$($unexpectedChecks -join ','))."
}
$missingPhase2ReleaseChecks = @($phase2ReleaseCheckNames | Where-Object { $final.CheckNames -notcontains $_ })
if ($missingPhase2ReleaseChecks.Count -ne 0) {
    throw "Final cumulative acceptance omitted frozen Phase-2 release checks: $($missingPhase2ReleaseChecks -join ', ')."
}
Assert-ExportArtifacts -Paths @($exports.Values)
Assert-CleanLaunchBoundary -Stage "Completed cumulative native acceptance"

$exportDescriptors = @($exports.GetEnumerator() | ForEach-Object {
    $file = Get-Item -LiteralPath $_.Value
    [pscustomobject]@{
        role = [string]$_.Key
        path = "validation/results/$($file.Name)"
        size = [int64]$file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
})
$receipt = [pscustomobject]@{
    schema_version = 2
    kind = "quickpls_v247_cumulative_native_acceptance_receipt"
    passed = $true
    supervisor_started_at_utc = $supervisorStartedUtc.ToString("o")
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
    report = "validation/results/v247_tauri_native_acceptance.json"
    checks = $final.CheckNames.Count
    unique_checks = $final.CheckNames.Count
    failures = 0
    console_errors = 0
    report_sha256 = $final.Sha256
    report_size = $final.Size
    final_scope = [string]$acceptanceContract.final_scope
    graceful_process_cleanup_verified = $true
    acceptance_contract = [pscustomobject]@{
        path = "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
        contract_id = [string]$acceptanceContract.contract_id
        contract_version = [string]$acceptanceContract.contract_version
        required_check_count = $expectedFinalCheckCount
        sha256 = (Get-FileHash -LiteralPath $acceptanceContractPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    assembler = [pscustomobject]@{
        path = "validation/assemble_v247_cumulative_native_acceptance.py"
        size = [int64](Get-Item -LiteralPath $cumulativeAssemblerPath).Length
        sha256 = (Get-FileHash -LiteralPath $cumulativeAssemblerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    full_report = [pscustomobject]@{
        path = "validation/results/v247_tauri_native_acceptance_full.json"
        size = [int64](Get-Item -LiteralPath $fullReportPath).Length
        sha256 = (Get-FileHash -LiteralPath $fullReportPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    exports = $exportDescriptors
}
$receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $cumulativeReceiptPath -Encoding UTF8
$receipt | ConvertTo-Json -Depth 6
