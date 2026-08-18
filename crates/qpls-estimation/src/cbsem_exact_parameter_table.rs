use crate::{
    CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1, CBSEM_CFA_SCORE_LM_SCOPE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1,
    CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1, CBSEM_FIT_METHOD_VERSION,
    CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP,
    CBSEM_ML_OPTIMIZER_OBJECTIVE_STAGNATION_TOLERANCE,
    CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE,
    CBSEM_ML_OPTIMIZER_STRICT_GRADIENT_NORM_TOLERANCE, CbsemAnalysis, CbsemCfaScoreLmBundleV1,
    CbsemCfaScoreLmOutcomeV1, CbsemCfaScoreLmRowV1, CbsemCfaScoreLmUnavailableReasonV1,
    CbsemExactCaseBootstrapParameterEstimateV1, CbsemExactCaseBootstrapParameterStandardErrorV1,
    CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
    CbsemExactCaseBootstrapRefitStandardErrorsV1, CbsemFitIndices, CbsemMatrixCell, CbsemParameter,
    CbsemRmseaIntervalAttributionV1, CbsemStandardizedParameter,
};
use faer::Mat;
use qpls_core::ModelSpec;
use statrs::distribution::{ContinuousCDF, Normal};
use statrs::function::gamma::gamma_lr;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3: &str = "cbsem_ml_exact_parameter_table_v3";
pub const CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4: &str = "cbsem_ml_exact_parameter_table_v4";
const CBSEM_GENERAL_ML_STABILITY_MARGIN_V1: f64 = 1e-8;
const CBSEM_GENERAL_ML_MAXIMUM_SPECTRAL_RADIUS_V1: f64 = 1.0 - CBSEM_GENERAL_ML_STABILITY_MARGIN_V1;
const CBSEM_INFORMATION_CHOLESKY_MINIMUM_PIVOT_V1: f64 = 1e-10;
const CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1: f64 = 0.90;
const CBSEM_NONCENTRAL_CHI_SQUARE_MIXTURE_TOLERANCE_V1: f64 = 1e-12;
const CBSEM_NONCENTRAL_CHI_SQUARE_MAXIMUM_TERMS_V1: usize = 100_000;
const CBSEM_NONCENTRAL_CHI_SQUARE_INVERSION_TOLERANCE_V1: f64 = 1e-11;
const CBSEM_NONCENTRAL_CHI_SQUARE_INVERSION_MAXIMUM_ITERATIONS_V1: usize = 256;
const CBSEM_NONCENTRAL_CHI_SQUARE_BRACKET_MAXIMUM_ITERATIONS_V1: usize = 128;
const CBSEM_EXPECTED_INFORMATION_SCALED_ONE_NORM_CONDITION_DIAGNOSTIC_V1: &str =
    "cbsem.expected_information.scaled_one_norm_condition_number.v1";
const CBSEM_EXPECTED_INFORMATION_SCALED_RECIPROCAL_ONE_NORM_CONDITION_DIAGNOSTIC_V1: &str =
    "cbsem.expected_information.scaled_reciprocal_one_norm_condition_number.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CbsemExactLatentCovarianceKindV3 {
    Exogenous,
    Disturbance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CbsemExactParameterTargetV3 {
    Loading {
        construct: String,
        indicator: String,
    },
    Regression {
        source: String,
        target: String,
    },
    FactorVariance {
        factor: String,
    },
    FactorCovariance {
        left: String,
        right: String,
        kind: CbsemExactLatentCovarianceKindV3,
    },
    ResidualVariance {
        indicator: String,
    },
    ResidualCovariance {
        left: String,
        right: String,
    },
    ObservedIntercept {
        indicator: String,
    },
    LatentMean {
        factor: String,
    },
}

impl CbsemExactParameterTargetV3 {
    fn family(&self) -> &'static str {
        match self {
            Self::Loading { .. } => "loading",
            Self::Regression { .. } => "regression",
            Self::FactorVariance { .. } | Self::ResidualVariance { .. } => "variance",
            Self::FactorCovariance { .. } | Self::ResidualCovariance { .. } => "covariance",
            Self::ObservedIntercept { .. } => "intercept",
            Self::LatentMean { .. } => "latent_mean",
        }
    }

    fn is_variance(&self) -> bool {
        matches!(
            self,
            Self::FactorVariance { .. } | Self::ResidualVariance { .. }
        )
    }

    fn default_start(
        &self,
        indicator_index: &HashMap<&str, usize>,
        sample_covariance: &[Vec<f64>],
        sample_means: Option<&[f64]>,
    ) -> f64 {
        match self {
            Self::Loading { .. } => 1.0,
            Self::Regression { .. } => 0.0,
            Self::FactorVariance { .. } => 0.05,
            Self::ResidualVariance { indicator } => indicator_index
                .get(indicator.as_str())
                .map(|index| 0.5 * sample_covariance[*index][*index])
                .unwrap_or(0.5),
            Self::FactorCovariance { .. } | Self::ResidualCovariance { .. } => 0.0,
            Self::ObservedIntercept { indicator } => indicator_index
                .get(indicator.as_str())
                .and_then(|index| sample_means.and_then(|means| means.get(*index)))
                .copied()
                .unwrap_or(0.0),
            Self::LatentMean { .. } => 0.0,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Loading { .. } => "loading",
            Self::Regression { .. } => "structural_path",
            Self::FactorVariance { .. } => "latent_variance",
            Self::FactorCovariance {
                kind: CbsemExactLatentCovarianceKindV3::Exogenous,
                ..
            } => "latent_covariance",
            Self::FactorCovariance {
                kind: CbsemExactLatentCovarianceKindV3::Disturbance,
                ..
            } => "disturbance_covariance",
            Self::ResidualVariance { .. } => "residual_variance",
            Self::ResidualCovariance { .. } => "residual_covariance",
            Self::ObservedIntercept { .. } => "observed_intercept",
            Self::LatentMean { .. } => "latent_mean",
        }
    }

    fn lhs_rhs(&self) -> (&str, &str) {
        match self {
            Self::Loading {
                construct,
                indicator,
            } => (construct, indicator),
            Self::Regression { source, target } => (target, source),
            Self::FactorVariance { factor } => (factor, factor),
            Self::FactorCovariance { left, right, .. }
            | Self::ResidualCovariance { left, right } => (left, right),
            Self::ResidualVariance { indicator } => (indicator, indicator),
            Self::ObservedIntercept { indicator } => (indicator, "1"),
            Self::LatentMean { factor } => (factor, "1"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CbsemExactParameterSpecificationV3 {
    Free {
        start: Option<f64>,
        lower: Option<f64>,
        upper: Option<f64>,
        equality_label: Option<String>,
    },
    Fixed {
        value: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CbsemExactParameterRowV3 {
    pub stable_id: String,
    pub name: String,
    pub target: CbsemExactParameterTargetV3,
    pub specification: CbsemExactParameterSpecificationV3,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum CbsemExactParameterTableErrorV3 {
    #[error("exact CB-SEM parameter id is duplicated: {0}")]
    DuplicateParameterId(String),
    #[error("exact CB-SEM result parameter name is duplicated: {0}")]
    DuplicateParameterName(String),
    #[error("exact CB-SEM parameter target is duplicated: {0}")]
    DuplicateParameterTarget(String),
    #[error("exact CB-SEM parameter {parameter} references unknown {kind} {id}")]
    UnknownTarget {
        parameter: String,
        kind: String,
        id: String,
    },
    #[error("exact CB-SEM equality label is empty for parameter {0}")]
    EmptyEqualityLabel(String),
    #[error("exact CB-SEM equality label {0} is attached to only one parameter")]
    SingletonEqualityLabel(String),
    #[error("exact CB-SEM equality label {label} mixes parameter families {families:?}")]
    EqualityFamilyMismatch {
        label: String,
        families: Vec<String>,
    },
    #[error("exact CB-SEM equality label {label} has conflicting explicit starts")]
    EqualityStartConflict { label: String },
    #[error("exact CB-SEM parameter {parameter} has non-finite bounds or start")]
    NonFiniteParameterValue { parameter: String },
    #[error("exact CB-SEM parameter {parameter} has an empty or reversed open bound interval")]
    InvalidBounds { parameter: String },
    #[error("exact CB-SEM equality label {label} has no common feasible bound interval")]
    EqualityBoundsEmpty { label: String },
    #[error(
        "exact CB-SEM parameter {parameter} start {start} is not strictly inside ({lower:?}, {upper:?})"
    )]
    StartOutsideBounds {
        parameter: String,
        start: f64,
        lower: Option<f64>,
        upper: Option<f64>,
    },
    #[error("fixed CB-SEM variance {parameter} must be strictly positive")]
    FixedVarianceNonPositive { parameter: String },
    #[error("exact CB-SEM factor variance is missing for {0}")]
    MissingFactorVariance(String),
    #[error("exact CB-SEM residual variance is missing for {0}")]
    MissingResidualVariance(String),
    #[error("raw-data mean structure requires one observed intercept for {0}")]
    MissingObservedIntercept(String),
    #[error("mean-structure parameter rows require an observed sample-mean vector")]
    MeanStructureDataRequired,
    #[error("observed sample means must contain {expected} finite values (found {actual})")]
    InvalidObservedMeans { expected: usize, actual: usize },
    #[error(
        "latent mean for {factor} requires its marker-indicator intercept {indicator} to be fixed"
    )]
    LatentMeanMarkerInterceptNotFixed { factor: String, indicator: String },
    #[error(
        "exact CB-SEM model is underidentified: {free_parameters} free dimensions exceed {observed_moments} observed moments"
    )]
    NegativeDegreesOfFreedom {
        free_parameters: usize,
        observed_moments: usize,
    },
    #[error(
        "exact CB-SEM model is locally underidentified at its declared start: Jacobian rank {rank} < {free_parameters}"
    )]
    LocalUnderidentification { rank: usize, free_parameters: usize },
    #[error("initial exact CB-SEM latent/disturbance covariance matrix is not positive definite")]
    InitialLatentCovarianceNotPositiveDefinite,
    #[error("initial exact CB-SEM residual covariance matrix is not positive definite")]
    InitialResidualCovarianceNotPositiveDefinite,
    #[error("initial exact CB-SEM implied covariance matrix is not positive definite")]
    InitialImpliedCovarianceNotPositiveDefinite,
    #[error(
        "exact CB-SEM structural system is unstable at {stage}: spectral radius {spectral_radius} must be below {maximum}"
    )]
    StructuralSystemUnstable {
        stage: String,
        spectral_radius: f64,
        maximum: f64,
    },
    #[error("exact CB-SEM structural stability could not be evaluated at {stage}: {message}")]
    StructuralStabilityUnavailable { stage: String, message: String },
    #[error("exact CB-SEM information matrix is singular")]
    InformationMatrixSingular,
    #[error(
        "exact CB-SEM expected-information matrix is not positive definite at pivot {pivot}: {value} <= {minimum}"
    )]
    InformationMatrixNotPositiveDefinite {
        pivot: usize,
        value: f64,
        minimum: f64,
    },
    #[error(
        "exact CB-SEM inverse-information variance is invalid for free dimension {dimension}: {value}"
    )]
    InformationVarianceInvalid { dimension: usize, value: f64 },
    #[error("exact CB-SEM numerical derivative is unavailable for free dimension {0}")]
    GradientUnavailable(usize),
    #[error("exact CB-SEM optimizer did not converge")]
    NonConvergence,
    #[error(
        "exact CB-SEM optimizer line search failed during {stage} after {iterations} iterations (objective {objective}, gradient norm {gradient_norm})"
    )]
    OptimizerLineSearchFailed {
        stage: String,
        iterations: u32,
        objective: f64,
        gradient_norm: f64,
    },
    #[error(
        "exact CB-SEM optimizer stagnated above the convergence tolerance during {stage} after {iterations} iterations (objective {objective}, gradient norm {gradient_norm})"
    )]
    OptimizerObjectiveStagnation {
        stage: String,
        iterations: u32,
        objective: f64,
        gradient_norm: f64,
    },
    #[error(
        "exact CB-SEM optimizer reached its iteration limit during {stage} after {iterations} iterations (objective {objective}, gradient norm {gradient_norm})"
    )]
    OptimizerIterationLimit {
        stage: String,
        iterations: u32,
        objective: f64,
        gradient_norm: f64,
    },
    #[error("exact CB-SEM execution was cancelled")]
    Cancelled,
    #[error("exact CB-SEM numerical failure: {0}")]
    Numerical(String),
}

#[derive(Debug, Clone, Copy)]
struct OpenDomainV3 {
    lower: Option<f64>,
    upper: Option<f64>,
}

impl OpenDomainV3 {
    fn contains(self, value: f64) -> bool {
        value.is_finite()
            && self.lower.is_none_or(|lower| value > lower)
            && self.upper.is_none_or(|upper| value < upper)
    }

    fn value_from_raw(self, raw: f64) -> f64 {
        match (self.lower, self.upper) {
            (None, None) => raw,
            (Some(lower), None) => lower + raw.exp(),
            (None, Some(upper)) => upper - raw.exp(),
            (Some(lower), Some(upper)) => {
                let logistic = if raw >= 0.0 {
                    1.0 / (1.0 + (-raw).exp())
                } else {
                    let exp = raw.exp();
                    exp / (1.0 + exp)
                };
                lower + (upper - lower) * logistic
            }
        }
    }

    fn raw_from_value(self, value: f64) -> f64 {
        match (self.lower, self.upper) {
            (None, None) => value,
            (Some(lower), None) => (value - lower).ln(),
            (None, Some(upper)) => (upper - value).ln(),
            (Some(lower), Some(upper)) => {
                let unit = (value - lower) / (upper - lower);
                (unit / (1.0 - unit)).ln()
            }
        }
    }

