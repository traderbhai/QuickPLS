# PLS MGA Permutation v1

`pls_mga_permutation_v1` is an experimental QuickPLS v0.6 permutation-based MGA payload emitted from `AnalysisMethod::Mga` when recipe metadata contains `group_methods = "mga_permutation"` and `mga_group_column` names a two-group observed column.

## Scope

- Fits the original two observed groups with the PLS-PM engine.
- Re-estimates group-specific PLS models for deterministic group-label permutations.
- Uses stable replicate ordering derived from the recipe seed.
- Reports original path differences, empirical two-sided p values, percentile ranks, usable permutation count, and warnings.
- Emits a strong warning when MICOM is absent or does not pass partial invariance.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, more than two groups, too-small groups, and publication-ready group-difference claims are unsupported in this preview.

## Validation

`npm run qpls:mga:permutation-reference` and `npm run qpls:v06:validate` write `validation/results/v06_group_methods_reference_report.json`.
