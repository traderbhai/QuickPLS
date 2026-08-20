//! Archive-bound job lifecycle for the General SEM PLS mediation and moderation Labs slices.

use crate::{
    DesktopJobs, InternalRecipeV4ExecutionFailureV1,
    recipe_v4_general_sem_canonical_result::{
        build_recipe_v4_general_sem_pls_canonical_result_v1,
        general_sem_multiple_mediation_bootstrap_capability_cell_v1,
        general_sem_multiple_moderation_bootstrap_capability_cell_v1,
        general_sem_multiple_moderation_point_capability_cell_v1,
    },
    recipe_v4_jobs::{DesktopRecipeV4Jobs, reserve_general_sem_pls_admission},
};
use chrono::{SecondsFormat, Utc};
use qpls_core::{
    AnalysisRecipeModelBindingV4, AnalysisRecipeV4, CapabilityCellReferenceV2,
    CapabilityRegistryV2, GeneralSemInferenceV1, MissingDataPolicyV4, ObservedScaleV4,
    SemCapabilityDecisionStatusV1, SemDataBindingV4, SemModelV4, SemVariableV4,
    compile_general_sem_pls_recipe_v1, compile_unpublished_general_sem_pls_higher_order_recipe_v1,
    pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1,
    pls_general_recursive_effects_capability_cell_v1, preflight_general_sem_pls_v1,
    sha256_serialized,
};
use qpls_data::{ColumnType, DataKind, Dataset, ScaleType};
use qpls_project::{
    LoadedProjectArchiveV6, ProjectArchiveDocumentV6, ProjectModelPayloadV6,
    load_project_archive_v6_from_file,
};
use qpls_runner::{
    RecipeV4GeneralSemPlsExecutionErrorV1, RecipeV4GeneralSemPlsExecutionResultV1,
    run_compiled_general_sem_pls_recipe_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

const INTERNAL_LABS_SURFACE: &str = "internal_labs";
const GENERAL_SEM_JOB_SCHEMA_VERSION: u32 = 1;
const MAXIMUM_RETAINED_GENERAL_SEM_JOBS: usize = 255;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalLabsGeneralSemPlsJobRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
    expected_archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    recipe_id: String,
    recipe_document_sha256: String,
    capability_cell: CapabilityCellReferenceV2,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InternalLabsGeneralSemPlsJobStateV1 {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl InternalLabsGeneralSemPlsJobStateV1 {
    fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Cancelling)
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InternalLabsGeneralSemPlsFailureStageV1 {
    Access,
    ArchiveAuthority,
    Capability,
    Compilation,
    Estimation,
    Canonicalization,
    Integrity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalLabsGeneralSemPlsIssueV1 {
    code: String,
    subject: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalLabsGeneralSemPlsFailureV1 {
    schema_version: u32,
    stage: InternalLabsGeneralSemPlsFailureStageV1,
    subject: String,
    code: String,
    message: String,
    corrective_action: String,
    issues: Vec<InternalLabsGeneralSemPlsIssueV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalLabsGeneralSemPlsJobSnapshotV1 {
    schema_version: u32,
    job_id: Uuid,
    state: InternalLabsGeneralSemPlsJobStateV1,
    phase: String,
    completed_units: u64,
    total_units: u64,
    message: Option<String>,
    failure: Option<InternalLabsGeneralSemPlsFailureV1>,
    queued_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

impl InternalLabsGeneralSemPlsJobSnapshotV1 {
    fn queued(job_id: Uuid) -> Self {
        Self {
            schema_version: GENERAL_SEM_JOB_SCHEMA_VERSION,
            job_id,
            state: InternalLabsGeneralSemPlsJobStateV1::Queued,
            phase: "queued".into(),
            completed_units: 0,
            total_units: 1,
            message: None,
            failure: None,
            queued_at: now_utc(),
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemArchiveIdentityV1 {
    archive_path: String,
    archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    recipe_id: String,
    recipe_document_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalLabsGeneralSemPlsCompletedResultV1 {
    schema_version: u32,
    archive_identity: GeneralSemArchiveIdentityV1,
    analytical_result: RecipeV4GeneralSemPlsExecutionResultV1,
    canonical_document: qpls_core::CanonicalResultDocumentV2,
}

struct InternalLabsGeneralSemPlsJobV1 {
    snapshot: InternalLabsGeneralSemPlsJobSnapshotV1,
    cancellation: Arc<AtomicBool>,
    archive_identity: Option<GeneralSemArchiveIdentityV1>,
    result: Option<InternalLabsGeneralSemPlsCompletedResultV1>,
}

#[derive(Clone, Default)]
pub(crate) struct DesktopGeneralSemPlsJobsV1(
    Arc<Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>>,
);

struct ResolvedGeneralSemArchiveV1 {
    archive_identity: GeneralSemArchiveIdentityV1,
    document: ProjectArchiveDocumentV6,
    dataset: Dataset,
    model: SemModelV4,
    recipe: AnalysisRecipeV4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneralSemPlsWorkerCheckpointV1 {
    DuringInteractionExecution,
    DuringModerationBootstrapMid,
    DuringModerationBootstrapLate,
    DuringHocBootstrapMid,
    DuringHocBootstrapLate,
    AfterExecutionBeforeCanonicalization,
    AfterCanonicalizationBeforePublication,
}

type GeneralSemPlsWorkerCheckpointHookV1 =
    Arc<dyn Fn(GeneralSemPlsWorkerCheckpointV1) + Send + Sync>;

fn notify_worker_checkpoint(
    hook: &Option<GeneralSemPlsWorkerCheckpointHookV1>,
    checkpoint: GeneralSemPlsWorkerCheckpointV1,
) {
    if let Some(hook) = hook {
        hook(checkpoint);
    }
}

fn failure(
    stage: InternalLabsGeneralSemPlsFailureStageV1,
    subject: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> InternalLabsGeneralSemPlsFailureV1 {
    InternalLabsGeneralSemPlsFailureV1 {
        schema_version: GENERAL_SEM_JOB_SCHEMA_VERSION,
        stage,
        subject: subject.into(),
        code: code.into(),
        message: message.into(),
        corrective_action: corrective_action.into(),
        issues: Vec::new(),
    }
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_access(
    request: &InternalLabsGeneralSemPlsJobRequestV1,
) -> Result<(), InternalLabsGeneralSemPlsFailureV1> {
    // This must remain the first decision. Denied callers cannot inspect or
    // hash any filesystem path.
    if request.surface != INTERNAL_LABS_SURFACE || !request.experimental_labs_enabled {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Access,
            "experimentalLabsEnabled",
            "general_sem_pls.internal_labs_required",
            "General SEM PLS execution is available only through Experimental Labs.",
            "Enable Experimental Labs and use the General SEM workspace.",
        ));
    }
    let archive_path = Path::new(&request.archive_path);
    if request.archive_path.trim().is_empty()
        || request.archive_path != request.archive_path.trim()
        || !archive_path.is_absolute()
        || !is_supported_local_path(archive_path)
        || !archive_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("qpls"))
    {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.absolute_archive_path_required",
            "The General SEM archive path must be an exact absolute local path.",
            "Select the newly created schema-6 .qpls archive and retry.",
        ));
    }
    for (subject, digest) in [
        (
            "expectedArchiveSha256",
            request.expected_archive_sha256.as_str(),
        ),
        (
            "modelScientificSha256",
            request.model_scientific_sha256.as_str(),
        ),
        (
            "recipeDocumentSha256",
            request.recipe_document_sha256.as_str(),
        ),
    ] {
        if !is_lowercase_sha256(digest) {
            return Err(failure(
                InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
                subject,
                "general_sem_pls.invalid_sha256",
                format!("{subject} must be a lowercase SHA-256 digest."),
                "Reinspect the strict schema-6 archive and retry with its exact identities.",
            ));
        }
    }
    Ok(())
}

fn is_supported_local_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        true
    }
}

fn parse_non_nil_uuid(
    value: &str,
    subject: &'static str,
) -> Result<Uuid, InternalLabsGeneralSemPlsFailureV1> {
    Uuid::parse_str(value)
        .ok()
        .filter(|value| !value.is_nil())
        .ok_or_else(|| {
            failure(
                InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
                subject,
                "general_sem_pls.invalid_uuid",
                format!("{subject} must be a non-nil UUID."),
                "Reinspect the strict schema-6 archive and retry with its exact identities.",
            )
        })
}

fn open_regular_non_reparse_archive(
    path: &Path,
) -> Result<File, InternalLabsGeneralSemPlsFailureV1> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.archive_unavailable",
            "The General SEM archive is unavailable.",
            "Verify that the selected local .qpls archive still exists and retry.",
        )
    })?;
    #[cfg(windows)]
    let is_reparse = {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    };
    #[cfg(not(windows))]
    let is_reparse = metadata.file_type().is_symlink();
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() || is_reparse {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.regular_non_reparse_archive_required",
            "The General SEM source must be a regular local file and cannot be a link or reparse point.",
            "Select the exact regular schema-6 .qpls archive.",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|_| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.archive_open_failed",
            "The General SEM archive could not be locked for a stable read.",
            "Retry after other local operations finish using the project archive.",
        )
    })?;
    let opened_metadata = file.metadata().map_err(archive_read_failure)?;
    #[cfg(windows)]
    let opened_is_reparse = {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        opened_metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    };
    #[cfg(not(windows))]
    let opened_is_reparse = opened_metadata.file_type().is_symlink();
    if !opened_metadata.file_type().is_file() || opened_is_reparse {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.regular_non_reparse_archive_required",
            "The opened General SEM source identity is not a regular non-reparse file.",
            "Select the exact regular schema-6 .qpls archive.",
        ));
    }
    Ok(file)
}

fn sha256_file(file: &mut File) -> Result<String, InternalLabsGeneralSemPlsFailureV1> {
    file.seek(SeekFrom::Start(0))
        .map_err(archive_read_failure)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(archive_read_failure)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(archive_read_failure)?;
    Ok(format!("{:x}", digest.finalize()))
}

fn archive_read_failure(_: std::io::Error) -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
        "archivePath",
        "general_sem_pls.archive_read_failed",
        "The General SEM archive could not be read completely.",
        "Retry after other local operations finish using the project archive.",
    )
}

fn resolve_archive_authority(
    request: &InternalLabsGeneralSemPlsJobRequestV1,
) -> Result<ResolvedGeneralSemArchiveV1, InternalLabsGeneralSemPlsFailureV1> {
    validate_access(request)?;
    let project_id = parse_non_nil_uuid(&request.project_id, "projectId")?;
    let dataset_id = parse_non_nil_uuid(&request.dataset_id, "datasetId")?;
    let recipe_id = parse_non_nil_uuid(&request.recipe_id, "recipeId")?;
    if request.model_id.trim().is_empty() || request.model_id != request.model_id.trim() {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "modelId",
            "general_sem_pls.invalid_model_id",
            "modelId must be a nonempty exact stable identity.",
            "Reinspect the strict schema-6 archive and select its promoted model.",
        ));
    }

    let archive_path = PathBuf::from(&request.archive_path);
    let mut source = open_regular_non_reparse_archive(&archive_path)?;
    let observed_sha256 = sha256_file(&mut source)?;
    if observed_sha256 != request.expected_archive_sha256 {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "expectedArchiveSha256",
            "general_sem_pls.archive_changed",
            "The General SEM archive digest changed before execution began.",
            "Reinspect the archive, refresh all resident identities, and start a new job.",
        ));
    }
    let loaded =
        load_project_archive_v6_from_file(source.try_clone().map_err(archive_read_failure)?)
            .map_err(|_| {
                failure(
                    InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
                    "archivePath",
                    "general_sem_pls.invalid_schema6_archive",
                    "The selected file is not a valid strict schema-6 QuickPLS archive.",
                    "Restore or recreate the General SEM project from a trusted source.",
                )
            })?;
    if sha256_file(&mut source)? != observed_sha256 {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
            "archivePath",
            "general_sem_pls.archive_changed_during_read",
            "The General SEM archive changed while its authority was being resolved.",
            "Retry after all writers finish and the archive is stable.",
        ));
    }
    validate_loaded_authority(
        request,
        project_id,
        dataset_id,
        recipe_id,
        observed_sha256,
        loaded,
    )
}

