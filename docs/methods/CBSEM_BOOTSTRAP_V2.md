# CB-SEM Bootstrap v2

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported method

`cbsem_bootstrap_v2` is a seeded nonparametric case bootstrap for models already accepted by `cbsem_ml_v1`: raw-data, single-group, listwise-complete, continuous, reflective CFA or recursive ML SEM. Each replicate samples `n` rows with replacement, recomputes the sample moments, and refits the complete frozen model with the same identification and optimizer contract.

For parameter `theta`, the bootstrap standard error is the sample standard deviation of the `B_ok` usable replicate estimates and the two-sided percentile interval is `[Q_0.025(theta*), Q_0.975(theta*)]`, using the frozen linear quantile rule. Allowed requested replicate counts are 1,000 through 10,000. At least `max(1000, ceil(0.90 B))` replicates must converge to admissible results; otherwise inference is unavailable and no interval is emitted.

This contract uses Efron (1979), *Bootstrap Methods: Another Look at the Jackknife*, DOI `10.1214/aos/1176344552`, for case resampling and Bollen and Stine (1992), *Bootstrapping Goodness-of-Fit Measures in Structural Equation Models*, DOI `10.1177/0049124192021002004`, for the distinct null-transformed goodness-of-fit bootstrap. Ordinary case-bootstrap parameter intervals must never be labeled a Bollen-Stine model-fit p-value.

## Reproducibility and failures

The persisted recipe includes seed, replicate count, confidence level, quantile rule, worker-independent replicate stream, optimizer settings, and failure policy. Every nonconverged, inadmissible, singular, or non-finite replicate is counted and reason-coded. Cancellation produces no completed bootstrap result and retry with the same seed must reproduce the uninterrupted payload.

## Current preview is inadmissible

The existing `cbsem_bootstrap_v1` engine preview constructs `estimate +/- 1.96 * analytical_standard_error` without resampling or refitting. It is a normal-theory interval preview, not a bootstrap, and is permanently barred from satisfying v2 evidence or being migrated/relabelled as v2.

## Exclusions

Ordinal/WLSMV or robust estimators, missing-data FIML, multilevel or clustered resampling, parametric bootstrap, BCa/studentized intervals, indirect-effect-specific products, and automatic model modification are excluded from this standalone v2 capability. A downstream MGA or moderator workflow may reuse the frozen case-resampling, seed, quantile, and failure protocol only when its group-stratified or LMS refit extension is independently qualified under that downstream method identity; such results are not standalone `cbsem_bootstrap_v2` evidence.
