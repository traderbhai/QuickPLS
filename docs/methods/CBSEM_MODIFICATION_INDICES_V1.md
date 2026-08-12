# CB-SEM Residual-Based Modification Screening v1

`cbsem_modification_indices_v1` is validated as a diagnostic screening payload for the documented QuickPLS v1.2.4 raw-data single-group reflective CFA/SEM ML scope.

## Scope

- Emits candidate residual-covariance and latent-covariance/free-path screens from residual-correlation magnitudes.
- Filters self-pairs, duplicates, and already-free parameters represented by the bounded model metadata.
- Reports a deterministic screening index `n × residual_correlation²` and uses the residual correlation as an expected-change diagnostic.
- Native Results labels this output `Residual-based modification diagnostics` and states that it is screening evidence, not an instruction to alter the model.

## Unsupported

Score-test modification indices from the expected or observed information matrix are not implemented. The payload is a residual-based prioritization screen inside the documented v1.2.4 scope; exact score-test MI evidence remains unsupported.

## Validation

`npm run qpls:cbsem:mi-reference` checks non-empty candidate generation, duplicate filtering, and stable ordering through `validation/results/cbsem_v07_reference_report.json`. The focused packaged acceptance also verifies non-empty finite screening rows, exact `cbsem_modification_indices_v1` persistence, native XLSX export, and same-run reopen.
