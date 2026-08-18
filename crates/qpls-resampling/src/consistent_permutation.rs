//! Fixed-size, two-group label permutation for the bounded `plsc_v2`
//! estimator.
//!
//! This is intentionally separate from the ordinary PLS-PM Freedman-Lane
//! permutation engine. Every original and permuted group is re-estimated with
//! PLSc, every requested permutation index is attempted exactly once, and a
//! failed refit is retained in the immutable ledger.

use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

use qpls_core::{
    AnalysisMethod, AnalysisRecipe, AnalysisSettings, MeasurementMode, MethodConfig,
    MissingDataPolicy, PlscPermutationTestTail, Preprocessing, ValidatedExecutionRecipe,
    WeightingScheme,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, PLSC_METHOD_VERSION, PlsResult, estimate_pls_validated_with_control,
};

use super::{
    PermutationPlan, ReplicateOutcome, ResamplingError, ResamplingProgress, align_pls_signs,
    complete_case_rows, permutation_indices, resample_model_dataset, run_permutation,
};

pub const PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION: &str = "plsc_permutation_v1";
pub const PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION: &str =
    "indexed_group_label_permutation_v1";
pub const PLSC_CONSISTENT_PERMUTATION_OPERATION: &str = "plsc_group_label_permutation_v1";
pub const PLSC_CONSISTENT_PERMUTATION_TEST_V1: &str = "two_tailed_absolute_difference_plus_one_v1";
pub const PLSC_CONSISTENT_PERMUTATION_TEST: &str =
    "two_tailed_and_directional_difference_plus_one_v2";
pub const PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION: &str =
    "plsc_directional_permutation_v1";
pub const PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST: &str = "directed_greater_less_plus_one_v1";
pub const PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION: &str =
    "plsc_permutation_selected_tail_v1";
pub const PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION: &str = "group_a_minus_group_b";
pub const PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY: &str =
    "no_retry_no_replacement_fixed_indexed_labels_v1";
pub const PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION: f64 = 0.90;
pub const PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL: f64 = 0.05;
pub const PLSC_CONSISTENT_PERMUTATION_GROUP_COLUMN_KEY: &str = "mga_group_column";
pub const PLSC_CONSISTENT_PERMUTATION_GROUP_A_KEY: &str = "mga_group_a";
pub const PLSC_CONSISTENT_PERMUTATION_GROUP_B_KEY: &str = "mga_group_b";
pub const PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING: &str = "Consistent permutation re-estimated complete plsc_v2 models for both original groups and for both groups in every fixed label assignment; ordinary PLS permutation estimates were not reused.";
pub const PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING: &str = "Failed or inadmissible PLSc group refits were retained in the fixed permutation ledger without retry, replacement, clamping, or ordinary-PLS fallback.";
pub const PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1: &str = "This internal v1 result reports two-tailed PLSc group-parameter differences only; MICOM, one-tailed inference, outer-weight/effect breadth, and more than two groups are not implemented.";
pub const PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING: &str = "This internal v1 result reports two-tailed and directed greater/less PLSc group-parameter differences; MICOM, outer-weight/effect breadth, and more than two groups are not implemented.";

const LABEL_ASSIGNMENT_DIGEST_DOMAIN: &[u8] =
    b"QuickPLS PLSc consistent permutation label assignment v1\0";
