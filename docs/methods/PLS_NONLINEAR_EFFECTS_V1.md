# Nonlinear Effects v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported nonlinear-effects diagnostic scope. Broader nonlinear models outside this contract remain unsupported.

`AnalysisMethod::NonlinearEffects` runs the ordinary PLS estimator first, then adds a fixed-score quadratic diagnostic for each declared structural path. The current result reports `method_version = "pls_quadratic_nonlinear_effects_v1"` and stores a typed `nonlinear_effects` payload.

Implemented contract:

- construct scores come from the same PLS execution recipe as the requested model;
- every target equation includes its existing linear structural predecessors;
- each predecessor receives a centered squared construct-score term;
- the payload reports the original linear path coefficient, quadratic coefficient, standard error, t-statistic, two-sided p-value, linear R2, augmented R2, delta R2, and warnings;
- delta R2 is reported as zero when the augmented model does not improve fixed-score R2 beyond numerical tolerance.

Unsupported outside the validated v1.2.3 diagnostic scope:

- PCA weighting;
- spline, logarithmic, exponential, or custom nonlinear terms;
- term-specific UI configuration;
- bootstrap/permutation inference for quadratic coefficients;
- validated publication interpretation.

Validation evidence:

- `npm run qpls:nonlinear:reference` writes `validation/results/nonlinear_effects_reference_report.json`.
- The reference script independently estimates PLS scores, constructs centered squared score terms, runs the augmented regression, and compares quadratic coefficients, standard errors, t-statistics, linear R2, augmented R2, and delta R2 within `1e-6`.
- Current observed max delta is `1.96e-12`.

Publication status: validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope. Broader nonlinear SEM estimation remains unsupported.
