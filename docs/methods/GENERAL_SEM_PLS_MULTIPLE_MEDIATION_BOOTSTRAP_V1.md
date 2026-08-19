# General SEM PLS multiple-mediation bootstrap v1

## Status and exact identity

- Capability owner: `smartpls.mediation`
- Capability cell: `qpls3.pls.general_sem_multiple_mediation_bootstrap`
- Method version: `general_sem_pls_full_model_case_bootstrap_v1`
- Product surface: opt-in Experimental Labs
- Coverage: partial
- Evidence maturity: engine-only
- QualificationSpec V2 readiness: false

This cell records one executable combination on the opt-in Labs surface. It
does not promote the generic mediation or bootstrap families and does not
inherit release status from the bounded Standard PLS algorithm or
indexed-resampling primitive.

## Supported model predicate

The request must resolve to a newly created schema-6 project carrying
`sem_generation = general_sem_v1` and compile for estimator
`qpls.pls_sem.v3`. The scientific graph must be a single-group recursive DAG
containing at least two specific indirect paths. Parallel, serial, and mixed
multiple-mediation paths may coexist.

The current scoring boundary accepts only constructs and relations supported by
`CompiledPlsPlanV2`: composite constructs with supported Mode A, Mode B, unit,
or custom scoring; continuous observed indicators; ordinary free relation
parameters; and structural relations between composites. Feedback, covariance
relations, common factors, observed-variable structural paths, constraints,
derived terms, interactions, higher-order constructs, group overrides, and
structural intercepts are blocked.

Conditional-effect probes and lazy specific-path materialization are blocked.
Specific paths are identified by their stable ordered relation IDs rather than
diagram coordinates or labels.

## Supported data and inference predicate

- raw numeric observations;
- listwise deletion;
- no case weights, cluster variable, or strata variable;
- no observed-variable missing-marker or transformation lineage metadata;
- case resampling with replacement and a complete model refit in every usable
  replicate;
- 2 through 10,000 requested replicates;
- JavaScript-safe nonnegative integer seed;
- two-sided inference only;
- percentile interval only, calculated by the frozen Type-7 quantile rule;
- sample standard error with denominator `B_usable - 1`;
- null-centred plus-one two-sided probability;
- publication only with at least `max(2, ceil(0.9 * B_requested))` usable
  replicates; and
- deterministic indexed streams, with scientific equality across supported
  worker counts after normalizing operational worker metadata.

BCa, studentized, and one-sided cells are not part of this version.

## Estimands

For a directed path `p` with two or more relations, the specific indirect
effect is the product of its ordered structural coefficients:

`specific(p) = product(beta_e for e in p)`.

For an ordered source-target pair, the total indirect effect is the sum over
all eligible specific paths. The total effect is the direct effect, if present,
plus the total indirect effect. Every reported result retains its stable path,
relation, model, recipe, dataset, plan, and capability identities.

The bootstrap distribution is formed by re-estimating the complete PLS model
for each indexed case-resample and then recomputing the same frozen estimands.
Failed replicates remain in the typed failure ledger and in the requested-count
denominator used by the 90% publication gate.

## Compatibility references

SmartPLS publicly documents that parallel and serial mediators should be
estimated together and that results include direct, specific indirect, total
indirect, and total effects:
[SmartPLS mediation](https://smartpls.com/documentation/algorithms-and-techniques/extended-relationships/mediation/).

SmartPLS publicly describes full re-estimation on case-resamples, percentile
intervals, two-sided testing, fixed seeds, and typical 10,000-subsample use:
[SmartPLS bootstrapping](https://smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/bootstrapping/).

These sources define observable workflow compatibility only. SmartPLS random
streams and undocumented internals are not an oracle, and this cell makes no
claim of numerical identity.

## Current evidence and remaining gates

The engine evidence binds the frozen contract, an implementation-independent
micro-reference for path products and inference summaries, and the implemented
Rust compiler/kernel/runner boundary. A separate independent deterministic
Monte Carlo report supplies a bounded engine smoke reference for unstandardized
observed-score path products, bootstrap means, Type-7 percentile truth
inclusion, and the usable-replicate gate: 96 mixed-mediation samples, 240
observations per sample, and 199 complete downstream OLS path-system refits per
case bootstrap. All frozen effects meet the declared bias, RMSE, bootstrap-bias,
and truth-inclusion smoke thresholds; hit counts and descriptive Wilson
intervals are reported. Deterministic replay, singular-fit rejection,
reconciliation, and the exact 90% usable-replicate boundary also pass.

That simulation never calls or compares production Rust. Its OLS coefficients
are unstandardized, whereas PLS path scores are standardized, and it does not
test sample standard errors, plus-one probabilities, null calibration,
interval-width efficiency, or latent-score recovery. With 96 trials, its
truth-inclusion rates are a smoke screen rather than nominal-coverage
qualification. It does not replace the pending independent full-PLS
simulations, null-rejection calibration, or SmartPLS observable comparison.

QuickPLS now connects the same-app native Labs project-mode workflow from a
newly created schema-6 `general_sem_v1` project through dataset/model
activation, preflight, job execution, progress, cancellation, canonical result
readback, persistence, close, and reopen. This implementation evidence does
not supply independent native or packaged qualification, a cross-runtime
golden, semantic export qualification, or release acceptance.

Before any maturity beyond `engine_only`, the cell still requires:

1. an independent full PLS-PM implementation that refits every replicate;
2. a public SmartPLS observable comparison with version and all settings
   recorded, plus explained differences;
3. full PLS-PM recovery, bias, interval-coverage, null-rejection, and
   failure-denominator simulations across parallel, serial, and mixed graphs;
4. a resident-authority-valid Rust result to deterministic schema-6 `.qpls`
   archive to frontend golden readback fixture;
5. semantic comparison and CSV/XLSX/HTML/SVG/PDF/PNG export readback;
6. independent qualification of the connected schema-6 native configuration,
   dataset/model activation, preflight, calculation, progress, cancellation,
   persistence, reopen, and recovery flows;
7. packaged offline Windows, accessibility, scaling, performance, memory, and
   soak evidence; and
8. a native QualificationSpec V2 with all eight immutable receipt stages and a
   strict report returning `qualification_ready: true`.

Until the later gates pass, the cell must remain `partial / engine_only / labs`.
The observed-score smoke reference cannot be reused as archive, native, or release
qualification evidence.
