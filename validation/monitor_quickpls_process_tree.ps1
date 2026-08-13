param(
    [Parameter(Mandatory = $true)]
    [int]$RootProcessId,
    [Parameter(Mandatory = $true)]
    [string]$SamplesPath,
    [Parameter(Mandatory = $true)]
    [string]$StopSignalPath,
    [int]$IntervalMilliseconds = 250
)

$ErrorActionPreference = "Stop"
function Test-FullyQualifiedWindowsPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or
        -not [System.IO.Path]::IsPathRooted($Path)) {
        return $false
    }
    $driveAbsolute = $Path -match '^[A-Za-z]:[\\/]'
    $uncAbsolute = $Path -match '^[\\/]{2}[^\\/]+[\\/]+[^\\/]+(?:[\\/]|$)'
    return $driveAbsolute -or $uncAbsolute
}

if ($RootProcessId -le 0) {
    throw "RootProcessId must be positive."
}
if (-not (Test-FullyQualifiedWindowsPath -Path $SamplesPath) -or
    -not (Test-FullyQualifiedWindowsPath -Path $StopSignalPath)) {
    throw "SamplesPath and StopSignalPath must be absolute."
}
$SamplesPath = [System.IO.Path]::GetFullPath($SamplesPath)
$StopSignalPath = [System.IO.Path]::GetFullPath($StopSignalPath)
if ($IntervalMilliseconds -lt 100 -or $IntervalMilliseconds -gt 5000) {
    throw "IntervalMilliseconds must be between 100 and 5000."
}

$rootDescriptor = $null
for ($attempt = 0; $attempt -lt 20; $attempt++) {
    $rootDescriptor = Get-CimInstance Win32_Process -Filter "ProcessId = $RootProcessId" -ErrorAction SilentlyContinue
    if ($rootDescriptor) { break }
    Start-Sleep -Milliseconds 100
}
if (-not $rootDescriptor) {
    throw "The exact root process $RootProcessId did not exist after the bounded monitor startup retry."
}
$rootCreationDate = [string]$rootDescriptor.CreationDate
$rootName = [string]$rootDescriptor.Name
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)

function Write-Sample {
    param([object]$Sample)
    $line = ($Sample | ConvertTo-Json -Depth 6 -Compress) + [Environment]::NewLine
    [System.IO.File]::AppendAllText($SamplesPath, $line, $utf8WithoutBom)
}

function Get-QuickPlsProcessRole {
    param([object]$Descriptor)
    if ([int]$Descriptor.ProcessId -eq $RootProcessId) { return "desktop_root" }
    $name = ([string]$Descriptor.Name).ToLowerInvariant()
    if ($name -ne "msedgewebview2.exe") { return "other_descendant" }
    $commandLine = [string]$Descriptor.CommandLine
    if ($commandLine -match '(?:^|\s)--type=renderer(?:\s|$)') { return "webview_renderer" }
    if ($commandLine -match '(?:^|\s)--type=gpu-process(?:\s|$)') { return "webview_gpu" }
    if ($commandLine -match '(?:^|\s)--type=utility(?:\s|$)') { return "webview_utility" }
    if ($commandLine -notmatch '(?:^|\s)--type=') { return "webview_browser" }
    return "webview_other"
}

while (-not (Test-Path -LiteralPath $StopSignalPath)) {
    $rows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Select-Object ProcessId, ParentProcessId, CreationDate, Name, ExecutablePath, CommandLine)
    $root = $rows | Where-Object {
        [int]$_.ProcessId -eq $RootProcessId -and
        [string]$_.CreationDate -eq $rootCreationDate -and
        [string]$_.Name -eq $rootName
    } | Select-Object -First 1
    if (-not $root) {
        throw "The exact root process $RootProcessId disappeared or changed identity before the monitor stop signal."
    }

    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($RootProcessId)
    $treeIds = [System.Collections.Generic.HashSet[int]]::new()
    $null = $treeIds.Add($RootProcessId)
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($child in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$child.ProcessId
            if ($treeIds.Add($childId)) {
                $pending.Enqueue($childId)
            }
        }
    }

    $processes = @()
    foreach ($descriptor in $rows | Where-Object { $treeIds.Contains([int]$_.ProcessId) } | Sort-Object ProcessId) {
        $live = Get-Process -Id ([int]$descriptor.ProcessId) -ErrorAction SilentlyContinue
        if ($live) {
            $processes += [ordered]@{
                pid = [int]$descriptor.ProcessId
                parent_pid = [int]$descriptor.ParentProcessId
                name = [string]$descriptor.Name
                role = Get-QuickPlsProcessRole -Descriptor $descriptor
                creation_date = [string]$descriptor.CreationDate
                working_set_bytes = [long]$live.WorkingSet64
                private_memory_bytes = [long]$live.PrivateMemorySize64
                handle_count = [int]$live.HandleCount
                thread_count = [int]$live.Threads.Count
            }
        }
    }
    [long]$total = 0
    [long]$totalPrivate = 0
    [int]$totalHandles = 0
    [int]$totalThreads = 0
    $roleCounts = [ordered]@{}
    foreach ($process in $processes) {
        $total += [long]$process["working_set_bytes"]
        $totalPrivate += [long]$process["private_memory_bytes"]
        $totalHandles += [int]$process["handle_count"]
        $totalThreads += [int]$process["thread_count"]
        $role = [string]$process["role"]
        if (-not $roleCounts.Contains($role)) { $roleCounts[$role] = 0 }
        $roleCounts[$role] = [int]$roleCounts[$role] + 1
    }
    Write-Sample ([ordered]@{
        recorded_at_utc = [DateTime]::UtcNow.ToString("o")
        root_present = $true
        root_pid = $RootProcessId
        total_working_set_bytes = $total
        total_private_memory_bytes = $totalPrivate
        total_handle_count = $totalHandles
        total_thread_count = $totalThreads
        process_role_counts = $roleCounts
        processes = $processes
    })
    Start-Sleep -Milliseconds $IntervalMilliseconds
}
