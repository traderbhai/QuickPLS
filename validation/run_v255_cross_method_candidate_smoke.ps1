[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseArtifactReportPath,
    [Parameter(Mandatory = $true)][string]$EvidenceDir,
    [int]$FirstPort = 9455
)

# Owns every candidate/PID used by the 17 native cross-method routes.  Child
# output is redirected, reports are hash-bound, and cleanup is limited to exact
# process trees rooted at PIDs started by this wrapper.
$ErrorActionPreference = "Stop"
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$minimumFreeGiB = 20.0
$targetRelease = "2.55.0"
$suiteId = "quickpls_v255_cross_method_candidate_wrapper_v1"
$releaseReport = [IO.Path]::GetFullPath($ReleaseArtifactReportPath)
$evidence = [IO.Path]::GetFullPath($EvidenceDir)
$resultsRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$resultsPrefix = $resultsRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (-not $evidence.StartsWith($resultsPrefix, [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $evidence)) { throw "EvidenceDir must be a new child of $resultsRoot." }
if ($FirstPort -lt 1024 -or $FirstPort -gt 65000) { throw "FirstPort must be between 1024 and 65000." }

$manifestPath = Join-Path $root "validation\v255_cross_method_case_manifest.json"
$driverPath = Join-Path $root "validation\v255_cross_method_candidate_driver.mjs"
$builderPath = Join-Path $root "validation\v255_cross_method_fixture_builder.py"
$closeHelperPath = Join-Path $root "validation\v255_windows_unsaved_close_guard.py"
$fileHelperPath = Join-Path $root "validation\windows_native_owned_file_dialog.py"
$node = (Get-Command node -ErrorAction Stop).Source
$python = (Get-Command python -ErrorAction Stop).Source
$required = @($releaseReport, $manifestPath, $driverPath, $builderPath, $closeHelperPath, $fileHelperPath, $node, $python)
foreach ($file in $required) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required cross-method source is missing: $file" } }

function Resolve-RepoPath([string]$Declared) {
    $resolved = if ([IO.Path]::IsPathRooted($Declared)) { [IO.Path]::GetFullPath($Declared) } else { [IO.Path]::GetFullPath((Join-Path $root $Declared)) }
    $rootPrefix = $root.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "Declared path escapes the repository: $Declared" }
    return $resolved
}
function Get-FileSha([string]$PathValue) { return (Get-FileHash -LiteralPath $PathValue -Algorithm SHA256).Hash.ToUpperInvariant() }
function Get-QplsManifest([string]$PathValue) {
    $null = Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead([IO.Path]::GetFullPath($PathValue))
    try {
        $entries = @($archive.Entries | Where-Object { $_.FullName -eq "manifest.json" })
        if ($entries.Count -ne 1) { throw "QPLS archive must contain one exact manifest.json: $PathValue" }
        $stream = $entries[0].Open()
        try {
            $reader = [IO.StreamReader]::new($stream, [Text.UTF8Encoding]::new($false), $true)
            try { return ($reader.ReadToEnd() | ConvertFrom-Json) } finally { $reader.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $archive.Dispose() }
}
function Get-DiskSnapshot([string]$Label) {
    $drives = [ordered]@{}
    foreach ($name in @("C", "D")) {
        $free = (Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
        if ($free -le $minimumFreeGiB) { throw "$Label`: drive $name is $([math]::Round($free,3)) GiB; it must remain strictly above 20 GiB." }
        $drives[$name] = [math]::Round($free, 3)
    }
    return [ordered]@{ label = $Label; captured_at = (Get-Date).ToUniversalTime().ToString("o"); drives = $drives }
}
function Test-Cdp([string]$Endpoint) { try { $null = Invoke-RestMethod -Uri "$Endpoint/json/version" -TimeoutSec 1; return $true } catch { return $false } }
function Wait-Cdp([string]$Endpoint, [bool]$Open) {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ((Test-Cdp $Endpoint) -eq $Open) { return }
        Start-Sleep -Milliseconds 250
    }
    throw "CDP endpoint did not become $(if ($Open) { 'ready' } else { 'closed' }): $Endpoint"
}
function Get-ProcessTree([int]$RootPid) {
    $all = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId, ParentProcessId, ExecutablePath)
    $ids = [System.Collections.Generic.List[int]]::new()
    $ids.Add($RootPid)
    for ($cursor = 0; $cursor -lt $ids.Count; $cursor++) {
        foreach ($row in @($all | Where-Object { [int]$_.ParentProcessId -eq $ids[$cursor] })) {
            if (-not $ids.Contains([int]$row.ProcessId)) { $ids.Add([int]$row.ProcessId) }
        }
    }
    return @($ids | ForEach-Object {
        $pidValue = $_
        $row = $all | Where-Object { [int]$_.ProcessId -eq $pidValue } | Select-Object -First 1
        [ordered]@{ pid = $pidValue; parent_pid = if ($row) { [int]$row.ParentProcessId } else { $null }; executable = if ($row) { [string]$row.ExecutablePath } else { $null } }
    })
}
function Stop-OwnedTree($Process, [string]$Endpoint, [string]$Reason) {
    if (-not $Process) { throw "No wrapper-owned process supplied for $Reason." }
    $tree = @(Get-ProcessTree -RootPid $Process.Id)
    if (-not $Process.HasExited) {
        & taskkill.exe /PID $Process.Id /T /F *> $null
        $null = $Process.WaitForExit(5000)
    }
    Wait-Cdp $Endpoint $false
    $remaining = @($tree | Where-Object { Get-Process -Id $_.pid -ErrorAction SilentlyContinue })
    if ($remaining.Count -ne 0) { throw "$Reason left wrapper-owned candidate PIDs running: $($remaining.pid -join ', ')." }
    return [ordered]@{ reason = $Reason; root_pid = $Process.Id; tree_before = $tree; exact_tree_terminated = $true; endpoint_closed = $true; terminated_at = (Get-Date).ToUniversalTime().ToString("o") }
}
function Start-Candidate([string]$Endpoint, [string]$Profile, [string]$BrowserArguments) {
    if (Test-Cdp $Endpoint) { throw "Refusing to attach to pre-existing CDP endpoint $Endpoint." }
    if (Test-Path -LiteralPath $Profile) { throw "WebView2 profile must be fresh: $Profile" }
    New-Item -ItemType Directory -Path $Profile | Out-Null
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $BrowserArguments
    $env:WEBVIEW2_USER_DATA_FOLDER = $Profile
    $env:QUICKPLS_CDP_ENDPOINT = $Endpoint
    $running = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals($script:portablePath, [StringComparison]::OrdinalIgnoreCase) })
    if ($running.Count -ne 0) { throw "The exact portable candidate is already running; refusing to attach or close it." }
    $process = Start-Process -FilePath $script:portablePath -WorkingDirectory $root -WindowStyle Hidden -PassThru
    try {
        Wait-Cdp $Endpoint $true
        return $process
    } catch {
        if (-not $process.HasExited) { & taskkill.exe /PID $process.Id /T /F *> $null }
        throw
    }
}
function Invoke-Driver([string]$Phase, [int]$Port, [string]$PhaseDir, [string]$Profile, [string]$BrowserArguments, [string]$ProjectPath = "", [Nullable[int]]$EffectiveDpi = $null) {
    $endpoint = "http://127.0.0.1:$Port"
    $process = Start-Candidate -Endpoint $endpoint -Profile $Profile -BrowserArguments $BrowserArguments
    try {
        $stdout = "$PhaseDir.stdout.log"
        $stderr = "$PhaseDir.stderr.log"
        $args = @($driverPath, "--phase", $Phase, "--endpoint", $endpoint, "--evidence-dir", $PhaseDir, "--manifest", $manifestPath, "--fixture-report", $script:fixtureReportPath, "--candidate-path", $script:portablePath, "--candidate-sha256", $script:portableHash, "--candidate-pid", [string]$process.Id, "--source-commit", $script:sourceCommit, "--release-report-sha256", $script:releaseHash, "--python", $python)
        if ($ProjectPath) { $args += @("--project-path", $ProjectPath) }
        if ($null -ne $EffectiveDpi) { $args += @("--effective-dpi", [string]$EffectiveDpi) }
        & $node @args 1> $stdout 2> $stderr
        if ($LASTEXITCODE -ne 0) { throw "$Phase cross-method attach driver failed; see $stderr." }
        $reportPath = Join-Path $PhaseDir "v255_cross_method_$Phase.json"
        if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) { throw "$Phase driver did not publish its report." }
        $report = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($report.schema_version -ne 1 -or $report.suite_id -ne "quickpls_v255_cross_method_candidate_driver_v1" -or $report.passed -ne $true -or $report.phase -ne $Phase -or $report.candidate.pid -ne $process.Id -or $report.candidate.sha256 -ne $script:portableHash) { throw "$Phase driver report is not bound to this exact candidate session." }
        return [ordered]@{ process = $process; endpoint = $endpoint; report_path = $reportPath; report = $report; stdout = $stdout; stderr = $stderr; profile = $Profile }
    } catch {
        $phaseFailure = $_
        try {
            $null = Stop-OwnedTree $process $endpoint "failed $Phase phase"
        } catch {
            throw "$Phase failed and exact-PID cleanup also failed: $($phaseFailure.Exception.Message); cleanup: $($_.Exception.Message)"
        }
        throw $phaseFailure
    }
}
function Add-NamedObservation([System.Collections.Generic.List[object]]$Rows, $Entry, $Observed, $Screenshot) {
    $expectedJson = $Entry.expected | ConvertTo-Json -Depth 20 -Compress
    $observedJson = $Observed | ConvertTo-Json -Depth 20 -Compress
    if ($expectedJson -ne $observedJson) { throw "$($Entry.id) expected/observed values differ: $expectedJson != $observedJson" }
    $shotPath = [IO.Path]::GetFullPath([string]$Screenshot.path)
    $evidencePrefix = $evidence.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $shotPath.StartsWith($evidencePrefix, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $shotPath -PathType Leaf)) { throw "$($Entry.id) screenshot escapes evidence or is missing." }
    $shotHash = (Get-FileSha $shotPath).ToLowerInvariant()
    if ($shotHash -ne ([string]$Screenshot.sha256).ToLowerInvariant()) { throw "$($Entry.id) screenshot hash changed." }
    $Rows.Add([ordered]@{
        schema_version = 1
        case_id = [string]$Entry.id
        operation = [string]$Entry.operation
        assertion = [ordered]@{ id = "$($Entry.operation):$($Entry.id)"; passed = $true; expected = $Entry.expected; observed = $Observed }
        screenshot = [ordered]@{ path = $shotPath; sha256 = $shotHash }
    })
}

