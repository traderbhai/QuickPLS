# QuickPLS product-finalization implementation status

Status date: 16 August 2026

This checkpoint tracks the engineering program for active SmartPLS
research-desktop parity. It is not a release, beta, signing, or product-parity
claim.

> Historical checkpoint: the counts and unfinished-work narrative below describe the 16 August 2026 program snapshot. They are preserved for chronology and must not be used as current availability. Current method coverage and customer surfaces come only from Capability Registry V2 and its active method manifests.

## Completion status

**Product finalized: no.**

The authoritative baseline remains 43 active catalogue rows plus two explicit
legacy exclusions. Registry V2 currently contains no option cell that is both
full coverage and release-qualified on the Standard surface. Standard therefore
correctly exposes no calculation method; executable narrower implementations
require an explicit Experimental Labs preference.

Registry evidence is now fail-closed against every linked live method manifest.
The current 48 option cells, including two explicit legacy exclusions, contain
39 absent, two engine-only, three archive-qualified, three native-qualified, and
one release-qualified evidence state. A reconciliation removed 24 overstated
cell claims while preserving all historical reports. The historical parity
ledger is no longer allowed to authorize current Registry V2 state.

Current counts and exact option-cell identities are read from
`validation/capabilities/capability_registry_v2.json`. Do not copy counts into
product code or infer full coverage from a historical method-promotion state.

## Status against the authoritative Product Finalization Program

This status uses only the wave and acceptance structure in the QuickPLS
Product Finalization Program. **Implemented** does not mean **qualified**;
completion is claimed only when the corresponding wave exit gate is met.

| Program section | Status | Completed | Remaining |
|---|---|---|---|
| Wave 0 — Freeze the active parity contract | **Done** | Registry V2 freezes 43 active rows and two intentional exclusions, with independent coverage, evidence, and surface states. | Keep the option-level catalogue current through the final documentation-delta audit. |
| Wave 1 — Universal model, capability registry, and qualification v2 | **Done — exit gate verified** | `SemModelV4`, immutable PLS/CB-SEM compilers, Recipe 4, project schema 6, Registry-driven customer surfaces, canonical results, QualificationSpec V2, and the exact diagram-origin round-trip proof satisfy all five Wave-1 exit conditions. | Keep the Internal/Labs boundaries truthful while later waves complete live Standard persistence, remaining scientific breadth, qualification, exports, and release certification. |
| Wave 2 — Diagram and data foundation | **Done — exit gate verified** | Standard activates native-resolved schema-6 ready/draft models as its sole scientific `SemModelV4` authority, projects the diagram one way, commits edits through digest CAS, switches strict models through metadata-only Explorer entries, binds descriptor-only source data, and persists model plus presentation authority only through validated new-copy save/reopen. Recipe/result-bound originals are technically edit-locked and fork through a native-digest-validated new revision without rewriting prior recipes or results. Dedicated controls plus a strict Expert complete-model fallback cover the Section-3 shape matrix, including explicit latent and observed controls; captions, notes, shapes, images, and lines remain presentation-only. Schema-5 save/autosave/calculation and stale bindings fail closed; dirty/replacement/close guards use authority digests, and clean saved projects close atomically with their source session. Versioned transformations and view-only sorting are non-destructive and reproducible. Current production receipts pass the 20/80 and 100/300 interaction budgets. Final gates pass: 166 qpls-core tests, 149 qpls-project unit plus two integration tests, 1,181 frontend tests, desktop workspace compilation, TypeScript, formatting/diff checks, production build, and an independent no-P0/P1 exit audit. | Begin the attached plan's parallel PLS Waves 3–5 and CB-SEM Waves 3–6. Later method waves—not Wave 2—must make each supported group/weight estimator cell executable and release-qualified. |
| Waves 3–5 — Complete PLS-SEM | **Partial** | Bounded PLS/PLSc, configured and fixed-score execution, assessment, resampling, group, prediction, comparison, canonical, archive, and oracle slices exist. Current Recipe-v4 PLS results now carry compiled control estimates, exact point-estimate scale attribution, and deterministic Mode-A/Mode-B convergence receipts; resampling parameter identities are typed, HTMT bootstrap tail/interval selection is explicit, and CCA residual diagnostics are complete and persistence-bound. Source now also contains a bounded Recipe-v4 v7 Internal execution/readback vertical for the exact Labs-only fixed-score quadratic diagnostic cell. GSCA, CCA, IPMA, and NCA have fresh source-bound engine/archive/native receipts; GSCA additionally has a supervised packaged Windows receipt and independent method audit and is the first truthfully `release_qualified` option cell, while CCA, IPMA, and NCA remain `native_qualified`. | All active PLS cells must reach full coverage and release-qualified evidence, including remaining result-location, inference, group, segmentation, simulation, package, performance, and review work. CCA/IPMA normal execution remains blocked by the still-unqualified base PLS cell. GSCA remains partial and Labs-only because bootstrap/inference breadth is absent despite its bounded point-estimation release receipt; CCA, IPMA, and NCA still require packaged, performance, and independent-review gates. Nonlinear closure still requires diagram authoring, qualified inference and plotting, real writer/readback coverage, and complete scientific/package qualification. |
| Waves 3–6 — Rebuild and complete CB-SEM | **Partial** | General parameter-table foundations plus bounded matrix/raw-mean CFA and product-indicator moderation slices exist. The exact ML path now admits stable identified reciprocal systems, rejects unstable systems by spectral radius, rejects indefinite information/nonpositive inverse variances without clamping, distinguishes optimizer failure causes, reports deterministic expected-information condition diagnostics and exact noncentral-chi-square RMSEA intervals, carries a bounded genuine score/LM modification-index family, and executes a separately versioned raw/listwise covariance-CFA exact case bootstrap with complete deterministic success/failure ledgers and selected two-sided/greater/less zero-null tests. Complete-result analytical-studentized Type-7 and BCa Type-7 options are now exposed through bounded Internal/Labs adapters v11/v12 with canonical/schema-6/native/export integration, and independently audited descriptive coverage-pilot plus 5,000/10,000-replicate resource evidence is retained. | General SEM breadth, importance-selected interval exposure, customer-ready comparison/groups/invariance/moderation, difficult models, additional independent references, final coverage simulations, broader maximum-axis evidence, packaging, and review. |
| Wave 7 — Reporting, comparison, interoperability, and peripheral closure | **Partial** | Canonical-result, bounded comparison, schema-6 readback, and several peripheral analytical foundations exist. | Frozen saved-report artifacts, one canonical source for every GUI/CLI/export format, semantic readback, result-derived datasets, peripheral breadth, examples, and PNG closure. |
| Wave 8 — Language cutover and final robustness certification | **Partial** | Registry-driven surfaces, Method Details, and customer-copy enforcement are implemented. | Full language cutover, packaged Windows/scaling/accessibility/performance/soak matrices, independent family reviews, and final SmartPLS delta audit. |
| Section 5 — Qualification and test plan | **Partial** | QualificationSpec V2, deterministic contracts, transparent oracles, adversarial tests, and bounded work evidence exist. | Qualification-scale simulations and complete persistence/export/native/package/performance/soak/review receipts for every active cell. |
| Section 6 — Final product-development acceptance criteria | **Not met** | The two intentional exclusions remain explicit and Standard correctly fails closed. | `0/43` active rows are full and `1/46` active option-cell instances is release-qualified; all 18 conjunctive final acceptance conditions remain the terminal gate. |

**Program completion: 3 of 9 sections are Done. The next active program phase
is parallel PLS Waves 3–5 and CB-SEM Waves 3–6.**