const PARAMETER_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLSc consistent permutation parameters v1\0";
const VALIDATION_TOLERANCE: f64 = 1e-10;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PlscPermutationParameterFamily {
    Path,
    OuterLoading,
    RhoA,
    ConstructCorrelation,
    RSquared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationGroupSummary {
    pub group: String,
    pub observations: usize,
    pub parameter_values_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlscPermutationStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationLedgerEntry {
    pub permutation_index: u32,
    pub label_assignment_sha256: String,
    pub status: PlscPermutationStatus,
    pub parameter_values_sha256: Option<String>,
    pub reason_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationFailedPermutation {
    pub permutation_index: u32,
    pub label_assignment_sha256: String,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationParameterInference {
    pub parameter: String,
    pub family: PlscPermutationParameterFamily,
    pub estimate_a: f64,
    pub estimate_b: f64,
    /// The original directed contrast, always Group A minus Group B.
    pub original: f64,
    pub exceedances: u32,
    pub p_value_two_sided: f64,
    /// Usable permutation count. Failed planned indices are never replaced.
    pub permutations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationDirectionalParameterInference {
    pub parameter: String,
    pub greater_or_equal: u32,
    pub less_or_equal: u32,
    pub p_value_greater: f64,
    pub p_value_less: f64,
    /// The exact usable-permutation denominator shared with the two-sided row.
    pub permutations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationDirectionalInference {
    pub method_version: String,
    pub test_method: String,
    pub parameters: Vec<PlscPermutationDirectionalParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationSelectedTailParameterInference {
    pub parameter: String,
    pub selected_exceedances: u32,
    pub selected_p_value: f64,
    pub permutations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscPermutationSelectedTailInference {
    pub method_version: String,
    pub orientation: String,
    pub selected_test_tail: PlscPermutationTestTail,
    pub parameters: Vec<PlscPermutationSelectedTailParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscConsistentPermutationResult {
    pub method_version: String,
    pub estimator_method_version: String,
    pub scheduler_method_version: String,
    pub plan: PermutationPlan,
    pub test_method: String,
    pub significance_level: f64,
    pub minimum_usable_fraction: f64,
    pub retry_policy: String,
    pub group_column: String,
    pub group_a: PlscPermutationGroupSummary,
    pub group_b: PlscPermutationGroupSummary,
    pub pooled_parameter_values_sha256: String,
    pub usable_permutations: u32,
    pub failed_permutations: Vec<PlscPermutationFailedPermutation>,
    pub permutation_ledger: Vec<PlscPermutationLedgerEntry>,
    pub parameters: Vec<PlscPermutationParameterInference>,
    /// Added by the current combined-v2 test contract. Historical two-sided-v1
    /// payloads deserialize without this field and remain strictly readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directional_inference: Option<PlscPermutationDirectionalInference>,
    /// Present only for an explicit directional recipe selection. The
    /// historical/default two-sided result bytes remain unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_tail_inference: Option<PlscPermutationSelectedTailInference>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PlscConsistentPermutationError {
    #[error("PLSc consistent permutation requires raw observations")]
    RawDataRequired,
    #[error("PLSc consistent permutation requires method plsc")]
    InvalidMethod,
    #[error("PLSc consistent permutation requires 99 to 10000 planned permutations")]
    InvalidPermutationCount,
    #[error("PLSc consistent permutation requires exactly two supported explicit groups: {0}")]
    InvalidGroups(String),
    #[error(
        "PLSc consistent permutation does not support this model, data, or inference combination: {0}"
    )]
    UnsupportedScope(String),
    #[error("PLSc consistent permutation point result is inconsistent: {0}")]
    InconsistentPointResult(String),
    #[error("original PLSc group fit failed for {group}: {message}")]
    OriginalGroupFit { group: String, message: String },
    #[error(
        "PLSc consistent permutation produced {usable} usable assignments; at least {required} are required"
    )]
    InsufficientUsablePermutations { usable: usize, required: usize },
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Clone)]
struct ParameterValue {
    family: PlscPermutationParameterFamily,
    value: f64,
}

type ParameterValues = BTreeMap<String, ParameterValue>;

#[derive(Debug, Clone)]
struct GroupPlan {
    group_column: String,
    group_a: String,
    group_b: String,
    test_tail: PlscPermutationTestTail,
    group_a_rows: Vec<usize>,
    group_b_rows: Vec<usize>,
    canonical_rows: Vec<usize>,
    canonical_complete_positions: Vec<usize>,
    canonical_labels: Vec<u8>,
}

#[derive(Debug, Clone)]
struct PermutationEstimate {
    label_assignment_sha256: String,
    differences: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermutationExceedanceCounts {
    two_sided: u32,
    greater_or_equal: u32,
    less_or_equal: u32,
    usable: u32,
}

fn permutation_exceedance_counts(
    values: impl IntoIterator<Item = f64>,
    original: f64,
) -> PermutationExceedanceCounts {
    values.into_iter().fold(
        PermutationExceedanceCounts {
            two_sided: 0,
            greater_or_equal: 0,
            less_or_equal: 0,
            usable: 0,
        },
        |mut counts, value| {
            counts.usable += 1;
            counts.two_sided += u32::from(value.abs() >= original.abs());
            counts.greater_or_equal += u32::from(value >= original);
            counts.less_or_equal += u32::from(value <= original);
            counts
        },
    )
}

fn plus_one_probability(exceedances: u32, usable: u32) -> f64 {
    (f64::from(exceedances) + 1.0) / (f64::from(usable) + 1.0)
}

/// Execute the bounded two-group consistent-permutation kernel. The validated
/// recipe remains a PLSc recipe; group identity is bound by the typed
/// `plsc_permutation` method configuration.
pub fn permutation_plsc_consistent_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlscConsistentPermutationResult, PlscConsistentPermutationError> {
    let recipe = execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| {
            PlscConsistentPermutationError::InconsistentPointResult(error.to_string())
        })?;
    validate_execution_scope(dataset, recipe, original)?;

    let complete_rows = complete_case_rows(dataset, recipe);
    if complete_rows.len() != original.used_observations {
        return Err(PlscConsistentPermutationError::InconsistentPointResult(
            "the point-result observation count differs from the complete model-case set".into(),
        ));
    }
    let group_plan = build_group_plan(dataset, recipe, &complete_rows)?;
    let pooled_values = plsc_permutation_parameter_values(original)
        .map_err(PlscConsistentPermutationError::InconsistentPointResult)?;
    let pooled_parameter_values_sha256 = parameter_values_sha256(&pooled_values);
    let base_execution = execution.without_outer_resampling().map_err(|error| {
        PlscConsistentPermutationError::InconsistentPointResult(error.to_string())
    })?;
    if base_execution.source().settings.method != AnalysisMethod::Plsc
        || base_execution.source().settings.bootstrap_samples != 0
        || base_execution.source().settings.studentized_inner_samples != 0
        || base_execution.source().settings.permutation_samples != 0
    {
        return Err(PlscConsistentPermutationError::InconsistentPointResult(
            "the derived group-refit recipe is not a point-only PLSc recipe".into(),
        ));
    }

    let complete_position_by_row = complete_rows
        .iter()
        .enumerate()
        .map(|(position, row)| (*row, position))
        .collect::<HashMap<_, _>>();
    let group_a_positions = group_plan
        .group_a_rows
        .iter()
        .map(|row| complete_position_by_row[row])
        .collect::<Vec<_>>();
    let group_b_positions = group_plan
        .group_b_rows
        .iter()
        .map(|row| complete_position_by_row[row])
        .collect::<Vec<_>>();
    let cancellation = &is_cancelled;
    if cancellation() {
        return Err(ResamplingError::Cancelled.into());
    }
    let group_a_values = estimate_group_values(
        dataset,
        &base_execution,
        original,
        &group_plan.group_a_rows,
        &group_a_positions,
        &pooled_values,
        cancellation,
    );
    if cancellation() {
        return Err(ResamplingError::Cancelled.into());
    }
    let group_a_values =
        group_a_values.map_err(|message| PlscConsistentPermutationError::OriginalGroupFit {
            group: group_plan.group_a.clone(),
            message,
        })?;
    let group_b_values = estimate_group_values(
        dataset,
        &base_execution,
        original,
        &group_plan.group_b_rows,
        &group_b_positions,
        &pooled_values,
        cancellation,
    );
    if cancellation() {
        return Err(ResamplingError::Cancelled.into());
    }
    let group_b_values =
        group_b_values.map_err(|message| PlscConsistentPermutationError::OriginalGroupFit {
            group: group_plan.group_b.clone(),
            message,
        })?;
    let original_differences = parameter_differences(&group_a_values, &group_b_values)
        .map_err(PlscConsistentPermutationError::InconsistentPointResult)?;

    let plan = PermutationPlan {
        permutations: recipe.settings.permutation_samples,
        master_seed: recipe.settings.seed,
        operation: PLSC_CONSISTENT_PERMUTATION_OPERATION.into(),
    };
    let run = run_permutation(
        group_plan.canonical_rows.len(),
        &plan,
        workers,
        |permutation_index| {
            let shuffled_labels = permuted_labels(
                &group_plan.canonical_labels,
                plan.master_seed,
                &plan.operation,
                permutation_index,
            );
            let label_assignment_sha256 = label_assignment_sha256(&shuffled_labels);
            let (rows_a, positions_a, rows_b, positions_b) = split_assignment(
                &group_plan.canonical_rows,
                &group_plan.canonical_complete_positions,
                &shuffled_labels,
            )?;
            let values_a = estimate_group_values(
                dataset,
                &base_execution,
                original,
                &rows_a,
                &positions_a,
                &pooled_values,
                cancellation,
            )?;
            let values_b = estimate_group_values(
                dataset,
                &base_execution,
                original,
                &rows_b,
                &positions_b,
                &pooled_values,
                cancellation,
            )?;
            let differences = parameter_differences(&values_a, &values_b)
                .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
            Ok::<_, String>(PermutationEstimate {
                label_assignment_sha256,
                differences,
            })
        },
        cancellation,
        &report_progress,
    )?;

    let usable = run
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Success { .. }))
        .count();
    let required =
        ((plan.permutations as f64 * PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION).ceil()
            as usize)
            .max(2);
    if usable < required {
        return Err(
            PlscConsistentPermutationError::InsufficientUsablePermutations { usable, required },
        );
    }

    let mut failed_permutations = Vec::new();
    let mut permutation_ledger = Vec::with_capacity(run.outcomes.len());
    let mut successful_differences = Vec::with_capacity(usable);
    for (position, outcome) in run.outcomes.iter().enumerate() {
        let permutation_index = position as u32;
        let expected_labels = permuted_labels(
            &group_plan.canonical_labels,
            plan.master_seed,
            &plan.operation,
            permutation_index,
        );
        let expected_assignment_sha256 = label_assignment_sha256(&expected_labels);
        match outcome {
            ReplicateOutcome::Success { value } => {
                if value.label_assignment_sha256 != expected_assignment_sha256 {
                    return Err(PlscConsistentPermutationError::InconsistentPointResult(
                        "the indexed label-assignment digest changed during aggregation".into(),
                    ));
                }
                let parameter_values_sha256 = difference_values_sha256(&value.differences);
                successful_differences.push(&value.differences);
                permutation_ledger.push(PlscPermutationLedgerEntry {
                    permutation_index,
                    label_assignment_sha256: expected_assignment_sha256,
                    status: PlscPermutationStatus::Success,
                    parameter_values_sha256: Some(parameter_values_sha256),
                    reason_code: None,
                    message: None,
                });
            }
            ReplicateOutcome::Failed { message } => {
                let (reason_code, detail) = split_failure(message);
                failed_permutations.push(PlscPermutationFailedPermutation {
                    permutation_index,
                    label_assignment_sha256: expected_assignment_sha256.clone(),
                    reason_code: reason_code.clone(),
                    message: detail.clone(),
                });
                permutation_ledger.push(PlscPermutationLedgerEntry {
                    permutation_index,
                    label_assignment_sha256: expected_assignment_sha256,
                    status: PlscPermutationStatus::Failed,
                    parameter_values_sha256: None,
                    reason_code: Some(reason_code),
                    message: Some(detail),
                });
            }
        }
    }

    let (parameters, directional_parameters) = original_differences
        .iter()
        .map(|(parameter, original_difference)| {
            let counts = permutation_exceedance_counts(
                successful_differences
                    .iter()
                    .map(|differences| differences[parameter]),
                *original_difference,
            );
            debug_assert_eq!(counts.usable, usable as u32);
            let value_a = &group_a_values[parameter];
            let value_b = &group_b_values[parameter];
            (
                PlscPermutationParameterInference {
                    parameter: parameter.clone(),
                    family: value_a.family,
                    estimate_a: value_a.value,
                    estimate_b: value_b.value,
                    original: *original_difference,
                    exceedances: counts.two_sided,
                    p_value_two_sided: plus_one_probability(counts.two_sided, counts.usable),
                    permutations: counts.usable,
                },
                PlscPermutationDirectionalParameterInference {
                    parameter: parameter.clone(),
                    greater_or_equal: counts.greater_or_equal,
                    less_or_equal: counts.less_or_equal,
                    p_value_greater: plus_one_probability(counts.greater_or_equal, counts.usable),
                    p_value_less: plus_one_probability(counts.less_or_equal, counts.usable),
                    permutations: counts.usable,
                },
            )
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let selected_tail_inference = match group_plan.test_tail {
        PlscPermutationTestTail::TwoSided => None,
        selected_test_tail => Some(PlscPermutationSelectedTailInference {
            method_version: PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION.into(),
            orientation: PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION.into(),
            selected_test_tail,
            parameters: directional_parameters
                .iter()
                .map(|parameter| {
                    let (selected_exceedances, selected_p_value) = match selected_test_tail {
                        PlscPermutationTestTail::GroupAGreater => {
                            (parameter.greater_or_equal, parameter.p_value_greater)
                        }
                        PlscPermutationTestTail::GroupALess => {
                            (parameter.less_or_equal, parameter.p_value_less)
                        }
                        PlscPermutationTestTail::TwoSided => unreachable!(
                            "the default two-sided selection omits the selected-tail receipt"
                        ),
                    };
                    PlscPermutationSelectedTailParameterInference {
                        parameter: parameter.parameter.clone(),
                        selected_exceedances,
                        selected_p_value,
                        permutations: parameter.permutations,
                    }
                })
                .collect(),
        }),
    };

    let result = PlscConsistentPermutationResult {
        method_version: PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION.into(),
        estimator_method_version: PLSC_METHOD_VERSION.into(),
        scheduler_method_version: PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION.into(),
        plan,
        test_method: PLSC_CONSISTENT_PERMUTATION_TEST.into(),
        significance_level: PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL,
        minimum_usable_fraction: PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION,
        retry_policy: PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY.into(),
        group_column: group_plan.group_column,
        group_a: PlscPermutationGroupSummary {
            group: group_plan.group_a,
            observations: group_a_positions.len(),
            parameter_values_sha256: parameter_values_sha256(&group_a_values),
        },
        group_b: PlscPermutationGroupSummary {
            group: group_plan.group_b,
            observations: group_b_positions.len(),
            parameter_values_sha256: parameter_values_sha256(&group_b_values),
        },
        pooled_parameter_values_sha256,
        usable_permutations: usable as u32,
        failed_permutations,
        permutation_ledger,
        parameters,
        directional_inference: Some(PlscPermutationDirectionalInference {
            method_version: PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION.into(),
            test_method: PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST.into(),
            parameters: directional_parameters,
        }),
        selected_tail_inference,
        warnings: vec![
            PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING.into(),
            PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING.into(),
            PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING.into(),
        ],
    };
    validate_plsc_consistent_permutation_result(&result, original, recipe)
        .map_err(PlscConsistentPermutationError::InconsistentPointResult)?;
    Ok(result)
}

fn validate_execution_scope(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    original: &PlsResult,
) -> Result<(), PlscConsistentPermutationError> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlscConsistentPermutationError::RawDataRequired);
    }
    if recipe.settings.method != AnalysisMethod::Plsc {
        return Err(PlscConsistentPermutationError::InvalidMethod);
    }
    if !matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::PlscPermutation { .. })
    ) {
        return Err(PlscConsistentPermutationError::InvalidMethod);
    }
    if !(99..=10_000).contains(&recipe.settings.permutation_samples) {
        return Err(PlscConsistentPermutationError::InvalidPermutationCount);
    }
    if recipe.settings.bootstrap_samples != 0
        || recipe.settings.studentized_inner_samples != 0
        || recipe.settings.confidence_level.to_bits() != 0.95_f64.to_bits()
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.weighting_scheme == WeightingScheme::Pca
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || recipe
            .metadata
            .contains_key("pls_model_fit_exact_inference")
    {
        return Err(PlscConsistentPermutationError::UnsupportedScope(
            "v1 requires point-only standardized/listwise reflective PLSc with path or factor weighting, 95% confidence, and no weights, controls, interactions, higher-order constructs, bootstrap, studentized inference, or exact-fit selector".into(),
        ));
    }
    if recipe.model.paths.is_empty() {
        return Err(PlscConsistentPermutationError::UnsupportedScope(
            "v1 requires at least one structural path".into(),
        ));
    }
    if recipe.model.constructs.iter().any(|construct| {
        construct.mode != MeasurementMode::Reflective || construct.indicators.len() < 2
    }) {
        return Err(PlscConsistentPermutationError::UnsupportedScope(
            "v1 requires reflective constructs with at least two indicators each".into(),
        ));
    }
    if !original.converged || original.method_version != PLSC_METHOD_VERSION {
        return Err(PlscConsistentPermutationError::InconsistentPointResult(
            "the base estimate is not a converged plsc_v2 result".into(),
        ));
    }
    Ok(())
}

