# General SEM Conditional Process V2

Status: additive implementation contract; not a replacement for any V1 method.

Method version: `general_sem_conditional_process_v2`

## Purpose and boundary

V2 evaluates explicitly selected conditional indirect paths. It supports
first-stage, second-stage, both-stage, multiple two-way, one bounded three-way,
longer-path, multiple-HOC, grouped, positive case-weighted and positive integer
frequency-weighted profiles only through the qualified envelopes below.

The method does not infer a path from the graph, does not change continuous
moderation V1, and does not attach a causal or counterfactual interpretation to
an associational PLS estimate. Unsupported combinations are rejected with
stable blocker codes; the engine must never drop an interaction, shorten a
path, change an interval method or remove a weight to make a request run.

## Mathematical contract

Each edge on a selected path is represented on the admitted standardized score
metric as:

```text
b_e(z) = beta_e + sum_j gamma_ej z_j + sum_{j<k} delta_ejk z_j z_k
```

For an explicitly ordered path `P`, the conditional path effect is:

```text
IE_P(z) = product over e in P of b_e(z)
```

The implementation multiplies edge functions into one canonical multivariate
polynomial. Conditional effects, first derivatives, pure second derivatives,
cross derivatives and finite probe contrasts are all evaluated from that same
polynomial. This avoids formula drift between first-stage, second-stage and
both-stage implementations.

The conventional scalar index of moderated mediation is emitted only when the
complete selected-path polynomial is affine in exactly one requested
moderator. If multiple moderators or degree-two-or-higher terms remain, the
result reports local derivatives and probe contrasts instead. They must not be
labelled as a constant Hayes index.

Every inferential target has a deterministic SHA-256 identity bound to method
version, selected path, estimand kind, moderator identities and joint probes.
Probe maps and polynomial powers are canonicalized by stable identity order.

## Qualified non-Cartesian profiles

| Profile | Structural envelope | Inference |
|---|---|---|
| `multi_two_way_percentile` | 1-8 two-way interactions; 1-8 explicit paths of 2-6 edges; at most 4 moderators | Type-7 percentile; two-sided, less or greater |
| `multi_two_way_bca` | Same structural envelope | Full delete-one BCa; all alternatives |
| `studentized` | At most 4 interactions; 1-2 paths of 2-4 edges; at most 3 moderators, 27 probes and 256 targets | Nested studentized; all alternatives |
| `bounded_three_way` | Exactly one three-way term with complete lower-order closure; at most 8 total interactions; 1-4 paths of 2-5 edges | Type-7 percentile; all alternatives |
| `multiple_hoc` | 1-4 pairwise-disjoint, nonnested HOCs; one admitted HOC approach per run; at most 2 two-way interactions; 1-8 explicit paths of 2-6 edges | Type-7 percentile; two-sided |
| `grouped` | 2-20 groups; at most 4 two-way interactions and 4 paths of 2-4 edges | Group-stratified Type-7 percentile; two-sided |
| `case_weighted` | Positive case weights; at most 4 two-way interactions and 4 paths of 2-4 edges | Type-7 percentile; two-sided |
| `frequency_weighted` | Positive integer frequencies; same structural bound | Count-space Type-7 percentile; two-sided |

The HOC approaches are repeated indicators, extended repeated indicators,
embedded two-stage and disjoint two-stage. A run uses one approach consistently
and executes HOC dependency stages before product construction. Nested, hybrid
or overlapping HOCs are outside V2.

Examples of rejected intersections include HOC plus groups, HOC plus weights,
groups plus weights, three-way plus HOC, a three-way term in a two-way profile,
or studentized inference outside its bounded profile.

## Probe and target contract

- Default standardized probe values are `-1`, `0` and `+1`.
- A request may contain at most four moderators and five values per moderator.
- Cartesian grids and explicit joint tuples are distinct input forms.
- Cartesian plans contain at most 81 joint probes; explicit plans contain at
  most 100.
- A plan may request at most 16 finite probe contrasts, 512 path-by-probe cells
  and 1,024 inferential targets.
- The studentized profile tightens these limits to 27 probes and 256 targets.
- Probe anchors are calculated from the original sample and frozen for every
  resample.
- Raw-unit probes are admitted only for an observed single-indicator moderator
  with a persisted transformation receipt. Composite and HOC probes remain on
  their declared standardized score metric.

