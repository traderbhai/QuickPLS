//! Internal/Labs PLS model-comparison calculation foundation.
//!
//! This module deliberately does not participate in capability promotion or
//! Standard-surface routing.  It executes two distinct, point-estimate PLS-PM
//! recipes on the same raw dataset and the same deterministic cross-validation
//! partitions.  The scientific contract is frozen in
//! `docs/methods/PLS_MODEL_COMPARISON_V1.md`.

use arrow::{
    array::{Array, Float64Array, Int64Array, UInt32Array},
    compute::take,
    record_batch::RecordBatch,
};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, MeasurementMode, MethodConfig,
    MissingDataPolicy, ModelSpec,
};
use qpls_data::{DataFingerprint, DataKind, Dataset};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ContinuousCDF, StudentsT};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

use crate::{EstimationError, EstimationProgress, PlsResult, estimate_pls_with_control};

pub const PLS_MODEL_COMPARISON_METHOD_VERSION_V1: &str = "pls_model_comparison_v1";
pub const PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1: &str =
    "seeded_sha256_shared_complete_rows_round_robin_v1";
pub const PLS_MODEL_COMPARISON_PREDICTION_VERSION_V1: &str = "plspredict_indicator_shared_folds_v1";
pub const PLS_MODEL_COMPARISON_CVPAT_VERSION_V1: &str = "cvpat_paired_model_loss_liengaard_2021_v1";
pub const PLS_MODEL_COMPARISON_BIC_VERSION_V1: &str =
    "prediction_oriented_equation_bic_sharma_2019_v1";
pub const PLS_MODEL_COMPARISON_AKAIKE_WEIGHT_VERSION_V1: &str =
    "bic_delta_akaike_weight_two_candidate_v1";

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const OFFICIAL_DEFAULT_FOLDS: usize = 10;
const OFFICIAL_DEFAULT_REPEATS: usize = 10;
const MINIMUM_COMPLETE_CASES: usize = 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelComparisonConfigV1 {
    pub folds: usize,
    pub repeats: usize,
    pub seed: u64,
    pub confidence_level: f64,
}

