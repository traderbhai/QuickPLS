# QuickPLS Development Ledger

Active QuickPLS 2.x working tracker: `docs/V2_ACTIVE_MILESTONE.md`.

Use the active tracker for the current grouped milestone rules and checkpoint. Permanent milestone notes remain in this ledger after a milestone is implemented and verified.

Last audited: 2026-07-19

Status vocabulary:

- **Proven**: the release scope and exit gate have direct current-state evidence.
- **Implemented, gate open**: executable functionality exists, but one or more release-gate proofs are missing.
- **Pending**: the planned implementation is not present.

This ledger is the release tracker. `DELIVERY_STATUS.md` is a concise product summary; it must not be used by itself as evidence that a numerical gate passed.

## Release Ledger

## v2.44.0 Native UI Production Binding Completion

- Bound default native-shell Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings surfaces to real workspace/project/run/report state wherever available.
- Restricted fallback mockup data to explicit mockup-parity review mode and removed fake CPU/memory/status telemetry from the default shell.
- Added v2.44 smoke/audit scripts for Home/project status, Data binding, Model binding, Setup binding, Run/Results/Report binding, Trust/Settings commands, and full production-binding audit.
- Added `docs/V2_44_0_NATIVE_UI_PRODUCTION_BINDING_COMPLETION.md`, updated the native frontend wiring matrix, and registered gate `v2_44_0_native_ui_production_binding_completion`.
- Updated release metadata to `2.44.0`; versioned artifacts remain deferred until explicitly requested.
- Boundary: frontend/native-shell binding only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.25.0 Model Workbench Integration

- Added `src/components/ModelIssuesPane.tsx` as a bottom native workbench pane for model readiness, selected-object context, publication checks, and quick actions.
- Marked the model explorer, SEM canvas, inspector, and bottom pane with v2.25 workbench evidence hooks.
- Updated model shell CSS to use a native four-pane workbench with a third output/status row and Focus Diagram mode that hides surrounding chrome while preserving canvas interaction.
- Added `validation/v2250_model_workbench_smoke.mjs`, `validation/v2250_model_workbench_audit.py`, `docs/V2_25_0_MODEL_WORKBENCH_INTEGRATION.md`, and registry gate `v2_25_0_model_workbench_integration`.
- Updated release metadata to `2.25.0` for the active native redesign checkpoint. Versioned artifacts remain deferred until release-candidate stage or explicit request.
- Boundary: frontend/product only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.24.0 Data Workbench Redesign

- Reworked Data into a tabbed desktop data workbench with `Data View`, `Variable View`, `Import History`, `Data Quality`, and `Notes`.
- Added a variable metadata table, variable issue table, and workbench tab styling while preserving native import and metadata update APIs.
- Added `validation/v2240_data_workbench_smoke.mjs`, `validation/v2240_data_workbench_audit.py`, `docs/V2_24_0_DATA_WORKBENCH_REDESIGN.md`, and registry gate `v2_24_0_data_workbench_redesign`.
- Updated release metadata to `2.24.0` for the active native redesign checkpoint. Versioned artifacts remain deferred until release-candidate stage or explicit request.
- Boundary: frontend/product only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.23.0 Home And Project Manager

- Reworked Home into a desktop project-manager start center with project launcher actions, current workspace summary, recent-project list, recovery/autosave status, and quick links.
- Added `validation/v2230_home_project_manager_smoke.mjs`, `validation/v2230_home_project_manager_audit.py`, `docs/V2_23_0_HOME_PROJECT_MANAGER.md`, and registry gate `v2_23_0_home_project_manager`.
- Updated release metadata to `2.23.0` for the active native redesign checkpoint. Versioned artifacts remain deferred until release-candidate stage or explicit request.
- Boundary: frontend/product only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.22.0 Menu Commands Dialogs Native Base

- Added `src/domain/desktopCommands.ts` as the UI-only command inventory for menu location, shortcuts, descriptions, and disabled-reason requirements.
- Expanded the React-rendered desktop menu bar to File, Edit, Data, Model, Calculate, Results, Report, View, Tools, Window, and Help.
- Wired menu/dialog command execution into permanent status-bar command feedback.
- Added `validation/v2220_native_commands_audit.py`, `docs/V2_22_0_MENU_COMMANDS_DIALOGS_NATIVE_BASE.md`, and registry gate `v2_22_0_menu_commands_dialogs_native_base`.
- Updated release metadata to `2.22.0` for the active native redesign checkpoint. Versioned artifacts remain deferred until release-candidate stage or explicit request.
- Boundary: frontend/product only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.21.0 Desktop Design System And Shell

- Added native shell markers and denser neutral desktop chrome for the title/menu/command strip.
- Added UI-only command status state and rendered last command feedback in the permanent bottom status bar.
- Added `validation/v2210_native_shell_audit.py`, `docs/V2_21_0_DESKTOP_DESIGN_SYSTEM_SHELL.md`, and registry gate `v2_21_0_desktop_design_system_shell`.
- Registered v2.23-v2.35 as open follow-on gates instead of claiming the whole native redesign is complete.
- Boundary: frontend/product only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.20.0 QuickPLS 2.0 Mockup Parity Release Audit

- Added final mockup-parity smoke and static audit scripts for the QuickPLS 2.0 frontend program.
- Verified v2.16 desktop shell, v2.17 Home/Data/Setup, v2.18 Model/Run/Results, and v2.19 Report/Trust/Settings remain clear in the registry.
- Audited target screen markers for Home, Data, Model, Setup, Run, Results, Report, Trust Center, Settings, menu bar, and frontend desktop dialogs.
- Added `validation/v2200_mockup_parity_smoke.mjs`, `validation/v2200_mockup_parity_audit.py`, `docs/V2_20_0_QUICKPLS_2_MOCKUP_PARITY_RELEASE_AUDIT.md`, and registry gate `v2_20_0_quickpls_2_mockup_parity_release_audit`.
- Updated release metadata to `2.20.0` and artifact labeling to `v2_20_0_quickpls_2_mockup_parity_release_audit`.
- Boundary: frontend/product audit only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.19.0 Report, Trust Center, And Settings Mockup Alignment

- Added v2.19 screen markers and mockup-alignment classes to Report, Trust Center, and Settings.
- Tightened Report into a compact desktop export flow with denser preset controls, four-step flow, report settings, export actions, and a constrained publication preview region.
- Tightened Trust Center into a mockup-style validation evidence workspace with compact confidence cards, method compatibility tables, evidence panels, and desktop density.
- Tightened Settings into compact grouped forms and preference panels using the same v2 shell and visual contract.
- Added `validation/v2190_report_trust_settings_smoke.mjs`, `validation/v2190_report_trust_settings_audit.py`, `docs/V2_19_0_REPORT_TRUST_SETTINGS_MOCKUP_ALIGNMENT.md`, and registry gate `v2_19_0_report_trust_settings_mockup_alignment`.
- Updated release metadata to `2.19.0` and artifact labeling to `v2_19_0_report_trust_settings_mockup_alignment`.
- Boundary: frontend/product mockup alignment only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.18.0 Model, Run, And Results Mockup Alignment

- Added v2.18 screen markers and mockup-alignment classes to Model, Run, and Results workspaces.
- Tightened the SEM designer shell around the existing canvas: compact toolbar, flatter context toolbar, cleaner overlay notices, and mockup-style canvas chrome without changing drag/drop or diagram logic.
- Tightened the Run workspace into a desktop calculation package layout with launch facts, readiness checks, output preview, execution plan, and handoff cards.
- Tightened the Results workspace into a desktop result workbook with compact section navigation, grouped controls, sticky selected-run context, denser lens panels, and cleaner table/card chrome.
- Fixed stale `R²` mojibake in touched Model/Run UI strings.
- Added `validation/v2180_model_run_results_smoke.mjs`, `validation/v2180_model_run_results_audit.py`, `docs/V2_18_0_MODEL_RUN_RESULTS_MOCKUP_ALIGNMENT.md`, and registry gate `v2_18_0_model_run_results_mockup_alignment`.
- Updated release metadata to `2.18.0` and artifact labeling to `v2_18_0_model_run_results_mockup_alignment`.
- Boundary: frontend/product mockup alignment only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.17.0 Home, Data, And Setup Mockup Alignment

- Added v2.17 screen markers and mockup-alignment classes to Home, Data, and Setup workspaces.
- Tightened the Home launcher into a denser desktop project command center with compact current-workspace, project-action, workflow, and status sections.
- Tightened the Data workspace into a mockup-style import/quality/preview/metadata workbench with denser cards and a dominant table region.
- Tightened the Setup workspace into a mockup-style method setup screen with compact selected calculation, readiness, method browser, sidecar requirements, presets, and run handoff.
- Added `validation/v2170_home_data_setup_smoke.mjs`, `validation/v2170_home_data_setup_audit.py`, `docs/V2_17_0_HOME_DATA_SETUP_MOCKUP_ALIGNMENT.md`, and registry gate `v2_17_0_home_data_setup_mockup_alignment`.
- Updated release metadata to `2.17.0` and artifact labeling to `v2_17_0_home_data_setup_mockup_alignment`.
- Boundary: frontend/product mockup alignment only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.16.0 Desktop Shell Visual Contract

- Added the QuickPLS 2.0 desktop shell foundation around the existing workspaces: compact title strip, React-rendered `File`, `Edit`, `Data`, `Model`, `Calculate`, `Results`, `Report`, `View`, and `Help` menu bar, and a tighter command strip.
- Added a frontend-only desktop dialog manager surface for New Project, Open Project guidance, Import Data, Export Options, Calculation Setup, Method Scope / Trust Evidence, Settings, and Help / Shortcuts.
- Added UI-only store/type state for active desktop menu/dialog handling without changing project archives, analysis recipes, result schemas, or numerical fingerprints.
- Added native-desktop visual tokens for menu popovers, command strip buttons, modal windows, form grids, status notices, and compact rail density.
- Added `validation/v2160_desktop_shell_smoke.mjs`, `validation/v2160_desktop_shell_audit.py`, `docs/V2_16_0_DESKTOP_SHELL_VISUAL_CONTRACT.md`, and registry gate `v2_16_0_desktop_shell_visual_contract`.
- Updated release metadata to `2.16.0` and artifact labeling to `v2_16_0_desktop_shell_visual_contract`.
- Boundary: frontend/product shell foundation only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.10.0 Results/Report Research Table Pass

- Added v2.10 research-table affordances to Results tables: captions, row/column context, scan guidance, copy, and per-table CSV export.
- Added v2.10 report-preview table affordances: row/column context, preview status, copy, and per-table CSV export.
- Added rendered smoke coverage for Results and Report with completed sample runs at `1440x900` and `1280x800`.
- Added `validation/v2100_results_report_tables_smoke.mjs`, `validation/v2100_results_report_tables_audit.py`, `docs/V2_10_0_RESULTS_REPORT_RESEARCH_TABLE_PASS.md`, and registry gate `v2_10_0_results_report_research_table_pass`.
- Updated release metadata to `2.10.0` and artifact labeling to `v2_10_0_results_report_research_table_pass`.
- Boundary: frontend/product Results and Report table pass only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.0 Navigation Hierarchy Polish

- Split the left rail into a primary `Research workflow` group and a separated `Support` utility group.
- Kept Home, Data, Model, Setup, Run, Results, and Report as the main researcher flow.
- Kept Trust and Settings available as support destinations without mixing them into the primary research sequence.
- Added rendered smoke and static audit coverage for the grouped rail, route behavior, current version metadata, and frontend-only boundary.
- Updated release artifact labeling to `v2_5_0_navigation_hierarchy_polish`.

## v2.4.1 QuickPLS 2 Release Readiness Audit

- Added release-readiness smoke and audit coverage for v2.4.1 version metadata, public docs, screenshot coverage, registry state, artifact labeling, and frontend-only boundaries.
- Updated README, installation, build-from-source, release notes, top-bar version copy, package metadata, Tauri metadata, and roadmap expectations to the v2.4.1 readiness gate.
- Preserved v2.4.0 as the public documentation refresh milestone while making v2.4.1 the current QuickPLS 2.x release-readiness proof.
- Updated release artifact labeling to `v2_4_1_quickpls_2_release_readiness_audit`.

## v2.4.0 Public Documentation And Screenshot Refresh

- Refreshed README current-release copy, download artifact pattern, screenshot links, v2 documentation links, verification command, and release-file guidance for the QuickPLS 2.x line.
- Refreshed installation and build-from-source guides to avoid stale v1.8.1 asset names and to point at the current v2.4.0 documentation gate.
- Added `docs/screenshots/v2/` from validated v2 smoke screenshots covering Home, Data, SEM Designer, Setup, Run, Results, Report, Trust Center, and Settings.
- Added `validation/v240_public_docs_smoke.mjs`, `validation/v240_public_docs_audit.py`, `docs/V2_4_0_PUBLIC_DOCUMENTATION_SCREENSHOT_REFRESH.md`, and registry gate `v2_4_0_public_documentation_screenshot_refresh`.
- Updated release artifact labeling to `v2_4_0_public_documentation_screenshot_refresh`.

## v2.3.2 Shared UI Verification Harness

- Added shared v2 smoke harness helpers for Vite preview startup/teardown, Playwright shell snapshots, screenshot output, integrity checks, and JSON result writing.
- Added shared v2 static audit helpers for version metadata, command-bar readiness contracts, forbidden text, SmartPLS-equivalence wording, and frontend-boundary checks.
- Migrated the v2.3.1 UI integrity smoke/audit onto the shared helpers and kept the v2.3.1 gate evidence passing.
- Added `validation/v232_shared_ui_harness_smoke.mjs`, `validation/v232_shared_ui_harness_audit.py`, `docs/V2_3_2_SHARED_UI_VERIFICATION_HARNESS.md`, and registry gate `v2_3_2_shared_ui_verification_harness`.
- Updated release artifact labeling to `v2_3_2_shared_ui_verification_harness`.

## v2.3.1 UI Integrity Consolidation

- Updated visible v2 shell milestone copy and release metadata to `2.3.1`.
- Added a rendered v2 shell smoke check for the title bar, workflow strip, Trust/Settings rail entries, command-bar blocker metadata, and rendered text integrity.
- Added a static source/document audit for v2 metadata alignment, artifact label alignment, no mojibake, no stale visible labels, no SmartPLS-equivalence claims, and frontend-only boundaries.
- Added `validation/v231_ui_integrity_smoke.mjs`, `validation/v231_ui_integrity_audit.py`, `docs/V2_3_1_UI_INTEGRITY_CONSOLIDATION.md`, and registry gate `v2_3_1_ui_integrity_consolidation`.
- Updated release artifact labeling to `v2_3_1_ui_integrity_consolidation`.

## v2.3.0 Global Command Bar Readiness

- Added inspectable command-bar run metadata for run state, selected method, disabled reason, blocker id, blocker action, and blocker target.
- Replaced the generic top blocker text with a nearby `Run disabled` chip that exposes exact readiness detail through title and accessibility text.
- Routed blocker-chip navigation through the workflow destination-context store contract with `coachId: "top-command-bar"`.
- Added `validation/v230_command_bar_smoke.mjs`, `validation/v230_command_bar_audit.py`, `docs/V2_3_0_GLOBAL_COMMAND_BAR_READINESS.md`, and registry gate `v2_3_0_global_command_bar_readiness`.
- Updated release metadata to `2.3.0` and artifact labeling to `v2_3_0_global_command_bar_readiness`.

## v2.2.9 Workflow Strip Context Alignment

- Added inspectable `data-workflow-*` metadata to workflow strip step buttons.
- Routed cross-workspace workflow strip clicks through the same destination-context store contract used by coach actions.
- Kept current-step clicks as ordinary navigation so they do not create redundant feedback.
- Added `validation/v229_workflow_strip_smoke.mjs`, `validation/v229_workflow_strip_audit.py`, `docs/V2_2_9_WORKFLOW_STRIP_CONTEXT_ALIGNMENT.md`, and registry gate `v2_2_9_workflow_strip_context_alignment`.
- Updated release metadata to `2.2.9` and artifact labeling to `v2_2_9_workflow_strip_context_alignment`.

## v2.2.8 Workflow Feedback Lifecycle

- Added a shared UI-only `clearWorkflowFeedback` store action.
- Added dismiss controls to workflow coach destination and command feedback notes.
- Cleared stale feedback on ordinary cross-workspace navigation without a new coach context.
- Cleared workflow feedback on dataset replacement, project reset, and project load.
- Added `validation/v228_feedback_lifecycle_smoke.mjs`, `validation/v228_feedback_lifecycle_audit.py`, `docs/V2_2_8_WORKFLOW_FEEDBACK_LIFECYCLE.md`, and registry gate `v2_2_8_workflow_feedback_lifecycle`.
- Updated release metadata to `2.2.8` and artifact labeling to `v2_2_8_workflow_feedback_lifecycle`.