fn build_group_plan(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    complete_rows: &[usize],
) -> Result<GroupPlan, PlscConsistentPermutationError> {
    let (group_column, group_a, group_b, test_tail) = plsc_permutation_group_config(recipe)?;
    if group_a == group_b {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "Group A and Group B values must be distinct".into(),
        ));
    }
    if recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter())
        .any(|indicator| indicator == &group_column)
    {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "the group column cannot also be a model indicator".into(),
        ));
    }
    let position = dataset
        .batch
        .schema()
        .index_of(&group_column)
        .map_err(|_| {
            PlscConsistentPermutationError::InvalidGroups(format!(
                "group column '{group_column}' is not present"
            ))
        })?;
    let array = dataset.batch.column(position).as_ref();
    ensure_supported_group_array(array)?;
    let mut group_a_rows = Vec::new();
    let mut group_b_rows = Vec::new();
    for row in complete_rows {
        let label = group_label(array, *row)?.ok_or_else(|| {
            PlscConsistentPermutationError::InvalidGroups(format!(
                "complete model case {} has a missing or empty group value",
                row + 1
            ))
        })?;
        if label == group_a {
            group_a_rows.push(*row);
        } else if label == group_b {
            group_b_rows.push(*row);
        } else {
            return Err(PlscConsistentPermutationError::InvalidGroups(format!(
                "complete model case {} has unselected group value '{label}'",
                row + 1
            )));
        }
    }
    if group_a_rows.len() < 10 || group_b_rows.len() < 10 {
        return Err(PlscConsistentPermutationError::InvalidGroups(format!(
            "at least ten complete cases are required per group; found {} for '{}' and {} for '{}'",
            group_a_rows.len(),
            group_a,
            group_b_rows.len(),
            group_b
        )));
    }
    let complete_position_by_row = complete_rows
        .iter()
        .enumerate()
        .map(|(position, row)| (*row, position))
        .collect::<HashMap<_, _>>();
    let (first_rows, first_label, second_rows, second_label) = if group_a < group_b {
        (&group_a_rows, 0_u8, &group_b_rows, 1_u8)
    } else {
        (&group_b_rows, 1_u8, &group_a_rows, 0_u8)
    };
    let mut canonical_rows = Vec::with_capacity(complete_rows.len());
    let mut canonical_complete_positions = Vec::with_capacity(complete_rows.len());
    let mut canonical_labels = Vec::with_capacity(complete_rows.len());
    for (rows, label) in [(first_rows, first_label), (second_rows, second_label)] {
        for row in rows {
            canonical_rows.push(*row);
            canonical_complete_positions.push(complete_position_by_row[row]);
            canonical_labels.push(label);
        }
    }
    Ok(GroupPlan {
        group_column,
        group_a,
        group_b,
        test_tail,
        group_a_rows,
        group_b_rows,
        canonical_rows,
        canonical_complete_positions,
        canonical_labels,
    })
}

