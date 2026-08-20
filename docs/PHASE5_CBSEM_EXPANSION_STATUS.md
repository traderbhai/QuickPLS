# Phase 5 CB-SEM expansion status

Status: bounded ordinary-ML foundation release-qualified; bootstrap-v2 candidate implemented; broader Phase 5 expansion is not complete.

The current repository supports one honest CB-SEM claim: `cbsem_ml_v1` is
release-qualified for the documented single-group, continuous reflective,
raw-data ordinary-ML CFA and recursive-SEM scope. Current method-audit and
packaged-acceptance identities support that bounded foundation only; they do
not promote any Phase 5 expansion method.

Passing `validation/phase5_cbsem_expansion_audit.py` means the source-bound
inventory below is truthful. It does **not** mean the absent methods work, that
Phase 5 is complete, or that expanded CB-SEM competitor claims are admissible.

## Current evidence boundary

| Track | Current state | What exists | Remaining boundary or blocker |
|---|---|---|---|
| CB-SEM/CFA ordinary ML | Release-qualified | Direct deterministic ML optimization, bounded fit and parameter tables, strict archive evidence, native setup/results/export evidence, current method audit, and genuine packaged Windows acceptance | Scope remains bounded; this evidence cannot promote bootstrap, invariance, MGA, comparison, moderator, robust ML, ordinal WLSMV, FIML, or general constraints |
| Nonparametric CB-SEM bootstrap v2 | Implemented candidate; qualification absent | Typed full raw complete-case resampling, complete ordinary-ML refits, deterministic indexed draws, exact failure accounting, percentile Type-7 intervals, strict project recomputation, native setup/results/export, and independent Python/lavaan source | Final preregistered simulation, current identity-bound native/export evidence, method audit, and genuine packaged Windows acceptance are not complete; v1 remains a separate ineligible analytical preview |
| Measurement invariance v2 | Absent | Synthetic v1 step diagnostics | v1 changes pooled fit values arithmetically; it does not fit configural, metric, or scalar constrained group models |
| CB-SEM MGA v2 | Absent | Pooled-fit group summaries in v1 | v1 apportions one pooled chi-square by group size; there are no group-specific/equality-constrained refits or qualified stratified bootstrap |
| CB-SEM model comparison v1 | Absent | AIC and BIC fields on individual fits | There is no typed two-model recipe, shared-data check, nesting validator, constrained refit, or comparison result |
| CB-SEM moderator v1 | Absent | PLS and PROCESS moderation elsewhere in the product | Validated CB-SEM blocks interactions; no LMS likelihood engine exists, and another estimator cannot evidence LMS |
| Robust ML | Absent | A typed `RobustMl` enum value | Recipe validation rejects non-ordinary-ML CB-SEM; no robust covariance/scaled statistic or difference test is implemented |
| Ordinal WLSMV/polychorics | Absent | A typed `Wlsmv` enum value | Recipe validation rejects it; no ordinal thresholds, polychoric correlations, weight matrix, or corrected test statistic is implemented |
| FIML | Absent | Listwise deletion | The missing-data policy exposes only listwise deletion; there is no case-pattern likelihood or missingness contract |
| General equality constraints | Absent | Invariance-step names and marker-fixed parameters | There are no typed fixed/equality constraints, shared parameter labels, constrained parameter mapping, or release ledger |

The v1 bootstrap, multigroup, and invariance payloads are ineligible previews.
They must never be relabeled, migrated, or cited as v2 evidence.

## Rank 3 General SEM candidate isolation

Two additive schema-6 candidate identities now have a private source-only
integration path:

- `qpls3.cbsem.general_sem_ml / cbsem_general_sem_ml_v1`
- `qpls3.cbsem.bootstrap.recursive_sem / cbsem_exact_recursive_sem_case_bootstrap_v1`

The internal native job recompiles the strict resident `SemModelV4` and
`AnalysisRecipeV4`, reuses the qualified V2 ML optimizer and the existing
indexed no-retry bootstrap scheduler, and projects one typed canonical
parameter/fit/identification/inference authority. The same deterministic table
projection is used during schema-6 append and fresh reopen, so payload/table,
plan, model, recipe, dataset, cell, and receipt drift fails closed. Cancellation
is checked before canonicalization, before job publication, and by the atomic
rollback-aware append path.

This is not a product or release claim. Both cells remain absent from the
Capability Registry, have no frontend route, have no registered Tauri command,
and are intentionally unavailable to users until the remaining scientific,
preflight, UI, qualification, performance, accessibility, export, and packaged
Windows gates pass. Existing `qpls3.cbsem.ml` and
`qpls3.cbsem.bootstrap` identities and results are unchanged.

## Fail-closed dependency plan

The machine-readable source of truth is
`validation/phase5_cbsem_expansion_contract.json`. The safest implementation
order is:

1. Preserve the bounded release-qualified ordinary-ML foundation as a strict dependency without broadening its claim.
2. Implement full-refit, seeded, worker-invariant case bootstrapping with a
   failure ledger and preregistered coverage simulation.
3. Add a genuine constrained optimizer and typed equality-constraint model.
4. Use that optimizer for configural/metric/scalar two-group invariance, then
   add the preregistered structural-path MGA workflow and its stratified
   bootstrap.
5. Build mechanical nesting validation and two-model comparison on identical
   retained cases.
6. Implement CB-SEM moderator analysis as its specified raw-data LMS estimator,
   not as factor-score OLS or PLS two-stage moderation.
7. Treat robust ML, ordinal WLSMV/polychorics, and FIML as separate scientific
   projects. Each needs its own method identity, numerical reference,
   simulations, boundary policy, persistence, native workflow, and release
   evidence.

Every promoted track must pass all method-factory tiers: frozen method spec,
independent numerical reference, simulation, boundaries, strict persistence,
native setup/results/export, method audit, and genuine packaged Windows
acceptance. Source markers, enums, preview payloads, or manual calculations are
never substitutes.

## Reproducing the audit

Run:

```powershell
python validation/phase5_cbsem_expansion_audit.py
python -m unittest validation/test_phase5_cbsem_expansion_audit.py
```

The audit writes one source-bound identity report per track and an aggregate
report under `validation/results/phase5_cbsem_expansion/`. The aggregate must
continue to report `phase5_complete: false` until every required track is
independently `release_qualified`.
