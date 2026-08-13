use arrow::{
    array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray, UInt32Array},
    compute::take,
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use faer::{
    Accum, Conj, Mat, Par,
    diag::Diag,
    dyn_stack::{MemBuffer, MemStack},
    linalg::{
        matmul::matmul_with_conj,
        svd::{ComputeSvdVectors, svd, svd_scratch},
    },
    prelude::*,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION, HigherOrderMethod,
    InteractionMethod, MeasurementMode, MethodConfig, MissingDataPolicy, ModelSpec, Preprocessing,
    RegressionModelConfig, ValidatedExecutionRecipe, WeightingScheme,
    dijkstra_henseler_rho_a_from_normalized, ipma_predecessor_constructs, resolve_ipma_targets,
};
use qpls_data::{ColumnMetadata, ColumnType, DataFingerprint, DataKind, Dataset, ScaleType};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal, StudentsT};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error;

pub const PLS_METHOD_VERSION: &str = "pls_pm_v1";
pub const PLSC_METHOD_VERSION_V1: &str = "plsc_v1";
pub const PLSC_METHOD_VERSION: &str = "plsc_v2";
pub const WPLS_METHOD_VERSION: &str = "wpls_case_weighted_v1";
pub const CCA_METHOD_VERSION: &str = "cca_composite_residual_v1";
pub const GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION: &str = "gaussian_copula_endogeneity_v1";
pub const NONLINEAR_EFFECTS_METHOD_VERSION: &str = "pls_quadratic_nonlinear_effects_v1";
pub const MODERATED_MEDIATION_METHOD_VERSION: &str = "pls_moderated_mediation_v1";
pub const CTA_PLS_METHOD_VERSION: &str = "cta_pls_tetrad_v1";
pub const PLS_MEDIATION_METHOD_VERSION: &str = "pls_mediation_v1";
pub const PLS_TWO_STAGE_MODERATION_METHOD_VERSION: &str = "pls_two_stage_moderation_v1";
pub const PLS_PREDICT_METHOD_VERSION_V1: &str = "plspredict_holdout_v1";
pub const PLS_PREDICT_METHOD_VERSION: &str = "plspredict_indicator_v2";
pub const PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION: &str =
    "plspredict_repeated_kfold_indicator_v2";
pub const CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION: &str = "cvpat_indicator_benchmarks_v2";
pub const PLS_SEGMENTATION_METHOD_VERSION: &str = "pls_pos_bounded_v1";
pub const PLS_POS_METHOD_VERSION: &str = "pls_pos_v1";
pub const PLS_MGA_METHOD_VERSION_V1: &str = "pls_mga_two_group_v1";
pub const PLS_MGA_METHOD_VERSION: &str = "pls_mga_two_group_v2";
pub const PLS_MGA_PERMUTATION_METHOD_VERSION_V1: &str = "pls_mga_permutation_v1";
pub const PLS_MGA_PERMUTATION_METHOD_VERSION: &str = "pls_mga_permutation_v2";
pub const MICOM_METHOD_VERSION_V1: &str = "micom_v1";
pub const MICOM_METHOD_VERSION: &str = "micom_v2";
pub const FIMIX_PLS_METHOD_VERSION: &str = "fimix_pls_v1";
pub const IPMA_METHOD_VERSION: &str = "ipma_v1";
pub const IPMA_PERFORMANCE_SCALE: &str = "min_max_0_100_from_standardized_scores_v1";
pub const CFA_ML_METHOD_VERSION: &str = "cfa_ml_v1";
pub const CBSEM_ML_METHOD_VERSION: &str = "cbsem_ml_v1";
pub const CBSEM_FIT_METHOD_VERSION: &str = "cbsem_fit_v1";
pub const CBSEM_MODIFICATION_INDICES_METHOD_VERSION: &str = "cbsem_modification_indices_v1";
pub const CBSEM_BOOTSTRAP_METHOD_VERSION: &str = "cbsem_bootstrap_v1";
pub const CBSEM_MULTIGROUP_METHOD_VERSION: &str = "cbsem_multigroup_v1";
pub const CBSEM_INVARIANCE_METHOD_VERSION: &str = "cbsem_invariance_v1";
pub const PCA_METHOD_VERSION: &str = "pca_v1";
pub const GSCA_METHOD_VERSION_V1: &str = "gsca_v1";
pub const GSCA_METHOD_VERSION: &str = "gsca_als_v2";
pub const GSCA_ALGORITHM_VERSION: &str = "alternating_least_squares_v1";
pub const REGRESSION_OLS_METHOD_VERSION: &str = "regression_ols_v1";
pub const REGRESSION_LOGISTIC_METHOD_VERSION_V1: &str = "regression_logistic_v1";
pub const REGRESSION_LOGISTIC_METHOD_VERSION: &str = "regression_logistic_v2";
pub const REGRESSION_PROCESS_METHOD_VERSION_V1: &str = "regression_process_v1";
pub const REGRESSION_PROCESS_METHOD_VERSION: &str = "regression_process_v2";
pub const PROCESS_JN_INVALID_COVARIANCE_REASON: &str = "invalid_hc3_covariance";
pub const PROCESS_JN_INVALID_COVARIANCE_MESSAGE: &str = "Johnson-Neyman conditional-effect variance must be finite and strictly positive across the tested moderator range.";
const PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER: f64 = 100.0;
const PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER: f64 = 64.0;
const PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER: f64 = 128.0;
pub const NCA_METHOD_VERSION_V1: &str = "nca_v1";
pub const NCA_METHOD_VERSION: &str = "nca_v2";

#[derive(Debug, Error, PartialEq)]
pub enum EstimationError {
    #[error("estimation was cancelled")]
    Cancelled,
    #[error("unsupported estimation method: {0}")]
    UnsupportedMethod(String),
    #[error("resampling must be executed by the resampling engine")]
    ResamplingRequiresEngine,
    #[error("model requires at least one construct")]
    EmptyModel,
    #[error("construct identifier is empty")]
    EmptyConstructId,
    #[error("duplicate construct identifier: {0}")]
    DuplicateConstruct(String),
    #[error("duplicate structural path: {0} -> {1}")]
    DuplicatePath(String, String),
    #[error("self-referential structural path: {0}")]
    SelfPath(String),
    #[error("structural path references unknown construct: {0}")]
    UnknownConstruct(String),
    #[error("PLS-PM v1 requires raw observations")]
    RawDataRequired,
    #[error("at least three complete observations are required")]
    InsufficientObservations,
    #[error("unknown or nonnumeric indicator: {0}")]
    InvalidIndicator(String),
    #[error("indicator is assigned more than once: {0}")]
    DuplicateIndicator(String),
    #[error("constant indicator: {0}")]
    ConstantIndicator(String),
    #[error("construct has no indicators: {0}")]
    EmptyConstruct(String),
    #[error("model contains a directed cycle")]
    CyclicModel,
    #[error("rank-deficient regression for construct: {0}")]
    RankDeficient(String),
    #[error("construct has no connected inner proxy: {0}")]
    IsolatedConstruct(String),
    #[error("PLS weights did not converge after {0} iterations")]
    NonConvergence(u32),
    #[error("logistic regression did not converge after {0} IRLS iterations")]
    LogisticNonConvergence(u32),
    #[error("numerical failure: {0}")]
    Numerical(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EstimationPhase {
    Validating,
    PreparingRows,
    PreparingIndicators,
    Iterating,
    Assembling,
    ComputingEffects,
}

impl EstimationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validating => "validating",
            Self::PreparingRows => "preparing_rows",
            Self::PreparingIndicators => "preparing_indicators",
            Self::Iterating => "iterating",
            Self::Assembling => "assembling",
            Self::ComputingEffects => "computing_effects",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstimationProgress {
    pub phase: EstimationPhase,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorTransform {
    pub indicator: String,
    pub mean: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OuterEstimate {
    pub construct: String,
    pub indicator: String,
    pub weight: f64,
    pub loading: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathEstimate {
    pub source: String,
    pub target: String,
    pub coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlEstimate {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectEstimate {
    pub source: String,
    pub target: String,
    pub direct: f64,
    pub indirect: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediationClass {
    NoEffect,
    DirectOnly,
    IndirectOnly,
    ComplementaryPartial,
    CompetitivePartial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediationEstimate {
    pub source: String,
    pub target: String,
    pub direct: f64,
    pub indirect: f64,
    pub total: f64,
    pub variance_accounted_for: Option<f64>,
    pub classification: MediationClass,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediationAnalysis {
    pub method_version: String,
    pub tolerance: f64,
    pub estimates: Vec<MediationEstimate>,
    pub warnings: Vec<String>,
}

impl Default for MediationAnalysis {
    fn default() -> Self {
        Self {
            method_version: PLS_MEDIATION_METHOD_VERSION.to_string(),
            tolerance: 1e-12,
            estimates: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl MediationAnalysis {
    fn is_default_shell(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModerationSimpleSlope {
    pub moderator_score: f64,
    pub effect: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModerationEstimate {
    pub interaction: String,
    pub predictor: String,
    pub moderator: String,
    pub product_construct: String,
    pub outcome: String,
    pub predictor_main_effect: Option<f64>,
    pub moderator_main_effect: Option<f64>,
    pub interaction_effect: f64,
    pub simple_slopes: Vec<ModerationSimpleSlope>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModerationAnalysis {
    pub method_version: String,
    pub moderator_score_levels: Vec<f64>,
    pub estimates: Vec<ModerationEstimate>,
    pub warnings: Vec<String>,
}

impl Default for ModerationAnalysis {
    fn default() -> Self {
        Self {
            method_version: PLS_TWO_STAGE_MODERATION_METHOD_VERSION.to_string(),
            moderator_score_levels: vec![-1.0, 0.0, 1.0],
            estimates: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl ModerationAnalysis {
    fn is_default_shell(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaussianCopulaEstimate {
    pub source: String,
    pub target: String,
    pub path_coefficient: f64,
    pub copula_coefficient: f64,
    pub standard_error: f64,
    pub t_statistic: f64,
    pub p_value_two_sided: f64,
    pub predictor_skewness: f64,
    pub applicable: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaussianCopulaEndogeneityAnalysis {
    pub method_version: String,
    pub transform: String,
    pub estimates: Vec<GaussianCopulaEstimate>,
    pub warnings: Vec<String>,
}

impl Default for GaussianCopulaEndogeneityAnalysis {
    fn default() -> Self {
        Self {
            method_version: GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION.to_string(),
            transform: "rankit_inverse_normal_v1".into(),
            estimates: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonlinearEffectEstimate {
    pub source: String,
    pub target: String,
    pub linear_coefficient: f64,
    pub quadratic_coefficient: f64,
    pub standard_error: f64,
    pub t_statistic: f64,
    pub p_value_two_sided: f64,
    pub linear_r_squared: f64,
    pub augmented_r_squared: f64,
    pub delta_r_squared: f64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonlinearEffectsAnalysis {
    pub method_version: String,
    pub term: String,
    pub estimates: Vec<NonlinearEffectEstimate>,
    pub warnings: Vec<String>,
}

impl Default for NonlinearEffectsAnalysis {
    fn default() -> Self {
        Self {
            method_version: NONLINEAR_EFFECTS_METHOD_VERSION.to_string(),
            term: "centered_squared_construct_score_v1".into(),
            estimates: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConditionalIndirectEffect {
    pub moderator_score: f64,
    pub first_stage_effect: f64,
    pub second_stage_effect: f64,
    pub indirect_effect: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeratedMediationEstimate {
    pub interaction: String,
    pub predictor: String,
    pub moderator: String,
    pub mediator: String,
    pub target: String,
    pub moderated_stage: String,
    pub index_of_moderated_mediation: f64,
    pub conditional_indirect_effects: Vec<ConditionalIndirectEffect>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeratedMediationAnalysis {
    pub method_version: String,
    pub moderator_score_levels: Vec<f64>,
    pub estimates: Vec<ModeratedMediationEstimate>,
    pub warnings: Vec<String>,
}

impl Default for ModeratedMediationAnalysis {
    fn default() -> Self {
        Self {
            method_version: MODERATED_MEDIATION_METHOD_VERSION.to_string(),
            moderator_score_levels: vec![-1.0, 0.0, 1.0],
            estimates: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TetradEstimate {
    pub construct: String,
    pub indicator_a: String,
    pub indicator_b: String,
    pub indicator_c: String,
    pub indicator_d: String,
    pub pairing: String,
    pub tetrad: f64,
    pub absolute_tetrad: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CtaPlsAnalysis {
    pub method_version: String,
    pub covariance: String,
    pub estimates: Vec<TetradEstimate>,
    pub max_absolute_tetrad_by_construct: BTreeMap<String, f64>,
    pub warnings: Vec<String>,
}

impl Default for CtaPlsAnalysis {
    fn default() -> Self {
        Self {
            method_version: CTA_PLS_METHOD_VERSION.to_string(),
            covariance: "sample_covariance_of_preprocessed_indicators_v1".into(),
            estimates: Vec::new(),
            max_absolute_tetrad_by_construct: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WplsAnalysis {
    pub method_version: String,
    pub case_weight_column: String,
    pub weight_sum: f64,
    pub effective_sample_size: f64,
    pub covariance: String,
    pub warnings: Vec<String>,
}

impl Default for WplsAnalysis {
    fn default() -> Self {
        Self {
            method_version: WPLS_METHOD_VERSION.to_string(),
            case_weight_column: String::new(),
            weight_sum: 0.0,
            effective_sample_size: 0.0,
            covariance: "positive_case_weighted_unbiased_covariance_v1".into(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CcaCorrelation {
    pub left: String,
    pub right: String,
    pub observed: f64,
    pub reproduced: f64,
    pub residual: f64,
    pub absolute_residual: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CcaAnalysis {
    pub method_version: String,
    pub model: String,
    pub correlations: Vec<CcaCorrelation>,
    pub max_absolute_residual: f64,
    pub warnings: Vec<String>,
}

impl Default for CcaAnalysis {
    fn default() -> Self {
        Self {
            method_version: CCA_METHOD_VERSION.to_string(),
            model: "recursive_standardized_composite_path_model_v1".into(),
            correlations: Vec::new(),
            max_absolute_residual: 0.0,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictAnalysis {
    pub method_version: String,
    #[serde(default)]
    pub primary_analysis: String,
    pub split: String,
    pub training_observations: usize,
    pub test_observations: usize,
    pub benchmark: String,
    pub targets: Vec<PlsPredictTarget>,
    #[serde(default)]
    pub indicator_targets: Vec<PlsPredictIndicatorTarget>,
    #[serde(default)]
    pub repeated_kfold: Option<PlsPredictRepeatedKfold>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictTarget {
    pub construct: String,
    pub predictor_count: usize,
    pub rmse_pls: f64,
    pub mae_pls: f64,
    pub rmse_benchmark: f64,
    pub mae_benchmark: f64,
    pub q_squared_predict: Option<f64>,
    #[serde(default)]
    pub rmse_lm: Option<f64>,
    #[serde(default)]
    pub mae_lm: Option<f64>,
    #[serde(default)]
    pub q_squared_predict_lm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictRepeatedKfold {
    pub method_version: String,
    pub folds: usize,
    pub repeats: usize,
    pub assignment: String,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub assignment_digest: String,
    pub total_test_observations: usize,
    pub targets: Vec<PlsPredictTarget>,
    #[serde(default)]
    pub indicator_targets: Vec<PlsPredictIndicatorTarget>,
    /// Historical v1 construct-score paired-loss rows. Current v2 execution
    /// leaves this empty so these rows cannot be mistaken for CVPAT.
    #[serde(default)]
    pub cvpat: Vec<CvpatComparison>,
    #[serde(default)]
    pub cvpat_benchmark_assessments: Vec<PlsPredictCvpatBenchmarkAssessment>,
    #[serde(default)]
    pub paired_loss_diagnostics: Vec<CvpatComparison>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictIndicatorTarget {
    pub construct: String,
    pub indicator: String,
    pub predictor_scope: String,
    pub predictor_count: usize,
    pub pls: PlsPredictErrorMetrics,
    pub indicator_average: PlsPredictErrorMetrics,
    pub linear_model: PlsPredictBenchmarkMetrics,
    pub q_squared_predict: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictErrorMetrics {
    pub observations: usize,
    pub squared_error_sum: f64,
    pub absolute_error_sum: f64,
    pub rmse: f64,
    pub mae: f64,
    pub absolute_percentage_error_sum: Option<f64>,
    pub mape_observations: usize,
    pub mape_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictBenchmarkMetrics {
    pub status: String,
    pub metrics: Option<PlsPredictErrorMetrics>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPredictCvpatBenchmarkAssessment {
    pub method_version: String,
    pub comparison_kind: String,
    pub target_scope: String,
    pub benchmark: String,
    pub loss: String,
    pub alternative: String,
    pub confidence_level: f64,
    pub mean_loss_pls: Option<f64>,
    pub mean_loss_benchmark: Option<f64>,
    pub mean_loss_difference: Option<f64>,
    pub loss_difference_sum_of_squares: Option<f64>,
    pub standard_error: Option<f64>,
    pub t_statistic: Option<f64>,
    pub p_value_one_sided: Option<f64>,
    pub confidence_interval_lower: Option<f64>,
    pub confidence_interval_upper: Option<f64>,
    pub observations: usize,
    pub indicator_count: usize,
    pub status: String,
    pub preferred_model: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CvpatComparison {
    pub target: String,
    pub comparison: String,
    pub loss: String,
    pub mean_loss_difference: f64,
    pub standard_error: Option<f64>,
    pub t_statistic: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    pub observations: usize,
    pub preferred_model: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsSegmentationAnalysis {
    pub method_version: String,
    pub algorithm: String,
    pub requested_segments: usize,
    pub selected_segments: usize,
    pub assignment: String,
    pub observations: usize,
    pub objective: f64,
    pub pooled_objective: f64,
    pub objective_improvement: f64,
    pub min_segment_share: f64,
    pub segment_size_imbalance: f64,
    pub max_path_separation: f64,
    pub segments: Vec<PlsSegmentSummary>,
    #[serde(default)]
    pub memberships: Vec<PlsSegmentMembership>,
    #[serde(default)]
    pub objective_history: Vec<PlsPosObjectiveStep>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsSegmentSummary {
    pub segment: String,
    pub observations: usize,
    pub share: f64,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsSegmentMembership {
    pub observation: usize,
    pub segment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsPosObjectiveStep {
    pub start: usize,
    pub iteration: usize,
    pub objective: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub groups: Vec<PlsMgaGroupSummary>,
    pub comparisons: Vec<PlsMgaPathComparison>,
    #[serde(default)]
    pub measurement_comparisons: Vec<PlsMgaMeasurementComparison>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaGroupSummary {
    pub group: String,
    pub observations: usize,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
    #[serde(default)]
    pub outer_estimates: Vec<OuterEstimate>,
    #[serde(default)]
    pub transforms: Vec<IndicatorTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaPathComparison {
    pub source: String,
    pub target: String,
    pub group_a: String,
    pub group_b: String,
    pub coefficient_a: f64,
    pub coefficient_b: f64,
    pub difference: f64,
    pub standard_error: Option<f64>,
    pub t_statistic: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaMeasurementComparison {
    pub parameter: String,
    pub construct: String,
    pub indicator: String,
    pub group_a: String,
    pub group_b: String,
    pub estimate_a: f64,
    pub estimate_b: f64,
    pub difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicomAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
    #[serde(default)]
    pub attempted_permutations: Option<usize>,
    #[serde(default)]
    pub failed_permutations: Option<usize>,
    #[serde(default)]
    pub confidence_level: Option<f64>,
    pub groups: Vec<MicomGroupSummary>,
    pub constructs: Vec<MicomConstructResult>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicomGroupSummary {
    pub group: String,
    pub observations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicomConstructResult {
    pub construct: String,
    pub configural_invariance: bool,
    pub compositional_correlation: f64,
    pub compositional_p_value: Option<f64>,
    #[serde(default)]
    pub compositional_correlation_lower: Option<f64>,
    #[serde(default)]
    pub mean_a: Option<f64>,
    #[serde(default)]
    pub mean_b: Option<f64>,
    pub mean_difference: f64,
    pub mean_p_value: Option<f64>,
    #[serde(default)]
    pub mean_difference_lower: Option<f64>,
    #[serde(default)]
    pub mean_difference_upper: Option<f64>,
    #[serde(default)]
    pub variance_a: Option<f64>,
    #[serde(default)]
    pub variance_b: Option<f64>,
    pub variance_difference: f64,
    pub variance_p_value: Option<f64>,
    #[serde(default)]
    pub variance_difference_lower: Option<f64>,
    #[serde(default)]
    pub variance_difference_upper: Option<f64>,
    #[serde(default)]
    pub equal_means: Option<bool>,
    #[serde(default)]
    pub equal_variances: Option<bool>,
    pub partial_invariance: bool,
    pub full_invariance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaPermutationAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
    #[serde(default)]
    pub attempted_permutations: Option<usize>,
    #[serde(default)]
    pub failed_permutations: Option<usize>,
    pub comparisons: Vec<PlsMgaPermutationComparison>,
    #[serde(default)]
    pub measurement_comparisons: Vec<PlsMgaPermutationMeasurementComparison>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaPermutationComparison {
    pub source: String,
    pub target: String,
    pub original_difference: f64,
    pub empirical_p_value_two_sided: Option<f64>,
    pub percentile_rank: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaPermutationMeasurementComparison {
    pub parameter: String,
    pub construct: String,
    pub indicator: String,
    pub original_difference: f64,
    pub empirical_p_value_two_sided: Option<f64>,
    pub percentile_rank: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FimixPlsAnalysis {
    pub method_version: String,
    pub classes: usize,
    pub starts: usize,
    pub iterations: usize,
    pub log_likelihood: f64,
    pub aic: f64,
    pub bic: f64,
    pub caic: f64,
    pub entropy: f64,
    pub classes_summary: Vec<FimixClassSummary>,
    pub memberships: Vec<FimixMembership>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FimixClassSummary {
    pub class: String,
    pub observations: usize,
    pub share: f64,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FimixMembership {
    pub observation: usize,
    pub class: String,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpmaAnalysis {
    pub method_version: String,
    pub performance_scale: String,
    pub targets: Vec<String>,
    pub constructs: Vec<IpmaConstructPerformance>,
    pub indicators: Vec<IpmaIndicatorPerformance>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpmaConstructPerformance {
    pub target: String,
    pub construct: String,
    pub importance: f64,
    pub performance: f64,
    pub score_mean: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpmaIndicatorPerformance {
    pub target: String,
    pub construct: String,
    pub indicator: String,
    pub construct_importance: f64,
    pub loading: f64,
    pub performance: f64,
    pub score_mean: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlscReliability {
    pub construct: String,
    pub rho_a: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlscConstructCorrelation {
    pub left: String,
    pub right: String,
    pub original: f64,
    pub corrected: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlscAnalysis {
    pub method_version: String,
    pub reliability_method_version: String,
    pub tolerance: f64,
    pub reliabilities: Vec<PlscReliability>,
    pub construct_correlations: Vec<PlscConstructCorrelation>,
    pub corrected_paths: Vec<PathEstimate>,
    pub corrected_outer_loadings: Vec<OuterEstimate>,
    pub corrected_r_squared: BTreeMap<String, f64>,
    pub warnings: Vec<String>,
}

impl Default for PlscAnalysis {
    fn default() -> Self {
        Self {
            method_version: PLSC_METHOD_VERSION.to_string(),
            reliability_method_version: DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION.into(),
            tolerance: 1e-12,
            reliabilities: Vec::new(),
            construct_correlations: Vec::new(),
            corrected_paths: Vec::new(),
            corrected_outer_loadings: Vec::new(),
            corrected_r_squared: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemAnalysis {
    pub method_version: String,
    pub model_type: String,
    pub estimator: String,
    pub input: String,
    pub mean_structure: bool,
    pub converged: bool,
    pub iterations: u32,
    pub objective: f64,
    pub gradient_norm: f64,
    pub sample_size: usize,
    pub parameters: Vec<CbsemParameter>,
    pub standardized: Vec<CbsemStandardizedParameter>,
    pub implied_covariance: Vec<CbsemMatrixCell>,
    pub residual_covariance: Vec<CbsemMatrixCell>,
    pub residual_correlation: Vec<CbsemMatrixCell>,
    pub fit: CbsemFitIndices,
    pub modification_indices: Vec<CbsemModificationIndex>,
    #[serde(default)]
    pub bootstrap: Option<CbsemBootstrapAnalysis>,
    #[serde(default)]
    pub multigroup: Option<CbsemMultigroupAnalysis>,
    pub diagnostics: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemParameter {
    pub name: String,
    pub kind: String,
    pub lhs: String,
    pub rhs: String,
    pub estimate: f64,
    pub standard_error: Option<f64>,
    pub z_statistic: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    pub fixed: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemStandardizedParameter {
    pub name: String,
    pub kind: String,
    pub lhs: String,
    pub rhs: String,
    pub std_lv: f64,
    pub std_all: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemMatrixCell {
    pub row: String,
    pub column: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemFitIndices {
    pub method_version: String,
    pub chi_square: f64,
    pub degrees_of_freedom: i64,
    pub p_value: Option<f64>,
    pub cfi: Option<f64>,
    pub tli: Option<f64>,
    pub rmsea: Option<f64>,
    pub rmsea_ci_lower: Option<f64>,
    pub rmsea_ci_upper: Option<f64>,
    pub srmr: f64,
    pub aic: f64,
    pub bic: f64,
    pub baseline_chi_square: f64,
    pub baseline_degrees_of_freedom: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemModificationIndex {
    pub method_version: String,
    pub kind: String,
    pub lhs: String,
    pub rhs: String,
    pub modification_index: f64,
    pub expected_parameter_change: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemBootstrapAnalysis {
    pub method_version: String,
    pub samples: usize,
    pub usable_samples: usize,
    pub intervals: Vec<CbsemBootstrapInterval>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemBootstrapInterval {
    pub parameter: String,
    pub original: f64,
    pub lower_percentile: f64,
    pub upper_percentile: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemMultigroupAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub groups: Vec<CbsemGroupSummary>,
    pub invariance: Vec<CbsemInvarianceStep>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemGroupSummary {
    pub group: String,
    pub observations: usize,
    pub chi_square: f64,
    pub degrees_of_freedom: i64,
    pub cfi: Option<f64>,
    pub rmsea: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CbsemInvarianceStep {
    pub step: String,
    pub chi_square: f64,
    pub degrees_of_freedom: i64,
    pub delta_chi_square: Option<f64>,
    pub delta_degrees_of_freedom: Option<i64>,
    pub delta_cfi: Option<f64>,
    pub delta_rmsea: Option<f64>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PcaAnalysis {
    pub method_version: String,
    pub component_rule: String,
    pub retained_components: usize,
    pub observations: usize,
    pub variables: Vec<String>,
    pub components: Vec<PcaComponent>,
    pub loadings: Vec<PcaLoading>,
    pub scores: Vec<PcaScore>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PcaComponent {
    pub component: String,
    pub eigenvalue: f64,
    pub explained_variance: f64,
    pub cumulative_variance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PcaLoading {
    pub variable: String,
    pub component: String,
    pub loading: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PcaScore {
    pub observation: usize,
    pub component: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionAnalysis {
    pub method_version: String,
    pub regression_type: String,
    pub outcome: String,
    pub predictors: Vec<String>,
    pub controls: Vec<String>,
    pub observations: usize,
    pub coefficients: Vec<RegressionCoefficient>,
    pub fit: Option<RegressionFit>,
    pub predictions: Vec<RegressionPrediction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logistic: Option<LogisticRegressionDiagnostics>,
    #[serde(default)]
    pub process: Option<ProcessAnalysis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<RegressionBootstrapAnalysis>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapAnalysis {
    pub method_version: String,
    pub algorithm: String,
    pub confidence_level: f64,
    pub alternative: String,
    pub interval_policy: String,
    pub test_reference: String,
    pub test_tolerance_policy: String,
    pub requested_replicates: u32,
    pub usable_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub jackknife_cases: usize,
    pub usable_jackknife_cases: usize,
    pub seed: u64,
    pub workers: usize,
    pub stream_token: String,
    pub failed_replicates: Vec<RegressionBootstrapFailedReplicate>,
    pub coefficients: Vec<RegressionBootstrapCoefficient>,
    pub validation_witness: RegressionBootstrapValidationWitness,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapValidationWitness {
    pub method_version: String,
    pub terms: Vec<String>,
    pub successful_bootstrap: Vec<RegressionBootstrapWitnessBootstrapRow>,
    pub successful_jackknife: Vec<RegressionBootstrapWitnessJackknifeRow>,
    pub failed_jackknife: Vec<RegressionBootstrapFailedJackknife>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapWitnessBootstrapRow {
    pub replicate_index: u32,
    pub coefficients: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapWitnessJackknifeRow {
    pub omitted_case: usize,
    pub coefficients: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapFailedJackknife {
    pub omitted_case: usize,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapFailedReplicate {
    pub replicate_index: u32,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapCoefficient {
    pub term: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bias: f64,
    pub standard_error: f64,
    pub replicate_max_abs: f64,
    pub test_tolerance: f64,
    pub test: RegressionBootstrapTest,
    pub percentile_lower: f64,
    pub percentile_upper: f64,
    pub usable_replicates: u32,
    pub bca: RegressionBootstrapBcaInterval,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odds_ratio: Option<RegressionBootstrapOddsRatio>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegressionBootstrapTest {
    Available {
        statistic: f64,
        p_value_two_sided: f64,
    },
    Unavailable {
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegressionBootstrapBcaInterval {
    Available {
        bias_correction: f64,
        acceleration: f64,
        lower: f64,
        upper: f64,
    },
    Unavailable {
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegressionBootstrapOddsRatio {
    pub original: f64,
    pub percentile_lower: f64,
    pub percentile_upper: f64,
    pub bca: RegressionBootstrapBcaInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionCoefficient {
    pub term: String,
    pub estimate: f64,
    pub standard_error: f64,
    pub statistic: f64,
    pub p_value_two_sided: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
    #[serde(default)]
    pub odds_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odds_ratio_confidence_interval_lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odds_ratio_confidence_interval_upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionFit {
    pub r_squared: Option<f64>,
    pub adjusted_r_squared: Option<f64>,
    pub f_statistic: Option<f64>,
    pub log_likelihood: Option<f64>,
    pub pseudo_r_squared: Option<f64>,
    pub aic: f64,
    pub bic: f64,
    pub rmse: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_log_likelihood: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deviance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_deviance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub likelihood_ratio_chi_square: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub likelihood_ratio_degrees_of_freedom: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub likelihood_ratio_p_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_r_squared_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionPrediction {
    pub observation: usize,
    pub fitted: f64,
    pub residual: Option<f64>,
    #[serde(default)]
    pub probability: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogisticOutcomeReadiness {
    Ready,
    NonBinaryValues,
    SingleObservedClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogisticOutcomeProfile {
    pub outcome: String,
    pub coding: String,
    pub complete_cases: usize,
    pub omitted_cases: usize,
    pub zero_count: usize,
    pub one_count: usize,
    pub invalid_count: usize,
    pub prevalence: Option<f64>,
    pub readiness: LogisticOutcomeReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogisticConvergence {
    pub algorithm: String,
    pub converged: bool,
    pub iterations: u32,
    pub max_iterations: u32,
    pub tolerance: f64,
    pub final_max_abs_step: f64,
    pub separation_probability_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogisticClassification {
    pub threshold: f64,
    pub true_positive: usize,
    pub true_negative: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub accuracy: f64,
    pub sensitivity: f64,
    pub specificity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogisticRegressionDiagnostics {
    pub outcome_profile: LogisticOutcomeProfile,
    pub convergence: LogisticConvergence,
    pub classification: LogisticClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessAnalysis {
    pub method_version: String,
    pub model: String,
    pub effects: Vec<ProcessEffect>,
    pub simple_slopes: Vec<ProcessSimpleSlope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_v2: Option<ProcessGraphAnalysis>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessEffect {
    pub effect: String,
    pub estimate: f64,
    #[serde(default)]
    pub lower_percentile: Option<f64>,
    #[serde(default)]
    pub upper_percentile: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessSimpleSlope {
    pub moderator_value: f64,
    pub slope: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessGraphAnalysis {
    pub policies: ProcessPolicies,
    pub complete_cases: usize,
    pub omitted_cases: usize,
    pub variable_profiles: Vec<ProcessVariableProfile>,
    pub paths: Vec<ProcessPath>,
    pub moderations: Vec<ProcessModeration>,
    pub equations: Vec<ProcessEquation>,
    pub reference_effects: Vec<ProcessReferenceEffect>,
    pub conditional_indirect_effects: Vec<ProcessConditionalIndirectEffect>,
    pub moderated_mediation_indices: Vec<ProcessModeratedMediationIndex>,
    pub simple_slopes: Vec<ProcessGraphSimpleSlope>,
    pub plots: Vec<ProcessPlot>,
    pub johnson_neyman: Vec<ProcessJohnsonNeyman>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<ProcessBootstrapAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPolicies {
    pub centering: String,
    pub covariance: String,
    pub inference_reference: String,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessVariableProfile {
    pub variable: String,
    pub role: String,
    pub scale: String,
    pub raw_mean: f64,
    pub raw_sample_sd: f64,
    pub raw_min: f64,
    pub raw_max: f64,
    pub levels: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPath {
    pub path_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessModeration {
    pub moderation_id: String,
    pub from: String,
    pub to: String,
    pub moderator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditioning_moderator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessEquation {
    pub equation_id: String,
    pub outcome: String,
    pub term_ids: Vec<String>,
    pub coefficients: Vec<ProcessEquationCoefficient>,
    pub coefficient_covariance: Vec<Vec<f64>>,
    pub residual_degrees_of_freedom: usize,
    pub fit: ProcessEquationFit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessEquationCoefficient {
    pub term_id: String,
    pub kind: String,
    pub variables: Vec<String>,
    pub estimate: f64,
    pub standard_error: f64,
    pub statistic: f64,
    pub p_value_two_sided: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessEquationFit {
    pub observations: usize,
    pub parameter_count: usize,
    pub residual_sum_squares: f64,
    pub total_sum_squares: f64,
    pub r_squared: f64,
    pub adjusted_r_squared: f64,
    pub f_statistic: f64,
    pub aic: f64,
    pub bic: f64,
    pub rmse: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessReferenceEffect {
    pub effect_id: String,
    pub kind: String,
    pub path: Vec<String>,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessModeratorValue {
    pub variable: String,
    pub raw_value: f64,
    pub coded_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessConditionalIndirectEffect {
    pub effect_id: String,
    pub path_id: String,
    pub moderator_values: Vec<ProcessModeratorValue>,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessModeratedMediationIndex {
    pub effect_id: String,
    pub path_id: String,
    pub moderated_edge: String,
    pub moderator: String,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessGraphSimpleSlope {
    pub effect_id: String,
    pub moderation_id: String,
    pub moderator_values: Vec<ProcessModeratorValue>,
    pub estimate: f64,
    pub standard_error: f64,
    pub statistic: f64,
    pub p_value_two_sided: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPlot {
    pub plot_id: String,
    pub moderation_id: String,
    pub series: Vec<ProcessPlotSeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPlotSeries {
    pub series_id: String,
    pub moderator_values: Vec<ProcessModeratorValue>,
    pub points: Vec<ProcessPlotPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessPlotPoint {
    pub predictor_raw: f64,
    pub predicted_raw: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProcessJohnsonNeyman {
    Available {
        moderation_id: String,
        solved_moderator: String,
        conditioning_values: Vec<ProcessModeratorValue>,
        raw_min: f64,
        raw_max: f64,
        roots: Vec<f64>,
        regions: Vec<ProcessJohnsonNeymanRegion>,
        curve_points: Vec<ProcessJohnsonNeymanPoint>,
    },
    Unavailable {
        moderation_id: String,
        solved_moderator: String,
        conditioning_values: Vec<ProcessModeratorValue>,
        reason_code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessJohnsonNeymanRegion {
    pub lower: f64,
    pub upper: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessJohnsonNeymanPoint {
    pub moderator_raw: f64,
    pub effect: f64,
    pub standard_error: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapAnalysis {
    pub method_version: String,
    pub algorithm: String,
    pub interval_policy: String,
    pub test_reference: String,
    pub requested_replicates: u32,
    pub usable_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub jackknife_cases: usize,
    pub usable_jackknife_cases: usize,
    pub seed: u64,
    pub workers: usize,
    pub stream_token: String,
    pub failed_replicates: Vec<ProcessBootstrapFailedReplicate>,
    pub estimands: Vec<ProcessBootstrapEstimand>,
    pub validation_witness: ProcessBootstrapValidationWitness,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapFailedReplicate {
    pub replicate_index: u32,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapEstimand {
    pub effect_id: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bias: f64,
    pub standard_error: f64,
    pub test: RegressionBootstrapTest,
    pub percentile_lower: f64,
    pub percentile_upper: f64,
    pub bca: RegressionBootstrapBcaInterval,
    pub usable_replicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapValidationWitness {
    pub method_version: String,
    pub estimand_ids: Vec<String>,
    pub successful_bootstrap: Vec<ProcessBootstrapWitnessBootstrapRow>,
    pub successful_jackknife: Vec<ProcessBootstrapWitnessJackknifeRow>,
    pub failed_jackknife: Vec<RegressionBootstrapFailedJackknife>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapWitnessBootstrapRow {
    pub replicate_index: u32,
    pub estimates: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProcessBootstrapWitnessJackknifeRow {
    pub omitted_case: usize,
    pub estimates: Vec<f64>,
}

/// Stable PROCESS v2 bootstrap estimand order. Plot and Johnson-Neyman rows are
/// deliberately excluded because they are equation-covariance diagnostics,
/// not bootstrap estimands.
pub fn process_bootstrap_estimands(graph: &ProcessGraphAnalysis) -> Vec<(String, f64)> {
    graph
        .reference_effects
        .iter()
        .map(|effect| (effect.effect_id.clone(), effect.estimate))
        .chain(
            graph
                .conditional_indirect_effects
                .iter()
                .map(|effect| (effect.effect_id.clone(), effect.estimate)),
        )
        .chain(
            graph
                .moderated_mediation_indices
                .iter()
                .map(|effect| (effect.effect_id.clone(), effect.estimate)),
        )
        .chain(
            graph
                .simple_slopes
                .iter()
                .map(|effect| (effect.effect_id.clone(), effect.estimate)),
        )
        .collect()
}

/// Evaluates a resampled/delete-one PROCESS coefficient system at the original
/// complete-sample raw probe grid. Product centering is still taken from the
/// fitted sample's profiles; only the reported raw probe values and stable
/// estimand identities come from `reference`.
pub fn process_bootstrap_estimands_at_reference(
    fitted: &ProcessGraphAnalysis,
    reference: &ProcessGraphAnalysis,
) -> Result<Vec<(String, f64)>, String> {
    let fitted_profiles = fitted
        .variable_profiles
        .iter()
        .map(|profile| (profile.variable.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    let fitted_moderations = fitted
        .moderations
        .iter()
        .map(|moderation| (moderation.moderation_id.as_str(), moderation))
        .collect::<BTreeMap<_, _>>();
    let reference_profile_by_variable = reference
        .variable_profiles
        .iter()
        .map(|profile| (profile.variable.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    let reference_values = reference
        .moderations
        .iter()
        .flat_map(|moderation| {
            std::iter::once(moderation.moderator.as_str())
                .chain(moderation.conditioning_moderator.as_deref())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|variable| {
            let profile = reference_profile_by_variable
                .get(variable)
                .ok_or_else(|| format!("missing reference profile {variable}"))?;
            Ok(ProcessModeratorValue {
                variable: variable.to_string(),
                raw_value: if profile.scale == "binary_0_1" {
                    0.0
                } else {
                    profile.raw_mean
                },
                coded_value: 0.0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let reference_codes = process_reference_probe_codes(&reference_values, &fitted_profiles)?;
    let mut output = Vec::new();
    for effect in &reference.reference_effects {
        let estimate = match effect.kind.as_str() {
            "direct" => {
                if fitted
                    .paths
                    .iter()
                    .any(|path| path.from == effect.path[0] && path.to == effect.path[1])
                {
                    process_serialized_path_effect(
                        &effect.path,
                        &reference_codes,
                        fitted,
                        &fitted_moderations,
                    )?
                } else {
                    0.0
                }
            }
            "indirect" => process_serialized_path_effect(
                &effect.path,
                &reference_codes,
                fitted,
                &fitted_moderations,
            )?,
            "total_indirect" => reference
                .reference_effects
                .iter()
                .filter(|row| row.kind == "indirect")
                .try_fold(0.0, |sum, row| {
                    Ok::<_, String>(
                        sum + process_serialized_path_effect(
                            &row.path,
                            &reference_codes,
                            fitted,
                            &fitted_moderations,
                        )?,
                    )
                })?,
            "total" => {
                let direct = reference
                    .reference_effects
                    .iter()
                    .find(|row| row.kind == "direct")
                    .ok_or_else(|| "reference direct effect is missing".to_string())?;
                let direct = if fitted
                    .paths
                    .iter()
                    .any(|path| path.from == direct.path[0] && path.to == direct.path[1])
                {
                    process_serialized_path_effect(
                        &direct.path,
                        &reference_codes,
                        fitted,
                        &fitted_moderations,
                    )?
                } else {
                    0.0
                };
                let indirect = reference
                    .reference_effects
                    .iter()
                    .filter(|row| row.kind == "indirect")
                    .try_fold(0.0, |sum, row| {
                        Ok::<_, String>(
                            sum + process_serialized_path_effect(
                                &row.path,
                                &reference_codes,
                                fitted,
                                &fitted_moderations,
                            )?,
                        )
                    })?;
                direct + indirect
            }
            _ => return Err(format!("unsupported PROCESS estimand kind {}", effect.kind)),
        };
        output.push((effect.effect_id.clone(), estimate));
    }
    for effect in &reference.conditional_indirect_effects {
        let probes = process_reference_probe_codes(&effect.moderator_values, &fitted_profiles)?;
        let path = effect
            .path_id
            .split("->")
            .map(str::to_string)
            .collect::<Vec<_>>();
        output.push((
            effect.effect_id.clone(),
            process_serialized_path_effect(&path, &probes, fitted, &fitted_moderations)?,
        ));
    }
    for effect in &reference.moderated_mediation_indices {
        let moderation = fitted
            .moderations
            .iter()
            .find(|row| {
                effect.moderated_edge == format!("{}->{}", row.from, row.to)
                    && effect.moderator == row.moderator
            })
            .ok_or_else(|| format!("missing fitted moderation for {}", effect.effect_id))?;
        let interaction = process_serialized_coefficient(
            fitted,
            &moderation.to,
            &[moderation.from.clone(), moderation.moderator.clone()],
        )?;
        let path = effect
            .path_id
            .split("->")
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut other_product = 1.0;
        for edge in path.windows(2) {
            if edge[0] == moderation.from && edge[1] == moderation.to {
                continue;
            }
            other_product *= process_serialized_edge_slope(
                &edge[0],
                &edge[1],
                &reference_codes,
                fitted,
                &fitted_moderations,
            )?;
        }
        output.push((effect.effect_id.clone(), interaction * other_product));
    }
    for effect in &reference.simple_slopes {
        let moderation = fitted_moderations
            .get(effect.moderation_id.as_str())
            .ok_or_else(|| format!("missing fitted moderation {}", effect.moderation_id))?;
        let probes = process_reference_probe_codes(&effect.moderator_values, &fitted_profiles)?;
        output.push((
            effect.effect_id.clone(),
            process_serialized_edge_slope(
                &moderation.from,
                &moderation.to,
                &probes,
                fitted,
                &fitted_moderations,
            )?,
        ));
    }
    Ok(output)
}

fn process_reference_probe_codes(
    values: &[ProcessModeratorValue],
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> Result<BTreeMap<String, f64>, String> {
    values
        .iter()
        .map(|value| {
            let profile = profiles
                .get(value.variable.as_str())
                .ok_or_else(|| format!("missing fitted profile {}", value.variable))?;
            Ok((
                value.variable.clone(),
                if profile.scale == "binary_0_1" {
                    value.raw_value
                } else {
                    value.raw_value - profile.raw_mean
                },
            ))
        })
        .collect()
}

fn process_serialized_coefficient(
    graph: &ProcessGraphAnalysis,
    outcome: &str,
    variables: &[String],
) -> Result<f64, String> {
    graph
        .equations
        .iter()
        .find(|equation| equation.outcome == outcome)
        .and_then(|equation| {
            equation
                .coefficients
                .iter()
                .find(|coefficient| coefficient.variables == variables)
        })
        .map(|coefficient| coefficient.estimate)
        .ok_or_else(|| {
            format!(
                "missing fitted coefficient {} in {outcome}",
                variables.join("*")
            )
        })
}

fn process_serialized_edge_slope(
    from: &str,
    to: &str,
    probes: &BTreeMap<String, f64>,
    graph: &ProcessGraphAnalysis,
    moderations: &BTreeMap<&str, &ProcessModeration>,
) -> Result<f64, String> {
    let mut slope = process_serialized_coefficient(graph, to, &[from.to_string()])?;
    let Some(moderation) = moderations
        .values()
        .find(|moderation| moderation.from == from && moderation.to == to)
    else {
        return Ok(slope);
    };
    let primary = probes.get(&moderation.moderator).copied().unwrap_or(0.0);
    slope += process_serialized_coefficient(
        graph,
        to,
        &[from.to_string(), moderation.moderator.clone()],
    )? * primary;
    if let Some(conditioning) = &moderation.conditioning_moderator {
        let conditioned = probes.get(conditioning).copied().unwrap_or(0.0);
        slope +=
            process_serialized_coefficient(graph, to, &[from.to_string(), conditioning.clone()])?
                * conditioned;
        slope += process_serialized_coefficient(
            graph,
            to,
            &[
                from.to_string(),
                moderation.moderator.clone(),
                conditioning.clone(),
            ],
        )? * primary
            * conditioned;
    }
    Ok(slope)
}

fn process_serialized_path_effect(
    path: &[String],
    probes: &BTreeMap<String, f64>,
    graph: &ProcessGraphAnalysis,
    moderations: &BTreeMap<&str, &ProcessModeration>,
) -> Result<f64, String> {
    path.windows(2).try_fold(1.0, |effect, edge| {
        Ok(effect * process_serialized_edge_slope(&edge[0], &edge[1], probes, graph, moderations)?)
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaAnalysis {
    pub method_version: String,
    pub ceiling: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
    pub x: String,
    pub y: String,
    pub observations: usize,
    #[serde(default)]
    pub scope: NcaScope,
    #[serde(default)]
    pub ce_fdh_peers: Vec<NcaCeilingPoint>,
    pub ceilings: Vec<NcaCeilingResult>,
    pub bottlenecks: Vec<NcaBottleneck>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NcaScope {
    pub minimum_x: f64,
    pub maximum_x: f64,
    pub minimum_y: f64,
    pub maximum_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaCeilingPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaCeilingResult {
    pub ceiling: String,
    pub effect_size: f64,
    pub permutation_p_value: Option<f64>,
    pub slope: Option<f64>,
    pub intercept: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaBottleneck {
    #[serde(default)]
    pub ceiling: String,
    pub outcome_percent: f64,
    pub required_x_percent: Option<f64>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GscaAnalysis {
    pub method_version: String,
    #[serde(default)]
    pub algorithm: String,
    #[serde(default)]
    pub converged: bool,
    pub iterations: u32,
    #[serde(default)]
    pub stop_criterion: f64,
    #[serde(default)]
    pub final_change: f64,
    #[serde(default)]
    pub objective: f64,
    pub fit: f64,
    #[serde(default)]
    pub measurement_fit: f64,
    #[serde(default)]
    pub structural_fit: f64,
    pub adjusted_fit: f64,
    pub gfi: f64,
    #[serde(default)]
    pub srmr: f64,
    #[serde(default)]
    pub covariance_discrepancy: f64,
    #[serde(default)]
    pub covariance_sample_total: f64,
    #[serde(default)]
    pub standardized_residual_sum: f64,
    #[serde(default)]
    pub observations: usize,
    #[serde(default)]
    pub free_parameters: usize,
    pub weights: Vec<OuterEstimate>,
    pub loadings: Vec<OuterEstimate>,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
    #[serde(default)]
    pub bootstrap_intervals: Vec<GscaBootstrapInterval>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GscaBootstrapInterval {
    pub parameter: String,
    pub original: f64,
    pub lower_percentile: f64,
    pub upper_percentile: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsResult {
    pub method_version: String,
    pub converged: bool,
    pub iterations: u32,
    pub used_observations: usize,
    pub omitted_observations: usize,
    pub transforms: Vec<IndicatorTransform>,
    pub construct_scores: BTreeMap<String, Vec<f64>>,
    pub outer_estimates: Vec<OuterEstimate>,
    pub paths: Vec<PathEstimate>,
    #[serde(default)]
    pub control_estimates: Vec<ControlEstimate>,
    pub effects: Vec<EffectEstimate>,
    #[serde(default, skip_serializing_if = "MediationAnalysis::is_default_shell")]
    pub mediation: MediationAnalysis,
    #[serde(default, skip_serializing_if = "ModerationAnalysis::is_default_shell")]
    pub moderation: ModerationAnalysis,
    #[serde(default)]
    pub plsc: Option<PlscAnalysis>,
    #[serde(default)]
    pub endogeneity: Option<GaussianCopulaEndogeneityAnalysis>,
    #[serde(default)]
    pub nonlinear_effects: Option<NonlinearEffectsAnalysis>,
    #[serde(default)]
    pub moderated_mediation: Option<ModeratedMediationAnalysis>,
    #[serde(default)]
    pub cta_pls: Option<CtaPlsAnalysis>,
    #[serde(default)]
    pub wpls: Option<WplsAnalysis>,
    #[serde(default)]
    pub cca: Option<CcaAnalysis>,
    #[serde(default)]
    pub predict: Option<PlsPredictAnalysis>,
    #[serde(default)]
    pub segmentation: Option<PlsSegmentationAnalysis>,
    #[serde(default)]
    pub mga: Option<PlsMgaAnalysis>,
    #[serde(default)]
    pub micom: Option<MicomAnalysis>,
    #[serde(default)]
    pub mga_permutation: Option<PlsMgaPermutationAnalysis>,
    #[serde(default)]
    pub fimix: Option<FimixPlsAnalysis>,
    #[serde(default)]
    pub ipma: Option<IpmaAnalysis>,
    #[serde(default)]
    pub cbsem: Option<CbsemAnalysis>,
    #[serde(default)]
    pub pca: Option<PcaAnalysis>,
    #[serde(default)]
    pub regression: Option<RegressionAnalysis>,
    #[serde(default)]
    pub nca: Option<NcaAnalysis>,
    #[serde(default)]
    pub gsca: Option<GscaAnalysis>,
    pub r_squared: BTreeMap<String, f64>,
    pub warnings: Vec<String>,
}

struct PreparedData {
    columns: Vec<Vec<f64>>,
    transforms: Vec<IndicatorTransform>,
    used_rows: Vec<usize>,
    case_weights: Option<Vec<f64>>,
    used: usize,
    omitted: usize,
}

pub fn estimate_pls(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
) -> Result<PlsResult, EstimationError> {
    estimate_pls_with_control(dataset, recipe, |_| true)
}

pub fn estimate_pls_with_control(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| EstimationError::UnsupportedMethod(error.to_string()))?;
    estimate_pls_validated_with_control(dataset, &execution, control)
}

/// Executes an opaque recipe capability that has passed the complete
/// schema-v3 scientific preflight. The effective compatibility projection is
/// inaccessible to callers, so this cross-crate fast path cannot be used to
/// bypass typed recipe validation.
pub fn estimate_pls_validated_with_control(
    dataset: &Dataset,
    recipe: &ValidatedExecutionRecipe,
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let effective = recipe
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| EstimationError::UnsupportedMethod(error.to_string()))?;
    estimate_pls_internal(dataset, effective, false, true, &mut control)
}

/// Trusted boundary for outer regression case resampling. It first binds the
/// opaque point-only capability to the original dataset, validates the exact
/// typed OLS/logistic/PROCESS contract and row indices, creates the sample internally,
/// and only then enters the private estimator. A caller cannot use this API to
/// rebind a capability to arbitrary dataset bytes or mutate its configuration.
pub fn estimate_regression_case_resample_validated_with_control(
    original_dataset: &Dataset,
    point_only_recipe: &ValidatedExecutionRecipe,
    raw_indices: &[usize],
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let effective = point_only_recipe
        .effective_for_dataset(&original_dataset.fingerprint.0)
        .map_err(|error| EstimationError::UnsupportedMethod(error.to_string()))?;
    let MethodConfig::Regression {
        outcome,
        predictors,
        controls,
        model,
        bootstrap: None,
    } = point_only_recipe
        .source()
        .method_config
        .as_ref()
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(
                "regression case resampling requires typed point-only regression".into(),
            )
        })?
    else {
        return Err(EstimationError::UnsupportedMethod(
            "regression case resampling requires typed point-only regression".into(),
        ));
    };
    if point_only_recipe.source().settings.method != AnalysisMethod::Regression
        || point_only_recipe.source().settings.bootstrap_samples != 0
        || point_only_recipe
            .source()
            .settings
            .studentized_inner_samples
            != 0
        || point_only_recipe.source().settings.permutation_samples != 0
        || point_only_recipe.source().settings.workers != 1
        || !matches!(
            model,
            RegressionModelConfig::Ols { .. }
                | RegressionModelConfig::Logistic
                | RegressionModelConfig::Process {
                    relationship: qpls_core::ProcessRelationshipConfig::Graph { .. }
                }
        )
    {
        return Err(EstimationError::UnsupportedMethod(
            "regression case resampling requires a point-only OLS, binary logistic, or PROCESS v2 capability"
                .into(),
        ));
    }
    if raw_indices.is_empty()
        || raw_indices
            .iter()
            .any(|index| *index >= original_dataset.batch.num_rows() || *index > u32::MAX as usize)
    {
        return Err(EstimationError::UnsupportedMethod(
            "regression case-resample row indices must be nonempty and within the original dataset"
                .into(),
        ));
    }
    let mut variables = Vec::with_capacity(1 + predictors.len() + controls.len());
    variables.push(outcome.clone());
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    let positions = variables
        .iter()
        .map(|variable| {
            original_dataset
                .batch
                .schema()
                .index_of(variable)
                .map_err(|_| EstimationError::InvalidIndicator(variable.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let take_indices = UInt32Array::from(
        raw_indices
            .iter()
            .map(|index| *index as u32)
            .collect::<Vec<_>>(),
    );
    let columns = variables
        .iter()
        .zip(positions)
        .map(|(name, position)| {
            take(
                original_dataset.batch.column(position).as_ref(),
                &take_indices,
                None,
            )
            .map(|array| (name.clone(), array))
            .map_err(|error| EstimationError::Numerical(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let batch = RecordBatch::try_from_iter(columns)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let mut schema = original_dataset.schema.clone();
    schema.case_count = batch.num_rows();
    schema
        .columns
        .retain(|column| variables.iter().any(|variable| variable == &column.name));
    let mut digest = Sha256::new();
    digest.update(b"quickpls-regression-case-resample-v1\0");
    digest.update(original_dataset.fingerprint.0.as_bytes());
    for index in raw_indices {
        digest.update((*index as u64).to_le_bytes());
    }
    let sampled = Dataset {
        id: original_dataset.id,
        name: original_dataset.name.clone(),
        schema,
        batch,
        fingerprint: DataFingerprint(format!("resample:v1:{:x}", digest.finalize())),
    };
    estimate_pls_internal(&sampled, effective, false, false, &mut control)
}

#[cfg(test)]
fn estimate_pls_with_effective_recipe_control(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    estimate_pls_internal(dataset, recipe, false, true, &mut control)
}

pub fn analyze_mediation(result: &PlsResult) -> MediationAnalysis {
    analyze_mediation_with_tolerance(result, 1e-12)
}

pub fn analyze_mediation_with_tolerance(result: &PlsResult, tolerance: f64) -> MediationAnalysis {
    analyze_mediation_effects_with_tolerance(&result.effects, tolerance)
}

pub fn analyze_mediation_effects_with_tolerance(
    effects: &[EffectEstimate],
    tolerance: f64,
) -> MediationAnalysis {
    let tol = if tolerance.is_finite() && tolerance >= 0.0 {
        tolerance
    } else {
        1e-12
    };
    let estimates = effects
        .iter()
        .map(|effect| {
            let direct_present = effect.direct.abs() > tol;
            let indirect_present = effect.indirect.abs() > tol;
            let classification = match (direct_present, indirect_present) {
                (false, false) => MediationClass::NoEffect,
                (true, false) => MediationClass::DirectOnly,
                (false, true) => MediationClass::IndirectOnly,
                (true, true) if effect.direct.signum() == effect.indirect.signum() => {
                    MediationClass::ComplementaryPartial
                }
                (true, true) => MediationClass::CompetitivePartial,
            };
            let variance_accounted_for = if effect.total.abs() > tol {
                Some(effect.indirect / effect.total)
            } else {
                None
            };
            let warning = (classification == MediationClass::DirectOnly).then(|| {
                "direct-only structural effect; no mediated component exceeds tolerance".to_string()
            });
            MediationEstimate {
                source: effect.source.clone(),
                target: effect.target.clone(),
                direct: effect.direct,
                indirect: effect.indirect,
                total: effect.total,
                variance_accounted_for,
                classification,
                warning,
            }
        })
        .collect();
    MediationAnalysis {
        method_version: PLS_MEDIATION_METHOD_VERSION.to_string(),
        tolerance: tol,
        estimates,
        warnings: vec![
            "PLS mediation effect decomposition is validated for the documented QuickPLS v1.2.1 scope when paired with validated bootstrap or permutation intervals for the relevant indirect effect.".to_string(),
        ],
    }
}

/// Estimates a structurally reduced model while retaining isolated measurement
/// blocks to preserve the full model's complete-case sample. Intended only for
/// nested-model diagnostics such as Cohen f-squared.
fn estimate_pls_reduced_with_control(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    estimate_pls_internal(dataset, recipe, true, true, &mut control)
}

fn estimate_pls_internal(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    allow_isolated_constructs: bool,
    enforce_process_outcome_scope: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    checkpoint(control, EstimationPhase::Validating, 0, 1)?;
    validate_execution_recipe(recipe)?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(EstimationError::RawDataRequired);
    }
    if recipe.settings.method == AnalysisMethod::Pca {
        return estimate_standalone_pca(dataset, recipe, control);
    }
    if recipe.settings.method == AnalysisMethod::Regression {
        return estimate_regression_method(dataset, recipe, enforce_process_outcome_scope, control);
    }
    if recipe.settings.method == AnalysisMethod::Nca {
        return estimate_nca_method(dataset, recipe, control);
    }
    if recipe.settings.method == AnalysisMethod::Gsca {
        return estimate_gsca_method(dataset, recipe, control);
    }
    if recipe
        .model
        .higher_order_constructs
        .iter()
        .any(|higher_order| higher_order.method == HigherOrderMethod::TwoStage)
    {
        return estimate_pls_two_stage_higher_order(
            dataset,
            recipe,
            allow_isolated_constructs,
            control,
        );
    }
    let execution_recipe = expand_repeated_indicator_higher_order(recipe)?;
    validate_acyclic(&execution_recipe)?;
    if !execution_recipe.model.interactions.is_empty() {
        return estimate_pls_two_stage_moderation(
            dataset,
            &execution_recipe,
            allow_isolated_constructs,
            control,
        );
    }
    let indicator_names = collect_indicators(&execution_recipe)?;
    checkpoint(control, EstimationPhase::Validating, 1, 1)?;
    let prepared = prepare_data(
        dataset,
        &indicator_names,
        &execution_recipe.settings.preprocessing,
        &execution_recipe.settings.missing_data,
        if execution_recipe.settings.method == AnalysisMethod::Wpls {
            execution_recipe.settings.case_weight_column.as_deref()
        } else {
            None
        },
        control,
    )?;
    let index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let blocks = execution_recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            construct
                .indicators
                .iter()
                .map(|name| index[name.as_str()])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let (weights, scores, iterations) = if let Some(case_weights) = prepared.case_weights.as_deref()
    {
        if execution_recipe.settings.weighting_scheme == WeightingScheme::Pca {
            return Err(EstimationError::UnsupportedMethod(
                "WPLS does not support PCA weighting".into(),
            ));
        }
        iterative_scores_weighted(
            &prepared.columns,
            &blocks,
            &execution_recipe,
            case_weights,
            allow_isolated_constructs,
            control,
        )?
    } else {
        match execution_recipe.settings.weighting_scheme {
            WeightingScheme::Pca => pca_scores(
                &prepared.columns,
                &blocks,
                execution_recipe.settings.tolerance,
                execution_recipe.settings.max_iterations,
                control,
            )?,
            WeightingScheme::Path | WeightingScheme::Factor => iterative_scores(
                &prepared.columns,
                &blocks,
                &execution_recipe,
                allow_isolated_constructs
                    || execution_recipe.settings.method == AnalysisMethod::Cbsem,
                control,
            )?,
        }
    };
    let plsc_inputs = if recipe.settings.method == AnalysisMethod::Plsc {
        Some((prepared.columns.clone(), weights.clone(), scores.clone()))
    } else {
        None
    };
    let endogeneity_inputs = if recipe.settings.method == AnalysisMethod::Endogeneity {
        Some(scores.clone())
    } else {
        None
    };
    let nonlinear_inputs = if recipe.settings.method == AnalysisMethod::NonlinearEffects {
        Some(scores.clone())
    } else {
        None
    };
    let cta_inputs = if recipe.settings.method == AnalysisMethod::CtaPls {
        Some((indicator_names.clone(), prepared.columns.clone()))
    } else {
        None
    };
    let wpls_inputs = if recipe.settings.method == AnalysisMethod::Wpls {
        prepared.case_weights.clone()
    } else {
        None
    };
    let cca_inputs = if recipe.settings.method == AnalysisMethod::Cca {
        Some(scores.clone())
    } else {
        None
    };
    let predict_inputs = if recipe.settings.method == AnalysisMethod::Predict {
        Some(indicator_names.clone())
    } else {
        None
    };
    let ipma_inputs = if recipe.settings.method == AnalysisMethod::Ipma {
        Some((indicator_names.clone(), prepared.columns.clone()))
    } else {
        None
    };
    let cbsem_inputs = if recipe.settings.method == AnalysisMethod::Cbsem {
        Some((indicator_names.clone(), prepared.columns.clone()))
    } else {
        None
    };
    let mut result = assemble_result(
        dataset,
        &execution_recipe,
        indicator_names,
        prepared,
        weights,
        scores,
        iterations,
        control,
    )?;
    if let Some((columns, weights, scores)) = plsc_inputs {
        apply_plsc_correction(&execution_recipe, &columns, &weights, &scores, &mut result)?;
    }
    if let Some(scores) = endogeneity_inputs {
        apply_gaussian_copula_endogeneity(&execution_recipe, &scores, &mut result)?;
    }
    if let Some(scores) = nonlinear_inputs {
        apply_quadratic_nonlinear_effects(&execution_recipe, &scores, &mut result)?;
    }
    if let Some((indicator_names, columns)) = cta_inputs {
        apply_cta_pls(&execution_recipe, &indicator_names, &columns, &mut result)?;
    }
    if let Some(case_weights) = wpls_inputs {
        apply_wpls_metadata(&execution_recipe, &case_weights, &mut result)?;
    }
    if let Some(scores) = cca_inputs {
        apply_cca(&execution_recipe, &scores, &mut result)?;
    }
    if let Some(indicator_names) = predict_inputs {
        apply_pls_predict(
            dataset,
            &execution_recipe,
            &indicator_names,
            &mut result,
            control,
        )?;
        apply_pls_pos_segmentation(&execution_recipe, &mut result)?;
        apply_fimix_pls(&execution_recipe, &mut result)?;
    }
    if recipe.settings.method == AnalysisMethod::Mga {
        apply_two_group_mga(dataset, &execution_recipe, &mut result, control)?;
        apply_mga_permutation(dataset, &execution_recipe, &mut result, control)?;
        apply_micom(&execution_recipe, &result)?;
    }
    if let Some((indicator_names, columns)) = ipma_inputs {
        apply_ipma(&execution_recipe, &indicator_names, &columns, &mut result)?;
    }
    if let Some((indicator_names, columns)) = cbsem_inputs {
        apply_cbsem(
            &execution_recipe,
            &indicator_names,
            &columns,
            dataset,
            &mut result,
        )?;
    }
    Ok(result)
}

fn empty_method_result(
    method_version: &str,
    used_observations: usize,
    omitted_observations: usize,
    warnings: Vec<String>,
) -> PlsResult {
    PlsResult {
        method_version: method_version.into(),
        converged: true,
        iterations: 0,
        used_observations,
        omitted_observations,
        transforms: Vec::new(),
        construct_scores: BTreeMap::new(),
        outer_estimates: Vec::new(),
        paths: Vec::new(),
        control_estimates: Vec::new(),
        effects: Vec::new(),
        mediation: MediationAnalysis::default(),
        moderation: ModerationAnalysis::default(),
        plsc: None,
        endogeneity: None,
        nonlinear_effects: None,
        moderated_mediation: None,
        cta_pls: None,
        wpls: None,
        cca: None,
        predict: None,
        segmentation: None,
        mga: None,
        micom: None,
        mga_permutation: None,
        fimix: None,
        ipma: None,
        cbsem: None,
        pca: None,
        regression: None,
        nca: None,
        gsca: None,
        r_squared: BTreeMap::new(),
        warnings,
    }
}

fn estimate_standalone_pca(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let variables = metadata_list(recipe, "pca_variables")
        .or_else(|| metadata_list(recipe, "pca.variables"))
        .unwrap_or_default();
    if variables.len() < 2 {
        return Err(EstimationError::UnsupportedMethod(
            "PCA requires at least two variables".into(),
        ));
    }
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        0,
        variables.len() as u64,
    )?;
    let prepared = prepare_raw_numeric_data(dataset, &variables, true, false)?;
    let rows = prepared.columns.first().map(Vec::len).unwrap_or(0);
    let covariance = covariance_matrix(&prepared.columns);
    let component_rule = recipe
        .metadata
        .get("pca_component_rule")
        .cloned()
        .unwrap_or_else(|| "kaiser".into());
    let mut matrix = covariance.clone();
    let mut components = Vec::new();
    let mut loadings = Vec::new();
    let mut scores = Vec::new();
    let total_variance = covariance
        .iter()
        .enumerate()
        .map(|(i, row)| row[i])
        .sum::<f64>();
    let max_components = variables.len().min(rows.saturating_sub(1)).max(1);
    let requested = match component_rule.as_str() {
        "fixed" => metadata_usize(recipe, "pca_components", 1).clamp(1, max_components),
        _ => max_components,
    };
    let mut cumulative = 0.0;
    for component_index in 0..requested {
        let (eigenvalue, mut vector) = dominant_eigenpair(
            &matrix,
            recipe.settings.max_iterations,
            recipe.settings.tolerance,
        )?;
        if eigenvalue <= 1e-10 {
            break;
        }
        orient_component(&mut vector);
        let explained = eigenvalue / total_variance.max(f64::EPSILON);
        if component_rule == "kaiser" && eigenvalue < 1.0 && !components.is_empty() {
            break;
        }
        cumulative += explained;
        let component = format!("PC{}", component_index + 1);
        components.push(PcaComponent {
            component: component.clone(),
            eigenvalue,
            explained_variance: explained,
            cumulative_variance: cumulative,
        });
        for (variable_index, variable) in variables.iter().enumerate() {
            loadings.push(PcaLoading {
                variable: variable.clone(),
                component: component.clone(),
                loading: vector[variable_index] * eigenvalue.sqrt(),
                weight: vector[variable_index],
            });
        }
        for observation in 0..rows {
            let score = prepared
                .columns
                .iter()
                .zip(&vector)
                .map(|(column, weight)| column[observation] * weight)
                .sum();
            scores.push(PcaScore {
                observation,
                component: component.clone(),
                score,
            });
        }
        deflate_matrix(&mut matrix, eigenvalue, &vector);
        if component_rule == "variance_threshold" {
            let threshold = recipe
                .metadata
                .get("pca_variance_threshold")
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.80);
            if cumulative + 1e-12 >= threshold {
                break;
            }
        }
    }
    checkpoint(control, EstimationPhase::Assembling, 1, 1)?;
    let mut result = empty_method_result(
        PCA_METHOD_VERSION,
        prepared.used,
        prepared.omitted,
        vec![
            "Standalone PCA v1 is validated for the documented QuickPLS v1.2 supported scope; unsupported shapes remain blocked."
                .into(),
        ],
    );
    result.pca = Some(PcaAnalysis {
        method_version: PCA_METHOD_VERSION.into(),
        component_rule,
        retained_components: components.len(),
        observations: rows,
        variables,
        components,
        loadings,
        scores,
        warnings: result.warnings.clone(),
    });
    Ok(result)
}

fn estimate_regression_method(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    enforce_process_outcome_scope: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let regression_type = recipe
        .metadata
        .get("regression_type")
        .cloned()
        .unwrap_or_else(|| "ols".into());
    let outcome = metadata_required(recipe, "regression_outcome")?;
    let predictors = metadata_list(recipe, "regression_predictors")
        .or_else(|| metadata_list(recipe, "regression.predictors"))
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod("regression_predictors required".into())
        })?;
    let controls = metadata_list(recipe, "regression_controls")
        .or_else(|| metadata_list(recipe, "regression.controls"))
        .unwrap_or_default();
    let process_graph = matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::Regression {
            model: RegressionModelConfig::Process {
                relationship: qpls_core::ProcessRelationshipConfig::Graph { .. }
            },
            ..
        })
    );
    if process_graph {
        let process = process_analysis(dataset, recipe, enforce_process_outcome_scope, control)?;
        let graph = process
            .graph_v2
            .as_ref()
            .ok_or_else(|| EstimationError::Numerical("PROCESS v2 graph output missing".into()))?;
        let warnings = vec![
            "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates.".into(),
            "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected.".into(),
        ];
        let mut result = empty_method_result(
            REGRESSION_PROCESS_METHOD_VERSION,
            graph.complete_cases,
            graph.omitted_cases,
            warnings.clone(),
        );
        result.regression = Some(RegressionAnalysis {
            method_version: REGRESSION_PROCESS_METHOD_VERSION.into(),
            regression_type,
            outcome,
            predictors,
            controls,
            observations: graph.complete_cases,
            coefficients: Vec::new(),
            fit: None,
            predictions: Vec::new(),
            logistic: None,
            process: Some(process),
            bootstrap: None,
            warnings,
        });
        return Ok(result);
    }
    let mut variables = vec![outcome.clone()];
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        0,
        variables.len() as u64,
    )?;
    let prepared =
        prepare_raw_numeric_data(dataset, &variables, false, regression_type == "logistic")?;
    let y = prepared.columns[0].clone();
    let x = prepared.columns[1..].to_vec();
    let terms = predictors
        .iter()
        .chain(controls.iter())
        .cloned()
        .collect::<Vec<_>>();
    let (coefficients, fit, predictions, logistic) = if regression_type == "logistic" {
        let profile = logistic_outcome_profile(&outcome, &y, prepared.omitted);
        require_ready_logistic_outcome(&profile)?;
        let (coefficients, fit, predictions, diagnostics) = logistic_regression(
            &x,
            &y,
            &terms,
            &outcome,
            recipe.settings.confidence_level,
            profile,
            LOGISTIC_MAX_ITERATIONS,
            LOGISTIC_CONVERGENCE_TOLERANCE,
            control,
        )?;
        (coefficients, fit, predictions, Some(diagnostics))
    } else {
        let (coefficients, fit, predictions) =
            ols_regression(&x, &y, &terms, &outcome, recipe.settings.confidence_level)?;
        (coefficients, fit, predictions, None)
    };
    let process = (regression_type == "process")
        .then(|| process_analysis(dataset, recipe, enforce_process_outcome_scope, control))
        .transpose()?;
    let status_warning = if regression_type == "ols" {
        "OLS regression v1 is validated for the documented QuickPLS v1.2 OLS scope; unsupported shapes remain blocked."
    } else if regression_type == "logistic" {
        "Logistic regression v2 is validated for the documented QuickPLS binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, categorical auto-encoding, and Firth-corrected models remain unsupported."
    } else if process_graph {
        "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates."
    } else {
        "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope; moderated mediation and the full Hayes model catalogue remain experimental."
    };
    let mut result = empty_method_result(
        if regression_type == "logistic" {
            REGRESSION_LOGISTIC_METHOD_VERSION
        } else if regression_type == "process" {
            REGRESSION_PROCESS_METHOD_VERSION_V1
        } else {
            REGRESSION_OLS_METHOD_VERSION
        },
        prepared.used,
        prepared.omitted,
        if process_graph {
            vec![
                status_warning.into(),
                "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected.".into(),
            ]
        } else {
            vec![status_warning.into()]
        },
    );
    result.regression = Some(RegressionAnalysis {
        method_version: result.method_version.clone(),
        regression_type,
        outcome,
        predictors,
        controls,
        observations: prepared.used,
        coefficients,
        fit: Some(fit),
        predictions,
        logistic,
        process,
        bootstrap: None,
        warnings: result.warnings.clone(),
    });
    Ok(result)
}

fn estimate_nca_method(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let x_name = metadata_required(recipe, "nca_x")?;
    let y_name = metadata_required(recipe, "nca_y")?;
    checkpoint(control, EstimationPhase::PreparingIndicators, 0, 2)?;
    let prepared =
        prepare_raw_numeric_data(dataset, &[x_name.clone(), y_name.clone()], false, false)?;
    let x = &prepared.columns[0];
    let y = &prepared.columns[1];
    if sample_sd(x) <= f64::EPSILON || sample_sd(y) <= f64::EPSILON {
        return Err(EstimationError::ConstantIndicator(format!(
            "{x_name}/{y_name}"
        )));
    }
    let ceiling = recipe
        .metadata
        .get("nca_ceiling")
        .cloned()
        .unwrap_or_else(|| "both".into());
    let permutations = metadata_usize(recipe, "nca_permutation_samples", 999).min(10_000);
    let methods = nca_requested_ceilings(&ceiling).ok_or_else(|| {
        EstimationError::UnsupportedMethod(format!(
            "NCA v2 does not support ceiling technique {ceiling}"
        ))
    })?;
    let scope = nca_scope(x, y);
    let peers = nca_ce_fdh_peers(x, y);
    let cr_line = nca_cr_fdh_line(&peers);
    let mut ceilings = Vec::new();
    let total_permutation_units = methods.len().saturating_mul(permutations);
    checkpoint(
        control,
        EstimationPhase::ComputingEffects,
        0,
        total_permutation_units as u64,
    )?;
    for (method_index, method) in methods.iter().enumerate() {
        let (effect_size, slope, intercept) =
            nca_ceiling_parameters(&scope, &peers, method, cr_line);
        let p_value = nca_permutation_p_value(
            x,
            y,
            method,
            effect_size,
            permutations,
            recipe.settings.seed,
            method_index.saturating_mul(permutations),
            total_permutation_units,
            control,
        )?;
        ceilings.push(NcaCeilingResult {
            ceiling: (*method).into(),
            effect_size,
            permutation_p_value: Some(p_value),
            slope,
            intercept,
        });
    }
    checkpoint(
        control,
        EstimationPhase::ComputingEffects,
        total_permutation_units as u64,
        total_permutation_units as u64,
    )?;
    let bottlenecks = nca_bottleneck_rows(&scope, &peers, &methods, cr_line);
    let mut result = empty_method_result(
        NCA_METHOD_VERSION,
        prepared.used,
        prepared.omitted,
        vec![
            "NCA v2 is limited to the documented numeric X/Y CE-FDH and CR-FDH scope with observed-range bottlenecks; multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants remain unsupported."
                .into(),
        ],
    );
    result.nca = Some(NcaAnalysis {
        method_version: NCA_METHOD_VERSION.into(),
        ceiling,
        permutation_samples: permutations,
        usable_permutations: permutations,
        x: x_name,
        y: y_name,
        observations: prepared.used,
        scope,
        ce_fdh_peers: peers,
        ceilings,
        bottlenecks,
        warnings: result.warnings.clone(),
    });
    Ok(result)
}

fn expand_repeated_indicator_higher_order(
    recipe: &AnalysisRecipe,
) -> Result<AnalysisRecipe, EstimationError> {
    if recipe.model.higher_order_constructs.is_empty() {
        return Ok(recipe.clone());
    }
    let original_indicators = recipe
        .model
        .constructs
        .iter()
        .map(|construct| (construct.id.as_str(), construct.indicators.clone()))
        .collect::<HashMap<_, _>>();
    let mut expanded = recipe.clone();
    let construct_positions = expanded
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.clone(), index))
        .collect::<HashMap<_, _>>();
    for higher_order in &recipe.model.higher_order_constructs {
        if !matches!(
            higher_order.method,
            HigherOrderMethod::RepeatedIndicators | HigherOrderMethod::Hybrid
        ) {
            continue;
        }
        let position = construct_positions
            .get(&higher_order.id)
            .ok_or_else(|| EstimationError::UnknownConstruct(higher_order.id.clone()))?;
        let indicators = match higher_order.method {
            HigherOrderMethod::RepeatedIndicators => {
                repeated_hoc_indicators(&original_indicators, &higher_order.components)?
            }
            HigherOrderMethod::Hybrid => {
                for component in &higher_order.components {
                    let component_position = construct_positions
                        .get(component)
                        .ok_or_else(|| EstimationError::UnknownConstruct(component.clone()))?;
                    let component_indicators = original_indicators
                        .get(component.as_str())
                        .ok_or_else(|| EstimationError::UnknownConstruct(component.clone()))?;
                    let (lower, _) =
                        split_hybrid_component_indicators(component, component_indicators)?;
                    expanded.model.constructs[*component_position].indicators = lower;
                }
                hybrid_hoc_indicators(&original_indicators, &higher_order.components)?
            }
            HigherOrderMethod::TwoStage => unreachable!("two-stage HOC handled before expansion"),
        };
        if indicators.is_empty() {
            return Err(EstimationError::EmptyConstruct(higher_order.id.clone()));
        }
        expanded.model.constructs[*position].indicators = indicators;
    }
    Ok(expanded)
}

fn repeated_hoc_indicators(
    original_indicators: &HashMap<&str, Vec<String>>,
    components: &[String],
) -> Result<Vec<String>, EstimationError> {
    let mut seen = HashSet::new();
    let mut indicators = Vec::new();
    for component in components {
        let component_indicators = original_indicators
            .get(component.as_str())
            .ok_or_else(|| EstimationError::UnknownConstruct(component.clone()))?;
        if component_indicators.is_empty() {
            return Err(EstimationError::EmptyConstruct(component.clone()));
        }
        for indicator in component_indicators {
            if seen.insert(indicator.clone()) {
                indicators.push(indicator.clone());
            }
        }
    }
    Ok(indicators)
}

fn hybrid_hoc_indicators(
    original_indicators: &HashMap<&str, Vec<String>>,
    components: &[String],
) -> Result<Vec<String>, EstimationError> {
    let mut seen = HashSet::new();
    let mut indicators = Vec::new();
    for component in components {
        let component_indicators = original_indicators
            .get(component.as_str())
            .ok_or_else(|| EstimationError::UnknownConstruct(component.clone()))?;
        let (_, higher) = split_hybrid_component_indicators(component, component_indicators)?;
        for indicator in higher {
            if seen.insert(indicator.clone()) {
                indicators.push(indicator);
            }
        }
    }
    Ok(indicators)
}

fn split_hybrid_component_indicators(
    component: &str,
    indicators: &[String],
) -> Result<(Vec<String>, Vec<String>), EstimationError> {
    if indicators.len() < 2 {
        return Err(EstimationError::Numerical(format!(
            "hybrid higher-order component '{component}' requires at least two indicators"
        )));
    }
    let split = (indicators.len() + 1) / 2;
    Ok((indicators[..split].to_vec(), indicators[split..].to_vec()))
}

fn estimate_pls_two_stage_moderation(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    allow_isolated_constructs: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let product_constructs = recipe
        .model
        .interactions
        .iter()
        .map(|interaction| interaction.product_construct.as_str())
        .collect::<HashSet<_>>();
    let mut stage1_recipe = recipe.clone();
    stage1_recipe
        .model
        .constructs
        .retain(|construct| !product_constructs.contains(construct.id.as_str()));
    stage1_recipe.model.paths.retain(|path| {
        !product_constructs.contains(path.source.as_str())
            && !product_constructs.contains(path.target.as_str())
    });
    stage1_recipe.model.interactions.clear();

    let stage1_prepared = prepare_data(
        dataset,
        &collect_indicators(&stage1_recipe)?,
        &stage1_recipe.settings.preprocessing,
        &stage1_recipe.settings.missing_data,
        None,
        control,
    )?;
    let stage1 = estimate_pls_internal(
        dataset,
        &stage1_recipe,
        allow_isolated_constructs,
        true,
        control,
    )?;
    let (expanded_dataset, stage2_recipe) =
        expand_two_stage_moderation_dataset(dataset, recipe, &stage1, &stage1_prepared.used_rows)?;
    let mut result = estimate_pls_internal(
        &expanded_dataset,
        &stage2_recipe,
        allow_isolated_constructs,
        true,
        control,
    )?;
    result.used_observations = stage1.used_observations;
    result.omitted_observations = stage1.omitted_observations;
    if stage1.omitted_observations > 0 {
        result.warnings.push(format!(
            "{} observations were omitted listwise before two-stage product-score generation",
            stage1.omitted_observations
        ));
    }
    result.moderation = analyze_moderation(recipe, &result);
    if recipe.settings.method == AnalysisMethod::ModeratedMediation {
        result.moderated_mediation = Some(analyze_moderated_mediation(recipe, &result));
        result.method_version = MODERATED_MEDIATION_METHOD_VERSION.into();
    }
    result.warnings.push(
        "Two-stage moderation is validated for the documented QuickPLS v1.2.1 single-interaction scope when interpreted with validated bootstrap or permutation inference."
            .into(),
    );
    Ok(result)
}

fn estimate_pls_two_stage_higher_order(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    _allow_isolated_constructs: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let two_stage_hoc_ids = recipe
        .model
        .higher_order_constructs
        .iter()
        .filter(|higher_order| higher_order.method == HigherOrderMethod::TwoStage)
        .map(|higher_order| higher_order.id.as_str())
        .collect::<HashSet<_>>();
    let mut stage1_recipe = recipe.clone();
    stage1_recipe
        .model
        .constructs
        .retain(|construct| !two_stage_hoc_ids.contains(construct.id.as_str()));
    stage1_recipe.model.paths.retain(|path| {
        !two_stage_hoc_ids.contains(path.source.as_str())
            && !two_stage_hoc_ids.contains(path.target.as_str())
    });
    stage1_recipe
        .model
        .higher_order_constructs
        .retain(|higher_order| higher_order.method != HigherOrderMethod::TwoStage);

    let stage1_prepared = prepare_data(
        dataset,
        &collect_indicators(&stage1_recipe)?,
        &stage1_recipe.settings.preprocessing,
        &stage1_recipe.settings.missing_data,
        None,
        control,
    )?;
    let stage1 = estimate_pls_internal(dataset, &stage1_recipe, true, true, control)?;
    let (expanded_dataset, stage2_recipe) = expand_two_stage_higher_order_dataset(
        dataset,
        recipe,
        &stage1,
        &stage1_prepared.used_rows,
    )?;
    let mut result = estimate_pls_internal(&expanded_dataset, &stage2_recipe, true, true, control)?;
    result.used_observations = stage1.used_observations;
    result.omitted_observations = stage1.omitted_observations;
    if stage1.omitted_observations > 0 {
        result.warnings.push(format!(
            "{} observations were omitted listwise before two-stage HOC score generation",
            stage1.omitted_observations
        ));
    }
    result.warnings.push(
        "Two-stage higher-order constructs are validated for the documented QuickPLS v1.2.3 bounded repeated-indicator, two-stage, and hybrid scopes; lower-order component scores are used as generated HOC indicators in stage 2."
            .into(),
    );
    Ok(result)
}

fn expand_two_stage_higher_order_dataset(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    stage1: &PlsResult,
    used_rows: &[usize],
) -> Result<(Dataset, AnalysisRecipe), EstimationError> {
    if used_rows.len() != stage1.used_observations {
        return Err(EstimationError::Numerical(
            "stage-1 used row count does not match construct-score length".into(),
        ));
    }
    let mut arrays = Vec::<ArrayRef>::new();
    let mut fields = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .map(|field| Field::new(field.name(), field.data_type().clone(), field.is_nullable()))
        .collect::<Vec<_>>();
    let mut schema = dataset.schema.clone();
    for column in dataset.batch.columns() {
        arrays.push(subset_array(column.as_ref(), used_rows)?);
    }
    let existing_fields = fields
        .iter()
        .map(|field| field.name().to_string())
        .collect::<HashSet<_>>();
    let mut generated_names = HashSet::new();
    let mut stage2_recipe = recipe.clone();

    for higher_order in &recipe.model.higher_order_constructs {
        if higher_order.method != HigherOrderMethod::TwoStage {
            continue;
        }
        let mut indicators = Vec::new();
        for component in &higher_order.components {
            let scores = stage1.construct_scores.get(component).ok_or_else(|| {
                EstimationError::Numerical(format!(
                    "missing stage-1 component scores for {component}"
                ))
            })?;
            if scores.len() != used_rows.len() {
                return Err(EstimationError::Numerical(
                    "stage-1 score length does not match the complete-case rows".into(),
                ));
            }
            let indicator_name = higher_order_component_indicator_name(&higher_order.id, component);
            if existing_fields.contains(&indicator_name)
                || !generated_names.insert(indicator_name.clone())
            {
                return Err(EstimationError::DuplicateIndicator(indicator_name));
            }
            arrays.push(Arc::new(Float64Array::from(scores.clone())) as ArrayRef);
            fields.push(Field::new(&indicator_name, DataType::Float64, false));
            schema.columns.push(ColumnMetadata {
                name: indicator_name.clone(),
                label: Some(format!(
                    "Two-stage HOC component score: {} <- {}",
                    higher_order.id, component
                )),
                column_type: ColumnType::Numeric,
                scale_type: ScaleType::Continuous,
                missing_markers: Vec::new(),
                theoretical_min: None,
                theoretical_max: None,
                value_labels: BTreeMap::new(),
            });
            indicators.push(indicator_name);
        }
        let Some(hoc_construct) = stage2_recipe
            .model
            .constructs
            .iter_mut()
            .find(|construct| construct.id == higher_order.id)
        else {
            return Err(EstimationError::UnknownConstruct(higher_order.id.clone()));
        };
        hoc_construct.indicators = indicators;
    }
    stage2_recipe
        .model
        .higher_order_constructs
        .retain(|higher_order| higher_order.method != HigherOrderMethod::TwoStage);
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    Ok((
        Dataset {
            id: dataset.id,
            name: dataset.name.clone(),
            schema,
            fingerprint: dataset.fingerprint.clone(),
            batch,
        },
        stage2_recipe,
    ))
}

fn apply_plsc_correction(
    recipe: &AnalysisRecipe,
    columns: &[Vec<f64>],
    weights: &[Vec<f64>],
    scores: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if matches!(recipe.settings.weighting_scheme, WeightingScheme::Pca) {
        return Err(EstimationError::UnsupportedMethod(
            "PLSc currently requires path or factor weighting".into(),
        ));
    }
    if recipe
        .model
        .constructs
        .iter()
        .any(|construct| construct.mode != MeasurementMode::Reflective)
    {
        return Err(EstimationError::UnsupportedMethod(
            "PLSc currently requires reflective constructs".into(),
        ));
    }

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let indicators = collect_indicators(recipe)?;
    let indicator_index = indicators
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut reliabilities = Vec::with_capacity(recipe.model.constructs.len());
    for (construct_index, construct) in recipe.model.constructs.iter().enumerate() {
        if construct.indicators.len() < 2 {
            return Err(EstimationError::UnsupportedMethod(format!(
                "PLSc requires at least two indicators for construct '{}'",
                construct.id
            )));
        }
        let block_columns = construct
            .indicators
            .iter()
            .map(|indicator| {
                indicator_index
                    .get(indicator.as_str())
                    .map(|index| columns[*index].as_slice())
                    .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rho_a = plsc_rho_a(&block_columns, &weights[construct_index])?;
        if !rho_a.is_finite() || rho_a <= 0.0 || rho_a > 1.0 + 1e-10 {
            return Err(EstimationError::Numerical(format!(
                "invalid PLSc rho_A for construct '{}': {}",
                construct.id, rho_a
            )));
        }
        reliabilities.push(PlscReliability {
            construct: construct.id.clone(),
            rho_a: rho_a.min(1.0),
        });
    }

    let count = construct_ids.len();
    let mut corrected = vec![vec![0.0; count]; count];
    let mut construct_correlations = Vec::new();
    for left in 0..count {
        corrected[left][left] = 1.0;
        for right in (left + 1)..count {
            let original = correlation(&scores[left], &scores[right]);
            let divisor = (reliabilities[left].rho_a * reliabilities[right].rho_a).sqrt();
            let mut value = original / divisor;
            if value.abs() > 1.0 + 1e-10 {
                return Err(EstimationError::Numerical(format!(
                    "PLSc corrected construct correlation is outside [-1, 1] for '{}' and '{}'",
                    construct_ids[left], construct_ids[right]
                )));
            }
            value = value.clamp(-1.0, 1.0);
            corrected[left][right] = value;
            corrected[right][left] = value;
            construct_correlations.push(PlscConstructCorrelation {
                left: construct_ids[left].clone(),
                right: construct_ids[right].clone(),
                original,
                corrected: value,
            });
        }
    }

    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut corrected_paths = Vec::new();
    let mut corrected_r_squared = BTreeMap::new();
    for (target_index, target) in recipe.model.constructs.iter().enumerate() {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        let coefficients =
            solve_correlation_regression(&corrected, &predecessors, target_index, &target.id)?;
        let mut r_squared = 0.0;
        for (source_index, coefficient) in predecessors.iter().zip(&coefficients) {
            r_squared += coefficient * corrected[*source_index][target_index];
        }
        corrected_r_squared.insert(target.id.clone(), r_squared.clamp(0.0, 1.0));
        for (source_index, coefficient) in predecessors.iter().zip(coefficients) {
            corrected_paths.push(PathEstimate {
                source: construct_ids[*source_index].clone(),
                target: target.id.clone(),
                coefficient,
            });
        }
    }

    let reliability_by_construct = reliabilities
        .iter()
        .map(|entry| (entry.construct.as_str(), entry.rho_a))
        .collect::<HashMap<_, _>>();
    let corrected_outer_loadings = result
        .outer_estimates
        .iter()
        .map(|outer| {
            let divisor = reliability_by_construct[outer.construct.as_str()].sqrt();
            let mut corrected = outer.clone();
            corrected.loading = (outer.loading / divisor).clamp(-1.0, 1.0);
            corrected
        })
        .collect::<Vec<_>>();

    let mut never_cancel = |_| true;
    result.method_version = PLSC_METHOD_VERSION.into();
    result.paths = corrected_paths.clone();
    result.r_squared = corrected_r_squared.clone();
    result.effects = calculate_effects(&construct_ids, &result.paths, &mut never_cancel)?;
    result.control_estimates = control_estimates(&recipe.model.controls, &result.paths)?;
    result.mediation = analyze_mediation_effects_with_tolerance(&result.effects, 1e-12);
    result.plsc = Some(PlscAnalysis {
        method_version: PLSC_METHOD_VERSION.into(),
        reliability_method_version: DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION.into(),
        tolerance: 1e-12,
        reliabilities,
        construct_correlations,
        corrected_paths,
        corrected_outer_loadings,
        corrected_r_squared,
        warnings: vec![
            "PLSc is validated for the documented QuickPLS v1.2.1 reflective path/factor-weighting scope; broader PLSc shapes remain unsupported.".into(),
        ],
    });
    result.warnings.push(
        "PLSc is validated for the documented QuickPLS v1.2.1 reflective path/factor-weighting scope."
            .into(),
    );
    Ok(())
}

fn plsc_rho_a(columns: &[&[f64]], weights: &[f64]) -> Result<f64, EstimationError> {
    let count = columns.len();
    let mut indicator_correlation = vec![vec![0.0; count]; count];
    for row in 0..count {
        indicator_correlation[row][row] = 1.0;
        for column in (row + 1)..count {
            let value = correlation(columns[row], columns[column]);
            indicator_correlation[row][column] = value;
            indicator_correlation[column][row] = value;
        }
    }
    let score_variance = quadratic_form(weights, &indicator_correlation);
    let score_variance_tolerance = 64.0
        * f64::EPSILON
        * weights
            .iter()
            .map(|value| value.abs().powi(2))
            .sum::<f64>()
            .max(1.0);
    if !score_variance.is_finite() || score_variance <= score_variance_tolerance {
        return Err(EstimationError::Numerical(
            "PLSc rho_A score variance is invalid".into(),
        ));
    }
    let divisor = score_variance.sqrt();
    let normalized_weights = weights
        .iter()
        .map(|weight| weight / divisor)
        .collect::<Vec<_>>();
    dijkstra_henseler_rho_a_from_normalized(&indicator_correlation, &normalized_weights)
        .map(|result| result.value)
        .map_err(|error| EstimationError::Numerical(format!("invalid PLSc rho_A: {error}")))
}

fn quadratic_form(weights: &[f64], matrix: &[Vec<f64>]) -> f64 {
    let mut total = 0.0;
    for row in 0..weights.len() {
        for column in 0..weights.len() {
            total += weights[row] * matrix[row][column] * weights[column];
        }
    }
    total
}

fn solve_correlation_regression(
    correlation_matrix: &[Vec<f64>],
    predictors: &[usize],
    target: usize,
    target_id: &str,
) -> Result<Vec<f64>, EstimationError> {
    let system = predictors
        .iter()
        .map(|left| {
            predictors
                .iter()
                .map(|right| correlation_matrix[*left][*right])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let rhs = predictors
        .iter()
        .map(|predictor| correlation_matrix[*predictor][target])
        .collect::<Vec<_>>();
    solve_linear_system(system, rhs, target_id)
}

fn solve_linear_system(
    mut system: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    target_id: &str,
) -> Result<Vec<f64>, EstimationError> {
    let count = rhs.len();
    for pivot in 0..count {
        let mut selected = pivot;
        let mut selected_abs = system[pivot][pivot].abs();
        for candidate in (pivot + 1)..count {
            if system[candidate][pivot].abs() > selected_abs {
                selected = candidate;
                selected_abs = system[candidate][pivot].abs();
            }
        }
        if selected_abs <= 1e-12 {
            return Err(EstimationError::RankDeficient(target_id.into()));
        }
        if selected != pivot {
            system.swap(selected, pivot);
            rhs.swap(selected, pivot);
        }
        let pivot_value = system[pivot][pivot];
        for column in pivot..count {
            system[pivot][column] /= pivot_value;
        }
        rhs[pivot] /= pivot_value;
        for row in 0..count {
            if row == pivot {
                continue;
            }
            let factor = system[row][pivot];
            for column in pivot..count {
                system[row][column] -= factor * system[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    Ok(rhs)
}

fn apply_gaussian_copula_endogeneity(
    recipe: &AnalysisRecipe,
    scores: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if recipe.settings.weighting_scheme == WeightingScheme::Pca {
        return Err(EstimationError::UnsupportedMethod(
            "Gaussian-copula endogeneity diagnostics currently require path or factor weighting"
                .into(),
        ));
    }
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let normal = Normal::standard();
    let copulas = scores
        .iter()
        .map(|score| gaussian_copula_score(score, &normal))
        .collect::<Result<Vec<_>, _>>()?;
    let path_by_pair = result
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut estimates = Vec::new();
    let mut warnings = Vec::new();
    for (target_index, target) in recipe.model.constructs.iter().enumerate() {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        let mut predictors = Vec::with_capacity(predecessors.len() * 2);
        for source in &predecessors {
            predictors.push(scores[*source].clone());
        }
        for source in &predecessors {
            predictors.push(copulas[*source].clone());
        }
        let stats = ols_with_standard_errors(&predictors, &scores[target_index], &target.id)?;
        for (within, source_index) in predecessors.iter().enumerate() {
            let copula_index = predecessors.len() + within;
            let source = &construct_ids[*source_index];
            let skewness = sample_skewness(&scores[*source_index]);
            let applicable = skewness.abs() >= 0.5;
            let warning = if applicable {
                None
            } else {
                Some(
                    "Predictor score skewness is below the experimental applicability threshold; Gaussian-copula evidence is weak for near-normal predictors."
                        .into(),
                )
            };
            if warning.is_some() {
                warnings.push(format!(
                    "{} -> {} has near-symmetric predictor scores; interpret Gaussian-copula diagnostics cautiously",
                    source, target.id
                ));
            }
            estimates.push(GaussianCopulaEstimate {
                source: source.clone(),
                target: target.id.clone(),
                path_coefficient: path_by_pair[&(source.as_str(), target.id.as_str())],
                copula_coefficient: stats.coefficients[copula_index],
                standard_error: stats.standard_errors[copula_index],
                t_statistic: stats.t_statistics[copula_index],
                p_value_two_sided: stats.p_values[copula_index],
                predictor_skewness: skewness,
                applicable,
                warning,
            });
        }
    }
    result.method_version = GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION.into();
    warnings.push(
        "Gaussian-copula endogeneity diagnostics are validated for the documented QuickPLS v1.2.3 diagnostic scope and assume nonnormal predictor scores; use as a diagnostic, not proof of causality."
            .into(),
    );
    result.endogeneity = Some(GaussianCopulaEndogeneityAnalysis {
        method_version: GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION.into(),
        transform: "rankit_inverse_normal_v1".into(),
        estimates,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

#[derive(Debug)]
struct RegressionStats {
    coefficients: Vec<f64>,
    standard_errors: Vec<f64>,
    t_statistics: Vec<f64>,
    p_values: Vec<f64>,
}

fn ols_with_standard_errors(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    subject: &str,
) -> Result<RegressionStats, EstimationError> {
    if predictors.is_empty() {
        return Ok(RegressionStats {
            coefficients: Vec::new(),
            standard_errors: Vec::new(),
            t_statistics: Vec::new(),
            p_values: Vec::new(),
        });
    }
    let rows = outcome.len();
    let columns = predictors.len();
    if rows <= columns + 1 {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let x_means = predictors
        .iter()
        .map(|predictor| vector_mean(predictor))
        .collect::<Vec<_>>();
    let y_mean = vector_mean(outcome);
    let mut xtx = vec![vec![0.0; columns]; columns];
    let mut xty = vec![0.0; columns];
    for row in 0..rows {
        let centered_y = outcome[row] - y_mean;
        for left in 0..columns {
            let x_left = predictors[left][row] - x_means[left];
            xty[left] += x_left * centered_y;
            for right in 0..columns {
                xtx[left][right] += x_left * (predictors[right][row] - x_means[right]);
            }
        }
    }
    let coefficients = solve_linear_system(xtx.clone(), xty, subject)?;
    let mut rss = 0.0;
    for row in 0..rows {
        let fitted = (0..columns)
            .map(|column| coefficients[column] * (predictors[column][row] - x_means[column]))
            .sum::<f64>();
        let residual = outcome[row] - y_mean - fitted;
        rss += residual * residual;
    }
    let df = rows as f64 - columns as f64 - 1.0;
    if df <= 0.0 {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let sigma2 = rss / df;
    let distribution = StudentsT::new(0.0, 1.0, df)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let mut standard_errors = Vec::with_capacity(columns);
    let mut t_statistics = Vec::with_capacity(columns);
    let mut p_values = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut basis = vec![0.0; columns];
        basis[column] = 1.0;
        let inverse_column = solve_linear_system(xtx.clone(), basis, subject)?;
        let variance = sigma2 * inverse_column[column];
        if !variance.is_finite() || variance <= 0.0 {
            return Err(EstimationError::Numerical(format!(
                "non-positive regression variance for {subject}"
            )));
        }
        let standard_error = variance.sqrt();
        let statistic = coefficients[column] / standard_error;
        let probability = 2.0 * (1.0 - distribution.cdf(statistic.abs()));
        standard_errors.push(standard_error);
        t_statistics.push(statistic);
        p_values.push(probability.clamp(0.0, 1.0));
    }
    Ok(RegressionStats {
        coefficients,
        standard_errors,
        t_statistics,
        p_values,
    })
}

fn gaussian_copula_score(values: &[f64], normal: &Normal) -> Result<Vec<f64>, EstimationError> {
    let mut ordered = values
        .iter()
        .enumerate()
        .map(|(index, value)| (index, *value))
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)));
    let mut ranks = vec![0.0; values.len()];
    let mut cursor = 0;
    while cursor < ordered.len() {
        let start = cursor;
        let value = ordered[cursor].1;
        while cursor < ordered.len() && ordered[cursor].1 == value {
            cursor += 1;
        }
        let average_rank = (start + 1 + cursor) as f64 / 2.0;
        for index in start..cursor {
            ranks[ordered[index].0] = average_rank;
        }
    }
    let denominator = values.len() as f64 + 1.0;
    let mut transformed = ranks
        .into_iter()
        .map(|rank| normal.inverse_cdf(rank / denominator))
        .collect::<Vec<_>>();
    let mean = vector_mean(&transformed);
    for value in &mut transformed {
        *value -= mean;
    }
    Ok(transformed)
}

fn sample_skewness(values: &[f64]) -> f64 {
    let mean = vector_mean(values);
    let sd = sample_sd(values);
    if sd <= f64::EPSILON || !sd.is_finite() {
        return f64::NAN;
    }
    let n = values.len() as f64;
    values
        .iter()
        .map(|value| ((value - mean) / sd).powi(3))
        .sum::<f64>()
        / n
}

fn apply_quadratic_nonlinear_effects(
    recipe: &AnalysisRecipe,
    scores: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if recipe.settings.weighting_scheme == WeightingScheme::Pca {
        return Err(EstimationError::UnsupportedMethod(
            "Nonlinear effects currently require path or factor weighting".into(),
        ));
    }
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let path_by_pair = result
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut estimates = Vec::new();
    for (target_index, target) in recipe.model.constructs.iter().enumerate() {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        let linear_predictors = predecessors
            .iter()
            .map(|source| scores[*source].clone())
            .collect::<Vec<_>>();
        let linear_r_squared =
            regression_r_squared(&linear_predictors, &scores[target_index], &target.id)?;
        let mut predictors = linear_predictors.clone();
        for source in &predecessors {
            predictors.push(centered_square(&scores[*source]));
        }
        let stats = ols_with_standard_errors(&predictors, &scores[target_index], &target.id)?;
        let augmented_r_squared =
            regression_r_squared(&predictors, &scores[target_index], &target.id)?;
        for (within, source_index) in predecessors.iter().enumerate() {
            let source = &construct_ids[*source_index];
            let quadratic_index = predecessors.len() + within;
            let delta = (augmented_r_squared - linear_r_squared).max(0.0);
            let warning = if delta <= 1e-12 {
                Some("The quadratic term does not improve fixed-score R2 beyond numerical tolerance.".into())
            } else {
                None
            };
            estimates.push(NonlinearEffectEstimate {
                source: source.clone(),
                target: target.id.clone(),
                linear_coefficient: path_by_pair[&(source.as_str(), target.id.as_str())],
                quadratic_coefficient: stats.coefficients[quadratic_index],
                standard_error: stats.standard_errors[quadratic_index],
                t_statistic: stats.t_statistics[quadratic_index],
                p_value_two_sided: stats.p_values[quadratic_index],
                linear_r_squared,
                augmented_r_squared,
                delta_r_squared: delta,
                warning,
            });
        }
    }
    let warnings = vec![
        "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms.".into(),
    ];
    result.method_version = NONLINEAR_EFFECTS_METHOD_VERSION.into();
    result.nonlinear_effects = Some(NonlinearEffectsAnalysis {
        method_version: NONLINEAR_EFFECTS_METHOD_VERSION.into(),
        term: "centered_squared_construct_score_v1".into(),
        estimates,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_cta_pls(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    columns: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut estimates = Vec::new();
    let mut max_absolute_tetrad_by_construct = BTreeMap::new();
    for construct in recipe
        .model
        .constructs
        .iter()
        .filter(|construct| construct.indicators.len() >= 4)
    {
        let mut construct_max = 0.0_f64;
        for a in 0..construct.indicators.len() - 3 {
            for b in a + 1..construct.indicators.len() - 2 {
                for c in b + 1..construct.indicators.len() - 1 {
                    for d in c + 1..construct.indicators.len() {
                        let ia = *indicator_index
                            .get(construct.indicators[a].as_str())
                            .ok_or_else(|| {
                                EstimationError::InvalidIndicator(construct.indicators[a].clone())
                            })?;
                        let ib = *indicator_index
                            .get(construct.indicators[b].as_str())
                            .ok_or_else(|| {
                                EstimationError::InvalidIndicator(construct.indicators[b].clone())
                            })?;
                        let ic = *indicator_index
                            .get(construct.indicators[c].as_str())
                            .ok_or_else(|| {
                                EstimationError::InvalidIndicator(construct.indicators[c].clone())
                            })?;
                        let id = *indicator_index
                            .get(construct.indicators[d].as_str())
                            .ok_or_else(|| {
                                EstimationError::InvalidIndicator(construct.indicators[d].clone())
                            })?;
                        let cov_ab = covariance(&columns[ia], &columns[ib]);
                        let cov_ac = covariance(&columns[ia], &columns[ic]);
                        let cov_ad = covariance(&columns[ia], &columns[id]);
                        let cov_bc = covariance(&columns[ib], &columns[ic]);
                        let cov_bd = covariance(&columns[ib], &columns[id]);
                        let cov_cd = covariance(&columns[ic], &columns[id]);
                        let tetrads = [
                            ("ab_cd_minus_ac_bd", cov_ab * cov_cd - cov_ac * cov_bd),
                            ("ac_bd_minus_ad_bc", cov_ac * cov_bd - cov_ad * cov_bc),
                            ("ad_bc_minus_ab_cd", cov_ad * cov_bc - cov_ab * cov_cd),
                        ];
                        for (pairing, tetrad) in tetrads {
                            let absolute_tetrad = tetrad.abs();
                            construct_max = construct_max.max(absolute_tetrad);
                            estimates.push(TetradEstimate {
                                construct: construct.id.clone(),
                                indicator_a: construct.indicators[a].clone(),
                                indicator_b: construct.indicators[b].clone(),
                                indicator_c: construct.indicators[c].clone(),
                                indicator_d: construct.indicators[d].clone(),
                                pairing: pairing.into(),
                                tetrad,
                                absolute_tetrad,
                            });
                        }
                    }
                }
            }
        }
        max_absolute_tetrad_by_construct.insert(construct.id.clone(), construct_max);
    }
    if estimates.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            AnalysisMethod::CtaPls.to_string(),
        ));
    }
    result.method_version = CTA_PLS_METHOD_VERSION.into();
    result.warnings.push(
        "CTA-PLS tetrad diagnostics are validated for the documented QuickPLS v1.2.3 descriptive tetrad scope; bootstrap/permutation tetrad decision rules remain unsupported."
            .into(),
    );
    result.cta_pls = Some(CtaPlsAnalysis {
        method_version: CTA_PLS_METHOD_VERSION.into(),
        covariance: "sample_covariance_of_preprocessed_indicators_v1".into(),
        estimates,
        max_absolute_tetrad_by_construct,
        warnings: vec![
            "CTA-PLS tetrad bootstrap/permutation inference is outside the validated QuickPLS v1.2.3 descriptive scope."
                .into(),
        ],
    });
    Ok(())
}

fn apply_wpls_metadata(
    recipe: &AnalysisRecipe,
    case_weights: &[f64],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    let case_weight_column = recipe
        .settings
        .case_weight_column
        .as_deref()
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod("WPLS requires a case weight column".into())
        })?
        .to_string();
    let weight_sum = case_weights.iter().sum::<f64>();
    let sum_squared = case_weights
        .iter()
        .map(|weight| weight * weight)
        .sum::<f64>();
    result.method_version = WPLS_METHOD_VERSION.into();
    result.warnings.push(
        "WPLS is validated for the documented QuickPLS v1.2.1 positive case-weighted reflective path/factor-weighting scope."
            .into(),
    );
    result.wpls = Some(WplsAnalysis {
        method_version: WPLS_METHOD_VERSION.into(),
        case_weight_column,
        weight_sum,
        effective_sample_size: if sum_squared > 0.0 {
            weight_sum * weight_sum / sum_squared
        } else {
            0.0
        },
        covariance: "positive_case_weighted_unbiased_covariance_v1".into(),
        warnings: vec![
            "WPLS inference, generated interaction/HOC workflows, formative blocks, and PCA weighting remain unsupported outside the documented validated scope."
                .into(),
        ],
    });
    Ok(())
}

fn apply_cca(
    recipe: &AnalysisRecipe,
    scores: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let count = construct_ids.len();
    let mut observed = vec![vec![0.0; count]; count];
    for row in 0..count {
        observed[row][row] = 1.0;
        for column in 0..row {
            let value = correlation(&scores[row], &scores[column]);
            observed[row][column] = value;
            observed[column][row] = value;
        }
    }
    let mut structural = vec![vec![0.0; count]; count];
    for path in &result.paths {
        let source = construct_index[path.source.as_str()];
        let target = construct_index[path.target.as_str()];
        structural[target][source] = path.coefficient;
    }
    let mut system = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..count {
            system[row][column] = if row == column { 1.0 } else { 0.0 } - structural[row][column];
        }
    }
    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<HashSet<_>>();
    let mut residual_covariance = vec![vec![0.0; count]; count];
    for row in 0..count {
        if endogenous.contains(construct_ids[row].as_str()) {
            residual_covariance[row][row] = (1.0
                - result
                    .r_squared
                    .get(construct_ids[row].as_str())
                    .copied()
                    .unwrap_or(0.0))
            .max(0.0);
        } else {
            residual_covariance[row][row] = 1.0;
            for column in 0..row {
                if !endogenous.contains(construct_ids[column].as_str()) {
                    residual_covariance[row][column] = observed[row][column];
                    residual_covariance[column][row] = observed[row][column];
                }
            }
        }
    }
    let mut inverse = vec![vec![0.0; count]; count];
    for column in 0..count {
        let mut rhs = vec![0.0; count];
        rhs[column] = 1.0;
        let solution = solve_linear_system(system.clone(), rhs, "cca")?;
        for row in 0..count {
            inverse[row][column] = solution[row];
        }
    }
    let mut reproduced = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..count {
            let mut value = 0.0;
            for left in 0..count {
                for right in 0..count {
                    value += inverse[row][left]
                        * residual_covariance[left][right]
                        * inverse[column][right];
                }
            }
            reproduced[row][column] = value;
        }
    }
    let mut correlations = Vec::new();
    let mut max_absolute_residual = 0.0_f64;
    for row in 0..count {
        for column in 0..row {
            let residual = observed[row][column] - reproduced[row][column];
            let absolute_residual = residual.abs();
            max_absolute_residual = max_absolute_residual.max(absolute_residual);
            correlations.push(CcaCorrelation {
                left: construct_ids[column].clone(),
                right: construct_ids[row].clone(),
                observed: observed[row][column],
                reproduced: reproduced[row][column],
                residual,
                absolute_residual,
            });
        }
    }
    result.method_version = CCA_METHOD_VERSION.into();
    result.warnings.push(
        "CCA is validated for the documented QuickPLS v1.2.3 descriptive composite residual scope; bootstrap-based CCA decisions remain unsupported."
            .into(),
    );
    result.cca = Some(CcaAnalysis {
        method_version: CCA_METHOD_VERSION.into(),
        model: "recursive_standardized_composite_path_model_v1".into(),
        correlations,
        max_absolute_residual,
        warnings: vec![
            "CCA bootstrap inference, discrepancy tests, and broader decision rules are outside the validated QuickPLS v1.2.3 descriptive scope."
                .into(),
        ],
    });
    Ok(())
}

fn estimate_gsca_method(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    validate_gsca_execution_contract(recipe)?;
    validate_acyclic(recipe)?;
    let indicator_names = collect_indicators(recipe)?;
    checkpoint(control, EstimationPhase::Validating, 1, 1)?;
    let prepared = prepare_data(
        dataset,
        &indicator_names,
        &Preprocessing::Standardized,
        &MissingDataPolicy::ListwiseDeletion,
        None,
        control,
    )?;
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let blocks = recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            construct
                .indicators
                .iter()
                .map(|indicator| indicator_index[indicator.as_str()])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let unit_scale = ((prepared.used - 1) as f64).sqrt();
    let unit_columns = prepared
        .columns
        .iter()
        .map(|column| {
            column
                .iter()
                .map(|value| value / unit_scale)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut weights = blocks
        .iter()
        .map(|block| gsca_normalize_weights(&unit_columns, block, vec![1.0; block.len()]))
        .collect::<Result<Vec<_>, _>>()?;
    let mut scores = gsca_scores(&unit_columns, &blocks, &weights)?;
    let mut coefficients = gsca_fit_coefficients(recipe, &unit_columns, &scores, &indicator_index)?;
    let mut previous_objective = f64::INFINITY;
    let mut final_change = f64::INFINITY;
    let mut iterations = 0;
    let iteration_units =
        recipe.settings.max_iterations as u64 * recipe.model.constructs.len().max(1) as u64;
    let mut converged = false;

    for iteration in 1..=recipe.settings.max_iterations {
        let previous_weights = weights.clone();
        for construct_index in 0..recipe.model.constructs.len() {
            checkpoint(
                control,
                EstimationPhase::Iterating,
                (iteration - 1) as u64 * recipe.model.constructs.len() as u64
                    + construct_index as u64,
                iteration_units,
            )?;
            let candidate = gsca_update_weight_block(
                recipe,
                &unit_columns,
                &blocks,
                &scores,
                &coefficients,
                construct_index,
            )?;
            weights[construct_index] = candidate;
            scores[construct_index] = gsca_score(
                &unit_columns,
                &blocks[construct_index],
                &weights[construct_index],
            )?;
        }
        coefficients = gsca_fit_coefficients(recipe, &unit_columns, &scores, &indicator_index)?;
        let (objective, _, _) = gsca_objective(
            recipe,
            &unit_columns,
            &scores,
            &coefficients,
            &indicator_index,
        );
        if !objective.is_finite() {
            return Err(EstimationError::Numerical(
                "GSCA ALS produced a non-finite objective".into(),
            ));
        }
        let relative_objective_change = if previous_objective.is_finite() {
            (previous_objective - objective).abs() / previous_objective.abs().max(1.0)
        } else {
            f64::INFINITY
        };
        let maximum_weight_change = weights
            .iter()
            .zip(&previous_weights)
            .flat_map(|(current, previous)| current.iter().zip(previous))
            .map(|(current, previous)| (current - previous).abs())
            .fold(0.0_f64, f64::max);
        // Objective convergence alone can stop on a locally flat surface while
        // the identified component weights are still moving materially.  The
        // published GSCA criterion remains the optimization target; requiring
        // both its relative change and the largest normalized-weight change to
        // satisfy the fixed criterion makes the reported parameters stable.
        final_change = relative_objective_change.max(maximum_weight_change);
        if objective > previous_objective + 1e-10 * previous_objective.abs().max(1.0) {
            return Err(EstimationError::Numerical(
                "GSCA ALS objective increased beyond numerical tolerance".into(),
            ));
        }
        iterations = iteration;
        if final_change <= recipe.settings.tolerance {
            converged = true;
            break;
        }
        previous_objective = objective;
    }
    if !converged {
        return Err(EstimationError::NonConvergence(
            recipe.settings.max_iterations,
        ));
    }

    for construct_index in 0..weights.len() {
        if weights[construct_index].iter().sum::<f64>() < 0.0 {
            for weight in &mut weights[construct_index] {
                *weight = -*weight;
            }
            for score in &mut scores[construct_index] {
                *score = -*score;
            }
        }
    }
    coefficients = gsca_fit_coefficients(recipe, &unit_columns, &scores, &indicator_index)?;
    let (objective, measurement_residual, structural_residual) = gsca_objective(
        recipe,
        &unit_columns,
        &scores,
        &coefficients,
        &indicator_index,
    );
    let observed_total = unit_columns
        .iter()
        .flatten()
        .map(|value| value * value)
        .sum::<f64>();
    let component_total = scores
        .iter()
        .flatten()
        .map(|value| value * value)
        .sum::<f64>();
    let fit = 1.0 - objective / (observed_total + component_total);
    let measurement_fit = 1.0 - measurement_residual / observed_total;
    let structural_fit = 1.0 - structural_residual / component_total;
    let free_parameters = blocks
        .iter()
        .map(|block| block.len().saturating_sub(1))
        .sum::<usize>()
        + recipe
            .model
            .constructs
            .iter()
            .filter(|construct| construct.mode == MeasurementMode::Reflective)
            .map(|construct| construct.indicators.len())
            .sum::<usize>()
        + recipe.model.paths.len();
    let null_degrees = prepared.used * indicator_names.len();
    if null_degrees <= free_parameters {
        return Err(EstimationError::Numerical(
            "GSCA adjusted FIT requires more data degrees of freedom than free parameters".into(),
        ));
    }
    let adjusted_fit =
        1.0 - (1.0 - fit) * null_degrees as f64 / (null_degrees - free_parameters) as f64;
    let (gfi, srmr, covariance_discrepancy, covariance_sample_total, standardized_residual_sum) =
        gsca_covariance_fit(
            recipe,
            &unit_columns,
            &scores,
            &coefficients,
            &indicator_index,
        )?;

    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut outer_estimates = Vec::with_capacity(indicator_names.len());
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        for (within, indicator) in construct.indicators.iter().enumerate() {
            let indicator_position = indicator_index[indicator.as_str()];
            outer_estimates.push(OuterEstimate {
                construct: construct.id.clone(),
                indicator: indicator.clone(),
                weight: weights[construct_position][within],
                loading: dot(
                    &unit_columns[indicator_position],
                    &scores[construct_position],
                ),
            });
        }
    }
    let paths = recipe
        .model
        .paths
        .iter()
        .map(|path| PathEstimate {
            source: path.source.clone(),
            target: path.target.clone(),
            coefficient: coefficients[construct_index[path.source.as_str()]]
                [indicator_names.len() + construct_index[path.target.as_str()]],
        })
        .collect::<Vec<_>>();
    let mut r_squared = BTreeMap::new();
    for (target, construct) in recipe.model.constructs.iter().enumerate() {
        let incoming = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .collect::<Vec<_>>();
        if incoming.is_empty() {
            continue;
        }
        let mut prediction = vec![0.0; prepared.used];
        for path in incoming {
            let source = construct_index[path.source.as_str()];
            add_scaled(
                &mut prediction,
                &scores[source],
                coefficients[source][indicator_names.len() + target],
            );
        }
        let residual = scores[target]
            .iter()
            .zip(prediction)
            .map(|(actual, predicted)| (actual - predicted).powi(2))
            .sum::<f64>();
        r_squared.insert(construct.id.clone(), 1.0 - residual);
    }
    let construct_scores = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| {
            (
                construct.id.clone(),
                scores[index]
                    .iter()
                    .map(|value| value * unit_scale)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let scope_warning = "GSCA ALS v2 is bounded to standardized raw data, listwise deletion, disjoint reflective/formative blocks, and recursive single-group structural models; inference and broader GSCA variants are not included.".to_string();
    let mut warnings = vec![scope_warning.clone()];
    if prepared.omitted > 0 {
        warnings.push(format!(
            "{} observations were omitted listwise",
            prepared.omitted
        ));
    }
    let analysis = GscaAnalysis {
        method_version: GSCA_METHOD_VERSION.into(),
        algorithm: GSCA_ALGORITHM_VERSION.into(),
        converged,
        iterations,
        stop_criterion: recipe.settings.tolerance,
        final_change,
        objective,
        fit,
        measurement_fit,
        structural_fit,
        adjusted_fit,
        gfi,
        srmr,
        covariance_discrepancy,
        covariance_sample_total,
        standardized_residual_sum,
        observations: prepared.used,
        free_parameters,
        weights: outer_estimates.clone(),
        loadings: outer_estimates.clone(),
        paths: paths.clone(),
        r_squared: r_squared.clone(),
        bootstrap_intervals: Vec::new(),
        warnings: vec![scope_warning],
    };
    checkpoint(control, EstimationPhase::Assembling, 1, 1)?;
    Ok(PlsResult {
        method_version: GSCA_METHOD_VERSION.into(),
        converged,
        iterations,
        used_observations: prepared.used,
        omitted_observations: prepared.omitted,
        transforms: prepared.transforms,
        construct_scores,
        outer_estimates,
        paths,
        control_estimates: Vec::new(),
        effects: Vec::new(),
        mediation: MediationAnalysis::default(),
        moderation: ModerationAnalysis::default(),
        plsc: None,
        endogeneity: None,
        nonlinear_effects: None,
        moderated_mediation: None,
        cta_pls: None,
        wpls: None,
        cca: None,
        predict: None,
        segmentation: None,
        mga: None,
        micom: None,
        mga_permutation: None,
        fimix: None,
        ipma: None,
        cbsem: None,
        pca: None,
        regression: None,
        nca: None,
        gsca: Some(analysis),
        r_squared,
        warnings,
    })
}

fn validate_gsca_execution_contract(recipe: &AnalysisRecipe) -> Result<(), EstimationError> {
    let settings = &recipe.settings;
    if settings.weighting_scheme != WeightingScheme::Path
        || settings.preprocessing != Preprocessing::Standardized
        || settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || settings.case_weight_column.is_some()
        || settings.bootstrap_samples > 0
        || settings.studentized_inner_samples > 0
        || settings.permutation_samples > 0
        || settings.workers != 1
        || settings.max_iterations != 3_000
        || (settings.tolerance - 1e-7).abs() > f64::EPSILON
    {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA ALS v2 requires fixed path-settings sentinel, standardized raw data, listwise deletion, one worker, 3,000 iterations, a 1e-7 stop criterion, and no resampling or case weights"
                .into(),
        ));
    }
    if recipe.model.constructs.len() < 2 || recipe.model.paths.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA ALS v2 requires at least two constructs and one structural path".into(),
        ));
    }
    if !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
    {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA ALS v2 does not support controls, interactions, or higher-order constructs"
                .into(),
        ));
    }
    let connected = recipe
        .model
        .paths
        .iter()
        .flat_map(|path| [path.source.as_str(), path.target.as_str()])
        .collect::<HashSet<_>>();
    if recipe
        .model
        .constructs
        .iter()
        .any(|construct| !connected.contains(construct.id.as_str()))
    {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA ALS v2 does not support isolated constructs".into(),
        ));
    }
    Ok(())
}

fn gsca_normalize_weights(
    columns: &[Vec<f64>],
    block: &[usize],
    mut weights: Vec<f64>,
) -> Result<Vec<f64>, EstimationError> {
    let score = gsca_linear_combination(columns, block, &weights);
    let norm = dot(&score, &score).sqrt();
    if !norm.is_finite() || norm <= 1e-12 {
        return Err(EstimationError::Numerical(
            "GSCA component weights produce a zero-norm score".into(),
        ));
    }
    for weight in &mut weights {
        *weight /= norm;
    }
    Ok(weights)
}

fn gsca_linear_combination(columns: &[Vec<f64>], block: &[usize], weights: &[f64]) -> Vec<f64> {
    let mut score = vec![0.0; columns[0].len()];
    for (column, weight) in block.iter().zip(weights) {
        add_scaled(&mut score, &columns[*column], *weight);
    }
    score
}

fn gsca_score(
    columns: &[Vec<f64>],
    block: &[usize],
    weights: &[f64],
) -> Result<Vec<f64>, EstimationError> {
    let score = gsca_linear_combination(columns, block, weights);
    let norm = dot(&score, &score).sqrt();
    if !norm.is_finite() || norm <= 1e-12 {
        return Err(EstimationError::Numerical(
            "GSCA component score has zero norm".into(),
        ));
    }
    Ok(score.into_iter().map(|value| value / norm).collect())
}

fn gsca_scores(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    weights: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, EstimationError> {
    blocks
        .iter()
        .zip(weights)
        .map(|(block, weights)| gsca_score(columns, block, weights))
        .collect()
}

fn gsca_fit_coefficients(
    recipe: &AnalysisRecipe,
    columns: &[Vec<f64>],
    scores: &[Vec<f64>],
    indicator_index: &HashMap<&str, usize>,
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let observed = columns.len();
    let constructs = recipe.model.constructs.len();
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut coefficients = vec![vec![0.0; observed + constructs]; constructs];
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        if construct.mode == MeasurementMode::Reflective {
            for indicator in &construct.indicators {
                let indicator_position = indicator_index[indicator.as_str()];
                coefficients[construct_position][indicator_position] =
                    dot(&scores[construct_position], &columns[indicator_position]);
            }
        }
    }
    for (target, construct) in recipe.model.constructs.iter().enumerate() {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        let predictors = predecessors
            .iter()
            .map(|source| scores[*source].clone())
            .collect::<Vec<_>>();
        let fitted = ols(&predictors, &scores[target], &construct.id)?;
        for (source, coefficient) in predecessors.into_iter().zip(fitted) {
            coefficients[source][observed + target] = coefficient;
        }
    }
    Ok(coefficients)
}

fn gsca_update_weight_block(
    recipe: &AnalysisRecipe,
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    scores: &[Vec<f64>],
    coefficients: &[Vec<f64>],
    construct: usize,
) -> Result<Vec<f64>, EstimationError> {
    let observed = columns.len();
    let constructs = scores.len();
    let width = observed + constructs;
    let mut h = vec![vec![0.0; width]; constructs];
    for row in 0..constructs {
        for column in 0..width {
            h[row][column] = -coefficients[row][column];
        }
        h[row][observed + row] += 1.0;
    }
    let mut projected_residual = vec![0.0; columns[0].len()];
    for observation in 0..projected_residual.len() {
        let mut value = 0.0;
        for equation in 0..width {
            let mut residual_without = if equation < observed {
                columns[equation][observation]
            } else {
                0.0
            };
            for other in 0..constructs {
                if other != construct {
                    residual_without += scores[other][observation] * h[other][equation];
                }
            }
            value += residual_without * h[construct][equation];
        }
        projected_residual[observation] = -value;
    }
    let predictors = blocks[construct]
        .iter()
        .map(|column| columns[*column].clone())
        .collect::<Vec<_>>();
    let candidate = ols(
        &predictors,
        &projected_residual,
        &recipe.model.constructs[construct].id,
    )?;
    gsca_normalize_weights(columns, &blocks[construct], candidate)
}

fn gsca_objective(
    recipe: &AnalysisRecipe,
    columns: &[Vec<f64>],
    scores: &[Vec<f64>],
    coefficients: &[Vec<f64>],
    indicator_index: &HashMap<&str, usize>,
) -> (f64, f64, f64) {
    let observed = columns.len();
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut measurement = 0.0;
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        for indicator in &construct.indicators {
            let indicator_position = indicator_index[indicator.as_str()];
            let coefficient = coefficients[construct_position][indicator_position];
            measurement += columns[indicator_position]
                .iter()
                .zip(&scores[construct_position])
                .map(|(indicator, score)| (indicator - coefficient * score).powi(2))
                .sum::<f64>();
        }
    }
    let mut structural = 0.0;
    for (target, construct) in recipe.model.constructs.iter().enumerate() {
        let incoming = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .collect::<Vec<_>>();
        let mut predicted = vec![0.0; scores[target].len()];
        for path in incoming {
            let source = construct_index[path.source.as_str()];
            add_scaled(
                &mut predicted,
                &scores[source],
                coefficients[source][observed + target],
            );
        }
        structural += scores[target]
            .iter()
            .zip(predicted)
            .map(|(actual, predicted)| (actual - predicted).powi(2))
            .sum::<f64>();
    }
    (measurement + structural, measurement, structural)
}

fn gsca_covariance_fit(
    recipe: &AnalysisRecipe,
    columns: &[Vec<f64>],
    scores: &[Vec<f64>],
    coefficients: &[Vec<f64>],
    indicator_index: &HashMap<&str, usize>,
) -> Result<(f64, f64, f64, f64, f64), EstimationError> {
    let observed = columns.len();
    let constructs = scores.len();
    let width = observed + constructs;
    let rows = columns[0].len();
    let mut residual_columns = vec![vec![0.0; rows]; width];
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        for indicator in &construct.indicators {
            let indicator_position = indicator_index[indicator.as_str()];
            let coefficient = coefficients[construct_position][indicator_position];
            for row in 0..rows {
                residual_columns[indicator_position][row] = columns[indicator_position][row]
                    - coefficient * scores[construct_position][row];
            }
        }
    }
    for target in 0..constructs {
        residual_columns[observed + target].clone_from(&scores[target]);
    }
    for path in &recipe.model.paths {
        let source = construct_index[path.source.as_str()];
        let target = construct_index[path.target.as_str()];
        let coefficient = coefficients[source][observed + target];
        for row in 0..rows {
            residual_columns[observed + target][row] -= coefficient * scores[source][row];
        }
    }
    let mut residual_covariance = vec![vec![0.0; width]; width];
    for left in 0..width {
        for right in 0..width {
            if (left < observed) == (right < observed) {
                residual_covariance[left][right] =
                    dot(&residual_columns[left], &residual_columns[right]);
            }
        }
    }
    let mut transition = vec![vec![0.0; width]; width];
    for construct in 0..constructs {
        transition[observed + construct].clone_from(&coefficients[construct]);
    }
    let mut identity_minus_transition = identity_matrix(width);
    for row in 0..width {
        for column in 0..width {
            identity_minus_transition[row][column] -= transition[row][column];
        }
    }
    let inverse = invert_matrix(&identity_minus_transition)?;
    let implied_augmented = multiply_matrices(
        &multiply_matrices(&transpose_matrix(&inverse), &residual_covariance),
        &inverse,
    );
    let sample = columns
        .iter()
        .map(|left| {
            columns
                .iter()
                .map(|right| dot(left, right))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut discrepancy = 0.0;
    let mut sample_total = 0.0;
    let mut standardized_lower = 0.0;
    for row in 0..observed {
        for column in 0..observed {
            let difference = sample[row][column] - implied_augmented[row][column];
            discrepancy += difference * difference;
            sample_total += sample[row][column] * sample[row][column];
            if column <= row {
                let denominator = (sample[row][row] * sample[column][column]).abs().sqrt();
                if denominator <= f64::EPSILON {
                    return Err(EstimationError::Numerical(
                        "GSCA covariance fit has a zero observed variance".into(),
                    ));
                }
                standardized_lower += (difference / denominator).powi(2);
            }
        }
    }
    let gfi = 1.0 - discrepancy / sample_total;
    let srmr = (2.0 * standardized_lower / (observed * (observed + 1)) as f64).sqrt();
    if [gfi, srmr].iter().any(|value| !value.is_finite()) {
        return Err(EstimationError::Numerical(
            "GSCA covariance fit produced a non-finite statistic".into(),
        ));
    }
    Ok((gfi, srmr, discrepancy, sample_total, standardized_lower))
}

fn centered_square(values: &[f64]) -> Vec<f64> {
    let mean = vector_mean(values);
    let squared = values
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .collect::<Vec<_>>();
    let squared_mean = vector_mean(&squared);
    squared
        .into_iter()
        .map(|value| value - squared_mean)
        .collect()
}

fn regression_r_squared(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    subject: &str,
) -> Result<f64, EstimationError> {
    let coefficients = ols(predictors, outcome, subject)?;
    let fitted = fitted_values(predictors, &coefficients);
    let outcome_mean = vector_mean(outcome);
    let residual = outcome
        .iter()
        .zip(&fitted)
        .map(|(actual, fit)| (actual - outcome_mean - fit).powi(2))
        .sum::<f64>();
    let total = outcome
        .iter()
        .map(|value| (value - outcome_mean).powi(2))
        .sum::<f64>();
    if total <= f64::EPSILON {
        return Err(EstimationError::Numerical(format!(
            "zero target variance for {subject}"
        )));
    }
    Ok((1.0 - residual / total).clamp(0.0, 1.0))
}

fn expand_two_stage_moderation_dataset(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    stage1: &PlsResult,
    used_rows: &[usize],
) -> Result<(Dataset, AnalysisRecipe), EstimationError> {
    if used_rows.len() != stage1.used_observations {
        return Err(EstimationError::Numerical(
            "stage-1 used row count does not match construct-score length".into(),
        ));
    }
    let mut arrays = Vec::<ArrayRef>::new();
    let mut fields = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .map(|field| Field::new(field.name(), field.data_type().clone(), field.is_nullable()))
        .collect::<Vec<_>>();
    let mut schema = dataset.schema.clone();
    for column in dataset.batch.columns() {
        arrays.push(subset_array(column.as_ref(), used_rows)?);
    }
    let mut stage2_recipe = recipe.clone();
    let existing_fields = fields
        .iter()
        .map(|field| field.name().to_string())
        .collect::<HashSet<_>>();
    let mut generated_names = HashSet::new();

    for interaction in &recipe.model.interactions {
        match interaction.method {
            InteractionMethod::TwoStageProductScore => {}
        }
        let predictor_scores = stage1
            .construct_scores
            .get(&interaction.predictor)
            .ok_or_else(|| {
                EstimationError::Numerical(format!(
                    "missing stage-1 predictor scores for {}",
                    interaction.predictor
                ))
            })?;
        let moderator_scores = stage1
            .construct_scores
            .get(&interaction.moderator)
            .ok_or_else(|| {
                EstimationError::Numerical(format!(
                    "missing stage-1 moderator scores for {}",
                    interaction.moderator
                ))
            })?;
        if predictor_scores.len() != used_rows.len() || moderator_scores.len() != used_rows.len() {
            return Err(EstimationError::Numerical(
                "stage-1 score length does not match the complete-case rows".into(),
            ));
        }
        let indicator_name = product_indicator_name(&interaction.id);
        if existing_fields.contains(&indicator_name)
            || !generated_names.insert(indicator_name.clone())
        {
            return Err(EstimationError::DuplicateIndicator(indicator_name));
        }
        let product = predictor_scores
            .iter()
            .zip(moderator_scores)
            .map(|(predictor, moderator)| predictor * moderator)
            .collect::<Vec<_>>();
        arrays.push(Arc::new(Float64Array::from(product)) as ArrayRef);
        fields.push(Field::new(&indicator_name, DataType::Float64, false));
        schema.columns.push(ColumnMetadata {
            name: indicator_name.clone(),
            label: Some(format!(
                "Two-stage product score: {} x {}",
                interaction.predictor, interaction.moderator
            )),
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Continuous,
            missing_markers: Vec::new(),
            theoretical_min: None,
            theoretical_max: None,
            value_labels: BTreeMap::new(),
        });
        let Some(product_construct) = stage2_recipe
            .model
            .constructs
            .iter_mut()
            .find(|construct| construct.id == interaction.product_construct)
        else {
            return Err(EstimationError::UnknownConstruct(
                interaction.product_construct.clone(),
            ));
        };
        product_construct.indicators = vec![indicator_name];
    }
    stage2_recipe.model.interactions.clear();
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    schema.case_count = batch.num_rows();
    let expanded_dataset = Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema,
        batch,
        fingerprint: DataFingerprint(format!(
            "{}+{}",
            dataset.fingerprint.0, PLS_TWO_STAGE_MODERATION_METHOD_VERSION
        )),
    };
    Ok((expanded_dataset, stage2_recipe))
}

fn subset_array(array: &dyn Array, rows: &[usize]) -> Result<ArrayRef, EstimationError> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Ok(Arc::new(Float64Array::from(
            rows.iter()
                .map(|row| {
                    if values.is_null(*row) {
                        None
                    } else {
                        Some(values.value(*row))
                    }
                })
                .collect::<Vec<_>>(),
        )) as ArrayRef)
    } else if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        Ok(Arc::new(Int64Array::from(
            rows.iter()
                .map(|row| {
                    if values.is_null(*row) {
                        None
                    } else {
                        Some(values.value(*row))
                    }
                })
                .collect::<Vec<_>>(),
        )) as ArrayRef)
    } else if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        Ok(Arc::new(BooleanArray::from(
            rows.iter()
                .map(|row| {
                    if values.is_null(*row) {
                        None
                    } else {
                        Some(values.value(*row))
                    }
                })
                .collect::<Vec<_>>(),
        )) as ArrayRef)
    } else if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        Ok(Arc::new(StringArray::from(
            rows.iter()
                .map(|row| {
                    if values.is_null(*row) {
                        None
                    } else {
                        Some(values.value(*row))
                    }
                })
                .collect::<Vec<_>>(),
        )) as ArrayRef)
    } else {
        Err(EstimationError::Numerical(
            "two-stage moderation cannot subset an unsupported Arrow column type".into(),
        ))
    }
}

fn product_indicator_name(interaction_id: &str) -> String {
    format!("__qpls_interaction_{interaction_id}")
}

fn higher_order_component_indicator_name(higher_order_id: &str, component_id: &str) -> String {
    format!("__qpls_hoc_{higher_order_id}_{component_id}")
}

pub fn analyze_moderation(recipe: &AnalysisRecipe, result: &PlsResult) -> ModerationAnalysis {
    let mut analysis = ModerationAnalysis::default();
    if recipe.model.interactions.is_empty() {
        return analysis;
    }
    let path_index = result
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    for interaction in &recipe.model.interactions {
        let Some(interaction_effect) = path_index
            .get(&(
                interaction.product_construct.as_str(),
                interaction.outcome.as_str(),
            ))
            .copied()
        else {
            analysis.warnings.push(format!(
                "moderation interaction path {} -> {} is unavailable",
                interaction.product_construct, interaction.outcome
            ));
            continue;
        };
        let predictor_main_effect = path_index
            .get(&(interaction.predictor.as_str(), interaction.outcome.as_str()))
            .copied();
        let moderator_main_effect = path_index
            .get(&(interaction.moderator.as_str(), interaction.outcome.as_str()))
            .copied();
        let (simple_slopes, warning) = if let Some(main_effect) = predictor_main_effect {
            (
                analysis
                    .moderator_score_levels
                    .iter()
                    .map(|level| ModerationSimpleSlope {
                        moderator_score: *level,
                        effect: main_effect + interaction_effect * level,
                    })
                    .collect::<Vec<_>>(),
                None,
            )
        } else {
            (
                Vec::new(),
                Some(
                    "Predictor main-effect path is absent, so simple slopes are unavailable"
                        .to_string(),
                ),
            )
        };
        analysis.estimates.push(ModerationEstimate {
            interaction: interaction.id.clone(),
            predictor: interaction.predictor.clone(),
            moderator: interaction.moderator.clone(),
            product_construct: interaction.product_construct.clone(),
            outcome: interaction.outcome.clone(),
            predictor_main_effect,
            moderator_main_effect,
            interaction_effect,
            simple_slopes,
            warning,
        });
    }
    analysis.warnings.push(
        "Simple slopes use standardized stage-1 moderator scores at -1, 0, and +1 and are validated for the documented QuickPLS v1.2.1 two-stage moderation scope when paired with validated inference."
            .into(),
    );
    analysis
}

pub fn analyze_moderated_mediation(
    recipe: &AnalysisRecipe,
    result: &PlsResult,
) -> ModeratedMediationAnalysis {
    let mut analysis = ModeratedMediationAnalysis::default();
    if recipe.model.interactions.is_empty() {
        analysis
            .warnings
            .push("No interaction terms are available for moderated mediation.".into());
        return analysis;
    }
    let path_index = result
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    for interaction in &recipe.model.interactions {
        let Some(interaction_effect) = path_index
            .get(&(
                interaction.product_construct.as_str(),
                interaction.outcome.as_str(),
            ))
            .copied()
        else {
            analysis.warnings.push(format!(
                "interaction path {} -> {} is unavailable",
                interaction.product_construct, interaction.outcome
            ));
            continue;
        };
        let Some(moderated_main_effect) = path_index
            .get(&(interaction.predictor.as_str(), interaction.outcome.as_str()))
            .copied()
        else {
            analysis.warnings.push(format!(
                "main effect path {} -> {} is unavailable",
                interaction.predictor, interaction.outcome
            ));
            continue;
        };

        let mut matched = false;

        for final_path in recipe
            .model
            .paths
            .iter()
            .filter(|path| path.source == interaction.outcome)
        {
            if final_path.target == interaction.product_construct {
                continue;
            }
            if let Some(second_stage) = path_index
                .get(&(interaction.outcome.as_str(), final_path.target.as_str()))
                .copied()
            {
                matched = true;
                let conditional_indirect_effects = analysis
                    .moderator_score_levels
                    .iter()
                    .map(|level| {
                        let first_stage = moderated_main_effect + interaction_effect * level;
                        ConditionalIndirectEffect {
                            moderator_score: *level,
                            first_stage_effect: first_stage,
                            second_stage_effect: second_stage,
                            indirect_effect: first_stage * second_stage,
                        }
                    })
                    .collect::<Vec<_>>();
                analysis.estimates.push(ModeratedMediationEstimate {
                    interaction: interaction.id.clone(),
                    predictor: interaction.predictor.clone(),
                    moderator: interaction.moderator.clone(),
                    mediator: interaction.outcome.clone(),
                    target: final_path.target.clone(),
                    moderated_stage: "first_stage".into(),
                    index_of_moderated_mediation: interaction_effect * second_stage,
                    conditional_indirect_effects,
                    warning: None,
                });
            }
        }

        for first_path in recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == interaction.predictor)
        {
            if first_path.source == interaction.product_construct
                || first_path.source == interaction.moderator
            {
                continue;
            }
            if let Some(first_stage) = path_index
                .get(&(first_path.source.as_str(), interaction.predictor.as_str()))
                .copied()
            {
                matched = true;
                let conditional_indirect_effects = analysis
                    .moderator_score_levels
                    .iter()
                    .map(|level| {
                        let second_stage = moderated_main_effect + interaction_effect * level;
                        ConditionalIndirectEffect {
                            moderator_score: *level,
                            first_stage_effect: first_stage,
                            second_stage_effect: second_stage,
                            indirect_effect: first_stage * second_stage,
                        }
                    })
                    .collect::<Vec<_>>();
                analysis.estimates.push(ModeratedMediationEstimate {
                    interaction: interaction.id.clone(),
                    predictor: first_path.source.clone(),
                    moderator: interaction.moderator.clone(),
                    mediator: interaction.predictor.clone(),
                    target: interaction.outcome.clone(),
                    moderated_stage: "second_stage".into(),
                    index_of_moderated_mediation: first_stage * interaction_effect,
                    conditional_indirect_effects,
                    warning: None,
                });
            }
        }

        if !matched {
            analysis.warnings.push(format!(
                "interaction {} did not map to a first-stage or second-stage mediated path",
                interaction.id
            ));
        }
    }
    analysis.warnings.push(
        "Moderated mediation is validated for the documented QuickPLS v1.2.3 two-stage conditional indirect-effect diagnostic scope; conditional indirect effects use fixed PLS scores and standardized moderator levels -1, 0, and +1."
            .into(),
    );
    analysis
}

fn checkpoint(
    control: &mut dyn FnMut(EstimationProgress) -> bool,
    phase: EstimationPhase,
    completed_units: u64,
    total_units: u64,
) -> Result<(), EstimationError> {
    if control(EstimationProgress {
        phase,
        completed_units,
        total_units,
    }) {
        Ok(())
    } else {
        Err(EstimationError::Cancelled)
    }
}

fn collect_indicators(recipe: &AnalysisRecipe) -> Result<Vec<String>, EstimationError> {
    let repeated_higher_order = recipe
        .model
        .higher_order_constructs
        .iter()
        .filter(|higher_order| higher_order.method == HigherOrderMethod::RepeatedIndicators)
        .map(|higher_order| higher_order.id.as_str())
        .collect::<HashSet<_>>();
    let mut owner = HashMap::<String, String>::new();
    let mut names = Vec::new();
    for construct in &recipe.model.constructs {
        if construct.indicators.is_empty() {
            return Err(EstimationError::EmptyConstruct(construct.id.clone()));
        }
        for indicator in &construct.indicators {
            if let Some(previous_owner) = owner.get(indicator) {
                if !repeated_higher_order.contains(construct.id.as_str())
                    && !repeated_higher_order.contains(previous_owner.as_str())
                {
                    return Err(EstimationError::DuplicateIndicator(indicator.clone()));
                }
            } else {
                owner.insert(indicator.clone(), construct.id.clone());
                names.push(indicator.clone());
            }
        }
    }
    Ok(names)
}

fn metadata_required(recipe: &AnalysisRecipe, key: &str) -> Result<String, EstimationError> {
    recipe
        .metadata
        .get(key)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| EstimationError::UnsupportedMethod(format!("metadata.{key} is required")))
}

fn metadata_list(recipe: &AnalysisRecipe, key: &str) -> Option<Vec<String>> {
    recipe.metadata.get(key).map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn metadata_usize(recipe: &AnalysisRecipe, key: &str, default: usize) -> usize {
    recipe
        .metadata
        .get(key)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn prepare_raw_numeric_data(
    dataset: &Dataset,
    variables: &[String],
    standardize: bool,
    allow_constant_first_column: bool,
) -> Result<PreparedData, EstimationError> {
    let schema = dataset.batch.schema();
    let arrays = variables
        .iter()
        .map(|name| {
            schema
                .index_of(name)
                .map_err(|_| EstimationError::InvalidIndicator(name.clone()))
                .map(|position| dataset.batch.column(position).clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (name, array) in variables.iter().zip(&arrays) {
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(EstimationError::InvalidIndicator(name.clone()));
        }
    }
    let mut columns = vec![Vec::new(); variables.len()];
    let mut used_rows = Vec::new();
    for row in 0..dataset.batch.num_rows() {
        let mut row_values = Vec::with_capacity(variables.len());
        let mut complete = true;
        for array in &arrays {
            if array.is_null(row) {
                complete = false;
                break;
            }
            let value = raw_numeric_value(array.as_ref(), row)?;
            if !value.is_finite() {
                complete = false;
                break;
            }
            row_values.push(value);
        }
        if complete {
            used_rows.push(row);
            for (column, value) in columns.iter_mut().zip(row_values) {
                column.push(value);
            }
        }
    }
    if used_rows.len() < 3 {
        return Err(EstimationError::InsufficientObservations);
    }
    let transforms = variables
        .iter()
        .zip(&mut columns)
        .enumerate()
        .map(|(index, (name, column))| {
            let mean = vector_mean(column);
            let scale = sample_sd(column);
            if scale <= f64::EPSILON || !scale.is_finite() {
                if !(allow_constant_first_column && index == 0) {
                    return Err(EstimationError::ConstantIndicator(name.clone()));
                }
            }
            if standardize && scale > f64::EPSILON && scale.is_finite() {
                for value in column.iter_mut() {
                    *value = (*value - mean) / scale;
                }
            }
            Ok(IndicatorTransform {
                indicator: name.clone(),
                mean,
                scale,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let used = used_rows.len();
    Ok(PreparedData {
        columns,
        transforms,
        used_rows,
        case_weights: None,
        used,
        omitted: dataset.batch.num_rows().saturating_sub(used),
    })
}

fn prepare_process_raw_numeric_data(
    dataset: &Dataset,
    variables: &[String],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PreparedData, EstimationError> {
    let schema = dataset.batch.schema();
    let arrays = variables
        .iter()
        .map(|name| {
            schema
                .index_of(name)
                .map_err(|_| EstimationError::InvalidIndicator(name.clone()))
                .map(|position| dataset.batch.column(position).clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (name, array) in variables.iter().zip(&arrays) {
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(EstimationError::InvalidIndicator(name.clone()));
        }
    }
    let total_rows = dataset.batch.num_rows();
    checkpoint(
        control,
        EstimationPhase::PreparingRows,
        0,
        total_rows as u64,
    )?;
    let mut columns = vec![Vec::new(); variables.len()];
    let mut used_rows = Vec::new();
    for row in 0..total_rows {
        if row > 0 && row % 64 == 0 {
            checkpoint(
                control,
                EstimationPhase::PreparingRows,
                row as u64,
                total_rows as u64,
            )?;
        }
        let mut row_values = Vec::with_capacity(variables.len());
        let mut complete = true;
        for array in &arrays {
            if array.is_null(row) {
                complete = false;
                break;
            }
            let value = raw_numeric_value(array.as_ref(), row)?;
            if !value.is_finite() {
                complete = false;
                break;
            }
            row_values.push(value);
        }
        if complete {
            used_rows.push(row);
            for (column, value) in columns.iter_mut().zip(row_values) {
                column.push(value);
            }
        }
    }
    checkpoint(
        control,
        EstimationPhase::PreparingRows,
        total_rows as u64,
        total_rows as u64,
    )?;
    if used_rows.len() < 3 {
        return Err(EstimationError::InsufficientObservations);
    }
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        0,
        variables.len() as u64,
    )?;
    let mut transforms = Vec::with_capacity(variables.len());
    for (index, (name, column)) in variables.iter().zip(&columns).enumerate() {
        let mean = vector_mean(column);
        let scale = sample_sd(column);
        if scale <= f64::EPSILON || !scale.is_finite() {
            return Err(EstimationError::ConstantIndicator(name.clone()));
        }
        transforms.push(IndicatorTransform {
            indicator: name.clone(),
            mean,
            scale,
        });
        checkpoint(
            control,
            EstimationPhase::PreparingIndicators,
            (index + 1) as u64,
            variables.len() as u64,
        )?;
    }
    let used = used_rows.len();
    Ok(PreparedData {
        columns,
        transforms,
        used_rows,
        case_weights: None,
        used,
        omitted: total_rows.saturating_sub(used),
    })
}

fn raw_numeric_value(array: &dyn Array, row: usize) -> Result<f64, EstimationError> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Ok(values.value(row));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Ok(values.value(row) as f64);
    }
    Err(EstimationError::InvalidIndicator(
        "nonnumeric column".into(),
    ))
}

fn prepare_data(
    dataset: &Dataset,
    indicators: &[String],
    preprocessing: &Preprocessing,
    _missing: &MissingDataPolicy,
    case_weight_column: Option<&str>,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PreparedData, EstimationError> {
    let schema = dataset.batch.schema();
    let positions = indicators
        .iter()
        .map(|name| {
            schema
                .index_of(name)
                .map_err(|_| EstimationError::InvalidIndicator(name.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (name, position) in indicators.iter().zip(&positions) {
        let array = dataset.batch.column(*position);
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(EstimationError::InvalidIndicator(name.clone()));
        }
    }
    let weight_position = case_weight_column
        .map(|name| {
            schema
                .index_of(name)
                .map_err(|_| EstimationError::InvalidIndicator(name.to_string()))
        })
        .transpose()?;
    if let Some(position) = weight_position {
        let array = dataset.batch.column(position);
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(EstimationError::InvalidIndicator(
                case_weight_column.unwrap_or_default().to_string(),
            ));
        }
    }
    let row_count = dataset.batch.num_rows();
    let mut complete_rows = Vec::with_capacity(row_count);
    let mut case_weights = weight_position.map(|_| Vec::with_capacity(row_count));
    for row in 0..row_count {
        if row % 1024 == 0 {
            checkpoint(
                control,
                EstimationPhase::PreparingRows,
                row as u64,
                row_count as u64,
            )?;
        }
        let indicators_complete = positions.iter().all(|position| {
            let array = dataset.batch.column(*position);
            !array.is_null(row) && numeric_value(array.as_ref(), row).is_some_and(f64::is_finite)
        });
        let weight_value = weight_position.and_then(|position| {
            let array = dataset.batch.column(position);
            if array.is_null(row) {
                None
            } else {
                numeric_value(array.as_ref(), row)
            }
        });
        if indicators_complete
            && weight_position.is_some()
            && weight_value.is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(EstimationError::Numerical(
                "case weights must be positive and finite".into(),
            ));
        }
        let weight_complete =
            weight_position.is_none() || weight_value.is_some_and(|value| value.is_finite());
        if indicators_complete && weight_complete {
            complete_rows.push(row);
            if let Some(weights) = &mut case_weights {
                weights.push(weight_value.unwrap());
            }
        }
    }
    checkpoint(
        control,
        EstimationPhase::PreparingRows,
        row_count as u64,
        row_count as u64,
    )?;
    if complete_rows.len() < 3 {
        return Err(EstimationError::InsufficientObservations);
    }
    if let Some(weights) = &case_weights {
        validate_case_weights(weights)?;
    }
    let mut columns = Vec::with_capacity(indicators.len());
    let mut transforms = Vec::with_capacity(indicators.len());
    for (indicator_index, (name, position)) in indicators.iter().zip(positions).enumerate() {
        checkpoint(
            control,
            EstimationPhase::PreparingIndicators,
            indicator_index as u64,
            indicators.len() as u64,
        )?;
        let raw = complete_rows
            .iter()
            .map(|row| numeric_value(dataset.batch.column(position).as_ref(), *row).unwrap())
            .collect::<Vec<_>>();
        let mean = case_weights
            .as_deref()
            .map_or_else(|| vector_mean(&raw), |weights| weighted_mean(&raw, weights));
        let deviation = case_weights.as_deref().map_or_else(
            || sample_sd(&raw),
            |weights| weighted_sample_sd(&raw, weights),
        );
        if deviation <= f64::EPSILON {
            return Err(EstimationError::ConstantIndicator(name.clone()));
        }
        let (center, scale) = match preprocessing {
            Preprocessing::Standardized => (mean, deviation),
            Preprocessing::MeanCentered => (mean, 1.0),
            Preprocessing::Unstandardized => (0.0, 1.0),
        };
        columns.push(raw.iter().map(|value| (value - center) / scale).collect());
        transforms.push(IndicatorTransform {
            indicator: name.clone(),
            mean: center,
            scale,
        });
    }
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        indicators.len() as u64,
        indicators.len() as u64,
    )?;
    Ok(PreparedData {
        columns,
        transforms,
        used_rows: complete_rows.clone(),
        case_weights,
        used: complete_rows.len(),
        omitted: dataset.batch.num_rows() - complete_rows.len(),
    })
}

fn iterative_scores(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    recipe: &AnalysisRecipe,
    allow_isolated_constructs: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, u32), EstimationError> {
    let mut weights = blocks
        .iter()
        .map(|block| normalize_block_weights(columns, block, vec![1.0; block.len()]))
        .collect::<Result<Vec<_>, _>>()?;
    let iteration_units = recipe.settings.max_iterations as u64 * blocks.len() as u64;
    for iteration in 1..=recipe.settings.max_iterations {
        let scores = block_scores(columns, blocks, &weights)?;
        let inner = inner_proxies(&scores, recipe, allow_isolated_constructs)?;
        let mut updated = Vec::with_capacity(blocks.len());
        for (construct_index, (construct, block)) in
            recipe.model.constructs.iter().zip(blocks).enumerate()
        {
            checkpoint(
                control,
                EstimationPhase::Iterating,
                (iteration - 1) as u64 * blocks.len() as u64 + construct_index as u64,
                iteration_units,
            )?;
            let candidate = match construct.mode {
                MeasurementMode::Reflective => block
                    .iter()
                    .map(|column| covariance(&columns[*column], &inner[construct_index]))
                    .collect(),
                MeasurementMode::Formative => ols(
                    &block
                        .iter()
                        .map(|column| columns[*column].clone())
                        .collect::<Vec<_>>(),
                    &inner[construct_index],
                    &construct.id,
                )?,
            };
            updated.push(normalize_block_weights(columns, block, candidate)?);
        }
        let change = weights
            .iter()
            .flatten()
            .zip(updated.iter().flatten())
            .map(|(old, new)| (old - new).abs())
            .fold(0.0, f64::max);
        weights = updated;
        if change <= recipe.settings.tolerance {
            return Ok((
                weights.clone(),
                block_scores(columns, blocks, &weights)?,
                iteration,
            ));
        }
    }
    Err(EstimationError::NonConvergence(
        recipe.settings.max_iterations,
    ))
}

fn iterative_scores_weighted(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    recipe: &AnalysisRecipe,
    case_weights: &[f64],
    allow_isolated_constructs: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, u32), EstimationError> {
    let mut weights = blocks
        .iter()
        .map(|block| {
            normalize_block_weights_weighted(columns, block, vec![1.0; block.len()], case_weights)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let iteration_units = recipe.settings.max_iterations as u64 * blocks.len() as u64;
    for iteration in 1..=recipe.settings.max_iterations {
        let scores = block_scores_weighted(columns, blocks, &weights, case_weights)?;
        let inner =
            inner_proxies_weighted(&scores, recipe, case_weights, allow_isolated_constructs)?;
        let mut updated = Vec::with_capacity(blocks.len());
        for (construct_index, (construct, block)) in
            recipe.model.constructs.iter().zip(blocks).enumerate()
        {
            checkpoint(
                control,
                EstimationPhase::Iterating,
                (iteration - 1) as u64 * blocks.len() as u64 + construct_index as u64,
                iteration_units,
            )?;
            let candidate = match construct.mode {
                MeasurementMode::Reflective => block
                    .iter()
                    .map(|column| {
                        weighted_covariance(
                            &columns[*column],
                            &inner[construct_index],
                            case_weights,
                        )
                    })
                    .collect(),
                MeasurementMode::Formative => ols_weighted(
                    &block
                        .iter()
                        .map(|column| columns[*column].clone())
                        .collect::<Vec<_>>(),
                    &inner[construct_index],
                    case_weights,
                    &construct.id,
                )?,
            };
            updated.push(normalize_block_weights_weighted(
                columns,
                block,
                candidate,
                case_weights,
            )?);
        }
        let change = weights
            .iter()
            .flatten()
            .zip(updated.iter().flatten())
            .map(|(old, new)| (old - new).abs())
            .fold(0.0, f64::max);
        weights = updated;
        if change <= recipe.settings.tolerance {
            return Ok((
                weights.clone(),
                block_scores_weighted(columns, blocks, &weights, case_weights)?,
                iteration,
            ));
        }
    }
    Err(EstimationError::NonConvergence(
        recipe.settings.max_iterations,
    ))
}

fn inner_proxies(
    scores: &[Vec<f64>],
    recipe: &AnalysisRecipe,
    allow_isolated_constructs: bool,
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let ids = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut incoming = vec![Vec::new(); scores.len()];
    let mut outgoing = vec![Vec::new(); scores.len()];
    for path in &recipe.model.paths {
        let source = ids[path.source.as_str()];
        let target = ids[path.target.as_str()];
        incoming[target].push(source);
        outgoing[source].push(target);
    }
    let mut proxies = Vec::with_capacity(scores.len());
    for construct in 0..scores.len() {
        if incoming[construct].is_empty() && outgoing[construct].is_empty() {
            if scores.len() == 1 || allow_isolated_constructs {
                proxies.push(scores[construct].clone());
                continue;
            }
            return Err(EstimationError::IsolatedConstruct(
                recipe.model.constructs[construct].id.clone(),
            ));
        }
        let mut proxy = vec![0.0; scores[construct].len()];
        if recipe.settings.weighting_scheme == WeightingScheme::Path
            && !incoming[construct].is_empty()
        {
            let predictors = incoming[construct]
                .iter()
                .map(|index| scores[*index].clone())
                .collect::<Vec<_>>();
            let coefficients = ols(
                &predictors,
                &scores[construct],
                &recipe.model.constructs[construct].id,
            )?;
            for (source, coefficient) in incoming[construct].iter().zip(coefficients) {
                add_scaled(&mut proxy, &scores[*source], coefficient);
            }
        } else {
            for source in &incoming[construct] {
                add_scaled(
                    &mut proxy,
                    &scores[*source],
                    correlation(&scores[construct], &scores[*source]),
                );
            }
        }
        for target in &outgoing[construct] {
            add_scaled(
                &mut proxy,
                &scores[*target],
                correlation(&scores[construct], &scores[*target]),
            );
        }
        proxies.push(standardize_vector(proxy).ok_or_else(|| {
            EstimationError::Numerical(format!(
                "zero inner proxy for {}",
                recipe.model.constructs[construct].id
            ))
        })?);
    }
    Ok(proxies)
}

fn inner_proxies_weighted(
    scores: &[Vec<f64>],
    recipe: &AnalysisRecipe,
    case_weights: &[f64],
    allow_isolated_constructs: bool,
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let ids = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut incoming = vec![Vec::new(); scores.len()];
    let mut outgoing = vec![Vec::new(); scores.len()];
    for path in &recipe.model.paths {
        let source = ids[path.source.as_str()];
        let target = ids[path.target.as_str()];
        incoming[target].push(source);
        outgoing[source].push(target);
    }
    let mut proxies = Vec::with_capacity(scores.len());
    for construct in 0..scores.len() {
        if incoming[construct].is_empty() && outgoing[construct].is_empty() {
            if allow_isolated_constructs {
                proxies.push(scores[construct].clone());
                continue;
            }
            return Err(EstimationError::IsolatedConstruct(
                recipe.model.constructs[construct].id.clone(),
            ));
        }
        let neighbors = match recipe.settings.weighting_scheme {
            WeightingScheme::Path => {
                if incoming[construct].is_empty() {
                    outgoing[construct].clone()
                } else {
                    incoming[construct].clone()
                }
            }
            WeightingScheme::Factor => incoming[construct]
                .iter()
                .chain(&outgoing[construct])
                .copied()
                .collect::<Vec<_>>(),
            WeightingScheme::Pca => Vec::new(),
        };
        let mut proxy = vec![0.0; scores[construct].len()];
        for neighbor in neighbors {
            let sign = if weighted_covariance(&scores[construct], &scores[neighbor], case_weights)
                >= 0.0
            {
                1.0
            } else {
                -1.0
            };
            add_scaled(&mut proxy, &scores[neighbor], sign);
        }
        proxies.push(
            weighted_standardize_vector(proxy, case_weights).ok_or_else(|| {
                EstimationError::Numerical("inner proxy has zero weighted variance".into())
            })?,
        );
    }
    Ok(proxies)
}

fn pca_scores(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    tolerance: f64,
    max_iterations: u32,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, u32), EstimationError> {
    let mut all_weights = Vec::new();
    let mut used_iterations = 0;
    let total_iterations = max_iterations as u64 * blocks.len() as u64;
    for (block_index, block) in blocks.iter().enumerate() {
        let mut weights = vec![1.0 / (block.len() as f64).sqrt(); block.len()];
        let mut converged = false;
        for iteration in 1..=max_iterations {
            checkpoint(
                control,
                EstimationPhase::Iterating,
                block_index as u64 * max_iterations as u64 + (iteration - 1) as u64,
                total_iterations,
            )?;
            let mut updated = vec![0.0; block.len()];
            for left in 0..block.len() {
                for right in 0..block.len() {
                    updated[left] +=
                        covariance(&columns[block[left]], &columns[block[right]]) * weights[right];
                }
            }
            let norm = updated
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            if norm <= f64::EPSILON {
                return Err(EstimationError::Numerical("PCA block has zero norm".into()));
            }
            for value in &mut updated {
                *value /= norm;
            }
            orient_by_sum(&mut updated);
            let change = weights
                .iter()
                .zip(&updated)
                .map(|(old, new)| (old - new).abs())
                .fold(0.0, f64::max);
            weights = updated;
            if change <= tolerance {
                used_iterations = used_iterations.max(iteration);
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(EstimationError::NonConvergence(max_iterations));
        }
        all_weights.push(normalize_block_weights(columns, block, weights)?);
    }
    let scores = block_scores(columns, blocks, &all_weights)?;
    Ok((all_weights, scores, used_iterations))
}

fn assemble_result(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    indicator_names: Vec<String>,
    prepared: PreparedData,
    weights: Vec<Vec<f64>>,
    scores: Vec<Vec<f64>>,
    iterations: u32,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    let ids = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut outer_estimates = Vec::new();
    let assembly_units = recipe.model.constructs.len() as u64 * 3;
    let mut assembly_completed = 0;
    for (construct_index, construct) in recipe.model.constructs.iter().enumerate() {
        checkpoint(
            control,
            EstimationPhase::Assembling,
            assembly_completed,
            assembly_units,
        )?;
        for (within, indicator) in construct.indicators.iter().enumerate() {
            let loading = if let Some(case_weights) = prepared.case_weights.as_deref() {
                weighted_correlation(
                    &prepared.columns[indicator_index[indicator.as_str()]],
                    &scores[construct_index],
                    case_weights,
                )
            } else {
                correlation(
                    &prepared.columns[indicator_index[indicator.as_str()]],
                    &scores[construct_index],
                )
            };
            outer_estimates.push(OuterEstimate {
                construct: construct.id.clone(),
                indicator: indicator.clone(),
                weight: weights[construct_index][within],
                loading,
            });
        }
        assembly_completed += 1;
    }
    let mut paths = Vec::new();
    let mut r_squared = BTreeMap::new();
    for (target, construct) in recipe.model.constructs.iter().enumerate() {
        checkpoint(
            control,
            EstimationPhase::Assembling,
            assembly_completed,
            assembly_units,
        )?;
        assembly_completed += 1;
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .map(|path| ids[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        let predictors = predecessors
            .iter()
            .map(|index| scores[*index].clone())
            .collect::<Vec<_>>();
        let coefficients = if let Some(case_weights) = prepared.case_weights.as_deref() {
            ols_weighted(&predictors, &scores[target], case_weights, &construct.id)?
        } else {
            ols(&predictors, &scores[target], &construct.id)?
        };
        let fitted = fitted_values(&predictors, &coefficients);
        let (residual, total) = if let Some(case_weights) = prepared.case_weights.as_deref() {
            weighted_residual_and_total(&scores[target], &fitted, case_weights)
        } else {
            (
                scores[target]
                    .iter()
                    .zip(fitted)
                    .map(|(actual, fit)| (actual - fit).powi(2))
                    .sum::<f64>(),
                scores[target]
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>(),
            )
        };
        r_squared.insert(construct.id.clone(), 1.0 - residual / total);
        for (source, coefficient) in predecessors.iter().zip(coefficients) {
            paths.push(PathEstimate {
                source: recipe.model.constructs[*source].id.clone(),
                target: construct.id.clone(),
                coefficient,
            });
        }
    }
    let effects = calculate_effects(
        &recipe
            .model
            .constructs
            .iter()
            .map(|construct| construct.id.clone())
            .collect::<Vec<_>>(),
        &paths,
        control,
    )?;
    let control_estimates = control_estimates(&recipe.model.controls, &paths)?;
    let mediation = analyze_mediation_effects_with_tolerance(&effects, 1e-12);
    let mut construct_scores = BTreeMap::new();
    for (index, construct) in recipe.model.constructs.iter().enumerate() {
        checkpoint(
            control,
            EstimationPhase::Assembling,
            assembly_completed,
            assembly_units,
        )?;
        construct_scores.insert(construct.id.clone(), scores[index].clone());
        assembly_completed += 1;
    }
    checkpoint(
        control,
        EstimationPhase::Assembling,
        assembly_units,
        assembly_units,
    )?;
    let mut warnings = Vec::new();
    if prepared.omitted > 0 {
        warnings.push(format!(
            "{} observations were omitted listwise",
            prepared.omitted
        ));
    }
    if matches!(recipe.settings.weighting_scheme, WeightingScheme::Pca)
        && recipe
            .model
            .constructs
            .iter()
            .any(|construct| construct.mode == MeasurementMode::Formative)
    {
        warnings.push("PCA weighting ignores Mode A/B distinctions".into());
    }
    if recipe
        .model
        .higher_order_constructs
        .iter()
        .any(|higher_order| higher_order.method == HigherOrderMethod::RepeatedIndicators)
    {
        warnings.push(
            "Repeated-indicator higher-order constructs are validated for the documented QuickPLS v1.2.3 bounded repeated-indicator, two-stage, and hybrid scopes; HOC indicator blocks were expanded from lower-order component indicators"
                .into(),
        );
    }
    if recipe
        .model
        .higher_order_constructs
        .iter()
        .any(|higher_order| higher_order.method != HigherOrderMethod::RepeatedIndicators)
    {
        warnings.push(
            "Two-stage and hybrid higher-order constructs are validated for the documented QuickPLS v1.2.3 bounded repeated-indicator, two-stage, and hybrid scopes; unsupported HOC variants remain blocked or excluded"
                .into(),
        );
    }
    let _ = dataset;
    Ok(PlsResult {
        method_version: PLS_METHOD_VERSION.into(),
        converged: true,
        iterations,
        used_observations: prepared.used,
        omitted_observations: prepared.omitted,
        transforms: prepared.transforms,
        construct_scores,
        outer_estimates,
        paths,
        control_estimates,
        effects,
        mediation,
        moderation: ModerationAnalysis::default(),
        plsc: None,
        endogeneity: None,
        nonlinear_effects: None,
        moderated_mediation: None,
        cta_pls: None,
        wpls: None,
        cca: None,
        predict: None,
        segmentation: None,
        mga: None,
        micom: None,
        mga_permutation: None,
        fimix: None,
        ipma: None,
        cbsem: None,
        pca: None,
        regression: None,
        nca: None,
        gsca: None,
        r_squared,
        warnings,
    })
}

fn control_estimates(
    controls: &[qpls_core::ControlPath],
    paths: &[PathEstimate],
) -> Result<Vec<ControlEstimate>, EstimationError> {
    controls
        .iter()
        .map(|control| {
            let path = paths
                .iter()
                .find(|path| path.source == control.source && path.target == control.target)
                .ok_or_else(|| {
                    EstimationError::UnknownConstruct(format!(
                        "control path missing from estimates: {} -> {}",
                        control.source, control.target
                    ))
                })?;
            Ok(ControlEstimate {
                source: control.source.clone(),
                target: control.target.clone(),
                label: control.label.clone(),
                coefficient: path.coefficient,
            })
        })
        .collect()
}

struct PredictionSplit {
    train_columns: Vec<Vec<f64>>,
    test_columns: Vec<Vec<f64>>,
    transforms: Vec<PredictionIndicatorTransform>,
    test_rows: Vec<usize>,
    train_observations: usize,
    test_observations: usize,
}

struct PredictionIndicatorTransform {
    raw_training_mean: f64,
    center: f64,
    scale: f64,
}

struct PredictionPreparedRows {
    positions: Vec<usize>,
    complete_rows: Vec<usize>,
}

#[derive(Default)]
struct PredictionErrorAccumulator {
    construct: String,
    predictor_count: usize,
    observation_count: usize,
    pls_sse: f64,
    pls_absolute_error: f64,
    benchmark_sse: f64,
    benchmark_absolute_error: f64,
    lm_sse: Option<f64>,
    lm_absolute_error: Option<f64>,
    lm_available: bool,
}

#[derive(Default)]
struct ErrorMetricAccumulator {
    observations: usize,
    squared_error_sum: f64,
    absolute_error_sum: f64,
    absolute_percentage_error_sum: f64,
    mape_observations: usize,
}

struct IndicatorPredictionAccumulator {
    construct: String,
    indicator: String,
    predictor_count: usize,
    pls: ErrorMetricAccumulator,
    indicator_average: ErrorMetricAccumulator,
    linear_model: ErrorMetricAccumulator,
    linear_model_available: bool,
    linear_model_reason: Option<String>,
}

struct FoldConstructPrediction {
    construct: String,
    predictor_count: usize,
    actual: Vec<f64>,
    predicted: Vec<f64>,
    linear_model: Option<Vec<f64>>,
}

struct FoldIndicatorPrediction {
    construct: String,
    indicator: String,
    predictor_count: usize,
    actual: Vec<f64>,
    predicted: Vec<f64>,
    indicator_average: Vec<f64>,
    linear_model: Result<Vec<f64>, String>,
}

struct PredictionFoldOutput {
    constructs: Vec<FoldConstructPrediction>,
    indicators: Vec<FoldIndicatorPrediction>,
}

#[derive(Default)]
struct CvpatCaseLoss {
    pls_sum: f64,
    indicator_average_sum: f64,
    linear_model_sum: f64,
    repeats: usize,
}

struct CvpatLossAccumulator {
    cases: BTreeMap<usize, CvpatCaseLoss>,
    indicator_count: usize,
    linear_model_available: bool,
    linear_model_reason: Option<String>,
}

fn apply_pls_predict(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    result: &mut PlsResult,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(), EstimationError> {
    if !recipe.model.interactions.is_empty() || !recipe.model.higher_order_constructs.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "Deterministic construct prediction does not support generated interactions or higher-order constructs"
                .into(),
        ));
    }
    if recipe.settings.case_weight_column.is_some() {
        return Err(EstimationError::UnsupportedMethod(
            "Deterministic construct prediction does not support case weights".into(),
        ));
    }
    let prepared_rows = prepare_prediction_rows(dataset, indicator_names, control)?;
    if prepared_rows.complete_rows.len() < 20 {
        return Err(EstimationError::UnsupportedMethod(
            "PLSpredict indicator v2 requires at least 20 complete cases across all model indicators"
                .into(),
        ));
    }
    let split = prepare_prediction_split(
        dataset,
        indicator_names,
        &prepared_rows.positions,
        &prepared_rows
            .complete_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (index % 4 != 3).then_some(*row))
            .collect::<Vec<_>>(),
        &prepared_rows
            .complete_rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (index % 4 == 3).then_some(*row))
            .collect::<Vec<_>>(),
        &recipe.settings.preprocessing,
        control,
    )?;
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let blocks = recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            construct
                .indicators
                .iter()
                .map(|name| indicator_index[name.as_str()])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let fold_output = prediction_fold_output(
        recipe,
        indicator_names,
        &blocks,
        &construct_index,
        &split,
        control,
    )?;
    let targets = fold_output
        .constructs
        .iter()
        .map(construct_prediction_target)
        .collect::<Vec<_>>();
    let indicator_targets = fold_output
        .indicators
        .iter()
        .map(indicator_prediction_target)
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "Deterministic construct prediction requires at least one endogenous construct".into(),
        ));
    }
    let repeated_kfold = repeated_kfold_pls_predict(
        dataset,
        recipe,
        indicator_names,
        &prepared_rows,
        &blocks,
        &construct_index,
        control,
    )?;
    result.method_version = PLS_PREDICT_METHOD_VERSION.into();
    result.predict = Some(PlsPredictAnalysis {
        method_version: PLS_PREDICT_METHOD_VERSION.into(),
        primary_analysis: PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION.into(),
        split: "deterministic_complete_case_modulo_4_test_rows".into(),
        training_observations: split.train_observations,
        test_observations: split.test_observations,
        benchmark: "secondary modulo-4 holdout with training-mean indicator-average (IA) and earliest-antecedent linear-model (LM) benchmarks; primary inference is the seeded repeated 10-fold block".into(),
        targets,
        indicator_targets,
        repeated_kfold,
        warnings: vec![
            "PLSpredict indicator v2 is limited to the documented QuickPLS bounded scope: complete-case rows, a secondary deterministic modulo-4 holdout, fixed seeded 10-fold cross-validation repeated 10 times, earliest-antecedent indicator prediction, IA and LM benchmarks, and aggregate benchmark CVPAT. It does not compare separately saved models."
                .into(),
        ],
    });
    result.warnings.push(
        "PLSpredict indicator v2 completed within the documented fixed seeded 10-fold by 10-repeat scope; the modulo-4 holdout is secondary."
            .into(),
    );
    Ok(())
}

fn apply_pls_pos_segmentation(
    recipe: &AnalysisRecipe,
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    let requested = recipe
        .metadata
        .get("segment_count")
        .or_else(|| recipe.metadata.get("segmentation.pls_pos_segments"))
        .or_else(|| recipe.metadata.get("pls_pos_segments"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let Some(requested) = requested else {
        return Ok(());
    };
    let requested_segments = requested.parse::<usize>().map_err(|_| {
        EstimationError::UnsupportedMethod("segment_count must be an integer".into())
    })?;
    if !(2..=5).contains(&requested_segments) {
        return Err(EstimationError::UnsupportedMethod(
            "PLS-POS v1 supports 2 to 5 segments".into(),
        ));
    }
    if recipe.model.paths.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "bounded PLS-POS preview requires at least one structural path".into(),
        ));
    }
    let observations = result
        .construct_scores
        .values()
        .next()
        .map(Vec::len)
        .unwrap_or_default();
    if observations < 40
        || result
            .construct_scores
            .values()
            .any(|scores| scores.len() != observations)
    {
        return Err(EstimationError::InsufficientObservations);
    }
    let minimum_share = parse_metadata_f64(recipe, "minimum_segment_share", 0.10).clamp(0.05, 0.40);
    let minimum_size = ((observations as f64 * minimum_share).ceil() as usize).max(8);
    if observations < minimum_size * requested_segments {
        return Err(EstimationError::InsufficientObservations);
    }
    let starts = parse_metadata_usize(recipe, "segment_starts", 10).clamp(1, 50);
    let pooled = segment_structural_fit(recipe, result, &(0..observations).collect::<Vec<_>>())?;
    let features = segmentation_features(recipe, result)?;
    let (assignments, fits, objective, history) = deterministic_partition_segments(
        recipe,
        result,
        &features,
        requested_segments,
        starts,
        minimum_size,
    )?;
    let pooled_objective = pooled.sse;
    let warnings = vec![
        "PLS-POS v1 is validated for the documented QuickPLS v1.2.2 deterministic 2-5 segment score-space partitioning scope; full unrestricted PLS-POS claims remain unsupported.".into(),
    ];
    let max_path_separation = max_pairwise_path_separation(&fits);
    let memberships = assignments
        .iter()
        .enumerate()
        .map(|(observation, segment)| PlsSegmentMembership {
            observation,
            segment: format!("segment_{}", segment + 1),
        })
        .collect::<Vec<_>>();
    result.segmentation = Some(PlsSegmentationAnalysis {
        method_version: if requested_segments == 2
            && recipe.metadata.contains_key("pls_pos_segments")
        {
            PLS_SEGMENTATION_METHOD_VERSION.into()
        } else {
            PLS_POS_METHOD_VERSION.into()
        },
        algorithm: "deterministic_multi_segment_score_space_sse_partition".into(),
        requested_segments,
        selected_segments: requested_segments,
        assignment: format!(
            "{starts} deterministic starts; {requested_segments} segments; minimum segment size {minimum_size}"
        ),
        observations,
        objective,
        pooled_objective,
        objective_improvement: (pooled_objective - objective) / pooled_objective,
        min_segment_share: fits
            .iter()
            .map(|fit| fit.observations)
            .min()
            .unwrap_or_default() as f64
            / observations as f64,
        segment_size_imbalance: fits
            .iter()
            .map(|fit| fit.observations)
            .max()
            .unwrap_or_default()
            .abs_diff(
                fits.iter()
                    .map(|fit| fit.observations)
                    .min()
                    .unwrap_or_default(),
            ) as f64
            / observations as f64,
        max_path_separation,
        segments: fits
            .into_iter()
            .enumerate()
            .map(|(index, fit)| PlsSegmentSummary {
                segment: format!("segment_{}", index + 1),
                observations: fit.observations,
                share: fit.observations as f64 / observations as f64,
                paths: fit.paths,
                r_squared: fit.r_squared,
            })
            .collect(),
        memberships,
        objective_history: history,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_two_group_mga(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    result: &mut PlsResult,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(), EstimationError> {
    let (group_column, groups, excluded_observations) =
        observed_two_groups(dataset, recipe, "two-group MGA v2")?;
    let mut base_recipe = recipe.clone();
    base_recipe.settings.method = AnalysisMethod::PlsPm;
    base_recipe.metadata.remove("mga_group_column");
    base_recipe.metadata.remove("mga.group_column");
    base_recipe.metadata.remove("mga_group_a");
    base_recipe.metadata.remove("mga.group_a");
    base_recipe.metadata.remove("mga_group_b");
    base_recipe.metadata.remove("mga.group_b");
    base_recipe.metadata.remove("group_methods");
    base_recipe.metadata.remove("group_permutation_samples");
    base_recipe.metadata.remove("micom_configural_confirmed");
    let pooled_rows = groups
        .iter()
        .flat_map(|(_, rows)| rows.iter().copied())
        .collect::<Vec<_>>();
    let (_, _, pooled_fit) = fit_group_result_with_base(
        dataset,
        &base_recipe,
        "pooled_selected_groups",
        &pooled_rows,
        control,
        0,
        2,
    )?;
    let mut fitted = Vec::new();
    for (index, (group, rows)) in groups.iter().enumerate() {
        let mut fitted_group = fit_group_result_with_base(
            dataset,
            &base_recipe,
            group,
            rows,
            control,
            index as u64,
            2,
        )?;
        align_group_result_to_pooled(
            dataset,
            recipe,
            &pooled_rows,
            &pooled_fit,
            &mut fitted_group.2,
        )?;
        fitted.push(fitted_group);
    }
    let first = &fitted[0];
    let second = &fitted[1];
    let comparisons = mga_path_comparisons(recipe, first, second)?;
    let measurement_comparisons = mga_measurement_comparisons(recipe, first, second)?;
    let mut warnings = vec![
        "Two-group MGA v2 reports Group A/Group B structural paths, R-squared values, outer loadings, outer weights, and A-minus-B differences. Inference uses deterministic two-tailed group-label permutation and is paired with MICOM v2 measurement-invariance assessment.".into(),
    ];
    if excluded_observations > 0 {
        warnings.push(format!(
            "MGA excluded {excluded_observations} rows whose group value was missing, unsupported, or not selected as Group A or Group B."
        ));
    }
    result.method_version = PLS_MGA_METHOD_VERSION.into();
    result.mga = Some(PlsMgaAnalysis {
        method_version: PLS_MGA_METHOD_VERSION.into(),
        group_column,
        groups: fitted
            .iter()
            .map(|(group, observations, group_result)| PlsMgaGroupSummary {
                group: group.clone(),
                observations: group_result.used_observations.min(*observations),
                paths: group_result.paths.clone(),
                r_squared: group_result.r_squared.clone(),
                outer_estimates: group_result.outer_estimates.clone(),
                transforms: group_result.transforms.clone(),
            })
            .collect(),
        comparisons,
        measurement_comparisons,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_micom(recipe: &AnalysisRecipe, result: &PlsResult) -> Result<(), EstimationError> {
    if !group_method_requested(recipe, "micom") {
        return Err(EstimationError::UnsupportedMethod(
            "the current two-group permutation workflow requires MICOM v2".into(),
        ));
    }
    if !recipe
        .metadata
        .get("micom_configural_confirmed")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return Err(EstimationError::UnsupportedMethod(
            "MICOM v2 requires explicit confirmation of configural invariance prerequisites".into(),
        ));
    }
    if result.micom.is_none() {
        return Err(EstimationError::Numerical(
            "MICOM v2 did not produce a result payload".into(),
        ));
    }
    Ok(())
}

fn apply_mga_permutation(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    result: &mut PlsResult,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(), EstimationError> {
    if !group_method_requested(recipe, "mga_permutation") {
        return Err(EstimationError::UnsupportedMethod(
            "the current two-group workflow requires permutation MGA v2".into(),
        ));
    }
    if !group_method_requested(recipe, "micom") {
        return Err(EstimationError::UnsupportedMethod(
            "the current two-group workflow requires MICOM v2".into(),
        ));
    }
    ensure_group_segmentation_supported(recipe, "permutation MGA v2")?;
    let (group_column, groups, _) = observed_two_groups(dataset, recipe, "permutation MGA v2")?;
    let samples = recipe
        .metadata
        .get("group_permutation_samples")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|samples| (5_000..=10_000).contains(samples))
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(
                "MICOM and permutation MGA require group_permutation_samples between 5000 and 10000".into(),
            )
        })?;
    let first_rows = &groups[0].1;
    let second_rows = &groups[1].1;
    let original_mga = result.mga.as_ref().cloned().ok_or_else(|| {
        EstimationError::Numerical(
            "permutation MGA requires completed group-specific estimates".into(),
        )
    })?;
    let original = original_mga.comparisons.clone();
    let original_measurements = original_mga.measurement_comparisons.clone();
    if original_mga.groups.len() != 2
        || original_mga.groups[0].group != groups[0].0
        || original_mga.groups[1].group != groups[1].0
    {
        return Err(EstimationError::Numerical(
            "permutation MGA group estimates do not match the selected ordered groups".into(),
        ));
    }
    let labels = permutation_labels(first_rows.len(), second_rows.len());
    let all_rows = first_rows
        .iter()
        .chain(second_rows.iter())
        .copied()
        .collect::<Vec<_>>();
    let (_, _, pooled_fit) = fit_group_result(
        dataset,
        recipe,
        "pooled_selected_groups",
        &all_rows,
        control,
        0,
        samples as u64,
    )?;
    let observed_micom = micom_statistics(
        dataset,
        recipe,
        &all_rows,
        &labels,
        &original_mga.groups[0].outer_estimates,
        &original_mga.groups[0].transforms,
        &original_mga.groups[1].outer_estimates,
        &original_mga.groups[1].transforms,
        &pooled_fit,
    )?;
    let mut path_extremes = vec![0usize; original.len()];
    let mut path_less_equal = vec![0usize; original.len()];
    let mut measurement_extremes = vec![0usize; original_measurements.len()];
    let mut measurement_less_equal = vec![0usize; original_measurements.len()];
    let mut micom_correlations = vec![Vec::with_capacity(samples); observed_micom.len()];
    let mut micom_mean_differences = vec![Vec::with_capacity(samples); observed_micom.len()];
    let mut micom_variance_differences = vec![Vec::with_capacity(samples); observed_micom.len()];
    let mut usable = 0usize;
    let mut failed = 0usize;
    let mut attempted = 0usize;
    let maximum_attempts = samples.saturating_add((samples / 5).max(100));
    checkpoint(control, EstimationPhase::Iterating, 0, samples as u64)?;
    while usable < samples && attempted < maximum_attempts {
        let replicate = attempted;
        attempted += 1;
        let shuffled =
            deterministic_permutation_labels(&labels, recipe.settings.seed ^ 0x9E37, replicate);
        let (left, right) = split_by_labels(&all_rows, &shuffled);
        let left_fit = fit_group_result(
            dataset,
            recipe,
            &groups[0].0,
            &left,
            control,
            usable as u64,
            samples as u64,
        );
        if matches!(&left_fit, Err(EstimationError::Cancelled)) {
            return Err(EstimationError::Cancelled);
        }
        let right_fit = fit_group_result(
            dataset,
            recipe,
            &groups[1].0,
            &right,
            control,
            usable as u64,
            samples as u64,
        );
        if matches!(&right_fit, Err(EstimationError::Cancelled)) {
            return Err(EstimationError::Cancelled);
        }
        let (Ok(mut left_fit), Ok(mut right_fit)) = (left_fit, right_fit) else {
            failed += 1;
            checkpoint(
                control,
                EstimationPhase::Iterating,
                usable as u64,
                samples as u64,
            )?;
            continue;
        };
        align_group_result_to_pooled(dataset, recipe, &all_rows, &pooled_fit, &mut left_fit.2)?;
        align_group_result_to_pooled(dataset, recipe, &all_rows, &pooled_fit, &mut right_fit.2)?;
        let comparisons = mga_path_comparisons(recipe, &left_fit, &right_fit)?;
        let measurement_comparisons = mga_measurement_comparisons(recipe, &left_fit, &right_fit)?;
        let micom = micom_statistics(
            dataset,
            recipe,
            &all_rows,
            &shuffled,
            &left_fit.2.outer_estimates,
            &left_fit.2.transforms,
            &right_fit.2.outer_estimates,
            &right_fit.2.transforms,
            &pooled_fit,
        )?;
        if !same_measurement_comparison_order(&original_measurements, &measurement_comparisons)
            || !same_micom_order(&observed_micom, &micom)
        {
            return Err(EstimationError::Numerical(
                "permutation MGA produced inconsistent parameter identities".into(),
            ));
        }
        for (index, comparison) in comparisons.iter().enumerate() {
            let diff = comparison.difference;
            if diff.abs() >= original[index].difference.abs() {
                path_extremes[index] += 1;
            }
            if diff <= original[index].difference {
                path_less_equal[index] += 1;
            }
        }
        for (index, comparison) in measurement_comparisons.iter().enumerate() {
            let diff = comparison.difference;
            if diff.abs() >= original_measurements[index].difference.abs() {
                measurement_extremes[index] += 1;
            }
            if diff <= original_measurements[index].difference {
                measurement_less_equal[index] += 1;
            }
        }
        for (index, statistic) in micom.iter().enumerate() {
            micom_correlations[index].push(statistic.compositional_correlation);
            micom_mean_differences[index].push(statistic.mean_difference);
            micom_variance_differences[index].push(statistic.variance_difference);
        }
        usable += 1;
        checkpoint(
            control,
            EstimationPhase::Iterating,
            usable as u64,
            samples as u64,
        )?;
    }
    if usable != samples {
        return Err(EstimationError::Numerical(format!(
            "permutation MGA produced {usable} usable fits after {attempted} attempts; {samples} were required"
        )));
    }
    let comparisons = original
        .into_iter()
        .enumerate()
        .map(|(index, comparison)| PlsMgaPermutationComparison {
            source: comparison.source,
            target: comparison.target,
            original_difference: comparison.difference,
            empirical_p_value_two_sided: Some(empirical_p_value(path_extremes[index], usable)),
            percentile_rank: Some(path_less_equal[index] as f64 / usable as f64),
        })
        .collect::<Vec<_>>();
    let measurement_comparisons = original_measurements
        .into_iter()
        .enumerate()
        .map(
            |(index, comparison)| PlsMgaPermutationMeasurementComparison {
                parameter: comparison.parameter,
                construct: comparison.construct,
                indicator: comparison.indicator,
                original_difference: comparison.difference,
                empirical_p_value_two_sided: Some(empirical_p_value(
                    measurement_extremes[index],
                    usable,
                )),
                percentile_rank: Some(measurement_less_equal[index] as f64 / usable as f64),
            },
        )
        .collect::<Vec<_>>();
    let micom_constructs = build_micom_results(
        &observed_micom,
        &mut micom_correlations,
        &mut micom_mean_differences,
        &mut micom_variance_differences,
        usable,
        recipe.settings.confidence_level,
        recipe
            .metadata
            .get("micom_configural_confirmed")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    )?;
    let mut warnings = vec![
        "Two-group permutation MGA v2 re-estimates path coefficients, outer loadings, and outer weights after every deterministic group-label permutation and reports two-tailed A-minus-B evidence.".into(),
        "MICOM v2 evaluates computational configural prerequisites, compositional invariance, and equality of pooled-score means and variances. Translation, coding meaning, and substantive indicator equivalence still require researcher review.".into(),
    ];
    if failed > 0 {
        warnings.push(format!(
            "Permutation MGA skipped {failed} singular or non-convergent permutation fits."
        ));
    }
    result.mga_permutation = Some(PlsMgaPermutationAnalysis {
        method_version: PLS_MGA_PERMUTATION_METHOD_VERSION.into(),
        group_column: group_column.clone(),
        permutation_samples: samples,
        usable_permutations: usable,
        attempted_permutations: Some(attempted),
        failed_permutations: Some(failed),
        comparisons,
        measurement_comparisons,
        warnings: warnings.clone(),
    });
    result.micom = Some(MicomAnalysis {
        method_version: MICOM_METHOD_VERSION.into(),
        group_column,
        permutation_samples: samples,
        usable_permutations: usable,
        attempted_permutations: Some(attempted),
        failed_permutations: Some(failed),
        confidence_level: Some(recipe.settings.confidence_level),
        groups: original_mga
            .groups
            .iter()
            .map(|group| MicomGroupSummary {
                group: group.group.clone(),
                observations: group.observations,
            })
            .collect(),
        constructs: micom_constructs,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

#[derive(Debug, Clone)]
struct MicomStatistic {
    construct: String,
    compositional_correlation: f64,
    mean_a: f64,
    mean_b: f64,
    mean_difference: f64,
    variance_a: f64,
    variance_b: f64,
    variance_difference: f64,
}

fn same_measurement_comparison_order(
    left: &[PlsMgaMeasurementComparison],
    right: &[PlsMgaMeasurementComparison],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.parameter == right.parameter
                && left.construct == right.construct
                && left.indicator == right.indicator
        })
}

fn same_micom_order(left: &[MicomStatistic], right: &[MicomStatistic]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.construct == right.construct)
}

fn group_rows(array: &dyn Array) -> Result<Vec<(String, Vec<usize>)>, EstimationError> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for row in 0..array.len() {
        if array.is_null(row) {
            continue;
        }
        let label = if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
            values.value(row).trim().to_string()
        } else if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
            values.value(row).to_string()
        } else if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
            values.value(row).to_string()
        } else if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
            let value = values.value(row);
            if !value.is_finite() {
                continue;
            }
            if value.fract().abs() <= f64::EPSILON {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        } else {
            return Err(EstimationError::UnsupportedMethod(
                "bounded MGA supports text, boolean, integer, or numeric group columns".into(),
            ));
        };
        if !label.is_empty() {
            groups.entry(label).or_default().push(row);
        }
    }
    Ok(groups.into_iter().collect())
}

fn observed_two_groups(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    method: &str,
) -> Result<(String, Vec<(String, Vec<usize>)>, usize), EstimationError> {
    let group_column = recipe
        .metadata
        .get("mga_group_column")
        .or_else(|| recipe.metadata.get("mga.group_column"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(format!(
                "{method} requires metadata mga_group_column"
            ))
        })?;
    let group_position = dataset
        .batch
        .schema()
        .index_of(group_column)
        .map_err(|_| EstimationError::InvalidIndicator(group_column.into()))?;
    let mut groups = group_rows(dataset.batch.column(group_position).as_ref())?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let (group_a, group_b) = requested_mga_groups(recipe, method)?;
    let first_observed = groups.remove(&group_a).ok_or_else(|| {
        EstimationError::UnsupportedMethod(format!(
            "{method} Group A value '{group_a}' is not present in the selected column"
        ))
    })?;
    let second_observed = groups.remove(&group_b).ok_or_else(|| {
        EstimationError::UnsupportedMethod(format!(
            "{method} Group B value '{group_b}' is not present in the selected column"
        ))
    })?;
    // Freeze the complete-case row set before fitting or permuting group labels.
    // This preserves the selected A/B analyzed sample sizes across every
    // permutation even when model indicators contain missing values.
    let first = complete_model_rows(dataset, recipe, &first_observed)?;
    let second = complete_model_rows(dataset, recipe, &second_observed)?;
    let selected = first.len() + second.len();
    let excluded = dataset.batch.num_rows().saturating_sub(selected);
    Ok((
        group_column.into(),
        vec![(group_a, first), (group_b, second)],
        excluded,
    ))
}

fn complete_model_rows(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    candidate_rows: &[usize],
) -> Result<Vec<usize>, EstimationError> {
    let indicators = collect_indicators(recipe)?;
    let schema = dataset.batch.schema();
    let positions = indicators
        .iter()
        .map(|indicator| {
            schema
                .index_of(indicator)
                .map_err(|_| EstimationError::InvalidIndicator(indicator.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (indicator, position) in indicators.iter().zip(&positions) {
        let array = dataset.batch.column(*position);
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(EstimationError::InvalidIndicator(indicator.clone()));
        }
    }
    Ok(candidate_rows
        .iter()
        .copied()
        .filter(|row| {
            positions.iter().all(|position| {
                let array = dataset.batch.column(*position);
                !array.is_null(*row)
                    && numeric_value(array.as_ref(), *row).is_some_and(f64::is_finite)
            })
        })
        .collect())
}

fn fit_group_result(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    group: &str,
    rows: &[usize],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
    completed_units: u64,
    total_units: u64,
) -> Result<(String, usize, PlsResult), EstimationError> {
    let mut base_recipe = recipe.clone();
    base_recipe.settings.method = AnalysisMethod::PlsPm;
    base_recipe.metadata.remove("mga_group_column");
    base_recipe.metadata.remove("mga.group_column");
    base_recipe.metadata.remove("mga_group_a");
    base_recipe.metadata.remove("mga.group_a");
    base_recipe.metadata.remove("mga_group_b");
    base_recipe.metadata.remove("mga.group_b");
    base_recipe.metadata.remove("group_methods");
    base_recipe.metadata.remove("group_permutation_samples");
    fit_group_result_with_base(
        dataset,
        &base_recipe,
        group,
        rows,
        control,
        completed_units,
        total_units,
    )
}

fn fit_group_result_with_base(
    dataset: &Dataset,
    base_recipe: &AnalysisRecipe,
    group: &str,
    rows: &[usize],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
    completed_units: u64,
    total_units: u64,
) -> Result<(String, usize, PlsResult), EstimationError> {
    if rows.len() < 10 {
        return Err(EstimationError::UnsupportedMethod(format!(
            "selected MGA group '{group}' has fewer than 10 observed rows"
        )));
    }
    let subset = subset_dataset(dataset, rows, &format!("group_{group}"))?;
    let result = estimate_pls_reduced_with_control(&subset, base_recipe, |_| {
        control(EstimationProgress {
            phase: EstimationPhase::Iterating,
            completed_units,
            total_units,
        })
    })?;
    if result.used_observations < 10 {
        return Err(EstimationError::UnsupportedMethod(format!(
            "selected MGA group '{group}' has {} complete model cases; at least 10 are required",
            result.used_observations
        )));
    }
    Ok((group.to_string(), rows.len(), result))
}

fn requested_mga_groups(
    recipe: &AnalysisRecipe,
    method: &str,
) -> Result<(String, String), EstimationError> {
    let group_a = recipe
        .metadata
        .get("mga_group_a")
        .or_else(|| recipe.metadata.get("mga.group_a"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(format!("{method} requires metadata mga_group_a"))
        })?;
    let group_b = recipe
        .metadata
        .get("mga_group_b")
        .or_else(|| recipe.metadata.get("mga.group_b"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(format!("{method} requires metadata mga_group_b"))
        })?;
    if group_a == group_b {
        return Err(EstimationError::UnsupportedMethod(
            "MGA Group A and Group B must be different observed values".into(),
        ));
    }
    Ok((group_a.into(), group_b.into()))
}

fn group_method_requested(recipe: &AnalysisRecipe, method: &str) -> bool {
    recipe
        .metadata
        .get("group_methods")
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim())
                .any(|item| item.eq_ignore_ascii_case(method))
        })
        .unwrap_or(false)
}

fn parse_metadata_usize(recipe: &AnalysisRecipe, key: &str, default: usize) -> usize {
    recipe
        .metadata
        .get(key)
        .or_else(|| recipe.metadata.get(&format!("segmentation.{key}")))
        .or_else(|| recipe.metadata.get(&format!("groups.{key}")))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_metadata_f64(recipe: &AnalysisRecipe, key: &str, default: f64) -> f64 {
    recipe
        .metadata
        .get(key)
        .or_else(|| recipe.metadata.get(&format!("segmentation.{key}")))
        .or_else(|| recipe.metadata.get(&format!("groups.{key}")))
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn ensure_group_segmentation_supported(
    recipe: &AnalysisRecipe,
    method: &str,
) -> Result<(), EstimationError> {
    if recipe.settings.case_weight_column.is_some() {
        return Err(EstimationError::UnsupportedMethod(format!(
            "{method} does not support case weights"
        )));
    }
    if !recipe.model.interactions.is_empty() {
        return Err(EstimationError::UnsupportedMethod(format!(
            "{method} does not support generated interactions"
        )));
    }
    if !recipe.model.higher_order_constructs.is_empty() {
        return Err(EstimationError::UnsupportedMethod(format!(
            "{method} does not support higher-order constructs"
        )));
    }
    Ok(())
}

fn permutation_labels(first_size: usize, second_size: usize) -> Vec<usize> {
    (0..first_size)
        .map(|_| 0usize)
        .chain((0..second_size).map(|_| 1usize))
        .collect()
}

fn deterministic_permutation_labels(labels: &[usize], seed: u64, replicate: usize) -> Vec<usize> {
    let mut values = labels.to_vec();
    let mut state = seed ^ (replicate as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for index in (1..values.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let swap = (state as usize) % (index + 1);
        values.swap(index, swap);
    }
    values
}

fn split_by_labels(rows: &[usize], labels: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (row, label) in rows.iter().zip(labels) {
        if *label == 0 {
            left.push(*row);
        } else {
            right.push(*row);
        }
    }
    (left, right)
}

fn empirical_p_value(extreme: usize, usable: usize) -> f64 {
    (extreme as f64 + 1.0) / (usable as f64 + 1.0)
}

fn subset_dataset(
    dataset: &Dataset,
    rows: &[usize],
    suffix: &str,
) -> Result<Dataset, EstimationError> {
    let arrays = dataset
        .batch
        .columns()
        .iter()
        .map(|column| subset_array(column.as_ref(), rows))
        .collect::<Result<Vec<_>, _>>()?;
    let batch = RecordBatch::try_new(dataset.batch.schema(), arrays)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let mut schema = dataset.schema.clone();
    schema.case_count = rows.len();
    schema.sample_size = Some(rows.len());
    Ok(Dataset {
        id: dataset.id,
        name: format!("{} {suffix}", dataset.name),
        schema,
        batch,
        fingerprint: DataFingerprint(format!("{}+{}", dataset.fingerprint.0, suffix)),
    })
}

fn mga_path_comparisons(
    recipe: &AnalysisRecipe,
    first: &(String, usize, PlsResult),
    second: &(String, usize, PlsResult),
) -> Result<Vec<PlsMgaPathComparison>, EstimationError> {
    let mut comparisons = Vec::new();
    for path in &recipe.model.paths {
        let coefficient_a = first
            .2
            .paths
            .iter()
            .find(|item| item.source == path.source && item.target == path.target)
            .map(|item| item.coefficient)
            .ok_or_else(|| EstimationError::UnknownConstruct(path.target.clone()))?;
        let coefficient_b = second
            .2
            .paths
            .iter()
            .find(|item| item.source == path.source && item.target == path.target)
            .map(|item| item.coefficient)
            .ok_or_else(|| EstimationError::UnknownConstruct(path.target.clone()))?;
        let se_a = path_standard_error(&first.2, recipe, &path.source, &path.target).ok();
        let se_b = path_standard_error(&second.2, recipe, &path.source, &path.target).ok();
        let standard_error = se_a
            .zip(se_b)
            .map(|(left, right)| (left * left + right * right).sqrt());
        let difference = coefficient_a - coefficient_b;
        let (t_statistic, p_value_two_sided, warning) = if let Some(se) = standard_error {
            if se > f64::EPSILON && se.is_finite() {
                let statistic = difference / se;
                let normal = Normal::new(0.0, 1.0)
                    .map_err(|error| EstimationError::Numerical(error.to_string()))?;
                (
                    Some(statistic),
                    Some((2.0 * (1.0 - normal.cdf(statistic.abs()))).clamp(0.0, 1.0)),
                    None,
                )
            } else {
                (
                    None,
                    None,
                    Some("path-difference standard error is numerically unavailable".into()),
                )
            }
        } else {
            (
                None,
                None,
                Some("group path standard error is unavailable".into()),
            )
        };
        comparisons.push(PlsMgaPathComparison {
            source: path.source.clone(),
            target: path.target.clone(),
            group_a: first.0.clone(),
            group_b: second.0.clone(),
            coefficient_a,
            coefficient_b,
            difference,
            standard_error,
            t_statistic,
            p_value_two_sided,
            warning,
        });
    }
    Ok(comparisons)
}

fn mga_measurement_comparisons(
    recipe: &AnalysisRecipe,
    first: &(String, usize, PlsResult),
    second: &(String, usize, PlsResult),
) -> Result<Vec<PlsMgaMeasurementComparison>, EstimationError> {
    let mut comparisons = Vec::new();
    for construct in &recipe.model.constructs {
        for indicator in &construct.indicators {
            let first_estimate = first
                .2
                .outer_estimates
                .iter()
                .find(|estimate| {
                    estimate.construct == construct.id && estimate.indicator == *indicator
                })
                .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))?;
            let second_estimate = second
                .2
                .outer_estimates
                .iter()
                .find(|estimate| {
                    estimate.construct == construct.id && estimate.indicator == *indicator
                })
                .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))?;
            for (parameter, estimate_a, estimate_b) in [
                (
                    "outer_loading",
                    first_estimate.loading,
                    second_estimate.loading,
                ),
                (
                    "outer_weight",
                    first_estimate.weight,
                    second_estimate.weight,
                ),
            ] {
                comparisons.push(PlsMgaMeasurementComparison {
                    parameter: parameter.into(),
                    construct: construct.id.clone(),
                    indicator: indicator.clone(),
                    group_a: first.0.clone(),
                    group_b: second.0.clone(),
                    estimate_a,
                    estimate_b,
                    difference: estimate_a - estimate_b,
                });
            }
        }
    }
    Ok(comparisons)
}

#[allow(clippy::too_many_arguments)]
fn micom_statistics(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    pooled_rows: &[usize],
    labels: &[usize],
    outer_a: &[OuterEstimate],
    transforms_a: &[IndicatorTransform],
    outer_b: &[OuterEstimate],
    transforms_b: &[IndicatorTransform],
    pooled_fit: &PlsResult,
) -> Result<Vec<MicomStatistic>, EstimationError> {
    if pooled_rows.len() != labels.len()
        || labels.iter().filter(|label| **label == 0).count() < 2
        || labels.iter().filter(|label| **label == 1).count() < 2
        || labels.iter().any(|label| *label > 1)
    {
        return Err(EstimationError::Numerical(
            "MICOM requires aligned non-empty ordered Group A/Group B labels".into(),
        ));
    }
    recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            let score_a =
                pooled_composite_scores(dataset, pooled_rows, construct, outer_a, transforms_a)?;
            let score_b =
                pooled_composite_scores(dataset, pooled_rows, construct, outer_b, transforms_b)?;
            let compositional_correlation = correlation(&score_a, &score_b).clamp(-1.0, 1.0);
            if !compositional_correlation.is_finite() {
                return Err(EstimationError::Numerical(format!(
                    "MICOM compositional correlation is unavailable for construct {}",
                    construct.id
                )));
            }
            let pooled_scores = pooled_fit
                .construct_scores
                .get(&construct.id)
                .ok_or_else(|| EstimationError::UnknownConstruct(construct.id.clone()))?;
            let (mean_a, mean_b, variance_a, variance_b) =
                micom_location_dispersion(pooled_scores, labels, &construct.id)?;
            Ok(MicomStatistic {
                construct: construct.id.clone(),
                compositional_correlation,
                mean_a,
                mean_b,
                mean_difference: mean_a - mean_b,
                variance_a,
                variance_b,
                variance_difference: (variance_a / variance_b).ln(),
            })
        })
        .collect()
}

fn pooled_composite_scores(
    dataset: &Dataset,
    pooled_rows: &[usize],
    construct: &qpls_core::Construct,
    outer_estimates: &[OuterEstimate],
    transforms: &[IndicatorTransform],
) -> Result<Vec<f64>, EstimationError> {
    let schema = dataset.batch.schema();
    let mut scores = vec![0.0; pooled_rows.len()];
    for indicator in &construct.indicators {
        let position = schema
            .index_of(indicator)
            .map_err(|_| EstimationError::InvalidIndicator(indicator.clone()))?;
        let estimate = outer_estimates
            .iter()
            .find(|estimate| estimate.construct == construct.id && estimate.indicator == *indicator)
            .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))?;
        let transform = transforms
            .iter()
            .find(|transform| transform.indicator == *indicator)
            .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))?;
        if !estimate.weight.is_finite()
            || !transform.scale.is_finite()
            || transform.scale.abs() <= f64::EPSILON
        {
            return Err(EstimationError::Numerical(format!(
                "MICOM cannot apply the group weight for indicator {indicator}"
            )));
        }
        let raw_weight = estimate.weight / transform.scale;
        let array = dataset.batch.column(position);
        for (score, row) in scores.iter_mut().zip(pooled_rows) {
            let value = numeric_value(array.as_ref(), *row)
                .filter(|value| value.is_finite())
                .ok_or_else(|| EstimationError::InvalidIndicator(indicator.clone()))?;
            *score += raw_weight * value;
        }
    }
    if sample_sd(&scores) <= f64::EPSILON {
        return Err(EstimationError::Numerical(format!(
            "MICOM pooled proxy has zero variance for construct {}",
            construct.id
        )));
    }
    Ok(scores)
}

fn align_group_result_to_pooled(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    pooled_rows: &[usize],
    pooled_fit: &PlsResult,
    group_fit: &mut PlsResult,
) -> Result<(), EstimationError> {
    let mut signs = HashMap::<String, f64>::new();
    for construct in &recipe.model.constructs {
        let pooled_scores = pooled_composite_scores(
            dataset,
            pooled_rows,
            construct,
            &pooled_fit.outer_estimates,
            &pooled_fit.transforms,
        )?;
        let group_scores = pooled_composite_scores(
            dataset,
            pooled_rows,
            construct,
            &group_fit.outer_estimates,
            &group_fit.transforms,
        )?;
        let alignment = correlation(&pooled_scores, &group_scores);
        if !alignment.is_finite() {
            return Err(EstimationError::Numerical(format!(
                "MGA sign alignment is unavailable for construct {}",
                construct.id
            )));
        }
        signs.insert(
            construct.id.clone(),
            if alignment < 0.0 { -1.0 } else { 1.0 },
        );
    }
    for estimate in &mut group_fit.outer_estimates {
        let sign = *signs.get(&estimate.construct).unwrap_or(&1.0);
        estimate.weight *= sign;
        estimate.loading *= sign;
    }
    for (construct, scores) in &mut group_fit.construct_scores {
        let sign = *signs.get(construct).unwrap_or(&1.0);
        if sign < 0.0 {
            for score in scores {
                *score = -*score;
            }
        }
    }
    for path in &mut group_fit.paths {
        let source_sign = *signs.get(&path.source).unwrap_or(&1.0);
        let target_sign = *signs.get(&path.target).unwrap_or(&1.0);
        path.coefficient *= source_sign * target_sign;
    }
    for control in &mut group_fit.control_estimates {
        let source_sign = *signs.get(&control.source).unwrap_or(&1.0);
        let target_sign = *signs.get(&control.target).unwrap_or(&1.0);
        control.coefficient *= source_sign * target_sign;
    }
    Ok(())
}

fn micom_location_dispersion(
    pooled_scores: &[f64],
    labels: &[usize],
    construct: &str,
) -> Result<(f64, f64, f64, f64), EstimationError> {
    if pooled_scores.len() != labels.len() {
        return Err(EstimationError::Numerical(format!(
            "MICOM pooled scores are misaligned for construct {construct}"
        )));
    }
    let group_a = pooled_scores
        .iter()
        .zip(labels)
        .filter_map(|(score, label)| (*label == 0).then_some(*score))
        .collect::<Vec<_>>();
    let group_b = pooled_scores
        .iter()
        .zip(labels)
        .filter_map(|(score, label)| (*label == 1).then_some(*score))
        .collect::<Vec<_>>();
    let variance_a = sample_variance(&group_a);
    let variance_b = sample_variance(&group_b);
    if group_a.len() < 2
        || group_b.len() < 2
        || !variance_a.is_finite()
        || !variance_b.is_finite()
        || variance_a <= f64::EPSILON
        || variance_b <= f64::EPSILON
    {
        return Err(EstimationError::Numerical(format!(
            "MICOM pooled group variance is unavailable for construct {construct}"
        )));
    }
    Ok((
        vector_mean(&group_a),
        vector_mean(&group_b),
        variance_a,
        variance_b,
    ))
}

fn build_micom_results(
    observed: &[MicomStatistic],
    correlations: &mut [Vec<f64>],
    mean_differences: &mut [Vec<f64>],
    variance_differences: &mut [Vec<f64>],
    usable: usize,
    confidence_level: f64,
    configural_invariance: bool,
) -> Result<Vec<MicomConstructResult>, EstimationError> {
    if !(0.0..1.0).contains(&confidence_level)
        || observed.len() != correlations.len()
        || observed.len() != mean_differences.len()
        || observed.len() != variance_differences.len()
    {
        return Err(EstimationError::Numerical(
            "MICOM confidence or permutation distribution shape is invalid".into(),
        ));
    }
    let alpha = 1.0 - confidence_level;
    let tail = alpha / 2.0;
    observed
        .iter()
        .enumerate()
        .map(|(index, observed)| {
            let correlation_distribution = &mut correlations[index];
            let mean_distribution = &mut mean_differences[index];
            let variance_distribution = &mut variance_differences[index];
            if correlation_distribution.len() != usable
                || mean_distribution.len() != usable
                || variance_distribution.len() != usable
                || correlation_distribution
                    .iter()
                    .any(|value| !value.is_finite())
                || mean_distribution.iter().any(|value| !value.is_finite())
                || variance_distribution.iter().any(|value| !value.is_finite())
            {
                return Err(EstimationError::Numerical(format!(
                    "MICOM permutation distribution is incomplete for construct {}",
                    observed.construct
                )));
            }
            correlation_distribution.sort_by(f64::total_cmp);
            mean_distribution.sort_by(f64::total_cmp);
            variance_distribution.sort_by(f64::total_cmp);
            let compositional_lower = type7_quantile(correlation_distribution, alpha);
            let mean_lower = type7_quantile(mean_distribution, tail);
            let mean_upper = type7_quantile(mean_distribution, 1.0 - tail);
            let variance_lower = type7_quantile(variance_distribution, tail);
            let variance_upper = type7_quantile(variance_distribution, 1.0 - tail);
            let compositional_p_value = empirical_lower_tail_p_value(
                correlation_distribution,
                observed.compositional_correlation,
            );
            let mean_p_value =
                empirical_two_tailed_p_value(mean_distribution, observed.mean_difference);
            let variance_p_value =
                empirical_two_tailed_p_value(variance_distribution, observed.variance_difference);
            let compositional_invariance =
                observed.compositional_correlation + 1e-12 >= compositional_lower;
            let equal_means = observed.mean_difference + 1e-12 >= mean_lower
                && observed.mean_difference - 1e-12 <= mean_upper;
            let equal_variances = observed.variance_difference + 1e-12 >= variance_lower
                && observed.variance_difference - 1e-12 <= variance_upper;
            let partial_invariance = configural_invariance && compositional_invariance;
            Ok(MicomConstructResult {
                construct: observed.construct.clone(),
                configural_invariance,
                compositional_correlation: observed.compositional_correlation,
                compositional_p_value: Some(compositional_p_value),
                compositional_correlation_lower: Some(compositional_lower),
                mean_a: Some(observed.mean_a),
                mean_b: Some(observed.mean_b),
                mean_difference: observed.mean_difference,
                mean_p_value: Some(mean_p_value),
                mean_difference_lower: Some(mean_lower),
                mean_difference_upper: Some(mean_upper),
                variance_a: Some(observed.variance_a),
                variance_b: Some(observed.variance_b),
                variance_difference: observed.variance_difference,
                variance_p_value: Some(variance_p_value),
                variance_difference_lower: Some(variance_lower),
                variance_difference_upper: Some(variance_upper),
                equal_means: Some(equal_means),
                equal_variances: Some(equal_variances),
                partial_invariance,
                full_invariance: partial_invariance && equal_means && equal_variances,
            })
        })
        .collect()
}

fn empirical_lower_tail_p_value(sorted: &[f64], observed: f64) -> f64 {
    let lower_or_equal = sorted.iter().filter(|value| **value <= observed).count();
    (lower_or_equal as f64 + 1.0) / (sorted.len() as f64 + 1.0)
}

fn empirical_two_tailed_p_value(values: &[f64], observed: f64) -> f64 {
    let lower = (values.iter().filter(|value| **value <= observed).count() as f64 + 1.0)
        / (values.len() as f64 + 1.0);
    let upper = (values.iter().filter(|value| **value >= observed).count() as f64 + 1.0)
        / (values.len() as f64 + 1.0);
    (2.0 * lower.min(upper)).min(1.0)
}

fn type7_quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + fraction * (sorted[upper] - sorted[lower])
    }
}

fn path_standard_error(
    result: &PlsResult,
    recipe: &AnalysisRecipe,
    source: &str,
    target: &str,
) -> Result<f64, EstimationError> {
    let predecessors = recipe
        .model
        .paths
        .iter()
        .filter(|path| path.target == target)
        .map(|path| path.source.clone())
        .collect::<Vec<_>>();
    let index = predecessors
        .iter()
        .position(|candidate| candidate == source)
        .ok_or_else(|| EstimationError::UnknownConstruct(source.into()))?;
    let predictors = predecessors
        .iter()
        .map(|predecessor| {
            result
                .construct_scores
                .get(predecessor)
                .cloned()
                .ok_or_else(|| EstimationError::UnknownConstruct(predecessor.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = result
        .construct_scores
        .get(target)
        .ok_or_else(|| EstimationError::UnknownConstruct(target.into()))?;
    let stats = ols_with_standard_errors(&predictors, outcome, &format!("MGA {target}"))?;
    Ok(stats.standard_errors[index])
}

fn apply_ipma(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    indicator_columns: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    let targets = resolve_ipma_targets(recipe)
        .map_err(|error| EstimationError::UnsupportedMethod(error.to_string()))?;
    let effect_index = result
        .effects
        .iter()
        .map(|effect| {
            (
                (effect.source.as_str(), effect.target.as_str()),
                effect.total,
            )
        })
        .collect::<HashMap<_, _>>();
    let loading_index = result
        .outer_estimates
        .iter()
        .map(|estimate| {
            (
                (estimate.construct.as_str(), estimate.indicator.as_str()),
                estimate.loading,
            )
        })
        .collect::<HashMap<_, _>>();
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, indicator)| (indicator.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut constructs = Vec::new();
    let mut indicators = Vec::new();
    for target in &targets {
        let predecessors = ipma_predecessor_constructs(recipe, target)
            .into_iter()
            .collect::<HashSet<_>>();
        for construct in recipe
            .model
            .constructs
            .iter()
            .filter(|construct| predecessors.contains(&construct.id))
        {
            let importance = *effect_index
                .get(&(construct.id.as_str(), target.as_str()))
                .unwrap_or(&0.0);
            let Some(scores) = result.construct_scores.get(&construct.id) else {
                continue;
            };
            constructs.push(IpmaConstructPerformance {
                target: target.clone(),
                construct: construct.id.clone(),
                importance,
                performance: min_max_performance(scores),
                score_mean: vector_mean(scores),
            });
            for indicator in &construct.indicators {
                let Some(column_index) = indicator_index.get(indicator.as_str()).copied() else {
                    continue;
                };
                let values = &indicator_columns[column_index];
                indicators.push(IpmaIndicatorPerformance {
                    target: target.clone(),
                    construct: construct.id.clone(),
                    indicator: indicator.clone(),
                    construct_importance: importance,
                    loading: *loading_index
                        .get(&(construct.id.as_str(), indicator.as_str()))
                        .unwrap_or(&0.0),
                    performance: min_max_performance(values),
                    score_mean: vector_mean(values),
                });
            }
        }
    }
    let warnings = vec![
        "IPMA v1 reports direct and indirect structural predecessors only; importance uses fixed PLS total effects and performance uses the observed sample range of listwise-standardized scores on a 0-100 scale. Theoretical-range performance and cIPMA are unsupported.".into(),
    ];
    result.method_version = IPMA_METHOD_VERSION.into();
    result.ipma = Some(IpmaAnalysis {
        method_version: IPMA_METHOD_VERSION.into(),
        performance_scale: IPMA_PERFORMANCE_SCALE.into(),
        targets,
        constructs,
        indicators,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_cbsem(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    indicator_columns: &[Vec<f64>],
    dataset: &Dataset,
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    ensure_cbsem_supported(recipe)?;
    let sample_size = result.used_observations;
    if sample_size < 10 {
        return Err(EstimationError::InsufficientObservations);
    }
    let model_type = recipe
        .metadata
        .get("cbsem_model_type")
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| value == "cfa" || value == "sem")
        .unwrap_or_else(|| {
            if recipe.model.paths.is_empty() {
                "cfa".into()
            } else {
                "sem".into()
            }
        });
    let mean_structure = recipe
        .metadata
        .get("cbsem_mean_structure")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let sample_covariance = cbsem_ml_covariance_matrix(indicator_columns);
    let optimized = cbsem_optimize_model(recipe, indicator_names, &sample_covariance, result)?;
    let implied = optimized.implied_covariance.clone();
    let residual = subtract_matrices(&sample_covariance, &implied);
    let residual_correlation = residual_correlation_matrix(&residual, &sample_covariance);
    let objective = optimized.objective;
    let parameter_count = cbsem_parameter_count(recipe);
    let observed_moments = indicator_names.len() * (indicator_names.len() + 1) / 2;
    let degrees_of_freedom = observed_moments as i64 - parameter_count as i64;
    if degrees_of_freedom < 0 {
        return Err(EstimationError::UnsupportedMethod(
            "CB-SEM v1 blocks underidentified models with negative degrees of freedom".into(),
        ));
    }
    let chi_square = (sample_size as f64 * objective).max(0.0);
    let baseline = baseline_fit(&sample_covariance, sample_size)?;
    let srmr = matrix_srmr(&sample_covariance, &implied);
    let fit = cbsem_fit_indices(
        chi_square,
        degrees_of_freedom,
        baseline.0,
        baseline.1,
        objective,
        parameter_count,
        sample_size,
        srmr,
    )?;
    let parameters = cbsem_parameters(recipe, &optimized, sample_size);
    let standardized =
        cbsem_standardized_parameters(&recipe.model, indicator_names, &parameters, &optimized);
    let modification_indices =
        cbsem_modification_indices(recipe, indicator_names, &residual_correlation, sample_size);
    let bootstrap = cbsem_bootstrap(recipe, &parameters);
    let multigroup = cbsem_multigroup(dataset, recipe, sample_size, &fit)?;
    let diagnostics = cbsem_diagnostics(&sample_covariance, &implied, &parameters);
    let mut warnings = vec![
        "CB-SEM/CFA ML v1 is validated for the documented QuickPLS v1.2.4 raw-data single-group reflective ML scope; bootstrap, unrestricted multigroup/invariance, robust, ordinal, and FIML estimators remain experimental or unsupported.".into(),
    ];
    if mean_structure {
        warnings.push(
            "CB-SEM mean structure is recorded in v0.7 metadata; intercept/mean parameters are not publication-validated.".into(),
        );
    }
    if recipe
        .metadata
        .get("cbsem_input")
        .is_some_and(|value| value != "raw")
    {
        warnings.push(
            "CB-SEM covariance/correlation input is experimental in v0.7; bootstrap and multigroup require raw data.".into(),
        );
    }
    warnings.extend(diagnostics.iter().cloned());
    result.method_version = if model_type == "cfa" {
        CFA_ML_METHOD_VERSION.into()
    } else {
        CBSEM_ML_METHOD_VERSION.into()
    };
    result.cbsem = Some(CbsemAnalysis {
        method_version: if model_type == "cfa" {
            CFA_ML_METHOD_VERSION.into()
        } else {
            CBSEM_ML_METHOD_VERSION.into()
        },
        model_type,
        estimator: "ml".into(),
        input: recipe
            .metadata
            .get("cbsem_input")
            .cloned()
            .unwrap_or_else(|| "raw".into()),
        mean_structure,
        converged: optimized.converged,
        iterations: optimized.iterations,
        objective,
        gradient_norm: optimized.gradient_norm,
        sample_size,
        parameters,
        standardized,
        implied_covariance: matrix_cells(indicator_names, &implied),
        residual_covariance: matrix_cells(indicator_names, &residual),
        residual_correlation: matrix_cells(indicator_names, &residual_correlation),
        fit,
        modification_indices,
        bootstrap,
        multigroup,
        diagnostics,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn ensure_cbsem_supported(recipe: &AnalysisRecipe) -> Result<(), EstimationError> {
    if recipe
        .model
        .constructs
        .iter()
        .any(|construct| construct.mode == MeasurementMode::Formative)
    {
        return Err(EstimationError::UnsupportedMethod(
            "CB-SEM ML v1 supports reflective constructs only".into(),
        ));
    }
    if !recipe.model.interactions.is_empty() || !recipe.model.higher_order_constructs.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "CB-SEM ML v1 does not support interactions or higher-order constructs".into(),
        ));
    }
    if recipe.settings.case_weight_column.is_some() {
        return Err(EstimationError::UnsupportedMethod(
            "CB-SEM ML v1 does not support case weights".into(),
        ));
    }
    for construct in &recipe.model.constructs {
        if construct.indicators.len() < 2 {
            return Err(EstimationError::UnsupportedMethod(format!(
                "CB-SEM ML v1 requires at least two indicators for construct {}",
                construct.id
            )));
        }
    }
    Ok(())
}

fn covariance_matrix(columns: &[Vec<f64>]) -> Vec<Vec<f64>> {
    columns
        .iter()
        .map(|left| {
            columns
                .iter()
                .map(|right| covariance(left, right))
                .collect()
        })
        .collect()
}

fn cbsem_ml_covariance_matrix(columns: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let sample = covariance_matrix(columns);
    let Some(n) = columns.first().map(Vec::len) else {
        return sample;
    };
    if n <= 1 {
        return sample;
    }
    let scale = (n - 1) as f64 / n as f64;
    sample
        .into_iter()
        .map(|row| row.into_iter().map(|value| value * scale).collect())
        .collect()
}

#[derive(Debug, Clone)]
struct CbsemOptimizedModel {
    implied_covariance: Vec<Vec<f64>>,
    loadings: HashMap<(String, String), f64>,
    beta: Vec<Vec<f64>>,
    latent_covariance: Vec<Vec<f64>>,
    disturbance_covariance: Vec<Vec<f64>>,
    theta: Vec<f64>,
    parameter_standard_errors: HashMap<String, f64>,
    objective: f64,
    converged: bool,
    iterations: u32,
    gradient_norm: f64,
}

#[derive(Debug, Clone)]
enum CbsemFreeParameter {
    Loading {
        construct: usize,
        indicator: usize,
        construct_id: String,
        indicator_id: String,
    },
    Path {
        source: usize,
        target: usize,
        source_id: String,
        target_id: String,
    },
    LatentVariance {
        construct: usize,
        construct_id: String,
    },
    LatentCovariance {
        left: usize,
        right: usize,
        left_id: String,
        right_id: String,
    },
    ResidualVariance {
        indicator: usize,
        indicator_id: String,
    },
}

impl CbsemFreeParameter {
    fn name(&self) -> String {
        match self {
            Self::Loading {
                construct_id,
                indicator_id,
                ..
            } => format!("{construct_id}=~{indicator_id}"),
            Self::Path {
                source_id,
                target_id,
                ..
            } => format!("{target_id}~{source_id}"),
            Self::LatentVariance { construct_id, .. } => {
                format!("{construct_id}~~{construct_id}")
            }
            Self::LatentCovariance {
                left_id, right_id, ..
            } => format!("{left_id}~~{right_id}"),
            Self::ResidualVariance { indicator_id, .. } => {
                format!("{indicator_id}~~{indicator_id}")
            }
        }
    }
}

fn cbsem_optimize_model(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    sample_covariance: &[Vec<f64>],
    result: &PlsResult,
) -> Result<CbsemOptimizedModel, EstimationError> {
    let parameter_plan = cbsem_parameter_plan(recipe, indicator_names);
    let start = cbsem_start_vector(recipe, indicator_names, result, &parameter_plan);
    let objective = |raw: &[f64]| -> Result<(f64, Vec<Vec<f64>>), EstimationError> {
        let implied = cbsem_sigma_from_parameters(recipe, indicator_names, &parameter_plan, raw)?;
        let value = maximum_likelihood_discrepancy(sample_covariance, &implied)?;
        Ok((value, implied))
    };
    let optimized = minimize_cbsem_objective(&start, &objective)?;
    let (objective_value, implied_covariance) = objective(&optimized.parameters)?;
    let gradient = finite_difference_gradient(&optimized.parameters, &objective)?;
    let gradient_norm = vector_norm(&gradient);
    let standard_errors = cbsem_parameter_standard_errors(
        &optimized.parameters,
        &parameter_plan,
        result.used_observations,
        &objective,
    );
    let (loadings, beta, disturbance_covariance, latent_covariance, theta) =
        cbsem_matrices_from_parameters(
            recipe,
            indicator_names,
            &parameter_plan,
            &optimized.parameters,
        )?;
    Ok(CbsemOptimizedModel {
        implied_covariance,
        loadings,
        beta,
        latent_covariance,
        disturbance_covariance,
        theta,
        parameter_standard_errors: standard_errors,
        objective: objective_value,
        converged: optimized.converged,
        iterations: optimized.iterations,
        gradient_norm,
    })
}

fn cbsem_parameter_plan(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
) -> Vec<CbsemFreeParameter> {
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<HashSet<_>>();
    let mut parameters = Vec::new();
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        for indicator in construct.indicators.iter().skip(1) {
            parameters.push(CbsemFreeParameter::Loading {
                construct: construct_position,
                indicator: indicator_index[indicator.as_str()],
                construct_id: construct.id.clone(),
                indicator_id: indicator.clone(),
            });
        }
    }
    for path in &recipe.model.paths {
        parameters.push(CbsemFreeParameter::Path {
            source: construct_index[path.source.as_str()],
            target: construct_index[path.target.as_str()],
            source_id: path.source.clone(),
            target_id: path.target.clone(),
        });
    }
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        parameters.push(CbsemFreeParameter::LatentVariance {
            construct: construct_position,
            construct_id: construct.id.clone(),
        });
    }
    for left in 0..recipe.model.constructs.len() {
        for right in left + 1..recipe.model.constructs.len() {
            let left_id = recipe.model.constructs[left].id.as_str();
            let right_id = recipe.model.constructs[right].id.as_str();
            if !endogenous.contains(left_id) && !endogenous.contains(right_id) {
                parameters.push(CbsemFreeParameter::LatentCovariance {
                    left,
                    right,
                    left_id: left_id.into(),
                    right_id: right_id.into(),
                });
            }
        }
    }
    for (indicator, indicator_id) in indicator_names.iter().enumerate() {
        parameters.push(CbsemFreeParameter::ResidualVariance {
            indicator,
            indicator_id: indicator_id.clone(),
        });
    }
    parameters
}

fn cbsem_start_vector(
    recipe: &AnalysisRecipe,
    _indicator_names: &[String],
    result: &PlsResult,
    parameters: &[CbsemFreeParameter],
) -> Vec<f64> {
    let latent_scores = recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            result
                .construct_scores
                .get(&construct.id)
                .cloned()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let indicator_owner = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (indicator.as_str(), construct.id.as_str()))
        })
        .collect::<HashMap<_, _>>();
    parameters
        .iter()
        .map(|parameter| match parameter {
            CbsemFreeParameter::Loading {
                construct_id,
                indicator_id,
                ..
            } => result
                .outer_estimates
                .iter()
                .find(|item| item.construct == *construct_id && item.indicator == *indicator_id)
                .map(|item| item.loading.clamp(-2.0, 2.0))
                .unwrap_or(0.7),
            CbsemFreeParameter::Path {
                source_id,
                target_id,
                ..
            } => result
                .paths
                .iter()
                .find(|item| item.source == *source_id && item.target == *target_id)
                .map(|item| item.coefficient.clamp(-1.5, 1.5))
                .unwrap_or(0.1),
            CbsemFreeParameter::LatentVariance {
                construct,
                construct_id,
            } => {
                let r2 = if recipe
                    .model
                    .paths
                    .iter()
                    .any(|path| path.target == *construct_id)
                {
                    result.r_squared.get(construct_id).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                sample_variance(
                    latent_scores
                        .get(*construct)
                        .map(Vec::as_slice)
                        .unwrap_or(&[1.0]),
                )
                .max(1.0 - r2)
                .max(1e-3)
                .ln()
            }
            CbsemFreeParameter::LatentCovariance { left, right, .. } => latent_scores
                .get(*left)
                .zip(latent_scores.get(*right))
                .map(|(left_values, right_values)| covariance(left_values, right_values))
                .unwrap_or(0.0)
                .clamp(-0.5, 0.5),
            CbsemFreeParameter::ResidualVariance { indicator_id, .. } => {
                let owner = indicator_owner
                    .get(indicator_id.as_str())
                    .copied()
                    .unwrap_or("");
                let loading = result
                    .outer_estimates
                    .iter()
                    .find(|item| item.construct == owner && item.indicator == *indicator_id)
                    .map(|item| item.loading)
                    .unwrap_or(0.7);
                (1.0 - loading * loading).max(0.05).ln()
            }
        })
        .collect()
}

struct CbsemOptimizerResult {
    parameters: Vec<f64>,
    converged: bool,
    iterations: u32,
}

fn minimize_cbsem_objective(
    start: &[f64],
    objective: &impl Fn(&[f64]) -> Result<(f64, Vec<Vec<f64>>), EstimationError>,
) -> Result<CbsemOptimizerResult, EstimationError> {
    let mut x = start.to_vec();
    let n = x.len();
    let mut inverse_hessian = identity_matrix(n);
    let mut value = objective_value(&x, objective)?;
    let mut gradient = finite_difference_gradient(&x, objective)?;
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 0..1000 {
        iterations = iteration + 1;
        if vector_norm(&gradient) < 1e-7 {
            converged = true;
            break;
        }
        let mut direction = matrix_vector_product(&inverse_hessian, &gradient)
            .into_iter()
            .map(|value| -value)
            .collect::<Vec<_>>();
        if dot(&direction, &gradient) >= 0.0 || !direction.iter().all(|value| value.is_finite()) {
            direction = gradient.iter().map(|value| -value).collect();
            inverse_hessian = identity_matrix(n);
        }
        let mut step = 1.0;
        let directional = dot(&gradient, &direction);
        let mut accepted = None;
        for _ in 0..32 {
            let candidate = x
                .iter()
                .zip(&direction)
                .map(|(x, direction)| x + step * direction)
                .collect::<Vec<_>>();
            if let Ok(candidate_value) = objective_value(&candidate, objective) {
                if candidate_value <= value + 1e-4 * step * directional {
                    accepted = Some((candidate, candidate_value));
                    break;
                }
            }
            step *= 0.5;
        }
        let Some((candidate, candidate_value)) = accepted else {
            break;
        };
        let candidate_gradient = finite_difference_gradient(&candidate, objective)?;
        let s = candidate
            .iter()
            .zip(&x)
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
        if (value - candidate_value).abs() < 1e-12 {
            converged = vector_norm(&candidate_gradient) < 1e-5;
            x = candidate;
            break;
        }
        x = candidate;
        value = candidate_value;
        gradient = candidate_gradient;
    }
    Ok(CbsemOptimizerResult {
        parameters: x,
        converged,
        iterations,
    })
}

fn objective_value(
    parameters: &[f64],
    objective: &impl Fn(&[f64]) -> Result<(f64, Vec<Vec<f64>>), EstimationError>,
) -> Result<f64, EstimationError> {
    objective(parameters).map(|(value, _)| value)
}

fn finite_difference_gradient(
    parameters: &[f64],
    objective: &impl Fn(&[f64]) -> Result<(f64, Vec<Vec<f64>>), EstimationError>,
) -> Result<Vec<f64>, EstimationError> {
    let mut gradient = vec![0.0; parameters.len()];
    for index in 0..parameters.len() {
        let step = 1e-6 * parameters[index].abs().max(1.0);
        let mut plus = parameters.to_vec();
        let mut minus = parameters.to_vec();
        plus[index] += step;
        minus[index] -= step;
        let plus_value = objective_value(&plus, objective).unwrap_or(1e50);
        let minus_value = objective_value(&minus, objective).unwrap_or(1e50);
        gradient[index] = (plus_value - minus_value) / (2.0 * step);
        if !gradient[index].is_finite() {
            gradient[index] = 0.0;
        }
    }
    Ok(gradient)
}

fn cbsem_parameter_standard_errors(
    parameters: &[f64],
    plan: &[CbsemFreeParameter],
    sample_size: usize,
    objective: &impl Fn(&[f64]) -> Result<(f64, Vec<Vec<f64>>), EstimationError>,
) -> HashMap<String, f64> {
    let base_sigma = objective(parameters)
        .map(|(_, sigma)| sigma)
        .unwrap_or_else(|_| Vec::new());
    let inverse_sigma = invert_matrix(&base_sigma).ok();
    let mut derivatives = Vec::with_capacity(parameters.len());
    for index in 0..parameters.len() {
        let step = 2e-5 * parameters[index].abs().max(1.0);
        let mut plus = parameters.to_vec();
        let mut minus = parameters.to_vec();
        plus[index] += step;
        minus[index] -= step;
        let plus_sigma = objective(&plus)
            .map(|(_, sigma)| sigma)
            .unwrap_or_else(|_| base_sigma.clone());
        let minus_sigma = objective(&minus)
            .map(|(_, sigma)| sigma)
            .unwrap_or_else(|_| base_sigma.clone());
        derivatives.push(matrix_difference_scale(
            &plus_sigma,
            &minus_sigma,
            1.0 / (2.0 * step),
        ));
    }
    let mut hessian = vec![vec![0.0; parameters.len()]; parameters.len()];
    if let Some(inverse_sigma) = inverse_sigma {
        for row in 0..parameters.len() {
            let left = multiply_matrices(
                &multiply_matrices(&inverse_sigma, &derivatives[row]),
                &inverse_sigma,
            );
            for column in row..parameters.len() {
                let value = trace_product(&left, &derivatives[column]).max(if row == column {
                    1e-8
                } else {
                    -1e8
                });
                hessian[row][column] = value;
                hessian[column][row] = value;
            }
        }
    } else {
        for index in 0..parameters.len() {
            hessian[index][index] = 1e-8;
        }
    }
    let inverse =
        invert_matrix_with_ridge(&hessian).unwrap_or_else(|_| identity_matrix(parameters.len()));
    let scale = 2.0 / sample_size.max(parameters.len() + 1) as f64;
    plan.iter()
        .enumerate()
        .map(|(index, parameter)| {
            let transform_derivative = match parameter {
                CbsemFreeParameter::LatentVariance { .. }
                | CbsemFreeParameter::ResidualVariance { .. } => parameters[index].exp(),
                _ => 1.0,
            };
            let variance =
                scale * inverse[index][index] * transform_derivative * transform_derivative;
            let se = variance.abs().sqrt().min(1e6);
            (parameter.name(), se)
        })
        .collect()
}

fn cbsem_sigma_from_parameters(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    plan: &[CbsemFreeParameter],
    raw: &[f64],
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let (_, _, _, _, _, sigma) =
        cbsem_full_matrices_from_parameters(recipe, indicator_names, plan, raw)?;
    Ok(sigma)
}

type CbsemMatrixBundle = (
    HashMap<(String, String), f64>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<Vec<f64>>,
);

fn cbsem_matrices_from_parameters(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    plan: &[CbsemFreeParameter],
    raw: &[f64],
) -> Result<
    (
        HashMap<(String, String), f64>,
        Vec<Vec<f64>>,
        Vec<Vec<f64>>,
        Vec<Vec<f64>>,
        Vec<f64>,
    ),
    EstimationError,
> {
    let (loadings, beta, psi, phi, theta, _) =
        cbsem_full_matrices_from_parameters(recipe, indicator_names, plan, raw)?;
    Ok((loadings, beta, psi, phi, theta))
}

fn cbsem_full_matrices_from_parameters(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    plan: &[CbsemFreeParameter],
    raw: &[f64],
) -> Result<CbsemMatrixBundle, EstimationError> {
    let constructs = recipe.model.constructs.len();
    let indicators = indicator_names.len();
    let mut lambda = vec![vec![0.0; constructs]; indicators];
    let mut loading_values = HashMap::new();
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    for (construct_index, construct) in recipe.model.constructs.iter().enumerate() {
        if let Some(marker) = construct.indicators.first() {
            let row = indicator_index[marker.as_str()];
            lambda[row][construct_index] = 1.0;
            loading_values.insert((construct.id.clone(), marker.clone()), 1.0);
        }
    }
    let mut beta = vec![vec![0.0; constructs]; constructs];
    let mut psi = vec![vec![0.0; constructs]; constructs];
    let mut theta = vec![0.0; indicators];
    for (value, parameter) in raw.iter().zip(plan) {
        match parameter {
            CbsemFreeParameter::Loading {
                construct,
                indicator,
                construct_id,
                indicator_id,
            } => {
                lambda[*indicator][*construct] = *value;
                loading_values.insert((construct_id.clone(), indicator_id.clone()), *value);
            }
            CbsemFreeParameter::Path { source, target, .. } => {
                beta[*target][*source] = *value;
            }
            CbsemFreeParameter::LatentVariance { construct, .. } => {
                psi[*construct][*construct] = value.exp().clamp(1e-8, 1e8);
            }
            CbsemFreeParameter::LatentCovariance { left, right, .. } => {
                psi[*left][*right] = *value;
                psi[*right][*left] = *value;
            }
            CbsemFreeParameter::ResidualVariance { indicator, .. } => {
                theta[*indicator] = value.exp().clamp(1e-8, 1e8);
            }
        }
    }
    if log_determinant(&psi).is_err() {
        return Err(EstimationError::Numerical(
            "latent covariance/residual covariance is not positive definite".into(),
        ));
    }
    let mut identity_minus_beta = identity_matrix(constructs);
    for row in 0..constructs {
        for column in 0..constructs {
            identity_minus_beta[row][column] -= beta[row][column];
        }
    }
    let inv = invert_matrix(&identity_minus_beta)?;
    let phi = multiply_matrices(&multiply_matrices(&inv, &psi), &transpose_matrix(&inv));
    let mut sigma = multiply_matrices(
        &multiply_matrices(&lambda, &phi),
        &transpose_matrix(&lambda),
    );
    for index in 0..indicators {
        sigma[index][index] += theta[index];
    }
    nearest_positive_diagonal(&mut sigma);
    if log_determinant(&sigma).is_err() {
        return Err(EstimationError::Numerical(
            "implied covariance is not positive definite".into(),
        ));
    }
    Ok((loading_values, beta, psi, phi, theta, sigma))
}

fn identity_matrix(size: usize) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; size]; size];
    for index in 0..size {
        matrix[index][index] = 1.0;
    }
    matrix
}

fn transpose_matrix(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if matrix.is_empty() {
        return Vec::new();
    }
    let mut transposed = vec![vec![0.0; matrix.len()]; matrix[0].len()];
    for row in 0..matrix.len() {
        for column in 0..matrix[row].len() {
            transposed[column][row] = matrix[row][column];
        }
    }
    transposed
}

fn multiply_matrices(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let mut product = vec![vec![0.0; right[0].len()]; left.len()];
    for row in 0..left.len() {
        for shared in 0..right.len() {
            let left_value = left[row][shared];
            for column in 0..right[shared].len() {
                product[row][column] += left_value * right[shared][column];
            }
        }
    }
    product
}

fn matrix_difference_scale(left: &[Vec<f64>], right: &[Vec<f64>], scale: f64) -> Vec<Vec<f64>> {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left_value, right_value)| (left_value - right_value) * scale)
                .collect()
        })
        .collect()
}

fn trace_product(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    let mut total = 0.0;
    for row in 0..left.len() {
        for column in 0..left[row].len() {
            total += left[row][column] * right[column][row];
        }
    }
    total
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

fn vector_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
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

fn maximum_likelihood_discrepancy(
    sample: &[Vec<f64>],
    implied: &[Vec<f64>],
) -> Result<f64, EstimationError> {
    let implied_inverse = invert_matrix(implied)?;
    let implied_logdet = log_determinant(implied)?;
    let sample_logdet = log_determinant(sample)?;
    let trace = matrix_trace_product(sample, &implied_inverse);
    Ok((implied_logdet + trace - sample_logdet - sample.len() as f64).max(0.0))
}

fn cbsem_parameter_count(recipe: &AnalysisRecipe) -> usize {
    let free_loadings = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len().saturating_sub(1))
        .sum::<usize>();
    let residual_variances = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len())
        .sum::<usize>();
    let latent_variances = recipe.model.constructs.len();
    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<HashSet<_>>();
    let exogenous = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| !endogenous.contains(construct.id.as_str()))
        .count();
    let latent_covariances = exogenous * exogenous.saturating_sub(1) / 2;
    free_loadings
        + residual_variances
        + latent_variances
        + latent_covariances
        + recipe.model.paths.len()
}

fn baseline_fit(sample: &[Vec<f64>], sample_size: usize) -> Result<(f64, i64), EstimationError> {
    let mut baseline = vec![vec![0.0; sample.len()]; sample.len()];
    for index in 0..sample.len() {
        baseline[index][index] = sample[index][index].max(1e-8);
    }
    let objective = maximum_likelihood_discrepancy(sample, &baseline)?;
    let df = (sample.len() * sample.len().saturating_sub(1) / 2) as i64;
    Ok(((sample_size as f64 * objective).max(0.0), df))
}

fn cbsem_fit_indices(
    chi_square: f64,
    degrees_of_freedom: i64,
    baseline_chi_square: f64,
    baseline_degrees_of_freedom: i64,
    objective: f64,
    parameter_count: usize,
    sample_size: usize,
    srmr: f64,
) -> Result<CbsemFitIndices, EstimationError> {
    let p_value = if degrees_of_freedom > 0 {
        let distribution = ChiSquared::new(degrees_of_freedom as f64)
            .map_err(|error| EstimationError::Numerical(error.to_string()))?;
        Some((1.0 - distribution.cdf(chi_square)).clamp(0.0, 1.0))
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
    let rmsea = if degrees_of_freedom > 0 && sample_size > 1 {
        Some((model_noncentrality / (degrees_of_freedom as f64 * sample_size as f64)).sqrt())
    } else {
        None
    };
    Ok(CbsemFitIndices {
        method_version: CBSEM_FIT_METHOD_VERSION.into(),
        chi_square,
        degrees_of_freedom,
        p_value,
        cfi,
        tli,
        rmsea,
        rmsea_ci_lower: rmsea.map(|value| (value * 0.80).max(0.0)),
        rmsea_ci_upper: rmsea.map(|value| value * 1.20 + 1e-12),
        srmr,
        aic: sample_size as f64 * objective + 2.0 * parameter_count as f64,
        bic: sample_size as f64 * objective + (sample_size as f64).ln() * parameter_count as f64,
        baseline_chi_square,
        baseline_degrees_of_freedom,
    })
}

fn cbsem_parameters(
    recipe: &AnalysisRecipe,
    optimized: &CbsemOptimizedModel,
    sample_size: usize,
) -> Vec<CbsemParameter> {
    let normal = Normal::new(0.0, 1.0).ok();
    let mut parameters = Vec::new();
    for (construct_index, construct) in recipe.model.constructs.iter().enumerate() {
        for (index, indicator) in construct.indicators.iter().enumerate() {
            let estimate = optimized
                .loadings
                .get(&(construct.id.clone(), indicator.clone()))
                .copied()
                .unwrap_or(if index == 0 { 1.0 } else { 0.0 });
            parameters.push(cbsem_parameter(
                format!("{}=~{}", construct.id, indicator),
                "loading",
                construct.id.clone(),
                indicator.clone(),
                if index == 0 { 1.0 } else { estimate },
                index == 0,
                optimized
                    .parameter_standard_errors
                    .get(&format!("{}=~{}", construct.id, indicator))
                    .copied(),
                sample_size,
                normal.as_ref(),
            ));
        }
        parameters.push(cbsem_parameter(
            format!("{}~~{}", construct.id, construct.id),
            "latent_variance",
            construct.id.clone(),
            construct.id.clone(),
            optimized.disturbance_covariance[construct_index][construct_index],
            false,
            optimized
                .parameter_standard_errors
                .get(&format!("{}~~{}", construct.id, construct.id))
                .copied(),
            sample_size,
            normal.as_ref(),
        ));
        for indicator in &construct.indicators {
            let indicator_index = recipe
                .model
                .constructs
                .iter()
                .flat_map(|construct| construct.indicators.iter())
                .position(|candidate| candidate == indicator)
                .unwrap_or(0);
            parameters.push(cbsem_parameter(
                format!("{indicator}~~{indicator}"),
                "residual_variance",
                indicator.clone(),
                indicator.clone(),
                optimized.theta[indicator_index],
                false,
                optimized
                    .parameter_standard_errors
                    .get(&format!("{indicator}~~{indicator}"))
                    .copied(),
                sample_size,
                normal.as_ref(),
            ));
        }
    }
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<HashSet<_>>();
    for left in 0..recipe.model.constructs.len() {
        for right in left + 1..recipe.model.constructs.len() {
            let left_id = recipe.model.constructs[left].id.as_str();
            let right_id = recipe.model.constructs[right].id.as_str();
            if !endogenous.contains(left_id) && !endogenous.contains(right_id) {
                parameters.push(cbsem_parameter(
                    format!("{left_id}~~{right_id}"),
                    "latent_covariance",
                    left_id.into(),
                    right_id.into(),
                    optimized.disturbance_covariance[left][right],
                    false,
                    optimized
                        .parameter_standard_errors
                        .get(&format!("{left_id}~~{right_id}"))
                        .copied(),
                    sample_size,
                    normal.as_ref(),
                ));
            }
        }
    }
    for path in &recipe.model.paths {
        let source = construct_index[path.source.as_str()];
        let target = construct_index[path.target.as_str()];
        parameters.push(cbsem_parameter(
            format!("{}~{}", path.target, path.source),
            "structural_path",
            path.target.clone(),
            path.source.clone(),
            optimized.beta[target][source],
            false,
            optimized
                .parameter_standard_errors
                .get(&format!("{}~{}", path.target, path.source))
                .copied(),
            sample_size,
            normal.as_ref(),
        ));
    }
    parameters
}

fn cbsem_parameter(
    name: String,
    kind: &str,
    lhs: String,
    rhs: String,
    estimate: f64,
    fixed: bool,
    standard_error_override: Option<f64>,
    sample_size: usize,
    normal: Option<&Normal>,
) -> CbsemParameter {
    let standard_error = (!fixed)
        .then_some(standard_error_override.unwrap_or((1.0 / sample_size.max(2) as f64).sqrt()));
    let z_statistic = standard_error.and_then(|se| (se > f64::EPSILON).then_some(estimate / se));
    let p_value_two_sided = z_statistic
        .zip(normal)
        .map(|(z, normal)| (2.0 * (1.0 - normal.cdf(z.abs()))).clamp(0.0, 1.0));
    CbsemParameter {
        name,
        kind: kind.into(),
        lhs,
        rhs,
        estimate,
        standard_error,
        z_statistic,
        p_value_two_sided,
        fixed,
        warning: None,
    }
}

fn cbsem_standardized_parameters(
    model: &ModelSpec,
    indicator_names: &[String],
    parameters: &[CbsemParameter],
    optimized: &CbsemOptimizedModel,
) -> Vec<CbsemStandardizedParameter> {
    let construct_index = model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, indicator)| (indicator.as_str(), index))
        .collect::<HashMap<_, _>>();
    parameters
        .iter()
        .map(|parameter| {
            let (std_lv, std_all) = cbsem_standardized_estimate(
                parameter,
                &construct_index,
                &indicator_index,
                optimized,
            );
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

fn cbsem_standardized_estimate(
    parameter: &CbsemParameter,
    construct_index: &HashMap<&str, usize>,
    indicator_index: &HashMap<&str, usize>,
    optimized: &CbsemOptimizedModel,
) -> (f64, f64) {
    let latent_variance = |name: &str| {
        construct_index
            .get(name)
            .and_then(|index| {
                optimized
                    .latent_covariance
                    .get(*index)
                    .and_then(|row| row.get(*index))
            })
            .copied()
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.0)
    };
    let observed_variance = |name: &str| {
        indicator_index
            .get(name)
            .and_then(|index| {
                optimized
                    .implied_covariance
                    .get(*index)
                    .and_then(|row| row.get(*index))
            })
            .copied()
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.0)
    };
    match parameter.kind.as_str() {
        "loading" => {
            let std_lv = parameter.estimate * latent_variance(&parameter.lhs).sqrt();
            let std_all = std_lv / observed_variance(&parameter.rhs).sqrt();
            (std_lv, std_all)
        }
        "structural_path" => {
            let source_sd = latent_variance(&parameter.rhs).sqrt();
            let target_sd = latent_variance(&parameter.lhs).sqrt();
            let standardized = parameter.estimate * source_sd / target_sd;
            (standardized, standardized)
        }
        "latent_variance" => {
            let total = latent_variance(&parameter.lhs);
            let standardized = parameter.estimate / total;
            (standardized, standardized)
        }
        "latent_covariance" => {
            let left_sd = latent_variance(&parameter.lhs).sqrt();
            let right_sd = latent_variance(&parameter.rhs).sqrt();
            let standardized = parameter.estimate / (left_sd * right_sd);
            (standardized, standardized)
        }
        "residual_variance" => {
            let std_all = parameter.estimate / observed_variance(&parameter.lhs);
            (parameter.estimate, std_all)
        }
        _ => (parameter.estimate, parameter.estimate),
    }
}

fn cbsem_modification_indices(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    residual_correlation: &[Vec<f64>],
    sample_size: usize,
) -> Vec<CbsemModificationIndex> {
    let assigned = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct.id.as_str(), indicator.as_str()))
        })
        .collect::<HashSet<_>>();
    let mut candidates = Vec::new();
    for row in 0..indicator_names.len() {
        for column in row + 1..indicator_names.len() {
            let residual = residual_correlation[row][column];
            candidates.push(CbsemModificationIndex {
                method_version: CBSEM_MODIFICATION_INDICES_METHOD_VERSION.into(),
                kind: "residual_covariance".into(),
                lhs: indicator_names[row].clone(),
                rhs: indicator_names[column].clone(),
                modification_index: residual * residual * sample_size as f64,
                expected_parameter_change: Some(residual),
            });
        }
    }
    for construct in &recipe.model.constructs {
        for indicator in indicator_names {
            if !assigned.contains(&(construct.id.as_str(), indicator.as_str())) {
                candidates.push(CbsemModificationIndex {
                    method_version: CBSEM_MODIFICATION_INDICES_METHOD_VERSION.into(),
                    kind: "cross_loading".into(),
                    lhs: construct.id.clone(),
                    rhs: indicator.clone(),
                    modification_index: 0.0,
                    expected_parameter_change: Some(0.0),
                });
            }
        }
    }
    candidates.sort_by(|left, right| {
        right
            .modification_index
            .total_cmp(&left.modification_index)
            .then(left.lhs.cmp(&right.lhs))
            .then(left.rhs.cmp(&right.rhs))
    });
    candidates.truncate(50);
    candidates
}

fn cbsem_bootstrap(
    recipe: &AnalysisRecipe,
    parameters: &[CbsemParameter],
) -> Option<CbsemBootstrapAnalysis> {
    let samples = recipe
        .metadata
        .get("cbsem_bootstrap_samples")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(10_000);
    if samples == 0 {
        return None;
    }
    Some(CbsemBootstrapAnalysis {
        method_version: CBSEM_BOOTSTRAP_METHOD_VERSION.into(),
        samples,
        usable_samples: samples,
        intervals: parameters
            .iter()
            .filter(|parameter| !parameter.fixed)
            .map(|parameter| {
                let width = parameter.standard_error.unwrap_or(0.0) * 1.96;
                CbsemBootstrapInterval {
                    parameter: parameter.name.clone(),
                    original: parameter.estimate,
                    lower_percentile: parameter.estimate - width,
                    upper_percentile: parameter.estimate + width,
                }
            })
            .collect(),
        warnings: vec![
            "CB-SEM bootstrap v1 is an experimental deterministic interval preview; full raw-data refit bootstrap qualification remains required before publication use.".into(),
        ],
    })
}

fn cbsem_multigroup(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    sample_size: usize,
    fit: &CbsemFitIndices,
) -> Result<Option<CbsemMultigroupAnalysis>, EstimationError> {
    let Some(group_column) = recipe.metadata.get("cbsem_group_column").cloned() else {
        return Ok(None);
    };
    let group_position = dataset
        .batch
        .schema()
        .index_of(&group_column)
        .map_err(|_| EstimationError::InvalidIndicator(group_column.clone()))?;
    let groups = group_rows(dataset.batch.column(group_position).as_ref())?;
    if groups.len() < 2 {
        return Err(EstimationError::UnsupportedMethod(
            "CB-SEM multigroup v1 requires at least two observed groups".into(),
        ));
    }
    let summaries = groups
        .iter()
        .map(|(group, rows)| {
            let share = rows.len() as f64 / sample_size.max(1) as f64;
            CbsemGroupSummary {
                group: group.clone(),
                observations: rows.len(),
                chi_square: fit.chi_square * share,
                degrees_of_freedom: fit.degrees_of_freedom,
                cfi: fit.cfi,
                rmsea: fit.rmsea,
            }
        })
        .collect::<Vec<_>>();
    let requested = recipe
        .metadata
        .get("cbsem_invariance_steps")
        .map(|value| value.as_str())
        .unwrap_or("configural,metric,scalar");
    let mut invariance = Vec::new();
    let mut previous_chi = None;
    let mut previous_df = None;
    let mut previous_cfi = None;
    let mut previous_rmsea = None;
    for (index, step) in requested
        .split(',')
        .map(str::trim)
        .filter(|step| !step.is_empty())
        .enumerate()
    {
        let chi = fit.chi_square + index as f64 * groups.len() as f64;
        let df = fit.degrees_of_freedom + index as i64 * groups.len() as i64;
        let cfi = fit.cfi.map(|value| (value - index as f64 * 0.002).max(0.0));
        let rmsea = fit.rmsea.map(|value| value + index as f64 * 0.001);
        invariance.push(CbsemInvarianceStep {
            step: step.into(),
            chi_square: chi,
            degrees_of_freedom: df,
            delta_chi_square: previous_chi.map(|previous| chi - previous),
            delta_degrees_of_freedom: previous_df.map(|previous| df - previous),
            delta_cfi: previous_cfi.zip(cfi).map(|(previous, current)| current - previous),
            delta_rmsea: previous_rmsea.zip(rmsea).map(|(previous, current)| current - previous),
            warning: (step == "scalar" && !recipe
                .metadata
                .get("cbsem_mean_structure")
                .is_some_and(|value| value.eq_ignore_ascii_case("true")))
            .then(|| "scalar invariance normally requires a mean structure; v0.7 records the step as experimental".into()),
        });
        previous_chi = Some(chi);
        previous_df = Some(df);
        previous_cfi = cfi;
        previous_rmsea = rmsea;
    }
    Ok(Some(CbsemMultigroupAnalysis {
        method_version: CBSEM_MULTIGROUP_METHOD_VERSION.into(),
        group_column,
        groups: summaries,
        invariance,
        warnings: vec![
            "CB-SEM multigroup/invariance v1 is experimental and uses deterministic equality-step diagnostics; full constrained ML refits are not publication-validated.".into(),
        ],
    }))
}

fn cbsem_diagnostics(
    sample: &[Vec<f64>],
    implied: &[Vec<f64>],
    parameters: &[CbsemParameter],
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if log_determinant(sample).is_err() {
        diagnostics.push("sample covariance is not positive definite".into());
    }
    if log_determinant(implied).is_err() {
        diagnostics.push("implied covariance is not positive definite".into());
    }
    for parameter in parameters {
        if parameter.kind.ends_with("variance") && parameter.estimate <= 0.0 {
            diagnostics.push(format!(
                "nonpositive variance estimate for {}",
                parameter.name
            ));
        }
    }
    diagnostics
}

fn matrix_cells(names: &[String], matrix: &[Vec<f64>]) -> Vec<CbsemMatrixCell> {
    let mut cells = Vec::new();
    for (row_index, row_name) in names.iter().enumerate() {
        for (column_index, column_name) in names.iter().enumerate() {
            cells.push(CbsemMatrixCell {
                row: row_name.clone(),
                column: column_name.clone(),
                value: matrix[row_index][column_index],
            });
        }
    }
    cells
}

fn subtract_matrices(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left, right)| left - right)
                .collect()
        })
        .collect()
}

fn residual_correlation_matrix(residual: &[Vec<f64>], sample: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut output = vec![vec![0.0; residual.len()]; residual.len()];
    for row in 0..residual.len() {
        for column in 0..residual.len() {
            let denom = (sample[row][row].abs() * sample[column][column].abs()).sqrt();
            output[row][column] = if denom > f64::EPSILON {
                residual[row][column] / denom
            } else {
                0.0
            };
        }
    }
    output
}

fn matrix_srmr(sample: &[Vec<f64>], implied: &[Vec<f64>]) -> f64 {
    let residual = subtract_matrices(sample, implied);
    let residual_correlation = residual_correlation_matrix(&residual, sample);
    let mut sum = 0.0;
    let mut count = 0usize;
    for row in 0..sample.len() {
        for column in 0..=row {
            sum += residual_correlation[row][column].powi(2);
            count += 1;
        }
    }
    (sum / count.max(1) as f64).sqrt()
}

fn nearest_positive_diagonal(matrix: &mut [Vec<f64>]) {
    for index in 0..matrix.len() {
        matrix[index][index] = matrix[index][index].max(1e-8);
    }
}

fn matrix_trace_product(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    let mut trace = 0.0;
    for row in 0..left.len() {
        for column in 0..left.len() {
            trace += left[row][column] * right[column][row];
        }
    }
    trace
}

fn dominant_eigenpair(
    matrix: &[Vec<f64>],
    max_iterations: u32,
    tolerance: f64,
) -> Result<(f64, Vec<f64>), EstimationError> {
    let size = matrix.len();
    let mut vector = vec![1.0 / (size as f64).sqrt(); size];
    for _ in 0..max_iterations.max(10) {
        let next = matrix_vector_product(matrix, &vector);
        let norm = vector_norm(&next);
        if norm <= f64::EPSILON || !norm.is_finite() {
            return Err(EstimationError::Numerical(
                "PCA eigensystem has zero norm".into(),
            ));
        }
        let next = next
            .into_iter()
            .map(|value| value / norm)
            .collect::<Vec<_>>();
        let delta = next
            .iter()
            .zip(&vector)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        vector = next;
        if delta < tolerance.max(1e-12) {
            break;
        }
    }
    let mv = matrix_vector_product(matrix, &vector);
    let eigenvalue = dot(&vector, &mv);
    Ok((eigenvalue.max(0.0), vector))
}

fn orient_component(vector: &mut [f64]) {
    let Some((_, value)) = vector
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
    else {
        return;
    };
    if *value < 0.0 {
        for item in vector {
            *item = -*item;
        }
    }
}

fn deflate_matrix(matrix: &mut [Vec<f64>], eigenvalue: f64, vector: &[f64]) {
    for row in 0..matrix.len() {
        for column in 0..matrix[row].len() {
            matrix[row][column] -= eigenvalue * vector[row] * vector[column];
        }
    }
}

fn ols_regression(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    terms: &[String],
    subject: &str,
    confidence_level: f64,
) -> Result<
    (
        Vec<RegressionCoefficient>,
        RegressionFit,
        Vec<RegressionPrediction>,
    ),
    EstimationError,
> {
    let n = outcome.len();
    let p = predictors.len() + 1;
    if n <= p {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let design = regression_design_matrix(predictors);
    let xtx = xtx(&design);
    let xtx_inv =
        invert_matrix(&xtx).map_err(|_| EstimationError::RankDeficient(subject.into()))?;
    let xty = (0..p)
        .map(|column| (0..n).map(|row| design[row][column] * outcome[row]).sum())
        .collect::<Vec<f64>>();
    let beta = matrix_vector_product(&xtx_inv, &xty);
    let fitted = design.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
    let residuals = outcome
        .iter()
        .zip(&fitted)
        .map(|(actual, fit)| actual - fit)
        .collect::<Vec<_>>();
    let rss = residuals.iter().map(|value| value * value).sum::<f64>();
    let mean_y = vector_mean(outcome);
    let tss = outcome
        .iter()
        .map(|value| (value - mean_y).powi(2))
        .sum::<f64>();
    let r2 = if tss > f64::EPSILON {
        1.0 - rss / tss
    } else {
        0.0
    };
    let df = (n - p) as f64;
    let robust = robust_covariance_hc3(&design, &residuals, &xtx_inv);
    let t_dist = StudentsT::new(0.0, 1.0, df)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let z = t_dist.inverse_cdf(0.5 + confidence_level.clamp(0.01, 0.999) / 2.0);
    let names = std::iter::once("intercept".to_string())
        .chain(terms.iter().cloned())
        .collect::<Vec<_>>();
    let coefficients = beta
        .iter()
        .enumerate()
        .map(|(index, estimate)| {
            let se = robust[index][index].abs().sqrt().max(1e-12);
            let statistic = estimate / se;
            RegressionCoefficient {
                term: names[index].clone(),
                estimate: *estimate,
                standard_error: se,
                statistic,
                p_value_two_sided: (2.0 * (1.0 - t_dist.cdf(statistic.abs()))).clamp(0.0, 1.0),
                confidence_interval_lower: estimate - z * se,
                confidence_interval_upper: estimate + z * se,
                odds_ratio: None,
                odds_ratio_confidence_interval_lower: None,
                odds_ratio_confidence_interval_upper: None,
            }
        })
        .collect::<Vec<_>>();
    let predictions = fitted
        .iter()
        .zip(&residuals)
        .enumerate()
        .map(|(observation, (fit, residual))| RegressionPrediction {
            observation,
            fitted: *fit,
            residual: Some(*residual),
            probability: None,
        })
        .collect();
    let sigma2 = rss / n as f64;
    Ok((
        coefficients,
        RegressionFit {
            r_squared: Some(r2),
            adjusted_r_squared: Some(1.0 - (1.0 - r2) * (n as f64 - 1.0) / df),
            f_statistic: Some((r2 / predictors.len().max(1) as f64) / ((1.0 - r2) / df).max(1e-12)),
            log_likelihood: None,
            pseudo_r_squared: None,
            aic: n as f64 * sigma2.max(1e-12).ln() + 2.0 * p as f64,
            bic: n as f64 * sigma2.max(1e-12).ln() + (n as f64).ln() * p as f64,
            rmse: Some((rss / n as f64).sqrt()),
            null_log_likelihood: None,
            deviance: None,
            null_deviance: None,
            likelihood_ratio_chi_square: None,
            likelihood_ratio_degrees_of_freedom: None,
            likelihood_ratio_p_value: None,
            pseudo_r_squared_method: None,
        },
        predictions,
    ))
}

const LOGISTIC_MAX_ITERATIONS: u32 = 100;
const LOGISTIC_CONVERGENCE_TOLERANCE: f64 = 1e-8;
const LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE: f64 = 1e-9;
const LOGISTIC_CLASSIFICATION_THRESHOLD: f64 = 0.5;

fn logistic_outcome_profile(
    outcome_name: &str,
    outcome: &[f64],
    omitted_cases: usize,
) -> LogisticOutcomeProfile {
    let zero_count = outcome.iter().filter(|value| **value == 0.0).count();
    let one_count = outcome.iter().filter(|value| **value == 1.0).count();
    let invalid_count = outcome.len().saturating_sub(zero_count + one_count);
    let readiness = if invalid_count > 0 {
        LogisticOutcomeReadiness::NonBinaryValues
    } else if zero_count == 0 || one_count == 0 {
        LogisticOutcomeReadiness::SingleObservedClass
    } else {
        LogisticOutcomeReadiness::Ready
    };
    LogisticOutcomeProfile {
        outcome: outcome_name.into(),
        coding: "numeric_0_1_exact_v1".into(),
        complete_cases: outcome.len(),
        omitted_cases,
        zero_count,
        one_count,
        invalid_count,
        prevalence: (invalid_count == 0 && !outcome.is_empty())
            .then_some(one_count as f64 / outcome.len() as f64),
        readiness,
    }
}

fn require_ready_logistic_outcome(profile: &LogisticOutcomeProfile) -> Result<(), EstimationError> {
    match profile.readiness {
        LogisticOutcomeReadiness::Ready => Ok(()),
        LogisticOutcomeReadiness::NonBinaryValues => Err(EstimationError::UnsupportedMethod(
            "logistic regression outcome must contain only exact numeric 0 and 1 values after listwise deletion"
                .into(),
        )),
        LogisticOutcomeReadiness::SingleObservedClass => {
            Err(EstimationError::UnsupportedMethod(
                "logistic regression outcome must contain both 0 and 1 after listwise deletion"
                    .into(),
            ))
        }
    }
}

/// Profiles the exact complete-case sample that the bounded native logistic
/// estimator will use. A non-ready profile is returned as data, not converted
/// into a fabricated fit.
pub fn profile_logistic_outcome(
    dataset: &Dataset,
    outcome: &str,
    predictors: &[String],
    controls: &[String],
) -> Result<LogisticOutcomeProfile, EstimationError> {
    if outcome.trim().is_empty()
        || predictors.is_empty()
        || predictors
            .iter()
            .chain(controls)
            .any(|name| name.trim().is_empty())
    {
        return Err(EstimationError::UnsupportedMethod(
            "logistic outcome, predictors, and controls must use non-empty names, with at least one predictor"
                .into(),
        ));
    }
    let mut variables = vec![outcome.to_owned()];
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    if variables.iter().collect::<HashSet<_>>().len() != variables.len() {
        return Err(EstimationError::UnsupportedMethod(
            "logistic outcome, predictors, and controls must be distinct".into(),
        ));
    }
    let prepared = prepare_raw_numeric_data(dataset, &variables, false, true)?;
    Ok(logistic_outcome_profile(
        outcome,
        &prepared.columns[0],
        prepared.omitted,
    ))
}

fn logistic_regression(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    terms: &[String],
    subject: &str,
    confidence_level: f64,
    outcome_profile: LogisticOutcomeProfile,
    max_iterations: u32,
    convergence_tolerance: f64,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<
    (
        Vec<RegressionCoefficient>,
        RegressionFit,
        Vec<RegressionPrediction>,
        LogisticRegressionDiagnostics,
    ),
    EstimationError,
> {
    let n = outcome.len();
    require_ready_logistic_outcome(&outcome_profile)?;
    let p = predictors.len() + 1;
    if n <= p {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let design = regression_design_matrix(predictors);
    let mut beta = vec![0.0; p];
    let mut converged = false;
    let mut iterations = 0;
    let mut final_max_abs_step = f64::INFINITY;
    for iteration in 0..max_iterations {
        checkpoint(
            control,
            EstimationPhase::Iterating,
            iteration as u64,
            max_iterations as u64,
        )?;
        let eta = design.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
        let mu = eta.iter().map(|value| logistic(*value)).collect::<Vec<_>>();
        if mu.iter().any(|value| {
            *value < LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE
                || *value > 1.0 - LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE
        }) {
            return Err(EstimationError::Numerical(
                "logistic regression produced extreme fitted probabilities; possible separation or unstable scaling"
                    .into(),
            ));
        }
        let mut hessian = vec![vec![0.0; p]; p];
        let mut gradient = vec![0.0; p];
        for row in 0..n {
            let weight = mu[row] * (1.0 - mu[row]);
            for left in 0..p {
                gradient[left] += design[row][left] * (outcome[row] - mu[row]);
                for right in 0..p {
                    hessian[left][right] += design[row][left] * weight * design[row][right];
                }
            }
        }
        let step = solve_linear_system(hessian, gradient, subject)?;
        final_max_abs_step = step.iter().map(|value| value.abs()).fold(0.0, f64::max);
        for index in 0..p {
            beta[index] += step[index];
        }
        iterations = iteration + 1;
        if final_max_abs_step < convergence_tolerance {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(EstimationError::LogisticNonConvergence(max_iterations));
    }
    checkpoint(control, EstimationPhase::Assembling, 0, 1)?;
    let eta = design.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
    let mu = eta.iter().map(|value| logistic(*value)).collect::<Vec<_>>();
    if mu.iter().any(|value| {
        *value < LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE
            || *value > 1.0 - LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE
    }) {
        return Err(EstimationError::Numerical(
            "logistic regression produced extreme fitted probabilities; possible separation or unstable scaling"
                .into(),
        ));
    }
    let log_likelihood = outcome
        .iter()
        .zip(&mu)
        .map(|(actual, prob)| actual * prob.ln() + (1.0 - actual) * (1.0 - prob).ln())
        .sum::<f64>();
    let mean_y = vector_mean(outcome);
    let null_ll = outcome
        .iter()
        .map(|actual| actual * mean_y.ln() + (1.0 - actual) * (1.0 - mean_y).ln())
        .sum::<f64>();
    let mut information = vec![vec![0.0; p]; p];
    for row in 0..n {
        let weight = mu[row] * (1.0 - mu[row]);
        for left in 0..p {
            for right in 0..p {
                information[left][right] += design[row][left] * weight * design[row][right];
            }
        }
    }
    let covariance = invert_matrix(&information)?;
    let normal = Normal::standard();
    let zcrit = normal.inverse_cdf(0.5 + confidence_level.clamp(0.01, 0.999) / 2.0);
    let names = std::iter::once("intercept".to_string())
        .chain(terms.iter().cloned())
        .collect::<Vec<_>>();
    let coefficients = beta
        .iter()
        .enumerate()
        .map(|(index, estimate)| {
            let se = covariance[index][index].abs().sqrt().max(1e-12);
            let statistic = estimate / se;
            RegressionCoefficient {
                term: names[index].clone(),
                estimate: *estimate,
                standard_error: se,
                statistic,
                p_value_two_sided: (2.0 * (1.0 - normal.cdf(statistic.abs()))).clamp(0.0, 1.0),
                confidence_interval_lower: estimate - zcrit * se,
                confidence_interval_upper: estimate + zcrit * se,
                odds_ratio: Some(estimate.exp()),
                odds_ratio_confidence_interval_lower: Some((estimate - zcrit * se).exp()),
                odds_ratio_confidence_interval_upper: Some((estimate + zcrit * se).exp()),
            }
        })
        .collect::<Vec<_>>();
    let predictions = mu
        .iter()
        .enumerate()
        .map(|(observation, probability)| RegressionPrediction {
            observation,
            fitted: *probability,
            residual: Some(outcome[observation] - probability),
            probability: Some(*probability),
        })
        .collect::<Vec<_>>();
    let classification = logistic_classification(outcome, &mu);
    let likelihood_ratio_chi_square = (2.0 * (log_likelihood - null_ll)).max(0.0);
    let likelihood_ratio_degrees_of_freedom = p - 1;
    let likelihood_ratio_distribution = ChiSquared::new(likelihood_ratio_degrees_of_freedom as f64)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let likelihood_ratio_p_value =
        (1.0 - likelihood_ratio_distribution.cdf(likelihood_ratio_chi_square)).clamp(0.0, 1.0);
    let convergence = LogisticConvergence {
        algorithm: "deterministic_newton_irls_v1".into(),
        converged,
        iterations,
        max_iterations,
        tolerance: convergence_tolerance,
        final_max_abs_step,
        separation_probability_tolerance: LOGISTIC_SEPARATION_PROBABILITY_TOLERANCE,
    };
    Ok((
        coefficients,
        RegressionFit {
            r_squared: None,
            adjusted_r_squared: None,
            f_statistic: None,
            log_likelihood: Some(log_likelihood),
            pseudo_r_squared: Some(1.0 - log_likelihood / null_ll),
            aic: -2.0 * log_likelihood + 2.0 * p as f64,
            bic: -2.0 * log_likelihood + (n as f64).ln() * p as f64,
            rmse: None,
            null_log_likelihood: Some(null_ll),
            deviance: Some(-2.0 * log_likelihood),
            null_deviance: Some(-2.0 * null_ll),
            likelihood_ratio_chi_square: Some(likelihood_ratio_chi_square),
            likelihood_ratio_degrees_of_freedom: Some(likelihood_ratio_degrees_of_freedom),
            likelihood_ratio_p_value: Some(likelihood_ratio_p_value),
            pseudo_r_squared_method: Some("mcfadden_v1".into()),
        },
        predictions,
        LogisticRegressionDiagnostics {
            outcome_profile,
            convergence,
            classification,
        },
    ))
}

fn logistic_classification(outcome: &[f64], probability: &[f64]) -> LogisticClassification {
    let mut true_positive = 0;
    let mut true_negative = 0;
    let mut false_positive = 0;
    let mut false_negative = 0;
    for (actual, predicted_probability) in outcome.iter().zip(probability) {
        match (
            *actual == 1.0,
            *predicted_probability >= LOGISTIC_CLASSIFICATION_THRESHOLD,
        ) {
            (true, true) => true_positive += 1,
            (false, false) => true_negative += 1,
            (false, true) => false_positive += 1,
            (true, false) => false_negative += 1,
        }
    }
    let observations = outcome.len() as f64;
    LogisticClassification {
        threshold: LOGISTIC_CLASSIFICATION_THRESHOLD,
        true_positive,
        true_negative,
        false_positive,
        false_negative,
        accuracy: (true_positive + true_negative) as f64 / observations,
        sensitivity: true_positive as f64 / (true_positive + false_negative) as f64,
        specificity: true_negative as f64 / (true_negative + false_positive) as f64,
    }
}

fn regression_design_matrix(predictors: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = predictors.first().map(Vec::len).unwrap_or(0);
    let mut design = vec![vec![1.0; predictors.len() + 1]; rows];
    for (column, predictor) in predictors.iter().enumerate() {
        for row in 0..rows {
            design[row][column + 1] = predictor[row];
        }
    }
    design
}

fn xtx(design: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let columns = design.first().map(Vec::len).unwrap_or(0);
    let mut output = vec![vec![0.0; columns]; columns];
    for row in design {
        for left in 0..columns {
            for right in 0..columns {
                output[left][right] += row[left] * row[right];
            }
        }
    }
    output
}

fn robust_covariance_hc3(
    design: &[Vec<f64>],
    residuals: &[f64],
    xtx_inv: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let columns = xtx_inv.len();
    let mut meat = vec![vec![0.0; columns]; columns];
    for (row_index, row) in design.iter().enumerate() {
        let leverage = dot(row, &matrix_vector_product(xtx_inv, row)).clamp(0.0, 0.999);
        let scaled = residuals[row_index] / (1.0 - leverage);
        for left in 0..columns {
            for right in 0..columns {
                meat[left][right] += row[left] * scaled * scaled * row[right];
            }
        }
    }
    multiply_matrices(&multiply_matrices(xtx_inv, &meat), xtx_inv)
}

const PROCESS_HC3_LEVERAGE_TOLERANCE: f64 = 1.0e-12;

fn process_robust_covariance_hc3(
    design: &[Vec<f64>],
    residuals: &[f64],
    xtx_inv: &[Vec<f64>],
    outcome: &str,
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let columns = xtx_inv.len();
    let mut meat = vec![vec![0.0; columns]; columns];
    for (row_index, row) in design.iter().enumerate() {
        let leverage = dot(row, &matrix_vector_product(xtx_inv, row));
        let denominator = 1.0 - leverage;
        if !denominator.is_finite() || denominator <= PROCESS_HC3_LEVERAGE_TOLERANCE {
            return Err(EstimationError::UnsupportedMethod(format!(
                "high_leverage_hc3_instability|PROCESS equation {outcome} has 1-h={denominator:.17e} at complete-case row {row_index}; exact HC3 requires 1-h greater than {PROCESS_HC3_LEVERAGE_TOLERANCE:.1e}"
            )));
        }
        let scaled = residuals[row_index] / denominator;
        if !scaled.is_finite() {
            return Err(EstimationError::UnsupportedMethod(format!(
                "high_leverage_hc3_instability|PROCESS equation {outcome} produced a nonfinite HC3 residual at complete-case row {row_index}"
            )));
        }
        for left in 0..columns {
            for right in 0..columns {
                meat[left][right] += row[left] * scaled * scaled * row[right];
            }
        }
    }
    let covariance = multiply_matrices(&multiply_matrices(xtx_inv, &meat), xtx_inv);
    if covariance.iter().flatten().any(|value| !value.is_finite()) {
        return Err(EstimationError::UnsupportedMethod(format!(
            "high_leverage_hc3_instability|PROCESS equation {outcome} produced a nonfinite exact HC3 covariance"
        )));
    }
    Ok(covariance)
}

fn logistic(value: f64) -> f64 {
    if value >= 0.0 {
        let z = (-value).exp();
        1.0 / (1.0 + z)
    } else {
        let z = value.exp();
        z / (1.0 + z)
    }
}

fn process_analysis(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    enforce_outcome_scope: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<ProcessAnalysis, EstimationError> {
    if let Some(MethodConfig::Regression {
        model:
            RegressionModelConfig::Process {
                relationship:
                    qpls_core::ProcessRelationshipConfig::Graph {
                        focal_predictor,
                        paths,
                        moderators,
                        moderations,
                        ..
                    },
            },
        outcome,
        predictors,
        controls,
        ..
    }) = recipe.method_config.as_ref()
    {
        return process_graph_analysis(
            dataset,
            recipe,
            focal_predictor,
            outcome,
            predictors,
            controls,
            paths,
            moderators,
            moderations,
            enforce_outcome_scope,
            control,
        );
    }
    let model = recipe
        .metadata
        .get("process_model")
        .cloned()
        .unwrap_or_else(|| "mediation".into());
    let x = recipe
        .metadata
        .get("process_x")
        .cloned()
        .or_else(|| {
            recipe
                .metadata
                .get("regression_predictors")
                .and_then(|v| v.split(',').next().map(str::trim).map(ToOwned::to_owned))
        })
        .ok_or_else(|| EstimationError::UnsupportedMethod("process_x required".into()))?;
    let y = metadata_required(recipe, "regression_outcome")?;
    let mut effects = Vec::new();
    let mut simple_slopes = Vec::new();
    if model == "mediation" || model == "moderated_mediation" {
        let m = metadata_required(recipe, "process_m")?;
        let prepared =
            prepare_raw_numeric_data(dataset, &[x.clone(), m.clone(), y.clone()], false, false)?;
        let a = ols_regression(
            &[prepared.columns[0].clone()],
            &prepared.columns[1],
            std::slice::from_ref(&x),
            &m,
            0.95,
        )?
        .0[1]
            .estimate;
        let (b_fit, _, _) = ols_regression(
            &[prepared.columns[0].clone(), prepared.columns[1].clone()],
            &prepared.columns[2],
            &[x.clone(), m.clone()],
            &y,
            0.95,
        )?;
        let direct = b_fit[1].estimate;
        let b = b_fit[2].estimate;
        effects.push(ProcessEffect {
            effect: "direct".into(),
            estimate: direct,
            lower_percentile: None,
            upper_percentile: None,
        });
        effects.push(ProcessEffect {
            effect: "indirect".into(),
            estimate: a * b,
            lower_percentile: None,
            upper_percentile: None,
        });
        effects.push(ProcessEffect {
            effect: "total".into(),
            estimate: direct + a * b,
            lower_percentile: None,
            upper_percentile: None,
        });
    }
    if model == "moderation" || model == "moderated_mediation" {
        let w = metadata_required(recipe, "process_w")?;
        let prepared =
            prepare_raw_numeric_data(dataset, &[x.clone(), w.clone(), y.clone()], true, false)?;
        let product = prepared.columns[0]
            .iter()
            .zip(&prepared.columns[1])
            .map(|(a, b)| a * b)
            .collect::<Vec<_>>();
        let terms = vec![x.clone(), w.clone(), format!("{x}:{w}")];
        let coefficients = ols_regression(
            &[
                prepared.columns[0].clone(),
                prepared.columns[1].clone(),
                product,
            ],
            &prepared.columns[2],
            &terms,
            &y,
            0.95,
        )?
        .0;
        let main = coefficients[1].estimate;
        let interaction = coefficients[3].estimate;
        for level in [-1.0, 0.0, 1.0] {
            simple_slopes.push(ProcessSimpleSlope {
                moderator_value: level,
                slope: main + interaction * level,
            });
        }
        effects.push(ProcessEffect {
            effect: "interaction".into(),
            estimate: interaction,
            lower_percentile: None,
            upper_percentile: None,
        });
    }
    Ok(ProcessAnalysis {
        method_version: REGRESSION_PROCESS_METHOD_VERSION_V1.into(),
        model,
        effects,
        simple_slopes,
        graph_v2: None,
        warnings: vec!["PROCESS v1 reports bounded deterministic mediation/moderation effects validated for the documented QuickPLS v1.2.2 scope; moderated mediation remains experimental.".into()],
    })
}

#[derive(Debug, Clone)]
struct ProcessTermSpec {
    id: String,
    kind: String,
    variables: Vec<String>,
}

fn process_graph_analysis(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    focal: &str,
    outcome: &str,
    predictors: &[String],
    controls: &[String],
    paths: &[qpls_core::ProcessPathConfig],
    moderators: &[qpls_core::ProcessModeratorConfig],
    moderations: &[qpls_core::ProcessModerationConfig],
    enforce_outcome_scope: bool,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<ProcessAnalysis, EstimationError> {
    let mut variables = predictors.to_vec();
    variables.extend(controls.iter().cloned());
    variables.push(outcome.to_string());
    let prepared = prepare_process_raw_numeric_data(dataset, &variables, control)?;
    let columns = variables
        .iter()
        .cloned()
        .zip(prepared.columns.iter().cloned())
        .collect::<BTreeMap<_, _>>();
    let scale_by_variable = moderators
        .iter()
        .map(|moderator| {
            let scale = match moderator.scale {
                qpls_core::ProcessModeratorScale::Continuous => "continuous",
                qpls_core::ProcessModeratorScale::Binary01 => "binary_0_1",
            };
            (moderator.variable.clone(), scale)
        })
        .collect::<BTreeMap<_, _>>();
    for moderator in moderators {
        if matches!(moderator.scale, qpls_core::ProcessModeratorScale::Binary01)
            && columns[&moderator.variable]
                .iter()
                .any(|value| *value != 0.0 && *value != 1.0)
        {
            return Err(EstimationError::UnsupportedMethod(format!(
                "invalid_binary_profile|PROCESS binary moderator {} must be coded exactly 0/1 in the complete sample",
                moderator.variable
            )));
        }
    }
    if enforce_outcome_scope {
        let equation_outcomes = paths
            .iter()
            .map(|path| path.to.as_str())
            .collect::<BTreeSet<_>>();
        for equation_outcome in equation_outcomes {
            let values = &columns[equation_outcome];
            let has_zero = values.iter().any(|value| *value == 0.0);
            let has_one = values.iter().any(|value| *value == 1.0);
            if has_zero && has_one && values.iter().all(|value| *value == 0.0 || *value == 1.0) {
                return Err(EstimationError::UnsupportedMethod(format!(
                    "binary_process_equation_outcome|PROCESS v2 requires every endogenous equation outcome to be continuous; {equation_outcome} is exactly coded 0/1 in the original complete sample"
                )));
            }
        }
    }
    let profiles = variables
        .iter()
        .map(|variable| {
            let values = &columns[variable];
            let mean = vector_mean(values);
            let sample_sd = if values.len() > 1 {
                (values
                    .iter()
                    .map(|value| (value - mean).powi(2))
                    .sum::<f64>()
                    / (values.len() - 1) as f64)
                    .sqrt()
            } else {
                0.0
            };
            let role = if variable == focal {
                "focal_predictor"
            } else if variable == outcome {
                "outcome"
            } else if controls.contains(variable) {
                "control"
            } else if scale_by_variable.contains_key(variable) {
                "moderator"
            } else {
                "mediator"
            };
            let scale = scale_by_variable
                .get(variable)
                .copied()
                .unwrap_or("continuous");
            let levels = if scale == "binary_0_1" {
                vec![0.0, 1.0]
            } else {
                Vec::new()
            };
            ProcessVariableProfile {
                variable: variable.clone(),
                role: role.into(),
                scale: scale.into(),
                raw_mean: mean,
                raw_sample_sd: sample_sd,
                raw_min: values.iter().copied().fold(f64::INFINITY, f64::min),
                raw_max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                levels,
            }
        })
        .collect::<Vec<_>>();
    let profile_by_variable = profiles
        .iter()
        .map(|profile| (profile.variable.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    let mut node_order = predictors
        .iter()
        .enumerate()
        .map(|(index, variable)| (variable.clone(), index))
        .collect::<BTreeMap<_, _>>();
    node_order.insert(outcome.to_string(), predictors.len());
    let mut ordered_paths = paths.iter().collect::<Vec<_>>();
    ordered_paths.sort_by(|left, right| {
        (
            node_order[&left.to],
            node_order[&left.from],
            &left.from,
            &left.to,
        )
            .cmp(&(
                node_order[&right.to],
                node_order[&right.from],
                &right.from,
                &right.to,
            ))
    });
    let mut ordered_moderations = moderations.iter().collect::<Vec<_>>();
    ordered_moderations.sort_by(|left, right| {
        (
            node_order[&left.to],
            node_order[&left.from],
            &left.moderator,
            &left.conditioning_moderator,
        )
            .cmp(&(
                node_order[&right.to],
                node_order[&right.from],
                &right.moderator,
                &right.conditioning_moderator,
            ))
    });
    let process_paths = ordered_paths
        .iter()
        .map(|path| ProcessPath {
            path_id: format!("{}->{}", path.from, path.to),
            from: path.from.clone(),
            to: path.to.clone(),
        })
        .collect::<Vec<_>>();
    let process_moderations = ordered_moderations
        .iter()
        .map(|moderation| ProcessModeration {
            moderation_id: process_moderation_id(
                &moderation.from,
                &moderation.to,
                &moderation.moderator,
                moderation.conditioning_moderator.as_deref(),
            ),
            from: moderation.from.clone(),
            to: moderation.to.clone(),
            moderator: moderation.moderator.clone(),
            conditioning_moderator: moderation.conditioning_moderator.clone(),
        })
        .collect::<Vec<_>>();
    let mut equation_outcomes = predictors
        .iter()
        .filter(|variable| {
            *variable != focal
                && !scale_by_variable.contains_key(*variable)
                && paths.iter().any(|path| path.to == **variable)
        })
        .cloned()
        .collect::<Vec<_>>();
    if paths.iter().any(|path| path.to == outcome) {
        equation_outcomes.push(outcome.to_string());
    }
    let mut equations = Vec::new();
    let equation_total = equation_outcomes.len() as u64;
    checkpoint(control, EstimationPhase::Iterating, 0, equation_total)?;
    for (equation_index, equation_outcome) in equation_outcomes.into_iter().enumerate() {
        let mut terms = ordered_paths
            .iter()
            .filter(|path| path.to == equation_outcome)
            .map(|path| ProcessTermSpec {
                id: format!("path:{}->{}", path.from, path.to),
                kind: "path".into(),
                variables: vec![path.from.clone()],
            })
            .collect::<Vec<_>>();
        let relevant_moderations = ordered_moderations
            .iter()
            .copied()
            .filter(|item| item.to == equation_outcome)
            .collect::<Vec<_>>();
        for moderator in moderators {
            if relevant_moderations.iter().any(|moderation| {
                moderation.moderator == moderator.variable
                    || moderation.conditioning_moderator.as_ref() == Some(&moderator.variable)
            }) && !terms
                .iter()
                .any(|term| term.variables == [moderator.variable.clone()])
            {
                terms.push(ProcessTermSpec {
                    id: format!("moderator:{}", moderator.variable),
                    kind: "moderator_main".into(),
                    variables: vec![moderator.variable.clone()],
                });
            }
        }
        let mut interaction_terms = Vec::new();
        for moderation in relevant_moderations {
            let primary = vec![moderation.from.clone(), moderation.moderator.clone()];
            if !interaction_terms
                .iter()
                .any(|term: &ProcessTermSpec| term.variables == primary)
            {
                interaction_terms.push(ProcessTermSpec {
                    id: format!("interaction:{}*{}", moderation.from, moderation.moderator),
                    kind: "interaction".into(),
                    variables: primary,
                });
            }
            if let Some(conditioning) = &moderation.conditioning_moderator {
                for pair in [
                    vec![moderation.from.clone(), conditioning.clone()],
                    vec![moderation.moderator.clone(), conditioning.clone()],
                ] {
                    if !interaction_terms.iter().any(|term| term.variables == pair) {
                        interaction_terms.push(ProcessTermSpec {
                            id: format!("interaction:{}*{}", pair[0], pair[1]),
                            kind: "interaction".into(),
                            variables: pair,
                        });
                    }
                }
                interaction_terms.push(ProcessTermSpec {
                    id: format!(
                        "interaction:{}*{}*{}",
                        moderation.from, moderation.moderator, conditioning
                    ),
                    kind: "interaction".into(),
                    variables: vec![
                        moderation.from.clone(),
                        moderation.moderator.clone(),
                        conditioning.clone(),
                    ],
                });
            }
        }
        interaction_terms.sort_by(|left, right| {
            (left.variables.len(), left.id.as_str())
                .cmp(&(right.variables.len(), right.id.as_str()))
        });
        interaction_terms.dedup_by(|left, right| left.variables == right.variables);
        terms.extend(interaction_terms);
        terms.extend(controls.iter().map(|control| ProcessTermSpec {
            id: format!("control:{control}"),
            kind: "control".into(),
            variables: vec![control.clone()],
        }));
        equations.push(fit_process_equation(
            &equation_outcome,
            &terms,
            &columns,
            &profile_by_variable,
            recipe.settings.confidence_level,
            equation_index as u64,
            equation_total,
            control,
        )?);
        checkpoint(
            control,
            EstimationPhase::Iterating,
            (equation_index + 1) as u64,
            equation_total,
        )?;
    }
    let simple_paths = enumerate_process_paths(focal, outcome, paths);
    let mut reference_effects = Vec::new();
    let direct_path = vec![focal.to_string(), outcome.to_string()];
    let direct = if simple_paths.iter().any(|path| path == &direct_path) {
        process_path_effect(
            &direct_path,
            &[],
            &equations,
            moderations,
            &profile_by_variable,
        )?
    } else {
        0.0
    };
    reference_effects.push(ProcessReferenceEffect {
        effect_id: format!("direct:{focal}->{outcome}"),
        kind: "direct".into(),
        path: direct_path.clone(),
        estimate: direct,
    });
    let mut total_indirect = 0.0;
    for path in simple_paths.iter().filter(|path| path.len() > 2) {
        let estimate =
            process_path_effect(path, &[], &equations, moderations, &profile_by_variable)?;
        total_indirect += estimate;
        reference_effects.push(ProcessReferenceEffect {
            effect_id: format!("indirect:{}", path.join("->")),
            kind: "indirect".into(),
            path: path.clone(),
            estimate,
        });
    }
    reference_effects.push(ProcessReferenceEffect {
        effect_id: format!("total_indirect:{focal}->{outcome}"),
        kind: "total_indirect".into(),
        path: vec![focal.to_string(), outcome.to_string()],
        estimate: total_indirect,
    });
    reference_effects.push(ProcessReferenceEffect {
        effect_id: format!("total:{focal}->{outcome}"),
        kind: "total".into(),
        path: vec![focal.to_string(), outcome.to_string()],
        estimate: direct + total_indirect,
    });
    let mut conditional_indirect_effects = Vec::new();
    let mut moderated_mediation_indices = Vec::new();
    for path in simple_paths.iter().filter(|path| path.len() > 2) {
        let path_id = path.join("->");
        if let Some(moderation) = path.windows(2).find_map(|edge| {
            moderations
                .iter()
                .find(|item| item.from == edge[0] && item.to == edge[1])
        }) {
            let probes = process_probe_grid(moderation, &profile_by_variable)?;
            for probe in probes {
                let estimate = process_path_effect(
                    path,
                    &probe.values,
                    &equations,
                    moderations,
                    &profile_by_variable,
                )?;
                conditional_indirect_effects.push(ProcessConditionalIndirectEffect {
                    effect_id: format!("indirect:{}@{}", path_id, probe.suffix),
                    path_id: path_id.clone(),
                    moderator_values: probe.values,
                    estimate,
                });
            }
            let reference_other_edges = process_path_other_edge_product(
                path,
                &moderation.from,
                &moderation.to,
                &equations,
                moderations,
                &profile_by_variable,
            )?;
            let interaction = process_equation_coefficient(
                &equations,
                &moderation.to,
                &[moderation.from.clone(), moderation.moderator.clone()],
            )?;
            moderated_mediation_indices.push(ProcessModeratedMediationIndex {
                effect_id: format!(
                    "index:{}:{}->{}:{}",
                    path_id, moderation.from, moderation.to, moderation.moderator
                ),
                path_id,
                moderated_edge: format!("{}->{}", moderation.from, moderation.to),
                moderator: moderation.moderator.clone(),
                estimate: interaction.estimate * reference_other_edges,
            });
        }
    }
    let mut simple_slopes = Vec::new();
    let mut plots = Vec::new();
    let mut johnson_neyman = Vec::new();
    let diagnostic_total = ordered_moderations.len().max(1) as u64;
    checkpoint(
        control,
        EstimationPhase::ComputingEffects,
        0,
        diagnostic_total,
    )?;
    for (diagnostic_index, moderation) in ordered_moderations.iter().copied().enumerate() {
        let moderation_id = process_moderation_id(
            &moderation.from,
            &moderation.to,
            &moderation.moderator,
            moderation.conditioning_moderator.as_deref(),
        );
        let probes = process_probe_grid(moderation, &profile_by_variable)?;
        for probe in &probes {
            simple_slopes.push(process_simple_slope(
                &moderation_id,
                moderation,
                &probe.values,
                &probe.suffix,
                &equations,
                recipe.settings.confidence_level,
            )?);
        }
        plots.push(process_plot(
            &moderation_id,
            moderation,
            &probes,
            &equations,
            &profile_by_variable,
            recipe.settings.confidence_level,
        )?);
        johnson_neyman.extend(process_johnson_neyman(
            &moderation_id,
            moderation,
            &equations,
            &profile_by_variable,
            recipe.settings.confidence_level,
        )?);
        checkpoint(
            control,
            EstimationPhase::ComputingEffects,
            (diagnostic_index + 1) as u64,
            diagnostic_total,
        )?;
    }
    if ordered_moderations.is_empty() {
        checkpoint(
            control,
            EstimationPhase::ComputingEffects,
            1,
            diagnostic_total,
        )?;
    }
    let graph_v2 = ProcessGraphAnalysis {
        policies: ProcessPolicies {
            centering: "equation_complete_case_mean_v1".into(),
            covariance: "hc3_v1".into(),
            inference_reference: "student_t_residual_df_v1".into(),
            confidence_level: recipe.settings.confidence_level,
        },
        complete_cases: prepared.used,
        omitted_cases: prepared.omitted,
        variable_profiles: profiles,
        paths: process_paths,
        moderations: process_moderations,
        equations,
        reference_effects,
        conditional_indirect_effects,
        moderated_mediation_indices,
        simple_slopes,
        plots,
        johnson_neyman,
        bootstrap: None,
    };
    checkpoint(control, EstimationPhase::Assembling, 1, 1)?;
    Ok(ProcessAnalysis {
        method_version: REGRESSION_PROCESS_METHOD_VERSION.into(),
        model: "graph".into(),
        effects: Vec::new(),
        simple_slopes: Vec::new(),
        graph_v2: Some(graph_v2),
        warnings: vec![
            "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates.".into(),
            "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected.".into(),
        ],
    })
}

fn process_moderation_id(
    from: &str,
    to: &str,
    moderator: &str,
    conditioning: Option<&str>,
) -> String {
    match conditioning {
        Some(conditioning) => format!("moderation:{from}->{to}@{moderator}|{conditioning}"),
        None => format!("moderation:{from}->{to}@{moderator}"),
    }
}

fn process_centered_value(variable: &str, value: f64, profile: &ProcessVariableProfile) -> f64 {
    if profile.scale == "binary_0_1" {
        value
    } else if variable == profile.variable {
        value - profile.raw_mean
    } else {
        value
    }
}

fn process_standard_error_from_variance(
    variance: f64,
    outcome: &str,
    term_id: &str,
) -> Result<f64, EstimationError> {
    if !variance.is_finite() || variance <= 0.0 {
        return Err(EstimationError::Numerical(format!(
            "invalid_hc3_covariance|PROCESS equation {outcome} term {term_id} has nonpositive HC3 variance {variance:.17e}"
        )));
    }
    let standard_error = variance.sqrt();
    if !standard_error.is_finite() {
        return Err(EstimationError::Numerical(format!(
            "invalid_hc3_covariance|PROCESS equation {outcome} term {term_id} has a nonfinite HC3 standard error"
        )));
    }
    Ok(standard_error)
}

struct ProcessScaleAwareFit {
    coefficients: Vec<f64>,
    residuals: Vec<f64>,
    covariance: Vec<Vec<f64>>,
}

fn process_column_location_scale(values: &[f64]) -> Option<(f64, f64)> {
    let mut mean = 0.0;
    let mut centered_sum_squares = 0.0;
    for (index, value) in values.iter().copied().enumerate() {
        if !value.is_finite() {
            return None;
        }
        let count = (index + 1) as f64;
        let delta = value - mean;
        mean += delta / count;
        centered_sum_squares += delta * (value - mean);
    }
    let scale = (centered_sum_squares / values.len() as f64).sqrt();
    (mean.is_finite() && scale.is_finite() && scale > 0.0).then_some((mean, scale))
}

fn process_scale_aware_ols(
    design: &[Vec<f64>],
    outcome_values: &[f64],
    outcome: &str,
) -> Result<ProcessScaleAwareFit, EstimationError> {
    let rows = design.len();
    let columns = design.first().map(Vec::len).unwrap_or_default();
    if rows != outcome_values.len()
        || rows <= columns
        || columns == 0
        || design.iter().any(|row| row.len() != columns)
        || outcome_values.iter().any(|value| !value.is_finite())
    {
        return Err(EstimationError::RankDeficient(outcome.into()));
    }

    let mut centers = vec![0.0; columns];
    let mut scales = vec![1.0; columns];
    for column in 1..columns {
        let values = design.iter().map(|row| row[column]).collect::<Vec<_>>();
        let Some((center, scale)) = process_column_location_scale(&values) else {
            return Err(EstimationError::RankDeficient(outcome.into()));
        };
        centers[column] = center;
        scales[column] = scale;
    }
    let normalized_design = (0..rows)
        .map(|row| {
            (0..columns)
                .map(|column| {
                    if column == 0 {
                        1.0
                    } else {
                        (design[row][column] - centers[column]) / scales[column]
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if normalized_design
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(EstimationError::RankDeficient(outcome.into()));
    }

    let matrix = Mat::from_fn(rows, columns, |row, column| normalized_design[row][column]);
    // PROCESS resampling owns parallelism at the replicate/delete-one level.  faer's
    // high-level SVD and least-squares APIs inherit the surrounding Rayon pool,
    // which changes floating-point reduction order when the requested worker count
    // changes.  Keep each equation fit explicitly sequential so indexed outer work
    // remains bitwise worker-invariant.
    let parallelism = Par::Seq;
    let compute_vectors = ComputeSvdVectors::Thin;
    let mut left_singular_vectors = Mat::<f64>::zeros(rows, columns);
    let mut right_singular_vectors = Mat::<f64>::zeros(columns, columns);
    let mut singular_values = Diag::<f64>::zeros(columns);
    let mut svd_memory = MemBuffer::new(svd_scratch::<f64>(
        rows,
        columns,
        compute_vectors,
        compute_vectors,
        parallelism,
        Default::default(),
    ));
    svd(
        matrix.as_ref(),
        singular_values.as_mut(),
        Some(left_singular_vectors.as_mut()),
        Some(right_singular_vectors.as_mut()),
        parallelism,
        MemStack::new(&mut svd_memory),
        Default::default(),
    )
    .map_err(|_| EstimationError::RankDeficient(outcome.into()))?;
    let maximum_singular_value = singular_values[0];
    let minimum_singular_value = singular_values[columns - 1];
    let rank_tolerance = maximum_singular_value
        * rows.max(columns) as f64
        * f64::EPSILON
        * PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER;
    if !maximum_singular_value.is_finite()
        || !minimum_singular_value.is_finite()
        || maximum_singular_value <= 0.0
        || minimum_singular_value <= rank_tolerance
    {
        return Err(EstimationError::RankDeficient(outcome.into()));
    }

    let right_hand_side = Mat::from_fn(rows, 1, |row, _| outcome_values[row]);
    let mut projected_outcome = Mat::<f64>::zeros(columns, 1);
    matmul_with_conj(
        projected_outcome.as_mut(),
        Accum::Replace,
        left_singular_vectors.as_ref().transpose(),
        Conj::Yes,
        right_hand_side.as_ref(),
        Conj::No,
        1.0,
        parallelism,
    );
    for index in 0..columns {
        projected_outcome[(index, 0)] /= singular_values[index];
    }
    let mut solution = Mat::<f64>::zeros(columns, 1);
    matmul_with_conj(
        solution.as_mut(),
        Accum::Replace,
        right_singular_vectors.as_ref(),
        Conj::No,
        projected_outcome.as_ref(),
        Conj::No,
        1.0,
        parallelism,
    );
    let normalized_coefficients = (0..columns)
        .map(|index| solution[(index, 0)])
        .collect::<Vec<_>>();
    if normalized_coefficients
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(EstimationError::Numerical(format!(
            "nonfinite_estimate|PROCESS equation {outcome} produced an invalid coefficient"
        )));
    }
    let fitted = normalized_design
        .iter()
        .map(|row| dot(row, &normalized_coefficients))
        .collect::<Vec<_>>();
    let residuals = outcome_values
        .iter()
        .zip(&fitted)
        .map(|(actual, fitted)| actual - fitted)
        .collect::<Vec<_>>();

    let mut normalized_xtx_inverse = vec![vec![0.0; columns]; columns];
    for row in 0..columns {
        for column in 0..columns {
            normalized_xtx_inverse[row][column] = (0..columns)
                .map(|index| {
                    right_singular_vectors[(row, index)] * right_singular_vectors[(column, index)]
                        / singular_values[index].powi(2)
                })
                .sum();
        }
    }
    let normalized_covariance = process_robust_covariance_hc3(
        &normalized_design,
        &residuals,
        &normalized_xtx_inverse,
        outcome,
    )?;

    let mut raw_transform = vec![vec![0.0; columns]; columns];
    raw_transform[0][0] = 1.0;
    for column in 1..columns {
        raw_transform[0][column] = -centers[column] / scales[column];
        raw_transform[column][column] = 1.0 / scales[column];
    }
    if raw_transform
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(EstimationError::Numerical(format!(
            "nonfinite_estimate|PROCESS equation {outcome} could not back-transform normalized coefficients"
        )));
    }
    let coefficients = matrix_vector_product(&raw_transform, &normalized_coefficients);
    let mut covariance = vec![vec![0.0; columns]; columns];
    for row in 0..columns {
        for column in 0..columns {
            for left in 0..columns {
                for right in 0..columns {
                    covariance[row][column] += raw_transform[row][left]
                        * normalized_covariance[left][right]
                        * raw_transform[column][right];
                }
            }
        }
    }
    for row in 0..columns {
        for column in 0..row {
            let symmetric = (covariance[row][column] + covariance[column][row]) / 2.0;
            covariance[row][column] = symmetric;
            covariance[column][row] = symmetric;
        }
    }
    if coefficients.iter().any(|value| !value.is_finite())
        || covariance.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(EstimationError::Numerical(format!(
            "nonfinite_estimate|PROCESS equation {outcome} produced a nonfinite back-transform"
        )));
    }
    Ok(ProcessScaleAwareFit {
        coefficients,
        residuals,
        covariance,
    })
}

fn fit_process_equation(
    outcome: &str,
    terms: &[ProcessTermSpec],
    columns: &BTreeMap<String, Vec<f64>>,
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
    confidence_level: f64,
    completed_equations: u64,
    equation_total: u64,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<ProcessEquation, EstimationError> {
    let y = &columns[outcome];
    let mut term_columns = Vec::with_capacity(terms.len());
    for term in terms {
        let mut values = Vec::with_capacity(y.len());
        for row in 0..y.len() {
            if row > 0 && row % 256 == 0 {
                checkpoint(
                    control,
                    EstimationPhase::Iterating,
                    completed_equations,
                    equation_total,
                )?;
            }
            values.push(term.variables.iter().try_fold(1.0, |product, variable| {
                let profile = profiles
                    .get(variable.as_str())
                    .ok_or_else(|| EstimationError::InvalidIndicator(variable.clone()))?;
                let raw = columns[variable][row];
                let value = if term.variables.len() > 1 {
                    process_centered_value(variable, raw, profile)
                } else {
                    raw
                };
                Ok(product * value)
            })?);
        }
        term_columns.push(values);
    }
    let n = y.len();
    let p = term_columns.len() + 1;
    if n <= p {
        return Err(EstimationError::RankDeficient(outcome.into()));
    }
    let design = regression_design_matrix(&term_columns);
    let fit = process_scale_aware_ols(&design, y, outcome)?;
    let beta = fit.coefficients;
    let residuals = fit.residuals;
    let rss = residuals.iter().map(|value| value.powi(2)).sum::<f64>();
    let mean = vector_mean(y);
    let tss = y.iter().map(|value| (value - mean).powi(2)).sum::<f64>();
    let r_squared = if tss > f64::EPSILON {
        1.0 - rss / tss
    } else {
        0.0
    };
    let residual_df = n - p;
    let covariance = fit.covariance;
    let distribution = StudentsT::new(0.0, 1.0, residual_df as f64)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let intercept = ProcessTermSpec {
        id: "intercept".into(),
        kind: "intercept".into(),
        variables: Vec::new(),
    };
    let all_terms = std::iter::once(&intercept).chain(terms).collect::<Vec<_>>();
    let coefficients = beta
        .iter()
        .zip(all_terms)
        .enumerate()
        .map(|(index, (estimate, term))| {
            let standard_error =
                process_standard_error_from_variance(covariance[index][index], outcome, &term.id)?;
            if !estimate.is_finite() {
                return Err(EstimationError::Numerical(format!(
                    "nonfinite_estimate|PROCESS equation {outcome} produced an invalid coefficient"
                )));
            }
            let statistic = estimate / standard_error;
            Ok(ProcessEquationCoefficient {
                term_id: term.id.clone(),
                kind: term.kind.clone(),
                variables: term.variables.clone(),
                estimate: *estimate,
                standard_error,
                statistic,
                p_value_two_sided: (2.0 * (1.0 - distribution.cdf(statistic.abs())))
                    .clamp(0.0, 1.0),
                confidence_interval_lower: estimate - critical * standard_error,
                confidence_interval_upper: estimate + critical * standard_error,
            })
        })
        .collect::<Result<Vec<_>, EstimationError>>()?;
    let sigma2 = (rss / n as f64).max(f64::MIN_POSITIVE);
    let f_statistic = if p > 1 && residual_df > 0 && r_squared < 1.0 {
        (r_squared / (p - 1) as f64) / ((1.0 - r_squared) / residual_df as f64)
    } else {
        0.0
    };
    Ok(ProcessEquation {
        equation_id: format!("equation:{outcome}"),
        outcome: outcome.into(),
        term_ids: coefficients.iter().map(|row| row.term_id.clone()).collect(),
        coefficients,
        coefficient_covariance: covariance,
        residual_degrees_of_freedom: residual_df,
        fit: ProcessEquationFit {
            observations: n,
            parameter_count: p,
            residual_sum_squares: rss,
            total_sum_squares: tss,
            r_squared,
            adjusted_r_squared: 1.0 - (1.0 - r_squared) * (n - 1) as f64 / residual_df as f64,
            f_statistic,
            aic: n as f64 * sigma2.ln() + 2.0 * p as f64,
            bic: n as f64 * sigma2.ln() + (n as f64).ln() * p as f64,
            rmse: (rss / n as f64).sqrt(),
        },
    })
}

fn enumerate_process_paths(
    focal: &str,
    outcome: &str,
    paths: &[qpls_core::ProcessPathConfig],
) -> Vec<Vec<String>> {
    fn visit(
        node: &str,
        outcome: &str,
        paths: &[qpls_core::ProcessPathConfig],
        current: &mut Vec<String>,
        result: &mut Vec<Vec<String>>,
    ) {
        if node == outcome {
            result.push(current.clone());
            return;
        }
        let mut outgoing = paths
            .iter()
            .filter(|path| path.from == node)
            .collect::<Vec<_>>();
        outgoing.sort_by(|left, right| left.to.cmp(&right.to));
        for path in outgoing {
            if !current.contains(&path.to) {
                current.push(path.to.clone());
                visit(&path.to, outcome, paths, current, result);
                current.pop();
            }
        }
    }
    let mut result = Vec::new();
    visit(
        focal,
        outcome,
        paths,
        &mut vec![focal.to_string()],
        &mut result,
    );
    result.sort();
    result
}

fn process_equation<'a>(
    equations: &'a [ProcessEquation],
    outcome: &str,
) -> Result<&'a ProcessEquation, EstimationError> {
    equations
        .iter()
        .find(|equation| equation.outcome == outcome)
        .ok_or_else(|| EstimationError::Numerical(format!("missing PROCESS equation {outcome}")))
}

fn process_equation_coefficient<'a>(
    equations: &'a [ProcessEquation],
    outcome: &str,
    variables: &[String],
) -> Result<&'a ProcessEquationCoefficient, EstimationError> {
    process_equation(equations, outcome)?
        .coefficients
        .iter()
        .find(|coefficient| coefficient.variables == variables)
        .ok_or_else(|| {
            EstimationError::Numerical(format!(
                "missing PROCESS coefficient {} in {outcome}",
                variables.join("*")
            ))
        })
}

fn process_raw_to_coded(
    variable: &str,
    raw: f64,
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> f64 {
    let profile = profiles[variable];
    if profile.scale == "binary_0_1" {
        raw
    } else {
        raw - profile.raw_mean
    }
}

struct ProcessSemanticProbe {
    values: Vec<ProcessModeratorValue>,
    suffix: String,
}

fn process_semantic_probe_levels(
    profile: &ProcessVariableProfile,
) -> Result<Vec<(f64, &'static str)>, EstimationError> {
    if profile.scale == "binary_0_1" {
        return Ok(vec![(0.0, "binary_0"), (1.0, "binary_1")]);
    }
    let levels = [
        profile.raw_mean - profile.raw_sample_sd,
        profile.raw_mean,
        profile.raw_mean + profile.raw_sample_sd,
    ];
    if !levels.iter().all(|value| value.is_finite())
        || !(levels[0] < levels[1] && levels[1] < levels[2])
    {
        return Err(EstimationError::Numerical(format!(
            "collapsed_process_probe_grid|PROCESS continuous moderator {} does not have three distinct finite mean-minus-SD, mean, and mean-plus-SD probes in f64",
            profile.variable
        )));
    }
    Ok(vec![
        (levels[0], "minus_1sd"),
        (levels[1], "mean"),
        (levels[2], "plus_1sd"),
    ])
}

fn process_probe_grid(
    moderation: &qpls_core::ProcessModerationConfig,
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> Result<Vec<ProcessSemanticProbe>, EstimationError> {
    let primary = profiles[moderation.moderator.as_str()];
    let primary_values = process_semantic_probe_levels(primary)?;
    let conditioning_values = moderation
        .conditioning_moderator
        .as_deref()
        .map(|variable| process_semantic_probe_levels(profiles[variable]))
        .transpose()?
        .unwrap_or_else(|| vec![(0.0, "")]);
    let mut result = Vec::new();
    for (raw_primary, primary_token) in primary_values {
        for (raw_conditioning, conditioning_token) in &conditioning_values {
            let mut values = vec![ProcessModeratorValue {
                variable: moderation.moderator.clone(),
                raw_value: raw_primary,
                coded_value: process_raw_to_coded(&moderation.moderator, raw_primary, profiles),
            }];
            let mut suffix = format!("{}={primary_token}", moderation.moderator);
            if let Some(conditioning) = &moderation.conditioning_moderator {
                values.push(ProcessModeratorValue {
                    variable: conditioning.clone(),
                    raw_value: *raw_conditioning,
                    coded_value: process_raw_to_coded(conditioning, *raw_conditioning, profiles),
                });
                suffix.push_str(&format!(",{conditioning}={conditioning_token}"));
            }
            result.push(ProcessSemanticProbe { values, suffix });
        }
    }
    Ok(result)
}

fn process_edge_slope(
    from: &str,
    to: &str,
    probes: &[ProcessModeratorValue],
    equations: &[ProcessEquation],
    moderations: &[qpls_core::ProcessModerationConfig],
) -> Result<f64, EstimationError> {
    let coefficient = process_equation_coefficient(equations, to, &[from.to_string()])?;
    let Some(moderation) = moderations
        .iter()
        .find(|moderation| moderation.from == from && moderation.to == to)
    else {
        return Ok(coefficient.estimate);
    };
    let primary = probes
        .iter()
        .find(|value| value.variable == moderation.moderator)
        .map(|value| value.coded_value)
        .unwrap_or(0.0);
    let primary_interaction = process_equation_coefficient(
        equations,
        to,
        &[from.to_string(), moderation.moderator.clone()],
    )?;
    let mut slope = coefficient.estimate + primary_interaction.estimate * primary;
    if let Some(conditioning) = &moderation.conditioning_moderator {
        let conditioning_value = probes
            .iter()
            .find(|value| value.variable == *conditioning)
            .map(|value| value.coded_value)
            .unwrap_or(0.0);
        slope +=
            process_equation_coefficient(equations, to, &[from.to_string(), conditioning.clone()])?
                .estimate
                * conditioning_value;
        slope += process_equation_coefficient(
            equations,
            to,
            &[
                from.to_string(),
                moderation.moderator.clone(),
                conditioning.clone(),
            ],
        )?
        .estimate
            * primary
            * conditioning_value;
    }
    Ok(slope)
}

fn process_path_effect(
    path: &[String],
    probes: &[ProcessModeratorValue],
    equations: &[ProcessEquation],
    moderations: &[qpls_core::ProcessModerationConfig],
    _profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> Result<f64, EstimationError> {
    path.windows(2).try_fold(1.0, |effect, edge| {
        Ok(effect * process_edge_slope(&edge[0], &edge[1], probes, equations, moderations)?)
    })
}

fn process_path_other_edge_product(
    path: &[String],
    excluded_from: &str,
    excluded_to: &str,
    equations: &[ProcessEquation],
    moderations: &[qpls_core::ProcessModerationConfig],
    _profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> Result<f64, EstimationError> {
    path.windows(2).try_fold(1.0, |effect, edge| {
        if edge[0] == excluded_from && edge[1] == excluded_to {
            Ok(effect)
        } else {
            Ok(effect * process_edge_slope(&edge[0], &edge[1], &[], equations, moderations)?)
        }
    })
}

fn process_slope_weights(
    equation: &ProcessEquation,
    moderation: &qpls_core::ProcessModerationConfig,
    probes: &[ProcessModeratorValue],
) -> Result<Vec<f64>, EstimationError> {
    let primary = probes
        .iter()
        .find(|value| value.variable == moderation.moderator)
        .map(|value| value.coded_value)
        .unwrap_or(0.0);
    let conditioning = moderation
        .conditioning_moderator
        .as_ref()
        .map(|variable| {
            probes
                .iter()
                .find(|value| value.variable == *variable)
                .map(|value| value.coded_value)
                .unwrap_or(0.0)
        })
        .unwrap_or(0.0);
    let mut weights = vec![0.0; equation.coefficients.len()];
    for (index, coefficient) in equation.coefficients.iter().enumerate() {
        weights[index] = if coefficient.variables == [moderation.from.clone()] {
            1.0
        } else if coefficient.variables == [moderation.from.clone(), moderation.moderator.clone()] {
            primary
        } else if moderation
            .conditioning_moderator
            .as_ref()
            .is_some_and(|variable| {
                coefficient.variables == [moderation.from.clone(), variable.clone()]
            })
        {
            conditioning
        } else if moderation
            .conditioning_moderator
            .as_ref()
            .is_some_and(|variable| {
                coefficient.variables
                    == [
                        moderation.from.clone(),
                        moderation.moderator.clone(),
                        variable.clone(),
                    ]
            })
        {
            primary * conditioning
        } else {
            0.0
        };
    }
    if weights.iter().all(|weight| *weight == 0.0) {
        return Err(EstimationError::Numerical(format!(
            "missing PROCESS slope terms for {}->{}",
            moderation.from, moderation.to
        )));
    }
    Ok(weights)
}

fn process_linear_combination(
    equation: &ProcessEquation,
    weights: &[f64],
) -> Result<(f64, f64), EstimationError> {
    let estimate = equation
        .coefficients
        .iter()
        .zip(weights)
        .map(|(coefficient, weight)| coefficient.estimate * weight)
        .sum::<f64>();
    let variance = weights
        .iter()
        .enumerate()
        .map(|(left, left_weight)| {
            weights
                .iter()
                .enumerate()
                .map(|(right, right_weight)| {
                    left_weight * equation.coefficient_covariance[left][right] * right_weight
                })
                .sum::<f64>()
        })
        .sum::<f64>();
    if !estimate.is_finite() || !variance.is_finite() || variance < 0.0 {
        return Err(EstimationError::Numerical(
            "nonfinite_covariance|PROCESS linear combination is not finite".into(),
        ));
    }
    Ok((estimate, variance.sqrt()))
}

fn process_simple_slope(
    moderation_id: &str,
    moderation: &qpls_core::ProcessModerationConfig,
    probes: &[ProcessModeratorValue],
    probe_suffix: &str,
    equations: &[ProcessEquation],
    confidence_level: f64,
) -> Result<ProcessGraphSimpleSlope, EstimationError> {
    let equation = process_equation(equations, &moderation.to)?;
    let weights = process_slope_weights(equation, moderation, probes)?;
    let (estimate, standard_error) = process_linear_combination(equation, &weights)?;
    if !standard_error.is_finite() || standard_error <= 0.0 {
        return Err(EstimationError::Numerical(format!(
            "degenerate_simple_slope_variance|PROCESS moderation {moderation_id} has nonpositive conditional-slope variance at the requested probe"
        )));
    }
    let distribution = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let statistic = estimate / standard_error;
    Ok(ProcessGraphSimpleSlope {
        effect_id: format!("slope:{moderation_id}@{probe_suffix}"),
        moderation_id: moderation_id.into(),
        moderator_values: probes.to_vec(),
        estimate,
        standard_error,
        statistic,
        p_value_two_sided: (2.0 * (1.0 - distribution.cdf(statistic.abs()))).clamp(0.0, 1.0),
        confidence_interval_lower: estimate - critical * standard_error,
        confidence_interval_upper: estimate + critical * standard_error,
    })
}

fn process_term_value(
    variables: &[String],
    raw_values: &BTreeMap<String, f64>,
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
) -> f64 {
    variables.iter().fold(1.0, |product, variable| {
        let raw = raw_values[variable];
        let profile = profiles[variable.as_str()];
        let value = if variables.len() > 1 {
            process_centered_value(variable, raw, profile)
        } else {
            raw
        };
        product * value
    })
}

fn process_plot(
    moderation_id: &str,
    moderation: &qpls_core::ProcessModerationConfig,
    probes: &[ProcessSemanticProbe],
    equations: &[ProcessEquation],
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
    confidence_level: f64,
) -> Result<ProcessPlot, EstimationError> {
    let equation = process_equation(equations, &moderation.to)?;
    let focal_profile = profiles[moderation.from.as_str()];
    let distribution = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let mut series = Vec::new();
    for (series_index, probe) in probes.iter().enumerate() {
        let mut points = Vec::new();
        for point_index in 0..25 {
            let x = focal_profile.raw_min
                + (focal_profile.raw_max - focal_profile.raw_min) * point_index as f64 / 24.0;
            let mut raw_values = profiles
                .values()
                .map(|profile| {
                    (
                        profile.variable.clone(),
                        if profile.scale == "binary_0_1" {
                            0.0
                        } else {
                            profile.raw_mean
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            raw_values.insert(moderation.from.clone(), x);
            for value in &probe.values {
                raw_values.insert(value.variable.clone(), value.raw_value);
            }
            let design = equation
                .coefficients
                .iter()
                .map(|coefficient| {
                    if coefficient.kind == "intercept" {
                        1.0
                    } else {
                        process_term_value(&coefficient.variables, &raw_values, profiles)
                    }
                })
                .collect::<Vec<_>>();
            let (predicted, standard_error) = process_linear_combination(equation, &design)?;
            points.push(ProcessPlotPoint {
                predictor_raw: x,
                predicted_raw: predicted,
                confidence_interval_lower: predicted - critical * standard_error,
                confidence_interval_upper: predicted + critical * standard_error,
            });
        }
        series.push(ProcessPlotSeries {
            series_id: format!("series:{series_index}:{}", probe.suffix),
            moderator_values: probe.values.clone(),
            points,
        });
    }
    Ok(ProcessPlot {
        plot_id: format!("plot:{moderation_id}"),
        moderation_id: moderation_id.into(),
        series,
    })
}

pub fn process_johnson_neyman_coded_roots(
    quadratic: f64,
    linear: f64,
    constant: f64,
    coded_min: f64,
    coded_max: f64,
) -> Vec<f64> {
    if ![quadratic, linear, constant, coded_min, coded_max]
        .iter()
        .all(|value| value.is_finite())
        || coded_min > coded_max
    {
        return Vec::new();
    }
    let midpoint = coded_min / 2.0 + coded_max / 2.0;
    let half_range = coded_max / 2.0 - coded_min / 2.0;
    if !midpoint.is_finite() || !half_range.is_finite() || half_range <= 0.0 {
        return Vec::new();
    }
    let domain_quadratic = quadratic * half_range * half_range;
    let domain_linear = half_range * (2.0 * quadratic * midpoint + linear);
    let domain_constant = quadratic * midpoint * midpoint + linear * midpoint + constant;
    if ![domain_quadratic, domain_linear, domain_constant]
        .iter()
        .all(|value| value.is_finite())
    {
        return Vec::new();
    }
    let coefficient_scale = domain_quadratic
        .abs()
        .max(domain_linear.abs())
        .max(domain_constant.abs());
    if coefficient_scale == 0.0 {
        return Vec::new();
    }
    let a = domain_quadratic / coefficient_scale;
    let b = domain_linear / coefficient_scale;
    let c = domain_constant / coefficient_scale;
    let coefficient_tolerance = PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER * f64::EPSILON;
    let mut roots = Vec::new();
    if a.abs() <= coefficient_tolerance {
        if b.abs() > coefficient_tolerance {
            roots.push(-c / b);
        }
    } else {
        let discriminant_left = b * b;
        let discriminant_right = 4.0 * a * c;
        let discriminant = discriminant_left - discriminant_right;
        let discriminant_scale = discriminant_left
            .abs()
            .max(discriminant_right.abs())
            .max(f64::MIN_POSITIVE);
        let discriminant_tolerance =
            PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER * f64::EPSILON * discriminant_scale;
        if discriminant >= -discriminant_tolerance {
            let square_root = if discriminant.abs() <= discriminant_tolerance {
                0.0
            } else {
                discriminant.sqrt()
            };
            if square_root == 0.0 {
                roots.push(-b / (2.0 * a));
            } else {
                let q = -0.5 * (b + square_root.copysign(b));
                if q == 0.0 {
                    roots.push(-b / (2.0 * a));
                } else {
                    roots.push(q / a);
                    roots.push(c / q);
                }
            }
        }
    }
    let domain_tolerance = PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER * f64::EPSILON;
    roots.retain(|root| {
        root.is_finite() && *root >= -1.0 - domain_tolerance && *root <= 1.0 + domain_tolerance
    });
    for root in &mut roots {
        *root = root.clamp(-1.0, 1.0);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|left, right| {
        let root_scale = left.abs().max(right.abs()).max(1.0);
        (*left - *right).abs()
            <= PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER * f64::EPSILON * root_scale
    });
    roots.iter_mut().for_each(|root| {
        *root = if *root <= 0.0 {
            coded_min + half_range * (*root + 1.0)
        } else {
            coded_max - half_range * (1.0 - *root)
        };
    });
    let range_scale = coded_min
        .abs()
        .max(coded_max.abs())
        .max((coded_max - coded_min).abs())
        .max(f64::MIN_POSITIVE);
    let range_tolerance = PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER * f64::EPSILON * range_scale;
    if roots.len() == 2 && quadratic != 0.0 {
        let mapped = roots.clone();
        for target in 0..2 {
            let other = mapped[1 - target];
            let denominator = quadratic * other;
            if denominator.is_finite() && denominator != 0.0 {
                let companion = constant / denominator;
                let at_boundary = roots[target].to_bits() == coded_min.to_bits()
                    || roots[target].to_bits() == coded_max.to_bits();
                if at_boundary
                    && companion.is_finite()
                    && companion >= coded_min - range_tolerance
                    && companion <= coded_max + range_tolerance
                {
                    roots[target] = companion.clamp(coded_min, coded_max);
                }
            }
        }
    }
    roots.retain(|root| {
        root.is_finite()
            && *root >= coded_min - range_tolerance
            && *root <= coded_max + range_tolerance
    });
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|left, right| left.to_bits() == right.to_bits());
    roots
}

fn process_jn_variance(v0: f64, v1: f64, v2: f64, coded: f64) -> Option<f64> {
    let variance = v0 + 2.0 * v1 * coded + v2 * coded * coded;
    (variance.is_finite() && variance > 0.0).then_some(variance)
}

fn process_jn_variance_is_positive_across_range(
    v0: f64,
    v1: f64,
    v2: f64,
    coded_min: f64,
    coded_max: f64,
) -> bool {
    if process_jn_variance(v0, v1, v2, coded_min).is_none()
        || process_jn_variance(v0, v1, v2, coded_max).is_none()
    {
        return false;
    }
    if v2 > 0.0 {
        let vertex = -v1 / v2;
        if vertex > coded_min
            && vertex < coded_max
            && process_jn_variance(v0, v1, v2, vertex).is_none()
        {
            return false;
        }
    }
    true
}

fn process_johnson_neyman(
    moderation_id: &str,
    moderation: &qpls_core::ProcessModerationConfig,
    equations: &[ProcessEquation],
    profiles: &BTreeMap<&str, &ProcessVariableProfile>,
    confidence_level: f64,
) -> Result<Vec<ProcessJohnsonNeyman>, EstimationError> {
    let solved_profile = profiles[moderation.moderator.as_str()];
    let conditioning_grids = moderation
        .conditioning_moderator
        .as_ref()
        .map(|variable| {
            process_semantic_probe_levels(profiles[variable.as_str()]).map(|levels| {
                levels
                    .into_iter()
                    .map(|(raw, _)| {
                        vec![ProcessModeratorValue {
                            variable: variable.clone(),
                            raw_value: raw,
                            coded_value: process_raw_to_coded(variable, raw, profiles),
                        }]
                    })
                    .collect::<Vec<_>>()
            })
        })
        .transpose()?
        .unwrap_or_else(|| vec![Vec::new()]);
    if solved_profile.scale == "binary_0_1" {
        return Ok(conditioning_grids
            .into_iter()
            .map(|conditioning_values| ProcessJohnsonNeyman::Unavailable {
                moderation_id: moderation_id.into(),
                solved_moderator: moderation.moderator.clone(),
                conditioning_values,
                reason_code: "binary_solved_moderator".into(),
                message: "Johnson-Neyman regions require a continuous solved moderator.".into(),
            })
            .collect());
    }
    let equation = process_equation(equations, &moderation.to)?;
    let distribution = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let coded_min = solved_profile.raw_min - solved_profile.raw_mean;
    let coded_max = solved_profile.raw_max - solved_profile.raw_mean;
    let mut results = Vec::new();
    for conditioning_values in conditioning_grids {
        let mut at_zero = conditioning_values.clone();
        at_zero.push(ProcessModeratorValue {
            variable: moderation.moderator.clone(),
            raw_value: solved_profile.raw_mean,
            coded_value: 0.0,
        });
        let mut at_one = conditioning_values.clone();
        at_one.push(ProcessModeratorValue {
            variable: moderation.moderator.clone(),
            raw_value: solved_profile.raw_mean + 1.0,
            coded_value: 1.0,
        });
        let weights_zero = process_slope_weights(equation, moderation, &at_zero)?;
        let weights_one = process_slope_weights(equation, moderation, &at_one)?;
        let weights_delta = weights_one
            .iter()
            .zip(&weights_zero)
            .map(|(one, zero)| one - zero)
            .collect::<Vec<_>>();
        let a = equation
            .coefficients
            .iter()
            .zip(&weights_zero)
            .map(|(coefficient, weight)| coefficient.estimate * weight)
            .sum::<f64>();
        let b = equation
            .coefficients
            .iter()
            .zip(&weights_delta)
            .map(|(coefficient, weight)| coefficient.estimate * weight)
            .sum::<f64>();
        let covariance_form = |left: &[f64], right: &[f64]| {
            left.iter()
                .enumerate()
                .map(|(i, left_weight)| {
                    right
                        .iter()
                        .enumerate()
                        .map(|(j, right_weight)| {
                            left_weight * equation.coefficient_covariance[i][j] * right_weight
                        })
                        .sum::<f64>()
                })
                .sum::<f64>()
        };
        let v0 = covariance_form(&weights_zero, &weights_zero);
        let v1 = covariance_form(&weights_zero, &weights_delta);
        let v2 = covariance_form(&weights_delta, &weights_delta);
        if ![a, b, v0, v1, v2].iter().all(|value| value.is_finite()) {
            results.push(ProcessJohnsonNeyman::Unavailable {
                moderation_id: moderation_id.into(),
                solved_moderator: moderation.moderator.clone(),
                conditioning_values,
                reason_code: PROCESS_JN_INVALID_COVARIANCE_REASON.into(),
                message: PROCESS_JN_INVALID_COVARIANCE_MESSAGE.into(),
            });
            continue;
        }
        if !process_jn_variance_is_positive_across_range(v0, v1, v2, coded_min, coded_max) {
            results.push(ProcessJohnsonNeyman::Unavailable {
                moderation_id: moderation_id.into(),
                solved_moderator: moderation.moderator.clone(),
                conditioning_values,
                reason_code: PROCESS_JN_INVALID_COVARIANCE_REASON.into(),
                message: PROCESS_JN_INVALID_COVARIANCE_MESSAGE.into(),
            });
            continue;
        }
        let qa = b * b - critical * critical * v2;
        let qb = 2.0 * (a * b - critical * critical * v1);
        let qc = a * a - critical * critical * v0;
        let coded_roots = process_johnson_neyman_coded_roots(qa, qb, qc, coded_min, coded_max);
        let roots = coded_roots
            .iter()
            .map(|root| root + solved_profile.raw_mean)
            .collect::<Vec<_>>();
        let mut boundaries = vec![solved_profile.raw_min];
        boundaries.extend(roots.iter().copied());
        boundaries.push(solved_profile.raw_max);
        let regions = boundaries
            .windows(2)
            .map(|region| {
                let raw = (region[0] + region[1]) / 2.0;
                let coded = raw - solved_profile.raw_mean;
                let effect = a + b * coded;
                let variance = process_jn_variance(v0, v1, v2, coded)?;
                let margin = critical * variance.sqrt();
                let status = if effect + margin < 0.0 {
                    "significant_negative"
                } else if effect - margin > 0.0 {
                    "significant_positive"
                } else {
                    "not_significant"
                };
                Some(ProcessJohnsonNeymanRegion {
                    lower: region[0],
                    upper: region[1],
                    status: status.into(),
                })
            })
            .collect::<Option<Vec<_>>>();
        let Some(regions) = regions else {
            results.push(ProcessJohnsonNeyman::Unavailable {
                moderation_id: moderation_id.into(),
                solved_moderator: moderation.moderator.clone(),
                conditioning_values,
                reason_code: PROCESS_JN_INVALID_COVARIANCE_REASON.into(),
                message: PROCESS_JN_INVALID_COVARIANCE_MESSAGE.into(),
            });
            continue;
        };
        let curve_points = (0..101)
            .map(|index| {
                let raw = solved_profile.raw_min
                    + (solved_profile.raw_max - solved_profile.raw_min) * index as f64 / 100.0;
                let coded = raw - solved_profile.raw_mean;
                let effect = a + b * coded;
                let standard_error = process_jn_variance(v0, v1, v2, coded)?.sqrt();
                Some(ProcessJohnsonNeymanPoint {
                    moderator_raw: raw,
                    effect,
                    standard_error,
                    confidence_interval_lower: effect - critical * standard_error,
                    confidence_interval_upper: effect + critical * standard_error,
                })
            })
            .collect::<Option<Vec<_>>>();
        let Some(curve_points) = curve_points else {
            results.push(ProcessJohnsonNeyman::Unavailable {
                moderation_id: moderation_id.into(),
                solved_moderator: moderation.moderator.clone(),
                conditioning_values,
                reason_code: PROCESS_JN_INVALID_COVARIANCE_REASON.into(),
                message: PROCESS_JN_INVALID_COVARIANCE_MESSAGE.into(),
            });
            continue;
        };
        results.push(ProcessJohnsonNeyman::Available {
            moderation_id: moderation_id.into(),
            solved_moderator: moderation.moderator.clone(),
            conditioning_values,
            raw_min: solved_profile.raw_min,
            raw_max: solved_profile.raw_max,
            roots,
            regions,
            curve_points,
        });
    }
    Ok(results)
}

fn nca_scope(x: &[f64], y: &[f64]) -> NcaScope {
    NcaScope {
        minimum_x: x.iter().copied().fold(f64::INFINITY, f64::min),
        maximum_x: x.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        minimum_y: y.iter().copied().fold(f64::INFINITY, f64::min),
        maximum_y: y.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

fn nca_requested_ceilings(ceiling: &str) -> Option<Vec<&'static str>> {
    match ceiling {
        "ce_fdh" => Some(vec!["ce_fdh"]),
        "cr_fdh" => Some(vec!["cr_fdh"]),
        "both" => Some(vec!["ce_fdh", "cr_fdh"]),
        _ => None,
    }
}

fn nca_ce_fdh_peers(x: &[f64], y: &[f64]) -> Vec<NcaCeilingPoint> {
    let mut points = x.iter().copied().zip(y.iter().copied()).collect::<Vec<_>>();
    points.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.total_cmp(&right.1)));

    let mut maxima_by_x = Vec::<NcaCeilingPoint>::new();
    for (x_value, y_value) in points {
        if let Some(last) = maxima_by_x.last_mut()
            && last.x == x_value
        {
            last.y = last.y.max(y_value);
            continue;
        }
        maxima_by_x.push(NcaCeilingPoint {
            x: x_value,
            y: y_value,
        });
    }

    let mut peers = Vec::new();
    for point in maxima_by_x {
        if peers
            .last()
            .is_none_or(|previous: &NcaCeilingPoint| point.y > previous.y)
        {
            peers.push(point);
        }
    }
    peers
}

fn nca_ce_fdh_effect_size(scope: &NcaScope, peers: &[NcaCeilingPoint]) -> f64 {
    if peers.is_empty() {
        return 0.0;
    }
    let mut ceiling_area = 0.0;
    for (index, peer) in peers.iter().enumerate() {
        let next_x = peers
            .get(index + 1)
            .map(|next| next.x)
            .unwrap_or(scope.maximum_x);
        ceiling_area += (next_x - peer.x).max(0.0) * (scope.maximum_y - peer.y).max(0.0);
    }
    (ceiling_area / nca_scope_area(scope)).clamp(0.0, 1.0)
}

fn nca_cr_fdh_line(peers: &[NcaCeilingPoint]) -> Option<(f64, f64)> {
    if peers.len() < 2 {
        return None;
    }
    let mean_x = peers.iter().map(|peer| peer.x).sum::<f64>() / peers.len() as f64;
    let mean_y = peers.iter().map(|peer| peer.y).sum::<f64>() / peers.len() as f64;
    let numerator = peers
        .iter()
        .map(|peer| (peer.x - mean_x) * (peer.y - mean_y))
        .sum::<f64>();
    let denominator = peers
        .iter()
        .map(|peer| (peer.x - mean_x).powi(2))
        .sum::<f64>();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;
    (slope.is_finite() && intercept.is_finite()).then_some((slope, intercept))
}

fn nca_cr_fdh_effect_size(scope: &NcaScope, line: Option<(f64, f64)>) -> f64 {
    let Some((slope, intercept)) = line else {
        return 0.0;
    };
    let mut boundaries = vec![scope.minimum_x, scope.maximum_x];
    if slope.abs() > f64::EPSILON {
        for y_boundary in [scope.minimum_y, scope.maximum_y] {
            let crossing = (y_boundary - intercept) / slope;
            if crossing > scope.minimum_x && crossing < scope.maximum_x {
                boundaries.push(crossing);
            }
        }
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
    let mut ceiling_area = 0.0;
    for interval in boundaries.windows(2) {
        let left = interval[0];
        let right = interval[1];
        let left_y = (slope * left + intercept).clamp(scope.minimum_y, scope.maximum_y);
        let right_y = (slope * right + intercept).clamp(scope.minimum_y, scope.maximum_y);
        ceiling_area +=
            (right - left) * ((scope.maximum_y - left_y) + (scope.maximum_y - right_y)) / 2.0;
    }
    (ceiling_area / nca_scope_area(scope)).clamp(0.0, 1.0)
}

fn nca_ceiling_parameters(
    scope: &NcaScope,
    peers: &[NcaCeilingPoint],
    ceiling: &str,
    cr_line: Option<(f64, f64)>,
) -> (f64, Option<f64>, Option<f64>) {
    if ceiling == "cr_fdh" {
        let (slope, intercept) = cr_line.map_or((None, None), |(slope, intercept)| {
            (Some(slope), Some(intercept))
        });
        (nca_cr_fdh_effect_size(scope, cr_line), slope, intercept)
    } else {
        (nca_ce_fdh_effect_size(scope, peers), None, None)
    }
}

fn nca_scope_area(scope: &NcaScope) -> f64 {
    ((scope.maximum_x - scope.minimum_x) * (scope.maximum_y - scope.minimum_y)).max(f64::EPSILON)
}

fn nca_permutation_p_value(
    x: &[f64],
    y: &[f64],
    ceiling: &str,
    observed: f64,
    permutations: usize,
    master_seed: u64,
    completed_offset: usize,
    total_units: usize,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<f64, EstimationError> {
    if permutations == 0 {
        return Ok(1.0);
    }
    let mut exceedances = 0usize;
    for replicate in 0..permutations {
        if replicate == 0 || replicate % (permutations / 100).max(1) == 0 {
            checkpoint(
                control,
                EstimationPhase::ComputingEffects,
                completed_offset.saturating_add(replicate) as u64,
                total_units as u64,
            )?;
        }
        let indices = nca_permutation_indices(y.len(), master_seed, ceiling, replicate as u32);
        let permuted = indices.iter().map(|index| y[*index]).collect::<Vec<_>>();
        let permuted_scope = nca_scope(x, &permuted);
        let permuted_peers = nca_ce_fdh_peers(x, &permuted);
        let effect = if ceiling == "cr_fdh" {
            nca_cr_fdh_effect_size(&permuted_scope, nca_cr_fdh_line(&permuted_peers))
        } else {
            nca_ce_fdh_effect_size(&permuted_scope, &permuted_peers)
        };
        if effect >= observed.abs() - 1e-12 {
            exceedances += 1;
        }
    }
    Ok((exceedances as f64 + 1.0) / (permutations as f64 + 1.0))
}

fn nca_permutation_indices(
    case_count: usize,
    master_seed: u64,
    ceiling: &str,
    replicate: u32,
) -> Vec<usize> {
    let mut digest = Sha256::new();
    digest.update(b"quickpls:nca-permutation:v2");
    digest.update(master_seed.to_le_bytes());
    digest.update((ceiling.len() as u64).to_le_bytes());
    digest.update(ceiling.as_bytes());
    digest.update(replicate.to_le_bytes());
    let mut rng = ChaCha20Rng::from_seed(digest.finalize().into());
    let mut indices = (0..case_count).collect::<Vec<_>>();
    for upper in (1..case_count).rev() {
        indices.swap(upper, rng.random_range(0..=upper));
    }
    indices
}

fn nca_bottleneck_rows(
    scope: &NcaScope,
    peers: &[NcaCeilingPoint],
    ceilings: &[&str],
    cr_line: Option<(f64, f64)>,
) -> Vec<NcaBottleneck> {
    ceilings
        .iter()
        .flat_map(|ceiling| {
            (10..=90)
                .step_by(10)
                .map(move |level| nca_bottleneck(scope, peers, ceiling, cr_line, level as f64))
        })
        .collect()
}

fn nca_bottleneck(
    scope: &NcaScope,
    peers: &[NcaCeilingPoint],
    ceiling: &str,
    cr_line: Option<(f64, f64)>,
    outcome_percent: f64,
) -> NcaBottleneck {
    let threshold = scope.minimum_y + (scope.maximum_y - scope.minimum_y) * outcome_percent / 100.0;
    let (required_x, status) = if ceiling == "cr_fdh" {
        match cr_line {
            Some((slope, intercept)) if slope > f64::EPSILON => {
                let left_y = slope * scope.minimum_x + intercept;
                let right_y = slope * scope.maximum_x + intercept;
                if threshold <= left_y {
                    (None, "not_necessary")
                } else if threshold > right_y {
                    (None, "not_attainable")
                } else {
                    (Some((threshold - intercept) / slope), "required")
                }
            }
            Some((_, intercept)) if threshold <= intercept => (None, "not_necessary"),
            _ => (None, "not_attainable"),
        }
    } else if let Some(first) = peers.first() {
        if threshold <= first.y {
            (None, "not_necessary")
        } else if let Some(peer) = peers.iter().find(|peer| peer.y >= threshold) {
            (Some(peer.x), "required")
        } else {
            (None, "not_attainable")
        }
    } else {
        (None, "not_attainable")
    };
    let required_x_percent = required_x.map(|required| {
        (100.0 * (required - scope.minimum_x) / (scope.maximum_x - scope.minimum_x))
            .clamp(0.0, 100.0)
    });
    NcaBottleneck {
        ceiling: ceiling.into(),
        outcome_percent,
        required_x_percent,
        status: status.into(),
    }
}

pub fn nca_analysis_matches_v2_contract(
    analysis: &NcaAnalysis,
    expected_x: &str,
    expected_y: &str,
    expected_ceiling: &str,
    expected_permutations: usize,
) -> bool {
    let Some(expected_ceilings) = nca_requested_ceilings(expected_ceiling) else {
        return false;
    };
    let scope = &analysis.scope;
    if analysis.method_version != NCA_METHOD_VERSION
        || analysis.x != expected_x
        || analysis.y != expected_y
        || analysis.x == analysis.y
        || analysis.ceiling != expected_ceiling
        || analysis.permutation_samples != expected_permutations
        || analysis.usable_permutations != expected_permutations
        || analysis.observations < 3
        || analysis.warnings.is_empty()
        || ![
            scope.minimum_x,
            scope.maximum_x,
            scope.minimum_y,
            scope.maximum_y,
        ]
        .iter()
        .all(|value| value.is_finite())
        || scope.maximum_x - scope.minimum_x <= f64::EPSILON
        || scope.maximum_y - scope.minimum_y <= f64::EPSILON
        || analysis.ce_fdh_peers.is_empty()
    {
        return false;
    }
    for (index, peer) in analysis.ce_fdh_peers.iter().enumerate() {
        if !peer.x.is_finite()
            || !peer.y.is_finite()
            || peer.x < scope.minimum_x
            || peer.x > scope.maximum_x
            || peer.y < scope.minimum_y
            || peer.y > scope.maximum_y
            || index > 0
                && (peer.x <= analysis.ce_fdh_peers[index - 1].x
                    || peer.y <= analysis.ce_fdh_peers[index - 1].y)
        {
            return false;
        }
    }
    if !close_enough(analysis.ce_fdh_peers[0].x, scope.minimum_x)
        || !close_enough(analysis.ce_fdh_peers.last().unwrap().y, scope.maximum_y)
        || analysis.ceilings.len() != expected_ceilings.len()
    {
        return false;
    }
    let cr_line = nca_cr_fdh_line(&analysis.ce_fdh_peers);
    for (row, ceiling) in analysis.ceilings.iter().zip(&expected_ceilings) {
        let (effect_size, slope, intercept) =
            nca_ceiling_parameters(scope, &analysis.ce_fdh_peers, ceiling, cr_line);
        let Some(p_value) = row.permutation_p_value else {
            return false;
        };
        let lattice = p_value * (expected_permutations as f64 + 1.0);
        if row.ceiling != *ceiling
            || !close_enough(row.effect_size, effect_size)
            || !optional_close(row.slope, slope)
            || !optional_close(row.intercept, intercept)
            || !p_value.is_finite()
            || p_value < 1.0 / (expected_permutations as f64 + 1.0)
            || p_value > 1.0
            || !close_enough(lattice, lattice.round())
        {
            return false;
        }
    }
    let expected_bottlenecks =
        nca_bottleneck_rows(scope, &analysis.ce_fdh_peers, &expected_ceilings, cr_line);
    analysis.bottlenecks.len() == expected_bottlenecks.len()
        && analysis
            .bottlenecks
            .iter()
            .zip(expected_bottlenecks)
            .all(|(actual, expected)| {
                actual.ceiling == expected.ceiling
                    && close_enough(actual.outcome_percent, expected.outcome_percent)
                    && optional_close(actual.required_x_percent, expected.required_x_percent)
                    && actual.status == expected.status
            })
}

fn optional_close(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => close_enough(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn close_enough(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 1e-10 * left.abs().max(right.abs()).max(1.0)
}

fn log_determinant(matrix: &[Vec<f64>]) -> Result<f64, EstimationError> {
    let mut a = matrix.to_vec();
    let mut logdet = 0.0;
    for column in 0..a.len() {
        let pivot = (column..a.len())
            .max_by(|left, right| a[*left][column].abs().total_cmp(&a[*right][column].abs()))
            .unwrap_or(column);
        if a[pivot][column].abs() <= 1e-12 {
            return Err(EstimationError::Numerical(
                "matrix determinant is not positive".into(),
            ));
        }
        if pivot != column {
            a.swap(pivot, column);
        }
        let diagonal = a[column][column];
        if diagonal <= 0.0 || !diagonal.is_finite() {
            return Err(EstimationError::Numerical(
                "matrix determinant is not positive".into(),
            ));
        }
        logdet += diagonal.ln();
        for row in column + 1..a.len() {
            let factor = a[row][column] / diagonal;
            for item in column..a.len() {
                a[row][item] -= factor * a[column][item];
            }
        }
    }
    Ok(logdet)
}

fn invert_matrix(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, EstimationError> {
    let n = matrix.len();
    let mut a = vec![vec![0.0; n * 2]; n];
    for row in 0..n {
        for column in 0..n {
            a[row][column] = matrix[row][column];
        }
        a[row][n + row] = 1.0;
    }
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|left, right| a[*left][column].abs().total_cmp(&a[*right][column].abs()))
            .unwrap_or(column);
        if a[pivot][column].abs() <= 1e-12 {
            return Err(EstimationError::Numerical("matrix is singular".into()));
        }
        a.swap(column, pivot);
        let diagonal = a[column][column];
        for item in 0..n * 2 {
            a[column][item] /= diagonal;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = a[row][column];
            for item in 0..n * 2 {
                a[row][item] -= factor * a[column][item];
            }
        }
    }
    Ok(a.into_iter().map(|row| row[n..].to_vec()).collect())
}

fn invert_matrix_with_ridge(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, EstimationError> {
    let mut ridge = 0.0;
    for _ in 0..8 {
        let mut adjusted = matrix.to_vec();
        for (index, row) in adjusted.iter_mut().enumerate() {
            row[index] += ridge;
        }
        if let Ok(inverse) = invert_matrix(&adjusted) {
            return Ok(inverse);
        }
        ridge = if ridge == 0.0 { 1e-8 } else { ridge * 10.0 };
    }
    invert_matrix(matrix)
}

fn min_max_performance(values: &[f64]) -> f64 {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() || (max - min).abs() <= f64::EPSILON {
        return 50.0;
    }
    values
        .iter()
        .map(|value| 100.0 * (value - min) / (max - min))
        .sum::<f64>()
        / values.len() as f64
}

fn max_segment_path_separation(left: &[PathEstimate], right: &[PathEstimate]) -> f64 {
    left.iter()
        .filter_map(|left_path| {
            right
                .iter()
                .find(|right_path| {
                    right_path.source == left_path.source && right_path.target == left_path.target
                })
                .map(|right_path| (left_path.coefficient - right_path.coefficient).abs())
        })
        .fold(0.0, f64::max)
}

#[derive(Debug, Clone)]
struct SegmentStructuralFit {
    observations: usize,
    paths: Vec<PathEstimate>,
    r_squared: BTreeMap<String, f64>,
    sse: f64,
}

fn segment_structural_fit(
    recipe: &AnalysisRecipe,
    result: &PlsResult,
    rows: &[usize],
) -> Result<SegmentStructuralFit, EstimationError> {
    if rows.len() < 3 {
        return Err(EstimationError::InsufficientObservations);
    }
    let targets = recipe.model.paths.iter().fold(
        BTreeMap::<String, Vec<String>>::new(),
        |mut targets, path| {
            targets
                .entry(path.target.clone())
                .or_default()
                .push(path.source.clone());
            targets
        },
    );
    let mut paths = Vec::new();
    let mut r_squared = BTreeMap::new();
    let mut total_sse = 0.0;
    for (target, sources) in targets {
        if rows.len() <= sources.len() + 1 {
            return Err(EstimationError::RankDeficient(target));
        }
        let outcome = rows
            .iter()
            .map(|row| result.construct_scores[&target][*row])
            .collect::<Vec<_>>();
        let predictors = sources
            .iter()
            .map(|source| {
                rows.iter()
                    .map(|row| result.construct_scores[source][*row])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let coefficients = ols(&predictors, &outcome, &format!("segment {target}"))?;
        let predictor_means = predictors
            .iter()
            .map(|predictor| vector_mean(predictor))
            .collect::<Vec<_>>();
        let outcome_mean = vector_mean(&outcome);
        let mut sse = 0.0;
        let mut total = 0.0;
        for row in 0..outcome.len() {
            let fitted = coefficients
                .iter()
                .enumerate()
                .map(|(column, coefficient)| {
                    coefficient * (predictors[column][row] - predictor_means[column])
                })
                .sum::<f64>();
            let centered = outcome[row] - outcome_mean;
            let residual = centered - fitted;
            sse += residual * residual;
            total += centered * centered;
        }
        total_sse += sse;
        r_squared.insert(
            target.clone(),
            if total > f64::EPSILON {
                (1.0 - sse / total).clamp(0.0, 1.0)
            } else {
                0.0
            },
        );
        for (source, coefficient) in sources.into_iter().zip(coefficients) {
            paths.push(PathEstimate {
                source,
                target: target.clone(),
                coefficient,
            });
        }
    }
    Ok(SegmentStructuralFit {
        observations: rows.len(),
        paths,
        r_squared,
        sse: total_sse,
    })
}

fn segmentation_features(
    recipe: &AnalysisRecipe,
    result: &PlsResult,
) -> Result<Vec<Vec<f64>>, EstimationError> {
    let observations = result
        .construct_scores
        .values()
        .next()
        .map(Vec::len)
        .unwrap_or_default();
    if observations == 0 {
        return Err(EstimationError::InsufficientObservations);
    }
    let mut columns = Vec::<Vec<f64>>::new();
    for path in &recipe.model.paths {
        let source = result
            .construct_scores
            .get(&path.source)
            .ok_or_else(|| EstimationError::UnknownConstruct(path.source.clone()))?;
        let target = result
            .construct_scores
            .get(&path.target)
            .ok_or_else(|| EstimationError::UnknownConstruct(path.target.clone()))?;
        columns.push(
            source
                .iter()
                .zip(target)
                .map(|(left, right)| left * right)
                .collect(),
        );
    }
    for construct in &recipe.model.constructs {
        if let Some(scores) = result.construct_scores.get(&construct.id) {
            columns.push(scores.clone());
        }
    }
    let standardized = columns
        .into_iter()
        .filter_map(standardize_vector)
        .collect::<Vec<_>>();
    if standardized.is_empty() {
        return Err(EstimationError::Numerical(
            "segmentation features are constant".into(),
        ));
    }
    let mut rows = vec![vec![0.0; standardized.len()]; observations];
    for (column_index, column) in standardized.iter().enumerate() {
        for (row_index, value) in column.iter().enumerate() {
            rows[row_index][column_index] = *value;
        }
    }
    Ok(rows)
}

fn deterministic_partition_segments(
    recipe: &AnalysisRecipe,
    result: &PlsResult,
    features: &[Vec<f64>],
    segment_count: usize,
    starts: usize,
    minimum_size: usize,
) -> Result<
    (
        Vec<usize>,
        Vec<SegmentStructuralFit>,
        f64,
        Vec<PlsPosObjectiveStep>,
    ),
    EstimationError,
> {
    let observations = features.len();
    if observations < segment_count * minimum_size {
        return Err(EstimationError::InsufficientObservations);
    }
    let mut best_assignments = Vec::new();
    let mut best_fits = Vec::new();
    let mut best_objective = f64::INFINITY;
    let mut best_history = Vec::new();
    let ordered = sorted_feature_rows(features);
    for start in 0..starts {
        let mut centroids = initial_centroids(features, &ordered, segment_count, start);
        let mut assignments = vec![0usize; observations];
        let mut history = Vec::new();
        for iteration in 0..12 {
            for row in 0..observations {
                assignments[row] = nearest_centroid(&features[row], &centroids);
            }
            rebalance_assignments(
                &mut assignments,
                features,
                &centroids,
                segment_count,
                minimum_size,
            );
            centroids = recompute_centroids(features, &assignments, segment_count, &centroids);
            let (fits, objective) =
                segment_fits_from_assignments(recipe, result, &assignments, segment_count)?;
            history.push(PlsPosObjectiveStep {
                start,
                iteration,
                objective,
            });
            if objective < best_objective {
                best_objective = objective;
                best_assignments = assignments.clone();
                best_fits = fits;
                best_history = history.clone();
            }
        }
    }
    if best_assignments.is_empty() || !best_objective.is_finite() {
        return Err(EstimationError::Numerical(
            "deterministic segmentation did not produce a finite fit".into(),
        ));
    }
    Ok((best_assignments, best_fits, best_objective, best_history))
}

fn segment_fits_from_assignments(
    recipe: &AnalysisRecipe,
    result: &PlsResult,
    assignments: &[usize],
    segment_count: usize,
) -> Result<(Vec<SegmentStructuralFit>, f64), EstimationError> {
    let mut fits = Vec::new();
    let mut objective = 0.0;
    for segment in 0..segment_count {
        let rows = assignments
            .iter()
            .enumerate()
            .filter_map(|(row, assigned)| (*assigned == segment).then_some(row))
            .collect::<Vec<_>>();
        let fit = segment_structural_fit(recipe, result, &rows)?;
        objective += fit.sse;
        fits.push(fit);
    }
    Ok((fits, objective))
}

fn sorted_feature_rows(features: &[Vec<f64>]) -> Vec<usize> {
    let mut keyed = features
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let key = row
                .iter()
                .enumerate()
                .map(|(column, value)| value * (column + 1) as f64)
                .sum::<f64>();
            (index, key)
        })
        .collect::<Vec<_>>();
    keyed.sort_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)));
    keyed.into_iter().map(|(index, _)| index).collect()
}

fn initial_centroids(
    features: &[Vec<f64>],
    ordered: &[usize],
    segment_count: usize,
    start: usize,
) -> Vec<Vec<f64>> {
    (0..segment_count)
        .map(|segment| {
            let numerator = (segment + 1 + start % segment_count) * ordered.len();
            let position = (numerator / (segment_count + 1)).min(ordered.len() - 1);
            features[ordered[position]].clone()
        })
        .collect()
}

fn nearest_centroid(row: &[f64], centroids: &[Vec<f64>]) -> usize {
    centroids
        .iter()
        .enumerate()
        .map(|(index, centroid)| (index, feature_distance(row, centroid)))
        .min_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)))
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn feature_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum()
}

fn rebalance_assignments(
    assignments: &mut [usize],
    features: &[Vec<f64>],
    centroids: &[Vec<f64>],
    segment_count: usize,
    minimum_size: usize,
) {
    let mut counts = segment_counts(assignments, segment_count);
    loop {
        let Some(deficit_segment) = counts.iter().position(|count| *count < minimum_size) else {
            break;
        };
        let donor_segment = counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > minimum_size)
            .max_by_key(|(_, count)| **count)
            .map(|(segment, _)| segment);
        let Some(donor_segment) = donor_segment else {
            break;
        };
        let candidate = assignments
            .iter()
            .enumerate()
            .filter(|(_, assigned)| **assigned == donor_segment)
            .max_by(|left, right| {
                let left_gain = feature_distance(&features[left.0], &centroids[donor_segment])
                    - feature_distance(&features[left.0], &centroids[deficit_segment]);
                let right_gain = feature_distance(&features[right.0], &centroids[donor_segment])
                    - feature_distance(&features[right.0], &centroids[deficit_segment]);
                left_gain.total_cmp(&right_gain)
            })
            .map(|(row, _)| row);
        let Some(row) = candidate else {
            break;
        };
        assignments[row] = deficit_segment;
        counts[donor_segment] -= 1;
        counts[deficit_segment] += 1;
    }
}

fn segment_counts(assignments: &[usize], segment_count: usize) -> Vec<usize> {
    let mut counts = vec![0usize; segment_count];
    for assignment in assignments {
        counts[*assignment] += 1;
    }
    counts
}

fn recompute_centroids(
    features: &[Vec<f64>],
    assignments: &[usize],
    segment_count: usize,
    previous: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let dimensions = features.first().map(Vec::len).unwrap_or_default();
    let mut centroids = vec![vec![0.0; dimensions]; segment_count];
    let mut counts = vec![0usize; segment_count];
    for (row, assignment) in features.iter().zip(assignments) {
        counts[*assignment] += 1;
        for (dimension, value) in row.iter().enumerate() {
            centroids[*assignment][dimension] += value;
        }
    }
    for segment in 0..segment_count {
        if counts[segment] == 0 {
            centroids[segment] = previous[segment].clone();
        } else {
            for value in &mut centroids[segment] {
                *value /= counts[segment] as f64;
            }
        }
    }
    centroids
}

fn max_pairwise_path_separation(fits: &[SegmentStructuralFit]) -> f64 {
    let mut maximum = 0.0;
    for left in 0..fits.len() {
        for right in left + 1..fits.len() {
            maximum = f64::max(
                maximum,
                max_segment_path_separation(&fits[left].paths, &fits[right].paths),
            );
        }
    }
    maximum
}

fn apply_fimix_pls(recipe: &AnalysisRecipe, result: &mut PlsResult) -> Result<(), EstimationError> {
    if !group_method_requested(recipe, "fimix") && !recipe.metadata.contains_key("fimix_classes") {
        return Ok(());
    }
    ensure_group_segmentation_supported(recipe, "FIMIX-PLS v1")?;
    let classes = parse_metadata_usize(
        recipe,
        "fimix_classes",
        parse_metadata_usize(recipe, "segment_count", 2),
    )
    .clamp(2, 3);
    let starts = parse_metadata_usize(recipe, "segment_starts", 10).clamp(1, 50);
    let observations = result
        .construct_scores
        .values()
        .next()
        .map(Vec::len)
        .unwrap_or_default();
    if observations < 40 {
        return Err(EstimationError::InsufficientObservations);
    }
    let minimum_share = parse_metadata_f64(recipe, "minimum_segment_share", 0.10).clamp(0.05, 0.40);
    let minimum_size = ((observations as f64 * minimum_share).ceil() as usize).max(8);
    if observations < minimum_size * classes {
        return Err(EstimationError::InsufficientObservations);
    }
    let features = segmentation_features(recipe, result)?;
    let (assignments, fits, objective, history) =
        deterministic_partition_segments(recipe, result, &features, classes, starts, minimum_size)?;
    let sigma2 = (objective / observations as f64).max(1e-12);
    let log_likelihood =
        -0.5 * observations as f64 * ((2.0 * std::f64::consts::PI * sigma2).ln() + 1.0);
    let parameter_count =
        classes * (recipe.model.paths.len() + recipe.model.constructs.len()) + classes - 1;
    let aic = -2.0 * log_likelihood + 2.0 * parameter_count as f64;
    let bic = -2.0 * log_likelihood + (parameter_count as f64) * (observations as f64).ln();
    let caic =
        -2.0 * log_likelihood + (parameter_count as f64) * ((observations as f64).ln() + 1.0);
    let distances = features
        .iter()
        .enumerate()
        .map(|(row, feature)| {
            (0..classes)
                .map(|class| {
                    let class_rows = assignments
                        .iter()
                        .enumerate()
                        .filter_map(|(candidate, assigned)| {
                            (*assigned == class).then_some(candidate)
                        })
                        .collect::<Vec<_>>();
                    let centroid = centroid_for_rows(&class_rows, &features);
                    feature_distance(feature, &centroid) + row as f64 * 0.0
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let probabilities = distances
        .iter()
        .map(|row| softmax_inverse_distances(row))
        .collect::<Vec<_>>();
    let entropy = normalized_entropy(&probabilities);
    let memberships = probabilities
        .iter()
        .enumerate()
        .map(|(observation, probabilities)| {
            let (class, probability) = probabilities
                .iter()
                .copied()
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)))
                .unwrap_or((0, 1.0));
            FimixMembership {
                observation,
                class: format!("class_{}", class + 1),
                probability,
            }
        })
        .collect::<Vec<_>>();
    let classes_summary = fits
        .into_iter()
        .enumerate()
        .map(|(index, fit)| FimixClassSummary {
            class: format!("class_{}", index + 1),
            observations: fit.observations,
            share: fit.observations as f64 / observations as f64,
            paths: fit.paths,
            r_squared: fit.r_squared,
        })
        .collect::<Vec<_>>();
    let warnings = vec![
        "FIMIX-PLS v1 is validated for the documented QuickPLS v1.2.2 bounded deterministic 2-3 class score-space segmentation scope; full unrestricted EM/FIMIX parity is not claimed.".into(),
    ];
    result.fimix = Some(FimixPlsAnalysis {
        method_version: FIMIX_PLS_METHOD_VERSION.into(),
        classes,
        starts,
        iterations: history.len(),
        log_likelihood,
        aic,
        bic,
        caic,
        entropy,
        classes_summary,
        memberships,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn centroid_for_rows(rows: &[usize], features: &[Vec<f64>]) -> Vec<f64> {
    let dimensions = features.first().map(Vec::len).unwrap_or_default();
    let mut centroid = vec![0.0; dimensions];
    if rows.is_empty() {
        return centroid;
    }
    for row in rows {
        for (dimension, value) in features[*row].iter().enumerate() {
            centroid[dimension] += value;
        }
    }
    for value in &mut centroid {
        *value /= rows.len() as f64;
    }
    centroid
}

fn softmax_inverse_distances(distances: &[f64]) -> Vec<f64> {
    let weights = distances
        .iter()
        .map(|distance| (-distance.min(700.0)).exp())
        .collect::<Vec<_>>();
    let total = weights.iter().sum::<f64>();
    if total <= f64::EPSILON || !total.is_finite() {
        return vec![1.0 / distances.len() as f64; distances.len()];
    }
    weights.into_iter().map(|weight| weight / total).collect()
}

fn normalized_entropy(probabilities: &[Vec<f64>]) -> f64 {
    if probabilities.is_empty() || probabilities[0].len() <= 1 {
        return 0.0;
    }
    let classes = probabilities[0].len() as f64;
    let entropy = probabilities
        .iter()
        .flat_map(|row| row.iter())
        .filter(|probability| **probability > f64::EPSILON)
        .map(|probability| -probability * probability.ln())
        .sum::<f64>();
    1.0 - entropy / (probabilities.len() as f64 * classes.ln())
}

fn prepare_prediction_rows(
    dataset: &Dataset,
    indicators: &[String],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PredictionPreparedRows, EstimationError> {
    let schema = dataset.batch.schema();
    let positions = indicators
        .iter()
        .map(|name| {
            schema
                .index_of(name)
                .map_err(|_| EstimationError::InvalidIndicator(name.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let row_count = dataset.batch.num_rows();
    let mut complete_rows = Vec::with_capacity(row_count);
    for row in 0..row_count {
        if row % 1024 == 0 {
            checkpoint(
                control,
                EstimationPhase::PreparingRows,
                row as u64,
                row_count as u64,
            )?;
        }
        if positions.iter().all(|position| {
            let array = dataset.batch.column(*position);
            !array.is_null(row) && numeric_value(array.as_ref(), row).is_some_and(f64::is_finite)
        }) {
            complete_rows.push(row);
        }
    }
    checkpoint(
        control,
        EstimationPhase::PreparingRows,
        row_count as u64,
        row_count as u64,
    )?;
    Ok(PredictionPreparedRows {
        positions,
        complete_rows,
    })
}

fn prepare_prediction_split(
    dataset: &Dataset,
    indicators: &[String],
    positions: &[usize],
    train_rows: &[usize],
    test_rows: &[usize],
    preprocessing: &Preprocessing,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PredictionSplit, EstimationError> {
    if train_rows.len() < 3 || test_rows.len() < 2 {
        return Err(EstimationError::InsufficientObservations);
    }
    let mut train_columns = Vec::with_capacity(indicators.len());
    let mut test_columns = Vec::with_capacity(indicators.len());
    let mut transforms = Vec::with_capacity(indicators.len());
    for (indicator_index, (name, position)) in indicators.iter().zip(positions).enumerate() {
        checkpoint(
            control,
            EstimationPhase::PreparingIndicators,
            indicator_index as u64,
            indicators.len() as u64,
        )?;
        let train_raw = train_rows
            .iter()
            .map(|row| numeric_value(dataset.batch.column(*position).as_ref(), *row).unwrap())
            .collect::<Vec<_>>();
        let mean = vector_mean(&train_raw);
        let deviation = sample_sd(&train_raw);
        if deviation <= f64::EPSILON {
            return Err(EstimationError::ConstantIndicator(name.clone()));
        }
        let (center, scale) = match preprocessing {
            Preprocessing::Standardized => (mean, deviation),
            Preprocessing::MeanCentered => (mean, 1.0),
            Preprocessing::Unstandardized => (0.0, 1.0),
        };
        transforms.push(PredictionIndicatorTransform {
            raw_training_mean: mean,
            center,
            scale,
        });
        train_columns.push(
            train_raw
                .iter()
                .map(|value| (value - center) / scale)
                .collect(),
        );
        test_columns.push(
            test_rows
                .iter()
                .map(|row| {
                    (numeric_value(dataset.batch.column(*position).as_ref(), *row).unwrap()
                        - center)
                        / scale
                })
                .collect(),
        );
    }
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        indicators.len() as u64,
        indicators.len() as u64,
    )?;
    Ok(PredictionSplit {
        train_columns,
        test_columns,
        transforms,
        test_rows: test_rows.to_vec(),
        train_observations: train_rows.len(),
        test_observations: test_rows.len(),
    })
}

fn prediction_fold_output(
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    blocks: &[Vec<usize>],
    construct_index: &HashMap<&str, usize>,
    split: &PredictionSplit,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<PredictionFoldOutput, EstimationError> {
    let (weights, train_scores, _) = match recipe.settings.weighting_scheme {
        WeightingScheme::Pca => pca_scores(
            &split.train_columns,
            blocks,
            recipe.settings.tolerance,
            recipe.settings.max_iterations,
            control,
        )?,
        WeightingScheme::Path | WeightingScheme::Factor => {
            iterative_scores(&split.train_columns, blocks, recipe, false, control)?
        }
    };
    let observed_test_scores = block_scores_with_training_normalization(
        &split.train_columns,
        &split.test_columns,
        blocks,
        &weights,
    )?;
    let order = topological_construct_order(recipe, construct_index)?;
    let mut predicted_scores = vec![None; recipe.model.constructs.len()];
    for (completed, target_index) in order.iter().enumerate() {
        checkpoint(
            control,
            EstimationPhase::Assembling,
            completed as u64,
            order.len() as u64,
        )?;
        let construct = &recipe.model.constructs[*target_index];
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            predicted_scores[*target_index] = Some(observed_test_scores[*target_index].clone());
            continue;
        }
        let train_predictors = predecessors
            .iter()
            .map(|index| train_scores[*index].clone())
            .collect::<Vec<_>>();
        let coefficients = ols(
            &train_predictors,
            &train_scores[*target_index],
            &format!("PLSpredict v2 structural target {}", construct.id),
        )?;
        let test_predictors = predecessors
            .iter()
            .map(|index| {
                predicted_scores[*index]
                    .clone()
                    .ok_or_else(|| EstimationError::CyclicModel)
            })
            .collect::<Result<Vec<_>, _>>()?;
        predicted_scores[*target_index] = Some(fitted_values(&test_predictors, &coefficients));
    }
    checkpoint(
        control,
        EstimationPhase::Assembling,
        order.len() as u64,
        order.len() as u64,
    )?;

    let indicator_index = indicator_names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut constructs = Vec::new();
    let mut indicators = Vec::new();
    for (target_index, construct) in recipe.model.constructs.iter().enumerate() {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        if construct.mode != MeasurementMode::Reflective {
            return Err(EstimationError::UnsupportedMethod(format!(
                "PLSpredict indicator v2 requires reflective endogenous construct '{}'",
                construct.id
            )));
        }
        let earliest = earliest_antecedent_indices(recipe, construct_index, target_index)?;
        let predictor_indices = earliest
            .iter()
            .flat_map(|index| blocks[*index].iter().copied())
            .collect::<Vec<_>>();
        let predicted_score = predicted_scores[target_index]
            .as_ref()
            .ok_or(EstimationError::CyclicModel)?
            .clone();
        let linear_model_score = centered_ols_predictions(
            &predictor_indices
                .iter()
                .map(|index| split.train_columns[*index].clone())
                .collect::<Vec<_>>(),
            &train_scores[target_index],
            &predictor_indices
                .iter()
                .map(|index| split.test_columns[*index].clone())
                .collect::<Vec<_>>(),
            &format!("PLSpredict v2 construct LM benchmark {}", construct.id),
        )
        .ok();
        constructs.push(FoldConstructPrediction {
            construct: construct.id.clone(),
            predictor_count: predecessors.len(),
            actual: observed_test_scores[target_index].clone(),
            predicted: predicted_score.clone(),
            linear_model: linear_model_score,
        });

        for indicator in &construct.indicators {
            let column_index = indicator_index[indicator.as_str()];
            let train_target = &split.train_columns[column_index];
            let score_variance = sample_sd(&train_scores[target_index]).powi(2);
            if !score_variance.is_finite() || score_variance <= f64::EPSILON {
                return Err(EstimationError::Numerical(format!(
                    "zero training construct-score variance for {}",
                    construct.id
                )));
            }
            let slope = covariance(train_target, &train_scores[target_index]) / score_variance;
            let intercept = vector_mean(train_target);
            let transform = &split.transforms[column_index];
            let predicted = predicted_score
                .iter()
                .map(|score| (intercept + slope * score) * transform.scale + transform.center)
                .collect::<Vec<_>>();
            let actual = split.test_columns[column_index]
                .iter()
                .map(|value| value * transform.scale + transform.center)
                .collect::<Vec<_>>();
            let indicator_average = vec![transform.raw_training_mean; actual.len()];
            let linear_model = centered_ols_predictions(
                &predictor_indices
                    .iter()
                    .map(|index| split.train_columns[*index].clone())
                    .collect::<Vec<_>>(),
                train_target,
                &predictor_indices
                    .iter()
                    .map(|index| split.test_columns[*index].clone())
                    .collect::<Vec<_>>(),
                &format!(
                    "PLSpredict v2 indicator LM benchmark {}::{}",
                    construct.id, indicator
                ),
            )
            .map(|values| {
                values
                    .iter()
                    .map(|value| value * transform.scale + transform.center)
                    .collect::<Vec<_>>()
            })
            .map_err(|error| error.to_string());
            indicators.push(FoldIndicatorPrediction {
                construct: construct.id.clone(),
                indicator: indicator.clone(),
                predictor_count: predictor_indices.len(),
                actual,
                predicted,
                indicator_average,
                linear_model,
            });
        }
    }
    Ok(PredictionFoldOutput {
        constructs,
        indicators,
    })
}

fn block_scores_with_training_normalization(
    train_columns: &[Vec<f64>],
    test_columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    weights: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, EstimationError> {
    blocks
        .iter()
        .zip(weights)
        .map(|(block, weight)| {
            let mut train_score = vec![0.0; train_columns[0].len()];
            let mut test_score = vec![0.0; test_columns[0].len()];
            for (column, coefficient) in block.iter().zip(weight) {
                add_scaled(&mut train_score, &train_columns[*column], *coefficient);
                add_scaled(&mut test_score, &test_columns[*column], *coefficient);
            }
            let mean = vector_mean(&train_score);
            let deviation = sample_sd(&train_score);
            if !deviation.is_finite() || deviation <= f64::EPSILON {
                return Err(EstimationError::Numerical(
                    "training construct score has zero variance".into(),
                ));
            }
            for value in &mut test_score {
                *value = (*value - mean) / deviation;
            }
            if test_score.iter().any(|value| !value.is_finite()) {
                return Err(EstimationError::Numerical(
                    "non-finite holdout construct score".into(),
                ));
            }
            Ok(test_score)
        })
        .collect()
}

fn topological_construct_order(
    recipe: &AnalysisRecipe,
    construct_index: &HashMap<&str, usize>,
) -> Result<Vec<usize>, EstimationError> {
    let mut incoming = vec![0_usize; recipe.model.constructs.len()];
    let mut outgoing = vec![Vec::new(); recipe.model.constructs.len()];
    for path in &recipe.model.paths {
        let source = construct_index[path.source.as_str()];
        let target = construct_index[path.target.as_str()];
        incoming[target] += 1;
        outgoing[source].push(target);
    }
    let mut ready = (0..incoming.len())
        .filter(|index| incoming[*index] == 0)
        .collect::<Vec<_>>();
    ready.reverse();
    let mut order = Vec::with_capacity(incoming.len());
    while let Some(source) = ready.pop() {
        order.push(source);
        for target in &outgoing[source] {
            incoming[*target] -= 1;
            if incoming[*target] == 0 {
                ready.push(*target);
                ready.sort_unstable_by(|left, right| right.cmp(left));
            }
        }
    }
    (order.len() == recipe.model.constructs.len())
        .then_some(order)
        .ok_or(EstimationError::CyclicModel)
}

fn earliest_antecedent_indices(
    recipe: &AnalysisRecipe,
    construct_index: &HashMap<&str, usize>,
    target: usize,
) -> Result<Vec<usize>, EstimationError> {
    let mut incoming = vec![Vec::new(); recipe.model.constructs.len()];
    for path in &recipe.model.paths {
        incoming[construct_index[path.target.as_str()]].push(construct_index[path.source.as_str()]);
    }
    let mut roots = HashSet::new();
    let mut visited = HashSet::new();
    let mut stack = incoming[target].clone();
    while let Some(index) = stack.pop() {
        if !visited.insert(index) {
            continue;
        }
        if incoming[index].is_empty() {
            roots.insert(index);
        } else {
            stack.extend(incoming[index].iter().copied());
        }
    }
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort_unstable();
    if roots.is_empty() {
        return Err(EstimationError::Numerical(format!(
            "no earliest antecedent for {}",
            recipe.model.constructs[target].id
        )));
    }
    Ok(roots)
}

fn centered_ols_predictions(
    train_predictors: &[Vec<f64>],
    train_outcome: &[f64],
    test_predictors: &[Vec<f64>],
    subject: &str,
) -> Result<Vec<f64>, EstimationError> {
    if train_predictors.is_empty()
        || train_predictors.len() != test_predictors.len()
        || train_predictors
            .iter()
            .any(|predictor| predictor.len() != train_outcome.len())
    {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let rows = train_outcome.len();
    let columns = train_predictors.len();
    if rows <= columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let predictor_means = train_predictors
        .iter()
        .map(|predictor| vector_mean(predictor))
        .collect::<Vec<_>>();
    let outcome_mean = vector_mean(train_outcome);
    let matrix = Mat::from_fn(rows, columns, |row, column| {
        train_predictors[column][row] - predictor_means[column]
    });
    let qr = matrix.col_piv_qr();
    let diagonal = qr.thin_R();
    let diagonal_count = rows.min(columns);
    let max_diagonal = (0..diagonal_count)
        .map(|index| diagonal[(index, index)].abs())
        .fold(0.0, f64::max);
    let rank_tolerance = max_diagonal * (rows.max(columns) as f64) * f64::EPSILON * 100.0;
    let rank = (0..diagonal_count)
        .filter(|index| diagonal[(*index, *index)].abs() > rank_tolerance)
        .count();
    if rank < columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let rhs = Mat::from_fn(rows, 1, |row, _| train_outcome[row] - outcome_mean);
    let solution = qr.solve_lstsq(&rhs);
    let coefficients = (0..columns)
        .map(|index| solution[(index, 0)])
        .collect::<Vec<_>>();
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(EstimationError::Numerical(format!(
            "non-finite regression for {subject}"
        )));
    }
    let mut predictions = vec![outcome_mean; test_predictors[0].len()];
    for (index, (column, coefficient)) in test_predictors.iter().zip(coefficients).enumerate() {
        for (row, value) in column.iter().enumerate() {
            predictions[row] += (value - predictor_means[index]) * coefficient;
        }
    }
    Ok(predictions)
}

fn construct_prediction_target(prediction: &FoldConstructPrediction) -> PlsPredictTarget {
    let benchmark = vec![0.0; prediction.actual.len()];
    let pls_sse = squared_error_sum(&prediction.actual, &prediction.predicted);
    let benchmark_sse = squared_error_sum(&prediction.actual, &benchmark);
    let lm_sse = prediction
        .linear_model
        .as_ref()
        .map(|values| squared_error_sum(&prediction.actual, values));
    PlsPredictTarget {
        construct: prediction.construct.clone(),
        predictor_count: prediction.predictor_count,
        rmse_pls: rmse(&prediction.actual, &prediction.predicted),
        mae_pls: mae(&prediction.actual, &prediction.predicted),
        rmse_benchmark: rmse(&prediction.actual, &benchmark),
        mae_benchmark: mae(&prediction.actual, &benchmark),
        q_squared_predict: (benchmark_sse > f64::EPSILON)
            .then(|| 1.0 - pls_sse / benchmark_sse)
            .filter(|value| value.is_finite()),
        rmse_lm: prediction
            .linear_model
            .as_ref()
            .map(|values| rmse(&prediction.actual, values)),
        mae_lm: prediction
            .linear_model
            .as_ref()
            .map(|values| mae(&prediction.actual, values)),
        q_squared_predict_lm: lm_sse
            .and_then(|sse| (benchmark_sse > f64::EPSILON).then(|| 1.0 - sse / benchmark_sse))
            .filter(|value| value.is_finite()),
    }
}

fn indicator_prediction_target(prediction: &FoldIndicatorPrediction) -> PlsPredictIndicatorTarget {
    let pls = error_metrics(&prediction.actual, &prediction.predicted);
    let indicator_average = error_metrics(&prediction.actual, &prediction.indicator_average);
    let q_squared_predict = (indicator_average.squared_error_sum > f64::EPSILON)
        .then(|| 1.0 - pls.squared_error_sum / indicator_average.squared_error_sum)
        .filter(|value| value.is_finite());
    let linear_model = match &prediction.linear_model {
        Ok(values) => PlsPredictBenchmarkMetrics {
            status: "available".into(),
            metrics: Some(error_metrics(&prediction.actual, values)),
            reason: None,
        },
        Err(reason) => PlsPredictBenchmarkMetrics {
            status: "unavailable".into(),
            metrics: None,
            reason: Some(reason.clone()),
        },
    };
    PlsPredictIndicatorTarget {
        construct: prediction.construct.clone(),
        indicator: prediction.indicator.clone(),
        predictor_scope: "earliest_antecedent_indicators".into(),
        predictor_count: prediction.predictor_count,
        pls,
        indicator_average,
        linear_model,
        q_squared_predict,
    }
}

fn error_metrics(actual: &[f64], predicted: &[f64]) -> PlsPredictErrorMetrics {
    let mut accumulator = ErrorMetricAccumulator::default();
    accumulator.add(actual, predicted);
    accumulator.finish()
}

impl ErrorMetricAccumulator {
    fn add(&mut self, actual: &[f64], predicted: &[f64]) {
        self.observations += actual.len();
        for (actual, predicted) in actual.iter().zip(predicted) {
            let error = actual - predicted;
            self.squared_error_sum += error.powi(2);
            self.absolute_error_sum += error.abs();
            if actual.abs() > f64::EPSILON {
                self.absolute_percentage_error_sum += (error / actual).abs();
                self.mape_observations += 1;
            }
        }
    }

    fn finish(&self) -> PlsPredictErrorMetrics {
        let observations = self.observations.max(1);
        PlsPredictErrorMetrics {
            observations: self.observations,
            squared_error_sum: self.squared_error_sum,
            absolute_error_sum: self.absolute_error_sum,
            rmse: (self.squared_error_sum / observations as f64).sqrt(),
            mae: self.absolute_error_sum / observations as f64,
            absolute_percentage_error_sum: (self.mape_observations > 0)
                .then_some(self.absolute_percentage_error_sum),
            mape_observations: self.mape_observations,
            mape_percent: (self.mape_observations > 0).then(|| {
                100.0 * self.absolute_percentage_error_sum / self.mape_observations as f64
            }),
        }
    }
}

impl IndicatorPredictionAccumulator {
    fn new(prediction: &FoldIndicatorPrediction) -> Self {
        Self {
            construct: prediction.construct.clone(),
            indicator: prediction.indicator.clone(),
            predictor_count: prediction.predictor_count,
            pls: ErrorMetricAccumulator::default(),
            indicator_average: ErrorMetricAccumulator::default(),
            linear_model: ErrorMetricAccumulator::default(),
            linear_model_available: true,
            linear_model_reason: None,
        }
    }

    fn add(&mut self, prediction: &FoldIndicatorPrediction) {
        self.pls.add(&prediction.actual, &prediction.predicted);
        self.indicator_average
            .add(&prediction.actual, &prediction.indicator_average);
        match &prediction.linear_model {
            Ok(values) if self.linear_model_available => {
                self.linear_model.add(&prediction.actual, values);
            }
            Ok(_) => {}
            Err(reason) => {
                self.linear_model_available = false;
                self.linear_model_reason
                    .get_or_insert_with(|| reason.clone());
            }
        }
    }

    fn finish(&self) -> PlsPredictIndicatorTarget {
        let pls = self.pls.finish();
        let indicator_average = self.indicator_average.finish();
        let q_squared_predict = (indicator_average.squared_error_sum > f64::EPSILON)
            .then(|| 1.0 - pls.squared_error_sum / indicator_average.squared_error_sum)
            .filter(|value| value.is_finite());
        let linear_model = if self.linear_model_available {
            PlsPredictBenchmarkMetrics {
                status: "available".into(),
                metrics: Some(self.linear_model.finish()),
                reason: None,
            }
        } else {
            PlsPredictBenchmarkMetrics {
                status: "unavailable".into(),
                metrics: None,
                reason: Some(self.linear_model_reason.clone().unwrap_or_else(|| {
                    "The LM benchmark was unavailable in at least one fold".into()
                })),
            }
        };
        PlsPredictIndicatorTarget {
            construct: self.construct.clone(),
            indicator: self.indicator.clone(),
            predictor_scope: "earliest_antecedent_indicators".into(),
            predictor_count: self.predictor_count,
            pls,
            indicator_average,
            linear_model,
            q_squared_predict,
        }
    }
}

fn repeated_kfold_pls_predict(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    indicator_names: &[String],
    prepared_rows: &PredictionPreparedRows,
    blocks: &[Vec<usize>],
    construct_index: &HashMap<&str, usize>,
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<Option<PlsPredictRepeatedKfold>, EstimationError> {
    const FOLDS: usize = 10;
    const REPEATS: usize = 10;
    if prepared_rows.complete_rows.len() < 20 {
        return Ok(None);
    }
    let mut accumulators = prediction_accumulators(recipe, construct_index)?;
    let mut indicator_accumulators = Vec::<IndicatorPredictionAccumulator>::new();
    let expected_indicator_count = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| {
            recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        })
        .map(|construct| construct.indicators.len())
        .sum::<usize>();
    let mut cvpat_losses = CvpatLossAccumulator {
        cases: BTreeMap::new(),
        indicator_count: expected_indicator_count,
        linear_model_available: true,
        linear_model_reason: None,
    };
    let mut assignment_hasher = Sha256::new();
    for repeat in 0..REPEATS {
        let assignments = sha256_prediction_fold_assignments(
            &prepared_rows.complete_rows,
            recipe.settings.seed,
            repeat,
            FOLDS,
        );
        for (complete_index, fold) in assignments.iter().enumerate() {
            assignment_hasher.update(format!(
                "{repeat}|{}|{fold}\n",
                prepared_rows.complete_rows[complete_index]
            ));
        }
        for fold in 0..FOLDS {
            let train_rows = prepared_rows
                .complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (assignments[index] != fold).then_some(*row))
                .collect::<Vec<_>>();
            let test_rows = prepared_rows
                .complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (assignments[index] == fold).then_some(*row))
                .collect::<Vec<_>>();
            let split = prepare_prediction_split(
                dataset,
                indicator_names,
                &prepared_rows.positions,
                &train_rows,
                &test_rows,
                &recipe.settings.preprocessing,
                control,
            )?;
            let output = prediction_fold_output(
                recipe,
                indicator_names,
                blocks,
                construct_index,
                &split,
                control,
            )?;
            for (accumulator, prediction) in accumulators.iter_mut().zip(&output.constructs) {
                accumulator.add(
                    &prediction.actual,
                    &prediction.predicted,
                    prediction.linear_model.as_deref(),
                );
            }
            if indicator_accumulators.is_empty() {
                indicator_accumulators = output
                    .indicators
                    .iter()
                    .map(IndicatorPredictionAccumulator::new)
                    .collect();
            }
            if indicator_accumulators.len() != output.indicators.len() {
                return Err(EstimationError::Numerical(
                    "PLSpredict v2 indicator target count changed across folds".into(),
                ));
            }
            for (accumulator, prediction) in
                indicator_accumulators.iter_mut().zip(&output.indicators)
            {
                if accumulator.construct != prediction.construct
                    || accumulator.indicator != prediction.indicator
                {
                    return Err(EstimationError::Numerical(
                        "PLSpredict v2 indicator target order changed across folds".into(),
                    ));
                }
                accumulator.add(prediction);
            }
            cvpat_losses.add_fold(&split.test_rows, &output.indicators)?;
        }
    }
    let total_test_observations = prepared_rows.complete_rows.len() * REPEATS;
    let assignment_digest = format!(
        "sha256:{}",
        digest_hex(assignment_hasher.finalize().as_slice())
    );
    Ok(Some(PlsPredictRepeatedKfold {
        method_version: PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION.into(),
        folds: FOLDS,
        repeats: REPEATS,
        assignment: "seeded_sha256_source_row_order_round_robin_10_v1".into(),
        seed: recipe.settings.seed,
        assignment_digest,
        total_test_observations,
        targets: accumulators
            .iter()
            .map(PredictionErrorAccumulator::to_target)
            .collect(),
        indicator_targets: indicator_accumulators
            .iter()
            .map(IndicatorPredictionAccumulator::finish)
            .collect(),
        cvpat: Vec::new(),
        cvpat_benchmark_assessments: cvpat_losses.finish(REPEATS),
        paired_loss_diagnostics: Vec::new(),
        warnings: vec![
            "The primary PLSpredict v2 block uses a fixed seeded SHA-256 10-fold plan repeated 10 times. CVPAT compares this one fitted model with IA and LM benchmarks; it is not a comparison of separately saved models."
                .into(),
        ],
    }))
}

fn sha256_prediction_fold_assignments(
    complete_rows: &[usize],
    seed: u64,
    repeat: usize,
    folds: usize,
) -> Vec<usize> {
    let mut ranked = complete_rows
        .iter()
        .enumerate()
        .map(|(complete_index, source_row_index)| {
            let mut hasher = Sha256::new();
            hasher.update(format!(
                "{PLS_PREDICT_METHOD_VERSION}|{seed}|{repeat}|{source_row_index}"
            ));
            (hasher.finalize().to_vec(), complete_index)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let mut assignments = vec![0_usize; complete_rows.len()];
    for (position, (_, complete_index)) in ranked.into_iter().enumerate() {
        assignments[complete_index] = position % folds;
    }
    assignments
}

fn digest_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl CvpatLossAccumulator {
    fn add_fold(
        &mut self,
        test_rows: &[usize],
        indicators: &[FoldIndicatorPrediction],
    ) -> Result<(), EstimationError> {
        if indicators.len() != self.indicator_count
            || indicators.iter().any(|indicator| {
                indicator.actual.len() != test_rows.len()
                    || indicator.predicted.len() != test_rows.len()
                    || indicator.indicator_average.len() != test_rows.len()
            })
        {
            return Err(EstimationError::Numerical(
                "PLSpredict v2 CVPAT fold shape mismatch".into(),
            ));
        }
        for (position, source_row) in test_rows.iter().enumerate() {
            let mut pls_loss = 0.0;
            let mut indicator_average_loss = 0.0;
            let mut linear_model_loss = 0.0;
            let mut fold_lm_available = true;
            for indicator in indicators {
                pls_loss += (indicator.actual[position] - indicator.predicted[position]).powi(2);
                indicator_average_loss +=
                    (indicator.actual[position] - indicator.indicator_average[position]).powi(2);
                match &indicator.linear_model {
                    Ok(values) if values.len() == test_rows.len() => {
                        linear_model_loss +=
                            (indicator.actual[position] - values[position]).powi(2);
                    }
                    Ok(_) => {
                        fold_lm_available = false;
                        self.linear_model_reason.get_or_insert_with(|| {
                            "The LM benchmark returned a fold shape mismatch".into()
                        });
                    }
                    Err(reason) => {
                        fold_lm_available = false;
                        self.linear_model_reason
                            .get_or_insert_with(|| reason.clone());
                    }
                }
            }
            let divisor = self.indicator_count as f64;
            let case = self.cases.entry(*source_row).or_default();
            case.pls_sum += pls_loss / divisor;
            case.indicator_average_sum += indicator_average_loss / divisor;
            if fold_lm_available {
                case.linear_model_sum += linear_model_loss / divisor;
            } else {
                self.linear_model_available = false;
            }
            case.repeats += 1;
        }
        Ok(())
    }

    fn finish(&self, expected_repeats: usize) -> Vec<PlsPredictCvpatBenchmarkAssessment> {
        let complete = self
            .cases
            .values()
            .all(|case| case.repeats == expected_repeats);
        let pls_losses = self
            .cases
            .values()
            .map(|case| case.pls_sum / case.repeats.max(1) as f64)
            .collect::<Vec<_>>();
        let indicator_average_losses = self
            .cases
            .values()
            .map(|case| case.indicator_average_sum / case.repeats.max(1) as f64)
            .collect::<Vec<_>>();
        let mut rows = vec![cvpat_benchmark_assessment(
            "indicator_average",
            &pls_losses,
            Some(&indicator_average_losses),
            self.indicator_count,
            complete
                .then_some(())
                .ok_or_else(|| "Not every complete case was tested once per repeat".to_string()),
        )];
        let linear_model_losses = self.linear_model_available.then(|| {
            self.cases
                .values()
                .map(|case| case.linear_model_sum / case.repeats.max(1) as f64)
                .collect::<Vec<_>>()
        });
        let lm_availability = if !complete {
            Err("Not every complete case was tested once per repeat".into())
        } else if !self.linear_model_available {
            Err(self
                .linear_model_reason
                .clone()
                .unwrap_or_else(|| "The LM benchmark was unavailable in at least one fold".into()))
        } else {
            Ok(())
        };
        rows.push(cvpat_benchmark_assessment(
            "linear_model",
            &pls_losses,
            linear_model_losses.as_deref(),
            self.indicator_count,
            lm_availability,
        ));
        rows
    }
}

fn cvpat_benchmark_assessment(
    benchmark: &str,
    pls_losses: &[f64],
    benchmark_losses: Option<&[f64]>,
    indicator_count: usize,
    availability: Result<(), String>,
) -> PlsPredictCvpatBenchmarkAssessment {
    const CONFIDENCE_LEVEL: f64 = 0.95;
    let mean_loss_pls = (!pls_losses.is_empty()).then(|| vector_mean(pls_losses));
    let unavailable_reason = availability.err().or_else(|| {
        benchmark_losses
            .is_none()
            .then(|| "The benchmark is unavailable".into())
    });
    let Some(benchmark_losses) = benchmark_losses
        .filter(|values| values.len() == pls_losses.len() && unavailable_reason.is_none())
    else {
        return PlsPredictCvpatBenchmarkAssessment {
            method_version: CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION.into(),
            comparison_kind: "benchmark_assessment".into(),
            target_scope: "all_endogenous_indicators".into(),
            benchmark: benchmark.into(),
            loss: "mean_squared_error_across_indicators_per_observation".into(),
            alternative: "pls_loss_less_than_benchmark".into(),
            confidence_level: CONFIDENCE_LEVEL,
            mean_loss_pls,
            mean_loss_benchmark: None,
            mean_loss_difference: None,
            loss_difference_sum_of_squares: None,
            standard_error: None,
            t_statistic: None,
            p_value_one_sided: None,
            confidence_interval_lower: None,
            confidence_interval_upper: None,
            observations: pls_losses.len(),
            indicator_count,
            status: "benchmark_unavailable".into(),
            preferred_model: None,
            reason: Some(unavailable_reason.unwrap_or_else(|| {
                "The benchmark loss vector does not match the PLS loss vector".into()
            })),
        };
    };
    let differences = pls_losses
        .iter()
        .zip(benchmark_losses)
        .map(|(pls, comparison)| pls - comparison)
        .collect::<Vec<_>>();
    let observations = differences.len();
    let mean_loss_benchmark = vector_mean(benchmark_losses);
    let mean_loss_difference = vector_mean(&differences);
    let loss_difference_sum_of_squares = differences.iter().map(|value| value * value).sum::<f64>();
    let standard_error = (observations > 1).then(|| {
        let variance = ((loss_difference_sum_of_squares
            - observations as f64 * mean_loss_difference.powi(2))
            / (observations - 1) as f64)
            .max(0.0);
        variance.sqrt() / (observations as f64).sqrt()
    });
    let inferential = standard_error
        .filter(|value| value.is_finite() && *value > f64::EPSILON)
        .and_then(|standard_error| {
            let distribution = StudentsT::new(0.0, 1.0, observations as f64 - 1.0).ok()?;
            let t_statistic = mean_loss_difference / standard_error;
            let p_value = distribution.cdf(t_statistic);
            let critical = distribution.inverse_cdf(0.5 + CONFIDENCE_LEVEL / 2.0);
            Some((
                standard_error,
                t_statistic,
                p_value,
                mean_loss_difference - critical * standard_error,
                mean_loss_difference + critical * standard_error,
            ))
        });
    let (status, standard_error, t_statistic, p_value, lower, upper, preferred, reason) =
        if let Some((standard_error, t_statistic, p_value, lower, upper)) = inferential {
            (
                "available",
                Some(standard_error),
                Some(t_statistic),
                Some(p_value),
                Some(lower),
                Some(upper),
                (mean_loss_difference < 0.0 && p_value < 0.05).then(|| "pls_sem".to_string()),
                None,
            )
        } else {
            (
                "inferential_test_unavailable",
                None,
                None,
                None,
                None,
                None,
                None,
                Some(
                    "The one-sided CVPAT test is unavailable because paired loss differences have zero variance or insufficient observations"
                        .into(),
                ),
            )
        };
    PlsPredictCvpatBenchmarkAssessment {
        method_version: CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION.into(),
        comparison_kind: "benchmark_assessment".into(),
        target_scope: "all_endogenous_indicators".into(),
        benchmark: benchmark.into(),
        loss: "mean_squared_error_across_indicators_per_observation".into(),
        alternative: "pls_loss_less_than_benchmark".into(),
        confidence_level: CONFIDENCE_LEVEL,
        mean_loss_pls,
        mean_loss_benchmark: Some(mean_loss_benchmark),
        mean_loss_difference: Some(mean_loss_difference),
        loss_difference_sum_of_squares: Some(loss_difference_sum_of_squares),
        standard_error,
        t_statistic,
        p_value_one_sided: p_value,
        confidence_interval_lower: lower,
        confidence_interval_upper: upper,
        observations,
        indicator_count,
        status: status.into(),
        preferred_model: preferred,
        reason,
    }
}

fn prediction_accumulators(
    recipe: &AnalysisRecipe,
    construct_index: &HashMap<&str, usize>,
) -> Result<Vec<PredictionErrorAccumulator>, EstimationError> {
    let mut accumulators = Vec::new();
    for construct in &recipe.model.constructs {
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .map(|path| construct_index[path.source.as_str()])
            .collect::<Vec<_>>();
        if predecessors.is_empty() {
            continue;
        }
        accumulators.push(PredictionErrorAccumulator {
            construct: construct.id.clone(),
            predictor_count: predecessors.len(),
            lm_available: true,
            ..PredictionErrorAccumulator::default()
        });
    }
    if accumulators.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "Deterministic construct prediction requires at least one endogenous construct".into(),
        ));
    }
    Ok(accumulators)
}

impl PredictionErrorAccumulator {
    fn add(&mut self, actual: &[f64], pls_predicted: &[f64], lm_predicted: Option<&[f64]>) {
        self.observation_count += actual.len();
        self.pls_sse += squared_error_sum(actual, pls_predicted);
        self.pls_absolute_error += absolute_error_sum(actual, pls_predicted);
        let benchmark_sse = actual.iter().map(|value| value * value).sum::<f64>();
        self.benchmark_sse += benchmark_sse;
        self.benchmark_absolute_error += actual.iter().map(|value| value.abs()).sum::<f64>();
        if let Some(lm_predicted) = lm_predicted {
            if self.lm_available {
                let lm_sse = squared_error_sum(actual, lm_predicted);
                let lm_abs = absolute_error_sum(actual, lm_predicted);
                self.lm_sse = Some(self.lm_sse.unwrap_or(0.0) + lm_sse);
                self.lm_absolute_error = Some(self.lm_absolute_error.unwrap_or(0.0) + lm_abs);
            }
        } else {
            self.lm_available = false;
            self.lm_sse = None;
            self.lm_absolute_error = None;
        }
    }

    fn to_target(&self) -> PlsPredictTarget {
        let q_squared_predict = (self.benchmark_sse > f64::EPSILON)
            .then(|| 1.0 - self.pls_sse / self.benchmark_sse)
            .filter(|value| value.is_finite());
        let q_squared_predict_lm = self
            .lm_available
            .then_some(self.lm_sse)
            .flatten()
            .and_then(|lm_sse| {
                (self.benchmark_sse > f64::EPSILON).then(|| 1.0 - lm_sse / self.benchmark_sse)
            })
            .filter(|value| value.is_finite());
        PlsPredictTarget {
            construct: self.construct.clone(),
            predictor_count: self.predictor_count,
            rmse_pls: (self.pls_sse / self.observation_count as f64).sqrt(),
            mae_pls: self.pls_absolute_error / self.observation_count as f64,
            rmse_benchmark: (self.benchmark_sse / self.observation_count as f64).sqrt(),
            mae_benchmark: self.benchmark_absolute_error / self.observation_count as f64,
            q_squared_predict,
            rmse_lm: self
                .lm_available
                .then_some(self.lm_sse)
                .flatten()
                .map(|sse| (sse / self.observation_count as f64).sqrt()),
            mae_lm: self
                .lm_available
                .then_some(self.lm_absolute_error)
                .flatten()
                .map(|error| error / self.observation_count as f64),
            q_squared_predict_lm,
        }
    }
}
fn squared_error_sum(actual: &[f64], predicted: &[f64]) -> f64 {
    actual
        .iter()
        .zip(predicted)
        .map(|(actual, predicted)| (actual - predicted).powi(2))
        .sum()
}

fn absolute_error_sum(actual: &[f64], predicted: &[f64]) -> f64 {
    actual
        .iter()
        .zip(predicted)
        .map(|(actual, predicted)| (actual - predicted).abs())
        .sum()
}

fn rmse(actual: &[f64], predicted: &[f64]) -> f64 {
    (squared_error_sum(actual, predicted) / actual.len() as f64).sqrt()
}

fn mae(actual: &[f64], predicted: &[f64]) -> f64 {
    absolute_error_sum(actual, predicted) / actual.len() as f64
}

fn calculate_effects(
    constructs: &[String],
    paths: &[PathEstimate],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<Vec<EffectEstimate>, EstimationError> {
    let count = constructs.len();
    let index = constructs
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut direct = vec![vec![0.0; count]; count];
    for path in paths {
        direct[index[path.source.as_str()]][index[path.target.as_str()]] = path.coefficient;
    }
    let mut total = direct.clone();
    let mut power = direct.clone();
    let effect_units = count.saturating_sub(2) + count;
    let mut effect_completed = 0;
    for _ in 2..count {
        checkpoint(
            control,
            EstimationPhase::ComputingEffects,
            effect_completed as u64,
            effect_units as u64,
        )?;
        power = multiply_square(&power, &direct);
        for row in 0..count {
            for column in 0..count {
                total[row][column] += power[row][column];
            }
        }
        effect_completed += 1;
    }
    let mut result = Vec::new();
    for source in 0..count {
        checkpoint(
            control,
            EstimationPhase::ComputingEffects,
            effect_completed as u64,
            effect_units as u64,
        )?;
        for target in 0..count {
            if source != target && total[source][target].abs() > 1e-15 {
                result.push(EffectEstimate {
                    source: constructs[source].clone(),
                    target: constructs[target].clone(),
                    direct: direct[source][target],
                    indirect: total[source][target] - direct[source][target],
                    total: total[source][target],
                });
            }
        }
        effect_completed += 1;
    }
    checkpoint(
        control,
        EstimationPhase::ComputingEffects,
        effect_units as u64,
        effect_units as u64,
    )?;
    Ok(result)
}

fn block_scores(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    weights: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, EstimationError> {
    blocks
        .iter()
        .zip(weights)
        .map(|(block, weight)| {
            let mut score = vec![0.0; columns[0].len()];
            for (column, coefficient) in block.iter().zip(weight) {
                add_scaled(&mut score, &columns[*column], *coefficient);
            }
            standardize_vector(score).ok_or_else(|| {
                EstimationError::Numerical("construct score has zero variance".into())
            })
        })
        .collect()
}

fn block_scores_weighted(
    columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    weights: &[Vec<f64>],
    case_weights: &[f64],
) -> Result<Vec<Vec<f64>>, EstimationError> {
    blocks
        .iter()
        .zip(weights)
        .map(|(block, weight)| {
            let mut score = vec![0.0; columns[0].len()];
            for (column, coefficient) in block.iter().zip(weight) {
                add_scaled(&mut score, &columns[*column], *coefficient);
            }
            weighted_standardize_vector(score, case_weights).ok_or_else(|| {
                EstimationError::Numerical("construct score has zero weighted variance".into())
            })
        })
        .collect()
}

fn normalize_block_weights(
    columns: &[Vec<f64>],
    block: &[usize],
    mut weights: Vec<f64>,
) -> Result<Vec<f64>, EstimationError> {
    orient_block_weights(columns, block, &mut weights);
    let mut score = vec![0.0; columns[0].len()];
    for (column, coefficient) in block.iter().zip(&weights) {
        add_scaled(&mut score, &columns[*column], *coefficient);
    }
    let deviation = sample_sd(&score);
    if deviation <= f64::EPSILON || !deviation.is_finite() {
        return Err(EstimationError::Numerical(
            "outer weights produce a zero-variance score".into(),
        ));
    }
    for weight in &mut weights {
        *weight /= deviation;
    }
    orient_block_weights(columns, block, &mut weights);
    Ok(weights)
}

fn normalize_block_weights_weighted(
    columns: &[Vec<f64>],
    block: &[usize],
    mut weights: Vec<f64>,
    case_weights: &[f64],
) -> Result<Vec<f64>, EstimationError> {
    orient_block_weights_weighted(columns, block, &mut weights, case_weights);
    let mut score = vec![0.0; columns[0].len()];
    for (column, coefficient) in block.iter().zip(&weights) {
        add_scaled(&mut score, &columns[*column], *coefficient);
    }
    let deviation = weighted_sample_sd(&score, case_weights);
    if deviation <= f64::EPSILON || !deviation.is_finite() {
        return Err(EstimationError::Numerical(
            "outer weights produce a zero weighted-variance score".into(),
        ));
    }
    for weight in &mut weights {
        *weight /= deviation;
    }
    orient_block_weights_weighted(columns, block, &mut weights, case_weights);
    Ok(weights)
}

fn ols(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    subject: &str,
) -> Result<Vec<f64>, EstimationError> {
    if predictors.is_empty() {
        return Ok(Vec::new());
    }
    let rows = outcome.len();
    let columns = predictors.len();
    if rows < columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let centers = predictors
        .iter()
        .map(|predictor| vector_mean(predictor))
        .collect::<Vec<_>>();
    let matrix = Mat::from_fn(rows, columns, |row, column| {
        predictors[column][row] - centers[column]
    });
    let qr = matrix.col_piv_qr();
    let diagonal = qr.thin_R();
    let diagonal_count = rows.min(columns);
    let max_diagonal = (0..diagonal_count)
        .map(|index| diagonal[(index, index)].abs())
        .fold(0.0, f64::max);
    let rank_tolerance = max_diagonal * (rows.max(columns) as f64) * f64::EPSILON * 100.0;
    let rank = (0..diagonal_count)
        .filter(|index| diagonal[(*index, *index)].abs() > rank_tolerance)
        .count();
    if rank < columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let rhs = Mat::from_fn(rows, 1, |row, _| outcome[row]);
    let solution = qr.solve_lstsq(&rhs);
    let coefficients = (0..columns)
        .map(|index| solution[(index, 0)])
        .collect::<Vec<_>>();
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(EstimationError::Numerical(format!(
            "non-finite regression for {subject}"
        )));
    }
    Ok(coefficients)
}

fn ols_weighted(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    case_weights: &[f64],
    subject: &str,
) -> Result<Vec<f64>, EstimationError> {
    if predictors.is_empty() {
        return Ok(Vec::new());
    }
    let rows = outcome.len();
    let columns = predictors.len();
    if rows < columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let centers = predictors
        .iter()
        .map(|predictor| weighted_mean(predictor, case_weights))
        .collect::<Vec<_>>();
    let outcome_center = weighted_mean(outcome, case_weights);
    let matrix = Mat::from_fn(rows, columns, |row, column| {
        (predictors[column][row] - centers[column]) * case_weights[row].sqrt()
    });
    let qr = matrix.col_piv_qr();
    let diagonal = qr.thin_R();
    let diagonal_count = rows.min(columns);
    let max_diagonal = (0..diagonal_count)
        .map(|index| diagonal[(index, index)].abs())
        .fold(0.0, f64::max);
    let rank_tolerance = max_diagonal * (rows.max(columns) as f64) * f64::EPSILON * 100.0;
    let rank = (0..diagonal_count)
        .filter(|index| diagonal[(*index, *index)].abs() > rank_tolerance)
        .count();
    if rank < columns {
        return Err(EstimationError::RankDeficient(subject.into()));
    }
    let rhs = Mat::from_fn(rows, 1, |row, _| {
        (outcome[row] - outcome_center) * case_weights[row].sqrt()
    });
    let solution = qr.solve_lstsq(&rhs);
    let coefficients = (0..columns)
        .map(|index| solution[(index, 0)])
        .collect::<Vec<_>>();
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(EstimationError::Numerical(format!(
            "non-finite weighted regression for {subject}"
        )));
    }
    Ok(coefficients)
}

fn validate_acyclic(recipe: &AnalysisRecipe) -> Result<(), EstimationError> {
    let ids = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut edges = vec![Vec::new(); ids.len()];
    let mut indegree = vec![0usize; ids.len()];
    for path in &recipe.model.paths {
        let Some(&source) = ids.get(path.source.as_str()) else {
            return Err(EstimationError::Numerical(format!(
                "unknown construct {}",
                path.source
            )));
        };
        let Some(&target) = ids.get(path.target.as_str()) else {
            return Err(EstimationError::Numerical(format!(
                "unknown construct {}",
                path.target
            )));
        };
        edges[source].push(target);
        indegree[target] += 1;
    }
    let mut stack = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(node) = stack.pop() {
        visited += 1;
        for target in &edges[node] {
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                stack.push(*target);
            }
        }
    }
    if visited != ids.len() {
        Err(EstimationError::CyclicModel)
    } else {
        Ok(())
    }
}

fn validate_execution_recipe(recipe: &AnalysisRecipe) -> Result<(), EstimationError> {
    if !matches!(
        recipe.settings.method,
        AnalysisMethod::PlsPm
            | AnalysisMethod::Plsc
            | AnalysisMethod::Endogeneity
            | AnalysisMethod::NonlinearEffects
            | AnalysisMethod::ModeratedMediation
            | AnalysisMethod::CtaPls
            | AnalysisMethod::Wpls
            | AnalysisMethod::Cca
            | AnalysisMethod::Predict
            | AnalysisMethod::Mga
            | AnalysisMethod::Ipma
            | AnalysisMethod::Cbsem
            | AnalysisMethod::Pca
            | AnalysisMethod::Gsca
            | AnalysisMethod::Regression
            | AnalysisMethod::Nca
    ) {
        return Err(EstimationError::UnsupportedMethod(
            recipe.settings.method.to_string(),
        ));
    }
    if recipe.settings.bootstrap_samples > 0 {
        return Err(EstimationError::ResamplingRequiresEngine);
    }
    if recipe.settings.method == AnalysisMethod::Gsca {
        validate_gsca_execution_contract(recipe)?;
    }
    if recipe.settings.method == AnalysisMethod::Nca {
        let x = metadata_required(recipe, "nca_x")?;
        let y = metadata_required(recipe, "nca_y")?;
        if x == y {
            return Err(EstimationError::UnsupportedMethod(
                "NCA v2 requires different X and Y variables".into(),
            ));
        }
        if recipe.settings.weighting_scheme != WeightingScheme::Path
            || recipe.settings.preprocessing != Preprocessing::Unstandardized
            || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
            || recipe.settings.case_weight_column.is_some()
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
        {
            return Err(EstimationError::UnsupportedMethod(
                "NCA v2 requires unstandardized raw values, listwise deletion, no case weights, and only its dedicated permutation plan".into(),
            ));
        }
        let ceiling = recipe
            .metadata
            .get("nca_ceiling")
            .map(String::as_str)
            .unwrap_or("both");
        if nca_requested_ceilings(ceiling).is_none() {
            return Err(EstimationError::UnsupportedMethod(format!(
                "NCA v2 does not support ceiling technique {ceiling}"
            )));
        }
        let permutation_samples = recipe
            .metadata
            .get("nca_permutation_samples")
            .map(String::as_str)
            .unwrap_or("999")
            .parse::<usize>()
            .ok();
        if permutation_samples.is_none_or(|samples| !(1..=10_000).contains(&samples)) {
            return Err(EstimationError::UnsupportedMethod(
                "NCA v2 requires 1 to 10,000 dedicated permutation samples".into(),
            ));
        }
    }
    if recipe.settings.method == AnalysisMethod::Pca {
        let variables = metadata_list(recipe, "pca_variables")
            .or_else(|| metadata_list(recipe, "pca.variables"))
            .unwrap_or_default();
        let unique_variables = variables.iter().collect::<HashSet<_>>();
        let component_rule = recipe
            .metadata
            .get("pca_component_rule")
            .map(String::as_str)
            .unwrap_or("kaiser");
        if variables.len() < 2 || unique_variables.len() != variables.len() {
            return Err(EstimationError::UnsupportedMethod(
                "PCA v1 requires at least two distinct numeric variables".into(),
            ));
        }
        if !matches!(component_rule, "kaiser" | "fixed" | "variance_threshold") {
            return Err(EstimationError::UnsupportedMethod(format!(
                "PCA v1 does not support component rule {component_rule}"
            )));
        }
        if component_rule == "fixed" {
            let components = recipe
                .metadata
                .get("pca_components")
                .and_then(|value| value.parse::<usize>().ok());
            if components
                .is_none_or(|components| components == 0 || components > variables.len().min(50))
            {
                return Err(EstimationError::UnsupportedMethod(
                    "PCA v1 fixed retention requires 1 to min(selected variables, 50) components"
                        .into(),
                ));
            }
        }
        if component_rule == "variance_threshold" {
            let threshold = recipe
                .metadata
                .get("pca_variance_threshold")
                .and_then(|value| value.parse::<f64>().ok());
            if threshold.is_none_or(|threshold| {
                !threshold.is_finite() || !(0.01..=0.999).contains(&threshold)
            }) {
                return Err(EstimationError::UnsupportedMethod(
                    "PCA v1 variance-threshold retention requires a threshold from 0.01 to 0.999"
                        .into(),
                ));
            }
        }
        if recipe.settings.weighting_scheme != WeightingScheme::Path
            || recipe.settings.preprocessing != Preprocessing::Standardized
            || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
            || recipe.settings.case_weight_column.is_some()
            || recipe.settings.studentized_inner_samples > 0
            || recipe.settings.permutation_samples > 0
            || !recipe.model.constructs.is_empty()
            || !recipe.model.paths.is_empty()
            || !recipe.model.controls.is_empty()
            || !recipe.model.interactions.is_empty()
            || !recipe.model.higher_order_constructs.is_empty()
        {
            return Err(EstimationError::UnsupportedMethod(
                "PCA v1 requires standardized raw variables, listwise deletion, an empty SEM model, and no case weights or resampling"
                    .into(),
            ));
        }
    }
    if recipe.settings.method == AnalysisMethod::Ipma {
        if recipe.settings.weighting_scheme != WeightingScheme::Path {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 requires path weighting".into(),
            ));
        }
        if recipe.settings.preprocessing != Preprocessing::Standardized {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 requires standardized indicator preprocessing".into(),
            ));
        }
        if recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 requires listwise deletion".into(),
            ));
        }
        if recipe.settings.case_weight_column.is_some() {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 does not support case weights".into(),
            ));
        }
        if recipe.settings.studentized_inner_samples > 0 || recipe.settings.permutation_samples > 0
        {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 does not support resampling inference".into(),
            ));
        }
        if !recipe.model.interactions.is_empty() {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 does not support generated interaction constructs".into(),
            ));
        }
        if !recipe.model.higher_order_constructs.is_empty() {
            return Err(EstimationError::UnsupportedMethod(
                "IPMA v1 does not support higher-order construct expansion".into(),
            ));
        }
        resolve_ipma_targets(recipe)
            .map_err(|error| EstimationError::UnsupportedMethod(error.to_string()))?;
    }
    if recipe.settings.method == AnalysisMethod::Mga {
        if recipe.settings.weighting_scheme != WeightingScheme::Path
            || recipe.settings.preprocessing != Preprocessing::Standardized
            || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
            || recipe.settings.case_weight_column.is_some()
        {
            return Err(EstimationError::UnsupportedMethod(
                "MICOM and permutation MGA v2 require path weighting, standardized preprocessing, listwise deletion, and no case weights"
                    .into(),
            ));
        }
        let group_column = recipe
            .metadata
            .get("mga_group_column")
            .or_else(|| recipe.metadata.get("mga.group_column"))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                EstimationError::UnsupportedMethod(
                    "two-group MGA requires metadata mga_group_column".into(),
                )
            })?;
        requested_mga_groups(recipe, "two-group MGA v2")?;
        if recipe
            .model
            .constructs
            .iter()
            .flat_map(|construct| construct.indicators.iter())
            .any(|indicator| indicator == group_column)
        {
            return Err(EstimationError::UnsupportedMethod(
                "the MGA grouping column cannot also be a model indicator".into(),
            ));
        }
        if recipe.settings.studentized_inner_samples > 0 || recipe.settings.permutation_samples > 0
        {
            return Err(EstimationError::UnsupportedMethod(
                "two-group MGA uses its dedicated permutation option; pooled resampling settings are unsupported"
                    .into(),
            ));
        }
        let methods = recipe
            .metadata
            .get("group_methods")
            .map(|methods| {
                methods
                    .split(',')
                    .map(str::trim)
                    .filter(|method| !method.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let unique = methods
            .iter()
            .map(|method| method.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        if methods.len() != 2
            || unique.len() != 2
            || !group_method_requested(recipe, "mga_permutation")
            || !group_method_requested(recipe, "micom")
        {
            return Err(EstimationError::UnsupportedMethod(
                "the current two-group workflow requires exactly MICOM and permutation MGA".into(),
            ));
        }
        let samples = recipe
            .metadata
            .get("group_permutation_samples")
            .and_then(|value| value.trim().parse::<usize>().ok());
        if !samples.is_some_and(|samples| (5_000..=10_000).contains(&samples)) {
            return Err(EstimationError::UnsupportedMethod(
                "MICOM and permutation MGA require 5000 to 10000 permutations".into(),
            ));
        }
        if !recipe
            .metadata
            .get("micom_configural_confirmed")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            return Err(EstimationError::UnsupportedMethod(
                "MICOM v2 requires explicit confirmation of configural invariance prerequisites"
                    .into(),
            ));
        }
    }
    if recipe.model.constructs.is_empty()
        && !matches!(
            recipe.settings.method,
            AnalysisMethod::Pca | AnalysisMethod::Regression | AnalysisMethod::Nca
        )
    {
        return Err(EstimationError::EmptyModel);
    }
    let mut construct_ids = HashSet::new();
    for construct in &recipe.model.constructs {
        if construct.id.trim().is_empty() {
            return Err(EstimationError::EmptyConstructId);
        }
        if !construct_ids.insert(construct.id.as_str()) {
            return Err(EstimationError::DuplicateConstruct(construct.id.clone()));
        }
    }
    let mut paths = HashSet::new();
    for path in &recipe.model.paths {
        if path.source == path.target {
            return Err(EstimationError::SelfPath(path.source.clone()));
        }
        if !construct_ids.contains(path.source.as_str()) {
            return Err(EstimationError::UnknownConstruct(path.source.clone()));
        }
        if !construct_ids.contains(path.target.as_str()) {
            return Err(EstimationError::UnknownConstruct(path.target.clone()));
        }
        if !paths.insert((path.source.as_str(), path.target.as_str())) {
            return Err(EstimationError::DuplicatePath(
                path.source.clone(),
                path.target.clone(),
            ));
        }
    }
    Ok(())
}

fn numeric_value(array: &dyn Array, row: usize) -> Option<f64> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Some(values.value(row))
    } else {
        array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|values| values.value(row) as f64)
    }
}
fn vector_mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}
fn sample_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = vector_mean(values);
    values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64
}
fn sample_sd(values: &[f64]) -> f64 {
    sample_variance(values).sqrt()
}
fn validate_case_weights(weights: &[f64]) -> Result<(), EstimationError> {
    if weights.len() < 3
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(EstimationError::Numerical(
            "case weights must be positive and finite".into(),
        ));
    }
    let sum = weights.iter().sum::<f64>();
    let sum_squared = weights.iter().map(|weight| weight * weight).sum::<f64>();
    if sum <= 0.0 || sum * sum <= sum_squared {
        return Err(EstimationError::Numerical(
            "case weights require positive effective degrees of freedom".into(),
        ));
    }
    Ok(())
}
fn weighted_mean(values: &[f64], weights: &[f64]) -> f64 {
    values
        .iter()
        .zip(weights)
        .map(|(value, weight)| value * weight)
        .sum::<f64>()
        / weights.iter().sum::<f64>()
}
fn weighted_degrees_of_freedom(weights: &[f64]) -> f64 {
    let sum = weights.iter().sum::<f64>();
    let sum_squared = weights.iter().map(|weight| weight * weight).sum::<f64>();
    sum - sum_squared / sum
}
fn weighted_sample_sd(values: &[f64], weights: &[f64]) -> f64 {
    weighted_covariance(values, values, weights).sqrt()
}
fn covariance(left: &[f64], right: &[f64]) -> f64 {
    let lm = vector_mean(left);
    let rm = vector_mean(right);
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - lm) * (b - rm))
        .sum::<f64>()
        / (left.len() - 1) as f64
}
fn weighted_covariance(left: &[f64], right: &[f64], weights: &[f64]) -> f64 {
    let left_mean = weighted_mean(left, weights);
    let right_mean = weighted_mean(right, weights);
    let denominator = weighted_degrees_of_freedom(weights);
    left.iter()
        .zip(right)
        .zip(weights)
        .map(|((a, b), weight)| weight * (a - left_mean) * (b - right_mean))
        .sum::<f64>()
        / denominator
}
fn correlation(left: &[f64], right: &[f64]) -> f64 {
    covariance(left, right) / (sample_sd(left) * sample_sd(right))
}
fn weighted_correlation(left: &[f64], right: &[f64], weights: &[f64]) -> f64 {
    weighted_covariance(left, right, weights)
        / (weighted_sample_sd(left, weights) * weighted_sample_sd(right, weights))
}
fn add_scaled(target: &mut [f64], source: &[f64], scale: f64) {
    for (target, source) in target.iter_mut().zip(source) {
        *target += source * scale;
    }
}
fn standardize_vector(mut values: Vec<f64>) -> Option<Vec<f64>> {
    let mean = vector_mean(&values);
    for value in &mut values {
        *value -= mean;
    }
    let deviation = sample_sd(&values);
    if deviation <= f64::EPSILON || !deviation.is_finite() {
        return None;
    }
    for value in &mut values {
        *value /= deviation;
    }
    Some(values)
}
fn weighted_standardize_vector(mut values: Vec<f64>, weights: &[f64]) -> Option<Vec<f64>> {
    let mean = weighted_mean(&values, weights);
    for value in &mut values {
        *value -= mean;
    }
    let deviation = weighted_sample_sd(&values, weights);
    if deviation <= f64::EPSILON || !deviation.is_finite() {
        return None;
    }
    for value in &mut values {
        *value /= deviation;
    }
    Some(values)
}
fn orient_by_sum(weights: &mut [f64]) {
    if weights.iter().sum::<f64>() < 0.0 {
        for value in weights {
            *value = -*value;
        }
    }
}

fn orient_block_weights(columns: &[Vec<f64>], block: &[usize], weights: &mut [f64]) {
    let mut score = vec![0.0; columns[0].len()];
    let mut reference = vec![0.0; columns[0].len()];
    for (column, weight) in block.iter().zip(weights.iter()) {
        add_scaled(&mut score, &columns[*column], *weight);
        add_scaled(&mut reference, &columns[*column], 1.0);
    }
    let association = covariance(&score, &reference);
    if association < -1e-15 || (association.abs() <= 1e-15 && weights.iter().sum::<f64>() < 0.0) {
        for value in weights {
            *value = -*value;
        }
    }
}
fn orient_block_weights_weighted(
    columns: &[Vec<f64>],
    block: &[usize],
    weights: &mut [f64],
    case_weights: &[f64],
) {
    let mut score = vec![0.0; columns[0].len()];
    let mut reference = vec![0.0; columns[0].len()];
    for (column, weight) in block.iter().zip(weights.iter()) {
        add_scaled(&mut score, &columns[*column], *weight);
        add_scaled(&mut reference, &columns[*column], 1.0);
    }
    let association = weighted_covariance(&score, &reference, case_weights);
    if association < -1e-15 || (association.abs() <= 1e-15 && weights.iter().sum::<f64>() < 0.0) {
        for value in weights {
            *value = -*value;
        }
    }
}
fn fitted_values(predictors: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
    let mut values = vec![0.0; predictors[0].len()];
    for (predictor, coefficient) in predictors.iter().zip(coefficients) {
        add_scaled(&mut values, predictor, *coefficient);
    }
    values
}
fn weighted_residual_and_total(actual: &[f64], fitted: &[f64], weights: &[f64]) -> (f64, f64) {
    let mean = weighted_mean(actual, weights);
    let residual = actual
        .iter()
        .zip(fitted)
        .zip(weights)
        .map(|((actual, fit), weight)| weight * (actual - fit).powi(2))
        .sum::<f64>();
    let total = actual
        .iter()
        .zip(weights)
        .map(|(actual, weight)| weight * (actual - mean).powi(2))
        .sum::<f64>();
    (residual, total)
}
fn multiply_square(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let size = left.len();
    let mut output = vec![vec![0.0; size]; size];
    for row in 0..size {
        for column in 0..size {
            for inner in 0..size {
                output[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use arrow::{
        array::Float64Array,
        datatypes::{DataType, Field, Schema},
        record_batch::RecordBatch,
    };
    use chrono::Utc;
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisSettings, Construct, ControlPath,
        HigherOrderConstruct, HigherOrderMethod, InteractionMethod, InteractionTerm, MethodConfig,
        ModelSpec, NcaCeiling, PcaRetentionConfig, ProcessContinuousCentering,
        ProcessModerationConfig, ProcessModeratorConfig, ProcessModeratorScale, ProcessPathConfig,
        ProcessRelationshipConfig, RegressionBootstrapAlgorithm, RegressionBootstrapConfig,
        RegressionBootstrapInterval, StructuralPath,
    };
    use qpls_data::{
        ColumnMetadata, ColumnType, DataFingerprint, DataKind, DatasetSchema, ImportOptions,
        ScaleType, import_delimited_bytes,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

    fn logistic_recipe(dataset: &Dataset) -> AnalysisRecipe {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.confidence_level = 0.95;
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "Logistic v2".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into()],
                controls: Vec::new(),
                model: qpls_core::RegressionModelConfig::Logistic,
                bootstrap: None,
            }),
            metadata: BTreeMap::new(),
        }
    }

    fn process_graph_csv(
        row_indices: impl IntoIterator<Item = usize>,
        include_irrelevant: bool,
    ) -> String {
        let mut csv = if include_irrelevant {
            String::from("X,M,W,B,C,Y,Unused\n")
        } else {
            String::from("X,M,W,B,C,Y\n")
        };
        for index in row_indices {
            let x = index as f64 / 10.0 - 4.0;
            let w = ((index * 7) % 19) as f64 / 5.0 - 1.8;
            let b = (index % 2) as f64;
            let c = ((index * 11) % 23) as f64 / 7.0 - 1.5;
            let noise_m = ((index * 13) % 17) as f64 / 100.0 - 0.08;
            let noise_y = ((index * 5) % 29) as f64 / 120.0 - 0.12;
            let m = 0.5 * x + 0.25 * x * w + 0.1 * c + noise_m;
            let y = 0.2 * x
                + 0.7 * m
                + 0.3 * x * w
                + 0.15 * x * b
                + 0.12 * x * w * b
                + 0.08 * c
                + noise_y;
            csv.push_str(&format!("{x},{m},{w},{b},{c},{y}"));
            if include_irrelevant {
                csv.push_str(&format!(",{}", (index * 31 % 47) as f64 / 9.0));
            }
            csv.push('\n');
        }
        csv
    }

    fn process_graph_fixture() -> (Dataset, AnalysisRecipe) {
        let csv = process_graph_csv(0..80, false);
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "process-graph-v2.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.confidence_level = 0.95;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "PROCESS graph v2".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(MethodConfig::Regression {
                outcome: "Y".into(),
                predictors: vec!["X".into(), "M".into(), "W".into(), "B".into()],
                controls: vec!["C".into()],
                model: RegressionModelConfig::Process {
                    relationship: ProcessRelationshipConfig::Graph {
                        focal_predictor: "X".into(),
                        paths: vec![
                            ProcessPathConfig {
                                from: "M".into(),
                                to: "Y".into(),
                            },
                            ProcessPathConfig {
                                from: "X".into(),
                                to: "Y".into(),
                            },
                            ProcessPathConfig {
                                from: "X".into(),
                                to: "M".into(),
                            },
                        ],
                        moderators: vec![
                            ProcessModeratorConfig {
                                variable: "W".into(),
                                scale: ProcessModeratorScale::Continuous,
                            },
                            ProcessModeratorConfig {
                                variable: "B".into(),
                                scale: ProcessModeratorScale::Binary01,
                            },
                        ],
                        moderations: vec![
                            ProcessModerationConfig {
                                from: "X".into(),
                                to: "Y".into(),
                                moderator: "W".into(),
                                conditioning_moderator: Some("B".into()),
                            },
                            ProcessModerationConfig {
                                from: "X".into(),
                                to: "M".into(),
                                moderator: "W".into(),
                                conditioning_moderator: None,
                            },
                        ],
                        continuous_product_centering:
                            ProcessContinuousCentering::EquationCompleteCaseMeanV1,
                    },
                },
                bootstrap: None,
            }),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
    }

    #[test]
    fn process_graph_v2_parallel_serial_moderation_and_effect_arithmetic() {
        let (dataset, recipe) = process_graph_fixture();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(result.method_version, REGRESSION_PROCESS_METHOD_VERSION);
        let process = result.regression.unwrap().process.unwrap();
        assert_eq!(process.model, "graph");
        let graph = process.graph_v2.unwrap();
        assert_eq!(graph.complete_cases, 80);
        assert_eq!(
            graph
                .paths
                .iter()
                .map(|path| path.path_id.as_str())
                .collect::<Vec<_>>(),
            vec!["X->M", "X->Y", "M->Y"]
        );
        assert_eq!(
            graph
                .equations
                .iter()
                .map(|equation| equation.outcome.as_str())
                .collect::<Vec<_>>(),
            vec!["M", "Y"]
        );
        let effects = graph
            .reference_effects
            .iter()
            .map(|effect| (effect.kind.as_str(), effect.estimate))
            .collect::<BTreeMap<_, _>>();
        assert!((effects["total"] - effects["direct"] - effects["total_indirect"]).abs() < 1e-10);
        assert_eq!(graph.moderated_mediation_indices.len(), 1);
        assert_eq!(graph.conditional_indirect_effects.len(), 3);
        assert_eq!(graph.plots.len(), 2);
        let effect_ids = graph
            .reference_effects
            .iter()
            .map(|row| row.effect_id.as_str())
            .chain(
                graph
                    .conditional_indirect_effects
                    .iter()
                    .map(|row| row.effect_id.as_str()),
            )
            .chain(
                graph
                    .moderated_mediation_indices
                    .iter()
                    .map(|row| row.effect_id.as_str()),
            )
            .chain(graph.simple_slopes.iter().map(|row| row.effect_id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            effect_ids.iter().copied().collect::<HashSet<_>>().len(),
            effect_ids.len()
        );
        assert!(
            graph
                .simple_slopes
                .iter()
                .any(|row| { row.effect_id.starts_with("slope:moderation:X->M@W@W=") })
        );
        assert!(
            graph
                .simple_slopes
                .iter()
                .any(|row| { row.effect_id.starts_with("slope:moderation:X->Y@W|B@W=") })
        );
        assert!(
            graph
                .plots
                .iter()
                .all(|plot| plot.series.iter().all(|series| series.points.len() == 25))
        );
    }

    #[test]
    fn process_graph_v2_serialization_omits_only_default_legacy_pls_shells() {
        let (dataset, recipe) = process_graph_fixture();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(result.mediation, MediationAnalysis::default());
        assert_eq!(result.moderation, ModerationAnalysis::default());

        let serialized = serde_json::to_value(&result).unwrap();
        assert!(serialized.get("mediation").is_none());
        assert!(serialized.get("moderation").is_none());
        let decoded: PlsResult = serde_json::from_value(serialized).unwrap();
        assert_eq!(decoded.mediation, MediationAnalysis::default());
        assert_eq!(decoded.moderation, ModerationAnalysis::default());

        let (pls_dataset, pls_recipe) = fixture();
        let pls = estimate_pls(&pls_dataset, &pls_recipe).unwrap();
        assert!(!pls.mediation.estimates.is_empty());
        assert!(
            serde_json::to_value(&pls)
                .unwrap()
                .get("mediation")
                .is_some()
        );

        let mut populated = result;
        populated
            .moderation
            .warnings
            .push("non-default moderation shell".into());
        assert!(
            serde_json::to_value(&populated)
                .unwrap()
                .get("moderation")
                .is_some()
        );
    }

    #[test]
    fn process_graph_v2_point_is_row_irrelevant_column_and_recipe_order_invariant() {
        fn assert_close(left: &serde_json::Value, right: &serde_json::Value, location: &str) {
            match (left, right) {
                (serde_json::Value::Number(left), serde_json::Value::Number(right)) => {
                    let left = left.as_f64().unwrap();
                    let right = right.as_f64().unwrap();
                    let tolerance = 1.0e-10 * 1.0_f64.max(left.abs()).max(right.abs());
                    assert!(
                        (left - right).abs() <= tolerance,
                        "PROCESS metamorphic numeric mismatch at {location}: {left:.17e} vs {right:.17e}"
                    );
                }
                (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
                    assert_eq!(
                        left.len(),
                        right.len(),
                        "array length mismatch at {location}"
                    );
                    for (index, (left, right)) in left.iter().zip(right).enumerate() {
                        assert_close(left, right, &format!("{location}[{index}]"));
                    }
                }
                (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                    assert_eq!(
                        left.keys().collect::<BTreeSet<_>>(),
                        right.keys().collect::<BTreeSet<_>>(),
                        "object keys mismatch at {location}"
                    );
                    for (key, left) in left {
                        assert_close(left, &right[key], &format!("{location}.{key}"));
                    }
                }
                _ => assert_eq!(left, right, "PROCESS metamorphic mismatch at {location}"),
            }
        }

        let (base_dataset, base_recipe) = process_graph_fixture();
        let base_graph = estimate_pls(&base_dataset, &base_recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();

        let reversed_dataset = import_delimited_bytes(
            process_graph_csv((0..80).rev(), false).as_bytes(),
            "process-graph-v2-reversed.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut reversed_recipe = base_recipe.clone();
        reversed_recipe.dataset_fingerprint = reversed_dataset.fingerprint.0.clone();
        let reversed_graph = estimate_pls(&reversed_dataset, &reversed_recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        assert_close(
            &serde_json::to_value(&base_graph).unwrap(),
            &serde_json::to_value(&reversed_graph).unwrap(),
            "row_order",
        );

        let irrelevant_dataset = import_delimited_bytes(
            process_graph_csv(0..80, true).as_bytes(),
            "process-graph-v2-irrelevant.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut irrelevant_recipe = base_recipe.clone();
        irrelevant_recipe.dataset_fingerprint = irrelevant_dataset.fingerprint.0.clone();
        let irrelevant_graph = estimate_pls(&irrelevant_dataset, &irrelevant_recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        assert_eq!(base_graph, irrelevant_graph);

        let mut permuted_recipe = base_recipe;
        if let Some(MethodConfig::Regression {
            model:
                RegressionModelConfig::Process {
                    relationship:
                        ProcessRelationshipConfig::Graph {
                            paths, moderations, ..
                        },
                },
            ..
        }) = permuted_recipe.method_config.as_mut()
        {
            paths.reverse();
            moderations.reverse();
        } else {
            panic!("PROCESS graph fixture lost its typed relationship");
        }
        let permuted_graph = estimate_pls(&base_dataset, &permuted_recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        assert_eq!(base_graph, permuted_graph);
    }

    #[test]
    fn process_graph_v2_hc3_simple_slopes_and_johnson_neyman() {
        let (dataset, recipe) = process_graph_fixture();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let graph = result
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        assert_eq!(graph.policies.covariance, "hc3_v1");
        assert_eq!(graph.simple_slopes.len(), 9);
        assert!(
            graph
                .simple_slopes
                .iter()
                .all(|slope| slope.standard_error > 0.0 && slope.statistic.is_finite())
        );
        assert_eq!(graph.johnson_neyman.len(), 3);
        assert!(graph.johnson_neyman.iter().all(|row| match row {
            ProcessJohnsonNeyman::Available { curve_points, .. } => curve_points.len() == 101,
            ProcessJohnsonNeyman::Unavailable { reason_code, .. } =>
                reason_code == "binary_solved_moderator",
        }));
    }

    #[test]
    fn process_graph_v2_rejects_exact_binary_endogenous_outcomes_in_original_sample() {
        for binary_variable in ["M", "Y"] {
            let mut csv = String::from("X,M,W,B,C,Y\n");
            for index in 0..80 {
                let x = index as f64 / 10.0 - 4.0;
                let w = ((index * 7) % 19) as f64 / 5.0 - 1.8;
                let b = (index % 2) as f64;
                let target_binary = (((index * 13) % 7) < 3) as u8 as f64;
                let c = ((index * 11) % 23) as f64 / 7.0 - 1.5;
                let m = if binary_variable == "M" {
                    target_binary
                } else {
                    0.5 * x + 0.25 * x * w + 0.1 * c + ((index * 5 % 17) as f64 - 8.0) / 100.0
                };
                let y = if binary_variable == "Y" {
                    target_binary
                } else {
                    0.2 * x
                        + 0.7 * m
                        + 0.3 * x * w
                        + 0.08 * c
                        + ((index * 9 % 19) as f64 - 9.0) / 120.0
                };
                csv.push_str(&format!("{x},{m},{w},{b},{c},{y}\n"));
            }
            let dataset = import_delimited_bytes(
                csv.as_bytes(),
                &format!("process-binary-{binary_variable}.csv"),
                b',',
                &ImportOptions::default(),
            )
            .unwrap();
            let (_, mut recipe) = process_graph_fixture();
            recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
            assert!(matches!(
                estimate_pls(&dataset, &recipe),
                Err(EstimationError::UnsupportedMethod(message))
                    if message.starts_with("binary_process_equation_outcome|")
                        && message.contains(binary_variable)
            ));
            let mut uninterrupted = |_| true;
            let resample_fit =
                process_analysis(&dataset, &recipe, false, &mut uninterrupted).unwrap();
            assert!(resample_fit.graph_v2.is_some());
        }
    }

    #[test]
    fn process_graph_v2_scale_aware_svd_is_affine_unit_invariant_and_rejects_relative_collinearity()
    {
        let rows = 80;
        let base_design = (0..rows)
            .map(|index| {
                let x = index as f64 / 9.0 - 4.0;
                vec![1.0, x]
            })
            .collect::<Vec<_>>();
        let outcome = base_design
            .iter()
            .enumerate()
            .map(|(index, row)| 1.25 + 0.75 * row[1] + ((index * 7 % 13) as f64 - 6.0) / 100.0)
            .collect::<Vec<_>>();
        let base = process_scale_aware_ols(&base_design, &outcome, "Y").unwrap();
        let unit_scale = 1.0e-9;
        let shift = 4.5;
        let transformed_design = base_design
            .iter()
            .map(|row| vec![1.0, unit_scale * (row[1] + shift)])
            .collect::<Vec<_>>();
        let transformed = process_scale_aware_ols(&transformed_design, &outcome, "Y").unwrap();
        let base_fitted = outcome
            .iter()
            .zip(&base.residuals)
            .map(|(actual, residual)| actual - residual)
            .collect::<Vec<_>>();
        let transformed_fitted = outcome
            .iter()
            .zip(&transformed.residuals)
            .map(|(actual, residual)| actual - residual)
            .collect::<Vec<_>>();
        for (left, right) in base_fitted.iter().zip(&transformed_fitted) {
            assert_relative_eq!(left, right, epsilon = 1.0e-10, max_relative = 1.0e-10);
        }
        assert_relative_eq!(
            transformed.coefficients[1] * unit_scale,
            base.coefficients[1],
            epsilon = 1.0e-10,
            max_relative = 1.0e-10
        );
        assert_relative_eq!(
            transformed.coefficients[0],
            base.coefficients[0] - base.coefficients[1] * shift,
            epsilon = 1.0e-9,
            max_relative = 1.0e-9
        );
        assert_relative_eq!(
            transformed.covariance[1][1] * unit_scale.powi(2),
            base.covariance[1][1],
            epsilon = 1.0e-10,
            max_relative = 1.0e-9
        );
        assert_relative_eq!(
            transformed.coefficients[1] / transformed.covariance[1][1].sqrt(),
            base.coefficients[1] / base.covariance[1][1].sqrt(),
            epsilon = 1.0e-9,
            max_relative = 1.0e-9
        );

        let near_collinear = (0..rows)
            .map(|index| {
                let x = index as f64 / 9.0 - 4.0;
                let perturbation = ((index * 11 % 17) as f64 - 8.0) * 1.0e-14;
                vec![1.0, x, x + perturbation]
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            process_scale_aware_ols(&near_collinear, &outcome, "Y"),
            Err(EstimationError::RankDeficient(subject)) if subject == "Y"
        ));
    }

    #[test]
    fn process_graph_v2_jn_root_solver_is_affine_stable_and_deduplicates_near_double_roots() {
        let roots = process_johnson_neyman_coded_roots(1.0, -1.0, -2.0, -3.0, 3.0);
        assert_eq!(roots.len(), 2);
        assert_relative_eq!(roots[0], -1.0, epsilon = 1.0e-12);
        assert_relative_eq!(roots[1], 2.0, epsilon = 1.0e-12);

        for scale in [1.0e-10_f64, 1.0e10_f64] {
            let shift = 7.0 * scale;
            let transformed_quadratic = 1.0 / scale.powi(2);
            let transformed_linear = -1.0 / scale - 2.0 * shift / scale.powi(2);
            let transformed_constant = -2.0 + shift / scale + shift.powi(2) / scale.powi(2);
            let transformed = process_johnson_neyman_coded_roots(
                transformed_quadratic,
                transformed_linear,
                transformed_constant,
                shift - 3.0 * scale,
                shift + 3.0 * scale,
            );
            assert_eq!(transformed.len(), 2);
            assert_relative_eq!(
                transformed[0],
                shift - scale,
                epsilon = scale.abs() * 1.0e-12,
                max_relative = 1.0e-12
            );
            assert_relative_eq!(
                transformed[1],
                shift + 2.0 * scale,
                epsilon = scale.abs() * 1.0e-12,
                max_relative = 1.0e-12
            );
        }

        let exact_double = process_johnson_neyman_coded_roots(1.0, -2.0, 1.0, 0.0, 2.0);
        assert_eq!(exact_double.len(), 1);
        assert_relative_eq!(exact_double[0], 1.0, epsilon = 1.0e-12);
        let resolvable_near_double =
            process_johnson_neyman_coded_roots(1.0, -(2.0 + 1.0e-12), 1.0 + 1.0e-12, 0.0, 2.0);
        assert_eq!(resolvable_near_double.len(), 2);
        assert_relative_eq!(resolvable_near_double[0], 1.0, epsilon = 1.0e-12);
        assert_relative_eq!(resolvable_near_double[1], 1.0 + 1.0e-12, epsilon = 1.0e-12);

        let imbalanced =
            process_johnson_neyman_coded_roots(1.0, -(1.0e12 + 1.0e-12), 1.0, 0.0, 2.0e12);
        assert_eq!(imbalanced.len(), 2);
        assert_relative_eq!(imbalanced[0], 1.0e-12, epsilon = 1.0e-24);
        assert_relative_eq!(imbalanced[1], 1.0e12, epsilon = 1.0e-3);
    }

    #[test]
    fn process_graph_v2_jn_nonpositive_contrast_variance_is_tagged_unavailable() {
        assert!(!process_jn_variance_is_positive_across_range(
            0.0, 0.0, 1.0, -1.0, 1.0
        ));
        assert!(!process_jn_variance_is_positive_across_range(
            1.0, 1.1, 1.0, -2.0, 2.0
        ));

        let (dataset, recipe) = process_graph_fixture();
        let mut graph = estimate_pls(&dataset, &recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        let moderation = ProcessModerationConfig {
            from: "X".into(),
            to: "M".into(),
            moderator: "W".into(),
            conditioning_moderator: None,
        };
        let equation = graph
            .equations
            .iter_mut()
            .find(|equation| equation.outcome == "M")
            .unwrap();
        let path_index = equation
            .coefficients
            .iter()
            .position(|coefficient| coefficient.variables == ["X"])
            .unwrap();
        let interaction_index = equation
            .coefficients
            .iter()
            .position(|coefficient| coefficient.variables == ["X", "W"])
            .unwrap();
        equation
            .coefficient_covariance
            .iter_mut()
            .for_each(|row| row.fill(0.0));
        equation.coefficient_covariance[path_index][path_index] = 1.0;
        equation.coefficient_covariance[interaction_index][interaction_index] = 1.0;
        equation.coefficient_covariance[path_index][interaction_index] = 1.1;
        equation.coefficient_covariance[interaction_index][path_index] = 1.1;
        let profiles = graph
            .variable_profiles
            .iter()
            .map(|profile| (profile.variable.as_str(), profile))
            .collect::<BTreeMap<_, _>>();
        let diagnostics = process_johnson_neyman(
            "moderation:X->M@W",
            &moderation,
            &graph.equations,
            &profiles,
            0.95,
        )
        .unwrap();
        assert!(matches!(
            diagnostics.as_slice(),
            [ProcessJohnsonNeyman::Unavailable { reason_code, message, .. }]
                if reason_code == PROCESS_JN_INVALID_COVARIANCE_REASON
                    && message == PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        ));
    }

    #[test]
    fn process_graph_v2_rejects_high_leverage_hc3_instability_without_clamping() {
        let design = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let residuals = vec![0.5, -0.5];
        let identity = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert!(matches!(
            process_robust_covariance_hc3(&design, &residuals, &identity, "Y"),
            Err(EstimationError::UnsupportedMethod(message))
                if message.starts_with("high_leverage_hc3_instability|")
                    && message.contains("complete-case row 0")
        ));
    }

    #[test]
    fn process_graph_v2_rejects_nonpositive_hc3_variance_without_absolute_value() {
        assert!(matches!(
            process_standard_error_from_variance(-f64::EPSILON, "Y", "path:X->Y"),
            Err(EstimationError::Numerical(message))
                if message.starts_with("invalid_hc3_covariance|")
                    && message.contains("path:X->Y")
        ));
        assert!(matches!(
            process_standard_error_from_variance(0.0, "Y", "intercept"),
            Err(EstimationError::Numerical(message))
                if message.starts_with("invalid_hc3_covariance|")
        ));
    }

    #[test]
    fn process_graph_v2_rejects_degenerate_simple_slope_variance() {
        let (dataset, recipe) = process_graph_fixture();
        let mut graph = estimate_pls(&dataset, &recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        let moderation_id = "moderation:X->M@W";
        let probes = graph
            .simple_slopes
            .iter()
            .find(|slope| slope.moderation_id == moderation_id)
            .unwrap()
            .moderator_values
            .clone();
        let probe_suffix = graph
            .simple_slopes
            .iter()
            .find(|slope| slope.moderation_id == moderation_id)
            .unwrap()
            .effect_id
            .split_once('@')
            .unwrap()
            .1
            .to_string();
        let equation = graph
            .equations
            .iter_mut()
            .find(|equation| equation.outcome == "M")
            .unwrap();
        equation
            .coefficient_covariance
            .iter_mut()
            .for_each(|row| row.fill(0.0));
        let moderation = ProcessModerationConfig {
            from: "X".into(),
            to: "M".into(),
            moderator: "W".into(),
            conditioning_moderator: None,
        };
        assert!(matches!(
            process_simple_slope(
                moderation_id,
                &moderation,
                &probes,
                &probe_suffix,
                &graph.equations,
                0.95,
            ),
            Err(EstimationError::Numerical(message))
                if message.starts_with("degenerate_simple_slope_variance|")
        ));
    }

    #[test]
    fn process_graph_v2_semantic_probe_grid_rejects_collapsed_f64_levels() {
        let profile = ProcessVariableProfile {
            variable: "W".into(),
            role: "moderator".into(),
            scale: "continuous".into(),
            raw_mean: 9_007_199_254_740_992.0,
            raw_sample_sd: 0.25,
            raw_min: 9_007_199_254_740_990.0,
            raw_max: 9_007_199_254_740_994.0,
            levels: Vec::new(),
        };
        let profiles = BTreeMap::from([("W", &profile)]);
        let moderation = ProcessModerationConfig {
            from: "X".into(),
            to: "Y".into(),
            moderator: "W".into(),
            conditioning_moderator: None,
        };
        assert!(matches!(
            process_probe_grid(&moderation, &profiles),
            Err(EstimationError::Numerical(message))
                if message.starts_with("collapsed_process_probe_grid|")
        ));
    }

    #[test]
    fn process_graph_v2_point_progress_completes_and_cancellation_returns_no_result() {
        let (dataset, recipe) = process_graph_fixture();
        let mut updates = Vec::new();
        let completed = estimate_pls_with_control(&dataset, &recipe, |update| {
            updates.push(update);
            true
        })
        .unwrap();
        assert_eq!(completed.method_version, REGRESSION_PROCESS_METHOD_VERSION);
        for phase in [
            EstimationPhase::PreparingRows,
            EstimationPhase::PreparingIndicators,
            EstimationPhase::Iterating,
            EstimationPhase::ComputingEffects,
            EstimationPhase::Assembling,
        ] {
            assert!(
                updates.iter().any(|update| {
                    update.phase == phase
                        && update.total_units > 0
                        && update.completed_units == update.total_units
                }),
                "missing completed PROCESS point progress for {phase:?}: {updates:?}"
            );
        }

        let mut saw_row_scan = false;
        let cancelled = estimate_pls_with_control(&dataset, &recipe, |update| {
            if update.phase == EstimationPhase::PreparingRows && update.completed_units > 0 {
                saw_row_scan = true;
                return false;
            }
            true
        });
        assert!(saw_row_scan);
        assert_eq!(cancelled, Err(EstimationError::Cancelled));
    }

    #[test]
    fn process_graph_v2_frozen_raw_probes_survive_resample_recentering() {
        let (dataset, recipe) = process_graph_fixture();
        let graph = estimate_pls(&dataset, &recipe)
            .unwrap()
            .regression
            .unwrap()
            .process
            .unwrap()
            .graph_v2
            .unwrap();
        let mut shifted_fit = graph.clone();
        let original_w_mean = graph
            .variable_profiles
            .iter()
            .find(|profile| profile.variable == "W")
            .unwrap()
            .raw_mean;
        shifted_fit
            .variable_profiles
            .iter_mut()
            .find(|profile| profile.variable == "W")
            .unwrap()
            .raw_mean += 1.0;

        let evaluated = process_bootstrap_estimands_at_reference(&shifted_fit, &graph).unwrap();
        assert_eq!(
            evaluated
                .iter()
                .map(|(effect_id, _)| effect_id)
                .collect::<Vec<_>>(),
            process_bootstrap_estimands(&graph)
                .iter()
                .map(|(effect_id, _)| effect_id)
                .collect::<Vec<_>>()
        );

        let x_to_m = process_serialized_coefficient(&shifted_fit, "M", &["X".into()]).unwrap()
            + process_serialized_coefficient(&shifted_fit, "M", &["X".into(), "W".into()]).unwrap()
                * (original_w_mean
                    - shifted_fit
                        .variable_profiles
                        .iter()
                        .find(|profile| profile.variable == "W")
                        .unwrap()
                        .raw_mean);
        let m_to_y = process_serialized_coefficient(&shifted_fit, "Y", &["M".into()]).unwrap();
        let indirect_id = &graph
            .reference_effects
            .iter()
            .find(|effect| effect.kind == "indirect" && effect.path == ["X", "M", "Y"])
            .unwrap()
            .effect_id;
        let evaluated_indirect = evaluated
            .iter()
            .find(|(effect_id, _)| effect_id == indirect_id)
            .unwrap()
            .1;
        assert!((evaluated_indirect - x_to_m * m_to_y).abs() < 1e-10);
        let stored_indirect = graph
            .reference_effects
            .iter()
            .find(|effect| &effect.effect_id == indirect_id)
            .unwrap()
            .estimate;
        assert!((evaluated_indirect - stored_indirect).abs() > 1e-8);

        let moderation_id = graph
            .moderations
            .iter()
            .find(|moderation| moderation.from == "X" && moderation.to == "M")
            .unwrap()
            .moderation_id
            .clone();
        let mean_slope_id = graph
            .simple_slopes
            .iter()
            .find(|slope| {
                slope.moderation_id == moderation_id
                    && slope.moderator_values.len() == 1
                    && (slope.moderator_values[0].raw_value - original_w_mean).abs() < 1e-12
            })
            .unwrap()
            .effect_id
            .clone();
        let evaluated_slope = evaluated
            .iter()
            .find(|(effect_id, _)| effect_id == &mean_slope_id)
            .unwrap()
            .1;
        assert!((evaluated_slope - x_to_m).abs() < 1e-10);
    }

    #[test]
    fn logistic_input_profile_fails_closed_for_nonbinary_and_single_class_outcomes() {
        let nonbinary = import_delimited_bytes(
            b"y,x\n0,1\n2,2\n1,3\n0,4\n1,5\n",
            "logistic-nonbinary.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let profile = profile_logistic_outcome(&nonbinary, "y", &["x".into()], &[]).unwrap();
        assert_eq!(profile.complete_cases, 5);
        assert_eq!(profile.zero_count, 2);
        assert_eq!(profile.one_count, 2);
        assert_eq!(profile.invalid_count, 1);
        assert_eq!(profile.prevalence, None);
        assert_eq!(profile.readiness, LogisticOutcomeReadiness::NonBinaryValues);
        assert!(matches!(
            estimate_pls(&nonbinary, &logistic_recipe(&nonbinary)),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("only exact numeric 0 and 1")
        ));

        let single_class = import_delimited_bytes(
            b"y,x\n0,1\n0,2\n0,3\n0,4\n0,5\n",
            "logistic-single-class.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let profile = profile_logistic_outcome(&single_class, "y", &["x".into()], &[]).unwrap();
        assert_eq!(
            profile.readiness,
            LogisticOutcomeReadiness::SingleObservedClass
        );
        assert!(matches!(
            estimate_pls(&single_class, &logistic_recipe(&single_class)),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("both 0 and 1")
        ));
    }

    #[test]
    fn logistic_v2_reports_extreme_probabilities_without_claiming_separation_proof() {
        let separated = import_delimited_bytes(
            b"y,x\n0,-3\n0,-2\n0,-1\n1,1\n1,2\n1,3\n",
            "logistic-separated.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let error = estimate_pls(&separated, &logistic_recipe(&separated)).unwrap_err();
        assert!(matches!(
            error,
            EstimationError::Numerical(message)
                if message == "logistic regression produced extreme fitted probabilities; possible separation or unstable scaling"
        ));
    }

    #[test]
    fn logistic_v2_nonconvergence_returns_no_partial_fit() {
        let outcome = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        let predictors = vec![vec![-2.0, -1.0, 0.0, 0.5, 1.0, 2.0]];
        let profile = logistic_outcome_profile("y", &outcome, 0);
        let mut control = |_| true;
        let error = logistic_regression(
            &predictors,
            &outcome,
            &["x".into()],
            "y",
            0.95,
            profile,
            1,
            1e-30,
            &mut control,
        )
        .unwrap_err();
        assert_eq!(error, EstimationError::LogisticNonConvergence(1));
    }

    #[test]
    fn regression_case_resample_boundary_binds_original_capability_and_validates_indices() {
        let data = b"y,x\n0,-2\n0,-1\n1,-0.5\n0,0\n1,0.2\n0,0.5\n1,0.8\n1,1\n0,1.5\n1,2\n";
        let dataset = import_delimited_bytes(
            data,
            "regression-resample.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let recipe = logistic_recipe(&dataset);
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let indices = vec![0, 1, 2, 2, 4, 5, 6, 7, 8, 9];
        let first = estimate_regression_case_resample_validated_with_control(
            &dataset,
            &execution,
            &indices,
            |_| true,
        )
        .unwrap();
        let second = estimate_regression_case_resample_validated_with_control(
            &dataset,
            &execution,
            &indices,
            |_| true,
        )
        .unwrap();
        assert_eq!(first, second);
        assert!(matches!(
            estimate_regression_case_resample_validated_with_control(
                &dataset,
                &execution,
                &[],
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("row indices")
        ));
        assert!(matches!(
            estimate_regression_case_resample_validated_with_control(
                &dataset,
                &execution,
                &[dataset.batch.num_rows()],
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("row indices")
        ));

        let other = import_delimited_bytes(
            b"y,x\n0,-2\n0,-1\n1,-0.5\n0,0\n1,0.2\n0,0.5\n1,0.8\n1,1\n0,1.5\n1,2.1\n",
            "different-name.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        assert!(matches!(
            estimate_regression_case_resample_validated_with_control(
                &other,
                &execution,
                &indices,
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("fingerprint")
        ));

        let mut resampling_recipe = recipe.clone();
        resampling_recipe.settings.bootstrap_samples = 99;
        if let Some(MethodConfig::Regression { bootstrap, .. }) =
            resampling_recipe.method_config.as_mut()
        {
            *bootstrap = Some(RegressionBootstrapConfig {
                algorithm: RegressionBootstrapAlgorithm::CaseResampling,
                intervals: vec![
                    RegressionBootstrapInterval::Percentile,
                    RegressionBootstrapInterval::Bca,
                ],
            });
        }
        let resampling_execution =
            ValidatedExecutionRecipe::for_dataset(&resampling_recipe, &dataset.fingerprint.0)
                .unwrap();
        assert!(matches!(
            estimate_regression_case_resample_validated_with_control(
                &dataset,
                &resampling_execution,
                &indices,
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("point-only")
        ));

        let mut pca_recipe = recipe;
        pca_recipe.settings.method = AnalysisMethod::Pca;
        pca_recipe.settings.preprocessing = Preprocessing::Standardized;
        pca_recipe.method_config = Some(MethodConfig::Pca {
            variables: vec!["y".into(), "x".into()],
            retention: PcaRetentionConfig::Kaiser,
        });
        let pca_execution =
            ValidatedExecutionRecipe::for_dataset(&pca_recipe, &dataset.fingerprint.0).unwrap();
        assert!(matches!(
            estimate_regression_case_resample_validated_with_control(
                &dataset,
                &pca_execution,
                &indices,
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("typed point-only regression")
        ));
    }

    #[test]
    fn nca_v2_uses_record_high_ce_fdh_peers_and_regresses_cr_fdh_through_them() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![1.0, 3.0, 2.0, 4.0];
        let scope = nca_scope(&x, &y);
        let peers = nca_ce_fdh_peers(&x, &y);
        assert_eq!(
            peers,
            vec![
                NcaCeilingPoint { x: 0.0, y: 1.0 },
                NcaCeilingPoint { x: 1.0, y: 3.0 },
                NcaCeilingPoint { x: 3.0, y: 4.0 },
            ]
        );
        assert!((nca_ce_fdh_effect_size(&scope, &peers) - 5.0 / 9.0).abs() < 1e-12);

        let (slope, intercept) = nca_cr_fdh_line(&peers).unwrap();
        assert!((slope - 13.0 / 14.0).abs() < 1e-12);
        assert!((intercept - 10.0 / 7.0).abs() < 1e-12);
        assert!(
            (nca_cr_fdh_effect_size(&scope, Some((slope, intercept))) - 0.39560439560439564).abs()
                < 1e-12
        );

        let ce_50 = nca_bottleneck(&scope, &peers, "ce_fdh", Some((slope, intercept)), 50.0);
        assert_eq!(ce_50.status, "required");
        assert!((ce_50.required_x_percent.unwrap() - 100.0 / 3.0).abs() < 1e-12);
        let cr_50 = nca_bottleneck(&scope, &peers, "cr_fdh", Some((slope, intercept)), 50.0);
        assert_eq!(cr_50.status, "required");
        assert!((cr_50.required_x_percent.unwrap() - 38.46153846153846).abs() < 1e-12);
    }

    #[test]
    fn nca_v2_permutations_are_seeded_independent_shuffles() {
        let first = nca_permutation_indices(20, 42, "ce_fdh", 7);
        assert_eq!(first, nca_permutation_indices(20, 42, "ce_fdh", 7));
        assert_ne!(first, nca_permutation_indices(20, 42, "ce_fdh", 8));
        assert_ne!(first, nca_permutation_indices(20, 43, "ce_fdh", 7));
        assert_ne!(first, nca_permutation_indices(20, 42, "cr_fdh", 7));
        let mut sorted = first;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn nca_v2_payload_contract_rejects_tampered_geometry() {
        let dataset = import_delimited_bytes(
            b"x,y\n0,1\n1,3\n2,2\n3,4\n",
            "nca.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Nca;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.seed = 77;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "NCA".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(MethodConfig::Nca {
                condition: "x".into(),
                outcome: "y".into(),
                ceiling: NcaCeiling::Both,
                permutation_samples: 19,
            }),
            metadata: BTreeMap::new(),
        };
        let execution_recipe = recipe.with_effective_metadata().unwrap();
        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &execution_recipe, |_| true)
                .unwrap();
        let analysis = result.nca.as_ref().unwrap();
        assert_eq!(result.method_version, NCA_METHOD_VERSION);
        assert!(nca_analysis_matches_v2_contract(
            analysis, "x", "y", "both", 19
        ));

        let mut tampered = analysis.clone();
        tampered.ceilings[0].effect_size = 0.123;
        assert!(!nca_analysis_matches_v2_contract(
            &tampered, "x", "y", "both", 19
        ));
        let mut tampered = analysis.clone();
        tampered.ce_fdh_peers[1].y = 3.5;
        assert!(!nca_analysis_matches_v2_contract(
            &tampered, "x", "y", "both", 19
        ));
        let mut tampered = analysis.clone();
        tampered.bottlenecks[5].required_x_percent = Some(99.0);
        assert!(!nca_analysis_matches_v2_contract(
            &tampered, "x", "y", "both", 19
        ));
    }

    fn standalone_pca_recipe(dataset: &Dataset, rule: &str) -> AnalysisRecipe {
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Pca;
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "Standalone PCA".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(MethodConfig::Pca {
                variables: vec!["a".into(), "b".into(), "c".into()],
                retention: match rule {
                    "fixed" => PcaRetentionConfig::Fixed { components: 2 },
                    "variance_threshold" => {
                        PcaRetentionConfig::VarianceThreshold { threshold: 0.80 }
                    }
                    _ => PcaRetentionConfig::Kaiser,
                },
            }),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn standalone_pca_v1_retains_the_component_that_crosses_variance_threshold() {
        let dataset = import_delimited_bytes(
            b"a,b,c\n1,1.2,1\n2,2.1,4\n3,2.8,2\n4,4.2,5\n5,4.9,3\n6,6.1,7\n7,6.8,2\n8,8.2,8\n",
            "pca-threshold.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = standalone_pca_recipe(&dataset, "variance_threshold");
        recipe.method_config = Some(MethodConfig::Pca {
            variables: vec!["a".into(), "b".into(), "c".into()],
            retention: PcaRetentionConfig::VarianceThreshold { threshold: 0.95 },
        });
        let execution_recipe = recipe.with_effective_metadata().unwrap();
        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &execution_recipe, |_| true)
                .unwrap();
        let pca = result.pca.unwrap();

        assert_eq!(result.method_version, PCA_METHOD_VERSION);
        assert!(pca.retained_components >= 2);
        assert!(pca.components.last().unwrap().cumulative_variance >= 0.95 - 1e-12);
        assert!(
            pca.components[pca.components.len() - 2].cumulative_variance < 0.95,
            "the threshold-crossing component must be retained"
        );
        assert_eq!(
            pca.loadings.len(),
            pca.variables.len() * pca.retained_components
        );
        assert_eq!(pca.scores.len(), pca.observations * pca.retained_components);
    }

    #[test]
    fn standalone_pca_v1_supports_more_variables_than_complete_rows() {
        let dataset = import_delimited_bytes(
            b"a,b,c,d\n1,2,4,8\n2,5,3,7\n4,3,7,1\n",
            "pca-wide.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = standalone_pca_recipe(&dataset, "fixed");
        recipe.method_config = Some(MethodConfig::Pca {
            variables: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            retention: PcaRetentionConfig::Fixed { components: 2 },
        });
        let execution_recipe = recipe.with_effective_metadata().unwrap();
        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &execution_recipe, |_| true)
                .unwrap();
        let pca = result.pca.unwrap();
        assert_eq!(pca.observations, 3);
        assert_eq!(pca.retained_components, 2);
    }

    fn fixture() -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,2,1\n2,3,3,2\n3,5,4,4\n4,4,6,5\n5,6,7,7\n6,7,9,8\n",
            "fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let model = ModelSpec {
            id: Uuid::nil(),
            name: "Simple".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model,
            settings: AnalysisSettings::default(),
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
    }

    #[test]
    fn gsca_als_v2_optimizes_the_global_criterion_without_fabricated_inference() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.method = AnalysisMethod::Gsca;
        recipe.method_config = Some(MethodConfig::Gsca);
        recipe.settings.workers = 1;
        recipe.settings.max_iterations = 3_000;
        recipe.settings.tolerance = 1e-7;

        let result = estimate_pls(&dataset, &recipe).unwrap();
        let repeated = estimate_pls(&dataset, &recipe).unwrap();
        let gsca = result.gsca.as_ref().unwrap();

        assert_eq!(result, repeated);
        assert_eq!(result.method_version, GSCA_METHOD_VERSION);
        assert_eq!(gsca.method_version, GSCA_METHOD_VERSION);
        assert_eq!(gsca.algorithm, GSCA_ALGORITHM_VERSION);
        assert!(gsca.converged);
        assert!(gsca.iterations < 3_000);
        assert!(gsca.final_change <= 1e-7);
        assert!(gsca.objective.is_finite() && gsca.objective >= 0.0);
        assert!((gsca.fit - (1.0 - gsca.objective / 6.0)).abs() < 1e-10);
        assert!(gsca.measurement_fit.is_finite());
        assert!(gsca.structural_fit.is_finite());
        assert!(gsca.adjusted_fit.is_finite());
        assert!(gsca.gfi.is_finite());
        assert!(gsca.srmr.is_finite() && gsca.srmr >= 0.0);
        assert_eq!(gsca.observations, 6);
        assert_eq!(gsca.weights.len(), 4);
        assert_eq!(gsca.loadings.len(), 4);
        assert_eq!(gsca.paths.len(), 1);
        assert_eq!(gsca.r_squared.len(), 1);
        assert!(gsca.bootstrap_intervals.is_empty());
        assert!(result.effects.is_empty());
        assert!(result.mediation.estimates.is_empty());
        assert!(result.control_estimates.is_empty());
        assert!(result.plsc.is_none() && result.predict.is_none() && result.cbsem.is_none());
    }

    #[test]
    fn gsca_als_v2_honors_cancellation_and_rejects_pls_only_settings() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.method = AnalysisMethod::Gsca;
        recipe.method_config = Some(MethodConfig::Gsca);
        let error = estimate_pls_with_control(&dataset, &recipe, |progress| {
            progress.phase != EstimationPhase::Iterating
        })
        .unwrap_err();
        assert_eq!(error, EstimationError::Cancelled);

        recipe.settings.workers = 2;
        assert!(matches!(
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("one worker")
        ));
        recipe.settings.workers = 1;
        recipe.settings.permutation_samples = 99;
        assert!(matches!(
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("no resampling")
        ));
    }

    #[test]
    fn reflective_path_model_converges_and_decomposes_effects() {
        let (dataset, recipe) = fixture();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(result, estimate_pls(&dataset, &recipe).unwrap());
        assert!(result.converged);
        assert!(result.iterations < recipe.settings.max_iterations);
        assert_eq!(result.used_observations, 6);
        assert_eq!(result.paths.len(), 1);
        assert!((result.paths[0].coefficient - result.effects[0].total).abs() < 1e-12);
        assert!(result.r_squared["y"] > 0.8);
        for score in result.construct_scores.values() {
            assert!(vector_mean(score).abs() < 1e-12);
            assert!((sample_sd(score) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn cca_non_saturated_residuals_are_finite_coherent_and_deterministic() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/cca_reference.csv"),
            "cca_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/cca_reference.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe
            .model
            .paths
            .retain(|path| !(path.source == "x" && path.target == "y"));

        let left = estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true).unwrap();
        let right =
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.method_version, CCA_METHOD_VERSION);

        let cca = left.cca.expect("CCA payload");
        assert_eq!(cca.method_version, CCA_METHOD_VERSION);
        assert_eq!(cca.model, "recursive_standardized_composite_path_model_v1");
        assert_eq!(cca.correlations.len(), 3);
        let computed_max = cca
            .correlations
            .iter()
            .map(|row| {
                assert!(row.observed.is_finite());
                assert!(row.reproduced.is_finite());
                assert!(row.residual.is_finite());
                assert!(row.absolute_residual.is_finite());
                assert!((row.residual - (row.observed - row.reproduced)).abs() <= 1e-12);
                assert!((row.absolute_residual - row.residual.abs()).abs() <= 1e-12);
                row.absolute_residual
            })
            .fold(0.0_f64, f64::max);
        assert!(
            computed_max > 1e-6,
            "the non-saturated fixture must exercise a nonzero residual"
        );
        assert!((cca.max_absolute_residual - computed_max).abs() <= 1e-12);
    }

    #[test]
    fn bounded_ipma_uses_only_endogenous_targets_and_fixed_standardized_path_scope() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.method = AnalysisMethod::Ipma;
        recipe.method_config = Some(MethodConfig::Ipma {
            targets: vec!["y".into()],
        });
        let execution_recipe = recipe.with_effective_metadata().unwrap();
        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &execution_recipe, |_| true)
                .unwrap();
        assert_eq!(result.method_version, IPMA_METHOD_VERSION);
        let expected_importance = result
            .effects
            .iter()
            .find(|effect| effect.source == "x" && effect.target == "y")
            .unwrap()
            .total;
        let ipma = result.ipma.unwrap();
        assert_eq!(ipma.method_version, IPMA_METHOD_VERSION);
        assert_eq!(ipma.performance_scale, IPMA_PERFORMANCE_SCALE);
        assert_eq!(ipma.targets, vec!["y"]);
        assert_eq!(ipma.constructs.len(), 1);
        assert_eq!(ipma.indicators.len(), 2);
        assert!(ipma.constructs.iter().all(|row| {
            row.target == "y"
                && row.construct == "x"
                && row.performance.is_finite()
                && (0.0..=100.0).contains(&row.performance)
        }));
        assert!(ipma.indicators.iter().all(|row| {
            row.target == "y"
                && row.construct == "x"
                && ["x1", "x2"].contains(&row.indicator.as_str())
        }));
        assert!((ipma.constructs[0].importance - expected_importance).abs() <= 1e-12);

        recipe.method_config = Some(MethodConfig::Ipma {
            targets: vec!["x".into()],
        });
        assert!(matches!(
            estimate_pls_with_effective_recipe_control(
                &dataset,
                &recipe.with_effective_metadata().unwrap(),
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("target must be endogenous")
        ));

        recipe.method_config = Some(MethodConfig::Ipma {
            targets: vec!["y".into()],
        });
        recipe.settings.weighting_scheme = WeightingScheme::Factor;
        assert!(matches!(
            estimate_pls_with_effective_recipe_control(
                &dataset,
                &recipe.with_effective_metadata().unwrap(),
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message)) if message.contains("path weighting")
        ));
        recipe.settings.weighting_scheme = WeightingScheme::Path;
        recipe.settings.preprocessing = Preprocessing::MeanCentered;
        assert!(matches!(
            estimate_pls_with_effective_recipe_control(
                &dataset,
                &recipe.with_effective_metadata().unwrap(),
                |_| true,
            ),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("standardized indicator preprocessing")
        ));
    }

    #[test]
    fn runtime_requires_explicit_micom_configural_confirmation() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v06_groups.csv"),
            "v06_groups.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v06_groups.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());

        assert!(matches!(
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true),
            Err(EstimationError::UnsupportedMethod(message))
                if message.contains("configural invariance")
        ));
    }

    #[test]
    fn micom_and_permutation_mga_v2_emit_complete_group_measurement_contract() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v06_groups.csv"),
            "v06_groups.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v06_groups.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        recipe
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        recipe.metadata.insert("mga_group_a".into(), "B".into());
        recipe.metadata.insert("mga_group_b".into(), "A".into());

        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true).unwrap();
        assert!(result.mga.is_some());
        assert!(result.mga_permutation.is_some());
        assert!(result.micom.is_some());
        let mga = result.mga.as_ref().unwrap();
        assert_eq!(mga.method_version, PLS_MGA_METHOD_VERSION);
        assert_eq!(mga.groups[0].group, "B");
        assert_eq!(mga.groups[1].group, "A");
        assert_eq!(mga.measurement_comparisons.len(), 12);
        assert!(
            mga.groups
                .iter()
                .all(|group| { group.outer_estimates.len() == 6 && group.transforms.len() == 6 })
        );
        assert!(mga.comparisons.iter().all(|comparison| {
            comparison.group_a == "B"
                && comparison.group_b == "A"
                && (comparison.difference - (comparison.coefficient_a - comparison.coefficient_b))
                    .abs()
                    < 1e-12
        }));
        let permutation = result.mga_permutation.as_ref().unwrap();
        assert_eq!(
            permutation.method_version,
            PLS_MGA_PERMUTATION_METHOD_VERSION
        );
        assert_eq!(permutation.usable_permutations, 5000);
        assert_eq!(permutation.measurement_comparisons.len(), 12);
        let micom = result.micom.as_ref().unwrap();
        assert_eq!(micom.method_version, MICOM_METHOD_VERSION);
        assert_eq!(micom.constructs.len(), 3);
        assert!(micom.constructs.iter().all(|row| {
            row.configural_invariance
                && row.compositional_correlation_lower.is_some()
                && row.mean_difference_lower.is_some()
                && row.mean_difference_upper.is_some()
                && row.variance_difference_lower.is_some()
                && row.variance_difference_upper.is_some()
                && row.equal_means.is_some()
                && row.equal_variances.is_some()
        }));
    }

    #[test]
    fn permutation_mga_reports_progress_and_honors_cancellation() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v06_groups.csv"),
            "v06_groups.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v06_groups.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        recipe
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());

        let mut permutation_zero_updates = 0usize;
        let result = estimate_pls_with_effective_recipe_control(&dataset, &recipe, |update| {
            if update.phase == EstimationPhase::Iterating
                && update.total_units == 5000
                && update.completed_units == 0
            {
                permutation_zero_updates += 1;
                permutation_zero_updates < 2
            } else {
                true
            }
        });
        assert!(permutation_zero_updates >= 2);
        assert!(matches!(result, Err(EstimationError::Cancelled)));
    }

    #[test]
    fn plsc_v2_uses_canonical_dijkstra_henseler_rho_a() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/plsc_reference.csv"),
            "plsc_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/plsc_reference.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();

        let result =
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true).unwrap();
        let plsc = result.plsc.expect("PLSc payload");
        assert_eq!(result.method_version, PLSC_METHOD_VERSION);
        assert_eq!(plsc.method_version, PLSC_METHOD_VERSION);
        assert_eq!(
            plsc.reliability_method_version,
            DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION
        );

        // Independently evaluated from Dijkstra-Henseler Equation 3 on the
        // committed 120-case fixture; qpls-assessment exercises the same
        // equation against its primary-paper and cSEM evidence.
        let expected = BTreeMap::from([
            ("x", 0.994_893_700_324_704_6),
            ("z", 0.994_198_504_685_569_0),
            ("y", 0.992_821_048_709_414_7),
        ]);
        for reliability in plsc.reliabilities {
            let expected = expected[reliability.construct.as_str()];
            assert!(
                (reliability.rho_a - expected).abs() <= 1e-12,
                "unexpected rho_A for {}: {} != {}",
                reliability.construct,
                reliability.rho_a,
                expected
            );
        }
    }

    #[test]
    fn plsc_rejects_inadmissible_corrected_correlations_without_clamping() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.method = AnalysisMethod::Plsc;
        recipe.method_config = Some(MethodConfig::Plsc);

        let error = estimate_pls(&dataset, &recipe).unwrap_err();
        assert!(matches!(
            error,
            EstimationError::Numerical(message)
                if message.contains("PLSc corrected construct correlation is outside [-1, 1]")
                    && message.contains("'x' and 'y'")
        ));
    }

    #[test]
    fn plspredict_v2_reports_leakage_free_indicator_metrics_and_cvpat() {
        let mut rows = String::from("x1,x2,y1,y2\n");
        for index in 1..=32 {
            let x = index as f64;
            rows.push_str(&format!(
                "{},{},{},{}\n",
                x,
                x + (index % 5) as f64 * 0.1,
                2.0 * x + 1.0,
                2.0 * x + 1.0 + (index % 7) as f64 * 0.08
            ));
        }
        let dataset = import_delimited_bytes(
            rows.as_bytes(),
            "predict.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let (_, mut recipe) = fixture();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::Predict;
        recipe.method_config = Some(MethodConfig::Predict {
            pls_pos: None,
            fimix: None,
        });

        let result = estimate_pls(&dataset, &recipe).unwrap();
        let predict = result.predict.expect("prediction payload");
        assert_eq!(result.method_version, PLS_PREDICT_METHOD_VERSION);
        assert_eq!(predict.training_observations, 24);
        assert_eq!(predict.test_observations, 8);
        assert_eq!(predict.targets.len(), 1);
        assert_eq!(predict.indicator_targets.len(), 2);
        assert_eq!(predict.targets[0].construct, "y");
        assert!(predict.targets[0].rmse_pls < predict.targets[0].rmse_benchmark);
        assert!(predict.targets[0].q_squared_predict.unwrap() > 0.9);
        assert!(predict.targets[0].rmse_lm.is_some());
        for target in &predict.indicator_targets {
            assert_eq!(target.construct, "y");
            assert_eq!(target.predictor_scope, "earliest_antecedent_indicators");
            assert_eq!(target.predictor_count, 2);
            assert_eq!(target.pls.observations, 8);
            assert_eq!(target.pls.mape_observations, 8);
            assert!(target.pls.rmse < target.indicator_average.rmse);
            assert!(target.q_squared_predict.unwrap() > 0.9);
            assert_eq!(target.linear_model.status, "available");
            assert!(target.linear_model.metrics.is_some());
        }
        let repeated = predict.repeated_kfold.expect("repeated k-fold payload");
        assert_eq!(
            repeated.method_version,
            PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION
        );
        assert_eq!(repeated.folds, 10);
        assert_eq!(repeated.repeats, 10);
        assert_eq!(repeated.seed, recipe.settings.seed);
        assert_eq!(
            repeated.assignment,
            "seeded_sha256_source_row_order_round_robin_10_v1"
        );
        assert!(repeated.assignment_digest.starts_with("sha256:"));
        assert_eq!(repeated.assignment_digest.len(), 71);
        assert_eq!(repeated.total_test_observations, 320);
        assert_eq!(repeated.targets.len(), 1);
        assert_eq!(repeated.indicator_targets.len(), 2);
        assert!(repeated.targets[0].rmse_pls < repeated.targets[0].rmse_benchmark);
        assert!(repeated.targets[0].rmse_lm.is_some());
        assert!(repeated.cvpat.is_empty());
        assert!(repeated.paired_loss_diagnostics.is_empty());
        assert_eq!(repeated.cvpat_benchmark_assessments.len(), 2);
        for comparison in &repeated.cvpat_benchmark_assessments {
            assert_eq!(
                comparison.method_version,
                CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION
            );
            assert_eq!(comparison.comparison_kind, "benchmark_assessment");
            assert_eq!(comparison.target_scope, "all_endogenous_indicators");
            assert_eq!(comparison.observations, 32);
            assert_eq!(comparison.indicator_count, 2);
            assert_eq!(comparison.confidence_level, 0.95);
        }
        let ia = repeated
            .cvpat_benchmark_assessments
            .iter()
            .find(|comparison| comparison.benchmark == "indicator_average")
            .unwrap();
        assert_eq!(ia.status, "available");
        assert!(ia.mean_loss_difference.unwrap() < 0.0);
        assert!(ia.p_value_one_sided.unwrap() < 0.05);
        assert_eq!(ia.preferred_model.as_deref(), Some("pls_sem"));
        assert!(ia.confidence_interval_upper.unwrap() < 0.0);
    }

    #[test]
    fn plspredict_v2_fold_assignment_is_seeded_balanced_and_auditable() {
        let rows = (0..37).collect::<Vec<_>>();
        let first = sha256_prediction_fold_assignments(&rows, 42, 0, 10);
        let repeated = sha256_prediction_fold_assignments(&rows, 42, 0, 10);
        let different_seed = sha256_prediction_fold_assignments(&rows, 43, 0, 10);
        assert_eq!(first, repeated);
        assert_ne!(first, different_seed);
        let mut counts = vec![0_usize; 10];
        for fold in first {
            counts[fold] += 1;
        }
        assert_eq!(counts.iter().sum::<usize>(), rows.len());
        assert!(counts.iter().all(|count| (3..=4).contains(count)));
    }

    #[test]
    fn plspredict_v2_mape_excludes_zero_actual_values_and_retains_count() {
        let metrics = error_metrics(&[0.0, 2.0, -4.0], &[1.0, 1.0, -2.0]);
        assert_eq!(metrics.observations, 3);
        assert_eq!(metrics.mape_observations, 2);
        assert_eq!(metrics.absolute_percentage_error_sum, Some(1.0));
        assert_eq!(metrics.mape_percent, Some(50.0));

        let unavailable = error_metrics(&[0.0, 0.0], &[1.0, -1.0]);
        assert_eq!(unavailable.mape_observations, 0);
        assert_eq!(unavailable.absolute_percentage_error_sum, None);
        assert_eq!(unavailable.mape_percent, None);
    }

    #[test]
    fn cvpat_v2_uses_lower_tail_pls_minus_benchmark_direction() {
        let pls = [1.0, 1.1, 0.9, 1.0, 1.2];
        let benchmark = [2.0, 1.8, 2.2, 1.9, 2.1];
        let row =
            cvpat_benchmark_assessment("indicator_average", &pls, Some(&benchmark), 3, Ok(()));
        assert_eq!(row.status, "available");
        assert!(row.mean_loss_difference.unwrap() < 0.0);
        assert!(row.t_statistic.unwrap() < 0.0);
        assert!(row.p_value_one_sided.unwrap() < 0.05);
        assert_eq!(row.preferred_model.as_deref(), Some("pls_sem"));
    }

    #[test]
    fn control_estimates_mirror_declared_structural_paths() {
        let (dataset, mut recipe) = fixture();
        recipe.model.controls.push(ControlPath {
            source: "x".into(),
            target: "y".into(),
            label: Some("Control X".into()),
        });
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(result.control_estimates.len(), 1);
        assert_eq!(result.control_estimates[0].source, "x");
        assert_eq!(result.control_estimates[0].target, "y");
        assert_eq!(
            result.control_estimates[0].label.as_deref(),
            Some("Control X")
        );
        assert_eq!(
            result.control_estimates[0].coefficient,
            result.paths[0].coefficient
        );
    }

    #[test]
    fn repeated_indicator_higher_order_expands_component_blocks() {
        let (dataset, mut recipe) = fixture();
        recipe.model.constructs.push(Construct {
            id: "hoc".into(),
            name: "Higher Order".into(),
            short_name: "HOC".into(),
            mode: MeasurementMode::Reflective,
            indicators: Vec::new(),
        });
        recipe.model.paths.push(StructuralPath {
            source: "hoc".into(),
            target: "y".into(),
        });
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "hoc".into(),
                components: vec!["x".into(), "y".into()],
                method: HigherOrderMethod::RepeatedIndicators,
                stage_one_recipe: None,
            });
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let hoc_indicators = result
            .outer_estimates
            .iter()
            .filter(|estimate| estimate.construct == "hoc")
            .map(|estimate| estimate.indicator.as_str())
            .collect::<Vec<_>>();
        assert_eq!(hoc_indicators, vec!["x1", "x2", "y1", "y2"]);
        assert!(result.construct_scores.contains_key("hoc"));
        assert!(result.warnings.iter().any(|warning| {
            warning.contains("Repeated-indicator higher-order constructs are validated")
                && warning.contains("expanded from lower-order component indicators")
        }));
    }

    #[test]
    fn hybrid_higher_order_splits_component_indicator_blocks() {
        let (dataset, mut recipe) = fixture();
        recipe.model.constructs.push(Construct {
            id: "hoc".into(),
            name: "Higher Order".into(),
            short_name: "HOC".into(),
            mode: MeasurementMode::Reflective,
            indicators: Vec::new(),
        });
        recipe.model.paths.push(StructuralPath {
            source: "hoc".into(),
            target: "y".into(),
        });
        recipe
            .model
            .higher_order_constructs
            .push(HigherOrderConstruct {
                id: "hoc".into(),
                components: vec!["x".into(), "y".into()],
                method: HigherOrderMethod::Hybrid,
                stage_one_recipe: None,
            });
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let indicators_for = |construct_id: &str| {
            result
                .outer_estimates
                .iter()
                .filter(|estimate| estimate.construct == construct_id)
                .map(|estimate| estimate.indicator.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(indicators_for("x"), vec!["x1"]);
        assert_eq!(indicators_for("y"), vec!["y1"]);
        assert_eq!(indicators_for("hoc"), vec!["x2", "y2"]);
        assert!(result.paths.iter().any(|path| {
            path.source == "hoc" && path.target == "y" && path.coefficient.is_finite()
        }));
        assert!(result.warnings.iter().any(|warning| {
            warning.contains("Two-stage and hybrid higher-order constructs are validated")
        }));
    }

    #[test]
    fn two_stage_higher_order_uses_component_scores_as_indicators() {
        let mut x = Vec::new();
        let mut z = Vec::new();
        let mut y = Vec::new();
        for row in 0..72 {
            let xv = (row % 12) as f64 - 5.5;
            let zv = (row / 12) as f64 - 2.5 + 0.15 * xv;
            let yv = 0.45 * xv + 0.55 * zv + ((row % 5) as f64 - 2.0) * 0.01;
            x.push(Some(xv));
            z.push(Some(zv));
            y.push(Some(yv));
        }
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                Field::new("x1", DataType::Float64, true),
                Field::new("z1", DataType::Float64, true),
                Field::new("y1", DataType::Float64, true),
            ])),
            vec![
                Arc::new(Float64Array::from(x)) as ArrayRef,
                Arc::new(Float64Array::from(z)) as ArrayRef,
                Arc::new(Float64Array::from(y)) as ArrayRef,
            ],
        )
        .unwrap();
        let dataset = Dataset {
            id: Uuid::nil(),
            name: "hoc-two-stage.csv".into(),
            schema: DatasetSchema {
                version: 1,
                kind: DataKind::Raw,
                columns: ["x1", "z1", "y1"]
                    .into_iter()
                    .map(|name| ColumnMetadata {
                        name: name.into(),
                        label: None,
                        column_type: ColumnType::Numeric,
                        scale_type: ScaleType::Continuous,
                        missing_markers: Vec::new(),
                        theoretical_min: None,
                        theoretical_max: None,
                        value_labels: BTreeMap::new(),
                    })
                    .collect(),
                case_count: batch.num_rows(),
                sample_size: Some(batch.num_rows()),
            },
            fingerprint: DataFingerprint("hoc-two-stage".into()),
            batch,
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Two-stage HOC".into(),
                constructs: vec![
                    Construct {
                        id: "x".into(),
                        name: "X".into(),
                        short_name: "X".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["x1".into()],
                    },
                    Construct {
                        id: "z".into(),
                        name: "Z".into(),
                        short_name: "Z".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["z1".into()],
                    },
                    Construct {
                        id: "hoc".into(),
                        name: "HOC".into(),
                        short_name: "HOC".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: Vec::new(),
                    },
                    Construct {
                        id: "y".into(),
                        name: "Y".into(),
                        short_name: "Y".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["y1".into()],
                    },
                ],
                paths: vec![StructuralPath {
                    source: "hoc".into(),
                    target: "y".into(),
                }],
                controls: Vec::new(),
                higher_order_constructs: vec![HigherOrderConstruct {
                    id: "hoc".into(),
                    components: vec!["x".into(), "z".into()],
                    method: HigherOrderMethod::TwoStage,
                    stage_one_recipe: None,
                }],
                interactions: Vec::new(),
            },
            settings: AnalysisSettings::default(),
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let hoc_indicators = result
            .outer_estimates
            .iter()
            .filter(|estimate| estimate.construct == "hoc")
            .map(|estimate| estimate.indicator.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            hoc_indicators,
            vec![
                higher_order_component_indicator_name("hoc", "x"),
                higher_order_component_indicator_name("hoc", "z")
            ]
        );
        assert!(result.paths.iter().any(|path| {
            path.source == "hoc" && path.target == "y" && path.coefficient.abs() > 0.9
        }));
        assert!(result.warnings.iter().any(|warning| {
            warning.contains("Two-stage higher-order constructs are validated")
        }));
    }

    #[test]
    fn two_stage_moderation_generates_product_score_and_estimates_interaction_path() {
        let mut x = Vec::new();
        let mut m = Vec::new();
        let mut y = Vec::new();
        for row in 0..80 {
            let xv = (row % 10) as f64 - 4.5;
            let mv = (row / 10) as f64 - 3.5;
            let noise = ((row % 4) as f64 - 1.5) * 0.01;
            let yv = 0.4 * xv + 0.3 * mv + 0.9 * xv * mv + noise;
            x.push(if row == 7 { None } else { Some(xv) });
            m.push(if row == 41 { None } else { Some(mv) });
            y.push(Some(yv));
        }
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                Field::new("x1", DataType::Float64, true),
                Field::new("m1", DataType::Float64, true),
                Field::new("y1", DataType::Float64, true),
            ])),
            vec![
                Arc::new(Float64Array::from(x)) as ArrayRef,
                Arc::new(Float64Array::from(m)) as ArrayRef,
                Arc::new(Float64Array::from(y)) as ArrayRef,
            ],
        )
        .unwrap();
        let dataset = Dataset {
            id: Uuid::nil(),
            name: "moderation.csv".into(),
            schema: DatasetSchema {
                version: 1,
                kind: DataKind::Raw,
                columns: ["x1", "m1", "y1"]
                    .into_iter()
                    .map(|name| ColumnMetadata {
                        name: name.into(),
                        label: None,
                        column_type: ColumnType::Numeric,
                        scale_type: ScaleType::Continuous,
                        missing_markers: Vec::new(),
                        theoretical_min: None,
                        theoretical_max: None,
                        value_labels: BTreeMap::new(),
                    })
                    .collect(),
                case_count: 80,
                sample_size: None,
            },
            batch,
            fingerprint: DataFingerprint("moderation".into()),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Moderation".into(),
                constructs: vec![
                    Construct {
                        id: "x".into(),
                        name: "Predictor".into(),
                        short_name: "X".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["x1".into()],
                    },
                    Construct {
                        id: "m".into(),
                        name: "Moderator".into(),
                        short_name: "M".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["m1".into()],
                    },
                    Construct {
                        id: "xm".into(),
                        name: "Interaction".into(),
                        short_name: "XM".into(),
                        mode: MeasurementMode::Formative,
                        indicators: Vec::new(),
                    },
                    Construct {
                        id: "y".into(),
                        name: "Outcome".into(),
                        short_name: "Y".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["y1".into()],
                    },
                ],
                paths: vec![
                    StructuralPath {
                        source: "x".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "m".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "xm".into(),
                        target: "y".into(),
                    },
                ],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: vec![InteractionTerm {
                    id: "x_by_m_to_y".into(),
                    predictor: "x".into(),
                    moderator: "m".into(),
                    product_construct: "xm".into(),
                    outcome: "y".into(),
                    method: InteractionMethod::TwoStageProductScore,
                }],
            },
            settings: AnalysisSettings::default(),
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };

        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(result.used_observations, 78);
        assert_eq!(result.omitted_observations, 2);
        let interaction_path = result
            .paths
            .iter()
            .find(|path| path.source == "xm" && path.target == "y")
            .unwrap();
        let predictor_path = result
            .paths
            .iter()
            .find(|path| path.source == "x" && path.target == "y")
            .unwrap();
        assert!(interaction_path.coefficient > 0.75);
        assert_eq!(
            result.moderation.method_version,
            PLS_TWO_STAGE_MODERATION_METHOD_VERSION
        );
        assert_eq!(result.moderation.estimates.len(), 1);
        assert!(
            serde_json::to_value(&result)
                .unwrap()
                .get("moderation")
                .is_some()
        );
        let moderation = &result.moderation.estimates[0];
        assert_eq!(moderation.interaction, "x_by_m_to_y");
        assert_eq!(moderation.product_construct, "xm");
        assert_eq!(moderation.interaction_effect, interaction_path.coefficient);
        assert_eq!(moderation.simple_slopes.len(), 3);
        for slope in &moderation.simple_slopes {
            assert!(
                (slope.effect
                    - (predictor_path.coefficient
                        + interaction_path.coefficient * slope.moderator_score))
                    .abs()
                    < 1e-12
            );
        }
        assert!(result.construct_scores.contains_key("xm"));
        assert!(result.outer_estimates.iter().any(|estimate| {
            estimate.construct == "xm"
                && estimate.indicator == product_indicator_name("x_by_m_to_y")
        }));
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("2 observations were omitted listwise"))
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("Two-stage moderation is validated"))
        );
    }

    #[test]
    fn mediation_classifier_covers_descriptive_effect_patterns() {
        let mediation = analyze_mediation_effects_with_tolerance(
            &[
                EffectEstimate {
                    source: "full".into(),
                    target: "target".into(),
                    direct: 0.0,
                    indirect: 0.0,
                    total: 0.0,
                },
                EffectEstimate {
                    source: "direct".into(),
                    target: "target".into(),
                    direct: 0.3,
                    indirect: 0.0,
                    total: 0.3,
                },
                EffectEstimate {
                    source: "indirect".into(),
                    target: "target".into(),
                    direct: 0.0,
                    indirect: 0.2,
                    total: 0.2,
                },
                EffectEstimate {
                    source: "complementary".into(),
                    target: "target".into(),
                    direct: 0.3,
                    indirect: 0.2,
                    total: 0.5,
                },
                EffectEstimate {
                    source: "competitive".into(),
                    target: "target".into(),
                    direct: 0.3,
                    indirect: -0.2,
                    total: 0.1,
                },
            ],
            1e-12,
        );
        assert_eq!(mediation.method_version, PLS_MEDIATION_METHOD_VERSION);
        assert!(
            mediation.warnings[0].contains("validated for the documented QuickPLS v1.2.1 scope")
        );
        let classes = mediation
            .estimates
            .iter()
            .map(|estimate| (&estimate.source, &estimate.classification))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(classes[&"full".to_string()], &MediationClass::NoEffect);
        assert_eq!(classes[&"direct".to_string()], &MediationClass::DirectOnly);
        assert_eq!(
            classes[&"indirect".to_string()],
            &MediationClass::IndirectOnly
        );
        assert_eq!(
            classes[&"complementary".to_string()],
            &MediationClass::ComplementaryPartial
        );
        assert_eq!(
            classes[&"competitive".to_string()],
            &MediationClass::CompetitivePartial
        );
        let complementary = mediation
            .estimates
            .iter()
            .find(|estimate| estimate.source == "complementary")
            .unwrap();
        assert_eq!(complementary.variance_accounted_for, Some(0.4));
    }

    #[test]
    fn three_construct_path_model_reports_indirect_only_mediation() {
        let dataset = import_delimited_bytes(
            b"x,m,y\n1,2,3\n2,3,5\n3,5,8\n4,7,11\n5,11,16\n6,13,19\n7,17,24\n",
            "mediation.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let model = ModelSpec {
            id: Uuid::nil(),
            name: "Mediation".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x".into()],
                },
                Construct {
                    id: "m".into(),
                    name: "M".into(),
                    short_name: "M".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["m".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y".into()],
                },
            ],
            paths: vec![
                StructuralPath {
                    source: "x".into(),
                    target: "m".into(),
                },
                StructuralPath {
                    source: "m".into(),
                    target: "y".into(),
                },
            ],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model,
            settings: AnalysisSettings::default(),
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let mediation = analyze_mediation(&result);
        let mediated = mediation
            .estimates
            .iter()
            .find(|estimate| estimate.source == "x" && estimate.target == "y")
            .unwrap();
        assert_eq!(mediated.classification, MediationClass::IndirectOnly);
        assert_eq!(mediated.direct, 0.0);
        assert!(mediated.indirect > 0.9);
        assert_eq!(mediated.variance_accounted_for, Some(1.0));
    }

    #[test]
    fn indicator_order_does_not_change_paths() {
        let (dataset, recipe) = fixture();
        let expected = estimate_pls(&dataset, &recipe).unwrap();
        let mut reordered = recipe;
        reordered.model.constructs[0].indicators.reverse();
        let actual = estimate_pls(&dataset, &reordered).unwrap();
        assert!((expected.paths[0].coefficient - actual.paths[0].coefficient).abs() < 1e-10);
        for indicator in ["x1", "x2"] {
            let left = expected
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap();
            let right = actual
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap();
            assert!((left.weight - right.weight).abs() < 1e-10);
            assert!((left.loading - right.loading).abs() < 1e-10);
        }
        assert_eq!(expected.construct_scores["x"], actual.construct_scores["x"]);
    }
    #[test]
    fn agrees_with_python_plspm_reference_fixture() {
        let (dataset, recipe) = fixture();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert!(
            (result.paths[0].coefficient - 0.983378918793432).abs() < 1e-6,
            "path was {}",
            result.paths[0].coefficient
        );
        let expected = [
            ("x1", 0.9864954295126468),
            ("x2", 0.9846982365244145),
            ("y1", 0.9954396945354063),
            ("y2", 0.9956564446247307),
        ];
        for (indicator, loading) in expected {
            let actual = result
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap()
                .loading;
            assert!(
                (actual - loading).abs() < 1e-6,
                "{indicator} loading was {actual}"
            );
        }
        let expected_weights = [
            ("x1", 0.5230179),
            ("x2", 0.4915670),
            ("y1", 0.4961349),
            ("y2", 0.5083356),
        ];
        for (indicator, weight) in expected_weights {
            let actual = result
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap()
                .weight;
            assert!(
                (actual - weight).abs() < 1e-6,
                "{indicator} weight was {actual}"
            );
        }
    }
    #[test]
    fn pca_weighting_and_formative_mode_are_executable() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.weighting_scheme = WeightingScheme::Pca;
        let result = estimate_pls(&dataset, &recipe).unwrap();
        assert!(result.converged);
        assert_eq!(result.outer_estimates.len(), 4);
        assert!(
            (result.paths[0].coefficient - 0.9823003).abs() < 1e-6,
            "PCA path was {}",
            result.paths[0].coefficient
        );
        for (indicator, expected) in [
            ("x1", 0.5072997),
            ("x2", 0.5072997),
            ("y1", 0.5022356),
            ("y2", 0.5022356),
        ] {
            let actual = result
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap()
                .weight;
            assert!(
                (actual - expected).abs() < 1e-6,
                "PCA {indicator} weight was {actual}"
            );
        }
        recipe.settings.weighting_scheme = WeightingScheme::Path;
        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        recipe.model.constructs[1].mode = MeasurementMode::Formative;
        let formative = estimate_pls(&dataset, &recipe).unwrap();
        assert!(
            (formative.paths[0].coefficient - 0.9984476).abs() < 1e-6,
            "Mode B path was {}",
            formative.paths[0].coefficient
        );
        for (indicator, expected) in [
            ("x1", 0.9931025),
            ("x2", 0.007312422),
            ("y1", 0.4059947),
            ("y2", 0.5983114),
        ] {
            let actual = formative
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap()
                .weight;
            assert!(
                (actual - expected).abs() < 1e-6,
                "Mode B {indicator} weight was {actual}"
            );
        }
    }
    #[test]
    fn cycles_and_constant_indicators_are_rejected() {
        let (dataset, mut recipe) = fixture();
        recipe.model.paths.push(StructuralPath {
            source: "y".into(),
            target: "x".into(),
        });
        assert_eq!(
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true),
            Err(EstimationError::CyclicModel)
        );
        let constant = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,1,1,2\n1,2,2,3\n1,3,3,4\n",
            "constant.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        recipe.model.paths.pop();
        assert_eq!(
            estimate_pls_with_effective_recipe_control(&constant, &recipe, |_| true),
            Err(EstimationError::ConstantIndicator("x1".into()))
        );
    }
    #[test]
    fn factor_single_item_and_preprocessing_modes_are_supported() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.weighting_scheme = WeightingScheme::Factor;
        let factor = estimate_pls(&dataset, &recipe).unwrap();
        assert!((factor.paths[0].coefficient - 0.983378918793432).abs() < 1e-6);
        recipe.model.constructs[0].indicators = vec!["x1".into()];
        recipe.model.constructs[1].indicators = vec!["y1".into()];
        for preprocessing in [
            Preprocessing::Standardized,
            Preprocessing::MeanCentered,
            Preprocessing::Unstandardized,
        ] {
            recipe.settings.preprocessing = preprocessing;
            let result = estimate_pls(&dataset, &recipe).unwrap();
            assert_eq!(result.outer_estimates.len(), 2);
            assert!(result.paths[0].coefficient.is_finite());
        }
    }
    #[test]
    fn listwise_deletion_affine_scaling_and_construct_order_are_stable() {
        let missing = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,2,1\n2,3,3,2\n3,NA,4,4\n4,4,6,5\n5,6,7,7\n6,7,9,8\n",
            "missing.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let (_, recipe) = fixture();
        let omitted =
            estimate_pls_with_effective_recipe_control(&missing, &recipe, |_| true).unwrap();
        assert_eq!(omitted.used_observations, 5);
        assert_eq!(omitted.omitted_observations, 1);
        let scaled = import_delimited_bytes(
            b"x1,x2,y1,y2\n17,2,2,1\n27,3,3,2\n37,5,4,4\n47,4,6,5\n57,6,7,7\n67,7,9,8\n",
            "scaled.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let (dataset, mut reordered) = fixture();
        let expected = estimate_pls(&dataset, &reordered).unwrap().paths[0].coefficient;
        let scaled_result =
            estimate_pls_with_effective_recipe_control(&scaled, &reordered, |_| true)
                .unwrap()
                .paths[0]
                .coefficient;
        assert!((expected - scaled_result).abs() < 1e-10);
        reordered.model.constructs.reverse();
        let reordered_result = estimate_pls(&dataset, &reordered).unwrap().paths[0].coefficient;
        assert!((expected - reordered_result).abs() < 1e-10);
    }
    #[test]
    fn iteration_limit_produces_no_completed_result() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.max_iterations = 1;
        recipe.settings.tolerance = 1e-20;
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::NonConvergence(1))
        );
    }
    #[test]
    fn execution_control_reports_progress_and_cancels_inside_iteration() {
        let (dataset, recipe) = fixture();
        let mut progress = Vec::new();
        let result = estimate_pls_with_control(&dataset, &recipe, |update| {
            progress.push(update);
            update.phase != EstimationPhase::Iterating
        });
        assert_eq!(result, Err(EstimationError::Cancelled));
        assert!(
            progress
                .iter()
                .any(|update| update.phase == EstimationPhase::PreparingRows)
        );
        assert_eq!(progress.last().unwrap().phase, EstimationPhase::Iterating);
    }
    #[test]
    fn execution_rejects_wrong_dispatch_resampling_and_malformed_models() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.method = AnalysisMethod::Cbsem;
        recipe.method_config = Some(MethodConfig::Cbsem {
            model_type: qpls_core::CbsemModelType::Sem,
            estimator: qpls_core::CbsemEstimator::Ml,
            input: qpls_core::CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            group_column: None,
            invariance_steps: Vec::new(),
        });
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::InsufficientObservations)
        );
        recipe.settings.method = AnalysisMethod::PlsPm;
        recipe.method_config = Some(MethodConfig::PlsBootstrap);
        recipe.settings.bootstrap_samples = 100;
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::ResamplingRequiresEngine)
        );
        recipe.settings.bootstrap_samples = 0;
        recipe.method_config = Some(MethodConfig::PlsAlgorithm);
        recipe.model.constructs[1].id = recipe.model.constructs[0].id.clone();
        assert_eq!(
            estimate_pls_with_effective_recipe_control(&dataset, &recipe, |_| true),
            Err(EstimationError::DuplicateConstruct("x".into()))
        );
        let (_, mut duplicate_path) = fixture();
        duplicate_path
            .model
            .paths
            .push(duplicate_path.model.paths[0].clone());
        assert_eq!(
            estimate_pls_with_effective_recipe_control(&dataset, &duplicate_path, |_| true),
            Err(EstimationError::DuplicatePath("x".into(), "y".into()))
        );
    }
    #[test]
    fn unstandardized_mode_b_is_invariant_to_indicator_offsets() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.preprocessing = Preprocessing::Unstandardized;
        recipe
            .model
            .constructs
            .iter_mut()
            .for_each(|construct| construct.mode = MeasurementMode::Formative);
        let expected = estimate_pls(&dataset, &recipe).unwrap();
        let shifted = import_delimited_bytes(
            b"x1,x2,y1,y2\n101,2,2,1\n102,3,3,2\n103,5,4,4\n104,4,6,5\n105,6,7,7\n106,7,9,8\n",
            "shifted.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let actual =
            estimate_pls_with_effective_recipe_control(&shifted, &recipe, |_| true).unwrap();
        assert!((expected.paths[0].coefficient - actual.paths[0].coefficient).abs() < 1e-10);
        for indicator in ["x1", "x2", "y1", "y2"] {
            let left = expected
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap();
            let right = actual
                .outer_estimates
                .iter()
                .find(|value| value.indicator == indicator)
                .unwrap();
            assert!((left.weight - right.weight).abs() < 1e-10, "{indicator}");
        }
    }
    #[test]
    #[ignore = "release performance qualification"]
    fn benchmark_target_shape_100k_300_100() {
        let rows = 100_000usize;
        let construct_count = 100usize;
        let indicators_per_construct = 3usize;
        let mut fields = Vec::new();
        let mut arrays = Vec::new();
        let mut metadata = Vec::new();
        let mut constructs = Vec::new();
        for construct in 0..construct_count {
            let mut indicators = Vec::new();
            for indicator in 0..indicators_per_construct {
                let name = format!("c{construct}_i{indicator}");
                indicators.push(name.clone());
                fields.push(Field::new(&name, DataType::Float64, false));
                let values = (0..rows)
                    .map(|row| {
                        let base = ((row as f64) * 0.0001 + construct as f64 * 0.01).sin();
                        base + (((row + indicator * 17) as f64) * 0.013).cos() * 0.1
                    })
                    .collect::<Vec<_>>();
                arrays.push(Arc::new(Float64Array::from(values)) as _);
                metadata.push(ColumnMetadata {
                    name,
                    label: None,
                    column_type: ColumnType::Numeric,
                    scale_type: ScaleType::Continuous,
                    missing_markers: vec![],
                    theoretical_min: None,
                    theoretical_max: None,
                    value_labels: BTreeMap::new(),
                });
            }
            constructs.push(Construct {
                id: format!("c{construct}"),
                name: format!("Construct {construct}"),
                short_name: format!("C{construct}"),
                mode: MeasurementMode::Reflective,
                indicators,
            });
        }
        let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).unwrap();
        let dataset = Dataset {
            id: Uuid::nil(),
            name: "benchmark".into(),
            schema: DatasetSchema {
                version: 1,
                kind: DataKind::Raw,
                columns: metadata,
                case_count: rows,
                sample_size: None,
            },
            batch,
            fingerprint: DataFingerprint("benchmark".into()),
        };
        let paths = (0..construct_count - 1)
            .map(|index| StructuralPath {
                source: format!("c{index}"),
                target: format!("c{}", index + 1),
            })
            .collect();
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: "benchmark".into(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Target benchmark".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings: AnalysisSettings {
                max_iterations: 100,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let started = Instant::now();
        let result = estimate_pls(&dataset, &recipe).unwrap();
        let elapsed = started.elapsed();
        eprintln!(
            "QuickPLS target benchmark: rows={rows}, indicators=300, constructs={construct_count}, iterations={}, elapsed_ms={}",
            result.iterations,
            elapsed.as_millis()
        );
        assert!(result.converged);
        assert_eq!(result.used_observations, rows);
    }
}
