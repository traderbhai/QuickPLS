# QuickPLS v1.5.6 Result-Specific Interpretation Engine

## Scope

v1.5.6 adds a deterministic, offline, frontend-only interpretation layer for existing QuickPLS results. It does not change statistical engines, formulas, result schemas, project format, validation tolerances, or numerical fingerprints.

## What Changed

- Added `src/domain/resultInterpretation.ts`.
- Results tabs now show value-specific finding cards before the numeric tables.
- The Interpretation tab now groups findings into:
  - Must address before reporting
  - Recommended checks
  - Optional advanced checks
  - Report wording
- Selected result rows now open row-level explanations that use the exact row values.
- The Report workspace adds an explicit `Include interpretation notes` option for HTML/print reports.
- Default CSV/XLSX exports remain numeric and do not include interpretation notes unless a report workflow explicitly opts in.

## Deterministic Rules

- PLS paths are ranked by absolute coefficient and flagged when negative or near zero.
- Reflective loadings use common `.708`, `.40-.708`, and `< .40` guides.
- Reliability uses `.70` guidance for alpha/rho_C and `.50` for AVE.
- HTMT uses `.85-.90` caution and `>= .90` issue guidance.
- Cross-loading findings are raised when an indicator loads higher outside its assigned construct.
- VIF uses `>= 3.3` caution and `>= 5` issue guidance.
- f2 uses `.02/.15/.35` small/medium/large guidance.
- Q2 above zero is treated as directional predictive relevance, not proof of model quality.
- Missing bootstrap/permutation outputs explicitly block p-value and confidence-interval claims.

## SEM Diagram Advisor

The advisor inspects the current model shape and result state to suggest relevant checks:

- mediation-like chains;
- interaction constructs;
- formative measurement blocks;
- targets with multiple predictors;
- missing bootstrap for structural paths;
- optional PLSpredict when prediction is a research objective.

## Evidence

- `src/domain/resultInterpretation.test.ts`
- `validation/v156_result_interpretation_smoke.mjs`
- `validation/v156_result_interpretation_audit.py`
- `validation/results/v156_result_interpretation_smoke.json`
- `validation/results/v156_result_interpretation_audit.json`

## Gate

```powershell
cargo run -p qpls-cli -- gate v1_5_6_result_specific_interpretation_engine
```

The gate passes only when the source contracts, smoke evidence, documentation, registry entry, and roadmap current-stage expectation are all present.
