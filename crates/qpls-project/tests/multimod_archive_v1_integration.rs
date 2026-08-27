use arrow::{
    array::{Array, ArrayRef, DictionaryArray, Float64Array, StringArray, UInt8Array, UInt32Array},
    datatypes::{DataType, Field, Schema, UInt32Type},
    ipc::{reader::StreamReader, writer::StreamWriter},
    record_batch::RecordBatch,
};
use chrono::{TimeZone, Utc};
use qpls_core::{
    CapabilityCellReferenceV2, CausalPositivityDiagnosticV1,
    INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION, InterventionalEffectResultV1,
    InterventionalMediationResultV1, MULTIMOD_SIDECAR_MAX_BYTES_V1, MultiModAnalysisResultV1,
    MultimodProvenanceV1, MultimodQualificationStateV1, MultimodReplicateLedgerSummaryV1,
    MultimodResultSidecarDescriptorV1,
};
use qpls_project::{
    MULTIMOD_ARROW_SIDECAR_MEDIA_TYPE_V1, MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
    MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1, MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1,
    MultiModArchiveErrorV1, MultiModResultAttachmentV1, MultiModSidecarPayloadV1,
    ProjectArchiveDocumentV6, deserialize_project_document_v6, encode_multimod_arrow_sidecar_v1,
    multimod_arrow_schema_sha256_v1, multimod_bca_jackknife_summary_batch_v1,
    multimod_membership_with_row_tokens_batch_v1, multimod_micom_null_statistics_batch_v1,
    multimod_resample_ledger_batch_v1, multimod_result_identity_sha256_v1,
    multimod_start_trace_batch_v1, multimod_target_ledger_batch_v1, serialize_project_document_v6,
    validate_multimod_sidecar_payload_v1, validate_multimod_sidecar_stream_v1,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
    sync::Arc,
};
use uuid::Uuid;

const RESULT_ID: &str = "result:multimod-causal-1";

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn labs_provenance(recipe_id: Uuid) -> MultimodProvenanceV1 {
    MultimodProvenanceV1 {
        method_version: "interventional_causal_mediation_v1".into(),
        recipe_id: recipe_id.to_string(),
        recipe_analytical_sha256: "a".repeat(64),
        config_sha256: "b".repeat(64),
        model_id: "00000000-0000-0000-0000-000000000203".into(),
        model_scientific_sha256: "c".repeat(64),
        dataset_id: "00000000-0000-0000-0000-000000000204".into(),
        dataset_fingerprint: "dataset-fingerprint".into(),
        engine_version: "2.56.0".into(),
        seed: 42,
        capability_cell: CapabilityCellReferenceV2 {
            registry_schema_version: 2,
            capability_id: "quickpls.multimod".into(),
            cell_id: "qpls.multimod.interventional_causal_mediation_v1".into(),
            capability_version: "interventional_causal_mediation_v1".into(),
        },
        qualification: MultimodQualificationStateV1::UnqualifiedLabs,
        candidate_qualification_receipt: None,
    }
}

fn scientific_result(
    provenance: MultimodProvenanceV1,
    sidecars: Vec<MultimodResultSidecarDescriptorV1>,
) -> MultiModAnalysisResultV1 {
    MultiModAnalysisResultV1::InterventionalMediationResultV1(InterventionalMediationResultV1 {
        schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
        provenance,
        interpretation_label: "assumption-dependent interventional estimate".into(),
        identification_assumptions: vec!["No unmeasured confounding".into()],
        positivity: vec![CausalPositivityDiagnosticV1 {
            variable_id: "treatment".into(),
            observed_minimum: 0.0,
            observed_maximum: 1.0,
            requested_value: 1.0,
            support_count: 50,
            minimum_required_count: 10,
            support_rule: "binary_arm_count".into(),
            supported: true,
        }],
        effects: vec![InterventionalEffectResultV1 {
            target_id: "effect:path-treatment-mediator-outcome".into(),
            path_id: "path:treatment-mediator-outcome".into(),
            estimand: "interventional_indirect".into(),
            estimate: 0.25,
            p_value: None,
            interval: None,
        }],
        replicate_ledger: MultimodReplicateLedgerSummaryV1 {
            requested: 500,
            usable: 500,
            minimum_required: 450,
            usable_fraction: 1.0,
            complete: true,
            ledger_sha256: "d".repeat(64),
            failure_counts: BTreeMap::new(),
            failures: Vec::new(),
        },
        sidecars,
    })
}

fn target_ledger_batch() -> RecordBatch {
    multimod_target_ledger_batch_v1(
        vec![0, 1],
        vec![
            "effect:path-treatment-mediator-outcome".into(),
            "effect:path-treatment-mediator-outcome".into(),
        ],
        vec![0.24, 0.26],
        vec![true, true],
        vec![String::new(), String::new()],
    )
    .unwrap()
}

fn micom_null_statistics_batch() -> RecordBatch {
    multimod_micom_null_statistics_batch_v1(
        vec![0, 0, 0, 0, 1],
        vec![0, 1, 0, 0, 0],
        vec![
            MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
            MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
            MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1,
            MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1,
            MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
        ],
        vec![0.99, 0.98, 0.01, -0.02, 0.97],
    )
    .unwrap()
}