The raw-unit receipt is not satisfied merely by recording that a transformation
occurred. It freezes the moderator and source-column identities, dataset
fingerprint, analysis-row-mask SHA-256, original-sample center, positive sample
standard deviation, score-orientation sign, retained-row mass digest, and the
fit scope. Ungrouped unweighted profiles may continue to read the legacy
sample-standardization receipt. Grouped analyses require one unweighted receipt
for every selected group. Case-weighted analyses use
`sum(w) - sum(w^2) / sum(w)` as the reliability-weight variance denominator;
frequency-weighted analyses use exact count-space moments and `sum(f) - 1`,
which is identical to physical row expansion. The only admitted conversion is
`orientation_sign * (raw - center) / standard_deviation`.

All raw anchors are calculated from the original sample and reused unchanged in
every point target, bootstrap, jackknife, and nested refit. Group-local
standardized values may therefore differ while the authored raw probe value and
target identity remain the same. The runner recomputes every identity and
numeric anchor before use; a stale dataset, row mask, source column, row mass,
group scope, or anchor fails closed.

The kernel represents a contrast as the conditional effect at the left joint
probe minus the effect at the right joint probe. The UI and exports must retain
that direction.

## Resampling summaries

Percentile intervals use R Type-7 quantiles. BCa requires the complete
delete-one jackknife; its bias correction uses half of exact bootstrap ties and
its acceleration is calculated from jackknife influence values. A constant
jackknife yields acceleration zero. There is no percentile fallback.

Studentized inference retains the observed estimate and standard error plus
each usable outer estimate and inner standard error. It inverts the outer
studentized pivots. Inner raw draws may be omitted only after their counts and
digests have been persisted. A nonfinite or nonpositive standard error makes
that replicate unusable; it is never converted to a percentile replicate.

The runner owns one deterministic replicate ledger for every target in the
analysis. No failed draw is replaced. Each required ledger must retain at least
90 percent of requested draws. The bounded studentized defaults are 1,000 outer
by 200 inner draws, with maxima of 5,000 outer, 1,000 inner and one million
inner refits.

## Weight semantics

Positive case weights are normalized to mean one for every fit and travel with
their rows during ordinary row bootstrap. The output retains the raw mean,
normalized values, Kish effective sample size and maximum/minimum ratio. It
warns when Kish ESS is below 25 percent of row count and rejects ratios above
`1e6`. These are analytic case weights, not sampling/survey weights.

Frequency weights are integers at least one with total at most `2^53`. All
estimation is count-space algebra equivalent to physically expanding a row its
frequency number of times. Bootstrap draws use a multinomial with total equal
to the expanded count and probabilities `f_i / sum(f)`, without allocating the
expanded dataset. Sampling weights, clusters, strata, PPS and survey variance
claims are not supported.

## Required persisted result

The canonical result must retain:

- profile and method versions;
- explicit path and relation identities;
- edge coefficients and the canonical path polynomial;
- frozen moderator anchors and joint probes;
- conditional effects, eligible scalar index, derivatives and contrasts;
- alternative, interval method and confidence level;
- shared replicate-ledger identity, requested/usable counts and failures;
- weight, HOC or group receipts when applicable;
- stable target IDs and blocker codes;
- input, model, result and sidecar digests.

Large target vectors, validity bitmaps, BCa jackknife summaries and
studentized outer estimates/standard errors/pivots belong in checksummed V6
Arrow sidecars. Missing, duplicate, altered or identity-mismatched sidecars
must prevent reopening the result.

## Qualification requirements

Qualification must independently cover every profile rather than treating the
table as a Cartesian capability claim. Minimum evidence includes:

- hand-calculated polynomial, derivative and contrast identities;
- independent reference reproduction for first-, second- and both-stage paths;
- null and non-null interaction/index simulations;
- one bounded three-way and complete hierarchy fixtures;
- 2-6 edge paths, multiple outcomes and the same moderator on multiple stages;
- multiple disjoint HOC scheduling fixtures;
- 2-, 3-, 5- and 20-group stratified resampling fixtures;
- case-weight scaling invariance and Kish ESS boundaries;
- exact expanded-row equivalence for frequency weights;
- Type-7, BCa and nested-studentized reference fixtures;
- two-sided and predeclared one-sided alternatives where admitted;
- row/order/sign/seed/worker invariance and shared-ledger verification;
- save/reopen, tamper rejection and semantic table export;
- legacy moderated-mediation and continuous-moderation golden regressions.

The registry may advertise a profile only after its live manifest is bound to
the exact source and installed-application evidence hashes.
