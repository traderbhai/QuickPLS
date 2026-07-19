# QuickPLS v0.9.3 Professional SEM Designer

QuickPLS v0.9.3 makes the editable SEM canvas, result diagram, publication preview, and SVG export use the same academic SEM visual grammar. The change is a product and project-layout milestone only; it does not change the validated statistical recipe or numerical engine.

## What Changed

- The default `sem` canvas uses the same clean academic style as the result/publication diagram: latent ovals, indicator rectangles, direct arrows, optional covariance arcs, and result overlays after a compatible run is selected.
- Indicator positions are now persistent UI layout metadata. Moving an indicator on the canvas does not change the engine recipe unless the indicator is reassigned to another construct.
- Result and publication modes are locked to prevent accidental edits. Edit mode remains draggable and supports construct, indicator, path, covariance, pan, residual placeholder, and caption placeholder tools.
- Right-click canvas actions cover common SmartPLS/AMOS-style workflows: rename, invert reflective/formative, align/reset indicators, route paths, mark controls, duplicate, and delete.
- Publication SVG export can use the current canvas layout by default, or a tidy publication layout when requested.

## Compatibility

Existing `.qpls` projects load without migration failure. Missing diagram-layout fields are computed from the existing construct positions, indicators, and paths. The optional layout metadata is UI/project state and is not part of the statistical model recipe.

## Limits

Residual/error and caption tools are present as designer placeholders in this milestone. They do not create new validated CB-SEM recipe semantics until those semantics are separately implemented and audited.