fn plsc_permutation_group_config(
    recipe: &AnalysisRecipe,
) -> Result<(String, String, String, PlscPermutationTestTail), PlscConsistentPermutationError> {
    let Some(MethodConfig::PlscPermutation {
        group_column,
        group_a,
        group_b,
        test_tail,
    }) = recipe.method_config.as_ref()
    else {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "method_config.kind must be plsc_permutation".into(),
        ));
    };
    let group_column = group_column.trim();
    let group_a = group_a.trim();
    let group_b = group_b.trim();
    if group_column.is_empty() || group_a.is_empty() || group_b.is_empty() {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "method_config.plsc_permutation requires non-empty group_column, group_a, and group_b"
                .into(),
        ));
    }
    Ok((
        group_column.to_owned(),
        group_a.to_owned(),
        group_b.to_owned(),
        *test_tail,
    ))
}

fn ensure_supported_group_array(array: &dyn Array) -> Result<(), PlscConsistentPermutationError> {
    if array.as_any().downcast_ref::<StringArray>().is_none()
        && array.as_any().downcast_ref::<BooleanArray>().is_none()
        && array.as_any().downcast_ref::<Int64Array>().is_none()
        && array.as_any().downcast_ref::<Float64Array>().is_none()
    {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "the group column must be text, Boolean, integer, or numeric".into(),
        ));
    }
    Ok(())
}

fn group_label(
    array: &dyn Array,
    row: usize,
) -> Result<Option<String>, PlscConsistentPermutationError> {
    if array.is_null(row) {
        return Ok(None);
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
            return Ok(None);
        }
        if value.fract().abs() <= f64::EPSILON {
            format!("{value:.0}")
        } else {
            value.to_string()
        }
    } else {
        return Err(PlscConsistentPermutationError::InvalidGroups(
            "the group column must be text, Boolean, integer, or numeric".into(),
        ));
    };
    Ok((!label.is_empty()).then_some(label))
}

fn estimate_group_values(
    dataset: &Dataset,
    base_execution: &ValidatedExecutionRecipe,
    pooled: &PlsResult,
    raw_rows: &[usize],
    complete_positions: &[usize],
    pooled_values: &ParameterValues,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<ParameterValues, String> {
    let sampled =
        resample_model_dataset(dataset, base_execution.effective(), raw_rows, is_cancelled)
            .map_err(|error| plsc_refit_failure(&error))?;
    let mut estimate =
        estimate_pls_validated_with_control(&sampled, base_execution, |_| !is_cancelled())
            .map_err(|error| plsc_refit_failure(&error))?;
    align_pls_signs(
        &mut estimate,
        &pooled.construct_scores,
        complete_positions,
        is_cancelled,
    )
    .map_err(|error| plsc_refit_failure(&error))?;
    let values = plsc_permutation_parameter_values(&estimate)
        .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
    ensure_same_parameter_identity(pooled_values, &values)
        .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
    Ok(values)
}

fn permuted_labels(
    canonical_labels: &[u8],
    master_seed: u64,
    operation: &str,
    permutation_index: u32,
) -> Vec<u8> {
    let permutation = permutation_indices(
        canonical_labels.len(),
        master_seed,
        operation,
        permutation_index,
    );
    permutation
        .iter()
        .map(|position| canonical_labels[*position])
        .collect()
}

fn split_assignment(
    canonical_rows: &[usize],
    canonical_positions: &[usize],
    labels: &[u8],
) -> Result<(Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>), String> {
    if canonical_rows.len() != canonical_positions.len() || canonical_rows.len() != labels.len() {
        return Err(
            "assignment_length_mismatch|canonical assignment vectors differ in length".into(),
        );
    }
    let mut rows_a = Vec::new();
    let mut positions_a = Vec::new();
    let mut rows_b = Vec::new();
    let mut positions_b = Vec::new();
    for ((row, position), label) in canonical_rows.iter().zip(canonical_positions).zip(labels) {
        match label {
            0 => {
                rows_a.push(*row);
                positions_a.push(*position);
            }
            1 => {
                rows_b.push(*row);
                positions_b.push(*position);
            }
            _ => {
                return Err(
                    "assignment_label_invalid|permuted group label is not zero or one".into(),
                );
            }
        }
    }
    Ok((rows_a, positions_a, rows_b, positions_b))
}

fn parameter_differences(
    group_a: &ParameterValues,
    group_b: &ParameterValues,
) -> Result<BTreeMap<String, f64>, String> {
    ensure_same_parameter_identity(group_a, group_b)?;
    let mut differences = BTreeMap::new();
    for (parameter, value_a) in group_a {
        let value_b = &group_b[parameter];
        let difference = value_a.value - value_b.value;
        if !difference.is_finite() {
            return Err(format!(
                "parameter {parameter} has a non-finite group difference"
            ));
        }
        differences.insert(parameter.clone(), difference);
    }
    Ok(differences)
}

fn plsc_permutation_parameter_values(result: &PlsResult) -> Result<ParameterValues, String> {
    if !result.converged || result.method_version != PLSC_METHOD_VERSION {
        return Err("estimate is not a converged plsc_v2 result".into());
    }
    let plsc = result
        .plsc
        .as_ref()
        .ok_or_else(|| "plsc_v2 correction payload is absent".to_string())?;
    if plsc.method_version != PLSC_METHOD_VERSION
        || plsc.corrected_paths != result.paths
        || plsc.corrected_r_squared != result.r_squared
    {
        return Err("PLSc correction payload differs from the result-level corrected model".into());
    }
    let mut values = BTreeMap::new();
    for reliability in &plsc.reliabilities {
        insert_parameter(
            &mut values,
            parameter_key("plsc_rho_a", &[&reliability.construct]),
            PlscPermutationParameterFamily::RhoA,
            reliability.rho_a,
        )?;
    }
    for correlation in &plsc.construct_correlations {
        insert_parameter(
            &mut values,
            parameter_key(
                "plsc_construct_correlation",
                &[&correlation.left, &correlation.right],
            ),
            PlscPermutationParameterFamily::ConstructCorrelation,
            correlation.corrected,
        )?;
    }
    for outer in &plsc.corrected_outer_loadings {
        insert_parameter(
            &mut values,
            parameter_key("plsc_outer_loading", &[&outer.construct, &outer.indicator]),
            PlscPermutationParameterFamily::OuterLoading,
            outer.loading,
        )?;
    }
    for path in &plsc.corrected_paths {
        insert_parameter(
            &mut values,
            parameter_key("plsc_path", &[&path.source, &path.target]),
            PlscPermutationParameterFamily::Path,
            path.coefficient,
        )?;
    }
    for (construct, value) in &plsc.corrected_r_squared {
        insert_parameter(
            &mut values,
            parameter_key("plsc_r_squared", &[construct]),
            PlscPermutationParameterFamily::RSquared,
            *value,
        )?;
    }
    if values.is_empty() {
        return Err("PLSc point result has no consistent-permutation parameters".into());
    }
    Ok(values)
}

fn insert_parameter(
    values: &mut ParameterValues,
    parameter: String,
    family: PlscPermutationParameterFamily,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("parameter {parameter} is non-finite"));
    }
    if values
        .insert(parameter.clone(), ParameterValue { family, value })
        .is_some()
    {
        return Err(format!("parameter identity {parameter} is duplicated"));
    }
    Ok(())
}