The current authoritative finalization-readiness gate remains intentionally
red while the structural contract passes. It reports two blocker classes: one
cell-level parity obligation is still open, and all 44 active-parity cells are
not yet simultaneously full, release-qualified, and Standard. Across all 46
active cell identities, coverage is 29 partial and 17 absent; evidence is 1
release-qualified, 3 native-qualified, 3 archive-qualified, 2 engine-only, and
37 absent; every cell
remains Labs. These states must be closed with evidence rather than relabeled to
meet a deadline.

### Wave 2 exit-gate detail

| Attached-plan exit condition | Status | Evidence or exact remainder |
|---|---|---|
| All required PLS and CB-SEM shapes can be authored and reopened | **Done — native authority and reopen verified** | A green Rust witness covers the full Section-3.1 representation matrix. Standard uses one native-resolved, digest-CAS `SemModelV4` authority with dedicated controls plus a strict Expert complete-model fallback, including common factors/composites, measurement and structural paths, covariance kinds, parameters, identification, means/intercepts/thresholds, groups, interactions, HOCs, polynomials, observed and latent controls, and the presentation-only object layer. Recipe-bound revisions preserve the original model, recipe, and results while saving the unbound fork. The Windows save-copy path strictly validates and reopens the reserved nested presentation layout and rejects every unrelated non-model mutation. |
| 20-construct/80-indicator applied model is fully usable | **Done for the measured interaction gate** | Production Chromium passes selection, drag, pan, zoom, typed preflight, fixture reload, 223.40 ms open p95, 64.46 ms painted edit-response p95, and 64 minimum FPS. |
| 100-construct/300-indicator stress model remains editable | **Done for the measured interaction gate** | Production Chromium passes the same interaction matrix with 696.46 ms open p95, 80.64 ms painted edit-response p95, and 64 minimum FPS. |
| Every data transformation is non-destructive, versioned, and reproducible | **Done — focused and native verified** | Recode, zero-input add-column, atomic one-or-many-column missing-marker cleanup, and sample-n-1 z-score create exact immutable child versions; sort is presentation-only. Core transformation tests are 13/13, focused project-lineage replay is 1/1, focused TypeScript tests are 32/32, full TypeScript typecheck is green, and both desktop save/reopen filters pass 1/1. |

## Accelerated delivery discipline

Development follows this same program in order; no replacement roadmap is in
use. To reduce elapsed time, work now moves in **exit-gate batches**, not a long
series of audit-only micro-slices. The completed Wave-2 integration batch
closed immutable transformation versioning, the Windows-safe new-destination
schema-6 persistence path, Standard's single native-resolved model authority
with CAS mutation, exact recipe-bound revision forks, Explorer switching,
latent-control identity, presentation-only authoring, dirty guards, and
lifecycle binding. The cross-language, persistence, accessibility, broad
Rust/frontend, performance, independent-review, and production-build checks
are green. Work now advances to the attached plan's parallel PLS and CB-SEM
method waves.

Parallel tracks must still declare non-overlapping file ownership and preserve
the same scientific contracts. Focused checks run while source changes; the
broader regression/build matrix runs once at the exit gate. A timeboxed track
that cannot prove a safe implementation stops with a precise blocker instead
of expanding scope or weakening acceptance criteria. Wave status changes only
when its program exit gate is actually green; partial implementation and red
evidence remain recorded as such. Waves 3–6 begin immediately after Wave 2,
with PLS and CB-SEM then proceeding in parallel as the attached program allows.

Current Registry partition: rows are 0 full, 27 partial, 16 absent, and two
intentionally excluded; option cells are 0 full, 29 partial, 17 absent, and two
excluded. Evidence across all 48 cells is 39 absent, two engine-only, three
archive-qualified, three native-qualified, and one release-qualified.

## Implemented foundations

### Parity and qualification authority

- The complete Wave-0 option-level acceptance matrix captures every required
  dimension for all active catalogue rows.
- Capability Registry V2 separates coverage, evidence, and customer surface at
  exact option-cell level.
- An established-method integration contract now resolves the live
  Registry V2 and method manifests for GSCA, CCA, IPMA, and NCA, then generates
  deterministic self-contained TypeScript/Rust lookup tables plus an ownership
  receipt. Strict code generation proves exact parity with the existing method,
  canonical-table, shared-base, and CLI branches; rejects schema, traversal,
  duplicate-key, symlink/reparse, and unexpected-output drift; and deliberately
  excludes coverage, evidence, surface, qualification, receipt, and timestamp
  claims. Fifteen focused mutation tests, generator `--check`, TypeScript
  compilation, `rustfmt`, JSON, and whitespace gates pass. The generated table
  now owns exactly those four TypeScript method/canonical branches and Rust CLI
  mappings: TypeScript preserves base-to-primary method order, canonical table
  ownership remains primary-only, and CLI preserves primary-to-base first-error
  order. Focused TypeScript and Cargo tests pass, GSCA canonical output is byte-
  identical, and an independent phase-2 audit found no P0/P1. Other consumers
  remain handwritten and will migrate one at a time behind the same parity gate;
  scientific estimation, Registry evidence, and qualification state are unchanged.
- The frontend build now uses a separate entry-reachable production TypeScript
  program while CI retains one full source-and-test typecheck before Vitest and
  Vite-only bundling. The production graph contains 129 shipped files and no test
  or fixture files, versus 342 files in the full gate. Measured cold typechecking
  improved by 42.4 percent and a repeat build by 67.8 percent; the resulting 19-file,
  2,208,950-byte Vite output remained byte-identical. Strict compiler settings,
  dependencies, aliases, and the full CI typecheck are unchanged. The focused
  acceleration contract, both TypeScript gates, bundle build, and independent
  P0/P1 review pass. The three stale 47-cell assertions and two matching Registry
  documents were reconciled to the authoritative 48-cell inventory; the complete
  frontend suite now passes 164 files and 1,227 tests without relaxing a gate.
- Trust Center availability, Top Bar method labels/run preflight, and native
  status-bar method counts now consume that same fail-closed Registry V2
  projection instead of legacy method status or hard-coded counts.
- Registry validation re-derives each linked manifest from current source and
  evidence hashes and rejects any cell whose evidence state exceeds that live
  result. The reconciliation is idempotent, preserves historical artifacts,
  and runs in dry-run mode unless an explicit write is requested.
- QualificationSpec V2 defines the eight-level evidence ladder and binds exact
  four-field capability identities.
- The TypeScript product bridge, Rust runtime parser, and `qpls methods` CLI
  projection consume the same registry source. The CLI includes the embedded
  registry SHA-256 and no longer presents the legacy execution label
  `Validated` as parity.
- The CLI `run` boundary now derives an exact, ordered set of Registry V2
  option cells from every supported recipe method and typed configuration.
  Requested add-ons are checked before their mandatory base method; plain PLS
  and point PLSc can no longer bypass unavailable base cells through an empty
  dependency list. Missing, legacy, mismatched, and unmapped configurations
  fail closed before calculation or output-file creation.
- Packaged Windows acceptance is manifest-derived rather than governed by a
  fixed aggregate check count.
- Complexity/performance profile V2 defines registry-derived budgets for both
  reference hardware classes, all required workload profiles, UI latency and
  frame-rate limits, progress/cancellation, memory, process cleanup, and the
  greater-than-20-percent regression rule. Its structural contract passes;
  measurement qualification remains intentionally red until the required
  current receipts are recorded.

### Scientific model and project foundations

- `SemModelV4` represents variables, scientific relations, parameter controls,
  constraints, groups, data binding, annotations, and presentation separately.
- The live workbench adapter fails closed on unresolved factor/composite intent
  and covariance meaning. It never compiles from the old covariance-filtered
  recipe projection.
- Immutable PLS and CB-SEM V2 compilers produce typed plans; unsupported
  estimator content remains preserved and blocked rather than discarded.
