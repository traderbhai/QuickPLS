# QuickPLS v2.2.3 Model Workflow Context

Status: `validated`

This frontend/product-only milestone brings the Model workspace into the same QuickPLS 2.0 workflow shell used by the other workspaces. The SEM Designer now shows the workflow strip and a state-aware coach above the existing Explorer, canvas, and Inspector so users can understand where model building sits between Data and Setup.

## Scope

- Render the workflow strip above the Model workspace.
- Render a Model-specific workflow coach above the SEM Designer.
- Keep Explorer, ModelCanvas, and Inspector as the existing designer surfaces.
- Add model coach states for missing data, incomplete diagram, setup handoff, and run-ready conditions.
- Keep the designer editable and preserve existing canvas tools.

## Non-goals

- No statistical engine changes.
- No estimator, formula, tolerance, or result-schema changes.
- No project archive or numerical fingerprint changes.
- No SEM canvas geometry, node, edge, or layout-engine rewrite.
- No new SmartPLS equivalence claim.

## Evidence

- `validation/results/v223_model_workflow_smoke.json`
- `validation/results/v223_model_workflow_audit.json`
- screenshots under `validation/results/screens/v223/model-workflow/`
- gate `v2_2_3_model_workflow_context`

## Artifact Policy

Completed version builds must use the existing versioned artifact workflow and write installer, portable executable, and checksums under `D:\QuickPLS\target\release\artifacts` without overwriting previous versions.
