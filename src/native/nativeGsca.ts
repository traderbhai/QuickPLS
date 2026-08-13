export const NATIVE_GSCA_METHOD_VERSION = "gsca_als_v2" as const;
export const NATIVE_GSCA_ALGORITHM_VERSION = "alternating_least_squares_v1" as const;
export const NATIVE_GSCA_ASSESSMENT_WARNING =
  "PLS assessment is not applicable to GSCA ALS component-model estimation.";
export const NATIVE_GSCA_ENGINE_SCOPE_WARNING =
  "GSCA ALS v2 is bounded to standardized raw data, listwise deletion, disjoint reflective/formative blocks, and recursive single-group structural models; inference and broader GSCA variants are not included.";
export const NATIVE_GSCA_SCOPE_NOTE =
  "Joint global least-squares ALS with fixed +1 initialization, 3,000 maximum iterations, a 1e-7 objective-and-weight stop criterion, listwise-standardized numeric indicators, disjoint reflective/formative blocks, and recursive single-group paths. No controls, covariance paths, interactions, higher-order constructs, case weights, multigroup analysis, or inference.";
