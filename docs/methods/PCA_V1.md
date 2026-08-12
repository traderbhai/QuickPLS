# PCA_V1

Status: validated bounded backend, archive, native desktop, export, and reopen workflow.

`pca_v1` is a standalone principal component analysis workflow for selected numeric raw-data columns. It is separate from the PCA weighting option used inside PLS-SEM estimation and does not create an editable SEM model.

## Contract

- Input is raw data only, with 2 to 50 distinct selected numeric variables.
- At least three complete finite observations are required. Selected variables are listwise-deleted, centered, and scaled using sample standard deviations; constant columns are rejected.
- The analysis uses the correlation-matrix eigensystem. More variables than complete observations are supported.
- Component order is descending by eigenvalue. Signs are oriented deterministically from the largest absolute eigenvector entry.
- Retention is controlled by metadata:
  - `pca_component_rule = kaiser|fixed|variance_threshold`
  - `pca_components` for a fixed count
  - `pca_variance_threshold` in `[0.01, 0.999]`; the first component that reaches or crosses the threshold is retained
- Output includes eigenvalues, explained and cumulative variance, loadings, eigenvector weights, complete-case scores, observations, variables, warnings, and `method_version = pca_v1`.
- The project contract requires an empty SEM recipe model, standardized/listwise settings, no case weights, and no external bootstrap or permutation settings.

## Native workflow

- Data exposes `Analyze...`, which opens the shared searchable calculation catalog with Principal Component Analysis under Standalone analysis.
- The setup selects numeric variables and one of the three retention rules. Correlation-matrix input, standardized values, listwise deletion, no rotation, and no inferential resampling are disclosed as fixed scope.
- Results contain Component summary, Component loadings and weights, and Calculation scope. Full component scores are added only when exporting, avoiding an unnecessary large in-memory UI table.
- CSV, HTML, reviewer pack, XLSX, and Print/PDF are table-only; no model SVG is offered.
- Native save/reopen retains the exact typed result and a null model association, so standalone PCA never creates a phantom editable model.

## Excluded scope

- Rotation methods.
- Pairwise deletion.
- Covariance/correlation-only imported matrices.
- Categorical encodings or automatic nonnumeric conversion.
- PCA inference, loading uncertainty, or component-score uncertainty.
- SmartPLS feature parity beyond this independently implemented bounded PCA workflow.

## Validation

- `npm run qpls:pca:reference` compares the bounded fixture against an independent NumPy eigensystem.
- `npm run qpls:promotion:pca` covers a hand-checkable two-variable fixture, high-dimensional and more-variables-than-rows shapes, listwise deletion, constant-column rejection, all retention rules, strict append/save/reopen validation, browser setup, genuine packaged execution, full-score XLSX export, and same-run reopen.
- The packaged acceptance uses a 140-row tracked fixture and a 95% variance threshold. PC3 remains below 95%, so retaining PC4 proves the threshold-crossing rule rather than merely exercising a fixed component count.
