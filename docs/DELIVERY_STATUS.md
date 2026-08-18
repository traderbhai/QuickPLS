# Delivery Status

## Active QuickPLS 2.x Work

Current active working tracker: `docs/V2_ACTIVE_MILESTONE.md`.

The QuickPLS 2.x frontend program now uses grouped milestones, targeted UI smoke/audit checks, and versioned desktop artifacts only for completed milestone versions.

## v2.25.0 Model Workbench Integration

Status: `validated`

This frontend-only milestone integrates the SEM designer into a native model workbench with a left explorer tree, central interactive SEM canvas, right property-sheet inspector, bottom issues/output pane, Focus Diagram mode, and publication checks. The SEM canvas logic and numerical behavior are preserved.

Evidence:

- `src/components/ModelIssuesPane.tsx`
- `src/components/ModelCanvas.tsx`
- `src/components/Explorer.tsx`
- `src/components/Inspector.tsx`
- `src/styles.css`
- `validation/v2250_model_workbench_smoke.mjs`
- `validation/v2250_model_workbench_audit.py`
- `validation/results/v2250_model_workbench_smoke.json`
- `validation/results/v2250_model_workbench_audit.json`
- `docs/V2_25_0_MODEL_WORKBENCH_INTEGRATION.md`
- gate `v2_25_0_model_workbench_integration`

## v2.24.0 Data Workbench Redesign

Status: `validated`

This frontend-only milestone turns Data into a native desktop-style workbench with Data View, Variable View, Import History, Data Quality, Notes, a dense data grid, variable metadata table, and method applicability guidance. Existing import APIs and dataset/project structures are preserved.

Evidence:

- `src/components/DataWorkspace.tsx`
- `src/styles.css`
- `validation/v2240_data_workbench_smoke.mjs`
- `validation/v2240_data_workbench_audit.py`
- `validation/results/v2240_data_workbench_smoke.json`
- `validation/results/v2240_data_workbench_audit.json`
- `docs/V2_24_0_DATA_WORKBENCH_REDESIGN.md`
- gate `v2_24_0_data_workbench_redesign`

## v2.23.0 Home And Project Manager

Status: `validated`

This frontend-only milestone turns Home into a compact desktop start center with project launcher actions, current workspace summary, recent-project list, recovery/autosave status, and quick links. It removes landing-page behavior without changing numerical behavior.

Evidence:

- `src/components/OnboardingWorkspace.tsx`
- `src/styles.css`
- `validation/v2230_home_project_manager_smoke.mjs`
- `validation/v2230_home_project_manager_audit.py`
- `validation/results/v2230_home_project_manager_smoke.json`
- `validation/results/v2230_home_project_manager_audit.json`
- `docs/V2_23_0_HOME_PROJECT_MANAGER.md`
- gate `v2_23_0_home_project_manager`

## v2.22.0 Menu Commands Dialogs Native Base

Status: `validated`

This frontend-only milestone starts the new SmartPLS/SPSS/AMOS-style native desktop redesign program by adding a typed command registry, full desktop menu coverage, dialog entry points, and permanent status-bar command feedback. It supersedes the earlier mockup-only shell direction without changing numerical behavior.

Evidence:

- `src/domain/desktopCommands.ts`
- `src/components/TopBar.tsx`
- `src/components/StatusBar.tsx`
- `validation/v2220_native_commands_audit.py`
- `validation/results/v2220_native_commands_audit.json`
- `docs/V2_22_0_MENU_COMMANDS_DIALOGS_NATIVE_BASE.md`
- gate `v2_22_0_menu_commands_dialogs_native_base`

## v2.21.0 Desktop Design System And Shell

Status: `validated`

This frontend-only milestone adds the native shell foundation: compact neutral desktop chrome, explicit v2.21 shell markers, and UI-only command feedback in the bottom status bar.

Evidence:

- `src/components/TopBar.tsx`
- `src/components/StatusBar.tsx`
- `src/styles.css`
- `validation/v2210_native_shell_audit.py`
- `validation/results/v2210_native_shell_audit.json`
- `docs/V2_21_0_DESKTOP_DESIGN_SYSTEM_SHELL.md`
- gate `v2_21_0_desktop_design_system_shell`

## v2.20.0 QuickPLS 2.0 Mockup Parity Release Audit

Status: `validated`

This frontend-only audit verifies the completed QuickPLS 2.0 mockup-parity program across the shell, menu/dialog layer, and all aligned workspaces. It confirms v2.16 through v2.19 remain clear and that no stale fixture wording, mojibake, SmartPLS-equivalence claim, or backend numerical change was introduced.

Evidence:

- `validation/v2200_mockup_parity_smoke.mjs`
- `validation/v2200_mockup_parity_audit.py`
- `validation/results/v2200_mockup_parity_smoke.json`
- `validation/results/v2200_mockup_parity_audit.json`
- `docs/V2_20_0_QUICKPLS_2_MOCKUP_PARITY_RELEASE_AUDIT.md`
- gate `v2_20_0_quickpls_2_mockup_parity_release_audit`

## v2.19.0 Report, Trust Center, And Settings Mockup Alignment

Status: `validated`

This frontend-only milestone aligns the Report export workflow, Trust Center validation evidence workspace, and Settings preferences workspace to the QuickPLS 2.0 mockup density while preserving export behavior, validation evidence display, and local UI preferences.

Evidence:

- `src/components/ReportsWorkspace.tsx`
- `src/components/TrustCenterWorkspace.tsx`
- `src/components/SettingsWorkspace.tsx`
- `src/styles.css`
- `validation/v2190_report_trust_settings_smoke.mjs`
- `validation/v2190_report_trust_settings_audit.py`
- `validation/results/v2190_report_trust_settings_smoke.json`
- `validation/results/v2190_report_trust_settings_audit.json`
- `docs/V2_19_0_REPORT_TRUST_SETTINGS_MOCKUP_ALIGNMENT.md`
- gate `v2_19_0_report_trust_settings_mockup_alignment`

## v2.18.0 Model, Run, And Results Mockup Alignment

Status: `validated`

This frontend-only milestone aligns the Model shell, Run calculation workspace, and Results workspace to the QuickPLS 2.0 mockup density and desktop workflow style while preserving SEM canvas behavior, run execution, result payloads, and interpretation logic.

Evidence:

- `src/components/ModelCanvas.tsx`
- `src/components/RunWorkspace.tsx`
- `src/components/RunHistory.tsx`
- `src/styles.css`
- `validation/v2180_model_run_results_smoke.mjs`
- `validation/v2180_model_run_results_audit.py`
- `validation/results/v2180_model_run_results_smoke.json`
- `validation/results/v2180_model_run_results_audit.json`
- `docs/V2_18_0_MODEL_RUN_RESULTS_MOCKUP_ALIGNMENT.md`
- gate `v2_18_0_model_run_results_mockup_alignment`

## v2.17.0 Home, Data, And Setup Mockup Alignment

Status: `validated`

This frontend-only milestone aligns the Home launcher, Data workspace, and Setup method-guidance workspace to the QuickPLS 2.0 mockup density and desktop workflow style while preserving the current statistical and project behavior.

Evidence:

- `src/components/OnboardingWorkspace.tsx`
- `src/components/DataWorkspace.tsx`
- `src/components/AnalysisCatalog.tsx`
- `src/styles.css`
- `validation/v2170_home_data_setup_smoke.mjs`
- `validation/v2170_home_data_setup_audit.py`
- `validation/results/v2170_home_data_setup_smoke.json`
- `validation/results/v2170_home_data_setup_audit.json`
- `docs/V2_17_0_HOME_DATA_SETUP_MOCKUP_ALIGNMENT.md`
- gate `v2_17_0_home_data_setup_mockup_alignment`

## v2.16.0 Desktop Shell Visual Contract

Status: `validated`

This frontend-only milestone establishes the mockup-parity desktop shell foundation for the QuickPLS 2.0 redesign: a React-rendered menu bar, compact command strip, native-style dialog shell, tighter desktop rail, and shared visual tokens around the existing workspaces.

Evidence:

- `src/components/TopBar.tsx`
- `src/store.ts`
- `src/types.ts`
- `src/styles.css`
- `validation/v2160_desktop_shell_smoke.mjs`
- `validation/v2160_desktop_shell_audit.py`
- `validation/results/v2160_desktop_shell_smoke.json`
- `validation/results/v2160_desktop_shell_audit.json`
- `docs/V2_16_0_DESKTOP_SHELL_VISUAL_CONTRACT.md`
- gate `v2_16_0_desktop_shell_visual_contract`

## v2.10.0 Results/Report Research Table Pass

Status: `validated`

This frontend-only milestone improves saved-run table scanning and report export-preview confidence in the Results and Report workspaces.

Evidence:

- `src/components/RunHistory.tsx`
- `src/components/ReportsWorkspace.tsx`
- `src/styles.css`
- `validation/v2100_results_report_tables_smoke.mjs`
- `validation/v2100_results_report_tables_audit.py`
- `validation/results/v2100_results_report_tables_smoke.json`
- `validation/results/v2100_results_report_tables_audit.json`
- screenshots under `validation/results/screens/v2100/results-report-tables/`
- gate `v2_10_0_results_report_research_table_pass`

## v2.5.0 Navigation Hierarchy Polish

Status: `validated`

This frontend-only milestone clarifies the left navigation rail by separating the main research workflow from support utilities.

Evidence:

- `src/components/NavRail.tsx`
- `src/styles.css`
- `validation/v250_navigation_hierarchy_smoke.mjs`
- `validation/v250_navigation_hierarchy_audit.py`
- `validation/results/v250_navigation_hierarchy_smoke.json`
- `validation/results/v250_navigation_hierarchy_audit.json`
- `docs/V2_5_0_NAVIGATION_HIERARCHY_POLISH.md`
- gate `v2_5_0_navigation_hierarchy_polish`

## v2.4.1 QuickPLS 2 Release Readiness Audit

Status: `validated`

This frontend/documentation milestone consolidates the current QuickPLS 2.x UI program as a coherent release-ready baseline before further design iteration.

Evidence:

- `validation/v241_release_readiness_smoke.mjs`
- `validation/v241_release_readiness_audit.py`
- `validation/results/v241_release_readiness_smoke.json`
- `validation/results/v241_release_readiness_audit.json`
- `docs/V2_4_1_QUICKPLS_2_RELEASE_READINESS_AUDIT.md`
- gate `v2_4_1_quickpls_2_release_readiness_audit`

## v2.4.0 Public Documentation And Screenshot Refresh

Status: `validated`

This frontend/product documentation milestone updates the public GitHub-facing documentation for the QuickPLS 2.x interface. It refreshes README, installation, build, tutorial links, and screenshot references so users can understand, install, build, verify, and use the current app without stale v1.8.1 release instructions.

Evidence:

- `README.md`
- `docs/INSTALLATION.md`
- `docs/BUILD_FROM_SOURCE.md`
- `docs/V2_4_0_PUBLIC_DOCUMENTATION_SCREENSHOT_REFRESH.md`
- screenshots under `docs/screenshots/v2/`
- `validation/results/v240_public_docs_smoke.json`
- `validation/results/v240_public_docs_audit.json`
- gate `v2_4_0_public_documentation_screenshot_refresh`

## v2.3.2 Shared UI Verification Harness

Status: `validated`

This frontend/product-only milestone consolidates repeated v2 UI smoke and static audit logic into shared validation helpers. It keeps v2.3.1 shell integrity checks passing while adding a focused v2.3.2 smoke for command-bar blocker navigation and shared harness coverage.

Evidence:

- `validation/lib/v2_ui_smoke_harness.mjs`
- `validation/lib/v2_ui_audit.py`
- `validation/v231_ui_integrity_smoke.mjs`
- `validation/v231_ui_integrity_audit.py`
- `validation/results/v232_shared_ui_harness_smoke.json`
- `validation/results/v232_shared_ui_harness_audit.json`
- screenshots under `validation/results/screens/v232/shared-ui-harness/`
- gate `v2_3_2_shared_ui_verification_harness`

## v2.3.1 UI Integrity Consolidation

Status: `validated`

This frontend/product-only milestone consolidates the v2 shell integrity checks after the command-bar readiness pass. It verifies visible version metadata, command-bar readiness metadata, v2 visual-contract wording, no mojibake in normal v2 UI/docs, no stale visible labels, and no SmartPLS-equivalence claims.

Evidence:

- `validation/results/v231_ui_integrity_smoke.json`
- `validation/results/v231_ui_integrity_audit.json`
- screenshots under `validation/results/screens/v231/ui-integrity/`
- gate `v2_3_1_ui_integrity_consolidation`

## v2.3.0 Global Command Bar Readiness

Status: `validated`

This frontend/product-only milestone aligns the global command bar with workflow readiness by exposing run-state metadata, showing exact nearby disabled-run reasons, and routing blocker actions through destination context.

Evidence:

- `validation/results/v230_command_bar_smoke.json`
- `validation/results/v230_command_bar_audit.json`
- screenshots under `validation/results/screens/v230/command-bar/`
- gate `v2_3_0_global_command_bar_readiness`

## v2.2.9 Workflow Strip Context Alignment

Status: `validated`

This frontend/product-only milestone aligns the top workflow strip with coach feedback by exposing workflow-step metadata and recording source/destination context when workflow-step navigation moves the researcher between workspaces.

Evidence:

- `validation/results/v229_workflow_strip_smoke.json`
- `validation/results/v229_workflow_strip_audit.json`
- screenshots under `validation/results/screens/v229/workflow-strip/`
- gate `v2_2_9_workflow_strip_context_alignment`

## v2.2.8 Workflow Feedback Lifecycle

Status: `validated`

This frontend/product-only milestone makes workflow coach destination and command feedback dismissible and clears stale feedback on cross-workspace navigation, dataset replacement, project reset, and project load.

Evidence:

- `validation/results/v228_feedback_lifecycle_smoke.json`
- `validation/results/v228_feedback_lifecycle_audit.json`
- screenshots under `validation/results/screens/v228/feedback-lifecycle/`
- gate `v2_2_8_workflow_feedback_lifecycle`

## v2.2.7 Workflow Command Feedback

Status: `validated`

This frontend/product-only milestone records coach-driven command requests as UI-only command context, renders a compact command feedback note after enabled command actions, and verifies disabled command actions remain inert.

Evidence:

- `validation/results/v227_command_feedback_smoke.json`
- `validation/results/v227_command_feedback_audit.json`
- screenshots under `validation/results/screens/v227/command-feedback/`
- gate `v2_2_7_workflow_command_feedback`

## v2.2.6 Workflow Destination Context

Status: `validated`

This frontend/product-only milestone records coach-driven workspace transitions as UI-only destination context, renders a compact landing note after enabled view-target actions, and verifies disabled coach actions remain inert.

Evidence:

- `validation/results/v226_destination_context_smoke.json`
- `validation/results/v226_destination_context_audit.json`
- screenshots under `validation/results/screens/v226/destination-context/`
- gate `v2_2_6_workflow_destination_context`

## v2.2.5 Workflow Coach Action Execution

Status: `validated`

This frontend/product-only milestone makes workflow coach actions executable and auditable. Coach buttons expose explicit target view and command event metadata, enabled view-target actions are smoke-tested by click-through behavior, disabled actions remain inert with nearby reasons, and action labels are consistent.

Evidence:

- `validation/results/v225_coach_execution_smoke.json`
- `validation/results/v225_coach_execution_audit.json`
- screenshots under `validation/results/screens/v225/coach-execution/`
- gate `v2_2_5_workflow_coach_action_execution`

## v2.2.4 Workflow Coach Action Clarity

Status: `validated`

This frontend/product-only milestone makes workflow coach actions more explicit and testable. Coach buttons expose stable action metadata, disabled actions show nearby reasons, duplicate secondary actions are suppressed, and command dispatch remains within the existing frontend command contract.

Evidence:

- `validation/results/v224_coach_actions_smoke.json`
- `validation/results/v224_coach_actions_audit.json`
- screenshots under `validation/results/screens/v224/coach-actions/`
- gate `v2_2_4_workflow_coach_action_clarity`

## v2.2.3 Model Workflow Context

Status: `validated`

This frontend/product-only milestone adds the workflow strip and state-aware coach to the Model workspace shell, keeping the existing SEM Designer, Explorer, and Inspector intact while making Data -> Model -> Setup handoff visible inside the designer.

Evidence:

- `validation/results/v223_model_workflow_smoke.json`
- `validation/results/v223_model_workflow_audit.json`
- screenshots under `validation/results/screens/v223/model-workflow/`
- gate `v2_2_3_model_workflow_context`

## v2.2.2 Workflow Step Clarity

Status: `validated`

This frontend/product-only milestone makes the workflow strip stateful and actionable, with step-level state, compact action labels, hover/accessibility reasons, and desktop-width overflow checks. Existing native command implementations, statistical engines, result schemas, project archive format, and numerical fingerprints are unchanged.

Evidence:
- `validation/results/v222_workflow_step_smoke.json`
- `validation/results/v222_workflow_step_audit.json`
- screenshots under `validation/results/screens/v222/workflow-step/`
- gate `v2_2_2_workflow_step_clarity`

