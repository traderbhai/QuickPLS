//! Internal/Labs-only, read-only bridge for strict schema-6 ZIP archives.
//!
//! This command deliberately calls the dedicated schema-6 ZIP reader and never
//! projects the archive into the live schema-5 `Project` service.

use qpls_core::{AnalysisRecipeModelBindingV4, AnalysisRecipeV4, sha256_serialized};
use qpls_data::preview_page;
use qpls_project::{
    LoadedProjectArchiveV6, ProjectArchiveDocumentV6, ProjectManifest, ProjectModelPayloadV6,
    load_project_archive_v6,
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
const ARCHIVE_DATASET_ROWS_SCHEMA_VERSION: u32 = 1;
const MAX_ARCHIVE_DATASET_ROW_PAGE_SIZE: usize = 500;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ReadRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6DatasetRowsRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
    expected_archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    offset: usize,
    limit: usize,
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

/// Exact runnable authority for the bounded General SEM project slice.
///
/// The full resident RecipeV4 is repeated deliberately: clients can restore
/// every scientific setting after a process restart, while the native digest
/// prevents them from inventing a document identity with a JavaScript hash.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectArchiveV6GeneralSemExecutionAuthorityV1 {
    schema_version: u32,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    recipe_id: String,
    recipe_document_sha256: String,
    recipe: AnalysisRecipeV4,
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
    general_sem_execution_authority: Option<ProjectArchiveV6GeneralSemExecutionAuthorityV1>,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6DatasetRowsPageV1 {
    schema_version: u32,
    archive_path: String,
    archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    offset: usize,
    limit: usize,
    row_count: usize,
    columns: Vec<String>,
    rows: Vec<std::collections::BTreeMap<String, Option<String>>>,
    source_rechecked_unchanged: bool,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectArchiveV6DatasetRowsOutcomeV1 {
    Ok {
        value: Box<ProjectArchiveV6DatasetRowsPageV1>,
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

fn dataset_rows_blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectArchiveV6DatasetRowsOutcomeV1 {
    ProjectArchiveV6DatasetRowsOutcomeV1::Blocked {
        diagnostic: ProjectArchiveV6ReadDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

fn general_sem_execution_authority(
    document: &ProjectArchiveDocumentV6,
) -> Result<Option<ProjectArchiveV6GeneralSemExecutionAuthorityV1>, String> {
    if !document.supports_general_sem_v1() {
        return Ok(None);
    }
    if document.datasets.is_empty() && document.models.is_empty() && document.recipes.is_empty() {
        return Ok(None);
    }
    let [dataset] = document.datasets.as_slice() else {
        return Err("the bounded general_sem_v1 execution archive must contain exactly one resident dataset".into());
    };
    let [model_record] = document.models.as_slice() else {
        return Err(
            "the bounded general_sem_v1 execution archive must contain exactly one model authority"
                .into(),
        );
    };
    let ProjectModelPayloadV6::SemModelV4 {
        scientific_sha256, ..
    } = &model_record.payload
    else {
        return Err(
            "the bounded general_sem_v1 execution model must be a promoted SemModelV4 authority"
                .into(),
        );
    };
    let mut authority_recipes = document
        .recipes
        .iter()
        // A completed MultiMod run is appended as an additive Recipe V4 that
        // retains the source General SEM configuration for exact compiler
        // provenance. It is a result recipe, not a second base execution
        // authority. Keep strict General SEM reopen anchored to the sole
        // sidecar-free/base recipe instead of rejecting a valid V6 archive
        // merely because it now contains completed MultiMod analyses.
        .filter(|recipe| {
            recipe.general_sem_config.is_some()
                && recipe.mga_multigroup.is_none()
                && recipe.pls_heterogeneity.is_none()
                && recipe.general_sem_conditional_process.is_none()
                && recipe.interventional_causal_mediation.is_none()
        });
    let Some(recipe) = authority_recipes.next() else {
        return Err("the bounded general_sem_v1 execution archive must contain exactly one resident GeneralSemConfigV1 RecipeV4 authority".into());
    };
    if authority_recipes.next().is_some() {
        return Err("the bounded general_sem_v1 execution archive must contain exactly one resident GeneralSemConfigV1 RecipeV4 authority".into());
    }
    let AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id,
        scientific_sha256: recipe_model_sha256,
    } = &recipe.model_binding
    else {
        return Err(
            "the bounded general_sem_v1 RecipeV4 must reference the resident model authority"
                .into(),
        );
    };
    if model_id != &model_record.model_id || recipe_model_sha256 != scientific_sha256 {
        return Err(
            "the bounded general_sem_v1 recipe/model authority binding is inconsistent".into(),
        );
    }
    if recipe.dataset_fingerprint != dataset.fingerprint.0 || recipe.general_sem_config.is_none() {
        return Err("the bounded general_sem_v1 recipe must bind the resident dataset and GeneralSemConfigV1".into());
    }
    Ok(Some(ProjectArchiveV6GeneralSemExecutionAuthorityV1 {
        schema_version: 1,
        project_id: document.project_id.to_string(),
        dataset_id: dataset.id.to_string(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model_id: model_record.model_id.clone(),
        model_scientific_sha256: scientific_sha256.clone(),
        recipe_id: recipe.id.to_string(),
        recipe_document_sha256: sha256_serialized(recipe),
        recipe: recipe.clone(),
    }))
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
        multimod_sidecars: _,
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
    let general_sem_execution_authority = match general_sem_execution_authority(&document) {
        Ok(authority) => authority,
        Err(error) => {
            return blocked(
                "schema6_archive_read.general_sem_authority_invalid",
                error,
                "Preserve the archive unchanged and reopen a strictly validated QuickPLS General SEM project.",
            );
        }
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
            general_sem_execution_authority,
            source_rechecked_unchanged: true,
        }),
    }
}

fn read_archive_dataset_rows_v1(
    request: &ProjectArchiveV6DatasetRowsRequestV1,
) -> ProjectArchiveV6DatasetRowsOutcomeV1 {
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return dataset_rows_blocked(
            "schema6_dataset_rows.internal_labs_required",
            "Schema-6 dataset paging is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and reopen the exact General SEM archive.",
        );
    }
    if request.limit == 0 || request.limit > MAX_ARCHIVE_DATASET_ROW_PAGE_SIZE {
        return dataset_rows_blocked(
            "schema6_dataset_rows.page_bounds_invalid",
            format!(
                "Dataset row page limit must be from 1 through {MAX_ARCHIVE_DATASET_ROW_PAGE_SIZE}."
            ),
            "Request a bounded page and retry.",
        );
    }
    if !lowercase_sha256(&request.expected_archive_sha256) {
        return dataset_rows_blocked(
            "schema6_dataset_rows.archive_sha256_invalid",
            "The expected archive digest must be a lowercase SHA-256 value.",
            "Reinspect the archive and retry from its exact strict read receipt.",
        );
    }

    let archive_path_text = request.archive_path.trim();
    let archive_path = Path::new(archive_path_text);
    if archive_path_text.is_empty() || !archive_path.is_absolute() {
        return dataset_rows_blocked(
            "schema6_dataset_rows.absolute_path_required",
            "The schema-6 ZIP archive path must be an absolute local path.",
            "Reopen the exact local General SEM archive.",
        );
    }
    match fs::symlink_metadata(archive_path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.regular_file_required",
                "The schema-6 ZIP source must be a regular local file and cannot be a symbolic link.",
                "Select the exact regular General SEM archive.",
            );
        }
        Err(error) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.source_unavailable",
                error.to_string(),
                "Verify that the archive exists and is readable, then reopen it.",
            );
        }
    }

    let (archive_bytes, before_sha256) = match sha256_file(archive_path) {
        Ok(identity) => identity,
        Err(error) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.source_unavailable",
                error.to_string(),
                "Verify that the archive is readable, then retry.",
            );
        }
    };
    if before_sha256 != request.expected_archive_sha256 {
        return dataset_rows_blocked(
            "schema6_dataset_rows.archive_identity_mismatch",
            "The General SEM archive no longer matches the active strict read receipt.",
            "Reopen and strictly validate the changed archive before reading data.",
        );
    }

    let loaded = match load_project_archive_v6(archive_path) {
        Ok(loaded) => loaded,
        Err(error) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.invalid_archive",
                error.to_string(),
                "Restore or recreate the schema-6 archive from a trusted source.",
            );
        }
    };
    if loaded.manifest.project_id.to_string() != request.project_id
        || loaded.document.project_id.to_string() != request.project_id
        || !loaded.document.supports_general_sem_v1()
    {
        return dataset_rows_blocked(
            "schema6_dataset_rows.project_identity_mismatch",
            "The archive is not the marked General SEM project bound to this request.",
            "Close the stale session and reopen the intended General SEM project.",
        );
    }
    let authority = match general_sem_execution_authority(&loaded.document) {
        Ok(Some(authority)) => authority,
        Ok(None) | Err(_) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.execution_authority_invalid",
                "The marked archive has no complete, internally consistent General SEM execution authority.",
                "Preserve the archive and reopen a validated populated General SEM project.",
            );
        }
    };
    if authority.project_id != request.project_id
        || authority.dataset_id != request.dataset_id
        || authority.dataset_fingerprint != request.dataset_fingerprint
    {
        return dataset_rows_blocked(
            "schema6_dataset_rows.execution_authority_mismatch",
            "The requested dataset is not bound by the resident General SEM execution authority.",
            "Use the dataset identified by the current strict execution receipt.",
        );
    }
    let Some(dataset) = loaded
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == request.dataset_id)
    else {
        return dataset_rows_blocked(
            "schema6_dataset_rows.dataset_not_resident",
            "The requested dataset is not resident in the validated archive.",
            "Reopen a complete General SEM archive containing its bound Arrow dataset.",
        );
    };
    if dataset.fingerprint.0 != request.dataset_fingerprint {
        return dataset_rows_blocked(
            "schema6_dataset_rows.dataset_fingerprint_mismatch",
            "The resident dataset fingerprint differs from the active execution authority.",
            "Preserve the archive and reopen a strictly validated copy.",
        );
    }

    let offset = request.offset.min(dataset.schema.case_count);
    let rows = preview_page(dataset, offset, request.limit);
    let columns = dataset
        .schema
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();

    let (rechecked_bytes, after_sha256) = match sha256_file(archive_path) {
        Ok(identity) => identity,
        Err(error) => {
            return dataset_rows_blocked(
                "schema6_dataset_rows.source_unavailable",
                error.to_string(),
                "Retry after other local file operations finish.",
            );
        }
    };
    if rechecked_bytes != archive_bytes
        || after_sha256 != before_sha256
        || after_sha256 != request.expected_archive_sha256
    {
        return dataset_rows_blocked(
            "schema6_dataset_rows.source_changed_during_read",
            "The General SEM archive changed while its data page was being read.",
            "Retry only after all archive writers have finished and reopen the changed archive.",
        );
    }

    ProjectArchiveV6DatasetRowsOutcomeV1::Ok {
        value: Box::new(ProjectArchiveV6DatasetRowsPageV1 {
            schema_version: ARCHIVE_DATASET_ROWS_SCHEMA_VERSION,
            archive_path: archive_path.to_string_lossy().into_owned(),
            archive_sha256: after_sha256,
            project_id: request.project_id.clone(),
            dataset_id: request.dataset_id.clone(),
            dataset_fingerprint: request.dataset_fingerprint.clone(),
            offset,
            limit: request.limit,
            row_count: dataset.schema.case_count,
            columns,
            rows,
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

#[tauri::command]
pub(crate) fn read_internal_project_archive_v6_dataset_rows(
    request: ProjectArchiveV6DatasetRowsRequestV1,
) -> ProjectArchiveV6DatasetRowsOutcomeV1 {
    read_archive_dataset_rows_v1(&request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{GeneralSemConfigV1, MethodConfig, SemDataBindingV4};
    use qpls_data::{Dataset, write_arrow};
    use qpls_project::{
        PROJECT_ARCHIVE_SCHEMA_V6_VERSION, Project, ProjectArchiveUpgradeRequestV6,
        append_recipe_v4_and_canonical_result_document_v2_file_v6,
        create_populated_general_sem_project_archive_v6, serialize_project_document_v6,
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

    fn write_blank_general_sem_archive(path: &Path) {
        let created_at = Utc.with_ymd_and_hms(2026, 8, 19, 10, 0, 0).unwrap();
        let document = ProjectArchiveDocumentV6::new_general_sem_v1(
            uuid::Uuid::from_u128(0x6000_0099),
            "Blank General SEM fixture",
            created_at,
        );
        let project_bytes = serialize_project_document_v6(&document).unwrap();
        let checksums = BTreeMap::from([(
            "project.json".to_owned(),
            format!("{:x}", Sha256::digest(&project_bytes)),
        )]);
        let manifest = ProjectManifest {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: document.project_id,
            name: document.name.clone(),
            created_at: document.created_at,
            modified_at: document.modified_at,
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

    fn write_schema6_zip(path: &Path, document: &ProjectArchiveDocumentV6, datasets: &[Dataset]) {
        let mut entries = vec![(
            "project.json".to_owned(),
            serialize_project_document_v6(document).unwrap(),
        )];
        for dataset in datasets {
            entries.push((
                format!("data/{}.arrow", dataset.id),
                write_arrow(&dataset.batch).unwrap(),
            ));
        }
        let checksums = entries
            .iter()
            .map(|(name, bytes)| {
                (
                    name.clone(),
                    format!("{:x}", Sha256::digest(bytes.as_slice())),
                )
            })
            .collect();
        let manifest = ProjectManifest {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: document.project_id,
            name: document.name.clone(),
            created_at: document.created_at,
            modified_at: document.modified_at,
            engine_version: qpls_core::ENGINE_VERSION.into(),
            checksum_algorithm: "sha256".into(),
            checksums,
        };
        entries.push((
            "manifest.json".to_owned(),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        ));

        let mut writer = ZipWriter::new(File::create(path).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn create_general_sem_authority_for_exact_cbsem(
        archive_path: &Path,
    ) -> (AnalysisRecipeV4, qpls_project::CanonicalResultDocumentV2) {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/fixtures/v255/archives/cbsem-exact-bootstrap-result.qpls");
        let loaded = load_project_archive_v6(&fixture).unwrap();
        let project_id = loaded.document.project_id;
        let exact_recipe = loaded.document.recipes[0].clone();
        assert!(exact_recipe.general_sem_config.is_none());
        let exact_document = loaded.document.canonical_result_documents[0]
            .canonical_document()
            .clone();
        let ProjectModelPayloadV6::SemModelV4 { model, .. } = &loaded.document.models[0].payload
        else {
            panic!("exact CB-SEM fixture must carry a promoted SemModelV4")
        };
        let mut authority_model = model.clone();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut authority_model.data_binding else {
            panic!("exact CB-SEM fixture must carry a raw-data SemModelV4")
        };
        *dataset_id = loaded.datasets[0].id.to_string();
        let authority_model_sha256 = authority_model.scientific_sha256().unwrap();

        let mut authority_recipe = exact_recipe.clone();
        authority_recipe.id =
            uuid::Uuid::parse_str("00000000-0000-0000-0000-00000000cba0").unwrap();
        authority_recipe.settings.bootstrap_samples = 0;
        let Some(MethodConfig::Cbsem {
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = authority_recipe.method_config.as_mut()
        else {
            panic!("exact CB-SEM fixture must carry typed CB-SEM settings")
        };
        *bootstrap_samples = 0;
        *bootstrap_v2 = None;
        authority_recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: authority_model.id.clone(),
            scientific_sha256: authority_model_sha256,
        };
        authority_recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        authority_recipe.metadata.insert(
            "execution_surface".into(),
            "native_general_sem_cbsem_standard_v1".into(),
        );
        authority_recipe
            .metadata
            .insert("general_sem_generation".into(), "general_sem_v1".into());

        create_populated_general_sem_project_archive_v6(
            archive_path,
            project_id,
            "General SEM exact CB-SEM append fixture",
            Utc.with_ymd_and_hms(2026, 8, 23, 12, 0, 0).unwrap(),
            &loaded.datasets[0],
            authority_model,
            authority_recipe,
        )
        .unwrap();
        (exact_recipe, exact_document)
    }

    fn populated_rows_request(
        directory: &tempfile::TempDir,
    ) -> ProjectArchiveV6DatasetRowsRequestV1 {
        let fixture =
            crate::project_archive_v6_general_sem_bootstrap::tests::general_sem_native_fixture_v1();
        let archive_path = directory.path().join("general-sem-rows.qpls");
        let receipt = create_populated_general_sem_project_archive_v6(
            &archive_path,
            uuid::Uuid::from_u128(0x6000_00aa),
            "General SEM row fixture",
            Utc.with_ymd_and_hms(2026, 8, 19, 11, 0, 0).unwrap(),
            &fixture.dataset,
            fixture.model,
            fixture.recipe,
        )
        .unwrap();
        ProjectArchiveV6DatasetRowsRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            archive_path: receipt.destination_archive_path,
            expected_archive_sha256: receipt.destination_archive_sha256,
            project_id: receipt.project_id.to_string(),
            dataset_id: receipt.resident_dataset_id.to_string(),
            dataset_fingerprint: receipt.resident_dataset_fingerprint,
            offset: 1,
            limit: 2,
        }
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
    fn blank_marked_project_remains_inspectable_without_execution_authority() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("blank-general-sem.qpls");
        write_blank_general_sem_archive(&archive_path);

        let outcome = inspect_archive_v6(&request(&archive_path));
        let ProjectArchiveV6ReadOutcomeV1::Ok { value } = outcome else {
            panic!("blank marked project was not inspectable");
        };
        assert!(value.project.supports_general_sem_v1());
        assert!(value.general_sem_execution_authority.is_none());
    }

    #[test]
    fn strict_inspection_selects_general_sem_authority_after_exact_cbsem_recipe_append() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory
            .path()
            .join("general-sem-with-exact-cbsem-result.qpls");
        let (exact_recipe, exact_document) =
            create_general_sem_authority_for_exact_cbsem(&archive_path);
        let source_sha256 = sha256_file(&archive_path).unwrap().1;
        let append = append_recipe_v4_and_canonical_result_document_v2_file_v6(
            &archive_path,
            &source_sha256,
            exact_recipe,
            exact_document,
        )
        .unwrap();
        assert_eq!(append.canonical_result_document_count, 1);

        let outcome = inspect_archive_v6(&request(&archive_path));
        let ProjectArchiveV6ReadOutcomeV1::Ok { value } = outcome else {
            panic!("strict inspection rejected the exact-CB-SEM result recipe")
        };
        assert_eq!(value.counts.recipes, 2);
        assert_eq!(value.counts.canonical_result_documents, 1);
        let authority = value
            .general_sem_execution_authority
            .as_ref()
            .expect("General SEM authority must remain selected");
        assert!(authority.recipe.general_sem_config.is_some());
        assert_eq!(authority.recipe_id, "00000000-0000-0000-0000-00000000cba0");
    }

    #[test]
    fn strict_inspection_rejects_duplicate_general_sem_recipe_authorities() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("single-general-sem-authority.qpls");
        create_general_sem_authority_for_exact_cbsem(&source_path);
        let loaded = load_project_archive_v6(&source_path).unwrap();
        let mut document = loaded.document;
        let mut duplicate = document.recipes[0].clone();
        duplicate.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-00000000cba1").unwrap();
        document.recipes.push(duplicate);
        document.ensure_valid().unwrap();

        let duplicate_path = directory
            .path()
            .join("duplicate-general-sem-authority.qpls");
        write_schema6_zip(&duplicate_path, &document, &loaded.datasets);
        assert!(matches!(
            inspect_archive_v6(&request(&duplicate_path)),
            ProjectArchiveV6ReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_archive_read.general_sem_authority_invalid"
                    && diagnostic.message.contains("exactly one resident GeneralSemConfigV1")
        ));
    }

    #[test]
    fn marked_archive_dataset_page_is_bound_to_every_strict_identity() {
        let directory = tempfile::tempdir().unwrap();
        let request = populated_rows_request(&directory);

        let outcome = read_archive_dataset_rows_v1(&request);
        let ProjectArchiveV6DatasetRowsOutcomeV1::Ok { value } = outcome else {
            panic!("strict General SEM dataset page was not readable");
        };
        assert_eq!(value.archive_sha256, request.expected_archive_sha256);
        assert_eq!(value.project_id, request.project_id);
        assert_eq!(value.dataset_id, request.dataset_id);
        assert_eq!(value.dataset_fingerprint, request.dataset_fingerprint);
        assert_eq!(value.offset, 1);
        assert_eq!(value.limit, 2);
        assert_eq!(value.rows.len(), 2);
        assert_eq!(
            value.rows[0].get("x1").and_then(Option::as_deref),
            Some("2")
        );
        assert!(value.source_rechecked_unchanged);
    }

    #[test]
    fn marked_archive_dataset_page_fails_closed_for_digest_project_dataset_and_fingerprint_drift() {
        let directory = tempfile::tempdir().unwrap();
        let request = populated_rows_request(&directory);
        let mutations: Vec<(Box<dyn Fn(&mut ProjectArchiveV6DatasetRowsRequestV1)>, &str)> = vec![
            (
                Box::new(|candidate| candidate.expected_archive_sha256 = "f".repeat(64)),
                "schema6_dataset_rows.archive_identity_mismatch",
            ),
            (
                Box::new(|candidate| {
                    candidate.project_id = uuid::Uuid::from_u128(0xdead).to_string()
                }),
                "schema6_dataset_rows.project_identity_mismatch",
            ),
            (
                Box::new(|candidate| {
                    candidate.dataset_id = uuid::Uuid::from_u128(0xbeef).to_string()
                }),
                "schema6_dataset_rows.execution_authority_mismatch",
            ),
            (
                Box::new(|candidate| candidate.dataset_fingerprint = "wrong-fingerprint".into()),
                "schema6_dataset_rows.execution_authority_mismatch",
            ),
        ];
        for (mutate, expected_code) in mutations {
            let mut candidate = request.clone();
            mutate(&mut candidate);
            assert!(matches!(
                read_archive_dataset_rows_v1(&candidate),
                ProjectArchiveV6DatasetRowsOutcomeV1::Blocked { diagnostic }
                    if diagnostic.code == expected_code
            ));
        }
    }

    #[test]
    fn dataset_page_wire_is_strict_camel_case_and_bounded() {
        let valid = serde_json::json!({
            "surface": "internal_labs",
            "experimentalLabsEnabled": true,
            "archivePath": r"D:\projects\general-sem.qpls",
            "expectedArchiveSha256": "a".repeat(64),
            "projectId": "00000000-0000-0000-0000-000000000601",
            "datasetId": "00000000-0000-0000-0000-000000000602",
            "datasetFingerprint": "dataset-fingerprint",
            "offset": 0,
            "limit": 100,
        });
        serde_json::from_value::<ProjectArchiveV6DatasetRowsRequestV1>(valid.clone()).unwrap();
        let mut unknown = valid.clone();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<ProjectArchiveV6DatasetRowsRequestV1>(unknown).is_err());

        let mut too_large: ProjectArchiveV6DatasetRowsRequestV1 =
            serde_json::from_value(valid).unwrap();
        too_large.limit = MAX_ARCHIVE_DATASET_ROW_PAGE_SIZE + 1;
        assert!(matches!(
            read_archive_dataset_rows_v1(&too_large),
            ProjectArchiveV6DatasetRowsOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_dataset_rows.page_bounds_invalid"
        ));
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
