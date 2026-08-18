# PLS-SEM Sample Size and Power v2

Status: Supported bounded scope. New prospective calculations use
`pls_sample_size_power_v2`. Historical v1 results remain readable under their
original identity, but v1 recipes cannot be executed or relabeled as v2.

This workflow estimates prospective Monte Carlo power for one deliberately
narrow design: exactly two ordinary reflective constructs joined by one
predictor-to-outcome path. It is not retrospective observed power, the
ten-times rule, an inverse-square-root heuristic, or a general guarantee that a
future sample is adequate.

## Supported design

- Two reflective constructs and one directed path, with 3-10 indicators per
  block.
- One declared population path in `[-0.80, 0.80]` and one ordered loading per
  indicator in `[0.50, 0.95]`.
- Standard-normal exogenous latent, structural disturbance, and independent
  indicator errors; generated data contain no missing values.
- Path weighting, standardized preprocessing, a bounded convergence tolerance,
  and 100-10,000 maximum PLS iterations.
- A strictly increasing 2-16 point sample-size grid from 30 through 5,000.
- 100-10,000 Monte Carlo replicates and an odd 99-1,999 indexed case-bootstrap
  replicates per generated dataset.
- Explicit alpha, target power, Wilson confidence level, master seed, and worker
  count.

Formative blocks, controls, multiple paths, higher-order constructs,
interactions, multigroup models, nonlinear or mixture designs, alternate
distributions, correlated errors, weights, clustering, ordinal data, and every
missing-data design are not available in v2.

## Inference and power decision

Each indexed Monte Carlo dataset is fitted with the production PLS-PM kernel.
The target path then receives a full indexed case bootstrap. For original path
estimate `b` and usable bootstrap estimate `b*`, v2 counts the null-centered
two-sided tail event

`abs(b* - b) >= abs(b)`.

With `E` exceedances and `U` usable bootstrap fits, the exact test probability
is

`p = (E + 1) / (U + 1)`.

At least 90% of the requested inner bootstrap fits must be usable. Failed fits
are retained in typed accounting; they are never retried or silently replaced.
An outer Monte Carlo replicate is rejected when its exact two-sided probability
is no greater than the declared alpha. Failed outer replicates remain in the
planned power denominator.

For `R` planned outer replicates,

`power_hat(n) = sum_r I(p_r <= alpha) / R`.

Each grid row reports a two-sided Wilson interval. The conservative result is
the first evaluated sample size whose Wilson lower bound reaches the target
power. Rows are not smoothed, interpolated, or extrapolated. If no row qualifies,
the result says that target power was not reached on the evaluated grid.

## Why v2 has a new identity

The v1 normal-reference ratio test failed its frozen null calibration: 190 of
2,500 null datasets rejected, or 7.60%, with a 95% Wilson interval of 6.625% to
8.705%. Changing that inference under the v1 identity would reinterpret saved
scientific evidence.

V2 therefore has new recipe, result, stream, and inference identities. Its
independent compact calibration uses a separate NumPy generator and Mode-A PLS
implementation and consumes no QuickPLS binary or Rust result. On the frozen
profile it produced:

- null `n=160`: 20/300 rejections (6.67%);
- null `n=320`: 15/300 rejections (5.00%);
- pooled null: 35/600 (5.83%), 95% Wilson interval 4.224% to 8.005%;
- signal `beta=0.45`, `n=60`: 75/100 rejections (75.00%);
- every outer run completed; one of 69,300 inner bootstrap fits failed and was
  retained in the exact accounting.

The compact gate requires the pooled null interval to contain 5%, pooled null
rejection at most 6.5%, each frozen null point at most 8%, signal recovery at
least 60%, no failed outer run, and an inner fit-failure rate at most 0.1%.

## Determinism, persistence, results, and export

The stream plan is keyed by method version, scenario identity, seed, sample
size, replicate index, and subdomain. Scheduling order and supported worker
count do not change the ordered scientific payload. Cancellation appends no
partial result; retrying the same recipe reproduces the uninterrupted payload.

The typed v2 result stores the frozen identity and digests, workload, ordered
outer-replicate ledger, target estimate, exact p value and decision, requested /
usable / failed inner-bootstrap counts, two-sided exceedance count, grid rows,
Wilson intervals, conservative decision, warnings, and exclusions. Project
validation recomputes all deterministic summaries and the plus-one probability,
and rejects identity, recipe, fingerprint, ledger, count, probability, decision,
digest, malformed-payload, and archive-checksum mutations atomically.

Native results and XLSX export contain:

- power by sample size;
- aggregate bootstrap tail accounting by sample size;
- named outer simulation failures;
- the complete declared design; and
- method, stream, inference, digest, and decision provenance.

Archive validation is structural and arithmetic validation of the stored typed
evidence. It is not cryptographic authentication and does not regenerate every
synthetic dataset or refit every bootstrap sample during reopen. Scientific
validity is provided separately by the engine tests and independent calibration.

## Scientific source

Muthen and Muthen (2002), *How to Use a Monte Carlo Study to Decide on Sample
Size and Determine Power*, https://doi.org/10.1207/S15328007SEM0904_8.

Independent v2 calibration:
`validation/pls_sample_size_power_v2_reference.py` and
`validation/results/pls_sample_size_power_v2_reference_report.json`.

## Focused release refresh

The compact scientific calibration is reused while its exact source remains
unchanged. Refresh only the three source tiers with:

`python validation/pls_sample_size_power_v2_factory.py`

After the one shared release desktop/CLI build is frozen, run the method-scoped
packaged gate once:

`python validation/pls_sample_size_power_v2_packaged_acceptance.py`

That adapter runs only the prospective-power v2 supervisor and final method
audit. It checks invalid setup, same-plan cancellation/retry, exact outer and
inner tail accounting, selected-run XLSX, immutable save/reopen, three actual
Tauri window sizes, functional offline behavior, sampled process-tree network
disclosure, and graceful process cleanup. The release-blocking offline condition
requires zero external QuickPLS application/page requests and no remote service
dependency for the analytical workflow. Microsoft-managed WebView2 background
connections are recorded separately as `platform_background_egress_observed`;
when present, `commercial_zero_egress_passed` remains `false` without failing
this bounded method qualification. The separate OS-enforced commercial
zero-process-egress gate remains pending. Historical v1 evidence is neither
executed nor relabeled.
