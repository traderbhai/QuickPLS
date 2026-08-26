[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-f0-9]{40}$')][string]$CandidateCommit,
    [Parameter(Mandatory = $true)][string]$PrepackageAuthority,
    [Parameter(Mandatory = $true)][string]$PrepackageManifestSet,
    [ValidateRange(600, 6480)][int]$OverallTimeoutSeconds = 6480
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
$packageClock = [Diagnostics.Stopwatch]::StartNew()
$buildLogDirectory = Join-Path $packageDirectory "build-logs"
$publicationReserveSeconds = 120
$buildWorkDeadlineSeconds = $OverallTimeoutSeconds - $publicationReserveSeconds

function Get-RemainingBuildSeconds {
    $remaining = [Math]::Floor($buildWorkDeadlineSeconds - $packageClock.Elapsed.TotalSeconds)
    if ($remaining -lt 1) {
        throw "Candidate packaging reached its $buildWorkDeadlineSeconds-second build/preflight cutoff; $publicationReserveSeconds seconds remain reserved inside the $OverallTimeoutSeconds-second deadline. Incremental compiler outputs are retained; no candidate package was published."
    }
    return [int]$remaining
}

function Get-RemainingOverallSeconds {
    $remaining = [Math]::Floor($OverallTimeoutSeconds - $packageClock.Elapsed.TotalSeconds)
    if ($remaining -lt 1) {
        throw "Candidate packaging reached its $OverallTimeoutSeconds-second whole-operation deadline. Staged or complete uncommitted artifacts may be retained, but no passing receipt is published."
    }
    return [int]$remaining
}

function Write-TextAtomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text
    )
    $parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $temporary = Join-Path $parent (".{0}.{1}.tmp" -f ([IO.Path]::GetFileName($Path)), [Guid]::NewGuid().ToString("N"))
    try {
        [IO.File]::WriteAllText($temporary, $Text, [Text.UTF8Encoding]::new($false))
        [IO.File]::Move($temporary, $Path, $true)
    }
    finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Resolve-ExactProcessLaunch {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments
    )
    $resolvedExecutable = (Get-Command $Executable -ErrorAction Stop).Source
    $extension = [IO.Path]::GetExtension($resolvedExecutable).ToLowerInvariant()
    if ($extension -notin @(".cmd", ".bat")) {
        return [pscustomobject]@{
            RequestedExecutable = $Executable
            ResolvedExecutable = $resolvedExecutable
            FileName = $resolvedExecutable
            Arguments = @($Arguments)
            LaunchKind = "native_argument_list"
        }
    }
    $shimName = [IO.Path]::GetFileName($resolvedExecutable).ToLowerInvariant()
    $cliName = switch ($shimName) {
        "npm.cmd" { "npm-cli.js" }
        "npx.cmd" { "npx-cli.js" }
        default { throw "Command-script launch is admitted only for the npm.cmd/npx.cmd Node shims; found $resolvedExecutable" }
    }
    $shimDirectory = Split-Path -Parent $resolvedExecutable
    $nodeExecutable = Join-Path $shimDirectory "node.exe"
    if (-not (Test-Path -LiteralPath $nodeExecutable -PathType Leaf)) {
        $nodeExecutable = (Get-Command "node.exe" -ErrorAction Stop).Source
    }
    $cliPath = Join-Path $shimDirectory ("node_modules\npm\bin\{0}" -f $cliName)
    if (-not (Test-Path -LiteralPath $cliPath -PathType Leaf)) {
        throw "The exact $shimName Node CLI entry point is missing: $cliPath"
    }
    return [pscustomobject]@{
        RequestedExecutable = $Executable
        ResolvedExecutable = $resolvedExecutable
        FileName = $nodeExecutable
        Arguments = @($cliPath) + @($Arguments)
        LaunchKind = "node_cli_argument_list"
    }
}

