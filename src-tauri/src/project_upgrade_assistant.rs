use chrono::Utc;
use qpls_core::{LegacyBasicModelInterpretationV4, LegacyDisplayCovarianceV4, sha256_hex};
#[cfg(test)]
use qpls_project::Project;
use qpls_project::{
    FutureProjectArchiveReadOnlyV6, ProjectArchiveInspectionV6, ProjectArchiveUpgradePlanV6,
    ProjectArchiveUpgradeReceiptV6, ProjectArchiveUpgradeRequestV6,
    ProjectArchiveUpgradeZipV6Error, ProjectArchiveV6Error, ProjectModelPayloadV6,
    confirm_project_legacy_estimand_v6, execute_project_upgrade_zip_copy_v6_with_control,
    inspect_project_document_bytes_v6, load_project, load_project_archive_v6,
    plan_project_upgrade_to_v6, serialize_project_document_v6,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const MAX_PENDING_UPGRADE_PLANS: usize = 16;

#[derive(Clone, Default)]
pub(crate) struct DesktopProjectUpgradePlans(
    Arc<Mutex<HashMap<Uuid, PendingProjectUpgradePlanV1>>>,
);

#[derive(Debug)]
struct PendingProjectUpgradePlanV1 {
    plan: ProjectArchiveUpgradePlanV6,
    plan_sha256: String,
    cancelled: Arc<AtomicBool>,
    executing: bool,
    committed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectUpgradeDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectUpgradeOutcomeV1<T> {
    Ok {
        value: T,
    },
    Blocked {
        diagnostic: ProjectUpgradeDiagnosticV1,
    },
}

impl<T> ProjectUpgradeOutcomeV1<T> {
    fn ok(value: T) -> Self {
        Self::Ok { value }
    }

    fn blocked(diagnostic: ProjectUpgradeDiagnosticV1) -> Self {
        Self::Blocked { diagnostic }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectUpgradeInspectRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    source_archive_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectUpgradeSourceKindV1 {
    ProjectArchive,
    StandaloneDocument,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectUpgradeAccessV1 {
    HistoricalUpgradeCopyRequired,
    CurrentV6Standalone,
    CurrentV6Archive,
    FutureReadOnly,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectUpgradeItemCountsV1 {
    datasets: usize,
    models: usize,
    recipes: usize,
    results: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectUpgradeFutureUnsupportedV1 {
    models: usize,
    recipes: usize,
    results: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectUpgradeInspectionV1 {
    source_archive_path: String,
    source_archive_sha256: String,
    source_kind: ProjectUpgradeSourceKindV1,
    schema_version: u32,
    access: ProjectUpgradeAccessV1,
    read_only: bool,
    upgrade_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    counts: ProjectUpgradeItemCountsV1,
    future_unsupported: ProjectUpgradeFutureUnsupportedV1,
    source_will_remain_unchanged: bool,
    destination_must_be_new: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyDisplayCovarianceRequestV1 {
    id: String,
    left_construct: String,
    right_construct: String,
    #[serde(default)]
    label: Option<String>,
}

impl From<LegacyDisplayCovarianceRequestV1> for LegacyDisplayCovarianceV4 {
    fn from(value: LegacyDisplayCovarianceRequestV1) -> Self {
        Self {
            id: value.id,
            left_construct: value.left_construct,
            right_construct: value.right_construct,
            label: value.label,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectUpgradeEstimandChoiceV1 {
    Composite,
    CommonFactor,
}

impl From<ProjectUpgradeEstimandChoiceV1> for LegacyBasicModelInterpretationV4 {
    fn from(value: ProjectUpgradeEstimandChoiceV1) -> Self {
        match value {
            ProjectUpgradeEstimandChoiceV1::Composite => Self::PlsComposite,
            ProjectUpgradeEstimandChoiceV1::CommonFactor => Self::CbsemCommonFactor,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectUpgradePlanRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    source_archive_path: String,
    destination_archive_path: String,
    expected_source_archive_sha256: String,
    #[serde(default)]
    legacy_display_covariances: BTreeMap<String, Vec<LegacyDisplayCovarianceRequestV1>>,
    #[serde(default)]
    estimand_confirmations: BTreeMap<String, ProjectUpgradeEstimandChoiceV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectUpgradeEstimandPromptV1 {
    model_id: String,
    model_name: String,
    choices: [&'static str; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    conversion_blocker: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProjectUpgradePlanStateV1 {
    ConfirmationRequired {
        source_archive_sha256: String,
        destination_archive_path: String,
        prompts: Vec<ProjectUpgradeEstimandPromptV1>,
        source_will_remain_unchanged: bool,
        destination_must_be_new: bool,
        historical_results_immutable: bool,
    },
    Ready {
        plan_id: Uuid,
        plan_sha256: String,
        source_archive_sha256: String,
        destination_archive_path: String,
        model_count: usize,
        recipe_count: usize,
        historical_result_count: usize,
        source_will_remain_unchanged: bool,
        destination_must_be_new: bool,
        historical_results_immutable: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectUpgradeExecuteRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    plan_id: Uuid,
    expected_plan_sha256: String,
    confirm_new_destination: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectUpgradeExecutionV1 {
    plan_id: Uuid,
    receipt: ProjectUpgradeReceiptViewV1,
    destination_written: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectUpgradeReceiptViewV1 {
    write: ProjectUpgradeWriteReceiptViewV1,
    source_archive_path: String,
    source_archive_sha256: String,
    source_verified_unchanged: bool,
    historical_results_immutable: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectUpgradeWriteReceiptViewV1 {
    schema_version: u32,
    project_id: Uuid,
    destination_archive_path: String,
    document_sha256: String,
    byte_length: u64,
    post_write_validated: bool,
}

impl From<ProjectArchiveUpgradeReceiptV6> for ProjectUpgradeReceiptViewV1 {
    fn from(value: ProjectArchiveUpgradeReceiptV6) -> Self {
        Self {
            write: ProjectUpgradeWriteReceiptViewV1 {
                schema_version: value.write.schema_version,
                project_id: value.write.project_id,
                destination_archive_path: value.write.destination_archive_path,
                document_sha256: value.write.document_sha256,
                byte_length: value.write.byte_length,
                post_write_validated: value.write.post_write_validated,
            },
            source_archive_path: value.source_archive_path,
            source_archive_sha256: value.source_archive_sha256,
            source_verified_unchanged: value.source_verified_unchanged,
            historical_results_immutable: value.historical_results_immutable,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectUpgradeCancelRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    plan_id: Uuid,
    expected_plan_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectUpgradeCancellationV1 {
    plan_id: Uuid,
    destination_archive_path: String,
    cancelled: bool,
    destination_written: bool,
}

#[tauri::command]
pub(crate) fn inspect_internal_project_upgrade_v6(
    request: ProjectUpgradeInspectRequestV1,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradeInspectionV1> {
    inspect_project_upgrade_v6_impl(request)
}

#[tauri::command]
pub(crate) fn plan_internal_project_upgrade_v6(
    request: ProjectUpgradePlanRequestV1,
    state: State<'_, DesktopProjectUpgradePlans>,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1> {
    plan_project_upgrade_v6_impl(request, &state)
}

#[tauri::command]
pub(crate) async fn execute_internal_project_upgrade_v6(
    request: ProjectUpgradeExecuteRequestV1,
    state: State<'_, DesktopProjectUpgradePlans>,
) -> Result<ProjectUpgradeOutcomeV1<ProjectUpgradeExecutionV1>, String> {
    let cleanup_state = state.inner().clone();
    let worker_state = cleanup_state.clone();
    let plan_id = request.plan_id;
    Ok(
        match tauri::async_runtime::spawn_blocking(move || {
            execute_project_upgrade_v6_impl(request, &worker_state)
        })
        .await
        {
            Ok(outcome) => outcome,
            Err(_) => {
                let mut plans = cleanup_state
                    .0
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                plans.remove(&plan_id);
                drop(plans);
                cleanup_state.0.clear_poison();
                ProjectUpgradeOutcomeV1::blocked(state_unavailable_diagnostic())
            }
        },
    )
}

#[tauri::command]
pub(crate) fn cancel_internal_project_upgrade_v6(
    request: ProjectUpgradeCancelRequestV1,
    state: State<'_, DesktopProjectUpgradePlans>,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradeCancellationV1> {
    cancel_project_upgrade_v6_impl(request, &state)
}

fn inspect_project_upgrade_v6_impl(
    request: ProjectUpgradeInspectRequestV1,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradeInspectionV1> {
    if let Err(diagnostic) =
        require_internal_labs(&request.surface, request.experimental_labs_enabled)
    {
        return ProjectUpgradeOutcomeV1::blocked(diagnostic);
    }
    let source = match require_absolute_existing_file(&request.source_archive_path, "source") {
        Ok(path) => path,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    let source_sha256 = match sha256_source_file(&source) {
        Ok(sha256) => sha256,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };

    if let Ok(project) = load_project(&source) {
        let confirmed_sha256 = match sha256_source_file(&source) {
            Ok(sha256) => sha256,
            Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
        };
        if confirmed_sha256 != source_sha256 {
            return ProjectUpgradeOutcomeV1::blocked(source_changed_during_inspection(
                &request.source_archive_path,
            ));
        }
        let future = project.read_only || project.source_archive_version > 5;
        return ProjectUpgradeOutcomeV1::ok(ProjectUpgradeInspectionV1 {
            source_archive_path: request.source_archive_path,
            source_archive_sha256: source_sha256,
            source_kind: ProjectUpgradeSourceKindV1::ProjectArchive,
            schema_version: project.source_archive_version,
            access: if future {
                ProjectUpgradeAccessV1::FutureReadOnly
            } else {
                ProjectUpgradeAccessV1::HistoricalUpgradeCopyRequired
            },
            read_only: future,
            upgrade_available: !future,
            project_id: Some(project.manifest.project_id),
            project_name: Some(project.manifest.name),
            counts: ProjectUpgradeItemCountsV1 {
                datasets: project.datasets.len(),
                models: project.models.len(),
                recipes: project.recipes.len(),
                results: project.results.len(),
            },
            future_unsupported: ProjectUpgradeFutureUnsupportedV1 {
                models: project.future_unsupported.models,
                recipes: project.future_unsupported.recipes,
                results: project.future_unsupported.results,
            },
            source_will_remain_unchanged: true,
            destination_must_be_new: true,
        });
    }

    if let Ok(archive) = load_project_archive_v6(&source) {
        let confirmed_sha256 = match sha256_source_file(&source) {
            Ok(sha256) => sha256,
            Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
        };
        if confirmed_sha256 != source_sha256 {
            return ProjectUpgradeOutcomeV1::blocked(source_changed_during_inspection(
                &request.source_archive_path,
            ));
        }
        return ProjectUpgradeOutcomeV1::ok(ProjectUpgradeInspectionV1 {
            source_archive_path: request.source_archive_path,
            source_archive_sha256: source_sha256,
            source_kind: ProjectUpgradeSourceKindV1::ProjectArchive,
            schema_version: archive.document.schema_version,
            access: ProjectUpgradeAccessV1::CurrentV6Archive,
            read_only: true,
            upgrade_available: false,
            project_id: Some(archive.document.project_id),
            project_name: Some(archive.document.name),
            counts: ProjectUpgradeItemCountsV1 {
                datasets: archive.document.datasets.len(),
                models: archive.document.models.len(),
                recipes: archive.document.recipes.len() + archive.document.historical_recipes.len(),
                results: archive.document.historical_results.len(),
            },
            future_unsupported: zero_future_unsupported(),
            source_will_remain_unchanged: true,
            destination_must_be_new: true,
        });
    }

    let bytes = match read_source_bytes(&source) {
        Ok(bytes) => bytes,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    let inspected_sha256 = sha256_hex(&bytes);
    if inspected_sha256 != source_sha256 {
        return ProjectUpgradeOutcomeV1::blocked(source_changed_during_inspection(
            &request.source_archive_path,
        ));
    }
    let inspection = inspect_project_document_bytes_v6(&bytes);
    let confirmed_sha256 = match sha256_source_file(&source) {
        Ok(sha256) => sha256,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    if confirmed_sha256 != inspected_sha256 {
        return ProjectUpgradeOutcomeV1::blocked(source_changed_during_inspection(
            &request.source_archive_path,
        ));
    }
    match inspection {
        Ok(ProjectArchiveInspectionV6::HistoricalUpgradeRequired { schema_version }) => {
            ProjectUpgradeOutcomeV1::ok(ProjectUpgradeInspectionV1 {
                source_archive_path: request.source_archive_path,
                source_archive_sha256: source_sha256,
                source_kind: ProjectUpgradeSourceKindV1::StandaloneDocument,
                schema_version,
                access: ProjectUpgradeAccessV1::HistoricalUpgradeCopyRequired,
                read_only: true,
                upgrade_available: false,
                project_id: None,
                project_name: None,
                counts: zero_counts(),
                future_unsupported: zero_future_unsupported(),
                source_will_remain_unchanged: true,
                destination_must_be_new: true,
            })
        }
        Ok(ProjectArchiveInspectionV6::Current(document)) => {
            ProjectUpgradeOutcomeV1::ok(ProjectUpgradeInspectionV1 {
                source_archive_path: request.source_archive_path,
                source_archive_sha256: source_sha256,
                source_kind: ProjectUpgradeSourceKindV1::StandaloneDocument,
                schema_version: document.schema_version,
                access: ProjectUpgradeAccessV1::CurrentV6Standalone,
                read_only: false,
                upgrade_available: false,
                project_id: Some(document.project_id),
                project_name: Some(document.name),
                counts: ProjectUpgradeItemCountsV1 {
                    datasets: document.datasets.len(),
                    models: document.models.len(),
                    recipes: document.recipes.len() + document.historical_recipes.len(),
                    results: document.historical_results.len(),
                },
                future_unsupported: zero_future_unsupported(),
                source_will_remain_unchanged: true,
                destination_must_be_new: true,
            })
        }
        Ok(ProjectArchiveInspectionV6::FutureReadOnly(summary)) => ProjectUpgradeOutcomeV1::ok(
            future_standalone_inspection(request.source_archive_path, inspected_sha256, summary),
        ),
        Err(error) => ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "archive_unreadable".into(),
            message: format!("QuickPLS could not inspect this project: {error}"),
            corrective_action: "Choose an intact QuickPLS project file and inspect it again."
                .into(),
            object_id: Some(request.source_archive_path),
        }),
    }
}

fn plan_project_upgrade_v6_impl(
    request: ProjectUpgradePlanRequestV1,
    state: &DesktopProjectUpgradePlans,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1> {
    if let Err(diagnostic) =
        require_internal_labs(&request.surface, request.experimental_labs_enabled)
    {
        return ProjectUpgradeOutcomeV1::blocked(diagnostic);
    }
    let source = match require_absolute_existing_file(&request.source_archive_path, "source") {
        Ok(path) => path,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    let destination = match require_absolute_new_destination(&request.destination_archive_path) {
        Ok(path) => path,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    if source == destination {
        return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "destination_matches_source".into(),
            message: "The upgraded copy cannot use the source project path.".into(),
            corrective_action: "Choose a different, unused destination path.".into(),
            object_id: Some(request.destination_archive_path),
        });
    }

    let observed_sha256 = match sha256_source_file(&source) {
        Ok(sha256) => sha256,
        Err(diagnostic) => return ProjectUpgradeOutcomeV1::blocked(diagnostic),
    };
    if observed_sha256 != request.expected_source_archive_sha256 {
        return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "source_changed".into(),
            message: "The source project changed after it was inspected.".into(),
            corrective_action: "Inspect the source project again before creating an upgrade plan."
                .into(),
            object_id: Some(request.source_archive_path),
        });
    }
    let source_project = match load_project(&source) {
        Ok(project) => project,
        Err(error) => {
            return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
                code: "source_not_project_archive".into(),
                message: format!(
                    "The source is not an upgradeable QuickPLS project archive: {error}"
                ),
                corrective_action: "Choose a schema 1 through 5 QuickPLS project archive.".into(),
                object_id: Some(request.source_archive_path),
            });
        }
    };
    if source_project.read_only || source_project.source_archive_version > 5 {
        return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "future_project_read_only".into(),
            message: format!(
                "Project schema {} is newer than this upgrade assistant and remains read-only.",
                source_project.source_archive_version
            ),
            corrective_action: "Open it with a QuickPLS version that supports that project schema."
                .into(),
            object_id: Some(request.source_archive_path),
        });
    }

    let display_covariances = request
        .legacy_display_covariances
        .into_iter()
        .map(|(model_id, drawings)| {
            (
                model_id,
                drawings.into_iter().map(Into::into).collect::<Vec<_>>(),
            )
        })
        .collect();
    let mut plan = match plan_project_upgrade_to_v6(
        &source_project,
        &ProjectArchiveUpgradeRequestV6 {
            source_archive_sha256: observed_sha256.clone(),
            source_archive_path: request.source_archive_path.clone(),
            destination_archive_path: request.destination_archive_path.clone(),
            upgraded_at: Utc::now(),
            legacy_display_covariances: display_covariances,
        },
    ) {
        Ok(plan) => plan,
        Err(error) => return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(error)),
    };

    let pending_ids = plan
        .document
        .models
        .iter()
        .filter(|record| model_requires_estimand_confirmation(&record.payload))
        .map(|record| record.model_id.clone())
        .collect::<Vec<_>>();
    for confirmed_id in request.estimand_confirmations.keys() {
        if !pending_ids.contains(confirmed_id) {
            return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
                code: "estimand_confirmation_not_applicable".into(),
                message: format!(
                    "Model {confirmed_id} does not require factor-versus-composite confirmation."
                ),
                corrective_action: "Remove that confirmation and create the plan again.".into(),
                object_id: Some(confirmed_id.clone()),
            });
        }
    }
    let missing = pending_ids
        .iter()
        .filter(|model_id| !request.estimand_confirmations.contains_key(*model_id))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        let prompts = missing
            .into_iter()
            .filter_map(|model_id| {
                plan.document
                    .models
                    .iter()
                    .find(|record| &record.model_id == model_id)
            })
            .filter_map(|record| match &record.payload {
                ProjectModelPayloadV6::LegacyEstimandUnspecified {
                    legacy_model,
                    automatic_conversion_blocker,
                    ..
                } => Some(ProjectUpgradeEstimandPromptV1 {
                    model_id: record.model_id.clone(),
                    model_name: legacy_model.name.clone(),
                    choices: ["composite", "common_factor"],
                    conversion_blocker: automatic_conversion_blocker.clone(),
                }),
                ProjectModelPayloadV6::SemModelV4 { .. }
                | ProjectModelPayloadV6::SemModelV4Draft { .. } => None,
            })
            .collect();
        return ProjectUpgradeOutcomeV1::ok(ProjectUpgradePlanStateV1::ConfirmationRequired {
            source_archive_sha256: observed_sha256,
            destination_archive_path: request.destination_archive_path,
            prompts,
            source_will_remain_unchanged: true,
            destination_must_be_new: true,
            historical_results_immutable: true,
        });
    }

    for model_id in pending_ids {
        let choice = request.estimand_confirmations[&model_id];
        plan.document =
            match confirm_project_legacy_estimand_v6(&plan.document, &model_id, choice.into()) {
                Ok(document) => document,
                Err(error) => {
                    return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(error));
                }
            };
    }
    if let Err(error) = plan.ensure_valid() {
        return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(error));
    }
    let document_bytes = match serialize_project_document_v6(&plan.document) {
        Ok(bytes) => bytes,
        Err(error) => return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(error)),
    };
    let plan_sha256 = sha256_hex(&document_bytes);
    let plan_id = Uuid::new_v4();
    let response = ProjectUpgradePlanStateV1::Ready {
        plan_id,
        plan_sha256: plan_sha256.clone(),
        source_archive_sha256: observed_sha256,
        destination_archive_path: request.destination_archive_path,
        model_count: plan.document.models.len(),
        recipe_count: plan.document.recipes.len() + plan.document.historical_recipes.len(),
        historical_result_count: plan.document.historical_results.len(),
        source_will_remain_unchanged: true,
        destination_must_be_new: true,
        historical_results_immutable: true,
    };
    let mut plans = match state.0.lock() {
        Ok(plans) => plans,
        Err(_) => return ProjectUpgradeOutcomeV1::blocked(state_unavailable_diagnostic()),
    };
    if plans.len() >= MAX_PENDING_UPGRADE_PLANS {
        return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "too_many_pending_plans".into(),
            message: "Too many project upgrade plans are waiting.".into(),
            corrective_action: "Cancel an earlier plan, then try again.".into(),
            object_id: None,
        });
    }
    plans.insert(
        plan_id,
        PendingProjectUpgradePlanV1 {
            plan,
            plan_sha256,
            cancelled: Arc::new(AtomicBool::new(false)),
            executing: false,
            committed: false,
        },
    );
    ProjectUpgradeOutcomeV1::ok(response)
}

fn model_requires_estimand_confirmation(payload: &ProjectModelPayloadV6) -> bool {
    matches!(
        payload,
        ProjectModelPayloadV6::LegacyEstimandUnspecified { .. }
    )
}

fn execute_project_upgrade_v6_impl(
    request: ProjectUpgradeExecuteRequestV1,
    state: &DesktopProjectUpgradePlans,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradeExecutionV1> {
    if let Err(diagnostic) =
        require_internal_labs(&request.surface, request.experimental_labs_enabled)
    {
        return ProjectUpgradeOutcomeV1::blocked(diagnostic);
    }
    if !request.confirm_new_destination {
        return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
            code: "new_destination_confirmation_required".into(),
            message: "Creating the upgraded copy requires explicit confirmation.".into(),
            corrective_action: "Confirm that QuickPLS should create the new destination file."
                .into(),
            object_id: Some(request.plan_id.to_string()),
        });
    }
    let (plan, cancelled) = {
        let mut plans = match state.0.lock() {
            Ok(plans) => plans,
            Err(_) => return ProjectUpgradeOutcomeV1::blocked(state_unavailable_diagnostic()),
        };
        let Some(existing) = plans.get_mut(&request.plan_id) else {
            return ProjectUpgradeOutcomeV1::blocked(unknown_plan_diagnostic(request.plan_id));
        };
        if existing.plan_sha256 != request.expected_plan_sha256 {
            return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
                code: "plan_changed".into(),
                message: "The upgrade plan identity does not match the prepared plan.".into(),
                corrective_action: "Create a new plan and review it before writing the copy."
                    .into(),
                object_id: Some(request.plan_id.to_string()),
            });
        }
        if existing.executing {
            return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
                code: "upgrade_already_executing".into(),
                message: "This upgrade plan is already writing its new copy.".into(),
                corrective_action: "Wait for execution to finish or cancel the active plan.".into(),
                object_id: Some(request.plan_id.to_string()),
            });
        }
        existing.executing = true;
        (existing.plan.clone(), Arc::clone(&existing.cancelled))
    };
    let Some(lineage) = plan.document.upgrade_lineage() else {
        if let Ok(mut plans) = state.0.lock() {
            plans.remove(&request.plan_id);
        }
        return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(
            ProjectArchiveV6Error::UpgradeOriginRequired,
        ));
    };
    let source = PathBuf::from(&lineage.source_archive_path);
    let destination = PathBuf::from(&lineage.destination_archive_path);
    let result = execute_project_upgrade_zip_copy_v6_with_control(
        &source,
        &destination,
        &plan,
        || cancelled.load(Ordering::Acquire),
        || {
            let Ok(mut plans) = state.0.lock() else {
                return false;
            };
            let Some(pending) = plans.get_mut(&request.plan_id) else {
                return false;
            };
            if pending.cancelled.load(Ordering::Acquire) {
                false
            } else {
                pending.committed = true;
                true
            }
        },
    );
    if let Ok(mut plans) = state.0.lock() {
        plans.remove(&request.plan_id);
    }
    match result {
        Ok(receipt) => ProjectUpgradeOutcomeV1::ok(ProjectUpgradeExecutionV1 {
            plan_id: request.plan_id,
            receipt: receipt.into(),
            destination_written: true,
        }),
        Err(error) => ProjectUpgradeOutcomeV1::blocked(zip_upgrade_error_diagnostic(error)),
    }
}

fn cancel_project_upgrade_v6_impl(
    request: ProjectUpgradeCancelRequestV1,
    state: &DesktopProjectUpgradePlans,
) -> ProjectUpgradeOutcomeV1<ProjectUpgradeCancellationV1> {
    if let Err(diagnostic) =
        require_internal_labs(&request.surface, request.experimental_labs_enabled)
    {
        return ProjectUpgradeOutcomeV1::blocked(diagnostic);
    }
    let mut plans = match state.0.lock() {
        Ok(plans) => plans,
        Err(_) => return ProjectUpgradeOutcomeV1::blocked(state_unavailable_diagnostic()),
    };
    let (destination, executing, committed) = {
        let Some(existing) = plans.get(&request.plan_id) else {
            return ProjectUpgradeOutcomeV1::blocked(unknown_plan_diagnostic(request.plan_id));
        };
        if existing.plan_sha256 != request.expected_plan_sha256 {
            return ProjectUpgradeOutcomeV1::blocked(ProjectUpgradeDiagnosticV1 {
                code: "plan_changed".into(),
                message: "The upgrade plan identity does not match the prepared plan.".into(),
                corrective_action: "Use the exact plan identity shown by the latest plan.".into(),
                object_id: Some(request.plan_id.to_string()),
            });
        }
        let Some(lineage) = existing.plan.document.upgrade_lineage() else {
            return ProjectUpgradeOutcomeV1::blocked(project_error_diagnostic(
                ProjectArchiveV6Error::UpgradeOriginRequired,
            ));
        };
        if !existing.committed {
            existing.cancelled.store(true, Ordering::Release);
        }
        (
            lineage.destination_archive_path.clone(),
            existing.executing,
            existing.committed,
        )
    };
    if !executing || committed {
        plans.remove(&request.plan_id);
    }
    ProjectUpgradeOutcomeV1::ok(ProjectUpgradeCancellationV1 {
        plan_id: request.plan_id,
        destination_archive_path: destination,
        cancelled: !committed,
        destination_written: committed,
    })
}

fn require_internal_labs(surface: &str, enabled: bool) -> Result<(), ProjectUpgradeDiagnosticV1> {
    if surface != INTERNAL_LABS_SURFACE || !enabled {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: "internal_labs_required".into(),
            message: "The project upgrade assistant is available only in Experimental Labs.".into(),
            corrective_action: "Enable Experimental Labs, then reopen the assistant.".into(),
            object_id: None,
        });
    }
    Ok(())
}

fn require_absolute_existing_file(
    value: &str,
    object: &str,
) -> Result<PathBuf, ProjectUpgradeDiagnosticV1> {
    let path = PathBuf::from(value);
    if value.trim().is_empty() || !path.is_absolute() {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: format!("{object}_path_invalid"),
            message: format!("The {object} path must be an absolute file path."),
            corrective_action: format!("Choose the {object} file with the file picker."),
            object_id: Some(value.into()),
        });
    }
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(path),
        Ok(_) => Err(ProjectUpgradeDiagnosticV1 {
            code: format!("{object}_not_file"),
            message: format!("The selected {object} path is not a regular file."),
            corrective_action: format!("Choose a QuickPLS {object} file."),
            object_id: Some(value.into()),
        }),
        Err(error) => Err(ProjectUpgradeDiagnosticV1 {
            code: format!("{object}_unavailable"),
            message: format!("The selected {object} file is unavailable: {error}"),
            corrective_action: format!("Choose an existing, readable {object} file."),
            object_id: Some(value.into()),
        }),
    }
}

fn require_absolute_new_destination(value: &str) -> Result<PathBuf, ProjectUpgradeDiagnosticV1> {
    let path = PathBuf::from(value);
    if value.trim().is_empty() || !path.is_absolute() {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: "destination_path_invalid".into(),
            message: "The destination path must be an absolute file path.".into(),
            corrective_action: "Choose a new destination with the file picker.".into(),
            object_id: Some(value.into()),
        });
    }
    if fs::symlink_metadata(&path).is_ok() {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: "destination_exists".into(),
            message: "The selected destination already exists and will not be overwritten.".into(),
            corrective_action: "Choose a different, unused destination path.".into(),
            object_id: Some(value.into()),
        });
    }
    let Some(parent) = path.parent() else {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: "destination_parent_invalid".into(),
            message: "The destination has no parent folder.".into(),
            corrective_action: "Choose a destination inside an existing folder.".into(),
            object_id: Some(value.into()),
        });
    };
    if !parent.is_dir() {
        return Err(ProjectUpgradeDiagnosticV1 {
            code: "destination_parent_unavailable".into(),
            message: "The destination folder does not exist.".into(),
            corrective_action: "Choose a destination inside an existing folder.".into(),
            object_id: Some(value.into()),
        });
    }
    Ok(path)
}

