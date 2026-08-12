# QuickPLS v2.11.0 Method Applicability Setup Polish

## Summary

v2.11.0 improves the Setup workflow so researchers no longer face a generic list of methods. QuickPLS now surfaces project-specific method availability based on the loaded data, SEM model, selected settings, and documented scope.

This milestone is frontend/product-only. It does not change estimators, formulas, validation tolerances, result schemas, project archive format, or numerical fingerprints.

## User-Facing Changes

- Setup now includes a compact method availability summary for recommended, available, setup-required, and blocked/scoped methods.
- Method cards expose the first missing requirement directly on the card.
- The selected method side panel explains why a method is not available yet, or confirms that all required checks are satisfied.
- Data and Model guidance remain wired to the same applicability engine.
- The top-bar method selector stays conservative and points users to Setup for broader method discovery.
- The v2.11 UI fixes the remaining `R²` mojibake risk in Setup fallback text.

## Evidence

- `npm run qpls:v2110:method-setup-smoke`
- `npm run qpls:v2110:method-setup-audit`
- `cargo run -p qpls-cli -- gate v2_11_0_method_applicability_setup_polish`

Evidence files:

- `validation/results/v2110_method_setup_applicability_smoke.json`
- `validation/results/v2110_method_setup_applicability_audit.json`

## Boundary

- Frontend-only.
- No backend/statistical changes.
- No versioned desktop artifacts until the v2.11 gate is clear and full pre-artifact checks are intentionally run.
