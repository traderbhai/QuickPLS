# QuickPLS User Guide

## Project Model

A QuickPLS project stores datasets, SEM diagrams, method setup, saved runs, report settings, layout metadata, and provenance. Analysis runs are immutable so results can be traced back to the dataset fingerprint, model, settings, random seed, engine version, and warnings.

## Home

Use Home to start new work, open a project, import a dataset, open the demo project, or continue from recovery/recent state when available.

## Data

The Data workspace is for import, inspection, and metadata.

Supported documented import paths include raw data and matrix inputs where method scope permits. Use the quality cards and selected-column metadata panel before modeling.

The `What can I do with this data?` guidance and prefix detection help decide the next step without showing every possible method at once.

## Model

The Model workspace is the SEM designer.

Use it to create constructs, assign indicators, draw paths, arrange the layout, validate the diagram, and select compatible result overlays. Layout metadata is UI-only and does not change numerical fingerprints.

## Setup

Setup is where method applicability is evaluated.

Each method is shown with status, reasons, expected outputs, and next actions. Methods are grouped by research workflow rather than shown as one flat catalog.

Common statuses:

- `Recommended`: ready and sensible for the current project.
- `Available`: runnable but not the primary recommendation.
- `Needs setup`: possible after required fields or settings are completed.
- `Not applicable`: incompatible with the current data/model.
- `Unsupported`: outside documented QuickPLS scope.
- `Experimental`: available only with explicit warning or watermark.

## Run

Run executes the selected method through the offline desktop engine. If the run is disabled, QuickPLS shows the exact blocker near the action.

## Results

Results are organized around researcher tasks:

- overview;
- measurement;
- structural;
- validity;
- inference;
- prediction;
- groups;
- diagnostics;
- interpretation;
- comparison.

Tables stay numerically clean. Interpretation findings are value-specific, expandable, and tied to the selected run.

## Report

Report prepares publication and review outputs. SVG is the audited figure export. CSV, HTML, and XLSX table exports are available where a completed compatible run exists.

Reviewer/reproducibility reports include scope, warnings, method version, fingerprints, and provenance.

## Method Scope

Always check method scope before reporting results:

- [Method Compatibility](METHOD_COMPATIBILITY.md)
- [Known Differences](V1_KNOWN_DIFFERENCES.md)
- [Validation Artifact Index](VALIDATION_ARTIFACT_INDEX_V1_0.md)

QuickPLS does not claim SmartPLS equivalence and does not import SmartPLS project files.
