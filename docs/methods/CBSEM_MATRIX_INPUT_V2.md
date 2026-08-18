# CB-SEM compiled moment input V2

Status date: 16 August 2026

`cbsem_ml_compiled_moment_input_v2` is an internal execution slice for a bounded
`CompiledCbsemPlanV2`. It makes covariance and correlation input scientifically
executable without fabricating case rows. It is connected to the internal
Recipe-v4 native job and canonical schema-6 result boundary. It is registered
as a separate `partial`/`absent` Labs option cell, but it has no Standard/GUI/CLI
activation or format-export qualification and is not release-qualified.

The scientific parameter projection documented below is the historical v2
boundary. The active Internal/Labs Recipe-v4 engine now emits the separately
versioned `cbsem_ml_exact_parameter_table_v3` result described in
`CBSEM_EXACT_PARAMETER_TABLE_V3.md`. V3 removes automatic exogenous covariance,
adds explicit latent/disturbance/residual covariance plus fixed/free/equality/
bound semantics, and leaves this v2 evidence package unborrowed and
unpromoted.

## Implemented boundary

The entry point `estimate_cbsem_ml_compiled_moments_v2` accepts the source
`AnalysisRecipeV4`, its immutable compiled artifact, the resolved `SemModelV4`,
and the exact in-memory dataset. Before estimation it:

- deterministically recompiles and verifies the artifact receipt;
- checks the compiled dataset fingerprint and recomputes dataset integrity from
  the current Arrow bytes and schema;
- requires exact dataset UUID, input-kind, variable-ID, source-column, and
  sample-size binding;
- canonicalizes matrix order without changing scientific variable identity;
- rejects nonsquare, nonnumeric, nonfinite, asymmetric, or non-positive-
  definite matrices;
- requires a correlation matrix to have a unit diagonal, values in `[-1, 1]`,
  and one finite positive standard deviation for every modeled variable; and
- preserves whether a supplied moment matrix uses denominator `n - 1` or `n`,
  then records the exact ML covariance matrix consumed by the optimizer.

Raw input retains the existing bounded behavior: continuous numeric indicators,
listwise deletion, no weight, cluster, or strata variable, and ML covariance
with denominator `n`. Matrix inputs require an exact declared sample size of at
least 10. Covariance input rejects separate means and scale metadata.
Correlation input rejects means and is explicitly converted to covariance with
the supplied standard deviations. Effective sample size, supplied degrees of
freedom, group sample sizes, and group estimation remain unsupported and fail
with typed diagnostics.

The executable model slice is deliberately narrow:

- single-group continuous common-factor CFA or recursive latent SEM;
- marker-loading identification with the marker fixed exactly to one;
- free unconstrained nonmarker loadings and structural regressions;
- explicit positive factor/disturbance and observed-residual variances;
- explicit covariances between every pair of exogenous factors; and
- unstandardized preprocessing with no mean structure.

Composites or causal measurement, derived terms and interactions, categorical
variables and thresholds, transformed indicators, cross-loadings, residual or
disturbance covariances, parameter constraints, bounds beyond the intrinsic
nonnegative variance boundary, structural intercepts, groups, and feedback
models are represented by `CompiledCbsemPlanV2` but rejected by this bounded
adapter. Nothing is silently dropped.

The result records compiler, plan, and scientific-model identities; input kind;
declared and used sample sizes; omitted raw cases; denominator and correlation
scales; canonical variable order; canonical ML-covariance SHA-256 and values;
the stable SemModelV4 ID for every engine parameter; and the bounded ML result.
A nonconverged optimizer result is returned as a typed failure.

## Internal native job and canonical-result boundary

The internal `cbsem_plan_v2` command and asynchronous job use the exact
`smartpls.cbsem / qpls3.cbsem.ml / cbsem_ml_v1` capability-cell identity. The
job shares one atomic four-job and CPU-worker admission budget with existing
Standard and internal Recipe-v4 work; there is no second CB-SEM pool. It:

- resolves only the exact resident project dataset and fingerprint;
- recompiles and revalidates the immutable Recipe-v4 receipt before execution;
- checks cancellation before compilation, during every optimizer iteration and
  line search, and before result publication;
- rechecks the active project and dataset immediately before publication;
- catches worker panics and retains typed terminal failures; and
- never exposes or commits a partial result after cancellation or failure.

