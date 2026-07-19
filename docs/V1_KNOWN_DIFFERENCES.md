# QuickPLS 1.0.0 Known Differences Register

This file records documented differences for the v1.0.0 supported scope. A difference is acceptable only when it is mapped to an audit artifact or method specification.

No unexplained deterministic numerical discrepancies above `1e-6` are accepted for v1.0.0 supported deterministic outputs.

## General

- QuickPLS implements published methods independently. It does not reverse-engineer SmartPLS or attempt to reproduce undocumented SmartPLS behavior.
- QuickPLS project files are not SmartPLS project files, and SmartPLS project import is not supported.
- Floating-point differences may occur where reference engines use different optimization, normalization, or sign-orientation conventions. Deterministic v1.0.0 coefficients must remain within documented tolerances or be listed here.

## PLS-PM

- Component sign orientation is deterministic in QuickPLS. Some reference engines may flip signs without changing equivalent model interpretation.
- Standardization is explicit in the recipe and defaults to the documented standardized PLS workflow for comparable tables.
- Single-item constructs are handled explicitly, with reliability and HTMT limitations recorded in diagnostics where applicable.
- Weight normalization conventions differ across reference packages; validation artifacts document equivalent settings.

## Assessment And Inference

- QuickPLS reports original signed HTMT and Ringle-style HTMT+ separately.
- cSEM `.absolute=TRUE` is documented as non-equivalent to QuickPLS HTMT+ for mixed-sign cross-block correlations.
- Stochastic inference uses indexed deterministic streams. Result ordering is designed to be worker-count invariant.

## CB-SEM And Extended Methods

- CB-SEM v1.0.0 support is bounded to documented raw-data reflective ML cases. Ordinal/polychoric/WLSMV/FIML claims are post-v1.
- GSCA, NCA, regression/PROCESS, and prediction/heterogeneity outputs are stable only for the documented bounded shapes covered by v1 audit evidence.

## Exports And Diagrams

- SVG is the audited publication diagram export.
- Browser print-to-PDF is documented as a user workflow, not a native audited PDF renderer.
- Native CLI PDF and PNG export are post-v1 unless separately implemented and audited.
