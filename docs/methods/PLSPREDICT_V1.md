# PLSpredict V1

`plspredict_holdout_v1` is validated for the documented QuickPLS v1.2.1 bounded prediction scope: deterministic complete-case holdout, repeated deterministic k-fold prediction, construct-score LM benchmarks, RMSE, MAE, Q2 predict, and bounded CVPAT paired-loss diagnostics.

The scope excludes separate saved-model CVPAT, indicator-level PLSpredict tables, generated interactions, higher-order constructs, case weights, and unsupported data shapes.

Evidence:

- `validation/results/plspredict_holdout_reference_report.json`
- `validation/results/prediction_heterogeneity_publication_audit.json`

The implementation details remain in `PLSPREDICT_HOLDOUT_V1.md`.
