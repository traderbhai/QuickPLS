# v1.6.1 Setup/Run workflow consolidation

This frontend-only milestone reduces duplication between Setup and Run.

## Scope

- Setup remains the primary place for method selection, readiness review, presets, resampling settings, and group/prediction configuration.
- Setup now exposes the production `quickpls:run-analysis` action directly after the run summary.
- Run becomes a compact execution monitor and handoff page instead of a second full setup page.
- Run links configuration changes back to Setup.

## Non-engine boundary

No statistical engines, formulas, result schemas, project format, validation tolerances, or numerical fingerprints are changed.

## Evidence

- `validation/results/v161_setup_run_smoke.json`
- `validation/results/v161_setup_run_audit.json`
- `cargo run -p qpls-cli -- gate v1_6_1_setup_run_workflow_consolidation`
