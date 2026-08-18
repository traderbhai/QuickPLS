# QualificationSpec V2

QualificationSpec V2 is a new, isolated qualification lane for a QuickPLS method **within one exact Capability Registry V2 cell**. It defines what the method estimates, when it applies, how outputs are compared, which difficult scenarios are mandatory, and which immutable evidence receipts are required before a cell can be considered promotion-ready.

This protocol does not modify or reinterpret the existing method-promotion manifests. In particular, a legacy `declared_state: release_qualified` is historical input only. It is not evidence of full V2 coverage.

## Files and authority

- `validation/qualification_v2/qualification_spec_v2.schema.json` is the Draft 2020-12 wire schema.
- `validation/qualification_spec_v2.py` supplies Python types, schema and semantic validation, strict receipt verification, registry-cell verification, the CLI, and the report-only V1 adapter.
- `validation/qualification_v2/fixtures/pls_algorithm_v1.migrated.json` is a realistic PLS Algorithm migration example.
- `validation/qualification_v2/fixtures/capability_registry_v2.fixture.json` is its non-authoritative registry fixture.
- `validation/test_qualification_spec_v2.py` contains mutation-oriented fail-closed tests.

The migrated example intentionally remains `compatibility_only`. Its legacy evidence bytes can be verified, but its unresolved list records why those bytes are not native V2 receipts and why the fixture cannot promote a product capability.

## Outcome semantics

The validator reports two different outcomes:

| Outcome | Meaning |
| --- | --- |
| `passed` | The document satisfies the JSON schema and all semantic invariants evaluated in the current call. In strict mode, supplied external registry and receipt checks are also part of this result. |
| `qualification_ready` | `passed` is true, migration status is `native` or `completed`, the exact Capability Registry cell was verified, and every receipt byte was verified. |

A normal validation can pass while `qualification_ready` is false. This lets authors iterate on a frozen contract without claiming that evidence exists.

Current source-scaffold examples make that distinction explicit:

| Exact registry cell | QualificationSpec V2 | Current result |
| --- | --- | --- |
| `qpls3.inference.consistent_bootstrap` / `plsc_bootstrap_v1` | `validation/qualification_v2/consistent_bootstrap_v1.qualification.json` | Schema, semantics, and registry identity pass; migration is compatibility-only, receipts are empty, and `qualification_ready` is false. |
| `qpls3.assessment.htmt` / `ringle_et_al_htmt_plus_v1` | `validation/qualification_v2/htmt_plus_v1.qualification.json` | Schema, semantics, and registry identity pass; required product breadth remains open, receipts are empty, and `qualification_ready` is false. |
| `qpls3.assessment.model_fit` / `pls_model_fit_v2` | `validation/qualification_v2/pls_model_fit_exact_v1.qualification.json` | Schema, semantics, and registry identity pass; the independent full-refit oracle and downstream qualification stages remain open, receipts are empty, and `qualification_ready` is false. |

The post-hoc technical sample-size v2 cell currently has a strict cell contract,
not a native QualificationSpec V2. The CB-SEM covariance/correlation source
adapter likewise has no registry cell or QualificationSpec V2 yet. Neither
source foundation may borrow readiness from a related method or from a bounded
release-qualified cell.

The V1 adapter returns a `LegacyManifestProjection`, not a V2 specification. Its fixed controls are:

- `v2_coverage_status: unassessed`;
- `promotion_authority: false`;
- `qualification_ready: false`;
- `capability_cell_candidate: null` unless an exact four-field mapping is supplied;
- `source_declared_state_is_informational_only: true`; and
- an explicit list of unresolved V2 requirements.

No adapter output field maps legacy `release_qualified` to full V2 coverage.

## One specification per method cell

Every specification binds these identities:

```json
{
  "registry_schema_version": 2,
  "capability_id": "smartpls.pls_algorithm",
  "cell_id": "qpls3.pls.algorithm",
  "capability_version": "pls_pm_v1"
}
```

