# QuickPLS v2.1.2 Setup/Run Mockup Alignment

## Summary

`v2_1_2_setup_run_mockup_alignment` applies the QuickPLS 2.1 desktop design-system primitives to the Setup and Run workspaces. The milestone focuses on method choice, applicability, readiness, calculation launch, and output preview surfaces.

This is a frontend/product-only milestone. It does not change statistical engines, formulas, method validation, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## User-Facing Scope

- Setup now uses the shared v2 page shell, page header, panels, status badges, cards, and guided method surfaces.
- Run now uses the same v2 shell and panel language as Home/Data, with a clearer calculation launch, readiness checklist, output preview, and execution provenance surface.
- Method applicability and disabled-run reasons remain visible near the action that they affect.
- The existing `quickpls:run-analysis` event wiring is preserved.
- `R²` text is rendered directly and must not regress to mojibake.

## Evidence

- `validation/v2112_setup_run_mockup_smoke.mjs`
- `validation/v2112_setup_run_mockup_audit.py`
- `validation/results/v2112_setup_run_mockup_smoke.json`
- `validation/results/v2112_setup_run_mockup_audit.json`
- Screenshots under `validation/results/screens/v2112/setup-run/`

## Verification

```powershell
npm run build
npm run qpls:v2112:setup-run-smoke
npm run qpls:v2112:setup-run-audit
cargo run -p qpls-cli -- gate v2_1_2_setup_run_mockup_alignment
```

For release artifacts:

```powershell
npm run qpls:desktop:build-versioned
```

Artifacts must be written under `D:\QuickPLS\target\release\artifacts` with version `2.1.2`, milestone label `v2_1_2_setup_run_mockup_alignment`, timestamp, architecture, and artifact type.

## Non-Goals

- No estimator or numerical result changes.
- No method promotion or scope expansion.
- No project archive migration.
- No SmartPLS equivalence or project-import claim.