fn attachment_and_payload(
    recipe_id: Uuid,
    provenance: MultimodProvenanceV1,
) -> (MultiModResultAttachmentV1, MultiModSidecarPayloadV1) {
    let result_without_sidecars = scientific_result(provenance.clone(), Vec::new());
    let identity_sha256 = multimod_result_identity_sha256_v1(&result_without_sidecars).unwrap();
    let payload = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &identity_sha256,
        "interventional-bootstrap:target-vectors",
        &target_ledger_batch(),
    )
    .unwrap();
    let result = scientific_result(provenance, vec![payload.descriptor.clone()]);
    let attachment = MultiModResultAttachmentV1::new(
        RESULT_ID,
        recipe_id,
        result,
        vec![payload.descriptor.clone()],
    )
    .unwrap();
    (attachment, payload)
}

#[test]
fn arrow_sidecar_encoding_round_trips_with_stable_descriptor_shape() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0001);
    let (_, payload) = attachment_and_payload(recipe_id, labs_provenance(recipe_id));

    assert_eq!(
        payload.descriptor.media_type,
        MULTIMOD_ARROW_SIDECAR_MEDIA_TYPE_V1
    );
    assert_eq!(payload.descriptor.row_count, 2);
    assert_eq!(payload.descriptor.column_count, 5);
    assert_eq!(
        payload.descriptor.uncompressed_bytes,
        payload.bytes.len() as u64
    );
    assert_eq!(payload.descriptor.sha256, sha256_bytes(&payload.bytes));
    validate_multimod_sidecar_payload_v1(RESULT_ID, &payload).unwrap();

    let mut reader = StreamReader::try_new(Cursor::new(payload.bytes.as_slice()), None).unwrap();
    assert_eq!(
        payload.descriptor.arrow_schema_sha256,
        multimod_arrow_schema_sha256_v1(reader.schema().as_ref()).unwrap()
    );
    let reopened = reader.next().unwrap().unwrap();
    assert_eq!(reopened.num_rows(), 2);
    assert_eq!(reopened.num_columns(), 5);
    let target_ids = reopened
        .column(1)
        .as_any()
        .downcast_ref::<DictionaryArray<UInt32Type>>()
        .expect("trusted target IDs are UInt32-keyed dictionaries");
    assert_eq!(target_ids.values().len(), 1);
    assert_eq!(
        target_ids
            .downcast_dict::<StringArray>()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![
            Some("effect:path-treatment-mediator-outcome"),
            Some("effect:path-treatment-mediator-outcome")
        ]
    );
    assert!(
        reopened
            .column(4)
            .as_any()
            .downcast_ref::<DictionaryArray<UInt32Type>>()
            .is_some()
    );
    assert!(reader.next().is_none());
}

#[test]
fn arrow_sidecar_registry_rejects_an_unknown_table_role_for_a_known_evidence_kind() {
    let error = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-unknown-table.arrow",
        &"a".repeat(64),
        "interventional-bootstrap:unknown-table",
        &target_ledger_batch(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        MultiModArchiveErrorV1::InvalidSidecarEntry(_)
    ));
}

#[test]
fn arrow_sidecar_registry_rejects_a_self_consistent_but_wrong_schema_for_a_known_role() {
    let wrong_schema = Arc::new(Schema::new(vec![Field::new(
        "payload",
        DataType::Utf8,
        false,
    )]));
    let wrong_batch = RecordBatch::try_new(
        wrong_schema,
        vec![Arc::new(StringArray::from(vec!["self-consistent"]))],
    )
    .unwrap();
    let error = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &"a".repeat(64),
        "interventional-bootstrap:target-vectors",
        &wrong_batch,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        MultiModArchiveErrorV1::SidecarSchemaMismatch(_)
    ));
}

#[test]
fn mga_null_audit_tables_are_bound_to_the_trusted_target_ledger_schema() {
    for (file_name, evidence_role) in [
        (
            "mga-pairwise-permutation-null-target-vectors.arrow",
            "mga-pairwise-permutation:null-target-vectors",
        ),
        (
            "mga-omnibus-permutation-null-target-vectors.arrow",
            "mga-omnibus-permutation:null-target-vectors",
        ),
        (
            "heterogeneity-pos-common-metric-micom-00-null-statistics.arrow",
            "heterogeneity-pos-common-metric:micom-00-null-statistics",
        ),
    ] {
        let payload = encode_multimod_arrow_sidecar_v1(
            RESULT_ID,
            file_name,
            &"a".repeat(64),
            evidence_role,
            &target_ledger_batch(),
        )
        .unwrap();
        assert_eq!(payload.descriptor.row_count, 2);
        assert_eq!(payload.descriptor.column_count, 5);
        validate_multimod_sidecar_payload_v1(RESULT_ID, &payload).unwrap();
    }

    let micom = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "mga-micom-pair-null-statistics.arrow",
        &"a".repeat(64),
        "mga-micom-pair:null-statistics",
        &micom_null_statistics_batch(),
    )
    .unwrap();
    assert_eq!(micom.descriptor.row_count, 5);
    assert_eq!(micom.descriptor.column_count, 4);
    validate_multimod_sidecar_payload_v1(RESULT_ID, &micom).unwrap();

    let mut reader = StreamReader::try_new(Cursor::new(micom.bytes.as_slice()), None).unwrap();
    let reopened = reader.next().unwrap().unwrap();
    let constructs = reopened
        .column(1)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .unwrap();
    let kinds = reopened
        .column(2)
        .as_any()
        .downcast_ref::<UInt8Array>()
        .unwrap();
    let values = reopened
        .column(3)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(
        (constructs.value(2), kinds.value(2), values.value(2)),
        (0, 1, 0.01)
    );
    assert!(reader.next().is_none());

    assert!(matches!(
        encode_multimod_arrow_sidecar_v1(
            RESULT_ID,
            "mga-micom-pair-wrong-null-statistics.arrow",
            &"a".repeat(64),
            "mga-micom-pair:null-statistics",
            &target_ledger_batch(),
        ),
        Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(_))
    ));
}