## v2.2.1 Command Handoff Consistency

Status: `validated`

This frontend/product-only milestone makes workflow coach actions and top-bar actions use one shared command-event contract for run, save, open, demo, and import workflows. Existing native command implementations, statistical engines, result schemas, project archive format, and numerical fingerprints are unchanged.

Evidence:
- `validation/results/v221_command_handoff_smoke.json`
- `validation/results/v221_command_handoff_audit.json`
- screenshots under `validation/results/screens/v221/command-handoff/`
- gate `v2_2_1_command_handoff_consistency`

## v2.2.0 Workflow Continuity And Command Clarity

Status: `validated`

This frontend/product-only milestone adds a compact workflow coach across the non-model workspaces so the app always explains the current state, the next practical action, and any calculation/report blocker. The SEM Designer, statistical engines, result schemas, project archive format, and numerical fingerprints are unchanged.

Evidence:
- `validation/results/v220_workflow_continuity_smoke.json`
- `validation/results/v220_workflow_continuity_audit.json`
- screenshots under `validation/results/screens/v220/workflow-continuity/`
- gate `v2_2_0_workflow_continuity_command_clarity`

## v2.1.5 Rendered Shell Consistency Audit

Status: `validated`

This frontend/product-only milestone adds one consolidated rendered QA gate across Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings. It creates a stable v2 shell baseline before the next larger QuickPLS 2.0 UI milestone. Statistical engines, result schemas, project archive format, and numerical fingerprints are unchanged.

Evidence:
- `validation/results/v2115_rendered_shell_consistency_smoke.json`
- `validation/results/v2115_rendered_shell_consistency_audit.json`
- screenshots under `validation/results/screens/v2115/rendered-shell/`
- gate `v2_1_5_rendered_shell_consistency_audit`

## v2.1.4 Model/Trust/Settings Shell Alignment

Status: `validated`

This frontend/product-only milestone completes the QuickPLS 2.1 shell-alignment pass for the remaining Model shell, Trust Center, Settings, and global shell surfaces. The SEM designer behavior, analysis engines, result schemas, and numerical fingerprints are unchanged.

Evidence:
- `validation/results/v2114_model_trust_settings_shell_smoke.json`
- `validation/results/v2114_model_trust_settings_shell_audit.json`
- screenshots under `validation/results/screens/v2114/model-trust-settings/`
- gate `v2_1_4_model_trust_settings_shell_alignment`

## v2.1.3 Results/Report Mockup Alignment

Status: `validated`

This frontend/product-only milestone applies the QuickPLS 2.1 page, panel, metric, status, and confidence primitives to Results and Report. Existing saved-run review, interpretation, comparison, CSV/HTML/XLSX/SVG, and print/PDF-path behavior is preserved while the surfaces move closer to the approved desktop mockup direction.

Evidence:
- `validation/results/v2113_results_report_mockup_smoke.json`
- `validation/results/v2113_results_report_mockup_audit.json`
- screenshots under `validation/results/screens/v2113/results-report/`
- gate `v2_1_3_results_report_mockup_alignment`

## v2.1.2 Setup/Run Mockup Alignment

Status: `validated`

This frontend/product-only milestone applies the QuickPLS 2.1 page, panel, metric, status, and calculation primitives to Setup and Run. Existing method applicability, readiness, run-command event wiring, and saved-run handoff behavior is preserved while the surfaces move closer to the approved desktop mockup direction.

Evidence:
- `validation/results/v2112_setup_run_mockup_smoke.json`
- `validation/results/v2112_setup_run_mockup_audit.json`
- screenshots under `validation/results/screens/v2112/setup-run/`
- gate `v2_1_2_setup_run_mockup_alignment`

## v2.0.11 Mockup Pixel Alignment

Status: `validated`

This frontend/product-only milestone closes the v2.0.10 medium visual gaps by giving the Results empty state the same v2 command surface as populated Results, keeping a visible `Why trust this result?` route in evidence-heavy workspaces, and blocking normal UI `R²` mojibake.

Evidence:
- `validation/results/v211_mockup_pixel_alignment_smoke.json`
- `validation/results/v211_mockup_pixel_alignment_audit.json`
- screenshots under `validation/results/screens/v211/pixel-alignment/`
- gate `v2_0_11_mockup_pixel_alignment`

## v2.0.10 Visual Gap Audit

Status: `validated`

This frontend/product-only milestone adds rendered screenshot evidence and an issue register for the QuickPLS 2.0 mockup-matching program.

Evidence:
- `validation/results/v210_visual_gap_smoke.json`
- `validation/results/v210_visual_gap_audit.json`
- screenshots under `validation/results/screens/v210/visual-gap/`
- gate `v2_0_10_visual_gap_audit`

## v2.0.8 Trust Center And Scope Transparency

Complete: the Trust Center now uses the QuickPLS 2.0 visual contract and exposes current-method confidence, validation artifacts, method scope/applicability, offline runtime boundaries, validation-only dependency boundaries, and SmartPLS non-equivalence wording. This is a frontend/product-only milestone; numerical engines, recipes, result schemas, project archives, validation tolerances, and fingerprints are unchanged.

Evidence: `validation/results/v208_trust_center_smoke.json`, `validation/results/v208_trust_center_audit.json`, and gate `v2_0_8_trust_center_scope_transparency`.

## v0.1 Foundation

Complete: Tauri/React shell, visual model editor, Rust contracts, validation primitives, method gates, and CLI foundation.

## v0.2 Data and Project Platform

Complete:

- Typed CSV, TSV, text, XLS, XLSX, XLSB, ODS, SAV, and ZSAV import.
- Arrow IPC storage, column metadata, scale types, missing markers, dataset kinds, and SHA-256 data fingerprints.
- Versioned `.qpls` ZIP manifest with checksums, atomic writes, retained backups, recovery loading, and future-version read-only mode.
- Native create/open/save/import commands and dialogs, with durable projects owned by Rust.
- CLI `import`, `inspect`, archive-aware `validate`, and `methods` commands.
- User-facing scale, label, bounds, and missing-marker metadata editor.
- Explicit covariance/correlation imports with square, numeric, finite, symmetry, diagonal, bounds, and sample-size validation.
- Debounced autosave, recovery notification, retained backup, and stale-autosave cleanup after explicit saves.
- Archive schema v4 with tested legacy and v3 payload migration, typed result provenance, nested PLS payload validation, strong compact-bootstrap consistency checks, bootstrap round trips, and future-version read-only behavior.
- CSV, XLSX, and SAV fixture generation plus checksum mutation and interrupted-archive tests.
- README instructions distinguish the native Tauri shell from the browser-only preview and identify the exact corporate-reputation and compact deterministic validation fixture paths.
- The Data workspace has a researcher-facing import source panel, `Load Sample Dataset` action, mode-specific raw/covariance/correlation guidance, data-quality cards, variable search/filter, metadata editing sections, and prefix-based construct creation. Browser preview parses bundled CSV text safely without a native dialog; desktop mode imports the same fixture embedded in the Rust binary, so installed builds do not depend on the development path.

## v0.3 PLS Core

Complete and validated for the documented estimator scope:

- Deterministic PLS-PM Mode A and Mode B estimation, reflective/formative and single-item blocks.
- Path and factor inner weighting plus PCA block weighting.
- Standardized, mean-centered, and unstandardized preprocessing with recorded transformations.
- Deterministic convergence, construct scores, weights, loadings, structural paths, direct/indirect/total effects, and R2.
- Listwise missing-data handling and actionable errors for constants, duplicate or unknown indicators, duplicate structural paths, directed cycles, rank deficiency, isolated constructs, and non-convergence. Core and frontend validation both reject directed structural cycles before execution.
- Cooperative checkpoints inside estimator iterations and result assembly allow cancellation without changing ordinary deterministic results.
- GUI and CLI execution through the same Rust estimator and schema-versioned result envelope. Desktop-completed results and recipes are appended to `.qpls` projects; CLI runs write the selected result envelope to the requested output path.
- GUI and CLI result assembly now share `qpls-runner`; the desktop test `desktop_runner_payload_matches_cli_serialized_artifact` compares canonical payload within `1e-12`, diagnostics, method version, dataset fingerprint, and settings against the CLI artifact while ignoring generated IDs/timestamps.
- CLI `export` writes gated v0.3 estimator-only CSV and HTML tables from completed PLS result envelopes. The export includes provenance, convergence/observation counts, weights, loadings, paths, effects, R2, and diagnostics, and deliberately excludes experimental v0.4 assessment and resampling artifacts.
- Model workbench with dedicated Select and Path tools, click-predictor/click-outcome path creation, reconnectable edges, quick selected-path reverse/routing controls, four-sided handles, duplicate/self-path prevention, fit-to-view, horizontal and vertical layout, snap-to-grid, multi-selection deletion, duplication, undo/redo, selected-construct alignment, and horizontal/vertical distribution. Diagram estimates are now an explicit selected-run overlay: projects open in model-only mode, completed runs can be shown from the canvas run picker, incompatible runs are not painted over edited models, and fresh PLS-PM runs select their new result after execution.
- Variable explorer supports checkbox batch selection, create-from-selection, assign-to-selected-construct, and dragging one or many variables onto an existing construct or empty canvas. Empty-canvas drops create a construct, unique indicator ownership is preserved, and default creation searches for a noncolliding position.
- `npm run qpls:pls:csem` generates executable cSEM 0.6.1 reference evidence for path Mode A, Mode B, factor, and PCA. `npm run qpls:pls:plspm` generates python-plspm 0.5.7 evidence for path Mode A, Mode B, and factor loadings/paths. `npm run qpls:pls:pca` generates an independent NumPy eigensystem reference for PCA paths, loadings, and weights. `npm run qpls:pls:published` generates and compares the documented cSEM `threecommonfactors` published example. All comparisons pass below `1e-6`.
- Metamorphic coverage for indicator/construct order, positive affine transformations, deterministic reruns, and preprocessing modes.
- Release benchmark at 100,000 rows, 300 indicators, and 100 constructs: 2.761 seconds estimator time and 659.1 MB monitored peak working set on the documented development machine.

PLS-PM core is marked `validated` for the v0.3 estimator scope. Current desktop/CLI result envelopes still include v0.4 assessment artifacts, so only estimator-only exports are enabled; full run-envelope publication exports remain gated until result-family separation and v0.4 validation are complete.

## v0.4 Assessment and Inference

Partial implementation, gate open:

- A dedicated assessment crate computes typed reliability/discriminant metrics, R2 and adjusted R2, VIF, Cohen f-squared, deterministic blindfolding cross-validated redundancy Q2, and saturated/estimated SRMR and d_ULS. Current `pls_assessment_v7` retains v5 Dijkstra-Henseler rho_A and v6 separately versioned original signed HTMT and HTMT+ artifacts, and changes Cohen f-squared to a fixed-score reduced structural regression matching cSEM `calculatef2()`. Project validation reads assessment v1-v7, preserves version-specific capabilities, rejects fields mislabeled under older envelopes, and binds applicability, construct order, settings, and warning semantics to the immutable saved recipe. `npm run qpls:assessment:simulation` adds deterministic generated-data evidence that R2, Q2, and f2 respond to known structural signal and degrade when exogenous blocks are permuted. `npm run qpls:assessment:published` compares the combined assessment group against the cSEM satisfaction README fixture.
- Assessment loops expose cooperative progress/cancellation checkpoints. VIF uses cancellation-aware correlation preparation and a bounded predictor-correlation solve, with monotonic nested progress and a stable phase total.
- Cohen f-squared removes one predictor from the already-estimated target construct-score equation and recomputes the reduced structural R2 by OLS. A target left without predecessors receives intercept-only excluded R2 of zero; reduced-regression failures and cancellation remain explicit, and progress is monotonic.
- Blindfolding is scoped to reflective endogenous constructs and stores its deterministic omission distance D/settings plus PRESS, SSO, and Q2. Every round forwards cancellation and reports monotonic nested progress. Q2 is labeled as in-sample predictive relevance, not out-of-sample predictive performance. `npm run qpls:blindfolding:python` compares the Rust output against a development-only independent NumPy implementation of the frozen omission, prediction, PRESS/SSO, and Q2 contract.
- Saturated and estimated SRMR/d_ULS are stored as descriptive correlation-residual discrepancies. d_G, NFI, and RMS_theta are excluded from v4 and remain pending.
- Current runs report original signed HTMT and HTMT+ as separate typed matrices. Formative and undersized blocks are not applicable; nonpositive signed or zero absolute within-block denominators are explicitly unavailable. HTMT+ is sign-invariant and values above one are retained. Legacy v2-v5 `htmt` matrices remain readable and are labeled HTMT+ legacy output in run history.
- GUI and CLI emit the same schema-versioned result-envelope type containing estimation, assessment, typed provenance, and diagnostics. Canonical end-to-end boundary equality is not yet proven because run IDs and timestamps are generated independently.
- `python validation\external_reference_probe.py` writes `validation/results/external_reference_probe.json`; `qpls evidence v04-assessment` writes `validation/results/v04_assessment_evidence.json`, mapping current assessment metric groups to specs, fixture paths, tests, tolerances, available reference scripts, and missing evidence. R/cSEM are development-only validation tools and are never runtime requirements for QuickPLS users. If R is installed outside PATH, the probe accepts `QPLS_RSCRIPT` and `QPLS_R`; Python validation scripts and the cSEM PowerShell wrappers auto-discover the local portable `Documents\PLS-Sem\dist-desktop\r-runtime` layout, standard Windows R installs, and registry entries. `npm run qpls:rho-a:csem`, `npm run qpls:htmt:csem`, and `npm run qpls:assessment:csem` now execute development-only cSEM fixtures and QuickPLS comparisons on this workstation. This closes traceability for the currently equivalent cSEM-backed metrics but does not validate metrics with missing external-reference evidence.
- Existing alpha, rho_C, AVE, cross-loading, Fornell-Larcker, and R2 reference fixtures agree with cSEM values within `1e-6`. Original HTMT and HTMT+ now have an independent Python formula fixture for the corporate-reputation data, positive-affine invariance, individual sign reversal, construct reordering, and HTMT+ values above one. `npm run qpls:htmt:csem` proves original signed HTMT agreement with cSEM 0.6.1; it also records that cSEM `.absolute=TRUE` is not equivalent to Ringle et al. HTMT+ for mixed-sign cross-block correlations. `npm run qpls:htmt:seminr` proves HTMT+ agreement with seminr 2.5.0 on the same mixed-sign fixture. `validation/results/htmt_published_ringle_2023.json` adds rounded Ringle et al. HTMT+ worked-example evidence. `npm run qpls:assessment:csem` proves cSEM-equivalent R2, adjusted R2, structural VIF, fixed-score Cohen f-squared, estimated/saturated SRMR, and estimated/saturated d_ULS agreement within `1e-6`. Formative and single-item applicability is explicit.
- Experimental rho_A uses the frozen `dijkstra_henseler_rho_a_v1` contract, converts persisted estimator weights into standardized-indicator coordinates, and verifies `R w` against persisted loadings. Hand, independent Decimal equation, primary-paper Equation 3, signed/improper/degenerate, preprocessing, factor/path, affine, permutation, listwise-missing, applicability, persistence, and tamper fixtures pass. The development-only cSEM 0.6.1 runner agrees with QuickPLS on the committed reflective Mode A fixture with maximum absolute difference `4.440892098500626e-16`; rho_A remains experimental only because the broader v0.4 assessment-family gate is still open.
- `qpls-resampling` implements indexed ChaCha20 bootstrap draws, strict replicate ordering, sign alignment, percentile and normal-reference summaries, compact BCa, and optional studentized output under current `indexed_resampling_v4`. Resampling v1-v3 remain readable but cannot carry fields introduced by later versions.
- Current `indexed_resampling_v4` adds optional compact `nested_studentized_v1` bootstrap-t inference. Enabling it requires at least 999 primary replicates and an odd 99..999 inner count; zero disables it. Inner streams are independently indexed, inner solutions align to their primary parent, and enabling the method does not alter primary, percentile, BCa, or permutation output.
- Bootstrap-t uses inner standard errors, Type-7 pivot quantiles, reversed-tail bounds, and fixed 90% requested-count usability thresholds. Compact rows preserve stable unavailable reasons without persisting raw nested fits. Nested execution reports `B * M` progress and forwards cancellation.
- Generic and PLS bootstrap outcomes, including percentile summaries, are exactly equal for one versus four workers in current tests.
- An automated CLI test runs the same fixture at one and four workers and proves exactly equal analytical payloads and diagnostics; only execution provenance such as worker count, generated ID, and timestamps differs.
- Generic deterministic `indexed_jackknife_v1` produces one ordered typed outcome per omitted case. BCa requires every transient PLS delete-one estimate to succeed, uses a clamped midrank bias proportion and all-case acceleration, and persists only compact summaries. Degenerate parameters retain nullable BCa rows with explicit reasons while percentile intervals remain available.
- Bootstrap and jackknife use distinct progress phases. Current v4 primary and BCa results are exactly equal at one versus four workers in current tests.
- Scoped Standard `freedman_lane_permutation_v1` is independently enabled with zero or 99 through 10,000 samples. For each direct path it conditions on fixed original construct scores, fits an intercept nuisance model, permutes nuisance residuals with an indexed path-domain ChaCha20 stream, reconstructs the target, and refits the full path equation using column-pivoted QR. Its supported interpretation remains conditional and approximate: measurement scores are not re-estimated, p values are raw and unadjusted, and current calibration is limited to homoscedastic Gaussian errors with exchangeable reduced-model residuals.
- The engine verifies that the full fixed-score regression reproduces the authoritative path statistic, reports `(exceedances + 1) / (P + 1)` for a two-sided absolute-coefficient test, supports cancellation and monotonic progress, and is exactly result-equal at one versus four workers.
- After requiring at least 90% usable replicates, PLS bootstrap reports original values, bootstrap means, bias, sample standard errors, normal-reference t statistics and two-sided p values, and Type-7 percentile bounds for weights, loadings, paths, effects, and R2. The t/p convention is not a studentized-bootstrap interval or resampling-under-the-null test. These are an initial inference slice, not complete validated inference.
- Bootstrap parameter identities use canonical JSON tuples, preventing collisions when construct or indicator IDs contain delimiters. `RESAMPLING_ENGINE_V4.md` is current; v1-v3 remain version-bounded compatibility contracts.
- Desktop and CLI expose independent bootstrap/permutation counts plus seed and workers. `PlsPmV3` can hold optional bootstrap and permutation artifacts independently; run history displays the Freedman-Lane path statistic, exceedances, count, and corrected two-sided p value.
- Desktop and CLI also expose the studentized inner count and persist an experimental high-cost warning. Run history displays inner count and bootstrap-t bounds alongside percentile and BCa columns.
- Researcher-facing app terminology now uses PLS-SEM for the run button, completed run name, method label, and analysis settings panel, while internal contracts continue using the precise `pls_pm` method identifier and PLS-PM method-version strings.
- Schema-v4 projects strictly validate permutation method/operation/seed/count provenance, canonical authoritative path coverage, original-statistic agreement, exceedance bounds, and recomputed plus-one p values. Requested artifacts cannot be silently omitted. Project coverage is twelve tests.
- Project validation accepts resampling v1-v4, rejects studentized data under legacy labels, and validates current nested settings, parameter coverage, usability counts, pivot quantiles, nullable reasons, and recomputed bounds.
- Bootstrap requests receive their own experimental warning; the base v0.3 PLS estimator warning has been retired after validation.
- Run history displays rho_A with visible row-level applicability/improper-estimate diagnostics, separate original HTMT and HTMT+ matrices with cell reasons, adjusted R2, VIF, Cohen f-squared, saturated/estimated SRMR and d_ULS, blindfolding D/Q2/PRESS/SSO, and percentile plus BCa inference. Wide bootstrap tables scroll horizontally while the parameter column remains sticky, and optional-field fallbacks support legacy payloads.
- The desktop job shell provides queued/running/cancelling/committing/completed/failed/cancelled states, in-kernel and replicate progress, cancellation, a four-active-job cap, panic finalization, project/read-only commit guards, consuming result retrieval, explicit dismissal, and bounded terminal retention. Four orchestration tests cover its lock and commit lifecycle.