    fn derivative(self, raw: f64) -> f64 {
        match (self.lower, self.upper) {
            (None, None) => 1.0,
            (Some(_), None) => raw.exp(),
            (None, Some(_)) => -raw.exp(),
            (Some(lower), Some(upper)) => {
                let logistic = if raw >= 0.0 {
                    1.0 / (1.0 + (-raw).exp())
                } else {
                    let exp = raw.exp();
                    exp / (1.0 + exp)
                };
                (upper - lower) * logistic * (1.0 - logistic)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct FreeDimensionV3 {
    key: String,
    domain: OpenDomainV3,
    start_value: f64,
}

#[derive(Debug, Clone)]
enum ResolvedSpecificationV3 {
    Free { dimension: usize },
    Fixed { value: f64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ResolvedTargetV3 {
    Loading { construct: usize, indicator: usize },
    Regression { source: usize, target: usize },
    FactorVariance { factor: usize },
    FactorCovariance { left: usize, right: usize },
    ResidualVariance { indicator: usize },
    ResidualCovariance { left: usize, right: usize },
    ObservedIntercept { indicator: usize },
    LatentMean { factor: usize },
}

#[derive(Debug, Clone)]
struct ResolvedRowV3 {
    stable_id: String,
    name: String,
    target_description: CbsemExactParameterTargetV3,
    target: ResolvedTargetV3,
    specification: ResolvedSpecificationV3,
}

#[derive(Debug)]
struct ExactPlanV3 {
    construct_ids: Vec<String>,
    indicator_ids: Vec<String>,
    dimensions: Vec<FreeDimensionV3>,
    rows: Vec<ResolvedRowV3>,
}

#[derive(Debug)]
struct ExactMatricesV3 {
    psi: Vec<Vec<f64>>,
    phi: Vec<Vec<f64>>,
    theta: Vec<Vec<f64>>,
    sigma: Vec<Vec<f64>>,
    implied_means: Vec<f64>,
    row_values: Vec<f64>,
    structural_spectral_radius: f64,
}

/// Resolve the independent optimizer-dimension membership used by the exact
/// plan. This identity is covariance-independent: covariance values affect a
/// dimension's default start, but equality labels alone determine how free
/// parameter rows collapse into optimizer dimensions.
pub(crate) fn exact_free_dimension_members_v3(
    rows: &[CbsemExactParameterRowV3],
) -> Result<BTreeMap<String, Vec<usize>>, CbsemExactParameterTableErrorV3> {
    let mut members = BTreeMap::<String, Vec<usize>>::new();
    for (row_index, row) in rows.iter().enumerate() {
        if let CbsemExactParameterSpecificationV3::Free { equality_label, .. } = &row.specification
        {
            let key = match equality_label {
                Some(label) if label.trim().is_empty() => {
                    return Err(CbsemExactParameterTableErrorV3::EmptyEqualityLabel(
                        row.stable_id.clone(),
                    ));
                }
                Some(label) => label.trim().to_string(),
                None => format!("parameter:{}", row.stable_id),
            };
            members.entry(key).or_default().push(row_index);
        }
    }
    for (key, indices) in &members {
        if !key.starts_with("parameter:") && indices.len() < 2 {
            return Err(CbsemExactParameterTableErrorV3::SingletonEqualityLabel(
                key.clone(),
            ));
        }
    }
    for (key, indices) in &members {
        let families = indices
            .iter()
            .map(|index| rows[*index].target.family().to_string())
            .collect::<HashSet<_>>();
        if families.len() != 1 {
            let mut families = families.into_iter().collect::<Vec<_>>();
            families.sort();
            return Err(CbsemExactParameterTableErrorV3::EqualityFamilyMismatch {
                label: key.clone(),
                families,
            });
        }
    }
    Ok(members)
}

#[cfg(test)]
mod exact_free_dimension_membership_tests {
    use super::*;

    fn free_row(
        stable_id: &str,
        target: CbsemExactParameterTargetV3,
        equality_label: Option<&str>,
    ) -> CbsemExactParameterRowV3 {
        CbsemExactParameterRowV3 {
            stable_id: stable_id.into(),
            name: stable_id.into(),
            target,
            specification: CbsemExactParameterSpecificationV3::Free {
                start: None,
                lower: None,
                upper: None,
                equality_label: equality_label.map(str::to_owned),
            },
        }
    }

    #[test]
    fn equality_membership_collapses_free_rows_into_fewer_optimizer_dimensions() {
        let rows = vec![
            free_row(
                "residual:x",
                CbsemExactParameterTargetV3::ResidualVariance {
                    indicator: "x".into(),
                },
                Some(" equal-residuals "),
            ),
            free_row(
                "residual:y",
                CbsemExactParameterTargetV3::ResidualVariance {
                    indicator: "y".into(),
                },
                Some("equal-residuals"),
            ),
            free_row(
                "loading:z",
                CbsemExactParameterTargetV3::Loading {
                    construct: "factor".into(),
                    indicator: "z".into(),
                },
                None,
            ),
        ];

        let members = exact_free_dimension_members_v3(&rows).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(members.len(), 2);
        assert_eq!(members["equal-residuals"], vec![0, 1]);
        assert_eq!(members["parameter:loading:z"], vec![2]);
    }

    #[test]
    fn unlabeled_free_rows_each_retain_one_optimizer_dimension() {
        let rows = vec![
            free_row(
                "residual:x",
                CbsemExactParameterTargetV3::ResidualVariance {
                    indicator: "x".into(),
                },
                None,
            ),
            free_row(
                "residual:y",
                CbsemExactParameterTargetV3::ResidualVariance {
                    indicator: "y".into(),
                },
                None,
            ),
        ];

        assert_eq!(exact_free_dimension_members_v3(&rows).unwrap().len(), 2);
    }

    #[test]
    fn invalid_equality_labels_retain_the_exact_plan_typed_errors() {
        let empty = vec![free_row(
            "residual:x",
            CbsemExactParameterTargetV3::ResidualVariance {
                indicator: "x".into(),
            },
            Some("   "),
        )];
        assert_eq!(
            exact_free_dimension_members_v3(&empty).unwrap_err(),
            CbsemExactParameterTableErrorV3::EmptyEqualityLabel("residual:x".into())
        );

        let singleton = vec![free_row(
            "residual:x",
            CbsemExactParameterTargetV3::ResidualVariance {
                indicator: "x".into(),
            },
            Some("only-one"),
        )];
        assert_eq!(
            exact_free_dimension_members_v3(&singleton).unwrap_err(),
            CbsemExactParameterTableErrorV3::SingletonEqualityLabel("only-one".into())
        );

        let mixed = vec![
            free_row(
                "residual:x",
                CbsemExactParameterTargetV3::ResidualVariance {
                    indicator: "x".into(),
                },
                Some("mixed"),
            ),
            free_row(
                "loading:y",
                CbsemExactParameterTargetV3::Loading {
                    construct: "factor".into(),
                    indicator: "y".into(),
                },
                Some("mixed"),
            ),
        ];
        assert_eq!(
            exact_free_dimension_members_v3(&mixed).unwrap_err(),
            CbsemExactParameterTableErrorV3::EqualityFamilyMismatch {
                label: "mixed".into(),
                families: vec!["loading".into(), "variance".into()],
            }
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExpectedInformationConditionDiagnosticsV1 {
    scaled_one_norm_condition_number: f64,
    scaled_reciprocal_one_norm_condition_number: f64,
}

impl ExpectedInformationConditionDiagnosticsV1 {
    fn canonical_diagnostic_lines(self) -> [String; 2] {
        [
            format!(
                "{CBSEM_EXPECTED_INFORMATION_SCALED_ONE_NORM_CONDITION_DIAGNOSTIC_V1}={:.17e}",
                self.scaled_one_norm_condition_number
            ),
            format!(
                "{CBSEM_EXPECTED_INFORMATION_SCALED_RECIPROCAL_ONE_NORM_CONDITION_DIAGNOSTIC_V1}={:.17e}",
                self.scaled_reciprocal_one_norm_condition_number
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ExactDimensionInferenceV1 {
    standard_errors: Vec<f64>,
    expected_information_condition: Option<ExpectedInformationConditionDiagnosticsV1>,
}

#[derive(Debug, Clone, PartialEq)]
struct ScaledInformationInverseV1 {
    inverse_information: Vec<Vec<f64>>,
    condition: ExpectedInformationConditionDiagnosticsV1,
}

pub(crate) struct CbsemExactParameterTableResultV4 {
    pub analysis: CbsemAnalysis,
    pub implied_means: Vec<f64>,
}

pub(crate) fn estimate_cbsem_ml_exact_parameter_table_v3_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    sample_size: usize,
    input_kind: &str,
    parameter_rows: &[CbsemExactParameterRowV3],
    score_lm_eligible: bool,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemAnalysis, CbsemExactParameterTableErrorV3> {
    estimate_cbsem_ml_exact_parameter_table_with_control(
        model,
        indicator_names,
        sample_covariance,
        None,
        sample_size,
        input_kind,
        parameter_rows,
        CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
        score_lm_eligible,
        should_cancel,
        progress,
    )
    .map(|result| result.analysis)
}

/// Minimal exact-v3 optimizer result used by the outer case-bootstrap
/// scheduler. The established path excludes expected-information standard
/// errors; the explicit studentization prerequisite may attach their atomic
/// outcome. Both paths exclude standardized estimates, fit/RMSEA, score/LM,
/// and every nested resampling family. A converged, admissible ML point
/// estimate is therefore retained when replicate-level information is
/// singular.
pub(crate) struct CbsemExactParameterTablePointRefitV1 {
    pub method_version: String,
    pub free_parameters: Vec<CbsemExactCaseBootstrapParameterEstimateV1>,
    pub standard_errors: Option<CbsemExactCaseBootstrapRefitStandardErrorsV1>,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
}

/// Minimal two-group joint-ML fit used only by the isolated exact
/// configural-to-metric prerequisite. It deliberately omits fit-index,
/// standard-error, score/LM, resampling, archive, and product payloads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CbsemExactTwoGroupParameterPointV1 {
    pub parameter_id: String,
    pub estimate: f64,
    pub fixed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CbsemExactTwoGroupJointFitV1 {
    pub dimension_count: usize,
    pub initialization_iterations: u32,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
    pub group_objectives: Vec<f64>,
    pub group_parameter_estimates: Vec<Vec<CbsemExactTwoGroupParameterPointV1>>,
    pub group_implied_covariances: Vec<Vec<Vec<f64>>>,
}

/// Fits two covariance-structure CFA groups in one optimizer. Configural
/// dimensions are namespaced by group. In metric mode, and only in metric
/// mode, corresponding free loading dimensions are unioned across groups.
/// Fixed marker loadings never enter the union. Because each group's existing
/// equality labels have already collapsed rows into local dimensions, the
/// cross-group union preserves and transitively extends those equalities.
#[allow(clippy::too_many_arguments)]
pub(crate) fn estimate_cbsem_ml_exact_two_group_joint_v1_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariances: [&[Vec<f64>]; 2],
    sample_sizes: [usize; 2],
    parameter_rows: &[CbsemExactParameterRowV3],
    metric_loadings_equal: bool,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactTwoGroupJointFitV1, CbsemExactParameterTableErrorV3> {
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    if sample_sizes.into_iter().any(|sample_size| sample_size == 0) {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "two-group exact ML requires positive group sample sizes".into(),
        ));
    }
    let plans = [
        compile_exact_plan(
            model,
            indicator_names,
            sample_covariances[0],
            None,
            parameter_rows,
        )?,
        compile_exact_plan(
            model,
            indicator_names,
            sample_covariances[1],
            None,
            parameter_rows,
        )?,
    ];
    let local_dimensions = plans[0].dimensions.len();
    if local_dimensions == 0 || plans[1].dimensions.len() != local_dimensions {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "two-group exact ML requires the same non-empty local dimension surface".into(),
        ));
    }
    for dimension in 0..local_dimensions {
        if plans[0].dimensions[dimension].key != plans[1].dimensions[dimension].key
            || !open_domains_bit_equal(
                plans[0].dimensions[dimension].domain,
                plans[1].dimensions[dimension].domain,
            )
        {
            return Err(CbsemExactParameterTableErrorV3::Numerical(format!(
                "two-group local dimension {dimension} differs in identity or open domain"
            )));
        }
    }
    let local_starts = [
        plans[0]
            .dimensions
            .iter()
            .map(|dimension| dimension.domain.raw_from_value(dimension.start_value))
            .collect::<Vec<_>>(),
        plans[1]
            .dimensions
            .iter()
            .map(|dimension| dimension.domain.raw_from_value(dimension.start_value))
            .collect::<Vec<_>>(),
    ];
    for group in 0..2 {
        let initial = matrices_from_raw(&plans[group], &local_starts[group]).map_err(|error| {
            map_matrix_evaluation_error(error, "two-group declared start", true)
        })?;
        let rank = local_identification_rank(
            &plans[group],
            &local_starts[group],
            &initial.sigma,
            &initial.implied_means,
            false,
        )?;
        if rank < local_dimensions {
            return Err(CbsemExactParameterTableErrorV3::LocalUnderidentification {
                rank,
                free_parameters: local_dimensions,
            });
        }
    }

    let mut local_solutions = Vec::with_capacity(2);
    let mut initialization_iterations = 0_u32;
    for group in 0..2 {
        let local_objective = |raw: &[f64]| -> Result<f64, MatrixEvaluationErrorV3> {
            matrices_from_raw(&plans[group], raw).and_then(|matrices| {
                maximum_likelihood_discrepancy(sample_covariances[group], &matrices.sigma)
            })
        };
        let initialized = minimize_exact_objective(
            &local_starts[group],
            &local_objective,
            should_cancel,
            &|_, _| {},
        )?;
        if !initialized.converged {
            return Err(optimizer_nonconvergence_error(
                &initialized,
                "two-group group-local initialization",
            ));
        }
        initialization_iterations =
            initialization_iterations.saturating_add(initialized.iterations);
        local_solutions.push(initialized.parameters);
    }

    let node_count = 2 * local_dimensions;
    let mut parent = (0..node_count).collect::<Vec<_>>();
    if metric_loadings_equal {
        if plans[0].rows.len() != plans[1].rows.len() {
            return Err(CbsemExactParameterTableErrorV3::Numerical(
                "two-group exact plans have inconsistent parameter rows".into(),
            ));
        }
        for (left, right) in plans[0].rows.iter().zip(&plans[1].rows) {
            if left.stable_id != right.stable_id
                || left.target_description != right.target_description
            {
                return Err(CbsemExactParameterTableErrorV3::Numerical(
                    "two-group exact plans have inconsistent parameter identity or targets".into(),
                ));
            }
            if !matches!(
                left.target_description,
                CbsemExactParameterTargetV3::Loading { .. }
            ) {
                continue;
            }
            match (&left.specification, &right.specification) {
                (
                    ResolvedSpecificationV3::Free { dimension: left },
                    ResolvedSpecificationV3::Free { dimension: right },
                ) => union_nodes(&mut parent, *left, local_dimensions + *right),
                (
                    ResolvedSpecificationV3::Fixed { value: left },
                    ResolvedSpecificationV3::Fixed { value: right },
                ) if left.to_bits() == right.to_bits() => {}
                _ => {
                    return Err(CbsemExactParameterTableErrorV3::Numerical(format!(
                        "corresponding loading {} has inconsistent fixed/free status or value",
                        left.stable_id
                    )));
                }
            }
        }
    }
    for node in 0..node_count {
        let root = find_node(&mut parent, node);
        parent[node] = root;
    }
    let mut root_to_global = BTreeMap::<usize, usize>::new();
    let mut local_to_global = vec![0usize; node_count];
    for node in 0..node_count {
        let next = root_to_global.len();
        let global = *root_to_global.entry(parent[node]).or_insert(next);
        local_to_global[node] = global;
    }
    let global_dimensions = root_to_global.len();
    let mut global_start_sum = vec![0.0_f64; global_dimensions];
    let mut global_start_count = vec![0_usize; global_dimensions];
    let mut global_domain = vec![None::<OpenDomainV3>; global_dimensions];
    for group in 0..2 {
        for local in 0..local_dimensions {
            let node = group * local_dimensions + local;
            let global = local_to_global[node];
            let domain = plans[group].dimensions[local].domain;
            let start = local_solutions[group][local];
            if let Some(existing) = global_domain[global] {
                if !open_domains_bit_equal(existing, domain) {
                    return Err(CbsemExactParameterTableErrorV3::Numerical(format!(
                        "metric loading union {global} has an incompatible open domain"
                    )));
                }
            } else {
                global_domain[global] = Some(domain);
            }
            global_start_sum[global] += start;
            global_start_count[global] += 1;
        }
    }
    let start = global_start_sum
        .into_iter()
        .zip(global_start_count)
        .map(|(sum, count)| sum / count as f64)
        .collect::<Vec<_>>();
    let total_sample_size = sample_sizes[0] + sample_sizes[1];
    let objective = |raw: &[f64]| -> Result<f64, MatrixEvaluationErrorV3> {
        let mut weighted = 0.0;
        for group in 0..2 {
            let local_raw = (0..local_dimensions)
                .map(|local| raw[local_to_global[group * local_dimensions + local]])
                .collect::<Vec<_>>();
            let matrices = matrices_from_raw(&plans[group], &local_raw)?;
            let discrepancy =
                maximum_likelihood_discrepancy(sample_covariances[group], &matrices.sigma)?;
            weighted += sample_sizes[group] as f64 / total_sample_size as f64 * discrepancy;
        }
        if weighted.is_finite() && weighted >= 0.0 {
            Ok(weighted)
        } else {
            Err(MatrixEvaluationErrorV3::Numerical(
                "two-group weighted ML objective is not finite and nonnegative".into(),
            ))
        }
    };
    let optimized = minimize_exact_objective(&start, &objective, should_cancel, progress)?;
    if !optimized.converged {
        return Err(optimizer_nonconvergence_error(
            &optimized,
            if metric_loadings_equal {
                "two-group metric joint ML"
            } else {
                "two-group configural joint ML"
            },
        ));
    }
    let mut group_objectives = Vec::with_capacity(2);
    let mut group_parameter_estimates = Vec::with_capacity(2);
    let mut group_implied_covariances = Vec::with_capacity(2);
    for group in 0..2 {
        let local_raw = (0..local_dimensions)
            .map(|local| optimized.parameters[local_to_global[group * local_dimensions + local]])
            .collect::<Vec<_>>();
        let matrices = matrices_from_raw(&plans[group], &local_raw)
            .map_err(|error| map_matrix_evaluation_error(error, "two-group solution", false))?;
        group_objectives.push(
            maximum_likelihood_discrepancy(sample_covariances[group], &matrices.sigma).map_err(
                |error| map_matrix_evaluation_error(error, "two-group fitted objective", false),
            )?,
        );
        group_parameter_estimates.push(
            plans[group]
                .rows
                .iter()
                .enumerate()
                .map(|(row_index, row)| CbsemExactTwoGroupParameterPointV1 {
                    parameter_id: row.stable_id.clone(),
                    estimate: matrices.row_values[row_index],
                    fixed: matches!(row.specification, ResolvedSpecificationV3::Fixed { .. }),
                })
                .collect(),
        );
        group_implied_covariances.push(matrices.sigma);
    }
    Ok(CbsemExactTwoGroupJointFitV1 {
        dimension_count: global_dimensions,
        initialization_iterations,
        iterations: optimized.iterations,
        objective: optimized.objective,
        gradient_norm: optimized.gradient_norm,
        group_objectives,
        group_parameter_estimates,
        group_implied_covariances,
    })
}

fn option_f64_bits_equal(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

fn open_domains_bit_equal(left: OpenDomainV3, right: OpenDomainV3) -> bool {
    option_f64_bits_equal(left.lower, right.lower) && option_f64_bits_equal(left.upper, right.upper)
}

fn find_node(parent: &mut [usize], node: usize) -> usize {
    if parent[node] != node {
        parent[node] = find_node(parent, parent[node]);
    }
    parent[node]
}

fn union_nodes(parent: &mut [usize], left: usize, right: usize) {
    let left = find_node(parent, left);
    let right = find_node(parent, right);
    if left != right {
        let (first, second) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        parent[second] = first;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    parameter_rows: &[CbsemExactParameterRowV3],
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactParameterTablePointRefitV1, CbsemExactParameterTableErrorV3> {
    estimate_cbsem_ml_exact_parameter_table_v3_point_refit_impl_with_control(
        model,
        indicator_names,
        sample_covariance,
        parameter_rows,
        None,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn estimate_cbsem_ml_exact_parameter_table_v3_point_refit_with_analytic_standard_errors_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    parameter_rows: &[CbsemExactParameterRowV3],
    sample_size: usize,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactParameterTablePointRefitV1, CbsemExactParameterTableErrorV3> {
    estimate_cbsem_ml_exact_parameter_table_v3_point_refit_impl_with_control(
        model,
        indicator_names,
        sample_covariance,
        parameter_rows,
        Some(sample_size),
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn estimate_cbsem_ml_exact_parameter_table_v3_point_refit_impl_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    parameter_rows: &[CbsemExactParameterRowV3],
    standard_error_sample_size: Option<usize>,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactParameterTablePointRefitV1, CbsemExactParameterTableErrorV3> {
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    let plan = compile_exact_plan(
        model,
        indicator_names,
        sample_covariance,
        None,
        parameter_rows,
    )?;
    let observed_moments = indicator_names.len() * (indicator_names.len() + 1) / 2;
    if plan.dimensions.len() > observed_moments {
        return Err(CbsemExactParameterTableErrorV3::NegativeDegreesOfFreedom {
            free_parameters: plan.dimensions.len(),
            observed_moments,
        });
    }
    let start = plan
        .dimensions
        .iter()
        .map(|dimension| dimension.domain.raw_from_value(dimension.start_value))
        .collect::<Vec<_>>();
    let initial = matrices_from_raw(&plan, &start)
        .map_err(|error| map_matrix_evaluation_error(error, "declared start", true))?;
    let rank =
        local_identification_rank(&plan, &start, &initial.sigma, &initial.implied_means, false)?;
    if rank < plan.dimensions.len() {
        return Err(CbsemExactParameterTableErrorV3::LocalUnderidentification {
            rank,
            free_parameters: plan.dimensions.len(),
        });
    }
    let objective = |raw: &[f64]| -> Result<f64, MatrixEvaluationErrorV3> {
        matrices_from_raw(&plan, raw)
            .and_then(|matrices| maximum_likelihood_discrepancy(sample_covariance, &matrices.sigma))
    };
    let optimized = minimize_exact_objective(&start, &objective, should_cancel, progress)?;
    if !optimized.converged {
        return Err(optimizer_nonconvergence_error(
            &optimized,
            "bootstrap covariance point refit",
        ));
    }
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    let matrices = matrices_from_raw(&plan, &optimized.parameters)
        .map_err(|error| map_matrix_evaluation_error(error, "converged solution", false))?;
    let free_parameters = plan
        .rows
        .iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            matches!(&row.specification, ResolvedSpecificationV3::Free { .. }).then(|| {
                CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: row.stable_id.clone(),
                    estimate: matrices.row_values[row_index],
                }
            })
        })
        .collect::<Vec<_>>();
    if free_parameters
        .iter()
        .any(|parameter| !parameter.estimate.is_finite())
        || !optimized.objective.is_finite()
        || !optimized.gradient_norm.is_finite()
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "bootstrap point refit produced a non-finite estimate, objective, or gradient".into(),
        ));
    }
    let standard_errors = if let Some(sample_size) = standard_error_sample_size {
        if should_cancel() {
            return Err(CbsemExactParameterTableErrorV3::Cancelled);
        }
        let receipt = exact_refit_standard_errors_v1(
            &plan,
            &optimized.parameters,
            &matrices.sigma,
            sample_size,
        );
        if should_cancel() {
            return Err(CbsemExactParameterTableErrorV3::Cancelled);
        }
        Some(receipt)
    } else {
        None
    };
    Ok(CbsemExactParameterTablePointRefitV1 {
        method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
        free_parameters,
        standard_errors,
        iterations: optimized.iterations,
        objective: optimized.objective,
        gradient_norm: optimized.gradient_norm,
    })
}

fn exact_refit_standard_errors_v1(
    plan: &ExactPlanV3,
    raw: &[f64],
    sigma: &[Vec<f64>],
    sample_size: usize,
) -> CbsemExactCaseBootstrapRefitStandardErrorsV1 {
    exact_refit_standard_errors_from_dimension_inference_v1(
        plan,
        raw,
        exact_dimension_standard_errors(plan, raw, sigma, sample_size),
    )
}

fn exact_refit_standard_errors_from_dimension_inference_v1(
    plan: &ExactPlanV3,
    raw: &[f64],
    inference: Result<ExactDimensionInferenceV1, CbsemExactParameterTableErrorV3>,
) -> CbsemExactCaseBootstrapRefitStandardErrorsV1 {
    let outcome = match inference {
        Ok(inference) => {
            if inference.standard_errors.len() != plan.dimensions.len()
                || raw.len() != plan.dimensions.len()
            {
                return CbsemExactCaseBootstrapRefitStandardErrorsV1 {
                    method_version:
                        CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1.into(),
                    outcome:
                        CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                            reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError,
                        },
                };
            }
            let parameters = plan
                .rows
                .iter()
                .filter_map(|row| match row.specification {
                    ResolvedSpecificationV3::Fixed { .. } => None,
                    ResolvedSpecificationV3::Free { dimension } => Some(
                        CbsemExactCaseBootstrapParameterStandardErrorV1 {
                            parameter_id: row.stable_id.clone(),
                            standard_error: inference.standard_errors[dimension]
                                * plan.dimensions[dimension]
                                    .domain
                                    .derivative(raw[dimension])
                                    .abs(),
                        },
                    ),
                })
                .collect::<Vec<_>>();
            if parameters.is_empty()
                || parameters.iter().any(|parameter| {
                    !parameter.standard_error.is_finite() || parameter.standard_error <= 0.0
                })
            {
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                    reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError,
                }
            } else {
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
                    information_method:
                        CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1.into(),
                    parameters,
                }
            }
        }
        Err(CbsemExactParameterTableErrorV3::InformationMatrixSingular) => {
            CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation,
            }
        }
        Err(CbsemExactParameterTableErrorV3::InformationMatrixNotPositiveDefinite { .. }) => {
            CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InformationNotPositiveDefinite,
            }
        }
        Err(CbsemExactParameterTableErrorV3::InformationVarianceInvalid { .. }) => {
            CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError,
            }
        }
        Err(CbsemExactParameterTableErrorV3::GradientUnavailable(_)) => {
            CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::DerivativeUnavailable,
            }
        }
        Err(_) => CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
            reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::NumericalInformationFailure,
        },
    };
    CbsemExactCaseBootstrapRefitStandardErrorsV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1.into(),
        outcome,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn estimate_cbsem_ml_exact_parameter_table_v4_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    sample_means: &[f64],
    sample_size: usize,
    input_kind: &str,
    parameter_rows: &[CbsemExactParameterRowV3],
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactParameterTableResultV4, CbsemExactParameterTableErrorV3> {
    estimate_cbsem_ml_exact_parameter_table_with_control(
        model,
        indicator_names,
        sample_covariance,
        Some(sample_means),
        sample_size,
        input_kind,
        parameter_rows,
        CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
        false,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn estimate_cbsem_ml_exact_parameter_table_with_control(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    sample_means: Option<&[f64]>,
    sample_size: usize,
    input_kind: &str,
    parameter_rows: &[CbsemExactParameterRowV3],
    method_version: &'static str,
    score_lm_eligible: bool,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<CbsemExactParameterTableResultV4, CbsemExactParameterTableErrorV3> {
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    if let Some(means) = sample_means {
        if means.len() != indicator_names.len() || means.iter().any(|value| !value.is_finite()) {
            return Err(CbsemExactParameterTableErrorV3::InvalidObservedMeans {
                expected: indicator_names.len(),
                actual: means.len(),
            });
        }
    }
    let plan = compile_exact_plan(
        model,
        indicator_names,
        sample_covariance,
        sample_means,
        parameter_rows,
    )?;
    let covariance_moments = indicator_names.len() * (indicator_names.len() + 1) / 2;
    let observed_moments = covariance_moments
        + if sample_means.is_some() {
            indicator_names.len()
        } else {
            0
        };
    if plan.dimensions.len() > observed_moments {
        return Err(CbsemExactParameterTableErrorV3::NegativeDegreesOfFreedom {
            free_parameters: plan.dimensions.len(),
            observed_moments,
        });
    }
    let start = plan
        .dimensions
        .iter()
        .map(|dimension| dimension.domain.raw_from_value(dimension.start_value))
        .collect::<Vec<_>>();
    let initial = matrices_from_raw(&plan, &start)
        .map_err(|error| map_matrix_evaluation_error(error, "declared start", true))?;
    let rank = local_identification_rank(
        &plan,
        &start,
        &initial.sigma,
        &initial.implied_means,
        sample_means.is_some(),
    )?;
    if rank < plan.dimensions.len() {
        return Err(CbsemExactParameterTableErrorV3::LocalUnderidentification {
            rank,
            free_parameters: plan.dimensions.len(),
        });
    }
    let objective = |raw: &[f64]| -> Result<f64, MatrixEvaluationErrorV3> {
        matrices_from_raw(&plan, raw).and_then(|matrices| {
            let covariance = maximum_likelihood_discrepancy(sample_covariance, &matrices.sigma)?;
            let mean = sample_means.map_or(Ok(0.0), |sample_means| {
                mean_structure_discrepancy(sample_means, &matrices.implied_means, &matrices.sigma)
            })?;
            Ok(covariance + mean)
        })
    };
    // The covariance parameters start on their documented generic scale while
    // raw location starts are already close to the observed means. Optimizing
    // all dimensions together from that mixed-scale point can make BFGS spend
    // its line-search budget on covariance curvature before it has a useful
    // inverse-Hessian scale for the mean block. V4 therefore performs a
    // covariance-only initialization from the same declared starts, followed
    // by the contracted joint covariance+mean objective. This is an
    // initializer only: convergence, inference, fit, and reported estimates
    // are all evaluated at the joint-ML solution.
    let (optimization_start, initialization_iterations) = if sample_means.is_some() {
        let covariance_objective = |raw: &[f64]| -> Result<f64, MatrixEvaluationErrorV3> {
            matrices_from_raw(&plan, raw).and_then(|matrices| {
                maximum_likelihood_discrepancy(sample_covariance, &matrices.sigma)
            })
        };
        let initialization_progress = |completed: u64, total: u64| {
            progress(completed, total.saturating_mul(2));
        };
        let initialized = minimize_exact_objective(
            &start,
            &covariance_objective,
            should_cancel,
            &initialization_progress,
        )?;
        if !initialized.converged {
            return Err(optimizer_nonconvergence_error(
                &initialized,
                "covariance initialization",
            ));
        }
        (initialized.parameters, initialized.iterations)
    } else {
        (start, 0)
    };
    let optimized = if sample_means.is_some() {
        let joint_progress = |completed: u64, total: u64| {
            progress(total.saturating_add(completed), total.saturating_mul(2));
        };
        minimize_exact_objective(
            &optimization_start,
            &objective,
            should_cancel,
            &joint_progress,
        )?
    } else {
        minimize_exact_objective(&optimization_start, &objective, should_cancel, progress)?
    };
    if !optimized.converged {
        return Err(optimizer_nonconvergence_error(
            &optimized,
            if sample_means.is_some() {
                "joint covariance-mean estimation"
            } else {
                "covariance estimation"
            },
        ));
    }
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    let matrices = matrices_from_raw(&plan, &optimized.parameters)
        .map_err(|error| map_matrix_evaluation_error(error, "converged solution", false))?;
    let objective_value = objective(&optimized.parameters)
        .map_err(|error| CbsemExactParameterTableErrorV3::Numerical(error.to_string()))?;
    let gradient = numerical_gradient(
        &optimized.parameters,
        objective_value,
        &objective,
        should_cancel,
    )?;
    let gradient_norm = vector_norm(&gradient);
    let dimension_inference = if sample_means.is_some() {
        exact_dimension_standard_errors_with_means(
            &plan,
            &optimized.parameters,
            &matrices.sigma,
            &matrices.implied_means,
            sample_size,
        )?
    } else {
        exact_dimension_standard_errors(&plan, &optimized.parameters, &matrices.sigma, sample_size)?
    };
    let parameters = exact_parameters(
        &plan,
        &matrices,
        &optimized.parameters,
        &dimension_inference.standard_errors,
    );
    let standardized = exact_standardized_parameters(&plan, &matrices, &parameters);
    let score_lm = if score_lm_eligible && sample_means.is_none() && model.paths.is_empty() {
        Some(exact_cfa_score_lm_bundle_v1(
            &plan,
            &optimized.parameters,
            sample_covariance,
            &matrices.sigma,
            sample_size,
        ))
    } else {
        None
    };
    let residual = subtract_matrices(sample_covariance, &matrices.sigma);
    let residual_correlation = residual_correlation_matrix(&residual, sample_covariance);
    let degrees_of_freedom = observed_moments as i64 - plan.dimensions.len() as i64;
    let chi_square = (sample_size as f64 * objective_value).max(0.0);
    let (baseline_chi_square, baseline_degrees_of_freedom) =
        baseline_fit(sample_covariance, sample_size)?;
    let fit = exact_fit_indices(
        chi_square,
        degrees_of_freedom,
        baseline_chi_square,
        baseline_degrees_of_freedom,
        objective_value,
        plan.dimensions.len(),
        sample_size,
        matrix_srmr(sample_covariance, &matrices.sigma),
    )?;
    let equality_dimensions = plan
        .dimensions
        .iter()
        .filter(|dimension| !dimension.key.starts_with("parameter:"))
        .count();
    let analysis = CbsemAnalysis {
        method_version: method_version.into(),
        model_type: if model.paths.is_empty() {
            "cfa".into()
        } else {
            "sem".into()
        },
        estimator: "ml".into(),
        input: input_kind.into(),
        mean_structure: sample_means.is_some(),
        converged: true,
        iterations: initialization_iterations.saturating_add(optimized.iterations),
        objective: objective_value,
        gradient_norm,
        sample_size,
        parameters,
        standardized,
        implied_covariance: matrix_cells(indicator_names, &matrices.sigma),
        residual_covariance: matrix_cells(indicator_names, &residual),
        residual_correlation: matrix_cells(indicator_names, &residual_correlation),
        fit,
        modification_indices: Vec::new(),
        score_lm,
        bootstrap: None,
        bootstrap_v2: None,
        exact_case_bootstrap: None,
        exact_case_bootstrap_studentized: None,
        exact_case_bootstrap_bca: None,
        multigroup: None,
        diagnostics: {
            let mut diagnostics = vec![
                format!("Exact compiled parameter rows: {}", plan.rows.len()),
                format!("Independent free dimensions: {}", plan.dimensions.len()),
                format!("Equality-constrained dimensions: {equality_dimensions}"),
                format!("RMSEA interval method: {CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1}."),
                format!(
                    "Structural-system stability: admissible (spectral radius {:.12}; required < {:.12}).",
                    matrices.structural_spectral_radius,
                    CBSEM_GENERAL_ML_MAXIMUM_SPECTRAL_RADIUS_V1
                ),
                "Undeclared latent, disturbance, and residual covariance cells were fixed to zero."
                    .into(),
            ];
            if sample_means.is_some() {
                diagnostics.push(format!(
                    "Covariance-structure initialization iterations: {initialization_iterations}"
                ));
                diagnostics.push(
                    "Observed means and the ML covariance were computed from the same listwise-complete raw rows."
                        .into(),
                );
            }
            if let Some(condition) = dimension_inference.expected_information_condition {
                diagnostics.extend(condition.canonical_diagnostic_lines());
            }
            diagnostics
        },
        warnings: vec![if sample_means.is_some() {
            "CB-SEM exact parameter-table v4 is an Internal/Labs scientific engine slice. Its raw-data mean structure is limited to continuous single-group CFA with explicit observed intercepts and marker-identified latent means; matrix means, structural intercepts, groups, resampling, effects coding, and modification-index inference remain blocked."
                .into()
        } else {
            "CB-SEM exact parameter-table v3 is an Internal/Labs scientific engine slice. It executes the immutable Recipe-v4 parameter table; genuine score/LM inference is limited to declared fixed-zero residual covariances in covariance-structure CFA, while means, groups, resampling, effects coding, and broader modification-index inference remain blocked."
                .into()
        }],
    };
    Ok(CbsemExactParameterTableResultV4 {
        analysis,
        implied_means: matrices.implied_means,
    })
}

fn compile_exact_plan(
    model: &ModelSpec,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    sample_means: Option<&[f64]>,
    input_rows: &[CbsemExactParameterRowV3],
) -> Result<ExactPlanV3, CbsemExactParameterTableErrorV3> {
    let construct_ids = model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut rows = input_rows.to_vec();
    rows.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut targets = HashSet::new();
    for row in &rows {
        if !ids.insert(row.stable_id.as_str()) {
            return Err(CbsemExactParameterTableErrorV3::DuplicateParameterId(
                row.stable_id.clone(),
            ));
        }
        if !names.insert(row.name.as_str()) {
            return Err(CbsemExactParameterTableErrorV3::DuplicateParameterName(
                row.name.clone(),
            ));
        }
        if !targets.insert(row.target.clone()) {
            return Err(CbsemExactParameterTableErrorV3::DuplicateParameterTarget(
                row.name.clone(),
            ));
        }
    }

    let members = exact_free_dimension_members_v3(&rows)?;

    let mut dimensions = Vec::new();
    let mut dimension_by_row = HashMap::new();
    for (key, indices) in members {
        let mut lower = None::<f64>;
        let mut upper = None::<f64>;
        let mut explicit_starts = Vec::new();
        for index in &indices {
            let row = &rows[*index];
            let CbsemExactParameterSpecificationV3::Free {
                start,
                lower: declared_lower,
                upper: declared_upper,
                ..
            } = row.specification
            else {
                unreachable!("members contain only free rows")
            };
            if [start, declared_lower, declared_upper]
                .into_iter()
                .flatten()
                .any(|value| !value.is_finite())
            {
                return Err(CbsemExactParameterTableErrorV3::NonFiniteParameterValue {
                    parameter: row.stable_id.clone(),
                });
            }
            if declared_lower
                .zip(declared_upper)
                .is_some_and(|(left, right)| left >= right)
            {
                return Err(CbsemExactParameterTableErrorV3::InvalidBounds {
                    parameter: row.stable_id.clone(),
                });
            }
            let effective_lower = if row.target.is_variance() {
                Some(declared_lower.unwrap_or(0.0).max(0.0))
            } else {
                declared_lower
            };
            lower = match (lower, effective_lower) {
                (Some(current), Some(candidate)) => Some(current.max(candidate)),
                (None, candidate) => candidate,
                (current, None) => current,
            };
            upper = match (upper, declared_upper) {
                (Some(current), Some(candidate)) => Some(current.min(candidate)),
                (None, candidate) => candidate,
                (current, None) => current,
            };
            if let Some(start) = start {
                explicit_starts.push((row.stable_id.clone(), start));
            }
        }
        if lower.zip(upper).is_some_and(|(left, right)| left >= right) {
            return Err(CbsemExactParameterTableErrorV3::EqualityBoundsEmpty { label: key });
        }
        if let Some((_, reference)) = explicit_starts.first() {
            if explicit_starts.iter().skip(1).any(|(_, value)| {
                (value - reference).abs() > 1e-12 * value.abs().max(reference.abs()).max(1.0)
            }) {
                return Err(CbsemExactParameterTableErrorV3::EqualityStartConflict { label: key });
            }
        }
        let domain = OpenDomainV3 { lower, upper };
        let default = indices
            .iter()
            .map(|index| {
                rows[*index]
                    .target
                    .default_start(&indicator_index, sample_covariance, sample_means)
            })
            .sum::<f64>()
            / indices.len() as f64;
        let start_value = explicit_starts
            .first()
            .map(|(_, value)| *value)
            .unwrap_or_else(|| feasible_default(domain, default));
        if !domain.contains(start_value) {
            return Err(CbsemExactParameterTableErrorV3::StartOutsideBounds {
                parameter: explicit_starts
                    .first()
                    .map(|(parameter, _)| parameter.clone())
                    .unwrap_or_else(|| rows[indices[0]].stable_id.clone()),
                start: start_value,
                lower,
                upper,
            });
        }
        let dimension = dimensions.len();
        for index in indices {
            dimension_by_row.insert(index, dimension);
        }
        dimensions.push(FreeDimensionV3 {
            key,
            domain,
            start_value,
        });
    }

    let mut resolved_rows = Vec::with_capacity(rows.len());
    let mut factor_variances = HashSet::new();
    let mut residual_variances = HashSet::new();
    let mut observed_intercepts = HashSet::new();
    let mut latent_means = HashSet::new();
    for (row_index, row) in rows.into_iter().enumerate() {
        let target = resolve_target(
            &row.stable_id,
            &row.target,
            &construct_index,
            &indicator_index,
        )?;
        match target {
            ResolvedTargetV3::FactorVariance { factor } => {
                factor_variances.insert(factor);
            }
            ResolvedTargetV3::ResidualVariance { indicator } => {
                residual_variances.insert(indicator);
            }
            ResolvedTargetV3::ObservedIntercept { indicator } => {
                observed_intercepts.insert(indicator);
            }
            ResolvedTargetV3::LatentMean { factor } => {
                latent_means.insert(factor);
            }
            _ => {}
        }
        let specification = match row.specification {
            CbsemExactParameterSpecificationV3::Free { .. } => ResolvedSpecificationV3::Free {
                dimension: dimension_by_row[&row_index],
            },
            CbsemExactParameterSpecificationV3::Fixed { value } => {
                if !value.is_finite() {
                    return Err(CbsemExactParameterTableErrorV3::NonFiniteParameterValue {
                        parameter: row.stable_id,
                    });
                }
                if row.target.is_variance() && value <= 0.0 {
                    return Err(CbsemExactParameterTableErrorV3::FixedVarianceNonPositive {
                        parameter: row.stable_id,
                    });
                }
                ResolvedSpecificationV3::Fixed { value }
            }
        };
        resolved_rows.push(ResolvedRowV3 {
            stable_id: row.stable_id,
            name: row.name,
            target_description: row.target,
            target,
            specification,
        });
    }
    for (index, id) in construct_ids.iter().enumerate() {
        if !factor_variances.contains(&index) {
            return Err(CbsemExactParameterTableErrorV3::MissingFactorVariance(
                id.clone(),
            ));
        }
    }
    for (index, id) in indicator_names.iter().enumerate() {
        if !residual_variances.contains(&index) {
            return Err(CbsemExactParameterTableErrorV3::MissingResidualVariance(
                id.clone(),
            ));
        }
    }
    if sample_means.is_some() {
        for (index, id) in indicator_names.iter().enumerate() {
            if !observed_intercepts.contains(&index) {
                return Err(CbsemExactParameterTableErrorV3::MissingObservedIntercept(
                    id.clone(),
                ));
            }
        }
        for factor in latent_means {
            let construct = model
                .constructs
                .iter()
                .find(|construct| construct.id == construct_ids[factor])
                .ok_or_else(|| CbsemExactParameterTableErrorV3::UnknownTarget {
                    parameter: format!("{}~1", construct_ids[factor]),
                    kind: "factor".into(),
                    id: construct_ids[factor].clone(),
                })?;
            let marker = construct.indicators.first().ok_or_else(|| {
                CbsemExactParameterTableErrorV3::LatentMeanMarkerInterceptNotFixed {
                    factor: construct.id.clone(),
                    indicator: "<missing>".into(),
                }
            })?;
            let marker_index = indicator_index
                .get(marker.as_str())
                .copied()
                .ok_or_else(|| {
                    CbsemExactParameterTableErrorV3::LatentMeanMarkerInterceptNotFixed {
                        factor: construct.id.clone(),
                        indicator: marker.clone(),
                    }
                })?;
            let anchored = resolved_rows.iter().any(|row| {
                row.target
                    == ResolvedTargetV3::ObservedIntercept {
                        indicator: marker_index,
                    }
                    && matches!(row.specification, ResolvedSpecificationV3::Fixed { .. })
            });
            if !anchored {
                return Err(
                    CbsemExactParameterTableErrorV3::LatentMeanMarkerInterceptNotFixed {
                        factor: construct.id.clone(),
                        indicator: marker.clone(),
                    },
                );
            }
        }
    } else if !observed_intercepts.is_empty() || !latent_means.is_empty() {
        return Err(CbsemExactParameterTableErrorV3::MeanStructureDataRequired);
    }
    Ok(ExactPlanV3 {
        construct_ids,
        indicator_ids: indicator_names.to_vec(),
        dimensions,
        rows: resolved_rows,
    })
}

fn feasible_default(domain: OpenDomainV3, requested: f64) -> f64 {
    if domain.contains(requested) {
        return requested;
    }
    match (domain.lower, domain.upper) {
        (Some(lower), Some(upper)) => lower + (upper - lower) * 0.5,
        (Some(lower), None) => lower + lower.abs().max(1.0),
        (None, Some(upper)) => upper - upper.abs().max(1.0),
        (None, None) => requested,
    }
}

fn resolve_target(
    parameter: &str,
    target: &CbsemExactParameterTargetV3,
    construct_index: &HashMap<&str, usize>,
    indicator_index: &HashMap<&str, usize>,
) -> Result<ResolvedTargetV3, CbsemExactParameterTableErrorV3> {
    let construct = |id: &str| {
        construct_index.get(id).copied().ok_or_else(|| {
            CbsemExactParameterTableErrorV3::UnknownTarget {
                parameter: parameter.into(),
                kind: "factor".into(),
                id: id.into(),
            }
        })
    };
    let indicator = |id: &str| {
        indicator_index.get(id).copied().ok_or_else(|| {
            CbsemExactParameterTableErrorV3::UnknownTarget {
                parameter: parameter.into(),
                kind: "indicator".into(),
                id: id.into(),
            }
        })
    };
    Ok(match target {
        CbsemExactParameterTargetV3::Loading {
            construct: construct_id,
            indicator: indicator_id,
        } => ResolvedTargetV3::Loading {
            construct: construct(construct_id)?,
            indicator: indicator(indicator_id)?,
        },
        CbsemExactParameterTargetV3::Regression { source, target } => {
            ResolvedTargetV3::Regression {
                source: construct(source)?,
                target: construct(target)?,
            }
        }
        CbsemExactParameterTargetV3::FactorVariance { factor } => {
            ResolvedTargetV3::FactorVariance {
                factor: construct(factor)?,
            }
        }
        CbsemExactParameterTargetV3::FactorCovariance { left, right, .. } => {
            ResolvedTargetV3::FactorCovariance {
                left: construct(left)?,
                right: construct(right)?,
            }
        }
        CbsemExactParameterTargetV3::ResidualVariance { indicator: id } => {
            ResolvedTargetV3::ResidualVariance {
                indicator: indicator(id)?,
            }
        }
        CbsemExactParameterTargetV3::ResidualCovariance { left, right } => {
            ResolvedTargetV3::ResidualCovariance {
                left: indicator(left)?,
                right: indicator(right)?,
            }
        }
        CbsemExactParameterTargetV3::ObservedIntercept { indicator: id } => {
            ResolvedTargetV3::ObservedIntercept {
                indicator: indicator(id)?,
            }
        }
        CbsemExactParameterTargetV3::LatentMean { factor: id } => ResolvedTargetV3::LatentMean {
            factor: construct(id)?,
        },
    })
}

#[derive(Debug, Clone, thiserror::Error)]
enum MatrixEvaluationErrorV3 {
    #[error("latent/disturbance covariance is not positive definite")]
    LatentNotPositiveDefinite,
    #[error("residual covariance is not positive definite")]
    ResidualNotPositiveDefinite,
    #[error("implied covariance is not positive definite")]
    ImpliedNotPositiveDefinite,
    #[error(
        "structural coefficient matrix is unstable: spectral radius {spectral_radius} must be below {maximum}"
    )]
    StructuralSystemUnstable { spectral_radius: f64, maximum: f64 },
    #[error("structural stability eigendecomposition failed: {0}")]
    StructuralStabilityUnavailable(String),
    #[error("{0}")]
    Numerical(String),
}

fn map_matrix_evaluation_error(
    error: MatrixEvaluationErrorV3,
    stage: &str,
    initial: bool,
) -> CbsemExactParameterTableErrorV3 {
    match error {
        MatrixEvaluationErrorV3::LatentNotPositiveDefinite if initial => {
            CbsemExactParameterTableErrorV3::InitialLatentCovarianceNotPositiveDefinite
        }
        MatrixEvaluationErrorV3::ResidualNotPositiveDefinite if initial => {
            CbsemExactParameterTableErrorV3::InitialResidualCovarianceNotPositiveDefinite
        }
        MatrixEvaluationErrorV3::ImpliedNotPositiveDefinite if initial => {
            CbsemExactParameterTableErrorV3::InitialImpliedCovarianceNotPositiveDefinite
        }
        MatrixEvaluationErrorV3::StructuralSystemUnstable {
            spectral_radius,
            maximum,
        } => CbsemExactParameterTableErrorV3::StructuralSystemUnstable {
            stage: stage.into(),
            spectral_radius,
            maximum,
        },
        MatrixEvaluationErrorV3::StructuralStabilityUnavailable(message) => {
            CbsemExactParameterTableErrorV3::StructuralStabilityUnavailable {
                stage: stage.into(),
                message,
            }
        }
        other => CbsemExactParameterTableErrorV3::Numerical(other.to_string()),
    }
}

fn matrices_from_raw(
    plan: &ExactPlanV3,
    raw: &[f64],
) -> Result<ExactMatricesV3, MatrixEvaluationErrorV3> {
    if raw.len() != plan.dimensions.len() {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "parameter vector length differs from exact plan".into(),
        ));
    }
    let values = raw
        .iter()
        .zip(&plan.dimensions)
        .map(|(raw, dimension)| dimension.domain.value_from_raw(*raw))
        .collect::<Vec<_>>();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "bound transform produced a non-finite value".into(),
        ));
    }
    let constructs = plan.construct_ids.len();
    let indicators = plan.indicator_ids.len();
    let mut lambda = vec![vec![0.0; constructs]; indicators];
    let mut beta = vec![vec![0.0; constructs]; constructs];
    let mut psi = vec![vec![0.0; constructs]; constructs];
    let mut theta = vec![vec![0.0; indicators]; indicators];
    let mut latent_intercepts = vec![0.0; constructs];
    let mut observed_intercepts = vec![0.0; indicators];
    let mut row_values = Vec::with_capacity(plan.rows.len());
    for row in &plan.rows {
        let value = match row.specification {
            ResolvedSpecificationV3::Free { dimension } => values[dimension],
            ResolvedSpecificationV3::Fixed { value } => value,
        };
        row_values.push(value);
        match row.target {
            ResolvedTargetV3::Loading {
                construct,
                indicator,
            } => lambda[indicator][construct] = value,
            ResolvedTargetV3::Regression { source, target } => beta[target][source] = value,
            ResolvedTargetV3::FactorVariance { factor } => psi[factor][factor] = value,
            ResolvedTargetV3::FactorCovariance { left, right } => {
                psi[left][right] = value;
                psi[right][left] = value;
            }
            ResolvedTargetV3::ResidualVariance { indicator } => {
                theta[indicator][indicator] = value;
            }
            ResolvedTargetV3::ResidualCovariance { left, right } => {
                theta[left][right] = value;
                theta[right][left] = value;
            }
            ResolvedTargetV3::ObservedIntercept { indicator } => {
                observed_intercepts[indicator] = value;
            }
            ResolvedTargetV3::LatentMean { factor } => latent_intercepts[factor] = value,
        }
    }
    if cholesky_log_determinant(&psi).is_err() {
        return Err(MatrixEvaluationErrorV3::LatentNotPositiveDefinite);
    }
    if cholesky_log_determinant(&theta).is_err() {
        return Err(MatrixEvaluationErrorV3::ResidualNotPositiveDefinite);
    }
    let mut identity_minus_beta = identity_matrix(constructs);
    for row in 0..constructs {
        for column in 0..constructs {
            identity_minus_beta[row][column] -= beta[row][column];
        }
    }
    let structural_spectral_radius = structural_spectral_radius(&beta)?;
    if structural_spectral_radius >= CBSEM_GENERAL_ML_MAXIMUM_SPECTRAL_RADIUS_V1 {
        return Err(MatrixEvaluationErrorV3::StructuralSystemUnstable {
            spectral_radius: structural_spectral_radius,
            maximum: CBSEM_GENERAL_ML_MAXIMUM_SPECTRAL_RADIUS_V1,
        });
    }
    let inverse =
        invert_matrix(&identity_minus_beta).map_err(MatrixEvaluationErrorV3::Numerical)?;
    let phi = multiply_matrices(
        &multiply_matrices(&inverse, &psi),
        &transpose_matrix(&inverse),
    );
    let latent_means = matrix_vector_product(&inverse, &latent_intercepts);
    let mut implied_means = matrix_vector_product(&lambda, &latent_means);
    for (value, intercept) in implied_means.iter_mut().zip(&observed_intercepts) {
        *value += intercept;
    }
    let mut sigma = multiply_matrices(
        &multiply_matrices(&lambda, &phi),
        &transpose_matrix(&lambda),
    );
    add_matrix_in_place(&mut sigma, &theta);
    if cholesky_log_determinant(&sigma).is_err() {
        return Err(MatrixEvaluationErrorV3::ImpliedNotPositiveDefinite);
    }
    Ok(ExactMatricesV3 {
        psi,
        phi,
        theta,
        sigma,
        implied_means,
        row_values,
        structural_spectral_radius,
    })
}

