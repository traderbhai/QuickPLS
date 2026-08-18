# PLS MGA Permutation v3 (historical)

This replacement-retry identity is archive-readable only. New combined executions use
`pls_mga_permutation_v4`; see `MICOM_MGA_V4.md`.

Status: implemented and source-tested; coordinated-build evidence remains pending.

Permutation MGA v3 reports explicit Group A minus Group B path, outer-loading, and outer-weight differences. Every usable permutation re-estimates both group PLS models. The two-tailed probability is

`p = (1 + count(|d_b| >= |d_observed|)) / (B + 1)`.

The v3 permutation plan canonicalizes the unordered pair of selected group values and the pooled row order before shuffling. A/B exchange complements the same seeded label vector. Consequently each attempted partition, successful pair, failed pair, and retry index maps exactly to the opposite direction; signed effects reverse and two-tailed probabilities remain identical without relaxing numerical tolerances.

Supported scope is limited to the coupled `micom_v3` workflow, 5,000–10,000 usable permutations, path weighting, standardized/listwise data, two selected groups, and the documented path/loading/weight tables. Parametric MGA, bootstrap MGA, one-tailed claims, omnibus multigroup tests, multiplicity adjustment, case weights, interactions, higher-order constructs, and more than two simultaneous groups are excluded.

`pls_mga_permutation_v2` remains readable as historical output. Its same-seed stream was not invariant to selected A/B order, so it is not current evidence and is never relabeled as v3.
