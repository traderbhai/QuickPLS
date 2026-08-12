# QuickPLS v2.0.10 Visual Gap Audit

## Summary

v2.0.10 adds a rendered-screen visual QA layer for the QuickPLS 2.0 redesign. It captures the actual app at desktop viewports, checks the visible shell against the approved mockup direction, and writes a structured issue register for the next implementation pass.

This milestone is frontend/product-only. No estimator changes, formula changes, result schema changes, project archive changes, validation tolerance changes, or numerical fingerprint changes are included.

## Scope

- Capture Home, Data, Model, Setup, Run, Results, Report, Trust, and Settings at `1440x900`.
- Capture a representative desktop subset at `1280x800`.
- Verify visible v2 shell structure, rail selection, first-viewport screen identity, horizontal overflow, mojibake safety, claim boundaries, trust entry points, and Results interpretation/reportability entry points.
- Preserve versioned artifact naming through `qpls:desktop:build-versioned`.

## Evidence

- `validation/v210_visual_gap_smoke.mjs`
- `validation/v210_visual_gap_audit.py`
- `validation/results/v210_visual_gap_smoke.json`
- `validation/results/v210_visual_gap_audit.json`
- Screenshots under `validation/results/screens/v210/visual-gap/`
- Gate: `cargo run -p qpls-cli -- gate v2_0_10_visual_gap_audit`

## Acceptance

- Every primary workspace screenshot is generated.
- The 1280 desktop subset is generated.
- No high-severity visual gap remains in the automated issue register.
- No mojibake or SmartPLS-equivalence claim appears in rendered screens.
- Version metadata and artifact labels use `2.0.10` and `v2_0_10_visual_gap_audit`.

## Next Use

The issue register is intended to drive the next QuickPLS 2.0 implementation pass. It makes visual gaps explicit before deeper pixel-level refinements continue.
