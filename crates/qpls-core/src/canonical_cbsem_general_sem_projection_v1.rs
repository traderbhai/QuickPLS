//! Deterministic presentation projection for the bounded CB-SEM General SEM
//! payload. Both the native result builder and schema-6 reopen validator call
//! this code, so tables cannot drift from the typed scientific authority.

use crate::{
    CanonicalCbsemBootstrapInferenceOutcomeV1, CanonicalCbsemEndpointV1,
    CanonicalCbsemParameterRoleV1, CanonicalCbsemParameterStateV1, CanonicalCbsemParameterTargetV1,
    CanonicalColumnRole, CanonicalColumnType, CanonicalGeneralSemResultsV1, CanonicalMissingReason,
    CanonicalResultCell, CanonicalResultColumn, CanonicalResultRow, CanonicalResultTable,
    CapabilityCellReferenceV2, cbsem_general_sem_ml_capability_cell_v1,
    cbsem_recursive_sem_bootstrap_capability_cell_v1,
};

pub const CBSEM_GENERAL_SEM_PARAMETERS_TABLE_ID_V1: &str = "cbsem_general_sem_parameters";
pub const CBSEM_GENERAL_SEM_FIT_TABLE_ID_V1: &str = "cbsem_general_sem_fit";
pub const CBSEM_GENERAL_SEM_IDENTIFICATION_TABLE_ID_V1: &str = "cbsem_general_sem_identification";
pub const CBSEM_RECURSIVE_SEM_BOOTSTRAP_INFERENCE_TABLE_ID_V1: &str =
    "cbsem_recursive_sem_bootstrap_inference";
pub const CBSEM_RECURSIVE_SEM_BOOTSTRAP_RECEIPT_TABLE_ID_V1: &str =
    "cbsem_recursive_sem_bootstrap_receipt";
pub const CBSEM_RECURSIVE_SEM_BOOTSTRAP_FAILURES_TABLE_ID_V1: &str =
    "cbsem_recursive_sem_bootstrap_failures";

pub fn canonical_cbsem_general_sem_tables_v1(
    results: &CanonicalGeneralSemResultsV1,
) -> Vec<CanonicalResultTable> {
    let point_cell = cbsem_general_sem_ml_capability_cell_v1();
    let mut tables = vec![
        parameter_table(results, &point_cell),
        fit_table(results, &point_cell),
        identification_table(results, &point_cell),
    ];
    if let Some(receipt) = &results.cbsem_bootstrap_receipt {
        let bootstrap_cell = cbsem_recursive_sem_bootstrap_capability_cell_v1();
        tables.extend([
            bootstrap_inference_table(results, &bootstrap_cell),
            bootstrap_receipt_table(receipt, &bootstrap_cell),
            bootstrap_failures_table(receipt, &bootstrap_cell),
        ]);
    }
    tables
}

