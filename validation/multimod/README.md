# MultiMod qualification evidence

Everything in this directory is a qualification contract or an unexecuted
template. No file here is evidence that a MultiMod method passed.

- `multimod_capability_index_v1.json` registers every candidate profile as
  Labs with absent evidence.
- `*.qualification.manifest.json` enumerate independent profile/procedure
  evidence cells. Their source bindings and reports intentionally remain
  `null`/`pending` until the implementation is frozen.
- `v256_multimod_qualification_plan_v1.json` covers the complete scientific,
  persistence, export, native, regression and offline acceptance matrix. Its
  gate rows are the source of truth for the current ready/pending count; any
  remaining pending row prevents a runnable release campaign.
- `multimod_issue_inventory_v1.schema.json` freezes the first-campaign issue
  record, including profile, gate, seed, input digest, failure signature,
  probable root component and invalidated downstream gates.
- `unsupported_intersections_v1.json` is the machine-readable fail-closed
  reason-code contract.

The PowerShell campaign driver is
`validation/run_v256_multimod_qualification.ps1`. It defaults to plan-only
mode. `-Execute` is intentionally rejected while any gate has
`implementation_status=pending` or no reviewed command binding. Once real gate
programs exist, update the plan through code review, freeze a clean candidate
commit, then run:

```powershell
pwsh -NoProfile -File .\validation\run_v256_multimod_qualification.ps1 `
  -Execute `
  -CandidateCommit <exact-lowercase-HEAD>
```

Use `-Resume` only for the same plan digest and candidate commit. The driver
never cleans Cargo output, edits source, merges, pushes, tags or publishes.
When an open issue invalidates a later gate, the driver follows the complete
transitive dependency graph and records that gate as `blocked` without running
it. The next clean candidate campaign reruns the full chain; this avoids
spending build or simulation time on evidence that cannot be promoted. A
blocked gate is terminal for campaign accounting, but it always produces a
`completed_with_issues` campaign and a nonzero driver exit; it can never count
as clean or release-acceptable evidence.

## Independent reference oracles

`reference_oracles_v1.py` and `test_reference_oracles_v1.py` are
standard-library-only validation references over the immutable small inputs in
`fixtures/reference_oracles_v1.json`. They cover:

- exhaustive two-group signed OLS-slope permutation, label reversal, and the
  frozen None/Holm/Bonferroni/Sidak/BH probability adjustments;
- explicit conditional-path polynomial multiplication, first/pure-second/cross
  derivatives, and left-minus-right probe contrasts for a two-edge both-stage
  and a three-edge path;
- observed linear binary and continuous-treatment g-computation targets plus
  necessary observed-support positivity pass/failure receipts; and
- a two-class Gaussian FIMIX log-sum-exp likelihood, posterior normalization,
  full parameter count, information criteria, and entropy identity.

The production MGA/FIMIX/POS and conditional/causal producers bind independent
references to actual QuickPLS raw-data runner commands. A producer is reusable
by downstream gates only through the campaign's hash-bound dependency verifier.
No successful qualification is implied until the whole campaign passes.

Conditional-process and interventional-causal coverage now uses one logical
production raw-runner campaign per family. After one Cargo build, a two-minute
diagnostic-only production sentinel gates deterministic exact-case shards;
dedicated sealed evidence shards own all boundary and assumption checks. Atomic
SHA-256 receipts make completed cases safely resumable, and aggregation in
frozen plan order precedes an independent verifier that recomputes the plan,
executable, commit, result, receipt, dependency, and exact case bindings.
Qualification draw counts and within-case shared-ledger semantics are unchanged. Downstream
gates consume the hash-bound aggregate receipt instead of rerunning the
expensive matrix. The exact cases, timeout/concurrency boundaries, narrow
studentized atomic-case limitation, independent all-target
point/percentile/BCa/studentized checks, analytic-DGP plus independent
g-computation recovery checks, retained ledgers, and explicit API boundaries
are documented in `conditional_causal_raw_qualification_v1.md`.

MGA remains one logical producer artifact, but after one build its exact 15
scientific executions and boundary receipt run as at most four independent
cells. Each scientific cell resumes only at an existing production MGA shard
boundary from immutable identity-bound cache entries. A two-minute
diagnostic-only sentinel compiles the exact two-group root authority and
prepares its production plan before any science starts; it publishes no
estimate. Thirty-minute cell slices are retried only after verified cache
progress, and a 110-minute wrapper cap retains cleanup time. Aggregation
requires the exact plan before the existing comparator can publish
`mga-production-science.json`. Heterogeneity likewise executes after one build
as a fail-fast, resumable shard graph. Exact 2/3/5/20-group and non-Cartesian MGA cells, independent raw-null
probability reconstruction, multi-scenario FIMIX recovery, P0/P2/P23 PLS-POS,
independent likelihood/posterior/R-squared/objective checks, common-metric
suppression, exact P0 PLS-POS K=2 through K=5 candidate/bootstrap coverage, and
independent K! ambiguity/majority decisions are documented in
`mga_heterogeneity_raw_qualification_v1.md`.

