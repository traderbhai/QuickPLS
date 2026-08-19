# General SEM upgrade execution

This document tracks the work that began on `codx/sem-platform-upgrade` and the
current `codx/sem-native-mediation-workflow-v1` integration branch. It is a
checkpoint record, not a claim that the General SEM roadmap or a Standard-tier
method is complete.

This work upgrades the existing QuickPLS application. The integration branch,
short-lived child branches, and separate worktrees are development-isolation
mechanisms only; they do not create a second application or a user-facing fork.
The `general_sem_v1` marker is likewise a project-generation safety boundary
inside QuickPLS, not a separate product format or estimator application.

Baseline commit: `4ad3252639a53b3a586d970f32373d72fe8befc5`.

- `qpls-core`: 174 tests passed at the branch point.
- TypeScript baseline: one pre-existing prop-contract error.
- Frontend baseline: 1,247 passed and 5 pre-existing failures.
- Scientific capabilities remain gated by Capability Registry V2.

## Checkpoint A: implemented General SEM foundation

The following additive contracts are implemented on the upgrade branch:

- Schema 6 has an optional `sem_generation: "general_sem_v1"` authority marker.
  Existing schema-6 documents omit it and retain their previous behavior.
- The marker is valid only with `origin.kind = "new_project"`; upgraded copies
  fail strict Rust and TypeScript validation if the marker is introduced.
- A dedicated Rust constructor creates and strictly reopens blank General SEM
  archives through the existing safe publication seam.
- Recipe v4 has an optional, strict `general_sem_config` contract in Rust and
  TypeScript. It carries requested effects, conditional probes, inference, and
  explicit output-limit policy without changing older recipes.
- The scientific model foundation now has deterministic topology compilation,
  cycle/SCC evidence, stable relation/path identities, additive interaction
  authoring contracts, and typed PLS/CB plan boundaries.
- `interaction_v2` preserves ordered two-or-more operand identities, focal-path
  identity, generation method, and hierarchy policy through schema-6 authority,
  diagram projection, save/reopen readback, inspection, cascade/undo, and
  scientific digesting. Legacy estimators reject it rather than flattening it.
- Capability preflight returns explicit Supported, Experimental Labs, or
  Blocked decisions. It does not silently reinterpret unsupported models.
- The native schema-6 preflight command revalidates the project, requires a
  newly created `general_sem_v1` project, binds the exact stored model, and runs
  the Rust compilers. The frontend panel remains a preview until that command is
  wired into the calculation dialog.
- `CanonicalResultDocumentV2` has an optional typed General SEM result extension
  for indirect/aggregate/conditional effects, plot data, higher-order stages,
  CB-SEM fit, and identification diagnostics. Legacy documents omit the
  extension and retain their old serialization behavior.
- The strict qpls-project canonical-result mirror accepts and revalidates that
  extension, so future schema-6 result persistence cannot lose or reinterpret
  General SEM fields.
- Advanced mutations, General SEM configuration, and General SEM results require
  `origin = new_project` plus `sem_generation = general_sem_v1`. Historical and
  upgraded-copy projects retain their existing behavior and cannot be relabeled
  through the new authority paths.
- The estimator compatibility panel renders the PLS and CB decisions, but it is
  intentionally reusable and is not yet wired into the calculation workflow.

### Verified evidence for checkpoint A

| Scope | Verified result | Qualification |
| --- | ---: | --- |
| `qpls-core` full suite | 229 passed | Green for topology, contracts, plans, preflight, and canonical validation |
| `qpls-project` full suite | 169 unit + 2 integration passed | Green for strict schema-6 validation, mutation gates, creation/publication/reopen, and canonical readback |
| `qpls-runner` full suite | 60 passed; 2 intentionally ignored | Green for the bounded General SEM PLS point adapter and existing runners |
| `quickpls-desktop` full suite | 143 passed; 1 intentionally ignored | Green for the native schema-6 bridge and existing desktop workflows |
| Frontend full Vitest suite | 169 files; 1,306 tests passed | Zero failed or skipped |
| Frontend full typecheck | Passed | Repository command `npm run typecheck:full`; equivalent app and Node checks also passed with temporary build-info paths |
| Frontend capability decision | 7 passed | Focused group |
| Frontend PLS/CB capability preflight | 7 passed | Focused group |
| Frontend estimator compatibility panel | 4 passed | Focused group |
| Frontend canonical-result parser | 14 passed | Focused group, including legacy omission and complete extension readback |
| Frontend moderation/authoring regression group | 65 passed | Prior focused verification across 4 files |
| Frontend interaction-v2 canvas/readback group | 171 passed | Focused verification across 7 files, also covered by the full suite |

