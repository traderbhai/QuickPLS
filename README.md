# QuickPLS

QuickPLS is a free, proprietary, fully offline Windows desktop application for researchers working with PLS-SEM, SEM diagrams, validated method workflows, results interpretation, and publication-ready exports.

Current development release: `v2.46.0`.

QuickPLS runs locally. It does not require an account, activation server, telemetry, cloud storage, R, Python, or remote computation at runtime.

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

The primary workflow has four surfaces: Launcher/Project, Data, Model, and Results. Before a project is open, Launcher stays focused on New, Open, Sample, and Recent projects. After opening a project, the same surface becomes a compact Workspace Explorer for Data, named Models, and saved report aliases. Calculation, export, method scope, run details, preferences, shortcuts, and About open only when requested instead of occupying permanent workspaces. Model and Results use one generic `Calculate` command. It opens a compact, searchable method browser instead of adding a ribbon or a separate menu item for every analysis.

The visual acceptance harness writes its current viewport evidence to [the native desktop screenshot folder](validation/results/screens/v247-native-desktop-acceptance/). Release evidence captured from the packaged Tauri application belongs in the same folder so native-window screenshots remain distinct from browser-harness captures.

## What Users Can Do

- Import raw, covariance, or correlation data with explicit matrix sample size and import-time missing markers.
- Inspect paged case data, edit truthful variable label/scale/range metadata, and review data quality, missing values, and import details without loading an entire native dataset into the grid at once. Page sizes are 50, 100, or 250 cases.
- Navigate the backend-owned immutable dataset history, activate an earlier or derived version, and search variables by name or label. Metadata edits and native Recode operations create fingerprinted descendants while retaining their source versions and lineage.
- Create a typed recoded variable in a raw dataset from the installed Windows app. Recode transforms the complete native dataset rather than the visible grid page, appends a derived column, and supports explicit handling for unmapped values; it is disabled in browser preview and read-only projects.
- Build SEM diagrams with latent constructs, indicators, paths, covariances, layout controls, keyboard editing, and result overlays. A selected substantive path can receive one promoted two-stage moderating effect; QuickPLS adds and protects the generated product construct plus the required moderator main-effect path. The native Higher-Order Construct dialog also authors one reflective-reflective disjoint two-stage HOC from at least two measured, measurement-only lower-order components, followed by one HOC-to-measured-outcome path.
- Open PLS Algorithm, Consistent PLS, Weighted PLS, bounded GSCA ALS, CCA composite residual diagnostics, Importance-Performance Map Analysis, bounded CB-SEM / CFA, Bootstrapping, Structural Path Randomization, MICOM and Two-Group Permutation MGA, PLSpredict / CVPAT, Necessary Condition Analysis, Principal Component Analysis, or Ordinary Least Squares Regression from `Calculate` or `Analyze`, validate the applicable data and model, and monitor only genuine native job progress and logs.
- Open completed results automatically, navigate only the diagrams and tables available for the selected run, and return to the same live model for editing. A PLS or Bootstrap run with a genuine indirect path adds a focused Mediation result group for direct, path-specific indirect, total indirect, total, and available bootstrap effect inference.
- Manage multiple named editable models from the Workspace Explorer, preserve each model's canvas presentation while switching, and save, rename, open, or remove report aliases without deleting their validated calculation results.
- Save and reopen native projects through canonical typed models, immutable recipes, and validated result envelopes. Workspace JSON retains positions and pane state but cannot override scientific model or result content.
- Export CSV, XLSX, HTML, reviewer-pack, and Print/PDF output from the focused Results export dialog; SVG is available only when the completed run has a graphical model result.
- Open method scope, provenance, run details, preferences, and diagnostics on demand; only actionable blockers interrupt the primary workflow.

## Current Scope

QuickPLS is validated only for documented scopes. The UI marks methods and shapes as `Validated scope`, `Experimental`, `Unsupported`, or `Not applicable` based on the current dataset, model, and settings.

