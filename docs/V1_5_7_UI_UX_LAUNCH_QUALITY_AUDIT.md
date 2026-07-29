# QuickPLS v1.5.7 UI/UX Launch-Quality Audit

This milestone converts the latest full-screen review into a durable launch-quality issue register. It is an audit and planning gate, not a redesign implementation. The goal is to stop treating the frontend as a set of isolated fixes and instead drive the remaining polish from a prioritized, screenshot-backed backlog.

## Non-Engine Boundary

This audit is product/UI-only. No statistical engine, estimator formula, validation tolerance, result schema, project format, or numerical fingerprints changes are included. No statistical engine behavior is considered in scope for this milestone.

## Severity Summary

| Priority | Meaning | Count | Primary screens |
|---|---:|---:|---|
| P0 | Must fix before launch-grade UI claim | 10 | Results, Report, Data, Model, Global |
| P1 | High-value quality improvement | 28 | Results, Setup, Model, Data, Report |
| P2 | Professional polish and consistency | 18 | Global, Home, Run, Report |
| P3 | Nice-to-have cleanup | 4 | Home, Global, Setup |

The most important pattern is clear: QuickPLS has a functional workflow, but the interface still carries prototype traits. The biggest remaining risks are Results readability, Report export confidence, SEM canvas surrounding chrome, and global design-system consistency.

## Screen Evidence

The supplied screenshots are preserved by `validation/v157_ui_ux_launch_quality_smoke.py` under:

```text
validation/results/screens/v157/ui-ux-launch-quality/
```

| Evidence | Screen/state | Key concern |
|---|---|---|
| `01_home.png` | Home | Launcher is clean but underpowered and sparse. |
| `02_data_top.png` | Data top | Import controls dominate after data exists. |
| `03_data_preview_top.png` | Data preview | Grid is useful but lacks mature data-table affordances. |
| `04_data_metadata.png` | Data metadata | Metadata panel consumes width and needs collapse behavior. |
| `05_model_canvas.png` | Model canvas | Context toolbar and side panels crowd the diagram. |
| `06_setup_top.png` | Setup top | Readiness is still too card-heavy. |
| `07_setup_presets.png` | Setup presets | Preset cards are visually blank and settings are too low. |
| `08_setup_method_table.png` | Setup method table | Method status table appears far below selected setup. |
| `09_run.png` | Run | Screen is clean but repeats Setup and remains sparse. |
| `10_results_overview_top.png` | Results overview | Tabs/tools are crowded and findings dominate. |
| `11_results_overview_table.png` | Results overview table | Findings repeat and tables are pushed down. |
| `12_results_structural_tables.png` | Results structural | Mediation/effects tables overflow and wrap awkwardly. |
| `13_report_setup.png` | Report setup | Controls are scattered; preset purpose is not visible. |
| `14_report_diagram_preview.png` | Report preview | Diagram label overlap remains visible. |
| `15_report_exports_comparison.png` | Report exports/comparison | Export cards and comparison placement need hierarchy. |

## Issue Register

