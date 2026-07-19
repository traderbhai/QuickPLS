# Indexed Resampling Engine Specification v1

Status: superseded by v2. Legacy v1 results remain readable.

## Bootstrap Sampling

For a PLS model with `N` complete cases across all model indicators, each nonparametric bootstrap replicate draws exactly `N` positions independently and uniformly with replacement from that fixed analysis sample. Rows excluded by listwise deletion in the original estimate cannot re-enter a replicate. This follows the empirical-distribution bootstrap introduced by Efron (1979): https://doi.org/10.1214/aos/1176344552.

## Reproducibility Contract

- Every random stream is derived independently from `(master_seed, operation_identifier, replicate_index)` using SHA-256 domain separation and ChaCha20.
- Replicate `i` therefore receives the same sample regardless of scheduling, requested worker count, or failures in other replicates.
- Worker count is execution provenance, not part of the analytical bootstrap plan or result identity.
- Results are aggregated strictly in replicate-index order. Progress notifications are serialized and monotonically increasing, but notification timing is not part of the numerical contract.
- A cancellation request prevents a completed `BootstrapRun` from being returned. Partial results are not publication artifacts.
- Each estimator failure is retained with its replicate index and message. PLS inference requires at least two successful replicates and at least 90% of requested replicates; otherwise the run fails without a publication result.

## Persisted PLS Inference

- Successful replicate estimates are transient. The project stores the plan, usable count, failed-replicate diagnostics, and compact parameter summaries.
- The bootstrap mean is the arithmetic mean of successful replicate estimates. Bias is `bootstrap_mean - original`, and standard error is the sample standard deviation with denominator `B - 1`.
- Two-sided percentile bounds use Hyndman-Fan Type 7 quantiles at `(1 - confidence_level) / 2` and `1 - (1 - confidence_level) / 2`.
- The persisted parameter set covers outer loadings, outer weights, structural paths, direct/indirect/total effects, and R-squared values.
- Parameter identities used delimiter-concatenated strings. Because identifier punctuation could collide, new results use v2; v1 payloads are retained only for backward compatibility.
- Studentized and BCa intervals, jackknife, permutation, coverage simulations, and 10,000-replicate memory/performance qualification remain release gates for v0.4.

## PLS Sign Alignment Dependency

PLS-specific integration must align each replicate's construct orientation to the original-sample solution before aggregating loadings, weights, paths, or effects. The resampling foundation intentionally does not infer statistical sign semantics from generic values.
