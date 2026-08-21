//! Historical Internal/Labs bridge for creating a blank schema-6 General SEM
//! project. The command remains registered for backward-compatible internal
//! automation, but has no product TypeScript caller and intentionally cannot
//! create Standard projects. The Standard product workflow is New Project plus
//! the populated, Registry-authorized bootstrap command.
//!
//! The Labs gate and all wire-value parsing run before the core writer can
//! inspect the destination filesystem. Persistence then delegates entirely to
//! qpls-project's strict-reopen, no-replace schema-6 publication path.

use chrono::{DateTime, Utc};
use qpls_project::{
    GeneralSemProjectArchiveCreationErrorV1, GeneralSemProjectArchiveCreationReceiptV1,
    ProjectArchiveV6SaveCopyError, create_general_sem_project_archive_v6,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const GENERAL_SEM_PROJECT_CREATION_RESULT_SCHEMA_VERSION: u32 = 1;
const DIAGNOSTIC_CODE_PREFIX: &str = "schema6_general_sem_create";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveCreationRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    destination_path: String,
    project_id: String,
    name: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveCreationResultV1 {
    schema_version: u32,
    receipt: GeneralSemProjectArchiveCreationReceiptV1,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveCreationDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum GeneralSemProjectArchiveCreationOutcomeV1 {
    Ok {
        value: GeneralSemProjectArchiveCreationResultV1,
    },
    Blocked {
        diagnostic: GeneralSemProjectArchiveCreationDiagnosticV1,
    },
}

fn blocked(
    code_suffix: &str,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> GeneralSemProjectArchiveCreationOutcomeV1 {
    GeneralSemProjectArchiveCreationOutcomeV1::Blocked {
        diagnostic: GeneralSemProjectArchiveCreationDiagnosticV1 {
            code: format!("{DIAGNOSTIC_CODE_PREFIX}.{code_suffix}"),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn publication_error(
    error: GeneralSemProjectArchiveCreationErrorV1,
) -> GeneralSemProjectArchiveCreationOutcomeV1 {
    let GeneralSemProjectArchiveCreationErrorV1::Publication(error) = error;
    let (code_suffix, message, corrective_action) = match error {
        ProjectArchiveV6SaveCopyError::DestinationExists(_) => (
            "destination_exists",
            "The destination archive already exists and was not changed.",
            "Choose a new .qpls destination filename; existing files are never overwritten.",
        ),
        ProjectArchiveV6SaveCopyError::AbsolutePathsRequired
        | ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer
        | ProjectArchiveV6SaveCopyError::InvalidDestinationName(_)
        | ProjectArchiveV6SaveCopyError::DestinationExtension => (
            "destination_invalid",
            "The destination is not a valid absolute .qpls project path.",
            "Choose a new absolute .qpls destination path without surrounding whitespace.",
        ),
        ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory
        | ProjectArchiveV6SaveCopyError::RemoteDestinationUnsupported
        | ProjectArchiveV6SaveCopyError::UnsupportedDestinationFilesystem(_) => (
            "destination_not_supported",
            "The destination folder does not support safe schema-6 publication.",
            "Choose a local NTFS folder whose immediate parent is not a reparse point.",
        ),
        ProjectArchiveV6SaveCopyError::UnsupportedPlatform => (
            "windows_desktop_required",
            "Schema-6 General SEM project creation requires the Windows desktop writer.",
            "Use the installed QuickPLS Windows desktop application.",
        ),
        ProjectArchiveV6SaveCopyError::Io(_) => (
            "io_failed",
            "A local filesystem operation prevented the project archive from being created.",
            "Confirm the destination folder is available and writable, then retry with a new filename.",
        ),
        ProjectArchiveV6SaveCopyError::Contract(_)
        | ProjectArchiveV6SaveCopyError::Project(_)
        | ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets
        | ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile
        | ProjectArchiveV6SaveCopyError::SourceDigestMismatch { .. }
        | ProjectArchiveV6SaveCopyError::SourceChangedDuringSave
        | ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged => (
            "invalid_contract",
            "The requested blank General SEM project did not satisfy the schema-6 project contract.",
            "Keep the Labs session open and retry with a new blank General SEM project request.",
        ),
        ProjectArchiveV6SaveCopyError::ArchiveLimit(_) => (
            "archive_limit",
            "The project archive exceeded a schema-6 safety limit.",
            "Retry with a valid blank General SEM project request and a new destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::StrictReopenMismatch
        | ProjectArchiveV6SaveCopyError::DestinationIdentityChanged
        | ProjectArchiveV6SaveCopyError::PublicationFailed(_) => (
            "commit_validation_failed",
            "The project archive could not pass safe publication validation.",
            "Keep the Labs session open and retry with a new local destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::CancelledBeforeCommit
        | ProjectArchiveV6SaveCopyError::Zip(_)
        | ProjectArchiveV6SaveCopyError::Json(_) => (
            "write_failed",
            "The project archive could not be created.",
            "Keep the Labs session open and retry with a new local destination filename.",
        ),
    };
    blocked(code_suffix, message, corrective_action)
}

fn create_general_sem_project_archive(
    request: GeneralSemProjectArchiveCreationRequestV1,
) -> GeneralSemProjectArchiveCreationOutcomeV1 {
    // This gate must remain the first decision: denied callers cannot cause
    // path lookup, directory inspection, temporary creation, or publication.
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return blocked(
            "internal_labs_required",
            "Schema-6 General SEM project creation is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and use the internal General SEM new-project action.",
        );
    }

    if request.destination_path.is_empty()
        || request.destination_path != request.destination_path.trim()
    {
        return blocked(
            "destination_invalid",
            "The destination path must be nonempty and cannot contain surrounding whitespace.",
            "Choose a new absolute .qpls destination path without surrounding whitespace.",
        );
    }
    if request.name.trim().is_empty() || request.name != request.name.trim() {
        return blocked(
            "name_invalid",
            "The project name must be nonempty and cannot contain surrounding whitespace.",
            "Enter a project name without surrounding whitespace.",
        );
    }

    let project_id = match Uuid::parse_str(&request.project_id) {
        Ok(project_id) if !project_id.is_nil() => project_id,
        _ => {
            return blocked(
                "project_id_invalid",
                "The projectId value must be a non-nil UUID.",
                "Generate a new UUID for the project and retry.",
            );
        }
    };
    let created_at = match DateTime::parse_from_rfc3339(&request.created_at) {
        Ok(created_at) => created_at.with_timezone(&Utc),
        Err(_) => {
            return blocked(
                "created_at_invalid",
                "The createdAt value must be an RFC3339 timestamp.",
                "Provide createdAt in RFC3339 form, for example 2026-08-18T09:30:00Z.",
            );
        }
    };

    match create_general_sem_project_archive_v6(
        Path::new(&request.destination_path),
        project_id,
        request.name,
        created_at,
    ) {
        Ok(receipt) => GeneralSemProjectArchiveCreationOutcomeV1::Ok {
            value: GeneralSemProjectArchiveCreationResultV1 {
                schema_version: GENERAL_SEM_PROJECT_CREATION_RESULT_SCHEMA_VERSION,
                receipt,
            },
        },
        Err(error) => publication_error(error),
    }
}

#[tauri::command]
pub(crate) async fn create_internal_general_sem_project_archive_v6(
    request: GeneralSemProjectArchiveCreationRequestV1,
) -> GeneralSemProjectArchiveCreationOutcomeV1 {
    match tauri::async_runtime::spawn_blocking(move || create_general_sem_project_archive(request))
        .await
    {
        Ok(outcome) => outcome,
        Err(_) => blocked(
            "worker_failed",
            "The General SEM project creation worker stopped before returning an outcome.",
            "Keep the Labs session open and retry with a new local destination filename.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_project::{
        PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectOriginV6, ProjectSemGenerationV6,
        load_project_archive_v6,
    };
    use std::fs;

    fn request(destination: &Path) -> GeneralSemProjectArchiveCreationRequestV1 {
        GeneralSemProjectArchiveCreationRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            destination_path: destination.to_string_lossy().into_owned(),
            project_id: "60000001-0000-4000-8000-000000000001".into(),
            name: "General SEM bridge project".into(),
            created_at: "2026-08-18T09:30:00Z".into(),
        }
    }

    #[test]
    fn registered_blank_creator_remains_historical_labs_only_and_creates_nothing_when_denied() {
        let directory = tempfile::tempdir().unwrap();
        for (index, (surface, enabled)) in [("standard", true), (INTERNAL_LABS_SURFACE, false)]
            .into_iter()
            .enumerate()
        {
            let destination = directory.path().join(format!("blocked-{index}.qpls"));
            let mut denied = request(&destination);
            denied.surface = surface.into();
            denied.experimental_labs_enabled = enabled;

            assert!(matches!(
                create_general_sem_project_archive(denied),
                GeneralSemProjectArchiveCreationOutcomeV1::Blocked { diagnostic }
                    if diagnostic.code
                        == "schema6_general_sem_create.internal_labs_required"
            ));
            assert!(!destination.exists());
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn valid_request_creates_and_strictly_reopens_general_sem_v1() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("general-sem.qpls");

        let GeneralSemProjectArchiveCreationOutcomeV1::Ok { value } =
            create_general_sem_project_archive(request(&destination))
        else {
            panic!("valid General SEM creation request was blocked")
        };

        assert_eq!(
            value.schema_version,
            GENERAL_SEM_PROJECT_CREATION_RESULT_SCHEMA_VERSION
        );
        assert_eq!(
            value.receipt.destination_archive_path,
            destination.to_string_lossy().into_owned()
        );
        assert!(value.receipt.strict_reopen_validated);
        let reopened = load_project_archive_v6(&destination).unwrap();
        assert_eq!(
            reopened.document.schema_version,
            PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        );
        assert!(matches!(
            reopened.document.origin,
            ProjectOriginV6::NewProject
        ));
        assert_eq!(
            reopened.document.sem_generation,
            Some(ProjectSemGenerationV6::GeneralSemV1)
        );
        assert!(reopened.document.upgrade_lineage().is_none());
        assert!(reopened.document.datasets.is_empty());
        assert!(reopened.document.models.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn duplicate_destination_is_blocked_and_existing_bytes_are_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("existing.qpls");
        let sentinel = b"existing archive bytes";
        fs::write(&destination, sentinel).unwrap();

        assert!(matches!(
            create_general_sem_project_archive(request(&destination)),
            GeneralSemProjectArchiveCreationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_create.destination_exists"
        ));
        assert_eq!(fs::read(&destination).unwrap(), sentinel);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_timestamp_and_project_id_are_blocked_before_filesystem_action() {
        let directory = tempfile::tempdir().unwrap();
        let invalid_id_destination = directory.path().join("invalid-id.qpls");
        let mut invalid_id = request(&invalid_id_destination);
        invalid_id.project_id = "not-a-uuid".into();
        assert!(matches!(
            create_general_sem_project_archive(invalid_id),
            GeneralSemProjectArchiveCreationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_create.project_id_invalid"
        ));

        let invalid_timestamp_destination = directory.path().join("invalid-time.qpls");
        let mut invalid_timestamp = request(&invalid_timestamp_destination);
        invalid_timestamp.created_at = "18 August 2026".into();
        assert!(matches!(
            create_general_sem_project_archive(invalid_timestamp),
            GeneralSemProjectArchiveCreationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_create.created_at_invalid"
        ));
        assert!(!invalid_id_destination.exists());
        assert!(!invalid_timestamp_destination.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn request_wire_denies_unknown_fields() {
        let value = serde_json::json!({
            "surface": "internal_labs",
            "experimentalLabsEnabled": true,
            "destinationPath": r"D:\\projects\\new-general-sem.qpls",
            "projectId": "60000001-0000-4000-8000-000000000001",
            "name": "General SEM",
            "createdAt": "2026-08-18T09:30:00Z",
            "unexpected": true
        });
        assert!(
            serde_json::from_value::<GeneralSemProjectArchiveCreationRequestV1>(value).is_err()
        );
    }

    #[test]
    fn publication_errors_map_to_stable_camel_case_diagnostics() {
        let invalid_contract =
            publication_error(GeneralSemProjectArchiveCreationErrorV1::Publication(
                ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets,
            ));
        assert!(matches!(
            invalid_contract,
            GeneralSemProjectArchiveCreationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_create.invalid_contract"
        ));

        let io_failure = publication_error(GeneralSemProjectArchiveCreationErrorV1::Publication(
            ProjectArchiveV6SaveCopyError::Io(std::io::Error::other(
                "private native failure detail",
            )),
        ));
        let wire = serde_json::to_value(io_failure).unwrap();
        assert_eq!(wire["status"], "blocked");
        assert_eq!(
            wire["diagnostic"]["code"],
            "schema6_general_sem_create.io_failed"
        );
        assert!(wire["diagnostic"]["correctiveAction"].is_string());
        assert!(wire["diagnostic"].get("corrective_action").is_none());
        assert!(
            !wire["diagnostic"]["message"]
                .as_str()
                .unwrap()
                .contains("private native failure detail")
        );
    }
}
