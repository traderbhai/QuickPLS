# PLS model-fit adapted Bollen--Stine inference v1

Status: bounded Labs implementation and qualification contract. This contract does not make a
coverage, qualification, or Standard-surface claim. The capability remains
`coverage_state=partial`, `evidence_state=absent`, and `surface=labs` until the
complete QualificationSpec V2 evidence ladder is accepted.

Qualification status: [PLS model-fit v2 qualification status](PLS_MODEL_FIT_QUALIFICATION_STATUS_V2.md).

## Scientific question and references

The procedure asks whether the observed discrepancy between the empirical
indicator correlation matrix and a PLS/PLSc model-implied correlation matrix
is larger than sampling error would plausibly produce when that implied matrix
is the population correlation matrix.

The normative computational reference is Dijkstra and Henseler (2015),
*Consistent and asymptotically normal PLS estimators for linear structural
equations*, equations 35--36 and the algorithm described on page 20. For each
model-implied matrix, centered standardized column observation vectors are
transformed by `Sigma_hat^(1/2) * S^(-1/2)`; equivalently, the stored row matrix
uses `Z * S^(-1/2) * Sigma_hat^(1/2)`. Cases are sampled with replacement, the
model is fully refitted, and the discrepancies are recomputed. Bollen and Stine
(1992) supplies the underlying null-transformed bootstrap rationale. The
current SmartPLS model-fit documentation requires separate saturated and
estimated runs and reports inference for SRMR, d_ULS, and d_G.

References:

- https://doi.org/10.1016/j.csda.2014.07.008
- https://doi.org/10.1177/0049124192021002004
- https://www.smartpls.com/documentation/algorithms-and-techniques/model-fit/

## Frozen v1 input contract

- Raw observations only.
- A schema-preflighted, converged `pls_pm_v1` or `plsc_v2` point estimate.
- The exact `pls_model_fit_v2` point-fit result recomputed from the same
  dataset, recipe, indicator identity/order, and complete-case sample.
- Recursive models currently supported by the v2 point-fit compiler.
- Listwise complete indicator rows; missing cases are not transformed or
  sampled.
- Between 999 and 10,000 fixed primary draws, inherited from the complete
  bootstrap recipe.
- The compiled Labs recipe explicitly records
  `metadata.pls_model_fit_exact_inference=true`; ordinary bootstrap recipes do
  not incur or claim the null-transformed workflow.
- One to 64 workers and a recorded 64-bit master seed.
- The observed correlation and both implied matrices must be finite,
  symmetric, unit-diagonal, and positive definite. There is no ridge repair,
  eigenvalue clamping, or nearest-correlation fallback.

Unsupported data/model combinations fail before a claimed result. A point-fit
value, ordinary parameter-bootstrap interval, or failed transformation is
never substituted for exact-fit inference.

## Null transformation

Let `Z` be the `n x p` matrix of complete cases after column centering and
sample-standardization, so `Z'Z/(n-1) = S`. For model variant `v`, let
`Sigma_v` be its original model-implied indicator correlation matrix. The
transformed row matrix is

```text
Z_v* = Z S^(-1/2) Sigma_v^(1/2)
```

where both powers are the unique symmetric positive-definite matrix powers
from a self-adjoint eigendecomposition. Therefore
`Z_v*' Z_v*/(n-1) = Sigma_v` up to the frozen numerical tolerance. Failure of
the identity check is a typed run failure.

The transformed data stay mean zero. The PLS or PLSc estimator performs its
normal preprocessing on every sampled dataset; no original weights, scores,
paths, loadings, or implied matrices are reused in a replicate.

## Indexed resampling and model variants

Two independent, domain-separated runs are mandatory:

1. `saturated`, targeting the original saturated implied correlation matrix.
2. `estimated`, targeting the original estimated implied correlation matrix.

For each variant and replicate index `b`, a ChaCha20 stream derived from the
master seed, operation identity, variant, and `b` generates exactly `n` row
positions with replacement. Draws are never retried or replaced. Execution
order and worker count cannot change the indexed samples or serialized result.

Each sample is fully re-estimated and reassessed. The replicate records SRMR,
d_ULS, and d_G for the same variant. Estimation, assessment, or criterion
failure remains in the fixed ledger with the replicate index, sample-index
digest, typed reason code, and message. A replicate may be partially usable
when only one criterion is undefined; criterion summaries retain their exact
usable-index digest.

Cancellation is checked during transformation, resampling, estimation, and
assessment. Cancellation returns a terminal cancelled error and commits no
partial analytical result.

## Summary and decisions

For each variant and criterion:

- `original` is the corresponding v2 point discrepancy.
- `upper_95` and `upper_99` are Hyndman--Fan Type 7 quantiles of successful
  indexed replicate values.
- `not_rejected_95` is `original <= upper_95`.
- `not_rejected_99` is `original <= upper_99`.
- `exceed_or_equal_count` counts bootstrap values greater than or equal to the
  original value.
- `empirical_upper_tail_probability` is that count divided by the usable
  replicate count. It is an empirical tail proportion, not an analytical
  chi-square p-value.

At least `ceil(0.90 * requested)` usable values and at least two values are
required independently for each criterion. Otherwise that criterion is
unavailable and no bound or decision is emitted. A variant is `available`
only when all three criteria are available, `partial` when at least one is
available, and `unavailable` otherwise. The bundle applies the same rule to
the two variants.

Chi-square, degrees of freedom, and NFI stay point/approximate measures and do
not receive Bollen--Stine decisions.

## Persistence and integrity

The result records:

- `pls_model_fit_exact_v1`, `pls_model_fit_v2`, estimator, and indexed
  resampling method identities;
- transformation, matrix-power, quantile, decision, digest, retry, and
  minimum-usable policies;
- indicator order, analytical sample size, seed, requested draws, and both
  domain-separated operation identities;
- complete per-variant ledgers and per-criterion usable-index digests;
- the original discrepancies, bounds, tail proportions, and decisions.

Archive validation recomputes covered counts, deterministic sample and usable
index digests, transformed-matrix witnesses, bounds, tail proportions,
decisions, status aggregation, and identity linkage. Native results and every
export derive from the same persisted payload. Covered semantic tampering is
rejected rather than normalized; the archive member checksum remains the
integrity boundary for changes that cannot be scientifically recomputed
without reopening the source data.

## Qualification boundary

Implementation, deterministic kernel tests, the transparent NumPy primitive
reference, the bounded NumPy/SciPy full-refit oracle, and native
projection/export tests are only early evidence. The full-refit oracle now
independently re-estimates recursive PLS-PM Mode-A/Mode-B models inside each
small fixed indexed validation draw and verifies separate saturated and
estimated distributions, Type-7 bounds, ledger identity, metamorphic
properties, and typed failures. It does not implement PLSc or the complete
supported model-shape matrix, and its checked-in 12-draw work report is not a
qualification receipt. See
[PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md](PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md).

Promotion still requires the complete QualificationSpec V2 contract, kernel,
the second-reference-or-documented-exception oracle rule, full PLS/PLSc and
advanced-shape breadth, pre-registered generative/simulation calibration,
complete metamorphic/adversarial evidence, persistence/export,
packaged-Windows, performance, accessibility, and independent
scientific-review gates. Until then this method remains hidden from Standard
Calculate and cannot be described as full SmartPLS parity.
