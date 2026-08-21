use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeV4,
    CompiledAnalysisRecipeV4, CompiledPlsBlockModeV2, CompiledPlsFixedScoringV2, CompiledPlsPlanV2,
    CompiledRecipePlanV4, Construct, ControlPath, ExecutionRecipeError, MeasurementMode,
    MethodConfig, ModelSpec, PlsAlgorithmConfigV2, PlsInitialOuterWeightV2,
    PlsInitialOuterWeightsV2, PlsPosthocTechnicalMinimumSampleSizeConfigV2,
    RecipeV4CompilationError, RecipeV4CompilationReceipt, RecipeV4CompilerTarget, SemModelV4,
    SemParameterV4, SemVariableV4, StructuralPath, StructuralRelationRoleV4,
    ValidatedExecutionRecipe, WeightingScheme, sha256_serialized,
    validate_compiled_analysis_recipe_v4,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, NONLINEAR_EFFECTS_METHOD_VERSION,
    PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1,
    PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1, PLS_METHOD_VERSION,
    PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2, PLS_SCORE_EXECUTION_METHOD_VERSION_V2,
    PlsAlgorithmInitializationV1, PlsAlgorithmTerminationReasonV1, PlsAlgorithmUpdateRuleV1,
    PlsConvergenceComparisonV1, PlsPointEstimateAttributionV1, PlsResolvedScoreBlockKindV2,
    PlsResult, estimate_pls_validated_with_compiled_plan_v2_with_control,
    estimate_pls_validated_with_control, pls_posthoc_minimum_sample_size_v2,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::RunnerProgress;

pub const RECIPE_V4_PLS_EXECUTION_RESULT_SCHEMA_VERSION: u32 = 1;
pub const LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v3";
pub const LEGACY_RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v4";
pub const RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v5";
pub const RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v6";
pub const RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v7";

/// Deterministic scientific provenance for the opt-in recipe-v4 execution
/// adapter. Wall-clock timestamps and generated run identifiers belong to a
/// future persistence boundary and are intentionally absent here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4PlsExecutionProvenanceV1 {
    adapter_version: String,
    compilation_receipt: RecipeV4CompilationReceipt,
    projected_recipe_schema_version: u32,
    projected_recipe_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    projected_initialization_sha256: Option<String>,
    dataset_id: String,
    estimator_method_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    posthoc_technical_minimum_sample_size: Option<PlsPosthocTechnicalMinimumSampleSizeConfigV2>,
}

impl RecipeV4PlsExecutionProvenanceV1 {
    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn compilation_receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.compilation_receipt
    }

    pub fn projected_recipe_schema_version(&self) -> u32 {
        self.projected_recipe_schema_version
    }

    pub fn projected_recipe_sha256(&self) -> &str {
        &self.projected_recipe_sha256
    }

    pub fn projected_initialization_sha256(&self) -> Option<&str> {
        self.projected_initialization_sha256.as_deref()
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn estimator_method_version(&self) -> &str {
        &self.estimator_method_version
    }

    pub fn posthoc_technical_minimum_sample_size(
        &self,
    ) -> Option<&PlsPosthocTechnicalMinimumSampleSizeConfigV2> {
        self.posthoc_technical_minimum_sample_size.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4PlsExecutionResultV1 {
    schema_version: u32,
    provenance: RecipeV4PlsExecutionProvenanceV1,
    estimation: PlsResult,
}

impl RecipeV4PlsExecutionResultV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn provenance(&self) -> &RecipeV4PlsExecutionProvenanceV1 {
        &self.provenance
    }

    pub fn estimation(&self) -> &PlsResult {
        &self.estimation
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeV4PlsExecutionError {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error(transparent)]
    Compilation(#[from] RecipeV4CompilationError),
    #[error("recipe-v4 PLS execution requires the compiled PLS plan target (found {0:?})")]
    CompilerTarget(RecipeV4CompilerTarget),
    #[error("recipe-v4 PLS execution requires raw observations")]
    RawDataRequired,
    #[error("execution dataset fingerprint does not match the compilation receipt")]
    DatasetFingerprintMismatch,
    #[error("execution dataset id {actual} does not match compiled plan dataset {expected}")]
    DatasetIdMismatch { expected: String, actual: String },
    #[error("compiled PLS projection references unknown SemModelV4 composite {0}")]
    UnknownComposite(String),
    #[error("compiled PLS projection references unknown SemModelV4 path parameter {0}")]
    UnknownPathParameter(String),
    #[error("compiled PLS initialization references unknown or ineligible stable identity {0}:{1}")]
    UnknownInitialWeightIdentity(String, String),
    #[error("compiled PLS initialization translation is not an exact bijection")]
    InitialWeightTranslationMismatch,
    #[error("compiled PLS model identifier cannot be projected deterministically")]
    InvalidModelIdentity,
    #[error(transparent)]
    Projection(#[from] ExecutionRecipeError),
    #[error("PLS estimation failed: {0}")]
    Estimation(EstimationError),
    #[error("production PLS estimator version changed from {expected} to {actual}")]
    EstimatorVersionMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("production PLS point-estimate scale attribution is missing or ambiguous")]
    PointEstimateAttributionMismatch,
    #[error("fixed-score scale receipt differs from the compiled fixed blocks")]
    FixedScoreScaleReceiptMismatch,
    #[error("production PLS algorithm convergence receipt is missing or inconsistent")]
    AlgorithmConvergenceReceiptMismatch,
    #[error("production PLS nonlinear payload is missing or has a drifted exact method identity")]
    NonlinearEffectsIdentityMismatch,
    #[error(
        "post-hoc technical minimum sample-size opt-in has a drifted capability or method identity"
    )]
    PosthocOptionIdentityMismatch,
    #[error(
        "post-hoc technical minimum sample-size capability is not executable in Capability Registry V2"
    )]
    PosthocCapabilityUnavailable,
    #[error(
        "Capability Registry V2 cannot authorize the post-hoc technical minimum sample-size option: {0}"
    )]
    PosthocCapabilityRegistry(String),
}