`exports.semantic.readback` now has an executable 25-profile matrix. It binds
canonical production projections to the native no-replace publication path for
CSV, XLSX, JSON, HTML, PDF, SVG and PNG, independently verifies the exact
profile/procedure and candidate-authority receipt inventory, and exercises
strict optional posterior and replicate-sidecar publication. The typed
qualification authority injection exists only under `cfg(test)`; normal native
publication continues to accept only the immutable embedded candidate
authority and remains fail closed while the application is Labs.

`metamorphic.global` now binds all 25 exact profiles to deterministic fixtures
that use the public Recipe V4 compiler and the four production raw runners. It
compares complete typed results and retained inference evidence under mapped
row order, input-column/indicator order, model declaration order, repeated
seed and worker-count changes; separately maps full MGA group-label contrasts,
requires a real nonidentity class-label alignment with complete target vectors,
checks an observed-data sign reversal, and proves production MGA cancel/resume
equals uninterrupted execution without a publishable partial result. Shard
topology is additionally receipt-bound to the production full-refit ledger
tests rather than inferred from a fixture helper.

The gate builds the four producers once, runs the exact 25-cell matrix through
at most four direct executable processes, and requires all four successful
family baselines before their dependent axes. Cell, scientific and wrapper
limits are 1,800, 6,480 and 6,600 seconds respectively; process-tree cleanup,
atomic SHA-256 receipts and a stable campaign work root make completed cells
safely resumable without reducing any scientific setting. See
`metamorphic_qualification_v1.md` for the frozen transport contract.

The V2 maximum-profile performance gate records its single three-example release
build as a SHA-256-bound resumable topology stage and then invokes the binaries
directly. A completed build stage is reused without another Cargo invocation;
an interrupted build attempt fails closed rather than silently becoming a
second build. The gate runs only the exact 20-group/190-pair MGA cell,
the heterogeneity sentinel and P23 discovery followed by the parallel fixed-K
FIMIX/POS P23 bootstrap pair, and the unchanged dedicated conditional
512-cell/1,024-target producer. MGA retains 5,000 permutations and 5,000
bootstraps; each heterogeneity branch retains 500 bootstraps. Portable atomic
receipts bind every result, dependency, executable, plan, log and measurement,
so a rerun resumes only verified stages. An interrupted heterogeneity
result/receipt pair is quarantined before its shard reruns. The wrapper reports
the conservative maximum of sampled concurrent working set and the sum of every
root process's `PeakWorkingSet64`, limits every atomic producer to 1,800 seconds,
reserves the final 120 seconds of its 6,480-second internal cap for verified V2
report publication, and remains beneath the gate's 6,600-second cap. The public
evidence identities are `qpls.v256.multimod.maximum-profile-performance.v2` and
`qpls.v256.multimod.performance-output-verification.v2`. It also exercises exact
sidecar warning/cap boundaries, proves the ordinal MICOM Arrow stream remains
within its conservative physical-byte prediction, and proves serialized-cache
cancellation/resume equals uninterrupted MGA. Missing metrics, stale receipts,
or exceeded budgets fail the gate.

The installed and portable gates now drive all four typed result families in
the exact package through ordinary production preflight, runner, archive,
canonical export and raw-sidecar commands. They require strict save/reopen,
missing/tampered-sidecar rejection, MGA cancellation/resume, semantic readback,
offline/error-free execution and accessibility; the installed lane additionally
requires a fresh isolated NSIS install and complete uninstall cleanup. Each
packaged wrapper is internally capped at 6,480 seconds beneath its 6,600-second
gate cap. Its driver receives one shared scientific deadline, while at least
1,020 seconds remain reserved for supervised Node/candidate process-tree
termination and, for the installed lane, exact NSIS uninstall and cleanup.

## Required non-circular promotion cycle

The final implementation must complete this sequence without changing source
after evidence is collected:

1. Freeze the exact clean candidate commit and complete every prepackage
   scientific, persistence, export, native and performance gate.
2. Materialize the external prepackage authority set from those hash-bound gate
   receipts. It is bound to Git HEAD, package version, plan/binding hashes,
   tracked capability index and sorted unique exact cells; it has no wildcard
   or family-only authority.
3. Package that same commit. `build.rs` validates and embeds the authority and
   manifest set, while the package-only Cargo feature and Vite build flag add
   the qualification fixture bridge. Both are compile-time conditions: no
   project, request or runtime environment can inject promotion authority.
4. Launch the exact receipt-bound installed and portable executables. Each must
   emit `ReleaseQualifiedCandidate` provenance for covered exact cells and
   complete the four-family production workflow matrix; missing covered cells
   remain Labs, while malformed or mismatched authority/receipts fail closed.
5. Bind both smoke receipts to the embedded authority, package receipt and exact
   executable hashes, then derive the final external live set and run release
   acceptance. The receipts identify this unmerged review candidate only; a
   later harness-disabled rebuild is not covered.

The materializer and final verifier therefore cannot treat an external JSON
state alone as proof of release-qualified runtime behavior.

Development static audits may pass with pending gates reported. Frozen mode and
`run_v256_multimod_qualification.ps1 -Execute` fail closed until every remaining
pending gate is replaced by a reviewed real command.
