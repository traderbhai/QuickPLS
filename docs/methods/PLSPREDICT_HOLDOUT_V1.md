# PLSpredict Holdout v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported prediction scope. Broader PLSpredict variants outside this contract remain unsupported.

`plspredict_holdout_v1` is the first QuickPLS v0.6 prediction slice. It provides deterministic, leak-free complete-case holdout and bounded repeated k-fold prediction for endogenous construct scores produced by the existing PLS-PM estimator.

## Scope

- Supported method id: `predict`.
- Supported models: recursive PLS path models with at least one endogenous construct.
- Supported weighting schemes: path, factor, and PCA.
- Supported preprocessing: standardized, mean-centered, and unstandardized, with train-split preprocessing parameters applied to the test split.
- Unsupported in this slice: generated interaction constructs, higher-order construct expansion, case-weighted prediction, seeded/random repeated folds, separate saved-model CVPAT, indicator-level PLSpredict tables, MGA, MICOM, FIMIX-PLS, and PLS-POS.

## Split

QuickPLS first identifies complete rows across all model indicators. It then assigns every fourth complete row, by complete-case order, to the test split:

- training rows: complete-case index modulo 4 is 0, 1, or 2
- test rows: complete-case index modulo 4 is 3

At least 8 complete observations are required for the holdout metrics.

## Repeated K-Fold Plan

When at least 15 complete observations are available, QuickPLS also reports `repeated_kfold`:

- folds: 5
- repeats: 3
- fold assignment: `(complete_case_index * (repeat + 1) + repeat) mod 5`

Each row is tested exactly once per repeat. Fold preprocessing, weights, paths, construct-score prediction, and benchmarks are recomputed from that fold's training rows only. Metrics are aggregated from summed errors across all fold test predictions.

## Estimation

For each indicator, centering and scaling parameters are computed from the training rows only. The same training parameters are applied to test rows.

Outer weights and structural path coefficients are estimated on the training split only. Test construct scores are computed by applying the training outer weights to train-standardized test indicators. Test scores are not re-centered or re-scaled on the test split.

For each endogenous construct, QuickPLS predicts the test construct score using the training structural coefficients and the corresponding test predictor construct scores.

QuickPLS also computes an optional linear-model benchmark for the same target construct score. This benchmark regresses the training target construct score on the training indicator columns belonging to predecessor constructs, then applies that linear model to the corresponding test indicator columns. If the benchmark regression is rank-deficient, the LM benchmark fields are unavailable for that target and split.

## CVPAT

The repeated k-fold payload includes an experimental CVPAT-style paired squared-loss comparison for each endogenous target:

- `pls_vs_training_mean_benchmark`: paired loss difference is `SE_pls - SE_mean`, where `SE_mean` is the squared test construct score under the training-mean benchmark fixed at zero.
- `pls_vs_lm_benchmark`: paired loss difference is `SE_pls - SE_lm` when the LM benchmark is available.
- The reported mean loss difference is negative when PLS has lower average squared prediction error.
- The t statistic is `mean(loss_difference) / (sd(loss_difference) / sqrt(n))`.
- The two-sided p value uses a Student t distribution with `n - 1` degrees of freedom.
- If paired differences have zero variance or fewer than two observations, the t statistic and p value are unavailable with an explicit warning.

This is an early repeated-fold paired-loss diagnostic. It is not yet a full CVPAT implementation with configurable alternative hypotheses, separate saved-model comparisons, seeded fold plans, or multiple-comparison policy.

## Model-Pair CVPAT

Recipes may request one bounded configurable model-pair comparison family through metadata:

```json
{
  "metadata": {
    "cvpat_drop_paths": "z->y"
  }
}
```

Multiple paths may be separated by commas, semicolons, or new lines. QuickPLS groups dropped paths by target construct. For each target group, the full PLS prediction is compared against a reduced structural model that removes the listed direct predecessor paths for that target while retaining the same fold, preprocessing, outer weights, and construct scores. If all direct predecessors are dropped for a target, the reduced model predicts the training-mean construct score fixed at zero.

The emitted comparison id is `pls_vs_model_pair:drop_<sources>_to_<target>`. The paired loss difference is `SE_full_pls - SE_reduced_model`, so negative values favor the full PLS model.

This model-pair contract is still experimental and limited to dropping direct structural paths within the same model. It does not yet support comparing separate saved model diagrams, alternative hypotheses, seeded/random fold plans, or multiple-comparison adjustment.

## Reported Metrics

For each endogenous construct:

- `rmse_pls`: root mean squared prediction error for the PLS structural prediction
- `mae_pls`: mean absolute prediction error for the PLS structural prediction
- `rmse_benchmark`: root mean squared error of the training-mean construct-score benchmark, fixed at zero
- `mae_benchmark`: mean absolute error of the same benchmark
- `q_squared_predict`: `1 - SSE_pls / SSE_benchmark`, unavailable when benchmark SSE is numerically zero
- `rmse_lm`: optional root mean squared error from the predecessor-indicator linear-model benchmark
- `mae_lm`: optional mean absolute error from the predecessor-indicator linear-model benchmark
- `q_squared_predict_lm`: optional `1 - SSE_lm / SSE_benchmark`

## Warnings

All outputs are marked experimental. Stable publication exports must continue to reject these results unless the user explicitly includes experimental output.

## Validation Evidence

- `validation/plspredict_holdout_reference.py` generates the bounded independent reference fixture for holdout prediction, repeated k-fold prediction, LM benchmarks, Q2-predict checks, CVPAT-style paired loss checks, model-pair drop-path comparisons, and experimental warning propagation.
- `validation/results/plspredict_holdout_reference_report.json` records the current passing v1 evidence and is consumed by the v0.6 prediction and heterogeneity publication audit.
- Broader publication promotion remains limited to the documented supported scope above; unsupported prediction, grouping, and saved-model comparison cases must remain blocked or explicitly experimental.
