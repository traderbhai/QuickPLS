# QuickPLS v1.3.1 SEM Diagram Geometry Polish

## Scope

v1.3.1 improves only the SEM diagram/editor/export layer. It does not change statistical engines, analysis recipes, method validation status, result schemas, or numerical fingerprints.

## What Changed

- Shared SEM geometry now drives canvas routing and SVG export.
- Structural paths attach to latent oval boundaries using the shortest valid border-to-border line.
- Indicator measurement paths attach to rectangle boundaries and stay outside the structural flow.
- Covariance display edges remain curved and double-headed.
- SmartPLS-like editable diagrams use larger latent ovals, larger indicator boxes, clearer labels, stronger arrowheads, and lighter measurement arrows.
- Default diagram layout theme is `smartpls_like`; publication export defaults remain grayscale unless changed by the user.
- Context menus add `Auto-place indicators`, `Tidy selected construct`, and `Tidy labels`.
- SVG export preserves current layout, indicator sides, label offsets, and numeric overlay precision while excluding edit-only chrome.

## Layout Rules

- Exogenous constructs place indicators on the left.
- Final endogenous constructs place indicators on the right.
- Mediators use top or bottom placement when that reduces conflict with structural paths.
- User-pinned indicator positions and edge-label offsets are preserved.
- `Arrange like SmartPLS` assigns columns, reorders constructs by structural neighbors, places indicators outside the causal flow, and keeps labels readable.

## Evidence

- Unit geometry tests: `src/domain/semGeometry.test.ts`
- SVG parity tests: `src/domain/publicationDiagram.test.ts`
- Visual smoke: `validation/v131_sem_geometry_smoke.mjs`
- Static audit: `validation/v131_sem_geometry_audit.py`
- Gate: `cargo run -p qpls-cli -- gate v1_3_1_sem_diagram_geometry_polish`

## Boundaries

- Native PDF/PNG export is unchanged.
- Statistical outputs and method promotion status are unchanged.
- Existing `.qpls` projects remain compatible because new behavior uses UI metadata only.
