# QuickPLS v0.1-v0.8 Publication-Readiness Audit

Status: promotion audit clear for the documented v0.1-v0.8 supported scope.

This document defines the hardening gate that must pass before any v0.1-v0.8 method family is marked publication-ready. The audit is intentionally stricter than the preview gates. A preview gate proves that a method is implemented and bounded evidence exists. A publication gate proves that the output is suitable for unrestricted research use.

## Command

```powershell
npm run qpls:publication-ready:v01-v08
```

This writes `validation/results/publication_ready_audit.json` and prints the registry gate `publication_ready_v0_1_to_v0_8`.

The focused blocker-1-through-5 audit can be regenerated with:

```powershell
npm run qpls:publication:v01-v04
```

All publication audit artifacts can be regenerated with:

```powershell
npm run qpls:publication:all
```

## Required Validation Runtime

R and Rscript are validation-only dependencies, never QuickPLS runtime dependencies.

Preferred Rscript path:

```text
C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\x64\Rscript.exe
```

The validation environment should record available package versions for `lavaan`, `cSEM`, `seminr`, `plspm`, `boot`, `sandwich`, `lmtest`, and optional `NCA`.

Current audit artifacts:

- `validation/results/publication_promotion_matrix.json`
- `validation/results/r_validation_runtime_audit.json`
- `validation/results/foundation_publication_audit.json`
- `validation/results/data_project_publication_audit.json`
- `validation/results/pls_publication_audit.json`
- `validation/results/pls_publication_bounded_benchmark.json`
- `validation/results/assessment_publication_metric_matrix.json`
- `validation/results/assessment_publication_audit.json`
- `validation/results/inference_publication_matrix.json`
- `validation/results/inference_publication_audit.json`
- `validation/results/extended_pls_publication_audit.json`
- `validation/results/prediction_heterogeneity_publication_audit.json`
- `validation/results/cbsem_publication_audit.json`
- `validation/results/extended_methods_publication_audit.json`
- `validation/results/gui_diagram_publication_audit.json`
- `validation/results/stable_export_publication_audit.json`
- `validation/results/documentation_publication_audit.json`
- `validation/results/performance_release_publication_audit.json`
- `validation/results/v09_smoke_check.json`
- `validation/results/v09_release_candidate_audit.json`
- `docs/RELEASE_NOTES_V0_9_RC1.md`
- `docs/SUPPORTED_SCOPE_V0_9_RC1.md`
- `docs/DEPENDENCY_NOTICES.md`
- `docs/KNOWN_DIFFERENCES.md`

Current required R packages available: `lavaan`, `cSEM`, `seminr`, `plspm`, `boot`, `sandwich`, and `lmtest`.

## Promotion Standard

A method can be promoted only when all applicable items are complete:

- Frozen method specification with equations, defaults, preprocessing, convergence, missing-data behavior, warnings, citations, and unsupported cases.
- Independent hand/Python fixtures for formula-level checks.
- At least two reference sources where feasible.
- Published worked examples where available.
- Deterministic agreement within `1e-6` when conventions match.
- Displayed-table agreement at four decimals when conventions match.
- Stochastic procedures checked for fixed-seed reproducibility, worker-count invariance, coverage, type-I error, power, and bias.
- Edge cases for missing, constant, collinear, singular, nonnormal, small-sample, high-dimensional, unidentified, nonconvergent, and inadmissible data.
- GUI and CLI produce identical serialized result payloads for the same recipe.
- Stable exports include correct precision, labels, warnings, citations, settings, random seed, engine version, and reproducibility appendix.
- Known differences from SmartPLS and reference engines are documented.
- Performance evidence is recorded on the target Windows machine.

## Closed Blocker Audits

- v0.1 foundation offline launch, recovery, local-only execution, and dependency audit.
- v0.2 data/project stress import, malformed input, archive migration, and recovery.
- v0.3 PLS core stable run envelope, reference examples, benchmark slice, and known-differences register.
- v0.4 assessment publication matrix, reference evidence aggregation, export warning/precision check, and assessment tests.
- v0.4 inference matrix, Monte Carlo/reference aggregation, worker-invariance, release-stress, cancellation, and resampling tests.

## Additional Closed Blocker Audits

- v0.5 extended PLS method-by-method promotion for the documented supported scope.
- v0.6 prediction, heterogeneity, invariance, and segmentation promotion for the documented supported scope.
- v0.7 CB-SEM/CFA raw-data reflective ML promotion for the documented supported scope.
- v0.8 PCA, regression, PROCESS, NCA, and GSCA promotion for the documented supported scope.
- Production SEM diagram and result-diagram workflow qualification.
- Stable report/export qualification for current CSV/HTML/XLSX, SVG diagram, and browser print-to-PDF surfaces.
- Methodology manual, known-differences register, benchmark report, and release qualification.

## Scope Control

The audit does not permit claims outside the exact documented supported scope. Unsupported estimators, inputs, model shapes, export surfaces, or workflows remain blocked or explicitly experimental until a new method specification and audit artifact promote them.

The machine-readable source of truth is `validation/development_slices.json`.
