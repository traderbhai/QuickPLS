//! Versioned General SEM execution-authority revision.
//!
//! A revision never rewrites its source archive. The native authority loads an
//! exclusively pinned `general_sem_v1` source, applies one versioned scientific
//! intent, rebinds and recompiles its resident RecipeV4, then publishes a new
//! schema-6 archive through the no-replace writer. Historical results remain
//! only in the immutable source archive.

use super::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectArchiveV6SaveCopyError,
    ProjectError, ProjectModelPayloadV6, ProjectModelRecordV6, load_project_archive_v6_from_file,
    project_archive_v6_save_copy::publish_new_project_archive_v6_document_with_resident_datasets_before_publish,
};
use chrono::{DateTime, Utc};
use qpls_core::{
    AnalysisRecipeModelBindingV4, CapabilityCellReferenceV2,
    CompiledPlsTwoWayModeratedMediationTargetV1, GeneralSemEffectEstimandV1,
    GeneralSemPlsRecipeCompilationErrorV1, InteractionHierarchyPolicyV2, InteractionMethodV4,
    HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4, SemDerivedTermV4, SemModelV4,
    SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
    StructuralRelationRoleV4, capability_cell_reference_identity_v2,
    compile_general_sem_pls_recipe_v1,
    pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1,
    pls_general_multiple_moderation_bootstrap_capability_cell_v1,
    pls_general_multiple_moderation_point_capability_cell_v1,
    pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1, sha256_serialized,
    specific_directed_path_identity_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use uuid::Uuid;

pub const GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_SCHEMA_VERSION: u32 = 1;
pub const GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_LAYOUT_KEY: &str =
    "general_sem_execution_authority_revision_v1";
pub const GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_SCHEMA_VERSION: u32 = 2;
pub const GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY: &str =
    "general_sem_execution_authority_revision_v2";
pub const GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1: &str = "native_general_sem_pls_labs_v1";
pub const GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1: &str =
    "native_general_sem_pls_standard_v1";
const MAX_SAFE_REVISION_NUMBER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemRevisionGenerationV1 {
    GeneralSemV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemRevisionInteractionMethodV1 {
    TwoStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemRevisionHierarchyPolicyV1 {
    Strong,
}

/// Additive scientific mutations accepted by revision schema v1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemExecutionAuthorityRevisionIntentV1 {
    AddGeneralSemInteractionV2 {
        intent_version: u32,
        sem_generation: GeneralSemRevisionGenerationV1,
        label: String,
        operands: [String; 2],
        focal_relation: String,
        outcome: String,
        method: GeneralSemRevisionInteractionMethodV1,
        hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1,
    },
    AddHigherOrder {
        term_id: String,
        output_id: String,
        label: String,
        components: Vec<String>,
        approach: HigherOrderConstructionApproachV4,
        measurement_type: HigherOrderMeasurementTypeV4,
        initial_path: GeneralSemRevisionHigherOrderPathV1,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemRevisionHigherOrderPathV1 {
    pub relation_id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthoritySourcePinV1 {
    pub project_id: Uuid,
    pub model_id: String,
    pub model_document_sha256: String,
    pub model_scientific_sha256: String,
    pub recipe_id: Uuid,
    pub recipe_document_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthorityRevisionIdentityV1 {
    pub project_id: Uuid,
    pub project_name: String,
    pub created_at: DateTime<Utc>,
    pub model_id: String,
    pub model_name: String,
    pub recipe_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthorityRevisionRequestV1 {
    pub source: GeneralSemExecutionAuthoritySourcePinV1,
    pub revision: GeneralSemExecutionAuthorityRevisionIdentityV1,
    pub intent: GeneralSemExecutionAuthorityRevisionIntentV1,
    pub expected_capability_cell: CapabilityCellReferenceV2,
    pub recipe_execution_surface: String,
}

/// Revision-v2 changes no hidden estimator state. It forks the exact resident
/// SemModelV4 identity and records one already-authored two-relation path in
/// the resident GeneralSemConfigV1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemExecutionAuthorityRevisionIntentV2 {
    SelectTwoWayModeratedMediationPath {
        intent_version: u32,
        sem_generation: GeneralSemRevisionGenerationV1,
        estimand_id: String,
        ordered_relation_ids: [String; 2],
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthorityRevisionRequestV2 {
    pub source: GeneralSemExecutionAuthoritySourcePinV1,
    pub revision: GeneralSemExecutionAuthorityRevisionIdentityV1,
    pub intent: GeneralSemExecutionAuthorityRevisionIntentV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneralSemRevisionAuthorityIdentityV1 {
    project_id: Uuid,
    model_id: String,
    model_document_sha256: String,
    model_scientific_sha256: String,
    recipe_id: Uuid,
    recipe_document_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneralSemRevisionCompilationIdentityV1 {
    compiler_version: String,
    capability_cell: CapabilityCellReferenceV2,
    recipe_analytical_sha256: String,
    general_sem_config_sha256: String,
    compiled_plan_sha256: String,
    compiled_artifact_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneralSemExecutionAuthorityRevisionLineageV1 {
    schema_version: u32,
    revision_number: u64,
    parent_revision_number: u64,
    source_archive_sha256: String,
    source_archive_bytes: u64,
    source: GeneralSemRevisionAuthorityIdentityV1,
    revised: GeneralSemRevisionAuthorityIdentityV1,
    compilation: GeneralSemRevisionCompilationIdentityV1,
    intent: GeneralSemExecutionAuthorityRevisionIntentV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneralSemExecutionAuthorityRevisionLineageV2 {
    schema_version: u32,
    revision_number: u64,
    parent_revision_number: u64,
    source_archive_sha256: String,
    source_archive_bytes: u64,
    source: GeneralSemRevisionAuthorityIdentityV1,
    revised: GeneralSemRevisionAuthorityIdentityV1,
    compilation: GeneralSemRevisionCompilationIdentityV1,
    supplemental_capability_cell: CapabilityCellReferenceV2,
    capability_dependencies: Vec<CapabilityCellReferenceV2>,
    compiled_target_sha256: String,
    compiled_target: CompiledPlsTwoWayModeratedMediationTargetV1,
    intent: GeneralSemExecutionAuthorityRevisionIntentV2,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthorityRevisionReceiptV1 {
    pub schema_version: u32,
    pub archive_schema_version: u32,
    pub revision_number: u64,
    pub source_archive_path: String,
    pub source_archive_sha256: String,
    pub source_archive_bytes: u64,
    pub source_verified_unchanged: bool,
    pub source_project_id: Uuid,
    pub source_model_id: String,
    pub source_model_document_sha256: String,
    pub source_model_scientific_sha256: String,
    pub source_recipe_id: Uuid,
    pub source_recipe_document_sha256: String,
    pub destination_archive_path: String,
    pub destination_archive_sha256: String,
    pub destination_archive_bytes: u64,
    pub strict_reopen_validated: bool,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub resident_dataset_id: Uuid,
    pub resident_dataset_fingerprint: String,
    pub resident_model_id: String,
    pub resident_model_document_sha256: String,
    pub resident_model_scientific_sha256: String,
    pub resident_recipe_id: Uuid,
    pub resident_recipe_document_sha256: String,
    pub compiler_version: String,
    pub capability_cell: CapabilityCellReferenceV2,
    pub recipe_analytical_sha256: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub compiled_artifact_identity_sha256: String,
    pub interaction_term_id: String,
    pub interaction_output_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSemExecutionAuthorityRevisionReceiptV2 {
    pub schema_version: u32,
    pub archive_schema_version: u32,
    pub revision_number: u64,
    pub source_archive_path: String,
    pub source_archive_sha256: String,
    pub source_archive_bytes: u64,
    pub source_verified_unchanged: bool,
    pub source_project_id: Uuid,
    pub source_model_id: String,
    pub source_model_document_sha256: String,
    pub source_model_scientific_sha256: String,
    pub source_recipe_id: Uuid,
    pub source_recipe_document_sha256: String,
    pub destination_archive_path: String,
    pub destination_archive_sha256: String,
    pub destination_archive_bytes: u64,
    pub strict_reopen_validated: bool,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub resident_dataset_id: Uuid,
    pub resident_dataset_fingerprint: String,
    pub resident_model_id: String,
    pub resident_model_document_sha256: String,
    pub resident_model_scientific_sha256: String,
    pub resident_recipe_id: Uuid,
    pub resident_recipe_document_sha256: String,
    pub compiler_version: String,
    pub primary_capability_cell: CapabilityCellReferenceV2,
    pub supplemental_capability_cell: CapabilityCellReferenceV2,
    pub capability_dependencies: Vec<CapabilityCellReferenceV2>,
    pub recipe_analytical_sha256: String,
    pub general_sem_config_sha256: String,
    pub compiled_plan_sha256: String,
    pub compiled_artifact_identity_sha256: String,
    pub compiled_target_sha256: String,
    pub compiled_target: CompiledPlsTwoWayModeratedMediationTargetV1,
}

pub type GeneralSemExecutionAuthorityRevisionErrorV2 = GeneralSemExecutionAuthorityRevisionErrorV1;

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemExecutionAuthorityRevisionErrorV1 {
    #[error(
        "General SEM execution-authority revision is supported only by the Windows desktop writer"
    )]
    UnsupportedPlatform,
    #[error("General SEM execution-authority revision request is invalid: {0}")]
    InvalidRequest(String),
    #[error("General SEM execution-authority source does not match its pinned identities: {0}")]
    SourceAuthorityMismatch(String),
    #[error("General SEM execution-authority intent is unsupported: {0}")]
    UnsupportedIntent(String),
    #[error("General SEM execution-authority model is invalid: {0}")]
    Model(String),
    #[error("General SEM execution-authority recipe compilation failed: {0}")]
    Compilation(#[from] GeneralSemPlsRecipeCompilationErrorV1),
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Publication(#[from] ProjectArchiveV6SaveCopyError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Creates a new General SEM revision archive and never mutates `source`.
pub fn create_general_sem_execution_authority_revision_v1(
    source: &Path,
    expected_source_archive_sha256: &str,
    destination: &Path,
    request: GeneralSemExecutionAuthorityRevisionRequestV1,
) -> Result<
    GeneralSemExecutionAuthorityRevisionReceiptV1,
    GeneralSemExecutionAuthorityRevisionErrorV1,
> {
    #[cfg(not(windows))]
    {
        let _ = (source, expected_source_archive_sha256, destination, request);
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedPlatform);
    }
    #[cfg(windows)]
    {
        create_general_sem_execution_authority_revision_windows_v1(
            source,
            expected_source_archive_sha256,
            destination,
            request,
        )
    }
}

/// Forks the exact resident model and RecipeV4 authority, selects one already-
/// authored two-relation SpecificPath, and publishes a new schema-6 archive.
/// Compilation derives the supplemental cell from immutable Registry V2; the
/// revision transaction never mutates Registry state or the source archive.
pub fn create_general_sem_execution_authority_revision_v2(
    source: &Path,
    expected_source_archive_sha256: &str,
    destination: &Path,
    request: GeneralSemExecutionAuthorityRevisionRequestV2,
) -> Result<
    GeneralSemExecutionAuthorityRevisionReceiptV2,
    GeneralSemExecutionAuthorityRevisionErrorV2,
> {
    #[cfg(not(windows))]
    {
        let _ = (source, expected_source_archive_sha256, destination, request);
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedPlatform);
    }
    #[cfg(windows)]
    {
        create_general_sem_execution_authority_revision_windows_v2(
            source,
            expected_source_archive_sha256,
            destination,
            request,
        )
    }
}

#[cfg(windows)]
fn create_general_sem_execution_authority_revision_windows_v1(
    source: &Path,
    expected_source_archive_sha256: &str,
    destination: &Path,
    request: GeneralSemExecutionAuthorityRevisionRequestV1,
) -> Result<
    GeneralSemExecutionAuthorityRevisionReceiptV1,
    GeneralSemExecutionAuthorityRevisionErrorV1,
> {
    validate_request_paths(source, destination, expected_source_archive_sha256)?;
    validate_new_identity(&request)?;

    let mut source_file = open_exclusive_non_reparse_source(source)?;
    let (source_archive_bytes, observed_source_sha256) = sha256_file_handle(&mut source_file)?;
    if observed_source_sha256 != expected_source_archive_sha256 {
        return Err(ProjectArchiveV6SaveCopyError::SourceDigestMismatch {
            expected: expected_source_archive_sha256.to_owned(),
            observed: observed_source_sha256,
        }
        .into());
    }
    let loaded = load_project_archive_v6_from_file(source_file.try_clone()?)?;
    let source_revision_number = validate_source_authority(
        &loaded.document,
        &request.source,
        source_archive_bytes,
        expected_source_archive_sha256,
    )?;
    let revision_number = source_revision_number
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_REVISION_NUMBER)
        .ok_or_else(|| {
            GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                "revision number exceeds the version-1 safe-integer range".into(),
            )
        })?;

    let source_model = sole_source_model(&loaded.document)?;
    let mut revised_model = source_model.clone();
    revised_model.id = request.revision.model_id.clone();
    revised_model.name = request.revision.model_name.clone();
    let (created_term_id, created_output_id) =
        apply_general_sem_revision_intent(&mut revised_model, &request.intent)?;
    revised_model
        .ensure_valid()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    let revised_model_document_sha256 = revised_model
        .model_document_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    let revised_model_scientific_sha256 = revised_model
        .scientific_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;

    let source_recipe = sole_source_recipe(&loaded.document)?;
    let mut revised_recipe = source_recipe.clone();
    revised_recipe.id = request.revision.recipe_id;
    revised_recipe.created_at = request.revision.created_at;
    revised_recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: revised_model.id.clone(),
        scientific_sha256: revised_model_scientific_sha256.clone(),
    };
    revised_recipe.metadata.insert(
        "execution_surface".into(),
        request.recipe_execution_surface.clone(),
    );
    revised_recipe
        .metadata
        .insert("general_sem_generation".into(), "general_sem_v1".into());
    let higher_order_revision = matches!(
        &request.intent,
        GeneralSemExecutionAuthorityRevisionIntentV1::AddHigherOrder { .. }
    );
    let selected_execution_cell = match (
        higher_order_revision,
        revised_recipe
            .general_sem_config
            .as_ref()
            .map(|config| config.inference),
    ) {
        (true, Some(qpls_core::GeneralSemInferenceV1::CaseBootstrap { .. })) => {
            pls_general_higher_order_bootstrap_capability_cell_v1()
        }
        (true, Some(qpls_core::GeneralSemInferenceV1::None)) => {
            pls_general_higher_order_point_capability_cell_v1()
        }
        (false, Some(qpls_core::GeneralSemInferenceV1::CaseBootstrap { .. })) => {
            pls_general_multiple_moderation_bootstrap_capability_cell_v1()
        }
        (false, Some(qpls_core::GeneralSemInferenceV1::None)) => {
            pls_general_multiple_moderation_point_capability_cell_v1()
        }
        (_, None) => {
            return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                "revised RecipeV4 must retain one GeneralSemConfigV1 inference authority".into(),
            ));
        }
    };
    if selected_execution_cell != request.expected_capability_cell {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "expected execution cell differs from the exact revised RecipeV4 inference selection"
                .into(),
        ));
    }
    let compiled = compile_general_sem_pls_recipe_v1(&revised_recipe, Some(&revised_model))?;
    let expected_primary_cell = if higher_order_revision {
        pls_general_higher_order_point_capability_cell_v1()
    } else {
        pls_general_multiple_moderation_point_capability_cell_v1()
    };
    if compiled.capability_cell() != &expected_primary_cell {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "revised compilation primary capability differs from the exact scientific editor intent".into(),
        ));
    }
    let revised_recipe_document_sha256 = sha256_serialized(&revised_recipe);
    let compilation = GeneralSemRevisionCompilationIdentityV1 {
        compiler_version: compiled.compiler_version().to_owned(),
        capability_cell: compiled.capability_cell().clone(),
        recipe_analytical_sha256: compiled.recipe_analytical_sha256().to_owned(),
        general_sem_config_sha256: compiled.general_sem_config_sha256().to_owned(),
        compiled_plan_sha256: compiled.plan().deterministic_sha256(),
        compiled_artifact_identity_sha256: compiled.artifact_identity_sha256().to_owned(),
    };

    let source_identity = GeneralSemRevisionAuthorityIdentityV1 {
        project_id: request.source.project_id,
        model_id: request.source.model_id.clone(),
        model_document_sha256: request.source.model_document_sha256.clone(),
        model_scientific_sha256: request.source.model_scientific_sha256.clone(),
        recipe_id: request.source.recipe_id,
        recipe_document_sha256: request.source.recipe_document_sha256.clone(),
    };
    let revised_identity = GeneralSemRevisionAuthorityIdentityV1 {
        project_id: request.revision.project_id,
        model_id: revised_model.id.clone(),
        model_document_sha256: revised_model_document_sha256.clone(),
        model_scientific_sha256: revised_model_scientific_sha256.clone(),
        recipe_id: revised_recipe.id,
        recipe_document_sha256: revised_recipe_document_sha256.clone(),
    };
    let lineage = GeneralSemExecutionAuthorityRevisionLineageV1 {
        schema_version: GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_SCHEMA_VERSION,
        revision_number,
        parent_revision_number: source_revision_number,
        source_archive_sha256: expected_source_archive_sha256.to_owned(),
        source_archive_bytes,
        source: source_identity.clone(),
        revised: revised_identity,
        compilation: compilation.clone(),
        intent: request.intent.clone(),
    };

    let [dataset_descriptor] = loaded.document.datasets.as_slice() else {
        return Err(source_mismatch(
            "source must contain exactly one resident dataset",
        ));
    };
    let [dataset] = loaded.datasets.as_slice() else {
        return Err(source_mismatch(
            "source must expose exactly one validated resident dataset",
        ));
    };
    let mut document = ProjectArchiveDocumentV6::new_general_sem_v1(
        request.revision.project_id,
        request.revision.project_name.clone(),
        request.revision.created_at,
    );
    document.datasets.push(dataset_descriptor.clone());
    document.models.push(ProjectModelRecordV6 {
        model_id: revised_model.id.clone(),
        payload: ProjectModelPayloadV6::SemModelV4 {
            model: revised_model,
            scientific_sha256: revised_model_scientific_sha256.clone(),
        },
    });
    document.recipes.push(revised_recipe);
    document.layouts.insert(
        GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_LAYOUT_KEY.into(),
        serde_json::to_value(&lineage)?,
    );
    document
        .ensure_valid()
        .map_err(ProjectArchiveV6SaveCopyError::Contract)?;
    let expected_source_sha256 = expected_source_archive_sha256.to_owned();
    let publication =
        publish_new_project_archive_v6_document_with_resident_datasets_before_publish(
            destination,
            &document,
            std::slice::from_ref(dataset),
            || {
                let (final_source_bytes, final_source_sha256) =
                    sha256_file_handle(&mut source_file)?;
                if final_source_bytes != source_archive_bytes
                    || final_source_sha256 != expected_source_sha256
                {
                    return Err(ProjectArchiveV6SaveCopyError::SourceChangedDuringSave);
                }
                Ok(())
            },
        )?;

    Ok(GeneralSemExecutionAuthorityRevisionReceiptV1 {
        schema_version: GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_SCHEMA_VERSION,
        archive_schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        revision_number,
        source_archive_path: source.to_string_lossy().into_owned(),
        source_archive_sha256: expected_source_archive_sha256.to_owned(),
        source_archive_bytes,
        source_verified_unchanged: true,
        source_project_id: source_identity.project_id,
        source_model_id: source_identity.model_id,
        source_model_document_sha256: source_identity.model_document_sha256,
        source_model_scientific_sha256: source_identity.model_scientific_sha256,
        source_recipe_id: source_identity.recipe_id,
        source_recipe_document_sha256: source_identity.recipe_document_sha256,
        destination_archive_path: destination.to_string_lossy().into_owned(),
        destination_archive_sha256: publication.destination_archive_sha256,
        destination_archive_bytes: publication.destination_archive_bytes,
        strict_reopen_validated: publication.strict_reopen_validated,
        project_id: request.revision.project_id,
        name: request.revision.project_name,
        created_at: request.revision.created_at,
        resident_dataset_id: dataset.id,
        resident_dataset_fingerprint: dataset.fingerprint.0.clone(),
        resident_model_id: document.models[0].model_id.clone(),
        resident_model_document_sha256: revised_model_document_sha256,
        resident_model_scientific_sha256: revised_model_scientific_sha256,
        resident_recipe_id: document.recipes[0].id,
        resident_recipe_document_sha256: revised_recipe_document_sha256,
        compiler_version: compilation.compiler_version,
        capability_cell: compilation.capability_cell,
        recipe_analytical_sha256: compilation.recipe_analytical_sha256,
        general_sem_config_sha256: compilation.general_sem_config_sha256,
        compiled_plan_sha256: compilation.compiled_plan_sha256,
        compiled_artifact_identity_sha256: compilation.compiled_artifact_identity_sha256,
        // Historical receipt field names are retained for wire compatibility;
        // they identify the created derived term/output for either supported intent.
        interaction_term_id: created_term_id,
        interaction_output_id: created_output_id,
    })
}

#[cfg(windows)]
fn create_general_sem_execution_authority_revision_windows_v2(
    source: &Path,
    expected_source_archive_sha256: &str,
    destination: &Path,
    request: GeneralSemExecutionAuthorityRevisionRequestV2,
) -> Result<
    GeneralSemExecutionAuthorityRevisionReceiptV2,
    GeneralSemExecutionAuthorityRevisionErrorV2,
> {
    validate_request_paths(source, destination, expected_source_archive_sha256)?;
    validate_new_identity_v2(&request)?;

    let mut source_file = open_exclusive_non_reparse_source(source)?;
    let (source_archive_bytes, observed_source_sha256) = sha256_file_handle(&mut source_file)?;
    if observed_source_sha256 != expected_source_archive_sha256 {
        return Err(ProjectArchiveV6SaveCopyError::SourceDigestMismatch {
            expected: expected_source_archive_sha256.to_owned(),
            observed: observed_source_sha256,
        }
        .into());
    }
    let loaded = load_project_archive_v6_from_file(source_file.try_clone()?)?;
    let source_revision_number = validate_source_authority(
        &loaded.document,
        &request.source,
        source_archive_bytes,
        expected_source_archive_sha256,
    )?;
    let revision_number = source_revision_number
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_REVISION_NUMBER)
        .ok_or_else(|| {
            GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                "revision number exceeds the safe-integer range".into(),
            )
        })?;

    let source_model = sole_source_model(&loaded.document)?;
    let mut revised_model = source_model.clone();
    revised_model.id = request.revision.model_id.clone();
    revised_model.name = request.revision.model_name.clone();
    revised_model
        .ensure_valid()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    let revised_model_document_sha256 = revised_model
        .model_document_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    let revised_model_scientific_sha256 = revised_model
        .scientific_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;

    let source_recipe = sole_source_recipe(&loaded.document)?;
    let mut revised_recipe = source_recipe.clone();
    revised_recipe.id = request.revision.recipe_id;
    revised_recipe.created_at = request.revision.created_at;
    revised_recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: revised_model.id.clone(),
        scientific_sha256: revised_model_scientific_sha256.clone(),
    };
    apply_moderated_mediation_path_revision_v2(&mut revised_recipe, &request.intent)?;
    revised_recipe.ensure_valid().map_err(|error| {
        GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(format!(
            "revised RecipeV4 is invalid: {error}"
        ))
    })?;

    let supplemental_capability_cell =
        pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    let compiled = compile_general_sem_pls_recipe_v1(&revised_recipe, Some(&revised_model))?;
    let compiled_target = compiled
        .plan()
        .two_way_moderated_mediation_target()
        .cloned()
        .ok_or_else(|| {
            GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
                "the revised model and Recipe did not compile the exact moderated-mediation target"
                    .into(),
            )
        })?;
    validate_compiled_revision_v2(
        &compiled_target,
        compiled.supplemental_capability_admission(),
        &supplemental_capability_cell,
        &request.intent,
    )?;
    let capability_dependencies = sorted_target_dependencies_v2(&compiled_target);
    let compiled_target_sha256 = sha256_serialized(&compiled_target);
    let revised_recipe_document_sha256 = sha256_serialized(&revised_recipe);
    let compilation = GeneralSemRevisionCompilationIdentityV1 {
        compiler_version: compiled.compiler_version().to_owned(),
        capability_cell: compiled.capability_cell().clone(),
        recipe_analytical_sha256: compiled.recipe_analytical_sha256().to_owned(),
        general_sem_config_sha256: compiled.general_sem_config_sha256().to_owned(),
        compiled_plan_sha256: compiled.plan().deterministic_sha256(),
        compiled_artifact_identity_sha256: compiled.artifact_identity_sha256().to_owned(),
    };

    let source_identity = GeneralSemRevisionAuthorityIdentityV1 {
        project_id: request.source.project_id,
        model_id: request.source.model_id.clone(),
        model_document_sha256: request.source.model_document_sha256.clone(),
        model_scientific_sha256: request.source.model_scientific_sha256.clone(),
        recipe_id: request.source.recipe_id,
        recipe_document_sha256: request.source.recipe_document_sha256.clone(),
    };
    let revised_identity = GeneralSemRevisionAuthorityIdentityV1 {
        project_id: request.revision.project_id,
        model_id: revised_model.id.clone(),
        model_document_sha256: revised_model_document_sha256.clone(),
        model_scientific_sha256: revised_model_scientific_sha256.clone(),
        recipe_id: revised_recipe.id,
        recipe_document_sha256: revised_recipe_document_sha256.clone(),
    };
    let lineage = GeneralSemExecutionAuthorityRevisionLineageV2 {
        schema_version: GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_SCHEMA_VERSION,
        revision_number,
        parent_revision_number: source_revision_number,
        source_archive_sha256: expected_source_archive_sha256.to_owned(),
        source_archive_bytes,
        source: source_identity.clone(),
        revised: revised_identity,
        compilation: compilation.clone(),
        supplemental_capability_cell: supplemental_capability_cell.clone(),
        capability_dependencies: capability_dependencies.clone(),
        compiled_target_sha256: compiled_target_sha256.clone(),
        compiled_target: compiled_target.clone(),
        intent: request.intent.clone(),
    };

    let [dataset_descriptor] = loaded.document.datasets.as_slice() else {
        return Err(source_mismatch(
            "source must contain exactly one resident dataset",
        ));
    };
    let [dataset] = loaded.datasets.as_slice() else {
        return Err(source_mismatch(
            "source must expose exactly one validated resident dataset",
        ));
    };
    let mut document = ProjectArchiveDocumentV6::new_general_sem_v1(
        request.revision.project_id,
        request.revision.project_name.clone(),
        request.revision.created_at,
    );
    document.datasets.push(dataset_descriptor.clone());
    document.models.push(ProjectModelRecordV6 {
        model_id: revised_model.id.clone(),
        payload: ProjectModelPayloadV6::SemModelV4 {
            model: revised_model,
            scientific_sha256: revised_model_scientific_sha256.clone(),
        },
    });
    document.recipes.push(revised_recipe);
    document.layouts.insert(
        GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY.into(),
        serde_json::to_value(&lineage)?,
    );
    document
        .ensure_valid()
        .map_err(ProjectArchiveV6SaveCopyError::Contract)?;
    let revised_pin = GeneralSemExecutionAuthoritySourcePinV1 {
        project_id: request.revision.project_id,
        model_id: document.models[0].model_id.clone(),
        model_document_sha256: revised_model_document_sha256.clone(),
        model_scientific_sha256: revised_model_scientific_sha256.clone(),
        recipe_id: document.recipes[0].id,
        recipe_document_sha256: revised_recipe_document_sha256.clone(),
    };
    if validate_source_authority(
        &document,
        &revised_pin,
        source_archive_bytes,
        expected_source_archive_sha256,
    )? != revision_number
    {
        return Err(source_mismatch(
            "revision-v2 lineage did not reconcile before publication",
        ));
    }

    let expected_source_sha256 = expected_source_archive_sha256.to_owned();
    let publication =
        publish_new_project_archive_v6_document_with_resident_datasets_before_publish(
            destination,
            &document,
            std::slice::from_ref(dataset),
            || {
                let (final_source_bytes, final_source_sha256) =
                    sha256_file_handle(&mut source_file)?;
                if final_source_bytes != source_archive_bytes
                    || final_source_sha256 != expected_source_sha256
                {
                    return Err(ProjectArchiveV6SaveCopyError::SourceChangedDuringSave);
                }
                Ok(())
            },
        )?;

    Ok(GeneralSemExecutionAuthorityRevisionReceiptV2 {
        schema_version: GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_SCHEMA_VERSION,
        archive_schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        revision_number,
        source_archive_path: source.to_string_lossy().into_owned(),
        source_archive_sha256: expected_source_archive_sha256.to_owned(),
        source_archive_bytes,
        source_verified_unchanged: true,
        source_project_id: source_identity.project_id,
        source_model_id: source_identity.model_id,
        source_model_document_sha256: source_identity.model_document_sha256,
        source_model_scientific_sha256: source_identity.model_scientific_sha256,
        source_recipe_id: source_identity.recipe_id,
        source_recipe_document_sha256: source_identity.recipe_document_sha256,
        destination_archive_path: destination.to_string_lossy().into_owned(),
        destination_archive_sha256: publication.destination_archive_sha256,
        destination_archive_bytes: publication.destination_archive_bytes,
        strict_reopen_validated: publication.strict_reopen_validated,
        project_id: request.revision.project_id,
        name: request.revision.project_name,
        created_at: request.revision.created_at,
        resident_dataset_id: dataset.id,
        resident_dataset_fingerprint: dataset.fingerprint.0.clone(),
        resident_model_id: document.models[0].model_id.clone(),
        resident_model_document_sha256: revised_model_document_sha256,
        resident_model_scientific_sha256: revised_model_scientific_sha256,
        resident_recipe_id: document.recipes[0].id,
        resident_recipe_document_sha256: revised_recipe_document_sha256,
        compiler_version: compilation.compiler_version,
        primary_capability_cell: compilation.capability_cell,
        supplemental_capability_cell,
        capability_dependencies,
        recipe_analytical_sha256: compilation.recipe_analytical_sha256,
        general_sem_config_sha256: compilation.general_sem_config_sha256,
        compiled_plan_sha256: compilation.compiled_plan_sha256,
        compiled_artifact_identity_sha256: compilation.compiled_artifact_identity_sha256,
        compiled_target_sha256,
        compiled_target,
    })
}

