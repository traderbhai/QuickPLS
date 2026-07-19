param(
  [string]$RscriptPath = $env:QPLS_RSCRIPT
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

$RscriptPath = & (Join-Path $root "validation\Resolve-Rscript.ps1") -ConfiguredPath $RscriptPath

$env:R_LIBS_USER = Join-Path $root "validation\r-library"
& $RscriptPath --vanilla `
  (Join-Path $root "validation\htmt_csem_reference.R") `
  (Join-Path $root "validation\fixtures\corporate_reputation.csv") `
  (Join-Path $root "validation\results\htmt_csem_0_6_1.csv")

python validation\htmt_csem_compare.py
