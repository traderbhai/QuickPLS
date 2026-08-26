[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet("installed", "portable")][string]$Kind,
    [Parameter(Mandatory = $true)][string]$PackageReceipt,
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][UInt64]$Seed
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if (-not $IsWindows) { throw "Packaged MultiMod smoke is Windows-only." }
$wrapperStartedAtUtc = [DateTimeOffset]::UtcNow
$wrapperClock = [Diagnostics.Stopwatch]::StartNew()
$wrapperMaximumSeconds = 6480
$minimumCleanupReserveSeconds = 1020
$finalizationReserveSeconds = 120
$postScienceReserveSeconds = $minimumCleanupReserveSeconds + $finalizationReserveSeconds
$scientificMaximumSeconds = $wrapperMaximumSeconds - $postScienceReserveSeconds
$cleanupDeadlineSeconds = $wrapperMaximumSeconds - $finalizationReserveSeconds
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$receiptPath = [IO.Path]::GetFullPath($PackageReceipt)
$outputPath = [IO.Path]::GetFullPath($Output)
$outputDirectory = Split-Path -Parent $outputPath
$driver = Join-Path $PSScriptRoot "multimod_packaged_smoke_driver_v1.mjs"
$driverOutputPath = Join-Path $outputDirectory `
    (".packaged-{0}-driver.{1}.tmp.json" -f $Kind, [Guid]::NewGuid().ToString("N"))
$driverStdoutPath = Join-Path $outputDirectory "packaged-$Kind-driver.stdout.log"
$driverStderrPath = Join-Path $outputDirectory "packaged-$Kind-driver.stderr.log"
foreach ($required in @($receiptPath, $driver)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Packaged smoke input is missing: $required" }
}
if (Test-Path -LiteralPath $outputPath) { throw "Packaged smoke output already exists: $outputPath" }
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-PeSubsystem([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) {
        throw "Candidate is not a valid PE executable: $Path"
    }
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    if ($peOffset -lt 0 -or $peOffset + 96 -ge $bytes.Length -or
        $bytes[$peOffset] -ne 0x50 -or $bytes[$peOffset + 1] -ne 0x45) {
        throw "Candidate PE header is invalid: $Path"
    }
    $optionalHeader = $peOffset + 24
    $magic = [BitConverter]::ToUInt16($bytes, $optionalHeader)
    if ($magic -notin @(0x10b, 0x20b)) { throw "Candidate PE optional-header magic is unsupported." }
    $subsystem = [BitConverter]::ToUInt16($bytes, $optionalHeader + 68)
    if ($subsystem -ne 2) { throw "Candidate PE subsystem is $subsystem; a console-free Windows GUI executable requires subsystem 2." }
    return [ordered]@{ code = $subsystem; identity = "windows_gui"; console_subsystem_absent = $true }
}

function Write-JsonAtomic([string]$Path, $Value) {
    $temporary = "$Path.tmp"
    [IO.File]::WriteAllText($temporary, (($Value | ConvertTo-Json -Depth 60) + "`n"), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Get-RemainingPhaseMilliseconds {
    param([Parameter(Mandatory = $true)][ValidateSet("science", "cleanup", "wrapper")][string]$Phase)
    $deadlineSeconds = switch ($Phase) {
        "science" { $scientificMaximumSeconds }
        "cleanup" { $cleanupDeadlineSeconds }
        "wrapper" { $wrapperMaximumSeconds }
    }
    $remaining = [Math]::Floor(($deadlineSeconds - $wrapperClock.Elapsed.TotalSeconds) * 1000.0)
    if ($remaining -lt 1) {
        throw "Packaged $Kind smoke exhausted its $Phase deadline after $([Math]::Round($wrapperClock.Elapsed.TotalSeconds, 3)) seconds."
    }
    return [int64]$remaining
}

function Assert-PhaseBudget {
    param([Parameter(Mandatory = $true)][ValidateSet("science", "cleanup", "wrapper")][string]$Phase)
    [void](Get-RemainingPhaseMilliseconds -Phase $Phase)
}

function Stop-ExactProcessTree {
    param(
        [Parameter(Mandatory = $true)][Diagnostics.Process]$Process,
        [Parameter(Mandatory = $true)][string]$Operation,
        [ValidateSet("cleanup", "wrapper")][string]$Phase = "cleanup",
        [ValidateRange(1, 60)][int]$MaximumSeconds = 60
    )
    $maximumMilliseconds = [Math]::Min(
        [int64]$MaximumSeconds * 1000L,
        (Get-RemainingPhaseMilliseconds -Phase $Phase)
    )
    if (-not $Process.HasExited) {
        try { $Process.Kill($true) } catch { throw "$Operation process-tree termination failed: $($_.Exception.Message)" }
    }
    if (-not $Process.WaitForExit([int][Math]::Min($maximumMilliseconds, [int]::MaxValue))) {
        throw "$Operation process tree did not terminate inside its bounded cleanup window."
    }
    $Process.Refresh()
    if (-not $Process.HasExited) { throw "$Operation process root remains active after process-tree termination." }
}

