# QuickPLS v2.2.9 Workflow Strip Context Alignment

## Scope

v2.2.9 is a frontend/product-only milestone that aligns the top workflow strip with the workflow coach feedback system. Workflow-step navigation now carries the same source and destination context as coach navigation, while current-step clicks avoid creating stale feedback.

## Changes

- Added inspectable workflow-step metadata for view, label, action, detail, and state.
- Routed workflow-strip step navigation through the existing `setView` destination-context contract.
- Preserved ordinary current-step behavior without adding redundant feedback.
- Added focused smoke and static audit coverage for workflow strip context behavior.

## Non-Goals

- No statistical engine changes.
- No formula, estimator, result schema, project archive, validation tolerance, or numerical fingerprint changes.
- No SmartPLS equivalence claim.

## Verification

```powershell
npm run build
npm run qpls:v229:workflow-strip-smoke
npm run qpls:v229:workflow-strip-audit
cargo run -p qpls-cli -- gate v2_2_9_workflow_strip_context_alignment
```

Versioned desktop artifacts are created only after the milestone gate clears.
