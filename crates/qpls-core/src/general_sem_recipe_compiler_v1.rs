use crate::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, CapabilityCellReferenceV2,
    CapabilityRegistryV2, CompiledAnalysisRecipeV4, CompiledPlsPlanV3, CompiledPlsPlanV3Error,
    CompiledRecipePlanV4, GeneralSemBootstrapIntervalV1, GeneralSemInferenceTailV1,
    GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1, MissingDataPolicy,
    PlsBootstrapTestTail, Preprocessing, RecipeV4CompilationError, RecipeV4CompilerTarget,
    SemModelV4, WeightingScheme, compile_analysis_recipe_v4,
    compile_pls_higher_order_lower_order_projection_v1, compile_pls_plan_v3,
    compile_pls_stage_one_projection_v3, pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1, validate_compiled_analysis_recipe_v4,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1: &str = "recipe_v4_to_compiled_pls_plan_v3_v1";
pub const GENERAL_SEM_PLS_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1";
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1";
pub const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_bootstrap_v1";
pub const GENERAL_SEM_PLS_HIGHER_ORDER_POINT_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_higher_order_point_v1";
pub const GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_higher_order_full_model_case_bootstrap_v1";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_ID_V1: &str = "smartpls.mediation";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CELL_ID_V1: &str = "qpls3.pls.mediation";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_VERSION_V1: &str = "pls_mediation_v1";
pub const PLS_GENERAL_BOOTSTRAP_CAPABILITY_ID_V1: &str = "smartpls.mediation";
pub const PLS_GENERAL_BOOTSTRAP_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_multiple_mediation_bootstrap";
pub const PLS_GENERAL_BOOTSTRAP_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_full_model_case_bootstrap_v1";
pub const PLS_GENERAL_MULTIPLE_MODERATION_CAPABILITY_ID_V1: &str = "smartpls.moderation";
pub const PLS_GENERAL_MULTIPLE_MODERATION_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_multiple_two_way_moderation_point";
pub const PLS_GENERAL_MULTIPLE_MODERATION_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_multiple_two_way_moderation_point_v1";
pub const PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_ID_V1: &str = "smartpls.moderation";
pub const PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap";
pub const PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1";

pub fn pls_general_recursive_effects_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_RECURSIVE_EFFECTS_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn pls_general_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_BOOTSTRAP_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_BOOTSTRAP_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_BOOTSTRAP_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn pls_general_multiple_moderation_point_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_MULTIPLE_MODERATION_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_MULTIPLE_MODERATION_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_MULTIPLE_MODERATION_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn pls_general_multiple_moderation_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_VERSION_V1.into(),
    }
}

/// Additive General-SEM compilation artifact. The existing recipe-v4 PLS
/// artifact remains authoritative for score estimation; the v3 plan adds the
/// exact topology and effect-estimand identities consumed after estimation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledGeneralSemPlsRecipeV1 {
    schema_version: u32,
    compiler_version: String,
    capability_cell: CapabilityCellReferenceV2,
    base_artifact: CompiledAnalysisRecipeV4,
    plan: CompiledPlsPlanV3,
    recipe_analytical_sha256: String,
    general_sem_config_sha256: String,
    artifact_identity_sha256: String,
}

