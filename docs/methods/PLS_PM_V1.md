# PLS-PM Method Specification v1

Status: release-qualified bounded scoped Standard. Capability coverage remains partial relative to the full comparator; the exact `pls_pm_v1` scope below has current engine, archive, native, and packaged evidence in Capability Registry V2.

## Scope

This specification covers recursive PLS path models with numeric raw observations, reflective Mode A blocks, formative Mode B blocks, single-item blocks, path and factor inner weighting, and PCA block weighting. Cyclic models and covariance-only inputs are rejected in v0.3.

The current SmartPLS parity defaults are a maximum of 3,000 iterations and a `1e-7` stop criterion. QuickPLS uses those defaults. Internal Recipe-v4 execution now implements Individual initialization and Unit/Custom fixed scoring under all three typed normalizations: `none`, `sum_to_one`, and `unit_variance`. That implementation does not by itself establish qualification or parity coverage for those options or every result type.

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

## Determinism and qualification

Construct and indicator order are taken from the versioned model recipe. No random initialization is used. This specification alone is not qualification evidence; the separate method-promotion identities bind the current hand fixtures, published examples, independent reference engines, deterministic and boundary checks, strict archive lifecycle, selected-run XLSX export, native results, and packaged Windows workflow for this bounded scope. The frozen independent-engine comparison tolerance is `1e-6`; it is a qualification agreement gate, not the estimator stop criterion, whose product default remains `1e-7`. QualificationSpec V2 retains broader options, large-scale matrices, cross-format breadth, and unrestricted parity as a non-blocking future full-parity backlog; scoped Standard does not imply those claims.
