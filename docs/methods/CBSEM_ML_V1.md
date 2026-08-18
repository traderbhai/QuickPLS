# CB-SEM ML v1

`cbsem_ml_v1` is release-qualified for the documented QuickPLS v1.2.4 raw-data single-group reflective ML SEM scope emitted from `AnalysisMethod::Cbsem` when recipe metadata sets `cbsem_model_type = "sem"` or leaves the default SEM mode. Current evidence binds genuine packaged setup, invalid-setup protection, ML results, XLSX export, save/reopen, physical viewports, and cleanup. The optional bootstrap-v2 candidate remains separately unqualified.

## Scope

- Compiles the current visual model into SEM matrices for reflective loading blocks, recursive latent paths, exogenous latent covariances, latent disturbance variances, observed residual variances, implied covariance, residual covariance, and residual correlation tables.
- Optimizes free SEM parameters directly with deterministic quasi-Newton iterations against `F_ml = log|Sigma(theta)| + tr(S Sigma(theta)^-1) - log|S| - p` on listwise raw-data covariance.
- Uses first-loading marker identification and supports simple one-factor CFA, correlated two/three-factor CFA, recursive latent regression, mediation-style SEM, and correlated exogenous factors.
- Reports optimized unstandardized parameters, expected-information standard errors, z statistics, p values, `std_lv`, `std_all`, fit indices, residual-based modification screening, diagnostics, and method warnings.

## Unsupported

Robust corrections, WLSMV/polychoric estimators, ordinal-specific treatment, equality constraints beyond the bounded preview, formative constructs, interactions, higher-order constructs, case weights, nonrecursive paths, mean-structure publication claims, unrestricted multigroup/invariance, and CB-SEM bootstrap publication claims remain experimental or unsupported outside the v1.2.4 scope.

## Validation

`npm run qpls:cbsem:sem-reference` writes the bounded reference report. `npm run qpls:cbsem:lavaan-sem` and `npm run qpls:cbsem:lavaan-validate` generate independent lavaan parity fixtures under `validation/results/`, including estimates, fit indices, expected-information SEs, z/p values, and standardized estimates. The factory repeats the frozen data and model shapes, exercises data and variable reorder equivalence plus exact repeated execution, and binds every report to the current method source bytes.

The v247 packaged acceptance proves a genuine 240-case desktop ML run, accessible numeric result tables, native XLSX export, typed archive persistence, and same-run reopen. That packaged evidence is reusable only while its coordinated build receipt still matches every current product source byte; otherwise the release tier remains blocked until a new build and complete acceptance run are captured.

This qualification is an independent QuickPLS method claim. It does not assert numerical or workflow equivalence with every CB-SEM feature in SmartPLS or with any excluded estimator, model shape, or inference procedure.
