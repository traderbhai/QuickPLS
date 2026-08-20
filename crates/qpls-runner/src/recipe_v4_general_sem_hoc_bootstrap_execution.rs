use crate::{
    GeneralSemPlsHigherOrderPointErrorV1, GeneralSemPlsHigherOrderPointResultV1,
    GeneralSemPlsHocPointRelationKindV1, GeneralSemPlsHocScoreAlignmentErrorV1,
    GeneralSemPlsHocScoreAlignmentReferenceV1, GeneralSemPlsHocStageAlignmentReferencesV1,
    RecipeV4PlsExecutionError, RecipeV4PlsExecutionResultV1,
    align_general_sem_pls_hoc_result_signs_v1, compile_general_sem_pls_hoc_point_context_v1,
    extended_effect_identity_v1, run_compiled_general_sem_pls_higher_order_point_with_context_v1,
    run_compiled_pls_recipe_v4_allowing_isolated,
};
use qpls_core::{
    AnalysisRecipeV4, CompiledPlsHocComponentRelationInterpretationV1, CompiledPlsPlanV3,
    CompiledRecipePlanV4, GeneralSemBootstrapIntervalV1, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
    GeneralSemPlsRecipeCompilationErrorV1, HigherOrderConstructionApproachV4,
    RecipeV4CompilationError, RecipeV4CompilerTarget, SemModelV4, StructuralRelationRoleV4,
    compile_analysis_recipe_v4, project_general_sem_pls_stage_one_recipe_v1, sha256_serialized,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, GeneralSemPlsDisjointHocScoreDatasetErrorV1,
    general_sem_pls_hoc_complete_case_rows_v1,
};
use qpls_resampling::{
    BootstrapPlan, GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1,
    GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1, GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1,
    GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1,
    GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1, GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1,
    ReplicateOutcome, ResamplingError, ResamplingProgress, resample_dataset_columns_v1,
    run_bootstrap,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
};

pub const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1: &str =
    qpls_core::PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_VERSION_V1;
pub const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1: &str =
    "general_sem_pls_higher_order_full_model_case_bootstrap_operation_v1";
pub const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_SIGN_ALIGNMENT_VERSION_V1: &str =
    "sampled_original_construct_score_covariance_v1";
