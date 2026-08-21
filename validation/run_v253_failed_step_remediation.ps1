[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReport,
    [string]$EvidenceDir = "",
    [string]$PythonPath = "python",
    [string]$CargoPath = "cargo",
    [string]$NpmPath = "npm.cmd",
    [string]$GitPath = "git",
    [string]$RscriptPath = $env:QPLS_RSCRIPT,
    [double]$MinimumFreeGiB = 20,
    [ValidateSet("labs")]
    [string]$ExpectSurface = "labs",
    [switch]$RequireR
)

# This runner is intentionally narrower than the consolidated pass. It accepts
# only the exact ten failed IDs recorded by the Version 2.53 Labs pass 2 report,
# reruns each once, and binds the baseline report, current source bytes, commands,
# logs, disk gates, and outputs into one immutable JSON plus a detached digest.

$ErrorActionPreference = "Stop"
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$expectedFailedIds = @(
    "production_reference",
    "reference",
    "rust_core_three_way",
    "rust_core_single_mediation",
    "rust_project_three_way_archive",
    "rust_project_single_mediation_archive",
    "rust_project_moderation_revision",
    "frontend_workflows",
    "frontend_typecheck",
    "release_audit"
)
$excludedSourcePrefixes = @(
    ".git/",
    ".vite/",
    "dist/",
    "node_modules/",
    "target/",
    "validation/results/"
)

function Write-Utf8Json {
    param(
        [Parameter(Mandatory = $true)][string]$LiteralPath,
        [Parameter(Mandatory = $true)]$Value,
        [int]$Depth = 30
    )
    $content = ($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine
    [System.IO.File]::WriteAllText($LiteralPath, $content, $utf8WithoutBom)
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)
    return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-StringSha256 {
    param([Parameter(Mandatory = $true)][string]$Value)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    $digest = [System.Security.Cryptography.SHA256]::HashData($bytes)
    return [System.Convert]::ToHexString($digest).ToLowerInvariant()
}

function Get-RepositoryRelativePath {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)
    $absolute = [System.IO.Path]::GetFullPath($LiteralPath)
    $rootPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $absolute.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside the repository: $absolute"
    }
    return [System.IO.Path]::GetRelativePath($repositoryRoot, $absolute).Replace("\", "/")
}

function Get-FileDescriptor {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)
    $item = Get-Item -LiteralPath $LiteralPath -ErrorAction Stop
    return [ordered]@{
        path = Get-RepositoryRelativePath -LiteralPath $item.FullName
        size = [long]$item.Length
        sha256 = Get-Sha256 -LiteralPath $item.FullName
    }
}

