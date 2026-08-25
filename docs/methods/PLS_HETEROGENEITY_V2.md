# PLS unobserved heterogeneity V2

Status: additive Labs implementation and frozen method specification. Recipe V4
contracts, compilation, raw-data point execution, discovery locking, fixed-K
bootstrap orchestration, and typed result/evidence families are integrated.
This is not a release-qualification claim until persistence, native UI, exports,
independent references, simulations, manifests, and installed-Windows evidence
are bound to the same final source identity.

This specification defines four separate identities:

- `qpls.fimix-pls.v2`;
- `qpls.pls-pos.published.v2`;
- `qpls.pls-pos.destination-scored-interactions.v2`; and
- `qpls.pos-common-metric-comparability.v1`.

They do not alter `fimix_pls_v1`, `pls_pos_v1`, or
`pls_pos_bounded_v1`. Those historical routines remain bounded score-space
diagnostics and must retain their existing serialization and wording.

## 1. Shared scientific boundary

The V2 engine accepts a fixed analyst-selected class or segment count from two
through five. `K=1` is a separately calculated pooled reference; it is not sent
through the segmentation engine. QuickPLS never auto-selects K and never repeats
model selection inside a bootstrap replicate.

Three non-Cartesian interaction profiles are identified:

- `p0_structural`: no generated interactions;
- `p2_multi_two_way`: one or more already-qualified TwoStage + Strong two-way
  interactions; and
- `p23_all_current`: exactly one bounded three-way interaction with all of its
  pairwise and main-effect hierarchy, plus any other qualified two-way terms.

Fourth-order and arbitrary k-way products are outside this version.

FIMIX receives one or more row-aligned structural equations. Every supplied
predictor and outcome column must have been sample-standardized once over the
same pooled complete-case rows. For P2/P23, product terms must also have been
constructed and standardized once on those pooled rows. The input contains a
metric identity, source digest, observation count, and explicit pooled-score and
pooled-product receipts. The kernel verifies dimensions, finite values, zero
sample means, and unit sample standard deviations before fitting.

This makes all class-specific coefficients, including gamma and delta product
coefficients, comparable on one metric. The compiler remains responsible for
mapping the generic coefficient IDs back to paths, interactions, and probe
slopes.

## 2. Genuine FIMIX-PLS V2

### 2.1 Likelihood

For respondent `i`, class `k`, and structural equation `q`, let `y_iq` be the
pooled standardized outcome, `x_iq` its pooled standardized design row,
`beta_kq` the class coefficient vector, and `sigma2_kq` the class residual
variance. Conditional independence of structural disturbances gives

```
log f_ik = log(pi_k)
           - 1/2 * sum_q [log(2*pi) + log(sigma2_kq)
                          + (y_iq - x_iq beta_kq)^2 / sigma2_kq]
```

and the optimized observed-data likelihood is

```
ell = sum_i log(sum_k exp(log f_ik)).
```

The E-step uses a maximum-shifted log-sum-exp. It returns the complete N by K
posterior matrix, not inverse-distance membership scores.

The M-step uses the same posterior weights for every structural equation:

```
pi_k       = sum_i tau_ik / n
beta_kq    = argmin_beta sum_i tau_ik (y_iq - x_iq beta)^2
sigma2_kq  = sum_i tau_ik residual_iq^2 / sum_i tau_ik.
```

Weighted regression is solved by twice-reorthogonalized modified Gram-Schmidt
QR. No ridge penalty, pseudo-inverse, variance clamp, or silent class repair is
used.

### 2.2 Defaults and admissibility

- K: 2 through 5, fixed before estimation.
- Seed: 42.
- Starts: 30; allowed range 10 through 100.
- Maximum EM iterations: 5,000.
- Relative likelihood tolerance: `1e-10` for three consecutive iterations.
- Likelihood-decrease tolerance: `1e-9` relative to likelihood scale; a material
  decrease fails that start.
- Residual-variance floor: `1e-8`; reaching it fails that start.
- Weighted-QR rank tolerance: `1e-11` times square root of effective class size.
- Minimum effective class size:
  `max(20, ceil(.05*n), max_q(number_of_predictors_q + 2))`.