- A bounded internal CB-SEM Recipe-v4 adapter now executes raw, covariance, and
  explicitly scaled correlation inputs from `CompiledCbsemPlanV2`. It verifies
  immutable compiler/data identities, exact sample and denominator metadata,
  matrix validity, stable parameter bindings, and raw-versus-moment numerical
  equivalence. The same Internal/Labs-only path now uses the shared cancellable
  native job budget and produces a validated `CanonicalResultDocumentV2` for
  exact schema-6 append/readback. It still has no Standard GUI/CLI activation,
  admitted qualification/package evidence, registry promotion, or export
  qualification.
- The Internal/Labs CB-SEM Recipe-v4 path now has a separately versioned exact
  parameter-table v3 engine. The drawn/compiled model alone determines
  exogenous-latent, endogenous-disturbance, and observed-residual covariance
  cells; undeclared cells are fixed to zero. Fixed/free rows, finite bounds,
  equality labels, marker or fixed-variance identification, stable IDs, local
  Jacobian-rank checks, and starting latent/residual/implied positive-
  definiteness checks are executable. Legacy schema-5 CB-SEM remains
  unchanged. This slice is still unqualified Internal work: Special Assumption
  materialization, general means/structural intercepts/thresholds/groups,
  format export readback, external-oracle receipts, packaged Windows evidence,
  and registry activation remain open.
- The same exact parameter-table ML foundation now executes identified
  reciprocal structural systems only when the full structural spectral radius
  is below `1 - 1e-8`, and emits a typed stability witness. Unstable or
  unevaluable systems fail closed. Expected information must be positive
  definite; nonpositive inverse-information variances are rejected instead of
  clamped. Line-search failure, objective stagnation, and iteration exhaustion
  retain distinct numerical witnesses, while scaled one-norm condition and
  reciprocal-condition diagnostics are deterministic for covariance-only and
  mean paths. The former heuristic RMSEA interval is replaced by a genuine,
  deterministic 90% noncentral-chi-square inversion using the `N - 1`
  denominator and an explicit method witness. Current Recipe-v4 adapters v5
  through v10 retain that method, confidence level, and both interval bounds in
  canonical schema-6 results; Rust and TypeScript readers recompute and bind
  the values exactly while retaining historical v2/v3/v4 compatibility. The
  noncentral-chi-square CDF now uses a mode-relative Poisson recurrence,
  compensated summation, and rigorous geometric omitted-tail bounds, avoiding
  a false nonconvergence caused by absolute mode-weight normalization roundoff
  without adding any clamp or fallback; the original failing fixture and
  independent extreme-lambda regressions through `1e6` pass, followed by a
  green 24/24 matrix-input regression suite.
  Current attachments also require their resident Recipe-v4, SemModelV4, and
  dataset descriptor and bind them through deterministic recompilation before
  specialized missing-data validation.
  Focused reciprocal, admissibility, optimizer, conditioning, numerical,
  canonical, and reopen tests pass; this does not add robust estimators, broad
  SEM syntax, bootstrap, comparison, group, package, or qualification evidence.
- The covariance-structure CFA exact engine now has a separate genuine
  score/Lagrange-multiplier v1 family for explicitly declared, off-diagonal
  residual-covariance parameters fixed at positive zero. It uses the exact ML
  score, analytic candidate derivative, expected-information nuisance Schur
  correction, one-degree-of-freedom chi-square probability, typed unavailable
  outcomes, stable parameter identity, and deterministic ordering; it never
  reuses the historical residual-correlation heuristic. Conditional Recipe-v4
  adapter v8, canonical tables, schema-6 recompilation, strict TypeScript
  readback, customer tables, and export bind MI/EPC arithmetic and endpoint
  source columns exactly. Mean structures, SEM paths, mean replacement,
  undeclared pairs, cross-loadings, equality releases, and other candidate
  classes remain explicitly outside this first bounded family.
- The same Internal/Labs exact path now has a distinct
  `cbsem_exact_case_bootstrap_v1` family and conditional Recipe-v4 adapter v9
  for raw, listwise, unweighted, unclustered, unstratified, single-group,
  covariance-structure CFA. It prepares and validates the complete-case source
  once, then performs full exact-v3 ML point refits on ordered with-replacement
  samples of size `N` using the ML `N` covariance denominator. Every requested
  primary replicate is attempted exactly once with no retry; cancellation
  aborts the aggregate, while numerical, convergence, admissibility, and moment
  failures remain in the ordered typed ledger. Requests admit the plan's 500
  pilot through 10,000 replicates. The 500 pilot is valid execution evidence but
  remains explicitly inference-unavailable because usable inference requires
  `max(1,000, ceil(0.90 * B))`; available runs emit sample-SD standard errors
  and 95% Type-7 percentile intervals only. The point-only refit seam does not
  nest fit/RMSEA, information/SE, score/LM, or another bootstrap. Adapter v9
  binds the outer recipe, exact point result, compiler/model/dataset identities,
  complete-case universe, deterministic seed schedule, stable parameter IDs,
  optimizer witnesses, and full success/failure partition. Rust schema-6
  validation reconstructs the base-point digest and every schedule digest and
  recomputes interval arithmetic before exact serialize/reopen; TypeScript
  validates the recorded structure, provenance, partition, and arithmetic but
  truthfully does not emulate Rust's RNG or claim raw-refit replay. Current
  execution emits conditional adapter v10, including for the default-omitted
  two-sided recipe selection, and derives null-centered inclusive two-sided,
  greater, and less counts from the same successful-refit ledger. Plus-one
  probabilities use usable refits only; the selected `p <= 0.05` decision is
  typed, every stable free-parameter ID remains present, and variance-boundary,
  zero-excluding, unsupported, and 500-pilot cases are explicitly unavailable
  rather than omitted. Schema-6 recompiles the resident recipe/model, derives
  eligibility, and recomputes the complete hypothesis receipt bit-exactly;
  historical v9 remains readable and rejects injected v10 artifacts. Focused
  evidence includes a real 500-refit runner test, cooperative cancellation,
  canonical attach/reopen, base/schedule-digest tamper rejection, 12 scheduler
  contract tests, 204 strict TypeScript/native tests, and direct-result parser
  coverage. This is not full CB-SEM bootstrap parity: important-result
  selection, 5,000/10,000 accepted product
  performance and archive-size evidence, qualification-scale coverage/failure-rate simulations,
  independent oracle comparison, packaging, qualification, and promotion
  remain open.
- A separate opt-in exact-refit wrapper now adds one deterministic
  expected-information/delta-method standard-error pass for future genuine
  bootstrap-t intervals. Identity-draw estimates and standard errors match the
  canonical point estimator bit-for-bit; singular, non-positive, derivative,
  invalid-variance, and numerical information outcomes are typed unavailable
  without converting the successful point refit into a failed bootstrap draw.
  The established refit type, APIs, and v9/v10 bytes remain unchanged. This
  wrapper is now consumed only by the bounded adapter-v11 path described below;
  direct callers remain explicit opt-in APIs rather than a Registry promotion.
