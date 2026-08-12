# REGRESSION_LOGISTIC_V1

Status: historical archive-readable contract. New execution and append are disabled;
use `regression_logistic_v2`.

`regression_logistic_v1` was the first standalone binary logistic regression
contract for raw numeric data. Existing results remain immutable and readable.
It is not accepted as newly generated scientific evidence because it did not
persist the complete-case outcome profile, optimizer convergence record,
classification identities, odds-ratio intervals, or expanded likelihood-fit
identities now required by v2.

## Contract

- Metadata selects `regression_type = logistic`.
- The outcome must contain binary 0/1 values after listwise deletion.
- The optimizer uses deterministic IRLS/Newton-style updates.
- Output includes coefficients, Wald standard errors, z statistics, p values, confidence intervals, odds ratios, log-likelihood, pseudo-R2, AIC, BIC, predicted probabilities, and convergence warnings.
- Complete separation, nonconvergence, rank deficiency, and insufficient complete cases are blocked or warned.

## Historical exclusions

- Multinomial or ordinal logistic regression.
- Firth correction.
- Clustered, weighted, or robust covariance estimators.
- Publication claims outside the documented binary numeric complete-case scope.

## Validation

The retained v1 evidence compares bounded-fixture estimates against an
independent Python IRLS implementation and an R `glm` reference. It remains
historical evidence for archived output only; it cannot promote or authorize a
new v1 run. Current execution, persistence, and promotion evidence must bind
`regression_logistic_v2`.