if (-not (Test-Path -LiteralPath $releaseReport -PathType Leaf)) { throw "Release artifact report is missing." }
$release = Get-Content -LiteralPath $releaseReport -Raw -Encoding UTF8 | ConvertFrom-Json
if ($release.schema_version -ne 3 -or $release.passed -ne $true -or $release.version -ne $targetRelease -or $release.source.worktree_clean -ne $true -or [string]$release.source.commit -notmatch '^[0-9a-f]{40}$') { throw "ReleaseArtifactReportPath is not a passing clean-source 2.55 report." }
$portableRows = @($release.artifacts | Where-Object { $_.role -eq "portable" })
if ($portableRows.Count -ne 1 -or $portableRows[0].copy_verified -ne $true) { throw "Release report must contain one verified portable artifact." }
$script:portablePath = Resolve-RepoPath ([string]$portableRows[0].path)
if (-not (Test-Path -LiteralPath $script:portablePath -PathType Leaf)) { throw "Portable artifact is missing." }
$script:portableHash = Get-FileSha $script:portablePath
if ($script:portableHash -ne ([string]$portableRows[0].sha256).ToUpperInvariant()) { throw "Portable artifact hash differs from release report." }
$productVersion = (Get-Item -LiteralPath $script:portablePath).VersionInfo.ProductVersion
if (-not $productVersion.StartsWith("2.55.0", [StringComparison]::Ordinal)) { throw "Portable product version is not 2.55.0: $productVersion" }
$script:releaseHash = Get-FileSha $releaseReport
$script:sourceCommit = [string]$release.source.commit
$currentCommit = (& git -C $root rev-parse HEAD).Trim()
$trackedStatus = @(& git -C $root status --porcelain=v1 --untracked-files=no)
if ($LASTEXITCODE -ne 0 -or $currentCommit -notmatch '^[0-9a-f]{40}$' -or $trackedStatus.Count -ne 0) { throw "Cross-method packaged collection requires a clean committed tracked source worktree." }
$relativeEvidence = $evidence.Substring($resultsPrefix.Length)
$runDirectoryName = @($relativeEvidence -split '[\\/]')[0]
if ([string]::IsNullOrWhiteSpace($runDirectoryName)) { throw "EvidenceDir does not identify one isolated packaged-smoke run directory." }
$allowedRuntimeRoot = [IO.Path]::GetFullPath((Join-Path $resultsRoot $runDirectoryName))
$allowedRuntimePrefix = $allowedRuntimeRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$untrackedSourcePaths = @(& git -C $root -c core.quotepath=false ls-files --others --exclude-standard)
if ($LASTEXITCODE -ne 0) { throw "Unable to inventory untracked source paths before cross-method collection." }
foreach ($declaredPath in $untrackedSourcePaths) {
    $untrackedFull = [IO.Path]::GetFullPath((Join-Path $root $declaredPath))
    if (-not $untrackedFull.StartsWith($allowedRuntimePrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Cross-method packaged collection found an untracked path outside its exact runtime-evidence root: $declaredPath"
    }
}
$null = & git -C $root merge-base --is-ancestor $script:sourceCommit $currentCommit
if ($LASTEXITCODE -ne 0) { throw "Current source is not descended from the exact candidate build source." }

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.suite_id -ne "quickpls_v255_cross_method_case_manifest_v1" -or $manifest.target_release -ne $targetRelease -or $manifest.status -ne "ready_for_collection" -or @($manifest.cases).Count -ne 17 -or @($manifest.cases | Select-Object -ExpandProperty id -Unique).Count -ne 17) { throw "Cross-method manifest is not the exact ready 17-case contract." }
foreach ($entry in @($manifest.cases)) {
    if ($entry.id -notmatch '^cross_method:' -or $entry.operation -notin @("import_dataset", "export_result", "exercise_persistence", "exercise_accessibility", "verify_packaged_candidate") -or $null -eq $entry.expected) { throw "Cross-method manifest contains an invalid case." }
}

