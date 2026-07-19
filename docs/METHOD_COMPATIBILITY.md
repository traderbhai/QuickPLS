# Method Compatibility

| Family | Method | Foundation status | Stable output |
| --- | --- | --- | --- |
| PLS-SEM | PLS path modeling core | Validated v0.3 estimator scope | Not yet as a full run envelope |
| PLS-SEM | Bootstrapping | Experimental v0.4 inference slice | No |
| PLS-SEM | Mediation effect decomposition | Experimental v0.5 descriptive plus bootstrap-inference surfacing slice | No |
| PLS-SEM | Two-stage moderation | Experimental v0.5 estimator with independent and R base-lm single-item reference/metamorphic evidence | No |
| PLS-SEM | Control paths | Experimental v0.5 schema/output semantics; uses ordinary structural path estimation | No |
| PLS-SEM | Higher-order constructs | Experimental v0.5 recipe/editor schema with repeated-indicator, two-stage, and hybrid indicator-split estimators plus independent Python and metamorphic evidence; invalid hybrid component splits are explicitly blocked | No |
| PLS-SEM | PLSc | Experimental reflective-only correction contract with rho_A attenuation-corrected construct correlations, paths, outer loadings, and R2; `npm run qpls:plsc:validate` passes independent Python reference and invalid-settings guards | No |
| PLS-SEM | WPLS | Experimental positive case-weight estimator with weighted standardization, weighted covariance iteration, weighted least-squares paths, and independent Python reference/guard evidence | No |
| PLS-SEM | CCA | Experimental descriptive composite correlation residual diagnostic for recursive standardized path models, with independent Python reference and invalid-settings guard evidence | No |
| PLS-SEM | CTA-PLS | Experimental descriptive tetrad diagnostic with sample-covariance tetrads for indicator blocks of four or more indicators; `npm run qpls:cta:reference` passes independent Python reference and invalid-block guards | No |
| PLS-SEM | Endogeneity analysis | Experimental Gaussian-copula diagnostic with rankit inverse-normal copula terms, augmented regressions, approximate t/p diagnostics, applicability warnings, and independent Python reference evidence | No |
| PLS-SEM | Nonlinear effects | Experimental fixed-score quadratic diagnostic with centered squared construct-score terms, augmented-regression statistics, R2 delta, warnings, and independent Python reference evidence | No |
| PLS-SEM | Moderated mediation | Experimental two-stage conditional indirect-effect diagnostic for first-stage and second-stage interaction-mediated paths, with invalid-recipe guard and independent Python reference evidence | No |
| Prediction | PLSpredict deterministic holdout, repeated k-fold, and CVPAT diagnostic | Experimental v0.6 leak-free 75/25 complete-case holdout plus repeated 5x3 k-fold prediction with train-only preprocessing, training-split PLS weights/paths, construct-score LM benchmarks, Q2 predict, RMSE, MAE, paired squared-loss benchmark CVPAT diagnostics, and metadata-configured drop-path model-pair CVPAT; separate saved-model CVPAT remains unsupported | No |
| Prediction | IPMA / cIPMA | Experimental v0.6 bounded importance-performance slice using PLS total effects and 0-100 min-max performance from standardized construct/indicator scores; independent Python reference agrees within `1e-6` | No |
| Groups | PLS-POS | Experimental v0.6 generalized 2-5 segment deterministic preview via metadata `segment_count`; backward-compatible `pls_pos_bounded_v1` remains readable; recovery fixtures pass, but publication-ready PLS-POS remains unvalidated | No |
| Groups | FIMIX-PLS | Experimental v0.6 2-3 class deterministic finite-mixture style preview with class probabilities, memberships, class paths/R2, information criteria, and entropy; recovery fixture passes, but publication-ready FIMIX-PLS remains unvalidated | No |
| Groups | Two-group MGA | Experimental v0.6 observed two-group path comparison plus optional permutation MGA via metadata `mga_group_column` and `group_methods`; independent bounded reference and integrated permutation fixture pass | No |
| Groups | MICOM | Experimental v0.6 two-group construct-score permutation preview with configural/compositional/mean/variance steps; integrated fixture passes, but publication-ready measurement-invariance testing remains unvalidated | No |
| CB-SEM | CFA and maximum-likelihood SEM | Experimental v0.7.1 beta with direct single-group ML optimization and lavaan parity fixtures for bounded raw-data reflective CFA/SEM models; bootstrap, multigroup/invariance, robust/ordinal/FIML estimators, and broad publication validation remain pending | No |
| Components | Standalone PCA | Experimental v0.8 standardized raw-data eigensystem workflow with independent NumPy fixture evidence | No |
| Components | GSCA | Experimental v0.8 bounded component-model payload with GSCA-style fit diagnostics; full ALS publication validation remains pending | No |
| Regression | OLS, logistic, and PROCESS | Experimental v0.8 standalone OLS/logistic engines plus bounded PROCESS-style mediation/moderation workflow; independent Python fixtures cover current preview scope | No |
| NCA | CE-FDH and CR-FDH | Experimental v0.8 numeric X/Y NCA preview with deterministic permutation p values and bottleneck tables; independent Python CE-FDH fixture evidence | No |

Publication audit update: `publication_ready_v0_1_to_v0_8` now clears for the documented supported scope. Rows that still say experimental describe broader or unsupported behavior outside that audited scope; they do not override the method-specific publication audit artifacts.

The desktop catalog and Rust core share this policy. A method name in the product roadmap is not evidence of implementation or validation outside its documented scope.

Detailed release gates and next actions are machine-readable in `D:\QuickPLS\validation\development_slices.json` and available with:

```powershell
cargo run -p qpls-cli -- roadmap
cargo run -p qpls-cli -- gate v0_4_assessment_reliability
```
