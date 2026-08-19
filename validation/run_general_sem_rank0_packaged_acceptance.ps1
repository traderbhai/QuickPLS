param(
    [Parameter(Mandatory = $true)][string]$InstalledExecutable,
    [Parameter(Mandatory = $true)][string]$PortableExecutable,
    [string]$PythonExecutable = "C:\Python313\python.exe",
    [string]$NodeExecutable = "node",
    [string]$OutputPath = "",
    [ValidateSet("", "mediation_point", "multiple_mediation_bootstrap", "multiple_two_way_moderation_point", "multiple_two_way_moderation_bootstrap")]
    [string]$VariantId = "",
    [switch]$RequireStandard
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

function Resolve-NewResultPath {
    param([string]$Path, [string]$RequiredExtension)
    $resolved = [System.IO.Path]::GetFullPath($Path)
    $resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
    if (-not $resolved.StartsWith($resultsRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Acceptance output must remain below validation\results: $resolved"
    }
    if ([System.IO.Path]::GetExtension($resolved) -ne $RequiredExtension) {
        throw "Acceptance output must use ${RequiredExtension}: $resolved"
    }
    if (Test-Path -LiteralPath $resolved) { throw "Acceptance output must be new: $resolved" }
    return $resolved
}

function Resolve-PackageExecutable {
    param([string]$Path, [string]$Kind)
    $resolved = [System.IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) { throw "$Kind executable is missing: $resolved" }
    if (((Get-Item -LiteralPath $resolved -Force).Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "$Kind executable must not be a symlink or reparse-point alias: $resolved"
    }
    if ([System.IO.Path]::GetExtension($resolved) -ne ".exe") { throw "$Kind package must resolve to an .exe: $resolved" }
    return $resolved
}

function Get-ExactDescendantProcesses {
    param([int]$RootProcessId)
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId, ParentProcessId, CreationDate, Name, ExecutablePath)
    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($RootProcessId)
    $descendants = [System.Collections.Generic.HashSet[int]]::new()
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($descendants.Add($childId)) { $pending.Enqueue($childId) }
        }
    }
    return @($rows | Where-Object { $descendants.Contains([int]$_.ProcessId) } | Sort-Object ProcessId)
}

function Get-LiveExactProcessIds {
    param([object[]]$Descriptors)
    $live = foreach ($descriptor in $Descriptors) {
        $pidValue = [int]$descriptor.ProcessId
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $pidValue" -ErrorAction SilentlyContinue
        if ($current -and $current.CreationDate -eq $descriptor.CreationDate -and $current.Name -eq $descriptor.Name) { $pidValue }
    }
    return @($live)
}

function Wait-CdpReady {
    param([string]$Endpoint)
    for ($attempt = 0; $attempt -lt 80; $attempt++) {
        try {
            $null = Invoke-RestMethod -Uri "$Endpoint/json/version" -TimeoutSec 1
            return
        } catch { Start-Sleep -Milliseconds 250 }
    }
    throw "QuickPLS WebView2 CDP did not become ready at $Endpoint."
}

function Test-CdpClosed {
    param([string]$Endpoint)
    try {
        $null = Invoke-RestMethod -Uri "$Endpoint/json/version" -TimeoutSec 1
        return $false
    } catch { return $true }
}

