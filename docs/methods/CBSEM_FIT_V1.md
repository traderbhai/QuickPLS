# CB-SEM Fit v1

`cbsem_fit_v1` is validated for the documented QuickPLS v1.2.4 raw-data single-group reflective CFA/SEM ML scope attached to `cfa_ml_v1` and `cbsem_ml_v1`.

## Scope

- Reports chi-square, degrees of freedom, p value, CFI, TLI, RMSEA, RMSEA CI placeholders where available, SRMR, AIC, BIC, baseline chi-square, and baseline degrees of freedom.
- Uses the ML discrepancy between the sample covariance matrix and the v0.7.1 optimized model-implied covariance for supported single-group raw-data CFA/SEM models.
- Computes a diagonal baseline/null-model comparison for incremental fit diagnostics.

## Unsupported

Robust/scaled fit indices, estimator-specific corrections, missing-data FIML, multilevel/complex-sample corrections, and fit claims outside the documented v1.2.4 estimator scope remain unsupported.

## Validation

`npm run qpls:cbsem:fit-reference` checks finite fields, expected bounds, and stable serialized payload shape through `validation/results/cbsem_v07_reference_report.json`. `npm run qpls:cbsem:lavaan-validate` checks supported single-group fit-index parity against lavaan.
