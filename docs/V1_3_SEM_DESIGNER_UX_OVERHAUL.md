# QuickPLS v1.3 SEM Designer UX Overhaul

QuickPLS v1.3 makes the academic SEM diagram the primary editable canvas. The same visual grammar is used for model editing, result diagrams, publication preview, and SVG export, while result and publication views remain locked to prevent accidental edits.

## Completed Scope

- Editable SEM canvas uses latent ovals, indicator rectangles, direct arrows, covariance arcs, and academic grayscale styling by default.
- Edit mode exposes restrained connection handles on latent constructs so users can create structural paths visually.
- Constructs and indicators remain draggable in edit mode; result and publication modes are view-locked.
- Indicator assignment and reassignment remain available through drag/drop and context menus.
- Context menus expose common actions: rename, duplicate, delete, measurement-mode inversion, indicator side placement, reset layout, path reverse, path delete, route changes, covariance conversion, control marking, and edge-label reset.
- Invalid self-path, duplicate structural path, and duplicate covariance attempts produce nearby canvas feedback instead of failing silently.
- Edge labels support mouse dragging plus keyboard nudging with arrow keys and reset with `Home`.
- Numeric overlays remain hidden until a compatible completed run is selected; stale or incompatible runs suppress estimates.
- SVG publication export continues to use the shared SEM visual grammar and remains the audited publication diagram format.

## Evidence

- `validation/v13_sem_designer_ux_smoke.mjs`
- `validation/v13_sem_designer_ux_audit.py`
- `validation/results/v13_sem_designer_ux_smoke.json`
- `validation/results/v13_sem_designer_ux_audit.json`
- `validation/results/screens/v13/sem-designer/`

## Boundaries

This milestone does not change the statistical engine, project recipe schema, or method-validation status. Layout metadata remains UI/project metadata and is excluded from numerical fingerprints.

Native PDF/PNG diagram export remains outside the audited v1.3 scope. SVG is the publication-grade audited export.
