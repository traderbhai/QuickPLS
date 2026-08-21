use crate::general_sem_registry_access_v1::{
    GeneralSemRegistryAccessErrorV1, authorize_general_sem_registry_read_access_v1,
    decision_declares_general_sem_execution_cell_v1, general_sem_recipe_execution_surface_v1,
    is_rank0_general_sem_execution_cell_v1, selected_general_sem_execution_cell_v1,
};
use crate::recipe_v4_canonical_result::validate_archived_recipe_v4_pls_method_identity;
use crate::recipe_v4_cbsem_canonical_result::validate_archived_recipe_v4_cbsem_method_identity;
use crate::recipe_v4_general_sem_canonical_result::validate_archived_general_sem_pls_method_identity_v1;
use qpls_core::{
    AnalysisRecipeModelBindingV4, CapabilityCellReferenceV2, SemModelV4,
    preflight_general_sem_pls_v1,
};
use qpls_project::{
    CanonicalResultDocumentAttachmentV2, CanonicalResultDocumentV2, ProjectArchiveDocumentV6,
    ProjectModelPayloadV6, canonical_result_document_v2_json, deserialize_project_document_v6,
    load_project_archive_v6,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const STANDARD_EXACT_CBSEM_SURFACE: &str = "standard_exact_cbsem";
const RESULT_READ_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectSchema6ResultReadRequestV1 {
    pub(crate) surface: String,
    pub(crate) experimental_labs_enabled: bool,
    #[serde(default)]
    pub(crate) capability_cell: Option<CapabilityCellReferenceV2>,
    pub(crate) archive_path: String,
    pub(crate) expected_source_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectSchema6CanonicalResultEntryV1 {
    pub(crate) document_id: String,
    pub(crate) run_id: String,
    pub(crate) canonical_document_sha256: String,
    pub(crate) immutable: bool,
    pub(crate) canonical_document_json: String,
    pub(crate) canonical_document: CanonicalResultDocumentV2,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectSchema6ResultReadSnapshotV1 {
    pub(crate) schema_version: u32,
    pub(crate) project_id: String,
    pub(crate) archive_path: String,
    pub(crate) source_document_sha256: String,
    pub(crate) canonical_result_document_count: usize,
    pub(crate) documents: Vec<ProjectSchema6CanonicalResultEntryV1>,
    pub(crate) source_rechecked_unchanged: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectSchema6ResultReadDiagnosticV1 {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) corrective_action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectSchema6ResultReadOutcomeV1 {
    Ok {
        value: ProjectSchema6ResultReadSnapshotV1,
    },
    Blocked {
        diagnostic: ProjectSchema6ResultReadDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectSchema6ResultReadOutcomeV1 {
    ProjectSchema6ResultReadOutcomeV1::Blocked {
        diagnostic: ProjectSchema6ResultReadDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_result_entry(
    attachment: &CanonicalResultDocumentAttachmentV2,
) -> Result<ProjectSchema6CanonicalResultEntryV1, String> {
    validate_archived_recipe_v4_cbsem_method_identity(attachment.canonical_document())?;
    validate_archived_recipe_v4_pls_method_identity(attachment.canonical_document())?;
    validate_archived_general_sem_pls_method_identity_v1(attachment.canonical_document())?;
    let canonical_json = canonical_result_document_v2_json(attachment.canonical_document())
        .map_err(|error| error.to_string())?;
    let observed_sha256 = sha256(&canonical_json);
    if observed_sha256 != attachment.canonical_document_sha256() {
        return Err(format!(
            "canonical document {} digest differs from its exact canonical JSON bytes",
            attachment.document_id()
        ));
    }
    let canonical_document_json =
        String::from_utf8(canonical_json).map_err(|error| error.to_string())?;
    Ok(ProjectSchema6CanonicalResultEntryV1 {
        document_id: attachment.document_id().to_owned(),
        run_id: attachment.run_id().to_owned(),
        canonical_document_sha256: attachment.canonical_document_sha256().to_owned(),
        immutable: attachment.immutable(),
        canonical_document_json,
        canonical_document: attachment.canonical_document().clone(),
    })
}

fn registry_access_block(
    error: GeneralSemRegistryAccessErrorV1,
) -> ProjectSchema6ResultReadOutcomeV1 {
    match error {
        GeneralSemRegistryAccessErrorV1::RegistryInvalid(detail) => blocked(
            "schema6_result_read.capability_registry_invalid",
            format!("Capability Registry V2 is invalid: {detail}"),
            "Keep the archive unchanged and repair the embedded registry before reopening results.",
        ),
        GeneralSemRegistryAccessErrorV1::CapabilityUnavailable => blocked(
            "schema6_result_read.capability_unavailable",
            "The exact General SEM execution cell is not available in Capability Registry V2.",
            "Refresh exact estimator access before reopening this result archive.",
        ),
        GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired => blocked(
            "schema6_result_read.standard_surface_required",
            "The exact General SEM execution cell requires the Standard surface.",
            "Refresh capability access and reopen through Standard without relabelling stored results.",
        ),
        GeneralSemRegistryAccessErrorV1::InternalLabsRequired => blocked(
            "schema6_result_read.internal_labs_required",
            "The recognized General SEM result cell belongs to the historical Labs read surface.",
            "Reopen it read-only through its stored Labs surface; Experimental Labs opt-in is not required for reading.",
        ),
    }
}

fn general_sem_recipe_model<'a>(
    document: &'a ProjectArchiveDocumentV6,
    binding: &'a AnalysisRecipeModelBindingV4,
) -> Option<&'a SemModelV4> {
    match binding {
        AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { model, .. } => Some(model),
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference { model_id, .. } => document
            .models
            .iter()
            .find(|record| record.model_id == *model_id)
            .and_then(|record| match &record.payload {
                ProjectModelPayloadV6::SemModelV4 { model, .. } => Some(model),
                ProjectModelPayloadV6::SemModelV4Draft { .. }
                | ProjectModelPayloadV6::LegacyEstimandUnspecified { .. } => None,
            }),
        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => None,
    }
}

fn validate_exact_general_sem_archive_read_authority(
    request: &ProjectSchema6ResultReadRequestV1,
    document: &ProjectArchiveDocumentV6,
) -> Result<(), ProjectSchema6ResultReadOutcomeV1> {
    let Some(requested_cell) = request.capability_cell.as_ref() else {
        return Ok(());
    };
    let matching_recipes = document
        .recipes
        .iter()
        .filter(|recipe| recipe.general_sem_config.is_some())
        .collect::<Vec<_>>();
    let [recipe] = matching_recipes.as_slice() else {
        return Err(blocked(
            "schema6_result_read.general_sem_recipe_authority_mismatch",
            "Exact-cell General SEM readback requires one resident GeneralSemConfigV1 RecipeV4 authority.",
            "Keep the archive unchanged and reopen it through its exact strict General SEM authority.",
        ));
    };
    let expected_surface = general_sem_recipe_execution_surface_v1(&request.surface);
    if recipe.metadata.get("execution_surface").map(String::as_str) != expected_surface
        || recipe
            .metadata
            .get("general_sem_generation")
            .map(String::as_str)
            != Some("general_sem_v1")
    {
        return Err(blocked(
            "schema6_result_read.recipe_execution_surface_mismatch",
            "The exact read surface differs from the resident RecipeV4 surface identity.",
            "Reopen read-only through the recipe's stored surface without relabelling the archive.",
        ));
    }
    let Some(model) = general_sem_recipe_model(document, &recipe.model_binding) else {
        return Err(blocked(
            "schema6_result_read.general_sem_model_authority_mismatch",
            "The resident General SEM RecipeV4 does not resolve to one executable SemModelV4 authority.",
            "Keep the archive unchanged and restore its exact resident model authority.",
        ));
    };
    let decision = preflight_general_sem_pls_v1(
        model,
        recipe
            .general_sem_config
            .as_ref()
            .expect("matching recipe has GeneralSemConfigV1"),
    )
    .map_err(|error| {
        blocked(
            "schema6_result_read.capability_decision_invalid",
            format!("The resident General SEM capability decision is invalid: {error}"),
            "Keep the archive unchanged and report this authority-contract failure.",
        )
    })?;
    let selected = selected_general_sem_execution_cell_v1(
        model,
        recipe
            .general_sem_config
            .as_ref()
            .expect("matching recipe has GeneralSemConfigV1"),
    );
    if !decision_declares_general_sem_execution_cell_v1(&decision, &selected)
        || &selected != requested_cell
    {
        return Err(blocked(
            "schema6_result_read.capability_archive_mismatch",
            "The requested read cell differs from the resident RecipeV4 point-or-supplemental execution owner.",
            "Rehydrate exact read access from the unchanged resident recipe and retry.",
        ));
    }
    Ok(())
}

fn read_schema6_results_using<F>(
    request: &ProjectSchema6ResultReadRequestV1,
    authorize: F,
) -> ProjectSchema6ResultReadOutcomeV1
where
    F: Fn(&str, bool, &CapabilityCellReferenceV2) -> Result<(), GeneralSemRegistryAccessErrorV1>,
{
    let historical_internal =
        request.capability_cell.is_none() && request.surface == INTERNAL_LABS_SURFACE;
    let current_exact_cbsem = request.capability_cell.is_none()
        && request.surface == STANDARD_EXACT_CBSEM_SURFACE
        && !request.experimental_labs_enabled;
    let exact_general_sem = if let Some(cell) = request.capability_cell.as_ref() {
        if !is_rank0_general_sem_execution_cell_v1(cell) {
            return blocked(
                "schema6_result_read.capability_unavailable",
                "The selected cell is not one of the exact General SEM result-reopen cells.",
                "Refresh exact General SEM estimator access before reading the archive.",
            );
        }
        if let Err(error) = authorize(&request.surface, request.experimental_labs_enabled, cell) {
            return registry_access_block(error);
        }
        true
    } else {
        false
    };
    if !historical_internal && !current_exact_cbsem && !exact_general_sem {
        return blocked(
            "schema6_result_read.surface_mismatch",
            "Schema-6 saved-result reopening requires historical Labs access, exact Registry-authorized General SEM access, or the exact-CB-SEM Standard boundary.",
            "Select the exact execution cell and matching surface before reopening results.",
        );
    }
    let archive_path = Path::new(request.archive_path.trim());
    if request.archive_path.trim().is_empty() || !archive_path.is_absolute() {
        return blocked(
            "schema6_result_read.absolute_path_required",
            "The schema-6 archive path must be an absolute local path.",
            "Select the exact schema-6 project copy before reopening saved results.",
        );
    }
    if request.expected_source_sha256.len() != 64
        || !request
            .expected_source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return blocked(
            "schema6_result_read.invalid_source_digest",
            "The expected schema-6 source digest must be a lowercase SHA-256 value.",
            "Inspect the current schema-6 project and pass its exact recorded digest.",
        );
    }
    let metadata = match fs::symlink_metadata(archive_path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            metadata
        }
        Ok(_) => {
            return blocked(
                "schema6_result_read.regular_file_required",
                "The schema-6 result source must be a regular local file and cannot be a symbolic link.",
                "Select the exact regular schema-6 project file.",
            );
        }
        Err(error) => {
            return blocked(
                "schema6_result_read.source_unavailable",
                error.to_string(),
                "Verify that the schema-6 project still exists and is readable, then retry.",
            );
        }
    };
    if metadata.len() == 0 {
        return blocked(
            "schema6_result_read.empty_source",
            "The schema-6 project file is empty.",
            "Select a complete schema-6 project file.",
        );
    }

    let source_bytes = match fs::read(archive_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return blocked(
                "schema6_result_read.source_unavailable",
                error.to_string(),
                "Verify that the schema-6 project is readable, then retry.",
            );
        }
    };
    let observed_sha256 = sha256(&source_bytes);
    if observed_sha256 != request.expected_source_sha256 {
        return blocked(
            "schema6_result_read.source_changed",
            format!(
                "The schema-6 project digest changed: expected {}, observed {observed_sha256}.",
                request.expected_source_sha256
            ),
            "Reinspect the current schema-6 project before reopening its saved results.",
        );
    }
    let document = match if source_bytes.starts_with(b"PK\x03\x04") {
        load_project_archive_v6(archive_path)
            .map(|archive| archive.document)
            .map_err(|error| error.to_string())
    } else {
        deserialize_project_document_v6(&source_bytes).map_err(|error| error.to_string())
    } {
        Ok(document) => document,
        Err(error) => {
            return blocked(
                "schema6_result_read.invalid_archive",
                error.to_string(),
                "Repair or restore the schema-6 project from a trusted copy before reopening results.",
            );
        }
    };
    let rechecked = match fs::read(archive_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return blocked(
                "schema6_result_read.source_unavailable",
                error.to_string(),
                "Retry after the schema-6 project finishes its current file operation.",
            );
        }
    };
    if rechecked != source_bytes || sha256(&rechecked) != observed_sha256 {
        return blocked(
            "schema6_result_read.source_changed_during_read",
            "The schema-6 project changed while its canonical results were being validated.",
            "Reinspect the current project and retry after other writers finish.",
        );
    }
    if let Err(outcome) = validate_exact_general_sem_archive_read_authority(request, &document) {
        return outcome;
    }

    let documents = match document
        .canonical_result_documents
        .iter()
        .map(canonical_result_entry)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(documents) => documents,
        Err(error) => {
            return blocked(
                "schema6_result_read.canonical_encoding_failed",
                error,
                "Restore the schema-6 project from a trusted copy before reopening results.",
            );
        }
    };
    ProjectSchema6ResultReadOutcomeV1::Ok {
        value: ProjectSchema6ResultReadSnapshotV1 {
            schema_version: RESULT_READ_SCHEMA_VERSION,
            project_id: document.project_id.to_string(),
            archive_path: archive_path.to_string_lossy().into_owned(),
            source_document_sha256: observed_sha256,
            canonical_result_document_count: documents.len(),
            documents,
            source_rechecked_unchanged: true,
        },
    }
}

fn read_schema6_results(
    request: &ProjectSchema6ResultReadRequestV1,
) -> ProjectSchema6ResultReadOutcomeV1 {
    read_schema6_results_using(request, |surface, _labs_enabled, cell| {
        authorize_general_sem_registry_read_access_v1(surface, cell)
    })
}

#[tauri::command]
pub(crate) fn read_internal_project_schema6_canonical_results_v2(
    request: ProjectSchema6ResultReadRequestV1,
) -> ProjectSchema6ResultReadOutcomeV1 {
    read_schema6_results(&request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_v4_canonical_result::build_recipe_v4_pls_canonical_result;
    use crate::recipe_v4_cbsem_canonical_result::build_recipe_v4_cbsem_canonical_result;
    use crate::recipe_v4_cbsem_execution::{
        execute_internal_recipe_v4_cbsem, resolve_internal_recipe_v4_cbsem_dataset,
    };
    use crate::{execute_internal_recipe_v4_pls, resolve_internal_recipe_v4_dataset};
    use chrono::{TimeZone, Utc};
    use qpls_data::{Dataset, write_arrow};
    use qpls_project::{
        PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6,
        ProjectArchiveUpgradeRequestV6, ProjectManifest, ProjectModelPayloadV6,
        ProjectModelRecordV6, attach_canonical_result_document_v2_v6,
        canonical_result_document_v2_sha256, deserialize_project_document_v6,
        plan_project_upgrade_to_v6, serialize_project_document_v6,
    };
    use std::{collections::BTreeMap, fs::File, io::Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    const CROSS_RUNTIME_CANONICAL_JSON: &str = r#"{"charts":[],"document_id":"result.contract:1","exclusions":[],"footnotes":[],"notices":[],"presentation":{"chart_defaults":{},"default_section_id":"results","default_table_id":"numeric_contract","missing_value_label":"-","precision":4},"provenance":{"capability_cell":{"capability_id":"quickpls.numeric_contract","capability_version":"numeric_contract_v1","cell_id":"quickpls.numeric_contract","registry_schema_version":2},"completed_at":"2026-08-14T00:00:01Z","dataset_fingerprint":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","dataset_id":"dataset-1","engine_version":"quickpls-test","method_version":"numeric_contract_v1","model_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","model_id":"model-1","project_id":"project-1","recipe_digest":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","recipe_id":"recipe-1","run_id":"run-1","seed":null,"started_at":"2026-08-14T00:00:00Z","workers":1},"schema_version":2,"sections":[{"chart_ids":[],"id":"results","table_ids":["numeric_contract"],"title":"Results"}],"tables":[{"columns":[{"data_type":"number","description":"Integral floating-point value","id":"integral","label":"Integral"},{"data_type":"number","description":"Non-integral floating-point value","id":"non_integral","label":"Non-integral"}],"footnote_ids":[],"id":"numeric_contract","rows":[{"cells":[{"kind":"number","value":1.0},{"kind":"number","value":0.9954396945354063}],"id":"row_1"}],"title":"Numeric contract"}],"title":"Cross-runtime numeric contract"}"#;

    fn request(path: &Path, digest: String) -> ProjectSchema6ResultReadRequestV1 {
        ProjectSchema6ResultReadRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            capability_cell: None,
            archive_path: path.to_string_lossy().into_owned(),
            expected_source_sha256: digest,
        }
    }

    fn write_schema6_zip(
        path: &Path,
        document: &ProjectArchiveDocumentV6,
        datasets: &[Dataset],
    ) -> Vec<u8> {
        let mut entries: Vec<(String, Vec<u8>)> = vec![(
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
            .map(|(name, bytes)| (name.clone(), sha256(bytes)))
            .collect::<BTreeMap<_, _>>();
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
        fs::read(path).unwrap()
    }

    fn install_explicit_pls_recipe(
        document: &mut ProjectArchiveDocumentV6,
        request: &crate::InternalRecipeV4PlsExecutionRequestV1,
    ) {
        document
            .models
            .retain(|record| record.model_id != request.model.id);
        document.models.push(ProjectModelRecordV6 {
            model_id: request.model.id.clone(),
            payload: ProjectModelPayloadV6::SemModelV4 {
                model: request.model.clone(),
                scientific_sha256: request.model.scientific_sha256().unwrap(),
            },
        });
        document
            .recipes
            .retain(|recipe| recipe.id != request.recipe.id);
        document.recipes.push(request.recipe.clone());
        document.ensure_valid().unwrap();
    }

    #[test]
    fn exact_recipe_result_reopens_from_schema6_without_reconstruction() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.qpls");
        let archive_path = directory.path().join("upgraded-v6.json");
        let (project, execution_request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &execution_request).unwrap();
        let analytical = execute_internal_recipe_v4_pls(&dataset, &execution_request).unwrap();
        let job_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000101").unwrap();
        let canonical = build_recipe_v4_pls_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-14T00:00:00.000Z",
            "2026-08-14T00:00:01.000Z",
            &execution_request,
            &analytical,
        )
        .unwrap();
        let archive_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        let expected_canonical_json =
            canonical_result_document_v2_json(&archive_canonical).unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: source_path.to_string_lossy().into_owned(),
                destination_archive_path: archive_path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 14, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let document =
            attach_canonical_result_document_v2_v6(&plan.document, archive_canonical).unwrap();
        let bytes = serialize_project_document_v6(&document).unwrap();
        fs::write(&archive_path, &bytes).unwrap();

        let outcome = read_schema6_results(&request(&archive_path, sha256(&bytes)));
        let ProjectSchema6ResultReadOutcomeV1::Ok { value } = outcome else {
            panic!("schema-6 result did not reopen: {outcome:?}");
        };
        assert_eq!(value.canonical_result_document_count, 1);
        assert!(value.source_rechecked_unchanged);
        assert_eq!(value.documents[0].run_id, job_id.to_string());
        assert_eq!(
            value.documents[0].canonical_document_json.as_bytes(),
            expected_canonical_json
        );
        assert_eq!(
            value.documents[0].canonical_document_sha256,
            sha256(value.documents[0].canonical_document_json.as_bytes())
        );
        assert_eq!(
            serde_json::to_value(&value.documents[0].canonical_document).unwrap(),
            serde_json::to_value(&canonical).unwrap()
        );

        let zip_path = directory.path().join("upgraded-v6.qpls");
        let zip_bytes = write_schema6_zip(&zip_path, &document, &project.datasets);
        let ProjectSchema6ResultReadOutcomeV1::Ok { value: zip_value } =
            read_schema6_results(&request(&zip_path, sha256(&zip_bytes)))
        else {
            panic!("schema-6 ZIP result did not reopen")
        };
        assert_eq!(zip_value.project_id, value.project_id);
        assert_eq!(zip_value.documents, value.documents);
        assert!(zip_value.source_rechecked_unchanged);
    }

    #[test]
    fn configured_score_execution_reopens_and_rejects_digest_regenerated_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("configured-source.qpls");
        let archive_path = directory.path().join("configured-v6.json");
        let (project, mut execution_request) =
            crate::internal_recipe_v4_pls_command_tests::fixture();
        execution_request.posthoc_technical_minimum_sample_size = None;
        execution_request.recipe.method_config =
            Some(qpls_core::MethodConfig::PlsAlgorithmConfiguredV2(
                qpls_core::PlsAlgorithmConfigV2::standard(),
            ));
        let dataset = resolve_internal_recipe_v4_dataset(&project, &execution_request).unwrap();
        let analytical = execute_internal_recipe_v4_pls(&dataset, &execution_request).unwrap();
        let canonical = build_recipe_v4_pls_canonical_result(
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000102").unwrap(),
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &execution_request,
            &analytical,
        )
        .unwrap();
        let archive_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(canonical).unwrap(),
        )
        .unwrap();
        let mut plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: source_path.to_string_lossy().into_owned(),
                destination_archive_path: archive_path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        install_explicit_pls_recipe(&mut plan.document, &execution_request);
        let document =
            attach_canonical_result_document_v2_v6(&plan.document, archive_canonical).unwrap();
        let exact_bytes = serialize_project_document_v6(&document).unwrap();
        fs::write(&archive_path, &exact_bytes).unwrap();
        assert!(matches!(
            read_schema6_results(&request(&archive_path, sha256(&exact_bytes))),
            ProjectSchema6ResultReadOutcomeV1::Ok { .. }
        ));

        let mut tampered = serde_json::to_value(document).unwrap();
        let tables = tampered["canonical_result_documents"][0]["canonical_document"]["tables"]
            .as_array_mut()
            .unwrap();
        let score_weights = tables
            .iter_mut()
            .find(|table| table["id"] == qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2)
            .unwrap();
        let resolved = score_weights["rows"][0]["cells"][7]["value"]
            .as_f64()
            .unwrap();
        score_weights["rows"][0]["cells"][7]["value"] = serde_json::json!(resolved * 0.5);
        let tampered_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            tampered["canonical_result_documents"][0]["canonical_document"].clone(),
        )
        .unwrap();
        tampered["canonical_result_documents"][0]["canonical_document_sha256"] =
            serde_json::json!(canonical_result_document_v2_sha256(&tampered_canonical).unwrap());
        let tampered_bytes = serde_json::to_vec(&tampered).unwrap();
        let error = deserialize_project_document_v6(&tampered_bytes).unwrap_err();
        assert!(error.to_string().contains("score-execution"));
    }

    #[test]
    fn nonlinear_v7_reopens_and_rejects_digest_regenerated_diagnostic_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("nonlinear-source.qpls");
        let archive_path = directory.path().join("nonlinear-v7.json");
        let (project, execution_request) =
            crate::internal_recipe_v4_pls_command_tests::nonlinear_fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &execution_request).unwrap();
        let analytical = execute_internal_recipe_v4_pls(&dataset, &execution_request).unwrap();
        let canonical = build_recipe_v4_pls_canonical_result(
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000107").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &execution_request,
            &analytical,
        )
        .unwrap();
        let archive_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(canonical).unwrap(),
        )
        .unwrap();
        let mut plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: source_path.to_string_lossy().into_owned(),
                destination_archive_path: archive_path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 16, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        install_explicit_pls_recipe(&mut plan.document, &execution_request);
        let document =
            attach_canonical_result_document_v2_v6(&plan.document, archive_canonical).unwrap();
        let exact_bytes = serialize_project_document_v6(&document).unwrap();
        fs::write(&archive_path, &exact_bytes).unwrap();
        let ProjectSchema6ResultReadOutcomeV1::Ok { value } =
            read_schema6_results(&request(&archive_path, sha256(&exact_bytes)))
        else {
            panic!("exact nonlinear v7 result did not reopen")
        };
        assert_eq!(
            value.documents[0]
                .canonical_document
                .provenance
                .engine_version,
            qpls_runner::RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        );

        let mut tampered = serde_json::to_value(document).unwrap();
        let diagnostics = tampered["canonical_result_documents"][0]["canonical_document"]["tables"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|table| table["id"] == "nonlinear_quadratic_diagnostics")
            .unwrap();
        let quadratic = diagnostics["rows"][0]["cells"][3]["value"]
            .as_f64()
            .unwrap();
        diagnostics["rows"][0]["cells"][3]["value"] = serde_json::json!(quadratic + 0.125);
        let tampered_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            tampered["canonical_result_documents"][0]["canonical_document"].clone(),
        )
        .unwrap();
        tampered["canonical_result_documents"][0]["canonical_document_sha256"] =
            serde_json::json!(canonical_result_document_v2_sha256(&tampered_canonical).unwrap());
        let tampered_bytes = serde_json::to_vec(&tampered).unwrap();
        fs::write(&archive_path, &tampered_bytes).unwrap();
        assert!(matches!(
            read_schema6_results(&request(&archive_path, sha256(&tampered_bytes))),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if matches!(
                    diagnostic.code.as_str(),
                    "schema6_result_read.canonical_encoding_failed"
                        | "schema6_result_read.invalid_archive"
                )
        ));
    }

    #[test]
    fn exact_cbsem_v3_identity_reopens_and_a_v2_relabel_fails_closed() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("cbsem-v3-schema6.json");
        let (project, execution_request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let dataset =
            resolve_internal_recipe_v4_cbsem_dataset(&project, &execution_request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &execution_request).unwrap();
        let job_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-00000000cb63").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &execution_request,
            &analytical,
        )
        .unwrap();
        let archive_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: directory
                    .path()
                    .join("source.qpls")
                    .to_string_lossy()
                    .into_owned(),
                destination_archive_path: archive_path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut current_document = plan.document.clone();
        current_document
            .recipes
            .push(execution_request.recipe.clone());
        current_document.ensure_valid().unwrap();
        let exact_document =
            attach_canonical_result_document_v2_v6(&current_document, archive_canonical.clone())
                .unwrap();
        let exact_bytes = serialize_project_document_v6(&exact_document).unwrap();
        fs::write(&archive_path, &exact_bytes).unwrap();

        let outcome = read_schema6_results(&request(&archive_path, sha256(&exact_bytes)));
        let ProjectSchema6ResultReadOutcomeV1::Ok { value } = outcome else {
            panic!("exact CB-SEM v3 result did not reopen: {outcome:?}");
        };
        assert_eq!(value.canonical_result_document_count, 1);
        assert_eq!(
            value.documents[0]
                .canonical_document
                .provenance
                .method_version,
            qpls_estimation::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert_eq!(
            value.documents[0]
                .canonical_document
                .provenance
                .engine_version,
            qpls_runner::RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
        );

        let mut stale = archive_canonical;
        stale.provenance.method_version = "cbsem_ml_compiled_moment_input_v2".into();
        stale.ensure_valid().unwrap();
        let _error = attach_canonical_result_document_v2_v6(&current_document, stale).unwrap_err();
    }

    #[test]
    fn exact_cbsem_raw_mean_v4_identity_reopens_with_mean_tables() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("cbsem-mean-v4-schema6.json");
        let (project, execution_request) = crate::recipe_v4_cbsem_execution::tests::mean_fixture();
        let dataset =
            resolve_internal_recipe_v4_cbsem_dataset(&project, &execution_request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &execution_request).unwrap();
        let job_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-00000000cb64").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &execution_request,
            &analytical,
        )
        .unwrap();
        let archive_canonical = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: directory
                    .path()
                    .join("source-mean.qpls")
                    .to_string_lossy()
                    .into_owned(),
                destination_archive_path: archive_path.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut current_document = plan.document;
        current_document
            .recipes
            .push(execution_request.recipe.clone());
        current_document.ensure_valid().unwrap();
        let document =
            attach_canonical_result_document_v2_v6(&current_document, archive_canonical).unwrap();
        let bytes = serialize_project_document_v6(&document).unwrap();
        fs::write(&archive_path, &bytes).unwrap();

        let outcome = read_schema6_results(&request(&archive_path, sha256(&bytes)));
        let ProjectSchema6ResultReadOutcomeV1::Ok { value } = outcome else {
            panic!("exact CB-SEM raw mean v4 result did not reopen: {outcome:?}");
        };
        let canonical = &value.documents[0].canonical_document;
        assert_eq!(
            canonical.provenance.method_version,
            qpls_estimation::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
        );
        assert_eq!(
            canonical.provenance.engine_version,
            qpls_runner::RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6
        );
        for table_id in ["observed_means", "implied_means", "residual_means"] {
            assert!(canonical.tables.iter().any(|table| table.id == table_id));
        }
    }

    #[test]
    fn cross_runtime_json_preserves_integral_and_non_integral_f64_lexemes() {
        let canonical =
            serde_json::from_str::<CanonicalResultDocumentV2>(CROSS_RUNTIME_CANONICAL_JSON)
                .unwrap();
        let canonical_bytes = canonical_result_document_v2_json(&canonical).unwrap();
        assert_eq!(canonical_bytes, CROSS_RUNTIME_CANONICAL_JSON.as_bytes());
        assert!(CROSS_RUNTIME_CANONICAL_JSON.contains(r#""value":1.0"#));
        assert!(CROSS_RUNTIME_CANONICAL_JSON.contains(r#""value":0.9954396945354063"#));

        let attachment = CanonicalResultDocumentAttachmentV2::from_document(canonical).unwrap();
        let entry = canonical_result_entry(&attachment).unwrap();
        assert_eq!(entry.canonical_document_json, CROSS_RUNTIME_CANONICAL_JSON);
        assert_eq!(
            entry.canonical_document_sha256,
            sha256(CROSS_RUNTIME_CANONICAL_JSON.as_bytes())
        );
        let wire = serde_json::to_value(&entry).unwrap();
        assert_eq!(wire["canonicalDocumentJson"], CROSS_RUNTIME_CANONICAL_JSON);
        assert_eq!(
            wire["canonicalDocument"]["tables"][0]["rows"][0]["cells"][0]["value"],
            1.0
        );
    }

    #[test]
    fn access_digest_and_tampering_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invalid.json");
        fs::write(&path, br#"{"schema_version":6}"#).unwrap();
        let mut denied = request(&path, sha256(br#"{"schema_version":6}"#));
        denied.surface = "standard".into();
        assert!(matches!(
            read_schema6_results(&denied),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.surface_mismatch"
        ));

        let mut exact = request(&path, sha256(br#"{"schema_version":6}"#));
        exact.surface = STANDARD_EXACT_CBSEM_SURFACE.into();
        exact.experimental_labs_enabled = false;
        assert!(!matches!(
            read_schema6_results(&exact),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.surface_mismatch"
        ));

        let stale = request(&path, "b".repeat(64));
        assert!(matches!(
            read_schema6_results(&stale),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.source_changed"
        ));

        let invalid = request(&path, sha256(br#"{"schema_version":6}"#));
        assert!(matches!(
            read_schema6_results(&invalid),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.invalid_archive"
        ));
    }

    #[test]
    fn read_only_labs_compatibility_and_standard_authorization_run_before_path_access() {
        let relative = Path::new("historical-general-sem.qpls");
        let mut historical = request(relative, "a".repeat(64));
        historical.experimental_labs_enabled = false;
        historical.capability_cell =
            Some(qpls_core::pls_general_recursive_effects_capability_cell_v1());
        assert!(matches!(
            read_schema6_results(&historical),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.absolute_path_required"
        ));

        let mut standard = historical.clone();
        standard.surface = "standard".into();
        assert!(matches!(
            read_schema6_results_using(&standard, |surface, enabled, _cell| {
                assert_eq!(surface, "standard");
                assert!(!enabled);
                Ok(())
            }),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.absolute_path_required"
        ));

        let mut tampered = standard;
        tampered
            .capability_cell
            .as_mut()
            .unwrap()
            .capability_version
            .push_str(".tampered");
        assert!(matches!(
            read_schema6_results_using(&tampered, |_surface, _enabled, _cell| {
                panic!("unknown cells must be rejected before Registry authorization or path access")
            }),
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_read.capability_unavailable"
        ));
    }
}
