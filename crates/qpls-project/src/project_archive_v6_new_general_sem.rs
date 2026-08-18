//! Safe creation of a blank schema-6 General SEM project archive.
//!
//! This path is deliberately separate from the historical `Project::new`
//! schema-5 workflow. Publication delegates to the schema-6 save-copy module's
//! pinned-parent, handle-owned temporary, strict-reopen, and no-replace commit
//! seam; no archive bytes are published before validation succeeds.

use super::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectArchiveV6SaveCopyError,
    project_archive_v6_save_copy::{
        ProjectArchiveV6NewDocumentPublication, publish_new_project_archive_v6_document,
    },
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

pub const GENERAL_SEM_PROJECT_ARCHIVE_CREATION_V1_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSemProjectArchiveCreationReceiptV1 {
    pub schema_version: u32,
    pub archive_schema_version: u32,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub destination_archive_path: String,
    pub destination_archive_sha256: String,
    pub destination_archive_bytes: u64,
    pub strict_reopen_validated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GeneralSemProjectArchiveCreationErrorV1 {
    #[error(transparent)]
    Publication(#[from] ProjectArchiveV6SaveCopyError),
}

/// Creates and atomically publishes a blank schema-6 General SEM archive.
///
/// `destination` must be an absolute, local `.qpls` path whose final component
/// does not exist. The caller supplies all project identity and timestamp
/// authority; no random identity or current-clock value is invented here.
pub fn create_general_sem_project_archive_v6(
    destination: &Path,
    project_id: Uuid,
    name: impl Into<String>,
    created_at: DateTime<Utc>,
) -> Result<GeneralSemProjectArchiveCreationReceiptV1, GeneralSemProjectArchiveCreationErrorV1> {
    let name = name.into();
    let document =
        ProjectArchiveDocumentV6::new_general_sem_v1(project_id, name.clone(), created_at);
    let ProjectArchiveV6NewDocumentPublication {
        destination_archive_sha256,
        destination_archive_bytes,
        strict_reopen_validated,
    } = publish_new_project_archive_v6_document(destination, &document)?;

    // Publication is the final fallible operation. Constructing this receipt is
    // infallible, so a successful no-replace commit can never be reported as an
    // error after the archive has become visible.
    Ok(GeneralSemProjectArchiveCreationReceiptV1 {
        schema_version: GENERAL_SEM_PROJECT_ARCHIVE_CREATION_V1_SCHEMA_VERSION,
        archive_schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id,
        name,
        created_at,
        destination_archive_path: destination.to_string_lossy().into_owned(),
        destination_archive_sha256,
        destination_archive_bytes,
        strict_reopen_validated,
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::{
        ProjectOriginV6, ProjectSemGenerationV6, load_project_archive_v6,
        project_archive_v6_save_copy::publish_new_project_archive_v6_document_with_hooks,
    };
    use chrono::TimeZone;
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io::Read,
        path::{Path, PathBuf},
    };
    use zip::ZipArchive;

    fn sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn zip_entry(path: &Path, name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entry = archive.by_name(name).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        bytes
    }

    fn temporary_links(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".schema6-save-") && name.ends_with(".tmp"))
            })
            .collect()
    }

    #[test]
    fn creates_deterministic_empty_general_sem_archive_and_strictly_reopens_it() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("general-sem-first.qpls");
        let second = directory.path().join("general-sem-second.qpls");
        let project_id = Uuid::from_u128(0x6000_0001);
        let created_at = Utc.with_ymd_and_hms(2026, 8, 18, 9, 30, 0).unwrap();

        let first_receipt = create_general_sem_project_archive_v6(
            &first,
            project_id,
            "General SEM project",
            created_at,
        )
        .unwrap();
        let second_receipt = create_general_sem_project_archive_v6(
            &second,
            project_id,
            "General SEM project",
            created_at,
        )
        .unwrap();

        assert_eq!(
            first_receipt.schema_version,
            GENERAL_SEM_PROJECT_ARCHIVE_CREATION_V1_SCHEMA_VERSION
        );
        assert_eq!(
            first_receipt.archive_schema_version,
            PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        );
        assert_eq!(first_receipt.project_id, project_id);
        assert_eq!(first_receipt.name, "General SEM project");
        assert_eq!(first_receipt.created_at, created_at);
        assert_eq!(
            first_receipt.destination_archive_path,
            first.to_string_lossy().into_owned()
        );
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        assert_eq!(
            first_receipt.destination_archive_sha256,
            second_receipt.destination_archive_sha256
        );
        assert_eq!(
            first_receipt.destination_archive_sha256,
            sha256(&fs::read(&first).unwrap())
        );
        assert_eq!(
            first_receipt.destination_archive_bytes,
            fs::metadata(&first).unwrap().len()
        );
        assert!(first_receipt.strict_reopen_validated);

        let reopened = load_project_archive_v6(&first).unwrap();
        assert_eq!(
            reopened.manifest.schema_version,
            PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        );
        assert_eq!(reopened.manifest.project_id, project_id);
        assert_eq!(reopened.manifest.name, "General SEM project");
        assert_eq!(reopened.manifest.created_at, created_at);
        assert_eq!(reopened.manifest.modified_at, created_at);
        assert_eq!(reopened.document.project_id, project_id);
        assert_eq!(reopened.document.created_at, created_at);
        assert_eq!(reopened.document.modified_at, created_at);
        assert!(matches!(
            reopened.document.origin,
            ProjectOriginV6::NewProject
        ));
        assert_eq!(
            reopened.document.sem_generation,
            Some(ProjectSemGenerationV6::GeneralSemV1)
        );
        assert!(reopened.document.supports_general_sem_v1());
        assert!(reopened.document.upgrade_lineage().is_none());
        assert!(reopened.document.datasets.is_empty());
        assert!(reopened.document.models.is_empty());
        assert!(reopened.document.recipes.is_empty());
        assert!(reopened.document.historical_recipes.is_empty());
        assert!(reopened.document.layouts.is_empty());
        assert!(reopened.document.historical_results.is_empty());
        assert!(reopened.document.canonical_result_documents.is_empty());
        assert!(reopened.datasets.is_empty());

        let project_bytes = zip_entry(&first, "project.json");
        let project_sha256 = sha256(&project_bytes);
        assert_eq!(reopened.manifest.checksum_algorithm, "sha256");
        assert_eq!(reopened.manifest.checksums.len(), 1);
        assert_eq!(
            reopened.manifest.checksums.get("project.json"),
            Some(&project_sha256)
        );
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[test]
    fn refuses_overwrite_without_changing_existing_bytes_or_leaving_a_temporary_link() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("already-exists.qpls");
        let sentinel = b"existing project bytes";
        fs::write(&destination, sentinel).unwrap();

        let result = create_general_sem_project_archive_v6(
            &destination,
            Uuid::from_u128(0x6000_0002),
            "Must not replace",
            Utc.with_ymd_and_hms(2026, 8, 18, 10, 0, 0).unwrap(),
        );

        assert!(matches!(
            result,
            Err(GeneralSemProjectArchiveCreationErrorV1::Publication(
                ProjectArchiveV6SaveCopyError::DestinationExists(path)
            )) if path == destination
        ));
        assert_eq!(fs::read(&destination).unwrap(), sentinel);
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[test]
    fn strict_reopen_failure_never_publishes_a_partial_archive() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("strict-reopen-failure.qpls");
        let document = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x6000_0003),
            "Injected failure",
            Utc.with_ymd_and_hms(2026, 8, 18, 10, 30, 0).unwrap(),
        );

        let result = publish_new_project_archive_v6_document_with_hooks(
            &destination,
            &document,
            |_| Ok(()),
            |file| {
                file.set_len(0)?;
                file.sync_all()?;
                Ok(())
            },
        );

        assert!(result.is_err());
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
    }
}