#[test]
fn target_ledger_rejects_nonfinite_valid_values_and_incoherent_failure_codes() {
    for (value, valid, failure_code) in [
        (f64::NAN, true, String::new()),
        (0.25, true, "unexpected_failure".to_string()),
        (0.0, false, String::new()),
    ] {
        assert!(matches!(
            multimod_target_ledger_batch_v1(
                vec![0],
                vec!["target:stable".into()],
                vec![value],
                vec![valid],
                vec![failure_code],
            ),
            Err(MultiModArchiveErrorV1::SidecarShapeMismatch(_))
        ));
    }
}

#[test]
fn micom_null_statistics_reject_invalid_ordinals_kinds_and_values() {
    for (replicates, constructs, kinds, values) in [
        (vec![0, 0], vec![0, 0], vec![0, 0], vec![0.9, 0.8]),
        (vec![0], vec![0], vec![3], vec![0.9]),
        (vec![0], vec![0], vec![0], vec![f64::NAN]),
    ] {
        assert!(matches!(
            multimod_micom_null_statistics_batch_v1(replicates, constructs, kinds, values,),
            Err(MultiModArchiveErrorV1::SidecarShapeMismatch(_))
        ));
    }
}

#[test]
fn empty_arrow_stream_cannot_hide_a_descriptor_column_count_mismatch() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0004);
    let (_, payload) = attachment_and_payload(recipe_id, labs_provenance(recipe_id));
    let reader = StreamReader::try_new(Cursor::new(payload.bytes.as_slice()), None).unwrap();
    let mut empty_stream = Vec::new();
    {
        let mut writer =
            StreamWriter::try_new(&mut empty_stream, reader.schema().as_ref()).unwrap();
        writer.finish().unwrap();
    }
    let mut descriptor = payload.descriptor;
    descriptor.column_count -= 1;
    assert!(matches!(
        validate_multimod_sidecar_stream_v1(
            RESULT_ID,
            &descriptor,
            Cursor::new(empty_stream.as_slice()),
        ),
        Err(MultiModArchiveErrorV1::SidecarShapeMismatch(_))
    ));
}

#[test]
fn scientific_identity_binds_estimates_and_provenance_but_not_its_sidecar_inventory() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0003);
    let provenance = labs_provenance(recipe_id);
    let baseline = scientific_result(provenance, Vec::new());
    let baseline_identity = multimod_result_identity_sha256_v1(&baseline).unwrap();

    let mut changed_result = baseline.clone();
    let MultiModAnalysisResultV1::InterventionalMediationResultV1(changed) = &mut changed_result
    else {
        unreachable!()
    };
    changed.effects[0].estimate += 0.01;
    assert_ne!(
        multimod_result_identity_sha256_v1(&changed_result).unwrap(),
        baseline_identity
    );

    let payload = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &baseline_identity,
        "interventional-bootstrap:target-vectors",
        &target_ledger_batch(),
    )
    .unwrap();
    let mut with_sidecar_result = baseline;
    let MultiModAnalysisResultV1::InterventionalMediationResultV1(with_sidecar) =
        &mut with_sidecar_result
    else {
        unreachable!()
    };
    with_sidecar.sidecars.push(payload.descriptor);
    assert_eq!(
        multimod_result_identity_sha256_v1(&with_sidecar_result).unwrap(),
        baseline_identity
    );
}

#[test]
fn attachment_rejects_an_aggregate_sidecar_set_above_the_per_run_cap() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0002);
    let provenance = labs_provenance(recipe_id);
    let result_without_sidecars = scientific_result(provenance.clone(), Vec::new());
    let identity_sha256 = multimod_result_identity_sha256_v1(&result_without_sidecars).unwrap();
    let template = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &identity_sha256,
        "interventional-bootstrap:target-vectors",
        &target_ledger_batch(),
    )
    .unwrap()
    .descriptor;
    let mut first = template.clone();
    first.entry_name =
        format!("results/{RESULT_ID}/interventional-bootstrap-a-target-vectors.arrow");
    first.uncompressed_bytes = MULTIMOD_SIDECAR_MAX_BYTES_V1 / 2 + 1;
    let mut second = template;
    second.entry_name =
        format!("results/{RESULT_ID}/interventional-bootstrap-b-target-vectors.arrow");
    second.uncompressed_bytes = MULTIMOD_SIDECAR_MAX_BYTES_V1 / 2 + 1;
    let sidecars = vec![first, second];
    let result = scientific_result(provenance, sidecars.clone());

    assert!(matches!(
        MultiModResultAttachmentV1::new(RESULT_ID, recipe_id, result, sidecars),
        Err(MultiModArchiveErrorV1::InvalidScientificResult(message))
            if message
                == "multimod_result.sidecar_total at result.sidecars: aggregate sidecar evidence exceeds the 512 MiB per-run cap"
    ));
}

