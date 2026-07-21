# PLS Mediation v1

Status: validated for the documented QuickPLS v1.2.1 PLS mediation effect-decomposition scope.

Method version: `pls_mediation_v1`

## Scope

This method classifies mediation from the already estimated PLS path model effects. It does not change the PLS algorithm, latent scores, measurement estimates, or structural path coefficients.

The slice covers:

- direct effect from construct `i` to construct `j`
- indirect effect from all directed paths from `i` to `j` with one or more mediators
- total effect as direct plus indirect
- variance accounted for as `indirect / total` when `abs(total) > 1e-12`
- descriptive class labels for no effect, direct-only, indirect-only, complementary partial, and competitive partial patterns
- bootstrap percentile, BCa, and bootstrap-t rows for `indirect_effect` parameters when the run includes the corresponding inference artifacts

The slice does not yet cover PROCESS-style covariate workflows, moderated mediation, a formal indirect-effect permutation estimator, broader mediation decision rules, or publication-ready mediation decisions.

## Definitions

For a construct pair `(i, j)`, QuickPLS consumes the effect decomposition produced by `pls_pm_v1`:

```text
total_ij = direct_ij + indirect_ij
```

The tolerance is fixed at `1e-12` for zero checks.

Classification:

- `no_effect`: `abs(direct) <= tolerance` and `abs(indirect) <= tolerance`
- `direct_only`: `abs(direct) > tolerance` and `abs(indirect) <= tolerance`
- `indirect_only`: `abs(direct) <= tolerance` and `abs(indirect) > tolerance`
- `complementary_partial`: direct and indirect are both nonzero and have the same sign
- `competitive_partial`: direct and indirect are both nonzero and have opposite signs

VAF is persisted as null when the total effect is zero within tolerance.

## Inference

The resampling engine serializes effect identities as JSON tuple keys:

```text
["direct_effect", ["i", "j"]]
["indirect_effect", ["i", "j"]]
["total_effect", ["i", "j"]]
```

The saved-run mediation table reads the `indirect_effect` row for each source-target pair and displays the available normal-reference p-value plus percentile, BCa, and bootstrap-t confidence intervals. Missing intervals remain `N/A` and retain the source artifact's unavailable reason in the cell title.

## Current Evidence

- `validation/mediation_reference.py` computes a single-item mediation fixture independently from sample-standardized OLS equations.
- `validation/results/mediation_reference_report.json` passed with maximum absolute difference below `1e-12`.
- `validation/mediation_r_reference.py` runs development-only R base `lm` against the same single-item fixture as a second executable reference source.
- `validation/results/mediation_r_reference_report.json` passed with maximum absolute difference below `1e-12`.
- `validation/mediation_published_example.py` reruns the documented cSEM `threecommonfactors` example, requires the existing cSEM path-coefficient agreement fixture to pass, and independently decomposes direct, indirect, total, VAF, and class labels from the validated path matrix.
- `validation/results/mediation_published_example_report.json` passed with exact agreement and a nonzero `eta1 -> eta3` indirect effect.
- `validation/mediation_metamorphic.py` adds a bounded deterministic signal simulation plus positive-affine, row-order, construct-order, and broken-mediator checks.
- `validation/results/mediation_metamorphic_report.json` passed with maximum independent-reference difference below `1e-10` and mediator-permutation degradation.
- `validation/mediation_randomization.py` adds a bounded indirect-effect randomization screen: QuickPLS is compared to independent standardized OLS equations for observed and permuted-mediator datasets, and 199 deterministic mediator permutations are used for a plus-one two-sided screen.
- `validation/results/mediation_randomization_report.json` passed with randomization p-value below `0.05` and material indirect-effect degradation after mediator randomization.
- `qpls-resampling::pls_bootstrap_carries_mediation_indirect_effect_inference` checks that a three-construct indirect effect is retained in bootstrap percentile and BCa artifacts.

## Warnings

`pls_mediation_v1` estimates are validated only for the documented PLS path-effect decomposition scope. Publication interpretation of indirect effects requires the relevant validated bootstrap or permutation interval. PROCESS-style observed-variable mediation, moderated mediation, and unsupported model shapes remain outside this scope.
