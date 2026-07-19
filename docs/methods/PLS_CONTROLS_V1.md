# PLS Controls v1

Status: experimental schema and reporting slice.

Method version: `pls_controls_v1`

## Scope

Control variables are represented as structural paths with additional semantics. This preserves the current PLS estimator: coefficients, R2, effects, VIF, bootstrap, and permutation use the same path equations as before. The added `ModelSpec.controls` array tells QuickPLS which paths are controls so the recipe, result, and UI can label them distinctly.

Each control declaration contains:

- `source`: control construct id
- `target`: endogenous construct id
- `label`: optional reader-facing label

The corresponding `source -> target` structural path must exist in `ModelSpec.paths`.

## Validation

`qpls-core` validates:

- duplicate control declarations
- self-controls
- unknown source or target constructs
- missing corresponding structural path
- experimental warning `method.controls.experimental`

Older recipes without `controls` deserialize with an empty array.

## Output

`qpls-estimation` emits `control_estimates`, each copied from the matching structural path coefficient. This makes report/UI separation explicit without changing numerical estimation.

## Current Evidence

- `qpls-core::controls_are_schema_validated_and_must_map_to_paths` covers schema errors and the experimental warning.
- `qpls-estimation::control_estimates_mirror_declared_structural_paths` verifies exact coefficient equality between a declared control and the corresponding path estimate.
- The path inspector can mark a selected path as a control, stores that role on the edge, and the desktop run serializer writes those edges into `ModelSpec.controls`.
- `src/store.test.ts` covers undoable control edge metadata.

## Limitations

Control paths use the same estimator and inference artifacts as ordinary structural paths. Publication surfaces must keep the experimental label until the broader v0.5 gate is promoted.
