use chrono::{TimeZone, Utc};
use qpls_core::{
    AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode, ModelSpec, StructuralPath,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_estimation::PlsModelComparisonConfigV1;
use qpls_project::{
    PROJECT_ARCHIVE_VERSION, Project, ProjectArchiveUpgradeRequestV6,
    append_canonical_result_document_v2_file_v6, plan_project_upgrade_to_v6,
    read_project_document_v6, write_project_document_v6_new,
};
use qpls_runner::{
    InternalLabsPlsModelComparisonRequestV1, PlsModelComparisonRunContextV1,
    build_pls_model_comparison_canonical_result_v2, comparison_capability_cell,
    run_internal_labs_pls_model_comparison_v1,
};
use std::collections::BTreeMap;
use uuid::Uuid;

fn comparison_fixture() -> (qpls_data::Dataset, InternalLabsPlsModelComparisonRequestV1) {
    let mut csv = String::from("x1,x2,z1,z2,y1,y2\n");
    for row in 0..36 {
        let t = row as f64 / 5.5;
        let x = (t * 0.75).sin() + row as f64 * 0.012;
        let z = (t * 1.05).cos() - row as f64 * 0.008;
        let noise = ((row * 13 % 11) as f64 - 5.0) * 0.014;
        let y = 0.61 * x + 0.55 * z + noise;
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            x + noise * 0.15,
            x * 0.95 - noise * 0.12,
            z - noise * 0.1,
            z * 1.02 + noise * 0.16,
            y + noise * 0.2,
            y * 0.97 - noise * 0.18
        ));
    }
    let bytes = csv.into_bytes();
    let dataset = import_delimited_bytes(
        &bytes,
        "schema6-pls-model-comparison.csv",
        b',',
        &ImportOptions::default(),
    )
    .unwrap();
    let constructs = vec![
        Construct {
            id: "x".into(),
            name: "X".into(),
            short_name: "X".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["x1".into(), "x2".into()],
        },
        Construct {
            id: "z".into(),
            name: "Z".into(),
            short_name: "Z".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["z1".into(), "z2".into()],
        },
        Construct {
            id: "y".into(),
            name: "Y".into(),
            short_name: "Y".into(),
            mode: MeasurementMode::Reflective,
            indicators: vec!["y1".into(), "y2".into()],
        },
    ];
    let model = |name: &str, include_z: bool| ModelSpec {
        id: Uuid::new_v4(),
        name: name.into(),
        constructs: if include_z {
            constructs.clone()
        } else {
            constructs
                .iter()
                .filter(|construct| construct.id != "z")
                .cloned()
                .collect()
        },
        paths: if include_z {
            vec![
                StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                },
                StructuralPath {
                    source: "z".into(),
                    target: "y".into(),
                },
            ]
        } else {
            vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }]
        },
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let settings = AnalysisSettings {
        workers: 1,
        ..AnalysisSettings::default()
    };
    let mut established =
        AnalysisRecipe::new(&bytes, model("Established", false), settings.clone());
    let mut alternative = AnalysisRecipe::new(&bytes, model("Alternative", true), settings);
    established.dataset_fingerprint = dataset.fingerprint.0.clone();
    alternative.dataset_fingerprint = dataset.fingerprint.0.clone();
    let request = InternalLabsPlsModelComparisonRequestV1::exact_internal_labs(
        &dataset,
        established,
        alternative,
        PlsModelComparisonConfigV1 {
            folds: 3,
            repeats: 1,
            seed: 9901,
            confidence_level: 0.95,
        },
    );
    (dataset, request)
}

#[test]
fn genuine_comparison_canonical_document_appends_atomically_and_reopens_exactly_in_schema6() {
    assert_eq!(PROJECT_ARCHIVE_VERSION, 5);
    let (dataset, request) = comparison_fixture();
    let mut source_project = Project::new("Schema-6 comparison integration");
    source_project.datasets.push(dataset.clone());
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("unchanged-source-v5.qpls");
    let destination = directory.path().join("comparison-v6.json");
    let upgrade = ProjectArchiveUpgradeRequestV6 {
        source_archive_sha256: "a".repeat(64),
        source_archive_path: source_path.to_string_lossy().into_owned(),
        destination_archive_path: destination.to_string_lossy().into_owned(),
        upgraded_at: Utc.timestamp_opt(1_786_752_000, 0).unwrap(),
        legacy_display_covariances: BTreeMap::new(),
    };
    let plan = plan_project_upgrade_to_v6(&source_project, &upgrade).unwrap();
    let initial_write = write_project_document_v6_new(&destination, &plan.document).unwrap();

    let execution =
        run_internal_labs_pls_model_comparison_v1(&dataset, &request, || false, |_| {}).unwrap();
    let core_document = build_pls_model_comparison_canonical_result_v2(
        &PlsModelComparisonRunContextV1 {
            run_id: Uuid::from_u128(701),
            project_id: plan.document.project_id,
            started_at: "2026-08-15T00:00:00Z".into(),
            completed_at: "2026-08-15T00:00:02Z".into(),
        },
        &execution,
    )
    .unwrap();
    let archive_document: qpls_project::CanonicalResultDocumentV2 =
        serde_json::from_value(serde_json::to_value(&core_document).unwrap()).unwrap();
    archive_document.ensure_valid().unwrap();

    let append = append_canonical_result_document_v2_file_v6(
        &destination,
        &initial_write.document_sha256,
        archive_document.clone(),
    )
    .unwrap();
    assert_eq!(append.canonical_document_id, core_document.document_id);
    assert_eq!(append.run_id, core_document.provenance.run_id);
    assert_ne!(
        append.updated_document_sha256,
        initial_write.document_sha256
    );

    let reopened = read_project_document_v6(&destination).unwrap();
    assert_eq!(reopened.schema_version, 6);
    assert!(reopened.datasets.iter().any(|resident| {
        resident.id == dataset.id && resident.fingerprint == dataset.fingerprint
    }));
    assert_eq!(reopened.canonical_result_documents.len(), 1);
    let attachment = &reopened.canonical_result_documents[0];
    assert!(attachment.immutable());
    assert_eq!(attachment.canonical_document(), &archive_document);
    assert_eq!(
        serde_json::to_value(attachment.canonical_document()).unwrap(),
        serde_json::to_value(&core_document).unwrap()
    );
    assert!(attachment.canonical_document().tables.iter().all(|table| {
        table.capability_cells.as_ref()
            == Some(&vec![
                serde_json::from_value(serde_json::to_value(comparison_capability_cell()).unwrap())
                    .unwrap(),
            ])
    }));

    // The live source Project and schema-5 archive contract are not mutated by
    // this schema-6-only append/reopen path.
    assert!(source_project.results.is_empty());
    assert!(source_project.recipes.is_empty());
    assert_eq!(source_project.datasets.len(), 1);
    assert_eq!(source_project.datasets[0].id, dataset.id);
    assert_eq!(source_project.datasets[0].fingerprint, dataset.fingerprint);
    assert!(!source_path.exists());
}
