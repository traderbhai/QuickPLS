# WPLS v1

Status: Supported Standard scope for documented positive case-weighted reflective WPLS with standardized preprocessing and path or factor weighting. The final current-source packaged receipt and audit are refreshed only after the shared release binary is frozen. Broader weighted estimators outside this contract remain unsupported.

`AnalysisMethod::Wpls` runs a case-weighted PLS estimator using an explicit `settings.case_weight_column`. The current result reports `method_version = "wpls_case_weighted_v1"` and stores a typed `wpls` payload with the weight column, weight sum, effective sample size, covariance convention, and warnings.

Implemented contract:

- complete cases are selected across model indicators and the case-weight column;
- missing weights are handled by the fixed listwise-deletion row policy and must reproduce an otherwise identical dataset with those rows removed;
- non-finite, zero, or negative weights are rejected before estimation;
- indicator preprocessing uses weighted means and unbiased weighted sample standard deviations;
- Mode A outer weights use weighted covariance between indicators and inner proxies;
- construct scores are standardized with the same case weights;
- structural paths are estimated with weighted least squares;
- outer loadings and R2 use the same weighted covariance and weighted residual definitions.

Outside the supported method scope:

- PCA weighting;
- formative constructs;
- generated interaction or higher-order construct workflows;
- bootstrap, permutation, or jackknife inference under case weights;
- publication-ready weighting recommendations.

Validation evidence:

- `npm run qpls:wpls:reference` writes `validation/results/wpls_reference_report.json`.
- The reference script independently implements weighted standardization, weighted covariance, weighted score iteration, weighted path estimation, weighted loadings, weighted R2, weight-sum metadata, effective sample size, missing-weight recipe validation, and negative-weight runtime rejection.
- Current observed max delta is `3.41e-13`.

Qualification status: the native method selection, case-weight picker, applicability checks, result tables, and export projections are source-bound and tested. Shipping still requires the final frozen-binary packaged Windows execution, export, save/reopen, and method audit to agree with the manifest. WPLS inference, generated interaction/HOC workflows, formative blocks, and PCA weighting remain outside the supported scope.
