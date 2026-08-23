param(
    [string]$PythonExecutable = "python"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$receiptPath = Join-Path $PSScriptRoot "results\v247_cumulative_native_acceptance_receipt.json"
$acceptanceContractPath = Join-Path $PSScriptRoot "capabilities\packaged_windows_acceptance_v2.manifest.json"
$bundledSampleCatalogPath = Join-Path $root "src\data\bundledSampleProjects.v1.json"
if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) {
    throw "The fresh cumulative v247 receipt is missing: $receiptPath"
}
if (-not (Test-Path -LiteralPath $acceptanceContractPath -PathType Leaf)) {
    throw "The packaged Windows acceptance contract is missing: $acceptanceContractPath"
}
if (-not (Test-Path -LiteralPath $bundledSampleCatalogPath -PathType Leaf)) {
    throw "The bundled sample catalog is missing: $bundledSampleCatalogPath"
}

$receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
$acceptanceContract = Get-Content -LiteralPath $acceptanceContractPath -Raw | ConvertFrom-Json
$expectedCheckCount = @($acceptanceContract.ordered_check_sets | ForEach-Object { @($_.required_check_ids) }).Count
$acceptanceContractSha256 = (Get-FileHash -LiteralPath $acceptanceContractPath -Algorithm SHA256).Hash.ToLowerInvariant()
$bundledSampleCatalogSha256 = (Get-FileHash -LiteralPath $bundledSampleCatalogPath -Algorithm SHA256).Hash.ToLowerInvariant()
$bundledSampleCatalogSize = [int64](Get-Item -LiteralPath $bundledSampleCatalogPath).Length
if ($receipt.kind -ne "quickpls_v247_cumulative_native_acceptance_receipt" -or
    [int]$receipt.schema_version -ne 2 -or
    $receipt.passed -ne $true -or
    [int]$receipt.checks -ne $expectedCheckCount -or
    [int]$receipt.unique_checks -ne $expectedCheckCount -or
    $receipt.acceptance_contract.contract_id -ne $acceptanceContract.contract_id -or
    $receipt.acceptance_contract.contract_version -ne $acceptanceContract.contract_version -or
    [int]$receipt.acceptance_contract.required_check_count -ne $expectedCheckCount -or
    $receipt.acceptance_contract.sha256 -ne $acceptanceContractSha256 -or
    $receipt.acceptance_contract.bundled_sample_catalog.path -ne "src/data/bundledSampleProjects.v1.json" -or
    [int64]$receipt.acceptance_contract.bundled_sample_catalog.size -ne $bundledSampleCatalogSize -or
    $receipt.acceptance_contract.bundled_sample_catalog.sha256 -ne $bundledSampleCatalogSha256 -or
    [int]$receipt.failures -ne 0 -or
    [int]$receipt.console_errors -ne 0 -or
    $receipt.graceful_process_cleanup_verified -ne $true) {
    throw "The cumulative receipt does not bind the current packaged Windows acceptance manifest."
}

$notBeforeUtc = [string]$receipt.supervisor_started_at_utc
if ([string]::IsNullOrWhiteSpace($notBeforeUtc)) {
    throw "The cumulative receipt has no supervisor start timestamp."
}

$adapters = @(
    [pscustomobject]@{ Name = "gsca_als_v2"; Script = "validation/gsca_als_v2_packaged_acceptance.py" },
    [pscustomobject]@{ Name = "cca_residuals_v1"; Script = "validation/cca_residuals_v1_packaged_acceptance.py" },
    [pscustomobject]@{ Name = "ipma_v1"; Script = "validation/ipma_v1_packaged_acceptance.py" },
    [pscustomobject]@{ Name = "cbsem_ml_v1"; Script = "validation/cbsem_ml_v1_packaged_adapter.py" },
    [pscustomobject]@{ Name = "plspredict_cvpat_v2"; Script = "validation/plspredict_cvpat_v2_packaged_acceptance.py" },
    [pscustomobject]@{ Name = "nca_v2"; Script = "validation/nca_v2_packaged_acceptance.py" }
)

Push-Location $root
try {
    foreach ($adapter in $adapters) {
        & $PythonExecutable $adapter.Script --not-before-utc $notBeforeUtc
        if ($LASTEXITCODE -ne 0) {
            throw "Phase-2 packaged adapter failed with exit code ${LASTEXITCODE}: $($adapter.Script)"
        }
    }

} finally {
    Pop-Location
}

Write-Host "Phase-2 release evidence derived successfully from one fresh manifest-defined cumulative run."
Write-Host "The six reviewed manifests already declare release_qualified; this script only refreshes bound evidence."