fn structural_spectral_radius(beta: &[Vec<f64>]) -> Result<f64, MatrixEvaluationErrorV3> {
    let size = beta.len();
    if size == 0 || beta.iter().any(|row| row.len() != size) {
        return Err(MatrixEvaluationErrorV3::StructuralStabilityUnavailable(
            "structural coefficient matrix must be nonempty and square".into(),
        ));
    }
    if beta.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MatrixEvaluationErrorV3::StructuralStabilityUnavailable(
            "structural coefficient matrix contains a non-finite value".into(),
        ));
    }
    let matrix = Mat::<f64>::from_fn(size, size, |row, column| beta[row][column]);
    let eigenvalues = matrix.eigenvalues().map_err(|error| {
        MatrixEvaluationErrorV3::StructuralStabilityUnavailable(format!("{error:?}"))
    })?;
    let radius = eigenvalues
        .iter()
        .map(|value| value.norm())
        .fold(0.0_f64, f64::max);
    if !radius.is_finite() {
        return Err(MatrixEvaluationErrorV3::StructuralStabilityUnavailable(
            "structural spectral radius is non-finite".into(),
        ));
    }
    Ok(radius)
}

fn maximum_likelihood_discrepancy(
    sample: &[Vec<f64>],
    implied: &[Vec<f64>],
) -> Result<f64, MatrixEvaluationErrorV3> {
    if sample.len() != implied.len()
        || sample.iter().any(|row| row.len() != sample.len())
        || implied.iter().any(|row| row.len() != implied.len())
    {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "sample and implied covariance dimensions differ".into(),
        ));
    }
    let inverse = invert_matrix(implied).map_err(MatrixEvaluationErrorV3::Numerical)?;
    let implied_logdet = cholesky_log_determinant(implied)?;
    let sample_logdet = cholesky_log_determinant(sample)?;
    let trace = matrix_trace_product(sample, &inverse);
    let value = implied_logdet + trace - sample_logdet - sample.len() as f64;
    if !value.is_finite() {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "ML discrepancy is not finite".into(),
        ));
    }
    Ok(value.max(0.0))
}