## v2.2.7 Workflow Command Feedback

- Added UI-only workflow command context to the workspace store.
- Updated workflow coach command actions to preserve source action, source workspace, command event, and coach message id.
- Rendered a compact command feedback note after enabled coach-driven command requests.
- Exposed command context through the smoke-only API for deterministic validation.
- Added `validation/v227_command_feedback_smoke.mjs`, `validation/v227_command_feedback_audit.py`, `docs/V2_2_7_WORKFLOW_COMMAND_FEEDBACK.md`, and registry gate `v2_2_7_workflow_command_feedback`.
- Updated release metadata to `2.2.7` and artifact labeling to `v2_2_7_workflow_command_feedback`.

## v2.2.6 Workflow Destination Context

- Added UI-only workflow destination context to the workspace store.
- Updated workflow coach navigation to preserve source action, source workspace, and destination workspace.
- Rendered a compact destination note after enabled coach-driven navigation.
- Exposed destination context through the smoke-only API for deterministic validation.
- Added `validation/v226_destination_context_smoke.mjs`, `validation/v226_destination_context_audit.py`, `docs/V2_2_6_WORKFLOW_DESTINATION_CONTEXT.md`, and registry gate `v2_2_6_workflow_destination_context`.
- Updated release metadata to `2.2.6` and artifact labeling to `v2_2_6_workflow_destination_context`.

## v2.2.5 Workflow Coach Action Execution

- Added explicit `data-action-view` and `data-action-event` metadata to workflow coach buttons.
- Kept v2.2.4 disabled-reason and duplicate-action protections while making action targets directly inspectable.
- Normalized common workflow coach labels such as `Import Data`, `Run Now`, `Run Method`, and `Review Model`.
- Added click-through smoke evidence proving enabled view-target coach actions navigate to their declared workspace and disabled actions remain inert.
- Added `validation/v225_coach_execution_smoke.mjs`, `validation/v225_coach_execution_audit.py`, `docs/V2_2_5_WORKFLOW_COACH_ACTION_EXECUTION.md`, and registry gate `v2_2_5_workflow_coach_action_execution`.
- Updated release metadata to `2.2.5` and artifact labeling to `v2_2_5_workflow_coach_action_execution`.

## v2.2.4 Workflow Coach Action Clarity

- Added stable action metadata to workflow coach buttons through `data-action-label` and `data-action-disabled`.
- Added visible disabled-action reason text with `aria-describedby` wiring so coach actions do not rely only on tooltip text.
- Suppressed duplicate secondary actions when a coach message repeats the primary action.
- Fixed the incomplete Model coach state to offer distinct `Open Setup` and `Open Data` actions.
- Kept command dispatch inside the existing frontend command event contract and delayed event dispatch until after requested workspace navigation.
- Added `validation/v224_coach_actions_smoke.mjs`, `validation/v224_coach_actions_audit.py`, `docs/V2_2_4_WORKFLOW_COACH_ACTION_CLARITY.md`, and registry gate `v2_2_4_workflow_coach_action_clarity`.
- Updated release metadata to `2.2.4` and artifact labeling to `v2_2_4_workflow_coach_action_clarity`.

## v2.2.3 Model Workflow Context

- Added a `model-workflow-band` around the Model workspace so the workflow strip and workspace coach appear above the existing SEM Designer shell.
- Added model-specific workflow coach states for missing data, incomplete diagrams, setup handoff, and run-ready calculation.
- Updated shell styling so Explorer, ModelCanvas, and Inspector stay in their existing grid row below the model workflow context.
- Added targeted rendered smoke and static audit evidence for model workflow context, designer-shell preservation, scoped wording, version metadata, and frontend-only boundaries.
- Added `validation/v223_model_workflow_smoke.mjs`, `validation/v223_model_workflow_audit.py`, `docs/V2_2_3_MODEL_WORKFLOW_CONTEXT.md`, and registry gate `v2_2_3_model_workflow_context`.
- Updated release metadata to `2.2.3` and artifact labeling to `v2_2_3_model_workflow_context`.

## v2.2.2 Workflow Step Clarity

- Added `src/domain/workflowProgress.ts` to derive Data, Model, Setup, Run, Results, and Report step states from dataset, model, readiness, and completed-run state.
- Updated the workflow strip to render explicit `complete`, `current`, `next`, `blocked`, and `ready` states with compact action labels, tooltips, accessible labels, and stable smoke-test markers.
- Reworked workflow strip styling into a desktop six-step grid that avoids horizontal overflow at audited desktop widths.
- Added targeted rendered smoke and static audit evidence for workflow state markers, overflow behavior, version metadata, script wiring, scoped wording, and frontend-only boundaries.
- Added `validation/v222_workflow_step_smoke.mjs`, `validation/v222_workflow_step_audit.py`, `docs/V2_2_2_WORKFLOW_STEP_CLARITY.md`, and registry gate `v2_2_2_workflow_step_clarity`.
- Updated release metadata to `2.2.2` and artifact labeling to `v2_2_2_workflow_step_clarity`.

## v2.2.1 Command Handoff Consistency

- Added `src/domain/workspaceCommands.ts` as the shared frontend command-event contract for run, save, open, demo, and import actions.
- Updated the workflow coach action type and renderer so command actions dispatch through the shared helper instead of local event strings.
- Updated the top bar command listener coverage so the workflow coach can trigger the same production command paths as the visible global buttons.
- Added targeted rendered smoke and static audit evidence for coach handoffs, command-event dispatch, version metadata, script wiring, scoped wording, and frontend-only boundaries.
- Added `validation/v221_command_handoff_smoke.mjs`, `validation/v221_command_handoff_audit.py`, `docs/V2_2_1_COMMAND_HANDOFF_CONSISTENCY.md`, and registry gate `v2_2_1_command_handoff_consistency`.
- Updated release metadata to `2.2.1` and artifact labeling to `v2_2_1_command_handoff_consistency`.

## v2.2.0 Workflow Continuity And Command Clarity

- Added centralized workflow-coach logic that derives next actions from dataset, model, method setup, readiness, run, and report state.
- Rendered the coach across Home, Data, Setup, Run, Results, Report, Trust Center, and Settings while leaving the SEM Designer branch unchanged.
- Added targeted rendered smoke and static audit evidence for workflow continuity, version metadata, script wiring, scoped wording, and frontend-only boundaries.
- Added `validation/v220_workflow_continuity_smoke.mjs`, `validation/v220_workflow_continuity_audit.py`, `docs/V2_2_0_WORKFLOW_CONTINUITY_COMMAND_CLARITY.md`, and registry gate `v2_2_0_workflow_continuity_command_clarity`.
- Updated release metadata to `2.2.0` and artifact labeling to `v2_2_0_workflow_continuity_command_clarity`.

## v2.1.5 Rendered Shell Consistency Audit

- Added a consolidated rendered QA gate across Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings at `1440x900` and `1280x800`.
- Verified the v2 shell keeps workspace markers, the primary rail, desktop top actions, current version text, page-top behavior, and scoped claim boundaries.
- Added `validation/v2115_rendered_shell_consistency_smoke.mjs`, `validation/v2115_rendered_shell_consistency_audit.py`, `docs/V2_1_5_RENDERED_SHELL_CONSISTENCY_AUDIT.md`, and registry gate `v2_1_5_rendered_shell_consistency_audit`.
- Updated release metadata to `2.1.5` and artifact labeling to `v2_1_5_rendered_shell_consistency_audit`.

## v2.1.4 Model/Trust/Settings Shell Alignment

- Completed the v2.1 shell alignment for the remaining Model shell, Trust Center, Settings, and global shell surfaces.
- Kept the SEM designer behavior intact while adding v2.1.4 workspace, toolbar, and overlay-status hooks for targeted visual QA.
- Converted Trust Center to the shared WorkspacePage, PageHeader, Panel, MetricCard, ResearchTable, Card, and StatusBadge primitives.
- Added `validation/v2114_model_trust_settings_shell_smoke.mjs`, `validation/v2114_model_trust_settings_shell_audit.py`, `docs/V2_1_4_MODEL_TRUST_SETTINGS_SHELL_ALIGNMENT.md`, and registry gate `v2_1_4_model_trust_settings_shell_alignment`.
- Updated release metadata to `2.1.4` and artifact labeling to `v2_1_4_model_trust_settings_shell_alignment`.

## v2.1.3 Results/Report Mockup Alignment

- Applied the v2.1 desktop design primitives to Results and Report, so saved-run review, interpretation controls, run confidence, report presets, publication setup, WYSIWYG SVG preview, and export outputs use the same WorkspacePage, PageHeader, Panel, MetricCard, Card, and StatusBadge grammar as the approved mockup direction.
- Preserved existing result table search, copy, export-current-table, interpretation, comparison, native XLSX, CSV, HTML, SVG, and print/PDF-path wiring while reducing ad hoc section wrappers.
- Added `validation/v2113_results_report_mockup_smoke.mjs`, `validation/v2113_results_report_mockup_audit.py`, `docs/V2_1_3_RESULTS_REPORT_MOCKUP_ALIGNMENT.md`, and registry gate `v2_1_3_results_report_mockup_alignment`.
- Updated release metadata to `2.1.3` and artifact labeling to `v2_1_3_results_report_mockup_alignment`.

## v2.1.2 Setup/Run Mockup Alignment

- Applied the v2.1 desktop design primitives to Setup and Run, so method selection, applicability, readiness checks, calculation preview, and launch controls use the same WorkspacePage, PageHeader, Panel, MetricCard, and StatusBadge grammar as the approved mockup direction.
- Preserved the existing analysis event wiring, method applicability rules, run readiness checks, and saved-run handoff while making the Setup/Run surfaces more consistent and less ad hoc.
- Added `validation/v2112_setup_run_mockup_smoke.mjs`, `validation/v2112_setup_run_mockup_audit.py`, `docs/V2_1_2_SETUP_RUN_MOCKUP_ALIGNMENT.md`, and registry gate `v2_1_2_setup_run_mockup_alignment`.
- Updated release metadata to `2.1.2` and artifact labeling to `v2_1_2_setup_run_mockup_alignment`.

## v2.0.11 Mockup Pixel Alignment

- Added a stricter Playwright smoke for the approved mockup-alignment direction, including empty Results, populated shell expectations, Trust entry visibility, version label visibility, and mojibake checks.
- Added `validation/v211_mockup_pixel_alignment_audit.py` to verify screenshot evidence, version metadata, registry state, docs, artifact labels, and claim boundaries.
- Added `docs/V2_0_11_MOCKUP_PIXEL_ALIGNMENT.md` and registry gate `v2_0_11_mockup_pixel_alignment`.
- Updated release metadata to `2.0.11` and artifact labeling to `v2_0_11_mockup_pixel_alignment`.

## v2.0.10 Visual Gap Audit

- Added a Playwright rendered-screen smoke covering Home, Data, Model, Setup, Run, Results, Report, Trust, and Settings at desktop viewports.
- Added `validation/v210_visual_gap_audit.py` to verify screenshot evidence, version metadata, registry state, docs, artifact labels, and claim boundaries.
- Added `docs/V2_0_10_VISUAL_GAP_AUDIT.md` and registry gate `v2_0_10_visual_gap_audit`.
- Updated release metadata to `2.0.10` and artifact labeling to `v2_0_10_visual_gap_audit`.