Cohen f-squared, rho_A, original HTMT, HTMT+, adjusted R2, VIF, Q2, SRMR/d_ULS, normal-reference t/p, BCa, and nested studentized/bootstrap-t remain experimental. Structural Path Randomization v1 is separately release-qualified as a scoped Standard only for direct-path fixed-score Freedman-Lane inference under its documented exchangeability, homoscedastic-Gaussian, and raw-unadjusted-p-value boundaries. Studentized infrastructure/schema failures now produce a compact explicit failed artifact without changing primary, percentile, or BCa output. The studentized slice now has bounded 999x99 execution, independent supplied-value Python/R Type-7 agreement, multi-variant matched-resample PLS/cSEM external-reference evidence, broader corporate-reputation 4-construct/9-indicator/3-path matched-resample cSEM evidence, matched-resample python-plspm external-reference evidence, normal and heavy-tail pilot simulations, cancellation evidence, 1/2/4/detected-max worker invariance, compact-fixture minimum/default/outer-stress/maximum-inner performance evidence, a broader corporate-reputation 999x99 performance smoke benchmark, the full preregistered large-simulation studentized qualification run, and release-stress maximum outer-plus-inner plus broader corporate 999x199 performance evidence. Job recovery, atomic durability, and 10,000-replicate performance remain pending. Nothing in the remaining experimental slice is validated or publication-ready.

The executable Monte Carlo harness self-check, regenerated quick JSON, and pilot JSON pass, but remain infrastructure only. Quick mode fixes and enforces the P1 availability gates with 8 simulations per scenario and 79 bootstrap replicates. Pilot mode uses 32 simulations per scenario, `n=100`, and 199 bootstrap replicates as a fast early-warning screen. Both record `qualification.evaluated: false`; they are insufficient for coverage, type-I, power, or accuracy claims.

Current verification: core 17 tests; assessment 35; estimation 18 passed plus one ignored release benchmark; resampling 21; project 12; CLI 8; desktop job shell 6; frontend 31. The production build passes. The independent Decimal rho_A runner, cSEM rho_A comparison, independent HTMT/HTMT+ fixture runner, seminr HTMT+ comparison, assessment simulation, published assessment fixture, assessment evidence report generator, mediation validation scripts including R base-lm second source and published/example evidence, moderation validation scripts including R base-lm second source, published-formula evidence, published empirical-data evidence, bounded simulation, moderation inference, bounded inference qualification, release-oriented moderation coverage qualification, higher-order schema/editor/repeated-indicator/two-stage/hybrid estimator/reference/metamorphic validation plus hybrid invalid-split guard, PLSc reflective-only correction reference plus invalid-settings guard, Gaussian-copula endogeneity reference evidence, nonlinear quadratic-effects reference evidence, moderated mediation reference evidence, CTA-PLS tetrad reference evidence, WPLS case-weighted reference evidence, and CCA composite-residual reference evidence also pass.

The moderation validation loop now reuses the built `target/debug/qpls.exe` CLI for repeated import/inspect/run calls. `npm run qpls:moderation:validate` completed in 17.87 seconds on 2026-07-19 with all moderation evidence checks passing.

The HOC validation loop now reuses the built `target/debug/qpls.exe` CLI for repeated import/inspect/run calls. `npm run qpls:hoc:validate` now covers repeated-indicator, metamorphic, two-stage, and hybrid-reference checks; `npm run qpls:hoc:hybrid-guard` separately proves invalid hybrid component splits are rejected.

The frontend build now uses Vite manual chunks for React Flow, icons, Tauri bindings, and remaining vendor code. `npm run build` completes without the previous large-chunk warning; the largest generated JS asset on 2026-07-19 was 317.20 kB before gzip.

PLSc uses the current reflective path/factor contract in `docs/methods/PLSC_V2.md`; `docs/methods/PLSC_V1.md` is now an explicit legacy-compatibility disclosure. `npm run qpls:plsc:reference` independently evaluates Dijkstra-Henseler Equation 3 and verifies corrected rho_A, construct correlations, paths, R2, and loadings, while `npm run qpls:plsc:unsupported-guard` proves unsupported formative PLSc recipes are rejected before execution.

Gaussian-copula endogeneity diagnostics are now experimental. `docs/methods/PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md` records the rankit inverse-normal copula contract, and `npm run qpls:endogeneity:reference` verifies copula coefficients, standard errors, t-statistics, and skewness against an independent Python fixture with observed max delta `5.54e-09`.

Nonlinear effects are now experimental as a fixed-score quadratic diagnostic. `docs/methods/PLS_NONLINEAR_EFFECTS_V1.md` records the centered squared construct-score contract, and `npm run qpls:nonlinear:reference` verifies quadratic coefficients, standard errors, t-statistics, and R2 deltas against an independent Python fixture with observed max delta `1.96e-12`.

Moderated mediation is now experimental as a two-stage conditional indirect-effect diagnostic. `docs/methods/PLS_MODERATED_MEDIATION_V1.md` records the first-stage/second-stage contract, and `npm run qpls:moderated-mediation:reference` verifies conditional indirect effects and the moderated mediation index against an independent Python fixture with observed delta `4.67e-14`.

CTA-PLS is now experimental as a descriptive tetrad diagnostic. `docs/methods/PLS_CTA_PLS_V1.md` records the sample-covariance tetrad contract, and `npm run qpls:cta:reference` verifies all tetrad pairings and the invalid-block guard against an independent Python fixture with observed max delta `4.94e-14`.

WPLS is now experimental as a positive case-weight estimator. `docs/methods/PLS_WPLS_V1.md` records the weighted-standardization, weighted-covariance, weighted-score, and weighted-path contract, and `npm run qpls:wpls:reference` verifies paths, weights, loadings, R2, weight metadata, and invalid-weight guards against an independent Python fixture with observed max delta `3.41e-13`.

CCA is now experimental as a descriptive composite-residual diagnostic. `docs/methods/PLS_CCA_V1.md` records the recursive composite correlation residual contract, and `npm run qpls:cca:reference` verifies observed, reproduced, residual, and max-residual values plus the invalid PCA guard against an independent Python fixture with observed max delta `3.51e-14`.

The desktop analysis catalog now mirrors those schema-recognized planned extended PLS method ids and marks `wpls`, `cca`, and `cta_pls` as experimental, so the GUI roadmap, backend recipe schema, and CLI guards remain aligned with estimator contracts.

## Accelerated Development Infrastructure

Complete:

- Added `validation/development_slices.json` as the machine-readable source of truth for release slices, promotion gates, evidence, blockers, and next actions.
- Added `qpls-core` registry parsing and invariant tests so a slice cannot be marked `validated` with open gates or stable output before validation.
- Added CLI commands `qpls roadmap` and `qpls gate <slice-id>` for daily planning and promotion checks.
- Added `qpls qualify v04-inference`, which runs the CLI 1/2/4 worker matrix, records a bounded 10,000-replicate bootstrap cancellation-latency smoke benchmark, records a bounded 999x99 cancellation-latency smoke benchmark triggered from the nested studentized-inner phase, validates or refreshes quick Monte Carlo evidence, validates pilot, sensitivity, bounded studentized, bounded studentized-sensitivity, and full Monte Carlo evidence when present, validates the bounded minimum 999x99 studentized execution artifact, validates the bounded 999x99 studentized worker matrix artifact, validates the bounded studentized performance benchmark, validates simple and corporate matched-resample external-reference reports, and writes `validation/results/v04_inference_qualification_quick.json`.
- Added npm aliases `npm run qpls:roadmap`, `npm run qpls:gate:v04`, `npm run qpls:probe:external`, `npm run qpls:pls:csem`, `npm run qpls:pls:published`, `npm run qpls:pls:plspm`, `npm run qpls:pls:pca`, `npm run qpls:rho-a:csem`, `npm run qpls:htmt:csem`, `npm run qpls:evidence:v03`, `npm run qpls:evidence:v04`, `npm run qpls:pilot:v04`, and `npm run qpls:qualify:v04`.
- Added `docs/ACCELERATED_DEVELOPMENT.md` with the method-slice factory procedure.
- Corrected `docs/METHOD_COMPATIBILITY.md` so bootstrap is tracked as experimental rather than unsupported.

Current acceleration target: continue v0.5 method slices while preserving the v0.3/v0.4 validation gates and experimental warnings. The v0.3 estimator-only CSV/HTML export split is implemented and tested. The v0.4 inference one-command harness, full Monte Carlo coverage/type-I qualification, normal/heavy-tail sensitivity Monte Carlo drift screen, supplied-value studentized Python/R Type-7 reference, multi-variant matched-resample PLS/cSEM external-reference fixture, broader corporate-reputation matched-resample cSEM fixture, matched-resample python-plspm external-reference fixture, bounded normal and heavy-tail 999x99 studentized Monte Carlo pilots, bounded minimum 999x99 studentized execution, bounded 999x99 studentized 1/2/4/detected-max-worker equality and timing evidence, bounded minimum/default/outer-stress/maximum-inner performance evidence, broader corporate-reputation 999x99 performance smoke evidence, release-stress maximum outer-plus-inner and broader corporate 999x199 performance evidence, ordinary bootstrap cancellation, nested studentized-inner cancellation smoke gates, executable sharded full `studentized-qualification` gate, resumable shard runner/status automation, and full preregistered studentized qualification are complete. The full studentized qualification has 40 of 40 shards complete; `coverage_beta_0_35`, `null_beta_0`, `heavy_tail_coverage_beta_0_35`, and `heavy_tail_null_beta_0` each reached 1,000/1,000 simulations with zero failures and full studentized availability. `validation/results/monte_carlo_studentized_qualification.json` evaluated successfully with `passed=true`, including studentized coverage `0.964`, studentized type-I/exclusion `0.03`, heavy-tail coverage `0.941`, and heavy-tail type-I/exclusion `0.054`. `validation/results/studentized_release_stress.json` passed with `maximum_outer_inner_1999x999` at 1,997,001 requested inner fits in about 174.25s and `broader_corporate_999x199` at 198,801 requested inner fits in about 695.02s on detected 12-worker local hardware. `qpls qualify v04-inference` now reports `qualification_passed=true`. v0.5 mediation has descriptive classification, bootstrap indirect-effect surfacing, independent equation evidence, R base-lm second-source evidence, published/example evidence, bounded simulation/metamorphic evidence, and a bounded indirect-effect randomization screen; v0.5 two-stage moderation now has independent, R base-lm, published-formula, published empirical-data, bounded simulation, inference-integration, bounded inference-qualification, and release-oriented coverage-qualification evidence. Broader v0.5 promotion remains blocked by other extended-method families.

## v0.4 Demo Evidence Project

Complete as regression infrastructure:

- Added `qpls demo create` to generate `validation/demo/quickpls_v04_demo.qpls` from the bundled corporate-reputation fixture through the normal import, model, recipe, estimation, assessment, bootstrap, permutation, and project-save paths.
- Added `qpls demo validate` to reload the project, rerun the recipe, and compare the canonical analytical output against `validation/demo/quickpls_v04_demo.expected.json` with exact structure and `1e-12` numeric tolerance.
- Added `validation/demo/quickpls_v04_demo.validation.json`, currently reporting `matches_expected: true`.
- Added a native `open_demo_project` command and desktop `Demo` toolbar action. The command builds and loads the demo project with dataset, model, analysis settings, and one saved run.

This demo is regression evidence for the current experimental engine. It is not independent method validation and must not be cited as publication-readiness evidence.

## v0.5 Extended PLS Models

Started as an experimental method slice:

