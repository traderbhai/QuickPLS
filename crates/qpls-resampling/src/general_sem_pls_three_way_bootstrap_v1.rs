use super::{
    BootstrapPlan, ReplicateOutcome, ResamplingError, ResamplingProgress, complete_case_rows,
    resample_model_dataset, run_bootstrap, type7_quantile,
};
use crate::general_sem_pls_bootstrap_v1::{
    align_general_sem_pls_moderation_score_vectors_v1, validate_point_execution_plan_domain_v1,
};
use qpls_core::{
    AnalysisMethod, CompiledPlsPlanV3, GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
    GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
    GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
    GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1,
    GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
    GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1, GeneralSemBootstrapIntervalV1,
    GeneralSemConfigV1, GeneralSemConfigV1ValidationError, GeneralSemInferenceTailV1,
    GeneralSemInferenceV1, MethodConfig, PlsAlgorithmConfigV2, ValidatedExecutionRecipe,
    pls_general_three_way_moderation_bootstrap_capability_cell_v1,
    pls_general_three_way_moderation_point_capability_cell_v1,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, GeneralSemPlsThreeWayPointErrorV1, GeneralSemPlsThreeWayPointResultV1,
    PLS_METHOD_VERSION, PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2,
    PLS_SCORE_EXECUTION_METHOD_VERSION_V2, PlsResult,
    estimate_general_sem_pls_three_way_moderation_v1_with_control,
    estimate_pls_validated_with_compiled_plan_v2_with_control, estimate_pls_validated_with_control,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

pub const GENERAL_SEM_PLS_THREE_WAY_BOOTSTRAP_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_THREE_WAY_SIGN_ALIGNMENT_VERSION_V1: &str =
    "sampled_original_construct_score_covariance_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayBootstrapInferenceV1 {
    pub target_id: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bootstrap_bias: f64,
    pub standard_error: f64,
    pub lower: f64,
    pub upper: f64,
    pub p_value_two_sided: f64,
    pub usable_replicates: u32,
    pub two_sided_exceedances: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneralSemPlsThreeWayBootstrapFailureCodeV1 {
    InsufficientObservations,
    ConstantIndicator,
    StageOneRankDeficient,
    IsolatedConstruct,
    StageOneNonconvergence,
    IndeterminateScoreSign,
    ConstantConstructScore,
    ConstantInteractionProduct,
    JointStageRankDeficient,
    TargetInventoryMismatch,
    NumericalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: GeneralSemPlsThreeWayBootstrapFailureCodeV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayBootstrapResultV1 {
    pub schema_version: u32,
    pub point_capability_cell: qpls_core::CapabilityCellReferenceV2,
    pub bootstrap_capability_cell: qpls_core::CapabilityCellReferenceV2,
    pub method_version: String,
    pub point_method_version: String,
    pub resampling_operation_version: String,
    pub resampling_stream_version: String,
    pub quantile_method_version: String,
    pub standard_error_method_version: String,
    pub summation_method_version: String,
    pub p_value_method_version: String,
    pub failure_policy_version: String,
    pub sign_alignment_method_version: String,
    pub product_scale_version: String,
    pub probe_policy_version: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub model_scientific_sha256: String,
    pub stage_one_model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub target_identity_set_sha256: String,
    pub target_ids: Vec<String>,
    pub interval: GeneralSemBootstrapIntervalV1,
    pub tail: GeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub shared_stage_one_reestimated_per_replicate: bool,
    pub score_vectors_sign_aligned_before_products: bool,
    pub all_lower_order_and_three_way_products_recomputed_per_replicate: bool,
    pub joint_stage_two_reestimated_per_replicate: bool,
    pub complete_joint_point_contract_validated_per_replicate: bool,
    pub all_three_way_targets_share_one_replicate_ledger: bool,
    pub failed_replicates: Vec<GeneralSemPlsThreeWayBootstrapFailedReplicateV1>,
    pub targets: Vec<GeneralSemPlsThreeWayBootstrapInferenceV1>,
}

impl GeneralSemPlsThreeWayBootstrapResultV1 {
    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
        point: &GeneralSemPlsThreeWayPointResultV1,
    ) -> Result<(), GeneralSemPlsThreeWayBootstrapErrorV1> {
        point.ensure_valid_against_plan_v1(plan)?;
        let expected_values = point.target_values_v1();
        let expected_ids = expected_values.keys().cloned().collect::<Vec<_>>();
        let expected_minimum = ((f64::from(self.resamples_requested) * 0.9).ceil() as u32).max(2);
        if self.schema_version != GENERAL_SEM_PLS_THREE_WAY_BOOTSTRAP_SCHEMA_VERSION_V1
            || self.point_capability_cell
                != pls_general_three_way_moderation_point_capability_cell_v1()
            || self.bootstrap_capability_cell
                != pls_general_three_way_moderation_bootstrap_capability_cell_v1()
            || self.method_version
                != GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
            || self.point_method_version
                != GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
            || self.resampling_operation_version
                != GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
            || self.resampling_stream_version
                != GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1
            || self.quantile_method_version != GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1
            || self.standard_error_method_version
                != GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
            || self.summation_method_version != GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1
            || self.p_value_method_version
                != GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
            || self.failure_policy_version != GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1
            || self.sign_alignment_method_version
                != GENERAL_SEM_PLS_THREE_WAY_SIGN_ALIGNMENT_VERSION_V1
            || self.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
            || self.probe_policy_version != GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1
            || self.compiled_plan_sha256 != plan.deterministic_sha256()
            || self.general_sem_config_sha256 != plan.general_sem_config_sha256()
            || self.model_scientific_sha256 != plan.scientific_hash()
            || self.stage_one_model_scientific_sha256
                != plan.stage_one_projection_scientific_sha256().unwrap_or("")
            || self.target_ids != expected_ids
            || self
                .targets
                .iter()
                .map(|row| row.target_id.as_str())
                .collect::<Vec<_>>()
                != self
                    .target_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            || self.resamples_usable as usize + self.failed_replicates.len()
                != self.resamples_requested as usize
            || self
                .targets
                .iter()
                .any(|row| row.usable_replicates != self.resamples_usable)
            || self.minimum_usable_resamples != expected_minimum
            || self.resamples_usable < expected_minimum
            || self.resamples_usable > self.resamples_requested
            || !(2..=10_000).contains(&self.resamples_requested)
            || !(1..=64).contains(&self.workers)
            || !self.confidence_level.is_finite()
            || !(0.0..1.0).contains(&self.confidence_level)
            || !self.complete_model_reestimated_per_replicate
            || !self.shared_stage_one_reestimated_per_replicate
            || !self.score_vectors_sign_aligned_before_products
            || !self.all_lower_order_and_three_way_products_recomputed_per_replicate
            || !self.joint_stage_two_reestimated_per_replicate
            || !self.complete_joint_point_contract_validated_per_replicate
            || !self.all_three_way_targets_share_one_replicate_ledger
        {
            return Err(GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(
                "three-way bootstrap receipt or target inventory differs from its exact contract".into(),
            ));
        }
        if self.target_identity_set_sha256 != sha256_value(&self.target_ids)?
            || self.target_ids.iter().any(|id| id.trim().is_empty())
            || self.target_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(
                GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(
                    "three-way bootstrap target identities are not canonical".into(),
                ),
            );
        }
        for target in &self.targets {
            let expected = expected_values.get(&target.target_id).ok_or_else(|| {
                GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(format!(
                    "unknown three-way bootstrap target {}",
                    target.target_id,
                ))
            })?;
            let expected_p = f64::from(target.two_sided_exceedances + 1)
                / f64::from(target.usable_replicates + 1);
            if target.original.to_bits() != expected.to_bits()
                || target.bootstrap_bias.to_bits()
                    != (target.bootstrap_mean - target.original).to_bits()
                || [
                    target.original,
                    target.bootstrap_mean,
                    target.bootstrap_bias,
                    target.standard_error,
                    target.lower,
                    target.upper,
                    target.p_value_two_sided,
                ]
                .iter()
                .any(|value| !value.is_finite())
                || target.standard_error < 0.0
                || target.lower > target.upper
                || target.two_sided_exceedances > target.usable_replicates
                || target.p_value_two_sided.to_bits() != expected_p.to_bits()
            {
                return Err(
                    GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(format!(
                        "three-way bootstrap target {} is internally inconsistent",
                        target.target_id
                    )),
                );
            }
        }
        let mut previous_failure = None;
        let mut failed_indices = BTreeSet::new();
        for failure in &self.failed_replicates {
            if failure.replicate_index >= self.resamples_requested
                || previous_failure.is_some_and(|previous| previous >= failure.replicate_index)
                || failure.message.trim().is_empty()
            {
                return Err(
                    GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(
                        "three-way bootstrap failure ledger is not canonical".into(),
                    ),
                );
            }
            previous_failure = Some(failure.replicate_index);
            failed_indices.insert(failure.replicate_index);
        }
        let usable_indices = (0..self.resamples_requested)
            .filter(|index| !failed_indices.contains(index))
            .collect::<Vec<_>>();
        if self.usable_replicate_indices_sha256 != sha256_value(&usable_indices)?
            || [
                &self.general_sem_config_sha256,
                &self.compiled_plan_sha256,
                &self.model_scientific_sha256,
                &self.stage_one_model_scientific_sha256,
                &self.complete_case_frame_sha256,
                &self.usable_replicate_indices_sha256,
                &self.target_identity_set_sha256,
            ]
            .iter()
            .any(|value| !is_lowercase_sha256(value))
        {
            return Err(
                GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(
                    "three-way bootstrap digest ledger is invalid".into(),
                ),
            );
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemPlsThreeWayBootstrapErrorV1 {
    #[error(transparent)]
    InvalidConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error("three-way bootstrap requires raw observations")]
    RawDataRequired,
    #[error("three-way bootstrap requires an exact three-way compiled plan")]
    ThreeWayPlanRequired,
    #[error("three-way bootstrap requires a point-only PLS execution")]
    InvalidPointExecution,
    #[error("three-way bootstrap requires percentile, two-sided case bootstrap")]
    InvalidInference,
    #[error("three-way bootstrap config differs from the compiled plan")]
    ConfigMismatch,
    #[error("three-way bootstrap point initialization differs from the point execution")]
    InitializationMismatch,
    #[error("three-way point estimate does not match an independent deterministic refit")]
    PointMismatch,
    #[error("three-way bootstrap original stage-one result is invalid")]
    OriginalStageOneInvalid,
    #[error("three-way bootstrap original point result is invalid")]
    OriginalPointInvalid,
    #[error(
        "three-way bootstrap retained {usable} usable replicates; at least {required} are required"
    )]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error(
        "three-way bootstrap replicate {replicate_index} violated its target contract: {message}"
    )]
    ReplicateContract {
        replicate_index: u32,
        message: String,
    },
    #[error("three-way bootstrap result contract is invalid: {0}")]
    InvalidResultContract(String),
    #[error("three-way bootstrap value could not be serialized: {0}")]
    Serialization(String),
    #[error(transparent)]
    Point(#[from] GeneralSemPlsThreeWayPointErrorV1),
    #[error(transparent)]
    Estimation(#[from] EstimationError),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

enum ReplicateRecordV1 {
    Usable(BTreeMap<String, f64>),
    Failed(GeneralSemPlsThreeWayBootstrapFailureCodeV1, String),
    Fatal(String),
    Cancelled,
}

#[derive(Serialize)]
struct CompleteCaseFrameIdentityV1 {
    dataset_id: String,
    dataset_fingerprint: String,
    source_columns: Vec<String>,
    raw_row_indices: Vec<u64>,
}

pub fn bootstrap_general_sem_pls_three_way_moderation_v1(
    dataset: &Dataset,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
    original_stage_one: &PlsResult,
    original_point: &GeneralSemPlsThreeWayPointResultV1,
    config: &GeneralSemConfigV1,
    initialization: Option<&PlsAlgorithmConfigV2>,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<GeneralSemPlsThreeWayBootstrapResultV1, GeneralSemPlsThreeWayBootstrapErrorV1> {
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }
    config.ensure_valid()?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::RawDataRequired);
    }
    if plan.three_way_interaction().is_none() {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::ThreeWayPlanRequired);
    }
    if point_execution.source().settings.method != AnalysisMethod::PlsPm
        || point_execution.source().settings.bootstrap_samples != 0
        || point_execution.source().settings.studentized_inner_samples != 0
        || point_execution.source().settings.permutation_samples != 0
    {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::InvalidPointExecution);
    }
    let GeneralSemInferenceV1::CaseBootstrap {
        resamples,
        seed,
        confidence_level,
        interval,
        tail,
    } = config.inference
    else {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::InvalidInference);
    };
    if interval != GeneralSemBootstrapIntervalV1::Percentile
        || tail != GeneralSemInferenceTailV1::TwoSided
    {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::InvalidInference);
    }
    let config_sha256 = sha256_value(config)?;
    if config_sha256 != plan.general_sem_config_sha256() {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::ConfigMismatch);
    }
    validate_point_execution_plan_domain_v1(dataset, point_execution, plan)
        .map_err(|_| GeneralSemPlsThreeWayBootstrapErrorV1::InvalidPointExecution)?;
    let configured_initialization = match point_execution.source().method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(value)) => Some(value),
        _ => None,
    };
    if configured_initialization != initialization {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::InitializationMismatch);
    }
    if !original_stage_one.converged {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::OriginalStageOneInvalid);
    }
    let compiled_scoring = initialization.is_some()
        || plan
            .base_plan()
            .blocks()
            .iter()
            .any(|block| block.fixed_scoring().is_some());
    let identity_valid = if compiled_scoring {
        original_stage_one.method_version == PLS_SCORE_EXECUTION_METHOD_VERSION_V2
            && original_stage_one
                .score_execution
                .as_ref()
                .is_some_and(|receipt| {
                    receipt.contract_version == PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2
                })
    } else {
        original_stage_one.method_version == PLS_METHOD_VERSION
            && original_stage_one.score_execution.is_none()
    };
    if !identity_valid {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::OriginalStageOneInvalid);
    }
    let refit = if compiled_scoring {
        estimate_pls_validated_with_compiled_plan_v2_with_control(
            dataset,
            point_execution,
            plan.base_plan(),
            initialization,
            |_| !is_cancelled(),
        )
    } else {
        estimate_pls_validated_with_control(dataset, point_execution, |_| !is_cancelled())
    }?;
    if refit != *original_stage_one {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::PointMismatch);
    }
    let refit_point = estimate_general_sem_pls_three_way_moderation_v1_with_control(
        plan,
        &refit.construct_scores,
        || !is_cancelled(),
    )?;
    if refit_point != *original_point {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::PointMismatch);
    }
    original_point
        .ensure_valid_against_plan_v1(plan)
        .map_err(|_| GeneralSemPlsThreeWayBootstrapErrorV1::OriginalPointInvalid)?;
    let original_values = original_point.target_values_v1();
    let target_ids = original_values.keys().cloned().collect::<Vec<_>>();
    let base_recipe = point_execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| {
            GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(error.to_string())
        })?;
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if original_stage_one.used_observations != complete_rows.len() {
        return Err(GeneralSemPlsThreeWayBootstrapErrorV1::OriginalStageOneInvalid);
    }
    let bootstrap_plan = BootstrapPlan {
        replicates: resamples,
        master_seed: seed,
        operation: GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1.into(),
    };
    let cancel = &is_cancelled;
    let run = run_bootstrap(
        complete_rows.len(),
        &bootstrap_plan,
        workers,
        |_index, sampled_positions| {
            let record = match estimate_replicate(
                dataset,
                base_recipe,
                point_execution,
                plan,
                &original_stage_one.construct_scores,
                &complete_rows,
                sampled_positions,
                compiled_scoring,
                initialization,
                cancel,
            ) {
                Ok(point) => {
                    let values = point.target_values_v1();
                    if values.keys().cloned().collect::<Vec<_>>() == target_ids {
                        ReplicateRecordV1::Usable(values)
                    } else {
                        ReplicateRecordV1::Failed(GeneralSemPlsThreeWayBootstrapFailureCodeV1::TargetInventoryMismatch, "replicate probe/target inventory differs from the original point target inventory".into())
                    }
                }
                Err(ReplicateFailureV1::Cancelled) => ReplicateRecordV1::Cancelled,
                Err(ReplicateFailureV1::Failed(code, message)) => {
                    ReplicateRecordV1::Failed(code, message)
                }
                Err(ReplicateFailureV1::Fatal(message)) => ReplicateRecordV1::Fatal(message),
            };
            Ok::<_, Infallible>(record)
        },
        cancel,
        report_progress,
    )?;
    let mut usable_indices = Vec::new();
    let mut usable = Vec::new();
    let mut failures = Vec::new();
    for (index, outcome) in run.outcomes.into_iter().enumerate() {
        let ReplicateOutcome::Success { value } = outcome else {
            return Err(
                GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(
                    "infallible scheduler returned a failed outer result".into(),
                ),
            );
        };
        match value {
            ReplicateRecordV1::Usable(values) => {
                usable_indices.push(index as u32);
                usable.push(values);
            }
            ReplicateRecordV1::Failed(reason_code, message) => {
                failures.push(GeneralSemPlsThreeWayBootstrapFailedReplicateV1 {
                    replicate_index: index as u32,
                    reason_code,
                    message,
                })
            }
            ReplicateRecordV1::Fatal(message) => {
                return Err(GeneralSemPlsThreeWayBootstrapErrorV1::ReplicateContract {
                    replicate_index: index as u32,
                    message,
                });
            }
            ReplicateRecordV1::Cancelled => return Err(ResamplingError::Cancelled.into()),
        }
    }
    let minimum_usable = ((f64::from(resamples) * 0.9).ceil() as usize).max(2);
    if usable.len() < minimum_usable {
        return Err(
            GeneralSemPlsThreeWayBootstrapErrorV1::InsufficientUsableReplicates {
                usable: usable.len(),
                required: minimum_usable,
            },
        );
    }
    let targets = summarize_targets(&target_ids, &original_values, &usable, confidence_level)?;
    let source_columns = plan
        .base_plan()
        .blocks()
        .iter()
        .flat_map(|block| block.indicators())
        .map(|indicator| indicator.source_column().to_string())
        .collect::<Vec<_>>();
    let complete_case_frame_sha256 = sha256_value(&CompleteCaseFrameIdentityV1 {
        dataset_id: dataset.id.to_string(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        source_columns,
        raw_row_indices: complete_rows.iter().map(|row| *row as u64).collect(),
    })?;
    let result = GeneralSemPlsThreeWayBootstrapResultV1 {
        schema_version: GENERAL_SEM_PLS_THREE_WAY_BOOTSTRAP_SCHEMA_VERSION_V1,
        point_capability_cell: pls_general_three_way_moderation_point_capability_cell_v1(),
        bootstrap_capability_cell: pls_general_three_way_moderation_bootstrap_capability_cell_v1(),
        method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1.into(),
        point_method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1.into(),
        resampling_operation_version:
            GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1.into(),
        resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1.into(),
        quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.into(),
        standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1.into(),
        summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1.into(),
        p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1.into(),
        failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1.into(),
        sign_alignment_method_version: GENERAL_SEM_PLS_THREE_WAY_SIGN_ALIGNMENT_VERSION_V1.into(),
        product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
        probe_policy_version: GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1.into(),
        general_sem_config_sha256: config_sha256,
        compiled_plan_sha256: plan.deterministic_sha256(),
        model_scientific_sha256: plan.scientific_hash().into(),
        stage_one_model_scientific_sha256: plan
            .stage_one_projection_scientific_sha256()
            .unwrap_or("")
            .into(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        complete_case_frame_sha256,
        usable_replicate_indices_sha256: sha256_value(&usable_indices)?,
        target_identity_set_sha256: sha256_value(&target_ids)?,
        target_ids,
        interval,
        tail,
        confidence_level,
        resamples_requested: resamples,
        resamples_usable: usable.len() as u32,
        minimum_usable_resamples: minimum_usable as u32,
        seed: seed.to_string(),
        workers: workers as u32,
        complete_model_reestimated_per_replicate: true,
        shared_stage_one_reestimated_per_replicate: true,
        score_vectors_sign_aligned_before_products: true,
        all_lower_order_and_three_way_products_recomputed_per_replicate: true,
        joint_stage_two_reestimated_per_replicate: true,
        complete_joint_point_contract_validated_per_replicate: true,
        all_three_way_targets_share_one_replicate_ledger: true,
        failed_replicates: failures,
        targets,
    };
    result.ensure_valid_against_plan_v1(plan, original_point)?;
    Ok(result)
}

enum ReplicateFailureV1 {
    Cancelled,
    Failed(GeneralSemPlsThreeWayBootstrapFailureCodeV1, String),
    Fatal(String),
}

#[allow(clippy::too_many_arguments)]
fn estimate_replicate(
    dataset: &Dataset,
    base_recipe: &qpls_core::AnalysisRecipe,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
    original_scores: &BTreeMap<String, Vec<f64>>,
    complete_rows: &[usize],
    sampled_positions: &[usize],
    compiled_scoring: bool,
    initialization: Option<&PlsAlgorithmConfigV2>,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<GeneralSemPlsThreeWayPointResultV1, ReplicateFailureV1> {
    let raw_indices = sampled_positions
        .iter()
        .map(|position| complete_rows[*position])
        .collect::<Vec<_>>();
    let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, is_cancelled)
        .map_err(map_estimation_failure)?;
    let mut stage_one = if compiled_scoring {
        estimate_pls_validated_with_compiled_plan_v2_with_control(
            &sampled,
            point_execution,
            plan.base_plan(),
            initialization,
            |_| !is_cancelled(),
        )
    } else {
        estimate_pls_validated_with_control(&sampled, point_execution, |_| !is_cancelled())
    }
    .map_err(map_estimation_failure)?;
    align_general_sem_pls_moderation_score_vectors_v1(
        &mut stage_one.construct_scores,
        original_scores,
        sampled_positions,
        is_cancelled,
    )
    .map_err(|error| {
        if error.to_string().contains("cancelled") {
            ReplicateFailureV1::Cancelled
        } else {
            ReplicateFailureV1::Failed(
                GeneralSemPlsThreeWayBootstrapFailureCodeV1::IndeterminateScoreSign,
                error.to_string(),
            )
        }
    })?;
    estimate_general_sem_pls_three_way_moderation_v1_with_control(
        plan,
        &stage_one.construct_scores,
        || !is_cancelled(),
    )
    .map_err(map_point_failure)
}

fn map_estimation_failure(error: EstimationError) -> ReplicateFailureV1 {
    let message = error.to_string();
    match error {
        EstimationError::Cancelled => ReplicateFailureV1::Cancelled,
        EstimationError::InsufficientObservations => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::InsufficientObservations,
            message,
        ),
        EstimationError::ConstantIndicator(_) => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::ConstantIndicator,
            message,
        ),
        EstimationError::RankDeficient(_) => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::StageOneRankDeficient,
            message,
        ),
        EstimationError::IsolatedConstruct(_) => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::IsolatedConstruct,
            message,
        ),
        EstimationError::NonConvergence(_) => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::StageOneNonconvergence,
            message,
        ),
        EstimationError::Numerical(_) => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::NumericalFailure,
            message,
        ),
        _ => ReplicateFailureV1::Fatal(message),
    }
}