| Release | Status | Evidence | Open gate or dependency |
| --- | --- | --- | --- |
| v0.1 Foundation | Proven | `src/`, `src-tauri/`, `crates/qpls-core`, `crates/qpls-cli`; current `cargo test --workspace` and `npm test` pass | None for the foundation scope. Later contract expansion must retain migration support. |
| v0.2 Data and project platform | Proven | `crates/qpls-data`, `crates/qpls-project`, and native project commands remain implemented; data/project tests pass. The Data workspace now presents import source, data quality, preview/metadata, sample dataset, and prefix-based construct creation workflows while preserving native import/project ownership. | None for the v0.2 preview scope. Broader format breadth remains future hardening, not a v0.2 blocker. |
| v0.3 PLS core | Proven for estimator scope | `crates/qpls-estimation`, typed contracts, method specification, reference fixtures, eleven estimator tests, cSEM/python-plspm/NumPy comparisons, the documented cSEM `threecommonfactors` fixture, GUI/CLI shared runner parity, estimator-only CLI CSV/HTML export, and the release benchmark are in place. The editor adds select/path tools, click-to-draw and reconnectable paths, quick selected-path reverse/routing controls, batch variable assignment, one-click grouped construct creation by variable prefix, one-click separate single-item construct creation from selected variables, drag-to-create/assign constructs, collision-aware placement, fit-to-view, horizontal/vertical layout, 10px snap-to-grid, selected-construct alignment/distribution, and explicit selected-run diagram estimates that default to model-only when a project is opened. | Full run-envelope publication output remains gated because current runs include experimental v0.4 assessment/inference artifacts. |
| v0.4 Assessment and inference | Implemented, gate open; partial | `qpls-assessment` supplies thirty-five tests and current `pls_assessment_v7` retains experimental typed rho_A and explicit original signed HTMT/HTMT+ artifacts while changing Cohen f-squared to the fixed-score cSEM-compatible contract; `external_reference_probe.py` records whether cSEM can run on the current workstation; `qpls evidence v04-assessment` writes a metric-to-evidence report with tolerances and missing blockers; `qpls-resampling` supplies twenty-one tests and `indexed_resampling_v4` adds compact `nested_studentized_v1` bootstrap-t results with explicit nested-infrastructure failure artifacts. Core has fourteen tests, project twelve, CLI eight, desktop job shell six, and frontend twenty-five. | rho_A now has primary-paper Equation 3 evidence and executed cSEM 0.6.1 agreement. Original HTMT has independent formula and cSEM 0.6.1 agreement. R2, adjusted R2, structural VIF, fixed-score f2, SRMR, and d_ULS have executed cSEM 0.6.1 agreement on both the corporate-reputation fixture and the cSEM satisfaction README fixture. Q2 has an independent NumPy blindfolding-contract comparison plus deterministic generated-data simulation showing Q2/R2/f2 signal and degradation under exogenous-block permutation. HTMT+ has independent formula, metamorphic fixtures, rounded Ringle et al. worked-example evidence, and executed seminr 2.5.0 agreement on the mixed-sign corporate-reputation fixture; cSEM `.absolute=TRUE` remains documented as non-equivalent and cSEM `htmt2` does not return a value for the mixed-sign fixture. Studentized validation now has full 999x99 preregistered normal/heavy-tail simulation coverage, independent-reference agreement, bounded and release-stress performance evidence, and worker-invariance evidence. v0.4 promotion still requires a release-family audit of product, recovery, export, and warning behavior. No v0.4 method is validated. |
| v0.5 Extended PLS models | Implemented, gate clear; experimental | `pls_mediation_v1` now adds descriptive direct/indirect/total/VAF mediation classification to PLS results and saved-run UI, the UI surfaces bootstrap percentile/BCa/bootstrap-t intervals for matching `indirect_effect` parameters when present, `validation/mediation_reference.py` independently confirms a single-item mediation equation fixture, `validation/mediation_r_reference.py` confirms the same fixture against development-only R base `lm`, `validation/mediation_published_example.py` confirms the documented cSEM `threecommonfactors` example with exact path-matrix mediation decomposition agreement, `validation/mediation_metamorphic.py` passes bounded simulation/metamorphic checks, and `validation/mediation_randomization.py` passes a bounded indirect-effect randomization screen. `ModelSpec.interactions`, the desktop two-stage interaction placeholder, experimental `pls_two_stage_moderation_v1` product-score estimation, saved-run moderation tables, and simple-slope output are implemented. `validation/moderation_reference.py` passes independent OLS agreement, complete-case missing-data row mapping, and bounded metamorphic checks for the single-item two-stage moderation path and simple-slope contract; `validation/moderation_r_reference.py` confirms the same fixture against development-only R base `lm`; `validation/moderation_published_formula.py` passes a fixed-table standard moderated-regression formula fixture against independent standardized OLS equations; `validation/moderation_published_empirical.py` passes the 32-row `mtcars` empirical dataset moderation fixture against independent standardized OLS equations; `validation/moderation_simulation.py` passes 20 generated signal and 20 generated null datasets against independent standardized OLS with bounded recovery/null thresholds; `validation/moderation_inference.py` passes bounded bootstrap/BCa/permutation integration and 1-vs-2 worker invariance for the generated product path; `validation/moderation_inference_qualification.py` passes six signal and six null generated datasets through the actual Freedman-Lane product-path permutation pipeline with 6/6 signal detection and 0/6 null false positives at p <= 0.05; `validation/moderation_coverage_qualification.py` passes a heavier 48-run, 199-permutation release-oriented coverage screen with 24/24 signal detections, 1/24 null flags, and independent standardized OLS agreement within `1e-10`. `ModelSpec.controls`, control edge marking, native recipe serialization, and `control_estimates` provide typed control-variable semantics without changing structural path estimation. `ModelSpec.higher_order_constructs`, `docs/methods/PLS_HIGHER_ORDER_V1.md`, and the construct inspector now define and serialize experimental higher-order declarations for repeated-indicator, two-stage, and hybrid workflows; repeated-indicator HOCs are expanded into ordered component-indicator blocks before PLS execution and assessment uses the same expanded execution recipe. `npm run qpls:hoc:reference` independently verifies repeated-indicator HOC paths, loadings, and weights with observed max absolute difference `4.49e-14`; `npm run qpls:hoc:metamorphic` passes affine, row-order, construct-order, component-order, warning, and degradation checks. Two-stage HOCs now use lower-order component scores as generated stage-2 HOC indicators, with assessment support; `npm run qpls:hoc:two-stage` independently verifies generated HOC paths, loadings, and weights with observed reference delta `5.38e-14` and passes bounded metamorphic checks. Hybrid HOCs now use an experimental indicator-split contract, with `npm run qpls:hoc:hybrid-reference` independently verifying hybrid paths/loadings/weights at observed reference delta `1.37e-14` and `npm run qpls:hoc:hybrid-guard` proving one-indicator components are blocked. PLSc now has an experimental reflective-only correction contract with `plsc_v1`, typed correction payload, independent Python reference agreement at observed max delta `4.57e-14`, and invalid-settings guards. Gaussian-copula endogeneity now has an experimental diagnostic contract with `gaussian_copula_endogeneity_v1`, typed copula diagnostics, independent Python reference agreement at observed max delta `5.54e-09`, and applicability warnings. Nonlinear effects now have an experimental fixed-score quadratic diagnostic contract with `pls_quadratic_nonlinear_effects_v1`, typed quadratic diagnostics, independent Python reference agreement at observed max delta `1.96e-12`, and R2 delta warnings. Moderated mediation now has an experimental conditional indirect-effect contract with `pls_moderated_mediation_v1`, typed conditional indirect effects, independent Python reference agreement at observed delta `4.67e-14`, and invalid-recipe guards. CTA-PLS now has an experimental tetrad diagnostic with independent Python reference agreement at observed max delta `4.94e-14`; WPLS now has an experimental positive case-weighted estimator with independent Python reference agreement at observed max delta `3.41e-13`; CCA now has an experimental composite residual diagnostic with independent Python reference agreement at observed max delta `3.51e-14`. | None for the current experimental v0.5 preview scope. Methods remain experimental and watermarked until future validation promotion criteria are met. |
| v0.6 Prediction and heterogeneity | Implemented, gate clear; MICOM withdrawn | `plspredict_holdout_v1`, `plspredict_repeated_kfold_v1`, CVPAT diagnostics, `ipma_v1`, `pls_pos_v1` with backward-compatible `pls_pos_bounded_v1`, `fimix_pls_v1`, `pls_mga_two_group_v1`, and `pls_mga_permutation_v1` remain implemented with GUI/CLI/report surfacing. The historical `micom_v1` schema remains readable, but new MICOM execution is blocked because the former compositional-invariance routine was scientifically invalid. | Reimplement MICOM with group-specific original/permuted weights and independently validate it before any execution or promotion claim. |
| v0.7 CB-SEM beta | Implemented, gate clear; experimental | `AnalysisMethod::Cbsem` emits `cfa_ml_v1`/`cbsem_ml_v1` with direct single-group ML optimization for bounded raw-data reflective CFA/SEM models, `cbsem_fit_v1`, residual matrices, lavaan-parity standardized estimates, expected-information SE/z/p values, modification-index screening, optional `cbsem_bootstrap_v1`, optional `cbsem_multigroup_v1`, desktop settings/saved-run surfacing, report tables, CLI experimental export rows, `qpls evidence v07-cbsem`, method specs, `validation/results/cbsem_v07_reference_report.json`, and `validation/results/cbsem_lavaan_reference_report.json`. | None for the current experimental beta scope. Constrained multigroup refits, robust/ordinal/FIML estimators, broader inadmissibility tests, performance qualification, and second-source validation remain required before publication-ready promotion. |
| v0.8 Extended methods | Historical preview; superseded in part | `pca_v1`, `regression_ols_v1`, `regression_logistic_v1`, bounded `regression_process_v1`, legacy `nca_v1`, and bounded `gsca_v1` were introduced with typed payloads and preview evidence. The former `nca_v1` ceiling geometry is not current scientific evidence and remains archive-readable only; current execution and reference qualification use `nca_v2`. | Retain provenance for historical output. Require each standalone workflow to pass current native setup, result, export, and save/reopen acceptance before primary-workbench promotion. |
| v0.1-v0.8 Publication-readiness audit | Proven for documented supported scope | `publication_ready_v0_1_to_v0_8` now tracks promotion blockers in `validation/development_slices.json`; `qpls evidence publication-ready` writes `validation/results/publication_ready_audit.json`; `docs/PUBLICATION_READY_AUDIT.md` defines the promotion standard and R validation runtime. All blocker audit artifacts are present: foundation, data/project, PLS, assessment, inference, extended PLS, prediction/heterogeneity, CB-SEM, extended methods, GUI/diagram, stable export, documentation, and performance/release. | None for the documented supported scope. Future method broadening must add a new spec and audit artifact before public claims are expanded. |
| v0.9 Publication and release candidate | Proven for RC scope | Version metadata is `0.9.0-rc.1`; `docs/RELEASE_NOTES_V0_9_RC1.md`, `docs/SUPPORTED_SCOPE_V0_9_RC1.md`, and `docs/DEPENDENCY_NOTICES.md` define the bounded release scope; `validation/v09_smoke_check.py` verifies release launch, fixture import/run/export, diagram estimate visibility, SVG export path, browser print/PDF path, and recovery coverage; `validation/v09_release_candidate_audit.py` verifies publication audit currency, release binary, NSIS artifact, docs, exports, and the registry gate. | Code signing remains outside this unsigned RC. Add a certificate and run a separate signing audit before public installer distribution. |
| v0.9.3 Professional SEM designer | Proven for designer scope | `docs/PROFESSIONAL_SEM_DESIGNER_V0_9_3.md`, `src/domain/diagramGraph.ts`, `src/store.ts`, `src/components/ModelCanvas.tsx`, and `src/domain/publicationDiagram.ts` add saved SEM designer layout metadata, editable academic SEM styling, indicator dragging, right-click object actions, locked result/publication modes, and current-canvas SVG export parity. `validation/v093_sem_designer_audit.py` and `validation/v093_sem_designer_visual_smoke.mjs` provide source and browser-smoke evidence. | Residual/error and caption tools are designer placeholders only until separate CB-SEM recipe semantics are implemented and audited. |
| v1.0 Stable | Proven for documented supported scope | Version metadata is `1.0.0`; `docs/V1_SUPPORTED_SCOPE.md`, `docs/V1_COMPATIBILITY_MATRIX.md`, `docs/V1_KNOWN_DIFFERENCES.md`, `docs/METHODOLOGY_MANUAL_V1_0.md`, `docs/VALIDATION_ARTIFACT_INDEX_V1_0.md`, `docs/RELEASE_NOTES_V1_0.md`, `docs/INSTALLATION_V1_0.md`, and `docs/DEPENDENCY_NOTICES_V1_0.md` define the stable release boundary. `validation/v10_numerical_discrepancy_audit.py`, `validation/v10_product_scope_audit.py`, `validation/v10_desktop_smoke_check.py`, `validation/v10_performance_audit.py`, and `validation/v10_release_packaging_audit.py` write final release evidence, and the registry gate is `v1_0_stable`. | Code signing remains outside v1.0 unless a certificate and separate signing audit are added. Native PDF/PNG, WLSMV/polychoric/FIML, SmartPLS project import, and expanded designer residual/caption recipe semantics remain post-v1. |

> PLSc correction (2026-08-10): the historical v0.5 row above records the evidence claim as it existed then. That claim is superseded. `plsc_v1` used a provisional reliability expression and its Python reference repeated that expression; it was not independent Dijkstra-Henseler rho_A evidence. Current `plsc_v2` uses the shared canonical Equation 3 kernel and `PLSC_V1.md` is legacy-only.

v0.5 product note: saved-run result panels now display PLSc corrections, WPLS weight metadata, CCA residuals, CTA-PLS tetrads, Gaussian-copula diagnostics, nonlinear diagnostics, and moderated-mediation conditional effects. The analysis catalog and top-bar run selector now persists a typed method choice, pass it into native recipes, and expose a WPLS case-weight column selector. Export-safe method tables now support watermarked Reports workspace CSV/HTML/XLSX downloads and `qpls export --include-experimental` rows while the default CLI export remains conservative. The Reports workspace also includes a saved-run comparison view for R2 and path coefficients, deterministic publication SVG diagrams from model layout and selected-run estimates, and a browser print/PDF report path. `qpls evidence v05-extended-pls` now aggregates 27 v0.5 reports with all listed artifacts present and passed. The current v0.5 experimental preview gate is clear; publication-validated promotion remains a later, stricter status.

Current focused verification: core 17 tests, assessment 35, estimation 18 passed plus one intentionally ignored release benchmark, resampling 21, project 12, CLI 8, desktop job shell 6, and frontend 31. The production build, independent Decimal rho_A runner, cSEM rho_A comparison, independent HTMT/HTMT+ fixture runner, seminr HTMT+ comparison, assessment simulation, published assessment fixture, assessment evidence report generator, full Monte Carlo qualification, normal/heavy-tail sensitivity Monte Carlo drift screen, bounded minimum 999x99 studentized execution, mediation validation scripts including R base-lm second source, published/example evidence, and indirect-effect randomization evidence, moderation validation scripts including R base-lm second source, published-formula evidence, published empirical-data evidence, bounded simulation, moderation inference, bounded inference qualification, release-oriented coverage qualification, higher-order schema/editor/repeated-indicator/two-stage/hybrid estimator plus independent Python reference/metamorphic validation, PLSc reflective-only reference/guard validation, Gaussian-copula endogeneity reference validation, nonlinear quadratic-effects reference validation, and moderated mediation reference validation pass.

Validation-loop speed note: moderation reference helpers now reuse the built `target/debug/qpls.exe` CLI instead of invoking `cargo run` for each import/inspect/run operation. `npm run qpls:moderation:validate` completed in 17.87 seconds on 2026-07-19 while preserving all reported validation checks.

HOC validation speed note: higher-order reference helpers now reuse the built `target/debug/qpls.exe` CLI instead of invoking `cargo run` for each import/inspect/run operation. `npm run qpls:hoc:validate` covers repeated-indicator, metamorphic, two-stage, and hybrid-reference checks; `npm run qpls:hoc:hybrid-guard` separately proves invalid hybrid component splits are rejected.

HOC safety note: hybrid higher-order constructs now use an experimental indicator-split contract: each lower-order component keeps the first indicator partition and the HOC receives the remaining component indicators. `qpls-core` emits `higher_order.hybrid_component_indicators` for components that cannot be split, and `npm run qpls:hoc:hybrid-guard` proves CLI validation/run rejection for that invalid recipe.

Frontend bundle note: Vite manual chunks now split React Flow, icons, Tauri bindings, and remaining vendor code so `npm run build` completes without the previous large-chunk warning. The largest generated JS asset on 2026-07-19 was `vendor-CtbQ7rgw.js` at 317.20 kB before gzip.

R validation runtime note: `validation/r_runtime.py` and `validation/Resolve-Rscript.ps1` now discover the local portable `Documents\PLS-Sem\dist-desktop\r-runtime` layout before standard Windows R installs, registry entries, and PATH. R remains a development-only reference runtime and is not a QuickPLS application dependency.

PLSc implementation note: `docs/methods/PLSC_V2.md` defines the current reflective-only correction contract. `qpls-estimation` emits `plsc_v2` using the canonical Dijkstra-Henseler rho_A equation shared with `qpls-assessment`, while `docs/methods/PLSC_V1.md` records why v1 is legacy-only. `npm run qpls:plsc:validate` writes `validation/results/plsc_reference_report.json` and `validation/results/plsc_unsupported_guard_report.json`; the reference uses an independent NumPy implementation of Equation 3, and formative PLSc recipes are rejected with `plsc.reflective_only`.

Gaussian-copula endogeneity note: `docs/methods/PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md` defines the experimental rankit inverse-normal copula diagnostic contract. `qpls-estimation` emits `gaussian_copula_endogeneity_v1` with augmented-regression copula coefficients, standard errors, t statistics, p values, predictor skewness, applicability flags, and warnings. `npm run qpls:endogeneity:reference` writes `validation/results/endogeneity_reference_report.json`; the reference fixture currently matches within observed max delta `5.54e-09`.

Nonlinear effects note: `docs/methods/PLS_NONLINEAR_EFFECTS_V1.md` defines the experimental fixed-score quadratic diagnostic contract. `qpls-estimation` emits `pls_quadratic_nonlinear_effects_v1` with centered squared construct-score coefficients, standard errors, t statistics, p values, fixed-score linear/augmented R2, delta R2, and warnings. `npm run qpls:nonlinear:reference` writes `validation/results/nonlinear_effects_reference_report.json`; the reference fixture currently matches within observed max delta `1.96e-12`.

Moderated mediation note: `docs/methods/PLS_MODERATED_MEDIATION_V1.md` defines the experimental conditional indirect-effect contract. `qpls-estimation` emits `pls_moderated_mediation_v1` with first-stage and second-stage mediated path mapping, conditional indirect effects at standardized moderator scores -1/0/+1, index of moderated mediation, and warnings. `npm run qpls:moderated-mediation:reference` writes `validation/results/moderated_mediation_reference_report.json`; the reference fixture currently matches within observed delta `4.67e-14` and proves interaction-free recipes are rejected with `moderated_mediation.interaction_required`.

CTA-PLS note: `docs/methods/PLS_CTA_PLS_V1.md` defines the experimental sample-covariance tetrad diagnostic contract. `qpls-estimation` emits `cta_pls_tetrad_v1` with typed tetrad estimates, pairing identifiers, max absolute tetrad by construct, and warnings. `npm run qpls:cta:reference` writes `validation/results/cta_pls_reference_report.json`; the reference fixture currently matches within observed max delta `4.94e-14` and proves recipes without a four-indicator block are rejected with `cta_pls.tetrad_block_required`.

WPLS note: `docs/methods/PLS_WPLS_V1.md` defines the experimental positive case-weighted PLS contract. `qpls-estimation` emits `wpls_case_weighted_v1` with typed case-weight metadata, weighted outer weights/loadings, weighted structural paths, weighted R2, and warnings. `npm run qpls:wpls:reference` writes `validation/results/wpls_reference_report.json`; the reference fixture currently matches within observed max delta `3.41e-13` and proves missing weight-column recipes and negative case weights are rejected.

CCA note: `docs/methods/PLS_CCA_V1.md` defines the experimental recursive composite correlation residual contract. `qpls-estimation` emits `cca_composite_residual_v1` with observed, reproduced, residual, absolute residual, max residual, and warnings. `npm run qpls:cca:reference` writes `validation/results/cca_reference_report.json`; the reference fixture currently matches within observed max delta `3.51e-14` and proves PCA weighting is rejected with `cca.pca_unsupported`.

Extended PLS guard note: all schema-recognized v0.5 extended PLS method identifiers now have experimental estimator or diagnostic contracts. `npm run qpls:extended-pls:unsupported-guard` remains as an empty regression guard and writes `validation/results/extended_pls_unsupported_guard_report.json` with zero pending unsupported methods.

Desktop catalog alignment note: `src/data/sample.ts` now lists `plsc`, `wpls`, `cca`, `endogeneity`, `nonlinear_effects`, `moderated_mediation`, and `cta_pls` as experimental PLS-SEM methods. `src/store.test.ts` guards the same visible availability policy.

## v0.3 Gate Audit

### Proven