function Stop-VerifiedProcessTree {
    param(
        [Parameter(Mandatory = $true)][Diagnostics.Process]$Process,
        [Parameter(Mandatory = $true)][string]$Stage
    )
    if ($Process.HasExited) { return }
    $primaryFailure = $null
    try {
        $Process.Kill($true)
    }
    catch {
        $primaryFailure = $_.Exception.Message
    }
    if (-not $Process.WaitForExit(10000) -and $IsWindows -and -not $Process.HasExited) {
        $taskkillInfo = [Diagnostics.ProcessStartInfo]::new()
        $taskkillInfo.FileName = (Get-Command "taskkill.exe" -ErrorAction Stop).Source
        $taskkillInfo.UseShellExecute = $false
        $taskkillInfo.CreateNoWindow = $true
        foreach ($argument in @("/PID", $Process.Id.ToString(), "/T", "/F")) {
            [void]$taskkillInfo.ArgumentList.Add($argument)
        }
        $taskkill = [Diagnostics.Process]::new()
        $taskkill.StartInfo = $taskkillInfo
        try {
            [void]$taskkill.Start()
            if (-not $taskkill.WaitForExit(10000)) {
                try { $taskkill.Kill($true) } catch { Write-Warning $_.Exception.Message }
                [void]$taskkill.WaitForExit(5000)
            }
        }
        finally {
            $taskkill.Dispose()
        }
    }
    if (-not $Process.HasExited -and -not $Process.WaitForExit(10000)) {
        $detail = if ($primaryFailure) { " Primary termination error: $primaryFailure" } else { "" }
        throw "Packaging stage $Stage retained a live process tree after verified cleanup.$detail"
    }
}