Do not qualify a broad UI label when model family, estimator, input representation, missing-data policy, inference mode, or output contract differs. Split those combinations into separate registry cells and separate specifications. This prevents a strong result in one supported cell from masking an unsupported cell.

The link uses the exact four-field identity defined by Capability Registry V2. `capability_version` must equal the QualificationSpec `method_version`. The validator resolves it only from the authoritative `capabilities[].qualification_links[]` collection:

```json
{
  "registry_schema_version": 2,
  "capabilities": [
    {
      "capability_id": "smartpls.pls_algorithm",
      "qualification_links": [
        {
          "registry_schema_version": 2,
          "capability_id": "smartpls.pls_algorithm",
          "cell_id": "qpls3.pls.algorithm",
          "capability_version": "pls_pm_v1"
        }
      ]
    }
  ]
}
```

Matching is exact: aliases, fuzzy IDs, malformed or duplicate links, containing-capability mismatches, capability-version mismatches, and method-version mismatches fail closed. Coverage and evidence remain separate registry axes; a matching link or `release_qualified` evidence state does not imply `coverage_state: full`.

## Qualification ladder

Every native or completed specification must cover all eight stages. Multiple roles may exist at one stage, but roles and artifact paths are unique.

| Stage | Required proof | Typical tests |
| --- | --- | --- |
| `contract` | Frozen estimands, equations, preprocessing, applicability predicates, diagnostics, output shapes, and method identity | Tiny published examples, hand calculations, schema and recipe contracts |
| `kernel` | Estimator implementation agrees with the frozen contract and behaves deterministically where promised | Focused unit tests, exact serialization, worker-count invariance, convergence and failure contracts |
| `oracle` | Independent results for every estimand, with provenance and versioned implementations | Published fixtures plus at least two computational independence groups |
| `generative` | Statistical recovery, calibration, and failure accounting under known populations | Seeded simulations, randomized properties, Monte Carlo intervals, failed-fit denominator checks |
| `adversarial` | Boundary, malformed, degenerate, non-identifiable, and numerically difficult cases | Zero variance, collinearity, singularity, extreme scaling, missingness, invalid graphs, overflow/underflow |
| `persistence_export` | Archive compatibility and cross-format semantic equivalence for the same completed run | Version round trips, corrupt archive matrix, canonical read-back comparison, provenance checks |
| `packaged_windows` | Installed and portable packages exercise the real workflow offline | Packaged E2E, real pointer gestures, keyboard-only flow, accessible tables, viewport and display scaling |
| `scale_reliability` | Declared complexity ceilings, performance budgets, cancellation, retry, memory, and result-size limits | Warm benchmark baselines, maximum-axis runs, compound stress, cancellation-before-commit, repeated-run stability |

A receipt cannot predate `spec_frozen_at_utc` or carry an implausible future timestamp. A contract change therefore requires a new freeze and new receipts; old receipts cannot silently qualify changed estimands or scenarios.

## Scientific contract

### Estimands

An estimand is the exact target quantity, not a screen name. Each estimand declares:

- a stable ID and human label;
- an operational definition;
- its unit or scale; and
- all output IDs that realize it.

Output IDs are globally unique across estimands. The comparison contract must cover exactly those IDs—no missing outputs and no extra convenient outputs.

### Ordered preprocessing

Preprocessing steps use a unique, contiguous, zero-based order. Each step declares its operation, parameters, and affected inputs or outputs. Missing-data handling, centering/scaling denominator, categorical encoding, sign orientation, correlation-matrix handling, and deterministic tie-breaking belong here when applicable.

This makes preprocessing part of the estimand rather than an undocumented implementation detail.

### Model and data predicates

Model and data applicability are executable contracts, each with:

- an explicit predicate expression;
- `error`, `not_applicable`, or `warning` behavior; and
- a stable diagnostic code.

Diagnostic codes must be globally unique. Unsupported inputs are qualified through explicit rejection scenarios, not by silently omitting them from the test matrix.

### Independent oracle strategy

Every estimand needs:

1. at least one primary-literature oracle; and
2. normally at least two computational oracle sources in different independence groups, including a versioned independent implementation.