- Recursive raw-data PLS-PM executes Mode A, Mode B, single-item blocks, path/factor inner weighting, and PCA block weighting.
- Estimation rejects cycles, constants, duplicated/unknown indicators, duplicate structural paths, insufficient complete observations, rank-deficient regressions, isolated constructs, and non-convergence. Core recipe validation and frontend model validation both reject directed structural cycles before execution.
- Scores, outer weights/loadings, structural paths, direct/indirect/total effects, R2, transforms, iteration counts, and used/omitted row counts are serialized.
- Fixed-input reruns are exactly equal in the estimator test.
- Indicator order, construct order, and positive affine transformation metamorphic checks exist.
- `validation/results/pls_csem_comparison.json` records executable cSEM 0.6.1 agreement for path Mode A, Mode B, factor, and PCA. `validation/results/pls_plspm_comparison.json` records python-plspm 0.5.7 agreement for path Mode A, Mode B, and factor loadings/paths. `validation/results/pls_pca_numpy_comparison.json` records independent NumPy eigensystem agreement for PCA paths, loadings, and weights. `validation/results/pls_csem_threecommonfactors_comparison.json` records executable agreement on the documented cSEM `threecommonfactors` example. The maximum absolute differences are below `1e-6`; reference breadth is closed for the current v0.3 estimator scope.
- The 100,000-row/300-indicator/100-construct estimator benchmark has recorded hardware, time, and memory.
- The native GUI and CLI both call `qpls_estimation::estimate_pls`.
- `AnalysisMethod` has a stable typed wire representation and rejects unknown identifiers.
- Schema-versioned `AnalysisResult`, `RunProvenance`, and `Diagnostic` bind completed desktop/CLI output to recipe ID, full dataset fingerprint, method and engine versions, settings, seed, and timestamps. CLI and desktop now share `qpls-runner` for result assembly, and `quickpls-desktop::desktop_runner_payload_matches_cli_serialized_artifact` proves canonical payload equality within `1e-12` against the CLI artifact.
- Project archive schema v4 stores tagged results and migrates schema-v3 and legacy untyped payloads with explicit provenance handling.
- CLI recipe selection and fingerprint-based dataset selection avoid silently using the first archive entries.
- Estimator iteration/assembly checkpoints support cooperative cancellation without changing the ordinary deterministic API.
- CLI `export` writes validated v0.3 estimator-only CSV/HTML tables from completed PLS envelopes and rejects legacy payloads. Assessment and resampling fields are deliberately excluded.

### Open

No open v0.3 estimator gate remains. Carry-over hardening is tracked under v0.4 and later:

- Full run-envelope publication output remains gated for experimental v0.4 assessment/inference artifacts.
- Method-specific settings should replace the remaining shared settings structure before unrelated estimator families use the contract.
- Additional randomized property, sign-orientation, high-collinearity, and UI responsiveness tests remain useful hardening but no longer block the v0.3 estimator gate.

## v0.4 Work Packages

Work packages are dependency ordered. A later package may start in parallel only after its input contracts are frozen.

### WP0: v0.3 Carry-Forward Hardening

- Typed `AnalysisMethod` is implemented with a stable wire format; split the remaining shared settings into versioned method-specific settings before unrelated estimator families use the contract.
- Schema-versioned `AnalysisResult`, `RunProvenance`, and `Diagnostic` are implemented; `ResultTable` and artifact references remain pending.
- Result provenance now binds recipe ID, dataset fingerprint, method/engine version, settings, seed, timestamps, warnings, and completion state.
- Explicit recipe selection and fingerprint-based dataset selection are implemented for CLI and desktop execution.
- Stable CLI export boundaries for validated estimator-only output versus experimental assessment/inference output are implemented for CSV/HTML.
- Keep the published cSEM `threecommonfactors`, cSEM/plspm variant, NumPy PCA, and canonical GUI/CLI serialization parity tests green as regression evidence.

### WP1: Freeze Assessment and Inference Specifications

Create versioned specifications before implementation for:

- Cronbach alpha, rho_A, rho_C, AVE, indicator and construct VIF, cross-loadings, Fornell-Larcker, HTMT and HTMT+.
- Adjusted R2, f2, Q2, SRMR, d_ULS, d_G, NFI, RMS_theta.
- Bootstrap, jackknife, permutation, percentile, studentized, and BCa intervals.
- Applicability by reflective/formative/single-item block, standardized/unstandardized input, missing-data behavior, zero/negative correlations, singular matrices, sign alignment, convergence failure, and undefined statistics.
- Exact formulas, matrix denominators, diagonal inclusion, finite-sample corrections, defaults, thresholds as contextual warnings rather than universal truth, citations, and known reference-engine differences.

Resolve the roadmap dependency explicitly: v0.4 Q2 requires a frozen blindfolding/cross-validated-redundancy algorithm even though broader prediction is v0.6. Implement only the Q2 fold/omission primitive now; keep PLSpredict/CVPAT in v0.6.

`docs/methods/RESAMPLING_ENGINE_V4.md` is the current frozen experimental resampling contract. It inherits v3 bootstrap/BCa behavior and adds compact `nested_studentized_v1` bootstrap-t inference defined in `STUDENTIZED_BOOTSTRAP_V1.md`. Resampling v1-v3 remain readable and cannot carry v4-only studentized results. BCa, studentized, and permutation all retain publication qualification gates.

`docs/methods/PLS_ASSESSMENT_V4.md` freezes the v4 fit/blindfolding additions, `docs/methods/PLS_RHO_A_V1.md` freezes the rho_A addition introduced in v5, and `docs/methods/PLS_HTMT_V1.md` freezes the explicit HTMT/HTMT+ addition introduced in v6. Current v7 changes f-squared to the fixed-score reduced structural regression convention. Project validation accepts assessment v1-v7 with explicit capability gates: v3 owns f-squared, v4 adds fit/blindfolding, v5 adds rho_A, and v6/v7 can carry the two typed HTMT artifacts.

### WP2: Assessment Engine (partial implementation)

- Dedicated `qpls-assessment` crate provides typed alpha, rho_A, rho_C, AVE, cross-loadings, Fornell-Larcker, original HTMT, HTMT+, R2, adjusted R2, inner-model VIF, formative-indicator VIF, Cohen f-squared, blindfolding Q2, saturated/estimated SRMR and d_ULS, applicability warnings, and thirty-five tests. Current `pls_assessment_v7` owns fixed-score f-squared output; v6 introduced explicit HTMT artifacts, v5 introduced rho_A, v3 introduced f-squared, and v4 owns fit/blindfolding for compatibility.
- rho_A now has an independent standard-library Decimal equation runner and committed exact scalar results, primary-paper Equation 3 fixture evidence, plus signed-weight, improper-range, numerical-boundary, zero-denominator/variance, nonfinite/constant, loading-mismatch, path/factor, all-preprocessing, affine, permutation, listwise-missing, applicability, and archive-tamper fixtures. The reflective cSEM/manual evidence producer is executed through `npm run qpls:rho-a:csem`; `validation/results/rho_a_csem_comparison.json` shows QuickPLS agreement with cSEM 0.6.1 within `4.440892098500626e-16`.
- Production project save/load requires every typed PLS result to resolve its immutable recipe, match persisted settings and construct order, and satisfy rho_A mode/weighting/count applicability plus exact warning semantics. Recipe-free compatibility validation is test-only.
- VIF uses a cancellation-aware predictor-correlation system and bounded p-by-p solve. Nested VIF progress is monotonic with a stable phase total, including cancellation checkpoints during large correlation passes.
- Cohen f-squared removes one unique directed path at a time while retaining the same data, settings, other paths, and isolated measurement blocks. If the target becomes predecessor-free, the reduced model uses intercept-only excluded R2 of zero. Reduced-estimator cancellation is forwarded, nested progress is monotonic, failures produce explicit unavailable values and warnings, and result order follows recipe path order.
- Blindfolding deterministically chooses and stores omission distance D and its settings, then stores PRESS, SSO, and Q2 for reflective endogenous constructs. Every omitted cell is addressed once, reduced rounds forward cancellation, and nested progress is monotonic. Formative or numerically unavailable targets return explicit unavailable values and warnings. This is an in-sample predictive-relevance diagnostic, not out-of-sample predictive performance. `validation/results/blindfolding_python_comparison.json` records independent NumPy agreement for Q2/PRESS/SSO on the corporate-reputation fixture. `validation/results/assessment_simulation_report.json` records generated-data evidence that Q2 remains positive for the true model and degrades after exogenous-block permutation.
- Correlation-residual fit reports saturated and estimated SRMR and d_ULS. These are descriptive discrepancy measures without a universal cutoff or model-fit test. d_G, NFI, and RMS_theta are deliberately excluded until their positive-definite, null-model, and outer-residual contracts are frozen.
- Original HTMT and HTMT+ now have an independent Python formula fixture for the corporate-reputation data, positive-affine invariance, individual sign reversal, construct reordering, and HTMT+ values above one. `validation/results/htmt_csem_comparison.json` proves original signed HTMT agreement with cSEM 0.6.1 and records the non-equivalence of cSEM `.absolute=TRUE` for Ringle et al. HTMT+ on mixed-sign cross-block correlations. `validation/results/htmt_seminr_comparison.json` proves HTMT+ agreement with seminr 2.5.0 on the same mixed-sign fixture. `validation/results/htmt_published_ringle_2023.json` adds rounded Ringle et al. HTMT+ worked-example evidence. Add simulation agreement for HTMT+, adjusted R2, both VIF families, Cohen f-squared, Q2, SRMR, and d_ULS before treating these families as complete.
- Produce typed measurement, structural, discriminant-validity, model-fit, and diagnostic result sections.
- Correlation, regression/VIF, and the correlation-residual primitives required for current SRMR/d_ULS are implemented. Positive-definite geodesic, null-model, and outer-residual primitives required for d_G, NFI, and RMS_theta remain pending.
- Return typed undefined/not-applicable diagnostics rather than NaN or silently omitted cells.
- Add hand-calculated unit fixtures, singular/near-singular cases, indicator/construct reorder tests, and reference tables from at least two independent sources where available.

### WP3: Deterministic Resampling Engine

- Dedicated `qpls-resampling` now derives indexed ChaCha20 streams from `(master_seed, operation, replicate_index)` and returns outcomes in strict replicate-index order.
- Generic bootstrap and Arrow-backed PLS bootstrap are implemented. The PLS runner aligns replicate signs to original construct scores; its internal run records each outcome, while the persisted PLS result stores a usable count, failed-replicate details, and inferential summaries rather than every successful raw estimate.
- Raw outcomes and percentile summaries are exactly equal for 1 and 4 workers in crate tests. An automated CLI test runs the same fixture with worker overrides of 1 and 4 and proves exactly equal analytical payloads and diagnostics while execution provenance retains the differing worker counts. Extend proof to 2 and maximum workers.
- Bootstrap requires at least 90% usable replicates (and at least two), then reports original value, bootstrap mean, bias, sample standard error, and Type-7 percentile bounds for weights, loadings, paths, effects, and R2. These v3 conventions are inherited by current `RESAMPLING_ENGINE_V4.md`; independent reference, simulation, and coverage qualification remain open.
- Bootstrap parameter identities use canonical JSON tuples. Current `indexed_resampling_v4` persists compact BCa and optional studentized summaries; resampling v1-v3 remain readable only without fields introduced by later versions.
- Generic deterministic `indexed_jackknife_v1` executes one typed outcome per omitted case in fixed case order, with serialized monotonic progress, cooperative cancellation, and exact worker-count invariance. The transient PLS adapter uses the fixed complete-case sample, aligns construct orientation, stores canonical parameter maps rather than full estimators, and is exactly equal at one versus four workers in current tests.
- BCa consumes all-success transient `indexed_jackknife_v1` estimates. Bias correction uses a clamped midrank tie proportion, acceleration uses all delete-one values, and adjusted tails select Type-7 bootstrap quantiles. Degenerate parameters persist a complete row with nullable BCa numerics and an explicit unavailable reason while percentile inference remains available.
- Bootstrap and jackknife report distinct progress phases. Current v4 primary and BCa results are exactly equal at one versus four workers in current tests. Pseudo-values and standalone jackknife bias/variance remain outside this slice.
- `freedman_lane_permutation_v1` is independently enabled with `permutation_samples = 0` or `99..10000`; it does not depend on bootstrap count. For each focal path it holds the original construct scores fixed, fits the nuisance-predecessor model with an intercept, permutes its residuals, reconstructs the target, and refits the full structural equation. It is not measurement-model re-estimation, MGA, or MICOM.
- Each path/permutation index uses an independently domain-separated ChaCha20 stream derived from seed, canonical path identity, and replicate index. Column-pivoted QR reproduces the original full-model path statistic before testing. Two-sided exceedances use absolute coefficients and the finite-sample correction `(exceedances + 1) / (P + 1)`.
- Permutation cancellation discards partial output, progress is monotonic, path/replicate ordering is stable, and complete results are exactly equal at one versus four workers. Dedicated Python/R arithmetic, exact deterministic-index and worker-invariance boundaries, calibrated paired homoscedastic Gaussian null/power scenarios, strict archive/tamper checks, focused frontend/type checks, three-viewport visual acceptance, genuine packaged cancellation/retry/completion, native XLSX, explicit save/reopen, and clean resource/process evidence now pass. Qualification remains limited to the documented fixed-score, exchangeable-residual contract.
- `indexed_resampling_v4` optionally performs nested bootstrap-t inference when `studentized_inner_samples > 0`. It requires at least 999 requested primary replicates and an odd inner count from 99 through 999; zero disables it. Inner streams are domain-separated by master seed, primary index, and inner index, and each inner solution aligns to its immediate primary parent.
- Studentized summaries use primary-bootstrap standard error, inner sample standard errors, Type-7 pivot quantiles, and reversed-tail bounds. At least 90% of requested inner fits are required per pivot and 90% of requested primary pivots per parameter. Degenerate or insufficient cases retain nullable rows with stable reasons while percentile and BCa results remain available.
- Persistence is compact: raw primary/inner fits and pivot vectors remain transient. Enabling studentization leaves primary samples, percentile, BCa, and permutation output unchanged. Nested progress reports against requested `B * M` fits and forwards cancellation.
- Estimator and assessment kernels now expose cooperative checkpoints, and bootstrap checks cancellation at replicate and estimator boundaries while reporting replicate progress. Cancellation discards the partial bootstrap run.
- The desktop job shell now covers queued/running/cancelling/committing/completed/failed/cancelled states, a four-active-job cap, panic finalization, consuming result retrieval, explicit dismissal, bounded terminal retention, project-identity/read-only commit guards, and lock-order-safe commit. Four orchestration tests cover cancellation-before-commit, commit guards, result consumption/retention, and registry responsiveness while waiting for the project lock.

### WP4: Inference and Intervals

