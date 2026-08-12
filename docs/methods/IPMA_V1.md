# Importance-Performance Map Analysis (IPMA) V1

`ipma_v1` is validated for the documented QuickPLS bounded IPMA scope. Native-workflow promotion includes a current packaged-Tauri run, XLSX export, explicit save, and same-run reopen artifact. It is not cIPMA and does not implement the broader SmartPLS IPMA range-correction and representation options.

## Scope

- Available through `AnalysisMethod::Ipma`.
- Uses the ordinary standardized PLS-PM estimator.
- Target constructs are read from recipe metadata `ipma_targets` or `ipma.targets` as a comma-separated construct-id list; each selected target must be endogenous. The native workbench selects one explicit target per run.
- If no target metadata is supplied through a non-native contract, every endogenous construct is used as a target.
- Construct importance is the total effect from each direct or indirect structural predecessor to the selected target. The target itself and unrelated constructs are excluded.
- Construct performance is the mean 0-100 min-max scaling of the construct's standardized score.
- Indicator performance is the mean 0-100 min-max scaling of the standardized indicator column for each included predecessor construct, reported with its loading and parent construct importance.

## Current Limitations

- This is a bounded IPMA workflow, not cIPMA.
- It uses observed complete-sample score ranges. It does not use variable-scale metadata or explicit theoretical minimum/maximum values, so it must not be presented as full SmartPLS IPMA parity.
- Case-weighted IPMA, generated interaction constructs, and higher-order constructs are blocked in this preview.
- Direct-predecessor-only views, alternative map representations, NCA/cIPMA integration, and resampling inference are unsupported.
- Output is validated only for the documented fixed-total-effect importance and 0-100 standardized-score performance scope.

## Validation Evidence

`npm run qpls:ipma:reference` writes `validation/results/ipma_reference_report.json`.

The fixture uses a transparent single-indicator mediated path model plus a disconnected `U -> V` component that must not appear for target `Y`. An independent Python reference standardizes the indicators, estimates structural OLS equations, identifies the direct and indirect predecessors of the selected target, decomposes total effects, computes observed-range 0-100 min-max performance, and compares the exact predecessor construct and indicator rows against QuickPLS within the `1e-6` deterministic gate.

`npm run qpls:promotion:ipma` additionally audits exact engine and project-persistence contracts, native setup/results/export source contracts, and packaged-Tauri evidence. The current packaged artifact visibly authors the reference model plus a disconnected `U -> V` negative-control branch, runs target `Y`, verifies that `Y`, `U`, `V`, `y1`, `u1`, and `v1` are absent from predecessor output, exports the three IPMA tables to XLSX, and restores the same validated run after save/reopen. The audit must return false whenever that genuine evidence is absent; source tests or synthetic completed-result fixtures are not substitutes.