These are targeted checkpoint results. They do not replace a final integrated
workspace gate, packaging check, or release qualification run.

## Checkpoint B: first PLS multiple-mediation point-estimation slice

The first executable vertical slice is implemented for a deliberately narrow
Labs boundary: a new-project, recursive composite-only PLS DAG with no derived
terms, no conditional probes, inference set to `none`, and eager bounded path
materialization.

Within that boundary the slice:

- compiles the recipe through the proven PLS plan and an additive deterministic
  General SEM topology/effect plan;
- enumerates stable specific paths across parallel and serial multiple
  mediation, while excluding control-role relations from causal decomposition;
- runs production PLS point estimation, reconciles coefficients to stable
  relation identities, and calculates requested specific indirect, total
  indirect, and total effects;
- emits the additive canonical General SEM result section with point estimates
  only; standard errors, intervals, and p-values remain absent by design; and
- fails closed for feedback, unsupported shapes, conditional probes,
  resampling inference, and lazy materialization instead of pretending those
  capabilities executed.

The full runner suite now passes with 60 tests green and 2 intentionally ignored
work-evidence harnesses. The full desktop suite also passes after tightening the
CB-SEM exact-bootstrap archive validator so studentized-table injection and
cross-version relabeling fail closed.

## Checkpoint C: bounded PLS multiple-mediation case bootstrap

The next executable slice adds Experimental Labs inference to the qualified PLS
multiple-mediation boundary. It is intentionally limited to a two-sided case
bootstrap with percentile intervals calculated by the frozen Type-7 quantile
rule. Replicate counts must be between 2 and 10,000 inclusive, and publication
requires at least `max(2, ceil(0.9 * B))` usable replicates.

Within that boundary the slice:

- refits the complete original PLS model inside every bootstrap replicate and
  refuses a stale point result, rather than resampling only derived effects;
- assigns deterministic indexed bootstrap streams so supported worker-count
  changes produce the same scientific result;
- reports specific indirect, total indirect, and total effects with stable
  typed effect identities, usable/exceedance counts, plus-one two-sided
  p-values, standard errors, and percentile intervals;
- binds the canonical inference receipt to the exact recipe, model, dataset,
  compiled PLS/effect plan, capability cell, initialization, bootstrap method,
  quantile rule, summation rule, seed, and requested/effective worker settings;
- makes schema-6 persistence re-resolve the resident Recipe V4, SemModel V4,
  and dataset, deterministically recompile the plan, and reject stale or
  tampered General SEM result authority; and
- mirrors the new receipt and invariants in the TypeScript canonical parser,
  adds JavaScript-safe seed and path-identity preflight checks, and keeps the
  estimator compatibility panel explicit that this is compiler qualification,
  not a native calculation workflow.

BCa, one-sided, studentized, conditional-probe, and lazy-materialization cells
remain blocked. When Checkpoint C closed, there was no native General SEM
execution command or complete project creation -> model/data binding ->
calculate journey. Checkpoint D below adds native execution and result plumbing,
but deliberately does not claim that the primary new-project authoring journey
is connected. Checkpoint C therefore remains a Labs-only engine checkpoint; it
was neither roadmap completion nor a Standard promotion.

### Verified evidence for checkpoint C

| Scope | Verified result | Qualification |
| --- | ---: | --- |
| `qpls-core` full suite | 238 passed | Green for the extended canonical contract and deterministic plan identities |
| `qpls-resampling` full suite | 115 passed | Green for the bounded kernel, deterministic partial-failure ledger, 90% no-result gate, and existing resampling regressions |
| `qpls-runner` full suite | 60 passed; 2 intentionally ignored | Green for the point and bounded bootstrap adapters |
| `qpls-project` full suite | 171 unit + 2 integration passed; authority test rerun passed after label hardening | Green for resident schema-6 authority recompile and strict persisted-result binding |
| Frontend focused shared-parser/readback group | 67 passed | Green for one strict General SEM parser across native, schema-6 result, and archive readers |
| Frontend full typecheck | Passed | Repository command `npm run typecheck:full` |
| Frontend full Vitest suite | 170 files; 1,325 passed | Green after final shared-reader hardening |
| `quickpls-desktop` full suite | 143 passed; 1 intentionally ignored | Green for native-shell compilation and existing desktop regressions |