The same implementation name, version, and maintainer cannot be relabeled as two independence groups. External GPL or otherwise incompatible packages remain `development_validation_only` and are never runtime dependencies of the proprietary application.

Where two computational sources genuinely do not exist, `oracle_exception` must name the reason, approver, approval timestamp, and at least two compensating evidence items. An exception lowers oracle diversity; it does not relax the rest of the ladder.

Recommended oracle mix, in descending independence:

- published paper table or author fixture;
- hand calculation for a micro case;
- separately maintained implementation in another language or numerical stack;
- a second independently maintained implementation;
- invariants and metamorphic relations; and
- seeded population-recovery simulations.

QuickPLS output should never be used as its own oracle.

## Scenario contract

### Mandatory axes

Every method declares these seven axes, with at least two values each:

- `model_topology`;
- `measurement_model`;
- `data_distribution`;
- `missingness`;
- `input_type`;
- `workload`; and
- `workers`.

Method-specific axes may be added. Negative-contract values—such as a summary-matrix input that the cell must reject—are valid and encouraged when paired with the expected diagnostic.

At least one `pairwise` combination must cover every pair of declared axis values. Targeted combinations add known fragile cases. A `compound` combination is mandatory for `compound_stress`. Every required complexity profile must appear in at least one combination.

Every combination declares `stressed_dimensions`. It is empty for micro, applied, large, and pairwise rows. Each `maximum_axis` row must be targeted and name exactly one dimension; the set of those rows must cover every workload field whose maximum exceeds the applied profile. A `compound_stress` row must name at least two dimensions. This makes “maximum axis” and “compound” mechanically different instead of relying on prose.

### Mandatory complexity profiles

| Profile | Purpose |
| --- | --- |
| `micro_exact` | Small enough for hand calculation, exact status/label checks, and exhaustive boundary enumeration |
| `applied` | Typical research workload used for primary workflow, oracle, and export qualification |
| `large` | Large routine desktop workload used for deterministic scheduling, memory, and performance regression |
| `maximum_axis` | Each documented maximum dimension is stressed separately with other dimensions controlled |
| `compound_stress` | Several high dimensions are combined to expose interaction failures, cancellation races, and result explosion |

The workload fields are `rows`, `indicators`, `constructs`, `resamples`, `groups`, and `candidate_models`. Non-applicable dimensions use zero or null according to the method contract. `micro_exact`, `applied`, and `large` values must be non-decreasing.

For `maximum_axis`, the workload is the ceiling vector. A combination uses the ceiling only for its single `stressed_dimensions` entry and holds other dimensions at the applied profile. For `compound_stress`, every named stressed dimension uses its compound value. At least one maximum dimension and at least two compound dimensions must genuinely exceed applied values.

The Monte Carlo policy fixes confidence level, maximum interval half-width, and requires failed fits to remain in the denominator. This prevents optimistic recovery claims caused by dropping hard failures.

## Per-output comparison rules

Numerical tolerances are output-specific; one global epsilon is not scientifically defensible. Every row also carries a nonempty rationale explaining the output's equivalence class, scale, oracle precision, and chosen bounds.

| Rule | Required parameters | Appropriate use |
| --- | --- | --- |
| `exact` | none | IDs, labels, dimensions, deterministic statuses, counts, serialized categorical results |
| `abs_relative` | absolute and relative tolerances | Scalars and elementwise coefficients spanning zero and non-zero magnitudes |
| `matrix_norm` | absolute and relative tolerances, norm, elementwise cap | Covariance, effect, Hessian, or loading matrices where global and local error both matter |
| `sign_orientation` | absolute and relative tolerances, orientation keys | Scores, weights, loadings, eigenvectors, or components equivalent up to sign |
| `subspace` | maximum principal angle and projector tolerance | Repeated-root or rank-deficient solutions where a basis is not unique |
| `label_permutation` | assignment metric plus absolute and relative tolerances | Mixtures, segments, classes, or clusters equivalent up to label permutation |
| `monte_carlo_interval` | confidence level, half-width, acceptance interval | Coverage, rejection rate, convergence rate, bias or recovery assessed over simulations |

