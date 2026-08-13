# Prediction-Oriented Model Selection v1

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported question

`prediction_oriented_model_selection_v1` selects one model from a finite, user-supplied candidate set of 2 through 20 PLS path models for a single frozen prediction task. Candidate generation is outside the method. All candidates must share the dataset fingerprint, prediction targets, eligible rows, preprocessing policy, loss metric, and estimator family.

Selection uses nested, seeded repeated cross-validation. Within each outer-training split, every candidate is evaluated on common inner folds. The candidate with minimum aggregated inner RMSE (or the explicitly selected MAE) is refit on the entire outer-training split and evaluated once on the untouched outer-test split. Ties within `1e-6` select, in order, the candidate with fewer free parameters and then the lexicographically smallest immutable candidate identifier.

The primary estimand is outer-test loss of the complete selection procedure, not the optimistic minimum of losses used to choose the candidate. Candidate-selection frequency and outer-fold regret relative to the best candidate in each outer fold are secondary stability diagnostics. PLS candidate comparison is grounded in Sharma et al. (2021), DOI `10.1111/deci.12329`; the separate outer loop follows the bias-control principle demonstrated by Varma and Simon (2006), *Bias in Error Estimation When Using Cross-Validation for Model Selection*, DOI `10.1186/1471-2105-7-91`.

## Multiplicity and leakage safeguards

- The candidate set, fold plan, targets, loss, tie rule, seed, and preprocessing are frozen before any outer-test outcome is inspected.
- Preprocessing, missing-data decisions, and all model selection happen inside each training split.
- All candidate failures remain in the fold ledger. A fold cannot select a candidate that failed, and a run is unavailable if fewer than 80% of planned outer folds are usable.
- The result exposes all candidates, not only the winner. No coefficient p-value is reinterpreted as selection evidence.

## Output and interpretation

Persist candidate definitions and checksums; nested fold assignments; per-candidate inner loss; chosen candidate per outer fold; outer predictions and losses; failure ledger; selection frequencies; final tie resolution; and provenance. The workflow may name a preferred candidate only for the frozen prediction task. It does not claim that the selected graph is true or that nominal post-selection inference remains valid.

## Exclusions

Automated path creation or deletion, evolutionary search, unrestricted hyperparameter tuning, a candidate set above 20, temporal or grouped cross-validation, mismatched target sets, formative prediction targets, and inference that ignores selection are excluded from v1.