function Get-SourceBinding {
    $paths = @(& $GitPath -C $repositoryRoot ls-files --cached --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) {
        throw "git ls-files failed while creating the source binding"
    }
    $normalized = @(
        $paths |
            ForEach-Object { ([string]$_).Replace("\", "/") } |
            Where-Object {
                $candidate = $_
                -not [string]::IsNullOrWhiteSpace($candidate) -and
                -not ($excludedSourcePrefixes | Where-Object { $candidate.StartsWith($_, [System.StringComparison]::Ordinal) })
            } |
            Sort-Object -Unique
    )
    $files = [System.Collections.Generic.List[object]]::new()
    $records = [System.Text.StringBuilder]::new()
    foreach ($relative in $normalized) {
        $absolute = Join-Path $repositoryRoot $relative
        if (Test-Path -LiteralPath $absolute -PathType Leaf) {
            $item = Get-Item -LiteralPath $absolute
            $size = [long]$item.Length
            $sha = Get-Sha256 -LiteralPath $absolute
        } else {
            $size = -1
            $sha = "missing"
        }
        $files.Add([ordered]@{ path = $relative; size = $size; sha256 = $sha })
        [void]$records.Append($relative).Append("`t").Append($size).Append("`t").Append($sha).Append("`n")
    }
    $head = (& $GitPath -C $repositoryRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($head)) {
        throw "git rev-parse HEAD failed while creating the source binding"
    }
    return [ordered]@{
        algorithm = "git_tracked_and_untracked_source_manifest_sha256_v1"
        git_head = $head
        excluded_prefixes = @($excludedSourcePrefixes)
        file_count = $files.Count
        manifest_sha256 = Get-StringSha256 -Value $records.ToString()
        files = @($files)
    }
}

function Get-DiskSnapshot {
    param([Parameter(Mandatory = $true)][string]$Label)
    $drives = [System.Collections.Generic.List[object]]::new()
    $errors = [System.Collections.Generic.List[string]]::new()
    foreach ($driveName in @("C", "D")) {
        try {
            $drive = Get-PSDrive -Name $driveName -PSProvider FileSystem -ErrorAction Stop
            $freeGiB = [math]::Round($drive.Free / 1GB, 3)
            $drivePassed = $freeGiB -gt $MinimumFreeGiB
            $drives.Add([ordered]@{
                name = $driveName
                free_gib = $freeGiB
                threshold_gib_exclusive = $MinimumFreeGiB
                passed = $drivePassed
            })
            if (-not $drivePassed) {
                $errors.Add("${driveName}: free space is not strictly above $MinimumFreeGiB GiB")
            }
        } catch {
            $drives.Add([ordered]@{
                name = $driveName
                free_gib = $null
                threshold_gib_exclusive = $MinimumFreeGiB
                passed = $false
            })
            $errors.Add("${driveName}: could not read free space: $($_.Exception.Message)")
        }
    }
    return [ordered]@{
        label = $Label
        captured_at = (Get-Date).ToUniversalTime().ToString("o")
        passed = $errors.Count -eq 0
        drives = @($drives)
        errors = @($errors)
    }
}

function Format-CommandLine {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][object[]]$Arguments
    )
    $rendered = @($Executable) + @($Arguments | ForEach-Object {
        $argument = [string]$_
        if ($argument -match '[\s"]') {
            '"' + $argument.Replace('"', '\"') + '"'
        } else {
            $argument
        }
    })
    return $rendered -join " "
}

function New-RemediationStep {
    param(
        [Parameter(Mandatory = $true)][string]$Id,
        [Parameter(Mandatory = $true)][string]$Description,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][object[]]$Arguments,
        [bool]$DiskIntensive = $false
    )
    return [ordered]@{
        id = $Id
        description = $Description
        executable = $Executable
        arguments = @($Arguments | ForEach-Object { [string]$_ })
        disk_intensive = $DiskIntensive
    }
}

function New-SkippedStepResult {
    param(
        [Parameter(Mandatory = $true)]$Step,
        [Parameter(Mandatory = $true)][string]$Reason,
        $DiskGate = $null
    )
    return [ordered]@{
        id = $Step.id
        description = $Step.description
        status = "skipped"
        reason = $Reason
        command = Format-CommandLine -Executable $Step.executable -Arguments $Step.arguments
        executable = $Step.executable
        arguments = @($Step.arguments)
        disk_intensive = $Step.disk_intensive
        started_at = $null
        ended_at = $null
        duration_ms = 0
        exit_code = $null
        stdout = $null
        stderr = $null
        stdout_sha256 = $null
        stderr_sha256 = $null
        stderr_contains_capability_unavailable = $false
        disk_gate = $DiskGate
    }
}

$baselinePath = [System.IO.Path]::GetFullPath($BaselineReport)
$resultsPrefix = $resultsRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $baselinePath.StartsWith($resultsPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "BaselineReport must be under $resultsRoot"
}
if (-not (Test-Path -LiteralPath $baselinePath -PathType Leaf)) {
    throw "BaselineReport does not exist: $baselinePath"
}
$baseline = Get-Content -LiteralPath $baselinePath -Raw | ConvertFrom-Json
if ($baseline.schema_version -ne 1 -or $baseline.suite_id -ne "quickpls_v253_consolidated_diagnostics_v1" -or $baseline.version -ne "2.53.0") {
    throw "BaselineReport is not the Version 2.53 consolidated diagnostic"
}
if ($baseline.passed -ne $false -or $baseline.summary.skipped -ne 0) {
    throw "BaselineReport must be the finite failed pass with no skipped steps"
}
$actualFailedIds = @($baseline.summary.failed_step_ids | ForEach-Object { [string]$_ })
if ((Compare-Object ($expectedFailedIds | Sort-Object) ($actualFailedIds | Sort-Object)).Count -ne 0) {
    throw "BaselineReport failed IDs are not the exact bounded ten-step recovery set"
}
if ([System.IO.Path]::GetFullPath([string]$baseline.repository_root) -ne $repositoryRoot) {
    throw "BaselineReport repository root differs from the current repository"
}