impl PlsModelComparisonConfigV1 {
    pub const fn official_defaults(seed: u64) -> Self {
        Self {
            folds: OFFICIAL_DEFAULT_FOLDS,
            repeats: OFFICIAL_DEFAULT_REPEATS,
            seed,
            confidence_level: 0.95,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsModelComparisonModelRoleV1 {
    Established,
    Alternative,
}

impl PlsModelComparisonModelRoleV1 {
    const fn label(self) -> &'static str {
        match self {
            Self::Established => "established",
            Self::Alternative => "alternative",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsModelComparisonPhaseV1 {
    Validating,
    AssigningFolds,
    EstimatingEstablished,
    EstimatingAlternative,
    ComputingPairedCvpat,
    EstimatingInSampleBic,
    Assembling,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlsModelComparisonProgressV1 {
    pub phase: PlsModelComparisonPhaseV1,
    pub completed_units: u64,
    pub total_units: u64,
    pub repeat: Option<usize>,
    pub fold: Option<usize>,
    pub model: Option<PlsModelComparisonModelRoleV1>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlsModelComparisonErrorV1 {
    #[error("PLS model comparison was cancelled")]
    Cancelled,
    #[error("PLS model comparison requires raw observations")]
    RawDataRequired,
    #[error("{model} recipe fingerprint does not match the exact comparison dataset")]
    DatasetFingerprintMismatch { model: String },
    #[error(
        "{model} must be an executable schema-v3 point-estimate PLS algorithm recipe: {detail}"
    )]
    UnsupportedRecipe { model: String, detail: String },
    #[error("the two PLS models have the same scientific model identity")]
    SameScientificModel,
    #[error("the two PLS recipes use incompatible estimation settings: {0}")]
    IncompatibleSettings(String),
    #[error(
        "model comparison v1 does not support {feature}; use a separately contracted extension"
    )]
    UnsupportedFeature { feature: String },
    #[error("the compared models must expose the same nonempty reflective endogenous targets: {0}")]
    IncompatiblePredictionTargets(String),
    #[error("unknown or nonnumeric indicator: {0}")]
    InvalidIndicator(String),
    #[error("indicator {0} is assigned more than once in a model")]
    DuplicateIndicator(String),
    #[error(
        "shared-fold model comparison requires at least {required} complete cases; found {found}"
    )]
    InsufficientCompleteCases { required: usize, found: usize },
    #[error("invalid shared-fold configuration: {0}")]
    InvalidFoldConfiguration(String),
    #[error("Arrow row subsetting failed: {0}")]
    Arrow(String),
    #[error("{model} estimation failed at repeat {repeat:?}, fold {fold:?}: {detail}")]
    Estimation {
        model: String,
        repeat: Option<usize>,
        fold: Option<usize>,
        detail: String,
    },
    #[error("prediction contract failed for {subject}: {detail}")]
    PredictionContract { subject: String, detail: String },
    #[error("prediction-oriented BIC requires n >= 3, finite SSE > 0, and p >= 1")]
    InvalidPredictionOrientedBicInput,
    #[error("BIC-based Akaike weights require two finite BIC values")]
    InvalidAkaikeWeightInput,
    #[error("paired CVPAT requires equal finite loss vectors with at least three cases")]
    InvalidCvpatInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlsModelComparisonModelIdentityV1 {
    pub role: PlsModelComparisonModelRoleV1,
    pub recipe_id: String,
    pub model_id: String,
    pub model_name: String,
    pub scientific_model_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlsModelComparisonFoldLedgerEntryV1 {
    pub repeat: usize,
    pub source_row: usize,
    pub fold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlsModelComparisonFoldPlanV1 {
    pub assignment_version: String,
    pub seed: u64,
    pub folds: usize,
    pub repeats: usize,
    pub complete_rows: Vec<usize>,
    pub assignment_digest: String,
    pub ledger: Vec<PlsModelComparisonFoldLedgerEntryV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonErrorMetricsV1 {
    pub observations: usize,
    pub squared_error_sum: f64,
    pub absolute_error_sum: f64,
    pub rmse: f64,
    pub mae: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonIndicatorPredictionV1 {
    pub construct: String,
    pub indicator: String,
    pub established: PlsModelComparisonErrorMetricsV1,
    pub alternative: PlsModelComparisonErrorMetricsV1,
    pub indicator_average: PlsModelComparisonErrorMetricsV1,
    pub q_squared_predict_established: Option<f64>,
    pub q_squared_predict_alternative: Option<f64>,
    pub lower_rmse_model: String,
    pub lower_mae_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonFoldLossV1 {
    pub repeat: usize,
    pub fold: usize,
    pub test_rows: Vec<usize>,
    pub established_mean_loss: f64,
    pub alternative_mean_loss: f64,
    pub average_loss_difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonCaseLossV1 {
    pub source_row: usize,
    pub repeats: usize,
    pub established_mean_loss: f64,
    pub alternative_mean_loss: f64,
    pub loss_difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonCvpatV1 {
    pub method_version: String,
    pub loss: String,
    pub target_scope: String,
    pub established_mean_loss: f64,
    pub alternative_mean_loss: f64,
    /// Liengaard et al. (2021), Eq. 1: alternative minus established.
    pub average_loss_difference: f64,
    pub sample_variance_of_case_differences: Option<f64>,
    pub standard_error: Option<f64>,
    pub t_statistic: Option<f64>,
    pub degrees_of_freedom: usize,
    pub p_value_one_sided_alternative_lower: Option<f64>,
    pub p_value_two_sided: Option<f64>,
    pub confidence_level: f64,
    pub confidence_interval_lower: Option<f64>,
    pub confidence_interval_upper: Option<f64>,
    pub observations: usize,
    pub lower_loss_model: String,
    pub directional_decision: String,
    pub status: String,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonBicRowV1 {
    pub construct: String,
    pub sample_size: usize,
    pub established_sse: f64,
    pub alternative_sse: f64,
    pub established_parameter_count: usize,
    pub alternative_parameter_count: usize,
    pub established_bic: f64,
    pub alternative_bic: f64,
    pub established_delta_bic: f64,
    pub alternative_delta_bic: f64,
    pub established_akaike_weight: f64,
    pub alternative_akaike_weight: f64,
    pub lower_bic_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelComparisonResultV1 {
    pub method_version: String,
    pub surface: String,
    pub qualified: bool,
    pub dataset_fingerprint: String,
    pub established: PlsModelComparisonModelIdentityV1,
    pub alternative: PlsModelComparisonModelIdentityV1,
    pub fold_plan: PlsModelComparisonFoldPlanV1,
    pub prediction_method_version: String,
    pub indicator_predictions: Vec<PlsModelComparisonIndicatorPredictionV1>,
    pub completed_fold_pairs: usize,
    pub failed_fold_pairs: usize,
    pub fold_losses: Vec<PlsModelComparisonFoldLossV1>,
    pub case_losses: Vec<PlsModelComparisonCaseLossV1>,
    pub cvpat: PlsModelComparisonCvpatV1,
    pub bic_method_version: String,
    pub akaike_weight_method_version: String,
    pub bic: Vec<PlsModelComparisonBicRowV1>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TargetKey {
    construct: String,
    indicator: String,
}

#[derive(Debug, Clone)]
struct FoldIndicatorPrediction {
    key: TargetKey,
    actual: Vec<f64>,
    predicted: Vec<f64>,
    indicator_average: Vec<f64>,
}

#[derive(Debug, Clone)]
struct ModelFoldPrediction {
    indicators: Vec<FoldIndicatorPrediction>,
}

#[derive(Debug, Clone, Default)]
struct MetricAccumulator {
    observations: usize,
    squared_error_sum: f64,
    absolute_error_sum: f64,
}

#[derive(Debug, Clone, Default)]
struct IndicatorComparisonAccumulator {
    established: MetricAccumulator,
    alternative: MetricAccumulator,
    indicator_average: MetricAccumulator,
}

#[derive(Debug, Clone, Copy, Default)]
struct CaseLossAccumulator {
    established_sum: f64,
    alternative_sum: f64,
    repeats: usize,
}

/// Executes the Internal/Labs two-model scientific calculation.
pub fn compare_pls_models_v1(
    dataset: &Dataset,
    established: &AnalysisRecipe,
    alternative: &AnalysisRecipe,
    config: PlsModelComparisonConfigV1,
) -> Result<PlsModelComparisonResultV1, PlsModelComparisonErrorV1> {
    compare_pls_models_v1_with_control(dataset, established, alternative, config, |_| true)
}

/// Executes with cooperative progress and cancellation. Returning `false` from
/// `control` cancels both the outer comparison and an in-flight PLS fit.
pub fn compare_pls_models_v1_with_control(
    dataset: &Dataset,
    established: &AnalysisRecipe,
    alternative: &AnalysisRecipe,
    config: PlsModelComparisonConfigV1,
    mut control: impl FnMut(PlsModelComparisonProgressV1) -> bool,
) -> Result<PlsModelComparisonResultV1, PlsModelComparisonErrorV1> {
    let total_units = (config
        .folds
        .saturating_mul(config.repeats)
        .saturating_mul(2)
        + 2) as u64;
    checkpoint(
        &mut control,
        PlsModelComparisonPhaseV1::Validating,
        0,
        total_units,
        None,
        None,
        None,
    )?;
    validate_config(config)?;
    validate_recipe(
        dataset,
        established,
        PlsModelComparisonModelRoleV1::Established,
    )?;
    validate_recipe(
        dataset,
        alternative,
        PlsModelComparisonModelRoleV1::Alternative,
    )?;
    validate_compatible_settings(established, alternative)?;
    let established_digest = scientific_model_digest_v1(&established.model);
    let alternative_digest = scientific_model_digest_v1(&alternative.model);
    if established_digest == alternative_digest {
        return Err(PlsModelComparisonErrorV1::SameScientificModel);
    }
    let targets = compatible_targets(established, alternative)?;
    let indicators = union_indicators(established, alternative)?;
    let positions = indicator_positions(dataset, &indicators)?;
    let complete_rows = complete_rows(dataset, &positions)?;
    let minimum = MINIMUM_COMPLETE_CASES.max(config.folds.saturating_mul(2));
    if complete_rows.len() < minimum {
        return Err(PlsModelComparisonErrorV1::InsufficientCompleteCases {
            required: minimum,
            found: complete_rows.len(),
        });
    }
    checkpoint(
        &mut control,
        PlsModelComparisonPhaseV1::AssigningFolds,
        0,
        total_units,
        None,
        None,
        None,
    )?;
    let fold_plan = shared_fold_plan_v1(&complete_rows, config)?;

    let mut metric_accumulators = targets
        .iter()
        .cloned()
        .map(|target| (target, IndicatorComparisonAccumulator::default()))
        .collect::<BTreeMap<_, _>>();
    let mut case_losses = complete_rows
        .iter()
        .copied()
        .map(|row| (row, CaseLossAccumulator::default()))
        .collect::<BTreeMap<_, _>>();
    let mut fold_losses = Vec::with_capacity(config.folds * config.repeats);
    let mut completed_units = 0_u64;

    for repeat in 0..config.repeats {
        let assignments = assignments_for_repeat(&fold_plan, repeat)?;
        for fold in 0..config.folds {
            let train_rows = complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (assignments[index] != fold).then_some(*row))
                .collect::<Vec<_>>();
            let test_rows = complete_rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (assignments[index] == fold).then_some(*row))
                .collect::<Vec<_>>();
            if train_rows.len() < 3 || test_rows.len() < 2 {
                return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
                    format!(
                        "repeat {repeat}, fold {fold} produced {} training and {} test cases",
                        train_rows.len(),
                        test_rows.len()
                    ),
                ));
            }

            let training = subset_dataset(dataset, &train_rows, "training")?;
            let established_fit = fit_point_model(
                &training,
                established,
                config.seed,
                PlsModelComparisonModelRoleV1::Established,
                Some(repeat),
                Some(fold),
                completed_units,
                total_units,
                &mut control,
            )?;
            let established_prediction = predict_fold(
                dataset,
                established,
                &established_fit,
                &train_rows,
                &test_rows,
                &targets,
            )?;
            completed_units += 1;

            let alternative_fit = fit_point_model(
                &training,
                alternative,
                config.seed,
                PlsModelComparisonModelRoleV1::Alternative,
                Some(repeat),
                Some(fold),
                completed_units,
                total_units,
                &mut control,
            )?;
            let alternative_prediction = predict_fold(
                dataset,
                alternative,
                &alternative_fit,
                &train_rows,
                &test_rows,
                &targets,
            )?;
            completed_units += 1;

            let fold_loss = accumulate_fold(
                repeat,
                fold,
                &test_rows,
                &established_prediction,
                &alternative_prediction,
                &mut metric_accumulators,
                &mut case_losses,
            )?;
            fold_losses.push(fold_loss);
        }
    }

    checkpoint(
        &mut control,
        PlsModelComparisonPhaseV1::ComputingPairedCvpat,
        completed_units,
        total_units,
        None,
        None,
        None,
    )?;
    if case_losses
        .values()
        .any(|case| case.repeats != config.repeats)
    {
        return Err(PlsModelComparisonErrorV1::PredictionContract {
            subject: "shared fold coverage".into(),
            detail: "every complete case must be predicted exactly once per repeat".into(),
        });
    }
    let established_case_losses = case_losses
        .values()
        .map(|case| case.established_sum / case.repeats as f64)
        .collect::<Vec<_>>();
    let alternative_case_losses = case_losses
        .values()
        .map(|case| case.alternative_sum / case.repeats as f64)
        .collect::<Vec<_>>();
    let case_loss_rows = case_losses
        .iter()
        .map(|(source_row, case)| {
            let established_mean_loss = case.established_sum / case.repeats as f64;
            let alternative_mean_loss = case.alternative_sum / case.repeats as f64;
            PlsModelComparisonCaseLossV1 {
                source_row: *source_row,
                repeats: case.repeats,
                established_mean_loss,
                alternative_mean_loss,
                loss_difference: alternative_mean_loss - established_mean_loss,
            }
        })
        .collect::<Vec<_>>();
    let cvpat = paired_cvpat_v1(
        &established_case_losses,
        &alternative_case_losses,
        config.confidence_level,
    )?;

    checkpoint(
        &mut control,
        PlsModelComparisonPhaseV1::EstimatingInSampleBic,
        completed_units,
        total_units,
        None,
        None,
        None,
    )?;
    let analysis_dataset = subset_dataset(dataset, &complete_rows, "complete_case_analysis")?;
    let established_full = fit_point_model(
        &analysis_dataset,
        established,
        config.seed,
        PlsModelComparisonModelRoleV1::Established,
        None,
        None,
        completed_units,
        total_units,
        &mut control,
    )?;
    completed_units += 1;
    let alternative_full = fit_point_model(
        &analysis_dataset,
        alternative,
        config.seed,
        PlsModelComparisonModelRoleV1::Alternative,
        None,
        None,
        completed_units,
        total_units,
        &mut control,
    )?;
    completed_units += 1;
    let bic = bic_rows(
        established,
        alternative,
        &established_full,
        &alternative_full,
        &targets,
        complete_rows.len(),
    )?;

    checkpoint(
        &mut control,
        PlsModelComparisonPhaseV1::Assembling,
        completed_units,
        total_units,
        None,
        None,
        None,
    )?;
    let indicator_predictions = metric_accumulators
        .into_iter()
        .map(|(key, accumulator)| indicator_result(key, accumulator))
        .collect::<Vec<_>>();
    Ok(PlsModelComparisonResultV1 {
        method_version: PLS_MODEL_COMPARISON_METHOD_VERSION_V1.into(),
        surface: INTERNAL_LABS_SURFACE.into(),
        qualified: false,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        established: model_identity(
            established,
            PlsModelComparisonModelRoleV1::Established,
            established_digest,
        ),
        alternative: model_identity(
            alternative,
            PlsModelComparisonModelRoleV1::Alternative,
            alternative_digest,
        ),
        fold_plan,
        prediction_method_version: PLS_MODEL_COMPARISON_PREDICTION_VERSION_V1.into(),
        indicator_predictions,
        completed_fold_pairs: config.folds * config.repeats,
        failed_fold_pairs: 0,
        fold_losses,
        case_losses: case_loss_rows,
        cvpat,
        bic_method_version: PLS_MODEL_COMPARISON_BIC_VERSION_V1.into(),
        akaike_weight_method_version: PLS_MODEL_COMPARISON_AKAIKE_WEIGHT_VERSION_V1.into(),
        bic,
        warnings: vec![
            "Internal/Labs scientific foundation only; this result is not release-qualified and is not available in Standard Calculate."
                .into(),
            "Model-comparison CVPAT uses exact shared folds and the paired case-loss equations from Liengaard et al. (2021); it is distinct from single-model IA/LM benchmark CVPAT."
                .into(),
            "BIC is the equation-level prediction-oriented residual-SSE criterion documented for PLS model selection. Generic likelihood BIC, AIC, GM, and whole-model BIC are not inferred or substituted."
                .into(),
        ],
    })
}

fn validate_config(config: PlsModelComparisonConfigV1) -> Result<(), PlsModelComparisonErrorV1> {
    if config.folds < 2 {
        return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
            "folds must be at least 2".into(),
        ));
    }
    if config.repeats == 0 {
        return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
            "repeats must be at least 1".into(),
        ));
    }
    if !config.confidence_level.is_finite()
        || config.confidence_level <= 0.0
        || config.confidence_level >= 1.0
    {
        return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
            "confidence_level must be finite and strictly between 0 and 1".into(),
        ));
    }
    Ok(())
}

fn validate_recipe(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    role: PlsModelComparisonModelRoleV1,
) -> Result<(), PlsModelComparisonErrorV1> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlsModelComparisonErrorV1::RawDataRequired);
    }
    if recipe.dataset_fingerprint != dataset.fingerprint.0 {
        return Err(PlsModelComparisonErrorV1::DatasetFingerprintMismatch {
            model: role.label().into(),
        });
    }
    if recipe.schema_version != ANALYSIS_RECIPE_SCHEMA_VERSION
        || recipe.settings.method != AnalysisMethod::PlsPm
        || recipe.method_config != Some(MethodConfig::PlsAlgorithm)
        || recipe.settings.bootstrap_samples != 0
        || recipe.settings.studentized_inner_samples != 0
        || recipe.settings.permutation_samples != 0
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
    {
        return Err(PlsModelComparisonErrorV1::UnsupportedRecipe {
            model: role.label().into(),
            detail: "only exact unweighted, listwise, point-estimate MethodConfig::PlsAlgorithm recipes are accepted"
                .into(),
        });
    }
    for (feature, present) in [
        (
            "observed control regressions",
            !recipe.model.controls.is_empty(),
        ),
        (
            "higher-order constructs",
            !recipe.model.higher_order_constructs.is_empty(),
        ),
        ("interaction terms", !recipe.model.interactions.is_empty()),
    ] {
        if present {
            return Err(PlsModelComparisonErrorV1::UnsupportedFeature {
                feature: feature.into(),
            });
        }
    }
    Ok(())
}

