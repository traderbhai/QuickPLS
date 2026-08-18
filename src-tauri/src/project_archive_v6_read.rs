//! Internal/Labs-only, read-only bridge for strict schema-6 ZIP archives.
//!
//! This command deliberately calls the dedicated schema-6 ZIP reader and never
//! projects the archive into the live schema-5 `Project` service.

use qpls_project::{
    LoadedProjectArchiveV6, ProjectArchiveDocumentV6, ProjectManifest, load_project_archive_v6,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read},
    path::Path,
};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const ARCHIVE_READ_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ReadRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6ReadAccessV1 {
    ReadOnly,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6LoaderV1 {
    StrictSchema6Zip,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectArchiveV6ResidentDatasetV1 {
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
struct ProjectArchiveV6CountsV1 {
    datasets: usize,
    models: usize,
    recipes: usize,
    historical_recipes: usize,
    historical_results: usize,
    canonical_result_documents: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ReadSnapshotV1 {
    schema_version: u32,
    access: ProjectArchiveV6ReadAccessV1,
    loader: ProjectArchiveV6LoaderV1,
    archive_path: String,
    archive_sha256: String,
    archive_bytes: u64,
    manifest: ProjectManifest,
    project: ProjectArchiveDocumentV6,
    resident_datasets: Vec<ProjectArchiveV6ResidentDatasetV1>,
    counts: ProjectArchiveV6CountsV1,
    source_rechecked_unchanged: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ReadDiagnosticV1 {
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
pub(crate) enum ProjectArchiveV6ReadOutcomeV1 {
    Ok {
        value: Box<ProjectArchiveV6ReadSnapshotV1>,
    },
    Blocked {
        diagnostic: ProjectArchiveV6ReadDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectArchiveV6ReadOutcomeV1 {
    ProjectArchiveV6ReadOutcomeV1::Blocked {
        diagnostic: ProjectArchiveV6ReadDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn sha256_file(path: &Path) -> io::Result<(u64, String)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        bytes += read as u64;
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

fn inspect_archive_v6(request: &ProjectArchiveV6ReadRequestV1) -> ProjectArchiveV6ReadOutcomeV1 {
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return blocked(
            "schema6_archive_read.internal_labs_required",
            "Schema-6 ZIP inspection is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and use the internal read-only schema-6 archive service.",
        );
    }

    let archive_path_text = request.archive_path.trim();
    let archive_path = Path::new(archive_path_text);
    if archive_path_text.is_empty() || !archive_path.is_absolute() {
        return blocked(
            "schema6_archive_read.absolute_path_required",
            "The schema-6 ZIP archive path must be an absolute local path.",
            "Select the exact local schema-6 .qpls archive and retry.",
        );
    }

    match fs::symlink_metadata(archive_path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return blocked(
                "schema6_archive_read.regular_file_required",
                "The schema-6 ZIP source must be a regular local file and cannot be a symbolic link.",
                "Select the exact regular schema-6 .qpls archive.",
            );
        }
        Err(error) => {
            return blocked(
                "schema6_archive_read.source_unavailable",
                error.to_string(),
                "Verify that the schema-6 archive exists and is readable, then retry.",
            );
        }
    }

    let (archive_bytes, before_sha256) = match sha256_file(archive_path) {
        Ok(identity) => identity,
        Err(error) => {
            return blocked(
                "schema6_archive_read.source_unavailable",
                error.to_string(),
                "Verify that the schema-6 archive is readable, then retry.",
            );
        }
    };

    let loaded = match load_project_archive_v6(archive_path) {
        Ok(loaded) => loaded,
        Err(error) => {
            return blocked(
                "schema6_archive_read.invalid_archive",
                error.to_string(),
                "Restore or recreate the schema-6 ZIP from a trusted source before inspecting it.",
            );
        }
    };

    let (rechecked_bytes, after_sha256) = match sha256_file(archive_path) {
        Ok(identity) => identity,
        Err(error) => {
            return blocked(
                "schema6_archive_read.source_unavailable",
                error.to_string(),
                "Retry after other local file operations finish.",
            );
        }
    };
    if rechecked_bytes != archive_bytes || after_sha256 != before_sha256 {
        return blocked(
            "schema6_archive_read.source_changed_during_read",
            "The schema-6 ZIP archive changed while it was being validated.",
            "Retry after all writers have finished and the archive is stable.",
        );
    }

    let LoadedProjectArchiveV6 {
        manifest,
        document,
        datasets,
    } = loaded;
    let resident_datasets = datasets
        .iter()
        .map(|dataset| ProjectArchiveV6ResidentDatasetV1 {
            dataset_id: dataset.id.to_string(),
            name: dataset.name.clone(),
            fingerprint: dataset.fingerprint.0.clone(),
            row_count: dataset.batch.num_rows(),
            column_count: dataset.batch.num_columns(),
            sample_size: dataset.schema.sample_size,
            arrow_resident: true,
        })
        .collect::<Vec<_>>();
    let counts = ProjectArchiveV6CountsV1 {
        datasets: document.datasets.len(),
        models: document.models.len(),
        recipes: document.recipes.len(),
        historical_recipes: document.historical_recipes.len(),
        historical_results: document.historical_results.len(),
        canonical_result_documents: document.canonical_result_documents.len(),
    };

    ProjectArchiveV6ReadOutcomeV1::Ok {
        value: Box::new(ProjectArchiveV6ReadSnapshotV1 {
            schema_version: ARCHIVE_READ_SNAPSHOT_SCHEMA_VERSION,
            access: ProjectArchiveV6ReadAccessV1::ReadOnly,
            loader: ProjectArchiveV6LoaderV1::StrictSchema6Zip,
            archive_path: archive_path.to_string_lossy().into_owned(),
            archive_sha256: before_sha256,
            archive_bytes,
            manifest,
            project: document,
            resident_datasets,
            counts,
            source_rechecked_unchanged: true,
        }),
    }
}

#[tauri::command]
pub(crate) fn inspect_internal_project_archive_v6_zip(
    request: ProjectArchiveV6ReadRequestV1,
) -> ProjectArchiveV6ReadOutcomeV1 {
    inspect_archive_v6(&request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_project::{
        PROJECT_ARCHIVE_SCHEMA_V6_VERSION, Project, ProjectArchiveUpgradeRequestV6,
        serialize_project_document_v6,
    };
    use std::{collections::BTreeMap, io::Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn request(path: &Path) -> ProjectArchiveV6ReadRequestV1 {
        ProjectArchiveV6ReadRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            archive_path: path.to_string_lossy().into_owned(),
        }
    }

    fn write_empty_schema6_archive(path: &Path) {
        let project = Project::new("Read-only schema-6 fixture");
        let plan = qpls_project::plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: path
                    .with_file_name("legacy-source.qpls")
                    .to_string_lossy()
                    .into_owned(),
                destination_archive_path: path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 10, 0, 0).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let project_bytes = serialize_project_document_v6(&plan.document).unwrap();
        let checksums = BTreeMap::from([(
            "project.json".to_owned(),
            format!("{:x}", Sha256::digest(&project_bytes)),
        )]);
        let manifest = ProjectManifest {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: plan.document.project_id,
            name: plan.document.name.clone(),
            created_at: plan.document.created_at,
            modified_at: plan.document.modified_at,
            engine_version: qpls_core::ENGINE_VERSION.into(),
            checksum_algorithm: "sha256".into(),
            checksums,
        };

        let mut writer = ZipWriter::new(File::create(path).unwrap());
        writer
            .start_file("project.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&project_bytes).unwrap();
        writer
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn strict_zip_inspection_returns_only_a_read_only_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("project-v6.qpls");
        write_empty_schema6_archive(&archive_path);

        let outcome = inspect_archive_v6(&request(&archive_path));
        let ProjectArchiveV6ReadOutcomeV1::Ok { value } = outcome else {
            panic!("strict schema-6 ZIP was not inspectable");
        };
        assert!(matches!(
            value.access,
            ProjectArchiveV6ReadAccessV1::ReadOnly
        ));
        assert!(matches!(
            value.loader,
            ProjectArchiveV6LoaderV1::StrictSchema6Zip
        ));
        assert_eq!(value.manifest.schema_version, 6);
        assert_eq!(value.project.schema_version, 6);
        assert_eq!(value.counts.datasets, 0);
        assert!(value.resident_datasets.is_empty());
        assert!(value.source_rechecked_unchanged);
    }

    #[test]
    fn standard_surface_and_non_zip_content_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("not-a-zip.qpls");
        fs::write(&path, br#"{"schema_version":6}"#).unwrap();

        let mut denied = request(&path);
        denied.surface = "standard".into();
        assert!(matches!(
            inspect_archive_v6(&denied),
            ProjectArchiveV6ReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_archive_read.internal_labs_required"
        ));

        assert!(matches!(
            inspect_archive_v6(&request(&path)),
            ProjectArchiveV6ReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_archive_read.invalid_archive"
        ));
    }
}
