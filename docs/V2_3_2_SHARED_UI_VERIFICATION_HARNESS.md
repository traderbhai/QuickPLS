# QuickPLS v2.3.2 Shared UI Verification Harness

Status: `validated`

This frontend/product-only milestone consolidates repeated v2 UI smoke and audit logic into reusable validation helpers. The milestone keeps the QuickPLS 2.0 visual shell, command-bar readiness contract, and v2.3.1 integrity checks intact while reducing duplicated validation code for future UI milestones.

## Scope

- Shared Playwright preview startup, teardown, screenshot, shell snapshot, and shell-integrity checks live in `validation/lib/v2_ui_smoke_harness.mjs`.
- Shared static audit helpers for version metadata, command-bar contracts, forbidden text, and frontend boundary checks live in `validation/lib/v2_ui_audit.py`.
- v2.3.1 smoke/audit now use the shared helpers; the v2.3.2 audit verifies that migration statically because older milestone scripts intentionally assert older visible version labels.
- v2.3.2 smoke/audit verify the shared harness itself and the top-command-bar blocker navigation contract.

## Evidence

- `validation/v231_ui_integrity_smoke.mjs`
- `validation/v231_ui_integrity_audit.py`
- `validation/results/v232_shared_ui_harness_smoke.json`
- `validation/results/v232_shared_ui_harness_audit.json`
- screenshots under `validation/results/screens/v232/shared-ui-harness/`
- gate `v2_3_2_shared_ui_verification_harness`

## Boundaries

- No statistical engine changes.
- No analysis result schema changes.
- No project archive format changes.
- No numerical fingerprint changes.
- No SmartPLS-equivalence claims.