The completed primary native calculation slice currently covers fourteen workflows: PLS Algorithm, Consistent PLS, Weighted PLS, bounded GSCA ALS, CCA composite residual diagnostics, Importance-Performance Map Analysis, bounded CB-SEM / CFA, Bootstrapping, Structural Path Randomization, MICOM and Two-Group Permutation MGA, PLSpredict / CVPAT, Necessary Condition Analysis, Principal Component Analysis, and Ordinary Least Squares Regression. Structural Path Randomization is the candidate/experimental single-model Freedman-Lane path-inference routine; it is not a predefined-group comparison. Its raw pathwise probabilities condition on fixed original PLS scores, assume exchangeable reduced-model residuals, and are unadjusted for multiplicity; current calibration covers homoscedastic Gaussian errors only. The joint MICOM/MGA workflow uses an explicit grouping variable and ordered Group A/Group B values, fits path-weighted standardized PLS independently to at least ten complete model cases per selected group, and requires 5,000–10,000 usable deterministic label permutations. It reports group paths, R², outer loadings, outer weights, A-minus-B differences, two-tailed permutation inference, and MICOM v2 Steps 1–3. Step 1 is an explicit researcher confirmation; Steps 2 and 3 use group-specific re-estimation and pooled-score permutation distributions. Other group values and incomplete cases are excluded and disclosed. Parametric/bootstrap MGA, one-tailed tests, interactions, higher-order constructs, case weights, and claims beyond the documented MICOM v2/permutation-MGA v2 scope remain unsupported. Consistent PLS is limited to reflective path/factor-weighted models with at least two indicators per construct; it uses the current method-versioned Dijkstra-Henseler rho_A correction and rejects inadmissible corrected correlation matrices instead of silently clamping them. Weighted PLS is limited to reflective standardized models and validates a positive finite numeric case-weight variable across the complete native dataset. PLSpredict / CVPAT v2 predicts reflective endogenous indicators with train-only preprocessing, a fixed seeded balanced 10-fold plan repeated 10 times, indicator-average and linear-model benchmarks, Q²_predict, RMSE/MAE/MAPE, and one-sided 95% aggregate benchmark tests. Construct scores are supplementary and the deterministic modulo-4 holdout is retained as a secondary diagnostic. The fixed folds are independently implemented rather than SmartPLS-randomized, and this workflow does not compare separately saved models. Historical `plspredict_holdout_v1` results remain readable under an explicit legacy label and are not relabeled as current indicator-level evidence.

The calculation catalog also contains a bounded CCA composite residual diagnostic workflow. It accepts only standardized, listwise, reflective recursive models with path or factor weighting, at least two constructs, and at least one structural path. It reports observed-versus-reproduced composite-correlation residuals and their maximum absolute value; it does not calculate thresholds, pass/fail classifications, p values, confidence intervals, or resampled inference. Despite the historical `CCA` identifier, this is not full SmartPLS CCA. Independent reference, strict persistence, browser setup, and fresh packaged-Tauri execution/export/save/reopen acceptance now pass for this bounded scope.

The bounded Importance-Performance Map Analysis workflow selects one endogenous target and reports only its direct and indirect structural predecessors. Importance uses total PLS effects; performance uses observed-range 0-100 scaling of listwise-standardized construct and indicator scores. A fresh packaged-Tauri run proves that the target and a disconnected negative-control branch are excluded from Results and XLSX output, and that the same run survives explicit save and reopen. Theoretical-range correction, alternative SmartPLS representations, NCA integration, resampling inference, and cIPMA remain unsupported.

The bounded CB-SEM / CFA workflow fits single-group reflective raw-data models by maximum likelihood with first-loading marker identification. It supports measurement-only CFA and recursive structural SEM, reports standardized and unstandardized parameters, fit indices, residual matrices, and residual-based modification screening, and exports the same tables with provenance. Fresh packaged acceptance proves a 240-case three-factor recursive SEM completes, exports to native XLSX, saves its strict typed payload, and reopens the same run. Mean structures, robust or ordinal estimators, FIML, bootstrap, multigroup/invariance, controls, interactions, and higher-order constructs remain unsupported in this workflow.

Mediation, two-stage moderation, and the bounded higher-order result group are outputs of the existing PLS workflow, not separate calculation methods. QuickPLS reports genuine structural-path products and aggregate engine effects, while bootstrap inference is available only for effect identities actually emitted by the resampler. The promoted moderation scope supports one two-way interaction on an existing substantive path, standardized path-weighted PLS, listwise deletion, no case weights, and conditional effects at moderator scores -1, 0, and +1. The packaged native HOC scope is point-estimate-only: it reports component loadings/weights, the HOC structural path, and the exact two-stage calculation scope without exposing generated pseudo-indicator names. Repeated-indicator and hybrid HOCs remain backend/reference capabilities rather than native authoring options; HOC resampling, multiple HOCs, higher-order moderation, and broader HOC systems remain outside the native scope.