fn validate_compatible_settings(
    established: &AnalysisRecipe,
    alternative: &AnalysisRecipe,
) -> Result<(), PlsModelComparisonErrorV1> {
    let left = &established.settings;
    let right = &alternative.settings;
    if left.weighting_scheme != right.weighting_scheme {
        return Err(PlsModelComparisonErrorV1::IncompatibleSettings(
            "weighting schemes differ".into(),
        ));
    }
    if left.preprocessing != right.preprocessing {
        return Err(PlsModelComparisonErrorV1::IncompatibleSettings(
            "preprocessing policies differ".into(),
        ));
    }
    if left.tolerance.to_bits() != right.tolerance.to_bits()
        || left.max_iterations != right.max_iterations
    {
        return Err(PlsModelComparisonErrorV1::IncompatibleSettings(
            "convergence settings differ".into(),
        ));
    }
    Ok(())
}

fn compatible_targets(
    established: &AnalysisRecipe,
    alternative: &AnalysisRecipe,
) -> Result<Vec<TargetKey>, PlsModelComparisonErrorV1> {
    fn targets(recipe: &AnalysisRecipe) -> Result<BTreeMap<String, Vec<String>>, String> {
        let endogenous = recipe
            .model
            .paths
            .iter()
            .map(|path| path.target.as_str())
            .collect::<BTreeSet<_>>();
        let mut result = BTreeMap::new();
        for construct in &recipe.model.constructs {
            if !endogenous.contains(construct.id.as_str()) {
                continue;
            }
            if construct.mode != MeasurementMode::Reflective {
                return Err(format!(
                    "endogenous construct '{}' is not reflective",
                    construct.id
                ));
            }
            let mut indicators = construct.indicators.clone();
            indicators.sort();
            if indicators.is_empty() {
                return Err(format!(
                    "endogenous construct '{}' has no indicators",
                    construct.id
                ));
            }
            result.insert(construct.id.clone(), indicators);
        }
        Ok(result)
    }
    let left =
        targets(established).map_err(PlsModelComparisonErrorV1::IncompatiblePredictionTargets)?;
    let right =
        targets(alternative).map_err(PlsModelComparisonErrorV1::IncompatiblePredictionTargets)?;
    if left.is_empty() || left != right {
        return Err(PlsModelComparisonErrorV1::IncompatiblePredictionTargets(
            "endogenous construct IDs and indicator sets differ".into(),
        ));
    }
    Ok(left
        .into_iter()
        .flat_map(|(construct, indicators)| {
            indicators.into_iter().map(move |indicator| TargetKey {
                construct: construct.clone(),
                indicator,
            })
        })
        .collect())
}

