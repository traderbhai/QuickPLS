# QuickPLS Multigroup MGA v1

Status: additive Internal/Labs kernel. It is not a replacement for the current
`pls_mga_two_group_v4`, `pls_mga_permutation_v4`, or `micom_v4` contracts and
must not be routed or promoted until the Recipe V4 adapter, MICOM layer,
persistence, exports, native workflow, independent references, simulations,
and installed-app evidence all qualify the exact integration commit.

The Rust calculation foundation is
`crates/qpls-estimation/src/multigroup_v1.rs`, with kernel identity
`mga_multigroup_kernel_v1`.

## Responsibility boundary

The kernel receives rows only after the upstream adapter has:

1. selected one grouping column that is not a model indicator;
2. selected 2–20 mutually exclusive observed group values;
3. applied the model's frozen complete-case rule;
4. retained excluded row identities and reasons outside the kernel;
5. compiled and qualified exactly one admitted model profile; and
6. frozen a stable, ordered parameter identity vector.

The kernel never reads a dataset, compiles a PLS model, aligns construct signs,
computes MICOM, interprets measurement invariance, or changes a model to make a
fit succeed. A profile adapter supplies every complete model refit through
`MultigroupRefitterV1`. A request includes the fit kind, typed group index,
replicate index, and exact source-row tokens. Repeated tokens mean a case
bootstrap sample. The callback must return every frozen parameter exactly once
under the same stable identity and family.

This boundary allows ordinary PLS, interactions, HOCs, case weights, frequency
weights, and PLSc to own their distinct scientific refit semantics without
putting a false common estimator inside the MGA scheduler. Each profile remains
a separately qualified capability cell. Unsupported profile intersections fail
before entering this kernel.

## Typed groups and eligibility

`GroupIndexV1` is checked, zero based, and bounded to indices 0–19. Selected
group values retain their source type:

- text;
- boolean;
- integer; or
- exact finite IEEE-754 numeric bits.

Consequently text `"1"`, integer `1`, numeric `1.0`, and boolean `true` do not
collapse into one group. Group indices must be contiguous and match the frozen
group order. Source-row tokens must be unique before resampling.

Eligibility rules are:

- 2–20 groups;
- at least 10 complete model cases per group;
- warning below 30 cases in any group;
- warning when largest/smallest complete-case size is greater than 2:1; and
- blocker when the ratio is greater than 10:1. Exactly 10:1 remains admissible.

The assessment returns stable blocker/warning enums and group counts. It never
drops, merges, stringifies, or reassigns a row.

## Deterministic resampling contract

All random streams use ChaCha20 with a SHA-256-derived seed containing:

- `quickpls/mga_multigroup_v1`;
- the exact operation domain;
- the master seed;
- the zero-based requested replicate index; and
- the relevant typed group positions.

The default seed is 42. Permutation and MGA bootstrap requests are 5,000 by
default and must be within 5,000–10,000. Every index is attempted exactly once.
There are no retries, replacements, fallback estimators, clamping of model
outputs, or extra draws. Inference requires

`max(1,000, ceil(0.90 * requested))`

usable draws. If the threshold is missed, the ledger remains available but the
inferential parameter table is empty and explicitly marked unavailable.
Successful observed group estimates and signed descriptive differences remain
visible; they are never converted into inferential claims.

Every requested index retains:

- its exact zero-based index;
- the partition or sample SHA-256 digest;
- usable/failed status for every required group fit; and
- a stable failure code plus diagnostic detail.

The plan digest hashes the method identity, index order, and every per-index
digest. Worker scheduling must not affect any scientific payload.

### Deterministic resumable execution

The runner freezes a top-level `qpls.mga.execution_plan.v1` before expensive
work. Its identity binds the compiled analytical identity, dataset fingerprint,
typed group design and selected rows, publishable parameter inventory, complete
MGA configuration, selected ordered comparisons, seed, draw counts, alternatives,
and multiplicity method. The plan has immutable shards for:

- each group point fit;
- each MICOM pair and pairwise permutation;
- the shared group-bootstrap bank and each selected derived bootstrap procedure;
- max-spread omnibus permutation;
- each qualified pooled/Welch pair and the optional K-group Wald cell; and
- final multiplicity aggregation over the exact unadjusted-row digest.

`prepare_compiled_raw_mga_execution_plan_v1` builds this graph from the raw
dataset/model/recipe authority. `run_compiled_raw_mga_resumable_v1` executes it
through an external `qpls.mga.execution_cache.v1`. A coordinator may persist the
plan and cache atomically outside the scientific result archive, reconstruct the
plan on reopen, and accept the cache only when its plan identity and every
payload SHA-256 validate. Completed payloads are immutable and reused. A failed
or cancelled shard is not committed; resuming that unfinished shard consumes the
same frozen draw stream and never replaces failed draws.

