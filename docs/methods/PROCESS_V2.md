# Graph-defined Path Analysis and PROCESS v2

`regression_process_v2` is QuickPLS's independently implemented, graph-defined
observed-variable path-analysis workflow. It does not execute or expose copied
numbered PROCESS templates. Authors declare directed equations and path-scoped
moderation relationships visually.

## Frozen scope

- Raw numeric data, one shared listwise-complete sample, unstandardized OLS
  equations, exact HC3 covariance, and fixed two-sided 95% Student-t inference.
  Each equation is solved after leaving the intercept unchanged and centering
  every non-intercept design column at its equation-sample mean, then dividing
  it by its population RMS deviation. A thin SVD rejects the equation as
  `rank_deficient_equation` when
  `s_min <= s_max * max(n, p) * machine_epsilon * 100`. Coefficients and HC3
  covariance are back-transformed to the original term units and original
  canonical term order; raw normal-equation pivots are not used.
  Exact HC3 divides each residual by `1 - leverage`; an equation fails with
  `high_leverage_hc3_instability` when that denominator is nonfinite or no
  greater than `1e-12`. It is never silently capped. A nonfinite, zero, or
  negative HC3 covariance diagonal fails with `invalid_hc3_covariance`; the
  estimator never applies an absolute-value repair to a variance.
- A requested conditional simple slope with zero or nonfinite derived variance
  fails with `degenerate_simple_slope_variance`; QuickPLS does not fabricate a
  zero statistic, unit p-value, or zero-width interval.
- One focal predictor and one terminal continuous outcome; up to four mediators,
  two distinct reusable moderators, eight predictors in total, one control,
  sixteen directed paths, four moderated paths, and fifty generated terms in
  any equation.
- Parallel and serial mediation. A mediated path may contain one moderated
  first-stage edge (`X` to its first mediator) or one moderated second-stage
  edge (its last mediator to `Y`), but not both. Intermediate-stage moderation
  is rejected.
- One- or two-moderator direct/non-mediated moderation. Two moderators generate
  the complete lower-order hierarchy and the three-way product.
- Continuous product participants use
  `equation_complete_case_mean_v1` centering. Exact numeric 0/1 moderators are
  not centered. Every bootstrap and delete-one fit recomputes its own means;
  reported probes remain fixed to the original complete sample.
- Engine-authored simple slopes, conditional indirect effects, indices of
  moderated mediation, predicted-outcome plot points, and Johnson-Neyman
  regions. Continuous probes use raw mean minus one sample SD, mean, and mean
  plus one sample SD; binary probes use 0 and 1.
- Johnson-Neyman arithmetic uses the fitted equation's HC3 coefficient
  covariance and residual-df Student-t critical value. Regions and roots are
  clipped to the original observed moderator range. Binary solved moderators
  receive a tagged unavailable result rather than fabricated output. The raw
  moderator interval is first mapped to the bounded domain `[-1, 1]`; the
  transformed quadratic coefficients are then divided by their maximum
  absolute magnitude. Linearization and discriminant decisions use
  `64 * machine_epsilon` relative tolerances, roots use the
  cancellation-resistant q-formula. Roots are range-clipped and deduplicated
  in normalized moderator coordinates at `128 * machine_epsilon`; endpoint-
  relative mapping and a valid Vieta companion recover a small boundary root
  that raw affine back-transformation would otherwise cancel. Conditional-
  effect variance must be finite and strictly positive over
  the complete tested range (including an in-range variance-quadratic vertex),
  otherwise the row is tagged `invalid_hc3_covariance`; no zero/negative
  variance is clamped into a confidence interval.

## Case bootstrap

The optional `regression_process_bootstrap_v1` layer uses seeded indexed case
resampling with replacement. It requires 99-10,000 requested replicates and at
least 90% usable fits. Type-7 percentile intervals are primary. BCa is available
for an estimand only when all delete-one fits are usable; otherwise the payload
contains an explicit tagged reason. Ratio tests use the original estimate divided
by its bootstrap standard error with a two-sided standard-normal reference.

The bootstrap stream, method version, and
`regression_process_bootstrap_validation_witness_v1` witness are distinct from
the release-qualified standalone OLS/logistic bootstrap. The full witness is
persisted for arithmetic and tamper validation but must never be rendered or
exported. Point-only provenance is `regression_process_v2`; bootstrap provenance
is `regression_process_v2+regression_process_bootstrap_v1`.

## Determinism and ordering

Top-level predictors are the focal predictor, mediators in topological order
with lexical tie-breaking, then moderators in declaration order. Equations use
that topological order. Terms are intercept, incoming graph parents, moderator
main effects, hierarchical two-way then three-way products, and controls in
recipe order. Bootstrap streams are indexed and invariant to worker count.
Conditional-effect and plot identities use semantic probe tokens
`minus_1sd`, `mean`, `plus_1sd`, `binary_0`, and `binary_1`; raw values remain
separate numeric evidence. Slope identities include the complete moderated-edge
identity, so a moderator reused across paths cannot collide.
Tokens are assigned from the canonical grid position, never inferred by rounding
the raw value. A continuous moderator is rejected when mean minus sample SD,
mean, and mean plus sample SD are not three distinct finite `f64` values.

Reference-effect tables and exports disclose the graph-wide reference policy:
continuous moderators on a relevant moderated edge are evaluated at their
original complete-sample raw mean (coded zero), and binary moderators are
evaluated at zero. This condition is never presented as an unconditional
effect.

