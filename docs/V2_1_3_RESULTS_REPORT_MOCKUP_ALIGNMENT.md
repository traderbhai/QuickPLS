# QuickPLS v2.1.3 Results/Report Mockup Alignment

## Summary

`v2_1_3_results_report_mockup_alignment` applies the QuickPLS 2.1 desktop design-system primitives to the Results and Report workspaces. The milestone focuses on saved-run review, interpretation controls, report packaging, publication preview, and export surfaces.

This is a frontend/product-only milestone. It does not change statistical engines, formulas, method validation, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## User-Facing Scope

- Results now uses the shared v2 page shell, page header, panels, metric cards, status badges, method confidence, and reportability primitives.
- Report now uses the same v2 shell and panel language as Home/Data/Setup/Run, with a clearer package hero, preset chooser, publication setup, export review, WYSIWYG SVG preview, and export actions.
- Existing table search, copy, current-table export, interpretation, comparison, native XLSX, CSV, HTML, SVG, and print/PDF paths remain wired.
- Results and Report keep scoped validation wording and must not make SmartPLS-equivalence or SmartPLS project-import claims.
- `R²` text is rendered directly and must not regress to mojibake.

## Evidence

- `validation/v2113_results_report_mockup_smoke.mjs`
- `validation/v2113_results_report_mockup_audit.py`
- `validation/results/v2113_results_report_mockup_smoke.json`
- `validation/results/v2113_results_report_mockup_audit.json`
- Screenshots under `validation/results/screens/v2113/results-report/`

## Verification

```powershell
npm run build
npm run qpls:v2113:results-report-smoke
npm run qpls:v2113:results-report-audit
cargo run -p qpls-cli -- gate v2_1_3_results_report_mockup_alignment
```

For release artifacts:

```powershell
npm run qpls:desktop:build-versioned
```

Artifacts must be written under `D:\QuickPLS\target\release\artifacts` with version `2.1.3`, milestone label `v2_1_3_results_report_mockup_alignment`, timestamp, architecture, and artifact type.

## Non-Goals

- No estimator or numerical result changes.
- No method promotion or scope expansion.
- No project archive migration.
- No SmartPLS equivalence or project-import claim.
