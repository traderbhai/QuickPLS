# QuickPLS v2.3.0 Global Command Bar Readiness

Status: `validated`

This frontend/product-only milestone aligns the global command bar with the workflow readiness system. The top Run control now exposes inspectable readiness metadata, shows the exact disabled-run reason next to the button, and routes blocker actions through the same destination-context contract used by workflow coach and workflow strip navigation.

## Scope

- Adds `data-command-bar-state`, `data-run-state`, `data-run-method`, `data-run-blocker-id`, `data-run-blocker-view`, `data-run-blocker-action`, and `data-run-disabled-reason` metadata to the command bar run cluster.
- Replaces the generic compact blocker text with a nearby `Run disabled` chip that carries the exact readiness detail in `aria-label` and `title`.
- Routes blocker-chip clicks through `setView` with `from`, `to`, `actionLabel`, and `coachId: "top-command-bar"` destination context.
- Keeps the top-bar method selector conservative by retaining the existing recommended/available method list plus `More methods in Setup`.

## Evidence

- `validation/results/v230_command_bar_smoke.json`
- `validation/results/v230_command_bar_audit.json`
- screenshots under `validation/results/screens/v230/command-bar/`
- gate `v2_3_0_global_command_bar_readiness`

## Boundary

No statistical engine changes.
No result schema changes.
No analysis recipe changes.
No project archive changes.
No numerical fingerprint changes.
