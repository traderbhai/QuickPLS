use crate::recipe_v4_cbsem_canonical_result::{
    build_recipe_v4_cbsem_canonical_result, validate_archived_recipe_v4_cbsem_method_identity,
};
use crate::recipe_v4_cbsem_execution::{
    InternalRecipeV4CbsemExecutionRequestV1, execute_internal_recipe_v4_cbsem,
    resolve_internal_recipe_v4_cbsem_dataset,
};
use chrono::{TimeZone, Utc};
use qpls_core::{AnalysisRecipeModelBindingV4, compile_analysis_recipe_v4};
use qpls_data::{DataKind, ImportOptions, import_delimited_bytes};
use qpls_project::{
    Project, ProjectArchiveUpgradeRequestV6, attach_canonical_result_document_v2_v6,
    canonical_result_document_v2_json, deserialize_project_document_v6, plan_project_upgrade_to_v6,
    serialize_project_document_v6,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

const FIXTURE_JSON: &str =
    include_str!("../../validation/fixtures/wave1_diagram_cbsem_roundtrip_v1.json");
const DATASET_CSV: &str = include_str!("../../validation/fixtures/corporate_reputation.csv");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureImportOptions {
    data_kind: String,
    sample_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureDataset {
    source: String,
    csv_text: String,
    import_options: FixtureImportOptions,
    id: String,
    name: String,
    fingerprint: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wave1Fixture {
    fixture_schema_version: u32,
    fixture_kind: String,
    dataset: FixtureDataset,
    project_id: String,
    job_id: String,
    expected_request: InternalRecipeV4CbsemExecutionRequestV1,
}

#[test]
fn diagram_recipe_cbsem_result_roundtrips_through_standalone_schema6_document_exactly() {
    let fixture: Wave1Fixture = serde_json::from_str(FIXTURE_JSON).unwrap();
    assert_eq!(fixture.fixture_schema_version, 1);
    assert_eq!(
        fixture.fixture_kind,
        "wave1_diagram_cbsem_standalone_schema6_roundtrip"
    );
    assert_eq!(
        fixture.dataset.source,
        "validation/fixtures/corporate_reputation.csv"
    );
    assert_eq!(fixture.dataset.import_options.data_kind, "raw");
    assert_eq!(fixture.dataset.import_options.sample_size, None);
    assert_eq!(
        fixture.dataset.csv_text.replace("\r\n", "\n"),
        DATASET_CSV.replace("\r\n", "\n")
    );

    let mut dataset = import_delimited_bytes(
        fixture.dataset.csv_text.as_bytes(),
        &fixture.dataset.name,
        b',',
        &ImportOptions {
            data_kind: DataKind::Raw,
            sample_size: None,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(dataset.fingerprint.0, fixture.dataset.fingerprint);
    dataset.id = Uuid::parse_str(&fixture.dataset.id).unwrap();
    dataset.name = fixture.dataset.name.clone();

    let request = fixture.expected_request;
    assert_eq!(request.dataset_id, dataset.id.to_string());
    assert_eq!(request.dataset_fingerprint, dataset.fingerprint.0);
    request.model.ensure_valid().unwrap();
    let AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        model,
        scientific_sha256,
    } = &request.recipe.model_binding
    else {
        panic!("fixture recipe must embed the diagram-origin SemModelV4");
    };
    assert_eq!(model, &request.model);
    assert_eq!(
        request.model.scientific_sha256().unwrap(),
        *scientific_sha256
    );

    let _compiled = compile_analysis_recipe_v4(
        &request.recipe,
        Some(&request.model),
        request.compiler_target,
        request.capability_cell.clone(),
    )
    .unwrap();

    let project_id = Uuid::parse_str(&fixture.project_id).unwrap();
    let mut project = Project::new("Wave-1 diagram CB-SEM roundtrip");
    project.manifest.project_id = project_id;
    project.datasets.push(dataset);
    let resident = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
    let analytical = execute_internal_recipe_v4_cbsem(&resident, &request).unwrap();
    assert!(analytical.estimation().analysis.converged);

    let job_id = Uuid::parse_str(&fixture.job_id).unwrap();
    let canonical = build_recipe_v4_cbsem_canonical_result(
        job_id,
        project_id,
        "2026-08-15T12:00:01.000Z",
        "2026-08-15T12:00:02.000Z",
        &request,
        &analytical,
    )
    .unwrap();
    assert_eq!(
        canonical.provenance.recipe_id,
        request.recipe.id.to_string()
    );
    assert_eq!(canonical.provenance.model_id, request.model.id);
    assert_eq!(canonical.provenance.dataset_id, request.dataset_id);
    assert!(
        canonical
            .tables
            .iter()
            .any(|table| table.id == "parameters")
    );

    let archived = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
        serde_json::to_value(&canonical).unwrap(),
    )
    .unwrap();
    validate_archived_recipe_v4_cbsem_method_identity(&archived).unwrap();
    let expected_document_json = canonical_result_document_v2_json(&archived).unwrap();
    let expected_tables = serde_json::to_value(&archived.tables).unwrap();

    let mut plan = plan_project_upgrade_to_v6(
        &project,
        &ProjectArchiveUpgradeRequestV6 {
            source_archive_sha256: "a".repeat(64),
            source_archive_path: r"D:\wave1-diagram-source.qpls".into(),
            destination_archive_path: r"D:\wave1-diagram-schema6.qpls".into(),
            upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 12, 0, 3).unwrap(),
            legacy_display_covariances: BTreeMap::new(),
        },
    )
    .unwrap();
    plan.document.recipes.push(request.recipe.clone());
    plan.document.ensure_valid().unwrap();
    let attached = attach_canonical_result_document_v2_v6(&plan.document, archived).unwrap();

    // This gate intentionally proves the standalone schema-6 JSON document
    // contract. It does not claim a `.qpls` ZIP/Arrow save roundtrip.
    let bytes = serialize_project_document_v6(&attached).unwrap();
    let reopened = deserialize_project_document_v6(&bytes).unwrap();
    assert_eq!(serialize_project_document_v6(&reopened).unwrap(), bytes);
    assert_eq!(reopened.canonical_result_documents.len(), 1);
    let reopened_result = &reopened.canonical_result_documents[0];
    assert_eq!(reopened_result.run_id(), job_id.to_string());
    assert_eq!(
        canonical_result_document_v2_json(reopened_result.canonical_document()).unwrap(),
        expected_document_json
    );
    assert_eq!(
        serde_json::to_value(&reopened_result.canonical_document().tables).unwrap(),
        expected_tables
    );
}
