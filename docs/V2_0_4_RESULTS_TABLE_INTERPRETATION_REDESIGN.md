# QuickPLS v2.0.4 Results Table And Interpretation Redesign

Status: validated after `qpls:v204:results-redesign` passes.

## Summary

v2.0.4 rebuilds the Results workspace around the approved QuickPLS 2.0 desktop visual contract. It makes Results feel like a scoped research workbook: one selected run, one active result view, a tab-aware interpretation lens, compact run confidence, and table sections with explicit row/column context.

## Complete

- Added a `Result workbook` navigation header that keeps the selected run visible beside the result tabs.
- Reworked selected-run context into two clear controls:
  - selected completed run;
  - active result view and tab hint.
- Added a tab-aware Results lens panel with:
  - the current research question;
  - evidence summary;
  - issue/review counts from existing interpretation findings;
  - report action guidance.
- Upgraded table section headers to show visible rows, visible columns, and wide-table guidance.
- Applied the v2 visual foundation to Results lens, run context, selected run card status, and table shell styling.
- Added smoke and audit scripts:
  - `validation/v204_results_redesign_smoke.mjs`
  - `validation/v204_results_redesign_audit.py`
- Added registry gate `v2_0_4_results_table_interpretation_redesign`.
- Updated release metadata to `2.0.4` and artifact labeling to `v2_0_4_results_table_interpretation_redesign`.

## Non-Goals

- No statistical engines, formulas, estimator behavior, result values, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.
- No SmartPLS equivalence claim.
- No new method promotion.
- No rewrite of the SEM Designer.

## Verification

Required commands:

```powershell
npm run build
npm test -- --run
npm run qpls:v204:results-redesign-smoke
npm run qpls:v204:results-redesign-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_4_results_table_interpretation_redesign
npm run qpls:desktop:build-versioned
```

## Next

Continue the QuickPLS 2.0 rebuild with Report export flow, Model shell polish around the existing SEM Designer, Run execution surface, Trust Center, and Settings against the same v2 visual contract.
