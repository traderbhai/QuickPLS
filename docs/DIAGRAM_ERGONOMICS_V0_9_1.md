# QuickPLS v0.9.1 Diagram Ergonomics

QuickPLS v0.9.1 changes the model editor from compact construct cards to a SEM-native visual layer. The statistical recipe remains unchanged: constructs, indicators, paths, controls, interactions, and higher-order metadata are still the source of truth for estimation.

## Supported Editor Behavior

- New projects default to `sem` diagram mode.
- Latent constructs render as ovals.
- Indicators render as observed-variable rectangles.
- Reflective and formative measurement links are visually distinct.
- Structural paths remain engine-facing model paths.
- Covariance arcs are visual annotations and are excluded from PLS recipe paths.
- Completed compatible runs can be selected for numeric overlays.
- Incompatible or stale runs suppress numeric overlays and show a warning.
- Reports export the same SEM-native publication SVG preview.

## Compatibility

Existing `.qpls` project workspaces load without migration failure. Missing diagram fields default to:

- `diagramMode = sem`
- overlay mode `model`
- publication diagram mode `publication`
- color SVG output with validation/provenance footer enabled

## Limits

This milestone improves diagram ergonomics and publication SVG output. It does not change validated numerical estimators, add new CB-SEM covariance recipe semantics, or promote unsupported model shapes.
