$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $Root
try {
  python validation/assessment_published_fixture_compare.py
}
finally {
  Pop-Location
}