No partial cache is a scientific result. The runner checks that every planned
shard, including multiplicity aggregation, is present and valid before returning
the public analysis or its finalized cache identity. A cancellation therefore
may preserve recoverable external work without publishing partial estimates.

### Ordered pairwise permutation

For ordered groups A and B, the observed statistic for parameter `j` is

`D_j = theta_Aj - theta_Bj`.

Each label permutation uses only the selected A/B rows and preserves both group
sizes. Its stream is canonical for the unordered pair. Reversing A and B
therefore uses the same partitions, reverses `D`, swaps the one-sided tails,
and preserves the two-sided probability.

For `B_ok` usable partitions, add-one probabilities are:

- two-sided: `(1 + count(|D*_j| >= |D_j|)) / (B_ok + 1)`;
- greater: `(1 + count(D*_j >= D_j)) / (B_ok + 1)`; and
- less: `(1 + count(D*_j <= D_j)) / (B_ok + 1)`.

Two-sided is the default. `greater` or `less` must be explicitly predeclared.

### K-group max-spread omnibus permutation

For 3–20 groups, the parameter-wise observed omnibus statistic is

`T_j = max_g(theta_gj) - min_g(theta_gj)`

which equals `max_(g<h) |theta_gj - theta_hj|`.

Every global permutation shuffles the complete typed label multiset across all
selected rows and therefore preserves every group size simultaneously. The
right-tailed probability is

`(1 + count(T*_j >= T_j)) / (B_ok + 1)`.

The omnibus result is the confirmatory first stage for three or more groups.
Selected pairwise follow-ups are separate tests and enter the declared
multiplicity family; a significant omnibus result does not manufacture pairwise
significance.

## Group bootstrap and PLS-MGA probability

Each group has an independent, domain-separated case-bootstrap stream, while
all streams retain the same requested replicate index. A pairwise calculation
uses only indices at which both required group fits succeeded.

For matched bootstrap difference
`D*_b = theta*_A,b - theta*_B,b`, the Henseler directional result is the
empirical probability

`P_A>B = (N(D*>0) + 0.5 N(D*=0)) / B_ok`.

This quantity is deliberately named `directional_probability_a_greater`; it is
not exported as a conventional two-sided p-value. At alpha `.05`, the bounded
decision is A lower when `P_A>B <= .05`, A higher when `P_A>B >= .95`, and not
significant otherwise. Reversing the ordered labels returns exactly
`1 - P_A>B` and reverses the signed point difference.

## Bias-corrected group intervals

Group-specific bootstrap intervals use Type-7 quantiles and Efron's
bias-corrected transform with acceleration fixed to zero. For point estimate
`theta` and finite bootstrap draws:

1. rank draws below `theta`, giving ties half weight;
2. apply the finite continuity correction `(+0.5)/(B+1)`;
3. let `z0` be the inverse-normal corrected rank;
4. transform nominal tail probability `p` to `Phi(2 z0 + Phi^-1(p))`; and
5. evaluate the empirical Type-7 quantile at both transformed tails.

The result persists `z0`, adjusted probabilities, and `acceleration = 0`. Its
method is **BC**, never BCa. Full BCa would require a separately retained and
qualified jackknife acceleration contract.

## Parametric sensitivity cells

Parametric procedures are separate, score-conditional sensitivity cells. For
ordinary General SEM PLS structural paths, QuickPLS recomputes the centered
structural equation with an implicit intercept on each group's estimated
scores, verifies that the coefficient matches the fitted path, and retains the
equation design receipt. These cells describe classical homoskedastic OLS
uncertainty conditional on the estimated scores; they are not full PLS
uncertainty and are not substitutes for label permutation, bootstrap MGA, or
MICOM.

### Equal-underlying-variance pooled t

For group `g`, let `RSS_g` be the structural equation residual sum of squares,
`p_g` the number of predictors including controls, and
`df_g = n_g - p_g - 1`. For coefficient `j`, retain
`q_gj = [(X_g' X_g)^(-1)]_jj` from the centered design, so that

`se_gj^2 = (RSS_g / df_g) q_gj`.

The equal-underlying-variance estimate is

`s_p^2 = (RSS_A + RSS_B) / (df_A + df_B)`.

For the signed `A - B` contrast,

`SE(D_j) = sqrt(s_p^2 (q_Aj + q_Bj))`,


### Welch-Satterthwaite t

For unequal estimator variances:

`SE(D) = sqrt(se_A^2 + se_B^2)`

and

`df = (se_A^2 + se_B^2)^2 /
      (se_A^4/df_A + se_B^4/df_B)`.

Both tests report two-sided, greater, and less Student-t probabilities, while
the selected alternative is frozen separately.

