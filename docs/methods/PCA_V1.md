# PCA_V1

Status: experimental v0.8 preview.

`pca_v1` is a standalone principal component analysis workflow for numeric raw-data columns. It is separate from the PLS block PCA weighting scheme.

## Contract

- Input is raw data only.
- Numeric variables are listwise-deleted, centered, and scaled with sample standard deviations.
- The analysis uses the correlation matrix eigensystem.
- Component signs are deterministically oriented by the largest absolute loading.
- Retention is controlled by metadata:
  - `pca_component_rule = kaiser|fixed|variance_threshold`
  - `pca_components` for fixed component count
- Output includes eigenvalues, explained variance, cumulative variance, loadings, weights, scores, retained-component rule, observations, variables, warnings, and `method_version = pca_v1`.

## Unsupported In v0.8

- Covariance/correlation-only input.
- Nonnumeric variables.
- Pairwise deletion.
- Rotation methods.
- Inference or component-score uncertainty.

## Validation

`npm run qpls:pca:reference` compares the bounded fixture against an independent NumPy eigensystem calculation. Stable publication claims require broader high-dimensional, missing-data, sign-orientation, and rotation evidence.