function Invoke-CheckedBounded {
    param(
        [Parameter(Mandatory = $true)][string]$Stage,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [ValidateRange(0, 6480)][int]$MaximumSeconds = 0
    )
    $remainingBuildSeconds = Get-RemainingBuildSeconds
    $stageMaximumSeconds = if ($MaximumSeconds -gt 0) {
        [Math]::Min($MaximumSeconds, $remainingBuildSeconds)
    }
    else {
        $remainingBuildSeconds
    }
    New-Item -ItemType Directory -Path $buildLogDirectory -Force | Out-Null
    $stdout = Join-Path $buildLogDirectory "$Stage.stdout.log"
    $stderr = Join-Path $buildLogDirectory "$Stage.stderr.log"
    $launch = Resolve-ExactProcessLaunch -Executable $Executable -Arguments $Arguments
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $launch.FileName
    $startInfo.WorkingDirectory = $repositoryRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in @($launch.Arguments)) { [void]$startInfo.ArgumentList.Add([string]$argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stdoutCopy = $null
    $stderrCopy = $null
    $processStarted = $false
    $executionFailure = $null
    $stageClock = [Diagnostics.Stopwatch]::StartNew()
    try {
        [void]$process.Start()
        $processStarted = $true
        $stdoutCopy = $process.StandardOutput.ReadToEndAsync()
        $stderrCopy = $process.StandardError.ReadToEndAsync()
        while (-not $process.HasExited) {
            if ($stageClock.Elapsed.TotalSeconds -ge $stageMaximumSeconds -or
                $packageClock.Elapsed.TotalSeconds -ge $buildWorkDeadlineSeconds) {
                Stop-VerifiedProcessTree -Process $process -Stage $Stage
                throw "Packaging stage $Stage exceeded its bounded $stageMaximumSeconds-second execution cutoff."
            }
            Start-Sleep -Milliseconds 250
        }
        if (-not $process.WaitForExit(10000)) { throw "Packaging stage $Stage did not finalize after exit." }
        if ($stageClock.Elapsed.TotalSeconds -gt $stageMaximumSeconds -or
            $packageClock.Elapsed.TotalSeconds -ge $buildWorkDeadlineSeconds) {
            throw "Packaging stage $Stage completed after its bounded $stageMaximumSeconds-second execution cutoff."
        }
    }
    catch {
        $executionFailure = $_
        if ($processStarted -and -not $process.HasExited) {
            try {
                Stop-VerifiedProcessTree -Process $process -Stage $Stage
            }
            catch {
                $executionFailure = [InvalidOperationException]::new(
                    "$($executionFailure.Exception.Message) Cleanup also failed: $($_.Exception.Message)"
                )
            }
        }
    }
    $drainFailure = $null
    try {
        if ($null -eq $stdoutCopy -or $null -eq $stderrCopy) {
            throw "Packaging stage $Stage did not initialize both redirected stream readers."
        }
        $tasks = [Threading.Tasks.Task[]]@($stdoutCopy, $stderrCopy)
        if (-not [Threading.Tasks.Task]::WaitAll($tasks, 30000)) { throw "Timed out draining packaging logs for $Stage." }
        Write-TextAtomic -Path $stdout -Text ([string]$stdoutCopy.Result)
        Write-TextAtomic -Path $stderr -Text ([string]$stderrCopy.Result)
    }
    catch {
        $drainFailure = $_
    }
    $exitCode = if ($processStarted -and $process.HasExited) { [int]$process.ExitCode } else { -1 }
    $elapsedMilliseconds = [long]$stageClock.ElapsedMilliseconds
    $process.Dispose()
    if ($null -ne $executionFailure) { throw $executionFailure }
    if ($null -ne $drainFailure) { throw $drainFailure }
    [void](Get-RemainingBuildSeconds)
    if ($exitCode -ne 0) {
        $tail = if (Test-Path -LiteralPath $stderr) { (Get-Content -LiteralPath $stderr -Tail 40) -join "`n" } else { "stderr unavailable" }
        throw "$Executable $($Arguments -join ' ') failed in stage $Stage with exit code $exitCode.`n$tail"
    }
    return [pscustomobject]@{
        Stage = $Stage
        RequestedExecutable = $Executable
        ResolvedExecutable = [string]$launch.ResolvedExecutable
        EffectiveExecutable = [string]$launch.FileName
        LaunchKind = [string]$launch.LaunchKind
        Arguments = @($Arguments)
        EffectiveArguments = @($launch.Arguments)
        Stdout = Get-Content -LiteralPath $stdout -Raw -Encoding UTF8
        Stderr = Get-Content -LiteralPath $stderr -Raw -Encoding UTF8
        ExitCode = $exitCode
        ElapsedMilliseconds = $elapsedMilliseconds
    }
}

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-JsonAtomic([string]$Path, $Value) {
    Write-TextAtomic -Path $Path -Text (($Value | ConvertTo-Json -Depth 30) + "`n")
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
    $manifestSet.surface -cne "standard" -or
    $manifestSet.promotion_allowed -ne $true -or
    $manifestSet.candidate_commit_sha -cne $CandidateCommit -or
    $manifestSet.candidate_version -cne $finalVersion -or
    $manifestSet.plan_sha256 -cne $planSha256 -or
    $manifestSet.gate_binding_sha256 -cne $bindingSha256 -or
    @($manifestSet.exact_profile_cells).Count -eq 0 -or
    (Compare-Object -CaseSensitive -SyncWindow 0 @($authority.binding.exact_profile_cells) @($manifestSet.exact_profile_cells))
) { throw "Prepackage candidate authority or manifest-set identity is invalid or stale." }
[void](Get-RemainingBuildSeconds)
$gitHead = Invoke-CheckedBounded -Stage "git-head" -Executable "git" -Arguments @("-C", $repositoryRoot, "rev-parse", "HEAD") -MaximumSeconds 60
$head = $gitHead.Stdout.Trim()
if ($head -cne $CandidateCommit) { throw "CandidateCommit differs from current HEAD." }
$gitStatus = Invoke-CheckedBounded -Stage "git-status" -Executable "git" -Arguments @("-C", $repositoryRoot, "status", "--porcelain=v1", "--untracked-files=all") -MaximumSeconds 60
if (-not [string]::IsNullOrWhiteSpace($gitStatus.Stdout)) { throw "Candidate worktree must be clean before packaging." }

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
    [void](Invoke-CheckedBounded -Stage "cargo-workspace-check" -Executable "cargo" -Arguments @("check", "--locked", "--workspace"))
    [void](Invoke-CheckedBounded -Stage "frontend-build" -Executable "npm.cmd" -Arguments @("run", "build"))
    # The frontend production build already passed above. Override only the
    # Tauri beforeBuild hook so packaging does not repeat that full build.
    [void](Invoke-CheckedBounded -Stage "tauri-nsis-build" -Executable "npm.cmd" -Arguments @("run", "tauri", "--", "build", "--features", "multimod-qualification-harness", "--bundles", "nsis", "--config", '{"build":{"beforeBuildCommand":""}}'))
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
$candidateDirectoryName = "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_candidate"
$finalCandidateDirectory = Join-Path $packageDirectory $candidateDirectoryName
$legacyPortable = Join-Path $packageDirectory "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_portable.exe"
$legacySetup = Join-Path $packageDirectory "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_setup.exe"
if (Test-Path -LiteralPath $outputPath) {
    throw "Candidate package receipt already exists: $outputPath"
}

# A campaign retry archives an interrupted complete directory, a legacy direct
# publication, or a staging directory before creating a new atomic candidate.
$packagePrefix = [IO.Path]::GetFullPath($packageDirectory).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$stalePaths = [System.Collections.Generic.List[string]]::new()
foreach ($candidate in @($finalCandidateDirectory, $legacyPortable, $legacySetup)) {
    if (Test-Path -LiteralPath $candidate) { $stalePaths.Add([IO.Path]::GetFullPath($candidate)) }
}
foreach ($candidate in @(Get-ChildItem -LiteralPath $packageDirectory -Directory -Filter ".$candidateDirectoryName.staging-*" -ErrorAction SilentlyContinue)) {
    $stalePaths.Add([IO.Path]::GetFullPath($candidate.FullName))
}
if ($stalePaths.Count -gt 0) {
    $historyDirectory = Join-Path $packageDirectory ("_attempt_history\package-{0}-{1}" -f [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssfffZ"), [Guid]::NewGuid().ToString("N"))
    $historyFull = [IO.Path]::GetFullPath($historyDirectory)
    if (-not $historyFull.StartsWith($packagePrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Package retry history path escapes the exact campaign package directory."
    }
    New-Item -ItemType Directory -Path $historyFull -Force | Out-Null
    foreach ($stalePath in $stalePaths) {
        if (-not $stalePath.StartsWith($packagePrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to move a stale package path outside the exact campaign package directory: $stalePath"
        }
        Move-Item -LiteralPath $stalePath -Destination (Join-Path $historyFull ([IO.Path]::GetFileName($stalePath)))
    }
    [void](Get-RemainingOverallSeconds)
}

[void](Get-RemainingOverallSeconds)
$stagingDirectory = Join-Path $packageDirectory (".{0}.staging-{1}" -f $candidateDirectoryName, [Guid]::NewGuid().ToString("N"))
$stagingFull = [IO.Path]::GetFullPath($stagingDirectory)
if (-not $stagingFull.StartsWith($packagePrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Package staging path escapes the exact campaign package directory."
}
New-Item -ItemType Directory -Path $stagingFull | Out-Null
$portableName = "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_portable.exe"
$setupName = "QuickPLS_${finalVersion}_${CandidateCommit.Substring(0,12)}_setup.exe"
$stagedPortable = Join-Path $stagingFull $portableName
$stagedSetup = Join-Path $stagingFull $setupName
$portable = Join-Path $finalCandidateDirectory $portableName
$setup = Join-Path $finalCandidateDirectory $setupName
Copy-Item -LiteralPath $sourcePortable -Destination $stagedPortable
[void](Get-RemainingOverallSeconds)
Copy-Item -LiteralPath $sourceSetup -Destination $stagedSetup
[void](Get-RemainingOverallSeconds)

$artifacts = @(
    [ordered]@{ role = "portable"; path = $portable; size = [long](Get-Item $stagedPortable).Length; sha256 = Get-LowerSha256 $stagedPortable },
    [ordered]@{ role = "setup"; path = $setup; size = [long](Get-Item $stagedSetup).Length; sha256 = Get-LowerSha256 $stagedSetup }
)
[void](Get-RemainingOverallSeconds)
if (Test-Path -LiteralPath $finalCandidateDirectory) {
    throw "Atomic candidate directory destination unexpectedly exists: $finalCandidateDirectory"
}
[IO.Directory]::Move($stagingFull, $finalCandidateDirectory)
[void](Get-RemainingOverallSeconds)

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
    build_execution = [ordered]@{
        internal_deadline_seconds = $OverallTimeoutSeconds
        build_preflight_cutoff_seconds = $buildWorkDeadlineSeconds
        publication_reserve_seconds = $publicationReserveSeconds
        cleanup_reserve_before_outer_gate_seconds = 120
        incremental_compiler_outputs_retained = $true
        package_artifacts_published_only_after_all_stages = $true
        elapsed_milliseconds = [long]$packageClock.Elapsed.TotalMilliseconds
    }
    publication = [ordered]@{
        contract = "qpls.multimod.candidate-package.atomic-directory-receipt-last.v1"
        candidate_directory = $finalCandidateDirectory
        complete_staging_directory_renamed_atomically = $true
        package_receipt_is_commit_marker = $true
        retry_archives_uncommitted_candidate_directories = $true
    }
    artifacts = $artifacts
    created_at_utc = (Get-Date).ToUniversalTime().ToString("o")
}
[void](Get-RemainingOverallSeconds)
Write-JsonAtomic -Path $outputPath -Value $receipt