- Bootstrap estimates, bias, standard errors, percentile intervals, `t = original / bootstrap SE`, and two-sided standard-normal reference p values are implemented. The t/p convention is unavailable for effectively zero standard error and is not a studentized-bootstrap interval or resampling-under-the-null test. `validation/results/monte_carlo_qualification.json` passes the preregistered bivariate-normal coverage/type-I/bias thresholds for percentile, BCa, and normal-reference summaries; broader non-normal, small-sample, multi-indicator, multiple-predictor, indirect-effect, R2, and performance scenarios remain open.
- Nested studentized/bootstrap-t intervals are implemented under the frozen v1 contract. `validation/results/studentized_supplied_reference.json` proves supplied-value bootstrap-t formula agreement between independent Python and R Type-7 calculations within about `3.6e-15`; it also records that R `boot::boot.ci(type="stud")` uses a non-equivalent finite-replicate endpoint convention for this fixture. `validation/results/pls_bootstrap_external_reference.json` proves PLS-integrated matched-resample path/loading/weight estimates and aggregate bootstrap mean, sample-SE, and Type-7 percentile summaries against cSEM across path Mode A, Mode B, factorial, and PCA variants on the bounded reflective fixture, with maximum replicate difference about `2.9e-10` and maximum summary difference about `2.3e-10`. `validation/results/pls_bootstrap_corporate_csem_reference.json` adds broader corporate-reputation cSEM evidence over 8 accepted matched resamples for a 4-construct/9-indicator/3-path model, with maximum replicate difference about `3.08e-8` and maximum summary difference about `2.54e-8`. `validation/results/pls_bootstrap_plspm_external_reference.json` adds an independent python-plspm matched-resample fixture across path Mode A, Mode B, and factorial path/loading summaries, with maximum replicate and summary differences about `3.5e-7`; weights are excluded because python-plspm uses a different normalization convention. `validation/results/monte_carlo_sensitivity.json` includes ordinary percentile/BCa/normal-reference drift scenarios for normal and standardized t(3) errors at beta=.35 and beta=0, with 96 completed simulations per scenario and zero failures. The Monte Carlo harness exposes `studentized-qualification`, a preregistered mode requiring 1,000 simulations per normal/heavy-tail scenario, 999 outer replicates, and 99 inner studentized replicates; deterministic sharding is provided by `--scenario`, `--simulations`, and `--simulation-offset`; `validation/plan_studentized_qualification_shards.py` creates the 40-shard full manifest; `validation/run_studentized_qualification_shards.py` provides safe-by-default resumable status, dry-run, filtered, and explicit `--execute` limited batch execution; and `validation/aggregate_studentized_qualification.py` recomputes combined counts/rates/checks into `validation/results/monte_carlo_studentized_qualification.json` while ignoring non-manifest smoke artifacts. `validation/results/studentized_qualification_shards/status.json` now reports 40 complete full-manifest shards. `validation/results/monte_carlo_studentized_qualification.json` evaluates the complete preregistered set with `passed=true`: `coverage_beta_0_35`, `null_beta_0`, `heavy_tail_coverage_beta_0_35`, and `heavy_tail_null_beta_0` each reached 1,000/1,000 simulations, zero failures, and full studentized availability; final studentized coverage is `0.964`, studentized type-I/exclusion is `0.03`, heavy-tail studentized coverage is `0.941`, and heavy-tail studentized type-I/exclusion is `0.054`. `validation/results/studentized_release_stress.json` adds release-stress evidence: `maximum_outer_inner_1999x999` requested 1,997,001 inner fits and completed in about 174.25s with peak working set about 61.6 MB, while `broader_corporate_999x199` requested 198,801 inner fits and completed in about 695.02s with peak working set about 43.0 MB and 33 available studentized parameters out of 38. `qpls qualify v04-inference` now reports `qualification_passed=true` and consumes this release-stress artifact. `validation/results/studentized_minimum_quickpls.json` proves a bounded minimum 999x99 run completes and stores a successful compact artifact. `validation/results/studentized_worker_matrix.json` proves exact payload and diagnostics equality for workers 1, 2, 4, and detected max 12 on the bounded 999x99 fixture, with observed elapsed times of about 81.69s, 38.13s, 23.47s, and 12.25s respectively on this machine. `validation/results/v04_inference_qualification_quick.json` proves cancellation from inside the nested studentized-inner phase after one completed inner replicate, with discard latency under the 1s smoke threshold. `validation/results/monte_carlo_studentized.json` and `validation/results/monte_carlo_studentized_sensitivity.json` remain bounded pilot artifacts for diagnostic comparison only; the full preregistered qualification artifact is the promotion evidence.
- Nested infrastructure or schema failures are explicitly stored as a failed studentized artifact with no parameter rows; primary, percentile, and BCa results remain available. Focused failure-path evidence and independent review are complete, while statistical and performance qualification remain open.
- BCa bias correction and acceleration are implemented with frozen midrank, clamp, all-success, degenerate-row, and compact-persistence rules. Publication promotion remains blocked by independent references, Monte Carlo coverage/bias/tail qualification, non-normal and small-sample behavior, the full worker matrix, cancellation latency, and the 10,000-replicate benchmark.
- Freedman-Lane conditional path statistics and plus-one two-sided p values are release-qualified for the documented fixed-score direct-path contract through `validation/results/structural_path_randomization_method_promotion_audit.json`. Probabilities remain pathwise and unadjusted; measurement-model re-estimation, heteroskedastic or broader non-Gaussian validity, MGA, MICOM, and causal claims remain outside qualification.
- Deterministic construct-score sign alignment is implemented for raw PLS bootstrap outcomes. Retain and test it when inferential summaries are aggregated.

### WP5: Product Integration

- Desktop/CLI settings include the independent studentized inner count. UI normalization enforces zero or odd 99..999 and raises primary bootstrap count to at least 999 when studentization is enabled; execution persists a high-cost experimental warning.
- Run history adds bootstrap-t lower/upper columns and the inner replicate count alongside percentile and BCa output, retaining unavailable reasons as cell tooltips. Complete publication-ready inference, comparison, diagnostics, and export-safe result workspaces remain pending.
- Desktop in-kernel/replicate progress, cancellation, committing, failed/completed states, polling, consuming result retrieval, dismissal, and bounded terminal cleanup are implemented. Add completed-with-warning presentation, restart recovery, deterministic retention ordering if required, and proof that runs become durable on disk only after atomic completion.
- CLI and desktop independently accept bootstrap and permutation overrides, execute through the same engines, and emit optional artifacts in the tagged `pls_pm_v3` envelope. Canonical cross-boundary equality remains unproven because IDs and timestamps differ.
- Schema-v4 projects strictly validate optional v4 studentized artifacts: method/version/settings, odd 99..999 inner count, at least 999 primary replicates, 90% policy, stream domain, one row per percentile parameter, count/scale conditions, complete-or-null rows, pivot quantiles, and recomputed reversed-tail bounds. Legacy resampling versions cannot be mislabeled with studentized output. Project coverage remains twelve tests.
- Bootstrap requests receive a dedicated `method.bootstrap.experimental` warning in addition to the base experimental-method warning, preventing the partial inference slice from being mistaken for validated output.
- `README.md` now distinguishes the native Tauri shell from the browser-only preview and gives the exact corporate-reputation and compact deterministic fixture paths, avoiding the earlier native-dialog and CSV-location ambiguity.
- Keep every v0.4 output watermarked experimental until the complete exit gate passes.

### WP6: Validation and Qualification

- Freeze reference package versions and executable runners for cSEM plus at least one independent source appropriate to each statistic.
- Add published data, simulated truth, non-normal, missing, formative, mixed, multicollinear, small-sample, and non-convergent scenarios.
- Exact generic/PLS bootstrap equality, generic/PLS jackknife worker invariance, exact current resampling-v4 BCa equality at one versus four workers, an automated CLI analytical-payload equality test, and bounded 999x99 studentized equality for workers 1, 2, 4, and detected max 12 are implemented. Add independent simulation/coverage evidence.
- `validation/monte_carlo`, `validation/results/monte_carlo_quick.json`, and `validation/results/monte_carlo_pilot.json` are executable infrastructure only. The harness self-check, regenerated quick report, and pilot report pass. Quick mode checks determinism, integration, schema output, and corrected P1 availability gates for BCa, normal-reference probabilities, and usable bootstrap rates. Pilot mode uses 32 simulations per scenario, `n=100`, and 199 bootstrap replicates as a fast early-warning screen. Both record `qualification.evaluated: false` and cannot support accuracy claims.
- Run the full preregistered qualification mode and add dedicated permutation scenarios for conditional type-I error, power, nuisance structures, non-normality, small samples, worker invariance, cancellation, and performance before promotion.
- Extend the completed supplied-value reference, multi-variant matched-resample cSEM/python-plspm fixtures, broader corporate cSEM matched-resample fixture, bounded 999x99 minimum, 1/2/4/detected-max-worker proof, nested cancellation proof, bounded normal/heavy-tail Monte Carlo pilots, compact-fixture minimum/default/outer-stress/maximum-inner benchmark, broader corporate smoke benchmark, full preregistered studentized qualification, and release-stress benchmark into additional preregistered small-sample, multi-indicator, multiple-predictor, indirect-effect, and R2 coverage scenarios before any broader publication-ready claim.
- Run preregistered Monte Carlo scenarios with at least 1,000 replications per cell. For nominal 95% intervals, require empirical coverage in `[0.92, 0.98]`; for nominal 5% null tests, require type-I error in `[0.03, 0.07]`. Require absolute standardized bias no greater than `0.05` in correctly specified baseline scenarios. Any exception needs a method-specific statistical justification recorded before promotion.
- Benchmark 10,000 bootstrap replicates on the documented 8-core Windows machine, recording runtime, peak memory, usable replicates, cancellation latency, and drift across thread counts.

## v0.4 Exit Criteria

v0.4 is complete only when all of the following are true:

1. Every planned statistic and interval has a frozen, cited, versioned specification and an applicability table.
2. All v0.3 contract-debt items in WP0 are closed.
3. Deterministic assessment values match two independent references within `1e-6` when definitions/settings are equivalent and match at four displayed decimals. Every difference is explained in the known-differences register.
4. Fixed-seed resampling is bit-for-bit identical across repeated runs and 1, 2, and maximum configured worker counts, including canonical JSON ordering.
5. Percentile, studentized, and BCa intervals; jackknife; and permutation pass hand fixtures, boundary cases, and the preregistered simulation thresholds.
6. Constant, missing, collinear, singular, non-normal, small-sample, high-dimensional, invalid-model, failed-replicate, and cancellation cases produce specified results or actionable diagnostics without panics, NaN serialization, or partial durable runs.
7. GUI and CLI produce the same canonical serialized result for the same archive, recipe, seed, and worker count.
8. The 10,000-resample benchmark records runtime, memory, cancellation latency, and zero numerical drift across thread counts on documented hardware.
9. Result views expose measurement, structural, quality, inference, fit, and diagnostics sections, and preserve immutable provenance through save/open/migration round trips.
10. Distributed dependency licenses pass review; GPL validation engines remain outside packaged artifacts.
11. `METHOD_COMPATIBILITY.md` remains experimental until all criteria above pass; promotion is blocked by any unexplained discrepancy.

## Critical Path

`WP0 method-specific settings and boundary parity` -> `WP1 remaining frozen formulas` -> `WP2 remaining assessment primitives` and `WP3 jackknife/permutation plus replicate policy` -> `WP4 intervals/tests` -> `WP5 complete result workspaces and durable jobs` -> `WP6 references/simulations/benchmark` -> `v0.4 gate audit`.

The highest-risk items are independent rho_A reference agreement, model-implied residual fit indices, studentized interval cost, BCa failed-replicate behavior, score-sign alignment, and exact thread-count invariance. They should receive fixtures before broad UI work.

## v1.1.1 Native UX Hardening Note

QuickPLS v1.1.1 adds a stricter desktop UX gate named `v1_1_1_native_ux_hardening`. It hardens the prior v1.1 UX pass with release-executable workflow evidence, dense SEM designer smoke fixtures, inspector progressive sections, collapsed advanced method settings, broader SVG preview/export parity, keyboard workflow evidence, and a disabled-action explanation audit.

Evidence is generated by `npm run qpls:v111:audit`, which writes:

- `validation/results/v111_native_gui_workflow_smoke.json`
- `validation/results/v111_sem_designer_dense_smoke.json`
- `validation/results/v111_settings_ux_smoke.json`
- `validation/results/v111_report_export_parity.json`
- `validation/results/v111_keyboard_native_smoke.json`
- `validation/results/v111_disabled_actions_audit.json`

`pywinauto` remains validation-only. The current hardening scripts record its availability and still require production native workflow and release-launch evidence; full literal Windows dialog clicking can be promoted when the validation environment includes UIA tooling.

## v1.2 Method Promotion Program Note

QuickPLS v1.2 starts a calculation-method promotion program rather than broadening stable claims globally.

Evidence added:

- `docs/METHOD_PROMOTION_CRITERIA.md`
- `docs/METHOD_PROMOTION_PROGRAM_V1_2.md`
- `validation/method_promotion_program_audit.py`
- `validation/promotion_matrix.py`
- `validation/pls_core_method_promotion_audit.py`
- `validation/assessment_method_promotion_audit.py`
- `validation/inference_method_promotion_audit.py`
- `validation/pca_method_promotion_audit.py`
- `validation/ols_method_promotion_audit.py`
- `validation/method_promotion_product_enforcement_audit.py`
- `validation/results/method_promotion_program_audit.json`
- `validation/results/method_promotion_matrix_v1_2.json`
- `validation/results/pls_core_method_promotion_audit.json`
- `validation/results/assessment_method_promotion_audit.json`
- `validation/results/inference_method_promotion_audit.json`
- `validation/results/pca_method_promotion_audit.json`
- `validation/results/ols_method_promotion_audit.json`
- `validation/results/method_promotion_product_enforcement_audit.json`
- `validation/development_slices.json` slice `v1_2_method_promotion_program`

Current gate posture:

- Promotion framework setup: passed.
- PLS core estimator-only promotion: passed.
- Assessment metrics promotion: passed.
- Inference/resampling promotion: passed.
- Standalone PCA promotion: passed.
- OLS regression promotion: passed.
- First method promotion batch: passed.
- Product enforcement of newly validated scopes: passed.
- Second-source and simulation expansion: open.

The first intended promotion batch is PLS core estimator output, assessment, inference/resampling for documented PLS settings, standalone PCA, and OLS regression. All five first-batch calculation scopes are now promoted and reflected in product labels/result exports. Every method outside a passed method-specific promotion gate remains experimental, watermarked, or unsupported according to its documented scope.

## v1.2.1 Second-Batch Method Promotion Note

Evidence added:

- `validation/mediation_method_promotion_audit.py`
- `validation/moderation_method_promotion_audit.py`
- `validation/plsc_method_promotion_audit.py`
- `validation/wpls_method_promotion_audit.py`
- `validation/ipma_method_promotion_audit.py`
- `validation/plspredict_method_promotion_audit.py`
- `validation/nca_method_promotion_audit.py`
- `validation/second_batch_product_enforcement_audit.py`
- `validation/second_batch_method_promotion_audit.py`
- `validation/development_slices.json` slice `v1_2_1_second_batch_method_promotion`

Promoted scopes:

- PLS mediation effect decomposition.
- Two-stage moderation.
- Reflective-only PLSc.
- Positive case-weighted reflective WPLS.
- Bounded predecessor-only IPMA using observed-sample 0-100 performance; cIPMA is excluded, and packaged-native execution, XLSX export, explicit save, and same-run reopen are accepted.
- Deterministic PLSpredict/repeated k-fold/CVPAT.
- Numeric X/Y NCA v2 with record-high CE-FDH peers, CR-FDH regression through those peers, seeded permutation p values, observed-range bottlenecks, and strict archive validation. Packaged-native workflow acceptance remains open.

Later-batch methods remain experimental until separate method-specific promotion gates pass.

## v1.2.2 Group, Prediction, And Regression Promotion Note

Evidence added:

- `validation/micom_method_promotion_audit.py`
- `validation/mga_permutation_method_promotion_audit.py`
- `validation/pls_pos_method_promotion_audit.py`
- `validation/fimix_pls_method_promotion_audit.py`
- `validation/logistic_method_promotion_audit.py`
- `validation/process_method_promotion_audit.py`
- `validation/third_batch_product_enforcement_audit.py`
- `validation/third_batch_method_promotion_audit.py`
- `validation/development_slices.json` slice `v1_2_2_group_prediction_regression_promotion`

Retained promoted scopes and withdrawal:

- MICOM promotion withdrawn; production execution blocked pending correct reimplementation and independent validation.
- Two-group permutation MGA.
- Deterministic 2-5 segment PLS-POS.
- Bounded deterministic 2-3 class FIMIX-PLS.
- Binary numeric complete-case logistic regression.
- Bounded PROCESS-style mediation and moderation.

Remaining later-batch methods remain experimental: CB-SEM/CFA, GSCA, higher-order constructs, nonlinear effects, endogeneity, CCA, CTA-PLS, and moderated mediation.

## QuickPLS 3 Wave 1 capability qualification checkpoint

- `regression_logistic_v2`, `regression_bootstrap_v1`, and bounded `freedman_lane_permutation_v1` Structural Path Randomization have current method-specific `release_qualified` evidence in the QuickPLS 3 parity ledger. Bounded graph-defined `regression_process_v2` remains `native_qualified`.
- PROCESS v2 evidence covers independent Python/R arithmetic, scale-aware OLS/HC3 and Johnson-Neyman boundaries, strict archive/tamper validation, focused frontend/type checks, responsive browser authoring, genuine packaged 10,000-resample execution, cancellation/retry, complete accessible result and plot-data tables, native XLSX, explicit save/reopen, and clean shutdown. Release qualification is pending a passing repeated-completion process-role stability gate.
- Historical `regression_logistic_v1` and `regression_process_v1` payloads remain archive-only and are not current qualification evidence.
- QuickPLS 2.46.0 coordinates these Wave 1 capability qualifications. Historical 2.45.0 evidence artifacts retain their original labels, and all method-specific exclusions remain in force.

## v1.2.3 / v1.2.4 Final Method Promotion Note

Evidence added:

- `validation/higher_order_method_promotion_audit.py`
- `validation/nonlinear_effects_method_promotion_audit.py`
- `validation/endogeneity_method_promotion_audit.py`
- `validation/cca_method_promotion_audit.py`
- `validation/cta_pls_method_promotion_audit.py`
- `validation/moderated_mediation_method_promotion_audit.py`
- `validation/fourth_batch_product_enforcement_audit.py`
- `validation/fourth_batch_method_promotion_audit.py`
- `validation/cbsem_cfa_method_promotion_audit.py`
- `validation/gsca_method_promotion_audit.py`
- `validation/fifth_batch_product_enforcement_audit.py`
- `validation/fifth_batch_method_promotion_audit.py`
- `validation/method_promotion_completion_audit.py`
- `validation/development_slices.json` slices `v1_2_3_extended_pls_diagnostics_promotion`, `v1_2_4_cbsem_gsca_promotion`, and `v1_2_5_method_promotion_completion`

Promoted scopes:

- Higher-order constructs, CCA, CTA-PLS, Gaussian-copula endogeneity diagnostics, nonlinear diagnostics, and PLS moderated mediation diagnostics for documented v1.2.3 bounds.
- Raw-data single-group reflective CB-SEM/CFA ML and bounded `gsca_als_v2` joint global least-squares ALS; `gsca_v1` remains legacy preview-only.

