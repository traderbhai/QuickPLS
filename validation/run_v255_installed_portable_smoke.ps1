[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ReleaseArtifactReportPath,
    [Parameter(Mandatory = $true)][string]$InstallReceiptPath,
    [Parameter(Mandatory = $true)][string]$VitestReportPath,
    [Parameter(Mandatory = $true)][string]$ConsolidatedReportPath,
    [string]$EvidenceBundlePath = "",
    [string]$EvidenceDir = "",
    [int]$FirstPort = 9255,
    [switch]$WaiveActualWindows200PercentScaling
)

# This runner launches one hidden, isolated candidate at a time. It never
# attaches to a running QuickPLS instance and may only terminate PIDs it started.
# Source collection may use the frozen 17-archive inventory plus the one explicit
# new-run declaration. Publication remains blocked until the curated indexes and
# evidence ZIP are complete and hash-bound.

$ErrorActionPreference = "Stop"
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$rootPrefix = $root.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$minimumFreeGiB = 20.0
$dpiWaiverCaseId = "cross_method:accessibility:actual Windows 200 percent scaling"
$dpiWaiverMetadata = [ordered]@{
    waiver_authority = "product_owner"
    waiver_date = "2026-08-22"
    reason = "product owner explicitly authorized ignoring the 200 percent scaling requirement"
}
$results = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$evidence = if ([string]::IsNullOrWhiteSpace($EvidenceDir)) { Join-Path $results "v255_installed_portable_smoke_$stamp" } else { [IO.Path]::GetFullPath($EvidenceDir) }
$prefix = $results.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (-not $evidence.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $evidence)) { throw "EvidenceDir must be a new child of $results" }
if ($FirstPort -lt 1024 -or $FirstPort -gt 64973) { throw "FirstPort must be between 1024 and 64973 so the complete installed, portable, and cross-method port range remains valid." }
$driver = Join-Path $root "validation\v255_method_evidence_crawler.mjs"
$lifecycleDriver = Join-Path $root "validation\v255_live_calculation_lifecycle_smoke.mjs"
$frozenArchiveCrawler = Join-Path $root "validation\v255_frozen_archive_reopen_crawler.mjs"
$posthocDriver = Join-Path $root "validation\v255_posthoc_minimum_sample_packaged_smoke.mjs"
$namedCaseDriver = Join-Path $root "validation\v255_named_case_driver.mjs"
$namedCaseManifest = Join-Path $root "validation\v255_named_case_manifest.json"
$namedEvidenceIndex = Join-Path $root "validation\v255_named_evidence_index.json"
$namedEvidenceVerifier = Join-Path $root "validation\v255_named_evidence_verifier.py"
$crossMethodWrapper = Join-Path $root "validation\run_v255_cross_method_candidate_smoke.ps1"
$crossMethodDriver = Join-Path $root "validation\v255_cross_method_candidate_driver.mjs"
$crossMethodManifest = Join-Path $root "validation\v255_cross_method_case_manifest.json"
$crossMethodBuilder = Join-Path $root "validation\v255_cross_method_fixture_builder.py"
$crossMethodCloseHelper = Join-Path $root "validation\v255_windows_unsaved_close_guard.py"
$crossMethodCsvFixture = Join-Path $root "validation\fixtures\v255_cross_method_numeric.csv"
$node = (Get-Command node -ErrorAction Stop).Source
$python = (Get-Command python -ErrorAction Stop).Source
$vitestReport = [IO.Path]::GetFullPath($VitestReportPath)
$consolidatedReport = [IO.Path]::GetFullPath($ConsolidatedReportPath)
$releaseArtifactReport = [IO.Path]::GetFullPath($ReleaseArtifactReportPath)
$installReceipt = [IO.Path]::GetFullPath($InstallReceiptPath)
$evidenceBundle = if ([string]::IsNullOrWhiteSpace($EvidenceBundlePath)) { $null } else { [IO.Path]::GetFullPath($EvidenceBundlePath) }
$requiredFiles = @($releaseArtifactReport, $installReceipt, $driver, $lifecycleDriver, $frozenArchiveCrawler, $posthocDriver, $namedCaseDriver, $namedCaseManifest, $namedEvidenceIndex, $namedEvidenceVerifier, $crossMethodWrapper, $crossMethodDriver, $crossMethodManifest, $crossMethodBuilder, $crossMethodCloseHelper, $crossMethodCsvFixture, $node, $python, $vitestReport, $consolidatedReport)
if ($evidenceBundle) { $requiredFiles += $evidenceBundle }
foreach ($file in $requiredFiles) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required file is missing: $file" } }
$namedCaseManifestPayload = Get-Content -LiteralPath $namedCaseManifest -Raw -Encoding UTF8 | ConvertFrom-Json
if (
    $namedCaseManifestPayload.schema_version -ne 1 -or
    $namedCaseManifestPayload.suite_id -ne "quickpls_v255_named_case_manifest_v1" -or
    $namedCaseManifestPayload.target_release -ne "2.55.0" -or
    $namedCaseManifestPayload.status -notin @("pending_collection", "ready")
) { throw "Named-case manifest has an unsupported identity or status." }
$namedCaseManifestReady = $namedCaseManifestPayload.status -eq "ready"
if ($namedCaseManifestReady) {
    $readyNamedCases = @($namedCaseManifestPayload.cases)
    if ($readyNamedCases.Count -eq 0 -or @($readyNamedCases | Where-Object { $_.candidate -eq "installed" }).Count -eq 0 -or @($readyNamedCases | Where-Object { $_.candidate -eq "portable" }).Count -eq 0) {
        throw "A ready named-case manifest must contain at least one installed and one portable case."
    }
}
$releaseArtifactReportHash = (Get-FileHash -LiteralPath $releaseArtifactReport -Algorithm SHA256).Hash.ToUpperInvariant()
$installReceiptHash = (Get-FileHash -LiteralPath $installReceipt -Algorithm SHA256).Hash.ToUpperInvariant()
function Resolve-ReleaseMember([string]$Declared) {
    if ([string]::IsNullOrWhiteSpace($Declared)) { throw "Release artifact report contains an empty path." }
    if ([IO.Path]::IsPathRooted($Declared)) { return [IO.Path]::GetFullPath($Declared) }
    return [IO.Path]::GetFullPath((Join-Path $root $Declared))
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
$releasePayload = Get-Content -LiteralPath $releaseArtifactReport -Raw -Encoding UTF8 | ConvertFrom-Json
if (
    $releasePayload.schema_version -ne 3 -or
    $releasePayload.passed -ne $true -or
    $releasePayload.version -ne "2.55.0" -or
    $releasePayload.source.worktree_clean -ne $true -or
    [string]$releasePayload.source.commit -notmatch '^[0-9a-f]{40}$' -or
    [string]$releasePayload.source.tree -notmatch '^[0-9a-f]{40}$' -or
    $releasePayload.build.passed -ne $true -or
    $releasePayload.build.source.commit -ne $releasePayload.source.commit -or
    $releasePayload.build.environment.CARGO_INCREMENTAL -ne "0" -or
    [double]$releasePayload.build.minimum_free_gib -ne 20.0
) { throw "ReleaseArtifactReportPath is not a passing source-bound 2.55 build report." }
if (@($releasePayload.build.disk_snapshots).Count -ne 2) { throw "Release build receipt must contain exactly two disk snapshots." }
foreach ($snapshot in @($releasePayload.build.disk_snapshots)) {
    if ([double]$snapshot.drives.C -le 20.0 -or [double]$snapshot.drives.D -le 20.0) {
        throw "Release build receipt disk snapshots must keep C and D strictly above 20 GiB."
    }
}
$portableRows = @($releasePayload.artifacts | Where-Object { $_.role -eq "portable" })
$setupRows = @($releasePayload.artifacts | Where-Object { $_.role -eq "setup" })
if ($portableRows.Count -ne 1 -or $setupRows.Count -ne 1) { throw "Release artifact report must contain one portable and one setup artifact." }
$portableFull = Resolve-ReleaseMember ([string]$portableRows[0].path)
$setupFull = Resolve-ReleaseMember ([string]$setupRows[0].path)
$installPayload = Get-Content -LiteralPath $installReceipt -Raw -Encoding UTF8 | ConvertFrom-Json
if (
    $installPayload.schema_version -ne 1 -or
    $installPayload.suite_id -ne "quickpls_v255_isolated_nsis_install_v1" -or
    $installPayload.passed -ne $true -or
    $installPayload.target_release -ne "2.55.0" -or
    $installPayload.installation_kind -ne "nsis_silent_fresh_destination" -or
    $installPayload.install_root_preexisting -ne $false -or
    $installPayload.installer_exit_code -ne 0 -or
    [double]$installPayload.minimum_free_gib -ne 20.0 -or
    $installPayload.source_commit -ne $releasePayload.source.commit -or
    $installPayload.release_artifact_report_sha256 -ne $releaseArtifactReportHash
) { throw "InstallReceiptPath is not a passing isolated installation of this release artifact report." }
foreach ($snapshot in @($installPayload.disk_snapshots)) {
    if ([double]$snapshot.drives.C -le 20.0 -or [double]$snapshot.drives.D -le 20.0) {
        throw "Install receipt disk snapshots must keep C and D strictly above 20 GiB."
    }
}
if (@($installPayload.disk_snapshots).Count -ne 2) { throw "Install receipt must contain exactly before/after disk snapshots." }
$installedFull = [IO.Path]::GetFullPath([string]$installPayload.installed_executable)
$installRootFull = [IO.Path]::GetFullPath([string]$installPayload.install_root)
$installRootPrefix = $installRootFull.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$installerArguments = @($installPayload.installer_arguments)
if (
    -not $installedFull.StartsWith($installRootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
    $installerArguments.Count -ne 2 -or
    $installerArguments[0] -ne "/S" -or
    $installerArguments[1] -ne "/D=$installRootFull"
) { throw "Install receipt does not bind the candidate to its fresh NSIS destination and exact silent arguments." }
foreach ($candidate in @($portableFull, $setupFull, $installedFull)) {
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "Candidate provenance member is missing: $candidate" }
}
$portableHash = (Get-FileHash -LiteralPath $portableFull -Algorithm SHA256).Hash.ToUpperInvariant()
$setupHash = (Get-FileHash -LiteralPath $setupFull -Algorithm SHA256).Hash.ToUpperInvariant()
$installedHash = (Get-FileHash -LiteralPath $installedFull -Algorithm SHA256).Hash.ToUpperInvariant()
$actualInstalledPortableEquivalence = Get-InstalledPortableEquivalence $installedFull $portableFull
$receiptInstalledPortableEquivalence = $installPayload.installed_portable_equivalence
$expectedEquivalenceProperties = @("all_other_bytes_identical", "installed_marker", "kind", "marker_offset", "passed", "portable_marker")
$receiptEquivalenceProperties = @($receiptInstalledPortableEquivalence.PSObject.Properties.Name | Sort-Object)
if (
    $receiptEquivalenceProperties.Count -ne $expectedEquivalenceProperties.Count -or
    @(Compare-Object -ReferenceObject $expectedEquivalenceProperties -DifferenceObject $receiptEquivalenceProperties -SyncWindow 0).Count -ne 0 -or
    -not ($receiptInstalledPortableEquivalence.kind -is [string]) -or
    $receiptInstalledPortableEquivalence.kind -ne $actualInstalledPortableEquivalence.kind -or
    -not ($receiptInstalledPortableEquivalence.passed -is [bool]) -or
    $receiptInstalledPortableEquivalence.passed -ne $true -or
    -not ($receiptInstalledPortableEquivalence.portable_marker -is [string]) -or
    $receiptInstalledPortableEquivalence.portable_marker -ne $actualInstalledPortableEquivalence.portable_marker -or
    -not ($receiptInstalledPortableEquivalence.installed_marker -is [string]) -or
    $receiptInstalledPortableEquivalence.installed_marker -ne $actualInstalledPortableEquivalence.installed_marker -or
    -not ($receiptInstalledPortableEquivalence.marker_offset -is [int] -or $receiptInstalledPortableEquivalence.marker_offset -is [long]) -or
    $receiptInstalledPortableEquivalence.marker_offset -ne $actualInstalledPortableEquivalence.marker_offset -or
    -not ($receiptInstalledPortableEquivalence.all_other_bytes_identical -is [bool]) -or
    $receiptInstalledPortableEquivalence.all_other_bytes_identical -ne $true
) { throw "Install receipt does not exactly bind the revalidated installed/portable Tauri NSIS marker equivalence." }
if (
    $portableHash -ne ([string]$portableRows[0].sha256).ToUpperInvariant() -or
    $portableRows[0].copy_verified -ne $true -or
    $setupHash -ne ([string]$setupRows[0].sha256).ToUpperInvariant() -or
    $setupRows[0].copy_verified -ne $true -or
    $setupHash -ne ([string]$installPayload.setup_sha256).ToUpperInvariant() -or
    -not ([IO.Path]::GetFullPath([string]$installPayload.portable_artifact)).Equals($portableFull, [StringComparison]::OrdinalIgnoreCase) -or
    $portableHash -ne ([string]$installPayload.portable_artifact_sha256).ToUpperInvariant() -or
    $installedHash -ne ([string]$installPayload.installed_executable_sha256).ToUpperInvariant() -or
    $installedHash -eq $portableHash -or
    -not ([IO.Path]::GetFullPath([string]$installPayload.setup)).Equals($setupFull, [StringComparison]::OrdinalIgnoreCase)
) { throw "Portable, setup, and distinct installed executable hashes are not mutually bound." }
if (
    (Get-FileHash -LiteralPath $portableFull -Algorithm SHA256).Hash.ToUpperInvariant() -ne $portableHash -or
    (Get-FileHash -LiteralPath $installedFull -Algorithm SHA256).Hash.ToUpperInvariant() -ne $installedHash
) { throw "Installed or portable executable bytes changed while marker equivalence was revalidated." }
if ($portableFull.Equals($installedFull, [StringComparison]::OrdinalIgnoreCase)) { throw "Installed and portable candidates must remain distinct files." }

$vitestPayload = Get-Content -LiteralPath $vitestReport -Raw -Encoding UTF8 | ConvertFrom-Json
if ($vitestPayload.success -ne $true -or [int]$vitestPayload.numFailedTests -ne 0 -or [int]$vitestPayload.numTotalTests -le 0) { throw "VitestReportPath is not a complete passing full-suite report." }
$vitestSha256 = (Get-FileHash -LiteralPath $vitestReport -Algorithm SHA256).Hash.ToLowerInvariant()
$consolidatedPayload = Get-Content -LiteralPath $consolidatedReport -Raw -Encoding UTF8 | ConvertFrom-Json
$expectedConsolidatedSteps = @("diff_check", "v255_evidence_contract", "v255_rebased_contract", "frontend_full_vitest", "rust_authority", "rust_archive_schema6_authoring", "rust_archive_three_way", "rust_desktop_three_way", "frontend_typecheck", "frontend_build", "python_export_semantic_readback", "rebaselined_interactions", "method_setup_crawler", "v255_final_evidence_contract")
$observedConsolidatedSteps = @($consolidatedPayload.steps | ForEach-Object { $_.id })
if ($consolidatedPayload.suite_id -ne "quickpls_v255_calculate_evidence_consolidated_diagnostics_v1" -or $consolidatedPayload.target_release -ne "2.55.0" -or $consolidatedPayload.passed -ne $true -or @($consolidatedPayload.summary.failed).Count -ne 0 -or @($consolidatedPayload.summary.skipped).Count -ne 0 -or (Compare-Object -ReferenceObject $expectedConsolidatedSteps -DifferenceObject $observedConsolidatedSteps -SyncWindow 0)) { throw "ConsolidatedReportPath is not the exact passing 2.55 diagnostic suite." }
if (@($consolidatedPayload.steps | Where-Object { $_.status -ne "passed" }).Count -ne 0 -or $consolidatedPayload.source.worktree_clean -ne $true -or $consolidatedPayload.artifacts.vitest_report_sha256 -ne $vitestSha256) { throw "The consolidated report is not bound to this passing Vitest report and a clean source commit." }
$packageVersion = (Get-Content -LiteralPath (Join-Path $root "package.json") -Raw -Encoding UTF8 | ConvertFrom-Json).version
if ($packageVersion -ne "2.55.0") { throw "Packaged 2.55 verification requires package version authority 2.55.0." }
$sourceCommit = (& git -C $root rev-parse HEAD).Trim()
$sourceStatus = @(& git -C $root status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$' -or $sourceStatus.Count -ne 0) { throw "Packaged verification requires a clean committed source worktree." }
$null = & git -C $root merge-base --is-ancestor $consolidatedPayload.source.commit $releasePayload.source.commit
if ($LASTEXITCODE -ne 0) { throw "The candidate build source is not descended from the exact passing consolidated source." }
$null = & git -C $root merge-base --is-ancestor $releasePayload.source.commit $sourceCommit
if ($LASTEXITCODE -ne 0) { throw "The current publication source is not descended from the exact candidate build source." }

function Get-DiskSnapshot([string]$Label) {
    $drives = [ordered]@{}
    foreach ($name in @("C", "D")) {
        $free = (Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
        if ($free -le $minimumFreeGiB) { throw "${Label}: drive $name has $([math]::Round($free,3)) GiB free; must remain strictly above $minimumFreeGiB GiB." }
        $drives[$name] = [math]::Round($free, 3)
    }
    return [ordered]@{ label = $Label; captured_at = (Get-Date).ToUniversalTime().ToString("o"); drives = $drives }
}
function Write-Utf8NoBom([string]$PathValue, [string]$TextValue) {
    [IO.File]::WriteAllText($PathValue, $TextValue, [Text.UTF8Encoding]::new($false))
}
function Test-Cdp([string]$Endpoint) { try { $null = Invoke-RestMethod -Uri "$Endpoint/json/version" -TimeoutSec 1; return $true } catch { return $false } }
function Wait-Cdp([string]$Endpoint, [bool]$Open) {
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    while ([DateTime]::UtcNow -lt $deadline) { if ((Test-Cdp $Endpoint) -eq $Open) { return }; Start-Sleep -Milliseconds 250 }
    throw "CDP endpoint did not become $(if ($Open) { "ready" } else { "closed" }): $Endpoint"
}
function Get-ProcessTree([int]$RootPid) {
    $all = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId, ParentProcessId, ExecutablePath, CreationDate)
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
        [ordered]@{ pid = $pidValue; parent_pid = if ($row) { [int]$row.ParentProcessId } else { $null }; executable = if ($row -and $row.ExecutablePath) { [IO.Path]::GetFullPath([string]$row.ExecutablePath) } else { $null }; creation_time = if ($row -and $row.CreationDate) { ([DateTime]$row.CreationDate).ToUniversalTime().ToString("o") } else { $null } }
    })
}
function Test-OwnedProcessIdentity($Row) {
    $current = Get-CimInstance Win32_Process -Filter "ProcessId = $([int]$Row.pid)" -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $current) { return $false }
    $currentCreation = if ($current.CreationDate) { ([DateTime]$current.CreationDate).ToUniversalTime().ToString("o") } else { $null }
    $currentExecutable = if ($current.ExecutablePath) { [IO.Path]::GetFullPath([string]$current.ExecutablePath) } else { $null }
    if (-not $Row.creation_time -or $currentCreation -ne [string]$Row.creation_time -or
        (($Row.executable -or $currentExecutable) -and -not [string]::Equals([string]$Row.executable, [string]$currentExecutable, [StringComparison]::OrdinalIgnoreCase))) {
        throw "PID $($Row.pid) no longer has the captured harness-owned creation/executable identity; refusing to terminate it."
    }
    return $true
}
function Update-OwnedTreeSnapshot($Process) {
    $treeByPid = @{}
    $saved = if ($Process.PSObject.Properties.Name -contains "QuickPlsOwnedTree") { @($Process.QuickPlsOwnedTree) } else { @() }
    $savedRoot = @($saved | Where-Object { [int]$_.pid -eq [int]$Process.Id } | Select-Object -First 1)
    if ($Process.HasExited -and $savedRoot.Count -eq 1) {
        $null = Test-OwnedProcessIdentity $savedRoot[0]
    }
    foreach ($row in $saved) {
        if ($null -ne $row -and $null -ne $row.pid -and -not $treeByPid.ContainsKey([string]$row.pid)) { $treeByPid[[string]$row.pid] = $row }
    }
    foreach ($row in @(Get-ProcessTree -RootPid $Process.Id)) {
        if ($null -ne $row -and $null -ne $row.pid -and -not $treeByPid.ContainsKey([string]$row.pid)) { $treeByPid[[string]$row.pid] = $row }
    }
    $snapshot = @($treeByPid.Values)
    $Process | Add-Member -NotePropertyName QuickPlsOwnedTree -NotePropertyValue $snapshot -Force
    return $snapshot
}
function Start-IsolatedCandidate([string]$Candidate, [string]$Endpoint) {
    if (Test-Cdp $Endpoint) { throw "Refusing to attach to an existing CDP endpoint: $Endpoint" }
    $process = Start-Process -FilePath $Candidate -WorkingDirectory $root -WindowStyle Hidden -PassThru
    try {
        $null = Update-OwnedTreeSnapshot $process
        Wait-Cdp $Endpoint $true
        $null = Update-OwnedTreeSnapshot $process
        return $process
    } catch {
        $startFailure = $_
        try {
            Stop-IsolatedCandidate $process $Endpoint
        } catch {
            $cleanupFailure = $_
            if (-not $process.HasExited) {
                & taskkill.exe /PID $process.Id /T /F *> $null
                $null = $process.WaitForExit(5000)
            }
            if (-not $process.HasExited) { throw "Candidate startup failed and the exact root PID could not be terminated: $($startFailure.Exception.Message); cleanup: $($cleanupFailure.Exception.Message)" }
        }
        throw $startFailure
    }
}
function Stop-IsolatedCandidate($Process, [string]$Endpoint) {
    if (-not $Process) { throw "No harness-owned candidate process was supplied for cleanup." }
    $tree = @(Update-OwnedTreeSnapshot $Process)
    if (-not $Process.HasExited) {
        # The only tree eligible for termination is rooted at this harness PID.
        & taskkill.exe /PID $Process.Id /T /F *> $null
        $null = $Process.WaitForExit(5000)
    }
    foreach ($row in @($tree | Where-Object { [int]$_.pid -ne [int]$Process.Id })) {
        if (Test-OwnedProcessIdentity $row) {
            & taskkill.exe /PID ([int]$row.pid) /T /F *> $null
        }
    }
    $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $remaining = @($tree | Where-Object { Test-OwnedProcessIdentity $_ })
        if ($remaining.Count -eq 0) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $cleanupDeadline)
    Wait-Cdp $Endpoint $false
    if ($remaining.Count -ne 0) { throw "Harness-owned candidate PIDs remained after exact-tree cleanup: $($remaining.pid -join ', ')." }
}
function Invoke-Candidate([string]$Name, [string]$Candidate, [int]$Port, [string]$BundleExtractDirectory) {
    $endpoint = "http://127.0.0.1:$Port"
    if (Test-Cdp $endpoint) { throw "$Name endpoint is already in use: $endpoint" }
    $candidateFull = [IO.Path]::GetFullPath($Candidate)
    $expectedCandidateHash = if ($Name -eq "portable") { $portableHash } elseif ($Name -eq "installed") { $installedHash } else { throw "Unknown candidate role: $Name" }
    $candidateHash = (Get-FileHash -LiteralPath $candidateFull -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($candidateHash -ne $expectedCandidateHash) { throw "$Name candidate no longer matches its build/install provenance." }
    $productVersion = (Get-Item -LiteralPath $candidateFull).VersionInfo.ProductVersion
    if (-not $productVersion -or -not $productVersion.StartsWith("2.55.0", [StringComparison]::Ordinal)) { throw "$Name candidate product version is '$productVersion', not 2.55.0." }
    $running = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals($candidateFull, [StringComparison]::OrdinalIgnoreCase) })
    if ($running.Count -ne 0) { throw "$Name candidate is already running; refusing to attach or close it." }
    $candidateEvidence = Join-Path $evidence $Name
    New-Item -ItemType Directory -Path $candidateEvidence -Force | Out-Null
    $crawlerEvidence = Join-Path $candidateEvidence "method_crawler"
    $previousArgs = [Environment]::GetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "Process")
    $previousProfile = [Environment]::GetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", "Process")
    $previousEndpoint = [Environment]::GetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", "Process")
    $process = $null
    $launchedPids = [System.Collections.Generic.List[int]]::new()
    $lifecycleEvidence = Join-Path $candidateEvidence "live_lifecycle"
    $projectPath = Join-Path $lifecycleEvidence "quickpls-v255-live-lifecycle.qpls"
    $lifecycleExecuteStdout = Join-Path $candidateEvidence "lifecycle_execute.stdout.log"
    $lifecycleExecuteStderr = Join-Path $candidateEvidence "lifecycle_execute.stderr.log"
    $lifecycleReopenStdout = Join-Path $candidateEvidence "lifecycle_reopen.stdout.log"
    $lifecycleReopenStderr = Join-Path $candidateEvidence "lifecycle_reopen.stderr.log"
    $crawlerStdout = Join-Path $candidateEvidence "method_crawler.stdout.log"
    $crawlerStderr = Join-Path $candidateEvidence "method_crawler.stderr.log"
    $namedCaseEvidence = Join-Path $candidateEvidence "named_case_driver"
    $namedCaseStdout = Join-Path $candidateEvidence "named_case_driver.stdout.log"
    $namedCaseStderr = Join-Path $candidateEvidence "named_case_driver.stderr.log"
    $namedEvidenceDriverReports = @()
    try {
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port --force-device-scale-factor=1 --disable-background-networking --disable-component-update"
        $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $candidateEvidence "webview2-profile"
        $env:QUICKPLS_CDP_ENDPOINT = $endpoint
        New-Item -ItemType Directory -Path $lifecycleEvidence -Force | Out-Null
        $process = Start-IsolatedCandidate $candidateFull $endpoint
        $launchedPids.Add($process.Id)
        & $node $lifecycleDriver --phase execute --endpoint $endpoint --evidence-dir $lifecycleEvidence --project-path $projectPath --python $python --candidate-pid $process.Id --candidate-path $candidateFull 1> $lifecycleExecuteStdout 2> $lifecycleExecuteStderr
        if ($LASTEXITCODE -ne 0) { throw "$Name live calculation execute phase failed." }
        Stop-IsolatedCandidate $process $endpoint
        $process = $null

        # A new hidden process is mandatory: no in-memory state may satisfy reopen.
        $process = Start-IsolatedCandidate $candidateFull $endpoint
        $launchedPids.Add($process.Id)
        & $node $lifecycleDriver --phase reopen --endpoint $endpoint --evidence-dir $lifecycleEvidence --project-path $projectPath --python $python --candidate-pid $process.Id --candidate-path $candidateFull 1> $lifecycleReopenStdout 2> $lifecycleReopenStderr
        if ($LASTEXITCODE -ne 0) { throw "$Name fresh-reopen phase failed." }
        Stop-IsolatedCandidate $process $endpoint
        $process = $null

        $lifecycleReport = Get-Content -LiteralPath (Join-Path $lifecycleEvidence "v255_live_calculation_lifecycle_smoke.json") -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($lifecycleReport.passed -ne $true -or $lifecycleReport.complete -ne $true -or @($lifecycleReport.failures).Count -ne 0) { throw "$Name live lifecycle report is incomplete or failed." }

        $effectivePosthocSupplementRelative = $null
        $posthocCollection = $null
        if ($Name -eq "portable") {
            $posthocEvidence = Join-Path $candidateEvidence "posthoc_minimum_sample"
            $posthocExecuteStdout = Join-Path $candidateEvidence "posthoc_execute.stdout.log"
            $posthocExecuteStderr = Join-Path $candidateEvidence "posthoc_execute.stderr.log"
            $posthocReopenStdout = Join-Path $candidateEvidence "posthoc_reopen.stdout.log"
            $posthocReopenStderr = Join-Path $candidateEvidence "posthoc_reopen.stderr.log"
            $process = Start-IsolatedCandidate $candidateFull $endpoint
            $launchedPids.Add($process.Id)
            & $node $posthocDriver --phase execute --endpoint $endpoint --evidence-dir $posthocEvidence 1> $posthocExecuteStdout 2> $posthocExecuteStderr
            if ($LASTEXITCODE -ne 0) { throw "Portable post-hoc minimum-sample execute phase failed." }
            Stop-IsolatedCandidate $process $endpoint
            $process = $null

            $process = Start-IsolatedCandidate $candidateFull $endpoint
            $launchedPids.Add($process.Id)
            & $node $posthocDriver --phase reopen --endpoint $endpoint --evidence-dir $posthocEvidence 1> $posthocReopenStdout 2> $posthocReopenStderr
            if ($LASTEXITCODE -ne 0) { throw "Portable post-hoc minimum-sample fresh-reopen phase failed." }
            Stop-IsolatedCandidate $process $endpoint
            $process = $null

            $posthocExecuteReceipt = Join-Path $posthocEvidence "v255_posthoc_minimum_sample_packaged_smoke.json"
            $posthocReopenReceipt = Join-Path $posthocEvidence "v255_posthoc_minimum_sample_reopen.json"
            $posthocExecute = Get-Content -LiteralPath $posthocExecuteReceipt -Raw -Encoding UTF8 | ConvertFrom-Json
            $posthocReopen = Get-Content -LiteralPath $posthocReopenReceipt -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($posthocExecute.status -ne "passed" -or $posthocExecute.phase -ne "execute" -or $posthocReopen.status -ne "passed" -or $posthocReopen.phase -ne "reopen" -or $posthocReopen.same_result_identity -ne $true) { throw "Portable post-hoc minimum-sample receipts are incomplete." }
            $posthocExecuteReceiptFull = [IO.Path]::GetFullPath($posthocExecuteReceipt)
            if (-not $posthocExecuteReceiptFull.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Generated post-hoc receipt must remain beneath the repository root."
            }
            $effectivePosthocSupplementRelative = $posthocExecuteReceiptFull.Substring($rootPrefix.Length).Replace('\', '/')
            $posthocCollection = [ordered]@{ status = "passed"; generated = $true; candidate = $candidateFull; candidate_sha256 = $candidateHash; build_source_commit = $releasePayload.source.commit; release_artifact_report_sha256 = $releaseArtifactReportHash; execute_receipt = $posthocExecuteReceipt; execute_receipt_sha256 = (Get-FileHash -LiteralPath $posthocExecuteReceipt -Algorithm SHA256).Hash.ToLowerInvariant(); reopen_receipt = $posthocReopenReceipt; reopen_receipt_sha256 = (Get-FileHash -LiteralPath $posthocReopenReceipt -Algorithm SHA256).Hash.ToLowerInvariant(); result_id = $posthocExecute.new_result_id; archive_path = $posthocExecute.archive_path; execute_stdout = $posthocExecuteStdout; execute_stderr = $posthocExecuteStderr; reopen_stdout = $posthocReopenStdout; reopen_stderr = $posthocReopenStderr }
        }

        $process = Start-IsolatedCandidate $candidateFull $endpoint
        $launchedPids.Add($process.Id)
        $crawlerArguments = @("--mode", "packaged", "--result-evidence-phase", $(if ($evidenceBundle) { "publication" } else { "source" }), "--endpoint", $endpoint, "--evidence-dir", $crawlerEvidence, "--require-results", "true", "--vitest-report", $vitestReport)
        if ($evidenceBundle) { $crawlerArguments += @("--evidence-bundle", $evidenceBundle, "--evidence-extract-dir", $BundleExtractDirectory) }
        & $node $driver @crawlerArguments 1> $crawlerStdout 2> $crawlerStderr
        if ($LASTEXITCODE -ne 0) { throw "$Name packaged evidence crawler failed." }
        $frozenArchiveCollection = $null
        if ($Name -eq "portable") {
            $frozenStaging = Join-Path $candidateEvidence "frozen_archive_reopen"
            $frozenStdout = Join-Path $candidateEvidence "frozen_archive_reopen.stdout.log"
            $frozenStderr = Join-Path $candidateEvidence "frozen_archive_reopen.stderr.log"
            $frozenArguments = @($frozenArchiveCrawler, "--endpoint", $endpoint, "--staging-dir", $frozenStaging)
            if ($effectivePosthocSupplementRelative) { $frozenArguments += @("--posthoc-supplement", $effectivePosthocSupplementRelative) }
            & $node @frozenArguments 1> $frozenStdout 2> $frozenStderr
            $frozenExit = $LASTEXITCODE
            $aggregateReceipt = Join-Path $frozenStaging "receipts\v255-frozen-archive-reopen-crawler.json"
            if (-not (Test-Path -LiteralPath $aggregateReceipt -PathType Leaf)) { throw "Portable frozen-archive crawler produced no aggregate receipt." }
            $aggregate = Get-Content -LiteralPath $aggregateReceipt -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($frozenExit -ne 0 -or $aggregate.status -ne "passed" -or @($aggregate.failures).Count -ne 0) { throw "Portable frozen-archive crawler failed with its freshly generated posthoc run." }
            $frozenArchiveCollection = [ordered]@{ status = $aggregate.status; exit_code = $frozenExit; aggregate_receipt = $aggregateReceipt; aggregate_receipt_sha256 = (Get-FileHash -LiteralPath $aggregateReceipt -Algorithm SHA256).Hash.ToLowerInvariant(); staging = $frozenStaging; stdout = $frozenStdout; stderr = $frozenStderr; posthoc_supplement = $effectivePosthocSupplementRelative }
        }
        if ($namedCaseManifestReady) {
            if (-not $process -or $process.HasExited) { throw "$Name named-case evidence requires the live wrapper-owned candidate process." }
            $namedCaseCandidatePid = $process.Id
            & $node $namedCaseDriver --endpoint $endpoint --manifest $namedCaseManifest --index $namedEvidenceIndex --evidence-dir $namedCaseEvidence --candidate-name $Name --candidate-pid $namedCaseCandidatePid --candidate-path $candidateFull --python $python 1> $namedCaseStdout 2> $namedCaseStderr
            if ($LASTEXITCODE -ne 0) { throw "$Name named-case evidence driver failed." }
            $namedCaseReportPath = Join-Path $namedCaseEvidence "v255_named_case_driver.json"
            if (-not (Test-Path -LiteralPath $namedCaseReportPath -PathType Leaf)) { throw "$Name named-case evidence driver produced no report." }
            $namedCaseReport = Get-Content -LiteralPath $namedCaseReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
            $expectedNamedCaseCount = @($readyNamedCases | Where-Object { $_.candidate -eq $Name }).Count
            if (
                $namedCaseReport.schema_version -ne 1 -or
                $namedCaseReport.suite_id -ne "quickpls_v255_named_case_driver_v1" -or
                $namedCaseReport.target_release -ne "2.55.0" -or
                $namedCaseReport.candidate -ne $Name -or
                [int]$namedCaseReport.candidate_process.pid -ne $namedCaseCandidatePid -or
                -not ([IO.Path]::GetFullPath([string]$namedCaseReport.candidate_process.executable).Equals($candidateFull, [StringComparison]::OrdinalIgnoreCase)) -or
                ([string]$namedCaseReport.candidate_process.executable_sha256).ToLowerInvariant() -ne $candidateHash.ToLowerInvariant() -or
                $namedCaseReport.process_safety.candidate_pid_bound -ne $true -or
                $namedCaseReport.process_safety.candidate_executable_bound -ne $true -or
                $namedCaseReport.status -ne "passed" -or
                $namedCaseReport.passed -ne $true -or
                @($namedCaseReport.failures).Count -ne 0 -or
                $namedCaseReport.offline.passed -ne $true -or
                @($namedCaseReport.cases).Count -ne $expectedNamedCaseCount -or
                @($namedCaseReport.cases | Where-Object { $_.status -ne "passed" }).Count -ne 0 -or
                @($namedCaseReport.named_evidence_observations).Count -ne $expectedNamedCaseCount
            ) { throw "$Name named-case evidence report is incomplete or false." }
            $namedEvidenceDriverReports = @([ordered]@{ path = $namedCaseReportPath; sha256 = (Get-FileHash -LiteralPath $namedCaseReportPath -Algorithm SHA256).Hash.ToLowerInvariant() })
        }
        $lifecycleReportPath = Join-Path $lifecycleEvidence "v255_live_calculation_lifecycle_smoke.json"
        $methodReportPath = Join-Path $crawlerEvidence "v255_method_evidence_crawler.json"
        $methodReport = Get-Content -LiteralPath $methodReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($methodReport.passed -ne $true -or @($methodReport.failures).Count -ne 0) { throw "$Name method evidence report is incomplete or false." }
        [ordered]@{ name = $Name; candidate_kind = $(if ($Name -eq "installed") { "fresh_nsis_install" } else { "portable_release_artifact" }); executable = $candidateFull; executable_sha256 = $candidateHash; product_version = $productVersion; build_source_commit = $releasePayload.source.commit; source_tree = $releasePayload.source.tree; source_manifest_sha256 = $releasePayload.source.tracked_manifest_sha256; release_artifact_report = $releaseArtifactReport; release_artifact_report_sha256 = $releaseArtifactReportHash; install_receipt = $(if ($Name -eq "installed") { $installReceipt } else { $null }); install_receipt_sha256 = $(if ($Name -eq "installed") { $installReceiptHash } else { $null }); launched_pids = @($launchedPids); status = "passed"; lifecycle = $lifecycleReportPath; lifecycle_sha256 = (Get-FileHash -LiteralPath $lifecycleReportPath -Algorithm SHA256).Hash.ToLowerInvariant(); lifecycle_execute_stdout = $lifecycleExecuteStdout; lifecycle_execute_stderr = $lifecycleExecuteStderr; lifecycle_reopen_stdout = $lifecycleReopenStdout; lifecycle_reopen_stderr = $lifecycleReopenStderr; evidence = $methodReportPath; evidence_sha256 = (Get-FileHash -LiteralPath $methodReportPath -Algorithm SHA256).Hash.ToLowerInvariant(); evidence_stdout = $crawlerStdout; evidence_stderr = $crawlerStderr; named_evidence_driver_reports = @($namedEvidenceDriverReports); named_evidence_driver_stdout = $(if ($namedCaseManifestReady) { $namedCaseStdout } else { $null }); named_evidence_driver_stderr = $(if ($namedCaseManifestReady) { $namedCaseStderr } else { $null }); posthoc_collection = $posthocCollection; frozen_archive_collection = $frozenArchiveCollection }
    } finally {
        try {
            if ($process) { Stop-IsolatedCandidate $process $endpoint }
        } finally {
            [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $previousArgs, "Process")
            [Environment]::SetEnvironmentVariable("WEBVIEW2_USER_DATA_FOLDER", $previousProfile, "Process")
            [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $previousEndpoint, "Process")
        }
    }
}

