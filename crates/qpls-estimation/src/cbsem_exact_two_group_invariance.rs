use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use qpls_core::{
    CompiledCbsemInputV2, CompiledCbsemPlanV2, MissingDataPolicyV4, ObservedRoleV4,
    ObservedScaleV4, SemCovarianceDenominatorV4, SemGroupV4, SemModelV4, SemVariableV4,
    compile_cbsem_plan_v2,
};
use qpls_data::{ColumnType, DataKind, Dataset, ScaleType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CbsemCompiledMomentErrorV2, CbsemExactParameterTableErrorV3, CbsemMomentInputKindV2,
    cbsem_exact_parameter_table::{
        CbsemExactTwoGroupJointFitV1, estimate_cbsem_ml_exact_two_group_joint_v1_with_control,
    },
    cbsem_matrix_input::{
        build_exact_two_group_cfa_projection_v1, covariance_ml_from_rows, covariance_sha256,
        ensure_strict_positive_definite, means_from_rows, numeric_cell, observed_means_sha256,
        validate_dataset_integrity,
    },
};

pub const CBSEM_EXACT_TWO_GROUP_INVARIANCE_METHOD_VERSION_V1: &str =
    "cbsem_exact_two_group_configural_metric_prerequisite_v1";
pub const CBSEM_EXACT_TWO_GROUP_ROW_INDEX_DIGEST_METHOD_V1: &str =
    "cbsem_exact_two_group_source_row_indices_v1";
pub const CBSEM_EXACT_TWO_GROUP_COMPLETE_ROWS_DIGEST_METHOD_V1: &str =
    "cbsem_exact_two_group_complete_rows_v1";
pub const CBSEM_EXACT_TWO_GROUP_COMBINED_AUTHORITY_DIGEST_METHOD_V1: &str =
    "cbsem_exact_two_group_combined_authority_v1";
pub const CBSEM_EXACT_TWO_GROUP_NEGATIVE_LRT_RELATIVE_TOLERANCE_V1: f64 = 1.0e-10;
pub const CBSEM_EXACT_TWO_GROUP_JOINT_OBJECTIVE_METHOD_VERSION_V1: &str =
    "cbsem_exact_two_group_joint_covariance_ml_ng_weighted_v1";
pub const CBSEM_EXACT_TWO_GROUP_MOMENT_METHOD_VERSION_V1: &str =
    "cbsem_exact_two_group_shared_listwise_groupwise_ml_n_moments_v1";
