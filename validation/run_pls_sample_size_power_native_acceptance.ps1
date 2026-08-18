param(
    [switch]$ConfirmFocusedAdapterExists,
    [string]$CumulativeReceiptPath = ""
)

$ErrorActionPreference = "Stop"
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot

if (-not $ConfirmFocusedAdapterExists) {
    throw "Focused installed-Windows PLS sample-size/power acceptance is not implemented. No release evidence was written. A dedicated adapter must exercise invalid setup, execute, cancel/retry, accessible results, same-run CSV/XLSX, save/reopen, offline monitoring, three exact viewports, and process cleanup."
}

if ([string]::IsNullOrWhiteSpace($CumulativeReceiptPath)) {
    throw "Explicit confirmation cannot replace a current cumulative packaged-acceptance receipt. No release evidence was written."
}

$receipt = [System.IO.Path]::GetFullPath($CumulativeReceiptPath)
$resultsRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "validation\results"))
$receiptParent = [System.IO.Path]::GetFullPath((Split-Path -Parent $receipt))
if ($receiptParent -ne $resultsRoot -or [System.IO.Path]::GetExtension($receipt) -ne ".json") {
    throw "The cumulative receipt must be a JSON file directly under validation\results. No release evidence was written: $receipt"
}
if (-not (Test-Path -LiteralPath $receipt -PathType Leaf)) {
    throw "The requested cumulative receipt does not exist. No release evidence was written: $receipt"
}

$harnessPath = Join-Path $repositoryRoot "validation\v247_tauri_native_acceptance.mjs"
$harness = [System.IO.File]::ReadAllText($harnessPath)
$requiredMarkers = @(
    'QUICKPLS_ACCEPTANCE_SCOPE === "pls_sample_size_power"',
    'plsSampleSizePowerInvalidSetup',
    'plsSampleSizePowerCancellationRetry',
    'plsSampleSizePowerPackagedViewports',
    'plsSampleSizePowerArchiveReopen',
    'plsSampleSizePowerSameRunExports'
)
$missing = @($requiredMarkers | Where-Object { -not $harness.Contains($_) })
if ($missing.Count -ne 0) {
    throw "The focused packaged adapter is still missing required fail-closed markers: $($missing -join ', '). No desktop process was launched and no release evidence was written."
}

throw "The focused adapter markers exist, but this scaffold intentionally remains fail-closed until an independent packaged-report verifier is implemented and source-bound by the method manifest. No desktop process was launched and no release evidence was written."
