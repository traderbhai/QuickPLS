# PLSc Consistent Bootstrap v1 Qualification Scaffold

## Status and non-claim

This document preregisters QualificationSpec V2 for capability cell `qpls3.inference.consistent_bootstrap` and method `plsc_bootstrap_v1`. It does not qualify or promote the method.

Current state is deliberately fail-closed:

- coverage: `partial`
- evidence: `absent`
- surface: `labs`, but not selectable while evidence is absent
- QualificationSpec migration: `compatibility_only`
- admitted immutable receipts: zero
- promotion decision: blocked

The Python and R files in this scaffold are arithmetic microreferences. They do not implement the PLSc estimator, do not run a product-sized bootstrap, and cannot satisfy kernel, independent-oracle, simulation, archive, native, export, packaged, performance, or review evidence roles.

## Frozen scientific contract

### Supported setup

The bounded v1 implementation accepts raw row-level numeric observations and the exact supported `plsc_v2` reflective scope:

- at least two uniquely bound indicators per construct;
- path or factor weighting under the existing `plsc_v2` contract;
- recursive supported structural models;
- model-wide listwise deletion, with no missing-to-zero conversion;
- no formative blocks, generated interactions, higher-order constructs, PCA weighting, case weights, groups, permutation inference, studentized inference, or matrix-only input;
- 1,000 through 10,000 primary bootstrap replicates;
- finite confidence level strictly between zero and one;
- fixed master seed and a supported worker count.

The exact option remains partial because selectable test direction, interval-family selection, complete measurement-assessment inference, broader PLSc model shapes, and confirmed official defaults are not complete.

The stored consistent-bootstrap payload can carry an optional separately versioned `pls_model_fit_exact_v1` bundle when that independent recipe selector is enabled. That bundle is not an estimand or evidence source for this capability cell. Its recipe selector, provenance marker, payload, and separate qualification must either all agree or all be absent; evidence may not be cross-credited between the two capabilities.

### Sampling and refitting

Let `D_cc` be the frozen model-wide complete-case frame with `n` rows. Primary replicate `b` draws `n` row indices with replacement from a domain-separated stream identified by:

```text
(master_seed, "plsc_consistent_bootstrap_v1", b)
```

Replicates are indexed before scheduling. A failed fit retains its original index and is not retried, replaced, clamped, or assigned another stream.

Every accepted primary and delete-one sample performs a full `plsc_v2` refit. Ordinary PLS estimates may not be reused. Each refit includes weights, rho_A correction, corrected construct correlations, corrected loadings, corrected paths, direct/indirect/total effects, and corrected R-squared. Construct signs are aligned to the original solution before extraction.

Canonical parameter identities are JSON strings of `(kind, parts)` and cover:

- `plsc_rho_a`
- `plsc_construct_correlation`
- `plsc_outer_loading`
- `plsc_outer_weight`
- `plsc_path`
- `plsc_direct_effect`
- `plsc_indirect_effect`
- `plsc_total_effect`
- `plsc_r_squared`

Every successful refit must contain exactly the original canonical identity set and only finite values.

### Failure and accounting semantics

The required minimum usable count is:

```text
max(2, ceil(requested_replicates * 0.90))
```

The `0.90` policy remains an unresolved scientific-review item; its presence in source is not evidence that the threshold is justified.

Allowed primary failure reasons are:

- `cancelled`
- `inadmissible_rho_a`
- `inadmissible_corrected_correlation`
- `plsc_nonconvergence`
- `singular_plsc_equation`
- `nonfinite_plsc_parameter`
- `parameter_identity_mismatch`
- `plsc_refit_failed`

Requested, usable, and failed counts must close exactly. The primary ledger must contain one row for every zero-based replicate index in exact order. A success has a sample-index digest and parameter-value digest, and no failure fields. A failure has the same sample-index digest as its failure record, one allowed reason, a nonempty diagnostic, no parameter digest, and no contribution to any sampling distribution.

A failed required delete-one PLSc fit makes every BCa interval unavailable while retaining percentile and normal-reference output. Numerical degeneracy may make an individual BCa interval unavailable with an explicit reason. Neither condition permits silently switching interval methods.

### Summaries and intervals

For each canonical parameter, successful primary refits determine:

- bootstrap mean;
- bias, defined as bootstrap mean minus original estimate;
- sample standard error with denominator `B_usable - 1`;
- two-sided standard-normal reference statistic and probability when standard error is greater than machine epsilon;
- Type-7 percentile endpoints at tail probabilities `(1-confidence)/2` and `1-(1-confidence)/2`;
- conditional BCa endpoints.

BCa uses:

