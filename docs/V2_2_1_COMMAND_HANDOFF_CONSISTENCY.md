# QuickPLS v2.2.1 Command Handoff Consistency

Status: `validated`

This frontend/product-only milestone makes the v2.2 workflow coach and global command bar use one shared command-event contract. Coach buttons, top-bar actions, and smoke tooling now agree on the same command names for run, save, open, demo, and import workflows.

## Scope

- Adds shared workspace command event types and dispatcher in `src/domain/workspaceCommands.ts`.
- Updates `src/domain/workflowCoach.ts` so coach actions use typed command events.
- Updates `src/components/WorkspaceCoach.tsx` so command actions dispatch through the shared helper.
- Updates `src/components/TopBar.tsx` so run, open, demo, save, and import command events are handled consistently.
- Keeps the existing native command implementations, file dialogs, project loading, and run job logic unchanged.

## Non-Goals

- No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes.
- No SmartPLS equivalence claim.
- No new runtime dependency.
- No broad redesign beyond command handoff consistency.

## Evidence

- `validation/v221_command_handoff_smoke.mjs`
- `validation/v221_command_handoff_audit.py`
- `validation/results/v221_command_handoff_smoke.json`
- `validation/results/v221_command_handoff_audit.json`
- screenshots under `validation/results/screens/v221/command-handoff/`
- gate `v2_2_1_command_handoff_consistency`

## Verification

```powershell
npm run build
npm run qpls:v221:commands-smoke
npm run qpls:v221:commands-audit
cargo run -p qpls-cli -- gate v2_2_1_command_handoff_consistency
```

Before building release artifacts for this completed version:

```powershell
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
npm run qpls:desktop:build-versioned
```

Versioned artifacts must be written under `D:\QuickPLS\target\release\artifacts` and include version `2.2.1`, milestone label `v2_2_1_command_handoff_consistency`, timestamp, architecture, and artifact type.