impl CompiledGeneralSemPlsRecipeV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn compiler_version(&self) -> &str {
        &self.compiler_version
    }

    pub fn capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.capability_cell
    }

    pub fn base_artifact(&self) -> &CompiledAnalysisRecipeV4 {
        &self.base_artifact
    }

    pub fn plan(&self) -> &CompiledPlsPlanV3 {
        &self.plan
    }

    pub fn general_sem_config_sha256(&self) -> &str {
        &self.general_sem_config_sha256
    }

    pub fn recipe_analytical_sha256(&self) -> &str {
        &self.recipe_analytical_sha256
    }

    pub fn artifact_identity_sha256(&self) -> &str {
        &self.artifact_identity_sha256
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum GeneralSemPlsRecipeCompilationErrorV1 {
    #[error("General SEM PLS compilation requires general_sem_config")]
    MissingGeneralSemConfig,
    #[error(
        "General SEM PLS moderation v1 does not execute authored conditional probes; bootstrap inference is gamma-only"
    )]
    ConditionalProbesNotYetExecutable,
    #[error("General SEM PLS interaction_v2 moderation requires an empty requested-effect list")]
    InteractionRequestedEffectsNotYetExecutable,
    #[error(
        "General SEM PLS interaction_v2 moderation does not yet execute moderated mediation or other directed chains"
    )]
    ModeratedMediationNotYetExecutable,
    #[error("General SEM PLS case-bootstrap v1 executes percentile intervals only")]
    BootstrapIntervalNotYetExecutable,
    #[error("General SEM PLS case-bootstrap v1 executes two-sided inference only")]
    BootstrapTailNotYetExecutable,
    #[error(
        "General SEM point inference requires recipe settings.bootstrap_samples=0 (found {found})"
    )]
    PointInferenceBootstrapSamplesMismatch { found: u32 },
    #[error("General SEM point inference requires recipe settings.bootstrap_test_tail=two_sided")]
    PointInferenceBootstrapTailMismatch,
    #[error(
        "General SEM point inference requires recipe settings.studentized_inner_samples=0 (found {found})"
    )]
    PointInferenceStudentizedSamplesMismatch { found: u32 },
    #[error(
        "General SEM case-bootstrap resamples ({expected}) differ from recipe settings.bootstrap_samples ({found})"
    )]
    BootstrapSamplesMismatch { expected: u32, found: u32 },
    #[error(
        "General SEM case-bootstrap seed ({expected}) differs from recipe settings.seed ({found})"
    )]
    BootstrapSeedMismatch { expected: u64, found: u64 },
    #[error(
        "General SEM case-bootstrap confidence_level ({expected}) differs from recipe settings.confidence_level ({found})"
    )]
    BootstrapConfidenceLevelMismatch { expected: f64, found: f64 },
    #[error("General SEM case-bootstrap requires recipe settings.bootstrap_test_tail=two_sided")]
    BootstrapRecipeTailMismatch,
    #[error(
        "General SEM percentile bootstrap requires recipe settings.studentized_inner_samples=0 (found {found})"
    )]
    BootstrapStudentizedSamplesMismatch { found: u32 },
    #[error("General SEM PLS v1 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotYetExecutable,
    #[error("General SEM HOC v1 requires PLS-PM estimation")]
    HigherOrderRequiresPlsPm,
    #[error("General SEM HOC v1 requires path weighting")]
    HigherOrderRequiresPathWeighting,
    #[error("General SEM HOC v1 requires standardized preprocessing")]
    HigherOrderRequiresStandardizedPreprocessing,
    #[error("General SEM HOC v1 requires recipe-level listwise deletion")]
    HigherOrderRequiresListwiseDeletion,
    #[error("General SEM HOC v1 does not support recipe-level case weighting")]
    HigherOrderCaseWeightsNotExecutable,
    #[error("General SEM HOC v1 does not support permutation requests")]
    HigherOrderPermutationNotExecutable,
    #[error("Capability Registry V2 cannot authorize the required General SEM PLS cells: {0}")]
    CapabilityRegistry(String),
    #[error("a required General SEM PLS capability cell is not available in Labs or Standard")]
    CapabilityUnavailable,
    #[error(transparent)]
    BaseCompilation(#[from] RecipeV4CompilationError),
    #[error(transparent)]
    PlsPlanV3(#[from] CompiledPlsPlanV3Error),
    #[error("compiled PLS v3 base plan differs from the proven recipe-v4 PLS v2 plan")]
    BasePlanMismatch,
    #[error(
        "General SEM PLS mediation point estimation requires at least one compiled specific indirect path (found {found})"
    )]
    MediationRequiresIndirectPath { found: usize },
    #[error(
        "General SEM multiple-mediation bootstrap requires at least two compiled specific indirect paths (found {found})"
    )]
    MultipleMediationRequiresTwoIndirectPaths { found: usize },
    #[error("compiled General SEM artifact contract is invalid")]
    InvalidArtifactContract,
    #[error("compiled General SEM artifact differs from deterministic recompilation")]
    ArtifactMismatch,
    #[error("General SEM compilation value could not be serialized: {0}")]
    Serialization(String),
}

pub fn compile_general_sem_pls_recipe_v1(
    recipe: &AnalysisRecipeV4,
    resolved_model: Option<&SemModelV4>,
) -> Result<CompiledGeneralSemPlsRecipeV1, GeneralSemPlsRecipeCompilationErrorV1> {
    let config = recipe
        .general_sem_config
        .as_ref()
        .ok_or(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)?;
    ensure_compilation_scope(recipe, config)?;
    let model = resolved_model_from_recipe(recipe, resolved_model)?;
    ensure_higher_order_recipe_scope(recipe, model)?;
    let plan = compile_pls_plan_v3(model, config)?;
    ensure_interaction_compilation_scope(config, &plan)?;
    ensure_capabilities_available(config, &plan)?;

    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let (base_recipe, base_model) = project_general_sem_pls_stage_one_recipe_v1(recipe, model)?;
    let base_artifact = compile_analysis_recipe_v4(
        &base_recipe,
        Some(&base_model),
        target,
        target.capability_cell_for_method(recipe.settings.method),
    )?;
    if plan.two_way_interactions().is_empty() && plan.higher_order_stage_plans().is_empty() {
        let found = plan.topology().specific_directed_paths().len();
        match config.inference {
            GeneralSemInferenceV1::None if found == 0 => {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::MediationRequiresIndirectPath { found },
                );
            }
            GeneralSemInferenceV1::CaseBootstrap { .. } if found < 2 => {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::MultipleMediationRequiresTwoIndirectPaths {
                        found,
                    },
                );
            }
            _ => {}
        }
    }
    let CompiledRecipePlanV4::PlsPlanV2 { plan: base_plan } = base_artifact.plan() else {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::BasePlanMismatch);
    };
    if plan.base_plan() != base_plan {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::BasePlanMismatch);
    }

    let capability_cell = capability_cell_for_plan(&plan);
    let compiler_version = compiler_version_for_plan(config, &plan);
    let recipe_analytical_sha256 = crate::recipe_v4_analytical_sha256(recipe, model)?;
    let general_sem_config_sha256 = plan.general_sem_config_sha256().to_string();
    let artifact_identity_sha256 = hash_serializable(&GeneralSemArtifactIdentityV1 {
        schema_version: GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1,
        compiler_version,
        capability_cell: &capability_cell,
        recipe_analytical_sha256: &recipe_analytical_sha256,
        base_analytical_identity_sha256: base_artifact.receipt().analytical_identity_sha256(),
        plan_sha256: &plan.deterministic_sha256(),
        general_sem_config_sha256: &general_sem_config_sha256,
    })?;
    Ok(CompiledGeneralSemPlsRecipeV1 {
        schema_version: GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1,
        compiler_version: compiler_version.into(),
        capability_cell,
        base_artifact,
        plan,
        recipe_analytical_sha256,
        general_sem_config_sha256,
        artifact_identity_sha256,
    })
}

