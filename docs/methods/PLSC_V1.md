# PLSc v1

Status: experimental; reflective-only attenuation correction is implemented and validation-gated.

`AnalysisMethod::Plsc` and the catalog id `plsc` run through the ordinary PLS estimator first, then apply a consistent PLS correction contract for reflective constructs. The current implementation reports `method_version = "plsc_v1"` and stores a typed `plsc` payload with rho_A reliabilities, original and corrected construct correlations, corrected paths, corrected outer loadings, corrected R2, and warnings.

Supported in this preview:

- reflective constructs only;
- path and factor weighting only;
- at least two indicators per construct;
- deterministic corrected construct correlations using rho_A attenuation correction;
- corrected structural paths from the corrected predictor correlation system;
- corrected R2 clamped to the valid reporting range;
- corrected outer loadings clamped to `[-1, 1]`.

Unsupported settings are blocked before estimation. `settings.weighting_scheme = "pca"` emits `plsc.pca_unsupported`, and any formative construct emits `plsc.reflective_only`.

Validation evidence:

- `npm run qpls:plsc:reference` writes `validation/results/plsc_reference_report.json` and compares QuickPLS against an independent Python PLS + PLSc correction fixture within `1e-6`; the current observed max delta is `4.57e-14`.
- `npm run qpls:plsc:unsupported-guard` writes `validation/results/plsc_unsupported_guard_report.json` and proves unsupported formative PLSc recipes are rejected before execution.

Publication status: validated for the documented QuickPLS v0.9.0-rc.1 supported PLSc scope. Broader estimator shapes, inadmissibility diagnostics, and unsupported settings remain outside scope until a later audit expands this contract.
