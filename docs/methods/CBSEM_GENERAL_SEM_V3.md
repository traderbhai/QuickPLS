# CB-SEM General SEM V3

Status: scoped Standard Version 2.50 capability. This document freezes the two
bounded Rank 3 identities and their exclusions under the user-approved
streamlined integration profile.

## Exact identities

- Point estimation: `qpls3.cbsem.general_sem_ml` /
  `cbsem_general_sem_ml_v1`
- Recursive case bootstrap: `qpls3.cbsem.bootstrap.recursive_sem` /
  `cbsem_exact_recursive_sem_case_bootstrap_v1`

Both cells require a newly created schema-6 `general_sem_v1` project containing
one promoted resident `SemModelV4`, one digest-bound Recipe V4, and raw
continuous single-group data with listwise deletion. The model must contain at
least one structural regression and must be recursive. Ordinary CFA retains its
existing qualified identities.

The point cell executes ordinary covariance-structure maximum likelihood by
reusing the established V2 optimizer. The complete resident parameter table is
authoritative: finite fixed values, free rows, compatible `equality_label`
groups, and finite open row bounds are executable. Derived rows, explicit
constraint objects, mean structures, feedback, composite or derived variables,
weights, clusters, strata, groups, categorical indicators, requested derived
effects, and conditional probes fail closed without rewriting the model.

The bootstrap cell adds 500 through 10,000 deterministic indexed no-retry
case-resampling refits, fixed two-sided 95% percentile Type-7 inference, an
ordered success/failure ledger, and the frozen 90% usable-refit threshold. It
does not provide BCa, studentized, one-sided, parametric, residual, robust,
weighted, clustered, multilevel, or CFA-bootstrap semantics.

Execution is admitted only after the exact requested cell and every dependency
cell are available in Capability Registry V2 on Standard, while historical
Labs-authored archives retain read-only compatibility.
The native boundary independently resolves the regular local archive, resident
dataset, promoted model, Recipe V4, scientific digests, inference owner, and
Registry evidence. Results use CanonicalResultDocumentV2 tables, exact-cell
schema-6 append and strict readback, and the shared provenance-bound export
dispatcher. Cancellation commits no analytical result.

## Evidence boundary

The Version 2.50 Standard decision uses the user-approved streamlined profile:
the consolidated integrated Rust/frontend regression, production build, and
short Standard-access smoke replace the previously planned independent and
per-cell packaged qualification. This does not expand either exact predicate or
claim numerical identity with SmartPLS.