pub(crate) fn cbsem_exact_covariance_ml_discrepancy_v1(
    sample: &[Vec<f64>],
    implied: &[Vec<f64>],
) -> Result<f64, String> {
    maximum_likelihood_discrepancy(sample, implied).map_err(|error| error.to_string())
}

fn mean_structure_discrepancy(
    observed: &[f64],
    implied: &[f64],
    covariance: &[Vec<f64>],
) -> Result<f64, MatrixEvaluationErrorV3> {
    if observed.len() != implied.len() || covariance.len() != observed.len() {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "observed and implied mean dimensions differ".into(),
        ));
    }
    let inverse = invert_matrix(covariance).map_err(MatrixEvaluationErrorV3::Numerical)?;
    let residual = observed
        .iter()
        .zip(implied)
        .map(|(observed, implied)| observed - implied)
        .collect::<Vec<_>>();
    let value = dot(&residual, &matrix_vector_product(&inverse, &residual));
    if !value.is_finite() || value < -1e-10 {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "mean-structure ML discrepancy is not finite and nonnegative".into(),
        ));
    }
    Ok(value.max(0.0))
}

pub(crate) fn cbsem_exact_mean_structure_ml_discrepancy_v1(
    observed: &[f64],
    implied: &[f64],
    covariance: &[Vec<f64>],
) -> Result<f64, String> {
    mean_structure_discrepancy(observed, implied, covariance).map_err(|error| error.to_string())
}

impl From<String> for MatrixEvaluationErrorV3 {
    fn from(value: String) -> Self {
        Self::Numerical(value)
    }
}

fn cholesky_log_determinant(matrix: &[Vec<f64>]) -> Result<f64, MatrixEvaluationErrorV3> {
    let size = matrix.len();
    if size == 0 || matrix.iter().any(|row| row.len() != size) {
        return Err(MatrixEvaluationErrorV3::Numerical(
            "positive-definite matrix must be non-empty and square".into(),
        ));
    }
    let mut lower = vec![vec![0.0; size]; size];
    for row in 0..size {
        for column in 0..=row {
            let prior = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let pivot = matrix[row][row] - prior;
                let scale = matrix[row][row].abs().max(1.0);
                if !pivot.is_finite() || pivot <= 1e-12 * scale {
                    return Err(MatrixEvaluationErrorV3::Numerical(format!(
                        "non-positive Cholesky pivot {row}"
                    )));
                }
                lower[row][column] = pivot.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - prior) / lower[column][column];
            }
        }
    }
    Ok(2.0 * (0..size).map(|index| lower[index][index].ln()).sum::<f64>())
}

fn local_identification_rank(
    plan: &ExactPlanV3,
    raw: &[f64],
    base_sigma: &[Vec<f64>],
    base_means: &[f64],
    include_means: bool,
) -> Result<usize, CbsemExactParameterTableErrorV3> {
    if raw.is_empty() {
        return Ok(0);
    }
    let mut base = lower_triangle(base_sigma);
    if include_means {
        base.extend_from_slice(base_means);
    }
    let mut jacobian = vec![vec![0.0; raw.len()]; base.len()];
    for parameter in 0..raw.len() {
        let derivative = sigma_derivative(plan, raw, base_sigma, parameter)?;
        let mut derivative = lower_triangle(&derivative);
        if include_means {
            derivative.extend(mean_derivative(plan, raw, base_means, parameter)?);
        }
        for (moment, value) in derivative.into_iter().enumerate() {
            jacobian[moment][parameter] = value;
        }
    }
    Ok(matrix_rank(&jacobian))
}

fn mean_derivative(
    plan: &ExactPlanV3,
    raw: &[f64],
    base_means: &[f64],
    parameter: usize,
) -> Result<Vec<f64>, CbsemExactParameterTableErrorV3> {
    let initial_step =
        CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP * raw[parameter].abs().max(1.0);
    for attempt in 0..20 {
        let step = initial_step * 0.5_f64.powi(attempt);
        let mut plus = raw.to_vec();
        let mut minus = raw.to_vec();
        plus[parameter] += step;
        minus[parameter] -= step;
        let plus = matrices_from_raw(plan, &plus).map(|value| value.implied_means);
        let minus = matrices_from_raw(plan, &minus).map(|value| value.implied_means);
        match (plus, minus) {
            (Ok(plus), Ok(minus)) => {
                return Ok(vector_difference_scale(&plus, &minus, 1.0 / (2.0 * step)));
            }
            (Ok(plus), Err(_)) => {
                return Ok(vector_difference_scale(&plus, base_means, 1.0 / step));
            }
            (Err(_), Ok(minus)) => {
                return Ok(vector_difference_scale(base_means, &minus, 1.0 / step));
            }
            (Err(_), Err(_)) => {}
        }
    }
    Err(CbsemExactParameterTableErrorV3::GradientUnavailable(
        parameter,
    ))
}

