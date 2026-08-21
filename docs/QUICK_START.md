# QuickPLS Quick Start

This guide follows the Version 2.50 desktop workflow: `Launcher → Data → Model → Calculate → Results`.

## 1. Start or open a project

Open QuickPLS and choose one:

- `Open demo project` to try a bundled workflow.
- `Import dataset` to start with CSV, TSV, XLSX, or SAV data.
- `Open project` to load an existing `.qpls` project.

Save early so autosave and recovery can preserve the project.

## 2. Import and inspect data

Open `Data`, import the dataset, and check row/variable counts, missing values, nonnumeric or constant columns, headers, and selected-variable metadata. Matrix input is available only for methods whose documented scope permits it.

## 3. Build the model

Open `Model` and use Canvas to add constructs, assign indicators, draw structural paths or supported covariances, arrange the diagram, and validate it. For common-factor CB-SEM, set the applicable construct representation and review the Parameter Table.

Use the Higher-Order Construct or Moderating Effect commands only when the model meets their displayed requirements. QuickPLS uses **Save As Revision** for General SEM HOC and moderated-mediation authoring so the source project remains unchanged.

## 4. Calculate

Choose `Calculate`. QuickPLS evaluates the resident model, data, settings, and exact Registry cells before showing an estimator as available.

- Choose **PLS-SEM** for supported composite models, mediation/moderation, higher-order PLS, or bounded moderated mediation.
- Choose **CB-SEM ML** for the bounded recursive common-factor scope.
- Choose **CB-SEM recursive bootstrap** when the model and fixed bootstrap settings meet its exact predicate.
- Use Data's `Analyze…` command for compatible model-free methods.

An unavailable action shows the corrective reason without modifying the diagram. Standard methods do not require the Experimental Labs preference.

## 5. Monitor or cancel

Start the calculation and follow native progress. Cancellation stops publication: no partial analytical result or partial archive attachment is created.

## 6. Review results

After completion, QuickPLS opens the verified canonical result in `Results`. Available groups depend on the method and may include measurement, structural, validity, inference, higher-order stages, conditional indirect effects, CB-SEM parameters, fit, identification, and bootstrap failures.

## 7. Export and reopen

Use `Export` from Results. Canonical General SEM results support:

- CSV;
- XLSX;
- self-contained HTML;
- PDF;
- SVG; and
- PNG.

Other result families display only their compatible formats. Save the project, close it, and reopen it to restore the same verified model, recipe, result, tables, and provenance.

For the exact supported boundaries, see [Method Compatibility](METHOD_COMPATIBILITY.md) and [Version 2.50 Release Notes](RELEASE_NOTES_V2_50_0.md).
