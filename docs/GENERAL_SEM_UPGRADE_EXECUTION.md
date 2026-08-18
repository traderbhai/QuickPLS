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

## Remaining Labs blockers

The roadmap is not complete. The following must remain blocked until their
compiler, execution, canonical-result, persistence, and qualification evidence
is implemented and rerun:

- bootstrap/resampling inference, including interval and tail semantics;
- conditional-effect probes and moderated-mediation execution;
- execution of multiple interactions in one model, three-way interaction
  authoring, and conditional interaction surfaces;
- higher-order-construct expansion and staged estimation beyond the preserved
  legacy bounded workflow;
- the CB-SEM General runtime adapter, including feedback/nonrecursive models;
- strict parsing of raw legacy-workspace `interaction_v2` payloads outside the
  schema-6 authority path;
- wiring the authoritative native preflight into the calculation workflow;
- a native General SEM execution, monitoring/cancellation, canonical-result
  persistence, result-view, and export path; and
- end-to-end UI workflow wiring for configuration, estimator selection,
  execution, progress/cancellation, results, export, reopen, and recovery.

No capability should be promoted from Blocked or Experimental Labs solely from
the foundation or the current PLS point-estimation slice.
