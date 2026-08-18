use crate::{
    AnalysisRecipeV4, CapabilityCellReferenceV2, CapabilityRegistryV2,
    CompiledAnalysisRecipeV4, CompiledPlsPlanV3, CompiledPlsPlanV3Error, CompiledRecipePlanV4,
    GeneralSemInferenceV1,
    GeneralSemSpecificPathLimitBehaviorV1, RecipeV4CompilationError, RecipeV4CompilerTarget,
    SemModelV4, compile_analysis_recipe_v4, compile_pls_plan_v3,
    validate_compiled_analysis_recipe_v4,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1: u32 = 1;
pub const GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1: &str =
    "recipe_v4_to_compiled_pls_plan_v3_v1";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_ID_V1: &str = "smartpls.mediation";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CELL_ID_V1: &str = "qpls3.pls.mediation";
pub const PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_VERSION_V1: &str = "pls_mediation_v1";

pub fn pls_general_recursive_effects_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_RECURSIVE_EFFECTS_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_RECURSIVE_EFFECTS_CAPABILITY_VERSION_V1.into(),
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

    pub fn artifact_identity_sha256(&self) -> &str {
        &self.artifact_identity_sha256
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum GeneralSemPlsRecipeCompilationErrorV1 {
    #[error("General SEM PLS compilation requires general_sem_config")]
    MissingGeneralSemConfig,
    #[error("General SEM PLS point-estimation v1 does not yet execute conditional probes")]
    ConditionalProbesNotYetExecutable,
    #[error("General SEM PLS point-estimation v1 does not yet execute resampling inference")]
    InferenceNotYetExecutable,
    #[error("General SEM PLS v1 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotYetExecutable,
    #[error("Capability Registry V2 cannot authorize the General SEM mediation cell: {0}")]
    CapabilityRegistry(String),
    #[error("the exact General SEM mediation capability cell is not available in Labs or Standard")]
    CapabilityUnavailable,
    #[error(transparent)]
    BaseCompilation(#[from] RecipeV4CompilationError),
    #[error(transparent)]
    PlsPlanV3(#[from] CompiledPlsPlanV3Error),
    #[error("compiled PLS v3 base plan differs from the proven recipe-v4 PLS v2 plan")]
    BasePlanMismatch,
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
    ensure_point_estimation_scope(config)?;
    ensure_capability_available()?;

    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let base_artifact = compile_analysis_recipe_v4(
        recipe,
        resolved_model,
        target,
        target.capability_cell_for_method(recipe.settings.method),
    )?;
    let model = resolved_model_from_recipe(recipe, resolved_model)?;
    let plan = compile_pls_plan_v3(model, config)?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan: base_plan } = base_artifact.plan() else {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::BasePlanMismatch);
    };
    if plan.base_plan() != base_plan {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::BasePlanMismatch);
    }

    let capability_cell = pls_general_recursive_effects_capability_cell_v1();
    let general_sem_config_sha256 = plan.general_sem_config_sha256().to_string();
    let artifact_identity_sha256 = hash_serializable(&GeneralSemArtifactIdentityV1 {
        schema_version: GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1,
        compiler_version: GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1,
        capability_cell: &capability_cell,
        base_analytical_identity_sha256: base_artifact
            .receipt()
            .analytical_identity_sha256(),
        plan_sha256: &plan.deterministic_sha256(),
        general_sem_config_sha256: &general_sem_config_sha256,
    })?;
    Ok(CompiledGeneralSemPlsRecipeV1 {
        schema_version: GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1,
        compiler_version: GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1.into(),
        capability_cell,
        base_artifact,
        plan,
        general_sem_config_sha256,
        artifact_identity_sha256,
    })
}

pub fn validate_compiled_general_sem_pls_recipe_v1(
    artifact: &CompiledGeneralSemPlsRecipeV1,
    recipe: &AnalysisRecipeV4,
    resolved_model: Option<&SemModelV4>,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    if artifact.schema_version != GENERAL_SEM_PLS_RECIPE_ARTIFACT_SCHEMA_VERSION_V1
        || artifact.compiler_version != GENERAL_SEM_PLS_RECIPE_COMPILER_VERSION_V1
        || artifact.capability_cell != pls_general_recursive_effects_capability_cell_v1()
        || artifact.general_sem_config_sha256 != artifact.plan.general_sem_config_sha256()
        || !is_lowercase_sha256(&artifact.artifact_identity_sha256)
    {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::InvalidArtifactContract);
    }
    validate_compiled_analysis_recipe_v4(
        artifact.base_artifact(),
        recipe,
        resolved_model,
    )?;
    let expected = compile_general_sem_pls_recipe_v1(recipe, resolved_model)?;
    if *artifact != expected {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::ArtifactMismatch);
    }
    Ok(())
}