function Start-SupervisedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FileName,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath
    )
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FileName
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) { [void]$startInfo.ArgumentList.Add([string]$argument) }
    $child = [Diagnostics.Process]::new()
    $child.StartInfo = $startInfo
    [void]$child.Start()
    return [pscustomobject]@{
        Process = $child
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
        StdoutCopy = $child.StandardOutput.ReadToEndAsync()
        StderrCopy = $child.StandardError.ReadToEndAsync()
        LogsSaved = $false
        ForcedTermination = $false
        ExitEpochMilliseconds = $null
    }
}

function Save-SupervisedProcessLogs {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][ValidateSet("science", "cleanup")][string]$Phase
    )
    if ($Job.LogsSaved) { return }
    $tasks = [Threading.Tasks.Task[]]@($Job.StdoutCopy, $Job.StderrCopy)
    $maximumMilliseconds = [int][Math]::Min(
        10000L,
        (Get-RemainingPhaseMilliseconds -Phase $Phase)
    )
    if (-not [Threading.Tasks.Task]::WaitAll($tasks, $maximumMilliseconds)) {
        throw "Packaged smoke driver logs did not drain inside the $Phase deadline."
    }
    [IO.File]::WriteAllText($Job.StdoutPath, $Job.StdoutCopy.Result, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($Job.StderrPath, $Job.StderrCopy.Result, [Text.UTF8Encoding]::new($false))
    $Job.LogsSaved = $true
}

function Stop-SupervisedDriver {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][string]$Reason,
        [ValidateRange(1, 60)][int]$MaximumSeconds = 60
    )
    if (-not $Job.Process.HasExited) {
        $Job.ForcedTermination = $true
        Stop-ExactProcessTree -Process $Job.Process -Operation "Node packaged-smoke driver ($Reason)" `
            -Phase wrapper -MaximumSeconds $MaximumSeconds
    }
    if ($Job.Process.HasExited) {
        $Job.ExitEpochMilliseconds = ([DateTimeOffset]$Job.Process.ExitTime.ToUniversalTime()).ToUnixTimeMilliseconds()
    }
    Save-SupervisedProcessLogs -Job $Job -Phase "cleanup"
}

function Wait-SupervisedDriver {
    param(
        [Parameter(Mandatory = $true)]$Job,
        [Parameter(Mandatory = $true)][int64]$ScientificDeadlineEpochMilliseconds
    )
    while (-not $Job.Process.HasExited) {
        $remaining = Get-RemainingPhaseMilliseconds -Phase "science"
        $wallRemaining = $ScientificDeadlineEpochMilliseconds - [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        if ($wallRemaining -lt 1) {
            throw "Packaged smoke driver reached its shared scientific deadline; the wrapper cleanup path will terminate Node before the candidate."
        }
        [Threading.Thread]::Sleep([int][Math]::Min(250L, [Math]::Min($remaining, $wallRemaining)))
        $Job.Process.Refresh()
    }
    $Job.ExitEpochMilliseconds = ([DateTimeOffset]$Job.Process.ExitTime.ToUniversalTime()).ToUnixTimeMilliseconds()
    if ([int64]$Job.ExitEpochMilliseconds -gt $ScientificDeadlineEpochMilliseconds -or
        $wrapperClock.Elapsed.TotalSeconds -gt $scientificMaximumSeconds) {
        throw "Packaged smoke driver exited after its shared scientific deadline; late success is rejected."
    }
    Save-SupervisedProcessLogs -Job $Job -Phase "science"
    if ($Job.Process.ExitCode -ne 0) {
        throw "Packaged smoke driver exited with code $($Job.Process.ExitCode). Check $($Job.StderrPath)."
    }
}

function Get-DiskSnapshot([string]$Label) {
    $drives = [ordered]@{}
    foreach ($driveName in @("C", "D")) {
        $minimum = if ($driveName -eq "C") { 20.0 } else { 25.0 }
        $free = (Get-PSDrive -Name $driveName -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
        if ($free -lt $minimum) { throw "$Label`: $driveName has $free GiB free; minimum is $minimum GiB." }
        $drives[$driveName] = [math]::Round($free, 3)
    }
    return [ordered]@{ label = $Label; captured_at_utc = (Get-Date).ToUniversalTime().ToString("o"); free_gib = $drives }
}

function Wait-ExactProcess(
    [Diagnostics.Process]$Process,
    [int]$MaximumSeconds,
    [string]$Operation,
    [ValidateSet("science", "cleanup")][string]$Phase = "science"
) {
    $maximumMilliseconds = [Math]::Min(
        [int64]$MaximumSeconds * 1000L,
        (Get-RemainingPhaseMilliseconds -Phase $Phase)
    )
    if (-not $Process.WaitForExit([int][Math]::Min($maximumMilliseconds, [int]::MaxValue))) {
        Stop-ExactProcessTree -Process $Process -Operation $Operation `
            -Phase $(if ($Phase -eq "science") { "cleanup" } else { "wrapper" }) `
            -MaximumSeconds 60
        throw "$Operation exceeded its bounded $Phase budget and its exact process tree was terminated."
    }
    if ($Phase -eq "science") { Assert-PhaseBudget -Phase "science" } else { Assert-PhaseBudget -Phase "cleanup" }
    if ($Process.ExitCode -ne 0) { throw "$Operation exited with code $($Process.ExitCode)." }
}