- An additional opt-in S2 scheduler now executes each exact case draw once,
  returns the byte-identical v10 base aggregate plus a separate analytically
  studentized sidecar, and retains one compact whole-vector standard-error
  receipt for every successful point refit. It computes finite
  `(theta* - theta_hat) / se*` pivots and reversed 95% Type-7 bootstrap-t
  bounds using the point standard error; B=500, insufficient usable refits,
  and point-information failure remain typed unavailable. Focused Rust gates
  cover base-byte equality, one callback per draw, 1-vs-4 worker identity,
  ledger/order tampering, cancellation, exact hand arithmetic, and preservation
  of point successes when an outer information receipt is unavailable. An
  independent source audit found no numerical P1. Complete-result execution is
  now wired through selector `analytic_studentized_type7` and adapter
  `compiled_recipe_v4_cbsem_plan_v2_execution_v11` on the Internal/Labs surface.
  The direct runner rejects B outside 500–10,000, workers above 12, complete-case
  N above 180, V above 9, free-row P above 18, and equality-aware optimizer D
  above 18 before point optimization, expected-information work, or schedule
  allocation; a dataset with more than 180 physical rows but exactly 180
  complete modeled rows remains valid. Accepted execution stores one atomic
  `{base, studentized}` wrapper, retains unchanged v10 base and zero-null tables,
  and adds four frozen studentized canonical tables. Schema-6 reopens against
  the resident recipe/model/dataset, reauthenticates ordered point/refit SE
  receipts, and bit-recomputes pivots and reversed Type-7 bounds. Native setup,
  readiness, results, and export expose the same caps and B=500 typed-
  unavailable state; archive copy states exactly that ledger/arithmetic were
  checked while raw refits and expected-information calculations were not
  replayed. Focused evidence includes six runner cap/v10/v11 tests, canonical
  attach/reopen/tamper and historical-v10 regression tests, 199 TypeScript/native
  tests, TypeScript compilation, and independent Rust plus TypeScript P0/P1
  approvals. The studentized slice remains Internal/Labs and Registry
  evidence/promotion is unchanged.
- A source-bound measurement-only harness now runs S1+S2 against the existing
  v0.7 CFA fixture with one warm-up, five measured process-tree runs, a worker
  comparison, and a typed cancellation probe. The first recorded case
  (`N=180`, three factor blocks, 18 free parameters, B=1,000, four scheduler
  workers) measured 5.88 s median and 6.58 s Type-7 p95 elapsed time, about
  16.7 MiB p95 process-tree working set, 1.20 MB compact combined S2 JSON,
  zero point-fit or analytical-SE failures, exact 1-vs-4 scientific equality,
  and 0.38 ms terminal cancellation latency after the trigger. The receipt is
  explicitly `measurement_only_no_caps_or_promotion`: it does not establish
  product N/V/P/D/B/worker, memory, canonical/export-size, or runtime caps.
  The follow-on predeclared Phase-0 matrix completed all compact, N, observed-
  variable/free-parameter, B, worker, size, and cancellation cases without a
  stop event on the recorded Windows 11, 6-core/12-thread, 15.31 GiB machine.
  W1/W4/W12 results are bit-identical. Across the largest available fixture
  (`N=180`, `V=9`, `P=18`), B=1,000 measured 14.54 s median at W1, 5.39 s at
  W4, and 3.67 s at W12; B=2,000/W4 measured 10.30 s median, 11.88 s p95,
  21.0 MiB p95 process-tree working set, and 2.39 MB compact combined JSON.
  Cancellation terminated in 0.25 ms after the trigger. The optimizer
  dimension remained explicitly unknown rather than inferred from `P`, and the
  matrix remained `no_caps_or_qualification`: it did not cover larger N/V/P/D,
  B=5,000/10,000, canonical-v11/export bytes, accepted maximum-axis baselines,
  or a fail-closed workload policy. The later Phase-1b and v11 work closes only
  those bounded resource, canonical/export, and scheduling gaps; broader-axis
  qualification remains blocked.
- The append-only Phase-1a applied run then exercised the predeclared
  `N=180`, `V=9`, `P=18`, `D=unknown`, B=5,000/W12 probe and independent
  B=10,000 cancellation cell. Runtime (17.74 s), process-tree working set
  (41.6 MB), compact combined S2 JSON size (9.59 MB), and cancellation latency
  (1.46 ms)
  stayed inside their frozen limits. The scientific probe retained seven
  typed optimizer-stagnation point failures and six singular-information
  standard-error outcomes, leaving 4,987/5,000 studentized-usable refits
  (99.74%) and available inference. Because the predeclared operational rule
  required zero failures, the probe was correctly rejected and the dependent
  B=10,000 completion cell was not run. The report remains
  `phase1_applied_evidence_failed_product_cap_blocked`; no threshold was
  relaxed after observing it, and no product cap or qualification is claimed.
  A separately preserved same-seed full ledger is forensic evidence only, not
  a manifest-bound qualification receipt. A new independent-seed gate must
  bind the authoritative optimizer dimension and the method's pre-existing
  usable-replicate policy rather than inventing a zero-failure requirement.
- The follow-on append-only Phase-1b gate exposed the prepared exact plan's
  equality-aware optimizer dimension without caching optimizer state or
  changing analysis bytes (`N=180`, `V=9`, `P=18`, `D=18`). Its immutable
  manifest bound the rebuilt binary, example, runner, schemas, fixtures, five
  exact unseen-seed cells, typed ledger allowlists, existing 90% usable policy,
  conservative one-sided 95% Clopper-Pearson upper bound, and the pre-Phase-1b
  runtime/RAM/size/cancellation ceilings. All five cells passed. B=5,000/W12
  measured 18.27 s median and 18.85 s Type-7 p95 with 4,994 usable refits and
  a 0.00237 upper failure-rate bound; its independent seed retained 4,999
  usable refits. B=10,000/W12 measured 35.21 s median and 36.37 s p95 with
  9,990 usable refits and a 0.00170 upper bound; its independent seed retained
  9,995 usable refits. Every point/SE outcome formed an exact typed partition,
  same-seed repeats were bit-identical, p95 working set stayed below 82 MB,
  compact result size stayed below 20 MB, and B=10,000 cancellation terminated
  in 1.50 ms with no orphan process. The schema-validated report status is
  `phase1b_applied_resource_candidate_evidence_passed_qualification_blocked`:
  it is bounded resource evidence, not coverage, canonical/export, package,
  broader-axis, scientific-review, qualification, or product-exposure proof.
- A separate append-only coverage program now binds two frozen covariance-CFA
  DGPs, exact PCG64DXSM/ChaCha schedule authority, percentile/S2/BCa arithmetic,
  independent SciPy checks, retained failure ledgers, worker replay, and exact
  Clopper-Pearson V2 endpoints. Historical pilot-a and pilot-c evidence remains
  preserved and rejected for its recorded numerical-contract defects; neither
  was overwritten or relabelled. Fresh conformance-d passed before pilot-d.
  Pilot-d then completed all eight serial shards and all 40 datasets at
  B=5,000/W12: the independently regenerated 17-file family contains 200,000
  planned bootstrap fits, one typed non-convergence, 39 singular-information SE
  outcomes, and zero delete-one failures. All 57 primary coverage cells observed
  17–20 hits out of 20; all six availability cells were 20/20 with a certified
  one-sided lower bound of `0.7871199821209823`. Independent audit reconstructed
  40,000 retained bootstrap schedules/source digests and all 1,800 delete-one
  rows, regenerated the report byte-for-byte, and proved outwardness plus
  adjacent-float tightness for all 348 emitted CP endpoints. The pilot report
  SHA-256 is
  `9c972b8604791f47e7cbbf34e1a9d799dce2fb71c7cc33cbf1c5b089af7dda8b`.
  This remains descriptive only: every final-acceptance decision is null, the
  3,200-dataset final and paired-confirmation stages remain non-executable, and
  Registry/product qualification remains blocked.