fn validate_request_paths(
    source: &Path,
    destination: &Path,
    expected_source_archive_sha256: &str,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV1> {
    if !source.is_absolute() || !destination.is_absolute() {
        return Err(ProjectArchiveV6SaveCopyError::AbsolutePathsRequired.into());
    }
    let normalized = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    if normalized(source) == normalized(destination) {
        return Err(ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer.into());
    }
    if !is_lowercase_sha256(expected_source_archive_sha256) {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "expected source archive SHA-256 must be lowercase hexadecimal".into(),
        ));
    }
    Ok(())
}

fn validate_new_identity(
    request: &GeneralSemExecutionAuthorityRevisionRequestV1,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV1> {
    validate_new_identity_parts(&request.source, &request.revision)
}

fn validate_new_identity_v2(
    request: &GeneralSemExecutionAuthorityRevisionRequestV2,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV2> {
    validate_new_identity_parts(&request.source, &request.revision)?;
    let GeneralSemExecutionAuthorityRevisionIntentV2::SelectTwoWayModeratedMediationPath {
        intent_version,
        sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
        estimand_id,
        ordered_relation_ids,
    } = &request.intent;
    if *intent_version != GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_SCHEMA_VERSION {
        return Err(
            GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
                "select_two_way_moderated_mediation_path requires intent_version=2".into(),
            ),
        );
    }
    for (field, value) in [
        ("estimand id", estimand_id.as_str()),
        ("first relation id", ordered_relation_ids[0].as_str()),
        ("second relation id", ordered_relation_ids[1].as_str()),
    ] {
        if value.is_empty() || value.trim() != value {
            return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                format!("{field} must be nonempty without surrounding whitespace"),
            ));
        }
    }
    if ordered_relation_ids[0] == ordered_relation_ids[1] {
        return Err(
            GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
                "the selected SpecificPath must contain two distinct ordered relation ids".into(),
            ),
        );
    }
    if estimand_id != &specific_directed_path_identity_v1(ordered_relation_ids) {
        return Err(
            GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
                "estimand id must equal the stable identity of the selected ordered relation path"
                    .into(),
            ),
        );
    }
    Ok(())
}