fn union_indicators(
    established: &AnalysisRecipe,
    alternative: &AnalysisRecipe,
) -> Result<Vec<String>, PlsModelComparisonErrorV1> {
    for recipe in [established, alternative] {
        let mut within = BTreeSet::new();
        for indicator in recipe
            .model
            .constructs
            .iter()
            .flat_map(|construct| construct.indicators.iter())
        {
            if !within.insert(indicator.clone()) {
                return Err(PlsModelComparisonErrorV1::DuplicateIndicator(
                    indicator.clone(),
                ));
            }
        }
    }
    Ok(established
        .model
        .constructs
        .iter()
        .chain(&alternative.model.constructs)
        .flat_map(|construct| construct.indicators.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn indicator_positions(
    dataset: &Dataset,
    indicators: &[String],
) -> Result<Vec<usize>, PlsModelComparisonErrorV1> {
    let schema = dataset.batch.schema();
    indicators
        .iter()
        .map(|name| {
            let position = schema
                .index_of(name)
                .map_err(|_| PlsModelComparisonErrorV1::InvalidIndicator(name.clone()))?;
            let array = dataset.batch.column(position).as_ref();
            if array.as_any().is::<Float64Array>() || array.as_any().is::<Int64Array>() {
                Ok(position)
            } else {
                Err(PlsModelComparisonErrorV1::InvalidIndicator(name.clone()))
            }
        })
        .collect()
}

fn complete_rows(
    dataset: &Dataset,
    positions: &[usize],
) -> Result<Vec<usize>, PlsModelComparisonErrorV1> {
    Ok((0..dataset.batch.num_rows())
        .filter(|row| {
            positions.iter().all(|position| {
                let array = dataset.batch.column(*position).as_ref();
                !array.is_null(*row)
                    && numeric_value(array, *row).is_some_and(|value| value.is_finite())
            })
        })
        .collect())
}

pub fn shared_fold_plan_v1(
    complete_rows: &[usize],
    config: PlsModelComparisonConfigV1,
) -> Result<PlsModelComparisonFoldPlanV1, PlsModelComparisonErrorV1> {
    validate_config(config)?;
    if complete_rows.len() < config.folds.saturating_mul(2) {
        return Err(PlsModelComparisonErrorV1::InsufficientCompleteCases {
            required: config.folds.saturating_mul(2),
            found: complete_rows.len(),
        });
    }
    if complete_rows
        .windows(2)
        .any(|window| window[0] >= window[1])
    {
        return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
            "complete_rows must be unique and strictly increasing".into(),
        ));
    }
    let mut ledger = Vec::with_capacity(complete_rows.len() * config.repeats);
    let mut digest = Sha256::new();
    digest.update(PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1.as_bytes());
    digest.update(config.seed.to_le_bytes());
    digest.update((config.folds as u64).to_le_bytes());
    digest.update((config.repeats as u64).to_le_bytes());
    for repeat in 0..config.repeats {
        let mut ranked = complete_rows
            .iter()
            .enumerate()
            .map(|(complete_index, source_row)| {
                let mut rank = Sha256::new();
                rank.update(PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1.as_bytes());
                rank.update(config.seed.to_le_bytes());
                rank.update((repeat as u64).to_le_bytes());
                rank.update((*source_row as u64).to_le_bytes());
                (rank.finalize().to_vec(), complete_index, *source_row)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        let mut assignments = vec![0_usize; complete_rows.len()];
        for (position, (_, complete_index, _)) in ranked.into_iter().enumerate() {
            assignments[complete_index] = position % config.folds;
        }
        for (complete_index, source_row) in complete_rows.iter().enumerate() {
            let fold = assignments[complete_index];
            digest.update((repeat as u64).to_le_bytes());
            digest.update((*source_row as u64).to_le_bytes());
            digest.update((fold as u64).to_le_bytes());
            ledger.push(PlsModelComparisonFoldLedgerEntryV1 {
                repeat,
                source_row: *source_row,
                fold,
            });
        }
    }
    Ok(PlsModelComparisonFoldPlanV1 {
        assignment_version: PLS_MODEL_COMPARISON_FOLD_ASSIGNMENT_VERSION_V1.into(),
        seed: config.seed,
        folds: config.folds,
        repeats: config.repeats,
        complete_rows: complete_rows.to_vec(),
        assignment_digest: format!("sha256:{}", hex_digest(digest.finalize().as_slice())),
        ledger,
    })
}

fn assignments_for_repeat(
    plan: &PlsModelComparisonFoldPlanV1,
    repeat: usize,
) -> Result<Vec<usize>, PlsModelComparisonErrorV1> {
    let rows = plan
        .ledger
        .iter()
        .filter(|entry| entry.repeat == repeat)
        .collect::<Vec<_>>();
    if rows.len() != plan.complete_rows.len()
        || rows
            .iter()
            .zip(&plan.complete_rows)
            .any(|(entry, expected)| entry.source_row != *expected || entry.fold >= plan.folds)
    {
        return Err(PlsModelComparisonErrorV1::InvalidFoldConfiguration(
            "fold ledger is incomplete or malformed".into(),
        ));
    }
    Ok(rows.iter().map(|entry| entry.fold).collect())
}

fn subset_dataset(
    dataset: &Dataset,
    rows: &[usize],
    purpose: &str,
) -> Result<Dataset, PlsModelComparisonErrorV1> {
    if rows.iter().any(|row| *row > u32::MAX as usize) {
        return Err(PlsModelComparisonErrorV1::Arrow(
            "source row exceeds Arrow UInt32 take range".into(),
        ));
    }
    let indices = UInt32Array::from(rows.iter().map(|row| *row as u32).collect::<Vec<_>>());
    let columns = dataset
        .batch
        .columns()
        .iter()
        .map(|column| take(column.as_ref(), &indices, None))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| PlsModelComparisonErrorV1::Arrow(error.to_string()))?;
    let batch = RecordBatch::try_new(dataset.batch.schema(), columns)
        .map_err(|error| PlsModelComparisonErrorV1::Arrow(error.to_string()))?;
    let mut schema = dataset.schema.clone();
    schema.case_count = rows.len();
    let mut digest = Sha256::new();
    digest.update(b"quickpls-pls-model-comparison-subset-v1\0");
    digest.update(dataset.fingerprint.0.as_bytes());
    digest.update(purpose.as_bytes());
    for row in rows {
        digest.update((*row as u64).to_le_bytes());
    }
    Ok(Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema,
        batch,
        fingerprint: DataFingerprint(format!(
            "sha256:{}",
            hex_digest(digest.finalize().as_slice())
        )),
    })
}

#[allow(clippy::too_many_arguments)]
fn fit_point_model(
    dataset: &Dataset,
    source: &AnalysisRecipe,
    seed: u64,
    role: PlsModelComparisonModelRoleV1,
    repeat: Option<usize>,
    fold: Option<usize>,
    completed_units: u64,
    total_units: u64,
    control: &mut impl FnMut(PlsModelComparisonProgressV1) -> bool,
) -> Result<PlsResult, PlsModelComparisonErrorV1> {
    let phase = match role {
        PlsModelComparisonModelRoleV1::Established => {
            PlsModelComparisonPhaseV1::EstimatingEstablished
        }
        PlsModelComparisonModelRoleV1::Alternative => {
            PlsModelComparisonPhaseV1::EstimatingAlternative
        }
    };
    checkpoint(
        control,
        phase,
        completed_units,
        total_units,
        repeat,
        fold,
        Some(role),
    )?;
    let mut recipe = source.clone();
    recipe.schema_version = ANALYSIS_RECIPE_SCHEMA_VERSION;
    recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
    recipe.settings.method = AnalysisMethod::PlsPm;
    recipe.settings.bootstrap_samples = 0;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.permutation_samples = 0;
    recipe.settings.seed = seed;
    recipe.settings.workers = 1;
    recipe.settings.case_weight_column = None;
    recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    recipe.metadata.clear();
    estimate_pls_with_control(dataset, &recipe, |_: EstimationProgress| {
        control(PlsModelComparisonProgressV1 {
            phase,
            completed_units,
            total_units,
            repeat,
            fold,
            model: Some(role),
        })
    })
    .map_err(|error| match error {
        EstimationError::Cancelled => PlsModelComparisonErrorV1::Cancelled,
        other => PlsModelComparisonErrorV1::Estimation {
            model: role.label().into(),
            repeat,
            fold,
            detail: other.to_string(),
        },
    })
}

fn predict_fold(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    fit: &PlsResult,
    train_rows: &[usize],
    test_rows: &[usize],
    targets: &[TargetKey],
) -> Result<ModelFoldPrediction, PlsModelComparisonErrorV1> {
    let schema = dataset.batch.schema();
    let transforms = fit
        .transforms
        .iter()
        .map(|transform| {
            (
                transform.indicator.as_str(),
                (transform.mean, transform.scale),
            )
        })
        .collect::<HashMap<_, _>>();
    let weights = fit
        .outer_estimates
        .iter()
        .map(|outer| {
            (
                (outer.construct.as_str(), outer.indicator.as_str()),
                outer.weight,
            )
        })
        .collect::<HashMap<_, _>>();
    let construct_index = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let order = topological_order(&recipe.model, &construct_index)?;
    let mut observed_test_scores = vec![Vec::<f64>::new(); recipe.model.constructs.len()];
    for (construct_position, construct) in recipe.model.constructs.iter().enumerate() {
        let mut train_weighted = vec![0.0; train_rows.len()];
        let mut test_weighted = vec![0.0; test_rows.len()];
        for indicator in &construct.indicators {
            let position = schema
                .index_of(indicator)
                .map_err(|_| PlsModelComparisonErrorV1::InvalidIndicator(indicator.clone()))?;
            let (center, scale) = transforms.get(indicator.as_str()).copied().ok_or_else(|| {
                PlsModelComparisonErrorV1::PredictionContract {
                    subject: construct.id.clone(),
                    detail: format!("missing training transform for {indicator}"),
                }
            })?;
            if !scale.is_finite() || scale <= f64::EPSILON {
                return Err(PlsModelComparisonErrorV1::PredictionContract {
                    subject: indicator.clone(),
                    detail: "nonpositive training scale".into(),
                });
            }
            let weight = weights
                .get(&(construct.id.as_str(), indicator.as_str()))
                .copied()
                .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                    subject: construct.id.clone(),
                    detail: format!("missing fitted weight for {indicator}"),
                })?;
            for (output, row) in train_weighted.iter_mut().zip(train_rows) {
                *output +=
                    ((numeric_value(dataset.batch.column(position).as_ref(), *row).ok_or_else(
                        || PlsModelComparisonErrorV1::InvalidIndicator(indicator.clone()),
                    )? - center)
                        / scale)
                        * weight;
            }
            for (output, row) in test_weighted.iter_mut().zip(test_rows) {
                *output +=
                    ((numeric_value(dataset.batch.column(position).as_ref(), *row).ok_or_else(
                        || PlsModelComparisonErrorV1::InvalidIndicator(indicator.clone()),
                    )? - center)
                        / scale)
                        * weight;
            }
        }
        let mean = vector_mean(&train_weighted);
        let sd = sample_sd(&train_weighted);
        if !sd.is_finite() || sd <= f64::EPSILON {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: construct.id.clone(),
                detail: "training composite score has zero variance".into(),
            });
        }
        observed_test_scores[construct_position] = test_weighted
            .iter()
            .map(|value| (value - mean) / sd)
            .collect();
    }

    let path_coefficients = fit
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut predicted_scores = vec![None; recipe.model.constructs.len()];
    for target_index in order {
        let target = &recipe.model.constructs[target_index];
        let incoming = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target.id)
            .collect::<Vec<_>>();
        if incoming.is_empty() {
            predicted_scores[target_index] = Some(observed_test_scores[target_index].clone());
            continue;
        }
        let mut predicted = vec![0.0; test_rows.len()];
        for path in incoming {
            let source_index = construct_index[path.source.as_str()];
            let source_scores = predicted_scores[source_index].as_ref().ok_or_else(|| {
                PlsModelComparisonErrorV1::PredictionContract {
                    subject: target.id.clone(),
                    detail: "structural order is cyclic".into(),
                }
            })?;
            let coefficient = path_coefficients
                .get(&(path.source.as_str(), path.target.as_str()))
                .copied()
                .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                    subject: target.id.clone(),
                    detail: format!("missing fitted path {} -> {}", path.source, path.target),
                })?;
            for (value, source) in predicted.iter_mut().zip(source_scores) {
                *value += coefficient * source;
            }
        }
        predicted_scores[target_index] = Some(predicted);
    }

    let mut indicators = Vec::with_capacity(targets.len());
    for target in targets {
        let target_index = *construct_index
            .get(target.construct.as_str())
            .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                subject: target.construct.clone(),
                detail: "target construct disappeared".into(),
            })?;
        let training_scores = fit.construct_scores.get(&target.construct).ok_or_else(|| {
            PlsModelComparisonErrorV1::PredictionContract {
                subject: target.construct.clone(),
                detail: "fitted training scores are missing".into(),
            }
        })?;
        if training_scores.len() != train_rows.len() {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: target.construct.clone(),
                detail: "training score row count mismatch".into(),
            });
        }
        let position = schema
            .index_of(&target.indicator)
            .map_err(|_| PlsModelComparisonErrorV1::InvalidIndicator(target.indicator.clone()))?;
        let (center, scale) = transforms
            .get(target.indicator.as_str())
            .copied()
            .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                subject: target.indicator.clone(),
                detail: "missing training transform".into(),
            })?;
        let training_indicator = train_rows
            .iter()
            .map(|row| {
                numeric_value(dataset.batch.column(position).as_ref(), *row)
                    .map(|value| (value - center) / scale)
                    .ok_or_else(|| {
                        PlsModelComparisonErrorV1::InvalidIndicator(target.indicator.clone())
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let score_variance = sample_variance(training_scores);
        if !score_variance.is_finite() || score_variance <= f64::EPSILON {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: target.construct.clone(),
                detail: "fitted training score has zero variance".into(),
            });
        }
        let slope = covariance(&training_indicator, training_scores) / score_variance;
        let intercept = vector_mean(&training_indicator);
        let predicted_score = predicted_scores[target_index].as_ref().ok_or_else(|| {
            PlsModelComparisonErrorV1::PredictionContract {
                subject: target.construct.clone(),
                detail: "target score prediction is missing".into(),
            }
        })?;
        let predicted = predicted_score
            .iter()
            .map(|score| (intercept + slope * score) * scale + center)
            .collect::<Vec<_>>();
        let actual = test_rows
            .iter()
            .map(|row| {
                numeric_value(dataset.batch.column(position).as_ref(), *row).ok_or_else(|| {
                    PlsModelComparisonErrorV1::InvalidIndicator(target.indicator.clone())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let raw_training_mean = train_rows
            .iter()
            .map(|row| numeric_value(dataset.batch.column(position).as_ref(), *row).unwrap())
            .sum::<f64>()
            / train_rows.len() as f64;
        if predicted.iter().any(|value| !value.is_finite()) {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: target.indicator.clone(),
                detail: "non-finite holdout prediction".into(),
            });
        }
        indicators.push(FoldIndicatorPrediction {
            key: target.clone(),
            actual,
            predicted,
            indicator_average: vec![raw_training_mean; test_rows.len()],
        });
    }
    Ok(ModelFoldPrediction { indicators })
}

fn topological_order(
    model: &ModelSpec,
    construct_index: &HashMap<&str, usize>,
) -> Result<Vec<usize>, PlsModelComparisonErrorV1> {
    let mut incoming = vec![0_usize; model.constructs.len()];
    let mut outgoing = vec![Vec::<usize>::new(); model.constructs.len()];
    for path in &model.paths {
        let source = construct_index
            .get(path.source.as_str())
            .copied()
            .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                subject: path.source.clone(),
                detail: "unknown path source".into(),
            })?;
        let target = construct_index
            .get(path.target.as_str())
            .copied()
            .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                subject: path.target.clone(),
                detail: "unknown path target".into(),
            })?;
        incoming[target] += 1;
        outgoing[source].push(target);
    }
    let mut ready = (0..incoming.len())
        .filter(|index| incoming[*index] == 0)
        .collect::<Vec<_>>();
    ready.sort_unstable_by(|left, right| right.cmp(left));
    let mut order = Vec::with_capacity(model.constructs.len());
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
    if order.len() != model.constructs.len() {
        return Err(PlsModelComparisonErrorV1::PredictionContract {
            subject: "structural model".into(),
            detail: "directed cycle".into(),
        });
    }
    Ok(order)
}