function Invoke-PackagedSession {
    param(
        [string]$Executable,
        [string]$PackageKind,
        [string]$VariantId,
        [string]$Phase,
        [int]$ScalePercent,
        [string]$EvidenceDir,
        [string]$ProjectPath,
        [string]$IdentityFile,
        [object]$PackageIdentity
    )
    $endpoint = "http://127.0.0.1:9222"
    $factor = ([double]$ScalePercent / 100).ToString("0.##", [System.Globalization.CultureInfo]::InvariantCulture)
    $priorWebView = [Environment]::GetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "Process")
    $priorEndpoint = [Environment]::GetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", "Process")
    $application = $null
    $rootDescriptor = $null
    $descriptors = @()
    $nodeExitCode = $null
    $closeExitCode = $null
    $forced = $false
    $forcedPids = @()
    $sessionError = $null
    $sessionResult = $null
    $launchedExecutablePath = $null
    $launchedExecutableSize = $null
    $launchedExecutableSha256 = $null
    $launchedExecutableIdentityMatched = $false
    try {
        $arguments = "--remote-debugging-port=9222 --force-device-scale-factor=$factor --disable-background-networking --disable-component-update"
        [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $arguments, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $endpoint, "Process")
        $application = Start-Process `
            -FilePath $Executable `
            -WorkingDirectory (Split-Path -Parent $Executable) `
            -WindowStyle Normal `
            -PassThru
        $rootProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $($application.Id)" -ErrorAction Stop
        $rootDescriptor = [pscustomobject]@{
            ProcessId = [int]$rootProcess.ProcessId
            ParentProcessId = [int]$rootProcess.ParentProcessId
            CreationDate = $rootProcess.CreationDate
            Name = $rootProcess.Name
            ExecutablePath = $rootProcess.ExecutablePath
        }
        $launchedExecutablePath = [System.IO.Path]::GetFullPath([string]$rootDescriptor.ExecutablePath)
        $launchedExecutableSize = (Get-Item -LiteralPath $launchedExecutablePath -ErrorAction Stop).Length
        $launchedExecutableSha256 = (Get-FileHash -LiteralPath $launchedExecutablePath -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
        $launchedExecutableIdentityMatched = [bool](
            $launchedExecutablePath -ieq [string]$PackageIdentity.resolved_path `
                -and $launchedExecutableSize -eq [long]$PackageIdentity.size `
                -and $launchedExecutableSha256 -ceq [string]$PackageIdentity.sha256
        )
        if (-not $launchedExecutableIdentityMatched) {
            throw "Launched process executable identity differs from the frozen $PackageKind package identity."
        }
        Wait-CdpReady -Endpoint $endpoint
        $nodeArguments = @(
            (Join-Path $PSScriptRoot "general_sem_rank0_packaged_acceptance.mjs"),
            "--phase", $Phase,
            "--package-kind", $PackageKind,
            "--variant-id", $VariantId,
            "--endpoint", $endpoint,
            "--evidence-dir", $EvidenceDir,
            "--project-path", $ProjectPath,
            "--python", $PythonExecutable,
            "--process-id", [string]$application.Id
        )
        if ($Phase -eq "reopen") {
            $nodeArguments += @("--scale-percent", [string]$ScalePercent, "--identity-file", $IdentityFile)
        }
        & $NodeExecutable @nodeArguments
        $nodeExitCode = $LASTEXITCODE
        if ($nodeExitCode -ne 0) { throw "General SEM $PackageKind/$VariantId/$Phase-$ScalePercent driver failed with exit code $nodeExitCode." }
    } catch {
        $sessionError = $_.Exception.Message
    } finally {
        [Environment]::SetEnvironmentVariable("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", $priorWebView, "Process")
        [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $priorEndpoint, "Process")
        if ($application) {
            $descriptors = @($rootDescriptor) + @(Get-ExactDescendantProcesses -RootProcessId $application.Id)
            $descriptors = @($descriptors | Where-Object { $null -ne $_ })
            try {
                $priorEndpointForClose = [Environment]::GetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", "Process")
                [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $endpoint, "Process")
                & $NodeExecutable (Join-Path $PSScriptRoot "close_tauri_test_window.mjs") 2>$null | Out-Null
                $closeExitCode = $LASTEXITCODE
                [Environment]::SetEnvironmentVariable("QUICKPLS_CDP_ENDPOINT", $priorEndpointForClose, "Process")
            } catch { $closeExitCode = -1 }
            $null = $application.WaitForExit(10000)
            if (-not $application.HasExited) {
                $forced = $true
                $liveBeforeForce = @(Get-LiveExactProcessIds -Descriptors $descriptors)
                foreach ($pidValue in $liveBeforeForce | Sort-Object -Descending) {
                    Stop-Process -Id $pidValue -Force -ErrorAction SilentlyContinue
                    $forcedPids += $pidValue
                }
                $null = $application.WaitForExit(3000)
            }
        }
        Start-Sleep -Milliseconds 250
        $lingering = @(Get-LiveExactProcessIds -Descriptors $descriptors)
        $cdpClosed = Test-CdpClosed -Endpoint $endpoint
        $parentExited = [bool]($application -and $application.HasExited)
        $sessionResult = [ordered]@{
            session_id = if ($Phase -eq "execute") { "primary" } else { "scale_$ScalePercent" }
            phase = $Phase
            scale_percent = $ScalePercent
            executable = $Executable
            launched_pid = if ($application) { $application.Id } else { $null }
            launched_executable_path = $launchedExecutablePath
            launched_executable_size = $launchedExecutableSize
            launched_executable_sha256 = $launchedExecutableSha256
            descendants_at_shutdown = @($descriptors | Where-Object { [int]$_.ProcessId -ne $application.Id })
            node_exit_code = $nodeExitCode
            graceful_close_exit_code = $closeExitCode
            graceful_exit_confirmed = [bool]($parentExited -and -not $forced)
            forced_termination = $forced
            forced_pids = @($forcedPids)
            lingering_pids = @($lingering)
            cdp_endpoint_closed = $cdpClosed
            error = $sessionError
            passed = [bool]($launchedExecutableIdentityMatched -and $nodeExitCode -eq 0 -and $closeExitCode -eq 0 -and $parentExited -and -not $forced -and $lingering.Count -eq 0 -and $cdpClosed)
        }
    }
    if ($sessionError) { throw $sessionError }
    return $sessionResult
}

