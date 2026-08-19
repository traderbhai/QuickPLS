# Capability Registry V2

Capability Registry V2 is the authoritative, fail-closed inventory for the 45
named entries in the SmartPLS 4 algorithms and techniques catalogue. It keeps
three questions separate:

1. **What does the official method cover?** The official row, lifecycle, URL,
   surfaces, and model/data predicates.
2. **How much of that documented breadth does each QuickPLS option cover?**
   Every `option_cells[]` entry owns its own `coverage_state`.
3. **What evidence has been accepted for that exact option?** Each cell owns
   its own `evidence_state` and exact qualification link.

A strong qualification result cannot promote a bounded implementation to full
coverage. The registry therefore prevents “release-qualified” from being
misread as “feature-equivalent.”

## Authoritative files

- Registry: `validation/capabilities/capability_registry_v2.json`
- JSON Schema: `validation/capabilities/capability_registry_v2.schema.json`
- Validator and APIs: `validation/capability_registry_v2.py`
- Active manifest evidence validator: `validation/method_promotion_manifest.py`
- Historical one-time reconciliation record: `validation/evidence_truth_reconciliation_v1.py` (not an active write path)
- Tests: `validation/test_capability_registry_v2.py`
- Rust runtime projection: `crates/qpls-core/src/capability_registry_v2.rs`

