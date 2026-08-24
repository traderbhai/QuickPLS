# MGA and heterogeneity raw qualification producers

These validation-only producers exercise the production Recipe V4 compiler,
raw-data scoring adapters, point estimators, resampling engines, and public
result projections. They do not use the identity-score adapters in
`multimod_scientific_probe_v1.rs`, and synthetic truth is visible only to the
independent Python comparator.

Nothing in this document is executed evidence. A result qualifies a gate only
after the frozen campaign records a passing, hash-bound receipt for the exact
candidate commit.

## MGA producer

`run_multimod_mga_qualification_v1.ps1` performs one optimized release Cargo
build and then runs
the frozen MGA matrix as 15 independent production cells plus one boundary
cell. Before any scientific cell starts, a diagnostic-only two-minute sentinel
must compile the exact two-group General SEM authority and prepare its full
production execution plan. It publishes no estimate and does not run or reduce
a scientific ledger. The normal pool then runs at most four cells concurrently.
Every scientific cell calls the production resumable MGA runner and atomically
retains one immutable, SHA-bound cache entry after each completed production MGA
shard. An interrupted 30-minute cell slice is retried only after QuickPLS
reopens and validates an increased cache against the exact compiled production
plan. Completed ledgers are reused unchanged; any uncommitted current shard is
restarted deterministically as one full 5,000-draw ledger, never shortened or
split. Only the complete exact inventory is aggregated and passed to the
unchanged independent Python comparator. The qualification-scale receipt
contains:

- ordinary recursive General SEM PLS at exactly 2, 3, 5, and 20 groups, with
  all 190 comparisons explicitly confirmed in the 20-group cell;
- structural paths, outer loadings, outer weights, and endogenous R-squared;
- separate multiple-two-way, bounded-three-way, bounded moderated-mediation,
  four-disjoint-HOC, positive case-weighted, count-space frequency-weighted,
  and reflective PLSc cells;
- count-space versus physically expanded-row frequency equivalence;
- pairwise MICOM, permutation MGA, Henseler directional probabilities, BC
  intervals, max-spread omnibus inference, multiplicity procedures, and a
  separate ordinary-PLS pooled/Welch/Wald sensitivity cell; and
- exact label reversal, typed exclusions, small-group, imbalance, heavy-run,
  and directional-predeclaration boundaries.

Independent probability reconstruction uses bounded production evidence. Each
pair retains one deterministic permutation-null target. MICOM retains every
construct's Step-2 compositional-correlation null series and the first
construct's Step-3 mean and log-variance-ratio series. The comparator
recalculates Type-7 quantiles, plus-one tails, Step-2/Step-3 decisions, and a
representative Henseler probability from the shared group-bootstrap bank. All
other targets retain stable identities and summaries without duplicating large
raw vectors.

The wrapper rejects a dirty tracked or untracked tree, non-baseline metamorphic
environment, non-seed-42 qualification, stale cell identity, or altered cache
checkpoint. Plan, commit, executable, seed, scale, environment, production
cache, and completed result SHA identities travel together. Build, sentinel,
cell slices, aggregation, and comparison are bounded child processes whose
Windows process trees are terminated on timeout. The family cap is 6,600
seconds; completed cells and production-shard caches remain external and can be
resumed by the same exact candidate. A partial scientific result is never
published. The final gate artifact remains `mga-production-science.json`.

The built producer also exposes the same resumable authority directly through
`--cell mga-general-20-groups` for the maximum-profile performance gate. This
mode requires the exact plan, commit, executable, environment, seed, scale, and
cache identities and emits one
`qpls.multimod.mga.qualification-cell-result.v1` envelope whose `payload` is
the ordinary 20-group matrix entry; it does not execute the other 14 scientific
cells or the boundary cell.

The Arrow cost preflight charges these audit distributions using their trusted
physical representation. Repeated target IDs are dictionary encoded, while
MICOM uses construct ordinals plus a statistic-kind code tied to its construct
table. The 20-group, three-construct qualification cell is expected to warn
above 128 MiB but remain below 512 MiB; wider target/construct inventories can
cross the hard cap and must fail closed before point estimation.

Standalone command, intentionally not executed during implementation:

```powershell
pwsh -NoProfile -File validation/multimod/run_multimod_mga_qualification_v1.ps1 `
  -Scale qualification -Seed 42 -Output <external-evidence>\mga-production-science.json