$snapshots = @()
$snapshots += Get-DiskSnapshot "before packaged smoke"
New-Item -ItemType Directory -Path $evidence | Out-Null
$bundleExtractDirectory = $null
if ($evidenceBundle) {
    $bundleExtractDirectory = Join-Path $evidence "verified_evidence_bundle"
    if (Test-Path -LiteralPath $bundleExtractDirectory) { throw "Refusing to reuse evidence extraction directory: $bundleExtractDirectory" }
}
$namedEvidenceReport = Join-Path $evidence "v255_named_evidence_verifier.json"
$namedEvidenceStdout = Join-Path $evidence "named_evidence_verifier.stdout.log"
$namedEvidenceStderr = Join-Path $evidence "named_evidence_verifier.stderr.log"
$namedEvidenceStage = if ($evidenceBundle) { "publication" } else { "source" }
$namedEvidenceArguments = @($namedEvidenceVerifier, "--stage", $namedEvidenceStage, "--output", $namedEvidenceReport)
if ($evidenceBundle) { $namedEvidenceArguments += @("--evidence-bundle", $evidenceBundle) }
& $python @namedEvidenceArguments 1> $namedEvidenceStdout 2> $namedEvidenceStderr
$namedEvidenceExit = $LASTEXITCODE
if (-not (Test-Path -LiteralPath $namedEvidenceReport -PathType Leaf)) { throw "Named-evidence verifier produced no report." }
$namedEvidencePayload = Get-Content -LiteralPath $namedEvidenceReport -Raw -Encoding UTF8 | ConvertFrom-Json
if ($namedEvidenceExit -ne 0 -or $namedEvidencePayload.passed -ne $true -or $namedEvidencePayload.stage -ne $namedEvidenceStage -or $namedEvidencePayload.target_release -ne "2.55.0") { throw "Named cross/specialized evidence verification failed for stage '$namedEvidenceStage'." }
$extractionCapacity = $null
if ($evidenceBundle) {
    $rawUncompressedBytes = $namedEvidencePayload.sources.publication_provenance.bundle_uncompressed_bytes
    $rawCompressedBytes = $namedEvidencePayload.sources.publication_provenance.bundle_compressed_bytes
    $rawMemberCount = $namedEvidencePayload.sources.publication_provenance.bundle_member_count
    $uncompressedBytes = [long]$rawUncompressedBytes
    $compressedBytes = [long]$rawCompressedBytes
    $memberCount = [long]$rawMemberCount
    if ($uncompressedBytes -ne $rawUncompressedBytes -or $compressedBytes -ne $rawCompressedBytes -or $memberCount -ne $rawMemberCount -or $uncompressedBytes -le 0 -or $compressedBytes -le 0 -or $memberCount -le 0) {
        throw "Publication verifier did not publish positive verified bundle size/member totals."
    }
    $reserveBytes = [long](0.25 * 1GB)
    $minimumFreeBytes = [long]($minimumFreeGiB * 1GB)
    if ($uncompressedBytes -gt [long]::MaxValue - $minimumFreeBytes - $reserveBytes) { throw "Verified bundle size exceeds the supported extraction-capacity range." }
    $requiredDBytes = $minimumFreeBytes + $uncompressedBytes + $reserveBytes
    $availableDBytes = [long]((Get-PSDrive -Name D -PSProvider FileSystem -ErrorAction Stop).Free)
    if ($availableDBytes -le $requiredDBytes) {
        throw "Evidence extraction requires D: free space strictly above 20 GiB plus $uncompressedBytes verified uncompressed bytes plus 0.25 GiB reserve; available=$availableDBytes required>$requiredDBytes."
    }
    $extractionCapacity = [ordered]@{ verified_member_count = $memberCount; verified_compressed_bytes = $compressedBytes; verified_uncompressed_bytes = $uncompressedBytes; reserve_bytes = $reserveBytes; required_d_free_bytes_strictly_above = $requiredDBytes; observed_d_free_bytes = $availableDBytes; passed = $true }
    # Extraction is permitted only after the verifier has rejected duplicate,
    # absolute, parent-traversal, backslash, and drive-qualified ZIP members.
    Expand-Archive -LiteralPath $evidenceBundle -DestinationPath $bundleExtractDirectory -ErrorAction Stop
}
$outcomes = @()
$runError = $null
$crossMethodReport = $null
try {
    $outcomes += Invoke-Candidate "installed" $installedFull $FirstPort $bundleExtractDirectory
    $snapshots += Get-DiskSnapshot "between installed and portable smoke"
    # Portable runs last and therefore supplies the fresh, candidate-bound
    # post-hoc archive used by the final frozen-reopen evidence collection.
    $outcomes += Invoke-Candidate "portable" $portableFull ($FirstPort + 1) $bundleExtractDirectory
    $crossMethodEvidence = Join-Path $evidence "portable\cross_method"
    $crossMethodStdout = Join-Path $evidence "portable\cross_method.stdout.log"
    $crossMethodStderr = Join-Path $evidence "portable\cross_method.stderr.log"
    $crossMethodArguments = @("-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", $crossMethodWrapper, "-ReleaseArtifactReportPath", $releaseArtifactReport, "-EvidenceDir", $crossMethodEvidence, "-FirstPort", [string]($FirstPort + 20))
    if ($WaiveActualWindows200PercentScaling) { $crossMethodArguments += "-WaiveActualWindows200PercentScaling" }
    & powershell.exe @crossMethodArguments 1> $crossMethodStdout 2> $crossMethodStderr
    if ($LASTEXITCODE -ne 0) { throw "Portable cross-method candidate wrapper failed; see $crossMethodStderr." }
    $crossMethodReportPath = Join-Path $crossMethodEvidence "v255_cross_method_candidate_smoke.json"
    if (-not (Test-Path -LiteralPath $crossMethodReportPath -PathType Leaf)) { throw "Portable cross-method wrapper produced no aggregate report." }
    $crossMethodReport = Get-Content -LiteralPath $crossMethodReportPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $crossWaived = @($crossMethodReport.named_evidence_observations | Where-Object { $_.status -eq "waived" })
    $crossPassed = @($crossMethodReport.named_evidence_observations | Where-Object { $_.assertion.passed -eq $true })
    $expectedWaiverCount = if ($WaiveActualWindows200PercentScaling) { 1 } else { 0 }
    $expectedQualificationStatus = if ($WaiveActualWindows200PercentScaling) { "passed_with_waiver" } else { "passed" }
    $waiverContractValid = if ($WaiveActualWindows200PercentScaling) {
        $crossMethodReport.release_waivers.Count -eq 1 -and
        $crossMethodReport.release_waivers[0].case_id -eq $dpiWaiverCaseId -and
        $crossMethodReport.release_waivers[0].status -eq "waived" -and
        $crossMethodReport.release_waivers[0].assertion_passed -eq $false -and
        $crossMethodReport.release_waivers[0].waiver_authority -eq $dpiWaiverMetadata.waiver_authority -and
        $crossMethodReport.release_waivers[0].waiver_date -eq $dpiWaiverMetadata.waiver_date -and
        $crossMethodReport.release_waivers[0].reason -eq $dpiWaiverMetadata.reason -and
        $crossWaived.Count -eq 1 -and
        $crossWaived[0].case_id -eq $dpiWaiverCaseId -and
        $crossWaived[0].assertion.passed -eq $false
    } else { @($crossMethodReport.release_waivers).Count -eq 0 -and $crossWaived.Count -eq 0 }
    if (
        $crossMethodReport.schema_version -ne 1 -or
        $crossMethodReport.suite_id -ne "quickpls_v255_cross_method_candidate_wrapper_v1" -or
        $crossMethodReport.target_release -ne "2.55.0" -or
        $crossMethodReport.passed -ne $true -or
        $crossMethodReport.qualification_status -ne $expectedQualificationStatus -or
        $crossMethodReport.source_commit -ne $releasePayload.source.commit -or
        $crossMethodReport.release_artifact_report.sha256 -ne $releaseArtifactReportHash -or
        $crossMethodReport.candidate.path -ne $portableFull -or
        $crossMethodReport.candidate.sha256 -ne $portableHash -or
        @($crossMethodReport.failures).Count -ne 0 -or
        @($crossMethodReport.named_evidence_observations).Count -ne 17 -or
        $crossPassed.Count -ne (17 - $expectedWaiverCount) -or
        -not $waiverContractValid -or
        @($crossMethodReport.named_evidence_observations.screenshot.sha256 | Select-Object -Unique).Count -ne 17
    ) { throw "Portable cross-method candidate report is incomplete, false, or unbound." }
    $portableOutcome = @($outcomes | Where-Object { $_.name -eq "portable" })[0]
    $crossMethodBinding = [ordered]@{ path = $crossMethodReportPath; sha256 = (Get-FileHash -LiteralPath $crossMethodReportPath -Algorithm SHA256).Hash.ToLowerInvariant() }
    $portableOutcome["named_evidence_driver_reports"] = @($portableOutcome.named_evidence_driver_reports) + @($crossMethodBinding)
    $portableOutcome["cross_method_collection"] = [ordered]@{ status = $expectedQualificationStatus; report = $crossMethodReportPath; report_sha256 = $crossMethodBinding.sha256; stdout = $crossMethodStdout; stderr = $crossMethodStderr; named_case_count = 17; passed_case_count = $crossPassed.Count; waived_case_count = $crossWaived.Count }
} catch { $runError = $_.Exception.Message
} finally { $snapshots += Get-DiskSnapshot "after packaged smoke" }
if (-not $runError) {
    if (
        (Get-FileHash -LiteralPath $releaseArtifactReport -Algorithm SHA256).Hash.ToUpperInvariant() -ne $releaseArtifactReportHash -or
        (Get-FileHash -LiteralPath $installReceipt -Algorithm SHA256).Hash.ToUpperInvariant() -ne $installReceiptHash -or
        (Get-FileHash -LiteralPath $portableFull -Algorithm SHA256).Hash.ToUpperInvariant() -ne $portableHash -or
        (Get-FileHash -LiteralPath $installedFull -Algorithm SHA256).Hash.ToUpperInvariant() -ne $installedHash
    ) { $runError = "Candidate provenance inputs changed while packaged evidence was collected." }
}
$outcomeNames = @($outcomes | ForEach-Object { $_.name })
$allOutcomesPassed = $outcomes.Count -eq 2 -and @($outcomes | Where-Object { $_.status -ne "passed" }).Count -eq 0 -and -not (Compare-Object -ReferenceObject @("installed", "portable") -DifferenceObject $outcomeNames -SyncWindow 0)
$reportPassed = $allOutcomesPassed -and -not $runError
$qualificationStatus = if (-not $reportPassed) { "failed" } elseif ($WaiveActualWindows200PercentScaling) { "passed_with_waiver" } else { "passed" }
[object[]]$releaseWaivers = @()
if ($WaiveActualWindows200PercentScaling -and $crossMethodReport) { [object[]]$releaseWaivers = @($crossMethodReport.release_waivers) }
$report = [ordered]@{ schema_version = 3; suite_id = "quickpls_v255_installed_portable_smoke_v3"; target_release = "2.55.0"; passed = $reportPassed; qualification_status = $qualificationStatus; release_waivers = $releaseWaivers; publication_source_commit = $sourceCommit; source_worktree_clean = $true; package_version = $packageVersion; candidate_build_source_commit = $releasePayload.source.commit; candidate_build_source_tree = $releasePayload.source.tree; candidate_source_manifest_sha256 = $releasePayload.source.tracked_manifest_sha256; release_artifact_report = $releaseArtifactReport; release_artifact_report_sha256 = $releaseArtifactReportHash; install_receipt = $installReceipt; install_receipt_sha256 = $installReceiptHash; consolidated_report = $consolidatedReport; consolidated_report_sha256 = (Get-FileHash -LiteralPath $consolidatedReport -Algorithm SHA256).Hash.ToLowerInvariant(); tested_source_commit = $consolidatedPayload.source.commit; release_publication_evidence_verified = [bool]$evidenceBundle; publication_evidence_status = $(if ($evidenceBundle) { "verified_bundle_required" } else { "collection_only_pending_curated_v255_result_captures_and_bundle" }); named_evidence_stage = $namedEvidenceStage; named_evidence_report = $namedEvidenceReport; named_evidence_report_sha256 = (Get-FileHash -LiteralPath $namedEvidenceReport -Algorithm SHA256).Hash.ToLowerInvariant(); named_evidence_stdout = $namedEvidenceStdout; named_evidence_stderr = $namedEvidenceStderr; named_evidence_verified = $namedEvidencePayload.passed -eq $true; code_signing = $false; process_safety = "only trees rooted at harness-launched PIDs were terminated"; minimum_free_gib = $minimumFreeGiB; vitest_report = $vitestReport; vitest_report_sha256 = $vitestSha256; evidence_bundle = $evidenceBundle; evidence_bundle_sha256 = $(if ($evidenceBundle) { (Get-FileHash -LiteralPath $evidenceBundle -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }); evidence_bundle_extraction = $bundleExtractDirectory; evidence_bundle_extraction_capacity = $extractionCapacity; outcomes = $outcomes; error = $runError; disk_snapshots = $snapshots }
$reportPath = Join-Path $evidence "v255_installed_portable_smoke.json"
Write-Utf8NoBom $reportPath (($report | ConvertTo-Json -Depth 16) + "`n")
$report | ConvertTo-Json -Depth 16
if ($runError) { throw $runError }
if ($report.passed -ne $true) { throw "Installed/portable packaged smoke produced a false aggregate report: $reportPath" }
