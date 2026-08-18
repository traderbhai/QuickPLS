# Importance-Performance Map Analysis (IPMA) V1

`ipma_v1` is the Standard QuickPLS workflow for the documented predecessor-only IPMA scope. Its scientific and source-tier contracts are qualified; current-build packaged receipts are refreshed at the coordinated release gate. It is not cIPMA and does not implement the broader SmartPLS IPMA range-correction and representation options.

## Scope

- Available through `AnalysisMethod::Ipma`.
- Uses the ordinary standardized PLS-PM estimator.
- The Standard native workflow stores one explicit endogenous target per run. The lower-level compatibility representation remains a target list so historical recipes can deserialize, but simultaneous multi-target analysis is outside this promoted product scope.
- Construct importance is the total effect from each direct or indirect structural predecessor to the selected target. The target itself and unrelated constructs are excluded.
- Construct performance is the mean 0-100 min-max scaling of the construct's standardized score.
- Indicator performance is the mean 0-100 min-max scaling of the standardized indicator column for each included predecessor construct, reported with its loading and parent construct importance.

## Scope Boundaries

- This is a bounded IPMA workflow, not cIPMA.
- It uses observed complete-sample score ranges. It does not use variable-scale metadata or explicit theoretical minimum/maximum values, so it must not be presented as full SmartPLS IPMA parity.
- Case-weighted IPMA, generated interaction constructs, and higher-order constructs are blocked.
- Direct-predecessor-only views, alternative map representations, NCA/cIPMA integration, and resampling inference are unsupported.
- Empty, non-finite, or constant score ranges fail closed; QuickPLS never substitutes a midpoint for an undefined performance value.

## Validation Evidence

`npm run qpls:ipma:reference` writes `validation/results/ipma_reference_report.json`.

The fixture uses a transparent single-indicator mediated path model plus a disconnected `U -> V` component that must not appear for target `Y`. An independent Python reference standardizes the indicators, estimates structural OLS equations, identifies the direct and indirect predecessors of the selected target, decomposes total effects, computes observed-range 0-100 min-max performance, and compares the exact predecessor construct and indicator rows against QuickPLS within the `1e-6` deterministic gate. A focused Rust matrix additionally proves construct/path reorder invariance, exact listwise used/omitted counts, and typed fail-closed handling for empty, non-finite, and constant performance ranges.

`npm run qpls:promotion:ipma` additionally audits exact engine and project-persistence contracts, native setup/results/export source contracts, and packaged-Tauri evidence. The current packaged artifact visibly authors the reference model plus a disconnected `U -> V` negative-control branch, runs target `Y`, verifies that `Y`, `U`, `V`, `y1`, `u1`, and `v1` are absent from predecessor output, exports the three IPMA tables to XLSX, and restores the same validated run after save/reopen. The audit must return false whenever that genuine evidence is absent; source tests or synthetic completed-result fixtures are not substitutes.
