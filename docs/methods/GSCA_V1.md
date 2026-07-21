# GSCA_V1

Status: validated for the documented QuickPLS v1.2.4 bounded deterministic GSCA component-model scope.

`gsca_v1` is a bounded generalized structured component analysis implementation for raw-data component models.

## Contract

- The promoted v1.2.4 scope uses existing component-score and path-estimation infrastructure to emit a GSCA-style payload for reflective/formative blocks and recursive paths.
- Output includes component weights, loadings, scores through the shared result envelope, paths, R2, FIT, AFIT, GFI-style diagnostics, bootstrap-interval placeholders, convergence metadata, warnings, and `method_version = gsca_v1`.
- Formative and reflective block semantics follow the existing QuickPLS model specification.

## Unsupported Outside v1.2.4

- Interactions.
- Higher-order constructs.
- Case weights.
- Covariance/correlation input.
- Nonrecursive models.
- Unrestricted GSCA variants outside the documented QuickPLS deterministic component-model scope.

## Validation

`npm run qpls:gsca:reference` verifies bounded payload shape, finite path estimates, and method-version surfacing. `validation/gsca_method_promotion_audit.py` promotes only this bounded deterministic scope; GSCA bootstrap and unrestricted ALS parity claims remain unsupported unless separately audited.
