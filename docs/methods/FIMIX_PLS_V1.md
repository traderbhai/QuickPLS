# FIMIX-PLS v1

Status: validated for the documented QuickPLS v1.2.2 bounded deterministic 2-3 class FIMIX-PLS scope.

`fimix_pls_v1` is a latent-class segmentation payload emitted from `AnalysisMethod::Predict` when recipe metadata contains `group_methods = "fimix"` or `fimix_classes`.

## Scope

- Supports 2 or 3 classes.
- Uses deterministic multiple starts over standardized construct-score/path-alignment features.
- Reports posterior-style class probabilities, dominant class memberships, class-specific path estimates, class R2, log-likelihood, AIC, BIC, CAIC, entropy, starts, and iteration diagnostics.
- Shares the same deterministic partitioning core as `pls_pos_v1`.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, too-small samples, singular segment fits, random-start EM qualification, and unrestricted FIMIX-PLS claims outside this deterministic contract are unsupported.

## Validation

`npm run qpls:fimix:recovery`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:fimix-pls` write the recovery and promotion artifacts. Known difference: QuickPLS promotes only this bounded deterministic score-space segmentation scope, not blanket full EM/FIMIX parity.
