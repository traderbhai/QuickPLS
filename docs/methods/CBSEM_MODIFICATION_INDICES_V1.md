# CB-SEM Modification Indices v1

`cbsem_modification_indices_v1` is validated as a diagnostic screening payload for the documented QuickPLS v1.2.4 raw-data single-group reflective CFA/SEM ML scope.

## Scope

- Emits candidate residual-covariance and latent-covariance/free-path screens from residual-correlation magnitudes.
- Filters self-pairs, duplicates, and already-free parameters where the v0.7 model metadata makes that detectable.
- Reports modification index and expected-parameter-change diagnostic fields.

## Unsupported

Score-test modification indices from the exact expected or observed information matrix are not implemented in v0.7. The payload is a diagnostic prioritization screen inside the documented v0.9.0-rc.1 supported scope; exact score-test MI evidence remains unsupported.

## Validation

`npm run qpls:cbsem:mi-reference` checks non-empty candidate generation, duplicate filtering, and stable ordering through `validation/results/cbsem_v07_reference_report.json`.
