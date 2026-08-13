# REGRESSION_OLS_V1

Status: validated for the bounded packaged-native scope below.

`regression_ols_v1` is QuickPLS's standalone ordinary least-squares workflow for raw numeric data. It does not create or require an SEM model.

## Validated Contract

- Metadata selects `regression_type = ols`, one numeric `regression_outcome`, one or more distinct numeric `regression_predictors`, and optional distinct numeric `regression_controls`.
- The estimator uses unstandardized observed values, an intercept, listwise deletion, HC3 heteroskedasticity-consistent standard errors, two-sided Student-t tests, and fixed 95% confidence intervals.
- Results contain the intercept and ordered predictor/control terms, coefficients, HC3 standard errors, t statistics, two-sided p values, confidence intervals, R2, adjusted R2, F statistic, RMSE, AIC, BIC, fitted values, and residuals.
- Rank-deficient designs, duplicate roles, nonnumeric variables, fewer than three complete rows, case weights, resampling, and non-OLS regression settings are rejected before or during execution.
- Completed results are validated atomically before commit. The project contract recomputes coefficient identities, test statistics, intervals, fitted/residual arithmetic, and fit statistics before append, save, or reopen.
- Standalone recipes use an empty wire model. Packaged save/reopen acceptance proves that OLS does not create a phantom editable model.

## Native Workflow

- Open Data and choose `Analyze…`, then select `Ordinary Least Squares Regression` from the shared calculation catalog.
- Select the outcome, predictors, and optional controls. The dialog keeps raw values, HC3 standard errors, listwise deletion, and 95% intervals fixed and displays actionable readiness blockers.
- A successful native job opens `Results > OLS regression` with `Coefficients`, `Model fit`, and `Calculation scope`.
- CSV, HTML, reviewer-pack, Print/PDF, and XLSX exports are table-only. XLSX also contains all fitted values and residuals plus run provenance.
- `Edit Data` returns to the dataset; no `Edit Model` action or model diagram is fabricated.

## Evidence

- `validation/ols_method_promotion_audit.py` compares QuickPLS against independently expressed NumPy HC3 equations and an R `lm` plus HC3 reference, checks rank-deficiency rejection, and consumes browser plus packaged-native evidence.
- `crates/qpls-project` includes runner-generated append, save, reopen, and tamper-rejection coverage for the exact `regression_ols_v1` envelope.
- `validation/results/v247_native_desktop_visual_acceptance.json` covers the responsive model-free setup at all three required viewports.
- `validation/results/v247_tauri_native_acceptance.json` covers a genuine 140-row native run, Results, real Windows XLSX export, explicit project save, and same-run reopen.

## Excluded Scope

- HC0 or HC4 public claims.
- Automatic categorical/dummy encoding.
- Survey weights, clustered standard errors, GLS, mixed models, panel models, regularization, or causal claims.
- Logistic regression and PROCESS-style workflows; those are separate method contracts.
