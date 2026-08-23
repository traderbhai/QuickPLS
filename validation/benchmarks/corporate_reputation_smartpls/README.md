# SmartPLS Corporate Reputation parity benchmark

This benchmark reproduces the eight-construct, 31-indicator Corporate Reputation model shown in the official SmartPLS example and compares QuickPLS point estimates with the published three-decimal values.

The verified dataset is also the source for QuickPLS's built-in `corporate_reputation` sample:

- `validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.csv`
- `validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.provenance.json`

The existing small `validation/fixtures/corporate_reputation.csv` remains a separate legacy smoke-test fixture and is not used by the product sample.

## Data preparation

The source workbook contains 344 observations. Exact numeric `-99` is the only missing marker in the 31 model indicators: 11 cells across eight rows (`cusl_1`: 3, `cusl_2`: 4, `cusl_3`: 3, `cusa`: 1). This benchmark selects SmartPLS's indicator-wise mean-replacement treatment, retains all 344 observations, and records the replacement means and file hashes. The supplied screenshot does not expose its missing-data setting, so the successful 48-value reproduction is the empirical evidence for this selected treatment rather than a claim that the setting was directly observed in the image.

```powershell
python validation/benchmarks/corporate_reputation_smartpls/prepare_data.py `
  "C:\path\to\Corporate Reputation Data.xlsx" `
  .codex-artifacts/corporate-reputation-benchmark/corporate_reputation_mean_replaced.csv `
  .codex-artifacts/corporate-reputation-benchmark/preprocessing_receipt.json
```

## QuickPLS run

The benchmark uses ordinary PLS-SEM, path weighting, standardized indicators, Mode A for reflective constructs, Mode B for formative constructs, initial weights of `+1`, a `1e-7` stop criterion, and at most 3,000 iterations.

```powershell
cargo run -p qpls-cli --example corporate_reputation_benchmark -- `
  .codex-artifacts/corporate-reputation-benchmark/corporate_reputation_mean_replaced.csv `
  .codex-artifacts/corporate-reputation-benchmark/preprocessing_receipt.json `
  validation/benchmarks/corporate_reputation_smartpls/smartpls_reference_v1.json `
  .codex-artifacts/corporate-reputation-benchmark/corporate_reputation_benchmark.qpls `
  .codex-artifacts/corporate-reputation-benchmark/corporate_reputation_benchmark.json
```

The `.qpls` archive contains the cleaned dataset, full model, recipe, and completed result. The JSON report contains the full-precision QuickPLS estimates and a parameter-by-parameter comparison.

## Verified result

On the attached workbook, QuickPLS converges in eight iterations, uses all 344 observations, omits none, and matches all 48 published SmartPLS values after rounding to three decimals. A second run produced byte-identical estimation and comparison payloads. A negative-control reference with one deliberately changed coefficient exited nonzero at 47/48 matches, confirming that the benchmark gate detects disagreement.

The saved project validates successfully. Its only recipe warning is the expected single-item warning for CUSA; the pictured SmartPLS model intentionally measures CUSA with the single indicator `cusa`.

## Interpretation boundary

The screenshot and book tables expose SmartPLS values to three decimals. A passing benchmark therefore proves identical displayed values at that precision; it does not prove bitwise identity with unexported SmartPLS internals. Full-precision SmartPLS exports should be added as a separate reference if they become available.
