# QuickPLS v2.36.0 Native Desktop UI Spec And Component Plan

## Purpose

This milestone freezes the QuickPLS 2.x visual and interaction direction before further implementation work. The product direction is a native Windows research workbench inspired by SmartPLS, SPSS, AMOS, and other professional statistical software, while preserving QuickPLS' validated statistical engines and current SEM designer behavior.

This is a planning/specification milestone. It does not require installer, portable executable, or checksum artifacts.

## Source Mockups

The following generated mockups are the implementation source of truth for the next redesign pass:

| Surface | Mockup path |
| --- | --- |
| Home / Project Manager | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_mOAP2bRNExdX2qXZcWnAUAGG.png` |
| Data Workbench | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_CZYfQpNAUfxW005dnSaQYjXl.png` |
| Model Workbench | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_87wjqS5PpUTmQkx4qI90H141.png` |
| Setup / Method Applicability Center | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_YRdo1lZIT5ACAInWXdqXhnEI.png` |
| Run / Calculation Monitor | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_8pwwkV1h4Y6rUX7b5um09D6z.png` |
| Results Workbook | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_UQcHMzc1A7oPnglWKpLXo9k6.png` |
| Report / Export Wizard | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_Au9nFkEISuxXT0FnBxIF5a1D.png` |
| Trust Center / Evidence Workbench | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_0iMMnRjmHqb1hwXPSjV3z3RC.png` |
| Settings / Preferences | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_MqmnrE6vtbxnoeNeNno2As9u.png` |
| Sample Project Gallery | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_5XzJhhnDaYB905CVUElfXpdK.png` |
| Import Data Dialog | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_8Bixbkr0ltT8WyqhyK1rBxcH.png` |
| Calculation Setup Dialog | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_E8dfRfcaHvJtOZPlrdDoXCnc.png` |
| Method Scope / Evidence Dialog | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_qMn4sCYwkMsRCaadRF8gxPug.png` |
| Export Options Dialog | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_zjyk8k3OGlFrrknYB7YoKEIr.png` |
| Help / Shortcuts Dialog | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_D0RcUlMzQyBN63D6f9FGLO4b.png` |
| Focus Diagram Mode | `C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da\call_8Iul9llgdREYcrFTEPTLMYBX.png` |

## Final Design Direction

Use the Model-First SEM Studio shell as the core application structure, then borrow:

- the guided method setup and evidence drawer from the Guided Research Workflow Suite;
- the tabbed workbook, result tables, and bottom output log from the Classic Statistical Workbench;
- native modal/task dialogs for import, calculation setup, evidence, export, help, and sample projects.

QuickPLS should feel like a desktop workbench, not a dashboard. The application should be pane-based, menu-driven, compact, and keyboard-friendly.

## Visual Contract

### Application Shell

Every screen uses the same shell:

- thin native-style title strip;
- menu bar: `File`, `Edit`, `Data`, `Model`, `Calculate`, `Results`, `Report`, `View`, `Tools`, `Window`, `Help`;
- compact command toolbar below the menu;
- left workflow rail: `Home`, `Data`, `Model`, `Setup`, `Run`, `Results`, `Report`, with `Trust Center` and `Settings` separated at the bottom;
- central workbench surface using split panes or tabbed workbook pages;
- optional right property/evidence/interpretation inspector;
- optional bottom docked pane for messages, issues, logs, output, provenance, or export history;
- permanent bottom status bar.

### Density And Spacing

- Use compact desktop spacing by default.
- Prefer 1px borders and splitter lines over large shadows.
- Use panel headers, table headers, and property-sheet labels instead of large dashboard cards.
- Keep headings short and functional.
- Avoid large call-to-action blocks except in modal footer actions or first-run project creation.

### Color

- Neutral shell: light gray application chrome and white content panes.
- Teal is reserved for active selection, primary action, active rail item, and active tab underline.
- Green/amber/red indicate validated/needs review/error status.
- Do not use gradients, decorative backgrounds, large color washes, or marketing-style palettes.

### Typography

- Use compact Windows-like desktop typography.
- Dense tables and inspectors should be readable at 9-12px equivalent sizes.
- Avoid hero-scale text.
- Labels must not collide with body text.
- Use `R-squared` in source text where typographic superscript is risky; render `R²` only when verified in UI/SVG.

### Controls

- Use native-like menu items, toolbar buttons, split buttons, dropdowns, property-sheet fields, checkboxes, radio groups, tree views, tables, tabs, and modal footers.
- Icon-only controls require tooltips and accessible labels.
- Disabled controls must expose a nearby reason or accessible description.

## Screen Implementation Map

### Home / Project Manager

Target purpose: desktop start center and project recovery surface.

Required layout:

- left command pane: new project, open project, import dataset, open sample project;
- central recent project table with path, modified date, dataset, runs, and status;
- right selected-project details and quick actions;
- bottom pane: messages, recovery, validation evidence, recent activity.

Replace:

- dashboard-style launcher cards;
- large empty whitespace;
- marketing-like copy.

Reuse:

- existing new/open/save/import/demo command handlers;
- existing recent-project and autosave/recovery state where available.

### Data Workbench

Target purpose: SPSS-like data and variable management.

Required layout:

- tabs: `Data View`, `Variable View`, `Import History`, `Data Quality`, `Notes`;
- dense grid with sticky row and column headers;
- right variable properties inspector;
- top data quality strip;
- bottom pane: import log, data issues, method applicability, notes.

Replace:

- large data quality cards as the primary visual;
- long scrolling data page;
- ambiguous raw versus matrix import copy.

Reuse:

- existing dataset preview, metadata editor, prefix detection, and data-quality summaries.

### Model Workbench

Target purpose: preserve the SEM designer while making the surrounding shell native.

Required layout:

- left SEM explorer tree;
- central SEM canvas;
- right object inspector;
- bottom pane: model issues, diagram advisor, calculation log, output;
- Focus Diagram mode for full-canvas editing.

Preserve:

- existing SEM designer drag/drop and layout behavior;
- construct/indicator/path/covariance recipe semantics;
- result overlay behavior.

Improve around the designer:

- property-sheet inspector density;
- native context menus;
- compact toolbar;
- status bar with selection, coordinates, zoom, model validity.

### Setup / Method Applicability Center

Target purpose: make method selection understandable and setting-aware.

Required layout:

- readiness strip: data, model, sample size, scope, inference;
- method lanes: recommended, available with setup, advanced diagnostics, not applicable;
- lower selected-method setup panel;
- right scope/evidence drawer;
- bottom expected outputs/blockers/warnings/next actions table.

Required behavior:

- bootstrap and permutation appear as add-ons, not confusing primary algorithms;
- unavailable methods show exact reasons;
- top method selector lists recommended/available methods by default.

### Run / Calculation Monitor

Target purpose: native computation-monitor workflow.

Required layout:

- left procedure checklist;
- center progress, algorithm status, iteration table, run log;
- right immutable run settings summary;
- bottom output preview and action footer;
- status bar with running/complete/cancelled state.

Required behavior:

- disabled run actions show exact blocker;
- completed runs hand off to Results and Report;
- cancellation state is clear.

### Results Workbook

Target purpose: professional statistical workbook.

Required layout:

- sticky selected-run header;
- workbook tabs: overview, measurement, structural, validity, inference, prediction, groups, diagnostics, interpretation, comparison;
- central research table shell;
- right interpretation/detail pane;
- finding lanes;
- bottom provenance/log/export history pane.

Required behavior:

- row selection opens value-specific interpretation;
- findings deduplicate symmetric metrics;
- table actions are consistent across all result families.

### Report / Export Wizard

Target purpose: desktop export workflow.

Required layout:

- four steps: select content, preview, document settings, export;
- preset list;
- content tree;
- WYSIWYG figure/table preview;
- right settings pane;
- bottom export action strip with destination and status.

Required behavior:

- SVG remains audited primary figure export;
- CSV/HTML/XLSX remain table/report exports;
- browser print/PDF path is documented but not overclaimed as native PDF export.

### Trust Center / Evidence Workbench

Target purpose: method trust, compatibility, evidence, and release integrity.

Required layout:

- method family tree and filters;
- compatibility matrix;
- validation evidence table;
- right evidence detail pane;
- bottom release integrity/audit/dependencies/method notes pane.

Required behavior:

- `Why trust this result?` links from Setup, Results, and Report land here or open the evidence dialog;
- validated, experimental, and unsupported scopes remain distinct.

### Settings / Preferences

Target purpose: native preferences surface.

Required layout:

- category tabs/tree: general, data, modeling, results, export, advanced;
- compact grouped property sheets;
- right interface preview;
- bottom pending changes/environment/preferences file pane;
- Apply, OK, Cancel, Reset Defaults.

Required behavior:

- preference changes must not affect numerical fingerprints;
- unsaved preference changes are visible.

## Dialog Implementation Map

### Sample Project Gallery

Required:

- category tree;
- sample table;
- preview/details pane;
- filters, difficulty, runnable-only toggle;
- `Open Sample`, `Copy to My Projects`, `View Documentation`, `Cancel`.

### Import Data Dialog

Required:

- wizard steps: source, options, preview, metadata, import;
- mode selection: raw data, covariance matrix, correlation matrix, sample dataset;
- preview grid;
- metadata preview;
- validation summary;
- desktop file dialog bridge.

### Calculation Setup Dialog

Required:

- navigation: algorithm, inference, prediction, groups, outputs, reproducibility, scope;
- algorithm and add-on settings;
- output preview tree;
- reproducibility panel;
- `Restore Defaults`, `Save Preset`, `Run`, `Cancel`.

### Method Scope / Evidence Dialog

Required:

- scope statement;
- current-project requirements checklist;
- unsupported variants;
- known differences;
- validation artifacts table;
- links to Trust Center and method docs.

### Export Options Dialog

Required:

- selected run and preset;
- format selection;
- destination/naming;
- content options;
- files-to-be-written preview;
- validation strip;
- `Export`, `Save as Preset`, `Cancel`.

### Help / Shortcuts Dialog

Required:

- search;
- current workspace filter;
- topic tree;
- shortcut table;
- context help;
- related documentation links.

### Focus Diagram Mode

Required:

- collapsed workflow rail;
- no explorer or inspector by default;
- compact canvas toolbar;
- minimap and zoom;
- result overlay allowed;
- `Esc` exits focus mode.

## Shared Component Inventory

Build or stabilize these reusable components before further screen-specific work:

- `DesktopShell`
- `DesktopMenuBar`
- `DesktopCommandToolbar`
- `WorkflowRail`
- `DockedPane`
- `PaneSplitter`
- `WorkbookTabs`
- `PropertySheet`
- `TreeView`
- `ResearchTable`
- `StatusBar`
- `StatusChip`
- `FindingLane`
- `EvidenceDrawer`
- `DialogManager`
- `TaskDialog`
- `WizardDialog`
- `CommandRegistry`
- `ContextMenu`
- `MessageCenter`
- `OutputLog`

## State Model

UI-only state may include:

- active menu;
- open dialog id and dialog payload;
- split-pane sizes;
- collapsed pane state;
- active workbook tabs;
- selected table row;
- active inspector tab;
- selected method card;
- selected report step;
- selected export preset;
- focus diagram mode;
- preferences draft.

This state must stay out of estimator inputs and numerical fingerprints unless it already belongs to existing harmless UI/project layout metadata.

## Implementation Milestone Order

The existing v2.32-v2.35 order remains valid, but the implementation should now use this blueprint:

1. `v2_32_0_trust_center_evidence_workbench`
   - implement Trust Center and evidence dialog direction.
2. `v2_33_0_settings_preferences_environment`
   - implement Settings and preference dialog direction.
3. `v2_34_0_desktop_polish_accessibility_qa`
   - unify remaining shell, command, menu, dialog, keyboard, status, and visual polish issues.
4. `v2_35_0_native_desktop_release_candidate`
   - run final screenshot parity, documentation, tests, and versioned artifact build.

If the team decides the current v2.21-v2.31 implementation is too far from the final mockups, add a consolidation pass before v2.32:

- `v2_36_1_shell_consolidation_against_final_mockups`
- `v2_36_2_screen_layout_consolidation_against_final_mockups`
- `v2_36_3_dialog_consolidation_against_final_mockups`

These optional passes should still be frontend-only.

## Non-Negotiable Boundaries

- Do not change statistical engines.
- Do not change formulas.
- Do not change result schemas.
- Do not change project archive format.
- Do not change validation tolerances.
- Do not change numerical fingerprints.
- Preserve SEM designer core behavior and recipe semantics.
- Do not add SmartPLS import, equivalence, or reverse-engineering claims.
- Do not build installer/portable/checksum artifacts for this planning milestone.

## Acceptance Criteria

This milestone is complete when:

- every main screen and supporting dialog has a named implementation target;
- every generated mockup is recorded;
- the visual contract is explicit enough for implementation;
- reusable components are listed;
- frontend-only and numerical-boundary rules are explicit;
- the v2.36 audit passes;
- the registry gate `v2_36_0_native_desktop_ui_spec_and_component_plan` reports all passed.

## Verification

```powershell
npm run qpls:v236:native-ui-spec-audit
cargo run -p qpls-cli -- gate v2_36_0_native_desktop_ui_spec_and_component_plan
```
