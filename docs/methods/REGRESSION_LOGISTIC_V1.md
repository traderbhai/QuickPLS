# REGRESSION_LOGISTIC_V1

Status: experimental v0.8 preview.

`regression_logistic_v1` provides standalone binary logistic regression for raw numeric data.

## Contract

- Metadata selects `regression_type = logistic`.
- The outcome must contain binary 0/1 values after listwise deletion.
- The optimizer uses deterministic IRLS/Newton-style updates.
- Output includes coefficients, Wald standard errors, z statistics, p values, confidence intervals, odds ratios, log-likelihood, pseudo-R2, AIC, BIC, predicted probabilities, and convergence warnings.
- Complete separation, nonconvergence, rank deficiency, and insufficient complete cases are blocked or warned.

## Unsupported In v0.8

- Multinomial or ordinal logistic regression.
- Firth correction.
- Clustered, weighted, or robust covariance estimators.
- Publication-stable inference claims.

## Validation

`npm run qpls:regression:logistic-reference` compares bounded-fixture estimates against an independent Python IRLS implementation. Later promotion requires rare-event, separation, and second-source R `glm` evidence.
