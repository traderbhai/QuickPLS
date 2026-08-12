# QuickPLS v2.15.0 Workflow Method Guidance Triage Pass

Status: validated after the v2.15 smoke/audit/gate pass.

This frontend-only milestone turns the v2.14 real-dataset triage lane for workflow and method confusion into a grouped product pass. It improves the way QuickPLS explains which methods fit the current data and model, why a method is blocked or only available after setup, and what the researcher should do next.

## What Changed

- Data now exposes a visible recommended next move derived from dataset-level method guidance.
- Model explorer guidance now states that recommendations are based on construct modes, assigned indicators, structural paths, and the current dataset.
- Setup now includes a selected-method decision panel with the status, first failed requirement, next action, expected outputs, and a short "If you expected another method" explanation.
- Method cards expose next-action and failed-check metadata for smoke/audit coverage.
- The top command bar uses current v2.15 guided setup wording and keeps the method selector conservative.
- R-squared output labels in method applicability remain encoded as `R²`.

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints changed. This is product guidance and frontend presentation only.

## Evidence

- `validation/results/v2150_workflow_method_guidance_smoke.json`
- `validation/results/v2150_workflow_method_guidance_audit.json`
- `cargo run -p qpls-cli -- gate v2_15_0_workflow_method_guidance_triage_pass`
