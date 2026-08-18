# Consistent Permutation v1

Status: bounded internal implementation, `coverage_state=absent`, and
`evidence_state=absent`. The method identity is `plsc_permutation_v1`. Its
registry surface remains Labs, but absent coverage and evidence keep it
non-executable and hidden; this is not a SmartPLS-parity or numerical-
qualification claim.

SmartPLS defines consistent permutation as the ordinary two-group permutation
workflow with every group and permuted-group model re-estimated using PLSc. The
official workflow also includes selectable test direction and MICOM. QuickPLS
v1 freezes a smaller executable kernel so that the PLSc-specific estimator,
randomization, persistence, and export semantics can be reviewed before any
coverage or evidence promotion.

## Scientific question and estimand

For two pre-defined groups A and B, v1 tests the null hypothesis that selected
group-specific `plsc_v2` parameters are exchangeable with respect to group
membership. The signed observed contrast is always

`difference_j = estimate_A,j - estimate_B,j`.

The v1 parameter manifest is limited to:

- attenuation-corrected structural paths;
- attenuation-corrected outer loadings;
- Dijkstra-Henseler rho_A values;
- attenuation-corrected construct correlations; and
- attenuation-corrected endogenous-construct R-squared values.

Outer weights, direct/indirect/total effects, confidence intervals, more than
two groups, and MICOM are not v1 outputs. A recipe-selectable direction is not
available, but the frozen combined result reports both directed tails alongside
the two-sided result. The remaining exclusions are parity blockers, not an
implicit scientific equivalence.

## Eligible model and data

- Raw observations only.
- Current schema-v3 `plsc` recipe and current `plsc_v2` point estimator.
- Exactly two distinct, explicitly named values from one text, Boolean,
  integer, or finite numeric group column.
- Every complete model case must have exactly one of those two group values.
  Missing or unselected group values among otherwise complete model cases fail
  preflight; they are not silently discarded.
- At least ten complete cases in each group.
- Reflective constructs only, at least two indicators per construct, path or
  factor weighting, standardized preprocessing, and listwise deletion.
- A recursive structural model with at least one path.
- No case weights, controls, interactions, higher-order constructs, bootstrap,
  studentized inference, or adapted exact-fit bootstrap in the same recipe.

The group column and directed labels are stored in the schema-v3 typed
`method_config` object with `kind=plsc_permutation`, `group_column`, `group_a`,
and `group_b`. Legacy `mga_*` metadata is only an internal compatibility
projection and is rejected when supplied by a schema-v3 caller. The group
column cannot also be a model indicator.

## Exact indexed randomization plan

V1 accepts 99 through 10,000 permutations; the intended applied default is
1,000 and final analyses should normally use at least 5,000. The current
combined test contract reports the two-sided result and both directed
greater-or-equal and less-or-equal tails at the recipe's fixed 0.05
significance level. Test direction is not a recipe selector.

The analyzed complete-case rows are placed in a canonical pooled order: the
lexicographically smaller group label first, then the larger label, preserving
source row order inside each label. Group A/B is an interpretation direction,
not the pooled-order identity. For fixed permutation index `b`, QuickPLS
derives one bijection from:

`(master_seed, "plsc_group_label_permutation_v1", b)`.

The frozen label vector is shuffled through that bijection. This assigns every
pooled observation exactly once, without replacement, while preserving the
original A and B sample sizes. The requested A/B swap therefore uses the same
row pool and complementary label vectors.

The original groups and both groups in every permutation receive full
`plsc_v2` re-estimation. Each solution is sign-aligned to the pooled PLSc point
solution before the canonical parameter map is extracted. A changed or missing
parameter identity fails that indexed permutation.

For each usable permutation, `d*_bj` is the A-minus-B difference. The
two-sided plus-one probability is

`p_j = (1 + count(|d*_bj| >= |d_j|)) / (1 + usable_permutations)`.

The same usable-permutation denominator produces the directed A-minus-B tails:

`p_greater,j = (1 + count(d*_bj >= d_j)) / (1 + usable_permutations)`, and

`p_less,j = (1 + count(d*_bj <= d_j)) / (1 + usable_permutations)`.

The result stores both directed counts and probabilities under the exact
`plsc_directional_permutation_v1` / `directed_greater_less_plus_one_v1`
identity. Swapping Group A and Group B reverses differences, preserves the
two-sided probability, and exchanges the greater and less tails.

There is no multiplicity adjustment.

## Determinism, failures, and cancellation

Permutation indices and result aggregation are independent of worker
scheduling. The same dataset fingerprint, recipe, seed, and worker count of
one or more must produce the same analytical payload. Changing only worker
count must not change it.

Each requested index is attempted once. An inadmissible rho_A, inadmissible
corrected correlation, PLSc nonconvergence, singular equation, nonfinite value,
or parameter-identity mismatch is retained as a typed failed entry. There is
no retry, replacement, clamping, uncorrected-PLS fallback, or silent omission.
At least 90% of the pre-planned indices must be usable; otherwise the operation
fails without committing a result. P-values condition explicitly on the
retained usable set. Cancellation returns a terminal cancellation and commits
no partial payload.

The result persists strict index order, label-assignment digests, successful
parameter-vector digests, failure reasons, requested/usable/failed counts,
group identities and counts, point-parameter digests, test definition, and the
immutable retry policy.

## Product and qualification boundary

Runner, project, native result, and export integration must fail closed unless
the exact method, scheduler, operation, estimator, groups, settings, ledgers,
parameter identities, arithmetic, and envelope provenance agree. The result is
always labelled Experimental/Internal in native tables and exports.

Qualification still requires an independent PLSc two-group oracle, hand and
published fixtures, A/B-swap and worker invariance, failure simulations,
archive tamper tests, semantic export readback, packaged Windows cancellation,
and current SmartPLS workflow/result capture. MICOM, official direction-
selection workflow coverage, official defaults, outer-weight/effect breadth,
chart breadth, and difficult model shapes remain explicit blockers. Registry
coverage and evidence remain unchanged until those obligations are separately
admitted.

## Scientific and parity references

- Dijkstra and Henseler (2015), *Consistent and Asymptotically Normal PLS
  Estimators for Linear Structural Equations*,
  https://doi.org/10.1016/j.csda.2014.07.008.
- Dijkstra and Henseler (2015), *Consistent Partial Least Squares Path
  Modeling*, https://doi.org/10.25300/MISQ/2015/39.2.02.
- SmartPLS, *Consistent Permutation*,
  https://www.smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/consistent-permutation/.
- SmartPLS, *Permutation*,
  https://www.smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/permutation/.