- Every class share must remain at least .05.
- `K * minimum_effective_class_size <= n`.
- The complete FIMIX parameter count must be smaller than n.

Each start begins from a deterministic balanced partition. Start zero uses a
stable pooled structural projection; subsequent starts use domain-separated
ChaCha20 seeds and balanced shuffles. A start fails, rather than being repaired,
for a collapsed class, residual-variance collapse, rank deficiency, nonfinite
quantity, material likelihood decrease, or failure to converge by the iteration
limit. Successful and failed starts retain typed diagnostics and histories.

### 2.3 Multistart stability and labels

Classes are first canonically ordered by their complete finite parameter
signatures. Candidate solutions close to the best likelihood are then aligned
to the best hard partition by exact K! maximum-overlap assignment.

The optimum is stable only when at least two starts reproduce it within all of:

- relative log-likelihood difference `1e-8`;
- maximum aligned structural coefficient difference `1e-6`; and
- mean aligned posterior difference `1e-4`.

If fewer than two starts reproduce the optimum, the run fails. Class numbers are
still arbitrary output labels; they are not substantive respondent types.

Every completed start additionally retains a validation-only reproducibility
receipt. The receipt contains its canonical hard partition, structural
coefficient signature, complete posterior matrix, final likelihood, and
ordered SHA-256 digests. This is the minimum evidence from which an independent
verifier can exhaustively realign labels and recompute all three tolerances;
the posterior matrix cannot be replaced by a trusted mean-difference scalar.
The selected scientific estimates remain unchanged. Engine, runner, and Arrow
sidecar boundaries each reject missing, duplicate, nonfinite,
dimension-inconsistent, digest-inconsistent, or selected-solution-inconsistent
receipts.
Arrow tables store the immutable source-row token once in the selected
posterior/membership table; completed-start tables join to that required table
by checked row ordinal instead of duplicating the token for every start and
class cell.

### 2.4 Retained estimates and criteria

The result retains:

- class proportions;
- class/equation coefficient vectors and residual variances;
- effective class sizes;
- complete posterior probabilities and dominant assignments;
- observed-data log likelihood;
- selected-start and all-start convergence histories;
- raw entropy and normalized classification certainty; and
- parameter count, AIC, AIC3, AIC4, BIC, CAIC, and HQ.

Class proportions are the final accepted M-step parameters used by the final
E-step and observed-data likelihood. Effective class sizes are the column sums
of that E-step's posterior matrix. With finite likelihood-based convergence,
the two probability families are each normalized but a class proportion need
not equal its resulting posterior effective size divided by `n` exactly.
Changing the proportion after the E-step would make the published parameter
vector inconsistent with the retained likelihood and posteriors.

For parameter count `p`, sample size `n`, and optimized likelihood `ell`:

```
p     = (K - 1) + K * sum_q(number_of_coefficients_q + 1 variance)
AIC   = -2ell + 2p
AIC3  = -2ell + 3p
AIC4  = -2ell + 4p
BIC   = -2ell + p log(n)
CAIC  = -2ell + p [log(n) + 1]
HQ    = -2ell + 2p log(log(n)).
```

Normalized certainty is
`1 - [-sum_i sum_k tau_ik log(tau_ik)] / [n log(K)]`.

## 3. PLS-POS V2

### 3.1 Publication-faithful P0 identity

`qpls.pls-pos.published.v2` admits P0 only. Its objective is the unweighted sum
of every endogenous construct's in-sample R-squared over every segment:

```
Q(G) = sum_g sum_(endogenous j) R2_gj.
```

Every segment outcome score is centered independently before its denominator
is calculated: `SST_gj = sum_i (y_igj - mean(y_gj))^2`. The retained outcome
audit stores that mean, centered SST, source-row identities, observed scores,
and fitted scores. Point execution, sidecar encoding, and the independent
comparator each reconstruct this identity; an origin-based `sum(y^2)`
denominator is not admitted even when numerical centering would make it close.