pub fn validate_compiled_general_sem_pls_recipe_v1(
    artifact: &CompiledGeneralSemPlsRecipeV1,
    recipe: &AnalysisRecipeV4,
    resolved_model: Option<&SemModelV4>,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    let config = recipe
        .general_sem_config
        .as_ref()
        .ok_or(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)?;
    ensure_compilation_scope(recipe, config)?;
    let model = resolved_model_from_recipe(recipe, resolved_model)?;
    ensure_higher_order_recipe_scope(recipe, model)?;
    let expected_plan = compile_pls_plan_v3(model, config)?;
    ensure_interaction_compilation_scope(config, &expected_plan)?;
    if artifact.schema_version != GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1
        || artifact.compiler_version != compiler_version_for_plan(config, &expected_plan)
        || artifact.capability_cell != capability_cell_for_plan(&expected_plan)
        || artifact.recipe_analytical_sha256
            != crate::recipe_v4_analytical_sha256(
                recipe,
                resolved_model_from_recipe(recipe, resolved_model)?,
            )?
        || artifact.general_sem_config_sha256 != artifact.plan.general_sem_config_sha256()
        || !is_lowercase_sha256(&artifact.artifact_identity_sha256)
    {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::InvalidArtifactContract);
    }
    let (base_recipe, base_model) = project_general_sem_pls_stage_one_recipe_v1(recipe, model)?;
    validate_compiled_analysis_recipe_v4(
        artifact.base_artifact(),
        &base_recipe,
        Some(&base_model),
    )?;
    let expected = compile_general_sem_pls_recipe_v1(recipe, resolved_model)?;
    if *artifact != expected {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::ArtifactMismatch);
    }
    Ok(())
}

fn ensure_compilation_scope(
    recipe: &AnalysisRecipeV4,
    config: &crate::GeneralSemConfigV1,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    if !config.conditional_effect_probes.is_empty() {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::ConditionalProbesNotYetExecutable);
    }
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(
            GeneralSemPlsRecipeCompilationErrorV1::LazySpecificPathMaterializationNotYetExecutable,
        );
    }
    match config.inference {
        GeneralSemInferenceV1::None => {
            if recipe.settings.bootstrap_samples != 0 {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::PointInferenceBootstrapSamplesMismatch {
                        found: recipe.settings.bootstrap_samples,
                    },
                );
            }
            if recipe.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::PointInferenceBootstrapTailMismatch,
                );
            }
            if recipe.settings.studentized_inner_samples != 0 {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::PointInferenceStudentizedSamplesMismatch {
                        found: recipe.settings.studentized_inner_samples,
                    },
                );
            }
        }
        GeneralSemInferenceV1::CaseBootstrap {
            resamples,
            seed,
            confidence_level,
            interval,
            tail,
        } => {
            if interval != GeneralSemBootstrapIntervalV1::Percentile {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::BootstrapIntervalNotYetExecutable,
                );
            }
            if tail != GeneralSemInferenceTailV1::TwoSided {
                return Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapTailNotYetExecutable);
            }
            if recipe.settings.bootstrap_samples != resamples {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::BootstrapSamplesMismatch {
                        expected: resamples,
                        found: recipe.settings.bootstrap_samples,
                    },
                );
            }
            if recipe.settings.seed != seed {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::BootstrapSeedMismatch {
                        expected: seed,
                        found: recipe.settings.seed,
                    },
                );
            }
            if recipe.settings.confidence_level.to_bits() != confidence_level.to_bits() {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::BootstrapConfidenceLevelMismatch {
                        expected: confidence_level,
                        found: recipe.settings.confidence_level,
                    },
                );
            }
            if recipe.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided {
                return Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapRecipeTailMismatch);
            }
            if recipe.settings.studentized_inner_samples != 0 {
                return Err(
                    GeneralSemPlsRecipeCompilationErrorV1::BootstrapStudentizedSamplesMismatch {
                        found: recipe.settings.studentized_inner_samples,
                    },
                );
            }
        }
    }
    Ok(())
}

