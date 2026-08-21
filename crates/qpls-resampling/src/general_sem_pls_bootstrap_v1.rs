use super::{
    BootstrapPlan, ReplicateOutcome, ResamplingError, ResamplingProgress, complete_case_rows,
    resample_model_dataset, run_bootstrap, type7_quantile,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, CompiledPlsBlockModeV2, CompiledPlsEffectEstimandV3,
    CompiledPlsPlanV3, GeneralSemBootstrapIntervalV1, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemEffectsV1, GeneralSemEffectsV1Error,
    GeneralSemInferenceTailV1, GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1,
    MeasurementMode, MethodConfig, PlsAlgorithmConfigV2, StructuralRelationRoleV4,
    ValidatedExecutionRecipe, compiled_pls_effect_identities_v1, decompose_general_sem_effects_v1,
    general_sem_effect_identity_set_sha256_v1,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1, GeneralSemPlsInteractionPointErrorV1,
    GeneralSemPlsMultipleInteractionPointResultV1, PLS_METHOD_VERSION,
    PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2, PLS_SCORE_EXECUTION_METHOD_VERSION_V2, PlsResult,
    estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control,
    estimate_pls_validated_with_compiled_plan_v2_with_control, estimate_pls_validated_with_control,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

pub const GENERAL_SEM_PLS_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_METHOD_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1;

pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_METHOD_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1;
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1: &str =
    "sampled_original_construct_score_covariance_v1";
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1: &str =
    "compiled_interaction_scientific_rescaled_gamma_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneralSemPlsModerationGammaTargetKindV1 {
    InteractionScientificRescaledGamma,
}

