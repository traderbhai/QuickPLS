# CB-SEM exact parameter table V3

Status date: 15 August 2026

`cbsem_ml_exact_parameter_table_v3` is an Internal/Labs scientific engine
slice. It replaces the automatic-covariance/diagonal-residual projection only
for the immutable Recipe-v4 + `CompiledCbsemPlanV2` moment-input path. The
legacy schema-5 CB-SEM implementation is unchanged.

This is implemented work, not a registry, qualification, parity, Standard,
GUI, CLI, archive/export, or release claim.

## Scientific execution contract

The SemModelV4 parameter table determines the matrices estimated by the v3
path:

- every effect loading and recursive latent regression is an explicit fixed or
  free row;
- every exogenous latent variance or endogenous disturbance variance is an
  explicit row;
- a covariance between two exogenous factors is estimated only when the model
  contains a `Variable`/`Variable` covariance relation;
- a covariance between two endogenous disturbances is estimated only when the
  model contains a `DisturbanceOf`/`DisturbanceOf` relation;
- an observed error covariance is estimated only when the model contains a
  `ResidualOf`/`ResidualOf` relation; and
- all undeclared off-diagonal latent/disturbance and residual covariance cells
  are fixed to zero.

Variable endpoints involving an endogenous factor, disturbance endpoints
involving an exogenous factor, observed-variable covariance endpoints, and
mixed endpoint kinds fail with a typed diagnostic. They are not
reinterpreted. The current slice remains single-group, continuous,
common-factor, no-mean, recursive ML. Cross-loadings, causal indicators,
effects coding/linear constraints, derived parameters, feedback systems, and
group overrides remain typed exclusions.

Marker-loading and fixed-factor-variance identification are executable.
Marker loadings and fixed identification variances must be exactly one.
Fixed nonmarker loadings, regressions, variances, and supported covariance rows
are preserved in matrices and results rather than being omitted from the
optimizer plan.

## Free parameters, equality, starts, and bounds

Free rows accept finite starts, finite lower/upper bounds, and an optional
`equality_label`. Rows with the same nonempty equality label share one
optimizer dimension and one scientific value. Their parameter families must
match, their bound intersection must have a nonempty open interior, and any
explicit starts must agree. A singleton equality label, blank label,
incompatible family, conflicting start, empty bound intersection, start outside
the feasible interior, or nonpositive fixed variance fails with a stable typed
error.

Bounds are enforced through an immutable transform:

- unbounded: identity;
- lower-only: `lower + exp(raw)`;
- upper-only: `upper - exp(raw)`; and
- two-sided: logistic mapping into `(lower, upper)`.

Variance rows have an intrinsic strict lower domain of zero. A boundary value
that is scientifically intended to be exact must therefore be represented as
a fixed parameter, not approximated by a free start on the boundary.

When a start is absent, the numerical initializer follows the documented
SmartPLS starting families: loading `1`, path/covariance `0`, latent variance
`0.05`, and residual variance `0.5 * observed variance`. An explicitly stored
SemModelV4 start remains authoritative. Historical legacy-to-V4 conversions
currently store their earlier explicit `0.7` loading, `1.0` latent-variance,
and `0.5` residual-variance starts; changing migration defaults is a separate
archive-semantics decision and is not silently done by this engine slice.

The optimizer retains the existing 1,000-iteration central implementation and
its stricter current stopping constants. SmartPLS-compatible exposed choices
for its documented gradient/function tolerances and one-versus-zero start
alternative require an explicit settings-schema addition before parity can be
claimed.

## Fail-closed numerical checks

Before optimization, v3 checks:

- unique stable parameter IDs, result names, and targets;
- one explicit factor/disturbance variance and residual variance for every
  modeled diagonal;
- independent free dimensions no greater than observed covariance moments;
- local Jacobian rank of `vech(Sigma)` at the declared start;
- positive definiteness of the starting latent/disturbance, residual, and
  implied covariance matrices; and
- invertibility of the structural system and information matrix.

The finite-difference implementation uses adaptive central/one-sided
derivatives. It never substitutes a zero gradient when a derivative cannot be
evaluated. Candidate points with inadmissible covariance matrices are rejected
by the line search. Nonconvergence, singular information, unavailable
derivatives, and cancellation are terminal typed failures; no partial result is
constructed.

The result carries every fixed and free scientific row, equality-reduced free
dimension count, standard errors for free rows, standardized values, exact
implied/residual matrices, and the existing immutable compiler/plan/model/data
provenance. Modification-index inference is deliberately empty for this slice
rather than populated with heuristic zeros.

## SmartPLS Special Assumptions boundary

Current SmartPLS documentation exposes explicit settings that can imply all
exogenous-latent correlations, imply causal-indicator correlations within a
construct, or fix causal-indicator variances to one. QuickPLS v3 does not infer
these choices from missing arrows or metadata. The three reserved metadata
flags fail with
`special_assumption_requires_materialized_parameters`.

A future settings contract must materialize every implied covariance/fixed
variance as a stable compiled parameter row, include it in results and
provenance, and make the setting visible in the recipe. Only then can a
no-arrow Special Assumption be scientifically different from an undeclared
covariance fixed to zero.

## Verification and remaining blockers

Focused Rust coverage includes exact absence/presence of a declared exogenous
covariance, fixed residual-covariance effects, fixed-row result retention,
bounded equality constraints, conflicting equality starts, invalid parameter
starts/labels, non-positive-definite declared residual matrices, matrix order,
raw/covariance/scaled-correlation equivalence, sample denominators, and tamper
failures. Central shared-target results must be recorded before any pass count
is claimed.

The transparent NumPy oracle
`validation/cbsem_exact_parameter_table_v3_oracle.py` independently constructs
Lambda, B, Psi, Theta, Phi, and Sigma for latent, disturbance, and residual
covariance microcases. It imports no QuickPLS product code. It is work evidence
only and emits no admitted receipt.

Activation remains blocked on:

1. a dedicated registry option cell and frozen QualificationSpec V2 contract;
2. current-product-versus-oracle receipts plus a second maintained external SEM
   implementation or approved exception;
3. generative parameter recovery, interval coverage, difficult conditioning,
   group/worker/order metamorphic, and adversarial campaigns;
4. live Recipe-v4 authoring/preflight for each endpoint/status/constraint;
5. canonical schema-6 save/reopen and every supported export readback for the
   new v3 rows;
6. packaged Windows, cancellation, accessibility, performance, memory, and
   soak evidence; and
7. explicit materialized Special Assumptions and parity settings/defaults.

No Capability Registry V2 coverage/evidence state, customer surface, or
qualification receipt is changed by this slice.

