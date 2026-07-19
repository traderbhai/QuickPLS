# QuickPLS 1.0.0 Supported Scope

QuickPLS 1.0.0 is stable for the documented supported scope covered by the `publication_ready_v0_1_to_v0_8`, `v0_9_publication_release_candidate`, and `v0_9_3_professional_sem_designer` gates.

Anything outside this document, the method specifications, and the v1 known-differences register is unsupported unless a later audit explicitly adds it.

## Product Scope

- Offline Windows desktop application.
- Project archives, data import, saved analysis recipes, immutable saved runs, report tables, and export surfaces.
- SEM/PLS diagram editing with the v0.9.3 professional SEM designer.
- Publication SVG diagram export from the current canvas or tidy publication layout.
- Diagram estimates only after a compatible completed run is selected.
- CLI and desktop execution through the same engine contracts and serialized result payloads.

## Data Scope

- Raw numeric data for audited workflows.
- CSV, TSV, XLSX, SAV, covariance, and correlation import validation where covered by the v0.2 and publication audit artifacts.
- Listwise missing-data handling and metadata policies as documented in method specifications.
- Malformed input, duplicate header, mixed type, corruption, migration, archive, autosave, and recovery checks covered by audit artifacts.

## Method Scope

- PLS-SEM core, assessment, inference, extended PLS, prediction/heterogeneity, bounded CB-SEM/CFA ML, bounded GSCA, PCA, regression/PROCESS, and NCA only for shapes explicitly covered by method specs and v1 audit artifacts.
- Stable/default exports include only audited stable surfaces.
- Supplemental or broader payloads require explicit opt-in and visible watermarks.
- Unsupported model shapes are blocked or labeled out of scope in diagnostics, docs, UI, or exports.

## Explicit Limits

- SmartPLS project files are not imported.
- QuickPLS does not claim identity with SmartPLS or reproduction of undocumented SmartPLS behavior.
- No reverse-engineering, binary decompilation, or proprietary behavior matching is part of QuickPLS.
- R/Rscript and R packages are validation-only tools and are not runtime dependencies.
- Native CLI PDF and PNG export are not part of v1.0.0.
- SVG is the audited publication diagram export.
- Residual/error and caption recipe semantics are post-v1 unless separately implemented and audited.
- The v1.0.0 installer is unsigned unless a signing certificate is supplied and a separate signing audit is passed.

## Required Release Commands

```powershell
npm run qpls:publication:all
npm run qpls:v093:sem-designer
npm run tauri -- build
npm run qpls:v10:audit
cargo run -p qpls-cli -- gate v1_0_stable
```

The v1.0.0 gate is clear only when `v1_0_stable` reports `0 open / 0 blocked`.
