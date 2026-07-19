# Bounded Two-Group MGA V1

`pls_mga_two_group_v1` is the first QuickPLS v0.6 multigroup-analysis slice. It is an experimental observed-group diagnostic for exactly two groups.

## Scope

- Available through `AnalysisMethod::Mga`.
- Requires recipe metadata `mga_group_column` or `mga.group_column`.
- The group column must identify exactly two non-missing observed groups after import.
- The estimator subsets the raw dataset by group, runs the ordinary standardized PLS-PM estimator independently for each group, and reports group-specific path coefficients and R2 values.
- For each structural path present in the recipe, QuickPLS reports coefficient A, coefficient B, difference `A - B`, an approximate standard error, t statistic, and two-sided normal p value.

## Current Limitations

- This is not permutation MGA.
- This is not MICOM measurement-invariance testing.
- This is not full multigroup publication evidence.
- Case-weighted MGA, generated interaction constructs, higher-order constructs, and more than two groups are blocked in this preview.
- The p values are approximate fixed-score diagnostics only; validated permutation and bootstrap variants remain future work.

## Validation Evidence

`npm run qpls:mga:reference` writes `validation/results/mga_reference_report.json`.

The fixture generates two observed groups with different `x -> y` and `z -> y` structural effects. An independent Python implementation of the same published PLS path-weighting stages estimates each group separately and compares path coefficients against QuickPLS. The current observed maximum absolute path delta is `3.33e-16`, well inside the `1e-6` deterministic gate.

The same report checks method-version provenance, group-column serialization, group counts, path-difference direction, p-value availability, and experimental warnings.
