# v1.6.2 Data/Home launch polish

This frontend-only milestone improves the project launch and data-to-model path.

## Scope

- Home computes a recommended next step from the current workflow state.
- Home uses a compact workflow status list for Data, Model, Setup/Run, and Report.
- Home keeps save/open/demo/recent project actions visible.
- Data keeps import source, quality cards, metadata preview, sample dataset details, and prefix-based construct creation in a clear launch path.

## Non-engine boundary

No statistical engines, formulas, import backends, result schemas, project format, validation tolerances, or numerical fingerprints are changed.

## Evidence

- `validation/results/v162_data_home_smoke.json`
- `validation/results/v162_data_home_audit.json`
- `cargo run -p qpls-cli -- gate v1_6_2_data_home_launch_polish`