fn validate_new_identity_parts(
    source: &GeneralSemExecutionAuthoritySourcePinV1,
    revision: &GeneralSemExecutionAuthorityRevisionIdentityV1,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV1> {
    for (field, value) in [
        ("source model id", source.model_id.as_str()),
        ("revision project name", revision.project_name.as_str()),
        ("revision model id", revision.model_id.as_str()),
        ("revision model name", revision.model_name.as_str()),
    ] {
        if value.is_empty() || value.trim() != value {
            return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                format!("{field} must be nonempty without surrounding whitespace"),
            ));
        }
    }
    for (field, digest) in [
        (
            "source model document",
            source.model_document_sha256.as_str(),
        ),
        (
            "source model scientific",
            source.model_scientific_sha256.as_str(),
        ),
        (
            "source recipe document",
            source.recipe_document_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(digest) {
            return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
                format!("{field} SHA-256 must be lowercase hexadecimal"),
            ));
        }
    }
    if source.project_id.is_nil()
        || source.recipe_id.is_nil()
        || revision.project_id.is_nil()
        || revision.recipe_id.is_nil()
    {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "project and recipe UUIDs must be non-nil".into(),
        ));
    }
    if source.project_id == revision.project_id
        || source.recipe_id == revision.recipe_id
        || source.model_id == revision.model_id
    {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "revision project, model, and recipe identities must all be new".into(),
        ));
    }
    if request.recipe_execution_surface != GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
        && request.recipe_execution_surface != GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1
    {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(
            "recipe execution surface must be an exact General SEM Labs or Standard v1 identity"
                .into(),
        ));
    }
    Ok(())
}

