# General SEM PLS three-way moderation bootstrap v1

Status: **Scoped Standard under `quickpls_v253_streamlined_integration_v1`**. This supplemental cell
retains a separate identity from the existing two-way bootstrap cell.

- Capability owner: `smartpls.moderation`
- Capability cell: `qpls3.pls.general_sem_three_way_moderation_bootstrap`
- Method version: `general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1`
- Primary point cell: `qpls3.pls.general_sem_three_way_moderation_point`

## Execution contract

PLS Bootstrapping routes an eligible one-term three-way model to this exact
cell. It performs one indexed full-model case-bootstrap traversal and refits the
complete stage-1/stage-2 model for every replicate. Main effects, all required
pairwise terms, and `X x W x Z` are estimated jointly rather than holding the
point-run scores fixed.

One shared usable/failure ledger governs the three-way coefficient, conditional
`X x W` effects, and the `W x Z` simple-slope grid. Inference is two-sided
percentile Type-7 under the existing deterministic seed and worker-index
conventions. Continuous moderators use standardized `-1/0/+1`; correctly coded
binary moderators use their actual `0/1` categories.

## Bounded scope

The cell has the same one-three-way-term, two-stage, strong-hierarchy,
single-group, composite-PLS, raw-data boundary as its point cell. It excludes
BCa, studentized and one-sided intervals, retries that change replicate indices,
fourth-order effects, HOC interactions, three-way moderated mediation, groups,
weights, clusters, permutation, and Johnson-Neyman output.

## Results and qualification

Applicable canonical Results entries are Three-Way Moderation, Conditional
Effects, Simple Slopes, Bootstrap Inference, and Run Details. The diagram
overlay highlights the focal path and both moderators without persisting a
visual anchor as scientific content.

Its activation receipt binds the compact complete-bootstrap replay, fixed
probes, failure ledger, deterministic worker, cancellation, result-navigation,
and export evidence. The exact archive append/reopen check runs once after
activation and is mandatory for finalization; packaged smoke remains a separate
product release step.

References:

- [SmartPLS moderation](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/moderation/)
- [SmartPLS bootstrapping](https://www.smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/bootstrapping/)
