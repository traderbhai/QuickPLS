[CmdletBinding()]
param(
    [string]$EvidenceDir = "",
    [string]$CargoPath = "cargo",
    [string]$NpmPath = "npm.cmd",
    [string]$NodePath = "node",
    [string]$PythonPath = "python",
    [string]$GitPath = "git",
    [double]$MinimumFreeGiB = 20.0,
    [double]$DefaultDiskStepHeadroomGiB = 0.5,
    [double]$EmergencyFreeGiB = 20.25,
    [int]$DiskWatchIntervalMilliseconds = 500
)

# QuickPLS 2.55 is intentionally code-first: one serial diagnostic pass records
# every failure, then one batch correction, then this identical script runs once
# more. It does not run a scientific requalification matrix or change versions.

$ErrorActionPreference = "Stop"
$canonicalToolInputs = [ordered]@{
    CargoPath = "cargo"
    NpmPath = "npm.cmd"
    NodePath = "node"
    PythonPath = "python"
    GitPath = "git"
}
foreach ($entry in $canonicalToolInputs.GetEnumerator()) {
    $observed = Get-Variable -Name $entry.Key -ValueOnly
    if ($observed -cne $entry.Value) {
        throw "The 2.55 release gate does not permit overriding $($entry.Key); expected '$($entry.Value)'."
    }
}
if ($MinimumFreeGiB -ne 20.0 -or $DefaultDiskStepHeadroomGiB -ne 0.5 -or $EmergencyFreeGiB -ne 20.25 -or $DiskWatchIntervalMilliseconds -ne 500) {
    throw "The 2.55 release gate requires its canonical disk floor, reserves, emergency floor, and polling interval."
}
$utf8 = [System.Text.UTF8Encoding]::new($false)
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$results = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$evidence = if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    Join-Path $results "v255_consolidated_diagnostics_$stamp"
} elseif ([IO.Path]::IsPathRooted($EvidenceDir)) {
    [IO.Path]::GetFullPath($EvidenceDir)
} else {
    [IO.Path]::GetFullPath((Join-Path $root $EvidenceDir))
}
$prefix = $results.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (-not $evidence.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw "EvidenceDir must be a new child of $results" }
if (Test-Path -LiteralPath $evidence) { throw "Refusing to reuse an existing evidence directory: $evidence" }
$logs = Join-Path $evidence "logs"
New-Item -ItemType Directory -Path $logs -Force | Out-Null
$sourceCommit = (& $GitPath -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') { throw "Unable to resolve the exact source commit for the 2.55 gate." }
$sourceStatus = @(& $GitPath -C $root status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) { throw "Unable to inspect the source worktree before the 2.55 gate." }
if ($sourceStatus.Count -ne 0) { throw "The 2.55 consolidated gate requires a clean committed source worktree." }
$gateScriptSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
$packageVersionAtGate = (Get-Content -LiteralPath (Join-Path $root "package.json") -Raw -Encoding UTF8 | ConvertFrom-Json).version
$resolvedTools = [ordered]@{}
foreach ($entry in $canonicalToolInputs.GetEnumerator()) {
    $command = Get-Command $entry.Value -CommandType Application -ErrorAction Stop
    $resolvedTools[$entry.Key] = [ordered]@{
        requested = $entry.Value
        resolved_path = $command.Source
        sha256 = if (Test-Path -LiteralPath $command.Source -PathType Leaf) {
            (Get-FileHash -LiteralPath $command.Source -Algorithm SHA256).Hash.ToLowerInvariant()
        } else { $null }
    }
}

function Write-Json([string]$Path, $Value) {
    [IO.File]::WriteAllText($Path, (($Value | ConvertTo-Json -Depth 32) + [Environment]::NewLine), $utf8)
}

function Get-DiskSnapshot([string]$Label, [double]$RequiredHeadroomGiB = 0.0) {
    $drives = @()
    foreach ($name in @("C", "D")) {
        try {
            $drive = Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop
            $free = [math]::Round($drive.Free / 1GB, 3)
            $requiredFree = $MinimumFreeGiB + $RequiredHeadroomGiB
            $drives += [ordered]@{ name = $name; free_gib = $free; floor_gib_exclusive = $MinimumFreeGiB; required_free_gib_exclusive = $requiredFree; reserved_headroom_gib = $RequiredHeadroomGiB; passed = $free -gt $requiredFree }
        } catch {
            $drives += [ordered]@{ name = $name; free_gib = $null; floor_gib_exclusive = $MinimumFreeGiB; required_free_gib_exclusive = ($MinimumFreeGiB + $RequiredHeadroomGiB); reserved_headroom_gib = $RequiredHeadroomGiB; passed = $false; error = $_.Exception.Message }
        }
    }
    [ordered]@{ label = $Label; captured_at = (Get-Date).ToUniversalTime().ToString("o"); passed = @($drives | Where-Object { -not $_.passed }).Count -eq 0; drives = $drives }
}

function New-Step([string]$Id, [string]$Description, [string]$Executable, [object[]]$Arguments, [bool]$DiskIntensive = $false, [double]$HeadroomGiB = 0.0) {
    [ordered]@{ id = $Id; description = $Description; executable = $Executable; arguments = @($Arguments); disk_intensive = $DiskIntensive; reserved_headroom_gib = $HeadroomGiB }
}

function Get-FreeDriveGiB() {
    $free = [ordered]@{}
    foreach ($name in @("C", "D")) {
        try {
            $free[$name] = [math]::Round(((Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop).Free / 1GB), 3)
        } catch {
            $free[$name] = $null
        }
    }
    $free
}

function ConvertTo-ProcessArgumentLine([object[]]$Arguments) {
    # All current gate paths are generated below D:\QuickPLS. Quote each value so
    # future evidence roots with spaces remain one argument for the child process.
    (($Arguments | ForEach-Object {
        '"' + ([string]$_).Replace('"', '\"') + '"'
    }) -join " ")
}

function Stop-LaunchedProcessTree([int]$ProcessId) {
    # This PID is returned by Start-Process in this invocation. /T therefore
    # reaches only children of the gate-owned launcher, never the user's app.
    & taskkill.exe /PID $ProcessId /T /F 1>$null 2>$null
}

function Invoke-Step($Step) {
    $stdout = Join-Path $logs "$($Step.id).stdout.log"
    $stderr = Join-Path $logs "$($Step.id).stderr.log"
    $started = (Get-Date).ToUniversalTime()
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $exit = -1
    $errorText = $null
    $watcher = $null
    try {
        if (-not $Step.disk_intensive) {
            & $Step.executable @($Step.arguments) 1> $stdout 2> $stderr
            $exit = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        } else {
            $process = Start-Process -FilePath $Step.executable -ArgumentList (ConvertTo-ProcessArgumentLine $Step.arguments) -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            $watcher = [ordered]@{
                launched_pid = $process.Id
                process_tree_termination_is_pid_scoped = $true
                emergency_free_gib_exclusive = $EmergencyFreeGiB
                poll_interval_milliseconds = $DiskWatchIntervalMilliseconds
                stopped_for_low_disk = $false
                samples = 0
                minimum_free_gib = [ordered]@{ C = $null; D = $null }
            }
            while (-not $process.HasExited) {
                $free = Get-FreeDriveGiB
                $watcher.samples += 1
                foreach ($name in @("C", "D")) {
                    if ($null -ne $free[$name] -and ($null -eq $watcher.minimum_free_gib[$name] -or $free[$name] -lt $watcher.minimum_free_gib[$name])) { $watcher.minimum_free_gib[$name] = $free[$name] }
                }
                $breaches = @($free.GetEnumerator() | Where-Object { $null -eq $_.Value -or $_.Value -le $EmergencyFreeGiB })
                if ($breaches.Count -gt 0) {
                    $watcher.stopped_for_low_disk = $true
                    $watcher.emergency_breaches = @($breaches | ForEach-Object { $_.Key })
                    Stop-LaunchedProcessTree $process.Id
                    $errorText = "emergency_disk_floor_reached: $($watcher.emergency_breaches -join ',') must remain above $EmergencyFreeGiB GiB"
                    break
                }
                Start-Sleep -Milliseconds $DiskWatchIntervalMilliseconds
                $process.Refresh()
            }
            $process.WaitForExit()
            $exit = if ($watcher.stopped_for_low_disk) { -1 } else { $process.ExitCode }
        }
    } catch {
        $errorText = $_.Exception.Message
        [IO.File]::AppendAllText($stderr, "launch failure: $errorText$([Environment]::NewLine)", $utf8)
    } finally { $timer.Stop() }
    [ordered]@{
        id = $Step.id; description = $Step.description
        executable = $Step.executable; arguments = @($Step.arguments)
        status = if ($exit -eq 0 -and -not $errorText) { "passed" } else { "failed" }
        exit_code = $exit; error = $errorText; duration_ms = $timer.ElapsedMilliseconds
        stdout = "logs/$([IO.Path]::GetFileName($stdout))"; stderr = "logs/$([IO.Path]::GetFileName($stderr))"
        stdout_sha256 = if (Test-Path -LiteralPath $stdout -PathType Leaf) { (Get-FileHash -LiteralPath $stdout -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        stderr_sha256 = if (Test-Path -LiteralPath $stderr -PathType Leaf) { (Get-FileHash -LiteralPath $stderr -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        started_at = $started.ToString("o"); ended_at = (Get-Date).ToUniversalTime().ToString("o")
        disk_watcher = $watcher
    }
}

$vitestReport = Join-Path $evidence "v255_full_vitest.json"
$rebasedReport = Join-Path $evidence "interaction_contracts\v255_rebased_interaction_contracts.json"
$sourceAuditReport = Join-Path $evidence "v255_source_contract_audit.json"
$finalAuditReport = Join-Path $evidence "v255_final_contract_audit.json"
$steps = @(
    (New-Step "diff_check" "Git whitespace-error check" $GitPath @("diff", "--check")),
    (New-Step "v255_evidence_contract" "Validate the 18-method evidence matrix and interaction-first rebaseline" $PythonPath @("validation/v255_product_completion_audit.py", "--output", $sourceAuditReport)),
    (New-Step "v255_rebased_contract" "Validate the 17 replacement interaction contracts" $NodePath @("validation/v255_rebased_interaction_contracts.mjs", "--mode", "contract")),
    (New-Step "frontend_full_vitest" "One full current frontend and domain Vitest traversal with machine-readable assertions" $NpmPath @("run", "test", "--", "--reporter=json", "--outputFile", $vitestReport) $true 2.5),
    (New-Step "rust_authority" "Focused exact Registry authority test" $CargoPath @("test", "--locked", "-p", "qpls-core", "embedded_registry_is_the_exact_option_cell_authority", "--", "--nocapture") $true 2.5),
    (New-Step "rust_archive_schema6_authoring" "Focused schema-6 author/save/reopen authority test" $CargoPath @("test", "--locked", "-p", "qpls-project", "--test", "schema6_sem_model_v4_authoring_shapes", "section_3_1_shapes_author_serialize_and_reopen_through_standalone_schema6", "--", "--exact", "--nocapture") $true 2.5),
    (New-Step "rust_archive_three_way" "Focused three-way canonical append/reopen/tamper lifecycle" $CargoPath @("test", "--locked", "-p", "qpls-project", "project_schema_v6::tests::three_way_canonical_append_reopen_and_tamper_fail_closed", "--lib", "--", "--exact", "--nocapture") $true 2.5),
    (New-Step "rust_desktop_three_way" "Focused desktop three-way execute/build/append/reopen lifecycle" $CargoPath @("test", "--locked", "-p", "quickpls-desktop", "recipe_v4_general_sem_canonical_result::tests::strict_v3_colon_ids_execute_build_append_and_reopen_three_way_canonical_result", "--lib", "--", "--exact", "--nocapture") $true 2.5),
    (New-Step "frontend_typecheck" "Full frontend typecheck" $NpmPath @("run", "typecheck:full") $true 1.0),
    (New-Step "frontend_build" "Production frontend bundle" $NpmPath @("run", "build:bundle") $true 1.5),
    (New-Step "python_export_semantic_readback" "Focused canonical CSV/XLSX/HTML/PDF/SVG/PNG semantic export readback" $PythonPath @("validation/test_general_sem_rank0_export_semantic_readback.py") $true 0.5),
    (New-Step "rebaselined_interactions" "1024x700 interaction evidence reconciled with exact Vitest assertions" $NodePath @("validation/v255_rebased_interaction_contracts.mjs", "--mode", "browser", "--evidence-dir", (Join-Path $evidence "interaction_contracts"), "--vitest-report", $vitestReport, "--port", "57655") $true 0.5),
    (New-Step "method_setup_crawler" "Serial complete Calculate setup and pre-candidate reusable-archive inventory crawl" $NodePath @("validation/v255_method_evidence_crawler.mjs", "--mode", "preview", "--result-evidence-phase", "source", "--evidence-dir", (Join-Path $evidence "method_evidence"), "--vitest-report", $vitestReport, "--port", "57656") $true 0.5),
    (New-Step "v255_final_evidence_contract" "Require all 17 rebaselined contracts and hash their reports" $PythonPath @("validation/v255_product_completion_audit.py", "--output", $finalAuditReport, "--final-stage", "--vitest-report", $vitestReport, "--rebaseline-report", $rebasedReport))
)

$snapshots = @()
$records = @()
$priorLocation = (Get-Location).Path
$priorCargoTarget = [Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Process")
$priorCargoIncremental = [Environment]::GetEnvironmentVariable("CARGO_INCREMENTAL", "Process")
try {
    Set-Location -LiteralPath $root
    $env:CARGO_TARGET_DIR = Join-Path $root "target"
    $env:CARGO_INCREMENTAL = "0"
    $snapshots += Get-DiskSnapshot "before_consolidated_pass"
    foreach ($step in $steps) {
        $gate = $null
        if ($step.disk_intensive) {
            $headroom = [double]$step.reserved_headroom_gib
            $gate = Get-DiskSnapshot "before_$($step.id)" $headroom
            $snapshots += $gate
            if (-not $gate.passed) {
                $records += [ordered]@{ id = $step.id; description = $step.description; status = "skipped"; reason = "disk_safety_gate_failed"; disk_gate = $gate }
                continue
            }
        }
        Write-Host "[v2.55 diagnostics] $($step.id): $($step.description)"
        $record = Invoke-Step $step
        if ($gate) {
            $after = Get-DiskSnapshot "after_$($step.id)" 0.0
            $snapshots += $after
            $record.disk_gate = [ordered]@{ before = $gate; after = $after }
            if (-not $after.passed) {
                $record.status = "failed"
                $record.error = "post_step_disk_floor_breached"
            }
        }
        $records += $record
    }
} finally {
    if ($null -eq $priorCargoTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $priorCargoTarget }
    if ($null -eq $priorCargoIncremental) { Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue } else { $env:CARGO_INCREMENTAL = $priorCargoIncremental }
    Set-Location -LiteralPath $priorLocation
}
$snapshots += Get-DiskSnapshot "after_consolidated_pass"
$failed = @($records | Where-Object { $_.status -eq "failed" } | ForEach-Object { $_.id })
$skipped = @($records | Where-Object { $_.status -eq "skipped" } | ForEach-Object { $_.id })
$report = [ordered]@{
    schema_version = 1; suite_id = "quickpls_v255_calculate_evidence_consolidated_diagnostics_v1"; target_release = "2.55.0"
    version_authority = "2.54.0 until this gate, remediation, packaged smoke, and release evidence all pass"
    source = [ordered]@{ commit = $sourceCommit; worktree_clean = $true; package_version = $packageVersionAtGate; gate_script = "validation/run_v255_consolidated_diagnostics.ps1"; gate_script_sha256 = $gateScriptSha256 }
    passed = $failed.Count -eq 0 -and $skipped.Count -eq 0 -and @($snapshots | Where-Object { -not $_.passed }).Count -eq 0
    policy = [ordered]@{ serial = $true; maximum_concurrent_calculations = 1; code_signing = $false; repeated_scientific_qualification_matrices = $false; batch_fix_then_identical_rerun = $true; canonical_parameters_locked = $true; minimum_free_gib_exclusive = $MinimumFreeGiB; emergency_free_gib_exclusive = $EmergencyFreeGiB; disk_watch_interval_milliseconds = $DiskWatchIntervalMilliseconds; default_disk_step_headroom_gib = $DefaultDiskStepHeadroomGiB; cargo_incremental = 0; cargo_target_dir = (Join-Path $root "target"); target_drive_reused = "D: workspace target"; tools = $resolvedTools; step_reserves_gib = [ordered]@{ frontend_full_vitest = 2.5; rust_authority = 2.5; rust_archive_schema6_authoring = 2.5; rust_archive_three_way = 2.5; rust_desktop_three_way = 2.5; frontend_typecheck = 1.0; frontend_build = 1.5; python_export_semantic_readback = 0.5; rebaselined_interactions = 0.5; method_setup_crawler = 0.5 } }
    summary = [ordered]@{ total = $records.Count; failed = $failed; skipped = $skipped }
    artifacts = [ordered]@{
        vitest_report = if (Test-Path -LiteralPath $vitestReport -PathType Leaf) { $vitestReport } else { $null }
        vitest_report_sha256 = if (Test-Path -LiteralPath $vitestReport -PathType Leaf) { (Get-FileHash -LiteralPath $vitestReport -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        rebaseline_report = if (Test-Path -LiteralPath $rebasedReport -PathType Leaf) { $rebasedReport } else { $null }
        rebaseline_report_sha256 = if (Test-Path -LiteralPath $rebasedReport -PathType Leaf) { (Get-FileHash -LiteralPath $rebasedReport -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        source_contract_audit = if (Test-Path -LiteralPath $sourceAuditReport -PathType Leaf) { $sourceAuditReport } else { $null }
        source_contract_audit_sha256 = if (Test-Path -LiteralPath $sourceAuditReport -PathType Leaf) { (Get-FileHash -LiteralPath $sourceAuditReport -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
        final_contract_audit = if (Test-Path -LiteralPath $finalAuditReport -PathType Leaf) { $finalAuditReport } else { $null }
        final_contract_audit_sha256 = if (Test-Path -LiteralPath $finalAuditReport -PathType Leaf) { (Get-FileHash -LiteralPath $finalAuditReport -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
    }
    disk_snapshots = $snapshots; steps = $records
    next_gate = "Run validation/run_v255_installed_portable_smoke.ps1 only after this source diagnostic and its one batch remediation pass succeed."
}
$reportPath = Join-Path $evidence "v255_consolidated_diagnostics.json"
Write-Json $reportPath $report
$report | ConvertTo-Json -Depth 32
if (-not $report.passed) { exit 1 }
