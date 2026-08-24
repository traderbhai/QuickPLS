//! Strict, read-only ZIP codec for schema-6 QuickPLS project archives.
//!
//! This module deliberately exposes no writer. The live project service remains
//! schema 5 while schema-6 archive bytes acquire an independently validated
//! read path.

use super::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectError, ProjectManifest,
    archive_integrity::{
        DEFAULT_ARCHIVE_LIMITS, MANIFEST_ENTRY_NAME, MAX_MANIFEST_UNCOMPRESSED_BYTES,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES, PROJECT_ENTRY_NAME,
        expected_project_entries_with_additional, preflight_archive, read_preflighted_entry,
        validate_expected_project_entries, validate_manifest_checksums,
        validate_raw_central_directory, verify_archive_checksums,
    },
    deserialize_project_document_v6, map_archive_integrity_error,
    reject_duplicate_json_object_keys, validate_multimod_sidecar_stream_v1,
    validate_project_data_lineage_resident_v1,
};
use qpls_data::{Dataset, dataset_from_descriptor};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Seek, SeekFrom},
    path::Path,
};
use zip::ZipArchive;

const SCHEMA_V6_MANIFEST_FIELDS: [&str; 8] = [
    "checksum_algorithm",
    "checksums",
    "created_at",
    "engine_version",
    "modified_at",
    "name",
    "project_id",
    "schema_version",
];

/// A fully validated schema-6 archive.
///
/// The schema-6 document remains the authority for models, recipes, history,
/// layouts, and result attachments. Resident Arrow datasets are returned
/// separately after their descriptor fingerprints and lineage bindings pass.
#[derive(Debug)]
pub struct LoadedProjectArchiveV6 {
    pub manifest: ProjectManifest,
    pub document: ProjectArchiveDocumentV6,
    pub datasets: Vec<Dataset>,
    /// Entry identities of sidecars whose manifest checksum, byte count,
    /// evidence/schema contract, and complete Arrow stream were validated.
    /// Payload bytes are intentionally not retained in memory.
    pub multimod_sidecars: BTreeSet<String>,
}

/// Strictly reads one schema-6 `.qpls` ZIP without projecting it into the live
/// schema-5 `Project` type.
pub fn load_project_archive_v6(path: &Path) -> Result<LoadedProjectArchiveV6, ProjectError> {
    load_project_archive_v6_from_file(File::open(path)?)
}

