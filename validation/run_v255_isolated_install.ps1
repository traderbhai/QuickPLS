[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseArtifactReportPath,
    [string]$InstallRoot = "",
    [string]$ReceiptPath = ""
)

# Installs the exact NSIS setup artifact into a brand-new isolated destination.
# It never uninstalls or overwrites an existing installation. The receipt binds
# the installer invocation and installed executable to the candidate build
# report and its clean-source commit.

$ErrorActionPreference = "Stop"
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$artifactBase = [IO.Path]::GetFullPath((Join-Path $root "target\release\artifacts"))
$artifactBasePrefix = $artifactBase.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$minimumFreeGiB = 20.0
$reportPath = [IO.Path]::GetFullPath($ReleaseArtifactReportPath)
$stamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
$installBase = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "QuickPLSValidation"
$destination = if ([string]::IsNullOrWhiteSpace($InstallRoot)) { Join-Path $installBase "v255-install-$stamp" } else { [IO.Path]::GetFullPath($InstallRoot) }
$receipt = if ([string]::IsNullOrWhiteSpace($ReceiptPath)) { Join-Path $root "target\release\artifacts\v255_isolated_install_$stamp.json" } else { [IO.Path]::GetFullPath($ReceiptPath) }

if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) { throw "Release artifact report is missing: $reportPath" }
if (-not $reportPath.StartsWith($artifactBasePrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "ReleaseArtifactReportPath must remain beneath the repository target/release/artifacts directory." }
$reportSha = (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToUpperInvariant()
if (Test-Path -LiteralPath $destination) { throw "InstallRoot must be a brand-new destination: $destination" }
if (Test-Path -LiteralPath $receipt) { throw "ReceiptPath already exists: $receipt" }
if (-not $receipt.StartsWith($artifactBasePrefix, [StringComparison]::OrdinalIgnoreCase)) { throw "ReceiptPath must remain beneath the repository target/release/artifacts directory." }
if ($destination -eq [IO.Path]::GetPathRoot($destination) -or [string]::IsNullOrWhiteSpace((Split-Path -Parent $destination))) {
    throw "InstallRoot must not be a filesystem root: $destination"
}
if ($destination -match '\s') {
    throw "InstallRoot may not contain whitespace because NSIS requires /D=<path> as one unquoted final argument: $destination"
}

function Resolve-ReportPath([string]$Declared) {
    if ([string]::IsNullOrWhiteSpace($Declared)) { throw "Release report contains an empty artifact path." }
    if ([IO.Path]::IsPathRooted($Declared)) { return [IO.Path]::GetFullPath($Declared) }
    [IO.Path]::GetFullPath((Join-Path $root $Declared))
}

function Get-DiskSnapshot([string]$LabelText) {
    $drives = [ordered]@{}
    foreach ($name in @("C", "D")) {
        $free = (Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
        if ($free -le $minimumFreeGiB) {
            throw "${LabelText}: drive $name has $([math]::Round($free, 3)) GiB free; more than $minimumFreeGiB GiB is required."
        }
        $drives[$name] = [math]::Round($free, 3)
    }
    [ordered]@{ label = $LabelText; captured_at = [DateTime]::UtcNow.ToString("o"); drives = $drives }
}

function Write-Utf8NoBom([string]$PathValue, [string]$TextValue) {
    [IO.File]::WriteAllText($PathValue, $TextValue, [Text.UTF8Encoding]::new($false))
}

function Get-InstalledPortableEquivalence([string]$InstalledPath, [string]$PortablePath) {
    $portableMarker = "__TAURI_BUNDLE_TYPE_VAR_UNK"
    $installedMarker = "__TAURI_BUNDLE_TYPE_VAR_NSS"
    $portableMarkerBytes = [Text.Encoding]::ASCII.GetBytes($portableMarker)
    $installedMarkerBytes = [Text.Encoding]::ASCII.GetBytes($installedMarker)
    if ($portableMarkerBytes.Length -ne $installedMarkerBytes.Length) {
        throw "The Tauri installed/portable marker contract has unequal marker lengths."
    }

    $portableBytes = [IO.File]::ReadAllBytes($PortablePath)
    $installedBytes = [IO.File]::ReadAllBytes($InstalledPath)
    if ($portableBytes.Length -ne $installedBytes.Length) {
        throw "Installed and portable executables differ in length."
    }

    $portableText = [Text.Encoding]::ASCII.GetString($portableBytes)
    $installedText = [Text.Encoding]::ASCII.GetString($installedBytes)
    $portableOffset = $portableText.IndexOf($portableMarker, [StringComparison]::Ordinal)
    $installedOffset = $installedText.IndexOf($installedMarker, [StringComparison]::Ordinal)
    if (
        $portableOffset -lt 0 -or
        $installedOffset -lt 0 -or
        $portableText.IndexOf($portableMarker, $portableOffset + 1, [StringComparison]::Ordinal) -ge 0 -or
        $installedText.IndexOf($installedMarker, $installedOffset + 1, [StringComparison]::Ordinal) -ge 0
    ) {
        throw "Installed and portable executables must each contain exactly one expected Tauri bundle marker."
    }
    if ($portableOffset -ne $installedOffset) {
        throw "Installed and portable Tauri bundle markers occur at different offsets."
    }

    $normalizedInstalledBytes = [byte[]]($installedBytes.Clone())
    for ($index = 0; $index -lt $portableMarkerBytes.Length; $index++) {
        $normalizedInstalledBytes[$installedOffset + $index] = $portableMarkerBytes[$index]
    }
    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        $normalizedInstalledSha = ([BitConverter]::ToString($sha256.ComputeHash($normalizedInstalledBytes))).Replace("-", "")
        $portableBytesSha = ([BitConverter]::ToString($sha256.ComputeHash($portableBytes))).Replace("-", "")
    } finally {
        $sha256.Dispose()
    }
    if ($normalizedInstalledSha -ne $portableBytesSha) {
        throw "Installed and portable executables differ outside the single Tauri NSIS bundle marker."
    }

    [ordered]@{
        kind = "tauri_nsis_bundle_marker_variant_v1"
        passed = $true
        portable_marker = $portableMarker
        installed_marker = $installedMarker
        marker_offset = $portableOffset
        all_other_bytes_identical = $true
    }
}

$existingProcesses = @(
    Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @("QuickPLS.exe", "quickpls-desktop.exe") }
)
if ($existingProcesses.Count -ne 0) {
    throw "A QuickPLS desktop process is already running. The isolated installer will not modify installation state while a user instance is active."
}
$uninstallRegistryPaths = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
$existingRegistrations = @(
    foreach ($registryPath in $uninstallRegistryPaths) {
        Get-ItemProperty -Path $registryPath -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -and [string]$_.DisplayName -match '^QuickPLS(?:\s|$)' }
    }
)
if ($existingRegistrations.Count -ne 0) {
    throw "An existing registered QuickPLS installation was found. Run installed-candidate evidence in a clean Windows account/VM; this harness will not replace or uninstall the user's copy."
}

