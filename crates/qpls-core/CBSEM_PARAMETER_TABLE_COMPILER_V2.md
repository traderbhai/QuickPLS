# CB-SEM parameter-table compiler V2

`compile_cbsem_plan_v2` is the deterministic representation boundary between a
valid `SemModelV4` and future CB-SEM estimators. It is not an estimator and it
does not make a scientific support claim.

## Representation contract

The compiler validates `SemModelV4`, canonicalizes declaration order, and
retains every scientific object:

- observed, common-factor, composite, and derived variables;
- effect loadings and causal measurement weights;
- structural regressions and their optional intercept parameters;
- variable, residual, disturbance, and mixed-endpoint covariances;
- a complete parameter table containing target, free/fixed/derived status,
  start values, bounds, equality labels, expressions, and group overrides;
- variance, covariance, intercept, mean, and ordinal-threshold parameters;
- equality, bound, and linear constraints;
- derived terms, group declarations, and feedback/nonrecursive structure; and
- raw-data options or complete covariance/correlation matrix metadata,
  including moments and group sample sizes.

Annotations and canvas presentation are intentionally absent. The plan can
reconstruct the canonical scientific `SemModelV4`, and the reconstructed model
must have the same scientific SHA-256. Plan JSON rejects unknown fields and
`deterministic_sha256` fingerprints the complete typed representation.

## Representation is not execution support

`validate_cbsem_ml_v1_estimator_capability_v2` evaluates the represented plan
against the existing bounded ML-v1 boundary. It currently reports, rather than
drops, unsupported groups, group overrides, mean/threshold structures,
structural intercepts, categorical variables, derived or composite semantics,
causal measurement, extended raw-data handling, and extended matrix metadata.

`ensure_cbsem_ml_v1_estimator_capability_v2` converts that report into a typed
error. The recipe-v4 compiler calls this second gate before it labels a plan
with the bounded `qpls3.cbsem.ml` capability cell. A separate internal
engine-only adapter can now execute the supported raw, covariance, and scaled-
correlation subset while applying stricter moment-input checks. Thus a model
may always be stored and compiled even when the current estimator is not
allowed to run it.

## Deliberate activation boundary

This foundation does not change the live legacy CB-SEM product dispatch. The
internal moment-input adapter reaches the bounded ML optimizer, but it does not
activate the Recipe-v4 job service, archive schema 6, GUI, CLI, result/export,
or registry-promotion paths. Numerical execution of multigroup, mean structure,
thresholds, causal indicators, moderation, or nonrecursive models remains
unsupported until each estimator path is implemented and independently
qualified. See `docs/methods/CBSEM_MATRIX_INPUT_V2.md` for the exact engine
boundary and remaining activation work.

## Verification

Focused tests cover hand-sized CFA/SEM fixtures, all covariance classifications,
free/fixed/derived parameter rows, starts and bounds, equality labels,
constraints, group overrides, matrix metadata, feedback, derived/composite
representability, JSON round-trip, declaration reordering, scientific
reconstruction, and deterministic plan digests.