Irrelevant parameters are rejected. Monte Carlo comparison confidence must match the scenario policy and its half-width cannot be weaker than the global limit.

Tolerance values must be justified from method scale, oracle precision, conditioning, and expected platform variation. They should not be widened merely to make a failing example pass.

## Randomized, property, and metamorphic evidence

The schema records the scenario and receipt contracts; method-specific receipt producers should implement these families where applicable:

- fixed-seed randomized valid models across every supported topology and measurement family;
- deterministic replay and worker-count invariance;
- row permutation invariance;
- permitted indicator/construct relabeling equivariance;
- scale and location transformations consistent with preprocessing;
- sign/orientation equivalence;
- duplicate-run and same-settings retry equivalence;
- known nested-model or monotonicity relations;
- analytically constrained matrices: symmetry, positive semidefiniteness, diagonals, probability/simplex bounds;
- no NaN/Infinity in successful outputs; and
- failed fits counted and classified, never discarded.

Generators must record seed, generator version, population parameters, expected invariants, shrink/reproduction payload, and the exact scenario-set digest.

## Adversarial and boundary matrix

Each applicable method should explicitly cover:

- zero rows, one row, minimum valid rows, and just-over-limit rows;
- zero indicators, empty blocks, single-indicator blocks, maximum indicators, and duplicate assignments;
- cycles, disconnected constructs, unsupported edges, unidentified blocks, and saturated/near-saturated models;
- constant, near-constant, duplicated, collinear, ill-conditioned, singular, and non-positive-definite data;
- all-missing columns, all-missing rows, patterned missingness, and post-filter sample collapse;
- extremely small/large values, mixed scales, overflow/underflow pressure, and locale-sensitive input;
- convergence at iteration one, at the limit, and non-convergence;
- repeated roots, sign ambiguity, label switching, and multiple equivalent optima;
- interrupted save, corrupt/duplicate archive entries, wrong feature or method identity, and future schema versions;
- cancellation before work, during expensive work, just before commit, during export, and immediate retry; and
- repeated open/run/export cycles to expose handle, memory, and state leaks.

Expected outcomes must be explicit: valid result, warning, not-applicable diagnostic, or hard error. A crash, hang, silent coercion, partial visible result, or partially committed archive always fails.

## Performance and reliability

Each required profile has a budget for every declared hardware class:

- maximum elapsed time;
- maximum peak working set;
- maximum result bytes; and
- maximum cancellation latency.

Hardware classes specify Windows/x86-64, minimum logical cores, memory, and notes. Baselines require at least one warmup and five measured runs and use median or p95. Runtime, memory, and result budgets must be non-decreasing through the `large` profile; maximum-axis and compound budgets cannot be below the large budget. Receipts at `packaged_windows` and `scale_reliability` must be captured on Windows and satisfy at least one declared hardware class.

Baseline comparison records both an absolute budget and allowed regression percentages. A faster baseline does not erase the absolute customer-facing ceiling, and a generous absolute ceiling does not hide a material regression.

## Archive and cross-format export

The archive contract identifies current, readable, and writable schema versions. Writable versions are a subset of readable versions, and the current version is both readable and writable. Future versions open only under the `verified_read_only` policy.

The full corruption matrix is mandatory: feature identity, method version, dataset fingerprint, checksum, duplicate entry, malformed payload, legacy reinterpretation, and interrupted save.

Export qualification requires CSV, XLSX, HTML, SVG, PDF, and PNG generation. CSV, XLSX, HTML, SVG, and PDF require semantic read-back against one canonical result projection from the same completed run. Provenance is mandatory, and validation-only witness values must be absent from customer exports.

Cross-format comparison should canonicalize table IDs, row/column labels, ordering, numeric values, missing-value representation, run identity, method version, dataset fingerprint, and units before applying the per-output comparison rules.

## Windows packaged E2E and cancellation