Successful jobs produce one native `CanonicalResultDocumentV2` containing run
and input provenance, the exact canonical ML covariance, stable-ID-bound
unstandardized parameters, standardized parameters, fit, implied covariance,
residual covariance, and residual correlation. The document is validated
against both the core canonical contract and the project schema-6 type before
publication. The existing digest-bound schema-6 append service receives that
exact native document, and readback tests compare its canonical JSON after
save/reopen. Schema-5 projects are never mutated by this path.

## Raw-versus-moment equivalence contract

Equivalent raw, sample-covariance, ML-covariance, and scaled-correlation inputs
must produce the same canonical ML covariance to `1e-12`. Both optimizations
must also report convergence and a final gradient norm no greater than the
optimizer's accepted stagnation threshold (`1e-5`).

Raw-scale parameter distance is not used as the numerical oracle: a gradient
stopping rule cannot bound that distance without a qualified Hessian condition
number. The qualification microcase instead compares:

- objective values using the optimizer's objective-stagnation absolute bound
  (`1e-12`) plus its finite-difference relative step (`1e-6`); and
- `std.lv`, `std.all`, and implied-covariance cells using the accepted gradient
  absolute bound (`1e-5`) plus the finite-difference relative step (`1e-6`).

Tests mutate values inside and beyond both envelopes and reject nonfinite
values, so the bounds cannot be widened implicitly to accommodate a fixture.
The optimizer uses the same named constants, preventing the test contract from
drifting away from its stopping rules.

## Current verification, not qualification

The shared development test run on 14 August 2026 passed:

- 7 of 7 focused `cbsem_matrix_input` engine tests;
- 2 of 2 focused core matrix-metadata/schema round-trip tests; and
- 75 of 75 desktop dependency tests in the concurrent integration batch.

The focused coverage includes a hand microcase across raw, covariance, and
scaled correlation inputs; variable-order invariance; exact parameter-ID and
semantic-role binding; denominator and sample-size behavior; typed unsupported
metadata; wrong shape; singular/non-positive-definite matrices; compiled-
artifact tampering; dataset fingerprint tampering; current-byte tampering; and
equivalence-boundary mutations.

These green tests establish a kernel boundary only. They are not independent
oracle, simulation, archive/export, packaged-Windows, accessibility,
performance, soak, or full QualificationSpec V2 evidence. The separate Registry
cell records this bounded contract at `partial`/`absent`; no evidence state or
customer surface was promoted.

For the 15 August native integration, the central shared-target batch passed 8
of 8 focused CB-SEM matrix-input tests and all 83 desktop tests, including the
internal command, shared job budget, cancellation, native canonical document,
and exact schema-6 reopen cases. TypeScript compilation and all 28 focused
project-service tests also pass. These integration gates do not constitute
scientific or packaged-product qualification.

## Independent oracle work evidence

`cbsem_matrix_input_numpy_scipy_oracle_v1` is a transparent validation-only
implementation. It imports no QuickPLS product code and invokes no product
binary. The oracle implements the covariance-structure ML discrepancy

`F_ML = log|Sigma| + trace(S Sigma^-1) - log|S| - p`