if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $resolvedEvidenceDir = Join-Path $resultsRoot "v253_failed_step_remediation_$timestamp"
} elseif ([System.IO.Path]::IsPathRooted($EvidenceDir)) {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)
} else {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $EvidenceDir))
}
if (-not $resolvedEvidenceDir.StartsWith($resultsPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "EvidenceDir must be a new child of $resultsRoot"
}
if (Test-Path -LiteralPath $resolvedEvidenceDir) {
    throw "Refusing to reuse an existing evidence directory: $resolvedEvidenceDir"
}
$logsDir = Join-Path $resolvedEvidenceDir "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

function Invoke-RemediationStep {
    param([Parameter(Mandatory = $true)]$Step)
    $stdoutPath = Join-Path $logsDir "$($Step.id).stdout.log"
    $stderrPath = Join-Path $logsDir "$($Step.id).stderr.log"
    $started = (Get-Date).ToUniversalTime()
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $exitCode = -1
    $launchError = $null
    $argumentList = @($Step.arguments)
    Write-Host "[v2.53 remediation] $($Step.id): $($Step.description)"
    try {
        $priorErrorPreference = $ErrorActionPreference
        $priorNativePreference = if (Test-Path Variable:PSNativeCommandUseErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference
        } else {
            $null
        }
        $ErrorActionPreference = "Continue"
        if (Test-Path Variable:PSNativeCommandUseErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference = $false
        }
        try {
            & $Step.executable @argumentList 1> $stdoutPath 2> $stderrPath
            $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        } finally {
            $ErrorActionPreference = $priorErrorPreference
            if (Test-Path Variable:PSNativeCommandUseErrorActionPreference) {
                $PSNativeCommandUseErrorActionPreference = $priorNativePreference
            }
        }
    } catch {
        $launchError = $_.Exception.Message
        [System.IO.File]::AppendAllText(
            $stderrPath,
            "command launch failure: $launchError$([Environment]::NewLine)",
            $utf8WithoutBom
        )
    } finally {
        $stopwatch.Stop()
    }
    $ended = (Get-Date).ToUniversalTime()
    $stderrContainsCapabilityUnavailable = $false
    if (Test-Path -LiteralPath $stderrPath -PathType Leaf) {
        $stderrContainsCapabilityUnavailable = [bool](Select-String -LiteralPath $stderrPath -SimpleMatch "CapabilityUnavailable" -Quiet)
    }
    $deferred = (
        $Step.id -in @("rust_project_three_way_archive", "rust_project_single_mediation_archive") -and
        $exitCode -eq 101 -and
        $stderrContainsCapabilityUnavailable
    )
    $status = if ($exitCode -eq 0 -and $null -eq $launchError) {
        "passed"
    } elseif ($deferred) {
        "deferred"
    } else {
        "failed"
    }
    $reason = if ($deferred) { "prepromotion_capability_unavailable" } else { $launchError }
    return [ordered]@{
        id = $Step.id
        description = $Step.description
        status = $status
        reason = $reason
        command = Format-CommandLine -Executable $Step.executable -Arguments $Step.arguments
        executable = $Step.executable
        arguments = @($Step.arguments)
        disk_intensive = $Step.disk_intensive
        started_at = $started.ToString("o")
        ended_at = $ended.ToString("o")
        duration_ms = $stopwatch.ElapsedMilliseconds
        exit_code = $exitCode
        stdout = [System.IO.Path]::GetRelativePath($resolvedEvidenceDir, $stdoutPath).Replace("\", "/")
        stderr = [System.IO.Path]::GetRelativePath($resolvedEvidenceDir, $stderrPath).Replace("\", "/")
        stdout_sha256 = if (Test-Path -LiteralPath $stdoutPath) { Get-Sha256 -LiteralPath $stdoutPath } else { $null }
        stderr_sha256 = if (Test-Path -LiteralPath $stderrPath) { Get-Sha256 -LiteralPath $stderrPath } else { $null }
        stderr_contains_capability_unavailable = $stderrContainsCapabilityUnavailable
        disk_gate = $null
    }
}

$productReport = Join-Path $resolvedEvidenceDir "general_sem_v253_product_reference.json"
$referenceReport = Join-Path $resolvedEvidenceDir "general_sem_v253_reference.json"
$releaseAuditReport = Join-Path $resolvedEvidenceDir "v253_mediation_moderation_release_audit.json"
$remediationReport = Join-Path $resolvedEvidenceDir "v253_failed_step_remediation.json"
$remediationReceipt = [System.IO.Path]::ChangeExtension($remediationReport, ".receipt.json")

$referenceArguments = @(
    "validation/general_sem_v253_reference.py",
    "--product-json", $productReport,
    "--require-product",
    "--output", $referenceReport
)
if ($RequireR) {
    if ([string]::IsNullOrWhiteSpace($RscriptPath)) {
        $localRRoot = Join-Path $env:LOCALAPPDATA "Programs\R"
        $RscriptPath = Get-ChildItem -LiteralPath $localRRoot -Filter Rscript.exe -File -Recurse -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            Select-Object -First 1 -ExpandProperty FullName
    }
    if (-not [string]::IsNullOrWhiteSpace($RscriptPath)) {
        $referenceArguments += @("--rscript", [System.IO.Path]::GetFullPath($RscriptPath))
    }
    $referenceArguments += "--require-r"
}

$frontendTests = @(
    "src/domain/moderationDiagramProjectionV1.test.ts",
    "src/domain/diagramGraph.test.ts",
    "src/components/ModelCanvas.standardAuthority.test.ts",
    "src/components/ConstructNode.presentation.test.ts",
    "src/native/nativeModeration.test.ts",
    "src/native/NativeModerationDialog.test.tsx",
    "src/native/NativeDesktopApp.test.ts",
    "src/native/nativeCommandAccessibilityContracts.test.ts",
    "src/domain/standardSemModelV4Authority.test.ts",
    "src/domain/nativeWorkbenchSemModelV4Adapter.test.ts",
    "src/domain/unifiedSemCalculationV1.test.ts",
    "src/domain/generalSemCapabilityPreflightV1.test.ts",
    "src/domain/generalSemModeratedMediationAuthoringV1.test.ts",
    "src/domain/canonicalGeneralSemThreeWayV1.test.ts",
    "src/domain/canonicalThreeWayModerationPresentationV1.test.ts",
    "src/domain/canonicalResultNavigationV1.test.ts",
    "src/native/NativeCalculationDialog.test.ts",
    "src/native/nativeCommands.test.ts",
    "src/native/nativeResults.test.ts",
    "src/native/nativeCanonicalResultDocumentV2.test.ts",
    "src/native/NativeResultsSurface.test.tsx",
    "src/domain/internalProjectSchema6ResultRead.test.ts",
    "src/domain/internalGeneralSemExecutionAuthorityRevisionV1.test.ts",
    "src/domain/internalRecipeV4GeneralSemWorkspace.test.ts",
    "src/services/internalGeneralSemExecutionAuthorityRevisionService.test.ts",
    "src/store.test.ts"
)

$steps = @(
    (New-RemediationStep -Id "production_reference" -Description "Bounded qpls-runner production reference" -Executable $CargoPath -Arguments @(
        "run", "--locked", "-p", "qpls-runner", "--example", "general_sem_v253_product_reference", "--",
        "--output", $productReport, "--workers", "1"
    ) -DiskIntensive $true),
    (New-RemediationStep -Id "reference" -Description "Compact independent Python/R/product reference" -Executable $PythonPath -Arguments $referenceArguments),
    (New-RemediationStep -Id "rust_core_three_way" -Description "Focused core three-way contracts" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-core", "three_way", "--", "--nocapture") -DiskIntensive $true),
    (New-RemediationStep -Id "rust_core_single_mediation" -Description "Focused core single-mediation routing" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-core", "single_indirect_path", "--", "--nocapture") -DiskIntensive $true),
    (New-RemediationStep -Id "rust_project_three_way_archive" -Description "Focused three-way canonical append/reopen/tamper" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "three_way_canonical_append_reopen_and_tamper_fail_closed", "--", "--nocapture") -DiskIntensive $true),
    (New-RemediationStep -Id "rust_project_single_mediation_archive" -Description "Focused single-mediation reopen identity" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "single_mediation_exact_cell_reopens_with_stable_identity", "--", "--nocapture") -DiskIntensive $true),
    (New-RemediationStep -Id "rust_project_moderation_revision" -Description "Focused moderation add/retarget/extend/remove lifecycle" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "moderating_effect_v3_add_retarget_extend_and_remove_preserves_shared_hierarchy", "--", "--nocapture") -DiskIntensive $true),
    (New-RemediationStep -Id "frontend_workflows" -Description "Focused Canvas/Calculate/Results/persistence workflows" -Executable $NpmPath -Arguments (@("run", "test", "--") + $frontendTests) -DiskIntensive $true),
    (New-RemediationStep -Id "frontend_typecheck" -Description "Full frontend typecheck" -Executable $NpmPath -Arguments @("run", "typecheck:full") -DiskIntensive $true),
    (New-RemediationStep -Id "release_audit" -Description "2.53 source/routing/Registry/reference audit" -Executable $PythonPath -Arguments @(
        "validation/v253_mediation_moderation_release_audit.py",
        "--expect-surface", $ExpectSurface,
        "--reference-report", $referenceReport,
        "--output", $releaseAuditReport
    ))
)
if ((Compare-Object ($steps.id | Sort-Object) ($expectedFailedIds | Sort-Object)).Count -ne 0) {
    throw "Recovery step definitions differ from the bounded failed-step set"
}

$sourceBefore = Get-SourceBinding
$priorCargoTarget = [Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Process")
$priorCargoIncremental = [Environment]::GetEnvironmentVariable("CARGO_INCREMENTAL", "Process")
$priorLocation = (Get-Location).Path
$env:CARGO_TARGET_DIR = Join-Path $repositoryRoot "target"
$env:CARGO_INCREMENTAL = "0"
$stepResults = [System.Collections.Generic.List[object]]::new()
$diskSnapshots = [System.Collections.Generic.List[object]]::new()
$diskPressureDetected = $false
$startedAt = (Get-Date).ToUniversalTime()

try {
    Set-Location -LiteralPath $repositoryRoot
    $initialDisk = Get-DiskSnapshot -Label "before_v253_failed_step_remediation"
    $diskSnapshots.Add($initialDisk)
    if (-not $initialDisk.passed) {
        $diskPressureDetected = $true
    }
    foreach ($step in $steps) {
        if ($step.disk_intensive) {
            $gate = Get-DiskSnapshot -Label "before_$($step.id)"
            $diskSnapshots.Add($gate)
            if (-not $gate.passed) {
                $diskPressureDetected = $true
                $stepResults.Add((New-SkippedStepResult -Step $step -Reason "disk_safety_gate_failed" -DiskGate $gate))
                continue
            }
        }
        $result = Invoke-RemediationStep -Step $step
        $stepResults.Add($result)
        if ($step.disk_intensive) {
            $after = Get-DiskSnapshot -Label "after_$($step.id)"
            $diskSnapshots.Add($after)
            $result.disk_gate = [ordered]@{ before = $gate; after = $after }
            if (-not $after.passed) {
                $diskPressureDetected = $true
            }
        }
    }
} finally {
    if ($null -eq $priorCargoTarget) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $priorCargoTarget
    }
    if ($null -eq $priorCargoIncremental) {
        Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_INCREMENTAL = $priorCargoIncremental
    }
    Set-Location -LiteralPath $priorLocation
}

$finalDisk = Get-DiskSnapshot -Label "after_v253_failed_step_remediation"
$diskSnapshots.Add($finalDisk)
$sourceAfter = Get-SourceBinding
$sourceStable = $sourceBefore.manifest_sha256 -eq $sourceAfter.manifest_sha256 -and $sourceBefore.git_head -eq $sourceAfter.git_head
$endedAt = (Get-Date).ToUniversalTime()
$failedSteps = @($stepResults | Where-Object { $_.status -eq "failed" } | ForEach-Object { $_.id })
$skippedSteps = @($stepResults | Where-Object { $_.status -eq "skipped" } | ForEach-Object { $_.id })
$deferredSteps = @($stepResults | Where-Object { $_.status -eq "deferred" } | ForEach-Object { $_.id })
$passedSteps = @($stepResults | Where-Object { $_.status -eq "passed" } | ForEach-Object { $_.id })
$expectedDeferredIds = @("rust_project_three_way_archive", "rust_project_single_mediation_archive")
$deferredExact = (Compare-Object ($deferredSteps | Sort-Object) ($expectedDeferredIds | Sort-Object)).Count -eq 0
$promotionEligible = (
    $failedSteps.Count -eq 0 -and
    $skippedSteps.Count -eq 0 -and
    $passedSteps.Count -eq 8 -and
    $deferredSteps.Count -eq 2 -and
    $deferredExact -and
    -not $diskPressureDetected -and
    $finalDisk.passed -and
    $sourceStable
)
$passed = $promotionEligible

$report = [ordered]@{
    schema_version = 1
    suite_id = "quickpls_v253_failed_step_remediation_v1"
    version = "2.53.0"
    passed = $passed
    promotion_eligible = $promotionEligible
    release_complete = $false
    repository_root = $repositoryRoot
    evidence_directory = $resolvedEvidenceDir
    baseline_report = Get-FileDescriptor -LiteralPath $baselinePath
    selected_step_ids = @($expectedFailedIds)
    started_at = $startedAt.ToString("o")
    ended_at = $endedAt.ToString("o")
    duration_ms = [math]::Round(($endedAt - $startedAt).TotalMilliseconds)
    policy = [ordered]@{
        selection = "exact_failed_step_ids_from_bound_baseline"
        no_third_broad_pass = $true
        ordinary_command_failures = "record and continue"
        disk_safety_failures = "skip disk-intensive commands"
        minimum_free_gib_exclusive = $MinimumFreeGiB
        cargo_incremental = 0
        expected_registry_surface = $ExpectSurface
    }
    summary = [ordered]@{
        total = $stepResults.Count
        passed = $passedSteps.Count
        deferred = $deferredSteps.Count
        failed = $failedSteps.Count
        skipped = $skippedSteps.Count
        deferred_step_ids = $deferredSteps
        failed_step_ids = $failedSteps
        skipped_step_ids = $skippedSteps
    }
    evidence = [ordered]@{
        production_reference = $productReport
        compact_reference = $referenceReport
        release_audit = $releaseAuditReport
    }
    source_binding = $sourceBefore
    source_binding_after = [ordered]@{
        stable = $sourceStable
        manifest_sha256 = $sourceAfter.manifest_sha256
        git_head = $sourceAfter.git_head
    }
    disk_snapshots = @($diskSnapshots)
    steps = @($stepResults)
}

Write-Utf8Json -LiteralPath $remediationReport -Value $report -Depth 50
$reportDescriptor = Get-FileDescriptor -LiteralPath $remediationReport
Write-Utf8Json -LiteralPath $remediationReceipt -Value ([ordered]@{
    schema_version = 1
    receipt_kind = "quickpls_v253_failed_step_remediation_sha256_v1"
    report = $reportDescriptor
})
$report | ConvertTo-Json -Depth 50
if (-not $passed) {
    exit 1
}
