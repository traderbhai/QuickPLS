# Indexed Resampling Engine Specification v2

Status: superseded by v3. Legacy v2 results remain readable and do not contain BCa summaries.

## Bootstrap Sampling

For a PLS model with `N` complete cases across all model indicators, each replicate draws exactly `N` positions independently and uniformly with replacement from that fixed analysis sample. Rows excluded by listwise deletion in the original estimate cannot re-enter a replicate.

## Reproducibility Contract

- Streams are independently derived from `(master_seed, operation_identifier, replicate_index)` using SHA-256 domain separation and ChaCha20.
- Replicate ordering and analytical output are invariant to worker scheduling and worker count. Worker count remains execution provenance.
- Cancellation discards partial output. Estimator failures retain their replicate index and message.
- PLS inference requires at least two successful replicates and at least 90% of requested replicates.

## Persisted Inference

- Successful raw replicate estimates are transient. Projects store the plan, usable count, failures, and compact summaries.
- Summaries contain the original estimate, arithmetic bootstrap mean, bias, sample standard error, and two-sided Hyndman-Fan Type 7 percentile bounds.
- The optional normal-reference test reports `t = original / bootstrap_standard_error` and `p = 2 * standard_normal_survival(abs(t))`. It is unavailable when the bootstrap standard error is effectively zero. This large-sample reporting convention is distinct from a studentized-bootstrap interval or a resampling-under-the-null test.
- Parameter identities are canonical JSON tuples of `(parameter_kind, [identifier_parts])`. This is the wire-format change from v1 and prevents delimiter collisions.
- Loadings, weights, paths, direct/indirect/total effects, and R-squared values are included after replicate score-sign alignment to the original solution.

Studentized intervals, BCa intervals, permutation, simulations, and 10,000-replicate qualification remain v0.4 release gates in this historical version.

Deterministic delete-one orchestration and transient PLS parameter integration are specified separately in `JACKKNIFE_ENGINE_V1.md`; BCa acceleration and interval publication remain pending.