fn accumulate_fold(
    repeat: usize,
    fold: usize,
    test_rows: &[usize],
    established: &ModelFoldPrediction,
    alternative: &ModelFoldPrediction,
    metrics: &mut BTreeMap<TargetKey, IndicatorComparisonAccumulator>,
    cases: &mut BTreeMap<usize, CaseLossAccumulator>,
) -> Result<PlsModelComparisonFoldLossV1, PlsModelComparisonErrorV1> {
    if established.indicators.len() != alternative.indicators.len()
        || established.indicators.is_empty()
    {
        return Err(PlsModelComparisonErrorV1::PredictionContract {
            subject: "paired fold output".into(),
            detail: "model target count mismatch".into(),
        });
    }
    for (left, right) in established.indicators.iter().zip(&alternative.indicators) {
        if left.key != right.key
            || left.actual != right.actual
            || left.actual.len() != test_rows.len()
            || left.predicted.len() != test_rows.len()
            || right.predicted.len() != test_rows.len()
        {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: "paired fold output".into(),
                detail: "models did not predict the exact same target cases".into(),
            });
        }
        let accumulator = metrics.get_mut(&left.key).ok_or_else(|| {
            PlsModelComparisonErrorV1::PredictionContract {
                subject: left.key.indicator.clone(),
                detail: "unknown target key".into(),
            }
        })?;
        accumulator.established.add(&left.actual, &left.predicted);
        accumulator.alternative.add(&right.actual, &right.predicted);
        accumulator
            .indicator_average
            .add(&left.actual, &left.indicator_average);
    }
    let mut established_fold_loss = 0.0;
    let mut alternative_fold_loss = 0.0;
    for (position, source_row) in test_rows.iter().enumerate() {
        let established_loss = established
            .indicators
            .iter()
            .map(|indicator| (indicator.actual[position] - indicator.predicted[position]).powi(2))
            .sum::<f64>()
            / established.indicators.len() as f64;
        let alternative_loss = alternative
            .indicators
            .iter()
            .map(|indicator| (indicator.actual[position] - indicator.predicted[position]).powi(2))
            .sum::<f64>()
            / alternative.indicators.len() as f64;
        let case = cases.get_mut(source_row).ok_or_else(|| {
            PlsModelComparisonErrorV1::PredictionContract {
                subject: format!("source row {source_row}"),
                detail: "case is outside the complete-case comparison set".into(),
            }
        })?;
        case.established_sum += established_loss;
        case.alternative_sum += alternative_loss;
        case.repeats += 1;
        established_fold_loss += established_loss;
        alternative_fold_loss += alternative_loss;
    }
    let established_mean_loss = established_fold_loss / test_rows.len() as f64;
    let alternative_mean_loss = alternative_fold_loss / test_rows.len() as f64;
    Ok(PlsModelComparisonFoldLossV1 {
        repeat,
        fold,
        test_rows: test_rows.to_vec(),
        established_mean_loss,
        alternative_mean_loss,
        average_loss_difference: alternative_mean_loss - established_mean_loss,
    })
}