/// Strictly reads one schema-6 archive from an already-open file identity.
///
/// This is the validation seam used by the Windows new-destination save-copy
/// writer. It prevents post-write validation from resolving the destination
/// pathname a second time after the writer has pinned and exclusively created
/// the file relative to its parent directory handle.
pub fn load_project_archive_v6_from_file(
    mut file: File,
) -> Result<LoadedProjectArchiveV6, ProjectError> {
    file.seek(SeekFrom::Start(0))?;
    validate_raw_central_directory(&mut file, DEFAULT_ARCHIVE_LIMITS)
        .map_err(map_archive_integrity_error)?;
    let mut archive = ZipArchive::new(file)?;
    let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS)
        .map_err(map_archive_integrity_error)?;

    let manifest_bytes = read_preflighted_entry(
        &mut archive,
        &preflight,
        MANIFEST_ENTRY_NAME,
        MAX_MANIFEST_UNCOMPRESSED_BYTES,
    )
    .map_err(map_archive_integrity_error)?;
    reject_duplicate_json_object_keys(&manifest_bytes, MANIFEST_ENTRY_NAME)?;
    let manifest_value: Value = serde_json::from_slice(&manifest_bytes)?;
    validate_schema_v6_manifest_fields(&manifest_value)?;
    let mut manifest: ProjectManifest = serde_json::from_value(manifest_value)?;
    if manifest.schema_version != PROJECT_ARCHIVE_SCHEMA_V6_VERSION {
        return Err(ProjectError::Invalid(format!(
            "schema-6 ZIP reader requires manifest schema version {} (found {})",
            PROJECT_ARCHIVE_SCHEMA_V6_VERSION, manifest.schema_version
        )));
    }
    if !manifest.checksum_algorithm.eq_ignore_ascii_case("sha256") {
        return Err(ProjectError::Invalid(format!(
            "unsupported archive checksum algorithm {}",
            manifest.checksum_algorithm
        )));
    }
    manifest.checksum_algorithm = "sha256".to_owned();

    let checksums = validate_manifest_checksums(&preflight, &manifest.checksums)
        .map_err(map_archive_integrity_error)?;
    verify_archive_checksums(&mut archive, &preflight, &checksums)
        .map_err(map_archive_integrity_error)?;

    let project_bytes = read_preflighted_entry(
        &mut archive,
        &preflight,
        PROJECT_ENTRY_NAME,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES,
    )
    .map_err(map_archive_integrity_error)?;
    let document = deserialize_project_document_v6(&project_bytes).map_err(|error| {
        ProjectError::Invalid(format!(
            "schema-6 project document failed strict validation: {error}"
        ))
    })?;
    validate_manifest_document_identity(&manifest, &document)?;

    let sidecar_entries = document
        .multimod_results
        .iter()
        .flat_map(|attachment| attachment.sidecars.iter())
        .map(|descriptor| descriptor.entry_name.clone())
        .collect::<Vec<_>>();
    let expected_entries = expected_project_entries_with_additional(
        document.datasets.iter().map(|item| item.id),
        sidecar_entries,
    )
    .map_err(map_archive_integrity_error)?;
    validate_expected_project_entries(&checksums, &expected_entries)
        .map_err(map_archive_integrity_error)?;

    let mut datasets = Vec::with_capacity(document.datasets.len());
    for descriptor in document.datasets.iter().cloned() {
        let entry_name = format!("data/{}.arrow", descriptor.id);
        let bytes = read_preflighted_entry(
            &mut archive,
            &preflight,
            &entry_name,
            DEFAULT_ARCHIVE_LIMITS.max_entry_uncompressed_bytes,
        )
        .map_err(map_archive_integrity_error)?;
        datasets.push(dataset_from_descriptor(descriptor, &bytes)?);
    }
    validate_project_data_lineage_resident_v1(&datasets, &document.layouts).map_err(|error| {
        ProjectError::Invalid(format!(
            "schema-6 resident dataset lineage failed validation: {error}"
        ))
    })?;

    let mut multimod_sidecars = BTreeSet::new();
    for attachment in &document.multimod_results {
        for descriptor in &attachment.sidecars {
            if checksums.get(&descriptor.entry_name) != Some(descriptor.sha256.as_str()) {
                return Err(ProjectError::Invalid(format!(
                    "schema-6 MultiMod sidecar descriptor checksum differs from the verified archive manifest: {}",
                    descriptor.entry_name
                )));
            }
            let entry = archive.by_name(&descriptor.entry_name)?;
            if entry.size() != descriptor.uncompressed_bytes {
                return Err(ProjectError::Invalid(format!(
                    "schema-6 MultiMod sidecar descriptor size differs from the ZIP entry: {}",
                    descriptor.entry_name
                )));
            }
            validate_multimod_sidecar_stream_v1(&attachment.result_id, descriptor, entry).map_err(
                |error| {
                    ProjectError::Invalid(format!(
                        "schema-6 MultiMod sidecar failed strict validation: {error}"
                    ))
                },
            )?;
            multimod_sidecars.insert(descriptor.entry_name.clone());
        }
    }

    Ok(LoadedProjectArchiveV6 {
        manifest,
        document,
        datasets,
        multimod_sidecars,
    })
}

fn validate_schema_v6_manifest_fields(value: &Value) -> Result<(), ProjectError> {
    let object = value.as_object().ok_or_else(|| {
        ProjectError::Invalid("schema-6 manifest must be a JSON object".to_owned())
    })?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = SCHEMA_V6_MANIFEST_FIELDS
        .into_iter()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        let unexpected = actual.difference(&expected).copied().collect::<Vec<_>>();
        return Err(ProjectError::Invalid(format!(
            "schema-6 manifest field set mismatch (missing: {}; unexpected: {})",
            display_names(&missing),
            display_names(&unexpected)
        )));
    }
    Ok(())
}

