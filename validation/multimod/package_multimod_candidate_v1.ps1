[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-f0-9]{40}$')][string]$CandidateCommit,
    [Parameter(Mandatory = $true)][string]$PrepackageAuthority,
    [Parameter(Mandatory = $true)][string]$PrepackageManifestSet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [IO.Path]::GetFullPath($Output)
$packageDirectory = Split-Path -Parent $outputPath
$campaignRoot = Split-Path -Parent $packageDirectory
$finalVersion = "2.56.0"
$planSha256 = [string]$env:QPLS_MULTIMOD_PLAN_SHA256
$bindingSha256 = [string]$env:QPLS_MULTIMOD_BINDING_SHA256
$prepackageAuthorityPath = [IO.Path]::GetFullPath($PrepackageAuthority)
$prepackageManifestSetPath = [IO.Path]::GetFullPath($PrepackageManifestSet)

function Invoke-Checked {
    param([Parameter(Mandatory = $true)][string]$Executable, [Parameter(Mandatory = $true)][string[]]$Arguments)
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Executable $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-JsonAtomic([string]$Path, $Value) {
    $temporary = "$Path.tmp"
    [IO.File]::WriteAllText($temporary, (($Value | ConvertTo-Json -Depth 30) + "`n"), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

if (-not $IsWindows) { throw "MultiMod candidate packaging is Windows-only." }
if ($env:QPLS_MULTIMOD_CANDIDATE_COMMIT -cne $CandidateCommit -or $env:QPLS_MULTIMOD_CANDIDATE_VERSION -cne $finalVersion) {
    throw "Candidate commit/version environment binding is missing or stale."
}
if ($planSha256 -cnotmatch '^[a-f0-9]{64}$' -or $bindingSha256 -cnotmatch '^[a-f0-9]{64}$') {
    throw "Plan and binding catalog digests must be propagated into packaging."
}
foreach ($authorityInput in @($prepackageAuthorityPath, $prepackageManifestSetPath)) {
    if (-not (Test-Path -LiteralPath $authorityInput -PathType Leaf)) {
        throw "Build-embedded MultiMod authority input is missing: $authorityInput"
    }
}
$expectedAuthorityPath = [IO.Path]::GetFullPath((Join-Path $campaignRoot "prepackage-authority\candidate-authority.json"))
$expectedManifestSetPath = [IO.Path]::GetFullPath((Join-Path $campaignRoot "prepackage-authority\manifest-set.json"))
if (
    -not $prepackageAuthorityPath.Equals($expectedAuthorityPath, [StringComparison]::OrdinalIgnoreCase) -or
    -not $prepackageManifestSetPath.Equals($expectedManifestSetPath, [StringComparison]::OrdinalIgnoreCase)
) { throw "Candidate packaging accepts only this campaign's fixed prepackage authority paths." }
$authority = Get-Content -LiteralPath $prepackageAuthorityPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 50
$manifestSet = Get-Content -LiteralPath $prepackageManifestSetPath -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 50
if (
    $authority.schema_version -ne 1 -or
    $authority.authority_kind -cne "qpls_multimod_embedded_candidate_authority_v1" -or
    $authority.state -cne "release_qualified_candidate" -or
    [string]$authority.binding.candidate_commit_sha -cne $CandidateCommit -or
    [string]$authority.binding.candidate_version -cne $finalVersion -or
    [string]$authority.binding.qualification_plan_sha256 -cne $planSha256 -or
    [string]$authority.binding.gate_binding_sha256 -cne $bindingSha256 -or
    [string]$authority.binding.prepackage_manifest_set_sha256 -cne (Get-LowerSha256 $prepackageManifestSetPath) -or
    [string]$authority.authority_binding_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
    $manifestSet.schema_version -ne 1 -or
    $manifestSet.manifest_set_id -cne "qpls.v256.multimod.prepackage-authority-set.v1" -or
    $manifestSet.stage -cne "prepackage_authority" -or
    $manifestSet.state -cne "release_qualified" -or
    $manifestSet.candidate_commit_sha -cne $CandidateCommit -or
    $manifestSet.candidate_version -cne $finalVersion -or
    $manifestSet.plan_sha256 -cne $planSha256 -or
    $manifestSet.gate_binding_sha256 -cne $bindingSha256 -or
    @($manifestSet.exact_profile_cells).Count -eq 0 -or
    (Compare-Object -CaseSensitive -SyncWindow 0 @($authority.binding.exact_profile_cells) @($manifestSet.exact_profile_cells))
) { throw "Prepackage candidate authority or manifest-set identity is invalid or stale." }
$head = (& git -C $repositoryRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $head -cne $CandidateCommit) { throw "CandidateCommit differs from current HEAD." }
if ((& git -C $repositoryRoot status --porcelain=v1 --untracked-files=all) -join "`n") { throw "Candidate worktree must be clean before packaging." }

$package = Get-Content -LiteralPath (Join-Path $repositoryRoot "package.json") -Raw | ConvertFrom-Json
$tauri = Get-Content -LiteralPath (Join-Path $repositoryRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath (Join-Path $repositoryRoot "Cargo.toml") -Raw
$cargoVersionMatch = [regex]::Match($cargoText, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
if (-not $cargoVersionMatch.Success) { throw "Workspace Cargo version is missing." }
$versions = @([string]$package.version, [string]$tauri.version, [string]$cargoVersionMatch.Groups[1].Value)
if (@($versions | Where-Object { $_ -cne $finalVersion }).Count -ne 0) {
    throw "Final candidate packaging requires package, Tauri and workspace version $finalVersion; found $($versions -join ', ')."
}
if ($tauri.bundle.targets -cne "nsis" -or $tauri.bundle.windows.webviewInstallMode.type -cne "offlineInstaller") {
    throw "Candidate must use the frozen NSIS target with the offline WebView installer."
}
foreach ($driveName in @("C", "D")) {
    $minimum = if ($driveName -eq "C") { 20.0 } else { 25.0 }
    $free = (Get-PSDrive -Name $driveName -PSProvider FileSystem -ErrorAction Stop).Free / 1GB
    if ($free -lt $minimum) { throw "Package disk guard failed for $driveName`: $free GiB is below $minimum GiB." }
}
if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -gt 0) {
    throw "Another Cargo process is active."
}

Push-Location -LiteralPath $repositoryRoot
try {
    $env:QPLS_MULTIMOD_BUILD_CANDIDATE_AUTHORITY_V1 = $prepackageAuthorityPath
    $env:QPLS_MULTIMOD_BUILD_PREPACKAGE_MANIFEST_SET_V1 = $prepackageManifestSetPath
    # This flag is consumed by Vite at compile time. It is deliberately paired
    # with the Cargo feature below so the qualification bridge and its native
    # fixture command exist only in this exact unmerged review candidate.
    $env:VITE_QPLS_MULTIMOD_QUALIFICATION_HARNESS_V1 = "1"
    Invoke-Checked -Executable "cargo" -Arguments @("check", "--locked", "--workspace")
    Invoke-Checked -Executable "npm.cmd" -Arguments @("run", "build")
    # The frontend production build already passed above. Override only the
    # Tauri beforeBuild hook so packaging does not repeat that full build.
    Invoke-Checked -Executable "npm.cmd" -Arguments @("run", "tauri", "--", "build", "--features", "multimod-qualification-harness", "--bundles", "nsis", "--config", '{"build":{"beforeBuildCommand":""}}')
}
finally {
    Remove-Item Env:QPLS_MULTIMOD_BUILD_CANDIDATE_AUTHORITY_V1 -ErrorAction SilentlyContinue
    Remove-Item Env:QPLS_MULTIMOD_BUILD_PREPACKAGE_MANIFEST_SET_V1 -ErrorAction SilentlyContinue
    Remove-Item Env:VITE_QPLS_MULTIMOD_QUALIFICATION_HARNESS_V1 -ErrorAction SilentlyContinue
    Pop-Location
}

$sourcePortable = Join-Path $repositoryRoot "target\release\quickpls-desktop.exe"
$sourceSetup = Join-Path $repositoryRoot "target\release\bundle\nsis\QuickPLS_${finalVersion}_x64-setup.exe"
foreach ($required in @($sourcePortable, $sourceSetup)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Expected candidate artifact is missing: $required" }
}
New-Item -ItemType Directory -Path $packageDirectory -Force | Out-Null
$portable = Join-Path $packageDirectory "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_portable.exe"
$setup = Join-Path $packageDirectory "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_setup.exe"
foreach ($destination in @($portable, $setup, $outputPath)) {
    if (Test-Path -LiteralPath $destination) { throw "Candidate package destination already exists: $destination" }
}
Copy-Item -LiteralPath $sourcePortable -Destination $portable
Copy-Item -LiteralPath $sourceSetup -Destination $setup

$artifacts = @(
    [ordered]@{ role = "portable"; path = $portable; size = [long](Get-Item $portable).Length; sha256 = Get-LowerSha256 $portable },
    [ordered]@{ role = "setup"; path = $setup; size = [long](Get-Item $setup).Length; sha256 = Get-LowerSha256 $setup }
)
$receipt = [ordered]@{
    schema_version = 1
    receipt_kind = "qpls_multimod_candidate_package_v1"
    candidate_commit_sha = $CandidateCommit
    version = $finalVersion
    plan_sha256 = $planSha256
    binding_sha256 = $bindingSha256
    prepackage_authority_sha256 = Get-LowerSha256 $prepackageAuthorityPath
    authority_binding_sha256 = [string]$authority.authority_binding_sha256
    prepackage_manifest_set_sha256 = Get-LowerSha256 $prepackageManifestSetPath
    isolated_candidate = $true
    public_artifacts_replaced = $false
    pushed = $false
    tagged = $false
    published = $false
    offline_webview_installer = $true
    installed_candidate_is_created_only_by_fresh_nsis_install = $true
    qualification_harness = [ordered]@{
        contract = "qpls.v256.multimod.build-only-packaged-qualification-harness.v1"
        cargo_feature = "multimod-qualification-harness"
        frontend_build_flag = "VITE_QPLS_MULTIMOD_QUALIFICATION_HARNESS_V1=1"
        compile_time_only = $true
        embedded_candidate_authority_required = $true
        request_or_runtime_environment_authority_forbidden = $true
        executable_specific_smoke_required = $true
        later_harness_disabled_rebuild_not_covered = $true
        unmerged_review_candidate = $true
    }
    artifacts = $artifacts
    created_at_utc = (Get-Date).ToUniversalTime().ToString("o")
}
Write-JsonAtomic -Path $outputPath -Value $receipt
$receipt | ConvertTo-Json -Depth 30
