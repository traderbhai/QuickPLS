# QuickPLS v2.5.2 Launcher And Support Shell Separation

Status: validated.

This frontend-only milestone separates launcher/support destinations from the primary calculation workflow band. Home, Trust Center, and Settings now open as support shells without the Data -> Model -> Setup -> Run -> Results -> Report workflow strip or coach. Data, Setup, Run, Results, Report, and the dedicated Model workspace keep the primary workflow guidance.

## What Changed

- `src/App.tsx` now renders `WorkflowStrip` and `WorkspaceCoach` only for primary workflow page views.
- Home, Trust Center, and Settings receive a `support-shell` host class and no primary workflow band.
- Data, Setup, Run, Results, and Report receive `has-workflow-band`.
- Model keeps its existing dedicated `model-workflow-band`.
- Added rendered smoke and static audit coverage for launcher/support shell behavior.

## Evidence

- `validation/v252_launcher_support_shell_smoke.mjs`
- `validation/v252_launcher_support_shell_audit.py`
- `validation/results/v252_launcher_support_shell_smoke.json`
- `validation/results/v252_launcher_support_shell_audit.json`
- `validation/results/screens/v252/launcher-support-shell/`

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, analysis recipes, or numerical fingerprints changed.
