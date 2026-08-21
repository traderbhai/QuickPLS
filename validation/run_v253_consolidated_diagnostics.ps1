[CmdletBinding()]
param(
    [string]$EvidenceDir = "",
    [string]$PythonPath = "python",
    [string]$CargoPath = "cargo",
    [string]$NpmPath = "npm.cmd",
    [string]$GitPath = "git",
    [string]$RscriptPath = $env:QPLS_RSCRIPT,
    [double]$MinimumFreeGiB = 20,
    [ValidateSet("labs", "standard", "either")]
    [string]$ExpectSurface = "labs",
    [switch]$RequireR
)

# One bounded QuickPLS 2.53 diagnostic traversal.  Ordinary failures are
# collected instead of terminating the traversal.  The only fail-fast safety
# rule is disk pressure: disk-intensive commands are skipped whenever either
# C: or D: is not strictly above MinimumFreeGiB.

$ErrorActionPreference = "Stop"
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $resolvedEvidenceDir = Join-Path $resultsRoot "v253_consolidated_diagnostics_$timestamp"
} elseif ([System.IO.Path]::IsPathRooted($EvidenceDir)) {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)
} else {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $EvidenceDir))
}

$resultsRootPrefix = $resultsRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedEvidenceDir.StartsWith($resultsRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "EvidenceDir must be a new child of $resultsRoot"
}
if (Test-Path -LiteralPath $resolvedEvidenceDir) {
    throw "Refusing to reuse an existing evidence directory: $resolvedEvidenceDir"
}

$logsDir = Join-Path $resolvedEvidenceDir "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

function Write-Utf8Json {
    param(
        [Parameter(Mandatory = $true)][string]$LiteralPath,
        [Parameter(Mandatory = $true)]$Value,
        [int]$Depth = 20
    )

    $content = ($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine
    [System.IO.File]::WriteAllText($LiteralPath, $content, $utf8WithoutBom)
}

function Get-Sha256OrNull {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)

    if (-not (Test-Path -LiteralPath $LiteralPath -PathType Leaf)) {
        return $null
    }
    return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
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

function New-DiagnosticStep {
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
        disk_gate = $DiskGate
    }
}

function Invoke-DiagnosticStep {
    param([Parameter(Mandatory = $true)]$Step)

    $stdoutPath = Join-Path $logsDir "$($Step.id).stdout.log"
    $stderrPath = Join-Path $logsDir "$($Step.id).stderr.log"
    $started = (Get-Date).ToUniversalTime()
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $exitCode = -1
    $launchError = $null
    $argumentList = @($Step.arguments)
    Write-Host "[v2.53 diagnostics] $($Step.id): $($Step.description)"

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
    $status = if ($exitCode -eq 0 -and $null -eq $launchError) { "passed" } else { "failed" }
    return [ordered]@{
        id = $Step.id
        description = $Step.description
        status = $status
        reason = $launchError
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
        stdout_sha256 = Get-Sha256OrNull -LiteralPath $stdoutPath
        stderr_sha256 = Get-Sha256OrNull -LiteralPath $stderrPath
        disk_gate = $null
    }
}

