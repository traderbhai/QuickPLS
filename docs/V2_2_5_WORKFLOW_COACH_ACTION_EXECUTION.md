# QuickPLS v2.2.5 Workflow Coach Action Execution

Status: `validated`

This frontend/product-only milestone makes QuickPLS 2.0 workflow coach actions executable and auditable. Coach buttons now expose explicit target metadata, enabled view actions are smoke-tested by click-through behavior, disabled actions remain inert with nearby reasons, and action labels use consistent researcher-facing wording.

## Scope

- Add `data-action-view` and `data-action-event` metadata to workflow coach buttons.
- Keep `data-action-label`, `data-action-disabled`, titles, and disabled reason wiring from v2.2.4.
- Verify enabled coach actions with declared view targets navigate to the expected workspace.
- Verify disabled coach actions do not trigger navigation or command handoff.
- Normalize common coach action labels to title case.

## Non-goals

- No statistical engine changes.
- No estimator, formula, tolerance, or result-schema changes.
- No project archive or numerical fingerprint changes.
- No new backend commands, workflow routes, or SmartPLS equivalence claims.

## Evidence

- `validation/results/v225_coach_execution_smoke.json`
- `validation/results/v225_coach_execution_audit.json`
- screenshots under `validation/results/screens/v225/coach-execution/`
- gate `v2_2_5_workflow_coach_action_execution`

## Artifact Policy

Completed version builds must use the existing versioned artifact workflow and write installer, portable executable, and checksums under `D:\QuickPLS\target\release\artifacts` without overwriting previous versions.