fn ensure_same_parameter_identity(
    left: &ParameterValues,
    right: &ParameterValues,
) -> Result<(), String> {
    if left.len() != right.len()
        || left
            .iter()
            .zip(right)
            .any(|((left_key, left_value), (right_key, right_value))| {
                left_key != right_key || left_value.family != right_value.family
            })
    {
        return Err("PLSc group parameter identities or families differ".into());
    }
    Ok(())
}

fn parameter_key(kind: &str, parts: &[&str]) -> String {
    serde_json::to_string(&(kind, parts))
        .expect("consistent-permutation parameter identity is serializable")
}

fn plsc_refit_failure(error: &EstimationError) -> String {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    let reason = if matches!(error, EstimationError::Cancelled) {
        "cancelled"
    } else if lower.contains("rho_a") || lower.contains("rho a") {
        "inadmissible_rho_a"
    } else if lower.contains("corrected construct correlation") {
        "inadmissible_corrected_correlation"
    } else if lower.contains("converg") {
        "plsc_nonconvergence"
    } else if lower.contains("singular") || lower.contains("collinear") {
        "singular_plsc_equation"
    } else if lower.contains("non-finite") || lower.contains("nonfinite") {
        "nonfinite_plsc_parameter"
    } else {
        "plsc_group_refit_failed"
    };
    format!("{reason}|{detail}")
}

fn split_failure(value: &str) -> (String, String) {
    let Some((reason, message)) = value.split_once('|') else {
        return ("plsc_group_refit_failed".into(), value.into());
    };
    let reason = if is_plsc_failure_reason(reason.trim()) {
        reason.trim()
    } else {
        "plsc_group_refit_failed"
    };
    let message = if message.trim().is_empty() {
        "PLSc group refit failed without a diagnostic message"
    } else {
        message.trim()
    };
    (reason.into(), message.into())
}

fn is_plsc_failure_reason(value: &str) -> bool {
    matches!(
        value,
        "cancelled"
            | "inadmissible_rho_a"
            | "inadmissible_corrected_correlation"
            | "plsc_nonconvergence"
            | "singular_plsc_equation"
            | "nonfinite_plsc_parameter"
            | "parameter_identity_mismatch"
            | "assignment_length_mismatch"
            | "assignment_label_invalid"
            | "plsc_group_refit_failed"
    )
}

fn label_assignment_sha256(labels: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(LABEL_ASSIGNMENT_DIGEST_DOMAIN);
    digest.update((labels.len() as u64).to_le_bytes());
    digest.update(labels);
    format!("{:x}", digest.finalize())
}

fn parameter_values_sha256(values: &ParameterValues) -> String {
    let projection = values
        .iter()
        .map(|(parameter, value)| (parameter.clone(), value.value))
        .collect::<BTreeMap<_, _>>();
    difference_values_sha256(&projection)
}

