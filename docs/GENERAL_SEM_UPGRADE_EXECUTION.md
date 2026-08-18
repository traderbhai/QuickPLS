# General SEM upgrade execution

This document tracks the isolated `codx/sem-platform-upgrade` implementation.
It is a checkpoint record, not a claim that the General SEM roadmap or a
Standard-tier method is complete.

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
remain blocked. There is no native General SEM execution command and no complete
new-project creation -> model/data binding -> calculate journey. This checkpoint
therefore remains Labs-only; it is neither roadmap completion nor a Standard
promotion.

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
above. They do not imply a user-visible native calculation workflow or a
Standard promotion.

## Remaining Labs blockers

The roadmap is not complete. The following must remain blocked until their
compiler, execution, canonical-result, persistence, and qualification evidence
is implemented and rerun:

- every resampling cell outside the bounded PLS case-bootstrap qualification,
  including BCa, one-sided and studentized inference, conditional probes, lazy
  materialization, and any unfrozen interval or tail semantics;
- conditional-effect probes and moderated-mediation execution;
- execution of multiple interactions in one model, three-way interaction
  authoring, and conditional interaction surfaces;
- higher-order-construct expansion and staged estimation beyond the preserved
  legacy bounded workflow;
- the CB-SEM General runtime adapter, including feedback/nonrecursive models;
- strict parsing of raw legacy-workspace `interaction_v2` payloads outside the
  schema-6 authority path;
- wiring the authoritative native preflight into the calculation workflow;
- a resident-authority-valid Rust output -> schema-6 archive -> frontend
  golden readback fixture; current frontend fixtures prove strict parser
  behavior but are not substitutes for that cross-runtime qualification;
- registering and independently qualifying an exact combination-specific
  Capability Registry V2 option cell for General SEM multiple-mediation,
  full-model percentile bootstrap instead of treating the existing generic
  bootstrap cell as qualification of that combined workflow;
- a native General SEM execution, monitoring/cancellation, canonical-result
  persistence, result-view, and export path; and
- General SEM-aware semantic result comparison and semantic export/readback;
  the current table-oriented projections intentionally do not interpret the
  additive General SEM result extension; and
- end-to-end UI workflow wiring for configuration, estimator selection,
  execution, progress/cancellation, results, export, reopen, and recovery.

No capability should be promoted from Blocked or Experimental Labs solely from
the foundation, point-estimation, or bounded case-bootstrap slices.
