# QuickPLS v2.22.0 Menu Commands Dialogs Native Base

Status: validated frontend foundation.

This milestone adds a first-class desktop command inventory and completes the native menu set expected for a professional Windows research application.

Implemented scope:

- `DesktopCommandRegistry` descriptor in `src/domain/desktopCommands.ts`.
- Menu coverage for File, Edit, Data, Model, Calculate, Results, Report, View, Tools, Window, and Help.
- Existing desktop task dialogs remain wired through the command/menu surface.
- Command execution writes UI-only feedback to the permanent status bar.
- Registry slice `v2_22_0_menu_commands_dialogs_native_base`.
- Audit evidence: `validation/results/v2220_native_commands_audit.json`.

Non-goals:

- No backend or numerical changes.
- No Tauri native menu replacement; the React menu remains intentional so web preview and desktop match.
- No installer build for this intermediate milestone.

Verification:

```powershell
npm run qpls:v2220:native-commands
```
