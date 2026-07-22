# QuickPLS v1.3.2 SEM Canvas Toolbar Redesign

QuickPLS v1.3.2 reorganizes the SEM canvas toolbar into a compact desktop modeling surface. The work is limited to the diagram UI/product layer and does not change statistical engines, analysis recipes, result schemas, project numerical fingerprints, or method-validation status.

## Completed Scope

- Primary toolbar now keeps only core modeling actions: undo, redo, select, pan, construct, path, covariance, arrange, fit, validate, view, results, and help.
- Object-specific actions moved into contextual toolbars for constructs, indicators, paths, covariance edges, and multi-selection.
- Arrange, View, and Results controls are grouped into dropdowns to avoid horizontal toolbar scrolling at normal desktop sizes.
- Low-value placeholder tools for residual/error nodes, captions, and observed-indicator creation are hidden from the permanent toolbar until their workflows are fully implemented.
- View controls are functional: users can switch diagram modes, apply diagram themes, toggle the grid/minimap, and lock/unlock layout movement.
- Construct pin/unpin and indicator reassignment are available from the contextual toolbar.
- Result/publication views remain locked for editing and continue to show visible explanations.
- Toolbar smoke and audit evidence is written under `validation/results/`.

## Evidence

- `validation/v132_toolbar_smoke.mjs`
- `validation/v132_toolbar_audit.py`
- `validation/results/v132_toolbar_smoke.json`
- `validation/results/v132_toolbar_audit.json`
- `validation/results/screens/v132/toolbar/`

## Non-Scope

- No statistical method changes.
- No statistical recipe or result schema migration.
- No native PDF/PNG export changes.
- Residual/error-node and caption workflows remain hidden from the permanent toolbar until their modeling behavior is fully specified.