for a marker-identified three-indicator common factor. It independently codes
the analytic gradient, audits it against central differences, and compares the
optimizer to the algebraic three-indicator solution. The method contract is
grounded in Kenneth Bollen's [Structural Equations with Latent Variables](https://doi.org/10.1002/9781118619179);
the validation optimizer uses the documented SciPy
[BFGS gradient stopping contract](https://docs.scipy.org/doc/scipy/reference/optimize.minimize-bfgs.html).

The preregistered small work matrix contains two population parameterizations,
six fixed-seed raw samples, raw/sample-covariance/scaled-correlation
representations, exact denominator conversion, and typed non-positive-
definite, wrong-shape, sample-size-mismatch, invalid-denominator, and missing-
correlation-scale failures. It also freezes the seven expected engine-name to
SemModelV4 parameter-ID mappings, including the fixed marker parameter.

The checked-in work report currently passes all of these validation-only
checks. The largest analytic-versus-numeric gradient delta is below `8e-10`;
raw/covariance/correlation canonical moments agree within `2e-15`; exact
population parameters are recovered within `4e-8`; and all preregistered
failure codes match. These are observed work-report values, not product
acceptance tolerances or qualification claims.

NumPy 1.26.4 and SciPy 1.15.2 were available locally. No R/lavaan, semopy, or
other maintained SEM implementation was available, and no dependency was
installed. The report therefore remains `qualification_role_satisfied=false`
and `receipt_eligible=false`. It also lacks a frozen current-product numerical
comparison and is deliberately too small to claim Monte Carlo recovery.

## QualificationSpec V2 work factory

The compatibility-only candidate specification binds the dedicated
cell `smartpls.cbsem / qpls3.cbsem.ml.matrix_input /
cbsem_ml_compiled_moment_input_v2`. That cell and qualification link now exist
in Capability Registry V2 as `partial`/`absent` Labs authority, so the factory
can verify its exact identity without borrowing readiness from the broader
`qpls3.cbsem.ml` cell.

The specification passes QualificationSpec V2 schema and semantic validation,
including the scientific estimands, ordered preprocessing, typed predicates,
pairwise/maximum/compound scenarios, per-output comparisons, archive/export,
Windows, cancellation, performance, and ten-role evidence contracts. Its
migration status is `compatibility_only`, its receipt list is empty, registry
verification is true, and `qualification_ready` is false.

The factory classifies only the method contract, oracle, small generative, and
oracle-side adversarial material as `work_evidence_only`. Kernel product
execution, archive, cross-format export, frontend, packaged Windows, and
performance roles remain blocked. It emits zero candidate receipt descriptors,
does not mutate the registry or legacy manifest, and sets
`promotion_allowed=false`.

Focused validation passed 13 of 13 Python tests, Ruff, deterministic report
regeneration, factory source/scenario hashing, and standalone QualificationSpec
V2 validation. Mutation tests prove that a changed work report becomes stale
and cannot be treated as current evidence.

## Remaining activation and qualification path

1. Add live setup and preflight for raw, covariance, and correlation datasets;
   require sample-size, denominator, and correlation-scale metadata in the
   editor rather than synthesizing defaults.
2. Exercise the same recipe through GUI and CLI, then qualify CSV, XLSX, HTML,
   SVG, and PNG semantics where applicable.
3. Keep the registered matrix-input cell outside Standard and its
   compatibility-only QualificationSpec V2 receipt list empty until product
   wiring and immutable role evidence exist. Add a frozen current-product
   comparison, published fixtures, and a second maintained covariance/
   correlation SEM implementation or a genuinely approved exception.
4. Complete generative recovery, difficult conditioning, missingness rejection,
   declaration and worker invariance, archive tamper/recovery, packaged Windows,
   accessibility, cancellation, performance, memory, and soak evidence.
5. Keep the cell outside Standard until coverage is full and all eight evidence
   stages pass. General means, constraints, groups, thresholds, moderation, and
   nonrecursive estimation remain separate CB-SEM engine work rather than being
   implied by this moment-input adapter.

## Source locations

- `crates/qpls-estimation/src/cbsem_matrix_input.rs`
- `crates/qpls-estimation/src/pls.rs` (bounded optimizer bridge and numerical
  stopping constants)
- `crates/qpls-core/src/sem_model_v4.rs` (matrix sample metadata)
- `crates/qpls-core/src/compiled_cbsem_plan_v2.rs` (typed capability checks)
- `crates/qpls-runner/src/recipe_v4_cbsem_execution.rs` (cancellable immutable
  runner adapter and execution provenance)
- `src-tauri/src/recipe_v4_cbsem_execution.rs` (internal access, compilation,
  resident-data resolution, and typed failure mapping)
- `src-tauri/src/recipe_v4_jobs.rs` (shared admission, cancellation, and atomic
  result publication)
- `src-tauri/src/recipe_v4_cbsem_canonical_result.rs` (canonical result and
  schema-6 validation)
- `validation/cbsem_matrix_input_v2_oracle.py` (transparent NumPy/SciPy oracle,
  small generative matrix, and typed adversarial cases)
- `validation/cbsem_matrix_input_v2_qualification_factory.py` (fail-closed
  source/scenario-bound work-evidence factory)
- `validation/qualification_v2/cbsem_matrix_input_v2.qualification.json`
  (compatibility-only candidate contract with zero receipts)
- `validation/results/method_factory/cbsem_matrix_input_v2/` (unadmitted work
  report and qualification factory audit)