fn read_source_bytes(path: &Path) -> Result<Vec<u8>, ProjectUpgradeDiagnosticV1> {
    let mut file = fs::File::open(path).map_err(|error| ProjectUpgradeDiagnosticV1 {
        code: "source_unavailable".into(),
        message: format!("The source project could not be opened: {error}"),
        corrective_action: "Choose a readable source project and try again.".into(),
        object_id: Some(path.display().to_string()),
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| ProjectUpgradeDiagnosticV1 {
            code: "source_unreadable".into(),
            message: format!("The source project could not be read: {error}"),
            corrective_action: "Choose an intact source project and try again.".into(),
            object_id: Some(path.display().to_string()),
        })?;
    Ok(bytes)
}

fn sha256_source_file(path: &Path) -> Result<String, ProjectUpgradeDiagnosticV1> {
    let mut file = fs::File::open(path).map_err(|error| ProjectUpgradeDiagnosticV1 {
        code: "source_unavailable".into(),
        message: format!("The source project could not be opened: {error}"),
        corrective_action: "Choose a readable source project and try again.".into(),
        object_id: Some(path.display().to_string()),
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ProjectUpgradeDiagnosticV1 {
                code: "source_unreadable".into(),
                message: format!("The source project could not be read: {error}"),
                corrective_action: "Choose an intact source project and try again.".into(),
                object_id: Some(path.display().to_string()),
            })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn future_standalone_inspection(
    source_archive_path: String,
    source_archive_sha256: String,
    summary: FutureProjectArchiveReadOnlyV6,
) -> ProjectUpgradeInspectionV1 {
    ProjectUpgradeInspectionV1 {
        source_archive_path,
        source_archive_sha256,
        source_kind: ProjectUpgradeSourceKindV1::StandaloneDocument,
        schema_version: summary.schema_version,
        access: ProjectUpgradeAccessV1::FutureReadOnly,
        read_only: true,
        upgrade_available: false,
        project_id: None,
        project_name: None,
        counts: ProjectUpgradeItemCountsV1 {
            datasets: summary.dataset_count,
            models: summary.model_count,
            recipes: summary.recipe_count,
            results: summary.result_count,
        },
        future_unsupported: ProjectUpgradeFutureUnsupportedV1 {
            models: summary.model_count,
            recipes: summary.recipe_count,
            results: summary.result_count,
        },
        source_will_remain_unchanged: true,
        destination_must_be_new: true,
    }
}

fn source_changed_during_inspection(source_archive_path: &str) -> ProjectUpgradeDiagnosticV1 {
    ProjectUpgradeDiagnosticV1 {
        code: "source_changed".into(),
        message: "The source project changed while QuickPLS was inspecting it.".into(),
        corrective_action: "Inspect the source project again before creating an upgrade plan."
            .into(),
        object_id: Some(source_archive_path.to_owned()),
    }
}

fn zero_counts() -> ProjectUpgradeItemCountsV1 {
    ProjectUpgradeItemCountsV1 {
        datasets: 0,
        models: 0,
        recipes: 0,
        results: 0,
    }
}

fn zero_future_unsupported() -> ProjectUpgradeFutureUnsupportedV1 {
    ProjectUpgradeFutureUnsupportedV1 {
        models: 0,
        recipes: 0,
        results: 0,
    }
}

fn project_error_diagnostic(error: ProjectArchiveV6Error) -> ProjectUpgradeDiagnosticV1 {
    let (code, corrective_action, object_id) = match &error {
        ProjectArchiveV6Error::DestinationExists(path) => (
            "destination_exists",
            "Choose a different, unused destination path.",
            Some(path.display().to_string()),
        ),
        ProjectArchiveV6Error::SourceDigestMismatch { .. }
        | ProjectArchiveV6Error::SourceChangedDuringUpgrade => (
            "source_changed",
            "Inspect the source project again before creating another copy.",
            None,
        ),
        ProjectArchiveV6Error::FutureSourceReadOnly(_) => (
            "future_project_read_only",
            "Open the project with a QuickPLS version that supports its schema.",
            None,
        ),
        ProjectArchiveV6Error::EstimandConfirmationRequired => (
            "estimand_confirmation_required",
            "Choose Composite or Common factor for every listed model.",
            None,
        ),
        ProjectArchiveV6Error::LegacyConversion(_) => (
            "model_conversion_unsupported",
            "Review the identified model and keep using the source project until its model shape can be converted safely.",
            None,
        ),
        ProjectArchiveV6Error::UpgradePathBinding { .. }
        | ProjectArchiveV6Error::DestinationMustBeNew
        | ProjectArchiveV6Error::EmptyUpgradePath => (
            "upgrade_path_changed",
            "Create a new plan using the exact source and a new destination.",
            None,
        ),
        _ => (
            "upgrade_plan_invalid",
            "Correct the identified project issue, then inspect and plan the copy again.",
            None,
        ),
    };
    ProjectUpgradeDiagnosticV1 {
        code: code.into(),
        message: error.to_string(),
        corrective_action: corrective_action.into(),
        object_id,
    }
}

fn zip_upgrade_error_diagnostic(
    error: ProjectArchiveUpgradeZipV6Error,
) -> ProjectUpgradeDiagnosticV1 {
    match error {
        ProjectArchiveUpgradeZipV6Error::Contract(error) => project_error_diagnostic(error),
        ProjectArchiveUpgradeZipV6Error::CleanupFailed {
            path,
            original_error,
            cleanup_error,
        } => ProjectUpgradeDiagnosticV1 {
            code: "upgrade_cleanup_failed".into(),
            message: format!(
                "Schema-6 ZIP cleanup failed for {} after {original_error}: {cleanup_error}",
                path.display()
            ),
            corrective_action:
                "Keep the source project and inspect the retained path before retrying.".into(),
            object_id: Some(path.display().to_string()),
        },
        ProjectArchiveUpgradeZipV6Error::DestinationOwnershipLost {
            path,
            retained_path,
            original_error,
            detail,
        } => ProjectUpgradeDiagnosticV1 {
            code: "upgrade_destination_ownership_lost".into(),
            message: format!(
                "The destination {} changed ownership after {original_error}. QuickPLS preserved the replacement at {}: {detail}",
                path.display(),
                retained_path.display()
            ),
            corrective_action:
                "Keep the source project, inspect the preserved replacement, and choose another unused destination."
                    .into(),
            object_id: Some(retained_path.display().to_string()),
        },
        ProjectArchiveUpgradeZipV6Error::TemporaryOwnershipLost {
            path,
            retained_path,
            original_error,
            detail,
        } => ProjectUpgradeDiagnosticV1 {
            code: "upgrade_temporary_ownership_lost".into(),
            message: format!(
                "The temporary upgrade file {} changed ownership after {original_error}. QuickPLS preserved the replacement at {}: {detail}",
                path.display(),
                retained_path.display()
            ),
            corrective_action:
                "Keep the source project, inspect the preserved replacement, and choose another unused destination."
                    .into(),
            object_id: Some(retained_path.display().to_string()),
        },
        other => {
            let (code, corrective_action) = match &other {
                ProjectArchiveUpgradeZipV6Error::Cancelled => (
                    "upgrade_cancelled",
                    "Create a new plan when you are ready to write the upgraded copy.",
                ),
                ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(_)
                | ProjectArchiveUpgradeZipV6Error::ProjectArchive(_) => (
                    "source_changed",
                    "Inspect the source project again before creating another copy.",
                ),
                ProjectArchiveUpgradeZipV6Error::ArchiveValidation { .. } => (
                    "upgrade_write_validation_failed",
                    "Keep using the unchanged source project and try a new destination.",
                ),
                ProjectArchiveUpgradeZipV6Error::ArchiveLimit(_)
                | ProjectArchiveUpgradeZipV6Error::Io(_)
                | ProjectArchiveUpgradeZipV6Error::Zip(_)
                | ProjectArchiveUpgradeZipV6Error::Json(_) => (
                    "upgrade_write_failed",
                    "Keep using the unchanged source project and try again.",
                ),
                ProjectArchiveUpgradeZipV6Error::Contract(_)
                | ProjectArchiveUpgradeZipV6Error::CleanupFailed { .. }
                | ProjectArchiveUpgradeZipV6Error::DestinationOwnershipLost { .. }
                | ProjectArchiveUpgradeZipV6Error::TemporaryOwnershipLost { .. } => {
                    unreachable!()
                }
            };
            ProjectUpgradeDiagnosticV1 {
                code: code.into(),
                message: other.to_string(),
                corrective_action: corrective_action.into(),
                object_id: None,
            }
        }
    }
}

fn state_unavailable_diagnostic() -> ProjectUpgradeDiagnosticV1 {
    ProjectUpgradeDiagnosticV1 {
        code: "upgrade_state_unavailable".into(),
        message: "The project upgrade assistant state is unavailable.".into(),
        corrective_action: "Close the assistant, reopen it, and create a new plan.".into(),
        object_id: None,
    }
}

fn unknown_plan_diagnostic(plan_id: Uuid) -> ProjectUpgradeDiagnosticV1 {
    ProjectUpgradeDiagnosticV1 {
        code: "upgrade_plan_not_found".into(),
        message: "The upgrade plan is no longer available and no file was written.".into(),
        corrective_action: "Create and review a new plan before writing the copy.".into(),
        object_id: Some(plan_id.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode, ModelSpec};
    use qpls_project::{save_project, write_project_document_v6_new};
    use tempfile::tempdir;

    fn gate() -> (String, bool) {
        (INTERNAL_LABS_SURFACE.into(), true)
    }

    fn save_source(project: &Project, path: &Path) -> String {
        save_project(path, project).unwrap();
        sha256_hex(&fs::read(path).unwrap())
    }

    fn simple_legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(91),
            name: "Ambiguous construct model".into(),
            constructs: vec![Construct {
                id: "x".into(),
                name: "X".into(),
                short_name: "X".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into()],
            }],
            paths: vec![],
            controls: vec![],
            higher_order_constructs: vec![],
            interactions: vec![],
        }
    }

    fn plan_request(
        source: &Path,
        destination: &Path,
        sha256: String,
    ) -> ProjectUpgradePlanRequestV1 {
        let (surface, experimental_labs_enabled) = gate();
        ProjectUpgradePlanRequestV1 {
            surface,
            experimental_labs_enabled,
            source_archive_path: source.to_str().unwrap().into(),
            destination_archive_path: destination.to_str().unwrap().into(),
            expected_source_archive_sha256: sha256,
            legacy_display_covariances: BTreeMap::new(),
            estimand_confirmations: BTreeMap::new(),
        }
    }

    #[test]
    fn inspection_reports_historical_and_future_sources_without_mutation() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let project = Project::new("Study");
        let source_sha256 = save_source(&project, &source);
        let (surface, experimental_labs_enabled) = gate();
        let outcome = inspect_project_upgrade_v6_impl(ProjectUpgradeInspectRequestV1 {
            surface: surface.clone(),
            experimental_labs_enabled,
            source_archive_path: source.to_str().unwrap().into(),
        });
        let ProjectUpgradeOutcomeV1::Ok { value } = outcome else {
            panic!("historical project must inspect");
        };
        assert_eq!(
            value.access,
            ProjectUpgradeAccessV1::HistoricalUpgradeCopyRequired
        );
        assert_eq!(value.source_archive_sha256, source_sha256);
        assert!(value.upgrade_available);
        assert!(!value.read_only);

        let future = directory.path().join("future.json");
        fs::write(
            &future,
            br#"{"schema_version":9,"datasets":[{}],"models":[{},{}],"recipes":[{}],"historical_results":[{},{}]}"#,
        )
        .unwrap();
        let outcome = inspect_project_upgrade_v6_impl(ProjectUpgradeInspectRequestV1 {
            surface,
            experimental_labs_enabled,
            source_archive_path: future.to_str().unwrap().into(),
        });
        let ProjectUpgradeOutcomeV1::Ok { value } = outcome else {
            panic!("future document must inspect");
        };
        assert_eq!(value.access, ProjectUpgradeAccessV1::FutureReadOnly);
        assert!(value.read_only);
        assert!(!value.upgrade_available);
        assert_eq!(value.counts.models, 2);
    }

    #[test]
    fn planning_requires_explicit_factor_or_composite_confirmation() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("ambiguous.qpls");
        let destination = directory.path().join("ambiguous-v6.qpls");
        let mut project = Project::new("Ambiguous");
        project.models.push(simple_legacy_model());
        let source_sha256 = save_source(&project, &source);
        let state = DesktopProjectUpgradePlans::default();
        let request = plan_request(&source, &destination, source_sha256.clone());
        let outcome = plan_project_upgrade_v6_impl(request, &state);
        let ProjectUpgradeOutcomeV1::Ok {
            value: ProjectUpgradePlanStateV1::ConfirmationRequired { prompts, .. },
        } = outcome
        else {
            panic!("ambiguous model must request confirmation");
        };
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].choices, ["composite", "common_factor"]);
        assert!(!destination.exists());

        let mut request = plan_request(&source, &destination, source_sha256);
        request.estimand_confirmations.insert(
            simple_legacy_model().id.to_string(),
            ProjectUpgradeEstimandChoiceV1::CommonFactor,
        );
        let outcome = plan_project_upgrade_v6_impl(request, &state);
        assert!(matches!(
            outcome,
            ProjectUpgradeOutcomeV1::Ok {
                value: ProjectUpgradePlanStateV1::Ready { .. }
            }
        ));
        assert!(!destination.exists());
    }

    #[test]
    fn plan_and_current_inspection_count_preserved_historical_recipes() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("recipe-count.qpls");
        let destination = directory.path().join("recipe-count-v6.json");
        let model = simple_legacy_model();
        let mut project = Project::new("Recipe count");
        project.models.push(model.clone());
        project.recipes.push(AnalysisRecipe::new(
            b"x1,x2\n1,2\n",
            model,
            AnalysisSettings::default(),
        ));
        let source_sha256 = save_source(&project, &source);
        let state = DesktopProjectUpgradePlans::default();
        let outcome = plan_project_upgrade_v6_impl(
            plan_request(&source, &destination, source_sha256),
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok {
            value:
                ProjectUpgradePlanStateV1::Ready {
                    plan_id,
                    recipe_count,
                    ..
                },
        } = outcome
        else {
            panic!("PLS recipe should produce a ready upgrade plan")
        };
        assert_eq!(recipe_count, 1);

        let document = state
            .0
            .lock()
            .unwrap()
            .get(&plan_id)
            .unwrap()
            .plan
            .document
            .clone();
        assert!(document.recipes.is_empty());
        assert_eq!(document.historical_recipes.len(), 1);
        let ready_model = match &document.models[0].payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. } => model.clone(),
            payload => panic!("PLS migration should produce a ready model, found {payload:?}"),
        };
        let draft_payload = ProjectModelPayloadV6::SemModelV4Draft {
            model_document_sha256: ready_model.model_document_sha256().unwrap(),
            model: ready_model,
        };
        assert!(!model_requires_estimand_confirmation(&draft_payload));
        write_project_document_v6_new(&destination, &document).unwrap();

        let (surface, experimental_labs_enabled) = gate();
        let inspection = inspect_project_upgrade_v6_impl(ProjectUpgradeInspectRequestV1 {
            surface,
            experimental_labs_enabled,
            source_archive_path: destination.to_str().unwrap().into(),
        });
        let ProjectUpgradeOutcomeV1::Ok { value } = inspection else {
            panic!("current standalone schema-v6 document must inspect")
        };
        assert_eq!(value.access, ProjectUpgradeAccessV1::CurrentV6Standalone);
        assert_eq!(value.counts.recipes, 1);
    }

    #[test]
    fn execute_writes_only_new_destination_and_preserves_source_identity() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let destination = directory.path().join("study-v6.qpls");
        let project = Project::new("Study");
        let source_sha256 = save_source(&project, &source);
        let state = DesktopProjectUpgradePlans::default();
        let outcome = plan_project_upgrade_v6_impl(
            plan_request(&source, &destination, source_sha256.clone()),
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok {
            value:
                ProjectUpgradePlanStateV1::Ready {
                    plan_id,
                    plan_sha256,
                    ..
                },
        } = outcome
        else {
            panic!("simple project must be ready");
        };
        let (surface, experimental_labs_enabled) = gate();
        let outcome = execute_project_upgrade_v6_impl(
            ProjectUpgradeExecuteRequestV1 {
                surface,
                experimental_labs_enabled,
                plan_id,
                expected_plan_sha256: plan_sha256,
                confirm_new_destination: true,
            },
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok { value } = outcome else {
            panic!("execution must succeed");
        };
        assert!(value.destination_written);
        assert!(destination.exists());
        assert_eq!(sha256_hex(&fs::read(&source).unwrap()), source_sha256);
        let reopened = load_project_archive_v6(&destination).unwrap();
        assert_eq!(reopened.document.project_id, project.manifest.project_id);

        let (surface, experimental_labs_enabled) = gate();
        let inspection = inspect_project_upgrade_v6_impl(ProjectUpgradeInspectRequestV1 {
            surface,
            experimental_labs_enabled,
            source_archive_path: destination.to_str().unwrap().into(),
        });
        let ProjectUpgradeOutcomeV1::Ok { value } = inspection else {
            panic!("current schema-6 ZIP must inspect through the strict reader");
        };
        assert_eq!(
            value.source_kind,
            ProjectUpgradeSourceKindV1::ProjectArchive
        );
        assert_eq!(value.access, ProjectUpgradeAccessV1::CurrentV6Archive);
        assert!(value.read_only);
        assert!(!value.upgrade_available);
    }

    #[test]
    fn cancel_removes_plan_without_creating_destination() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let destination = directory.path().join("study-v6.qpls");
        let source_sha256 = save_source(&Project::new("Study"), &source);
        let state = DesktopProjectUpgradePlans::default();
        let outcome = plan_project_upgrade_v6_impl(
            plan_request(&source, &destination, source_sha256),
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok {
            value:
                ProjectUpgradePlanStateV1::Ready {
                    plan_id,
                    plan_sha256,
                    ..
                },
        } = outcome
        else {
            panic!("plan must be ready");
        };
        let (surface, experimental_labs_enabled) = gate();
        let cancelled = cancel_project_upgrade_v6_impl(
            ProjectUpgradeCancelRequestV1 {
                surface: surface.clone(),
                experimental_labs_enabled,
                plan_id,
                expected_plan_sha256: plan_sha256.clone(),
            },
            &state,
        );
        assert!(matches!(
            cancelled,
            ProjectUpgradeOutcomeV1::Ok {
                value: ProjectUpgradeCancellationV1 {
                    cancelled: true,
                    destination_written: false,
                    ..
                }
            }
        ));
        let execution = execute_project_upgrade_v6_impl(
            ProjectUpgradeExecuteRequestV1 {
                surface,
                experimental_labs_enabled,
                plan_id,
                expected_plan_sha256: plan_sha256,
                confirm_new_destination: true,
            },
            &state,
        );
        assert!(matches!(
            execution,
            ProjectUpgradeOutcomeV1::Blocked {
                diagnostic: ProjectUpgradeDiagnosticV1 { ref code, .. }
            } if code == "upgrade_plan_not_found"
        ));
        assert!(!destination.exists());
    }

    #[test]
    fn cancel_signals_an_in_flight_writer_without_losing_its_cleanup_token() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let destination = directory.path().join("study-v6.qpls");
        let source_sha256 = save_source(&Project::new("Study"), &source);
        let state = DesktopProjectUpgradePlans::default();
        let outcome = plan_project_upgrade_v6_impl(
            plan_request(&source, &destination, source_sha256),
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok {
            value:
                ProjectUpgradePlanStateV1::Ready {
                    plan_id,
                    plan_sha256,
                    ..
                },
        } = outcome
        else {
            panic!("plan must be ready");
        };
        let token = {
            let mut plans = state.0.lock().unwrap();
            let pending = plans.get_mut(&plan_id).unwrap();
            pending.executing = true;
            Arc::clone(&pending.cancelled)
        };

        let (surface, experimental_labs_enabled) = gate();
        let cancelled = cancel_project_upgrade_v6_impl(
            ProjectUpgradeCancelRequestV1 {
                surface,
                experimental_labs_enabled,
                plan_id,
                expected_plan_sha256: plan_sha256,
            },
            &state,
        );

        assert!(matches!(
            cancelled,
            ProjectUpgradeOutcomeV1::Ok {
                value: ProjectUpgradeCancellationV1 {
                    cancelled: true,
                    destination_written: false,
                    ..
                }
            }
        ));
        assert!(token.load(Ordering::Acquire));
        assert!(state.0.lock().unwrap().contains_key(&plan_id));
        state.0.lock().unwrap().remove(&plan_id);
        assert!(!destination.exists());
    }

    #[test]
    fn execute_rechecks_source_identity_before_creating_destination() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let destination = directory.path().join("study-v6.qpls");
        let source_sha256 = save_source(&Project::new("Study"), &source);
        let state = DesktopProjectUpgradePlans::default();
        let outcome = plan_project_upgrade_v6_impl(
            plan_request(&source, &destination, source_sha256),
            &state,
        );
        let ProjectUpgradeOutcomeV1::Ok {
            value:
                ProjectUpgradePlanStateV1::Ready {
                    plan_id,
                    plan_sha256,
                    ..
                },
        } = outcome
        else {
            panic!("plan must be ready");
        };
        fs::write(&source, b"changed after planning").unwrap();
        let (surface, experimental_labs_enabled) = gate();
        let execution = execute_project_upgrade_v6_impl(
            ProjectUpgradeExecuteRequestV1 {
                surface,
                experimental_labs_enabled,
                plan_id,
                expected_plan_sha256: plan_sha256,
                confirm_new_destination: true,
            },
            &state,
        );
        assert!(matches!(
            execution,
            ProjectUpgradeOutcomeV1::Blocked {
                diagnostic: ProjectUpgradeDiagnosticV1 { ref code, .. }
            } if code == "source_changed"
        ));
        assert!(!destination.exists());
    }

    #[test]
    fn changed_source_and_existing_destination_fail_before_a_plan_is_stored() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("study.qpls");
        let destination = directory.path().join("study-v6.qpls");
        let source_sha256 = save_source(&Project::new("Study"), &source);
        let state = DesktopProjectUpgradePlans::default();
        let mut request = plan_request(&source, &destination, source_sha256);
        request.expected_source_archive_sha256 = "0".repeat(64);
        assert!(matches!(
            plan_project_upgrade_v6_impl(request, &state),
            ProjectUpgradeOutcomeV1::Blocked {
                diagnostic: ProjectUpgradeDiagnosticV1 { ref code, .. }
            } if code == "source_changed"
        ));

        fs::write(&destination, b"existing").unwrap();
        let request = plan_request(
            &source,
            &destination,
            sha256_hex(&fs::read(&source).unwrap()),
        );
        assert!(matches!(
            plan_project_upgrade_v6_impl(request, &state),
            ProjectUpgradeOutcomeV1::Blocked {
                diagnostic: ProjectUpgradeDiagnosticV1 { ref code, .. }
            } if code == "destination_exists"
        ));
    }
}
