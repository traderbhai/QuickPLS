//! Authority-gated bridge for the Windows-only schema-6 new-copy writer.
//!
//! The command accepts only a detached schema-6 document and the exact strict
//! source snapshot identity. It never reaches the Standard/schema-5 project,
//! autosave, recovery, or in-place save lanes.

use crate::multimod_candidate_authority_v1::multimod_standard_surface_authorized_v1;
use qpls_project::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectArchiveV6SaveCopyError,
    ProjectArchiveV6SaveCopyReceipt, ProjectManifest, load_project_archive_v6_from_file,
    save_project_archive_v6_model_copy, serialize_project_document_v6,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const STANDARD_MULTIMOD_SURFACE_V1: &str = "standard_multimod_v1";
const SAVE_COPY_RESULT_SCHEMA_VERSION: u32 = 1;
const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6SaveCopyRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    source_archive_path: String,
    expected_source_archive_sha256: String,
    destination_archive_path: String,
    project: ProjectArchiveDocumentV6,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6SaveCopyPersistenceV1 {
    PersistedNewCopy,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6SaveCopyAccessV1 {
    ReadOnly,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6SaveCopyLoaderV1 {
    StrictSchema6Zip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProjectArchiveV6SaveCopySurfaceErrorV1 {
    InternalLabsRequired,
    StandardAuthorityRequired,
    EmbeddedAuthorityInvalid(String),
    SurfaceInvalid,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectArchiveV6SaveCopyResidentDatasetV1 {
    dataset_id: String,
    name: String,
    fingerprint: String,
    row_count: usize,
    column_count: usize,
    sample_size: Option<usize>,
    arrow_resident: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectArchiveV6SaveCopyCountsV1 {
    datasets: usize,
    models: usize,
    recipes: usize,
    historical_recipes: usize,
    historical_results: usize,
    canonical_result_documents: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectArchiveV6SaveCopySnapshotV1 {
    schema_version: u32,
    access: ProjectArchiveV6SaveCopyAccessV1,
    loader: ProjectArchiveV6SaveCopyLoaderV1,
    archive_path: String,
    archive_sha256: String,
    archive_bytes: u64,
    manifest: ProjectManifest,
    project: ProjectArchiveDocumentV6,
    resident_datasets: Vec<ProjectArchiveV6SaveCopyResidentDatasetV1>,
    counts: ProjectArchiveV6SaveCopyCountsV1,
    source_rechecked_unchanged: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6SaveCopyResultV1 {
    schema_version: u32,
    persistence: ProjectArchiveV6SaveCopyPersistenceV1,
    receipt: ProjectArchiveV6SaveCopyReceipt,
    snapshot: ProjectArchiveV6SaveCopySnapshotV1,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6SaveCopyDiagnosticV1 {
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
pub(crate) enum ProjectArchiveV6SaveCopyOutcomeV1 {
    Ok {
        value: Box<ProjectArchiveV6SaveCopyResultV1>,
    },
    Blocked {
        diagnostic: ProjectArchiveV6SaveCopyDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectArchiveV6SaveCopyOutcomeV1 {
    ProjectArchiveV6SaveCopyOutcomeV1::Blocked {
        diagnostic: ProjectArchiveV6SaveCopyDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn validate_save_copy_surface_using_v1(
    surface: &str,
    experimental_labs_enabled: bool,
    standard_authorized: impl FnOnce() -> Result<bool, String>,
) -> Result<(), ProjectArchiveV6SaveCopySurfaceErrorV1> {
    match surface {
        INTERNAL_LABS_SURFACE if experimental_labs_enabled => Ok(()),
        INTERNAL_LABS_SURFACE => Err(ProjectArchiveV6SaveCopySurfaceErrorV1::InternalLabsRequired),
        STANDARD_MULTIMOD_SURFACE_V1 if experimental_labs_enabled => {
            Err(ProjectArchiveV6SaveCopySurfaceErrorV1::SurfaceInvalid)
        }
        STANDARD_MULTIMOD_SURFACE_V1 => match standard_authorized() {
            Ok(true) => Ok(()),
            Ok(false) => Err(ProjectArchiveV6SaveCopySurfaceErrorV1::StandardAuthorityRequired),
            Err(error) => {
                Err(ProjectArchiveV6SaveCopySurfaceErrorV1::EmbeddedAuthorityInvalid(error))
            }
        },
        _ => Err(ProjectArchiveV6SaveCopySurfaceErrorV1::SurfaceInvalid),
    }
}

fn validate_save_copy_surface_v1(
    surface: &str,
    experimental_labs_enabled: bool,
) -> Result<(), ProjectArchiveV6SaveCopySurfaceErrorV1> {
    validate_save_copy_surface_using_v1(
        surface,
        experimental_labs_enabled,
        multimod_standard_surface_authorized_v1,
    )
}

fn save_copy_surface_blocked(
    error: ProjectArchiveV6SaveCopySurfaceErrorV1,
) -> ProjectArchiveV6SaveCopyOutcomeV1 {
    match error {
        ProjectArchiveV6SaveCopySurfaceErrorV1::InternalLabsRequired => blocked(
            "schema6_save_copy.internal_labs_required",
            "Schema-6 Save copy through the historical internal surface requires explicit Experimental Labs opt-in.",
            "Enable Experimental Labs for the historical internal surface, or use a release-qualified Standard MultiMod package.",
        ),
        ProjectArchiveV6SaveCopySurfaceErrorV1::StandardAuthorityRequired => blocked(
            "schema6_save_copy.standard_authority_required",
            "Standard schema-6 Save copy requires release-qualified immutable MultiMod authority embedded in this executable.",
            "Use the exact release-qualified candidate package.",
        ),
        ProjectArchiveV6SaveCopySurfaceErrorV1::EmbeddedAuthorityInvalid(message) => blocked(
            "schema6_save_copy.embedded_authority_invalid",
            message,
            "Do not inspect, execute, save, or export MultiMod results from this executable.",
        ),
        ProjectArchiveV6SaveCopySurfaceErrorV1::SurfaceInvalid => blocked(
            "schema6_save_copy.surface_invalid",
            "The requested schema-6 Save copy surface is unsupported.",
            "Use standard_multimod_v1 for a release-qualified package or internal_labs with explicit Labs opt-in.",
        ),
    }
}

fn writer_error(error: ProjectArchiveV6SaveCopyError) -> ProjectArchiveV6SaveCopyOutcomeV1 {
    let (code, corrective_action) = match &error {
        ProjectArchiveV6SaveCopyError::DestinationExists(_) => (
            "schema6_save_copy.destination_exists",
            "Choose a new destination filename. Existing files are never overwritten.",
        ),
        ProjectArchiveV6SaveCopyError::SourceDigestMismatch { .. }
        | ProjectArchiveV6SaveCopyError::SourceChangedDuringSave => (
            "schema6_save_copy.stale_source",
            "Reopen the current strict schema-6 archive and retry from its latest digest.",
        ),
        ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged => (
            "schema6_save_copy.non_model_change_rejected",
            "Keep datasets, recipes, layouts, results, identity, and timestamps equal to the strict source snapshot.",
        ),
        ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory
        | ProjectArchiveV6SaveCopyError::RemoteDestinationUnsupported
        | ProjectArchiveV6SaveCopyError::UnsupportedDestinationFilesystem(_) => (
            "schema6_save_copy.destination_not_supported",
            "Choose a local NTFS folder whose immediate parent is not a reparse point.",
        ),
        ProjectArchiveV6SaveCopyError::AbsolutePathsRequired
        | ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer
        | ProjectArchiveV6SaveCopyError::InvalidDestinationName(_)
        | ProjectArchiveV6SaveCopyError::DestinationExtension => (
            "schema6_save_copy.destination_invalid",
            "Choose a different absolute .qpls destination path that does not already exist.",
        ),
        ProjectArchiveV6SaveCopyError::UnsupportedPlatform => (
            "schema6_save_copy.windows_desktop_required",
            "Use the installed QuickPLS Windows desktop application.",
        ),
        ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile => (
            "schema6_save_copy.source_invalid",
            "Reopen a regular local schema-6 .qpls source file and retry.",
        ),
        ProjectArchiveV6SaveCopyError::CancelledBeforeCommit => (
            "schema6_save_copy.cancelled",
            "Choose Save copy again when ready.",
        ),
        ProjectArchiveV6SaveCopyError::StrictReopenMismatch
        | ProjectArchiveV6SaveCopyError::DestinationIdentityChanged
        | ProjectArchiveV6SaveCopyError::PublicationFailed(_) => (
            "schema6_save_copy.commit_validation_failed",
            "Keep the current session open and retry with a new local destination.",
        ),
        ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets
        | ProjectArchiveV6SaveCopyError::ArchiveLimit(_)
        | ProjectArchiveV6SaveCopyError::Contract(_)
        | ProjectArchiveV6SaveCopyError::Project(_)
        | ProjectArchiveV6SaveCopyError::Io(_)
        | ProjectArchiveV6SaveCopyError::Zip(_)
        | ProjectArchiveV6SaveCopyError::Json(_) => (
            "schema6_save_copy.write_failed",
            "Keep the current session open, review the diagnostic, and retry with a new destination.",
        ),
    };
    blocked(code, error.to_string(), corrective_action)
}

fn open_source_preview(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path)
}

fn sha256_file_handle(file: &mut File) -> std::io::Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::new(file.try_clone()?);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(format!("{:x}", digest.finalize()))
}

fn save_copy(request: ProjectArchiveV6SaveCopyRequestV1) -> ProjectArchiveV6SaveCopyOutcomeV1 {
    if let Err(error) =
        validate_save_copy_surface_v1(&request.surface, request.experimental_labs_enabled)
    {
        return save_copy_surface_blocked(error);
    }

    if request.source_archive_path.is_empty()
        || request.destination_archive_path.is_empty()
        || request.source_archive_path != request.source_archive_path.trim()
        || request.destination_archive_path != request.destination_archive_path.trim()
    {
        return blocked(
            "schema6_save_copy.path_not_canonical",
            "Schema-6 archive paths must be nonempty and cannot have surrounding whitespace.",
            "Choose the source and destination again without editing their exact native paths.",
        );
    }
    let source = Path::new(&request.source_archive_path);
    let destination = Path::new(&request.destination_archive_path);

    let mut preview_file = match open_source_preview(source) {
        Ok(file) => file,
        Err(error) => {
            return blocked(
                "schema6_save_copy.source_unavailable",
                error.to_string(),
                "Reopen the current strict schema-6 source archive and retry.",
            );
        }
    };
    let preview_sha256 = match sha256_file_handle(&mut preview_file) {
        Ok(digest) => digest,
        Err(error) => {
            return blocked(
                "schema6_save_copy.source_unavailable",
                error.to_string(),
                "Reopen the current strict schema-6 source archive and retry.",
            );
        }
    };
    if preview_sha256 != request.expected_source_archive_sha256 {
        return blocked(
            "schema6_save_copy.stale_source",
            "The active schema-6 session digest no longer matches its source archive.",
            "Reopen the current strict schema-6 archive before saving a new copy.",
        );
    }

    let loaded = match preview_file
        .try_clone()
        .map_err(ProjectArchiveV6SaveCopyError::from)
        .and_then(|file| load_project_archive_v6_from_file(file).map_err(Into::into))
    {
        Ok(loaded) => loaded,
        Err(error) => {
            return blocked(
                "schema6_save_copy.source_invalid",
                error.to_string(),
                "Reopen a trusted strict schema-6 ZIP archive and retry.",
            );
        }
    };
    let project_bytes = match serialize_project_document_v6(&request.project) {
        Ok(bytes) => bytes,
        Err(error) => return writer_error(error.into()),
    };
    let mut checksums = BTreeMap::from([(
        "project.json".to_owned(),
        format!("{:x}", Sha256::digest(&project_bytes)),
    )]);
    for descriptor in &request.project.datasets {
        let entry_name = format!("data/{}.arrow", descriptor.id);
        let Some(digest) = loaded.manifest.checksums.get(&entry_name) else {
            return blocked(
                "schema6_save_copy.non_model_change_rejected",
                format!("The strict source does not bind {entry_name}."),
                "Keep the detached dataset authority equal to the strict source snapshot.",
            );
        };
        checksums.insert(entry_name, digest.clone());
    }
    let manifest = ProjectManifest {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: request.project.project_id,
        name: request.project.name.clone(),
        created_at: request.project.created_at,
        modified_at: request.project.modified_at,
        engine_version: qpls_core::ENGINE_VERSION.to_owned(),
        checksum_algorithm: "sha256".to_owned(),
        checksums,
    };
    let resident_datasets = loaded
        .datasets
        .iter()
        .map(|dataset| ProjectArchiveV6SaveCopyResidentDatasetV1 {
            dataset_id: dataset.id.to_string(),
            name: dataset.name.clone(),
            fingerprint: dataset.fingerprint.0.clone(),
            row_count: dataset.batch.num_rows(),
            column_count: dataset.batch.num_columns(),
            sample_size: dataset.schema.sample_size,
            arrow_resident: true,
        })
        .collect::<Vec<_>>();
    let counts = ProjectArchiveV6SaveCopyCountsV1 {
        datasets: request.project.datasets.len(),
        models: request.project.models.len(),
        recipes: request.project.recipes.len(),
        historical_recipes: request.project.historical_recipes.len(),
        historical_results: request.project.historical_results.len(),
        canonical_result_documents: request.project.canonical_result_documents.len(),
    };

    // Release the exclusively pinned preview identity before the core writer
    // independently pins and rechecks the same expected source digest.
    drop(preview_file);

    let receipt = match save_project_archive_v6_model_copy(
        source,
        &request.expected_source_archive_sha256,
        destination,
        &request.project,
    ) {
        Ok(receipt) => receipt,
        Err(error) => return writer_error(error),
    };

    // The core writer has committed at this point. Everything below only moves
    // already-validated values into the serializable success payload; there is
    // no further filesystem operation, validation, or cancellation point.
    let snapshot = ProjectArchiveV6SaveCopySnapshotV1 {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        access: ProjectArchiveV6SaveCopyAccessV1::ReadOnly,
        loader: ProjectArchiveV6SaveCopyLoaderV1::StrictSchema6Zip,
        archive_path: receipt.destination_archive_path.clone(),
        archive_sha256: receipt.destination_archive_sha256.clone(),
        archive_bytes: receipt.destination_archive_bytes,
        manifest,
        project: request.project,
        resident_datasets,
        counts,
        source_rechecked_unchanged: receipt.source_verified_unchanged,
    };
    ProjectArchiveV6SaveCopyOutcomeV1::Ok {
        value: Box::new(ProjectArchiveV6SaveCopyResultV1 {
            schema_version: SAVE_COPY_RESULT_SCHEMA_VERSION,
            persistence: ProjectArchiveV6SaveCopyPersistenceV1::PersistedNewCopy,
            receipt,
            snapshot,
        }),
    }
}

#[tauri::command]
pub(crate) async fn save_internal_project_archive_v6_copy(
    request: ProjectArchiveV6SaveCopyRequestV1,
) -> ProjectArchiveV6SaveCopyOutcomeV1 {
    match tauri::async_runtime::spawn_blocking(move || save_copy(request)).await {
        Ok(outcome) => outcome,
        Err(error) => blocked(
            "schema6_save_copy.worker_failed",
            error.to_string(),
            "Keep the current Labs session open and retry with a new local destination.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multimod_candidate_authority_v1::with_typed_qualification_test_authority_v1;
    use std::path::PathBuf;

    #[test]
    fn collision_maps_to_a_typed_no_clobber_diagnostic() {
        let outcome = writer_error(ProjectArchiveV6SaveCopyError::DestinationExists(
            PathBuf::from(r"D:\projects\existing.qpls"),
        ));
        assert!(matches!(
            outcome,
            ProjectArchiveV6SaveCopyOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_save_copy.destination_exists"
        ));
    }

    #[test]
    fn standard_save_copy_requires_only_embedded_release_authority() {
        let mixed_surface = validate_save_copy_surface_using_v1(
            STANDARD_MULTIMOD_SURFACE_V1,
            true,
            || -> Result<bool, String> {
                panic!("an invalid Standard/Labs pair must not consult authority")
            },
        )
        .unwrap_err();
        assert_eq!(
            mixed_surface,
            ProjectArchiveV6SaveCopySurfaceErrorV1::SurfaceInvalid
        );

        let labs_disabled = validate_save_copy_surface_using_v1(
            INTERNAL_LABS_SURFACE,
            false,
            || -> Result<bool, String> { panic!("Labs denial must not consult authority") },
        )
        .unwrap_err();
        assert_eq!(
            labs_disabled,
            ProjectArchiveV6SaveCopySurfaceErrorV1::InternalLabsRequired
        );

        let denied =
            validate_save_copy_surface_using_v1(STANDARD_MULTIMOD_SURFACE_V1, false, || Ok(false))
                .unwrap_err();
        assert_eq!(
            denied,
            ProjectArchiveV6SaveCopySurfaceErrorV1::StandardAuthorityRequired
        );

        with_typed_qualification_test_authority_v1(
            &["conditional.multi_two_way_percentile.v2::explicit_path_target_math"],
            |_| {
                validate_save_copy_surface_v1(STANDARD_MULTIMOD_SURFACE_V1, false).unwrap();
            },
        )
        .unwrap();

        validate_save_copy_surface_using_v1(INTERNAL_LABS_SURFACE, true, || {
            panic!("historical Labs access must not consult Standard authority")
        })
        .unwrap();
    }
}
