# QuickPLS Quick Start

This guide follows the Version 2.51 candidate workflow: `Launcher → Data → Canvas → Calculate → Results`.

## 1. Start or open a project

Open QuickPLS and choose one:

- `Open demo project` to try a bundled workflow.
- `Import dataset` to start with CSV, TSV, XLSX, or SAV data.
- `Open project` to load an existing `.qpls` project.

Save early so autosave and recovery can preserve the project.

## 2. Import and inspect data

Open `Data`, import the dataset, and check row/variable counts, missing values, nonnumeric or constant columns, headers, and selected-variable metadata. Matrix input is available only for methods whose documented scope permits it.

## 3. Build the model

Open `Model` and use Canvas to add constructs, assign indicators, draw structural paths or supported covariances, arrange the diagram, and validate it. Canvas is the only permanent model-authoring document. For common-factor CB-SEM, set the applicable construct representation and open the **Advanced Parameter Table** from Canvas or CB-SEM setup when fixed/free parameters, bounds, or equality labels need attention.

Use the Higher-Order Construct or Moderating Effect commands only when the model meets their displayed requirements. QuickPLS detects mediation from the drawn paths and offers bounded moderated-mediation selection in PLS Bootstrapping setup. Advanced changes use **Save As Revision** so the source project remains unchanged.

## 4. Calculate

Choose `Calculate`. The searchable catalogue still contains 18 methods. QuickPLS evaluates the resident model, data, settings, and exact Registry cells, then shows only the settings relevant to the selected method.

- Choose **PLS Algorithm** for point estimation. Eligible mediation, moderation, and higher-order output is detected from the model.
- Choose **Bootstrapping** for inference. Eligible multiple mediation, simultaneous moderation, higher-order, and bounded moderated-mediation calculations route automatically.
- Choose **CB-SEM** for bounded common-factor ML, with case bootstrap selected inside its method settings when eligible.
- Use Data's `Analyze…` command for compatible model-free methods.

Detected feature summaries explain what will be calculated. An unavailable action shows the corrective reason and, where possible, a direct action such as opening the Advanced Parameter Table or creating a calculation-ready revision. A blocked setup never modifies the diagram. Standard methods do not require the Experimental Labs preference.

## 5. Monitor or cancel

Start the calculation and follow native progress. Cancellation stops publication: no partial analytical result or partial archive attachment is created.

## 6. Review results

After completion, QuickPLS opens the verified canonical result in `Results`. Its searchable sidebar contains only groups owned by that run, which may include Overview, Measurement Model, Structural Model, Direct/Indirect/Total Effects, Moderation, Higher-Order Constructs, Moderated Mediation, CB-SEM Parameters, Model Fit, Bootstrap Inference, and Run Details.

## 7. Export and reopen

Use `Export` from Results. Canonical General SEM results support:

- CSV;
- XLSX;
- self-contained HTML;
- PDF;
- SVG; and
- PNG.

Other result families display only their compatible formats. Save the project, close it, and reopen it to restore the same verified model, recipe, result, tables, and provenance.

For the exact supported boundaries, see [Method Compatibility](METHOD_COMPATIBILITY.md) and the [Version 2.51 Candidate Release Notes](RELEASE_NOTES_V2_51_0.md).
