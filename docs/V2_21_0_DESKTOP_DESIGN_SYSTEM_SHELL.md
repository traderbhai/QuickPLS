# QuickPLS v2.21.0 Desktop Design System And Shell

Status: validated frontend foundation.

This milestone starts the native desktop redesign program. It keeps the existing SEM designer and all statistical behavior unchanged, but moves the shared shell further away from a web-dashboard presentation.

Implemented scope:

- Native shell markers on the title/menu/command strip.
- Denser neutral Windows-style menu and command strip styling.
- UI-only desktop command feedback in the permanent status bar.
- Registry slice `v2_21_0_desktop_design_system_shell`.
- Audit evidence: `validation/results/v2210_native_shell_audit.json`.

Non-goals:

- No estimator, formula, project archive, result schema, or validation tolerance changes.
- No installer build for this intermediate milestone.
- No claim that v2.23-v2.35 work is complete.

Verification:

```powershell
npm run qpls:v2210:native-shell
```