/// Exact inferential target for the parity-first moderation bootstrap cell.
/// Every replicate still refits and validates the full joint equation, but
/// only the scientific gamma (the coefficient per standardized moderator
/// unit) is summarized as bootstrap inference in this version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsModerationGammaTargetIdentityV1 {
    pub kind: GeneralSemPlsModerationGammaTargetKindV1,
    pub target_version: String,
    pub target_id: String,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub interaction_effect_relation_id: String,
    pub interaction_effect_parameter_id: String,
    pub generated_product_column_id: String,
    pub focal_predictor_id: String,
    pub moderator_id: String,
    pub outcome_id: String,
    pub stage_one_model_scientific_sha256: String,
    pub product_scale_version: String,
    pub method_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsModerationBootstrapGammaInferenceV1 {
    pub target: GeneralSemPlsModerationGammaTargetIdentityV1,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneralSemPlsModerationBootstrapFailureCodeV1 {
    InsufficientObservations,
    ConstantIndicator,
    StageOneRankDeficient,
    IsolatedConstruct,
    StageOneNonconvergence,
    IndeterminateScoreSign,
    ConstantConstructScore,
    ConstantInteractionProduct,
    JointStageRankDeficient,
    NumericalFailure,
}

impl GeneralSemPlsModerationBootstrapFailureCodeV1 {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InsufficientObservations => "insufficient_observations",
            Self::ConstantIndicator => "constant_indicator",
            Self::StageOneRankDeficient => "stage_one_rank_deficient",
            Self::IsolatedConstruct => "isolated_construct",
            Self::StageOneNonconvergence => "stage_one_nonconvergence",
            Self::IndeterminateScoreSign => "indeterminate_score_sign",
            Self::ConstantConstructScore => "constant_construct_score",
            Self::ConstantInteractionProduct => "constant_interaction_product",
            Self::JointStageRankDeficient => "joint_stage_rank_deficient",
            Self::NumericalFailure => "numerical_failure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsModerationBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: GeneralSemPlsModerationBootstrapFailureCodeV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsMultipleModerationBootstrapResultV1 {
    pub schema_version: u32,
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
    pub gamma_target_version: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub model_scientific_sha256: String,
    pub stage_one_model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub gamma_target_identity_set_sha256: String,
    pub gamma_target_ids: Vec<String>,
    pub interval: GeneralSemBootstrapIntervalV1,
    pub tail: GeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    /// Decimal u64 wire form; this remains exact in JavaScript runtimes.
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub shared_stage_one_reestimated_per_replicate: bool,
    pub score_vectors_sign_aligned_before_products: bool,
    pub product_scaling_recomputed_per_replicate: bool,
    pub joint_stage_two_reestimated_per_replicate: bool,
    pub complete_joint_point_contract_validated_per_replicate: bool,
    pub failed_replicates: Vec<GeneralSemPlsModerationBootstrapFailedReplicateV1>,
    pub interaction_gammas: Vec<GeneralSemPlsModerationBootstrapGammaInferenceV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsBootstrapEffectInferenceV1 {
    pub effect_id: String,
    pub estimand_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneralSemPlsBootstrapFailureCodeV1 {
    InsufficientObservations,
    ConstantIndicator,
    RankDeficient,
    IsolatedConstruct,
    EstimationNonconvergence,
    NumericalFailure,
}

impl GeneralSemPlsBootstrapFailureCodeV1 {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InsufficientObservations => "insufficient_observations",
            Self::ConstantIndicator => "constant_indicator",
            Self::RankDeficient => "rank_deficient",
            Self::IsolatedConstruct => "isolated_construct",
            Self::EstimationNonconvergence => "estimation_nonconvergence",
            Self::NumericalFailure => "numerical_failure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: GeneralSemPlsBootstrapFailureCodeV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsBootstrapResultV1 {
    pub schema_version: u32,
    pub method_version: String,
    pub resampling_operation_version: String,
    pub resampling_stream_version: String,
    pub quantile_method_version: String,
    pub standard_error_method_version: String,
    pub summation_method_version: String,
    pub p_value_method_version: String,
    pub failure_policy_version: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub model_scientific_sha256: String,
    pub source_dataset_fingerprint: String,
    pub complete_case_frame_sha256: String,
    pub usable_replicate_indices_sha256: String,
    pub effect_identity_set_sha256: String,
    pub effect_ids: Vec<String>,
    pub interval: GeneralSemBootstrapIntervalV1,
    pub tail: GeneralSemInferenceTailV1,
    pub confidence_level: f64,
    pub resamples_requested: u32,
    pub resamples_usable: u32,
    pub minimum_usable_resamples: u32,
    /// Decimal u64 wire form; this remains exact in JavaScript runtimes.
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub failed_replicates: Vec<GeneralSemPlsBootstrapFailedReplicateV1>,
    pub effects: Vec<GeneralSemPlsBootstrapEffectInferenceV1>,
}

impl GeneralSemPlsBootstrapResultV1 {
    pub fn ensure_valid(&self) -> Result<(), GeneralSemPlsBootstrapErrorV1> {
        let invalid = |message: &str| {
            GeneralSemPlsBootstrapErrorV1::InvalidResultContract(message.to_string())
        };
        if self.schema_version != GENERAL_SEM_PLS_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1
            || self.method_version != GENERAL_SEM_PLS_BOOTSTRAP_METHOD_VERSION_V1
            || self.resampling_operation_version != GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1
            || self.resampling_stream_version != GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1
            || self.quantile_method_version != GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1
            || self.standard_error_method_version
                != GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1
            || self.summation_method_version != GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1
            || self.p_value_method_version != GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1
            || self.failure_policy_version != GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1
        {
            return Err(invalid(
                "schema or algorithm version is not the exact v1 contract",
            ));
        }
        for (name, digest) in [
            ("general_sem_config_sha256", &self.general_sem_config_sha256),
            ("compiled_plan_sha256", &self.compiled_plan_sha256),
            ("model_scientific_sha256", &self.model_scientific_sha256),
            (
                "complete_case_frame_sha256",
                &self.complete_case_frame_sha256,
            ),
            (
                "usable_replicate_indices_sha256",
                &self.usable_replicate_indices_sha256,
            ),
            (
                "effect_identity_set_sha256",
                &self.effect_identity_set_sha256,
            ),
        ] {
            if !is_lowercase_sha256_v1(digest) {
                return Err(invalid(&format!("{name} must be a lowercase SHA-256")));
            }
        }
        if !is_dataset_fingerprint_v1(&self.source_dataset_fingerprint) {
            return Err(invalid(
                "source_dataset_fingerprint must be a bare SHA-256 or v2-prefixed SHA-256",
            ));
        }
        if self.effect_ids.is_empty()
            || self
                .effect_ids
                .iter()
                .any(|effect_id| effect_id.trim().is_empty())
            || self.effect_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid(
                "effect_ids must be nonempty, unique, and strictly ordered",
            ));
        }
        if self.interval != GeneralSemBootstrapIntervalV1::Percentile
            || self.tail != GeneralSemInferenceTailV1::TwoSided
            || !self.confidence_level.is_finite()
            || self.confidence_level <= 0.0
            || self.confidence_level >= 1.0
        {
            return Err(invalid(
                "interval, tail, or confidence level is outside the v1 contract",
            ));
        }
        if !(2..=10_000).contains(&self.resamples_requested) {
            return Err(invalid("resamples_requested must be between 2 and 10000"));
        }
        let expected_minimum = minimum_usable_replicates(self.resamples_requested) as u32;
        if self.minimum_usable_resamples != expected_minimum
            || self.resamples_usable < expected_minimum
            || self.resamples_usable > self.resamples_requested
        {
            return Err(invalid(
                "usable replicate counts violate the 90 percent gate",
            ));
        }
        let seed = self
            .seed
            .parse::<u64>()
            .map_err(|_| invalid("seed must be a canonical decimal integer"))?;
        if seed.to_string() != self.seed || seed > qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1
        {
            return Err(invalid(
                "seed must be a JavaScript-safe canonical decimal integer",
            ));
        }
        if !(1..=64).contains(&self.workers) || !self.complete_model_reestimated_per_replicate {
            return Err(invalid(
                "workers or full-model re-estimation flag is invalid",
            ));
        }
        if self.resamples_usable as usize + self.failed_replicates.len()
            != self.resamples_requested as usize
        {
            return Err(invalid(
                "usable and failed ledgers do not cover the requested plan",
            ));
        }
        let mut previous_failure = None;
        let mut failed_indices = BTreeSet::new();
        for failure in &self.failed_replicates {
            if failure.replicate_index >= self.resamples_requested
                || previous_failure.is_some_and(|previous| previous >= failure.replicate_index)
                || failure.message.trim().is_empty()
            {
                return Err(invalid("failed replicate ledger is not canonical"));
            }
            previous_failure = Some(failure.replicate_index);
            failed_indices.insert(failure.replicate_index);
        }
        let usable_indices = (0..self.resamples_requested)
            .filter(|index| !failed_indices.contains(index))
            .collect::<Vec<_>>();
        if usable_indices.len() != self.resamples_usable as usize
            || sha256_serialized(&usable_indices)? != self.usable_replicate_indices_sha256
        {
            return Err(invalid(
                "usable replicate index digest contradicts the failure ledger",
            ));
        }
        let effect_row_ids = self
            .effects
            .iter()
            .map(|effect| effect.effect_id.clone())
            .collect::<Vec<_>>();
        if effect_row_ids != self.effect_ids {
            return Err(invalid(
                "effect rows must exactly cover effect_ids in canonical order",
            ));
        }
        for effect in &self.effects {
            if effect.estimand_id.trim().is_empty()
                || [
                    effect.original,
                    effect.bootstrap_mean,
                    effect.bootstrap_bias,
                    effect.standard_error,
                    effect.lower,
                    effect.upper,
                    effect.p_value_two_sided,
                ]
                .iter()
                .any(|value| !value.is_finite())
                || effect.standard_error < 0.0
                || effect.lower > effect.upper
                || effect.usable_replicates != self.resamples_usable
                || effect.two_sided_exceedances > effect.usable_replicates
                || !approximately_equal_v1(
                    effect.bootstrap_bias,
                    effect.bootstrap_mean - effect.original,
                )
                || !approximately_equal_v1(
                    effect.p_value_two_sided,
                    f64::from(effect.two_sided_exceedances + 1)
                        / f64::from(effect.usable_replicates + 1),
                )
            {
                return Err(invalid(&format!(
                    "effect {} violates the inference summary contract",
                    effect.effect_id
                )));
            }
        }
        Ok(())
    }
}

impl GeneralSemPlsMultipleModerationBootstrapResultV1 {
    pub fn ensure_valid(&self) -> Result<(), GeneralSemPlsMultipleModerationBootstrapErrorV1> {
        let invalid = |message: &str| {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidResultContract(
                message.to_string(),
            )
        };
        if self.schema_version
            != GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1
            || self.method_version
                != GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
            || self.point_method_version != GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
            || self.resampling_operation_version
                != GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1
            || self.resampling_stream_version != GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1
            || self.quantile_method_version != GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1
            || self.standard_error_method_version
                != GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1
            || self.summation_method_version != GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1
            || self.p_value_method_version != GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1
            || self.failure_policy_version != GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1
            || self.sign_alignment_method_version
                != GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1
            || self.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
            || self.gamma_target_version
                != GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1
        {
            return Err(invalid(
                "schema or algorithm version is not the exact moderation-bootstrap v1 contract",
            ));
        }
        for (name, digest) in [
            ("general_sem_config_sha256", &self.general_sem_config_sha256),
            ("compiled_plan_sha256", &self.compiled_plan_sha256),
            ("model_scientific_sha256", &self.model_scientific_sha256),
            (
                "stage_one_model_scientific_sha256",
                &self.stage_one_model_scientific_sha256,
            ),
            (
                "complete_case_frame_sha256",
                &self.complete_case_frame_sha256,
            ),
            (
                "usable_replicate_indices_sha256",
                &self.usable_replicate_indices_sha256,
            ),
            (
                "gamma_target_identity_set_sha256",
                &self.gamma_target_identity_set_sha256,
            ),
        ] {
            if !is_lowercase_sha256_v1(digest) {
                return Err(invalid(&format!("{name} must be a lowercase SHA-256")));
            }
        }
        if self.stage_one_model_scientific_sha256 == self.model_scientific_sha256 {
            return Err(invalid(
                "moderation bootstrap requires a distinct interaction-free stage-one model",
            ));
        }
        if !is_dataset_fingerprint_v1(&self.source_dataset_fingerprint) {
            return Err(invalid(
                "source_dataset_fingerprint must be a bare SHA-256 or v2-prefixed SHA-256",
            ));
        }
        if self.gamma_target_ids.is_empty()
            || self
                .gamma_target_ids
                .iter()
                .any(|target_id| target_id.trim().is_empty())
            || self
                .gamma_target_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid(
                "gamma_target_ids must be nonempty, unique, and strictly ordered",
            ));
        }
        if self.interval != GeneralSemBootstrapIntervalV1::Percentile
            || self.tail != GeneralSemInferenceTailV1::TwoSided
            || !self.confidence_level.is_finite()
            || self.confidence_level <= 0.0
            || self.confidence_level >= 1.0
        {
            return Err(invalid(
                "interval, tail, or confidence level is outside the moderation-bootstrap v1 contract",
            ));
        }
        if !(2..=10_000).contains(&self.resamples_requested) {
            return Err(invalid("resamples_requested must be between 2 and 10000"));
        }
        let expected_minimum = minimum_usable_replicates(self.resamples_requested) as u32;
        if self.minimum_usable_resamples != expected_minimum
            || self.resamples_usable < expected_minimum
            || self.resamples_usable > self.resamples_requested
        {
            return Err(invalid(
                "usable replicate counts violate the exact 90 percent gate",
            ));
        }
        let seed = self
            .seed
            .parse::<u64>()
            .map_err(|_| invalid("seed must be a canonical decimal integer"))?;
        if seed.to_string() != self.seed || seed > qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1
        {
            return Err(invalid(
                "seed must be a JavaScript-safe canonical decimal integer",
            ));
        }
        if !(1..=64).contains(&self.workers)
            || !self.complete_model_reestimated_per_replicate
            || !self.shared_stage_one_reestimated_per_replicate
            || !self.score_vectors_sign_aligned_before_products
            || !self.product_scaling_recomputed_per_replicate
            || !self.joint_stage_two_reestimated_per_replicate
            || !self.complete_joint_point_contract_validated_per_replicate
        {
            return Err(invalid(
                "workers or full moderation-pipeline execution receipts are invalid",
            ));
        }
        if self.resamples_usable as usize + self.failed_replicates.len()
            != self.resamples_requested as usize
        {
            return Err(invalid(
                "usable and failed ledgers do not cover the requested plan",
            ));
        }
        let mut previous_failure = None;
        let mut failed_indices = BTreeSet::new();
        for failure in &self.failed_replicates {
            if failure.replicate_index >= self.resamples_requested
                || previous_failure.is_some_and(|previous| previous >= failure.replicate_index)
                || failure.message.trim().is_empty()
            {
                return Err(invalid("failed replicate ledger is not canonical"));
            }
            previous_failure = Some(failure.replicate_index);
            failed_indices.insert(failure.replicate_index);
        }
        let usable_indices = (0..self.resamples_requested)
            .filter(|index| !failed_indices.contains(index))
            .collect::<Vec<_>>();
        if usable_indices.len() != self.resamples_usable as usize
            || moderation_sha256_serialized(&usable_indices)?
                != self.usable_replicate_indices_sha256
        {
            return Err(invalid(
                "usable replicate index digest contradicts the failure ledger",
            ));
        }
        let actual_target_ids = self
            .interaction_gammas
            .iter()
            .map(|gamma| gamma.target.target_id.clone())
            .collect::<Vec<_>>();
        if actual_target_ids != self.gamma_target_ids {
            return Err(invalid(
                "gamma inference rows must exactly cover gamma_target_ids in canonical order",
            ));
        }
        let identities = self
            .interaction_gammas
            .iter()
            .map(|gamma| gamma.target.clone())
            .collect::<Vec<_>>();
        if moderation_sha256_serialized(&identities)? != self.gamma_target_identity_set_sha256 {
            return Err(invalid(
                "gamma target identity digest contradicts the typed target ledger",
            ));
        }
        let mut interaction_ids = BTreeSet::new();
        for gamma in &self.interaction_gammas {
            let target = &gamma.target;
            if target.kind
                != GeneralSemPlsModerationGammaTargetKindV1::InteractionScientificRescaledGamma
                || target.target_version
                    != GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1
                || target.target_id != target.interaction_effect_relation_id
                || target.stage_one_model_scientific_sha256
                    != self.stage_one_model_scientific_sha256
                || !is_lowercase_sha256_v1(&target.stage_one_model_scientific_sha256)
                || target.product_scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
                || target.method_version != GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
                || !interaction_ids.insert(target.interaction_id.as_str())
                || [
                    target.interaction_id.as_str(),
                    target.focal_relation_id.as_str(),
                    target.interaction_effect_relation_id.as_str(),
                    target.interaction_effect_parameter_id.as_str(),
                    target.generated_product_column_id.as_str(),
                    target.focal_predictor_id.as_str(),
                    target.moderator_id.as_str(),
                    target.outcome_id.as_str(),
                ]
                .iter()
                .any(|value| value.trim().is_empty())
                || [
                    gamma.original,
                    gamma.bootstrap_mean,
                    gamma.bootstrap_bias,
                    gamma.standard_error,
                    gamma.lower,
                    gamma.upper,
                    gamma.p_value_two_sided,
                ]
                .iter()
                .any(|value| !value.is_finite())
                || gamma.standard_error < 0.0
                || gamma.lower > gamma.upper
                || gamma.usable_replicates != self.resamples_usable
                || gamma.two_sided_exceedances > gamma.usable_replicates
                || !approximately_equal_v1(
                    gamma.bootstrap_bias,
                    gamma.bootstrap_mean - gamma.original,
                )
                || !approximately_equal_v1(
                    gamma.p_value_two_sided,
                    f64::from(gamma.two_sided_exceedances + 1)
                        / f64::from(gamma.usable_replicates + 1),
                )
            {
                return Err(invalid(&format!(
                    "gamma target {} violates the exact inference contract",
                    target.target_id
                )));
            }
        }
        Ok(())
    }

    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
        original_point: &GeneralSemPlsMultipleInteractionPointResultV1,
    ) -> Result<(), GeneralSemPlsMultipleModerationBootstrapErrorV1> {
        self.ensure_valid()?;
        original_point
            .ensure_valid_against_plan_v1(plan)
            .map_err(GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalInteractionPoint)?;
        let invalid = |message: &str| {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidResultContract(
                message.to_string(),
            )
        };
        if plan.two_way_interactions().is_empty()
            || self.general_sem_config_sha256 != plan.general_sem_config_sha256()
            || self.compiled_plan_sha256 != plan.deterministic_sha256()
            || self.model_scientific_sha256 != plan.scientific_hash()
            || plan.stage_one_projection_scientific_sha256()
                != Some(self.stage_one_model_scientific_sha256.as_str())
        {
            return Err(invalid(
                "moderation bootstrap provenance differs from the compiled PLS v3 plan",
            ));
        }
        let expected_identities = moderation_gamma_target_identities_v1(plan);
        let actual_identities = self
            .interaction_gammas
            .iter()
            .map(|gamma| gamma.target.clone())
            .collect::<Vec<_>>();
        if actual_identities != expected_identities {
            return Err(invalid(
                "moderation gamma identities differ from the compiled interaction inventory",
            ));
        }
        let original_by_target = original_point
            .interaction_coefficients()
            .iter()
            .map(|coefficient| {
                (
                    coefficient.interaction_effect_relation_id(),
                    coefficient.raw_product_estimate(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for gamma in &self.interaction_gammas {
            let original_matches = original_by_target
                .get(gamma.target.target_id.as_str())
                .is_some_and(|original| original.to_bits() == gamma.original.to_bits());
            if !original_matches {
                return Err(invalid(&format!(
                    "gamma target {} original estimate differs from the validated joint point result",
                    gamma.target.target_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemPlsMultipleModerationBootstrapErrorV1 {
    #[error(transparent)]
    InvalidConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error("General SEM PLS moderation bootstrap requires raw observations")]
    RawDataRequired,
    #[error("General SEM PLS moderation bootstrap requires a PLS-PM point execution")]
    InvalidPointExecutionMethod,
    #[error(
        "General SEM PLS moderation bootstrap requires a point-only projected execution recipe"
    )]
    PointExecutionContainsOuterResampling,
    #[error("General SEM PLS moderation bootstrap requires at least one compiled interaction")]
    InteractionPlanRequired,
    #[error("General SEM PLS moderation bootstrap requires case-bootstrap inference")]
    CaseBootstrapRequired,
    #[error("General SEM PLS moderation bootstrap v1 executes percentile intervals only")]
    UnsupportedInterval,
    #[error("General SEM PLS moderation bootstrap v1 executes two-sided inference only")]
    UnsupportedTail,
    #[error("General SEM PLS moderation bootstrap v1 does not execute authored conditional probes")]
    ConditionalProbesNotImplemented,
    #[error("General SEM PLS moderation bootstrap v1 does not implement lazy path materialization")]
    LazyPathMaterializationNotImplemented,
    #[error("compiled PLS v3 plan does not match the supplied General SEM configuration")]
    CompiledPlanConfigMismatch,
    #[error("point-execution recipe and compiled PLS v3 plan disagree: {0}")]
    PointExecutionPlanDomainMismatch(String),
    #[error("point-execution recipe and supplied PLS initialization disagree")]
    PointExecutionInitializationMismatch,
    #[error("the independent shared stage-one refit was cancelled")]
    PointRefitCancelled,
    #[error("the independent shared stage-one refit failed: {0}")]
    PointRefit(EstimationError),
    #[error("the independent shared stage-one refit did not converge")]
    PointRefitNotConverged,
    #[error("the supplied original PLS result differs from an independent same-input refit")]
    OriginalPointEstimateMismatch,
    #[error("original PLS result is not converged")]
    OriginalNotConverged,
    #[error(
        "original PLS result does not carry the expected estimator and score-execution identity"
    )]
    OriginalEstimatorIdentityMismatch,
    #[error("original PLS construct-score domain differs from the compiled plan")]
    OriginalConstructScoreDomainMismatch,
    #[error(
        "original PLS result used {original} observations but the fixed complete-case frame contains {frame}"
    )]
    OriginalObservationCountMismatch { original: usize, frame: usize },
    #[error("the supplied original joint moderation point result is invalid: {0}")]
    OriginalInteractionPoint(GeneralSemPlsInteractionPointErrorV1),
    #[error("the independent joint moderation point refit was cancelled")]
    InteractionPointRefitCancelled,
    #[error("the independent joint moderation point refit failed: {0}")]
    InteractionPointRefit(GeneralSemPlsInteractionPointErrorV1),
    #[error(
        "the supplied original joint moderation point result differs from an independent refit"
    )]
    OriginalInteractionPointMismatch,
    #[error(
        "General SEM PLS moderation bootstrap produced {usable} usable replicates; at least {required} are required"
    )]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error("General SEM PLS moderation bootstrap summary is invalid: {0}")]
    InvalidSummary(String),
    #[error("General SEM PLS moderation bootstrap result contract is invalid: {0}")]
    InvalidResultContract(String),
    #[error(
        "General SEM PLS moderation bootstrap replicate {replicate_index} violated the execution contract: {message}"
    )]
    ReplicateContract {
        replicate_index: u32,
        message: String,
    },
    #[error("General SEM PLS moderation bootstrap value could not be serialized: {0}")]
    Serialization(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemPlsBootstrapErrorV1 {
    #[error(transparent)]
    InvalidConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error("General SEM PLS bootstrap requires raw observations")]
    RawDataRequired,
    #[error("General SEM PLS bootstrap requires a PLS-PM point-execution capability")]
    InvalidPointExecutionMethod,
    #[error("General SEM PLS bootstrap requires a point-only projected execution recipe")]
    PointExecutionContainsOuterResampling,
    #[error(
        "General SEM PLS mediation bootstrap does not accept interaction plans; use the moderation bootstrap kernel"
    )]
    InteractionPlanNotSupported,
    #[error("General SEM PLS bootstrap requires case-bootstrap inference")]
    CaseBootstrapRequired,
    #[error("General SEM PLS bootstrap v1 executes percentile intervals only")]
    UnsupportedInterval,
    #[error("General SEM PLS bootstrap v1 executes two-sided inference only")]
    UnsupportedTail,
    #[error("General SEM PLS bootstrap v1 does not implement conditional-effect probes")]
    ConditionalProbesNotImplemented,
    #[error("General SEM PLS bootstrap v1 does not implement lazy path materialization")]
    LazyPathMaterializationNotImplemented,
    #[error("compiled PLS v3 plan does not match the supplied General SEM configuration")]
    CompiledPlanConfigMismatch,
    #[error("point-execution recipe and compiled PLS v3 plan disagree: {0}")]
    PointExecutionPlanDomainMismatch(String),
    #[error("point-execution recipe and supplied PLS initialization disagree")]
    PointExecutionInitializationMismatch,
    #[error("the independent point-estimate refit was cancelled")]
    PointRefitCancelled,
    #[error("the independent point-estimate refit failed: {0}")]
    PointRefit(EstimationError),
    #[error("the independent point-estimate refit did not converge")]
    PointRefitNotConverged,
    #[error("the supplied original PLS result differs from an independent same-input refit")]
    OriginalPointEstimateMismatch,
    #[error("original PLS result is not converged")]
    OriginalNotConverged,
    #[error(
        "original PLS result does not carry the expected estimator and score-execution identity"
    )]
    OriginalEstimatorIdentityMismatch,
    #[error("original PLS construct-score domain differs from the compiled plan")]
    OriginalConstructScoreDomainMismatch,
    #[error(
        "original PLS result used {original} observations but the fixed complete-case frame contains {frame}"
    )]
    OriginalObservationCountMismatch { original: usize, frame: usize },
    #[error("original PLS effect extraction failed: {0}")]
    OriginalEffectExtraction(String),
    #[error(
        "General SEM PLS bootstrap produced {usable} usable replicates; at least {required} are required"
    )]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error("General SEM PLS bootstrap summary is invalid: {0}")]
    InvalidSummary(String),
    #[error("General SEM PLS bootstrap result contract is invalid: {0}")]
    InvalidResultContract(String),
    #[error(
        "General SEM PLS bootstrap replicate {replicate_index} violated the execution contract: {message}"
    )]
    ReplicateContract {
        replicate_index: u32,
        message: String,
    },
    #[error("General SEM PLS bootstrap value could not be serialized: {0}")]
    Serialization(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Clone)]