fn parameter_table(
    results: &CanonicalGeneralSemResultsV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    let columns = vec![
        text_column("parameter_id", "Parameter ID", CanonicalColumnRole::Label),
        text_column("role", "Role", CanonicalColumnRole::Label),
        text_column("target_kind", "Target kind", CanonicalColumnRole::Label),
        text_column("source", "Source", CanonicalColumnRole::Label),
        text_column("target", "Target", CanonicalColumnRole::Label),
        text_column("relation_id", "Relation ID", CanonicalColumnRole::Label),
        text_column("state", "State", CanonicalColumnRole::Diagnostic),
        number_column(
            "fixed_value",
            "Fixed value",
            CanonicalColumnRole::Diagnostic,
        ),
        text_column(
            "equality_label",
            "Equality label",
            CanonicalColumnRole::Diagnostic,
        ),
        number_column(
            "lower_bound",
            "Lower bound",
            CanonicalColumnRole::Diagnostic,
        ),
        number_column(
            "upper_bound",
            "Upper bound",
            CanonicalColumnRole::Diagnostic,
        ),
        number_column("estimate", "Estimate", CanonicalColumnRole::Estimate),
        number_column("standard_error", "SE", CanonicalColumnRole::Uncertainty),
        number_column("z_value", "z", CanonicalColumnRole::Uncertainty),
        number_column("p_value", "p", CanonicalColumnRole::Decision),
        number_column(
            "standardized_estimate",
            "Standardized estimate",
            CanonicalColumnRole::Estimate,
        ),
    ];
    let rows = results
        .cbsem_parameters
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let (target_kind, source, target) = target_cells(&row.target);
            let (state, fixed, equality, lower, upper) = state_cells(&row.state);
            CanonicalResultRow {
                id: format!("cbsem_parameter_{index:04}"),
                cells: vec![
                    text(&row.parameter_id),
                    text(match row.role {
                        CanonicalCbsemParameterRoleV1::Loading => "loading",
                        CanonicalCbsemParameterRoleV1::Regression => "regression",
                        CanonicalCbsemParameterRoleV1::Covariance => "covariance",
                        CanonicalCbsemParameterRoleV1::Variance => "variance",
                    }),
                    text(target_kind),
                    source,
                    target,
                    optional_text(row.relation_id.as_deref()),
                    text(state),
                    optional_number(fixed),
                    optional_text(equality),
                    optional_number(lower),
                    optional_number(upper),
                    number(row.estimate),
                    optional_number(row.standard_error),
                    optional_number(row.z_value),
                    optional_number(row.p_value),
                    optional_number(row.standardized_estimate),
                ],
            }
        })
        .collect();
    table(
        CBSEM_GENERAL_SEM_PARAMETERS_TABLE_ID_V1,
        "CB-SEM parameter table",
        "Typed estimates mapped one-to-one to the strict resident parameter table.",
        columns,
        rows,
        capability_cell,
    )
}

fn fit_table(
    results: &CanonicalGeneralSemResultsV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    let columns = vec![
        text_column("fit_id", "Fit ID", CanonicalColumnRole::Label),
        number_column("chi_square", "Chi-square", CanonicalColumnRole::Diagnostic),
        number_column("degrees_of_freedom", "df", CanonicalColumnRole::Diagnostic),
        number_column("chi_square_p_value", "p", CanonicalColumnRole::Decision),
        number_column("rmsea", "RMSEA", CanonicalColumnRole::Diagnostic),
        number_column(
            "rmsea_ci_level",
            "RMSEA CI level",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "rmsea_ci_lower",
            "RMSEA CI lower",
            CanonicalColumnRole::Uncertainty,
        ),
        number_column(
            "rmsea_ci_upper",
            "RMSEA CI upper",
            CanonicalColumnRole::Uncertainty,
        ),
        number_column("cfi", "CFI", CanonicalColumnRole::Diagnostic),
        number_column("tli", "TLI", CanonicalColumnRole::Diagnostic),
        number_column("srmr", "SRMR", CanonicalColumnRole::Diagnostic),
        number_column("aic", "AIC", CanonicalColumnRole::Diagnostic),
        number_column("bic", "BIC", CanonicalColumnRole::Diagnostic),
    ];
    let rows = results
        .cbsem_fit
        .iter()
        .enumerate()
        .map(|(index, fit)| CanonicalResultRow {
            id: format!("cbsem_fit_{index:04}"),
            cells: vec![
                text(&fit.fit_id),
                number(fit.chi_square),
                number(f64::from(fit.degrees_of_freedom)),
                optional_number(fit.chi_square_p_value),
                optional_number(fit.rmsea),
                optional_number(
                    fit.rmsea_interval
                        .as_ref()
                        .map(|value| value.confidence_level),
                ),
                optional_number(fit.rmsea_interval.as_ref().map(|value| value.lower)),
                optional_number(fit.rmsea_interval.as_ref().map(|value| value.upper)),
                optional_number(fit.cfi),
                optional_number(fit.tli),
                optional_number(fit.srmr),
                optional_number(fit.aic),
                optional_number(fit.bic),
            ],
        })
        .collect();
    table(
        CBSEM_GENERAL_SEM_FIT_TABLE_ID_V1,
        "CB-SEM model fit",
        "Normal-theory ML fit statistics from the same point result.",
        columns,
        rows,
        capability_cell,
    )
}