The broader v1.2 method-promotion program is now closed for documented bounded scopes. Unsupported variants remain excluded by method docs, product labels, warnings, and export status.

## v1.3 SEM Designer UX Overhaul Note

Evidence added:

- `validation/v13_sem_designer_ux_smoke.mjs`
- `validation/v13_sem_designer_ux_audit.py`
- `validation/development_slices.json` slice `v1_3_sem_designer_ux_overhaul`
- `docs/V1_3_SEM_DESIGNER_UX_OVERHAUL.md`

Completed scope:

- Academic SEM diagram style is the default editable model canvas.
- Edit mode exposes subtle visual connection handles for path creation.
- Constructs and indicators remain draggable in edit mode while result and publication modes are locked.
- Edge labels support pointer dragging plus keyboard nudge/reset behavior.
- Context menus cover common researcher actions without requiring the dense inspector.
- Duplicate/self-path and duplicate covariance attempts show nearby feedback.
- Publication SVG remains the audited WYSIWYG diagram export.
## v1.3.1 SEM Diagram Geometry Polish

Status: complete for the diagram-layer milestone.

Evidence:

- `src/domain/semGeometry.ts`
- `src/domain/semGeometry.test.ts`
- `validation/v131_sem_geometry_smoke.mjs`
- `validation/v131_sem_geometry_audit.py`
- `docs/V1_3_1_SEM_DIAGRAM_GEOMETRY_POLISH.md`
- `validation/development_slices.json` slice `v1_3_1_sem_diagram_geometry_polish`

Notes:

- This milestone improves connector geometry, indicator placement, path-label controls, SmartPLS-like visual tokens, and SVG/canvas parity.
- It deliberately does not change engines, analysis recipes, result schemas, method validation, or numerical fingerprints.

## v1.3.2 SEM Canvas Toolbar Redesign

Status: complete for the toolbar/product-layer milestone.

Evidence:

- `src/components/ModelCanvas.tsx`
- `src/styles.css`
- `validation/v132_toolbar_smoke.mjs`
- `validation/v132_toolbar_audit.py`
- `docs/V1_3_2_SEM_CANVAS_TOOLBAR_REDESIGN.md`
- `validation/development_slices.json` slice `v1_3_2_sem_canvas_toolbar_redesign`

Notes:

- This milestone reduces the permanent SEM canvas toolbar to core modeling actions and moves object-specific controls into contextual toolbars.
- Arrange, View, and Results are grouped into dropdowns to prevent normal desktop toolbar scrolling.
- View controls now apply real diagram themes, grid/minimap visibility, and layout locking.
- Contextual controls now include indicator reassignment, construct pin/unpin, and multi-selection alignment/distribution evidence.
- Placeholder residual/caption/observed-indicator tools remain hidden from the permanent toolbar until those workflows are fully specified.
- It deliberately does not change engines, analysis recipes, result schemas, method validation, or numerical fingerprints.

## v1.3.3 SEM Explorer Sidebar Redesign

Status: complete for the sidebar/product-layer milestone.

Evidence:

- `src/components/Explorer.tsx`
- `src/components/ModelCanvas.tsx`
- `src/store.ts`
- `src/styles.css`
- `validation/v133_sem_sidebar_smoke.mjs`
- `validation/v133_sem_sidebar_audit.py`
- `docs/V1_3_3_SEM_EXPLORER_SIDEBAR_REDESIGN.md`
- `validation/development_slices.json` slice `v1_3_3_sem_explorer_sidebar_redesign`

Notes:

- The left sidebar now behaves as a SEM Explorer with Constructs, Variables, Structure, and Issues tabs.
- Common model-building actions are available without opening the right inspector: construct focus, rename, duplicate, delete, path creation, variable assignment, indicator layout commands, path routing, label reset, and issue fix actions.
- Sidebar width, collapsed state, and active tab are UI-only preferences and do not affect method recipes or numerical fingerprints.
- The milestone deliberately does not change engines, result schemas, method validation, or export calculations.

## v1.4 Frontend Success Program

Status: complete for the frontend/product-layer program.

Evidence:

- `src/components/Ui.tsx`
- `src/components/OnboardingWorkspace.tsx`
- `src/components/AnalysisCatalog.tsx`
- `src/components/RunHistory.tsx`
- `src/components/ReportsWorkspace.tsx`
- `src/components/Explorer.tsx`
- `src/components/Inspector.tsx`
- `src/components/ModelCanvas.tsx`
- `src/store.ts`
- `src/types.ts`
- `src/styles.css`
- `validation/v14_frontend_success_audit.py`
- `validation/v14_frontend_success_smoke.mjs`
- `docs/V1_4_FRONTEND_SUCCESS_PROGRAM.md`
- `validation/development_slices.json` slices `v1_4_0_frontend_design_system_foundation` through `v1_4_frontend_success_program`

Notes:

- The release line adds a desktop design system, guided method setup, result workspace tabs, publication presets, onboarding/start workflow, explorer search and issue filtering, and large-model canvas controls.
- All added state is UI-only: `UiPreferences`, `ResultWorkspaceState`, `MethodSetupState`, `OnboardingState`, and `LargeModelViewState`.
- The app version is `1.4.7`; release artifacts use the label `v1_4_frontend_success_program`.
- This milestone deliberately does not change estimators, method formulas, analysis result schemas, validation tolerances, or numerical fingerprints.

## v1.5.0 Researcher UX Refinement

Status: complete for the researcher-productivity frontend milestone.

Evidence:

- `src/components/ProductivityOverlays.tsx`
- `src/components/TopBar.tsx`
- `src/components/StatusBar.tsx`
- `src/components/AnalysisCatalog.tsx`
- `src/components/RunHistory.tsx`
- `src/components/ReportsWorkspace.tsx`
- `src/components/Explorer.tsx`
- `src/styles.css`
- `validation/v150_researcher_ux_smoke.mjs`
- `validation/v150_researcher_ux_audit.py`
- `docs/V1_5_0_RESEARCHER_UX_REFINEMENT.md`
- `validation/development_slices.json` slice `v1_5_0_researcher_ux_refinement`

Notes:

- The milestone adds command palette, shortcut overlay, toast feedback, status-bar autosave messaging, method run summary, result headline cards, current-table export, publication export stepper, and variable prefix grouping.
- The app version is `1.5.0`; release artifacts use the label `v1_5_0_researcher_ux_refinement`.
- This milestone deliberately does not change estimators, method formulas, analysis result schemas, validation tolerances, or numerical fingerprints.

## v1.5.1 Navigation Workspace Hardening

Status: complete for the left-navigation and workspace-hardening milestone.

Evidence:

- `src/components/NavRail.tsx`
- `src/components/WorkflowStrip.tsx`
- `src/components/OnboardingWorkspace.tsx`
- `src/components/DataWorkspace.tsx`
- `src/components/AnalysisCatalog.tsx`
- `src/components/RunHistory.tsx`
- `src/components/ProductivityOverlays.tsx`
- `validation/v151_navigation_smoke.mjs`
- `validation/v151_navigation_audit.py`
- `docs/V1_5_1_NAVIGATION_WORKSPACE_HARDENING.md`
- `validation/development_slices.json` slice `v1_5_1_navigation_workspace_hardening`

Notes:

- The primary rail now follows Home, Data, Model, Setup, Run, Results, and Report.
- Groups remains available as a workflow through Setup and the Results Groups tab, but it is no longer a permanent rail item.
- The app version is `1.5.1`; release artifacts use the label `v1_5_1_navigation_workspace_hardening`.
- This milestone deliberately does not change estimators, method formulas, analysis result schemas, validation tolerances, or numerical fingerprints.

## v1.5.2 Data Workspace Hardening

Status: complete for the Data-workspace UX hardening milestone.

Evidence:

- `src/components/DataWorkspace.tsx`
- `src/domain/dataWorkspace.ts`
- `src/domain/dataWorkspace.test.ts`
- `validation/v152_data_workspace_smoke.mjs`
- `validation/v152_data_workspace_audit.py`
- `docs/V1_5_2_DATA_WORKSPACE_HARDENING.md`
- `validation/development_slices.json` slice `v1_5_2_data_workspace_hardening`

Notes:

- The Data workspace now separates import source, quality summary, preview/metadata, and model handoff.
- Sample data is presented as a researcher-facing action; development fixture paths are hidden behind validation details.
- Covariance/correlation matrix modes show persistent sample-size and desktop-runtime requirements while keeping the current loaded dataset preview clearly labeled.
- The app version is `1.5.2`; release artifacts use the label `v1_5_2_data_workspace_hardening`.
- This milestone deliberately does not change estimators, method formulas, analysis result schemas, validation tolerances, or numerical fingerprints.
## v1.5.3 Layout, Copy, And Readiness Polish

- Added shared card heading/action structure to prevent title/body collisions across Home, Setup, Report, and status cards.
- Replaced the repeated top disabled-run banner with a compact blocker chip and localized action-specific disabled reasons.
- Added workspace scroll reset on rail changes, Data selected-column profile statistics, model generic-path label suppression, Setup scope-status copy, progressive group/prediction setup, Results blocker-aware empty states, and Report preset/export alignment.
- Added `validation/v153_layout_copy_smoke.mjs`, `validation/v153_layout_copy_audit.py`, `docs/V1_5_3_LAYOUT_COPY_READINESS_POLISH.md`, and the `v1_5_3_layout_copy_readiness_polish` registry gate.
- Updated release metadata to `1.5.3` and artifact labeling to `v1_5_3_layout_copy_readiness_polish` for fresh non-overwriting desktop builds.

## v1.5.4 Results Workspace Hardening

- Reworked the Results workspace into tab-specific review sections for Summary, Measurement Model, Structural Model, Reliability and Validity, Inference, Prediction, Groups, Diagnostics, and Comparison.
- Added interpretation notes, current-tab export scoping, sticky result controls, and explicit not-run states for method families without payloads.
- Added diagram/result linking: selected SEM paths highlight matching result rows in the relevant tables.
- Added `validation/v154_results_workspace_smoke.mjs`, `validation/v154_results_workspace_audit.py`, `validation/v154_results_native_smoke.py`, `docs/V1_5_4_RESULTS_WORKSPACE_HARDENING.md`, and the `v1_5_4_results_workspace_hardening` registry gate.
- Updated release metadata to `1.5.4` and artifact labeling to `v1_5_4_results_workspace_hardening` for fresh non-overwriting desktop builds.
- Boundary: frontend/product polish only; no numerical backend or result-schema changes.

## v1.5.5 Results Interpretation Polish

- Added a frontend-only interpretation registry for validated result families, including threshold guidance, rationale, scope status, and report wording.
- Added researcher controls for result precision, interpretation visibility, table-level copy, row detail, and current-tab exports.
- Added an Interpretation tab with next-step guidance, copyable report wording, and result availability mapping.
- Implemented bounded two-run comparison for compatible PLS-family runs with metadata, path, R², and measurement deltas.
- Added `validation/v155_results_interpretation_smoke.mjs`, `validation/v155_results_interpretation_audit.py`, `docs/V1_5_5_RESULTS_INTERPRETATION_POLISH.md`, and the `v1_5_5_results_interpretation_polish` registry gate.
- Updated release metadata to `1.5.5` and artifact labeling to `v1_5_5_results_interpretation_polish` for fresh non-overwriting desktop builds.

## v1.5.6 Result-Specific Interpretation Engine

- Added `src/domain/resultInterpretation.ts` with deterministic findings for path coefficients, loadings, reliability, AVE, HTMT, cross-loadings, VIF, f2, Q2, inference availability, bootstrap intervals, mediation/moderation, method payloads, and SEM diagram-advisor checks.
- Added `src/domain/resultInterpretation.test.ts` with fixed fixtures for weak/negative paths, weak loadings, low AVE/reliability, high HTMT, high VIF, f2 classification, missing inference, bootstrap intervals, mediation, and diagram-shape advice.
- Updated `src/components/RunHistory.tsx` to render computed finding cards, exact-value row details, prioritized interpretation checklists, report wording, and copy controls.
- Updated `src/components/ReportsWorkspace.tsx` with an explicit `Include interpretation notes` option for HTML/print reports while keeping default numeric exports clean.
- Added `validation/v156_result_interpretation_smoke.mjs`, `validation/v156_result_interpretation_audit.py`, `docs/V1_5_6_RESULT_SPECIFIC_INTERPRETATION_ENGINE.md`, and the `v1_5_6_result_specific_interpretation_engine` registry gate.
- Boundary: frontend/product polish only; no numerical backend, result-schema, formula, or validation-tolerance changes.

## v1.5.7 UI/UX Launch-Quality Audit

- Converted the latest full-screen user-supplied screenshots into a formal launch-quality issue register with 60 concrete issues across Home, Data, Model, Setup, Run, Results, and Report.
- Preserved the screenshot evidence under `validation/results/screens/v157/ui-ux-launch-quality/` through `validation/v157_ui_ux_launch_quality_smoke.py`.
- Added `validation/v157_ui_ux_launch_quality_audit.py` to verify screenshot coverage, issue-register completeness, remediation sequencing, registry wiring, and non-engine boundary wording.
- Added `docs/V1_5_7_UI_UX_LAUNCH_QUALITY_AUDIT.md` and the `v1_5_7_ui_ux_launch_quality_audit` registry gate.
- Defined the next remediation sequence: Results workspace launch redesign, Report publication workflow redesign, Model canvas shell and panel polish, Setup/Run consolidation, Data/Home launch polish, and global design-system/accessibility pass.
- Boundary: audit/planning only; no statistical backend, result-schema, formula, project-format, or numerical-fingerprint changes.

## v1.5.8 Results Workspace Launch Redesign

- Replaced the Results mixed action strip with a dedicated workbench shell that separates section navigation from table tools.
- Added tab hints, grouped utility controls, row-count metadata, and wide-table scroll guidance for more predictable desktop use.
- Triaged interpretation findings by severity and capped visible cards while keeping full checklist/copy access.
- Deduplicated HTMT matrix warnings to unique construct pairs.
- Split mediation results into narrower effect summary, inference, and classification tables.
- Added `validation/v158_results_launch_smoke.mjs`, `validation/v158_results_launch_audit.py`, `docs/V1_5_8_RESULTS_WORKSPACE_LAUNCH_REDESIGN.md`, and the `v1_5_8_results_workspace_launch_redesign` registry gate.
- Boundary: frontend/product polish only; no numerical backend, result-schema, formula, or validation-tolerance changes.

## v1.5.9 Report Publication Workflow Redesign

- Reworked Report presets into selectable cards and grouped publication setup by Figure, Statistics, Tables, and Notes.
- Replaced passive export cards with explicit export actions and nearby disabled reasons for CSV, HTML, XLSX, Print/PDF, and SVG.
- Added preview layout-risk guidance so users know when to switch from current canvas to tidy publication layout before exporting.
- Moved run comparison to the Results comparison workspace and kept Report focused on publication/export output.
- Improved SmartPLS-like SVG labels with label backgrounds and automatic structural-label offsets.
- Added `validation/v159_report_publication_smoke.mjs`, `validation/v159_report_publication_audit.py`, `docs/V1_5_9_REPORT_PUBLICATION_WORKFLOW_REDESIGN.md`, and the `v1_5_9_report_publication_workflow_redesign` registry gate.
- Boundary: frontend/product polish only; no numerical backend, result-schema, formula, or validation-tolerance changes.
## v1.6.0 Model Canvas Shell And Panel Polish

- Added collapsible inspector state and model-shell grid classes so researchers can reclaim canvas width for medium and large SEM diagrams.
- Added View menu controls for explorer/inspector collapse, opt-in minimap, selected-neighborhood isolation, and indicator collapse.
- Reduced selected-object toolbar crowding with grouped route/indicator controls and simplified the left SEM explorer cards.
- Added `validation/v160_model_canvas_smoke.mjs`, `validation/v160_model_canvas_audit.py`, and `docs/V1_6_0_MODEL_CANVAS_SHELL_AND_PANEL_POLISH.md`.

## v1.6.1 Setup/Run Workflow Consolidation

- Made Setup the primary configuration and launch surface by wiring its run action to the production `quickpls:run-analysis` event.
- Reduced duplicate readiness presentation by turning Run into a compact execution monitor and handoff workspace.
- Added `validation/v161_setup_run_smoke.mjs`, `validation/v161_setup_run_audit.py`, and `docs/V1_6_1_SETUP_RUN_WORKFLOW_CONSOLIDATION.md`.
- Boundary: frontend/product polish only; no numerical backend, result-schema, formula, or validation-tolerance changes.

## v2.0.3 Visual Fidelity Foundation

