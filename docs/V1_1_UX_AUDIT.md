# QuickPLS v1.1 UX Audit

This document tracks expert-user observations and implementation notes for the v1.1 frontend, usability, and workflow polish program. The target is a researcher-grade Windows desktop experience that is faster and clearer than comparable SEM tools while preserving QuickPLS' validated numerical scope.

## Current Direction

- Use one professional academic SEM visual grammar across editing, result viewing, publication preview, and SVG export.
- Keep the app workflow explicit: Data -> Model -> Validate -> Run -> Results -> Report.
- Prevent premature estimates: loadings, paths, and R2 appear only after a compatible completed run is selected.
- Make readiness visible before analysis so users understand why a run is unavailable or risky.
- Prefer direct manipulation in the canvas over inspector-only workflows.
- Optimize for desktop use. Mobile responsiveness is useful for browser smoke checks, but it is not a product priority for v1.1.

## Implemented v1.1 UX Improvements

- Added a workflow strip for Data, Model, Validate, Run, Results, Groups, and Report.
- Added an analysis readiness panel with blockers and warnings surfaced in Run, toolbar, status bar, catalog, and history contexts.
- Added a dedicated Run workspace so analysis execution is not hidden inside method selection.
- Updated stale version/status language to QuickPLS v1.0 stable scope.
- Improved SmartPLS-like SEM graph layout spacing, structural edge handles, measurement edge handles, and label backgrounds.
- Added neighbor-aware auto-layout ordering for the general arrange command and SmartPLS-style tidy layout so adjacent columns are ordered by parent/child structure instead of arbitrary starting y-position.
- Added live desktop drag feedback on the SEM canvas: snap coordinates, alignment guide lines, and construct-relative alignment labels while moving constructs.
- Strengthened active selected/dragging states for SmartPLS-style latent constructs and indicator boxes.
- Added persistent edge-label offsets in UI layout metadata, canvas rendering, and SVG export; right-click edge actions can nudge labels up/down/left/right or reset them.
- Added direct pointer-dragging for visible SEM edge labels with one undo checkpoint per drag and zoom-aware offset calculation.
- Added deterministic automatic label offsets when graph labels would otherwise share the same midpoint.
- Added construct-level context-menu actions to place all indicators left, right, top, or bottom without changing the engine-facing model recipe.
- Verified R-squared canvas rendering no longer shows mojibake in browser smoke state.
- Added visible drag/drop cues when dataset variables are dragged toward the SEM canvas.
- Made variable drops explicit: dropping on a latent construct assigns indicators, while dropping on empty canvas creates a construct.
- Added drop placement protection so new constructs are nudged into open canvas space instead of overlapping existing constructs.
- Strengthened SmartPLS-style selected and drop-target states for latent constructs and indicators.
- Added accessible labels to icon-only SEM canvas toolbar controls.
- Centralized analysis method selectability so non-standalone workflows appear as "Configured elsewhere" instead of silently behaving like runnable methods.
- Added consistent status labels in analysis dropdowns and method table rows: Validated, Experimental, Configured elsewhere, and Unsupported.
- Split the top command-bar method selector from the method-status badge so long method names stay readable while status remains visible.
- Improved desktop table readability with larger text, sticky headers, sticky first columns for wide tables, row striping, hover rows, tabular numeric alignment, and table captions where generated.
- Improved report workspace preview hierarchy for publication review.
- Added visible disabled-action guidance in the command bar, Report export area, Data metadata panel, and locked result/publication diagram mode.
- Moved Report export-gate guidance above the publication preview so users see why result exports are disabled without scrolling past the WYSIWYG diagram.
- Added focus-visible styling and active navigation state.
- Added keyboard-focusable, named scroll regions for the Data preview, Results summary/quality blocks, bootstrap/permutation tables, Groups tables, and Report comparison/export tables.
- Added visible focus treatment for table scroll regions so keyboard users can see which large output table is active.
- Reworked the Report R-squared checkbox label as `R<sup>2</sup>` to avoid Unicode mojibake in packaged/browser builds.
- Added Vitest accessibility contract coverage so large table focus regions and the Report R-squared label cannot silently regress.
- Added Enter-to-edit keyboard behavior on the SEM canvas: selected constructs open the construct-name prompt and selected paths open the path-label prompt, while text inputs remain protected from global shortcuts.
- Added shortcut contract coverage for Undo, Redo, Duplicate, Delete, Escape, Enter, Path, Covariance, Select, and Fit View.
- Added a repeatable desktop-width v1.1 smoke harness that captures Data, Model, Validate, Run, Results, and Report at 1440x900.
- Added completed-run smoke coverage for Results tables, model diagram result overlays, and Report WYSIWYG SVG preview using a guarded fixture that exercises the same `AnalysisRun` overlay contract as native engine output.
- Rebuilt the Tauri release executable and NSIS installer after the v1.1 frontend changes.
- Added a v1.1 native desktop smoke audit that verifies the release executable is newer than the current frontend bundle, launches without a dev server, and links to the completed-run v1.1 UX smoke evidence.
- Made readiness blockers actionable: visible design/preview data without a desktop fingerprint now explains the reproducibility issue and points to Data, while method-setting blockers point to Validate/setup.
- Added inline readiness action buttons to Validate, Run, and empty Results states so users can move directly to the workspace needed to clear a blocker.
- Added a persistent canvas overlay status strip that clearly distinguishes model-only diagrams, active result overlays, and blocked stale/incompatible overlays.
- Added canvas-level next-action guidance tied to analysis readiness so the Model workspace can point directly to Run, Validate, or Data instead of leaving users at a static diagram.
- Added a persistent desktop footer readiness checklist with compact Data/Runtime, Constructs, Indicators, Model structure, Sample size, and Method status pills visible across workspaces.
- Strengthened selected path-label feedback so selected SEM paths are easier to identify while editing dense diagrams.

