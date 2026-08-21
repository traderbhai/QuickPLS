# General SEM PLS simultaneous two-way moderation bootstrap v1

## Status and exact scope

This is a Standard capability with `partial` coverage and release-qualified
evidence under `rank0_streamlined_plan4b_v1`. Package and performance evidence
is representative rather than per-cell, as explicitly accepted for this
profile. The exact predicate and blocked surfaces below remain unchanged.

The executable predicate is deliberately narrow:

- a newly created schema-6 project carrying `general_sem_v1` authority;
- estimator `qpls.pls_sem.v3` with composite constructs and raw continuous,
  unweighted, single-group data;
- listwise deletion and no authored missing-value markers or transformation
  lineage;
- a direct-only acyclic structural graph;
- one or more two-operand `interaction_v2` terms using two-stage construction
  and strong hierarchy;
- every required lower-order focal and moderator path;
- same-focal or different-focal simultaneous interactions;
- indexed case resampling with replacement, a fixed seed, 2 through 10,000
  requested replicates, two-sided Type-7 percentile intervals, and the exact
  `max(2, ceil(0.9 B))` usable-replicate gate; and
- no mediation estimands, directed structural chain, higher-order construct,
  group, weight, authored conditional-probe policy, or other derived-term
  semantics.

The General SEM moderation point cell remains the primary artifact authority.
This cell adds bootstrap inference only for each interaction's scientific
rescaled gamma. Standardized-product beta, ordinary joint-stage coefficients,
fixed `-1/0/+1` slopes, and plot points remain point-only.

## Full-model replicate contract

Every indexed case replicate reruns the complete supported pipeline. QuickPLS
does not resample a frozen matrix of point-fit construct scores.

1. Draw a complete-case row index vector with replacement from the versioned
   indexed stream.
2. Rebuild the resampled dataset and rerun the shared stage-one PLS score model.
3. Orient every replicate construct-score vector against the corresponding
   sampled original score vector before constructing any interaction product.
4. Sample-standardize each operand score, rebuild every product, and recompute
   each product's mean and sample standard deviation within that replicate.
5. Refit every complete joint stage-two outcome equation with all ordinary and
   interaction predictors that belong to it.
6. Recompute ordinary coefficients, standardized-product coefficients,
   scientific gammas, fixed probes, and plot values, then validate the complete
   joint point contract.
7. Extract only the typed scientific-gamma targets for inference. A failed
   replicate is retained at its index with a typed reason; it is not silently
   replaced or removed from the requested denominator.

For standardized operand scores `z(x)` and `z(w)`, define the replicate product

```text
p_i = z(x_i) z(w_i)
z(p_i) = (p_i - mean(p)) / sd_sample(p)
```

The joint stage-two solve uses `z(p)`. If its coefficient is
`beta_product_standardized`, the inferential target is

```text
gamma_scientific = beta_product_standardized / sd_sample(p)
```

The target is bound to the interaction effect relation, parameter and generated
product column; focal relation, predictor, moderator and outcome; stage-one
scientific digest; product-scale version; and point method version. One stable,
ordered target exists per compiled interaction.

## Inference contract

For the usable gamma estimates `gamma_1, ..., gamma_U` and original point
estimate `gamma_0`, v1 publishes:

```text
bootstrap_mean = sum(gamma_b) / U
bias = bootstrap_mean - gamma_0
SE = sqrt(sum((gamma_b - bootstrap_mean)^2) / (U - 1))
CI = [Type7(alpha/2), Type7(1 - alpha/2)]
exceedances = count(|gamma_b - gamma_0| >= |gamma_0|)
p_two_sided = (exceedances + 1) / (U + 1)
```

At least `max(2, ceil(0.9 B))` of the `B` requested replicates must be usable.
Below that exact boundary, no moderation-bootstrap result is published.
Replicate failures are ordered by index and use the closed v1 code set:
insufficient observations, constant indicator, stage-one rank deficiency,
isolated construct, stage-one nonconvergence, indeterminate score sign,
constant construct score, constant interaction product, joint-stage rank
deficiency, or numerical failure.

Indexed execution makes the scientific result and failure ledger invariant to
supported worker scheduling. Cancellation is checked during resampling,
stage-one estimation, sign alignment, product/joint estimation, aggregation,
canonical assembly, and publication. Cancellation publishes no partial result.

## QuickPLS workflow boundary

The capability extends the existing QuickPLS application; it is not a second
application. A supported Recipe V4 request is compiled to the point primary
cell plus this supplemental inference cell. The native job lifecycle executes
the shared point and full-model bootstrap pipeline, renders a gamma-only
inference surface with accessible text/table information, and appends the same
canonical result to the resident schema-6 project. Strict close/reopen
validation rejects changed target identities, method receipts, counts,
intervals, failure ledgers, source pins, or digests.

The presence of native controls, canonical persistence, accessible readback,
and deterministic engine tests does not promote the cell. Cross-surface golden
equality, semantic CSV/XLSX/HTML/PDF/SVG/PNG export and report readback,
packaged Windows acceptance,
performance/memory/soak evidence, and independent review remain required.

## Versioned provenance

- capability version:
  `general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1`;
- analytical method:
  `qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1`;
- operation:
  `general_sem_pls_multiple_two_way_moderation_case_bootstrap_v1`;
- point method: `qpls.general-sem-pls.multiple-two-way.point.v1`;
- product scale:
  `qpls.general-sem-pls.two-stage-product.sample-standardized.v1`;
- resampling stream: `indexed_case_resampling_v1`;
- quantile: `type7_quantile_v1`;
- standard error: `sample_standard_error_b_minus_1_v1`;
- probability: `null_centered_plus_one_v1`;
- usable policy: `minimum_usable_fraction_0_9_v1`;
- sign orientation: `sampled_original_construct_score_covariance_v1`; and
- gamma target: `compiled_interaction_scientific_rescaled_gamma_v1`.

## Independent evidence boundary

The validation-only Python reference is production-independent and uses only
the Python standard library. It covers deterministic indexed case resampling,
score-vector orientation arithmetic, per-replicate product rescaling, complete
same-focal and different-focal joint observed-score equations, scientific gamma
rescaling, Type-7 interpolation, `B-1` standard error, null-centered plus-one
probability, the exact 90% gate, replay/evaluation-order invariance, and
constant-product and singular-equation rejection.

That reference operates on observed construct-score proxies. It does not
implement indicator-level PLS weights, loadings, iterations, score recovery,
or the production random stream, and it does not compare against production
outputs. It is a bounded independent engine smoke, not a full PLS oracle,
coverage or null-calibration study, SmartPLS numerical-parity claim, native or
packaged acceptance receipt, or release qualification.

SmartPLS public moderation and bootstrapping documentation is used only as a
terminology, workflow, and observable-output benchmark. Independent scientific
references remain mandatory, including Henseler and Chin (2010), DOI
`10.1080/10705510903439003`.

`qualification_ready=true`; `promotion_allowed=true` for this exact cell. The
streamlined release decision does not claim three-way moderation, moderated
mediation, arbitrary probes, broader data handling, or SmartPLS numerical
identity.