- Added `docs/V2_UI_VISUAL_CONTRACT.md` to make the selected QuickPLS 2.0 mockup direction an explicit implementation contract.
- Hardened the shared `--q2-*` design tokens and reusable `.qpls2-*` primitives for panels, card typography, chips, actions, and desktop spacing.
- Added Results v2 styling hooks so the upcoming Results redesign can align with the same foundation instead of another one-off surface.
- Added `validation/v203_visual_fidelity_smoke.mjs`, `validation/v203_visual_fidelity_audit.py`, and `docs/V2_0_3_VISUAL_FIDELITY_FOUNDATION.md`.
- Updated release metadata to `2.0.3` and artifact labeling to `v2_0_3_visual_fidelity_foundation`.
- Boundary: frontend/product visual foundation only; no estimator, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.0.4 Results Table And Interpretation Redesign

- Rebuilt the Results workspace around a v2 research-workbook frame with clearer result navigation and selected-run context.
- Added a tab-aware interpretation lens using existing deterministic interpretation findings and selected-run payloads.
- Added row/column/construct metadata to result table headers and preserved wide-table guidance for dense statistical outputs.
- Added `validation/v204_results_redesign_smoke.mjs`, `validation/v204_results_redesign_audit.py`, and `docs/V2_0_4_RESULTS_TABLE_INTERPRETATION_REDESIGN.md`.
- Updated release metadata to `2.0.4` and artifact labeling to `v2_0_4_results_table_interpretation_redesign`.
- Boundary: frontend/product Results redesign only; no estimator, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.0.5 Report Export Flow Redesign

- Rebuilt Report around a v2 report package hero, export-readiness metrics, preset command center, and explicit four-step export flow.
- Preserved existing SVG, CSV, HTML, desktop XLSX, and browser Print/PDF behavior while making disabled reasons and output status more visible.
- Reframed settings, export review, diagram preview, comparison link, and export actions with shared v2 panel/tokens.
- Added `validation/v205_report_redesign_smoke.mjs`, `validation/v205_report_redesign_audit.py`, and `docs/V2_0_5_REPORT_EXPORT_FLOW_REDESIGN.md`.
- Updated release metadata to `2.0.5` and artifact labeling to `v2_0_5_report_export_flow_redesign`.
- Boundary: frontend/product Report redesign only; no estimator, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v1.6.2 Data/Home Launch Polish

- Added a workflow-aware Home next-step launcher and compact status list.
- Kept the Data workspace bridge to model creation visible through prefix detection and `Open Model Designer`.
- Added `validation/v162_data_home_smoke.mjs`, `validation/v162_data_home_audit.py`, and `docs/V1_6_2_DATA_HOME_LAUNCH_POLISH.md`.
- Boundary: frontend/product polish only; no numerical backend, import backend, result-schema, formula, or validation-tolerance changes.

## v1.6.3 Global Design-System And Accessibility Pass

- Replaced stale live v1.5.3 header copy with the current v1.6.3 design/accessibility milestone label.
- Added static smoke and audit checks for stale release text, mojibake markers, scoped status language, keyboard focus contracts, and accessible Run disabled-state wiring.
- Added `validation/v163_design_accessibility_smoke.mjs`, `validation/v163_design_accessibility_audit.py`, and `docs/V1_6_3_GLOBAL_DESIGN_ACCESSIBILITY_PASS.md`.
- Boundary: frontend/product polish only; no numerical backend, result-schema, formula, project-format, or validation-tolerance changes.

## v1.7 SmartPLS-Competitive Researcher Experience

- Added method-scope transparency, Method Confidence panels, and reviewer-facing validation/context copy.
- Added continuous workflow cues across Setup, Results, Report, and Home sample workflows.
- Added Focus Diagram mode and publication-check copy for SEM diagrams.
- Added value-driven reportability checklist items and threshold-color toggle.
- Added Reviewer Pack preset and deterministic sample-project gallery/guided dataset workflow.
- Added v1.7 docs, static audits, registry slices, and release metadata for `1.7.6`.
- Boundary: frontend/product polish only; no estimator, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v1.8 Results And Report Refinement From Real User Testing

- Added v1.8 audit evidence for Results and Report review across bundled/generated real-like dataset profiles.
- Replaced the crowded Results action area with grouped menus and a compact sticky selected-run context.
- Promoted the reusable table shell for bootstrap and HTMT refinements: split bootstrap CI sections and unique HTMT construct-pair rows by default.
- Deduplicated interpretation findings at the source using canonical metric/object keys.
- Changed finding cards to value-specific sections: what the value says, why it matters, what to inspect next, and report wording.
- Reworked Report into Select run, Choose preset, Review figure/table preview, and Export, with export status feedback.
- Added `validation/v18_*` smoke/audit scripts, `docs/V1_8_RESULTS_REPORT_REFINEMENT_REAL_USER_TESTING.md`, and the `v1_8_results_report_refinement_real_user_testing` registry gate.
- Updated release metadata to `1.8.0` and artifact labeling to `v1_8_results_report_refinement_real_user_testing`.
- Boundary: frontend/product polish only; no statistical engine, formula, result schema, project format, validation tolerance, or numerical fingerprint changes.

## v1.8.1 Method Applicability And Guided Setup

- Added `src/domain/methodApplicability.ts` to classify each method as recommended, available, needs setup, not applicable, unsupported, or experimental for the current dataset/model/settings.
- Updated Setup to show recommendation groups, method-specific reasons, expected outputs, and next actions instead of a flat generic catalog.
- Updated the top-bar method selector to prefer only recommended/available primary methods and direct the full catalog to Setup.
- Added Data and Model guidance panels for “What can I do with this data/model?”.
- Added `src/domain/methodApplicability.test.ts`, `validation/v181_method_applicability_smoke.mjs`, `validation/v181_method_applicability_audit.py`, and `docs/V1_8_1_METHOD_APPLICABILITY_GUIDED_SETUP.md`.
- Updated release metadata to `1.8.1` and artifact labeling to `v1_8_1_method_applicability_guided_setup`.
- Boundary: frontend/product guidance only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.0.0 Design System And Shell

- Added Trust Center and Settings as first-class v2 workspace routes.
- Updated the navigation rail, command palette, smoke API, and app routing for the expanded desktop shell.
- Added shared v2 shell tokens and reusable panel styles to anchor later screen redesigns to the approved mockup direction.
- Added `validation/v200_shell_smoke.mjs`, `validation/v200_shell_audit.py`, and `docs/V2_0_0_DESIGN_SYSTEM_AND_SHELL.md`.
- Updated release metadata to `2.0.0` and artifact labeling to `v2_0_0_design_system_and_shell`.
- Boundary: frontend/product shell only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.0.1 Home And Data Redesign

- Converted Home into a v2 project command center with a current-workspace hero, recommended next step, command grid, workflow status, sample gallery, and guided dataset flow.
- Converted Data into a v2 workbench with import source and data quality in a top grid, method applicability guidance, prefix construct creation, and a dominant preview/metadata editor.
- Kept existing native import, sample dataset loading, metadata update, browser CSV preview, and model handoff behavior wired to the same commands.
- Added `validation/v201_home_data_smoke.mjs`, `validation/v201_home_data_audit.py`, and `docs/V2_0_1_HOME_DATA_REDESIGN.md`.
- Updated release metadata to `2.0.1` and artifact labeling to `v2_0_1_home_data_redesign`.
- Boundary: frontend/product workspace redesign only; no statistical engine, formula, import backend, result schema, project archive format, validation tolerance, or numerical fingerprint changes.
## v2.0.2 Setup Method Guidance Redesign

- Rebuilt Setup around the QuickPLS 2.0 mockup direction with a selected calculation hero, readiness panel, guided method browser, requirements sidecar, presets, launch summary, and calculation preview.
- Added selected-method requirement checks and exact next-action labels so users can see why a method is recommended, available after setup, unavailable, or unsupported.
- Kept Basic/Expert modes and existing method settings while removing duplicated flat settings presentation from the main page.
- Added `setup-v2-*` CSS using the shared v2 shell tokens.
- Added validation scripts and evidence files:
  - `validation/v202_setup_guidance_smoke.mjs`
  - `validation/v202_setup_guidance_audit.py`
  - `validation/results/v202_setup_guidance_smoke.json`
  - `validation/results/v202_setup_guidance_audit.json`
- Added `docs/V2_0_2_SETUP_METHOD_GUIDANCE_REDESIGN.md`.
- Updated release metadata to `2.0.2` and artifact labeling to `v2_0_2_setup_method_guidance_redesign`.
- No statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.

## v2.0.6 Model Shell And SEM Designer Surround

- Applied v2 split-workbench styling to the Model workspace shell around the existing SEM Designer.
- Added stable class hooks for the SEM Explorer, Model canvas, toolbar, and Inspector so the screen can match the approved QuickPLS 2.0 mockup direction.
- Restyled explorer cards, guidance, tabs, canvas toolbars, context controls, inspector sections, and method notes with shared v2 tokens.
- Fixed remaining Model/Inspector `R²` encoding issues.
- Added `validation/v206_model_shell_smoke.mjs`, `validation/v206_model_shell_audit.py`, and `docs/V2_0_6_MODEL_SHELL_SEM_DESIGNER_SURROUND.md`.
- Updated release metadata to `2.0.6` and artifact labeling to `v2_0_6_model_shell_sem_designer_surround`.
- Boundary: frontend/product Model shell polish only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.0.7 Run Execution Surface Redesign

- Rebuilt the Run workspace as a v2 calculation launch surface with method scope, readiness state, local disabled-run reasons, expected output preview, execution provenance, and Results/Report handoff.
- Preserved the existing `quickpls:run-analysis` event and desktop execution boundary.
- Added `validation/v207_run_surface_smoke.mjs`, `validation/v207_run_surface_audit.py`, and `docs/V2_0_7_RUN_EXECUTION_SURFACE_REDESIGN.md`.
- Updated release metadata to `2.0.7` and artifact labeling to `v2_0_7_run_execution_surface_redesign`.
- Boundary: frontend/product Run workspace polish only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.
# v2.0.8 Trust Center And Scope Transparency

- Applied the QuickPLS 2.0 shell and visual density to the Trust Center.
- Added current-method confidence, validation artifact index, method scope matrix, and offline/legal boundary panels.
- Reused existing method applicability logic for method availability wording.
- Kept the milestone frontend/product-only with no estimator, recipe, result schema, project archive, tolerance, or numerical fingerprint changes.
- Added `validation/v208_trust_center_smoke.mjs`, `validation/v208_trust_center_audit.py`, and registry gate `v2_0_8_trust_center_scope_transparency`.

## v2.0.9 Mockup Fidelity System

- Hardened `docs/V2_UI_VISUAL_CONTRACT.md` so the approved QuickPLS 2.0 mockup is treated as an enforceable product contract.
- Added source-level smoke coverage for shared v2 primitives, workspace coverage, milestone text, encoding safety, and SmartPLS-equivalence claim boundaries.
- Added audit coverage for package/Tauri/Cargo version consistency, registry state, roadmap expectations, documentation, and versioned artifact script conventions.
- Added `validation/v209_mockup_fidelity_smoke.mjs`, `validation/v209_mockup_fidelity_audit.py`, `docs/V2_0_9_MOCKUP_FIDELITY_SYSTEM.md`, and registry gate `v2_0_9_mockup_fidelity_system`.
- Updated release metadata to `2.0.9` and artifact labeling to `v2_0_9_mockup_fidelity_system`.
- Boundary: frontend/product fidelity system only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.1.0 Design System Foundation

- Added first-class v2.1 primitives in `src/components/Ui.tsx`: `WorkspacePage`, `Panel`, `MetricCard`, `CommandGroup`, `ToolbarButton`, and `InlineNotice`.
- Updated existing header/card primitives to compose the `qpls2` token language instead of relying only on one-off legacy classes.
- Added a Settings design-system preview with primitive samples for panel density, status wording, local disabled reasons, command groups, and toolbar buttons.
- Added `validation/v2100_design_system_smoke.mjs`, `validation/v2100_design_system_audit.py`, `docs/V2_1_0_DESIGN_SYSTEM_FOUNDATION.md`, and registry gate `v2_1_0_design_system_foundation`.
- Updated release metadata to `2.1.0` and artifact labeling to `v2_1_0_design_system_foundation`.
- Boundary: frontend/product design-system foundation only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.1.1 Home/Data Mockup Alignment

- Applied the v2.1 primitive system directly to Home and Data instead of leaving them as mixed custom shells.
- Home now uses `WorkspacePage`, `PageHeader`, `Panel`, `Card`, `MetricCard`, and `InlineNotice` for the command center, workspace metrics, and start-from-dataset guidance.
- Data now uses `WorkspacePage`, `PageHeader`, `Panel`, `MetricCard`, and `InlineNotice` for import source, sample warning, quality metrics, and metadata workflow.
- Kept native dataset import, validation fixture loading, metadata save, prefix construct creation, and method guidance wiring unchanged.
- Added `validation/v2111_home_data_mockup_smoke.mjs`, `validation/v2111_home_data_mockup_audit.py`, `docs/V2_1_1_HOME_DATA_MOCKUP_ALIGNMENT.md`, and registry gate `v2_1_1_home_data_mockup_alignment`.
- Updated release metadata to `2.1.1` and artifact labeling to `v2_1_1_home_data_mockup_alignment`.
- Boundary: frontend/product Home/Data mockup alignment only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.1 Workflow Navigation Parity

- Scoped the top workflow strip as the primary research workflow progress surface.
- Added a visible `Workflow` label and stable workflow scope/count attributes for smoke coverage.
- Preserved Trust and Settings as left-rail Support utilities instead of workflow steps.
- Added `validation/v251_workflow_navigation_smoke.mjs`, `validation/v251_workflow_navigation_audit.py`, and `docs/V2_5_1_WORKFLOW_NAVIGATION_PARITY.md`.
- Updated release metadata to `2.5.1` and artifact labeling to `v2_5_1_workflow_navigation_parity`.
- Boundary: frontend/product navigation parity only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.2 Launcher And Support Shell Separation

- Separated Home, Trust Center, and Settings from the primary calculation workflow strip and coach.
- Preserved workflow guidance on Data, Model, Setup, Run, Results, and Report.
- Added `validation/v252_launcher_support_shell_smoke.mjs`, `validation/v252_launcher_support_shell_audit.py`, and `docs/V2_5_2_LAUNCHER_SUPPORT_SHELL_SEPARATION.md`.
- Updated release metadata to `2.5.2` and artifact labeling to `v2_5_2_launcher_support_shell_separation`.
- Boundary: frontend/product shell routing only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.3 Support Utility Shell Polish

- Added a support utility bar for Home, Trust Center, and Settings so launcher/support destinations remain connected after leaving the calculation workflow strip.
- Preserved the primary calculation workflow on Data, Setup, Run, Results, and Report, and preserved Model as the dedicated SEM Designer surface.
- Added `validation/v253_support_shell_smoke.mjs`, `validation/v253_support_shell_audit.py`, and `docs/V2_5_3_SUPPORT_UTILITY_SHELL_POLISH.md`.
- Updated release metadata to `2.5.3` and artifact labeling to `v2_5_3_support_utility_shell_polish`.
- Boundary: frontend/product support-shell polish only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.4 Visual Contract Support-Shell Alignment

- Updated `docs/V2_UI_VISUAL_CONTRACT.md` so the calculation workflow strip excludes Home and explicitly covers Data, Model, Setup, Run, Results, and Report.
- Added support utility shell rules for Home, Trust Center, and Settings.
- Documented the Model workspace SEM Designer workflow-band exception.
- Fixed the remaining R-squared encoding artifact in the v2 visual contract.
- Added `validation/v254_visual_contract_audit.py` and `docs/V2_5_4_VISUAL_CONTRACT_SUPPORT_SHELL_ALIGNMENT.md`.
- Updated release metadata to `2.5.4` and artifact labeling to `v2_5_4_visual_contract_support_shell_alignment`.
- Boundary: frontend/product visual-contract alignment only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.5.5 Support Shell Viewport Alignment

- Wrapped the support utility bar in a workspace-aligned frame so Home, Trust Center, and Settings support controls share the same max width and gutters as page content.
- Added responsive support-shell CSS for narrower desktop/browser preview widths.
- Added rendered smoke evidence for Home, Trust Center, Settings, and Data at `1440x900` and `1280x800`.
- Added `validation/v255_support_viewport_smoke.mjs`, `validation/v255_support_viewport_audit.py`, and `docs/V2_5_5_SUPPORT_SHELL_VIEWPORT_ALIGNMENT.md`.
- Updated release metadata to `2.5.5` and artifact labeling to `v2_5_5_support_shell_viewport_alignment`.
- Boundary: frontend/product support-shell layout only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.6.0 Launch-Quality Visual Consolidation

