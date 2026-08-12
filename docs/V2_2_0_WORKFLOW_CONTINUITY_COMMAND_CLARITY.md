# QuickPLS v2.2.0 Workflow Continuity And Command Clarity

Status: `validated`

This frontend/product-only milestone adds a compact workflow coach to the non-model workspaces so researchers always see the current state, the next practical action, and the blocker when calculation or reporting is not ready.

## Scope

- Adds centralized workflow guidance in `src/domain/workflowCoach.ts`.
- Adds a reusable rendered coach component in `src/components/WorkspaceCoach.tsx`.
- Shows workflow guidance after the existing workflow strip on Home, Data, Setup, Run, Results, Report, Trust Center, and Settings.
- Keeps the Model/SEM Designer branch unchanged because the v2.1.5 rendered shell baseline already covers the SEM canvas and the current goal is workflow clarity around it.
- Uses current dataset, model, method settings, run state, readiness, and desktop availability to choose deterministic next actions.

## Non-Goals

- No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes.
- No SmartPLS equivalence claim.
- No new runtime dependency.
- No artifact build until the milestone gate clears.

## Evidence

- `validation/v220_workflow_continuity_smoke.mjs`
- `validation/v220_workflow_continuity_audit.py`
- `validation/results/v220_workflow_continuity_smoke.json`
- `validation/results/v220_workflow_continuity_audit.json`
- screenshots under `validation/results/screens/v220/workflow-continuity/`
- gate `v2_2_0_workflow_continuity_command_clarity`

## Verification

```powershell
npm run build
npm run qpls:v220:workflow-smoke
npm run qpls:v220:workflow-audit
cargo run -p qpls-cli -- gate v2_2_0_workflow_continuity_command_clarity
```

Before building release artifacts for this completed version:

```powershell
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
npm run qpls:desktop:build-versioned
```

Versioned artifacts must be written under `D:\QuickPLS\target\release\artifacts` and include version `2.2.0`, milestone label `v2_2_0_workflow_continuity_command_clarity`, timestamp, architecture, and artifact type.
