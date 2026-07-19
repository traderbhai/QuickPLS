# PROCESS_V1

Status: experimental v0.8 preview.

`regression_process_v1` is a bounded PROCESS-style workflow implemented as generated regression recipes.

## Contract

- Metadata selects `regression_type = process`.
- Supported preview models:
  - `process_model = mediation`
  - `process_model = moderation`
  - `process_model = moderated_mediation` as bounded role output where roles are supplied
- Mediation reports direct, indirect, and total effects from component OLS regressions.
- Moderation reports product-term effects and simple slopes for bounded moderator values.
- Output includes effect rows, simple slopes, component-regression coefficients, warnings, and `method_version = regression_process_v1`.

## Unsupported In v0.8

- Full Hayes PROCESS model catalogue.
- Complex serial/parallel mediation families.
- Johnson-Neyman publication claims.
- Bootstrap CI promotion; current effect intervals remain unavailable unless separately qualified.

## Validation

`npm run qpls:process:reference` checks the mediation indirect effect against independent Python OLS equations. Later promotion requires a full model-number compatibility matrix, bootstrap coverage, and R/base regression parity for component models.