$productReport = Join-Path $resolvedEvidenceDir "general_sem_v253_product_reference.json"
$referenceReport = Join-Path $resolvedEvidenceDir "general_sem_v253_reference.json"
$releaseAuditReport = Join-Path $resolvedEvidenceDir "v253_mediation_moderation_release_audit.json"
$diagnosticReport = Join-Path $resolvedEvidenceDir "v253_consolidated_diagnostics.json"
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
    (New-DiagnosticStep -Id "production_reference" -Description "Bounded qpls-runner production point, probe, and indexed-bootstrap product" -Executable $CargoPath -Arguments @(
        "run", "--locked",
        "-p", "qpls-runner",
        "--example", "general_sem_v253_product_reference",
        "--",
        "--output", $productReport,
        "--workers", "1"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "reference" -Description "Compact independent three-way and single-mediation reference" -Executable $PythonPath -Arguments $referenceArguments),
    (New-DiagnosticStep -Id "registry" -Description "Capability Registry V2 structure and legacy projection validation" -Executable $PythonPath -Arguments @("validation/capability_registry_v2.py", "--check-legacy", "--skip-reference-check")),
    (New-DiagnosticStep -Id "manifests" -Description "New exact-cell promotion-manifest validation" -Executable $PythonPath -Arguments @(
        "validation/method_promotion_manifest.py",
        "validation/methods/general_sem_pls_single_mediation_bootstrap_v1.manifest.json",
        "validation/methods/general_sem_pls_three_way_moderation_point_v1.manifest.json",
        "validation/methods/general_sem_pls_three_way_moderation_bootstrap_v1.manifest.json",
        "--json"
    )),
    (New-DiagnosticStep -Id "rustfmt" -Description "Rust formatting check" -Executable $CargoPath -Arguments @("fmt", "--all", "--", "--check")),
    (New-DiagnosticStep -Id "diff_check" -Description "Git whitespace-error check" -Executable $GitPath -Arguments @("diff", "--check")),
    (New-DiagnosticStep -Id "rust_check" -Description "Consolidated Rust compilation" -Executable $CargoPath -Arguments @(
        "check", "--locked",
        "-p", "qpls-core",
        "-p", "qpls-estimation",
        "-p", "qpls-resampling",
        "-p", "qpls-project",
        "-p", "qpls-runner",
        "-p", "quickpls-desktop"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_core_three_way" -Description "Focused core three-way contracts" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-core", "three_way", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_core_single_mediation" -Description "Focused core single-mediation routing" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-core", "single_indirect_path", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_estimation_three_way" -Description "Focused three-way point estimator" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-estimation", "general_sem_pls_three_way_v1::tests", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_resampling_three_way" -Description "Focused three-way complete-refit bootstrap" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-resampling", "three_way_bootstrap", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_resampling_single_mediation" -Description "Distinct single-mediation bootstrap identity" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-resampling", "single_mediation_bootstrap_identity", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_project_moderation_revision" -Description "Focused moderation add, retarget, three-way hierarchy, and removal lifecycle" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "moderating_effect_v3_add_retarget_extend_and_remove_preserves_shared_hierarchy", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_project_three_way_archive" -Description "Focused three-way canonical append, reopen, and tamper rejection" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "three_way_canonical_append_reopen_and_tamper_fail_closed", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_project_single_mediation_archive" -Description "Focused single-mediation exact-cell reopen and identity" -Executable $CargoPath -Arguments @("test", "--locked", "-p", "qpls-project", "single_mediation_exact_cell_reopens_with_stable_identity", "--", "--nocapture") -DiskIntensive $true),
    (New-DiagnosticStep -Id "frontend_workflows" -Description "Focused Canvas, Calculate, Results, persistence, and compatibility workflows" -Executable $NpmPath -Arguments (@("run", "test", "--") + $frontendTests) -DiskIntensive $true),
    (New-DiagnosticStep -Id "frontend_typecheck" -Description "One full frontend typecheck" -Executable $NpmPath -Arguments @("run", "typecheck:full") -DiskIntensive $true),
    (New-DiagnosticStep -Id "frontend_build" -Description "One production frontend bundle" -Executable $NpmPath -Arguments @("run", "build:bundle") -DiskIntensive $true),
    (New-DiagnosticStep -Id "release_audit" -Description "2.53 source, exact routing, Registry, manifests, and reference-evidence audit" -Executable $PythonPath -Arguments @(
        "validation/v253_mediation_moderation_release_audit.py",
        "--expect-surface", $ExpectSurface,
        "--reference-report", $referenceReport,
        "--output", $releaseAuditReport
    ))
)

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
    $initialDisk = Get-DiskSnapshot -Label "before_consolidated_pass"
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

        $result = Invoke-DiagnosticStep -Step $step
        $stepResults.Add($result)

        if ($step.disk_intensive) {
            $afterStepDisk = Get-DiskSnapshot -Label "after_$($step.id)"
            $diskSnapshots.Add($afterStepDisk)
            $result.disk_gate = [ordered]@{
                before = $gate
                after = $afterStepDisk
            }
            if (-not $afterStepDisk.passed) {
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

$finalDisk = Get-DiskSnapshot -Label "after_consolidated_pass"
$diskSnapshots.Add($finalDisk)
$endedAt = (Get-Date).ToUniversalTime()
$failedSteps = @($stepResults | Where-Object { $_.status -eq "failed" } | ForEach-Object { $_.id })
$skippedSteps = @($stepResults | Where-Object { $_.status -eq "skipped" } | ForEach-Object { $_.id })
$passedSteps = @($stepResults | Where-Object { $_.status -eq "passed" } | ForEach-Object { $_.id })
$passed = $failedSteps.Count -eq 0 -and $skippedSteps.Count -eq 0 -and -not $diskPressureDetected -and $finalDisk.passed

$report = [ordered]@{
    schema_version = 1
    suite_id = "quickpls_v253_consolidated_diagnostics_v1"
    version = "2.53.0"
    passed = $passed
    repository_root = $repositoryRoot
    evidence_directory = $resolvedEvidenceDir
    started_at = $startedAt.ToString("o")
    ended_at = $endedAt.ToString("o")
    duration_ms = [math]::Round(($endedAt - $startedAt).TotalMilliseconds)
    policy = [ordered]@{
        ordinary_command_failures = "record and continue"
        disk_safety_failures = "skip disk-intensive commands"
        minimum_free_gib_exclusive = $MinimumFreeGiB
        cargo_incremental = 0
        cargo_target_dir = Join-Path $repositoryRoot "target"
        repeated_historical_scientific_matrices = $false
        packaged_or_browser_execution = $false
        expected_registry_surface = $ExpectSurface
    }
    summary = [ordered]@{
        total = $stepResults.Count
        passed = $passedSteps.Count
        failed = $failedSteps.Count
        skipped = $skippedSteps.Count
        failed_step_ids = $failedSteps
        skipped_step_ids = $skippedSteps
    }
    evidence = [ordered]@{
        production_reference = $productReport
        compact_reference = $referenceReport
        release_audit = $releaseAuditReport
    }
    disk_snapshots = @($diskSnapshots)
    steps = @($stepResults)
}

Write-Utf8Json -LiteralPath $diagnosticReport -Value $report -Depth 30
$report | ConvertTo-Json -Depth 30
if (-not $passed) {
    exit 1
}
