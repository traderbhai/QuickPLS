# QuickPLS v2.29.0 Research Table System

Status: validated.

This frontend-only milestone upgrades the Results workspace table layer so statistical outputs use one desktop research-table shell instead of scattered ad hoc table behavior.

## Completed

- Added a v2.29 research-table marker and shared toolbar to Results tables.
- Added per-table search, sort, precision, compact/comfortable density, copy table, export table, copy selected rows, and row-detail interpretation.
- Added row selection with select-all for visible rows.
- Preserved sticky headers and added sticky first data-column behavior even when the row-selection column is present.
- Converted construct-score holdout, repeated-fold, and paired prediction-loss diagnostic tables to the shared table shell while retaining archive-compatible internal IDs.
- Preserved the earlier mediation, HTMT pair-list, bootstrap, inference, structural, measurement, validity, diagnostics, interpretation, and comparison table paths inside the same shell.

## Evidence

- `npm run build`
- `npm run qpls:v2290:research-tables-smoke`
- `npm run qpls:v2290:research-tables-audit`
- `cargo run -p qpls-cli -- gate v2_29_0_research_table_system`

Generated evidence:

- `validation/results/v2290_research_tables_smoke.json`
- `validation/results/v2290_research_tables_audit.json`
- `validation/results/screens/v2290/research-tables/`

## Boundary

This milestone changes only frontend presentation and interaction. It does not change statistical engines, formulas, method validation, result schemas, project archive format, validation tolerances, or numerical fingerprints.