These results qualify only the bounded Labs compiler/runtime cell described
above. At the Checkpoint C boundary they did not imply a user-visible native
calculation workflow, and they still do not imply a Standard promotion.

## Checkpoint D: connected same-app General SEM project-mode UX

The bounded multiple-mediation slice has native archive-bound execution,
monitor/cancel, canonical result, persistence/reopen, and XLSX plumbing in the
existing QuickPLS desktop application. It is not a second application.

The primary canvas journey is now connected as an opt-in Labs project mode in
the same QuickPLS application. The New Project dialog lets desktop users choose
Standard or General SEM. General SEM first creates a fresh, empty ordinary
desktop project with a new identity, then binds a transient
`general_sem_v1` draft marker to that exact identity. The marker is cleared by
ordinary project creation, open, close, reset, identity drift, or strict
schema-6 activation. It is never written as an unmarked legacy project.

Only that explicitly fresh draft may adapt its newly imported raw dataset and
newly authored canvas into `SemModelV4`. An arbitrary existing project cannot
be adapted, copied, upgraded, or relabelled by the General SEM workspace.
Ordinary Save and autosave are blocked while the transient draft is active;
the General SEM **Save and activate project** action is the only persistence
route for that draft.

The connected authority flow is:

1. The New Project controller creates a fresh empty desktop project, verifies
   its exact identity and empty state, and records only a transient
   `general_sem_v1` draft marker. Raw data is imported and the model is authored
   on the existing QuickPLS canvas.
2. The General SEM workspace adapts the canvas only when the exact draft marker
   is present. It creates a new schema-6 project file marked
   `general_sem_v1`, binding exactly one dataset, promoted model, and Recipe V4.
3. QuickPLS strictly inspects the completed bootstrap receipt and opens the
   exact returned snapshot in the internal schema-6 session. The existing
   native authority resolver then atomically installs the archive's strict
   `SemModelV4` and Recipe V4 as the active same-app canvas authorities. The
   transient draft marker is cleared only through this activation.
4. The compatibility preflight evaluates the exact selected model and General
   SEM configuration against the registered PLS and CB-SEM predicates.
5. Archive-bound job admission reopens the project file, verifies its SHA-256
   and resident project/dataset/model/recipe identities, recompiles the stored
   authority, and then starts the native General SEM PLS job. The existing
   QuickPLS Labs surface monitors progress, supports cancellation and dismissal,
   and publishes only a completed canonical result.
6. QuickPLS appends the result to the same schema-6 archive, strictly reopens it,
   and can export its table projection to XLSX. Stale archives, authority
   mismatches, tampering, cancellation, failed computation, or inadmissibility
   do not publish a partial result.

The workflow is bound to the exact Experimental Labs cell
`smartpls.mediation::qpls3.pls.general_sem_multiple_mediation_bootstrap::general_sem_pls_full_model_case_bootstrap_v1`.
Bootstrap execution requires at least two compiled indirect paths; the narrower
single-path point-estimation behavior remains readable without relabeling it as
multiple mediation. The cell remains registered as
`partial / engine_only / labs` with `qualification_ready = false`. Connecting
the native workflow is implementation progress, not evidence sufficient to
promote the cell to Standard.

### Verified evidence for checkpoint D