const MINIMUM_COMPLETE_OBSERVATIONS_PER_GROUP_V1: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactTwoGroupQualificationStatusV1 {
    EngineOnlyPrerequisiteNotProductQualified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactTwoGroupScalarKindV1 {
    Utf8,
    Float64,
    Int64,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactTwoGroupFitIndexUnavailableReasonV1 {
    NotFrozenForEnginePrerequisite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CbsemExactTwoGroupObservedMeansUsageV1 {
    PersistedForAuthorityNotConsumedByCovarianceStructureMl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CbsemExactTwoGroupFitIndexOutcomeV1 {
    Unavailable {
        reason: CbsemExactTwoGroupFitIndexUnavailableReasonV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupNullOmissionAuthorityV1 {
    pub count: usize,
    pub source_row_indices_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupGroupMomentAuthorityV1 {
    pub group_id: String,
    pub label: String,
    pub declared_value: String,
    pub canonical_value: String,
    pub selected_observations: usize,
    pub selected_source_row_indices_sha256: String,
    pub listwise_omitted_observations: usize,
    pub complete_observations: usize,
    pub complete_source_row_indices_sha256: String,
    pub complete_rows_sha256: String,
    pub canonical_ml_covariance_sha256: String,
    pub canonical_observed_means_sha256: String,
    pub weight: f64,
    pub covariance_ml: Vec<Vec<f64>>,
    pub observed_means: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupDataAuthorityV1 {
    pub row_index_digest_method: String,
    pub complete_rows_digest_method: String,
    pub combined_authority_digest_method: String,
    pub dataset_id: String,
    pub dataset_fingerprint: String,
    pub source_row_count: usize,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub grouping_variable_id: String,
    pub grouping_source_column: String,
    pub grouping_scalar_kind: CbsemExactTwoGroupScalarKindV1,
    pub input_kind: CbsemMomentInputKindV2,
    pub missing_data: MissingDataPolicyV4,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub observed_means_usage: CbsemExactTwoGroupObservedMeansUsageV1,
    pub ordered_indicator_variable_ids: Vec<String>,
    pub ordered_indicator_source_columns: Vec<String>,
    pub null_grouping_omissions: CbsemExactTwoGroupNullOmissionAuthorityV1,
    pub shared_complete_case_observations: usize,
    pub shared_complete_case_rows_sha256: String,
    pub groups: Vec<CbsemExactTwoGroupGroupMomentAuthorityV1>,
    pub combined_authority_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupParameterEstimateV1 {
    pub parameter_id: String,
    pub estimate: f64,
    pub fixed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupGroupFitV1 {
    pub group_id: String,
    pub sample_size: usize,
    pub weight: f64,
    pub objective: f64,
    pub chi_square: f64,
    pub parameters: Vec<CbsemExactTwoGroupParameterEstimateV1>,
    pub implied_covariance: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupModelFitV1 {
    pub model: String,
    pub estimator: String,
    pub exact_parameter_table_method_version: String,
    pub joint_objective_method_version: String,
    pub covariance_moment_method_version: String,
    pub objective_definition: String,
    pub input_kind: CbsemMomentInputKindV2,
    pub missing_data: MissingDataPolicyV4,
    pub covariance_denominator: SemCovarianceDenominatorV4,
    pub mean_structure: bool,
    pub observed_means_usage: CbsemExactTwoGroupObservedMeansUsageV1,
    pub constraint_scope: String,
    pub shared_complete_case_rows_sha256: String,
    pub combined_data_authority_sha256: String,
    pub plan_sha256: String,
    pub model_scientific_sha256: String,
    pub converged: bool,
    pub admissible: bool,
    pub initialization_iterations: u32,
    pub iterations: u32,
    pub gradient_norm: f64,
    pub free_dimensions: usize,
    pub observed_moments: usize,
    pub degrees_of_freedom: i64,
    pub objective: f64,
    pub chi_square: f64,
    pub groups: Vec<CbsemExactTwoGroupGroupFitV1>,
    pub cfi: CbsemExactTwoGroupFitIndexOutcomeV1,
    pub rmsea: CbsemExactTwoGroupFitIndexOutcomeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupNestingResultV1 {
    pub configural_free_dimensions: usize,
    pub metric_free_dimensions: usize,
    pub dimension_reduction: usize,
    pub raw_chi_square_difference: f64,
    pub negative_difference_tolerance: f64,
    pub likelihood_ratio_statistic: f64,
    pub delta_degrees_of_freedom: i64,
    pub p_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupInvarianceResultV1 {
    pub method_version: String,
    pub qualification_status: CbsemExactTwoGroupQualificationStatusV1,
    pub data: CbsemExactTwoGroupDataAuthorityV1,
    pub configural: CbsemExactTwoGroupModelFitV1,
    pub metric: CbsemExactTwoGroupModelFitV1,
    pub nesting: CbsemExactTwoGroupNestingResultV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemExactTwoGroupProgressV1 {
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CbsemExactTwoGroupInvarianceErrorV1 {
    #[error("resolved SemModelV4 is invalid: {0}")]
    InvalidModel(String),
    #[error(
        "supplied compiled CB-SEM plan does not exactly match the internally compiled resolved model"
    )]
    PlanIdentityMismatch,
    #[error("two-group exact prerequisite plan is unsupported: {0}")]
    UnsupportedPlan(String),
    #[error("live dataset id {actual} differs from compiled plan dataset id {expected}")]
    DatasetIdMismatch { expected: String, actual: String },
    #[error("grouping source column {0} is absent or ambiguous")]
    GroupingSourceColumn(String),
    #[error("grouping source column {0} is also an indicator source column")]
    GroupingSourceIsIndicator(String),
    #[error("group level {group_id} value {value:?} is invalid for scalar kind {kind:?}")]
    InvalidDeclaredGroupValue {
        group_id: String,
        value: String,
        kind: CbsemExactTwoGroupScalarKindV1,
    },
    #[error("declared group levels collapse to the same canonical value {0:?}")]
    DuplicateCanonicalGroupValue(String),
    #[error("source row {row} contains unmapped group value {value:?}")]
    UnmappedGroupValue { row: usize, value: String },
    #[error("source row {row} contains an invalid group scalar: {reason}")]
    InvalidGroupScalar { row: usize, reason: String },
    #[error("group {group_id} has {found} complete observations; at least {minimum} are required")]
    InsufficientGroupObservations {
        group_id: String,
        found: usize,
        minimum: usize,
    },
    #[error("configural-to-metric loading constraints did not reduce dimension")]
    NoMetricDimensionReduction,
    #[error("two-group model has negative degrees of freedom")]
    NegativeDegreesOfFreedom,
    #[error("metric chi-square difference {difference} is below negative tolerance {tolerance}")]
    NegativeChiSquareDifference { difference: f64, tolerance: f64 },
    #[error("two-group chi-square upper-tail evaluation failed")]
    ChiSquareTail,
    #[error("two-group data-authority receipt failed integrity validation: {0}")]
    AuthorityIntegrity(String),
    #[error(transparent)]
    Moment(#[from] CbsemCompiledMomentErrorV2),
    #[error(transparent)]
    Exact(#[from] CbsemExactParameterTableErrorV3),
}

#[derive(Debug)]
struct PreparedGroupV1 {
    group_id: String,
    label: String,
    declared_value: String,
    canonical_value: String,
    selected_source_rows: Vec<usize>,
    complete_source_rows: Vec<usize>,
    complete_rows: Vec<Vec<f64>>,
    covariance_ml: Vec<Vec<f64>>,
    observed_means: Vec<f64>,
}

pub fn estimate_cbsem_ml_exact_two_group_configural_metric_v1(
    dataset: &Dataset,
    supplied_plan: &CompiledCbsemPlanV2,
    resolved_model: &SemModelV4,
) -> Result<CbsemExactTwoGroupInvarianceResultV1, CbsemExactTwoGroupInvarianceErrorV1> {
    estimate_cbsem_ml_exact_two_group_configural_metric_v1_with_control(
        dataset,
        supplied_plan,
        resolved_model,
        || false,
        |_| {},
    )
}

pub fn estimate_cbsem_ml_exact_two_group_configural_metric_v1_with_control(
    dataset: &Dataset,
    supplied_plan: &CompiledCbsemPlanV2,
    resolved_model: &SemModelV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemExactTwoGroupProgressV1) + Sync,
) -> Result<CbsemExactTwoGroupInvarianceResultV1, CbsemExactTwoGroupInvarianceErrorV1> {
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled.into());
    }
    progress(CbsemExactTwoGroupProgressV1 {
        phase: "integrity".into(),
        completed_units: 0,
        total_units: 1,
    });
    resolved_model
        .ensure_valid()
        .map_err(|error| CbsemExactTwoGroupInvarianceErrorV1::InvalidModel(error.to_string()))?;
    let plan = compile_cbsem_plan_v2(resolved_model)
        .map_err(|error| CbsemExactTwoGroupInvarianceErrorV1::InvalidModel(error.to_string()))?;
    if plan.deterministic_sha256() != supplied_plan.deterministic_sha256()
        || plan.scientific_hash() != supplied_plan.scientific_hash()
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::PlanIdentityMismatch);
    }
    validate_dataset_integrity(dataset)?;
    let actual_dataset_id = dataset.id.to_string();
    if plan.input().dataset_id() != actual_dataset_id {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::DatasetIdMismatch {
            expected: plan.input().dataset_id().into(),
            actual: actual_dataset_id,
        });
    }
    validate_fixed_scope(&plan, dataset)?;
    let (grouping_variable, levels) = match plan.group() {
        SemGroupV4::ObservedGroups {
            grouping_variable,
            levels,
        } if levels.len() == 2 => (grouping_variable.as_str(), levels.as_slice()),
        _ => {
            return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
                "exactly two declared observed groups are required".into(),
            ));
        }
    };
    let (
        grouping_source_column,
        grouping_scale,
        grouping_role,
        grouping_has_missing_markers,
        grouping_has_transformation_lineage,
    ) = plan
        .variables()
        .iter()
        .find_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                scale,
                role,
                missing_markers,
                transformation_lineage,
                ..
            } if id == grouping_variable => Some((
                source_column.clone(),
                *scale,
                role.clone(),
                !missing_markers.is_empty(),
                !transformation_lineage.is_empty(),
            )),
            _ => None,
        })
        .ok_or_else(|| {
            CbsemExactTwoGroupInvarianceErrorV1::GroupingSourceColumn(grouping_variable.into())
        })?;
    let projection = build_exact_two_group_cfa_projection_v1(&plan, grouping_variable)?;
    if projection
        .indicator_names
        .iter()
        .any(|source| source == &grouping_source_column)
    {
        return Err(
            CbsemExactTwoGroupInvarianceErrorV1::GroupingSourceIsIndicator(grouping_source_column),
        );
    }
    let ordered_indicator_variable_ids = ordered_indicator_ids(&plan, &projection.indicator_names)?;
    let (group_position, scalar_kind) = resolve_grouping_column(
        dataset,
        &grouping_source_column,
        grouping_scale,
        &grouping_role,
        grouping_has_missing_markers,
        grouping_has_transformation_lineage,
    )?;
    let indicator_positions = resolve_indicator_columns(dataset, &projection.indicator_names)?;
    let group_array = dataset.batch.column(group_position).as_ref();
    let mut canonical_to_group = BTreeMap::<String, usize>::new();
    let mut prepared = Vec::with_capacity(2);
    for (index, level) in levels.iter().enumerate() {
        let canonical =
            canonicalize_declared_group_value(&level.value, &scalar_kind).ok_or_else(|| {
                CbsemExactTwoGroupInvarianceErrorV1::InvalidDeclaredGroupValue {
                    group_id: level.id.clone(),
                    value: level.value.clone(),
                    kind: scalar_kind.clone(),
                }
            })?;
        if canonical_to_group
            .insert(canonical.clone(), index)
            .is_some()
        {
            return Err(
                CbsemExactTwoGroupInvarianceErrorV1::DuplicateCanonicalGroupValue(canonical),
            );
        }
        prepared.push(PreparedGroupV1 {
            group_id: level.id.clone(),
            label: level.label.clone(),
            declared_value: level.value.clone(),
            canonical_value: canonical,
            selected_source_rows: Vec::new(),
            complete_source_rows: Vec::new(),
            complete_rows: Vec::new(),
            covariance_ml: Vec::new(),
            observed_means: Vec::new(),
        });
    }
    let mut null_group_rows = Vec::new();
    for row in 0..dataset.batch.num_rows() {
        if group_array.is_null(row) {
            null_group_rows.push(row);
            continue;
        }
        let canonical =
            canonical_group_scalar(group_array, row, &scalar_kind).map_err(|reason| {
                CbsemExactTwoGroupInvarianceErrorV1::InvalidGroupScalar { row, reason }
            })?;
        let Some(group) = canonical_to_group.get(&canonical).copied() else {
            return Err(CbsemExactTwoGroupInvarianceErrorV1::UnmappedGroupValue {
                row,
                value: canonical,
            });
        };
        prepared[group].selected_source_rows.push(row);
        if indicator_positions
            .iter()
            .any(|position| dataset.batch.column(*position).is_null(row))
        {
            continue;
        }
        let mut values = Vec::with_capacity(indicator_positions.len());
        for (column, position) in indicator_positions.iter().enumerate() {
            let value = numeric_cell(dataset.batch.column(*position).as_ref(), row)
                .ok_or(CbsemCompiledMomentErrorV2::MatrixCellInvalid { row, column })?;
            if !value.is_finite() {
                return Err(CbsemCompiledMomentErrorV2::RawValueNonFinite {
                    column: projection.indicator_names[column].clone(),
                    row,
                }
                .into());
            }
            values.push(value);
        }
        prepared[group].complete_source_rows.push(row);
        prepared[group].complete_rows.push(values);
    }
    for group in &mut prepared {
        if group.complete_rows.len() < MINIMUM_COMPLETE_OBSERVATIONS_PER_GROUP_V1 {
            return Err(
                CbsemExactTwoGroupInvarianceErrorV1::InsufficientGroupObservations {
                    group_id: group.group_id.clone(),
                    found: group.complete_rows.len(),
                    minimum: MINIMUM_COMPLETE_OBSERVATIONS_PER_GROUP_V1,
                },
            );
        }
        group.covariance_ml = covariance_ml_from_rows(&group.complete_rows);
        ensure_strict_positive_definite(&group.covariance_ml)?;
        group.observed_means = means_from_rows(&group.complete_rows);
    }
    let sample_sizes = [
        prepared[0].complete_rows.len(),
        prepared[1].complete_rows.len(),
    ];
    let total_sample_size = sample_sizes[0] + sample_sizes[1];
    let covariance_refs = [
        prepared[0].covariance_ml.as_slice(),
        prepared[1].covariance_ml.as_slice(),
    ];
    let configural_progress = |completed_units, total_units| {
        progress(CbsemExactTwoGroupProgressV1 {
            phase: "configural_joint_ml".into(),
            completed_units,
            total_units,
        });
    };
    let configural = estimate_cbsem_ml_exact_two_group_joint_v1_with_control(
        &projection.model,
        &projection.indicator_names,
        covariance_refs,
        sample_sizes,
        &projection.parameter_rows,
        false,
        &should_cancel,
        &configural_progress,
    )?;
    let metric_progress = |completed_units, total_units| {
        progress(CbsemExactTwoGroupProgressV1 {
            phase: "metric_joint_ml".into(),
            completed_units,
            total_units,
        });
    };
    let metric = estimate_cbsem_ml_exact_two_group_joint_v1_with_control(
        &projection.model,
        &projection.indicator_names,
        covariance_refs,
        sample_sizes,
        &projection.parameter_rows,
        true,
        &should_cancel,
        &metric_progress,
    )?;
    progress(CbsemExactTwoGroupProgressV1 {
        phase: "metric_complete".into(),
        completed_units: 1,
        total_units: 1,
    });
    if should_cancel() {
        return Err(CbsemExactParameterTableErrorV3::Cancelled.into());
    }
    let covariance_moments =
        projection.indicator_names.len() * (projection.indicator_names.len() + 1) / 2;
    let observed_moments = 2 * covariance_moments;
    if configural.dimension_count > observed_moments || metric.dimension_count > observed_moments {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::NegativeDegreesOfFreedom);
    }
    let dimension_reduction = configural
        .dimension_count
        .checked_sub(metric.dimension_count)
        .filter(|reduction| *reduction > 0)
        .ok_or(CbsemExactTwoGroupInvarianceErrorV1::NoMetricDimensionReduction)?;
    let data = build_data_authority(
        dataset,
        &plan,
        grouping_variable,
        &grouping_source_column,
        scalar_kind,
        &ordered_indicator_variable_ids,
        &projection.indicator_names,
        &null_group_rows,
        &prepared,
    );
    validate_cbsem_exact_two_group_data_authority_v1(&data)?;
    let configural = model_fit_receipt(
        "configural",
        &configural,
        observed_moments,
        sample_sizes,
        total_sample_size,
        &data,
    )?;
    let metric = model_fit_receipt(
        "metric_loadings",
        &metric,
        observed_moments,
        sample_sizes,
        total_sample_size,
        &data,
    )?;
    let (raw_difference, negative_tolerance, statistic) =
        exact_nested_lrt_statistic(configural.chi_square, metric.chi_square)?;
    let delta_df = metric.degrees_of_freedom - configural.degrees_of_freedom;
    if delta_df != dimension_reduction as i64 || delta_df <= 0 {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::NoMetricDimensionReduction);
    }
    let p_value = exact_chi_square_upper_tail(statistic, delta_df)?;
    let configural_free_dimensions = configural.free_dimensions;
    let metric_free_dimensions = metric.free_dimensions;
    Ok(CbsemExactTwoGroupInvarianceResultV1 {
        method_version: CBSEM_EXACT_TWO_GROUP_INVARIANCE_METHOD_VERSION_V1.into(),
        qualification_status:
            CbsemExactTwoGroupQualificationStatusV1::EngineOnlyPrerequisiteNotProductQualified,
        data,
        configural,
        metric,
        nesting: CbsemExactTwoGroupNestingResultV1 {
            configural_free_dimensions,
            metric_free_dimensions,
            dimension_reduction,
            raw_chi_square_difference: raw_difference,
            negative_difference_tolerance: negative_tolerance,
            likelihood_ratio_statistic: statistic,
            delta_degrees_of_freedom: delta_df,
            p_value,
        },
    })
}

fn exact_nested_lrt_statistic(
    configural_chi_square: f64,
    metric_chi_square: f64,
) -> Result<(f64, f64, f64), CbsemExactTwoGroupInvarianceErrorV1> {
    if !configural_chi_square.is_finite()
        || !metric_chi_square.is_finite()
        || configural_chi_square < 0.0
        || metric_chi_square < 0.0
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::ChiSquareTail);
    }
    let difference = metric_chi_square - configural_chi_square;
    let tolerance = CBSEM_EXACT_TWO_GROUP_NEGATIVE_LRT_RELATIVE_TOLERANCE_V1
        * metric_chi_square
            .abs()
            .max(configural_chi_square.abs())
            .max(1.0);
    if difference < -tolerance {
        return Err(
            CbsemExactTwoGroupInvarianceErrorV1::NegativeChiSquareDifference {
                difference,
                tolerance,
            },
        );
    }
    Ok((difference, tolerance, difference.max(0.0)))
}

