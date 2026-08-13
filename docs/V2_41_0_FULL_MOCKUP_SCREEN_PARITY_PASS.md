# QuickPLS v2.41.0 Full Mockup Screen Parity Pass

QuickPLS v2.41.0 turns the supplied QuickPLS 2.0 mockup set into a strict parity contract for the `?native_shell=1&mockup_parity=1` route.

## Scope

- Frontend/product only.
- No estimator, validation, project archive, result schema, tolerance, or numerical fingerprint changes.
- Preserve the existing SEM designer behavior while matching the mockup workbench chrome around it.
- Keep older frontend capabilities available outside the parity route until post-parity review.

## Evidence

- `validation/mockups/v2410_mockup_manifest.json` maps every supplied mockup PNG to an app state or dialog.
- `validation/v2410_mockup_manifest_audit.py` verifies the manifest coverage.
- `validation/v2410_mockup_visual_parity_smoke.mjs` captures every mapped state at the mockup viewport under `validation/results/screens/v2410/mockup-parity/`.
- `validation/v2410_mockup_visual_parity_audit.py` checks the parity route, smoke output, docs, registry, and frontend-only boundaries.
- `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md` records existing non-mockup features intentionally deferred from the parity route.

## Acceptance

The gate `v2_41_0_full_mockup_screen_parity_pass` passes only when the mockup manifest, screenshot smoke, static audit, and registry slice are all present and passing. P0/P1/P2 mismatches remain blocking; only tiny rendering differences can be documented later as low-severity P3 differences.