pub const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1: &str =
    "compiled_hoc_component_and_structural_relation_target_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemPlsHocBootstrapTargetKindV1 {
    ComponentLoading,
    ComponentWeight,
    HocStructuralPath,
    ExtendedTotalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocBootstrapTargetIdentityV1 {
    pub kind: GeneralSemPlsHocBootstrapTargetKindV1,
    pub target_version: String,
    pub target_id: String,
    pub relation_id: String,
    pub parameter_id: String,
    pub source_id: String,
    pub target_variable_id: String,
    pub point_method_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocBootstrapTargetInferenceV1 {
    pub target: GeneralSemPlsHocBootstrapTargetIdentityV1,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemPlsHocBootstrapFailureCodeV1 {
    InsufficientObservations,
    ConstantIndicator,
    StageOneRankDeficient,
    IsolatedConstruct,
    StageOneNonconvergence,
    IndeterminateScoreSign,
    ConstantComponentScore,
    StageTwoRankDeficient,
    StageTwoNonconvergence,
    ComponentCollinearity,
    NumericalFailure,
}

impl GeneralSemPlsHocBootstrapFailureCodeV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsufficientObservations => "insufficient_observations",
            Self::ConstantIndicator => "constant_indicator",
            Self::StageOneRankDeficient => "stage_one_rank_deficient",
            Self::IsolatedConstruct => "isolated_construct",
            Self::StageOneNonconvergence => "stage_one_nonconvergence",
            Self::IndeterminateScoreSign => "indeterminate_score_sign",
            Self::ConstantComponentScore => "constant_component_score",
            Self::StageTwoRankDeficient => "stage_two_rank_deficient",
            Self::StageTwoNonconvergence => "stage_two_nonconvergence",
            Self::ComponentCollinearity => "component_collinearity",
            Self::NumericalFailure => "numerical_failure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocBootstrapFailedReplicateV1 {
    pub replicate_index: u32,
    pub reason_code: GeneralSemPlsHocBootstrapFailureCodeV1,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsDisjointHocBootstrapResultV1 {
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
    pub target_version: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub hoc_stage_plan_sha256: String,
    pub model_scientific_sha256: String,
    pub stage_one_model_scientific_sha256: String,
    pub stage_two_model_scientific_sha256: String,
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
    /// Canonical decimal u64 wire form; exact in JavaScript runtimes.
    pub seed: String,
    pub workers: u32,
    pub complete_model_reestimated_per_replicate: bool,
    pub stage_one_reestimated_per_replicate: bool,
    pub generated_component_values_recalculated_per_replicate: bool,
    pub stage_one_scores_sign_aligned_per_replicate: bool,
    pub stage_two_reestimated_per_replicate: bool,
    pub stage_two_scores_sign_aligned_per_replicate: bool,
    pub complete_point_contract_validated_per_replicate: bool,
    pub failed_replicates: Vec<GeneralSemPlsHocBootstrapFailedReplicateV1>,
    pub targets: Vec<GeneralSemPlsHocBootstrapTargetInferenceV1>,
}

impl GeneralSemPlsDisjointHocBootstrapResultV1 {
    pub fn ensure_valid(&self) -> Result<(), GeneralSemPlsDisjointHocBootstrapErrorV1> {
        fn invalid(message: impl Into<String>) -> GeneralSemPlsDisjointHocBootstrapErrorV1 {
            GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidResultContract(message.into())
        }
        if self.schema_version != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1
            || self.method_version != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1
            || self.point_method_version
                != crate::GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1
            || self.resampling_operation_version
                != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1
            || self.resampling_stream_version != GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1
            || self.quantile_method_version != GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1
            || self.standard_error_method_version
                != GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1
            || self.summation_method_version != GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1
            || self.p_value_method_version != GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1
            || self.failure_policy_version != GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1
            || self.sign_alignment_method_version
                != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_SIGN_ALIGNMENT_VERSION_V1
            || self.target_version != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1
        {
            return Err(invalid(
                "schema or algorithm version is not the exact HOC bootstrap v1 contract",
            ));
        }
        for (name, digest) in [
            ("general_sem_config_sha256", &self.general_sem_config_sha256),
            ("compiled_plan_sha256", &self.compiled_plan_sha256),
            ("hoc_stage_plan_sha256", &self.hoc_stage_plan_sha256),
            ("model_scientific_sha256", &self.model_scientific_sha256),
            (
                "stage_one_model_scientific_sha256",
                &self.stage_one_model_scientific_sha256,
            ),
            (
                "stage_two_model_scientific_sha256",
                &self.stage_two_model_scientific_sha256,
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
                "target_identity_set_sha256",
                &self.target_identity_set_sha256,
            ),
        ] {
            if !is_lowercase_sha256_v1(digest) {
                return Err(invalid(format!("{name} must be a lowercase SHA-256")));
            }
        }
        if !is_dataset_fingerprint_v1(&self.source_dataset_fingerprint) {
            return Err(invalid(
                "source_dataset_fingerprint must be a bare or v2-prefixed SHA-256",
            ));
        }
        if self.target_ids.is_empty()
            || self
                .target_ids
                .iter()
                .any(|target_id| target_id.trim().is_empty())
            || self.target_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid(
                "target_ids must be nonempty, unique, and strictly ordered",
            ));
        }
        if self.interval != GeneralSemBootstrapIntervalV1::Percentile
            || self.tail != GeneralSemInferenceTailV1::TwoSided
            || !self.confidence_level.is_finite()
            || !(0.0..1.0).contains(&self.confidence_level)
        {
            return Err(invalid(
                "interval, tail, or confidence level is outside the exact v1 contract",
            ));
        }
        let minimum = minimum_usable_replicates_v1(self.resamples_requested) as u32;
        if !(2..=10_000).contains(&self.resamples_requested)
            || self.minimum_usable_resamples != minimum
            || self.resamples_usable < minimum
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
        if !(1..=64).contains(&self.workers)
            || !self.complete_model_reestimated_per_replicate
            || !self.stage_one_reestimated_per_replicate
            || !self.generated_component_values_recalculated_per_replicate
            || !self.stage_one_scores_sign_aligned_per_replicate
            || !self.stage_two_reestimated_per_replicate
            || !self.stage_two_scores_sign_aligned_per_replicate
            || !self.complete_point_contract_validated_per_replicate
        {
            return Err(invalid("full two-stage refit flags are not all asserted"));
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
            || sha256_serialized(&usable_indices) != self.usable_replicate_indices_sha256
        {
            return Err(invalid(
                "usable replicate index digest contradicts the failure ledger",
            ));
        }
        let identities = self
            .targets
            .iter()
            .map(|inference| inference.target.clone())
            .collect::<Vec<_>>();
        let row_ids = identities
            .iter()
            .map(|identity| identity.target_id.clone())
            .collect::<Vec<_>>();
        if row_ids != self.target_ids
            || sha256_serialized(&identities) != self.target_identity_set_sha256
        {
            return Err(invalid(
                "target rows or identity digest differ from the canonical target inventory",
            ));
        }
        for inference in &self.targets {
            let target = &inference.target;
            if target.target_version != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1
                || target.point_method_version
                    != crate::GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1
                || target.target_id != target.relation_id
                || [
                    target.target_id.as_str(),
                    target.relation_id.as_str(),
                    target.parameter_id.as_str(),
                    target.source_id.as_str(),
                    target.target_variable_id.as_str(),
                ]
                .iter()
                .any(|value| value.trim().is_empty())
                || [
                    inference.original,
                    inference.bootstrap_mean,
                    inference.bootstrap_bias,
                    inference.standard_error,
                    inference.lower,
                    inference.upper,
                    inference.p_value_two_sided,
                ]
                .iter()
                .any(|value| !value.is_finite())
                || inference.standard_error < 0.0
                || inference.lower > inference.upper
                || inference.usable_replicates != self.resamples_usable
                || inference.two_sided_exceedances > inference.usable_replicates
                || !approximately_equal_v1(
                    inference.bootstrap_bias,
                    inference.bootstrap_mean - inference.original,
                )
                || !approximately_equal_v1(
                    inference.p_value_two_sided,
                    f64::from(inference.two_sided_exceedances + 1)
                        / f64::from(inference.usable_replicates + 1),
                )
            {
                return Err(invalid(format!(
                    "target {} violates the exact inference contract",
                    target.target_id
                )));
            }
        }
        Ok(())
    }

    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
        original_point: &GeneralSemPlsHigherOrderPointResultV1,
    ) -> Result<(), GeneralSemPlsDisjointHocBootstrapErrorV1> {
        self.ensure_valid()?;
        original_point
            .ensure_valid_against_plan_v1(plan)
            .map_err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalPoint)?;
        let [hoc] = plan.higher_order_stage_plans() else {
            return Err(invalid_contract(
                "compiled plan must contain exactly one HOC",
            ));
        };
        if self.general_sem_config_sha256 != plan.general_sem_config_sha256()
            || self.compiled_plan_sha256 != plan.deterministic_sha256()
            || self.hoc_stage_plan_sha256 != sha256_serialized(hoc)
            || self.model_scientific_sha256 != plan.scientific_hash()
            || self.stage_one_model_scientific_sha256 != plan.base_plan().scientific_hash()
            || self.stage_two_model_scientific_sha256
                != original_point
                    .stages()
                    .last()
                    .expect("validated point results always contain a final HOC stage")
                    .receipt()
                    .model_scientific_sha256()
            || self.source_dataset_fingerprint
                != original_point.stages()[0].receipt().dataset_fingerprint()
        {
            return Err(invalid_contract(
                "bootstrap provenance differs from the compiled HOC plan",
            ));
        }
        let expected_identities = hoc_bootstrap_target_identities_v1(plan)?;
        let actual_identities = self
            .targets
            .iter()
            .map(|inference| inference.target.clone())
            .collect::<Vec<_>>();
        if actual_identities != expected_identities {
            return Err(invalid_contract(
                "bootstrap target identities differ from the compiled HOC inventory",
            ));
        }
        let original_values = hoc_bootstrap_target_values_v1(original_point, &expected_identities)?;
        for inference in &self.targets {
            if original_values
                .get(&inference.target.target_id)
                .is_none_or(|original| original.to_bits() != inference.original.to_bits())
            {
                return Err(invalid_contract(format!(
                    "target {} original estimate differs from the validated point result",
                    inference.target.target_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemPlsDisjointHocBootstrapErrorV1 {
    #[error(transparent)]
    InvalidConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error("higher-order bootstrap requires raw observations")]
    RawDataRequired,
    #[error("higher-order bootstrap requires exactly one bounded HOC")]
    DisjointTwoStageRequired,
    #[error("higher-order bootstrap does not support an interaction plan")]
    InteractionPlanNotSupported,
    #[error("higher-order bootstrap requires case-bootstrap inference")]
    CaseBootstrapRequired,
    #[error("higher-order bootstrap v1 requires percentile intervals")]
    UnsupportedInterval,
    #[error("higher-order bootstrap v1 requires two-sided inference")]
    UnsupportedTail,
    #[error("recipe General SEM configuration differs from the requested bootstrap configuration")]
    RecipeConfigMismatch,
    #[error("compiled HOC plan configuration differs from the requested bootstrap configuration")]
    CompiledPlanConfigMismatch,
    #[error("compiled HOC plan is not bound to the execution dataset")]
    DatasetBindingMismatch,
    #[error("bootstrap worker count must be between 1 and 64")]
    InvalidWorkerCount,
    #[error("stage-one recipe projection failed: {0}")]
    StageOneRecipe(GeneralSemPlsRecipeCompilationErrorV1),
    #[error("stage-one recipe compilation failed: {0}")]
    StageOneCompilation(RecipeV4CompilationError),
    #[error("compiled stage-one plan differs from the HOC lower-order plan")]
    StageOnePlanMismatch,
    #[error("original stage-one refit failed: {0}")]
    OriginalStageOneRefit(RecipeV4PlsExecutionError),
    #[error("original stage-one estimate differs from an independent deterministic refit")]
    OriginalStageOneMismatch,
    #[error("original stage-one construct-score domain differs from the complete-case frame")]
    OriginalStageOneScoreDomainMismatch,
    #[error("original HOC point result is invalid: {0}")]
    OriginalPoint(GeneralSemPlsHigherOrderPointErrorV1),
    #[error("original HOC point refit failed: {0}")]
    OriginalPointRefit(GeneralSemPlsHigherOrderPointErrorV1),
    #[error("original HOC point result differs from an independent deterministic two-stage refit")]
    OriginalPointMismatch,
    #[error("complete-case frame construction failed: {0}")]
    CompleteCaseFrame(GeneralSemPlsDisjointHocScoreDatasetErrorV1),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
    #[error(
        "only {usable} of {requested} HOC bootstrap replicates were usable; at least {minimum} are required"
    )]
    InsufficientUsableReplicates {
        requested: u32,
        usable: u32,
        minimum: u32,
        failed_replicates: Vec<GeneralSemPlsHocBootstrapFailedReplicateV1>,
    },
    #[error("higher-order bootstrap summary is invalid: {0}")]
    InvalidSummary(String),
    #[error("higher-order bootstrap result contract is invalid: {0}")]
    InvalidResultContract(String),
}

#[derive(Debug)]
enum HocBootstrapReplicateRecordV1 {
    Usable {
        target_values: BTreeMap<String, f64>,
    },
    Failed {
        reason_code: GeneralSemPlsHocBootstrapFailureCodeV1,
        message: String,
    },
    Fatal {
        message: String,
    },
    Cancelled,
}

#[derive(Debug, Serialize)]
struct HocCompleteCaseFrameIdentityV1<'a> {
    dataset_id: String,
    dataset_fingerprint: &'a str,
    source_columns: &'a [String],
    raw_row_indices: Vec<u64>,
}

#[allow(clippy::too_many_arguments)]
pub fn bootstrap_general_sem_pls_higher_order_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
    original_stage_one: &RecipeV4PlsExecutionResultV1,
    original_point: &GeneralSemPlsHigherOrderPointResultV1,
    config: &GeneralSemConfigV1,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<GeneralSemPlsDisjointHocBootstrapResultV1, GeneralSemPlsDisjointHocBootstrapErrorV1> {
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }
    config.ensure_valid()?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::RawDataRequired);
    }
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::DisjointTwoStageRequired);
    };
    if !plan.two_way_interactions().is_empty() {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InteractionPlanNotSupported);
    }
    let GeneralSemInferenceV1::CaseBootstrap {
        resamples,
        seed,
        confidence_level,
        interval,
        tail,
    } = config.inference
    else {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::CaseBootstrapRequired);
    };
    if interval != GeneralSemBootstrapIntervalV1::Percentile {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::UnsupportedInterval);
    }
    if tail != GeneralSemInferenceTailV1::TwoSided {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::UnsupportedTail);
    }
    if recipe.general_sem_config.as_ref() != Some(config) {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::RecipeConfigMismatch);
    }
    if plan.general_sem_config_sha256() != sha256_serialized(config).as_str() {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::CompiledPlanConfigMismatch);
    }
    let dataset_id = dataset.id.to_string();
    if plan.base_plan().dataset_id() != dataset_id.as_str()
        || recipe.dataset_fingerprint.as_str() != dataset.fingerprint.0.as_str()
        || plan.scientific_hash() != plan.topology().model_scientific_sha256()
    {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::DatasetBindingMismatch);
    }
    if !(1..=64).contains(&workers) {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidWorkerCount);
    }

    let (stage_one_recipe, stage_one_model) =
        project_general_sem_pls_stage_one_recipe_v1(recipe, resolved_model)
            .map_err(GeneralSemPlsDisjointHocBootstrapErrorV1::StageOneRecipe)?;
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let stage_one_artifact = compile_analysis_recipe_v4(
        &stage_one_recipe,
        Some(&stage_one_model),
        target,
        target.capability_cell_for_method(recipe.settings.method),
    )
    .map_err(GeneralSemPlsDisjointHocBootstrapErrorV1::StageOneCompilation)?;
    let CompiledRecipePlanV4::PlsPlanV2 {
        plan: compiled_stage_one,
    } = stage_one_artifact.plan()
    else {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::StageOnePlanMismatch);
    };
    if compiled_stage_one != plan.base_plan() {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::StageOnePlanMismatch);
    }
    let independently_refitted_stage_one = run_compiled_pls_recipe_v4_allowing_isolated(
        dataset,
        &stage_one_recipe,
        &stage_one_model,
        &stage_one_artifact,
        &is_cancelled,
        |_| {},
    )
    .map_err(|error| match error {
        RecipeV4PlsExecutionError::Cancelled => ResamplingError::Cancelled.into(),
        other => GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalStageOneRefit(other),
    })?;
    if independently_refitted_stage_one.estimation() != original_stage_one.estimation() {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalStageOneMismatch);
    }

    original_point
        .ensure_valid_against_plan_v1(plan)
        .map_err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalPoint)?;
    let point_context = compile_general_sem_pls_hoc_point_context_v1(recipe, resolved_model, plan)
        .map_err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalPointRefit)?;
    let independently_refitted_point =
        run_compiled_general_sem_pls_higher_order_point_with_context_v1(
            dataset,
            plan,
            &independently_refitted_stage_one,
            &point_context,
            None,
            &is_cancelled,
            |_| {},
        )
        .map_err(|error| match error {
            GeneralSemPlsHigherOrderPointErrorV1::Cancelled => ResamplingError::Cancelled.into(),
            other => GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalPointRefit(other),
        })?;
    if independently_refitted_point.result() != original_point {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalPointMismatch);
    }

    let source_columns = plan
        .base_plan()
        .blocks()
        .iter()
        .flat_map(|block| block.indicators())
        .map(|indicator| indicator.source_column().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let complete_rows =
        general_sem_pls_hoc_complete_case_rows_v1(dataset, plan.base_plan(), || !is_cancelled())
            .map_err(|error| match error {
                GeneralSemPlsDisjointHocScoreDatasetErrorV1::Cancelled => {
                    GeneralSemPlsDisjointHocBootstrapErrorV1::Resampling(ResamplingError::Cancelled)
                }
                other => GeneralSemPlsDisjointHocBootstrapErrorV1::CompleteCaseFrame(other),
            })?;
    let expected_stage_one_ids = plan
        .base_plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id())
        .collect::<BTreeSet<_>>();
    let original_stage_one_ids = original_stage_one
        .estimation()
        .construct_scores
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if original_stage_one.estimation().used_observations != complete_rows.len()
        || original_stage_one_ids != expected_stage_one_ids
        || original_stage_one
            .estimation()
            .construct_scores
            .values()
            .any(|scores| scores.len() != complete_rows.len())
        || independently_refitted_point
            .stage_construct_scores()
            .iter()
            .flat_map(|scores| scores.values())
            .any(|scores| scores.len() != complete_rows.len())
    {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::OriginalStageOneScoreDomainMismatch);
    }
    let target_identities = hoc_bootstrap_target_identities_v1(plan)?;
    let target_ids = target_identities
        .iter()
        .map(|identity| identity.target_id.clone())
        .collect::<Vec<_>>();
    let original_values = hoc_bootstrap_target_values_v1(original_point, &target_identities)?;
    let generated_score_ids = hoc
        .component_mappings()
        .iter()
        .map(|mapping| mapping.generated_score_variable_id().to_string())
        .collect::<BTreeSet<_>>();
    let original_stage_scores = independently_refitted_point.stage_construct_scores();
    let bootstrap_plan = BootstrapPlan {
        replicates: resamples,
        master_seed: seed,
        operation: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1.into(),
    };
    let cancellation = &is_cancelled;
    let run = run_bootstrap(
        complete_rows.len(),
        &bootstrap_plan,
        workers,
        |_replicate_index, sampled_positions| {
            let record = (|| {
                let raw_indices = sampled_positions
                    .iter()
                    .map(|position| complete_rows[*position])
                    .collect::<Vec<_>>();
                let sampled =
                    resample_dataset_columns_v1(dataset, &source_columns, &raw_indices, || {
                        cancellation()
                    })
                    .map_err(classify_resampling_error_v1)?;
                let mut stage_one = run_compiled_pls_recipe_v4_allowing_isolated(
                    &sampled,
                    &stage_one_recipe,
                    &stage_one_model,
                    &stage_one_artifact,
                    cancellation,
                    |_| {},
                )
                .map_err(classify_stage_one_error_v1)?;
                align_general_sem_pls_hoc_result_signs_v1(
                    stage_one.estimation_mut(),
                    GeneralSemPlsHocScoreAlignmentReferenceV1::new(
                        &original_stage_one.estimation().construct_scores,
                        sampled_positions,
                    ),
                    cancellation,
                )
                .map_err(classify_score_alignment_error_v1)?;
                let point = run_compiled_general_sem_pls_higher_order_point_with_context_v1(
                    &sampled,
                    plan,
                    &stage_one,
                    &point_context,
                    Some(GeneralSemPlsHocStageAlignmentReferencesV1::new(
                        original_stage_scores,
                        sampled_positions,
                    )),
                    cancellation,
                    |_| {},
                )
                .map_err(|error| classify_stage_two_error_v1(error, &generated_score_ids))?;
                hoc_bootstrap_target_values_v1(point.result(), &target_identities).map_err(
                    |error| HocBootstrapReplicateRecordV1::Fatal {
                        message: error.to_string(),
                    },
                )
            })();
            Ok::<_, Infallible>(match record {
                Ok(target_values) => HocBootstrapReplicateRecordV1::Usable { target_values },
                Err(record) => record,
            })
        },
        cancellation,
        report_progress,
    )?;
    if cancellation() {
        return Err(ResamplingError::Cancelled.into());
    }

    let mut usable_indices = Vec::new();
    let mut replicate_values = Vec::new();
    let mut failed_replicates = Vec::new();
    for (replicate_index, outcome) in run.outcomes.into_iter().enumerate() {
        if cancellation() {
            return Err(ResamplingError::Cancelled.into());
        }
        let ReplicateOutcome::Success { value } = outcome else {
            return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(
                "infallible indexed scheduler returned a failed outer outcome".into(),
            ));
        };
        match value {
            HocBootstrapReplicateRecordV1::Usable { target_values } => {
                usable_indices.push(replicate_index as u32);
                replicate_values.push(target_values);
            }
            HocBootstrapReplicateRecordV1::Failed {
                reason_code,
                message,
            } => failed_replicates.push(GeneralSemPlsHocBootstrapFailedReplicateV1 {
                replicate_index: replicate_index as u32,
                reason_code,
                message,
            }),
            HocBootstrapReplicateRecordV1::Fatal { message } => {
                return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(
                    format!("replicate {replicate_index} violated the target contract: {message}"),
                ));
            }
            HocBootstrapReplicateRecordV1::Cancelled => {
                return Err(ResamplingError::Cancelled.into());
            }
        }
    }
    let minimum_usable = minimum_usable_replicates_v1(resamples);
    if replicate_values.len() < minimum_usable {
        return Err(
            GeneralSemPlsDisjointHocBootstrapErrorV1::InsufficientUsableReplicates {
                requested: resamples,
                usable: replicate_values.len() as u32,
                minimum: minimum_usable as u32,
                failed_replicates,
            },
        );
    }
    let targets = target_identities
        .iter()
        .map(|identity| {
            if cancellation() {
                return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::Resampling(
                    ResamplingError::Cancelled,
                ));
            }
            let values = replicate_values
                .iter()
                .map(|replicate| {
                    replicate.get(&identity.target_id).copied().ok_or_else(|| {
                        GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(format!(
                            "usable replicate is missing target {}",
                            identity.target_id
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let original = original_values
                .get(&identity.target_id)
                .copied()
                .ok_or_else(|| {
                    GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(format!(
                        "validated point result is missing target {}",
                        identity.target_id
                    ))
                })?;
            summarize_target_v1(identity.clone(), original, values, confidence_level)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if cancellation() {
        return Err(ResamplingError::Cancelled.into());
    }
    let complete_case_frame_sha256 = sha256_serialized(&HocCompleteCaseFrameIdentityV1 {
        dataset_id,
        dataset_fingerprint: &dataset.fingerprint.0,
        source_columns: &source_columns,
        raw_row_indices: complete_rows.iter().map(|row| *row as u64).collect(),
    });
    let result = GeneralSemPlsDisjointHocBootstrapResultV1 {
        schema_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_RESULT_SCHEMA_VERSION_V1,
        method_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1.into(),
        point_method_version: crate::GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1
            .into(),
        resampling_operation_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1
            .into(),
        resampling_stream_version: GENERAL_SEM_PLS_BOOTSTRAP_STREAM_VERSION_V1.into(),
        quantile_method_version: GENERAL_SEM_PLS_BOOTSTRAP_QUANTILE_VERSION_V1.into(),
        standard_error_method_version: GENERAL_SEM_PLS_BOOTSTRAP_STANDARD_ERROR_VERSION_V1.into(),
        summation_method_version: GENERAL_SEM_PLS_BOOTSTRAP_SUMMATION_VERSION_V1.into(),
        p_value_method_version: GENERAL_SEM_PLS_BOOTSTRAP_P_VALUE_VERSION_V1.into(),
        failure_policy_version: GENERAL_SEM_PLS_BOOTSTRAP_FAILURE_POLICY_VERSION_V1.into(),
        sign_alignment_method_version:
            GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_SIGN_ALIGNMENT_VERSION_V1.into(),
        target_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1.into(),
        general_sem_config_sha256: plan.general_sem_config_sha256().into(),
        compiled_plan_sha256: plan.deterministic_sha256(),
        hoc_stage_plan_sha256: sha256_serialized(hoc),
        model_scientific_sha256: plan.scientific_hash().into(),
        stage_one_model_scientific_sha256: plan.base_plan().scientific_hash().into(),
        stage_two_model_scientific_sha256: point_context
            .final_projection()
            .projected_scientific_sha256()
            .into(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        complete_case_frame_sha256,
        usable_replicate_indices_sha256: sha256_serialized(&usable_indices),
        target_identity_set_sha256: sha256_serialized(&target_identities),
        target_ids,
        interval,
        tail,
        confidence_level,
        resamples_requested: resamples,
        resamples_usable: replicate_values.len() as u32,
        minimum_usable_resamples: minimum_usable as u32,
        seed: seed.to_string(),
        workers: workers as u32,
        complete_model_reestimated_per_replicate: true,
        stage_one_reestimated_per_replicate: true,
        generated_component_values_recalculated_per_replicate: true,
        stage_one_scores_sign_aligned_per_replicate: true,
        stage_two_reestimated_per_replicate: true,
        stage_two_scores_sign_aligned_per_replicate: true,
        complete_point_contract_validated_per_replicate: true,
        failed_replicates,
        targets,
    };
    result.ensure_valid_against_plan_v1(plan, original_point)?;
    Ok(result)
}

pub fn bootstrap_general_sem_pls_disjoint_higher_order_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
    original_stage_one: &RecipeV4PlsExecutionResultV1,
    original_point: &GeneralSemPlsHigherOrderPointResultV1,
    config: &GeneralSemConfigV1,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<GeneralSemPlsDisjointHocBootstrapResultV1, GeneralSemPlsDisjointHocBootstrapErrorV1> {
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::DisjointTwoStageRequired);
    };
    if hoc.approach() != &HigherOrderConstructionApproachV4::DisjointTwoStage {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::DisjointTwoStageRequired);
    }
    bootstrap_general_sem_pls_higher_order_v1(
        dataset,
        recipe,
        resolved_model,
        plan,
        original_stage_one,
        original_point,
        config,
        workers,
        is_cancelled,
        report_progress,
    )
}

fn hoc_bootstrap_target_identities_v1(
    plan: &CompiledPlsPlanV3,
) -> Result<Vec<GeneralSemPlsHocBootstrapTargetIdentityV1>, GeneralSemPlsDisjointHocBootstrapErrorV1>
{
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::DisjointTwoStageRequired);
    };
    let mut identities = hoc
        .component_mappings()
        .iter()
        .map(|mapping| GeneralSemPlsHocBootstrapTargetIdentityV1 {
            kind: match mapping.relation_interpretation() {
                CompiledPlsHocComponentRelationInterpretationV1::Loading => {
                    GeneralSemPlsHocBootstrapTargetKindV1::ComponentLoading
                }
                CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => {
                    GeneralSemPlsHocBootstrapTargetKindV1::ComponentWeight
                }
            },
            target_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1.into(),
            target_id: mapping.generated_component_relation_id().into(),
            relation_id: mapping.generated_component_relation_id().into(),
            parameter_id: mapping.generated_component_parameter_id().into(),
            source_id: mapping.component_relation_source_id().into(),
            target_variable_id: mapping.component_relation_target_id().into(),
            point_method_version:
                crate::GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1.into(),
        })
        .collect::<Vec<_>>();
    identities.extend(
        plan.topology()
            .structural_relations()
            .iter()
            .filter(|relation| {
                relation.role() == StructuralRelationRoleV4::Structural
                    && (relation.source() == hoc.output_variable_id()
                        || relation.target() == hoc.output_variable_id())
            })
            .map(|relation| GeneralSemPlsHocBootstrapTargetIdentityV1 {
                kind: GeneralSemPlsHocBootstrapTargetKindV1::HocStructuralPath,
                target_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1.into(),
                target_id: relation.relation_id().into(),
                relation_id: relation.relation_id().into(),
                parameter_id: relation.parameter_id().into(),
                source_id: relation.source().into(),
                target_variable_id: relation.target().into(),
                point_method_version:
                    crate::GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1.into(),
            }),
    );
    if hoc.approach() == &HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
        for authored_id in hoc
            .technical_paths()
            .iter()
            .map(|path| path.authored_antecedent_relation_id())
            .collect::<BTreeSet<_>>()
        {
            let relation = plan
                .topology()
                .structural_relations()
                .iter()
                .find(|relation| relation.relation_id() == authored_id)
                .ok_or_else(|| {
                    invalid_contract("extended repeated authored relation is absent from topology")
                })?;
            let target_id = extended_effect_identity_v1("total", hoc, authored_id);
            identities.push(GeneralSemPlsHocBootstrapTargetIdentityV1 {
                kind: GeneralSemPlsHocBootstrapTargetKindV1::ExtendedTotalEffect,
                target_version: GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_TARGET_VERSION_V1.into(),
                target_id: target_id.clone(),
                relation_id: target_id.clone(),
                parameter_id: format!("{target_id}:estimand"),
                source_id: relation.source().into(),
                target_variable_id: relation.target().into(),
                point_method_version: crate::GENERAL_SEM_PLS_HIGHER_ORDER_POINT_METHOD_VERSION_V1
                    .into(),
            });
        }
    }
    identities.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    if identities.is_empty()
        || identities
            .windows(2)
            .any(|pair| pair[0].target_id >= pair[1].target_id)
    {
        return Err(invalid_contract(
            "compiled HOC bootstrap target inventory is empty or colliding",
        ));
    }
    Ok(identities)
}

fn hoc_bootstrap_target_values_v1(
    point: &GeneralSemPlsHigherOrderPointResultV1,
    identities: &[GeneralSemPlsHocBootstrapTargetIdentityV1],
) -> Result<BTreeMap<String, f64>, GeneralSemPlsDisjointHocBootstrapErrorV1> {
    let rows = point
        .stages()
        .iter()
        .flat_map(|stage| stage.relation_estimates())
        .collect::<Vec<_>>();
    identities
        .iter()
        .map(|identity| {
            let expected_kind = match identity.kind {
                GeneralSemPlsHocBootstrapTargetKindV1::ComponentLoading => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentLoading
                }
                GeneralSemPlsHocBootstrapTargetKindV1::ComponentWeight => {
                    GeneralSemPlsHocPointRelationKindV1::ComponentWeight
                }
                GeneralSemPlsHocBootstrapTargetKindV1::HocStructuralPath => {
                    GeneralSemPlsHocPointRelationKindV1::AuthoredStructural
                }
                GeneralSemPlsHocBootstrapTargetKindV1::ExtendedTotalEffect => {
                    GeneralSemPlsHocPointRelationKindV1::ExtendedTotalEffect
                }
            };
            let matches = rows
                .iter()
                .filter(|row| {
                    row.relation_id() == identity.relation_id.as_str()
                        && row.parameter_id() == identity.parameter_id.as_str()
                        && row.source_id() == identity.source_id.as_str()
                        && row.target_id() == identity.target_variable_id.as_str()
                        && row.kind() == expected_kind
                })
                .collect::<Vec<_>>();
            let [row] = matches.as_slice() else {
                return Err(invalid_contract(format!(
                    "point result does not contain exactly one target row for {}",
                    identity.target_id
                )));
            };
            if !row.estimate().is_finite() {
                return Err(invalid_contract(format!(
                    "point result target {} is non-finite",
                    identity.target_id
                )));
            }
            Ok((identity.target_id.clone(), row.estimate()))
        })
        .collect()
}