fn selected_effect_for_revision_v2(
    intent: &GeneralSemExecutionAuthorityRevisionIntentV2,
) -> GeneralSemEffectEstimandV1 {
    let GeneralSemExecutionAuthorityRevisionIntentV2::SelectTwoWayModeratedMediationPath {
        estimand_id,
        ordered_relation_ids,
        ..
    } = intent;
    GeneralSemEffectEstimandV1::SpecificPath {
        estimand_id: estimand_id.clone(),
        ordered_relation_ids: ordered_relation_ids.to_vec(),
    }
}

fn apply_moderated_mediation_path_revision_v2(
    recipe: &mut qpls_core::AnalysisRecipeV4,
    intent: &GeneralSemExecutionAuthorityRevisionIntentV2,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV2> {
    let selected_effect = selected_effect_for_revision_v2(intent);
    let config = recipe.general_sem_config.as_mut().ok_or_else(|| {
        GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
            "the resident RecipeV4 must contain GeneralSemConfigV1".into(),
        )
    })?;
    if !config.requested_effect_estimands.is_empty()
        && config.requested_effect_estimands.as_slice() != [selected_effect.clone()]
    {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
            "the resident RecipeV4 has conflicting effect requests; revision-v2 will not discard them"
                .into(),
        ));
    }
    config.requested_effect_estimands = vec![selected_effect];
    config.ensure_valid().map_err(|error| {
        GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(format!(
            "the selected SpecificPath is invalid: {error}"
        ))
    })
}

fn sorted_target_dependencies_v2(
    target: &CompiledPlsTwoWayModeratedMediationTargetV1,
) -> Vec<CapabilityCellReferenceV2> {
    let mut dependencies = vec![
        target.base_pls_capability_cell().clone(),
        target.moderation_point_capability_cell().clone(),
    ];
    dependencies.sort_by_key(capability_cell_reference_identity_v2);
    dependencies
}

fn validate_compiled_revision_v2(
    target: &CompiledPlsTwoWayModeratedMediationTargetV1,
    admitted: Option<&CapabilityCellReferenceV2>,
    expected_supplemental: &CapabilityCellReferenceV2,
    intent: &GeneralSemExecutionAuthorityRevisionIntentV2,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV2> {
    let GeneralSemExecutionAuthorityRevisionIntentV2::SelectTwoWayModeratedMediationPath {
        estimand_id,
        ordered_relation_ids,
        ..
    } = intent;
    if admitted != Some(expected_supplemental)
        || target.bootstrap_capability_cell() != expected_supplemental
        || target.estimand_id() != estimand_id
        || target.ordered_relation_ids() != ordered_relation_ids
    {
        return Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(
            "compiled moderated-mediation target does not match the exact Registry admission and selected SpecificPath"
                .into(),
        ));
    }
    Ok(())
}

fn validate_source_authority(
    document: &ProjectArchiveDocumentV6,
    expected: &GeneralSemExecutionAuthoritySourcePinV1,
    source_archive_bytes: u64,
    source_archive_sha256: &str,
) -> Result<u64, GeneralSemExecutionAuthorityRevisionErrorV1> {
    if !document.supports_general_sem_v1() {
        return Err(source_mismatch(
            "source must carry exact new-project general_sem_v1 authority",
        ));
    }
    if document.project_id != expected.project_id {
        return Err(source_mismatch("source project id changed"));
    }
    if !document.historical_recipes.is_empty() || !document.historical_results.is_empty() {
        return Err(source_mismatch(
            "General SEM revisions cannot import legacy recipe or result lanes",
        ));
    }
    if document.datasets.len() != 1 {
        return Err(source_mismatch("source must contain exactly one dataset"));
    }
    let model = sole_source_model(document)?;
    let model_record = &document.models[0];
    let ProjectModelPayloadV6::SemModelV4 {
        scientific_sha256, ..
    } = &model_record.payload
    else {
        return Err(source_mismatch("source model is not a promoted SemModelV4"));
    };
    let model_document_sha256 = model
        .model_document_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    let calculated_scientific_sha256 = model
        .scientific_sha256()
        .map_err(|error| GeneralSemExecutionAuthorityRevisionErrorV1::Model(error.to_string()))?;
    if model_record.model_id != expected.model_id
        || model.id != expected.model_id
        || model_document_sha256 != expected.model_document_sha256
        || scientific_sha256 != &expected.model_scientific_sha256
        || calculated_scientific_sha256 != expected.model_scientific_sha256
    {
        return Err(source_mismatch("source model authority or digest changed"));
    }
    let recipe = sole_source_recipe(document)?;
    if recipe.id != expected.recipe_id
        || sha256_serialized(recipe) != expected.recipe_document_sha256
        || recipe.general_sem_config.is_none()
        || recipe.dataset_fingerprint != document.datasets[0].fingerprint.0
        || !matches!(
            &recipe.model_binding,
            AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference { model_id, scientific_sha256 }
                if model_id == &expected.model_id
                    && scientific_sha256 == &expected.model_scientific_sha256
        )
    {
        return Err(source_mismatch(
            "source RecipeV4 authority or digest changed",
        ));
    }
    let revision_v1 = document
        .layouts
        .get(GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_LAYOUT_KEY);
    let revision_v2 = document
        .layouts
        .get(GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY);
    if revision_v1.is_some() && revision_v2.is_some() {
        return Err(source_mismatch(
            "source carries more than one execution-authority revision lineage",
        ));
    }
    if let Some(value) = revision_v2 {
        let lineage: GeneralSemExecutionAuthorityRevisionLineageV2 =
            serde_json::from_value(value.clone())?;
        let supplemental = pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
        let compiled = compile_general_sem_pls_recipe_v1(recipe, Some(model))?;
        let target = compiled
            .plan()
            .two_way_moderated_mediation_target()
            .ok_or_else(|| source_mismatch("source revision-v2 target is missing"))?;
        validate_compiled_revision_v2(
            target,
            compiled.supplemental_capability_admission(),
            &supplemental,
            &lineage.intent,
        )?;
        let observed_compilation = GeneralSemRevisionCompilationIdentityV1 {
            compiler_version: compiled.compiler_version().to_owned(),
            capability_cell: compiled.capability_cell().clone(),
            recipe_analytical_sha256: compiled.recipe_analytical_sha256().to_owned(),
            general_sem_config_sha256: compiled.general_sem_config_sha256().to_owned(),
            compiled_plan_sha256: compiled.plan().deterministic_sha256(),
            compiled_artifact_identity_sha256: compiled.artifact_identity_sha256().to_owned(),
        };
        if lineage.schema_version != GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_SCHEMA_VERSION
            || lineage.revision_number == 0
            || lineage.revision_number > MAX_SAFE_REVISION_NUMBER
            || lineage.parent_revision_number.checked_add(1) != Some(lineage.revision_number)
            || !is_lowercase_sha256(&lineage.source_archive_sha256)
            || lineage.source_archive_bytes == 0
            || lineage.source.project_id.is_nil()
            || lineage.source.recipe_id.is_nil()
            || lineage.source.model_id.is_empty()
            || !is_lowercase_sha256(&lineage.source.model_document_sha256)
            || !is_lowercase_sha256(&lineage.source.model_scientific_sha256)
            || !is_lowercase_sha256(&lineage.source.recipe_document_sha256)
            || lineage.source.project_id == lineage.revised.project_id
            || lineage.source.recipe_id == lineage.revised.recipe_id
            || lineage.source.model_id == lineage.revised.model_id
            || lineage.revised.project_id != expected.project_id
            || lineage.revised.model_id != expected.model_id
            || lineage.revised.model_document_sha256 != expected.model_document_sha256
            || lineage.revised.model_scientific_sha256 != expected.model_scientific_sha256
            || lineage.revised.recipe_id != expected.recipe_id
            || lineage.revised.recipe_document_sha256 != expected.recipe_document_sha256
            || lineage.compilation != observed_compilation
            || lineage.supplemental_capability_cell != supplemental
            || lineage.capability_dependencies != sorted_target_dependencies_v2(target)
            || lineage.compiled_target != *target
            || lineage.compiled_target_sha256 != sha256_serialized(target)
            || source_archive_bytes == 0
            || !is_lowercase_sha256(source_archive_sha256)
        {
            return Err(source_mismatch(
                "source revision-v2 lineage is invalid, stale, or differs from recompilation",
            ));
        }
        return Ok(lineage.revision_number);
    }
    let Some(value) = revision_v1 else {
        return Ok(0);
    };
    let lineage: GeneralSemExecutionAuthorityRevisionLineageV1 =
        serde_json::from_value(value.clone())?;
    if lineage.schema_version != GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_SCHEMA_VERSION
        || lineage.revision_number == 0
        || lineage.revision_number > MAX_SAFE_REVISION_NUMBER
        || lineage.parent_revision_number.checked_add(1) != Some(lineage.revision_number)
        || lineage.source_archive_sha256.len() != 64
        || lineage.source_archive_bytes == 0
        || lineage.revised.project_id != expected.project_id
        || lineage.revised.model_id != expected.model_id
        || lineage.revised.model_document_sha256 != expected.model_document_sha256
        || lineage.revised.model_scientific_sha256 != expected.model_scientific_sha256
        || lineage.revised.recipe_id != expected.recipe_id
        || lineage.revised.recipe_document_sha256 != expected.recipe_document_sha256
        || source_archive_bytes == 0
        || !is_lowercase_sha256(source_archive_sha256)
    {
        return Err(source_mismatch(
            "source revision lineage is invalid or stale",
        ));
    }
    Ok(lineage.revision_number)
}

fn sole_source_model(
    document: &ProjectArchiveDocumentV6,
) -> Result<&SemModelV4, GeneralSemExecutionAuthorityRevisionErrorV1> {
    let [record] = document.models.as_slice() else {
        return Err(source_mismatch("source must contain exactly one model"));
    };
    match &record.payload {
        ProjectModelPayloadV6::SemModelV4 { model, .. } => Ok(model),
        _ => Err(source_mismatch(
            "source model must be ready, not draft or legacy",
        )),
    }
}

fn sole_source_recipe(
    document: &ProjectArchiveDocumentV6,
) -> Result<&qpls_core::AnalysisRecipeV4, GeneralSemExecutionAuthorityRevisionErrorV1> {
    let [recipe] = document.recipes.as_slice() else {
        return Err(source_mismatch("source must contain exactly one RecipeV4"));
    };
    Ok(recipe)
}

fn apply_general_sem_revision_intent(
    model: &mut SemModelV4,
    intent: &GeneralSemExecutionAuthorityRevisionIntentV1,
) -> Result<(String, String), GeneralSemExecutionAuthorityRevisionErrorV1> {
    match intent {
        GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 { .. } => {
            apply_interaction_v2_revision(model, intent)
        }
        GeneralSemExecutionAuthorityRevisionIntentV1::AddHigherOrder {
            term_id,
            output_id,
            label,
            components,
            approach,
            measurement_type,
            initial_path,
        } => apply_higher_order_revision(
            model,
            term_id,
            output_id,
            label,
            components,
            approach,
            measurement_type,
            initial_path,
        ),
    }
}

