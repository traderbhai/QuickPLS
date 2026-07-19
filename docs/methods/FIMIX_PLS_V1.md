# FIMIX-PLS v1

`fimix_pls_v1` is an experimental QuickPLS v0.6 latent-class segmentation preview emitted from `AnalysisMethod::Predict` when recipe metadata contains `group_methods = "fimix"` or `fimix_classes`.

## Scope

- Supports 2 or 3 classes.
- Uses deterministic multiple starts over standardized construct-score/path-alignment features.
- Reports posterior-style class probabilities, dominant class memberships, class-specific path estimates, class R2, log-likelihood, AIC, BIC, CAIC, entropy, starts, and iteration diagnostics.
- Shares the same deterministic partitioning core as `pls_pos_v1`.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, too-small samples, singular segment fits, random-start EM qualification, and publication-ready FIMIX-PLS claims are unsupported in this preview.

## Validation

`npm run qpls:fimix:recovery` and `npm run qpls:v06:validate` write `validation/results/v06_group_methods_reference_report.json`.
