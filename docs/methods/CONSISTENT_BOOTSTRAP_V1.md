# Consistent Bootstrap v1

Status: contract-only and `absent` in the QuickPLS 3 method-promotion factory. The current validator deliberately rejects every PLSc recipe with bootstrap, studentized-bootstrap, or permutation settings. This document does not claim implementation or validation.

`plsc_bootstrap_v1` is the planned nonparametric case bootstrap for the bounded `plsc_v2` estimator. It depends on a release-qualified PLSc point estimator and the deterministic scheduling, canonical identity, cancellation, and aggregation rules of `indexed_resampling_v4`. Ordinary PLS bootstrap evidence cannot qualify this method: every bootstrap and jackknife sample must re-estimate the complete PLSc pipeline, including Mode A weights, rho_A, attenuation correction, corrected construct correlations, paths, loadings, and R-squared.

## Bounded v1 scope

- The model must satisfy the exact reflective, two-or-more-indicator, path-or-factor weighting, complete-case scope of `plsc_v2`.
- Generated interactions, higher-order constructs, PCA weighting, case weights, multigroup inference, permutation inference, and covariance/correlation-only input are excluded.
- A release run requests 1,000 through 10,000 indexed case-resampling replicates and a fixed seed. Each replicate contains the same number of cases as the frozen complete-case sample and samples with replacement.
- V1 reports original PLSc estimates, bootstrap means and bias, standard errors, two-sided normal-reference diagnostics, percentile intervals, and BCa intervals at the frozen confidence level. Studentized intervals are excluded until separately qualified for PLSc.
- Primary replicate `(b)` uses a domain-separated stream derived from `(master_seed, "plsc_bootstrap_v1", b)`. Replicate ordering and analytical output are independent of scheduling and worker count.

For canonical parameter `j`, successful replicate estimates are `theta*_bj = PLSc_v2(D*_b)_j`. The bootstrap standard error is the sample standard deviation of successful `theta*_bj` values. Percentile intervals use the frozen Type 7 quantile rule. BCa uses the same mid-rank bias correction and delete-one acceleration rules documented in `RESAMPLING_ENGINE_V3.md`, but every delete-one fit is a full PLSc v2 fit.

## Orientation and failure policy

Each successful resample is aligned to the original solution using the frozen indicator/construct orientation rule before parameter aggregation. Parameter identities must exactly match the original reflective model.

A materially improper rho_A, inadmissible attenuation-corrected correlation, nonconvergence, singular structural equation, identity mismatch, or nonfinite estimate fails that replicate. Failed replicates are never silently replaced or assigned a different stream. Requested, attempted, successful, and failed counts and stable reason counts are persisted. Publication-facing inference requires the preregistered minimum successful fraction; the method audit must freeze and justify that threshold before any evidence is accepted. A failed delete-one PLSc fit makes BCa unavailable rather than changing its acceleration sample.

## Persistence and product contract

The target result envelope is `pls_pm_v3` with a typed `plsc_bootstrap_v1` artifact. The archive stores no raw resampled datasets or fitted models. It stores the plan, stream domain, counts, canonical parameter identities, original estimates, compact summaries, intervals, unavailable reasons, and exact provenance. Legacy ordinary-PLS bootstrap output must never be reinterpreted as consistent bootstrap evidence.

The native setup must make the PLSc dependency and unsupported shapes visible before execution. Results and exports must distinguish ordinary from attenuation-corrected parameters, show failed-replicate accounting, and retain scope warnings. GUI and CLI must generate the same replicate plan and analytical payload for the same seed, recipe, and dataset fingerprint. Packaged acceptance must cover cancellation and retry, save/reopen, all exports, offline execution, and process cleanup.

## Qualification work still required

Release qualification requires the source and evidence chain frozen by `validation/methods/consistent_bootstrap_v1.manifest.json`: an independent implementation, published and hand fixtures, bias/coverage/failure simulations, boundary and determinism tests, strict persistence, native results and exports, a current method audit, and installed Windows acceptance.

## Scientific sources

- Dijkstra and Henseler (2015), *Consistent and Asymptotically Normal PLS Estimators for Linear Structural Equations*, https://doi.org/10.1016/j.csda.2014.07.008.
- Dijkstra and Henseler (2015), *Consistent Partial Least Squares Path Modeling*, https://doi.org/10.25300/MISQ/2015/39.2.02.
- Efron and Tibshirani (1993), *An Introduction to the Bootstrap*.
