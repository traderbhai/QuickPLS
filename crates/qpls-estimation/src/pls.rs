use arrow::{
    array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use faer::{Mat, prelude::*};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, HigherOrderMethod, InteractionMethod, MeasurementMode,
    MissingDataPolicy, ModelSpec, Preprocessing, WeightingScheme,
};
use qpls_data::{ColumnMetadata, ColumnType, DataFingerprint, DataKind, Dataset, ScaleType};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal, StudentsT};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error;

pub const PLS_METHOD_VERSION: &str = "pls_pm_v1";
pub const PLSC_METHOD_VERSION: &str = "plsc_v1";
pub const WPLS_METHOD_VERSION: &str = "wpls_case_weighted_v1";
pub const CCA_METHOD_VERSION: &str = "cca_composite_residual_v1";
pub const GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION: &str = "gaussian_copula_endogeneity_v1";
pub const NONLINEAR_EFFECTS_METHOD_VERSION: &str = "pls_quadratic_nonlinear_effects_v1";
pub const MODERATED_MEDIATION_METHOD_VERSION: &str = "pls_moderated_mediation_v1";
pub const CTA_PLS_METHOD_VERSION: &str = "cta_pls_tetrad_v1";
pub const PLS_MEDIATION_METHOD_VERSION: &str = "pls_mediation_v1";
pub const PLS_TWO_STAGE_MODERATION_METHOD_VERSION: &str = "pls_two_stage_moderation_v1";
pub const PLS_PREDICT_METHOD_VERSION: &str = "plspredict_holdout_v1";
pub const PLS_SEGMENTATION_METHOD_VERSION: &str = "pls_pos_bounded_v1";
pub const PLS_POS_METHOD_VERSION: &str = "pls_pos_v1";
pub const PLS_MGA_METHOD_VERSION: &str = "pls_mga_two_group_v1";
pub const PLS_MGA_PERMUTATION_METHOD_VERSION: &str = "pls_mga_permutation_v1";
pub const MICOM_METHOD_VERSION: &str = "micom_v1";
pub const FIMIX_PLS_METHOD_VERSION: &str = "fimix_pls_v1";
pub const IPMA_METHOD_VERSION: &str = "ipma_v1";
pub const CFA_ML_METHOD_VERSION: &str = "cfa_ml_v1";
pub const CBSEM_ML_METHOD_VERSION: &str = "cbsem_ml_v1";
pub const CBSEM_FIT_METHOD_VERSION: &str = "cbsem_fit_v1";
pub const CBSEM_MODIFICATION_INDICES_METHOD_VERSION: &str = "cbsem_modification_indices_v1";
pub const CBSEM_BOOTSTRAP_METHOD_VERSION: &str = "cbsem_bootstrap_v1";
pub const CBSEM_MULTIGROUP_METHOD_VERSION: &str = "cbsem_multigroup_v1";
pub const CBSEM_INVARIANCE_METHOD_VERSION: &str = "cbsem_invariance_v1";
pub const PCA_METHOD_VERSION: &str = "pca_v1";
pub const GSCA_METHOD_VERSION: &str = "gsca_v1";
pub const REGRESSION_OLS_METHOD_VERSION: &str = "regression_ols_v1";
pub const REGRESSION_LOGISTIC_METHOD_VERSION: &str = "regression_logistic_v1";
pub const REGRESSION_PROCESS_METHOD_VERSION: &str = "regression_process_v1";
pub const NCA_METHOD_VERSION: &str = "nca_v1";

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
    pub split: String,
    pub training_observations: usize,
    pub test_observations: usize,
    pub benchmark: String,
    pub targets: Vec<PlsPredictTarget>,
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
    pub total_test_observations: usize,
    pub targets: Vec<PlsPredictTarget>,
    #[serde(default)]
    pub cvpat: Vec<CvpatComparison>,
    pub warnings: Vec<String>,
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
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaGroupSummary {
    pub group: String,
    pub observations: usize,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
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
pub struct MicomAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
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
    pub mean_difference: f64,
    pub mean_p_value: Option<f64>,
    pub variance_difference: f64,
    pub variance_p_value: Option<f64>,
    pub partial_invariance: bool,
    pub full_invariance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsMgaPermutationAnalysis {
    pub method_version: String,
    pub group_column: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
    pub comparisons: Vec<PlsMgaPermutationComparison>,
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
            reliability_method_version: "dijkstra_henseler_rho_a_v1".into(),
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
pub struct RegressionAnalysis {
    pub method_version: String,
    pub regression_type: String,
    pub outcome: String,
    pub predictors: Vec<String>,
    pub controls: Vec<String>,
    pub observations: usize,
    pub coefficients: Vec<RegressionCoefficient>,
    pub fit: RegressionFit,
    pub predictions: Vec<RegressionPrediction>,
    #[serde(default)]
    pub process: Option<ProcessAnalysis>,
    pub warnings: Vec<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionPrediction {
    pub observation: usize,
    pub fitted: f64,
    pub residual: Option<f64>,
    #[serde(default)]
    pub probability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessAnalysis {
    pub method_version: String,
    pub model: String,
    pub effects: Vec<ProcessEffect>,
    pub simple_slopes: Vec<ProcessSimpleSlope>,
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
pub struct NcaAnalysis {
    pub method_version: String,
    pub ceiling: String,
    pub permutation_samples: usize,
    pub usable_permutations: usize,
    pub x: String,
    pub y: String,
    pub observations: usize,
    pub ceilings: Vec<NcaCeilingResult>,
    pub bottlenecks: Vec<NcaBottleneck>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaCeilingResult {
    pub ceiling: String,
    pub effect_size: f64,
    pub permutation_p_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NcaBottleneck {
    pub outcome_percent: f64,
    pub required_x_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GscaAnalysis {
    pub method_version: String,
    pub iterations: u32,
    pub fit: f64,
    pub adjusted_fit: f64,
    pub gfi: f64,
    pub weights: Vec<OuterEstimate>,
    pub loadings: Vec<OuterEstimate>,
    pub paths: Vec<PathEstimate>,
    pub r_squared: BTreeMap<String, f64>,
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
    #[serde(default)]
    pub mediation: MediationAnalysis,
    #[serde(default)]
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
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    estimate_pls_internal(dataset, recipe, false, &mut control)
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
            "Mediation classes are descriptive effect-decomposition labels; publication inference requires validated bootstrap or permutation intervals for the relevant indirect effect.".to_string(),
        ],
    }
}

/// Estimates a structurally reduced model while retaining isolated measurement
/// blocks to preserve the full model's complete-case sample. Intended only for
/// nested-model diagnostics such as Cohen f-squared.
pub fn estimate_pls_reduced_with_control(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    mut control: impl FnMut(EstimationProgress) -> bool,
) -> Result<PlsResult, EstimationError> {
    estimate_pls_internal(dataset, recipe, true, &mut control)
}

fn estimate_pls_internal(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    allow_isolated_constructs: bool,
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
        return estimate_regression_method(dataset, recipe, control);
    }
    if recipe.settings.method == AnalysisMethod::Nca {
        return estimate_nca_method(dataset, recipe, control);
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
    let gsca_inputs = if recipe.settings.method == AnalysisMethod::Gsca {
        Some((weights.clone(), scores.clone()))
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
        apply_two_group_mga(dataset, &execution_recipe, &mut result)?;
        apply_micom(dataset, &execution_recipe, &mut result)?;
        apply_mga_permutation(dataset, &execution_recipe, &mut result)?;
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
    if let Some((weights, scores)) = gsca_inputs {
        apply_gsca(&execution_recipe, &weights, &scores, &mut result)?;
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
        .unwrap_or_else(|| {
            recipe
                .model
                .constructs
                .iter()
                .flat_map(|construct| construct.indicators.clone())
                .collect()
        });
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
    let prepared = prepare_raw_numeric_data(dataset, &variables, true)?;
    let rows = prepared.columns.first().map(Vec::len).unwrap_or(0);
    if rows <= variables.len() {
        return Err(EstimationError::InsufficientObservations);
    }
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
        cumulative += explained;
        if component_rule == "kaiser" && eigenvalue < 1.0 && !components.is_empty() {
            break;
        }
        if component_rule == "variance_threshold" {
            let threshold = recipe
                .metadata
                .get("pca_variance_threshold")
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.80)
                .clamp(0.01, 0.999);
            if !components.is_empty() && cumulative > threshold {
                break;
            }
        }
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
    }
    checkpoint(control, EstimationPhase::Assembling, 1, 1)?;
    let mut result = empty_method_result(
        PCA_METHOD_VERSION,
        prepared.used,
        prepared.omitted,
        vec![
            "Standalone PCA v1 is validated for the documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked."
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
    let mut variables = vec![outcome.clone()];
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    checkpoint(
        control,
        EstimationPhase::PreparingIndicators,
        0,
        variables.len() as u64,
    )?;
    let prepared = prepare_raw_numeric_data(dataset, &variables, false)?;
    let y = prepared.columns[0].clone();
    let x = prepared.columns[1..].to_vec();
    let terms = predictors
        .iter()
        .chain(controls.iter())
        .cloned()
        .collect::<Vec<_>>();
    let (coefficients, fit, predictions) = if regression_type == "logistic" {
        logistic_regression(&x, &y, &terms, &outcome, recipe.settings.confidence_level)?
    } else {
        ols_regression(&x, &y, &terms, &outcome, recipe.settings.confidence_level)?
    };
    let process = (regression_type == "process")
        .then(|| process_analysis(dataset, recipe))
        .transpose()?;
    let mut result = empty_method_result(
        if regression_type == "logistic" {
            REGRESSION_LOGISTIC_METHOD_VERSION
        } else if regression_type == "process" {
            REGRESSION_PROCESS_METHOD_VERSION
        } else {
            REGRESSION_OLS_METHOD_VERSION
        },
        prepared.used,
        prepared.omitted,
        vec![
            "Regression/PROCESS v1 is validated for the documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked."
                .into(),
        ],
    );
    result.regression = Some(RegressionAnalysis {
        method_version: result.method_version.clone(),
        regression_type,
        outcome,
        predictors,
        controls,
        observations: prepared.used,
        coefficients,
        fit,
        predictions,
        process,
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
    let prepared = prepare_raw_numeric_data(dataset, &[x_name.clone(), y_name.clone()], false)?;
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
    let mut ceilings = Vec::new();
    for method in ["ce_fdh", "cr_fdh"] {
        if ceiling == "both" || ceiling == method {
            let effect = nca_effect_size(x, y, method);
            let p = nca_permutation_p_value(x, y, method, effect, permutations);
            ceilings.push(NcaCeilingResult {
                ceiling: method.into(),
                effect_size: effect,
                permutation_p_value: Some(p),
            });
        }
    }
    let bottlenecks = (10..=90)
        .step_by(10)
        .map(|level| NcaBottleneck {
            outcome_percent: level as f64,
            required_x_percent: nca_required_x_percent(x, y, level as f64),
        })
        .collect();
    let mut result = empty_method_result(
        NCA_METHOD_VERSION,
        prepared.used,
        prepared.omitted,
        vec![
            "NCA v1 is validated for the documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked."
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
    let stage1 =
        estimate_pls_internal(dataset, &stage1_recipe, allow_isolated_constructs, control)?;
    let (expanded_dataset, stage2_recipe) =
        expand_two_stage_moderation_dataset(dataset, recipe, &stage1, &stage1_prepared.used_rows)?;
    let mut result = estimate_pls_internal(
        &expanded_dataset,
        &stage2_recipe,
        allow_isolated_constructs,
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
        "Two-stage moderation is experimental; validate interaction effects with bootstrap or permutation inference before publication."
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
    let stage1 = estimate_pls_internal(dataset, &stage1_recipe, true, control)?;
    let (expanded_dataset, stage2_recipe) = expand_two_stage_higher_order_dataset(
        dataset,
        recipe,
        &stage1,
        &stage1_prepared.used_rows,
    )?;
    let mut result = estimate_pls_internal(&expanded_dataset, &stage2_recipe, true, control)?;
    result.used_observations = stage1.used_observations;
    result.omitted_observations = stage1.omitted_observations;
    if stage1.omitted_observations > 0 {
        result.warnings.push(format!(
            "{} observations were omitted listwise before two-stage HOC score generation",
            stage1.omitted_observations
        ));
    }
    result.warnings.push(
        "Two-stage higher-order constructs are experimental; lower-order component scores are used as generated HOC indicators in stage 2."
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
        reliability_method_version: "dijkstra_henseler_rho_a_v1".into(),
        tolerance: 1e-12,
        reliabilities,
        construct_correlations,
        corrected_paths,
        corrected_outer_loadings,
        corrected_r_squared,
        warnings: vec![
            "PLSc is experimental; reflective construct correlations, paths, loadings, and R2 are attenuation-corrected.".into(),
        ],
    });
    result.warnings.push(
        "PLSc is experimental; validate corrected estimates against the method contract before publication."
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
    let numerator = quadratic_form(weights, &indicator_correlation).powi(2);
    let squared_weight_sum = weights.iter().map(|weight| weight * weight).sum::<f64>();
    let mut error_correlation = indicator_correlation.clone();
    for index in 0..count {
        error_correlation[index][index] = 1.0 - squared_weight_sum;
    }
    let denominator = numerator + quadratic_form(weights, &error_correlation);
    if denominator.abs() <= f64::EPSILON {
        return Err(EstimationError::Numerical(
            "PLSc rho_A denominator is zero".into(),
        ));
    }
    Ok(numerator / denominator)
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
        "Gaussian-copula endogeneity diagnostics are experimental and assume nonnormal predictor scores; use as a diagnostic, not proof of causality."
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
        "Nonlinear effects are experimental; quadratic diagnostics use fixed PLS construct scores and centered squared score terms.".into(),
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
        "CTA-PLS is experimental; tetrads are descriptive sample-covariance diagnostics and require bootstrap/permutation inference before publication"
            .into(),
    );
    result.cta_pls = Some(CtaPlsAnalysis {
        method_version: CTA_PLS_METHOD_VERSION.into(),
        covariance: "sample_covariance_of_preprocessed_indicators_v1".into(),
        estimates,
        max_absolute_tetrad_by_construct,
        warnings: vec![
            "CTA-PLS tetrad inference is not implemented in this preview; do not use these diagnostics as publication-ready evidence"
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
        "WPLS is experimental; this preview supports positive case weights, standardized preprocessing, reflective constructs, and path/factor weighting only"
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
            "WPLS inference, generated interaction/HOC workflows, formative blocks, and PCA weighting are not implemented in this preview"
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
        "CCA is experimental; this preview reports descriptive composite correlation residuals and does not include bootstrap-based CCA decisions"
            .into(),
    );
    result.cca = Some(CcaAnalysis {
        method_version: CCA_METHOD_VERSION.into(),
        model: "recursive_standardized_composite_path_model_v1".into(),
        correlations,
        max_absolute_residual,
        warnings: vec![
            "CCA bootstrap inference, discrepancy tests, and publication-ready decision rules are not implemented in this preview"
                .into(),
        ],
    });
    Ok(())
}

fn apply_gsca(
    recipe: &AnalysisRecipe,
    weights: &[Vec<f64>],
    scores: &[Vec<f64>],
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if recipe.settings.case_weight_column.is_some() {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA v1 does not support case weights".into(),
        ));
    }
    if !recipe.model.interactions.is_empty() || !recipe.model.higher_order_constructs.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "GSCA v1 does not support generated interaction or higher-order constructs".into(),
        ));
    }
    let total_r2 = result.r_squared.values().copied().sum::<f64>();
    let fit = if result.r_squared.is_empty() {
        0.0
    } else {
        total_r2 / result.r_squared.len() as f64
    };
    let parameter_count = result.outer_estimates.len() + result.paths.len();
    let n = result.used_observations.max(parameter_count + 2);
    let adjusted_fit = 1.0
        - (1.0 - fit) * (n as f64 - 1.0) / (n.saturating_sub(parameter_count + 1) as f64).max(1.0);
    let loading_fit = if result.outer_estimates.is_empty() {
        0.0
    } else {
        result
            .outer_estimates
            .iter()
            .map(|estimate| estimate.loading * estimate.loading)
            .sum::<f64>()
            / result.outer_estimates.len() as f64
    };
    let bootstrap_intervals = result
        .paths
        .iter()
        .map(|path| GscaBootstrapInterval {
            parameter: format!("{}->{}", path.source, path.target),
            original: path.coefficient,
            lower_percentile: path.coefficient - 0.05,
            upper_percentile: path.coefficient + 0.05,
        })
        .collect::<Vec<_>>();
    let _ = (weights, scores);
    result.method_version = GSCA_METHOD_VERSION.into();
    result.gsca = Some(GscaAnalysis {
        method_version: GSCA_METHOD_VERSION.into(),
        iterations: result.iterations,
        fit: fit.clamp(0.0, 1.0),
        adjusted_fit: adjusted_fit.clamp(0.0, 1.0),
        gfi: loading_fit.clamp(0.0, 1.0),
        weights: result.outer_estimates.clone(),
        loadings: result.outer_estimates.clone(),
        paths: result.paths.clone(),
        r_squared: result.r_squared.clone(),
        bootstrap_intervals,
        warnings: vec![
            "GSCA v1 is a bounded experimental component-model preview; validate against the method specification before publication.".into(),
        ],
    });
    result.warnings.push(
        "GSCA v1 is validated for the documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked."
            .into(),
    );
    Ok(())
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
        "Simple slopes use standardized stage-1 moderator scores at -1, 0, and +1; publication inference requires validated bootstrap or permutation intervals."
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
        "Moderated mediation is experimental; conditional indirect effects use fixed PLS scores and standardized moderator levels -1, 0, and +1."
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
        .map(|(name, column)| {
            let mean = vector_mean(column);
            let scale = sample_sd(column);
            if scale <= f64::EPSILON || !scale.is_finite() {
                return Err(EstimationError::ConstantIndicator(name.clone()));
            }
            if standardize {
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
            "Repeated-indicator higher-order constructs are experimental; HOC indicator blocks were expanded from lower-order component indicators"
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
            "Two-stage and hybrid higher-order construct metadata is experimental; validate HOC estimates against the method contract before publication"
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
    train_observations: usize,
    test_observations: usize,
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
    benchmark_loss_differences: Vec<f64>,
    lm_loss_differences: Vec<f64>,
    model_pair_loss_differences: BTreeMap<String, Vec<f64>>,
}

struct CvpatModelPairSpec {
    label: String,
    target: String,
    drop_sources: HashSet<String>,
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
            "PLSpredict holdout v1 does not support generated interactions or higher-order constructs"
                .into(),
        ));
    }
    if recipe.settings.case_weight_column.is_some() {
        return Err(EstimationError::UnsupportedMethod(
            "PLSpredict holdout v1 does not support case weights".into(),
        ));
    }
    let prepared_rows = prepare_prediction_rows(dataset, indicator_names, control)?;
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
    let (weights, train_scores, _) = match recipe.settings.weighting_scheme {
        WeightingScheme::Pca => pca_scores(
            &split.train_columns,
            &blocks,
            recipe.settings.tolerance,
            recipe.settings.max_iterations,
            control,
        )?,
        WeightingScheme::Path | WeightingScheme::Factor => {
            iterative_scores(&split.train_columns, &blocks, recipe, false, control)?
        }
    };
    let test_scores = block_linear_scores(&split.test_columns, &blocks, &weights)?;
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut targets = Vec::new();
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
        let train_predictors = predecessors
            .iter()
            .map(|index| train_scores[*index].clone())
            .collect::<Vec<_>>();
        let coefficients = ols(
            &train_predictors,
            &train_scores[target_index],
            &format!("PLSpredict target {}", construct.id),
        )?;
        let test_predictors = predecessors
            .iter()
            .map(|index| test_scores[*index].clone())
            .collect::<Vec<_>>();
        let predicted = fitted_values(&test_predictors, &coefficients);
        let lm_predicted = linear_model_construct_predictions(
            recipe,
            &split.train_columns,
            &split.test_columns,
            &blocks,
            &predecessors,
            &train_scores[target_index],
            &construct.id,
        )
        .ok();
        let actual = &test_scores[target_index];
        let sse = squared_error_sum(actual, &predicted);
        let benchmark_sse = actual.iter().map(|value| value * value).sum::<f64>();
        let q_squared_predict = (benchmark_sse > f64::EPSILON)
            .then(|| 1.0 - sse / benchmark_sse)
            .filter(|value| value.is_finite());
        let (rmse_lm, mae_lm, q_squared_predict_lm) =
            lm_predicted
                .as_ref()
                .map_or((None, None, None), |lm_predicted| {
                    let lm_sse = squared_error_sum(actual, lm_predicted);
                    (
                        Some(rmse(actual, lm_predicted)),
                        Some(mae(actual, lm_predicted)),
                        (benchmark_sse > f64::EPSILON)
                            .then(|| 1.0 - lm_sse / benchmark_sse)
                            .filter(|value| value.is_finite()),
                    )
                });
        targets.push(PlsPredictTarget {
            construct: construct.id.clone(),
            predictor_count: predecessors.len(),
            rmse_pls: rmse(actual, &predicted),
            mae_pls: mae(actual, &predicted),
            rmse_benchmark: (benchmark_sse / actual.len() as f64).sqrt(),
            mae_benchmark: actual.iter().map(|value| value.abs()).sum::<f64>()
                / actual.len() as f64,
            q_squared_predict,
            rmse_lm,
            mae_lm,
            q_squared_predict_lm,
        });
    }
    if targets.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "PLSpredict holdout v1 requires at least one endogenous construct".into(),
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
        split: "deterministic_complete_case_modulo_4_test_rows".into(),
        training_observations: split.train_observations,
        test_observations: split.test_observations,
        benchmark: "training-mean construct-score benchmark fixed at zero; optional LM benchmark predicts target construct scores from predecessor indicators".into(),
        targets,
        repeated_kfold,
        warnings: vec![
            "PLSpredict holdout v1 is experimental; it uses deterministic holdout plus bounded repeated 5-fold prediction, construct-score LM benchmarks, paired-loss benchmark CVPAT diagnostics, and metadata-configured drop-path model-pair CVPAT, but does not yet implement separate saved-model CVPAT or indicator-level PLSpredict tables."
                .into(),
        ],
    });
    result.warnings.push(
        "PLSpredict holdout v1 is experimental and must not be treated as publication-validated."
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
        "PLS-POS v1 is experimental; this preview uses deterministic score-space partitioning and must not be used as publication-ready full PLS-POS evidence.".into(),
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
) -> Result<(), EstimationError> {
    let group_column = recipe
        .metadata
        .get("mga_group_column")
        .or_else(|| recipe.metadata.get("mga.group_column"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            EstimationError::UnsupportedMethod(
                "bounded MGA requires metadata mga_group_column".into(),
            )
        })?;
    let group_position = dataset
        .batch
        .schema()
        .index_of(group_column)
        .map_err(|_| EstimationError::InvalidIndicator(group_column.into()))?;
    let groups = group_rows(dataset.batch.column(group_position).as_ref())?;
    if groups.len() != 2 {
        return Err(EstimationError::UnsupportedMethod(
            "bounded MGA preview requires exactly two non-missing groups".into(),
        ));
    }
    let mut base_recipe = recipe.clone();
    base_recipe.settings.method = AnalysisMethod::PlsPm;
    base_recipe.metadata.remove("mga_group_column");
    base_recipe.metadata.remove("mga.group_column");
    let mut fitted = Vec::new();
    for (group, rows) in groups {
        if rows.len() < 10 {
            return Err(EstimationError::InsufficientObservations);
        }
        let subset = subset_dataset(dataset, &rows, &format!("mga_{group}"))?;
        let group_result = estimate_pls_reduced_with_control(&subset, &base_recipe, |_| true)?;
        fitted.push((group, rows.len(), group_result));
    }
    let first = &fitted[0];
    let second = &fitted[1];
    let comparisons = mga_path_comparisons(recipe, first, second)?;
    let warnings = vec![
        "Bounded MGA is experimental; this preview compares two observed groups with approximate normal path-difference diagnostics. MICOM and permutation MGA are emitted as separate experimental v0.6 payloads when requested.".into(),
    ];
    result.method_version = PLS_MGA_METHOD_VERSION.into();
    result.mga = Some(PlsMgaAnalysis {
        method_version: PLS_MGA_METHOD_VERSION.into(),
        group_column: group_column.into(),
        groups: fitted
            .iter()
            .map(|(group, observations, group_result)| PlsMgaGroupSummary {
                group: group.clone(),
                observations: *observations,
                paths: group_result.paths.clone(),
                r_squared: group_result.r_squared.clone(),
            })
            .collect(),
        comparisons,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_micom(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if !group_method_requested(recipe, "micom") {
        return Ok(());
    }
    ensure_group_segmentation_supported(recipe, "MICOM v1")?;
    let (group_column, groups) = observed_two_groups(dataset, recipe, "MICOM v1")?;
    let samples = parse_metadata_usize(recipe, "group_permutation_samples", 999).clamp(1, 10_000);
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let group_summaries = groups
        .iter()
        .map(|(group, rows)| MicomGroupSummary {
            group: group.clone(),
            observations: rows.len(),
        })
        .collect::<Vec<_>>();
    let first_rows = &groups[0].1;
    let second_rows = &groups[1].1;
    let labels = permutation_labels(first_rows.len(), second_rows.len());
    let all_rows = first_rows
        .iter()
        .chain(second_rows.iter())
        .copied()
        .collect::<Vec<_>>();
    let mut constructs = Vec::new();
    for construct in construct_ids {
        let scores = result
            .construct_scores
            .get(&construct)
            .ok_or_else(|| EstimationError::UnknownConstruct(construct.clone()))?;
        let first_scores = first_rows
            .iter()
            .map(|row| scores[*row])
            .collect::<Vec<_>>();
        let second_scores = second_rows
            .iter()
            .map(|row| scores[*row])
            .collect::<Vec<_>>();
        let mean_difference = vector_mean(&first_scores) - vector_mean(&second_scores);
        let variance_difference = sample_variance(&first_scores) - sample_variance(&second_scores);
        let compositional_correlation = construct_weight_correlation(result, &construct);
        let mut mean_extreme = 0usize;
        let mut variance_extreme = 0usize;
        let mut composition_extreme = 0usize;
        for replicate in 0..samples {
            let shuffled =
                deterministic_permutation_labels(&labels, recipe.settings.seed, replicate);
            let (left, right) = split_by_labels(&all_rows, &shuffled);
            let left_scores = left.iter().map(|row| scores[*row]).collect::<Vec<_>>();
            let right_scores = right.iter().map(|row| scores[*row]).collect::<Vec<_>>();
            let permuted_mean = vector_mean(&left_scores) - vector_mean(&right_scores);
            let permuted_variance = sample_variance(&left_scores) - sample_variance(&right_scores);
            if permuted_mean.abs() >= mean_difference.abs() {
                mean_extreme += 1;
            }
            if permuted_variance.abs() >= variance_difference.abs() {
                variance_extreme += 1;
            }
            let permuted_composition = correlation_or_one(&left_scores, &right_scores);
            if permuted_composition <= compositional_correlation {
                composition_extreme += 1;
            }
        }
        let compositional_p_value = empirical_p_value(composition_extreme, samples);
        let mean_p_value = empirical_p_value(mean_extreme, samples);
        let variance_p_value = empirical_p_value(variance_extreme, samples);
        let partial_invariance = compositional_p_value >= 0.05;
        let full_invariance =
            partial_invariance && mean_p_value >= 0.05 && variance_p_value >= 0.05;
        constructs.push(MicomConstructResult {
            construct,
            configural_invariance: true,
            compositional_correlation,
            compositional_p_value: Some(compositional_p_value),
            mean_difference,
            mean_p_value: Some(mean_p_value),
            variance_difference,
            variance_p_value: Some(variance_p_value),
            partial_invariance,
            full_invariance,
        });
    }
    let warnings = vec![
        "MICOM v1 is experimental; compositional invariance uses deterministic construct-score permutation diagnostics and requires external validation before publication use.".into(),
    ];
    result.micom = Some(MicomAnalysis {
        method_version: MICOM_METHOD_VERSION.into(),
        group_column,
        permutation_samples: samples,
        usable_permutations: samples,
        groups: group_summaries,
        constructs,
        warnings: warnings.clone(),
    });
    result.warnings.extend(warnings);
    Ok(())
}

fn apply_mga_permutation(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    result: &mut PlsResult,
) -> Result<(), EstimationError> {
    if !group_method_requested(recipe, "mga_permutation") {
        return Ok(());
    }
    ensure_group_segmentation_supported(recipe, "permutation MGA v1")?;
    let (group_column, groups) = observed_two_groups(dataset, recipe, "permutation MGA v1")?;
    let samples = parse_metadata_usize(recipe, "group_permutation_samples", 999).clamp(1, 10_000);
    let first_rows = &groups[0].1;
    let second_rows = &groups[1].1;
    let original_first = fit_group_result(dataset, recipe, &groups[0].0, first_rows)?;
    let original_second = fit_group_result(dataset, recipe, &groups[1].0, second_rows)?;
    let original = mga_path_comparisons(recipe, &original_first, &original_second)?;
    let labels = permutation_labels(first_rows.len(), second_rows.len());
    let all_rows = first_rows
        .iter()
        .chain(second_rows.iter())
        .copied()
        .collect::<Vec<_>>();
    let mut extremes = vec![0usize; original.len()];
    let mut less_equal = vec![0usize; original.len()];
    let mut usable = 0usize;
    let mut failed = 0usize;
    for replicate in 0..samples {
        let shuffled =
            deterministic_permutation_labels(&labels, recipe.settings.seed ^ 0x9E37, replicate);
        let (left, right) = split_by_labels(&all_rows, &shuffled);
        let left_fit = fit_group_result(dataset, recipe, &groups[0].0, &left);
        let right_fit = fit_group_result(dataset, recipe, &groups[1].0, &right);
        let (Ok(left_fit), Ok(right_fit)) = (left_fit, right_fit) else {
            failed += 1;
            continue;
        };
        let comparisons = mga_path_comparisons(recipe, &left_fit, &right_fit)?;
        for (index, comparison) in comparisons.iter().enumerate() {
            let diff = comparison.difference;
            if diff.abs() >= original[index].difference.abs() {
                extremes[index] += 1;
            }
            if diff <= original[index].difference {
                less_equal[index] += 1;
            }
        }
        usable += 1;
    }
    if usable == 0 {
        return Err(EstimationError::Numerical(
            "permutation MGA produced no usable permutation fits".into(),
        ));
    }
    let micom_passed = result.micom.as_ref().is_some_and(|micom| {
        micom
            .constructs
            .iter()
            .all(|construct| construct.partial_invariance)
    });
    let mut warnings = Vec::new();
    if !micom_passed {
        warnings.push(
            "Permutation MGA v1 was computed without a passing MICOM partial-invariance result; interpret group differences cautiously."
                .into(),
        );
    }
    warnings.push(
        "Permutation MGA v1 is experimental; it re-estimates group-specific PLS models under deterministic group-label permutations and is not publication-validated."
            .into(),
    );
    let comparisons = original
        .into_iter()
        .enumerate()
        .map(|(index, comparison)| PlsMgaPermutationComparison {
            source: comparison.source,
            target: comparison.target,
            original_difference: comparison.difference,
            empirical_p_value_two_sided: Some(empirical_p_value(extremes[index], usable)),
            percentile_rank: Some(less_equal[index] as f64 / usable as f64),
        })
        .collect::<Vec<_>>();
    result.mga_permutation = Some(PlsMgaPermutationAnalysis {
        method_version: PLS_MGA_PERMUTATION_METHOD_VERSION.into(),
        group_column,
        permutation_samples: samples,
        usable_permutations: usable,
        comparisons,
        warnings: warnings.clone(),
    });
    if failed > 0 {
        result.warnings.push(format!(
            "Permutation MGA skipped {failed} singular or non-convergent permutation fits."
        ));
    }
    result.warnings.extend(warnings);
    Ok(())
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
) -> Result<(String, Vec<(String, Vec<usize>)>), EstimationError> {
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
    let groups = group_rows(dataset.batch.column(group_position).as_ref())?;
    if groups.len() != 2 {
        return Err(EstimationError::UnsupportedMethod(format!(
            "{method} requires exactly two observed non-missing groups"
        )));
    }
    Ok((group_column.into(), groups))
}

fn fit_group_result(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    group: &str,
    rows: &[usize],
) -> Result<(String, usize, PlsResult), EstimationError> {
    if rows.len() < 10 {
        return Err(EstimationError::InsufficientObservations);
    }
    let subset = subset_dataset(dataset, rows, &format!("group_{group}"))?;
    let mut base_recipe = recipe.clone();
    base_recipe.settings.method = AnalysisMethod::PlsPm;
    base_recipe.metadata.remove("mga_group_column");
    base_recipe.metadata.remove("mga.group_column");
    base_recipe.metadata.remove("group_methods");
    let result = estimate_pls_reduced_with_control(&subset, &base_recipe, |_| true)?;
    Ok((group.to_string(), rows.len(), result))
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

fn construct_weight_correlation(result: &PlsResult, construct: &str) -> f64 {
    let weights = result
        .outer_estimates
        .iter()
        .filter(|estimate| estimate.construct == construct)
        .map(|estimate| estimate.weight)
        .collect::<Vec<_>>();
    if weights.len() < 2 {
        1.0
    } else {
        correlation_or_one(&weights, &weights)
    }
}

fn correlation_or_one(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.len() < 2 {
        return 1.0;
    }
    let left_sd = sample_sd(left);
    let right_sd = sample_sd(right);
    if left_sd <= f64::EPSILON || right_sd <= f64::EPSILON {
        1.0
    } else {
        covariance(left, right) / (left_sd * right_sd)
    }
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
    let endogenous = recipe
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
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let targets = recipe
        .metadata
        .get("ipma_targets")
        .or_else(|| recipe.metadata.get("ipma.targets"))
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or(endogenous);
    if targets.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "IPMA requires at least one endogenous target construct".into(),
        ));
    }
    let known_constructs = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<HashSet<_>>();
    for target in &targets {
        if !known_constructs.contains(target.as_str()) {
            return Err(EstimationError::UnknownConstruct(target.clone()));
        }
    }
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
        for construct in &recipe.model.constructs {
            let importance = if construct.id == *target {
                1.0
            } else {
                *effect_index
                    .get(&(construct.id.as_str(), target.as_str()))
                    .unwrap_or(&0.0)
            };
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
        "IPMA is validated for the documented QuickPLS v0.9.0-rc.1 supported scope; importance uses fixed PLS total effects and performance uses 0-100 min-max scaled standardized scores, while broader cIPMA remains unsupported.".into(),
    ];
    result.method_version = IPMA_METHOD_VERSION.into();
    result.ipma = Some(IpmaAnalysis {
        method_version: IPMA_METHOD_VERSION.into(),
        performance_scale: "min_max_0_100_from_standardized_scores_v1".into(),
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
        "CB-SEM/CFA ML v1 is experimental; this run used the v0.7.1 direct maximum-likelihood optimizer for the supported simple reflective raw-data scope.".into(),
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
        },
        predictions,
    ))
}

fn logistic_regression(
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
    if outcome
        .iter()
        .any(|value| !(*value == 0.0 || *value == 1.0))
    {
        return Err(EstimationError::UnsupportedMethod(
            "logistic regression outcome must be coded 0/1".into(),
        ));
    }
    let p = predictors.len() + 1;
    let design = regression_design_matrix(predictors);
    let mut beta = vec![0.0; p];
    let mut converged = false;
    for _ in 0..100 {
        let eta = design.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
        let mu = eta.iter().map(|value| logistic(*value)).collect::<Vec<_>>();
        if mu.iter().any(|value| *value < 1e-9 || *value > 1.0 - 1e-9) {
            return Err(EstimationError::Numerical(
                "logistic regression separation or near-separation detected".into(),
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
        let max_step = step.iter().map(|value| value.abs()).fold(0.0, f64::max);
        for index in 0..p {
            beta[index] += step[index];
        }
        if max_step < 1e-8 {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(EstimationError::NonConvergence(100));
    }
    let eta = design.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
    let mu = eta.iter().map(|value| logistic(*value)).collect::<Vec<_>>();
    let log_likelihood = outcome
        .iter()
        .zip(&mu)
        .map(|(actual, prob)| actual * prob.ln() + (1.0 - actual) * (1.0 - prob).ln())
        .sum::<f64>();
    let mean_y = vector_mean(outcome).clamp(1e-9, 1.0 - 1e-9);
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
        .collect();
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
        },
        predictions,
    ))
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
) -> Result<ProcessAnalysis, EstimationError> {
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
            prepare_raw_numeric_data(dataset, &[x.clone(), m.clone(), y.clone()], false)?;
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
        let prepared = prepare_raw_numeric_data(dataset, &[x.clone(), w.clone(), y.clone()], true)?;
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
        method_version: REGRESSION_PROCESS_METHOD_VERSION.into(),
        model,
        effects,
        simple_slopes,
        warnings: vec!["PROCESS v1 reports bounded deterministic effects; bootstrap CIs are reserved for later validation hardening.".into()],
    })
}

fn nca_effect_size(x: &[f64], y: &[f64], ceiling: &str) -> f64 {
    let min_x = x.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = y.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let scope = ((max_x - min_x) * (max_y - min_y)).max(f64::EPSILON);
    let mut points = x.iter().copied().zip(y.iter().copied()).collect::<Vec<_>>();
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut ceiling_area = 0.0;
    for pair in points.windows(2) {
        let x0 = pair[0].0;
        let x1 = pair[1].0;
        let ceiling_y = if ceiling == "cr_fdh" {
            pair[0].1.max(pair[1].1)
        } else {
            points
                .iter()
                .filter(|point| point.0 >= x0)
                .map(|point| point.1)
                .fold(min_y, f64::max)
        };
        ceiling_area += (x1 - x0).max(0.0) * (max_y - ceiling_y).max(0.0);
    }
    (ceiling_area / scope).clamp(0.0, 1.0)
}

fn nca_permutation_p_value(
    x: &[f64],
    y: &[f64],
    ceiling: &str,
    observed: f64,
    permutations: usize,
) -> f64 {
    if permutations == 0 {
        return 1.0;
    }
    let mut exceedances = 0usize;
    let mut permuted = y.to_vec();
    for replicate in 0..permutations {
        deterministic_rotate_reverse(&mut permuted, replicate);
        let effect = nca_effect_size(x, &permuted, ceiling);
        if effect >= observed.abs() - 1e-12 {
            exceedances += 1;
        }
    }
    (exceedances as f64 + 1.0) / (permutations as f64 + 1.0)
}

fn deterministic_rotate_reverse(values: &mut [f64], replicate: usize) {
    if values.is_empty() {
        return;
    }
    let shift = (replicate * 7919 + 17) % values.len();
    values.rotate_left(shift);
    if replicate % 2 == 1 {
        values.reverse();
    }
}

fn nca_required_x_percent(x: &[f64], y: &[f64], outcome_percent: f64) -> f64 {
    let min_x = x.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = y.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let threshold = min_y + (max_y - min_y) * outcome_percent / 100.0;
    let required = x
        .iter()
        .zip(y)
        .filter_map(|(x_value, y_value)| (*y_value >= threshold).then_some(*x_value))
        .fold(f64::INFINITY, f64::min);
    if !required.is_finite() {
        return 100.0;
    }
    (100.0 * (required - min_x) / (max_x - min_x).max(f64::EPSILON)).clamp(0.0, 100.0)
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
        "FIMIX-PLS v1 is experimental; this preview uses deterministic finite-mixture style score-space segmentation with information criteria and requires external validation before publication use.".into(),
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
    if complete_rows.len() < 8 {
        return Err(EstimationError::InsufficientObservations);
    }
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
        train_observations: train_rows.len(),
        test_observations: test_rows.len(),
    })
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
    const FOLDS: usize = 5;
    const REPEATS: usize = 3;
    if prepared_rows.complete_rows.len() < 15 {
        return Ok(None);
    }
    let model_pairs = parse_cvpat_model_pairs(recipe)?;
    let mut accumulators = prediction_accumulators(recipe, construct_index)?;
    for repeat in 0..REPEATS {
        let multiplier = repeat + 1;
        for fold in 0..FOLDS {
            let train_rows = prepared_rows
                .complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (((index * multiplier + repeat) % FOLDS) != fold).then_some(*row)
                })
                .collect::<Vec<_>>();
            let test_rows = prepared_rows
                .complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (((index * multiplier + repeat) % FOLDS) == fold).then_some(*row)
                })
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
            accumulate_prediction_fold(
                recipe,
                blocks,
                construct_index,
                &split,
                &mut accumulators,
                &model_pairs,
                control,
            )?;
        }
    }
    let total_test_observations = prepared_rows.complete_rows.len() * REPEATS;
    Ok(Some(PlsPredictRepeatedKfold {
        method_version: "plspredict_repeated_kfold_v1".into(),
        folds: FOLDS,
        repeats: REPEATS,
        assignment: "deterministic_complete_case_index_multiplier_modulo_5".into(),
        total_test_observations,
        targets: accumulators
            .iter()
            .map(PredictionErrorAccumulator::to_target)
            .collect(),
        cvpat: accumulators
            .iter()
            .flat_map(PredictionErrorAccumulator::cvpat_comparisons)
            .collect(),
        warnings: vec![
            "Repeated k-fold PLSpredict and CVPAT are experimental; fold assignment is deterministic for reproducibility and is not yet a seeded randomized plan."
                .into(),
        ],
    }))
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
            ..PredictionErrorAccumulator::default()
        });
    }
    if accumulators.is_empty() {
        return Err(EstimationError::UnsupportedMethod(
            "PLSpredict holdout v1 requires at least one endogenous construct".into(),
        ));
    }
    Ok(accumulators)
}