Equation-fit payloads persist observations, parameter count, residual sum of
squares, and total sum of squares. Archive validation derives R-squared,
adjusted R-squared, F, AIC, BIC, and RMSE from those sufficient statistics.
The current PROCESS v2 estimation root omits the unrelated legacy PLS mediation
and two-stage-moderation shells. Historical payloads still deserialize through
their existing defaults, while newly trusted PROCESS v2 archives require both
legacy shell fields to be absent.
Exports include every persisted conditional plot point (25 per series) and
every available Johnson-Neyman curve point (101 per row), but never the internal
bootstrap validation witness.

## Explicit exclusions

Binary/logistic outcomes or equations, categorical auto-coding beyond exact 0/1
moderators, weights, clusters, custom confidence levels or tails, studentized
intervals, cycles or reciprocal paths, intermediate-stage moderated mediation,
multiple moderated edges on one indirect path, and two-moderator products on a
mediated path are unsupported. SmartPLS project import and numerical-identity
claims are outside scope.

The original global complete-case profile must be continuous for every
endogenous equation outcome, including mediators and the terminal outcome; an
exact observed `{0,1}` level set is rejected. Bootstrap and delete-one fits carry
that validated original scope and do not reinterpret a single-class resample as
a newly supported or unsupported outcome family.

Graph-role variable names cannot contain C0/C1 control characters or the
reserved identity delimiters `->`, `@`, `|`, `*`, `,`, and `=`. This bounded
restriction keeps archived effect, path, moderation, and semantic-probe IDs
unambiguous.

Historical `regression_process_v1` results remain immutable and archive-readable
under their original label. They are not evidence for v2, cannot be executed as
new schema-v3 jobs, and legacy v1 recipes are not silently migrated into an
executable contract; users must author a graph-defined v2 relationship.

## Qualification status

The bounded `regression_process_v2` capability has passed its current
method-specific and packaged qualification evidence. The coordinated QuickPLS
2.46.0 Wave 1 release records this bounded capability qualification. The
qualifying evidence intentionally exercised conservative candidate/experimental
presentation; that presentation and the method's explicit warnings do not
expand or weaken the scientific exclusions above. Historical v1 output remains
archive-only.

## Method references and comparison scope

These references freeze the public equations and terminology used by the
independent validation suite; they are not runtime dependencies.

- MacKinnon, J. G., and White, H. (1985), "Some
  Heteroskedasticity-Consistent Covariance Matrix Estimators with Improved
  Finite Sample Properties," *Journal of Econometrics*, 29(3), 305-325.
  [doi:10.1016/0304-4076(85)90158-7](https://doi.org/10.1016/0304-4076(85)90158-7).
  This is the source for the HC3 leverage adjustment; QuickPLS additionally
  freezes its explicit near-unit-leverage failure boundary above.
- Student (1908), "The Probable Error of a Mean," *Biometrika*, 6(1), 1-25.
  [doi:10.1093/biomet/6.1.1](https://doi.org/10.1093/biomet/6.1.1).
  QuickPLS uses the equation residual degrees of freedom for its fixed
  two-sided Student-t coefficient and conditional-effect inference.
- Preacher, K. J., Curran, P. J., and Bauer, D. J. (2006), "Computational Tools
  for Probing Interactions in Multiple Linear Regression, Multilevel Modeling,
  and Latent Curve Analysis," *Journal of Educational and Behavioral
  Statistics*, 31(4), 437-448.
  [doi:10.3102/10769986031004437](https://doi.org/10.3102/10769986031004437).
  This supplies the simple-slope, conditional-effect, and Johnson-Neyman
  probing framework used by the bounded observed-variable workflow.
- Hayes, A. F. (2015), "An Index and Test of Linear Moderated Mediation,"
  *Multivariate Behavioral Research*, 50(1), 1-22.
  [doi:10.1080/00273171.2014.962683](https://doi.org/10.1080/00273171.2014.962683).
  QuickPLS reports its independently implemented, path-scoped moderated-
  mediation indices; it does not copy numbered templates or macros.
- Efron, B. (1979), "Bootstrap Methods: Another Look at the Jackknife,"
  *Annals of Statistics*, 7(1), 1-26.
  [doi:10.1214/aos/1176344552](https://doi.org/10.1214/aos/1176344552), and
  Efron, B. (1987), "Better Bootstrap Confidence Intervals," *Journal of the
  American Statistical Association*, 82(397), 171-185.
  [doi:10.1080/01621459.1987.10478410](https://doi.org/10.1080/01621459.1987.10478410).
  These ground case resampling and the conditional BCa interval calculation.
- Hyndman, R. J., and Fan, Y. (1996), "Sample Quantiles in Statistical
  Packages," *The American Statistician*, 50(4), 361-365.
  [doi:10.1080/00031305.1996.10473566](https://doi.org/10.1080/00031305.1996.10473566).
  QuickPLS freezes the paper's Type-7 interpolation identity for percentile
  and adjusted BCa endpoints.
- The comparison catalogue is the official
  [SmartPLS Algorithms and Techniques](https://smartpls.com/documentation/algorithms-and-techniques/)
  snapshot frozen on 2026-08-12. It establishes product-outcome scope only;
  it is not a claim of copied internals or numerical identity.
