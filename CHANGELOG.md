# Changelog

## 1.8.1 - 2026-07-29

Current GitHub-ready release.

### Added

- Method applicability guidance for dataset/model-aware setup.
- Recommended, available, needs setup, not applicable, unsupported, and experimental method states.
- Data and Model guidance panels explaining what analyses fit the current project.
- Results and Report refinement from real-like dataset audits.
- Value-specific interpretation panels and report wording support.
- Updated GitHub README, installation guide, quick start, user guide, build guide, screenshots, release notes, and checksums.

### Changed

- Top-bar method selection is now conservative and directs users to Setup for the full method catalog.
- Bootstrap, permutation, group workflows, NCA, regression, prediction, and other workflows are surfaced with required setup fields and exact unavailable reasons.
- GitHub documentation now points to versioned release artifacts under `target/release/artifacts`.

### Known Limits

- Installer remains unsigned.
- QuickPLS remains proprietary source-available software, not open source.
- SmartPLS project import and SmartPLS equivalence claims are not supported.
- Native PDF/PNG export is not promoted unless separately audited.

## 1.0.0 - 2026-07-19

First stable QuickPLS release for the documented supported scope.

### Added

- Offline Windows desktop application with project archives, data import, saved runs, reports, and CLI workflows.
- Professional SEM designer with academic canvas style, draggable indicators, persistent layout metadata, result overlays, and SVG publication export.
- v1.0 supported-scope documentation, compatibility matrix, known differences register, methodology manual, validation artifact index, and release audits.
- Final v10 audits for numerical discrepancies, product scope, desktop smoke/recovery, performance/reproducibility, and packaging.

### Validated Scope

- PLS-SEM core, assessment, inference, extended PLS, prediction/heterogeneity, bounded CB-SEM/CFA ML, bounded GSCA, PCA, regression/PROCESS, and NCA for documented method shapes.

### Known Limits

- Installer is unsigned.
- SmartPLS project import is not supported.
- Native CLI PDF/PNG export is post-v1.
- Ordinal/polychoric/WLSMV/FIML CB-SEM is post-v1.
