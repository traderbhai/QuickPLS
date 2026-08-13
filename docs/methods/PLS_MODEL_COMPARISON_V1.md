# PLS Model Comparison v1

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported question

`pls_model_comparison_v1` compares exactly two user-specified, theoretically plausible PLS path models on the same immutable dataset. Both models must use the same prediction targets, row eligibility, preprocessing, seeded outer folds, repetitions, and loss function. The workflow reports paired out-of-sample loss differences using the CVPAT construction. It may additionally report target-specific BIC and Geweke-Meese (GM) values only when both candidates meet the published Sharma et al. assumptions.

For target observation `i`, `d_i = L_A,i - L_B,i`; negative values favor model A. The primary comparison is the mean paired loss difference and its CVPAT inference. For endogenous target `t` and candidate `k`, `BIC_tk = n log(SSE_tk/n) + p_tk log(n)`. `GM_tk = SSE_tk/MSE_t,full + p_tk log(n)`, where the user-confirmed saturated full model supplies `MSE_t,full` under the same cases and target definition. BIC/GM are secondary, remain target-specific, and are unavailable if the normal homoscedastic residual approximation, parameter count, or saturated full model is not established.

The paired predictive test follows Liengaard et al. (2021), *Prediction: Coveted, Yet Forsaken? Introducing a Cross-Validated Predictive Ability Test in Partial Least Squares Path Modeling*, DOI `10.1111/deci.12445`. The target-specific criteria follow Sharma et al. (2019), *PLS-Based Model Selection: The Role of Alternative Explanations in Information Systems Research*, DOI `10.17005/1jais.00538`, and Sharma et al. (2021), *Prediction-Oriented Model Selection in Partial Least Squares Path Modeling*, DOI `10.1111/deci.12329`.

## Preconditions and blocking rules

- Both models and the comparison direction are frozen before execution.
- Dataset fingerprint, eligible rows, prediction targets, fold membership, repetition count, seed, benchmark, and loss definition must be identical.
- A target present in only one model, an in-fold preprocessing difference, a failed candidate fold, or a non-comparable BIC/GM definition blocks the affected comparison; failures are never silently dropped.
- Comparison is pairwise. A larger candidate set belongs to `prediction_oriented_model_selection_v1` and must not be decomposed into unadjusted pairwise tests.

## Output and interpretation

Persist both candidate identities and checksums; fold-level losses; usable and failed fold counts; mean and standard error of `d`; test statistic, degrees of freedom, two-sided p-value and confidence interval; BIC/GM values and deltas when applicable; direction; tie policy; warnings; and full provenance. A lower predictive loss or information criterion is conditional evidence for the frozen prediction task, not proof of causal truth, construct validity, or universal superiority.

Saved-run side-by-side tables are descriptive UI and cannot satisfy this method identity.

## Exclusions

Automatic graph search, post-hoc candidate creation, unequal targets or folds, time-series/group leakage controls, formative prediction targets, more than two candidates, and causal-model adjudication are excluded from v1.