function Get-QuickPlsRegistrations {
    $paths = @(
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )
    return @(
        foreach ($path in $paths) {
            Get-ItemProperty -Path $path -ErrorAction SilentlyContinue |
                Where-Object { $_.DisplayName -and [string]$_.DisplayName -match '^QuickPLS(?:\s|$)' }
        }
    )
}

function Get-InstalledPortableEquivalence([string]$InstalledPath, [string]$PortablePath) {
    $portableMarker = "__TAURI_BUNDLE_TYPE_VAR_UNK"
    $installedMarker = "__TAURI_BUNDLE_TYPE_VAR_NSS"
    $portableBytes = [IO.File]::ReadAllBytes($PortablePath)
    $installedBytes = [IO.File]::ReadAllBytes($InstalledPath)
    if ($portableBytes.Length -ne $installedBytes.Length) { throw "Installed and portable executable lengths differ." }
    $portableText = [Text.Encoding]::ASCII.GetString($portableBytes)
    $installedText = [Text.Encoding]::ASCII.GetString($installedBytes)
    $portableOffset = $portableText.IndexOf($portableMarker, [StringComparison]::Ordinal)
    $installedOffset = $installedText.IndexOf($installedMarker, [StringComparison]::Ordinal)
    if ($portableOffset -lt 0 -or $portableOffset -ne $installedOffset) { throw "Installed and portable Tauri markers are not aligned." }
    if ($portableText.IndexOf($portableMarker, $portableOffset + 1, [StringComparison]::Ordinal) -ge 0 -or $installedText.IndexOf($installedMarker, $installedOffset + 1, [StringComparison]::Ordinal) -ge 0) {
        throw "Installed or portable executable contains duplicate Tauri markers."
    }
    $normalized = [byte[]]$installedBytes.Clone()
    $marker = [Text.Encoding]::ASCII.GetBytes($portableMarker)
    [Array]::Copy($marker, 0, $normalized, $installedOffset, $marker.Length)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $normalizedHash = ([Convert]::ToHexString($sha.ComputeHash($normalized))).ToLowerInvariant()
        $portableHash = ([Convert]::ToHexString($sha.ComputeHash($portableBytes))).ToLowerInvariant()
    } finally { $sha.Dispose() }
    if ($normalizedHash -cne $portableHash) { throw "Installed executable differs from portable outside the one NSIS marker." }
    return [ordered]@{ kind = "tauri_nsis_bundle_marker_variant_v1"; passed = $true; marker_offset = $portableOffset; all_other_bytes_identical = $true }
}

$package = Get-Content -LiteralPath $receiptPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 30
if (
    $package.receipt_kind -cne "qpls_multimod_candidate_package_v1" -or
    $package.version -cne [string]$env:QPLS_MULTIMOD_CANDIDATE_VERSION -or
    $package.candidate_commit_sha -cne [string]$env:QPLS_MULTIMOD_CANDIDATE_COMMIT -or
    $package.plan_sha256 -cne [string]$env:QPLS_MULTIMOD_PLAN_SHA256 -or
    $package.binding_sha256 -cne [string]$env:QPLS_MULTIMOD_BINDING_SHA256 -or
    [string]$package.prepackage_authority_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
    [string]$package.authority_binding_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
    [string]$package.prepackage_manifest_set_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
    $package.installed_candidate_is_created_only_by_fresh_nsis_install -ne $true -or
    $package.qualification_harness.contract -cne "qpls.v256.multimod.build-only-packaged-qualification-harness.v1" -or
    $package.qualification_harness.cargo_feature -cne "multimod-qualification-harness" -or
    $package.qualification_harness.compile_time_only -ne $true -or
    $package.qualification_harness.embedded_candidate_authority_required -ne $true -or
    $package.qualification_harness.request_or_runtime_environment_authority_forbidden -ne $true -or
    $package.qualification_harness.executable_specific_smoke_required -ne $true -or
    $package.qualification_harness.later_harness_disabled_rebuild_not_covered -ne $true -or
    $package.qualification_harness.unmerged_review_candidate -ne $true
) { throw "Package receipt identity or install policy is invalid." }
$portableRows = @($package.artifacts | Where-Object { $_.role -ceq "portable" })
$setupRows = @($package.artifacts | Where-Object { $_.role -ceq "setup" })
if ($portableRows.Count -ne 1 -or $setupRows.Count -ne 1 -or @($package.artifacts).Count -ne 2) {
    throw "Package receipt must contain exactly one portable executable and one NSIS setup."
}
$portable = [IO.Path]::GetFullPath([string]$portableRows[0].path)
$setup = [IO.Path]::GetFullPath([string]$setupRows[0].path)
foreach ($row in @($portableRows[0], $setupRows[0])) {
    $path = [IO.Path]::GetFullPath([string]$row.path)
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-LowerSha256 $path) -cne [string]$row.sha256 -or [long](Get-Item -LiteralPath $path).Length -ne [long]$row.size) {
        throw "Package artifact bytes differ from the source-bound receipt: $path"
    }
}