impl MetricAccumulator {
    fn add(&mut self, actual: &[f64], predicted: &[f64]) {
        self.observations += actual.len();
        for (actual, predicted) in actual.iter().zip(predicted) {
            let error = actual - predicted;
            self.squared_error_sum += error * error;
            self.absolute_error_sum += error.abs();
        }
    }

    fn finish(self) -> PlsModelComparisonErrorMetricsV1 {
        PlsModelComparisonErrorMetricsV1 {
            observations: self.observations,
            squared_error_sum: self.squared_error_sum,
            absolute_error_sum: self.absolute_error_sum,
            rmse: (self.squared_error_sum / self.observations as f64).sqrt(),
            mae: self.absolute_error_sum / self.observations as f64,
        }
    }
}

fn indicator_result(
    key: TargetKey,
    accumulator: IndicatorComparisonAccumulator,
) -> PlsModelComparisonIndicatorPredictionV1 {
    let established = accumulator.established.finish();
    let alternative = accumulator.alternative.finish();
    let indicator_average = accumulator.indicator_average.finish();
    let q_squared_predict_established = (indicator_average.squared_error_sum > f64::EPSILON)
        .then(|| 1.0 - established.squared_error_sum / indicator_average.squared_error_sum)
        .filter(|value| value.is_finite());
    let q_squared_predict_alternative = (indicator_average.squared_error_sum > f64::EPSILON)
        .then(|| 1.0 - alternative.squared_error_sum / indicator_average.squared_error_sum)
        .filter(|value| value.is_finite());
    let lower_rmse_model = if established.rmse <= alternative.rmse {
        "established"
    } else {
        "alternative"
    };
    let lower_mae_model = if established.mae <= alternative.mae {
        "established"
    } else {
        "alternative"
    };
    PlsModelComparisonIndicatorPredictionV1 {
        construct: key.construct,
        indicator: key.indicator,
        established,
        alternative,
        indicator_average,
        q_squared_predict_established,
        q_squared_predict_alternative,
        lower_rmse_model: lower_rmse_model.into(),
        lower_mae_model: lower_mae_model.into(),
    }
}

/// Sharma et al. (2019), equation-level prediction-oriented BIC:
/// `n * ln(SSE / n) + p * ln(n)`.
pub fn prediction_oriented_bic_v1(
    sample_size: usize,
    sse: f64,
    parameter_count: usize,
) -> Result<f64, PlsModelComparisonErrorV1> {
    if sample_size < 3 || !sse.is_finite() || sse <= 0.0 || parameter_count == 0 {
        return Err(PlsModelComparisonErrorV1::InvalidPredictionOrientedBicInput);
    }
    let n = sample_size as f64;
    let bic = n * (sse / n).ln() + parameter_count as f64 * n.ln();
    if bic.is_finite() {
        Ok(bic)
    } else {
        Err(PlsModelComparisonErrorV1::InvalidPredictionOrientedBicInput)
    }
}

/// Returns `(first_delta, second_delta, first_weight, second_weight)` using
/// `exp(-0.5 * delta_i) / sum(exp(-0.5 * delta_j))`.
pub fn bic_akaike_weights_two_candidate_v1(
    first_bic: f64,
    second_bic: f64,
) -> Result<(f64, f64, f64, f64), PlsModelComparisonErrorV1> {
    if !first_bic.is_finite() || !second_bic.is_finite() {
        return Err(PlsModelComparisonErrorV1::InvalidAkaikeWeightInput);
    }
    let minimum = first_bic.min(second_bic);
    let first_delta = first_bic - minimum;
    let second_delta = second_bic - minimum;
    let first_relative = (-0.5 * first_delta).exp();
    let second_relative = (-0.5 * second_delta).exp();
    let denominator = first_relative + second_relative;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(PlsModelComparisonErrorV1::InvalidAkaikeWeightInput);
    }
    Ok((
        first_delta,
        second_delta,
        first_relative / denominator,
        second_relative / denominator,
    ))
}

/// Paired case-loss CVPAT from Liengaard et al. (2021), Eqs. 1-3. The
/// directional p-value tests whether the alternative model has lower loss.
pub fn paired_cvpat_v1(
    established_losses: &[f64],
    alternative_losses: &[f64],
    confidence_level: f64,
) -> Result<PlsModelComparisonCvpatV1, PlsModelComparisonErrorV1> {
    if established_losses.len() != alternative_losses.len()
        || established_losses.len() < 3
        || established_losses
            .iter()
            .chain(alternative_losses)
            .any(|value| !value.is_finite() || *value < 0.0)
        || !confidence_level.is_finite()
        || confidence_level <= 0.0
        || confidence_level >= 1.0
    {
        return Err(PlsModelComparisonErrorV1::InvalidCvpatInput);
    }
    let observations = established_losses.len();
    let established_mean_loss = vector_mean(established_losses);
    let alternative_mean_loss = vector_mean(alternative_losses);
    let differences = alternative_losses
        .iter()
        .zip(established_losses)
        .map(|(alternative, established)| alternative - established)
        .collect::<Vec<_>>();
    let average_loss_difference = vector_mean(&differences);
    let variance = sample_variance(&differences);
    let lower_loss_model = if average_loss_difference < 0.0 {
        "alternative"
    } else if average_loss_difference > 0.0 {
        "established"
    } else {
        "tie"
    };
    let unavailable = !variance.is_finite() || variance <= f64::EPSILON;
    if unavailable {
        return Ok(PlsModelComparisonCvpatV1 {
            method_version: PLS_MODEL_COMPARISON_CVPAT_VERSION_V1.into(),
            loss: "mean_squared_prediction_error_across_all_endogenous_indicators_per_case".into(),
            target_scope: "all_common_reflective_endogenous_indicators".into(),
            established_mean_loss,
            alternative_mean_loss,
            average_loss_difference,
            sample_variance_of_case_differences: None,
            standard_error: None,
            t_statistic: None,
            degrees_of_freedom: observations - 1,
            p_value_one_sided_alternative_lower: None,
            p_value_two_sided: None,
            confidence_level,
            confidence_interval_lower: None,
            confidence_interval_upper: None,
            observations,
            lower_loss_model: lower_loss_model.into(),
            directional_decision: "test_unavailable".into(),
            status: "unavailable".into(),
            unavailable_reason: Some(
                "paired case-loss differences have zero or non-finite sample variance".into(),
            ),
        });
    }
    let standard_error = (variance / observations as f64).sqrt();
    let t_statistic = average_loss_difference / standard_error;
    let distribution = StudentsT::new(0.0, 1.0, (observations - 1) as f64)
        .map_err(|_| PlsModelComparisonErrorV1::InvalidCvpatInput)?;
    let cdf = distribution.cdf(t_statistic).clamp(0.0, 1.0);
    let p_value_two_sided = (2.0 * cdf.min(1.0 - cdf)).clamp(0.0, 1.0);
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let confidence_interval_lower = average_loss_difference - critical * standard_error;
    let confidence_interval_upper = average_loss_difference + critical * standard_error;
    let alpha = 1.0 - confidence_level;
    let directional_decision = if cdf < alpha && average_loss_difference < 0.0 {
        "alternative_has_significantly_lower_loss"
    } else {
        "alternative_not_shown_to_have_lower_loss"
    };
    Ok(PlsModelComparisonCvpatV1 {
        method_version: PLS_MODEL_COMPARISON_CVPAT_VERSION_V1.into(),
        loss: "mean_squared_prediction_error_across_all_endogenous_indicators_per_case".into(),
        target_scope: "all_common_reflective_endogenous_indicators".into(),
        established_mean_loss,
        alternative_mean_loss,
        average_loss_difference,
        sample_variance_of_case_differences: Some(variance),
        standard_error: Some(standard_error),
        t_statistic: Some(t_statistic),
        degrees_of_freedom: observations - 1,
        p_value_one_sided_alternative_lower: Some(cdf),
        p_value_two_sided: Some(p_value_two_sided),
        confidence_level,
        confidence_interval_lower: Some(confidence_interval_lower),
        confidence_interval_upper: Some(confidence_interval_upper),
        observations,
        lower_loss_model: lower_loss_model.into(),
        directional_decision: directional_decision.into(),
        status: "available".into(),
        unavailable_reason: None,
    })
}

