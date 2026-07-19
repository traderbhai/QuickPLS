# REGRESSION_OLS_V1

Status: experimental v0.8 preview.

`regression_ols_v1` provides standalone ordinary least-squares regression for raw numeric data.

## Contract

- Metadata selects `regression_type = ols`.
- Required metadata:
  - `regression_outcome`
  - `regression_predictors`
- Optional metadata:
  - `regression_controls`
  - `robust_se = none|hc0|hc3|hc4`
- The v0.8 engine fits an intercept model, reports coefficients, standard errors, t statistics, two-sided p values, confidence intervals, residual diagnostics, R2, adjusted R2, F statistic, AIC, BIC, RMSE, fitted values, and residuals.
- Rank-deficient designs and insufficient complete cases are rejected.

## Unsupported In v0.8

- Categorical encoding helpers.
- Survey weights or clustered standard errors.
- GLS, mixed models, and panel models.
- Publication-validated robust-SE promotion.

## Validation

`npm run qpls:regression:ols-reference` compares coefficients and R2 against an independent NumPy least-squares fixture. R/base `lm` remains a validation-only second-source candidate for later promotion.