| ID | Priority | Screen | Category | Issue | Recommendation |
|---|---|---|---|---|---|
| UX-001 | P0 | Global | global | Visible version/status can drift from build metadata. | Wire visible app version and release label from one source and add a static drift check. |
| UX-002 | P1 | Global | global | Primary Run action appears on every workspace and competes with local tasks. | Make global Run compact outside Setup/Run or move it into command palette/shortcut behavior. |
| UX-003 | P2 | Global | color | Teal is overloaded as brand, active state, status, and primary action color. | Separate brand/action/status palettes. |
| UX-004 | P2 | Global | layout | Bottom status bar is useful but visually noisy. | Group chips by project/data/model/method and collapse low-priority details. |
| UX-005 | P1 | Global | layout | Nested scrollbars appear in Data, Results, and Report. | Define one scroll owner per workspace and avoid component scroll unless table overflow requires it. |
| UX-006 | P2 | Global | design-system | Cards have weak hierarchy. | Add primary, secondary, warning, result, and export card variants. |
| UX-007 | P2 | Global | layout | Desktop density is inconsistent. | Standardize page gutters, row height, section spacing, and table density tokens. |
| UX-008 | P2 | Global | workflow | Workflow strip duplicates left navigation. | Convert strip into compact readiness ribbon or hide on dense workspaces. |
| UX-009 | P3 | Global | visual | Icon weight and alignment are not fully unified. | Normalize icon size, stroke, button padding, and baseline alignment. |
| UX-010 | P1 | Global | accessibility | Launch accessibility is not proven from screenshots. | Add keyboard/focus/contrast smoke and require visible focus states. |
| UX-011 | P2 | Home | layout | Home feels static and underpowered. | Add recent projects, recovery state, and last-run/project summary. |
| UX-012 | P3 | Home | workflow | Home cards duplicate rail steps. | Make cards contextual to current project state or remove duplicate workflow cards. |
| UX-013 | P2 | Home | layout | Large unused space makes the launcher feel unfinished. | Use a two-column launcher with project state and recent work. |
| UX-014 | P1 | Home | workflow | Continue recent project is not a real recent-project list. | Render actual recent projects or hide this card. |
| UX-015 | P3 | Home | copy | Save/autosave message is too passive. | Use a stronger project state panel with Save project as primary when unsaved. |
| UX-016 | P0 | Data | layout | Import source dominates even after data is loaded. | Collapse import controls after successful import and make preview/quality dominant. |
| UX-017 | P1 | Data | tables | Data grid lacks mature desktop table controls. | Add column resize, sort, freeze, density, and stronger selected-column affordances. |
| UX-018 | P1 | Data | workflow | Data quality cards do not lead to actions. | Link each quality card to metadata, Setup, or issue details. |
| UX-019 | P2 | Data | copy | Prefix chips look technical. | Show construct-name preview with indicator count and confidence. |
| UX-020 | P1 | Data | layout | Metadata panel consumes too much width during inspection. | Make metadata collapsible and allow grid-first mode. |
| UX-021 | P2 | Data | copy | Browser/desktop import copy is implementation-oriented. | Use researcher-centered copy around formats, missing values, and saved project state. |
| UX-022 | P0 | Model | layout | Selected-object context toolbar is crowded. | Keep high-frequency actions visible and move secondary actions into grouped menus. |
| UX-023 | P0 | Model | diagram | Diagram labels remain fragile and tiny. | Add label collision rules, larger label tokens, and preview warnings. |
| UX-024 | P1 | Model | layout | Left explorer construct cards are too dense. | Move to row-based inventory with hover/context actions. |
| UX-025 | P1 | Model | workflow | Right inspector duplicates canvas/sidebar controls. | Make inspector primarily properties/results; keep normal actions on canvas/context menus. |
| UX-026 | P2 | Model | layout | Mini-map feels decorative. | Hide by default or make it a real navigation/focus control. |
| UX-027 | P1 | Model | layout | Canvas width is constrained by both side panels. | Add obvious one-click collapse and remember panel state. |
| UX-028 | P2 | Model | copy | Result overlay banner consumes canvas space. | Replace with compact overlay status in toolbar. |
| UX-029 | P1 | Setup | layout | Readiness cards remain too card-heavy. | Use compact checklist with expandable details. |
| UX-030 | P1 | Setup | workflow | Method presets do not explain setting changes. | Make preset cards selectable and show expected outputs/settings delta. |
| UX-031 | P1 | Setup | workflow | Advanced methods appear too early. | Move group/prediction/CB-SEM/extended methods into Expert sections. |
| UX-032 | P3 | Setup | controls | Bootstrap checkbox looks unfinished. | Use a setting row/toggle with sample count and inference consequence. |
| UX-033 | P1 | Setup | layout | Method status table is too low. | Use a split setup layout: selection left, readiness/details right. |
| UX-034 | P1 | Setup | copy | Validated-scope boundaries are not always near the relevant setting. | Add selected-method scope details beside settings. |
| UX-035 | P1 | Run | layout | Run repeats Setup readiness. | Compress readiness to a top checklist and expand only failures. |
| UX-036 | P1 | Run | workflow | Global Run and page Run compete. | Keep one primary run action visible on Run screen. |
| UX-037 | P2 | Run | copy | Post-run cards are generic. | Use result-aware next steps after completion. |
| UX-038 | P2 | Run | layout | Run screen is sparse at desktop size. | Add execution summary, output preview, provenance, and last-run handoff. |
| UX-039 | P0 | Results | layout | Result tabs wrap into a bulky grid. | Replace with a left section list or compact tab bar with overflow. |
| UX-040 | P0 | Results | tables | Result tables are hard to scan. | Build a reusable research table shell with sticky headers, column groups, density, and horizontal overflow affordances. |
| UX-041 | P0 | Results | interpretation | Findings repeat symmetrical HTMT issues. | Deduplicate mirrored findings and cap visible cards by priority. |
| UX-042 | P0 | Results | layout | Issue cards dominate the page. | Summarize findings into Must address / Review / Info lanes with expandable details. |
| UX-043 | P0 | Results | tables | Mediation table is especially hard to read. | Split mediation into direct/indirect/total, inference, and classification subtables. |
| UX-044 | P1 | Results | controls | Search/export/precision/filter controls are crowded. | Group tools into table, export, display, and interpretation menus. |
| UX-045 | P1 | Results | interpretation | Interpretation hierarchy is not yet a canonical reporting checklist. | Make Interpretation the primary report-readiness checklist. |
| UX-046 | P2 | Results | workflow | Result summary cards are plain and passive. | Make summary cards clickable filters. |
| UX-047 | P2 | Results | copy | Scope warning line is repeated and low-priority. | Use one run-level scope chip with expandable details. |
| UX-048 | P0 | Report | diagram | Publication diagram still has label overlap. | Apply export label-collision rules and fail preview audit when overlap is visible. |
| UX-049 | P1 | Report | layout | Publication preview uses nested scrolling. | Use fit-to-width/fit-page controls and a single clear preview frame. |
| UX-050 | P1 | Report | workflow | Export cards look passive. | Convert exports into explicit buttons with destination feedback and disabled reasons. |
| UX-051 | P1 | Report | controls | Report controls are scattered. | Group controls into Figure, Statistics, Tables, and Notes sections. |
| UX-052 | P1 | Report | workflow | Run comparison placement is confusing. | Keep comparison in Results and link a summary from Report only. |
| UX-053 | P2 | Report | copy | PDF path reads like a limitation rather than a workflow. | Add guided print/PDF action or move to documentation. |
| UX-054 | P1 | Global | accessibility | Subtle text contrast may be weak. | Run contrast audit and raise secondary text contrast. |
| UX-055 | P1 | Global | accessibility | Icon-only actions need stronger labels/tooltips. | Require accessible names and visible tooltips for icon actions. |
| UX-056 | P1 | Tables | accessibility | Keyboard table navigation is not visibly proven. | Add keyboard smoke for data/results table search, row focus, and export. |
| UX-057 | P1 | Model | performance | Large-model visual quality remains unproven. | Add 8/32 and 20/80 model screenshots to the next designer QA. |
| UX-058 | P1 | Global | design-system | Styles appear accumulated rather than token-governed. | Create token inventory and block one-off card/table/status styles. |
| UX-059 | P0 | Global | launch | There is no single launch-grade visual acceptance checklist. | Add launch UI QA checklist with required screenshot states and pass/fail rules. |
| UX-060 | P0 | Global | planning | Remaining fixes need dependency order. | Sequence work: shell/design-system, Results, Report, Model, Setup/Run, Data/Home. |