fn difference_values_sha256(values: &BTreeMap<String, f64>) -> String {
    let mut digest = Sha256::new();
    digest.update(PARAMETER_DIGEST_DOMAIN);
    digest.update((values.len() as u64).to_le_bytes());
    for (parameter, value) in values {
        digest.update((parameter.len() as u64).to_le_bytes());
        digest.update(parameter.as_bytes());
        let canonical_value = format!("{value:.12e}");
        digest.update((canonical_value.len() as u64).to_le_bytes());
        digest.update(canonical_value.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= VALIDATION_TOLERANCE * left.abs().max(right.abs()).max(1.0)
}

fn canonical_labels_for_validation(
    group_a: &str,
    count_a: usize,
    group_b: &str,
    count_b: usize,
) -> Vec<u8> {
    let mut labels = Vec::with_capacity(count_a + count_b);
    if group_a < group_b {
        labels.extend(std::iter::repeat_n(0, count_a));
        labels.extend(std::iter::repeat_n(1, count_b));
    } else {
        labels.extend(std::iter::repeat_n(1, count_b));
        labels.extend(std::iter::repeat_n(0, count_a));
    }
    labels
}

/// Strict archive/native/export validator. It binds the result to the linked
/// `plsc_v2` point result, recipe settings, directed group labels, complete
/// accounting, indexed assignments, parameter identity, and p-value
/// arithmetic. It deliberately cannot promote the capability or add MICOM.
pub fn validate_plsc_consistent_permutation_result(
    result: &PlscConsistentPermutationResult,
    original: &PlsResult,
    recipe: &AnalysisRecipe,
) -> Result<(), String> {
    let (expected_group_column, expected_group_a, expected_group_b, expected_test_tail) =
        plsc_permutation_group_config(recipe).map_err(|error| error.to_string())?;
    validate_plsc_consistent_permutation_result_internal(
        result,
        original,
        &recipe.settings,
        Some((
            expected_group_column.as_str(),
            expected_group_a.as_str(),
            expected_group_b.as_str(),
        )),
        Some(expected_test_tail),
    )
}

/// Settings-only strict validation for semantic exporters that receive an
/// immutable result envelope but not the archived recipe document. Project
/// persistence uses [`validate_plsc_consistent_permutation_result`] and binds
/// the same payload to the recipe's exact typed group configuration as an
/// additional guard.
pub fn validate_plsc_consistent_permutation_result_for_settings(
    result: &PlscConsistentPermutationResult,
    original: &PlsResult,
    settings: &AnalysisSettings,
) -> Result<(), String> {
    validate_plsc_consistent_permutation_result_internal(result, original, settings, None, None)
}

fn validate_plsc_consistent_permutation_result_internal(
    result: &PlscConsistentPermutationResult,
    original: &PlsResult,
    settings: &AnalysisSettings,
    expected_groups: Option<(&str, &str, &str)>,
    expected_test_tail: Option<PlscPermutationTestTail>,
) -> Result<(), String> {
    let current_test = result.test_method == PLSC_CONSISTENT_PERMUTATION_TEST;
    let legacy_test = result.test_method == PLSC_CONSISTENT_PERMUTATION_TEST_V1;
    if result.method_version != PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION
        || result.estimator_method_version != PLSC_METHOD_VERSION
        || result.scheduler_method_version != PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION
        || result.plan.operation != PLSC_CONSISTENT_PERMUTATION_OPERATION
        || result.plan.permutations != settings.permutation_samples
        || result.plan.master_seed != settings.seed
        || !(99..=10_000).contains(&result.plan.permutations)
        || !(current_test || legacy_test)
        || result.significance_level.to_bits()
            != PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL.to_bits()
        || result.minimum_usable_fraction.to_bits()
            != PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION.to_bits()
        || result.retry_policy != PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY
        || settings.method != AnalysisMethod::Plsc
        || settings.bootstrap_samples != 0
        || settings.studentized_inner_samples != 0
        || settings.confidence_level.to_bits() != 0.95_f64.to_bits()
    {
        return Err(
            "method, scheduler, plan, test, or immutable setting identity is invalid".into(),
        );
    }
    if result.group_column.trim().is_empty()
        || result.group_a.group.trim().is_empty()
        || result.group_b.group.trim().is_empty()
        || result.group_a.group == result.group_b.group
        || result.group_a.observations < 10
        || result.group_b.observations < 10
        || result.group_a.observations + result.group_b.observations != original.used_observations
    {
        return Err("group identity or complete-case accounting is invalid".into());
    }
    if expected_groups.is_some_and(|(group_column, group_a, group_b)| {
        group_a == group_b
            || result.group_column != group_column
            || result.group_a.group != group_a
            || result.group_b.group != group_b
    }) {
        return Err("group identity differs from the linked recipe".into());
    }
    let pooled_values = plsc_permutation_parameter_values(original)?;
    if !is_sha256_hex(&result.pooled_parameter_values_sha256)
        || result.pooled_parameter_values_sha256 != parameter_values_sha256(&pooled_values)
    {
        return Err("pooled point-parameter digest differs from the linked plsc_v2 result".into());
    }
    let required =
        ((result.plan.permutations as f64 * result.minimum_usable_fraction).ceil() as u32).max(2);
    if result.usable_permutations < required
        || result.usable_permutations as usize + result.failed_permutations.len()
            != result.plan.permutations as usize
        || result.permutation_ledger.len() != result.plan.permutations as usize
    {
        return Err("permutation accounting is invalid".into());
    }
    let failed = result
        .failed_permutations
        .iter()
        .map(|entry| (entry.permutation_index, entry))
        .collect::<BTreeMap<_, _>>();
    if failed.len() != result.failed_permutations.len() {
        return Err("failure ledger contains duplicate permutation indices".into());
    }
    let labels = canonical_labels_for_validation(
        &result.group_a.group,
        result.group_a.observations,
        &result.group_b.group,
        result.group_b.observations,
    );
    for (position, entry) in result.permutation_ledger.iter().enumerate() {
        if entry.permutation_index != position as u32 {
            return Err("permutation ledger is not in exact index order".into());
        }
        let assignment = permuted_labels(
            &labels,
            result.plan.master_seed,
            &result.plan.operation,
            entry.permutation_index,
        );
        if entry.label_assignment_sha256 != label_assignment_sha256(&assignment) {
            return Err("permutation label-assignment digest is invalid".into());
        }
        match entry.status {
            PlscPermutationStatus::Success => {
                if failed.contains_key(&entry.permutation_index)
                    || entry
                        .parameter_values_sha256
                        .as_deref()
                        .is_none_or(|digest| !is_sha256_hex(digest))
                    || entry.reason_code.is_some()
                    || entry.message.is_some()
                {
                    return Err("successful permutation ledger entry is malformed".into());
                }
            }
            PlscPermutationStatus::Failed => {
                let failure = failed.get(&entry.permutation_index).ok_or_else(|| {
                    "failed permutation is absent from failure ledger".to_string()
                })?;
                if entry.parameter_values_sha256.is_some()
                    || entry.reason_code.as_deref() != Some(failure.reason_code.as_str())
                    || entry.message.as_deref() != Some(failure.message.as_str())
                    || entry.label_assignment_sha256 != failure.label_assignment_sha256
                    || !is_plsc_failure_reason(&failure.reason_code)
                    || failure.message.trim().is_empty()
                {
                    return Err("failed permutation ledger entry is malformed".into());
                }
            }
        }
    }

    if result.parameters.len() != pooled_values.len() {
        return Err("parameter table differs in length from the linked PLSc point result".into());
    }
    let names = result
        .parameters
        .iter()
        .map(|entry| entry.parameter.as_str())
        .collect::<BTreeSet<_>>();
    if names.len() != result.parameters.len()
        || names != pooled_values.keys().map(String::as_str).collect()
    {
        return Err("parameter identities differ from the linked PLSc point result".into());
    }
    if result
        .parameters
        .iter()
        .map(|entry| entry.parameter.as_str())
        .ne(pooled_values.keys().map(String::as_str))
    {
        return Err("parameter table is not in canonical identity order".into());
    }
    let mut group_a_values = BTreeMap::new();
    let mut group_b_values = BTreeMap::new();
    for entry in &result.parameters {
        let expected = &pooled_values[&entry.parameter];
        let expected_probability =
            plus_one_probability(entry.exceedances, result.usable_permutations);
        let probability_matches = if current_test {
            entry.p_value_two_sided.to_bits() == expected_probability.to_bits()
        } else {
            approximately_equal(entry.p_value_two_sided, expected_probability)
        };
        if entry.family != expected.family
            || !entry.estimate_a.is_finite()
            || !entry.estimate_b.is_finite()
            || !entry.original.is_finite()
            || !approximately_equal(entry.original, entry.estimate_a - entry.estimate_b)
            || entry.exceedances > result.usable_permutations
            || entry.permutations != result.usable_permutations
            || !entry.p_value_two_sided.is_finite()
            || entry.p_value_two_sided <= 0.0
            || entry.p_value_two_sided > 1.0
            || !probability_matches
        {
            return Err(
                "parameter family, contrast, count, or p-value arithmetic is invalid".into(),
            );
        }
        group_a_values.insert(
            entry.parameter.clone(),
            ParameterValue {
                family: entry.family,
                value: entry.estimate_a,
            },
        );
        group_b_values.insert(
            entry.parameter.clone(),
            ParameterValue {
                family: entry.family,
                value: entry.estimate_b,
            },
        );
    }
    validate_directional_inference(result, current_test)?;
    validate_selected_tail_inference(result, current_test, expected_test_tail)?;
    if !is_sha256_hex(&result.group_a.parameter_values_sha256)
        || !is_sha256_hex(&result.group_b.parameter_values_sha256)
        || result.group_a.parameter_values_sha256 != parameter_values_sha256(&group_a_values)
        || result.group_b.parameter_values_sha256 != parameter_values_sha256(&group_b_values)
    {
        return Err("original group parameter digest is invalid".into());
    }
    let expected_bounded_scope_warning = if current_test {
        PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING
    } else {
        PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1
    };
    if result.warnings
        != [
            PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING,
            PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING,
            expected_bounded_scope_warning,
        ]
    {
        return Err("bounded-scope warning identity is invalid".into());
    }
    Ok(())
}

fn validate_directional_inference(
    result: &PlscConsistentPermutationResult,
    current_test: bool,
) -> Result<(), String> {
    if !current_test {
        if result.directional_inference.is_some() {
            return Err("historical two-sided test contains a directional contract".into());
        }
        return Ok(());
    }
    let directional = result
        .directional_inference
        .as_ref()
        .ok_or_else(|| "current permutation test is missing directional inference".to_string())?;
    if directional.method_version != PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION
        || directional.test_method != PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST
        || directional.parameters.len() != result.parameters.len()
    {
        return Err("directional method, test, or parameter count is invalid".into());
    }
    for (two_sided, directed) in result.parameters.iter().zip(&directional.parameters) {
        let expected_greater =
            plus_one_probability(directed.greater_or_equal, result.usable_permutations);
        let expected_less =
            plus_one_probability(directed.less_or_equal, result.usable_permutations);
        let two_sided_dominates_observed_tail = if two_sided.original > 0.0 {
            two_sided.exceedances >= directed.greater_or_equal
        } else if two_sided.original < 0.0 {
            two_sided.exceedances >= directed.less_or_equal
        } else {
            two_sided.exceedances == result.usable_permutations
        };
        if directed.parameter != two_sided.parameter
            || directed.permutations != result.usable_permutations
            || directed.greater_or_equal > result.usable_permutations
            || directed.less_or_equal > result.usable_permutations
            || directed.greater_or_equal + directed.less_or_equal < result.usable_permutations
            || !two_sided_dominates_observed_tail
            || !directed.p_value_greater.is_finite()
            || !directed.p_value_less.is_finite()
            || directed.p_value_greater <= 0.0
            || directed.p_value_greater > 1.0
            || directed.p_value_less <= 0.0
            || directed.p_value_less > 1.0
            || directed.p_value_greater.to_bits() != expected_greater.to_bits()
            || directed.p_value_less.to_bits() != expected_less.to_bits()
        {
            return Err(
                "directional parameter identity, count, denominator, or p-value is invalid".into(),
            );
        }
    }
    Ok(())
}

fn validate_selected_tail_inference(
    result: &PlscConsistentPermutationResult,
    current_test: bool,
    expected_test_tail: Option<PlscPermutationTestTail>,
) -> Result<(), String> {
    if !current_test {
        if result.selected_tail_inference.is_some()
            || expected_test_tail.is_some_and(|tail| tail != PlscPermutationTestTail::TwoSided)
        {
            return Err(
                "historical two-sided test contains or is linked to a selected-tail contract"
                    .into(),
            );
        }
        return Ok(());
    }

    let selected_test_tail = result
        .selected_tail_inference
        .as_ref()
        .map(|receipt| receipt.selected_test_tail)
        .unwrap_or(PlscPermutationTestTail::TwoSided);
    if expected_test_tail.is_some_and(|expected| expected != selected_test_tail) {
        return Err("selected test tail differs from the linked recipe".into());
    }

    let Some(receipt) = result.selected_tail_inference.as_ref() else {
        return Ok(());
    };
    let directional = result
        .directional_inference
        .as_ref()
        .ok_or_else(|| "selected-tail receipt requires directional inference".to_string())?;
    if receipt.method_version != PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION
        || receipt.orientation != PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION
        || receipt.selected_test_tail == PlscPermutationTestTail::TwoSided
        || receipt.parameters.len() != result.parameters.len()
        || receipt.parameters.len() != directional.parameters.len()
    {
        return Err(
            "selected-tail method, orientation, selection, or parameter count is invalid".into(),
        );
    }

    for ((two_sided, directed), selected) in result
        .parameters
        .iter()
        .zip(&directional.parameters)
        .zip(&receipt.parameters)
    {
        let (expected_exceedances, expected_p_value) = match receipt.selected_test_tail {
            PlscPermutationTestTail::GroupAGreater => {
                (directed.greater_or_equal, directed.p_value_greater)
            }
            PlscPermutationTestTail::GroupALess => (directed.less_or_equal, directed.p_value_less),
            PlscPermutationTestTail::TwoSided => unreachable!(
                "an explicit selected-tail receipt cannot select the default two-sided test"
            ),
        };
        if selected.parameter != two_sided.parameter
            || selected.parameter != directed.parameter
            || selected.permutations != result.usable_permutations
            || selected.permutations != directed.permutations
            || selected.selected_exceedances != expected_exceedances
            || selected.selected_p_value.to_bits() != expected_p_value.to_bits()
        {
            return Err(
                "selected-tail parameter order, denominator, count, or p-value is invalid".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directional_validation_fixture() -> PlscConsistentPermutationResult {
        PlscConsistentPermutationResult {
            method_version: PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION.into(),
            estimator_method_version: PLSC_METHOD_VERSION.into(),
            scheduler_method_version: PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION.into(),
            plan: PermutationPlan {
                permutations: 4,
                master_seed: 7,
                operation: PLSC_CONSISTENT_PERMUTATION_OPERATION.into(),
            },
            test_method: PLSC_CONSISTENT_PERMUTATION_TEST.into(),
            significance_level: PLSC_CONSISTENT_PERMUTATION_SIGNIFICANCE_LEVEL,
            minimum_usable_fraction: PLSC_CONSISTENT_PERMUTATION_MINIMUM_USABLE_FRACTION,
            retry_policy: PLSC_CONSISTENT_PERMUTATION_RETRY_POLICY.into(),
            group_column: "group".into(),
            group_a: PlscPermutationGroupSummary {
                group: "A".into(),
                observations: 10,
                parameter_values_sha256: "00".repeat(32),
            },
            group_b: PlscPermutationGroupSummary {
                group: "B".into(),
                observations: 10,
                parameter_values_sha256: "00".repeat(32),
            },
            pooled_parameter_values_sha256: "00".repeat(32),
            usable_permutations: 4,
            failed_permutations: Vec::new(),
            permutation_ledger: Vec::new(),
            parameters: vec![PlscPermutationParameterInference {
                parameter: "path:x:y".into(),
                family: PlscPermutationParameterFamily::Path,
                estimate_a: 2.0,
                estimate_b: 1.0,
                original: 1.0,
                exceedances: 3,
                p_value_two_sided: plus_one_probability(3, 4),
                permutations: 4,
            }],
            directional_inference: Some(PlscPermutationDirectionalInference {
                method_version: PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION.into(),
                test_method: PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST.into(),
                parameters: vec![PlscPermutationDirectionalParameterInference {
                    parameter: "path:x:y".into(),
                    greater_or_equal: 2,
                    less_or_equal: 3,
                    p_value_greater: plus_one_probability(2, 4),
                    p_value_less: plus_one_probability(3, 4),
                    permutations: 4,
                }],
            }),
            selected_tail_inference: None,
            warnings: vec![
                PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING.into(),
                PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING.into(),
                PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING.into(),
            ],
        }
    }

    #[test]
    fn directional_plus_one_counts_match_hand_calculation() {
        let counts = permutation_exceedance_counts([-2.0, -1.0, 0.0, 1.0, 2.0, 2.0], 1.0);
        assert_eq!(
            counts,
            PermutationExceedanceCounts {
                two_sided: 5,
                greater_or_equal: 3,
                less_or_equal: 4,
                usable: 6,
            }
        );
        assert_eq!(
            plus_one_probability(counts.two_sided, counts.usable),
            6.0 / 7.0
        );
        assert_eq!(
            plus_one_probability(counts.greater_or_equal, counts.usable),
            4.0 / 7.0
        );
        assert_eq!(
            plus_one_probability(counts.less_or_equal, counts.usable),
            5.0 / 7.0
        );
    }

    #[test]
    fn swapping_groups_swaps_directional_tails_and_retains_two_sided_value() {
        let values = [-2.0, -1.0, 0.0, 1.0, 2.0, 2.0];
        let original = permutation_exceedance_counts(values, 1.0);
        let swapped = permutation_exceedance_counts(values.map(|value| -value), -1.0);
        assert_eq!(swapped.two_sided, original.two_sided);
        assert_eq!(swapped.greater_or_equal, original.less_or_equal);
        assert_eq!(swapped.less_or_equal, original.greater_or_equal);
        assert_eq!(
            plus_one_probability(swapped.greater_or_equal, swapped.usable).to_bits(),
            plus_one_probability(original.less_or_equal, original.usable).to_bits()
        );
        assert_eq!(
            plus_one_probability(swapped.less_or_equal, swapped.usable).to_bits(),
            plus_one_probability(original.greater_or_equal, original.usable).to_bits()
        );
    }

    #[test]
    fn failed_indices_are_excluded_from_directional_denominator() {
        let outcomes = [Some(-2.0), None, Some(1.0), None, Some(3.0)];
        let counts = permutation_exceedance_counts(outcomes.into_iter().flatten(), 1.0);
        assert_eq!(counts.usable, 3);
        assert_eq!(counts.greater_or_equal, 2);
        assert_eq!(counts.less_or_equal, 2);
        assert_eq!(
            plus_one_probability(counts.greater_or_equal, counts.usable),
            3.0 / 4.0
        );
    }

    #[test]
    fn current_directional_contract_rejects_missing_or_tampered_payloads() {
        let valid = directional_validation_fixture();
        validate_directional_inference(&valid, true).unwrap();

        let mut missing = valid.clone();
        missing.directional_inference = None;
        assert!(validate_directional_inference(&missing, true).is_err());

        let mut tampered_count = valid.clone();
        tampered_count
            .directional_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .greater_or_equal = 5;
        assert!(validate_directional_inference(&tampered_count, true).is_err());

        let mut tampered_probability = valid.clone();
        let probability = &mut tampered_probability
            .directional_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .p_value_less;
        *probability = f64::from_bits(probability.to_bits() + 1);
        assert!(validate_directional_inference(&tampered_probability, true).is_err());

        let mut historical = valid;
        historical.test_method = PLSC_CONSISTENT_PERMUTATION_TEST_V1.into();
        historical.directional_inference = None;
        historical.warnings[2] = PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1.into();
        validate_directional_inference(&historical, false).unwrap();
    }

    #[test]
    fn selected_tail_receipt_binds_direction_order_denominator_count_and_probability() {
        let default = directional_validation_fixture();
        let default_wire = serde_json::to_value(&default).unwrap();
        assert!(default_wire.get("selected_tail_inference").is_none());
        assert_eq!(
            serde_json::from_value::<PlscConsistentPermutationResult>(default_wire).unwrap(),
            default
        );

        let mut greater = directional_validation_fixture();
        greater.selected_tail_inference = Some(PlscPermutationSelectedTailInference {
            method_version: PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION.into(),
            orientation: PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION.into(),
            selected_test_tail: PlscPermutationTestTail::GroupAGreater,
            parameters: vec![PlscPermutationSelectedTailParameterInference {
                parameter: "path:x:y".into(),
                selected_exceedances: 2,
                selected_p_value: plus_one_probability(2, 4),
                permutations: 4,
            }],
        });
        validate_selected_tail_inference(
            &greater,
            true,
            Some(PlscPermutationTestTail::GroupAGreater),
        )
        .unwrap();

        let mut less = greater.clone();
        let receipt = less.selected_tail_inference.as_mut().unwrap();
        receipt.selected_test_tail = PlscPermutationTestTail::GroupALess;
        receipt.parameters[0].selected_exceedances = 3;
        receipt.parameters[0].selected_p_value = plus_one_probability(3, 4);
        validate_selected_tail_inference(&less, true, Some(PlscPermutationTestTail::GroupALess))
            .unwrap();

        let mut wrong_order = greater.clone();
        wrong_order
            .selected_tail_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .parameter = "path:y:x".into();
        assert!(validate_selected_tail_inference(&wrong_order, true, None).is_err());

        let mut wrong_denominator = greater.clone();
        wrong_denominator
            .selected_tail_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .permutations = 3;
        assert!(validate_selected_tail_inference(&wrong_denominator, true, None).is_err());

        let mut wrong_count = greater.clone();
        wrong_count
            .selected_tail_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .selected_exceedances = 3;
        assert!(validate_selected_tail_inference(&wrong_count, true, None).is_err());

        let mut wrong_probability = greater.clone();
        let probability = &mut wrong_probability
            .selected_tail_inference
            .as_mut()
            .unwrap()
            .parameters[0]
            .selected_p_value;
        *probability = f64::from_bits(probability.to_bits() + 1);
        assert!(validate_selected_tail_inference(&wrong_probability, true, None).is_err());

        assert!(
            validate_selected_tail_inference(
                &greater,
                true,
                Some(PlscPermutationTestTail::GroupALess),
            )
            .is_err()
        );
        assert!(
            validate_selected_tail_inference(
                &directional_validation_fixture(),
                true,
                Some(PlscPermutationTestTail::GroupAGreater),
            )
            .is_err()
        );

        let mut historical = greater;
        historical.test_method = PLSC_CONSISTENT_PERMUTATION_TEST_V1.into();
        historical.directional_inference = None;
        assert!(validate_selected_tail_inference(&historical, false, None).is_err());
    }

    #[test]
    fn indexed_directional_counts_are_seed_and_worker_order_invariant() {
        let labels = canonical_labels_for_validation("A", 17, "B", 11);
        let indexed_value = |index| {
            permuted_labels(
                &labels,
                20_260_815,
                PLSC_CONSISTENT_PERMUTATION_OPERATION,
                index,
            )
            .iter()
            .enumerate()
            .map(|(position, label)| {
                if *label == 0 {
                    position as f64
                } else {
                    -(position as f64)
                }
            })
            .sum::<f64>()
        };
        let serial = (0..99).map(indexed_value).collect::<Vec<_>>();
        let worker_order = (0..4)
            .flat_map(|worker| (worker..99).step_by(4))
            .map(indexed_value)
            .collect::<Vec<_>>();
        assert_eq!(
            permutation_exceedance_counts(serial.clone(), 5.0),
            permutation_exceedance_counts(serial, 5.0)
        );
        assert_eq!(
            permutation_exceedance_counts(worker_order, 5.0),
            permutation_exceedance_counts((0..99).map(indexed_value), 5.0)
        );
    }

    #[test]
    fn indexed_assignments_preserve_group_sizes_and_swap_as_complements() {
        let labels = canonical_labels_for_validation("A", 17, "B", 11);
        let swapped = canonical_labels_for_validation("B", 11, "A", 17);
        for index in 0..99 {
            let assignment = permuted_labels(
                &labels,
                20_260_815,
                PLSC_CONSISTENT_PERMUTATION_OPERATION,
                index,
            );
            let swapped_assignment = permuted_labels(
                &swapped,
                20_260_815,
                PLSC_CONSISTENT_PERMUTATION_OPERATION,
                index,
            );
            assert_eq!(assignment.iter().filter(|label| **label == 0).count(), 17);
            assert_eq!(assignment.iter().filter(|label| **label == 1).count(), 11);
            assert!(
                assignment
                    .iter()
                    .zip(swapped_assignment)
                    .all(|(left, right)| *left == 1 - right)
            );
        }
    }

    #[test]
    fn assignment_digest_is_index_and_direction_stable() {
        let labels = canonical_labels_for_validation("B", 13, "A", 19);
        let first = permuted_labels(&labels, 91, PLSC_CONSISTENT_PERMUTATION_OPERATION, 7);
        let repeat = permuted_labels(&labels, 91, PLSC_CONSISTENT_PERMUTATION_OPERATION, 7);
        let next = permuted_labels(&labels, 91, PLSC_CONSISTENT_PERMUTATION_OPERATION, 8);
        assert_eq!(
            label_assignment_sha256(&first),
            label_assignment_sha256(&repeat)
        );
        assert_ne!(
            label_assignment_sha256(&first),
            label_assignment_sha256(&next)
        );
    }
}
