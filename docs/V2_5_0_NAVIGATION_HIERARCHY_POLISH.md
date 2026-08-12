# QuickPLS v2.5.0 Navigation Hierarchy Polish

Status: validated.

This frontend-only milestone clarifies the QuickPLS 2.x desktop shell by separating the main research workflow from support utilities in the left navigation rail.

## Scope

- Primary rail workflow: Home, Data, Model, Setup, Run, Results, Report.
- Support utilities: Trust and Settings.
- Keep all existing workspace routes and behavior available.
- Preserve the existing workflow strip and workspace coach behavior.
- No statistical engines, formulas, result schemas, recipes, project archive format, validation tolerances, or numerical fingerprints change.

## Evidence

- `src/components/NavRail.tsx`
- `src/styles.css`
- `validation/v250_navigation_hierarchy_smoke.mjs`
- `validation/v250_navigation_hierarchy_audit.py`
- `validation/results/v250_navigation_hierarchy_smoke.json`
- `validation/results/v250_navigation_hierarchy_audit.json`

## Verification

```powershell
npm run build
npm run qpls:v250:navigation-smoke
npm run qpls:v250:navigation-audit
cargo run -p qpls-cli -- gate v2_5_0_navigation_hierarchy_polish
```

Expected gate:

```text
v2_5_0_navigation_hierarchy_polish
gates passed/open/blocked: all passed / 0 open / 0 blocked
promotion gate: clear
```

## Artifact Rule

Only completed milestone versions should create artifacts. For v2.5.0, artifacts must use:

```text
QuickPLS_2.5.0_v2_5_0_navigation_hierarchy_polish_<timestamp>_x64_setup.exe
QuickPLS_2.5.0_v2_5_0_navigation_hierarchy_polish_<timestamp>_x64_portable.exe
QuickPLS_2.5.0_v2_5_0_navigation_hierarchy_polish_<timestamp>_x64_checksums.txt
```
