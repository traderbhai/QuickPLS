# CCA v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported CCA diagnostic scope. Broader CCA decision rules outside this contract remain unsupported.

`AnalysisMethod::Cca` runs the ordinary PLS estimator first, then computes composite-level correlation residual diagnostics for a recursive standardized path model. The current result reports `method_version = "cca_composite_residual_v1"` and stores a typed `cca` payload.

Implemented contract:

- construct scores come from the same PLS execution recipe as the requested model;
- observed composite correlations are Pearson correlations among construct scores;
- the reproduced correlation matrix is computed from `(I - B)^-1 Psi (I - B)^-T`;
- `B` contains the standardized structural path coefficients;
- exogenous construct covariances in `Psi` use observed composite correlations;
- endogenous residual variances in `Psi` use `1 - R2`, with off-diagonal endogenous residual covariances fixed to zero;
- the payload reports observed, reproduced, residual, absolute residual, and max absolute residual.

Unsupported outside the validated v1.2.3 descriptive scope:

- PCA weighting;
- formative composites;
- generated two-stage interaction constructs;
- bootstrap-based CCA discrepancy decisions;
- dedicated fit decision rules, result tables, and publication diagrams.

Validation evidence:

- `npm run qpls:cca:reference` writes `validation/results/cca_reference_report.json`.
- The reference script independently estimates PLS scores, rebuilds the recursive composite covariance identity, compares all observed/reproduced/residual rows, and checks the invalid PCA guard.
- Current observed max delta is `3.51e-14`.

Publication status: validated for the documented QuickPLS v1.2.3 descriptive composite residual diagnostic scope. Bootstrap decisions, discrepancy tests, and broader CCA decision rules remain unsupported.
