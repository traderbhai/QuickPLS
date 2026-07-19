$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $Root
try {
  python validation/assessment_simulation.py
}
finally {
  Pop-Location
}
