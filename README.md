# QuickPLS

QuickPLS is a free, proprietary, Windows desktop application for researchers working with PLS-SEM, SEM diagrams, reproducible analysis workflows, results interpretation, and publication-ready exports. Its analytical workflows require no internet connection, account, or cloud service. The QuickPLS application and page make no external requests; the Microsoft-managed WebView2 runtime can still make its own background service connections unless a separately verified OS-level network boundary is applied.

Current development release: `v2.46.0`.

QuickPLS runs locally. It does not require an account, activation server, cloud storage, R, Python, or remote computation at runtime, and QuickPLS product telemetry is disabled. This functional-offline scope is not a claim that the complete WebView2 process tree has zero egress or no platform-runtime telemetry.

> QuickPLS is independently implemented from published methods and permitted documentation. It does not import SmartPLS project files, does not reverse-engineer SmartPLS, and does not claim identical results for undocumented SmartPLS behavior.

QuickPLS 2.46.0 is the evidence-backed baseline for the QuickPLS 3.0 competitor program. The program does not treat a method name or engine preview as product parity: each advertised capability must pass its own scientific, archive, native-workflow, export, packaged-release, and claim-scope gates before promotion.

## Download

Use the latest GitHub Release and download one of these Windows assets. For local development builds, the same files are created under `target/release/artifacts/`.