fn classify_resampling_error_v1(error: EstimationError) -> HocBootstrapReplicateRecordV1 {
    match error {
        EstimationError::Cancelled => HocBootstrapReplicateRecordV1::Cancelled,
        EstimationError::Numerical(message) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::NumericalFailure,
            message,
        ),
        other => HocBootstrapReplicateRecordV1::Fatal {
            message: other.to_string(),
        },
    }
}

fn classify_stage_one_error_v1(error: RecipeV4PlsExecutionError) -> HocBootstrapReplicateRecordV1 {
    match error {
        RecipeV4PlsExecutionError::Cancelled
        | RecipeV4PlsExecutionError::Estimation(EstimationError::Cancelled) => {
            HocBootstrapReplicateRecordV1::Cancelled
        }
        RecipeV4PlsExecutionError::Estimation(EstimationError::InsufficientObservations) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::InsufficientObservations,
                EstimationError::InsufficientObservations.to_string(),
            )
        }
        RecipeV4PlsExecutionError::Estimation(error @ EstimationError::ConstantIndicator(_)) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::ConstantIndicator,
                error.to_string(),
            )
        }
        RecipeV4PlsExecutionError::Estimation(error @ EstimationError::RankDeficient(_)) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::StageOneRankDeficient,
                error.to_string(),
            )
        }
        RecipeV4PlsExecutionError::Estimation(error @ EstimationError::IsolatedConstruct(_)) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::IsolatedConstruct,
                error.to_string(),
            )
        }
        RecipeV4PlsExecutionError::Estimation(error @ EstimationError::NonConvergence(_)) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::StageOneNonconvergence,
                error.to_string(),
            )
        }
        RecipeV4PlsExecutionError::Estimation(error @ EstimationError::Numerical(_)) => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::NumericalFailure,
                error.to_string(),
            )
        }
        other => HocBootstrapReplicateRecordV1::Fatal {
            message: other.to_string(),
        },
    }
}

