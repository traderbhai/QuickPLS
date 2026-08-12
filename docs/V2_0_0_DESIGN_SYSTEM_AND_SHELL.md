# QuickPLS v2.0.0 Design System And Shell

## Summary

v2.0.0 starts the QuickPLS 2.0 desktop redesign. This milestone creates the shared application shell and visual foundation needed to match the approved professional mockup across later screens.

This is a frontend/product milestone only. It does not change statistical engines, formulas, method validation, result schemas, project archive format, or numerical fingerprints.

## Completed Scope

- Updated app, Tauri, package-lock, and Rust workspace version metadata to `2.0.0`.
- Updated release artifact label to `v2_0_0_design_system_and_shell`.
- Added first-class navigation routes for:
  - Trust Center
  - Settings
- Added Trust Center workspace for validated scope, runtime boundaries, and method compatibility summary.
- Added Settings workspace for UI-only desktop preferences.
- Added shared v2 shell tokens and workspace panel styles.
- Updated command palette access for Trust Center and Settings.
- Added smoke/audit scripts:
  - `validation/v200_shell_smoke.mjs`
  - `validation/v200_shell_audit.py`

## Non-Goals

- No estimator or backend numerical changes.
- No result payload changes.
- No project format change.
- No SEM Designer rewrite in this milestone.
- No SmartPLS equivalence claim.

## Next Milestones

1. `v2_0_1_home_data_redesign`
2. `v2_0_2_setup_method_guidance_redesign`
3. `v2_0_3_results_table_interpretation_redesign`
4. `v2_0_4_report_export_redesign`
5. `v2_0_5_trust_center_settings`
6. `v2_0_6_sem_designer_visual_integration`
7. `v2_0_7_full_visual_qa_and_release`

## Verification

```powershell
npm test -- --run
npm run build
npm run qpls:v200:shell-smoke
npm run qpls:v200:shell-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_0_design_system_and_shell
npm run qpls:desktop:build-versioned
```
