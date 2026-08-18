# PLS post-hoc technical minimum sample size v1

Status: historical point-estimate-only contract retained for archive and export
compatibility. New runner results use the inference-aware v2 contract in
`PLS_POSTHOC_TECHNICAL_MINIMUM_SAMPLE_SIZE_V2.md`.
This deterministic result belongs to a completed PLS algorithm run. It is separate from the
prospective Monte Carlo workflow identified by
`qpls3.pls.sample_size_power`; neither cell may substitute for the other.

## Result contract

For the selected driver coefficient `beta_min`, v1 reports:

`n_technical = ceil((2.486 / abs(beta_min))^2)`

The fixed constant represents a directional test with `alpha = 0.05` and
power `= 0.80`. No customer setting changes those assumptions. The result is
retrospective: it summarizes an estimated PLS model after calculation. It is
not prospective power analysis, an observed-power calculation, evidence of
sample representativeness, or a guarantee that the model, measurement design,
subgroups, or future estimates are adequately powered.

The eligible drivers are finite estimated structural path coefficients from
the same PLS result. The driver is the smallest absolute coefficient. Ties are
resolved deterministically by source stable ID and then target stable ID, both
in ascending binary string order. The result stores the signed driver and its
source/target identity so a saved report can explain the calculation.

If there is no structural path, the result is not applicable. If the smallest
absolute coefficient is exactly zero, the formula has no finite result; v1
reports a typed `undefined_zero_path` state and no required sample size. It
must not replace zero with an epsilon, return zero, or silently clamp to the
largest integer. A finite mathematical result outside the supported integer
range likewise returns a typed range failure rather than a fabricated value.

## Official-wording nuance

The [SmartPLS technical sample-size page](https://smartpls.com/documentation/algorithms-and-techniques/core-algorithm/pls-power/)
describes using the smallest absolute **statistically significant** path
coefficient. A point-estimate-only PLS run does not establish significance.
This v1 contract therefore freezes the smallest absolute *estimated* path as
its deterministic driver and records that difference explicitly. It must not
be called full SmartPLS parity unless a separately qualified inference-aware
selection rule resolves that difference, or an independently reviewed parity
decision accepts and documents it.

## Qualification obligations

Before full coverage or further evidence advancement, qualification must cover the
published formula examples (`0.10 -> 619`, `0.15 -> 275`, `0.20 -> 155`,
`0.25 -> 99`, `0.30 -> 69`, and `0.40 -> 39`), sign invariance, deterministic
ties, path declaration reordering, zero/no-path behavior, persistence,
cross-format export, an independent calculation oracle, and the documented
inference-aware driver rule. The current registry cell therefore remains
`coverage_state=partial`, `evidence_state=engine_only`, and `surface=labs`; it
cannot appear as a Standard parity result.