$installRoot = Join-Path $outputDirectory "isolated-nsis-install"
$installReceipt = $null
$candidate = $portable
$candidateSha = [string]$portableRows[0].sha256
$uninstaller = $null
$installWasCreated = $false
$installCleaned = $Kind -eq "portable"
$diskSnapshots = [System.Collections.Generic.List[object]]::new()
$diskSnapshots.Add((Get-DiskSnapshot "before packaged $Kind smoke"))
Assert-PhaseBudget -Phase "science"
if ($Kind -eq "installed") {
    if (Test-Path -LiteralPath $installRoot) { throw "Fresh NSIS destination already exists: $installRoot" }
    if (@(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $_.Name -in @("QuickPLS.exe", "quickpls-desktop.exe") }).Count -ne 0) {
        throw "A QuickPLS process is active; isolated NSIS installation will not alter installation state."
    }
    if (@(Get-QuickPlsRegistrations).Count -ne 0) {
        throw "An existing QuickPLS uninstall registration exists; use a clean Windows account or VM for installed qualification."
    }
    $installParent = Split-Path -Parent $installRoot
    New-Item -ItemType Directory -Path $installParent -Force | Out-Null
    try {
        $installerArguments = @("/S", "/D=$installRoot")
        $installer = Start-Process -FilePath $setup -ArgumentList $installerArguments -WorkingDirectory $installParent -WindowStyle Hidden -PassThru
        Wait-ExactProcess $installer 900 "isolated NSIS installation" -Phase "science"
        $installWasCreated = Test-Path -LiteralPath $installRoot -PathType Container
        if (-not $installWasCreated) { throw "NSIS reported success without creating its fresh destination." }
        $candidates = @(Get-ChildItem -LiteralPath $installRoot -Recurse -File -Filter "*.exe" | Where-Object { $_.Name -notmatch '^(unins|uninstall)' -and $_.VersionInfo.ProductVersion -and $_.VersionInfo.ProductVersion.StartsWith("2.56.0", [StringComparison]::Ordinal) })
        if ($candidates.Count -ne 1) { throw "Expected one installed QuickPLS 2.56 executable; found $($candidates.Count)." }
        $candidate = $candidates[0].FullName
        $candidateSha = Get-LowerSha256 $candidate
        $equivalence = Get-InstalledPortableEquivalence $candidate $portable
        $uninstallers = @(Get-ChildItem -LiteralPath $installRoot -Recurse -File -Filter "*.exe" | Where-Object { $_.Name -match '^(unins|uninstall)' })
        if ($uninstallers.Count -ne 1) { throw "Expected one exact NSIS uninstaller; found $($uninstallers.Count)." }
        $uninstaller = $uninstallers[0].FullName
        $installReceipt = [ordered]@{
            schema_version = 1
            receipt_id = "qpls.v256.multimod.isolated-nsis-install.v1"
            installation_kind = "nsis_silent_fresh_destination"
            candidate_commit_sha = [string]$package.candidate_commit_sha
            version = [string]$package.version
            package_receipt_sha256 = Get-LowerSha256 $receiptPath
            setup_path = $setup
            setup_sha256 = Get-LowerSha256 $setup
            portable_path = $portable
            portable_sha256 = Get-LowerSha256 $portable
            install_root = [IO.Path]::GetFullPath($installRoot)
            install_root_preexisting = $false
            installer_pid = $installer.Id
            installer_arguments = $installerArguments
            installer_exit_code = $installer.ExitCode
            installed_executable = $candidate
            installed_executable_sha256 = $candidateSha
            installed_portable_equivalence = $equivalence
            uninstall_executable = $uninstaller
        }
    } catch {
        $setupError = $_
        $setupCleanupError = $null
        try {
            if (Test-Path -LiteralPath $installRoot -PathType Container) {
                $cleanupUninstallers = @(Get-ChildItem -LiteralPath $installRoot -Recurse -File -Filter "*.exe" | Where-Object { $_.Name -match '^(unins|uninstall)' })
                if ($cleanupUninstallers.Count -ne 1) { throw "Setup failure left no unique NSIS uninstaller." }
                $cleanupUninstall = Start-Process -FilePath $cleanupUninstallers[0].FullName -ArgumentList @("/S") -WorkingDirectory $installRoot -WindowStyle Hidden -PassThru
                Wait-ExactProcess $cleanupUninstall 900 "failed-setup NSIS cleanup" -Phase "cleanup"
            }
        } catch { $setupCleanupError = $_ }
        if ($setupCleanupError) { throw "$($setupError.Exception.Message); failed-setup cleanup also failed: $($setupCleanupError.Exception.Message)" }
        throw $setupError
    }
}

$peSubsystem = Get-PeSubsystem $candidate
$packageReceiptSha256 = Get-LowerSha256 $receiptPath

$port = 19000 + ([int]($Seed % 1000)) + $(if ($Kind -eq "installed") { 0 } else { 1 })
$endpoint = "http://127.0.0.1:$port"
$profile = Join-Path $outputDirectory "$Kind-webview-profile"
if (Test-Path -LiteralPath $profile) { throw "Packaged smoke WebView profile must be new: $profile" }
New-Item -ItemType Directory -Path $profile | Out-Null

