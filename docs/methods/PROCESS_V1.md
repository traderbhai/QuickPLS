# PROCESS_V1

Status: validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope.

`regression_process_v1` is a bounded PROCESS-style workflow implemented as generated regression recipes.

## Contract

- Metadata selects `regression_type = process`.
- Supported validated models:
  - `process_model = mediation`
  - `process_model = moderation`
- Mediation reports direct, indirect, and total effects from component OLS regressions.
- Moderation reports product-term effects and simple slopes for bounded moderator values.
- Output includes effect rows, simple slopes, component-regression coefficients, warnings, and `method_version = regression_process_v1`.

## Unsupported

- Full Hayes PROCESS model catalogue.
- Complex serial/parallel mediation families.
- Johnson-Neyman publication claims.
- Bootstrap CI promotion; current effect intervals remain unavailable unless separately qualified.
- `process_model = moderated_mediation`, which remains experimental until a dedicated promotion gate qualifies conditional indirect effects.

## Validation

`npm run qpls:process:reference` checks the bounded effects against independent Python OLS equations. `npm run qpls:promotion:process` verifies Python equations, R base-lm component parity, mediation/moderation fixtures, product enforcement, and the moderated-mediation exclusion.
