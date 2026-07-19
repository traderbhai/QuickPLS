# Nested Studentized Bootstrap Specification v1

Status: implemented experimental inference. `nested_studentized_v1` is not validated or publication-ready.

## Scope and Configuration

This method adds two-sided nonparametric bootstrap-t confidence intervals to the existing fixed complete-case PLS bootstrap. It applies to the same canonical loading, weight, path, effect, and R-squared parameter identities as percentile and BCa inference. It does not define a null-resampling test, replace the existing normal-reference statistic, or make experimental inference publication-ready.

Studentized inference is independently enabled. It requires at least 999 requested outer replicates. The number of inner replicates per successful outer model is an odd integer with minimum `99`, default `199`, and maximum `999`. Zero disables studentized inference. Confidence level, master seed, complete-case sample, preprocessing, estimator settings, convergence rules, and outer bootstrap plan are inherited from `indexed_resampling_v4`.

## Nested Bootstrap-T Definition

Let `theta_hat` be an original-sample estimate. For outer replicate `b = 0,...,B-1`, let `theta*_b` be the estimate from the existing size-`N` empirical bootstrap draw. Conditional on that outer sample, draw `M` size-`N` inner samples with replacement and obtain `theta**_(b,m)` for `m = 0,...,M-1`.

For each parameter and usable outer replicate:

1. Compute `se*_b` as the sample standard deviation of its usable inner estimates, with denominator `M_b - 1`.
2. Compute the pivot `t*_b = (theta*_b - theta_hat) / se*_b`.
3. Compute `se_hat` as the sample standard deviation, denominator `B_theta - 1`, of the usable outer estimates already used by ordinary bootstrap inference.
4. Let `q_lo` and `q_hi` be the lower- and upper-tail quantiles of usable `t*_b` values. For confidence `1-alpha`, the interval is `[theta_hat - q_hi*se_hat, theta_hat - q_lo*se_hat]`, where the tail probabilities are `alpha/2` and `1-alpha/2`.

Both pivot quantiles use Hyndman-Fan Type 7 interpolation, matching QuickPLS percentile and BCa summaries. Bounds are reported on the estimator's native scale, are not clipped to theoretical parameter ranges, and must be finite and ordered. No transformation-based interval is part of v1.

This is the conventional nested bootstrap-t construction described by Efron and Tibshirani and by Davison and Hinkley. Hall's comparison supplies the higher-order accuracy rationale under the method's regularity conditions; it is not a guarantee for every PLS estimand.

## Deterministic Sampling and Sign Alignment

- Outer samples are exactly the existing indexed bootstrap samples; enabling studentization must not change percentile or BCa estimates.
- Inner sample `(b,m)` uses a separately domain-separated ChaCha20 stream derived from the canonical tuple `(master_seed, "pls_pm_studentized_inner_v1", b, m)`. Indices are zero-based. Parameter identity and worker count are not stream inputs because one fitted inner model supplies all parameters.
- SHA-256 domain separation, integer encoding, uniform with-replacement draw rules, and strict index-order aggregation follow the existing resampling engine. Scheduling cannot alter draws or serialized analytical output.
- Each outer solution is aligned to original construct scores at its sampled positions under the existing sign rule. Each inner solution is then aligned to its immediate outer parent's scores at the inner sampled positions. Canonical parameter identities remain those of the original model.
- Failed estimates are never retried, replaced, or assigned a new stream. Cancellation checks occur before every outer and inner fit and at estimator checkpoints; cancellation discards the whole in-progress studentized artifact.

## Failure and Degeneracy Policy

An outer model failure retains the existing bootstrap failure semantics. For a successful outer model, an inner fit failure is recorded once and is unusable for every affected parameter. A finite parameter estimate may still be unusable independently of other parameters.

For parameter `theta` in outer replicate `b`, `se*_b` requires at least `ceil(0.90*M)` finite aligned inner estimates and at least two values. Otherwise that outer pivot is unavailable. The final parameter interval requires at least `ceil(0.90*B_requested)` finite pivots and at least two pivots. These denominators always refer to requested counts; QuickPLS never relaxes them to the number that happened to converge.

For a value set `x` centered at `c`, define `tol = 64*f64::EPSILON*max(1, abs(c), max(abs(x)))`. A nonfinite sample standard error or standard error `<= tol` is numerically zero. A zero inner standard error makes only that outer pivot unavailable. A zero original `se_hat`, a nonfinite pivot or quantile, insufficient pivots, or unordered/nonfinite bounds makes that parameter's studentized interval unavailable. The row remains present with null numerical fields and a stable reason code; percentile and BCa output remain available. Infrastructure or schema failure produces a failed studentized artifact with no parameter rows and an explicit `nested_infrastructure_failure` summary; primary, percentile, and BCa results remain available.