fn exact_chi_square_upper_tail(
    statistic: f64,
    degrees_of_freedom: i64,
) -> Result<f64, CbsemExactTwoGroupInvarianceErrorV1> {
    if !statistic.is_finite() || statistic < 0.0 || degrees_of_freedom <= 0 {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::ChiSquareTail);
    }
    let distribution = ChiSquared::new(degrees_of_freedom as f64)
        .map_err(|_| CbsemExactTwoGroupInvarianceErrorV1::ChiSquareTail)?;
    let p_value = distribution.sf(statistic);
    if p_value.is_finite() && (0.0..=1.0).contains(&p_value) {
        Ok(p_value)
    } else {
        Err(CbsemExactTwoGroupInvarianceErrorV1::ChiSquareTail)
    }
}

fn validate_fixed_scope(
    plan: &CompiledCbsemPlanV2,
    dataset: &Dataset,
) -> Result<(), CbsemExactTwoGroupInvarianceErrorV1> {
    if dataset.schema.kind != DataKind::Raw || dataset.schema.sample_size.is_some() {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "only live raw observations without declared matrix sample size are accepted".into(),
        ));
    }
    match plan.input() {
        CompiledCbsemInputV2::Raw {
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
            ..
        } => {}
        _ => {
            return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
                "requires raw listwise ML without weights, clusters, or strata".into(),
            ));
        }
    }
    if !plan.regressions().is_empty() {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "the isolated two-group prerequisite is covariance-structure CFA only".into(),
        ));
    }
    if plan
        .parameters()
        .iter()
        .any(|parameter| !parameter.group_overrides().is_empty())
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "group overrides and partial releases are not accepted".into(),
        ));
    }
    Ok(())
}

fn ordered_indicator_ids(
    plan: &CompiledCbsemPlanV2,
    source_columns: &[String],
) -> Result<Vec<String>, CbsemExactTwoGroupInvarianceErrorV1> {
    let source_to_id = plan
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((source_column.as_str(), id.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    source_columns
        .iter()
        .map(|source| {
            source_to_id
                .get(source.as_str())
                .map(|id| (*id).into())
                .ok_or_else(|| {
                    CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(format!(
                        "indicator source {source} has no unique observed-variable identity"
                    ))
                })
        })
        .collect()
}

fn resolve_indicator_columns(
    dataset: &Dataset,
    source_columns: &[String],
) -> Result<Vec<usize>, CbsemExactTwoGroupInvarianceErrorV1> {
    let batch_schema = dataset.batch.schema();
    source_columns
        .iter()
        .map(|source_column| {
            let metadata_matches = dataset
                .schema
                .columns
                .iter()
                .enumerate()
                .filter(|(_, column)| column.name == *source_column)
                .collect::<Vec<_>>();
            let field_matches = batch_schema
                .fields()
                .iter()
                .enumerate()
                .filter(|(_, field)| field.name() == source_column)
                .collect::<Vec<_>>();
            if metadata_matches.len() != 1
                || field_matches.len() != 1
                || metadata_matches[0].0 != field_matches[0].0
            {
                return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
                    format!(
                        "indicator source column {source_column} is absent, duplicated, or schema/batch ambiguous"
                    ),
                ));
            }
            let (position, metadata) = metadata_matches[0];
            let array = dataset.batch.column(position).as_ref();
            if metadata.column_type != ColumnType::Numeric
                || metadata.scale_type != ScaleType::Continuous
                || !(array.as_any().is::<Float64Array>() || array.as_any().is::<Int64Array>())
            {
                return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
                    format!(
                        "indicator source column {source_column} must bind one numeric continuous Float64/Int64 Arrow column"
                    ),
                ));
            }
            Ok(position)
        })
        .collect()
}

