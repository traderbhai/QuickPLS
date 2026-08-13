# QuickPLS v2.0.6 Model Shell And SEM Designer Surround

Status: validated after `qpls:v206:model-shell` passes.

## Summary

v2.0.6 applies the QuickPLS 2.0 visual contract to the Model workspace shell around the existing SEM Designer. The milestone keeps the current SEM diagram grammar and editing behavior, while making the surrounding explorer, canvas toolbar, overlay/status surfaces, and inspector feel like one professional desktop workbench.

## Complete

- Added v2 shell hooks for the Model workspace:
  - `model-v2-explorer`;
  - `model-v2-canvas`;
  - `model-v2-toolbar`;
  - `model-v2-inspector`.
- Restyled the SEM Explorer with v2 panel headers, status cards, guidance cards, tabs, selected rows, and summary chips.
- Restyled the canvas toolbar, context toolbar, dropdown menus, and overlay/status surfaces using the shared v2 desktop tokens.
- Restyled the inspector tabs, section cards, form controls, actions, and method notes so the right panel matches the v2 shell.
- Fixed remaining visible `R²` encoding in Model and Inspector surfaces.
- Added smoke and audit scripts:
  - `validation/v206_model_shell_smoke.mjs`;
  - `validation/v206_model_shell_audit.py`.
- Added registry gate `v2_0_6_model_shell_sem_designer_surround`.
- Updated release metadata to `2.0.6` and artifact labeling to `v2_0_6_model_shell_sem_designer_surround`.

## Non-Goals

- No statistical engines, formulas, estimator behavior, result values, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.
- No change to the SEM Designer's underlying recipe model.
- No SmartPLS equivalence claim.
- No new method promotion.

## Verification

Required commands:

```powershell
npm run build
npm test -- --run
npm run qpls:v206:model-shell-smoke
npm run qpls:v206:model-shell-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_6_model_shell_sem_designer_surround
npm run qpls:desktop:build-versioned
```

## Next

Continue the QuickPLS 2.0 rebuild with the Run execution surface, Trust Center, Settings, and final visual parity passes.