Every candidate allocation must perform a complete segment-specific PLS refit,
including measurement-score estimation, deterministic score orientation, and
all structural equations. The PLS adapter supplies a complete ordered parameter
signature and an auditable refit receipt. Missing outcomes, inconsistent
parameter dimensions, duplicate outcomes, nonfinite parameters, invalid R2, or
an incomplete receipt invalidate that candidate.

The frozen ten-start plan uses a deterministic hybrid coverage bank: one pooled
projection presegmentation, balanced feature-axis quantile presegmentations,
and at least two domain-seeded balanced shuffles. Generated partitions are
unique up to label permutation. When a same-K FIMIX result is available, its
dominant partition is the sole FIMIX start at position ten and any equivalent
generated partition is excluded; otherwise all ten starts come from the hybrid
bank. A bounded deterministic fallback fails closed if ten label-distinct
starts cannot be formed.

For each start, QuickPLS performs strict best-improvement hill climbing:

1. inspect every permissible respondent-to-destination move;
2. preserve the configured minimum segment size;
3. fully refit the candidate partition;
4. select the greatest strict full-objective improvement, breaking numerical
   ties by respondent and destination index;
5. apply one move and repeat the complete search; and
6. stop only after a full sweep finds no strict improvement.

The accepted-move cap is `max(1000, 2*n)`. Reaching the cap before a complete
no-improvement search fails that start. A candidate full-refit failure also
fails that start before it can be described as a complete no-improvement
sweep. Its respondent, source segment, destination segment, and reason are
retained as typed evidence; failed candidates are not silently skipped or
replaced by approximate fits.

At least two starts must reproduce the same canonical partition, objective, and
ordered parameter signatures. Objective histories must be strictly increasing.
Every completed start retains its canonical partition, ordered segment
parameter signatures, final objective, and ordered SHA-256 receipts. The
independent comparator recomputes the exhaustive alignment, exact-partition
gate, objective tolerance, parameter tolerance, and complete reproducing-start
inventory rather than trusting the published inventory.

### 3.2 Destination-scored interaction extension

P2/P23 use the separate identity
`qpls.pls-pos.destination-scored-interactions.v2`. It must never be labelled
unmodified or publication-faithful PLS-POS.

For every candidate move and destination segment, its refit receipt additionally
proves that the adapter:

- reran interaction stage one;
- restandardized operands within the destination segment;
- rebuilt destination products;
- refitted all joint lower-order and interaction equations; and
- recalculated the exact full PLS-POS objective.

The same ten-start, minimum-size, strict-improvement, full-depth, canonical-label,
and two-start stability rules apply.

## 4. Exact label alignment

The shared alignment function accepts two complete partitions and K from two
through five. It constructs the K by K overlap table and evaluates all K!
candidate-to-reference mappings.

It returns the exact maximum-overlap mapping, overlap matrix, matched count,
match share, tie/ambiguity flag, and mutual-majority flag. A bootstrap replicate
is inferentially usable only when the optimum mapping is unique and every
matched pair is a strict majority in both its candidate and reference class.
Ties and non-majority matches fail the replicate; label numbers are never
matched naively.

## 5. Fixed-K bootstrap ledger

The shared V2 bootstrap contract supports FIMIX, published PLS-POS, and the
destination-scored interaction extension.

- Bootstrap replicates: 500 through 10,000.
- Interactive default: 1,000.
- Publication preset: 5,000.
- Confidence: .95 by default.
- Intervals: two-sided Type-7 percentile.
- K and algorithm remain fixed.
- Each replicate reruns the complete relevant multi-start pipeline.
- Each replicate index has one SHA-256-domain-separated seed and one attempt.
- There are no replacement retries.
- At least `ceil(.90 * requested)` replicates must be usable.

The qualification fixture sizes are deliberately typed by purpose. The
general n=400 matrix is retained for point estimation, recovery, power, and
boundary evidence; it is not the runtime fixture for the seven bootstrap
ledgers. Those ledgers use
`qpls.multimod.heterogeneity.k2-fixed-bootstrap-n80-dual-outcome.v1`: balanced
n=80, fixed K=2, 40 cases per true class, and exactly 500 requested replicates.
Its interaction fixture design is `dual_endogenous_anchor_v1`, its
purpose is `fixed_k_full_pipeline_bootstrap_inference_and_ledger_qualification`,
and its performance scope is explicitly
`n80_fixed_k_bootstrap_not_a_500_draw_n400_runtime_claim`.

