# Indicator-Level PLSpredict and CVPAT Benchmarks v2

Status: current bounded QuickPLS prediction contract. This method implements indicator-level out-of-sample prediction and benchmark assessment for a deliberately restricted, deterministic PLS-SEM scope. It does not claim complete SmartPLS feature parity or a comparison between separately saved models.

## Versioned result contract

- Top-level prediction method: `plspredict_indicator_v2`
- Primary repeated prediction block: `plspredict_repeated_kfold_indicator_v2`
- CVPAT benchmark rows: `cvpat_indicator_benchmarks_v2`
- Archive-readable predecessor: `plspredict_holdout_v1`

New results must use the v2 contract. A v1 result may be saved and reopened for archive compatibility, receives a legacy warning on load, and cannot be appended as new scientific evidence.

## Supported scope and readiness

The v2 method requires:

- a recursive path model with at least one endogenous construct;
- reflective measurement for every endogenous target construct;
- listwise complete cases across the union of model indicators;
- at least 20 complete cases after listwise filtering; and
- no case weights, generated interaction constructs, higher-order constructs, bootstrap, studentized bootstrap, or permutation resampling.

The engine enforces the complete-case threshold. A raw row count displayed before filtering is not proof that the analysis is ready. Rank-deficient linear-model benchmark fits are marked unavailable with a reason; they do not create fabricated values or abort otherwise valid PLS and indicator-average results.

## Primary repeated cross-validation plan

The primary table uses fixed 10-fold cross-validation repeated 10 times. This milestone does not expose configurable fold, repeat, or confidence settings.

For each repeat, complete rows are sorted by the binary SHA-256 digest of the UTF-8 string:

```text
plspredict_indicator_v2|<seed>|<zero-based repeat>|<source row index>
```

Sorted rows are assigned to folds by balanced round-robin position modulo 10. The emitted assignment token is `seeded_sha256_source_row_order_round_robin_10_v1`. `assignment_digest` is required for v2 and has the form `sha256:<64 lowercase hex characters>`. It hashes the complete assignment manifest, in repeat and complete-row order, using one UTF-8 line per assignment:

```text
<repeat>|<source row index>|<fold>\n
```

This scheme is seed-driven, reproducible without a language-specific random-number generator, and auditable from source-row identities. Each complete case is tested once per repeat. Aggregated prediction metrics therefore contain `complete cases × 10` test predictions per target.

## Train-only estimation and prediction

Every fold estimates preprocessing parameters, outer weights, score normalization, structural paths, indicator reconstruction coefficients, indicator-average benchmarks, and linear-model benchmarks using that fold's training rows only. Test rows are transformed with the saved training parameters.

For a root construct, held-out scores are formed from its held-out indicators using the training outer weights and score normalization. Endogenous scores are predicted recursively in topological model order from predicted predecessor scores, so observed mediator or target indicators are not used as predictors for that row.

For each reflective endogenous indicator, the training indicator is regressed on its training construct score. The fitted loading/intercept relationship maps the predicted held-out construct score back to the indicator's original data scale.

The linear-model (LM) benchmark predicts each endogenous indicator directly from all indicators of its earliest structural antecedent constructs. `predictor_scope` is exactly `earliest_antecedent_indicators`. The indicator-average (IA) benchmark predicts the target indicator with its training-fold raw mean.

## Reported prediction rows

The current payload retains construct-level RMSE/MAE rows for continuity and adds the primary indicator rows. Every indicator row identifies its construct, indicator, predictor scope, and predictor count, and reports:

- PLS-SEM observations, squared-error sum, absolute-error sum, RMSE, MAE, and MAPE;
- IA observations, squared-error sum, absolute-error sum, RMSE, MAE, and MAPE;
- LM metrics with an explicit `available` or `unavailable` status and reason; and
- `Q²_predict = 1 - SSE_PLS / SSE_IA`, unavailable when IA squared error is numerically zero.

MAPE excludes observations whose actual indicator value is zero. `mape_observations` records the denominator. When that count is zero, `absolute_percentage_error_sum` and `mape_percent` are null rather than `N/A`, zero, or an invented statistic.

## CVPAT benchmark assessment

The repeated block contains exactly two aggregate benchmark assessments: `indicator_average` and `linear_model`. These are a single fitted PLS-SEM specification assessed against benchmarks, not a comparison of saved models.

Within each fold and source observation, squared losses are averaged across all endogenous target indicators. For inference, each source observation's loss is then averaged across the 10 repeats. Consequently the paired-test sample size is the number of complete cases, not complete cases multiplied by repeats.

For each benchmark, the paired difference is:

```text
PLS-SEM loss - benchmark loss
```

A negative difference favors PLS-SEM. The assessment uses a one-sided lower-tail Student t test with `n - 1` degrees of freedom and fixed 95% confidence. The payload persists the two means, mean difference, sum of squared case-level differences, standard error, t statistic, one-sided p value, and two-sided 95% confidence interval. `preferred_model` is `pls_sem` only when the mean difference is negative and the one-sided p value is below 0.05; otherwise it is null. Zero-variance, insufficient, or unavailable benchmark cases use an explicit status and reason with no inferred significance.

## Secondary compatibility diagnostic

The top-level v2 object retains the historical deterministic complete-case modulo-4 split as a secondary diagnostic for schema and workflow continuity. Every fourth complete row is the test row. It is not the primary v2 evidence, and the payload's `primary_analysis` points to `plspredict_repeated_kfold_indicator_v2`.

## Scientific limitations

- Folds, repeats, and the 95% confidence level are fixed in this version.
- Only reflective endogenous indicator targets are supported.
- Missing data are handled only by listwise deletion in this prediction scope.
- The LM benchmark uses earliest antecedent indicators and may be unavailable under fold-level rank deficiency.
- The result does not compare alternate saved model specifications and does not implement every option or output available in SmartPLS.
- Historical v1 reduced-path paired-loss rows remain archive data only and are not relabeled as current CVPAT.

## Method sources

- SmartPLS, [PLSpredict](https://www.smartpls.com/documentation/algorithms-and-techniques/predict/) — documented indicator metrics, IA and LM benchmarks, Q²_predict, and the default 10-fold/10-repeat workflow.
- SmartPLS, [Cross-validated predictive ability test (CVPAT)](https://smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/cvpat/) — benchmark assessment direction and pooled out-of-sample comparison.
- Liengaard et al., [Prediction: Coveted, Yet Forsaken? Introducing a Cross-validated Predictive Ability Test in Partial Least Squares Path Modeling](https://epub.ub.uni-muenchen.de/95554/1/95554.pdf) — case-level loss aggregation and paired inferential test.
- ISS Analytics, [pls-predict reference implementation](https://github.com/ISS-Analytics/pls-predict) — training-only transformations, PLS indicator prediction, and LM benchmark construction used as method context.

QuickPLS independently implements and versions this bounded offline workflow; the citations define scientific semantics, not copied branding or proprietary internals.
