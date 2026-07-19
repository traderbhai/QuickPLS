# PLS Assessment Method Specification v1

Status: superseded by v2. Legacy v1 payloads remain readable.

V1 was the initial experimental assessment wire contract. It contained Cronbach alpha, composite reliability rho_C, AVE, cross-loadings, a Fornell-Larcker matrix, exact estimator R-squared reuse, warnings, and, in later compatible payloads, an optional HTMT matrix. Optional tables may therefore be absent in archived v1 results.

The formulas retained by v2 are:

- Standardized alpha: `K / (K - 1) * (1 - K / (1' R 1))`.
- rho_C: `(sum lambda_k)^2 / ((sum lambda_k)^2 + sum(1 - lambda_k^2))`.
- AVE: `sum(lambda_k^2) / K`.
- Cross-loadings: indicator-to-construct-score Pearson correlations on estimator complete cases.
- Fornell-Larcker: `sqrt(AVE)` on reflective diagonals and construct-score correlations off diagonal.
- HTMT, when present: mean absolute cross-block correlation divided by the geometric mean of the two mean absolute within-block correlations.

V1 did not require adjusted R-squared, structural VIF, or formative-indicator VIF. Those fields and their strict archive validation begin with `pls_assessment_v2`; see `PLS_ASSESSMENT_V2.md`.
