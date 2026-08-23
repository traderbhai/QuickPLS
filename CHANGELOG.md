# QuickPLS Changelog

This changelog summarizes public product releases. Detailed historical milestone notes remain under `docs/`.

## [2.55.5] - 2026-08-23 (local preview)

- Added three OI-based editable samples—Mediation, Moderated Mediation (Point Topology), and a disjoint two-stage Higher-Order model—while reusing one embedded 305-case dataset.
- Replaced duplicated frontend/backend sample lists with one versioned manifest that declares model topology, runs, layout, provenance, and acceptance evidence.
- Added current-engine reference tests and explicit evidence boundaries where the qualified QuickPLS specification is close to, but not identical to, a supplied screenshot.

A fresh `QuickPLS_2.55.5_x64-setup.exe` installer and provenance-bound artifact
package are required for this source. No earlier installer qualifies as a
2.55.5 candidate.

## [2.55.4] - 2026-08-23 (local preview)

- Centered every measurement loading and weight badge on its exact connector path instead of applying a generic above/below offset.
- Applied the presentation correction consistently to Straight, Curved, Orthogonal, and Polyline measurement connectors in Model, Results, and Publication diagrams.
- Kept structural-path label placement, analytical values, measurement direction, model identity, and stored results unchanged.
- Added **Organizational Identification Model** as the fourth bundled sample with 305 cases, four reflective constructs, 21 modeled indicators, three paths, and a completed PLS-SEM result.
- Added deterministic provenance and a 27/27 three-decimal screenshot-parity test while preserving every supplied row and leaving `gender` unassigned.

An earlier unsigned local 2.55.4 installer with SHA-256
`9380af48bf3ed847ce744e5d68560f296ba27ab88264015c171fed187899dce1`
predates the fourth sample and is now historical. A fresh
`QuickPLS_2.55.4_x64-setup.exe` installer and provenance-bound install/portable
smoke, evidence, and final publication audit are required for the current
source.

## [2.55.3] - 2026-08-23 (local preview)

- Added presentation-only Straight, Curved, Orthogonal, and Polyline routing for construct–indicator measurement connectors.
- Added per-connector and per-construct route controls, undoable bend editing, persistence, and orphan-route cleanup without changing model identity or analytical results.
- Retained the 2.55.2 Results → Edit Model fix so the model canvas becomes editable while completed-result diagrams remain read-only in Results.

An unsigned local installer was built at
`target/release/bundle/nsis/QuickPLS_2.55.3_x64-setup.exe` with SHA-256
`bd88a2d15a5ebeacb91279095c806b92c2b7eda79234bda3d59a9cbde52978d1`.
It is a tested local preview, not a formally qualified public candidate; public
release still requires provenance-bound install/portable smoke, evidence, and a
final publication audit.

## [2.55.2] - 2026-08-23 (local preview)

- Fixed Results → Edit Model so it exits locked result/publication presentation mode and restores an editable SEM canvas.
- Made all bundled sample projects open their Model canvas in editable mode.
- Kept completed-result diagrams read-only within Results.

The 2.55.2 local preview was superseded by 2.55.3 before public release. Its
unsigned local artifacts and historical evidence do not qualify 2.55.3.

## [2.55.1] - 2026-08-23 (local preview)

- Added the full 344-case Corporate Reputation project to the built-in sample gallery.
- Bundled its eight-construct, 31-indicator mixed formative/reflective PLS-SEM model and all 13 structural paths.
- Stored a completed QuickPLS result and added deterministic checks for 48 SmartPLS reference values at three-decimal display precision.
- Preserved the existing synthetic Corporate Reputation fixture used by lower-level engine and CLI tests.

The 2.55.1 local preview was superseded by 2.55.2 before public release because
the sample's Model canvas retained its locked result presentation mode.

## [2.55.0] - Unreleased

- Added model-aware Canvas layout and routing that reserves construct/indicator envelopes, avoids eligible obstacles, preserves pinned manual routes, and exposes presentation-only bend and moderation-anchor editing.
- Replaced the compact PROCESS sketch with the shared read-only diagram renderer and independent Fit/zoom controls.
- Consolidated the shared Calculate setup surface for 1024×700 use, stable action labels, footer-aware scrolling, scientific-first eligibility, and method-specific fixed-setting disclosure.
- Rebaselined the interaction harness around current Model/context/keyboard entry and preserved the exact 18-method public catalogue.
- Added fail-closed setup, cross-method, specialized-result, archive, export, persistence, accessibility, provenance, build, install, and packaged-evidence contracts.

