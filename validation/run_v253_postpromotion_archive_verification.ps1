[CmdletBinding()]
param(
    [string]$PromotionReport = "validation/results/general_sem_v253_standard_v1/promotion-report.json",
    [string]$EvidenceDir = "",
    [string]$CargoPath = "cargo",
    [string]$GitPath = "git",
    [double]$MinimumFreeGiB = 20
)

# Close only the two archive checks that are intentionally blocked until their
# exact Registry cells are activated. This is not a third consolidated pass.

$ErrorActionPreference = "Stop"
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "results"))
$resultsPrefix = $resultsRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$selectedIds = @("rust_project_three_way_archive", "rust_project_single_mediation_archive")
$excludedSourcePrefixes = @(".git/", ".vite/", "dist/", "node_modules/", "target/", "validation/results/")

function Write-Utf8Json {
    param([string]$LiteralPath, $Value, [int]$Depth = 40)
    [System.IO.File]::WriteAllText(
        $LiteralPath,
        ($Value | ConvertTo-Json -Depth $Depth) + [Environment]::NewLine,
        $utf8WithoutBom
    )
}

function Get-Sha256 {
    param([string]$LiteralPath)
    return (Get-FileHash -LiteralPath $LiteralPath -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-StringSha256 {
    param([string]$Value)
    $digest = [System.Security.Cryptography.SHA256]::HashData([System.Text.Encoding]::UTF8.GetBytes($Value))
    return [System.Convert]::ToHexString($digest).ToLowerInvariant()
}

function Get-RelativePath {
    param([string]$LiteralPath)
    $absolute = [System.IO.Path]::GetFullPath($LiteralPath)
    $rootPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $absolute.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside the repository: $absolute"
    }
    return [System.IO.Path]::GetRelativePath($repositoryRoot, $absolute).Replace("\", "/")
}

function Get-Descriptor {
    param([string]$LiteralPath)
    $item = Get-Item -LiteralPath $LiteralPath -ErrorAction Stop
    return [ordered]@{
        path = Get-RelativePath -LiteralPath $item.FullName
        size = [long]$item.Length
        sha256 = Get-Sha256 -LiteralPath $item.FullName
    }
}

function Get-SourceBinding {
    $listed = @(& $GitPath -C $repositoryRoot ls-files --cached --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) { throw "git ls-files failed" }
    $paths = @(
        $listed |
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
    foreach ($relative in $paths) {
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
    if ($LASTEXITCODE -ne 0) { throw "git rev-parse HEAD failed" }
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
    param([string]$Label)
    $drives = [System.Collections.Generic.List[object]]::new()
    foreach ($name in @("C", "D")) {
        $drive = Get-PSDrive -Name $name -PSProvider FileSystem -ErrorAction Stop
        $freeGiB = [math]::Round($drive.Free / 1GB, 3)
        $drives.Add([ordered]@{
            name = $name
            free_gib = $freeGiB
            threshold_gib_exclusive = $MinimumFreeGiB
            passed = $freeGiB -gt $MinimumFreeGiB
        })
    }
    return [ordered]@{
        label = $Label
        captured_at = (Get-Date).ToUniversalTime().ToString("o")
        passed = @($drives | Where-Object { -not $_.passed }).Count -eq 0
        drives = @($drives)
    }
}

$promotionPath = if ([System.IO.Path]::IsPathRooted($PromotionReport)) {
    [System.IO.Path]::GetFullPath($PromotionReport)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $PromotionReport))
}
if (-not (Test-Path -LiteralPath $promotionPath -PathType Leaf)) { throw "Promotion report is missing" }
$promotion = Get-Content -LiteralPath $promotionPath -Raw | ConvertFrom-Json
if ($promotion.report_kind -ne "general_sem_v253_streamlined_standard_promotion_v1" -or $promotion.promotion_completed -ne $true -or $promotion.release_complete -ne $false) {
    throw "Promotion report is not the pending Version 2.53 activation checkpoint"
}

if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
    $resolvedEvidenceDir = Join-Path $resultsRoot "v253_postpromotion_archive_$timestamp"
} elseif ([System.IO.Path]::IsPathRooted($EvidenceDir)) {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)
} else {
    $resolvedEvidenceDir = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $EvidenceDir))
}
if (-not $resolvedEvidenceDir.StartsWith($resultsPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "EvidenceDir must be a new child of $resultsRoot"
}
if (Test-Path -LiteralPath $resolvedEvidenceDir) { throw "Refusing to reuse EvidenceDir" }
$logsDir = Join-Path $resolvedEvidenceDir "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null
$reportPath = Join-Path $resolvedEvidenceDir "v253_postpromotion_archive_verification.json"
$receiptPath = [System.IO.Path]::ChangeExtension($reportPath, ".receipt.json")

$steps = @(
    [ordered]@{
        id = "rust_project_three_way_archive"
        description = "Post-promotion three-way canonical append, reopen, and tamper rejection"
        arguments = @("test", "--locked", "-p", "qpls-project", "three_way_canonical_append_reopen_and_tamper_fail_closed", "--", "--nocapture")
    },
    [ordered]@{
        id = "rust_project_single_mediation_archive"
        description = "Post-promotion single-mediation exact-cell reopen and identity"
        arguments = @("test", "--locked", "-p", "qpls-project", "single_mediation_exact_cell_reopens_with_stable_identity", "--", "--nocapture")
    }
)

$sourceBefore = Get-SourceBinding
$diskBefore = Get-DiskSnapshot -Label "before_v253_postpromotion_archive"
$results = [System.Collections.Generic.List[object]]::new()
$priorLocation = (Get-Location).Path
$priorTarget = [Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Process")
$priorIncremental = [Environment]::GetEnvironmentVariable("CARGO_INCREMENTAL", "Process")
$env:CARGO_TARGET_DIR = Join-Path $repositoryRoot "target"
$env:CARGO_INCREMENTAL = "0"
$started = (Get-Date).ToUniversalTime()
try {
    Set-Location -LiteralPath $repositoryRoot
    foreach ($step in $steps) {
        $stdout = Join-Path $logsDir "$($step.id).stdout.log"
        $stderr = Join-Path $logsDir "$($step.id).stderr.log"
        $stepStarted = (Get-Date).ToUniversalTime()
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        $exitCode = -1
        $launchError = $null
        if (-not $diskBefore.passed) {
            $launchError = "disk_safety_gate_failed"
        } else {
            try {
                $arguments = @($step.arguments)
                $priorError = $ErrorActionPreference
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
                    & $CargoPath @arguments 1> $stdout 2> $stderr
                    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
                } finally {
                    $ErrorActionPreference = $priorError
                    if (Test-Path Variable:PSNativeCommandUseErrorActionPreference) {
                        $PSNativeCommandUseErrorActionPreference = $priorNativePreference
                    }
                }
            } catch {
                $launchError = $_.Exception.Message
                [System.IO.File]::AppendAllText($stderr, "command launch failure: $launchError$([Environment]::NewLine)", $utf8WithoutBom)
            }
        }
        $watch.Stop()
        $results.Add([ordered]@{
            id = $step.id
            description = $step.description
            status = if ($exitCode -eq 0 -and $null -eq $launchError) { "passed" } else { "failed" }
            reason = $launchError
            executable = $CargoPath
            arguments = @($step.arguments)
            started_at = $stepStarted.ToString("o")
            ended_at = (Get-Date).ToUniversalTime().ToString("o")
            duration_ms = $watch.ElapsedMilliseconds
            exit_code = $exitCode
            stdout = [System.IO.Path]::GetRelativePath($resolvedEvidenceDir, $stdout).Replace("\", "/")
            stderr = [System.IO.Path]::GetRelativePath($resolvedEvidenceDir, $stderr).Replace("\", "/")
            stdout_sha256 = if (Test-Path -LiteralPath $stdout) { Get-Sha256 -LiteralPath $stdout } else { $null }
            stderr_sha256 = if (Test-Path -LiteralPath $stderr) { Get-Sha256 -LiteralPath $stderr } else { $null }
        })
    }
} finally {
    if ($null -eq $priorTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $priorTarget }
    if ($null -eq $priorIncremental) { Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue } else { $env:CARGO_INCREMENTAL = $priorIncremental }
    Set-Location -LiteralPath $priorLocation
}

$ended = (Get-Date).ToUniversalTime()
$diskAfter = Get-DiskSnapshot -Label "after_v253_postpromotion_archive"
$sourceAfter = Get-SourceBinding
$sourceStable = $sourceBefore.manifest_sha256 -eq $sourceAfter.manifest_sha256 -and $sourceBefore.git_head -eq $sourceAfter.git_head
$failed = @($results | Where-Object { $_.status -ne "passed" })
$passed = $failed.Count -eq 0 -and $diskBefore.passed -and $diskAfter.passed -and $sourceStable
$report = [ordered]@{
    schema_version = 1
    suite_id = "quickpls_v253_postpromotion_archive_verification_v1"
    version = "2.53.0"
    passed = $passed
    release_verification_complete = $passed
    repository_root = $repositoryRoot
    evidence_directory = $resolvedEvidenceDir
    promotion_report = Get-Descriptor -LiteralPath $promotionPath
    selected_step_ids = @($selectedIds)
    started_at = $started.ToString("o")
    ended_at = $ended.ToString("o")
    duration_ms = [math]::Round(($ended - $started).TotalMilliseconds)
    summary = [ordered]@{
        total = $results.Count
        passed = @($results | Where-Object { $_.status -eq "passed" }).Count
        deferred = 0
        failed = $failed.Count
        skipped = 0
    }
    source_binding = $sourceBefore
    source_binding_after = [ordered]@{ stable = $sourceStable; manifest_sha256 = $sourceAfter.manifest_sha256; git_head = $sourceAfter.git_head }
    disk_snapshots = @($diskBefore, $diskAfter)
    steps = @($results)
}
Write-Utf8Json -LiteralPath $reportPath -Value $report
Write-Utf8Json -LiteralPath $receiptPath -Value ([ordered]@{
    schema_version = 1
    receipt_kind = "quickpls_v253_postpromotion_archive_verification_sha256_v1"
    report = Get-Descriptor -LiteralPath $reportPath
})
$report | ConvertTo-Json -Depth 40
if (-not $passed) { exit 1 }