fn bic_rows(
    established_recipe: &AnalysisRecipe,
    alternative_recipe: &AnalysisRecipe,
    established_fit: &PlsResult,
    alternative_fit: &PlsResult,
    targets: &[TargetKey],
    sample_size: usize,
) -> Result<Vec<PlsModelComparisonBicRowV1>, PlsModelComparisonErrorV1> {
    let constructs = targets
        .iter()
        .map(|target| target.construct.clone())
        .collect::<BTreeSet<_>>();
    constructs
        .into_iter()
        .map(|construct| {
            let (established_sse, established_parameter_count) =
                structural_equation_sse(established_recipe, established_fit, &construct)?;
            let (alternative_sse, alternative_parameter_count) =
                structural_equation_sse(alternative_recipe, alternative_fit, &construct)?;
            let established_bic = prediction_oriented_bic_v1(
                sample_size,
                established_sse,
                established_parameter_count,
            )?;
            let alternative_bic = prediction_oriented_bic_v1(
                sample_size,
                alternative_sse,
                alternative_parameter_count,
            )?;
            let (
                established_delta_bic,
                alternative_delta_bic,
                established_akaike_weight,
                alternative_akaike_weight,
            ) = bic_akaike_weights_two_candidate_v1(established_bic, alternative_bic)?;
            Ok(PlsModelComparisonBicRowV1 {
                construct,
                sample_size,
                established_sse,
                alternative_sse,
                established_parameter_count,
                alternative_parameter_count,
                established_bic,
                alternative_bic,
                established_delta_bic,
                alternative_delta_bic,
                established_akaike_weight,
                alternative_akaike_weight,
                lower_bic_model: if established_bic <= alternative_bic {
                    "established".into()
                } else {
                    "alternative".into()
                },
            })
        })
        .collect()
}

fn structural_equation_sse(
    recipe: &AnalysisRecipe,
    fit: &PlsResult,
    target: &str,
) -> Result<(f64, usize), PlsModelComparisonErrorV1> {
    let outcome = fit.construct_scores.get(target).ok_or_else(|| {
        PlsModelComparisonErrorV1::PredictionContract {
            subject: target.into(),
            detail: "full-sample construct score is missing".into(),
        }
    })?;
    let incoming = recipe
        .model
        .paths
        .iter()
        .filter(|path| path.target == target)
        .collect::<Vec<_>>();
    if incoming.is_empty() {
        return Err(PlsModelComparisonErrorV1::PredictionContract {
            subject: target.into(),
            detail: "prediction-oriented BIC requires an endogenous structural equation".into(),
        });
    }
    let coefficients = fit
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut fitted = vec![0.0; outcome.len()];
    for path in &incoming {
        let predictor = fit.construct_scores.get(&path.source).ok_or_else(|| {
            PlsModelComparisonErrorV1::PredictionContract {
                subject: target.into(),
                detail: format!("predictor score '{}' is missing", path.source),
            }
        })?;
        if predictor.len() != outcome.len() {
            return Err(PlsModelComparisonErrorV1::PredictionContract {
                subject: target.into(),
                detail: "full-sample score length mismatch".into(),
            });
        }
        let coefficient = coefficients
            .get(&(path.source.as_str(), path.target.as_str()))
            .copied()
            .ok_or_else(|| PlsModelComparisonErrorV1::PredictionContract {
                subject: target.into(),
                detail: format!(
                    "fitted coefficient '{} -> {}' is missing",
                    path.source, target
                ),
            })?;
        for (value, predictor) in fitted.iter_mut().zip(predictor) {
            *value += coefficient * predictor;
        }
    }
    let sse = outcome
        .iter()
        .zip(fitted)
        .map(|(actual, predicted)| (actual - predicted).powi(2))
        .sum::<f64>();
    // Sharma et al.'s regression-equation count: predictor constructs plus
    // one intercept/error term in the criterion's complexity penalty.
    Ok((sse, incoming.len() + 1))
}

fn model_identity(
    recipe: &AnalysisRecipe,
    role: PlsModelComparisonModelRoleV1,
    digest: String,
) -> PlsModelComparisonModelIdentityV1 {
    PlsModelComparisonModelIdentityV1 {
        role,
        recipe_id: recipe.id.to_string(),
        model_id: recipe.model.id.to_string(),
        model_name: recipe.model.name.clone(),
        scientific_model_digest: digest,
    }
}

fn scientific_model_digest_v1(model: &ModelSpec) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-pls-scientific-model-identity-v1\0");
    let mut constructs = model.constructs.iter().collect::<Vec<_>>();
    constructs.sort_by(|left, right| left.id.cmp(&right.id));
    for construct in constructs {
        digest.update(b"construct\0");
        digest.update(construct.id.as_bytes());
        digest.update([match construct.mode {
            MeasurementMode::Reflective => 0,
            MeasurementMode::Formative => 1,
        }]);
        let mut indicators = construct.indicators.clone();
        indicators.sort();
        for indicator in indicators {
            digest.update(b"indicator\0");
            digest.update(indicator.as_bytes());
        }
    }
    let mut paths = model.paths.iter().collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then(left.target.cmp(&right.target))
    });
    for path in paths {
        digest.update(b"path\0");
        digest.update(path.source.as_bytes());
        digest.update(b"\0");
        digest.update(path.target.as_bytes());
    }
    format!("sha256:{}", hex_digest(digest.finalize().as_slice()))
}

