# CB-SEM ML v1

`cbsem_ml_v1` is validated for the documented QuickPLS v1.2.4 raw-data single-group reflective ML SEM scope emitted from `AnalysisMethod::Cbsem` when recipe metadata sets `cbsem_model_type = "sem"` or leaves the default SEM mode.

## Scope

- Compiles the current visual model into SEM matrices for reflective loading blocks, recursive latent paths, exogenous latent covariances, latent disturbance variances, observed residual variances, implied covariance, residual covariance, and residual correlation tables.
- Optimizes free SEM parameters directly with deterministic quasi-Newton iterations against `F_ml = log|Sigma(theta)| + tr(S Sigma(theta)^-1) - log|S| - p` on listwise raw-data covariance.
- Uses first-loading marker identification and supports simple one-factor CFA, correlated two/three-factor CFA, recursive latent regression, mediation-style SEM, and correlated exogenous factors.
- Reports optimized unstandardized parameters, expected-information standard errors, z statistics, p values, `std_lv`, `std_all`, fit indices, modification-index screening, diagnostics, and method warnings.

## Unsupported

Robust corrections, WLSMV/polychoric estimators, ordinal-specific treatment, equality constraints beyond the bounded preview, formative constructs, interactions, higher-order constructs, case weights, nonrecursive paths, mean-structure publication claims, unrestricted multigroup/invariance, and CB-SEM bootstrap publication claims remain experimental or unsupported outside the v1.2.4 scope.

## Validation

`npm run qpls:cbsem:sem-reference` writes the bounded v0.7 smoke report. `npm run qpls:cbsem:lavaan-sem` and `npm run qpls:cbsem:lavaan-validate` generate development-only lavaan parity fixtures under `validation/results/`, including estimates, fit indices, expected-information SEs, z/p values, and standardized estimates.