#[test]
fn scientific_sidecar_schemas_validate_complete_ledgers_and_posterior_identity() {
    let ledger = multimod_resample_ledger_batch_v1(
        vec![0, 1],
        vec![42, 43],
        vec![true, false],
        vec![String::new(), "rank_deficient".into()],
        vec!["shard-0".into(), "shard-0".into()],
    )
    .unwrap();
    assert_eq!(ledger.num_rows(), 2);

    let trace = multimod_start_trace_batch_v1(
        vec!["fimix_pls_v2".into(), "fimix_pls_v2".into()],
        vec![2, 2],
        vec![0, 0],
        vec![0, 1],
        vec![-20.0, -19.0],
        vec![false, true],
        vec![String::new(), String::new()],
    )
    .unwrap();
    assert_eq!(trace.num_columns(), 7);

    let posterior = multimod_membership_with_row_tokens_batch_v1(
        vec![3, 3, 8, 8],
        vec![
            "row:3".into(),
            "row:3".into(),
            "row:8".into(),
            "row:8".into(),
        ],
        vec![0, 1, 0, 1],
        vec![0.8, 0.2, 0.25, 0.75],
    )
    .unwrap();
    assert_eq!(posterior.num_rows(), 4);
    assert!(
        multimod_membership_with_row_tokens_batch_v1(
            vec![3, 3],
            vec!["row:3".into(), "row:3".into()],
            vec![0, 1],
            vec![0.8, 0.3],
        )
        .is_err()
    );

    let jackknife = multimod_bca_jackknife_summary_batch_v1(
        vec!["target:a".into()],
        vec![50],
        vec![0.25],
        vec![0.01],
        vec![0.4],
        vec![0.006_588_078_458_684_124],
        vec![true],
    )
    .unwrap();
    assert_eq!(jackknife.num_rows(), 1);
}

#[test]
fn sidecar_descriptor_schema_shape_size_and_hash_tampering_fail_closed() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0002);
    let (_, payload) = attachment_and_payload(recipe_id, labs_provenance(recipe_id));

    let mut schema_tampered = payload.clone();
    schema_tampered.descriptor.schema_version += 1;
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &schema_tampered),
        Err(MultiModArchiveErrorV1::InvalidSidecarEntry(_))
    ));

    let mut row_shape_tampered = payload.clone();
    row_shape_tampered.descriptor.row_count += 1;
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &row_shape_tampered),
        Err(MultiModArchiveErrorV1::SidecarShapeMismatch(_))
    ));

    let mut column_shape_tampered = payload.clone();
    column_shape_tampered.descriptor.column_count += 1;
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &column_shape_tampered),
        Err(MultiModArchiveErrorV1::SidecarShapeMismatch(_))
    ));

    let mut schema_identity_tampered = payload.clone();
    schema_identity_tampered.descriptor.arrow_schema_sha256 = "e".repeat(64);
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &schema_identity_tampered),
        Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(_))
    ));

    let mut size_tampered = payload.clone();
    size_tampered.descriptor.uncompressed_bytes += 1;
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &size_tampered),
        Err(MultiModArchiveErrorV1::SidecarSizeMismatch(_))
    ));

    let mut hash_tampered = payload;
    hash_tampered.bytes[0] ^= 0x01;
    assert!(matches!(
        validate_multimod_sidecar_payload_v1(RESULT_ID, &hash_tampered),
        Err(MultiModArchiveErrorV1::SidecarDigestMismatch(_))
    ));
}

