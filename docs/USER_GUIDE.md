# QuickPLS User Guide

This guide describes the Version 2.55 source workflow.
QuickPLS runs analyses locally without an account, cloud service, R, or Python
at runtime. Formal first diagnostic `20260822T142953Z` at source `2e3a23f`
executed all 14 steps and passed 13; the sole failure was `frontend_typecheck`,
where `src/data/v255NamedSemEvidenceFixtures.test.ts` reported TypeScript error
`TS2339`. The same-script, same-suite formal final diagnostic
`20260822T183205Z` at source `0fa74eb` passed 14/14, including 453/453 Vitest
suites, 1702/1702 tests, 17/17 rebaseline assertions, and zero captured console
errors. Both formal records use runner
SHA-256 `64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796`;
the reports are separate evidence records and are not byte-identical.
A new provenance-bound unsigned 2.55 candidate build, isolated install, and
full installed-and-portable smoke remain pending, followed by evidence
collection, bundling, and publication. The verified `v2.54.0` release remains
available; code signing is excluded.

Candidate/install/smoke/probe attempts `20260822T163918Z`, `20260822T170818Z`,
`20260822T173336Z`, `20260822T174301Z`, `20260822T175510Z`, and every earlier
2.55 candidate, install, smoke, or probe attempt are historical and ineligible.
The install wrapper accepts only Tauri's exact
three-byte `UNK` → `NSS` marker transition and rejects every other byte
difference. One new candidate, isolated install, and full smoke are still required.
Exactly one case—the actual Windows 200% scaling case—may use the opt-in waiver;
its real DPI screenshot and receipt remain required, its status remains
`waived`, and the other 54 named cases must pass. If an existing registered
installation must be removed for the isolated install, only its exact registered
uninstaller may be used and project files, recovery data, and QuickPLS
application user data must remain untouched. Installed and portable journeys are
both mandatory; neither substitutes for the other.

## Projects and scientific authority

A `.qpls` project stores immutable dataset versions, SEM models, recipes, completed results, report settings, layout metadata, and provenance. A completed result remains bound to its dataset, model, settings, seed, method version, capability cell, and engine identity.

Advanced SEM calculations use `SemModelV4` as the model authority, `AnalysisRecipeV4` as the requested analysis, compiled plans as execution authority, `CanonicalResultDocumentV2` as result authority, and schema-6 as persistence authority. These are internal contracts; users work through Canvas, Calculate, and Results. Canvas layout cannot override scientific content.

## Launcher and Data

Use Launcher to create or open a project, open a sample, or continue from recovery. Use Data to import and inspect raw or supported matrix data, review missing values and quality, search variables, edit metadata, and create immutable derived dataset versions.

## Canvas and Advanced Parameter Table

Use Model to create constructs, assign indicators, draw paths and supported covariances, arrange the diagram, and validate the model. Layout changes do not alter analytical fingerprints.

The model navigator separates Indicators, Constructs, and Relationships. Use a
navigator row to focus its model element. Canvas arrangement includes tidy
selection, alignment, distribution, and whole-model direction, while Fit can
target the structure, all diagram content, or the current selection. Indicator
side/free placement, pins, path routes, and label offsets remain presentation
metadata and survive unrelated scientific edits.

The Parameter Table is no longer a permanent workspace tab. Open the resizable **Advanced Parameter Table** from Canvas, CB-SEM setup, or a corrective preflight action. Construct representation and the active table define the model sent to native preflight and estimation. Supported fixed/free rows, equality labels, and row bounds are preserved; unsupported constraint objects remain visible and block calculation instead of being discarded.

## Advanced model authoring

The same Canvas supports bounded advanced authoring:

- **Mediation:** draw the component paths. QuickPLS detects eligible indirect paths automatically from substantive directed structural relationships. Covariance, control, measurement, generated, and interaction-hierarchy relations do not count.
- **Moderation:** drag a moderator onto an eligible structural path, use the path context menu, or select the path and press `M`. QuickPLS shows a compact `×` anchor and dashed moderator connector. Enter edits the selected effect; Delete removes only that effect. The anchor is presentation-only and never becomes a persisted scientific relationship.
- **Three-way moderation:** add a second moderator to an eligible parent two-way interaction. The bounded source cell requires one three-way term, two-stage construction, strong hierarchy, all main and pairwise lower-order effects, and supported continuous or `0/1` binary moderators. Its point/bootstrap Registry cells are scoped Standard for this exact predicate.
- **Higher-order constructs:** select at least two eligible constructs, then open **Higher-Order Construct…** from the Model menu or the selection context menu. Choose whether the HOC explains its dimensions or the dimensions form the HOC. QuickPLS derives the RR/RF/FR/FF type from that direction and the dimensions' existing Mode A/B measurement, then recommends a construction approach from the current topology. Use **Edit Higher-Order Construct…** from the HOC context menu, Properties, or Enter key to revise it.
- **Moderated mediation:** in PLS Bootstrapping setup, choose one eligible two-relation path and one first- or second-stage interaction. Probes are fixed at standardized `−1`, `0`, and `+1`.
- **CB-SEM:** use common-factor constructs and the Advanced Parameter Table for bounded recursive ML or recursive case bootstrap.