## Implementation Sequence

1. `v1_5_8_results_workspace_launch_redesign`
   Fix the Results information architecture, table shell, findings triage, deduplication, mediation layout, and result-control toolbar.

2. `v1_5_9_report_publication_workflow_redesign`
   Fix publication diagram preview, export control grouping, explicit export buttons, comparison placement, and report-ready output hierarchy.

3. `v1_6_0_model_canvas_shell_and_panel_polish`
   Fix canvas side-panel pressure, context toolbar crowding, label readability, mini-map behavior, and large-model screenshot evidence.

4. `v1_6_1_setup_run_workflow_consolidation`
   Merge repeated readiness patterns, make method presets self-explanatory, hide advanced method families until Expert mode, and simplify Run.

5. `v1_6_2_data_home_launch_polish`
   Collapse Data import after load, strengthen the data grid, improve prefix-to-construct preview, and make Home a true project launcher.

6. `v1_6_3_global_design_system_and_accessibility_pass`
   Normalize typography, spacing, color semantics, cards, badges, tables, icons, focus states, status bar, and launch accessibility checks.

## Acceptance Criteria

- All 15 supplied screenshots are preserved under `validation/results/screens/v157/ui-ux-launch-quality/`.
- The issue register contains at least 60 traceable issues across Home, Data, Model, Setup, Run, Results, and Report.
- Results and Report are explicitly identified as the first launch-impact implementation milestones.
- The audit distinguishes planning evidence from implementation completion.
- No unsupported statistical or SmartPLS-equivalence claim is introduced.
- No numerical engine, result schema, project format, validation tolerance, or numerical fingerprints changes are made.

## Gate

Run:

```powershell
npm run qpls:v157:ui-ux-launch-audit
cargo run -p qpls-cli -- gate v1_5_7_ui_ux_launch_quality_audit
```

This gate is complete when the audit evidence exists and the issue register is deep enough to drive the next implementation work.
