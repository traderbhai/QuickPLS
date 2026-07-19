# Dijkstra-Henseler rho_A Specification v1

Status: implemented experimental method introduced under `pls_assessment_v5` and retained by current `pls_assessment_v7`. Equation-level independent fixtures, primary-paper Equation 3 fixtures, and the cSEM 0.6.1 comparison pass; broader v0.4 release validation remains open, so the method is not yet publication-ready.

## Scope and Source Definition

QuickPLS reports Dijkstra-Henseler's `rho_A` as the estimated reliability of a PLS Mode A construct score under a reflective common-factor measurement model. It is not Cronbach's alpha, rho_C, an average of those coefficients, or a reliability measure for arbitrary composites.

The definition follows Equation 3 and Appendix B, Equations 7-18, of Dijkstra and Henseler (2015). Their construction chooses a loading vector proportional to the Mode A weight vector so that the off-diagonal elements of the empirical indicator covariance matrix are reproduced as closely as possible in unweighted least squares. The paper and appendices are available from the authors at https://www.henseler.com/Dijkstra-Henseler-MISQ-2015.pdf and have DOI https://doi.org/10.25300/MISQ/2015/39.2.02. The related asymptotic treatment is Dijkstra and Henseler (2015), https://doi.org/10.1016/j.csda.2014.07.008.

## QuickPLS Input Basis

For one construct with `K` indicators, use exactly the model-wide complete cases retained by the authoritative PLS estimate. Let `Z` be the `n x K` matrix formed by centering every retained indicator and dividing by its sample standard deviation with denominator `n-1`. Let `R = Z'Z/(n-1)` be the empirical indicator correlation matrix. Its diagonal is one within numerical tolerance.

The estimator's persisted outer weights refer to its configured preprocessing scale, so they must first be expressed in standardized-indicator coordinates. For estimator weight `a_j`, raw complete-case sample standard deviation `s_j`, and persisted preprocessing scale `t_j`, define:

`u_j = a_j * s_j / t_j`.

Thus `u_j = a_j` for standardized estimation, while mean-centered and unstandardized weights are multiplied by the raw indicator standard deviation. Location shifts vanish when the construct score is standardized. Reject nonfinite weights and nonpositive or inconsistent scales.

Let `q = u' R u`. Normalize `w = u / sqrt(q)` so that `w' R w = 1`. The resulting score `eta_hat = Z w` must agree with the persisted mean-zero, unit-sample-variance construct score in the estimator's orientation within `1e-10`; equivalently, `R w` must agree with the persisted indicator-score correlations/outer loadings within `1e-10`. A mismatch is an inconsistent estimation input, not an alternative rho_A convention.

Use signed weights exactly as estimated. Do not take absolute values or orient indicators individually. Reversing the entire construct orientation changes `w` to `-w` and leaves rho_A exactly unchanged.

## Exact Formula

Define the off-diagonal matrices:

- `A = R - diag(R)`;
- `D = w w' - diag(w w')`.

Define:

- `g = w' w`;
- `numerator = w' A w = sum_(j != k) w_j w_k R_jk`;
- `denominator = w' D w = sum_(j != k) w_j^2 w_k^2 = g^2 - sum_j w_j^4`.

The fitted squared proportionality factor is:

`c_squared = numerator / denominator`.

The Dijkstra-Henseler reliability estimate is:

`rho_A = g^2 * c_squared = (w'w)^2 * [w'(R - diag(R))w] / [w'(w w' - diag(w w'))w]`.

This is the standardized-correlation form of the source equation. Because `w'Rw = 1`, the fitted consistent loading vector would be `lambda_consistent = sqrt(c_squared) * w` when `c_squared >= 0`, and `rho_A = (w'lambda_consistent)^2`. QuickPLS computes the algebraic rho_A formula directly; it does not substitute the upward-biased ordinary PLS loadings.

## Applicability

- Available only for reflective constructs estimated by iterative Mode A under the path or factor inner-weighting scheme.
- Not applicable to formative/Mode B constructs. The assessment value is null; it is not reported as one. A future PLSc attenuation step may separately impose a reliability-one convention for a composite, but that value is not rho_A.
- Not applicable to PCA-weighted blocks because those weights are not Mode A weights and the cited consistency argument does not apply.
- Not identified for a single-item reflective construct because no off-diagonal indicator correlation exists. QuickPLS does not assume error-free measurement silently.
- A two-indicator reflective construct is computable when both weights contribute, but receives a limited-information warning because only one distinct off-diagonal correlation informs the fit.
- No universal pass/fail cutoff is enforced. Interpretation belongs in the report and must retain any warnings.

## Numerical Policy and Diagnostics

All matrix products use `f64` with a fixed deterministic summation order matching recipe indicator order. Before weight normalization, require finite `q` and `q > q_tol`, where `q_tol = 64*f64::EPSILON*max(1, sum_j abs(u_j)^2)`. After normalization, require `abs(w'Rw - 1) <= 1e-10`.

