//! Authoritative Internal/Labs estimator preflight for schema-6 General SEM.
//!
//! The frontend may provide a fast preview, but only this bridge evaluates the
//! exact Rust compiler contracts. It never mutates or persists project data.

use qpls_core::{
    GeneralSemConfigV1, SemCapabilityDecisionV1, SemModelV4, preflight_general_sem_cbsem_v1,
    preflight_general_sem_pls_v1,
};
use qpls_project::{ProjectArchiveDocumentV6, ProjectModelPayloadV6, ProjectModelRecordV6};
use serde::{Deserialize, Serialize};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const GENERAL_SEM_PREFLIGHT_RESULT_SCHEMA_VERSION: u32 = 1;
const DIAGNOSTIC_CODE_PREFIX: &str = "schema6_general_sem_preflight";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemEstimatorPreflightRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
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
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return blocked(
            "internal_labs_required",
            "General SEM estimator preflight is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and use a newly created General SEM project.",
        );
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
            "Create a new General SEM project in Experimental Labs; upgraded and unmarked projects retain their existing behavior.",
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
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec,
        SemCapabilityDecisionStatusV1, StructuralPath, convert_legacy_basic_model_v4,
    };
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
                        id: "y".into(),
                        name: "Outcome".into(),
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
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn marked_project_and_model() -> (ProjectArchiveDocumentV6, SemModelV4) {
        let model = model();
        let project = ProjectArchiveDocumentV6::new_general_sem_v1(
            Uuid::from_u128(0x6002),
            "General SEM preflight",
            Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap(),
        );
        let project = insert_sem_model_v4_draft_v6(&project, model.clone()).unwrap();
        (project, model)
    }

    fn request() -> GeneralSemEstimatorPreflightRequestV1 {
        let (project, model) = marked_project_and_model();
        GeneralSemEstimatorPreflightRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
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
}
