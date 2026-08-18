use super::{
    BootstrapPlan, ReplicateOutcome, ResamplingError, ResamplingProgress, complete_case_rows,
    resample_model_dataset, run_bootstrap, type7_quantile,
};
use qpls_core::{
    AnalysisMethod, CompiledPlsBlockModeV2, CompiledPlsEffectEstimandV3, CompiledPlsPlanV3,
    GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemConfigV1ValidationError,
    GeneralSemEffectsV1, GeneralSemEffectsV1Error, GeneralSemInferenceTailV1,
    GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1, MeasurementMode, MethodConfig,
    PlsAlgorithmConfigV2, StructuralRelationRoleV4, ValidatedExecutionRecipe,
    compiled_pls_effect_identities_v1, decompose_general_sem_effects_v1,
    general_sem_effect_identity_set_sha256_v1,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, PLS_METHOD_VERSION, PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2,
    PLS_SCORE_EXECUTION_METHOD_VERSION_V2, PlsResult,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, AnalysisSettings, Construct,
        GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1, LegacyBasicModelInterpretationV4, MeasurementMode,
        MethodConfig, ModelSpec, SemDataBindingV4, StructuralPath, compile_pls_plan_v3,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    const FAILURE_BOUNDARY_CASES: usize = 8;
    const FAILURE_BOUNDARY_RESAMPLES: u32 = 20;
    const FAILURE_BOUNDARY_PUBLISHABLE_SEED: u64 = 2;
    const FAILURE_BOUNDARY_REJECTED_SEED: u64 = 8;

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
}
