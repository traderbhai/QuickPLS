# Regression bootstrap v1

## Scope and identity

QuickPLS `regression_bootstrap_v1` is an independently implemented,
fully offline case-resampling workflow for current typed
`regression_ols_v1` and `regression_logistic_v2` point estimates. It is not
PLS-PM resampling and never occupies the generic PLS bootstrap payload.

Current result provenance is exactly one of:

- `regression_ols_v1+regression_bootstrap_v1`
- `regression_logistic_v2+regression_bootstrap_v1`

The nested result records these frozen tokens:

- algorithm: `indexed_case_resampling_v1`
- stream: `quickpls_indexed_resampling_v1`
- interval policy: `percentile_primary_bca_conditional_v1`
- test reference: `standard_normal_bootstrap_ratio_v1`
- test tolerance: `64eps_max_1_original_replicates_v1`
- alternative: `two_sided`
- minimum usable fraction: `0.90`

Historical recipes and point-only results remain immutable. A schema-v1/v2
recipe or a legacy method version is readable but cannot be reinterpreted or
appended as current bootstrap evidence.

## Recipe contract and defaults

Schema-v3 `method_config.kind = regression` adds the optional strict object:

```json
{
  "bootstrap": {
    "algorithm": "case_resampling",
    "intervals": ["percentile", "bca"]
  }
}
```

The replicate count remains in `settings.bootstrap_samples`, the master seed
in `settings.seed`, the worker count in `settings.workers`, and the confidence
level in `settings.confidence_level`. The current contract requires:

- 99 to 10,000 replicates;
- fixed two-sided 95% confidence intervals;
- 1 to 64 workers, with index-addressed streams and ordered reduction so the
  scientific output is invariant to worker count;
- listwise-complete, unstandardized observed values and sampling with
  replacement;
- UI recommendation of 10,000 final subsamples and 1,000 exploratory
  subsamples. Focused automated evidence may use a smaller valid count.
- at most 50 predictors and controls plus the intercept. This v1 bound keeps
  the exact validation witness and project archive size finite at 10,000
  replicates and is a documented QuickPLS difference.

Studentized intervals, custom alpha/tails, permutation inference, case
weights, categorical auto-encoding, multinomial/ordinal logistic regression,
and PROCESS bootstrapping are excluded from v1.

## Arithmetic

For usable coefficient replicates
`b*(1), ..., b*(B)` and point estimate `b`, QuickPLS reports:

```text
bootstrap_mean = sum(b*(r)) / B
bias = bootstrap_mean - b
bootstrap_se = sqrt(sum((b*(r) - bootstrap_mean)^2) / (B - 1))
```

Percentile limits use sorted Type-7 quantiles at 0.025 and 0.975. Logistic
odds-ratio limits are computed on the sorted `exp(b*)` distribution; they are
not fabricated from missing coefficient output.

The reported bootstrap-ratio statistic is:

```text
z_boot = b / bootstrap_se
p_two_sided = 2 * Phi(-abs(z_boot))
```

This standard-normal reference is QuickPLS's frozen bounded choice for both
OLS and logistic regression and is separate from the point-estimate HC3 t or
logistic Wald test. The UI must not infer significance solely from an interval.

The ratio is unavailable, never `NaN` or `N/A`, when:

```text
bootstrap_se <= 64 * machine_epsilon
                     * max(1, abs(b), max(abs(b*(r))))
```

The row then records `degenerate_bootstrap_standard_error` with a nonempty
message.

BCa uses the midrank bias correction and delete-one acceleration documented by
the engine's pure summary function. All original complete cases must produce a
usable delete-one fit. Otherwise every affected coefficient and odds-ratio BCa
row is explicitly unavailable with `incomplete_jackknife`. Samples with fewer
than three delete-one estimates use `insufficient_jackknife_estimates`; a zero
or undefined acceleration uses `degenerate_jackknife_acceleration`. Percentile
inference remains available when BCa is unavailable.

A run completes only when at least `ceil(0.90 * requested_replicates)`
bootstrap fits are usable. Each failed replicate retains its index, stable
reason code, and message. The result also records `jackknife_cases` and
`usable_jackknife_cases`, so archive validation can enforce that an Available
BCa interval used the complete delete-one set. Cancellation aborts the run and
no partial result is committed.

The nested `regression_bootstrap_validation_witness_v1` is internal validation
data and is not rendered or exported. It retains exact term order, every usable
bootstrap replicate index and coefficient vector, and every successful or
failed delete-one index. Bootstrap successes plus the public failed-replicate
records must be the exact complement `0..B-1`; delete-one successes and
failures must be the exact complement `0..n-1`. Archive validation reruns the
same pure summary function over this bounded witness and rejects any changed
percentile, BCa, odds-ratio, standard-error, ratio-test, count, or identity
field. Because JSON decimal serialization can shift a recomputed floating-point
summary by a few ULPs, archive validation compares finite numeric summary fields
within `64 * machine_epsilon * max(1, abs(stored), abs(recomputed))`; term order,
counts, indices, tagged statuses, reason codes, messages, and optional-field
shapes remain exact.

## Reproducibility and known differences

QuickPLS derives a ChaCha20 stream from the fixed master seed, operation token,
and replicate index. Replicate results are reduced in index order. This makes
the output reproducible and worker-invariant; it is an independently
implemented behavior and does not claim SmartPLS random-number identity or
bitwise numerical identity.

The comparison scope was frozen from the official SmartPLS documentation:

- <https://smartpls.com/documentation/algorithms-and-techniques/regression-bootstrapping/>
- <https://smartpls.com/documentation/algorithms-and-techniques/>

QuickPLS differs by requiring an explicit fixed seed, by exposing failed-fit
and BCa-unavailability reasons, and by using the documented
`standard_normal_bootstrap_ratio_v1` reference for both regression families.

## Stable focused evidence

The promotion audit binds the failure-boundary contract to these named Rust
tests in `crates/qpls-resampling/src/lib.rs`:

- `regression_bootstrap_failure_boundary_listwise_complete_cases_are_the_only_sampling_frame`
- `regression_bootstrap_failure_boundary_captures_zero_based_single_class_replicates`
- `regression_bootstrap_failure_boundary_rejects_below_ninety_percent_usable`
- `regression_bootstrap_failure_boundary_real_delete_one_failure_disables_all_bca`

Strict nested-payload parsing plus append, save, checksum-updated-load, reopen,
and tamper rejection are bound to
`regression_bootstrap_append_save_reopen_and_tamper_contract_are_atomic` in
`crates/qpls-project/src/lib.rs`.