The official catalogue was reverified on 2026-08-14 against the
[SmartPLS algorithms and techniques catalogue](https://smartpls.com/documentation/algorithms-and-techniques/).
Every registry row carries its own official method URL.

## Frozen baseline

| Measure | Count |
| --- | ---: |
| Catalogue rows | 45 |
| Active official rows | 43 |
| `full` coverage | 0 |
| `partial` coverage | 32 |
| `absent` coverage | 11 |
| `intentionally_excluded` | 2 |

The 45 rows deliberately preserve both official PCA catalogue entries. The
active count excludes the two official legacy rows but never deletes them.
The consistent-bootstrap row is `partial` because a bounded full-PLSc-refit
implementation and native contract exist. Its linked QualificationSpec V2 is
valid but has zero accepted receipts; evidence remains `absent`, so the option
is still non-executable in the catalogue. The HTMT row remains `absent` even
though a bounded source foundation and valid QualificationSpec V2 now exist:
required inference breadth is incomplete and no qualification receipts have
been accepted.

The consistent-permutation row remains `absent`. A bounded Internal/Labs source
foundation now implements exact two-group reflective PLSc fixed-size label
reassignment, full group refits, selected corrected-parameter differences,
two-tailed inference, strict persistence/provenance, and deterministic no-retry
accounting. MICOM, one-tailed inference, broader parameter and model breadth,
independent-oracle and simulation evidence, and packaged acceptance remain
absent. Source existence therefore does not make this an executable catalogue
cell or parity claim. The bounded source anchors are
`crates/qpls-resampling/src/consistent_permutation.rs`,
`crates/qpls-core/src/validation.rs`, `crates/qpls-runner/src/lib.rs`,
`crates/qpls-project/src/lib.rs`, `crates/qpls-cli/src/main.rs`,
`src/native/nativeConsistentPermutation.ts`, its focused native test,
`docs/methods/CONSISTENT_PERMUTATION_V1.md`, and the deliberately absent-state
`validation/methods/consistent_permutation_v1.manifest.json`. The generated
legacy row keeps `implementation_evidence` empty because editable source paths
must never make an absent cell appear evidence-backed.

The model-fit row is also `partial`: its v2 point-fit contract reports SRMR,
`d_ULS`, `d_G`, Chi-square, degrees of freedom, and NFI for saturated and
estimated models. A separately selected bounded adapted Bollen-Stine source
lane now provides saturated and estimated fixed-index full-refit ledgers and
HI95/HI99 decisions. Its QualificationSpec V2 is valid but has zero accepted
receipts, so evidence remains `absent` and the catalogue/CLI option remains
non-executable. A transparent independent NumPy/SciPy full-refit PLS-PM oracle
and small generative/adversarial work matrices now exist, but they are
validation-only and are not admitted immutable receipts. A valid qualification
contract or work artifact is not qualification evidence.

The raw-data CB-SEM ML option and the duplicate CFA catalogue row are scoped
Standard under their exact point-estimator identity. The parent CB-SEM row stays
conservatively in Labs because its separately registered covariance and
scaled-correlation `CompiledCbsemPlanV2` option remains absent/Labs. Exact CFA
case bootstrap is a separate scoped Standard row/cell under
`cbsem_exact_case_bootstrap_v1`; historical `cbsem_bootstrap_v1` and schema-3
`cbsem_bootstrap_v2` remain readable but are not relabeled or promotional.

Evidence is a separate internal axis:

| Evidence state | Count |
| --- | ---: |
| `absent` | 16 |
| `engine_only` | 3 |
| `archive_qualified` | 0 |
| `native_qualified` | 0 |
| `release_qualified` | 26 |

These labels are governance data, not customer copy.

The authoritative inventory contains 50 option cells: 48 active and two
legacy. Their baseline coverage counts are 0 full, 37 partial, 11 absent, and
2 intentionally excluded. The different cell count is expected because the
sample-size/power, mediation, and permutation catalogue rows contain separately
governed option cells. The frozen 45-row counts above remain the conservative
compatibility projection used by historical reports.

Option-cell evidence is 16 absent, three engine-only, two archive-qualified,
zero native-qualified, and 29 release-qualified. Row evidence is 16 absent,
three engine-only, zero archive-qualified, zero native-qualified, and 26
release-qualified; the row projection remains conservative when independently
governed cells on one official row differ.

## Coverage and lifecycle rules

Each option cell's `coverage_state` has exactly four values:

- `full`: all documented settings, supported model/data shapes, calculations,
  result tables, diagnostics, exports, and relevant diagram behavior have
  accepted coverage evidence.
- `partial`: QuickPLS has a bounded implementation, but at least one documented
  official setting, shape, output, diagnostic, or workflow remains unmatched.
- `absent`: the active official method is not yet available as an accepted
  QuickPLS capability.
- `intentionally_excluded`: a recorded product/scientific decision excludes a
  legacy official method.

`official_lifecycle` remains a row property: `active` for 43 rows and `legacy` for the two
explicit exclusions. Lifecycle describes the official catalogue row; it is not
a QuickPLS readiness claim.

## Customer visibility rule

The registry contains the machine-readable policy used by
`resolve_customer_visibility()` for an exact `capability_id` + `cell_id`:

- **Standard** only when `surface == "standard"`, evidence is
  `release_qualified`, and coverage is either `full` or `partial` with a
  nonempty documented QuickPLS scope. Partial remains partial; Supported does
  not claim complete SmartPLS breadth.
- **Labs** requires `surface == "labs"`, executable evidence, non-absent
  coverage, and explicit opt-in. A malformed Standard cell fails closed and is
  not silently converted into a Labs option.
- **Legacy** and **internal** are non-customer surfaces. The two intentional
  exclusions use Legacy.
- `available == false` for `absent` and `intentionally_excluded` cells, so
  Labs routing never implies that a calculation can run.
- Evidence labels are `internal_only`; they must not appear in customer-facing
  setup, result, report, or export copy.

The current snapshot assigns bounded PLS-SEM Algorithm, reflective-only PLSc v2 point estimation, indexed-resampling PLS Bootstrap v4, bounded case-weighted WPLS, descriptive CCA composite-residual diagnostics, descriptive CTA-PLS sample-covariance tetrad diagnostics, predecessor-only observed-range IPMA, indicator-level PLSpredict/CVPAT v2, raw-data CB-SEM ML/CFA point estimation, exact-CFA case bootstrap, the two official PCA rows, GSCA ALS,
observed-variable NCA, OLS, binary logistic regression, regression
bootstrapping, graph-defined PROCESS v2, and its dedicated PROCESS bootstrap
plus the coupled exactly-two-group MICOM/permutation-MGA v4 workflow, the
single-model fixed-score Structural Path Randomization v1 workflow, the
one-HOC reflective-reflective disjoint two-stage point-estimate workflow, and
the inference-aware post-hoc technical minimum sample-size v2 cell and the
independently qualified bounded prospective sample-size/power v2 workflow to Standard
through release-qualified scoped option cells. Other active rows
stay in Labs and the two exclusions remain Legacy. GSCA Standard status does
not claim bootstrapping or inference; bounded PLS Standard status does not
claim cyclic or covariance-only models, prediction,
group comparison, PLSc, WPLS, CB-SEM inference outside the separately registered
exact-CFA case-bootstrap family, or SmartPLS numerical identity;
PLS Bootstrap Standard status is restricted to the registered indexed-resampling
v4 PLS-PM scope; regression bootstrap, PLSc bootstrap, broader null-distribution
permutation designs, and unsupported PLS shapes remain separate or excluded;
WPLS does not claim formative blocks, higher-order constructs, interactions,
PCA weighting, inference, or nonpositive/missing case weights;
PLSc v2 Standard status covers only reflective recursive point estimation with
at least two indicators per construct and path or factor weighting; formative
blocks, PCA weighting, generated interactions, HOCs, PLSc resampling/MGA, and
reinterpretation of historical `plsc_v1` output remain excluded;
CCA does not claim inferential CCA, resampled discrepancy tests, thresholds,
fit classification, formative models, or full SmartPLS CCA parity;
CTA-PLS does not claim bootstrap, permutation, or asymptotic inference,
vanishing-tetrad decisions, PCA weighting, blocks with fewer than four
indicators, or reflective/formative measurement classification;
IPMA does not claim simultaneous multiple targets, theoretical-range correction,
resampling inference, alternative representations, NCA integration, or cIPMA;
MICOM/permutation-MGA does not claim more than two simultaneous groups,
parametric, bootstrap, one-tailed, omnibus, or multiplicity-adjusted MGA,
consistent PLS, case weights, interactions, or higher-order constructs;
the Standard HOC workflow does not claim repeated-indicator, hybrid, formative,
nested, multiple-HOC, incoming-HOC-path, or HOC resampling workflows;
PLSpredict/CVPAT v2 covers reflective endogenous indicator targets and one fitted model assessed against IA and LM benchmarks; saved competing-model comparison, formative targets, time-series validation, grouped leakage controls, and broader model shapes remain excluded;
NCA does not claim multiple conditions,
latent scores, cIPMA, or theoretical-range workflows; OLS and logistic do not
claim categorical helpers, weights, clusters, mixed/panel models,
multinomial/ordinal outcomes, penalization, or Firth variants. Regression
bootstrap remains fixed to the documented OLS/binary-logistic case-resampling
contract; studentized intervals, custom alpha/tails, and broader regression
families remain excluded. PROCESS does not claim binary equations, weighted or
clustered inference, copied templates, studentized intervals, custom tails or
alpha, or unrestricted catalogue breadth.
Exact-CFA case-bootstrap Standard status is restricted to raw continuous,
single-group reflective CFA with listwise deletion and the documented indexed
no-retry percentile Type-7, analytic-studentized Type-7, or complete-delete-one
BCa Type-7 contracts. Recursive structural SEM, multigroup, ordinal/WLSMV,
robust/FIML, weighted, clustered, multilevel, parametric, and Bollen-Stine
bootstrap remain excluded.

## Qualification-link contract

Every qualification identity has exactly this shape and no additional fields:

```json
{
  "registry_schema_version": 2,
  "capability_id": "smartpls.pls_algorithm",
  "cell_id": "qpls3.pls.algorithm",
  "capability_version": "pls_pm_v1"
}
```

The deterministic identity is the four-value tuple in that order. The validator
cross-checks:

- schema version and exact keys;
- capability ID against the containing registry row;
- cell IDs against the frozen legacy crosswalk (with explicit exclusion cells);
- local manifest `feature.id` and `feature.method_version`;
- the current source-bound state re-derived by
  `method_promotion_manifest.py`; an invalid linked manifest or a registry
  evidence state above that derivation fails closed;
- duplicate identities; and
- external qualification-link sets supplied to the API.

The sample-size catalogue row intentionally contains two distinct frozen
cells: `qpls3.pls.posthoc_technical_minimum_sample_size` is an inference-aware
result linked to a PLS algorithm run, while `qpls3.pls.sample_size_power` is the
separate prospective Monte Carlo workflow. The former has a cell-identity
manifest under `validation/capabilities/` and the latter is governed by the
active `pls_sample_size_power_v2` method-promotion manifest. Both are scoped
Standard with independently release-qualified evidence. The post-hoc v2 result
selects the smallest statistically significant path from complete
linked two-sided PLS bootstrap probabilities and returns a corrective typed
unavailable state for point-only runs. The prospective v2 workflow instead
simulates exactly two ordinary reflective constructs joined by one path under
its declared bounded Gaussian/no-missing design; it does not infer power from
the observed dataset. Historical v1 results remain readable under their v1
identity and are never relabelled as v2.

The official page does not specify the
significance-test source/configuration or no-significant-path behavior, so the
documented QuickPLS scope deliberately freezes complete two-sided
normal-reference bootstrap inference and does not claim SmartPLS numerical
identity. Compact evidence covers a 24-scenario Python-versus-base-R matrix and
one-worker/four-worker linked-bootstrap invariance; no separate maximum-axis or
soak claim is made. One cell can never promote the other; the parent row is
partial, release-qualified, and Standard only because both exact bounded cells
independently satisfy that surface gate.

The August 2026 evidence-truth reconciliation removed stale active pointers
from 21 non-MICOM manifests while preserving their historical reports on disk.
Moderation, mediation, and higher-order models reran only their retained engine
and archive gates into new non-overwriting evidence directories; no native or
release state was promoted. Legacy `implementation_evidence` lists are empty
for absent row projections so source existence cannot be mistaken for accepted
qualification evidence.

Qualification references may include both the compatibility V1 manifest and a
native QualificationSpec V2 file. The option's four-field qualification link
does not change when that V2 contract is added. Registry validation requires
every reference to remain repository-contained, while method-specific V2 tests
must additionally prove that the document resolves to the same exact cell.
Contract validity (`passed: true`) and registry resolution do not imply
`qualification_ready: true`; immutable receipts and a completed migration are
still required.

Available lookup APIs:

```python
from validation.capability_registry_v2 import (
    build_qualification_link_index,
    cross_validate_qualification_links,
    load_registry,
    lookup_capability,
    lookup_option_cell,
    lookup_qualification_link,
    resolve_customer_visibility,
)

registry = load_registry()
row = lookup_capability(registry, "smartpls.pls_algorithm")
cell = lookup_option_cell(registry, row["capability_id"], "qpls3.pls.algorithm")
visibility = resolve_customer_visibility(registry, row, cell["cell_id"])
resolved = lookup_qualification_link(registry, row["qualification_links"][0])
report = cross_validate_qualification_links(registry, external_links)
```

## Surface and applicability fields

Every option cell's required scalar `surface` is one of `standard`, `labs`,
`legacy`, or `internal`. The row-level scalar is only the conservative
compatibility projection. The separate `product_areas` array declares what parity work
must assess: `diagram`, `data`, `settings`, `calculation`, `results`,
and `reporting`. Listing a product area is an assessment obligation, not a
completeness claim.

Each cell owns `supported_model_predicate`, `supported_data_predicate`,
`settings_schema`, `result_schema`, `documentation_reference`,
`qualification_spec`, and `known_differences`. Its qualification specification
contains exactly one four-field link matching the cell identity. The retained
row predicates/references and qualification arrays are checked compatibility
projections; the historical row `scope_statement` is likewise compatibility
copy and cannot override a cell. Identity/state drift from the authoritative
cells is a validation failure.

## Explicit legacy exclusions

- **Blindfolding** remains a visible `intentionally_excluded` row. The
  [official Blindfolding page](https://smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/blindfolding/)
  calls it a legacy procedure superseded by PLSpredict and CVPAT. QuickPLS keeps
  the legacy implementation evidence addressable but does not promote it into
  the active product surface.
- **Goodness of Fit (GoF)** remains a visible `intentionally_excluded` row. The
  [official GoF page](https://smartpls.com/documentation/algorithms-and-techniques/validity-and-model-fit/goodness-of-fit/)
  describes its limitations, while the official catalogue advises against
  relying on it. QuickPLS routes users to method-appropriate model assessment
  instead.

Removing these rows, silently calling them absent, or treating them as active
would fail validation.

## Generated legacy catalogue projection

The schema-v1 competitor catalogue is a generated compatibility report, not a
second source of truth. V2 freezes its header and stores one `legacy_row`
projection per capability. The
projection derives legacy status from the conservative aggregate of the
authoritative option cells: `engine_only`
and `archive_qualified` map to `engine-preview`; active `absent` maps to
`absent`; and `intentionally_excluded` maps to `deferred`. This makes the
old file reproducible without making its status field authoritative.

APIs:

- `generate_legacy_catalogue(registry)` returns the full legacy document in
  memory.
- `check_legacy_catalogue(registry, legacy)` compares metadata, every ordered
  row, and canonical SHA-256 digests.
- `write_generated_legacy_catalogue(registry, path)` uses an atomic replacement
  and verifies the materialized report by reading it back.
- `--print-legacy` prints the projection to stdout.
- `--write-legacy [PATH]` validates Registry V2 first and writes only after that
  validation succeeds. Omitting `PATH` targets the checked-in compatibility
  report.

Run the focused gate:

```powershell
python validation\method_promotion_manifest.py
python validation\capability_registry_v2.py --check-legacy
python validation\capability_registry_v2.py --write-legacy --check-legacy
python -m unittest validation.test_capability_registry_v2 validation.test_method_promotion_manifest
```

Manifest validation and `--check-legacy` verification never write. The old
`evidence_truth_reconciliation_v1.py --write` workflow is retired because its
frozen pre-promotion write set cannot govern current identities. Legacy
materialization remains an explicit maintenance action; a difference remains a
failure until the generated changes are reviewed and committed.

## Runtime and CLI projection

The Rust core embeds the same registry JSON at compile time and validates the
registry identity, ordered catalogue rows, option-cell ownership and identity,
conservative row projection, legacy exclusions, and the exact Standard gate.
The legacy `METHOD_CAPABILITIES` table remains only as a transitional internal
execution dispatch table while recipe-v4 cutover is staged; it is not a
coverage or customer-availability authority.

`qpls methods` now reads the embedded Registry V2 projection. Its text and JSON
forms report each exact option cell's coverage, evidence, surface, and actual
customer availability. It never converts the old execution label `Validated`
into a parity claim. The JSON form includes the embedded source SHA-256 so a
diagnostic can be tied to the registry bytes compiled into that executable.

```powershell
target\debug\qpls.exe methods
target\debug\qpls.exe methods --json
```

## Change protocol

Any capability change must update the registry, schema if the contract changes,
and focused tests in one reviewed change. The gate must confirm:

1. exactly 45 ordered rows and 43 active rows;
2. the declared coverage counts;
3. exactly Blindfolding and GoF as intentional exclusions;
4. exact option-cell identities, states, qualification links, and manifest versions;
5. official SmartPLS URLs and nonempty settings/result references;
6. repository-contained qualification references;
7. customer visibility behavior; and
8. deterministic legacy projection compatibility.

A promotion to `full` requires surface-by-surface evidence and known-difference
closure. Evidence state alone is never sufficient.