$release = Get-Content -LiteralPath $reportPath -Raw -Encoding UTF8 | ConvertFrom-Json
if (
    $release.schema_version -ne 3 -or
    $release.passed -ne $true -or
    $release.version -ne "2.55.0" -or
    $release.source.worktree_clean -ne $true -or
    [string]$release.source.commit -notmatch '^[0-9a-f]{40}$' -or
    $release.build.passed -ne $true -or
    $release.build.source.commit -ne $release.source.commit -or
    $release.build.environment.CARGO_INCREMENTAL -ne "0" -or
    [double]$release.build.minimum_free_gib -ne 20.0
) {
    throw "Release artifact report is not a passing source-bound 2.55 candidate report."
}
$setupRows = @($release.artifacts | Where-Object { $_.role -eq "setup" })
$portableRows = @($release.artifacts | Where-Object { $_.role -eq "portable" })
if ($setupRows.Count -ne 1 -or $portableRows.Count -ne 1) { throw "Release report must contain exactly one setup and one portable artifact." }
$setupRow = $setupRows[0]
$portableRow = $portableRows[0]
$setup = Resolve-ReportPath ([string]$setupRow.path)
$portable = Resolve-ReportPath ([string]$portableRow.path)
foreach ($candidate in @($setup, $portable)) {
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "Release artifact is missing: $candidate" }
}
$setupSha = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToUpperInvariant()
$portableSha = (Get-FileHash -LiteralPath $portable -Algorithm SHA256).Hash.ToUpperInvariant()
if ($setupSha -ne ([string]$setupRow.sha256).ToUpperInvariant() -or $setupRow.copy_verified -ne $true -or $portableSha -ne ([string]$portableRow.sha256).ToUpperInvariant() -or $portableRow.copy_verified -ne $true) {
    throw "Current setup or portable bytes do not match the release artifact report."
}