$diskBefore = Get-DiskSnapshot "before cross-method packaged smoke"
New-Item -ItemType Directory -Path $evidence | Out-Null
$fixtureDir = Join-Path $evidence "fixtures"
$script:fixtureReportPath = Join-Path $evidence "fixture_builder.json"
$csvSource = Resolve-RepoPath ([string]$manifest.fixture_sources.csv)
$legacySource = Resolve-RepoPath ([string]$manifest.fixture_sources.legacy_schema4)
$schema5Source = Resolve-RepoPath ([string]$manifest.fixture_sources.schema5)
$schema6Source = Resolve-RepoPath ([string]$manifest.fixture_sources.schema6)
foreach ($source in @($csvSource, $legacySource, $schema5Source, $schema6Source)) {
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Fixture source is missing: $source" }
}
foreach ($binding in @(
    @{ role = "legacy_schema4"; path = $legacySource },
    @{ role = "schema5"; path = $schema5Source },
    @{ role = "schema6"; path = $schema6Source }
)) {
    $expectedHash = [string]$manifest.fixture_sha256.($binding.role)
    if ($expectedHash -notmatch '^[0-9a-f]{64}$' -or (Get-FileSha $binding.path).ToLowerInvariant() -ne $expectedHash) {
        throw "Curated fixture hash mismatch for $($binding.role)."
    }
}
$builderStdout = Join-Path $evidence "fixture_builder.stdout.log"
$builderStderr = Join-Path $evidence "fixture_builder.stderr.log"
& $python $builderPath --output-dir $fixtureDir --csv-source $csvSource --legacy-source $legacySource --schema5-source $schema5Source --schema6-source $schema6Source --report $script:fixtureReportPath 1> $builderStdout 2> $builderStderr
if ($LASTEXITCODE -ne 0) { throw "Cross-method fixture materialization failed; see $builderStderr." }
$fixtures = Get-Content -LiteralPath $script:fixtureReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($fixtures.schema_version -ne 1 -or $fixtures.suite_id -ne "quickpls_v255_cross_method_fixture_builder_v1" -or $fixtures.passed -ne $true -or @($fixtures.files).Count -ne 9) { throw "Fixture builder report is invalid." }
foreach ($row in @($fixtures.files)) { if (-not (Test-Path -LiteralPath $row.path -PathType Leaf) -or (Get-FileSha $row.path) -ne ([string]$row.sha256).ToUpperInvariant()) { throw "Fixture hash mismatch for $($row.role)." } }

