# QuickPLS 1.0.0 User Guide

## Overview

QuickPLS is organized around projects. A project contains datasets, model diagrams, analysis settings, saved runs, reports, and provenance.

## Data

Use the Data workspace to import and inspect research data. QuickPLS v1.0.0 validates supported CSV, TSV, XLSX, SAV, covariance, and correlation workflows within the documented scope.

Check missing values, column metadata, and warnings before building a model.

## SEM Designer

The default canvas uses academic SEM diagram grammar:

- Latent constructs are ovals.
- Observed indicators are rectangles.
- Structural paths use single-headed arrows.
- Measurement links connect constructs and indicators.
- Result overlays appear only after a compatible completed run is selected.

You can drag constructs, indicators, and labels where supported. Layout metadata is saved separately from the numerical recipe.

## Running Methods

Choose the method in the analysis controls, configure method-specific settings, and run. QuickPLS stores runs with data fingerprint, model, settings, seed, warnings, method version, and result payload.

## Reports

Reports include:

- Run provenance.
- Result tables.
- Warnings and scope status.
- Publication SVG diagrams.
- CSV, HTML, XLSX, and browser print-to-PDF export paths.

## Interpreting Warnings

Warnings are part of the output. Do not ignore scope, unsupported-shape, stale-result, missing-data, convergence, or experimental-watermark warnings.

## Validation Scope

Use these docs before citing QuickPLS results:

- `docs/V1_SUPPORTED_SCOPE.md`
- `docs/V1_COMPATIBILITY_MATRIX.md`
- `docs/V1_KNOWN_DIFFERENCES.md`
- `docs/METHODOLOGY_MANUAL_V1_0.md`
- `docs/VALIDATION_ARTIFACT_INDEX_V1_0.md`

