# CFA ML v1

`cfa_ml_v1` is an experimental QuickPLS v0.7 CFA beta emitted from `AnalysisMethod::Cbsem` when recipe metadata sets `cbsem_model_type = "cfa"`.

## Scope

- Supports reflective measurement-only CFA models with at least two observed indicators per latent factor.
- Allows isolated latent factors because CFA does not require structural paths.
- Uses first-loading marker scaling and direct ML optimization of loadings, latent variances/covariances, and observed residual variances.
- Reports optimized factor loadings, latent variances/covariances, observed residual variances, expected-information standard errors, standardized solutions, residual matrices, fit indices, modification-index screening, diagnostics, and warnings.

## Unsupported

Cross-loadings as free parameters, ordinal/polychoric/WLSMV estimation, mean/intercept estimation beyond the v0.7 scalar-invariance preview, single-indicator latent factors, formative blocks, generated interactions, higher-order constructs, case weights, robust corrections, and publication-ready CFA claims are unsupported.

## Validation

`npm run qpls:cbsem:cfa-reference` writes the bounded v0.7 smoke report. `npm run qpls:cbsem:lavaan-cfa` and `npm run qpls:cbsem:lavaan-validate` compare generated one-factor, two-factor, and three-factor CFA fixtures against lavaan.