$installed = Resolve-PackageExecutable -Path $InstalledExecutable -Kind "installed"
$portable = Resolve-PackageExecutable -Path $PortableExecutable -Kind "portable"
if (-not (Test-Path -LiteralPath $PythonExecutable -PathType Leaf) -and $PythonExecutable -ne "python") {
    throw "Python executable is missing: $PythonExecutable"
}
$existingQuickPls = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $_.Name -ieq "quickpls-desktop.exe" `
        -or ($_.ExecutablePath -and ([System.IO.Path]::GetFullPath($_.ExecutablePath) -ieq $installed -or [System.IO.Path]::GetFullPath($_.ExecutablePath) -ieq $portable))
})
if ($existingQuickPls.Count -gt 0) {
    throw "Close every existing QuickPLS process before Rank 0 packaged acceptance."
}
$existingCdpListener = @(Get-NetTCPConnection -State Listen -LocalPort 9222 -ErrorAction SilentlyContinue)
if ($existingCdpListener.Count -gt 0) {
    throw "TCP port 9222 is already listening; packaged acceptance requires an unambiguous private CDP endpoint."
}
$installedVersion = (Get-Item -LiteralPath $installed).VersionInfo
$portableVersion = (Get-Item -LiteralPath $portable).VersionInfo
if ($installedVersion.ProductVersion -ne $portableVersion.ProductVersion -or $installedVersion.FileVersion -ne $portableVersion.FileVersion) {
    throw "Installed and portable executables do not have the same product/file version."
}
$installedHash = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToLowerInvariant()
$portableHash = (Get-FileHash -LiteralPath $portable -Algorithm SHA256).Hash.ToLowerInvariant()
if ($installedHash -cne $portableHash) {
    throw "Installed and portable executables must be byte-identical qualification builds."
}
$buildFingerprint = $installedHash
$osBuild = [System.Environment]::OSVersion.Version.Build
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
if ($osBuild -lt 22000 -or $architecture -ne "x64") {
    throw "Rank 0 packaged qualification requires Windows 11 x86-64."
}
$processorRows = @(Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop)
$computerSystem = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
$physicalCores = [int](($processorRows | Measure-Object -Property NumberOfCores -Sum).Sum)
$logicalCores = [int][System.Environment]::ProcessorCount
$memoryBytes = [long]$computerSystem.TotalPhysicalMemory
$cpuIdentity = [string](($processorRows | ForEach-Object { $_.Name.Trim() } | Sort-Object -Unique) -join "; ")
if ($physicalCores -lt 6 -or $logicalCores -lt $physicalCores -or $memoryBytes -lt 16GB -or [string]::IsNullOrWhiteSpace($cpuIdentity)) {
    throw "Host does not satisfy the standard_windows_6c16g qualification profile."
}
$hardwareFingerprint = [ordered]@{
    os = "windows_11"
    architecture = "x86_64"
    cpu = $cpuIdentity
    physical_cores = $physicalCores
    logical_cores = $logicalCores
    memory_bytes = $memoryBytes
}
$packageProvenance = [ordered]@{
    evidence_kind = "windows_pe_package_identity_v1"
    file_identity_source = "resolved_path_size_sha256"
    version_identity_source = "System.Diagnostics.FileVersionInfo"
}
$packageIdentities = @(
    [ordered]@{
        package_kind = "installed"
        resolved_path = $installed
        size = [long](Get-Item -LiteralPath $installed).Length
        sha256 = $installedHash
        product_version = [string]$installedVersion.ProductVersion
        file_version = [string]$installedVersion.FileVersion
        provenance = $packageProvenance
    },
    [ordered]@{
        package_kind = "portable"
        resolved_path = $portable
        size = [long](Get-Item -LiteralPath $portable).Length
        sha256 = $portableHash
        product_version = [string]$portableVersion.ProductVersion
        file_version = [string]$portableVersion.FileVersion
        provenance = $packageProvenance
    }
)
$fingerprintLines = [System.Collections.Generic.List[string]]::new()
foreach ($identity in $packageIdentities) {
    $fingerprintLines.Add("package_kind=$($identity.package_kind)")
    $fingerprintLines.Add("resolved_path=$($identity.resolved_path)")
    $fingerprintLines.Add("size=$($identity.size)")
    $fingerprintLines.Add("sha256=$($identity.sha256)")
    $fingerprintLines.Add("product_version=$($identity.product_version)")
    $fingerprintLines.Add("file_version=$($identity.file_version)")
    $fingerprintLines.Add("provenance.evidence_kind=$($identity.provenance.evidence_kind)")
    $fingerprintLines.Add("provenance.file_identity_source=$($identity.provenance.file_identity_source)")
    $fingerprintLines.Add("provenance.version_identity_source=$($identity.provenance.version_identity_source)")
}
$fingerprintMaterial = [string]::Join("`n", $fingerprintLines)
$fingerprintBytes = [System.Text.Encoding]::UTF8.GetBytes($fingerprintMaterial)
$fingerprintAlgorithm = [System.Security.Cryptography.SHA256]::Create()
try { $packageSetFingerprint = -join ($fingerprintAlgorithm.ComputeHash($fingerprintBytes) | ForEach-Object { $_.ToString("x2") }) }
finally { $fingerprintAlgorithm.Dispose() }

