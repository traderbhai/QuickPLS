# REGRESSION_OLS_V1

Status: Supported in Standard for the exact standalone OLS scope below.

`regression_ols_v1` is QuickPLS's standalone ordinary least-squares workflow for raw numeric data. It does not create or require an SEM model.

## Validated Contract

- Metadata selects `regression_type = ols`, one numeric `regression_outcome`, one or more distinct numeric `regression_predictors`, and optional distinct numeric `regression_controls`.
- The estimator uses unstandardized observed values, an intercept, listwise deletion, exact HC3 heteroskedasticity-consistent standard errors, two-sided Student-t tests, and fixed 95% confidence intervals. It does not cap leverage, take an absolute covariance diagonal, or floor standard errors.
- Results contain the intercept and ordered predictor/control terms, coefficients, HC3 standard errors, t statistics, two-sided p values, confidence intervals, R2, adjusted R2, F statistic, RMSE, AIC, BIC, fitted values, and residuals.
- The complete-case design must have positive residual degrees of freedom (`n > p`, including the intercept). Rank-deficient or constant-predictor designs and undefined HC3 covariance are rejected without a partial result. HC3 is undefined when leverage or a scaled residual is nonfinite, `1 - h <= 1e-12`, the covariance is nonfinite, or a covariance diagonal is nonpositive.
- Duplicate roles, nonnumeric variables, fewer than three complete rows, case weights, resampling, and non-OLS regression settings are rejected before or during execution. Results report the exact listwise-used and omitted observation counts.
- Completed results are validated atomically before commit. The project contract recomputes coefficient identities, test statistics, intervals, fitted/residual arithmetic, and fit statistics before append, save, or reopen.
- Standalone recipes use an empty wire model. Packaged save/reopen acceptance proves that OLS does not create a phantom editable model.

## Native Workflow

- Open Data and choose `Analyze…`, then select `Ordinary Least Squares Regression` from the shared calculation catalog.
- Select the outcome, predictors, and optional controls. The dialog keeps raw values, HC3 standard errors, listwise deletion, and 95% intervals fixed and displays actionable readiness blockers.
- A successful native job opens `Results > OLS regression` with `Coefficients`, `Model fit`, and `Calculation scope`.
- CSV, HTML, reviewer-pack, Print/PDF, and XLSX exports are table-only. XLSX also contains all fitted values and residuals plus run provenance.
- `Edit Data` returns to the dataset; no `Edit Model` action or model diagram is fabricated.

## Current promotion evidence

- `validation/ols_v1_reference.py` independently expresses OLS, HC3, Student-t,
  confidence-interval, fit, fitted-value, and residual equations with NumPy and
  SciPy. It imports no QuickPLS implementation code and remains a development
  validation dependency only.
- `validation/ols_v1_factory.py` binds the current coordinated CLI and current
  source bytes. Its engine gate covers an orthogonal hand fixture, 32 frozen
  simulation replicates across four scenarios, targeted rank/constant/
  residual-degrees-of-freedom/undefined-HC3 boundaries, exact listwise counts,
  repeat determinism, predictor-order mapping, and row-order invariance.
- `crates/qpls-project` supplies the focused runner-generated append, save,
  reopen, recipe-pairing, and method-version/statistic/fit/prediction/save-time
  tamper-rejection contract for `regression_ols_v1`.
- Source-tier evidence runs only the declared OLS-facing Vitest files plus the
  TypeScript project check to prove current setup, result, and export contracts.
- `validation/ols_v1_packaged_acceptance.py` binds the exact frozen desktop and
  CLI build to focused setup, lifecycle, result, XLSX, save/reopen, visual,
  cleanup, and non-circular method-audit evidence.

The manifest remains fail-closed: stale or missing source, packaged, or audit
receipts reduce the live derived state and cannot substantiate Standard release
evidence.

## Excluded Scope

- HC0 or HC4 public claims.
- Automatic categorical/dummy encoding.
- Survey weights, clustered standard errors, GLS, mixed models, panel models, regularization, or causal claims.
- Logistic regression and PROCESS-style workflows; those are separate method contracts.