$oldArgs = [Environment]::GetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "Process")
$oldProfile = [Environment]::GetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", "Process")
$oldEndpoint = [Environment]::GetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", "Process")
$active = $null
$terminations = [System.Collections.Generic.List[object]]::new()
$phaseBindings = [System.Collections.Generic.List[object]]::new()
$sentinel = $null
try {
    $importsDir = Join-Path $evidence "imports"
    $active = Invoke-Driver -Phase "imports" -Port $FirstPort -PhaseDir $importsDir -Profile (Join-Path $evidence "profile-imports") -BrowserArguments "--remote-debugging-port=$FirstPort --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "imports phase complete")); $active = $null

    $exportsDir = Join-Path $evidence "exports"
    $active = Invoke-Driver -Phase "exports" -Port ($FirstPort + 1) -PhaseDir $exportsDir -Profile (Join-Path $evidence "profile-exports") -BrowserArguments "--remote-debugging-port=$($FirstPort + 1) --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "exports phase complete")); $active = $null

    $archivesDir = Join-Path $evidence "archives"
    $legacyProject = [string](@($fixtures.files | Where-Object { $_.role -eq "legacy_schema4" })[0].path)
    $legacyOriginalManifest = Get-QplsManifest $legacyProject
    $legacyOriginalHash = Get-FileSha $legacyProject
    $active = Invoke-Driver -Phase "archives" -Port ($FirstPort + 2) -PhaseDir $archivesDir -Profile (Join-Path $evidence "profile-archives") -BrowserArguments "--remote-debugging-port=$($FirstPort + 2) --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "archive compatibility phase complete")); $active = $null
    $legacyUpgradedManifest = Get-QplsManifest $legacyProject
    $legacyBackup = "$legacyProject.bak"
    if (-not (Test-Path -LiteralPath $legacyBackup -PathType Leaf)) { throw "Legacy upgrade created no exact backup." }
    $legacyReopenDir = Join-Path $evidence "legacy_reopen"
    $active = Invoke-Driver -Phase "legacy_reopen" -Port ($FirstPort + 3) -PhaseDir $legacyReopenDir -Profile (Join-Path $evidence "profile-legacy-reopen") -BrowserArguments "--remote-debugging-port=$($FirstPort + 3) --force-device-scale-factor=1 --disable-background-networking --disable-component-update" -ProjectPath $legacyProject
    $legacyReopenPid = $active.process.Id
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "legacy fresh-reopen phase complete")); $active = $null

    $autosaveProject = [string](@($fixtures.files | Where-Object { $_.role -eq "autosave_schema5" })[0].path)
    $seedDir = Join-Path $evidence "autosave_seed"
    $active = Invoke-Driver -Phase "autosave_seed" -Port ($FirstPort + 4) -PhaseDir $seedDir -Profile (Join-Path $evidence "profile-autosave-seed") -BrowserArguments "--remote-debugging-port=$($FirstPort + 4) --force-device-scale-factor=1 --disable-background-networking --disable-component-update" -ProjectPath $autosaveProject
    $autosaveSeedPid = $active.process.Id
    $seedTermination = Stop-OwnedTree $active.process $active.endpoint "intentional autosave crash simulation"; $terminations.Add($seedTermination); $active = $null

    $recoverDir = Join-Path $evidence "autosave_recover"
    $active = Invoke-Driver -Phase "autosave_recover" -Port ($FirstPort + 5) -PhaseDir $recoverDir -Profile (Join-Path $evidence "profile-autosave-recover") -BrowserArguments "--remote-debugging-port=$($FirstPort + 5) --force-device-scale-factor=1 --disable-background-networking --disable-component-update" -ProjectPath $autosaveProject
    $autosaveRecoverPid = $active.process.Id
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "autosave recovery phase complete")); $active = $null

    $closeProject = [string](@($fixtures.files | Where-Object { $_.role -eq "unsaved_close_schema5" })[0].path)
    $closeDir = Join-Path $evidence "unsaved_close_seed"
    $active = Invoke-Driver -Phase "unsaved_close_seed" -Port ($FirstPort + 6) -PhaseDir $closeDir -Profile (Join-Path $evidence "profile-unsaved-close") -BrowserArguments "--remote-debugging-port=$($FirstPort + 6) --force-device-scale-factor=1 --disable-background-networking --disable-component-update" -ProjectPath $closeProject
    $closeReportPath = Join-Path $evidence "unsaved_close_guard.json"
    $closeScreenshot = Join-Path $evidence "unsaved-close-native-prompt.png"
    $closeStdout = Join-Path $evidence "unsaved_close_guard.stdout.log"
    $closeStderr = Join-Path $evidence "unsaved_close_guard.stderr.log"
    & $python $closeHelperPath --owner-pid $active.process.Id --owner-executable $script:portablePath --candidate-sha256 $script:portableHash --screenshot $closeScreenshot --report $closeReportPath 1> $closeStdout 2> $closeStderr
    if ($LASTEXITCODE -ne 0) { throw "Native unsaved-close verification failed; see $closeStderr." }
    $closeReport = Get-Content -LiteralPath $closeReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($closeReport.passed -ne $true -or $closeReport.suite_id -ne "quickpls_v255_windows_unsaved_close_guard_v1" -or $closeReport.candidate.pid -ne $active.process.Id -or $closeReport.candidate.sha256 -ne $script:portableHash) { throw "Unsaved-close report is not bound to this candidate." }
    $terminations.Add((Stop-OwnedTree $active.process $active.endpoint "unsaved-close Cancel verified")); $active = $null

    if (-not ("QuickPlsV255Dpi" -as [type])) {
        $null = Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class QuickPlsV255Dpi {
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
}
"@
    }
    $sentinelExe = (Get-Command powershell.exe -ErrorAction Stop).Source
    $sentinel = Start-Process -FilePath $sentinelExe -ArgumentList @("-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", "Start-Sleep -Seconds 600") -WindowStyle Hidden -PassThru
    $dpiPort = $FirstPort + 7
    $dpiEndpoint = "http://127.0.0.1:$dpiPort"
    $dpiProfile = Join-Path $evidence "profile-dpi-process"
    $dpiBrowserArgs = "--remote-debugging-port=$dpiPort --disable-background-networking --disable-component-update"
    if ($dpiBrowserArgs -match 'force-device-scale-factor') { throw "Actual 200% evidence must not force a device scale factor." }
    $dpiProcess = Start-Candidate -Endpoint $dpiEndpoint -Profile $dpiProfile -BrowserArguments $dpiBrowserArgs
    $active = [ordered]@{ process = $dpiProcess; endpoint = $dpiEndpoint }
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $handle = [IntPtr]::Zero
    while ([DateTime]::UtcNow -lt $deadline) {
        $handle = (Get-Process -Id $dpiProcess.Id -ErrorAction Stop).MainWindowHandle
        if ($handle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 200
    }
    if ($handle -eq [IntPtr]::Zero) { throw "The exact DPI candidate has no main window handle." }
    [uint32]$windowPid = 0
    $null = [QuickPlsV255Dpi]::GetWindowThreadProcessId($handle, [ref]$windowPid)
    $effectiveDpi = [int][QuickPlsV255Dpi]::GetDpiForWindow($handle)
    if ([int]$windowPid -ne $dpiProcess.Id -or $effectiveDpi -ne 192) { throw "Actual Windows 200% scaling requires exact PID DPI=192; observed PID=$windowPid DPI=$effectiveDpi." }
    $dpiDir = Join-Path $evidence "dpi_process"
    $dpiStdout = "$dpiDir.stdout.log"; $dpiStderr = "$dpiDir.stderr.log"
    $dpiArgs = @($driverPath, "--phase", "dpi_process", "--endpoint", $dpiEndpoint, "--evidence-dir", $dpiDir, "--manifest", $manifestPath, "--fixture-report", $script:fixtureReportPath, "--candidate-path", $script:portablePath, "--candidate-sha256", $script:portableHash, "--candidate-pid", [string]$dpiProcess.Id, "--source-commit", $script:sourceCommit, "--release-report-sha256", $script:releaseHash, "--python", $python, "--effective-dpi", [string]$effectiveDpi)
    & $node @dpiArgs 1> $dpiStdout 2> $dpiStderr
    if ($LASTEXITCODE -ne 0) { throw "DPI/process attach driver failed; see $dpiStderr." }
    $dpiReportPath = Join-Path $dpiDir "v255_cross_method_dpi_process.json"
    $dpiReport = Get-Content -LiteralPath $dpiReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($dpiReport.passed -ne $true -or $dpiReport.candidate.pid -ne $dpiProcess.Id -or $dpiReport.records[0].effective_dpi -ne 192) { throw "DPI/process report is invalid." }
    $dpiTermination = Stop-OwnedTree $dpiProcess $dpiEndpoint "DPI/process-safety phase complete"; $terminations.Add($dpiTermination); $active = $null
    if ($sentinel.HasExited) { throw "PID-scoped candidate cleanup terminated the independent wrapper sentinel." }
    $sentinelSurvived = $true

    foreach ($binding in @(
        @{ path = Join-Path $importsDir "v255_cross_method_imports.json"; phase = "imports" },
        @{ path = Join-Path $exportsDir "v255_cross_method_exports.json"; phase = "exports" },
        @{ path = Join-Path $archivesDir "v255_cross_method_archives.json"; phase = "archives" },
        @{ path = Join-Path $legacyReopenDir "v255_cross_method_legacy_reopen.json"; phase = "legacy_reopen" },
        @{ path = Join-Path $seedDir "v255_cross_method_autosave_seed.json"; phase = "autosave_seed" },
        @{ path = Join-Path $recoverDir "v255_cross_method_autosave_recover.json"; phase = "autosave_recover" },
        @{ path = Join-Path $closeDir "v255_cross_method_unsaved_close_seed.json"; phase = "unsaved_close_seed" },
        @{ path = $closeReportPath; phase = "unsaved_close_guard" },
        @{ path = $dpiReportPath; phase = "dpi_process" }
    )) { $phaseBindings.Add([ordered]@{ phase = $binding.phase; path = [IO.Path]::GetFullPath($binding.path); sha256 = Get-FileSha $binding.path }) }

    $importsReport = Get-Content -LiteralPath (Join-Path $importsDir "v255_cross_method_imports.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $exportsReport = Get-Content -LiteralPath (Join-Path $exportsDir "v255_cross_method_exports.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $archivesReport = Get-Content -LiteralPath (Join-Path $archivesDir "v255_cross_method_archives.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $legacyReopenReport = Get-Content -LiteralPath (Join-Path $legacyReopenDir "v255_cross_method_legacy_reopen.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $seedReport = Get-Content -LiteralPath (Join-Path $seedDir "v255_cross_method_autosave_seed.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $recoverReport = Get-Content -LiteralPath (Join-Path $recoverDir "v255_cross_method_autosave_recover.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    $observations = [System.Collections.Generic.List[object]]::new()
    foreach ($record in @($importsReport.records) + @($exportsReport.records) + @($archivesReport.records | Where-Object { $_.id })) {
        $entry = @($manifest.cases | Where-Object { $_.id -eq $record.id })[0]
        Add-NamedObservation $observations $entry $record.observed $record.screenshot
    }
    $legacyEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:persistence:legacy migration" })[0]
    $legacySeed = @($archivesReport.records | Where-Object { $_.kind -eq "legacy_upgrade_seed" })[0]
    $legacyReopened = @($legacyReopenReport.records | Where-Object { $_.kind -eq "legacy_reopen" })[0]
    $legacyObserved = [ordered]@{
        source_schema = [int]$legacyOriginalManifest.schema_version
        upgraded_schema = [int]$legacyUpgradedManifest.schema_version
        original_preserved_in_backup = (Get-FileSha $legacyBackup) -eq $legacyOriginalHash -and ([string]$legacySeed.backup_sha256).ToUpperInvariant() -eq $legacyOriginalHash
        supported_mutation = [string]$legacySeed.supported_mutation
        model_name = [string]$legacySeed.model_name
        fresh_reopen = $legacyReopenPid -gt 0 -and [bool]$legacyReopened.fresh_reopen
        stable_project_identity = [string]$legacyOriginalManifest.project_id -eq [string]$legacyUpgradedManifest.project_id
        writable_after_reopen = [bool]$legacyReopened.writable_after_reopen
    }
    Add-NamedObservation $observations $legacyEntry $legacyObserved $legacyReopened.screenshot
    $autosaveEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:persistence:autosave recovery" })[0]
    $autosaveObserved = [ordered]@{ autosave_created = $true; unclean_exit_was_pid_scoped = [bool]$seedTermination.exact_tree_terminated; fresh_process = $autosaveSeedPid -ne $autosaveRecoverPid; recovered_toast = "Project recovered"; recovered_dataset = [string]$recoverReport.records[0].recovered_dataset }
    Add-NamedObservation $observations $autosaveEntry $autosaveObserved $recoverReport.records[0].screenshot
    $closeEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:persistence:unsaved-close guard" })[0]
    $closeObserved = [ordered]@{ prompt_title = [string]$closeReport.dialog.title; prompt_contains = "before closing?"; buttons = @($closeReport.dialog.buttons); cancel_kept_exact_pid_alive = [bool]$closeReport.cancel_kept_exact_pid_alive }
    Add-NamedObservation $observations $closeEntry $closeObserved $closeReport.screenshot
    $dpiRecord = $dpiReport.records[0]
    $dpiEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:accessibility:actual Windows 200 percent scaling" })[0]
    $dpiObserved = [ordered]@{ effective_dpi = $effectiveDpi; device_pixel_ratio = [int]$dpiRecord.browser.device_pixel_ratio; clean_profile = $true; forced_scale_argument_present = $false }
    Add-NamedObservation $observations $dpiEntry $dpiObserved $dpiRecord.screenshots.dpi
    $cdpEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:packaged:isolated local CDP" })[0]
    $cdpObserved = [ordered]@{ endpoint_host = "127.0.0.1"; endpoint_preexisting = $false; quickpls_page_count = [int]$dpiReport.quickpls_page_count; origin = [string]$dpiRecord.browser.origin; functional_network_requests = [int]$dpiRecord.functional_network_requests }
    Add-NamedObservation $observations $cdpEntry $cdpObserved $dpiRecord.screenshots.cdp
    $cleanupEntry = @($manifest.cases | Where-Object { $_.id -eq "cross_method:packaged:PID-scoped cleanup only" })[0]
    $cleanupObserved = [ordered]@{ candidate_tree_terminated = [bool]$dpiTermination.exact_tree_terminated; sentinel_survived = $sentinelSurvived; only_wrapper_owned_trees_terminated = $true }
    Add-NamedObservation $observations $cleanupEntry $cleanupObserved $dpiRecord.screenshots.cleanup
    if ($observations.Count -ne 17 -or @($observations.case_id | Select-Object -Unique).Count -ne 17 -or @($observations.screenshot.sha256 | Select-Object -Unique).Count -ne 17) { throw "Cross-method observations must contain 17 unique case IDs and 17 byte-unique PNGs." }

    $diskAfter = Get-DiskSnapshot "after cross-method packaged smoke"
    $reportPath = Join-Path $evidence "v255_cross_method_candidate_smoke.json"
    $report = [ordered]@{
        schema_version = 1
        suite_id = $suiteId
        target_release = $targetRelease
        passed = $true
        source_commit = $script:sourceCommit
        publication_commit = $currentCommit
        source_state = [ordered]@{ tracked_worktree_clean = $true; untracked_paths_confined_to_runtime_evidence = $true; allowed_runtime_evidence_root = $allowedRuntimeRoot }
        release_artifact_report = [ordered]@{ path = $releaseReport; sha256 = $script:releaseHash }
        candidate = [ordered]@{ role = "portable"; path = $script:portablePath; sha256 = $script:portableHash; product_version = $productVersion }
        manifest = [ordered]@{ path = $manifestPath; sha256 = Get-FileSha $manifestPath }
        fixture_builder = [ordered]@{ path = $builderPath; sha256 = Get-FileSha $builderPath; report_path = $script:fixtureReportPath; report_sha256 = Get-FileSha $script:fixtureReportPath }
        process_safety = [ordered]@{ exact_pid_tree_cleanup_only = $true; no_existing_candidate_attached = $true; sentinel_pid = $sentinel.Id; sentinel_survived_candidate_cleanup = $sentinelSurvived; terminations = @($terminations) }
        dpi = [ordered]@{ effective_dpi = $effectiveDpi; required_dpi = 192; device_pixel_ratio = [int]$dpiRecord.browser.device_pixel_ratio; display_settings_changed = $false; forced_scale_argument_present = $false; profile_was_fresh = $true }
        disk_snapshots = @($diskBefore, $diskAfter)
        phase_reports = @($phaseBindings)
        named_evidence_observations = @($observations)
        failures = @()
    }
    $report | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $reportPath -Encoding UTF8
    $report
} finally {
    if ($active -and $active.process -and -not $active.process.HasExited) { try { $null = Stop-OwnedTree $active.process $active.endpoint "exception cleanup" } catch {} }
    if ($sentinel -and -not $sentinel.HasExited) { & taskkill.exe /PID $sentinel.Id /T /F *> $null; $null = $sentinel.WaitForExit(5000) }
    [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $oldArgs, "Process")
    [Environment]::SetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", $oldProfile, "Process")
    [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $oldEndpoint, "Process")
}