- The distinct BCa engine now also powers bounded complete-result selector
  `bca_type7` through Internal/Labs adapter
  `compiled_recipe_v4_cbsem_plan_v2_execution_v12`. It performs exactly one
  base schedule of B refits followed by exactly one no-retry
  N-1 ML refit for every complete-case omission, with covariance denominator
  N-1, stable point-parameter identity, ordered success/failure receipts, and
  top-level cancellation. It derives unclamped midrank bias correction from
  the existing successful bootstrap ledger, complete-delete-one acceleration,
  normal-CDF adjusted probabilities, and Type-7 bounds. Any missing delete-one
  fit blocks all intervals; B=500, boundary bias probabilities, degenerate
  acceleration, singular adjustments, and nonfinite/reversed results are typed
  unavailable. Its validator regenerates every base schedule/source-row digest
  and bit-recomputes the supplied percentile intervals before any delete-one
  fit, so finite witness, digest, interval, original-value, and ordering drift
  cannot influence BCa. Real N-1 estimation and N=10 boundary tests pass, as
  do nine BCa formula/ledger/worker/integrity tests after focused gates caught
  and corrected both an open-interval zero-boundary bug and the incomplete base
  validation. V12 reuses the v11 fail-before-work B/W/N/V/P/D limits, stores the
  unchanged base ledger plus BCa sidecar atomically, and assigns no completed
  wrapper after cancellation. Four frozen canonical tables preserve summary,
  parameter intervals, successful delete-one refits, and typed failures.
  Schema-6 recompiles the resident recipe/model/dataset, reconstructs the
  complete-case frame from the ordered omission partition, reauthenticates every
  base and delete-one digest, and bit-recomputes BCa arithmetic. Native setup,
  readiness, results, and export preserve available, B=500, and incomplete-
  delete-one states and state that raw base/delete-one ML fits were not replayed.
  Focused evidence includes 11 core/resampling/runner tests, atomic Tauri
  save/reopen/tamper and project seed-binding tests, 204 TypeScript/native tests,
  TypeScript compilation, and an independent end-to-end P0/P1 approval. Registry
  state and qualification remain unchanged; final coverage, paired confirmation,
  packaged-Windows, independent-review, and promotion receipts remain required.
- A separate non-product ordinary-ML nested-model LRT prerequisite now proves
  strict nesting only when two compiled exact plans have identical raw-
  listwise scientific surfaces and the restricted plan is obtained solely by
  interior free-to-fixed restrictions or a coarser same-family equality
  partition. Independent free-dimension reduction must equal the fitted
  degrees-of-freedom difference. The comparison consumes compiled artifacts
  plus compiled-moment results rather than caller-authored fit receipts, binds
  full data/moment authority, reconstructs the scientific model hash, verifies
  fixed/open-domain/equality estimates, bit-recomputes `S - Sigma`, the exact
  covariance/mean ML objective, and `chi-square = N * objective`, and uses the
  direct chi-square survival tail with a frozen small-negative tolerance.
  Coherent cross-run analysis splices, altered plans/data/moments, boundary
  restrictions, equality splits, and fit/df/method drift fail closed. Eight
  focused Rust tests pass and an independent repaired-byte audit found no P0/P1.
  This prerequisite has no runner, schema-6, Tauri, TypeScript, registry, or UI
  caller; it is not yet a customer model-comparison workflow or qualification
  claim, and the historical synthetic multigroup/invariance preview remains
  permanently ineligible.
- A quarantined engine-only two-group CFA prerequisite now performs genuine
  joint configural and metric-loading ML on one shared raw/listwise row
  authority. It binds exact group scalars, selected/listwise/null row digests,
  ML-N covariances, persisted-but-not-consumed observed means, plan/model/data
  identities, and method/constraint scopes. Configural dimensions remain
  group-local; the metric step unions only corresponding free loadings while
  preserving within-group equality labels, fixed markers, domains, and group
  moment authority. The joint objective is `sum(N_g / N * F_g)`, total
  chi-square is `sum(N_g * F_g)`, dimension reduction must equal the positive
  degrees-of-freedom increase, and the LRT uses the direct chi-square survival
  tail with bounded negative-roundoff handling. Seven focused Rust tests pass,
  the legacy single-group hand equivalence regression remains green, and an
  independent source audit found no P0/P1. A validation-only unequal-N
  (`N_A=180`, `N_B=220`) two-factor oracle now independently fits the same
  configural and metric-loading models in Python/SciPy: `D/df` moves from
  `26/16` to `22/20`, the engine/reference LRT is approximately
  `21.62648` on four degrees of freedom (`p=0.000237814`), the maximum
  parameter difference is `2.93e-6`, and the label-only A/B swap is invariant
  within `5.47e-13`. The first report was rejected because import-assigned
  random dataset UUIDs made its full scientific digest nonreproducible. A new
  append-only v2 report freezes distinct dataset UUIDs before compilation,
  proves two byte-identical executions, explicitly supersedes rather than
  overwrites the rejected report, and passed a second independent P0/P1 audit.
  Its manifest SHA-256 is
  `02b0cd12d277115d16db4a587f4ff79746502f758b0fff1987adfc8ba7e2b1af`
  and report SHA-256 is
  `ff26e1306f91ff2769c449c70bd095164104294fd6ee98f289cde7fbd9fd78ee`.
  CFI/RMSEA remain typed unavailable; no recipe, runner, archive, CLI,
  TypeScript, UI, or legacy synthetic payload exposes this prerequisite.
  R/lavaan is not installed on the evidence host and no result was fabricated;
  an executed lavaan cross-check, strong/scalar/strict sequences, partial
  releases, persistence, coverage, and product qualification remain required.
- A transparent NumPy/SciPy CB-SEM matrix-input oracle and compatibility-only
  QualificationSpec V2 work factory now cover hand recovery, analytic-gradient
  checks, raw/covariance/scaled-correlation equivalence, stable parameter IDs,
  and typed matrix failures. The dedicated matrix-input cell is registered
  separately as `partial` coverage, `absent` evidence, and a Labs surface; it
  does not borrow readiness from the broader raw-ML cell. This remains
  unadmitted work evidence: all receipts are empty, no second maintained
  external SEM reference exists, and
  `qualification_ready`/`promotion_allowed` remain false.
- The Internal/Labs CB-SEM Recipe-v4 path also has a separately versioned raw
  continuous single-group CFA mean-structure v4 slice. It executes explicit
  observed intercepts and an optional marker-identified latent mean with a
  fixed marker-intercept anchor through joint covariance-and-mean ML. A
  live-generated, identity-bound product fixture matches a transparent
  NumPy/SciPy oracle for one just-identified one-factor, three-indicator
  microcase, including 11 stable-ID parameters, standard errors, means,
  covariance, objective, convergence, and gradient. This is not a qualification
  receipt or breadth claim; broader identified CFA shapes, a second maintained
  SEM implementation, simulations, archive/export/package evidence, and review
  remain open.
- A bounded Internal/Labs CB-SEM Recipe-v4 continuous-raw mean-replacement v1
  path now executes point-only, unweighted, single-group, mean-structure-off
  models without modifying the source dataset. It retains rows missing every
  modeled value, fills cells from per-variable observed means, records exact
  variable/case/cell treatment receipts and five-percent/above-fifteen-percent
  warnings, and validates native canonical schema-6 append/readback. It rejects
  integers outside the exact `f64` range, polls cancellation during preparation
  and covariance work, and is bounded to 100,000 rows, 300 variables, 10 million
  modeled cells, and one million imputed cells. This path is unregistered and
  unqualified; the separate PLS-oriented validation oracle is work-only and
  does not complete PLS or general missing-data parity.
- Weight-declaration v1 now preserves and resolves the existing scientific
  distinction between case, frequency, and sampling weights, including exact
  sampling normalization, dataset identity, stable variable identity, and
  physical source column. Recipe-v4 PLS and CB-SEM preflight return
  kind-specific corrective diagnostics and schema 6 binds declarations to the
  independently stored recipe/model digest relationship. No weighted
  Recipe-v4 estimator is enabled, legacy WPLS remains case-only and unchanged,
  and no authoring control was added because the live schema-3 native model
  cannot yet persist SemModelV4 data-binding state without creating a second
  scientific authority. A fully coordinated rewrite of an unsigned archive
  and all of its self-contained digests remains an explicitly unauthenticated
  boundary rather than a claimed tamper-detection capability.