- `pls_mediation_v1` stores direct, indirect, total, VAF, and descriptive mediation class labels in PLS results.
- Saved runs surface available bootstrap percentile, BCa, and bootstrap-t intervals for matching `indirect_effect` rows.
- `npm run qpls:mediation:reference` independently verifies the single-item mediation equations against QuickPLS with maximum absolute difference below `1e-12`.
- `npm run qpls:mediation:r-reference` verifies the same single-item mediation fixture against development-only R base `lm` as a second executable reference source, also below `1e-12`.
- `npm run qpls:mediation:published` reruns the documented cSEM `threecommonfactors` example, requires cSEM path-coefficient agreement, and independently verifies mediation direct/indirect/total/VAF/classification decomposition from the path matrix with exact agreement.
- `npm run qpls:mediation:metamorphic` verifies bounded generated-signal behavior, positive-affine invariance, row-order invariance, construct-order invariance, and degradation after mediator permutation.
- `npm run qpls:mediation:randomization` verifies the observed and permuted-mediator indirect effect against independent standardized OLS equations and runs a deterministic 199-permutation mediator-randomization screen.
- `ModelSpec.interactions` stores two-stage moderation metadata, the model inspector can create interaction placeholder constructs, native recipe validation warns that moderation is experimental, and `pls_two_stage_moderation_v1` generates product-score indicators from stage-1 construct scores before estimating the interaction path in stage 2.
- Saved runs surface moderation interaction effects, bootstrap interval lookups for the product path when available, and simple slopes at `-1`, `0`, and `+1` standardized moderator scores.
- `npm run qpls:moderation:reference` independently verifies the single-item two-stage moderation path and simple-slope coefficients against standardized OLS and checks positive-affine invariance, row-order invariance, construct-order invariance, complete-case missing-data row mapping, warning persistence, and degradation after moderator permutation.
- `npm run qpls:moderation:r-reference` verifies the same two-stage moderation fixture against development-only R base `lm` as a second executable reference source.
- `npm run qpls:moderation:published-formula` verifies a fixed-table standard moderated-regression formula fixture against independent standardized OLS equations. This is not a published empirical-data replication.
- `npm run qpls:moderation:published-empirical` verifies the 32-row `mtcars` empirical dataset moderation model `mpg ~ wt + hp + wt*hp` against independent standardized OLS equations and simple slopes.
- `npm run qpls:moderation:simulation` verifies 20 generated signal datasets and 20 generated null datasets against independent standardized OLS equations, with bounded recovery and null-signal thresholds.
- `npm run qpls:moderation:inference` verifies bounded percentile bootstrap, BCa, and Freedman-Lane permutation integration for the generated product path, including one-worker versus two-worker analytical-payload invariance.
- `npm run qpls:moderation:inference-qualification` verifies six generated signal and six generated null moderation datasets through the actual Freedman-Lane product-path permutation pipeline, with bounded signal detection and false-positive containment.
- `npm run qpls:moderation:coverage-qualification` verifies a heavier 48-run, 199-permutation release-oriented coverage screen; the current report detected 24/24 signal datasets, flagged 1/24 null datasets, matched independent standardized OLS within `1e-10`, and completed in 22.43 seconds.
- `ModelSpec.controls` adds backward-compatible control-variable path semantics. Core validation rejects malformed control declarations, estimation emits `control_estimates` matching the corresponding structural path coefficients, the path inspector can mark selected paths as controls, native recipes serialize the marked paths, and saved runs display control paths separately when present.
- `ModelSpec.higher_order_constructs` adds a backward-compatible higher-order construct recipe contract for repeated-indicator, two-stage, and hybrid declarations. Core validation rejects duplicate ids, unknown HOC ids, insufficient components, self-components, unknown components, duplicate components, and invalid hybrid component splits while warning that HOC methods remain experimental. The construct inspector can mark HOC constructs, choose repeated-indicator, two-stage, or hybrid methods, select lower-order components, and serialize the declaration into native recipes. Repeated-indicator HOCs are expanded into ordered component-indicator blocks before PLS execution and assessment uses the same expanded execution recipe. `npm run qpls:hoc:reference` independently verifies paths, HOC loadings, and HOC weights with observed max absolute difference `4.49e-14`; `npm run qpls:hoc:metamorphic` passes affine, row-order, construct-order, component-order, warning, and degradation checks. Two-stage HOCs now use lower-order component scores as generated stage-2 HOC indicators, with assessment support; `npm run qpls:hoc:two-stage` independently verifies generated HOC paths/loadings/weights with observed reference delta `5.38e-14` and passes affine, row-order, construct-order, component-order, assessment, warning, and degradation checks. Hybrid HOCs now use an experimental indicator-split contract; `npm run qpls:hoc:hybrid-reference` independently verifies hybrid paths/loadings/weights with observed reference delta `1.37e-14`, and `npm run qpls:hoc:hybrid-guard` proves one-indicator components are blocked with `higher_order.hybrid_component_indicators`.
- `AnalysisMethod::Plsc` now emits method-versioned `plsc_v2` results using the shared canonical Dijkstra-Henseler rho_A equation, corrected construct correlations, corrected structural paths, bounded corrected R2, corrected outer loadings, and explicit inadmissibility guards. Legacy `plsc_v1` archives remain readable with a non-current warning because v1 used a provisional reliability expression that its Python reference repeated. `npm run qpls:plsc:validate` regenerates the canonical independent-reference comparison and invalid-settings rejection evidence.
- `AnalysisMethod::Endogeneity` now has an experimental Gaussian-copula diagnostic contract. QuickPLS emits `gaussian_copula_endogeneity_v1` with rankit inverse-normal copula terms, augmented-regression diagnostics, and applicability warnings. `npm run qpls:endogeneity:reference` verifies the diagnostic against an independent Python fixture.
- `AnalysisMethod::NonlinearEffects` now has an experimental quadratic diagnostic contract. QuickPLS emits `pls_quadratic_nonlinear_effects_v1` with centered squared construct-score terms, augmented-regression diagnostics, and R2 deltas. `npm run qpls:nonlinear:reference` verifies the diagnostic against an independent Python fixture.
- `AnalysisMethod::ModeratedMediation` now has an experimental conditional indirect-effect contract. QuickPLS emits `pls_moderated_mediation_v1` with conditional indirect effects at standardized moderator scores `-1`, `0`, and `+1`, plus the index of moderated mediation. `npm run qpls:moderated-mediation:reference` verifies the diagnostic and invalid-recipe guard.
- `AnalysisMethod::CtaPls` now has an experimental sample-covariance tetrad diagnostic contract. QuickPLS emits `cta_pls_tetrad_v1` with all three tetrad pairings for indicator quadruples, max absolute tetrad summaries, and publication warnings. `npm run qpls:cta:reference` verifies the diagnostic and invalid-block guard.
- `AnalysisMethod::Wpls` now has an experimental case-weighted PLS contract. QuickPLS emits `wpls_case_weighted_v1` with positive case-weight metadata, weighted outer weights/loadings, weighted paths, and weighted R2. `npm run qpls:wpls:reference` verifies the estimator and invalid-weight guards.
- `AnalysisMethod::Cca` now has an experimental composite residual contract. QuickPLS emits `cca_composite_residual_v1` with observed/reproduced/residual composite correlations and max absolute residual. `npm run qpls:cca:reference` verifies the diagnostic and invalid PCA guard.
- Saved-run result panels now surface method-specific payloads for PLSc, WPLS, CCA, CTA-PLS, Gaussian-copula endogeneity, nonlinear effects, and moderated mediation, including method warnings and diagnostic tables.
- The analysis catalog and top-bar run selector now persist a typed method choice, pass the selected method into native analysis recipes, expose runnable validated/experimental methods, and provide the WPLS case-weight column selector.
- Export-safe method tables now cover PLSc, WPLS, CCA, CTA-PLS, Gaussian-copula endogeneity, nonlinear effects, and moderated mediation. The Reports workspace previews and downloads watermarked CSV/HTML tables, and `qpls export --include-experimental` emits watermarked extended-method rows while the default CLI export remains conservative.
- The Reports workspace now includes a saved-run comparison view for R2 and path coefficients with baseline, comparison, and delta columns.
- Publication SVG diagrams now export from the Reports workspace using the current model layout and selected-run path, R2, and loading estimates, with experimental watermarking when needed.
- XLSX report tables now export from both `qpls export --format xlsx` and the native desktop Reports workspace, with workbook read-back tests. Reports also exposes a browser print/PDF path from the same watermarked HTML report.
- `qpls evidence v05-extended-pls` now writes `validation/results/v05_extended_pls_evidence.json`, aggregating 27 v0.5 validation reports. The current artifact reports all listed v0.5 evidence present and passed, with no open v0.5 registry gates.

The current v0.5 preview gate is clear. The methods remain experimental and watermarked until future validation promotion criteria are met.

## v0.6 Prediction and Heterogeneity

Started, with the first prediction slice implemented:

- `AnalysisMethod::Predict` now runs an experimental `plspredict_holdout_v1` contract. It uses complete-case rows, assigns every fourth complete row to a deterministic test split, computes preprocessing parameters and PLS weights/paths on training rows only, applies training transforms/weights to test rows, and reports RMSE, MAE, benchmark errors, construct-score LM benchmark errors, and Q2 predict for endogenous constructs.
- The same payload now includes bounded `plspredict_repeated_kfold_v1` metrics when at least 15 complete observations are available. It runs 3 deterministic repeats of 5 folds, recomputing preprocessing, weights, paths, PLS predictions, LM benchmarks, and paired loss comparisons from each fold's training rows only.
- The repeated-fold block now includes an early CVPAT diagnostic comparing PLS squared prediction loss against the training-mean benchmark, the construct-score LM benchmark when available, and metadata-configured reduced structural models that drop specified direct paths. It reports mean paired loss difference, standard error, t statistic, two-sided p value, observations, preferred model, and warnings.
- `docs/methods/IPMA_V1.md` freezes a bounded IPMA contract. `AnalysisMethod::Ipma` uses predecessor PLS total effects as construct importance and observed-range 0-100 min-max scaled listwise-standardized construct/indicator scores as performance. Recipes can set metadata `ipma_targets` to a comma-separated endogenous target list; the native workbench selects one explicit target per run. Theoretical-range correction and cIPMA are unsupported. `npm run qpls:ipma:reference` writes `validation/results/ipma_reference_report.json`; the independent Python fixture verifies the bounded construct and indicator rows.
- `docs/methods/PLS_POS_BOUNDED_V1.md` freezes the legacy bounded segmentation contract, while `docs/methods/PLS_POS_V1.md` now defines experimental generalized 2-5 segment PLS-POS output with deterministic starts, minimum segment share, stable memberships, objective history, segment path estimates, and segment R2. `npm run qpls:pos:recovery` and `npm run qpls:v06:validate` write `validation/results/v06_group_methods_reference_report.json`; the current integrated fixture recovers 3 segments with objective improvement about `0.6792`.
- `docs/methods/FIMIX_PLS_V1.md` defines an experimental 2-3 class FIMIX-PLS preview with deterministic starts, posterior-style probabilities, memberships, class paths/R2, log-likelihood, AIC, BIC, CAIC, and entropy. `npm run qpls:fimix:recovery` writes the same integrated v0.6 report; the current fixture emits 3 classes with finite BIC and entropy about `0.5037`.
- `docs/methods/PLS_MGA_TWO_GROUP_V1.md` freezes the observed two-group MGA contract, and `docs/methods/PLS_MGA_PERMUTATION_V1.md` adds experimental permutation MGA. `AnalysisMethod::Mga` requires metadata `mga_group_column`, estimates each observed group independently with the PLS-PM engine, and can re-estimate deterministic group-label permutations when `group_methods` includes `mga_permutation`. `npm run qpls:mga:reference` still writes the original independent reference report; `npm run qpls:mga:permutation-reference` writes the integrated v0.6 report.
- `docs/methods/MICOM_V1.md` now records that the former two-group MICOM routine and its validation claim are withdrawn. `group_methods = "micom"` is blocked by core validation and estimator execution; historical payloads remain readable only.
- `docs/methods/PLSPREDICT_HOLDOUT_V1.md` freezes the initial holdout/repeated-fold/CVPAT contract and explicitly excludes separate saved-model CVPAT, seeded/random repeated folds, indicator-level PLSpredict tables, MGA, MICOM, FIMIX-PLS, PLS-POS, generated interactions, higher-order constructs, and case-weighted prediction.
- The runner, assessment compatibility guard, desktop method selector, analysis settings, run history, tabbed Groups workspace, report table builder, native XLSX export path, and CLI experimental export rows now recognize the prediction payload, repeated-fold block, CVPAT comparisons, LM benchmark fields, IPMA, PLS-POS, FIMIX-PLS, MGA, MICOM, and permutation-MGA payloads.
- `npm run qpls:plspredict:reference` writes `validation/results/plspredict_holdout_reference_report.json`; the current report passes method-version, split-count, target-shape, predictive-improvement, LM benchmark, repeated-k-fold plan, repeated-k-fold improvement, repeated-k-fold LM benchmark, benchmark CVPAT, drop-path model-pair CVPAT, Q2 threshold, and experimental-warning checks.
- The v0.6 registry retains passed gates for PLSpredict, repeated-k-fold/LM benchmark prediction, CVPAT diagnostics, IPMA, generalized PLS-POS, FIMIX-PLS, two-group MGA, permutation MGA, integrated validation fixtures, CLI exports, report tables, saved-run surfacing, and the tabbed Groups workspace. Its historical MICOM gate is superseded by the execution-withdrawal safety audit.

PLSpredict holdout remains experimental and watermarked. It is a development preview slice, not a validated publication method.

## v0.7 CB-SEM Beta

Implemented as an experimental beta:

- `AnalysisMethod::Cbsem` is runnable from the desktop and CLI with metadata-controlled CFA/SEM mode, ML estimator label, raw input, optional mean-structure flag, standardized-solution selection, optional group column, invariance-step selection, and optional CB-SEM bootstrap interval preview.
- `cfa_ml_v1`, `cbsem_ml_v1`, `cbsem_fit_v1`, `cbsem_modification_indices_v1`, `cbsem_bootstrap_v1`, and `cbsem_multigroup_v1` payloads are surfaced in saved runs, reports, CLI experimental exports, and `qpls evidence v07-cbsem`.
- v0.7.1 adds a direct single-group ML optimizer for supported raw-data reflective CFA/SEM models and lavaan parity fixtures for one-factor CFA, correlated two/three-factor CFA, latent regression SEM, latent mediation SEM, and correlated-exogenous SEM.
- Core validation blocks formative constructs, generated interactions, higher-order constructs, case weights, and latent factors with fewer than two indicators.
- Method specs are frozen in `docs/methods/CBSEM_ML_V1.md`, `docs/methods/CFA_ML_V1.md`, `docs/methods/CBSEM_FIT_V1.md`, `docs/methods/CBSEM_MODIFICATION_INDICES_V1.md`, and `docs/methods/CBSEM_MULTIGROUP_INVARIANCE_V1.md`.
- `npm run qpls:v07:validate` writes `validation/results/cbsem_v07_reference_report.json` and `validation/results/cbsem_lavaan_reference_report.json`.

This is not publication-validated CB-SEM. The current direct optimizer is validated only for the bounded single-group raw-data reflective fixtures above. Exact constrained multigroup refits, robust/ordinal/FIML estimators, broad inadmissibility coverage, and second-source numerical agreement remain future promotion requirements.

## v0.8 Extended Methods

Implemented as an experimental preview:

- `AnalysisMethod::Pca` runs `pca_v1` as a standalone standardized raw-data PCA workflow with deterministic sign orientation, component retention metadata, eigenvalues, explained variance, loadings, weights, and scores.
- `AnalysisMethod::Regression` runs `regression_ols_v1`, `regression_logistic_v1`, and bounded `regression_process_v1` depending on `regression_type` metadata. OLS reports fit, coefficients, HC-style standard errors, confidence intervals, predictions, and residuals. Logistic regression reports deterministic IRLS estimates, odds ratios, pseudo-R2, AIC/BIC, and predicted probabilities. PROCESS-style output currently covers bounded mediation/moderation effect rows and simple slopes.
- The original extended-method preview emitted `nca_v1`; that numerical interpretation is now superseded and retained only for explicit legacy archive compatibility. Current execution emits `nca_v2` with record-high CE-FDH peers, CR-FDH regression through those peers, seeded permutation p values, and status-bearing observed-range bottleneck rows.
- The historical extended-method preview emitted `gsca_v1`; it is now legacy/archive-only. Current `AnalysisMethod::Gsca` execution emits independently checked `gsca_als_v2`, a bounded joint global least-squares ALS component model with strict native results, XLSX export, persistence, and same-run reopen evidence.
- Desktop analysis settings, saved runs, report tables, CLI experimental export rows, method specs, compatibility status, and evidence aggregation now include PCA, regression/PROCESS, NCA, and GSCA.
- `npm run qpls:v08:validate` retains the aggregate historical coverage report and refreshes a separate report for each v0.8 method. Single-section commands write only their matching `v08_<method>_reference_report.json`; they cannot overwrite another method's promotion evidence. Current GSCA evidence is method-specific: `npm run qpls:gsca:reference` writes `validation/results/gsca_als_v2_reference_report.json` and compares the joint global criterion, weights, loadings, paths, R2, fit, and residual diagnostics with an independent SciPy optimizer.

All v0.8 methods remain experimental and watermarked. They are suitable for workflow feedback and validation hardening, not unrestricted publication use.

## Publication-Readiness Audit

Started before v0.9:

- `publication_ready_v0_1_to_v0_8` is now a machine-readable promotion gate in `validation/development_slices.json`.
- `qpls evidence publication-ready` writes `validation/results/publication_ready_audit.json`.
- `npm run qpls:publication-ready:v01-v08` runs the evidence writer and prints the gate.
- All v0.1-v0.8 publication audit blockers are now closed for the documented supported scope. The remaining audit artifacts are `validation/results/extended_pls_publication_audit.json`, `validation/results/prediction_heterogeneity_publication_audit.json`, `validation/results/cbsem_publication_audit.json`, `validation/results/extended_methods_publication_audit.json`, `validation/results/gui_diagram_publication_audit.json`, `validation/results/stable_export_publication_audit.json`, `validation/results/documentation_publication_audit.json`, and `validation/results/performance_release_publication_audit.json`.
- Public claims must remain bounded to the method specifications and audit artifacts; unsupported estimators, inputs, model shapes, and export surfaces remain blocked or explicitly experimental.

This gate now reaches `0 open / 0 blocked` when regenerated by `npm run qpls:publication:all`.

## v0.9 Release Candidate

Implemented as a Windows release candidate for the documented supported scope:

- Version metadata is `0.9.0-rc.1` in npm, Rust workspace, and Tauri release configuration.
- `docs/RELEASE_NOTES_V0_9_RC1.md`, `docs/SUPPORTED_SCOPE_V0_9_RC1.md`, and `docs/DEPENDENCY_NOTICES.md` define the RC scope, export surface, unsigned installer status, dependency notices, SmartPLS project-import exclusion, and non-equivalence/no-reverse-engineering statements.
- `npm run qpls:v09:smoke` writes `validation/results/v09_smoke_check.json` and verifies release executable launch without a dev server, validation CSV fixture run/export, CSV/HTML/XLSX export readability, selected-run diagram estimate visibility, SVG export path, browser print/PDF path, and recovery coverage.
- `npm run qpls:v09:audit` writes `validation/results/v09_release_candidate_audit.json` and verifies publication audit currency, version consistency, release binary, NSIS installer artifact, docs, stable exports, smoke artifact, and the `v0_9_publication_release_candidate` gate.

