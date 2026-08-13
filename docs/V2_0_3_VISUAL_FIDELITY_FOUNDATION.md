# QuickPLS v2.0.3 Visual Fidelity Foundation

Status: validated after `qpls:v203:visual-fidelity` passes.

## Summary

v2.0.3 freezes the QuickPLS 2.0 mockup-aligned visual foundation before the remaining workspace rebuilds continue. It makes the selected mockup direction actionable through shared tokens, reusable primitives, screen acceptance rules, and smoke/audit evidence.

## Complete

- Added `docs/V2_UI_VISUAL_CONTRACT.md` as the authoritative v2 visual contract.
- Hardened shared `--q2-*` CSS tokens for page gutters, panel padding, section gaps, controls, shadows, chips, card typography, and action buttons.
- Standardized `.qpls2-panel`, page title/subtitle, card body, chip, and primary/secondary action primitives.
- Added Results v2 styling hooks so the existing selected-run workbench can adopt the same visual foundation.
- Added v2.0.3 smoke and audit scripts:
  - `validation/v203_visual_fidelity_smoke.mjs`
  - `validation/v203_visual_fidelity_audit.py`
- Added registry gate `v2_0_3_visual_fidelity_foundation`.
- Updated release metadata to `2.0.3` and artifact labeling to `v2_0_3_visual_fidelity_foundation`.

## Non-Goals

- No estimator, formula, inference, assessment, result schema, project archive, or validation tolerance changes.
- No SmartPLS equivalence claim.
- No new method promotion.
- No rewrite of the SEM numerical recipe or diagram result payloads.

## Verification

Required commands:

```powershell
npm run build
npm test -- --run
npm run qpls:v203:visual-fidelity-smoke
npm run qpls:v203:visual-fidelity-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_3_visual_fidelity_foundation
npm run qpls:desktop:build-versioned
```

## Next

Use this contract for v2.0.4 Results table and interpretation redesign, then continue Report, Model shell, Run, Trust Center, and Settings refinements against the same visual foundation.
