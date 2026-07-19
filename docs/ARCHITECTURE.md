# QuickPLS Foundation Architecture

QuickPLS is local-first. The React desktop surface owns interaction and visualization; the Tauri boundary exposes a small typed command set; Rust owns project validation, numerical work, reproducibility, and exports.

## Boundaries

- `src/`: desktop workspace, visual model editor, data preview, capability catalog, runs, and reports.
- `crates/qpls-core`: versioned contracts, method gates, deterministic statistical primitives, and model validation.
- `crates/qpls-data`: typed research-data import, metadata, Arrow IPC, previews, and SHA-256 fingerprints.
- `crates/qpls-project`: `.qpls` manifests, checksums, atomic writes, recovery, and version handling.
- `crates/qpls-cli`: headless access to the same contracts and validation behavior.
- `src-tauri`: desktop packaging and IPC only. Statistical logic does not live here.

## Project Container

The `.qpls` container is a ZIP archive with a versioned manifest, Arrow data, JSON model and recipe contracts, and diagram layouts. Every payload has a SHA-256 checksum. Saves use a temporary archive and atomic replacement while retaining the preceding valid archive as `.qpls.bak`. Immutable result records and attachments are reserved until their schemas exist.

## Scientific Gate

A method moves from `unsupported` to `experimental` only when an executable estimator and formula tests exist. It moves to `validated` only after deterministic reference comparison, stochastic simulation where applicable, metamorphic tests, documented tolerances, and benchmark evidence. Stable UI and exports expose only validated methods.
