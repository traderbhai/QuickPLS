# QuickPLS Known Differences Register

Status: initial publication-audit register.

This file records documented convention differences between QuickPLS and reference engines. A difference listed here is not automatically acceptable for publication-ready output; each method promotion audit must decide whether the difference is justified, blocked, or unsupported.

## PLS-PM v1

- Sign orientation: QuickPLS uses deterministic orientation rules so repeated runs and worker counts serialize stable loadings, weights, and scores. Reference engines may flip component signs without changing model fit.
- Standardization: QuickPLS records preprocessing mode in the recipe and defaults to standardized PLS for comparable publication tables.
- Single-item constructs: QuickPLS treats single-item reflective measures explicitly and records reliability/HTMT limitations in assessment output.
- Weighting schemes: path, factor, and PCA weighting are implemented as documented method variants; reference packages differ in some weight normalization conventions.
- Bootstrap references: python-plspm weight outputs are excluded from some bootstrap comparisons because its normalization convention is not equivalent to QuickPLS.
- Stable v0.3 export: default CLI export is estimator-only and excludes experimental assessment/resampling tables unless `--include-experimental` is explicitly used.

## HTMT / HTMT+

- QuickPLS records original signed HTMT and Ringle-style HTMT+ separately.
- cSEM `.absolute=TRUE` is documented as non-equivalent to QuickPLS HTMT+ for mixed-sign cross-block correlations.

## CB-SEM And Extended Methods

- v0.7 and v0.8 outputs remain experimental until their later publication promotion audits close.