The bounded Necessary Condition Analysis workflow accepts one observed numeric condition/outcome pair, reports CE-FDH and CR-FDH ceilings, seeded permutation evidence, and observed-range bottlenecks, and does not create a phantom SEM model. Multiple conditions or outcomes, latent-score NCA, theoretical ranges, categorical encoding, alternate ceilings, and cIPMA integration remain unsupported.

The bounded standalone Principal Component Analysis workflow analyzes 2 to 50 selected numeric raw-data variables from their listwise standardized correlation matrix, supports Kaiser, fixed-count, and cumulative-variance retention, and reports deterministic eigenvalues, explained variance, loadings, weights, and complete-case scores. It does not create a phantom SEM model; full scores are generated only for export. Rotation, pairwise deletion, matrix-only input, categorical conversion, and inference remain unsupported.

The bounded standalone Ordinary Least Squares Regression workflow analyzes one numeric outcome against selected numeric predictors and optional controls using unstandardized values, an intercept, listwise deletion, HC3 robust standard errors, and fixed two-sided 95% intervals. It reports coefficients, fit, fitted values, residuals, and exact scope without creating a phantom SEM model. Automatic categorical encoding, weights, clustered errors, HC0/HC4 claims, GLS, mixed models, and panel models are outside this workflow.

The current standalone Binary Logistic Regression workflow emits `regression_logistic_v2` for an exactly coded numeric 0/1 outcome with both classes represented in the listwise-complete sample. It uses deterministic single-worker Newton IRLS, reports Wald inference, odds ratios and their intervals, likelihood diagnostics, fitted probabilities, convergence details, and explicitly in-sample descriptive classification. Independent Python and validation-only R `glm` comparisons, strict archive validation, three browser viewports, real packaged XLSX export, explicit save, and same-run reopen are accepted. Multinomial, ordinal, weighted, clustered, Firth-corrected, and out-of-sample classification claims remain unsupported; `regression_logistic_v1` is historical archive output only.

The bounded Regression Bootstrapping workflow emits `regression_bootstrap_v1` for supported OLS and binary-logistic coefficient inference. It uses deterministic indexed case resampling, Type-7 percentile-primary intervals, conditional BCa intervals, and a documented standard-normal bootstrap-ratio test. Independent Python/R reference checks, exact boundary and tamper tests, three browser viewports, genuine packaged 10,000-resample OLS and logistic runs, cancellation, native XLSX, explicit save, and same-run reopen are accepted. Studentized intervals, custom tails or alpha levels, weights, multinomial outcomes, and ordinal outcomes remain excluded.

The bounded Graph-defined Path Analysis and PROCESS workflow emits `regression_process_v2`. It independently authors continuous-outcome OLS equation graphs with mediation, continuous or exact-0/1 moderation, bounded moderated mediation, conditional effects, HC3/Student-t simple slopes and Johnson-Neyman regions, and optional dedicated percentile/conditional-BCa case bootstrap. Independent Python/R reference checks, exact scientific and archive-tamper boundaries, responsive setup, genuine packaged 10,000-resample execution, cancellation/retry, accessible result and plot-data tables, native XLSX, explicit save/reopen, repeated completion identity, and clean process/resource lifecycle evidence pass. The coordinated public 2.46.0 Wave 1 release packages this qualified capability while retaining conservative candidate/experimental presentation until the full QuickPLS 3.0 parity gate.

SmartPLS-class breadth is not yet complete. Native workflow and result coverage for CTA-PLS, endogeneity, nonlinear effects, moderated mediation, and FIMIX remains a redesign backlog even where a bounded numerical engine already exists. The bounded GSCA workflow now uses independently checked joint global least-squares ALS (`gsca_als_v2`) with native results, XLSX export, strict persistence, and same-run reopen evidence. Historical `gsca_v1`, `micom_v1`, `regression_logistic_v1`, and `regression_process_v1` payloads remain legacy-only and are never reinterpreted as current evidence. The current release must not be described as final workflow or feature parity.

Primary documentation:

- [Method Compatibility](docs/METHOD_COMPATIBILITY.md)
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
5. Use the generic `Calculate` command, or Data's `Analyze…` command for model-free methods, search or select one of the fourteen available workflows, review its settings and any actionable blocker, then start the calculation.
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
