# Consistent Permutation v1

Status: contract-only and `absent` in the QuickPLS 3 method-promotion factory. QuickPLS currently rejects permutation settings for `plsc_v2`; the existing `pls_mga_permutation_v2` and `micom_v2` artifacts use ordinary PLS and are not evidence for this method.

`plsc_permutation_v1` is the planned two-group label-permutation procedure in which the complete PLSc v2 estimator is re-run for both groups after every assignment. It depends on release-qualified `plsc_v2` and a separately qualified PLSc-aware measurement-invariance implementation. Reusing a standard-PLS permutation distribution and merely relabeling its estimates as consistent is prohibited.

## Bounded v1 scope

- Exactly two selected observed groups with at least ten complete supported model cases per group.
- Identical model, indicator ordering, preprocessing, missing-data policy, weighting scheme, convergence settings, and sign-alignment rules in both groups.
- Reflective `plsc_v2` scope only: at least two indicators per construct, path or factor weighting, and no interactions, higher-order constructs, case weights, matrix-only input, or more than two simultaneous groups.
- A release run requests 5,000 through 10,000 usable label permutations and a fixed seed.
- Structural paths and corrected outer loadings are compared as `estimate_A - estimate_B`. The paired consistent-MICOM result covers configural invariance, compositional invariance, and equality of composite means and variances under the same frozen permutation plan.

For usable permutation `b`, exactly `n_A` cases are assigned without replacement to Group A and the remainder to Group B. Both group models are independently estimated with `plsc_v2`, aligned to the pooled reference under the frozen rule, and differenced. For parameter `j`, the two-sided add-one probability is

`p_j = (1 + count(|d*_bj| >= |d_j|)) / (B + 1)`.

The stream for permutation `b` is derived from `(master_seed, "plsc_permutation_v1", b)`. Worker count and scheduling cannot affect assignments, usable attempts, result ordering, or the analytical payload.

## Failure and interpretation policy

A nonconvergent or inadmissible PLSc group fit fails the permutation attempt. Failed attempts are not relabeled or silently removed: requested, usable, attempted, failed, and reason counts are persisted, with a frozen finite attempt cap. The calculation fails if it cannot collect the requested usable distribution within that cap.

Permutation evidence is conditional on exchangeability of group labels under the null and the frozen configural model. A small probability is not evidence of causality or practical importance. Structural comparisons must be blocked or explicitly marked uninterpretable for constructs that do not establish at least partial measurement invariance under the paired PLSc-aware invariance result. Loading differences are diagnostic and do not replace the invariance hierarchy.

## Persistence and product contract

The target envelope is `pls_pm_v3` with a typed `plsc_permutation_v1` plan/result and a separately typed PLSc-aware invariance payload. The archive stores no raw permutations. It stores group identity, fixed complete-case manifest, stream domain, counts, canonical parameter identities, original differences, empirical probabilities, invariance rows, warnings, and exact provenance.

Native setup must require group selection and explicit configural confirmation, block unsupported shapes, and estimate the workload before execution. Results and exports must show the invariance hierarchy before structural differences. GUI and CLI must produce the same assignment plan and analytical payload. Installed acceptance must cover cancellation/retry, export, save/reopen, offline execution, and process cleanup.

## Qualification work still required

All roles in `validation/methods/consistent_permutation_v1.manifest.json` are currently empty: independent PLSc re-estimation, null calibration and power simulation, boundary/determinism tests, strict archive validation, native workflow, exports, identity-bound audit, and installed Windows acceptance.

## Scientific sources

- Dijkstra and Henseler (2015), *Consistent Partial Least Squares Path Modeling*, https://doi.org/10.25300/MISQ/2015/39.2.02.
- Chin and Dibbern (2010), *An Introduction to a Permutation Based Procedure for Multi-Group PLS Analysis*, https://doi.org/10.1007/978-3-540-32827-8_8.
- Henseler, Ringle, and Sarstedt (2016), *Testing Measurement Invariance of Composites Using Partial Least Squares*, https://doi.org/10.1108/IMR-09-2014-0304.
