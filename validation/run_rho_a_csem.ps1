param(
  [string]$RscriptPath = $env:QPLS_RSCRIPT
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

$RscriptPath = & (Join-Path $root "validation\Resolve-Rscript.ps1") -ConfiguredPath $RscriptPath

$env:R_LIBS_USER = Join-Path $root "validation\r-library"
& $RscriptPath --vanilla `
  (Join-Path $root "validation\rho_a_csem_reference.R") `
  (Join-Path $root "validation\fixtures\rho_a_reference.csv") `
  (Join-Path $root "validation\results\rho_a_csem_0_6_1.csv")

cargo run -p qpls-cli -- run `
  validation\fixtures\rho_a_reference.recipe.json `
  --data validation\fixtures\rho_a_reference.csv `
  --output validation\results\rho_a_quickpls_reference.json `
  --allow-experimental

python validation\rho_a_csem_compare.py
