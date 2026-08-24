# QuickPLS MultiMod boundaries and qualification

Status: implementation candidate, Labs only, evidence absent.

This document is the product-level boundary for the additive MultiMod work. It
does not promote a method, replace a historical contract, or imply that the
desktop workflow is ready. Promotion is allowed only after the exact final
commit passes every profile-specific source, numerical, persistence, export,
native, packaged-Windows and offline gate declared in the source-bound
manifests under `validation/multimod/`.

## Protected behavior

- Continuous moderation V1 is a protected regression boundary. MultiMod does
  not change its recipe, estimator, serialization, labels or golden results.
- Quadratic/self-moderation is outside this MultiMod candidate and receives no
  recipe, runtime, result, UI, or capability claim from this work.
- Historical MGA, FIMIX/POS and moderated-mediation results keep their exact
  method identities and remain readable without reinterpretation.
- The project archive remains schema 6. MultiMod adds checked, typed sidecars
  to that generation and does not silently reinterpret an older archive.
- Python and R references are qualification tools only. The shipped Windows
  application remains self-contained and fully offline.
- A cancelled or failed sharded run may retain an external execution cache,
  but it must not publish a partial scientific result.

## Additive public identities

| Configuration | Result family | Method boundary |
|---|---|---|
| `MgaMultigroupV1` | `PlsMultigroupAnalysisV1` | Typed 2-20 group comparison envelope; every model profile and inferential procedure is qualified independently. |
| `PlsUnobservedHeterogeneityConfigV2` | `PlsHeterogeneityAnalysisV2` | Genuine FIMIX V2 and separately named PLS-POS V2 methods; V1 is unchanged. |
| `GeneralSemConditionalProcessConfigV2` | `GeneralSemConditionalProcessResultV2` | Explicit conditional indirect paths and bounded, non-Cartesian profiles; no causal wording. |
| `InterventionalCausalMediationConfigV1` | `InterventionalMediationResultV1` | Separate observed-data g-computation workflow with required identification declarations. |

The frozen scientific specifications are:

- `docs/methods/MGA_MULTIGROUP_V1.md`
- `docs/methods/PLS_HETEROGENEITY_V2.md`
- `docs/methods/GENERAL_SEM_CONDITIONAL_PROCESS_V2.md`
- `docs/methods/INTERVENTIONAL_CAUSAL_MEDIATION_V1.md`

## Qualification cells

Every row below is a distinct evidence identity. Passing one row cannot be
used as evidence for another row or for an unsupported intersection.

### MGA profiles

| Profile identity | Qualified only when independently evidenced |
|---|---|
| `mga.general_sem_pls.v1` | Ordinary recursive General SEM PLS structures admitted by the frozen compiler. |
| `mga.multiple_two_way_moderation.v1` | One or more TwoStage + Strong two-way terms, including path, gamma and fixed-probe slope contrasts. |
| `mga.bounded_three_way_moderation.v1` | Exactly one three-way term with complete main and pairwise lower-order closure. |
| `mga.bounded_two_way_moderated_mediation.v1` | The current one-path, one-interaction scientific shape only. |
| `mga.multiple_nonnested_hoc.v1` | Up to four pairwise-disjoint, nonnested second-order HOCs using one admitted approach per run. |
| `mga.case_weighted_pls.v1` | Finite strictly positive case weights. |
| `mga.frequency_weighted_pls.v1` | Positive integer frequencies with count-space equivalence to expanded rows. |
| `mga.reflective_plsc.v1` | Reflective PLSc with the full correction repeated inside every resample. |

For each MGA profile, every admitted procedure has a separate evidence cell.
Permutation, Henseler directional probability, bootstrap-BC, MICOM and
multiplicity procedures are profile-qualified independently. Pooled-variance,
Welch-Satterthwaite and K-group inverse-variance Wald procedures are limited to
ordinary General SEM structural/control paths because the other profiles do
not yet expose a qualified coefficient-variance contract. An omnibus
permutation claim is made only for the max-spread K-group statistic. MICOM
Steps 2-3 remain pairwise; there is no invented omnibus MICOM result.

### Heterogeneity profiles

| Profile identity | Method identity |
|---|---|
| `fimix.p0_structural.v2` | `qpls.fimix-pls.v2` |
| `fimix.p2_multi_two_way.v2` | `qpls.fimix-pls.v2` |
| `fimix.p23_all_current.v2` | `qpls.fimix-pls.v2` |
| `pos.published.p0_structural.v2` | `qpls.pls-pos.published.v2` |
| `pos.destination_scored.p2_multi_two_way.v2` | `qpls.pls-pos.destination-scored-interactions.v2` |
| `pos.destination_scored.p23_all_current.v2` | `qpls.pls-pos.destination-scored-interactions.v2` |
| `pos.common_metric.p2_multi_two_way.v1` | `qpls.pos-common-metric-comparability.v1` |
| `pos.common_metric.p23_all_current.v1` | `qpls.pos-common-metric-comparability.v1` |

