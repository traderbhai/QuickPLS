# PLS Assessment Method Specification v2

Status: superseded by v3. Legacy v2 results remain readable and may omit effect sizes.

## Scope and Inputs

This specification covers Cronbach alpha, composite reliability rho_C, average variance extracted (AVE), cross-loadings, Fornell-Larcker, HTMT, structural R-squared and adjusted R-squared, inner-model VIF, and formative-indicator VIF for a converged `pls_pm_v1` result. It uses the same raw data, model-wide complete cases, standardized outer loadings, and unit-variance construct scores as estimation.

## Measurement Quality

- Standardized Cronbach alpha is `K / (K - 1) * (1 - K / (1' R 1))` for indicator correlation matrix `R`.
- Composite reliability is `(sum lambda_k)^2 / ((sum lambda_k)^2 + sum(1 - lambda_k^2))`.
- AVE is `sum(lambda_k^2) / K`.
- Reliability and AVE are not applicable to formative blocks; alpha is undefined for single-item reflective blocks.
- Cross-loadings are indicator-to-construct-score Pearson correlations on the estimation rows.
- Fornell-Larcker uses `sqrt(AVE)` on reflective diagonals and construct-score correlations off diagonal.
- HTMT is the mean absolute cross-block indicator correlation divided by the geometric mean of the two mean absolute within-block correlations. Formative, undersized, or zero-denominator cells are not applicable. The definition follows Henseler, Ringle, and Sarstedt (2015): https://doi.org/10.1007/s11747-014-0403-8.

## Structural Quality

For `n` observations and `k` direct predecessors, adjusted R-squared is `1 - (1 - R^2)(n - 1)/(n - k - 1)` and is unavailable when `n <= k + 1`. This is the standard intercept-model correction described by Penn State STAT 501: https://online.stat.psu.edu/stat501/Lesson05.

For predictor `j`, `VIF_j = 1/(1 - R_j^2)`, where the auxiliary regression predicts `j` from the other predictors in that equation: https://online.stat.psu.edu/stat462/node/180/.

- Inner VIF uses construct scores for the direct predecessors of each endogenous construct.
- Formative VIF uses standardized complete-case indicators within each formative block.
- A sole predictor has VIF one. Perfect auxiliary explanation produces an unavailable value plus a collinearity warning, never infinity.
- No interpretation cutoff is enforced by the engine.

## Determinism and Validation

Assessment contains no random operation, reuses estimator R-squared exactly, exposes cooperative cancellation checkpoints, and rejects non-finite or dimensionally inconsistent inputs. Unsupported cells are explicit null values with contextual warnings. V2 project payloads require internally consistent, unique, finite measurement, matrix, structural-quality, and VIF tables. Cohen f-squared begins with v3.

## Reference Fixture

`validation/fixtures/simple_reflective.csv` agrees with development-only cSEM 0.6.1 for AVE, rho_C, standardized alpha, construct correlation, and R-squared within `1e-6`; own-construct cross-loadings reproduce estimator loadings within `1e-12`. HTMT, adjusted R-squared, and VIF still require the full two-reference release gate.
