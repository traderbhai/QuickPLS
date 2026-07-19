# QuickPLS

QuickPLS is an offline Windows research application for reproducible structural equation modeling. This repository currently contains a **validated v0.3 PLS core estimator** plus experimental v0.4 assessment and inference slices; the full application is not yet publication-ready.

## Implemented

- Tauri 2, React, TypeScript, and Rust workspace
- Interactive construct/path editor and construct inspector
- Typed CSV/TSV, Excel, ODS, and SPSS SAV/ZSAV import backed by Apache Arrow
- Checksummed `.qpls` archives with atomic replacement, backup recovery, and future-version read-only loading
- Native project dialogs and Rust-owned durable data; Zustand stores transient UI state only
- Versioned model and immutable analysis-recipe contracts
- Model validation, method capability gates, fingerprints, and CLI
- Tested deterministic primitives for standardization, correlation, variance, and Cronbach's alpha
- Explicit compatibility and numerical validation policies
- Validated v0.3 PLS-PM Mode A/B engine with path/factor/PCA weighting, scores, weights, loadings, paths, effects, and R2
- Gated CLI `csv`/`html` export for validated v0.3 estimator-only tables
- Development-only two-engine reference validation against Python plspm and R cSEM

## Run the desktop preview

```powershell
npm install
npm run tauri dev
```

Use the native QuickPLS window opened by Tauri for project/data dialogs and analysis jobs. The page at `http://127.0.0.1:1420` is a browser-only frontend preview; browser security prevents it from invoking the native file and engine commands.

The principal validation dataset is bundled at:

```text
D:\QuickPLS\validation\fixtures\corporate_reputation.csv
```

The compact deterministic PLS fixture is `D:\QuickPLS\validation\fixtures\simple_reflective.csv`.

## Verify

```powershell
npm run build
cargo test --workspace
cargo run -p qpls-cli -- methods
cargo run -p qpls-cli -- roadmap
cargo run -p qpls-cli -- gate v0_4_assessment_reliability
cargo run -p qpls-cli -- qualify v04-inference --refresh-quick-monte-carlo
cargo run -p qpls-cli -- demo create
cargo run -p qpls-cli -- demo validate
cargo run -p qpls-cli -- run validation/fixtures/simple_reflective.recipe.json --data validation/fixtures/simple_reflective.csv --output target/simple_reflective.result.json --allow-experimental
cargo run -p qpls-cli -- export target/simple_reflective.result.json --format csv --output target/simple_reflective.estimator.csv
cargo run -p qpls-cli -- import tests/fixtures/research-data.csv study.qpls
cargo run -p qpls-cli -- inspect study.qpls
```

The v0.3 PLS core estimator is validated for its documented scope. CLI export intentionally includes only validated estimator values. Current desktop/CLI result envelopes also include v0.4 assessment/inference artifacts, which remain experimental and must not be used as publication-ready evidence.

## Accelerated Development

QuickPLS development now uses the method-slice registry at `D:\QuickPLS\validation\development_slices.json`. Each slice records its release, status, gates, evidence, and next actions. See `D:\QuickPLS\docs\ACCELERATED_DEVELOPMENT.md` for the operating procedure.

The v0.4 demo evidence project is generated at `D:\QuickPLS\validation\demo\quickpls_v04_demo.qpls`; its expected and validation reports sit beside it.
