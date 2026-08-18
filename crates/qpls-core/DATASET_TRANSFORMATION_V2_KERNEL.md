# DatasetTransformationV2 Rust kernel

`dataset_transformation_v2` is the dormant, authoritative Rust calculation
kernel for non-destructive derived variables. It mirrors the
`src/domain/datasetTransformationsV2.ts` contract without exposing a runner,
project command, or user-interface entry point.

## Supported operations

- reverse scale;
- exact typed recode;
- two-column or column/constant arithmetic;
- row sum and mean with `propagate` or `available` missing-value behavior and
  an explicit minimum complete-variable count;
- numeric zero/one dummy creation;
- exact-value and inclusive/exclusive numeric-range group derivation.

Both preview and apply call the same row evaluator. Preview evaluates the full
resident batch for issue and missing-value counts, then returns a bounded row
window. Apply creates a new Arrow `Dataset`; the source `Dataset` is never
mutated and an existing column is never overwritten.

## Fail-closed boundaries

The kernel requires a raw dataset whose declared case and column counts match
the resident Arrow batch. Static and row-specific problems return
`DatasetTransformationIssueV2` values with stable codes, fields, and optional
zero-based row indexes. Unknown JSON fields, unsupported cell types, duplicate
or overlapping exact rules, row-level rule overlap, division by zero,
non-finite inputs/results, output-type coercion, and invalid lineage options are
rejected.

The public cell wire remains exactly finite number, string, or null. A recode
with Boolean metadata therefore stores physical numeric zero/one values rather
than widening the project-row wire to JSON booleans.

## Identity and repository compatibility

Specifications use sorted-key canonical JSON and SHA-256. Lineage binds the
source fingerprint, canonical specification hash, output dataset identity,
input/output columns, timestamp, and missing count. Output datasets use the
existing qpls-data `v2:<sha256>` Arrow/schema fingerprint envelope so they can
round-trip through `DatasetDescriptor`; the lineage records that exact
fingerprint.

Two representation constraints differ intentionally from the TypeScript
in-memory shape:

- Rust output identifiers must parse as UUIDs because `qpls_data::Dataset.id`
  is a UUID.
- Arrow columns have one physical type, so mixed numeric/text recode or group
  outputs fail with a typed issue instead of being silently coerced. Boolean
  recodes remain numeric zero/one at the cell boundary.

These constraints do not activate or alter the current project archive,
runner, CLI, or desktop command paths.