P0 bootstrap fixtures remain single-outcome models with endogenous `y` only.
P2, P23, and the P2 common-metric failure fixture model both `y` and the
auxiliary endogenous anchor `v`. The anchor has the three ordinary paths
`x -> v`, `z -> v`, and `w -> v`; every two- or three-way interaction remains
on `y` only. The auxiliary equation improves a fixed bootstrap fixture's
segment anchor without changing the scientific interaction targets.
The negative-control common-metric fixture adds an X location separation for
structural identifiability and suppresses the class-two X3 loading. This keeps
the intended X compositional-invariance failure explicit without introducing
the pooled-score variance collapse caused by a full sign reversal.

A usable ledger row requires a finite fit statistic, a reproducible
target-payload digest, and unique mutual-majority label alignment. The digest
is SHA-256 over the versioned domain
`quickpls:heterogeneity:target-payload:v2\0`, a little-endian `u64` target
count, and each ordered finite IEEE-754 `f64` bit pattern in little-endian
form. Reopening and qualification rebuild that digest from the retained target
vectors. They also exhaust all K! mappings from the retained overlap table and
recompute the mapping, matched count/share, ambiguity, and mutual-majority
decision rather than trusting stored flags. Typed failures include fit
failure, ambiguous labels, lack of mutual majority, common-metric failure,
nonfinite targets, and cancellation. Missing indices or cancellation prevent a
qualified result rather than publishing a partial distribution.

The bootstrap orchestrator—not this point/search kernel—owns row resampling,
full-pipeline reruns, target-vector persistence, and Type-7 quantiles.

## 6. POS common-metric comparability gate

Destination-scored segment coefficients are local descriptive quantities until
the separately identified `qpls.pos-common-metric-comparability.v1` gate passes.

After segmentation is fixed, the inference adapter must:

1. create one pooled measurement/scoring metric;
2. apply that exact metric to every aligned segment;
3. prove configural identity for every construct used by a requested contrast;
4. supply every pairwise segment compositional-invariance result for those
   constructs; and
5. retain Step 3 mean/variance equality results descriptively.

Missing/duplicate construct evidence, missing/duplicate pair evidence,
configural failure, compositional-invariance failure, or a non-common metric
sets the gate to `descriptive_only`. Gamma, delta, slope, and conditional-effect
differences must then be suppressed. Step 3 failures are counted and displayed
but do not block standardized coefficient comparison once Steps 1 and 2 pass.

The retained gate is valid only when the required-construct set is identical in
the gate input, derived result, every MICOM pair, and the pooled common-metric
parameter family. For K segments, every required construct must carry exactly
all `K*(K-1)/2` unordered compositional and Step-3 pairs, and MICOM must carry
exactly that same pair inventory. Missing, duplicate, extra, or mismatched
construct/pair evidence fails sidecar publication.

Every POS interaction bootstrap replicate repeats segmentation, pooled
common-metric refitting, comparability evaluation, and exact label alignment.

## 7. Interpretation boundary

FIMIX and PLS-POS are exploratory heterogeneity procedures. A forced K,
in-sample objective improvement, low information criterion, high entropy, or
stable recovered partition does not establish that the population contains
substantively real classes, that K is correct, that results generalize, or that
class differences are causal. Tandem FIMIX/POS runs remain separate result
objects; cross-tabulation and adjusted Rand index are descriptive only.

This V2 profile does not combine segmentation with observed groups, case or
frequency weights, higher-order constructs, nested/hybrid HOCs, PLSc, survey
design, or automatic class-count selection. Such combinations fail closed
unless separately versioned and qualified.

## 8. Integrated Labs boundary

