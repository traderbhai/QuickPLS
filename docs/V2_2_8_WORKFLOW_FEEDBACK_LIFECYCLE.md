# QuickPLS v2.2.8 Workflow Feedback Lifecycle

## Scope

v2.2.8 is a frontend/product-only milestone for workflow coach feedback. It makes destination and command feedback dismissible and prevents stale feedback from lingering after ordinary navigation, dataset replacement, project reset, or project load.

## Changes

- Added a shared `clearWorkflowFeedback` UI action in the workspace store.
- Added a compact `Dismiss` control to workflow coach destination and command feedback.
- Cleared workflow feedback on cross-workspace navigation when no new coach context is supplied.
- Cleared workflow feedback when data or project state is replaced.
- Added focused smoke and static audit coverage for the feedback lifecycle.

## Non-Goals

- No statistical engine changes.
- No formula, estimator, result schema, project archive, validation tolerance, or numerical fingerprint changes.
- No SmartPLS equivalence claim.

## Verification

```powershell
npm run build
npm run qpls:v228:feedback-lifecycle-smoke
npm run qpls:v228:feedback-lifecycle-audit
cargo run -p qpls-cli -- gate v2_2_8_workflow_feedback_lifecycle
```

Versioned desktop artifacts are created only after the milestone gate clears.