- An Internal-only CB-SEM product-indicator moderation foundation now compiles
  a distinct immutable source plan, expands declared-marker all-pairs products,
  and executes the expanded model through the exact parameter-table engine.
  It rejects endogenous or cross-loaded source factors, resampling, drifted
  optimizer settings, nonfinite products, and oversized materialization before
  allocation; v1 is bounded to 81 product columns and 10 million product cells.
  A real runner test binds the separate source-moderator and inner-ML receipts,
  and an independent NumPy/SciPy oracle checks the transformation and interaction
  estimate. Its identity is deliberately distinct from the existing LMS cell
  and is unregistered, so it inherits no LMS evidence and has no Labs or
  Standard activation. Registry governance, full inference/fit semantics,
  schema-6/export/frontend/package/performance evidence, broader simulations,
  a second maintained implementation, and independent review remain open.
- The live model workspace includes a synchronized read-only Parameter Table
  with typed diagnostics and stable source links back to the canvas.
- The staged Internal/Labs project-schema-6 contract is corrected across
  `qpls-core`, `qpls-project`, and Tauri. Origins are truthful (`new_project` or
  source-bound `upgraded_copy`); authoring-integrity-checked
  `sem_model_v4_draft` payloads may be stored but cannot bind Recipe 4, compile,
  or execute; schema-1-through-3 recipes remain immutable historical envelopes;
  legacy results carry an explicit `bound` or `unbound_legacy` recipe
  relationship; and migration never fabricates Recipe 4.
- Schema 6 retains source-preserving upgrade-copy execution, future-version
  read-only inspection, and source-digest-bound atomic canonical-result append.
  Append preserves origin, draft payloads, historical recipe envelopes, and
  historical results while retaining cancellation-before-commit,
  concurrent-writer exclusion, rollback retention, and exact post-write reopen
  validation. Focused verification is green: 12 `qpls-core` tests, 31
  `qpls-project` tests, and one targeted Tauri desktop test.
- Live `.qpls` writing and the normal product service remain schema 5. This
  staged contract does not complete Standard cutover or Registry promotion.
- A strict read-only schema-6 ZIP codec now validates exact archive entries,
  manifest checksums and document identity, canonical schema-6 JSON, resident
  Arrow data, legacy/v2 dataset fingerprints, and dataset lineage. The legacy
  project loader rejects schema 6 explicitly instead of returning a partial
  future-project projection. The codec is green across seven adversarial tests
  and the complete project regression suite. The reader itself never writes or
  activates a project.
- The Internal/Labs upgrade assistant now writes a real schema-6 ZIP only to a
  new destination. Before commit it re-derives the authoritative migration and
  binds every source-derived model, dataset, layout, historical recipe/result,
  and origin field; it streams the validated Arrow bytes unchanged, writes an
  exact manifest, reopens the temporary and persisted archive through the
  strict reader, and rechecks the unchanged source. Cancellation, validation
  failure, source drift, and publication races use identity-aware cleanup that
  never deletes a racing replacement. Verification is green for 10 writer
  tests, 7 strict-reader tests, 8 native assistant tests, all 139
  `qpls-project` library tests plus its integration test, and the desktop
  library check.
- A separate Internal/Labs native command and strict TypeScript service can
  inspect that schema-6 ZIP as a typed read-only snapshot. Its 2 Rust tests,
  16 focused TypeScript tests, and full TypeScript build pass. It exposes no
  save, autosave, backup, or recovery path.
- The Settings Labs surface now exposes one isolated read-only schema-6
  session over that strict service. It supports native browse and an exact
  path, retains the full project document plus resident-dataset summaries,
  prevents a late asynchronous open from reactivating a closed session, and
  never enters the Standard/schema-5 store. The UI states that the session is
  not compilable, runnable, or savable. Within that detached session, a strict
  Labs model-authority editor can insert/replace exact SemModelV4 draft JSON and
  promote the selected draft with its current CAS digest. Every mutation is
  visibly `not_persisted`; closing discards it and blocks late asynchronous
  reactivation. The consolidated Wave-1 frontend gate is green across 12 test
  files and 65 tests, and the full TypeScript build passes.
- Pure schema-6 model-authority mutations now support insert draft, compare-and-
  swap replacement of an unreferenced draft, and exact draft promotion. They
  clone the document, preserve every non-model lane, reject stale identities or
  invalid authoring/readiness, and freeze any model referenced by a current
  Recipe 4 or canonical attachment. A strict Internal/Labs Rust/TypeScript
  bridge now executes those mutations in memory and returns an explicit
  `not_persisted` result while preserving every non-model lane. These paths are
  covered by the green 139-test `qpls-project` gate, 2 desktop Rust tests, and
  9 focused TypeScript tests; they are not yet connected to live authoring.
- A strict TypeScript schema-6 mirror validates the
  corrected origin, ready/draft models, historical recipe/result bindings, all
  current Recipe-4 settings and method-config variants, Rust missing/null
  defaults, product-indicator specifications, scalar enums, and signed-zero
  boundaries without recomputing Rust canonical hashes. Its focused 27-test
  suite and full TypeScript build pass.
- One shared Wave-1 fixture now proves the remaining round-trip gate without a
  production-only reconstruction: TypeScript builds the exact Recipe-4/CB-SEM
  request from a real diagram adapter, while a Rust test consumes the same
  request and CSV/import contract through validation, compilation, the real
  CB-SEM executor, native canonical-result construction, schema-6 attachment,
  standalone serialization, and exact reopen/table comparison. The central
  checks pass at 14/14 TypeScript tests and 1/1 Rust test. This evidence is a
  standalone schema-6 document round-trip and does not claim ZIP persistence.
- Schema-6 in-place save and generation save-as remain explicitly unfinished.
  Two provisional save-as experiments were removed after identity/path-race
  review; no unsafe save API is exposed. A race-closed retry requires pinned
  ancestor/parent directory handles and a native relative child create before
  any commit is attempted. Editable live authoring, save, autosave, backup,
  recovery, and Standard cutover remain exit-gate work.
- An internal Labs recipe-v4 PLS job bridge reaches the production point
  estimator with typed status, authoritative cancellation, shared worker/job
  limits, active-project and dataset rechecks, and no partial-result retention.
  A successful job now returns one native-built canonical document validated
  against both the live result contract and schema-6 archive wire contract;
  the internal service can attach that exact document through the atomic,
  source-digest-bound schema-6 writer without rebuilding tables in TypeScript.
  A strict read-only service reopens those attachments, verifies the expected
  file digest before and after parsing, and returns the same canonical payload
  for saved-run comparison.
  This remains an Internal Labs path and does not replace the qualified legacy
  runner or schema-5 product workflow.

### Results, reporting, and customer experience

- `CanonicalResultDocumentV2` supplies typed cells, sections, tables, charts,
  notices, exact option-cell attribution, analytical identity, and run
  provenance.
- Exact run comparison is live in Results and Reports and fails closed on
  incompatible or historical-unattributed documents.
- A format-neutral semantic export projection supports deterministic readback;
  existing qualified CSV/HTML/XLSX writers remain active until per-method
  cutover is qualified.
- Canonical comparison/readback is live only for supported result-backed
  documents. `NativeSavedReport` remains a bookmark containing `resultId`,
  `name`, and `savedAt`, rather than a frozen canonical report. Normal GUI and
  CLI CSV/HTML/XLSX/SVG/Print paths still reconstruct legacy result
  projections, and PNG export is absent. Wave 7 therefore remains open.
- Schema 6 can carry immutable, digest-verified canonical-result attachments,
  and the internal Recipe-v4 PLS path can now produce and append one. The live
  schema-5 writer and Standard shell do not yet activate this path.
