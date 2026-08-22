[CmdletBinding()]
param(
    [string]$BuildRoot = "",
    [string]$ArtifactDirectory = "",
    [string]$ReleaseReportPath = "",
    [string]$Label = "v2_55_0_calculate_evidence"
)

# Builds into a brand-new Cargo target directory and preserves artifacts only
# through validation/package_release_artifacts.py. The resulting report binds
# the binaries to the exact clean Git source, version authorities, successful
# build commands, logs, and copied artifact hashes.

$ErrorActionPreference = "Stop"
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$python = (Get-Command python -ErrorAction Stop).Source
$npm = (Get-Command npm.cmd -ErrorAction Stop).Source
$cargo = (Get-Command cargo.exe -ErrorAction Stop).Source
$packager = Join-Path $root "validation\package_release_artifacts.py"
$minimumFreeGiB = 20.0
$minimumFreeBytes = [long]($minimumFreeGiB * 1GB)
$preflightRequiredGiB = [ordered]@{ C = 26.5; D = 20.5 }
$watcherPollIntervalMs = 1000
$watcherSamples = [System.Collections.Generic.List[object]]::new()
$stamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
$defaultBuildBase = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "QuickPLSBuild"
$target = if ([string]::IsNullOrWhiteSpace($BuildRoot)) { Join-Path $defaultBuildBase "v255-$stamp" } else { [IO.Path]::GetFullPath($BuildRoot) }
$artifactBase = [IO.Path]::GetFullPath((Join-Path $root "target\release\artifacts"))
$artifactBasePrefix = $artifactBase.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$artifactRoot = if ([string]::IsNullOrWhiteSpace($ArtifactDirectory)) { $artifactBase } else { [IO.Path]::GetFullPath($ArtifactDirectory) }
$reportPath = if ([string]::IsNullOrWhiteSpace($ReleaseReportPath)) { Join-Path $artifactRoot "v255_release_artifacts_$stamp.json" } else { [IO.Path]::GetFullPath($ReleaseReportPath) }
$artifactRootPrefix = $artifactRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$releaseDir = Join-Path $target "release"
$sessionPath = Join-Path $target "v255_build_session.json"
$packageStdout = Join-Path $target "package_release_artifacts.stdout.log"
$packageStderr = Join-Path $target "package_release_artifacts.stderr.log"