fn ensure_higher_order_recipe_scope(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    let has_higher_order = model
        .derived_terms
        .iter()
        .any(|term| matches!(term, crate::SemDerivedTermV4::HigherOrder { .. }));
    if !has_higher_order {
        return Ok(());
    }
    if recipe.settings.method != AnalysisMethod::PlsPm {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::HigherOrderRequiresPlsPm);
    }
    if recipe.settings.weighting_scheme != WeightingScheme::Path {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::HigherOrderRequiresPathWeighting);
    }
    if recipe.settings.preprocessing != Preprocessing::Standardized {
        return Err(
            GeneralSemPlsRecipeCompilationErrorV1::HigherOrderRequiresStandardizedPreprocessing,
        );
    }
    if recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::HigherOrderRequiresListwiseDeletion);
    }
    if recipe.settings.case_weight_column.is_some() {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::HigherOrderCaseWeightsNotExecutable);
    }
    if recipe.settings.permutation_samples != 0 {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::HigherOrderPermutationNotExecutable);
    }
    Ok(())
}

/// Produces the exact point-scoring recipe that corresponds to a validated
/// General SEM request. The returned recipe is the only recipe that may be
/// paired with `CompiledGeneralSemPlsRecipeV1::base_artifact()`; the outer
/// General SEM artifact and config digest remain authoritative for inference.
pub fn project_general_sem_pls_base_recipe_v1(
    recipe: &AnalysisRecipeV4,
) -> Result<AnalysisRecipeV4, GeneralSemPlsRecipeCompilationErrorV1> {
    let config = recipe
        .general_sem_config
        .as_ref()
        .ok_or(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)?;
    ensure_compilation_scope(recipe, config)?;
    let mut base_recipe = recipe.clone();
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        base_recipe.settings.bootstrap_samples = 0;
        base_recipe.settings.studentized_inner_samples = 0;
    }
    Ok(base_recipe)
}

/// Projects an interaction- or HOC-bearing General SEM recipe to the exact
/// ordinary PLS model used only for shared lower-order construct scoring. The
/// source recipe/model remain authoritative in the outer compiled artifact.
pub fn project_general_sem_pls_stage_one_recipe_v1(
    recipe: &AnalysisRecipeV4,
    source_model: &SemModelV4,
) -> Result<(AnalysisRecipeV4, SemModelV4), GeneralSemPlsRecipeCompilationErrorV1> {
    let mut stage_one_recipe = project_general_sem_pls_base_recipe_v1(recipe)?;
    let has_higher_order = source_model
        .derived_terms
        .iter()
        .any(|term| matches!(term, crate::SemDerivedTermV4::HigherOrder { .. }));
    let projection = if has_higher_order {
        compile_pls_higher_order_lower_order_projection_v1(source_model)
            .map_err(CompiledPlsPlanV3Error::from)?
    } else {
        compile_pls_stage_one_projection_v3(source_model).map_err(CompiledPlsPlanV3Error::from)?
    };
    if projection.source_scientific_sha256() == projection.projected_scientific_sha256() {
        return Ok((stage_one_recipe, source_model.clone()));
    }
    let stage_one_model = projection.projected_model().clone();
    stage_one_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: projection.projected_scientific_sha256().to_string(),
        model: stage_one_model.clone(),
    };
    stage_one_recipe
        .ensure_valid()
        .map_err(RecipeV4CompilationError::from)?;
    Ok((stage_one_recipe, stage_one_model))
}

fn compiler_version_for_plan(
    config: &crate::GeneralSemConfigV1,
    plan: &CompiledPlsPlanV3,
) -> &'static str {
    if !plan.higher_order_stage_plans().is_empty() {
        return match config.inference {
            GeneralSemInferenceV1::None => {
                GENERAL_SEM_PLS_HIGHER_ORDER_POINT_RECIPE_COMPILER_VERSION_V1
            }
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
            }
        };
    }
    if !plan.two_way_interactions().is_empty() {
        return match config.inference {
            GeneralSemInferenceV1::None => {
                GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_RECIPE_COMPILER_VERSION_V1
            }
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
            }
        };
    }
    match config.inference {
        GeneralSemInferenceV1::None => GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1,
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            GENERAL_SEM_PLS_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
        }
    }
}

fn ensure_capabilities_available(
    config: &crate::GeneralSemConfigV1,
    plan: &CompiledPlsPlanV3,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    let mut expected_cells = vec![capability_cell_for_plan(plan)];
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        expected_cells.push(if !plan.higher_order_stage_plans().is_empty() {
            pls_general_higher_order_bootstrap_capability_cell_v1()
        } else if plan.two_way_interactions().is_empty() {
            pls_general_bootstrap_capability_cell_v1()
        } else {
            pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        });
    }
    let registry = CapabilityRegistryV2::embedded().map_err(|error| {
        GeneralSemPlsRecipeCompilationErrorV1::CapabilityRegistry(error.to_string())
    })?;
    for expected in expected_cells {
        let available = registry.option_cells().any(|cell| {
            cell.capability_id == expected.capability_id
                && cell.cell_id == expected.cell_id
                && cell.capability_version == expected.capability_version
                && (cell.standard_available() || cell.labs_available())
        });
        if !available {
            return Err(GeneralSemPlsRecipeCompilationErrorV1::CapabilityUnavailable);
        }
    }
    Ok(())
}

fn capability_cell_for_plan(plan: &CompiledPlsPlanV3) -> CapabilityCellReferenceV2 {
    if !plan.higher_order_stage_plans().is_empty() {
        pls_general_higher_order_point_capability_cell_v1()
    } else if plan.two_way_interactions().is_empty() {
        pls_general_recursive_effects_capability_cell_v1()
    } else {
        pls_general_multiple_moderation_point_capability_cell_v1()
    }
}

