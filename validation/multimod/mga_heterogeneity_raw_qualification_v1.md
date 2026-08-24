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

`run_multimod_mga_qualification_v1.ps1` runs the Rust producer once and then
the independent Python comparator. The qualification-scale receipt contains:

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

`run_multimod_heterogeneity_qualification_v2.ps1` performs one shared heavy
production run. Downstream POS, common-metric, and bootstrap gates verify this
producer's immutable artifact rather than rerunning segmentation.

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
