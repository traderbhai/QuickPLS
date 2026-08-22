# QuickPLS

QuickPLS is a free, proprietary, Windows desktop application for researchers working with PLS-SEM, SEM diagrams, reproducible analysis workflows, results interpretation, and publication-ready exports. Its analytical workflows require no internet connection, account, or cloud service. The QuickPLS application and page make no external requests; the Microsoft-managed WebView2 runtime can still make its own background service connections unless a separately verified OS-level network boundary is applied.

Current source version: **2.55.0**. Formal first diagnostic `20260822T142953Z` at source `2e3a23f` executed all 14 steps and passed 13; the sole failure was `frontend_typecheck`, where `src/data/v255NamedSemEvidenceFixtures.test.ts` reported TypeScript error `TS2339`. The same-script, same-suite formal final diagnostic `20260822T155928Z` at source `6aa92b7` passed 14/14, including 453/453 Vitest suites, 1697/1697 tests, all 17 rebaseline assertions, and zero captured console errors. Both formal records use runner SHA-256 `64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796`; they are separate evidence records and are not byte-identical. Renderer console/page-error evidence fails closed, attach-only phases use fresh wrapper-owned processes, and candidate/phase/trusted-driver PID, role, suite, and SHA-256 bindings are closed. This is source evidence, not packaged validation. Candidate/install/smoke attempts `20260822T145055Z`, `20260822T151222Z`, and every earlier 2.55 candidate, install, or smoke attempt are historical and ineligible. One new provenance-bound unsigned candidate build, isolated install, and full installed-and-portable smoke remain pending, followed by evidence collection, bundling, and publication. Exactly one named case—the actual Windows 200% scaling case—may use the opt-in owner waiver; its real DPI screenshot and receipt remain mandatory, its status remains `waived` rather than `passed`, and the other 54 named cases must pass. If an existing registered QuickPLS installation must be removed, only its exact registered uninstaller may be used; project files, recovery data, and QuickPLS application user data must remain untouched. Portable evidence does not replace installed evidence. Latest published public pre-release: [`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). Code signing is excluded; method labels such as **Standard** describe bounded analytical capability, not installer signing state.

QuickPLS runs locally. It does not require an account, activation server, cloud storage, R, Python, or remote computation at runtime, and QuickPLS product telemetry is disabled. This functional-offline scope is not a claim that the complete WebView2 process tree has zero egress or no platform-runtime telemetry.

> QuickPLS is independently implemented from published methods and permitted documentation. It does not import SmartPLS project files, does not reverse-engineer SmartPLS, and does not claim identical results for undocumented SmartPLS behavior.

QuickPLS 2.51.0 places the Rank 0–3 SEM capabilities behind one `Canvas → Calculate → Results` workflow. The existing 18-method Calculate catalogue is unchanged: model features and selected inference options route to the appropriate bounded engine internally. The verified unsigned Windows preview and checksums are published on GitHub.

The verified 2.52 unsigned Windows candidate makes higher-order constructs diagram-native: select eligible constructs, open **Higher-Order Construct…** from the Model or context menu, and edit the result from the diagram or Properties. It also presents PLS model fit as descriptive information, with exact-fit availability shown as one compact state instead of a permanent warning.

QuickPLS 2.53.0 workflow implementation and its three bounded exact-cell
qualifications are complete. Mediation stays diagram-derived from ordinary
paths. Moderation is attached to the focal path through a compact visual anchor
and dashed moderator connector, with drag, context-menu, and `M`-key entry
points. The new single-path mediation-bootstrap and true three-way moderation
point/bootstrap cells are independently scoped Standard; the existing qualified
two-way, multiple-mediation, and moderated-mediation cells keep their exact
identities and states. The public Calculate catalogue remains exactly 18 methods.
The unsigned Windows candidate and its packaged moderation create → calculate →
Results → save/fresh-reopen journey passed, and the unsigned assets remain
available from the [`v2.53.0` release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.53.0).

QuickPLS 2.54.0 refines Canvas editing, model navigation, arrangement, and
researcher-facing Results without changing the 18-method catalogue or any
numerical engine, estimand, Registry cell, or stored result identity. The latest
consolidated source diagnostic recorded 8/9 passing steps, followed by a 69/69
focused remediation pass. The final unsigned Windows candidate then passed the
isolated 10/10 create → calculate → Results → save → fresh-reopen packaged
journey with zero application-page external requests and zero console errors.
The release-artifact package and SHA-256 checksum verification also passed;
code signing is excluded.

QuickPLS 2.55.0 source adds model-aware diagram routing and editable presentation
routes, a full read-only PROCESS diagram preview, responsive shared Calculate
dialogs, scientific-first eligibility messages, and truthful fixed-setting
summaries for the existing 18 public methods. Its rebaselined interaction suite
and evidence contracts cover all 18 setup surfaces and 64 declared setup cases,
and require 29 cross-method journeys and 26 specialized journeys for the
candidate evidence package. Formal first diagnostic `20260822T142953Z` at source
`2e3a23f` executed all 14 steps and passed 13; the sole failure was
`frontend_typecheck`, where `src/data/v255NamedSemEvidenceFixtures.test.ts`
reported TypeScript error `TS2339`. The same-script, same-suite formal final
diagnostic `20260822T155928Z` at source `6aa92b7` passed 14/14,
including 453/453 Vitest suites, 1697/1697 tests, all 17 rebaseline assertions,
and zero captured console errors. Both formal records used runner SHA-256
`64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796`.
The formal reports are separate evidence records and are not byte-identical.
The final pass includes full Vitest, focused Rust authority/archive lifecycle
tests, full TypeScript checking, a production frontend build, six-format
semantic export readback, the 17-item regression rebaseline, the 18-method setup
crawl, and the final evidence contract. The trust-chain remediation fails closed
on renderer console/page errors, enforces fresh wrapper-owned attach-only phase
processes, and binds candidate, phase, and trusted-driver process roles and
hashes. Exactly one opt-in waiver applies only to actual Windows 200% scaling;
the observed-DPI screenshot and receipt remain required and the other 54 cases
must pass. A fresh unsigned setup, portable, CLI, and checksum package,
installed and portable journeys, the evidence bundle, and publication all
remain pending; the latest downloadable release therefore remains 2.54.0.

Candidate/install/smoke attempts `20260822T145055Z`, `20260822T151222Z`, and
every earlier 2.55 candidate, install, or smoke attempt are historical and
ineligible.
Exact typed post-hoc authority now takes precedence over the generic bootstrap
fallback, but it still requires fresh packaged validation. The install comparison
accepts only Tauri's exact three-byte `UNK` → `NSS` marker transition and rejects
any other byte difference. One new provenance-bound unsigned candidate, isolated
install, and full installed-and-portable smoke are still required.

## Download

Use the assets from the [QuickPLS 2.54.0 release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). For local development builds, versioned files are created under `target/release/artifacts/`.

- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_setup.exe` - installer, recommended for normal use.
- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_portable.exe` - portable executable.
- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_cli.exe` - command-line executable for batch recipes.
- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_checksums.txt` - SHA-256 hashes.

The installer is currently unsigned. Windows SmartScreen may show a warning until code signing is added and audited.

## Native Desktop Workbench

The default frontend is a compact Windows scientific workbench with one operating-system title bar, a Windows-style menu and context toolbar, a project tree, a central document surface, an optional properties pane, and a minimal status bar. It starts maximized with a DPI-safe 1280 x 720 restored size. The supported minimum window is 1024 x 700; optional panes collapse before the model canvas or result tables are compressed.

The primary workflow has four surfaces: Launcher/Project, Data, Model, and Results. Before a project is open, Launcher stays focused on New, Open, Sample, and Recent projects. After opening a project, the same surface becomes a compact Workspace Explorer for Data, named Models, and saved report aliases. Canvas is the only permanent model-authoring document. Calculation, the Advanced Parameter Table, export, Method Details, Run Details, preferences, shortcuts, and About open only when requested instead of occupying permanent workspaces. Model and Results use one generic `Calculate` command, which opens the existing compact, searchable 18-method browser.

The visual acceptance harness writes its current viewport evidence to [the native desktop screenshot folder](validation/results/screens/v247-native-desktop-acceptance/). Release evidence captured from the packaged Tauri application belongs in the same folder so native-window screenshots remain distinct from browser-harness captures.

## What Users Can Do

- Import raw, covariance, or correlation data with explicit matrix sample size and import-time missing markers.
- Inspect paged case data, edit truthful variable label/scale/range metadata, and review data quality, missing values, and import details without loading an entire native dataset into the grid at once. Page sizes are 50, 100, or 250 cases.
- Navigate the backend-owned immutable dataset history, activate an earlier or derived version, and search variables by name or label. Metadata edits and native Recode operations create fingerprinted descendants while retaining their source versions and lineage.
- Create a typed recoded variable in a raw dataset from the installed Windows app. Recode transforms the complete native dataset rather than the visible grid page, appends a derived column, and supports explicit handling for unmapped values; it is disabled in browser preview and read-only projects.
- Build SEM diagrams with latent constructs, indicators, paths, covariances, layout controls, keyboard editing, and result overlays. Select eligible constructs and use the Model or context menu to create a higher-order construct; its RR/RF/FR/FF type and recommended construction approach are derived from the selected dimensions and topology. Mediation is detected from substantive drawn paths. Moderation is authored against the focal path with a compact anchor and dashed connector, while bounded moderated mediation is selected in PLS Bootstrapping. Generated interaction terms stay hidden in the ordinary diagram and remain available in Expert/Diagnostics.
- Use the same Canvas and `Calculate` workflow for scoped-Standard PLS-SEM and CB-SEM estimators. PLS Algorithm, Bootstrapping, and CB-SEM use the resident model topology to reach the qualified mediation, simultaneous moderation, higher-order PLS, moderated-mediation, or recursive common-factor CB-SEM cell when applicable. Internal compatibility adapters do not appear as separate General SEM or Exact CB-SEM tabs.
- Open completed results automatically, navigate only the diagrams and tables available for the selected run, and return to the same live model for editing. A PLS or Bootstrap run with a genuine indirect path adds a focused Mediation result group for direct, path-specific indirect, total indirect, total, and available bootstrap effect inference. PLS model fit appears as **Model fit — descriptive** with neutral interpretation details and a compact exact-fit state.
- Manage multiple named editable models from the Workspace Explorer, preserve each model's canvas presentation while switching, and save, rename, open, or remove report aliases without deleting their completed calculation results.
- Save and reopen native projects through canonical typed models, immutable recipes, and integrity-checked result envelopes. Workspace JSON retains positions and pane state but cannot override scientific model or result content.
- Export from the focused Results action. Canonical General SEM results support CSV, XLSX, self-contained HTML, PDF, SVG, and PNG from one provenance-bound result document; other result families expose only the formats they own, including reviewer-pack and Windows Print/PDF paths where documented.
- Open Method Details, Run Details, preferences, and diagnostics on demand; only actionable blockers interrupt the primary workflow.

## Product Finalization Status

QuickPLS now separates scientific coverage from verification maturity. The option-level [Capability Registry V2](validation/capabilities/capability_registry_v2.json) is the product source of truth; legacy catalogue rows are generated compatibility projections and are not authoritative for individual settings.

- **Standard** shows only an exact option cell that has passed the full release evidence ladder and has either complete coverage or a nonempty, explicitly documented bounded scope. Scoped Standard remains partial coverage and never implies unrestricted SmartPLS parity.
- **Experimental Labs** is disabled by default. When enabled, it exposes executable but incomplete workflows with one concise warning and direct requirements in Method Details.
- **Legacy** keeps discontinued or historical analyses readable without advertising them in normal Calculate.
- Internal evidence states, source hashes, and promotion terminology are confined to validation reports and Run Details rather than repeated throughout the customer workflow.

The current active-parity baseline contains 43 SmartPLS catalogue rows plus two intentionally excluded legacy rows. No active row is classified as unrestricted SmartPLS parity. The Registry contains 41 scoped-Standard exact cells; its conservative compatibility projection is 27 Standard rows, 16 Labs rows, and two Legacy rows. This row-level projection does not demote any previously qualified exact cell. Version 2.50 promoted the bounded Rank 2 moderated-mediation cell and both bounded Rank 3 CB-SEM cells under the streamlined integration profile. Versions 2.51–2.55 change user workflow and presentation or add separately versioned cells without relabelling historical scientific identities. Scoped Standard never implies unrestricted SmartPLS parity.

The implemented product foundation includes the option-cell registry, universal `SemModelV4` authority, estimator-specific compiled plans, schema-6 project migration, recipe-schema-4 execution receipts, canonical result/comparison/export contracts, deterministic data transformations, Method Details, Run Details, and a synchronized SEM parameter table. The Rank 0–3 Version 2.50 workflows are connected to the production Canvas, calculation, cancellation, Results, export, append, and strict reopen paths.

The documented post-hoc PLS technical minimum-sample-size result is distinct from prospective Monte Carlo power analysis. Both exact v2 option cells are independently scoped Standard: the post-hoc result uses complete linked-bootstrap inference with typed unavailable states, while prospective v2 runs its bounded two-construct Monte Carlo design. Neither cell promotes or substitutes for the other, and historical prospective v1 results remain readable only under their original identity.

Substantial scientific work remains across full PLS inference and assessment, prediction and model selection, heterogeneity and segmentation, and general CB-SEM estimation, constraints, bootstrap, groups, invariance, model comparison, and moderation. The product is finalized only after every active option cell is complete, passes its full qualification specification, and succeeds through packaged Windows, accessibility, persistence, export, performance, and soak gates. The current build must not be described as final SmartPLS parity.

Primary documentation:

- [QuickPLS 2.55.0 Release Notes — final source gate complete; fresh candidate pending](docs/RELEASE_NOTES_V2_55_0.md)
- [QuickPLS 2.54.0 Release Notes — verified unsigned candidate](docs/RELEASE_NOTES_V2_54_0.md)
- [QuickPLS 2.53.0 Release Notes — verified unsigned candidate](docs/RELEASE_NOTES_V2_53_0.md)
- [QuickPLS 2.52.0 Release Notes](docs/RELEASE_NOTES_V2_52_0.md)
- [QuickPLS 2.51.0 Release Notes](docs/RELEASE_NOTES_V2_51_0.md)
- [QuickPLS 2.50.0 Release Notes](docs/RELEASE_NOTES_V2_50_0.md)
- [Rank 0–3 SEM Upgrade Status](docs/SEM_UPGRADE_RANKS_0_3_STATUS.md)
- [Method Compatibility](docs/METHOD_COMPATIBILITY.md)
- [Historical Product Finalization Snapshot](docs/PRODUCT_FINALIZATION_IMPLEMENTATION_STATUS.md)
- [Capability Registry V2](docs/CAPABILITY_REGISTRY_V2.md)
- [QuickPLS 3.0 Competitor Program](docs/QUICKPLS_3_COMPETITOR_PROGRAM.md)
- [Method Promotion Factory](docs/METHOD_PROMOTION_FACTORY.md)
- [Commercial Readiness](docs/QUICKPLS_3_COMMERCIAL_READINESS.md)
- [Supported v1 Scope](docs/V1_SUPPORTED_SCOPE.md)
- [Known Differences](docs/V1_KNOWN_DIFFERENCES.md)
- [Methodology Manual](docs/METHODOLOGY_MANUAL_V1_0.md)
- [Validation Artifact Index](docs/VALIDATION_ARTIFACT_INDEX_V1_0.md)
- [Native Desktop Redesign](docs/NATIVE_DESKTOP_REDESIGN.md)
- [QuickPLS 2.9 Acceptance Backlog](docs/V2_9_0_ACCEPTANCE_BACKLOG_AND_NEXT_PASS.md)

Important limits:

- No SmartPLS project import.
- No guaranteed identity with SmartPLS outputs.
- No reproduction of undocumented SmartPLS behavior.
- No runtime dependency on R, Rscript, Python, lavaan, cSEM, seminr, plspm, NumPy, or validation tooling.
- PDF/PNG publication is available for canonical General SEM results; older result families may expose SVG or Windows Print/PDF instead.
- Code signing is excluded; the installer and portable executable are unsigned.

## Quick Start

See [Quick Start](docs/QUICK_START.md).

Short version:

1. Install QuickPLS or launch the portable executable.
2. From the Launcher, create a project, open a saved project, or open the sample project.
3. Import a dataset and inspect its cases, variables, quality, and import details in `Data`; use Versions to reactivate an immutable dataset snapshot, search the variable list, create a native full-data Recode descendant, or configure explicit Group A/Group B values from an unassigned grouping variable.
4. After importing the first dataset, choose `New Model…` in the Data toolbar (or press `Ctrl+Shift+N`), name the model, and choose Create. QuickPLS opens `Model` automatically. Build or edit the path model by assigning variables, creating constructs and paths, and using the properties pane.
5. Use the generic `Calculate` command, or Data's `Analyze…` command for model-free methods. Choose from the same 18-method catalogue; QuickPLS detects mediation, moderation, higher-order constructs, moderated mediation, and common-factor CB-SEM requirements from the resident model. Open the Advanced Parameter Table from Canvas or CB-SEM setup only when parameter-level control is needed.
6. After a successful native job, inspect the automatically opened `Results`; use `Edit Model` to return to the same live model.
7. Open `Export` from Results. Canonical General SEM results offer CSV, XLSX, HTML, PDF, SVG, and PNG; other result types show only their compatible table, report, or diagram formats.

## Build From Source

QuickPLS source is available for inspection and contribution under the proprietary license in [LICENSE.md](LICENSE.md). It is not open-source software.

See [Build From Source](docs/BUILD_FROM_SOURCE.md).

Basic commands:

```powershell
npm install
npm test -- --run
npm run build
cargo check --workspace
cargo test --workspace
npm run tauri -- build
```

Versioned release artifacts are written to:

```text
target/release/artifacts/
```

## Verify The Native Redesign

```powershell
npm run qpls:native:redesign-gate
cargo check --workspace
cargo test --workspace
npm run qpls:promotion:pls-core
npm run qpls:promotion:assessment
npm run qpls:promotion:inference
npm run qpls:plsc:validate
npm run qpls:promotion:wpls
npm run qpls:cca:reference
npm run qpls:promotion:cca
npm run tauri -- build
```

The combined frontend gate runs the production build, Vitest suite, and native-shell visual acceptance harness. To run those checks separately:

```powershell
npm test -- --run
npm run build
npm run qpls:v247:native-desktop-visual
```

See [Native Desktop Redesign](docs/NATIVE_DESKTOP_REDESIGN.md) for the manual packaged-app workflow, keyboard checks, viewport matrix, truthfulness checks, and screenshot evidence requirements, including Recode, Consistent PLS, Weighted PLS, CCA composite residual diagnostics, and PLSpredict / CVPAT. Browser fixtures verify catalog and setup contracts without fabricating completed native jobs. Current-method promotion additionally requires a genuine packaged lifecycle, native XLSX export, strict archive validation, and same-run reopen proof. The redesign and packaged screenshot set are not yet final SmartPLS-class parity evidence.

## Release Files And Checksums

Local versioned builds write setup, portable, CLI, and checksum files to `target/release/artifacts/`. Candidate/install/smoke attempts `20260822T145055Z`, `20260822T151222Z`, and every earlier local Version 2.55.0 candidate, install, or smoke attempt are historical and ineligible. The marker-only install comparison is corrected but has no eligible packaged proof. One new provenance-bound unsigned candidate must still be built and complete the required isolated install plus full installed-and-portable smoke before evidence collection, bundling, and publication. The latest published files remain the verified Version 2.54.0 setup, portable app, CLI, and checksums on the [`v2.54.0` release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0).

## Documentation

- [Version 2.55 Release Notes — final source gate complete; fresh candidate pending](docs/RELEASE_NOTES_V2_55_0.md)
- [Version 2.54 Release Notes — verified unsigned candidate](docs/RELEASE_NOTES_V2_54_0.md)
- [Version 2.53 Release Notes — verified unsigned candidate](docs/RELEASE_NOTES_V2_53_0.md)
- [Version 2.52 Release Notes](docs/RELEASE_NOTES_V2_52_0.md)
- [Version 2.51 Release Notes](docs/RELEASE_NOTES_V2_51_0.md)
- [Version 2.50 Release Notes](docs/RELEASE_NOTES_V2_50_0.md)
- [Changelog](CHANGELOG.md)
- [Installation](docs/INSTALLATION.md)
- [Quick Start](docs/QUICK_START.md)
- [User Guide](docs/USER_GUIDE.md)
- [Build From Source](docs/BUILD_FROM_SOURCE.md)
- [First PLS Model Tutorial](docs/FIRST_PLS_MODEL_TUTORIAL.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [FAQ](docs/FAQ.md)
- [Previous Milestone Notes v2.45.0](docs/V2_45_0_MOCKUP_VISIBLE_FEATURE_COMPLETION.md)
- [Dependency Notices](docs/DEPENDENCY_NOTICES.md)

## License

QuickPLS is free to use but proprietary. See [LICENSE.md](LICENSE.md) and [EULA.md](EULA.md).

Unless a separate written agreement says otherwise, you may use QuickPLS as an end user, inspect the source in this repository, and submit issues or proposed changes, but you may not sell, sublicense, redistribute modified builds, remove notices, or present derived software as QuickPLS.

## Security

Please report security issues using [SECURITY.md](SECURITY.md). Do not disclose suspected vulnerabilities publicly before they are reviewed.