Candidate K is inspected and explicitly locked. K=1 is a read-only pooled
baseline. Discovery is never repeated as model selection in bootstrap. A
failed POS common-metric gate preserves destination-local descriptive output
but suppresses between-segment gamma, delta, slope and effect tests.

### Conditional-process profiles

| Profile identity | Inference identity |
|---|---|
| `conditional.multi_two_way_percentile.v2` | Type-7 percentile, all predeclared alternatives. |
| `conditional.multi_two_way_bca.v2` | Full delete-one BCa, all predeclared alternatives. |
| `conditional.studentized.v2` | Nested studentized inference within its smaller structural and target limits. |
| `conditional.bounded_three_way_percentile.v2` | One bounded three-way closure, Type-7 percentile. |
| `conditional.multiple_hoc_percentile.v2` | Disjoint nonnested HOCs, Type-7 percentile, two-sided. |
| `conditional.grouped_percentile.v2` | 2-20 groups, stratified Type-7 percentile, two-sided. |
| `conditional.case_weighted_percentile.v2` | Positive case weights, Type-7 percentile, two-sided. |
| `conditional.frequency_weighted_percentile.v2` | Positive integer frequencies, count-space Type-7 percentile, two-sided. |

Every indirect path is selected explicitly. All requested targets share one
replicate ledger. A scalar index of moderated mediation is emitted only when
the selected-path polynomial is affine in exactly one moderator; otherwise
the result reports correctly named local derivatives and finite contrasts.

### Interventional mediation profile

`interventional.observed_gcomp.v1` is one separate cell. It accepts directly
observed variables, an explicit adjustment set, temporal-order and
identification declarations, a binary treatment or explicit continuous
`x0 -> x1` contrast, and recursive selected paths of two through four edges.
Its mandatory wording is “assumption-dependent interventional estimate”; it
never says that causality was established.

## Evidence ladder

Each profile/procedure cell starts with `surface=labs` and
`evidence_state=absent`. Promotion requires all of the following against one
exact candidate commit:

1. Frozen method specification, configuration/result schema and compiler
   receipt identity.
2. Hand calculation plus an independent Python or R reference where
   applicable.
3. Preregistered simulation, recovery, null and boundary reports.
4. Metamorphic invariance for row/order/group/class/sign/seed/worker/shard and
   cancel-resume transformations appropriate to the method.
5. Strict schema-6 save/reopen and missing, duplicate, mismatched or tampered
   Arrow-sidecar rejection.
6. Semantic CSV, XLSX, JSON, HTML, PDF and chart readback for the formats
   admitted by the final product contract.
7. Native accessibility, cancellation, recovery and maximum-profile evidence.
8. Isolated installed and portable Windows offline smoke evidence.
9. Continuous-moderation V1 golden and legacy archive/recipe/result regression
   evidence.
10. A live manifest whose source hashes and candidate commit match the final
    unmerged commit exactly.

Missing or stale evidence derives `absent`; a declarative manifest can never
promote itself. Registry state must not exceed the live derivation.

The frozen competitor catalogue in `capability_registry_v2.json` has exactly
45 official rows and conservatively projects a whole row to the least mature
option cell. Adding an absent MultiMod cell there would incorrectly downgrade
already release-qualified legacy MGA or mediation cells. Pending MultiMod work
is therefore registered additively in
`validation/multimod/multimod_capability_index_v1.json` and the six Labs cell
manifest templates. It must enter the customer registry only through a
separately reviewed registry-generation change that can preserve qualified V1
availability while keeping each V2 profile absent until its own evidence is
live.

## Campaign boundary

`validation/run_v256_multimod_qualification.ps1` is a resumable campaign
driver draft. Until every plan gate is marked ready and bound to a real command,
the driver refuses execution and can only print its plan. The campaign writes
one issue inventory and never edits source. Foundational fixes invalidate all
dependent gates as recorded in the plan. Open upstream issues are resolved
through the transitive dependency graph, and invalidated gates are recorded as
blocked without being executed. Blocked gates are terminal for campaign
accounting but force a `completed_with_issues` result and a nonzero exit; only
`passed` gates with valid evidence can contribute to release acceptance.
Qualification does not merge, push, tag,
publish or replace public installer artifacts.
