# QuickPLS User Guide

This guide describes the Version 2.50 Windows workflow. QuickPLS runs analyses locally without an account, cloud service, R, or Python at runtime.

## Projects and scientific authority

A `.qpls` project stores immutable dataset versions, SEM models, recipes, completed results, report settings, layout metadata, and provenance. A completed result remains bound to its dataset, model, settings, seed, method version, capability cell, and engine identity.

General SEM projects use `SemModelV4` as the model authority, `AnalysisRecipeV4` as the requested analysis, compiled plans as execution authority, `CanonicalResultDocumentV2` as result authority, and schema-6 as persistence authority. Canvas layout cannot override scientific content.

## Launcher and Data

Use Launcher to create or open a project, open a sample, or continue from recovery. Use Data to import and inspect raw or supported matrix data, review missing values and quality, search variables, edit metadata, and create immutable derived dataset versions.

## Model and Parameter Table

Use Model to create constructs, assign indicators, draw paths and supported covariances, arrange the diagram, and validate the model. Layout changes do not alter analytical fingerprints.

For CB-SEM General, construct representation and the active Parameter Table define the model sent to native preflight and estimation. Supported fixed/free rows, equality labels, and row bounds are preserved; unsupported constraint objects remain visible and block calculation instead of being discarded.

## General SEM authoring

Version 2.50 adds bounded authoring to the same Canvas:

- **Mediation and simultaneous moderation:** select compatible General SEM PLS calculations from the normal estimator cards.
- **Higher-order constructs:** author one non-nested second-order HOC using an eligible repeated, extended-repeated, embedded two-stage, or disjoint two-stage combination.
- **Moderated mediation:** choose one eligible two-relation path and one first- or second-stage interaction. Probes are fixed at standardized `−1`, `0`, and `+1`.
- **CB-SEM:** use common-factor constructs and the resident Parameter Table for bounded recursive ML or recursive case bootstrap.

Higher-order and moderated-mediation changes use **Save As Revision**. The source file is never silently replaced.

## Calculate and preflight

Use the generic `Calculate` command. Preflight evaluates the exact model, dataset, recipe, and Registry cell and displays applicable PLS-SEM and CB-SEM estimator cards. A card can be:

- `Supported` — a matching scoped-Standard cell can run;
- `Experimental` — a matching Labs cell can run only after Labs opt-in; or
- `Blocked` — the setup is incompatible, with a corrective explanation.

Starting a run opens native progress. Cancellation publishes no partial analytical result.

## Results

Verified results open in the normal Results workspace and remain available after strict save/close/reopen. Depending on the selected estimator, result groups can include:

- measurement loadings, weights, and collinearity;
- structural paths and effects;
- mediation and moderation output;
- higher-order stages and generated-variable mappings;
- conditional indirect effects and moderated-mediation indices;
- CB-SEM parameters, standardized estimates, fit, and identification; and
- bootstrap inference and ordered failure accounting.

Researcher-authored paths remain distinguishable from generated technical paths. Reflective HOC relationships report loadings; formative relationships report weights.

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
- [Version 2.50 Release Notes](RELEASE_NOTES_V2_50_0.md)
- [FAQ](FAQ.md)