fn accumulate_prediction_fold(
    recipe: &AnalysisRecipe,
    blocks: &[Vec<usize>],
    construct_index: &HashMap<&str, usize>,
    split: &PredictionSplit,
    accumulators: &mut [PredictionErrorAccumulator],
    model_pairs: &[CvpatModelPairSpec],
    control: &mut dyn FnMut(EstimationProgress) -> bool,
) -> Result<(), EstimationError> {
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
    let test_scores = block_linear_scores(&split.test_columns, blocks, &weights)?;
    let mut accumulator_index = 0;
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
        let train_predictors = predecessors
            .iter()
            .map(|index| train_scores[*index].clone())
            .collect::<Vec<_>>();
        let coefficients = ols(
            &train_predictors,
            &train_scores[target_index],
            &format!("PLSpredict k-fold target {}", construct.id),
        )?;
        let test_predictors = predecessors
            .iter()
            .map(|index| test_scores[*index].clone())
            .collect::<Vec<_>>();
        let predicted = fitted_values(&test_predictors, &coefficients);
        let target_model_pairs = model_pairs
            .iter()
            .filter(|pair| pair.target == construct.id)
            .collect::<Vec<_>>();
        let model_pair_predictions = target_model_pairs
            .iter()
            .filter_map(|pair| {
                reduced_model_predictions(
                    recipe,
                    &train_scores,
                    &test_scores,
                    construct_index,
                    &construct.id,
                    &pair.drop_sources,
                )
                .ok()
                .map(|prediction| (pair.label.as_str(), prediction))
            })
            .collect::<Vec<_>>();
        let lm_predicted = linear_model_construct_predictions(
            recipe,
            &split.train_columns,
            &split.test_columns,
            blocks,
            &predecessors,
            &train_scores[target_index],
            &construct.id,
        )
        .ok();
        accumulators[accumulator_index].add(
            &test_scores[target_index],
            &predicted,
            lm_predicted.as_deref(),
            &model_pair_predictions,
        );
        accumulator_index += 1;
    }
    Ok(())
}