The v0.9 RC installer is intentionally unsigned. Windows SmartScreen warnings are documented; code signing remains a later packaging task unless a certificate is provided.

## v0.9.3 Professional SEM Designer

Implemented for the designer scope:

- The editable `sem` canvas, result diagram, publication preview, and SVG export now share the same academic SEM visual grammar.
- Diagram layout metadata is persisted separately from the numerical recipe through `diagramVersion`, construct layouts, indicator layouts, edge layouts, viewport, and theme fields.
- Indicators can be moved on the canvas and their positions persist. Dropping an indicator near another construct reassigns it while preserving single ownership.
- Right-click menus provide direct canvas actions for constructs, indicators, paths, and the empty canvas, including rename, invert measurement model, align/reset indicators, route paths, mark controls, duplicate, and delete.
- Result and publication modes are locked against accidental edits; edit mode remains draggable.
- Reports can export the current canvas layout or a tidy publication layout, with SVG remaining the audited publication diagram format.

`npm run qpls:v093:sem-designer` writes `validation/results/v093_sem_designer_audit.json`, browser-smoke screenshots under `validation/results/screens/v093/`, and gates `v0_9_3_professional_sem_designer`.

## v1.0 Stable

Implemented for the documented stable scope:

- Version metadata is `1.0.0` in npm, Rust workspace, and Tauri release configuration.
- `docs/V1_SUPPORTED_SCOPE.md`, `docs/V1_COMPATIBILITY_MATRIX.md`, `docs/V1_KNOWN_DIFFERENCES.md`, `docs/METHODOLOGY_MANUAL_V1_0.md`, `docs/VALIDATION_ARTIFACT_INDEX_V1_0.md`, `docs/RELEASE_NOTES_V1_0.md`, `docs/INSTALLATION_V1_0.md`, and `docs/DEPENDENCY_NOTICES_V1_0.md` define the stable release boundary.
- `validation/v10_numerical_discrepancy_audit.py` aggregates method-family publication evidence and known-difference coverage for all v1.0-supported deterministic outputs.
- `validation/v10_product_scope_audit.py` verifies UI/CLI/export status wording, provenance/warnings, experimental opt-in, stale diagram overlay blocking, and absence of SmartPLS equivalence/import/reverse-engineering claims.
- `validation/v10_desktop_smoke_check.py`, `validation/v10_performance_audit.py`, and `validation/v10_release_packaging_audit.py` verify release launch, exports, recovery evidence, performance/reproducibility evidence, version metadata, release executable, NSIS installer, and explicit unsigned-installer status.

`npm run qpls:v10:release` regenerates publication evidence, reruns the v0.9.3 designer gate, builds the desktop release, runs all v1.0 audits, and gates `v1_0_stable`.

## Later Releases

Post-v1 work includes signed installer verification when a certificate is available, native audited PDF/PNG export, polychoric/WLSMV and FIML estimators, SmartPLS project import if legally and technically scoped later, and expanded residual/error/caption recipe semantics.

## v1.1.1 Native UX Hardening

Implemented as a stricter desktop UX hardening milestone on top of v1.1:

- `validation/v111_native_gui_workflow_smoke.py` launches the release executable with smoke UI enabled, runs the production native workflow smoke, verifies import-style ingestion, PLS execution, save/reopen, layout persistence, and XLSX export evidence, and records whether `pywinauto` is available for literal Windows UI Automation.
- `validation/v111_sem_designer_dense_smoke.mjs` captures medium, large, mediation, and formative SEM diagrams at desktop viewport and checks SmartPLS-style arrangement, structural edge rendering, and latent overlap behavior.
- `validation/v111_settings_ux_smoke.mjs` verifies method settings progressive disclosure: recommended defaults stay visible while resampling/reproducibility controls are collapsed until opened.
- `validation/v111_report_export_parity.mjs` compares WYSIWYG preview SVGs and exported SVGs across model-only, completed-result, large, formative, current-layout, and tidy-layout cases.
- `validation/v111_keyboard_native_smoke.py` launches the release executable and reuses the completed-result keyboard workflow contract for Results and Report surfaces.
- `validation/v111_disabled_actions_audit.py` verifies low-frequency disabled actions have visible or accessible reasons across Run, Report, Data, canvas lock, covariance/path, native-only XLSX, and settings surfaces.

`npm run qpls:v111:release` runs the hardening suite and gates `v1_1_1_native_ux_hardening`. Mobile remains non-gating. SVG remains the audited publication export; native PDF/PNG remains post-v1 unless separately implemented and audited.

## v1.3 SEM Designer UX Overhaul

Status: validated.

- The academic SEM diagram is now the default editable canvas style rather than only a publication/result view.
- Edit mode exposes subtle latent-construct connection handles for visual path creation while result and publication modes remain locked.
- Construct labels no longer use a button surface that can interfere with dragging; double-click rename remains available.
- Edge labels support pointer dragging and keyboard nudging/reset, improving label collision control in dense diagrams.
- Duplicate/self-path and duplicate covariance attempts now show nearby canvas feedback instead of failing silently.
- Context-menu and toolbar actions cover rename, duplicate, delete, reverse, route, control, covariance conversion, indicator side movement, reset layout, and fit/arrange actions.

Evidence:

- `validation/results/v13_sem_designer_ux_smoke.json`
- `validation/results/v13_sem_designer_ux_audit.json`
- `validation/results/screens/v13/sem-designer/`

Command:

`npm run qpls:v13:sem-designer`

## v1.2 Method Promotion Program

Started as the next development milestone:

- Added the active registry slice `v1_2_method_promotion_program`.
- Added `docs/METHOD_PROMOTION_CRITERIA.md` to define the evidence required before an experimental calculation can become researcher-ready.
- Added `docs/METHOD_PROMOTION_PROGRAM_V1_2.md` to define the work packages, first promotion batch, and product-enforcement rules.
- Added `validation/method_promotion_program_audit.py`, which writes `validation/results/method_promotion_program_audit.json` and verifies that the promotion framework is wired into docs, registry, and the current method backlog.
- Updated `validation/promotion_matrix.py`, which writes `validation/results/method_promotion_matrix_v1_2.json` with first-batch promotion rows and later-batch backlog summaries.
- Added `validation/pls_core_method_promotion_audit.py`, which promotes deterministic PLS core estimator-only output while keeping assessment and inference separately gated.
- Added `validation/assessment_method_promotion_audit.py`, which promotes documented assessment metrics while keeping d_G, NFI, RMS_theta, and inference separately gated.
- Added `validation/inference_method_promotion_audit.py`, which promotes documented PLS inference/resampling procedures while excluding unsupported shapes and unaudited stochastic claims.
- Added `validation/pca_method_promotion_audit.py`, which promotes standalone PCA for documented raw-data PCA scope while excluding rotations, pairwise deletion, covariance/correlation input, and inference.
- Added `validation/ols_method_promotion_audit.py`, which promotes documented OLS regression with HC3 standard errors while keeping logistic, PROCESS, HC0/HC4 claims, and advanced regression families excluded.
- Added `validation/method_promotion_product_enforcement_audit.py`, which verifies product-facing status enforcement across the method catalog, readiness panel, top bar, run workspace, result/export tables, and newly generated engine warnings.
- Added npm aliases `qpls:promotion:pls-core`, `qpls:promotion:assessment`, `qpls:promotion:inference`, `qpls:promotion:pca`, `qpls:promotion:ols`, `qpls:promotion:product-enforcement`, `qpls:promotion:program`, `qpls:promotion:gate`, and `qpls:v12:method-promotion`.

The setup gates are passed, and the full first calculation batch is now promoted for documented scopes: PLS core estimator-only output, documented assessment metrics, documented PLS inference/resampling, standalone PCA, and OLS regression. Product labels and exports now enforce this bounded scope: PCA and OLS can appear as validated, while logistic regression, PROCESS, NCA, CB-SEM, GSCA, segmentation, and other unpromoted methods remain experimental or watermarked. The milestone intentionally remains open for second-source and simulation expansion.

## v1.2.1 Second-Batch Method Promotion

Implemented as a bounded method-promotion milestone:

- Added `v1_2_1_second_batch_method_promotion` to the registry.
- Added method-specific promotion audits for mediation, two-stage moderation, PLSc, WPLS, IPMA, PLSpredict/CVPAT, and NCA.
- Added `validation/second_batch_product_enforcement_audit.py` and `validation/second_batch_method_promotion_audit.py`.
- Updated product-facing statuses so PLSc, WPLS, and Deterministic Construct Prediction are validated only for documented scopes. NCA v2 is numerically and archive qualified for its bounded standalone raw-data scope, while packaged-native workflow acceptance remains open. IPMA has a fresh packaged run, native XLSX export, explicit save, and same-run reopen evidence for its bounded predecessor-only observed-range scope.
- Updated engine warning text and result/export table status for promoted second-batch payloads.
- Added method-scope aliases `MEDIATION_V1.md`, `TWO_STAGE_MODERATION_V1.md`, and `PLSPREDICT_V1.md`.

## v1.2.2 Group, Prediction, And Regression Promotion

Implemented as a bounded method-promotion milestone:

- Added `v1_2_2_group_prediction_regression_promotion` to the registry.
- Added method-specific audits for permutation MGA, PLS-POS, FIMIX-PLS, logistic regression, and bounded PROCESS mediation/moderation; the former MICOM promotion audit now verifies its safety withdrawal.
- Added `validation/third_batch_product_enforcement_audit.py` and `validation/third_batch_method_promotion_audit.py`.
- Updated product-facing statuses so MGA, FIMIX-PLS, PLS-POS, logistic regression, and bounded PROCESS are validated only for documented scopes; MICOM is not a validated or executable scope.
- Updated engine warning text and result/export table status for promoted third-batch payloads.
- Updated method docs and compatibility notes while keeping moderated mediation, CB-SEM/CFA, GSCA, HOC, nonlinear effects, endogeneity, CCA, and CTA-PLS experimental.

## v1.2.3 / v1.2.4 Final Method Promotion

Implemented as the final bounded method-promotion milestones:

- Added `v1_2_3_extended_pls_diagnostics_promotion`, `v1_2_4_cbsem_gsca_promotion`, and `v1_2_5_method_promotion_completion` to the registry.
- Added method-specific promotion audits for higher-order constructs, nonlinear effects, Gaussian-copula endogeneity diagnostics, CCA, CTA-PLS, PLS moderated mediation, CB-SEM/CFA, and GSCA.
- Added `validation/fourth_batch_product_enforcement_audit.py`, `validation/fourth_batch_method_promotion_audit.py`, `validation/fifth_batch_product_enforcement_audit.py`, `validation/fifth_batch_method_promotion_audit.py`, and `validation/method_promotion_completion_audit.py`.
- Updated product-facing statuses so the remaining methods are validated only for documented bounded scopes.
- Updated engine warning text and result/export table status for promoted fourth- and fifth-batch payloads.
- Updated method docs, alias docs, and compatibility notes while keeping unsupported variants excluded: CB-SEM bootstrap and unrestricted multigroup/invariance, robust/ordinal/FIML estimators, broad constraints, unrestricted GSCA variants, broader nonlinear SEM, bootstrap-based CCA/CTA decisions, unsupported HOC variants, and the full Hayes PROCESS catalogue.

The bounded IPMA native promotion is accepted: canonical reference, source, strict project persistence, genuine packaged execution, native XLSX export, explicit save, and same-run reopen all pass. This does not promote theoretical-range correction, alternate SmartPLS representations, NCA integration, resampling inference, or cIPMA.
## v1.3.1 SEM Diagram Geometry Polish

Complete:

- Added shared SEM geometry for canvas and SVG export boundary routing.
- Structural arrows now attach to latent oval boundaries; indicator measurement arrows attach to rectangle boundaries.
- SmartPLS-like editable diagrams use larger ovals, larger indicator boxes, clearer labels, stronger structural arrows, lighter measurement arrows, and distinct covariance arcs.
- Added context commands for `Auto-place indicators`, `Tidy selected construct`, and `Tidy labels`.
- Added visual smoke and static audits for dense diagram quality and SVG/canvas parity.

## v1.3.2 SEM Canvas Toolbar Redesign

Complete:

- Replaced the long scrollable SEM canvas toolbar with a compact primary toolbar, contextual object toolbar, and grouped View/Results controls.
- Permanent toolbar now prioritizes core desktop modeling actions: undo, redo, select, pan, construct, path, covariance, arrange, fit, validate, view, results, and help.
- Construct, indicator, path, covariance, and multi-selection actions now appear only when relevant.
- Arrange, View, and Results controls use dropdowns to avoid normal desktop horizontal overflow.
- Theme, grid/minimap, and layout-lock controls are functional and covered by toolbar smoke evidence.
- Indicator reassignment, construct pin/unpin, and multi-selection alignment/distribution are covered by contextual-toolbar evidence.
- Added `validation/v132_toolbar_smoke.mjs`, `validation/v132_toolbar_audit.py`, and the registry slice `v1_3_2_sem_canvas_toolbar_redesign`.

## v1.3.3 SEM Explorer Sidebar Redesign

Complete:

- Replaced the dense left model tree and duplicated Data/Model tab strip with a SEM-native Explorer.
- Added dedicated Constructs, Variables, Structure, and Issues tabs.
- Exposed common construct, variable, path, covariance, and model-issue actions directly in the sidebar.
- Added resizable/collapsible desktop sidebar state that remains UI-only and separate from numerical recipes.
- Added sidebar-to-canvas focus events for constructs and paths.
- Added `validation/v133_sem_sidebar_smoke.mjs`, `validation/v133_sem_sidebar_audit.py`, and the registry slice `v1_3_3_sem_explorer_sidebar_redesign`.

## v1.4 Frontend Success Program

Complete:

- Added a reusable desktop design system surface for page headers, action strips, cards, tabs, empty states, and status badges.
- Added a desktop-first Start workspace with new/open/demo/import/recent project entry points.
- Reworked method setup into Basic/Expert modes with researcher presets and readiness cards.
- Reworked results into workflow tabs with search, density controls, copy actions, and result-to-diagram focus hooks.
- Added publication/report presets for thesis, journal figure, journal tables, presentation, and reproducibility reports.
- Extended SEM designer controls with large-model collapse, isolate neighborhood, clear isolation, and fit selected actions.
- Strengthened explorer/inspector usability through global search, issue filters, and contextual result details.
- Added `validation/v14_frontend_success_audit.py`, `validation/v14_frontend_success_smoke.mjs`, and registry slices through `v1_4_frontend_success_program`.
- Updated versioned build metadata to `1.4.7` with artifact label `v1_4_frontend_success_program`.

Scope note: v1.4 is frontend/product-only. It does not change statistical engines, analysis recipes, result schemas, validation tolerances, or numerical fingerprints.

## v1.5.0 Researcher UX Refinement

Complete:

- Added a desktop command palette (`Ctrl+K`) for common navigation, method presets, SEM canvas actions, and export entry points.
- Added a keyboard shortcut overlay (`?`) with the core SEM designer shortcuts.
- Added toast notifications for save/open/recovery/import/export/run-complete feedback.
- Added clearer status-bar autosave and shortcut access messaging.
- Added a method setup “What will run” summary so researchers can verify method, data/model counts, seed, workers, resampling, and scope before running.
- Added Results headline cards, current-table export, and clearer table-to-diagram focus wording.
- Added a four-step publication export flow: select run, choose diagram style, preview figure, export tables/SVG.
- Added variable prefix grouping chips in the SEM Explorer.
- Added `validation/v150_researcher_ux_smoke.mjs`, `validation/v150_researcher_ux_audit.py`, and the registry slice `v1_5_0_researcher_ux_refinement`.
- Updated versioned build metadata to `1.5.0` with artifact label `v1_5_0_researcher_ux_refinement`.

Scope note: v1.5.0 is frontend/product-only. It does not change statistical engines, analysis recipes, result schemas, validation tolerances, or numerical fingerprints.

## v1.5.1 Navigation Workspace Hardening

Complete:

- Redesigned the primary left rail into Home, Data, Model, Setup, Run, Results, and Report.
- Renamed the previous Start workspace to Home and added current-project/autosave status.
- Renamed the previous Validate rail item to Setup because that page owns method selection, validation, and readiness.
- Removed Groups from the permanent rail while preserving group workflows through Setup and the Results Groups tab.
- Added next-step guidance in Data, Setup, Results, and Report so every workspace has a clear forward action.
- Added `validation/v151_navigation_smoke.mjs`, `validation/v151_navigation_audit.py`, and the registry slice `v1_5_1_navigation_workspace_hardening`.
- Updated versioned build metadata to `1.5.1` with artifact label `v1_5_1_navigation_workspace_hardening`.