## v1.1 Completion Closure

No remaining v1.1 desktop release blockers are open. The v1.1 release scope is the native Windows desktop workflow, with browser-based Playwright used only for deterministic desktop-viewport visual evidence. Mobile is non-gating.

- Native workflow coverage is closed by `validation/results/v11_native_workflow_smoke.json`, which runs the desktop crate workflow smoke for import-style dataset ingestion, PLS run, project save/reopen, layout persistence, and XLSX export.
- Report parity is closed by `validation/results/v11_report_export_parity.json`, which compares the WYSIWYG Report SVG preview against the exported SVG labels and checks that edit chrome is absent.
- Keyboard workflow coverage is closed by `validation/results/v11_keyboard_workflow_smoke.json`, which verifies named, focusable Results and Report table regions.
- Desktop visual coverage is closed by `validation/results/v11_desktop_ux_smoke.json` and `validation/results/screens/v11/`, which capture Data, Model, Validate, Run, Results, Report, completed-result overlays, and publication preview at `1440x900`.
- Release artifact freshness is closed by `validation/results/v11_native_desktop_smoke.json`, which verifies the rebuilt release executable and NSIS installer are newer than the frontend bundle and launch without a dev server.

## Post-v1.1 Follow-Up Candidates

1. Broader visual regression fixtures for very large models beyond the current smoke shapes.
2. Richer native OS-level pointer automation if a stable Windows GUI automation harness is adopted.
3. Additional method-family-specific settings wizards for experimental analyses.
4. Native PDF/PNG export if those formats are separately implemented and audited.

## Latest Desktop Evidence

- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-21\01_model_canvas_desktop.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-21\state.json`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-22\01_drag_alignment_guide.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-22\02_canvas_after_drag.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-22\state.json`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-23\01_command_bar_disabled_run_reason.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-24\01_report_top_export_reason.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-24\state.json`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-25\01_custom_edge_canvas_smoke.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-25\state.json`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-26\01_demo_canvas_edge_label_check.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-26\state.json`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-30\01_data_grid_focus.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-30\02_report_r2_label.png`
- `C:\Users\MOHD~1.NAV\AppData\Local\Temp\quickpls-v11-ux-after-30\state.json`
- `D:\QuickPLS\validation\results\v11_desktop_ux_smoke.json`
- `D:\QuickPLS\validation\results\screens\v11\v11_01_data_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_02_model_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_03_validate_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_04_run_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_05_results_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_06_report_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_07_results_completed_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_08_model_completed_overlay_1440x900.png`
- `D:\QuickPLS\validation\results\screens\v11\v11_09_report_completed_1440x900.png`
- `D:\QuickPLS\validation\results\v11_native_desktop_smoke.json`
- `D:\QuickPLS\validation\results\v11_native_workflow_smoke.json`
- `D:\QuickPLS\validation\results\v11_report_export_parity.json`
- `D:\QuickPLS\validation\results\v11_keyboard_workflow_smoke.json`
- `D:\QuickPLS\validation\results\v11_desktop_ux_audit.json`
- `D:\QuickPLS\validation\results\v11\report-parity\report-parity.png`
- `D:\QuickPLS\validation\results\v11\report-parity\quickpls-publication-diagram.svg`
- `D:\QuickPLS\validation\results\v11\keyboard-smoke\results-keyboard.png`
- `D:\QuickPLS\validation\results\v11\keyboard-smoke\report-keyboard.png`
- `D:\QuickPLS\target\release\quickpls-desktop.exe`
- `D:\QuickPLS\target\release\bundle\nsis\QuickPLS_1.0.0_x64-setup.exe`

## Latest Verification

- `npm test -- --run` passes with 10 files and 72 tests, including `src/components/accessibilityContracts.test.ts`, shortcut wiring coverage, persistent footer readiness coverage, and construct-level indicator-side regression coverage.
- `npm run build` passes and emits the production frontend bundle.
- `npm run qpls:v11:ux-smoke` passes. It builds the production frontend, serves it at desktop viewport `1440x900`, captures Data, Model, Validate, Run, Results, and Report screenshots, verifies desktop-first workflow labels, confirms large table accessibility metadata, checks visible disabled Run/Report guidance, confirms the Report R-squared label renders without mojibake, injects a completed run through a guarded smoke hook, and verifies completed Results tables plus Model/Report diagram overlays for loadings, path coefficients, and R-squared values.
- The v1.1 UX smoke now also verifies the Model canvas overlay status strip, the canvas next-action prompt, the persistent footer readiness checklist, and explicitly checks that R-squared text has no mojibake.
- `npm run tauri -- build` passes and rebuilds `target\release\quickpls-desktop.exe` plus the NSIS installer.
- `npm run qpls:v11:native-smoke` passes. It verifies the rebuilt release executable exists, the installer exists, the executable is newer than the current frontend `dist`, the app launches without a dev server, and the v1.1 completed-run UX smoke artifact is passing.
- `npm run qpls:v11:report-parity` passes. It verifies the Report WYSIWYG SVG preview and downloaded SVG export contain the same key loading/path/R-squared labels, omit edit chrome, and avoid R-squared mojibake.
- `npm run qpls:v11:keyboard-smoke` passes. It verifies named, focusable Results and Report table regions at desktop viewport.
- `npm run qpls:v11:native-workflow` passes. It runs `quickpls-desktop::desktop_native_v11_workflow_smoke_import_run_save_reopen_and_export` for native import-style ingestion, PLS run, project save/reopen, layout persistence, and XLSX export coverage.
- `npm run qpls:v11:audit` passes and verifies all v1.1 desktop UX completion evidence.
- `npm run qpls:v11:release` passes end-to-end and `cargo run -p qpls-cli -- gate v1_1_desktop_ux_polish` reports `7/0/0` with promotion gate clear.
- Latest rebuilt artifacts after v1.1 release pass:
  - `D:\QuickPLS\target\release\quickpls-desktop.exe` written `2026-07-19T23:34:08+0530`
  - `D:\QuickPLS\target\release\bundle\nsis\QuickPLS_1.0.0_x64-setup.exe` written `2026-07-19T23:34:08+0530`