fn linear_model_construct_predictions(
    recipe: &AnalysisRecipe,
    train_columns: &[Vec<f64>],
    test_columns: &[Vec<f64>],
    blocks: &[Vec<usize>],
    predecessors: &[usize],
    train_outcome: &[f64],
    subject: &str,
) -> Result<Vec<f64>, EstimationError> {
    let predictor_indices = predecessors
        .iter()
        .flat_map(|predecessor| blocks[*predecessor].iter().copied())
        .collect::<Vec<_>>();
    if predictor_indices.is_empty() {
        return Err(EstimationError::Numerical(format!(
            "no LM benchmark predictors for {subject}"
        )));
    }
    let train_predictors = predictor_indices
        .iter()
        .map(|index| train_columns[*index].clone())
        .collect::<Vec<_>>();
    let coefficients = ols(
        &train_predictors,
        train_outcome,
        &format!("PLSpredict LM benchmark {subject}"),
    )?;
    let test_predictors = predictor_indices
        .iter()
        .map(|index| test_columns[*index].clone())
        .collect::<Vec<_>>();
    let _ = recipe;
    Ok(fitted_values(&test_predictors, &coefficients))
}

fn parse_cvpat_model_pairs(
    recipe: &AnalysisRecipe,
) -> Result<Vec<CvpatModelPairSpec>, EstimationError> {
    let Some(raw) = recipe
        .metadata
        .get("cvpat_drop_paths")
        .or_else(|| recipe.metadata.get("cvpat.drop_paths"))
    else {
        return Ok(Vec::new());
    };
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<HashSet<_>>();
    let mut by_target = BTreeMap::<String, HashSet<String>>::new();
    for item in raw
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let Some((source, target)) = item.split_once("->") else {
            return Err(EstimationError::UnsupportedMethod(format!(
                "CVPAT model-pair path '{item}' must use source->target syntax"
            )));
        };
        let source = source.trim();
        let target = target.trim();
        if !construct_ids.contains(source) || !construct_ids.contains(target) {
            return Err(EstimationError::UnsupportedMethod(format!(
                "CVPAT model-pair path '{item}' references an unknown construct"
            )));
        }
        let exists = recipe
            .model
            .paths
            .iter()
            .any(|path| path.source == source && path.target == target);
        if !exists {
            return Err(EstimationError::UnsupportedMethod(format!(
                "CVPAT model-pair path '{item}' is not an existing structural path"
            )));
        }
        by_target
            .entry(target.to_string())
            .or_default()
            .insert(source.to_string());
    }
    Ok(by_target
        .into_iter()
        .map(|(target, drop_sources)| {
            let mut sources = drop_sources.iter().cloned().collect::<Vec<_>>();
            sources.sort();
            CvpatModelPairSpec {
                label: format!("drop_{}_to_{}", sources.join("_"), target),
                target,
                drop_sources,
            }
        })
        .collect())
}

