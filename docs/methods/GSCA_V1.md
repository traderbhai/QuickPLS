# GSCA_V1

Status: experimental v0.8 preview.

`gsca_v1` is a bounded generalized structured component analysis preview for raw-data component models.

## Contract

- The v0.8 preview uses existing component-score and path-estimation infrastructure to emit a GSCA-style payload for reflective/formative blocks and recursive paths.
- Output includes component weights, loadings, scores through the shared result envelope, paths, R2, FIT, AFIT, GFI-style diagnostics, bootstrap-interval placeholders, convergence metadata, warnings, and `method_version = gsca_v1`.
- Formative and reflective block semantics follow the existing QuickPLS model specification.

## Unsupported In v0.8

- Interactions.
- Higher-order constructs.
- Case weights.
- Covariance/correlation input.
- Nonrecursive models.
- Publication-ready ALS parity claims.

## Validation

`npm run qpls:gsca:reference` verifies bounded payload shape, finite path estimates, and method-version surfacing. Full GSCA validation remains a later promotion task requiring an independent ALS fixture, simulated recovery, bootstrap qualification, and second-source comparison.
