# PLS-PM Method Specification v1

Status: validated for the documented v0.3 estimator scope.

## Scope

This specification covers recursive PLS path models with numeric raw observations, reflective Mode A blocks, formative Mode B blocks, single-item blocks, path and factor inner weighting, and PCA block weighting. Cyclic models and covariance-only inputs are rejected in v0.3.

## Data

- The default preprocessing is column centering followed by sample-standard-deviation scaling (`n - 1`). Mean-centered and unstandardized inputs are also supported and their transformations are recorded.
- Rows missing any model indicator are removed listwise. The used and omitted observation counts are reported.
- Constant indicators, duplicate assignments, unknown indicators, non-finite values, duplicate construct identifiers/paths, and rank-deficient Mode B or structural regressions are errors. The estimator rejects any method other than `pls_pm`; resampling is dispatched separately.

## Iterative PLS

1. Initialize every block weight to equal positive values and rescale it so its outer proxy has sample variance one.
2. For factor weighting, use the correlations of connected outer proxies as inner weights.
3. For path weighting, use multiple-regression coefficients for predecessor proxies and correlations for successor proxies.
4. Form each inner proxy as the weighted sum of adjacent outer proxies and standardize it.
5. Mode A updates each outer weight with the covariance between its indicator and the inner proxy. Mode B updates the block jointly by least-squares regression of the inner proxy on mean-centered indicators, including when unstandardized preprocessing is requested. This is equivalent to fitting an intercept and makes Mode B weights invariant to arbitrary indicator offsets.
6. Rescale updated block weights to produce a unit-variance outer proxy. Orient each block so its first nonzero outer weight is positive.
7. Stop when the maximum absolute signed weight change is no greater than the configured tolerance. Reaching the iteration limit is a non-convergence error and never produces a completed result.

PCA weighting replaces steps 2-6 with the dominant eigenvector of each block covariance matrix, using deterministic power iteration and the same orientation and score normalization. Component orientation is chosen by nonnegative covariance with the order-independent unit-weighted block composite; a near-zero fallback uses the sum of weights.

All regressions use column-pivoted QR. Numerical rank is read from the absolute diagonal of the pivoted `R` factor using `max(|diag(R)|) * max(n, p) * epsilon * 100`; `X'X` is not formed.

## Final Estimates

- Construct scores have mean zero and sample variance one.
- Outer loadings are indicator-score correlations. Outer weights refer to the recorded preprocessed indicator scale.
- Each endogenous construct is regressed on all direct predecessors using column-pivoted QR least squares.
- R-squared is `1 - SSE/SST` on the standardized construct score.
- Direct effects are structural coefficients. Total effects are the finite path expansion `B + B^2 + ... + B^(K-1)` for a recursive `K`-construct model; indirect effects equal total minus direct.

## Determinism and Promotion

Construct and indicator order are taken from the versioned model recipe. No random initialization is used. A method remains `unsupported` until hand fixtures, published examples, two independent reference engines, order/scale metamorphic tests, CLI/GUI serialization equality, and the `1e-6` deterministic agreement gate pass.
