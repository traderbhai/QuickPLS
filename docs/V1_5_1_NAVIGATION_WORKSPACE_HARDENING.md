# QuickPLS v1.5.1 Navigation Workspace Hardening

QuickPLS v1.5.1 refines the extreme-left navigation into a clearer desktop researcher workflow:

```text
Home -> Data -> Model -> Setup -> Run -> Results -> Report
```

The change is UI-only. It does not alter statistical engines, analysis recipes, result schemas, project numerical fingerprints, or method validation claims.

## Completed Changes

- `Start` is now `Home`, with project status, save/open access, demo entry, data/model/report shortcuts, and recovery/autosave guidance.
- `Validate` is now `Setup`, because the screen configures methods, validates readiness, and exposes basic/expert settings.
- `Groups` is removed from the primary rail. Group and segmentation workflows are configured in Setup and reviewed in the `Groups` tab inside Results.
- Data now includes an explicit next-step bridge to the SEM designer after import and metadata review.
- Results includes a visible Groups workflow bridge and keeps group-specific outputs under the Results workspace.
- Command palette entries now use `Open Setup`, `Open Results: Groups`, and `Open Publication Report`.
- The rail tooltips describe each workspace in researcher terms.

## Evidence

- `validation/v151_navigation_smoke.mjs` captures screenshots for Home, Data, Model, Setup, Run, Results Summary, Results Groups, and Report.
- `validation/v151_navigation_audit.py` verifies rail labels, terminology, screen availability, smoke evidence, versioning, and registry wiring.
- Gate: `cargo run -p qpls-cli -- gate v1_5_1_navigation_workspace_hardening`.

## Build Notes

Versioned desktop artifacts must use version `1.5.1` and label `v1_5_1_navigation_workspace_hardening` so this installer does not overwrite prior v1.5.0 artifacts.