Scope note: v1.5.1 is frontend/navigation-only. It does not change statistical engines, analysis recipes, result schemas, validation tolerances, or numerical fingerprints.

## v1.5.2 Data Workspace Hardening

Complete:

- Reorganized Data into Import Source, Data Quality, and Preview And Metadata zones.
- Replaced duplicate validation-fixture actions with one researcher-facing `Load Sample Dataset` action.
- Added raw, covariance, and correlation import-mode guidance with inline matrix sample-size readiness.
- Added data-quality cards for rows, variables, missing cells, nonnumeric variables, constant columns, header issues, and sample readiness.
- Added variable search/filter, clearer selected-column metadata editing, and a visible horizontal-scroll hint.
- Added `Create Constructs From Prefixes` with detected prefix preview and handoff to the Model designer.
- Added `validation/v152_data_workspace_smoke.mjs`, `validation/v152_data_workspace_audit.py`, and the registry slice `v1_5_2_data_workspace_hardening`.
- Updated versioned build metadata to `1.5.2` with artifact label `v1_5_2_data_workspace_hardening`.

Scope note: v1.5.2 is frontend/Data-workspace-only. It does not change statistical engines, analysis recipes, result schemas, validation tolerances, or numerical fingerprints.
## v1.5.3 Layout, Copy, And Readiness Polish

Status: validated.

This frontend-only milestone closes the visible screen-review issues from the v1.5.2 SmartPLS-user audit: card text collisions, repeated global warnings, scroll-state confusion, sparse readiness pages, Setup scope wording, Report control alignment, and missing local disabled reasons. Evidence is recorded in `validation/results/v153_layout_copy_smoke.json` and `validation/results/v153_layout_copy_audit.json`.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

## v2.0.2 Setup Method Guidance Redesign

Status: validated.

This frontend-only milestone continues the QuickPLS 2.0 mockup-matching program by rebuilding Setup as a professional guided calculation workspace.

Complete:

- Replaced the older flat Setup page with a selected calculation command panel, readiness and scope panel, guided method browser, selected-method requirements sidecar, research presets, launch summary, and calculation preview.
- Added method-specific requirement checks with exact next actions beside the selected method.
- Kept Basic/Expert setup modes while moving advanced reproducibility and group/prediction settings into the selected-method sidecar.
- Added `setup-v2-*` styles that reuse the QuickPLS 2.0 desktop design tokens.
- Added `validation/v202_setup_guidance_smoke.mjs`, `validation/v202_setup_guidance_audit.py`, `docs/V2_0_2_SETUP_METHOD_GUIDANCE_REDESIGN.md`, and the registry slice `v2_0_2_setup_method_guidance_redesign`.
- Release metadata now uses `2.0.2`; versioned desktop artifacts use the label `v2_0_2_setup_method_guidance_redesign`.

## v2.0.3 Visual Fidelity Foundation

Status: validated.

This frontend-only milestone freezes the selected QuickPLS 2.0 mockup direction as a repeatable visual contract.

Complete:

- Added `docs/V2_UI_VISUAL_CONTRACT.md` for desktop target viewports, shell rules, shared tokens, component primitives, typography, status wording, and acceptance criteria.
- Hardened `src/styles.css` with shared `--q2-*` page, panel, action, chip, shadow, and typography tokens.
- Standardized reusable `.qpls2-*` primitives for page titles, subtitles, panels, command rows, cards, chips, and actions.
- Added Results v2 shell styling hooks so the next Results workspace pass uses the same foundation.
- Added `validation/v203_visual_fidelity_smoke.mjs`, `validation/v203_visual_fidelity_audit.py`, `docs/V2_0_3_VISUAL_FIDELITY_FOUNDATION.md`, and the registry slice `v2_0_3_visual_fidelity_foundation`.
- Release metadata now uses `2.0.3`; versioned desktop artifacts use the label `v2_0_3_visual_fidelity_foundation`.

## v2.0.4 Results Table And Interpretation Redesign

Status: validated.

This frontend-only milestone applies the QuickPLS 2.0 visual contract to the Results workspace.

Complete:

- Added a `Result workbook` navigation header and clearer selected-run context.
- Added tab-aware Results lens guidance for overview, measurement, structural, validity, inference, prediction, groups, diagnostics, interpretation, and comparison views.
- Added compact evidence cards for tab-specific findings, report action, and scope/status context.
- Upgraded result table section headers with row count, visible-column count, construct count, and wide-table guidance.
- Added `validation/v204_results_redesign_smoke.mjs`, `validation/v204_results_redesign_audit.py`, `docs/V2_0_4_RESULTS_TABLE_INTERPRETATION_REDESIGN.md`, and the registry slice `v2_0_4_results_table_interpretation_redesign`.
- Release metadata now uses `2.0.4`; versioned desktop artifacts use the label `v2_0_4_results_table_interpretation_redesign`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.11.0 Method Applicability Setup Polish

Status: in validation.

This frontend-only milestone improves Setup so QuickPLS explains which analyses fit the current data, SEM model, settings, and documented method scope.

Complete:

- Added a Setup availability summary for recommended, available, setup-required, and blocked/scoped methods.
- Added card-level missing requirement copy and selected-method "why not available yet" guidance.
- Kept Data and Model guidance wired to the same applicability engine.
- Kept the top-bar method selector conservative, with broader discovery in Setup.
- Added targeted smoke/audit scripts and v2.11 milestone documentation.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.9.0 Acceptance Backlog And Next Pass

Status: validated.

This frontend/product governance milestone converts the current rendered shell evidence and release handoff state into a grouped acceptance backlog for the next QuickPLS 2.x UI pass.

Complete:

- Added a rendered smoke that captures Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings at `1440x900` and `1280x800`.
- Generated `validation/results/v290_acceptance_backlog.json` with `do_next`, `defer`, and `do_not_do` workstreams.
- Added a static audit that verifies backlog structure, version metadata, scripts, docs, registry state, and frontend-only boundaries.
- Updated release metadata to `2.9.0` and artifact labeling to `v2_9_0_acceptance_backlog_and_next_pass`.

Boundary: frontend/product backlog governance only; no statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes.

## v2.8.0 Release Handoff Consistency

Status: validated.

This documentation/release milestone aligns QuickPLS 2.x public handoff instructions with the current app version, verification gate, screenshots, and artifact workflow.

Complete:

- README, installation, and source-build docs now identify `v2.8.0` as the current development release.
- Build and verification commands point to `npm run qpls:v280:release-handoff` and `cargo run -p qpls-cli -- gate v2_8_0_release_handoff_consistency`.
- Release notes and a milestone note document the v2.8 handoff scope.
- The audit verifies screenshot references, artifact naming guidance, stale-version markers, encoding safety, non-equivalence wording, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.7.0 Visual Issue Register

Status: validated.

This frontend-only milestone creates a repeatable rendered-screen issue register for the QuickPLS 2.x shell.

Complete:

- Added rendered smoke coverage for Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings at `1440x900` and `1280x800`.
- Generated `validation/results/v270_visual_issue_register.json` so future UI work starts from concrete evidence.
- Verified desktop views avoid document-level horizontal overflow.
- Verified disabled controls are described, rendered text has no R-squared mojibake, no normal user-facing `Validation fixture` wording appears, and no SmartPLS equivalence claim appears.
- Added static audit evidence for version metadata, active milestone tracking, scripts, source contracts, smoke evidence, issue-register evidence, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.1.2 Setup/Run Mockup Alignment

Status: validated.

This frontend/product-only milestone applies the v2.1 design-system primitives to the Setup and Run workspaces.

Complete:

- Rebuilt Setup around shared `WorkspacePage`, `PageHeader`, `Panel`, `Card`, `StatusBadge`, and guided method surfaces.
- Rebuilt Run around shared `WorkspacePage`, `PageHeader`, `Panel`, `MetricCard`, and `StatusBadge` while preserving the existing `quickpls:run-analysis` launch event.
- Kept method applicability, readiness, exact disabled reasons, output preview, provenance, and Results/Report handoff aligned with the QuickPLS 2.0 visual contract.
- Added `validation/v2112_setup_run_mockup_smoke.mjs`, `validation/v2112_setup_run_mockup_audit.py`, `docs/V2_1_2_SETUP_RUN_MOCKUP_ALIGNMENT.md`, and the registry slice `v2_1_2_setup_run_mockup_alignment`.
- Updated release metadata to `2.1.2` and artifact labeling to `v2_1_2_setup_run_mockup_alignment`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.1.1 Home/Data Mockup Alignment

Status: validated.

This frontend/product-only milestone applies the v2.1 design-system primitives to the Home and Data workspaces.

Complete:

- Rebuilt Home around shared `WorkspacePage`, `PageHeader`, `Panel`, `Card`, `MetricCard`, and `InlineNotice` primitives.
- Rebuilt Data around shared `WorkspacePage`, `PageHeader`, `Panel`, `MetricCard`, and `InlineNotice` primitives while retaining native import and metadata APIs.
- Kept first-viewport workflow actions, data quality, prefix construct creation, and metadata editing aligned with the QuickPLS 2.0 visual contract.
- Added `validation/v2111_home_data_mockup_smoke.mjs`, `validation/v2111_home_data_mockup_audit.py`, `docs/V2_1_1_HOME_DATA_MOCKUP_ALIGNMENT.md`, and the registry slice `v2_1_1_home_data_mockup_alignment`.
- Updated release metadata to `2.1.1` and artifact labeling to `v2_1_1_home_data_mockup_alignment`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.0.7 Run Execution Surface Redesign

Status: validated.

This frontend-only milestone applies the QuickPLS 2.0 visual contract to the Run workspace.

Complete:

- Rebuilt Run as a calculation launch surface with method scope, readiness state, output preview, execution provenance, and completed-run handoff.
- Put disabled-run reasons directly beside the launch action.
- Preserved the existing `quickpls:run-analysis` event and desktop execution boundary.
- Updated release metadata to `2.0.7` and artifact labeling to `v2_0_7_run_execution_surface_redesign`.
- Added `validation/v207_run_surface_smoke.mjs`, `validation/v207_run_surface_audit.py`, `docs/V2_0_7_RUN_EXECUTION_SURFACE_REDESIGN.md`, and the registry slice `v2_0_7_run_execution_surface_redesign`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.0.5 Report Export Flow Redesign

Status: validated.

This frontend-only milestone applies the QuickPLS 2.0 visual contract to the Report workspace.

Complete:

- Added a report package hero showing selected preset, selected run, table count, SVG readiness, and ready export outputs.
- Added a v2 command center for export presets and the four-step report flow.
- Reframed report settings, export review, preview shell, comparison link, and export actions with v2 panel styling.
- Preserved the existing CSV, HTML, desktop XLSX, browser Print/PDF path, and SVG export actions with explicit disabled reasons.
- Added `validation/v205_report_redesign_smoke.mjs`, `validation/v205_report_redesign_audit.py`, `docs/V2_0_5_REPORT_EXPORT_FLOW_REDESIGN.md`, and the registry slice `v2_0_5_report_export_flow_redesign`.
- Release metadata now uses `2.0.5`; versioned desktop artifacts use the label `v2_0_5_report_export_flow_redesign`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

No statistical engines, formulas, result schemas, recipes, project archive format, validation tolerances, or numerical fingerprints changed.

No statistical engines, formulas, result schemas, recipes, project format, import backends, validation tolerances, or numerical fingerprints changed.

## v2.0.0 Design System And Shell

Status: validated.

This frontend-only milestone starts the QuickPLS 2.0 redesign with the shared shell needed to match the approved professional desktop mockup.

Complete:

- Added first-class Trust Center and Settings workspace routes.
- Updated the left rail, command palette, smoke API, and app routing for the expanded v2 shell.
- Added shared v2 design tokens and reusable panel styling for later workspace redesigns.
- Updated version metadata to `2.0.0` and artifact labeling to `v2_0_0_design_system_and_shell`.
- Added `validation/v200_shell_smoke.mjs`, `validation/v200_shell_audit.py`, and `docs/V2_0_0_DESIGN_SYSTEM_AND_SHELL.md`.

No statistical engines, formulas, result schemas, recipes, project format, import backends, validation tolerances, or numerical fingerprints changed.

## v2.0.1 Home And Data Redesign

Status: validated.

This frontend-only milestone applies the QuickPLS 2.0 desktop mockup direction to the Home and Data workspaces.

Complete:

- Converted Home into a project command center with a current-workspace hero, recommended next step, primary command grid, workflow status, sample gallery, and guided dataset workflow.
- Reworked Data into a workbench with import source and data-quality panels at the top, method applicability guidance, prefix construct creation, and a dominant preview/metadata editor.
- Kept existing native import, sample dataset loading, metadata update, and browser CSV preview behavior.
- Updated version metadata to `2.0.1` and artifact labeling to `v2_0_1_home_data_redesign`.
- Added `validation/v201_home_data_smoke.mjs`, `validation/v201_home_data_audit.py`, and `docs/V2_0_1_HOME_DATA_REDESIGN.md`.

No statistical engines, formulas, result schemas, recipes, project format, import backends, validation tolerances, or numerical fingerprints changed.

## v1.8.1 Method Applicability And Guided Setup

Status: validated.

This frontend-only milestone adds SmartPLS-style method guidance so researchers see which analyses are recommended, available after setup, not applicable, unsupported, or experimental for the current dataset/model/settings.

Complete:

- Added a deterministic frontend method applicability engine with method-specific checks and next-action labels.
- Reworked Setup into recommendation sections plus Show all methods for unavailable/unsupported choices.
- Made bootstrap an inference add-on instead of a primary top-bar analysis choice.
- Added Data and Model guidance panels explaining what can be done with the current data/model.
- Added v1.8.1 smoke/audit scripts, documentation, tests, and the `v1_8_1_method_applicability_guided_setup` registry gate.

No statistical engines, formulas, result schemas, recipes, project format, import backends, or numerical fingerprints changed.

## v1.5.9 Report Publication Workflow Redesign

Status: validated.

This frontend-only milestone addresses the Report workspace issues from the v1.5.7 launch audit.

Complete:

- Replaced passive report preset buttons with selectable preset cards.
- Grouped publication setup into Figure, Statistics, Tables, and Notes sections.
- Added explicit CSV, HTML, XLSX, Print/PDF, and SVG export actions with action-specific disabled reasons.
- Added publication preview layout-risk guidance and a visible current-canvas versus tidy-publication status.
- Moved run comparison out of Report and into the Results comparison workspace handoff.
- Improved SmartPLS-like publication SVG label readability with construct label backgrounds and structural label offsets.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

Release metadata now uses `1.5.8`; versioned desktop artifacts use the label `v1_5_8_results_workspace_launch_redesign`.

Release metadata now uses `1.5.3`; versioned desktop artifacts use the label `v1_5_3_layout_copy_readiness_polish`.

## v1.5.4 Results Workspace Hardening

Status: validated.

This frontend-only milestone turns the Results page into a workflow-specific review surface. Summary, Measurement Model, Structural Model, Reliability and Validity, Inference, Prediction, Groups, Diagnostics, and Comparison now render scoped sections instead of repeating broad run output.

Complete:

- Added tab-specific result tables, interpretation notes, empty states, and selected-tab export behavior.
- Added diagram-to-result linking evidence: selecting a SEM path highlights related Summary, Measurement, and Structural rows.
- Kept result action controls visible while scrolling.
- Replaced generic Groups/Prediction dumps with method-specific empty states when those payloads are not present.
- Added `validation/v154_results_workspace_smoke.mjs`, `validation/v154_results_workspace_audit.py`, `validation/v154_results_native_smoke.py`, and the registry slice `v1_5_4_results_workspace_hardening`.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

Release metadata now uses `1.5.4`; versioned desktop artifacts use the label `v1_5_4_results_workspace_hardening`.

## v1.5.5 Results Interpretation Polish

Status: validated.

This frontend-only milestone adds researcher-facing interpretation to the Results workspace without changing any numerical backend behavior.

Complete:

- Added expandable interpretation panels with threshold guidance, methodological rationale, and report wording.
- Added result precision, interpretation-column visibility, table-level copy, row-detail selection, and current-tab export support.
- Added an Interpretation tab for next-step guidance, copyable report wording, and availability mapping.
- Replaced the placeholder Comparison tab with bounded two-run comparison for compatible PLS-family runs.
- Added `validation/v155_results_interpretation_smoke.mjs`, `validation/v155_results_interpretation_audit.py`, `docs/V1_5_5_RESULTS_INTERPRETATION_POLISH.md`, and the registry slice `v1_5_5_results_interpretation_polish`.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

Release metadata now uses `1.5.5`; versioned desktop artifacts use the label `v1_5_5_results_interpretation_polish`.

## v1.5.6 Result-Specific Interpretation Engine

Status: complete.

