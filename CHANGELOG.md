# QuickPLS Changelog

This changelog summarizes public product releases. Detailed historical milestone notes remain under `docs/`.

## [2.50.0] - 2026-08-21

- Integrated the Rank 0–3 General SEM upgrade into the existing Windows desktop application.
- Promoted bounded General SEM mediation, simultaneous moderation, higher-order PLS, two-way moderated mediation, CB-SEM ML, and recursive-SEM bootstrap cells to scoped Standard.
- Unified PLS-SEM and CB-SEM around one Canvas, native preflight, calculation monitor, canonical Results workspace, export action, and schema-6 save/reopen authority.
- Added canonical General SEM CSV, XLSX, HTML, PDF, SVG, and PNG publication.
- Published Windows setup, portable, and CLI binaries with SHA-256 checksums as an unsigned GitHub pre-release.

See the [2.50.0 release notes](docs/RELEASE_NOTES_V2_50_0.md) and [GitHub Release](https://github.com/traderbhai/QuickPLS/releases/tag/v2.50.0).

## Earlier releases

### 1.8.1 - 2026-07-29

#### Added

- Method applicability guidance for dataset/model-aware setup.
- Recommended, available, needs setup, not applicable, unsupported, and experimental method states.
- Data and Model guidance panels explaining what analyses fit the current project.
- Results and Report refinement from real-like dataset audits.
- Value-specific interpretation panels and report wording support.
- Updated GitHub README, installation guide, quick start, user guide, build guide, screenshots, release notes, and checksums.

#### Changed

- Top-bar method selection became conservative and directs users to Setup for the full method catalog.
- Bootstrap, permutation, group workflows, NCA, regression, prediction, and other workflows gained required setup fields and exact unavailable reasons.
- GitHub documentation began pointing to versioned release artifacts under `target/release/artifacts`.

#### Known limits

- Installer remained unsigned.
- QuickPLS remained proprietary source-available software, not open source.
- SmartPLS project import and SmartPLS equivalence claims were not supported.
- Native PDF/PNG export was not promoted at that milestone.

### 1.0.0 - 2026-07-19

First stable QuickPLS release for its documented supported scope.

#### Added

- Offline Windows desktop application with project archives, data import, saved runs, reports, and CLI workflows.
- Professional SEM designer with academic canvas style, draggable indicators, persistent layout metadata, result overlays, and SVG publication export.
- v1.0 supported-scope documentation, compatibility matrix, known-differences register, methodology manual, validation artifact index, and release audits.
- Numerical, product-scope, desktop recovery, performance/reproducibility, and packaging evidence for the documented v1 predicate.

Earlier milestone notes are retained as `docs/RELEASE_NOTES_*.md` and version-specific documents under `docs/`.

[2.50.0]: https://github.com/traderbhai/QuickPLS/releases/tag/v2.50.0
