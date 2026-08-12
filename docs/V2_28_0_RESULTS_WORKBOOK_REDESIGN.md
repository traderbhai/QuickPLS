# QuickPLS v2.28.0 Results Workbook Redesign

## Scope

`v2_28_0_results_workbook_redesign` is a frontend/product milestone in the native desktop redesign program. It changes the Results workspace presentation only.

No statistical engines, formulas, result payloads, project archive format, validation tolerances, or numerical fingerprints changed.

## Completed Work

- Added a native-style Results workbook body with a central table area and a right interpretation/detail pane.
- Kept the sticky selected-run context visible with method, observations, seed, fingerprint, warnings, report handoff, and method-confidence details.
- Added a right-side Method Confidence section for the selected run.
- Added findings lanes for `Must address`, `Review`, and `Info`, driven by the existing deterministic interpretation engine.
- Added tab-specific detail copy and quick actions to open the Interpretation checklist or prepare a Report.
- Added a provenance footer with run id, creation time, seed, fingerprint, and warning count.
- Preserved existing result tabs, interpretation logic, run comparison, table exports, and diagram focus behavior.

## Evidence

- `validation/v2280_results_workbook_smoke.mjs`
- `validation/v2280_results_workbook_audit.py`
- `validation/results/v2280_results_workbook_smoke.json`
- `validation/results/v2280_results_workbook_audit.json`
- `validation/results/screens/v2280/results-workbook/`

## Validation Commands

```powershell
npm run build
npm run qpls:v2280:results-workbook-smoke
npm run qpls:v2280:results-workbook-audit
cargo run -p qpls-cli -- gate v2_28_0_results_workbook_redesign
```

## Boundary

This milestone does not promote, demote, or alter any method. It only makes completed results read more like a professional statistical workbook.