fn resolve_grouping_column(
    dataset: &Dataset,
    source_column: &str,
    model_scale: ObservedScaleV4,
    model_role: &ObservedRoleV4,
    has_missing_markers: bool,
    has_transformation_lineage: bool,
) -> Result<(usize, CbsemExactTwoGroupScalarKindV1), CbsemExactTwoGroupInvarianceErrorV1> {
    if *model_role != ObservedRoleV4::Control
        || !matches!(
            model_scale,
            ObservedScaleV4::Nominal | ObservedScaleV4::Binary | ObservedScaleV4::Identifier
        )
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "grouping variable must be an observed control with nominal, binary, or identifier scale"
                .into(),
        ));
    }
    if has_missing_markers || has_transformation_lineage {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "grouping-variable missing markers and transformation lineage must be empty in this bounded raw slice"
                .into(),
        ));
    }
    let schema_matches = dataset
        .schema
        .columns
        .iter()
        .enumerate()
        .filter(|(_, column)| column.name == source_column)
        .collect::<Vec<_>>();
    let batch_schema = dataset.batch.schema();
    let batch_matches = batch_schema
        .fields()
        .iter()
        .enumerate()
        .filter(|(_, field)| field.name() == source_column)
        .collect::<Vec<_>>();
    if schema_matches.len() != 1
        || batch_matches.len() != 1
        || schema_matches[0].0 != batch_matches[0].0
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::GroupingSourceColumn(
            source_column.into(),
        ));
    }
    let (position, metadata) = schema_matches[0];
    let array = dataset.batch.column(position).as_ref();
    let scalar_kind = group_scalar_kind(array).ok_or_else(|| {
        CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "grouping column must be Arrow UTF-8, float64, int64, or boolean".into(),
        )
    })?;
    let metadata_type_matches = matches!(
        (&scalar_kind, metadata.column_type),
        (CbsemExactTwoGroupScalarKindV1::Utf8, ColumnType::Text)
            | (
                CbsemExactTwoGroupScalarKindV1::Float64 | CbsemExactTwoGroupScalarKindV1::Int64,
                ColumnType::Numeric
            )
            | (CbsemExactTwoGroupScalarKindV1::Boolean, ColumnType::Boolean)
    );
    let metadata_scale_matches = matches!(
        (model_scale, metadata.scale_type),
        (ObservedScaleV4::Nominal, ScaleType::Nominal)
            | (ObservedScaleV4::Binary, ScaleType::Binary)
            | (ObservedScaleV4::Identifier, ScaleType::Identifier)
    );
    if !metadata_type_matches || !metadata_scale_matches {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "grouping model scale, dataset metadata, and Arrow scalar type must match exactly"
                .into(),
        ));
    }
    Ok((position, scalar_kind))
}

fn group_scalar_kind(array: &dyn Array) -> Option<CbsemExactTwoGroupScalarKindV1> {
    if array.as_any().is::<StringArray>() {
        Some(CbsemExactTwoGroupScalarKindV1::Utf8)
    } else if array.as_any().is::<Float64Array>() {
        Some(CbsemExactTwoGroupScalarKindV1::Float64)
    } else if array.as_any().is::<Int64Array>() {
        Some(CbsemExactTwoGroupScalarKindV1::Int64)
    } else if array.as_any().is::<BooleanArray>() {
        Some(CbsemExactTwoGroupScalarKindV1::Boolean)
    } else {
        None
    }
}

fn canonicalize_declared_group_value(
    value: &str,
    kind: &CbsemExactTwoGroupScalarKindV1,
) -> Option<String> {
    match kind {
        CbsemExactTwoGroupScalarKindV1::Utf8 => Some(value.to_owned()),
        CbsemExactTwoGroupScalarKindV1::Float64 => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(canonical_f64),
        CbsemExactTwoGroupScalarKindV1::Int64 => {
            value.parse::<i64>().ok().map(|value| value.to_string())
        }
        CbsemExactTwoGroupScalarKindV1::Boolean => match value {
            "true" => Some("true".into()),
            "false" => Some("false".into()),
            _ => None,
        },
    }
}

fn canonical_group_scalar(
    array: &dyn Array,
    row: usize,
    kind: &CbsemExactTwoGroupScalarKindV1,
) -> Result<String, String> {
    match kind {
        CbsemExactTwoGroupScalarKindV1::Utf8 => array
            .as_any()
            .downcast_ref::<StringArray>()
            .map(|values| values.value(row).to_owned()),
        CbsemExactTwoGroupScalarKindV1::Float64 => array
            .as_any()
            .downcast_ref::<Float64Array>()
            .and_then(|values| {
                let value = values.value(row);
                value.is_finite().then(|| canonical_f64(value))
            }),
        CbsemExactTwoGroupScalarKindV1::Int64 => array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|values| values.value(row).to_string()),
        CbsemExactTwoGroupScalarKindV1::Boolean => array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .map(|values| values.value(row).to_string()),
    }
    .ok_or_else(|| "group scalar type/value is inconsistent with its canonical kind".into())
}

