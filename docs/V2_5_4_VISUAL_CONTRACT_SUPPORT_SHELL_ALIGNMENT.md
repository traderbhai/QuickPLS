# QuickPLS v2.5.4 Visual Contract Support-Shell Alignment

Status: validated.

This frontend/product milestone aligns the QuickPLS 2.0 visual contract with the v2.5 shell decisions: Home, Trust Center, and Settings are support utilities, while Data, Model, Setup, Run, Results, and Report remain the primary calculation workflow.

## What Changed

- Updated `docs/V2_UI_VISUAL_CONTRACT.md` so the workflow strip no longer lists Home as a calculation step.
- Added the support utility shell as a first-class visual contract rule for Home, Trust Center, and Settings.
- Documented that Model may keep its dedicated SEM Designer workflow band while other calculation pages use the shared workflow strip and coach.
- Fixed the remaining R-squared encoding artifact in the visual contract.
- Added a static audit to keep the support-shell contract, version metadata, and frontend-only boundary enforceable.

## Evidence

- `validation/v254_visual_contract_audit.py`
- `validation/results/v254_visual_contract_audit.json`
- `docs/V2_UI_VISUAL_CONTRACT.md`
- `docs/DELIVERY_STATUS.md`
- `docs/DEVELOPMENT_LEDGER.md`

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, analysis recipes, or numerical fingerprints changed.