fn ensure_point_estimation_scope(
    config: &crate::GeneralSemConfigV1,
) -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    if !config.conditional_effect_probes.is_empty() {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::ConditionalProbesNotYetExecutable);
    }
    if config.inference != GeneralSemInferenceV1::None {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::InferenceNotYetExecutable);
    }
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(
            GeneralSemPlsRecipeCompilationErrorV1::LazySpecificPathMaterializationNotYetExecutable,
        );
    }
    Ok(())
}

fn ensure_capability_available() -> Result<(), GeneralSemPlsRecipeCompilationErrorV1> {
    let expected = pls_general_recursive_effects_capability_cell_v1();
    let registry = CapabilityRegistryV2::embedded().map_err(|error| {
        GeneralSemPlsRecipeCompilationErrorV1::CapabilityRegistry(error.to_string())
    })?;
    let available = registry.option_cells().any(|cell| {
        cell.capability_id == expected.capability_id
            && cell.cell_id == expected.cell_id
            && cell.capability_version == expected.capability_version
            && (cell.standard_available() || cell.labs_available())
    });
    if !available {
        return Err(GeneralSemPlsRecipeCompilationErrorV1::CapabilityUnavailable);
    }
    Ok(())
}

fn resolved_model_from_recipe<'a>(
    recipe: &'a AnalysisRecipeV4,
    resolved_model: Option<&'a SemModelV4>,
) -> Result<&'a SemModelV4, RecipeV4CompilationError> {
    match &recipe.model_binding {
        crate::AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => Ok(model),
        crate::AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference { .. } => resolved_model
            .ok_or(RecipeV4CompilationError::UnresolvedModelReference),
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
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4,
        AnalysisSettings, Construct, GeneralSemConfigV1, LegacyBasicModelInterpretationV4,
        MeasurementMode, MethodConfig, ModelSpec, StructuralPath,
        confirm_legacy_recipe_estimand_v4, migrate_analysis_recipe_to_v4_pending,
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

    #[test]
    fn compilation_is_additive_deterministic_and_tamper_detecting() {
        let (recipe, model) = recipe_and_model();
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model)).unwrap();
        assert_eq!(artifact.schema_version(), 1);
        assert_eq!(artifact.plan().schema_version(), 3);
        assert_eq!(artifact.plan().base_plan(), match artifact.base_artifact().plan() {
            CompiledRecipePlanV4::PlsPlanV2 { plan } => plan,
            _ => unreachable!(),
        });
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
    fn absent_config_and_unimplemented_inference_are_explicitly_blocked() {
        let (mut recipe, model) = recipe_and_model();
        recipe.general_sem_config = None;
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::MissingGeneralSemConfig)
        ));

        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 100,
            seed: 7,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        recipe.general_sem_config = Some(config);
        assert!(matches!(
            compile_general_sem_pls_recipe_v1(&recipe, Some(&model)),
            Err(GeneralSemPlsRecipeCompilationErrorV1::InferenceNotYetExecutable)
        ));
    }
}