The HOC dialog keeps construction approach and the optional legacy short code under **Advanced**. Measurement-only dimensions default to disjoint two-stage when eligible; dimensions already in structural relationships prefer embedded two-stage. Repeated and extended-repeated approaches appear only for combinations supported by the current bounded workflow; hybrid remains read-only compatibility.

Advanced changes use **Save As Revision**. Creating or editing an HOC in an activated immutable project writes one atomic versioned revision while retaining the original project unchanged. Its existing scientific term/output identities and authored structural paths are preserved when the HOC is edited. Historical General SEM and Exact CB-SEM payloads remain readable through hidden compatibility adapters; they are not separate workspace tabs.

## Calculate and preflight

Use the generic `Calculate` command. Its 18-method catalogue is unchanged. Preflight evaluates the exact model, dataset, recipe, and Registry cell, then routes PLS Algorithm, Bootstrapping, or CB-SEM to the bounded capability owned by the current topology and inference settings. A method can be:

- `Supported` — a matching scoped-Standard cell can run;
- `Experimental` — a matching Labs cell can run only after Labs opt-in; or
- `Blocked` — the setup is incompatible, with a corrective explanation.

The setup shows detected features, expected result groups, and direct correction actions. An eligible HOC appears as one compact row with its name, RR/RF/FR/FF type, approach, and an **Edit…** action. PLS Algorithm routes to the existing HOC point cell; PLS Bootstrapping routes to the existing point plus full-model case-bootstrap cells. Starting a run opens native progress. Cancellation publishes no partial analytical result.

## Results

Verified results open in the normal Results workspace and remain available after strict save/close/reopen. Depending on the selected estimator, result groups can include:

- measurement loadings, weights, and collinearity;
- structural paths and effects;
- mediation and moderation output;
- three-way conditional effects and two-dimensional simple-slope output for the exact scoped Standard cell;
- higher-order stages and generated-variable mappings;
- conditional indirect effects and moderated-mediation indices;
- CB-SEM parameters, standardized estimates, fit, and identification; and
- bootstrap inference and ordered failure accounting.

Researcher-authored paths remain distinguishable from generated technical paths. Reflective HOC relationships report loadings; formative relationships report weights. HOC results are grouped as component relationships, HOC structural paths, extended effects when applicable, and bootstrap inference. Selecting an HOC result highlights the HOC and its dimensions without adding stored scientific edges.

Normal result labels come from the immutable model snapshot attached to the
run. Primary identity columns remain sticky in wide tables, numeric cells align
as numbers, and interval tables expose their confidence level. Generated term
IDs, hashes, and raw receipts remain under Run Details or Diagnostics.

### PLS model fit

PLS point-fit output is titled **Model fit — descriptive**. SRMR and NFI are approximate fit measures; `d_ULS` and `d_G` remain descriptive unless the completed result contains separately linked exact-fit inference. Use the title's information button or Model Fit Details for the full interpretation.

The Properties pane reports exactly one state: **Exact-fit bootstrap: Not run**, **Exact-fit results available**, **Exact-fit results partial**, **Exact-fit results unavailable**, or **Exact-fit run failed**. Amber or red is reserved for an exact-fit run that was requested but incomplete or failed. Adapted Bollen–Stine is not offered as a Calculate option in Version 2.52.

## Export

Canonical General SEM exports are generated from the same result document shown in Results:

- CSV for a selected table;
- XLSX with multiple sheets;
- self-contained HTML;
- PDF;
- SVG; and
- PNG.

Stable result IDs and provenance accompany the publication contract. Other method families expose their compatible table, report, or diagram formats.

## Scope and interpretation

Always review Method Details and the exact supported predicate before reporting results. Scoped Standard does not mean unrestricted SmartPLS parity, identical undocumented behavior, or a causal claim.

- [Quick Start](QUICK_START.md)
- [Method Compatibility](METHOD_COMPATIBILITY.md)
- [Known Differences](KNOWN_DIFFERENCES.md)
- [Version 2.55 Release Notes](RELEASE_NOTES_V2_55_0.md)
- [Version 2.54 Release Notes](RELEASE_NOTES_V2_54_0.md)
- [Version 2.53 Release Notes](RELEASE_NOTES_V2_53_0.md)
- [Version 2.52 Release Notes](RELEASE_NOTES_V2_52_0.md)
- [FAQ](FAQ.md)