fn sigma_derivative(
    plan: &ExactPlanV3,
    raw: &[f64],
    base_sigma: &[Vec<f64>],
    parameter: usize,
) -> Result<Vec<Vec<f64>>, CbsemExactParameterTableErrorV3> {
    let initial_step =
        CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP * raw[parameter].abs().max(1.0);
    for attempt in 0..20 {
        let step = initial_step * 0.5_f64.powi(attempt);
        let mut plus = raw.to_vec();
        let mut minus = raw.to_vec();
        plus[parameter] += step;
        minus[parameter] -= step;
        let plus = matrices_from_raw(plan, &plus).map(|value| value.sigma);
        let minus = matrices_from_raw(plan, &minus).map(|value| value.sigma);
        match (plus, minus) {
            (Ok(plus), Ok(minus)) => {
                return Ok(matrix_difference_scale(&plus, &minus, 1.0 / (2.0 * step)));
            }
            (Ok(plus), Err(_)) => {
                return Ok(matrix_difference_scale(&plus, base_sigma, 1.0 / step));
            }
            (Err(_), Ok(minus)) => {
                return Ok(matrix_difference_scale(base_sigma, &minus, 1.0 / step));
            }
            (Err(_), Err(_)) => {}
        }
    }
    Err(CbsemExactParameterTableErrorV3::GradientUnavailable(
        parameter,
    ))
}

#[derive(Debug)]
struct ExactOptimizerResultV3 {
    parameters: Vec<f64>,
    converged: bool,
    iterations: u32,
    objective: f64,
    gradient_norm: f64,
    failure: Option<ExactOptimizerFailureV3>,
}

#[derive(Debug, Clone, Copy)]
enum ExactOptimizerFailureV3 {
    LineSearchFailed,
    ObjectiveStagnation,
    IterationLimit,
}

fn minimize_exact_objective(
    start: &[f64],
    objective: &impl Fn(&[f64]) -> Result<f64, MatrixEvaluationErrorV3>,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(u64, u64) + Sync),
) -> Result<ExactOptimizerResultV3, CbsemExactParameterTableErrorV3> {
    const MAXIMUM_ITERATIONS: u64 = 1_000;
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled);
    }
    progress(0, MAXIMUM_ITERATIONS);
    let mut current = start.to_vec();
    let mut value = objective(&current)
        .map_err(|error| CbsemExactParameterTableErrorV3::Numerical(error.to_string()))?;
    if current.is_empty() {
        return Ok(ExactOptimizerResultV3 {
            parameters: current,
            converged: true,
            iterations: 0,
            objective: value,
            gradient_norm: 0.0,
            failure: None,
        });
    }
    let mut gradient = numerical_gradient(&current, value, objective, should_cancel)?;
    let mut inverse_hessian = identity_matrix(current.len());
    let mut converged = false;
    let mut iterations = 0;
    let mut failure = None;
    for iteration in 0..MAXIMUM_ITERATIONS {
        if should_cancel() {
            return Err(CbsemExactParameterTableErrorV3::Cancelled);
        }
        iterations = iteration + 1;
        if vector_norm(&gradient) < CBSEM_ML_OPTIMIZER_STRICT_GRADIENT_NORM_TOLERANCE {
            converged = true;
            break;
        }
        let mut direction = matrix_vector_product(&inverse_hessian, &gradient)
            .into_iter()
            .map(|value| -value)
            .collect::<Vec<_>>();
        if dot(&direction, &gradient) >= 0.0 || !direction.iter().all(|value| value.is_finite()) {
            direction = gradient.iter().map(|value| -value).collect();
            inverse_hessian = identity_matrix(current.len());
        }
        let directional = dot(&gradient, &direction);
        let mut step = 1.0;
        let mut accepted = None;
        for _ in 0..32 {
            if should_cancel() {
                return Err(CbsemExactParameterTableErrorV3::Cancelled);
            }
            let candidate = current
                .iter()
                .zip(&direction)
                .map(|(value, direction)| value + step * direction)
                .collect::<Vec<_>>();
            if let Ok(candidate_value) = objective(&candidate) {
                if candidate_value <= value + 1e-4 * step * directional {
                    accepted = Some((candidate, candidate_value));
                    break;
                }
            }
            step *= 0.5;
        }
        let Some((candidate, candidate_value)) = accepted else {
            failure = Some(ExactOptimizerFailureV3::LineSearchFailed);
            break;
        };
        let candidate_gradient =
            numerical_gradient(&candidate, candidate_value, objective, should_cancel)?;
        let s = candidate
            .iter()
            .zip(&current)
            .map(|(new, old)| new - old)
            .collect::<Vec<_>>();
        let y = candidate_gradient
            .iter()
            .zip(&gradient)
            .map(|(new, old)| new - old)
            .collect::<Vec<_>>();
        let ys = dot(&y, &s);
        if ys > 1e-12 {
            inverse_hessian = bfgs_inverse_update(&inverse_hessian, &s, &y, ys);
        }
        if (value - candidate_value).abs() < CBSEM_ML_OPTIMIZER_OBJECTIVE_STAGNATION_TOLERANCE {
            converged = vector_norm(&candidate_gradient)
                < CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE;
            current = candidate;
            value = candidate_value;
            gradient = candidate_gradient;
            if !converged {
                failure = Some(ExactOptimizerFailureV3::ObjectiveStagnation);
            }
            break;
        }
        current = candidate;
        value = candidate_value;
        gradient = candidate_gradient;
        progress(iterations, MAXIMUM_ITERATIONS);
    }
    if !converged && failure.is_none() {
        failure = Some(ExactOptimizerFailureV3::IterationLimit);
    }
    progress(iterations, MAXIMUM_ITERATIONS);
    Ok(ExactOptimizerResultV3 {
        parameters: current,
        converged,
        iterations: iterations as u32,
        objective: value,
        gradient_norm: vector_norm(&gradient),
        failure,
    })
}

fn optimizer_nonconvergence_error(
    result: &ExactOptimizerResultV3,
    stage: &str,
) -> CbsemExactParameterTableErrorV3 {
    let fields = || {
        (
            stage.to_owned(),
            result.iterations,
            result.objective,
            result.gradient_norm,
        )
    };
    match result
        .failure
        .unwrap_or(ExactOptimizerFailureV3::IterationLimit)
    {
        ExactOptimizerFailureV3::LineSearchFailed => {
            let (stage, iterations, objective, gradient_norm) = fields();
            CbsemExactParameterTableErrorV3::OptimizerLineSearchFailed {
                stage,
                iterations,
                objective,
                gradient_norm,
            }
        }
        ExactOptimizerFailureV3::ObjectiveStagnation => {
            let (stage, iterations, objective, gradient_norm) = fields();
            CbsemExactParameterTableErrorV3::OptimizerObjectiveStagnation {
                stage,
                iterations,
                objective,
                gradient_norm,
            }
        }
        ExactOptimizerFailureV3::IterationLimit => {
            let (stage, iterations, objective, gradient_norm) = fields();
            CbsemExactParameterTableErrorV3::OptimizerIterationLimit {
                stage,
                iterations,
                objective,
                gradient_norm,
            }
        }
    }
}

fn numerical_gradient(
    raw: &[f64],
    base_value: f64,
    objective: &impl Fn(&[f64]) -> Result<f64, MatrixEvaluationErrorV3>,
    should_cancel: &(impl Fn() -> bool + Sync),
) -> Result<Vec<f64>, CbsemExactParameterTableErrorV3> {
    let mut gradient = Vec::with_capacity(raw.len());
    for parameter in 0..raw.len() {
        if should_cancel() {
            return Err(CbsemExactParameterTableErrorV3::Cancelled);
        }
        let initial_step =
            CBSEM_ML_OPTIMIZER_FINITE_DIFFERENCE_RELATIVE_STEP * raw[parameter].abs().max(1.0);
        let mut derivative = None;
        for attempt in 0..20 {
            if should_cancel() {
                return Err(CbsemExactParameterTableErrorV3::Cancelled);
            }
            let step = initial_step * 0.5_f64.powi(attempt);
            let mut plus = raw.to_vec();
            let mut minus = raw.to_vec();
            plus[parameter] += step;
            minus[parameter] -= step;
            let plus = objective(&plus);
            let minus = objective(&minus);
            derivative = match (plus, minus) {
                (Ok(plus), Ok(minus)) => Some((plus - minus) / (2.0 * step)),
                (Ok(plus), Err(_)) => Some((plus - base_value) / step),
                (Err(_), Ok(minus)) => Some((base_value - minus) / step),
                (Err(_), Err(_)) => None,
            };
            if derivative.is_some_and(f64::is_finite) {
                break;
            }
        }
        let Some(derivative) = derivative.filter(|value| value.is_finite()) else {
            return Err(CbsemExactParameterTableErrorV3::GradientUnavailable(
                parameter,
            ));
        };
        gradient.push(derivative);
    }
    Ok(gradient)
}

