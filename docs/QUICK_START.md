# QuickPLS Quick Start

This guide follows the main desktop workflow: `Home -> Data -> Model -> Setup -> Run -> Results -> Report`.

## 1. Start Or Open A Project

Open QuickPLS and choose one:

- `Open demo project` to try the bundled corporate reputation workflow.
- `Import dataset` to start from your own CSV, TSV, XLSX, or SAV file.
- `Open project` to load an existing `.qpls` project.

Save the project early so autosave and recovery can track your work.

## 2. Import And Inspect Data

Open `Data`.

Use `Import Data` for your dataset. After import, check:

- row and variable counts;
- missing cells;
- nonnumeric variables;
- constant columns;
- header issues;
- selected-column metadata.

Use `Create Constructs From Prefixes` when variable names follow patterns like `COMP1`, `COMP2`, `COMP3`.

## 3. Build The SEM Diagram

Open `Model`.

Use the canvas to:

- add constructs;
- assign indicators;
- draw structural paths;
- create covariances where supported;
- arrange the diagram;
- validate the model.

Estimates are hidden until a compatible completed run is selected.

## 4. Choose A Method

Open `Setup`.

QuickPLS evaluates the current data and model and groups methods as:

- recommended;
- available;
- needs setup;
- not applicable;
- unsupported;
- experimental.

Select the method card and complete any required setup fields. Bootstrap, permutation, group analysis, NCA, regression, and prediction settings appear only where relevant.

## 5. Run

Open `Run` and click `Run selected method`.

After completion, QuickPLS saves an immutable run with data fingerprint, recipe, seed, method version, warnings, estimates, and provenance.

## 6. Review Results

Open `Results`.

Use tabs for overview, measurement, structural, validity, inference, prediction, groups, diagnostics, interpretation, and comparison. Interpretation panels use actual result values and remain conservative.

## 7. Export

Open `Report`.

Choose a preset, verify the selected run, preview the diagram, and export:

- SVG publication diagram;
- CSV tables;
- HTML report;
- XLSX workbook;
- browser print-to-PDF workflow where documented.
