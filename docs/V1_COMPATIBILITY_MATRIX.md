# QuickPLS 1.0.0 Compatibility Matrix

This matrix is the reader-facing v1.0.0 scope summary. Detailed evidence is indexed in `docs/VALIDATION_ARTIFACT_INDEX_V1_0.md`.

Stable in v1.0.0 means validated only for the documented supported scope in this matrix and the linked method specifications.

| Area | v1.0.0 status | Supported scope | Key exclusions |
| --- | --- | --- | --- |
| Data/project platform | Validated | `.qpls` archives, local datasets, imports, metadata, autosave/recovery, migration, corruption checks | Cloud sync, account activation, remote computation |
| PLS-SEM core | Validated | Documented Mode A/B, weighting schemes, standardized run envelope, scores, loadings, weights, paths, effects, R² | SmartPLS project import, undocumented SmartPLS behavior matching |
| Assessment | Validated for documented scope | Reliability, validity, HTMT/HTMT+, VIF, R²/adjusted R², f², Q², selected fit diagnostics where specified | Unsupported metric conventions outside method docs |
| Inference/resampling | Validated for documented scope | Deterministic bootstrap, BCa, studentized, jackknife, permutation where audited | Unqualified stochastic claims outside covered settings |
| Extended PLS | Validated for documented bounded scope | Mediation, moderation, PLSc, WPLS, CCA, CTA-PLS, endogeneity, nonlinear effects, HOC bounded workflows | Broader estimator variants not covered by method specs |
| Prediction/heterogeneity | Validated for documented bounded scope | PLSpredict/CVPAT, IPMA, MGA/MICOM/permutation MGA, FIMIX, PLS-POS where audited | Unbounded segmentation claims and unsupported generated shapes |
| CB-SEM/CFA | Validated for bounded raw-data reflective ML scope | Single-group CFA/SEM ML fixtures, fit, residuals, modification indices, bootstrap/multigroup preview where audited | WLSMV, polychoric, FIML, ordinal claims, unsupported constraints |
| Extended methods | Validated for documented bounded scope | PCA, OLS/logistic regression, PROCESS-style workflows, NCA, bounded GSCA | Publication claims beyond bounded fixtures |
| SEM designer | Validated | Editable academic SEM canvas, persistent layout, draggable indicators, current-canvas SVG export | Residual/error and caption recipe semantics |
| Exports | Validated | CLI CSV/HTML/XLSX, desktop report CSV/HTML/XLSX, SVG diagram, browser print-to-PDF path | Native CLI PDF/PNG |
| Windows packaging | Validated unsigned | Release executable and NSIS installer | Signed installer unless certificate and signing audit are added |

Default behavior must not present excluded features as validated. Experimental outputs require explicit opt-in and visible watermarking.

