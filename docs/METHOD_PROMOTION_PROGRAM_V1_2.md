# QuickPLS v1.2 Method Promotion Program

The v1.2 method promotion program converts implemented experimental calculation families into researcher-ready validated methods one bounded scope at a time.

The program starts from the current state:

- QuickPLS v1.0/v1.1.1 product and desktop workflow gates are validated.
- Several method families have implementation and validation artifacts.
- Many method rows remain experimental because the broader publication claim still lacks full method-specific promotion evidence.

## Goal

Create a repeatable promotion pipeline that answers three questions for every calculation family:

1. What exact scope is supported?
2. What evidence proves the numbers are correct and reproducible?
3. What must stay experimental, watermarked, or unsupported?

## Work Packages

### WP1: Status Reconciliation

Reconcile:

- `docs/V1_COMPATIBILITY_MATRIX.md`
- `docs/METHOD_COMPATIBILITY.md`
- `docs/V1_SUPPORTED_SCOPE.md`
- `validation/development_slices.json`

The output must distinguish bounded validated scope from broader experimental method surfaces without contradictory labels.

### WP2: Promotion Matrix

Maintain a machine-readable method promotion matrix listing:

- method family;
- current status;
- candidate promoted scope;
- required artifacts;
- current evidence;
- missing evidence;
- known differences;
- proposed promotion batch.

### WP3: First Stable Method Batch

Prepare the first promotion batch:

- PLS core full stable run envelope;
- assessment metrics;
- inference/resampling for documented PLS settings;
- standalone PCA;
- OLS regression.

These methods are prioritized because their equations and reference engines are comparatively mature and their researcher use is common.

### WP4: Simulation And Second-Source Expansion

Extend validation for medium and high-risk methods:

- CCA/CTA and remaining extended PLS diagnostics;
- higher-order constructs, nonlinear effects, endogeneity, and moderated mediation;
- CB-SEM/CFA;
- GSCA.

### WP5: Product Enforcement

Update the app so researcher-ready methods appear without experimental watermarks only inside the promoted scope. Anything outside that scope must remain blocked, hidden, or explicitly watermarked.

Current enforcement pass:

- PLS core, documented PLS assessment/inference scope, standalone PCA, OLS, second-batch PLS/prediction methods, and v1.2.2 group/prediction/regression methods are eligible for validated product status within their evidence-backed bounds. Corrected NCA v2, standalone PCA v1, and bounded OLS v1 now have genuine packaged execution, native XLSX, canonical save, and same-run reopen evidence; none of those claims extends beyond its documented model-free scope.
- The broad Regression method remains setting-aware: OLS, logistic, and bounded PROCESS mediation/moderation are validated; PROCESS moderated mediation remains experimental.
- Higher-order constructs, nonlinear effects, endogeneity, CCA, CTA-PLS, PLS moderated mediation, CB-SEM/CFA, and GSCA are validated only for the bounded v1.2.3/v1.2.4 scopes listed below.
- Newly generated result warnings and export tables must not mark an unpromoted method as validated.

## v1.2.1 Second Batch

The second promotion batch retains validated bounded numerical scopes. Bounded IPMA, NCA v2, and current indicator-level prediction promotion require independent numerical/reference evidence, strict archive contracts, genuine packaged runs, native XLSX export, explicit save, and same-run reopen:

- PLS mediation effect decomposition with validated indirect-effect inference.
- Two-stage moderation with one generated product-score interaction and validated interaction inference.
- Reflective-only PLSc with path/factor weighting.
- Positive case-weighted reflective WPLS with standardized preprocessing and path/factor weighting.
- Bounded IPMA using predecessor total effects and observed-range standardized-score performance; numerical/reference, strict persistence, native UI, packaged execution, XLSX export, and save/reopen contracts pass. Theoretical-range correction and cIPMA remain unsupported.
- PLSpredict / CVPAT v2 with train-only reflective endogenous-indicator prediction, a fixed seeded balanced 10-fold plan repeated 10 times, IA and LM benchmarks, Q²_predict, RMSE/MAE/MAPE, and one-sided 95% aggregate benchmark tests. Construct scores and the modulo-4 holdout are supplementary; folds are independently fixed rather than SmartPLS-randomized, and separate saved-model comparison remains unsupported. `plspredict_holdout_v1` is archive-readable legacy output only.
- Numeric X/Y NCA v2 with record-high CE-FDH peers, CR-FDH regression through those peers, seeded permutation p values, status-bearing observed-range bottlenecks, strict append/save/reopen validation, and packaged setup/results/XLSX/reopen acceptance. The former nca_v1 result is legacy-only.

## v1.2.2 Group, Prediction, And Regression Batch

The third promotion batch retains these validated bounded scopes:

- MICOM v2 plus two-group permutation MGA v2 with explicit ordered groups, 5,000–10,000 usable deterministic label permutations, group-specific path/loading/weight re-estimation, MICOM Steps 1–3, strict result persistence, native table-only export, and packaged save/reopen evidence. Historical `micom_v1` remains withdrawn.
- Deterministic PLS-POS with 2-5 segments, deterministic starts, objective history, memberships, segment paths, and segment R2.
- Bounded deterministic 2-3 class FIMIX-PLS with probabilities, memberships, information criteria, entropy, and no unrestricted EM/FIMIX parity claim.
- Binary numeric complete-case logistic regression with deterministic IRLS, Wald tests, odds ratios, probabilities, log-likelihood, pseudo-R2, AIC, and BIC.
- Bounded PROCESS-style mediation and moderation generated from OLS component models.

## v1.2.3 Extended PLS Diagnostics Batch

The fourth promotion batch is validated for bounded scopes:

- Higher-order constructs using repeated-indicator, two-stage, and documented hybrid contracts.
- CCA as a descriptive composite residual diagnostic.
- CTA-PLS as a descriptive sample-covariance tetrad diagnostic.
- Gaussian-copula endogeneity as a diagnostic screen, not causal proof.
- Nonlinear effects as fixed-score centered-quadratic diagnostics.
- PLS moderated mediation as a two-stage conditional indirect-effect diagnostic.

Bootstrap decision rules, unrestricted nonlinear SEM, broad HOC variants, and full Hayes PROCESS catalogue claims remain unsupported unless separately audited.

## v1.2.4 CB-SEM/CFA And GSCA Batch

The fifth promotion batch is validated for bounded scopes:

- Raw-data single-group reflective measurement-only CFA or recursive SEM ML with marker identification, lavaan parity fixtures, standardized and unstandardized parameters, residual matrices, fit indices, and residual-based modification screening. Packaged native execution, XLSX export, strict typed persistence, and same-run reopen are accepted for this bounded scope.
- Bounded `gsca_als_v2` joint global least-squares ALS for listwise-standardized raw data, disjoint reflective/formative blocks, and recursive single-group paths. Evidence covers weights, loadings, paths, R2, objective, FIT/adjusted/local FIT, GFI, SRMR, convergence, independent numerical comparison, genuine packaged execution, native XLSX, strict typed persistence, and same-run reopen. Historical `gsca_v1` remains legacy preview-only.

CB-SEM mean structures, bootstrap, unrestricted multigroup/invariance, robust/ordinal/FIML estimators, controls, interactions, higher-order constructs, broad constraints, GSCA score export, GSCA inference, and unrestricted GSCA variants remain experimental or unsupported.

## Gate

The registry slice is `v1_2_method_promotion_program`.

This gate is clear when the first through fifth promotion batches are complete for their documented scopes and product/export surfaces enforce the same scope boundaries.
