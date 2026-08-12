# QuickPLS Native Frontend Wiring Matrix

This matrix records the current native-shell command and production-binding state for `v2_44_0_native_ui_production_binding_completion`. The rule is simple: a visible command must either call a real QuickPLS action, open a real dialog/workspace, bind to real workspace data, or be intentionally absent from the default shell.

## v2.44 Production-Binding Status

| Area | Status |
| --- | --- |
| Home and recent projects | Bound to opened project/session state plus the persisted local recent-project list. Fallback demo projects are restricted to explicit mockup-parity mode. |
| Message center and status bar | Bound to project lifecycle, data/model/run/report state, command feedback, offline mode, and scoped engine state. Fake CPU/memory meters are absent from the default shell. |
| Data workbench | Bound to active dataset rows, headers, quality summaries, selected-variable metadata, and real Data command dialogs. Import preview uses current dialog/workspace state instead of static customer-survey data. |
| Model workbench | Bound to live constructs, indicators, paths, model issues, SEM canvas selection, and selected-run overlays. Static fallback constructs/paths are restricted to explicit mockup-parity mode. |
| Setup and calculation setup | Bound to the real method applicability/settings state, selected add-ons, blockers, expected outputs, and method scope details. |
| Run monitor | Bound to real run readiness/job lifecycle/log state; Save Log exports the visible run log when available. Pause remains intentionally absent. |
| Results workbook | Bound to selected saved run payloads and selected-row/interpretation state. No fallback result rows or static run summary appear in the default shell. |
| Report wizard | Bound to selected run/report summary, figure preview, export destination, and existing SVG/CSV/HTML/XLSX/reviewer-pack export commands. |
| Trust Center and Settings | Bound to local docs/evidence/status data and persisted UI preferences. Settings no longer shows fake unsaved-change status. |
| Mockup-only data | Available only through explicit `mockup_parity` review mode; it is not used as default-shell production data when real project/run state exists. |

## Wired Shell Commands

| Area | Command | Current behavior |
| --- | --- | --- |
| File/Home | New Project | Opens the native-shell New Project dialog. |
| File/Home | Open Project | Dispatches the existing `quickpls:open-project` command handled by the hidden production command bridge. |
| File/Home | Save | Dispatches the existing `quickpls:save-project` command. |
| File/Home | Save As | Dispatches `quickpls:save-project-as`, forcing the existing native save dialog path. |
| File/Home | Close Project | Opens the unsaved-changes decision dialog with Save and close, Close without saving, and Cancel. Closing clears project, dataset, model selection, run/report selection, transient warnings, and active workspace state. |
| File/Home | Exit | Closes the desktop window through the browser/Tauri window close path. |
| Home | Recent | Focuses the Recent Projects pane and reports the action in the native status bar. |
| Data | Import Data | Opens the Import Data dialog; the dialog Import action dispatches `quickpls:import-data`. |
| Data | Transform / Add Column / Recode / Missing Values / Filter / Sort | Opens desktop task dialogs and dispatches structured payloads to the Data workspace. Transform creates a standardized numeric copy, Add Column appends a project dataset column, Recode and Missing Values update current values/metadata, Filter narrows visible columns, and Sort reorders rows by a selected column. |
| Data | Create Constructs | Dispatches `quickpls:data-create-constructs-from-prefixes`, using the existing Data workspace prefix grouping and model creation path. |
| Data | Validate Data | Dispatches `quickpls:data-show-quality`, switching the embedded Data workspace to its quality/readiness tab. |
| Edit | Undo / Redo / Cut / Copy / Paste | Undo/Redo dispatch to the live model command bridge where applicable. Cut/Copy/Paste use the active text field or current browser selection/clipboard path. |
| Model | Add Latent / Add Indicator / Select / Pan / Path / Covariance / Delete / Arrange / Check Diagram / Focus Diagram / Zoom | Dispatches native shell model events consumed by the existing live SEM designer. Add Indicator assigns the next unassigned dataset column to the selected construct when possible. The canvas remains the single source of model-edit behavior. |
| Setup | Constructs / Indicators / Diagram / Groups / Quality / Export Tables / Export Figures | Navigates to the existing Model, Results, or Report workspaces rather than exposing no-op shell commands. |
| Calculate/Run | Validate | Opens/focuses the Run workspace readiness checklist and reports the action in the native status bar. |
| Calculate/Run | Calculation Setup | Opens the Calculation Setup dialog. |
| Calculate/Run | Run | Dispatches the existing `quickpls:run-analysis` command through the production run handler. |
| Calculate/Run | Cancel | Dispatches the existing `quickpls:cancel-analysis` command. Pause/resume is intentionally absent because the current production runner supports cancellation, not safe suspension/resumption. |
| Results | Select Run / Method Confidence / Copy Table / Export Table / Show Interpretation / Compare Runs / Prepare Report | Dispatches native shell result events consumed by the existing saved-run workbook. Copy/export use the current result tab and selected run. |
| Report | Select Run / Preview / Export SVG / Export Tables / Export Workbook / Print / Reviewer Pack / Open Folder | Dispatches native shell report events consumed by the existing report wizard and export flow. Open Folder calls the desktop command that opens the default QuickPLS export folder. |
| Trust Center | Refresh Evidence / Open Method Doc / Export Evidence Index / Verify Checksums / Known Differences / About Validation | Dispatches native shell Trust Center events. Refresh reports the bundled evidence state, Open Method Doc copies the relevant documentation path, Export Evidence Index writes the current evidence matrix as CSV, and Verify Checksums checks the latest versioned release checksum file and opens a release-integrity detail dialog. |
| Tools | Trust Center / Preferences / Method Scope | Navigates to Trust Center/Settings or opens Method Scope. |
| Help | Shortcuts / Documentation | Shortcuts opens the Help / Shortcuts dialog. Documentation opens the local in-app documentation browser with Quick Start, Data Import, SEM Designer, Method Setup, Running Analyses, Results Interpretation, Report Export, and Trust Center sections. |
| Settings | Apply / OK / Cancel / Reset Defaults / Import Preferences / Export Preferences | Dispatches settings events consumed by the existing UI-preferences workspace. Reset returns UI preferences to QuickPLS defaults; import reads a validated JSON preferences file; export writes a JSON preferences file. |
| View/Window | Status Bar / Save Layout / Reset Layout / Close Pane / Restore Pane | Persists harmless UI-only layout preferences in local storage, toggles the native status bar, hides/restores optional panes, and restores desktop defaults without touching project numerical fingerprints. |

## Real Workspaces Embedded In Native Shell

These surfaces are backend/store-backed by the existing QuickPLS implementation:

- Data workspace: existing import/data preview/metadata UI.
- Model workspace: existing SEM designer, explorer, inspector, and issue pane.
- Setup workspace: existing analysis catalog and applicability UI.
- Run workspace: existing readiness, job launch, cancellation, and run monitor UI.
- Results workspace: existing saved-run workbook and table exports.
- Report workspace: existing SVG/table/HTML/XLSX report export flow.
- Trust Center and Settings workspaces: existing product/support surfaces.

## Intentionally Absent From The Default Shell

- Calculation pause/resume is not shown. QuickPLS currently supports cancellation, not safe job suspension/resumption.
- Native PDF/PNG export remains absent; SVG is the audited diagram export and browser Print/PDF is the documented PDF path.
- Any older non-mockup extras remain tracked in `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md` until explicitly reintroduced through native desktop patterns.
