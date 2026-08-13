# QuickPLS v2.0.1 Home and Data Redesign

## Scope

This milestone applies the QuickPLS 2.0 desktop design direction to the Home and Data workspaces. It is a frontend/product milestone only.

No statistical engines, formulas, result schemas, project archive formats, validation tolerances, or numerical fingerprints are changed.

## Home Workspace

Home is now a project command center:

- current workspace hero with save/open actions;
- one recommended next step based on project state;
- compact primary actions for model building, data import, demo project, and opening a project;
- workflow status for Data, Model, Setup/Run, and Report;
- sample project gallery;
- guided dataset workflow summary.

## Data Workspace

Data is now a desktop workbench:

- import source and data quality are presented together at the top;
- raw/covariance/correlation import requirements remain explicit;
- method guidance explains what can be done with the loaded data;
- prefix-based construct creation remains available;
- the preview table and metadata editor remain the dominant working area;
- native import, sample dataset loading, metadata update, and browser CSV preview continue to use existing APIs.

## Validation Evidence

- `validation/v201_home_data_smoke.mjs`
- `validation/v201_home_data_audit.py`
- `validation/results/v201_home_data_smoke.json`
- `validation/results/v201_home_data_audit.json`

## Release Artifact Rule

When a desktop build is requested, versioned artifacts must be created under:

```text
D:\QuickPLS\target\release\artifacts
```

using the existing non-overwriting installer, portable executable, and checksums naming convention.
