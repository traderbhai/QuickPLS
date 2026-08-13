# v2.2.7 Workflow Command Feedback

Status: `validated`

This frontend/product-only milestone makes QuickPLS 2.0 workflow coach command actions acknowledge what the user requested before handing off to the existing command event path.

## Scope

- Store the last coach-driven command request as UI-only state.
- Render a compact command feedback note after enabled coach command actions are clicked.
- Keep existing view destination context from v2.2.6.
- Dispatch command actions through the existing `quickpls:*` frontend command contract.
- Verify disabled coach command actions do not write command feedback.

## Non-Goals

- No statistical engine changes.
- No formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.
- No new native command implementation.
- No artifact build until the milestone gate clears.

## Evidence

- `validation/results/v227_command_feedback_smoke.json`
- `validation/results/v227_command_feedback_audit.json`
- screenshots under `validation/results/screens/v227/command-feedback/`
- gate `v2_2_7_workflow_command_feedback`

## Artifact Convention

When a versioned desktop build is requested after this gate clears, artifacts must be preserved under `D:\QuickPLS\target\release\artifacts` with version `2.2.7`, label `v2_2_7_workflow_command_feedback`, timestamp, architecture, and artifact type.
