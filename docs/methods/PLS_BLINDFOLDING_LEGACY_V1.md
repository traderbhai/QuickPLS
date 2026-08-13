# PLS Blindfolding Cross-Validated Redundancy v1 (Legacy)

Status: frozen **legacy/deprecated bounded** release contract. No implementation or promotion evidence is admitted by this document.

## Why this capability is legacy

Blindfolding is retained only to reproduce the established Stone-Geisser cross-validated redundancy assessment for supported historical workflows. It is not a modern out-of-sample prediction workflow: omitted observations are reconstructed through repeated refits on the same dataset, and its `Q^2` must not be presented as `Q2_predict`, PLSpredict, CVPAT, external validation, or predictive generalizability. New predictive work should use the qualified PLSpredict/CVPAT capability.

The method traces to Stone (1974), *Cross-Validatory Choice and Assessment of Statistical Predictions*, DOI `10.1111/j.2517-6161.1974.tb00994.x`, and Geisser (1974), *A Predictive Approach to the Random Effect Model*, DOI `10.1093/biomet/61.1.101`.

## Frozen bounded algorithm

`blindfolding_q2_v1` applies only to endogenous reflective constructs. It chooses omission distance 7 when valid, otherwise the first valid distance in 5, 6, 8, 9, 10, 11, 12; a valid distance is smaller than `n` and does not divide `n`. Omission is deterministic from persisted indicator and row order. Each round replaces omitted indicator cells with the retained training mean, refits the same PLS recipe, predicts the omitted standardized indicators, and accumulates prediction error sum of squares `PRESS` and observation benchmark sum of squares `SSO`.

For each applicable construct, `Q2_redundancy = 1 - PRESS/SSO`. Zero `SSO`, failed nested estimation, invalid omission distance, non-finite data, or a formative target makes the construct or run explicitly unavailable; it never becomes zero by default. Cancellation emits no completed result.

## Persistence and interpretation

Persist the legacy/deprecated flag, method version, omission distance and selection rule, row/indicator order, mean-replacement rule, `PRESS`, `SSO`, `Q2_redundancy`, unavailable reasons, nested-fit failures, and provenance. The current assessment payload has blindfolding values but no dedicated stable method identity, so it cannot satisfy this contract until versioning and strict validation are added.

## Exclusions

Exogenous or formative constructs, configurable omission patterns, missing-data imputation claims, external holdouts, time/group leakage control, `Q2_predict`, predictive model comparison, and any claim of out-of-sample generalization are excluded from this legacy v1 capability.
