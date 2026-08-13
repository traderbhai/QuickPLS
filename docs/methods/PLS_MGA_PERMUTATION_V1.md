# PLS MGA Permutation v1

Status: validated for the documented QuickPLS v1.2.2 two-group permutation MGA scope.

`pls_mga_permutation_v1` is a permutation-based MGA payload emitted from `AnalysisMethod::Mga` when recipe metadata contains `group_methods = "mga_permutation"`, `mga_group_column`, and explicit distinct `mga_group_a` / `mga_group_b` values.

## Scope

- Fits the two explicitly selected group values with the PLS-PM engine; additional observed values are excluded and disclosed.
- Freezes complete model cases before reassigning group labels, preserving the analyzed A/B sizes in every usable permutation.
- Re-estimates group-specific PLS models for deterministic group-label permutations without replacement.
- Uses stable replicate ordering derived from the recipe seed.
- Reports original path differences, empirical two-sided p values, percentile ranks, usable permutation count, and warnings.
- Emits a strong warning that permutation MGA does not establish measurement invariance. Bundled MICOM execution is disabled pending a scientifically valid, independently validated reimplementation.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, more than two selected groups at once, groups with fewer than ten complete model cases, measurement-model comparisons, MICOM, bootstrap MGA families, one-tailed inference, and broader group-difference claims outside this contract are unsupported.

## Validation

`npm run qpls:mga:permutation-reference`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:mga-permutation` write the reference and promotion artifacts. Promotion is limited to this documented scope.
