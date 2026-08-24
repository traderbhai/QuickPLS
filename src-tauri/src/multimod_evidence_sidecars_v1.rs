//! Typed Arrow persistence for the additive MultiMod runner evidence.
//!
//! Scientific runner evidence must never be hidden in an opaque JSON cell.
//! This adapter normalizes every built-in evidence family into typed tables
//! before the result is attached to an Archive V6 document.

use arrow::{
    array::{BooleanArray, Float64Array, StringArray, UInt8Array, UInt32Array, UInt64Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use qpls_core::MULTIMOD_SIDECAR_MAX_BYTES_V1;
use qpls_estimation::{
    FimixPlsV2Result, FrequencyPairwisePartitionPlanV1, GroupBootstrapBanksV1, GroupIndexV1,
    InverseVarianceWaldResultV1, MicomPairwiseResultV1, OmnibusPermutationResultV1,
    OrdinaryPlsPathStandardErrorV1, POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2,
    PairwiseParametricTestV1, PairwisePartitionPlanV1, PairwisePermutationResultV1,
    ParameterFamilyV1, ParameterIdentityV1, PermutationLedgerEntryV1, PlsPosV2Result,
    PooledStructuralBaselineV2, PosScoringContractV2, RefitFailureCodeV1, ResampleFitStatusV1,
    validate_fimix_multistart_evidence_v2, validate_pos_multistart_evidence_v2,
};
use qpls_project::{
    MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
    MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1, MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1,
    MultiModSidecarPayloadV1, encode_multimod_arrow_sidecar_v1,
    multimod_bca_jackknife_summary_batch_v1, multimod_membership_with_row_tokens_batch_v1,
    multimod_micom_null_statistics_batch_v1, multimod_resample_ledger_batch_v1,
    multimod_start_trace_batch_v1, multimod_target_ledger_batch_v1,
    validate_multimod_sidecar_payload_v1,
};
use qpls_resampling::{MultiModFinalLedgerV1, MultiModRefitOutcomeV1};
use qpls_runner::{
    ConditionalCaseBootstrapLedgerV2, ConditionalDeleteOneLedgerV2,
    ConditionalFrequencyBootstrapLedgerV2, ConditionalProcessAnalysisFrameV2,
    ConditionalStudentizedLedgerV2, MultiModRunnerEvidenceV1, PreparedConditionalInferenceV2,
    PreparedHeterogeneityBootstrapV2, PreparedInterventionalBootstrapV1,
    PreparedPosCommonMetricEvidenceV1, PreparedReplicateStatusV1, PreparedSharedReplicateLedgerV1,
    PreparedTargetReplicatesV1, RawConditionalProcessEvidenceV2,
    RawHeterogeneityPreparationReceiptV2,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) struct EncodedMultiModEvidenceV1 {
    pub payloads: Vec<MultiModSidecarPayloadV1>,
    pub total_uncompressed_bytes: u64,
}

struct EvidenceTableV1 {
    leaf_suffix: String,
    batch: RecordBatch,
}

enum MetadataValueV1 {
    String(String),
    Unsigned(u64),
    Number(f64),
    Boolean(bool),
}

struct MetadataRowV1 {
    field_name: String,
    value: MetadataValueV1,
}

fn metadata_string(field: &str, value: impl Into<String>) -> MetadataRowV1 {
    MetadataRowV1 {
        field_name: field.into(),
        value: MetadataValueV1::String(value.into()),
    }
}

fn metadata_u64(field: &str, value: impl TryInto<u64>) -> Result<MetadataRowV1, String> {
    Ok(MetadataRowV1 {
        field_name: field.into(),
        value: MetadataValueV1::Unsigned(
            value
                .try_into()
                .map_err(|_| format!("{field} does not fit the Arrow unsigned metadata type"))?,
        ),
    })
}

fn metadata_number(field: &str, value: f64) -> Result<MetadataRowV1, String> {
    if !value.is_finite() {
        return Err(format!("{field} is nonfinite"));
    }
    Ok(MetadataRowV1 {
        field_name: field.into(),
        value: MetadataValueV1::Number(value),
    })
}

fn metadata_bool(field: &str, value: bool) -> MetadataRowV1 {
    MetadataRowV1 {
        field_name: field.into(),
        value: MetadataValueV1::Boolean(value),
    }
}

fn stable_enum_id_v1<T: Serialize>(value: &T) -> Result<String, String> {
    match serde_json::to_value(value).map_err(|error| error.to_string())? {
        serde_json::Value::String(value) => Ok(value),
        other => Err(format!(
            "expected a unit enum stable identifier, found {other}"
        )),
    }
}

fn metadata_batch_v1(evidence_kind: &str, rows: Vec<MetadataRowV1>) -> Result<RecordBatch, String> {
    if rows.is_empty() {
        return Err(format!(
            "{evidence_kind} has no scalar metadata receipt fields"
        ));
    }
    let length = rows.len();
    let mut kinds = Vec::with_capacity(length);
    let mut names = Vec::with_capacity(length);
    let mut value_types = Vec::with_capacity(length);
    let mut string_values = Vec::<Option<String>>::with_capacity(length);
    let mut unsigned_values = Vec::<Option<u64>>::with_capacity(length);
    let mut number_values = Vec::<Option<f64>>::with_capacity(length);
    let mut boolean_values = Vec::<Option<bool>>::with_capacity(length);
    for row in rows {
        kinds.push(evidence_kind.to_owned());
        names.push(row.field_name);
        match row.value {
            MetadataValueV1::String(value) => {
                value_types.push("string".to_owned());
                string_values.push(Some(value));
                unsigned_values.push(None);
                number_values.push(None);
                boolean_values.push(None);
            }
            MetadataValueV1::Unsigned(value) => {
                value_types.push("unsigned".to_owned());
                string_values.push(None);
                unsigned_values.push(Some(value));
                number_values.push(None);
                boolean_values.push(None);
            }
            MetadataValueV1::Number(value) => {
                value_types.push("number".to_owned());
                string_values.push(None);
                unsigned_values.push(None);
                number_values.push(Some(value));
                boolean_values.push(None);
            }
            MetadataValueV1::Boolean(value) => {
                value_types.push("boolean".to_owned());
                string_values.push(None);
                unsigned_values.push(None);
                number_values.push(None);
                boolean_values.push(Some(value));
            }
        }
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("evidence_kind", DataType::Utf8, false),
        Field::new("field_name", DataType::Utf8, false),
        Field::new("value_type", DataType::Utf8, false),
        Field::new("string_value", DataType::Utf8, true),
        Field::new("unsigned_value", DataType::UInt64, true),
        Field::new("number_value", DataType::Float64, true),
        Field::new("boolean_value", DataType::Boolean, true),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(kinds)),
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(value_types)),
            Arc::new(StringArray::from(string_values)),
            Arc::new(UInt64Array::from(unsigned_values)),
            Arc::new(Float64Array::from(number_values)),
            Arc::new(BooleanArray::from(boolean_values)),
        ],
    )
    .map_err(|error| error.to_string())
}

fn table_v1(suffix: impl Into<String>, batch: RecordBatch) -> EvidenceTableV1 {
    EvidenceTableV1 {
        leaf_suffix: suffix.into(),
        batch,
    }
}

fn u32_checked(value: usize, subject: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{subject} exceeds the Arrow UInt32 contract"))
}

fn group_u8(group: GroupIndexV1) -> u8 {
    u8::from(group)
}

fn parameter_family_id_v1(family: ParameterFamilyV1) -> &'static str {
    match family {
        ParameterFamilyV1::StructuralPath => "structural_path",
        ParameterFamilyV1::OuterLoading => "outer_loading",
        ParameterFamilyV1::OuterWeight => "outer_weight",
        ParameterFamilyV1::RSquared => "r_squared",
        ParameterFamilyV1::SpecificIndirect => "specific_indirect",
        ParameterFamilyV1::TotalIndirect => "total_indirect",
        ParameterFamilyV1::InteractionGamma => "interaction_gamma",
        ParameterFamilyV1::ThreeWayDelta => "three_way_delta",
        ParameterFamilyV1::SimpleSlope => "simple_slope",
        ParameterFamilyV1::Other => "other",
    }
}

fn refit_failure_code_v1(code: RefitFailureCodeV1) -> Result<String, String> {
    stable_enum_id_v1(&code)
}

fn source_row_token_v1(dataset_id: &str, source_row: u64) -> String {
    format!("qpls.row.v1:{dataset_id}:{source_row}")
}

fn analysis_row_tokens_v1(
    dataset_id: &str,
    evidence: &[MultiModRunnerEvidenceV1],
) -> Result<Option<Vec<String>>, String> {
    let receipts = evidence
        .iter()
        .filter_map(|item| match item {
            MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(receipt) => Some(receipt),
            _ => None,
        })
        .collect::<Vec<_>>();
    if receipts.len() > 1 {
        return Err("heterogeneity evidence contains duplicate raw-preparation receipts".into());
    }
    if let Some(receipt) = receipts.first() {
        receipt.ensure_valid()?;
    }
    Ok(receipts.first().map(|receipt| {
        receipt
            .source_row_tokens
            .iter()
            .map(|source_row| source_row_token_v1(dataset_id, *source_row))
            .collect()
    }))
}

pub(crate) fn encode_multimod_runner_evidence_v1(
    result_id: &str,
    result_identity_sha256: &str,
    dataset_id: &str,
    evidence: &[MultiModRunnerEvidenceV1],
) -> Result<EncodedMultiModEvidenceV1, String> {
    if evidence.is_empty() {
        return Err("the runner returned no MultiMod evidence".into());
    }
    let analysis_row_tokens = analysis_row_tokens_v1(dataset_id, evidence)?;
    let mut payloads = Vec::new();
    let mut total_uncompressed_bytes = 0u64;
    for (evidence_index, item) in evidence.iter().enumerate() {
        let (kind, tables) =
            encode_evidence_tables_v1(item, dataset_id, analysis_row_tokens.as_deref())?;
        if tables.is_empty() {
            return Err(format!("{kind} produced no typed evidence table"));
        }
        for (table_index, table) in tables.into_iter().enumerate() {
            // Optional evidence tables with a declared zero count are omitted;
            // the scalar metadata receipt carries that count. Archive V6 never
            // admits empty Arrow sidecars, and no scientific observations are
            // lost by omitting an empty relation.
            if table.batch.num_rows() == 0 {
                continue;
            }
            let leaf_name = format!(
                "evidence-{evidence_index:04}-{kind}-{table_index:02}-{}.arrow",
                table.leaf_suffix
            );
            let payload = encode_multimod_arrow_sidecar_v1(
                result_id,
                &leaf_name,
                result_identity_sha256,
                &format!("{kind}:{}", table.leaf_suffix),
                &table.batch,
            )
            .map_err(|error| error.to_string())?;
            validate_multimod_sidecar_payload_v1(result_id, &payload)
                .map_err(|error| error.to_string())?;
            total_uncompressed_bytes = total_uncompressed_bytes
                .checked_add(payload.descriptor.uncompressed_bytes)
                .ok_or_else(|| "the MultiMod sidecar byte total overflowed".to_owned())?;
            if total_uncompressed_bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
                return Err("the typed evidence exceeds the 512 MiB per-run archive cap".into());
            }
            payloads.push(payload);
        }
    }
    Ok(EncodedMultiModEvidenceV1 {
        payloads,
        total_uncompressed_bytes,
    })
}

