# QuickPLS v2.31.0 Report Export Wizard

v2.31.0 converts the Report workspace into a desktop-style export wizard. The milestone is frontend/product-only: it does not change statistical engines, formulas, result schemas, project archives, validation tolerances, or numerical fingerprints.

## User-Facing Changes

- Adds a four-step wizard: Select content, Preview, Document settings, Export.
- Keeps presets for Journal figure, Journal tables, Thesis appendix, Reviewer pack, Presentation, and Full reproducibility report.
- Shows the publication SVG preview and export table previews before users export.
- Groups document settings into figure settings, table settings, notes/interpretation, and provenance/reviewer-pack settings.
- Keeps detailed run comparison in Results and exposes a Report link to the Results Comparison workspace.
- Shows explicit enabled/disabled reasons for CSV, HTML, XLSX, Print/PDF, and SVG outputs.

## Validation

Targeted validation:

```powershell
npm run build
npm run qpls:v2310:report-export-wizard-smoke
npm run qpls:v2310:report-export-wizard-audit
cargo run -p qpls-cli -- gate v2_31_0_report_export_wizard
```

Smoke evidence is written to:

- `validation/results/v2310_report_export_wizard_smoke.json`
- `validation/results/screens/v2310/report-export-wizard/`

Audit evidence is written to:

- `validation/results/v2310_report_export_wizard_audit.json`