### K-group inverse-variance Wald

For independent estimates with positive qualified SEs, weights are
`w_g = 1/se_g^2`, the common estimate is
`theta_bar = sum(w_g theta_g)/sum(w_g)`, and

`Q = sum(w_g (theta_g - theta_bar)^2)`.

The right-tailed reference is chi-square with `K-1` degrees of freedom. This is
an across-group parameter heterogeneity sensitivity test; it is not an omnibus
MICOM test.

## Multiplicity

One explicitly declared family of finite raw probabilities can use:

- Holm step-down FWER, the confirmatory default;
- Bonferroni single-step FWER;
- Sidak single-step FWER;
- Benjamini-Hochberg step-up FDR, labelled exploratory; or
- no adjustment, which must remain explicit.

Stable hypothesis identity breaks probability ties deterministically. Holm and
BH enforce their required monotonic adjusted values. Unavailable hypotheses are
not silently assigned `p=1`; the integration layer must persist the admitted
family and explain exclusions.

## MICOM and interpretation gate

MICOM is intentionally outside this parameter-generic kernel. The Recipe V4
adapter must attach one structured Step-1 configural-review checklist and
pairwise Step-2/Step-3 MICOM results generated by the qualified model-specific
measurement adapter. There is no invented K-group omnibus MICOM result.

Numerical group differences remain visible for diagnosis, but substantive path
interpretation is blocked whenever a required construct lacks at least partial
measurement invariance. For PLSc the consistent correction and consistent
permutation must be repeated inside every relevant refit and labelled
measurement invariance of composites.

## Profile and product exclusions

This kernel does not itself qualify the planned model-profile breadth. The
integration must keep ordinary PLS, multiple two-way, bounded three-way,
bounded moderated mediation, multiple disjoint HOCs, case weights, integer
frequency weights, and PLSc as non-Cartesian profiles with stable blockers for
inadmissible combinations.

Frequency-weight resampling must be proven equivalent to expanded rows without
material expansion. Case weights must retain their separately frozen positive
weight semantics. Survey, sampling, PPS, cluster, and strata claims remain
unsupported.

## Required qualification before promotion

Qualification builds retain bounded raw audit distributions without changing
the canonical MGA estimates. For every selected pair, the first stable target
retains its usable permutation null differences. Every MICOM construct retains
its Step-2 compositional-correlation null series, while the first stable
construct additionally retains Step-3 mean-difference and log-variance-ratio
series. Every selected omnibus target retains its usable null maximum-spread
series. Pairwise, omnibus, and bootstrap target vectors use trusted,
dictionary-backed Arrow `TargetLedger` tables. MICOM null statistics instead
store a typed construct ordinal and statistic-kind code linked to the sibling
construct table, so construct IDs are not repeated for every permutation.
These tables allow an independent comparator to reproduce all three pairwise
permutation tails, omnibus right tails, MICOM lower quantiles and decisions,
representative Henseler probabilities, and published multiplicity-adjusted
probabilities. The preflight uses the same compact row layouts, includes the
omnibus null vectors, warns above 128 MiB, and rejects above 512 MiB before any
point fit starts.

- Independent equation-level implementation of partitions, probabilities,
  BC intervals, pooled/Welch/Wald statistics, and every adjustment.
- Hand fixtures for 2, 3, 5, and 20 groups.
- Exact group-label reversal and row-order invariance.
- Balanced, 2:1 warning, exactly 10:1, over-10:1, missing-row, non-normal, and
  small-group boundaries.
- Null type-I-error and non-null power simulations for each inferential cell.
- Failure campaigns proving the 90% boundary and no replacement draws.
- Seed, worker-count, shard, cancel/resume, and save/reopen determinism.
- Semantic CLI/GUI/export parity and complete exclusion/failure ledgers.
- Archive tamper rejection and exact source-bound manifest evidence.
- Installed and portable fully offline Windows acceptance.
- Golden regression evidence for current continuous moderation and the legacy
  two-group MGA/MICOM V4 payloads.

## Scientific sources

- Henseler, Ringle, and Sinkovics (2009), *The Use of Partial Least Squares Path
  Modeling in International Marketing*, DOI `10.1108/S1474-7979(2009)0000020014`.
- Sarstedt, Henseler, and Ringle (2011), *Multigroup Analysis in Partial Least
  Squares Path Modeling: Alternative Methods and Empirical Results*.
- Henseler, Ringle, and Sarstedt (2016), *Testing Measurement Invariance of
  Composites Using Partial Least Squares*, DOI `10.1108/IMR-09-2014-0304`.

Product terminology may be compared with the public SmartPLS MGA and MICOM
documentation, but QuickPLS qualification must use independent equations,
fixtures, and simulations rather than another product's output as its sole
scientific oracle.
