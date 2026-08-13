# QuickPLS v2.0.7 Run Execution Surface Redesign

## Summary

v2.0.7 applies the QuickPLS 2.0 desktop visual contract to the Run workspace. The page now behaves like a calculation launch surface rather than a sparse confirmation page.

This milestone is frontend/product-only. It does not change statistical engines, method formulas, analysis recipes, result schemas, project archives, validation tolerances, or numerical fingerprints.

## What Changed

- Added a v2 calculation hero with method scope, readiness state, and the primary `Run selected method` action.
- Moved disabled-run explanations next to the disabled run action.
- Added a compact readiness checklist with direct navigation back to Data, Model, or Setup where applicable.
- Added an expected output preview for paths, loadings/weights, R², reliability/validity, inference, and report handoff.
- Added an execution provenance panel showing method, seed, workers, bootstrap, permutation, construct count, indicator count, path count, and saved-run count.
- Added clear handoff cards for Results and Report after a completed run exists.
- Preserved the existing `quickpls:run-analysis` event and desktop execution boundary.

## Acceptance

- `npm run qpls:v207:run-surface` passes.
- `cargo run -p qpls-cli -- gate v2_0_7_run_execution_surface_redesign` reports no open or blocked gates.
- Version metadata uses `2.0.7`.
- Fresh installer, portable executable, and checksums can be generated through `npm run qpls:desktop:build-versioned`.

## Out Of Scope

- No estimator behavior changes.
- No result payload changes.
- No new method validation claims.
- No project schema or archive-format changes.