```

## FIMIX-PLS and PLS-POS producer

`run_multimod_heterogeneity_qualification_v2.ps1` performs one optimized
release Cargo build,
runs a small raw-runner sentinel, and then executes the production matrix as a
dependency-aware set of deterministic executable shards. Recovery seeds,
simulation scenarios, candidate-K cells, POS discoveries, failure boundaries,
and fixed-K bootstrap cells each publish an atomic SHA-256 receipt. A bootstrap
shard consumes the exact retained discovery identity instead of rerunning its
discovery. Independent point shards may run concurrently; bootstrap concurrency
is bounded separately. No Cargo build or test process runs concurrently.

The wrapper refuses a dirty tracked or untracked source tree and any inherited
non-baseline metamorphism, compact fixture, sign transform, or worker count
other than one. The comparator independently requires the baseline 400-row,
seed-42 qualification identity. The default wall-clock budget is 110 minutes,
no scientific shard may run for more than 30 minutes, and the sentinel is
capped at two minutes. Cargo build, plan generation, every shard, aggregation,
and comparison all run as supervised child processes bounded by the remaining
family budget; termination kills the process tree and waits for it before the
wrapper exits. A failed sentinel stops before expensive work. Any later failure or
timeout stops the active batch immediately while preserving verified shard
receipts; rerunning the same exact commit, seed, scale, plan, and executable
resumes only missing or invalid shards. Missing, altered, stale, substituted,
or dependency-mismatched receipts are rotated into `_attempt_history` and
recomputed. Only a complete, deterministically aggregated matrix is passed to
the independent comparator and published as the gate artifact. Downstream POS,
common-metric, and bootstrap gates continue to verify that single immutable
artifact rather than rerunning segmentation.

FIMIX coverage includes five distinct strong-separation data seeds, ten
moderate-power runs, five overlap runs, five 75/25 imbalance runs, five
non-normal runs, a homogeneous null, and candidate K=2 through K=5. The
receipt retains the exact pooled standardized equation designs and outcomes.
The comparator independently reconstructs Gaussian-mixture log-sum-exp
likelihood, every posterior row, complete parameter count, AIC/AIC3/AIC4/BIC/
CAIC/HQ, entropy, monotone 30-start traces, and adjusted Rand indices.

PLS-POS coverage includes publication-faithful P0 and separately identified
destination-scored P2/P23 runs, each with ten starts and full no-improvement
sweeps. Final segment receipts retain source-row identities plus observed and
fitted endogenous scores. The comparator independently recomputes each
segment/outcome R-squared and the unweighted total objective, then checks
partition recovery, overlap, and homogeneous-null thresholds.

The fixed-K matrix contains FIMIX P0/P2/P23, PLS-POS P0/P2/P23, a common-metric
failure fixture, and exact publication-profile P0 PLS-POS cells at K=3, K=4,
and K=5 in addition to K=2. Each K=3 through K=5 cell uses a balanced 400-row
strong-separation fixture, a POS-only ten-seeded-start plan, and one
500-replicate no-retry ledger. The comparator independently enumerates all K!
label mappings, validates every retained target digest and overlap decision,
requires at least 450 usable draws, and reproduces every Type-7 interval.
Production-function decision probes separately require nonidentity-majority
acceptance and ambiguous/non-majority rejection at K=3, K=4, and K=5.
Common-metric pass and suppression decisions remain independently rebuilt from
the retained MICOM Step-2 null-correlation series in their separately qualified
K=2 cells.

Standalone command, intentionally not executed during implementation:

```powershell
pwsh -NoProfile -File validation/multimod/run_multimod_heterogeneity_qualification_v2.ps1 `
  -Scale qualification -Seed 42 -Output <external-evidence>\heterogeneity-production-science.json
```

The resumable checkpoint directory defaults to
`<Output>.heterogeneity-shards`. `-WorkRoot` may place it on a larger evidence
volume. `-MaxParallelShards` and `-MaxParallelBootstrapShards` tune only
independent post-build executable work; they do not broaden the scientific
contract or alter seeds, ledgers, estimator settings, or aggregation order.

## FIMIX failure-boundary binding

`fimix.collapse.boundaries` combines two non-substitutable evidence sources.
The raw production-runner receipt exercises rank-deficient, zero-variance, and
sub-five-percent rare-class data. Focused in-module tests then call the actual
private EM likelihood, E-step, responsibility, variance, and weighted-QR
validators and require the stable `LikelihoodDecrease`, `NonFinite`,
`CollapsedClass`, `VarianceCollapse`, and `RankDeficient` codes. They also bind
the dynamic minimum-class formula and the five-percent share rule. The campaign
uses the exact `fimix_failure_boundary_` test filter; it does not add a shipped
fault-injection API or substitute an identity-score adapter.

These tests and the raw producer were written but intentionally not executed
during the implementation-only batch.
