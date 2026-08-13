# QuickPLS v2.2.2 Workflow Step Clarity

## Scope

v2.2.2 makes the QuickPLS 2.0 workflow strip explicit and actionable. The strip now shows state, short action text, and detailed hover/accessibility reasons for Data, Model, Setup, Run, Results, and Report.

## What Changed

- Added `src/domain/workflowProgress.ts` as a frontend-only state resolver for workflow steps.
- Updated `WorkflowStrip` to show `complete`, `current`, `next`, `blocked`, and `ready` states with accessible labels and details.
- Reworked workflow strip styling into a compact six-step desktop grid that avoids horizontal scrolling at normal desktop widths.
- Added targeted smoke and static audit scripts for workflow step state, overflow, version metadata, docs, and registry coverage.

## Non-Goals

- No statistical engine changes.
- No formula, estimator, validation tolerance, result schema, project archive, or numerical fingerprint changes.
- No installer build for intermediate patches; artifacts are generated only after completed milestone gates clear.

## Verification

```powershell
npm run build
npm run qpls:v222:workflow-step-smoke
npm run qpls:v222:workflow-step-audit
cargo run -p qpls-cli -- gate v2_2_2_workflow_step_clarity
```

Before creating versioned artifacts:

```powershell
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
npm run qpls:desktop:build-versioned
```
