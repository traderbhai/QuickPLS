# Indexed Resampling Engine Specification v3

Status: superseded by v4. Legacy v3 results remain readable and contain BCa but no studentized summary.

## Inherited Bootstrap Contract

V3 retains the fixed complete-case sampling, indexed ChaCha streams, sign alignment, canonical parameter identities, failed-replicate policy, percentile intervals, and normal-reference tests specified by v2. Bootstrap and delete-one jackknife callbacks have distinct progress phases. Their result ordering and compact summaries are invariant to worker scheduling and worker count.

## Bias-Corrected and Accelerated Intervals

For original estimate `theta`, successful bootstrap estimates `theta*_b`, and all `N` successful delete-one estimates `theta_(i)`:

- The bias proportion is `(count(theta*_b < theta) + 0.5 * count(theta*_b = theta)) / B`. This mid-rank tie rule is clamped to `[0.5/B, 1 - 0.5/B]` before `z0 = Phi^-1(proportion)` so finite bootstrap samples cannot create infinite corrections.
- With `theta_bar = mean(theta_(i))`, acceleration is `a = sum((theta_bar - theta_(i))^3) / (6 * sum((theta_bar - theta_(i))^2)^(3/2))`.
- A nominal tail probability `alpha` becomes `Phi(z0 + (z0 + z_alpha) / (1 - a * (z0 + z_alpha)))`.
- Bounds are two-sided Hyndman-Fan Type 7 quantiles of the successful bootstrap estimates at the adjusted tail probabilities.

This is the standard BCa construction introduced by Efron (1987), DOI: https://doi.org/10.1080/01621459.1987.10478410.

## Failure and Persistence Policy

- BCa uses the same complete-case sample and aligned parameter identities as the original and bootstrap estimates.
- Every delete-one model must estimate successfully. A failed jackknife case fails BCa inference instead of silently changing the acceleration sample.
- A parameter with zero delete-one variation or a numerically undefined adjusted-tail transformation has no valid BCa interval. Its four BCa numeric fields are null and it carries an explicit reason; percentile inference remains available.
- Projects persist only confidence level, jackknife case count, parameter identity, bias correction, acceleration, bounds, and any unavailable reason. Raw bootstrap and jackknife estimates remain transient.
- V3 archives require exactly one BCa row per percentile parameter and require the jackknife case count to equal the authoritative PLS estimation sample. V1/v2 archives remain readable but must not carry V3-only BCa data.

## Remaining Release Gates

BCa is experimental until Monte Carlo coverage, bias, tail behavior, non-normal data, small-sample behavior, full worker-count invariance, cancellation latency, and 10,000-replicate performance are qualified. Studentized intervals and permutation inference remain pending.