- Standard qualification remains registry-driven in the real Calculate
  dialog. Experimental Labs also keeps the 14 established implemented
  workflows selectable (PLS-SEM Algorithm, Consistent PLS, Weighted PLS,
  GSCA, CCA, IPMA, CB-SEM/CFA, PLS bootstrapping, structural-path
  randomization, MICOM/permutation MGA, PLSpredict/CVPAT, NCA, PCA, and
  Regression); Registry evidence controls qualification claims rather than
  removing those bounded workflows. Method-specific readiness still blocks an
  invalid setup, such as a model method without a model or MGA without groups.
- Method Details uses the common nine-section customer template, is accessible
  from setup and Results, and binds Results explanations to the selected
  immutable run rather than current workspace settings.
- The strict customer-copy gate rejects internal promotion and qualification
  language from normal product surfaces.
- Data transformations are non-destructive and versioned in TypeScript and
  Rust, with an accessible live Data workflow. Native recode and
  single-column missing-marker replacement, and z-score standardization
  activate only after exact child version, parent, canonical transformation,
  and lineage checks. Z-score records an explicit sample-(n-1) denominator,
  compensated observed-value statistics, and creates one target column in one
  child version. Add-column, multi-column missing handling, and browser
  mutations remain fail-closed rather than rewriting the active dataset. Row
  sorting is a stable
  presentation-only preview that retains source row numbers and never changes
  dataset bytes, fingerprints, or version history. The reserved project
  lineage is a strict typed contract: malformed declared
  lineage cannot become an empty history, v5 save/reopen deterministically
  replays reconstructable transforms against resident Arrow data, and schema 6
  separately validates descriptor bindings without claiming raw replay.
  Missing lineage remains a backward-readable unknown-ancestry state,
  referential-only legacy operations are not fabricated as replay evidence,
  and unknown nonreserved layouts are preserved.
- The live model Properties pane is organized into Model, Parameter,
  Appearance, and Data Binding sections with Basic/Expert progressive
  disclosure, real typed readiness/workload facts, and keyboard tab
  navigation. Existing Arrow-key canvas movement now persists into model state
  with an undo checkpoint instead of remaining a temporary visual change.
  Current Chromium production-preview evidence passes selection, drag, pan,
  zoom, real preflight, and deterministic fixture reload for both the
  20-construct/80-indicator applied profile and the 100-construct/300-indicator
  stress profile. The applied p95 measurements are 211.80 ms open and 76.84 ms
  browser-input-to-painted-edit response with 64 minimum FPS; stress is 615.96
  ms open and 73.42 ms edit response with 64 minimum FPS. The receipt does not
  claim a saved-project archive reopen; full scientific author/reopen authority
  and broader packaged accessibility evidence remain open.

### First parity-specific correction

- The documented post-hoc PLS technical minimum sample-size result is distinct
  from prospective Monte Carlo power analysis.
- Its current exact cell is
  `qpls3.pls.posthoc_technical_minimum_sample_size`, version
  `pls_posthoc_technical_minimum_sample_size_v2`, with partial coverage,
  engine-only evidence, and Experimental Labs surface.
- Its compatibility-only QualificationSpec V2 is now linked to that exact cell
  and has zero accepted receipts.
- It must remain outside Standard until the official significance-test
  source/configuration and no-significant-path behavior, immutable source-bound
  evidence, and packaged acceptance are closed. New executions now require an
  explicit typed recipe-level Labs opt-in; ordinary PLS and bootstrap runs no
  longer attach the result automatically.
- Prospective Monte Carlo power remains a separate unavailable beyond-parity
  experiment and cannot satisfy the SmartPLS parity obligation.

### Bounded assessment and resampling foundations

- Recipe-v4 PLS now preserves compiled structural-control identity through the
  estimator, a dedicated canonical `control_estimates` table, and archive
  validation that binds endpoint, label, coefficient, and section ownership.
  Current adapter generations v5/v6 also require exact point-estimate scale
  attribution and deterministic Path/Factor convergence receipts with ordered
  Mode-A, Mode-B, and fixed-block semantics. Legacy v3/v4 documents remain
  readable under an explicit allowlist; unknown adapter identities fail
  closed. Focused estimator, runner, project, desktop, TypeScript, and tamper
  gates are green, but this is result-contract progress rather than full PLS
  cell qualification.
- A bounded Internal execution vertical is now wired in source for the exact
  Labs cell `qpls3.pls.nonlinear_quadratic`, method
  `pls_quadratic_nonlinear_effects_v1`, and Recipe-v4 adapter
  `compiled_recipe_v4_pls_plan_v2_execution_v7`. It reuses the immutable base
  PLS plan and reports fixed-score, centered-squared diagnostics for every
  structural predecessor under raw-data Path or Factor weighting. PCA, fixed
  scoring, score-execution receipts, PlsPm-only convergence receipts, the
  post-hoc sample-size payload, resampling, term-specific authoring, and chart
  surfaces remain excluded or fail closed.
- The v7 canonical contract owns exactly
  `nonlinear_quadratic_diagnostics`, `nonlinear_equation_fit`, and
  `nonlinear_method_scope`. Schema-6, native, and TypeScript source validators
  bind method/adapter identity, primary/base ownership, table and row order,
  dataset fingerprint, numeric coherence, the one exact scope warning, and the
  absence of reserved mixed-family artifacts. The format-neutral semantic
  export preview preserves those tables through readback but intentionally
  invokes no file writer.
- Completed evidence at this checkpoint includes an independent source-contract
  audit with no P0/P1 findings after hardening; green combined Rust compilation;
  2/2 core, 1/1 runner, 1/1 schema-6, and 3/3 desktop focused tests; 31/31 focused
  TypeScript parser tests; the full 164-file/1,227-test frontend suite; full and
  production-graph TypeScript checks; and a successful Vite production build.
  The Registry cell nevertheless remains `partial` / `engine_only` / `labs`;
  this bounded vertical does not
  satisfy the attached plan's full eight-level gate. Remaining work includes
  a second independent computational reference or approved exception,
  qualification-scale simulation and adversarial evidence, real
  CSV/XLSX/HTML writer readback, full save/recovery/version coverage, native
  GUI/CLI parity, packaged offline Windows acceptance, and independent
  scientific review.
- The source-tier factory has regenerated and validated exact engine, archive,
  and native receipts for `qpls3.gsca.als`,
  `qpls3.assessment.cca_residuals`, `qpls3.assessment.ipma`, and
  `qpls3.standalone.nca`. CCA, IPMA, and NCA remain truthfully
  `native_qualified`. GSCA additionally passed a fresh supervised release-build
  desktop run with 29/29 method-scoped checks, exact process-tree cleanup, 52
  observed internal requests and zero external requests, 19 retained
  screenshots, XLSX and project-archive bindings, and independent method-audit
  validation. Its current packaged and method-audit identities are retained at
  `validation/results/method_factory/gsca_als_v2/packaged_acceptance.identity.json`
  and
  `validation/results/method_factory/gsca_als_v2/method_audit.identity.json`;
  the Registry therefore records GSCA as the first `release_qualified` option
  cell. This is a bounded point-estimation receipt, not full GSCA coverage:
  bootstrap and inference remain absent, the cell remains partial and Labs-only,
  and Standard stays fail-closed. Normal CCA and IPMA execution still stops at
  the absent `qpls3.pls.algorithm` dependency. The hidden debug-only
  qualification route remains unavailable in release builds.