fn exact_dimension_standard_errors(
    plan: &ExactPlanV3,
    raw: &[f64],
    sigma: &[Vec<f64>],
    sample_size: usize,
) -> Result<ExactDimensionInferenceV1, CbsemExactParameterTableErrorV3> {
    if raw.is_empty() {
        return Ok(ExactDimensionInferenceV1 {
            standard_errors: Vec::new(),
            expected_information_condition: None,
        });
    }
    let inverse_sigma = invert_matrix(sigma)
        .map_err(|_| CbsemExactParameterTableErrorV3::InformationMatrixSingular)?;
    let derivatives = (0..raw.len())
        .map(|parameter| sigma_derivative(plan, raw, sigma, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    let mut information = vec![vec![0.0; raw.len()]; raw.len()];
    for row in 0..raw.len() {
        let left = multiply_matrices(
            &multiply_matrices(&inverse_sigma, &derivatives[row]),
            &inverse_sigma,
        );
        for column in row..raw.len() {
            let value =
                0.5 * sample_size as f64 * matrix_trace_product(&left, &derivatives[column]);
            information[row][column] = value;
            information[column][row] = value;
        }
    }
    let inverse = invert_scaled_information_matrix(&information)?;
    Ok(ExactDimensionInferenceV1 {
        standard_errors: information_standard_errors(&inverse.inverse_information)?,
        expected_information_condition: Some(inverse.condition),
    })
}

fn exact_dimension_standard_errors_with_means(
    plan: &ExactPlanV3,
    raw: &[f64],
    sigma: &[Vec<f64>],
    implied_means: &[f64],
    sample_size: usize,
) -> Result<ExactDimensionInferenceV1, CbsemExactParameterTableErrorV3> {
    if raw.is_empty() {
        return Ok(ExactDimensionInferenceV1 {
            standard_errors: Vec::new(),
            expected_information_condition: None,
        });
    }
    let inverse_sigma = invert_matrix(sigma)
        .map_err(|_| CbsemExactParameterTableErrorV3::InformationMatrixSingular)?;
    let covariance_derivatives = (0..raw.len())
        .map(|parameter| sigma_derivative(plan, raw, sigma, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    let mean_derivatives = (0..raw.len())
        .map(|parameter| mean_derivative(plan, raw, implied_means, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    let mut information = vec![vec![0.0; raw.len()]; raw.len()];
    for row in 0..raw.len() {
        let left_covariance = multiply_matrices(
            &multiply_matrices(&inverse_sigma, &covariance_derivatives[row]),
            &inverse_sigma,
        );
        let inverse_weighted_mean = matrix_vector_product(&inverse_sigma, &mean_derivatives[row]);
        for column in row..raw.len() {
            let covariance_information =
                0.5 * matrix_trace_product(&left_covariance, &covariance_derivatives[column]);
            let mean_information = dot(&mean_derivatives[column], &inverse_weighted_mean);
            let value = sample_size as f64 * (covariance_information + mean_information);
            information[row][column] = value;
            information[column][row] = value;
        }
    }
    let inverse = invert_scaled_information_matrix(&information)?;
    Ok(ExactDimensionInferenceV1 {
        standard_errors: information_standard_errors(&inverse.inverse_information)?,
        expected_information_condition: Some(inverse.condition),
    })
}

struct CfaScoreLmSharedV1 {
    inverse_sigma: Vec<Vec<f64>>,
    discrepancy_gradient_weight: Vec<Vec<f64>>,
    nuisance_derivatives: Vec<Vec<Vec<f64>>>,
    nuisance_scores: Vec<f64>,
    nuisance_information: Vec<Vec<f64>>,
    inverse_nuisance_information: Option<Vec<Vec<f64>>>,
}

fn exact_cfa_score_lm_bundle_v1(
    plan: &ExactPlanV3,
    raw: &[f64],
    sample_covariance: &[Vec<f64>],
    sigma: &[Vec<f64>],
    sample_size: usize,
) -> CbsemCfaScoreLmBundleV1 {
    let candidates = plan
        .rows
        .iter()
        .filter_map(|row| match (&row.target, &row.specification) {
            (
                ResolvedTargetV3::ResidualCovariance { left, right },
                ResolvedSpecificationV3::Fixed { value },
            ) if left != right && value.to_bits() == 0.0_f64.to_bits() => {
                Some((row, *left, *right))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let shared = cfa_score_lm_shared_v1(plan, raw, sample_covariance, sigma, sample_size);
    let mut rows = candidates
        .into_iter()
        .map(|(row, left, right)| {
            let (lhs, rhs) = row.target_description.lhs_rhs();
            CbsemCfaScoreLmRowV1 {
                parameter_id: row.stable_id.clone(),
                kind: "residual_covariance".into(),
                lhs: lhs.into(),
                rhs: rhs.into(),
                outcome: match &shared {
                    Ok(shared) => cfa_score_lm_candidate_outcome_v1(
                        shared,
                        sigma.len(),
                        left,
                        right,
                        sample_size,
                    ),
                    Err(reason) => CbsemCfaScoreLmOutcomeV1::Unavailable { reason: *reason },
                },
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.parameter_id.cmp(&right.parameter_id));
    CbsemCfaScoreLmBundleV1 {
        method_version: CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1.into(),
        scope: CBSEM_CFA_SCORE_LM_SCOPE_V1.into(),
        rows,
    }
}

fn cfa_score_lm_shared_v1(
    plan: &ExactPlanV3,
    raw: &[f64],
    sample_covariance: &[Vec<f64>],
    sigma: &[Vec<f64>],
    sample_size: usize,
) -> Result<CfaScoreLmSharedV1, CbsemCfaScoreLmUnavailableReasonV1> {
    let size = sigma.len();
    if sample_size == 0
        || size == 0
        || sigma.iter().any(|row| row.len() != size)
        || sample_covariance.len() != size
        || sample_covariance.iter().any(|row| row.len() != size)
        || sigma
            .iter()
            .chain(sample_covariance)
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation);
    }
    let inverse_sigma = invert_matrix(sigma)
        .map_err(|_| CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable)?;
    let inverse_sample_inverse = multiply_matrices(
        &multiply_matrices(&inverse_sigma, sample_covariance),
        &inverse_sigma,
    );
    let discrepancy_gradient_weight = subtract_matrices(&inverse_sigma, &inverse_sample_inverse);
    let nuisance_derivatives = (0..raw.len())
        .map(|parameter| sigma_derivative(plan, raw, sigma, parameter))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation)?;
    let multiplier = 0.5 * sample_size as f64;
    let nuisance_scores = nuisance_derivatives
        .iter()
        .map(|derivative| {
            -multiplier * matrix_trace_product(&discrepancy_gradient_weight, derivative)
        })
        .collect::<Vec<_>>();
    let mut nuisance_information = vec![vec![0.0; raw.len()]; raw.len()];
    for row in 0..raw.len() {
        let weighted = multiply_matrices(
            &multiply_matrices(&inverse_sigma, &nuisance_derivatives[row]),
            &inverse_sigma,
        );
        for column in row..raw.len() {
            let value = multiplier * matrix_trace_product(&weighted, &nuisance_derivatives[column]);
            nuisance_information[row][column] = value;
            nuisance_information[column][row] = value;
        }
    }
    if nuisance_scores.iter().any(|value| !value.is_finite())
        || nuisance_information
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation);
    }
    let inverse_nuisance_information = if nuisance_information.is_empty() {
        None
    } else {
        Some(
            invert_scaled_information_matrix(&nuisance_information)
                .map_err(|_| CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable)?
                .inverse_information,
        )
    };
    Ok(CfaScoreLmSharedV1 {
        inverse_sigma,
        discrepancy_gradient_weight,
        nuisance_derivatives,
        nuisance_scores,
        nuisance_information,
        inverse_nuisance_information,
    })
}

fn cfa_score_lm_candidate_outcome_v1(
    shared: &CfaScoreLmSharedV1,
    size: usize,
    left: usize,
    right: usize,
    sample_size: usize,
) -> CbsemCfaScoreLmOutcomeV1 {
    if left >= size || right >= size || left == right {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation,
        };
    }
    let mut candidate_derivative = vec![vec![0.0; size]; size];
    candidate_derivative[left][right] = 1.0;
    candidate_derivative[right][left] = 1.0;
    let multiplier = 0.5 * sample_size as f64;
    let score = -multiplier
        * matrix_trace_product(&shared.discrepancy_gradient_weight, &candidate_derivative);
    let weighted_candidate = multiply_matrices(
        &multiply_matrices(&shared.inverse_sigma, &candidate_derivative),
        &shared.inverse_sigma,
    );
    let candidate_information =
        multiplier * matrix_trace_product(&weighted_candidate, &candidate_derivative);
    let cross_information = shared
        .nuisance_derivatives
        .iter()
        .map(|derivative| multiplier * matrix_trace_product(&weighted_candidate, derivative))
        .collect::<Vec<_>>();
    if !score.is_finite()
        || !candidate_information.is_finite()
        || cross_information.iter().any(|value| !value.is_finite())
    {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation,
        };
    }

    let mut joint_information = shared.nuisance_information.clone();
    for (row, value) in cross_information.iter().copied().enumerate() {
        joint_information[row].push(value);
    }
    let mut candidate_row = cross_information.clone();
    candidate_row.push(candidate_information);
    joint_information.push(candidate_row);
    if ensure_information_positive_definite(&joint_information).is_err() {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: CbsemCfaScoreLmUnavailableReasonV1::EfficientInformationNonPositive,
        };
    }

    cfa_score_lm_available_outcome_v1(
        score,
        candidate_information,
        &cross_information,
        &shared.nuisance_scores,
        shared.inverse_nuisance_information.as_deref(),
    )
}

fn cfa_score_lm_available_outcome_v1(
    score: f64,
    candidate_information: f64,
    cross_information: &[f64],
    nuisance_scores: &[f64],
    inverse_nuisance_information: Option<&[Vec<f64>]>,
) -> CbsemCfaScoreLmOutcomeV1 {
    let (efficient_score, efficient_information) =
        if let Some(inverse) = inverse_nuisance_information {
            if inverse.len() != cross_information.len()
                || inverse
                    .iter()
                    .any(|row| row.len() != cross_information.len())
                || nuisance_scores.len() != cross_information.len()
            {
                return CbsemCfaScoreLmOutcomeV1::Unavailable {
                    reason: CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable,
                };
            }
            let inverse_score = matrix_vector_product(inverse, nuisance_scores);
            let inverse_cross = matrix_vector_product(inverse, cross_information);
            (
                score - dot(cross_information, &inverse_score),
                candidate_information - dot(cross_information, &inverse_cross),
            )
        } else if cross_information.is_empty() && nuisance_scores.is_empty() {
            (score, candidate_information)
        } else {
            return CbsemCfaScoreLmOutcomeV1::Unavailable {
                reason: CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable,
            };
        };
    if !efficient_score.is_finite()
        || !efficient_information.is_finite()
        || efficient_information <= 0.0
    {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: if efficient_information.is_finite() {
                CbsemCfaScoreLmUnavailableReasonV1::EfficientInformationNonPositive
            } else {
                CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation
            },
        };
    }
    let modification_index = efficient_score * efficient_score / efficient_information;
    let expected_parameter_change = efficient_score / efficient_information;
    let p_value = central_chi_square_cdf(modification_index, 1.0)
        .ok()
        .map(|cdf| (1.0 - cdf).clamp(0.0, 1.0));
    let Some(p_value) = p_value else {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation,
        };
    };
    if [modification_index, expected_parameter_change, p_value]
        .into_iter()
        .any(|value| !value.is_finite())
    {
        return CbsemCfaScoreLmOutcomeV1::Unavailable {
            reason: CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation,
        };
    }
    CbsemCfaScoreLmOutcomeV1::Available {
        score: canonical_positive_zero(score),
        efficient_score: canonical_positive_zero(efficient_score),
        candidate_information: canonical_positive_zero(candidate_information),
        efficient_information: canonical_positive_zero(efficient_information),
        modification_index: canonical_positive_zero(modification_index),
        expected_parameter_change: canonical_positive_zero(expected_parameter_change),
        p_value: canonical_positive_zero(p_value),
    }
}

fn canonical_positive_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn invert_scaled_information_matrix(
    information: &[Vec<f64>],
) -> Result<ScaledInformationInverseV1, CbsemExactParameterTableErrorV3> {
    let scales = (0..information.len())
        .map(|index| information[index][index])
        .map(|diagonal| {
            if diagonal.is_finite() && diagonal > 0.0 {
                Ok(diagonal.sqrt())
            } else {
                Err(CbsemExactParameterTableErrorV3::InformationMatrixSingular)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let scaled = (0..information.len())
        .map(|row| {
            (0..information.len())
                .map(|column| information[row][column] / (scales[row] * scales[column]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    ensure_information_positive_definite(&scaled)?;
    let inverse_scaled = invert_matrix(&scaled)
        .map_err(|_| CbsemExactParameterTableErrorV3::InformationMatrixSingular)?;
    let scaled_one_norm = matrix_one_norm(&scaled);
    let inverse_scaled_one_norm = matrix_one_norm(&inverse_scaled);
    let scaled_one_norm_condition_number = scaled_one_norm * inverse_scaled_one_norm;
    if !scaled_one_norm_condition_number.is_finite() || scaled_one_norm_condition_number <= 0.0 {
        return Err(CbsemExactParameterTableErrorV3::InformationMatrixSingular);
    }
    let scaled_reciprocal_one_norm_condition_number = 1.0 / scaled_one_norm_condition_number;
    if !scaled_reciprocal_one_norm_condition_number.is_finite()
        || scaled_reciprocal_one_norm_condition_number <= 0.0
    {
        return Err(CbsemExactParameterTableErrorV3::InformationMatrixSingular);
    }
    let inverse_information = (0..information.len())
        .map(|row| {
            (0..information.len())
                .map(|column| inverse_scaled[row][column] / (scales[row] * scales[column]))
                .collect()
        })
        .collect();
    Ok(ScaledInformationInverseV1 {
        inverse_information,
        condition: ExpectedInformationConditionDiagnosticsV1 {
            scaled_one_norm_condition_number,
            scaled_reciprocal_one_norm_condition_number,
        },
    })
}

fn matrix_one_norm(matrix: &[Vec<f64>]) -> f64 {
    (0..matrix.len())
        .map(|column| matrix.iter().map(|row| row[column].abs()).sum::<f64>())
        .max_by(f64::total_cmp)
        .unwrap_or(0.0)
}

fn ensure_information_positive_definite(
    information: &[Vec<f64>],
) -> Result<(), CbsemExactParameterTableErrorV3> {
    let size = information.len();
    if size == 0 || information.iter().any(|row| row.len() != size) {
        return Err(CbsemExactParameterTableErrorV3::InformationMatrixSingular);
    }
    let minimum = CBSEM_INFORMATION_CHOLESKY_MINIMUM_PIVOT_V1 * size.max(1) as f64;
    let mut lower = vec![vec![0.0; size]; size];
    for row in 0..size {
        for column in 0..=row {
            let value = information[row][column];
            let transpose = information[column][row];
            if !value.is_finite()
                || !transpose.is_finite()
                || (value - transpose).abs() > 1e-12 * value.abs().max(transpose.abs()).max(1.0)
            {
                return Err(
                    CbsemExactParameterTableErrorV3::InformationMatrixNotPositiveDefinite {
                        pivot: row,
                        value: f64::NAN,
                        minimum,
                    },
                );
            }
            let prior = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let pivot = value - prior;
                if !pivot.is_finite() || pivot <= minimum {
                    return Err(
                        CbsemExactParameterTableErrorV3::InformationMatrixNotPositiveDefinite {
                            pivot: row,
                            value: pivot,
                            minimum,
                        },
                    );
                }
                lower[row][column] = pivot.sqrt();
            } else {
                lower[row][column] = (value - prior) / lower[column][column];
            }
        }
    }
    Ok(())
}

fn information_standard_errors(
    inverse_information: &[Vec<f64>],
) -> Result<Vec<f64>, CbsemExactParameterTableErrorV3> {
    inverse_information
        .iter()
        .enumerate()
        .map(|(dimension, row)| {
            let value = row.get(dimension).copied().unwrap_or(f64::NAN);
            if !value.is_finite() || value <= 0.0 {
                Err(
                    CbsemExactParameterTableErrorV3::InformationVarianceInvalid {
                        dimension,
                        value,
                    },
                )
            } else {
                Ok(value.sqrt())
            }
        })
        .collect()
}

fn exact_parameters(
    plan: &ExactPlanV3,
    matrices: &ExactMatricesV3,
    raw: &[f64],
    raw_standard_errors: &[f64],
) -> Vec<CbsemParameter> {
    let normal = Normal::new(0.0, 1.0).ok();
    plan.rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let estimate = matrices.row_values[row_index];
            let (fixed, standard_error) = match row.specification {
                ResolvedSpecificationV3::Fixed { .. } => (true, None),
                ResolvedSpecificationV3::Free { dimension } => (
                    false,
                    Some(
                        raw_standard_errors[dimension]
                            * plan.dimensions[dimension]
                                .domain
                                .derivative(raw[dimension])
                                .abs(),
                    ),
                ),
            };
            let z_statistic = standard_error
                .filter(|value| *value > f64::EPSILON)
                .map(|value| estimate / value);
            let p_value_two_sided = z_statistic
                .zip(normal.as_ref())
                .map(|(z, normal)| (2.0 * (1.0 - normal.cdf(z.abs()))).clamp(0.0, 1.0));
            let (lhs, rhs) = row.target_description.lhs_rhs();
            CbsemParameter {
                name: row.name.clone(),
                kind: row.target_description.kind().into(),
                lhs: lhs.into(),
                rhs: rhs.into(),
                estimate,
                standard_error,
                z_statistic,
                p_value_two_sided,
                fixed,
                warning: None,
            }
        })
        .collect()
}

fn exact_standardized_parameters(
    plan: &ExactPlanV3,
    matrices: &ExactMatricesV3,
    parameters: &[CbsemParameter],
) -> Vec<CbsemStandardizedParameter> {
    let construct_index = plan
        .construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let indicator_index = plan
        .indicator_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    parameters
        .iter()
        .map(|parameter| {
            let lhs_factor = construct_index.get(parameter.lhs.as_str()).copied();
            let rhs_factor = construct_index.get(parameter.rhs.as_str()).copied();
            let lhs_indicator = indicator_index.get(parameter.lhs.as_str()).copied();
            let rhs_indicator = indicator_index.get(parameter.rhs.as_str()).copied();
            let (std_lv, std_all) = match parameter.kind.as_str() {
                "loading" => {
                    let factor_variance =
                        lhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let observed_variance =
                        rhs_indicator.map_or(1.0, |index| matrices.sigma[index][index]);
                    let std_lv = parameter.estimate * factor_variance.sqrt();
                    (std_lv, std_lv / observed_variance.sqrt())
                }
                "structural_path" => {
                    let target_variance =
                        lhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let source_variance =
                        rhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let value =
                        parameter.estimate * source_variance.sqrt() / target_variance.sqrt();
                    (value, value)
                }
                "latent_variance" => {
                    let total = lhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let value = parameter.estimate / total;
                    (value, value)
                }
                "latent_covariance" => {
                    let left = lhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let right = rhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let value = parameter.estimate / (left * right).sqrt();
                    (value, value)
                }
                "disturbance_covariance" => {
                    let left = lhs_factor.map_or(1.0, |index| matrices.psi[index][index]);
                    let right = rhs_factor.map_or(1.0, |index| matrices.psi[index][index]);
                    let value = parameter.estimate / (left * right).sqrt();
                    (value, value)
                }
                "residual_variance" => {
                    let observed = lhs_indicator.map_or(1.0, |index| matrices.sigma[index][index]);
                    (parameter.estimate, parameter.estimate / observed)
                }
                "residual_covariance" => {
                    let left = lhs_indicator.map_or(1.0, |index| matrices.theta[index][index]);
                    let right = rhs_indicator.map_or(1.0, |index| matrices.theta[index][index]);
                    let std_lv = parameter.estimate / (left * right).sqrt();
                    let observed_left =
                        lhs_indicator.map_or(1.0, |index| matrices.sigma[index][index]);
                    let observed_right =
                        rhs_indicator.map_or(1.0, |index| matrices.sigma[index][index]);
                    (
                        std_lv,
                        parameter.estimate / (observed_left * observed_right).sqrt(),
                    )
                }
                "observed_intercept" => {
                    let observed = lhs_indicator.map_or(1.0, |index| matrices.sigma[index][index]);
                    (parameter.estimate, parameter.estimate / observed.sqrt())
                }
                "latent_mean" => {
                    let latent = lhs_factor.map_or(1.0, |index| matrices.phi[index][index]);
                    let value = parameter.estimate / latent.sqrt();
                    (value, value)
                }
                _ => (parameter.estimate, parameter.estimate),
            };
            CbsemStandardizedParameter {
                name: parameter.name.clone(),
                kind: parameter.kind.clone(),
                lhs: parameter.lhs.clone(),
                rhs: parameter.rhs.clone(),
                std_lv,
                std_all,
            }
        })
        .collect()
}

fn baseline_fit(
    sample: &[Vec<f64>],
    sample_size: usize,
) -> Result<(f64, i64), CbsemExactParameterTableErrorV3> {
    let mut baseline = vec![vec![0.0; sample.len()]; sample.len()];
    for index in 0..sample.len() {
        baseline[index][index] = sample[index][index];
    }
    let objective = maximum_likelihood_discrepancy(sample, &baseline)
        .map_err(|error| CbsemExactParameterTableErrorV3::Numerical(error.to_string()))?;
    Ok((
        (sample_size as f64 * objective).max(0.0),
        (sample.len() * sample.len().saturating_sub(1) / 2) as i64,
    ))
}

#[allow(clippy::too_many_arguments)]
fn exact_fit_indices(
    chi_square: f64,
    degrees_of_freedom: i64,
    baseline_chi_square: f64,
    baseline_degrees_of_freedom: i64,
    objective: f64,
    parameter_count: usize,
    sample_size: usize,
    srmr: f64,
) -> Result<CbsemFitIndices, CbsemExactParameterTableErrorV3> {
    if !chi_square.is_finite() || chi_square < 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "exact CB-SEM chi-square must be finite and nonnegative".into(),
        ));
    }
    if !baseline_chi_square.is_finite() || baseline_chi_square < 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "exact CB-SEM baseline chi-square must be finite and nonnegative".into(),
        ));
    }
    if !objective.is_finite() || !srmr.is_finite() || sample_size == 0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "exact CB-SEM fit inputs must be finite and sample size must be positive".into(),
        ));
    }
    let p_value = if degrees_of_freedom > 0 {
        Some((1.0 - central_chi_square_cdf(chi_square, degrees_of_freedom as f64)?).clamp(0.0, 1.0))
    } else {
        None
    };
    let model_noncentrality = (chi_square - degrees_of_freedom as f64).max(0.0);
    let baseline_noncentrality =
        (baseline_chi_square - baseline_degrees_of_freedom as f64).max(f64::EPSILON);
    let cfi = Some((1.0 - model_noncentrality / baseline_noncentrality).clamp(0.0, 1.0));
    let tli = if degrees_of_freedom > 0 && baseline_degrees_of_freedom > 0 {
        let model_ratio = chi_square / degrees_of_freedom as f64;
        let baseline_ratio = baseline_chi_square / baseline_degrees_of_freedom as f64;
        Some((baseline_ratio - model_ratio) / (baseline_ratio - 1.0))
    } else {
        None
    };
    let (rmsea, rmsea_ci_lower, rmsea_ci_upper) =
        cbsem_exact_rmsea_90_percent_interval_v1(chi_square, degrees_of_freedom, sample_size)?;
    let aic = sample_size as f64 * objective + 2.0 * parameter_count as f64;
    let bic = sample_size as f64 * objective + (sample_size as f64).ln() * parameter_count as f64;
    if !aic.is_finite() || !bic.is_finite() {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "exact CB-SEM information criteria were non-finite".into(),
        ));
    }
    Ok(CbsemFitIndices {
        method_version: CBSEM_FIT_METHOD_VERSION.into(),
        chi_square,
        degrees_of_freedom,
        p_value,
        cfi,
        tli,
        rmsea,
        rmsea_interval_attribution: Some(CbsemRmseaIntervalAttributionV1 {
            method_version: CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1.into(),
            confidence_level: CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1,
        }),
        rmsea_ci_lower,
        rmsea_ci_upper,
        srmr,
        aic,
        bic,
        baseline_chi_square,
        baseline_degrees_of_freedom,
    })
}

pub fn cbsem_exact_rmsea_90_percent_interval_v1(
    chi_square: f64,
    degrees_of_freedom: i64,
    sample_size: usize,
) -> Result<(Option<f64>, Option<f64>, Option<f64>), CbsemExactParameterTableErrorV3> {
    if !chi_square.is_finite() || chi_square < 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "RMSEA chi-square must be finite and nonnegative".into(),
        ));
    }
    if degrees_of_freedom <= 0 || sample_size <= 1 {
        return Ok((None, None, None));
    }
    if chi_square == 0.0 {
        return Ok((Some(0.0), Some(0.0), Some(0.0)));
    }

    let degrees_of_freedom = degrees_of_freedom as f64;
    let denominator = degrees_of_freedom * sample_size.saturating_sub(1) as f64;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "RMSEA N-minus-one denominator was non-finite or nonpositive".into(),
        ));
    }

    let alpha = 1.0 - CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1;
    let lower_noncentrality =
        invert_noncentral_chi_square_cdf(chi_square, degrees_of_freedom, 1.0 - alpha / 2.0)?;
    let upper_noncentrality =
        invert_noncentral_chi_square_cdf(chi_square, degrees_of_freedom, alpha / 2.0)?;
    if lower_noncentrality > upper_noncentrality {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "RMSEA noncentral chi-square interval endpoints were reversed".into(),
        ));
    }

    let rmsea = ((chi_square - degrees_of_freedom).max(0.0) / denominator).sqrt();
    let lower = (lower_noncentrality / denominator).sqrt();
    let upper = (upper_noncentrality / denominator).sqrt();
    if !rmsea.is_finite()
        || !lower.is_finite()
        || !upper.is_finite()
        || lower > rmsea
        || rmsea > upper
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "RMSEA point estimate or 90% interval was non-finite or unordered".into(),
        ));
    }
    Ok((Some(rmsea), Some(lower), Some(upper)))
}

fn invert_noncentral_chi_square_cdf(
    chi_square: f64,
    degrees_of_freedom: f64,
    target_probability: f64,
) -> Result<f64, CbsemExactParameterTableErrorV3> {
    if !chi_square.is_finite()
        || chi_square < 0.0
        || !degrees_of_freedom.is_finite()
        || degrees_of_freedom <= 0.0
        || !target_probability.is_finite()
        || !(0.0..1.0).contains(&target_probability)
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square inversion received an invalid input".into(),
        ));
    }

    let central_probability = noncentral_chi_square_cdf(chi_square, degrees_of_freedom, 0.0)?;
    if central_probability <= target_probability {
        return Ok(0.0);
    }

    let mut lower = 0.0;
    let mut upper = (chi_square - degrees_of_freedom).max(1.0);
    let mut bracketed = false;
    for _ in 0..CBSEM_NONCENTRAL_CHI_SQUARE_BRACKET_MAXIMUM_ITERATIONS_V1 {
        if noncentral_chi_square_cdf(chi_square, degrees_of_freedom, upper)? <= target_probability {
            bracketed = true;
            break;
        }
        upper *= 2.0;
        if !upper.is_finite() {
            return Err(CbsemExactParameterTableErrorV3::Numerical(
                "noncentral chi-square inversion bracket became non-finite".into(),
            ));
        }
    }
    if !bracketed {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square inversion could not bracket a root".into(),
        ));
    }

    for _ in 0..CBSEM_NONCENTRAL_CHI_SQUARE_INVERSION_MAXIMUM_ITERATIONS_V1 {
        let midpoint = lower + (upper - lower) / 2.0;
        let probability = noncentral_chi_square_cdf(chi_square, degrees_of_freedom, midpoint)?;
        if probability > target_probability {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
        if upper - lower <= CBSEM_NONCENTRAL_CHI_SQUARE_INVERSION_TOLERANCE_V1 * upper.max(1.0) {
            let result = lower + (upper - lower) / 2.0;
            return if result.is_finite() {
                Ok(result)
            } else {
                Err(CbsemExactParameterTableErrorV3::Numerical(
                    "noncentral chi-square inversion produced a non-finite root".into(),
                ))
            };
        }
    }

    Err(CbsemExactParameterTableErrorV3::Numerical(
        "noncentral chi-square inversion did not converge".into(),
    ))
}