/// Executes the deliberately bounded, point-estimation-only PLS slice of a
/// recipe-v4 compilation artifact.
///
/// This function is an opt-in library boundary used by the internal Tauri
/// recipe-v4 command. Before projecting into the existing production estimator it deterministically
/// recompiles and compares the complete plan and receipt, binds both dataset
/// fingerprint and dataset id, and retains the exact compilation receipt in
/// the returned provenance.
pub fn run_compiled_pls_recipe_v4(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledAnalysisRecipeV4,
    posthoc_technical_minimum_sample_size: Option<&PlsPosthocTechnicalMinimumSampleSizeConfigV2>,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<RecipeV4PlsExecutionResultV1, RecipeV4PlsExecutionError> {
    if should_cancel() {
        return Err(RecipeV4PlsExecutionError::Cancelled);
    }

    let nonlinear_effects = recipe.settings.method == AnalysisMethod::NonlinearEffects
        && matches!(
            recipe.method_config.as_ref(),
            Some(MethodConfig::NonlinearEffects)
        );
    if nonlinear_effects && posthoc_technical_minimum_sample_size.is_some() {
        return Err(RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch);
    }
    if let Some(config) = posthoc_technical_minimum_sample_size {
        if !config.is_exact_v2()
            || !config.has_coherent_base_and_inference()
            || config.base_analysis
                != qpls_core::PlsPosthocTechnicalMinimumSampleSizeBaseAnalysisV2::PlsAlgorithm
            || config.inference
                != qpls_core::PlsPosthocTechnicalMinimumSampleSizeInferenceV2::PointEstimateOnly
            || config.method_version
                != qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION
        {
            return Err(RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch);
        }
        let registry = qpls_core::CapabilityRegistryV2::embedded().map_err(|error| {
            RecipeV4PlsExecutionError::PosthocCapabilityRegistry(error.to_string())
        })?;
        let available = registry.option_cells().any(|cell| {
            cell.capability_id == config.capability_cell.capability_id
                && cell.cell_id == config.capability_cell.cell_id
                && cell.capability_version == config.capability_cell.capability_version
                && (cell.standard_available() || cell.labs_available())
        });
        if !available {
            return Err(RecipeV4PlsExecutionError::PosthocCapabilityUnavailable);
        }
    }

    validate_compiled_analysis_recipe_v4(artifact, recipe, Some(resolved_model))?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(RecipeV4PlsExecutionError::CompilerTarget(
            artifact.plan().target(),
        ));
    };
    if dataset.schema.kind != DataKind::Raw {
        return Err(RecipeV4PlsExecutionError::RawDataRequired);
    }
    if dataset.fingerprint.0 != artifact.receipt().dataset_fingerprint()
        || dataset.fingerprint.0 != recipe.dataset_fingerprint
    {
        return Err(RecipeV4PlsExecutionError::DatasetFingerprintMismatch);
    }
    let actual_dataset_id = dataset.id.to_string();
    if plan.dataset_id() != actual_dataset_id {
        return Err(RecipeV4PlsExecutionError::DatasetIdMismatch {
            expected: plan.dataset_id().into(),
            actual: actual_dataset_id,
        });
    }

    let projected = project_pls_plan_to_current_recipe(recipe, resolved_model, plan)?;
    let source_initialization = match recipe.method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
        _ => None,
    };
    let projected_initialization = source_initialization
        .map(|config| translate_initialization_to_projected_columns(config, plan))
        .transpose()?;
    let projected_initialization_sha256 = projected_initialization
        .as_ref()
        .map(|initialization| sha256_serialized(initialization));
    let projected_recipe_sha256 = sha256_serialized(&projected);
    let execution = ValidatedExecutionRecipe::for_dataset(&projected, &dataset.fingerprint.0)?;
    let has_fixed_scoring = plan
        .blocks()
        .iter()
        .any(|block| block.fixed_scoring().is_some());
    let score_execution_v2 = source_initialization.is_some() || has_fixed_scoring;
    let mut report_progress = |update: qpls_estimation::EstimationProgress| {
        progress(RunnerProgress {
            phase: update.phase.as_str().into(),
            completed_units: update.completed_units,
            total_units: update.total_units,
        });
        !should_cancel()
    };
    let mut estimation = if score_execution_v2 {
        estimate_pls_validated_with_compiled_plan_v2_with_control(
            dataset,
            &execution,
            plan,
            source_initialization,
            &mut report_progress,
        )
    } else {
        estimate_pls_validated_with_control(dataset, &execution, &mut report_progress)
    }
    .map_err(|error| match error {
        EstimationError::Cancelled => RecipeV4PlsExecutionError::Cancelled,
        other => RecipeV4PlsExecutionError::Estimation(other),
    })?;

    let expected_method_version = if nonlinear_effects {
        NONLINEAR_EFFECTS_METHOD_VERSION
    } else if score_execution_v2 {
        PLS_SCORE_EXECUTION_METHOD_VERSION_V2
    } else {
        PLS_METHOD_VERSION
    };
    let score_execution_identity_valid = if score_execution_v2 {
        estimation.score_execution.as_ref().is_some_and(|resolved| {
            resolved.contract_version == PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2
        })
    } else {
        estimation.score_execution.is_none()
    };
    if estimation.method_version != expected_method_version || !score_execution_identity_valid {
        return Err(RecipeV4PlsExecutionError::EstimatorVersionMismatch {
            expected: expected_method_version,
            actual: estimation.method_version,
        });
    }
    let nonlinear_identity_valid = if nonlinear_effects {
        estimation
            .nonlinear_effects
            .as_ref()
            .is_some_and(|analysis| {
                analysis.method_version == NONLINEAR_EFFECTS_METHOD_VERSION
                    && analysis.term == "centered_squared_construct_score_v1"
            })
            && estimation.posthoc_minimum_sample_size.is_none()
    } else {
        estimation.nonlinear_effects.is_none()
    };
    if !nonlinear_identity_valid {
        return Err(RecipeV4PlsExecutionError::NonlinearEffectsIdentityMismatch);
    }
    if estimation.point_estimate_attribution.as_ref()
        != Some(&PlsPointEstimateAttributionV1::for_preprocessing(
            recipe.settings.preprocessing.clone(),
        ))
    {
        return Err(RecipeV4PlsExecutionError::PointEstimateAttributionMismatch);
    }
    if !fixed_score_scale_receipt_matches(&estimation, plan) {
        return Err(RecipeV4PlsExecutionError::FixedScoreScaleReceiptMismatch);
    }
    let algorithm_convergence_identity_valid = if nonlinear_effects {
        // The production nonlinear method preserves its own exact method
        // identity through estimation and intentionally does not emit the
        // PlsPm-only convergence-receipt family.
        estimation.algorithm_convergence_receipt.is_none()
    } else if matches!(
        recipe.settings.weighting_scheme,
        WeightingScheme::Path | WeightingScheme::Factor
    ) {
        algorithm_convergence_receipt_matches(&estimation, recipe, plan, source_initialization)
    } else {
        true
    };
    if !algorithm_convergence_identity_valid {
        return Err(RecipeV4PlsExecutionError::AlgorithmConvergenceReceiptMismatch);
    }
    // Point estimation cannot establish which structural paths are
    // statistically significant. When the exact Labs option is requested, it
    // therefore emits the explicit inference-unavailable state. Without that
    // opt-in, no secondary analytical claim is attached.
    if posthoc_technical_minimum_sample_size.is_some() {
        let control_pairs = plan
            .paths()
            .iter()
            .filter(|path| path.role() == StructuralRelationRoleV4::Control)
            .map(|path| (path.source(), path.target()))
            .collect::<HashSet<_>>();
        let substantive_paths = estimation
            .paths
            .iter()
            .filter(|path| !control_pairs.contains(&(path.source.as_str(), path.target.as_str())))
            .cloned()
            .collect::<Vec<_>>();
        estimation.posthoc_minimum_sample_size = Some(pls_posthoc_minimum_sample_size_v2(
            &substantive_paths,
            estimation.used_observations,
            None,
        ));
    }
    let current_generation = estimation.algorithm_convergence_receipt.is_some();
    let provenance = RecipeV4PlsExecutionProvenanceV1 {
        adapter_version: if nonlinear_effects {
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        } else if score_execution_v2 {
            if current_generation {
                RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2
            } else {
                LEGACY_RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2
            }
        } else if current_generation {
            RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION
        } else {
            LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION
        }
        .into(),
        compilation_receipt: artifact.receipt().clone(),
        projected_recipe_schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        projected_recipe_sha256,
        projected_initialization_sha256,
        dataset_id: dataset.id.to_string(),
        estimator_method_version: estimation.method_version.clone(),
        posthoc_technical_minimum_sample_size: posthoc_technical_minimum_sample_size.cloned(),
    };
    Ok(RecipeV4PlsExecutionResultV1 {
        schema_version: RECIPE_V4_PLS_EXECUTION_RESULT_SCHEMA_VERSION,
        provenance,
        estimation,
    })
}