fn validate_manifest_document_identity(
    manifest: &ProjectManifest,
    document: &ProjectArchiveDocumentV6,
) -> Result<(), ProjectError> {
    if manifest.project_id != document.project_id
        || manifest.name != document.name
        || manifest.created_at != document.created_at
        || manifest.modified_at != document.modified_at
    {
        return Err(ProjectError::Invalid(
            "schema-6 manifest identity or timestamps differ from project.json".to_owned(),
        ));
    }
    Ok(())
}

fn display_names(names: &[&str]) -> String {
    if names.is_empty() {
        "<none>".to_owned()
    } else {
        names.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Project, ProjectArchiveUpgradeRequestV6, ProjectModelPayloadV6,
        append_canonical_result_document_v2_file_v6, attach_canonical_result_document_v2_v6,
        plan_project_upgrade_to_v6, serialize_project_document_v6,
    };
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisPayload, AnalysisRecipe,
        AnalysisResult, AnalysisSettings, Construct, MeasurementMode, MethodConfig, ModelSpec,
        RESULT_SCHEMA_VERSION, RunProvenance, RunStatus, StructuralPath,
    };
    use qpls_data::{DataFingerprint, ImportOptions, import_delimited_bytes, write_arrow};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        io::{Read, Write},
    };
    use uuid::Uuid;
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(10),
            name: "Legacy model".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        }
    }

    fn historical_recipe() -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(20),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "historical-dataset".into(),
            model: legacy_model(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        }
    }

    fn historical_result(recipe: &AnalysisRecipe) -> AnalysisResult {
        let completed_at = Utc.timestamp_opt(1_700_000_001, 0).unwrap();
        AnalysisResult {
            schema_version: RESULT_SCHEMA_VERSION,
            id: Uuid::from_u128(30),
            status: RunStatus::Completed,
            provenance: RunProvenance {
                recipe_id: recipe.id,
                dataset_fingerprint: recipe.dataset_fingerprint.clone(),
                method: recipe.settings.method,
                method_version: "historical".into(),
                engine_version: "historical".into(),
                seed: 7,
                settings: recipe.settings.clone(),
                started_at: recipe.created_at,
                completed_at,
            },
            diagnostics: Vec::new(),
            payload: AnalysisPayload::Legacy {
                value: json!({"coefficient": 0.9954396945354063}),
            },
        }
    }

    fn canonical_result(project_id: Uuid) -> crate::CanonicalResultDocumentV2 {
        serde_json::from_value(json!({
            "schema_version": 2,
            "document_id": "result.document:zip-codec",
            "title": "Preserved result",
            "provenance": {
                "run_id": "run-zip-codec",
                "project_id": project_id.to_string(),
                "model_id": "model-1",
                "model_digest": "a".repeat(64),
                "dataset_id": "dataset-1",
                "dataset_fingerprint": "b".repeat(64),
                "recipe_id": "recipe-1",
                "recipe_digest": "c".repeat(64),
                "capability_cell": {
                    "registry_schema_version": 2,
                    "capability_id": "smartpls.pls_algorithm",
                    "cell_id": "qpls3.pls.algorithm",
                    "capability_version": "pls_pm_v1"
                },
                "method_version": "pls_pm_v1",
                "engine_version": "historical-adapter",
                "seed": 42,
                "workers": 1,
                "started_at": "2026-08-14T00:00:00Z",
                "completed_at": "2026-08-14T00:00:01Z"
            },
            "sections": [],
            "tables": [],
            "charts": [],
            "notices": [],
            "exclusions": [],
            "footnotes": [],
            "presentation": {
                "default_section_id": null,
                "default_table_id": null,
                "precision": 4,
                "missing_value_label": "N/A",
                "chart_defaults": {}
            }
        }))
        .unwrap()
    }

    fn fixture() -> (ProjectArchiveDocumentV6, Vec<Dataset>) {
        let v2_dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,3,4\n5,6,7,8\n",
            "v2.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut legacy_dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n9,10,11,12\n13,14,15,16\n",
            "legacy.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        legacy_dataset.fingerprint =
            DataFingerprint(sha256(&write_arrow(&legacy_dataset.batch).unwrap()));

        let recipe = historical_recipe();
        let mut project = Project::new("Schema-6 ZIP fixture");
        project.datasets = vec![v2_dataset, legacy_dataset];
        project.models.push(legacy_model());
        project.recipes.push(recipe.clone());
        project.results.push(historical_result(&recipe));
        project.layouts.insert(
            "workspace".into(),
            json!({"zoom": 1.25, "selection": ["x"], "draft": true}),
        );
        let request = ProjectArchiveUpgradeRequestV6 {
            source_archive_sha256: "d".repeat(64),
            source_archive_path: r"D:\source.qpls".into(),
            destination_archive_path: r"D:\destination.qpls".into(),
            upgraded_at: Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
            legacy_display_covariances: BTreeMap::new(),
        };
        let mut document = plan_project_upgrade_to_v6(&project, &request)
            .unwrap()
            .document;
        let ready_model = match &document.models[0].payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. } => model.clone(),
            payload => panic!("fixture expected a ready model, found {payload:?}"),
        };
        let model_document_sha256 = ready_model.model_document_sha256().unwrap();
        document.models[0].payload = ProjectModelPayloadV6::SemModelV4Draft {
            model: ready_model,
            model_document_sha256,
        };
        document = attach_canonical_result_document_v2_v6(
            &document,
            canonical_result(document.project_id),
        )
        .unwrap();
        document.ensure_valid().unwrap();
        (document, project.datasets)
    }

    fn write_archive(path: &Path, document: &ProjectArchiveDocumentV6, datasets: &[Dataset]) {
        let project_bytes = serialize_project_document_v6(document).unwrap();
        let mut entries = vec![(PROJECT_ENTRY_NAME.to_owned(), project_bytes)];
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
            MANIFEST_ENTRY_NAME.to_owned(),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        ));
        write_raw_entries(path, &entries);
    }

    fn read_raw_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        (0..archive.len())
            .map(|index| {
                let mut entry = archive.by_index(index).unwrap();
                let name = entry.name().to_owned();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                (name, bytes)
            })
            .collect()
    }

    #[test]
    fn zip_append_preserves_resident_arrow_entries_and_strictly_reopens() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("append-schema6.qpls");
        let (document, datasets) = fixture();
        write_archive(&path, &document, &datasets);
        let source_bytes = std::fs::read(&path).unwrap();
        let source_sha256 = sha256(&source_bytes);
        let arrow_before = read_raw_entries(&path)
            .into_iter()
            .filter(|(name, _)| name.starts_with("data/"))
            .collect::<BTreeMap<_, _>>();
        let mut appended = canonical_result(document.project_id);
        appended.document_id = "result.document:zip-append".into();
        appended.provenance.run_id = "run-zip-append".into();

        let receipt =
            append_canonical_result_document_v2_file_v6(&path, &source_sha256, appended).unwrap();

        let reopened = load_project_archive_v6(&path).unwrap();
        assert_eq!(receipt.source_document_sha256, source_sha256);
        assert_eq!(
            receipt.updated_document_sha256,
            sha256(&std::fs::read(&path).unwrap())
        );
        assert_eq!(receipt.canonical_result_document_count, 2);
        assert_eq!(reopened.document.canonical_result_documents.len(), 2);
        let arrow_after = read_raw_entries(&path)
            .into_iter()
            .filter(|(name, _)| name.starts_with("data/"))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(arrow_after, arrow_before);
    }

    fn write_raw_entries(path: &Path, entries: &[(String, Vec<u8>)]) {
        let mut writer = ZipWriter::new(File::create(path).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn rewrite_central_directory_entry_name(path: &Path, from: &str, to: &str) {
        const CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
        const END_OF_CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

        assert_eq!(from.len(), to.len());
        let mut bytes = std::fs::read(path).unwrap();
        let eocd_offset = bytes
            .windows(END_OF_CENTRAL_DIRECTORY_SIGNATURE.len())
            .rposition(|window| window == END_OF_CENTRAL_DIRECTORY_SIGNATURE)
            .unwrap();
        let entry_count =
            u16::from_le_bytes([bytes[eocd_offset + 10], bytes[eocd_offset + 11]]) as usize;
        let mut cursor = u32::from_le_bytes([
            bytes[eocd_offset + 16],
            bytes[eocd_offset + 17],
            bytes[eocd_offset + 18],
            bytes[eocd_offset + 19],
        ]) as usize;
        let mut rewritten = 0;

        for _ in 0..entry_count {
            assert_eq!(
                &bytes[cursor..cursor + CENTRAL_DIRECTORY_SIGNATURE.len()],
                CENTRAL_DIRECTORY_SIGNATURE.as_slice()
            );
            let name_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
            let extra_len = u16::from_le_bytes([bytes[cursor + 30], bytes[cursor + 31]]) as usize;
            let comment_len = u16::from_le_bytes([bytes[cursor + 32], bytes[cursor + 33]]) as usize;
            let name_start = cursor + 46;
            let name_end = name_start + name_len;
            if &bytes[name_start..name_end] == from.as_bytes() {
                bytes[name_start..name_end].copy_from_slice(to.as_bytes());
                rewritten += 1;
            }
            cursor = name_end + extra_len + comment_len;
        }

        assert_eq!(
            rewritten, 1,
            "expected one central-directory entry to rewrite"
        );
        std::fs::write(path, bytes).unwrap();
    }

    fn rewrite_manifest(path: &Path, mutate: impl FnOnce(&mut Value)) {
        let mut entries = read_raw_entries(path);
        let (_, bytes) = entries
            .iter_mut()
            .find(|(name, _)| name == MANIFEST_ENTRY_NAME)
            .unwrap();
        let mut manifest: Value = serde_json::from_slice(bytes).unwrap();
        mutate(&mut manifest);
        *bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        write_raw_entries(path, &entries);
    }

    fn rewrite_entry_and_checksum(path: &Path, target: &str, mutate: impl FnOnce(&mut Vec<u8>)) {
        let mut entries = read_raw_entries(path);
        let (_, target_bytes) = entries.iter_mut().find(|(name, _)| name == target).unwrap();
        mutate(target_bytes);
        let checksum = sha256(target_bytes);
        let (_, manifest_bytes) = entries
            .iter_mut()
            .find(|(name, _)| name == MANIFEST_ENTRY_NAME)
            .unwrap();
        let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
        manifest["checksums"][target] = Value::String(checksum);
        *manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        write_raw_entries(path, &entries);
    }

    #[test]
    fn strict_read_preserves_document_and_resident_binds_v2_and_legacy_datasets() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("strict-v6.qpls");
        let (document, datasets) = fixture();
        let expected_document = serialize_project_document_v6(&document).unwrap();
        write_archive(&path, &document, &datasets);

        assert!(matches!(
            crate::load_project(&path),
            Err(ProjectError::Invalid(message))
                if message.contains("dedicated strict schema-6 ZIP reader")
        ));

        let loaded = load_project_archive_v6(&path).unwrap();

        assert_eq!(loaded.manifest.schema_version, 6);
        assert_eq!(loaded.manifest.project_id, document.project_id);
        assert_eq!(
            serialize_project_document_v6(&loaded.document).unwrap(),
            expected_document
        );
        assert_eq!(loaded.datasets.len(), 2);
        for (actual, expected) in loaded.datasets.iter().zip(&datasets) {
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.fingerprint, expected.fingerprint);
            assert_eq!(actual.batch, expected.batch);
        }
        assert!(matches!(
            &loaded.document.models[0].payload,
            ProjectModelPayloadV6::SemModelV4Draft { .. }
        ));
        assert_eq!(loaded.document.historical_recipes.len(), 1);
        assert_eq!(loaded.document.historical_results.len(), 1);
        assert_eq!(loaded.document.canonical_result_documents.len(), 1);
        assert_eq!(loaded.document.layouts, document.layouts);
    }

    #[test]
    fn manifest_shape_schema_and_document_identity_are_strict() {
        let directory = tempfile::tempdir().unwrap();
        let (document, datasets) = fixture();

        for (name, mutate, expected) in [
            (
                "unknown-field.qpls",
                (|manifest: &mut Value| manifest["unexpected"] = Value::Bool(true))
                    as fn(&mut Value),
                "field set mismatch",
            ),
            (
                "wrong-schema.qpls",
                (|manifest: &mut Value| manifest["schema_version"] = json!(5)) as fn(&mut Value),
                "requires manifest schema version 6",
            ),
            (
                "wrong-name.qpls",
                (|manifest: &mut Value| manifest["name"] = json!("Different")) as fn(&mut Value),
                "differ from project.json",
            ),
            (
                "wrong-algorithm.qpls",
                (|manifest: &mut Value| manifest["checksum_algorithm"] = json!("sha512"))
                    as fn(&mut Value),
                "unsupported archive checksum algorithm",
            ),
        ] {
            let path = directory.path().join(name);
            write_archive(&path, &document, &datasets);
            rewrite_manifest(&path, mutate);
            let error = load_project_archive_v6(&path).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "unexpected error for {name}: {error}"
            );
        }
    }

    #[test]
    fn checksum_entry_set_and_duplicate_zip_names_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let (document, datasets) = fixture();

        let checksum_path = directory.path().join("checksum.qpls");
        write_archive(&checksum_path, &document, &datasets);
        let mut entries = read_raw_entries(&checksum_path);
        entries
            .iter_mut()
            .find(|(name, _)| name == PROJECT_ENTRY_NAME)
            .unwrap()
            .1
            .push(b' ');
        write_raw_entries(&checksum_path, &entries);
        assert!(matches!(
            load_project_archive_v6(&checksum_path),
            Err(ProjectError::ChecksumMismatch(name)) if name == PROJECT_ENTRY_NAME
        ));

        let unexpected_path = directory.path().join("unexpected-entry.qpls");
        write_archive(&unexpected_path, &document, &datasets);
        let mut entries = read_raw_entries(&unexpected_path);
        let extra = b"unexpected".to_vec();
        let (_, manifest_bytes) = entries
            .iter_mut()
            .find(|(name, _)| name == MANIFEST_ENTRY_NAME)
            .unwrap();
        let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
        manifest["checksums"]["extra.bin"] = Value::String(sha256(&extra));
        *manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        entries.push(("extra.bin".into(), extra));
        write_raw_entries(&unexpected_path, &entries);
        assert!(
            load_project_archive_v6(&unexpected_path)
                .unwrap_err()
                .to_string()
                .contains("entry set mismatch")
        );

        let missing_path = directory.path().join("missing-entry.qpls");
        write_archive(&missing_path, &document, &datasets);
        let missing_name = format!("data/{}.arrow", datasets[0].id);
        let mut entries = read_raw_entries(&missing_path);
        entries.retain(|(name, _)| name != &missing_name);
        let (_, manifest_bytes) = entries
            .iter_mut()
            .find(|(name, _)| name == MANIFEST_ENTRY_NAME)
            .unwrap();
        let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
        manifest["checksums"]
            .as_object_mut()
            .unwrap()
            .remove(&missing_name);
        *manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        write_raw_entries(&missing_path, &entries);
        assert!(
            load_project_archive_v6(&missing_path)
                .unwrap_err()
                .to_string()
                .contains("entry set mismatch")
        );

        let duplicate_path = directory.path().join("duplicate-entry.qpls");
        write_archive(&duplicate_path, &document, &datasets);
        let mut entries = read_raw_entries(&duplicate_path);
        let shadow_entry_name = "shadow.entry";
        entries.push((shadow_entry_name.into(), b"duplicate name probe".to_vec()));
        write_raw_entries(&duplicate_path, &entries);
        rewrite_central_directory_entry_name(
            &duplicate_path,
            shadow_entry_name,
            PROJECT_ENTRY_NAME,
        );
        assert!(
            load_project_archive_v6(&duplicate_path)
                .unwrap_err()
                .to_string()
                .contains("duplicate ZIP entry name")
        );
    }

    #[test]
    fn duplicate_manifest_and_project_json_keys_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let (document, datasets) = fixture();

        let manifest_path = directory.path().join("duplicate-manifest-key.qpls");
        write_archive(&manifest_path, &document, &datasets);
        let mut entries = read_raw_entries(&manifest_path);
        let (_, manifest_bytes) = entries
            .iter_mut()
            .find(|(name, _)| name == MANIFEST_ENTRY_NAME)
            .unwrap();
        let manifest_text = String::from_utf8(manifest_bytes.clone()).unwrap();
        *manifest_bytes = manifest_text
            .replacen("\"name\":", "\"name\": \"duplicate\", \"name\":", 1)
            .into_bytes();
        write_raw_entries(&manifest_path, &entries);
        assert!(
            load_project_archive_v6(&manifest_path)
                .unwrap_err()
                .to_string()
                .contains("duplicate JSON object key")
        );

        let project_path = directory.path().join("duplicate-project-key.qpls");
        write_archive(&project_path, &document, &datasets);
        rewrite_entry_and_checksum(&project_path, PROJECT_ENTRY_NAME, |bytes| {
            let project_text = String::from_utf8(bytes.clone()).unwrap();
            *bytes = project_text
                .replacen(
                    "\"schema_version\":6",
                    "\"schema_version\":6,\"schema_version\":6",
                    1,
                )
                .into_bytes();
        });
        assert!(
            load_project_archive_v6(&project_path)
                .unwrap_err()
                .to_string()
                .contains("duplicate JSON object key")
        );
    }

    #[test]
    fn rechecksummed_draft_and_arrow_tampering_fail_semantic_validation() {
        let directory = tempfile::tempdir().unwrap();
        let (document, datasets) = fixture();

        let document_path = directory.path().join("tampered-draft.qpls");
        write_archive(&document_path, &document, &datasets);
        rewrite_entry_and_checksum(&document_path, PROJECT_ENTRY_NAME, |bytes| {
            let mut value: Value = serde_json::from_slice(bytes).unwrap();
            value["models"][0]["payload"]["model"]["name"] = json!("Tampered draft");
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        assert!(
            load_project_archive_v6(&document_path)
                .unwrap_err()
                .to_string()
                .contains("mismatched identity or digest")
        );

        let arrow_path = directory.path().join("tampered-arrow.qpls");
        write_archive(&arrow_path, &document, &datasets);
        let entry_name = format!("data/{}.arrow", datasets[0].id);
        let replacement_arrow = write_arrow(&datasets[1].batch).unwrap();
        rewrite_entry_and_checksum(&arrow_path, &entry_name, |bytes| {
            *bytes = replacement_arrow;
        });
        let error = load_project_archive_v6(&arrow_path).unwrap_err();
        assert!(
            matches!(&error, ProjectError::Data(_)),
            "unexpected Arrow tamper error: {error}"
        );
    }

    #[test]
    fn embedded_schema_drift_fails_after_checksum_recomputation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("embedded-schema.qpls");
        let (document, datasets) = fixture();
        write_archive(&path, &document, &datasets);
        rewrite_entry_and_checksum(&path, PROJECT_ENTRY_NAME, |bytes| {
            let mut value: Value = serde_json::from_slice(bytes).unwrap();
            value["schema_version"] = json!(5);
            *bytes = serde_json::to_vec(&value).unwrap();
        });
        assert!(
            load_project_archive_v6(&path)
                .unwrap_err()
                .to_string()
                .contains("requires schema_version 6")
        );
    }

    #[test]
    fn v5_archive_still_uses_the_live_loader_and_is_not_accepted_as_v6() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("live-v5.qpls");
        let project = Project::new("Live v5 compatibility");
        crate::save_project(&path, &project).unwrap();

        let live = crate::load_project(&path).unwrap();
        assert_eq!(live.source_archive_version, crate::PROJECT_ARCHIVE_VERSION);
        assert_eq!(live.manifest.project_id, project.manifest.project_id);
        assert!(
            load_project_archive_v6(&path)
                .unwrap_err()
                .to_string()
                .contains("requires manifest schema version 6")
        );
    }
}