- Added a deterministic frontend interpretation engine in `src/domain/resultInterpretation.ts` that converts existing run values into exact findings, recommended actions, linked result tabs, row-level explanations, SEM diagram-advisor checks, and report-ready wording.
- Updated Results to show finding cards, prioritized interpretation checklists, exact-value row details, and copy controls while preserving clean numeric result tables.
- Updated Report with an explicit `Include interpretation notes` option for HTML/print workflows; default numeric CSV/XLSX exports remain unchanged.
- Added `src/domain/resultInterpretation.test.ts`, `validation/v156_result_interpretation_smoke.mjs`, `validation/v156_result_interpretation_audit.py`, `docs/V1_5_6_RESULT_SPECIFIC_INTERPRETATION_ENGINE.md`, and the registry slice `v1_5_6_result_specific_interpretation_engine`.

Release metadata now uses `1.5.6`; versioned desktop artifacts use the label `v1_5_6_result_specific_interpretation_engine`.

## v1.5.7 UI/UX Launch-Quality Audit

Status: complete.

- Preserved the 15 user-supplied full-screen screenshots under `validation/results/screens/v157/ui-ux-launch-quality/`.
- Added a 60-item launch-quality issue register covering layout, density, hierarchy, color, copy, results tables, report flow, SEM diagram presentation, accessibility, and workflow sequencing.
- Added a dependency-ordered remediation sequence for v1.5.8 through v1.6.3 so implementation can proceed from highest-impact launch blockers first.

## v1.6.0 Model Canvas Shell And Panel Polish

Status: validated.

- Added a collapsible right inspector and View-menu controls for collapsing the left explorer, collapsing the right inspector, showing the minimap, isolating selected neighborhoods, and collapsing measurement indicators.
- Made the minimap opt-in and moved the result overlay into compact canvas chrome so the diagram itself gets more working space.
- Grouped secondary selected-object actions in the canvas context toolbar and simplified SEM explorer construct cards.
- Added smoke and static audit evidence under `validation/results/v160_model_canvas_smoke.json` and `validation/results/v160_model_canvas_audit.json`.
- Added `validation/v157_ui_ux_launch_quality_smoke.py`, `validation/v157_ui_ux_launch_quality_audit.py`, `docs/V1_5_7_UI_UX_LAUNCH_QUALITY_AUDIT.md`, and the registry slice `v1_5_7_ui_ux_launch_quality_audit`.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

Release metadata was advanced to `1.6.0`; fresh versioned desktop artifacts use the label `v1_6_0_model_canvas_shell_and_panel_polish`.

## v1.6.1 Setup/Run Workflow Consolidation

Status: validated.

This frontend-only milestone consolidates the analysis setup and execution flow.

Complete:

- Setup now remains the primary configuration surface and can launch the configured run directly through the production run event.
- Setup includes a consolidated ready-to-run summary with method, scope, resampling, seed, worker count, and launch action.
- The duplicate readiness and run-state cards were removed from Setup.
- Run now behaves as a compact execution monitor and result handoff page instead of repeating the full Setup readiness grid.
- Run links settings changes back to Setup.
- Added `validation/v161_setup_run_smoke.mjs`, `validation/v161_setup_run_audit.py`, `docs/V1_6_1_SETUP_RUN_WORKFLOW_CONSOLIDATION.md`, and the registry slice `v1_6_1_setup_run_workflow_consolidation`.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

## v1.6.2 Data/Home Launch Polish

Status: validated.

This frontend-only milestone improves the project start and data-to-model flow.

Complete:

- Home now computes a recommended next step from current data/model/run state.
- Home uses a compact workflow status list instead of sparse duplicate workflow cards.
- Save, open, demo, recent project, import, and model-start actions remain visible.
- Data keeps `Open Model Designer`, prefix detection, and `Create Constructs From Prefixes` as the main data-to-model bridge.
- Added `validation/v162_data_home_smoke.mjs`, `validation/v162_data_home_audit.py`, `docs/V1_6_2_DATA_HOME_LAUNCH_POLISH.md`, and the registry slice `v1_6_2_data_home_launch_polish`.

No statistical engines, formulas, result schemas, recipes, project format, import backends, or numerical fingerprints changed.

## v1.8 Results And Report Refinement From Real User Testing

Status: validated.

This frontend-only release focuses on Results and Report usability from real-like dataset review.

Complete:

- Added grouped Results menus for View, Table, Export, and Interpretation controls.
- Added sticky run-level context and a `Why trust this result?` drawer to reduce repeated confidence text.
- Split bootstrap output into estimates, percentile CI, BCa CI, and bootstrap-t CI sections.
- Reworked HTMT to default to unique construct-pair rows, with full matrix view available on demand.
- Deduplicated interpretation findings, especially symmetric HTMT findings.
- Reworked Report into a four-step export flow with export review and destination/status feedback.
- Added v1.8 smoke/audit scripts and the `v1_8_results_report_refinement_real_user_testing` registry gate.

No statistical engines, formulas, result schemas, recipes, project format, import backends, validation tolerances, or numerical fingerprints changed.

## v1.6.3 Global Design-System And Accessibility Pass

Status: validated.

This frontend-only milestone closes the final remediation item from the v1.5.7 launch-quality audit.

Complete:

- Updated the visible top-bar milestone label so the app no longer shows stale v1.5.3 wording.
- Added source-level smoke and audit evidence for release-label consistency, mojibake prevention, scoped method-status language, and accessible disabled Run reasons.
- Preserved existing keyboard-focus, table-region, SEM overlay, shortcut, and readiness-checklist accessibility contracts.
- Added `validation/v163_design_accessibility_smoke.mjs`, `validation/v163_design_accessibility_audit.py`, `docs/V1_6_3_GLOBAL_DESIGN_ACCESSIBILITY_PASS.md`, and the registry slice `v1_6_3_global_design_system_and_accessibility_pass`.

## v1.7 SmartPLS-Competitive Researcher Experience

- Added researcher trust surfaces: `Why trust this result?`, method scope drawer, and Method Confidence panels.
- Strengthened the workflow path with Setup calculation previews, Results-to-Report handoff, Focus Diagram mode, and sample-guided workflows.
- Added reportability checklist logic using existing result values for indicator reliability, reliability/validity, collinearity, structural paths, R², f², prediction, inference, and warnings.
- Added Reviewer Pack export preset and explicit interpretation-note opt-in behavior.
- Added v1.7 static audits and registry gates for v1.7.0 through v1.7.6 plus the final program gate.
- Release metadata now uses `1.7.6`; versioned desktop artifacts use the label `v1_7_smartpls_competitive_researcher_experience`.

No statistical engines, formulas, result schemas, recipes, project format, import backends, or numerical fingerprints changed.

## v1.5.8 Results Workspace Launch Redesign

Status: validated.

This frontend-only milestone addresses the highest-priority Results workspace issues from the v1.5.7 launch audit.

Complete:

- Replaced the crowded Results action strip with a dedicated workbench shell, section navigation tiles, and grouped table tools.
- Added triaged finding cards so the highest-priority issues appear first without flooding the page.
- Removed duplicate HTMT symmetric-pair findings.
- Split mediation effects into summary, inference, and classification tables.
- Added row-count metadata and wide-table scroll guidance.
- Cleaned remaining Results `R²`, `f²`, and `Q²` mojibake.

No statistical engines, formulas, result schemas, recipes, project format, or numerical fingerprints changed.

## v2.0.6 Model Shell And SEM Designer Surround

Status: validated.

This frontend-only milestone applies the QuickPLS 2.0 visual contract to the Model workspace shell around the existing SEM Designer.

Complete:

- Added v2 shell hooks and styling for the SEM Explorer, Model canvas, canvas toolbar, overlay/status surfaces, and Inspector.
- Preserved the existing SEM Designer grammar and behavior: latent ovals, indicator rectangles, measurement links, structural paths, result overlays, and SVG/export parity.
- Fixed remaining Model/Inspector `R²` encoding issues.
- Added `validation/v206_model_shell_smoke.mjs`, `validation/v206_model_shell_audit.py`, `docs/V2_0_6_MODEL_SHELL_SEM_DESIGNER_SURROUND.md`, and the registry slice `v2_0_6_model_shell_sem_designer_surround`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.0.9 Mockup Fidelity System

Status: validated.

This frontend/product-only milestone makes the approved QuickPLS 2.0 desktop mockup enforceable before additional screen rebuilds continue.

Complete:

- Expanded the QuickPLS 2.0 visual contract with source-of-truth, viewport, mockup-matching, screen completion, and versioned artifact rules.
- Added `validation/v209_mockup_fidelity_smoke.mjs` to verify v2 primitives, workspace source coverage, current milestone text, encoding safety, and claim boundaries.
- Added `validation/v209_mockup_fidelity_audit.py` to verify version metadata, registry state, roadmap expectations, docs, and artifact script conventions.
- Added `docs/V2_0_9_MOCKUP_FIDELITY_SYSTEM.md` and the registry slice `v2_0_9_mockup_fidelity_system`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.1.0 Design System Foundation

Status: validated.

This frontend/product-only milestone starts the deeper QuickPLS 2.x rebuild by turning the approved desktop mockup into reusable v2.1 design-system primitives.

Complete:

- Added shared primitives for workspace pages, workspace headers, panels, metric cards, command groups, toolbar buttons, inline notices, and cards.
- Added a visible Settings design-system preview so the primitive language can be inspected in-app.
- Kept the QuickPLS 2.0 visual contract as the source of truth for future workspace rebuilds.
- Added `validation/v2100_design_system_smoke.mjs`, `validation/v2100_design_system_audit.py`, `docs/V2_1_0_DESIGN_SYSTEM_FOUNDATION.md`, and the registry slice `v2_1_0_design_system_foundation`.
- Updated release metadata to `2.1.0` and artifact labeling to `v2_1_0_design_system_foundation`.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.5.1 Workflow Navigation Parity

Status: validated.

This frontend-only milestone aligns the top workflow strip with the left navigation hierarchy introduced in v2.5.0.

Complete:

- The left rail remains the full navigation surface: Research workflow plus Support utilities.
- The top workflow strip is now explicitly scoped to the primary calculation workflow only.
- Added a visible `Workflow` label and stable `data-workflow-scope` / `data-workflow-count` hooks.
- Added rendered smoke evidence for workflow order, support-route exclusion, Settings navigation, and shell integrity.
- Added static audit evidence for version metadata, registry state, scripts, roadmap expectations, no mojibake, no SmartPLS-equivalence claim, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.5.2 Launcher And Support Shell Separation

Status: validated.

This frontend-only milestone separates launcher/support destinations from the primary calculation workflow band.

Complete:

- Home, Trust Center, and Settings now render as support shells without the primary workflow strip or coach.
- Data, Setup, Run, Results, and Report keep the workflow strip and coach.
- Model keeps its dedicated workflow band around the SEM Designer.
- Added rendered smoke evidence for Home, Data, Trust, Settings, and Model shell behavior.
- Added static audit evidence for version metadata, registry state, scripts, roadmap expectations, source contracts, no mojibake, no SmartPLS-equivalence claim, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.5.3 Support Utility Shell Polish

Status: validated.

This frontend-only milestone makes Home, Trust Center, and Settings feel intentionally grouped as support utilities after they were removed from the primary calculation workflow.

Complete:

- Added a local support utility bar on Home, Trust Center, and Settings with direct switching between launcher, evidence/scope, and local preferences.
- Kept Data, Setup, Run, Results, and Report focused on the primary research workflow without support utility controls.
- Kept Model on its dedicated SEM Designer workflow surface without support utility controls.
- Added rendered smoke evidence for support utility switching, workflow route separation, and shared shell integrity.
- Added static audit evidence for version metadata, registry state, scripts, roadmap expectations, source contracts, no mojibake, no SmartPLS-equivalence claim, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.5.4 Visual Contract Support-Shell Alignment

Status: validated.

This frontend/product milestone aligns the QuickPLS 2.0 visual contract with the support-shell information architecture.

Complete:

- Updated the v2 visual contract so the calculation workflow strip is Data, Model, Setup, Run, Results, and Report.
- Documented Home, Trust Center, and Settings as support utilities with local support navigation instead of workflow progress controls.
- Documented the Model workspace exception: it may keep a dedicated SEM Designer workflow band while calculation pages use the shared workflow treatment.
- Fixed the remaining R-squared encoding artifact in the visual contract.
- Added static audit evidence for version metadata, registry state, scripts, source contracts, no mojibake, no SmartPLS-equivalence claim, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.5.5 Support Shell Viewport Alignment

Status: validated.

This frontend-only milestone aligns the support utility bar with Home, Trust Center, and Settings workspace content and verifies desktop viewport fit.

Complete:

- Wrapped the support utility bar in a workspace-aligned frame using the same max width and gutters as support page content.
- Preserved Home, Trust Center, and Settings as support utilities without workflow progress controls.
- Preserved calculation workflow pages without support utility controls.
- Added rendered smoke evidence at `1440x900` and `1280x800` for support-shell alignment and horizontal overflow.
- Added static audit evidence for version metadata, registry state, scripts, frame CSS, source contracts, no mojibake, no SmartPLS-equivalence claim, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.6.0 Launch-Quality Visual Consolidation

Status: validated.

This frontend-only milestone adds a grouped launch-quality visual consolidation gate across the QuickPLS 2.x shell.

Complete:

- Added rendered smoke coverage for support utilities and workflow screens at `1440x900` and `1280x800`.
- Verified Home, Trust Center, and Settings keep the support utility shell.
- Verified Data, Model, Setup, Run, Results, and Report keep the primary workflow shell without support utility controls.
- Verified desktop views avoid document-level horizontal overflow.
- Verified disabled buttons have descriptions, rendered text has no R-squared mojibake, and no SmartPLS equivalence claim appears.
- Added static audit evidence for active milestone tracking, scripts, source contracts, smoke evidence, and frontend-only boundaries.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.12.0 Real Dataset Review Protocol

Status: validated.

This frontend/product milestone adds a privacy-safe protocol for reviewing real researcher datasets without committing private data or value-revealing screenshots.

Complete:

- Added a manual checklist for Data, Setup, Results, and Report review with private datasets.
- Added an anonymized issue-register template that separates product issues, method guidance gaps, export gaps, and statistical evidence gaps.
- Documented the no-private-data persistence rule for raw datasets, private `.qpls` files, screenshots, and exported reports.
- Added targeted smoke and audit evidence for the protocol, template, registry, scripts, and version metadata.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.

## v2.13.0 Real Dataset Protocol Entrypoints

Status: validated.

This frontend-only milestone makes the v2.12 real dataset review protocol discoverable inside QuickPLS.

Complete:

- Added a Trust Center section for the real dataset review protocol and template.
- Added a Settings section summarizing private-dataset review rules.
- Added a Home notice for private dataset review workflows.
- Added targeted smoke and static audit evidence for the entrypoints.

No statistical engines, formulas, result schemas, recipes, project format, validation tolerances, or numerical fingerprints changed.
## QuickPLS 2.14.0

- `v2_14_0_real_dataset_feedback_triage` is validated.
- The milestone adds anonymized real-dataset feedback triage so private researcher observations can be converted into grouped frontend milestones without committing raw private data, private projects, or value-revealing screenshots.
- Evidence:
  - `docs/V2_14_0_REAL_DATASET_FEEDBACK_TRIAGE.md`
  - `validation/templates/real_dataset_feedback_triage_template.json`
  - `validation/results/v2140_real_dataset_triage_smoke.json`
  - `validation/results/v2140_real_dataset_triage_audit.json`
  - `validation/results/v2140_real_dataset_triage_backlog.json`
- Latest artifacts:
  - `target/release/artifacts/QuickPLS_2.14.0_v2_14_0_real_dataset_feedback_triage_20260730-051333_x64_setup.exe`
  - `target/release/artifacts/QuickPLS_2.14.0_v2_14_0_real_dataset_feedback_triage_20260730-051333_x64_portable.exe`
  - `target/release/artifacts/QuickPLS_2.14.0_v2_14_0_real_dataset_feedback_triage_20260730-051333_x64_checksums.txt`

## QuickPLS 2.15.0

- `v2_15_0_workflow_method_guidance_triage_pass` is validated.
- The milestone strengthens workflow and method guidance across Data, Model, Setup, and the top command bar.
- Evidence:
  - `docs/V2_15_0_WORKFLOW_METHOD_GUIDANCE_TRIAGE_PASS.md`
  - `validation/results/v2150_workflow_method_guidance_smoke.json`
  - `validation/results/v2150_workflow_method_guidance_audit.json`
- Latest artifacts:
  - `target/release/artifacts/QuickPLS_2.15.0_v2_15_0_workflow_method_guidance_triage_pass_20260730-054628_x64_setup.exe`
  - `target/release/artifacts/QuickPLS_2.15.0_v2_15_0_workflow_method_guidance_triage_pass_20260730-054628_x64_portable.exe`
  - `target/release/artifacts/QuickPLS_2.15.0_v2_15_0_workflow_method_guidance_triage_pass_20260730-054628_x64_checksums.txt`

## QuickPLS 2.26.0

- `v2_26_0_method_setup_applicability_center` is validated.
- The milestone turns Setup into a native desktop calculation setup center with recommended, available, available-with-setup, diagnostic, standalone, and not-applicable method lanes.
- Bootstrap is presented as an inference add-on rather than a confusing primary algorithm.
- Selected methods show exact requirement checks, blocker actions, expected outputs, and scope evidence.
- Evidence:
  - `docs/V2_26_0_METHOD_SETUP_APPLICABILITY_CENTER.md`
  - `validation/results/v2260_method_setup_smoke.json`
  - `validation/results/v2260_method_setup_audit.json`
