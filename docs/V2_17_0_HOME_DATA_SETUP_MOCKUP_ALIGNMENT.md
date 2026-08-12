# QuickPLS v2.17.0 Home, Data, And Setup Mockup Alignment

Status: `validated`

This frontend-only milestone aligns the first three workflow workspaces with the QuickPLS 2.0 mockup direction. It uses the v2.16 desktop shell and tightens the Home launcher, Data workbench, and Setup method guidance into denser, native-desktop-style screens.

## Scope

- Home now carries an explicit v2.17 launcher contract with compact current-project, project-action, workflow, and summary sections.
- Data now carries an explicit v2.17 workbench contract with import source, data quality overview, prefix bridge, preview table, and metadata inspector.
- Setup now carries an explicit v2.17 method-guidance contract with selected method summary, readiness, recommended methods, selected-method requirements, presets, and run handoff.
- Version metadata advances to `2.17.0`.
- No statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.

## Evidence

- `src/components/OnboardingWorkspace.tsx`
- `src/components/DataWorkspace.tsx`
- `src/components/AnalysisCatalog.tsx`
- `src/styles.css`
- `validation/v2170_home_data_setup_smoke.mjs`
- `validation/v2170_home_data_setup_audit.py`
- `validation/results/v2170_home_data_setup_smoke.json`
- `validation/results/v2170_home_data_setup_audit.json`
- gate `v2_17_0_home_data_setup_mockup_alignment`

## Boundaries

The milestone is a visual/product alignment pass only. Existing commands remain wired to current store actions, events, and native Tauri file dialogs. SEM canvas behavior is not changed in this milestone.
