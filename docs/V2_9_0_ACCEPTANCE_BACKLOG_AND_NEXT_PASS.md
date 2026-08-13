# QuickPLS v2.9.0 Acceptance Backlog And Next Pass

## Scope

This milestone turns the current QuickPLS 2.x release handoff and visual issue evidence into a grouped acceptance backlog for the next UI pass.

It does not redesign a screen by itself. It defines the next grouped workstreams, decisions, acceptance evidence, and boundaries so the following implementation does not drift into small unrelated fixes.

## Complete

- Captures current Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings screenshots at `1440x900` and `1280x800`.
- Generates `validation/results/v290_acceptance_backlog.json`.
- Splits upcoming work into `do_next`, `defer`, and `do_not_do` streams.
- Keeps Results/Report refinement, method applicability follow-up, and real-dataset review protocol as the next grouped work.
- Defers SEM Designer core changes unless the user explicitly requests them.
- Blocks micro-fix drift by requiring every next milestone to define target screens, smoke/audit, and final gate.

## Evidence

- `validation/results/v290_acceptance_backlog_smoke.json`
- `validation/results/v290_acceptance_backlog_audit.json`
- `validation/results/v290_acceptance_backlog.json`
- screenshots under `validation/results/screens/v290/acceptance-backlog/`

## Boundary

This is a frontend/product-only governance milestone. No estimator, method validation, result schema, project archive behavior, or numerical fingerprint changes are part of this work.

## Next Pass

The next implementation should pick one `do_next` stream from the generated backlog and complete it as a release-sized UI milestone with its own targeted smoke/audit and gate.
