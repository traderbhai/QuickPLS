# PLS Assessment Method Specification v4

Status: frozen for the v0.4 deterministic assessment preview; experimental until independent-reference and simulation gates pass. V1, v2, and v3 result payloads remain readable. V4 adds blindfolding cross-validated redundancy and correlation-residual fit measures.

Current `pls_assessment_v7` retains the v4 fit/blindfolding contract, v5 rho_A, and v6 explicit original HTMT/HTMT+ artifacts. It changes Cohen f-squared to the fixed-score reduced structural regression convention documented in `PLS_ASSESSMENT_V3.md`, so older `pls_assessment_v6` project results remain readable but are not mislabeled as current output.

## Blindfolding Cross-Validated Redundancy

QuickPLS reports Stone-Geisser cross-validated redundancy Q-squared for reflective endogenous constructs. It follows the systematic cell-omission procedure described in the SmartPLS blindfolding documentation: https://www.smartpls.com/documentation/algorithms-and-techniques/blindfolding/. SmartPLS has discontinued this method in favor of PLSpredict and CVPAT; QuickPLS therefore labels Q-squared as an in-sample predictive-relevance diagnostic, not out-of-sample predictive performance.

- The recipe contract does not yet expose an omission-distance setting. V4 records a deterministic assessment setting: prefer `D = 7`, otherwise choose the first valid distance from `5, 6, 8, 9, 10, 11, 12`. A distance is valid only when `D < n` and `n mod D != 0`. If none is valid, blindfolding is unavailable with a warning.
- For each reflective endogenous construct and round `r = 0..D-1`, target-indicator cells are addressed column-major and omitted when `(indicator_offset * n + row) mod D = r`. Thus every target-indicator cell is omitted exactly once, while `n mod D != 0` prevents a complete observation row from being omitted in one round.
- Omitted cells are replaced by that indicator's mean over retained cells, and the same PLS recipe is re-estimated. This explicitly freezes the missing-value treatment rather than relying on an undocumented estimator default.
- The structural prediction of the target score is the sum of its predecessor scores times the round-specific path coefficients. Each omitted standardized indicator is predicted as its round-specific outer loading times that structural score.
- For each target, `PRESS` is the sum of squared standardized omitted-cell prediction errors and `SSO` is the sum of squared standardized omitted values relative to the retained-cell mean benchmark. `Q^2 = 1 - PRESS / SSO`. A zero `SSO`, a failed round, or a formative target produces an unavailable value and warning.

The method uses model-wide complete cases selected by the base estimator. Every round forwards cooperative cancellation into PLS estimation. The stored settings, PRESS, SSO, and Q-squared make the calculation auditable.

## Correlation-Residual Fit

V4 reports saturated and estimated variants of SRMR and d_ULS. Let `S` be the empirical standardized indicator correlation matrix and `Sigma` the corresponding model-implied matrix.

- `d_ULS = 0.5 * sum_ij (S_ij - Sigma_ij)^2`, equivalently the sum of squared lower-triangle residuals including the diagonal.
- `SRMR = sqrt(d_ULS / (K(K+1)/2))` for `K` indicators.

These definitions match cSEM's `calculateDL()` and `calculateSRMR()` implementations and its published fit documentation: https://floschuberth.github.io/cSEM/articles/Using-assess.html.

The saturated construct correlation matrix is the empirical construct-score correlation matrix. The estimated construct correlation matrix retains empirical exogenous correlations and recursively propagates the fitted structural paths in topological order, with unit construct variances. Reflective indicator correlations are `lambda_i * correlation(construct_i, construct_j) * lambda_j`; formative indicators within the same block retain their empirical correlation, following the composite-block convention. Indicator diagonals equal one.

These are descriptive discrepancy measures. No universal cutoff or model-fit test is applied. d_G is not implemented because a positive-definite model-implied matrix and its geodesic convention require further admissibility work. NFI and RMS_theta are not implemented because their precise null-model and outer-residual contracts are not yet frozen.

## Versioning

`pls_assessment_v4` adds optional `model_fit` and `blindfolding` fields. Missing fields deserialize as empty for v1-v3 archives. Nonempty v4-only fields under legacy version labels are invalid. Current payload validation requires finite nonnegative fit values, the SRMR/d_ULS identity, valid recorded omission settings, unique construct rows, and `Q^2 = 1 - PRESS/SSO` whenever all three values are present.