Both installed and portable packages are mandatory. Qualification covers at least 1024×700, 1280×720, and 1440×900 at 100%, 125%, 150%, and 200% display scaling. The run must be offline, exercise real pointer events for diagram interactions, support keyboard-only operation, and expose result tables accessibly.

Cancellation declares applicability for validation, estimation, resampling, comparison, and export. Potentially long estimators must be cancellable during estimation. Required invariants are:

- no partial result becomes visible;
- no partial result is committed;
- the archive remains unchanged;
- the same settings can be retried; and
- observed cancellation latency stays within both the phase contract and performance budget.

## Immutable receipt descriptors

Every receipt binds:

- role and ladder stage;
- evidence class (`qualification` or `compatibility_fixture`);
- `qualification_id`, `capability_id`, `cell_id`, and `method_version`;
- repository-relative artifact path, byte size, and SHA-256;
- generation timestamp after the frozen spec;
- source-set and scenario-set SHA-256 values;
- build fingerprint; and
- Windows/architecture/CPU/core/memory fingerprint.

Roles and paths are unique. Native or completed migrations reject every `compatibility_fixture` receipt. All receipts for one specification agree on source-set digest, scenario-set digest, and build fingerprint. The scenario digest is calculated as SHA-256 of UTF-8 JSON with sorted keys, no insignificant whitespace, no NaN/Infinity, and this exact separator policy:

```python
json.dumps(value, ensure_ascii=False, allow_nan=False,
           sort_keys=True, separators=(",", ":"))
```

Strict verification resolves every artifact path inside the repository, rejects traversal, reads the exact bytes, and checks size and SHA-256. Receipt producers remain responsible for including the declared source descriptors and stage-specific measurements in the immutable artifact.

## Authoring and validation flow

1. Choose one authoritative Capability Registry V2 method cell.
2. Freeze estimands, preprocessing, model/data predicates, output IDs, and method version.
3. Select primary literature and computational oracle independence groups.
4. Define mandatory axes, complexity profiles, pairwise/targeted/compound combinations, and the Monte Carlo policy.
5. Assign one comparison rule and justified tolerance contract to every output.
6. Set hardware classes, absolute budgets, regression limits, archive versions, export projection, Windows matrix, and cancellation invariants.
7. Record `spec_frozen_at_utc` before generating evidence.
8. Generate independent stage receipts under one source set, scenario set, and build fingerprint.
9. Run non-strict validation while authoring.
10. Run strict validation with the exact registry document before proposing a cell-state update.

Commands:

```powershell
python validation/qualification_spec_v2.py validate `
  validation/qualification_v2/fixtures/pls_algorithm_v1.migrated.json

python validation/qualification_spec_v2.py validate `
  validation/qualification_v2/fixtures/pls_algorithm_v1.migrated.json `
  --registry validation/qualification_v2/fixtures/capability_registry_v2.fixture.json `
  --strict

python validation/qualification_spec_v2.py adapt-v1 `
  validation/methods/pls_algorithm_v1.manifest.json

python validation/test_qualification_spec_v2.py
```

The strict fixture command exits non-zero by design because the fixture is `compatibility_only`, even though its linked registry and receipt bytes verify. That is the intended protection against accidental legacy promotion.

## Promotion checklist

A cell-state change is allowed only when all of these are true:

- migration is `native` or explicitly `completed`, with no unresolved items;
- the exact registry cell identity and method version verify;
- every estimand has primary literature and sufficient independent computational coverage, or an approved exception;
- every required axis, complexity profile, pairwise pair, targeted boundary, and compound scenario is present;
- every output has one valid comparison rule;
- all performance budgets, archive cases, export formats/read-backs, Windows cases, and cancellation invariants are covered;
- all eight ladder stages have immutable receipts generated after the freeze;
- every receipt descriptor identity, scenario hash, build fingerprint, artifact size, and SHA-256 verifies; and
- the strict report returns `qualification_ready: true`.

Passing the schema alone, passing legacy gates, or having polished UI copy is never sufficient evidence for a V2 capability claim.
