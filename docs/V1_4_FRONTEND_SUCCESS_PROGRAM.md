# QuickPLS v1.4 Frontend Success Program

QuickPLS v1.4 is a frontend-focused release line for making the Windows desktop app feel more polished, researcher-oriented, and predictable. It does not change statistical formulas, estimator behavior, method validation tolerances, numerical result schemas, or analysis fingerprints.

## Completed Milestones

- `v1_4_0_frontend_design_system_foundation`: shared desktop UI primitives, density tokens, consistent status badges, and wording/mojibake checks.
- `v1_4_1_sem_designer_completion`: stronger SEM canvas controls, locked result/publication behavior, and large-model view actions.
- `v1_4_2_explorer_inspector_simplification`: global SEM explorer search, issue filters, and lighter contextual inspector sections.
- `v1_4_3_method_setup_experience`: Basic/Expert setup modes, readiness cards, and saved method presets.
- `v1_4_4_results_workspace_redesign`: researcher workflow tabs, table search/density tools, and result-to-diagram focus hooks.
- `v1_4_5_publication_export_workflow`: journal/thesis/presentation export presets and clearer WYSIWYG report controls.
- `v1_4_6_onboarding_demo_workflow`: desktop-first Start workspace with new/open/demo/import/recent project entry points.
- `v1_4_7_large_model_desktop_polish`: UI-only large-model state, isolate/collapse/focus controls, and versioned artifact labeling.

## Non-Engine Boundary

The v1.4 work is intentionally limited to product and UI layers. New state such as `UiPreferences`, `ResultWorkspaceState`, `MethodSetupState`, `OnboardingState`, and `LargeModelViewState` is UI-only. It must not alter model recipes, run provenance, estimator crates, validated formulas, or numerical fingerprints.

## Evidence

- `validation/v14_frontend_success_audit.py`
- `validation/v14_frontend_success_smoke.mjs`
- `validation/results/v1_4_*_audit.json`
- `validation/results/v14_frontend_success_smoke.json`
- `validation/results/screens/v14/frontend-success/`

## Gate

```powershell
npm run qpls:v14:frontend-success
cargo run -p qpls-cli -- gate v1_4_frontend_success_program
```

Desktop release artifacts are generated with:

```powershell
npm run qpls:desktop:build-versioned
```

The artifact label is `v1_4_frontend_success_program`, and the app version is `1.4.7`.
