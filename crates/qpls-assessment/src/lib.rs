use arrow::{
    array::{Array, ArrayRef, Float64Array, Int64Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use faer::{Mat, prelude::*};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, Construct, HigherOrderMethod, MeasurementMode, WeightingScheme,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    CBSEM_ML_METHOD_VERSION, CCA_METHOD_VERSION, CFA_ML_METHOD_VERSION, CTA_PLS_METHOD_VERSION,
    EstimationError, EstimationPhase, EstimationProgress,
    GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION, IPMA_METHOD_VERSION,
    MODERATED_MEDIATION_METHOD_VERSION, NONLINEAR_EFFECTS_METHOD_VERSION, PLS_METHOD_VERSION,
    PLS_MGA_METHOD_VERSION, PLS_PREDICT_METHOD_VERSION, PLSC_METHOD_VERSION, PlsResult,
    WPLS_METHOD_VERSION, estimate_pls_with_control,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};
use thiserror::Error;

pub const ASSESSMENT_METHOD_VERSION_V1: &str = "pls_assessment_v1";
pub const ASSESSMENT_METHOD_VERSION_V2: &str = "pls_assessment_v2";
pub const ASSESSMENT_METHOD_VERSION_V3: &str = "pls_assessment_v3";
pub const ASSESSMENT_METHOD_VERSION_V4: &str = "pls_assessment_v4";
pub const ASSESSMENT_METHOD_VERSION_V5: &str = "pls_assessment_v5";
pub const ASSESSMENT_METHOD_VERSION_V6: &str = "pls_assessment_v6";
pub const ASSESSMENT_METHOD_VERSION: &str = "pls_assessment_v7";
pub const RHO_A_METHOD_VERSION: &str = "dijkstra_henseler_rho_a_v1";
pub const HTMT_PLUS_METHOD_VERSION: &str = "ringle_et_al_htmt_plus_v1";
pub const HTMT_ORIGINAL_METHOD_VERSION: &str = "henseler_et_al_htmt_v1";
const NESTED_PROGRESS_SCALE: u64 = 1_000_000;

#[derive(Debug, Error, PartialEq)]
pub enum AssessmentError {
    #[error("assessment was cancelled")]
    Cancelled,
    #[error("PLS assessment requires raw observations")]
    RawDataRequired,
    #[error("dataset fingerprint does not match the analysis recipe")]
    DatasetMismatch,
    #[error("assessment requires a converged PLS-PM v1 result")]
    InvalidEstimationResult,
    #[error("unknown or nonnumeric indicator: {0}")]
    InvalidIndicator(String),
    #[error("estimation result is inconsistent with the recipe: {0}")]
    ResultMismatch(String),
    #[error("assessment numerical failure: {0}")]
    Numerical(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentPhase {
    PreparingRows,
    PreparingIndicators,
    ConstructQuality,
    CrossLoadings,
    FornellLarcker,
    Htmt,
    StructuralQuality,
    StructuralVif,
    FormativeVif,
    EffectSize,
    ModelFit,
    Blindfolding,
}

impl AssessmentPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreparingRows => "assessment_preparing_rows",
            Self::PreparingIndicators => "assessment_preparing_indicators",
            Self::ConstructQuality => "assessment_construct_quality",
            Self::CrossLoadings => "assessment_cross_loadings",
            Self::FornellLarcker => "assessment_fornell_larcker",
            Self::Htmt => "assessment_htmt",
            Self::StructuralQuality => "assessment_structural_quality",
            Self::StructuralVif => "assessment_structural_vif",
            Self::FormativeVif => "assessment_formative_vif",
            Self::EffectSize => "assessment_effect_size",
            Self::ModelFit => "assessment_model_fit",
            Self::Blindfolding => "assessment_blindfolding",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentProgress {
    pub phase: AssessmentPhase,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstructQuality {
    pub construct: String,
    pub cronbach_alpha: Option<f64>,
    pub rho_c: Option<f64>,
    pub ave: Option<f64>,
    #[serde(default)]
    pub rho_a: Option<f64>,
    #[serde(default)]
    pub rho_a_status: Option<RhoAStatus>,
    #[serde(default)]
    pub rho_a_reason: Option<String>,
    #[serde(default)]
    pub rho_a_warning_codes: Vec<String>,
    #[serde(default)]
    pub rho_a_indicator_count: Option<usize>,
    #[serde(default)]
    pub score_variance_before_normalization: Option<f64>,
    #[serde(default)]
    pub normalized_weight_norm_squared: Option<f64>,
    #[serde(default)]
    pub off_diagonal_numerator: Option<f64>,
    #[serde(default)]
    pub off_diagonal_denominator: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RhoAStatus {
    Available,
    NotApplicable,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossLoading {
    pub indicator: String,
    pub assigned_construct: String,
    pub construct: String,
    pub loading: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FornellLarckerMatrix {
    pub constructs: Vec<String>,
    /// Row-major values. Reflective diagonal cells contain sqrt(AVE), while
    /// formative diagonal cells are not applicable.
    pub values: Vec<Vec<Option<f64>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HtmtMatrix {
    pub constructs: Vec<String>,
    /// Symmetric row-major matrix. Reflective diagonals are one. Cells are not
    /// applicable when either block is formative or has fewer than two items.
    pub values: Vec<Vec<Option<f64>>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HtmtStatus {
    Available,
    NotApplicable,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HtmtCell {
    pub value: Option<f64>,
    pub status: HtmtStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HtmtAssessment {
    pub constructs: Vec<String>,
    pub correlation_type: String,
    pub absolute_correlations: bool,
    pub cells: Vec<Vec<HtmtCell>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralQuality {
    pub construct: String,
    pub predictor_count: usize,
    pub r_squared: f64,
    pub adjusted_r_squared: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralVif {
    pub target_construct: String,
    pub predictor_construct: String,
    pub vif: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormativeIndicatorVif {
    pub construct: String,
    pub indicator: String,
    pub vif: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralEffectSize {
    pub source_construct: String,
    pub target_construct: String,
    pub included_r_squared: f64,
    pub excluded_r_squared: Option<f64>,
    pub f_squared: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitMeasures {
    pub srmr: f64,
    pub d_uls: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsModelFit {
    pub saturated: FitMeasures,
    pub estimated: FitMeasures,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindfoldingSettings {
    pub omission_distance: usize,
    pub selection: String,
    pub missing_value_treatment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossValidatedRedundancy {
    pub construct: String,
    pub q_squared: Option<f64>,
    pub prediction_error_sum_squares: Option<f64>,
    pub observation_sum_squares: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlindfoldingResult {
    pub settings: BlindfoldingSettings,
    pub constructs: Vec<CrossValidatedRedundancy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssessmentResult {
    pub method_version: String,
    #[serde(default)]
    pub rho_a_method_version: Option<String>,
    pub construct_quality: Vec<ConstructQuality>,
    pub cross_loadings: Vec<CrossLoading>,
    pub fornell_larcker: FornellLarckerMatrix,
    #[serde(default)]
    pub htmt: Option<HtmtMatrix>,
    #[serde(default)]
    pub htmt_plus_method_version: Option<String>,
    #[serde(default)]
    pub htmt_plus: Option<HtmtAssessment>,
    #[serde(default)]
    pub htmt_original_method_version: Option<String>,
    #[serde(default)]
    pub htmt_original: Option<HtmtAssessment>,
    pub r_squared: BTreeMap<String, f64>,
    #[serde(default)]
    pub structural_quality: Vec<StructuralQuality>,
    #[serde(default)]
    pub structural_vif: Vec<StructuralVif>,
    #[serde(default)]
    pub formative_indicator_vif: Vec<FormativeIndicatorVif>,
    #[serde(default)]
    pub f_squared: Vec<StructuralEffectSize>,
    #[serde(default)]
    pub model_fit: Option<PlsModelFit>,
    #[serde(default)]
    pub blindfolding: Option<BlindfoldingResult>,
    pub warnings: Vec<String>,
}

/// Computes deterministic PLS measurement and structural assessment tables.
/// The input result remains the authority for scores, loadings, complete-case
/// count, and structural R-squared values.
pub fn assess_pls(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
) -> Result<AssessmentResult, AssessmentError> {
    assess_pls_with_control(dataset, recipe, estimation, |_| true)
}

pub fn assess_pls_with_control(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
    mut control: impl FnMut(AssessmentProgress) -> bool,
) -> Result<AssessmentResult, AssessmentError> {
    let execution_recipe = expand_higher_order_for_assessment(recipe)?;
    validate_inputs(dataset, &execution_recipe, estimation)?;
    let mut complete_columns = complete_case_columns(dataset, recipe, &mut control)?;
    add_two_stage_higher_order_columns(&mut complete_columns, recipe, estimation)?;
    let recipe = &execution_recipe;
    if complete_columns.values().next().map(Vec::len) != Some(estimation.used_observations) {
        return Err(AssessmentError::ResultMismatch(
            "complete-case observation count differs from estimator output".into(),
        ));
    }

    let loading_by_construct_indicator = estimation
        .outer_estimates
        .iter()
        .map(|estimate| {
            (
                (estimate.construct.as_str(), estimate.indicator.as_str()),
                estimate.loading,
            )
        })
        .collect::<HashMap<_, _>>();
    let weight_by_construct_indicator = estimation
        .outer_estimates
        .iter()
        .map(|estimate| {
            (
                (estimate.construct.as_str(), estimate.indicator.as_str()),
                estimate.weight,
            )
        })
        .collect::<HashMap<_, _>>();
    let transform_by_indicator = estimation
        .transforms
        .iter()
        .map(|transform| (transform.indicator.as_str(), transform.scale))
        .collect::<HashMap<_, _>>();
    let mut warnings = Vec::new();
    let mut quality = Vec::with_capacity(recipe.model.constructs.len());
    for (construct_index, construct) in recipe.model.constructs.iter().enumerate() {
        checkpoint(
            &mut control,
            AssessmentPhase::ConstructQuality,
            construct_index as u64,
            recipe.model.constructs.len() as u64,
        )?;
        if construct.mode == MeasurementMode::Formative {
            quality.push(ConstructQuality {
                construct: construct.id.clone(),
                cronbach_alpha: None,
                rho_c: None,
                ave: None,
                rho_a: None,
                rho_a_status: Some(RhoAStatus::NotApplicable),
                rho_a_reason: Some("rho_a.formative_not_applicable".into()),
                rho_a_warning_codes: Vec::new(),
                rho_a_indicator_count: Some(construct.indicators.len()),
                score_variance_before_normalization: None,
                normalized_weight_norm_squared: None,
                off_diagonal_numerator: None,
                off_diagonal_denominator: None,
            });
            warnings.push(format!(
                "Reliability and AVE are not applicable to formative construct '{}'",
                construct.id
            ));
            continue;
        }
        let loadings = construct
            .indicators
            .iter()
            .map(|indicator| {
                loading_by_construct_indicator
                    .get(&(construct.id.as_str(), indicator.as_str()))
                    .copied()
                    .ok_or_else(|| {
                        AssessmentError::ResultMismatch(format!(
                            "missing outer loading for '{}:{indicator}'",
                            construct.id
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let columns = construct
            .indicators
            .iter()
            .map(|indicator| complete_columns[indicator].as_slice())
            .collect::<Vec<_>>();
        let local_weight_by_indicator = construct
            .indicators
            .iter()
            .filter_map(|indicator| {
                weight_by_construct_indicator
                    .get(&(construct.id.as_str(), indicator.as_str()))
                    .map(|value| (indicator.as_str(), *value))
            })
            .collect::<HashMap<_, _>>();
        let local_loading_by_indicator = construct
            .indicators
            .iter()
            .filter_map(|indicator| {
                loading_by_construct_indicator
                    .get(&(construct.id.as_str(), indicator.as_str()))
                    .map(|value| (indicator.as_str(), *value))
            })
            .collect::<HashMap<_, _>>();
        let rho_a = calculate_rho_a(
            construct,
            &columns,
            recipe.settings.weighting_scheme.clone(),
            &local_weight_by_indicator,
            &transform_by_indicator,
            &local_loading_by_indicator,
        );
        for code in &rho_a.warning_codes {
            warnings.push(format!("{} for construct '{}'", code, construct.id));
        }
        if let Some(reason) = &rho_a.reason {
            warnings.push(format!("{} for construct '{}'", reason, construct.id));
        }
        let alpha = if columns.len() < 2 {
            warnings.push(format!(
                "Cronbach alpha is undefined for single-item construct '{}'",
                construct.id
            ));
            None
        } else {
            Some(standardized_cronbach_alpha(&columns)?)
        };
        let ave = loadings
            .iter()
            .map(|loading| loading * loading)
            .sum::<f64>()
            / loadings.len() as f64;
        let loading_sum = loadings.iter().sum::<f64>();
        let error_sum = loadings
            .iter()
            .map(|loading| 1.0 - loading * loading)
            .sum::<f64>();
        let denominator = loading_sum * loading_sum + error_sum;
        if denominator <= f64::EPSILON || !denominator.is_finite() || !ave.is_finite() {
            return Err(AssessmentError::Numerical(format!(
                "invalid reliability denominator for construct '{}'",
                construct.id
            )));
        }
        quality.push(ConstructQuality {
            construct: construct.id.clone(),
            cronbach_alpha: alpha,
            rho_c: Some(loading_sum * loading_sum / denominator),
            ave: Some(ave),
            rho_a: rho_a.value,
            rho_a_status: Some(rho_a.status),
            rho_a_reason: rho_a.reason,
            rho_a_warning_codes: rho_a.warning_codes,
            rho_a_indicator_count: Some(construct.indicators.len()),
            score_variance_before_normalization: rho_a.score_variance,
            normalized_weight_norm_squared: rho_a.weight_norm_squared,
            off_diagonal_numerator: rho_a.numerator,
            off_diagonal_denominator: rho_a.denominator,
        });
    }
    checkpoint(
        &mut control,
        AssessmentPhase::ConstructQuality,
        recipe.model.constructs.len() as u64,
        recipe.model.constructs.len() as u64,
    )?;

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.clone())
        .collect::<Vec<_>>();
    let mut cross_loadings = Vec::new();
    let cross_loading_total = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len())
        .sum::<usize>()
        * construct_ids.len();
    let mut cross_loading_index = 0;
    for assigned in &recipe.model.constructs {
        for indicator in &assigned.indicators {
            for construct in &construct_ids {
                checkpoint(
                    &mut control,
                    AssessmentPhase::CrossLoadings,
                    cross_loading_index as u64,
                    cross_loading_total as u64,
                )?;
                cross_loadings.push(CrossLoading {
                    indicator: indicator.clone(),
                    assigned_construct: assigned.id.clone(),
                    construct: construct.clone(),
                    loading: correlation(
                        &complete_columns[indicator],
                        estimation.construct_scores.get(construct).ok_or_else(|| {
                            AssessmentError::ResultMismatch(format!(
                                "missing construct score for '{construct}'"
                            ))
                        })?,
                    )?,
                });
                cross_loading_index += 1;
            }
        }
    }
    checkpoint(
        &mut control,
        AssessmentPhase::CrossLoadings,
        cross_loading_total as u64,
        cross_loading_total as u64,
    )?;

    let ave_by_construct = quality
        .iter()
        .map(|row| (row.construct.as_str(), row.ave))
        .collect::<HashMap<_, _>>();
    let mut matrix = vec![vec![None; construct_ids.len()]; construct_ids.len()];
    let matrix_units = construct_ids.len() * construct_ids.len();
    for row in 0..construct_ids.len() {
        for column in 0..construct_ids.len() {
            checkpoint(
                &mut control,
                AssessmentPhase::FornellLarcker,
                (row * construct_ids.len() + column) as u64,
                matrix_units as u64,
            )?;
            matrix[row][column] = if row == column {
                ave_by_construct[construct_ids[row].as_str()].map(f64::sqrt)
            } else {
                Some(correlation(
                    &estimation.construct_scores[&construct_ids[row]],
                    &estimation.construct_scores[&construct_ids[column]],
                )?)
            };
        }
    }
    checkpoint(
        &mut control,
        AssessmentPhase::FornellLarcker,
        matrix_units as u64,
        matrix_units as u64,
    )?;

    let empty_htmt_cell = HtmtCell {
        value: None,
        status: HtmtStatus::Unavailable,
        reason: Some("htmt.uninitialized".into()),
    };
    let mut htmt_plus =
        vec![vec![empty_htmt_cell.clone(); construct_ids.len()]; construct_ids.len()];
    let mut htmt_original = vec![vec![empty_htmt_cell; construct_ids.len()]; construct_ids.len()];
    let htmt_units = construct_ids.len() * construct_ids.len();
    for row in 0..construct_ids.len() {
        for column in 0..construct_ids.len() {
            checkpoint(
                &mut control,
                AssessmentPhase::Htmt,
                (row * construct_ids.len() + column) as u64,
                htmt_units as u64,
            )?;
            let left = &recipe.model.constructs[row];
            let right = &recipe.model.constructs[column];
            htmt_plus[row][column] =
                htmt_cell(left, right, &complete_columns, row == column, true)?;
            htmt_original[row][column] =
                htmt_cell(left, right, &complete_columns, row == column, false)?;
            if row < column {
                for (label, cell) in [
                    ("HTMT+", &htmt_plus[row][column]),
                    ("original HTMT", &htmt_original[row][column]),
                ] {
                    if cell.status == HtmtStatus::Unavailable {
                        warnings.push(format!(
                            "{} is unavailable for '{}' and '{}': {}",
                            label,
                            left.id,
                            right.id,
                            cell.reason.as_deref().unwrap_or("unknown reason")
                        ));
                    }
                }
            }
        }
    }
    checkpoint(
        &mut control,
        AssessmentPhase::Htmt,
        htmt_units as u64,
        htmt_units as u64,
    )?;

    let endogenous = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| estimation.r_squared.contains_key(&construct.id))
        .collect::<Vec<_>>();
    let mut structural_quality = Vec::with_capacity(endogenous.len());
    for (index, construct) in endogenous.iter().enumerate() {
        checkpoint(
            &mut control,
            AssessmentPhase::StructuralQuality,
            index as u64,
            endogenous.len() as u64,
        )?;
        let predictor_count = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .count();
        let r_squared = estimation.r_squared[&construct.id];
        let adjusted_r_squared = if estimation.used_observations > predictor_count + 1 {
            Some(
                1.0 - (1.0 - r_squared) * (estimation.used_observations - 1) as f64
                    / (estimation.used_observations - predictor_count - 1) as f64,
            )
        } else {
            warnings.push(format!(
                "Adjusted R-squared is undefined for '{}' because n <= predictors + 1",
                construct.id
            ));
            None
        };
        structural_quality.push(StructuralQuality {
            construct: construct.id.clone(),
            predictor_count,
            r_squared,
            adjusted_r_squared,
        });
    }
    checkpoint(
        &mut control,
        AssessmentPhase::StructuralQuality,
        endogenous.len() as u64,
        endogenous.len() as u64,
    )?;

    let structural_vif_units = endogenous
        .iter()
        .map(|target| {
            recipe
                .model
                .paths
                .iter()
                .filter(|path| path.target == target.id)
                .count()
        })
        .sum::<usize>();
    let mut structural_vif = Vec::with_capacity(structural_vif_units);
    for target in &endogenous {
        let predictors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target.id)
            .map(|path| path.source.as_str())
            .collect::<Vec<_>>();
        for predictor in &predictors {
            let item_index = structural_vif.len() as u64;
            let remaining = predictors
                .iter()
                .filter(|candidate| *candidate != predictor)
                .map(|candidate| estimation.construct_scores[*candidate].as_slice())
                .collect::<Vec<_>>();
            let value = variance_inflation_factor_with_control(
                &estimation.construct_scores[*predictor],
                &remaining,
                |completed_units, total_units| {
                    control(nested_progress(
                        AssessmentPhase::StructuralVif,
                        item_index,
                        structural_vif_units as u64,
                        completed_units,
                        total_units,
                    ))
                },
            )?;
            if value.is_none() {
                warnings.push(format!(
                    "Inner VIF is undefined for '{}' predicting '{}' because it is perfectly explained by the other predictors",
                    predictor, target.id
                ));
            }
            structural_vif.push(StructuralVif {
                target_construct: target.id.clone(),
                predictor_construct: (*predictor).to_owned(),
                vif: value,
            });
        }
    }
    checkpoint(
        &mut control,
        AssessmentPhase::StructuralVif,
        structural_vif_units as u64 * NESTED_PROGRESS_SCALE,
        structural_vif_units as u64 * NESTED_PROGRESS_SCALE,
    )?;

    let formative_units = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| construct.mode == MeasurementMode::Formative)
        .map(|construct| construct.indicators.len())
        .sum::<usize>();
    let mut formative_indicator_vif = Vec::with_capacity(formative_units);
    for construct in recipe
        .model
        .constructs
        .iter()
        .filter(|construct| construct.mode == MeasurementMode::Formative)
    {
        for indicator in &construct.indicators {
            let item_index = formative_indicator_vif.len() as u64;
            let remaining = construct
                .indicators
                .iter()
                .filter(|candidate| *candidate != indicator)
                .map(|candidate| complete_columns[candidate].as_slice())
                .collect::<Vec<_>>();
            let value = variance_inflation_factor_with_control(
                &complete_columns[indicator],
                &remaining,
                |completed_units, total_units| {
                    control(nested_progress(
                        AssessmentPhase::FormativeVif,
                        item_index,
                        formative_units as u64,
                        completed_units,
                        total_units,
                    ))
                },
            )?;
            if value.is_none() {
                warnings.push(format!(
                    "Formative indicator VIF is undefined for '{}' in '{}' because it is perfectly explained by the other indicators",
                    indicator, construct.id
                ));
            }
            formative_indicator_vif.push(FormativeIndicatorVif {
                construct: construct.id.clone(),
                indicator: indicator.clone(),
                vif: value,
            });
        }
    }
    checkpoint(
        &mut control,
        AssessmentPhase::FormativeVif,
        formative_units as u64 * NESTED_PROGRESS_SCALE,
        formative_units as u64 * NESTED_PROGRESS_SCALE,
    )?;

    let mut seen_paths = HashSet::new();
    let unique_paths = recipe
        .model
        .paths
        .iter()
        .filter(|path| seen_paths.insert((path.source.clone(), path.target.clone())))
        .collect::<Vec<_>>();
    let mut f_squared = Vec::with_capacity(unique_paths.len());
    for (path_index, path) in unique_paths.iter().enumerate() {
        checkpoint(
            &mut control,
            AssessmentPhase::EffectSize,
            path_index as u64 * NESTED_PROGRESS_SCALE,
            unique_paths.len() as u64 * NESTED_PROGRESS_SCALE,
        )?;
        let included_r_squared =
            estimation
                .r_squared
                .get(&path.target)
                .copied()
                .ok_or_else(|| {
                    AssessmentError::ResultMismatch(format!(
                        "missing included R-squared for structural target '{}'",
                        path.target
                    ))
                })?;
        let path_start = path_index as u64 * NESTED_PROGRESS_SCALE;
        let path_end = path_start + NESTED_PROGRESS_SCALE;
        let remaining_predictors = recipe
            .model
            .paths
            .iter()
            .filter(|candidate| candidate.target == path.target && candidate.source != path.source)
            .map(|candidate| estimation.construct_scores[candidate.source.as_str()].as_slice())
            .collect::<Vec<_>>();
        if !control(AssessmentProgress {
            phase: AssessmentPhase::EffectSize,
            completed_units: path_start,
            total_units: unique_paths.len() as u64 * NESTED_PROGRESS_SCALE,
        }) {
            return Err(AssessmentError::Cancelled);
        }
        let reduced_r_squared = fixed_score_structural_r_squared(
            &estimation.construct_scores[path.target.as_str()],
            &remaining_predictors,
            &path.target,
        );
        let (excluded_r_squared, effect_size) = match reduced_r_squared {
            Ok(excluded) => {
                let denominator = 1.0 - included_r_squared;
                if denominator <= 1e-12 {
                    warnings.push(format!(
                        "Cohen f-squared is undefined for '{} -> {}' because included R-squared is one within numerical tolerance",
                        path.source, path.target
                    ));
                    (Some(excluded), None)
                } else {
                    let value = (included_r_squared - excluded) / denominator;
                    if value.is_finite() {
                        (Some(excluded), Some(value))
                    } else {
                        warnings.push(format!(
                            "Cohen f-squared is undefined for '{} -> {}' because the comparison produced a non-finite value",
                            path.source, path.target
                        ));
                        (Some(excluded), None)
                    }
                }
            }
            Err(error) => {
                warnings.push(format!(
                    "Cohen f-squared is unavailable for '{} -> {}' because fixed-score reduced structural regression failed: {}",
                    path.source, path.target, error
                ));
                (None, None)
            }
        };
        f_squared.push(StructuralEffectSize {
            source_construct: path.source.clone(),
            target_construct: path.target.clone(),
            included_r_squared,
            excluded_r_squared,
            f_squared: effect_size,
        });
        checkpoint(
            &mut control,
            AssessmentPhase::EffectSize,
            path_end,
            unique_paths.len() as u64 * NESTED_PROGRESS_SCALE,
        )?;
    }

    let model_fit = Some(calculate_model_fit(
        recipe,
        estimation,
        &complete_columns,
        &mut control,
    )?);
    let blindfolding = calculate_blindfolding(
        dataset,
        recipe,
        estimation,
        &complete_columns,
        &mut warnings,
        &mut control,
    )?;

    Ok(AssessmentResult {
        method_version: ASSESSMENT_METHOD_VERSION.into(),
        rho_a_method_version: Some(RHO_A_METHOD_VERSION.into()),
        construct_quality: quality,
        cross_loadings,
        fornell_larcker: FornellLarckerMatrix {
            constructs: construct_ids,
            values: matrix,
        },
        htmt: None,
        htmt_plus_method_version: Some(HTMT_PLUS_METHOD_VERSION.into()),
        htmt_plus: Some(HtmtAssessment {
            constructs: recipe
                .model
                .constructs
                .iter()
                .map(|construct| construct.id.clone())
                .collect(),
            correlation_type: "pearson".into(),
            absolute_correlations: true,
            cells: htmt_plus,
        }),
        htmt_original_method_version: Some(HTMT_ORIGINAL_METHOD_VERSION.into()),
        htmt_original: Some(HtmtAssessment {
            constructs: recipe
                .model
                .constructs
                .iter()
                .map(|construct| construct.id.clone())
                .collect(),
            correlation_type: "pearson".into(),
            absolute_correlations: false,
            cells: htmt_original,
        }),
        r_squared: estimation.r_squared.clone(),
        structural_quality,
        structural_vif,
        formative_indicator_vif,
        f_squared,
        model_fit,
        blindfolding,
        warnings,
    })
}

fn calculate_model_fit(
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
    columns: &BTreeMap<String, Vec<f64>>,
    control: &mut impl FnMut(AssessmentProgress) -> bool,
) -> Result<PlsModelFit, AssessmentError> {
    let indicators = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct, indicator))
        })
        .collect::<Vec<_>>();
    let indicator_count = indicators.len();
    let mut observed = vec![vec![0.0; indicator_count]; indicator_count];
    for row in 0..indicator_count {
        observed[row][row] = 1.0;
        for column in (row + 1)..indicator_count {
            checkpoint(
                control,
                AssessmentPhase::ModelFit,
                (row * indicator_count + column) as u64,
                (indicator_count * indicator_count * 3) as u64,
            )?;
            let value = correlation(&columns[indicators[row].1], &columns[indicators[column].1])?;
            observed[row][column] = value;
            observed[column][row] = value;
        }
    }
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<Vec<_>>();
    let construct_count = construct_ids.len();
    let construct_index = construct_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<HashMap<_, _>>();
    let mut empirical_construct = vec![vec![0.0; construct_count]; construct_count];
    for row in 0..construct_count {
        empirical_construct[row][row] = 1.0;
        for column in (row + 1)..construct_count {
            let value = correlation(
                &estimation.construct_scores[construct_ids[row]],
                &estimation.construct_scores[construct_ids[column]],
            )?;
            empirical_construct[row][column] = value;
            empirical_construct[column][row] = value;
        }
    }
    let estimated_construct =
        implied_construct_correlations(recipe, estimation, &empirical_construct, &construct_index)?;
    let loadings = estimation
        .outer_estimates
        .iter()
        .map(|estimate| {
            (
                (estimate.construct.as_str(), estimate.indicator.as_str()),
                estimate.loading,
            )
        })
        .collect::<HashMap<_, _>>();
    let saturated = implied_indicator_correlations(
        &indicators,
        &observed,
        &empirical_construct,
        &construct_index,
        &loadings,
    );
    let estimated = implied_indicator_correlations(
        &indicators,
        &observed,
        &estimated_construct,
        &construct_index,
        &loadings,
    );
    checkpoint(
        control,
        AssessmentPhase::ModelFit,
        (indicator_count * indicator_count * 3) as u64,
        (indicator_count * indicator_count * 3) as u64,
    )?;
    Ok(PlsModelFit {
        saturated: fit_measures(&observed, &saturated),
        estimated: fit_measures(&observed, &estimated),
    })
}

fn implied_construct_correlations(
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
    empirical: &[Vec<f64>],
    index: &HashMap<&str, usize>,
) -> Result<Vec<Vec<f64>>, AssessmentError> {
    let count = recipe.model.constructs.len();
    let mut indegree = vec![0usize; count];
    let mut successors = vec![Vec::new(); count];
    for path in &recipe.model.paths {
        let source = index[path.source.as_str()];
        let target = index[path.target.as_str()];
        indegree[target] += 1;
        successors[source].push(target);
    }
    let mut queue = indegree
        .iter()
        .enumerate()
        .filter(|(_, degree)| **degree == 0)
        .map(|(index, _)| index)
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(count);
    while let Some(current) = queue.pop_front() {
        order.push(current);
        for successor in &successors[current] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                queue.push_back(*successor);
            }
        }
    }
    if order.len() != count {
        return Err(AssessmentError::ResultMismatch(
            "model fit requires an acyclic structural model".into(),
        ));
    }
    let coefficients = estimation
        .paths
        .iter()
        .map(|path| {
            (
                (path.source.as_str(), path.target.as_str()),
                path.coefficient,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut implied = vec![vec![0.0; count]; count];
    for index in 0..count {
        implied[index][index] = 1.0;
    }
    let exogenous = recipe
        .model
        .constructs
        .iter()
        .enumerate()
        .filter(|(_, construct)| {
            !recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    for (position, left) in exogenous.iter().enumerate() {
        for right in exogenous.iter().skip(position + 1) {
            implied[*left][*right] = empirical[*left][*right];
            implied[*right][*left] = empirical[*left][*right];
        }
    }
    let mut processed: Vec<usize> = Vec::new();
    for target in order {
        let target_id = recipe.model.constructs[target].id.as_str();
        let predecessors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == target_id)
            .collect::<Vec<_>>();
        if !predecessors.is_empty() {
            for other in &processed {
                let value = predecessors
                    .iter()
                    .map(|path| {
                        coefficients[&(path.source.as_str(), target_id)]
                            * implied[index[path.source.as_str()]][*other]
                    })
                    .sum::<f64>();
                implied[target][*other] = value;
                implied[*other][target] = value;
            }
        }
        processed.push(target);
    }
    Ok(implied)
}

fn implied_indicator_correlations(
    indicators: &[(&qpls_core::Construct, &String)],
    observed: &[Vec<f64>],
    construct_correlations: &[Vec<f64>],
    construct_index: &HashMap<&str, usize>,
    loadings: &HashMap<(&str, &str), f64>,
) -> Vec<Vec<f64>> {
    let count = indicators.len();
    let mut implied = vec![vec![0.0; count]; count];
    for row in 0..count {
        implied[row][row] = 1.0;
        for column in (row + 1)..count {
            let (left_construct, left_indicator) = indicators[row];
            let (right_construct, right_indicator) = indicators[column];
            let value = if left_construct.id == right_construct.id
                && left_construct.mode == MeasurementMode::Formative
            {
                observed[row][column]
            } else {
                loadings[&(left_construct.id.as_str(), left_indicator.as_str())]
                    * construct_correlations[construct_index[left_construct.id.as_str()]]
                        [construct_index[right_construct.id.as_str()]]
                    * loadings[&(right_construct.id.as_str(), right_indicator.as_str())]
            };
            implied[row][column] = value;
            implied[column][row] = value;
        }
    }
    implied
}

fn fit_measures(observed: &[Vec<f64>], implied: &[Vec<f64>]) -> FitMeasures {
    let count = observed.len();
    let mut d_uls = 0.0;
    for row in 0..count {
        for column in 0..=row {
            d_uls += (observed[row][column] - implied[row][column]).powi(2);
        }
    }
    FitMeasures {
        srmr: (d_uls / (count * (count + 1) / 2) as f64).sqrt(),
        d_uls,
    }
}

fn calculate_blindfolding(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
    complete_columns: &BTreeMap<String, Vec<f64>>,
    warnings: &mut Vec<String>,
    control: &mut impl FnMut(AssessmentProgress) -> bool,
) -> Result<Option<BlindfoldingResult>, AssessmentError> {
    let observation_count = estimation.used_observations;
    let Some(distance) = [7usize, 5, 6, 8, 9, 10, 11, 12]
        .into_iter()
        .find(|distance| *distance < observation_count && observation_count % distance != 0)
    else {
        warnings.push(
            "Blindfolding Q-squared is unavailable because no omission distance from 5 through 12 is smaller than n and non-dividing"
                .into(),
        );
        return Ok(None);
    };
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
        .collect::<Vec<_>>();
    let total_rounds = endogenous.len() * distance;
    let loading_map = estimation
        .outer_estimates
        .iter()
        .map(|estimate| (estimate.indicator.as_str(), estimate.loading))
        .collect::<HashMap<_, _>>();
    let indicator_order = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().cloned())
        .collect::<Vec<_>>();
    let mut blindfold_recipe = recipe.clone();
    if blindfold_recipe.settings.method == AnalysisMethod::Mga {
        blindfold_recipe.settings.method = AnalysisMethod::PlsPm;
        blindfold_recipe.metadata.remove("mga_group_column");
        blindfold_recipe.metadata.remove("mga.group_column");
    }
    let mut rows = Vec::with_capacity(endogenous.len());
    for (construct_index, construct) in endogenous.iter().enumerate() {
        if construct.mode == MeasurementMode::Formative {
            warnings.push(format!(
                "Blindfolding cross-validated redundancy is not applicable to formative construct '{}'",
                construct.id
            ));
            rows.push(CrossValidatedRedundancy {
                construct: construct.id.clone(),
                q_squared: None,
                prediction_error_sum_squares: None,
                observation_sum_squares: None,
            });
            continue;
        }
        let mut prediction_error = 0.0;
        let mut observation_error = 0.0;
        let mut failed = None;
        for round in 0..distance {
            let item_index = construct_index * distance + round;
            checkpoint(
                control,
                AssessmentPhase::Blindfolding,
                item_index as u64 * NESTED_PROGRESS_SCALE,
                total_rounds as u64 * NESTED_PROGRESS_SCALE,
            )?;
            let mut blinded_columns = complete_columns.clone();
            let mut omitted = Vec::new();
            for (indicator_offset, indicator) in construct.indicators.iter().enumerate() {
                let original = &complete_columns[indicator];
                let omitted_rows = (0..observation_count)
                    .filter(|row| (indicator_offset * observation_count + row) % distance == round)
                    .collect::<Vec<_>>();
                let omitted_set = omitted_rows.iter().copied().collect::<HashSet<_>>();
                let retained_mean = original
                    .iter()
                    .enumerate()
                    .filter(|(row, _)| !omitted_set.contains(row))
                    .map(|(_, value)| value)
                    .sum::<f64>()
                    / (observation_count - omitted_rows.len()) as f64;
                for row in omitted_rows {
                    omitted.push((indicator.clone(), row, original[row]));
                    blinded_columns.get_mut(indicator).unwrap()[row] = retained_mean;
                }
            }
            let blinded_dataset = dataset_from_complete_columns(
                dataset,
                &indicator_order,
                &blinded_columns,
                observation_count,
            )?;
            let round_start = item_index as u64 * NESTED_PROGRESS_SCALE;
            let round_end = round_start + NESTED_PROGRESS_SCALE;
            let mut last_nested_progress = round_start;
            let round_estimate =
                estimate_pls_with_control(&blinded_dataset, &blindfold_recipe, |progress| {
                    let mut mapped = estimation_nested_progress(
                        item_index as u64,
                        total_rounds as u64,
                        progress,
                    );
                    mapped.phase = AssessmentPhase::Blindfolding;
                    mapped.completed_units = mapped
                        .completed_units
                        .clamp(last_nested_progress, round_end);
                    last_nested_progress = mapped.completed_units;
                    control(mapped)
                });
            let round_estimate = match round_estimate {
                Ok(result) => result,
                Err(EstimationError::Cancelled) => return Err(AssessmentError::Cancelled),
                Err(error) => {
                    failed = Some(error.to_string());
                    break;
                }
            };
            let predecessors = recipe
                .model
                .paths
                .iter()
                .filter(|path| path.target == construct.id)
                .collect::<Vec<_>>();
            let predicted_score = (0..observation_count)
                .map(|row| {
                    predecessors
                        .iter()
                        .map(|path| {
                            let coefficient = round_estimate
                                .paths
                                .iter()
                                .find(|estimate| {
                                    estimate.source == path.source && estimate.target == path.target
                                })
                                .map(|estimate| estimate.coefficient)
                                .unwrap_or(0.0);
                            coefficient * round_estimate.construct_scores[&path.source][row]
                        })
                        .sum::<f64>()
                })
                .collect::<Vec<_>>();
            let round_loadings = round_estimate
                .outer_estimates
                .iter()
                .map(|estimate| (estimate.indicator.as_str(), estimate.loading))
                .collect::<HashMap<_, _>>();
            for (indicator, row, actual) in omitted {
                let training = &blinded_columns[&indicator];
                let mean = training.iter().sum::<f64>() / observation_count as f64;
                let sd = (training
                    .iter()
                    .map(|value| (value - mean).powi(2))
                    .sum::<f64>()
                    / (observation_count - 1) as f64)
                    .sqrt();
                if sd <= f64::EPSILON {
                    failed = Some(format!(
                        "indicator '{indicator}' has zero blindfold variance"
                    ));
                    break;
                }
                let actual_standardized = (actual - mean) / sd;
                let predicted_standardized = round_loadings
                    .get(indicator.as_str())
                    .copied()
                    .unwrap_or(loading_map[indicator.as_str()])
                    * predicted_score[row];
                prediction_error += (actual_standardized - predicted_standardized).powi(2);
                observation_error += actual_standardized.powi(2);
            }
            if failed.is_some() {
                break;
            }
        }
        if let Some(error) = failed {
            warnings.push(format!(
                "Blindfolding Q-squared is unavailable for '{}' because a round failed: {}",
                construct.id, error
            ));
            rows.push(CrossValidatedRedundancy {
                construct: construct.id.clone(),
                q_squared: None,
                prediction_error_sum_squares: None,
                observation_sum_squares: None,
            });
        } else if observation_error <= f64::EPSILON {
            warnings.push(format!(
                "Blindfolding Q-squared is unavailable for '{}' because the omission benchmark sum of squares is zero",
                construct.id
            ));
            rows.push(CrossValidatedRedundancy {
                construct: construct.id.clone(),
                q_squared: None,
                prediction_error_sum_squares: Some(prediction_error),
                observation_sum_squares: Some(observation_error),
            });
        } else {
            rows.push(CrossValidatedRedundancy {
                construct: construct.id.clone(),
                q_squared: Some(1.0 - prediction_error / observation_error),
                prediction_error_sum_squares: Some(prediction_error),
                observation_sum_squares: Some(observation_error),
            });
        }
    }
    checkpoint(
        control,
        AssessmentPhase::Blindfolding,
        total_rounds as u64 * NESTED_PROGRESS_SCALE,
        total_rounds as u64 * NESTED_PROGRESS_SCALE,
    )?;
    Ok(Some(BlindfoldingResult {
        settings: BlindfoldingSettings {
            omission_distance: distance,
            selection: "preferred_7_then_smallest_valid_5_to_12".into(),
            missing_value_treatment: "indicator_mean_replacement".into(),
        },
        constructs: rows,
    }))
}

fn dataset_from_complete_columns(
    source: &Dataset,
    indicators: &[String],
    columns: &BTreeMap<String, Vec<f64>>,
    observation_count: usize,
) -> Result<Dataset, AssessmentError> {
    let fields = indicators
        .iter()
        .map(|indicator| Field::new(indicator, DataType::Float64, false))
        .collect::<Vec<_>>();
    let arrays = indicators
        .iter()
        .map(|indicator| Arc::new(Float64Array::from(columns[indicator].clone())) as ArrayRef)
        .collect::<Vec<_>>();
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| AssessmentError::Numerical(error.to_string()))?;
    let mut schema = source.schema.clone();
    schema
        .columns
        .retain(|column| indicators.contains(&column.name));
    schema.case_count = observation_count;
    Ok(Dataset {
        id: source.id,
        name: source.name.clone(),
        schema,
        batch,
        fingerprint: source.fingerprint.clone(),
    })
}

pub fn variance_inflation_factor(
    target: &[f64],
    remaining_predictors: &[&[f64]],
) -> Result<Option<f64>, AssessmentError> {
    variance_inflation_factor_with_control(target, remaining_predictors, |_, _| true)
}

/// Computes VIF from the standardized predictor correlation system. The
/// callback receives row-work progress and is checked at least every 1,024
/// observations during every correlation pass, and immediately around the
/// bounded predictor-by-predictor solve.
pub fn variance_inflation_factor_with_control(
    target: &[f64],
    remaining_predictors: &[&[f64]],
    mut control: impl FnMut(u64, u64) -> bool,
) -> Result<Option<f64>, AssessmentError> {
    if remaining_predictors
        .iter()
        .any(|predictor| predictor.len() != target.len())
    {
        return Err(AssessmentError::ResultMismatch(
            "VIF columns have incompatible lengths".into(),
        ));
    }
    if target.len() < 2 {
        return Err(AssessmentError::ResultMismatch(
            "at least two observations are required".into(),
        ));
    }
    if remaining_predictors.is_empty() {
        if !control(0, 1) || !control(1, 1) {
            return Err(AssessmentError::Cancelled);
        }
        return Ok(Some(1.0));
    }

    let predictor_count = remaining_predictors.len();
    let correlation_count = predictor_count + predictor_count * (predictor_count - 1) / 2;
    let observation_units = correlation_count as u64 * target.len() as u64;
    let total_units = observation_units + 1;
    if !control(0, total_units) {
        return Err(AssessmentError::Cancelled);
    }
    let mut completed_units = 0;
    let mut target_correlations = vec![0.0; predictor_count];
    for (index, predictor) in remaining_predictors.iter().enumerate() {
        target_correlations[index] = checked_correlation(
            target,
            predictor,
            completed_units,
            total_units,
            &mut control,
        )?;
        completed_units += target.len() as u64;
    }
    let mut predictor_correlations = vec![vec![0.0; predictor_count]; predictor_count];
    for row in 0..predictor_count {
        predictor_correlations[row][row] = 1.0;
        for column in (row + 1)..predictor_count {
            let value = checked_correlation(
                remaining_predictors[row],
                remaining_predictors[column],
                completed_units,
                total_units,
                &mut control,
            )?;
            completed_units += target.len() as u64;
            predictor_correlations[row][column] = value;
            predictor_correlations[column][row] = value;
        }
    }
    if !control(observation_units, total_units) {
        return Err(AssessmentError::Cancelled);
    }
    let matrix = Mat::from_fn(predictor_count, predictor_count, |row, column| {
        predictor_correlations[row][column]
    });
    let rhs = Mat::from_fn(predictor_count, 1, |row, _| target_correlations[row]);
    let solution = matrix.col_piv_qr().solve_lstsq(&rhs);
    if !control(total_units, total_units) {
        return Err(AssessmentError::Cancelled);
    }
    let r_squared = target_correlations
        .iter()
        .enumerate()
        .map(|(index, correlation)| correlation * solution[(index, 0)])
        .sum::<f64>()
        .max(0.0);
    if !r_squared.is_finite() {
        return Err(AssessmentError::Numerical(
            "VIF auxiliary regression produced a non-finite R-squared".into(),
        ));
    }
    if 1.0 - r_squared <= 1e-12 {
        Ok(None)
    } else {
        Ok(Some(1.0 / (1.0 - r_squared)))
    }
}

fn fixed_score_structural_r_squared(
    target: &[f64],
    predictors: &[&[f64]],
    subject: &str,
) -> Result<f64, AssessmentError> {
    if target.len() < 2 {
        return Err(AssessmentError::ResultMismatch(
            "at least two observations are required".into(),
        ));
    }
    if predictors
        .iter()
        .any(|predictor| predictor.len() != target.len())
    {
        return Err(AssessmentError::ResultMismatch(
            "structural regression columns have incompatible lengths".into(),
        ));
    }
    if predictors.is_empty() {
        return Ok(0.0);
    }
    let rows = target.len();
    let columns = predictors.len();
    if rows < columns {
        return Err(AssessmentError::Numerical(format!(
            "rank-deficient reduced regression for {subject}"
        )));
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
        return Err(AssessmentError::Numerical(format!(
            "rank-deficient reduced regression for {subject}"
        )));
    }
    let rhs = Mat::from_fn(rows, 1, |row, _| target[row]);
    let solution = qr.solve_lstsq(&rhs);
    let fitted = (0..rows)
        .map(|row| {
            (0..columns)
                .map(|column| (predictors[column][row] - centers[column]) * solution[(column, 0)])
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let residual = target
        .iter()
        .zip(fitted)
        .map(|(actual, fit)| (actual - fit).powi(2))
        .sum::<f64>();
    let total = target.iter().map(|value| value * value).sum::<f64>();
    if total <= f64::EPSILON || !total.is_finite() {
        return Err(AssessmentError::Numerical(format!(
            "zero-variance structural target for {subject}"
        )));
    }
    let r_squared = 1.0 - residual / total;
    if !r_squared.is_finite() {
        return Err(AssessmentError::Numerical(format!(
            "non-finite reduced regression R-squared for {subject}"
        )));
    }
    Ok(r_squared.max(0.0))
}

fn vector_mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn checked_correlation(
    left: &[f64],
    right: &[f64],
    completed_before: u64,
    total_units: u64,
    control: &mut impl FnMut(u64, u64) -> bool,
) -> Result<f64, AssessmentError> {
    if left.len() != right.len() || left.len() < 2 {
        return Err(AssessmentError::ResultMismatch(
            "VIF correlation columns have incompatible lengths".into(),
        ));
    }
    let mut mean_left = 0.0;
    let mut mean_right = 0.0;
    let mut sum_squares_left = 0.0;
    let mut sum_squares_right = 0.0;
    let mut co_moment = 0.0;
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        if index % 1024 == 0 && !control(completed_before + index as u64, total_units) {
            return Err(AssessmentError::Cancelled);
        }
        if !left.is_finite() || !right.is_finite() {
            return Err(AssessmentError::Numerical(
                "VIF contains a non-finite value".into(),
            ));
        }
        let count = (index + 1) as f64;
        let left_delta = left - mean_left;
        mean_left += left_delta / count;
        let right_delta = right - mean_right;
        mean_right += right_delta / count;
        sum_squares_left += left_delta * (left - mean_left);
        sum_squares_right += right_delta * (right - mean_right);
        co_moment += left_delta * (right - mean_right);
    }
    if !control(completed_before + left.len() as u64, total_units) {
        return Err(AssessmentError::Cancelled);
    }
    let denominator = (sum_squares_left * sum_squares_right).sqrt();
    if denominator <= f64::EPSILON || !denominator.is_finite() {
        return Err(AssessmentError::Numerical(
            "VIF contains a constant column".into(),
        ));
    }
    Ok(co_moment / denominator)
}

#[cfg(test)]
fn htmt_ratio(
    left: &qpls_core::Construct,
    right: &qpls_core::Construct,
    columns: &BTreeMap<String, Vec<f64>>,
) -> Result<Option<f64>, AssessmentError> {
    let cell = htmt_cell(left, right, columns, false, true)?;
    Ok(cell.value)
}

fn htmt_cell(
    left: &qpls_core::Construct,
    right: &qpls_core::Construct,
    columns: &BTreeMap<String, Vec<f64>>,
    diagonal: bool,
    absolute: bool,
) -> Result<HtmtCell, AssessmentError> {
    let not_applicable =
        if left.mode == MeasurementMode::Formative || right.mode == MeasurementMode::Formative {
            Some("htmt.formative_not_applicable")
        } else if left.indicators.len() < 2 || right.indicators.len() < 2 {
            Some("htmt.single_indicator_not_applicable")
        } else {
            None
        };
    if let Some(reason) = not_applicable {
        return Ok(HtmtCell {
            value: None,
            status: HtmtStatus::NotApplicable,
            reason: Some(reason.into()),
        });
    }
    if diagonal {
        return Ok(HtmtCell {
            value: Some(1.0),
            status: HtmtStatus::Available,
            reason: None,
        });
    }

    let heterotrait = mean_cross_correlations(left, right, columns, absolute)?;
    let left_monotrait = mean_within_correlations(&left.indicators, columns, absolute)?;
    let right_monotrait = mean_within_correlations(&right.indicators, columns, absolute)?;
    let tolerance = 64.0 * f64::EPSILON;
    if left_monotrait <= tolerance || right_monotrait <= tolerance {
        return Ok(HtmtCell {
            value: None,
            status: HtmtStatus::Unavailable,
            reason: Some(if absolute {
                "htmt.zero_monotrait_denominator".into()
            } else {
                "htmt.original_nonpositive_monotrait_mean".into()
            }),
        });
    }
    let value = htmt_value_from_means(heterotrait, left_monotrait, right_monotrait)?;
    let denominator = (left_monotrait * right_monotrait).sqrt();
    if !denominator.is_finite() || !value.is_finite() {
        return Err(AssessmentError::Numerical(
            "HTMT produced a non-finite value".into(),
        ));
    }
    Ok(HtmtCell {
        value: Some(value),
        status: HtmtStatus::Available,
        reason: None,
    })
}

fn htmt_value_from_means(
    heterotrait: f64,
    left_monotrait: f64,
    right_monotrait: f64,
) -> Result<f64, AssessmentError> {
    let denominator = (left_monotrait * right_monotrait).sqrt();
    let value = heterotrait / denominator;
    if !denominator.is_finite() || !value.is_finite() {
        return Err(AssessmentError::Numerical(
            "HTMT produced a non-finite value".into(),
        ));
    }
    Ok(value)
}

fn mean_cross_correlations(
    left: &qpls_core::Construct,
    right: &qpls_core::Construct,
    columns: &BTreeMap<String, Vec<f64>>,
    absolute: bool,
) -> Result<f64, AssessmentError> {
    let mut sum = 0.0;
    let mut count = 0;
    for left_indicator in &left.indicators {
        for right_indicator in &right.indicators {
            let value = canonical_htmt_correlation(correlation(
                &columns[left_indicator],
                &columns[right_indicator],
            )?)?;
            sum += if absolute { value.abs() } else { value };
            count += 1;
        }
    }
    Ok(sum / count as f64)
}

fn mean_within_correlations(
    indicators: &[String],
    columns: &BTreeMap<String, Vec<f64>>,
    absolute: bool,
) -> Result<f64, AssessmentError> {
    let mut sum = 0.0;
    let mut count = 0;
    for left in 0..indicators.len() {
        for right in (left + 1)..indicators.len() {
            let value = canonical_htmt_correlation(correlation(
                &columns[&indicators[left]],
                &columns[&indicators[right]],
            )?)?;
            sum += if absolute { value.abs() } else { value };
            count += 1;
        }
    }
    if count == 0 {
        return Err(AssessmentError::Numerical(
            "HTMT requires at least two indicators per construct".into(),
        ));
    }
    Ok(sum / count as f64)
}

fn canonical_htmt_correlation(value: f64) -> Result<f64, AssessmentError> {
    let tolerance = 64.0 * f64::EPSILON;
    if !value.is_finite() || value.abs() > 1.0 + tolerance {
        return Err(AssessmentError::Numerical(
            "HTMT correlation falls outside [-1, 1]".into(),
        ));
    }
    Ok(value.clamp(-1.0, 1.0))
}

fn checkpoint(
    control: &mut impl FnMut(AssessmentProgress) -> bool,
    phase: AssessmentPhase,
    completed_units: u64,
    total_units: u64,
) -> Result<(), AssessmentError> {
    if control(AssessmentProgress {
        phase,
        completed_units,
        total_units,
    }) {
        Ok(())
    } else {
        Err(AssessmentError::Cancelled)
    }
}

fn nested_progress(
    phase: AssessmentPhase,
    item_index: u64,
    item_count: u64,
    completed_units: u64,
    total_units: u64,
) -> AssessmentProgress {
    let fraction = if total_units == 0 {
        NESTED_PROGRESS_SCALE
    } else {
        ((completed_units.min(total_units) as u128 * NESTED_PROGRESS_SCALE as u128)
            / total_units as u128) as u64
    };
    AssessmentProgress {
        phase,
        completed_units: item_index
            .saturating_mul(NESTED_PROGRESS_SCALE)
            .saturating_add(fraction),
        total_units: item_count.saturating_mul(NESTED_PROGRESS_SCALE),
    }
}

fn estimation_nested_progress(
    item_index: u64,
    item_count: u64,
    progress: EstimationProgress,
) -> AssessmentProgress {
    const PHASE_COUNT: u64 = 6;
    let phase_index = match progress.phase {
        EstimationPhase::Validating => 0,
        EstimationPhase::PreparingRows => 1,
        EstimationPhase::PreparingIndicators => 2,
        EstimationPhase::Iterating => 3,
        EstimationPhase::Assembling => 4,
        EstimationPhase::ComputingEffects => 5,
    };
    let phase_start = phase_index * NESTED_PROGRESS_SCALE / PHASE_COUNT;
    let phase_end = (phase_index + 1) * NESTED_PROGRESS_SCALE / PHASE_COUNT;
    let phase_width = phase_end - phase_start;
    let within_phase = if progress.total_units == 0 {
        phase_width
    } else {
        ((progress.completed_units.min(progress.total_units) as u128 * phase_width as u128)
            / progress.total_units as u128) as u64
    };
    AssessmentProgress {
        phase: AssessmentPhase::EffectSize,
        completed_units: item_index
            .saturating_mul(NESTED_PROGRESS_SCALE)
            .saturating_add(phase_start)
            .saturating_add(within_phase),
        total_units: item_count.saturating_mul(NESTED_PROGRESS_SCALE),
    }
}

fn validate_inputs(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
) -> Result<(), AssessmentError> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(AssessmentError::RawDataRequired);
    }
    if dataset.fingerprint.0 != recipe.dataset_fingerprint {
        return Err(AssessmentError::DatasetMismatch);
    }
    if !estimation.converged
        || !(estimation.method_version == PLS_METHOD_VERSION
            || estimation.method_version == PLSC_METHOD_VERSION
            || estimation.method_version == GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION
            || estimation.method_version == NONLINEAR_EFFECTS_METHOD_VERSION
            || estimation.method_version == MODERATED_MEDIATION_METHOD_VERSION
            || estimation.method_version == CTA_PLS_METHOD_VERSION
            || estimation.method_version == WPLS_METHOD_VERSION
            || estimation.method_version == CCA_METHOD_VERSION
            || estimation.method_version == PLS_MGA_METHOD_VERSION
            || estimation.method_version == IPMA_METHOD_VERSION
            || estimation.method_version == CFA_ML_METHOD_VERSION
            || estimation.method_version == CBSEM_ML_METHOD_VERSION
            || estimation.method_version == PLS_PREDICT_METHOD_VERSION)
    {
        return Err(AssessmentError::InvalidEstimationResult);
    }
    for construct in &recipe.model.constructs {
        let score = estimation
            .construct_scores
            .get(&construct.id)
            .ok_or_else(|| {
                AssessmentError::ResultMismatch(format!(
                    "missing construct score for '{}'",
                    construct.id
                ))
            })?;
        if score.len() != estimation.used_observations
            || score.iter().any(|value| !value.is_finite())
        {
            return Err(AssessmentError::ResultMismatch(format!(
                "invalid construct score for '{}'",
                construct.id
            )));
        }
    }
    Ok(())
}

fn expand_higher_order_for_assessment(
    recipe: &AnalysisRecipe,
) -> Result<AnalysisRecipe, AssessmentError> {
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
        let position = construct_positions.get(&higher_order.id).ok_or_else(|| {
            AssessmentError::ResultMismatch(format!(
                "missing higher-order construct '{}'",
                higher_order.id
            ))
        })?;
        let indicators = match higher_order.method {
            HigherOrderMethod::RepeatedIndicators => {
                let mut seen = HashSet::new();
                let mut indicators = Vec::new();
                for component in &higher_order.components {
                    let component_indicators =
                        original_indicators.get(component.as_str()).ok_or_else(|| {
                            AssessmentError::ResultMismatch(format!(
                                "missing higher-order component '{component}'"
                            ))
                        })?;
                    for indicator in component_indicators {
                        if seen.insert(indicator.clone()) {
                            indicators.push(indicator.clone());
                        }
                    }
                }
                indicators
            }
            HigherOrderMethod::TwoStage => higher_order
                .components
                .iter()
                .map(|component| higher_order_component_indicator_name(&higher_order.id, component))
                .collect(),
            HigherOrderMethod::Hybrid => {
                let mut seen = HashSet::new();
                let mut indicators = Vec::new();
                for component in &higher_order.components {
                    let component_position =
                        construct_positions.get(component).ok_or_else(|| {
                            AssessmentError::ResultMismatch(format!(
                                "missing higher-order component '{component}'"
                            ))
                        })?;
                    let component_indicators =
                        original_indicators.get(component.as_str()).ok_or_else(|| {
                            AssessmentError::ResultMismatch(format!(
                                "missing higher-order component '{component}'"
                            ))
                        })?;
                    let (lower, higher) =
                        split_hybrid_component_indicators(component, component_indicators)?;
                    expanded.model.constructs[*component_position].indicators = lower;
                    for indicator in higher {
                        if seen.insert(indicator.clone()) {
                            indicators.push(indicator);
                        }
                    }
                }
                indicators
            }
        };
        if indicators.is_empty() {
            return Err(AssessmentError::ResultMismatch(format!(
                "higher-order construct '{}' has no generated indicators",
                higher_order.id
            )));
        }
        expanded.model.constructs[*position].indicators = indicators;
    }
    Ok(expanded)
}

fn split_hybrid_component_indicators(
    component: &str,
    indicators: &[String],
) -> Result<(Vec<String>, Vec<String>), AssessmentError> {
    if indicators.len() < 2 {
        return Err(AssessmentError::ResultMismatch(format!(
            "hybrid higher-order component '{component}' requires at least two indicators"
        )));
    }
    let split = (indicators.len() + 1) / 2;
    Ok((indicators[..split].to_vec(), indicators[split..].to_vec()))
}

fn add_two_stage_higher_order_columns(
    columns: &mut BTreeMap<String, Vec<f64>>,
    recipe: &AnalysisRecipe,
    estimation: &PlsResult,
) -> Result<(), AssessmentError> {
    for higher_order in &recipe.model.higher_order_constructs {
        if higher_order.method != HigherOrderMethod::TwoStage {
            continue;
        }
        for component in &higher_order.components {
            let scores = estimation.construct_scores.get(component).ok_or_else(|| {
                AssessmentError::ResultMismatch(format!(
                    "missing two-stage HOC component score for '{component}'"
                ))
            })?;
            if scores.len() != estimation.used_observations {
                return Err(AssessmentError::ResultMismatch(format!(
                    "invalid two-stage HOC component score length for '{component}'"
                )));
            }
            columns.insert(
                higher_order_component_indicator_name(&higher_order.id, component),
                scores.clone(),
            );
        }
    }
    Ok(())
}

fn higher_order_component_indicator_name(higher_order_id: &str, component_id: &str) -> String {
    format!("__qpls_hoc_{higher_order_id}_{component_id}")
}

fn complete_case_columns(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    control: &mut impl FnMut(AssessmentProgress) -> bool,
) -> Result<BTreeMap<String, Vec<f64>>, AssessmentError> {
    let indicators = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter())
        .collect::<Vec<_>>();
    let positions = indicators
        .iter()
        .map(|indicator| {
            let position = dataset
                .batch
                .schema()
                .index_of(indicator)
                .map_err(|_| AssessmentError::InvalidIndicator((*indicator).clone()))?;
            let array = dataset.batch.column(position);
            if array.as_any().downcast_ref::<Float64Array>().is_none()
                && array.as_any().downcast_ref::<Int64Array>().is_none()
            {
                return Err(AssessmentError::InvalidIndicator((*indicator).clone()));
            }
            Ok(((*indicator).clone(), position))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let row_count = dataset.batch.num_rows();
    let mut complete_rows = Vec::with_capacity(row_count);
    for row in 0..row_count {
        if row % 1024 == 0 {
            checkpoint(
                control,
                AssessmentPhase::PreparingRows,
                row as u64,
                row_count as u64,
            )?;
        }
        if positions.iter().all(|(_, position)| {
            let array = dataset.batch.column(*position);
            !array.is_null(row) && numeric_value(array.as_ref(), row).is_some_and(f64::is_finite)
        }) {
            complete_rows.push(row);
        }
    }
    checkpoint(
        control,
        AssessmentPhase::PreparingRows,
        row_count as u64,
        row_count as u64,
    )?;
    let copy_units = positions.len() * complete_rows.len();
    let mut copied = 0;
    let mut columns = BTreeMap::new();
    for (indicator, position) in positions {
        let mut values = Vec::with_capacity(complete_rows.len());
        for row in &complete_rows {
            if copied % 4096 == 0 {
                checkpoint(
                    control,
                    AssessmentPhase::PreparingIndicators,
                    copied as u64,
                    copy_units as u64,
                )?;
            }
            values.push(numeric_value(dataset.batch.column(position).as_ref(), *row).unwrap());
            copied += 1;
        }
        columns.insert(indicator, values);
    }
    checkpoint(
        control,
        AssessmentPhase::PreparingIndicators,
        copy_units as u64,
        copy_units as u64,
    )?;
    Ok(columns)
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

fn standardized_cronbach_alpha(columns: &[&[f64]]) -> Result<f64, AssessmentError> {
    let count = columns.len();
    let mut total_variance = count as f64;
    for left in 0..count {
        for right in (left + 1)..count {
            total_variance += 2.0 * correlation(columns[left], columns[right])?;
        }
    }
    if total_variance <= f64::EPSILON {
        return Err(AssessmentError::Numerical(
            "standardized item sum has zero variance".into(),
        ));
    }
    let count = count as f64;
    Ok(count / (count - 1.0) * (1.0 - count / total_variance))
}

struct RhoACalculation {
    value: Option<f64>,
    status: RhoAStatus,
    reason: Option<String>,
    warning_codes: Vec<String>,
    score_variance: Option<f64>,
    weight_norm_squared: Option<f64>,
    numerator: Option<f64>,
    denominator: Option<f64>,
}

impl RhoACalculation {
    fn unavailable(reason: &str, score_variance: Option<f64>) -> Self {
        Self {
            value: None,
            status: RhoAStatus::Unavailable,
            reason: Some(reason.into()),
            warning_codes: Vec::new(),
            score_variance,
            weight_norm_squared: None,
            numerator: None,
            denominator: None,
        }
    }

    fn not_applicable(reason: &str) -> Self {
        Self {
            value: None,
            status: RhoAStatus::NotApplicable,
            reason: Some(reason.into()),
            warning_codes: Vec::new(),
            score_variance: None,
            weight_norm_squared: None,
            numerator: None,
            denominator: None,
        }
    }
}

fn calculate_rho_a(
    construct: &Construct,
    columns: &[&[f64]],
    weighting_scheme: WeightingScheme,
    weights: &HashMap<&str, f64>,
    transform_scales: &HashMap<&str, f64>,
    loadings: &HashMap<&str, f64>,
) -> RhoACalculation {
    if weighting_scheme == WeightingScheme::Pca {
        return RhoACalculation::not_applicable("rho_a.pca_weights_not_applicable");
    }
    if columns.len() == 1 {
        return RhoACalculation::not_applicable("rho_a.single_indicator_not_identified");
    }

    let mut standardized_weights = Vec::with_capacity(columns.len());
    for ((indicator, column), _) in construct
        .indicators
        .iter()
        .zip(columns)
        .zip(0..columns.len())
    {
        let Some(weight) = weights.get(indicator.as_str()).copied() else {
            return RhoACalculation::unavailable("rho_a.estimation_input_mismatch", None);
        };
        let Some(scale) = transform_scales.get(indicator.as_str()).copied() else {
            return RhoACalculation::unavailable("rho_a.estimation_input_mismatch", None);
        };
        let raw_sd = sample_standard_deviation(column);
        if !weight.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || !raw_sd.is_finite()
            || raw_sd <= 0.0
        {
            return RhoACalculation::unavailable("rho_a.invalid_indicator_scale", None);
        }
        standardized_weights.push(weight * raw_sd / scale);
    }

    let count = columns.len();
    let mut correlations = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..count {
            correlations[row][column] = match correlation(columns[row], columns[column]) {
                Ok(value) if value.is_finite() => value,
                _ => return RhoACalculation::unavailable("rho_a.nonfinite_result", None),
            };
        }
    }
    let score_variance = quadratic_form(&standardized_weights, &correlations);
    let q_tolerance = 64.0
        * f64::EPSILON
        * standardized_weights
            .iter()
            .map(|value| value.abs().powi(2))
            .sum::<f64>()
            .max(1.0);
    if !score_variance.is_finite() || score_variance <= q_tolerance {
        return RhoACalculation::unavailable("rho_a.invalid_score_variance", Some(score_variance));
    }
    let divisor = score_variance.sqrt();
    let normalized = standardized_weights
        .iter()
        .map(|value| value / divisor)
        .collect::<Vec<_>>();
    let normalized_variance = quadratic_form(&normalized, &correlations);
    if !normalized_variance.is_finite() || (normalized_variance - 1.0).abs() > 1e-10 {
        return RhoACalculation::unavailable("rho_a.invalid_score_variance", Some(score_variance));
    }
    for row in 0..count {
        let reproduced_loading = (0..count)
            .map(|column| correlations[row][column] * normalized[column])
            .sum::<f64>();
        let Some(persisted_loading) = loadings.get(construct.indicators[row].as_str()).copied()
        else {
            return RhoACalculation::unavailable(
                "rho_a.estimation_input_mismatch",
                Some(score_variance),
            );
        };
        if !persisted_loading.is_finite() || (reproduced_loading - persisted_loading).abs() > 1e-10
        {
            return RhoACalculation::unavailable(
                "rho_a.estimation_input_mismatch",
                Some(score_variance),
            );
        }
    }

    rho_a_from_normalized_inputs(&correlations, &normalized, score_variance)
}

fn rho_a_from_normalized_inputs(
    correlations: &[Vec<f64>],
    weights: &[f64],
    score_variance: f64,
) -> RhoACalculation {
    let weight_norm_squared = weights.iter().map(|value| value * value).sum::<f64>();
    let fourth_sum = weights.iter().map(|value| value.powi(4)).sum::<f64>();
    let denominator = weight_norm_squared.powi(2) - fourth_sum;
    let numerator = (0..weights.len())
        .flat_map(|row| (0..weights.len()).map(move |column| (row, column)))
        .filter(|(row, column)| row != column)
        .map(|(row, column)| weights[row] * weights[column] * correlations[row][column])
        .sum::<f64>();
    let tolerance = 64.0 * f64::EPSILON * weight_norm_squared.powi(2).max(fourth_sum).max(1.0);
    if !weight_norm_squared.is_finite() || !numerator.is_finite() || !denominator.is_finite() {
        return RhoACalculation::unavailable("rho_a.nonfinite_result", Some(score_variance));
    }
    if denominator <= tolerance {
        return RhoACalculation::unavailable(
            "rho_a.off_diagonal_denominator_zero",
            Some(score_variance),
        );
    }
    let mut value = weight_norm_squared.powi(2) * numerator / denominator;
    if !value.is_finite() {
        return RhoACalculation::unavailable("rho_a.nonfinite_result", Some(score_variance));
    }
    let boundary_tolerance = 64.0 * f64::EPSILON * value.abs().max(1.0);
    let mut warning_codes = Vec::new();
    if value < 0.0 {
        if value >= -boundary_tolerance {
            value = 0.0;
        } else {
            warning_codes.push("rho_a.improper_below_zero".into());
        }
    } else if value > 1.0 {
        if value <= 1.0 + boundary_tolerance {
            value = 1.0;
        } else {
            warning_codes.push("rho_a.improper_above_one".into());
        }
    }
    if weights.len() == 2 {
        warning_codes.push("rho_a.two_indicator_limited_information".into());
    }
    RhoACalculation {
        value: Some(value),
        status: RhoAStatus::Available,
        reason: None,
        warning_codes,
        score_variance: Some(score_variance),
        weight_norm_squared: Some(weight_norm_squared),
        numerator: Some(numerator),
        denominator: Some(denominator),
    }
}

fn quadratic_form(weights: &[f64], matrix: &[Vec<f64>]) -> f64 {
    (0..weights.len())
        .flat_map(|row| (0..weights.len()).map(move |column| (row, column)))
        .map(|(row, column)| weights[row] * weights[column] * matrix[row][column])
        .sum()
}

fn sample_standard_deviation(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
        .sqrt()
}

fn correlation(left: &[f64], right: &[f64]) -> Result<f64, AssessmentError> {
    if left.len() != right.len() || left.len() < 2 {
        return Err(AssessmentError::ResultMismatch(
            "correlation columns have incompatible lengths".into(),
        ));
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let mut covariance = 0.0;
    let mut left_ss = 0.0;
    let mut right_ss = 0.0;
    for (left, right) in left.iter().zip(right) {
        let left = left - left_mean;
        let right = right - right_mean;
        covariance += left * right;
        left_ss += left * left;
        right_ss += right * right;
    }
    let denominator = (left_ss * right_ss).sqrt();
    if denominator <= f64::EPSILON || !denominator.is_finite() {
        return Err(AssessmentError::Numerical(
            "correlation contains a constant column".into(),
        ));
    }
    Ok(covariance / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use chrono::Utc;
    use qpls_core::{
        AnalysisSettings, Construct, ModelSpec, PROJECT_SCHEMA_VERSION, StructuralPath,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_estimation::estimate_pls;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn fixture() -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let recipe = AnalysisRecipe {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Simple reflective".into(),
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
            },
            settings: AnalysisSettings::default(),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
    }

    fn corporate_reputation_fixture() -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/corporate_reputation.csv"),
            "corporate_reputation.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let recipe = AnalysisRecipe {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Corporate reputation".into(),
                constructs: vec![
                    Construct {
                        id: "comp".into(),
                        name: "Competence".into(),
                        short_name: "COMP".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["COMP1".into(), "COMP2".into(), "COMP3".into()],
                    },
                    Construct {
                        id: "like".into(),
                        name: "Likeability".into(),
                        short_name: "LIKE".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["LIKE1".into(), "LIKE2".into()],
                    },
                    Construct {
                        id: "satisfaction".into(),
                        name: "Customer satisfaction".into(),
                        short_name: "CUSA".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["CUSA1".into(), "CUSA2".into()],
                    },
                    Construct {
                        id: "loyalty".into(),
                        name: "Customer loyalty".into(),
                        short_name: "CUSL".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["CUSL1".into(), "CUSL2".into()],
                    },
                ],
                paths: vec![
                    StructuralPath {
                        source: "comp".into(),
                        target: "satisfaction".into(),
                    },
                    StructuralPath {
                        source: "like".into(),
                        target: "satisfaction".into(),
                    },
                    StructuralPath {
                        source: "satisfaction".into(),
                        target: "loyalty".into(),
                    },
                ],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings: AnalysisSettings::default(),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
    }

    fn triangular_fixture() -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            b"x,z,y\n1,2,1\n2,1,3\n3,4,3\n4,3,5\n5,5,6\n6,7,8\n7,6,7\n8,9,10\n9,8,9\n10,10,12\n",
            "triangular.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let constructs = ["x", "z", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![id.into()],
            })
            .collect();
        let recipe = AnalysisRecipe {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::nil(),
                name: "Triangular".into(),
                constructs,
                paths: vec![
                    StructuralPath {
                        source: "x".into(),
                        target: "z".into(),
                    },
                    StructuralPath {
                        source: "x".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "z".into(),
                        target: "y".into(),
                    },
                ],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings: AnalysisSettings::default(),
            metadata: BTreeMap::new(),
        };
        (dataset, recipe)
    }

    #[test]
    fn standardized_alpha_matches_hand_calculation() {
        let first = [1.0, 2.0, 3.0, 4.0];
        let second = [1.0, 2.0, 4.0, 5.0];
        let r = correlation(&first, &second).unwrap();
        assert_abs_diff_eq!(
            standardized_cronbach_alpha(&[&first, &second]).unwrap(),
            2.0 * r / (1.0 + r),
            epsilon = 1e-14
        );
    }

    #[test]
    fn rho_a_matches_three_and_two_indicator_hand_fixtures() {
        let three = vec![
            vec![1.0, 0.5, 0.5],
            vec![0.5, 1.0, 0.5],
            vec![0.5, 0.5, 1.0],
        ];
        let equal_three = vec![1.0 / 6.0_f64.sqrt(); 3];
        let result = rho_a_from_normalized_inputs(&three, &equal_three, 1.0);
        assert_eq!(result.status, RhoAStatus::Available);
        assert_abs_diff_eq!(result.weight_norm_squared.unwrap(), 0.5, epsilon = 1e-14);
        assert_abs_diff_eq!(result.numerator.unwrap(), 0.5, epsilon = 1e-14);
        assert_abs_diff_eq!(result.denominator.unwrap(), 1.0 / 6.0, epsilon = 1e-14);
        assert_abs_diff_eq!(result.value.unwrap(), 0.75, epsilon = 1e-14);

        let two = vec![vec![1.0, 0.6], vec![0.6, 1.0]];
        let equal_two = vec![1.0 / 3.2_f64.sqrt(); 2];
        let result = rho_a_from_normalized_inputs(&two, &equal_two, 1.0);
        assert_abs_diff_eq!(result.value.unwrap(), 0.75, epsilon = 1e-14);
        assert_eq!(
            result.warning_codes,
            vec!["rho_a.two_indicator_limited_information"]
        );
    }

    #[test]
    fn rho_a_is_invariant_to_construct_orientation() {
        let correlations = vec![
            vec![1.0, 0.4, 0.2],
            vec![0.4, 1.0, 0.3],
            vec![0.2, 0.3, 1.0],
        ];
        let weights = vec![0.4, 0.5, 0.3];
        let variance = quadratic_form(&weights, &correlations);
        let normalized = weights
            .iter()
            .map(|value| value / variance.sqrt())
            .collect::<Vec<_>>();
        let reversed = normalized.iter().map(|value| -value).collect::<Vec<_>>();
        let forward = rho_a_from_normalized_inputs(&correlations, &normalized, variance);
        let reverse = rho_a_from_normalized_inputs(&correlations, &reversed, variance);
        assert_abs_diff_eq!(
            forward.value.unwrap(),
            reverse.value.unwrap(),
            epsilon = 1e-14
        );
    }

    #[test]
    fn rho_a_matches_independent_decimal_reference_and_metamorphics() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/results/rho_a_reference.json"
        ))
        .unwrap();
        let expected = |case: &str, field: &str| {
            reference[case][field]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };
        let correlations = vec![
            vec![1.0, -0.4, 0.1],
            vec![-0.4, 1.0, -0.2],
            vec![0.1, -0.2, 1.0],
        ];
        let incoming = vec![0.7, -0.2, 0.5];
        let normalize = |weights: &[f64], matrix: &[Vec<f64>]| {
            let variance = quadratic_form(weights, matrix);
            (
                weights
                    .iter()
                    .map(|value| value / variance.sqrt())
                    .collect::<Vec<_>>(),
                variance,
            )
        };
        let (weights, variance) = normalize(&incoming, &correlations);
        let result = rho_a_from_normalized_inputs(&correlations, &weights, variance);
        assert_abs_diff_eq!(
            variance,
            expected("unequal_signed", "score_variance"),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.weight_norm_squared.unwrap(),
            expected("unequal_signed", "weight_norm_squared"),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.numerator.unwrap(),
            expected("unequal_signed", "off_diagonal_numerator"),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.denominator.unwrap(),
            expected("unequal_signed", "off_diagonal_denominator"),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.value.unwrap(),
            expected("unequal_signed", "rho_a"),
            epsilon = 1e-14
        );

        let scaled_incoming = incoming
            .iter()
            .map(|value| value * 17.0)
            .collect::<Vec<_>>();
        let (scaled_weights, scaled_variance) = normalize(&scaled_incoming, &correlations);
        let scaled = rho_a_from_normalized_inputs(&correlations, &scaled_weights, scaled_variance);
        assert_abs_diff_eq!(
            result.value.unwrap(),
            scaled.value.unwrap(),
            epsilon = 1e-14
        );

        let permutation = [2usize, 0, 1];
        let permuted_correlations = permutation
            .iter()
            .map(|row| {
                permutation
                    .iter()
                    .map(|column| correlations[*row][*column])
                    .collect()
            })
            .collect::<Vec<Vec<f64>>>();
        let permuted_incoming = permutation
            .iter()
            .map(|index| incoming[*index])
            .collect::<Vec<_>>();
        let (permuted_weights, permuted_variance) =
            normalize(&permuted_incoming, &permuted_correlations);
        let permuted = rho_a_from_normalized_inputs(
            &permuted_correlations,
            &permuted_weights,
            permuted_variance,
        );
        assert_abs_diff_eq!(
            result.value.unwrap(),
            permuted.value.unwrap(),
            epsilon = 1e-14
        );
    }

    #[test]
    fn rho_a_matches_dijkstra_henseler_2015_equation_3_fixture() {
        let primary: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/results/rho_a_primary_dijkstra_henseler_2015.json"
        ))
        .unwrap();
        assert_eq!(
            primary["method_version"].as_str().unwrap(),
            RHO_A_METHOD_VERSION
        );
        let tolerance = primary["tolerance"].as_f64().unwrap();
        for fixture in primary["fixtures"].as_array().unwrap() {
            let correlations = fixture["covariance_matrix"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    row.as_array()
                        .unwrap()
                        .iter()
                        .map(|value| value.as_f64().unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let incoming = fixture["incoming_weight_vector"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap())
                .collect::<Vec<_>>();
            let score_variance = quadratic_form(&incoming, &correlations);
            let weights = incoming
                .iter()
                .map(|value| value / score_variance.sqrt())
                .collect::<Vec<_>>();
            let result = rho_a_from_normalized_inputs(&correlations, &weights, score_variance);
            assert_eq!(
                result.status,
                RhoAStatus::Available,
                "{}",
                fixture["name"].as_str().unwrap()
            );
            let expected = &fixture["expected"];
            assert_abs_diff_eq!(
                result.weight_norm_squared.unwrap(),
                expected["weight_norm_squared"].as_f64().unwrap(),
                epsilon = tolerance
            );
            assert_abs_diff_eq!(
                result.numerator.unwrap(),
                expected["off_diagonal_numerator"].as_f64().unwrap(),
                epsilon = tolerance
            );
            assert_abs_diff_eq!(
                result.denominator.unwrap(),
                expected["off_diagonal_denominator"].as_f64().unwrap(),
                epsilon = tolerance
            );
            assert_abs_diff_eq!(
                result.value.unwrap(),
                expected["rho_a"].as_f64().unwrap(),
                epsilon = tolerance
            );
        }
    }

    #[test]
    fn rho_a_preserves_improper_values_and_reports_degenerate_inputs() {
        let normalize = |weights: &[f64], matrix: &[Vec<f64>]| {
            let variance = quadratic_form(weights, matrix);
            (
                weights
                    .iter()
                    .map(|value| value / variance.sqrt())
                    .collect::<Vec<_>>(),
                variance,
            )
        };
        let below = vec![
            vec![1.0, -0.2, -0.2],
            vec![-0.2, 1.0, -0.2],
            vec![-0.2, -0.2, 1.0],
        ];
        let (weights, variance) = normalize(&[1.0, 1.0, 1.0], &below);
        let result = rho_a_from_normalized_inputs(&below, &weights, variance);
        assert_abs_diff_eq!(result.value.unwrap(), -1.0, epsilon = 1e-14);
        assert_eq!(result.warning_codes, vec!["rho_a.improper_below_zero"]);

        let above = vec![
            vec![1.0, -0.7, -0.7],
            vec![-0.7, 1.0, 0.1],
            vec![-0.7, 0.1, 1.0],
        ];
        let (weights, variance) = normalize(&[-2.0, 0.5, 0.5], &above);
        let result = rho_a_from_normalized_inputs(&above, &weights, variance);
        assert_abs_diff_eq!(result.value.unwrap(), 1.9035250463821892, epsilon = 1e-14);
        assert_eq!(result.warning_codes, vec!["rho_a.improper_above_one"]);

        let identity = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let degenerate = rho_a_from_normalized_inputs(&identity, &[1.0, 0.0, 0.0], 1.0);
        assert_eq!(degenerate.status, RhoAStatus::Unavailable);
        assert_eq!(
            degenerate.reason.as_deref(),
            Some("rho_a.off_diagonal_denominator_zero")
        );

        let mut nonfinite = identity;
        nonfinite[0][1] = f64::NAN;
        let result = rho_a_from_normalized_inputs(&nonfinite, &[0.5, 0.5, 0.5], 1.0);
        assert_eq!(result.reason.as_deref(), Some("rho_a.nonfinite_result"));
    }

    #[test]
    fn rho_a_canonicalizes_only_roundoff_boundary_excursions() {
        let two_item = |correlation: f64| {
            let matrix = vec![vec![1.0, correlation], vec![correlation, 1.0]];
            let weight = 1.0 / (2.0 + 2.0 * correlation).sqrt();
            rho_a_from_normalized_inputs(&matrix, &[weight, weight], 2.0 + 2.0 * correlation)
        };
        let near_zero = two_item(-1e-15);
        assert_eq!(near_zero.value, Some(0.0));
        assert!(
            !near_zero
                .warning_codes
                .contains(&"rho_a.improper_below_zero".to_string())
        );
        let below_zero = two_item(-1e-12);
        assert!(below_zero.value.unwrap() < 0.0);
        assert!(
            below_zero
                .warning_codes
                .contains(&"rho_a.improper_below_zero".to_string())
        );

        let near_one = two_item(1.0 + 1e-15);
        assert_eq!(near_one.value, Some(1.0));
        assert!(
            !near_one
                .warning_codes
                .contains(&"rho_a.improper_above_one".to_string())
        );
        let above_one = two_item(1.0 + 1e-12);
        assert!(above_one.value.unwrap() > 1.0);
        assert!(
            above_one
                .warning_codes
                .contains(&"rho_a.improper_above_one".to_string())
        );

        let identity = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let below_denominator_tolerance =
            rho_a_from_normalized_inputs(&identity, &[1.0, 1e-8, 0.0], 1.0);
        assert_eq!(
            below_denominator_tolerance.reason.as_deref(),
            Some("rho_a.off_diagonal_denominator_zero")
        );
        let above_denominator_tolerance =
            rho_a_from_normalized_inputs(&identity, &[1.0, 1e-6, 0.0], 1.0);
        assert_eq!(above_denominator_tolerance.status, RhoAStatus::Available);
    }

    #[test]
    fn rho_a_coordinate_checks_cover_all_preprocessing_modes_and_mismatch() {
        let (dataset, recipe) = fixture();
        for preprocessing in [
            qpls_core::Preprocessing::Standardized,
            qpls_core::Preprocessing::MeanCentered,
            qpls_core::Preprocessing::Unstandardized,
        ] {
            let mut configured = recipe.clone();
            configured.settings.preprocessing = preprocessing;
            let estimation = estimate_pls(&dataset, &configured).unwrap();
            let assessment = assess_pls(&dataset, &configured, &estimation).unwrap();
            assert!(assessment.construct_quality.iter().all(|row| {
                row.rho_a_status == Some(RhoAStatus::Available)
                    && row.rho_a.is_some_and(f64::is_finite)
                    && row.rho_a_reason.is_none()
            }));
        }

        let mut factor_recipe = recipe.clone();
        factor_recipe.settings.weighting_scheme = WeightingScheme::Factor;
        let factor_estimation = estimate_pls(&dataset, &factor_recipe).unwrap();
        let factor_assessment = assess_pls(&dataset, &factor_recipe, &factor_estimation).unwrap();
        assert!(
            factor_assessment
                .construct_quality
                .iter()
                .all(|row| row.rho_a_status == Some(RhoAStatus::Available))
        );

        let missing = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,2,1\n2,3,3,2\n3,5,4,4\n4,4,6,5\n5,6,7,7\n6,7,9,8\n100,,100,100\n",
            "missing.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut missing_recipe = recipe.clone();
        missing_recipe.dataset_fingerprint = missing.fingerprint.0.clone();
        let missing_estimation = estimate_pls(&missing, &missing_recipe).unwrap();
        let missing_assessment =
            assess_pls(&missing, &missing_recipe, &missing_estimation).unwrap();
        let baseline_estimation = estimate_pls(&dataset, &recipe).unwrap();
        let baseline_assessment = assess_pls(&dataset, &recipe, &baseline_estimation).unwrap();
        assert_eq!(
            missing_estimation.used_observations,
            dataset.batch.num_rows()
        );
        for (actual, expected) in missing_assessment
            .construct_quality
            .iter()
            .zip(&baseline_assessment.construct_quality)
        {
            assert_abs_diff_eq!(
                actual.rho_a.unwrap(),
                expected.rho_a.unwrap(),
                epsilon = 1e-14
            );
        }

        let mut estimation = estimate_pls(&dataset, &recipe).unwrap();
        estimation.outer_estimates[0].loading += 0.01;
        let assessment = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(
            assessment.construct_quality[0].rho_a_reason.as_deref(),
            Some("rho_a.estimation_input_mismatch")
        );
    }

    #[test]
    fn rho_a_reports_constant_nonfinite_and_zero_score_inputs() {
        let construct = Construct {
            id: "c".into(),
            name: "C".into(),
            short_name: "C".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["a".into(), "b".into()],
        };
        let weights = HashMap::from([("a", 1.0), ("b", 1.0)]);
        let scales = HashMap::from([("a", 1.0), ("b", 1.0)]);
        let loadings = HashMap::from([("a", 0.5), ("b", 0.5)]);

        let constant_a = [1.0, 1.0, 1.0];
        let regular_b = [1.0, 2.0, 3.0];
        let result = calculate_rho_a(
            &construct,
            &[&constant_a, &regular_b],
            WeightingScheme::Path,
            &weights,
            &scales,
            &loadings,
        );
        assert_eq!(
            result.reason.as_deref(),
            Some("rho_a.invalid_indicator_scale")
        );

        let nonfinite_a = [1.0, f64::NAN, 3.0];
        let result = calculate_rho_a(
            &construct,
            &[&nonfinite_a, &regular_b],
            WeightingScheme::Path,
            &weights,
            &scales,
            &loadings,
        );
        assert_eq!(
            result.reason.as_deref(),
            Some("rho_a.invalid_indicator_scale")
        );

        let positive = [1.0, 2.0, 3.0, 4.0];
        let negative = [-1.0, -2.0, -3.0, -4.0];
        let result = calculate_rho_a(
            &construct,
            &[&positive, &negative],
            WeightingScheme::Path,
            &weights,
            &scales,
            &loadings,
        );
        assert_eq!(
            result.reason.as_deref(),
            Some("rho_a.invalid_score_variance")
        );
    }

    #[test]
    fn rho_a_is_invariant_to_positive_affine_data_and_recipe_reordering() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.tolerance = 1e-12;
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let baseline = assess_pls(&dataset, &recipe, &estimation).unwrap();
        let transformed = import_delimited_bytes(
            b"x1,x2,y1,y2\n12,11,12,3\n14,14,13,7\n16,20,14,15\n18,17,16,19\n20,23,17,27\n22,26,19,31\n",
            "affine.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut transformed_recipe = recipe.clone();
        transformed_recipe.dataset_fingerprint = transformed.fingerprint.0.clone();
        let transformed_estimation = estimate_pls(&transformed, &transformed_recipe).unwrap();
        let transformed_assessment =
            assess_pls(&transformed, &transformed_recipe, &transformed_estimation).unwrap();
        for (left, right) in baseline
            .construct_quality
            .iter()
            .zip(&transformed_assessment.construct_quality)
        {
            assert_abs_diff_eq!(left.rho_a.unwrap(), right.rho_a.unwrap(), epsilon = 1e-12);
        }

        let mut reordered_recipe = recipe;
        reordered_recipe.model.constructs.reverse();
        for construct in &mut reordered_recipe.model.constructs {
            construct.indicators.reverse();
        }
        let reordered_estimation = estimate_pls(&dataset, &reordered_recipe).unwrap();
        let reordered = assess_pls(&dataset, &reordered_recipe, &reordered_estimation).unwrap();
        for baseline_row in &baseline.construct_quality {
            let reordered_row = reordered
                .construct_quality
                .iter()
                .find(|row| row.construct == baseline_row.construct)
                .unwrap();
            assert_abs_diff_eq!(
                baseline_row.rho_a.unwrap(),
                reordered_row.rho_a.unwrap(),
                epsilon = 1e-12
            );
        }
    }

    #[test]
    fn rho_a_applicability_is_explicit_for_pca_and_formative_blocks() {
        let (dataset, mut recipe) = fixture();
        recipe.settings.weighting_scheme = WeightingScheme::Pca;
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let assessment = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert!(assessment.construct_quality.iter().all(|row| {
            row.rho_a_status == Some(RhoAStatus::NotApplicable)
                && row.rho_a_reason.as_deref() == Some("rho_a.pca_weights_not_applicable")
        }));

        recipe.settings.weighting_scheme = WeightingScheme::Path;
        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let assessment = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(
            assessment.construct_quality[0].rho_a_reason.as_deref(),
            Some("rho_a.formative_not_applicable")
        );

        recipe.model.constructs[0].mode = MeasurementMode::Reflective;
        recipe.model.constructs[0].indicators = vec!["x1".into()];
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let assessment = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(
            assessment.construct_quality[0].rho_a_reason.as_deref(),
            Some("rho_a.single_indicator_not_identified")
        );
    }

    #[test]
    fn fit_measures_match_hand_calculated_triangle_residuals() {
        let observed = vec![vec![1.0, 0.4], vec![0.4, 1.0]];
        let implied = vec![vec![1.0, 0.1], vec![0.1, 1.0]];
        let fit = fit_measures(&observed, &implied);
        assert_abs_diff_eq!(fit.d_uls, 0.09, epsilon = 1e-14);
        assert_abs_diff_eq!(fit.srmr, (0.09_f64 / 3.0).sqrt(), epsilon = 1e-14);
    }

    #[test]
    fn vif_matches_auxiliary_regression_identity_and_flags_perfect_explanation() {
        let target = [1.0, 2.0, 3.0, 4.0, 5.0];
        let predictor = [1.0, 2.0, 4.0, 3.0, 5.0];
        let expected = 1.0 / (1.0 - correlation(&target, &predictor).unwrap().powi(2));
        assert_abs_diff_eq!(
            variance_inflation_factor(&target, &[&predictor])
                .unwrap()
                .unwrap(),
            expected,
            epsilon = 1e-12
        );
        assert_eq!(
            variance_inflation_factor(&target, &[&target]).unwrap(),
            None
        );
        assert_eq!(variance_inflation_factor(&target, &[]).unwrap(), Some(1.0));
    }

    #[test]
    fn vif_cancels_during_large_correlation_preparation() {
        let target = (0..100_000)
            .map(|index| (index as f64 * 0.013).sin())
            .collect::<Vec<_>>();
        let predictor = (0..100_000)
            .map(|index| (index as f64 * 0.017).cos())
            .collect::<Vec<_>>();
        let mut last_completed = 0;
        let result =
            variance_inflation_factor_with_control(&target, &[&predictor], |completed, _| {
                last_completed = completed;
                completed < 2_048
            });
        assert_eq!(result, Err(AssessmentError::Cancelled));
        assert_eq!(last_completed, 2_048);
    }

    #[test]
    fn cohen_f_squared_matches_reference_regressions_and_path_order_is_invariant() {
        let (dataset, mut recipe) = triangular_fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        let expected = [
            ("x", "z", 0.0, 9.56871118012422),
            ("x", "y", 0.905736330090574, 1.03127209868783),
            ("z", "y", 0.946438555294644, 0.154186637332702),
        ];
        assert_eq!(result.f_squared.len(), expected.len());
        for (source, target, excluded, effect) in expected {
            let row = result
                .f_squared
                .iter()
                .find(|row| row.source_construct == source && row.target_construct == target)
                .unwrap();
            assert_abs_diff_eq!(row.excluded_r_squared.unwrap(), excluded, epsilon = 1e-6);
            assert_abs_diff_eq!(row.f_squared.unwrap(), effect, epsilon = 1e-6);
        }

        recipe.model.paths.reverse();
        let reordered_estimation = estimate_pls(&dataset, &recipe).unwrap();
        let reordered = assess_pls(&dataset, &recipe, &reordered_estimation).unwrap();
        for row in &result.f_squared {
            let same_path = reordered
                .f_squared
                .iter()
                .find(|candidate| {
                    candidate.source_construct == row.source_construct
                        && candidate.target_construct == row.target_construct
                })
                .unwrap();
            assert_abs_diff_eq!(
                same_path.f_squared.unwrap(),
                row.f_squared.unwrap(),
                epsilon = 1e-10
            );
        }
    }

    #[test]
    fn single_predictor_effect_size_uses_intercept_only_excluded_r_squared() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(result.f_squared.len(), 1);
        assert_eq!(result.f_squared[0].excluded_r_squared, Some(0.0));
        assert_abs_diff_eq!(
            result.f_squared[0].f_squared.unwrap(),
            estimation.r_squared["y"] / (1.0 - estimation.r_squared["y"]),
            epsilon = 1e-10
        );
        assert!(
            !result
                .warnings
                .iter()
                .any(|warning| warning.contains("reduced model failed"))
        );
    }

    #[test]
    fn effect_size_retains_an_isolated_source_measurement_block() {
        let (dataset, mut recipe) = triangular_fixture();
        recipe
            .model
            .paths
            .retain(|path| !(path.source == "x" && path.target == "z"));
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();

        let isolated_source = result
            .f_squared
            .iter()
            .find(|row| row.source_construct == "x" && row.target_construct == "y")
            .unwrap();
        assert!(isolated_source.excluded_r_squared.is_some());
        assert!(isolated_source.f_squared.is_some());
        assert!(
            !result
                .warnings
                .iter()
                .any(|warning| warning.contains("reduced model failed"))
        );
    }

    #[test]
    fn perfect_included_r_squared_produces_denominator_warning() {
        let (_, mut recipe) = triangular_fixture();
        let dataset = import_delimited_bytes(
            b"x,z,y\n1,2,3\n2,1,3\n3,4,7\n4,3,7\n5,5,10\n6,7,13\n7,6,13\n8,9,17\n9,8,17\n10,10,20\n",
            "perfect-triangular.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        let target_rows = result
            .f_squared
            .iter()
            .filter(|row| row.target_construct == "y")
            .collect::<Vec<_>>();
        assert_eq!(target_rows.len(), 2);
        assert!(target_rows.iter().all(|row| row.f_squared.is_none()));
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("included R-squared is one"))
        );
    }

    #[test]
    fn cancellation_is_forwarded_into_reduced_effect_size_estimation() {
        let (dataset, recipe) = triangular_fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut effect_calls = 0;
        let result = assess_pls_with_control(&dataset, &recipe, &estimation, |progress| {
            if progress.phase == AssessmentPhase::EffectSize {
                effect_calls += 1;
                effect_calls < 2
            } else {
                true
            }
        });
        assert_eq!(result, Err(AssessmentError::Cancelled));
        assert_eq!(effect_calls, 2);
    }

    #[test]
    fn effect_size_reports_monotonic_fixed_score_progress() {
        let (dataset, recipe) = triangular_fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut updates = Vec::new();
        assess_pls_with_control(&dataset, &recipe, &estimation, |progress| {
            if progress.phase == AssessmentPhase::EffectSize {
                updates.push((progress.completed_units, progress.total_units));
            }
            true
        })
        .unwrap();

        let expected_total = recipe.model.paths.len() as u64 * NESTED_PROGRESS_SCALE;
        assert!(updates.len() >= recipe.model.paths.len() * 2);
        assert!(updates.iter().all(|(_, total)| *total == expected_total));
        assert!(updates.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert_eq!(updates.last().unwrap().0, expected_total);
    }

    #[test]
    fn legacy_payload_without_new_assessment_fields_defaults_to_empty() {
        let (dataset, recipe) = triangular_fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        let mut value = serde_json::to_value(result).unwrap();
        value.as_object_mut().unwrap().remove("f_squared");
        value.as_object_mut().unwrap().remove("model_fit");
        value.as_object_mut().unwrap().remove("blindfolding");
        let restored: AssessmentResult = serde_json::from_value(value).unwrap();
        assert!(restored.f_squared.is_empty());
        assert!(restored.model_fit.is_none());
        assert!(restored.blindfolding.is_none());
    }

    #[test]
    fn htmt_matches_hand_calculated_absolute_correlation_ratio() {
        let left = Construct {
            id: "a".into(),
            name: "A".into(),
            short_name: "A".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["a1".into(), "a2".into()],
        };
        let right = Construct {
            id: "b".into(),
            name: "B".into(),
            short_name: "B".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["b1".into(), "b2".into()],
        };
        let columns = BTreeMap::from([
            ("a1".into(), vec![1.0, 2.0, 3.0, 4.0, 5.0]),
            ("a2".into(), vec![1.0, 2.0, 4.0, 4.0, 6.0]),
            ("b1".into(), vec![2.0, 1.0, 4.0, 3.0, 7.0]),
            ("b2".into(), vec![1.0, 0.0, 3.0, 5.0, 8.0]),
        ]);
        assert_abs_diff_eq!(
            htmt_ratio(&left, &right, &columns).unwrap().unwrap(),
            0.9481994254637219,
            epsilon = 1e-14
        );
    }

    #[test]
    fn htmt_zero_monotrait_denominator_is_not_applicable() {
        let left = Construct {
            id: "a".into(),
            name: "A".into(),
            short_name: "A".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["a1".into(), "a2".into()],
        };
        let right = Construct {
            id: "b".into(),
            name: "B".into(),
            short_name: "B".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["b1".into(), "b2".into()],
        };
        let columns = BTreeMap::from([
            ("a1".into(), vec![1.0, 0.0, -1.0, 0.0]),
            ("a2".into(), vec![0.0, 1.0, 0.0, -1.0]),
            ("b1".into(), vec![1.0, 2.0, 3.0, 4.0]),
            ("b2".into(), vec![1.0, 3.0, 2.0, 5.0]),
        ]);
        assert_eq!(htmt_ratio(&left, &right, &columns).unwrap(), None);
    }

    #[test]
    fn htmt_original_and_plus_have_explicit_sign_semantics() {
        let left = Construct {
            id: "a".into(),
            name: "A".into(),
            short_name: "A".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["a1".into(), "a2".into()],
        };
        let right = Construct {
            id: "b".into(),
            name: "B".into(),
            short_name: "B".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["b1".into(), "b2".into()],
        };
        let columns = BTreeMap::from([
            ("a1".into(), vec![1.0, 2.0, 3.0, 4.0, 5.0]),
            ("a2".into(), vec![1.0, 2.0, 4.0, 4.0, 6.0]),
            ("b1".into(), vec![2.0, 1.0, 4.0, 3.0, 7.0]),
            ("b2".into(), vec![1.0, 0.0, 3.0, 5.0, 8.0]),
        ]);
        let plus = htmt_cell(&left, &right, &columns, false, true).unwrap();
        let original = htmt_cell(&left, &right, &columns, false, false).unwrap();
        assert_eq!(plus.status, HtmtStatus::Available);
        assert_eq!(original.status, HtmtStatus::Available);
        assert_abs_diff_eq!(
            plus.value.unwrap(),
            original.value.unwrap(),
            epsilon = 1e-14
        );

        let mut reversed = columns.clone();
        for indicator in &right.indicators {
            for value in reversed.get_mut(indicator).unwrap() {
                *value = -*value;
            }
        }
        let reversed_plus = htmt_cell(&left, &right, &reversed, false, true).unwrap();
        let reversed_original = htmt_cell(&left, &right, &reversed, false, false).unwrap();
        assert_abs_diff_eq!(
            reversed_plus.value.unwrap(),
            plus.value.unwrap(),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            reversed_original.value.unwrap(),
            -original.value.unwrap(),
            epsilon = 1e-14
        );

        let mut one_indicator_reversed = columns.clone();
        for value in one_indicator_reversed.get_mut("b1").unwrap() {
            *value = -*value;
        }
        let one_reversed_plus =
            htmt_cell(&left, &right, &one_indicator_reversed, false, true).unwrap();
        let one_reversed_original =
            htmt_cell(&left, &right, &one_indicator_reversed, false, false).unwrap();
        assert_abs_diff_eq!(
            one_reversed_plus.value.unwrap(),
            plus.value.unwrap(),
            epsilon = 1e-14
        );
        assert_eq!(one_reversed_original.status, HtmtStatus::Unavailable);
        assert_eq!(
            one_reversed_original.reason.as_deref(),
            Some("htmt.original_nonpositive_monotrait_mean")
        );
    }

    #[test]
    fn htmt_values_above_one_are_preserved() {
        assert_abs_diff_eq!(
            htmt_value_from_means(0.2, 0.1, 0.1).unwrap(),
            2.0,
            epsilon = 1e-14
        );
        assert_eq!(htmt_value_from_means(0.0, 0.25, 0.36).unwrap(), 0.0);
    }

    #[test]
    fn htmt_plus_matches_ringle_2023_rounded_formula_examples() {
        let published: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../validation/results/htmt_published_ringle_2023.json"
        ))
        .unwrap();
        assert_eq!(
            published["method_version"].as_str().unwrap(),
            HTMT_PLUS_METHOD_VERSION
        );
        let tolerance = published["rounding_tolerance"].as_f64().unwrap();
        for fixture in published["fixtures"].as_array().unwrap() {
            let actual = htmt_value_from_means(
                fixture["heterotrait_mean_abs"].as_f64().unwrap(),
                fixture["left_monotrait_mean_abs"].as_f64().unwrap(),
                fixture["right_monotrait_mean_abs"].as_f64().unwrap(),
            )
            .unwrap();
            let expected = fixture["reported_htmt_plus"].as_f64().unwrap();
            assert!(
                (actual - expected).abs() <= tolerance,
                "{}: actual {actual} expected {expected}",
                fixture["name"].as_str().unwrap()
            );
        }
        for fixture in published["original_htmt_unavailable_examples"]
            .as_array()
            .unwrap()
        {
            let left = fixture["left_monotrait_mean"].as_f64().unwrap();
            let right = fixture["right_monotrait_mean"].as_f64().unwrap();
            assert!(
                left <= 64.0 * f64::EPSILON || right <= 64.0 * f64::EPSILON,
                "{} should be unavailable before denominator evaluation",
                fixture["name"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn htmt_matches_independent_corporate_reputation_reference() {
        let reference: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../validation/results/htmt_reference.json"
        ))
        .unwrap();
        assert_eq!(
            reference["method_versions"]["htmt_plus"],
            HTMT_PLUS_METHOD_VERSION
        );
        assert_eq!(
            reference["method_versions"]["htmt_original"],
            HTMT_ORIGINAL_METHOD_VERSION
        );
        assert!(
            reference["metamorphic_checks"]["positive_affine_htmt_plus_max_delta"]
                .as_f64()
                .unwrap()
                <= 1e-12
        );
        assert!(
            reference["metamorphic_checks"]["positive_affine_original_max_delta"]
                .as_f64()
                .unwrap()
                <= 1e-12
        );
        assert!(
            reference["metamorphic_checks"]["reverse_one_indicator_plus_max_delta"]
                .as_f64()
                .unwrap()
                <= 1e-12
        );
        assert_eq!(
            reference["metamorphic_checks"]["reverse_one_indicator_original_has_unavailable"],
            true
        );
        let baseline = reference["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|fixture| fixture["name"] == "baseline")
            .unwrap();
        let (dataset, recipe) = corporate_reputation_fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let assessment = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_htmt_matches_reference(
            assessment.htmt_plus.as_ref().unwrap(),
            &baseline["htmt_plus"],
            1e-12,
        );
        assert_htmt_matches_reference(
            assessment.htmt_original.as_ref().unwrap(),
            &baseline["htmt_original"],
            1e-12,
        );
        let constructs = baseline["constructs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            assessment.htmt_plus.as_ref().unwrap().constructs,
            constructs
        );
        assert!(
            assessment.htmt_plus.as_ref().unwrap().cells[2][3]
                .value
                .unwrap()
                > 1.0
        );
    }

    fn assert_htmt_matches_reference(
        actual: &HtmtAssessment,
        expected: &serde_json::Value,
        epsilon: f64,
    ) {
        let expected = expected.as_array().unwrap();
        assert_eq!(actual.cells.len(), expected.len());
        for (row_index, row) in expected.iter().enumerate() {
            let row = row.as_array().unwrap();
            assert_eq!(actual.cells[row_index].len(), row.len());
            for (column_index, expected_cell) in row.iter().enumerate() {
                let actual_cell = &actual.cells[row_index][column_index];
                let expected_status = expected_cell["status"].as_str().unwrap();
                assert_eq!(
                    format!("{:?}", actual_cell.status).to_lowercase(),
                    expected_status
                );
                assert_eq!(
                    actual_cell.reason.as_deref(),
                    expected_cell["reason"].as_str()
                );
                match expected_cell["value"].as_f64() {
                    Some(expected_value) => {
                        assert_abs_diff_eq!(
                            actual_cell.value.unwrap(),
                            expected_value,
                            epsilon = epsilon
                        );
                    }
                    None => assert_eq!(actual_cell.value, None),
                }
            }
        }
    }

    #[test]
    fn reference_fixture_matches_csem_assessment() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        let x = &result.construct_quality[0];
        let y = &result.construct_quality[1];
        assert_abs_diff_eq!(x.ave.unwrap(), 0.9714019, epsilon = 1e-6);
        assert_abs_diff_eq!(y.ave.unwrap(), 0.9911160, epsilon = 1e-6);
        assert_abs_diff_eq!(x.rho_c.unwrap(), 0.985493521976651, epsilon = 1e-6);
        assert_abs_diff_eq!(y.rho_c.unwrap(), 0.995538165708917, epsilon = 1e-6);
        assert_abs_diff_eq!(x.cronbach_alpha.unwrap(), 0.970588235294118, epsilon = 1e-6);
        assert_abs_diff_eq!(y.cronbach_alpha.unwrap(), 0.991037659286314, epsilon = 1e-6);
        assert_abs_diff_eq!(result.r_squared["y"], 0.9670341, epsilon = 1e-6);
        let fit = result.model_fit.as_ref().unwrap();
        assert_abs_diff_eq!(fit.estimated.srmr, 0.02302786723091, epsilon = 1e-6);
        assert_abs_diff_eq!(fit.estimated.d_uls, 0.00530282669204417, epsilon = 1e-6);
        assert_abs_diff_eq!(fit.saturated.srmr, 0.02302786723091, epsilon = 1e-6);
        let blindfolding = result.blindfolding.as_ref().unwrap();
        assert_eq!(blindfolding.settings.omission_distance, 5);
        let redundancy = &blindfolding.constructs[0];
        assert_abs_diff_eq!(
            redundancy.q_squared.unwrap(),
            0.6056051520272379,
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            redundancy.prediction_error_sum_squares.unwrap(),
            10.808524585586486,
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            redundancy.q_squared.unwrap(),
            1.0 - redundancy.prediction_error_sum_squares.unwrap()
                / redundancy.observation_sum_squares.unwrap(),
            epsilon = 1e-14
        );
        let repeated = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(result.blindfolding, repeated.blindfolding);
        assert_eq!(result.structural_quality[0].predictor_count, 1);
        assert_abs_diff_eq!(
            result.structural_quality[0].adjusted_r_squared.unwrap(),
            1.0 - (1.0 - result.r_squared["y"]) * 5.0 / 4.0,
            epsilon = 1e-14
        );
        assert_eq!(
            result.structural_vif,
            vec![StructuralVif {
                target_construct: "y".into(),
                predictor_construct: "x".into(),
                vif: Some(1.0),
            }]
        );
        let htmt = result.htmt_plus.as_ref().unwrap();
        assert_eq!(htmt.cells[0][0].value, Some(1.0));
        assert_abs_diff_eq!(
            htmt.cells[0][1].value.unwrap(),
            htmt.cells[1][0].value.unwrap(),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.fornell_larcker.values[0][0].unwrap(),
            x.ave.unwrap().sqrt(),
            epsilon = 1e-14
        );
        assert_abs_diff_eq!(
            result.fornell_larcker.values[0][1].unwrap(),
            0.983378918793432,
            epsilon = 1e-6
        );
        for outer in &estimation.outer_estimates {
            let own = result
                .cross_loadings
                .iter()
                .find(|cell| cell.indicator == outer.indicator && cell.construct == outer.construct)
                .unwrap();
            assert_abs_diff_eq!(own.loading, outer.loading, epsilon = 1e-12);
        }
        for (indicator, construct, expected) in [
            ("x1", "x", 0.986495429526668),
            ("x2", "x", 0.984698236509496),
            ("y1", "x", 0.966968318944406),
            ("y2", "x", 0.990747409555462),
            ("x1", "y", 0.998328541823392),
            ("x2", "y", 0.938295591930478),
            ("y1", "y", 0.995439694533641),
            ("y2", "y", 0.995656444626453),
        ] {
            let actual = result
                .cross_loadings
                .iter()
                .find(|cell| cell.indicator == indicator && cell.construct == construct)
                .unwrap()
                .loading;
            assert_abs_diff_eq!(actual, expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn formative_metrics_are_not_applicable_and_r2_is_reused_exactly() {
        let (dataset, mut recipe) = fixture();
        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let result = assess_pls(&dataset, &recipe, &estimation).unwrap();
        assert_eq!(
            result.construct_quality[0],
            ConstructQuality {
                construct: "x".into(),
                cronbach_alpha: None,
                rho_c: None,
                ave: None,
                rho_a: None,
                rho_a_status: Some(RhoAStatus::NotApplicable),
                rho_a_reason: Some("rho_a.formative_not_applicable".into()),
                rho_a_warning_codes: Vec::new(),
                rho_a_indicator_count: Some(2),
                score_variance_before_normalization: None,
                normalized_weight_norm_squared: None,
                off_diagonal_numerator: None,
                off_diagonal_denominator: None,
            }
        );
        assert_eq!(result.fornell_larcker.values[0][0], None);
        let htmt_plus = result.htmt_plus.as_ref().unwrap();
        assert_eq!(htmt_plus.cells[0][1].status, HtmtStatus::NotApplicable);
        assert_eq!(htmt_plus.cells[0][1].value, None);
        assert_eq!(result.r_squared, estimation.r_squared);
        let indicator_correlation = correlation(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            &[2.0, 3.0, 5.0, 4.0, 6.0, 7.0],
        )
        .unwrap();
        let expected_vif = 1.0 / (1.0 - indicator_correlation.powi(2));
        assert_eq!(result.formative_indicator_vif.len(), 2);
        for value in &result.formative_indicator_vif {
            assert_abs_diff_eq!(value.vif.unwrap(), expected_vif, epsilon = 1e-10);
        }
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("formative"))
        );
    }

    #[test]
    fn rejects_dataset_and_result_mismatches() {
        let (dataset, mut recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        recipe.dataset_fingerprint = "different".into();
        assert_eq!(
            assess_pls(&dataset, &recipe, &estimation),
            Err(AssessmentError::DatasetMismatch)
        );
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let mut invalid = estimation;
        invalid.construct_scores.remove("x");
        assert!(matches!(
            assess_pls(&dataset, &recipe, &invalid),
            Err(AssessmentError::ResultMismatch(_))
        ));
    }

    #[test]
    fn execution_control_reports_progress_and_cancels_cross_loadings() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut progress = Vec::new();
        let result = assess_pls_with_control(&dataset, &recipe, &estimation, |update| {
            progress.push(update);
            update.phase != AssessmentPhase::CrossLoadings
        });
        assert_eq!(result, Err(AssessmentError::Cancelled));
        assert!(
            progress
                .iter()
                .any(|update| update.phase == AssessmentPhase::ConstructQuality)
        );
        assert_eq!(
            progress.last().unwrap().phase,
            AssessmentPhase::CrossLoadings
        );
    }

    #[test]
    fn execution_control_reaches_new_structural_diagnostic_phase() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut progress = Vec::new();
        let result = assess_pls_with_control(&dataset, &recipe, &estimation, |update| {
            progress.push(update);
            update.phase != AssessmentPhase::StructuralVif
        });
        assert_eq!(result, Err(AssessmentError::Cancelled));
        assert_eq!(
            progress.last().unwrap().phase,
            AssessmentPhase::StructuralVif
        );
        assert!(
            progress
                .iter()
                .any(|update| update.phase == AssessmentPhase::StructuralQuality)
        );
    }

    #[test]
    fn blindfolding_cancellation_and_no_valid_distance_are_explicit() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let cancelled = assess_pls_with_control(&dataset, &recipe, &estimation, |progress| {
            progress.phase != AssessmentPhase::Blindfolding
        });
        assert_eq!(cancelled, Err(AssessmentError::Cancelled));

        let short = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,2,1\n2,3,3,2\n3,5,4,4\n4,4,6,5\n5,6,7,7\n",
            "short.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut short_recipe = recipe;
        short_recipe.dataset_fingerprint = short.fingerprint.0.clone();
        let short_estimation = estimate_pls(&short, &short_recipe).unwrap();
        let result = assess_pls(&short, &short_recipe, &short_estimation).unwrap();
        assert!(result.blindfolding.is_none());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("no omission distance"))
        );
    }

    #[test]
    fn blindfolding_reports_monotonic_nested_estimation_progress() {
        let (dataset, recipe) = fixture();
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut updates = Vec::new();
        assess_pls_with_control(&dataset, &recipe, &estimation, |progress| {
            if progress.phase == AssessmentPhase::Blindfolding {
                updates.push((progress.completed_units, progress.total_units));
            }
            true
        })
        .unwrap();

        let expected_total = 5 * NESTED_PROGRESS_SCALE;
        assert!(updates.len() > 10);
        assert!(updates.iter().all(|(_, total)| *total == expected_total));
        assert!(updates.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert!(
            updates.iter().any(|(completed, _)| {
                *completed > 0 && *completed % NESTED_PROGRESS_SCALE != 0
            })
        );
        assert_eq!(updates.last().unwrap().0, expected_total);
    }

    #[test]
    fn nested_vif_progress_is_monotonic_with_a_stable_phase_total() {
        let (dataset, mut recipe) = fixture();
        recipe.model.constructs[0].mode = MeasurementMode::Formative;
        let estimation = estimate_pls(&dataset, &recipe).unwrap();
        let mut progress = Vec::new();
        assess_pls_with_control(&dataset, &recipe, &estimation, |update| {
            progress.push(update);
            true
        })
        .unwrap();
        for phase in [
            AssessmentPhase::StructuralVif,
            AssessmentPhase::FormativeVif,
        ] {
            let updates = progress
                .iter()
                .filter(|update| update.phase == phase)
                .collect::<Vec<_>>();
            assert!(!updates.is_empty());
            assert!(
                updates
                    .windows(2)
                    .all(|pair| pair[0].completed_units <= pair[1].completed_units)
            );
            assert!(
                updates
                    .iter()
                    .all(|update| update.total_units == updates[0].total_units)
            );
        }
    }
}