fn map_point_failure(error: GeneralSemPlsThreeWayPointErrorV1) -> ReplicateFailureV1 {
    let message = error.to_string();
    match error {
        GeneralSemPlsThreeWayPointErrorV1::Cancelled => ReplicateFailureV1::Cancelled,
        GeneralSemPlsThreeWayPointErrorV1::InsufficientObservations { .. } => {
            ReplicateFailureV1::Failed(
                GeneralSemPlsThreeWayBootstrapFailureCodeV1::InsufficientObservations,
                message,
            )
        }
        GeneralSemPlsThreeWayPointErrorV1::InvalidStageOneScore { .. } => {
            ReplicateFailureV1::Failed(
                GeneralSemPlsThreeWayBootstrapFailureCodeV1::ConstantConstructScore,
                message,
            )
        }
        GeneralSemPlsThreeWayPointErrorV1::ConstantProduct { .. } => ReplicateFailureV1::Failed(
            GeneralSemPlsThreeWayBootstrapFailureCodeV1::ConstantInteractionProduct,
            message,
        ),
        GeneralSemPlsThreeWayPointErrorV1::RankDeficient { .. }
        | GeneralSemPlsThreeWayPointErrorV1::InsufficientEquationObservations { .. } => {
            ReplicateFailureV1::Failed(
                GeneralSemPlsThreeWayBootstrapFailureCodeV1::JointStageRankDeficient,
                message,
            )
        }
        GeneralSemPlsThreeWayPointErrorV1::InvalidResultContract(_)
        | GeneralSemPlsThreeWayPointErrorV1::NoThreeWayInteraction
        | GeneralSemPlsThreeWayPointErrorV1::LowerOrderCoefficientMissing { .. } => {
            ReplicateFailureV1::Fatal(message)
        }
        GeneralSemPlsThreeWayPointErrorV1::MissingStageOneScore { .. }
        | GeneralSemPlsThreeWayPointErrorV1::ScoreLengthMismatch { .. } => {
            ReplicateFailureV1::Fatal(message)
        }
    }
}