fn classify_score_alignment_error_v1(
    error: GeneralSemPlsHocScoreAlignmentErrorV1,
) -> HocBootstrapReplicateRecordV1 {
    match error {
        GeneralSemPlsHocScoreAlignmentErrorV1::Cancelled => {
            HocBootstrapReplicateRecordV1::Cancelled
        }
        error @ GeneralSemPlsHocScoreAlignmentErrorV1::IndeterminateSign { .. } => {
            failed_record_v1(
                GeneralSemPlsHocBootstrapFailureCodeV1::IndeterminateScoreSign,
                error.to_string(),
            )
        }
        other => HocBootstrapReplicateRecordV1::Fatal {
            message: other.to_string(),
        },
    }
}

fn classify_stage_two_error_v1(
    error: GeneralSemPlsHigherOrderPointErrorV1,
    generated_score_ids: &BTreeSet<String>,
) -> HocBootstrapReplicateRecordV1 {
    match error {
        GeneralSemPlsHigherOrderPointErrorV1::Cancelled => HocBootstrapReplicateRecordV1::Cancelled,
        GeneralSemPlsHigherOrderPointErrorV1::ScoreDataset(
            error @ GeneralSemPlsDisjointHocScoreDatasetErrorV1::InsufficientObservations,
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::InsufficientObservations,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(EstimationError::ConstantIndicator(id)),
        ) if generated_score_ids.contains(&id) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::ConstantComponentScore,
            EstimationError::ConstantIndicator(id).to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(error @ EstimationError::ConstantIndicator(_)),
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::ConstantIndicator,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(error @ EstimationError::RankDeficient(_)),
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::StageTwoRankDeficient,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(error @ EstimationError::IsolatedConstruct(_)),
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::IsolatedConstruct,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(error @ EstimationError::NonConvergence(_)),
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::StageTwoNonconvergence,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::PointEstimation(
            RecipeV4PlsExecutionError::Estimation(error @ EstimationError::Numerical(_)),
        ) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::NumericalFailure,
            error.to_string(),
        ),
        GeneralSemPlsHigherOrderPointErrorV1::StageTwoScoreAlignment(error) => {
            classify_score_alignment_error_v1(error)
        }
        error @ (GeneralSemPlsHigherOrderPointErrorV1::ComponentCollinearityUndefined {
            ..
        }
        | GeneralSemPlsHigherOrderPointErrorV1::ComponentCollinearity { .. }) => failed_record_v1(
            GeneralSemPlsHocBootstrapFailureCodeV1::ComponentCollinearity,
            error.to_string(),
        ),
        other => HocBootstrapReplicateRecordV1::Fatal {
            message: other.to_string(),
        },
    }
}