Persisted parameter reason codes are `insufficient_pivots`, `zero_outer_standard_error`, and `invalid_bounds`. Fewer than 90% usable inner fits makes that outer replicate unavailable for bootstrap-t pivots without removing its valid primary estimate from percentile or BCa inference.

## Persistence Contract

Projects persist a compact `nested_studentized_v1` artifact containing method version, confidence level, requested inner count, minimum-usable policy, and stream-domain identifier. A successful artifact has `failure = null` and one row for every ordinary-bootstrap parameter. Requested and usable outer counts remain authoritative in the enclosing bootstrap artifact. Each successful-artifact row stores canonical identity, original estimate, outer standard error, the maximum absolute outer-estimate scale used by the numerical-zero rule, pivot count, lower/upper pivot quantiles, lower/upper bounds, and an optional stable unavailable reason.

Raw inner samples, fitted models, scores, estimates, and pivot vectors are transient and must not enter `.qpls`. For the successful form, archive validation requires exactly one studentized row per percentile parameter, finite ordered numeric fields when available, null bounds plus a recognized reason when unavailable, and settings consistent with authoritative recipe provenance. For the failed form, validation requires the stable `nested_infrastructure_failure` summary, a positive bounded failed-primary count, an in-range first failed-primary index, a nonempty message, and zero parameter rows. Older result envelopes remain readable and cannot contain this artifact.

## Computational Warning

Nested studentization performs up to `B*M` additional full PLS fits. The minimum supported plan (`B=999`, `M=99`) adds 98,901 inner fits; at `B=999`, the default inner plan adds 198,801. Desktop and CLI must show and persist an experimental high-cost warning with these counts before execution. Progress reports outer and inner phases, memory remains bounded through ordered streaming aggregation, and no UI should imply that the default is inexpensive. Maximum settings are allowed only after benchmark qualification.

## Validation and Promotion Gates

Implementation remains experimental until all of the following pass:

- Hand fixtures verify inner sample standard errors, pivots, Type 7 quantiles, reversed-tail interval bounds, tolerance boundaries, and every reason code.
- Supplied-index fixtures agree with two independent implementations, including a development-only comparison with R `boot::boot.ci(type="stud")` where estimands/settings are equivalent. GPL validation tools are not distributed.
- Fixed-seed analytical output is exactly equal for 1, 2, and maximum supported workers, and repeated runs preserve outer percentile/BCa output byte-for-byte.
- Metamorphic tests cover row and indicator reordering, construct reordering, admissible scale changes, canonical identities containing delimiters, sign-reversed latent solutions, and native-bound diagnostics.
- Failure tests cover constant, singular, nonconvergent, nonfinite, insufficient-inner, zero-inner-SE, zero-outer-SE, cancellation, and corrupt persisted artifacts without redraws or denominator drift.
- At least 1,000 completed simulations per preregistered scenario assess nominal 95% coverage and null exclusion. Promotion requires coverage in `[0.925,0.975]`, type-I error in `[0.025,0.075]`, at least 99% interval availability, and no unexplained bias or tail asymmetry. Scenarios include normal and non-normal errors, small samples, multi-indicator reflective/formative models, multiple predictors, indirect effects, and R-squared.
- Sensitivity runs compare inner counts 99, 199, and 999 and outer counts at least 999 and 9,999; the default is retained only if interval behavior is stable within preregistered Monte Carlo uncertainty.
- A documented 8-core Windows benchmark records runtime, peak memory, cancellation latency, worker scaling, unavailable rates, and exact numerical drift for minimum, default, and maximum plans. Cancellation latency must remain at most one second at the 95th percentile, there must be no out-of-memory failure under the published hardware profile, and worker-count numerical drift must be zero.

No validation or accuracy claim follows from this specification alone.

## Method References

- Efron, B. (1979), "Bootstrap Methods: Another Look at the Jackknife," *The Annals of Statistics*, https://doi.org/10.1214/aos/1176344552.
- Hall, P. (1988), "Theoretical Comparison of Bootstrap Confidence Intervals," *The Annals of Statistics*, https://doi.org/10.1214/aos/1176350933.
- Efron, B. and Tibshirani, R. J. (1993), *An Introduction to the Bootstrap*, https://doi.org/10.1007/978-1-4899-4541-9.
- Davison, A. C. and Hinkley, D. V. (1997), *Bootstrap Methods and Their Application*, https://doi.org/10.1017/CBO9780511802843.
- Hyndman, R. J. and Fan, Y. (1996), "Sample Quantiles in Statistical Packages," *The American Statistician*, https://doi.org/10.1080/00031305.1996.10473566.