Formal first diagnostic `20260822T142953Z` at source `2e3a23f` executed all 14 consolidated source-gate steps and passed 13; the sole failure was `frontend_typecheck`, where `src/data/v255NamedSemEvidenceFixtures.test.ts` reported TypeScript error `TS2339`. The final consolidated diagnostic `20260823T030939Z` at source `e5723df08b7205ce75f1887c5f4709f235ad893c` passed 14/14. Its report is `validation/results/v255_consolidated_diagnostics_20260823T030939Z/v255_consolidated_diagnostics.json`, has SHA-256 `03da7a8e0db2924d0157eb0cb0ca92e841fffd61f470d5cd16ccd58f87fe9b2a`, and is retained in evidence commit `8a727262c07dd38bae38d8154e1662c78fbb8ee7`. Both formal evidence records use runner SHA-256 `64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796`; they are separate and are not byte-identical. The final pass includes 453/453 Vitest suites and 1724/1724 tests, focused Rust authority/archive/routing/lifecycle tests, full TypeScript checking, a production frontend build, six-format semantic export readback, 17/17 rebaseline assertions with zero captured console errors, the exact 18-method setup crawl, and the final evidence contract. Renderer console/page-error evidence fails closed, attach-only phases use fresh wrapper-owned processes, and candidate/phase/trusted-driver PID, role, suite, and SHA-256 bindings are closed. Exactly one opt-in owner waiver is permitted for the actual Windows 200% scaling case; its real observed DPI screenshot and receipt remain required, it remains `waived` rather than `passed`, and the other 54 named evidence cases must pass. The superseded final diagnostic `20260823T000930Z`, candidate/install/smoke attempts `20260823T004848Z` and `20260823T005212Z`, portable probe `20260822T233111Z`, and all prior candidate, install, smoke, diagnostic, or probe attempts are historical and ineligible. One new provenance-bound unsigned 2.55 candidate build, isolated install, full installed-and-portable smoke, evidence collection and bundling, final audit, and publication remain pending. If an existing registered installation must be removed, only its exact registered uninstaller may be used; project files, recovery data, and QuickPLS application user data must remain untouched, and portable evidence does not replace installed evidence. No 2.55 download is claimed; the latest published public pre-release remains [`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). Code signing remains excluded.

## [2.54.0] - 2026-08-22

- Unified visible Canvas edits behind an authority-aware applied/blocked command path while preserving strict revisions, stable IDs, undo behavior, and presentation metadata.
- Added Relationships navigation, indicator-side controls, local tidy/alignment/distribution, and structure/all/selection fitting without changing scientific model identities.
- Reworked normal Results to use authored labels, sticky identity columns, numeric alignment, confidence metadata, and a compact Calculate action from the empty state.
- Preserved the exact 18-method catalogue and all existing numerical engines, estimands, Registry cells, and stored result identities.

The consolidated source diagnostic recorded 8/9 passing steps; the focused remediation pass then passed 69/69 targeted tests. The final unsigned Windows candidate passed the isolated 10/10 create → calculate → Results → save → fresh-reopen packaged journey with zero application-page external requests and zero console errors. Release-artifact packaging and SHA-256 checksum verification passed; code signing is excluded. See the [2.54.0 release notes](docs/RELEASE_NOTES_V2_54_0.md) and [release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0).

## [2.53.0] - 2026-08-21

- Moved mediation and moderation into the normal diagram-native Canvas → Calculate → Results workflow while preserving exactly 18 public methods.
- Added a presentation-only moderation anchor with drag, context-menu, and keyboard entry points; generated interaction terms remain hidden in normal Canvas use.
- Added separately versioned cells for exactly-one-path mediation bootstrap and bounded true three-way moderation point/bootstrap.
- Promoted all three cells independently to scoped Standard after their compact reference, consolidated integration, and post-promotion archive checks passed.

See the [2.53.0 release notes](docs/RELEASE_NOTES_V2_53_0.md) and [GitHub prerelease](https://github.com/traderbhai/QuickPLS/releases/tag/v2.53.0). The unsigned Windows candidate and packaged save/fresh-reopen smoke passed; code signing remains intentionally excluded.

## [2.52.0] - 2026-08-21

- Made higher-order construct authoring diagram-native through the Model menu, context menu, Properties pane, and one compact create/edit dialog.
- Added topology-aware RR/RF/FR/FF guidance, stable Save As Revision replacement, presentation-only component overlays, and compact Calculate routing without changing HOC engines or identities.
- Replaced ordinary PLS model-fit warning banners with a neutral descriptive-fit presentation and payload-derived exact-fit status.
- Refined categorized HOC Results and moved internal estimator/model/dataset identities to collapsed Run Details.
- Built and verified one unsigned Windows candidate with an automated HOC create, calculate, Results, save, fresh-process reopen journey.

See the [2.52.0 release notes](docs/RELEASE_NOTES_V2_52_0.md). Publication and code signing were not performed from this workspace.

## [2.51.0] - 2026-08-21

- Made Canvas the only permanent model-authoring document.
- Routed the unchanged 18-method Calculate catalogue to applicable mediation, moderation, higher-order, moderated-mediation, and bounded CB-SEM capabilities from the resident model.
- Moved parameter-level editing into an on-demand Advanced Parameter Table and retained safe calculation-ready revisions for older projects.
- Integrated advanced canonical output into the normal categorized Results workflow while keeping historical General SEM and Exact CB-SEM archives readable through compatibility adapters.

The consolidated verification, Windows artifact build, packaged smoke journey, and checksum publication passed. See the [2.51.0 release notes](docs/RELEASE_NOTES_V2_51_0.md) and [GitHub prerelease](https://github.com/traderbhai/QuickPLS/releases/tag/v2.51.0).

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