- Boundary: frontend/product method setup presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.27.0

- `v2_27_0_calculation_run_monitor` is validated.
- The milestone rebuilds Run as a native desktop calculation monitor with a procedure checklist, central progress/log panel, immutable settings summary, output availability list, cancellation handoff, and completed-run handoff.
- Evidence:
  - `docs/V2_27_0_CALCULATION_RUN_MONITOR.md`
  - `validation/results/v2270_run_monitor_smoke.json`
  - `validation/results/v2270_run_monitor_audit.json`
- Boundary: frontend/product run monitoring only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.28.0

- `v2_28_0_results_workbook_redesign` is validated.
- The milestone rebuilds Results into a native desktop statistical workbook with a sticky selected-run header, central result table area, right interpretation/method-confidence pane, findings lanes, and provenance footer.
- Evidence:
  - `docs/V2_28_0_RESULTS_WORKBOOK_REDESIGN.md`
  - `validation/results/v2280_results_workbook_smoke.json`
  - `validation/results/v2280_results_workbook_audit.json`
- Boundary: frontend/product results presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.29.0

- `v2_29_0_research_table_system` is validated.
- The milestone upgrades Results tables into one shared research-table shell with sticky headers, sticky first data column, table search, sorting, precision, density, selected-row copy, table export, and row-detail interpretation.
- PLSpredict and CVPAT tables now use the same shell as the measurement, structural, validity, inference, mediation, diagnostics, interpretation, and comparison outputs.
- Evidence:
  - `docs/V2_29_0_RESEARCH_TABLE_SYSTEM.md`
  - `validation/results/v2290_research_tables_smoke.json`
  - `validation/results/v2290_research_tables_audit.json`
- Boundary: frontend/product table presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.30.0

- `v2_30_0_interpretation_reportability_assistant` is validated.
- The milestone adds a Results reportability assistant with canonical checklist lanes, value-specific explanations, next inspection targets, and copyable report snippets from the selected run.
- Threshold colors remain methodological guidance, not universal pass/fail rules.
- Evidence:
  - `docs/V2_30_0_INTERPRETATION_REPORTABILITY_ASSISTANT.md`
  - `validation/results/v2300_reportability_assistant_smoke.json`
  - `validation/results/v2300_reportability_assistant_audit.json`
- Boundary: frontend/product interpretation presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.31.0

- `v2_31_0_report_export_wizard` is validated.
- The milestone turns Report into a four-step desktop export wizard: Select content, Preview, Document settings, and Export.
- Detailed run comparison remains in Results; Report links to the Results Comparison workspace instead of duplicating comparison tables.
- Evidence:
  - `docs/V2_31_0_REPORT_EXPORT_WIZARD.md`
  - `validation/results/v2310_report_export_wizard_smoke.json`
  - `validation/results/v2310_report_export_wizard_audit.json`
- Boundary: frontend/product report presentation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.36.0

- `v2_36_0_native_desktop_ui_spec_and_component_plan` is validated.
- The milestone freezes the native desktop UI blueprint from the final QuickPLS 2.0 screen and dialog mockups, including the workbench shell, workflow screens, task dialogs, focus diagram mode, reusable component inventory, and implementation map.
- Evidence:
  - `docs/V2_36_0_NATIVE_DESKTOP_UI_SPEC_AND_COMPONENT_PLAN.md`
  - `validation/results/v236_native_ui_spec_audit.json`
- Boundary: planning/frontend specification only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.37.0

- `v2_37_0_native_frontend_prototype_shell` is validated.
- The milestone adds an isolated QuickPLS 2 native desktop frontend prototype behind `?native_prototype=1`, using dummy data to match the new workbench direction before backend wiring.
- The prototype includes the full desktop shell, menu bar, command strip, workflow rail, Home, Data, Model, Setup, Run, Results, Report, Trust Center, Settings, and task dialogs.
- Evidence:
  - `docs/V2_37_0_NATIVE_FRONTEND_PROTOTYPE_SHELL.md`
  - `validation/results/v2370_native_frontend_prototype_smoke.json`
  - `validation/results/v2370_native_frontend_prototype_audit.json`
- Boundary: frontend prototype only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.38.0

- `v2_38_0_native_frontend_backend_adapters` is validated.
- The milestone connects the isolated native prototype to existing frontend workspace state through read-only adapters for project summary, dataset preview, variables, constructs, structural paths, methods, completed-run rows, and trust evidence.
- Static prototype data remains available only as fallback when workspace content is absent.
- Evidence:
  - `docs/V2_38_0_NATIVE_FRONTEND_BACKEND_ADAPTERS.md`
  - `validation/results/v2380_native_frontend_backend_adapters_smoke.json`
  - `validation/results/v2380_native_frontend_backend_adapters_audit.json`
- Boundary: frontend/product adapter only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.39.0

- `v2_39_0_native_frontend_screen_replacement_plan` is validated.
- The milestone adds an opt-in production-candidate native shell route behind `?native_shell=1`, keeps the isolated prototype behind `?native_prototype=1`, and preserves the default legacy app route.
- The candidate shell synchronizes native rail navigation with the existing workspace ids so screen replacement can proceed safely route by route.
- Evidence:
  - `docs/V2_39_0_NATIVE_FRONTEND_SCREEN_REPLACEMENT_PLAN.md`
  - `validation/results/v2390_native_frontend_screen_replacement_smoke.json`
  - `validation/results/v2390_native_frontend_screen_replacement_audit.json`
- Boundary: frontend/product shell-routing bridge only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## QuickPLS 2.40.0

- `v2_40_0_mockup_fidelity_native_shell_alignment` is validated.
- The milestone tightens the production-candidate native shell against the supplied QuickPLS 2.0 mockups with a mockup-style ribbon, hierarchical SEM Explorer, tabbed Object Inspector, and bottom output pane.
- Older UI surfaces that are not present in the mockups are documented separately for later product decisions instead of being mixed into the current parity target.
- Evidence:
  - `docs/V2_40_0_MOCKUP_FIDELITY_NATIVE_SHELL_ALIGNMENT.md`
  - `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md`
  - `validation/results/v2400_mockup_fidelity_smoke.json`
  - `validation/results/v2400_mockup_fidelity_audit.json`

## v2.41.0 Full Mockup Screen Parity Pass

- `v2_41_0_full_mockup_screen_parity_pass` is validated.
- Added a strict manifest that maps every supplied QuickPLS 2.0 mockup PNG to a rendered screen or dialog state.
- Added the `?native_shell=1&mockup_parity=1` route so parity review uses deterministic mockup data while the normal native shell remains backend-wired.
- Tightened the native shell, ribbon, Model workbench panes, object inspector, diagram geometry, and bottom pane structure toward the supplied desktop mockup proportions.
- Added targeted evidence:
  - `validation/results/v2410_mockup_manifest_audit.json`
  - `validation/results/v2410_mockup_visual_parity_smoke.json`
  - `validation/results/v2410_mockup_visual_parity_audit.json`
- Boundary: frontend/product mockup-fidelity alignment only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.42.0 Make Native Mockup Shell Default

- `v2_42_0_make_native_mockup_shell_default` is validated.
- The production-candidate native mockup-parity shell is now the default app UI.
- `?native_shell=1` remains accepted for compatibility, `?native_prototype=1` remains the isolated static prototype, and `?legacy_shell=1` is the explicit fallback to the older shell during transition testing.
- Added targeted evidence:
  - `docs/V2_42_0_MAKE_NATIVE_MOCKUP_SHELL_DEFAULT.md`
  - `validation/results/v2420_native_default_shell_smoke.json`
  - `validation/results/v2420_native_default_shell_audit.json`
- Boundary: frontend/product routing only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.42.1 Native Shell QA Test Pack

- `v2_42_1_native_shell_qa_test_pack` is validated.
- Added screen-by-screen QA coverage for Home, Data, Model, Setup, Run, Results, Report, Trust Center, Settings, Import Data, and Calculation Setup.
- Added interaction wiring smoke for desktop menus, rail navigation, dialog close paths, Escape handling, SEM designer integration, and backend-adapter Run/Results/Report surfaces.
- Added old-shell trace audit to keep the default route free of stale v1/v1.5 wording, old dashboard chrome, mojibake, duplicated global controls, and unsupported equivalence claims.
- Version metadata is set to `2.42.1`; the release artifact label is `v2_42_1_native_shell_qa_test_pack`.

## v2.43.0 Full Native Frontend/Backend Wiring

- `v2_43_0_full_native_frontend_backend_wiring` is validated.
- Default-shell commands now route to real production behavior, real task dialogs, or intentional absence.
- Close Project, structured Data command dialogs, layout/status bar preferences, offline documentation, and checksum detail verification are wired.
- Pause is intentionally absent because the current runner supports cancellation, not safe suspension/resumption.
- Version metadata is set to `2.43.0`; the release artifact label is `v2_43_0_full_native_frontend_backend_wiring`.
- Evidence:
  - `docs/V2_42_1_NATIVE_SHELL_QA_TEST_PACK.md`
  - `validation/results/v2421_native_screen_qa_smoke.json`
  - `validation/results/v2421_native_interaction_wiring_smoke.json`
  - `validation/results/v2421_native_web_trace_audit.json`
  - `validation/results/v2421_native_qa_test_pack_audit.json`
- Boundary: frontend/product QA, dialog keyboard behavior, release metadata, docs, and validation only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.

## v2.44.0 Native UI Production Binding Completion

- `v2_44_0_native_ui_production_binding_completion` is validated.
- Default native-shell Home/Data/Model/Setup/Run/Results/Report surfaces now bind to real workspace/project/run/report state where available.
- Explicit mockup-parity fallback data remains available only for visual review mode, not normal production use.
- Fake telemetry and stale static run/project strings are removed from the default native shell.
- Version metadata is set to `2.44.0`; the release artifact label is `v2_44_0_native_ui_production_binding_completion`.
- Evidence:
  - `docs/V2_44_0_NATIVE_UI_PRODUCTION_BINDING_COMPLETION.md`
  - `validation/results/v244_home_project_status_smoke.json`
  - `validation/results/v244_data_binding_smoke.json`
  - `validation/results/v244_model_binding_smoke.json`
  - `validation/results/v244_setup_binding_smoke.json`
  - `validation/results/v244_run_results_report_binding_smoke.json`
  - `validation/results/v244_trust_settings_commands_smoke.json`
  - `validation/results/v244_production_binding_audit.json`
- Boundary: frontend/native-shell production binding only; no statistical engine, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.
# v2.45.0 Mockup Visible Feature Completion

- Status: validated.
- Scope: native-shell frontend/product binding only.
- Evidence: `validation/results/v245_mockup_feature_completion_smoke.json` and `validation/results/v245_mockup_feature_completion_audit.json`.
- Notes: Data tabs, Model explorer/bottom panes, Setup evidence drawer, Run output preview, and Trust release integrity now use live/default-shell state instead of fixed mockup placeholders. No numerical backend behavior changed.

## QuickPLS 3.0 parity program — Wave 1 logistic checkpoint

- `qpls3.standalone.logistic` is the first parity-ledger capability to reach `release_qualified` under the catalogue snapshot dated 2026-08-12.
- Current execution emits `regression_logistic_v2` from a typed recipe-v3 standalone Regression contract; `regression_logistic_v1` remains readable historical output and cannot be appended as current evidence.
- Accepted scope is an exactly coded numeric 0/1 outcome with both classes in the listwise-complete sample, deterministic single-worker Newton IRLS, fixed 95% Wald inference, odds-ratio intervals, likelihood diagnostics, fitted probabilities, convergence evidence, and explicitly in-sample descriptive classification.
- Independent Python and validation-only R `glm` arithmetic, strict archive-v5 append/save/reopen/tamper gates, browser setup at 1024×700, 1280×720, and 1440×900, real packaged Tauri execution, native XLSX, strict-profile failure recovery, explicit save, and same-run reopen pass.
- Evidence: `validation/results/logistic_v2_reference_report.json`, `validation/results/logistic_method_promotion_audit.json`, and `validation/results/logistic_v2_packaged_acceptance.json`.
- This checkpoint did not change the package/release version. At the time of this logistic checkpoint, Wave 1 remained open for regression bootstrapping and the full graph-defined PROCESS workflow; no new installer was claimed.

## QuickPLS 3.0 parity program - Wave 1 regression-bootstrap checkpoint

- `qpls3.standalone.regression_bootstrap` reached `release_qualified` evidence status for the bounded `regression_bootstrap_v1` OLS and binary-logistic coefficient-inference scope.
- Independent Python/R reference checks, exact scientific and archive-tamper boundaries, three-viewport browser acceptance, genuine packaged 10,000-resample OLS and logistic execution, cancellation, native XLSX export, explicit save, and same-run reopen pass.
- Evidence: `validation/results/regression_bootstrap_v1_reference_report.json`, `validation/results/regression_bootstrap_v1_boundary_test_report.json`, `validation/results/regression_bootstrap_v1_packaged_acceptance.json`, and `validation/results/regression_bootstrap_method_promotion_audit.json`.
- This checkpoint did not change the package/release version or publish an installer. PROCESS v2 and the coordinated Wave 1 2.46.0 release transition remained open at this point.

## QuickPLS 3.0 parity program - Wave 1 PROCESS capability checkpoint

- `qpls3.standalone.process` is currently `native_qualified` for its documented bounded `regression_process_v2` scope.
- Independent Python/R reference checks, exact scientific and archive-tamper boundaries, the focused frontend/type gate, three-viewport browser acceptance, genuine packaged 10,000-resample execution, cancellation/retry, accessible result and plot-data tables, native XLSX export, explicit save/reopen, and clean shutdown pass. Release qualification is withheld because the current repeated-completion resource report did not prove terminal process-role stability.
- Evidence: `validation/results/process_v2_reference_report.json`, `validation/results/process_v2_boundary_test_report.json`, `validation/results/process_v2_frontend_gate_report.json`, `validation/results/v247_native_desktop_visual_acceptance.json`, `validation/results/process_v2_packaged_acceptance.json`, and `validation/results/process_v2_method_promotion_audit.json`.
- This checkpoint does not change package version or publish an installer. The repository remains on the 2.45.0 recovery/candidate version; Wave 1 2.46.0 remains open for coordinated version metadata, public status-label transition, rebuilt artifacts, and release gates.

## QuickPLS 2.46.0 Wave 1 capability qualification

- The QuickPLS 3 parity ledger records 17 in-scope capabilities: 14 are `native_qualified`, while Structural Path Randomization v1, Binary Logistic Regression v2, and Regression Bootstrapping v1 are `release_qualified` through current method-specific and packaged evidence. Graph-defined PROCESS v2 remains native-qualified pending a passing repeated-completion process-role stability gate.
- `qpls3.inference.structural_path_randomization` reached `release_qualified` evidence status for bounded `freedman_lane_permutation_v1`: single-model direct structural-score paths using fixed original converged PLS scores, intercept nuisance equations, deterministic path-specific streams, two-sided unadjusted plus-one probabilities, and exchangeable reduced-model residuals.
- Independent Python/R recomputation, exact deterministic-index and worker-invariance boundaries, calibrated paired homoscedastic Gaussian null/power scenarios, strict archive/tamper validation, focused frontend/type checks, three-viewport visual acceptance, genuine packaged cancellation/retry/completion, native XLSX export, explicit save/reopen, and clean process/resource evidence pass.
- Evidence: `validation/results/structural_path_randomization_reference_report.json`, `validation/results/structural_path_randomization_boundary_test_report.json`, `validation/results/structural_path_randomization_frontend_gate_report.json`, `validation/results/v247_native_desktop_visual_acceptance.json`, `validation/results/structural_path_randomization_v1_packaged_acceptance.json`, and `validation/results/structural_path_randomization_method_promotion_audit.json`.
- This qualification does not cover measurement-model re-estimation, multiplicity adjustment, heteroskedastic or broader non-Gaussian validity, MGA, MICOM, causal proof, or numerical identity with another product. The explicit conditional/approximate interpretation warning remains required.
- The coordinated QuickPLS 2.46.0 Wave 1 release records these capability qualifications. Installer construction, checksum publication, and signing remain governed by their separate release-artifact gates and are not implied by this evidence checkpoint.

## QuickPLS 3 competitor-program segmentation correction

The historical v1.2.2 promotion record above is retained as history, but it is not current QuickPLS 3 qualification. Product, result-table, export, interpretation, compatibility, and recipe-provenance surfaces now identify `pls_pos_v1` and `fimix_pls_v1` as bounded diagnostic previews. In particular, `fimix_pls_v1` does not implement a full finite-mixture EM likelihood and its inverse-distance membership scores are not posterior probabilities. Both methods remain absent in the identity-bound QuickPLS 3 promotion factory until their new evidence ladders pass; a future full FIMIX implementation requires a new method version.
