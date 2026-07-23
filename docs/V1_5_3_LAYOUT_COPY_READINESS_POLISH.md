# QuickPLS v1.5.3 Layout, Copy, And Readiness Polish

## Scope

This milestone completes the screen-review recommendations from `docs/V1_5_2_SCREEN_REVIEW_SMARTPLS_USER_AUDIT.md`. It is frontend-only product polish.

No statistical engines, formulas, method-validation logic, analysis recipes, result schemas, project format, or numerical fingerprints are changed.

## Completed Changes

- Shared cards now render title, description, and actions as separate stacked regions to prevent title/body collisions.
- The repeated top warning is replaced by a compact blocker chip that links to the relevant workspace.
- Disabled primary actions show local, action-specific reasons in Setup, Run, Results, and Report.
- Workspace navigation resets the main content scroll to the top when changing rail sections.
- Home is tighter and project-action focused.
- Data includes selected-column profile statistics and hides old validation-fixture wording from normal copy.
- Model editing hides generic `Path` labels unless a path is selected, reducing canvas clutter before results exist.
- Setup uses `Scope status` and scope-specific wording instead of contradictory experimental labels for validated methods.
- Group and prediction workflows are progressive under Expert setup.
- Run uses a compact readiness display and blocks Results handoff until a run exists.
- Results empty states now point to the actual blocker and preview the future result sections.
- Report presets, checkboxes, export disabled reasons, and diagram preview wording are aligned and readable.

## Evidence

- `validation/v153_layout_copy_smoke.mjs`
- `validation/v153_layout_copy_audit.py`
- `validation/results/v153_layout_copy_smoke.json`
- `validation/results/v153_layout_copy_audit.json`
- `validation/results/screens/v153/layout-copy/`
- Versioned desktop artifacts from `npm run qpls:desktop:build-versioned` using the label `v1_5_3_layout_copy_readiness_polish`.

## Gate

```powershell
npm run qpls:v153:layout-copy-polish
cargo run -p qpls-cli -- gate v1_5_3_layout_copy_readiness_polish
```

## Boundaries

- SVG remains the audited publication diagram export.
- Native PDF/PNG remain out of scope.
- Desktop Windows remains the target; mobile behavior is non-gating.
- Versioned desktop artifacts use version `1.5.3` and the `v1_5_3_layout_copy_readiness_polish` label so prior installer/portable files are not overwritten.
