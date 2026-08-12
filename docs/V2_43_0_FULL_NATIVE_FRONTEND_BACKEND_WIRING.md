# QuickPLS v2.43.0 Full Native Frontend/Backend Wiring

`v2_43_0_full_native_frontend_backend_wiring` closes the main native-shell wiring gap. Visible default-shell commands now either call existing production behavior, open a real task dialog, or are intentionally absent.

## Completed

- Close Project now opens a Save and close / Close without saving / Cancel decision dialog and clears active project state safely.
- Pause is removed from the default Run UI; QuickPLS supports cancellation, not fake suspension.
- Data commands use desktop task dialogs for transform, add column, recode, missing values, filter, and sort, then dispatch structured payloads into the real Data workspace.
- Workbench layout commands save/reset layout, close/restore optional panes, and toggle the status bar using UI-only local preferences.
- Help > Documentation opens an offline in-app documentation dialog.
- Trust Center checksum verification opens a release-integrity detail dialog.
- The native wiring matrix documents every visible command state.

## Boundary

This milestone does not change estimator formulas, result schemas, validation tolerances, project archive semantics, or numerical fingerprints.

## Evidence

```powershell
npm run qpls:v243:full-wiring
cargo check -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_43_0_full_native_frontend_backend_wiring
```