fn ensure_interaction_compilation_scope(
    config: &crate::GeneralSemConfigV1,
    plan: &CompiledPlsPlanV3,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    if plan.two_way_interactions().is_empty() {
        return Ok(());
    }
    if !config.requested_effect_estimands.is_empty() {
        return Err(
            GeneralSemPlsRecipeCompilationErrorV1::InteractionRequestedEffectsNotYetExecutable,
        );
    }
    if !plan.topology().specific_directed_paths().is_empty() {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::ModeratedMediationNotYetExecutable);
    }
    Ok(())
}

fn resolved_model_from_recipe<'a>(
    recipe: &'a AnalysisRecipeV4,
    resolved_model: Option<&'a SemModelV4>,
) -> Result<&'a SemModelV4, RecipeV4CompilationError> {
    match &recipe.model_binding {
        crate::AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => Ok(model),
        crate::AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference { .. } => {
            resolved_model.ok_or(RecipeV4CompilationError::UnresolvedModelReference)
        }
        crate::AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => {
            Err(RecipeV4CompilationError::LegacyEstimandUnspecified)
        }
    }
}

#[derive(Serialize)]
struct GeneralSemArtifactIdentityV1<'a> {
    schema_version: u32,
    compiler_version: &'a str,
    capability_cell: &'a CapabilityCellReferenceV2,
    recipe_analytical_sha256: &'a str,
    base_analytical_identity_sha256: &'a str,
    plan_sha256: &'a str,
    general_sem_config_sha256: &'a str,
}