fn encode_evidence_tables_v1(
    evidence: &MultiModRunnerEvidenceV1,
    dataset_id: &str,
    analysis_row_tokens: Option<&[String]>,
) -> Result<(&'static str, Vec<EvidenceTableV1>), String> {
    match evidence {
        MultiModRunnerEvidenceV1::MgaPairwisePartitionPlan(value) => Ok((
            "mga-pairwise-partition-plan",
            encode_pairwise_partition_plan_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaFrequencyPairwisePartitionPlan(value) => Ok((
            "mga-frequency-pairwise-partition-plan",
            encode_frequency_pairwise_partition_plan_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaPairwisePermutation(value) => Ok((
            "mga-pairwise-permutation",
            encode_pairwise_permutation_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaOmnibusPermutation(value) => Ok((
            "mga-omnibus-permutation",
            encode_omnibus_permutation_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaBootstrapBanks(value) => Ok((
            "mga-bootstrap-banks",
            encode_group_bootstrap_banks_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaMicomPair(value) => {
            Ok(("mga-micom-pair", encode_micom_pair_v1(value)?))
        }
        MultiModRunnerEvidenceV1::MgaOrdinaryPlsPathStandardError {
            parameter,
            group,
            receipt,
        } => Ok((
            "mga-ordinary-pls-path-standard-error",
            encode_ordinary_pls_path_standard_error_v1(parameter, *group, receipt)?,
        )),
        MultiModRunnerEvidenceV1::MgaPairwiseParametric(value) => Ok((
            "mga-pairwise-parametric",
            encode_pairwise_parametric_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::MgaParametricWald(value) => {
            Ok(("mga-parametric-wald", encode_parametric_wald_v1(value)?))
        }
        MultiModRunnerEvidenceV1::FimixCandidate { k, result } => Ok((
            "fimix-candidate",
            encode_fimix_candidate_v1(*k, result, analysis_row_tokens)?,
        )),
        MultiModRunnerEvidenceV1::PlsPosCandidate { k, result } => Ok((
            "pls-pos-candidate",
            encode_pls_pos_candidate_v1(*k, result, analysis_row_tokens)?,
        )),
        MultiModRunnerEvidenceV1::HeterogeneityPooledBaseline(value) => Ok((
            "heterogeneity-pooled-baseline",
            encode_heterogeneity_pooled_baseline_v2(value)?,
        )),
        MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(value) => Ok((
            "heterogeneity-raw-preparation",
            encode_raw_heterogeneity_preparation_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::HeterogeneityPosCommonMetric(value) => Ok((
            "heterogeneity-pos-common-metric",
            encode_pos_common_metric_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::HeterogeneityBootstrap(value) => Ok((
            "heterogeneity-bootstrap",
            encode_heterogeneity_bootstrap_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::ConditionalInference(value) => Ok((
            "conditional-inference",
            encode_conditional_inference_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::ConditionalRawPreparation(value) => Ok((
            "conditional-raw-preparation",
            encode_conditional_raw_preparation_v2(dataset_id, value)?,
        )),
        MultiModRunnerEvidenceV1::ConditionalRawFullRefit(value) => Ok((
            "conditional-raw-full-refit",
            encode_conditional_raw_full_refit_v2(value)?,
        )),
        MultiModRunnerEvidenceV1::InterventionalBootstrap(value) => Ok((
            "interventional-bootstrap",
            encode_interventional_bootstrap_v1(value)?,
        )),
        MultiModRunnerEvidenceV1::InterventionalFullRefitLedger(value) => Ok((
            "interventional-full-refit-ledger",
            encode_interventional_full_refit_ledger_v1(value)?,
        )),
    }
}

fn encode_pos_common_metric_v1(
    value: &PreparedPosCommonMetricEvidenceV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    value.ensure_valid()?;
    let gate = &value.gate_input;
    let result = &value.gate_result;
    let metadata = metadata_batch_v1(
        "heterogeneity_pos_common_metric",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_string("pooled_metric_id", gate.pooled_metric_id.clone()),
            metadata_string("pooled_metric_sha256", gate.pooled_metric_sha256.clone()),
            metadata_u64("segments", gate.segments)?,
            metadata_bool(
                "applied_identically_to_all_segments",
                gate.applied_identically_to_all_segments,
            ),
            metadata_string("gate_method_version", result.method_version.clone()),
            metadata_string("gate_status", stable_enum_id_v1(&result.status)?),
            metadata_bool(
                "inferential_gamma_delta_slope_contrasts_allowed",
                result.inferential_gamma_delta_slope_contrasts_allowed,
            ),
            metadata_u64(
                "step3_failed_mean_comparisons",
                result.step3_failed_mean_comparisons,
            )?,
            metadata_u64(
                "step3_failed_variance_comparisons",
                result.step3_failed_variance_comparisons,
            )?,
            metadata_bool(
                "step3_required_for_standardized_path_comparison",
                result.step3_required_for_standardized_path_comparison,
            ),
            metadata_u64("micom_pair_count", value.micom_pairs.len())?,
            metadata_u64(
                "common_metric_parameter_count",
                value.common_metric_parameters.len(),
            )?,
        ],
    )?;

    let required = gate
        .required_construct_ids
        .iter()
        .map(|construct_id| {
            let evidence = gate
                .evidence
                .iter()
                .find(|entry| entry.construct_id == *construct_id);
            (
                construct_id.clone(),
                evidence.is_some_and(|entry| entry.configural_identity_passed),
                result.required_construct_ids.contains(construct_id),
            )
        })
        .collect::<Vec<_>>();
    let constructs = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("construct_id", DataType::Utf8, false),
            Field::new("configural_identity_passed", DataType::Boolean, false),
            Field::new("retained_by_gate", DataType::Boolean, false),
        ])),
        vec![
            Arc::new(StringArray::from(
                required
                    .iter()
                    .map(|(construct, _, _)| construct.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                required
                    .iter()
                    .map(|(_, configural, _)| *configural)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                required
                    .iter()
                    .map(|(_, _, retained)| *retained)
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let compositional = gate
        .evidence
        .iter()
        .flat_map(|entry| {
            entry.compositional_invariance.iter().map(move |pair| {
                (
                    entry.construct_id.clone(),
                    pair.left_segment,
                    pair.right_segment,
                    pair.passed,
                    pair.permutation_p_value,
                )
            })
        })
        .collect::<Vec<_>>();
    let compositional_pairs = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("construct_id", DataType::Utf8, false),
            Field::new("left_segment", DataType::UInt32, false),
            Field::new("right_segment", DataType::UInt32, false),
            Field::new("compositional_invariance_passed", DataType::Boolean, false),
            Field::new("permutation_p_value", DataType::Float64, true),
        ])),
        vec![
            Arc::new(StringArray::from(
                compositional
                    .iter()
                    .map(|row| row.0.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                compositional
                    .iter()
                    .map(|row| u32_checked(row.1, "left POS segment"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(UInt32Array::from(
                compositional
                    .iter()
                    .map(|row| u32_checked(row.2, "right POS segment"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(BooleanArray::from(
                compositional.iter().map(|row| row.3).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                compositional.iter().map(|row| row.4).collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let step3 = gate
        .evidence
        .iter()
        .flat_map(|entry| {
            entry.step3_equality.iter().map(move |pair| {
                (
                    entry.construct_id.clone(),
                    pair.left_segment,
                    pair.right_segment,
                    pair.mean_equality_passed,
                    pair.variance_equality_passed,
                )
            })
        })
        .collect::<Vec<_>>();
    let step3_pairs = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("construct_id", DataType::Utf8, false),
            Field::new("left_segment", DataType::UInt32, false),
            Field::new("right_segment", DataType::UInt32, false),
            Field::new("mean_equality_passed", DataType::Boolean, false),
            Field::new("variance_equality_passed", DataType::Boolean, false),
        ])),
        vec![
            Arc::new(StringArray::from(
                step3.iter().map(|row| row.0.clone()).collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                step3
                    .iter()
                    .map(|row| u32_checked(row.1, "left POS Step 3 segment"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(UInt32Array::from(
                step3
                    .iter()
                    .map(|row| u32_checked(row.2, "right POS Step 3 segment"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(BooleanArray::from(
                step3.iter().map(|row| row.3).collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                step3.iter().map(|row| row.4).collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let blockers = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("blocker_code", DataType::Utf8, false),
            Field::new("construct_id", DataType::Utf8, true),
            Field::new("left_segment", DataType::UInt32, true),
            Field::new("right_segment", DataType::UInt32, true),
            Field::new("message", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(StringArray::from(
                result
                    .blockers
                    .iter()
                    .map(|blocker| stable_enum_id_v1(&blocker.code))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(
                result
                    .blockers
                    .iter()
                    .map(|blocker| blocker.construct_id.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                result
                    .blockers
                    .iter()
                    .map(|blocker| {
                        blocker
                            .left_segment
                            .map(|segment| u32_checked(segment, "left blocker segment"))
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(UInt32Array::from(
                result
                    .blockers
                    .iter()
                    .map(|blocker| {
                        blocker
                            .right_segment
                            .map(|segment| u32_checked(segment, "right blocker segment"))
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(
                result
                    .blockers
                    .iter()
                    .map(|blocker| blocker.message.clone())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let parameters = &value.common_metric_parameters;
    let parameter_receipts = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("segment_id", DataType::UInt8, false),
            Field::new("target_id", DataType::Utf8, false),
            Field::new("target_kind", DataType::Utf8, false),
            Field::new("metric", DataType::Utf8, false),
            Field::new("estimate", DataType::Float64, false),
            Field::new("standard_error", DataType::Float64, true),
            Field::new("p_value", DataType::Float64, true),
            Field::new("interval_family", DataType::Utf8, true),
            Field::new("interval_alternative", DataType::Utf8, true),
            Field::new("interval_confidence_level", DataType::Float64, true),
            Field::new("interval_lower", DataType::Float64, true),
            Field::new("interval_upper", DataType::Float64, true),
        ])),
        vec![
            Arc::new(UInt8Array::from(
                parameters
                    .iter()
                    .map(|row| row.class_id)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                parameters
                    .iter()
                    .map(|row| row.parameter.target_id.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                parameters
                    .iter()
                    .map(|row| row.parameter.target_kind.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                parameters
                    .iter()
                    .map(|row| row.metric.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| row.parameter.estimate)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| row.parameter.standard_error)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| row.parameter.p_value)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                parameters
                    .iter()
                    .map(|row| {
                        row.parameter
                            .interval
                            .as_ref()
                            .map(|interval| interval.family.clone())
                    })
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                parameters
                    .iter()
                    .map(|row| {
                        row.parameter
                            .interval
                            .as_ref()
                            .map(|interval| stable_enum_id_v1(&interval.alternative))
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| {
                        row.parameter
                            .interval
                            .as_ref()
                            .map(|interval| interval.confidence_level)
                    })
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| {
                        row.parameter
                            .interval
                            .as_ref()
                            .and_then(|interval| interval.lower)
                    })
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                parameters
                    .iter()
                    .map(|row| {
                        row.parameter
                            .interval
                            .as_ref()
                            .and_then(|interval| interval.upper)
                    })
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("construct-gate", constructs),
        table_v1("compositional-pairs", compositional_pairs),
        table_v1("step3-pairs", step3_pairs),
        table_v1("blockers", blockers),
        table_v1("common-metric-parameters", parameter_receipts),
    ];
    for (index, pair) in value.micom_pairs.iter().enumerate() {
        for mut table in encode_micom_pair_v1(pair)? {
            table.leaf_suffix = format!("micom-{index:02}-{}", table.leaf_suffix);
            tables.push(table);
        }
    }
    Ok(tables)
}

fn encode_pairwise_partition_plan_v1(
    value: &PairwisePartitionPlanV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_pairwise_partition_plan",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("group_a", group_u8(value.pair.group_a))?,
            metadata_u64("group_b", group_u8(value.pair.group_b))?,
            metadata_u64("seed", value.seed)?,
            metadata_u64("requested", value.requested)?,
            metadata_u64("group_low_count", value.group_low_count)?,
            metadata_u64("group_high_count", value.group_high_count)?,
            metadata_string(
                "observed_membership_sha256",
                value.observed_membership_sha256.clone(),
            ),
            metadata_string("plan_sha256", value.plan_sha256.clone()),
        ],
    )?;
    let entries_schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("partition_sha256", DataType::Utf8, false),
    ]));
    let entries = RecordBatch::try_new(
        entries_schema,
        vec![
            Arc::new(UInt32Array::from(
                value
                    .entries
                    .iter()
                    .map(|entry| u32_checked(entry.replicate, "partition replicate"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(
                value
                    .entries
                    .iter()
                    .map(|entry| entry.partition_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("entries", entries),
    ])
}

fn encode_frequency_pairwise_partition_plan_v1(
    value: &FrequencyPairwisePartitionPlanV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_frequency_pairwise_partition_plan",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("group_a", group_u8(value.pair.group_a))?,
            metadata_u64("group_b", group_u8(value.pair.group_b))?,
            metadata_u64("seed", value.seed)?,
            metadata_u64("requested", value.requested)?,
            metadata_u64("group_low_total", value.group_low_total)?,
            metadata_u64("group_high_total", value.group_high_total)?,
            metadata_string(
                "observed_membership_sha256",
                value.observed_membership_sha256.clone(),
            ),
            metadata_string("plan_sha256", value.plan_sha256.clone()),
        ],
    )?;
    let entries_schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("partition_sha256", DataType::Utf8, false),
    ]));
    let entries = RecordBatch::try_new(
        entries_schema,
        vec![
            Arc::new(UInt32Array::from(
                value
                    .entries
                    .iter()
                    .map(|entry| u32_checked(entry.replicate, "frequency partition replicate"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(
                value
                    .entries
                    .iter()
                    .map(|entry| entry.partition_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("entries", entries),
    ])
}

fn permutation_ledger_batch_v1(
    entries: &[PermutationLedgerEntryV1],
) -> Result<RecordBatch, String> {
    let mut replicate_index = Vec::new();
    let mut partition_sha256 = Vec::new();
    let mut replicate_usable = Vec::new();
    let mut group_index = Vec::new();
    let mut group_usable = Vec::new();
    let mut failure_code = Vec::new();
    let mut failure_detail = Vec::new();
    for entry in entries {
        for fit in &entry.group_fits {
            replicate_index.push(u32_checked(entry.replicate, "permutation replicate")?);
            partition_sha256.push(entry.partition_sha256.clone());
            replicate_usable.push(entry.status == ResampleFitStatusV1::Usable);
            group_index.push(group_u8(fit.group));
            group_usable.push(fit.status == ResampleFitStatusV1::Usable);
            if let Some(failure) = &fit.failure {
                failure_code.push(refit_failure_code_v1(failure.code)?);
                failure_detail.push(failure.detail.clone());
            } else {
                failure_code.push(String::new());
                failure_detail.push(String::new());
            }
        }
    }
    if replicate_index.is_empty() {
        return Err("permutation ledger contains no group-fit rows".into());
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("partition_sha256", DataType::Utf8, false),
        Field::new("replicate_usable", DataType::Boolean, false),
        Field::new("group_index", DataType::UInt8, false),
        Field::new("group_usable", DataType::Boolean, false),
        Field::new("failure_code", DataType::Utf8, false),
        Field::new("failure_detail", DataType::Utf8, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(replicate_index)),
            Arc::new(StringArray::from(partition_sha256)),
            Arc::new(BooleanArray::from(replicate_usable)),
            Arc::new(UInt8Array::from(group_index)),
            Arc::new(BooleanArray::from(group_usable)),
            Arc::new(StringArray::from(failure_code)),
            Arc::new(StringArray::from(failure_detail)),
        ],
    )
    .map_err(|error| error.to_string())
}

fn pairwise_point_parameter_batch_v1(
    value: &PairwisePermutationResultV1,
) -> Result<RecordBatch, String> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("parameter_id", DataType::Utf8, false),
        Field::new("parameter_family", DataType::Utf8, false),
        Field::new("estimate_a", DataType::Float64, false),
        Field::new("estimate_b", DataType::Float64, false),
        Field::new("difference_a_minus_b", DataType::Float64, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(
                value
                    .point_estimates
                    .iter()
                    .map(|row| row.parameter.stable_id.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                value
                    .point_estimates
                    .iter()
                    .map(|row| parameter_family_id_v1(row.parameter.family))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .point_estimates
                    .iter()
                    .map(|row| row.estimate_a)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .point_estimates
                    .iter()
                    .map(|row| row.estimate_b)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .point_estimates
                    .iter()
                    .map(|row| row.difference_a_minus_b)
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())
}

fn pairwise_inference_parameter_batch_v1(
    value: &PairwisePermutationResultV1,
) -> Result<RecordBatch, String> {
    let selected_alternative = value
        .parameters
        .iter()
        .map(|row| stable_enum_id_v1(&row.selected_alternative))
        .collect::<Result<Vec<_>, _>>()?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("parameter_id", DataType::Utf8, false),
        Field::new("parameter_family", DataType::Utf8, false),
        Field::new("estimate_a", DataType::Float64, false),
        Field::new("estimate_b", DataType::Float64, false),
        Field::new("difference_a_minus_b", DataType::Float64, false),
        Field::new("p_value_two_sided", DataType::Float64, false),
        Field::new("p_value_greater", DataType::Float64, false),
        Field::new("p_value_less", DataType::Float64, false),
        Field::new("selected_alternative", DataType::Utf8, false),
        Field::new("selected_probability", DataType::Float64, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.parameter.stable_id.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                value
                    .parameters
                    .iter()
                    .map(|row| parameter_family_id_v1(row.parameter.family))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.estimate_a)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.estimate_b)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.difference_a_minus_b)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.p_value_two_sided)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.p_value_greater)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.p_value_less)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(selected_alternative)),
            Arc::new(Float64Array::from(
                value
                    .parameters
                    .iter()
                    .map(|row| row.selected_probability)
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())
}

fn pairwise_null_target_batch_v1(
    value: &PairwisePermutationResultV1,
) -> Result<RecordBatch, String> {
    if value
        .parameters
        .iter()
        .any(|row| !row.null_differences.is_empty() && row.null_differences.len() != value.usable)
    {
        return Err(
            "pairwise permutation null vectors disagree with the usable-ledger count".into(),
        );
    }
    let audit_parameters = value
        .parameters
        .iter()
        .filter(|row| !row.null_differences.is_empty())
        .collect::<Vec<_>>();
    if audit_parameters.len() != 1 {
        return Err(
            "pairwise permutation must retain exactly one bounded null audit target".into(),
        );
    }
    let mut replicate_index = Vec::with_capacity(value.requested);
    let mut target_id = Vec::with_capacity(replicate_index.capacity());
    let mut estimates = Vec::with_capacity(replicate_index.capacity());
    let mut valid = Vec::with_capacity(replicate_index.capacity());
    let mut failure_code = Vec::with_capacity(replicate_index.capacity());
    let mut usable_index = 0usize;
    for ledger in &value.ledger {
        let usable = ledger.status == ResampleFitStatusV1::Usable;
        for parameter in &audit_parameters {
            replicate_index.push(u32_checked(ledger.replicate, "pairwise null replicate")?);
            target_id.push(parameter.parameter.stable_id.clone());
            estimates.push(if usable {
                parameter.null_differences[usable_index]
            } else {
                0.0
            });
            valid.push(usable);
            failure_code.push(if usable {
                String::new()
            } else {
                "pairwise_refit_failed".into()
            });
        }
        if usable {
            usable_index += 1;
        }
    }
    multimod_target_ledger_batch_v1(replicate_index, target_id, estimates, valid, failure_code)
        .map_err(|error| error.to_string())
}

fn omnibus_null_target_batch_v1(value: &OmnibusPermutationResultV1) -> Result<RecordBatch, String> {
    if value.parameters.is_empty() {
        return Err("omnibus permutation contains no inferential targets".into());
    }
    if value.parameters.iter().any(|row| {
        row.null_maximum_pairwise_spreads.len() != value.usable
            || row
                .null_maximum_pairwise_spreads
                .iter()
                .any(|spread| !spread.is_finite() || *spread < 0.0)
    }) {
        return Err(
            "omnibus permutation null vectors disagree with the usable ledger or contain invalid spreads"
                .into(),
        );
    }
    let row_count = value
        .requested
        .checked_mul(value.parameters.len())
        .ok_or_else(|| "omnibus null target row count overflows usize".to_owned())?;
    let mut replicate_index = Vec::with_capacity(row_count);
    let mut target_id = Vec::with_capacity(row_count);
    let mut estimates = Vec::with_capacity(row_count);
    let mut valid = Vec::with_capacity(row_count);
    let mut failure_code = Vec::with_capacity(row_count);
    let mut usable_index = 0usize;
    for ledger in &value.ledger {
        let usable = ledger.status == ResampleFitStatusV1::Usable;
        for parameter in &value.parameters {
            replicate_index.push(u32_checked(ledger.replicate, "omnibus null replicate")?);
            target_id.push(parameter.parameter.stable_id.clone());
            estimates.push(if usable {
                parameter.null_maximum_pairwise_spreads[usable_index]
            } else {
                0.0
            });
            valid.push(usable);
            failure_code.push(if usable {
                String::new()
            } else {
                "omnibus_refit_failed".into()
            });
        }
        if usable {
            usable_index += 1;
        }
    }
    multimod_target_ledger_batch_v1(replicate_index, target_id, estimates, valid, failure_code)
        .map_err(|error| error.to_string())
}

fn encode_pairwise_permutation_v1(
    value: &PairwisePermutationResultV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_pairwise_permutation",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("group_a", group_u8(value.pair.group_a))?,
            metadata_u64("group_b", group_u8(value.pair.group_b))?,
            metadata_u64("seed", value.seed)?,
            metadata_u64("requested", value.requested)?,
            metadata_u64("attempted", value.attempted)?,
            metadata_u64("usable", value.usable)?,
            metadata_u64("failed", value.failed)?,
            metadata_u64("minimum_usable", value.minimum_usable)?,
            metadata_string("retry_policy", value.retry_policy.clone()),
            metadata_string("plan_sha256", value.plan_sha256.clone()),
            metadata_string("availability", stable_enum_id_v1(&value.availability)?),
        ],
    )?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("ledger", permutation_ledger_batch_v1(&value.ledger)?),
    ];
    if !value.point_estimates.is_empty() {
        tables.push(table_v1(
            "point-estimates",
            pairwise_point_parameter_batch_v1(value)?,
        ));
    }
    if !value.parameters.is_empty() {
        tables.push(table_v1(
            "inference",
            pairwise_inference_parameter_batch_v1(value)?,
        ));
        tables.push(table_v1(
            "null-target-vectors",
            pairwise_null_target_batch_v1(value)?,
        ));
    }
    Ok(tables)
}

fn encode_omnibus_permutation_v1(
    value: &OmnibusPermutationResultV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_omnibus_permutation",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("seed", value.seed)?,
            metadata_u64("requested", value.requested)?,
            metadata_u64("attempted", value.attempted)?,
            metadata_u64("usable", value.usable)?,
            metadata_u64("failed", value.failed)?,
            metadata_u64("minimum_usable", value.minimum_usable)?,
            metadata_string("retry_policy", value.retry_policy.clone()),
            metadata_string("plan_sha256", value.plan_sha256.clone()),
            metadata_string("availability", stable_enum_id_v1(&value.availability)?),
        ],
    )?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("ledger", permutation_ledger_batch_v1(&value.ledger)?),
    ];
    if !value.group_point_estimates.is_empty() {
        let mut groups = Vec::new();
        let mut parameter_index = Vec::new();
        let mut parameter_id = Vec::<Option<String>>::new();
        let mut parameter_family = Vec::<Option<String>>::new();
        let mut estimate = Vec::new();
        for group in &value.group_point_estimates {
            for (index, observed) in group.values.iter().enumerate() {
                let parameter = value.parameters.get(index).map(|row| &row.parameter);
                groups.push(group_u8(group.group));
                parameter_index.push(u32_checked(index, "omnibus parameter index")?);
                parameter_id.push(parameter.map(|value| value.stable_id.clone()));
                parameter_family
                    .push(parameter.map(|value| parameter_family_id_v1(value.family).to_owned()));
                estimate.push(*observed);
            }
        }
        let schema = Arc::new(Schema::new(vec![
            Field::new("group_index", DataType::UInt8, false),
            Field::new("parameter_index", DataType::UInt32, false),
            Field::new("parameter_id", DataType::Utf8, true),
            Field::new("parameter_family", DataType::Utf8, true),
            Field::new("estimate", DataType::Float64, false),
        ]));
        tables.push(table_v1(
            "group-point-estimates",
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(UInt8Array::from(groups)),
                    Arc::new(UInt32Array::from(parameter_index)),
                    Arc::new(StringArray::from(parameter_id)),
                    Arc::new(StringArray::from(parameter_family)),
                    Arc::new(Float64Array::from(estimate)),
                ],
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    if !value.parameters.is_empty() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("parameter_id", DataType::Utf8, false),
            Field::new("parameter_family", DataType::Utf8, false),
            Field::new("observed_maximum_pairwise_spread", DataType::Float64, false),
            Field::new("p_value_right_tailed", DataType::Float64, false),
        ]));
        tables.push(table_v1(
            "inference",
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(
                        value
                            .parameters
                            .iter()
                            .map(|row| row.parameter.stable_id.clone())
                            .collect::<Vec<_>>(),
                    )),
                    Arc::new(StringArray::from(
                        value
                            .parameters
                            .iter()
                            .map(|row| parameter_family_id_v1(row.parameter.family))
                            .collect::<Vec<_>>(),
                    )),
                    Arc::new(Float64Array::from(
                        value
                            .parameters
                            .iter()
                            .map(|row| row.observed_maximum_pairwise_spread)
                            .collect::<Vec<_>>(),
                    )),
                    Arc::new(Float64Array::from(
                        value
                            .parameters
                            .iter()
                            .map(|row| row.p_value_right_tailed)
                            .collect::<Vec<_>>(),
                    )),
                ],
            )
            .map_err(|error| error.to_string())?,
        ));
        tables.push(table_v1(
            "null-target-vectors",
            omnibus_null_target_batch_v1(value)?,
        ));
    }
    Ok(tables)
}

fn encode_group_bootstrap_banks_v1(
    value: &GroupBootstrapBanksV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_bootstrap_banks",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("seed", value.seed)?,
            metadata_u64("requested", value.requested)?,
            metadata_u64("attempted", value.attempted)?,
            metadata_u64("minimum_usable", value.minimum_usable)?,
            metadata_string("retry_policy", value.retry_policy.clone()),
            metadata_string("plan_sha256", value.plan_sha256.clone()),
            metadata_string("availability", stable_enum_id_v1(&value.availability)?),
        ],
    )?;
    let mut ledger_replicate = Vec::new();
    let mut ledger_usable = Vec::new();
    let mut ledger_group = Vec::new();
    let mut ledger_group_usable = Vec::new();
    let mut ledger_sample_sha256 = Vec::new();
    let mut ledger_failure_code = Vec::new();
    let mut ledger_failure_detail = Vec::new();
    let mut failure_by_cell = BTreeMap::<(usize, u8), String>::new();
    for entry in &value.ledger {
        for group in &entry.groups {
            let failure_code = group
                .failure
                .as_ref()
                .map(|failure| refit_failure_code_v1(failure.code))
                .transpose()?
                .unwrap_or_default();
            ledger_replicate.push(u32_checked(entry.replicate, "bootstrap replicate")?);
            ledger_usable.push(entry.status == ResampleFitStatusV1::Usable);
            ledger_group.push(group_u8(group.group));
            ledger_group_usable.push(group.status == ResampleFitStatusV1::Usable);
            ledger_sample_sha256.push(group.sample_sha256.clone());
            ledger_failure_code.push(failure_code.clone());
            ledger_failure_detail.push(
                group
                    .failure
                    .as_ref()
                    .map(|failure| failure.detail.clone())
                    .unwrap_or_default(),
            );
            failure_by_cell.insert((entry.replicate, group_u8(group.group)), failure_code);
        }
    }
    if ledger_replicate.is_empty() {
        return Err("group bootstrap ledger contains no rows".into());
    }
    let ledger_schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("replicate_usable", DataType::Boolean, false),
        Field::new("group_index", DataType::UInt8, false),
        Field::new("group_usable", DataType::Boolean, false),
        Field::new("sample_sha256", DataType::Utf8, false),
        Field::new("failure_code", DataType::Utf8, false),
        Field::new("failure_detail", DataType::Utf8, false),
    ]));
    let ledger = RecordBatch::try_new(
        ledger_schema,
        vec![
            Arc::new(UInt32Array::from(ledger_replicate)),
            Arc::new(BooleanArray::from(ledger_usable)),
            Arc::new(UInt8Array::from(ledger_group)),
            Arc::new(BooleanArray::from(ledger_group_usable)),
            Arc::new(StringArray::from(ledger_sample_sha256)),
            Arc::new(StringArray::from(ledger_failure_code)),
            Arc::new(StringArray::from(ledger_failure_detail)),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut replicate_index = Vec::new();
    let mut target_id = Vec::new();
    let mut target_value = Vec::new();
    let mut target_valid = Vec::new();
    let mut target_failure = Vec::new();
    let mut point_group = Vec::new();
    let mut point_parameter_id = Vec::new();
    let mut point_parameter_family = Vec::new();
    let mut point_estimate = Vec::new();
    for bank in &value.groups {
        if bank.point_estimates.len() != value.parameters.len()
            || bank.replicate_estimates.len() != value.requested
        {
            return Err(
                "group bootstrap bank has incompatible parameter or replicate dimensions".into(),
            );
        }
        for (parameter, estimate) in value.parameters.iter().zip(&bank.point_estimates) {
            point_group.push(group_u8(bank.group));
            point_parameter_id.push(parameter.stable_id.clone());
            point_parameter_family.push(parameter_family_id_v1(parameter.family));
            point_estimate.push(*estimate);
        }
        for (replicate, estimates) in bank.replicate_estimates.iter().enumerate() {
            if estimates
                .as_ref()
                .is_some_and(|estimates| estimates.len() != value.parameters.len())
            {
                return Err("group bootstrap replicate target width changed".into());
            }
            for (parameter_index, parameter) in value.parameters.iter().enumerate() {
                let observed = estimates.as_ref().map(|values| values[parameter_index]);
                let valid = observed.is_some();
                replicate_index.push(u32_checked(replicate, "bootstrap target replicate")?);
                target_id.push(format!(
                    "group:{}:{}",
                    group_u8(bank.group),
                    parameter.stable_id
                ));
                target_value.push(observed.unwrap_or_default());
                target_valid.push(valid);
                let ledger_code = failure_by_cell
                    .get(&(replicate, group_u8(bank.group)))
                    .cloned()
                    .unwrap_or_else(|| "missing_group_ledger".into());
                target_failure.push(if valid {
                    String::new()
                } else if ledger_code.is_empty() {
                    "missing_target_estimate".into()
                } else {
                    ledger_code
                });
            }
        }
    }
    let target_ledger = multimod_target_ledger_batch_v1(
        replicate_index,
        target_id,
        target_value,
        target_valid,
        target_failure,
    )
    .map_err(|error| error.to_string())?;
    let point_schema = Arc::new(Schema::new(vec![
        Field::new("group_index", DataType::UInt8, false),
        Field::new("parameter_id", DataType::Utf8, false),
        Field::new("parameter_family", DataType::Utf8, false),
        Field::new("estimate", DataType::Float64, false),
    ]));
    let point = RecordBatch::try_new(
        point_schema,
        vec![
            Arc::new(UInt8Array::from(point_group)),
            Arc::new(StringArray::from(point_parameter_id)),
            Arc::new(StringArray::from(point_parameter_family)),
            Arc::new(Float64Array::from(point_estimate)),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("ledger", ledger),
        table_v1("target-vectors", target_ledger),
        table_v1("point-estimates", point),
    ])
}

fn encode_micom_pair_v1(value: &MicomPairwiseResultV1) -> Result<Vec<EvidenceTableV1>, String> {
    let receipt = &value.configural_receipt;
    let metadata = metadata_batch_v1(
        "mga_micom_pair",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_u64("group_a", group_u8(value.pair.group_a))?,
            metadata_u64("group_b", group_u8(value.pair.group_b))?,
            metadata_bool(
                "configural.identical_indicators_and_coding",
                receipt.identical_indicators_and_coding,
            ),
            metadata_bool(
                "configural.identical_data_treatment",
                receipt.identical_data_treatment,
            ),
            metadata_bool(
                "configural.identical_algorithm_settings",
                receipt.identical_algorithm_settings,
            ),
            metadata_bool(
                "configural.identical_model_specification",
                receipt.identical_model_specification,
            ),
            metadata_bool(
                "configural.deterministic_orientation_reviewed",
                receipt.deterministic_orientation_reviewed,
            ),
            metadata_bool(
                "configural.analyst_review_confirmed",
                receipt.analyst_review_confirmed,
            ),
            metadata_u64("requested_permutations", value.requested_permutations)?,
            metadata_u64("usable_permutations", value.usable_permutations)?,
            metadata_u64(
                "minimum_usable_permutations",
                value.minimum_usable_permutations,
            )?,
            metadata_string("partition_plan_sha256", value.partition_plan_sha256.clone()),
            metadata_string("ledger_sha256", value.ledger_sha256.clone()),
            metadata_bool("complete", value.complete),
        ],
    )?;
    let mut replicate_index = Vec::new();
    let mut seed = Vec::new();
    let mut usable = Vec::new();
    let mut failure_code = Vec::new();
    let mut shard_id = Vec::new();
    for entry in &value.ledger {
        replicate_index.push(u32_checked(entry.replicate, "MICOM replicate")?);
        seed.push(entry.seed);
        shard_id.push(format!("micom:{}", entry.partition_sha256));
        match &entry.status {
            qpls_estimation::MicomPermutationStatusV1::Usable => {
                usable.push(true);
                failure_code.push(String::new());
            }
            qpls_estimation::MicomPermutationStatusV1::Failed { code, .. } => {
                usable.push(false);
                failure_code.push(refit_failure_code_v1(*code)?);
            }
        }
    }
    let ledger =
        multimod_resample_ledger_batch_v1(replicate_index, seed, usable, failure_code, shard_id)
            .map_err(|error| error.to_string())?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("construct_id", DataType::Utf8, false),
        Field::new(
            "observed_compositional_correlation",
            DataType::Float64,
            false,
        ),
        Field::new("compositional_lower_quantile", DataType::Float64, true),
        Field::new(
            "compositional_invariance_probability",
            DataType::Float64,
            true,
        ),
        Field::new("compositional_invariance", DataType::Boolean, false),
        Field::new(
            "observed_mean_difference_a_minus_b",
            DataType::Float64,
            false,
        ),
        Field::new(
            "mean_difference_two_sided_probability",
            DataType::Float64,
            true,
        ),
        Field::new("equal_means", DataType::Boolean, false),
        Field::new(
            "observed_log_variance_ratio_a_minus_b",
            DataType::Float64,
            false,
        ),
        Field::new(
            "variance_difference_two_sided_probability",
            DataType::Float64,
            true,
        ),
        Field::new("equal_variances", DataType::Boolean, false),
        Field::new("partial_measurement_invariance", DataType::Boolean, false),
        Field::new("full_measurement_invariance", DataType::Boolean, false),
    ]));
    let constructs = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.construct_id.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.observed_compositional_correlation)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.compositional_lower_quantile)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.compositional_invariance_probability)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.compositional_invariance)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.observed_mean_difference_a_minus_b)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.mean_difference_two_sided_probability)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.equal_means)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.observed_log_variance_ratio_a_minus_b)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.variance_difference_two_sided_probability)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.equal_variances)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.partial_measurement_invariance)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(BooleanArray::from(
                value
                    .constructs
                    .iter()
                    .map(|row| row.full_measurement_invariance)
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    let usable_replicates = value
        .ledger
        .iter()
        .filter_map(|entry| {
            (entry.status == qpls_estimation::MicomPermutationStatusV1::Usable)
                .then_some(entry.replicate)
        })
        .collect::<Vec<_>>();
    if value.constructs.iter().any(|construct| {
        construct.permutation_compositional_correlations.len() != usable_replicates.len()
            || (!construct.permutation_mean_differences.is_empty()
                && construct.permutation_mean_differences.len() != usable_replicates.len())
            || (!construct.permutation_log_variance_ratios.is_empty()
                && construct.permutation_log_variance_ratios.len() != usable_replicates.len())
    }) || value
        .constructs
        .iter()
        .filter(|construct| !construct.permutation_mean_differences.is_empty())
        .count()
        != 1
    {
        return Err("MICOM null-statistic vectors disagree with the usable ledger".into());
    }
    let mut null_replicate_index = Vec::new();
    let mut null_construct_index = Vec::new();
    let mut null_statistic_kind = Vec::new();
    let mut null_value = Vec::new();
    for (construct_index, construct) in value.constructs.iter().enumerate() {
        for (statistic_kind, values) in [
            (
                MULTIMOD_MICOM_COMPOSITIONAL_CORRELATION_STATISTIC_V1,
                &construct.permutation_compositional_correlations,
            ),
            (
                MULTIMOD_MICOM_MEAN_DIFFERENCE_STATISTIC_V1,
                &construct.permutation_mean_differences,
            ),
            (
                MULTIMOD_MICOM_LOG_VARIANCE_RATIO_STATISTIC_V1,
                &construct.permutation_log_variance_ratios,
            ),
        ] {
            if values.is_empty() {
                continue;
            }
            for (replicate, statistic) in usable_replicates.iter().zip(values) {
                null_replicate_index
                    .push(u32_checked(*replicate, "MICOM null-statistic replicate")?);
                null_construct_index.push(u32_checked(
                    construct_index,
                    "MICOM null-statistic construct index",
                )?);
                null_statistic_kind.push(statistic_kind);
                null_value.push(*statistic);
            }
        }
    }
    let null_statistics = multimod_micom_null_statistics_batch_v1(
        null_replicate_index,
        null_construct_index,
        null_statistic_kind,
        null_value,
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("ledger", ledger),
        table_v1("constructs", constructs),
        table_v1("null-statistics", null_statistics),
    ])
}

fn encode_ordinary_pls_path_standard_error_v1(
    parameter: &ParameterIdentityV1,
    group: GroupIndexV1,
    value: &OrdinaryPlsPathStandardErrorV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_ordinary_pls_path_standard_error",
        vec![
            metadata_string("parameter_id", parameter.stable_id.clone()),
            metadata_string("parameter_family", parameter_family_id_v1(parameter.family)),
            metadata_u64("group_index", group_u8(group))?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string("source", value.source.clone()),
            metadata_string("target", value.target.clone()),
            metadata_number("estimate", value.estimate)?,
            metadata_number("standard_error", value.standard_error)?,
            metadata_u64("observations", value.observations)?,
            metadata_u64("predictor_count", value.predictor_count)?,
            metadata_number(
                "variance_degrees_of_freedom",
                value.variance_degrees_of_freedom,
            )?,
            metadata_number("residual_sum_of_squares", value.residual_sum_of_squares)?,
            metadata_number(
                "coefficient_variance_factor",
                value.coefficient_variance_factor,
            )?,
        ],
    )?;
    Ok(vec![table_v1("metadata", metadata)])
}

fn encode_pairwise_parametric_v1(
    value: &PairwiseParametricTestV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_pairwise_parametric",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_string("method", stable_enum_id_v1(&value.method)?),
            metadata_u64("group_a", group_u8(value.pair.group_a))?,
            metadata_u64("group_b", group_u8(value.pair.group_b))?,
            metadata_number("difference_a_minus_b", value.difference_a_minus_b)?,
            metadata_number(
                "standard_error_of_difference",
                value.standard_error_of_difference,
            )?,
            metadata_number("t_statistic", value.t_statistic)?,
            metadata_number("degrees_of_freedom", value.degrees_of_freedom)?,
            metadata_number("p_value_two_sided", value.p_value_two_sided)?,
            metadata_number("p_value_greater", value.p_value_greater)?,
            metadata_number("p_value_less", value.p_value_less)?,
            metadata_string(
                "selected_alternative",
                stable_enum_id_v1(&value.selected_alternative)?,
            ),
            metadata_number("selected_probability", value.selected_probability)?,
        ],
    )?;
    Ok(vec![table_v1("metadata", metadata)])
}

fn encode_parametric_wald_v1(
    value: &InverseVarianceWaldResultV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "mga_parametric_wald",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_number(
                "inverse_variance_weighted_mean",
                value.inverse_variance_weighted_mean,
            )?,
            metadata_number("chi_square", value.chi_square)?,
            metadata_u64("degrees_of_freedom", value.degrees_of_freedom)?,
            metadata_number("p_value_right_tailed", value.p_value_right_tailed)?,
        ],
    )?;
    let schema = Arc::new(Schema::new(vec![Field::new(
        "group_index",
        DataType::UInt8,
        false,
    )]));
    let groups = RecordBatch::try_new(
        schema,
        vec![Arc::new(UInt8Array::from(
            value
                .groups
                .iter()
                .map(|group| group_u8(*group))
                .collect::<Vec<_>>(),
        ))],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("groups", groups),
    ])
}

fn require_analysis_row_tokens_v1<'a>(
    row_tokens: Option<&'a [String]>,
    observations: usize,
    subject: &str,
) -> Result<&'a [String], String> {
    let row_tokens = row_tokens.ok_or_else(|| {
        format!("{subject} is missing its raw-preparation source-row token receipt")
    })?;
    if row_tokens.len() != observations {
        return Err(format!(
            "{subject} has {} analysis rows but {} source-row tokens",
            observations,
            row_tokens.len()
        ));
    }
    Ok(row_tokens)
}

fn encode_fimix_candidate_v1(
    k: u8,
    value: &FimixPlsV2Result,
    analysis_row_tokens: Option<&[String]>,
) -> Result<Vec<EvidenceTableV1>, String> {
    validate_fimix_multistart_evidence_v2(value).map_err(|error| error.to_string())?;
    let row_tokens =
        require_analysis_row_tokens_v1(analysis_row_tokens, value.observations, "FIMIX candidate")?;
    if value.classes.len() != usize::from(k)
        || value.posteriors.len() != value.observations
        || value.hard_assignments.len() != value.observations
    {
        return Err("FIMIX candidate class or observation dimensions are inconsistent".into());
    }
    let metadata = metadata_batch_v1(
        "fimix_candidate",
        vec![
            metadata_u64("k", k)?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "interaction_profile",
                stable_enum_id_v1(&value.interaction_profile)?,
            ),
            metadata_string("metric.metric_id", value.metric.metric_id.clone()),
            metadata_string("metric.source_sha256", value.metric.source_sha256.clone()),
            metadata_u64("metric.observation_count", value.metric.observation_count)?,
            metadata_bool(
                "metric.scores_standardized_once_on_pooled_rows",
                value.metric.scores_standardized_once_on_pooled_rows,
            ),
            metadata_bool(
                "metric.products_standardized_once_on_pooled_rows",
                value.metric.products_standardized_once_on_pooled_rows,
            ),
            metadata_u64("observations", value.observations)?,
            metadata_u64("selected_start_index", value.selected_start_index)?,
            metadata_u64("iterations", value.iterations)?,
            metadata_number("log_likelihood", value.log_likelihood)?,
            metadata_u64("criteria.parameter_count", value.criteria.parameter_count)?,
            metadata_number("criteria.aic", value.criteria.aic)?,
            metadata_number("criteria.aic3", value.criteria.aic3)?,
            metadata_number("criteria.aic4", value.criteria.aic4)?,
            metadata_number("criteria.bic", value.criteria.bic)?,
            metadata_number("criteria.caic", value.criteria.caic)?,
            metadata_number("criteria.hq", value.criteria.hq)?,
            metadata_number("entropy.raw", value.entropy.raw)?,
            metadata_number(
                "entropy.normalized_certainty",
                value.entropy.normalized_certainty,
            )?,
            metadata_u64(
                "minimum_effective_class_size",
                value.minimum_effective_class_size,
            )?,
            metadata_u64(
                "stability.required_reproducing_starts",
                value.stability.required_reproducing_starts,
            )?,
            metadata_number(
                "stability.maximum_aligned_coefficient_difference",
                value.stability.maximum_aligned_coefficient_difference,
            )?,
            metadata_number(
                "stability.maximum_aligned_mean_posterior_difference",
                value.stability.maximum_aligned_mean_posterior_difference,
            )?,
            metadata_bool("stability.stable", value.stability.stable),
            metadata_u64(
                "multistart_evidence.schema_version",
                value.multistart_evidence.schema_version,
            )?,
            metadata_number(
                "multistart_evidence.relative_log_likelihood_tolerance",
                value.multistart_evidence.relative_log_likelihood_tolerance,
            )?,
            metadata_number(
                "multistart_evidence.maximum_coefficient_difference_tolerance",
                value
                    .multistart_evidence
                    .maximum_coefficient_difference_tolerance,
            )?,
            metadata_number(
                "multistart_evidence.mean_posterior_difference_tolerance",
                value
                    .multistart_evidence
                    .mean_posterior_difference_tolerance,
            )?,
        ],
    )?;

    let mut posterior_row = Vec::with_capacity(value.observations * usize::from(k));
    let mut posterior_token = Vec::with_capacity(value.observations * usize::from(k));
    let mut posterior_class = Vec::with_capacity(value.observations * usize::from(k));
    let mut posterior_probability = Vec::with_capacity(value.observations * usize::from(k));
    for (row_index, posterior) in value.posteriors.iter().enumerate() {
        if posterior.len() != usize::from(k) {
            return Err("FIMIX posterior row changed class width".into());
        }
        for (class_index, probability) in posterior.iter().enumerate() {
            posterior_row.push(u32_checked(row_index, "FIMIX posterior row")?);
            posterior_token.push(row_tokens[row_index].clone());
            posterior_class.push(
                u8::try_from(class_index)
                    .map_err(|_| "FIMIX class index exceeds UInt8".to_owned())?,
            );
            posterior_probability.push(*probability);
        }
    }
    let posteriors = multimod_membership_with_row_tokens_batch_v1(
        posterior_row,
        posterior_token,
        posterior_class,
        posterior_probability,
    )
    .map_err(|error| error.to_string())?;

    let assignment_schema = Arc::new(Schema::new(vec![
        Field::new("row_index", DataType::UInt32, false),
        Field::new("stable_row_token", DataType::Utf8, false),
        Field::new("hard_class_id", DataType::UInt8, false),
    ]));
    let assignments = RecordBatch::try_new(
        assignment_schema,
        vec![
            Arc::new(UInt32Array::from(
                (0..value.observations)
                    .map(|row| u32_checked(row, "FIMIX assignment row"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(row_tokens.to_vec())),
            Arc::new(UInt8Array::from(
                value
                    .hard_assignments
                    .iter()
                    .map(|class| {
                        if *class >= usize::from(k) {
                            Err("FIMIX hard assignment is outside K".to_owned())
                        } else {
                            u8::try_from(*class)
                                .map_err(|_| "FIMIX hard assignment exceeds UInt8".to_owned())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut class_id = Vec::new();
    let mut class_proportion = Vec::new();
    let mut class_effective_observations = Vec::new();
    let mut equation_id = Vec::new();
    let mut outcome_id = Vec::new();
    let mut parameter_id = Vec::new();
    let mut coefficient = Vec::new();
    let mut residual_variance = Vec::new();
    for class in &value.classes {
        for equation in &class.equations {
            for parameter in &equation.coefficients {
                class_id.push(class.class_id.clone());
                class_proportion.push(class.proportion);
                class_effective_observations.push(class.effective_observations);
                equation_id.push(equation.equation_id.clone());
                outcome_id.push(equation.outcome_id.clone());
                parameter_id.push(parameter.parameter_id.clone());
                coefficient.push(parameter.estimate);
                residual_variance.push(equation.residual_variance);
            }
        }
    }
    let coefficient_schema = Arc::new(Schema::new(vec![
        Field::new("class_id", DataType::Utf8, false),
        Field::new("class_proportion", DataType::Float64, false),
        Field::new("class_effective_observations", DataType::Float64, false),
        Field::new("equation_id", DataType::Utf8, false),
        Field::new("outcome_id", DataType::Utf8, false),
        Field::new("parameter_id", DataType::Utf8, false),
        Field::new("coefficient", DataType::Float64, false),
        Field::new("residual_variance", DataType::Float64, false),
    ]));
    let coefficients = RecordBatch::try_new(
        coefficient_schema,
        vec![
            Arc::new(StringArray::from(class_id)),
            Arc::new(Float64Array::from(class_proportion)),
            Arc::new(Float64Array::from(class_effective_observations)),
            Arc::new(StringArray::from(equation_id)),
            Arc::new(StringArray::from(outcome_id)),
            Arc::new(StringArray::from(parameter_id)),
            Arc::new(Float64Array::from(coefficient)),
            Arc::new(Float64Array::from(residual_variance)),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut start_index = Vec::new();
    let mut start_seed = Vec::new();
    let mut start_converged = Vec::new();
    let mut start_iterations = Vec::new();
    let mut start_final_objective = Vec::new();
    let mut start_maximum_decrease = Vec::new();
    let mut start_failure_code = Vec::<Option<String>>::new();
    let mut start_failure_message = Vec::<Option<String>>::new();
    let mut trace_algorithm = Vec::new();
    let mut trace_k = Vec::new();
    let mut trace_start = Vec::new();
    let mut trace_iteration = Vec::new();
    let mut trace_objective = Vec::new();
    let mut trace_converged = Vec::new();
    let mut trace_failure_code = Vec::new();
    let mut effective_start = Vec::new();
    let mut effective_class = Vec::new();
    let mut effective_observations = Vec::new();
    for start in &value.starts {
        let failure_code = start
            .failure_code
            .as_ref()
            .map(stable_enum_id_v1)
            .transpose()?;
        start_index.push(u32_checked(start.start_index, "FIMIX start index")?);
        start_seed.push(start.start_seed);
        start_converged.push(start.converged);
        start_iterations.push(u32_checked(start.iterations, "FIMIX start iterations")?);
        start_final_objective.push(start.final_log_likelihood);
        start_maximum_decrease.push(start.maximum_likelihood_decrease);
        start_failure_code.push(failure_code.clone());
        start_failure_message.push(start.failure_message.clone());
        for (class_index, observed) in start.final_effective_class_sizes.iter().enumerate() {
            effective_start.push(u32_checked(
                start.start_index,
                "FIMIX effective-size start",
            )?);
            effective_class.push(
                u8::try_from(class_index)
                    .map_err(|_| "FIMIX effective-size class exceeds UInt8".to_owned())?,
            );
            effective_observations.push(*observed);
        }
        for trace in &start.trace {
            trace_algorithm.push("fimix_pls_v2".to_owned());
            trace_k.push(k);
            trace_start.push(u32_checked(start.start_index, "FIMIX trace start")?);
            trace_iteration.push(u32_checked(trace.iteration, "FIMIX trace iteration")?);
            trace_objective.push(trace.log_likelihood);
            trace_converged.push(start.converged);
            trace_failure_code.push(failure_code.clone().unwrap_or_default());
        }
    }
    let start_schema = Arc::new(Schema::new(vec![
        Field::new("start_index", DataType::UInt32, false),
        Field::new("start_seed", DataType::UInt64, false),
        Field::new("converged", DataType::Boolean, false),
        Field::new("iterations", DataType::UInt32, false),
        Field::new("final_log_likelihood", DataType::Float64, true),
        Field::new("maximum_likelihood_decrease", DataType::Float64, false),
        Field::new("failure_code", DataType::Utf8, true),
        Field::new("failure_message", DataType::Utf8, true),
    ]));
    let starts = RecordBatch::try_new(
        start_schema,
        vec![
            Arc::new(UInt32Array::from(start_index)),
            Arc::new(UInt64Array::from(start_seed)),
            Arc::new(BooleanArray::from(start_converged)),
            Arc::new(UInt32Array::from(start_iterations)),
            Arc::new(Float64Array::from(start_final_objective)),
            Arc::new(Float64Array::from(start_maximum_decrease)),
            Arc::new(StringArray::from(start_failure_code)),
            Arc::new(StringArray::from(start_failure_message)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let effective_schema = Arc::new(Schema::new(vec![
        Field::new("start_index", DataType::UInt32, false),
        Field::new("class_id", DataType::UInt8, false),
        Field::new("effective_observations", DataType::Float64, false),
    ]));
    let effective_sizes = RecordBatch::try_new(
        effective_schema,
        vec![
            Arc::new(UInt32Array::from(effective_start)),
            Arc::new(UInt8Array::from(effective_class)),
            Arc::new(Float64Array::from(effective_observations)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let reproducing_schema = Arc::new(Schema::new(vec![Field::new(
        "start_index",
        DataType::UInt32,
        false,
    )]));
    let reproducing = RecordBatch::try_new(
        reproducing_schema,
        vec![Arc::new(UInt32Array::from(
            value
                .stability
                .reproducing_start_indices
                .iter()
                .map(|index| u32_checked(*index, "FIMIX reproducing start"))
                .collect::<Result<Vec<_>, _>>()?,
        ))],
    )
    .map_err(|error| error.to_string())?;

    let completed = &value.multistart_evidence.completed_starts;
    let receipt_start = completed
        .iter()
        .map(|start| u32_checked(start.start_index, "FIMIX multistart receipt"))
        .collect::<Result<Vec<_>, _>>()?;
    let receipts = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("fit_statistic", DataType::Float64, false),
            Field::new("partition_sha256", DataType::Utf8, false),
            Field::new("numeric_signature_sha256", DataType::Utf8, false),
            Field::new("posterior_sha256", DataType::Utf8, true),
            Field::new("fit_statistic_sha256", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(receipt_start)),
            Arc::new(Float64Array::from(
                completed
                    .iter()
                    .map(|start| start.final_log_likelihood)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.partition_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.coefficient_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| Some(start.posterior_sha256.clone()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.fit_statistic_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut retained_assignment_start = Vec::new();
    let mut retained_assignment_row = Vec::new();
    let mut retained_assignment_class = Vec::new();
    let mut retained_signature_start = Vec::new();
    let mut retained_signature_class = Vec::new();
    let mut retained_signature_index = Vec::new();
    let mut retained_signature_value = Vec::new();
    let mut retained_posterior_start = Vec::new();
    let mut retained_posterior_row = Vec::new();
    let mut retained_posterior_class = Vec::new();
    let mut retained_posterior_value = Vec::new();
    for start in completed {
        let start_index = u32_checked(start.start_index, "FIMIX retained start")?;
        for (row_index, class) in start.canonical_hard_assignments.iter().enumerate() {
            retained_assignment_start.push(start_index);
            retained_assignment_row.push(u32_checked(row_index, "FIMIX retained assignment")?);
            retained_assignment_class.push(
                u8::try_from(*class)
                    .map_err(|_| "FIMIX retained class exceeds UInt8".to_owned())?,
            );
        }
        for (class_index, signature) in start.canonical_coefficient_signatures.iter().enumerate() {
            for (parameter_index, observed) in signature.iter().enumerate() {
                retained_signature_start.push(start_index);
                retained_signature_class.push(
                    u8::try_from(class_index)
                        .map_err(|_| "FIMIX retained class exceeds UInt8".to_owned())?,
                );
                retained_signature_index.push(u32_checked(
                    parameter_index,
                    "FIMIX retained coefficient index",
                )?);
                retained_signature_value.push(*observed);
            }
        }
        for (row_index, posterior) in start.canonical_posteriors.iter().enumerate() {
            for (class_index, probability) in posterior.iter().enumerate() {
                retained_posterior_start.push(start_index);
                retained_posterior_row
                    .push(u32_checked(row_index, "FIMIX retained posterior row")?);
                retained_posterior_class.push(
                    u8::try_from(class_index)
                        .map_err(|_| "FIMIX retained posterior class exceeds UInt8".to_owned())?,
                );
                retained_posterior_value.push(*probability);
            }
        }
    }
    let retained_assignments = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("row_index", DataType::UInt32, false),
            Field::new("class_id", DataType::UInt8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_assignment_start)),
            Arc::new(UInt32Array::from(retained_assignment_row)),
            Arc::new(UInt8Array::from(retained_assignment_class)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let retained_signatures = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("class_id", DataType::UInt8, false),
            Field::new("parameter_index", DataType::UInt32, false),
            Field::new("value", DataType::Float64, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_signature_start)),
            Arc::new(UInt8Array::from(retained_signature_class)),
            Arc::new(UInt32Array::from(retained_signature_index)),
            Arc::new(Float64Array::from(retained_signature_value)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let retained_posteriors = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("row_index", DataType::UInt32, false),
            Field::new("class_id", DataType::UInt8, false),
            Field::new("posterior_probability", DataType::Float64, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_posterior_start)),
            Arc::new(UInt32Array::from(retained_posterior_row)),
            Arc::new(UInt8Array::from(retained_posterior_class)),
            Arc::new(Float64Array::from(retained_posterior_value)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("posteriors", posteriors),
        table_v1("hard-assignments", assignments),
        table_v1("class-coefficients", coefficients),
        table_v1("start-diagnostics", starts),
        table_v1("start-effective-sizes", effective_sizes),
        table_v1("reproducing-starts", reproducing),
        table_v1("multistart-receipts", receipts),
        table_v1("multistart-assignments", retained_assignments),
        table_v1("multistart-coefficient-signatures", retained_signatures),
        table_v1("multistart-posteriors", retained_posteriors),
    ];
    if !trace_algorithm.is_empty() {
        tables.push(table_v1(
            "start-traces",
            multimod_start_trace_batch_v1(
                trace_algorithm,
                trace_k,
                trace_start,
                trace_iteration,
                trace_objective,
                trace_converged,
                trace_failure_code,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn pos_scoring_contract_id_v1(value: PosScoringContractV2) -> Result<String, String> {
    match value {
        PosScoringContractV2::PublishedP0FullSegmentPls => {
            Ok("published_p0_full_segment_pls".into())
        }
        PosScoringContractV2::DestinationScoredInteractions { profile } => Ok(format!(
            "destination_scored_interactions:{}",
            stable_enum_id_v1(&profile)?
        )),
    }
}

fn encode_pls_pos_candidate_v1(
    k: u8,
    value: &PlsPosV2Result,
    analysis_row_tokens: Option<&[String]>,
) -> Result<Vec<EvidenceTableV1>, String> {
    validate_pos_multistart_evidence_v2(value).map_err(|error| error.to_string())?;
    let row_tokens = require_analysis_row_tokens_v1(
        analysis_row_tokens,
        value.observations,
        "PLS-POS candidate",
    )?;
    if value.assignments.len() != value.observations || value.segments.len() != usize::from(k) {
        return Err("PLS-POS candidate segment or observation dimensions are inconsistent".into());
    }
    let metadata = metadata_batch_v1(
        "pls_pos_candidate",
        vec![
            metadata_u64("k", k)?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "scoring_contract",
                pos_scoring_contract_id_v1(value.scoring_contract)?,
            ),
            metadata_u64("observations", value.observations)?,
            metadata_u64("selected_start_index", value.selected_start_index)?,
            metadata_number("objective", value.objective)?,
            metadata_string("objective_definition", value.objective_definition.clone()),
            metadata_u64("accepted_moves", value.accepted_moves)?,
            metadata_u64(
                "multistart_evidence.schema_version",
                value.multistart_evidence.schema_version,
            )?,
            metadata_u64(
                "multistart_evidence.required_reproducing_starts",
                value.multistart_evidence.required_reproducing_starts,
            )?,
            metadata_number(
                "multistart_evidence.objective_and_parameter_tolerance",
                value.multistart_evidence.objective_and_parameter_tolerance,
            )?,
        ],
    )?;

    let mut membership_row = Vec::with_capacity(value.observations * usize::from(k));
    let mut membership_token = Vec::with_capacity(value.observations * usize::from(k));
    let mut membership_class = Vec::with_capacity(value.observations * usize::from(k));
    let mut membership_probability = Vec::with_capacity(value.observations * usize::from(k));
    for (row_index, assignment) in value.assignments.iter().enumerate() {
        if *assignment >= usize::from(k) {
            return Err("PLS-POS assignment is outside K".into());
        }
        for class_index in 0..usize::from(k) {
            membership_row.push(u32_checked(row_index, "PLS-POS membership row")?);
            membership_token.push(row_tokens[row_index].clone());
            membership_class.push(
                u8::try_from(class_index)
                    .map_err(|_| "PLS-POS class index exceeds UInt8".to_owned())?,
            );
            membership_probability.push(if class_index == *assignment { 1.0 } else { 0.0 });
        }
    }
    let membership = multimod_membership_with_row_tokens_batch_v1(
        membership_row,
        membership_token,
        membership_class,
        membership_probability,
    )
    .map_err(|error| error.to_string())?;

    let mut segment_id = Vec::new();
    let mut segment_observations = Vec::new();
    let mut segment_objective = Vec::new();
    let mut receipt_method = Vec::new();
    let mut full_segment_pls_refit = Vec::new();
    let mut measurement_scores_reestimated = Vec::new();
    let mut score_orientation_reapplied = Vec::new();
    let mut interaction_stage_one_refit = Vec::new();
    let mut interaction_operands_restandardized = Vec::new();
    let mut interaction_products_rebuilt = Vec::new();
    let mut joint_structural_equations_refit = Vec::new();
    let mut r2_segment = Vec::new();
    let mut r2_outcome = Vec::new();
    let mut r2_value = Vec::new();
    let mut signature_segment = Vec::new();
    let mut signature_index = Vec::new();
    let mut signature_value = Vec::new();
    let mut audit_segment = Vec::new();
    let mut audit_outcome = Vec::new();
    let mut audit_source_row = Vec::new();
    let mut audit_observed = Vec::new();
    let mut audit_fitted = Vec::new();
    let mut audit_observed_mean = Vec::new();
    let mut audit_centered_sst = Vec::new();
    for segment in &value.segments {
        if segment.fit.outcome_fit_audits.len() != segment.fit.r_squared.len()
            || segment
                .fit
                .outcome_fit_audits
                .iter()
                .map(|audit| audit.outcome_id.as_str())
                .collect::<BTreeSet<_>>()
                != segment
                    .fit
                    .r_squared
                    .iter()
                    .map(|row| row.outcome_id.as_str())
                    .collect::<BTreeSet<_>>()
        {
            return Err(
                "PLS-POS outcome audit inventory differs from its R-squared inventory".into(),
            );
        }
        let receipt = &segment.fit.receipt;
        segment_id.push(segment.segment_id.clone());
        segment_observations.push(
            u64::try_from(segment.observations)
                .map_err(|_| "PLS-POS segment observations exceed UInt64".to_owned())?,
        );
        segment_objective.push(segment.objective_contribution);
        receipt_method.push(receipt.method_version.clone());
        full_segment_pls_refit.push(receipt.full_segment_pls_refit);
        measurement_scores_reestimated.push(receipt.measurement_scores_reestimated);
        score_orientation_reapplied.push(receipt.score_orientation_reapplied);
        interaction_stage_one_refit.push(receipt.interaction_stage_one_refit);
        interaction_operands_restandardized
            .push(receipt.interaction_operands_restandardized_within_destination);
        interaction_products_rebuilt.push(receipt.interaction_products_rebuilt_within_destination);
        joint_structural_equations_refit.push(receipt.joint_structural_equations_refit);
        for row in &segment.fit.r_squared {
            r2_segment.push(segment.segment_id.clone());
            r2_outcome.push(row.outcome_id.clone());
            r2_value.push(row.r_squared);
        }
        for (index, observed) in segment.fit.parameter_signature.iter().enumerate() {
            signature_segment.push(segment.segment_id.clone());
            signature_index.push(u32_checked(index, "PLS-POS signature index")?);
            signature_value.push(*observed);
        }
        for audit in &segment.fit.outcome_fit_audits {
            if audit.source_row_indices.len() != audit.observed_scores.len()
                || audit.observed_scores.len() != audit.fitted_scores.len()
                || audit.source_row_indices.len() != segment.observations
                || audit
                    .source_row_indices
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != segment.observations
                || audit
                    .observed_scores
                    .iter()
                    .chain(&audit.fitted_scores)
                    .any(|value| !value.is_finite())
            {
                return Err("PLS-POS outcome audit dimensions are inconsistent".into());
            }
            let observed_mean =
                audit.observed_scores.iter().sum::<f64>() / audit.observed_scores.len() as f64;
            let centered_sst = audit
                .observed_scores
                .iter()
                .map(|value| (value - observed_mean).powi(2))
                .sum::<f64>();
            let residual_sse = audit
                .observed_scores
                .iter()
                .zip(&audit.fitted_scores)
                .map(|(observed, fitted)| (observed - fitted).powi(2))
                .sum::<f64>();
            let reconstructed_r_squared = (1.0 - residual_sse / centered_sst).clamp(0.0, 1.0);
            let reported_r_squared = segment
                .fit
                .r_squared
                .iter()
                .find(|row| row.outcome_id == audit.outcome_id)
                .map(|row| row.r_squared);
            if observed_mean.abs() > POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2
                || !audit.observed_mean.is_finite()
                || (audit.observed_mean - observed_mean).abs() > 1.0e-12
                || !centered_sst.is_finite()
                || centered_sst <= f64::EPSILON
                || !audit.centered_total_sum_of_squares.is_finite()
                || (audit.centered_total_sum_of_squares - centered_sst).abs()
                    > 1.0e-12 * (1.0 + centered_sst.abs())
                || reported_r_squared
                    .is_none_or(|value| (value - reconstructed_r_squared).abs() > 1.0e-10)
            {
                return Err(
                    "PLS-POS outcome audit does not reproduce centered standardized-score R-squared"
                        .into(),
                );
            }
            for ((source_row, observed), fitted) in audit
                .source_row_indices
                .iter()
                .zip(&audit.observed_scores)
                .zip(&audit.fitted_scores)
            {
                audit_segment.push(segment.segment_id.clone());
                audit_outcome.push(audit.outcome_id.clone());
                audit_source_row.push(u32_checked(*source_row, "PLS-POS audit source row")?);
                audit_observed.push(*observed);
                audit_fitted.push(*fitted);
                audit_observed_mean.push(audit.observed_mean);
                audit_centered_sst.push(audit.centered_total_sum_of_squares);
            }
        }
    }
    let segment_schema = Arc::new(Schema::new(vec![
        Field::new("segment_id", DataType::Utf8, false),
        Field::new("observations", DataType::UInt64, false),
        Field::new("objective_contribution", DataType::Float64, false),
        Field::new("receipt_method_version", DataType::Utf8, false),
        Field::new("full_segment_pls_refit", DataType::Boolean, false),
        Field::new("measurement_scores_reestimated", DataType::Boolean, false),
        Field::new("score_orientation_reapplied", DataType::Boolean, false),
        Field::new("interaction_stage_one_refit", DataType::Boolean, false),
        Field::new(
            "interaction_operands_restandardized_within_destination",
            DataType::Boolean,
            false,
        ),
        Field::new(
            "interaction_products_rebuilt_within_destination",
            DataType::Boolean,
            false,
        ),
        Field::new("joint_structural_equations_refit", DataType::Boolean, false),
    ]));
    let segments = RecordBatch::try_new(
        segment_schema,
        vec![
            Arc::new(StringArray::from(segment_id)),
            Arc::new(UInt64Array::from(segment_observations)),
            Arc::new(Float64Array::from(segment_objective)),
            Arc::new(StringArray::from(receipt_method)),
            Arc::new(BooleanArray::from(full_segment_pls_refit)),
            Arc::new(BooleanArray::from(measurement_scores_reestimated)),
            Arc::new(BooleanArray::from(score_orientation_reapplied)),
            Arc::new(BooleanArray::from(interaction_stage_one_refit)),
            Arc::new(BooleanArray::from(interaction_operands_restandardized)),
            Arc::new(BooleanArray::from(interaction_products_rebuilt)),
            Arc::new(BooleanArray::from(joint_structural_equations_refit)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let r2_schema = Arc::new(Schema::new(vec![
        Field::new("segment_id", DataType::Utf8, false),
        Field::new("outcome_id", DataType::Utf8, false),
        Field::new("r_squared", DataType::Float64, false),
    ]));
    let r_squared = RecordBatch::try_new(
        r2_schema,
        vec![
            Arc::new(StringArray::from(r2_segment)),
            Arc::new(StringArray::from(r2_outcome)),
            Arc::new(Float64Array::from(r2_value)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let signature_schema = Arc::new(Schema::new(vec![
        Field::new("segment_id", DataType::Utf8, false),
        Field::new("parameter_index", DataType::UInt32, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let signatures = RecordBatch::try_new(
        signature_schema,
        vec![
            Arc::new(StringArray::from(signature_segment)),
            Arc::new(UInt32Array::from(signature_index)),
            Arc::new(Float64Array::from(signature_value)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let outcome_audit = if audit_segment.is_empty() {
        None
    } else {
        Some(
            RecordBatch::try_new(
                Arc::new(Schema::new(vec![
                    Field::new("segment_id", DataType::Utf8, false),
                    Field::new("outcome_id", DataType::Utf8, false),
                    Field::new("source_row_index", DataType::UInt32, false),
                    Field::new("observed_score", DataType::Float64, false),
                    Field::new("fitted_score", DataType::Float64, false),
                    Field::new("observed_mean", DataType::Float64, false),
                    Field::new("centered_total_sum_of_squares", DataType::Float64, false),
                ])),
                vec![
                    Arc::new(StringArray::from(audit_segment)),
                    Arc::new(StringArray::from(audit_outcome)),
                    Arc::new(UInt32Array::from(audit_source_row)),
                    Arc::new(Float64Array::from(audit_observed)),
                    Arc::new(Float64Array::from(audit_fitted)),
                    Arc::new(Float64Array::from(audit_observed_mean)),
                    Arc::new(Float64Array::from(audit_centered_sst)),
                ],
            )
            .map_err(|error| error.to_string())?,
        )
    };

    let mut start_index = Vec::new();
    let mut start_completed = Vec::new();
    let mut start_accepted_moves = Vec::new();
    let mut start_final_objective = Vec::new();
    let mut start_failure_reason = Vec::new();
    let mut trace_algorithm = Vec::new();
    let mut trace_k = Vec::new();
    let mut trace_start = Vec::new();
    let mut trace_iteration = Vec::new();
    let mut trace_objective = Vec::new();
    let mut trace_converged = Vec::new();
    let mut trace_failure_code = Vec::new();
    let mut failure_start = Vec::new();
    let mut failure_observation = Vec::new();
    let mut failure_source = Vec::new();
    let mut failure_destination = Vec::new();
    let mut failure_reason = Vec::new();
    for start in &value.starts {
        start_index.push(u32_checked(start.start_index, "PLS-POS start index")?);
        start_completed.push(start.completed);
        start_accepted_moves.push(u32_checked(start.accepted_moves, "PLS-POS accepted moves")?);
        start_final_objective.push(start.final_objective);
        start_failure_reason.push(start.failure_reason.clone());
        for (iteration, objective) in start.objective_history.iter().enumerate() {
            trace_algorithm.push("pls_pos_v2".to_owned());
            trace_k.push(k);
            trace_start.push(u32_checked(start.start_index, "PLS-POS trace start")?);
            trace_iteration.push(u32_checked(iteration, "PLS-POS trace iteration")?);
            trace_objective.push(*objective);
            trace_converged.push(start.completed);
            trace_failure_code.push(if start.completed {
                String::new()
            } else {
                start
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| "start_failed".into())
            });
        }
        for failed in &start.candidate_refit_failures {
            failure_start.push(u32_checked(start.start_index, "PLS-POS failure start")?);
            failure_observation.push(u32_checked(
                failed.observation,
                "PLS-POS failed observation",
            )?);
            failure_source.push(
                u8::try_from(failed.source_segment)
                    .map_err(|_| "PLS-POS failure source exceeds UInt8".to_owned())?,
            );
            failure_destination.push(
                u8::try_from(failed.destination_segment)
                    .map_err(|_| "PLS-POS failure destination exceeds UInt8".to_owned())?,
            );
            failure_reason.push(failed.reason.clone());
        }
    }
    let start_schema = Arc::new(Schema::new(vec![
        Field::new("start_index", DataType::UInt32, false),
        Field::new("completed", DataType::Boolean, false),
        Field::new("accepted_moves", DataType::UInt32, false),
        Field::new("final_objective", DataType::Float64, true),
        Field::new("failure_reason", DataType::Utf8, true),
    ]));
    let starts = RecordBatch::try_new(
        start_schema,
        vec![
            Arc::new(UInt32Array::from(start_index)),
            Arc::new(BooleanArray::from(start_completed)),
            Arc::new(UInt32Array::from(start_accepted_moves)),
            Arc::new(Float64Array::from(start_final_objective)),
            Arc::new(StringArray::from(start_failure_reason)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let reproducing_schema = Arc::new(Schema::new(vec![Field::new(
        "start_index",
        DataType::UInt32,
        false,
    )]));
    let reproducing = RecordBatch::try_new(
        reproducing_schema,
        vec![Arc::new(UInt32Array::from(
            value
                .reproducing_start_indices
                .iter()
                .map(|index| u32_checked(*index, "PLS-POS reproducing start"))
                .collect::<Result<Vec<_>, _>>()?,
        ))],
    )
    .map_err(|error| error.to_string())?;

    let completed = &value.multistart_evidence.completed_starts;
    let receipts = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("fit_statistic", DataType::Float64, false),
            Field::new("partition_sha256", DataType::Utf8, false),
            Field::new("numeric_signature_sha256", DataType::Utf8, false),
            Field::new("posterior_sha256", DataType::Utf8, true),
            Field::new("fit_statistic_sha256", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(
                completed
                    .iter()
                    .map(|start| u32_checked(start.start_index, "PLS-POS multistart receipt"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(Float64Array::from(
                completed
                    .iter()
                    .map(|start| start.final_objective)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.partition_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.parameter_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(vec![None::<String>; completed.len()])),
            Arc::new(StringArray::from(
                completed
                    .iter()
                    .map(|start| start.fit_statistic_sha256.clone())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut retained_assignment_start = Vec::new();
    let mut retained_assignment_row = Vec::new();
    let mut retained_assignment_class = Vec::new();
    let mut retained_signature_start = Vec::new();
    let mut retained_signature_class = Vec::new();
    let mut retained_signature_index = Vec::new();
    let mut retained_signature_value = Vec::new();
    for start in completed {
        let start_index = u32_checked(start.start_index, "PLS-POS retained start")?;
        for (row_index, segment) in start.canonical_assignments.iter().enumerate() {
            retained_assignment_start.push(start_index);
            retained_assignment_row.push(u32_checked(row_index, "PLS-POS retained assignment")?);
            retained_assignment_class.push(
                u8::try_from(*segment)
                    .map_err(|_| "PLS-POS retained segment exceeds UInt8".to_owned())?,
            );
        }
        for (segment_index, signature) in start.canonical_parameter_signatures.iter().enumerate() {
            for (parameter_index, observed) in signature.iter().enumerate() {
                retained_signature_start.push(start_index);
                retained_signature_class.push(
                    u8::try_from(segment_index)
                        .map_err(|_| "PLS-POS retained segment exceeds UInt8".to_owned())?,
                );
                retained_signature_index.push(u32_checked(
                    parameter_index,
                    "PLS-POS retained parameter index",
                )?);
                retained_signature_value.push(*observed);
            }
        }
    }
    let retained_assignments = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("row_index", DataType::UInt32, false),
            Field::new("class_id", DataType::UInt8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_assignment_start)),
            Arc::new(UInt32Array::from(retained_assignment_row)),
            Arc::new(UInt8Array::from(retained_assignment_class)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let retained_signatures = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("class_id", DataType::UInt8, false),
            Field::new("parameter_index", DataType::UInt32, false),
            Field::new("value", DataType::Float64, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_signature_start)),
            Arc::new(UInt8Array::from(retained_signature_class)),
            Arc::new(UInt32Array::from(retained_signature_index)),
            Arc::new(Float64Array::from(retained_signature_value)),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("memberships", membership),
        table_v1("segments", segments),
        table_v1("segment-r-squared", r_squared),
        table_v1("parameter-signatures", signatures),
        table_v1("start-diagnostics", starts),
        table_v1("reproducing-starts", reproducing),
        table_v1("multistart-receipts", receipts),
        table_v1("multistart-assignments", retained_assignments),
        table_v1("multistart-parameter-signatures", retained_signatures),
    ];
    if let Some(outcome_audit) = outcome_audit {
        tables.push(table_v1("outcome-fit-audit", outcome_audit));
    }
    if !trace_algorithm.is_empty() {
        tables.push(table_v1(
            "start-traces",
            multimod_start_trace_batch_v1(
                trace_algorithm,
                trace_k,
                trace_start,
                trace_iteration,
                trace_objective,
                trace_converged,
                trace_failure_code,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    if !failure_start.is_empty() {
        let failure_schema = Arc::new(Schema::new(vec![
            Field::new("start_index", DataType::UInt32, false),
            Field::new("observation", DataType::UInt32, false),
            Field::new("source_segment", DataType::UInt8, false),
            Field::new("destination_segment", DataType::UInt8, false),
            Field::new("reason", DataType::Utf8, false),
        ]));
        tables.push(table_v1(
            "candidate-refit-failures",
            RecordBatch::try_new(
                failure_schema,
                vec![
                    Arc::new(UInt32Array::from(failure_start)),
                    Arc::new(UInt32Array::from(failure_observation)),
                    Arc::new(UInt8Array::from(failure_source)),
                    Arc::new(UInt8Array::from(failure_destination)),
                    Arc::new(StringArray::from(failure_reason)),
                ],
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn encode_heterogeneity_pooled_baseline_v2(
    value: &PooledStructuralBaselineV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    if value.equations.is_empty() {
        return Err("heterogeneity pooled baseline has no structural equations".into());
    }
    let metadata = metadata_batch_v1(
        "heterogeneity_pooled_baseline",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_string("metric_source_sha256", value.metric_source_sha256.clone()),
            metadata_u64("observations", value.observations)?,
            metadata_u64("equation_count", value.equations.len())?,
        ],
    )?;
    let mut equation_id = Vec::new();
    let mut outcome_id = Vec::new();
    let mut parameter_id = Vec::new();
    let mut estimate = Vec::new();
    let mut residual_variance = Vec::new();
    let mut r_squared = Vec::new();
    for equation in &value.equations {
        if equation.coefficients.is_empty()
            || !equation.residual_variance.is_finite()
            || !equation.r_squared.is_finite()
        {
            return Err(format!(
                "pooled baseline equation {} has incomplete diagnostics",
                equation.equation_id
            ));
        }
        for coefficient in &equation.coefficients {
            if !coefficient.estimate.is_finite() {
                return Err(format!(
                    "pooled baseline coefficient {} is nonfinite",
                    coefficient.parameter_id
                ));
            }
            equation_id.push(equation.equation_id.clone());
            outcome_id.push(equation.outcome_id.clone());
            parameter_id.push(coefficient.parameter_id.clone());
            estimate.push(coefficient.estimate);
            residual_variance.push(equation.residual_variance);
            r_squared.push(equation.r_squared);
        }
    }
    let coefficients = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("equation_id", DataType::Utf8, false),
            Field::new("outcome_id", DataType::Utf8, false),
            Field::new("parameter_id", DataType::Utf8, false),
            Field::new("estimate", DataType::Float64, false),
            Field::new("residual_variance", DataType::Float64, false),
            Field::new("r_squared", DataType::Float64, false),
        ])),
        vec![
            Arc::new(StringArray::from(equation_id)),
            Arc::new(StringArray::from(outcome_id)),
            Arc::new(StringArray::from(parameter_id)),
            Arc::new(Float64Array::from(estimate)),
            Arc::new(Float64Array::from(residual_variance)),
            Arc::new(Float64Array::from(r_squared)),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("coefficients", coefficients),
    ])
}

fn encode_raw_heterogeneity_preparation_v1(
    value: &RawHeterogeneityPreparationReceiptV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    value.ensure_valid()?;
    let metadata = metadata_batch_v1(
        "heterogeneity_raw_preparation",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "general_sem_plan_sha256",
                value.general_sem_plan_sha256.clone(),
            ),
            metadata_string("pooled_metric_sha256", value.pooled_metric_sha256.clone()),
            metadata_string(
                "interaction_profile",
                stable_enum_id_v1(&value.fimix_input.interaction_profile)?,
            ),
            metadata_string(
                "pooled_metric_id",
                value.fimix_input.metric.metric_id.clone(),
            ),
            metadata_u64(
                "metric_observation_count",
                value.fimix_input.metric.observation_count,
            )?,
            metadata_bool(
                "scores_standardized_once_on_pooled_rows",
                value
                    .fimix_input
                    .metric
                    .scores_standardized_once_on_pooled_rows,
            ),
            metadata_bool(
                "products_standardized_once_on_pooled_rows",
                value
                    .fimix_input
                    .metric
                    .products_standardized_once_on_pooled_rows,
            ),
            metadata_u64("equation_count", value.fimix_input.equations.len())?,
            metadata_u64("analysis_rows", value.source_row_tokens.len())?,
            metadata_u64("omitted_source_rows", value.omitted_source_rows)?,
            metadata_bool("unique_analysis_positions", value.unique_analysis_positions),
        ],
    )?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("analysis_position", DataType::UInt32, false),
        Field::new("source_row_token", DataType::UInt64, false),
    ]));
    let row_map = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt32Array::from(
                (0..value.source_row_tokens.len())
                    .map(|row| u32_checked(row, "heterogeneity analysis position"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(UInt64Array::from(value.source_row_tokens.clone())),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut equation_id = Vec::new();
    let mut outcome_id = Vec::new();
    let mut row_index = Vec::new();
    let mut include_intercept = Vec::new();
    let mut predictor_id = Vec::new();
    let mut predictor_value = Vec::new();
    let mut outcome_value = Vec::new();
    for equation in &value.fimix_input.equations {
        if equation.design.len() != equation.outcome.len()
            || equation
                .design
                .iter()
                .any(|row| row.len() != equation.predictor_ids.len())
        {
            return Err("raw heterogeneity FIMIX input dimensions are inconsistent".into());
        }
        for (row, (design, outcome)) in equation.design.iter().zip(&equation.outcome).enumerate() {
            for (predictor, observed) in equation.predictor_ids.iter().zip(design) {
                equation_id.push(equation.equation_id.clone());
                outcome_id.push(equation.outcome_id.clone());
                row_index.push(u32_checked(row, "FIMIX input row")?);
                include_intercept.push(equation.include_intercept);
                predictor_id.push(predictor.clone());
                predictor_value.push(*observed);
                outcome_value.push(*outcome);
            }
        }
    }
    let fimix_input = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("equation_id", DataType::Utf8, false),
            Field::new("outcome_id", DataType::Utf8, false),
            Field::new("row_index", DataType::UInt32, false),
            Field::new("include_intercept", DataType::Boolean, false),
            Field::new("predictor_id", DataType::Utf8, false),
            Field::new("predictor_value", DataType::Float64, false),
            Field::new("outcome_value", DataType::Float64, false),
        ])),
        vec![
            Arc::new(StringArray::from(equation_id)),
            Arc::new(StringArray::from(outcome_id)),
            Arc::new(UInt32Array::from(row_index)),
            Arc::new(BooleanArray::from(include_intercept)),
            Arc::new(StringArray::from(predictor_id)),
            Arc::new(Float64Array::from(predictor_value)),
            Arc::new(Float64Array::from(outcome_value)),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("source-row-map", row_map),
        table_v1("fimix-input", fimix_input),
    ])
}

fn encode_heterogeneity_bootstrap_v1(
    value: &PreparedHeterogeneityBootstrapV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    value.ensure_valid()?;
    let metadata = metadata_batch_v1(
        "heterogeneity_bootstrap",
        vec![
            metadata_string(
                "target_payload_digest_method",
                "qpls.heterogeneity.target-payload-sha256-binary-f64-le.v2",
            ),
            metadata_string(
                "label_alignment_validation_method",
                "qpls.heterogeneity.exhaustive-overlap-alignment.v2",
            ),
            metadata_u64("replicate_entries", value.entries.len())?,
            metadata_u64("target_count", value.targets.len())?,
            metadata_bool(
                "complete_stage_one_and_segmentation_rerun",
                value.complete_stage_one_and_segmentation_rerun,
            ),
            metadata_bool(
                "pooled_common_metric_refit_repeated",
                value.pooled_common_metric_refit_repeated,
            ),
            metadata_bool(
                "exhaustive_label_alignment_applied",
                value.exhaustive_label_alignment_applied,
            ),
        ],
    )?;
    if value.entries.is_empty() {
        return Err("heterogeneity bootstrap evidence has no ledger entries".into());
    }
    let mut replicate_index = Vec::new();
    let mut seed = Vec::new();
    let mut status = Vec::new();
    let mut fit_statistic = Vec::new();
    let mut target_payload_sha256 = Vec::new();
    let mut failure_reason = Vec::new();
    let mut alignment_matched = Vec::new();
    let mut alignment_share = Vec::new();
    let mut alignment_ambiguous = Vec::new();
    let mut alignment_mutual_majority = Vec::new();
    let mut failure_by_replicate = BTreeMap::<usize, String>::new();
    let mut mapping_replicate = Vec::new();
    let mut mapping_candidate = Vec::new();
    let mut mapping_reference = Vec::new();
    let mut overlap_replicate = Vec::new();
    let mut overlap_reference = Vec::new();
    let mut overlap_candidate = Vec::new();
    let mut overlap_count = Vec::new();
    for entry in &value.entries {
        let status_id = stable_enum_id_v1(&entry.status)?;
        let usable = status_id == "usable";
        replicate_index.push(u32_checked(
            entry.replicate_index,
            "heterogeneity bootstrap replicate",
        )?);
        seed.push(entry.seed);
        status.push(status_id.clone());
        fit_statistic.push(entry.fit_statistic);
        target_payload_sha256.push(entry.target_payload_sha256.clone());
        failure_reason.push(entry.failure_reason.clone());
        alignment_matched.push(
            entry
                .label_alignment
                .as_ref()
                .map(|alignment| alignment.matched_observations as u64),
        );
        alignment_share.push(
            entry
                .label_alignment
                .as_ref()
                .map(|alignment| alignment.match_share),
        );
        alignment_ambiguous.push(
            entry
                .label_alignment
                .as_ref()
                .map(|alignment| alignment.ambiguous),
        );
        alignment_mutual_majority.push(
            entry
                .label_alignment
                .as_ref()
                .map(|alignment| alignment.mutual_majority),
        );
        failure_by_replicate.insert(
            entry.replicate_index,
            if usable { String::new() } else { status_id },
        );
        if let Some(alignment) = &entry.label_alignment {
            for (candidate, reference) in alignment.candidate_to_reference.iter().enumerate() {
                mapping_replicate.push(u32_checked(
                    entry.replicate_index,
                    "heterogeneity alignment replicate",
                )?);
                mapping_candidate.push(
                    u8::try_from(candidate)
                        .map_err(|_| "alignment candidate label exceeds UInt8".to_owned())?,
                );
                mapping_reference.push(
                    u8::try_from(*reference)
                        .map_err(|_| "alignment reference label exceeds UInt8".to_owned())?,
                );
            }
            for (reference, row) in alignment.overlap.iter().enumerate() {
                for (candidate, count) in row.iter().enumerate() {
                    overlap_replicate.push(u32_checked(
                        entry.replicate_index,
                        "heterogeneity overlap replicate",
                    )?);
                    overlap_reference.push(
                        u8::try_from(reference)
                            .map_err(|_| "overlap reference label exceeds UInt8".to_owned())?,
                    );
                    overlap_candidate.push(
                        u8::try_from(candidate)
                            .map_err(|_| "overlap candidate label exceeds UInt8".to_owned())?,
                    );
                    overlap_count.push(
                        u64::try_from(*count)
                            .map_err(|_| "overlap count exceeds UInt64".to_owned())?,
                    );
                }
            }
        }
    }
    let ledger_schema = Arc::new(Schema::new(vec![
        Field::new("replicate_index", DataType::UInt32, false),
        Field::new("seed", DataType::UInt64, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("fit_statistic", DataType::Float64, true),
        Field::new("target_payload_sha256", DataType::Utf8, true),
        Field::new("failure_reason", DataType::Utf8, true),
        Field::new("alignment_matched_observations", DataType::UInt64, true),
        Field::new("alignment_match_share", DataType::Float64, true),
        Field::new("alignment_ambiguous", DataType::Boolean, true),
        Field::new("alignment_mutual_majority", DataType::Boolean, true),
    ]));
    let ledger = RecordBatch::try_new(
        ledger_schema,
        vec![
            Arc::new(UInt32Array::from(replicate_index)),
            Arc::new(UInt64Array::from(seed)),
            Arc::new(StringArray::from(status)),
            Arc::new(Float64Array::from(fit_statistic)),
            Arc::new(StringArray::from(target_payload_sha256)),
            Arc::new(StringArray::from(failure_reason)),
            Arc::new(UInt64Array::from(alignment_matched)),
            Arc::new(Float64Array::from(alignment_share)),
            Arc::new(BooleanArray::from(alignment_ambiguous)),
            Arc::new(BooleanArray::from(alignment_mutual_majority)),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut target_replicate = Vec::new();
    let mut target_id = Vec::new();
    let mut target_value = Vec::new();
    let mut target_valid = Vec::new();
    let mut target_failure = Vec::new();
    for target in &value.targets {
        if target.estimates.len() != value.entries.len() {
            return Err(format!(
                "heterogeneity target {} does not share the complete replicate ledger",
                target.target_id
            ));
        }
        for (position, observed) in target.estimates.iter().enumerate() {
            let entry = &value.entries[position];
            let valid = observed.is_some();
            let ledger_failure = failure_by_replicate
                .get(&entry.replicate_index)
                .cloned()
                .unwrap_or_else(|| "missing_ledger_entry".into());
            if valid && !ledger_failure.is_empty() {
                return Err(format!(
                    "heterogeneity target {} is populated for a failed replicate",
                    target.target_id
                ));
            }
            target_replicate.push(u32_checked(
                entry.replicate_index,
                "heterogeneity target replicate",
            )?);
            target_id.push(target.target_id.clone());
            target_value.push(observed.unwrap_or_default());
            target_valid.push(valid);
            target_failure.push(if valid {
                String::new()
            } else if ledger_failure.is_empty() {
                "missing_target_estimate".into()
            } else {
                ledger_failure
            });
        }
    }
    let targets = multimod_target_ledger_batch_v1(
        target_replicate,
        target_id,
        target_value,
        target_valid,
        target_failure,
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("ledger", ledger),
        table_v1("target-vectors", targets),
    ];
    if !mapping_replicate.is_empty() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("replicate_index", DataType::UInt32, false),
            Field::new("candidate_label", DataType::UInt8, false),
            Field::new("reference_label", DataType::UInt8, false),
        ]));
        tables.push(table_v1(
            "label-mapping",
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(UInt32Array::from(mapping_replicate)),
                    Arc::new(UInt8Array::from(mapping_candidate)),
                    Arc::new(UInt8Array::from(mapping_reference)),
                ],
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    if !overlap_replicate.is_empty() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("replicate_index", DataType::UInt32, false),
            Field::new("reference_label", DataType::UInt8, false),
            Field::new("candidate_label", DataType::UInt8, false),
            Field::new("overlap_count", DataType::UInt64, false),
        ]));
        tables.push(table_v1(
            "label-overlap",
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(UInt32Array::from(overlap_replicate)),
                    Arc::new(UInt8Array::from(overlap_reference)),
                    Arc::new(UInt8Array::from(overlap_candidate)),
                    Arc::new(UInt64Array::from(overlap_count)),
                ],
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn prepared_ledger_failure_code_v1(status: &PreparedReplicateStatusV1) -> Result<String, String> {
    match status {
        PreparedReplicateStatusV1::Usable => Ok(String::new()),
        PreparedReplicateStatusV1::Failed {
            kind, stable_code, ..
        } => {
            if stable_code.trim().is_empty() {
                Ok(stable_enum_id_v1(kind)?)
            } else {
                Ok(stable_code.clone())
            }
        }
    }
}

fn prepared_resample_ledger_batch_v1(
    ledger: &PreparedSharedReplicateLedgerV1,
) -> Result<RecordBatch, String> {
    if ledger.entries.len() != ledger.requested as usize {
        return Err("prepared shared ledger does not contain every requested replicate".into());
    }
    let mut replicate_index = Vec::with_capacity(ledger.entries.len());
    let mut seed = Vec::with_capacity(ledger.entries.len());
    let mut usable = Vec::with_capacity(ledger.entries.len());
    let mut failure_code = Vec::with_capacity(ledger.entries.len());
    let mut shard_id = Vec::with_capacity(ledger.entries.len());
    for (position, entry) in ledger.entries.iter().enumerate() {
        if entry.replicate_index as usize != position {
            return Err("prepared shared ledger replicate identities are not contiguous".into());
        }
        replicate_index.push(entry.replicate_index);
        seed.push(entry.seed);
        usable.push(matches!(&entry.status, PreparedReplicateStatusV1::Usable));
        failure_code.push(prepared_ledger_failure_code_v1(&entry.status)?);
        shard_id.push(format!("{}:shared", ledger.domain));
    }
    multimod_resample_ledger_batch_v1(replicate_index, seed, usable, failure_code, shard_id)
        .map_err(|error| error.to_string())
}

fn prepared_target_tables_v1(
    ledger: &PreparedSharedReplicateLedgerV1,
    targets: &[PreparedTargetReplicatesV1],
    expected_jackknife_count: Option<usize>,
) -> Result<Vec<EvidenceTableV1>, String> {
    if targets.is_empty() {
        return Err("prepared inference contains no explicit target vector".into());
    }
    let mut replicate_index = Vec::new();
    let mut target_id = Vec::new();
    let mut value = Vec::new();
    let mut valid = Vec::new();
    let mut failure_code = Vec::new();
    let mut outer_replicate = Vec::new();
    let mut outer_target_id = Vec::new();
    let mut outer_value = Vec::new();
    let mut outer_valid = Vec::new();
    let mut outer_failure = Vec::new();
    let mut summary_target = Vec::new();
    let mut summary_observed_se = Vec::new();
    let mut summary_has_observed_se = Vec::new();
    let mut jackknife_replicate = Vec::new();
    let mut jackknife_target = Vec::new();
    let mut jackknife_value = Vec::new();
    let mut jackknife_valid = Vec::new();
    let mut jackknife_failure = Vec::new();
    let mut bca_target = Vec::new();
    let mut bca_count = Vec::new();
    let mut bca_mean = Vec::new();
    let mut bca_cube = Vec::new();
    let mut bca_square = Vec::new();
    let mut bca_acceleration = Vec::new();
    let mut bca_complete = Vec::new();
    for target in targets {
        if target.estimates.len() != ledger.entries.len() {
            return Err(format!(
                "target {} does not use the complete shared replicate ledger",
                target.target_id
            ));
        }
        if !target.outer_standard_errors.is_empty()
            && target.outer_standard_errors.len() != ledger.entries.len()
        {
            return Err(format!(
                "target {} has an incomplete studentized outer-SE vector",
                target.target_id
            ));
        }
        summary_target.push(target.target_id.clone());
        summary_observed_se.push(target.observed_standard_error.unwrap_or_default());
        summary_has_observed_se.push(target.observed_standard_error.is_some());
        for (position, (entry, observed)) in
            ledger.entries.iter().zip(&target.estimates).enumerate()
        {
            let ledger_failure = prepared_ledger_failure_code_v1(&entry.status)?;
            let is_valid = observed.is_some();
            if is_valid && !ledger_failure.is_empty() {
                return Err(format!(
                    "target {} is populated for failed replicate {position}",
                    target.target_id
                ));
            }
            replicate_index.push(entry.replicate_index);
            target_id.push(target.target_id.clone());
            value.push(observed.unwrap_or_default());
            valid.push(is_valid);
            failure_code.push(if is_valid {
                String::new()
            } else if ledger_failure.is_empty() {
                "missing_target_estimate".into()
            } else {
                ledger_failure.clone()
            });
            if let Some(outer) = target.outer_standard_errors.get(position) {
                let outer_is_valid = outer.is_some();
                if outer_is_valid && !ledger_failure.is_empty() {
                    return Err(format!(
                        "target {} has an outer SE for failed replicate {position}",
                        target.target_id
                    ));
                }
                outer_replicate.push(entry.replicate_index);
                outer_target_id.push(format!("{}:outer_standard_error", target.target_id));
                outer_value.push(outer.unwrap_or_default());
                outer_valid.push(outer_is_valid);
                outer_failure.push(if outer_is_valid {
                    String::new()
                } else if ledger_failure.is_empty() {
                    "missing_outer_standard_error".into()
                } else {
                    ledger_failure
                });
            }
        }
        if !target.delete_one_jackknife_estimates.is_empty() {
            if target.delete_one_jackknife_estimates.len() < 2 {
                return Err(format!(
                    "target {} has fewer than two delete-one estimates",
                    target.target_id
                ));
            }
            if expected_jackknife_count
                .is_some_and(|expected| target.delete_one_jackknife_estimates.len() != expected)
            {
                return Err(format!(
                    "target {} does not contain the complete delete-one jackknife",
                    target.target_id
                ));
            }
            for (index, observed) in target.delete_one_jackknife_estimates.iter().enumerate() {
                if !observed.is_finite() {
                    return Err(format!(
                        "target {} has a nonfinite delete-one estimate",
                        target.target_id
                    ));
                }
                jackknife_replicate.push(u32_checked(index, "jackknife row")?);
                jackknife_target.push(target.target_id.clone());
                jackknife_value.push(*observed);
                jackknife_valid.push(true);
                jackknife_failure.push(String::new());
            }
            let count = target.delete_one_jackknife_estimates.len();
            let mean = target.delete_one_jackknife_estimates.iter().sum::<f64>() / count as f64;
            let centered_square_sum = target
                .delete_one_jackknife_estimates
                .iter()
                .map(|observed| (mean - observed).powi(2))
                .sum::<f64>();
            let centered_cube_sum = target
                .delete_one_jackknife_estimates
                .iter()
                .map(|observed| (mean - observed).powi(3))
                .sum::<f64>();
            let denominator = 6.0 * centered_square_sum.powf(1.5);
            let acceleration = if denominator > f64::EPSILON {
                centered_cube_sum / denominator
            } else {
                0.0
            };
            bca_target.push(target.target_id.clone());
            bca_count.push(u32_checked(count, "BCa jackknife count")?);
            bca_mean.push(mean);
            bca_cube.push(centered_cube_sum);
            bca_square.push(centered_square_sum);
            bca_acceleration.push(acceleration);
            bca_complete.push(expected_jackknife_count.is_none_or(|expected| expected == count));
        }
    }
    let target_ledger =
        multimod_target_ledger_batch_v1(replicate_index, target_id, value, valid, failure_code)
            .map_err(|error| error.to_string())?;
    let summary_schema = Arc::new(Schema::new(vec![
        Field::new("target_id", DataType::Utf8, false),
        Field::new("observed_standard_error", DataType::Float64, true),
    ]));
    let summary = RecordBatch::try_new(
        summary_schema,
        vec![
            Arc::new(StringArray::from(summary_target)),
            Arc::new(Float64Array::from(
                summary_observed_se
                    .into_iter()
                    .zip(summary_has_observed_se)
                    .map(|(value, present)| present.then_some(value))
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("target-vectors", target_ledger),
        table_v1("target-summary", summary),
    ];
    if !outer_replicate.is_empty() {
        tables.push(table_v1(
            "studentized-outer-standard-errors",
            multimod_target_ledger_batch_v1(
                outer_replicate,
                outer_target_id,
                outer_value,
                outer_valid,
                outer_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    if !jackknife_replicate.is_empty() {
        tables.push(table_v1(
            "delete-one-target-vectors",
            multimod_target_ledger_batch_v1(
                jackknife_replicate,
                jackknife_target,
                jackknife_value,
                jackknife_valid,
                jackknife_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
        tables.push(table_v1(
            "bca-jackknife-summary",
            multimod_bca_jackknife_summary_batch_v1(
                bca_target,
                bca_count,
                bca_mean,
                bca_cube,
                bca_square,
                bca_acceleration,
                bca_complete,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn encode_conditional_inference_v1(
    value: &PreparedConditionalInferenceV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "conditional_inference",
        vec![
            metadata_u64("ledger.master_seed", value.ledger.master_seed)?,
            metadata_string("ledger.domain", value.ledger.domain.clone()),
            metadata_u64("ledger.requested", value.ledger.requested)?,
            metadata_u64("analysis_observations", value.analysis_observations)?,
            metadata_bool(
                "complete_model_refit_per_replicate",
                value.complete_model_refit_per_replicate,
            ),
            metadata_bool(
                "original_sample_probe_anchors_frozen",
                value.original_sample_probe_anchors_frozen,
            ),
            metadata_bool(
                "hoc_dependency_stages_repeated",
                value.hoc_dependency_stages_repeated,
            ),
            metadata_bool(
                "stratified_group_resampling",
                value.stratified_group_resampling,
            ),
            metadata_bool(
                "weights_travel_with_resampled_rows",
                value.weights_travel_with_resampled_rows,
            ),
            metadata_bool(
                "frequency_count_space_resampling",
                value.frequency_count_space_resampling,
            ),
            metadata_bool(
                "nested_inner_refits_complete",
                value.nested_inner_refits_complete,
            ),
        ],
    )?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1(
            "resample-ledger",
            prepared_resample_ledger_batch_v1(&value.ledger)?,
        ),
    ];
    tables.extend(prepared_target_tables_v1(
        &value.ledger,
        &value.targets,
        Some(value.analysis_observations),
    )?);
    Ok(tables)
}

fn prefix_tables_v1(prefix: &str, mut tables: Vec<EvidenceTableV1>) -> Vec<EvidenceTableV1> {
    for table in &mut tables {
        table.leaf_suffix = format!("{prefix}-{}", table.leaf_suffix);
    }
    tables
}

fn encode_conditional_raw_preparation_v2(
    dataset_id: &str,
    value: &ConditionalProcessAnalysisFrameV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    if value.strata.is_empty() || value.required_source_columns.is_empty() {
        return Err("conditional raw preparation has an empty frame authority".into());
    }
    let metadata = metadata_batch_v1(
        "conditional_raw_preparation",
        vec![
            metadata_string("method_version", value.method_version.clone()),
            metadata_string("dataset_fingerprint", value.dataset_fingerprint.clone()),
            metadata_string(
                "analysis_row_mask_sha256",
                value.analysis_row_mask_sha256.clone(),
            ),
            metadata_u64(
                "required_source_column_count",
                value.required_source_columns.len(),
            )?,
            metadata_u64("stratum_count", value.strata.len())?,
            metadata_u64("excluded_row_count", value.excluded_rows.len())?,
        ],
    )?;
    let columns = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("column_index", DataType::UInt32, false),
            Field::new("source_column", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(
                (0..value.required_source_columns.len())
                    .map(|index| u32_checked(index, "required source column index"))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(StringArray::from(value.required_source_columns.clone())),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut stratum_index = Vec::new();
    let mut group_id = Vec::<Option<String>>::new();
    let mut stratum_position = Vec::new();
    let mut source_row = Vec::new();
    let mut stable_row_token = Vec::new();
    let mut case_weight = Vec::new();
    let mut frequency = Vec::new();
    for (index, stratum) in value.strata.iter().enumerate() {
        if stratum.source_rows.is_empty()
            || stratum.case_weights.as_ref().is_some_and(|weights| {
                weights.len() != stratum.source_rows.len()
                    || weights
                        .iter()
                        .any(|weight| !weight.is_finite() || *weight <= 0.0)
            })
            || stratum
                .frequencies
                .as_ref()
                .is_some_and(|counts| counts.len() != stratum.source_rows.len())
        {
            return Err(format!(
                "conditional raw preparation stratum {index} has an invalid row/weight/count shape"
            ));
        }
        for (position, observed_row) in stratum.source_rows.iter().enumerate() {
            stratum_index.push(u32_checked(index, "conditional stratum index")?);
            group_id.push(stratum.group_id.clone());
            stratum_position.push(u32_checked(position, "conditional stratum position")?);
            source_row.push(*observed_row);
            stable_row_token.push(source_row_token_v1(dataset_id, u64::from(*observed_row)));
            case_weight.push(
                stratum
                    .case_weights
                    .as_ref()
                    .map(|weights| weights[position]),
            );
            frequency.push(stratum.frequencies.as_ref().map(|counts| counts[position]));
        }
    }
    let rows = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("stratum_index", DataType::UInt32, false),
            Field::new("group_id", DataType::Utf8, true),
            Field::new("stratum_position", DataType::UInt32, false),
            Field::new("source_row", DataType::UInt32, false),
            Field::new("stable_row_token", DataType::Utf8, false),
            Field::new("case_weight", DataType::Float64, true),
            Field::new("frequency", DataType::UInt64, true),
        ])),
        vec![
            Arc::new(UInt32Array::from(stratum_index)),
            Arc::new(StringArray::from(group_id)),
            Arc::new(UInt32Array::from(stratum_position)),
            Arc::new(UInt32Array::from(source_row)),
            Arc::new(StringArray::from(stable_row_token)),
            Arc::new(Float64Array::from(case_weight)),
            Arc::new(UInt64Array::from(frequency)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut excluded_source_row = Vec::new();
    let mut excluded_row_token = Vec::new();
    let mut exclusion_reason = Vec::new();
    for excluded in &value.excluded_rows {
        excluded_source_row.push(excluded.source_row);
        excluded_row_token.push(source_row_token_v1(
            dataset_id,
            u64::from(excluded.source_row),
        ));
        exclusion_reason.push(stable_enum_id_v1(&excluded.reason)?);
    }
    let exclusions = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("source_row", DataType::UInt32, false),
            Field::new("stable_row_token", DataType::Utf8, false),
            Field::new("reason", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(excluded_source_row)),
            Arc::new(StringArray::from(excluded_row_token)),
            Arc::new(StringArray::from(exclusion_reason)),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(vec![
        table_v1("metadata", metadata),
        table_v1("required-source-columns", columns),
        table_v1("analysis-rows", rows),
        table_v1("excluded-rows", exclusions),
    ])
}

fn encode_conditional_raw_full_refit_v2(
    value: &RawConditionalProcessEvidenceV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    match value {
        RawConditionalProcessEvidenceV2::PercentileCase { bootstrap } => Ok(prefix_tables_v1(
            "case",
            encode_case_full_refit_ledger_v1(bootstrap, "conditional_percentile_case")?,
        )),
        RawConditionalProcessEvidenceV2::BcaCase {
            bootstrap,
            delete_one,
        } => {
            let mut tables = prefix_tables_v1(
                "bootstrap",
                encode_case_full_refit_ledger_v1(bootstrap, "conditional_bca_bootstrap")?,
            );
            tables.extend(prefix_tables_v1(
                "delete-one",
                encode_delete_one_full_refit_ledger_v1(delete_one)?,
            ));
            Ok(tables)
        }
        RawConditionalProcessEvidenceV2::StudentizedCase {
            nested,
            observed_inner,
        } => {
            let mut tables =
                prefix_tables_v1("nested", encode_studentized_full_refit_ledger_v1(nested)?);
            tables.extend(prefix_tables_v1(
                "observed-inner",
                encode_case_full_refit_ledger_v1(
                    observed_inner,
                    "conditional_studentized_observed_inner",
                )?,
            ));
            Ok(tables)
        }
        RawConditionalProcessEvidenceV2::GroupedStratified { groups } => {
            if groups.is_empty() {
                return Err("grouped conditional raw evidence has no group ledgers".into());
            }
            let mut group_ids = BTreeSet::new();
            let mut tables = Vec::new();
            for (index, group) in groups.iter().enumerate() {
                if group.group_id.trim().is_empty() || !group_ids.insert(group.group_id.as_str()) {
                    return Err(
                        "grouped conditional raw evidence has blank or duplicate group ids".into(),
                    );
                }
                let prefix = format!("group-{index:02}");
                tables.push(table_v1(
                    format!("{prefix}-identity"),
                    metadata_batch_v1(
                        "conditional_grouped_stratified_identity",
                        vec![
                            metadata_u64("group_index", index)?,
                            metadata_string("group_id", group.group_id.clone()),
                        ],
                    )?,
                ));
                tables.extend(prefix_tables_v1(
                    &prefix,
                    encode_case_full_refit_ledger_v1(
                        &group.ledger,
                        "conditional_grouped_stratified_ledger",
                    )?,
                ));
            }
            Ok(tables)
        }
        RawConditionalProcessEvidenceV2::FrequencyCountSpace { bootstrap } => Ok(prefix_tables_v1(
            "frequency",
            encode_frequency_full_refit_ledger_v1(bootstrap)?,
        )),
    }
}

fn full_refit_target_width_v1<Draw>(
    value: &MultiModFinalLedgerV1<Draw, Vec<f64>>,
    evidence_kind: &str,
) -> Result<usize, String> {
    let target_width = value
        .records
        .iter()
        .find_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => Some(value.len()),
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .unwrap_or_default();
    if value.records.iter().any(|record| {
        matches!(
            &record.outcome,
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() != target_width || value.iter().any(|item| !item.is_finite())
        )
    }) {
        return Err(format!(
            "{evidence_kind} target width changed or contains nonfinite values"
        ));
    }
    Ok(target_width)
}

fn final_ledger_metadata_v1<Draw>(
    evidence_kind: &str,
    value: &MultiModFinalLedgerV1<Draw, Vec<f64>>,
) -> Result<RecordBatch, String> {
    metadata_batch_v1(
        evidence_kind,
        vec![
            metadata_u64("schema_version", value.schema_version)?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "execution_identity_sha256",
                value.execution_identity_sha256.clone(),
            ),
            metadata_u64("requested", value.requested)?,
            metadata_u64("usable", value.usable)?,
            metadata_u64("minimum_required", value.minimum_required)?,
            metadata_number("usable_fraction", value.usable_fraction)?,
            metadata_bool("complete", value.complete),
            metadata_string("ledger_sha256", value.ledger_sha256.clone()),
        ],
    )
}

fn usable_indices_batch_v1(indices: &[u32]) -> Result<RecordBatch, String> {
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![Field::new(
            "record_index",
            DataType::UInt32,
            false,
        )])),
        vec![Arc::new(UInt32Array::from(indices.to_vec()))],
    )
    .map_err(|error| error.to_string())
}

fn encode_delete_one_full_refit_ledger_v1(
    value: &ConditionalDeleteOneLedgerV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    let evidence_kind = "conditional_bca_delete_one";
    let metadata = final_ledger_metadata_v1(evidence_kind, value)?;
    let target_width = full_refit_target_width_v1(value, evidence_kind)?;
    let mut record_index = Vec::new();
    let mut attempt_count = Vec::new();
    let mut omitted_row = Vec::new();
    let mut retained_rows_sha256 = Vec::new();
    let mut case_weights_sha256 = Vec::new();
    let mut draw_identity_sha256 = Vec::new();
    let mut outcome_usable = Vec::new();
    let mut outcome_identity_sha256 = Vec::new();
    let mut failure_code = Vec::new();
    let mut failure_message = Vec::new();
    let mut record_identity_sha256 = Vec::new();
    let mut retained_record_index = Vec::new();
    let mut retained_position = Vec::new();
    let mut retained_source_row = Vec::new();
    let mut retained_case_weight = Vec::new();
    let mut target_replicate = Vec::new();
    let mut target_id = Vec::new();
    let mut target_value = Vec::new();
    let mut target_valid = Vec::new();
    let mut target_failure = Vec::new();
    for record in &value.records {
        if record.draw.case_weights.as_ref().is_some_and(|weights| {
            weights.len() != record.draw.retained_source_rows.len()
                || weights.iter().any(|weight| !weight.is_finite())
        }) {
            return Err("delete-one case weights do not match retained source rows".into());
        }
        record_index.push(record.index);
        attempt_count.push(record.attempt_count);
        omitted_row.push(record.draw.omitted_row);
        retained_rows_sha256.push(record.draw.retained_rows_sha256.clone());
        case_weights_sha256.push(record.draw.case_weights_sha256.clone());
        draw_identity_sha256.push(record.draw.draw_identity_sha256.clone());
        record_identity_sha256.push(record.record_identity_sha256.clone());
        let (estimate, outcome_hash, code, message) = match &record.outcome {
            MultiModRefitOutcomeV1::Success {
                value,
                value_sha256,
            } => (
                Some(value.as_slice()),
                value_sha256.clone(),
                String::new(),
                String::new(),
            ),
            MultiModRefitOutcomeV1::Failed {
                failure,
                failure_sha256,
            } => (
                None,
                failure_sha256.clone(),
                failure.code.clone(),
                failure.message.clone(),
            ),
        };
        outcome_usable.push(estimate.is_some());
        outcome_identity_sha256.push(outcome_hash);
        failure_code.push(code.clone());
        failure_message.push(message);
        for (position, source_row) in record.draw.retained_source_rows.iter().enumerate() {
            retained_record_index.push(record.index);
            retained_position.push(u32_checked(position, "delete-one retained position")?);
            retained_source_row.push(*source_row);
            retained_case_weight.push(
                record
                    .draw
                    .case_weights
                    .as_ref()
                    .map(|weights| weights[position]),
            );
        }
        for target_index in 0..target_width {
            let observed = estimate.map(|items| items[target_index]);
            target_replicate.push(record.index);
            target_id.push(format!("target_index:{target_index}"));
            target_value.push(observed.unwrap_or_default());
            target_valid.push(observed.is_some());
            target_failure.push(if observed.is_some() {
                String::new()
            } else if code.is_empty() {
                "missing_target_estimate".into()
            } else {
                code.clone()
            });
        }
    }
    let records = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("record_index", DataType::UInt32, false),
            Field::new("attempt_count", DataType::UInt8, false),
            Field::new("omitted_row", DataType::UInt32, false),
            Field::new("retained_rows_sha256", DataType::Utf8, false),
            Field::new("case_weights_sha256", DataType::Utf8, true),
            Field::new("draw_identity_sha256", DataType::Utf8, false),
            Field::new("outcome_usable", DataType::Boolean, false),
            Field::new("outcome_identity_sha256", DataType::Utf8, false),
            Field::new("failure_code", DataType::Utf8, false),
            Field::new("failure_message", DataType::Utf8, false),
            Field::new("record_identity_sha256", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(record_index)),
            Arc::new(UInt8Array::from(attempt_count)),
            Arc::new(UInt32Array::from(omitted_row)),
            Arc::new(StringArray::from(retained_rows_sha256)),
            Arc::new(StringArray::from(case_weights_sha256)),
            Arc::new(StringArray::from(draw_identity_sha256)),
            Arc::new(BooleanArray::from(outcome_usable)),
            Arc::new(StringArray::from(outcome_identity_sha256)),
            Arc::new(StringArray::from(failure_code)),
            Arc::new(StringArray::from(failure_message)),
            Arc::new(StringArray::from(record_identity_sha256)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let retained_rows = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("record_index", DataType::UInt32, false),
            Field::new("retained_position", DataType::UInt32, false),
            Field::new("source_row", DataType::UInt32, false),
            Field::new("case_weight", DataType::Float64, true),
        ])),
        vec![
            Arc::new(UInt32Array::from(retained_record_index)),
            Arc::new(UInt32Array::from(retained_position)),
            Arc::new(UInt32Array::from(retained_source_row)),
            Arc::new(Float64Array::from(retained_case_weight)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("records", records),
        table_v1("retained-rows", retained_rows),
        table_v1(
            "usable-indices",
            usable_indices_batch_v1(&value.usable_indices)?,
        ),
    ];
    if target_width > 0 {
        tables.push(table_v1(
            "target-vectors",
            multimod_target_ledger_batch_v1(
                target_replicate,
                target_id,
                target_value,
                target_valid,
                target_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn encode_frequency_full_refit_ledger_v1(
    value: &ConditionalFrequencyBootstrapLedgerV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    let evidence_kind = "conditional_frequency_count_space";
    let metadata = final_ledger_metadata_v1(evidence_kind, value)?;
    let target_width = full_refit_target_width_v1(value, evidence_kind)?;
    let mut record_index = Vec::new();
    let mut attempt_count = Vec::new();
    let mut draw_replicate_index = Vec::new();
    let mut total_count = Vec::new();
    let mut counts_sha256 = Vec::new();
    let mut draw_identity_sha256 = Vec::new();
    let mut outcome_usable = Vec::new();
    let mut outcome_identity_sha256 = Vec::new();
    let mut failure_code = Vec::new();
    let mut failure_message = Vec::new();
    let mut record_identity_sha256 = Vec::new();
    let mut count_record_index = Vec::new();
    let mut compact_position = Vec::new();
    let mut count = Vec::new();
    let mut target_replicate = Vec::new();
    let mut target_id = Vec::new();
    let mut target_value = Vec::new();
    let mut target_valid = Vec::new();
    let mut target_failure = Vec::new();
    for record in &value.records {
        record_index.push(record.index);
        attempt_count.push(record.attempt_count);
        draw_replicate_index.push(record.draw.replicate_index);
        total_count.push(record.draw.total_count);
        counts_sha256.push(record.draw.counts_sha256.clone());
        draw_identity_sha256.push(record.draw.draw_identity_sha256.clone());
        record_identity_sha256.push(record.record_identity_sha256.clone());
        let (estimate, outcome_hash, code, message) = match &record.outcome {
            MultiModRefitOutcomeV1::Success {
                value,
                value_sha256,
            } => (
                Some(value.as_slice()),
                value_sha256.clone(),
                String::new(),
                String::new(),
            ),
            MultiModRefitOutcomeV1::Failed {
                failure,
                failure_sha256,
            } => (
                None,
                failure_sha256.clone(),
                failure.code.clone(),
                failure.message.clone(),
            ),
        };
        outcome_usable.push(estimate.is_some());
        outcome_identity_sha256.push(outcome_hash);
        failure_code.push(code.clone());
        failure_message.push(message);
        for (position, observed_count) in record.draw.counts.iter().enumerate() {
            count_record_index.push(record.index);
            compact_position.push(u32_checked(position, "frequency compact position")?);
            count.push(*observed_count);
        }
        for target_index in 0..target_width {
            let observed = estimate.map(|items| items[target_index]);
            target_replicate.push(record.index);
            target_id.push(format!("target_index:{target_index}"));
            target_value.push(observed.unwrap_or_default());
            target_valid.push(observed.is_some());
            target_failure.push(if observed.is_some() {
                String::new()
            } else if code.is_empty() {
                "missing_target_estimate".into()
            } else {
                code.clone()
            });
        }
    }
    let records = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("record_index", DataType::UInt32, false),
            Field::new("attempt_count", DataType::UInt8, false),
            Field::new("draw_replicate_index", DataType::UInt32, false),
            Field::new("total_count", DataType::UInt64, false),
            Field::new("counts_sha256", DataType::Utf8, false),
            Field::new("draw_identity_sha256", DataType::Utf8, false),
            Field::new("outcome_usable", DataType::Boolean, false),
            Field::new("outcome_identity_sha256", DataType::Utf8, false),
            Field::new("failure_code", DataType::Utf8, false),
            Field::new("failure_message", DataType::Utf8, false),
            Field::new("record_identity_sha256", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(record_index)),
            Arc::new(UInt8Array::from(attempt_count)),
            Arc::new(UInt32Array::from(draw_replicate_index)),
            Arc::new(UInt64Array::from(total_count)),
            Arc::new(StringArray::from(counts_sha256)),
            Arc::new(StringArray::from(draw_identity_sha256)),
            Arc::new(BooleanArray::from(outcome_usable)),
            Arc::new(StringArray::from(outcome_identity_sha256)),
            Arc::new(StringArray::from(failure_code)),
            Arc::new(StringArray::from(failure_message)),
            Arc::new(StringArray::from(record_identity_sha256)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let counts = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("record_index", DataType::UInt32, false),
            Field::new("compact_position", DataType::UInt32, false),
            Field::new("count", DataType::UInt64, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(count_record_index)),
            Arc::new(UInt32Array::from(compact_position)),
            Arc::new(UInt64Array::from(count)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("records", records),
        table_v1("counts", counts),
        table_v1(
            "usable-indices",
            usable_indices_batch_v1(&value.usable_indices)?,
        ),
    ];
    if target_width > 0 {
        tables.push(table_v1(
            "target-vectors",
            multimod_target_ledger_batch_v1(
                target_replicate,
                target_id,
                target_value,
                target_valid,
                target_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn encode_studentized_full_refit_ledger_v1(
    value: &ConditionalStudentizedLedgerV2,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "conditional_studentized_nested",
        vec![
            metadata_u64("schema_version", value.schema_version)?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "execution_identity_sha256",
                value.execution_identity_sha256.clone(),
            ),
            metadata_u64("requested_outer", value.requested_outer)?,
            metadata_u64("usable_outer", value.usable_outer)?,
            metadata_u64("minimum_outer_required", value.minimum_outer_required)?,
            metadata_u64("requested_inner_per_outer", value.requested_inner_per_outer)?,
            metadata_u64("minimum_inner_required", value.minimum_inner_required)?,
            metadata_bool("complete", value.complete),
            metadata_string("ledger_sha256", value.ledger_sha256.clone()),
        ],
    )?;
    let outer_width = value
        .records
        .iter()
        .find_map(|record| match &record.outer.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => Some(value.len()),
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .unwrap_or_default();
    let inner_width = value
        .records
        .iter()
        .flat_map(|record| &record.inner_records)
        .find_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => Some(value.len()),
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .unwrap_or_default();
    if value.records.iter().any(|record| {
        matches!(
            &record.outer.outcome,
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() != outer_width || value.iter().any(|item| !item.is_finite())
        ) || record.inner_records.iter().any(|inner| {
            matches!(
                &inner.outcome,
                MultiModRefitOutcomeV1::Success { value, .. }
                    if value.len() != inner_width || value.iter().any(|item| !item.is_finite())
            )
        })
    }) {
        return Err("studentized target widths changed or contain nonfinite values".into());
    }

    let mut outer_record_index = Vec::new();
    let mut outer_attempt_count = Vec::new();
    let mut outer_draw_replicate = Vec::new();
    let mut outer_source_rows_sha256 = Vec::new();
    let mut outer_case_weights_sha256 = Vec::new();
    let mut outer_draw_identity_sha256 = Vec::new();
    let mut outer_outcome_usable = Vec::new();
    let mut outer_outcome_identity_sha256 = Vec::new();
    let mut outer_failure_code = Vec::new();
    let mut outer_failure_message = Vec::new();
    let mut outer_record_identity_sha256 = Vec::new();
    let mut inner_ledger_identity_sha256 = Vec::new();
    let mut inner_record_count = Vec::new();
    let mut inner_usable_count = Vec::new();
    let mut outer_draw_record = Vec::new();
    let mut outer_draw_position = Vec::new();
    let mut outer_draw_source_row = Vec::new();
    let mut outer_draw_case_weight = Vec::new();
    let mut outer_target_replicate = Vec::new();
    let mut outer_target_id = Vec::new();
    let mut outer_target_value = Vec::new();
    let mut outer_target_valid = Vec::new();
    let mut outer_target_failure = Vec::new();

    let mut inner_outer_record = Vec::new();
    let mut inner_record_index = Vec::new();
    let mut inner_attempt_count = Vec::new();
    let mut inner_draw_outer_replicate = Vec::new();
    let mut inner_draw_inner_replicate = Vec::new();
    let mut inner_outer_draw_identity_sha256 = Vec::new();
    let mut inner_source_rows_sha256 = Vec::new();
    let mut inner_case_weights_sha256 = Vec::new();
    let mut inner_draw_identity_sha256 = Vec::new();
    let mut inner_outcome_usable = Vec::new();
    let mut inner_outcome_identity_sha256 = Vec::new();
    let mut inner_failure_code = Vec::new();
    let mut inner_failure_message = Vec::new();
    let mut inner_record_identity_sha256 = Vec::new();
    let mut inner_draw_outer_record = Vec::new();
    let mut inner_draw_record = Vec::new();
    let mut inner_draw_position = Vec::new();
    let mut inner_draw_source_row = Vec::new();
    let mut inner_draw_case_weight = Vec::new();
    let mut inner_target_outer_record = Vec::new();
    let mut inner_target_record = Vec::new();
    let mut inner_target_index = Vec::new();
    let mut inner_target_value = Vec::<Option<f64>>::new();
    let mut inner_target_valid = Vec::new();
    let mut inner_target_failure = Vec::new();

    for outer in &value.records {
        if outer
            .outer
            .draw
            .case_weights
            .as_ref()
            .is_some_and(|weights| {
                weights.len() != outer.outer.draw.source_rows.len()
                    || weights.iter().any(|weight| !weight.is_finite())
            })
        {
            return Err("studentized outer case weights do not match source rows".into());
        }
        outer_record_index.push(outer.outer.index);
        outer_attempt_count.push(outer.outer.attempt_count);
        outer_draw_replicate.push(outer.outer.draw.replicate_index);
        outer_source_rows_sha256.push(outer.outer.draw.source_rows_sha256.clone());
        outer_case_weights_sha256.push(outer.outer.draw.case_weights_sha256.clone());
        outer_draw_identity_sha256.push(outer.outer.draw.draw_identity_sha256.clone());
        outer_record_identity_sha256.push(outer.outer.record_identity_sha256.clone());
        inner_ledger_identity_sha256.push(outer.inner_ledger_identity_sha256.clone());
        inner_record_count.push(u32_checked(
            outer.inner_records.len(),
            "studentized inner record count",
        )?);
        inner_usable_count.push(u32_checked(
            outer
                .inner_records
                .iter()
                .filter(|record| matches!(&record.outcome, MultiModRefitOutcomeV1::Success { .. }))
                .count(),
            "studentized inner usable count",
        )?);
        let (outer_estimate, outcome_hash, code, message) = match &outer.outer.outcome {
            MultiModRefitOutcomeV1::Success {
                value,
                value_sha256,
            } => (
                Some(value.as_slice()),
                value_sha256.clone(),
                String::new(),
                String::new(),
            ),
            MultiModRefitOutcomeV1::Failed {
                failure,
                failure_sha256,
            } => (
                None,
                failure_sha256.clone(),
                failure.code.clone(),
                failure.message.clone(),
            ),
        };
        outer_outcome_usable.push(outer_estimate.is_some());
        outer_outcome_identity_sha256.push(outcome_hash);
        outer_failure_code.push(code.clone());
        outer_failure_message.push(message);
        for (position, source_row) in outer.outer.draw.source_rows.iter().enumerate() {
            outer_draw_record.push(outer.outer.index);
            outer_draw_position.push(u32_checked(position, "studentized outer draw position")?);
            outer_draw_source_row.push(*source_row);
            outer_draw_case_weight.push(
                outer
                    .outer
                    .draw
                    .case_weights
                    .as_ref()
                    .map(|weights| weights[position]),
            );
        }
        for target_index in 0..outer_width {
            let observed = outer_estimate.map(|items| items[target_index]);
            outer_target_replicate.push(outer.outer.index);
            outer_target_id.push(format!("target_index:{target_index}"));
            outer_target_value.push(observed.unwrap_or_default());
            outer_target_valid.push(observed.is_some());
            outer_target_failure.push(if observed.is_some() {
                String::new()
            } else if code.is_empty() {
                "missing_target_estimate".into()
            } else {
                code.clone()
            });
        }

        for inner in &outer.inner_records {
            if inner.draw.case_weights.as_ref().is_some_and(|weights| {
                weights.len() != inner.draw.source_rows.len()
                    || weights.iter().any(|weight| !weight.is_finite())
            }) {
                return Err("studentized inner case weights do not match source rows".into());
            }
            inner_outer_record.push(outer.outer.index);
            inner_record_index.push(inner.index);
            inner_attempt_count.push(inner.attempt_count);
            inner_draw_outer_replicate.push(inner.draw.outer_replicate_index);
            inner_draw_inner_replicate.push(inner.draw.inner_replicate_index);
            inner_outer_draw_identity_sha256.push(inner.draw.outer_draw_identity_sha256.clone());
            inner_source_rows_sha256.push(inner.draw.source_rows_sha256.clone());
            inner_case_weights_sha256.push(inner.draw.case_weights_sha256.clone());
            inner_draw_identity_sha256.push(inner.draw.draw_identity_sha256.clone());
            inner_record_identity_sha256.push(inner.record_identity_sha256.clone());
            let (estimate, outcome_hash, inner_code, inner_message) = match &inner.outcome {
                MultiModRefitOutcomeV1::Success {
                    value,
                    value_sha256,
                } => (
                    Some(value.as_slice()),
                    value_sha256.clone(),
                    String::new(),
                    String::new(),
                ),
                MultiModRefitOutcomeV1::Failed {
                    failure,
                    failure_sha256,
                } => (
                    None,
                    failure_sha256.clone(),
                    failure.code.clone(),
                    failure.message.clone(),
                ),
            };
            inner_outcome_usable.push(estimate.is_some());
            inner_outcome_identity_sha256.push(outcome_hash);
            inner_failure_code.push(inner_code.clone());
            inner_failure_message.push(inner_message);
            for (position, source_row) in inner.draw.source_rows.iter().enumerate() {
                inner_draw_outer_record.push(outer.outer.index);
                inner_draw_record.push(inner.index);
                inner_draw_position.push(u32_checked(position, "studentized inner draw position")?);
                inner_draw_source_row.push(*source_row);
                inner_draw_case_weight.push(
                    inner
                        .draw
                        .case_weights
                        .as_ref()
                        .map(|weights| weights[position]),
                );
            }
            for target_index in 0..inner_width {
                let observed = estimate.map(|items| items[target_index]);
                inner_target_outer_record.push(outer.outer.index);
                inner_target_record.push(inner.index);
                inner_target_index
                    .push(u32_checked(target_index, "studentized inner target index")?);
                inner_target_value.push(observed);
                inner_target_valid.push(observed.is_some());
                inner_target_failure.push(if observed.is_some() {
                    String::new()
                } else if inner_code.is_empty() {
                    "missing_target_estimate".into()
                } else {
                    inner_code.clone()
                });
            }
        }
    }

    let outer_records = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("outer_record_index", DataType::UInt32, false),
            Field::new("attempt_count", DataType::UInt8, false),
            Field::new("draw_replicate_index", DataType::UInt32, false),
            Field::new("source_rows_sha256", DataType::Utf8, false),
            Field::new("case_weights_sha256", DataType::Utf8, true),
            Field::new("draw_identity_sha256", DataType::Utf8, false),
            Field::new("outcome_usable", DataType::Boolean, false),
            Field::new("outcome_identity_sha256", DataType::Utf8, false),
            Field::new("failure_code", DataType::Utf8, false),
            Field::new("failure_message", DataType::Utf8, false),
            Field::new("record_identity_sha256", DataType::Utf8, false),
            Field::new("inner_ledger_identity_sha256", DataType::Utf8, false),
            Field::new("inner_record_count", DataType::UInt32, false),
            Field::new("inner_usable_count", DataType::UInt32, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(outer_record_index)),
            Arc::new(UInt8Array::from(outer_attempt_count)),
            Arc::new(UInt32Array::from(outer_draw_replicate)),
            Arc::new(StringArray::from(outer_source_rows_sha256)),
            Arc::new(StringArray::from(outer_case_weights_sha256)),
            Arc::new(StringArray::from(outer_draw_identity_sha256)),
            Arc::new(BooleanArray::from(outer_outcome_usable)),
            Arc::new(StringArray::from(outer_outcome_identity_sha256)),
            Arc::new(StringArray::from(outer_failure_code)),
            Arc::new(StringArray::from(outer_failure_message)),
            Arc::new(StringArray::from(outer_record_identity_sha256)),
            Arc::new(StringArray::from(inner_ledger_identity_sha256)),
            Arc::new(UInt32Array::from(inner_record_count)),
            Arc::new(UInt32Array::from(inner_usable_count)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let outer_rows = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("outer_record_index", DataType::UInt32, false),
            Field::new("draw_position", DataType::UInt32, false),
            Field::new("source_row", DataType::UInt32, false),
            Field::new("case_weight", DataType::Float64, true),
        ])),
        vec![
            Arc::new(UInt32Array::from(outer_draw_record)),
            Arc::new(UInt32Array::from(outer_draw_position)),
            Arc::new(UInt32Array::from(outer_draw_source_row)),
            Arc::new(Float64Array::from(outer_draw_case_weight)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let inner_records = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("outer_record_index", DataType::UInt32, false),
            Field::new("inner_record_index", DataType::UInt32, false),
            Field::new("attempt_count", DataType::UInt8, false),
            Field::new("draw_outer_replicate_index", DataType::UInt32, false),
            Field::new("draw_inner_replicate_index", DataType::UInt32, false),
            Field::new("outer_draw_identity_sha256", DataType::Utf8, false),
            Field::new("source_rows_sha256", DataType::Utf8, false),
            Field::new("case_weights_sha256", DataType::Utf8, true),
            Field::new("draw_identity_sha256", DataType::Utf8, false),
            Field::new("outcome_usable", DataType::Boolean, false),
            Field::new("outcome_identity_sha256", DataType::Utf8, false),
            Field::new("failure_code", DataType::Utf8, false),
            Field::new("failure_message", DataType::Utf8, false),
            Field::new("record_identity_sha256", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(inner_outer_record)),
            Arc::new(UInt32Array::from(inner_record_index)),
            Arc::new(UInt8Array::from(inner_attempt_count)),
            Arc::new(UInt32Array::from(inner_draw_outer_replicate)),
            Arc::new(UInt32Array::from(inner_draw_inner_replicate)),
            Arc::new(StringArray::from(inner_outer_draw_identity_sha256)),
            Arc::new(StringArray::from(inner_source_rows_sha256)),
            Arc::new(StringArray::from(inner_case_weights_sha256)),
            Arc::new(StringArray::from(inner_draw_identity_sha256)),
            Arc::new(BooleanArray::from(inner_outcome_usable)),
            Arc::new(StringArray::from(inner_outcome_identity_sha256)),
            Arc::new(StringArray::from(inner_failure_code)),
            Arc::new(StringArray::from(inner_failure_message)),
            Arc::new(StringArray::from(inner_record_identity_sha256)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let inner_rows = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("outer_record_index", DataType::UInt32, false),
            Field::new("inner_record_index", DataType::UInt32, false),
            Field::new("draw_position", DataType::UInt32, false),
            Field::new("source_row", DataType::UInt32, false),
            Field::new("case_weight", DataType::Float64, true),
        ])),
        vec![
            Arc::new(UInt32Array::from(inner_draw_outer_record)),
            Arc::new(UInt32Array::from(inner_draw_record)),
            Arc::new(UInt32Array::from(inner_draw_position)),
            Arc::new(UInt32Array::from(inner_draw_source_row)),
            Arc::new(Float64Array::from(inner_draw_case_weight)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let inner_targets = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new("outer_record_index", DataType::UInt32, false),
            Field::new("inner_record_index", DataType::UInt32, false),
            Field::new("target_index", DataType::UInt32, false),
            Field::new("value", DataType::Float64, true),
            Field::new("valid", DataType::Boolean, false),
            Field::new("failure_code", DataType::Utf8, false),
        ])),
        vec![
            Arc::new(UInt32Array::from(inner_target_outer_record)),
            Arc::new(UInt32Array::from(inner_target_record)),
            Arc::new(UInt32Array::from(inner_target_index)),
            Arc::new(Float64Array::from(inner_target_value)),
            Arc::new(BooleanArray::from(inner_target_valid)),
            Arc::new(StringArray::from(inner_target_failure)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("outer-records", outer_records),
        table_v1("outer-draw-rows", outer_rows),
        table_v1("inner-records", inner_records),
        table_v1("inner-draw-rows", inner_rows),
        table_v1("inner-target-vectors", inner_targets),
        table_v1(
            "usable-outer-indices",
            usable_indices_batch_v1(&value.usable_outer_indices)?,
        ),
    ];
    if outer_width > 0 {
        tables.push(table_v1(
            "outer-target-vectors",
            multimod_target_ledger_batch_v1(
                outer_target_replicate,
                outer_target_id,
                outer_target_value,
                outer_target_valid,
                outer_target_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

fn encode_interventional_bootstrap_v1(
    value: &PreparedInterventionalBootstrapV1,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        "interventional_bootstrap",
        vec![
            metadata_u64("ledger.master_seed", value.ledger.master_seed)?,
            metadata_string("ledger.domain", value.ledger.domain.clone()),
            metadata_u64("ledger.requested", value.ledger.requested)?,
            metadata_bool(
                "complete_observed_equation_refit_per_path_and_replicate",
                value.complete_observed_equation_refit_per_path_and_replicate,
            ),
            metadata_bool(
                "g_computation_repeated_per_replicate",
                value.g_computation_repeated_per_replicate,
            ),
            metadata_bool(
                "fixed_identification_and_positivity_contract",
                value.fixed_identification_and_positivity_contract,
            ),
        ],
    )?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1(
            "resample-ledger",
            prepared_resample_ledger_batch_v1(&value.ledger)?,
        ),
    ];
    tables.extend(prepared_target_tables_v1(
        &value.ledger,
        &value.targets,
        None,
    )?);
    Ok(tables)
}

fn encode_interventional_full_refit_ledger_v1(
    value: &MultiModFinalLedgerV1<qpls_resampling::MultiModCaseBootstrapDrawV1, Vec<f64>>,
) -> Result<Vec<EvidenceTableV1>, String> {
    encode_case_full_refit_ledger_v1(value, "interventional_full_refit_ledger")
}

fn encode_case_full_refit_ledger_v1(
    value: &ConditionalCaseBootstrapLedgerV2,
    evidence_kind: &str,
) -> Result<Vec<EvidenceTableV1>, String> {
    let metadata = metadata_batch_v1(
        evidence_kind,
        vec![
            metadata_u64("schema_version", value.schema_version)?,
            metadata_string("method_version", value.method_version.clone()),
            metadata_string(
                "execution_identity_sha256",
                value.execution_identity_sha256.clone(),
            ),
            metadata_u64("requested", value.requested)?,
            metadata_u64("usable", value.usable)?,
            metadata_u64("minimum_required", value.minimum_required)?,
            metadata_number("usable_fraction", value.usable_fraction)?,
            metadata_bool("complete", value.complete),
            metadata_string("ledger_sha256", value.ledger_sha256.clone()),
        ],
    )?;
    let target_width = value
        .records
        .iter()
        .find_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => Some(value.len()),
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .unwrap_or_default();
    if value.records.iter().any(|record| {
        matches!(
            &record.outcome,
            MultiModRefitOutcomeV1::Success { value, .. } if value.len() != target_width
        )
    }) {
        return Err(format!(
            "{evidence_kind} target width changed across replicates"
        ));
    }
    let mut record_index = Vec::new();
    let mut attempt_count = Vec::new();
    let mut draw_replicate_index = Vec::new();
    let mut source_rows_sha256 = Vec::new();
    let mut case_weights_sha256 = Vec::new();
    let mut draw_identity_sha256 = Vec::new();
    let mut outcome_usable = Vec::new();
    let mut outcome_identity_sha256 = Vec::new();
    let mut failure_code = Vec::new();
    let mut failure_message = Vec::new();
    let mut record_identity_sha256 = Vec::new();
    let mut draw_record_index = Vec::new();
    let mut draw_position = Vec::new();
    let mut draw_source_row = Vec::new();
    let mut draw_case_weight = Vec::new();
    let mut target_replicate = Vec::new();
    let mut target_id = Vec::new();
    let mut target_value = Vec::new();
    let mut target_valid = Vec::new();
    let mut target_failure = Vec::new();
    for record in &value.records {
        if record
            .draw
            .case_weights
            .as_ref()
            .is_some_and(|weights| weights.len() != record.draw.source_rows.len())
        {
            return Err(format!(
                "{evidence_kind} draw case-weight width does not match source rows"
            ));
        }
        record_index.push(record.index);
        attempt_count.push(record.attempt_count);
        draw_replicate_index.push(record.draw.replicate_index);
        source_rows_sha256.push(record.draw.source_rows_sha256.clone());
        case_weights_sha256.push(record.draw.case_weights_sha256.clone());
        draw_identity_sha256.push(record.draw.draw_identity_sha256.clone());
        record_identity_sha256.push(record.record_identity_sha256.clone());
        let (estimate, outcome_hash, code, message) = match &record.outcome {
            MultiModRefitOutcomeV1::Success {
                value,
                value_sha256,
            } => (
                Some(value.as_slice()),
                value_sha256.clone(),
                String::new(),
                String::new(),
            ),
            MultiModRefitOutcomeV1::Failed {
                failure,
                failure_sha256,
            } => (
                None,
                failure_sha256.clone(),
                failure.code.clone(),
                failure.message.clone(),
            ),
        };
        outcome_usable.push(estimate.is_some());
        outcome_identity_sha256.push(outcome_hash);
        failure_code.push(code.clone());
        failure_message.push(message);
        for (position, source_row) in record.draw.source_rows.iter().enumerate() {
            draw_record_index.push(record.index);
            draw_position.push(u32_checked(position, "case-bootstrap draw position")?);
            draw_source_row.push(*source_row);
            draw_case_weight.push(
                record
                    .draw
                    .case_weights
                    .as_ref()
                    .map(|weights| weights[position]),
            );
        }
        for target_index in 0..target_width {
            let observed = estimate.map(|values| values[target_index]);
            target_replicate.push(record.index);
            target_id.push(format!("target_index:{target_index}"));
            target_value.push(observed.unwrap_or_default());
            target_valid.push(observed.is_some());
            target_failure.push(if observed.is_some() {
                String::new()
            } else if code.is_empty() {
                "missing_target_estimate".into()
            } else {
                code.clone()
            });
        }
    }
    let record_schema = Arc::new(Schema::new(vec![
        Field::new("record_index", DataType::UInt32, false),
        Field::new("attempt_count", DataType::UInt8, false),
        Field::new("draw_replicate_index", DataType::UInt32, false),
        Field::new("source_rows_sha256", DataType::Utf8, false),
        Field::new("case_weights_sha256", DataType::Utf8, true),
        Field::new("draw_identity_sha256", DataType::Utf8, false),
        Field::new("outcome_usable", DataType::Boolean, false),
        Field::new("outcome_identity_sha256", DataType::Utf8, false),
        Field::new("failure_code", DataType::Utf8, false),
        Field::new("failure_message", DataType::Utf8, false),
        Field::new("record_identity_sha256", DataType::Utf8, false),
    ]));
    let records = RecordBatch::try_new(
        record_schema,
        vec![
            Arc::new(UInt32Array::from(record_index)),
            Arc::new(UInt8Array::from(attempt_count)),
            Arc::new(UInt32Array::from(draw_replicate_index)),
            Arc::new(StringArray::from(source_rows_sha256)),
            Arc::new(StringArray::from(case_weights_sha256)),
            Arc::new(StringArray::from(draw_identity_sha256)),
            Arc::new(BooleanArray::from(outcome_usable)),
            Arc::new(StringArray::from(outcome_identity_sha256)),
            Arc::new(StringArray::from(failure_code)),
            Arc::new(StringArray::from(failure_message)),
            Arc::new(StringArray::from(record_identity_sha256)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let draw_schema = Arc::new(Schema::new(vec![
        Field::new("record_index", DataType::UInt32, false),
        Field::new("draw_position", DataType::UInt32, false),
        Field::new("source_row", DataType::UInt32, false),
        Field::new("case_weight", DataType::Float64, true),
    ]));
    let draw_rows = RecordBatch::try_new(
        draw_schema,
        vec![
            Arc::new(UInt32Array::from(draw_record_index)),
            Arc::new(UInt32Array::from(draw_position)),
            Arc::new(UInt32Array::from(draw_source_row)),
            Arc::new(Float64Array::from(draw_case_weight)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let usable_schema = Arc::new(Schema::new(vec![Field::new(
        "record_index",
        DataType::UInt32,
        false,
    )]));
    let usable_indices = RecordBatch::try_new(
        usable_schema,
        vec![Arc::new(UInt32Array::from(value.usable_indices.clone()))],
    )
    .map_err(|error| error.to_string())?;
    let mut tables = vec![
        table_v1("metadata", metadata),
        table_v1("records", records),
        table_v1("draw-rows", draw_rows),
        table_v1("usable-indices", usable_indices),
    ];
    if target_width > 0 {
        tables.push(table_v1(
            "target-vectors",
            multimod_target_ledger_batch_v1(
                target_replicate,
                target_id,
                target_value,
                target_valid,
                target_failure,
            )
            .map_err(|error| error.to_string())?,
        ));
    }
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_receipts_are_typed_columns_not_json_envelopes() {
        let batch = metadata_batch_v1(
            "fixture",
            vec![
                metadata_string("method_version", "fixture.v1"),
                metadata_u64("requested", 5_000usize).unwrap(),
                metadata_number("confidence", 0.95).unwrap(),
                metadata_bool("complete", true),
            ],
        )
        .unwrap();
        let names = batch
            .schema()
            .fields()
            .iter()
            .map(|field| field.name().clone())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "evidence_kind".to_owned(),
                "field_name".to_owned(),
                "value_type".to_owned(),
                "string_value".to_owned(),
                "unsigned_value".to_owned(),
                "number_value".to_owned(),
                "boolean_value".to_owned(),
            ]
        );
        assert_eq!(batch.num_rows(), 4);
    }

    #[test]
    fn analysis_row_tokens_require_one_unambiguous_raw_receipt() {
        let receipt = RawHeterogeneityPreparationReceiptV2 {
            method_version: "qpls.heterogeneity.raw-preparation.v2".into(),
            general_sem_plan_sha256: "a".repeat(64),
            pooled_metric_sha256: "b".repeat(64),
            source_row_tokens: vec![2, 7],
            omitted_source_rows: 3,
            unique_analysis_positions: true,
            fimix_input: qpls_estimation::StandardizedFimixInputV2 {
                interaction_profile:
                    qpls_estimation::HeterogeneityInteractionProfileV2::P0Structural,
                metric: qpls_estimation::PooledStandardizedMetricReceiptV2 {
                    metric_id: "fixture".into(),
                    source_sha256: "b".repeat(64),
                    observation_count: 2,
                    scores_standardized_once_on_pooled_rows: true,
                    products_standardized_once_on_pooled_rows: false,
                },
                equations: vec![qpls_estimation::StandardizedStructuralEquationV2 {
                    equation_id: "eq:y".into(),
                    outcome_id: "y".into(),
                    predictor_ids: vec!["x".into()],
                    design: vec![vec![-1.0], vec![1.0]],
                    outcome: vec![-1.0, 1.0],
                    include_intercept: false,
                }],
            },
        };
        let evidence = vec![MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(
            receipt,
        )];
        assert_eq!(
            analysis_row_tokens_v1("dataset", &evidence).unwrap(),
            Some(vec![
                "qpls.row.v1:dataset:2".into(),
                "qpls.row.v1:dataset:7".into(),
            ])
        );
    }
}
