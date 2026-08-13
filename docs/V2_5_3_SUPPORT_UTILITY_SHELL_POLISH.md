# QuickPLS v2.5.3 Support Utility Shell Polish

Status: validated.

This frontend-only milestone makes the launcher/support destinations feel intentional after v2.5.2 removed the primary calculation workflow band from Home, Trust Center, and Settings.

## What Changed

- Added a shared support utility bar for Home, Trust Center, and Settings.
- The support bar provides local switching among Project launcher, Evidence and scope, and Local desktop preferences.
- Data, Model, Setup, Run, Results, and Report remain focused on the primary calculation workflow and do not show the support bar.
- The primary workflow strip remains unchanged for calculation workspaces.

## Evidence

- `validation/v253_support_shell_smoke.mjs`
- `validation/v253_support_shell_audit.py`
- `validation/results/v253_support_shell_smoke.json`
- `validation/results/v253_support_shell_audit.json`
- `validation/results/screens/v253/support-shell/`

## Boundary

No statistical engines, formulas, result schemas, project archive format, validation tolerances, analysis recipes, or numerical fingerprints changed.
