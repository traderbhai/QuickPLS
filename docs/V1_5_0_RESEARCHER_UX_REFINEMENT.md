# QuickPLS v1.5.0 Researcher UX Refinement

QuickPLS v1.5.0 continues the frontend polish program with desktop workflow improvements that make the app easier to operate repeatedly during research work. This milestone does not change statistical engines, formulas, method validation tolerances, result schemas, project recipe semantics, or numerical fingerprints.

## Delivered UX Improvements

- Desktop command palette opened with `Ctrl+K`.
- Keyboard shortcut overlay opened with `?`.
- Toast notifications for save, project open/recovery, dataset import, export, and completed runs.
- Clearer status bar messaging for autosave state, offline mode, engine scope, and shortcut access.
- Method setup “What will run” summary showing selected method, dataset, construct/path counts, seed, workers, bootstrap, permutation, and validation scope.
- Results workspace headline cards for selected run, strongest `R²`, path count, and warnings.
- Current-table CSV export from the Results workspace.
- Publication export stepper: select run, choose diagram style, preview figure, export tables/SVG.
- Variable prefix grouping chips in the SEM Explorer for faster construct creation from datasets.

## Evidence

- `validation/v150_researcher_ux_smoke.mjs`
- `validation/v150_researcher_ux_audit.py`
- `validation/results/v150_researcher_ux_smoke.json`
- `validation/results/v150_researcher_ux_audit.json`
- `validation/results/screens/v150/researcher-ux/`

## Gate

```powershell
npm run qpls:v150:researcher-ux
cargo run -p qpls-cli -- gate v1_5_0_researcher_ux_refinement
```

Desktop release artifacts are generated with:

```powershell
npm run qpls:desktop:build-versioned
```

The artifact label is `v1_5_0_researcher_ux_refinement`, and the app version is `1.5.0`.
