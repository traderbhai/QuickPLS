# Gaussian-Copula Endogeneity v1

Status: implemented for the bounded QuickPLS Gaussian-copula diagnostic scope. Engine, archive, Native Results, and same-run export checks are available, but factory identities and packaged desktop acceptance are still required before a release-qualified claim.

`AnalysisMethod::Endogeneity` runs the ordinary PLS estimator first, then adds a Gaussian-copula diagnostic for each structural equation. The current result reports `method_version = "gaussian_copula_endogeneity_v1"` and stores a typed `endogeneity` payload.

The frozen method identity is `gaussian_copula_endogeneity_v1`.

Implemented contract:

- construct scores come from the same PLS execution recipe as the requested model;
- for every target construct, all structural predecessors are included in the augmented regression;
- each predecessor also receives a rankit inverse-normal copula term;
- the payload reports the original path coefficient, copula coefficient, standard error, t-statistic, two-sided p-value, predictor skewness, applicability flag, and warning;
- predictors with absolute sample skewness below `0.5` are marked as weak-applicability cases because the diagnostic assumes nonnormal predictor scores.
- Native Results projects the typed diagnostics into an accessible table, and the same-run table set is used by CSV, HTML, and XLSX export paths.

Unsupported outside the bounded diagnostic scope:

- PCA weighting;
- bootstrap/permutation inference for copula coefficients;
- control paths, generated interaction constructs, and higher-order-construct execution;
- instrumental-variable correction, custom copula families, and automatic path correction;
- validated causal interpretation.

Validation evidence:

- `npm run qpls:endogeneity:reference` writes `validation/results/endogeneity_reference_report.json`.
- The reference script independently estimates PLS scores, applies the rankit inverse-normal transform, runs the augmented regression, and compares copula coefficients, standard errors, t-statistics, and skewness within `1e-6`.
- Current observed max delta is `5.54e-09`.
- `npm run qpls:endogeneity:factory:check` runs the deterministic null/signal matrix, boundary and archive checks, and focused Native Results/export tests without writing promotion identities.
- Identity writing is a separate operation and must bind the final checkout bytes. Packaged desktop acceptance remains fail-closed and is not implied by source-level checks.

Interpretation: a significant or nonsignificant copula coefficient is diagnostic, not proof of causality. Do not describe this method as a causal correction or as release-qualified until the manifest derives that state from current evidence.