1. Midrank bias probability `(below + 0.5*tied)/B_usable`, clamped to `[0.5/B, 1-0.5/B]`.
2. Bias correction `z0 = Phi^-1(probability)`.
3. Full delete-one `plsc_v2` acceleration `sum(delta^3) / (6 * sum(delta^2)^(3/2))`, where `delta` is the jackknife mean minus a delete-one estimate.
4. Standard BCa adjusted tail probabilities followed by Type-7 quantiles.

No normal-reference result is available for a nonfinite original value, nonfinite standard error, or standard error at or below machine epsilon.

### Digest contract

Sample-index SHA-256 input is:

```text
"QuickPLS PLSc consistent bootstrap sample v1\0"
|| count_u64_little_endian
|| each_sample_index_u64_little_endian
```

Parameter-value SHA-256 input is:

```text
"QuickPLS PLSc consistent bootstrap parameters v1\0"
|| parameter_count_u64_little_endian
|| for each key in ascending UTF-8 order:
     key_length_u64_little_endian || key_utf8
     || value_text_length_u64_little_endian || value_text_utf8
```

`value_text` is Rust scientific notation with 12 digits after the decimal point, or 13 significant decimal digits. This encoding is the archive-stable witness boundary; it is not permission to round analytical results.

## Transparent microreferences

The frozen fixture is `validation/fixtures/consistent_bootstrap_v1_microcases.json`. Its principal case contains nine usable values `1..9`, original value `5`, one retained failure, and delete-one values `4, 4.5, 5, 5.5, 6`.

Hand-checkable expected output is:

| Quantity | Expected |
|---|---:|
| requested / usable / failed | 10 / 9 / 1 |
| required usable under the arithmetic rule | 9 |
| mean / bias | 5 / 0 |
| sample standard error | 2.7386127875258306 |
| normal-reference statistic | 1.8257418583505538 |
| two-sided probability | 0.06788915486182903 |
| Type-7 95% percentile interval | [1.2, 8.8] |
| BCa bias correction / acceleration | 0 / 0 |
| BCa interval | [1.2, 8.8] |

The second case has constant primary and delete-one values. Its standard error is zero, its normal-reference diagnostic is unavailable, its percentile endpoints both equal two, and BCa is unavailable because acceleration is degenerate.

`validation/consistent_bootstrap_v1_reference.py` independently implements the frozen digest and post-refit arithmetic with the Python standard library. `validation/consistent_bootstrap_v1_reference.R` independently checks the numerical arithmetic using base R. The R script is present but was not executed when this scaffold was created because `Rscript` was unavailable in the environment. Neither script implements PLSc, so neither is a full computational oracle.

The V2 contract separately preregisters two required future full-refit oracle slots at `validation/oracles/consistent_bootstrap_v1_full_plsc_reference.py` and `validation/oracles/consistent_bootstrap_v1_full_plsc_reference.R`. Those files do not exist yet. Their declarations describe the required evidence target; they do not claim that an implementation or result is present. Immutable oracle receipts cannot be admitted until both implementations exist, are independently maintained, execute the same indexed cases, and pass the frozen comparisons.

The 10-replicate fixture is intentionally below the executable product minimum. It is a contract microcase only and must never be admitted as a product kernel receipt.

## Boundary and failure inventory

The fixture freezes these required classes:

| Class | Required case IDs |
|---|---|
| Settings | `replicate_count_999`, `replicate_count_10001`, `studentized_requested`, `permutation_requested` |
| Input and point estimate | `raw_rows_required`, `ordinary_pls_point_rejected` |
| Unsupported model extensions | `formative_construct_rejected`, `single_indicator_construct_rejected`, `generated_interaction_rejected`, `higher_order_construct_rejected`, `group_inference_rejected`, `case_weights_rejected`, `pca_weighting_rejected` |
| Primary and jackknife failures | `insufficient_usable_replicates`, `delete_one_refit_failure`, `nonfinite_parameter`, `unknown_failure_reason` |
| Ledger and digest tampering | `duplicate_ledger_index`, `missing_ledger_index`, `out_of_order_ledger_index`, `sample_digest_tamper`, `parameter_identity_tamper`, `parameter_digest_tamper` |
| Cancellation | `cancel_before_resampling`, `cancel_during_resampling` |
| Archive | `archive_method_version_tamper`, `archive_dataset_fingerprint_tamper`, `archive_member_checksum_tamper`, `archive_duplicate_member`, `archive_malformed_payload`, `archive_legacy_reinterpretation`, `archive_interrupted_save` |
| Native and export | `native_projection_wrong_feature`, `native_projection_incomplete_accounting`, `export_wrong_source_run`, `product_hidden_while_evidence_absent` |

The inventory is preregistration, not execution evidence. Every case still needs a source-bound immutable result showing either a correct output or its exact typed failure.

## Qualification evidence matrix

