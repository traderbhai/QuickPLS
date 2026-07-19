# Gaussian-Copula Endogeneity v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported Gaussian-copula diagnostic scope. Broader endogeneity claims outside this contract remain unsupported.

`AnalysisMethod::Endogeneity` runs the ordinary PLS estimator first, then adds a Gaussian-copula diagnostic for each structural equation. The current result reports `method_version = "gaussian_copula_endogeneity_v1"` and stores a typed `endogeneity` payload.

Implemented contract:

- construct scores come from the same PLS execution recipe as the requested model;
- for every target construct, all structural predecessors are included in the augmented regression;
- each predecessor also receives a rankit inverse-normal copula term;
- the payload reports the original path coefficient, copula coefficient, standard error, t-statistic, two-sided p-value, predictor skewness, applicability flag, and warning;
- predictors with absolute skewness below `0.5` are marked as weak-applicability cases because the diagnostic assumes nonnormal predictor scores.

Unsupported in this preview:

- PCA weighting;
- bootstrap/permutation inference for copula coefficients;
- UI-specific endogeneity tables beyond typed result serialization;
- validated causal interpretation.

Validation evidence:

- `npm run qpls:endogeneity:reference` writes `validation/results/endogeneity_reference_report.json`.
- The reference script independently estimates PLS scores, applies the rankit inverse-normal transform, runs the augmented regression, and compares copula coefficients, standard errors, t-statistics, and skewness within `1e-6`.
- Current observed max delta is `5.54e-09`.

Publication status: experimental. Treat the output as a diagnostic screen until broader simulation, published examples, UI/report review, and release-family promotion are complete.