fn hash_serializable<T: Serialize>(
    value: &T,
) -> Result<String, GeneralSemPlsRecipeCompilationErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| GeneralSemPlsRecipeCompilationErrorV1::Serialization(error.to_string()))
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
        AnalysisRecipeModelBindingV4, AnalysisSettings, CompiledPlsInteractionV3Error, Construct,
        GeneralSemConditionalEffectProbeV1, GeneralSemConditionalProbeValuesV1, GeneralSemConfigV1,
        GeneralSemEffectEstimandV1, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
        SemDerivedTermV4, SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
        StructuralPath, StructuralRelationRoleV4, confirm_legacy_recipe_estimand_v4,
        migrate_analysis_recipe_to_v4_pending,
    };
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn recipe_and_model() -> (AnalysisRecipeV4, SemModelV4) {
        let constructs = ["x", "m1", "m2", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [
            ("x", "m1"),
            ("x", "m2"),
            ("x", "y"),
            ("m1", "m2"),
            ("m1", "y"),
            ("m2", "y"),
        ]
        .into_iter()
        .map(|(source, target)| StructuralPath {
            source: source.into(),
            target: target.into(),
        })
        .collect();
        let model = ModelSpec {
            id: Uuid::from_u128(0x5031_5303),
            name: "General recursive fixture".into(),
            constructs,
            paths,
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let legacy = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_5304),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "dataset-fingerprint".into(),
            model: model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&legacy).unwrap();
        let (mut recipe, model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        (recipe, model)
    }

    fn add_compiler_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == focal_predictor_id && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        let output = format!("derived:{interaction_id}");
        let effect_relation = format!("relation:{interaction_id}:effect");
        let effect_parameter = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: effect_relation,
            source: output.clone(),
            target: "construct:y".into(),
            parameter: effect_parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: effect_parameter,
            label: format!("{interaction_id} -> Y"),
            target: SemParameterTargetV4::Regression {
                source: output.clone(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.into(),
            output,
            operands: vec![focal_predictor_id.into(), moderator_id.into()],
            focal_relation,
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
        model.ensure_valid().unwrap();
    }

    fn interaction_recipe_and_model_for_layout(
        different_focal: bool,
    ) -> (AnalysisRecipeV4, SemModelV4) {
        let source_model = ModelSpec {
            id: Uuid::from_u128(0x5031_53a0),
            name: "Compiler multiple moderation fixture".into(),
            constructs: ["x", "w", "z", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.into(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [("x", "y"), ("w", "y"), ("z", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.into(),
                    target: target.into(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let legacy = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x5031_53a1),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "dataset-fingerprint".into(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&legacy).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source_model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        add_compiler_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        add_compiler_interaction(
            &mut model,
            if different_focal {
                "interaction:z_by_w"
            } else {
                "interaction:x_by_z"
            },
            if different_focal {
                "construct:z"
            } else {
                "construct:x"
            },
            if different_focal {
                "construct:w"
            } else {
                "construct:z"
            },
        );
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        recipe.ensure_valid().unwrap();
        (recipe, model)
    }

    fn interaction_recipe_and_model() -> (AnalysisRecipeV4, SemModelV4) {
        interaction_recipe_and_model_for_layout(false)
    }

    fn rebind_recipe_model(recipe: &mut AnalysisRecipeV4, model: &SemModelV4) {
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
    }

    fn configure_percentile_bootstrap(recipe: &mut AnalysisRecipeV4) {
        let inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 7,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        recipe.general_sem_config.as_mut().unwrap().inference = inference;
        recipe.settings.bootstrap_samples = 500;
        recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.seed = 7;
        recipe.settings.confidence_level = 0.95;
    }

    #[test]
    fn compilation_is_additive_deterministic_and_tamper_detecting() {
        let (recipe, model) = recipe_and_model();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        assert_eq!(artifact.schema_version(), 1);
        assert_eq!(artifact.plan().schema_version(), 3);
        assert_eq!(
            artifact.plan().base_plan(),
            match artifact.base_artifact().plan() {
                CompiledRecipePlanV4::PlsPlanV2 { plan } => plan,
                _ => unreachable!(),
            }
        );
        validate_compiled_general_sem_pls_recipe_v1(&artifact, &recipe, Some(&model)).unwrap();
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model))
                .unwrap()
                .artifact_identity_sha256(),
            artifact.artifact_identity_sha256()
        );

        let mut tampered = artifact;
        tampered.artifact_identity_sha256 = "0".repeat(64);
        assert_eq!(
            validate_compiled_general_sem_pls_recipe_v1(&tampered, &recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::ArtifactMismatch)
        );
    }

    #[test]
    fn multiple_moderation_compilation_projects_one_shared_stage_one_model() {
        let (recipe, model) = interaction_recipe_and_model();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        assert_eq!(artifact.plan().two_way_interactions().len(), 2);
        assert_eq!(
            artifact.compiler_version(),
            GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_RECIPE_COMPILER_VERSION_V1
        );
        assert_eq!(
            artifact.capability_cell(),
            &pls_general_multiple_moderation_point_capability_cell_v1()
        );
        let (stage_one_recipe, stage_one_model) =
            project_general_sem_pls_stage_one_recipe_v1(&recipe, &model).unwrap();
        assert!(stage_one_model.derived_terms.is_empty());
        assert_eq!(
            stage_one_model.scientific_sha256().unwrap(),
            artifact
                .plan()
                .stage_one_projection_scientific_sha256()
                .unwrap()
        );
        assert_ne!(
            artifact.plan().scientific_hash(),
            stage_one_model.scientific_sha256().unwrap()
        );
        validate_compiled_analysis_recipe_v4(
            artifact.base_artifact(),
            &stage_one_recipe,
            Some(&stage_one_model),
        )
        .unwrap();
        validate_compiled_general_sem_pls_recipe_v1(&artifact, &recipe, Some(&model)).unwrap();
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model))
                .unwrap()
                .artifact_identity_sha256(),
            artifact.artifact_identity_sha256()
        );
    }

    #[test]
    fn simultaneous_moderation_bootstrap_compiles_for_same_and_different_focal_paths() {
        for different_focal in [false, true] {
            let (mut recipe, model) = interaction_recipe_and_model_for_layout(different_focal);
            let point_bytes = serde_json::to_vec(
                &compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap(),
            )
            .unwrap();
            configure_percentile_bootstrap(&mut recipe);

            let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
            assert_eq!(
                artifact.compiler_version(),
                GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
            );
            assert_eq!(
                artifact.capability_cell(),
                &pls_general_multiple_moderation_point_capability_cell_v1()
            );
            assert_eq!(artifact.plan().two_way_interactions().len(), 2);
            validate_compiled_general_sem_pls_recipe_v1(&artifact, &recipe, Some(&model)).unwrap();
            assert_eq!(
                serde_json::to_vec(
                    &compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap()
                )
                .unwrap(),
                serde_json::to_vec(&artifact).unwrap()
            );

            let (point_recipe, point_model) =
                interaction_recipe_and_model_for_layout(different_focal);
            assert_eq!(
                serde_json::to_vec(
                    &compile_general_sem_pls_recipe_v1(&point_recipe, Some(&point_model)).unwrap()
                )
                .unwrap(),
                point_bytes,
                "compiling the supplemental bootstrap cell must not change point-artifact bytes"
            );
        }
    }

    #[test]
    fn interaction_bootstrap_keeps_interval_tail_probe_effect_and_chain_boundaries_typed() {
        let (mut recipe, model) = interaction_recipe_and_model();
        configure_percentile_bootstrap(&mut recipe);
        if let GeneralSemInferenceV1::CaseBootstrap { interval, .. } =
            &mut recipe.general_sem_config.as_mut().unwrap().inference
        {
            *interval = GeneralSemBootstrapIntervalV1::Bca;
        }
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapIntervalNotYetExecutable)
        );

        configure_percentile_bootstrap(&mut recipe);
        if let GeneralSemInferenceV1::CaseBootstrap { tail, .. } =
            &mut recipe.general_sem_config.as_mut().unwrap().inference
        {
            *tail = GeneralSemInferenceTailV1::OneSidedUpper;
        }
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapTailNotYetExecutable)
        );

        let (mut recipe, model) = interaction_recipe_and_model();
        recipe
            .general_sem_config
            .as_mut()
            .unwrap()
            .conditional_effect_probes = vec![GeneralSemConditionalEffectProbeV1 {
            probe_id: "probe:w".into(),
            moderator_id: "construct:w".into(),
            values: GeneralSemConditionalProbeValuesV1::DataDerivedMeanPlusMinusOneSd,
        }];
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::ConditionalProbesNotYetExecutable)
        );

        let (mut recipe, model) = interaction_recipe_and_model();
        recipe
            .general_sem_config
            .as_mut()
            .unwrap()
            .requested_effect_estimands = vec![GeneralSemEffectEstimandV1::TotalEffect {
            estimand_id: "effect:x_to_y".into(),
            source_id: "construct:x".into(),
            target_id: "construct:y".into(),
        }];
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::InteractionRequestedEffectsNotYetExecutable)
        );

        let (mut recipe, mut model) = interaction_recipe_and_model();
        let chain_parameter = "parameter:chain:x_to_w".to_string();
        model.relations.push(SemRelationV4::Structural {
            id: "relation:chain:x_to_w".into(),
            source: "construct:x".into(),
            target: "construct:w".into(),
            parameter: chain_parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: chain_parameter,
            label: "X -> W".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:x".into(),
                target: "construct:w".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        rebind_recipe_model(&mut recipe, &model);
        recipe.ensure_valid().unwrap();
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::ModeratedMediationNotYetExecutable)
        );
    }

    #[test]
    fn interaction_bootstrap_keeps_order_method_hierarchy_and_derived_scope_boundaries_typed() {
        let (mut recipe, mut model) = interaction_recipe_and_model();
        let SemDerivedTermV4::InteractionV2 {
            operands,
            hierarchy_policy,
            ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        operands.push("construct:z".into());
        // Weak hierarchy keeps this deliberately unsupported three-way term
        // scientifically valid long enough to exercise the compiler's typed
        // interaction-order boundary.
        *hierarchy_policy = InteractionHierarchyPolicyV2::Weak;
        model.ensure_valid().unwrap();
        rebind_recipe_model(&mut recipe, &model);
        configure_percentile_bootstrap(&mut recipe);
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::PlsPlanV3(
                CompiledPlsPlanV3Error::Interaction(
                    CompiledPlsInteractionV3Error::UnsupportedInteractionOrder { .. }
                )
            ))
        ));

        let (mut recipe, mut model) = interaction_recipe_and_model();
        let SemDerivedTermV4::InteractionV2 { method, .. } = &mut model.derived_terms[0] else {
            unreachable!()
        };
        *method = InteractionMethodV4::Orthogonalizing;
        model.ensure_valid().unwrap();
        rebind_recipe_model(&mut recipe, &model);
        configure_percentile_bootstrap(&mut recipe);
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::PlsPlanV3(
                CompiledPlsPlanV3Error::Interaction(
                    CompiledPlsInteractionV3Error::UnsupportedInteractionMethod { .. }
                )
            ))
        ));

        let (mut recipe, mut model) = interaction_recipe_and_model();
        let SemDerivedTermV4::InteractionV2 {
            hierarchy_policy, ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        *hierarchy_policy = InteractionHierarchyPolicyV2::Weak;
        model.ensure_valid().unwrap();
        rebind_recipe_model(&mut recipe, &model);
        configure_percentile_bootstrap(&mut recipe);
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::PlsPlanV3(
                CompiledPlsPlanV3Error::Interaction(
                    CompiledPlsInteractionV3Error::UnsupportedInteractionHierarchy { .. }
                )
            ))
        ));

        let (mut recipe, mut model) = interaction_recipe_and_model();
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x_squared".into(),
            label: "X squared".into(),
        });
        model.derived_terms.push(SemDerivedTermV4::Polynomial {
            id: "polynomial:x_squared".into(),
            output: "derived:x_squared".into(),
            source: "construct:x".into(),
            degree: 2,
        });
        model.ensure_valid().unwrap();
        rebind_recipe_model(&mut recipe, &model);
        configure_percentile_bootstrap(&mut recipe);
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::PlsPlanV3(
                CompiledPlsPlanV3Error::Interaction(
                    CompiledPlsInteractionV3Error::UnsupportedDerivedTerm { .. }
                )
            ))
        ));
    }

    #[test]
    fn absent_config_is_explicitly_blocked_and_point_none_remains_compilable() {
        let (mut recipe, model) = recipe_and_model();
        compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        recipe.general_sem_config = None;
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)
        ));
    }

    #[test]
    fn coherent_percentile_two_sided_bootstrap_compiles_deterministically() {
        let (mut recipe, model) = recipe_and_model();
        configure_percentile_bootstrap(&mut recipe);

        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        assert_eq!(
            artifact.compiler_version(),
            GENERAL_SEM_PLS_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
        );
        let projected = project_general_sem_pls_base_recipe_v1(&recipe).unwrap();
        assert_eq!(projected.settings.bootstrap_samples, 0);
        assert_eq!(projected.settings.studentized_inner_samples, 0);
        assert_eq!(projected.settings.seed, 7);
        assert_eq!(
            projected.settings.confidence_level.to_bits(),
            0.95_f64.to_bits()
        );
        assert_eq!(projected.general_sem_config, recipe.general_sem_config);
        validate_compiled_analysis_recipe_v4(artifact.base_artifact(), &projected, Some(&model))
            .unwrap();
        validate_compiled_general_sem_pls_recipe_v1(&artifact, &recipe, Some(&model)).unwrap();
        assert_eq!(
            artifact.plan().general_sem_config_sha256(),
            artifact.general_sem_config_sha256()
        );
        assert_eq!(
            artifact.base_artifact().receipt().compiler_target(),
            RecipeV4CompilerTarget::PlsPlanV2
        );
        assert_eq!(
            artifact.artifact_identity_sha256(),
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model))
                .unwrap()
                .artifact_identity_sha256()
        );
    }

    #[test]
    fn mediation_point_cell_rejects_a_direct_only_recursive_model() {
        let (mut recipe, mut model) = recipe_and_model();
        model.relations.retain(|relation| match relation {
            crate::SemRelationV4::Structural { source, target, .. } => {
                source == "construct:x" && target == "construct:y"
            }
            _ => true,
        });
        let retained_parameter_ids = model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                crate::SemRelationV4::Structural { parameter, .. } => Some(parameter.as_str()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        model.parameters.retain(|parameter| {
            retained_parameter_ids.contains(parameter.id())
                || !matches!(
                    parameter.target(),
                    crate::SemParameterTargetV4::Regression { .. }
                )
        });
        model.ensure_valid().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };

        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::MediationRequiresIndirectPath { found: 0 })
        );
    }

    #[test]
    fn exact_multiple_mediation_bootstrap_cell_rejects_a_single_indirect_path() {
        let (mut recipe, mut model) = recipe_and_model();
        configure_percentile_bootstrap(&mut recipe);
        model.relations.retain(|relation| match relation {
            crate::SemRelationV4::Structural { source, target, .. } => {
                !(source == "construct:x" && target == "construct:m2")
                    && !(source == "construct:m1" && target == "construct:m2")
                    && !(source == "construct:m2" && target == "construct:y")
            }
            _ => true,
        });
        let retained_parameter_ids = model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                crate::SemRelationV4::Structural { parameter, .. } => Some(parameter.as_str()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        model.parameters.retain(|parameter| {
            retained_parameter_ids.contains(parameter.id())
                || !matches!(
                    parameter.target(),
                    crate::SemParameterTargetV4::Regression { .. }
                )
        });
        model.ensure_valid().unwrap();
        let scientific_sha256 = model.scientific_sha256().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256,
        };

        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::MultipleMediationRequiresTwoIndirectPaths {
                    found: 1,
                }
            )
        );
    }

    #[test]
    fn bca_and_one_sided_bootstrap_remain_typed_blocked() {
        let (mut recipe, model) = recipe_and_model();
        configure_percentile_bootstrap(&mut recipe);
        if let GeneralSemInferenceV1::CaseBootstrap { interval, .. } =
            &mut recipe.general_sem_config.as_mut().unwrap().inference
        {
            *interval = GeneralSemBootstrapIntervalV1::Bca;
        }
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapIntervalNotYetExecutable)
        );

        configure_percentile_bootstrap(&mut recipe);
        if let GeneralSemInferenceV1::CaseBootstrap { tail, .. } =
            &mut recipe.general_sem_config.as_mut().unwrap().inference
        {
            *tail = GeneralSemInferenceTailV1::OneSidedUpper;
        }
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapTailNotYetExecutable)
        );
    }

    #[test]
    fn bootstrap_recipe_settings_must_exactly_match_general_sem_inference() {
        let (mut recipe, model) = recipe_and_model();
        configure_percentile_bootstrap(&mut recipe);

        let mut mismatch = recipe.clone();
        mismatch.settings.bootstrap_samples = 501;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&mismatch, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::BootstrapSamplesMismatch {
                    expected: 500,
                    found: 501,
                }
            )
        );

        let mut mismatch = recipe.clone();
        mismatch.settings.seed = 8;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&mismatch, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::BootstrapSeedMismatch {
                    expected: 7,
                    found: 8,
                }
            )
        );

        let mut mismatch = recipe.clone();
        mismatch.settings.confidence_level = 0.90;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&mismatch, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::BootstrapConfidenceLevelMismatch {
                    expected: 0.95,
                    found: 0.90,
                }
            )
        );

        let mut mismatch = recipe.clone();
        mismatch.settings.bootstrap_test_tail = PlsBootstrapTestTail::OneSidedGreater;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&mismatch, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::BootstrapRecipeTailMismatch)
        );

        let mut mismatch = recipe;
        mismatch.settings.studentized_inner_samples = 99;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&mismatch, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::BootstrapStudentizedSamplesMismatch {
                    found: 99,
                }
            )
        );
    }

    #[test]
    fn point_none_rejects_orphaned_bootstrap_controls() {
        let (recipe, model) = recipe_and_model();

        let mut orphaned = recipe.clone();
        orphaned.settings.bootstrap_samples = 2;
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&orphaned, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::PointInferenceBootstrapSamplesMismatch {
                    found: 2
                }
            )
        ));

        let mut orphaned = recipe.clone();
        orphaned.settings.bootstrap_test_tail = PlsBootstrapTestTail::OneSidedLess;
        assert_eq!(
            compile_general_sem_pls_recipe_v1(&orphaned, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::PointInferenceBootstrapTailMismatch)
        );

        let mut orphaned = recipe;
        orphaned.settings.studentized_inner_samples = 99;
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&orphaned, Some(&model)),
            Err(
                GeneralSemPlsRecipeCompilationErrorV1::PointInferenceStudentizedSamplesMismatch {
                    found: 99
                }
            )
        ));
    }
}