fn validate_loaded_authority(
    request: &InternalLabsGeneralSemPlsJobRequestV1,
    project_id: Uuid,
    dataset_id: Uuid,
    recipe_id: Uuid,
    observed_sha256: String,
    loaded: LoadedProjectArchiveV6,
) -> Result<ResolvedGeneralSemArchiveV1, InternalLabsGeneralSemPlsFailureV1> {
    let LoadedProjectArchiveV6 {
        document, datasets, ..
    } = loaded;
    if !document.supports_general_sem_v1() || document.project_id != project_id {
        return Err(authority_mismatch(
            "projectId",
            "The archive is not the requested newly created general_sem_v1 project.",
        ));
    }
    if document.datasets.len() != 1
        || datasets.len() != 1
        || document.models.len() != 1
        || document.recipes.len() != 1
        || !document.historical_recipes.is_empty()
        || !document.historical_results.is_empty()
    {
        return Err(authority_mismatch(
            "archivePath",
            "The General SEM v1 execution archive must contain exactly one dataset, one promoted model, and one RecipeV4 with no legacy content.",
        ));
    }
    let dataset = datasets.into_iter().next().expect("exact count checked");
    if dataset.id != dataset_id
        || dataset.fingerprint.0 != request.dataset_fingerprint
        || document.datasets[0].id != dataset_id
        || document.datasets[0].fingerprint.0 != request.dataset_fingerprint
    {
        return Err(authority_mismatch(
            "datasetId",
            "The requested dataset identity differs from the sole resident dataset authority.",
        ));
    }
    let record = &document.models[0];
    let ProjectModelPayloadV6::SemModelV4 {
        model,
        scientific_sha256,
    } = &record.payload
    else {
        return Err(authority_mismatch(
            "modelId",
            "General SEM execution requires a promoted SemModelV4, not a draft or legacy model.",
        ));
    };
    if record.model_id != request.model_id
        || model.id != request.model_id
        || scientific_sha256 != &request.model_scientific_sha256
        || model.scientific_sha256().ok().as_deref()
            != Some(request.model_scientific_sha256.as_str())
    {
        return Err(authority_mismatch(
            "modelScientificSha256",
            "The requested model identity or digest differs from the promoted resident model.",
        ));
    }
    let recipe = &document.recipes[0];
    if recipe.id != recipe_id
        || sha256_serialized(recipe) != request.recipe_document_sha256
        || recipe.dataset_fingerprint != request.dataset_fingerprint
    {
        return Err(authority_mismatch(
            "recipeDocumentSha256",
            "The requested RecipeV4 identity or document digest differs from the resident recipe.",
        ));
    }
    match &recipe.model_binding {
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id,
            scientific_sha256,
        } if model_id == &request.model_id
            && scientific_sha256 == &request.model_scientific_sha256 => {}
        _ => {
            return Err(authority_mismatch(
                "recipeId",
                "The resident RecipeV4 is not bound to the promoted resident model authority.",
            ));
        }
    }
    let archive_identity = GeneralSemArchiveIdentityV1 {
        archive_path: request.archive_path.clone(),
        archive_sha256: observed_sha256,
        project_id: request.project_id.clone(),
        dataset_id: request.dataset_id.clone(),
        dataset_fingerprint: request.dataset_fingerprint.clone(),
        model_id: request.model_id.clone(),
        model_scientific_sha256: request.model_scientific_sha256.clone(),
        recipe_id: request.recipe_id.clone(),
        recipe_document_sha256: request.recipe_document_sha256.clone(),
    };
    let resident_model = model.clone();
    let resident_recipe = recipe.clone();
    Ok(ResolvedGeneralSemArchiveV1 {
        archive_identity,
        document,
        dataset,
        model: resident_model,
        recipe: resident_recipe,
    })
}

fn authority_mismatch(
    subject: impl Into<String>,
    message: impl Into<String>,
) -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::ArchiveAuthority,
        subject,
        "general_sem_pls.archive_authority_mismatch",
        message,
        "Reinspect the strict schema-6 archive and rebuild the execution request from its sole resident authorities.",
    )
}

fn validate_exact_capability_and_data(
    request: &InternalLabsGeneralSemPlsJobRequestV1,
    resolved: &ResolvedGeneralSemArchiveV1,
) -> Result<qpls_core::CompiledGeneralSemPlsRecipeV1, InternalLabsGeneralSemPlsFailureV1> {
    let config = resolved.recipe.general_sem_config.as_ref().ok_or_else(|| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::Capability,
            "recipeId",
            "general_sem_pls.general_sem_config_required",
            "The resident RecipeV4 does not contain GeneralSemConfigV1.",
            "Create a bound General SEM RecipeV4 before execution.",
        )
    })?;
    let has_higher_order = resolved
        .model
        .derived_terms
        .iter()
        .any(|term| matches!(term, qpls_core::SemDerivedTermV4::HigherOrder { .. }));
    let decision = preflight_general_sem_pls_v1(&resolved.model, config).map_err(|error| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::Capability,
            "modelId",
            "general_sem_pls.preflight_contract_invalid",
            format!("The authoritative PLS capability preflight failed: {error}"),
            "Keep the archive unchanged and report this capability-contract error.",
        )
    })?;
    if !has_higher_order && decision.status() == SemCapabilityDecisionStatusV1::Blocked {
        let issues = decision
            .diagnostics()
            .iter()
            .map(|diagnostic| InternalLabsGeneralSemPlsIssueV1 {
                code: diagnostic.code().into(),
                subject: diagnostic.subject().unwrap_or("model").into(),
                message: diagnostic.message().into(),
            })
            .collect();
        return Err(InternalLabsGeneralSemPlsFailureV1 {
            issues,
            ..failure(
                InternalLabsGeneralSemPlsFailureStageV1::Capability,
                "modelId",
                "general_sem_pls.preflight_blocked",
                decision.summary(),
                "Use the estimator compatibility inspector and correct every blocking diagnostic.",
            )
        });
    }

    validate_dataset_predicate(&resolved.dataset, &resolved.model)?;
    let artifact = (if has_higher_order {
        compile_unpublished_general_sem_pls_higher_order_recipe_v1(
            &resolved.recipe,
            Some(&resolved.model),
        )
    } else {
        compile_general_sem_pls_recipe_v1(&resolved.recipe, Some(&resolved.model))
    })
    .map_err(|error| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::Compilation,
            "recipeId",
            "general_sem_pls.compilation_failed",
            format!("The resident General SEM recipe could not compile: {error}"),
            "Correct the resident model/config and create a new General SEM project archive.",
        )
    })?;
    let has_interactions = !artifact.plan().two_way_interactions().is_empty();
    let expected_cell = if has_higher_order {
        match config.inference {
            GeneralSemInferenceV1::None => pls_general_higher_order_point_capability_cell_v1(),
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                pls_general_higher_order_bootstrap_capability_cell_v1()
            }
        }
    } else if has_interactions {
        match config.inference {
            GeneralSemInferenceV1::None => {
                general_sem_multiple_moderation_point_capability_cell_v1()
            }
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                general_sem_multiple_moderation_bootstrap_capability_cell_v1()
            }
        }
    } else {
        match config.inference {
            GeneralSemInferenceV1::None => pls_general_recursive_effects_capability_cell_v1(),
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                general_sem_multiple_mediation_bootstrap_capability_cell_v1()
            }
        }
    };
    let compiled_primary_cell = artifact.capability_cell();
    let compiled_cell_matches = if has_higher_order {
        compiled_primary_cell == &pls_general_higher_order_point_capability_cell_v1()
    } else if has_interactions {
        compiled_primary_cell == &general_sem_multiple_moderation_point_capability_cell_v1()
    } else {
        compiled_primary_cell == &pls_general_recursive_effects_capability_cell_v1()
    };
    if !compiled_cell_matches {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Capability,
            "capabilityCell",
            "general_sem_pls.compiled_capability_cell_mismatch",
            "The exact option cell does not match the authoritative compiled PLS v3 plan.",
            "Keep the archive unchanged and report this compiler/capability identity mismatch.",
        ));
    }
    if request.capability_cell != expected_cell {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Capability,
            "capabilityCell",
            "general_sem_pls.capability_cell_mismatch",
            "The selected option cell differs from the resident compiled model and inference request.",
            "Use the exact General SEM mediation point/bootstrap cell or simultaneous two-way moderation point/supplemental-bootstrap Labs cell selected by preflight.",
        ));
    }
    let registry = CapabilityRegistryV2::embedded().map_err(|error| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::Capability,
            "capabilityCell",
            "general_sem_pls.capability_registry_invalid",
            format!("Capability Registry V2 is invalid: {error}"),
            "Keep the archive unchanged and repair the embedded registry before execution.",
        )
    })?;
    if !has_higher_order {
        let matching_cells = registry
            .option_cells()
            .filter(|cell| {
                cell.capability_id == expected_cell.capability_id
                    && cell.cell_id == expected_cell.cell_id
                    && cell.capability_version == expected_cell.capability_version
            })
            .collect::<Vec<_>>();
        if !matches!(matching_cells.as_slice(), [cell] if cell.labs_available() || cell.standard_available())
        {
            return Err(failure(
                InternalLabsGeneralSemPlsFailureStageV1::Capability,
                "capabilityCell",
                "general_sem_pls.capability_unavailable",
                "The exact General SEM option cell is not uniquely available in Labs or Standard.",
                "Repair Capability Registry V2 before running this recipe.",
            ));
        }
    }

    let found = artifact.plan().topology().specific_directed_paths().len();
    match config.inference {
        GeneralSemInferenceV1::None if !has_higher_order && !has_interactions && found == 0 => {
            return Err(failure(
                InternalLabsGeneralSemPlsFailureStageV1::Capability,
                "modelId",
                "general_sem_pls.mediation_required",
                "The General SEM PLS mediation point cell requires at least one compiled indirect path.",
                "Author a supported mediator path, or use the existing ordinary PLS workflow for a direct-only recursive model.",
            ));
        }
        GeneralSemInferenceV1::CaseBootstrap { .. }
            if !has_higher_order && !has_interactions && found < 2 =>
        {
            return Err(failure(
                InternalLabsGeneralSemPlsFailureStageV1::Capability,
                "modelId",
                "general_sem_pls.multiple_mediation_required",
                "This Labs cell requires at least two distinct compiled indirect paths.",
                "Author parallel, serial, or mixed multiple mediation with at least two indirect paths.",
            ));
        }
        _ => {}
    }
    Ok(artifact)
}

fn validate_dataset_predicate(
    dataset: &Dataset,
    model: &SemModelV4,
) -> Result<(), InternalLabsGeneralSemPlsFailureV1> {
    let SemDataBindingV4::Raw {
        dataset_id,
        missing_data,
        weight,
        cluster_variable,
        strata_variable,
    } = &model.data_binding
    else {
        return Err(data_predicate_failure(
            "General SEM PLS mediation/moderation execution requires raw resident data.",
        ));
    };
    if dataset_id != &dataset.id.to_string()
        || *missing_data != MissingDataPolicyV4::ListwiseDeletion
        || weight.is_some()
        || cluster_variable.is_some()
        || strata_variable.is_some()
        || dataset.schema.kind != DataKind::Raw
    {
        return Err(data_predicate_failure(
            "The exact Labs cell requires raw unweighted single-level data with listwise deletion.",
        ));
    }
    for variable in &model.variables {
        let SemVariableV4::Observed {
            source_column,
            scale,
            missing_markers,
            transformation_lineage,
            ..
        } = variable
        else {
            if !matches!(
                variable,
                SemVariableV4::Composite { .. } | SemVariableV4::Derived { .. }
            ) {
                return Err(data_predicate_failure(
                    "The exact Labs cell accepts observed indicators, composite constructs, and compiler-bound derived interaction outputs only.",
                ));
            }
            continue;
        };
        let Some(metadata) = dataset
            .schema
            .columns
            .iter()
            .find(|column| column.name == *source_column)
        else {
            return Err(data_predicate_failure(format!(
                "Observed source column {source_column} is absent from the resident dataset."
            )));
        };
        if *scale != ObservedScaleV4::Continuous
            || !missing_markers.is_empty()
            || !transformation_lineage.is_empty()
            || metadata.column_type != ColumnType::Numeric
            || metadata.scale_type != ScaleType::Continuous
        {
            return Err(data_predicate_failure(format!(
                "Observed source column {source_column} must be continuous numeric data for this exact Labs cell. Missing rows are handled by the declared listwise-deletion policy."
            )));
        }
    }
    Ok(())
}

fn data_predicate_failure(message: impl Into<String>) -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::Capability,
        "datasetId",
        "general_sem_pls.data_predicate_blocked",
        message,
        "Use continuous numeric raw data with listwise deletion and without weights, clusters, strata, or transformation lineage.",
    )
}

fn verify_archive_identity(
    identity: &GeneralSemArchiveIdentityV1,
) -> Result<(), InternalLabsGeneralSemPlsFailureV1> {
    let mut file = open_regular_non_reparse_archive(Path::new(&identity.archive_path))?;
    let observed = sha256_file(&mut file)?;
    if observed != identity.archive_sha256 {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Integrity,
            "archivePath",
            "general_sem_pls.stale_archive",
            "The General SEM archive changed after job admission; no result was published.",
            "Reinspect the archive and start a new analysis from its current resident authorities.",
        ));
    }
    Ok(())
}

fn map_shared_admission_failure(
    shared: InternalRecipeV4ExecutionFailureV1,
) -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::Integrity,
        shared.subject,
        shared.code,
        shared.message,
        shared.corrective_action,
    )
}

fn set_running(
    jobs: &Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>,
    job_id: Uuid,
    phase: &str,
    completed_units: u64,
    total_units: u64,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == InternalLabsGeneralSemPlsJobStateV1::Queued {
            job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        if job.snapshot.state.is_active() {
            job.snapshot.phase = phase.into();
            job.snapshot.completed_units = completed_units.min(total_units);
            job.snapshot.total_units = total_units.max(1);
        }
    }
}

fn finish_cancelled(jobs: &Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>, job_id: Uuid) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn finish_failed(
    jobs: &Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>,
    job_id: Uuid,
    terminal_failure: InternalLabsGeneralSemPlsFailureV1,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Failed;
        job.snapshot.phase = "failed".into();
        job.snapshot.message = Some(terminal_failure.message.clone());
        job.snapshot.failure = Some(terminal_failure);
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>) {
    if jobs.len() <= MAXIMUM_RETAINED_GENERAL_SEM_JOBS {
        return;
    }
    let removable = jobs
        .iter()
        .filter_map(|(job_id, job)| job.snapshot.state.is_terminal().then_some(*job_id))
        .take(jobs.len() - MAXIMUM_RETAINED_GENERAL_SEM_JOBS)
        .collect::<Vec<_>>();
    for job_id in removable {
        jobs.remove(&job_id);
    }
}