fn algorithm_convergence_receipt_matches(
    estimation: &PlsResult,
    recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
    initialization: Option<&PlsAlgorithmConfigV2>,
) -> bool {
    let Some(receipt) = estimation.algorithm_convergence_receipt.as_ref() else {
        return false;
    };
    let estimated_blocks = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_none())
        .count();
    let termination_valid = if estimated_blocks == 0 {
        receipt.termination_reason == PlsAlgorithmTerminationReasonV1::AllBlocksFixed
            && receipt.performed_iterations == 0
            && receipt.final_max_outer_weight_change.is_none()
    } else {
        receipt.termination_reason == PlsAlgorithmTerminationReasonV1::ConvergedTolerance
            && receipt.performed_iterations == estimation.iterations
            && receipt.final_max_outer_weight_change.is_some_and(|change| {
                change.is_finite() && change >= 0.0 && change <= recipe.settings.tolerance
            })
    };
    receipt.contract_version == PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1
        && receipt.weighting_scheme == recipe.settings.weighting_scheme
        && receipt.maximum_iterations == recipe.settings.max_iterations
        && receipt.stop_criterion.to_bits() == recipe.settings.tolerance.to_bits()
        && receipt.comparison == PlsConvergenceComparisonV1::LessThanOrEqual
        && receipt.estimated_block_updates
            == u64::from(receipt.performed_iterations) * estimated_blocks as u64
        && termination_valid
        && receipt.blocks.len() == plan.blocks().len()
        && receipt
            .blocks
            .iter()
            .zip(plan.blocks())
            .all(|(actual, block)| {
                let expected_rule = match (block.fixed_scoring(), block.mode()) {
                    (Some(_), _) => PlsAlgorithmUpdateRuleV1::FixedNoUpdate,
                    (None, CompiledPlsBlockModeV2::ModeA) => {
                        PlsAlgorithmUpdateRuleV1::ModeACovariance
                    }
                    (None, CompiledPlsBlockModeV2::ModeB) => PlsAlgorithmUpdateRuleV1::ModeBOls,
                };
                let expected_initialization = match block.fixed_scoring() {
                    Some(CompiledPlsFixedScoringV2::Unit { .. }) => {
                        PlsAlgorithmInitializationV1::FixedUnitWeights
                    }
                    Some(CompiledPlsFixedScoringV2::Custom { .. }) => {
                        PlsAlgorithmInitializationV1::FixedCustomWeights
                    }
                    None if initialization.is_some_and(|config| {
                        matches!(
                            &config.initial_outer_weights,
                            PlsInitialOuterWeightsV2::Individual { .. }
                        )
                    }) =>
                    {
                        PlsAlgorithmInitializationV1::IndividualRequestedWeights
                    }
                    None => PlsAlgorithmInitializationV1::StandardUnitWeights,
                };
                actual.construct_id == block.construct_id()
                    && actual.indicator_order.len() == block.indicators().len()
                    && actual
                        .indicator_order
                        .iter()
                        .zip(block.indicators())
                        .all(|(actual, indicator)| actual == indicator.source_column())
                    && actual.update_rule == expected_rule
                    && actual.initialization == expected_initialization
            })
}

