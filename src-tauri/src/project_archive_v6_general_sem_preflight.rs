//! Authoritative Registry-driven estimator preflight for schema-6 General SEM.
//!
//! The frontend may provide a fast preview, but only this bridge evaluates the
//! exact Rust compiler contracts. It never mutates or persists project data.

use crate::general_sem_registry_access_v1::{
    GENERAL_SEM_INTERNAL_LABS_SURFACE as INTERNAL_LABS_SURFACE,
    GENERAL_SEM_STANDARD_SURFACE as STANDARD_SURFACE, GeneralSemRegistryAccessErrorV1,
    authorize_general_sem_registry_access_v1,
    decision_declares_general_sem_execution_cell_v1, is_general_sem_execution_cell_v1,
    is_rank3_general_sem_cbsem_execution_cell_v1,
    selected_general_sem_cbsem_execution_cell_v1, selected_general_sem_execution_cell_v1,
};
use qpls_core::{
    CapabilityCellReferenceV2, GeneralSemConfigV1, SemCapabilityDecisionV1, SemDataBindingV4,
    SemModelV4, SemParameterV4, SemVariableV4, preflight_general_sem_cbsem_v1,
    preflight_general_sem_pls_v1, sha256_serialized,
};
use qpls_data::{ColumnType, DataKind, ScaleType};
use qpls_project::{ProjectArchiveDocumentV6, ProjectModelPayloadV6, ProjectModelRecordV6};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION: u32 = 2;
const DIAGNOSTIC_CODE_PREFIX: &str = "schema6_general_sem_preflight";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    capability_cell: CapabilityCellReferenceV2,
    project: ProjectArchiveDocumentV6,
    model_id: String,
    config: GeneralSemConfigV1,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightResultV1 {
    schema_version: u32,
    pls: SemCapabilityDecisionV1,
    cbsem: SemCapabilityDecisionV1,
    authority: GeneralSemEstimatorPreflightAuthorityV2,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightAuthorityV2 {
    source: String,
    model_id: String,
    model_scientific_sha256: String,
    parameter_table_sha256: String,
    parameter_count: usize,
    free_parameter_count: usize,
    fixed_parameter_count: usize,
    derived_parameter_count: usize,
    equality_labeled_parameter_count: usize,
    bounded_parameter_count: usize,
    explicit_constraint_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum GeneralSemEstimatorPreflightOutcomeV1 {
    Ok {
        value: GeneralSemEstimatorPreflightResultV1,
    },
    Blocked {
        diagnostic: GeneralSemEstimatorPreflightDiagnosticV1,
    },
}

fn blocked(
    code_suffix: &str,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> GeneralSemEstimatorPreflightOutcomeV1 {
    GeneralSemEstimatorPreflightOutcomeV1::Blocked {
        diagnostic: GeneralSemEstimatorPreflightDiagnosticV1 {
            code: format!("{DIAGNOSTIC_CODE_PREFIX}.{code_suffix}"),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn promoted_model_from_record(record: &ProjectModelRecordV6) -> Option<(&SemModelV4, &str)> {
    match &record.payload {
        ProjectModelPayloadV6::SemModelV4 {
            model,
            scientific_sha256,
        } => Some((model, scientific_sha256)),
        ProjectModelPayloadV6::SemModelV4Draft { .. }
        | ProjectModelPayloadV6::LegacyEstimandUnspecified { .. } => None,
    }
}

fn parameter_table_authority(
    model: &SemModelV4,
    scientific_sha256: &str,
) -> GeneralSemEstimatorPreflightAuthorityV2 {
    let mut free_parameter_count = 0;
    let mut fixed_parameter_count = 0;
    let mut derived_parameter_count = 0;
    let mut equality_labeled_parameter_count = 0;
    let mut bounded_parameter_count = 0;
    for parameter in &model.parameters {
        match parameter {
            SemParameterV4::Free {
                lower,
                upper,
                equality_label,
                ..
            } => {
                free_parameter_count += 1;
                equality_labeled_parameter_count += usize::from(equality_label.is_some());
                bounded_parameter_count += usize::from(lower.is_some() || upper.is_some());
            }
            SemParameterV4::Fixed { .. } => fixed_parameter_count += 1,
            SemParameterV4::Derived { .. } => derived_parameter_count += 1,
        }
    }
    GeneralSemEstimatorPreflightAuthorityV2 {
        source: "resident_schema6_sem_model_v4_parameter_table".into(),
        model_id: model.id.clone(),
        model_scientific_sha256: scientific_sha256.into(),
        parameter_table_sha256: sha256_serialized(&model.parameters),
        parameter_count: model.parameters.len(),
        free_parameter_count,
        fixed_parameter_count,
        derived_parameter_count,
        equality_labeled_parameter_count,
        bounded_parameter_count,
        explicit_constraint_count: model.constraints.len(),
    }
}

fn preflight_general_sem_estimators(
    request: GeneralSemEstimatorPreflightRequestV1,
) -> GeneralSemEstimatorPreflightOutcomeV1 {
    if !is_general_sem_execution_cell_v1(&request.capability_cell) {
        return blocked(
            "capability_cell_not_general_sem_execution",
            "The requested Registry cell does not own a bounded General SEM execution workflow.",
            "Refresh estimator compatibility and use the exact cell selected for the resident model and inference settings.",
        );
    }
    if let Err(error) = authorize_general_sem_registry_access_v1(
        &request.surface,
        request.experimental_labs_enabled,
        &request.capability_cell,
    ) {
        return match error {
            GeneralSemRegistryAccessErrorV1::RegistryInvalid(detail) => blocked(
                "capability_registry_invalid",
                format!("Capability Registry V2 is invalid: {detail}"),
                "Keep the project unchanged and repair the embedded registry before preflight.",
            ),
            GeneralSemRegistryAccessErrorV1::CapabilityUnavailable => blocked(
                "capability_unavailable",
                "The exact requested General SEM option cell is not uniquely executable in Standard or Labs.",
                "Use an exact available Capability Registry V2 cell and rerun estimator preflight.",
            ),
            GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired => blocked(
                "standard_surface_required",
                "The exact requested General SEM option cell is qualified for the Standard surface.",
                "Refresh capability access and rerun preflight through the Standard surface.",
            ),
            GeneralSemRegistryAccessErrorV1::InternalLabsRequired => blocked(
                "internal_labs_required",
                "The exact requested General SEM option cell is available only through Experimental Labs.",
                "Enable Experimental Labs and rerun preflight, or choose a Standard-qualified cell.",
            ),
        };
    }
    if let Err(error) = authorize_general_sem_registry_access_v1(
        &request.surface,
        request.experimental_labs_enabled,
        &request.capability_cell,
    ) {
        return match error {
            GeneralSemRegistryAccessErrorV1::RegistryInvalid(detail) => blocked(
                "capability_registry_invalid",
                format!("Capability Registry V2 is invalid: {detail}"),
                "Keep the project unchanged and restore the embedded Registry before preflight.",
            ),
            GeneralSemRegistryAccessErrorV1::CapabilityUnavailable => blocked(
                "capability_unavailable",
                "The exact requested General SEM execution cell is not uniquely available in Capability Registry V2.",
                "Refresh capability access; do not substitute a neighboring CB-SEM or PLS cell.",
            ),
            GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired => blocked(
                "standard_surface_required",
                "The exact requested General SEM execution cell is qualified for the Standard surface.",
                "Refresh capability access and rerun preflight through Standard.",
            ),
            GeneralSemRegistryAccessErrorV1::InternalLabsRequired => blocked(
                "internal_labs_required",
                "The exact requested General SEM execution cell is available only through Experimental Labs.",
                "Enable Experimental Labs and rerun exact-cell preflight.",
            ),
        };
    }
    if let Err(error) = request.project.ensure_valid() {
        return blocked(
            "project_invalid",
            format!("The supplied schema-6 project is invalid: {error}"),
            "Reopen the strict schema-6 archive and retry from its current project authority.",
        );
    }
    if !request.project.supports_general_sem_v1() {
        return blocked(
            "general_sem_v1_project_required",
            "Advanced estimator preflight requires a newly created schema-6 general_sem_v1 project.",
            "Create a new General SEM project before importing data or authoring the model; upgraded and unmarked projects retain their existing behavior.",
        );
    }

    let Some(record) = request
        .project
        .models
        .iter()
        .find(|record| record.model_id == request.model_id)
    else {
        return blocked(
            "model_not_bound",
            "The requested model is not present in the supplied schema-6 project authority.",
            "Refresh the project and select a current General SEM model before preflight.",
        );
    };
    let Some((authoritative_model, scientific_sha256)) = promoted_model_from_record(record) else {
        return blocked(
            "sem_model_v4_required",
            "The selected project record is not a promoted SemModelV4 scientific authority.",
            "Finish parameter-table authoring and promote the exact resident SemModelV4 before estimator preflight.",
        );
    };
    let authority = parameter_table_authority(authoritative_model, scientific_sha256);
    if let SemDataBindingV4::Raw { dataset_id, .. } = &authoritative_model.data_binding {
        let Ok(dataset_id) = Uuid::parse_str(dataset_id) else {
            return blocked(
                "dataset_binding_invalid",
                "The model raw-data binding does not contain a valid resident dataset identity.",
                "Rebind the model to the exact resident raw dataset before preflight.",
            );
        };
        let Some(dataset) = request
            .project
            .datasets
            .iter()
            .find(|candidate| candidate.id == dataset_id)
        else {
            return blocked(
                "dataset_not_bound",
                "The model dataset is not resident in the supplied schema-6 project authority.",
                "Refresh the project and bind the model to a resident dataset.",
            );
        };
        if dataset.schema.kind != DataKind::Raw {
            return blocked(
                "raw_dataset_required",
                "The exact General SEM PLS slice requires a resident raw case-level dataset.",
                "Choose a raw dataset; retain matrix input for a qualified CB-SEM cell.",
            );
        }
        for variable in &authoritative_model.variables {
            let SemVariableV4::Observed { source_column, .. } = variable else {
                continue;
            };
            let Some(column) = dataset
                .schema
                .columns
                .iter()
                .find(|column| column.name == *source_column)
            else {
                return blocked(
                    "observed_column_missing",
                    format!(
                        "Observed source column {source_column} is absent from the resident dataset."
                    ),
                    "Restore the exact resident source column or correct the observed-variable binding.",
                );
            };
            if column.column_type != ColumnType::Numeric
                || column.scale_type != ScaleType::Continuous
            {
                return blocked(
                    "continuous_numeric_columns_required",
                    format!(
                        "Observed source column {source_column} is not continuous numeric data."
                    ),
                    "Use continuous numeric indicators for this exact PLS cell, or retain the authored semantics for a future qualified cell.",
                );
            }
        }
    }

    let pls = match preflight_general_sem_pls_v1(authoritative_model, &request.config) {
        Ok(decision) => decision,
        Err(error) => {
            return blocked(
                "decision_contract_invalid",
                format!("The PLS capability decision could not satisfy its contract: {error}"),
                "Keep the model unchanged and report this internal capability-decision error.",
            );
        }
    };
    // Capability cells are canonically sorted; exact execution ownership comes
    // from model topology plus inference, never from collection position.
    if !is_rank3_general_sem_cbsem_execution_cell_v1(&request.capability_cell) {
        let selected = selected_general_sem_execution_cell_v1(authoritative_model, &request.config);
        if !decision_declares_general_sem_execution_cell_v1(&pls, &selected)
            || selected != request.capability_cell
        {
            return blocked(
                "capability_cell_mismatch",
                "The requested option cell differs from the exact native PLS decision for this model and config.",
                "Refresh the unchanged project authority and rerun exact capability preflight.",
            );
        }
    }
    let cbsem = match preflight_general_sem_cbsem_v1(authoritative_model, &request.config) {
        Ok(decision) => decision,
        Err(error) => {
            return blocked(
                "decision_contract_invalid",
                format!("The CB-SEM capability decision could not satisfy its contract: {error}"),
                "Keep the model unchanged and report this internal capability-decision error.",
            );
        }
    };
    let selected = if is_rank3_general_sem_cbsem_execution_cell_v1(&request.capability_cell) {
        let selected = selected_general_sem_cbsem_execution_cell_v1(&request.config);
        if !decision_declares_general_sem_execution_cell_v1(&cbsem, &selected) {
            return blocked(
                "capability_decision_missing_selected_cell",
                "The CB-SEM capability decision did not declare its exact point-or-bootstrap execution owner.",
                "Keep the model unchanged and report this internal authority error.",
            );
        }
        selected
    } else {
        let selected = selected_general_sem_execution_cell_v1(authoritative_model, &request.config);
        if !decision_declares_general_sem_execution_cell_v1(&pls, &selected) {
            return blocked(
                "capability_decision_missing_selected_cell",
                "The PLS capability decision did not declare its exact point-or-bootstrap execution owner.",
                "Keep the model unchanged and report this internal authority error.",
            );
        }
        selected
    };
    if selected != request.capability_cell {
        return blocked(
            "capability_cell_mismatch",
            "The requested option cell differs from the exact estimator and inference owner selected from the resident model authority.",
            "Refresh the unchanged schema-6 project and rerun estimator compatibility before calculation.",
        );
    }
    GeneralSemEstimatorPreflightOutcomeV1::Ok {
        value: GeneralSemEstimatorPreflightResultV1 {
            schema_version: GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION,
            pls,
            cbsem,
            authority,
        },
    }
}

#[tauri::command]
pub(crate) fn preflight_internal_general_sem_estimators_v1(
    request: GeneralSemEstimatorPreflightRequestV1,
) -> GeneralSemEstimatorPreflightOutcomeV1 {
    preflight_general_sem_estimators(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        Construct, GeneralSemBootstrapIntervalV1, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec,
        SemCapabilityDecisionStatusV1, SemDataBindingV4, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{DatasetDescriptor, ImportOptions, import_delimited_bytes};
    use qpls_project::{
        ProjectArchiveDocumentV6, ProjectOriginV6, ProjectUpgradeLineageV6,
        SourcePreservationPolicyV6, UpgradeWritePolicyV6, insert_sem_model_v4_draft_v6,
        promote_sem_model_v4_draft_v6,
    };
    use uuid::Uuid;

    fn model() -> SemModelV4 {
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x6001),
                name: "Recursive mediation-ready model".into(),
                constructs: vec![
                    Construct {
                        id: "x".into(),
                        name: "Predictor".into(),
                        short_name: "X".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["x1".into(), "x2".into()],
                    },
                    Construct {
                        id: "m".into(),
                        name: "Mediator".into(),
                        short_name: "M".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["m1".into(), "m2".into()],
                    },
                    Construct {
                        id: "y".into(),
                        name: "Outcome".into(),
                        short_name: "Y".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["y1".into(), "y2".into()],
                    },
                ],
                paths: [("x", "m"), ("m", "y"), ("x", "y")]
                    .into_iter()
                    .map(|(source, target)| StructuralPath {
                        source: source.into(),
                        target: target.into(),
                    })
                    .collect(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn marked_project_and_model() -> (ProjectArchiveDocumentV6, SemModelV4) {
        let dataset = import_delimited_bytes(
            b"x1,x2,m1,m2,y1,y2\n1,2,2,1,3,2\n2,1,3,2,4,3\n3,4,4,3,5,4\n4,3,5,4,6,5\n",
            "general-sem-preflight.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut model = model();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!();
        };
        *dataset_id = dataset.id.to_string();
        let mut project = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x6002),
            "General SEM preflight",
            Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap(),
        );
        project.datasets.push(DatasetDescriptor::from(&dataset));
        let model_document_sha256 = model.model_document_sha256().unwrap();
        let project = insert_sem_model_v4_draft_v6(&project, model.clone()).unwrap();
        let project = promote_sem_model_v4_draft_v6(
            &project,
            &model.id,
            &model_document_sha256,
        )
        .unwrap();
        (project, model)
    }

    fn request() -> GeneralSemEstimatorPreflightRequestV1 {
        let (project, model) = marked_project_and_model();
        GeneralSemEstimatorPreflightRequestV1 {
            surface: STANDARD_SURFACE.into(),
            experimental_labs_enabled: false,
            capability_cell: qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            project,
            model_id: model.id,
            config: GeneralSemConfigV1::default(),
        }
    }

    #[test]
    fn valid_marked_project_returns_authoritative_pls_and_cbsem_decisions() {
        let GeneralSemEstimatorPreflightOutcomeV1::Ok { value } =
            preflight_general_sem_estimators(request())
        else {
            panic!("valid marked General SEM project was blocked")
        };
        assert_eq!(
            value.schema_version,
            GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION
        );
        assert_eq!(
            value.pls.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(value.cbsem.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert_eq!(
            value.authority.source,
            "resident_schema6_sem_model_v4_parameter_table"
        );
        assert_eq!(value.authority.model_id, request().model_id);
        assert_eq!(value.authority.parameter_count, value.authority.free_parameter_count
            + value.authority.fixed_parameter_count
            + value.authority.derived_parameter_count);
        assert_eq!(value.authority.explicit_constraint_count, 0);
        assert_eq!(value.authority.parameter_table_sha256.len(), 64);
    }

    #[test]
    fn labs_gate_unmarked_and_upgraded_projects_fail_closed() {
        let mut denied = request();
        denied.surface = INTERNAL_LABS_SURFACE.into();
        denied.experimental_labs_enabled = true;
        assert!(matches!(
            preflight_general_sem_estimators(denied),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("standard_surface_required")
        ));

        let mut unmarked = request();
        unmarked.project.sem_generation = None;
        assert!(matches!(
            preflight_general_sem_estimators(unmarked),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("general_sem_v1_project_required")
        ));

        let mut upgraded = request();
        upgraded.project.sem_generation = None;
        upgraded.project.origin = ProjectOriginV6::UpgradedCopy {
            lineage: ProjectUpgradeLineageV6 {
                source_project_id: upgraded.project.project_id,
                source_archive_schema_version: 5,
                source_archive_sha256: "a".repeat(64),
                source_archive_path: r"D:\source.qpls".into(),
                destination_archive_path: r"D:\upgraded.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 18, 12, 30, 0).unwrap(),
                source_preservation: SourcePreservationPolicyV6::Required,
                write_policy: UpgradeWritePolicyV6::NewArchiveOnly,
                historical_results_immutable: true,
            },
        };
        assert!(matches!(
            preflight_general_sem_estimators(upgraded),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("general_sem_v1_project_required")
        ));
    }

    #[test]
    fn unbound_model_and_caller_supplied_model_payload_fail_closed() {
        let mut unbound = request();
        unbound.model_id = "model:not-in-project".into();
        assert!(matches!(
            preflight_general_sem_estimators(unbound),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("model_not_bound")
        ));

        let mut wire = serde_json::to_value(request()).unwrap();
        wire["model"] = serde_json::to_value(model()).unwrap();
        assert!(serde_json::from_value::<GeneralSemEstimatorPreflightRequestV1>(wire).is_err());
    }

    #[test]
    fn bootstrap_preflight_rejects_the_point_dependency_as_execution_owner() {
        let mut wrong = request();
        wrong.config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 7,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        assert!(matches!(
            preflight_general_sem_estimators(wrong),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("capability_cell_mismatch")
        ));
    }

    #[test]
    fn missing_or_noncontinuous_resident_dataset_descriptors_fail_closed() {
        let mut missing_dataset = request();
        missing_dataset.project.datasets.clear();
        assert!(matches!(
            preflight_general_sem_estimators(missing_dataset),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("dataset_not_bound")
                    && !diagnostic.corrective_action.is_empty()
        ));

        let mut missing_column = request();
        missing_column.project.datasets[0]
            .schema
            .columns
            .retain(|column| column.name != "x1");
        assert!(matches!(
            preflight_general_sem_estimators(missing_column),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("observed_column_missing")
                    && diagnostic.message.contains("x1")
        ));

        let mut noncontinuous_column = request();
        noncontinuous_column.project.datasets[0]
            .schema
            .columns
            .iter_mut()
            .find(|column| column.name == "x1")
            .unwrap()
            .scale_type = ScaleType::Ordinal;
        assert!(matches!(
            preflight_general_sem_estimators(noncontinuous_column),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("continuous_numeric_columns_required")
                    && diagnostic.message.contains("x1")
        ));
    }
}