fn apply_interaction_v2_revision(
    model: &mut SemModelV4,
    intent: &GeneralSemExecutionAuthorityRevisionIntentV1,
) -> Result<(String, String), GeneralSemExecutionAuthorityRevisionErrorV1> {
    let GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
        intent_version,
        sem_generation,
        label,
        operands: [predictor, moderator],
        focal_relation,
        outcome,
        method,
        hierarchy_policy,
    } = intent
    else {
        return Err(unsupported_intent("interaction revision intent required"));
    };
    if *intent_version != 1
        || *sem_generation != GeneralSemRevisionGenerationV1::GeneralSemV1
        || *method != GeneralSemRevisionInteractionMethodV1::TwoStage
        || *hierarchy_policy != GeneralSemRevisionHierarchyPolicyV1::Strong
    {
        return Err(unsupported_intent(
            "schema v1 requires general_sem_v1, two_stage, and strong hierarchy",
        ));
    }
    for (field, value) in [
        ("label", label.as_str()),
        ("predictor", predictor.as_str()),
        ("moderator", moderator.as_str()),
        ("focal relation", focal_relation.as_str()),
        ("outcome", outcome.as_str()),
    ] {
        if value.is_empty() || value.trim() != value {
            return Err(unsupported_intent(format!(
                "{field} must be a nonempty stable value"
            )));
        }
    }
    if predictor == moderator || predictor == outcome || moderator == outcome {
        return Err(unsupported_intent(
            "predictor, moderator, and outcome must be distinct",
        ));
    }
    for (id, position) in [
        (predictor.as_str(), "source"),
        (moderator.as_str(), "source"),
        (outcome.as_str(), "target"),
    ] {
        let Some(variable) = model.variables.iter().find(|variable| variable.id() == id) else {
            return Err(unsupported_intent(format!(
                "unknown structural {position} {id}"
            )));
        };
        ensure_structural_role(variable, position)?;
    }
    let focal = model
        .relations
        .iter()
        .find(|relation| relation.id() == focal_relation);
    if !matches!(
        focal,
        Some(SemRelationV4::Structural {
            source,
            target,
            role: StructuralRelationRoleV4::Structural,
            ..
        }) if source == predictor && target == outcome
    ) {
        return Err(unsupported_intent(
            "focal relation is not the exact non-control predictor-to-outcome path",
        ));
    }

    let term_id = general_sem_interaction_v2_term_id(focal_relation, predictor, moderator);
    let output_id = format!(
        "general-sem:v1:interaction-output:{}",
        encode_uri_component(&term_id)
    );
    if model.derived_terms.iter().any(|term| term.id() == term_id)
        || model
            .variables
            .iter()
            .any(|variable| variable.id() == output_id)
    {
        return Err(unsupported_intent(
            "deterministic interaction identity already exists",
        ));
    }
    let semantic_duplicate = model.derived_terms.iter().any(|term| match term {
        SemDerivedTermV4::Interaction {
            predictor: existing_predictor,
            moderator: existing_moderator,
            focal_relation: existing_focal,
            ..
        } => {
            existing_predictor == predictor
                && existing_moderator == moderator
                && existing_focal == focal_relation
        }
        SemDerivedTermV4::InteractionV2 {
            operands,
            focal_relation: existing_focal,
            ..
        } => {
            operands.len() == 2
                && &operands[0] == predictor
                && &operands[1] == moderator
                && existing_focal == focal_relation
        }
        _ => false,
    });
    if semantic_duplicate {
        return Err(unsupported_intent(
            "the same moderating effect already exists",
        ));
    }

    let existing_main = model.relations.iter().find(|relation| {
        matches!(relation, SemRelationV4::Structural { source, target, .. }
            if source == moderator && target == outcome)
    });
    if matches!(
        existing_main,
        Some(SemRelationV4::Structural {
            role: StructuralRelationRoleV4::Control,
            ..
        })
    ) {
        return Err(unsupported_intent(
            "moderator-to-outcome relationship is a control path",
        ));
    }

    model.variables.push(SemVariableV4::Derived {
        id: output_id.clone(),
        label: label.clone(),
    });
    model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
        id: term_id.clone(),
        output: output_id.clone(),
        operands: vec![predictor.clone(), moderator.clone()],
        focal_relation: focal_relation.clone(),
        method: InteractionMethodV4::TwoStage,
        hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
        product_indicator: None,
    });
    if existing_main.is_none() {
        add_structural_relation(
            model,
            format!(
                "general-sem:v1:interaction-moderator-main:{}",
                encode_uri_component(&term_id)
            ),
            moderator.clone(),
            outcome.clone(),
            "Moderator main effect",
        )?;
    }
    add_structural_relation(
        model,
        format!(
            "general-sem:v1:interaction-effect:{}",
            encode_uri_component(&term_id)
        ),
        output_id.clone(),
        outcome.clone(),
        "Interaction effect",
    )?;
    Ok((term_id, output_id))
}

#[allow(clippy::too_many_arguments)]
fn apply_higher_order_revision(
    model: &mut SemModelV4,
    term_id: &str,
    output_id: &str,
    label: &str,
    components: &[String],
    approach: &HigherOrderConstructionApproachV4,
    measurement_type: &HigherOrderMeasurementTypeV4,
    initial_path: &GeneralSemRevisionHigherOrderPathV1,
) -> Result<(String, String), GeneralSemExecutionAuthorityRevisionErrorV1> {
    for (field, value) in [
        ("term id", term_id),
        ("output id", output_id),
        ("label", label),
        (
            "initial path relation id",
            initial_path.relation_id.as_str(),
        ),
        ("initial path source", initial_path.source.as_str()),
        ("initial path target", initial_path.target.as_str()),
        ("initial path label", initial_path.label.as_str()),
    ] {
        if value.is_empty() || value.trim() != value {
            return Err(unsupported_intent(format!(
                "higher-order {field} must be nonempty without surrounding whitespace"
            )));
        }
    }
    if components.len() < 2 {
        return Err(unsupported_intent(
            "higher-order revision requires at least two lower-order components",
        ));
    }
    let component_set = components.iter().collect::<std::collections::BTreeSet<_>>();
    if component_set.len() != components.len() {
        return Err(unsupported_intent(
            "higher-order components must be distinct",
        ));
    }
    if model.derived_terms.iter().any(|term| term.id() == term_id)
        || model
            .variables
            .iter()
            .any(|variable| variable.id() == output_id)
        || model
            .derived_terms
            .iter()
            .any(|term| matches!(term, SemDerivedTermV4::HigherOrder { .. }))
    {
        return Err(unsupported_intent(
            "higher-order term/output identity already exists or the model already contains a HOC",
        ));
    }
    for component in components {
        if !matches!(
            model
                .variables
                .iter()
                .find(|variable| variable.id() == component),
            Some(SemVariableV4::Composite { .. })
        ) {
            return Err(unsupported_intent(format!(
                "higher-order component {component} is not an ordinary composite"
            )));
        }
    }
    let other_endpoint = match (
        initial_path.source.as_str() == output_id,
        initial_path.target.as_str() == output_id,
    ) {
        (true, false) => initial_path.target.as_str(),
        (false, true) => initial_path.source.as_str(),
        _ => {
            return Err(unsupported_intent(
                "higher-order initial path must use the new HOC output as exactly one endpoint",
            ));
        }
    };
    if components
        .iter()
        .any(|component| component == other_endpoint)
        || !matches!(
            model
                .variables
                .iter()
                .find(|variable| variable.id() == other_endpoint),
            Some(SemVariableV4::Composite { .. })
        )
    {
        return Err(unsupported_intent(
            "higher-order initial path must connect to an ordinary composite outside the component set",
        ));
    }
    model.variables.push(SemVariableV4::Derived {
        id: output_id.to_owned(),
        label: label.to_owned(),
    });
    model.derived_terms.push(SemDerivedTermV4::HigherOrder {
        id: term_id.to_owned(),
        output: output_id.to_owned(),
        components: components.to_vec(),
        approach: approach.clone(),
        measurement_type: measurement_type.clone(),
    });
    add_structural_relation(
        model,
        initial_path.relation_id.clone(),
        initial_path.source.clone(),
        initial_path.target.clone(),
        &initial_path.label,
    )?;
    Ok((term_id.to_owned(), output_id.to_owned()))
}