The additive implementation now spans the core Recipe V4 compiler,
`qpls-estimation`, and the raw-data runner. It compiles pooled score/product
receipts without V1 metadata, performs complete destination refits through the
`PlsPosFullRefitterV2` contract, publishes a non-selectable pooled K=1 baseline,
requires a discovery-bound analyst lock before fixed-K inference, and suppresses
POS segment contrasts unless the separately named pooled common-metric
comparability gate passes. The publication-faithful
`qpls.pls-pos.published.v2` identity remains distinct from the
`qpls.pls-pos.destination-scored-interactions.v2` extension.

The remaining integration and qualification responsibilities are:

1. qualify save/reopen, role/schema identity, cost-cap, and tamper rejection for
   the implemented archive-V6 posterior, membership, start-trace, bootstrap,
   target-vector, common-metric, and validity sidecars;
2. finish native discovery/lock/results flows and semantic exports against the
   typed result identities;
3. retain all new capability cells as Labs/absent until independent references,
   simulations, persistence/export, accessibility, performance, and
   installed/portable offline gates pass against the exact final commit; and
4. preserve byte-compatible V1 recipes, results, archives, runner behavior, and
   method naming.

No new crate dependency is required. V1 contracts and method identities remain
unchanged.

## 9. Qualification matrix

Validation receipts may add bounded, noncanonical audit tables. FIMIX retains
the exact pooled standardized equation design and outcome vectors so an
independent implementation can recompute log-sum-exp likelihood, posterior
probabilities, and criteria from class proportions, coefficients, and
variances. Final PLS-POS fits retain source-row identities plus observed and
fitted endogenous score rows so each segment R-squared and the unweighted
objective can be reconstructed without trusting the published objective
contributions. These fields do not change V1 wires or canonical V2 estimates.

Before release qualification, independent automation must cover:

- hand-reproduced one- and multi-equation EM identities;
- log-sum-exp extremes and posterior row sums;
- monotone likelihood and weighted-M-step equivalence;
- two through five classes, 10/30/100 starts, and fixed-seed determinism;
- rank, variance, collapsed-class, nonfinite, likelihood-decrease, and
  maximum-iteration failures;
- information-criterion parameter counts and entropy identities;
- strong-separation recovery with median ARI at least .80, homogeneous nulls,
  overlap, imbalance, and weak separation;
- P0, P2, and P23 gamma/delta/probe recovery on the pooled FIMIX metric;
- a literal independent PLS-POS implementation of the published reassignment
  algorithm and full R2 objective;
- strict objective monotonicity, full candidate-refit receipts, local-optimum
  multistarts, and move-cap failure;
- exact label-permutation invariance and ambiguous/non-majority bootstrap
  rejection;
- common-metric pass/fail fixtures, including non-blocking Step 3 failures;
- fixed-K 500/1,000/5,000/10,000 bootstrap ledgers and the 90% usability gate;
- save/reopen and missing, altered, duplicated, or identity-mismatched sidecar
  rejection;
- semantic CSV/XLSX/JSON exports and installed/portable offline Windows flows;
  and
- byte-compatible V1 recipes, results, archives, and continuous moderation.

The production qualification receipt exposes every completed FIMIX and PLS-POS
start's retained values and digests. Its independent Python comparator rebuilds
the digests, exhaustively aligns each start to the selected partition, and
recomputes the exact reproducing-start inventory and configured tolerances. A
release claim still depends on the complete source-bound campaign and live
manifest gates described above.

