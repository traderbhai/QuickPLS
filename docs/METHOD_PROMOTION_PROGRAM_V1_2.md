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

- PLS core, documented PLS assessment/inference scope, standalone PCA, OLS, second-batch PLS/prediction/NCA methods, and v1.2.2 group/prediction/regression methods are eligible for validated product status.
- The broad Regression method remains setting-aware: OLS, logistic, and bounded PROCESS mediation/moderation are validated; PROCESS moderated mediation remains experimental.
- GSCA, CB-SEM/CFA, higher-order constructs, nonlinear effects, endogeneity, CCA, CTA-PLS, and moderated mediation remain experimental until their own promotion gates pass.
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

## v1.2.2 Group, Prediction, And Regression Batch

The third promotion batch is validated for bounded scopes:

- MICOM for exactly two observed groups with configural, compositional, mean, and variance permutation diagnostics.
- Two-group permutation MGA with deterministic group-label permutation and MICOM warning enforcement.
- Deterministic PLS-POS with 2-5 segments, deterministic starts, objective history, memberships, segment paths, and segment R2.
- Bounded deterministic 2-3 class FIMIX-PLS with probabilities, memberships, information criteria, entropy, and no unrestricted EM/FIMIX parity claim.
- Binary numeric complete-case logistic regression with deterministic IRLS, Wald tests, odds ratios, probabilities, log-likelihood, pseudo-R2, AIC, and BIC.
- Bounded PROCESS-style mediation and moderation generated from OLS component models.

Remaining later-batch methods remain experimental: CB-SEM/CFA, GSCA, higher-order constructs, nonlinear effects, endogeneity, CCA, CTA-PLS, and moderated mediation.

## Gate

The registry slice is `v1_2_method_promotion_program`.

This gate remains open until the remaining higher-risk method families are promoted with reproducible evidence. The first, second, and third batches are complete for their documented scopes.
