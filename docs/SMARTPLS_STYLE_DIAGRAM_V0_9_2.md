# QuickPLS v0.9.2 SmartPLS-Like Result Diagram

QuickPLS v0.9.2 adds a dedicated `smartpls_result` diagram mode for result review and publication SVG export. It is a visual presentation layer only. The validated model recipe, construct definitions, indicators, structural paths, method settings, and numerical engine payloads are unchanged.

## Visual Grammar

- Latent constructs render as filled grey ovals.
- Construct labels render below the latent oval.
- Endogenous constructs show `R²` inside the oval when a compatible completed run is selected.
- Observed indicators render as light grey rectangles outside the structural flow.
- Reflective and formative measurement arrows remain directionally correct.
- Measurement loadings or weights render on measurement arrows.
- Structural path coefficients render directly on structural arrows.
- Canvas grid, handles, delete icons, mode badges, construct metadata, minimap, and edit affordances are hidden in result mode and omitted from SVG export.

## Layout

The SmartPLS-like layout is deterministic:

- exogenous constructs are stacked on the left;
- mediators are placed in middle columns;
- final endogenous constructs are placed on the right;
- exogenous indicators are placed to the left of their construct;
- final endogenous indicators are placed to the right of their construct;
- mediator indicators are placed on the side expected to reduce structural-path interference.

The editor keeps the existing interactive SEM mode for model building. The SmartPLS-like result mode is non-editing by default: pan, zoom, and selection remain available, but accidental drag, delete, and connection edits are disabled.

## Result Rules

Numeric overlays are shown only when a completed compatible run is selected. If no run is selected, the app shows the model-only SmartPLS-like layout with a non-exported warning. If the selected run is stale or incompatible with the current model, numeric overlays are suppressed.

## Export

The Reports workspace defaults publication SVG diagrams to `smartpls_result` with grayscale styling. Controls allow precision, loadings, path coefficients, `R²`, grayscale, high contrast, and QuickPLS color variants.

SVG is the audited publication export for this milestone. Native PDF/PNG export remains outside this v0.9.2 audit unless separately implemented and validated.
