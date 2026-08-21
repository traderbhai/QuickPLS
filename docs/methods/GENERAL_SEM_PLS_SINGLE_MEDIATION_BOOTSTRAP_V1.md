# General SEM PLS single-mediation bootstrap v1

Status: **Scoped Standard under `quickpls_v253_streamlined_integration_v1`**. The exact cell retains its
independent identity and every documented boundary.

- Capability owner: `smartpls.mediation`
- Capability cell: `qpls3.pls.general_sem_single_mediation_bootstrap`
- Method version: `general_sem_pls_single_mediation_full_model_case_bootstrap_v1`

## User workflow

Mediation is diagram-native. Draw the substantive structural paths that form the
indirect chain, such as `X -> M -> Y`, then choose **Bootstrapping** from the
unchanged Calculate catalogue. QuickPLS discovers eligible indirect paths from
ordinary directed structural relationships. There is no mediation node, anchor,
or separate mediation workspace.

Covariances, controls, measurement relationships, generated interaction
hierarchy relationships, and indicator or generated-variable endpoints never
create mediation paths. When exactly one eligible indirect path exists,
Bootstrapping selects this cell; two or more paths continue to use the existing
multiple-mediation bootstrap identity.

## Bounded scientific contract

The cell performs one indexed full-model case-bootstrap traversal. Every usable
replicate refits the complete stage model, and the path-specific indirect effect
is recomputed from the replicate coefficients. The point estimate, percentile
two-sided Type-7 interval, usable/failure ledger, deterministic seed policy, and
worker-independent replicate indexing use the existing General SEM bootstrap
conventions.

The v1 boundary is a single-group recursive composite PLS model with raw numeric
data, listwise deletion, no weights, exactly one discovered indirect path, no
interaction or HOC terms, and no authored conditional probes. Causal mediation,
counterfactual effects, cyclic models, BCa, studentized intervals, groups,
clusters, weights, and non-listwise handling are outside this exact cell.

## Results and identity

Results remain canonical General SEM entries for Direct Effects, Specific
Indirect Effects, Total Indirect Effects, Total Effects, Bootstrap Inference,
and Run Details. The new cell identity prevents a single-path run from being
silently relabelled as the historical multiple-mediation capability.

## Qualification boundary

The compact independent Python/R/product reference and consolidated routing,
native, and export evidence permit activation. The exact schema-6 append/reopen
check runs once against that activated cell and is mandatory for the final
cell-specific receipt. Earlier multiple-mediation qualification remains separate.

SmartPLS documentation is used only as a public terminology and observable
workflow reference. QuickPLS is an independent implementation:
[SmartPLS mediation](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/mediation/).