fn failed_record_v1(
    reason_code: GeneralSemPlsHocBootstrapFailureCodeV1,
    message: String,
) -> HocBootstrapReplicateRecordV1 {
    HocBootstrapReplicateRecordV1::Failed {
        reason_code,
        message,
    }
}

fn summarize_target_v1(
    target: GeneralSemPlsHocBootstrapTargetIdentityV1,
    original: f64,
    mut values: Vec<f64>,
    confidence_level: f64,
) -> Result<GeneralSemPlsHocBootstrapTargetInferenceV1, GeneralSemPlsDisjointHocBootstrapErrorV1> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) || !original.is_finite() {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(
            format!(
                "target {} has fewer than two finite values",
                target.target_id
            ),
        ));
    }
    let mean = stable_sum_v1(&values) / values.len() as f64;
    let squared_deviations = values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .collect::<Vec<_>>();
    let standard_error = (stable_sum_v1(&squared_deviations) / (values.len() - 1) as f64).sqrt();
    values.sort_by(|left, right| left.total_cmp(right));
    let alpha = 1.0 - confidence_level;
    let lower = type7_quantile_v1(&values, alpha / 2.0);
    let upper = type7_quantile_v1(&values, 1.0 - alpha / 2.0);
    let exceedances = values
        .iter()
        .filter(|value| (**value - original).abs() >= original.abs())
        .count();
    let inference = GeneralSemPlsHocBootstrapTargetInferenceV1 {
        target,
        original: canonical_zero_v1(original),
        bootstrap_mean: canonical_zero_v1(mean),
        bootstrap_bias: canonical_zero_v1(mean - original),
        standard_error: canonical_zero_v1(standard_error),
        lower: canonical_zero_v1(lower),
        upper: canonical_zero_v1(upper),
        p_value_two_sided: (exceedances + 1) as f64 / (values.len() + 1) as f64,
        usable_replicates: values.len() as u32,
        two_sided_exceedances: exceedances as u32,
    };
    if [
        inference.bootstrap_mean,
        inference.bootstrap_bias,
        inference.standard_error,
        inference.lower,
        inference.upper,
        inference.p_value_two_sided,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidSummary(
            format!(
                "target {} produced a non-finite summary",
                inference.target.target_id
            ),
        ));
    }
    Ok(inference)
}

