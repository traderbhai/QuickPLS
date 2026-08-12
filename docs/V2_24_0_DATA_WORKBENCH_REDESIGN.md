# QuickPLS v2.24.0 Data Workbench Redesign

Status: `validated`

Milestone gate: `v2_24_0_data_workbench_redesign`

## Summary

v2.24.0 converts the Data workspace from a long web-style form into a native desktop-style data workbench. The screen now behaves more like an SPSS Data View / Variable View surface while preserving the existing QuickPLS dataset/import APIs and project format.

## Implemented Scope

- Added tabbed Data workspace sections:
  - `Data View`
  - `Variable View`
  - `Import History`
  - `Data Quality`
  - `Notes`
- Kept the dense data grid and right-side column metadata/property inspector.
- Added a Variable View table with one row per variable, including type, scale, missing, complete, unique, min, and max values.
- Added a Data Quality workbench with variable-level issue rows for constant columns, missing-heavy variables, invalid headers, and duplicate headers.
- Kept method applicability guidance in the data-quality context.
- Kept native import behavior and matrix import warnings unchanged.

## Boundary

This is frontend/product work only. It does not change statistical engines, formulas, method validation, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## Evidence

- `src/components/DataWorkspace.tsx`
- `src/styles.css`
- `validation/v2240_data_workbench_smoke.mjs`
- `validation/v2240_data_workbench_audit.py`
- `validation/results/v2240_data_workbench_smoke.json`
- `validation/results/v2240_data_workbench_audit.json`
- `cargo run -p qpls-cli -- gate v2_24_0_data_workbench_redesign`
