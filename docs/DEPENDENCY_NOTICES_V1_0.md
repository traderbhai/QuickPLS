# QuickPLS 1.0.0 Dependency Notices

QuickPLS 1.0.0 is a free proprietary desktop application. Dependencies distributed with the application must remain compatible with that distribution model.

## Runtime Stack

- Tauri 2 desktop shell and Rust backend.
- React and TypeScript frontend.
- React Flow canvas.
- Rust workspace crates for data, project archives, estimation, assessment, resampling, runner, CLI, and desktop integration.

## Validation-Only Tools

These tools may be used during development validation but are not bundled and are not runtime requirements:

- R and Rscript.
- lavaan, cSEM, seminr, plspm, boot, sandwich, lmtest, and similar R packages.
- Python/NumPy validation scripts.

## Licensing Rules

- GPL reference engines are validation references only.
- GPL code must not be linked, copied, translated into the proprietary runtime, or distributed inside QuickPLS.
- Any new distributed dependency after v1.0.0 requires a dependency-license review.

## Packaging

- The default v1.0.0 installer is unsigned unless a signing certificate is provided.
- No account, activation server, telemetry, cloud sync, or remote computation is permitted.
