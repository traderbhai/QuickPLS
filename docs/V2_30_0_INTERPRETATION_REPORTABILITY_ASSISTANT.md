# QuickPLS v2.30.0 Interpretation And Reportability Assistant

## Scope

`v2_30_0_interpretation_reportability_assistant` adds a frontend-only reportability assistant inside the Results workbook. It turns existing run values, assessment payloads, and diagram-derived recommendations into a checklist that tells the researcher what is ready, what needs review, what is unavailable, and what is not applicable.

## User-Facing Changes

- Added a reportability assistant on the Results overview and Interpretation tab.
- Grouped checklist items into `Must address`, `Review before reporting`, `Ready evidence`, and `Unavailable / not applicable`.
- Added value-specific sections for each checklist item:
  - what the value says;
  - why it matters;
  - what to inspect next;
  - report wording.
- Added copyable report snippets generated from the selected run values.
- Kept threshold colors as optional methodological guidance, not universal pass/fail rules.
- Preserved the existing research-table shell and row-detail interpretation behavior.

## Validation

Targeted evidence:

- `validation/results/v2300_reportability_assistant_smoke.json`
- `validation/results/v2300_reportability_assistant_audit.json`
- screenshots under `validation/results/screens/v2300/reportability-assistant/`

Commands:

```powershell
npm run build
npm run qpls:v2300:reportability-assistant-smoke
npm run qpls:v2300:reportability-assistant-audit
cargo run -p qpls-cli -- gate v2_30_0_interpretation_reportability_assistant
```

## Boundary

This milestone is frontend/product presentation only. It does not change statistical engines, formulas, estimator crates, result schemas, project archive format, validation tolerances, or numerical fingerprints.
