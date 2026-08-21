//! Authoritative Registry-driven estimator preflight for schema-6 General SEM.
//!
//! The frontend may provide a fast preview, but only this bridge evaluates the
//! exact Rust compiler contracts. It never mutates or persists project data.

use crate::general_sem_registry_access_v1::{
    GENERAL_SEM_INTERNAL_LABS_SURFACE as INTERNAL_LABS_SURFACE, GeneralSemRegistryAccessErrorV1,
    authorize_general_sem_registry_access_v1, decision_declares_general_sem_execution_cell_v1,
    selected_general_sem_execution_cell_v1,
};
use qpls_core::{
    CapabilityCellReferenceV2, GeneralSemConfigV1, SemCapabilityDecisionV1, SemDataBindingV4,
    SemModelV4, SemVariableV4, preflight_general_sem_cbsem_v1, preflight_general_sem_pls_v1,
};
use qpls_data::{ColumnType, DataKind, ScaleType};
use qpls_project::{ProjectArchiveDocumentV6, ProjectModelPayloadV6, ProjectModelRecordV6};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION: u32 = 1;
const DIAGNOSTIC_CODE_PREFIX: &str = "schema6_general_sem_preflight";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    capability_cell: CapabilityCellReferenceV2,
    project: ProjectArchiveDocumentV6,
    model: SemModelV4,
    config: GeneralSemConfigV1,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightResultV1 {
    schema_version: u32,
    pls: SemCapabilityDecisionV1,
    cbsem: SemCapabilityDecisionV1,
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

fn model_from_record(record: &ProjectModelRecordV6) -> Option<&SemModelV4> {
    match &record.payload {
        ProjectModelPayloadV6::SemModelV4 { model, .. }
        | ProjectModelPayloadV6::SemModelV4Draft { model, .. } => Some(model),
        ProjectModelPayloadV6::LegacyEstimandUnspecified { .. } => None,
    }
}

fn preflight_general_sem_estimators(
    request: GeneralSemEstimatorPreflightRequestV1,
) -> GeneralSemEstimatorPreflightOutcomeV1 {
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
        .find(|record| record.model_id == request.model.id)
    else {
        return blocked(
            "model_not_bound",
            "The requested model is not present in the supplied schema-6 project authority.",
            "Refresh the project and select a current General SEM model before preflight.",
        );
    };
    let Some(authoritative_model) = model_from_record(record) else {
        return blocked(
            "sem_model_v4_required",
            "The selected project record is legacy estimand-unspecified authority, not SemModelV4.",
            "Author a new SemModelV4 model in the General SEM project.",
        );
    };
    if authoritative_model != &request.model {
        return blocked(
            "model_authority_mismatch",
            "The requested model content differs from the exact model stored in the schema-6 project.",
            "Refresh the model from the strict project authority and rerun estimator preflight.",
        );
    }
    if let SemDataBindingV4::Raw { dataset_id, .. } = &request.model.data_binding {
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
        for variable in &request.model.variables {
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

    let pls = match preflight_general_sem_pls_v1(&request.model, &request.config) {
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
    let selected = selected_general_sem_execution_cell_v1(&request.model, &request.config);
    if !decision_declares_general_sem_execution_cell_v1(&pls, &selected)
        || selected != request.capability_cell
    {
        return blocked(
            "capability_cell_mismatch",
            "The requested option cell differs from the exact native PLS decision for this model and config.",
            "Refresh the unchanged project authority and rerun exact capability preflight.",
        );
    }
    let cbsem = match preflight_general_sem_cbsem_v1(&request.model, &request.config) {
        Ok(decision) => decision,
        Err(error) => {
            return blocked(
                "decision_contract_invalid",
                format!("The CB-SEM capability decision could not satisfy its contract: {error}"),
                "Keep the model unchanged and report this internal capability-decision error.",
            );
        }
    };
    GeneralSemEstimatorPreflightOutcomeV1::Ok {
        value: GeneralSemEstimatorPreflightResultV1 {
            schema_version: GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION,
            pls,
            cbsem,
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
        let project = insert_sem_model_v4_draft_v6(&project, model.clone()).unwrap();
        (project, model)
    }

    fn request() -> GeneralSemEstimatorPreflightRequestV1 {
        let (project, model) = marked_project_and_model();
        GeneralSemEstimatorPreflightRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            capability_cell: qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            project,
            model,
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
    }

    #[test]
    fn labs_gate_unmarked_and_upgraded_projects_fail_closed() {
        let mut denied = request();
        denied.experimental_labs_enabled = false;
        assert!(matches!(
            preflight_general_sem_estimators(denied),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("internal_labs_required")
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
    fn unbound_and_tampered_model_content_fail_closed() {
        let mut unbound = request();
        unbound.model.id = "model:not-in-project".into();
        assert!(matches!(
            preflight_general_sem_estimators(unbound),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("model_not_bound")
        ));

        let mut tampered = request();
        tampered.model.name = "Changed outside project authority".into();
        assert!(matches!(
            preflight_general_sem_estimators(tampered),
            GeneralSemEstimatorPreflightOutcomeV1::Blocked { diagnostic }
                if diagnostic.code.ends_with("model_authority_mismatch")
        ));
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
