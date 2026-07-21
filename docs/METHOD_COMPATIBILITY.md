# Method Compatibility

| Family | Method | Foundation status | Stable output |
| --- | --- | --- | --- |
| PLS-SEM | PLS path modeling core | Estimator-only output validated for the documented v0.3/v1.2 scope; full PLS run-envelope status depends on assessment and inference promotion | Yes, estimator-only |
| PLS-SEM | Inference/resampling | Validated for documented PLS resampling scope: percentile bootstrap, BCa, studentized/bootstrap-t, jackknife support, and Freedman-Lane path permutation under audited settings; unsupported model shapes and unaudited small-sample/non-normal claims remain excluded | Yes, documented PLS inference scope |
| PLS-SEM | Mediation effect decomposition | Validated for documented PLS mediation scope: direct, indirect, total, VAF, classification, and validated bootstrap/permutation indirect-effect interpretation; PROCESS-style mediation and moderated mediation remain excluded | Yes, documented PLS mediation scope |
| PLS-SEM | Two-stage moderation | Validated for documented single-interaction two-stage moderation scope with generated product-score construct, simple slopes at standardized `-1/0/+1`, and validated bootstrap/permutation interpretation; moderated mediation and broader interaction systems remain excluded | Yes, documented two-stage moderation scope |
| PLS-SEM | Control paths | Experimental v0.5 schema/output semantics; uses ordinary structural path estimation | No |
| PLS-SEM | Higher-order constructs | Experimental v0.5 recipe/editor schema with repeated-indicator, two-stage, and hybrid indicator-split estimators plus independent Python and metamorphic evidence; invalid hybrid component splits are explicitly blocked | No |
| PLS-SEM | PLSc | Validated for documented reflective-only PLSc scope with path/factor weighting, rho_A attenuation-corrected construct correlations, paths, outer loadings, and R2; formative/PCA-weighting PLSc remains unsupported | Yes, documented PLSc scope |
| PLS-SEM | WPLS | Validated for documented positive case-weighted reflective WPLS scope with standardized preprocessing and path/factor weighting; WPLS inference, formative blocks, HOC/interactions, and PCA weighting remain unsupported | Yes, documented WPLS scope |
| PLS-SEM | CCA | Experimental descriptive composite correlation residual diagnostic for recursive standardized path models, with independent Python reference and invalid-settings guard evidence | No |
| PLS-SEM | CTA-PLS | Experimental descriptive tetrad diagnostic with sample-covariance tetrads for indicator blocks of four or more indicators; `npm run qpls:cta:reference` passes independent Python reference and invalid-block guards | No |
| PLS-SEM | Endogeneity analysis | Experimental Gaussian-copula diagnostic with rankit inverse-normal copula terms, augmented regressions, approximate t/p diagnostics, applicability warnings, and independent Python reference evidence | No |
| PLS-SEM | Nonlinear effects | Experimental fixed-score quadratic diagnostic with centered squared construct-score terms, augmented-regression statistics, R2 delta, warnings, and independent Python reference evidence | No |
| PLS-SEM | Moderated mediation | Experimental two-stage conditional indirect-effect diagnostic for first-stage and second-stage interaction-mediated paths, with invalid-recipe guard and independent Python reference evidence | No |
| Prediction | PLSpredict deterministic holdout, repeated k-fold, and CVPAT diagnostic | Validated for documented deterministic complete-case holdout, repeated k-fold, train-only preprocessing, construct-score LM benchmarks, Q2 predict, RMSE, MAE, and bounded CVPAT diagnostics; separate saved-model CVPAT and indicator-level PLSpredict remain unsupported | Yes, documented PLSpredict/CVPAT scope |
| Prediction | IPMA / cIPMA | Validated for documented bounded importance-performance scope using PLS total effects and 0-100 min-max performance from standardized construct/indicator scores; broader cIPMA extensions remain unsupported | Yes, documented IPMA scope |
| Groups | PLS-POS | Validated for documented deterministic 2-5 segment PLS-POS scope with deterministic starts, minimum segment-share guard, objective history, stable memberships, segment paths, and segment R2; backward-compatible `pls_pos_bounded_v1` remains readable but not promoted | Yes, documented PLS-POS scope |
| Groups | FIMIX-PLS | Validated for documented bounded deterministic 2-3 class score-space segmentation scope with class probabilities, memberships, class paths/R2, information criteria, and entropy; unrestricted EM/FIMIX parity is not claimed | Yes, documented FIMIX-PLS scope |
| Groups | Two-group MGA | Validated for documented two-group group-specific estimates and permutation MGA via metadata `mga_group_column` and `group_methods`; approximate normal path-difference diagnostics remain descriptive | Yes, documented permutation MGA scope |
| Groups | MICOM | Validated for documented two-group construct-score permutation scope with configural/compositional/mean/variance steps; broader invariance claims remain unsupported | Yes, documented MICOM scope |
| Assessment | Reliability, validity, structural quality, and fit diagnostics | Validated for documented assessment scope: alpha, rho_A, rho_C, AVE, cross-loadings, Fornell-Larcker, HTMT/HTMT+, VIF, R2/adjusted R2, f2, Q2, SRMR, and d_ULS; d_G, NFI, and RMS_theta remain excluded | Yes, documented assessment metrics |
| CB-SEM | CFA and maximum-likelihood SEM | Experimental v0.7.1 beta with direct single-group ML optimization and lavaan parity fixtures for bounded raw-data reflective CFA/SEM models; bootstrap, multigroup/invariance, robust/ordinal/FIML estimators, and broad publication validation remain pending | No |
| Components | Standalone PCA | Validated for documented PCA scope: standardized numeric raw-data PCA with listwise deletion, deterministic sign orientation, fixed/Kaiser/variance-threshold retention, eigenvalues, loadings, weights, and scores; rotations, pairwise deletion, covariance/correlation-only input, and inference remain excluded | Yes, documented PCA scope |
| Components | GSCA | Experimental v0.8 bounded component-model payload with GSCA-style fit diagnostics; full ALS publication validation remains pending | No |
| Regression | OLS regression | Validated for documented OLS scope: raw-data OLS with intercept, numeric predictors/controls, complete-case rows, HC3 robust standard errors, t statistics, p values, confidence intervals, fit diagnostics, fitted values, and residuals; HC0/HC4 claims, categorical encoding, survey weights, clustered SEs, GLS, mixed models, and panel models remain excluded | Yes, documented OLS scope |
| Regression | Logistic regression | Validated for documented binary 0/1 numeric complete-case scope with deterministic IRLS, Wald SE/z/p, odds ratios, predicted probabilities, log-likelihood, pseudo-R2, AIC, and BIC; multinomial, ordinal, weighted, clustered, and Firth-corrected models remain unsupported | Yes, documented logistic scope |
| Regression | PROCESS-style workflows | Validated for bounded mediation and moderation workflows generated from OLS component models; moderated mediation, the full Hayes catalogue, Johnson-Neyman claims, and complex serial/parallel mediation remain experimental or unsupported | Yes, bounded mediation/moderation scope |
| NCA | CE-FDH and CR-FDH | Validated for documented numeric X/Y CE-FDH and CR-FDH scope with deterministic permutation p values and bottleneck tables; nonnumeric variables and broader ceiling variants remain unsupported | Yes, documented NCA scope |

Publication audit update: `publication_ready_v0_1_to_v0_8` now clears for the documented supported scope. Rows that still say experimental describe broader or unsupported behavior outside that audited scope; they do not override the method-specific publication audit artifacts.

The desktop catalog and Rust core share this policy. A method name in the product roadmap is not evidence of implementation or validation outside its documented scope.

Detailed release gates and next actions are machine-readable in `D:\QuickPLS\validation\development_slices.json` and available with:

```powershell
cargo run -p qpls-cli -- roadmap
cargo run -p qpls-cli -- gate v0_4_assessment_reliability
```
