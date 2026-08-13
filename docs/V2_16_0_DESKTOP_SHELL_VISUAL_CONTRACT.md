# QuickPLS v2.16.0 Desktop Shell Visual Contract

## Scope

This milestone establishes the QuickPLS 2.0 desktop shell foundation from the selected mockup. It is frontend-only and does not change statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## Completed Changes

- Added a React-rendered desktop menu bar with `File`, `Edit`, `Data`, `Model`, `Calculate`, `Results`, `Report`, `View`, and `Help`.
- Added transient UI state for active desktop menu and active desktop dialog.
- Added desktop-style dialogs for new project, open project guidance, import data, export options, calculation setup, method scope/trust evidence, settings, and help/shortcuts.
- Tightened the global shell to a native-style title strip, menu bar, command strip, main workspace, and status bar.
- Preserved existing SEM canvas logic and drag/drop behavior.
- Kept every shell command wired to existing QuickPLS commands, store actions, or workspace navigation.

## Non-Goals

- No screen-specific Home/Data/Setup/Run/Results/Report redesign beyond the shared shell.
- No native Tauri menu implementation.
- No desktop artifact build for this intermediate milestone.
- No SmartPLS equivalence claim.

## Verification

Run:

```powershell
npm run build
npm run qpls:v2160:desktop-shell-smoke
npm run qpls:v2160:desktop-shell-audit
cargo run -p qpls-cli -- gate v2_16_0_desktop_shell_visual_contract
```

## Next Milestone

`v2_17_0_home_data_setup_mockup_alignment` should align Home, Data, and Setup screens to the selected mockup using this shell foundation.
