# QuickPLS v2.5.5 Support Shell Viewport Alignment

Status: validated.

This frontend-only milestone aligns the Home, Trust Center, and Settings support utility bar with the same workspace gutters used by the support pages, and verifies the support shell at the two desktop target viewports.

## What Changed

- Wrapped the support utility bar in a `support-utility-frame` that shares the workspace max width and page gutters.
- Kept Home, Trust Center, and Settings visually connected as support utilities without reintroducing workflow progress controls.
- Preserved Data, Model, Setup, Run, Results, and Report as calculation workflow screens without support utility controls.
- Added viewport smoke evidence at `1440x900` and `1280x800`.
- Added static audit coverage for frame CSS, support-shell implementation hooks, version metadata, and frontend-only boundaries.

## Evidence

- `validation/v255_support_viewport_smoke.mjs`
- `validation/v255_support_viewport_audit.py`
- `validation/results/v255_support_viewport_smoke.json`
- `validation/results/v255_support_viewport_audit.json`
- `validation/results/screens/v255/support-viewport/`

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, analysis recipes, or numerical fingerprints changed.
