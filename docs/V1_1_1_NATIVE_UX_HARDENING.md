# QuickPLS v1.1.1 Native UX Hardening

QuickPLS v1.1.1 tightens the desktop UX evidence added in v1.1. The target remains the Windows desktop application; mobile layouts are non-gating.

## Completed Scope

- Native workflow evidence now has a stricter `v111_native_gui_workflow_smoke` artifact that launches the release executable, runs the production native workflow smoke, records the UIA automation dependency status, and verifies release-only workflow coverage for import-style ingestion, PLS execution, save/reopen, layout persistence, autosave/recovery evidence, and XLSX export.
- SEM designer evidence now includes medium and large model visual smoke fixtures, SmartPLS-style arrangement, latent overlap checks, structural edge rendering checks, and saved screenshots under `validation/results/screens/v111/sem-designer/`.
- The inspector is simplified through progressive sections: Essentials, Indicators, Layout, Advanced semantics, and Diagnostics. Common layout actions are available without opening advanced semantics.
- Method settings now use progressive disclosure. Recommended defaults are visible, and advanced resampling/reproducibility controls are collapsed until needed.
- Report/export parity now compares preview and exported SVGs across model-only, completed-result, large, formative, current-layout, and tidy-layout cases.
- Keyboard workflow evidence now launches the release executable and reuses the completed-result keyboard table contract for Results and Report surfaces.
- Disabled-action coverage now has a source-level audit for Run, Report, Data metadata, canvas lock state, covariance/path actions, native-only XLSX, and progressive settings guidance.

## Evidence

- `validation/results/v111_native_gui_workflow_smoke.json`
- `validation/results/v111_sem_designer_dense_smoke.json`
- `validation/results/v111_settings_ux_smoke.json`
- `validation/results/v111_report_export_parity.json`
- `validation/results/v111_keyboard_native_smoke.json`
- `validation/results/v111_disabled_actions_audit.json`
- `validation/results/screens/v111/`

## Validation Boundary

`pywinauto` is a validation-only dependency for literal Windows UI Automation. It is not bundled with QuickPLS and is not required at runtime. When unavailable, the v1.1.1 native scripts still require release-executable launch plus production native workflow evidence and explicitly record the missing UIA layer instead of silently claiming OS-dialog automation.

Native PDF/PNG export remains out of scope. SVG remains the audited publication diagram export.