fn identification_table(
    results: &CanonicalGeneralSemResultsV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    let columns = vec![
        text_column("diagnostic_id", "Diagnostic ID", CanonicalColumnRole::Label),
        text_column("scope", "Scope", CanonicalColumnRole::Diagnostic),
        text_column("subject_id", "Subject ID", CanonicalColumnRole::Label),
        text_column("status", "Status", CanonicalColumnRole::Decision),
        text_column("code", "Code", CanonicalColumnRole::Diagnostic),
        text_column("message", "Message", CanonicalColumnRole::Diagnostic),
        number_column("degrees_of_freedom", "df", CanonicalColumnRole::Diagnostic),
    ];
    let rows = results
        .identification_diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| CanonicalResultRow {
            id: format!("cbsem_identification_{index:04}"),
            cells: vec![
                text(&diagnostic.diagnostic_id),
                text(enum_token(&diagnostic.scope)),
                text(&diagnostic.subject_id),
                text(enum_token(&diagnostic.status)),
                text(&diagnostic.code),
                text(&diagnostic.message),
                diagnostic
                    .degrees_of_freedom
                    .map(|value| number(value as f64))
                    .unwrap_or_else(missing),
            ],
        })
        .collect();
    table(
        CBSEM_GENERAL_SEM_IDENTIFICATION_TABLE_ID_V1,
        "CB-SEM identification evidence",
        "Conservative identification evidence compiled from the resident model.",
        columns,
        rows,
        capability_cell,
    )
}

fn bootstrap_inference_table(
    results: &CanonicalGeneralSemResultsV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    let columns = vec![
        text_column("parameter_id", "Parameter ID", CanonicalColumnRole::Label),
        number_column(
            "point_estimate",
            "Point estimate",
            CanonicalColumnRole::Estimate,
        ),
        text_column("status", "Status", CanonicalColumnRole::Decision),
        text_column(
            "unavailable_reason",
            "Unavailable reason",
            CanonicalColumnRole::Diagnostic,
        ),
        number_column(
            "bootstrap_mean",
            "Bootstrap mean",
            CanonicalColumnRole::Estimate,
        ),
        number_column("bootstrap_bias", "Bias", CanonicalColumnRole::Diagnostic),
        number_column("standard_error", "SE", CanonicalColumnRole::Uncertainty),
        number_column("lower", "Lower", CanonicalColumnRole::Uncertainty),
        number_column("upper", "Upper", CanonicalColumnRole::Uncertainty),
        number_column("p_value", "p", CanonicalColumnRole::Decision),
        number_column(
            "usable_replicates",
            "Usable",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "two_sided_exceedances",
            "Exceedances",
            CanonicalColumnRole::Provenance,
        ),
    ];
    let rows = results
        .cbsem_bootstrap_inference
        .iter()
        .enumerate()
        .map(|(index, inference)| {
            let (status, reason, value) = match &inference.outcome {
                CanonicalCbsemBootstrapInferenceOutcomeV1::Available { value } => {
                    ("available", None, Some(value))
                }
                CanonicalCbsemBootstrapInferenceOutcomeV1::Unavailable { reason } => {
                    ("unavailable", Some(enum_token(reason)), None)
                }
            };
            CanonicalResultRow {
                id: format!("cbsem_bootstrap_inference_{index:04}"),
                cells: vec![
                    text(&inference.parameter_id),
                    number(inference.point_estimate),
                    text(status),
                    optional_text(reason.as_deref()),
                    optional_number(value.and_then(|item| item.bootstrap_mean)),
                    optional_number(value.and_then(|item| item.bootstrap_bias)),
                    optional_number(value.and_then(|item| item.standard_error)),
                    optional_number(value.and_then(|item| item.lower)),
                    optional_number(value.and_then(|item| item.upper)),
                    optional_number(value.and_then(|item| item.p_value)),
                    optional_number(
                        value
                            .and_then(|item| item.bootstrap_usable_replicates)
                            .map(f64::from),
                    ),
                    optional_number(
                        value
                            .and_then(|item| item.bootstrap_two_sided_exceedances)
                            .map(f64::from),
                    ),
                ],
            }
        })
        .collect();
    table(
        CBSEM_RECURSIVE_SEM_BOOTSTRAP_INFERENCE_TABLE_ID_V1,
        "Recursive-SEM bootstrap inference",
        "Exact percentile Type-7 case-bootstrap inference for eligible free parameters.",
        columns,
        rows,
        capability_cell,
    )
}

