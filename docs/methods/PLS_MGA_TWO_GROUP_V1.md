# Bounded Two-Group MGA V1

`pls_mga_two_group_v1` is the bounded QuickPLS observed-group estimator used by the native Two-Group Permutation MGA workflow. It compares exactly two explicitly selected values even when the grouping variable contains additional observed categories.

## Scope

- Available through `AnalysisMethod::Mga`.
- Requires recipe metadata `mga_group_column`, `mga_group_a`, and `mga_group_b` with distinct, observed, ordered values.
- The grouping variable cannot also be a model indicator. Rows with missing, unsupported, or unselected group values are excluded and disclosed.
- The complete model-case row sets are fixed before fitting. Each selected group requires at least ten complete cases.
- The estimator runs ordinary PLS-PM independently for Group A and Group B and reports group-specific path coefficients and R2 values.
- For each structural path, QuickPLS records coefficient A, coefficient B, and difference `A - B`. Approximate standard errors, t statistics, and normal p values remain descriptive engine diagnostics and are omitted from the primary native MGA report.

## Current Limitations

- This estimator alone is not permutation inference; the native workflow requires the paired `pls_mga_permutation_v1` artifact.
- This is not MICOM measurement-invariance testing.
- This is not full multigroup publication evidence.
- Case-weighted MGA, generated interaction constructs, higher-order constructs, measurement-model group comparisons, and comparing more than two selected groups at once are blocked.
- Bootstrap MGA, PLS-MGA, parametric MGA, Welch-Satterthwaite testing, and one-tailed group inference are not implemented.

## Validation Evidence

`npm run qpls:mga:reference` writes `validation/results/mga_reference_report.json`.

The fixture generates two observed groups with different `x -> y` and `z -> y` structural effects. An independent Python implementation of the same published PLS path-weighting stages estimates each group separately and compares path coefficients against QuickPLS. The current observed maximum absolute path delta is `3.33e-16`, well inside the `1e-6` deterministic gate.

The same report checks method-version provenance, explicit group ordering, group-column serialization, complete-case counts, and path-difference direction.