struct SelectedEffectValueV1 {
    estimand_id: String,
    value: f64,
}

#[derive(Debug, Clone)]
enum GeneralSemPlsBootstrapReplicateRecordV1 {
    Usable {
        effect_values: BTreeMap<String, SelectedEffectValueV1>,
    },
    Failed {
        reason_code: GeneralSemPlsBootstrapFailureCodeV1,
        message: String,
    },
    Fatal {
        message: String,
    },
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
enum GeneralSemPlsBootstrapReplicateErrorV1 {
    #[error("resampled dataset construction failed: {0}")]
    Resample(EstimationError),
    #[error("replicate estimation failed: {0}")]
    Estimation(EstimationError),
    #[error("replicate sign alignment failed: {0}")]
    SignAlignment(EstimationError),
    #[error(
        "PLS result does not contain exactly one coefficient for relation {relation_id} ({source_id} -> {target_id})"
    )]
    RelationEstimateCardinality {
        relation_id: String,
        source_id: String,
        target_id: String,
    },
    #[error(transparent)]
    EffectDecomposition(#[from] GeneralSemEffectsV1Error),
    #[error("compiled effect estimand {0} is absent from the complete decomposition")]
    MissingCompiledEffect(String),
    #[error("compiled effect id {0} is duplicated")]
    DuplicateEffectId(String),
}

impl GeneralSemPlsBootstrapReplicateErrorV1 {
    fn into_record(self) -> GeneralSemPlsBootstrapReplicateRecordV1 {
        match self {
            Self::Resample(EstimationError::Cancelled)
            | Self::SignAlignment(EstimationError::Cancelled)
            | Self::Estimation(EstimationError::Cancelled) => {
                GeneralSemPlsBootstrapReplicateRecordV1::Cancelled
            }
            Self::Estimation(error @ EstimationError::InsufficientObservations) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::InsufficientObservations,
                error.to_string(),
            ),
            Self::Estimation(error @ EstimationError::ConstantIndicator(_)) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::ConstantIndicator,
                error.to_string(),
            ),
            Self::Estimation(error @ EstimationError::RankDeficient(_)) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::RankDeficient,
                error.to_string(),
            ),
            Self::Estimation(error @ EstimationError::IsolatedConstruct(_)) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::IsolatedConstruct,
                error.to_string(),
            ),
            Self::Estimation(error @ EstimationError::NonConvergence(_)) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::EstimationNonconvergence,
                error.to_string(),
            ),
            Self::Estimation(error @ EstimationError::Numerical(_)) => failed_record(
                GeneralSemPlsBootstrapFailureCodeV1::NumericalFailure,
                error.to_string(),
            ),
            error => GeneralSemPlsBootstrapReplicateRecordV1::Fatal {
                message: error.to_string(),
            },
        }
    }
}

fn failed_record(
    reason_code: GeneralSemPlsBootstrapFailureCodeV1,
    message: String,
) -> GeneralSemPlsBootstrapReplicateRecordV1 {
    GeneralSemPlsBootstrapReplicateRecordV1::Failed {
        reason_code,
        message,
    }
}

/// Full-model, indexed case bootstrap for the first General SEM PLS inference
/// slice. The caller supplies the already-projected point-only execution
/// capability and the exact compiled PLS v3 plan. Every usable replicate
/// re-estimates the complete score and structural model before effects are
/// decomposed through stable relation identities.
pub fn bootstrap_general_sem_pls_v1(
    dataset: &Dataset,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
    original: &PlsResult,
    config: &GeneralSemConfigV1,
    initialization: Option<&PlsAlgorithmConfigV2>,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<GeneralSemPlsBootstrapResultV1, GeneralSemPlsBootstrapErrorV1> {
    if is_cancelled() {
        return Err(GeneralSemPlsBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }
    config.ensure_valid()?;
    if !plan.two_way_interactions().is_empty() {
        return Err(GeneralSemPlsBootstrapErrorV1::InteractionPlanNotSupported);
    }
    if dataset.schema.kind != DataKind::Raw {
        return Err(GeneralSemPlsBootstrapErrorV1::RawDataRequired);
    }
    if point_execution.source().settings.method != AnalysisMethod::PlsPm {
        return Err(GeneralSemPlsBootstrapErrorV1::InvalidPointExecutionMethod);
    }
    if point_execution.source().settings.bootstrap_samples != 0
        || point_execution.source().settings.studentized_inner_samples != 0
        || point_execution.source().settings.permutation_samples != 0
    {
        return Err(GeneralSemPlsBootstrapErrorV1::PointExecutionContainsOuterResampling);
    }
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(GeneralSemPlsBootstrapErrorV1::LazyPathMaterializationNotImplemented);
    }
    if !config.conditional_effect_probes.is_empty() {
        return Err(GeneralSemPlsBootstrapErrorV1::ConditionalProbesNotImplemented);
    }
    let GeneralSemInferenceV1::CaseBootstrap {
        resamples,
        seed,
        confidence_level,
        interval,
        tail,
    } = config.inference
    else {
        return Err(GeneralSemPlsBootstrapErrorV1::CaseBootstrapRequired);
    };
    if interval != GeneralSemBootstrapIntervalV1::Percentile {
        return Err(GeneralSemPlsBootstrapErrorV1::UnsupportedInterval);
    }
    if tail != GeneralSemInferenceTailV1::TwoSided {
        return Err(GeneralSemPlsBootstrapErrorV1::UnsupportedTail);
    }
    let config_sha256 = sha256_serialized(config)?;
    if plan.general_sem_config_sha256() != config_sha256 {
        return Err(GeneralSemPlsBootstrapErrorV1::CompiledPlanConfigMismatch);
    }
    validate_point_execution_plan_domain_v1(dataset, point_execution, plan)?;
    let recipe_initialization = match point_execution.source().method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
        _ => None,
    };
    if recipe_initialization != initialization {
        return Err(GeneralSemPlsBootstrapErrorV1::PointExecutionInitializationMismatch);
    }
    if !original.converged {
        return Err(GeneralSemPlsBootstrapErrorV1::OriginalNotConverged);
    }

    let compiled_scoring = initialization.is_some()
        || plan
            .base_plan()
            .blocks()
            .iter()
            .any(|block| block.fixed_scoring().is_some());
    let original_estimator_identity_valid = if compiled_scoring {
        original.method_version == PLS_SCORE_EXECUTION_METHOD_VERSION_V2
            && original.score_execution.as_ref().is_some_and(|receipt| {
                receipt.contract_version == PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2
            })
    } else {
        original.method_version == PLS_METHOD_VERSION && original.score_execution.is_none()
    };
    if !original_estimator_identity_valid {
        return Err(GeneralSemPlsBootstrapErrorV1::OriginalEstimatorIdentityMismatch);
    }

    let independently_refitted = if compiled_scoring {
        estimate_pls_validated_with_compiled_plan_v2_with_control(
            dataset,
            point_execution,
            plan.base_plan(),
            initialization,
            |_| !is_cancelled(),
        )
    } else {
        estimate_pls_validated_with_control(dataset, point_execution, |_| !is_cancelled())
    }
    .map_err(|error| match error {
        EstimationError::Cancelled => GeneralSemPlsBootstrapErrorV1::PointRefitCancelled,
        error => GeneralSemPlsBootstrapErrorV1::PointRefit(error),
    })?;
    if !independently_refitted.converged {
        return Err(GeneralSemPlsBootstrapErrorV1::PointRefitNotConverged);
    }
    if independently_refitted != *original {
        return Err(GeneralSemPlsBootstrapErrorV1::OriginalPointEstimateMismatch);
    }
    if is_cancelled() {
        return Err(GeneralSemPlsBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }

    let base_recipe = point_execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| GeneralSemPlsBootstrapErrorV1::InvalidSummary(error.to_string()))?;
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if original.used_observations != complete_rows.len() {
        return Err(
            GeneralSemPlsBootstrapErrorV1::OriginalObservationCountMismatch {
                original: original.used_observations,
                frame: complete_rows.len(),
            },
        );
    }
    let expected_construct_ids = plan
        .base_plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id())
        .collect::<BTreeSet<_>>();
    let actual_construct_ids = original
        .construct_scores
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_construct_ids != expected_construct_ids
        || original
            .construct_scores
            .values()
            .any(|scores| scores.len() != complete_rows.len())
    {
        return Err(GeneralSemPlsBootstrapErrorV1::OriginalConstructScoreDomainMismatch);
    }
    let original_effects = selected_effect_values(plan, original).map_err(|error| {
        GeneralSemPlsBootstrapErrorV1::OriginalEffectExtraction(error.to_string())
    })?;
    let effect_ids = original_effects.keys().cloned().collect::<Vec<_>>();
    if effect_ids.is_empty() {
        return Err(GeneralSemPlsBootstrapErrorV1::InvalidSummary(
            "compiled plan contains no effect estimands".to_string(),
        ));
    }

    let bootstrap_plan = BootstrapPlan {
        replicates: resamples,
        master_seed: seed,
        operation: GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1.to_string(),
    };
    let cancellation = &is_cancelled;
    let run = run_bootstrap(
        complete_rows.len(),
        &bootstrap_plan,
        workers,
        |_replicate_index, sampled_positions| {
            let result = (|| {
                let raw_indices = sampled_positions
                    .iter()
                    .map(|position| complete_rows[*position])
                    .collect::<Vec<_>>();
                let sampled =
                    resample_model_dataset(dataset, base_recipe, &raw_indices, cancellation)
                        .map_err(GeneralSemPlsBootstrapReplicateErrorV1::Resample)?;
                let mut estimate = if compiled_scoring {
                    estimate_pls_validated_with_compiled_plan_v2_with_control(
                        &sampled,
                        point_execution,
                        plan.base_plan(),
                        initialization,
                        |_| !cancellation(),
                    )
                } else {
                    estimate_pls_validated_with_control(&sampled, point_execution, |_| {
                        !cancellation()
                    })
                }
                .map_err(GeneralSemPlsBootstrapReplicateErrorV1::Estimation)?;
                align_general_sem_pls_signs_v1(
                    &mut estimate,
                    &original.construct_scores,
                    sampled_positions,
                    cancellation,
                )
                .map_err(GeneralSemPlsBootstrapReplicateErrorV1::SignAlignment)?;
                let effect_values = selected_effect_values(plan, &estimate)?;
                Ok::<_, GeneralSemPlsBootstrapReplicateErrorV1>(effect_values)
            })();
            Ok::<_, Infallible>(match result {
                Ok(effect_values) => {
                    GeneralSemPlsBootstrapReplicateRecordV1::Usable { effect_values }
                }
                Err(error) => error.into_record(),
            })
        },
        cancellation,
        report_progress,
    )?;

    let mut usable_indices = Vec::new();
    let mut replicate_effects = Vec::new();
    let mut failures = Vec::new();
    for (replicate_index, outcome) in run.outcomes.iter().enumerate() {
        let ReplicateOutcome::Success { value } = outcome else {
            return Err(GeneralSemPlsBootstrapErrorV1::InvalidSummary(
                "the infallible indexed scheduler returned a failed outer outcome".to_string(),
            ));
        };
        match value {
            GeneralSemPlsBootstrapReplicateRecordV1::Usable { effect_values } => {
                usable_indices.push(replicate_index as u32);
                replicate_effects.push(effect_values);
            }
            GeneralSemPlsBootstrapReplicateRecordV1::Failed {
                reason_code,
                message,
            } => failures.push(GeneralSemPlsBootstrapFailedReplicateV1 {
                replicate_index: replicate_index as u32,
                reason_code: reason_code.clone(),
                message: message.clone(),
            }),
            GeneralSemPlsBootstrapReplicateRecordV1::Fatal { message } => {
                return Err(GeneralSemPlsBootstrapErrorV1::ReplicateContract {
                    replicate_index: replicate_index as u32,
                    message: message.clone(),
                });
            }
            GeneralSemPlsBootstrapReplicateRecordV1::Cancelled => {
                return Err(GeneralSemPlsBootstrapErrorV1::Resampling(
                    ResamplingError::Cancelled,
                ));
            }
        }
    }
    let minimum_usable = minimum_usable_replicates(resamples);
    if replicate_effects.len() < minimum_usable {
        return Err(
            GeneralSemPlsBootstrapErrorV1::InsufficientUsableReplicates {
                usable: replicate_effects.len(),
                required: minimum_usable,
            },
        );
    }
    if replicate_effects.len() + failures.len() != resamples as usize {
        return Err(GeneralSemPlsBootstrapErrorV1::InvalidSummary(
            "usable and failed replicate ledgers do not cover the requested plan".to_string(),
        ));
    }

    let effects = summarize_effects(
        &effect_ids,
        &original_effects,
        &replicate_effects,
        confidence_level,
    )?;
    let source_columns = plan
        .base_plan()
        .blocks()
        .iter()
        .flat_map(|block| block.indicators())
        .map(|indicator| indicator.source_column().to_string())
        .collect::<Vec<_>>();
    let frame_rows = complete_rows
        .iter()
        .map(|row| *row as u64)
        .collect::<Vec<_>>();
    let complete_case_frame_sha256 = sha256_serialized(&CompleteCaseFrameIdentityV1 {
        dataset_id: dataset.id.to_string(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        source_columns,
        raw_row_indices: frame_rows,
    })?;
    if is_cancelled() {
        return Err(GeneralSemPlsBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }

    let result = GeneralSemPlsBootstrapResultV1 {
        schema_version: GENERAL_SEM_PLS_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1,
        method_version: GENERAL_SEM_PLS_BOOTSTRAP_METHOD_VERSION_V1.to_string(),
        resampling_operation_version: GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1.to_string(),
        resampling_stream_version: GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1.to_string(),
        quantile_method_version: GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1.to_string(),
        standard_error_method_version: GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1
            .to_string(),
        summation_method_version: GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1.to_string(),
        p_value_method_version: GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1.to_string(),
        failure_policy_version: GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1.to_string(),
        general_sem_config_sha256: config_sha256,
        compiled_plan_sha256: plan.deterministic_sha256(),
        model_scientific_sha256: plan.topology().model_scientific_sha256().to_string(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        complete_case_frame_sha256,
        usable_replicate_indices_sha256: sha256_serialized(&usable_indices)?,
        effect_identity_set_sha256: general_sem_effect_identity_set_sha256_v1(
            &compiled_pls_effect_identities_v1(plan.effect_estimands()),
        ),
        effect_ids,
        interval,
        tail,
        confidence_level,
        resamples_requested: resamples,
        resamples_usable: replicate_effects.len() as u32,
        minimum_usable_resamples: minimum_usable as u32,
        seed: seed.to_string(),
        workers: workers as u32,
        complete_model_reestimated_per_replicate: true,
        failed_replicates: failures,
        effects,
    };
    result.ensure_valid()?;
    Ok(result)
}

#[derive(Debug, Clone)]
enum GeneralSemPlsModerationBootstrapReplicateRecordV1 {
    Usable {
        gamma_values: BTreeMap<String, f64>,
    },
    Failed {
        reason_code: GeneralSemPlsModerationBootstrapFailureCodeV1,
        message: String,
    },
    Fatal {
        message: String,
    },
    Cancelled,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
enum GeneralSemPlsModerationScoreAlignmentErrorV1 {
    #[error("score sign alignment was cancelled")]
    Cancelled,
    #[error("replicate and original construct-score domains differ")]
    ConstructDomainMismatch,
    #[error("original construct score is missing for {construct_id}")]
    MissingOriginalScore { construct_id: String },
    #[error(
        "sampled position {sampled_position} is outside original construct score {construct_id}"
    )]
    SampledPositionOutOfBounds {
        construct_id: String,
        sampled_position: usize,
    },
    #[error(
        "replicate construct score {construct_id} has {actual} observations; expected {expected}"
    )]
    ScoreLengthMismatch {
        construct_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("construct score {construct_id} contains a non-finite alignment value")]
    NonFiniteScore { construct_id: String },
    #[error("construct score sign is indeterminate for {construct_id}")]
    IndeterminateSign { construct_id: String },
}