fn noncentral_chi_square_cdf(
    chi_square: f64,
    degrees_of_freedom: f64,
    noncentrality: f64,
) -> Result<f64, CbsemExactParameterTableErrorV3> {
    if !chi_square.is_finite()
        || chi_square < 0.0
        || !degrees_of_freedom.is_finite()
        || degrees_of_freedom <= 0.0
        || !noncentrality.is_finite()
        || noncentrality < 0.0
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square CDF received an invalid input".into(),
        ));
    }
    if chi_square == 0.0 {
        return Ok(0.0);
    }
    if noncentrality == 0.0 {
        return central_chi_square_cdf(chi_square, degrees_of_freedom);
    }

    let poisson_mean = noncentrality / 2.0;
    let mode_as_f64 = poisson_mean.floor();
    if !mode_as_f64.is_finite() || mode_as_f64 >= usize::MAX as f64 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture mode was not representable".into(),
        ));
    }
    let mode = mode_as_f64 as usize;
    // Evaluate weights relative to the Poisson mode. Computing the absolute
    // mode weight through exp(-mu + k ln(mu) - ln(k!)) loses several ulps of
    // normalization for large, otherwise ordinary noncentralities. Relative
    // recurrence keeps its largest weight at one; the final quotient removes
    // the common (and unnecessary) Poisson normalization constant.
    let mode_weight = 1.0;
    let mut probability = 0.0;
    let mut probability_correction = 0.0;
    let mut mass = 0.0;
    let mut mass_correction = 0.0;
    add_noncentral_chi_square_mixture_term(
        chi_square,
        degrees_of_freedom,
        mode,
        mode_weight,
        &mut probability,
        &mut probability_correction,
        &mut mass,
        &mut mass_correction,
    )?;

    let mut lower_index = mode;
    let mut lower_weight = mode_weight;
    let mut upper_index = mode;
    let mut upper_weight = mode_weight;
    for _ in 0..CBSEM_NONCENTRAL_CHI_SQUARE_MAXIMUM_TERMS_V1 {
        if lower_index > 0 {
            lower_weight *= lower_index as f64 / poisson_mean;
            lower_index -= 1;
            add_noncentral_chi_square_mixture_term(
                chi_square,
                degrees_of_freedom,
                lower_index,
                lower_weight,
                &mut probability,
                &mut probability_correction,
                &mut mass,
                &mut mass_correction,
            )?;
        }
        upper_index = upper_index.checked_add(1).ok_or_else(|| {
            CbsemExactParameterTableErrorV3::Numerical(
                "noncentral chi-square Poisson-mixture index overflowed".into(),
            )
        })?;
        upper_weight *= poisson_mean / upper_index as f64;
        add_noncentral_chi_square_mixture_term(
            chi_square,
            degrees_of_freedom,
            upper_index,
            upper_weight,
            &mut probability,
            &mut probability_correction,
            &mut mass,
            &mut mass_correction,
        )?;

        let lower_tail_bound = if lower_index == 0 {
            0.0
        } else {
            let next_ratio = lower_index as f64 / poisson_mean;
            geometric_poisson_tail_bound(lower_weight, next_ratio)?
        };
        let next_upper_index = upper_index.checked_add(1).ok_or_else(|| {
            CbsemExactParameterTableErrorV3::Numerical(
                "noncentral chi-square Poisson-mixture index overflowed".into(),
            )
        })?;
        let upper_tail_bound =
            geometric_poisson_tail_bound(upper_weight, poisson_mean / next_upper_index as f64)?;
        let omitted_mass_bound = lower_tail_bound + upper_tail_bound;
        if !omitted_mass_bound.is_finite() || omitted_mass_bound < 0.0 {
            return Err(CbsemExactParameterTableErrorV3::Numerical(
                "noncentral chi-square Poisson-mixture tail bound was invalid".into(),
            ));
        }
        if omitted_mass_bound <= CBSEM_NONCENTRAL_CHI_SQUARE_MIXTURE_TOLERANCE_V1 * mass {
            return checked_probability(probability, mass);
        }
    }

    Err(CbsemExactParameterTableErrorV3::Numerical(
        "noncentral chi-square Poisson mixture did not converge".into(),
    ))
}

fn geometric_poisson_tail_bound(
    boundary_weight: f64,
    next_ratio: f64,
) -> Result<f64, CbsemExactParameterTableErrorV3> {
    if !boundary_weight.is_finite()
        || boundary_weight < 0.0
        || !next_ratio.is_finite()
        || !(0.0..1.0).contains(&next_ratio)
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture tail ratio was invalid".into(),
        ));
    }
    let first_omitted_weight = boundary_weight * next_ratio;
    let bound = first_omitted_weight / (1.0 - next_ratio);
    if !bound.is_finite() || bound < 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture tail bound was invalid".into(),
        ));
    }
    Ok(bound)
}

#[allow(clippy::too_many_arguments)]
fn add_noncentral_chi_square_mixture_term(
    chi_square: f64,
    degrees_of_freedom: f64,
    index: usize,
    weight: f64,
    probability: &mut f64,
    probability_correction: &mut f64,
    mass: &mut f64,
    mass_correction: &mut f64,
) -> Result<(), CbsemExactParameterTableErrorV3> {
    if !weight.is_finite() || weight < 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture weight was invalid".into(),
        ));
    }
    let component_degrees_of_freedom = degrees_of_freedom + 2.0 * index as f64;
    if !component_degrees_of_freedom.is_finite() {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square mixture degrees of freedom were non-finite".into(),
        ));
    }
    let component_probability = central_chi_square_cdf(chi_square, component_degrees_of_freedom)?;
    compensated_add(
        probability,
        probability_correction,
        weight * component_probability,
    );
    compensated_add(mass, mass_correction, weight);
    if !probability.is_finite() || !mass.is_finite() {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture sum was non-finite".into(),
        ));
    }
    Ok(())
}

fn central_chi_square_cdf(
    chi_square: f64,
    degrees_of_freedom: f64,
) -> Result<f64, CbsemExactParameterTableErrorV3> {
    if !chi_square.is_finite()
        || chi_square < 0.0
        || !degrees_of_freedom.is_finite()
        || degrees_of_freedom <= 0.0
    {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "central chi-square CDF received an invalid input".into(),
        ));
    }
    if chi_square == 0.0 {
        return Ok(0.0);
    }
    let probability = gamma_lr(degrees_of_freedom / 2.0, chi_square / 2.0);
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "central chi-square CDF produced an invalid probability".into(),
        ));
    }
    Ok(probability)
}

fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let corrected_value = value - *correction;
    let updated = *sum + corrected_value;
    *correction = (updated - *sum) - corrected_value;
    *sum = updated;
}

fn checked_probability(
    probability: f64,
    mass: f64,
) -> Result<f64, CbsemExactParameterTableErrorV3> {
    if !probability.is_finite() || !mass.is_finite() || probability < 0.0 || mass <= 0.0 {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture probability was invalid".into(),
        ));
    }
    let normalized_probability = probability / mass;
    if !normalized_probability.is_finite() || !(0.0..=1.0).contains(&normalized_probability) {
        return Err(CbsemExactParameterTableErrorV3::Numerical(
            "noncentral chi-square Poisson-mixture probability was invalid".into(),
        ));
    }
    Ok(normalized_probability)
}

fn matrix_cells(names: &[String], matrix: &[Vec<f64>]) -> Vec<CbsemMatrixCell> {
    let mut cells = Vec::with_capacity(names.len() * names.len());
    for (row, row_name) in names.iter().enumerate() {
        for (column, column_name) in names.iter().enumerate() {
            cells.push(CbsemMatrixCell {
                row: row_name.clone(),
                column: column_name.clone(),
                value: matrix[row][column],
            });
        }
    }
    cells
}

fn residual_correlation_matrix(residual: &[Vec<f64>], sample: &[Vec<f64>]) -> Vec<Vec<f64>> {
    (0..residual.len())
        .map(|row| {
            (0..residual.len())
                .map(|column| {
                    let denominator = (sample[row][row] * sample[column][column]).sqrt();
                    if denominator > f64::EPSILON {
                        residual[row][column] / denominator
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

fn matrix_srmr(sample: &[Vec<f64>], implied: &[Vec<f64>]) -> f64 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for row in 0..sample.len() {
        for column in 0..=row {
            let denominator = (sample[row][row] * sample[column][column]).sqrt();
            if denominator > f64::EPSILON {
                let residual = (sample[row][column] - implied[row][column]) / denominator;
                sum += residual * residual;
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        (sum / count as f64).sqrt()
    }
}

fn lower_triangle(matrix: &[Vec<f64>]) -> Vec<f64> {
    let mut values = Vec::with_capacity(matrix.len() * (matrix.len() + 1) / 2);
    for row in 0..matrix.len() {
        for column in 0..=row {
            values.push(matrix[row][column]);
        }
    }
    values
}

fn matrix_rank(matrix: &[Vec<f64>]) -> usize {
    if matrix.is_empty() || matrix[0].is_empty() {
        return 0;
    }
    let mut reduced = matrix.to_vec();
    let rows = reduced.len();
    let columns = reduced[0].len();
    let scale = reduced
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let tolerance = 1e-8 * scale * rows.max(columns) as f64;
    let mut rank = 0;
    for column in 0..columns {
        let pivot = (rank..rows).max_by(|left, right| {
            reduced[*left][column]
                .abs()
                .total_cmp(&reduced[*right][column].abs())
        });
        let Some(pivot) = pivot else {
            continue;
        };
        if reduced[pivot][column].abs() <= tolerance {
            continue;
        }
        reduced.swap(rank, pivot);
        let pivot_value = reduced[rank][column];
        for value in &mut reduced[rank][column..] {
            *value /= pivot_value;
        }
        for row in 0..rows {
            if row == rank {
                continue;
            }
            let factor = reduced[row][column];
            for candidate in column..columns {
                reduced[row][candidate] -= factor * reduced[rank][candidate];
            }
        }
        rank += 1;
        if rank == rows {
            break;
        }
    }
    rank
}

fn identity_matrix(size: usize) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; size]; size];
    for index in 0..size {
        matrix[index][index] = 1.0;
    }
    matrix
}

fn transpose_matrix(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut transposed = vec![vec![0.0; matrix.len()]; matrix[0].len()];
    for row in 0..matrix.len() {
        for column in 0..matrix[row].len() {
            transposed[column][row] = matrix[row][column];
        }
    }
    transposed
}

fn multiply_matrices(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut product = vec![vec![0.0; right[0].len()]; left.len()];
    for row in 0..left.len() {
        for common in 0..right.len() {
            for column in 0..right[common].len() {
                product[row][column] += left[row][common] * right[common][column];
            }
        }
    }
    product
}

fn add_matrix_in_place(target: &mut [Vec<f64>], addition: &[Vec<f64>]) {
    for row in 0..target.len() {
        for column in 0..target[row].len() {
            target[row][column] += addition[row][column];
        }
    }
}

fn subtract_matrices(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            left.iter()
                .zip(right)
                .map(|(left, right)| left - right)
                .collect()
        })
        .collect()
}

fn matrix_difference_scale(left: &[Vec<f64>], right: &[Vec<f64>], scale: f64) -> Vec<Vec<f64>> {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            left.iter()
                .zip(right)
                .map(|(left, right)| (left - right) * scale)
                .collect()
        })
        .collect()
}

fn vector_difference_scale(left: &[f64], right: &[f64], scale: f64) -> Vec<f64> {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right) * scale)
        .collect()
}

fn matrix_trace_product(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    (0..left.len())
        .map(|row| {
            (0..left.len())
                .map(|column| left[row][column] * right[column][row])
                .sum::<f64>()
        })
        .sum()
}

fn invert_matrix(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let size = matrix.len();
    if size == 0 || matrix.iter().any(|row| row.len() != size) {
        return Err("matrix must be non-empty and square".into());
    }
    let mut augmented = vec![vec![0.0; size * 2]; size];
    for row in 0..size {
        augmented[row][..size].copy_from_slice(&matrix[row]);
        augmented[row][size + row] = 1.0;
    }
    for column in 0..size {
        let pivot = (column..size)
            .max_by(|left, right| {
                augmented[*left][column]
                    .abs()
                    .total_cmp(&augmented[*right][column].abs())
            })
            .expect("non-empty pivot range");
        if augmented[pivot][column].abs() <= 1e-14 {
            return Err(format!("singular matrix at pivot {column}"));
        }
        augmented.swap(column, pivot);
        let pivot_value = augmented[column][column];
        for value in &mut augmented[column] {
            *value /= pivot_value;
        }
        for row in 0..size {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for candidate in 0..size * 2 {
                augmented[row][candidate] -= factor * augmented[column][candidate];
            }
        }
    }
    Ok(augmented
        .into_iter()
        .map(|row| row[size..].to_vec())
        .collect())
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(vector)
                .map(|(left, right)| left * right)
                .sum()
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn vector_norm(vector: &[f64]) -> f64 {
    dot(vector, vector).sqrt()
}

fn bfgs_inverse_update(
    inverse_hessian: &[Vec<f64>],
    s: &[f64],
    y: &[f64],
    ys: f64,
) -> Vec<Vec<f64>> {
    let rho = 1.0 / ys;
    let hy = matrix_vector_product(inverse_hessian, y);
    let yhy = dot(y, &hy);
    let coefficient = (1.0 + yhy * rho) * rho;
    let mut updated = inverse_hessian.to_vec();
    for row in 0..s.len() {
        for column in 0..s.len() {
            updated[row][column] += coefficient * s[row] * s[column]
                - rho * (s[row] * hy[column] + hy[row] * s[column]);
        }
    }
    updated
}

#[cfg(test)]
mod cfa_score_lm_tests {
    use super::*;

    fn fixed_residual_covariance_row(
        stable_id: &str,
        left_id: &str,
        right_id: &str,
        left: usize,
        right: usize,
        value: f64,
    ) -> ResolvedRowV3 {
        ResolvedRowV3 {
            stable_id: stable_id.into(),
            name: format!("{left_id}~~{right_id}"),
            target_description: CbsemExactParameterTargetV3::ResidualCovariance {
                left: left_id.into(),
                right: right_id.into(),
            },
            target: ResolvedTargetV3::ResidualCovariance { left, right },
            specification: ResolvedSpecificationV3::Fixed { value },
        }
    }

    #[test]
    fn closed_form_score_lm_uses_chi_square_one_survival() {
        let outcome = cfa_score_lm_available_outcome_v1(2.0, 10.0, &[], &[], None);
        let CbsemCfaScoreLmOutcomeV1::Available {
            score,
            efficient_score,
            candidate_information,
            efficient_information,
            modification_index,
            expected_parameter_change,
            p_value,
        } = outcome
        else {
            panic!("closed-form candidate must be available")
        };
        assert_eq!(score.to_bits(), 2.0_f64.to_bits());
        assert_eq!(efficient_score.to_bits(), 2.0_f64.to_bits());
        assert_eq!(candidate_information.to_bits(), 10.0_f64.to_bits());
        assert_eq!(efficient_information.to_bits(), 10.0_f64.to_bits());
        assert!((modification_index - 0.4).abs() <= 1e-15);
        assert!((expected_parameter_change - 0.2).abs() <= 1e-15);
        let expected_p = 1.0 - central_chi_square_cdf(0.4, 1.0).unwrap();
        assert!((p_value - expected_p).abs() <= 1e-15);
    }

    #[test]
    fn nuisance_schur_correction_is_applied_to_score_and_information() {
        let inverse_nuisance = vec![vec![0.25]];
        let outcome =
            cfa_score_lm_available_outcome_v1(6.0, 10.0, &[2.0], &[3.0], Some(&inverse_nuisance));
        let CbsemCfaScoreLmOutcomeV1::Available {
            efficient_score,
            efficient_information,
            modification_index,
            expected_parameter_change,
            ..
        } = outcome
        else {
            panic!("positive Schur complement must be available")
        };
        assert_eq!(efficient_score.to_bits(), 4.5_f64.to_bits());
        assert_eq!(efficient_information.to_bits(), 9.0_f64.to_bits());
        assert_eq!(modification_index.to_bits(), 2.25_f64.to_bits());
        assert_eq!(expected_parameter_change.to_bits(), 0.5_f64.to_bits());
    }

    #[test]
    fn candidates_are_explicit_fixed_positive_zero_only_and_stably_ordered() {
        let plan = ExactPlanV3 {
            construct_ids: Vec::new(),
            indicator_ids: vec!["a".into(), "b".into(), "c".into()],
            dimensions: Vec::new(),
            rows: vec![
                fixed_residual_covariance_row("parameter:z", "b", "c", 1, 2, 0.0),
                fixed_residual_covariance_row("parameter:negative", "a", "b", 0, 1, -0.0),
                ResolvedRowV3 {
                    stable_id: "parameter:variance".into(),
                    name: "a~~a".into(),
                    target_description: CbsemExactParameterTargetV3::ResidualVariance {
                        indicator: "a".into(),
                    },
                    target: ResolvedTargetV3::ResidualVariance { indicator: 0 },
                    specification: ResolvedSpecificationV3::Fixed { value: 0.0 },
                },
                fixed_residual_covariance_row("parameter:a", "a", "c", 0, 2, 0.0),
                fixed_residual_covariance_row("parameter:nonzero", "a", "b", 0, 1, 1e-12),
            ],
        };
        let sample = vec![
            vec![1.0, 0.1, 0.2],
            vec![0.1, 1.0, 0.05],
            vec![0.2, 0.05, 1.0],
        ];
        let sigma = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let bundle = exact_cfa_score_lm_bundle_v1(&plan, &[], &sample, &sigma, 100);

        assert_eq!(bundle.method_version, CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1);
        assert_eq!(bundle.scope, CBSEM_CFA_SCORE_LM_SCOPE_V1);
        assert_eq!(bundle.rows.len(), 2);
        assert_eq!(bundle.rows[0].parameter_id, "parameter:a");
        assert_eq!(
            (bundle.rows[0].lhs.as_str(), bundle.rows[0].rhs.as_str()),
            ("a", "c")
        );
        assert_eq!(bundle.rows[1].parameter_id, "parameter:z");
        assert_eq!(
            (bundle.rows[1].lhs.as_str(), bundle.rows[1].rhs.as_str()),
            ("b", "c")
        );
        assert!(
            bundle
                .rows
                .iter()
                .all(|row| matches!(&row.outcome, CbsemCfaScoreLmOutcomeV1::Available { .. }))
        );
    }

    #[test]
    fn exact_zero_results_are_serialized_as_positive_zero() {
        let outcome = cfa_score_lm_available_outcome_v1(0.0, 2.0, &[], &[], None);
        let CbsemCfaScoreLmOutcomeV1::Available {
            score,
            efficient_score,
            modification_index,
            expected_parameter_change,
            ..
        } = outcome
        else {
            panic!("zero-score candidate must be available")
        };
        for value in [
            score,
            efficient_score,
            modification_index,
            expected_parameter_change,
        ] {
            assert_eq!(value.to_bits(), 0.0_f64.to_bits());
        }
    }

    #[test]
    fn nonpositive_efficient_information_is_typed_unavailable() {
        let inverse_nuisance = vec![vec![1.0]];
        assert_eq!(
            cfa_score_lm_available_outcome_v1(1.0, 1.0, &[2.0], &[0.0], Some(&inverse_nuisance),),
            CbsemCfaScoreLmOutcomeV1::Unavailable {
                reason: CbsemCfaScoreLmUnavailableReasonV1::EfficientInformationNonPositive,
            }
        );
    }
}

#[cfg(test)]
mod exact_refit_standard_error_tests {
    use super::*;

    fn two_parameter_plan() -> ExactPlanV3 {
        ExactPlanV3 {
            construct_ids: Vec::new(),
            indicator_ids: vec!["x".into(), "y".into()],
            dimensions: vec![
                FreeDimensionV3 {
                    key: "parameter:a".into(),
                    domain: OpenDomainV3 {
                        lower: None,
                        upper: None,
                    },
                    start_value: 1.0,
                },
                FreeDimensionV3 {
                    key: "parameter:z".into(),
                    domain: OpenDomainV3 {
                        lower: None,
                        upper: None,
                    },
                    start_value: 1.0,
                },
            ],
            rows: vec![
                ResolvedRowV3 {
                    stable_id: "parameter:a".into(),
                    name: "x~~x".into(),
                    target_description: CbsemExactParameterTargetV3::ResidualVariance {
                        indicator: "x".into(),
                    },
                    target: ResolvedTargetV3::ResidualVariance { indicator: 0 },
                    specification: ResolvedSpecificationV3::Free { dimension: 0 },
                },
                ResolvedRowV3 {
                    stable_id: "parameter:z".into(),
                    name: "y~~y".into(),
                    target_description: CbsemExactParameterTargetV3::ResidualVariance {
                        indicator: "y".into(),
                    },
                    target: ResolvedTargetV3::ResidualVariance { indicator: 1 },
                    specification: ResolvedSpecificationV3::Free { dimension: 1 },
                },
            ],
        }
    }

    #[test]
    fn refit_standard_error_rows_preserve_stable_order_and_positive_bits() {
        let receipt = exact_refit_standard_errors_from_dimension_inference_v1(
            &two_parameter_plan(),
            &[1.0, 1.0],
            Ok(ExactDimensionInferenceV1 {
                standard_errors: vec![0.25, 0.5],
                expected_information_condition: None,
            }),
        );
        assert_eq!(
            receipt.method_version,
            CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
        );
        let CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
            information_method,
            parameters,
        } = receipt.outcome
        else {
            panic!("positive finite expected-information SEs must be available")
        };
        assert_eq!(
            information_method,
            CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
        );
        assert_eq!(
            parameters
                .iter()
                .map(|parameter| parameter.parameter_id.as_str())
                .collect::<Vec<_>>(),
            vec!["parameter:a", "parameter:z"]
        );
        assert_eq!(parameters[0].standard_error.to_bits(), 0.25_f64.to_bits());
        assert_eq!(parameters[1].standard_error.to_bits(), 0.5_f64.to_bits());
    }

    #[test]
    fn singular_or_indefinite_information_is_typed_unavailable_without_losing_point() {
        for (error, expected_reason) in [
            (
                CbsemExactParameterTableErrorV3::InformationMatrixSingular,
                CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation,
            ),
            (
                CbsemExactParameterTableErrorV3::InformationMatrixNotPositiveDefinite {
                    pivot: 1,
                    value: -1.0,
                    minimum: 1e-10,
                },
                CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InformationNotPositiveDefinite,
            ),
        ] {
            let receipt = exact_refit_standard_errors_from_dimension_inference_v1(
                &two_parameter_plan(),
                &[1.0, 1.0],
                Err(error),
            );
            assert_eq!(
                receipt.outcome,
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                    reason: expected_reason,
                }
            );
            let point = CbsemExactParameterTablePointRefitV1 {
                method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
                free_parameters: vec![CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: "parameter:a".into(),
                    estimate: 1.25,
                }],
                standard_errors: Some(receipt),
                iterations: 5,
                objective: 0.1,
                gradient_norm: 1e-8,
            };
            assert_eq!(point.free_parameters[0].estimate.to_bits(), 1.25_f64.to_bits());
        }
    }

    #[test]
    fn zero_or_nonfinite_standard_error_is_whole_vector_unavailable() {
        for invalid in [0.0, f64::INFINITY] {
            let receipt = exact_refit_standard_errors_from_dimension_inference_v1(
                &two_parameter_plan(),
                &[1.0, 1.0],
                Ok(ExactDimensionInferenceV1 {
                    standard_errors: vec![0.25, invalid],
                    expected_information_condition: None,
                }),
            );
            assert_eq!(
                receipt.outcome,
                CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                    reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError,
                }
            );
        }
    }
}