fn run_worker(
    job_id: Uuid,
    request: InternalLabsGeneralSemPlsJobRequestV1,
    resolved: ResolvedGeneralSemArchiveV1,
    cancellation: Arc<AtomicBool>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>>,
    _admission: crate::recipe_v4_jobs::PlsModelComparisonAdmissionReservationV1,
) {
    run_worker_with_checkpoint_hook(
        job_id,
        request,
        resolved,
        cancellation,
        jobs,
        _admission,
        None,
    );
}

fn run_worker_with_checkpoint_hook(
    job_id: Uuid,
    request: InternalLabsGeneralSemPlsJobRequestV1,
    resolved: ResolvedGeneralSemArchiveV1,
    cancellation: Arc<AtomicBool>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalLabsGeneralSemPlsJobV1>>>,
    _admission: crate::recipe_v4_jobs::PlsModelComparisonAdmissionReservationV1,
    checkpoint_hook: Option<GeneralSemPlsWorkerCheckpointHookV1>,
) {
    set_running(&jobs, job_id, "capability_preflight", 0, 1);
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    let artifact = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        validate_exact_capability_and_data(&request, &resolved)
    })) {
        Ok(Ok(artifact)) => artifact,
        Ok(Err(error)) => {
            finish_failed(&jobs, job_id, error);
            return;
        }
        Err(_) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    InternalLabsGeneralSemPlsFailureStageV1::Integrity,
                    "compilation",
                    "general_sem_pls.preflight_terminated_unexpectedly",
                    "The General SEM capability/compilation worker terminated unexpectedly.",
                    "Discard the job and retry. If it repeats, export a diagnostic bundle.",
                ),
            );
            return;
        }
    };
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    set_running(&jobs, job_id, "compilation", 1, 1);
    let execution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if cancellation.load(Ordering::Acquire) {
            return Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled);
        }
        run_compiled_general_sem_pls_recipe_v1(
            &resolved.dataset,
            &resolved.recipe,
            &resolved.model,
            &artifact,
            || cancellation.load(Ordering::Acquire),
            |progress| {
                if progress.phase == "general_sem_interaction_point"
                    && progress.completed_units == 0
                {
                    notify_worker_checkpoint(
                        &checkpoint_hook,
                        GeneralSemPlsWorkerCheckpointV1::DuringInteractionExecution,
                    );
                }
                if progress.phase == "general_sem_moderation_bootstrap"
                    && progress.total_units > 1
                    && progress.completed_units == progress.total_units / 2
                {
                    notify_worker_checkpoint(
                        &checkpoint_hook,
                        GeneralSemPlsWorkerCheckpointV1::DuringModerationBootstrapMid,
                    );
                }
                if progress.phase == "general_sem_hoc_bootstrap"
                    && progress.total_units > 1
                    && progress.completed_units == progress.total_units / 2
                {
                    notify_worker_checkpoint(
                        &checkpoint_hook,
                        GeneralSemPlsWorkerCheckpointV1::DuringHocBootstrapMid,
                    );
                }
                if progress.phase == "general_sem_hoc_bootstrap"
                    && progress.total_units > 1
                    && progress.completed_units == progress.total_units - 1
                {
                    notify_worker_checkpoint(
                        &checkpoint_hook,
                        GeneralSemPlsWorkerCheckpointV1::DuringHocBootstrapLate,
                    );
                }
                if progress.phase == "general_sem_moderation_bootstrap"
                    && progress.total_units > 1
                    && progress.completed_units == progress.total_units - 1
                {
                    notify_worker_checkpoint(
                        &checkpoint_hook,
                        GeneralSemPlsWorkerCheckpointV1::DuringModerationBootstrapLate,
                    );
                }
                set_running(
                    &jobs,
                    job_id,
                    &progress.phase,
                    progress.completed_units,
                    progress.total_units,
                );
            },
        )
    }));
    let analytical_result = match execution {
        Ok(Ok(result)) => result,
        Ok(Err(RecipeV4GeneralSemPlsExecutionErrorV1::Cancelled)) => {
            finish_cancelled(&jobs, job_id);
            return;
        }
        Ok(Err(error)) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    InternalLabsGeneralSemPlsFailureStageV1::Estimation,
                    "recipeId",
                    "general_sem_pls.execution_failed",
                    format!("General SEM PLS execution failed: {error}"),
                    "Review the model/data diagnostics, then create a corrected project or retry the unchanged archive.",
                ),
            );
            return;
        }
        Err(_) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    InternalLabsGeneralSemPlsFailureStageV1::Integrity,
                    "execution",
                    "general_sem_pls.worker_terminated_unexpectedly",
                    "The General SEM PLS worker terminated unexpectedly.",
                    "Discard the job and retry. If it repeats, export a diagnostic bundle.",
                ),
            );
            return;
        }
    };
    notify_worker_checkpoint(
        &checkpoint_hook,
        GeneralSemPlsWorkerCheckpointV1::AfterExecutionBeforeCanonicalization,
    );
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    set_running(&jobs, job_id, "canonicalization", 0, 1);
    let started_at = jobs
        .lock()
        .ok()
        .and_then(|jobs| {
            jobs.get(&job_id)
                .and_then(|job| job.snapshot.started_at.clone())
        })
        .unwrap_or_else(now_utc);
    let completed_at = now_utc();
    let canonicalization = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_recipe_v4_general_sem_pls_canonical_result_v1(
            job_id,
            resolved.document.project_id,
            resolved.dataset.id,
            &started_at,
            &completed_at,
            &resolved.recipe,
            &resolved.model,
            &analytical_result,
        )
    }));
    let canonical_document = match canonicalization {
        Ok(Ok(document)) => document,
        Ok(Err(errors)) => {
            finish_failed(
                &jobs,
                job_id,
                InternalLabsGeneralSemPlsFailureV1 {
                    issues: errors
                        .iter()
                        .enumerate()
                        .map(|(index, message)| InternalLabsGeneralSemPlsIssueV1 {
                            code: format!("general_sem_pls.canonical_issue_{index:04}"),
                            subject: "canonicalDocument".into(),
                            message: message.clone(),
                        })
                        .collect(),
                    ..failure(
                        InternalLabsGeneralSemPlsFailureStageV1::Canonicalization,
                        "canonicalDocument",
                        "general_sem_pls.canonicalization_failed",
                        "The completed analytical result could not satisfy the canonical result contract.",
                        "Keep the archive unchanged and report the canonical validation issues.",
                    )
                },
            );
            return;
        }
        Err(_) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    InternalLabsGeneralSemPlsFailureStageV1::Canonicalization,
                    "canonicalDocument",
                    "general_sem_pls.canonicalization_terminated_unexpectedly",
                    "The General SEM canonicalization worker terminated unexpectedly.",
                    "Discard the job and retry. If it repeats, export a diagnostic bundle.",
                ),
            );
            return;
        }
    };
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    if let Err(error) = verify_archive_identity(&resolved.archive_identity) {
        finish_failed(&jobs, job_id, error);
        return;
    }
    notify_worker_checkpoint(
        &checkpoint_hook,
        GeneralSemPlsWorkerCheckpointV1::AfterCanonicalizationBeforePublication,
    );
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == InternalLabsGeneralSemPlsJobStateV1::Cancelling
            || job.cancellation.load(Ordering::Acquire)
        {
            job.result = None;
            job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Cancelled;
            job.snapshot.phase = "cancelled".into();
            job.snapshot.message = None;
            job.snapshot.failure = None;
            job.snapshot.completed_at = Some(now_utc());
            return;
        }
        job.result = Some(InternalLabsGeneralSemPlsCompletedResultV1 {
            schema_version: GENERAL_SEM_JOB_SCHEMA_VERSION,
            archive_identity: resolved.archive_identity,
            analytical_result,
            canonical_document,
        });
        job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Completed;
        job.snapshot.phase = "completed".into();
        job.snapshot.completed_units = 1;
        job.snapshot.total_units = 1;
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(completed_at);
    }
}

#[tauri::command]
pub(crate) fn start_internal_labs_general_sem_pls_job_v1(
    request: InternalLabsGeneralSemPlsJobRequestV1,
    standard_jobs: State<'_, DesktopJobs>,
    shared_recipe_jobs: State<'_, DesktopRecipeV4Jobs>,
    jobs: State<'_, DesktopGeneralSemPlsJobsV1>,
) -> Result<InternalLabsGeneralSemPlsJobSnapshotV1, InternalLabsGeneralSemPlsFailureV1> {
    validate_access(&request)?;
    let resolved = resolve_archive_authority(&request)?;
    let job_id = Uuid::new_v4();
    let admission = reserve_general_sem_pls_admission(
        job_id,
        resolved.recipe.settings.workers,
        standard_jobs.0.clone(),
        shared_recipe_jobs.inner().clone(),
    )
    .map_err(map_shared_admission_failure)?;
    let snapshot = InternalLabsGeneralSemPlsJobSnapshotV1::queued(job_id);
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = jobs.0.lock().map_err(|_| {
            failure(
                InternalLabsGeneralSemPlsFailureStageV1::Integrity,
                "jobs",
                "general_sem_pls.job_state_unavailable",
                "The General SEM job state is temporarily unavailable.",
                "Retry after current analyses finish.",
            )
        })?;
        prune_terminal_jobs(&mut jobs);
        jobs.insert(
            job_id,
            InternalLabsGeneralSemPlsJobV1 {
                snapshot: snapshot.clone(),
                cancellation: cancellation.clone(),
                archive_identity: Some(resolved.archive_identity.clone()),
                result: None,
            },
        );
    }
    let jobs = jobs.0.clone();
    let worker_jobs = jobs.clone();
    let spawn_result = std::thread::Builder::new()
        .name(format!("qpls-general-sem-pls-{job_id}"))
        .spawn(move || {
            run_worker(
                job_id,
                request,
                resolved,
                cancellation,
                worker_jobs,
                admission,
            )
        });
    if spawn_result.is_err() {
        if let Ok(mut jobs) = jobs.lock() {
            jobs.remove(&job_id);
        }
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Integrity,
            "jobs",
            "general_sem_pls.worker_spawn_failed",
            "The General SEM worker could not be started; no analysis was run.",
            "Retry after other local analyses finish. If it repeats, restart QuickPLS.",
        ));
    }
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn status_internal_labs_general_sem_pls_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopGeneralSemPlsJobsV1>,
) -> Result<InternalLabsGeneralSemPlsJobSnapshotV1, InternalLabsGeneralSemPlsFailureV1> {
    let jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    jobs.get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| unknown_job(job_id))
}

#[tauri::command]
pub(crate) fn cancel_internal_labs_general_sem_pls_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopGeneralSemPlsJobsV1>,
) -> Result<InternalLabsGeneralSemPlsJobSnapshotV1, InternalLabsGeneralSemPlsFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| unknown_job(job_id))?;
    if matches!(
        job.snapshot.state,
        InternalLabsGeneralSemPlsJobStateV1::Queued | InternalLabsGeneralSemPlsJobStateV1::Running
    ) {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = InternalLabsGeneralSemPlsJobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn dismiss_internal_labs_general_sem_pls_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopGeneralSemPlsJobsV1>,
) -> Result<(), InternalLabsGeneralSemPlsFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let terminal = jobs
        .get(&job_id)
        .map(|job| job.snapshot.state.is_terminal())
        .ok_or_else(|| unknown_job(job_id))?;
    if !terminal {
        return Err(failure(
            InternalLabsGeneralSemPlsFailureStageV1::Integrity,
            "jobId",
            "general_sem_pls.active_job_cannot_be_dismissed",
            "An active General SEM job cannot be dismissed.",
            "Wait for completion or cancellation before dismissing the job.",
        ));
    }
    jobs.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn result_internal_labs_general_sem_pls_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopGeneralSemPlsJobsV1>,
) -> Result<InternalLabsGeneralSemPlsCompletedResultV1, InternalLabsGeneralSemPlsFailureV1> {
    take_completed_result(job_id, jobs.inner())
}

fn take_completed_result(
    job_id: Uuid,
    jobs: &DesktopGeneralSemPlsJobsV1,
) -> Result<InternalLabsGeneralSemPlsCompletedResultV1, InternalLabsGeneralSemPlsFailureV1> {
    let identity = {
        let jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
        let job = jobs.get(&job_id).ok_or_else(|| unknown_job(job_id))?;
        if job.snapshot.state != InternalLabsGeneralSemPlsJobStateV1::Completed {
            return Err(failure(
                InternalLabsGeneralSemPlsFailureStageV1::Integrity,
                "jobId",
                "general_sem_pls.result_not_available",
                "A General SEM result is available only after successful completion.",
                "Wait for completion or inspect the typed terminal failure.",
            ));
        }
        job.archive_identity.clone().ok_or_else(|| {
            failure(
                InternalLabsGeneralSemPlsFailureStageV1::Integrity,
                "jobId",
                "general_sem_pls.archive_identity_missing",
                "The completed General SEM job lost its archive identity.",
                "Discard the job and run the analysis again.",
            )
        })?
    };
    if let Err(error) = verify_archive_identity(&identity) {
        if let Ok(mut jobs) = jobs.0.lock() {
            jobs.remove(&job_id);
        }
        return Err(error);
    }
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let mut job = jobs.remove(&job_id).ok_or_else(|| unknown_job(job_id))?;
    job.result.take().ok_or_else(|| {
        failure(
            InternalLabsGeneralSemPlsFailureStageV1::Integrity,
            "jobId",
            "general_sem_pls.completed_result_missing",
            "The completed General SEM job did not retain its result.",
            "Discard the job and run the analysis again.",
        )
    })
}

