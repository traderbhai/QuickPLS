# QuickPLS 0.9.0-rc.1 Release Notes

QuickPLS 0.9.0-rc.1 is a Windows release candidate for the documented supported scope that passed the `publication_ready_v0_1_to_v0_8` audit. It is not v1.0 and it does not broaden support beyond the method specifications, validation artifacts, and known-differences register shipped in this repository.

## Release Status

- Version: `0.9.0-rc.1`.
- Platform: Windows desktop.
- Runtime model: fully offline, no account, no activation, no telemetry, no cloud storage, and no remote computation.
- Installer status: unsigned NSIS release-candidate installer.
- Expected Windows behavior: SmartScreen or Windows Defender may warn because the RC is not code-signed. This is a packaging trust warning, not an online activation requirement.
- Code signing: pending until a signing certificate is provided and a separate signing audit is completed.

## Supported Scope

The release is validated for the documented supported scope described in `docs/SUPPORTED_SCOPE_V0_9_RC1.md`, `docs/METHOD_COMPATIBILITY.md`, `docs/KNOWN_DIFFERENCES.md`, and the publication audit artifacts under `validation/results/`.

QuickPLS does not claim unrestricted SmartPLS equivalence. Results are independently implemented from documented methods and validated through the repository evidence suite. Unsupported shapes must remain blocked or explicitly marked outside scope.

## Export Surface

- CLI: CSV, HTML, XLSX.
- Desktop/report workspace: CSV, HTML, XLSX, SVG diagram, and browser print-to-PDF.
- Native CLI PDF and PNG export are not part of v0.9.0-rc.1 unless implemented and audited separately later.

## Installer And Portable Launch

- Production executable: `target/release/quickpls-desktop.exe`.
- NSIS installer: `target/release/bundle/nsis/*.exe`.
- Portable launch: run `quickpls-desktop.exe` directly from the release build folder.
- Installer launch: run the NSIS installer and accept the expected unsigned-app Windows warning if you trust this local build.

## Validation Index

Required gate artifacts:

- `validation/results/publication_ready_audit.json`
- `validation/results/foundation_publication_audit.json`
- `validation/results/data_project_publication_audit.json`
- `validation/results/pls_publication_audit.json`
- `validation/results/assessment_publication_audit.json`
- `validation/results/inference_publication_audit.json`
- `validation/results/extended_pls_publication_audit.json`
- `validation/results/prediction_heterogeneity_publication_audit.json`
- `validation/results/cbsem_publication_audit.json`
- `validation/results/extended_methods_publication_audit.json`
- `validation/results/gui_diagram_publication_audit.json`
- `validation/results/stable_export_publication_audit.json`
- `validation/results/documentation_publication_audit.json`
- `validation/results/performance_release_publication_audit.json`
- `validation/results/v09_smoke_check.json`
- `validation/results/v09_release_candidate_audit.json`

## Non-Goals

- SmartPLS project files are not imported.
- SmartPLS binaries are not decompiled, converted, or reverse-engineered.
- No statement in this RC should be read as unrestricted v1.0 stability.
- R/Rscript and R packages are validation-only tools; they are not bundled and are not QuickPLS runtime dependencies.
