[CmdletBinding()]
param(
    [double]$MinimumSystemFreeGB = 15,
    [double]$MinimumWorkspaceFreeGB = 25,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Get-DirectorySizeBytes {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)

    if (-not (Test-Path -LiteralPath $LiteralPath -PathType Container)) {
        return [int64]0
    }
    $measurement = Get-ChildItem -LiteralPath $LiteralPath -File -Recurse -Force -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum
    if ($null -eq $measurement.Sum) {
        return [int64]0
    }
    return [int64]$measurement.Sum
}

function Get-DriveSnapshot {
    param([Parameter(Mandatory = $true)][string]$Name)

    $drive = Get-PSDrive -Name $Name -PSProvider FileSystem
    return [ordered]@{
        name = $drive.Name
        free_gb = [math]::Round($drive.Free / 1GB, 2)
        used_gb = [math]::Round($drive.Used / 1GB, 2)
    }
}

$workspaceRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$workspaceDriveName = (Get-Item -LiteralPath $workspaceRoot).PSDrive.Name
$systemDriveName = (Get-Item -LiteralPath $env:SystemRoot).PSDrive.Name
$expectedTarget = [System.IO.Path]::GetFullPath((Join-Path $workspaceRoot "target"))
$configuredTarget = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    $expectedTarget
} else {
    [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
}

$systemDrive = Get-DriveSnapshot -Name $systemDriveName
$workspaceDrive = Get-DriveSnapshot -Name $workspaceDriveName
$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()

if ($systemDrive.free_gb -lt $MinimumSystemFreeGB) {
    $errors.Add("system_drive_free_space_below_threshold")
}
if ($workspaceDrive.free_gb -lt $MinimumWorkspaceFreeGB) {
    $errors.Add("workspace_drive_free_space_below_threshold")
}
if (-not $configuredTarget.Equals($expectedTarget, [System.StringComparison]::OrdinalIgnoreCase)) {
    $errors.Add("cargo_target_dir_is_not_the_shared_workspace_target")
}
if ($env:CARGO_INCREMENTAL -and $env:CARGO_INCREMENTAL -ne "0") {
    $warnings.Add("cargo_incremental_is_not_disabled")
}

$buildProcesses = @(Get-Process cargo, rustc -ErrorAction SilentlyContinue | ForEach-Object {
    [ordered]@{
        name = $_.ProcessName
        id = $_.Id
        started_at = $_.StartTime.ToUniversalTime().ToString("o")
    }
})
$targetBytes = Get-DirectorySizeBytes -LiteralPath $expectedTarget

$report = [ordered]@{
    schema_version = 1
    passed = $errors.Count -eq 0
    thresholds = [ordered]@{
        system_free_gb = $MinimumSystemFreeGB
        workspace_free_gb = $MinimumWorkspaceFreeGB
    }
    system_drive = $systemDrive
    workspace_drive = $workspaceDrive
    workspace_root = $workspaceRoot
    expected_cargo_target_dir = $expectedTarget
    configured_cargo_target_dir = $configuredTarget
    target_size_gb = [math]::Round($targetBytes / 1GB, 2)
    active_build_processes = $buildProcesses
    errors = @($errors)
    warnings = @($warnings)
}

if ($Json) {
    $report | ConvertTo-Json -Depth 8
} else {
    Write-Host "QuickPLS shared-build disk guard: $(if ($report.passed) { 'PASS' } else { 'FAIL' })"
    Write-Host "System drive $($systemDrive.name): $($systemDrive.free_gb) GB free (minimum $MinimumSystemFreeGB GB)"
    Write-Host "Workspace drive $($workspaceDrive.name): $($workspaceDrive.free_gb) GB free (minimum $MinimumWorkspaceFreeGB GB)"
    Write-Host "Shared Cargo target: $expectedTarget ($($report.target_size_gb) GB)"
    if ($warnings.Count) { Write-Host "Warnings: $($warnings -join ', ')" }
    if ($errors.Count) { Write-Host "Errors: $($errors -join ', ')" }
}

if (-not $report.passed) {
    exit 1
}