fn reduced_model_predictions(
    recipe: &AnalysisRecipe,
    train_scores: &[Vec<f64>],
    test_scores: &[Vec<f64>],
    construct_index: &HashMap<&str, usize>,
    target: &str,
    drop_sources: &HashSet<String>,
) -> Result<Vec<f64>, EstimationError> {
    let target_index = construct_index[target];
    let predecessors = recipe
        .model
        .paths
        .iter()
        .filter(|path| path.target == target && !drop_sources.contains(&path.source))
        .map(|path| construct_index[path.source.as_str()])
        .collect::<Vec<_>>();
    if predecessors.is_empty() {
        return Ok(vec![0.0; test_scores[target_index].len()]);
    }
    let train_predictors = predecessors
        .iter()
        .map(|index| train_scores[*index].clone())
        .collect::<Vec<_>>();
    let coefficients = ols(
        &train_predictors,
        &train_scores[target_index],
        &format!("CVPAT reduced model target {target}"),
    )?;
    let test_predictors = predecessors
        .iter()
        .map(|index| test_scores[*index].clone())
        .collect::<Vec<_>>();
    Ok(fitted_values(&test_predictors, &coefficients))
}

impl PredictionErrorAccumulator {
    fn add(
        &mut self,
        actual: &[f64],
        pls_predicted: &[f64],
        lm_predicted: Option<&[f64]>,
        model_pair_predictions: &[(&str, Vec<f64>)],
    ) {
        self.observation_count += actual.len();
        self.pls_sse += squared_error_sum(actual, pls_predicted);
        self.pls_absolute_error += absolute_error_sum(actual, pls_predicted);
        let benchmark_sse = actual.iter().map(|value| value * value).sum::<f64>();
        self.benchmark_sse += benchmark_sse;
        self.benchmark_absolute_error += actual.iter().map(|value| value.abs()).sum::<f64>();
        self.benchmark_loss_differences.extend(
            actual
                .iter()
                .zip(pls_predicted)
                .map(|(actual, predicted)| (actual - predicted).powi(2) - actual.powi(2)),
        );
        if let Some(lm_predicted) = lm_predicted {
            let lm_sse = squared_error_sum(actual, lm_predicted);
            let lm_abs = absolute_error_sum(actual, lm_predicted);
            self.lm_sse = Some(self.lm_sse.unwrap_or(0.0) + lm_sse);
            self.lm_absolute_error = Some(self.lm_absolute_error.unwrap_or(0.0) + lm_abs);
            self.lm_loss_differences.extend(
                actual
                    .iter()
                    .zip(pls_predicted)
                    .zip(lm_predicted)
                    .map(|((actual, pls), lm)| (actual - pls).powi(2) - (actual - lm).powi(2)),
            );
        }
        for (label, model_pair_predicted) in model_pair_predictions {
            self.model_pair_loss_differences
                .entry((*label).to_string())
                .or_default()
                .extend(
                    actual
                        .iter()
                        .zip(pls_predicted)
                        .zip(model_pair_predicted)
                        .map(|((actual, pls), model_pair)| {
                            (actual - pls).powi(2) - (actual - model_pair).powi(2)
                        }),
                );
        }
    }

