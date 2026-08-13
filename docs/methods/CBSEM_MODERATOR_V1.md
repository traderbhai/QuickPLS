# CB-SEM Moderator v1

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported interaction

`cbsem_moderator_v1` estimates one interaction between a continuous latent predictor `xi` and one observed continuous exogenous moderator `W` in a single-group recursive reflective ML SEM. The moderator is centered by its model-wide complete-case mean. The structural equation is `eta = alpha + beta_x xi + beta_w W_c + beta_xw (xi * W_c) + zeta`.

The latent interaction is estimated from the raw-data likelihood using the Latent Moderated Structural Equations (LMS) method, not by treating factor scores as error-free and not by silently generating product indicators. The primary estimand is the unstandardized interaction coefficient `beta_xw`. Simple slopes are `beta_x + beta_xw w` at the persisted moderator mean and mean plus/minus one sample standard deviation. The workflow independently qualifies reuse of the `cbsem_bootstrap_v2` case-resampling, seed, quantile, and failure protocol while refitting the LMS model; those intervals belong to `cbsem_moderator_v1`, not the standalone bootstrap identity. The likelihood-ratio comparison to the no-interaction model is reported because conventional global chi-square fit is not directly interchangeable across LMS and ordinary-normal models.

The estimator follows Klein and Moosbrugger (2000), *Maximum Likelihood Estimation of Latent Interaction Effects with the LMS Method*, DOI `10.1007/BF02296338`.

## Preconditions and output

The base model must be identified and admissible; `xi` and `eta` are reflective continuous latent variables; `W` is observed, finite, nonconstant, exogenous, and not reused as an indicator. Persist centering/scaling, interaction identity, quadrature/mixture settings, convergence diagnostics, unstandardized coefficients, simple slopes, likelihood-ratio result, bootstrap interval/failures, and provenance.

The coefficient is scale-dependent. A statistically nonzero interaction does not prove causation, eliminate confounding, or justify extrapolation beyond the observed moderator range.

## Exclusions

Latent-by-latent, categorical, endogenous, multiple, quadratic, three-way, multigroup, ordinal, robust, and multilevel interactions; Johnson-Neyman claims outside supported observed range; factor-score OLS; and product-indicator substitutions are excluded from v1.
