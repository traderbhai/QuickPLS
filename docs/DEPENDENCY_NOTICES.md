# QuickPLS Dependency Notices

QuickPLS 0.9.0-rc.1 is a free proprietary desktop application. Distributed dependencies must remain compatible with that distribution model before public release.

## Runtime Stack

- Tauri 2 desktop shell and Rust backend.
- React and TypeScript frontend.
- React Flow model canvas.
- Apache Arrow data representation through Rust crates.
- Rust numerical and utility crates used by the QuickPLS workspace, including matrix, optimization, random-stream, distribution, archive, and serialization libraries.

## Validation-Only Tools

The following tools may be used during development validation but are not bundled with QuickPLS and are not runtime requirements:

- R and Rscript.
- lavaan, cSEM, seminr, plspm, boot, sandwich, lmtest, and other R packages used by validation scripts.
- Python/NumPy reference scripts under `validation/`.

## Licensing Rules

- GPL reference engines may be executed only as development validation references.
- GPL code must not be linked, copied, translated into the proprietary runtime, or distributed inside the QuickPLS application.
- Any new dependency added after this RC must be reviewed before release packaging.
- Distribution must include license texts and notices required by the final dependency inventory.

## Release-Candidate Packaging

- The v0.9.0-rc.1 NSIS installer is unsigned.
- Code signing, certificate handling, and final legal review are release tasks after this RC unless a certificate is provided earlier.
- No account, activation server, telemetry, cloud sync, or remote computation dependency is permitted.
