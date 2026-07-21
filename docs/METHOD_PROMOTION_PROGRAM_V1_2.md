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

- mediation/moderation/PROCESS;
- PLSc/WPLS/CCA/CTA;
- PLSpredict/CVPAT/IPMA/NCA;
- MICOM/MGA/FIMIX/PLS-POS;
- CB-SEM/CFA;
- GSCA.

### WP5: Product Enforcement

Update the app so researcher-ready methods appear without experimental watermarks only inside the promoted scope. Anything outside that scope must remain blocked, hidden, or explicitly watermarked.

Current enforcement pass:

- PLS core, documented PLS assessment/inference scope, standalone PCA, and OLS regression are eligible for validated product status.
- The broad Regression method remains setting-aware: OLS is validated, while logistic regression and PROCESS-style workflows remain experimental.
- NCA, GSCA, CB-SEM/CFA, segmentation, prediction/heterogeneity methods, and extended PLS methods remain experimental until their own promotion gates pass.
- Newly generated result warnings and export tables must not mark an unpromoted method as validated.

## v1.2.1 Second Batch

The second promotion batch is validated for bounded scopes:

- PLS mediation effect decomposition with validated indirect-effect inference.
- Two-stage moderation with one generated product-score interaction and validated interaction inference.
- Reflective-only PLSc with path/factor weighting.
- Positive case-weighted reflective WPLS with standardized preprocessing and path/factor weighting.
- Bounded IPMA/cIPMA using PLS total effects and standardized-score performance.
- Deterministic PLSpredict holdout, repeated k-fold, LM benchmark, Q2 predict, RMSE/MAE, and bounded CVPAT diagnostics.
- Numeric X/Y NCA with CE-FDH and CR-FDH ceilings, deterministic permutation p values, and bottleneck tables.

Later-batch methods remain experimental: CB-SEM/CFA, GSCA, MICOM/MGA, FIMIX-PLS, PLS-POS, logistic regression, PROCESS-style workflows, higher-order constructs, nonlinear effects, endogeneity, CCA, CTA-PLS, and moderated mediation.

## Gate

The registry slice is `v1_2_method_promotion_program`.

This gate should remain open until the first method batch is actually promoted with reproducible evidence. The initial program setup is considered complete when the criteria docs, registry slice, audit script, and initial backlog artifact exist.