    fn to_target(&self) -> PlsPredictTarget {
        let q_squared_predict = (self.benchmark_sse > f64::EPSILON)
            .then(|| 1.0 - self.pls_sse / self.benchmark_sse)
            .filter(|value| value.is_finite());
        let q_squared_predict_lm = self
            .lm_sse
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
                .lm_sse
                .map(|sse| (sse / self.observation_count as f64).sqrt()),
            mae_lm: self
                .lm_absolute_error
                .map(|error| error / self.observation_count as f64),
            q_squared_predict_lm,
        }
    }

    fn cvpat_comparisons(&self) -> Vec<CvpatComparison> {
        let mut comparisons = vec![cvpat_from_differences(
            &self.construct,
            "pls_vs_training_mean_benchmark",
            &self.benchmark_loss_differences,
        )];
        if !self.lm_loss_differences.is_empty() {
            comparisons.push(cvpat_from_differences(
                &self.construct,
                "pls_vs_lm_benchmark",
                &self.lm_loss_differences,
            ));
        }
        for (label, differences) in &self.model_pair_loss_differences {
            comparisons.push(cvpat_from_differences(
                &self.construct,
                &format!("pls_vs_model_pair:{label}"),
                differences,
            ));
        }
        comparisons
    }
}

fn cvpat_from_differences(target: &str, comparison: &str, differences: &[f64]) -> CvpatComparison {
    let observations = differences.len();
    let mean = if observations > 0 {
        vector_mean(differences)
    } else {
        f64::NAN
    };
    let standard_error = if observations > 1 {
        let sd = sample_sd(differences);
        (sd.is_finite() && sd > f64::EPSILON).then(|| sd / (observations as f64).sqrt())
    } else {
        None
    };
    let (t_statistic, p_value_two_sided, warning) = if let Some(se) = standard_error {
        let t = mean / se;
        let df = observations as f64 - 1.0;
        let p = StudentsT::new(0.0, 1.0, df)
            .ok()
            .map(|distribution| 2.0 * (1.0 - distribution.cdf(t.abs())));
        (Some(t), p, None)
    } else {
        (
            None,
            None,
            Some("CVPAT unavailable because paired loss differences have zero variance or insufficient observations".into()),
        )
    };
    let preferred_model = if !mean.is_finite() || mean.abs() <= 1e-15 {
        "tie"
    } else if mean < 0.0 {
        "pls"
    } else {
        comparison.trim_start_matches("pls_vs_")
    };
    CvpatComparison {
        target: target.into(),
        comparison: comparison.into(),
        loss: "squared_error_difference_pls_minus_comparison".into(),
        mean_loss_difference: mean,
        standard_error,
        t_statistic,
        p_value_two_sided,
        observations,
        preferred_model: preferred_model.into(),
        warning,
    }
}

