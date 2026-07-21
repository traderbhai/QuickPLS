# PLS-POS v1

Status: validated for the documented QuickPLS v1.2.2 deterministic 2-5 segment PLS-POS scope.

`pls_pos_v1` is a generalized PLS-POS payload emitted from `AnalysisMethod::Predict` when recipe metadata contains `segment_count`.

## Scope

- Supports 2 to 5 segments.
- Uses deterministic multiple starts and a minimum segment-share guard.
- Optimizes segment-specific structural residual SSE over standardized construct-score/path-alignment features.
- Reports stable memberships, segment-specific path estimates, segment R2, objective history, pooled objective, objective improvement, minimum segment share, imbalance, and maximum pairwise path separation.
- Keeps `pls_pos_bounded_v1` backward-compatible for older metadata using `pls_pos_segments = "2"`.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, too-small samples, singular segment fits, and unrestricted PLS-POS claims outside this deterministic contract are unsupported.

## Validation

`npm run qpls:pos:recovery`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:pls-pos` write the recovery and promotion artifacts. Promotion is limited to this documented scope.
