use crate::recipe_v4_canonical_result::validate_archived_recipe_v4_pls_method_identity;
use crate::recipe_v4_cbsem_canonical_result::validate_archived_recipe_v4_cbsem_method_identity;
use crate::recipe_v4_general_sem_canonical_result::validate_archived_general_sem_pls_method_identity_v1;
use qpls_core::AnalysisRecipeV4;
use qpls_project::{
    CanonicalResultDocumentV2, ProjectArchiveCanonicalAppendReceiptV6, ProjectArchiveV6Error,
    append_canonical_result_document_v2_file_v6,
    append_recipe_v4_and_canonical_result_document_v2_file_v6,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const STANDARD_EXACT_CBSEM_SURFACE: &str = "standard_exact_cbsem";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectSchema6ResultAppendRequestV1 {
    pub(crate) surface: String,
    pub(crate) experimental_labs_enabled: bool,
    pub(crate) archive_path: String,
    pub(crate) expected_source_sha256: String,
    #[serde(default)]
    pub(crate) recipe: Option<AnalysisRecipeV4>,
    pub(crate) canonical_document: CanonicalResultDocumentV2,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectSchema6ResultAppendDiagnosticV1 {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) corrective_action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectSchema6ResultAppendOutcomeV1 {
    Ok {
        value: ProjectArchiveCanonicalAppendReceiptV6,
    },
    Blocked {
        diagnostic: ProjectSchema6ResultAppendDiagnosticV1,
    },
}

fn blocked(
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> ProjectSchema6ResultAppendOutcomeV1 {
    ProjectSchema6ResultAppendOutcomeV1::Blocked {
        diagnostic: ProjectSchema6ResultAppendDiagnosticV1 {
            code: code.into(),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn validate_request_access(
    request: &ProjectSchema6ResultAppendRequestV1,
) -> Result<PathBuf, ProjectSchema6ResultAppendOutcomeV1> {
    let historical_internal =
        request.surface == INTERNAL_LABS_SURFACE && request.experimental_labs_enabled;
    let current_exact_cbsem = request.surface == STANDARD_EXACT_CBSEM_SURFACE
        && !request.experimental_labs_enabled
        && matches!(
            (
                request
                    .canonical_document
                    .provenance
                    .capability_cell
                    .capability_id
                    .as_str(),
                request
                    .canonical_document
                    .provenance
                    .capability_cell
                    .cell_id
                    .as_str(),
            ),
            ("smartpls.cbsem", "qpls3.cbsem.ml")
                | ("smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap")
        );
    if !historical_internal && !current_exact_cbsem {
        return Err(blocked(
            "schema6_result_append.surface_mismatch",
            "Schema-6 result attachment requires the historical internal-Labs boundary or the current exact-CB-SEM Standard boundary with its matching capability cell.",
            "Use standard_exact_cbsem with Experimental Labs disabled only for a current CB-SEM point or exact-bootstrap canonical document.",
        ));
    }
    let archive_path = Path::new(request.archive_path.trim());
    if request.archive_path.trim().is_empty() || !archive_path.is_absolute() {
        return Err(blocked(
            "schema6_result_append.absolute_path_required",
            "The schema-6 archive path must be an absolute local path.",
            "Select the exact schema-6 project copy before attaching the result.",
        ));
    }
    Ok(archive_path.to_path_buf())
}

fn map_append_error(error: ProjectArchiveV6Error) -> ProjectSchema6ResultAppendOutcomeV1 {
    let code = match error {
        ProjectArchiveV6Error::SourceDigestMismatch { .. }
        | ProjectArchiveV6Error::SourceChangedDuringAppend => {
            "schema6_result_append.source_changed"
        }
        ProjectArchiveV6Error::AppendAlreadyInProgress(_) => {
            "schema6_result_append.concurrent_writer"
        }
        ProjectArchiveV6Error::AppendCancelled => "schema6_result_append.cancelled",
        ProjectArchiveV6Error::DuplicateCanonicalResultDocumentId(_)
        | ProjectArchiveV6Error::DuplicateCanonicalResultRunId(_) => {
            "schema6_result_append.duplicate_result"
        }
        ProjectArchiveV6Error::AppendArchiveMustBeRegularFile(_) => {
            "schema6_result_append.regular_file_required"
        }
        ProjectArchiveV6Error::PostWriteValidation
        | ProjectArchiveV6Error::AppendRollbackFailed { .. } => {
            "schema6_result_append.post_write_integrity"
        }
        _ => "schema6_result_append.invalid_request",
    };
    blocked(
        code,
        error.to_string(),
        "Reopen the schema-6 project, verify its current digest and result identity, then retry without changing the source file.",
    )
}

#[tauri::command]
pub(crate) fn append_internal_project_schema6_canonical_result_v2(
    request: ProjectSchema6ResultAppendRequestV1,
) -> ProjectSchema6ResultAppendOutcomeV1 {
    let archive_path = match validate_request_access(&request) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };
    if let Err(error) =
        validate_archived_recipe_v4_cbsem_method_identity(&request.canonical_document)
    {
        return blocked(
            "schema6_result_append.cbsem_method_identity_mismatch",
            error,
            "Rebuild the CB-SEM canonical result from the exact matching Recipe-v4 execution before attaching it.",
        );
    }
    if let Err(error) =
        validate_archived_general_sem_pls_method_identity_v1(&request.canonical_document)
    {
        return blocked(
            "schema6_result_append.general_sem_method_identity_mismatch",
            error,
            "Rebuild the General SEM canonical result from the exact resident compiled plan and native execution before attaching it.",
        );
    }
    if let Err(error) = validate_archived_recipe_v4_pls_method_identity(&request.canonical_document)
    {
        return blocked(
            "schema6_result_append.pls_method_identity_mismatch",
            error,
            "Rebuild the PLS canonical result from the exact matching Recipe-v4 execution before attaching it.",
        );
    }
    let appended = match request.recipe {
        Some(recipe) => append_recipe_v4_and_canonical_result_document_v2_file_v6(
            &archive_path,
            &request.expected_source_sha256,
            recipe,
            request.canonical_document,
        ),
        None => append_canonical_result_document_v2_file_v6(
            &archive_path,
            &request.expected_source_sha256,
            request.canonical_document,
        ),
    };
    match appended {
        Ok(value) => ProjectSchema6ResultAppendOutcomeV1::Ok { value },
        Err(error) => map_append_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str) -> ProjectSchema6ResultAppendRequestV1 {
        ProjectSchema6ResultAppendRequestV1 {
            surface: INTERNAL_LABS_SURFACE.into(),
            experimental_labs_enabled: true,
            archive_path: path.into(),
            expected_source_sha256: "a".repeat(64),
            recipe: None,
            canonical_document: serde_json::from_value(serde_json::json!({
                "schema_version": 2,
                "document_id": "result.document:command-test",
                "title": "Command test",
                "provenance": {
                    "run_id": "run-command-test",
                    "project_id": "00000000-0000-0000-0000-000000000001",
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
                    "engine_version": "qpls-estimation-test",
                    "seed": 42,
                    "workers": 1,
                    "started_at": "2026-08-14T00:00:00Z",
                    "completed_at": "2026-08-14T00:00:01Z"
                },
                "sections": [], "tables": [], "charts": [], "notices": [],
                "exclusions": [], "footnotes": [],
                "presentation": {
                    "default_section_id": null,
                    "default_table_id": null,
                    "precision": 4,
                    "missing_value_label": "N/A",
                    "chart_defaults": {}
                }
            }))
            .unwrap(),
        }
    }

    #[test]
    fn command_access_accepts_only_historical_internal_or_exact_cbsem_standard_and_requires_an_absolute_path()
     {
        let mut standard = request(r"D:\study-v6.json");
        standard.surface = "standard".into();
        assert!(matches!(
            validate_request_access(&standard),
            Err(ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic })
                if diagnostic.code == "schema6_result_append.surface_mismatch"
        ));

        let mut exact = request(r"D:\study-v6.json");
        exact.surface = STANDARD_EXACT_CBSEM_SURFACE.into();
        exact.experimental_labs_enabled = false;
        exact
            .canonical_document
            .provenance
            .capability_cell
            .capability_id = "smartpls.cbsem_bootstrapping".into();
        exact.canonical_document.provenance.capability_cell.cell_id =
            "qpls3.cbsem.bootstrap".into();
        exact
            .canonical_document
            .provenance
            .capability_cell
            .capability_version = "cbsem_exact_case_bootstrap_v1".into();
        validate_request_access(&exact).unwrap();

        let relative = request("study-v6.json");
        assert!(matches!(
            validate_request_access(&relative),
            Err(ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic })
                if diagnostic.code == "schema6_result_append.absolute_path_required"
        ));
    }

    #[test]
    fn stale_cbsem_v2_method_identity_is_rejected_before_archive_io() {
        let mut stale = request(r"D:\cbsem-stale-v2.json");
        stale
            .canonical_document
            .provenance
            .capability_cell
            .capability_id = "smartpls.cbsem".into();
        stale.canonical_document.provenance.capability_cell.cell_id = "qpls3.cbsem.ml".into();
        stale
            .canonical_document
            .provenance
            .capability_cell
            .capability_version = "cbsem_ml_v1".into();
        stale.canonical_document.provenance.method_version =
            "cbsem_ml_compiled_moment_input_v2".into();

        assert!(matches!(
            append_internal_project_schema6_canonical_result_v2(stale),
            ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_result_append.cbsem_method_identity_mismatch"
        ));
    }
}
