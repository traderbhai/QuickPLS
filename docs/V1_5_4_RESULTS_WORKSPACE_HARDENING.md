# QuickPLS v1.5.4 Results Workspace Hardening

## Scope

This milestone hardens the Results workspace after the sample-run screen review. It is frontend/product work only.

No statistical engines, formulas, method-validation logic, analysis recipes, result schemas, project format, or numerical fingerprints are changed.

## Completed Changes

- Result tabs now show tab-specific content instead of repeating the same broad summary.
- Summary separates KPIs, path coefficients, and total effects.
- Measurement Model shows outer loadings/weights, cross-loadings, and a diagram-focus helper for structural paths.
- Structural Model shows path coefficients, total effects, R²/adjusted R², inner VIF, Cohen f², model-fit diagnostics, and blindfolding Q² where available.
- Reliability and Validity shows construct reliability, Fornell-Larcker, HTMT/HTMT+, and warnings with compact status notes.
- Inference shows bootstrap summaries when available and clear empty states when no inference run exists.
- Prediction, Groups, and Diagnostics use method-specific sections or clear unsupported/not-run states rather than generic payload dumps.
- Selecting a diagram path highlights related result rows in Summary, Measurement, and Structural tabs.
- Current-table export is scoped to the selected result tab.
- Result action controls stay visible while scrolling the workspace.
- Stale v1.0 wording and mojibake-prone labels are audited out of the Results source and smoke output.

## Evidence

- `validation/v154_results_workspace_smoke.mjs`
- `validation/v154_results_workspace_audit.py`
- `validation/v154_results_native_smoke.py`
- `validation/results/v154_results_workspace_smoke.json`
- `validation/results/v154_results_workspace_audit.json`
- `validation/results/v154_results_native_smoke.json`
- `validation/results/screens/v154/results-workspace/`

## Gate

```powershell
npm run qpls:v154:results-workspace
cargo run -p qpls-cli -- gate v1_5_4_results_workspace_hardening
```

## Boundaries

- Result values come from existing saved run payloads.
- Native PDF/PNG remain out of scope.
- Desktop Windows remains the target; mobile behavior is non-gating.
- Versioned desktop artifacts use version `1.5.4` and the `v1_5_4_results_workspace_hardening` label so prior installer/portable files are not overwritten.
