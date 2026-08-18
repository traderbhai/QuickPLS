# CB-SEM product-indicator moderation v1

Status: Internal-only scientific foundation. This document is not a Labs or Standard availability claim, a qualification promotion, or evidence of complete SmartPLS parity. Its source compiler identity is intentionally unregistered; it inherits no evidence from the existing LMS capability cell.

## Question answered

The method estimates whether a moderator common factor changes the structural effect of a predictor common factor on a common-factor outcome. It represents the latent interaction with observed products of the predictor and moderator indicators and estimates that expanded covariance structure with the exact CB-SEM parameter-table ML engine.

## External contract basis

The active [SmartPLS CB-SEM moderator documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/cbsem-moderation/) specifies two-way CB-SEM moderation, creation of an interaction construct from a moderation-on-path relation, the Cartesian product-indicator approach, and double mean centering as its default product-indicator variant. It also lists single mean centering and other computation approaches and exposes simple-slope plots and bootstrap inference.

The scientific basis is deliberately independent of SmartPLS:

- Kenny and Judd introduced measured-variable products as indicators of latent product variables: [doi:10.1037/0033-2909.96.1.201](https://doi.org/10.1037/0033-2909.96.1.201).
- Marsh, Wen, and Hau evaluated constrained, unconstrained, generalized appended product-indicator, and QML strategies and indicator construction: [doi:10.1037/1082-989X.9.3.275](https://doi.org/10.1037/1082-989X.9.3.275).
- Lin, Wen, Marsh, and Lin clarified double mean centering and recommended it over single mean centering and orthogonalization under their evaluated conditions: [doi:10.1080/10705511.2010.488999](https://doi.org/10.1080/10705511.2010.488999).

QuickPLS v1 implements an unconstrained product-indicator model. It does not claim numerical identity with proprietary SmartPLS output.

## Versioned contract

Method version: `cbsem_unconstrained_product_indicator_moderation_v1`

Transformation version: `cbsem_product_indicator_all_pairs_transform_v1`

Result schema: `1`

Internal source compiler identity: `(registry_schema_version=2, capability_id=smartpls.cbsem_moderator, cell_id=qpls3.cbsem.moderator.product_indicator, capability_version=cbsem_product_indicator_v1)`. This triple is deliberately distinct from the existing LMS triple `(2, smartpls.cbsem_moderator, qpls3.cbsem.moderator, cbsem_moderator_v1)`. The expanded data-materialized model has a separate inner `qpls3.cbsem.ml / cbsem_ml_v1` dependency receipt; that inner receipt does not replace or qualify the source product-indicator identity.

The scientific `SemModelV4` interaction must explicitly contain:

- one predictor common factor `X`;
- one moderator common factor `M`;
- a focal structural relation `X -> Y`;
- a main-effect relation `M -> Y`;
- one derived interaction output `X_by_M -> Y`;
- `method=product_indicator`;
- explicit centering, standardization, and pairing settings.

Missing construction settings fail semantic validation. Two-stage and orthogonalizing terms are not reinterpreted as product indicators. LMS is neither encoded nor executed by this method.

### Data and scope

- Raw, continuous, case-level data only.
- Listwise deletion across every observed indicator in the expanded SEM.
- Unweighted, single-group, recursive SEM only.
- One two-way factor-by-factor interaction per v1 run.
- Predictor and moderator require distinct, non-overlapping effect-indicator blocks with at least two indicators each.
- Each predictor/moderator source indicator must have exactly one loading, on its declared source factor. Cross-loadings are rejected because v1 does not generate the additional induced latent-product components.
- Predictor and moderator must be exogenous common factors identified by declared marker loadings.
- Predictor and moderator source-indicator residuals must be locally independent. An authored residual covariance touching either source block is rejected because the bounded generated product-error covariance pattern would otherwise be incomplete.
- The outcome must be a distinct common factor.
- CB-SEM ML point estimation without a mean structure, bootstrap, invariance steps, clusters, strata, or case weights.
- Generic bootstrap, studentized-inner, and permutation counts must all be zero. The v1 recipe sentinels are exactly `max_iterations=1000` and `tolerance=1e-7`, matching the fixed exact-engine optimizer contract rather than implying user-configurable convergence behavior.
- At least ten complete observations; the exact engine may impose additional identification or numerical requirements.
- Internal safety envelopes allow at most `81` Cartesian product columns and `10,000,000` complete-row-by-product cells. Checked arithmetic estimates `8` raw bytes and a conservative `24` peak work bytes per product cell (live product vector, Arrow copy, and one work copy), with a `256 MiB` product-work ceiling. Count overflow or either exceeded limit fails before base/product/Arrow vector allocation. These are fail-safe Internal bounds, not qualified performance claims and not a relaxation of the separate 100,000-row maximum-axis requirement.

### Product construction

Let predictor indicator `x_i` and moderator indicator `m_j` have complete-case means `xbar_i` and `mbar_j`. When requested, sample standard deviations use denominator `n - 1`.

For each constituent:

- `centering=none`, `standardization=none`: use the raw value.
- `centering=mean_center` or `double_mean_center`, `standardization=none`: subtract its complete-case mean.
- `standardization=sample_standard_deviation`: use `(value - mean) / sample_sd`; this operation necessarily centers even if `centering=none`.

`pairing=all_pairs` creates every Cartesian pair in canonical indicator-ID order. For each pair, form `q_ij = transformed(x_i) * transformed(m_j)`.

- `centering=none` or `mean_center`: final product `p_ij = q_ij`.
- `centering=double_mean_center`: final product `p_ij = q_ij - mean(q_ij)`.

All statistics are computed only after listwise deletion. Means and sample standard deviations use scale-aware accumulation and no absolute epsilon cutoff, so valid positive rescaling remains admissible under sample standardization. True zero variance and non-finite centering/statistics/product overflow are distinct typed failures. The run records all means, sample standard deviations, pre-second-centering product means, final product means, and final product standard deviations.

### Deterministic materialization and identification

The compiler converts the derived interaction output into an explicit common factor only in the immutable estimator plan. It generates stable IDs from the interaction term and source indicator IDs, never from declaration order.

- The product of the predictor's declared marker indicator and the moderator's declared marker indicator is the interaction marker; its loading is fixed exactly to `1`.
- Remaining product-indicator loadings are free.
- Every product indicator has a free positive residual variance.
- The latent interaction has a free positive variance and a fixed-zero mean.
- The interaction factor covaries freely with the predictor and moderator factors.
- Residual covariances are freely estimated between product indicators that share a predictor or moderator indicator.
- The interaction effect retains the exact user-authored structural parameter stable ID.

The source Recipe-v4 artifact contains the immutable product plan, which binds both the unexpanded scientific interaction and the expanded estimator plan. The run records the distinct Internal product-indicator source receipt and generic-ML inner dependency receipt, the source and transformed dataset fingerprints, the complete/omitted row counts, construction statistics, stable product mappings, and exact estimator output.

Execution is exposed only through the versioned `internal` runner request. The normal CB-SEM runner continues to reject derived terms, and both `labs` and `standard` surface values fail closed. The isolated runner forwards progress and cancellation, refuses a tampered surface or request version, and checks result, method, and transformation versions before returning a payload. Registry governance is required before any future Labs or Standard exposure.

## Typed failures

Preflight rejects, with corrective codes, among other cases:

- missing moderator main effect;
- missing or wrong interaction method/settings;
- multiple interactions or another derived term;
- observed/composite predictor, moderator, or outcome in this bounded slice;
- shared predictor/moderator indicators;
- endogenous predictor/moderator factors, non-marker factor identification, cross-loaded source indicators, or residual covariances touching their source indicators;
- non-continuous or pre-transformed input indicators;
- covariance/correlation input;
- weights, groups, clusters, strata, feedback, or structural intercepts;
- missing, duplicate, nonnumeric, non-finite, or true zero-variance source columns, plus non-finite centered values/statistics or product overflow;
- Cartesian product-count overflow, more than 81 product columns, checked cell/byte-estimate overflow, more than 10,000,000 materialized product cells, or the 256 MiB product-work estimate;
- underidentification, singular information, inadmissible starts, and nonconvergence from the exact engine.

Cancellation is checked at immutable boundaries, at bounded intervals during row/product materialization, and in every exact optimizer iteration. Cancellation returns no result payload. Arrow serialization/fingerprinting currently provides pre/post checkpoints rather than an interruptible library call, so the maximum-axis one-second cancellation budget remains unqualified for this Internal slice.

## Results and interpretation

The primary moderation parameter is the user-authored `X_by_M -> Y` regression. Its scale depends on the selected constituent transformations and marker-product identification. Main and interaction coefficients must be interpreted together.

Product indicators are nonnormal even when their constituents are normal. The current point-estimate slice does not qualify normal-theory standard errors, p-values, bootstrap inference, simple-slope plots, or interaction-aware global fit for publication use. The result therefore carries machine-readable exclusions for the nested generic engine's standard-error/z/p fields and fit block plus one warning; results and export surfaces must not present those fields as supported product-indicator outputs. Those are separate option cells and remain hidden from Standard.

## Explicitly outside v1

- LMS or QML estimation.
- SmartPLS generic-indicator behavior where it differs from the documented all-pairs product construction.
- Orthogonalization/residual centering.
- Matched-pair indicator selection.
- Multiple simultaneous interactions, quadratic terms, moderated mediation, observed moderators, categorical/ordinal indicators, missing-data FIML, multigroup estimation, bootstrap inference, and simple-slope chart/export workflows.
- Registry, evidence-state, or Standard visibility changes.

## Implemented foundation evidence

The validation-only `validation/cbsem_product_indicator_moderation_v1_oracle.py` imports no QuickPLS code. It independently reconstructs the all-pairs transformation and the 32-free-dimension unconstrained covariance model, proves full local Jacobian rank and positive-definite start matrices, and fits a deterministic full-rank fixture with NumPy/SciPy ML. Its default scope matches the Rust minimum of two indicators per source factor and ten complete rows; its explicit hand-case mode is transformation-only. Focused tests cover hand-calculated raw, mean-centered, double-mean-centered, and sample-standardized products; row and declaration reorder; moderate and `1e-20` positive rescaling under standardization; distinct zero-variance, source-nonfinite, and product-overflow failures; and inside/boundary/outside/overflow resource-envelope cases without allocating the boundary workload.

The Rust foundation tests separately cover stable compiler materialization, source-marker identification, explicit covariance-ID goldens, exogenous/simple-structure/local-independence preflight, product-count and checked memory envelopes, declared-method and optimizer/resampling fail-closed behavior, distinct source/expanded receipts, product arithmetic, row-order and small-scale standardization equivalence, sample-covariance admissibility, exact parameter-table execution, typed overflow, stable interaction identity, mid-materialization cancellation, and result exclusions. Frozen cross-language assertions compare the first transformed rows, all product means/sample deviations and ML variances, and the interaction estimate to the independent NumPy/SciPy oracle while the real runner test also asserts the Internal source receipt and generic-ML inner receipt. These are foundation checks only; they do not change the capability registry or satisfy the remaining promotion matrix below.

## Qualification evidence required before any promotion

- a full frozen cross-language payload comparing every QuickPLS transformed column, expanded covariance cell, and parameter ID to the independent NumPy/SciPy oracle (the runner interaction-estimate cross-check is already present);
- a second independent implementation such as lavaan plus semTools/modsem with an explicitly aligned model;
- row/declaration reorder, positive affine rescaling, save/reopen, GUI/CLI, seed/worker, and export/readback properties where applicable;
- null/signal simulations reporting convergence and failed fits;
- adversarial missingness, zero variance, collinearity, underidentification, Heywood, singular-information, cancellation, and tampering cases;
- packaged Windows, accessibility, performance, and soak checks;
- independent scientific review.
