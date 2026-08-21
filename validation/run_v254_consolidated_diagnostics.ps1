[CmdletBinding()]
param(
    [string]$EvidenceDir = "",
    [string]$CargoPath = "cargo",
    [string]$NpmPath = "npm.cmd",
    [string]$NodePath = "node",
    [string]$GitPath = "git",
    [double]$MinimumFreeGiB = 20.0
)

# One bounded source diagnostic for the QuickPLS 2.54 Canvas/Results release.
# Commands are deliberately run once and failures are collected so one pass
# yields one correction list. Disk-intensive commands are fail-closed whenever
# either C: or D: is not strictly above the configured free-space threshold.

$ErrorActionPreference = "Stop"
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $resolvedEvidenceDir = Join-Path $resultsRoot "v254_consolidated_diagnostics_$timestamp"
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
        [int]$Depth = 24
    )
    [System.IO.File]::WriteAllText(
        $LiteralPath,
        (($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine),
        $utf8WithoutBom
    )
}

function Get-Sha256OrNull {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)
    if (-not (Test-Path -LiteralPath $LiteralPath -PathType Leaf)) { return $null }
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
            $passed = $freeGiB -gt $MinimumFreeGiB
            $drives.Add([ordered]@{
                name = $driveName
                free_gib = $freeGiB
                threshold_gib_exclusive = $MinimumFreeGiB
                passed = $passed
            })
            if (-not $passed) {
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
    return (@($Executable) + @($Arguments | ForEach-Object {
        $argument = [string]$_
        if ($argument -match '[\s"]') { '"' + $argument.Replace('"', '\"') + '"' } else { $argument }
    })) -join " "
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
    Write-Host "[v2.54 diagnostics] $($Step.id): $($Step.description)"

    try {
        $priorErrorPreference = $ErrorActionPreference
        $priorNativePreference = if (Test-Path Variable:PSNativeCommandUseErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference
        } else { $null }
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
    return [ordered]@{
        id = $Step.id
        description = $Step.description
        status = if ($exitCode -eq 0 -and $null -eq $launchError) { "passed" } else { "failed" }
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

$steps = @(
    (New-DiagnosticStep -Id "diff_check" -Description "Git whitespace-error check" -Executable $GitPath -Arguments @("diff", "--check")),
    (New-DiagnosticStep -Id "frontend_full_vitest" -Description "One full Vitest traversal across the frontend and domain suite" -Executable $NpmPath -Arguments @("run", "test") -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_authority" -Description "Focused embedded Registry authority test" -Executable $CargoPath -Arguments @(
        "test", "--locked", "-p", "qpls-core",
        "embedded_registry_is_the_exact_option_cell_authority",
        "--", "--nocapture"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_archive" -Description "Focused schema-6 model-authority archive author/save/reopen test" -Executable $CargoPath -Arguments @(
        "test", "--locked", "-p", "qpls-project",
        "--test", "schema6_sem_model_v4_authoring_shapes",
        "section_3_1_shapes_author_serialize_and_reopen_through_standalone_schema6",
        "--", "--exact", "--nocapture"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_routing" -Description "Focused exact Registry three-way point/bootstrap routing test" -Executable $CargoPath -Arguments @(
        "test", "--locked", "-p", "qpls-core",
        "bounded_three_way_point_and_bootstrap_route_to_their_distinct_cells",
        "--", "--nocapture"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "rust_check" -Description "One consolidated Rust compile check; no repeated scientific qualification matrices" -Executable $CargoPath -Arguments @(
        "check", "--locked",
        "-p", "qpls-core",
        "-p", "qpls-project",
        "-p", "qpls-runner",
        "-p", "quickpls-desktop"
    ) -DiskIntensive $true),
    (New-DiagnosticStep -Id "frontend_typecheck" -Description "One full frontend typecheck" -Executable $NpmPath -Arguments @("run", "typecheck:full") -DiskIntensive $true),
    (New-DiagnosticStep -Id "frontend_build" -Description "One production frontend bundle" -Executable $NpmPath -Arguments @("run", "build:bundle") -DiskIntensive $true),
    (New-DiagnosticStep -Id "headless_canvas_results" -Description "Headless Canvas and Results interaction crawl against the production bundle" -Executable $NodePath -Arguments @(
        "validation/v254_canvas_results_packaged_smoke.mjs",
        "--phase", "headless",
        "--evidence-dir", $resolvedEvidenceDir,
        "--port", "57654"
    ) -DiskIntensive $true)
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
    if (-not $initialDisk.passed) { $diskPressureDetected = $true }

    foreach ($step in $steps) {
        $gate = $null
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
            $result.disk_gate = [ordered]@{ before = $gate; after = $afterStepDisk }
            if (-not $afterStepDisk.passed) { $diskPressureDetected = $true }
        }
    }
} finally {
    if ($null -eq $priorCargoTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    else { $env:CARGO_TARGET_DIR = $priorCargoTarget }
    if ($null -eq $priorCargoIncremental) { Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue }
    else { $env:CARGO_INCREMENTAL = $priorCargoIncremental }
    Set-Location -LiteralPath $priorLocation
}

$finalDisk = Get-DiskSnapshot -Label "after_consolidated_pass"
$diskSnapshots.Add($finalDisk)
$endedAt = (Get-Date).ToUniversalTime()
$failedSteps = @($stepResults | Where-Object { $_.status -eq "failed" } | ForEach-Object { $_.id })
$skippedSteps = @($stepResults | Where-Object { $_.status -eq "skipped" } | ForEach-Object { $_.id })
$passedSteps = @($stepResults | Where-Object { $_.status -eq "passed" } | ForEach-Object { $_.id })
$passed = $failedSteps.Count -eq 0 -and $skippedSteps.Count -eq 0 -and -not $diskPressureDetected -and $finalDisk.passed
$reportPath = Join-Path $resolvedEvidenceDir "v254_consolidated_diagnostics.json"
$report = [ordered]@{
    schema_version = 1
    suite_id = "quickpls_v254_canvas_results_consolidated_diagnostics_v1"
    version = "2.54.0"
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
        packaged_execution = $false
        headless_browser_execution = $true
        code_signing = $false
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
        headless_canvas_results_crawl = Join-Path $resolvedEvidenceDir "v254_canvas_results_headless_crawl.json"
        packaged_smoke = "run separately against the one frozen unsigned Windows candidate"
    }
    disk_snapshots = @($diskSnapshots)
    steps = @($stepResults)
}

Write-Utf8Json -LiteralPath $reportPath -Value $report -Depth 30
$report | ConvertTo-Json -Depth 30
if (-not $passed) { exit 1 }