fn bootstrap_receipt_table(
    receipt: &crate::CanonicalCbsemBootstrapReceiptV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    let fields = [
        ("method_version", receipt.method_version.as_str()),
        (
            "resampling_operation_version",
            receipt.resampling_operation_version.as_str(),
        ),
        (
            "quantile_method_version",
            receipt.quantile_method_version.as_str(),
        ),
        (
            "compiled_plan_sha256",
            receipt.compiled_plan_sha256.as_str(),
        ),
        ("base_plan_sha256", receipt.base_plan_sha256.as_str()),
        (
            "parameter_inventory_sha256",
            receipt.parameter_inventory_sha256.as_str(),
        ),
        (
            "model_scientific_sha256",
            receipt.model_scientific_sha256.as_str(),
        ),
        (
            "general_sem_config_sha256",
            receipt.general_sem_config_sha256.as_str(),
        ),
        (
            "recipe_analytical_sha256",
            receipt.recipe_analytical_sha256.as_str(),
        ),
        (
            "source_dataset_fingerprint",
            receipt.source_dataset_fingerprint.as_str(),
        ),
        (
            "complete_case_frame_sha256",
            receipt.complete_case_frame_sha256.as_str(),
        ),
        (
            "usable_replicate_indices_sha256",
            receipt.usable_replicate_indices_sha256.as_str(),
        ),
        ("seed", receipt.seed.as_str()),
    ];
    let mut columns = fields
        .iter()
        .map(|(id, _)| text_column(id, id, CanonicalColumnRole::Provenance))
        .collect::<Vec<_>>();
    columns.extend([
        number_column(
            "confidence_level",
            "Confidence level",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "resamples_requested",
            "Requested",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "resamples_usable",
            "Usable",
            CanonicalColumnRole::Provenance,
        ),
        number_column(
            "minimum_usable_resamples",
            "Minimum usable",
            CanonicalColumnRole::Provenance,
        ),
        number_column("workers", "Workers", CanonicalColumnRole::Provenance),
        boolean_column(
            "complete_model_reestimated",
            "Complete refit",
            CanonicalColumnRole::Provenance,
        ),
    ]);
    let mut cells = fields
        .iter()
        .map(|(_, value)| text(*value))
        .collect::<Vec<_>>();
    cells.extend([
        number(receipt.confidence_level),
        number(f64::from(receipt.resamples_requested)),
        number(f64::from(receipt.resamples_usable)),
        number(f64::from(receipt.minimum_usable_resamples)),
        number(f64::from(receipt.workers)),
        CanonicalResultCell::Boolean {
            value: receipt.complete_model_reestimated_per_replicate,
        },
    ]);
    table(
        CBSEM_RECURSIVE_SEM_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
        "Recursive-SEM bootstrap receipt",
        "Archive-bound resampling and compilation provenance.",
        columns,
        vec![CanonicalResultRow {
            id: "cbsem_bootstrap_receipt".into(),
            cells,
        }],
        capability_cell,
    )
}

fn bootstrap_failures_table(
    receipt: &crate::CanonicalCbsemBootstrapReceiptV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    table(
        CBSEM_RECURSIVE_SEM_BOOTSTRAP_FAILURES_TABLE_ID_V1,
        "Recursive-SEM bootstrap failures",
        "Ordered no-retry failed-replicate ledger.",
        vec![
            number_column("replicate_index", "Replicate", CanonicalColumnRole::Label),
            text_column("reason_code", "Reason", CanonicalColumnRole::Diagnostic),
            text_column("message", "Message", CanonicalColumnRole::Diagnostic),
        ],
        receipt
            .failed_replicates
            .iter()
            .enumerate()
            .map(|(index, failure)| CanonicalResultRow {
                id: format!("cbsem_bootstrap_failure_{index:04}"),
                cells: vec![
                    number(f64::from(failure.replicate_index)),
                    text(enum_token(&failure.reason_code)),
                    text(&failure.message),
                ],
            })
            .collect(),
        capability_cell,
    )
}

