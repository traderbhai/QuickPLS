# General SEM PLS three-way moderation point v1

Status: **Scoped Standard under `quickpls_v253_streamlined_integration_v1`**. This remains a bounded
one-term strong-hierarchy cell.

- Capability owner: `smartpls.moderation`
- Capability cell: `qpls3.pls.general_sem_three_way_moderation_point`
- Method version: `general_sem_pls_three_way_moderation_point_v1`

## User workflow

Moderation is authored on Canvas against the focal structural path. A two-way
effect appears as a small `x` anchor on that path with a dashed moderator
connector. Adding a second moderator to the parent interaction defines one true
three-way term. The anchor and connector are presentation only; the persisted
scientific authority remains `SemModelV4 interaction_v2` with ordered operands
and the original focal relation.

Users can drag a moderator onto an eligible path, use the path context menu, or
press `M`. Generated product constructs are hidden in the normal diagram and can
be inspected only through Expert/Diagnostics presentation. None of these visual
objects becomes a structural relationship or participates in mediation discovery.

## Bounded scientific contract

The v1 cell admits one three-way term per model, PLS two-stage score
construction, and strong hierarchy. It jointly estimates the main effects,
`X x W`, `X x Z`, `W x Z`, and `X x W x Z`. The canonical output includes the
three-way coefficient, conditional `X x W` effects at fixed `Z` probes, and
simple slopes of `X` over the `W x Z` probe grid.

Continuous moderators use standardized `-1`, `0`, and `+1` probes. A binary
moderator is admitted only when its observed categories are coded `0/1`, in
which case those actual categories are used. Results use accessible tables and
two-dimensional simple-slope charts; a 3D chart is not required.

The cell excludes a second three-way term, fourth-order interactions, HOC
interactions, three-way moderated mediation, groups, permutation, unsupported
weights or missing-data handling, and Johnson-Neyman regions. The separate
observed-variable PROCESS workflow is not reinterpreted as latent-variable
PLS-SEM.

## Identity and qualification

The point method has its own Registry identity and cannot be substituted with a
two-way moderation result or reconstructed from UI state. Its activation receipt binds the compact Python/R/product matrix, routing,
canonical result, and native workflow evidence. The exact archive append/reopen
check runs once after activation and is mandatory for finalization.

SmartPLS documentation is used only as a public terminology and observable
workflow reference. QuickPLS is an independent implementation:
[SmartPLS moderation](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/moderation/).