The validation-only producer is checkpointed rather than monolithic. After one
Cargo build, a fast raw-runner sentinel must pass before independent recovery,
scenario, candidate-K, POS-discovery, boundary, and bootstrap shards may run.
Every shard is bound to the exact plan, commit, executable, seed, scale, and
dependency receipts and is atomically sealed with SHA-256. Fixed-K inference
reuses the retained discovery identity from its prerequisite shard. A discovery
prerequisite is admissible only when the exact algorithm/K candidate is
`converged_stable`, has at least two reproducing starts, has no blocker, and has
one matching retained point-result receipt. Otherwise the discovery shard fails
before any dependent bootstrap work starts. Independent
non-Cargo executable shards may run concurrently, while bootstrap concurrency
is separately bounded. On a 12-logical-core qualification host, POS bootstrap
execution permits at most four producer processes with three threads each.
Each retained bootstrap cell freezes one serialized
prepared point/common-metric execution, then runs the exact 500-draw ledger as
100 modulo caches with five owned draws per cache. A process resumes only its
same validated cache, has a 25-minute internal cancellation budget within the
30-minute external child bound, and must add at least one verified record.
Finalization requires all 100 complete caches and the global 90% usable gate.
Prepared executions and caches are immutable generation files; their payload
and receipt are published by one atomically replaced current-generation
pointer. Orphan generations left before that switch are ignored, while a bad
current pointer fails closed. One relative-path cache-inventory manifest binds
all 100 current generations to the exact commit, executable, plan,
dependencies, prepared execution, and cache identities before Rust
finalization. Missing files, tampering, or mixed identities fail closed. Deterministic aggregation recreates
the same raw report shape consumed by the independent comparator; partial
output can never become a scientific result. The wrapper requires a fully clean tracked/untracked tree
and baseline, one-worker, non-compact, unsigned fixture environment; the
comparator separately enforces both the 400-row seed-42 point/recovery identity
and the typed balanced n=80 fixed-K dual-outcome bootstrap identity.
Cargo build, plan generation, sentinel, all shards, aggregation, and comparison
are supervised against the remaining budget with process-tree termination and
bounded exit waits. A two-minute sentinel, 108-minute internal work cutoff
inside the 110-minute outer gate, and 30-minute per-shard cap retain valid checkpoints for exact-identity resume
instead of allowing an unbounded, uncheckpointed run.

The production bootstrap matrix retains seven distinct 500-draw K=2 profile
cells: FIMIX P0/P2/P23, publication PLS-POS P0, destination-scored PLS-POS
P2/P23, and the common-metric failure fixture. All seven use the typed
`qpls.multimod.heterogeneity.k2-fixed-bootstrap-n80-dual-outcome.v1` plan,
balanced n=80 (40 cases per true class), and rerun the complete stage-one, product,
multi-start, segmentation, common-metric-when-requested, and label-alignment
pipeline for every one of the 500 no-retry draws. This qualifies inference and
ledger semantics; it is not a claim that 500 n=400 bootstrap draws meet the
same runtime budget. P0 is single-outcome; every P2/P23/common-metric fixture
adds the modeled `v` outcome with ordinary `x/z/w -> v` anchor paths while all
interactions continue to target `y` only. Publication-profile PLS-POS point discovery is
additionally exercised at each exact K from 3 through 5.
Those point-only cells use the typed
`qpls.multimod.heterogeneity.pos-published-p0-k3-k5-point-discovery.v2` plan.
K=3 and K=4 retain balanced n=120 strong-separation fixtures (40 and 30 rows
per class); K=5 uses balanced n=200 (40 rows per class). They retain the exact `p0_structural` /
`qpls.pls-pos.published.v2` identity, and POS-only ten-seeded-start discovery.
They do not claim or execute K=3 through K=5 bootstrap inference. Their
ordinary P0 models use three structural paths with distinct class-specific
coefficient vectors and no interaction terms; a one-path symmetric zero-effect
middle class is not used as a recovery fixture. Separate production-function
probes exhaustively enumerate all K! mappings (6, 24, and 120 at K=3, K=4, and
K=5) and require a nonidentity mutual-majority pass plus ambiguous and unique-
nonmajority rejection at each K. These exact point, bootstrap-profile, and
alignment cells remain Labs/absent until the frozen campaign produces commit-
bound passing evidence; they are not a generic `K <= 5` implementation claim.

Scientific context:

- Hahn, Johnson, Herrmann, and Huber (2002), *Capturing Customer Heterogeneity
  using a Finite Mixture PLS Approach*, https://doi.org/10.1007/BF03396655.
- Becker, Rai, Ringle, and Volckner (2013), *Discovering Unobserved
  Heterogeneity in Structural Equation Models to Avert Validity Threats*,
  https://aisel.aisnet.org/misq/vol37/iss3/3/.
