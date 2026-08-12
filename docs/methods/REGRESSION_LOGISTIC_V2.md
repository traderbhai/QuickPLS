# REGRESSION_LOGISTIC_V2

Status: current backend and project-persistence contract for the bounded native
binary logistic workflow.

`regression_logistic_v2` fits an intercept plus selected unstandardized numeric
predictors and controls by deterministic Newton IRLS. It uses listwise complete
cases, requires the outcome to contain both exact numeric values `0` and `1`,
and stops without a completed result for non-binary values, a single observed
class, rank deficiency, extreme fitted probabilities indicating possible
separation or unstable scaling, nonconvergence, or cancellation. The
probability guard is conservative diagnostic protection, not a proof of
separation.

## Frozen equations and defaults

- Probability: `p_i = logistic(x_i beta)` using a numerically stable logistic
  transform.
- Score: `X' (y - p)`; information matrix: `X' W X`, where
  `W_ii = p_i (1 - p_i)`.
- Update: solve `(X' W X) delta = X' (y - p)`, then
  `beta <- beta + delta`.
- Convergence: maximum absolute update below `1e-8`, with at most 100 updates.
- Execution is single-worker (`workers = 1`) and deterministic.
- Extreme-probability guard: any fitted probability below `1e-9` or above
  `1 - 1e-9`; this indicates possible separation or unstable scaling.
- Inference: model-based inverse-information standard errors, Wald z tests,
  two-sided normal p values, and fixed 95% confidence intervals.
- Odds ratios and their confidence bounds are exponentiated coefficient-scale
  estimates and bounds.
- Fit: log likelihood, null log likelihood, deviance, null deviance, McFadden
  pseudo-R2, likelihood-ratio chi-square and p value, AIC, and BIC.
- Classification: descriptive threshold `0.5`, with the confusion matrix,
  accuracy, sensitivity, and specificity. This in-sample classification is
  not a validated predictive performance estimate.

## Persisted provenance

The result stores the exact complete-case outcome profile, numeric coding rule,
class counts and prevalence; optimizer name, iteration count, tolerances and
final update; every complete-case probability and residual; coefficient and
odds-ratio inference; classification counts; and fit identities. Project append,
save, and reopen validation recomputes the arithmetic and rejects tampering
atomically. Historical `regression_logistic_v1` results remain readable but
cannot be appended as new evidence.

## Exclusions

Multinomial and ordinal outcomes, categorical auto-encoding, weights, clustered
or robust covariance, Firth correction, penalization, external resampling, and
confidence levels other than two-sided 95% are outside this contract.
