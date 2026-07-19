# QuickPLS 0.9.0-rc.1 Supported Scope

QuickPLS 0.9.0-rc.1 is validated for the method shapes and product workflows covered by the `publication_ready_v0_1_to_v0_8` audit. Anything outside this file, the method specifications, and the known-differences register is unsupported unless a later audit explicitly adds it.

## Product Scope

- Offline Windows desktop application.
- Project archives, data import, saved analysis recipes, immutable saved runs, reports, and export surfaces.
- Visual SEM/PLS diagram editing and publication SVG export.
- Result estimates on diagrams only after a saved run exists and is selected.
- CLI and desktop use the same engine and serialized result contracts.

## Data Scope

- Raw numeric data for audited model workflows.
- CSV, TSV, XLSX, SAV, and covariance/correlation import validation where covered by v0.2 audit artifacts.
- Listwise missing-data handling and metadata policies as documented in method specs.
- Malformed, duplicate-header, mixed-type, corruption, migration, and recovery fixtures covered by the publication audit.

## Method Scope

- PLS-SEM core and documented assessment/inference metrics where publication audit artifacts exist.
- Extended PLS families, prediction/heterogeneity, CB-SEM/CFA ML, GSCA, PCA, regression, PROCESS-style workflows, and NCA only for their documented bounded shapes.
- Unsupported cases remain blocked or explicitly marked out of scope in diagnostics and exports.
- Stable/default exports include only audited stable surfaces; supplemental payloads require explicit scope-aware export.

## Explicit Limits

- SmartPLS project files are not imported.
- No reverse-engineering, binary decompilation, or undocumented SmartPLS behavior matching is part of QuickPLS.
- No claim is made that QuickPLS is identical to SmartPLS.
- Native CLI PDF and PNG export are not included in v0.9.0-rc.1.
- Code signing is not included in this unsigned RC.
- R/Rscript are validation-only and are never runtime dependencies.

## Required Release Commands

```powershell
npm run qpls:publication:all
npm run qpls:v09:build
npm run qpls:v09:smoke
npm run qpls:v09:audit
cargo run -p qpls-cli -- gate v0_9_publication_release_candidate
```

The RC is clear only when the v0.9 gate reports `4/0/0` and `publication_ready_v0_1_to_v0_8` remains `15/0/0`.
