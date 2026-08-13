# QuickPLS 2.10.0 Release Notes

QuickPLS 2.10.0 is a focused Results/Report research table pass.

## What Changed

- Results tables now include clearer scan context, table captions, row/column counts, and per-table CSV export.
- Report preview tables use the same research-table treatment with copy/export controls and preview metadata.
- Rendered smoke evidence now covers Results and Report with completed sample runs at desktop viewports.
- The milestone remains frontend/product-only.

## What Did Not Change

- Statistical engines.
- Numerical formulas.
- Result schemas and serialized payload values.
- Project archive format.
- Method validation tolerances.
- Numerical fingerprints.

## Verification

```powershell
npm run qpls:v2100:results-report-tables
cargo run -p qpls-cli -- gate v2_10_0_results_report_research_table_pass
```