#[derive(Debug, thiserror::Error)]
enum GeneralSemPlsModerationBootstrapReplicateErrorV1 {
    #[error("resampled dataset construction failed: {0}")]
    Resample(EstimationError),
    #[error("shared stage-one estimation failed: {0}")]
    StageOne(EstimationError),
    #[error("construct-score sign alignment failed: {0}")]
    SignAlignment(GeneralSemPlsModerationScoreAlignmentErrorV1),
    #[error("joint stage-two moderation estimation failed: {0}")]
    JointStage(GeneralSemPlsInteractionPointErrorV1),
    #[error("joint stage-two gamma inventory differs from the compiled interaction plan")]
    GammaInventoryMismatch,
}

impl GeneralSemPlsModerationBootstrapReplicateErrorV1 {
    fn into_record(self) -> GeneralSemPlsModerationBootstrapReplicateRecordV1 {
        use GeneralSemPlsModerationBootstrapFailureCodeV1 as Failure;
        match self {
            Self::Resample(EstimationError::Cancelled)
            | Self::StageOne(EstimationError::Cancelled)
            | Self::SignAlignment(GeneralSemPlsModerationScoreAlignmentErrorV1::Cancelled)
            | Self::JointStage(GeneralSemPlsInteractionPointErrorV1::Cancelled) => {
                GeneralSemPlsModerationBootstrapReplicateRecordV1::Cancelled
            }
            Self::StageOne(error @ EstimationError::InsufficientObservations) => {
                failed_moderation_record(Failure::InsufficientObservations, error.to_string())
            }
            Self::StageOne(error @ EstimationError::ConstantIndicator(_)) => {
                failed_moderation_record(Failure::ConstantIndicator, error.to_string())
            }
            Self::StageOne(error @ EstimationError::RankDeficient(_)) => {
                failed_moderation_record(Failure::StageOneRankDeficient, error.to_string())
            }
            Self::StageOne(error @ EstimationError::IsolatedConstruct(_)) => {
                failed_moderation_record(Failure::IsolatedConstruct, error.to_string())
            }
            Self::StageOne(error @ EstimationError::NonConvergence(_)) => {
                failed_moderation_record(Failure::StageOneNonconvergence, error.to_string())
            }
            Self::StageOne(error @ EstimationError::Numerical(_)) => {
                failed_moderation_record(Failure::NumericalFailure, error.to_string())
            }
            Self::SignAlignment(
                error @ GeneralSemPlsModerationScoreAlignmentErrorV1::IndeterminateSign { .. },
            ) => failed_moderation_record(Failure::IndeterminateScoreSign, error.to_string()),
            Self::SignAlignment(
                error @ GeneralSemPlsModerationScoreAlignmentErrorV1::NonFiniteScore { .. },
            ) => failed_moderation_record(Failure::NumericalFailure, error.to_string()),
            Self::JointStage(
                error @ GeneralSemPlsInteractionPointErrorV1::InsufficientObservations { .. },
            ) => failed_moderation_record(Failure::InsufficientObservations, error.to_string()),
            Self::JointStage(
                error @ GeneralSemPlsInteractionPointErrorV1::ConstantStageOneScore { .. },
            ) => failed_moderation_record(Failure::ConstantConstructScore, error.to_string()),
            Self::JointStage(
                error @ GeneralSemPlsInteractionPointErrorV1::ConstantProduct { .. },
            ) => failed_moderation_record(Failure::ConstantInteractionProduct, error.to_string()),
            Self::JointStage(
                error @ GeneralSemPlsInteractionPointErrorV1::RankDeficient { .. },
            ) => failed_moderation_record(Failure::JointStageRankDeficient, error.to_string()),
            Self::JointStage(
                error @ GeneralSemPlsInteractionPointErrorV1::NonFiniteScore { .. },
            ) => failed_moderation_record(Failure::NumericalFailure, error.to_string()),
            error => GeneralSemPlsModerationBootstrapReplicateRecordV1::Fatal {
                message: error.to_string(),
            },
        }
    }
}

fn failed_moderation_record(
    reason_code: GeneralSemPlsModerationBootstrapFailureCodeV1,
    message: String,
) -> GeneralSemPlsModerationBootstrapReplicateRecordV1 {
    GeneralSemPlsModerationBootstrapReplicateRecordV1::Failed {
        reason_code,
        message,
    }
}

/// Full-model indexed case bootstrap for simultaneous two-way PLS moderation.
/// Only scientific rescaled gamma is an inferential target in this parity-first
/// version. Nevertheless, every usable replicate reruns shared stage one,
/// aligns the complete score vectors before product construction, recomputes
/// all product scales, solves every joint stage-two equation, rebuilds slopes,
/// and validates the complete point contract before gamma is extracted.
pub fn bootstrap_general_sem_pls_multiple_two_way_moderation_v1(
    dataset: &Dataset,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
    original_stage_one: &PlsResult,
    original_point: &GeneralSemPlsMultipleInteractionPointResultV1,
    config: &GeneralSemConfigV1,
    initialization: Option<&PlsAlgorithmConfigV2>,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<
    GeneralSemPlsMultipleModerationBootstrapResultV1,
    GeneralSemPlsMultipleModerationBootstrapErrorV1,
> {
    if is_cancelled() {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }
    config.ensure_valid()?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::RawDataRequired);
    }
    if point_execution.source().settings.method != AnalysisMethod::PlsPm {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidPointExecutionMethod);
    }
    if point_execution.source().settings.bootstrap_samples != 0
        || point_execution.source().settings.studentized_inner_samples != 0
        || point_execution.source().settings.permutation_samples != 0
    {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::PointExecutionContainsOuterResampling,
        );
    }
    if plan.two_way_interactions().is_empty() {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::InteractionPlanRequired);
    }
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::LazyPathMaterializationNotImplemented,
        );
    }
    if !config.conditional_effect_probes.is_empty() {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::ConditionalProbesNotImplemented,
        );
    }
    let GeneralSemInferenceV1::CaseBootstrap {
        resamples,
        seed,
        confidence_level,
        interval,
        tail,
    } = config.inference
    else {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::CaseBootstrapRequired);
    };
    if interval != GeneralSemBootstrapIntervalV1::Percentile {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::UnsupportedInterval);
    }
    if tail != GeneralSemInferenceTailV1::TwoSided {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::UnsupportedTail);
    }
    let config_sha256 = moderation_sha256_serialized(config)?;
    if plan.general_sem_config_sha256() != config_sha256 {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::CompiledPlanConfigMismatch);
    }
    validate_point_execution_plan_domain_v1(dataset, point_execution, plan).map_err(|error| {
        let message = match error {
            GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(message) => message,
            other => other.to_string(),
        };
        GeneralSemPlsMultipleModerationBootstrapErrorV1::PointExecutionPlanDomainMismatch(message)
    })?;
    let recipe_initialization = match point_execution.source().method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
        _ => None,
    };
    if recipe_initialization != initialization {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::PointExecutionInitializationMismatch,
        );
    }
    if !original_stage_one.converged {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalNotConverged);
    }

    let compiled_scoring = initialization.is_some()
        || plan
            .base_plan()
            .blocks()
            .iter()
            .any(|block| block.fixed_scoring().is_some());
    let original_estimator_identity_valid = if compiled_scoring {
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
    if !original_estimator_identity_valid {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalEstimatorIdentityMismatch,
        );
    }

    let independently_refitted = if compiled_scoring {
        estimate_pls_validated_with_compiled_plan_v2_with_control(
            dataset,
            point_execution,
            plan.base_plan(),
            initialization,
            |_| !is_cancelled(),
        )
    } else {
        estimate_pls_validated_with_control(dataset, point_execution, |_| !is_cancelled())
    }
    .map_err(|error| match error {
        EstimationError::Cancelled => {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::PointRefitCancelled
        }
        error => GeneralSemPlsMultipleModerationBootstrapErrorV1::PointRefit(error),
    })?;
    if !independently_refitted.converged {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::PointRefitNotConverged);
    }
    if independently_refitted != *original_stage_one {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalPointEstimateMismatch);
    }
    original_point
        .ensure_valid_against_plan_v1(plan)
        .map_err(GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalInteractionPoint)?;
    let independently_refitted_point =
        estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
            plan,
            &independently_refitted.construct_scores,
            || !is_cancelled(),
        )
        .map_err(|error| match error {
            GeneralSemPlsInteractionPointErrorV1::Cancelled => {
                GeneralSemPlsMultipleModerationBootstrapErrorV1::InteractionPointRefitCancelled
            }
            error => GeneralSemPlsMultipleModerationBootstrapErrorV1::InteractionPointRefit(error),
        })?;
    if independently_refitted_point != *original_point {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalInteractionPointMismatch,
        );
    }
    if is_cancelled() {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }

    let base_recipe = point_execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(error.to_string())
        })?;
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if original_stage_one.used_observations != complete_rows.len() {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalObservationCountMismatch {
                original: original_stage_one.used_observations,
                frame: complete_rows.len(),
            },
        );
    }
    let expected_construct_ids = plan
        .base_plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id())
        .collect::<BTreeSet<_>>();
    let actual_construct_ids = original_stage_one
        .construct_scores
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_construct_ids != expected_construct_ids
        || original_stage_one
            .construct_scores
            .values()
            .any(|scores| scores.len() != complete_rows.len())
        || original_point.observation_count() != complete_rows.len()
    {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalConstructScoreDomainMismatch,
        );
    }
    let gamma_identities = moderation_gamma_target_identities_v1(plan);
    let gamma_target_ids = gamma_identities
        .iter()
        .map(|identity| identity.target_id.clone())
        .collect::<Vec<_>>();
    let original_gamma_values =
        moderation_gamma_values_v1(plan, original_point).map_err(|error| {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(error.to_string())
        })?;

    let bootstrap_plan = BootstrapPlan {
        replicates: resamples,
        master_seed: seed,
        operation: GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1.to_string(),
    };
    let cancellation = &is_cancelled;
    let run = run_bootstrap(
        complete_rows.len(),
        &bootstrap_plan,
        workers,
        |_replicate_index, sampled_positions| {
            let result = estimate_moderation_bootstrap_replicate_v1(
                dataset,
                base_recipe,
                point_execution,
                plan,
                &original_stage_one.construct_scores,
                &complete_rows,
                sampled_positions,
                compiled_scoring,
                initialization,
                cancellation,
            )
            .and_then(|point| moderation_gamma_values_v1(plan, &point));
            Ok::<_, Infallible>(match result {
                Ok(gamma_values) => {
                    GeneralSemPlsModerationBootstrapReplicateRecordV1::Usable { gamma_values }
                }
                Err(error) => error.into_record(),
            })
        },
        cancellation,
        report_progress,
    )?;

    let mut usable_indices = Vec::new();
    let mut replicate_gamma_values = Vec::new();
    let mut failures = Vec::new();
    for (replicate_index, outcome) in run.outcomes.iter().enumerate() {
        let ReplicateOutcome::Success { value } = outcome else {
            return Err(
                GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(
                    "the infallible indexed scheduler returned a failed outer outcome".into(),
                ),
            );
        };
        match value {
            GeneralSemPlsModerationBootstrapReplicateRecordV1::Usable { gamma_values } => {
                usable_indices.push(replicate_index as u32);
                replicate_gamma_values.push(gamma_values);
            }
            GeneralSemPlsModerationBootstrapReplicateRecordV1::Failed {
                reason_code,
                message,
            } => failures.push(GeneralSemPlsModerationBootstrapFailedReplicateV1 {
                replicate_index: replicate_index as u32,
                reason_code: reason_code.clone(),
                message: message.clone(),
            }),
            GeneralSemPlsModerationBootstrapReplicateRecordV1::Fatal { message } => {
                return Err(
                    GeneralSemPlsMultipleModerationBootstrapErrorV1::ReplicateContract {
                        replicate_index: replicate_index as u32,
                        message: message.clone(),
                    },
                );
            }
            GeneralSemPlsModerationBootstrapReplicateRecordV1::Cancelled => {
                return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::Resampling(
                    ResamplingError::Cancelled,
                ));
            }
        }
    }
    let minimum_usable = minimum_usable_replicates(resamples);
    if replicate_gamma_values.len() < minimum_usable {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InsufficientUsableReplicates {
                usable: replicate_gamma_values.len(),
                required: minimum_usable,
            },
        );
    }
    if replicate_gamma_values.len() + failures.len() != resamples as usize {
        return Err(
            GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(
                "usable and failed replicate ledgers do not cover the requested plan".into(),
            ),
        );
    }
    let interaction_gammas = summarize_moderation_gammas_v1(
        &gamma_identities,
        &original_gamma_values,
        &replicate_gamma_values,
        confidence_level,
    )?;
    let source_columns = plan
        .base_plan()
        .blocks()
        .iter()
        .flat_map(|block| block.indicators())
        .map(|indicator| indicator.source_column().to_string())
        .collect::<Vec<_>>();
    let frame_rows = complete_rows
        .iter()
        .map(|row| *row as u64)
        .collect::<Vec<_>>();
    let complete_case_frame_sha256 = moderation_sha256_serialized(&CompleteCaseFrameIdentityV1 {
        dataset_id: dataset.id.to_string(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        source_columns,
        raw_row_indices: frame_rows,
    })?;
    if is_cancelled() {
        return Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::Resampling(
            ResamplingError::Cancelled,
        ));
    }
    let result = GeneralSemPlsMultipleModerationBootstrapResultV1 {
        schema_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1,
        method_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_METHOD_VERSION_V1.into(),
        point_method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into(),
        resampling_operation_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1
            .into(),
        resampling_stream_version: GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1.into(),
        quantile_method_version: GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1.into(),
        standard_error_method_version: GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1.into(),
        summation_method_version: GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1.into(),
        p_value_method_version: GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1.into(),
        failure_policy_version: GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1.into(),
        sign_alignment_method_version:
            GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1.into(),
        product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
        gamma_target_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1.into(),
        general_sem_config_sha256: config_sha256,
        compiled_plan_sha256: plan.deterministic_sha256(),
        model_scientific_sha256: plan.scientific_hash().into(),
        stage_one_model_scientific_sha256: plan
            .stage_one_projection_scientific_sha256()
            .expect("an interaction plan has a stage-one projection")
            .into(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        complete_case_frame_sha256,
        usable_replicate_indices_sha256: moderation_sha256_serialized(&usable_indices)?,
        gamma_target_identity_set_sha256: moderation_sha256_serialized(&gamma_identities)?,
        gamma_target_ids,
        interval,
        tail,
        confidence_level,
        resamples_requested: resamples,
        resamples_usable: replicate_gamma_values.len() as u32,
        minimum_usable_resamples: minimum_usable as u32,
        seed: seed.to_string(),
        workers: workers as u32,
        complete_model_reestimated_per_replicate: true,
        shared_stage_one_reestimated_per_replicate: true,
        score_vectors_sign_aligned_before_products: true,
        product_scaling_recomputed_per_replicate: true,
        joint_stage_two_reestimated_per_replicate: true,
        complete_joint_point_contract_validated_per_replicate: true,
        failed_replicates: failures,
        interaction_gammas,
    };
    result.ensure_valid_against_plan_v1(plan, original_point)?;
    Ok(result)
}