| Required role | Current state | Admission requirement |
|---|---|---|
| Method contract | Scaffold only | Independent review of estimands, equations, settings, official parity scope, output schema, and failure semantics |
| Kernel execution | Missing | Product-sized full-refit PLSc micro/published cases with exact seed, index, count, digest, and typed-failure receipts |
| Oracle independence | Missing | Two independently maintained full-reestimation PLSc bootstrap implementations, or an approved stricter V2 exception |
| Generative recovery | Missing | Preregistered bias, SE, coverage, Type-I error, power, and failure simulations with failed fits retained |
| Adversarial boundaries | Inventory only | Executed boundary, metamorphic, determinism, tamper, and cancellation matrix |
| Archive persistence | Missing | Atomic save/reopen/readback, legacy/future behavior, tamper/recovery, and replayability evidence |
| Cross-format export | Missing | Same-run semantic readback for all required formats |
| Frontend contract | Missing | GUI/CLI/native equivalence, accessible results, fail-closed projection, cancellation/retry, and hidden-state evidence |
| Packaged Windows E2E | Missing | Installed and portable offline matrix across viewport, scaling, keyboard, pointer, cleanup, and retry cases |
| Performance scale | Missing | Measured micro/applied/large/maximum/compound budgets, cancellation latency, memory, regression, and soak receipts |

## Archive, native, and export evidence requirements

### Archive persistence

An acceptable archive receipt must use the real runner and project API, append atomically, save, close, reopen, and compare the canonical analytical payload. It must cover schema versions 1 through 5, future-version read-only behavior, interrupted save/recovery, and structural plus semantic tampering. Tampering includes method and estimator versions, dataset fingerprint, archive checksum, duplicate entries/keys, malformed payloads, count/ledger order, failure reasons, parameter identities, digests, intervals, and legacy ordinary-bootstrap reinterpretation.

The current archive stores successful parameter digests but not the corresponding successful parameter vectors. A validator can check digest shape and linkage but cannot independently recompute each digest without replay. Qualification must either add a replayable witness or obtain a documented independent justification with equivalent tamper evidence. Until then, archive qualification is blocked.

### Native and GUI/CLI behavior

Evidence must prove that the same model, data, recipe, version, seed, confidence, and worker configuration produces the same plan and analytical result through GUI and CLI. It must exercise compatible setup, every invalid setup class, progress, cancellation before and during resampling, terminal cancellation within one second, no partial visible or committed result, unchanged archive, same-settings retry, accessible result and failure tables, and fail-closed handling of malformed or misattributed payloads.

While registry evidence is absent, the method must remain non-selectable. Source presence, native projection tests, or a method details surface cannot override the registry gate.

### Cross-format export

The QualificationSpec requires canonical output to CSV, XLSX, HTML, SVG, PDF, and PNG. Semantic readback is required for CSV, XLSX, HTML, SVG, and PDF. The comparison must include table IDs and cells, ordering, labels, precision, missing-value representation, warnings, unavailable reasons, footnotes, chart data, method/engine versions, seed, worker count, dataset and recipe fingerprints, and exact source-run identity. PNG requires visual/source-run evidence even though it is not a semantic readback format.

Existing CSV/XLSX/HTML source paths are implementation presence, not accepted evidence. SVG, PDF, and PNG closure is also still required by the V2 operational contract.

## Performance and packaged requirements

The frozen V2 scenario matrix has exactly five profiles: micro exact, applied, large, maximum axis, and compound stress. The maximum-axis profile separately stresses 100,000 rows, 300 indicators, 100 constructs, and 10,000 resamples. Standard hardware is Windows x86-64 with at least six logical cores and 16 GiB RAM; workstation hardware has at least twelve logical cores and 32 GiB RAM.

One warmup and five measured runs are required. Receipts gate on the frozen median elapsed time, memory, result size, cancellation latency, and no more than 20% accepted-baseline regression. Declared budgets have not been measured and are not performance claims.

Installed and portable packages must run offline at 1024x700, 1280x720, and 1440x900 and at 100%, 125%, 150%, and 200% Windows scaling. Keyboard-only operation, accessible tables, real pointer interaction, clean exit, and no orphan process or listener are mandatory.

## Fail-closed promotion decision

`validation/consistent_bootstrap_v1_promotion_audit.py` has two modes:

```powershell
python validation/consistent_bootstrap_v1_promotion_audit.py --scaffold-only
python validation/consistent_bootstrap_v1_promotion_audit.py
```

The first succeeds only when the scaffold is internally valid and promotion remains blocked. The second is the promotion decision and must exit nonzero in the current state. A generated report with `passed=false` is therefore the correct result, not a failed scaffold.

Promotion remains blocked by incomplete option breadth, missing full PLSc computational oracles, no admitted simulations or adversarial executions, missing archive/native/export/package/performance receipts, no independent scientific review, non-replayable successful parameter digests, and the unreviewed 90% usable threshold. Evidence state must remain `absent` until admitted tiers genuinely pass; a valid preregistration alone cannot change it.
