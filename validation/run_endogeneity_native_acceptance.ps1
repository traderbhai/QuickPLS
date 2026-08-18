param(
    [switch]$ConfirmFocusedAdapterExists
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

if (-not $ConfirmFocusedAdapterExists) {
    throw "Focused packaged Gaussian-copula endogeneity acceptance is not implemented. No release evidence was written. Add a dedicated scope to validation/v247_tauri_native_acceptance.mjs, independently verify the saved archive and XLSX, then rerun with explicit confirmation."
}

throw "Confirmation cannot override the missing packaged adapter. This scaffold remains fail-closed until the dedicated endogeneity workflow and independent archive/XLSX verifier are implemented."
