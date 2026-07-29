# v1.6.3 Global design-system and accessibility pass

Status: validated.

This frontend-only milestone closes the last planned remediation item from the v1.5.7 UI/UX launch-quality audit. It focuses on global release-label consistency, source-level mojibake prevention, keyboard focus contracts, scoped status wording, and disabled-action accessibility.

## Completed

- Updated the visible top-bar milestone label to `v1.6.3 design and accessibility pass`.
- Added source-level audits for stale live release text, mojibake markers, scoped validation wording, and accessible Run disabled-state wiring.
- Reused and enforced existing Vitest contracts for keyboard-focusable tables, SEM canvas overlay state, keyboard shortcuts, and the persistent readiness checklist.
- Added smoke and audit evidence:
  - `validation/results/v163_design_accessibility_smoke.json`
  - `validation/results/v163_design_accessibility_audit.json`

## Boundary

No statistical engine, formula, method validation tolerance, result payload, project schema, import backend, or numerical fingerprint was changed.