- PLS resampling now uses one canonical typed identity for outer loadings,
  outer weights, paths, direct/indirect/total effects, and R-squared. Current
  indexed-resampling archives require the complete bit-exact identity-to-point
  map; older versions retain their legacy reader. HTMT bootstrap inference now
  selects bias-corrected percentile or percentile intervals and one-tailed
  upper or two-sided tests through typed recipe settings, with exact
  recipe-derived archive validation and unchanged default wire. General PLS
  bootstrap additionally supports explicit greater- or less-sided selection
  through a default-omitted typed setting and a null-centered, plus-one test
  receipt derived from the same fixed successful-replicate ledger. The setting
  and receipt survive schema-6 save/reopen, are validated before legacy payload
  deserialization, render with truthful selected-tail copy, and export through
  the CLI; the legacy two-sided normal-reference payload remains byte-
  compatible. The archived receipt contains aggregate counts rather than raw
  replicate draws, so reopen proves identity/accounting/arithmetic but cannot
  independently recompute exceedance counts. CCA now emits canonical pairwise
  observed/reproduced residual diagnostics and an exact failure ledger that
  survive append/reopen. None of these bounded contracts supplies the
  simulations, external references, package receipts, or review needed for
  Registry promotion.
- `pls_score_execution_v2` now implements bounded Internal Recipe-v4 Standard
  and Individual initialization for estimated blocks and SemModel-owned Unit
  and Custom fixed scoring. `none`, `sum_to_one`, and `unit_variance`
  normalization execute end to end with exact requested/resolved/final weight
  identities, signed-sum and signed-zero-safe validation, canonical result
  tokens, schema-6 reopen validation, and mixed/fixed-only iteration
  accounting. A required current-generation scale receipt now separates each
  fixed scoring coefficient from the later score centering/sample-SD scaling
  and binds bit-exact effective unit-score weights through runner, canonical,
  Rust archive, and strict TypeScript live/saved readers; current receipt and
  table identities, ownership, order, tokens, and preprocessing attribution
  are cross-bound while historical generations retain compatible optional
  reads. Configured and fixed Path or Factor execution now supports
  standardized, mean-centered, and unstandardized preprocessing with exact
  transform and scale receipts.
  Resampling, case weights, interactions, higher-order constructs, and v2 PCA
  weighting remain fail-closed; legacy `pls_pm_v1` behavior is unchanged.
- A fresh source-hash-bound desktop harness produced non-promotional current-
  product work evidence for Path/Mode A, Path/Mode B, Factor/Mode A, and legacy
  PCA/Mode A through the real Recipe-v4 compiler, runner, native canonical
  builder, and retained schema-6 append/reopen chain. The frozen source digest
  is `be460f30f03579c8918d8aa79568fe75adde7f3bed28379649fcea74115fb055`;
  all four comparisons retain exact nine-quantity membership and a maximum
  independent-oracle difference of `5.551115123125783e-16`. The validator now
  binds the current schema-6 `origin.kind=upgraded_copy` lineage and rejects
  unexpected historical recipes. The focused factory/current-product/oracle
  suite is green at 39 tests, and verification emits only a source-contract
  candidate receipt (`fad3e6b1...e142`) with zero attached qualification
  receipts. It does not qualify Individual or Unit/Custom execution, broader
  preprocessing/model shapes, simulation, package, performance, or review.
  The PLS cell therefore remains partial, evidence-absent, Labs-only, and
  unavailable in Standard.
- PLS model-fit v2 now has an independent NumPy/SciPy validation-only oracle
  that performs bounded full PLS-PM point and exact-fit refits, plus small
  generative and adversarial work matrices. PLSc and advanced-shape oracle
  breadth, a second independent implementation or approved exception,
  qualification-scale calibration, and all immutable product/package receipts
  remain open. The registry cell therefore stays partial, evidence-absent,
  Labs-only, and non-executable in the catalogue.
- PLSc consistent permutation now has a bounded Internal/Labs source foundation
  for exactly two reflective groups, fixed-size indexed label reassignment,
  full PLSc refits, selected corrected-parameter differences, and combined
  two-sided plus greater-or-equal/less-or-equal directional plus-one inference
  from one fixed successful-replicate ledger. Recipes can now select the
  default two-sided test or an explicit group-A-greater/group-A-less
  alternative. Explicit one-sided runs carry a typed selected-tail receipt and
  exactly one provenance marker bound to the recipe, A-minus-B orientation,
  ordered parameter rows, shared usable denominator, exceedance counts, and
  bit-exact plus-one probabilities; default and historical result bytes omit
  that additive receipt. Current results reopen, render, and export the
  selected alternative semantically through native and CLI surfaces, while
  historical two-sided v1 results remain readable. MICOM completion, broader
  parameter/model support, an independent PLSc oracle, simulations, and
  packaged acceptance remain absent. Its registry cell therefore remains
  absent/absent and is not a parity claim.
- MICOM v3.1 now has a separate typed Internal/Labs execution contract rather
  than reusing the historical combined MICOM/MGA configuration. It requires
  explicit configural-invariance confirmation, uses pooled-reference
  orientation, performs exactly the requested number of deterministic
  size-preserving attempts with no replacement retries, records complete Step
  2/Step 3 accounting, and preserves exact project/native/CLI provenance. Its
  fast pooled/group preflight and a full 5,000-attempt product execution pass.
  The run is work evidence only: product-to-independent-oracle comparison,
  calibration, broader model/data support, packaging, accessibility,
  performance, and review remain open, so the shared registry cell remains
  absent/absent/Labs.
- A genuine two-model PLS comparison engine now runs actual point-estimate PLS
  refits on one immutable shared fold plan, produces indicator PLSpredict
  metrics, paired case-loss CVPAT, equation-level prediction-oriented BIC, and
  two-candidate BIC-based Akaike weights. It rejects same-model or mismatched
  data/fold inputs and supports cancellation and typed failures. This engine is
  not the earlier descriptive saved-run comparison. It now has a strict
  hash-bound runner request, shared desktop job/worker admission, authoritative
  cancellation, six exact-attributed canonical tables, active-project and
  resident-dataset rechecks, and atomic schema-6 append/reopen coverage. GUI,
  public CLI, semantic export readback, broader archive/tamper campaigns, and
  packaged qualification remain open. Its QualificationSpec V2 factory has
  zero receipts and keeps the exact cell absent/absent/Labs.

## Material work still open

1. Qualify the connected schema-6/project and recipe-4 internal path across
   migration, cancellation, save/reopen, comparison, export, and installed-app
   behavior; then cut over Standard and retire schema-5 writes.
2. Make every scientific diagram relation executable for its supported
   estimator, including covariance, residual covariance, constraints, means,
   thresholds, groups, moderation, and identified feedback models.
3. Complete every active PLS option cell, including inference variants,
   assessment, prediction, groups, heterogeneity, and segmentation.
4. Replace the bounded CB-SEM estimator with a general audited parameter-table
   engine and complete fit, bootstrap, comparison, groups, invariance, and
   documented moderation.
5. Build every GUI, CLI, archive, CSV, XLSX, HTML, SVG, and PNG representation
   from CanonicalResultDocumentV2, then qualify semantic readback per method.
6. Complete large-model editor performance, accessibility, Windows scaling,
   cancellation, memory, soak, and maximum-axis profiles on recorded hardware.
7. Promote an option cell only after full coverage and every required evidence
   tier independently pass; then move that exact cell to Standard.
8. Run the final official-documentation delta audit and independent scientific
   reviews for PLS core/inference, PLS advanced/groups, CB-SEM, and resampling.
9. Rebuild current-source qualification receipts for every previously
   overstated method; no archived or historical report may be reused as current
   evidence without passing its source, build, scenario, and environment binds.

## Development gates

Fast foundation gates:

```powershell
npm run qpls:product:foundation
npx tsc -b --pretty false
cargo test -p qpls-core capability_registry_v2 --lib
cargo test -p qpls-cli methods_command_uses_option_cell_registry_instead_of_legacy_validated_labels
```

Broader frontend, Rust workspace, packaged Windows, measured performance,
simulation, and soak gates remain separate. A green structural foundation
command cannot be used as a product-finalization claim.
