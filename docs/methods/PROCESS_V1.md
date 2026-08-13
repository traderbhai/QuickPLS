# PROCESS_V1

Status: validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope.

`regression_process_v1` is a historical bounded PROCESS-style workflow implemented as generated regression recipes. It is retained for immutable archive compatibility and is not current QuickPLS 3.0 evidence. New graph-defined work uses `regression_process_v2`; see `PROCESS_V2.md`.

Schema-v3 execution rejects these historical relationship variants. Migrating a
schema-v2 v1 recipe returns an explicit archive-only error instead of silently
creating a newly executable v1 recipe. Historical result payloads remain
readable and displayable under their original version.

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

`npm run qpls:process:reference` checks the bounded effects against independent Python OLS equations and writes the method-specific `validation/results/v08_process_reference_report.json`. `npm run qpls:promotion:process` fails closed unless that exact current report is bound to `regression_process_v1`; evidence from another v0.8 section is not accepted. The promotion also verifies R base-lm component parity, mediation/moderation fixtures, product enforcement, and the moderated-mediation exclusion.
