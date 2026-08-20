//! Registry-driven bridge for versioned General SEM authority revisions.

use crate::general_sem_registry_access_v1::{
    GeneralSemRegistryAccessErrorV1, authorize_general_sem_registry_access_v1,
    general_sem_recipe_execution_surface_v1, is_general_sem_execution_cell_v1,
};

use qpls_project::{
    GeneralSemExecutionAuthorityRevisionErrorV1, GeneralSemExecutionAuthorityRevisionReceiptV1,
    GeneralSemExecutionAuthorityRevisionRequestV1, ProjectArchiveV6SaveCopyError,
    create_general_sem_execution_authority_revision_v1,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const DIAGNOSTIC_PREFIX: &str = "schema6_general_sem_revision";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemExecutionAuthorityRevisionCommandRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    source_archive_path: String,
    expected_source_archive_sha256: String,
    destination_archive_path: String,
    revision: GeneralSemExecutionAuthorityRevisionRequestV1,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemExecutionAuthorityRevisionDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemExecutionAuthorityRevisionResultV1 {
    schema_version: u32,
    persistence: &'static str,
    receipt: GeneralSemExecutionAuthorityRevisionReceiptV1,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    Ok {
        value: GeneralSemExecutionAuthorityRevisionResultV1,
    },
    Blocked {
        diagnostic: GeneralSemExecutionAuthorityRevisionDiagnosticV1,
    },
}

fn blocked(
    suffix: &str,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    GeneralSemExecutionAuthorityRevisionOutcomeV1::Blocked {
        diagnostic: GeneralSemExecutionAuthorityRevisionDiagnosticV1 {
            code: format!("{DIAGNOSTIC_PREFIX}.{suffix}"),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn map_revision_error(
    error: GeneralSemExecutionAuthorityRevisionErrorV1,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    match error {
        GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedPlatform => blocked(
            "windows_desktop_required",
            "General SEM authority revision requires the installed Windows desktop writer.",
            "Use the installed QuickPLS Windows application through the Registry-authorized General SEM workflow.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::InvalidRequest(message) => blocked(
            "request_invalid",
            message,
            "Refresh the active marked project and rebuild the revision from its exact current authority.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::SourceAuthorityMismatch(message) => blocked(
            "source_authority_stale",
            message,
            "Preserve the source unchanged, reopen it strictly, and retry from its current model and RecipeV4 identities.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::UnsupportedIntent(message) => blocked(
            "interaction_unsupported",
            message,
            "Select a qualified predictor-to-outcome path and distinct moderator, then retry the exact two-stage strong-hierarchy intent.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::Model(message) => blocked(
            "model_invalid",
            message,
            "Keep the source unchanged and correct the model before creating a new revision.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::Compilation(error) => blocked(
            "recipe_compilation_blocked",
            error.to_string(),
            "Use a General SEM RecipeV4 capability cell that explicitly supports this exact revised interaction model.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::Project(error) => blocked(
            "source_archive_invalid",
            error.to_string(),
            "Reopen a trusted, strictly valid schema-6 General SEM source archive.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::Publication(error) => {
            map_publication_error(error)
        }
        GeneralSemExecutionAuthorityRevisionErrorV1::Io(error) => blocked(
            "io_failed",
            error.to_string(),
            "Confirm the source and local destination folders are available, then retry with a new destination filename.",
        ),
        GeneralSemExecutionAuthorityRevisionErrorV1::Json(error) => blocked(
            "lineage_invalid",
            error.to_string(),
            "Preserve the source unchanged and reopen a revision with valid version-1 lineage.",
        ),
    }
}

fn registry_access_block(
    error: GeneralSemRegistryAccessErrorV1,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    match error {
        GeneralSemRegistryAccessErrorV1::RegistryInvalid(detail) => blocked(
            "capability_registry_invalid",
            format!("Capability Registry V2 is invalid: {detail}"),
            "Keep the source unchanged and repair the embedded registry before creating a revision.",
        ),
        GeneralSemRegistryAccessErrorV1::CapabilityUnavailable => blocked(
            "capability_unavailable",
            "The exact revised General SEM moderation cell is not executable.",
            "Refresh exact estimator access and rebuild the revision from the unchanged source.",
        ),
        GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired => blocked(
            "standard_surface_required",
            "The exact revised General SEM moderation cell requires the Standard surface.",
            "Refresh capability access and retry through Standard without changing the source.",
        ),
        GeneralSemRegistryAccessErrorV1::InternalLabsRequired => blocked(
            "internal_labs_required",
            "The exact revised General SEM moderation cell requires Experimental Labs opt-in.",
            "Enable Experimental Labs or use a Standard-qualified moderation cell.",
        ),
    }
}

fn map_publication_error(
    error: ProjectArchiveV6SaveCopyError,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    match error {
        ProjectArchiveV6SaveCopyError::DestinationExists(_) => blocked(
            "destination_exists",
            "The destination already exists and was not changed.",
            "Choose a new .qpls filename; General SEM revisions never overwrite a file.",
        ),
        ProjectArchiveV6SaveCopyError::SourceDigestMismatch { .. }
        | ProjectArchiveV6SaveCopyError::SourceChangedDuringSave => blocked(
            "source_archive_stale",
            error.to_string(),
            "Reinspect the unchanged source archive and retry from its exact current SHA-256 identity.",
        ),
        ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile => blocked(
            "source_not_supported",
            error.to_string(),
            "Choose the exact local regular .qpls source file, not a link or reparse point.",
        ),
        ProjectArchiveV6SaveCopyError::AbsolutePathsRequired
        | ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer
        | ProjectArchiveV6SaveCopyError::InvalidDestinationName(_)
        | ProjectArchiveV6SaveCopyError::DestinationExtension => blocked(
            "destination_invalid",
            error.to_string(),
            "Choose a different absolute local .qpls destination without surrounding whitespace.",
        ),
        ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory
        | ProjectArchiveV6SaveCopyError::RemoteDestinationUnsupported
        | ProjectArchiveV6SaveCopyError::UnsupportedDestinationFilesystem(_) => blocked(
            "destination_not_supported",
            error.to_string(),
            "Choose a local NTFS folder whose immediate parent is not a reparse point.",
        ),
        ProjectArchiveV6SaveCopyError::StrictReopenMismatch
        | ProjectArchiveV6SaveCopyError::DestinationIdentityChanged
        | ProjectArchiveV6SaveCopyError::PublicationFailed(_) => blocked(
            "commit_validation_failed",
            error.to_string(),
            "Keep the source unchanged and retry with a new local destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::UnsupportedPlatform => blocked(
            "windows_desktop_required",
            error.to_string(),
            "Use the installed QuickPLS Windows desktop application.",
        ),
        ProjectArchiveV6SaveCopyError::ArchiveLimit(_) => blocked(
            "archive_limit",
            error.to_string(),
            "Reduce unsupported archive content and retry with a new destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::Contract(_)
        | ProjectArchiveV6SaveCopyError::Project(_)
        | ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets
        | ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged => blocked(
            "authority_invalid",
            error.to_string(),
            "Preserve the source and correct the model, recipe, dataset, or lineage authority before retrying.",
        ),
        ProjectArchiveV6SaveCopyError::CancelledBeforeCommit => blocked(
            "cancelled_before_commit",
            error.to_string(),
            "Retry when ready; no destination was published.",
        ),
        ProjectArchiveV6SaveCopyError::Io(_)
        | ProjectArchiveV6SaveCopyError::Zip(_)
        | ProjectArchiveV6SaveCopyError::Json(_) => blocked(
            "write_failed",
            error.to_string(),
            "Confirm the local files are available and retry with a new destination filename.",
        ),
    }
}

fn revise_using<F>(
    request: GeneralSemExecutionAuthorityRevisionCommandRequestV1,
    authorize: F,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1
where
    F: Fn(
        &str,
        bool,
        &qpls_core::CapabilityCellReferenceV2,
    ) -> Result<(), GeneralSemRegistryAccessErrorV1>,
{
    let capability_cell = &request.revision.expected_capability_cell;
    if !is_general_sem_execution_cell_v1(capability_cell)
        || !matches!(
            capability_cell.capability_id.as_str(),
            "smartpls.moderation" | "smartpls.higher_order_models"
        )
    {
        return blocked(
            "capability_unavailable",
            "General SEM scientific revision requires an exact bounded moderation or higher-order point/bootstrap cell.",
            "Refresh the marked General SEM workflow before choosing a destination.",
        );
    }
    if let Err(error) = authorize(
        &request.surface,
        request.experimental_labs_enabled,
        capability_cell,
    ) {
        return registry_access_block(error);
    }
    if general_sem_recipe_execution_surface_v1(&request.surface)
        != Some(request.revision.recipe_execution_surface.as_str())
    {
        return blocked(
            "recipe_execution_surface_mismatch",
            "The revised RecipeV4 execution-surface identity disagrees with its Registry-authorized command surface.",
            "Refresh exact capability access and rebuild the revision request from the unchanged source.",
        );
    }
    for (field, value) in [
        ("sourceArchivePath", request.source_archive_path.as_str()),
        (
            "destinationArchivePath",
            request.destination_archive_path.as_str(),
        ),
    ] {
        if value.is_empty() || value.trim() != value {
            return blocked(
                "path_invalid",
                format!("{field} must be nonempty without surrounding whitespace."),
                "Reinspect the source and choose a new absolute .qpls destination.",
            );
        }
    }

    match create_general_sem_execution_authority_revision_v1(
        Path::new(&request.source_archive_path),
        &request.expected_source_archive_sha256,
        Path::new(&request.destination_archive_path),
        request.revision,
    ) {
        Ok(receipt) => GeneralSemExecutionAuthorityRevisionOutcomeV1::Ok {
            value: GeneralSemExecutionAuthorityRevisionResultV1 {
                schema_version: 1,
                persistence: "persisted_new_revision",
                receipt,
            },
        },
        Err(error) => map_revision_error(error),
    }
}

fn revise(
    request: GeneralSemExecutionAuthorityRevisionCommandRequestV1,
) -> GeneralSemExecutionAuthorityRevisionOutcomeV1 {
    revise_using(request, authorize_general_sem_registry_access_v1)
}

#[tauri::command]
pub(crate) async fn revise_internal_general_sem_execution_authority_v1(
    request: GeneralSemExecutionAuthorityRevisionCommandRequestV1,
) -> Result<GeneralSemExecutionAuthorityRevisionOutcomeV1, String> {
    Ok(
        match tauri::async_runtime::spawn_blocking(move || revise(request)).await {
            Ok(outcome) => outcome,
            Err(_) => blocked(
                "worker_failed",
                "The General SEM revision worker stopped before returning an outcome.",
                "Keep the source unchanged and retry with a new destination filename.",
            ),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_project::{
        GeneralSemExecutionAuthorityRevisionIdentityV1,
        GeneralSemExecutionAuthorityRevisionIntentV1, GeneralSemExecutionAuthoritySourcePinV1,
        GeneralSemRevisionGenerationV1, GeneralSemRevisionHierarchyPolicyV1,
        GeneralSemRevisionInteractionMethodV1,
    };
    use uuid::Uuid;

    fn request(
        surface: &str,
        recipe_execution_surface: &str,
    ) -> GeneralSemExecutionAuthorityRevisionCommandRequestV1 {
        GeneralSemExecutionAuthorityRevisionCommandRequestV1 {
            surface: surface.into(),
            experimental_labs_enabled: surface == "internal_labs",
            source_archive_path: " not-a-path ".into(),
            expected_source_archive_sha256: "x".into(),
            destination_archive_path: "not-a-path".into(),
            revision: GeneralSemExecutionAuthorityRevisionRequestV1 {
                source: GeneralSemExecutionAuthoritySourcePinV1 {
                    project_id: Uuid::from_u128(1),
                    model_id: "model:source".into(),
                    model_document_sha256: "a".repeat(64),
                    model_scientific_sha256: "b".repeat(64),
                    recipe_id: Uuid::from_u128(2),
                    recipe_document_sha256: "c".repeat(64),
                },
                revision: GeneralSemExecutionAuthorityRevisionIdentityV1 {
                    project_id: Uuid::from_u128(3),
                    project_name: "Revision".into(),
                    created_at: chrono::Utc::now(),
                    model_id: "model:revision".into(),
                    model_name: "Revision".into(),
                    recipe_id: Uuid::from_u128(4),
                },
                intent: GeneralSemExecutionAuthorityRevisionIntentV1::AddGeneralSemInteractionV2 {
                    intent_version: 1,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    label: "X x W".into(),
                    operands: ["x".into(), "w".into()],
                    focal_relation: "x-y".into(),
                    outcome: "y".into(),
                    method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                    hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
                },
                expected_capability_cell:
                    qpls_core::pls_general_multiple_moderation_point_capability_cell_v1(),
                recipe_execution_surface: recipe_execution_surface.into(),
            },
        }
    }

    #[test]
    fn labs_gate_blocks_before_any_path_access() {
        let outcome = revise_using(
            request(
                "internal_labs",
                qpls_project::GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1,
            ),
            |_surface, _enabled, _cell| Err(GeneralSemRegistryAccessErrorV1::InternalLabsRequired),
        );
        assert!(matches!(
            outcome,
            GeneralSemExecutionAuthorityRevisionOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("internal_labs_required")
        ));
    }

    #[test]
    fn standard_authorized_revision_reaches_validation_without_labs_or_filesystem_access() {
        let outcome = revise_using(
            request(
                "standard",
                qpls_project::GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1,
            ),
            |surface, enabled, cell| {
                assert_eq!(surface, "standard");
                assert!(!enabled);
                assert_eq!(
                    cell,
                    &qpls_core::pls_general_multiple_moderation_point_capability_cell_v1()
                );
                Ok(())
            },
        );
        assert!(matches!(
            outcome,
            GeneralSemExecutionAuthorityRevisionOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("path_invalid")
        ));
    }
}