function Test-Cdp {
    try { $null = Invoke-RestMethod -Uri "$endpoint/json/version" -TimeoutSec 1; return $true } catch { return $false }
}
function Wait-Cdp(
    [bool]$Open,
    [ValidateSet("science", "cleanup")][string]$Phase,
    [ValidateRange(1, 60)][int]$MaximumSeconds = 45
) {
    $phaseMilliseconds = Get-RemainingPhaseMilliseconds -Phase $Phase
    $deadline = [DateTimeOffset]::UtcNow.AddMilliseconds(
        [Math]::Min([int64]$MaximumSeconds * 1000L, $phaseMilliseconds)
    )
    while ([DateTimeOffset]::UtcNow -lt $deadline) {
        if ((Test-Cdp) -eq $Open) { return }
        Start-Sleep -Milliseconds 250
    }
    throw "Candidate CDP endpoint did not become $(if ($Open) { 'ready' } else { 'closed' })."
}

if (Test-Cdp) { throw "Refusing to attach to a pre-existing CDP endpoint: $endpoint" }
$sameExecutable = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $_.ExecutablePath -and [IO.Path]::GetFullPath([string]$_.ExecutablePath).Equals($candidate, [StringComparison]::OrdinalIgnoreCase) })
if ($sameExecutable.Count -gt 0) { throw "The exact $Kind candidate is already running." }

$oldBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$oldUserData = $env:WEBVIEW2_USER_DATA_FOLDER
$oldCdp = $env:QUICKPLS_CDP_ENDPOINT
$process = $null
$nodeJob = $null
$driverScientificDeadlineEpochMilliseconds = $null
$driverStartedAtUtc = $null
$cleanupSequence = [Collections.Generic.List[string]]::new()
$cleanup = [ordered]@{
    contract_id = "qpls.v256.multimod.packaged-process-cleanup.v1"
    cleanup_started_at_utc = $null
    cleanup_completed_at_utc = $null
    cleanup_elapsed_milliseconds = $null
    node_root_started = $false
    node_pid = $null
    node_process_tree_supervised = $true
    node_forced_termination = $false
    node_process_tree_terminated = $false
    exact_root_started = $false
    exact_root_terminated = $false
    candidate_process_tree_termination_requested = $false
    active_work_cancelled_via_candidate_termination = $false
    endpoint_closed = $false
    candidate_termination_before_uninstall = $false
    exact_nsis_uninstall_attempted = $false
    nsis_uninstall_exit_code = $null
    install_root_removed = $installCleaned
    registration_removed = $installCleaned
    cleanup_sequence = $cleanupSequence
}
$operationError = $null
$cleanupError = $null
try {
    Assert-PhaseBudget -Phase "science"
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$port --disable-background-networking --disable-component-update --disable-sync --metrics-recording-only --disable-quic --proxy-server=http://127.0.0.1:9"
    $env:WEBVIEW2_USER_DATA_FOLDER = $profile
    $env:QUICKPLS_CDP_ENDPOINT = $endpoint
    $process = Start-Process -FilePath $candidate -WorkingDirectory (Split-Path -Parent $candidate) -WindowStyle Hidden -PassThru
    $cleanup.exact_root_started = $true
    Wait-Cdp $true -Phase "science" -MaximumSeconds 45
    $node = (Get-Command node -ErrorAction Stop).Source
    $remainingScienceMilliseconds = Get-RemainingPhaseMilliseconds -Phase "science"
    $driverScientificDeadlineEpochMilliseconds = `
        [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() + $remainingScienceMilliseconds
    $driverArguments = @(
        $driver,
        "--endpoint", $endpoint,
        "--kind", $Kind,
        "--candidate-path", $candidate,
        "--candidate-sha256", $candidateSha,
        "--candidate-pid", ([string]$process.Id),
        "--candidate-commit", ([string]$package.candidate_commit_sha),
        "--candidate-version", ([string]$package.version),
        "--plan-sha256", ([string]$package.plan_sha256),
        "--binding-sha256", ([string]$package.binding_sha256),
        "--authority-document-sha256", ([string]$package.prepackage_authority_sha256),
        "--authority-binding-sha256", ([string]$package.authority_binding_sha256),
        "--prepackage-manifest-set-sha256", ([string]$package.prepackage_manifest_set_sha256),
        "--package-receipt-sha256", $packageReceiptSha256,
        "--scientific-deadline-epoch-ms", ([string]$driverScientificDeadlineEpochMilliseconds),
        "--seed", ([string]$Seed),
        "--output", $driverOutputPath
    )
    $driverStartedAtUtc = [DateTimeOffset]::UtcNow
    $nodeJob = Start-SupervisedProcess -FileName $node -Arguments $driverArguments `
        -StdoutPath $driverStdoutPath -StderrPath $driverStderrPath
    $cleanup.node_root_started = $true
    $cleanup.node_pid = $nodeJob.Process.Id
    Wait-SupervisedDriver -Job $nodeJob `
        -ScientificDeadlineEpochMilliseconds $driverScientificDeadlineEpochMilliseconds
} catch { $operationError = $_ }
finally {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArguments
    $env:WEBVIEW2_USER_DATA_FOLDER = $oldUserData
    $env:QUICKPLS_CDP_ENDPOINT = $oldCdp
    $cleanupClock = [Diagnostics.Stopwatch]::StartNew()
    $terminationClock = [Diagnostics.Stopwatch]::StartNew()
    $cleanupFailures = [Collections.Generic.List[string]]::new()
    $cleanup.cleanup_started_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
    try {
        if ($nodeJob) {
            $terminationSeconds = 60 - [int][Math]::Ceiling($terminationClock.Elapsed.TotalSeconds)
            if ($terminationSeconds -lt 1) { throw "Cleanup left no time for mandatory Node process-tree termination." }
            Stop-SupervisedDriver -Job $nodeJob -Reason "wrapper_finalization" `
                -MaximumSeconds ([Math]::Min(30, $terminationSeconds))
        }
    } catch { [void]$cleanupFailures.Add("Node driver cleanup: $($_.Exception.Message)") }
    if ($nodeJob) {
        $cleanup.node_forced_termination = [bool]$nodeJob.ForcedTermination
        $cleanup.node_process_tree_terminated = $nodeJob.Process.HasExited
        if ($cleanup.node_process_tree_terminated) { [void]$cleanupSequence.Add("node_driver_process_tree_closed") }
    }
    try {
        $cleanup.candidate_process_tree_termination_requested = $true
        if ($process) {
            $terminationSeconds = 60 - [int][Math]::Ceiling($terminationClock.Elapsed.TotalSeconds)
            if ($terminationSeconds -lt 1) { throw "Node cleanup left no time for mandatory candidate process-tree termination." }
            Stop-ExactProcessTree -Process $process -Operation "exact packaged QuickPLS candidate" `
                -Phase wrapper -MaximumSeconds $terminationSeconds
        }
        else {
            $cleanup.exact_root_terminated = $true
            $cleanup.active_work_cancelled_via_candidate_termination = $true
            [void]$cleanupSequence.Add("candidate_process_tree_absent")
        }
    } catch { [void]$cleanupFailures.Add("Candidate process-tree cleanup: $($_.Exception.Message)") }
    if ($process) {
        $cleanup.exact_root_terminated = $process.HasExited
        $cleanup.active_work_cancelled_via_candidate_termination = $cleanup.exact_root_terminated
        if ($cleanup.exact_root_terminated) { [void]$cleanupSequence.Add("candidate_process_tree_terminated") }
    }
    try {
        $terminationSeconds = 60 - [int][Math]::Ceiling($terminationClock.Elapsed.TotalSeconds)
        if ($terminationSeconds -lt 1) { throw "Process-tree cleanup left no time to verify candidate endpoint closure." }
        Wait-Cdp $false -Phase "cleanup" -MaximumSeconds ([Math]::Min(45, $terminationSeconds))
        $cleanup.endpoint_closed = $true
        [void]$cleanupSequence.Add("candidate_endpoint_closed")
    } catch { [void]$cleanupFailures.Add("Candidate endpoint cleanup: $($_.Exception.Message)") }
    $cleanup.candidate_termination_before_uninstall = `
        $cleanup.exact_root_terminated -and $cleanup.endpoint_closed
    if ($Kind -eq "installed" -and $installWasCreated) {
        if (-not $cleanup.candidate_termination_before_uninstall -or
            ($nodeJob -and -not $cleanup.node_process_tree_terminated)) {
            [void]$cleanupFailures.Add("Candidate and Node process trees must terminate before NSIS uninstall.")
        }
        elseif (-not $uninstaller -or -not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
            [void]$cleanupFailures.Add("Exact NSIS uninstaller is unavailable for cleanup.")
        }
        else {
            try {
                $cleanup.exact_nsis_uninstall_attempted = $true
                $uninstall = Start-Process -FilePath $uninstaller -ArgumentList @("/S") -WorkingDirectory $installRoot -WindowStyle Hidden -PassThru
                Wait-ExactProcess $uninstall 900 "isolated NSIS uninstall" -Phase "cleanup"
                $cleanup.nsis_uninstall_exit_code = $uninstall.ExitCode
                [void]$cleanupSequence.Add("exact_nsis_uninstall_completed")
            } catch { [void]$cleanupFailures.Add("Exact NSIS uninstall: $($_.Exception.Message)") }
        }
        try {
            $verificationDeadline = [DateTimeOffset]::UtcNow.AddMilliseconds(
                [Math]::Min(60000L, (Get-RemainingPhaseMilliseconds -Phase "cleanup"))
            )
            while ((Test-Path -LiteralPath $installRoot) -and [DateTimeOffset]::UtcNow -lt $verificationDeadline) { Start-Sleep -Milliseconds 250 }
            $cleanup.install_root_removed = -not (Test-Path -LiteralPath $installRoot)
            $cleanup.registration_removed = @(Get-QuickPlsRegistrations).Count -eq 0
            if (-not $cleanup.install_root_removed -or -not $cleanup.registration_removed) { throw "Isolated NSIS uninstall did not remove its destination and registration." }
            [void]$cleanupSequence.Add("nsis_install_root_and_registration_removed")
        } catch { [void]$cleanupFailures.Add("NSIS cleanup verification: $($_.Exception.Message)") }
    }
    if ($cleanupFailures.Count -gt 0) {
        $cleanupError = [Exception]::new(($cleanupFailures -join " | "))
    }
    try {
        $cleanupClock.Stop()
        $cleanup.cleanup_completed_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
        $cleanup.cleanup_elapsed_milliseconds = [int64]$cleanupClock.ElapsedMilliseconds
    } catch { if (-not $cleanupError) { $cleanupError = $_.Exception } }
}

$diskSnapshots.Add((Get-DiskSnapshot "after packaged $Kind cleanup"))
if ($operationError -and $cleanupError) { throw "$($operationError.Exception.Message); cleanup also failed: $($cleanupError.Message)" }
if ($operationError) { throw $operationError }
if ($cleanupError) { throw $cleanupError }
Assert-PhaseBudget -Phase "wrapper"
if (-not (Test-Path -LiteralPath $driverOutputPath -PathType Leaf)) { throw "Packaged smoke driver did not create its report." }
$report = Get-Content -LiteralPath $driverOutputPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 60
if (
    $report.passed -ne $true -or
    $report.report_id -cne "qpls.v256.multimod.packaged-offline-production-smoke.v1" -or
    $report.qualification_coverage_complete -ne $true -or
    $report.package_kind -cne $Kind -or
    $report.candidate_commit_sha -cne [string]$package.candidate_commit_sha -or
    $report.candidate_version -cne [string]$package.version -or
    $report.plan_sha256 -cne [string]$package.plan_sha256 -or
    $report.binding_sha256 -cne [string]$package.binding_sha256 -or
    $report.authority_document_sha256 -cne [string]$package.prepackage_authority_sha256 -or
    $report.authority_binding_sha256 -cne [string]$package.authority_binding_sha256 -or
    $report.manifest_set_sha256 -cne [string]$package.prepackage_manifest_set_sha256 -or
    $report.package_receipt_sha256 -cne $packageReceiptSha256 -or
    [UInt64]$report.seed -ne $Seed -or
    [int64]$report.timing.scientific_deadline_epoch_ms -ne $driverScientificDeadlineEpochMilliseconds -or
    $report.timing.poll_deadlines_clamped_to_family_and_driver -ne $true -or
    $report.timing.late_exit_rejection_enabled -ne $true -or
    [int64]$report.timing.completed_epoch_ms -gt $driverScientificDeadlineEpochMilliseconds -or
    [int64]$nodeJob.ExitEpochMilliseconds -gt $driverScientificDeadlineEpochMilliseconds -or
    $report.candidate.sha256 -cne $candidateSha -or
    [int]$report.candidate.pid -ne $process.Id -or
    $report.offline.passed -ne $true -or
    $report.candidate_receipt_tamper_failed_closed -ne $true -or
    $report.cancellation_recovery_verified -ne $true -or
    $report.standard_surface_verified -ne $true -or
    $report.labs_opt_in_not_required -ne $true -or
    $report.lab_badge_absent -ne $true -or
    $report.harness.compile_time_feature_required -ne $true -or
    $report.harness.embedded_candidate_authority_required -ne $true -or
    $report.harness.runtime_or_request_authority_injection -ne $false -or
    $report.harness.unmerged_review_candidate -ne $true -or
    $report.harness.later_harness_disabled_rebuild_covered -ne $false -or
    @($report.families).Count -ne 4 -or
    @($report.families | Where-Object {
        $_.valid_run_completed -ne $true -or
        $_.result_identity_verified -ne $true -or
        $_.archive_reopened -ne $true -or
        $_.semantic_export_readback -ne $true -or
        $_.sidecar_integrity_verified -ne $true -or
        @($_.exports | Where-Object { $_.format -in @("csv", "xlsx", "json", "html", "pdf") }).Count -lt 5 -or
        -not $_.raw_sidecar_export.strictReopenValidated
    }).Count -ne 0 -or
    $cleanup.node_process_tree_supervised -ne $true -or
    $cleanup.node_process_tree_terminated -ne $true -or
    $cleanup.candidate_process_tree_termination_requested -ne $true -or
    $cleanup.active_work_cancelled_via_candidate_termination -ne $true -or
    $cleanup.exact_root_terminated -ne $true -or
    $cleanup.endpoint_closed -ne $true -or
    $cleanup.candidate_termination_before_uninstall -ne $true -or
    $cleanup.install_root_removed -ne $true -or
    $cleanup.registration_removed -ne $true
) { throw "Packaged smoke report, isolated install, or exact-process cleanup is invalid." }
if ($Kind -eq "installed" -and
    ($cleanup.exact_nsis_uninstall_attempted -ne $true -or
     $null -eq $cleanup.nsis_uninstall_exit_code -or
     [int]$cleanup.nsis_uninstall_exit_code -ne 0)) {
    throw "Installed packaged smoke lacks a successful exact NSIS uninstall receipt."
}
if ($Kind -eq "portable" -and
    ($cleanup.exact_nsis_uninstall_attempted -ne $false -or $null -ne $cleanup.nsis_uninstall_exit_code)) {
    throw "Portable packaged smoke must not claim an NSIS uninstall."
}
$timingProvenance = [ordered]@{
    contract_id = "qpls.v256.multimod.packaged-hard-time-budget.v1"
    wrapper_started_at_utc = $wrapperStartedAtUtc.ToString("o")
    driver_started_at_utc = $driverStartedAtUtc.ToString("o")
    driver_exit_epoch_ms = [int64]$nodeJob.ExitEpochMilliseconds
    scientific_deadline_epoch_ms = [int64]$driverScientificDeadlineEpochMilliseconds
    wrapper_maximum_seconds = $wrapperMaximumSeconds
    scientific_maximum_seconds = $scientificMaximumSeconds
    minimum_cleanup_reserve_seconds = $minimumCleanupReserveSeconds
    finalization_reserve_seconds = $finalizationReserveSeconds
    poll_deadlines_clamped_to_family_and_driver = $true
    late_exit_rejection_enabled = $true
    wrapper_elapsed_milliseconds = [int64]$wrapperClock.ElapsedMilliseconds
}
$report | Add-Member -NotePropertyName process_cleanup -NotePropertyValue $cleanup -Force
$report | Add-Member -NotePropertyName timing_provenance -NotePropertyValue $timingProvenance -Force
$report | Add-Member -NotePropertyName package_receipt_sha256 -NotePropertyValue $packageReceiptSha256 -Force
$report | Add-Member -NotePropertyName isolated_install -NotePropertyValue $installReceipt -Force
$report | Add-Member -NotePropertyName disk_snapshots -NotePropertyValue @($diskSnapshots) -Force
$report | Add-Member -NotePropertyName pe_subsystem -NotePropertyValue $peSubsystem -Force
$productionEvidencePath = Join-Path $outputDirectory "packaged-production-workflow-evidence.json"
if (Test-Path -LiteralPath $productionEvidencePath) { throw "Packaged production evidence already exists: $productionEvidencePath" }
Write-JsonAtomic $productionEvidencePath $report
Remove-Item -LiteralPath $driverOutputPath

$familyReceipts = @(
    foreach ($family in $report.families) {
        [ordered]@{
            family_id = [string]$family.family_id
            valid_run_completed = $true
            result_identity_verified = $true
            archive_reopened = $true
            semantic_export_readback = $true
            sidecar_integrity_verified = $true
        }
    }
)
$runtimeReceipt = [ordered]@{
    schema_version = 1
    receipt_kind = "qpls_multimod_runtime_promotion_smoke_v1"
    lane = $Kind
    candidate_commit_sha = [string]$package.candidate_commit_sha
    candidate_version = [string]$package.version
    plan_sha256 = [string]$package.plan_sha256
    binding_sha256 = [string]$package.binding_sha256
    authority_document_sha256 = [string]$package.prepackage_authority_sha256
    authority_binding_sha256 = [string]$package.authority_binding_sha256
    manifest_set_sha256 = [string]$package.prepackage_manifest_set_sha256
    package_receipt_sha256 = $packageReceiptSha256
    executable_path = [IO.Path]::GetFullPath($candidate)
    executable_sha256 = $candidateSha
    offline = $true
    console_window_absent = $peSubsystem.console_subsystem_absent
    qualification_state = "release_qualified_candidate"
    unqualified_authority_fails_closed = $true
    standard_surface_verified = [bool]$report.standard_surface_verified
    labs_opt_in_not_required = [bool]$report.labs_opt_in_not_required
    lab_badge_absent = [bool]$report.lab_badge_absent
    post_evidence_source_change_required = $false
    qualification_coverage_complete = $true
    qualification_harness_contract = "qpls.v256.multimod.build-only-packaged-qualification-harness.v1"
    unmerged_review_candidate = $true
    later_harness_disabled_rebuild_covered = $false
    production_workflow_evidence_path = [IO.Path]::GetFullPath($productionEvidencePath)
    production_workflow_evidence_sha256 = Get-LowerSha256 $productionEvidencePath
    families = $familyReceipts
    cancellation_recovery_verified = $true
    isolated_nsis_install_verified = ($Kind -eq "installed" -and $installWasCreated -and $installReceipt.installed_portable_equivalence.passed)
    uninstall_cleanup_verified = ($Kind -eq "installed" -and $cleanup.install_root_removed -and $cleanup.registration_removed)
    timing_provenance = $timingProvenance
    cleanup_provenance = $cleanup
}
Assert-PhaseBudget -Phase "wrapper"
$runtimeStagingPath = Join-Path $outputDirectory `
    (".runtime-promotion-{0}.{1}.staged.json" -f $Kind, [Guid]::NewGuid().ToString("N"))
Write-JsonAtomic $runtimeStagingPath $runtimeReceipt
Assert-PhaseBudget -Phase "wrapper"
Move-Item -LiteralPath $runtimeStagingPath -Destination $outputPath
Assert-PhaseBudget -Phase "wrapper"
$runtimeReceipt | ConvertTo-Json -Depth 30