fn fixed_score_scale_receipt_matches(estimation: &PlsResult, plan: &CompiledPlsPlanV2) -> bool {
    let Some(execution) = estimation.score_execution.as_ref() else {
        return plan
            .blocks()
            .iter()
            .all(|block| block.fixed_scoring().is_none());
    };
    let fixed = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_some())
        .collect::<Vec<_>>();
    if fixed.is_empty() {
        return estimation.fixed_score_scale_receipt.is_none();
    }
    let Some(receipt) = estimation.fixed_score_scale_receipt.as_ref() else {
        return false;
    };
    receipt.contract_version == PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1
        && receipt.blocks.len() == fixed.len()
        && fixed.iter().zip(&receipt.blocks).all(|(compiled, scale)| {
            let Some(resolved_block) = execution
                .blocks
                .iter()
                .find(|block| block.construct_id == compiled.construct_id())
            else {
                return false;
            };
            let resolved = match &resolved_block.scoring {
                PlsResolvedScoreBlockKindV2::FixedUnit {
                    resolved_effective_weights,
                    ..
                }
                | PlsResolvedScoreBlockKindV2::FixedCustom {
                    resolved_effective_weights,
                    ..
                } => resolved_effective_weights,
                _ => return false,
            };
            scale.construct_id == compiled.construct_id()
                && scale.indicator_ids == resolved_block.indicator_ids
                && scale.pre_standardization_center.is_finite()
                && scale.pre_standardization_scale.is_finite()
                && scale.pre_standardization_scale > f64::EPSILON
                && scale.effective_unit_score_weights.len() == resolved.len()
                && scale.effective_unit_score_weights.iter().zip(resolved).all(
                    |(effective, coefficient)| {
                        effective.indicator_id == coefficient.indicator_id
                            && effective.value.to_bits()
                                == (coefficient.value / scale.pre_standardization_scale).to_bits()
                    },
                )
        })
}

