param(
    [Parameter(Mandatory = $true)][int]$RootProcessId,
    [Parameter(Mandatory = $true)][string]$SamplesPath,
    [Parameter(Mandatory = $true)][string]$StopSignalPath,
    [ValidateRange(100, 5000)][int]$IntervalMilliseconds = 250
)

$ErrorActionPreference = "Stop"
$samplesFullPath = [System.IO.Path]::GetFullPath($SamplesPath)
$stopFullPath = [System.IO.Path]::GetFullPath($StopSignalPath)
$parent = Split-Path -Parent $samplesFullPath
if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
    throw "The network-sample output parent does not exist: $parent"
}
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
$rootDescriptor = $null
for ($attempt = 0; $attempt -lt 20; $attempt++) {
    $rootDescriptor = Get-CimInstance Win32_Process -Filter "ProcessId = $RootProcessId" -ErrorAction SilentlyContinue
    if ($rootDescriptor) { break }
    Start-Sleep -Milliseconds 100
}
if (-not $rootDescriptor) { throw "The exact root process $RootProcessId did not exist during monitor startup." }
$rootCreationDate = [string]$rootDescriptor.CreationDate
$rootName = [string]$rootDescriptor.Name

function Get-ExactProcessTree {
    param([int]$RootId)
    $rows = @(Get-CimInstance Win32_Process -ErrorAction Stop |
        Select-Object ProcessId, ParentProcessId, CreationDate, Name, ExecutablePath)
    $root = @($rows | Where-Object {
        [int]$_.ProcessId -eq $RootId -and
        [string]$_.CreationDate -eq $rootCreationDate -and
        [string]$_.Name -eq $rootName
    })
    if ($root.Count -ne 1) { return @() }
    $pending = New-Object 'System.Collections.Generic.Queue[int]'
    $seen = New-Object 'System.Collections.Generic.HashSet[int]'
    $pending.Enqueue($RootId)
    $null = $seen.Add($RootId)
    while ($pending.Count -gt 0) {
        $parentId = $pending.Dequeue()
        foreach ($row in $rows | Where-Object { [int]$_.ParentProcessId -eq $parentId }) {
            $childId = [int]$row.ProcessId
            if ($seen.Add($childId)) { $pending.Enqueue($childId) }
        }
    }
    return @($rows | Where-Object { $seen.Contains([int]$_.ProcessId) } | Sort-Object ProcessId)
}

function Test-LoopbackOrUnspecifiedAddress {
    param([string]$Address)
    if ([string]::IsNullOrWhiteSpace($Address)) { return $true }
    $normalized = $Address.Trim().TrimStart('[').TrimEnd(']').ToLowerInvariant()
    return $normalized -eq "0.0.0.0" -or $normalized -eq "::" -or
        $normalized -eq "::1" -or $normalized -eq "localhost" -or
        $normalized.StartsWith("127.") -or $normalized.StartsWith("::ffff:127.")
}

while (-not (Test-Path -LiteralPath $stopFullPath -PathType Leaf)) {
    $tree = @(Get-ExactProcessTree -RootId $RootProcessId)
    if ($tree.Count -eq 0) {
        throw "The exact root process $RootProcessId disappeared or changed identity before the network monitor stop signal."
    }
    $ids = @($tree | ForEach-Object { [int]$_.ProcessId })
    $idSet = New-Object 'System.Collections.Generic.HashSet[int]'
    foreach ($id in $ids) { $null = $idSet.Add($id) }
    $connections = @()
    if ($ids.Count -gt 0) {
        $connections = @(Get-NetTCPConnection -ErrorAction Stop | Where-Object {
            $idSet.Contains([int]$_.OwningProcess)
        } | ForEach-Object {
            $remoteAddress = [string]$_.RemoteAddress
            $state = [string]$_.State
            $remoteAccess = $state -notin @("Listen", "Bound", "Closed") -and
                -not (Test-LoopbackOrUnspecifiedAddress -Address $remoteAddress)
            [ordered]@{
                owning_process = [int]$_.OwningProcess
                local_address = [string]$_.LocalAddress
                local_port = [int]$_.LocalPort
                remote_address = $remoteAddress
                remote_port = [int]$_.RemotePort
                state = $state
                remote_access = [bool]$remoteAccess
            }
        })
    }
    $sample = [ordered]@{
        recorded_at_utc = [DateTime]::UtcNow.ToString("o")
        root_pid = $RootProcessId
        root_present = @($tree | Where-Object { [int]$_.ProcessId -eq $RootProcessId }).Count -eq 1
        process_ids = @($ids)
        connections = @($connections)
        remote_connections = @($connections | Where-Object { $_.remote_access -eq $true })
        observation = "sampled_exact_process_tree_tcp_v1"
    }
    [System.IO.File]::AppendAllText(
        $samplesFullPath,
        (($sample | ConvertTo-Json -Compress -Depth 8) + [Environment]::NewLine),
        $utf8WithoutBom
    )
    Start-Sleep -Milliseconds $IntervalMilliseconds
}