#[allow(clippy::too_many_arguments)]
fn checkpoint(
    control: &mut impl FnMut(PlsModelComparisonProgressV1) -> bool,
    phase: PlsModelComparisonPhaseV1,
    completed_units: u64,
    total_units: u64,
    repeat: Option<usize>,
    fold: Option<usize>,
    model: Option<PlsModelComparisonModelRoleV1>,
) -> Result<(), PlsModelComparisonErrorV1> {
    control(PlsModelComparisonProgressV1 {
        phase,
        completed_units,
        total_units,
        repeat,
        fold,
        model,
    })
    .then_some(())
    .ok_or(PlsModelComparisonErrorV1::Cancelled)
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

fn covariance(left: &[f64], right: &[f64]) -> f64 {
    let left_mean = vector_mean(left);
    let right_mean = vector_mean(right);
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>()
        / (left.len() - 1) as f64
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{AnalysisSettings, Construct, MeasurementMode, ModelSpec, StructuralPath};
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use uuid::Uuid;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.15}, expected={expected:.15}, tolerance={tolerance}"
        );
    }

    #[test]
    fn formula_micro_oracle_matches_hand_calculation() {
        let bic = prediction_oriented_bic_v1(10, 5.0, 3).unwrap();
        assert_close(bic, 10.0 * 0.5_f64.ln() + 3.0 * 10.0_f64.ln(), 1e-14);
        let (first_delta, second_delta, first_weight, second_weight) =
            bic_akaike_weights_two_candidate_v1(10.0, 12.0).unwrap();
        assert_close(first_delta, 0.0, 0.0);
        assert_close(second_delta, 2.0, 0.0);
        assert_close(first_weight, 0.731_058_578_630_004_9, 1e-15);
        assert_close(second_weight, 0.268_941_421_369_995_1, 1e-15);
    }

    #[test]
    fn paired_cvpat_uses_alternative_minus_established_case_loss() {
        let established = [4.0, 1.0, 9.0, 4.0];
        let alternative = [1.0, 1.0, 4.0, 9.0];
        let result = paired_cvpat_v1(&established, &alternative, 0.95).unwrap();
        assert_eq!(result.status, "available");
        assert_close(result.average_loss_difference, -0.75, 1e-15);
        assert_close(
            result.sample_variance_of_case_differences.unwrap(),
            18.916_666_666_666_668,
            1e-14,
        );
        assert_close(result.standard_error.unwrap(), 2.174_664_725_116_648, 1e-14);
        assert_close(
            result.t_statistic.unwrap(),
            -0.344_880_749_357_706_35,
            1e-14,
        );
        assert_eq!(result.lower_loss_model, "alternative");
    }

    #[test]
    fn zero_variance_cvpat_is_explicitly_unavailable_without_nan() {
        let result = paired_cvpat_v1(&[2.0, 3.0, 4.0], &[1.0, 2.0, 3.0], 0.95).unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.average_loss_difference, -1.0);
        assert!(result.t_statistic.is_none());
        assert!(result.p_value_one_sided_alternative_lower.is_none());
        assert!(result.unavailable_reason.is_some());
    }

    #[test]
    fn shared_fold_plan_is_seeded_balanced_and_exactly_repeated() {
        let rows = (0..23).collect::<Vec<_>>();
        let config = PlsModelComparisonConfigV1 {
            folds: 5,
            repeats: 3,
            seed: 47,
            confidence_level: 0.95,
        };
        let first = shared_fold_plan_v1(&rows, config).unwrap();
        let repeat = shared_fold_plan_v1(&rows, config).unwrap();
        let changed =
            shared_fold_plan_v1(&rows, PlsModelComparisonConfigV1 { seed: 48, ..config }).unwrap();
        assert_eq!(first, repeat);
        assert_eq!(
            first.assignment_digest,
            "sha256:b08f53b2641bc2a2bc8eef4c46c56a5b4f5ad3a413fc195f210ec68212a25c74"
        );
        assert_ne!(first.assignment_digest, changed.assignment_digest);
        assert_eq!(first.ledger.len(), rows.len() * config.repeats);
        for repeat in 0..config.repeats {
            let assignments = assignments_for_repeat(&first, repeat).unwrap();
            let counts = (0..config.folds)
                .map(|fold| assignments.iter().filter(|value| **value == fold).count())
                .collect::<Vec<_>>();
            assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);
        }
    }

    fn fixture() -> (Dataset, AnalysisRecipe, AnalysisRecipe) {
        let mut csv = String::from("x1,x2,z1,z2,y1,y2\n");
        for row in 0..48 {
            let t = row as f64 / 7.0;
            let x = (t * 0.7).sin() + row as f64 * 0.015;
            let z = (t * 1.1).cos() - row as f64 * 0.009;
            let noise = ((row * 17 % 13) as f64 - 6.0) * 0.018;
            let y = 0.62 * x + 0.56 * z + noise;
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                x + noise * 0.2,
                x * 0.93 - noise * 0.15,
                z - noise * 0.12,
                z * 1.04 + noise * 0.18,
                y + noise * 0.25,
                y * 0.96 - noise * 0.2
            ));
        }
        let bytes = csv.into_bytes();
        let dataset = import_delimited_bytes(
            &bytes,
            "model-comparison.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let constructs = vec![
            Construct {
                id: "x".into(),
                name: "X".into(),
                short_name: "X".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into()],
            },
            Construct {
                id: "z".into(),
                name: "Z".into(),
                short_name: "Z".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["z1".into(), "z2".into()],
            },
            Construct {
                id: "y".into(),
                name: "Y".into(),
                short_name: "Y".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["y1".into(), "y2".into()],
            },
        ];
        let model = |name: &str, include_z: bool| ModelSpec {
            id: Uuid::new_v4(),
            name: name.into(),
            constructs: if include_z {
                constructs.clone()
            } else {
                constructs
                    .iter()
                    .filter(|construct| construct.id != "z")
                    .cloned()
                    .collect()
            },
            paths: if include_z {
                vec![
                    StructuralPath {
                        source: "x".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "z".into(),
                        target: "y".into(),
                    },
                ]
            } else {
                vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }]
            },
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let settings = AnalysisSettings {
            seed: 991,
            workers: 1,
            ..AnalysisSettings::default()
        };
        let mut established =
            AnalysisRecipe::new(&bytes, model("Established", false), settings.clone());
        let mut alternative = AnalysisRecipe::new(&bytes, model("Alternative", true), settings);
        established.dataset_fingerprint = dataset.fingerprint.0.clone();
        alternative.dataset_fingerprint = dataset.fingerprint.0.clone();
        (dataset, established, alternative)
    }

    #[test]
    fn product_engine_runs_two_real_pls_models_on_exact_shared_folds() {
        let (dataset, established, alternative) = fixture();
        let config = PlsModelComparisonConfigV1 {
            folds: 4,
            repeats: 2,
            seed: 7_301,
            confidence_level: 0.95,
        };
        let result = compare_pls_models_v1(&dataset, &established, &alternative, config).unwrap();
        let repeated = compare_pls_models_v1(&dataset, &established, &alternative, config).unwrap();
        assert_eq!(result, repeated);
        assert_eq!(result.surface, "internal_labs");
        assert!(!result.qualified);
        assert_ne!(
            result.established.scientific_model_digest,
            result.alternative.scientific_model_digest
        );
        assert_eq!(result.fold_plan.ledger.len(), 48 * 2);
        assert_eq!(result.completed_fold_pairs, 8);
        assert_eq!(result.failed_fold_pairs, 0);
        assert_eq!(result.fold_losses.len(), 8);
        assert_eq!(result.case_losses.len(), 48);
        assert!(result.case_losses.iter().all(|row| row.repeats == 2));
        assert_eq!(result.indicator_predictions.len(), 2);
        assert!(result.indicator_predictions.iter().all(|row| {
            row.established.observations == 96 && row.alternative.observations == 96
        }));
        assert_eq!(result.cvpat.observations, 48);
        assert_eq!(result.cvpat.lower_loss_model, "alternative");
        assert_eq!(result.bic.len(), 1);
        assert_eq!(result.bic[0].construct, "y");
        assert_eq!(result.bic[0].established_parameter_count, 2);
        assert_eq!(result.bic[0].alternative_parameter_count, 3);
        assert_close(
            result.bic[0].established_akaike_weight + result.bic[0].alternative_akaike_weight,
            1.0,
            1e-15,
        );
    }

    #[test]
    fn product_engine_cancels_inside_a_real_model_fit() {
        let (dataset, established, alternative) = fixture();
        let result = compare_pls_models_v1_with_control(
            &dataset,
            &established,
            &alternative,
            PlsModelComparisonConfigV1 {
                folds: 4,
                repeats: 1,
                seed: 1,
                confidence_level: 0.95,
            },
            |progress| progress.phase != PlsModelComparisonPhaseV1::EstimatingAlternative,
        );
        assert_eq!(result.unwrap_err(), PlsModelComparisonErrorV1::Cancelled);
    }

    #[test]
    fn product_engine_rejects_same_scientific_model_despite_renaming() {
        let (dataset, established, mut alternative) = fixture();
        alternative.model = established.model.clone();
        alternative.model.id = Uuid::new_v4();
        alternative.model.name = "Renamed only".into();
        let result = compare_pls_models_v1(
            &dataset,
            &established,
            &alternative,
            PlsModelComparisonConfigV1 {
                folds: 4,
                repeats: 1,
                seed: 1,
                confidence_level: 0.95,
            },
        );
        assert_eq!(
            result.unwrap_err(),
            PlsModelComparisonErrorV1::SameScientificModel
        );
    }
}
