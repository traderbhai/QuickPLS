[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][UInt64]$Seed
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "../..")).Path
$outputPath = [IO.Path]::GetFullPath($Output)
$outputDirectory = Split-Path -Parent $outputPath
$workRoot = Join-Path $outputDirectory "maximum-profile-performance-work"
$verifier = Join-Path $PSScriptRoot "verify_multimod_performance_profiles_v1.py"
$budgets = [ordered]@{
    mga_20_groups_190_pairs = [ordered]@{ maximum_seconds = 10800; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 2GB }
    heterogeneity_locked_p23 = [ordered]@{ maximum_seconds = 14400; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 4GB }
    conditional_sidecar_resume = [ordered]@{ maximum_seconds = 10800; maximum_peak_working_set_bytes = 12GB; maximum_output_bytes = 2GB }
}

function Get-LowerSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-JsonAtomic([string]$Path, $Value) {
    $temporary = "$Path.tmp"
    [IO.File]::WriteAllText($temporary, (($Value | ConvertTo-Json -Depth 80) + "`n"), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Get-DescendantProcessRows([int]$RootPid) {
    $rows = @(Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object ProcessId, ParentProcessId, WorkingSetSize)
    if ($rows.Count -eq 0) { throw "Win32_Process returned no rows; peak working-set metrics are unavailable." }
    $selected = [Collections.Generic.HashSet[uint32]]::new()
    $null = $selected.Add([uint32]$RootPid)
    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($row in $rows) {
            if ($selected.Contains([uint32]$row.ParentProcessId) -and $selected.Add([uint32]$row.ProcessId)) {
                $changed = $true
            }
        }
    }
    return @($rows | Where-Object { $selected.Contains([uint32]$_.ProcessId) })
}

function Invoke-MeasuredCargoWorkload {
    param(
        [Parameter(Mandatory = $true)][string]$WorkloadId,
        [Parameter(Mandatory = $true)][string]$Example,
        [Parameter(Mandatory = $true)][string]$ResultPath,
        [Parameter(Mandatory = $true)]$Budget,
        [string[]]$AdditionalArguments = @()
    )
    if (@(Get-Process -Name cargo -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "Another Cargo process is active before $WorkloadId; performance measurements require one Cargo process at a time."
    }
    $stdout = Join-Path $workRoot "$WorkloadId.stdout.log"
    $stderr = Join-Path $workRoot "$WorkloadId.stderr.log"
    $arguments = @(
        "run", "--release", "--locked", "-p", "qpls-runner", "--example", $Example,
        "--", "--output", $ResultPath, "--seed", ([string]$Seed)
    ) + $AdditionalArguments
    $stopwatch = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath "cargo" -ArgumentList $arguments -WorkingDirectory $repositoryRoot -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
    $peak = [long]0
    $samples = 0
    try {
        while (-not $process.HasExited) {
            $rows = @(Get-DescendantProcessRows $process.Id)
            if ($rows.Count -eq 0) {
                $process.Refresh()
                if ($process.HasExited) { break }
                throw "$WorkloadId process-tree metrics became unavailable."
            }
            $workingSet = [long]0
            foreach ($row in $rows) {
                if ($null -eq $row.WorkingSetSize) { throw "$WorkloadId has a process row without WorkingSetSize." }
                $workingSet += [long]$row.WorkingSetSize
            }
            if ($workingSet -gt $peak) { $peak = $workingSet }
            $samples += 1
            if ($stopwatch.Elapsed.TotalSeconds -gt [double]$Budget.maximum_seconds) {
                & taskkill.exe /PID $process.Id /T /F *> $null
                $null = $process.WaitForExit(30000)
                throw "$WorkloadId exceeded the predeclared $($Budget.maximum_seconds)-second budget."
            }
            Start-Sleep -Milliseconds 250
            $process.Refresh()
        }
        $process.WaitForExit()
    }
    catch {
        if (-not $process.HasExited) {
            & taskkill.exe /PID $process.Id /T /F *> $null
            $null = $process.WaitForExit(30000)
        }
        throw
    }
    finally { $stopwatch.Stop() }
    if ($process.ExitCode -ne 0) {
        $tail = if (Test-Path -LiteralPath $stderr) { (Get-Content -LiteralPath $stderr -Tail 40) -join "`n" } else { "stderr unavailable" }
        throw "$WorkloadId exited with code $($process.ExitCode).`n$tail"
    }
    if ($samples -lt 1 -or $peak -le 0) { throw "$WorkloadId produced no usable peak working-set samples." }
    if ($peak -gt [long]$Budget.maximum_peak_working_set_bytes) {
        throw "$WorkloadId peak working set $peak exceeded $($Budget.maximum_peak_working_set_bytes)."
    }
    if (-not (Test-Path -LiteralPath $ResultPath -PathType Leaf)) { throw "$WorkloadId did not create its declared production output." }
    $size = [long](Get-Item -LiteralPath $ResultPath).Length
    if ($size -le 0 -or $size -gt [long]$Budget.maximum_output_bytes) {
        throw "$WorkloadId output size $size is outside its predeclared budget."
    }
    return [ordered]@{
        workload_id = $WorkloadId
        example = $Example
        command = @("cargo") + $arguments
        exit_code = $process.ExitCode
        wall_time_milliseconds = [long]$stopwatch.Elapsed.TotalMilliseconds
        peak_working_set_bytes = $peak
        working_set_sample_count = $samples
        output_path = [IO.Path]::GetFullPath($ResultPath)
        output_size_bytes = $size
        output_sha256 = Get-LowerSha256 $ResultPath
        stdout_path = [IO.Path]::GetFullPath($stdout)
        stdout_size_bytes = [long](Get-Item -LiteralPath $stdout).Length
        stdout_sha256 = Get-LowerSha256 $stdout
        stderr_path = [IO.Path]::GetFullPath($stderr)
        stderr_size_bytes = [long](Get-Item -LiteralPath $stderr).Length
        stderr_sha256 = Get-LowerSha256 $stderr
        predeclared_budget = $Budget
        budget_passed = $true
    }
}

if (-not $IsWindows) { throw "Maximum-profile process-tree performance qualification is Windows-only." }
if (Test-Path -LiteralPath $outputPath) { throw "Performance qualification output already exists: $outputPath" }
if (Test-Path -LiteralPath $workRoot) { throw "Performance qualification work root must be new: $workRoot" }
if (-not (Test-Path -LiteralPath $verifier -PathType Leaf)) { throw "Performance output verifier is missing: $verifier" }
New-Item -ItemType Directory -Path $workRoot -Force | Out-Null

$mgaOutput = Join-Path $workRoot "mga-20-groups-production.json"
$heterogeneityOutput = Join-Path $workRoot "heterogeneity-locked-p23-production.json"
$maximumOutput = Join-Path $workRoot "conditional-sidecar-resume-production.json"
$verificationOutput = Join-Path $workRoot "performance-output-verification.json"
$measurements = [Collections.Generic.List[object]]::new()
$measurements.Add((Invoke-MeasuredCargoWorkload -WorkloadId "mga_20_groups_190_pairs" -Example "multimod_mga_qualification_v1" -ResultPath $mgaOutput -Budget $budgets.mga_20_groups_190_pairs -AdditionalArguments @("--scale", "qualification")))
$measurements.Add((Invoke-MeasuredCargoWorkload -WorkloadId "heterogeneity_locked_p23" -Example "multimod_heterogeneity_qualification_v2" -ResultPath $heterogeneityOutput -Budget $budgets.heterogeneity_locked_p23 -AdditionalArguments @("--scale", "qualification")))
$measurements.Add((Invoke-MeasuredCargoWorkload -WorkloadId "conditional_sidecar_resume" -Example "multimod_maximum_profiles_performance_v1" -ResultPath $maximumOutput -Budget $budgets.conditional_sidecar_resume))

$python = (Get-Command python -ErrorAction Stop).Source
& $python $verifier --mga $mgaOutput --heterogeneity $heterogeneityOutput --maximum $maximumOutput --output $verificationOutput
if ($LASTEXITCODE -ne 0) { throw "Maximum-profile output verification failed with exit code $LASTEXITCODE." }
$verification = Get-Content -LiteralPath $verificationOutput -Raw -Encoding UTF8 | ConvertFrom-Json -Depth 80
if ($verification.passed -ne $true -or $verification.report_id -cne "qpls.v256.multimod.performance-output-verification.v1") {
    throw "Maximum-profile structural verification did not pass."
}

$sourceFiles = @(
    "Cargo.lock",
    "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/examples/multimod_mga_qualification_v1.rs",
    "crates/qpls-runner/examples/multimod_heterogeneity_qualification_v2.rs",
    "crates/qpls-runner/examples/multimod_maximum_profiles_performance_v1.rs",
    "crates/qpls-runner/src/multimod_execution_v1.rs",
    "crates/qpls-runner/src/multimod_conditional_raw_v2.rs",
    "crates/qpls-estimation/src/heterogeneity_v2.rs",
    "crates/qpls-estimation/src/multigroup_v1.rs",
    "crates/qpls-project/src/multimod_archive_v1.rs",
    "validation/multimod/run_multimod_performance_profiles_v1.ps1",
    "validation/multimod/verify_multimod_performance_profiles_v1.py"
)
$sourceDigests = @(
    foreach ($relative in $sourceFiles) {
        $path = Join-Path $repositoryRoot $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Performance source binding is missing: $relative" }
        [ordered]@{ path = $relative.Replace("\", "/"); sha256 = Get-LowerSha256 $path; size = [long](Get-Item -LiteralPath $path).Length }
    }
)
$report = [ordered]@{
    schema_version = 1
    report_id = "qpls.v256.multimod.maximum-profile-performance.v1"
    passed = $true
    seed = $Seed
    measurement_contract = "windows_process_tree_working_set_and_stopwatch_v1"
    metrics_fail_closed_when_unavailable = $true
    predeclared_budgets = $budgets
    workloads = @($measurements)
    production_output_verification = $verification
    production_output_verification_path = [IO.Path]::GetFullPath($verificationOutput)
    production_output_verification_sha256 = Get-LowerSha256 $verificationOutput
    source_digests = $sourceDigests
    sidecar_size_metrics = [ordered]@{
        warning_bytes = [long]$verification.maximum.archive_sidecar_boundaries.warning_bytes
        maximum_bytes = [long]$verification.maximum.archive_sidecar_boundaries.maximum_bytes
        aggregate_maximum_admitted_bytes = [long]$verification.maximum.archive_sidecar_boundaries.aggregate_maximum_admitted_bytes
        maximum_plus_one_rejected = $verification.maximum.archive_sidecar_boundaries.aggregate_maximum_plus_one_rejected
    }
    output_total_bytes = [long](($measurements | Measure-Object -Property output_size_bytes -Sum).Sum)
    one_cargo_process_at_a_time = $true
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
}
Write-JsonAtomic $outputPath $report
$report | ConvertTo-Json -Depth 80
