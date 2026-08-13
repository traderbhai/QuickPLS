# QuickPLS v2.10.0 Results/Report Research Table Pass

Status: targeted frontend/product-only milestone.

This pass improves Results and Report table scanning for saved-run review without changing statistical engines, method validation, result schemas, project archives, or numerical fingerprints.

## Scope

- Results tables expose a shared research-table marker, caption, row/column count, scan affordance, copy action, and per-table CSV export.
- Report preview tables use the same research-table presentation with preview metadata, copy, and per-table CSV export.
- Rendered smoke evidence covers Results and Report with completed sample runs at `1440x900` and `1280x800`.
- Static audit verifies version metadata, registry status, v2.10 UI markers, documentation, claim boundaries, and frontend-only scope.

## Evidence

- `validation/v2100_results_report_tables_smoke.mjs`
- `validation/v2100_results_report_tables_audit.py`
- `validation/results/v2100_results_report_tables_smoke.json`
- `validation/results/v2100_results_report_tables_audit.json`
- `validation/results/screens/v2100/results-report-tables/`

## Verification

```powershell
npm run qpls:v2100:results-report-tables
```

Final gate:

```powershell
cargo run -p qpls-cli -- gate v2_10_0_results_report_research_table_pass
```

## Boundary

No estimator, formula, method-validation, result-schema, project-archive, import-backend, validation-tolerance, or numerical-fingerprint changes are part of this milestone.