fn estimate_moderation_bootstrap_replicate_v1(
    dataset: &Dataset,
    base_recipe: &AnalysisRecipe,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
    original_scores: &BTreeMap<String, Vec<f64>>,
    complete_rows: &[usize],
    sampled_positions: &[usize],
    compiled_scoring: bool,
    initialization: Option<&PlsAlgorithmConfigV2>,
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<
    GeneralSemPlsMultipleInteractionPointResultV1,
    GeneralSemPlsModerationBootstrapReplicateErrorV1,
> {
    let raw_indices = sampled_positions
        .iter()
        .map(|position| complete_rows[*position])
        .collect::<Vec<_>>();
    let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, is_cancelled)
        .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::Resample)?;
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
    .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::StageOne)?;
    align_general_sem_pls_moderation_score_vectors_v1(
        &mut stage_one.construct_scores,
        original_scores,
        sampled_positions,
        is_cancelled,
    )
    .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::SignAlignment)?;
    let point = estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
        plan,
        &stage_one.construct_scores,
        || !is_cancelled(),
    )
    .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::JointStage)?;
    point
        .ensure_valid_against_plan_v1(plan)
        .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::JointStage)?;
    Ok(point)
}

fn align_general_sem_pls_moderation_score_vectors_v1(
    replicate_scores: &mut BTreeMap<String, Vec<f64>>,
    original_scores: &BTreeMap<String, Vec<f64>>,
    sampled_positions: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<(), GeneralSemPlsModerationScoreAlignmentErrorV1> {
    if replicate_scores.keys().collect::<Vec<_>>() != original_scores.keys().collect::<Vec<_>>() {
        return Err(GeneralSemPlsModerationScoreAlignmentErrorV1::ConstructDomainMismatch);
    }
    for (construct_id, replicate) in replicate_scores {
        if is_cancelled() {
            return Err(GeneralSemPlsModerationScoreAlignmentErrorV1::Cancelled);
        }
        let original = original_scores.get(construct_id).ok_or_else(|| {
            GeneralSemPlsModerationScoreAlignmentErrorV1::MissingOriginalScore {
                construct_id: construct_id.clone(),
            }
        })?;
        let aligned_reference = sampled_positions
            .iter()
            .map(|position| {
                original.get(*position).copied().ok_or_else(|| {
                    GeneralSemPlsModerationScoreAlignmentErrorV1::SampledPositionOutOfBounds {
                        construct_id: construct_id.clone(),
                        sampled_position: *position,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if replicate.len() != aligned_reference.len() {
            return Err(
                GeneralSemPlsModerationScoreAlignmentErrorV1::ScoreLengthMismatch {
                    construct_id: construct_id.clone(),
                    expected: aligned_reference.len(),
                    actual: replicate.len(),
                },
            );
        }
        if replicate
            .iter()
            .chain(&aligned_reference)
            .any(|value| !value.is_finite())
        {
            return Err(
                GeneralSemPlsModerationScoreAlignmentErrorV1::NonFiniteScore {
                    construct_id: construct_id.clone(),
                },
            );
        }
        let (covariance, absolute_cross_product_sum) =
            score_alignment_covariance_v1(&aligned_reference, replicate);
        let tolerance =
            64.0 * f64::EPSILON * absolute_cross_product_sum.max(covariance.abs()).max(1.0);
        let sign = if covariance > tolerance {
            1.0
        } else if covariance < -tolerance {
            -1.0
        } else {
            return Err(
                GeneralSemPlsModerationScoreAlignmentErrorV1::IndeterminateSign {
                    construct_id: construct_id.clone(),
                },
            );
        };
        if sign < 0.0 {
            for value in replicate {
                if is_cancelled() {
                    return Err(GeneralSemPlsModerationScoreAlignmentErrorV1::Cancelled);
                }
                *value = -*value;
            }
        }
    }
    Ok(())
}

fn score_alignment_covariance_v1(left: &[f64], right: &[f64]) -> (f64, f64) {
    let left_mean = stable_sum(left) / left.len() as f64;
    let right_mean = stable_sum(right) / right.len() as f64;
    let cross_products = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .collect::<Vec<_>>();
    (
        stable_sum(&cross_products),
        stable_sum(
            &cross_products
                .iter()
                .map(|value| value.abs())
                .collect::<Vec<_>>(),
        ),
    )
}

fn moderation_gamma_target_identities_v1(
    plan: &CompiledPlsPlanV3,
) -> Vec<GeneralSemPlsModerationGammaTargetIdentityV1> {
    let stage_one_model_scientific_sha256 = plan
        .stage_one_projection_scientific_sha256()
        .expect("an interaction plan has a stage-one projection")
        .to_string();
    let mut identities = plan
        .two_way_interactions()
        .iter()
        .map(|interaction| GeneralSemPlsModerationGammaTargetIdentityV1 {
            kind: GeneralSemPlsModerationGammaTargetKindV1::InteractionScientificRescaledGamma,
            target_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1.to_string(),
            target_id: interaction.interaction_effect_relation_id().to_string(),
            interaction_id: interaction.interaction_id().to_string(),
            focal_relation_id: interaction.focal_relation_id().to_string(),
            interaction_effect_relation_id: interaction
                .interaction_effect_relation_id()
                .to_string(),
            interaction_effect_parameter_id: interaction
                .interaction_effect_parameter_id()
                .to_string(),
            generated_product_column_id: interaction.generated_product_column_id().to_string(),
            focal_predictor_id: interaction.focal_predictor_id().to_string(),
            moderator_id: interaction.moderator_id().to_string(),
            outcome_id: interaction.outcome_id().to_string(),
            stage_one_model_scientific_sha256: stage_one_model_scientific_sha256.clone(),
            product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.to_string(),
            method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.to_string(),
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    identities
}

fn moderation_gamma_values_v1(
    plan: &CompiledPlsPlanV3,
    point: &GeneralSemPlsMultipleInteractionPointResultV1,
) -> Result<BTreeMap<String, f64>, GeneralSemPlsModerationBootstrapReplicateErrorV1> {
    point
        .ensure_valid_against_plan_v1(plan)
        .map_err(GeneralSemPlsModerationBootstrapReplicateErrorV1::JointStage)?;
    let values = point
        .interaction_coefficients()
        .iter()
        .map(|coefficient| {
            (
                coefficient.interaction_effect_relation_id().to_string(),
                coefficient.raw_product_estimate(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = moderation_gamma_target_identities_v1(plan)
        .into_iter()
        .map(|identity| identity.target_id)
        .collect::<BTreeSet<_>>();
    if values.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(GeneralSemPlsModerationBootstrapReplicateErrorV1::GammaInventoryMismatch);
    }
    Ok(values)
}

fn summarize_moderation_gammas_v1(
    identities: &[GeneralSemPlsModerationGammaTargetIdentityV1],
    original: &BTreeMap<String, f64>,
    replicates: &[&BTreeMap<String, f64>],
    confidence_level: f64,
) -> Result<
    Vec<GeneralSemPlsModerationBootstrapGammaInferenceV1>,
    GeneralSemPlsMultipleModerationBootstrapErrorV1,
> {
    identities
        .iter()
        .map(|identity| {
            let original = original.get(&identity.target_id).ok_or_else(|| {
                GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(format!(
                    "missing original gamma target {}",
                    identity.target_id
                ))
            })?;
            let values = replicates
                .iter()
                .map(|replicate| {
                    replicate.get(&identity.target_id).copied().ok_or_else(|| {
                        GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(format!(
                            "a usable replicate is missing gamma target {}",
                            identity.target_id
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let summary = summarize_effect(
                &identity.target_id,
                &identity.interaction_id,
                *original,
                values,
                confidence_level,
            )
            .map_err(|error| {
                GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidSummary(error.to_string())
            })?;
            Ok(GeneralSemPlsModerationBootstrapGammaInferenceV1 {
                target: identity.clone(),
                original: summary.original,
                bootstrap_mean: summary.bootstrap_mean,
                bootstrap_bias: summary.bootstrap_bias,
                standard_error: summary.standard_error,
                lower: summary.lower,
                upper: summary.upper,
                p_value_two_sided: summary.p_value_two_sided,
                usable_replicates: summary.usable_replicates,
                two_sided_exceedances: summary.two_sided_exceedances,
            })
        })
        .collect()
}

fn validate_point_execution_plan_domain_v1(
    dataset: &Dataset,
    point_execution: &ValidatedExecutionRecipe,
    plan: &CompiledPlsPlanV3,
) -> Result<(), GeneralSemPlsBootstrapErrorV1> {
    let source = point_execution.source();
    if plan.base_plan().dataset_id() != dataset.id.to_string() {
        return Err(
            GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(
                "compiled dataset id differs from the execution dataset".to_string(),
            ),
        );
    }
    if plan.scientific_hash() != plan.topology().model_scientific_sha256() {
        return Err(
            GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(
                "compiled plan scientific identities disagree".to_string(),
            ),
        );
    }
    if source.model.constructs.len() != plan.base_plan().blocks().len() {
        return Err(
            GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(
                "construct count differs".to_string(),
            ),
        );
    }
    for (construct, block) in source
        .model
        .constructs
        .iter()
        .zip(plan.base_plan().blocks())
    {
        let expected_mode = match block.mode() {
            CompiledPlsBlockModeV2::ModeA => MeasurementMode::Reflective,
            CompiledPlsBlockModeV2::ModeB => MeasurementMode::Formative,
        };
        let expected_indicators = block
            .indicators()
            .iter()
            .map(|indicator| indicator.source_column())
            .collect::<Vec<_>>();
        if construct.id != block.construct_id()
            || construct.mode != expected_mode
            || construct
                .indicators
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_indicators
        {
            return Err(
                GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(format!(
                    "construct {} differs",
                    construct.id
                )),
            );
        }
    }
    let recipe_structural = source
        .model
        .paths
        .iter()
        .map(|path| (path.source.as_str(), path.target.as_str()))
        .collect::<BTreeSet<_>>();
    let plan_structural = plan
        .base_plan()
        .paths()
        .iter()
        .filter(|path| path.role() == StructuralRelationRoleV4::Structural)
        .map(|path| (path.source(), path.target()))
        .collect::<BTreeSet<_>>();
    let recipe_controls = source
        .model
        .controls
        .iter()
        .map(|path| (path.source.as_str(), path.target.as_str()))
        .collect::<BTreeSet<_>>();
    let plan_controls = plan
        .base_plan()
        .paths()
        .iter()
        .filter(|path| path.role() == StructuralRelationRoleV4::Control)
        .map(|path| (path.source(), path.target()))
        .collect::<BTreeSet<_>>();
    if recipe_structural != plan_structural || recipe_controls != plan_controls {
        return Err(
            GeneralSemPlsBootstrapErrorV1::PointExecutionPlanDomainMismatch(
                "structural or control paths differ".to_string(),
            ),
        );
    }
    Ok(())
}

fn selected_effect_values(
    plan: &CompiledPlsPlanV3,
    estimation: &PlsResult,
) -> Result<BTreeMap<String, SelectedEffectValueV1>, GeneralSemPlsBootstrapReplicateErrorV1> {
    let relation_coefficients = plan
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| {
            let coefficients = match relation.role() {
                StructuralRelationRoleV4::Structural => estimation
                    .paths
                    .iter()
                    .filter(|estimate| {
                        estimate.source == relation.source() && estimate.target == relation.target()
                    })
                    .map(|estimate| estimate.coefficient)
                    .collect::<Vec<_>>(),
                StructuralRelationRoleV4::Control => estimation
                    .control_estimates
                    .iter()
                    .filter(|estimate| {
                        estimate.source == relation.source() && estimate.target == relation.target()
                    })
                    .map(|estimate| estimate.coefficient)
                    .collect::<Vec<_>>(),
            };
            if coefficients.len() != 1 {
                return Err(
                    GeneralSemPlsBootstrapReplicateErrorV1::RelationEstimateCardinality {
                        relation_id: relation.relation_id().to_string(),
                        source_id: relation.source().to_string(),
                        target_id: relation.target().to_string(),
                    },
                );
            }
            Ok((relation.relation_id().to_string(), coefficients[0]))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let decomposition = decompose_general_sem_effects_v1(plan.topology(), &relation_coefficients)?;
    let mut selected = BTreeMap::new();
    for estimand in plan.effect_estimands() {
        let (effect_id, estimand_id, value) = selected_effect(estimand, &decomposition)?;
        if selected
            .insert(
                effect_id.clone(),
                SelectedEffectValueV1 { estimand_id, value },
            )
            .is_some()
        {
            return Err(GeneralSemPlsBootstrapReplicateErrorV1::DuplicateEffectId(
                effect_id,
            ));
        }
    }
    Ok(selected)
}

fn selected_effect(
    estimand: &CompiledPlsEffectEstimandV3,
    decomposition: &GeneralSemEffectsV1,
) -> Result<(String, String, f64), GeneralSemPlsBootstrapReplicateErrorV1> {
    match estimand {
        CompiledPlsEffectEstimandV3::SpecificIndirect {
            estimand_id,
            path_identity,
            ..
        } => decomposition
            .specific_indirect_effects()
            .iter()
            .find(|effect| effect.specific_path_identity() == path_identity)
            .map(|effect| {
                (
                    path_identity.clone(),
                    estimand_id.clone(),
                    effect.coefficient(),
                )
            })
            .ok_or_else(|| {
                GeneralSemPlsBootstrapReplicateErrorV1::MissingCompiledEffect(estimand_id.clone())
            }),
        CompiledPlsEffectEstimandV3::TotalIndirect {
            estimand_id,
            source_id,
            target_id,
            ..
        } => pair_effect(decomposition, estimand_id, source_id, target_id).map(|effect| {
            (
                estimand_id.clone(),
                estimand_id.clone(),
                effect.total_indirect_effect(),
            )
        }),
        CompiledPlsEffectEstimandV3::TotalEffect {
            estimand_id,
            source_id,
            target_id,
            ..
        } => pair_effect(decomposition, estimand_id, source_id, target_id).map(|effect| {
            (
                estimand_id.clone(),
                estimand_id.clone(),
                effect.total_effect(),
            )
        }),
    }
}

fn pair_effect<'a>(
    decomposition: &'a GeneralSemEffectsV1,
    estimand_id: &str,
    source_id: &str,
    target_id: &str,
) -> Result<&'a qpls_core::GeneralSemPairEffectsV1, GeneralSemPlsBootstrapReplicateErrorV1> {
    decomposition
        .pair_effects()
        .iter()
        .find(|effect| effect.source_id() == source_id && effect.target_id() == target_id)
        .ok_or_else(|| {
            GeneralSemPlsBootstrapReplicateErrorV1::MissingCompiledEffect(estimand_id.to_string())
        })
}

fn align_general_sem_pls_signs_v1(
    estimate: &mut PlsResult,
    original_scores: &BTreeMap<String, Vec<f64>>,
    sampled_positions: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<(), EstimationError> {
    let mut signs = BTreeMap::new();
    for (construct, replicate_scores) in &estimate.construct_scores {
        if is_cancelled() {
            return Err(EstimationError::Cancelled);
        }
        let original = original_scores.get(construct).ok_or_else(|| {
            EstimationError::Numerical(format!(
                "missing original score for General SEM construct {construct}"
            ))
        })?;
        let aligned_reference = sampled_positions
            .iter()
            .map(|position| {
                original.get(*position).copied().ok_or_else(|| {
                    EstimationError::Numerical(format!(
                        "General SEM bootstrap score position {position} is outside construct {construct}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if aligned_reference.len() != replicate_scores.len() {
            return Err(EstimationError::Numerical(format!(
                "General SEM bootstrap score alignment length mismatch for {construct}"
            )));
        }
        signs.insert(
            construct.clone(),
            if covariance_v1(&aligned_reference, replicate_scores) < 0.0 {
                -1.0
            } else {
                1.0
            },
        );
    }
    for path in &mut estimate.paths {
        path.coefficient *= relation_sign_v1(&signs, &path.source, &path.target)?;
    }
    for control in &mut estimate.control_estimates {
        control.coefficient *= relation_sign_v1(&signs, &control.source, &control.target)?;
    }
    Ok(())
}

fn relation_sign_v1(
    signs: &BTreeMap<String, f64>,
    source: &str,
    target: &str,
) -> Result<f64, EstimationError> {
    let source_sign = signs.get(source).ok_or_else(|| {
        EstimationError::Numerical(format!(
            "missing General SEM bootstrap sign for construct {source}"
        ))
    })?;
    let target_sign = signs.get(target).ok_or_else(|| {
        EstimationError::Numerical(format!(
            "missing General SEM bootstrap sign for construct {target}"
        ))
    })?;
    Ok(source_sign * target_sign)
}

fn covariance_v1(left: &[f64], right: &[f64]) -> f64 {
    let left_mean = stable_sum(left) / left.len() as f64;
    let right_mean = stable_sum(right) / right.len() as f64;
    stable_sum(
        &left
            .iter()
            .zip(right)
            .map(|(left, right)| (left - left_mean) * (right - right_mean))
            .collect::<Vec<_>>(),
    )
}

fn summarize_effects(
    effect_ids: &[String],
    original: &BTreeMap<String, SelectedEffectValueV1>,
    replicates: &[&BTreeMap<String, SelectedEffectValueV1>],
    confidence_level: f64,
) -> Result<Vec<GeneralSemPlsBootstrapEffectInferenceV1>, GeneralSemPlsBootstrapErrorV1> {
    effect_ids
        .iter()
        .map(|effect_id| {
            let original = original.get(effect_id).ok_or_else(|| {
                GeneralSemPlsBootstrapErrorV1::InvalidSummary(format!(
                    "missing original effect {effect_id}"
                ))
            })?;
            let values = replicates
                .iter()
                .map(|replicate| {
                    replicate
                        .get(effect_id)
                        .map(|value| value.value)
                        .ok_or_else(|| {
                            GeneralSemPlsBootstrapErrorV1::InvalidSummary(format!(
                                "a usable replicate is missing effect {effect_id}"
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            summarize_effect(
                effect_id,
                &original.estimand_id,
                original.value,
                values,
                confidence_level,
            )
        })
        .collect()
}

fn summarize_effect(
    effect_id: &str,
    estimand_id: &str,
    original: f64,
    mut values: Vec<f64>,
    confidence_level: f64,
) -> Result<GeneralSemPlsBootstrapEffectInferenceV1, GeneralSemPlsBootstrapErrorV1> {
    if !original.is_finite() || values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(GeneralSemPlsBootstrapErrorV1::InvalidSummary(format!(
            "effect {effect_id} requires a finite point estimate and at least two finite replicates"
        )));
    }
    let mean = stable_sum(&values) / values.len() as f64;
    let squared_deviations = values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .collect::<Vec<_>>();
    let standard_error = (stable_sum(&squared_deviations) / (values.len() - 1) as f64).sqrt();
    values.sort_by(|left, right| left.total_cmp(right));
    let alpha = 1.0 - confidence_level;
    let lower = type7_quantile(&values, alpha / 2.0);
    let upper = type7_quantile(&values, 1.0 - alpha / 2.0);
    let exceedances = values
        .iter()
        .filter(|value| (**value - original).abs() >= original.abs())
        .count();
    let p_value_two_sided = (exceedances + 1) as f64 / (values.len() + 1) as f64;
    let result = GeneralSemPlsBootstrapEffectInferenceV1 {
        effect_id: effect_id.to_string(),
        estimand_id: estimand_id.to_string(),
        original: canonical_zero(original),
        bootstrap_mean: canonical_zero(mean),
        bootstrap_bias: canonical_zero(mean - original),
        standard_error: canonical_zero(standard_error),
        lower: canonical_zero(lower),
        upper: canonical_zero(upper),
        p_value_two_sided,
        usable_replicates: values.len() as u32,
        two_sided_exceedances: exceedances as u32,
    };
    if [
        result.bootstrap_mean,
        result.bootstrap_bias,
        result.standard_error,
        result.lower,
        result.upper,
        result.p_value_two_sided,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(GeneralSemPlsBootstrapErrorV1::InvalidSummary(format!(
            "effect {effect_id} produced a non-finite summary"
        )));
    }
    Ok(result)
}

fn minimum_usable_replicates(requested: u32) -> usize {
    ((f64::from(requested) * 0.9).ceil() as usize).max(2)
}

fn stable_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for value in values {
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            compensation += (sum - updated) + value;
        } else {
            compensation += (value - updated) + sum;
        }
        sum = updated;
    }
    sum + compensation
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn is_lowercase_sha256_v1(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_dataset_fingerprint_v1(value: &str) -> bool {
    value
        .strip_prefix("v2:")
        .map_or_else(|| is_lowercase_sha256_v1(value), is_lowercase_sha256_v1)
}

fn approximately_equal_v1(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1e-12 * scale
}

#[derive(Serialize)]
struct CompleteCaseFrameIdentityV1 {
    dataset_id: String,
    dataset_fingerprint: String,
    source_columns: Vec<String>,
    raw_row_indices: Vec<u64>,
}

fn sha256_serialized<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, GeneralSemPlsBootstrapErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| GeneralSemPlsBootstrapErrorV1::Serialization(error.to_string()))
}

fn moderation_sha256_serialized<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, GeneralSemPlsMultipleModerationBootstrapErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| {
            GeneralSemPlsMultipleModerationBootstrapErrorV1::Serialization(error.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct, ControlPath,
        GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
        SemDataBindingV4, SemDerivedTermV4, SemModelV4, SemParameterTargetV4, SemParameterV4,
        SemRelationV4, SemVariableV4, StructuralPath, StructuralRelationRoleV4,
        compile_pls_plan_v3, convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    const FAILURE_BOUNDARY_CASES: usize = 8;
    const FAILURE_BOUNDARY_RESAMPLES: u32 = 20;
    const FAILURE_BOUNDARY_PUBLISHABLE_SEED: u64 = 2;
    const FAILURE_BOUNDARY_REJECTED_SEED: u64 = 8;

    struct ModerationBootstrapFixtureV1 {
        dataset: Dataset,
        execution: ValidatedExecutionRecipe,
        plan: CompiledPlsPlanV3,
        original_stage_one: PlsResult,
        original_point: GeneralSemPlsMultipleInteractionPointResultV1,
        config: GeneralSemConfigV1,
    }

    fn structural_relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source: actual_source,
                    target: actual_target,
                    ..
                } if actual_source == source && actual_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn add_two_stage_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = structural_relation_id(model, focal_predictor_id, "construct:y");
        let output = format!("derived:{interaction_id}");
        let relation_id = format!("relation:{interaction_id}:effect");
        let parameter_id = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.to_string(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: relation_id,
            source: output.clone(),
            target: "construct:y".to_string(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter_id,
            label: format!("{interaction_id} -> Y"),
            target: SemParameterTargetV4::Regression {
                source: output.clone(),
                target: "construct:y".to_string(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.to_string(),
            output,
            operands: vec![focal_predictor_id.to_string(), moderator_id.to_string()],
            focal_relation,
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
    }

    fn moderation_bootstrap_fixture(
        interactions_on_different_focal_paths: bool,
    ) -> ModerationBootstrapFixtureV1 {
        let mut csv = String::from("x1,x2,w1,w2,z1,z2,y1,y2\n");
        for row in 0..72 {
            let row = row as f64;
            let x = (0.17 * row).sin() + 0.013 * row;
            let w = (0.31 * row + 0.4).cos() + 0.20 * (0.07 * row).sin();
            let z = (0.23 * row - 0.2).sin() - 0.15 * (0.11 * row).cos();
            let second_product = if interactions_on_different_focal_paths {
                w * z
            } else {
                x * z
            };
            let y = 0.25 * x + 0.20 * w - 0.15 * z + 0.55 * x * w - 0.35 * second_product
                + 0.05 * (0.73 * row).sin();
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                x + 0.07 * (1.07 * row).sin(),
                0.91 * x + 0.05 * (0.83 * row).cos(),
                w + 0.06 * (0.97 * row).cos(),
                0.94 * w - 0.04 * (0.67 * row).sin(),
                z + 0.05 * (0.89 * row).sin(),
                0.92 * z + 0.04 * (0.59 * row).cos(),
                y + 0.05 * (1.13 * row).cos(),
                0.95 * y - 0.04 * (0.79 * row).sin(),
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "general-sem-multiple-moderation-bootstrap.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5331),
            name: "General SEM multiple moderation bootstrap".to_string(),
            constructs: ["x", "w", "z", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.to_string(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [("x", "y"), ("w", "y"), ("z", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.to_string(),
                    target: target.to_string(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        add_two_stage_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        if interactions_on_different_focal_paths {
            add_two_stage_interaction(
                &mut model,
                "interaction:w_by_z",
                "construct:w",
                "construct:z",
            );
        } else {
            add_two_stage_interaction(
                &mut model,
                "interaction:x_by_z",
                "construct:x",
                "construct:z",
            );
        }

        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 20,
            seed: 7_331,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        let projected_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5332),
            name: legacy_model.name,
            constructs: plan
                .base_plan()
                .blocks()
                .iter()
                .map(|block| Construct {
                    id: block.construct_id().to_string(),
                    name: block.construct_id().to_string(),
                    short_name: block.construct_id().to_string(),
                    mode: match block.mode() {
                        CompiledPlsBlockModeV2::ModeA => MeasurementMode::Reflective,
                        CompiledPlsBlockModeV2::ModeB => MeasurementMode::Formative,
                    },
                    indicators: block
                        .indicators()
                        .iter()
                        .map(|indicator| indicator.source_column().to_string())
                        .collect(),
                })
                .collect(),
            paths: plan
                .base_plan()
                .paths()
                .iter()
                .filter(|path| path.role() == StructuralRelationRoleV4::Structural)
                .map(|path| StructuralPath {
                    source: path.source().to_string(),
                    target: path.target().to_string(),
                })
                .collect(),
            controls: plan
                .base_plan()
                .paths()
                .iter()
                .filter(|path| path.role() == StructuralRelationRoleV4::Control)
                .map(|path| ControlPath {
                    source: path.source().to_string(),
                    target: path.target().to_string(),
                    label: None,
                })
                .collect(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_5333),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: projected_model,
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                bootstrap_samples: 0,
                studentized_inner_samples: 0,
                permutation_samples: 0,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let original_stage_one =
            estimate_pls_validated_with_control(&dataset, &execution, |_| true).unwrap();
        let original_point =
            estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                &plan,
                &original_stage_one.construct_scores,
                || true,
            )
            .unwrap();
        ModerationBootstrapFixtureV1 {
            dataset,
            execution,
            plan,
            original_stage_one,
            original_point,
            config,
        }
    }

    fn integration_fixture() -> (
        Dataset,
        ValidatedExecutionRecipe,
        CompiledPlsPlanV3,
        PlsResult,
        GeneralSemConfigV1,
    ) {
        let mut csv = String::from("x1,x2,m1,m2,y1,y2\n");
        for row in 0..40 {
            let x = row as f64 / 5.0 - 4.0;
            let wave = (row % 5) as f64 - 2.0;
            let mediator = 0.65 * x + 0.18 * wave;
            let outcome = 0.30 * x + 0.72 * mediator + 0.11 * ((row * 3) % 7) as f64;
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                x + 0.03 * wave,
                x - 0.02 * wave,
                mediator + 0.04 * ((row + 1) % 3) as f64,
                mediator - 0.03 * ((row + 2) % 4) as f64,
                outcome + 0.05 * ((row + 3) % 5) as f64,
                outcome - 0.02 * ((row + 4) % 6) as f64,
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "general-sem-bootstrap.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5311),
            name: "General SEM bootstrap integration".to_string(),
            constructs: ["x", "m", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.to_string(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [("x", "m"), ("m", "y"), ("x", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.to_string(),
                    target: target.to_string(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 20,
            seed: GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        let projected_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5312),
            name: legacy_model.name,
            constructs: plan
                .base_plan()
                .blocks()
                .iter()
                .map(|block| Construct {
                    id: block.construct_id().to_string(),
                    name: block.construct_id().to_string(),
                    short_name: block.construct_id().to_string(),
                    mode: match block.mode() {
                        qpls_core::CompiledPlsBlockModeV2::ModeA => MeasurementMode::Reflective,
                        qpls_core::CompiledPlsBlockModeV2::ModeB => MeasurementMode::Formative,
                    },
                    indicators: block
                        .indicators()
                        .iter()
                        .map(|indicator| indicator.source_column().to_string())
                        .collect(),
                })
                .collect(),
            paths: plan
                .base_plan()
                .paths()
                .iter()
                .map(|path| StructuralPath {
                    source: path.source().to_string(),
                    target: path.target().to_string(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_5313),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: projected_model,
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                bootstrap_samples: 0,
                studentized_inner_samples: 0,
                permutation_samples: 0,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let original = estimate_pls_validated_with_control(&dataset, &execution, |_| true).unwrap();
        (dataset, execution, plan, original, config)
    }

    fn failure_boundary_fixture(
        one_rows: usize,
        seed: u64,
    ) -> (
        Dataset,
        ValidatedExecutionRecipe,
        CompiledPlsPlanV3,
        PlsResult,
        GeneralSemConfigV1,
    ) {
        assert!((1..FAILURE_BOUNDARY_CASES).contains(&one_rows));
        let mut csv = String::from("x1,x2,y1,y2\n");
        for row in 0..FAILURE_BOUNDARY_CASES {
            let x1 = if row < one_rows { 1.0 } else { 0.0 };
            let x2 = 0.2 * row as f64 + 2.0 * x1;
            let wave = ((row * 5 + 1) % 7) as f64 - 3.0;
            let outcome = 0.85 * x2 + 0.07 * wave;
            csv.push_str(&format!(
                "{x1},{x2},{},{}\n",
                outcome + 0.03 * (row % 3) as f64,
                outcome - 0.02 * ((row + 1) % 4) as f64,
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "general-sem-bootstrap-failure-boundary.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5321),
            name: "General SEM bootstrap failure boundary".to_string(),
            constructs: ["x", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.to_string(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [("x", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.to_string(),
                    target: target.to_string(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy_model,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: FAILURE_BOUNDARY_RESAMPLES,
            seed,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        let projected_model = ModelSpec {
            id: Uuid::from_u128(0x5031_5322),
            name: legacy_model.name,
            constructs: plan
                .base_plan()
                .blocks()
                .iter()
                .map(|block| Construct {
                    id: block.construct_id().to_string(),
                    name: block.construct_id().to_string(),
                    short_name: block.construct_id().to_string(),
                    mode: match block.mode() {
                        qpls_core::CompiledPlsBlockModeV2::ModeA => MeasurementMode::Reflective,
                        qpls_core::CompiledPlsBlockModeV2::ModeB => MeasurementMode::Formative,
                    },
                    indicators: block
                        .indicators()
                        .iter()
                        .map(|indicator| indicator.source_column().to_string())
                        .collect(),
                })
                .collect(),
            paths: plan
                .base_plan()
                .paths()
                .iter()
                .map(|path| StructuralPath {
                    source: path.source().to_string(),
                    target: path.target().to_string(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_5323),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: projected_model,
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                bootstrap_samples: 0,
                studentized_inner_samples: 0,
                permutation_samples: 0,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let original = estimate_pls_validated_with_control(&dataset, &execution, |_| true).unwrap();
        (dataset, execution, plan, original, config)
    }

    fn expected_constant_x1_replicates(one_rows: usize, seed: u64) -> Vec<u32> {
        (0..FAILURE_BOUNDARY_RESAMPLES)
            .filter(|replicate_index| {
                let sampled_positions = crate::bootstrap_indices(
                    FAILURE_BOUNDARY_CASES,
                    seed,
                    GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1,
                    *replicate_index,
                );
                let first_is_one = sampled_positions[0] < one_rows;
                sampled_positions
                    .iter()
                    .all(|position| (*position < one_rows) == first_is_one)
            })
            .collect()
    }

    #[test]
    fn type7_sample_standard_error_bias_and_null_centered_probability_are_frozen() {
        let summary =
            summarize_effect("effect_a", "estimand_a", 2.0, vec![1.0, 2.0, 3.0, 4.0], 0.5).unwrap();
        assert_eq!(summary.bootstrap_mean, 2.5);
        assert_eq!(summary.bootstrap_bias, 0.5);
        assert!((summary.standard_error - 1.290_994_448_735_805_6).abs() < 1e-15);
        assert_eq!(summary.lower, 1.75);
        assert_eq!(summary.upper, 3.25);
        assert_eq!(summary.p_value_two_sided, 0.4);
        assert_eq!(summary.usable_replicates, 4);
        assert_eq!(summary.two_sided_exceedances, 1);
    }

    #[test]
    fn usable_gate_is_exactly_ceiling_ninety_percent_with_minimum_two() {
        assert_eq!(minimum_usable_replicates(2), 2);
        assert_eq!(minimum_usable_replicates(9), 9);
        assert_eq!(minimum_usable_replicates(10), 9);
        assert_eq!(minimum_usable_replicates(11), 10);
        assert_eq!(minimum_usable_replicates(10_000), 9_000);
    }

    #[test]
    fn summary_rejects_partial_or_nonfinite_effect_ledgers() {
        assert!(summarize_effect("e", "q", 0.1, vec![0.2], 0.95).is_err());
        assert!(summarize_effect("e", "q", 0.1, vec![0.2, f64::NAN], 0.95).is_err());
    }

    #[test]
    fn full_model_bootstrap_is_numerically_identical_across_worker_counts() {
        let (dataset, execution, plan, original, config) = integration_fixture();
        let execute = |workers| {
            let progress = Arc::new(Mutex::new(Vec::new()));
            let result = bootstrap_general_sem_pls_v1(
                &dataset,
                &execution,
                &plan,
                &original,
                &config,
                None,
                workers,
                || false,
                {
                    let progress = progress.clone();
                    move |update| progress.lock().unwrap().push(update)
                },
            )
            .unwrap();
            assert_eq!(progress.lock().unwrap().len(), 20);
            result
        };
        let serial = execute(1);
        let parallel = execute(4);
        let mut normalized_parallel = parallel.clone();
        normalized_parallel.workers = serial.workers;
        assert_eq!(serial, normalized_parallel);
        assert_eq!(serial.effects, parallel.effects);
        assert_eq!(serial.failed_replicates, parallel.failed_replicates);
        assert_eq!(
            serial.usable_replicate_indices_sha256,
            parallel.usable_replicate_indices_sha256
        );
        assert_eq!(serial.effect_ids, parallel.effect_ids);
        assert_eq!(serial.resamples_requested, 20);
        assert_eq!(serial.resamples_usable, 20);
        assert_eq!(serial.minimum_usable_resamples, 18);
        assert!(serial.complete_model_reestimated_per_replicate);
        assert!(serial.failed_replicates.is_empty());
        assert!(
            serial
                .effects
                .iter()
                .any(|effect| effect.effect_id.starts_with("sem_specific_path_v1_"))
        );
        assert!(serial.effects.iter().all(|effect| {
            effect.usable_replicates == 20
                && effect.lower <= effect.upper
                && (0.0..=1.0).contains(&effect.p_value_two_sided)
        }));
    }

    #[test]
    fn full_model_bootstrap_publishes_ordered_typed_failure_ledger_above_usable_gate() {
        let expected_failure_indices =
            expected_constant_x1_replicates(4, FAILURE_BOUNDARY_PUBLISHABLE_SEED);
        assert_eq!(expected_failure_indices, vec![5]);
        let (dataset, execution, plan, original, config) =
            failure_boundary_fixture(4, FAILURE_BOUNDARY_PUBLISHABLE_SEED);
        let progress = Arc::new(Mutex::new(Vec::new()));
        let result = bootstrap_general_sem_pls_v1(
            &dataset,
            &execution,
            &plan,
            &original,
            &config,
            None,
            1,
            || false,
            {
                let progress = progress.clone();
                move |update| progress.lock().unwrap().push(update)
            },
        )
        .unwrap();

        assert_eq!(progress.lock().unwrap().len(), 20);
        assert_eq!(result.resamples_requested, 20);
        assert_eq!(result.resamples_usable, 19);
        assert_eq!(result.minimum_usable_resamples, 18);
        assert_eq!(
            result.failed_replicates,
            expected_failure_indices
                .into_iter()
                .map(|replicate_index| GeneralSemPlsBootstrapFailedReplicateV1 {
                    replicate_index,
                    reason_code: GeneralSemPlsBootstrapFailureCodeV1::ConstantIndicator,
                    message: "constant indicator: x1".to_string(),
                })
                .collect::<Vec<_>>()
        );
        assert!(
            result
                .effects
                .iter()
                .all(|effect| effect.usable_replicates == 19)
        );
        result.ensure_valid().unwrap();
    }

    #[test]
    fn full_model_bootstrap_returns_typed_error_without_result_below_usable_gate() {
        let expected_failure_indices =
            expected_constant_x1_replicates(1, FAILURE_BOUNDARY_REJECTED_SEED);
        assert_eq!(expected_failure_indices, vec![1, 4, 6, 8, 10, 17, 18]);
        let (dataset, execution, plan, original, config) =
            failure_boundary_fixture(1, FAILURE_BOUNDARY_REJECTED_SEED);
        let progress = Arc::new(Mutex::new(Vec::new()));
        let error = bootstrap_general_sem_pls_v1(
            &dataset,
            &execution,
            &plan,
            &original,
            &config,
            None,
            1,
            || false,
            {
                let progress = progress.clone();
                move |update| progress.lock().unwrap().push(update)
            },
        )
        .unwrap_err();

        assert_eq!(progress.lock().unwrap().len(), 20);
        assert!(matches!(
            error,
            GeneralSemPlsBootstrapErrorV1::InsufficientUsableReplicates {
                usable: 13,
                required: 18,
            }
        ));
    }

    #[test]
    fn full_model_bootstrap_cancellation_is_terminal() {
        let (dataset, execution, plan, original, config) = integration_fixture();
        assert!(matches!(
            bootstrap_general_sem_pls_v1(
                &dataset,
                &execution,
                &plan,
                &original,
                &config,
                None,
                2,
                || true,
                |_| {},
            ),
            Err(GeneralSemPlsBootstrapErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
    }

    #[test]
    fn stale_same_size_point_result_is_rejected_before_resampling() {
        let (dataset, execution, plan, mut original, config) = integration_fixture();
        original.paths[0].coefficient += 0.001;
        assert!(matches!(
            bootstrap_general_sem_pls_v1(
                &dataset,
                &execution,
                &plan,
                &original,
                &config,
                None,
                1,
                || false,
                |_| {},
            ),
            Err(GeneralSemPlsBootstrapErrorV1::OriginalPointEstimateMismatch)
        ));
    }

    #[test]
    fn deserialized_result_contract_rejects_inference_ledger_tampering() {
        let (dataset, execution, plan, original, config) = integration_fixture();
        let result = bootstrap_general_sem_pls_v1(
            &dataset,
            &execution,
            &plan,
            &original,
            &config,
            None,
            2,
            || false,
            |_| {},
        )
        .unwrap();
        result.ensure_valid().unwrap();

        let mut tampered = result;
        tampered.effects[0].p_value_two_sided += 0.01;
        assert!(matches!(
            tampered.ensure_valid(),
            Err(GeneralSemPlsBootstrapErrorV1::InvalidResultContract(_))
        ));
    }

    #[test]
    fn moderation_full_model_bootstrap_is_worker_invariant_for_same_and_different_focal_paths() {
        for different_focal_paths in [false, true] {
            let fixture = moderation_bootstrap_fixture(different_focal_paths);
            let execute = |workers| {
                let progress = Arc::new(Mutex::new(Vec::new()));
                let result = bootstrap_general_sem_pls_multiple_two_way_moderation_v1(
                    &fixture.dataset,
                    &fixture.execution,
                    &fixture.plan,
                    &fixture.original_stage_one,
                    &fixture.original_point,
                    &fixture.config,
                    None,
                    workers,
                    || false,
                    {
                        let progress = progress.clone();
                        move |update| progress.lock().unwrap().push(update)
                    },
                )
                .unwrap();
                assert_eq!(progress.lock().unwrap().len(), 20);
                result
            };

            let serial = execute(1);
            let parallel = execute(4);
            let mut normalized_parallel = parallel.clone();
            normalized_parallel.workers = serial.workers;
            assert_eq!(serial, normalized_parallel);
            assert_eq!(serial.interaction_gammas.len(), 2);
            assert_eq!(serial.resamples_requested, 20);
            assert_eq!(serial.resamples_usable, 20);
            assert_eq!(serial.minimum_usable_resamples, 18);
            assert!(serial.failed_replicates.is_empty());
            assert!(serial.complete_model_reestimated_per_replicate);
            assert!(serial.shared_stage_one_reestimated_per_replicate);
            assert!(serial.score_vectors_sign_aligned_before_products);
            assert!(serial.product_scaling_recomputed_per_replicate);
            assert!(serial.joint_stage_two_reestimated_per_replicate);
            assert!(serial.complete_joint_point_contract_validated_per_replicate);
            assert!(serial.interaction_gammas.iter().all(|gamma| {
                !gamma.target.generated_product_column_id.is_empty()
                    && gamma.target.stage_one_model_scientific_sha256
                        == serial.stage_one_model_scientific_sha256
                    && gamma.target.product_scale_version
                        == GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
                    && gamma.target.method_version
                        == GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
            }));
            serial
                .ensure_valid_against_plan_v1(&fixture.plan, &fixture.original_point)
                .unwrap();

            let wire = serde_json::to_value(&serial).unwrap();
            assert!(wire.get("structural_coefficients").is_none());
            assert!(wire.get("standardized_product_estimates").is_none());
            assert!(wire.get("simple_slopes").is_none());
            assert!(wire.get("plot_points").is_none());
            assert!(
                wire["interaction_gammas"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|gamma| gamma.get("standardized_product_estimate").is_none()
                        && gamma.get("simple_slopes").is_none())
            );
        }
    }

    #[test]
    fn moderation_score_alignment_precedes_recomputed_product_scaling() {
        let fixture = moderation_bootstrap_fixture(false);
        let observation_count = fixture.original_stage_one.used_observations;
        let sampled_positions = (0..observation_count)
            .map(|position| position / 2)
            .collect::<Vec<_>>();
        let mut replicate_scores = fixture
            .original_stage_one
            .construct_scores
            .iter()
            .map(|(construct_id, scores)| {
                let mut sampled = sampled_positions
                    .iter()
                    .map(|position| scores[*position])
                    .collect::<Vec<_>>();
                if construct_id == "construct:x" || construct_id == "construct:y" {
                    sampled.iter_mut().for_each(|value| *value = -*value);
                }
                (construct_id.clone(), sampled)
            })
            .collect::<BTreeMap<_, _>>();

        align_general_sem_pls_moderation_score_vectors_v1(
            &mut replicate_scores,
            &fixture.original_stage_one.construct_scores,
            &sampled_positions,
            &|| false,
        )
        .unwrap();
        for (construct_id, scores) in &replicate_scores {
            let original = &fixture.original_stage_one.construct_scores[construct_id];
            assert_eq!(
                scores,
                &sampled_positions
                    .iter()
                    .map(|position| original[*position])
                    .collect::<Vec<_>>()
            );
        }

        let replicate_point =
            estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                &fixture.plan,
                &replicate_scores,
                || true,
            )
            .unwrap();
        replicate_point
            .ensure_valid_against_plan_v1(&fixture.plan)
            .unwrap();
        assert!(
            replicate_point
                .product_scale_receipts()
                .iter()
                .any(|receipt| {
                    let original = fixture
                        .original_point
                        .product_scale_receipts()
                        .iter()
                        .find(|candidate| candidate.interaction_id() == receipt.interaction_id())
                        .unwrap();
                    (receipt.unstandardized_product_sample_standard_deviation()
                        - original.unstandardized_product_sample_standard_deviation())
                    .abs()
                        > 1e-12
                })
        );
        for receipt in replicate_point.product_scale_receipts() {
            let coefficient = replicate_point
                .interaction_coefficients()
                .iter()
                .find(|coefficient| coefficient.interaction_id() == receipt.interaction_id())
                .unwrap();
            let reconstructed = coefficient.raw_product_estimate()
                * receipt.unstandardized_product_sample_standard_deviation();
            let scale = reconstructed
                .abs()
                .max(coefficient.standardized_product_estimate().abs())
                .max(1.0);
            assert!(
                (reconstructed - coefficient.standardized_product_estimate()).abs()
                    <= 1e-12 * scale
            );
        }
    }

    #[test]
    fn moderation_failure_codes_and_exact_ninety_percent_gate_are_frozen() {
        let failure_code = |error: GeneralSemPlsModerationBootstrapReplicateErrorV1| {
            let GeneralSemPlsModerationBootstrapReplicateRecordV1::Failed { reason_code, .. } =
                error.into_record()
            else {
                panic!("expected a typed failed-replicate record")
            };
            reason_code
        };
        assert_eq!(
            failure_code(GeneralSemPlsModerationBootstrapReplicateErrorV1::StageOne(
                EstimationError::ConstantIndicator("x1".to_string()),
            )),
            GeneralSemPlsModerationBootstrapFailureCodeV1::ConstantIndicator
        );
        assert_eq!(
            failure_code(
                GeneralSemPlsModerationBootstrapReplicateErrorV1::SignAlignment(
                    GeneralSemPlsModerationScoreAlignmentErrorV1::IndeterminateSign {
                        construct_id: "construct:x".to_string(),
                    },
                )
            ),
            GeneralSemPlsModerationBootstrapFailureCodeV1::IndeterminateScoreSign
        );
        assert_eq!(
            failure_code(
                GeneralSemPlsModerationBootstrapReplicateErrorV1::JointStage(
                    GeneralSemPlsInteractionPointErrorV1::ConstantProduct {
                        interaction_id: "interaction:x_by_w".to_string(),
                    },
                )
            ),
            GeneralSemPlsModerationBootstrapFailureCodeV1::ConstantInteractionProduct
        );
        assert_eq!(
            failure_code(
                GeneralSemPlsModerationBootstrapReplicateErrorV1::JointStage(
                    GeneralSemPlsInteractionPointErrorV1::RankDeficient {
                        outcome_id: "construct:y".to_string(),
                        predictor_id: "derived:interaction:x_by_w".to_string(),
                    },
                )
            ),
            GeneralSemPlsModerationBootstrapFailureCodeV1::JointStageRankDeficient
        );
        assert_eq!(minimum_usable_replicates(20), 18);
        assert!(17 < minimum_usable_replicates(20));
        assert!(18 >= minimum_usable_replicates(20));
    }

    #[test]
    fn moderation_full_model_bootstrap_cancellation_is_terminal() {
        let fixture = moderation_bootstrap_fixture(false);
        assert!(matches!(
            bootstrap_general_sem_pls_multiple_two_way_moderation_v1(
                &fixture.dataset,
                &fixture.execution,
                &fixture.plan,
                &fixture.original_stage_one,
                &fixture.original_point,
                &fixture.config,
                None,
                2,
                || true,
                |_| {},
            ),
            Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
    }

    #[test]
    fn moderation_bootstrap_rejects_tampered_original_point_before_resampling() {
        let fixture = moderation_bootstrap_fixture(false);
        let mut wire = serde_json::to_value(&fixture.original_point).unwrap();
        let raw_gamma = wire["interaction_coefficients"][0]["raw_product_estimate"]
            .as_f64()
            .unwrap();
        wire["interaction_coefficients"][0]["raw_product_estimate"] =
            serde_json::json!(raw_gamma + 0.01);
        let tampered = serde_json::from_value(wire).unwrap();
        let progress = Arc::new(Mutex::new(Vec::new()));
        let error = bootstrap_general_sem_pls_multiple_two_way_moderation_v1(
            &fixture.dataset,
            &fixture.execution,
            &fixture.plan,
            &fixture.original_stage_one,
            &tampered,
            &fixture.config,
            None,
            1,
            || false,
            {
                let progress = progress.clone();
                move |update| progress.lock().unwrap().push(update)
            },
        )
        .unwrap_err();
        assert!(progress.lock().unwrap().is_empty());
        assert!(matches!(
            error,
            GeneralSemPlsMultipleModerationBootstrapErrorV1::OriginalInteractionPoint(
                GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(_)
            )
        ));
    }

    #[test]
    fn moderation_bootstrap_result_rejects_gamma_target_tampering() {
        let fixture = moderation_bootstrap_fixture(false);
        let result = bootstrap_general_sem_pls_multiple_two_way_moderation_v1(
            &fixture.dataset,
            &fixture.execution,
            &fixture.plan,
            &fixture.original_stage_one,
            &fixture.original_point,
            &fixture.config,
            None,
            2,
            || false,
            |_| {},
        )
        .unwrap();
        result
            .ensure_valid_against_plan_v1(&fixture.plan, &fixture.original_point)
            .unwrap();

        let mut tampered = result;
        tampered.interaction_gammas[0].target.moderator_id = "construct:other".to_string();
        assert!(matches!(
            tampered.ensure_valid(),
            Err(GeneralSemPlsMultipleModerationBootstrapErrorV1::InvalidResultContract(_))
        ));
    }
}
