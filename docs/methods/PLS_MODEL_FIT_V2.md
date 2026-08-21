# PLS Model Fit v2

Status: implemented point-estimate contract with bounded Labs exact-fit inference; not release-qualified and not full SmartPLS parity.

Qualification status: [PLS model-fit v2 qualification status](PLS_MODEL_FIT_QUALIFICATION_STATUS_V2.md).

## Question and supported output

`pls_model_fit_v2` describes the discrepancy between the observed indicator-correlation matrix and two model-implied indicator-correlation matrices produced by the same completed PLS or PLSc run:

- the **saturated model**, in which all construct correlations are represented; and
- the **estimated model**, in which the recursive structural model restricts construct correlations.

For both variants the result records SRMR, `d_ULS`, `d_G`, maximum-likelihood-function Chi-square, degrees of freedom, and NFI. It also records analytical sample size, deterministic indicator order, the observed and both implied correlation matrices, the lower-triangle convention, and the logarithm convention used by `d_G`.

The active SmartPLS model-fit inventory and its distinction between saturated and estimated models are documented at <https://smartpls.com/documentation/algorithms-and-techniques/model-fit/>. SmartPLS is a workflow comparator, not the sole numerical oracle.

## Frozen equations

Let `S` be the observed indicator-correlation matrix, `Sigma_hat` a model-implied indicator-correlation matrix, `p` the number of indicators, `N` the complete-case analytical sample size, and `phi_k` the eigenvalues of `S^-1 Sigma_hat`.

```text
d_ULS = 1/2 trace((S - Sigma_hat)^2)
      = sum(i >= j) (S_ij - Sigma_hat_ij)^2

SRMR = sqrt(d_ULS / (p(p + 1)/2))

d_G = 1/2 sum(k = 1..p) ln(phi_k)^2

F_ML = trace(Sigma_hat^-1 S) - ln|S| + ln|Sigma_hat| - p

Chi-square = (N - 1) F_ML

NFI = 1 - Chi-square_model / Chi-square_null
```

`d_G` uses the natural logarithm, following equation 36 of Dijkstra and Henseler (2015), *Consistent and asymptotically normal PLS estimators for linear structural equations*, DOI `10.1016/j.csda.2014.07.008`. The implementation uses a symmetric positive-definite generalized-eigenvalue construction rather than explicitly forming `S^-1 Sigma_hat`.

Degrees of freedom use the standardized correlation-model parameter accounting documented by cSEM: nonredundant off-diagonal indicator moments minus measurement, structural, and free construct-correlation parameters. Reflective blocks count their loadings; composite blocks count normalized weights and free within-block indicator correlations. The saturated variant counts all construct correlations, while the estimated variant counts structural paths and free exogenous correlations.

## Failure semantics

SRMR and `d_ULS` remain deterministic descriptive discrepancies for finite symmetric correlation matrices. `d_G` and Chi-square are criterion-level unavailable when the observed or implied matrix is not positive definite, a decomposition fails, or the criterion is non-finite. NFI is unavailable whenever either required Chi-square is unavailable or the null-model Chi-square is numerically zero. No unavailable criterion is replaced with zero, an identity information matrix, an absolute value, or another statistic.

Stored v2 results are semantically checked by recomputing matrix-derived values. Historical fit blocks without the v2 identity remain readable as historical SRMR/`d_ULS` results and are never reinterpreted as v2.

## Interpretation boundary

SRMR and NFI are approximate fit measures. Raw `d_ULS` and `d_G` values do not by themselves establish exact fit. QuickPLS has a separately versioned bounded Labs implementation of the adapted Bollen-Stine workflow for supported raw-data PLS-PM and PLSc recipes. Its internal explicit recipe selector runs independent saturated and estimated null transformations, fixed indexed full refits, Type-7 HI95 and HI99 bounds, empirical upper-tail proportions, and decisions for SRMR, `d_ULS`, and `d_G`. Ordinary parameter-bootstrap intervals are never relabelled as exact-fit inference.

The point-fit object remains a descriptive matrix-and-criterion payload and does not embed the resampling result. Availability comes only from a linked `pls_model_fit_exact_v1` bundle and provenance marker. A missing selector or bundle therefore means that exact-fit inference was not run; it does not authorize a substitute statistic.

## Version 2.52 desktop presentation

The ordinary Results entry is titled **Model fit — descriptive**. Its interpretation appears behind a neutral information button and in Model Fit Details instead of as a persistent amber warning. The compact exact-fit state is derived from the linked payload's aggregate status and uses one of these labels:

- **Exact-fit bootstrap: Not run**;
- **Exact-fit results available**;
- **Exact-fit results partial**;
- **Exact-fit results unavailable**; or
- **Exact-fit run failed**.

Amber or red is used only when exact-fit inference was requested but incomplete or failed. The bounded adapted Bollen–Stine Registry cell remains unqualified and is not exposed as a Calculate option in Version 2.52. Historical stored reason codes and result identities remain unchanged; presentation translates them without rewriting the archive. Full interpretation text remains available in Model Fit Details and exports.

Consequently, the active capability remains partial. A transparent independent
NumPy/SciPy oracle now performs complete recursive PLS-PM refits for every
small indexed validation draw and independently reproduces the point matrices
and criteria. This closes the previous primitive-only gap for its bounded
Mode-A/Mode-B PLS-PM scope, but it is not full PLS/PLSc breadth or accepted
qualification evidence. Full parity and qualification still require closing
supported reflective/formative/mixed and difficult-model breadth, independent
PLSc and advanced-shape refits, the second-oracle-or-documented-exception rule,
pre-registered Type-I-error/power/failure simulations, real
999/5,000/10,000-draw save/reopen and all-format readback, packaged Windows and
accessibility execution, maximum-axis performance and soak evidence, and
independent scientific review. The frozen exact-fit details and present
evidence boundary are documented in
[PLS_MODEL_FIT_EXACT_V1.md](PLS_MODEL_FIT_EXACT_V1.md), and the bounded oracle
contract is documented in
[PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md](PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md).

RMS_theta is not part of the current SmartPLS parity inventory and is not produced by this contract.