fn translate_initialization_to_projected_columns(
    initialization: &PlsAlgorithmConfigV2,
    plan: &CompiledPlsPlanV2,
) -> Result<PlsAlgorithmConfigV2, RecipeV4PlsExecutionError> {
    let PlsInitialOuterWeightsV2::Individual { weights } = &initialization.initial_outer_weights
    else {
        return Ok(initialization.clone());
    };
    let stable_to_source = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_none())
        .flat_map(|block| {
            block.indicators().iter().map(|indicator| {
                (
                    (
                        block.construct_id().to_owned(),
                        indicator.variable_id().to_owned(),
                    ),
                    indicator.source_column().to_owned(),
                )
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut translated = weights
        .iter()
        .map(|weight| {
            let key = (weight.construct_id.clone(), weight.indicator_id.clone());
            let source_column = stable_to_source.get(&key).ok_or_else(|| {
                RecipeV4PlsExecutionError::UnknownInitialWeightIdentity(
                    weight.construct_id.clone(),
                    weight.indicator_id.clone(),
                )
            })?;
            Ok(PlsInitialOuterWeightV2 {
                construct_id: weight.construct_id.clone(),
                indicator_id: source_column.clone(),
                value: weight.value,
            })
        })
        .collect::<Result<Vec<_>, RecipeV4PlsExecutionError>>()?;
    translated.sort_by(|left, right| {
        (&left.construct_id, &left.indicator_id).cmp(&(&right.construct_id, &right.indicator_id))
    });
    if translated.len() != stable_to_source.len()
        || translated.windows(2).any(|pair| {
            (&pair[0].construct_id, &pair[0].indicator_id)
                >= (&pair[1].construct_id, &pair[1].indicator_id)
        })
    {
        return Err(RecipeV4PlsExecutionError::InitialWeightTranslationMismatch);
    }
    Ok(PlsAlgorithmConfigV2 {
        initialization_contract_version: initialization.initialization_contract_version.clone(),
        initial_outer_weights: PlsInitialOuterWeightsV2::Individual {
            weights: translated,
        },
    })
}

pub(crate) fn project_pls_plan_to_current_recipe(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    plan: &CompiledPlsPlanV2,
) -> Result<AnalysisRecipe, RecipeV4PlsExecutionError> {
    let labels = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Composite { id, label, .. } => Some((id.as_str(), label.as_str())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let constructs = plan
        .blocks()
        .iter()
        .map(|block| {
            let label = labels.get(block.construct_id()).ok_or_else(|| {
                RecipeV4PlsExecutionError::UnknownComposite(block.construct_id().into())
            })?;
            Ok(Construct {
                id: block.construct_id().into(),
                name: (*label).into(),
                short_name: (*label).into(),
                mode: match block.mode() {
                    CompiledPlsBlockModeV2::ModeA => MeasurementMode::Reflective,
                    CompiledPlsBlockModeV2::ModeB => MeasurementMode::Formative,
                },
                indicators: block
                    .indicators()
                    .iter()
                    .map(|indicator| indicator.source_column().to_owned())
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, RecipeV4PlsExecutionError>>()?;
    let paths = plan
        .paths()
        .iter()
        .map(|path| StructuralPath {
            source: path.source().into(),
            target: path.target().into(),
        })
        .collect();
    let parameter_labels = model
        .parameters
        .iter()
        .map(|parameter| {
            let label = match parameter {
                SemParameterV4::Free { label, .. }
                | SemParameterV4::Fixed { label, .. }
                | SemParameterV4::Derived { label, .. } => label,
            };
            (parameter.id(), label.as_str())
        })
        .collect::<HashMap<_, _>>();
    let controls = plan
        .paths()
        .iter()
        .filter(|path| path.role() == StructuralRelationRoleV4::Control)
        .map(|path| {
            let label = parameter_labels.get(path.parameter_id()).ok_or_else(|| {
                RecipeV4PlsExecutionError::UnknownPathParameter(path.parameter_id().into())
            })?;
            Ok(ControlPath {
                source: path.source().into(),
                target: path.target().into(),
                label: Some((*label).into()),
            })
        })
        .collect::<Result<Vec<_>, RecipeV4PlsExecutionError>>()?;

    Ok(AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: recipe.id,
        created_at: recipe.created_at,
        dataset_fingerprint: recipe.dataset_fingerprint.clone(),
        model: ModelSpec {
            id: deterministic_model_uuid(plan)?,
            name: model.name.clone(),
            constructs,
            paths,
            controls,
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        },
        settings: recipe.settings.clone(),
        method_config: Some(match recipe.method_config.as_ref() {
            Some(MethodConfig::NonlinearEffects) => MethodConfig::NonlinearEffects,
            _ => MethodConfig::PlsAlgorithm,
        }),
        // Recipe-v4 metadata remains in the exact compilation receipt. It is
        // deliberately not projected into the legacy estimator contract, so
        // an annotation can never become an implicit execution option.
        metadata: Default::default(),
    })
}

fn deterministic_model_uuid(plan: &CompiledPlsPlanV2) -> Result<Uuid, RecipeV4PlsExecutionError> {
    if let Ok(id) = Uuid::parse_str(plan.model_id()) {
        return Ok(id);
    }
    let digest = plan.scientific_hash().as_bytes();
    if digest.len() != 64 {
        return Err(RecipeV4PlsExecutionError::InvalidModelIdentity);
    }
    let mut bytes = [0u8; 16];
    for (index, slot) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        let pair = std::str::from_utf8(&digest[offset..offset + 2])
            .map_err(|_| RecipeV4PlsExecutionError::InvalidModelIdentity)?;
        *slot = u8::from_str_radix(pair, 16)
            .map_err(|_| RecipeV4PlsExecutionError::InvalidModelIdentity)?;
    }
    Ok(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisSettings, Construct,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, Preprocessing,
        RecipeV4CompilationError, SemDataBindingV4, StructuralPath, compile_analysis_recipe_v4,
        confirm_legacy_recipe_estimand_v4, migrate_analysis_recipe_to_v4_pending,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_estimation::PlsPosthocMinimumSampleSizeStatus;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    fn source_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(101),
            name: "Runner fixture".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "Predictor".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Outcome".into(),
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
        }
    }

    fn fixture_from_source_model(
        source_model: ModelSpec,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,2,1\n2,3,3,2\n3,5,4,4\n4,4,6,5\n5,6,7,7\n6,7,9,8\n",
            "recipe-v4-runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(202),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model,
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::from([("study".into(), "fixture".into())]),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        fixture_from_source_model(source_model())
    }

    fn nonlinear_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let (dataset, mut recipe, model, _) = fixture();
        recipe.settings.method = AnalysisMethod::NonlinearEffects;
        recipe.method_config = Some(MethodConfig::NonlinearEffects);
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            target,
            target.capability_cell_for_method(AnalysisMethod::NonlinearEffects),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    #[test]
    fn validated_compilation_dispatches_the_production_estimator_deterministically() {
        let (dataset, recipe, model, artifact) = fixture();
        let progress = Mutex::new(Vec::new());
        let first = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();
        let second = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.schema_version(), 1);
        assert_eq!(first.estimation().method_version, PLS_METHOD_VERSION);
        assert!(first.estimation().converged);
        assert_eq!(first.estimation().paths.len(), 1);
        assert!(first.estimation().posthoc_minimum_sample_size.is_none());
        assert!(
            first
                .provenance()
                .posthoc_technical_minimum_sample_size()
                .is_none()
        );

        let posthoc = qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2();
        let opted = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            Some(&posthoc),
            || false,
            |_| {},
        )
        .unwrap();
        let technical = opted
            .estimation()
            .posthoc_minimum_sample_size
            .as_ref()
            .expect("the exact Recipe-v4 opt-in must attach the typed post-hoc result");
        assert_eq!(
            technical.status,
            PlsPosthocMinimumSampleSizeStatus::InferenceUnavailable
        );
        assert_eq!(technical.technically_required_sample_size, None);
        assert_eq!(technical.significance_source, None);
        assert_eq!(technical.test, "directional");
        assert_eq!(
            opted.provenance().posthoc_technical_minimum_sample_size(),
            Some(&posthoc)
        );
        assert_eq!(first.provenance().compilation_receipt(), artifact.receipt());
        assert_eq!(
            first.provenance().adapter_version(),
            RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION
        );
        assert_eq!(first.provenance().dataset_id(), dataset.id.to_string());
        assert!(!first.provenance().projected_recipe_sha256().is_empty());
        assert!(!progress.lock().unwrap().is_empty());
    }

    #[test]
    fn nonlinear_v7_preserves_the_exact_method_through_private_projection() {
        let (dataset, recipe, model, artifact) = nonlinear_fixture();
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        let projected = project_pls_plan_to_current_recipe(&recipe, &model, plan).unwrap();
        assert_eq!(projected.settings.method, AnalysisMethod::NonlinearEffects);
        assert_eq!(
            projected.method_config,
            Some(MethodConfig::NonlinearEffects)
        );

        let first = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();
        let second = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.provenance().adapter_version(),
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        );
        assert_eq!(
            first.provenance().estimator_method_version(),
            NONLINEAR_EFFECTS_METHOD_VERSION
        );
        assert!(first.estimation().algorithm_convergence_receipt.is_none());
        assert!(first.estimation().posthoc_minimum_sample_size.is_none());
        let nonlinear = first
            .estimation()
            .nonlinear_effects
            .as_ref()
            .expect("typed nonlinear result");
        assert_eq!(nonlinear.method_version, NONLINEAR_EFFECTS_METHOD_VERSION);
        assert_eq!(nonlinear.term, "centered_squared_construct_score_v1");
        assert_eq!(nonlinear.estimates.len(), 1);
        assert_eq!(nonlinear.warnings.len(), 1);
        assert_eq!(nonlinear.estimates[0].source, "construct:x");
        assert_eq!(nonlinear.estimates[0].target, "construct:y");
        assert_eq!(
            artifact.receipt().capability_cell(),
            &RecipeV4CompilerTarget::PlsPlanV2
                .capability_cell_for_method(AnalysisMethod::NonlinearEffects)
        );

        let posthoc = qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2();
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &artifact,
                Some(&posthoc),
                || false,
                |_| {},
            ),
            Err(RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch)
        ));
    }

    #[test]
    fn compiled_pls_convergence_receipt_proves_mode_a_b_order_and_stopping() {
        let mut source = source_model();
        source.constructs[1].mode = MeasurementMode::Formative;
        let (dataset, recipe, model, artifact) = fixture_from_source_model(source);
        let result = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();
        let receipt = result
            .estimation()
            .algorithm_convergence_receipt
            .as_ref()
            .expect("typed convergence receipt");
        assert_eq!(receipt.performed_iterations, result.estimation().iterations);
        assert_eq!(receipt.blocks[0].construct_id, "construct:x");
        assert_eq!(receipt.blocks[1].construct_id, "construct:y");
        assert_eq!(
            receipt.blocks[0].indicator_order,
            vec!["x1".to_string(), "x2".to_string()]
        );
        assert_eq!(
            receipt.blocks[1].indicator_order,
            vec!["y1".to_string(), "y2".to_string()]
        );
        assert_eq!(
            receipt.blocks[0].update_rule,
            PlsAlgorithmUpdateRuleV1::ModeACovariance
        );
        assert_eq!(
            receipt.blocks[1].update_rule,
            PlsAlgorithmUpdateRuleV1::ModeBOls
        );
        assert!(
            receipt
                .final_max_outer_weight_change
                .is_some_and(|change| change <= receipt.stop_criterion)
        );
    }

    #[test]
    fn compiled_pls_point_estimates_carry_exact_preprocessing_scale_attribution() {
        for preprocessing in [
            Preprocessing::Standardized,
            Preprocessing::MeanCentered,
            Preprocessing::Unstandardized,
        ] {
            let (dataset, mut recipe, model, _) = fixture();
            recipe.settings.preprocessing = preprocessing.clone();
            let artifact = compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                RecipeV4CompilerTarget::PlsPlanV2,
                RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            )
            .unwrap();
            let result = run_compiled_pls_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &artifact,
                None,
                || false,
                |_| {},
            )
            .unwrap();
            assert_eq!(
                result.estimation().point_estimate_attribution.as_ref(),
                Some(&PlsPointEstimateAttributionV1::for_preprocessing(
                    preprocessing.clone(),
                ))
            );
            assert!(result.estimation().transforms.iter().all(|transform| {
                match &preprocessing {
                    Preprocessing::Standardized => transform.scale != 1.0,
                    Preprocessing::MeanCentered => transform.mean != 0.0 && transform.scale == 1.0,
                    Preprocessing::Unstandardized => {
                        transform.mean == 0.0 && transform.scale == 1.0
                    }
                }
            }));
        }
    }

    #[test]
    fn compiled_control_role_reaches_the_typed_pls_result_without_losing_the_path() {
        let (dataset, mut recipe, mut model, _) = fixture();
        let relation = model
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, qpls_core::SemRelationV4::Structural { .. }))
            .expect("fixture structural path");
        let qpls_core::SemRelationV4::Structural { role, .. } = relation else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Control;
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();

        let result = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();

        assert_eq!(result.estimation().paths.len(), 1);
        assert_eq!(result.estimation().control_estimates.len(), 1);
        let control = &result.estimation().control_estimates[0];
        assert_eq!(
            (control.source.as_str(), control.target.as_str()),
            ("construct:x", "construct:y")
        );
        assert_eq!(
            control.coefficient,
            result.estimation().paths[0].coefficient
        );
        assert!(
            control
                .label
                .as_deref()
                .is_some_and(|label| !label.is_empty())
        );
    }

    #[test]
    fn individual_initialization_translates_stable_ids_without_changing_result_identity() {
        let (dataset, mut recipe, model, baseline) = fixture();
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = baseline.plan() else {
            unreachable!()
        };
        let mut weights = plan
            .blocks()
            .iter()
            .flat_map(|block| {
                block
                    .indicators()
                    .iter()
                    .enumerate()
                    .map(move |(index, indicator)| PlsInitialOuterWeightV2 {
                        construct_id: block.construct_id().into(),
                        indicator_id: indicator.variable_id().into(),
                        value: (index + 1) as f64,
                    })
            })
            .collect::<Vec<_>>();
        weights.sort_by(|left, right| {
            (&left.construct_id, &left.indicator_id)
                .cmp(&(&right.construct_id, &right.indicator_id))
        });
        recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
            PlsAlgorithmConfigV2 {
                initialization_contract_version:
                    qpls_core::PLS_INITIAL_OUTER_WEIGHTS_CONTRACT_VERSION_V2.into(),
                initial_outer_weights: PlsInitialOuterWeightsV2::Individual { weights },
            },
        ));
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();

        let result = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();

        assert_eq!(
            result.estimation().method_version,
            PLS_SCORE_EXECUTION_METHOD_VERSION_V2
        );
        assert!(
            result
                .provenance()
                .projected_initialization_sha256()
                .is_some()
        );
        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2
        );
        let resolved = result.estimation().score_execution.as_ref().unwrap();
        assert_eq!(
            resolved.contract_version,
            PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2
        );
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        for (block, resolved_block) in plan.blocks().iter().zip(&resolved.blocks) {
            assert_eq!(resolved_block.construct_id, block.construct_id());
            assert_eq!(
                resolved_block.indicator_ids,
                block
                    .indicators()
                    .iter()
                    .map(|indicator| indicator.variable_id().to_owned())
                    .collect::<Vec<_>>()
            );
            for indicator in block.indicators() {
                assert!(result.estimation().outer_estimates.iter().any(|estimate| {
                    estimate.construct == block.construct_id()
                        && estimate.indicator == indicator.source_column()
                }));
            }
        }
    }

    #[test]
    fn custom_unit_variance_block_passes_through_and_only_estimated_blocks_update() {
        let mut source = source_model();
        source.constructs[0].mode = MeasurementMode::Formative;
        let (dataset, mut recipe, mut model, _) = fixture_from_source_model(source);
        let SemVariableV4::Composite { weighting, .. } = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        else {
            unreachable!()
        };
        *weighting = qpls_core::CompositeWeightingV4::Custom {
            weights: BTreeMap::from([("observed:x1".into(), 2.0), ("observed:x2".into(), -1.0)]),
            normalization: qpls_core::CompositeWeightNormalizationV4::UnitVariance,
        };
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();

        let result = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();
        let execution = result.estimation().score_execution.as_ref().unwrap();
        assert_eq!(execution.iteration_accounting.estimated_block_count, 1);
        assert_eq!(execution.iteration_accounting.fixed_block_count, 1);
        assert_eq!(
            execution.iteration_accounting.estimated_block_updates,
            u64::from(execution.iteration_accounting.performed_iterations)
        );
        let fixed = execution
            .blocks
            .iter()
            .find(|block| block.construct_id == "construct:x")
            .unwrap();
        assert!(matches!(
            &fixed.scoring,
            qpls_estimation::PlsResolvedScoreBlockKindV2::FixedCustom {
                normalization: qpls_core::CompositeWeightNormalizationV4::UnitVariance,
                ..
            }
        ));
        let scale = result
            .estimation()
            .fixed_score_scale_receipt
            .as_ref()
            .unwrap();
        assert_eq!(scale.blocks.len(), 1);
        assert_eq!(scale.blocks[0].construct_id, "construct:x");
        let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
            unreachable!()
        };
        assert!(fixed_score_scale_receipt_matches(result.estimation(), plan));
        let mut tampered = result.estimation().clone();
        tampered.fixed_score_scale_receipt.as_mut().unwrap().blocks[0]
            .effective_unit_score_weights[0]
            .value += 0.125;
        assert!(!fixed_score_scale_receipt_matches(&tampered, plan));
        tampered.fixed_score_scale_receipt = None;
        assert!(!fixed_score_scale_receipt_matches(&tampered, plan));
    }

    #[test]
    fn configured_fixed_path_factor_executes_mean_centered_and_unstandardized_exactly() {
        for preprocessing in [Preprocessing::MeanCentered, Preprocessing::Unstandardized] {
            for weighting_scheme in [WeightingScheme::Path, WeightingScheme::Factor] {
                let mut source = source_model();
                source.constructs[0].mode = MeasurementMode::Formative;
                let (dataset, mut recipe, mut model, _) = fixture_from_source_model(source);
                let SemVariableV4::Composite { weighting, .. } = model
                    .variables
                    .iter_mut()
                    .find(|variable| variable.id() == "construct:x")
                    .unwrap()
                else {
                    unreachable!()
                };
                *weighting = qpls_core::CompositeWeightingV4::Custom {
                    weights: BTreeMap::from([
                        ("observed:x1".into(), 2.0),
                        ("observed:x2".into(), -1.0),
                    ]),
                    normalization: qpls_core::CompositeWeightNormalizationV4::None,
                };
                recipe.settings.preprocessing = preprocessing.clone();
                recipe.settings.weighting_scheme = weighting_scheme;
                recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
                    PlsAlgorithmConfigV2::standard(),
                ));
                recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                    model: model.clone(),
                    scientific_sha256: model.scientific_sha256().unwrap(),
                };
                let artifact = compile_analysis_recipe_v4(
                    &recipe,
                    Some(&model),
                    RecipeV4CompilerTarget::PlsPlanV2,
                    RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
                )
                .unwrap();
                let result = run_compiled_pls_recipe_v4(
                    &dataset,
                    &recipe,
                    &model,
                    &artifact,
                    None,
                    || false,
                    |_| {},
                )
                .unwrap();
                assert_eq!(
                    result.estimation().point_estimate_attribution.as_ref(),
                    Some(&PlsPointEstimateAttributionV1::for_preprocessing(
                        preprocessing.clone()
                    ))
                );
                assert!(result.estimation().transforms.iter().all(|transform| {
                    match &preprocessing {
                        Preprocessing::MeanCentered => {
                            transform.mean != 0.0 && transform.scale.to_bits() == 1.0_f64.to_bits()
                        }
                        Preprocessing::Unstandardized => {
                            transform.mean.to_bits() == 0.0_f64.to_bits()
                                && transform.scale.to_bits() == 1.0_f64.to_bits()
                        }
                        Preprocessing::Standardized => unreachable!(),
                    }
                }));
                let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
                    unreachable!()
                };
                assert!(fixed_score_scale_receipt_matches(result.estimation(), plan));
                let receipt = &result
                    .estimation()
                    .fixed_score_scale_receipt
                    .as_ref()
                    .unwrap()
                    .blocks[0];
                assert_eq!(receipt.construct_id, "construct:x");
                assert!(receipt.pre_standardization_scale > f64::EPSILON);
                match &preprocessing {
                    Preprocessing::MeanCentered => {
                        assert!(receipt.pre_standardization_center.abs() < 1e-12)
                    }
                    Preprocessing::Unstandardized => {
                        assert!(receipt.pre_standardization_center.abs() > f64::EPSILON)
                    }
                    Preprocessing::Standardized => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn fixed_only_unit_scoring_performs_zero_iterative_updates() {
        let mut source = source_model();
        for construct in &mut source.constructs {
            construct.mode = MeasurementMode::Formative;
        }
        let (dataset, mut recipe, mut model, _) = fixture_from_source_model(source);
        for variable in &mut model.variables {
            if let SemVariableV4::Composite { weighting, .. } = variable {
                *weighting = qpls_core::CompositeWeightingV4::Unit {
                    normalization: qpls_core::CompositeWeightNormalizationV4::UnitVariance,
                };
            }
        }
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::PlsPlanV2,
            RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        )
        .unwrap();
        let result = run_compiled_pls_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            None,
            || false,
            |_| {},
        )
        .unwrap();
        let accounting = &result
            .estimation()
            .score_execution
            .as_ref()
            .unwrap()
            .iteration_accounting;
        assert_eq!(accounting.estimated_block_count, 0);
        assert_eq!(accounting.fixed_block_count, 2);
        assert_eq!(accounting.performed_iterations, 0);
        assert_eq!(accounting.estimated_block_updates, 0);
        assert_eq!(result.estimation().iterations, 0);
    }

    #[test]
    fn posthoc_option_identity_tampering_fails_before_estimation() {
        let (dataset, recipe, model, artifact) = fixture();
        let exact = qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2();
        for (field, value) in [
            (
                "method_version",
                serde_json::json!("inverse_square_root_posthoc_v1"),
            ),
            ("capability_version", serde_json::json!("drifted_v3")),
        ] {
            let mut encoded = serde_json::to_value(&exact).unwrap();
            if field == "capability_version" {
                encoded["capability_cell"][field] = value;
            } else {
                encoded[field] = value;
            }
            let tampered: qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2 =
                serde_json::from_value(encoded).unwrap();
            assert!(matches!(
                run_compiled_pls_recipe_v4(
                    &dataset,
                    &recipe,
                    &model,
                    &artifact,
                    Some(&tampered),
                    || false,
                    |_| {},
                ),
                Err(RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch)
            ));
        }

        let claimed_bootstrap_without_a_bootstrap_plan =
            qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::bootstrap_v2();
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &artifact,
                Some(&claimed_bootstrap_without_a_bootstrap_plan),
                || false,
                |_| {},
            ),
            Err(RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch)
        ));
    }

    #[test]
    fn capability_recipe_and_model_digest_tampering_fail_before_estimation() {
        let (dataset, recipe, model, artifact) = fixture();
        let mut value = serde_json::to_value(&artifact).unwrap();
        value["receipt"]["capability_cell"]["cell_id"] = serde_json::json!("qpls3.cbsem.ml");
        let tampered: CompiledAnalysisRecipeV4 = serde_json::from_value(value).unwrap();
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &tampered,
                None,
                || false,
                |_| {},
            ),
            Err(RecipeV4PlsExecutionError::Compilation(
                RecipeV4CompilationError::CapabilityCellMismatch
            ))
        ));

        let mut changed_recipe = recipe.clone();
        changed_recipe
            .metadata
            .insert("note".into(), "changed after compilation".into());
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &dataset,
                &changed_recipe,
                &model,
                &artifact,
                None,
                || false,
                |_| {}
            ),
            Err(RecipeV4PlsExecutionError::Compilation(
                RecipeV4CompilationError::ArtifactMismatch
            ))
        ));

        let mut changed_model = model.clone();
        changed_model.name.push_str(" changed");
        assert!(
            run_compiled_pls_recipe_v4(
                &dataset,
                &recipe,
                &changed_model,
                &artifact,
                None,
                || false,
                |_| {}
            )
            .is_err()
        );
    }

    #[test]
    fn dataset_identity_and_cancellation_are_bound_before_dispatch() {
        let (dataset, recipe, model, artifact) = fixture();
        let mut wrong_fingerprint = dataset.clone();
        wrong_fingerprint.fingerprint.0 = "different".into();
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &wrong_fingerprint,
                &recipe,
                &model,
                &artifact,
                None,
                || false,
                |_| {}
            ),
            Err(RecipeV4PlsExecutionError::DatasetFingerprintMismatch)
        ));

        let mut wrong_id = dataset.clone();
        wrong_id.id = Uuid::from_u128(999);
        assert!(matches!(
            run_compiled_pls_recipe_v4(
                &wrong_id,
                &recipe,
                &model,
                &artifact,
                None,
                || false,
                |_| {},
            ),
            Err(RecipeV4PlsExecutionError::DatasetIdMismatch { .. })
        ));

        assert!(matches!(
            run_compiled_pls_recipe_v4(&dataset, &recipe, &model, &artifact, None, || true, |_| {},),
            Err(RecipeV4PlsExecutionError::Cancelled)
        ));
    }
}
