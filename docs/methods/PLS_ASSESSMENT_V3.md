# PLS Assessment Method Specification v3

Status: superseded by v4. Legacy v3 results remain readable and may contain Cohen f-squared, but not v4 fit or blindfolding fields.

V3 retains the v2 definitions for standardized Cronbach alpha, rho_C, AVE, cross-loadings, Fornell-Larcker, HTMT, R-squared, adjusted R-squared, inner-model VIF, and formative-indicator VIF. Their formulas and applicability rules remain unchanged in `PLS_ASSESSMENT_V2.md`.

## Cohen f-squared

For every unique directed structural path, QuickPLS estimates a reduced structural equation with that path removed while retaining the already-estimated construct scores. The target equation is recomputed by OLS with all other direct predecessors, matching cSEM `calculatef2()`. If the removed path was the target's only predecessor, the excluded R-squared is zero. The local effect size is:

`f^2 = (R^2_included - R^2_excluded) / (1 - R^2_included)`

This is Cohen's R-squared-change effect definition (Cohen, 1988, *Statistical Power Analysis for the Behavioral Sciences*, Chapter 9) applied to the fixed latent-score structural equation.

- When removing the path leaves the target with no predecessors, its intercept-only excluded R-squared is zero.
- When `1 - R^2_included <= 1e-12`, f-squared is unavailable and a denominator warning is emitted.
- When the fixed-score reduced structural regression is rank deficient or non-finite, included R-squared remains available while excluded R-squared and f-squared are null, with the regression error preserved in warnings.
- Cancellation is checked around every path-level reduced regression. Partial effect-size output is not returned after cancellation.
- Result rows retain deterministic recipe path order. Duplicate paths are rejected by current recipe validation.

## Result Contract

Every v3 assessment result contains exactly one effect-size row per persisted structural path. Project validation requires unique path identities, included R-squared equal to the authoritative full estimation result, finite optional values, and formula agreement whenever both excluded R-squared and f-squared are available. V1 and v2 remain explicit read-compatibility contracts and may omit this table.

## Validation State

The implementation has hard-coded regression-reference values, path-order metamorphic coverage, cancellation, failed-reduced-regression, perfect-denominator, and legacy-deserialization tests. cSEM agreement is covered by `npm run qpls:assessment:csem`; simulation qualification remains required before validation promotion.
