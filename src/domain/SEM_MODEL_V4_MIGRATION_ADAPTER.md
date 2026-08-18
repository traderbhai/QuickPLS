# SemModelV4 current-graph migration adapter

`semModelV4MigrationAdapter.ts` is the staged TypeScript boundary between the
current QuickPLS project model and `SemModelV4`. It does not change the active
project archive version, rewrite a project, or recalculate historical results.

## Source authority

The adapter follows the existing native-project split:

- `NativeCanonicalModelSpec` supplies legacy scientific content.
- `NativeModelPresentation` supplies React Flow positions, routing, and the old
  covariance display arcs.
- `SemDataBindingV4` is supplied explicitly, or derived from the current
  `Dataset` descriptor with `currentDatasetToSemDataBindingV4`.

Matrix migration requires the study sample size explicitly. Dataset row count is
the matrix dimension and is never substituted for study N.

An explicitly supplied missing-data policy or matrix-moment payload is retained.
Estimator compilers still reject policies or metadata they do not yet implement;
migration does not silently replace them with listwise defaults.

The input is copied before validation. The migration artifact and all authoring
results are recursively frozen, so confirmation and covariance operations never
mutate the open project or their input artifact.

## Estimand migration

| Legacy method intent | Migration result |
|---|---|
| `pls_sem` | Reflective constructs become Mode A composites; formative constructs become Mode B composites. |
| `cbsem` | Reflective constructs become identified common factors. A formative construct prevents automatic conversion. |
| `method_neutral` or `mixed` | The artifact remains `legacy_estimand_unspecified`. |

`requireConfirmedSemModelV4`, `compileMigratedPlsPlanV2`, and
`compileMigratedCbsemPlanV2` reject an unspecified artifact. The only transition
out of that state is `confirmLegacyEstimandSemModelV4`, with an explicit
`pls_composite` or `cbsem_common_factor` choice.

Controls, interactions, and higher-order declarations are preserved in the
source snapshot but stop automatic conversion. Their future dedicated adapters
must define exact SemModelV4 semantics before they can compile.

## Stable identity and order

- The current model ID and dataset ID are unchanged.
- Construct and indicator variable IDs use the same deterministic namespaces as
  the Rust converter: `construct:<legacy-id>` and `observed:<source-column>`.
- Generated scientific covariance IDs are UTF-8 hex encodings of their stable
  authoring ID; no time or random value participates.
- The copied legacy model, presentation, construct order, indicator order, path
  order, and matrix-column order survive `roundTripCurrentQuickPlsGraphV4`.
- SemModelV4 scientific hashing remains order-independent through
  `scientificSemModelV4HashInput`.

## Covariance behavior

Legacy edges with `data.role === "covariance"` become
`display_only_covariance` annotations. They are absent from estimator plans and
from scientific hash input.

The APIs intentionally keep presentation and science separate:

- `authorPresentationCovarianceV4` adds only a copied React Flow edge and a
  display annotation.
- `authorScientificCovarianceV4` adds a scientific relation, parameter, and
  scientific presentation-edge reference.
- `convertPresentationCovarianceToScientificV4` adds the scientific relation
  and parameter while retaining the source edge, annotation, and stable lineage
  that links all four IDs. Repeating the same conversion is idempotent.

The old graph cannot represent the new distinction by itself. Round-trip output
therefore returns the exact legacy graph plus a separate
`scientific_covariances` collection. Live editor integration must adopt that
collection before a scientific covariance can be edited through the current
path inspector.

## Fail-closed checks

Migration rejects duplicate IDs, duplicate or missing indicator ownership,
unknown endpoints, self relations, duplicate paths or covariance pairs,
unrecognized edge roles, canonical/presentation control-role disagreement,
non-text covariance labels, invalid coordinates, unsupported data-binding
semantics, and matrix-variable mismatch.

Generated measurement edges are recognized as derived presentation objects and
do not override canonical measurement content.

## Deferred integration

- Wire the adapter into the schema-v6 upgrade-copy assistant after schema-v6 ZIP
  read/write is activated as one coordinated change.
- Store V4 scientific covariances distinctly in the live editor instead of
  reusing the old display-only `PathEdgeData.role` value.
- Add dedicated controls, interactions, and higher-order migration adapters.
- Bind recipe-schema-v4 compilation method by method.
- Keep schema-v1-to-v5 archives and their historical results unchanged until an
  explicit upgrade-copy action is implemented.