#[cfg(test)]
mod rmsea_interval_tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn noncentral_cdf_at_zero_noncentrality_matches_analytic_df_two_cdf() {
        let chi_square = 7.25;
        let expected = 1.0 - (-chi_square / 2.0_f64).exp();

        let central = central_chi_square_cdf(chi_square, 2.0).unwrap();
        let noncentral = noncentral_chi_square_cdf(chi_square, 2.0, 0.0).unwrap();

        assert_close(central, expected, 2e-15);
        assert_eq!(noncentral.to_bits(), central.to_bits());
    }

    #[test]
    fn rmsea_interval_matches_independent_noncentral_chi_square_fixture() {
        // Reference values were independently generated with scipy.stats.ncx2 and brentq.
        let (rmsea, lower, upper) =
            cbsem_exact_rmsea_90_percent_interval_v1(20.0, 10, 200).unwrap();

        assert_close(rmsea.unwrap(), 0.070_888_120_500_833_6, 2e-12);
        assert_close(lower.unwrap(), 0.021_808_635_489_297_24, 2e-10);
        assert_close(upper.unwrap(), 0.116_030_666_212_127_24, 2e-10);

        let denominator = 10.0 * 199.0;
        let lower_noncentrality = lower.unwrap().powi(2) * denominator;
        let upper_noncentrality = upper.unwrap().powi(2) * denominator;
        assert_close(lower_noncentrality, 0.946_476_997_991_019_9, 2e-8);
        assert_close(upper_noncentrality, 26.791_599_848_243_866, 2e-8);
        assert_close(
            noncentral_chi_square_cdf(20.0, 10.0, lower_noncentrality).unwrap(),
            0.95,
            2e-10,
        );
        assert_close(
            noncentral_chi_square_cdf(20.0, 10.0, upper_noncentrality).unwrap(),
            0.05,
            2e-10,
        );
    }

    #[test]
    fn mode_relative_mixture_covers_fixed_parameter_rmsea_regression() {
        // This is the valid high-chi-square fit produced by the fully fixed
        // three-indicator covariance fixture in cbsem_matrix_input. Reference
        // values were independently generated with scipy.stats.ncx2 and brentq.
        let chi_square = 4_484.454_131_137_745;
        let degrees_of_freedom = 6.0;
        let noncentrality = 3_918.647_364_745_527;
        assert_close(
            noncentral_chi_square_cdf(chi_square, degrees_of_freedom, noncentrality).unwrap(),
            0.999_992_488_809_460_3,
            2e-12,
        );

        let (rmsea, lower, upper) =
            cbsem_exact_rmsea_90_percent_interval_v1(chi_square, 6, 40).unwrap();
        assert_close(rmsea.unwrap(), 4.374_779_179_354_087, 2e-12);
        assert_close(lower.unwrap(), 4.267_709_535_016_018, 2e-9);
        assert_close(upper.unwrap(), 4.482_824_453_753_786, 2e-9);
        assert_close(
            noncentral_chi_square_cdf(
                chi_square,
                degrees_of_freedom,
                lower.unwrap().powi(2) * degrees_of_freedom * 39.0,
            )
            .unwrap(),
            0.95,
            2e-10,
        );
        assert_close(
            noncentral_chi_square_cdf(
                chi_square,
                degrees_of_freedom,
                upper.unwrap().powi(2) * degrees_of_freedom * 39.0,
            )
            .unwrap(),
            0.05,
            2e-10,
        );
    }

    #[test]
    fn mode_relative_mixture_is_stable_for_extreme_valid_noncentralities() {
        // Independent scipy.stats.ncx2 references exercise Poisson modes far
        // beyond the range where an absolute exp/log-gamma mode weight keeps
        // enough normalization accuracy for the inversion tolerance.
        let hundred_thousand = noncentral_chi_square_cdf(100_006.0, 6.0, 100_000.0).unwrap();
        let one_million = noncentral_chi_square_cdf(1_000_006.0, 6.0, 1_000_000.0).unwrap();
        assert_close(hundred_thousand, 0.500_630_768_149_855_5, 2e-9);
        assert_close(one_million, 0.500_199_470_666_449_9, 2e-9);

        let lower_noncentrality = noncentral_chi_square_cdf(1_000_006.0, 6.0, 995_000.0).unwrap();
        let higher_noncentrality =
            noncentral_chi_square_cdf(1_000_006.0, 6.0, 1_005_000.0).unwrap();
        assert!(lower_noncentrality > one_million);
        assert!(one_million > higher_noncentrality);
        assert_close(lower_noncentrality, 0.993_853_669_245_230_3, 2e-9);
        assert_close(higher_noncentrality, 0.006_273_411_400_781_86, 2e-9);
        assert_eq!(
            noncentral_chi_square_cdf(0.0, 6.0, 1_000_000.0)
                .unwrap()
                .to_bits(),
            0.0_f64.to_bits()
        );
    }

    #[test]
    fn rmsea_uses_n_minus_one_and_is_bitwise_deterministic() {
        let first = cbsem_exact_rmsea_90_percent_interval_v1(10.0, 5, 100).unwrap();
        let second = cbsem_exact_rmsea_90_percent_interval_v1(10.0, 5, 100).unwrap();

        assert_eq!(first, second);
        assert_close(first.0.unwrap(), 0.100_503_781_525_921_21, 2e-15);
        assert_eq!(first.1.unwrap().to_bits(), 0.0_f64.to_bits());
        assert_close(first.2.unwrap(), 0.191_315_170_755_591_35, 2e-10);
    }

    #[test]
    fn rmsea_interval_has_explicit_degenerate_boundaries() {
        assert_eq!(
            cbsem_exact_rmsea_90_percent_interval_v1(5.0, 0, 100).unwrap(),
            (None, None, None)
        );
        assert_eq!(
            cbsem_exact_rmsea_90_percent_interval_v1(5.0, 5, 1).unwrap(),
            (None, None, None)
        );
        assert_eq!(
            cbsem_exact_rmsea_90_percent_interval_v1(0.0, 5, 100).unwrap(),
            (Some(0.0), Some(0.0), Some(0.0))
        );
    }

    #[test]
    fn rmsea_interval_fails_closed_on_invalid_or_unrepresentable_inputs() {
        assert!(cbsem_exact_rmsea_90_percent_interval_v1(f64::NAN, 5, 100).is_err());
        assert!(invert_noncentral_chi_square_cdf(10.0, 5.0, 1.0).is_err());
        assert!(noncentral_chi_square_cdf(10.0, 5.0, f64::MAX / 2.0).is_err());
        assert!(geometric_poisson_tail_bound(1.0, 1.0).is_err());
    }

    #[test]
    fn rmsea_interval_method_identity_is_exact() {
        assert_eq!(
            CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1,
            "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1"
        );
        assert_eq!(CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1, 0.90);
    }
}

#[cfg(test)]
mod information_admissibility_tests {
    use super::*;

    #[test]
    fn information_condition_diagnostics_report_identity_exactly() {
        let inverse = invert_scaled_information_matrix(&[vec![1.0, 0.0], vec![0.0, 1.0]]).unwrap();

        assert_eq!(
            inverse.inverse_information,
            vec![vec![1.0, 0.0], vec![0.0, 1.0]]
        );
        assert_eq!(
            inverse.condition.scaled_one_norm_condition_number.to_bits(),
            1.0_f64.to_bits()
        );
        assert_eq!(
            inverse
                .condition
                .scaled_reciprocal_one_norm_condition_number
                .to_bits(),
            1.0_f64.to_bits()
        );
    }

    #[test]
    fn information_condition_diagnostics_match_known_two_by_two_matrix() {
        let inverse = invert_scaled_information_matrix(&[vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();

        assert!((inverse.condition.scaled_one_norm_condition_number - 3.0).abs() <= 1e-12);
        assert!(
            (inverse
                .condition
                .scaled_reciprocal_one_norm_condition_number
                - 1.0 / 3.0)
                .abs()
                <= 1e-12
        );
    }

    #[test]
    fn information_condition_diagnostics_remain_finite_near_singularity_without_clamping() {
        let correlation = 1.0 - 1e-8;
        let inverse =
            invert_scaled_information_matrix(&[vec![1.0, correlation], vec![correlation, 1.0]])
                .unwrap();
        let condition = inverse.condition.scaled_one_norm_condition_number;
        let reciprocal = inverse
            .condition
            .scaled_reciprocal_one_norm_condition_number;

        assert!(condition.is_finite() && condition > 1e8);
        assert!(reciprocal.is_finite() && reciprocal > 0.0 && reciprocal < 1e-8);
        assert_eq!(reciprocal.to_bits(), (1.0 / condition).to_bits());
    }

    #[test]
    fn information_admissibility_rejects_indefinite_expected_information() {
        let error =
            invert_scaled_information_matrix(&[vec![1.0, 2.0], vec![2.0, 1.0]]).unwrap_err();

        assert!(matches!(
            error,
            CbsemExactParameterTableErrorV3::InformationMatrixNotPositiveDefinite {
                pivot: 1,
                value,
                minimum,
            } if value < 0.0 && minimum > 0.0
        ));
    }

    #[test]
    fn information_admissibility_rejects_nonpositive_inverse_variance_without_clamping() {
        let error = information_standard_errors(&[vec![-0.25, 0.0], vec![0.0, 1.0]]).unwrap_err();

        assert!(matches!(
            error,
            CbsemExactParameterTableErrorV3::InformationVarianceInvalid {
                dimension: 0,
                value,
            } if value.to_bits() == (-0.25_f64).to_bits()
        ));
    }

    #[test]
    fn information_admissibility_preserves_positive_definite_standard_errors() {
        let inverse = invert_scaled_information_matrix(&[vec![4.0, 1.0], vec![1.0, 9.0]]).unwrap();
        let standard_errors = information_standard_errors(&inverse.inverse_information).unwrap();

        assert!((standard_errors[0] - (9.0_f64 / 35.0).sqrt()).abs() <= 1e-12);
        assert!((standard_errors[1] - (4.0_f64 / 35.0).sqrt()).abs() <= 1e-12);
    }

    #[test]
    fn information_condition_fixture_is_bitwise_and_textually_deterministic() {
        let fixture = [
            vec![9.0, 3.0, 1.0],
            vec![3.0, 16.0, 2.0],
            vec![1.0, 2.0, 25.0],
        ];

        let first = invert_scaled_information_matrix(&fixture).unwrap();
        let second = invert_scaled_information_matrix(&fixture).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first.condition.canonical_diagnostic_lines(),
            second.condition.canonical_diagnostic_lines()
        );
        let [condition, reciprocal] = first.condition.canonical_diagnostic_lines();
        assert!(
            condition
                .starts_with("cbsem.expected_information.scaled_one_norm_condition_number.v1=")
        );
        assert!(reciprocal.starts_with(
            "cbsem.expected_information.scaled_reciprocal_one_norm_condition_number.v1="
        ));
    }
}

#[cfg(test)]
mod optimizer_diagnostics_tests {
    use super::*;

    #[test]
    fn optimizer_diagnostics_report_line_search_failure_with_numerical_witness() {
        let objective = |value: &[f64]| {
            Ok(if value[0] >= 0.0 {
                value[0]
            } else {
                -2.0 * value[0]
            })
        };
        let result = minimize_exact_objective(&[0.0], &objective, &|| false, &|_, _| {}).unwrap();
        assert!(!result.converged);

        assert!(matches!(
            optimizer_nonconvergence_error(&result, "test objective"),
            CbsemExactParameterTableErrorV3::OptimizerLineSearchFailed {
                stage,
                iterations: 1,
                objective,
                gradient_norm,
            } if stage == "test objective"
                && objective.to_bits() == 0.0_f64.to_bits()
                && (gradient_norm - 0.5).abs() <= 1e-8
        ));
    }

    #[test]
    fn optimizer_diagnostics_report_objective_stagnation_with_numerical_witness() {
        let objective = |value: &[f64]| Ok(-value[0]);
        let result = minimize_exact_objective(&[0.0], &objective, &|| false, &|_, _| {}).unwrap();
        assert!(!result.converged);
        let error = optimizer_nonconvergence_error(&result, "test objective");

        assert!(
            matches!(
                error,
                CbsemExactParameterTableErrorV3::OptimizerObjectiveStagnation {
                    stage,
                    iterations,
                    objective,
                    gradient_norm,
                } if stage == "test objective"
                    && iterations > 1
                    && objective.is_finite()
                    && gradient_norm
                        > CBSEM_ML_OPTIMIZER_STAGNATION_GRADIENT_NORM_TOLERANCE
            ),
            "{result:?}"
        );
    }
}