fn type7_quantile_v1(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] + (position - lower as f64) * (sorted[upper] - sorted[lower])
    }
}

fn stable_sum_v1(values: &[f64]) -> f64 {
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

fn minimum_usable_replicates_v1(requested: u32) -> usize {
    ((f64::from(requested) * 0.9).ceil() as usize).max(2)
}

fn canonical_zero_v1(value: f64) -> f64 {
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

fn invalid_contract(message: impl Into<String>) -> GeneralSemPlsDisjointHocBootstrapErrorV1 {
    GeneralSemPlsDisjointHocBootstrapErrorV1::InvalidResultContract(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_v4_general_sem_hoc_point_execution::tests::{fixture, fixture_for};
    use qpls_core::{
        GeneralSemBootstrapIntervalV1, GeneralSemInferenceTailV1,
        HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4, compile_pls_plan_v3,
    };

    fn bootstrap_fixture(
        measurement_type: HigherOrderMeasurementTypeV4,
        workers: usize,
        resamples: u32,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledPlsPlanV3,
        RecipeV4PlsExecutionResultV1,
        GeneralSemPlsHigherOrderPointResultV1,
        GeneralSemConfigV1,
        usize,
    ) {
        let (dataset, recipe, model, stage_one, point) = fixture(measurement_type);
        finish_bootstrap_fixture(dataset, recipe, model, stage_one, point, workers, resamples)
    }

    fn bootstrap_fixture_for(
        measurement_type: HigherOrderMeasurementTypeV4,
        approach: HigherOrderConstructionApproachV4,
        endogenous: bool,
        workers: usize,
        resamples: u32,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledPlsPlanV3,
        RecipeV4PlsExecutionResultV1,
        GeneralSemPlsHigherOrderPointResultV1,
        GeneralSemConfigV1,
        usize,
    ) {
        let (dataset, recipe, model, stage_one, point) =
            fixture_for(measurement_type, approach, endogenous);
        finish_bootstrap_fixture(dataset, recipe, model, stage_one, point, workers, resamples)
    }

    fn finish_bootstrap_fixture(
        dataset: Dataset,
        mut recipe: AnalysisRecipeV4,
        model: SemModelV4,
        stage_one: RecipeV4PlsExecutionResultV1,
        point: GeneralSemPlsHigherOrderPointResultV1,
        workers: usize,
        resamples: u32,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledPlsPlanV3,
        RecipeV4PlsExecutionResultV1,
        GeneralSemPlsHigherOrderPointResultV1,
        GeneralSemConfigV1,
        usize,
    ) {
        let config = GeneralSemConfigV1 {
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples,
                seed: 71,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            },
            ..GeneralSemConfigV1::default()
        };
        recipe.settings.bootstrap_samples = resamples;
        recipe.settings.seed = 71;
        recipe.settings.confidence_level = 0.95;
        recipe.general_sem_config = Some(config.clone());
        recipe.ensure_valid().unwrap();
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        (
            dataset, recipe, model, plan, stage_one, point, config, workers,
        )
    }

    fn execute(
        measurement_type: HigherOrderMeasurementTypeV4,
        workers: usize,
        resamples: u32,
    ) -> GeneralSemPlsDisjointHocBootstrapResultV1 {
        let (dataset, recipe, model, plan, stage_one, point, config, workers) =
            bootstrap_fixture(measurement_type, workers, resamples);
        bootstrap_general_sem_pls_disjoint_higher_order_v1(
            &dataset,
            &recipe,
            &model,
            &plan,
            &stage_one,
            &point,
            &config,
            workers,
            || false,
            |_| {},
        )
        .unwrap()
    }

    #[test]
    fn all_four_disjoint_hcm_types_have_one_shared_complete_target_ledger() {
        for measurement_type in [
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
            HigherOrderMeasurementTypeV4::FormativeReflective,
            HigherOrderMeasurementTypeV4::FormativeFormative,
        ] {
            let result = execute(measurement_type, 1, 2);
            assert_eq!(result.resamples_requested, 2);
            assert_eq!(result.resamples_usable, 2);
            assert_eq!(result.targets.len(), 4);
            assert_eq!(
                result
                    .targets
                    .iter()
                    .filter(|target| {
                        target.target.kind
                            == GeneralSemPlsHocBootstrapTargetKindV1::HocStructuralPath
                    })
                    .count(),
                2
            );
            assert!(result.failed_replicates.is_empty());
            assert!(result.complete_point_contract_validated_per_replicate);
        }
    }

    #[test]
    fn every_runtime_approach_refits_its_complete_stage_pipeline() {
        for (approach, measurement_type, endogenous) in [
            (
                HigherOrderConstructionApproachV4::RepeatedIndicators,
                HigherOrderMeasurementTypeV4::ReflectiveReflective,
                true,
            ),
            (
                HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
                HigherOrderMeasurementTypeV4::ReflectiveFormative,
                true,
            ),
            (
                HigherOrderConstructionApproachV4::EmbeddedTwoStage,
                HigherOrderMeasurementTypeV4::FormativeFormative,
                true,
            ),
        ] {
            let (dataset, recipe, model, plan, stage_one, point, config, workers) =
                bootstrap_fixture_for(measurement_type, approach.clone(), endogenous, 1, 2);
            let result = bootstrap_general_sem_pls_higher_order_v1(
                &dataset,
                &recipe,
                &model,
                &plan,
                &stage_one,
                &point,
                &config,
                workers,
                || false,
                |_| {},
            )
            .unwrap();
            assert_eq!(point.approach(), &approach);
            assert_eq!(result.resamples_usable, 2);
            assert!(result.complete_point_contract_validated_per_replicate);
            if approach == HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
                assert!(result.targets.iter().any(|target| {
                    target.target.kind == GeneralSemPlsHocBootstrapTargetKindV1::ExtendedTotalEffect
                }));
            }
        }
    }

    #[test]
    fn indexed_bootstrap_is_scientifically_identical_across_worker_counts() {
        let (dataset, recipe, model, plan, stage_one, point, config, _) =
            bootstrap_fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective, 1, 10);
        let run = |workers| {
            bootstrap_general_sem_pls_disjoint_higher_order_v1(
                &dataset,
                &recipe,
                &model,
                &plan,
                &stage_one,
                &point,
                &config,
                workers,
                || false,
                |_| {},
            )
            .unwrap()
        };
        let serial = run(1);
        let parallel = run(4);
        let mut normalized = parallel.clone();
        normalized.workers = serial.workers;
        assert_eq!(serial, normalized);
    }

    #[test]
    fn cancellation_publishes_no_bootstrap_result() {
        let (dataset, recipe, model, plan, stage_one, point, config, workers) =
            bootstrap_fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective, 1, 2);
        assert!(matches!(
            bootstrap_general_sem_pls_disjoint_higher_order_v1(
                &dataset,
                &recipe,
                &model,
                &plan,
                &stage_one,
                &point,
                &config,
                workers,
                || true,
                |_| {},
            ),
            Err(GeneralSemPlsDisjointHocBootstrapErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
    }

    #[test]
    fn target_identity_tampering_fails_closed() {
        let (dataset, recipe, model, plan, stage_one, point, config, workers) =
            bootstrap_fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective, 1, 2);
        let mut result = bootstrap_general_sem_pls_disjoint_higher_order_v1(
            &dataset,
            &recipe,
            &model,
            &plan,
            &stage_one,
            &point,
            &config,
            workers,
            || false,
            |_| {},
        )
        .unwrap();
        result.targets[0].target.parameter_id.push_str(":tampered");
        assert!(result.ensure_valid_against_plan_v1(&plan, &point).is_err());
    }
}
