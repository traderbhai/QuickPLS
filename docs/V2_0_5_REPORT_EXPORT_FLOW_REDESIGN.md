# QuickPLS v2.0.5 Report Export Flow Redesign

Status: validated after `qpls:v205:report-redesign` passes.

## Summary

v2.0.5 applies the QuickPLS 2.0 visual contract to the Report workspace. The screen now behaves like a report package workbench: choose a preset, confirm the selected run, review figure/table readiness, preview the publication diagram, and export explicit outputs.

## Complete

- Added a v2 report package hero showing:
  - selected preset;
  - selected run state;
  - table count;
  - SVG figure readiness;
  - ready export outputs.
- Added a v2 command center for export presets and the four-step report flow.
- Kept report presets deterministic and presentation-only; they do not change result values.
- Preserved existing export behavior for:
  - SVG diagram;
  - CSV tables;
  - HTML report;
  - desktop XLSX;
  - browser Print/PDF path.
- Reframed settings, export review, comparison link, and preview shell with v2 panel styles.
- Added smoke and audit scripts:
  - `validation/v205_report_redesign_smoke.mjs`
  - `validation/v205_report_redesign_audit.py`
- Added registry gate `v2_0_5_report_export_flow_redesign`.
- Updated release metadata to `2.0.5` and artifact labeling to `v2_0_5_report_export_flow_redesign`.

## Non-Goals

- No statistical engines, formulas, estimator behavior, result values, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.
- No new native PDF or PNG export.
- No SmartPLS equivalence claim.
- No new method promotion.

## Verification

Required commands:

```powershell
npm run build
npm test -- --run
npm run qpls:v205:report-redesign-smoke
npm run qpls:v205:report-redesign-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_5_report_export_flow_redesign
npm run qpls:desktop:build-versioned
```

## Next

Continue the QuickPLS 2.0 rebuild with Model shell polish around the existing SEM Designer, then Run execution surface, Trust Center, and Settings.
