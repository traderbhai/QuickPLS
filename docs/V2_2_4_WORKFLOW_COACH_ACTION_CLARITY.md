# QuickPLS v2.2.4 Workflow Coach Action Clarity

Status: `validated`

This frontend/product-only milestone hardens QuickPLS 2.0 workflow coach actions. Coach actions now expose stable action metadata for validation, show nearby disabled reasons, suppress duplicate secondary actions, and dispatch command events after any requested workspace navigation.

## Scope

- Add stable `data-action-label` and `data-action-disabled` metadata to coach action buttons.
- Show disabled reasons beside disabled coach actions.
- Suppress duplicate secondary actions when primary and secondary actions resolve to the same target.
- Fix the Model incomplete-diagram coach so it offers distinct Setup and Data actions.
- Keep workflow coach behavior scoped to existing frontend command events.

## Non-goals

- No statistical engine changes.
- No estimator, formula, tolerance, or result-schema changes.
- No project archive or numerical fingerprint changes.
- No new workflow routes, backend commands, or SmartPLS equivalence claims.

## Evidence

- `validation/results/v224_coach_actions_smoke.json`
- `validation/results/v224_coach_actions_audit.json`
- screenshots under `validation/results/screens/v224/coach-actions/`
- gate `v2_2_4_workflow_coach_action_clarity`

## Artifact Policy

Completed version builds must use the existing versioned artifact workflow and write installer, portable executable, and checksums under `D:\QuickPLS\target\release\artifacts` without overwriting previous versions.