$stamp = Get-Date -Format "yyyyMMdd-HHmmssfff"
$sessionRoot = Join-Path $repositoryRoot "validation\results\general_sem_rank0_packaged\$stamp"
[System.IO.Directory]::CreateDirectory($sessionRoot) | Out-Null
$packageIdentityEvidence = [ordered]@{
    schema_version = 1
    evidence_kind = "general_sem_rank0_package_identities"
    packages = $packageIdentities
    package_set_fingerprint = $packageSetFingerprint
    hardware_fingerprint = $hardwareFingerprint
}
$packageIdentityPath = Join-Path $sessionRoot "raw-package-identities.json"
[System.IO.File]::WriteAllText($packageIdentityPath, (($packageIdentityEvidence | ConvertTo-Json -Depth 12) + [Environment]::NewLine), [System.Text.UTF8Encoding]::new($false))
if ([string]::IsNullOrWhiteSpace($OutputPath)) { $OutputPath = Join-Path $sessionRoot "general_sem_rank0_packaged_acceptance.json" }
$OutputPath = Resolve-NewResultPath -Path $OutputPath -RequiredExtension ".json"

$packages = [ordered]@{ installed = $installed; portable = $portable }
$variants = @(
    "mediation_point",
    "multiple_mediation_bootstrap",
    "multiple_two_way_moderation_point",
    "multiple_two_way_moderation_bootstrap"
)
if (-not [string]::IsNullOrWhiteSpace($VariantId)) { $variants = @($VariantId) }
foreach ($package in $packages.GetEnumerator()) {
    foreach ($variant in $variants) {
        $evidenceDir = Join-Path $sessionRoot "$($package.Key)\$variant"
        [System.IO.Directory]::CreateDirectory($evidenceDir) | Out-Null
        $projectPath = Join-Path $evidenceDir "rank0-general-sem.qpls"
        $identityFile = Join-Path $evidenceDir "raw-run-trace.json"
        $sessions = @()
        $sessions += Invoke-PackagedSession `
            -Executable $package.Value -PackageKind $package.Key -VariantId $variant `
            -Phase "execute" -ScalePercent 100 -EvidenceDir $evidenceDir `
            -ProjectPath $projectPath -IdentityFile "" -PackageIdentity ($packageIdentities | Where-Object { $_.package_kind -eq $package.Key })
        foreach ($scale in @(100, 125, 150, 200)) {
            $sessions += Invoke-PackagedSession `
                -Executable $package.Value -PackageKind $package.Key -VariantId $variant `
                -Phase "reopen" -ScalePercent $scale -EvidenceDir $evidenceDir `
                -ProjectPath $projectPath -IdentityFile $identityFile -PackageIdentity ($packageIdentities | Where-Object { $_.package_kind -eq $package.Key })
        }
        $cleanup = [ordered]@{
            schema_version = 1
            evidence_kind = "general_sem_rank0_process_cleanup"
            package_kind = $package.Key
            variant_id = $variant
            sessions = @($sessions)
            passed = [bool]($sessions.Count -eq 5 -and @($sessions | Where-Object { -not $_.passed }).Count -eq 0)
        }
        $cleanupPath = Join-Path $evidenceDir "raw-process-cleanup.json"
        [System.IO.File]::WriteAllText($cleanupPath, (($cleanup | ConvertTo-Json -Depth 12) + [Environment]::NewLine), [System.Text.UTF8Encoding]::new($false))
        if (-not $cleanup.passed) { throw "Exact process cleanup failed for $($package.Key)/$variant." }
    }
}

$composerArguments = @(
    (Join-Path $PSScriptRoot "general_sem_rank0_packaged_runner.py"),
    "--raw-root", $sessionRoot,
    "--output", $OutputPath,
    "--build-fingerprint", $buildFingerprint,
    "--package-identities", $packageIdentityPath,
    "--installed-executable", $installed,
    "--portable-executable", $portable
)
if (-not [string]::IsNullOrWhiteSpace($VariantId)) { $composerArguments += @("--variant-id", $VariantId) }
if ($RequireStandard) { $composerArguments += "--require-standard" }
& $PythonExecutable @composerArguments
if ($LASTEXITCODE -ne 0) { throw "Rank 0 packaged report composition/validation failed with exit code $LASTEXITCODE." }
Get-Item -LiteralPath $OutputPath | Select-Object FullName, Length, LastWriteTime
