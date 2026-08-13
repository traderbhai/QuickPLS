# QuickPLS v2.27.0 Calculation Run Monitor

Status: validated.

This frontend/product milestone turns the Run workspace into a native-style calculation monitor. It does not change statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## Completed

- Added a procedure checklist for validation, recipe preparation, engine execution, result commit, and results handoff.
- Added a central progress and run-log panel driven by the shared `runMonitor` UI state.
- Added an immutable run-settings summary with method, scope status, seed, worker count, data fingerprint, recipe fingerprint, bootstrap, permutation, produced outputs, and unavailable outputs.
- Wired Run workspace launch/cancel controls to the same top-bar native job events.
- Added shared run monitor state so queued, validating, running, cancelling, completed, failed, cancelled, and blocked states can be rendered consistently.
- Added deterministic smoke fixtures for run-monitor states without launching the numerical engine in browser preview.

## Evidence

- `src/components/RunWorkspace.tsx`
- `src/components/TopBar.tsx`
- `src/store.ts`
- `src/types.ts`
- `validation/v2270_run_monitor_smoke.mjs`
- `validation/v2270_run_monitor_audit.py`
- `validation/results/v2270_run_monitor_smoke.json`
- `validation/results/v2270_run_monitor_audit.json`

## Boundary

This is UI-only calculation monitoring. It preserves existing native Tauri job commands and analysis output behavior.
