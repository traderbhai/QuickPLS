# QuickPLS 3 Wave 0 baseline

Captured on 12 August 2026 from branch `codx/quickpls-3-parity` before recipe-schema and archive-format modernization. This document records the recoverable engineering baseline; it is not a QuickPLS 3 release claim.

## Recovery point

- Curated source checkpoint: `4fe2d3f` (`chore(rust): reconcile current clippy mechanics`).
- Evidence-governance checkpoints: `d73fadf` and `eaf5723`.
- Pre-parity release artifacts are preserved outside the disposable Cargo cache under `D:\QuickPLS\release`.
- Portable executable: 25,013,248 bytes, SHA-256 `17C0D7D3735D959B001FA4A0058AA1A2E3367A52D87BE8D4372E9894AE845F54`.
- Setup executable: 5,386,495 bytes, SHA-256 `0B6EE07AEACB74263F76AF8F9A8D791CA946F868B87F9FA5A1FE89E9B835B2B9`.
- These binaries remain the 2.45.0 recovery build. They are not Wave 1 or QuickPLS 3 artifacts.

## Baseline gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Frontend unit/integration tests | Pass: 67 files, 519 tests | `npm test -- --run` |
| Frontend production build | Pass: 1,817 modules; largest chunks 406.43 kB application and 317.21 kB vendor | `npm run build` |
| Rust formatter | Pass | `cargo fmt --all -- --check` |
| Rust workspace tests | Pass: 215 tests; 1 intentional performance test ignored | `CARGO_BUILD_JOBS=1 cargo test --workspace --all-targets` |
| Ordinary Rust Clippy | Pass with warnings | `CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets` |
| Strict Rust Clippy | Not clean: existing estimator complexity, argument-count, and range-loop warning debt | `CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets -- -D warnings` |
| Browser visual acceptance | Pass after deterministic Model context-menu/editor focus repair: 76/76 required regular screenshots, 0 failures, 0 console errors | `validation/results/v247_native_desktop_visual_acceptance.json` |
| Parity ledger validation | Pass: 13 native-qualified, 1 engine-only, 0 release-qualified | `python validation/parity_ledger.py` |

The strict-Clippy failure is recorded debt, not concealed as a clean gate. Scientific estimator code must not be mechanically refactored merely to silence style lints without method-level regression evidence.

## Resource snapshot

After the baseline gates and before contract modernization:

- Cargo `target`: 12,988,262,978 bytes.
- Production `dist`: 1,021,690 bytes.
- Validation results: 132,360,329 bytes.
- Free space on drive D: 130,127,077,376 bytes.
- Active Cargo, rustc, Node, WebView, and QuickPLS processes: none.

The Cargo cache is intentionally retained until the first packaged acceptance run to avoid an unnecessary multi-gigabyte rebuild. Disposable screenshots, autosaves, caches, temporary projects, and installers remain ignored by source control; compact manifests and evidence reports remain tracked.

## Qualification boundary

The current ledger describes 14 accepted calculation catalogue entries, not complete SmartPLS feature equivalence. No feature is `release_qualified` at this baseline. Structural Path Randomization remains `engine_only` because current packaged evidence does not prove its completed result, export, and same-run reopen workflow. Later waves must promote each feature through the evidence ladder defined in `docs/QUICKPLS_3_PARITY_LEDGER.md`.
