# QuickPLS v2.0.11 Mockup Pixel Alignment

v2.0.11 is a frontend/product-only milestone that tightens the QuickPLS 2.0 mockup-alignment pass by closing the medium visual gaps recorded by the v2.0.10 visual audit.

## Scope

- Results empty state now uses the same v2 command surface as populated Results.
- Results, Setup, Report, and Trust expose a visible confidence route such as `Why trust this result?`.
- Remaining normal UI `R²` mojibake is blocked by source and screenshot checks.
- Fresh desktop screenshots are captured for primary workspaces at 1440x900 plus key surfaces at 1280x800.

## Non-Goals

- No estimator changes.
- No formula, validation tolerance, result schema, project archive, or numerical fingerprint changes.
- No SmartPLS equivalence, SmartPLS project import, or undocumented-behavior claim.

## Verification

- `npm run qpls:v211:pixel-smoke`
- `npm run qpls:v211:pixel-audit`
- `cargo run -p qpls-cli -- gate v2_0_11_mockup_pixel_alignment`

## Release Artifacts

Versioned desktop artifacts must be generated with:

```powershell
npm run qpls:desktop:build-versioned
```

The output remains under `D:\QuickPLS\target\release\artifacts` with installer, portable executable, and checksums.
