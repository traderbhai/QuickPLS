# QuickPLS v2.44.0 Native UI Production Binding Completion

Milestone: `v2_44_0_native_ui_production_binding_completion`

## Scope

This milestone completes the current UI-only and partially wired production-binding pass for the default native QuickPLS shell. It does not address the separate mockup features that are not fully created or not fully visible; those remain tracked for the next milestone.

## Completed Binding

- Home, recent projects, project details, message-center text, and status-bar fields now use current workspace state or persisted local recent-project metadata.
- Data workbench adapters use the active dataset for rows, columns, selected variable metadata, quality cards, and import preview content.
- Model workbench adapters use live constructs, paths, SEM canvas selection, inspector state, model issues, and selected-run result context.
- Setup and Calculation Setup surfaces use the real method applicability/settings state, including bootstrap as an add-on and method-specific output previews.
- Run, Results, and Report surfaces use selected job/run/report state rather than static run identifiers or fake telemetry.
- Trust Center and Settings command surfaces remain bound to local docs/evidence and persisted UI preferences.

## Intentional Absences

- Calculation pause/resume remains absent because QuickPLS supports cancellation, not safe job suspension.
- Native PDF/PNG export remains absent; SVG is the audited figure export and browser Print/PDF remains the documented PDF path.
- CPU and memory meters are hidden unless real lightweight telemetry is added later.
- Mockup-only extra features not in the default product remain in `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md`.

## Evidence

- `validation/results/v244_home_project_status_smoke.json`
- `validation/results/v244_data_binding_smoke.json`
- `validation/results/v244_model_binding_smoke.json`
- `validation/results/v244_setup_binding_smoke.json`
- `validation/results/v244_run_results_report_binding_smoke.json`
- `validation/results/v244_trust_settings_commands_smoke.json`
- `validation/results/v244_production_binding_audit.json`

## Verification

```powershell
npm run build
npm run qpls:v244:production-binding
cargo check -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_44_0_native_ui_production_binding_completion
```

## Numerical Boundary

This milestone changes native frontend adapters, shell status binding, UI-only validation scripts, docs, registry, and release metadata only. It does not change estimator formulas, statistical result schemas, validation tolerances, project archive semantics, or numerical fingerprints.