For the off-diagonal denominator use `d_tol = 64*f64::EPSILON*max(1, g^2, sum_j w_j^4)`. A nonfinite value or `denominator <= d_tol` makes rho_A unavailable. Any other nonfinite intermediate or result also makes it unavailable. Do not regularize `R`, add a ridge, delete indicators, change weight signs, or fall back to alpha/rho_C.

In the population model rho_A is a reliability in `[0,1]`, but finite samples can produce an improper estimate. Do not broadly clamp it. A result in `[-b_tol,0)` is canonicalized to zero and a result in `(1,1+b_tol]` to one, with `b_tol = 64*f64::EPSILON*max(1, abs(rho_A))`. A value below `-b_tol` or above `1+b_tol` is persisted unchanged with an improper-estimate warning. A negative value cannot produce real-valued consistent loadings and must block its later use in PLSc; a value above one must likewise block attenuation correction unless a separately validated policy is frozen.

Stable reason and warning codes:

- `rho_a.formative_not_applicable`
- `rho_a.pca_weights_not_applicable`
- `rho_a.single_indicator_not_identified`
- `rho_a.invalid_indicator_scale`
- `rho_a.invalid_score_variance`
- `rho_a.estimation_input_mismatch`
- `rho_a.off_diagonal_denominator_zero`
- `rho_a.nonfinite_result`
- `rho_a.two_indicator_limited_information`
- `rho_a.improper_below_zero`
- `rho_a.improper_above_one`

The first three are non-applicability reasons. The next five are numerical unavailability reasons. The final three are warnings attached to an otherwise persisted finite result.

## Persistence Contract

Adding rho_A changes the meaning of the assessment payload and therefore requires a new assessment envelope version rather than mutating `pls_assessment_v4`. The recommended next envelope is `pls_assessment_v5`, with top-level `rho_a_method_version: "dijkstra_henseler_rho_a_v1"`.

Each construct-quality row persists:

- `rho_a: Option<f64>`;
- `rho_a_status: "available" | "not_applicable" | "unavailable"`;
- `rho_a_reason: Option<stable_code>`;
- `rho_a_warning_codes: Vec<stable_code>`;
- `rho_a_indicator_count`;
- nullable audit components `score_variance_before_normalization`, `normalized_weight_norm_squared`, `off_diagonal_numerator`, and `off_diagonal_denominator`.

An available row has a finite rho_A and all audit components, no reason, and only recognized warning codes. A nonavailable row has null rho_A and a reason consistent with construct mode, weighting scheme, indicator count, and audit fields. Project validation recomputes the formula identity within `1e-12`, requires one unique row per model construct in recipe order, and rejects rho_A fields under v1-v4 envelopes. Legacy envelopes remain readable with rho_A absent.

## Focused Validation Fixtures

1. **Three-item hand fixture:** for an equicorrelation matrix with every off-diagonal `0.5` and equal normalized weights `w_j = 1/sqrt(6)`, verify `g=0.5`, `numerator=0.5`, `denominator=1/6`, and `rho_A=0.75` within `1e-12`.
2. **Two-item hand fixture:** for correlation `0.6` and equal unit-score weights `w_j = 1/sqrt(3.2)`, verify `rho_A = 2r/(1+r) = 0.75`, plus the two-indicator warning.
3. **Direct-matrix fixture:** use unequal signed weights and a fixed positive-definite `R`; independently calculate every scalar sum and the matrix expression, including an improper negative case and a value above one.
4. **Reference fixtures:** reproduce the population example in Dijkstra and Henseler's paper and compare equivalent Mode A results with development-only cSEM. Require `1e-6` agreement after matching complete cases, standardization, inner weighting, orientation, and weight normalization. A second independent equation-level implementation must agree within `1e-12` on supplied `R,w` fixtures.
5. **Metamorphic fixtures:** require invariance to construct orientation reversal, positive affine indicator transformations under standardized estimation, common rescaling of a fixed incoming weight vector, and simultaneous permutation of indicator rows/columns/weights. Construct and unrelated-indicator reordering must not change a row. Do not require equality across standardized, mean-centered, and unstandardized estimator settings: raw-scale Mode A can fit a different composite. For every setting, independently verify the coordinate conversion by reproducing the persisted loadings from `R w`.
6. **Applicability fixtures:** reflective two-/multi-item, reflective single-item, formative, PCA, zero-weight/zero-denominator, constant indicator, nonfinite input, input-score mismatch, and both improper-range warnings must produce exactly the frozen status and code.
7. **Persistence fixtures:** v5 round-trip and formula validation, rejected duplicate/missing rows, rejected inconsistent audit scalars/status/reasons, and clean v1-v4 reads without rho_A.

The hand, independent Decimal equation, primary-paper Equation 3, local metamorphic, applicability, archive-integrity, and cSEM 0.6.1 comparison suites are implemented. Full v0.4 promotion remains blocked by the remaining assessment-family release gates and full release review. No publication-ready validation claim follows from the currently passing local suite.