| Scope | Verified result | Qualification |
| --- | ---: | --- |
| `qpls-core` full suite | 254 passed | Green after merging the mediation and simultaneous two-way moderation compiler, preflight, plan, and canonical contracts |
| `qpls-estimation` full suite | 166 passed; 1 intentionally ignored | Green for existing estimation plus the joint same- and different-focal two-way moderation point estimator |
| `qpls-project` full suite | 172 unit + 2 integration passed | Green for schema-6 authority, persistence, strict reopen, result binding, and moderation-plan reconciliation |
| `qpls-runner` full suite | 62 passed; 2 intentionally ignored | Green for the bounded mediation point/bootstrap runners and moderation point runner |
| `qpls-resampling` full suite | 115 passed | Green for current bounded deterministic case-bootstrap kernel and regressions |
| Frontend full Vitest suite | 177 files; 1,388 passed | Green after the combined connected project mode, strict reopen/data paging, lifecycle recovery, mediation readback, and moderation readback changes |
| Frontend full typecheck | Passed | Repository command `npm run typecheck:full` |
| Frontend production build | Passed | Repository command `npm run build`; this is not a packaged-Windows acceptance run |
| `quickpls-desktop` full suite | 168 passed; 1 intentionally ignored | Green for native archive authority, project-mode lifecycle, strict paging, cancellation, one-shot result, and canonical point/bootstrap behavior |
| Exact multiple-mediation evidence manifest | Passed; 29/29 source descriptors current | Derived `engine_only`; deterministic simulation replay, 14/14 micro-reference checks, focused registry, and complexity gates passed |
| Exact simultaneous two-way moderation Registry cell | Passed | Registered as `partial / engine_only / labs` with `qualification_ready = false`; native calculation publication remains blocked |

These checks establish bounded engine and workflow-plumbing evidence only. The
same-app new-project authoring and authority-activation journey is connected,
but it remains an Experimental Labs path and does not replace an independent
scientific oracle, cross-runtime qualification fixture, packaged Windows
verification, or Capability Registry promotion audit.

## Remaining Labs blockers

The roadmap is not complete. The following must remain blocked until their
compiler, execution, canonical-result, persistence, and qualification evidence
is implemented and rerun:

- every resampling cell outside the bounded PLS case-bootstrap qualification,
  including BCa, one-sided and studentized inference, conditional probes, lazy
  materialization, and any unfrozen interval or tail semantics;
- conditional-effect probes and moderated-mediation execution;
- native-workflow integration of the simultaneous two-way moderation point
  slice. Its compiler, joint same- and different-focal point estimator,
  canonical-result contracts, TypeScript readback, and exact Registry cell are
  integrated into this branch, but the same-app native calculation workspace
  does not yet execute or publish that moderation cell. Moderation bootstrap,
  three-way execution, authored probes, and moderated mediation also remain
  blocked;
- release hardening for the now-connected same-app General SEM project flow,
  including packaged-Windows roundtrip, crash recovery during draft promotion,
  accessibility verification, and cross-runtime acceptance evidence;
- higher-order-construct expansion and staged estimation beyond the preserved
  legacy bounded workflow;
- the CB-SEM General runtime adapter, including feedback/nonrecursive models;
- strict parsing of raw legacy-workspace `interaction_v2` payloads outside the
  schema-6 authority path;
- an independently generated, resident-authority-valid Rust output -> schema-6
  archive -> frontend golden readback fixture; strict native reopen and
  frontend parser tests are not substitutes for that cross-runtime
  qualification;
- advancing the now-registered exact
  `qpls3.pls.general_sem_multiple_mediation_bootstrap` cell beyond its current
  `partial / engine_only / labs` state: it still needs an independent full-PLS
  refit oracle, public SmartPLS settings comparison, statistical simulations,
  cross-runtime schema-6 golden readback, semantic exports/readback, packaged
  Windows evidence, and accepted QualificationSpec V2 receipts;
- General SEM semantic export qualification beyond the current typed result
  comparison and table-projection readback. CSV, HTML, SVG, PNG, packaged XLSX,
  and cross-runtime semantic readback still require exact capability evidence;
- accessibility acceptance, scaling, large-model performance, memory, soak,
  crash-recovery, and packaged offline Windows evidence for the complete
  workflow;
- exact Registry promotion audit after the broader legacy evidence registry's
  56 stale or source-bound failures are resolved or explicitly dispositioned;
  the exact new-cell manifest passing does not make the overall registry green;
  and
- Standard-quality runtime and workflow slices for simultaneous two-way
  moderation, three-way moderation, moderated mediation, expanded HOCs, and
  general CB-SEM (including separately identified feedback cells).

No capability should be promoted from Blocked or Experimental Labs solely from
the foundation, point-estimation, bounded case-bootstrap, or same-app native
workflow slices.