#[test]
fn arrow_schema_identity_is_order_stable_and_semantically_sensitive() {
    let field_metadata_a = HashMap::from([
        ("zeta".to_owned(), "last".to_owned()),
        ("alpha".to_owned(), "first".to_owned()),
    ]);
    let field_metadata_b = HashMap::from([
        ("alpha".to_owned(), "first".to_owned()),
        ("zeta".to_owned(), "last".to_owned()),
    ]);
    let schema_metadata_a = HashMap::from([
        ("worker".to_owned(), "one".to_owned()),
        ("contract".to_owned(), "ledger".to_owned()),
    ]);
    let schema_metadata_b = HashMap::from([
        ("contract".to_owned(), "ledger".to_owned()),
        ("worker".to_owned(), "one".to_owned()),
    ]);
    let schema_a = Schema::new_with_metadata(
        vec![Field::new("estimate", DataType::Float64, true).with_metadata(field_metadata_a)],
        schema_metadata_a,
    );
    let schema_b = Schema::new_with_metadata(
        vec![Field::new("estimate", DataType::Float64, true).with_metadata(field_metadata_b)],
        schema_metadata_b,
    );
    let expected = multimod_arrow_schema_sha256_v1(&schema_a).unwrap();
    assert_eq!(
        expected,
        multimod_arrow_schema_sha256_v1(&schema_b).unwrap()
    );

    for changed in [
        Schema::new_with_metadata(
            vec![
                Field::new("renamed", DataType::Float64, true).with_metadata(HashMap::from([
                    ("alpha".to_owned(), "first".to_owned()),
                    ("zeta".to_owned(), "last".to_owned()),
                ])),
            ],
            schema_b.metadata.clone(),
        ),
        Schema::new_with_metadata(
            vec![
                Field::new("estimate", DataType::UInt32, true).with_metadata(HashMap::from([
                    ("alpha".to_owned(), "first".to_owned()),
                    ("zeta".to_owned(), "last".to_owned()),
                ])),
            ],
            schema_b.metadata.clone(),
        ),
        Schema::new_with_metadata(
            vec![
                Field::new("estimate", DataType::Float64, false).with_metadata(HashMap::from([
                    ("alpha".to_owned(), "first".to_owned()),
                    ("zeta".to_owned(), "last".to_owned()),
                ])),
            ],
            schema_b.metadata.clone(),
        ),
        Schema::new_with_metadata(
            vec![
                Field::new("estimate", DataType::Float64, true).with_metadata(HashMap::from([
                    ("alpha".to_owned(), "changed".to_owned()),
                    ("zeta".to_owned(), "last".to_owned()),
                ])),
            ],
            schema_b.metadata.clone(),
        ),
        Schema::new_with_metadata(
            vec![
                Field::new("estimate", DataType::Float64, true).with_metadata(HashMap::from([
                    ("alpha".to_owned(), "first".to_owned()),
                    ("zeta".to_owned(), "last".to_owned()),
                ])),
            ],
            HashMap::from([
                ("contract".to_owned(), "changed".to_owned()),
                ("worker".to_owned(), "one".to_owned()),
            ]),
        ),
    ] {
        assert_ne!(expected, multimod_arrow_schema_sha256_v1(&changed).unwrap());
    }
}

#[test]
fn pre_identity_multimod_v1_descriptor_fails_closed() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0006);
    let (_, payload) = attachment_and_payload(recipe_id, labs_provenance(recipe_id));
    let mut wire = serde_json::to_value(payload.descriptor).unwrap();
    wire.as_object_mut().unwrap().remove("arrow_schema_sha256");

    assert!(serde_json::from_value::<MultimodResultSidecarDescriptorV1>(wire).is_err());
}

#[test]
fn attachment_rejects_a_sidecar_inventory_that_differs_from_the_result() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0003);
    let provenance = labs_provenance(recipe_id);
    let result_without_sidecars = scientific_result(provenance.clone(), Vec::new());
    let identity_sha256 = multimod_result_identity_sha256_v1(&result_without_sidecars).unwrap();
    let payload = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &identity_sha256,
        "interventional-bootstrap:target-vectors",
        &target_ledger_batch(),
    )
    .unwrap();
    let result = scientific_result(provenance, vec![payload.descriptor]);

    assert!(matches!(
        MultiModResultAttachmentV1::new(RESULT_ID, recipe_id, result, Vec::new()),
        Err(MultiModArchiveErrorV1::ResultSidecarInventoryMismatch(id)) if id == RESULT_ID
    ));
}

#[test]
fn v6_documents_without_the_additive_multimod_field_default_to_an_empty_inventory() {
    let document = ProjectArchiveDocumentV6::new_general_sem_v1(
        Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0004),
        "Pre-MultiMod schema V6 fixture",
        Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap(),
    );
    let encoded = serialize_project_document_v6(&document).unwrap();
    let wire: Value = serde_json::from_slice(&encoded).unwrap();

    assert!(wire.get("multimod_results").is_none());
    let reopened = deserialize_project_document_v6(&encoded).unwrap();
    assert!(reopened.multimod_results.is_empty());
    assert_eq!(reopened.project_id, document.project_id);
    assert_eq!(reopened.name, document.name);
    assert_eq!(reopened.schema_version, document.schema_version);
}

#[test]
fn same_shape_arrow_schema_substitution_is_rejected() {
    let recipe_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0005);
    let (_, original) = attachment_and_payload(recipe_id, labs_provenance(recipe_id));
    let alternate_schema = Arc::new(Schema::new(vec![
        Field::new("different_row_token", DataType::UInt32, false),
        Field::new("different_class_token", DataType::UInt8, false),
        Field::new("different_probability", DataType::Float64, false),
        Field::new("different_worker_token", DataType::UInt32, false),
        Field::new("different_validity_token", DataType::UInt8, false),
    ]));
    let alternate_batch = RecordBatch::try_new(
        alternate_schema,
        vec![
            Arc::new(UInt32Array::from(vec![0, 1])) as ArrayRef,
            Arc::new(UInt8Array::from(vec![1, 1])) as ArrayRef,
            Arc::new(Float64Array::from(vec![0.9, 0.8])) as ArrayRef,
            Arc::new(UInt32Array::from(vec![7, 7])) as ArrayRef,
            Arc::new(UInt8Array::from(vec![1, 1])) as ArrayRef,
        ],
    )
    .unwrap();
    let alternate = encode_multimod_arrow_sidecar_v1(
        RESULT_ID,
        "interventional-bootstrap-target-vectors.arrow",
        &original.descriptor.identity_sha256,
        "interventional-bootstrap:target-vectors",
        &alternate_batch,
    );
    assert!(matches!(
        alternate,
        Err(MultiModArchiveErrorV1::SidecarSchemaMismatch(_))
    ));
}