- `QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe` - installer.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_portable.exe` - portable executable.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_cli.exe` - command-line executable for batch recipes.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_checksums.txt` - SHA-256 hashes.

The installer is currently unsigned. Windows SmartScreen may show a warning until code signing is added and audited.

## Native Desktop Workbench

The default frontend is a compact Windows scientific workbench with one operating-system title bar, a Windows-style menu and context toolbar, a project tree, a central document surface, an optional properties pane, and a minimal status bar. It starts maximized with a DPI-safe 1280 x 720 restored size. The supported minimum window is 1024 x 700; optional panes collapse before the model canvas or result tables are compressed.

The primary workflow has four surfaces: Launcher/Project, Data, Model, and Results. Before a project is open, Launcher stays focused on New, Open, Sample, and Recent projects. After opening a project, the same surface becomes a compact Workspace Explorer for Data, named Models, and saved report aliases. Calculation, export, Method Details, Run Details, preferences, shortcuts, and About open only when requested instead of occupying permanent workspaces. Model and Results use one generic `Calculate` command. It opens a compact, searchable method browser instead of adding a ribbon or a separate menu item for every analysis.

The visual acceptance harness writes its current viewport evidence to [the native desktop screenshot folder](validation/results/screens/v247-native-desktop-acceptance/). Release evidence captured from the packaged Tauri application belongs in the same folder so native-window screenshots remain distinct from browser-harness captures.

## What Users Can Do

- Import raw, covariance, or correlation data with explicit matrix sample size and import-time missing markers.
- Inspect paged case data, edit truthful variable label/scale/range metadata, and review data quality, missing values, and import details without loading an entire native dataset into the grid at once. Page sizes are 50, 100, or 250 cases.
- Navigate the backend-owned immutable dataset history, activate an earlier or derived version, and search variables by name or label. Metadata edits and native Recode operations create fingerprinted descendants while retaining their source versions and lineage.
- Create a typed recoded variable in a raw dataset from the installed Windows app. Recode transforms the complete native dataset rather than the visible grid page, appends a derived column, and supports explicit handling for unmapped values; it is disabled in browser preview and read-only projects.
- Build SEM diagrams with latent constructs, indicators, paths, covariances, layout controls, keyboard editing, and result overlays. Under its stated requirements, a selected substantive path can receive one two-stage moderating effect; QuickPLS adds and protects the generated product construct plus the required moderator main-effect path. The Higher-Order Construct dialog can also author a reflective-reflective disjoint two-stage HOC from measured lower-order components to a measured outcome.
- Open release-qualified scoped Standard workflows directly, including bounded PLS, reflective PLSc point estimation and consistent bootstrapping, case-weighted WPLS, CCA, descriptive CTA-PLS, IPMA, Structural Path Randomization v1, PLSpredict/CVPAT v2, the coupled two-group MICOM/permutation-MGA v4 workflow, PCA, GSCA, regression, PROCESS, and NCA cells. When Experimental Labs is enabled, additional executable but incomplete assessment, segmentation, CB-SEM, and inference workflows appear only when their exact requirements match the current data, model, and settings. Calculate shows actionable incompatibilities before execution and displays only genuine native job progress and logs.
- Open completed results automatically, navigate only the diagrams and tables available for the selected run, and return to the same live model for editing. A PLS or Bootstrap run with a genuine indirect path adds a focused Mediation result group for direct, path-specific indirect, total indirect, total, and available bootstrap effect inference.
- Manage multiple named editable models from the Workspace Explorer, preserve each model's canvas presentation while switching, and save, rename, open, or remove report aliases without deleting their completed calculation results.
- Save and reopen native projects through canonical typed models, immutable recipes, and integrity-checked result envelopes. Workspace JSON retains positions and pane state but cannot override scientific model or result content.
- Export CSV, XLSX, HTML, reviewer-pack, and Print/PDF output from the focused Results export dialog; SVG is available only when the completed run has a graphical model result.
- Open Method Details, Run Details, preferences, and diagnostics on demand; only actionable blockers interrupt the primary workflow.

## Product Finalization Status

QuickPLS now separates scientific coverage from verification maturity. The option-level [Capability Registry V2](validation/capabilities/capability_registry_v2.json) is the product source of truth; legacy catalogue rows are generated compatibility projections and are not authoritative for individual settings.

- **Standard** shows only an exact option cell that has passed the full release evidence ladder and has either complete coverage or a nonempty, explicitly documented bounded scope. Scoped Standard remains partial coverage and never implies unrestricted SmartPLS parity.
- **Experimental Labs** is disabled by default. When enabled, it exposes executable but incomplete workflows with one concise warning and direct requirements in Method Details.
- **Legacy** keeps discontinued or historical analyses readable without advertising them in normal Calculate.
- Internal evidence states, source hashes, and promotion terminology are confined to validation reports and Run Details rather than repeated throughout the customer workflow.

The current active-parity baseline contains 43 SmartPLS catalogue rows plus two intentionally excluded legacy rows. No active row is yet classified as full SmartPLS parity. Twenty-six official rows and 29 exact option-cell registrations are nevertheless available as scoped Standard because their bounded claims have release-qualified evidence; remaining executable-but-incomplete cells stay in Experimental Labs. Scoped Standard never implies unrestricted SmartPLS parity.

The implemented product foundation now includes the option-cell registry, a universal `SemModelV4` contract, estimator-specific compiler foundations, schema-6 project migration, recipe-schema-4 execution receipts, canonical result/comparison/export contracts, deterministic data transformations, Method Details, Run Details, and a synchronized SEM parameter table. These foundations are being connected to the production workflows before existing evidence is regenerated.

The documented post-hoc PLS technical minimum-sample-size result is distinct from prospective Monte Carlo power analysis. Both exact v2 option cells are independently scoped Standard: the post-hoc result uses complete linked-bootstrap inference with typed unavailable states, while prospective v2 runs its bounded two-construct Monte Carlo design. Neither cell promotes or substitutes for the other, and historical prospective v1 results remain readable only under their original identity.

Substantial scientific work remains across full PLS inference and assessment, prediction and model selection, heterogeneity and segmentation, and general CB-SEM estimation, constraints, bootstrap, groups, invariance, model comparison, and moderation. The product is finalized only after every active option cell is complete, passes its full qualification specification, and succeeds through packaged Windows, accessibility, persistence, export, performance, and soak gates. The current build must not be described as final SmartPLS parity.

Primary documentation:

- [Method Compatibility](docs/METHOD_COMPATIBILITY.md)
- [Product Finalization Implementation Status](docs/PRODUCT_FINALIZATION_IMPLEMENTATION_STATUS.md)
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
- Native PDF/PNG file generation is not promoted; use SVG or the Windows Print/PDF workflow where documented.
- Installer signing is still pending.

## Quick Start

See [Quick Start](docs/QUICK_START.md).

Short version:

1. Install QuickPLS or launch the portable executable.
2. From the Launcher, create a project, open a saved project, or open the sample project.
3. Import a dataset and inspect its cases, variables, quality, and import details in `Data`; use Versions to reactivate an immutable dataset snapshot, search the variable list, create a native full-data Recode descendant, or configure explicit Group A/Group B values from an unassigned grouping variable.
4. After importing the first dataset, choose `New Model…` in the Data toolbar (or press `Ctrl+Shift+N`), name the model, and choose Create. QuickPLS opens `Model` automatically. Build or edit the path model by assigning variables, creating constructs and paths, and using the properties pane.
5. Use the generic `Calculate` command, or Data's `Analyze…` command for model-free methods. Standard lists only completed parity cells; during development, enable Experimental Labs to use an executable workflow that matches its stated requirements. Review its settings and any actionable blocker, then start the calculation.
6. After a successful native job, inspect the automatically opened `Results`; use `Edit Model` to return to the same live model.
7. Open `Export` from Results for CSV, XLSX, HTML, reviewer pack, or the Windows Print/PDF workflow. Export SVG only for a completed run with a graphical model result; group-only MGA results intentionally omit it.

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

Local versioned builds write setup, portable, CLI, and checksum files to `target/release/artifacts/`. GitHub release notes should point to the matching checksum file for the uploaded version.

## Documentation

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
