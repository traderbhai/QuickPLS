# QuickPLS v1.3.3 SEM Explorer Sidebar Redesign

## Scope

v1.3.3 redesigns the left model sidebar into a SEM-native explorer for desktop use. The change is UI/product only: it does not alter statistical engines, analysis recipes, result schemas, project numerical fingerprints, or method validation claims.

## Completed Changes

- Replaced the duplicated inner Data/Model tab strip with a focused SEM Explorer.
- Added four model-building tabs: Constructs, Variables, Structure, and Issues.
- Added project status, dataset row count, data/validation shortcuts, and compact model summary.
- Added direct construct actions for focus, rename, duplicate, delete, path creation, pin/unpin, indicator-side placement, and indicator layout reset.
- Added variable assignment workflows for selected variables, grouped constructs, separate constructs, assigning to a selected construct, unassigning, and drag/drop into the canvas.
- Added a Structure tab for structural paths and covariances with focus, route, reverse, reset-label, and delete controls.
- Added an Issues tab for missing indicators, missing paths, single-item constructs, missing-data warnings, and fix actions.
- Added persistent UI-only sidebar preferences: active tab, collapsed state, and resizable width.
- Added canvas focus events so sidebar selections can center constructs and paths in the diagram.

## Validation Evidence

- `validation/v133_sem_sidebar_smoke.mjs`
- `validation/v133_sem_sidebar_audit.py`
- `validation/results/v133_sem_sidebar_smoke.json`
- `validation/results/v133_sem_sidebar_audit.json`
- `validation/results/screens/v133/sidebar/`

## Acceptance Boundary

The sidebar is now a researcher-facing SEM explorer rather than a secondary inspector. It helps users understand and edit the model inventory without crowding the canvas. Advanced property editing still belongs in the right inspector, while common actions are directly available in the explorer and canvas context controls.

## Release Artifacts

Versioned desktop builds must use version `1.3.3` and the label `v1_3_3_sem_explorer_sidebar_redesign` so installer and portable executable copies do not overwrite older user-test builds.