fn summarize_targets(
    target_ids: &[String],
    originals: &BTreeMap<String, f64>,
    replicates: &[BTreeMap<String, f64>],
    confidence_level: f64,
) -> Result<Vec<GeneralSemPlsThreeWayBootstrapInferenceV1>, GeneralSemPlsThreeWayBootstrapErrorV1> {
    target_ids
        .iter()
        .map(|target_id| {
            let original = *originals.get(target_id).ok_or_else(|| {
                GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(format!(
                    "missing original target {target_id}"
                ))
            })?;
            let mut values = replicates
                .iter()
                .map(|rows| {
                    rows.get(target_id).copied().ok_or_else(|| {
                        GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(format!(
                            "replicate missing target {target_id}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if !original.is_finite()
                || values.len() < 2
                || values.iter().any(|value| !value.is_finite())
            {
                return Err(
                    GeneralSemPlsThreeWayBootstrapErrorV1::InvalidResultContract(format!(
                        "target {target_id} has an invalid bootstrap sample"
                    )),
                );
            }
            let mean = stable_sum(&values) / values.len() as f64;
            let standard_error = (stable_sum(
                &values
                    .iter()
                    .map(|value| (value - mean).powi(2))
                    .collect::<Vec<_>>(),
            ) / (values.len() - 1) as f64)
                .sqrt();
            let exceedances = values
                .iter()
                .filter(|value| (**value - original).abs() >= original.abs())
                .count();
            values.sort_by(f64::total_cmp);
            let alpha = 1.0 - confidence_level;
            Ok(GeneralSemPlsThreeWayBootstrapInferenceV1 {
                target_id: target_id.clone(),
                original,
                bootstrap_mean: mean,
                bootstrap_bias: mean - original,
                standard_error,
                lower: type7_quantile(&values, alpha / 2.0),
                upper: type7_quantile(&values, 1.0 - alpha / 2.0),
                p_value_two_sided: (exceedances + 1) as f64 / (values.len() + 1) as f64,
                usable_replicates: values.len() as u32,
                two_sided_exceedances: exceedances as u32,
            })
        })
        .collect()
}

fn stable_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for value in values {
        let updated = sum + value;
        compensation += if sum.abs() >= value.abs() {
            (sum - updated) + value
        } else {
            (value - updated) + sum
        };
        sum = updated;
    }
    sum + compensation
}

fn sha256_value<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, GeneralSemPlsThreeWayBootstrapErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| GeneralSemPlsThreeWayBootstrapErrorV1::Serialization(error.to_string()))
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_way_bootstrap_versions_are_additive_and_do_not_alias_two_way() {
        assert_ne!(
            GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
            qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        );
        assert_ne!(
            pls_general_three_way_moderation_bootstrap_capability_cell_v1().cell_id,
            qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1().cell_id
        );
    }
}
