# QuickPLS v2.25.0 Model Workbench Integration

Status: `validated`

## Scope

This frontend-only milestone integrates the existing SEM designer into a native desktop model workbench. It preserves the current React Flow canvas logic, drag/drop behavior, model recipe serialization, statistical engines, result schemas, project format, validation tolerances, and numerical fingerprints.

## Implemented Changes

- Added a four-pane model workbench: left model explorer tree, central SEM canvas, right property-sheet inspector, and bottom model issues/output pane.
- Added `ModelIssuesPane` for analysis readiness, selected-object context, publication checks, and quick workbench actions.
- Added v2.25 workbench markers for explorer, canvas, inspector, and issues/output pane so smoke tests can verify composition.
- Added Focus Diagram styling that hides explorer, inspector, coach, and bottom pane while preserving the interactive canvas.
- Tightened inspector and workbench panel styling toward a native desktop property-sheet layout.

## Evidence

- `src/App.tsx`
- `src/components/Explorer.tsx`
- `src/components/Inspector.tsx`
- `src/components/ModelCanvas.tsx`
- `src/components/ModelIssuesPane.tsx`
- `src/styles.css`
- `validation/v2250_model_workbench_smoke.mjs`
- `validation/v2250_model_workbench_audit.py`
- `validation/results/v2250_model_workbench_smoke.json`
- `validation/results/v2250_model_workbench_audit.json`
- gate `v2_25_0_model_workbench_integration`

## Boundary

No backend, estimator, method-validation, result-value, project-archive, or numerical-fingerprint behavior changed in this milestone.