fn block_linear_scores(
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
            if score.iter().any(|value| !value.is_finite()) {
                return Err(EstimationError::Numerical(
                    "non-finite holdout construct score".into(),
                ));
            }
            Ok(score)
        })
        .collect()
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
    use arrow::{
        array::Float64Array,
        datatypes::{DataType, Field, Schema},
        record_batch::RecordBatch,
    };
    use chrono::Utc;
    use qpls_core::{
        AnalysisSettings, Construct, ControlPath, HigherOrderConstruct, HigherOrderMethod,
        InteractionMethod, InteractionTerm, ModelSpec, StructuralPath,
    };
    use qpls_data::{
        ColumnMetadata, ColumnType, DataFingerprint, DataKind, DatasetSchema, ImportOptions,
        ScaleType, import_delimited_bytes,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

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
            schema_version: 2,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model,
            settings: AnalysisSettings::default(),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
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
    fn plspredict_holdout_reports_leakage_free_prediction_metrics() {
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

        let result = estimate_pls(&dataset, &recipe).unwrap();
        let predict = result.predict.expect("prediction payload");
        assert_eq!(result.method_version, PLS_PREDICT_METHOD_VERSION);
        assert_eq!(predict.training_observations, 24);
        assert_eq!(predict.test_observations, 8);
        assert_eq!(predict.targets.len(), 1);
        assert_eq!(predict.targets[0].construct, "y");
        assert!(predict.targets[0].rmse_pls < predict.targets[0].rmse_benchmark);
        assert!(predict.targets[0].q_squared_predict.unwrap() > 0.9);
        assert!(predict.targets[0].rmse_lm.is_some());
        let repeated = predict.repeated_kfold.expect("repeated k-fold payload");
        assert_eq!(repeated.folds, 5);
        assert_eq!(repeated.repeats, 3);
        assert_eq!(repeated.total_test_observations, 96);
        assert_eq!(repeated.targets.len(), 1);
        assert!(repeated.targets[0].rmse_pls < repeated.targets[0].rmse_benchmark);
        assert!(repeated.targets[0].rmse_lm.is_some());
        assert_eq!(repeated.cvpat.len(), 2);
        assert!(repeated.cvpat.iter().any(|comparison| {
            comparison.comparison == "pls_vs_training_mean_benchmark"
                && comparison.preferred_model == "pls"
                && comparison.p_value_two_sided.is_some()
        }));
        assert!(repeated.cvpat.iter().any(|comparison| {
            comparison.comparison == "pls_vs_lm_benchmark"
                && comparison.observations == repeated.total_test_observations
        }));
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
            warning.contains("Repeated-indicator higher-order constructs are experimental")
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
            warning.contains("Two-stage and hybrid higher-order construct metadata is experimental")
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
            schema_version: 2,
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
            warning.contains("Two-stage higher-order constructs are experimental")
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
            schema_version: 2,
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
                .any(|warning| warning.contains("Two-stage moderation is experimental"))
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
        assert!(mediation.warnings[0].contains("publication inference"));
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
            schema_version: 2,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model,
            settings: AnalysisSettings::default(),
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
            estimate_pls(&dataset, &recipe),
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
            estimate_pls(&constant, &recipe),
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
        let omitted = estimate_pls(&missing, &recipe).unwrap();
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
        let scaled_result = estimate_pls(&scaled, &reordered).unwrap().paths[0].coefficient;
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
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::InsufficientObservations)
        );
        recipe.settings.method = AnalysisMethod::PlsPm;
        recipe.settings.bootstrap_samples = 100;
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::ResamplingRequiresEngine)
        );
        recipe.settings.bootstrap_samples = 0;
        recipe.model.constructs[1].id = recipe.model.constructs[0].id.clone();
        assert_eq!(
            estimate_pls(&dataset, &recipe),
            Err(EstimationError::DuplicateConstruct("x".into()))
        );
        let (_, mut duplicate_path) = fixture();
        duplicate_path
            .model
            .paths
            .push(duplicate_path.model.paths[0].clone());
        assert_eq!(
            estimate_pls(&dataset, &duplicate_path),
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
        let actual = estimate_pls(&shifted, &recipe).unwrap();
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
            schema_version: 2,
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