#[cfg(windows)]
mod strict_archive {
    use super::*;
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisRecipeV4, AnalysisSettings, CausalIdentificationChecklistV1,
        CausalLinearEquationV1, CausalLinearTermV1, CausalPositivityPolicyV1, GeneralSemConfigV1,
        INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION, InterventionalCausalMediationConfigV1,
        LegacyEstimandConfirmationV4, MissingDataPolicyV4, MultiModCompilationReceiptV1,
        MultiModCompilerTargetV1, ObservedCausalPathV1, ObservedRoleV4, ObservedScaleV4,
        ObservedTreatmentContrastV1, SEM_MODEL_V4_SCHEMA_VERSION, SemDataBindingV4, SemGroupV4,
        SemModelV4, SemParameterTargetV4, SemParameterV4, SemPresentationV4, SemRelationV4,
        SemVariableV4, StructuralRelationRoleV4, compile_multimod_recipe_v1,
    };
    use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
    use qpls_project::{
        append_multimod_result_and_canonical_to_archive_v6,
        create_populated_general_sem_project_archive_v6, load_project_archive_v6,
    };
    use qpls_runner::{MultiModCanonicalRunContextV1, build_multimod_canonical_result_v2};
    use std::{
        fs::{self, File},
        io::{Read, Write},
        path::{Path, PathBuf},
    };
    use tempfile::tempdir;
    use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

    fn observed(
        id: &str,
        scale: ObservedScaleV4,
        role: ObservedRoleV4,
        categories: &[&str],
    ) -> SemVariableV4 {
        SemVariableV4::Observed {
            id: id.into(),
            label: id.into(),
            source_column: id.into(),
            scale,
            role,
            categories: categories.iter().map(|value| (*value).into()).collect(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        }
    }

    fn push_structural(model: &mut SemModelV4, source: &str, target: &str) {
        let relation_id = format!("relation:{source}:{target}");
        let parameter_id = format!("parameter:{source}:{target}");
        model.relations.push(SemRelationV4::Structural {
            id: relation_id,
            source: source.into(),
            target: target.into(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter_id,
            label: format!("{source} to {target}"),
            target: SemParameterTargetV4::Regression {
                source: source.into(),
                target: target.into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }

    fn observed_causal_model(dataset: &Dataset) -> SemModelV4 {
        let mut model = SemModelV4 {
            schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
            id: "model:multimod-causal-archive-fixture".into(),
            name: "MultiMod causal archive fixture".into(),
            variables: vec![
                observed(
                    "treatment",
                    ObservedScaleV4::Binary,
                    ObservedRoleV4::Structural,
                    &["0", "1"],
                ),
                observed(
                    "mediator",
                    ObservedScaleV4::Continuous,
                    ObservedRoleV4::Structural,
                    &[],
                ),
                observed(
                    "outcome",
                    ObservedScaleV4::Continuous,
                    ObservedRoleV4::Structural,
                    &[],
                ),
                observed(
                    "covariate",
                    ObservedScaleV4::Continuous,
                    ObservedRoleV4::Control,
                    &[],
                ),
            ],
            relations: Vec::new(),
            parameters: Vec::new(),
            constraints: Vec::new(),
            derived_terms: Vec::new(),
            group: SemGroupV4::SingleGroup,
            data_binding: SemDataBindingV4::Raw {
                dataset_id: dataset.id.to_string(),
                missing_data: MissingDataPolicyV4::ListwiseDeletion,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
            },
            annotations: Vec::new(),
            presentation: SemPresentationV4::None,
        };
        for (source, target) in [
            ("treatment", "mediator"),
            ("covariate", "mediator"),
            ("treatment", "outcome"),
            ("mediator", "outcome"),
            ("covariate", "outcome"),
        ] {
            push_structural(&mut model, source, target);
        }
        model.ensure_valid().unwrap();
        model
    }

    fn causal_recipe(dataset: &Dataset, model: &SemModelV4) -> AnalysisRecipeV4 {
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0101),
            created_at: Utc.with_ymd_and_hms(2026, 8, 24, 9, 15, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model.id.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::NotLegacy,
            settings: AnalysisSettings {
                method: AnalysisMethod::Regression,
                bootstrap_samples: 500,
                seed: 42,
                confidence_level: 0.95,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: None,
            general_sem_config: Some(GeneralSemConfigV1::default()),
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: Some(InterventionalCausalMediationConfigV1 {
                schema_version: INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION,
                treatment: "treatment".into(),
                treatment_contrast: ObservedTreatmentContrastV1::Binary {
                    control: 0.0,
                    treated: 1.0,
                },
                outcome: "outcome".into(),
                mediators: vec!["mediator".into()],
                baseline_moderators: Vec::new(),
                adjustment_covariates: vec!["covariate".into()],
                paths: vec![ObservedCausalPathV1 {
                    path_id: "path:treatment-mediator-outcome".into(),
                    ordered_variable_ids: vec![
                        "treatment".into(),
                        "mediator".into(),
                        "outcome".into(),
                    ],
                    equations: vec![
                        CausalLinearEquationV1 {
                            equation_id: "equation:mediator".into(),
                            outcome_variable_id: "mediator".into(),
                            terms: vec![
                                CausalLinearTermV1 {
                                    term_id: "term:treatment".into(),
                                    factor_variable_ids: vec!["treatment".into()],
                                },
                                CausalLinearTermV1 {
                                    term_id: "term:covariate".into(),
                                    factor_variable_ids: vec!["covariate".into()],
                                },
                            ],
                        },
                        CausalLinearEquationV1 {
                            equation_id: "equation:outcome".into(),
                            outcome_variable_id: "outcome".into(),
                            terms: vec![
                                CausalLinearTermV1 {
                                    term_id: "term:mediator".into(),
                                    factor_variable_ids: vec!["mediator".into()],
                                },
                                CausalLinearTermV1 {
                                    term_id: "term:covariate".into(),
                                    factor_variable_ids: vec!["covariate".into()],
                                },
                            ],
                        },
                    ],
                }],
                positivity_policy: CausalPositivityPolicyV1::default(),
                identification: CausalIdentificationChecklistV1 {
                    temporal_order_declared: true,
                    adjustment_set_justified: true,
                    consistency_assumption_acknowledged: true,
                    no_unmeasured_treatment_outcome_confounding_acknowledged: true,
                    no_unmeasured_treatment_mediator_confounding_acknowledged: true,
                    no_unmeasured_mediator_outcome_confounding_acknowledged: true,
                    no_exposure_induced_mediator_outcome_confounder_confirmed: true,
                    no_recanting_witness_confirmed: true,
                    linear_model_specification_reviewed: true,
                    positivity_reviewed: true,
                },
                bootstrap_resamples: 500,
                seed: 42,
                confidence_level: 0.95,
            }),
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        recipe.ensure_valid().unwrap();
        recipe
    }

    fn compiled_provenance(recipe: &AnalysisRecipeV4, model: &SemModelV4) -> MultimodProvenanceV1 {
        let compiled = compile_multimod_recipe_v1(
            recipe,
            model,
            MultiModCompilerTargetV1::InterventionalCausalMediationV1,
        )
        .unwrap();
        provenance_from_receipt(compiled.receipt(), 42)
    }

    fn provenance_from_receipt(
        receipt: &MultiModCompilationReceiptV1,
        seed: u64,
    ) -> MultimodProvenanceV1 {
        MultimodProvenanceV1 {
            method_version: receipt.method_version.clone(),
            recipe_id: receipt.recipe_id.clone(),
            recipe_analytical_sha256: receipt.recipe_analytical_sha256.clone(),
            config_sha256: receipt.config_sha256.clone(),
            model_id: receipt.model_id.clone(),
            model_scientific_sha256: receipt.model_scientific_sha256.clone(),
            dataset_id: receipt.dataset_id.clone(),
            dataset_fingerprint: receipt.dataset_fingerprint.clone(),
            engine_version: env!("CARGO_PKG_VERSION").into(),
            seed,
            capability_cell: receipt.capability_cell.clone(),
            qualification: MultimodQualificationStateV1::UnqualifiedLabs,
            candidate_qualification_receipt: None,
        }
    }

    fn dataset() -> Dataset {
        import_delimited_bytes(
            b"treatment,mediator,outcome,covariate\n\
              0,1.0,2.0,0.1\n\
              1,2.2,4.1,0.2\n\
              0,1.4,2.8,0.3\n\
              1,2.8,5.0,0.4\n\
              0,1.7,3.1,0.5\n\
              1,3.1,5.8,0.6\n\
              0,2.0,3.7,0.7\n\
              1,3.5,6.4,0.8\n\
              0,2.3,4.0,0.9\n\
              1,3.9,7.1,1.0\n\
              0,2.6,4.6,1.1\n\
              1,4.2,7.7,1.2\n",
            "multimod-causal-archive-fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ArchiveMutation {
        Missing,
        Tampered,
        Duplicate,
    }

    fn rewrite_central_directory_entry_name(path: &Path, from: &str, to: &str) {
        const CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
        const END_OF_CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

        assert_eq!(from.len(), to.len());
        let mut bytes = fs::read(path).unwrap();
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

        assert_eq!(rewritten, 1, "expected one shadow entry to rewrite");
        fs::write(path, bytes).unwrap();
    }

    fn rewrite_archive_with_sidecar_mutation(
        source: &Path,
        destination: &Path,
        sidecar_entry: &str,
        mutation: ArchiveMutation,
    ) {
        let mut input = ZipArchive::new(File::open(source).unwrap()).unwrap();
        let output = File::create(destination).unwrap();
        let mut writer = ZipWriter::new(output);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut sidecar_bytes = None;
        let mut duplicate_shadow_entry = None;

        for index in 0..input.len() {
            let mut entry = input.by_index(index).unwrap();
            let name = entry.name().to_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if name == sidecar_entry {
                sidecar_bytes = Some(bytes.clone());
                if mutation == ArchiveMutation::Missing {
                    continue;
                }
                if mutation == ArchiveMutation::Tampered {
                    bytes[0] ^= 0x01;
                }
            }
            writer.start_file(name, options).unwrap();
            writer.write_all(&bytes).unwrap();
        }

        if mutation == ArchiveMutation::Duplicate {
            let shadow_entry = "z".repeat(sidecar_entry.len());
            writer.start_file(&shadow_entry, options).unwrap();
            writer.write_all(&sidecar_bytes.unwrap()).unwrap();
            duplicate_shadow_entry = Some(shadow_entry);
        }
        writer.finish().unwrap().sync_all().unwrap();

        if let Some(shadow_entry) = duplicate_shadow_entry {
            rewrite_central_directory_entry_name(destination, &shadow_entry, sidecar_entry);
        }
    }

    fn archive_sha256(path: &Path) -> String {
        sha256_bytes(&fs::read(path).unwrap())
    }

    fn create_source_archive(
        directory: &Path,
    ) -> (
        PathBuf,
        MultiModResultAttachmentV1,
        MultiModSidecarPayloadV1,
        qpls_project::CanonicalResultDocumentV2,
    ) {
        let archive = directory.join("multimod-source.qpls");
        let dataset = dataset();
        let model = observed_causal_model(&dataset);
        let recipe = causal_recipe(&dataset, &model);
        let provenance = compiled_provenance(&recipe, &model);
        let (attachment, payload) = attachment_and_payload(recipe.id, provenance.clone());
        let project_id = Uuid::from_u128(0x6d75_6c74_696d_6f64_0000_0000_0000_0102);
        let canonical_core = build_multimod_canonical_result_v2(
            &MultiModCanonicalRunContextV1 {
                run_id: attachment.result_id.clone(),
                project_id: project_id.to_string(),
                recipe_id: provenance.recipe_id.clone(),
                recipe_analytical_sha256: provenance.recipe_analytical_sha256.clone(),
                model_id: model.id.clone(),
                model_scientific_sha256: model.scientific_sha256().unwrap(),
                dataset_id: dataset.id.to_string(),
                dataset_fingerprint: dataset.fingerprint.0.clone(),
                engine_version: provenance.engine_version.clone(),
                workers: 1,
                started_at: "2026-08-24T09:30:01Z".into(),
                completed_at: "2026-08-24T09:30:02Z".into(),
            },
            &attachment.result,
        )
        .unwrap();
        let canonical: qpls_project::CanonicalResultDocumentV2 = serde_json::from_value(
            serde_json::to_value(canonical_core).expect("canonical core document serializes"),
        )
        .expect("canonical core and Archive V6 documents share the strict wire contract");
        create_populated_general_sem_project_archive_v6(
            &archive,
            project_id,
            "MultiMod archive fixture",
            Utc.with_ymd_and_hms(2026, 8, 24, 9, 30, 0).unwrap(),
            &dataset,
            model,
            recipe,
        )
        .unwrap();
        (archive, attachment, payload, canonical)
    }

    #[test]
    fn strict_append_reopen_and_corrupted_sidecar_archives_fail_closed() {
        let directory = tempdir().unwrap();
        let (archive, attachment, payload, canonical) = create_source_archive(directory.path());
        let source_sha256 = archive_sha256(&archive);

        assert!(matches!(
            append_multimod_result_and_canonical_to_archive_v6(
                &archive,
                &source_sha256,
                attachment.clone(),
                Vec::new(),
                canonical.clone(),
                || false,
            ),
            Err(MultiModArchiveErrorV1::SidecarSetMismatch)
        ));
        assert_eq!(archive_sha256(&archive), source_sha256);

        assert!(matches!(
            append_multimod_result_and_canonical_to_archive_v6(
                &archive,
                &source_sha256,
                attachment.clone(),
                vec![payload.clone(), payload.clone()],
                canonical.clone(),
                || false,
            ),
            Err(MultiModArchiveErrorV1::SidecarSetMismatch)
        ));
        assert_eq!(archive_sha256(&archive), source_sha256);

        let mut tampered_payload = payload.clone();
        tampered_payload.bytes[0] ^= 0x01;
        assert!(matches!(
            append_multimod_result_and_canonical_to_archive_v6(
                &archive,
                &source_sha256,
                attachment.clone(),
                vec![tampered_payload],
                canonical.clone(),
                || false,
            ),
            Err(MultiModArchiveErrorV1::SidecarDigestMismatch(_))
        ));
        assert_eq!(archive_sha256(&archive), source_sha256);

        let receipt = append_multimod_result_and_canonical_to_archive_v6(
            &archive,
            &source_sha256,
            attachment,
            vec![payload.clone()],
            canonical,
            || false,
        )
        .unwrap();
        assert!(receipt.source_verified_at_commit);
        assert!(receipt.post_write_validated);
        assert_eq!(receipt.sidecar_count, 1);

        let reopened = load_project_archive_v6(&archive).unwrap();
        assert_eq!(reopened.document.multimod_results.len(), 1);
        assert!(
            reopened
                .multimod_sidecars
                .contains(&payload.descriptor.entry_name)
        );

        for (name, mutation) in [
            ("missing-sidecar.qpls", ArchiveMutation::Missing),
            ("tampered-sidecar.qpls", ArchiveMutation::Tampered),
            ("duplicate-sidecar.qpls", ArchiveMutation::Duplicate),
        ] {
            let mutated = directory.path().join(name);
            rewrite_archive_with_sidecar_mutation(
                &archive,
                &mutated,
                &payload.descriptor.entry_name,
                mutation,
            );
            assert!(
                load_project_archive_v6(&mutated).is_err(),
                "strict reopen unexpectedly admitted {mutation:?} sidecar corruption"
            );
        }
    }
}