$diskBefore = Get-DiskSnapshot "before isolated NSIS install"
$installParent = Split-Path -Parent $destination
New-Item -ItemType Directory -Path $installParent -Force | Out-Null
$arguments = @("/S", "/D=$destination")
$started = [DateTime]::UtcNow.ToString("o")
$installer = Start-Process -FilePath $setup -ArgumentList $arguments -WorkingDirectory $installParent -WindowStyle Hidden -PassThru
$installer.WaitForExit()
$completed = [DateTime]::UtcNow.ToString("o")
if ($installer.ExitCode -ne 0) { throw "NSIS setup exited with code $($installer.ExitCode)." }
$setupShaAfter = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToUpperInvariant()
if ($setupShaAfter -ne $setupSha) { throw "The setup artifact changed while the isolated installation was running." }

$deadline = [DateTime]::UtcNow.AddSeconds(30)
do {
    $installedCandidates = @(
        Get-ChildItem -LiteralPath $destination -Recurse -File -Filter "*.exe" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Name -notmatch '^(unins|uninstall)' -and
                $_.VersionInfo.ProductVersion -and
                $_.VersionInfo.ProductVersion.StartsWith("2.55.0", [StringComparison]::Ordinal)
            }
    )
    if ($installedCandidates.Count -eq 1) { break }
    Start-Sleep -Milliseconds 250
} while ([DateTime]::UtcNow -lt $deadline)
if ($installedCandidates.Count -ne 1) {
    throw "Expected exactly one installed QuickPLS 2.55 executable below $destination; found $($installedCandidates.Count)."
}
$installed = $installedCandidates[0].FullName
$installedSha = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToUpperInvariant()
if ($installedSha -eq $portableSha) {
    throw "Installed and portable executable hashes must be distinct Tauri package identities."
}
$installedPortableEquivalence = Get-InstalledPortableEquivalence $installed $portable
if (
    (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToUpperInvariant() -ne $reportSha -or
    (Get-FileHash -LiteralPath $portable -Algorithm SHA256).Hash.ToUpperInvariant() -ne $portableSha -or
    (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToUpperInvariant() -ne $installedSha
) {
    throw "The release report, portable candidate, or installed candidate changed while installation evidence was collected."
}
$diskAfter = Get-DiskSnapshot "after isolated NSIS install"
New-Item -ItemType Directory -Path (Split-Path -Parent $receipt) -Force | Out-Null
$payload = [ordered]@{
    schema_version = 1
    suite_id = "quickpls_v255_isolated_nsis_install_v1"
    passed = $true
    target_release = "2.55.0"
    installation_kind = "nsis_silent_fresh_destination"
    installation_preflight = [ordered]@{ running_quickpls_processes = 0; existing_quickpls_registrations = 0; user_installation_preserved = $true }
    source_commit = $release.source.commit
    source_tree = $release.source.tree
    source_manifest_sha256 = $release.source.tracked_manifest_sha256
    release_artifact_report = $reportPath
    release_artifact_report_sha256 = $reportSha
    setup = $setup
    setup_sha256 = $setupSha
    setup_report_sha256 = ([string]$setupRow.sha256).ToUpperInvariant()
    install_root = [IO.Path]::GetFullPath($destination)
    install_root_preexisting = $false
    installer_pid = $installer.Id
    installer_arguments = $arguments
    installer_exit_code = $installer.ExitCode
    started_at_utc = $started
    completed_at_utc = $completed
    installed_executable = $installed
    installed_executable_sha256 = $installedSha
    portable_artifact = $portable
    portable_artifact_sha256 = $portableSha
    installed_portable_equivalence = $installedPortableEquivalence
    product_version = $installedCandidates[0].VersionInfo.ProductVersion
    minimum_free_gib = $minimumFreeGiB
    disk_snapshots = @($diskBefore, $diskAfter)
}
Write-Utf8NoBom $receipt (($payload | ConvertTo-Json -Depth 10) + "`n")
$payload | ConvertTo-Json -Depth 10
