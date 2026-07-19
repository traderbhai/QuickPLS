# PLS-POS v1

`pls_pos_v1` is an experimental QuickPLS v0.6 generalized PLS-POS preview emitted from `AnalysisMethod::Predict` when recipe metadata contains `segment_count`.

## Scope

- Supports 2 to 5 segments.
- Uses deterministic multiple starts and a minimum segment-share guard.
- Optimizes segment-specific structural residual SSE over standardized construct-score/path-alignment features.
- Reports stable memberships, segment-specific path estimates, segment R2, objective history, pooled objective, objective improvement, minimum segment share, imbalance, and maximum pairwise path separation.
- Keeps `pls_pos_bounded_v1` backward-compatible for older metadata using `pls_pos_segments = "2"`.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, too-small samples, singular segment fits, and publication-ready PLS-POS claims are unsupported in this preview.

## Validation

`npm run qpls:pos:recovery` and `npm run qpls:v06:validate` write `validation/results/v06_group_methods_reference_report.json`.