fn canonical_f64(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn build_data_authority(
    dataset: &Dataset,
    plan: &CompiledCbsemPlanV2,
    grouping_variable: &str,
    grouping_source_column: &str,
    scalar_kind: CbsemExactTwoGroupScalarKindV1,
    indicator_variable_ids: &[String],
    indicator_source_columns: &[String],
    null_group_rows: &[usize],
    prepared: &[PreparedGroupV1],
) -> CbsemExactTwoGroupDataAuthorityV1 {
    let total_complete = prepared
        .iter()
        .map(|group| group.complete_rows.len())
        .sum::<usize>();
    let groups = prepared
        .iter()
        .map(|group| {
            let sample_size = group.complete_rows.len();
            CbsemExactTwoGroupGroupMomentAuthorityV1 {
                group_id: group.group_id.clone(),
                label: group.label.clone(),
                declared_value: group.declared_value.clone(),
                canonical_value: group.canonical_value.clone(),
                selected_observations: group.selected_source_rows.len(),
                selected_source_row_indices_sha256: row_indices_digest(
                    b"selected\0",
                    &group.group_id,
                    &group.selected_source_rows,
                ),
                listwise_omitted_observations: group.selected_source_rows.len() - sample_size,
                complete_observations: sample_size,
                complete_source_row_indices_sha256: row_indices_digest(
                    b"complete\0",
                    &group.group_id,
                    &group.complete_source_rows,
                ),
                complete_rows_sha256: complete_rows_digest(group),
                canonical_ml_covariance_sha256: covariance_sha256(
                    CbsemMomentInputKindV2::Raw,
                    sample_size,
                    indicator_variable_ids,
                    SemCovarianceDenominatorV4::MaximumLikelihoodN,
                    &group.covariance_ml,
                ),
                canonical_observed_means_sha256: observed_means_sha256(
                    sample_size,
                    indicator_variable_ids,
                    &group.observed_means,
                ),
                weight: sample_size as f64 / total_complete as f64,
                covariance_ml: group.covariance_ml.clone(),
                observed_means: group.observed_means.clone(),
            }
        })
        .collect::<Vec<_>>();
    let shared_complete_case_rows_sha256 = shared_complete_rows_digest(prepared);
    let mut authority = CbsemExactTwoGroupDataAuthorityV1 {
        row_index_digest_method: CBSEM_EXACT_TWO_GROUP_ROW_INDEX_DIGEST_METHOD_V1.into(),
        complete_rows_digest_method: CBSEM_EXACT_TWO_GROUP_COMPLETE_ROWS_DIGEST_METHOD_V1.into(),
        combined_authority_digest_method:
            CBSEM_EXACT_TWO_GROUP_COMBINED_AUTHORITY_DIGEST_METHOD_V1.into(),
        dataset_id: dataset.id.to_string(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        source_row_count: dataset.batch.num_rows(),
        plan_sha256: plan.deterministic_sha256(),
        model_scientific_sha256: plan.scientific_hash().into(),
        grouping_variable_id: grouping_variable.into(),
        grouping_source_column: grouping_source_column.into(),
        grouping_scalar_kind: scalar_kind,
        input_kind: CbsemMomentInputKindV2::Raw,
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
        observed_means_usage:
            CbsemExactTwoGroupObservedMeansUsageV1::PersistedForAuthorityNotConsumedByCovarianceStructureMl,
        ordered_indicator_variable_ids: indicator_variable_ids.to_vec(),
        ordered_indicator_source_columns: indicator_source_columns.to_vec(),
        null_grouping_omissions: CbsemExactTwoGroupNullOmissionAuthorityV1 {
            count: null_group_rows.len(),
            source_row_indices_sha256: row_indices_digest(
                b"null_group\0",
                grouping_variable,
                null_group_rows,
            ),
        },
        shared_complete_case_observations: total_complete,
        shared_complete_case_rows_sha256,
        groups,
        combined_authority_sha256: String::new(),
    };
    authority.combined_authority_sha256 = combined_authority_digest(&authority);
    authority
}

fn row_indices_digest(domain: &[u8], group_id: &str, rows: &[usize]) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_TWO_GROUP_ROW_INDEX_DIGEST_METHOD_V1.as_bytes());
    digest.update(domain);
    update_string(&mut digest, group_id);
    digest.update((rows.len() as u64).to_le_bytes());
    for row in rows {
        digest.update((*row as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn complete_rows_digest(group: &PreparedGroupV1) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_TWO_GROUP_COMPLETE_ROWS_DIGEST_METHOD_V1.as_bytes());
    digest.update(b"group\0");
    update_string(&mut digest, &group.group_id);
    digest.update((group.complete_rows.len() as u64).to_le_bytes());
    for (source_row, values) in group.complete_source_rows.iter().zip(&group.complete_rows) {
        digest.update((*source_row as u64).to_le_bytes());
        for value in values {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn shared_complete_rows_digest(groups: &[PreparedGroupV1]) -> String {
    let mut rows = groups
        .iter()
        .flat_map(|group| {
            group
                .complete_source_rows
                .iter()
                .copied()
                .zip(group.complete_rows.iter())
                .map(move |(source_row, values)| (source_row, group.group_id.as_str(), values))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(source_row, _, _)| *source_row);
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_TWO_GROUP_COMPLETE_ROWS_DIGEST_METHOD_V1.as_bytes());
    digest.update(b"shared\0");
    digest.update((rows.len() as u64).to_le_bytes());
    for (source_row, group_id, values) in rows {
        digest.update((source_row as u64).to_le_bytes());
        update_string(&mut digest, group_id);
        for value in values {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn combined_authority_digest(authority: &CbsemExactTwoGroupDataAuthorityV1) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_TWO_GROUP_COMBINED_AUTHORITY_DIGEST_METHOD_V1.as_bytes());
    for value in [
        authority.row_index_digest_method.as_str(),
        authority.complete_rows_digest_method.as_str(),
        authority.combined_authority_digest_method.as_str(),
        authority.dataset_id.as_str(),
        authority.dataset_fingerprint.as_str(),
        authority.plan_sha256.as_str(),
        authority.model_scientific_sha256.as_str(),
        authority.grouping_variable_id.as_str(),
        authority.grouping_source_column.as_str(),
        authority
            .null_grouping_omissions
            .source_row_indices_sha256
            .as_str(),
        authority.shared_complete_case_rows_sha256.as_str(),
    ] {
        update_string(&mut digest, value);
    }
    update_string(
        &mut digest,
        match authority.grouping_scalar_kind {
            CbsemExactTwoGroupScalarKindV1::Utf8 => "utf8",
            CbsemExactTwoGroupScalarKindV1::Float64 => "float64",
            CbsemExactTwoGroupScalarKindV1::Int64 => "int64",
            CbsemExactTwoGroupScalarKindV1::Boolean => "boolean",
        },
    );
    update_string(
        &mut digest,
        match authority.input_kind {
            CbsemMomentInputKindV2::Raw => "raw",
            CbsemMomentInputKindV2::Covariance => "covariance",
            CbsemMomentInputKindV2::Correlation => "correlation",
        },
    );
    match &authority.missing_data {
        MissingDataPolicyV4::ListwiseDeletion => update_string(&mut digest, "listwise_deletion"),
        MissingDataPolicyV4::PairwiseDeletion => update_string(&mut digest, "pairwise_deletion"),
        MissingDataPolicyV4::MeanReplacement => update_string(&mut digest, "mean_replacement"),
        MissingDataPolicyV4::FullInformationMaximumLikelihood => {
            update_string(&mut digest, "full_information_maximum_likelihood")
        }
        MissingDataPolicyV4::MultipleImputation { imputations } => {
            update_string(&mut digest, "multiple_imputation");
            digest.update(imputations.to_le_bytes());
        }
    }
    update_string(
        &mut digest,
        match authority.covariance_denominator {
            SemCovarianceDenominatorV4::SampleNMinusOne => "sample_n_minus_one",
            SemCovarianceDenominatorV4::MaximumLikelihoodN => "maximum_likelihood_n",
        },
    );
    update_string(
        &mut digest,
        match authority.observed_means_usage {
            CbsemExactTwoGroupObservedMeansUsageV1::PersistedForAuthorityNotConsumedByCovarianceStructureMl => {
                "persisted_for_authority_not_consumed_by_covariance_structure_ml"
            }
        },
    );
    digest.update((authority.source_row_count as u64).to_le_bytes());
    digest.update((authority.shared_complete_case_observations as u64).to_le_bytes());
    digest.update((authority.null_grouping_omissions.count as u64).to_le_bytes());
    digest.update(b"indicator_variable_ids\0");
    digest.update((authority.ordered_indicator_variable_ids.len() as u64).to_le_bytes());
    for value in &authority.ordered_indicator_variable_ids {
        update_string(&mut digest, value);
    }
    digest.update(b"indicator_source_columns\0");
    digest.update((authority.ordered_indicator_source_columns.len() as u64).to_le_bytes());
    for value in &authority.ordered_indicator_source_columns {
        update_string(&mut digest, value);
    }
    digest.update(b"groups\0");
    digest.update((authority.groups.len() as u64).to_le_bytes());
    for (group_index, group) in authority.groups.iter().enumerate() {
        digest.update((group_index as u64).to_le_bytes());
        for value in [
            group.group_id.as_str(),
            group.label.as_str(),
            group.declared_value.as_str(),
            group.canonical_value.as_str(),
            group.selected_source_row_indices_sha256.as_str(),
            group.complete_source_row_indices_sha256.as_str(),
            group.complete_rows_sha256.as_str(),
            group.canonical_ml_covariance_sha256.as_str(),
            group.canonical_observed_means_sha256.as_str(),
        ] {
            update_string(&mut digest, value);
        }
        digest.update((group.selected_observations as u64).to_le_bytes());
        digest.update((group.listwise_omitted_observations as u64).to_le_bytes());
        digest.update((group.complete_observations as u64).to_le_bytes());
        digest.update(group.weight.to_bits().to_le_bytes());
        digest.update((group.covariance_ml.len() as u64).to_le_bytes());
        for row in &group.covariance_ml {
            digest.update((row.len() as u64).to_le_bytes());
            for value in row {
                digest.update(value.to_bits().to_le_bytes());
            }
        }
        digest.update((group.observed_means.len() as u64).to_le_bytes());
        for value in &group.observed_means {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn update_string(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value.as_bytes());
}

pub fn validate_cbsem_exact_two_group_data_authority_v1(
    authority: &CbsemExactTwoGroupDataAuthorityV1,
) -> Result<(), CbsemExactTwoGroupInvarianceErrorV1> {
    let invalid =
        |message: &str| CbsemExactTwoGroupInvarianceErrorV1::AuthorityIntegrity(message.into());
    if authority.row_index_digest_method != CBSEM_EXACT_TWO_GROUP_ROW_INDEX_DIGEST_METHOD_V1
        || authority.complete_rows_digest_method
            != CBSEM_EXACT_TWO_GROUP_COMPLETE_ROWS_DIGEST_METHOD_V1
        || authority.combined_authority_digest_method
            != CBSEM_EXACT_TWO_GROUP_COMBINED_AUTHORITY_DIGEST_METHOD_V1
        || authority.input_kind != CbsemMomentInputKindV2::Raw
        || authority.missing_data != MissingDataPolicyV4::ListwiseDeletion
        || authority.covariance_denominator
            != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || authority.observed_means_usage
            != CbsemExactTwoGroupObservedMeansUsageV1::PersistedForAuthorityNotConsumedByCovarianceStructureMl
    {
        return Err(invalid("method, input, missing-data, denominator, or means-use contract differs"));
    }
    if authority.dataset_id.is_empty()
        || authority.grouping_variable_id.is_empty()
        || authority.grouping_source_column.is_empty()
        || authority
            .ordered_indicator_variable_ids
            .iter()
            .any(|value| value.is_empty())
        || authority
            .ordered_indicator_source_columns
            .iter()
            .any(|value| value.is_empty())
        || !canonical_dataset_fingerprint(&authority.dataset_fingerprint)
    {
        return Err(invalid(
            "dataset, grouping, indicator, or fingerprint identity is invalid",
        ));
    }
    let indicators = authority.ordered_indicator_variable_ids.len();
    if indicators == 0
        || authority.ordered_indicator_source_columns.len() != indicators
        || authority
            .ordered_indicator_variable_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != indicators
        || authority
            .ordered_indicator_source_columns
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != indicators
        || authority.groups.len() != 2
        || authority.groups[0].group_id >= authority.groups[1].group_id
        || authority.groups[0].canonical_value == authority.groups[1].canonical_value
    {
        return Err(invalid(
            "ordered indicator or group identity surface is invalid",
        ));
    }
    let selected = authority
        .groups
        .iter()
        .map(|group| group.selected_observations)
        .sum::<usize>();
    let complete = authority
        .groups
        .iter()
        .map(|group| group.complete_observations)
        .sum::<usize>();
    if selected + authority.null_grouping_omissions.count != authority.source_row_count
        || complete != authority.shared_complete_case_observations
        || authority.shared_complete_case_observations
            < 2 * MINIMUM_COMPLETE_OBSERVATIONS_PER_GROUP_V1
    {
        return Err(invalid(
            "source, selected, null, or complete-case counts do not partition",
        ));
    }
    for group in &authority.groups {
        if group.group_id.is_empty()
            || group.label.is_empty()
            || group.declared_value.is_empty()
            || group.canonical_value.is_empty()
            || group.complete_observations < MINIMUM_COMPLETE_OBSERVATIONS_PER_GROUP_V1
            || group.selected_observations
                != group.complete_observations + group.listwise_omitted_observations
            || group.covariance_ml.len() != indicators
            || group
                .covariance_ml
                .iter()
                .any(|row| row.len() != indicators || row.iter().any(|value| !value.is_finite()))
            || group.observed_means.len() != indicators
            || group.observed_means.iter().any(|value| !value.is_finite())
            || !group.weight.is_finite()
            || !(0.0..1.0).contains(&group.weight)
        {
            return Err(invalid(
                "group count or persisted moment surface is invalid",
            ));
        }
        ensure_strict_positive_definite(&group.covariance_ml)?;
        let expected_covariance = covariance_sha256(
            CbsemMomentInputKindV2::Raw,
            group.complete_observations,
            &authority.ordered_indicator_variable_ids,
            SemCovarianceDenominatorV4::MaximumLikelihoodN,
            &group.covariance_ml,
        );
        let expected_means = observed_means_sha256(
            group.complete_observations,
            &authority.ordered_indicator_variable_ids,
            &group.observed_means,
        );
        let expected_weight =
            group.complete_observations as f64 / authority.shared_complete_case_observations as f64;
        if group.canonical_ml_covariance_sha256 != expected_covariance
            || group.canonical_observed_means_sha256 != expected_means
            || group.weight.to_bits() != expected_weight.to_bits()
        {
            return Err(invalid(
                "group moment hash or N_g/N_total weight is inconsistent",
            ));
        }
    }
    let digests = std::iter::once(
        authority
            .null_grouping_omissions
            .source_row_indices_sha256
            .as_str(),
    )
    .chain(std::iter::once(
        authority.shared_complete_case_rows_sha256.as_str(),
    ))
    .chain([
        authority.plan_sha256.as_str(),
        authority.model_scientific_sha256.as_str(),
        authority.combined_authority_sha256.as_str(),
    ])
    .chain(authority.groups.iter().flat_map(|group| {
        [
            group.selected_source_row_indices_sha256.as_str(),
            group.complete_source_row_indices_sha256.as_str(),
            group.complete_rows_sha256.as_str(),
            group.canonical_ml_covariance_sha256.as_str(),
            group.canonical_observed_means_sha256.as_str(),
        ]
    }));
    if digests.into_iter().any(|digest| !lowercase_sha256(digest)) {
        return Err(invalid("authority contains a malformed SHA-256 digest"));
    }
    if authority.combined_authority_sha256 != combined_authority_digest(authority) {
        return Err(invalid(
            "combined authority digest differs from serialized authority fields",
        ));
    }
    Ok(())
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_dataset_fingerprint(value: &str) -> bool {
    value
        .strip_prefix("v2:")
        .map_or_else(|| lowercase_sha256(value), lowercase_sha256)
}

fn model_fit_receipt(
    model: &str,
    fit: &CbsemExactTwoGroupJointFitV1,
    observed_moments: usize,
    sample_sizes: [usize; 2],
    total_sample_size: usize,
    data_authority: &CbsemExactTwoGroupDataAuthorityV1,
) -> Result<CbsemExactTwoGroupModelFitV1, CbsemExactTwoGroupInvarianceErrorV1> {
    if fit.group_objectives.len() != 2
        || fit.group_parameter_estimates.len() != 2
        || fit.group_implied_covariances.len() != 2
        || data_authority.groups.len() != 2
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "joint optimizer returned an inconsistent two-group surface".into(),
        ));
    }
    if fit.dimension_count == 0
        || !fit.objective.is_finite()
        || fit.objective < 0.0
        || !fit.gradient_norm.is_finite()
        || fit
            .group_objectives
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        || fit
            .group_parameter_estimates
            .iter()
            .flatten()
            .any(|parameter| parameter.parameter_id.is_empty() || !parameter.estimate.is_finite())
        || fit.group_implied_covariances.iter().any(|matrix| {
            matrix.len() != data_authority.ordered_indicator_variable_ids.len()
                || matrix.iter().any(|row| {
                    row.len() != data_authority.ordered_indicator_variable_ids.len()
                        || row.iter().any(|value| !value.is_finite())
                })
        })
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "joint optimizer returned a non-finite or malformed fit receipt".into(),
        ));
    }
    for matrix in &fit.group_implied_covariances {
        ensure_strict_positive_definite(matrix)?;
    }
    let objective = (0..2)
        .map(|group| {
            sample_sizes[group] as f64 / total_sample_size as f64 * fit.group_objectives[group]
        })
        .sum::<f64>();
    let optimizer_tolerance = 1.0e-12 * objective.abs().max(fit.objective.abs()).max(1.0);
    if !objective.is_finite()
        || (objective - fit.objective).abs() > optimizer_tolerance
        || !fit.gradient_norm.is_finite()
    {
        return Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(
            "joint optimizer objective receipt failed mechanical recomputation".into(),
        ));
    }
    let groups = (0..2)
        .map(|group| CbsemExactTwoGroupGroupFitV1 {
            group_id: data_authority.groups[group].group_id.clone(),
            sample_size: sample_sizes[group],
            weight: sample_sizes[group] as f64 / total_sample_size as f64,
            objective: fit.group_objectives[group],
            chi_square: sample_sizes[group] as f64 * fit.group_objectives[group],
            parameters: fit.group_parameter_estimates[group]
                .iter()
                .map(|parameter| CbsemExactTwoGroupParameterEstimateV1 {
                    parameter_id: parameter.parameter_id.clone(),
                    estimate: parameter.estimate,
                    fixed: parameter.fixed,
                })
                .collect(),
            implied_covariance: fit.group_implied_covariances[group].clone(),
        })
        .collect::<Vec<_>>();
    let chi_square = groups.iter().map(|group| group.chi_square).sum::<f64>();
    let degrees_of_freedom = observed_moments as i64 - fit.dimension_count as i64;
    Ok(CbsemExactTwoGroupModelFitV1 {
        model: model.into(),
        estimator: "ml".into(),
        exact_parameter_table_method_version:
            crate::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
        joint_objective_method_version:
            CBSEM_EXACT_TWO_GROUP_JOINT_OBJECTIVE_METHOD_VERSION_V1.into(),
        covariance_moment_method_version:
            CBSEM_EXACT_TWO_GROUP_MOMENT_METHOD_VERSION_V1.into(),
        objective_definition: "sum_g (N_g / N_total) * F_ml_g; chi_square = sum_g N_g * F_ml_g"
            .into(),
        input_kind: CbsemMomentInputKindV2::Raw,
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
        mean_structure: false,
        observed_means_usage:
            CbsemExactTwoGroupObservedMeansUsageV1::PersistedForAuthorityNotConsumedByCovarianceStructureMl,
        constraint_scope: if model == "configural" {
            "group_local_dimensions_with_namespaced_existing_equalities"
        } else {
            "corresponding_free_loadings_equal_across_groups; fixed_markers_unchanged; existing_equalities_transitively_preserved"
        }
        .into(),
        shared_complete_case_rows_sha256: data_authority
            .shared_complete_case_rows_sha256
            .clone(),
        combined_data_authority_sha256: data_authority.combined_authority_sha256.clone(),
        plan_sha256: data_authority.plan_sha256.clone(),
        model_scientific_sha256: data_authority.model_scientific_sha256.clone(),
        converged: true,
        admissible: true,
        initialization_iterations: fit.initialization_iterations,
        iterations: fit.iterations,
        gradient_norm: fit.gradient_norm,
        free_dimensions: fit.dimension_count,
        observed_moments,
        degrees_of_freedom,
        objective,
        chi_square,
        groups,
        cfi: CbsemExactTwoGroupFitIndexOutcomeV1::Unavailable {
            reason: CbsemExactTwoGroupFitIndexUnavailableReasonV1::NotFrozenForEnginePrerequisite,
        },
        rmsea: CbsemExactTwoGroupFitIndexOutcomeV1::Unavailable {
            reason: CbsemExactTwoGroupFitIndexUnavailableReasonV1::NotFrozenForEnginePrerequisite,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        CompiledCbsemParameterStatusV2, Construct, LegacyBasicModelInterpretationV4,
        MeasurementMode, ModelSpec, ObservedTransformationOperationV4,
        ObservedTransformationStepV4, RecodeUnmappedPolicyV4, SemDataBindingV4, SemGroupLevelV4,
        SemParameterTargetV4, SemParameterV4, convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn dataset(
        group_a: usize,
        group_b: usize,
        null_group: bool,
        third_group: bool,
        singular_a: bool,
    ) -> Dataset {
        dataset_with_indicator_missing(group_a, group_b, null_group, third_group, singular_a, false)
    }

    fn dataset_with_indicator_missing(
        group_a: usize,
        group_b: usize,
        null_group: bool,
        third_group: bool,
        singular_a: bool,
        indicator_missing: bool,
    ) -> Dataset {
        let mut csv = String::from("group,x1,x2,x3\n");
        if null_group {
            csv.push_str("NA,0.1,0.2,0.3\n");
        }
        let maximum = group_a.max(group_b);
        for index in 0..maximum {
            for group in ["A", "B"] {
                let count = if group == "A" { group_a } else { group_b };
                if index >= count {
                    continue;
                }
                let angle = std::f64::consts::TAU * index as f64 / count as f64;
                let root_two = 2.0_f64.sqrt();
                let latent = root_two * angle.sin();
                let error_1 = root_two * angle.cos();
                let error_2 = root_two * (2.0 * angle).sin();
                let error_3 = root_two * (2.0 * angle).cos();
                let (x1, x2, x3) = if group == "A" {
                    let x1 = 0.70_f64.sqrt() * latent + 0.45_f64.sqrt() * error_1;
                    if singular_a {
                        (x1, 2.0 * x1, -0.5 * x1)
                    } else {
                        (
                            x1,
                            0.80 * 0.70_f64.sqrt() * latent + 0.55_f64.sqrt() * error_2,
                            0.60 * 0.70_f64.sqrt() * latent + 0.65_f64.sqrt() * error_3,
                        )
                    }
                } else {
                    (
                        0.75_f64.sqrt() * latent + 0.50_f64.sqrt() * error_1,
                        0.55 * 0.75_f64.sqrt() * latent + 0.60_f64.sqrt() * error_2,
                        0.90 * 0.75_f64.sqrt() * latent + 0.55_f64.sqrt() * error_3,
                    )
                };
                if indicator_missing
                    && ((group == "A" && index == 2) || (group == "B" && index == 3))
                {
                    csv.push_str(&format!("{group},{x1:.17},NA,{x3:.17}\n"));
                } else {
                    csv.push_str(&format!("{group},{x1:.17},{x2:.17},{x3:.17}\n"));
                }
            }
        }
        if third_group {
            csv.push_str("C,1.0,2.0,3.5\n");
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "two-group-exact-microcase.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn grouped_model(dataset: &Dataset) -> SemModelV4 {
        let legacy = ModelSpec {
            id: uuid::Uuid::from_u128(0xCB5E_2001),
            name: "Two-group one-factor CFA".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "Factor".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
            paths: Vec::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:group".into(),
            label: "Group".into(),
            source_column: "group".into(),
            scale: ObservedScaleV4::Nominal,
            role: ObservedRoleV4::Control,
            categories: vec!["A".into(), "B".into()],
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        // Intentionally reverse the authoring order. The compiled plan is the
        // canonical authority and sorts levels by stable id.
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:group".into(),
            levels: vec![
                SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "Group B".into(),
                },
                SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "Group A".into(),
                },
            ],
        };
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        model
    }

    fn plan(model: &SemModelV4) -> CompiledCbsemPlanV2 {
        compile_cbsem_plan_v2(model).unwrap()
    }

    fn parameter<'a>(
        fit: &'a CbsemExactTwoGroupGroupFitV1,
        parameter_id: &str,
    ) -> &'a CbsemExactTwoGroupParameterEstimateV1 {
        fit.parameters
            .iter()
            .find(|parameter| parameter.parameter_id == parameter_id)
            .unwrap()
    }

    #[test]
    fn genuine_configural_metric_fit_binds_group_rows_moments_dimensions_and_fixed_markers() {
        let dataset = dataset(32, 34, true, false, false);
        let model = grouped_model(&dataset);
        let plan = plan(&model);
        let result =
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model)
                .unwrap();

        assert_eq!(
            result.qualification_status,
            CbsemExactTwoGroupQualificationStatusV1::EngineOnlyPrerequisiteNotProductQualified
        );
        assert_eq!(
            result
                .data
                .groups
                .iter()
                .map(|group| group.group_id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(
            result
                .data
                .groups
                .iter()
                .map(|group| group.declared_value.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert_eq!(result.data.null_grouping_omissions.count, 1);
        assert_eq!(result.data.shared_complete_case_observations, 66);
        assert_eq!(result.data.groups[0].complete_observations, 32);
        assert_eq!(result.data.groups[1].complete_observations, 34);
        assert_eq!(result.data.groups[0].selected_observations, 32);
        assert_eq!(result.data.groups[1].selected_observations, 34);
        assert_eq!(result.data.input_kind, CbsemMomentInputKindV2::Raw);
        assert_eq!(
            result.data.covariance_denominator,
            SemCovarianceDenominatorV4::MaximumLikelihoodN
        );
        validate_cbsem_exact_two_group_data_authority_v1(&result.data).unwrap();

        assert_eq!(result.configural.free_dimensions, 12);
        assert_eq!(result.metric.free_dimensions, 10);
        assert_eq!(result.nesting.dimension_reduction, 2);
        assert_eq!(result.configural.observed_moments, 12);
        assert_eq!(result.configural.degrees_of_freedom, 0);
        assert_eq!(result.metric.degrees_of_freedom, 2);
        assert_eq!(result.nesting.delta_degrees_of_freedom, 2);
        assert!(result.nesting.p_value.is_finite());
        assert!((0.0..=1.0).contains(&result.nesting.p_value));
        for fit in [&result.configural, &result.metric] {
            let hand_objective = fit
                .groups
                .iter()
                .map(|group| group.weight * group.objective)
                .sum::<f64>();
            let hand_chi_square = fit
                .groups
                .iter()
                .map(|group| group.sample_size as f64 * group.objective)
                .sum::<f64>();
            assert!((fit.objective - hand_objective).abs() < 1.0e-14);
            assert!((fit.chi_square - hand_chi_square).abs() < 1.0e-12);
            assert_eq!(fit.estimator, "ml");
            assert_eq!(
                fit.exact_parameter_table_method_version,
                crate::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
            );
            assert_eq!(
                fit.joint_objective_method_version,
                CBSEM_EXACT_TWO_GROUP_JOINT_OBJECTIVE_METHOD_VERSION_V1
            );
            assert_eq!(
                fit.covariance_moment_method_version,
                CBSEM_EXACT_TWO_GROUP_MOMENT_METHOD_VERSION_V1
            );
            assert!(!fit.mean_structure);
            assert_eq!(
                fit.combined_data_authority_sha256,
                result.data.combined_authority_sha256
            );
        }

        let parameter_by_id = plan
            .parameters()
            .iter()
            .map(|parameter| (parameter.id(), parameter))
            .collect::<BTreeMap<_, _>>();
        let mut free_loading_ids = Vec::new();
        let mut marker_id = None;
        for loading in plan.loadings() {
            match parameter_by_id[loading.parameter_id()].specification() {
                CompiledCbsemParameterStatusV2::Free { .. } => {
                    free_loading_ids.push(loading.parameter_id())
                }
                CompiledCbsemParameterStatusV2::Fixed { value }
                    if value.to_bits() == 1.0_f64.to_bits() =>
                {
                    marker_id = Some(loading.parameter_id())
                }
                _ => {}
            }
        }
        assert_eq!(free_loading_ids.len(), 2);
        for loading_id in free_loading_ids {
            let left = parameter(&result.metric.groups[0], loading_id);
            let right = parameter(&result.metric.groups[1], loading_id);
            assert!(!left.fixed && !right.fixed);
            assert_eq!(left.estimate.to_bits(), right.estimate.to_bits());
        }
        let marker_id = marker_id.unwrap();
        for group in &result.metric.groups {
            let marker = parameter(group, marker_id);
            assert!(marker.fixed);
            assert_eq!(marker.estimate.to_bits(), 1.0_f64.to_bits());
        }
    }

    #[test]
    fn existing_loading_equality_is_preserved_transitively_by_metric_union() {
        let dataset = dataset(32, 34, false, false, false);
        let mut model = grouped_model(&dataset);
        let free_loadings = model
            .parameters
            .iter_mut()
            .filter(|parameter| {
                matches!(
                    parameter,
                    SemParameterV4::Free {
                        target: SemParameterTargetV4::Loading { .. },
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(free_loadings.len(), 2);
        for parameter in free_loadings {
            let SemParameterV4::Free { equality_label, .. } = parameter else {
                unreachable!()
            };
            *equality_label = Some("within_factor_loading_equality".into());
        }
        model.ensure_valid().unwrap();
        let plan = plan(&model);
        let result =
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model)
                .unwrap();
        assert_eq!(result.configural.free_dimensions, 10);
        assert_eq!(result.metric.free_dimensions, 9);
        assert_eq!(result.nesting.dimension_reduction, 1);
        for group in &result.metric.groups {
            let loading_values = group
                .parameters
                .iter()
                .filter(|parameter| {
                    !parameter.fixed
                        && plan.loadings().iter().any(|loading| {
                            loading.parameter_id() == parameter.parameter_id.as_str()
                        })
                })
                .map(|parameter| parameter.estimate.to_bits())
                .collect::<Vec<_>>();
            assert_eq!(loading_values.len(), 2);
            assert_eq!(loading_values[0], loading_values[1]);
        }
    }

    #[test]
    fn shared_listwise_universe_drives_both_fit_stages_and_has_stable_digests() {
        let dataset = dataset_with_indicator_missing(25, 26, true, false, false, true);
        let model = grouped_model(&dataset);
        let plan = plan(&model);
        let first = estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model)
            .unwrap();
        assert_eq!(first.data.null_grouping_omissions.count, 1);
        assert_eq!(first.data.groups[0].selected_observations, 25);
        assert_eq!(first.data.groups[1].selected_observations, 26);
        assert_eq!(first.data.groups[0].listwise_omitted_observations, 1);
        assert_eq!(first.data.groups[1].listwise_omitted_observations, 1);
        assert_eq!(first.data.groups[0].complete_observations, 24);
        assert_eq!(first.data.groups[1].complete_observations, 25);
        assert_eq!(first.data.shared_complete_case_observations, 49);
        for fit in [&first.configural, &first.metric] {
            assert_eq!(fit.groups[0].sample_size, 24);
            assert_eq!(fit.groups[1].sample_size, 25);
            assert_eq!(
                fit.shared_complete_case_rows_sha256,
                first.data.shared_complete_case_rows_sha256
            );
            assert_eq!(
                fit.combined_data_authority_sha256,
                first.data.combined_authority_sha256
            );
        }
        let second =
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model)
                .unwrap();
        assert_eq!(
            first.data.shared_complete_case_rows_sha256,
            second.data.shared_complete_case_rows_sha256
        );
        assert_eq!(
            first.data.combined_authority_sha256,
            second.data.combined_authority_sha256
        );
    }

    #[test]
    fn authority_validator_rejects_label_moment_and_method_tampering() {
        let dataset = dataset(28, 29, false, false, false);
        let model = grouped_model(&dataset);
        let plan = plan(&model);
        let result =
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model)
                .unwrap();

        let mut label = result.data.clone();
        label.groups[0].label.push_str(" tampered");
        assert!(matches!(
            validate_cbsem_exact_two_group_data_authority_v1(&label),
            Err(CbsemExactTwoGroupInvarianceErrorV1::AuthorityIntegrity(_))
        ));

        let mut moment = result.data.clone();
        moment.groups[0].covariance_ml[0][0] += 0.125;
        assert!(validate_cbsem_exact_two_group_data_authority_v1(&moment).is_err());

        let mut method = result.data.clone();
        method.input_kind = CbsemMomentInputKindV2::Covariance;
        assert!(matches!(
            validate_cbsem_exact_two_group_data_authority_v1(&method),
            Err(CbsemExactTwoGroupInvarianceErrorV1::AuthorityIntegrity(_))
        ));

        let mut uppercase = result.data.clone();
        uppercase.groups[0].complete_rows_sha256 = uppercase.groups[0]
            .complete_rows_sha256
            .to_ascii_uppercase();
        uppercase.combined_authority_sha256 = combined_authority_digest(&uppercase);
        assert!(matches!(
            validate_cbsem_exact_two_group_data_authority_v1(&uppercase),
            Err(CbsemExactTwoGroupInvarianceErrorV1::AuthorityIntegrity(_))
        ));
    }

    #[test]
    fn grouping_contract_rejects_third_group_small_group_singular_group_and_plan_splice() {
        let third = dataset(24, 24, false, true, false);
        let third_model = grouped_model(&third);
        let third_plan = plan(&third_model);
        assert!(matches!(
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(
                &third,
                &third_plan,
                &third_model
            ),
            Err(CbsemExactTwoGroupInvarianceErrorV1::UnmappedGroupValue { .. })
        ));

        let small = dataset(9, 24, false, false, false);
        let small_model = grouped_model(&small);
        let small_plan = plan(&small_model);
        assert!(matches!(
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(
                &small,
                &small_plan,
                &small_model
            ),
            Err(CbsemExactTwoGroupInvarianceErrorV1::InsufficientGroupObservations { .. })
        ));

        let singular = dataset(24, 24, false, false, true);
        let singular_model = grouped_model(&singular);
        let singular_plan = plan(&singular_model);
        assert!(matches!(
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(
                &singular,
                &singular_plan,
                &singular_model
            ),
            Err(CbsemExactTwoGroupInvarianceErrorV1::Moment(
                CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite { .. }
            ))
        ));

        let dataset = dataset(24, 24, false, false, false);
        let model = grouped_model(&dataset);
        let original_plan = plan(&model);
        let mut changed = model.clone();
        let SemParameterV4::Free { start, .. } = changed
            .parameters
            .iter_mut()
            .find(|parameter| matches!(parameter, SemParameterV4::Free { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        *start = Some(0.731);
        changed.ensure_valid().unwrap();
        assert!(matches!(
            estimate_cbsem_ml_exact_two_group_configural_metric_v1(
                &dataset,
                &original_plan,
                &changed
            ),
            Err(CbsemExactTwoGroupInvarianceErrorV1::PlanIdentityMismatch)
        ));
    }

    #[test]
    fn grouping_missing_marker_and_lineage_are_rejected_before_row_selection() {
        let dataset = dataset(24, 24, false, false, false);
        for lineage in [false, true] {
            let mut model = grouped_model(&dataset);
            let SemVariableV4::Observed {
                missing_markers,
                transformation_lineage,
                ..
            } = model
                .variables
                .iter_mut()
                .find(|variable| variable.id() == "observed:group")
                .unwrap()
            else {
                unreachable!()
            };
            if lineage {
                transformation_lineage.push(ObservedTransformationStepV4 {
                    id: "group-recode".into(),
                    input_columns: vec!["group_raw".into()],
                    output_column: "group".into(),
                    operation: ObservedTransformationOperationV4::Recode {
                        mappings: BTreeMap::from([
                            ("alpha".into(), "A".into()),
                            ("beta".into(), "B".into()),
                        ]),
                        unmapped: RecodeUnmappedPolicyV4::Reject,
                    },
                });
            } else {
                missing_markers.push("UNKNOWN".into());
            }
            model.ensure_valid().unwrap();
            let plan = plan(&model);
            assert!(matches!(
                estimate_cbsem_ml_exact_two_group_configural_metric_v1(&dataset, &plan, &model),
                Err(CbsemExactTwoGroupInvarianceErrorV1::UnsupportedPlan(_))
            ));
        }
    }

    #[test]
    fn lrt_tail_tolerance_and_post_metric_cancellation_are_fail_closed() {
        let distribution = ChiSquared::new(10.0).unwrap();
        assert_eq!(1.0 - distribution.cdf(100.0), 0.0);
        let tail = exact_chi_square_upper_tail(100.0, 10).unwrap();
        assert!(tail.is_finite() && tail > 0.0);

        let (_, tolerance, statistic) = exact_nested_lrt_statistic(
            100.0,
            100.0 - 0.5 * CBSEM_EXACT_TWO_GROUP_NEGATIVE_LRT_RELATIVE_TOLERANCE_V1 * 100.0,
        )
        .unwrap();
        assert!(tolerance > 0.0);
        assert_eq!(statistic.to_bits(), 0.0_f64.to_bits());
        assert!(matches!(
            exact_nested_lrt_statistic(100.0, 99.0),
            Err(CbsemExactTwoGroupInvarianceErrorV1::NegativeChiSquareDifference { .. })
        ));

        let dataset = dataset(24, 25, false, false, false);
        let model = grouped_model(&dataset);
        let plan = plan(&model);
        let cancel = AtomicBool::new(false);
        let result = estimate_cbsem_ml_exact_two_group_configural_metric_v1_with_control(
            &dataset,
            &plan,
            &model,
            || cancel.load(Ordering::SeqCst),
            |progress| {
                if progress.phase == "metric_complete" {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(matches!(
            result,
            Err(CbsemExactTwoGroupInvarianceErrorV1::Exact(
                CbsemExactParameterTableErrorV3::Cancelled
            ))
        ));
    }
}
