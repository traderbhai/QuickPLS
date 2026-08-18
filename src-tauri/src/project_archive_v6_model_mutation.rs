//! Internal/Labs-only, in-memory model mutations for schema-6 project documents.
//!
//! This bridge deliberately has no filesystem, active-project, autosave, or
//! recovery capability. Persistence remains a separate, not-yet-cut-over lane.

use qpls_core::SemModelV4;
use qpls_project::{
    ProjectArchiveDocumentV6, ProjectArchiveV6Error, insert_sem_model_v4_draft_v6,
    promote_sem_model_v4_draft_v6, replace_sem_model_v4_draft_v6,
};
use serde::{Deserialize, Serialize};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const MODEL_MUTATION_RESULT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ModelMutationRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    project: ProjectArchiveDocumentV6,
    mutation: ProjectArchiveV6ModelMutationV1,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum ProjectArchiveV6ModelMutationV1 {
    InsertDraft {
        draft: SemModelV4,
    },
    ReplaceDraft {
        model_id: String,
        expected_model_document_sha256: String,
        replacement: SemModelV4,
    },
    PromoteDraft {
        model_id: String,
        expected_model_document_sha256: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectArchiveV6ModelMutationPersistenceV1 {
    NotPersisted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ModelMutationResultV1 {
    schema_version: u32,
    persistence: ProjectArchiveV6ModelMutationPersistenceV1,
    project: ProjectArchiveDocumentV6,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6ModelMutationDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectArchiveV6ModelMutationOutcomeV1 {
    Ok {
        value: Box<ProjectArchiveV6ModelMutationResultV1>,
    },
    Blocked {
        diagnostic: ProjectArchiveV6ModelMutationDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectArchiveV6ModelMutationOutcomeV1 {
    ProjectArchiveV6ModelMutationOutcomeV1::Blocked {
        diagnostic: ProjectArchiveV6ModelMutationDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn mutation_error(error: ProjectArchiveV6Error) -> ProjectArchiveV6ModelMutationOutcomeV1 {
    let (code, corrective_action) = match &error {
        ProjectArchiveV6Error::DuplicateOrEmptyModelId(_) => (
            "schema6_model_mutation.model_id_unavailable",
            "Choose a new, nonempty model revision identifier and retry.",
        ),
        ProjectArchiveV6Error::UnknownModel(_) => (
            "schema6_model_mutation.model_not_found",
            "Refresh the ephemeral schema-6 document and select an existing model.",
        ),
        ProjectArchiveV6Error::ModelMutationRequiresDraft(_) => (
            "schema6_model_mutation.draft_required",
            "Select an unpromoted SemModelV4 draft; executable model authorities are immutable.",
        ),
        ProjectArchiveV6Error::ModelMutationReferenced(_) => (
            "schema6_model_mutation.model_referenced",
            "Create a new model revision identifier instead of changing referenced authority.",
        ),
        ProjectArchiveV6Error::ModelMutationIdentityMismatch { .. } => (
            "schema6_model_mutation.identity_mismatch",
            "Keep the replacement model id equal to the selected draft id.",
        ),
        ProjectArchiveV6Error::ModelDocumentDigestMismatch { .. } => (
            "schema6_model_mutation.stale_model_digest",
            "Refresh the ephemeral document and retry with its current model document digest.",
        ),
        ProjectArchiveV6Error::InvalidSemModel(_) => (
            "schema6_model_mutation.invalid_sem_model",
            "Resolve the reported SemModelV4 authoring or readiness issues and retry.",
        ),
        _ => (
            "schema6_model_mutation.invalid_project",
            "Reload a strict, valid schema-6 project document before retrying the mutation.",
        ),
    };
    blocked(code, error.to_string(), corrective_action)
}

fn non_model_fields_are_unchanged(
    source: &ProjectArchiveDocumentV6,
    mutated: &ProjectArchiveDocumentV6,
) -> bool {
    let mut normalized = mutated.clone();
    normalized.models = source.models.clone();
    match (serde_json::to_vec(&normalized), serde_json::to_vec(source)) {
        (Ok(normalized), Ok(source)) => normalized == source,
        _ => false,
    }
}

fn mutate_project_archive_v6_model(
    request: ProjectArchiveV6ModelMutationRequestV1,
) -> ProjectArchiveV6ModelMutationOutcomeV1 {
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return blocked(
            "schema6_model_mutation.internal_labs_required",
            "Schema-6 model mutation is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and use the internal ephemeral schema-6 model service.",
        );
    }

    let source = request.project;
    let mutation = match request.mutation {
        ProjectArchiveV6ModelMutationV1::InsertDraft { draft } => {
            insert_sem_model_v4_draft_v6(&source, draft)
        }
        ProjectArchiveV6ModelMutationV1::ReplaceDraft {
            model_id,
            expected_model_document_sha256,
            replacement,
        } => replace_sem_model_v4_draft_v6(
            &source,
            &model_id,
            &expected_model_document_sha256,
            replacement,
        ),
        ProjectArchiveV6ModelMutationV1::PromoteDraft {
            model_id,
            expected_model_document_sha256,
        } => promote_sem_model_v4_draft_v6(&source, &model_id, &expected_model_document_sha256),
    };
    let mutated = match mutation {
        Ok(mutated) => mutated,
        Err(error) => return mutation_error(error),
    };

    if !non_model_fields_are_unchanged(&source, &mutated) {
        return blocked(
            "schema6_model_mutation.non_model_change_rejected",
            "The in-memory model operation unexpectedly changed non-model project authority.",
            "Keep the source document unchanged and report this internal bridge failure.",
        );
    }

    ProjectArchiveV6ModelMutationOutcomeV1::Ok {
        value: Box::new(ProjectArchiveV6ModelMutationResultV1 {
            schema_version: MODEL_MUTATION_RESULT_SCHEMA_VERSION,
            persistence: ProjectArchiveV6ModelMutationPersistenceV1::NotPersisted,
            project: mutated,
        }),
    }
}

#[tauri::command]
pub(crate) fn mutate_internal_project_archive_v6_model(
    request: ProjectArchiveV6ModelMutationRequestV1,
) -> ProjectArchiveV6ModelMutationOutcomeV1 {
    mutate_project_archive_v6_model(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use qpls_project::{PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectModelPayloadV6, ProjectOriginV6};
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn project() -> ProjectArchiveDocumentV6 {
        ProjectArchiveDocumentV6 {
            schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: Uuid::from_u128(101),
            name: "Ephemeral model mutation".into(),
            created_at: Utc.with_ymd_and_hms(2026, 8, 15, 10, 0, 0).unwrap(),
            modified_at: Utc.with_ymd_and_hms(2026, 8, 15, 10, 1, 0).unwrap(),
            datasets: Vec::new(),
            models: Vec::new(),
            recipes: Vec::new(),
            historical_recipes: Vec::new(),
            layouts: BTreeMap::from([(
                "model_editor.presentation".into(),
                serde_json::json!({"zoom": 1.25}),
            )]),
            historical_results: Vec::new(),
            canonical_result_documents: Vec::new(),
            origin: ProjectOriginV6::NewProject,
        }
    }

    fn draft() -> SemModelV4 {
        let legacy = ModelSpec {
            id: Uuid::from_u128(202),
            name: "Draft model".into(),
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
        };
        convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    fn request(
        project: ProjectArchiveDocumentV6,
        mutation: ProjectArchiveV6ModelMutationV1,
    ) -> ProjectArchiveV6ModelMutationRequestV1 {
        ProjectArchiveV6ModelMutationRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            project,
            mutation,
        }
    }

    #[test]
    fn insert_is_ephemeral_and_changes_only_models() {
        let source = project();
        let source_before = source.clone();
        let expected_model = draft();
        let outcome = mutate_project_archive_v6_model(request(
            source,
            ProjectArchiveV6ModelMutationV1::InsertDraft {
                draft: expected_model.clone(),
            },
        ));
        let ProjectArchiveV6ModelMutationOutcomeV1::Ok { value } = outcome else {
            panic!("valid draft insert was blocked")
        };

        assert_eq!(
            value.persistence,
            ProjectArchiveV6ModelMutationPersistenceV1::NotPersisted
        );
        assert!(non_model_fields_are_unchanged(
            &source_before,
            &value.project
        ));
        assert_eq!(value.project.models.len(), 1);
        let ProjectModelPayloadV6::SemModelV4Draft { model, .. } = &value.project.models[0].payload
        else {
            panic!("insert must remain an authoring draft")
        };
        assert_eq!(model, &expected_model);
    }

    #[test]
    fn stale_digest_and_non_labs_surface_fail_closed_with_typed_codes() {
        let inserted = mutate_project_archive_v6_model(request(
            project(),
            ProjectArchiveV6ModelMutationV1::InsertDraft { draft: draft() },
        ));
        let ProjectArchiveV6ModelMutationOutcomeV1::Ok { value } = inserted else {
            panic!("fixture draft insert was blocked")
        };
        let model_id = value.project.models[0].model_id.clone();

        let stale = mutate_project_archive_v6_model(request(
            value.project,
            ProjectArchiveV6ModelMutationV1::PromoteDraft {
                model_id,
                expected_model_document_sha256: "f".repeat(64),
            },
        ));
        assert!(matches!(
            stale,
            ProjectArchiveV6ModelMutationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_model_mutation.stale_model_digest"
        ));

        let mut denied = request(
            project(),
            ProjectArchiveV6ModelMutationV1::InsertDraft { draft: draft() },
        );
        denied.surface = "standard".into();
        assert!(matches!(
            mutate_project_archive_v6_model(denied),
            ProjectArchiveV6ModelMutationOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_model_mutation.internal_labs_required"
        ));
    }
}
