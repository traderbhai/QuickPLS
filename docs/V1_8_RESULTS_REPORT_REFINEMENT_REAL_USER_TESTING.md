# QuickPLS v1.8 Results And Report Refinement From Real User Testing

Status: validated.

This frontend-only release refines the Results and Report workspaces using bundled/generated real-like datasets so development is not blocked by private user data.

## Completed

- Added a v1.8 real-dataset Results/Report audit covering the corporate reputation demo, larger PLS-SEM, mediation, HTMT-warning, and bootstrap-enabled fixture profiles.
- Replaced crowded Results controls with grouped menus: View, Table, Export, and Interpretation.
- Added a compact sticky run context with selected run, method, observations, seed, warnings, scope status, and a `Why trust this result?` details drawer.
- Reduced repeated scope/warning text in result sections by keeping confidence details at the run level.
- Split bootstrap output into estimates, percentile CI, BCa CI, and bootstrap-t CI tables.
- Changed HTMT display to show one row per construct pair by default, with the full matrix behind an explicit disclosure.
- Deduplicated interpretation findings at the source using canonical metric/object keys, including symmetric HTMT pairs.
- Changed finding cards to use value-specific sections: what the value says, why it matters, what to inspect next, and report wording.
- Reworked Report into a clearer four-step flow: select run, choose preset, review figure/table preview, export.
- Added export review and export status feedback for CSV, HTML, XLSX, SVG, and browser print/PDF paths.

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.

## Evidence

- `validation/results/v18_real_dataset_results_report_audit.json`
- `validation/results/v18_results_clutter_smoke.json`
- `validation/results/v18_table_layouts_smoke.json`
- `validation/results/v18_interpretation_wording_smoke.json`
- `validation/results/v18_report_export_flow_smoke.json`
- `validation/results/v18_results_report_refinement_audit.json`