foreach ($required in @($python, $npm, $cargo, $packager)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required build input is missing: $required" }
}
if ((Get-Content -LiteralPath (Join-Path $root "package.json") -Raw -Encoding UTF8 | ConvertFrom-Json).version -ne "2.55.0") {
    throw "The 2.55 candidate build may run only after all version authorities are 2.55.0."
}
if (Test-Path -LiteralPath $target) { throw "BuildRoot must be a brand-new directory: $target" }
$targetParent = Split-Path -Parent $target
if ([string]::IsNullOrWhiteSpace($targetParent) -or $target -eq [IO.Path]::GetPathRoot($target)) {
    throw "BuildRoot must not be a filesystem root: $target"
}
if (Test-Path -LiteralPath $reportPath) { throw "ReleaseReportPath already exists: $reportPath" }
if (-not ($artifactRoot.Equals($artifactBase, [StringComparison]::OrdinalIgnoreCase) -or $artifactRoot.StartsWith($artifactBasePrefix, [StringComparison]::OrdinalIgnoreCase))) {
    throw "ArtifactDirectory must remain at or beneath the repository target/release/artifacts directory: $artifactBase"
}
if (-not $reportPath.StartsWith($artifactRootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "ReleaseReportPath must remain beneath ArtifactDirectory: $artifactRoot"
}

function Get-DiskSnapshot([string]$LabelText, [bool]$RequireFloor = $true) {
    $drives = [ordered]@{}
    foreach ($name in @("C", "D")) {
        $free = (Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
        if ($RequireFloor -and $free -le $minimumFreeGiB) {
            throw "${LabelText}: drive $name has $([math]::Round($free, 3)) GiB free; more than $minimumFreeGiB GiB is required."
        }
        $drives[$name] = [math]::Round($free, 3)
    }
    [ordered]@{ label = $LabelText; captured_at = [DateTime]::UtcNow.ToString("o"); drives = $drives }
}

function Get-ExactDriveBytes {
    [ordered]@{
        C = [long]((Get-PSDrive -Name C -PSProvider FileSystem -ErrorAction Stop).Free)
        D = [long]((Get-PSDrive -Name D -PSProvider FileSystem -ErrorAction Stop).Free)
    }
}

function Get-BuildPreflight {
    $bytes = Get-ExactDriveBytes
    $requiredBytes = [ordered]@{
        C = [long]($preflightRequiredGiB.C * 1GB)
        D = [long]($preflightRequiredGiB.D * 1GB)
    }
    foreach ($name in @("C", "D")) {
        if ([long]$bytes[$name] -le [long]$requiredBytes[$name]) {
            throw "Build preflight requires drive $name free space strictly above $($preflightRequiredGiB[$name]) GiB; observed $([math]::Round([long]$bytes[$name] / 1GB, 3)) GiB."
        }
    }
    [ordered]@{
        captured_at = [DateTime]::UtcNow.ToString("o")
        observed_free_bytes = $bytes
        required_free_bytes_exclusive = $requiredBytes
        required_free_gib_exclusive = $preflightRequiredGiB
        passed = $true
    }
}

function Get-ExactProcessTree([int]$RootPid) {
    $processes = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId, ParentProcessId)
    $seen = [System.Collections.Generic.HashSet[int]]::new()
    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($RootPid)
    while ($pending.Count -gt 0) {
        $parent = $pending.Dequeue()
        if (-not $seen.Add($parent)) { continue }
        foreach ($child in @($processes | Where-Object { [int]$_.ParentProcessId -eq $parent })) {
            $pending.Enqueue([int]$child.ProcessId)
        }
    }
    @($seen | Sort-Object)
}

function Add-BuildWatcherSample([string]$CommandId, [int]$RootPid, [string]$State) {
    $bytes = Get-ExactDriveBytes
    $tree = Get-ExactProcessTree $RootPid
    $breached = [long]$bytes.C -le $minimumFreeBytes -or [long]$bytes.D -le $minimumFreeBytes
    $sample = [ordered]@{
        captured_at = [DateTime]::UtcNow.ToString("o")
        command_id = $CommandId
        root_pid = $RootPid
        process_tree_pids = @($tree)
        state = $State
        free_bytes = $bytes
        floor_breached = $breached
    }
    $watcherSamples.Add($sample)
    $sample
}

function Stop-ExactBuildTree([Diagnostics.Process]$Process) {
    if ($Process -and -not $Process.HasExited) {
        # This root PID was returned by Start-Process in this invocation. /T
        # therefore reaches only that exact wrapper-owned build process tree.
        & taskkill.exe /PID $Process.Id /T /F *> $null
        $null = $Process.WaitForExit(10000)
        if (-not $Process.HasExited) {
            throw "Could not terminate exact build process tree rooted at PID $($Process.Id)."
        }
    }
}

function Get-LogBinding([string]$PathValue) {
    if (-not (Test-Path -LiteralPath $PathValue -PathType Leaf)) { throw "Build log is missing: $PathValue" }
    $item = Get-Item -LiteralPath $PathValue
    [ordered]@{
        path = $item.FullName
        bytes = [long]$item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToUpperInvariant()
    }
}

function Write-Utf8NoBom([string]$PathValue, [string]$TextValue) {
    [IO.File]::WriteAllText($PathValue, $TextValue, [Text.UTF8Encoding]::new($false))
}

function New-BuildSessionPayload([bool]$Passed, [object[]]$Commands, $FinalDiskSnapshot) {
    $breachDetected = @($watcherSamples | Where-Object { $_.floor_breached -eq $true }).Count -gt 0
    [ordered]@{
        schema_version = 2
        suite_id = "quickpls_unsigned_candidate_build_session_v2"
        passed = $Passed
        target_release = "2.55.0"
        source = $source
        target_directory = [IO.Path]::GetFullPath($target)
        target_preexisting = $false
        started_at_utc = $started
        completed_at_utc = $completed
        environment = [ordered]@{ CARGO_INCREMENTAL = "0" }
        commands = @($Commands)
        minimum_free_gib = $minimumFreeGiB
        disk_snapshots = @($diskBefore, $FinalDiskSnapshot)
        disk_watcher = [ordered]@{
            policy = [ordered]@{
                minimum_free_gib_exclusive = $minimumFreeGiB
                minimum_free_bytes_exclusive = $minimumFreeBytes
                preflight_reserve_gib = [ordered]@{ C = 6.5; D = 0.5 }
                preflight_required_free_gib_exclusive = $preflightRequiredGiB
                preflight_required_free_bytes_exclusive = [ordered]@{
                    C = [long]($preflightRequiredGiB.C * 1GB)
                    D = [long]($preflightRequiredGiB.D * 1GB)
                }
                poll_interval_ms = $watcherPollIntervalMs
                breach_action = "terminate_only_exact_wrapper_owned_process_tree"
            }
            preflight = $buildPreflight
            samples = @($watcherSamples.ToArray())
            breach_detected = $breachDetected
            exact_pid_tree_only = $true
        }
    }
}

function Invoke-BuildCommand([string]$Id, [string]$Executable, [string[]]$Arguments) {
    $stdout = Join-Path $target "$Id.stdout.log"
    $stderr = Join-Path $target "$Id.stderr.log"
    $process = Start-Process -FilePath $Executable -ArgumentList $Arguments -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    try {
        while (-not $process.HasExited) {
            $sample = Add-BuildWatcherSample $Id $process.Id "running"
            if ($sample.floor_breached -eq $true) {
                Stop-ExactBuildTree $process
                throw "Build command '$Id' crossed the 20 GiB C/D floor; only process tree $($process.Id) was terminated."
            }
            $null = $process.WaitForExit($watcherPollIntervalMs)
        }
        $finalSample = Add-BuildWatcherSample $Id $process.Id "completed"
        if ($finalSample.floor_breached -eq $true) {
            throw "Build command '$Id' completed at or below the 20 GiB C/D floor."
        }
    } catch {
        Stop-ExactBuildTree $process
        throw
    }
    $exitCode = $process.ExitCode
    $record = [ordered]@{
        id = $Id
        executable = [IO.Path]::GetFullPath($Executable)
        arguments = @($Arguments)
        exit_code = $exitCode
        stdout = Get-LogBinding $stdout
        stderr = Get-LogBinding $stderr
    }
    if ($exitCode -ne 0) { throw "Build command '$Id' failed with exit code $exitCode. See $stderr" }
    $record
}

$diskBefore = Get-DiskSnapshot "before unsigned 2.55 candidate build"
$sourceText = (& $python $packager --source-provenance-only) -join "`n"
if ($LASTEXITCODE -ne 0) { throw "Could not capture clean-source provenance before the build." }
$source = $sourceText | ConvertFrom-Json
if ($source.worktree_clean -ne $true -or [string]$source.commit -notmatch '^[0-9a-f]{40}$') {
    throw "The source-provenance snapshot is incomplete."
}
$buildPreflight = Get-BuildPreflight

New-Item -ItemType Directory -Path $target -Force | Out-Null
$started = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ")
$priorCargoTarget = [Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Process")
$priorCargoIncremental = [Environment]::GetEnvironmentVariable("CARGO_INCREMENTAL", "Process")
$commandRecords = [System.Collections.Generic.List[object]]::new()
$buildFailure = $null
try {
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_INCREMENTAL = "0"
    try {
        $commandRecords.Add((Invoke-BuildCommand "tauri_desktop_bundle" $npm @("run", "tauri", "--", "build", "--bundles", "nsis", "--ci", "--", "--locked")))
        $commandRecords.Add((Invoke-BuildCommand "locked_release_cli" $cargo @("build", "--locked", "--release", "-p", "qpls-cli")))
    } catch {
        $buildFailure = $_.Exception.Message
    }
} finally {
    if ($null -eq $priorCargoTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    else { $env:CARGO_TARGET_DIR = $priorCargoTarget }
    if ($null -eq $priorCargoIncremental) { Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue }
    else { $env:CARGO_INCREMENTAL = $priorCargoIncremental }
}
$completed = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ")

if ($buildFailure) {
    $diskAfterBuild = Get-DiskSnapshot "after failed locked unsigned 2.55 candidate build" $false
    $failedSession = New-BuildSessionPayload $false @($commandRecords.ToArray()) $diskAfterBuild
    Write-Utf8NoBom $sessionPath (($failedSession | ConvertTo-Json -Depth 12) + "`n")
    throw "$buildFailure Failed build-session receipt: $sessionPath"
}

foreach ($requiredOutput in @(
    (Join-Path $releaseDir "quickpls-desktop.exe"),
    (Join-Path $releaseDir "qpls.exe"),
    (Join-Path $releaseDir "bundle\nsis\QuickPLS_2.55.0_x64-setup.exe")
)) {
    if (-not (Test-Path -LiteralPath $requiredOutput -PathType Leaf) -or (Get-Item -LiteralPath $requiredOutput).Length -le 0) {
        throw "Candidate build output is missing or empty: $requiredOutput"
    }
}
$diskAfterBuild = Get-DiskSnapshot "after locked unsigned 2.55 candidate build"
$watcherBreachDetected = @($watcherSamples | Where-Object { $_.floor_breached -eq $true }).Count -gt 0
if ($watcherBreachDetected) {
    throw "The successful build path contains a disk-watcher floor breach and cannot be packaged."
}

$session = New-BuildSessionPayload $true @($commandRecords.ToArray()) $diskAfterBuild
Write-Utf8NoBom $sessionPath (($session | ConvertTo-Json -Depth 12) + "`n")

New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
& $python $packager --channel unsigned-preview --label $Label --release-dir $releaseDir --artifact-dir $artifactRoot --report $reportPath --build-session $sessionPath 1> $packageStdout 2> $packageStderr
if ($LASTEXITCODE -ne 0) { throw "Release artifact preservation failed. See $packageStderr" }
$releaseReport = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($releaseReport.passed -ne $true -or $releaseReport.schema_version -ne 3 -or $releaseReport.version -ne "2.55.0" -or $releaseReport.source.commit -ne $source.commit) {
    throw "The release artifact report is incomplete or is not bound to this build source."
}
if (@($releaseReport.artifacts | Where-Object { $_.role -in @("portable", "cli", "setup") -and $_.copy_verified -eq $true }).Count -ne 3) {
    throw "The release artifact report does not contain all three verified candidate artifacts."
}
$diskAfter = Get-DiskSnapshot "after unsigned 2.55 candidate build"

[ordered]@{
    schema_version = 1
    suite_id = "quickpls_v255_unsigned_candidate_build_v1"
    passed = $true
    target_release = "2.55.0"
    source_commit = $source.commit
    source_tree = $source.tree
    source_manifest_sha256 = $source.tracked_manifest_sha256
    release_report = [IO.Path]::GetFullPath($reportPath)
    release_report_sha256 = (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToUpperInvariant()
    build_session = $sessionPath
    build_session_sha256 = (Get-FileHash -LiteralPath $sessionPath -Algorithm SHA256).Hash.ToUpperInvariant()
    minimum_free_gib = $minimumFreeGiB
    disk_snapshots = @($diskBefore, $diskAfterBuild, $diskAfter)
} | ConvertTo-Json -Depth 12
