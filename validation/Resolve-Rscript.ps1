param(
  [string]$ConfiguredPath = $env:QPLS_RSCRIPT
)

$ErrorActionPreference = "Stop"

function Test-RscriptCandidate {
  param([string]$Candidate)
  if (-not $Candidate) {
    return $null
  }
  $command = Get-Command $Candidate -ErrorAction SilentlyContinue
  if (-not $command) {
    return $null
  }
  try {
    & $command.Source --version *> $null
    if ($LASTEXITCODE -eq 0) {
      return $command.Source
    }
  } catch {
    return $null
  }
  return $null
}

$configured = Test-RscriptCandidate $ConfiguredPath
if ($configured) {
  Write-Output $configured
  exit 0
}

$roots = @(
  (Join-Path ([Environment]::GetFolderPath("MyDocuments")) "PLS-Sem\dist-desktop\r-runtime"),
  (Join-Path $env:LOCALAPPDATA "Programs\R"),
  "C:\Program Files\R",
  "C:\Program Files (x86)\R"
)

foreach ($root in $roots) {
  if (-not (Test-Path $root)) {
    continue
  }
  foreach ($candidate in @(
    (Join-Path $root "bin\Rscript.exe"),
    (Join-Path $root "bin\x64\Rscript.exe")
  )) {
    $resolved = Test-RscriptCandidate $candidate
    if ($resolved) {
      Write-Output $resolved
      exit 0
    }
  }
  foreach ($versionDir in (Get-ChildItem -LiteralPath $root -Directory -Filter "R-*" | Sort-Object Name -Descending)) {
    foreach ($candidate in @(
      (Join-Path $versionDir.FullName "bin\Rscript.exe"),
      (Join-Path $versionDir.FullName "bin\x64\Rscript.exe")
    )) {
      $resolved = Test-RscriptCandidate $candidate
      if ($resolved) {
        Write-Output $resolved
        exit 0
      }
    }
  }
}

foreach ($registryKey in @(
  "HKLM:\SOFTWARE\R-core\R",
  "HKLM:\SOFTWARE\WOW6432Node\R-core\R",
  "HKCU:\SOFTWARE\R-core\R"
)) {
  if (-not (Test-Path $registryKey)) {
    continue
  }
  $installPath = (Get-ItemProperty -Path $registryKey -Name InstallPath -ErrorAction SilentlyContinue).InstallPath
  foreach ($candidate in @(
    (Join-Path $installPath "bin\Rscript.exe"),
    (Join-Path $installPath "bin\x64\Rscript.exe")
  )) {
    $resolved = Test-RscriptCandidate $candidate
    if ($resolved) {
      Write-Output $resolved
      exit 0
    }
  }
}

$pathCandidate = Test-RscriptCandidate "Rscript.exe"
if ($pathCandidate) {
  Write-Output $pathCandidate
  exit 0
}

throw "Rscript.exe was not found. Set QPLS_RSCRIPT to the full Rscript.exe path."