fn job_state_failure() -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::Integrity,
        "jobs",
        "general_sem_pls.job_state_unavailable",
        "The General SEM job state is temporarily unavailable.",
        "Retry after current analyses finish.",
    )
}

fn unknown_job(job_id: Uuid) -> InternalLabsGeneralSemPlsFailureV1 {
    failure(
        InternalLabsGeneralSemPlsFailureStageV1::Integrity,
        "jobId",
        "general_sem_pls.unknown_job",
        format!("No General SEM PLS job exists with ID {job_id}."),
        "Refresh the General SEM job state and select an existing job.",
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::{
        project_archive_v6_general_sem_bootstrap::tests::{
            GeneralSemNativeFixtureV1, general_sem_native_fixture_v1,
        },
        project_schema6_result_append::{
            ProjectSchema6ResultAppendOutcomeV1, ProjectSchema6ResultAppendRequestV1,
            append_internal_project_schema6_canonical_result_v2,
        },
        project_schema6_result_read::{
            ProjectSchema6ResultReadOutcomeV1, ProjectSchema6ResultReadRequestV1,
            read_internal_project_schema6_canonical_results_v2,
        },
    };
    use chrono::TimeZone;
    use qpls_core::{
        AnalysisRecipeModelBindingV4, CompositeWeightingV4, GeneralSemBootstrapIntervalV1,
        GeneralSemConfigV1, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
        HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4,
        InteractionHierarchyPolicyV2, InteractionMethodV4, PlsBootstrapTestTail, SemDerivedTermV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
        StructuralRelationRoleV4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_project::{
        canonical_result_document_v2_json, create_populated_general_sem_project_archive_v6,
        load_project_archive_v6,
    };

    struct PublishedFixtureV1 {
        _directory: tempfile::TempDir,
        request: InternalLabsGeneralSemPlsJobRequestV1,
    }

    fn published_fixture_from(
        fixture: GeneralSemNativeFixtureV1,
        file_name: &str,
        capability_cell: CapabilityCellReferenceV2,
    ) -> PublishedFixtureV1 {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join(file_name);
        let receipt = create_populated_general_sem_project_archive_v6(
            &destination,
            Uuid::from_u128(0x7201),
            "General SEM native job fixture",
            Utc.timestamp_opt(1_700_000_100, 0).unwrap(),
            &fixture.dataset,
            fixture.model,
            fixture.recipe,
        )
        .unwrap();
        PublishedFixtureV1 {
            _directory: directory,
            request: InternalLabsGeneralSemPlsJobRequestV1 {
                surface: INTERNAL_LABS_SURFACE.into(),
                experimental_labs_enabled: true,
                archive_path: receipt.destination_archive_path,
                expected_archive_sha256: receipt.destination_archive_sha256,
                project_id: receipt.project_id.to_string(),
                dataset_id: receipt.resident_dataset_id.to_string(),
                dataset_fingerprint: receipt.resident_dataset_fingerprint,
                model_id: receipt.resident_model_id,
                model_scientific_sha256: receipt.resident_model_scientific_sha256,
                recipe_id: receipt.resident_recipe_id.to_string(),
                recipe_document_sha256: receipt.resident_recipe_document_sha256,
                capability_cell,
            },
        }
    }

    fn published_fixture() -> PublishedFixtureV1 {
        published_fixture_from(
            general_sem_native_fixture_v1(),
            "general-sem-point.qpls",
            pls_general_recursive_effects_capability_cell_v1(),
        )
    }

    fn direct_only_moderation_fixture(same_focal: bool) -> GeneralSemNativeFixtureV1 {
        let mut fixture = general_sem_native_fixture_v1();
        let retained_pairs = [
            ("construct:x", "construct:y"),
            ("construct:m1", "construct:y"),
            ("construct:m2", "construct:y"),
        ];
        let mut removed_parameter_ids = std::collections::BTreeSet::new();
        fixture.model.relations.retain(|relation| match relation {
            SemRelationV4::Structural {
                source,
                target,
                parameter,
                ..
            } if !retained_pairs.contains(&(source.as_str(), target.as_str())) => {
                removed_parameter_ids.insert(parameter.clone());
                false
            }
            _ => true,
        });
        fixture
            .model
            .parameters
            .retain(|parameter| !removed_parameter_ids.contains(parameter.id()));

        let x_to_y = structural_relation_id(&fixture.model, "construct:x", "construct:y");
        add_two_way_interaction(
            &mut fixture.model,
            "interaction:x_by_m1",
            "construct:x",
            "construct:m1",
            &x_to_y,
        );
        if same_focal {
            add_two_way_interaction(
                &mut fixture.model,
                "interaction:x_by_m2",
                "construct:x",
                "construct:m2",
                &x_to_y,
            );
        } else {
            let m1_to_y = structural_relation_id(&fixture.model, "construct:m1", "construct:y");
            add_two_way_interaction(
                &mut fixture.model,
                "interaction:m1_by_m2",
                "construct:m1",
                "construct:m2",
                &m1_to_y,
            );
        }
        fixture.model.ensure_valid().unwrap();
        fixture.recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: fixture.model.id.clone(),
            scientific_sha256: fixture.model.scientific_sha256().unwrap(),
        };
        fixture
    }

    fn moderation_bootstrap_fixture(same_focal: bool) -> GeneralSemNativeFixtureV1 {
        let mut fixture = direct_only_moderation_fixture(same_focal);
        fixture.recipe.settings.bootstrap_samples = 20;
        fixture.recipe.settings.seed = qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1;
        fixture.recipe.settings.confidence_level = 0.95;
        fixture.recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
        fixture.recipe.settings.studentized_inner_samples = 0;
        fixture.recipe.settings.workers = 1;
        fixture.recipe.general_sem_config = Some(GeneralSemConfigV1 {
            inference: GeneralSemInferenceV1::CaseBootstrap {
                resamples: 20,
                seed: qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1,
                confidence_level: 0.95,
                interval: GeneralSemBootstrapIntervalV1::Percentile,
                tail: GeneralSemInferenceTailV1::TwoSided,
            },
            ..GeneralSemConfigV1::default()
        });
        fixture.recipe.ensure_valid().unwrap();
        fixture
    }

    fn disjoint_hoc_fixture(
        measurement_type: HigherOrderMeasurementTypeV4,
        bootstrap: bool,
    ) -> GeneralSemNativeFixtureV1 {
        let mut fixture = general_sem_native_fixture_v1();
        let components = ["construct:m1", "construct:m2"];
        let mode_b = matches!(
            measurement_type,
            HigherOrderMeasurementTypeV4::FormativeReflective
                | HigherOrderMeasurementTypeV4::FormativeFormative
        );
        let mut removed_parameter_ids = std::collections::BTreeSet::new();
        fixture.model.relations.retain(|relation| match relation {
            SemRelationV4::Structural {
                source,
                target,
                parameter,
                ..
            } if components.contains(&source.as_str()) || components.contains(&target.as_str()) => {
                removed_parameter_ids.insert(parameter.clone());
                false
            }
            _ => true,
        });
        fixture
            .model
            .parameters
            .retain(|parameter| !removed_parameter_ids.contains(parameter.id()));
        if mode_b {
            for variable in &mut fixture.model.variables {
                if let SemVariableV4::Composite { id, weighting, .. } = variable
                    && components.contains(&id.as_str())
                {
                    *weighting = CompositeWeightingV4::ModeB;
                }
            }
            for relation in &mut fixture.model.relations {
                if let SemRelationV4::MeasurementEffect {
                    id,
                    construct,
                    indicator,
                    parameter,
                } = relation.clone()
                    && components.contains(&construct.as_str())
                {
                    *relation = SemRelationV4::MeasurementCausal {
                        id,
                        indicator,
                        composite: construct,
                        parameter,
                    };
                }
            }
            for parameter in &mut fixture.model.parameters {
                if let SemParameterV4::Free { target, .. } = parameter
                    && let SemParameterTargetV4::Loading {
                        construct,
                        indicator,
                    } = target.clone()
                    && components.contains(&construct.as_str())
                {
                    *target = SemParameterTargetV4::Weight {
                        indicator,
                        composite: construct,
                    };
                }
            }
        }

        let output = "derived:hoc".to_string();
        fixture.model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: "Higher order".into(),
        });
        for (id, parameter, source, target) in [
            (
                "relation:x_hoc",
                "parameter:x_hoc",
                "construct:x",
                output.as_str(),
            ),
            (
                "relation:hoc_y",
                "parameter:hoc_y",
                output.as_str(),
                "construct:y",
            ),
        ] {
            fixture.model.relations.push(SemRelationV4::Structural {
                id: id.into(),
                source: source.into(),
                target: target.into(),
                parameter: parameter.into(),
                role: StructuralRelationRoleV4::Structural,
                intercept_parameter: None,
            });
            fixture.model.parameters.push(SemParameterV4::Free {
                id: parameter.into(),
                label: format!("{source} -> {target}"),
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
        fixture
            .model
            .derived_terms
            .push(SemDerivedTermV4::HigherOrder {
                id: "term:hoc".into(),
                output,
                components: components.into_iter().map(str::to_string).collect(),
                approach: HigherOrderConstructionApproachV4::DisjointTwoStage,
                measurement_type,
            });
        fixture.model.ensure_valid().unwrap();
        fixture.recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: fixture.model.id.clone(),
            scientific_sha256: fixture.model.scientific_sha256().unwrap(),
        };
        if bootstrap {
            fixture.recipe.settings.bootstrap_samples = 20;
            fixture.recipe.settings.seed = 7_301;
            fixture.recipe.settings.confidence_level = 0.95;
            fixture.recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
            fixture.recipe.settings.studentized_inner_samples = 0;
            fixture.recipe.settings.workers = 1;
            fixture.recipe.general_sem_config = Some(GeneralSemConfigV1 {
                inference: GeneralSemInferenceV1::CaseBootstrap {
                    resamples: 20,
                    seed: 7_301,
                    confidence_level: 0.95,
                    interval: GeneralSemBootstrapIntervalV1::Percentile,
                    tail: GeneralSemInferenceTailV1::TwoSided,
                },
                ..GeneralSemConfigV1::default()
            });
        } else {
            fixture.recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        }
        fixture.recipe.ensure_valid().unwrap();
        fixture
    }

    fn structural_relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source: relation_source,
                    target: relation_target,
                    ..
                } if relation_source == source && relation_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn add_two_way_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
        focal_relation_id: &str,
    ) {
        let outcome_id = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural { id, target, .. } if id == focal_relation_id => {
                    Some(target.clone())
                }
                _ => None,
            })
            .unwrap();
        let output_id = format!("derived:{interaction_id}");
        let effect_relation_id = format!("relation:{interaction_id}:effect");
        let effect_parameter_id = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output_id.clone(),
            label: interaction_id.into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: effect_relation_id,
            source: output_id.clone(),
            target: outcome_id.clone(),
            parameter: effect_parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: effect_parameter_id,
            label: format!("{interaction_id} -> {outcome_id}"),
            target: SemParameterTargetV4::Regression {
                source: output_id.clone(),
                target: outcome_id,
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.into(),
            output: output_id,
            operands: vec![focal_predictor_id.into(), moderator_id.into()],
            focal_relation: focal_relation_id.into(),
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
    }

    fn job_state(
        job_id: Uuid,
        cancellation: Arc<AtomicBool>,
        identity: GeneralSemArchiveIdentityV1,
    ) -> DesktopGeneralSemPlsJobsV1 {
        let state = DesktopGeneralSemPlsJobsV1::default();
        state.0.lock().unwrap().insert(
            job_id,
            InternalLabsGeneralSemPlsJobV1 {
                snapshot: InternalLabsGeneralSemPlsJobSnapshotV1::queued(job_id),
                cancellation,
                archive_identity: Some(identity),
                result: None,
            },
        );
        state
    }

    fn admission(job_id: Uuid) -> crate::recipe_v4_jobs::PlsModelComparisonAdmissionReservationV1 {
        reserve_general_sem_pls_admission(
            job_id,
            1,
            Arc::new(Mutex::new(HashMap::new())),
            DesktopRecipeV4Jobs::default(),
        )
        .unwrap()
    }

    fn completed_moderation_job(
        same_focal: bool,
        job_id: Uuid,
    ) -> InternalLabsGeneralSemPlsCompletedResultV1 {
        let published = published_fixture_from(
            direct_only_moderation_fixture(same_focal),
            if same_focal {
                "general-sem-same-focal-moderation.qpls"
            } else {
                "general-sem-different-focal-moderation.qpls"
            },
            general_sem_multiple_moderation_point_capability_cell_v1(),
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker(
            job_id,
            published.request,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );
        assert_eq!(
            state.0.lock().unwrap().get(&job_id).unwrap().snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Completed
        );
        take_completed_result(job_id, &state).unwrap()
    }

    fn completed_moderation_bootstrap_job(
        same_focal: bool,
        job_id: Uuid,
    ) -> InternalLabsGeneralSemPlsCompletedResultV1 {
        let published = published_fixture_from(
            moderation_bootstrap_fixture(same_focal),
            if same_focal {
                "general-sem-same-focal-moderation-bootstrap.qpls"
            } else {
                "general-sem-different-focal-moderation-bootstrap.qpls"
            },
            general_sem_multiple_moderation_bootstrap_capability_cell_v1(),
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker(
            job_id,
            published.request,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );
        assert_eq!(
            state.0.lock().unwrap().get(&job_id).unwrap().snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Completed
        );
        take_completed_result(job_id, &state).unwrap()
    }

    fn completed_disjoint_hoc_job(
        measurement_type: HigherOrderMeasurementTypeV4,
        bootstrap: bool,
        job_id: Uuid,
    ) -> (
        PublishedFixtureV1,
        InternalLabsGeneralSemPlsCompletedResultV1,
    ) {
        let published = published_fixture_from(
            disjoint_hoc_fixture(measurement_type, bootstrap),
            if bootstrap {
                "general-sem-disjoint-hoc-bootstrap.qpls"
            } else {
                "general-sem-disjoint-hoc-point.qpls"
            },
            if bootstrap {
                pls_general_higher_order_bootstrap_capability_cell_v1()
            } else {
                pls_general_higher_order_point_capability_cell_v1()
            },
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker(
            job_id,
            published.request.clone(),
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );
        assert_eq!(
            state.0.lock().unwrap().get(&job_id).unwrap().snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Completed
        );
        let completed = take_completed_result(job_id, &state).unwrap();
        (published, completed)
    }

    fn assert_moderation_result_surface_identity(document: &qpls_core::CanonicalResultDocumentV2) {
        let results = document.general_sem_results.as_ref().unwrap();
        let structural_table = document
            .tables
            .iter()
            .find(|table| table.id == "structural_paths")
            .unwrap();
        let structural_ledger = results
            .joint_stage_structural_coefficients
            .iter()
            .filter(|coefficient| {
                coefficient.role == qpls_core::CanonicalStructuralRelationRoleV1::Structural
            })
            .collect::<Vec<_>>();
        assert_eq!(structural_table.rows.len(), structural_ledger.len());
        for coefficient in structural_ledger {
            let row = structural_table
                .rows
                .iter()
                .find(|row| {
                    matches!(
                        row.cells.first(),
                        Some(qpls_core::CanonicalResultCell::Text { value })
                            if value == &coefficient.relation_id
                    )
                })
                .unwrap();
            assert!(matches!(
                row.cells.as_slice(),
                [
                    qpls_core::CanonicalResultCell::Text { value: relation_id },
                    qpls_core::CanonicalResultCell::Text { value: parameter_id },
                    qpls_core::CanonicalResultCell::Text { value: source_id },
                    qpls_core::CanonicalResultCell::Text { value: target_id },
                    qpls_core::CanonicalResultCell::Number { value: estimate, .. },
                ] if relation_id == &coefficient.relation_id
                    && parameter_id == &coefficient.parameter_id
                    && source_id == &coefficient.source_id
                    && target_id == &coefficient.target_id
                    && estimate.to_bits() == coefficient.estimate.estimate.to_bits()
            ));
        }
        let control_ledger = results
            .joint_stage_structural_coefficients
            .iter()
            .filter(|coefficient| {
                coefficient.role == qpls_core::CanonicalStructuralRelationRoleV1::Control
            })
            .collect::<Vec<_>>();
        let control_table = document
            .tables
            .iter()
            .find(|table| table.id == qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
        if control_ledger.is_empty() {
            assert!(control_table.is_none());
        } else {
            let control_table = control_table.unwrap();
            assert_eq!(control_table.rows.len(), control_ledger.len());
            for coefficient in control_ledger {
                let row = control_table
                    .rows
                    .iter()
                    .find(|row| {
                        matches!(
                            row.cells.first(),
                            Some(qpls_core::CanonicalResultCell::Text { value })
                                if value == &coefficient.relation_id
                        )
                    })
                    .unwrap();
                assert!(matches!(
                    row.cells.as_slice(),
                    [
                        qpls_core::CanonicalResultCell::Text { value: relation_id },
                        qpls_core::CanonicalResultCell::Text { value: parameter_id },
                        qpls_core::CanonicalResultCell::Text { value: source_id },
                        qpls_core::CanonicalResultCell::Text { value: target_id },
                        _,
                        qpls_core::CanonicalResultCell::Number { value: estimate, .. },
                    ] if relation_id == &coefficient.relation_id
                        && parameter_id == &coefficient.parameter_id
                        && source_id == &coefficient.source_id
                        && target_id == &coefficient.target_id
                        && estimate.to_bits() == coefficient.estimate.estimate.to_bits()
                ));
            }
        }
        let interaction_table = document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_interaction_effects")
            .unwrap();
        assert_eq!(
            interaction_table.rows.len(),
            results.interaction_effects.len()
        );
        for (row, effect) in interaction_table
            .rows
            .iter()
            .zip(&results.interaction_effects)
        {
            assert!(matches!(
                row.cells.as_slice(),
                [
                    qpls_core::CanonicalResultCell::Text { value: effect_id },
                    qpls_core::CanonicalResultCell::Text { value: interaction_id },
                    qpls_core::CanonicalResultCell::Text { value: focal_relation_id },
                    ..
                ] if effect_id == &effect.effect_id
                    && interaction_id == &effect.interaction_id
                    && focal_relation_id == &effect.focal_relation_id
            ));
        }

        let slope_table = document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_conditional_slopes")
            .unwrap();
        assert_eq!(slope_table.rows.len(), results.conditional_effects.len());
        for (row, effect) in slope_table.rows.iter().zip(&results.conditional_effects) {
            assert!(matches!(
                row.cells.as_slice(),
                [
                    qpls_core::CanonicalResultCell::Text { value: effect_id },
                    qpls_core::CanonicalResultCell::Text { value: interaction_id },
                    qpls_core::CanonicalResultCell::Text { value: interaction_effect_id },
                    qpls_core::CanonicalResultCell::Text { value: focal_relation_id },
                    qpls_core::CanonicalResultCell::Text { value: probe_id },
                    qpls_core::CanonicalResultCell::Number { value: probe_index, .. },
                    _,
                    _,
                    qpls_core::CanonicalResultCell::Number { value: moderator_value, .. },
                    qpls_core::CanonicalResultCell::Number { value: estimate, .. },
                    _,
                ] if effect_id == &effect.effect_id
                    && interaction_id == &effect.interaction_id
                    && Some(interaction_effect_id.as_str()) == effect.interaction_effect_id.as_deref()
                    && focal_relation_id == &effect.focal_relation_id
                    && probe_id == &effect.probe_id
                    && probe_index.to_bits() == f64::from(effect.probe_value_index).to_bits()
                    && moderator_value.to_bits() == effect.moderator_value.to_bits()
                    && estimate.to_bits() == effect.value.estimate.to_bits()
            ));
        }

        let plot_table = document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_interaction_plots")
            .unwrap();
        let expected_plot_rows = results
            .interaction_plots
            .iter()
            .map(|plot| {
                plot.series
                    .iter()
                    .map(|series| series.points.len())
                    .sum::<usize>()
            })
            .sum::<usize>();
        assert_eq!(plot_table.rows.len(), expected_plot_rows);
        assert_eq!(document.charts.len(), results.interaction_plots.len());
        for (index, (chart, plot)) in document
            .charts
            .iter()
            .zip(&results.interaction_plots)
            .enumerate()
        {
            assert_eq!(
                chart.id,
                format!("general_sem_interaction_chart_{index:04}")
            );
            assert_eq!(
                chart.source_table_id.as_deref(),
                Some("general_sem_interaction_plots")
            );
            assert_eq!(chart.series.len(), plot.series.len());
            for (chart_series, plot_series) in chart.series.iter().zip(&plot.series) {
                assert_eq!(chart_series.id, plot_series.series_id);
                assert_eq!(
                    chart_series.group.as_deref(),
                    Some(plot.interaction_id.as_str())
                );
                assert_eq!(chart_series.points.len(), plot_series.points.len());
                for (chart_point, plot_point) in chart_series.points.iter().zip(&plot_series.points)
                {
                    assert!(matches!(
                        chart_point.x,
                        qpls_core::CanonicalChartX::Number(value)
                            if value.to_bits() == plot_point.focal_value.to_bits()
                    ));
                    assert_eq!(
                        chart_point.y.to_bits(),
                        plot_point.predicted_value.to_bits()
                    );
                    assert_eq!(
                        chart_point.lower.map(f64::to_bits),
                        plot_point.lower.map(f64::to_bits)
                    );
                    assert_eq!(
                        chart_point.upper.map(f64::to_bits),
                        plot_point.upper.map(f64::to_bits)
                    );
                }
            }
        }
    }

    fn assert_moderation_bootstrap_surface_identity(
        completed: &InternalLabsGeneralSemPlsCompletedResultV1,
    ) {
        let document = &completed.canonical_document;
        assert_moderation_result_surface_identity(document);
        let point_cell = general_sem_multiple_moderation_point_capability_cell_v1();
        let bootstrap_cell = general_sem_multiple_moderation_bootstrap_capability_cell_v1();
        assert_eq!(document.provenance.capability_cell, point_cell);
        assert_eq!(
            document.provenance.method_version,
            qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        );
        assert_eq!(
            completed.analytical_result.adapter_version(),
            qpls_runner::RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        );
        assert_eq!(
            document.title,
            "General SEM simultaneous two-way PLS moderation bootstrap inference"
        );
        let cells = document.capability_cells.as_ref().unwrap();
        assert_eq!(cells.len(), 3);
        assert!(cells.contains(&point_cell));
        assert!(cells.contains(&bootstrap_cell));

        let results = document.general_sem_results.as_ref().unwrap();
        let receipt = results.inference_receipt.as_ref().unwrap();
        let raw = completed
            .analytical_result
            .moderation_bootstrap_inference()
            .unwrap();
        assert_eq!(receipt.capability_cell, bootstrap_cell);
        assert_eq!(receipt.effect_ids, raw.gamma_target_ids);
        assert_eq!(receipt.resamples_requested, raw.resamples_requested);
        assert_eq!(receipt.resamples_usable, raw.resamples_usable);
        assert_eq!(receipt.failed_replicates.len(), raw.failed_replicates.len());
        assert_eq!(
            results.interaction_effects.len(),
            raw.interaction_gammas.len()
        );
        for (effect, gamma) in results
            .interaction_effects
            .iter()
            .zip(&raw.interaction_gammas)
        {
            assert_eq!(effect.effect_id, gamma.target.target_id);
            assert_eq!(effect.interaction_id, gamma.target.interaction_id);
            assert_eq!(effect.focal_relation_id, gamma.target.focal_relation_id);
            assert_eq!(
                effect.interaction_effect_relation_id,
                gamma.target.interaction_effect_relation_id
            );
            assert_eq!(
                effect.interaction_effect_parameter_id,
                gamma.target.interaction_effect_parameter_id
            );
            assert_eq!(
                effect.generated_product_column_id,
                gamma.target.generated_product_column_id
            );
            assert_eq!(effect.focal_predictor_id, gamma.target.focal_predictor_id);
            assert_eq!(effect.moderator_id, gamma.target.moderator_id);
            assert_eq!(effect.outcome_id, gamma.target.outcome_id);
            assert_eq!(
                effect.stage_one_model_scientific_sha256,
                gamma.target.stage_one_model_scientific_sha256
            );
            assert_eq!(
                effect.product_scale_version,
                gamma.target.product_scale_version
            );
            assert_eq!(effect.method_version, gamma.target.method_version);
            assert_eq!(
                effect.scientific_rescaled_gamma.estimate.to_bits(),
                gamma.original.to_bits()
            );
            assert_eq!(
                effect
                    .scientific_rescaled_gamma
                    .standard_error
                    .map(f64::to_bits),
                Some(gamma.standard_error.to_bits())
            );
            assert!(
                effect
                    .standardized_product_coefficient
                    .standard_error
                    .is_none()
            );
        }
        assert!(
            results
                .joint_stage_structural_coefficients
                .iter()
                .all(|coefficient| coefficient.estimate.standard_error.is_none())
        );
        assert!(
            results
                .conditional_effects
                .iter()
                .all(|effect| effect.value.standard_error.is_none())
        );
        assert!(
            document
                .charts
                .iter()
                .all(|chart| chart.series.iter().all(|series| {
                    series
                        .points
                        .iter()
                        .all(|point| point.lower.is_none() && point.upper.is_none())
                }))
        );

        let point_table_ids = [
            "general_sem_interaction_effects",
            "general_sem_conditional_slopes",
            "general_sem_interaction_plots",
        ];
        for table_id in point_table_ids {
            let table = document
                .tables
                .iter()
                .find(|table| table.id == table_id)
                .unwrap();
            assert_eq!(
                table.capability_cells.as_deref(),
                Some(&[point_cell.clone()][..])
            );
        }
        let gamma_table = document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_moderation_gamma_inference")
            .unwrap();
        let receipt_table = document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_moderation_bootstrap_receipt")
            .unwrap();
        assert_eq!(gamma_table.rows.len(), results.interaction_effects.len());
        assert_eq!(gamma_table.columns.len(), 21);
        assert_eq!(receipt_table.rows.len(), 1);
        assert_eq!(receipt_table.columns.len(), 40);
        assert_eq!(
            gamma_table.capability_cells.as_deref(),
            Some(&[bootstrap_cell.clone()][..])
        );
        assert_eq!(
            receipt_table.capability_cells.as_deref(),
            Some(&[bootstrap_cell.clone()][..])
        );
        assert_eq!(
            document
                .sections
                .iter()
                .rev()
                .take(2)
                .map(|section| section.id.as_str())
                .collect::<Vec<_>>(),
            vec!["general_sem_moderation_bootstrap", "general_sem_moderation"]
        );
        let bootstrap_section = document.sections.last().unwrap();
        assert!(bootstrap_section.chart_ids.is_empty());
        assert_eq!(
            bootstrap_section.table_ids,
            [
                "general_sem_moderation_gamma_inference",
                "general_sem_moderation_bootstrap_receipt",
            ]
        );
    }

    #[test]
    fn job_request_wire_is_strict_camel_case_and_denies_unknown_fields() {
        let published = published_fixture();
        let request = published.request;
        let valid = serde_json::json!({
            "surface": request.surface,
            "experimentalLabsEnabled": request.experimental_labs_enabled,
            "archivePath": request.archive_path,
            "expectedArchiveSha256": request.expected_archive_sha256,
            "projectId": request.project_id,
            "datasetId": request.dataset_id,
            "datasetFingerprint": request.dataset_fingerprint,
            "modelId": request.model_id,
            "modelScientificSha256": request.model_scientific_sha256,
            "recipeId": request.recipe_id,
            "recipeDocumentSha256": request.recipe_document_sha256,
            "capabilityCell": request.capability_cell,
        });
        serde_json::from_value::<InternalLabsGeneralSemPlsJobRequestV1>(valid.clone()).unwrap();

        let mut unknown = valid.clone();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<InternalLabsGeneralSemPlsJobRequestV1>(unknown).is_err());

        let mut snake = valid;
        let body = snake.as_object_mut().unwrap();
        let digest = body.remove("expectedArchiveSha256").unwrap();
        body.insert("expected_archive_sha256".into(), digest);
        assert!(serde_json::from_value::<InternalLabsGeneralSemPlsJobRequestV1>(snake).is_err());
    }

    #[test]
    fn archive_sha_model_and_recipe_guards_reject_tampering() {
        let published = published_fixture();

        let mut archive_tamper = published.request.clone();
        archive_tamper.expected_archive_sha256 = "0".repeat(64);
        let failure = match resolve_archive_authority(&archive_tamper) {
            Err(failure) => failure,
            Ok(_) => panic!("archive digest tamper was accepted"),
        };
        assert_eq!(failure.code, "general_sem_pls.archive_changed");
        assert_eq!(failure.subject, "expectedArchiveSha256");

        let mut model_tamper = published.request.clone();
        model_tamper.model_scientific_sha256 = "0".repeat(64);
        let failure = match resolve_archive_authority(&model_tamper) {
            Err(failure) => failure,
            Ok(_) => panic!("model digest tamper was accepted"),
        };
        assert_eq!(failure.code, "general_sem_pls.archive_authority_mismatch");
        assert_eq!(failure.subject, "modelScientificSha256");

        let mut recipe_tamper = published.request.clone();
        recipe_tamper.recipe_document_sha256 = "0".repeat(64);
        let failure = match resolve_archive_authority(&recipe_tamper) {
            Err(failure) => failure,
            Ok(_) => panic!("recipe document digest tamper was accepted"),
        };
        assert_eq!(failure.code, "general_sem_pls.archive_authority_mismatch");
        assert_eq!(failure.subject, "recipeDocumentSha256");
    }

    #[test]
    fn exact_capability_mismatch_is_blocked_after_authority_resolution() {
        let published = published_fixture();
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let mut mismatch = published.request.clone();
        mismatch.capability_cell = general_sem_multiple_mediation_bootstrap_capability_cell_v1();

        let failure = validate_exact_capability_and_data(&mismatch, &resolved).unwrap_err();
        assert_eq!(
            failure.stage,
            InternalLabsGeneralSemPlsFailureStageV1::Capability
        );
        assert_eq!(failure.subject, "capabilityCell");
        assert_eq!(failure.code, "general_sem_pls.capability_cell_mismatch");
    }

    #[test]
    fn point_job_accepts_one_indirect_path_while_the_bootstrap_cell_stays_narrow() {
        let published = published_fixture();
        let mut resolved = resolve_archive_authority(&published.request).unwrap();
        let mut removed_parameter_ids = std::collections::HashSet::new();
        resolved.model.relations.retain(|relation| {
            if let SemRelationV4::Structural {
                source,
                target,
                parameter,
                ..
            } = relation
                && (source == "construct:m2" || target == "construct:m2")
            {
                removed_parameter_ids.insert(parameter.clone());
                false
            } else {
                true
            }
        });
        resolved
            .model
            .parameters
            .retain(|parameter| !removed_parameter_ids.contains(parameter.id()));
        resolved.recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: resolved.model.id.clone(),
            scientific_sha256: resolved.model.scientific_sha256().unwrap(),
        };

        let artifact = validate_exact_capability_and_data(&published.request, &resolved).unwrap();
        assert_eq!(
            artifact.plan().topology().specific_directed_paths().len(),
            1
        );
    }

    #[test]
    fn same_focal_moderation_job_publishes_joint_stage_two_authority() {
        let completed = completed_moderation_job(true, Uuid::from_u128(0x7310));
        let moderation_cell = general_sem_multiple_moderation_point_capability_cell_v1();
        assert_eq!(
            completed.canonical_document.provenance.capability_cell,
            moderation_cell
        );
        assert_eq!(
            completed.canonical_document.title,
            "General SEM simultaneous two-way PLS moderation point estimates"
        );
        let interactions = completed
            .analytical_result
            .interaction_point_estimation()
            .unwrap();
        assert_eq!(interactions.interaction_coefficients().len(), 2);
        assert_eq!(
            interactions
                .interaction_coefficients()
                .iter()
                .map(|coefficient| coefficient.focal_relation_id())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            1
        );
        let final_coefficients = interactions
            .structural_coefficients()
            .iter()
            .map(|coefficient| {
                (
                    coefficient.relation_id().to_string(),
                    (
                        coefficient.source_id().to_string(),
                        coefficient.target_id().to_string(),
                        coefficient.estimate(),
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let published_coefficients = completed
            .canonical_document
            .tables
            .iter()
            .find(|table| table.id == "structural_paths")
            .unwrap()
            .rows
            .iter()
            .map(|row| match row.cells.as_slice() {
                [
                    qpls_core::CanonicalResultCell::Text { value: relation_id },
                    qpls_core::CanonicalResultCell::Text {
                        value: _parameter_id,
                    },
                    qpls_core::CanonicalResultCell::Text { value: source },
                    qpls_core::CanonicalResultCell::Text { value: target },
                    qpls_core::CanonicalResultCell::Number { value, .. },
                ] => (
                    relation_id.clone(),
                    (source.clone(), target.clone(), *value),
                ),
                _ => panic!("unexpected joint structural row"),
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(published_coefficients, final_coefficients);
        let stage_one_coefficients = completed
            .analytical_result
            .point_estimation()
            .estimation()
            .paths
            .iter()
            .map(|path| ((path.source.clone(), path.target.clone()), path.coefficient))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert!(
            final_coefficients
                .values()
                .any(|(source, target, coefficient)| {
                    stage_one_coefficients
                        .get(&(source.clone(), target.clone()))
                        .is_some_and(|stage_one| stage_one.to_bits() != coefficient.to_bits())
                })
        );
        for stale_table in [
            "effects",
            "r_squared",
            qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2,
            qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
        ] {
            assert!(
                completed
                    .canonical_document
                    .tables
                    .iter()
                    .all(|table| table.id != stale_table)
            );
        }
        let canonical = completed
            .canonical_document
            .general_sem_results
            .as_ref()
            .unwrap();
        assert!(canonical.specific_indirect_effects.is_empty());
        assert!(canonical.aggregate_effects.is_empty());
        assert_eq!(canonical.interaction_effects.len(), 2);
        assert_eq!(canonical.conditional_effects.len(), 6);
        assert_eq!(canonical.interaction_plots.len(), 2);
        assert!(completed.canonical_document.tables.iter().any(|table| {
            table.id == "general_sem_interaction_effects" && table.rows.len() == 2
        }));
        assert_eq!(completed.canonical_document.charts.len(), 2);
        assert!(completed.canonical_document.tables.iter().any(|table| {
            table.id == "general_sem_interaction_plots" && table.rows.len() == 18
        }));
        let measurement_section = completed
            .canonical_document
            .sections
            .iter()
            .find(|section| section.id == "measurement_model")
            .unwrap();
        assert_eq!(measurement_section.title, "Stage-one measurement model");
        assert!(measurement_section.table_ids.iter().all(|table_id| {
            completed
                .canonical_document
                .tables
                .iter()
                .find(|table| table.id.as_str() == table_id.as_str())
                .is_some_and(|table| table.title.starts_with("Stage-one "))
        }));
        let plot_table = completed
            .canonical_document
            .tables
            .iter()
            .find(|table| table.id == "general_sem_interaction_plots")
            .unwrap();
        assert!(completed.canonical_document.charts.iter().all(|chart| {
            chart.source_table_id.as_deref() == Some(plot_table.id.as_str())
                && chart.series.len() == 3
                && chart.series.iter().all(|series| series.points.len() == 3)
        }));
    }

    #[test]
    fn different_focal_moderation_job_publishes_both_joint_equations() {
        let completed = completed_moderation_job(false, Uuid::from_u128(0x7311));
        let canonical = completed
            .canonical_document
            .general_sem_results
            .as_ref()
            .unwrap();
        assert_eq!(canonical.interaction_effects.len(), 2);
        assert_eq!(
            canonical
                .interaction_effects
                .iter()
                .map(|effect| effect.focal_relation_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            2
        );
        assert_eq!(canonical.conditional_effect_probes.len(), 2);
        assert_eq!(canonical.conditional_effects.len(), 6);
        assert_eq!(canonical.interaction_plots.len(), 2);
        assert!(canonical.interaction_plots.iter().all(|plot| {
            plot.series.len() == 3 && plot.series.iter().all(|series| series.points.len() == 3)
        }));
    }

    #[test]
    fn same_and_different_focal_moderation_bootstrap_jobs_publish_exact_gamma_only_inference() {
        for (same_focal, job_id) in [
            (true, Uuid::from_u128(0x7350)),
            (false, Uuid::from_u128(0x7351)),
        ] {
            let completed = completed_moderation_bootstrap_job(same_focal, job_id);
            assert_moderation_bootstrap_surface_identity(&completed);
            let effects = &completed
                .canonical_document
                .general_sem_results
                .as_ref()
                .unwrap()
                .interaction_effects;
            assert_eq!(effects.len(), 2);
            assert_eq!(
                effects
                    .iter()
                    .map(|effect| effect.focal_relation_id.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                if same_focal { 1 } else { 2 }
            );
        }
    }

    #[test]
    fn disjoint_hoc_native_point_jobs_publish_all_four_exact_hcm_contracts() {
        for (index, measurement_type) in [
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
            HigherOrderMeasurementTypeV4::FormativeReflective,
            HigherOrderMeasurementTypeV4::FormativeFormative,
        ]
        .into_iter()
        .enumerate()
        {
            let (_, completed) = completed_disjoint_hoc_job(
                measurement_type.clone(),
                false,
                Uuid::from_u128(0x73a0 + index as u128),
            );
            let document = &completed.canonical_document;
            let results = document.general_sem_results.as_ref().unwrap();
            assert_eq!(results.higher_order_stages.len(), 2);
            assert!(results.higher_order_inference_receipt.is_none());
            assert_eq!(
                results.higher_order_stages[1].measurement_type.as_ref(),
                Some(&measurement_type)
            );
            assert_eq!(
                results.higher_order_stages[1].approach.as_ref(),
                Some(&HigherOrderConstructionApproachV4::DisjointTwoStage)
            );
            let expected_component_kind = match measurement_type {
                HigherOrderMeasurementTypeV4::ReflectiveReflective
                | HigherOrderMeasurementTypeV4::FormativeReflective => {
                    qpls_core::CanonicalHocRelationKindV1::ComponentLoading
                }
                HigherOrderMeasurementTypeV4::ReflectiveFormative
                | HigherOrderMeasurementTypeV4::FormativeFormative => {
                    qpls_core::CanonicalHocRelationKindV1::ComponentWeight
                }
            };
            assert_eq!(
                results.higher_order_stages[1]
                    .relation_estimates
                    .iter()
                    .filter(|relation| relation.kind == Some(expected_component_kind))
                    .count(),
                2
            );
            assert!(document.tables.iter().any(|table| {
                table.id == "general_sem_higher_order_stages" && table.rows.len() == 2
            }));
            assert!(document.tables.iter().any(|table| {
                table.id == "general_sem_higher_order_targets"
                    && table.rows.len() == results.higher_order_stages[1].relation_estimates.len()
            }));
            assert!(
                document
                    .tables
                    .iter()
                    .all(|table| table.id != "general_sem_higher_order_bootstrap_receipt")
            );
        }
    }

    #[test]
    fn disjoint_hoc_bootstrap_appends_reopens_and_rejects_table_or_receipt_tampering() {
        let (published, completed) = completed_disjoint_hoc_job(
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            true,
            Uuid::from_u128(0x73b0),
        );
        let archive_path = PathBuf::from(&published.request.archive_path);
        let before_bytes = fs::read(&archive_path).unwrap();
        let results = completed
            .canonical_document
            .general_sem_results
            .as_ref()
            .unwrap();
        let receipt = results.higher_order_inference_receipt.as_ref().unwrap();
        assert_eq!(receipt.target_ids.len(), 4);
        assert_eq!(
            results.higher_order_stages[1]
                .relation_estimates
                .iter()
                .filter(|relation| relation.value.standard_error.is_some())
                .count(),
            receipt.target_ids.len()
        );
        let archive_document = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&completed.canonical_document).unwrap(),
        )
        .unwrap();

        let mut table_tamper = serde_json::to_value(&archive_document).unwrap();
        let target_table = table_tamper["tables"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|table| table["id"] == "general_sem_higher_order_targets")
            .unwrap();
        let estimate = target_table["rows"][0]["cells"][5]["value"]
            .as_f64()
            .unwrap();
        target_table["rows"][0]["cells"][5]["value"] = serde_json::json!(estimate + 0.1);
        let table_tamper = serde_json::from_value(table_tamper).unwrap();
        assert!(matches!(
            append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: table_tamper,
                }
            ),
            ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
        ));
        assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

        let mut receipt_tamper = archive_document.clone();
        receipt_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .higher_order_inference_receipt
            .as_mut()
            .unwrap()
            .target_identity_set_sha256 = "a".repeat(64);
        assert!(matches!(
            append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: receipt_tamper,
                }
            ),
            ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
        ));
        assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

        let append = append_internal_project_schema6_canonical_result_v2(
            ProjectSchema6ResultAppendRequestV1 {
                surface: INTERNAL_LABS_SURFACE.into(),
                experimental_labs_enabled: true,
                archive_path: published.request.archive_path.clone(),
                expected_source_sha256: published.request.expected_archive_sha256.clone(),
                recipe: None,
                canonical_document: archive_document.clone(),
            },
        );
        let append_receipt = match append {
            ProjectSchema6ResultAppendOutcomeV1::Ok { value } => value,
            ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic } => {
                panic!("HOC append blocked: {diagnostic:?}")
            }
        };
        let reopened =
            read_internal_project_schema6_canonical_results_v2(ProjectSchema6ResultReadRequestV1 {
                surface: INTERNAL_LABS_SURFACE.into(),
                experimental_labs_enabled: true,
                archive_path: published.request.archive_path.clone(),
                expected_source_sha256: append_receipt.updated_document_sha256,
            });
        let snapshot = match reopened {
            ProjectSchema6ResultReadOutcomeV1::Ok { value } => value,
            ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic } => {
                panic!("HOC reopen blocked: {diagnostic:?}")
            }
        };
        assert_eq!(snapshot.documents.len(), 1);
        assert_eq!(snapshot.documents[0].canonical_document, archive_document);
    }

    #[test]
    fn moderation_bootstrap_admission_requires_the_supplemental_cell_while_compilation_stays_point_primary()
     {
        let published = published_fixture_from(
            moderation_bootstrap_fixture(true),
            "general-sem-moderation-bootstrap-admission.qpls",
            general_sem_multiple_moderation_bootstrap_capability_cell_v1(),
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let artifact = validate_exact_capability_and_data(&published.request, &resolved).unwrap();
        assert_eq!(
            artifact.capability_cell(),
            &general_sem_multiple_moderation_point_capability_cell_v1()
        );
        assert_eq!(
            artifact.compiler_version(),
            qpls_core::GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
        );

        let mut point_cell_tamper = published.request.clone();
        point_cell_tamper.capability_cell =
            general_sem_multiple_moderation_point_capability_cell_v1();
        let failure =
            validate_exact_capability_and_data(&point_cell_tamper, &resolved).unwrap_err();
        assert_eq!(failure.code, "general_sem_pls.capability_cell_mismatch");
    }

    #[test]
    fn moderation_native_lifecycle_appends_reopens_and_preserves_exact_surfaces() {
        for (same_focal, fixture_index) in [(true, 0_u128), (false, 1_u128)] {
            let published = published_fixture_from(
                direct_only_moderation_fixture(same_focal),
                if same_focal {
                    "general-sem-lifecycle-same-focal.qpls"
                } else {
                    "general-sem-lifecycle-different-focal.qpls"
                },
                general_sem_multiple_moderation_point_capability_cell_v1(),
            );
            let archive_path = PathBuf::from(&published.request.archive_path);
            let before_bytes = fs::read(&archive_path).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(&before_bytes)),
                published.request.expected_archive_sha256
            );
            assert_eq!(
                load_project_archive_v6(&archive_path)
                    .unwrap()
                    .document
                    .canonical_result_documents
                    .len(),
                0
            );

            let resolved = resolve_archive_authority(&published.request).unwrap();
            let job_id = Uuid::from_u128(0x7330 + fixture_index);
            let cancellation = Arc::new(AtomicBool::new(false));
            let state = job_state(
                job_id,
                cancellation.clone(),
                resolved.archive_identity.clone(),
            );
            run_worker(
                job_id,
                published.request.clone(),
                resolved,
                cancellation,
                state.0.clone(),
                admission(job_id),
            );
            let completed = take_completed_result(job_id, &state).unwrap();
            assert_eq!(
                completed.archive_identity.archive_sha256,
                published.request.expected_archive_sha256
            );
            assert_moderation_result_surface_identity(&completed.canonical_document);
            let archive_document =
                serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
                    serde_json::to_value(&completed.canonical_document).unwrap(),
                )
                .unwrap();
            let expected_document_json =
                canonical_result_document_v2_json(&archive_document).unwrap();
            let expected_document_sha256 = format!("{:x}", Sha256::digest(&expected_document_json));

            let mut stale_method = archive_document.clone();
            stale_method.provenance.method_version = "stale_moderation_method".into();
            let stale_method_append = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: stale_method,
                },
            );
            assert!(matches!(
                stale_method_append,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic }
                    if diagnostic.code
                        == "schema6_result_append.general_sem_method_identity_mismatch"
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let mut mismatched_value = serde_json::to_value(&archive_document).unwrap();
            let structural_rows = mismatched_value["tables"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|table| table["id"] == "structural_paths")
                .unwrap()["rows"]
                .as_array_mut()
                .unwrap();
            let estimate = structural_rows[0]["cells"][4]["value"].as_f64().unwrap();
            structural_rows[0]["cells"][4]["value"] = serde_json::json!(estimate + 0.125);
            let mismatched_document =
                serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(mismatched_value)
                    .unwrap();
            let mismatched_append = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: mismatched_document,
                },
            );
            assert!(matches!(
                mismatched_append,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let focal_relation_id = archive_document
                .general_sem_results
                .as_ref()
                .unwrap()
                .interaction_effects[0]
                .focal_relation_id
                .clone();
            let mut coherent_focal_tamper = serde_json::to_value(&archive_document).unwrap();
            let ledger =
                coherent_focal_tamper["general_sem_results"]["joint_stage_structural_coefficients"]
                    .as_array_mut()
                    .unwrap();
            let focal_ledger_entry = ledger
                .iter_mut()
                .find(|entry| entry["relation_id"] == focal_relation_id)
                .unwrap();
            let focal_estimate = focal_ledger_entry["estimate"]["estimate"].as_f64().unwrap();
            focal_ledger_entry["estimate"]["estimate"] = serde_json::json!(focal_estimate + 0.125);
            let focal_structural_rows = coherent_focal_tamper["tables"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|table| table["id"] == "structural_paths")
                .unwrap()["rows"]
                .as_array_mut()
                .unwrap();
            let focal_row = focal_structural_rows
                .iter_mut()
                .find(|row| row["cells"][0]["value"] == focal_relation_id)
                .unwrap();
            focal_row["cells"][4]["value"] = serde_json::json!(focal_estimate + 0.125);
            let coherent_focal_document = serde_json::from_value::<
                qpls_project::CanonicalResultDocumentV2,
            >(coherent_focal_tamper)
            .unwrap();
            let coherent_focal_append = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: coherent_focal_document,
                },
            );
            assert!(matches!(
                coherent_focal_append,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let append = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: archive_document.clone(),
                },
            );
            let receipt = match append {
                ProjectSchema6ResultAppendOutcomeV1::Ok { value } => value,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic } => {
                    panic!("moderation append blocked: {diagnostic:?}")
                }
            };
            assert_eq!(
                receipt.source_document_sha256,
                published.request.expected_archive_sha256
            );
            assert_ne!(
                receipt.updated_document_sha256,
                receipt.source_document_sha256
            );
            assert_eq!(receipt.canonical_result_document_count, 1);
            assert!(receipt.source_verified_at_commit);
            assert!(receipt.post_write_validated);
            assert!(receipt.rollback_copy_removed);
            let after_bytes = fs::read(&archive_path).unwrap();
            assert_ne!(after_bytes, before_bytes);
            assert_eq!(
                format!("{:x}", Sha256::digest(&after_bytes)),
                receipt.updated_document_sha256
            );

            let reopened = read_internal_project_schema6_canonical_results_v2(
                ProjectSchema6ResultReadRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: receipt.updated_document_sha256.clone(),
                },
            );
            let snapshot = match reopened {
                ProjectSchema6ResultReadOutcomeV1::Ok { value } => value,
                ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic } => {
                    panic!("moderation reopen blocked: {diagnostic:?}")
                }
            };
            assert_eq!(
                snapshot.source_document_sha256,
                receipt.updated_document_sha256
            );
            assert_eq!(snapshot.canonical_result_document_count, 1);
            assert!(snapshot.source_rechecked_unchanged);
            assert_eq!(snapshot.documents.len(), 1);
            let reopened_result = &snapshot.documents[0];
            assert!(reopened_result.immutable);
            assert_eq!(reopened_result.document_id, receipt.canonical_document_id);
            assert_eq!(reopened_result.run_id, receipt.run_id);
            assert_eq!(
                reopened_result.canonical_document_sha256,
                expected_document_sha256
            );
            assert_eq!(
                reopened_result.canonical_document_json.as_bytes(),
                expected_document_json
            );
            assert_eq!(reopened_result.canonical_document, archive_document);
            let reopened_core = serde_json::from_value::<qpls_core::CanonicalResultDocumentV2>(
                serde_json::to_value(&reopened_result.canonical_document).unwrap(),
            )
            .unwrap();
            assert_eq!(reopened_core, completed.canonical_document);
            assert_moderation_result_surface_identity(&reopened_core);
            assert_eq!(
                load_project_archive_v6(&archive_path)
                    .unwrap()
                    .document
                    .canonical_result_documents
                    .len(),
                1
            );
        }
    }

    #[test]
    fn moderation_bootstrap_native_lifecycle_reconciles_appends_and_strictly_reopens() {
        for (same_focal, fixture_index) in [(true, 0_u128), (false, 1_u128)] {
            let published = published_fixture_from(
                moderation_bootstrap_fixture(same_focal),
                &format!("general-sem-moderation-bootstrap-lifecycle-{fixture_index}.qpls"),
                general_sem_multiple_moderation_bootstrap_capability_cell_v1(),
            );
            let archive_path = PathBuf::from(&published.request.archive_path);
            let before_bytes = fs::read(&archive_path).unwrap();
            let resolved = resolve_archive_authority(&published.request).unwrap();
            let job_id = Uuid::from_u128(0x7360 + fixture_index);
            let cancellation = Arc::new(AtomicBool::new(false));
            let state = job_state(
                job_id,
                cancellation.clone(),
                resolved.archive_identity.clone(),
            );
            run_worker(
                job_id,
                published.request.clone(),
                resolved,
                cancellation,
                state.0.clone(),
                admission(job_id),
            );
            let completed = take_completed_result(job_id, &state).unwrap();
            assert_moderation_bootstrap_surface_identity(&completed);
            let archive_document =
                serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
                    serde_json::to_value(&completed.canonical_document).unwrap(),
                )
                .unwrap();
            let expected_document_json =
                canonical_result_document_v2_json(&archive_document).unwrap();

            let mut stale_method = archive_document.clone();
            stale_method.provenance.method_version = "stale_moderation_bootstrap_method".into();
            let stale_method_outcome = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: stale_method,
                },
            );
            assert!(matches!(
                stale_method_outcome,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let mut capability_tamper = archive_document.clone();
            capability_tamper
                .general_sem_results
                .as_mut()
                .unwrap()
                .inference_receipt
                .as_mut()
                .unwrap()
                .capability_cell = general_sem_multiple_moderation_point_capability_cell_v1();
            let capability_outcome = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: capability_tamper,
                },
            );
            assert!(matches!(
                capability_outcome,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let mut table_tamper = serde_json::to_value(&archive_document).unwrap();
            let gamma_row = table_tamper["tables"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|table| table["id"] == "general_sem_moderation_gamma_inference")
                .unwrap()["rows"][0]["cells"]
                .as_array_mut()
                .unwrap();
            let point_gamma = gamma_row[12]["value"].as_f64().unwrap();
            gamma_row[12]["value"] = serde_json::json!(point_gamma + 0.125);
            let table_tamper =
                serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(table_tamper)
                    .unwrap();
            let table_outcome = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: table_tamper,
                },
            );
            assert!(matches!(
                table_outcome,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { .. }
            ));
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);

            let append = append_internal_project_schema6_canonical_result_v2(
                ProjectSchema6ResultAppendRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: published.request.expected_archive_sha256.clone(),
                    recipe: None,
                    canonical_document: archive_document.clone(),
                },
            );
            let append_receipt = match append {
                ProjectSchema6ResultAppendOutcomeV1::Ok { value } => value,
                ProjectSchema6ResultAppendOutcomeV1::Blocked { diagnostic } => {
                    panic!("moderation bootstrap append blocked: {diagnostic:?}")
                }
            };
            assert_eq!(append_receipt.canonical_result_document_count, 1);
            assert!(append_receipt.post_write_validated);

            let reopened = read_internal_project_schema6_canonical_results_v2(
                ProjectSchema6ResultReadRequestV1 {
                    surface: INTERNAL_LABS_SURFACE.into(),
                    experimental_labs_enabled: true,
                    archive_path: published.request.archive_path.clone(),
                    expected_source_sha256: append_receipt.updated_document_sha256.clone(),
                },
            );
            let snapshot = match reopened {
                ProjectSchema6ResultReadOutcomeV1::Ok { value } => value,
                ProjectSchema6ResultReadOutcomeV1::Blocked { diagnostic } => {
                    panic!("moderation bootstrap reopen blocked: {diagnostic:?}")
                }
            };
            assert_eq!(snapshot.documents.len(), 1);
            assert_eq!(
                snapshot.documents[0].canonical_document_json.as_bytes(),
                expected_document_json
            );
            assert_eq!(snapshot.documents[0].canonical_document, archive_document);
            let reopened_core = serde_json::from_value::<qpls_core::CanonicalResultDocumentV2>(
                serde_json::to_value(&snapshot.documents[0].canonical_document).unwrap(),
            )
            .unwrap();
            assert_eq!(reopened_core, completed.canonical_document);
        }
    }

    #[test]
    fn moderation_capability_tamper_is_blocked_after_compiled_plan_classification() {
        let published = published_fixture_from(
            direct_only_moderation_fixture(true),
            "general-sem-moderation-capability-tamper.qpls",
            pls_general_recursive_effects_capability_cell_v1(),
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let failure =
            validate_exact_capability_and_data(&published.request, &resolved).unwrap_err();
        assert_eq!(failure.code, "general_sem_pls.capability_cell_mismatch");
        assert_eq!(failure.subject, "capabilityCell");
    }

    #[test]
    fn cancelled_moderation_job_never_publishes_a_partial_result() {
        let published = published_fixture_from(
            direct_only_moderation_fixture(false),
            "general-sem-cancelled-moderation.qpls",
            general_sem_multiple_moderation_point_capability_cell_v1(),
        );
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0x7312);
        let cancellation = Arc::new(AtomicBool::new(true));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker(
            job_id,
            published.request,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );
        let jobs = state.0.lock().unwrap();
        let job = jobs.get(&job_id).unwrap();
        assert_eq!(
            job.snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Cancelled
        );
        assert!(job.result.is_none());
    }

    #[test]
    fn moderation_cancellation_checkpoints_never_publish_or_mutate_the_archive() {
        for (checkpoint, checkpoint_index) in [
            (
                GeneralSemPlsWorkerCheckpointV1::DuringInteractionExecution,
                0_u128,
            ),
            (
                GeneralSemPlsWorkerCheckpointV1::AfterExecutionBeforeCanonicalization,
                1_u128,
            ),
            (
                GeneralSemPlsWorkerCheckpointV1::AfterCanonicalizationBeforePublication,
                2_u128,
            ),
        ] {
            let published = published_fixture_from(
                direct_only_moderation_fixture(false),
                &format!("general-sem-cancel-checkpoint-{checkpoint_index}.qpls"),
                general_sem_multiple_moderation_point_capability_cell_v1(),
            );
            let archive_path = PathBuf::from(&published.request.archive_path);
            let before_bytes = fs::read(&archive_path).unwrap();
            let before_sha256 = format!("{:x}", Sha256::digest(&before_bytes));
            let before_result_count = load_project_archive_v6(&archive_path)
                .unwrap()
                .document
                .canonical_result_documents
                .len();
            let resolved = resolve_archive_authority(&published.request).unwrap();
            let job_id = Uuid::from_u128(0x7340 + checkpoint_index);
            let cancellation = Arc::new(AtomicBool::new(false));
            let checkpoint_seen = Arc::new(AtomicBool::new(false));
            let cancellation_for_hook = cancellation.clone();
            let checkpoint_seen_by_hook = checkpoint_seen.clone();
            let hook: GeneralSemPlsWorkerCheckpointHookV1 = Arc::new(move |observed| {
                if observed == checkpoint {
                    checkpoint_seen_by_hook.store(true, Ordering::Release);
                    cancellation_for_hook.store(true, Ordering::Release);
                }
            });
            let state = job_state(
                job_id,
                cancellation.clone(),
                resolved.archive_identity.clone(),
            );
            run_worker_with_checkpoint_hook(
                job_id,
                published.request,
                resolved,
                cancellation,
                state.0.clone(),
                admission(job_id),
                Some(hook),
            );

            assert!(checkpoint_seen.load(Ordering::Acquire));
            let jobs = state.0.lock().unwrap();
            let job = jobs.get(&job_id).unwrap();
            assert_eq!(
                job.snapshot.state,
                InternalLabsGeneralSemPlsJobStateV1::Cancelled
            );
            assert!(job.snapshot.completed_at.is_some());
            assert!(job.result.is_none());
            drop(jobs);
            let unavailable = take_completed_result(job_id, &state).unwrap_err();
            assert_eq!(unavailable.code, "general_sem_pls.result_not_available");

            let after_bytes = fs::read(&archive_path).unwrap();
            assert_eq!(after_bytes, before_bytes);
            assert_eq!(format!("{:x}", Sha256::digest(&after_bytes)), before_sha256);
            assert_eq!(
                load_project_archive_v6(&archive_path)
                    .unwrap()
                    .document
                    .canonical_result_documents
                    .len(),
                before_result_count
            );
        }
    }

    #[test]
    fn moderation_bootstrap_mid_and_late_cancellation_never_publish_or_mutate_the_archive() {
        for (checkpoint, checkpoint_index) in [
            (
                GeneralSemPlsWorkerCheckpointV1::DuringModerationBootstrapMid,
                0_u128,
            ),
            (
                GeneralSemPlsWorkerCheckpointV1::DuringModerationBootstrapLate,
                1_u128,
            ),
        ] {
            let published = published_fixture_from(
                moderation_bootstrap_fixture(false),
                &format!("general-sem-bootstrap-cancel-{checkpoint_index}.qpls"),
                general_sem_multiple_moderation_bootstrap_capability_cell_v1(),
            );
            let archive_path = PathBuf::from(&published.request.archive_path);
            let before_bytes = fs::read(&archive_path).unwrap();
            let before_result_count = load_project_archive_v6(&archive_path)
                .unwrap()
                .document
                .canonical_result_documents
                .len();
            let resolved = resolve_archive_authority(&published.request).unwrap();
            let job_id = Uuid::from_u128(0x7370 + checkpoint_index);
            let cancellation = Arc::new(AtomicBool::new(false));
            let checkpoint_seen = Arc::new(AtomicBool::new(false));
            let cancellation_for_hook = cancellation.clone();
            let checkpoint_seen_by_hook = checkpoint_seen.clone();
            let hook: GeneralSemPlsWorkerCheckpointHookV1 = Arc::new(move |observed| {
                if observed == checkpoint {
                    checkpoint_seen_by_hook.store(true, Ordering::Release);
                    cancellation_for_hook.store(true, Ordering::Release);
                }
            });
            let state = job_state(
                job_id,
                cancellation.clone(),
                resolved.archive_identity.clone(),
            );
            run_worker_with_checkpoint_hook(
                job_id,
                published.request,
                resolved,
                cancellation,
                state.0.clone(),
                admission(job_id),
                Some(hook),
            );

            assert!(checkpoint_seen.load(Ordering::Acquire));
            let jobs = state.0.lock().unwrap();
            let job = jobs.get(&job_id).unwrap();
            assert_eq!(
                job.snapshot.state,
                InternalLabsGeneralSemPlsJobStateV1::Cancelled
            );
            assert!(job.result.is_none());
            drop(jobs);
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);
            assert_eq!(
                load_project_archive_v6(&archive_path)
                    .unwrap()
                    .document
                    .canonical_result_documents
                    .len(),
                before_result_count
            );
        }
    }

    #[test]
    fn disjoint_hoc_bootstrap_mid_and_late_cancellation_never_publishes_or_mutates_archive() {
        for (checkpoint, checkpoint_index) in [
            (
                GeneralSemPlsWorkerCheckpointV1::DuringHocBootstrapMid,
                0_u128,
            ),
            (
                GeneralSemPlsWorkerCheckpointV1::DuringHocBootstrapLate,
                1_u128,
            ),
        ] {
            let published = published_fixture_from(
                disjoint_hoc_fixture(HigherOrderMeasurementTypeV4::ReflectiveReflective, true),
                &format!("general-sem-hoc-bootstrap-cancel-{checkpoint_index}.qpls"),
                pls_general_higher_order_bootstrap_capability_cell_v1(),
            );
            let archive_path = PathBuf::from(&published.request.archive_path);
            let before_bytes = fs::read(&archive_path).unwrap();
            let before_result_count = load_project_archive_v6(&archive_path)
                .unwrap()
                .document
                .canonical_result_documents
                .len();
            let resolved = resolve_archive_authority(&published.request).unwrap();
            let job_id = Uuid::from_u128(0x73c0 + checkpoint_index);
            let cancellation = Arc::new(AtomicBool::new(false));
            let checkpoint_seen = Arc::new(AtomicBool::new(false));
            let cancellation_for_hook = cancellation.clone();
            let checkpoint_seen_by_hook = checkpoint_seen.clone();
            let hook: GeneralSemPlsWorkerCheckpointHookV1 = Arc::new(move |observed| {
                if observed == checkpoint {
                    checkpoint_seen_by_hook.store(true, Ordering::Release);
                    cancellation_for_hook.store(true, Ordering::Release);
                }
            });
            let state = job_state(
                job_id,
                cancellation.clone(),
                resolved.archive_identity.clone(),
            );
            run_worker_with_checkpoint_hook(
                job_id,
                published.request,
                resolved,
                cancellation,
                state.0.clone(),
                admission(job_id),
                Some(hook),
            );

            assert!(checkpoint_seen.load(Ordering::Acquire));
            let jobs = state.0.lock().unwrap();
            let job = jobs.get(&job_id).unwrap();
            assert_eq!(
                job.snapshot.state,
                InternalLabsGeneralSemPlsJobStateV1::Cancelled
            );
            assert!(job.result.is_none());
            drop(jobs);
            assert_eq!(fs::read(&archive_path).unwrap(), before_bytes);
            assert_eq!(
                load_project_archive_v6(&archive_path)
                    .unwrap()
                    .document
                    .canonical_result_documents
                    .len(),
                before_result_count
            );
        }
    }

    #[test]
    fn listwise_native_predicate_accepts_nullable_continuous_indicator_columns() {
        let fixture = general_sem_native_fixture_v1();
        let dataset = import_delimited_bytes(
            b"x1,x2,m11,m12,m21,m22,y1,y2\n1,2,2,1,1,3,2,1\n2,,3,2,2,2,3,2\n3,4,4,3,4,3,5,4\n4,3,5,5,3,5,6,5\n5,6,7,6,6,7,8,7\n",
            "native-general-sem-listwise.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        assert!(dataset.batch.column(1).null_count() > 0);
        let mut model = fixture.model;
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!();
        };
        *dataset_id = dataset.id.to_string();

        validate_dataset_predicate(&dataset, &model).unwrap();
    }

    #[test]
    fn cancellation_is_terminal_and_never_publishes_a_result() {
        let published = published_fixture();
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0x7301);
        let cancellation = Arc::new(AtomicBool::new(true));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );

        run_worker(
            job_id,
            published.request,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );

        let jobs = state.0.lock().unwrap();
        let job = jobs.get(&job_id).unwrap();
        assert_eq!(
            job.snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Cancelled
        );
        assert!(job.snapshot.completed_at.is_some());
        assert!(job.result.is_none());
        drop(jobs);
        let failure = take_completed_result(job_id, &state).unwrap_err();
        assert_eq!(failure.code, "general_sem_pls.result_not_available");
    }

    #[test]
    fn completed_point_result_is_consumed_exactly_once() {
        let published = published_fixture();
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0x7302);
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = job_state(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );

        run_worker(
            job_id,
            published.request,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
        );

        assert_eq!(
            state.0.lock().unwrap().get(&job_id).unwrap().snapshot.state,
            InternalLabsGeneralSemPlsJobStateV1::Completed
        );
        let completed = take_completed_result(job_id, &state).unwrap();
        let point = pls_general_recursive_effects_capability_cell_v1();
        assert_eq!(
            completed.canonical_document.provenance.capability_cell,
            point
        );
        assert!(
            completed
                .canonical_document
                .capability_cells
                .as_ref()
                .is_some_and(|cells| cells.contains(&point))
        );
        assert!(state.0.lock().unwrap().get(&job_id).is_none());

        let failure = take_completed_result(job_id, &state).unwrap_err();
        assert_eq!(failure.code, "general_sem_pls.unknown_job");
    }
}
