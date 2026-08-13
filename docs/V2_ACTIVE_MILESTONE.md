# QuickPLS 2.x Active Milestone

This file is the single active working tracker for the QuickPLS 2.x frontend program. Keep detailed one-off plans out of chat and avoid creating a new planning document unless a milestone is completed and needs a permanent evidence note.

## Operating Rules

- Work in larger grouped milestones, not tiny micro-fixes.
- Keep this file as the single active milestone tracker instead of repeating full plans in chat. Permanent docs are added only when a milestone is completed or needs evidence.
- Keep QuickPLS 2.x UI work frontend-only unless backend or statistical changes are explicitly requested.
- For pure UI milestones, run `npm run build`, the targeted smoke/audit, and the final gate.
- Run full Rust test coverage only before versioned desktop artifact builds.
- Build installer, portable exe, and checksums only for completed milestone versions.
- Store versioned desktop artifacts under `D:\QuickPLS\target\release\artifacts` using the existing non-overwriting naming convention.
- Avoid broad repository scans unless needed; inspect target files first.
- Do not create versioned desktop artifacts for small patches or unfinished milestone work.
- Do not commit or push unless explicitly requested.

## Current Checkpoint

### Compact native workbench redesign pass (2026-08-09)

The default native shell now uses a compact statistical-workbench layout instead of per-screen ribbons and a vertical application rail.

- one context-sensitive command row replaces the tall Home, Data, Model, Setup, Run, Results, Report, Trust, and Settings ribbons;
- primary workflow destinations use a document-style horizontal workspace strip;
- the live Model workbench remains bound to `Explorer`, `ModelCanvas`, `Inspector`, and `ModelIssuesPane`;
- model guidance cards are removed from the persistent left pane so the project tree, canvas, property inspector, and output tray remain dominant;
- results retain completed-result-only binding and use the existing workbook/table/interpretation layout;
- desktop scaling keeps the shell within the viewport at 1440 x 900 and 1024 x 768, with compact icon treatment at narrower widths;
- the visual direction is independently branded QuickPLS and does not copy SmartPLS assets, product identity, or proprietary implementation details.

Verification for this pass:

- `npm run build`
- `npm run qpls:v244:production-binding-audit`
- `npm run qpls:v245:mockup-feature-audit`
- rendered browser QA: one compact command surface, zero visible legacy ribbons, one React Flow pane, 13 model nodes, and no console errors.

Current completed checkpoint: `v2_43_0_full_native_frontend_backend_wiring`.

Verified evidence:

- `npm run build`
- `npm run qpls:v2210:native-shell-smoke`
- `npm run qpls:v2210:native-shell-audit`
- `cargo run -p qpls-cli -- gate v2_21_0_desktop_design_system_shell`
- `npm run qpls:v2220:native-commands-smoke`
- `npm run qpls:v2220:native-commands-audit`
- `cargo run -p qpls-cli -- gate v2_22_0_menu_commands_dialogs_native_base`
- `npm run qpls:v2230:home-smoke`
- `npm run qpls:v2230:home-audit`
- `cargo run -p qpls-cli -- gate v2_23_0_home_project_manager`
- `npm run qpls:v2240:data-workbench-smoke`
- `npm run qpls:v2240:data-workbench-audit`
- `cargo run -p qpls-cli -- gate v2_24_0_data_workbench_redesign`
- `npm run qpls:v2250:model-workbench-smoke`
- `npm run qpls:v2250:model-workbench-audit`
- `cargo run -p qpls-cli -- gate v2_25_0_model_workbench_integration`
- `npm run qpls:v2260:method-setup-smoke`
- `npm run qpls:v2260:method-setup-audit`
- `cargo run -p qpls-cli -- gate v2_26_0_method_setup_applicability_center`
- `npm run qpls:v2270:run-monitor-smoke`
- `npm run qpls:v2270:run-monitor-audit`
- `cargo run -p qpls-cli -- gate v2_27_0_calculation_run_monitor`
- `npm run qpls:v2280:results-workbook-smoke`
- `npm run qpls:v2280:results-workbook-audit`
- `cargo run -p qpls-cli -- gate v2_28_0_results_workbook_redesign`
- `npm run qpls:v2290:research-tables-smoke`
- `npm run qpls:v2290:research-tables-audit`
- `cargo run -p qpls-cli -- gate v2_29_0_research_table_system`
- `npm run qpls:v2300:reportability-assistant-smoke`
- `npm run qpls:v2300:reportability-assistant-audit`
- `cargo run -p qpls-cli -- gate v2_30_0_interpretation_reportability_assistant`
- `npm run qpls:v2310:report-export-wizard-smoke`
- `npm run qpls:v2310:report-export-wizard-audit`
- `cargo run -p qpls-cli -- gate v2_31_0_report_export_wizard`
- `npm run qpls:v236:native-ui-spec-audit`
- `cargo run -p qpls-cli -- gate v2_36_0_native_desktop_ui_spec_and_component_plan`
- `npm run build`
- `npm run qpls:v237:native-prototype-smoke`
- `npm run qpls:v237:native-prototype-audit`
- `cargo run -p qpls-cli -- gate v2_37_0_native_frontend_prototype_shell`
- `npm run build`
- `npm run qpls:v238:native-adapters-smoke`
- `npm run qpls:v238:native-adapters-audit`
- `cargo run -p qpls-cli -- gate v2_38_0_native_frontend_backend_adapters`
- `npm run build`
- `npm run qpls:v239:screen-replacement-smoke`
- `npm run qpls:v239:screen-replacement-audit`
- `cargo run -p qpls-cli -- gate v2_39_0_native_frontend_screen_replacement_plan`
- `npm run build`
- `npm run qpls:v2400:mockup-fidelity-smoke`
- `npm run qpls:v2400:mockup-fidelity-audit`
- `cargo run -p qpls-cli -- gate v2_40_0_mockup_fidelity_native_shell_alignment`
- `npm run build`
- `npm run qpls:v2410:mockup-manifest-audit`
- `npm run qpls:v2410:mockup-parity-smoke`
- `npm run qpls:v2410:mockup-parity-audit`
- `cargo run -p qpls-cli -- gate v2_41_0_full_mockup_screen_parity_pass`
- `npm run build`
- `npm run qpls:v2420:native-default-smoke`
- `npm run qpls:v2420:native-default-audit`
- `cargo run -p qpls-cli -- gate v2_42_0_make_native_mockup_shell_default`
- `npm run qpls:v2421:screen-qa`
- `npm run qpls:v2421:interaction-wiring`
- `npm run qpls:v2421:web-trace-audit`
- `npm run qpls:v2421:qa-test-pack`
- `cargo run -p qpls-cli -- gate v2_43_0_full_native_frontend_backend_wiring`

Latest completed artifact checkpoint:

- `QuickPLS_2.29.0_v2_29_0_research_table_system_20260730-102405_x64_setup.exe`
- `QuickPLS_2.29.0_v2_29_0_research_table_system_20260730-102405_x64_portable.exe`
- `QuickPLS_2.29.0_v2_29_0_research_table_system_20260730-102405_x64_checksums.txt`

## Next Active Milestone

- milestone id: `v2_42_0_make_native_mockup_shell_default`
- user-facing problem solved: the v2 native mockup-parity workbench is now the default app UI instead of an opt-in query route.
- target screens/components: default app route, compatibility `?native_shell=1` route, explicit `?native_prototype=1` static prototype route, explicit `?legacy_shell=1` fallback route, all native workbench screens.
- targeted smoke/audit command: `npm run qpls:v2420:native-default`
- final gate: `cargo run -p qpls-cli -- gate v2_42_0_make_native_mockup_shell_default`

Acceptance boundary:

- frontend/product work only unless explicitly broadened;
- no estimator, method-validation, result-schema, project-archive, or numerical-fingerprint changes;
- private datasets may be reviewed manually, but raw private files and value-revealing screenshots must stay outside the repository;
- no versioned desktop artifacts until the current milestone gate is clear and full pre-artifact tests have passed.