- Added a grouped launch-quality smoke harness across Home, Trust Center, Settings, Data, Model, Setup, Run, Results, and Report.
- Verified support shell separation, workflow shell presence, disabled-action descriptions, horizontal-overflow safety, encoding safety, and non-equivalence wording at `1440x900` and `1280x800`.
- Added `validation/v260_launch_quality_smoke.mjs`, `validation/v260_launch_quality_audit.py`, and `docs/V2_6_0_LAUNCH_QUALITY_VISUAL_CONSOLIDATION.md`.
- Updated release metadata to `2.6.0` and artifact labeling to `v2_6_0_launch_quality_visual_consolidation`.
- Boundary: frontend/product launch-quality validation only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.7.0 Visual Issue Register

- Added a rendered visual issue register for Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings at `1440x900` and `1280x800`.
- Added `validation/v270_visual_issue_register_smoke.mjs`, `validation/v270_visual_issue_register_audit.py`, `docs/V2_7_0_VISUAL_ISSUE_REGISTER.md`, and registry gate `v2_7_0_visual_issue_register`.
- The smoke writes `validation/results/v270_visual_issue_register.json` and screenshots under `validation/results/screens/v270/visual-issue-register/`.
- Updated release metadata to `2.7.0` and artifact labeling to `v2_7_0_visual_issue_register`.
- Boundary: frontend/product visual QA governance only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.9.0 Acceptance Backlog And Next Pass

- Added a rendered acceptance-backlog smoke across the QuickPLS 2.x workflow and support screens at `1440x900` and `1280x800`.
- Generated `validation/results/v290_acceptance_backlog.json` to keep future work grouped into `do_next`, `defer`, and `do_not_do` decisions.
- Added `validation/v290_acceptance_backlog_smoke.mjs`, `validation/v290_acceptance_backlog_audit.py`, `docs/V2_9_0_ACCEPTANCE_BACKLOG_AND_NEXT_PASS.md`, and registry gate `v2_9_0_acceptance_backlog_and_next_pass`.
- Updated release metadata to `2.9.0` and artifact labeling to `v2_9_0_acceptance_backlog_and_next_pass`.
- Boundary: frontend/product backlog governance only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.8.0 Release Handoff Consistency

- Updated public README, installation, and source-build handoff docs to the current QuickPLS 2.x verification path.
- Added `validation/v280_release_handoff_audit.py`, `docs/V2_8_0_RELEASE_HANDOFF_CONSISTENCY.md`, `docs/RELEASE_NOTES_V2_8_0.md`, and registry gate `v2_8_0_release_handoff_consistency`.
- Updated release metadata to `2.8.0` and artifact labeling to `v2_8_0_release_handoff_consistency`.
- Boundary: documentation/release handoff only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.11.0 Method Applicability Setup Polish

- Added project-specific method availability counts in Setup.
- Added method-card data attributes and missing-requirement copy for recommendation, setup, and blocked/scoped states.
- Added selected-method "why not available yet" guidance.
- Updated Data, Model, and top-bar method guidance markers for v2.11 validation.
- Added `validation/v2110_method_setup_applicability_smoke.mjs`, `validation/v2110_method_setup_applicability_audit.py`, `docs/V2_11_0_METHOD_APPLICABILITY_SETUP_POLISH.md`, and `docs/RELEASE_NOTES_V2_11_0.md`.
- Updated release metadata to `2.11.0` and artifact labeling to `v2_11_0_method_applicability_setup_polish`.
- Boundary: frontend/product method setup guidance only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.
## v2.12.0 Real Dataset Review Protocol

- Added `docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md` as the manual real-dataset review checklist.
- Added `validation/templates/real_dataset_issue_register_template.json` for anonymized product and evidence-gap notes.
- Added `validation/v2120_real_dataset_protocol_smoke.mjs` and `validation/v2120_real_dataset_protocol_audit.py`.
- Updated version metadata, active milestone tracker, development registry, and release artifact label to `2.12.0`.
- Boundary: frontend/product protocol only; no numerical backend, result schema, project archive, or validation-tolerance changes.

## v2.13.0 Real Dataset Protocol Entrypoints

- Added a Trust Center protocol table for the v2.12 real dataset review protocol and anonymized template.
- Added Settings privacy summary cards for no private data commits, screenshot redaction, anonymized notes, and fixture-only gates.
- Added a Home notice that directs private dataset reviewers to the Trust Center protocol.
- Added `validation/v2130_real_dataset_entrypoints_smoke.mjs` and `validation/v2130_real_dataset_entrypoints_audit.py`.
- Boundary: frontend-only entrypoints; no numerical backend, result schema, project archive, or validation-tolerance changes.
## v2.14.0 - Real Dataset Feedback Triage

- Added a privacy-safe triage layer for manual real-dataset review notes.
- Added `validation/templates/real_dataset_feedback_triage_template.json` so issue notes are anonymized and categorized before implementation.
- Added v2.14 smoke/audit scripts and generated backlog evidence.
- Preserved the QuickPLS 2.x boundary: no estimator, result schema, project archive, method validation, or numerical fingerprint changes.

## v2.15.0 - Workflow Method Guidance Triage Pass

- Added selected-method decision guidance in Setup with next action, first failed requirement, and expected outputs.
- Added dataset-level recommended next move guidance in Data.
- Added visible model-shape guidance action labels in the Model explorer.
- Updated the top command bar to current guided setup wording while keeping the method selector conservative.
- Added `validation/v2150_workflow_method_guidance_smoke.mjs`, `validation/v2150_workflow_method_guidance_audit.py`, and `docs/V2_15_0_WORKFLOW_METHOD_GUIDANCE_TRIAGE_PASS.md`.
- Boundary: frontend/product method guidance only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.26.0 - Method Setup And Applicability Center

- Converted Setup into a desktop calculation setup center with explicit method availability lanes.
- Added v2.26 category tabs for recommended, available, needs-setup, diagnostics, standalone, and not-applicable method states.
- Added an inference add-ons panel so bootstrap remains attached to compatible estimators.
- Kept unavailable methods visible with exact reasons and next actions under the scoped/blocked lane.
- Fixed stale `R²` method-output mojibake.
- Added `validation/v2260_method_setup_smoke.mjs`, `validation/v2260_method_setup_audit.py`, and `docs/V2_26_0_METHOD_SETUP_APPLICABILITY_CENTER.md`.
- Boundary: frontend/product method setup presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.27.0 - Calculation Run Monitor

- Rebuilt the Run workspace around a native desktop computation-monitor layout.
- Added shared `runMonitor` UI state for blocked, queued, validating, running, cancelling, completed, failed, and cancelled states.
- Wired Run workspace controls to the same top-bar run/cancel event path used by the native job lifecycle.
- Added procedure, progress/log, immutable settings, output availability, and handoff panels.
- Added `validation/v2270_run_monitor_smoke.mjs`, `validation/v2270_run_monitor_audit.py`, and `docs/V2_27_0_CALCULATION_RUN_MONITOR.md`.
- Boundary: frontend/product run monitoring only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.28.0 - Results Workbook Redesign

- Rebuilt the Results workspace around a statistical-workbook layout with central tables and a right interpretation/detail pane.
- Added a sticky selected-run header, findings lanes, method-confidence summary, and selected-run provenance footer.
- Preserved existing result tabs, interpretation logic, table export, comparison, and diagram focus behavior.
- Added `validation/v2280_results_workbook_smoke.mjs`, `validation/v2280_results_workbook_audit.py`, and `docs/V2_28_0_RESULTS_WORKBOOK_REDESIGN.md`.
- Boundary: frontend/product results presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.29.0 - Research Table System

- Upgraded the shared Results `SectionTable` into a stronger desktop research-table shell.
- Added local per-table search, sticky first data column with row-selection support, select-all, copy selected rows, precision controls, density controls, sort controls, export current table, and row-detail interpretation.
- Converted PLSpredict target metrics and CVPAT paired-loss comparisons from bespoke tables into the shared table shell.
- Added `validation/v2290_research_tables_smoke.mjs`, `validation/v2290_research_tables_audit.py`, and `docs/V2_29_0_RESEARCH_TABLE_SYSTEM.md`.
- Boundary: frontend/product table presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.30.0 - Interpretation And Reportability Assistant

- Added a Results reportability assistant that groups run-specific checklist items into must-address, review, ready, and unavailable/not-applicable lanes.
- Added value-specific explanation sections for what the value says, why it matters, what to inspect next, and report wording.
- Added copyable report snippets generated from the selected run's existing interpretation paragraphs.
- Added `validation/v2300_reportability_assistant_smoke.mjs`, `validation/v2300_reportability_assistant_audit.py`, and `docs/V2_30_0_INTERPRETATION_REPORTABILITY_ASSISTANT.md`.
- Boundary: frontend/product interpretation presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.31.0 - Report Export Wizard

- Converted Report into a four-step desktop wizard for selecting content, previewing figure/tables, configuring document settings, and exporting package outputs.
- Added explicit step navigation with rendered markers for smoke validation.
- Kept report comparison as a link into Results Comparison instead of duplicating detailed comparison output inside Report.
- Added `validation/v2310_report_export_wizard_smoke.mjs`, `validation/v2310_report_export_wizard_audit.py`, and `docs/V2_31_0_REPORT_EXPORT_WIZARD.md`.
- Boundary: frontend/product report presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.36.0 - Native Desktop UI Spec And Component Plan

- Recorded the final QuickPLS 2.0 native desktop screen, dialog, and focus-mode mockups as the implementation source of truth.
- Defined the workbench visual contract, reusable desktop component inventory, screen-by-screen requirements, dialog requirements, and v2.32-v2.35 implementation order.
- Added `validation/v236_native_ui_spec_audit.py` and `docs/V2_36_0_NATIVE_DESKTOP_UI_SPEC_AND_COMPONENT_PLAN.md`.
- Boundary: planning/frontend specification only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.37.0 - Native Frontend Prototype Shell

- Added an isolated QuickPLS 2 native desktop prototype route behind `?native_prototype=1`.
- Implemented the full dummy-data workbench shell with a desktop menu bar, command strip, workflow rail, support rail entries, status bar, task dialogs, and all primary researcher workspaces.
- Added `validation/v237_native_frontend_prototype_smoke.mjs`, `validation/v237_native_frontend_prototype_audit.py`, and `docs/V2_37_0_NATIVE_FRONTEND_PROTOTYPE_SHELL.md`.
- Boundary: frontend prototype only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.38.0 - Native Frontend Backend Adapters

- Added `src/v2/nativePrototypeAdapters.ts` as a read-only bridge from the existing workspace store into the isolated native prototype.
- Updated the prototype shell screens to consume project, dataset, variable, construct, path, method, run, and trust data from the adapter with fallback static data only when needed.
- Added `validation/v238_native_adapters_smoke.mjs`, `validation/v238_native_adapters_audit.py`, and `docs/V2_38_0_NATIVE_FRONTEND_BACKEND_ADAPTERS.md`.
- Boundary: frontend/product adapter only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.39.0 - Native Frontend Screen Replacement Plan

- Added an opt-in `?native_shell=1` production-candidate shell route.
- Kept `?native_prototype=1` available for isolated prototype validation and kept the existing application as the default route.
- Added explicit native-view to workspace-view mapping so rail navigation updates existing workspace state.
- Added `validation/v239_screen_replacement_smoke.mjs`, `validation/v239_screen_replacement_audit.py`, and `docs/V2_39_0_NATIVE_FRONTEND_SCREEN_REPLACEMENT_PLAN.md`.
- Boundary: frontend/product shell-routing bridge only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.40.0 - Mockup Fidelity Native Shell Alignment

- Tightened the `?native_shell=1` production-candidate shell against the supplied QuickPLS 2.0 mockups.
- Added a mockup-style ribbon command strip, denser desktop workbench chrome, hierarchical SEM Explorer, tabbed Object Inspector, and bottom output/status pane.
- Added `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md` to list older app features not visible in the mockups for later product decisions.
- Added `validation/v2400_mockup_fidelity_smoke.mjs`, `validation/v2400_mockup_fidelity_audit.py`, and `docs/V2_40_0_MOCKUP_FIDELITY_NATIVE_SHELL_ALIGNMENT.md`.

## v2.41.0 Full Mockup Screen Parity Pass

- Added `validation/mockups/v2410_mockup_manifest.json` to map every supplied mockup PNG to a QuickPLS screen or dialog state.
- Added `validation/v2410_mockup_manifest_audit.py`, `validation/v2410_mockup_visual_parity_smoke.mjs`, and `validation/v2410_mockup_visual_parity_audit.py`.
- Added a deterministic `mockup_parity=1` frontend route for screenshot parity while preserving the normal backend-wired native shell.
- Updated the native shell and Model workbench chrome toward the supplied desktop mockup layout without changing backend or numerical behavior.
- Boundary: frontend/product mockup-fidelity alignment only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.42.0 - Make Native Mockup Shell Default

- Changed `src/App.tsx` so the production-candidate native shell is the default app route.
- Preserved `?native_shell=1`, `?native_prototype=1`, and added `?legacy_shell=1` as the explicit fallback route for the older shell.
- Added `validation/v2420_native_default_shell_smoke.mjs`, `validation/v2420_native_default_shell_audit.py`, and `docs/V2_42_0_MAKE_NATIVE_MOCKUP_SHELL_DEFAULT.md`.
- Boundary: frontend/product routing only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.42.1 - Native Shell QA Test Pack

- Added `validation/v2421_native_screen_qa_smoke.mjs`, `validation/v2421_native_interaction_wiring_smoke.mjs`, `validation/v2421_native_web_trace_audit.py`, and `validation/v2421_native_qa_test_pack_audit.py`.
- Captured default native-shell screenshots for every main workspace plus Import Data and Calculation Setup dialogs.
- Verified menu toggles, rail navigation, dialog close paths, Escape handling, SEM designer integration, and backend-adapter Run/Results/Report surfaces.
- Updated package, Tauri, Rust workspace, lockfile, release artifact label, roadmap current-stage expectation, and active milestone docs for version `2.42.1`.

## v2.43.0 - Full Native Frontend/Backend Wiring

- Added Close Project with an unsaved-changes decision dialog and store-level project clearing.
- Removed fake Pause exposure from the default native shell; cancellation remains the supported run interruption behavior.
- Added desktop task dialogs for Data Transform, Add Column, Recode, Missing Values, Filter, and Sort, wired to structured Data workspace command payloads.
- Added UI-only workbench layout/status bar persistence commands and an offline documentation dialog.
- Added release-integrity details for Trust Center checksum verification.
- Updated the native frontend wiring matrix, validation scripts, registry slice, roadmap current-stage expectation, and release-facing metadata for version `2.43.0`.
- Boundary: frontend/product QA, dialog keyboard behavior, release metadata, docs, and validation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.
# v2.45.0 Mockup Visible Feature Completion

- Completed the first post-v2.44 visible-feature pass for the default native shell.
- Added milestone docs and validation scripts for `v2_45_0_mockup_visible_feature_completion`.
- Updated version metadata to `2.45.0` and the release artifact label for future explicit builds.
- Preserved statistical formulas, result schemas, validation tolerances, project archive semantics, and numerical fingerprints.

## QuickPLS 2.46.0 Wave 1 capability qualification

- The QuickPLS 3 parity ledger records 17 in-scope capabilities: 14 are `native_qualified`, while Structural Path Randomization v1, Binary Logistic Regression v2, and Regression Bootstrapping v1 are `release_qualified`. Graph-defined PROCESS v2 remains native-qualified pending repeated-completion process-role stability evidence.
- `qpls3.inference.structural_path_randomization` is qualified only for single-model direct structural-score paths using fixed original converged PLS scores, intercept nuisance equations, path-specific deterministic streams, two-sided unadjusted plus-one probabilities, and exchangeable reduced-model residuals.
- Its independent Python/R arithmetic, exact deterministic-index and worker-invariance boundaries, calibrated paired homoscedastic Gaussian null/power scenarios, strict archive/tamper checks, focused frontend/type checks, three-viewport visual acceptance, genuine packaged cancellation/retry/completion, native XLSX, explicit save/reopen, and clean process/resource evidence pass.
- Evidence: `validation/results/structural_path_randomization_reference_report.json`, `validation/results/structural_path_randomization_boundary_test_report.json`, `validation/results/structural_path_randomization_frontend_gate_report.json`, `validation/results/v247_native_desktop_visual_acceptance.json`, `validation/results/structural_path_randomization_v1_packaged_acceptance.json`, and `validation/results/structural_path_randomization_method_promotion_audit.json`.
- This qualification does not cover measurement-model re-estimation, multiplicity adjustment, heteroskedastic or broader non-Gaussian validity, MGA, MICOM, causal proof, or numerical identity with another product. The explicit conditional/approximate interpretation warning remains required.
