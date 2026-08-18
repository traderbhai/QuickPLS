# PLS MGA Permutation v2

Status: implemented but Phase 4 promotion-blocked. Signed A-minus-B path/loading/weight estimates reverse exactly when the selected Group A/B order is exchanged, but attempted/failed fit counts and two-tailed p values do not remain invariant. The frozen boundary therefore fails and permutation MGA v2 is not currently engine-, native-, or release-qualified by the QuickPLS 3 factory. Historical `pls_mga_permutation_v1` archives remain readable under their original, narrower contract.

Permutation MGA v2 is the inferential companion to QuickPLS two-group MGA v2. It re-estimates both group models after every deterministic label permutation and reports A-minus-B differences for structural paths, outer loadings, and outer weights. The workflow runs together with `micom_v2`; it does not use measurement-parameter tests as a substitute for measurement-invariance assessment.

## Bounded execution contract

- Exactly two distinct observed group values are selected, with at least ten complete model cases in each group.
- Path weighting, standardized preprocessing, and listwise deletion are fixed for the current contract.
- The recipe requests exactly `group_methods = "micom,mga_permutation"`, explicitly confirms MICOM Step 1, and requests 5,000 to 10,000 usable permutations.
- Case weights, generated interactions, higher-order constructs, correlation/covariance-only inputs, and more than two simultaneous groups are unsupported.

Rows with missing, unsupported, or unselected group values are excluded and disclosed. The original and permuted samples contain only the complete cases frozen for the selected Group A/Group B pair.

## Estimation and orientation

QuickPLS fits a pooled reference and separate original Group A and Group B PLS models. Each group solution is sign-aligned to the pooled solution. For every permutation, labels are shuffled without replacement while group sizes remain fixed; both group models are then re-estimated and aligned using the same rules.

The implementation reuses each successful pair of permuted group fits for all three evidence families:

- Structural path-coefficient differences.
- Outer-loading and outer-weight differences.
- MICOM Step 2 compositional correlations.

MICOM Step 3 uses the fixed pooled-model construct scores with each permuted label assignment, as documented in [MICOM_V2.md](MICOM_V2.md).

## Inference

Every reported original difference is `estimate_A - estimate_B`. For a statistic `d` and `B` usable permutations, the two-tailed empirical p value is

`p = (1 + count(|d_b| >= |d_observed|)) / (B + 1)`.

The percentile rank is `count(d_b <= d_observed) / B`. It is descriptive and directional; the persisted inferential field is the add-one two-tailed p value. QuickPLS records requested, usable, attempted, and failed permutation counts. Singular or non-convergent assignments are skipped deterministically, and the calculation fails if it cannot collect the requested number of usable fits within its attempt bound.

## Result payload

`pls_mga_permutation_v2` contains:

- One comparison row per structural path.
- Two measurement rows per model indicator: one outer loading and one outer weight.
- Original A-minus-B differences, empirical two-tailed p values, and percentile ranks.
- The selected group column, permutation counts, method version, and warnings.

The companion `pls_mga_two_group_v2` payload contains the underlying group paths, R-squared values, outer estimates, transforms, and non-inferential A-minus-B comparisons. The companion `micom_v2` payload must be inspected before making MGA claims for any composite.

## Interpretation limits

A small permutation p value indicates that the observed parameter difference is unusual under exchangeable group labels within this calculation contract. It is not evidence of causality, practical importance, or semantic equivalence. Path comparisons are meaningful only for composites with at least partial MICOM invariance. Loading/weight differences are diagnostic parameter comparisons and do not themselves establish or refute the MICOM hierarchy.

This scope does not include parametric MGA, bootstrap MGA, one-tailed tests, omnibus tests across more than two groups, multiplicity correction, interactions, higher-order constructs, or case-weighted analyses.

## Scientific basis and validation

The label-permutation workflow is paired with the measurement-invariance hierarchy and pooled-score equations described by Henseler, Ringle, and Sarstedt (2016), [doi:10.1108/IMR-09-2014-0304](https://doi.org/10.1108/IMR-09-2014-0304). The paper's open version is available from the [University of Twente research repository](https://ris.utwente.nl/ws/files/287320134/2016_IMR_Henseler_Ringle_Sarstedt.pdf).

`validation/micom_v2_reference.py` independently reproduces the original fits, every deterministic permutation fit, path/loading/weight distributions, and the coupled MICOM result. Run the paired promotion evidence with:

```powershell
$env:QUICKPLS_CLI_PATH = "D:\QuickPLS\target\release\qpls.exe"
python validation/micom_v2_reference.py --run-quickpls --permutations 5000
python validation/mga_permutation_method_promotion_audit.py
```

The audit must remain blocked when the paired independent report is absent, uses fewer than 5,000 permutations, disagrees numerically, or lacks the native configural-confirmation and persistence contracts.