fn add_structural_relation(
    model: &mut SemModelV4,
    relation_id: String,
    source: String,
    target: String,
    label: &str,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV1> {
    if model
        .relations
        .iter()
        .any(|relation| relation.id() == relation_id)
    {
        return Err(unsupported_intent(
            "deterministic relationship identity already exists",
        ));
    }
    let parameter_id = format!(
        "standard:v1:relationship-parameter:{}",
        encode_uri_component(&relation_id)
    );
    if model
        .parameters
        .iter()
        .any(|parameter| parameter.id() == parameter_id)
    {
        return Err(unsupported_intent(
            "deterministic parameter identity already exists",
        ));
    }
    model.relations.push(SemRelationV4::Structural {
        id: relation_id,
        source: source.clone(),
        target: target.clone(),
        parameter: parameter_id.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter_id,
        label: label.into(),
        target: SemParameterTargetV4::Regression { source, target },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    Ok(())
}

fn ensure_structural_role(
    variable: &SemVariableV4,
    position: &str,
) -> Result<(), GeneralSemExecutionAuthorityRevisionErrorV1> {
    let SemVariableV4::Observed {
        role, scale, id, ..
    } = variable
    else {
        return Ok(());
    };
    let allowed = if position == "source" {
        matches!(
            role,
            qpls_core::ObservedRoleV4::Structural
                | qpls_core::ObservedRoleV4::Both
                | qpls_core::ObservedRoleV4::Control
        )
    } else {
        matches!(
            role,
            qpls_core::ObservedRoleV4::Structural | qpls_core::ObservedRoleV4::Both
        )
    };
    if !allowed || matches!(scale, qpls_core::ObservedScaleV4::Identifier) {
        return Err(unsupported_intent(format!(
            "observed variable {id} cannot be used as structural {position}"
        )));
    }
    Ok(())
}

fn general_sem_interaction_v2_term_id(focal: &str, predictor: &str, moderator: &str) -> String {
    format!(
        "general-sem:v1:interaction:{}:{}:{}",
        encode_uri_component(focal),
        encode_uri_component(predictor),
        encode_uri_component(moderator)
    )
}

/// Byte-for-byte equivalent to JavaScript `encodeURIComponent` for valid UTF-8
/// strings (serde JSON cannot carry isolated UTF-16 surrogate code units).
fn encode_uri_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                *byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(*byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn source_mismatch(message: impl Into<String>) -> GeneralSemExecutionAuthorityRevisionErrorV1 {
    GeneralSemExecutionAuthorityRevisionErrorV1::SourceAuthorityMismatch(message.into())
}

fn unsupported_intent(message: impl Into<String>) -> GeneralSemExecutionAuthorityRevisionErrorV1 {
    GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(message.into())
}

#[cfg(windows)]
fn open_exclusive_non_reparse_source(
    source: &Path,
) -> Result<File, GeneralSemExecutionAuthorityRevisionErrorV1> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(source)?;
    let attributes = file.metadata()?.file_attributes();
    if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile.into());
    }
    Ok(file)
}

fn sha256_file_handle(file: &mut File) -> Result<(u64, String), std::io::Error> {
    file.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes = bytes.checked_add(read as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "source size overflow")
        })?;
        digest.update(&buffer[..read]);
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_ids_match_encode_uri_component_contract() {
        let term = general_sem_interaction_v2_term_id("path:a/b", "X ü", "W!'()");
        assert_eq!(
            term,
            "general-sem:v1:interaction:path%3Aa%2Fb:X%20%C3%BC:W!'()"
        );
        assert_eq!(
            encode_uri_component(&term),
            "general-sem%3Av1%3Ainteraction%3Apath%253Aa%252Fb%3AX%2520%25C3%25BC%3AW!'()"
        );
    }

    #[test]
    fn exact_intent_rejects_wrong_version_before_mutation() {
        let mut model = SemModelV4 {
            schema_version: 4,
            id: "model:new".into(),
            name: "New".into(),
            variables: Vec::new(),
            relations: Vec::new(),
            parameters: Vec::new(),
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: qpls_core::SemGroupV4::SingleGroup,
            data_binding: qpls_core::SemDataBindingV4::Raw {
                dataset_id: Uuid::from_u128(1).to_string(),
                missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: qpls_core::SemPresentationV4::None,
        };
        let before = model.clone();
        let result = apply_interaction_v2_revision(
            &mut model,
            &GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
                intent_version: 2,
                sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                label: "X x W".into(),
                operands: ["x".into(), "w".into()],
                focal_relation: "x-y".into(),
                outcome: "y".into(),
                method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
            },
        );
        assert!(matches!(
            result,
            Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(_))
        ));
        assert_eq!(model, before);
    }

    #[test]
    fn higher_order_revision_adds_the_hoc_and_initial_path_atomically() {
        let composite = |id: &str| SemVariableV4::Composite {
            id: id.into(),
            label: id.to_uppercase(),
            weighting: qpls_core::CompositeWeightingV4::ModeA,
        };
        let mut model = SemModelV4 {
            schema_version: 4,
            id: "model:hoc-revision".into(),
            name: "HOC revision".into(),
            variables: vec![composite("a"), composite("b"), composite("y")],
            relations: Vec::new(),
            parameters: Vec::new(),
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: qpls_core::SemGroupV4::SingleGroup,
            data_binding: qpls_core::SemDataBindingV4::Raw {
                dataset_id: Uuid::from_u128(1).to_string(),
                missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: qpls_core::SemPresentationV4::None,
        };
        let intent = GeneralSemExecutionAuthorityRevisionIntentV1::AddHigherOrder {
            term_id: "hoc:term".into(),
            output_id: "hoc:output".into(),
            label: "Higher order".into(),
            components: vec!["a".into(), "b".into()],
            approach: HigherOrderConstructionApproachV4::DisjointTwoStage,
            measurement_type: HigherOrderMeasurementTypeV4::ReflectiveReflective,
            initial_path: GeneralSemRevisionHigherOrderPathV1 {
                relation_id: "hoc:path".into(),
                source: "hoc:output".into(),
                target: "y".into(),
                label: "HOC -> Y".into(),
            },
        };

        let created = apply_general_sem_revision_intent(&mut model, &intent).unwrap();

        assert_eq!(created, ("hoc:term".into(), "hoc:output".into()));
        assert!(model.derived_terms.iter().any(|term| matches!(
            term,
            SemDerivedTermV4::HigherOrder { id, output, components, .. }
                if id == "hoc:term"
                    && output == "hoc:output"
                    && components.iter().map(String::as_str).eq(["a", "b"])
        )));
        assert!(model.relations.iter().any(|relation| matches!(
            relation,
            SemRelationV4::Structural { id, source, target, .. }
                if id == "hoc:path" && source == "hoc:output" && target == "y"
        )));
    }

    #[cfg(windows)]
    mod windows {
        use super::super::*;
        use crate::{
            ProjectModelPayloadV6, ProjectOriginV6, ProjectSemGenerationV6,
            create_populated_general_sem_project_archive_v6, load_project_archive_v6,
        };
        use chrono::TimeZone;
        use qpls_core::{
            ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
            AnalysisRecipeModelBindingV4, AnalysisRecipeV4, AnalysisSettings, Construct,
            GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
            GeneralSemInferenceV1, LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig,
            ModelSpec, PlsBootstrapTestTail, SemDataBindingV4, SemModelV4, SemRelationV4,
            StructuralPath, confirm_legacy_recipe_estimand_v4,
            migrate_analysis_recipe_to_v4_pending, sha256_serialized,
        };
        use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
        use sha2::{Digest, Sha256};
        use std::{
            collections::BTreeMap,
            fs,
            path::{Path, PathBuf},
        };

        struct SourceFixture {
            source: PathBuf,
            destination: PathBuf,
            source_bytes: Vec<u8>,
            source_sha256: String,
            dataset: Dataset,
            model: SemModelV4,
            recipe: AnalysisRecipeV4,
            request: GeneralSemExecutionAuthorityRevisionRequestV1,
        }

        #[derive(Clone, Copy)]
        enum ModeratedMediationStageFixture {
            First,
            Second,
        }

        struct ModeratedMediationSourceFixture {
            source: PathBuf,
            destination: PathBuf,
            source_bytes: Vec<u8>,
            source_sha256: String,
            dataset: Dataset,
            model: SemModelV4,
            recipe: AnalysisRecipeV4,
            request: GeneralSemExecutionAuthorityRevisionRequestV2,
        }

        fn fixture_recipe_and_model() -> (Dataset, AnalysisRecipeV4, SemModelV4) {
            let source_model = ModelSpec {
                id: Uuid::from_u128(0x7265_7669_7369_6f6e_5f73_6f75_7263_6501),
                name: "General SEM revision source".into(),
                constructs: ["x", "w", "y"]
                    .into_iter()
                    .map(|id| Construct {
                        id: id.into(),
                        name: id.to_uppercase(),
                        short_name: id.to_uppercase(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec![format!("{id}1"), format!("{id}2")],
                    })
                    .collect(),
                paths: [("x", "y"), ("w", "y")]
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
            let dataset = import_delimited_bytes(
                b"x1,x2,w1,w2,y1,y2\n1,2,2,1,2,1\n2,1,3,2,3,2\n3,4,4,3,5,4\n4,3,5,5,6,5\n5,6,7,6,8,7\n6,5,6,7,9,8\n7,8,9,7,11,9\n8,7,8,9,10,11\n9,10,11,10,13,12\n10,9,12,11,14,13\n11,12,13,12,16,15\n12,11,14,13,17,16\n",
                "general-sem-revision.csv",
                b',',
                &ImportOptions::default(),
            )
            .unwrap();
            let source_recipe = AnalysisRecipe {
                schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
                id: Uuid::from_u128(0x7265_7669_7369_6f6e_5f73_6f75_7263_6502),
                created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
                dataset_fingerprint: dataset.fingerprint.0.clone(),
                model: source_model.clone(),
                settings: AnalysisSettings {
                    method: AnalysisMethod::PlsPm,
                    workers: 1,
                    ..AnalysisSettings::default()
                },
                method_config: Some(MethodConfig::PlsAlgorithm),
                metadata: BTreeMap::new(),
            };
            let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe).unwrap();
            let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
                &pending,
                &source_model,
                &[],
                LegacyBasicModelInterpretationV4::PlsComposite,
            )
            .unwrap();
            let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
                unreachable!()
            };
            *dataset_id = dataset.id.to_string();
            recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model.id.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            };
            recipe.general_sem_config = Some(GeneralSemConfigV1::default());
            recipe.ensure_valid().unwrap();
            (dataset, recipe, model)
        }

        fn temporary_links(directory: &Path) -> Vec<PathBuf> {
            fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.contains(".schema6-save-") && name.ends_with(".tmp")
                        })
                })
                .collect()
        }

        fn archive_sha256(bytes: &[u8]) -> String {
            format!("{:x}", Sha256::digest(bytes))
        }

        fn write_source_with_bootstrap(directory: &Path, bootstrap: bool) -> SourceFixture {
            let source = directory.join("general-sem-source.qpls");
            let destination = directory.join("general-sem-revision.qpls");
            let (dataset, mut recipe, model) = fixture_recipe_and_model();
            if bootstrap {
                recipe.general_sem_config.as_mut().unwrap().inference =
                    GeneralSemInferenceV1::CaseBootstrap {
                        resamples: 500,
                        seed: 7,
                        confidence_level: 0.95,
                        interval: GeneralSemBootstrapIntervalV1::Percentile,
                        tail: GeneralSemInferenceTailV1::TwoSided,
                    };
                recipe.settings.bootstrap_samples = 500;
                recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
                recipe.settings.studentized_inner_samples = 0;
                recipe.settings.seed = 7;
                recipe.settings.confidence_level = 0.95;
                recipe.ensure_valid().unwrap();
            }
            let source_project_id = Uuid::from_u128(0x7265_7669_7369_6f6e_5f70_726f_6a65_6301);
            create_populated_general_sem_project_archive_v6(
                &source,
                source_project_id,
                "General SEM source",
                Utc.with_ymd_and_hms(2026, 8, 19, 9, 0, 0).unwrap(),
                &dataset,
                model.clone(),
                recipe.clone(),
            )
            .unwrap();
            let source_bytes = fs::read(&source).unwrap();
            let source_sha256 = archive_sha256(&source_bytes);
            let focal_relation = model
                .relations
                .iter()
                .find_map(|relation| match relation {
                    SemRelationV4::Structural {
                        id, source, target, ..
                    } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                    _ => None,
                })
                .unwrap();
            let request = GeneralSemExecutionAuthorityRevisionRequestV1 {
                source: GeneralSemExecutionAuthoritySourcePinV1 {
                    project_id: source_project_id,
                    model_id: model.id.clone(),
                    model_document_sha256: model.model_document_sha256().unwrap(),
                    model_scientific_sha256: model.scientific_sha256().unwrap(),
                    recipe_id: recipe.id,
                    recipe_document_sha256: sha256_serialized(&recipe),
                },
                revision: GeneralSemExecutionAuthorityRevisionIdentityV1 {
                    project_id: Uuid::from_u128(0x7265_7669_7369_6f6e_5f70_726f_6a65_6302),
                    project_name: "General SEM revision 1".into(),
                    created_at: Utc.with_ymd_and_hms(2026, 8, 19, 10, 0, 0).unwrap(),
                    model_id: "model:general-sem-revision-1".into(),
                    model_name: "General SEM revision 1".into(),
                    recipe_id: Uuid::from_u128(0x7265_7669_7369_6f6e_5f72_6563_6970_6501),
                },
                intent: GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
                    intent_version: 1,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    label: "X x W".into(),
                    operands: ["construct:x".into(), "construct:w".into()],
                    focal_relation,
                    outcome: "construct:y".into(),
                    method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                    hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
                },
                expected_capability_cell: if bootstrap {
                    qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
                } else {
                    qpls_core::pls_general_multiple_moderation_point_capability_cell_v1()
                },
                recipe_execution_surface: GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1.into(),
            };
            SourceFixture {
                source,
                destination,
                source_bytes,
                source_sha256,
                dataset,
                model,
                recipe,
                request,
            }
        }

        fn structural_relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
            model
                .relations
                .iter()
                .find_map(|relation| match relation {
                    SemRelationV4::Structural {
                        id,
                        source: observed_source,
                        target: observed_target,
                        role: StructuralRelationRoleV4::Structural,
                        ..
                    } if observed_source == source && observed_target == target => Some(id.clone()),
                    _ => None,
                })
                .unwrap()
        }

        fn moderated_mediation_recipe_and_model(
            stage: ModeratedMediationStageFixture,
        ) -> (Dataset, AnalysisRecipeV4, SemModelV4, [String; 2]) {
            let source_model = ModelSpec {
                id: Uuid::from_u128(match stage {
                    ModeratedMediationStageFixture::First => {
                        0x7265_7632_5f66_6972_7374_5f6d_6f64_656c
                    }
                    ModeratedMediationStageFixture::Second => {
                        0x7265_7632_7365_636f_6e64_6d6f_6465_6c01
                    }
                }),
                name: "Moderated mediation revision source".into(),
                constructs: ["x", "m", "w", "y"]
                    .into_iter()
                    .map(|id| Construct {
                        id: id.into(),
                        name: id.to_uppercase(),
                        short_name: id.to_uppercase(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec![format!("{id}1"), format!("{id}2")],
                    })
                    .collect(),
                paths: [("x", "m"), ("m", "y"), ("x", "y"), ("w", "m"), ("w", "y")]
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
            let dataset = import_delimited_bytes(
                b"x1,x2,m1,m2,w1,w2,y1,y2\n1,2,2,1,2,1,2,1\n2,1,3,2,3,2,3,2\n3,4,4,3,4,3,5,4\n4,3,5,5,5,5,6,5\n5,6,7,6,7,6,8,7\n6,5,6,7,6,7,9,8\n7,8,9,7,9,7,11,9\n8,7,8,9,8,9,10,11\n9,10,11,10,11,10,13,12\n10,9,12,11,12,11,14,13\n11,12,13,12,13,12,16,15\n12,11,14,13,14,13,17,16\n",
                "moderated-mediation-revision.csv",
                b',',
                &ImportOptions::default(),
            )
            .unwrap();
            let source_recipe = AnalysisRecipe {
                schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
                id: Uuid::from_u128(0x7265_7632_5f73_6f75_7263_655f_7265_6301),
                created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
                dataset_fingerprint: dataset.fingerprint.0.clone(),
                model: source_model.clone(),
                settings: AnalysisSettings {
                    method: AnalysisMethod::PlsPm,
                    workers: 1,
                    ..AnalysisSettings::default()
                },
                method_config: Some(MethodConfig::PlsAlgorithm),
                metadata: BTreeMap::new(),
            };
            let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe).unwrap();
            let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
                &pending,
                &source_model,
                &[],
                LegacyBasicModelInterpretationV4::PlsComposite,
            )
            .unwrap();
            let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
                unreachable!()
            };
            *dataset_id = dataset.id.to_string();
            let path = [
                structural_relation_id(&model, "construct:x", "construct:m"),
                structural_relation_id(&model, "construct:m", "construct:y"),
            ];
            let (focal_relation, operands, outcome, label) = match stage {
                ModeratedMediationStageFixture::First => (
                    path[0].clone(),
                    ["construct:x".into(), "construct:w".into()],
                    "construct:m".to_string(),
                    "X x W",
                ),
                ModeratedMediationStageFixture::Second => (
                    path[1].clone(),
                    ["construct:m".into(), "construct:w".into()],
                    "construct:y".to_string(),
                    "M x W",
                ),
            };
            apply_interaction_v2_revision(
                &mut model,
                &GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
                    intent_version: 1,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    label: label.into(),
                    operands,
                    focal_relation,
                    outcome,
                    method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                    hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
                },
            )
            .unwrap();
            model.ensure_valid().unwrap();
            let seed = 20_260_820;
            let resamples = 20;
            recipe.settings.bootstrap_samples = resamples;
            recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
            recipe.settings.studentized_inner_samples = 0;
            recipe.settings.seed = seed;
            recipe.settings.confidence_level = 0.95;
            recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model.id.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            };
            let mut config = GeneralSemConfigV1::default();
            config.inference = GeneralSemInferenceV1::CaseBootstrap {
                resamples,
                seed,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            };
            recipe.general_sem_config = Some(config);
            recipe.ensure_valid().unwrap();
            (dataset, recipe, model, path)
        }

        fn write_moderated_mediation_source(
            directory: &Path,
            stage: ModeratedMediationStageFixture,
        ) -> ModeratedMediationSourceFixture {
            let suffix = match stage {
                ModeratedMediationStageFixture::First => "first",
                ModeratedMediationStageFixture::Second => "second",
            };
            let source = directory.join(format!("moderated-mediation-{suffix}-source.qpls"));
            let destination = directory.join(format!("moderated-mediation-{suffix}-revision.qpls"));
            let (dataset, recipe, model, ordered_relation_ids) =
                moderated_mediation_recipe_and_model(stage);
            let (source_project_id, revision_project_id, revision_recipe_id) = match stage {
                ModeratedMediationStageFixture::First => (
                    Uuid::from_u128(0x7265_7632_5f66_6972_7374_5f70_726f_6a01),
                    Uuid::from_u128(0x7265_7632_5f66_6972_7374_5f70_726f_6a02),
                    Uuid::from_u128(0x7265_7632_5f66_6972_7374_5f72_6563_6901),
                ),
                ModeratedMediationStageFixture::Second => (
                    Uuid::from_u128(0x7265_7632_7365_636f_6e64_5f70_726f_6a01),
                    Uuid::from_u128(0x7265_7632_7365_636f_6e64_5f70_726f_6a02),
                    Uuid::from_u128(0x7265_7632_7365_636f_6e64_5f72_6563_6901),
                ),
            };
            create_populated_general_sem_project_archive_v6(
                &source,
                source_project_id,
                "Moderated mediation source",
                Utc.with_ymd_and_hms(2026, 8, 20, 9, 0, 0).unwrap(),
                &dataset,
                model.clone(),
                recipe.clone(),
            )
            .unwrap();
            let source_bytes = fs::read(&source).unwrap();
            let source_sha256 = archive_sha256(&source_bytes);
            let estimand_id = specific_directed_path_identity_v1(&ordered_relation_ids);
            let request = GeneralSemExecutionAuthorityRevisionRequestV2 {
                source: GeneralSemExecutionAuthoritySourcePinV1 {
                    project_id: source_project_id,
                    model_id: model.id.clone(),
                    model_document_sha256: model.model_document_sha256().unwrap(),
                    model_scientific_sha256: model.scientific_sha256().unwrap(),
                    recipe_id: recipe.id,
                    recipe_document_sha256: sha256_serialized(&recipe),
                },
                revision: GeneralSemExecutionAuthorityRevisionIdentityV1 {
                    project_id: revision_project_id,
                    project_name: format!("Moderated mediation {suffix}-stage revision"),
                    created_at: Utc.with_ymd_and_hms(2026, 8, 20, 10, 0, 0).unwrap(),
                    model_id: format!("model:moderated-mediation-{suffix}-revision"),
                    model_name: format!("Moderated mediation {suffix}-stage revision"),
                    recipe_id: revision_recipe_id,
                },
                intent: GeneralSemExecutionAuthorityRevisionIntentV2::SelectTwoWayModeratedMediationPath {
                    intent_version: 2,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    estimand_id,
                    ordered_relation_ids,
                },
            };
            ModeratedMediationSourceFixture {
                source,
                destination,
                source_bytes,
                source_sha256,
                dataset,
                model,
                recipe,
                request,
            }
        }

        fn write_source(directory: &Path) -> SourceFixture {
            write_source_with_bootstrap(directory, false)
        }

        fn assert_source_unchanged(fixture: &SourceFixture) {
            let observed = fs::read(&fixture.source).unwrap();
            assert_eq!(observed, fixture.source_bytes);
            assert_eq!(archive_sha256(&observed), fixture.source_sha256);
        }

        fn assert_moderated_mediation_source_unchanged(fixture: &ModeratedMediationSourceFixture) {
            let observed = fs::read(&fixture.source).unwrap();
            assert_eq!(observed, fixture.source_bytes);
            assert_eq!(archive_sha256(&observed), fixture.source_sha256);
        }

        #[test]
        fn revision_publishes_a_strict_reopened_new_authority_and_preserves_source_bytes() {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_source(directory.path());

            let receipt = create_general_sem_execution_authority_revision_v1(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            )
            .unwrap();

            assert_source_unchanged(&fixture);
            assert!(temporary_links(directory.path()).is_empty());
            assert_eq!(receipt.schema_version, 1);
            assert_eq!(
                receipt.archive_schema_version,
                PROJECT_ARCHIVE_SCHEMA_V6_VERSION
            );
            assert_eq!(receipt.revision_number, 1);
            assert!(receipt.source_verified_unchanged);
            assert!(receipt.strict_reopen_validated);
            assert_eq!(
                receipt.source_archive_bytes,
                fixture.source_bytes.len() as u64
            );
            assert_eq!(receipt.source_archive_sha256, fixture.source_sha256);
            assert_eq!(receipt.source_project_id, fixture.request.source.project_id);
            assert_eq!(receipt.source_model_id, fixture.model.id);
            assert_eq!(receipt.source_recipe_id, fixture.recipe.id);
            assert_eq!(
                receipt.destination_archive_sha256,
                archive_sha256(&fs::read(&fixture.destination).unwrap())
            );
            assert_eq!(
                receipt.destination_archive_bytes,
                fs::metadata(&fixture.destination).unwrap().len()
            );

            let reopened = load_project_archive_v6(&fixture.destination).unwrap();
            assert!(reopened.document.supports_general_sem_v1());
            assert!(matches!(
                reopened.document.origin,
                ProjectOriginV6::NewProject
            ));
            assert_eq!(
                reopened.document.sem_generation,
                Some(ProjectSemGenerationV6::GeneralSemV1)
            );
            assert_eq!(
                reopened.document.project_id,
                fixture.request.revision.project_id
            );
            assert_eq!(reopened.document.datasets.len(), 1);
            assert_eq!(reopened.datasets.len(), 1);
            assert_eq!(reopened.document.models.len(), 1);
            assert_eq!(reopened.document.recipes.len(), 1);
            assert!(reopened.document.historical_recipes.is_empty());
            assert!(reopened.document.historical_results.is_empty());
            assert!(reopened.document.canonical_result_documents.is_empty());
            assert_eq!(reopened.datasets[0].id, fixture.dataset.id);
            assert_eq!(
                reopened.datasets[0].fingerprint.0,
                fixture.dataset.fingerprint.0
            );

            let ProjectModelPayloadV6::SemModelV4 {
                model: revised_model,
                scientific_sha256,
            } = &reopened.document.models[0].payload
            else {
                panic!("revision must contain a ready SemModelV4 authority")
            };
            assert_eq!(revised_model.id, fixture.request.revision.model_id);
            assert_ne!(revised_model.id, fixture.model.id);
            assert_eq!(
                revised_model.model_document_sha256().unwrap(),
                receipt.resident_model_document_sha256
            );
            assert_eq!(
                revised_model.scientific_sha256().unwrap(),
                receipt.resident_model_scientific_sha256
            );
            assert_eq!(scientific_sha256, &receipt.resident_model_scientific_sha256);
            assert!(revised_model.derived_terms.iter().any(|term| {
                matches!(term, SemDerivedTermV4::InteractionV2 { id, output, operands, focal_relation, method: InteractionMethodV4::TwoStage, hierarchy_policy: InteractionHierarchyPolicyV2::Strong, product_indicator: None }
                    if id == &receipt.interaction_term_id
                        && output == &receipt.interaction_output_id
                        && operands.len() == 2
                        && operands[0] == "construct:x"
                        && operands[1] == "construct:w"
                        && focal_relation == match &fixture.request.intent {
                            GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 { focal_relation, .. } => focal_relation,
                            GeneralSemExecutionAuthorityRevisionIntentV1::AddHigherOrder { .. } => unreachable!("interaction fixture"),
                        })
            }));

            let revised_recipe = &reopened.document.recipes[0];
            assert_eq!(revised_recipe.id, fixture.request.revision.recipe_id);
            assert_ne!(revised_recipe.id, fixture.recipe.id);
            assert_eq!(
                sha256_serialized(revised_recipe),
                receipt.resident_recipe_document_sha256
            );
            assert!(matches!(
                &revised_recipe.model_binding,
                AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                    model_id,
                    scientific_sha256,
                } if model_id == &receipt.resident_model_id
                    && scientific_sha256 == &receipt.resident_model_scientific_sha256
            ));
            let recompiled =
                compile_general_sem_pls_recipe_v1(revised_recipe, Some(revised_model)).unwrap();
            assert_eq!(recompiled.compiler_version(), receipt.compiler_version);
            assert_eq!(recompiled.capability_cell(), &receipt.capability_cell);
            assert_eq!(
                recompiled.recipe_analytical_sha256(),
                receipt.recipe_analytical_sha256
            );
            assert_eq!(
                recompiled.general_sem_config_sha256(),
                receipt.general_sem_config_sha256
            );
            assert_eq!(
                recompiled.plan().deterministic_sha256(),
                receipt.compiled_plan_sha256
            );
            assert_eq!(
                recompiled.artifact_identity_sha256(),
                receipt.compiled_artifact_identity_sha256
            );

            let lineage: GeneralSemExecutionAuthorityRevisionLineageV1 = serde_json::from_value(
                reopened.document.layouts[GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V1_LAYOUT_KEY]
                    .clone(),
            )
            .unwrap();
            assert_eq!(lineage.schema_version, 1);
            assert_eq!(lineage.revision_number, 1);
            assert_eq!(lineage.parent_revision_number, 0);
            assert_eq!(lineage.source_archive_sha256, fixture.source_sha256);
            assert_eq!(
                lineage.source_archive_bytes,
                fixture.source_bytes.len() as u64
            );
            assert_eq!(lineage.source.project_id, fixture.request.source.project_id);
            assert_eq!(
                lineage.revised.project_id,
                fixture.request.revision.project_id
            );
            assert_eq!(lineage.revised.model_id, receipt.resident_model_id);
            assert_eq!(lineage.revised.recipe_id, receipt.resident_recipe_id);
            assert_eq!(
                lineage.compilation.compiler_version,
                receipt.compiler_version
            );
            assert_eq!(lineage.compilation.capability_cell, receipt.capability_cell);
            assert_eq!(lineage.intent, fixture.request.intent);
        }

        #[test]
        fn bootstrap_revision_authorizes_supplemental_cell_but_preserves_point_primary_compilation()
        {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_source_with_bootstrap(directory.path(), true);
            assert_eq!(
                fixture.request.expected_capability_cell,
                qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()
            );

            let receipt = create_general_sem_execution_authority_revision_v1(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            )
            .unwrap();

            assert_source_unchanged(&fixture);
            assert_eq!(
                receipt.capability_cell,
                qpls_core::pls_general_multiple_moderation_point_capability_cell_v1()
            );
            let reopened = load_project_archive_v6(&fixture.destination).unwrap();
            let revised_recipe = &reopened.document.recipes[0];
            assert!(matches!(
                revised_recipe
                    .general_sem_config
                    .as_ref()
                    .map(|config| config.inference),
                Some(GeneralSemInferenceV1::CaseBootstrap { .. })
            ));
            assert_eq!(revised_recipe.settings.bootstrap_samples, 500);
            assert_eq!(
                revised_recipe
                    .metadata
                    .get("execution_surface")
                    .map(String::as_str),
                Some(GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1)
            );
        }

        #[test]
        fn stale_source_digest_fails_closed_without_destination_or_temporary() {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_source(directory.path());
            let stale_sha256 = if fixture.source_sha256 == "0".repeat(64) {
                "1".repeat(64)
            } else {
                "0".repeat(64)
            };

            let result = create_general_sem_execution_authority_revision_v1(
                &fixture.source,
                &stale_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::Publication(
                    ProjectArchiveV6SaveCopyError::SourceDigestMismatch { expected, observed }
                )) if expected == stale_sha256 && observed == fixture.source_sha256
            ));
            assert_source_unchanged(&fixture);
            assert!(!fixture.destination.exists());
            assert!(temporary_links(directory.path()).is_empty());
        }

        #[test]
        fn unsupported_intent_fails_closed_without_destination_or_temporary() {
            let directory = tempfile::tempdir().unwrap();
            let mut fixture = write_source(directory.path());
            let GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
                intent_version,
                ..
            } = &mut fixture.request.intent
            else {
                unreachable!("interaction fixture")
            };
            *intent_version = 2;

            let result = create_general_sem_execution_authority_revision_v1(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(_))
            ));
            assert_source_unchanged(&fixture);
            assert!(!fixture.destination.exists());
            assert!(temporary_links(directory.path()).is_empty());
        }

        #[test]
        fn destination_no_replace_failure_preserves_both_source_and_existing_bytes() {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_source(directory.path());
            let sentinel = b"existing destination bytes";
            fs::write(&fixture.destination, sentinel).unwrap();

            let result = create_general_sem_execution_authority_revision_v1(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::Publication(
                    ProjectArchiveV6SaveCopyError::DestinationExists(path)
                )) if path == fixture.destination
            ));
            assert_source_unchanged(&fixture);
            assert_eq!(fs::read(&fixture.destination).unwrap(), sentinel);
            assert!(temporary_links(directory.path()).is_empty());
        }

        #[test]
        fn revision_v2_publishes_exact_first_and_second_stage_authorities() {
            for stage in [
                ModeratedMediationStageFixture::First,
                ModeratedMediationStageFixture::Second,
            ] {
                let directory = tempfile::tempdir().unwrap();
                let fixture = write_moderated_mediation_source(directory.path(), stage);
                let receipt = create_general_sem_execution_authority_revision_v2(
                    &fixture.source,
                    &fixture.source_sha256,
                    &fixture.destination,
                    fixture.request.clone(),
                )
                .unwrap();

                assert_moderated_mediation_source_unchanged(&fixture);
                assert!(temporary_links(directory.path()).is_empty());
                assert_eq!(receipt.schema_version, 2);
                assert_eq!(receipt.revision_number, 1);
                assert!(receipt.source_verified_unchanged);
                assert!(receipt.strict_reopen_validated);
                assert_eq!(receipt.source_recipe_id, fixture.recipe.id);
                assert_eq!(receipt.resident_dataset_id, fixture.dataset.id);
                assert_eq!(
                    receipt.supplemental_capability_cell,
                    pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1()
                );
                assert_eq!(
                    receipt.capability_dependencies,
                    sorted_target_dependencies_v2(&receipt.compiled_target)
                );
                assert_eq!(
                    receipt.compiled_target_sha256,
                    sha256_serialized(&receipt.compiled_target)
                );

                let reopened = load_project_archive_v6(&fixture.destination).unwrap();
                assert_eq!(reopened.document.models.len(), 1);
                assert_eq!(reopened.document.recipes.len(), 1);
                assert!(reopened.document.canonical_result_documents.is_empty());
                assert!(reopened.document.historical_results.is_empty());
                let ProjectModelPayloadV6::SemModelV4 {
                    model: revised_model,
                    scientific_sha256,
                } = &reopened.document.models[0].payload
                else {
                    panic!("revision-v2 must contain a ready SemModelV4 authority")
                };
                let mut expected_model = fixture.model.clone();
                expected_model.id = fixture.request.revision.model_id.clone();
                expected_model.name = fixture.request.revision.model_name.clone();
                assert_eq!(revised_model, &expected_model);
                assert_eq!(scientific_sha256, &receipt.resident_model_scientific_sha256);

                let revised_recipe = &reopened.document.recipes[0];
                let selected_effect = selected_effect_for_revision_v2(&fixture.request.intent);
                assert_eq!(
                    revised_recipe
                        .general_sem_config
                        .as_ref()
                        .unwrap()
                        .requested_effect_estimands,
                    vec![selected_effect]
                );
                assert!(matches!(
                    &revised_recipe.model_binding,
                    AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                        model_id,
                        scientific_sha256,
                    } if model_id == &receipt.resident_model_id
                        && scientific_sha256 == &receipt.resident_model_scientific_sha256
                ));
                let recompiled =
                    compile_general_sem_pls_recipe_v1(revised_recipe, Some(revised_model))
                        .unwrap();
                assert_eq!(recompiled.compiler_version(), receipt.compiler_version);
                assert_eq!(
                    recompiled.capability_cell(),
                    &receipt.primary_capability_cell
                );
                assert_eq!(
                    recompiled.plan().deterministic_sha256(),
                    receipt.compiled_plan_sha256
                );
                assert_eq!(
                    recompiled.artifact_identity_sha256(),
                    receipt.compiled_artifact_identity_sha256
                );
                assert_eq!(
                    recompiled.plan().two_way_moderated_mediation_target(),
                    Some(&receipt.compiled_target)
                );

                let lineage: GeneralSemExecutionAuthorityRevisionLineageV2 =
                    serde_json::from_value(
                        reopened.document.layouts
                            [GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY]
                            .clone(),
                    )
                    .unwrap();
                assert_eq!(lineage.schema_version, 2);
                assert_eq!(lineage.revision_number, 1);
                assert_eq!(lineage.parent_revision_number, 0);
                assert_eq!(lineage.source_archive_sha256, fixture.source_sha256);
                assert_eq!(lineage.compiled_target, receipt.compiled_target);
                assert_eq!(lineage.intent, fixture.request.intent);
            }
        }

        #[test]
        fn revision_v2_stale_sha_rejection_preserves_source_and_creates_nothing() {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_moderated_mediation_source(
                directory.path(),
                ModeratedMediationStageFixture::First,
            );
            let stale_sha256 = if fixture.source_sha256 == "0".repeat(64) {
                "1".repeat(64)
            } else {
                "0".repeat(64)
            };

            let result = create_general_sem_execution_authority_revision_v2(
                &fixture.source,
                &stale_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::Publication(
                    ProjectArchiveV6SaveCopyError::SourceDigestMismatch { expected, observed }
                )) if expected == stale_sha256 && observed == fixture.source_sha256
            ));
            assert_moderated_mediation_source_unchanged(&fixture);
            assert!(!fixture.destination.exists());
            assert!(temporary_links(directory.path()).is_empty());
        }

        #[test]
        fn revision_v2_rejected_path_preserves_source_and_creates_nothing() {
            let directory = tempfile::tempdir().unwrap();
            let mut fixture = write_moderated_mediation_source(
                directory.path(),
                ModeratedMediationStageFixture::Second,
            );
            let GeneralSemExecutionAuthorityRevisionIntentV2::SelectTwoWayModeratedMediationPath {
                estimand_id,
                ordered_relation_ids,
                ..
            } = &mut fixture.request.intent;
            ordered_relation_ids[1] = "relation:not-authored".into();
            *estimand_id = specific_directed_path_identity_v1(ordered_relation_ids);

            let result = create_general_sem_execution_authority_revision_v2(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::Compilation(_))
            ));
            assert_moderated_mediation_source_unchanged(&fixture);
            assert!(!fixture.destination.exists());
            assert!(temporary_links(directory.path()).is_empty());
        }

        #[test]
        fn revision_v2_destination_no_replace_preserves_source_and_destination_bytes() {
            let directory = tempfile::tempdir().unwrap();
            let fixture = write_moderated_mediation_source(
                directory.path(),
                ModeratedMediationStageFixture::First,
            );
            let sentinel = b"existing revision-v2 destination bytes";
            fs::write(&fixture.destination, sentinel).unwrap();

            let result = create_general_sem_execution_authority_revision_v2(
                &fixture.source,
                &fixture.source_sha256,
                &fixture.destination,
                fixture.request.clone(),
            );

            assert!(matches!(
                result,
                Err(GeneralSemExecutionAuthorityRevisionErrorV1::Publication(
                    ProjectArchiveV6SaveCopyError::DestinationExists(path)
                )) if path == fixture.destination
            ));
            assert_moderated_mediation_source_unchanged(&fixture);
            assert_eq!(fs::read(&fixture.destination).unwrap(), sentinel);
            assert!(temporary_links(directory.path()).is_empty());
        }
    }
}