fn target_cells(
    target: &CanonicalCbsemParameterTargetV1,
) -> (&'static str, CanonicalResultCell, CanonicalResultCell) {
    match target {
        CanonicalCbsemParameterTargetV1::Loading {
            factor_id,
            indicator_id,
        } => ("loading", text(factor_id), text(indicator_id)),
        CanonicalCbsemParameterTargetV1::Regression {
            source_id,
            target_id,
        } => ("regression", text(source_id), text(target_id)),
        CanonicalCbsemParameterTargetV1::Covariance { left, right } => (
            "covariance",
            text(endpoint_token(left)),
            text(endpoint_token(right)),
        ),
        CanonicalCbsemParameterTargetV1::Variance { endpoint } => {
            ("variance", missing(), text(endpoint_token(endpoint)))
        }
    }
}

fn endpoint_token(endpoint: &CanonicalCbsemEndpointV1) -> String {
    match endpoint {
        CanonicalCbsemEndpointV1::Variable { variable_id } => format!("variable:{variable_id}"),
        CanonicalCbsemEndpointV1::Residual { variable_id } => format!("residual:{variable_id}"),
        CanonicalCbsemEndpointV1::Disturbance { variable_id } => {
            format!("disturbance:{variable_id}")
        }
    }
}

fn state_cells(
    state: &CanonicalCbsemParameterStateV1,
) -> (
    &'static str,
    Option<f64>,
    Option<&str>,
    Option<f64>,
    Option<f64>,
) {
    match state {
        CanonicalCbsemParameterStateV1::Fixed { value } => {
            ("fixed", Some(*value), None, None, None)
        }
        CanonicalCbsemParameterStateV1::Free {
            equality_label,
            lower,
            upper,
        } => ("free", None, equality_label.as_deref(), *lower, *upper),
    }
}

fn enum_token<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("canonical CB-SEM enum serialization cannot fail")
        .trim_matches('"')
        .to_owned()
}

fn table(
    id: &str,
    title: &str,
    description: &str,
    columns: Vec<CanonicalResultColumn>,
    rows: Vec<CanonicalResultRow>,
    capability_cell: &CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    CanonicalResultTable {
        id: id.into(),
        title: title.into(),
        description: Some(description.into()),
        columns,
        rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    }
}

fn text_column(id: &str, label: &str, role: CanonicalColumnRole) -> CanonicalResultColumn {
    column(id, label, CanonicalColumnType::Text, role)
}

fn number_column(id: &str, label: &str, role: CanonicalColumnRole) -> CanonicalResultColumn {
    column(id, label, CanonicalColumnType::Number, role)
}

fn boolean_column(id: &str, label: &str, role: CanonicalColumnRole) -> CanonicalResultColumn {
    column(id, label, CanonicalColumnType::Boolean, role)
}

fn column(
    id: &str,
    label: &str,
    data_type: CanonicalColumnType,
    role: CanonicalColumnRole,
) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type,
        description: label.into(),
        role: Some(role),
        unit: None,
        default_precision: (data_type == CanonicalColumnType::Number).then_some(4),
    }
}

fn text(value: impl Into<String>) -> CanonicalResultCell {
    CanonicalResultCell::Text {
        value: value.into(),
    }
}

fn number(value: f64) -> CanonicalResultCell {
    CanonicalResultCell::Number {
        value,
        display: None,
    }
}

fn optional_number(value: Option<f64>) -> CanonicalResultCell {
    value.map(number).unwrap_or_else(missing)
}

fn optional_text(value: Option<&str>) -> CanonicalResultCell {
    value.map(text).unwrap_or_else(missing)
}

fn missing() -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason: CanonicalMissingReason::NotEstimated,
        display: None,
    }
}
