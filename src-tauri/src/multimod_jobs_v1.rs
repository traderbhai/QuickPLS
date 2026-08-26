//! Strict archive-bound native lifecycle for the additive MultiMod suite.
//!
//! Only runner profiles reported as `built_in_from_dataset` may enter a
//! worker. A completed result is published atomically with its staged Recipe
//! V4 and Arrow evidence, then freshly reopened before it becomes visible.

use crate::{
    DesktopJobs,
    multimod_candidate_authority_v1::{
        embedded_multimod_cache_authority_sha256_v1, multimod_standard_surface_authorized_v1,
        promote_completed_multimod_result_v1,
    },
    recipe_v4_jobs::{
        DesktopRecipeV4Jobs, PlsModelComparisonAdmissionReservationV1,
        reserve_multimod_admission_v1,
    },
};
use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use chrono::{DateTime, SecondsFormat, Utc};
use qpls_core::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, CompiledMultiModRecipeV1,
    ConditionalRawProbeFitMetricReceiptV2, GeneralSemConditionalProcessConfigV2,
    InterventionalCausalMediationConfigV1, MULTIMOD_SIDECAR_MAX_BYTES_V1, MgaModelProfileV1,
    MgaMultigroupV1, MultiModAnalysisResultV1, MultiModCompilerTargetV1,
    MultimodQualificationStateV1, PlsUnobservedHeterogeneityConfigV2, SemModelV4, SemVariableV4,
    TypedGroupValueV1 as CoreTypedGroupValueV1, compile_multimod_recipe_v1, sha256_serialized,
};
use qpls_data::{DataKind, Dataset, ScaleType};
use qpls_estimation::{
    FrequencyMultigroupDesignV1, FrequencySelectedGroupRowV1, GroupIdentityV1, GroupIndexV1,
    MultigroupDesignV1, SelectedGroupRowV1, TypedGroupValueV1 as EstimationTypedGroupValueV1,
    assess_frequency_multigroup_design_v1, assess_multigroup_design_v1,
    multimod_frequency_counts_for_source_rows_v1,
};
use qpls_project::{
    CanonicalResultDocumentV2, MultiModArchiveAppendReceiptV1, MultiModResultAttachmentV1,
    MultiModSidecarPayloadV1, ProjectArchiveDocumentV6, ProjectModelPayloadV6,
    append_multimod_recipe_result_and_canonical_to_archive_v6, load_project_archive_v6,
    multimod_result_identity_sha256_v1, validate_multimod_sidecar_payload_v1,
};
use qpls_runner::{
    ConditionalProcessRawAuthorityV2, MgaExecutionCacheV1, MgaExecutionPlanV1,
    MultiModCanonicalRunContextV1, MultiModRunOutputV1, MultiModRunnerErrorV1,
    MultiModRunnerProgressV1, MultiModRuntimeReadinessV1, MultiModRuntimeSupportV1,
    build_multimod_canonical_result_v2, multimod_mga_publishable_parameter_identities_v1,
    multimod_runtime_support_v1, prepare_compiled_raw_mga_execution_plan_v1,
    prepare_conditional_process_analysis_frame_v2,
    prepare_conditional_raw_probe_fit_metric_receipts_v2,
    run_compiled_general_sem_conditional_process_raw_output_v2,
    run_compiled_interventional_causal_mediation_raw_v1,
    run_compiled_raw_mga_resumable_with_checkpoint_v1, run_compiled_raw_pls_heterogeneity_v2,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

const MULTIMOD_JOB_SCHEMA_VERSION_V1: u32 = 1;
const MULTIMOD_CACHE_SCHEMA_VERSION_V1: u32 = 1;
const MULTIMOD_INTERNAL_LABS_SURFACE_V1: &str = "internal_labs_multimod_v1";
pub(crate) const MULTIMOD_STANDARD_SURFACE_V1: &str = "standard_multimod_v1";
const MULTIMOD_SIDECAR_WARNING_BYTES_V1: u64 = 128 * 1024 * 1024;
const MAXIMUM_RETAINED_MULTIMOD_JOBS_V1: usize = 255;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMgaEngineV1 {
    Ordinary,
    FrequencyWeighted,
    Interaction,
    MultipleHoc,
}

#[cfg(test)]
fn native_mga_engine_v1(profile: MgaModelProfileV1) -> NativeMgaEngineV1 {
    match profile {
        MgaModelProfileV1::GeneralSemPls
        | MgaModelProfileV1::CaseWeightedPls
        | MgaModelProfileV1::ReflectivePlsc => NativeMgaEngineV1::Ordinary,
        MgaModelProfileV1::FrequencyWeightedPls => NativeMgaEngineV1::FrequencyWeighted,
        MgaModelProfileV1::MultipleTwoWayModeration
        | MgaModelProfileV1::BoundedThreeWayModeration
        | MgaModelProfileV1::BoundedTwoWayModeratedMediation => NativeMgaEngineV1::Interaction,
        MgaModelProfileV1::MultipleNonnestedHoc => NativeMgaEngineV1::MultipleHoc,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "config",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum MultiModJobConfigV1 {
    MgaMultigroupV1(MgaMultigroupV1),
    PlsHeterogeneityV2(PlsUnobservedHeterogeneityConfigV2),
    GeneralSemConditionalProcessV2(GeneralSemConditionalProcessConfigV2),
    InterventionalCausalMediationV1(InterventionalCausalMediationConfigV1),
}

impl MultiModJobConfigV1 {
    fn target(&self) -> MultiModCompilerTargetV1 {
        match self {
            Self::MgaMultigroupV1(_) => MultiModCompilerTargetV1::MgaMultigroupV1,
            Self::PlsHeterogeneityV2(_) => MultiModCompilerTargetV1::PlsHeterogeneityV2,
            Self::GeneralSemConditionalProcessV2(_) => {
                MultiModCompilerTargetV1::GeneralSemConditionalProcessV2
            }
            Self::InterventionalCausalMediationV1(_) => {
                MultiModCompilerTargetV1::InterventionalCausalMediationV1
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModExternalCacheReceiptV1 {
    schema_version: u32,
    cache_id: Uuid,
    cache_directory: String,
    manifest_sha256: String,
    embedded_authority_sha256: String,
    source_archive_sha256: String,
    result_id: String,
    recipe_id: Uuid,
    target: MultiModCompilerTargetV1,
    stage: MultiModExternalCacheStageV1,
    created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MultiModExternalCacheStageV1 {
    MgaExecution,
    ArchiveReady,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModJobRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
    expected_archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    source_recipe_id: String,
    source_recipe_document_sha256: String,
    staged_recipe_id: String,
    staged_created_at: String,
    config: MultiModJobConfigV1,
    #[serde(default)]
    resume_cache: Option<MultiModExternalCacheReceiptV1>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModGroupingProfileRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
    expected_archive_sha256: String,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    source_recipe_id: String,
    source_recipe_document_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModRawProbeMetricRequestV2 {
    staged: MultiModJobRequestV1,
    moderator_id: String,
    orientation_sign: i8,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModGroupingProfileGroupV1 {
    group_id: String,
    label: String,
    value: CoreTypedGroupValueV1,
    selected_rows: usize,
    complete_cases: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModGroupingProfileColumnV1 {
    column: String,
    label: String,
    used_as_indicator: bool,
    groups: Vec<MultiModGroupingProfileGroupV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModGroupingProfileV1 {
    schema_version: u32,
    archive_sha256: String,
    dataset_fingerprint: String,
    columns: Vec<MultiModGroupingProfileColumnV1>,
    omitted_high_cardinality_columns: Vec<String>,
    source_rechecked_unchanged: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModPreflightV1 {
    schema_version: u32,
    target: MultiModCompilerTargetV1,
    capability_cell_id: String,
    readiness: MultiModRuntimeReadinessV1,
    stable_reason_codes: Vec<String>,
    staged_recipe_id: Uuid,
    staged_recipe_document_sha256: String,
    compilation_identity_sha256: String,
    mga_group_eligibility: Option<NativeMgaGroupEligibilityV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum NativeMgaCountBasisV1 {
    PhysicalCompleteRows,
    FrequencyExpandedCases,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeMgaGroupCountV1 {
    group_id: String,
    label: String,
    physical_complete_rows: usize,
    effective_complete_cases: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeMgaGroupEligibilityV1 {
    count_basis: NativeMgaCountBasisV1,
    groups: Vec<NativeMgaGroupCountV1>,
    maximum_imbalance_ratio: Option<f64>,
    eligible: bool,
    warning_codes: Vec<String>,
    blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MultiModJobStateV1 {
    Queued,
    Running,
    Cancelling,
    Publishing,
    Completed,
    Failed,
    Cancelled,
}

impl MultiModJobStateV1 {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    fn is_active(self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MultiModFailureStageV1 {
    Access,
    ArchiveAuthority,
    Compilation,
    RuntimeSupport,
    Estimation,
    Evidence,
    Cache,
    Publication,
    Reopen,
    Integrity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModJobFailureV1 {
    schema_version: u32,
    stage: MultiModFailureStageV1,
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModJobSnapshotV1 {
    schema_version: u32,
    job_id: Uuid,
    target: MultiModCompilerTargetV1,
    state: MultiModJobStateV1,
    phase: String,
    shard_id: String,
    completed_units: u64,
    total_units: u64,
    message: Option<String>,
    warning_codes: Vec<String>,
    failure: Option<MultiModJobFailureV1>,
    resume_cache: Option<MultiModExternalCacheReceiptV1>,
    queued_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

impl MultiModJobSnapshotV1 {
    fn queued(job_id: Uuid, target: MultiModCompilerTargetV1) -> Self {
        Self {
            schema_version: MULTIMOD_JOB_SCHEMA_VERSION_V1,
            job_id,
            target,
            state: MultiModJobStateV1::Queued,
            phase: "multimod_queued".into(),
            shard_id: "multimod:queued".into(),
            completed_units: 0,
            total_units: 1,
            message: None,
            warning_codes: Vec::new(),
            failure: None,
            resume_cache: None,
            queued_at: now_utc(),
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModCompletedResultV1 {
    schema_version: u32,
    job_id: Uuid,
    archive_path: String,
    archive_sha256: String,
    project_id: Uuid,
    dataset_id: Uuid,
    model_id: String,
    attachment: MultiModResultAttachmentV1,
    canonical_document: CanonicalResultDocumentV2,
    append_receipt: MultiModArchiveAppendReceiptV1,
    cache_receipt: MultiModExternalCacheReceiptV1,
    cache_removed_after_commit: bool,
}

struct MultiModJobV1 {
    snapshot: MultiModJobSnapshotV1,
    cancellation: Arc<AtomicBool>,
    result: Option<MultiModCompletedResultV1>,
}

#[derive(Clone, Default)]
pub(crate) struct DesktopMultiModJobsV1(Arc<Mutex<HashMap<Uuid, MultiModJobV1>>>);

struct ResolvedMultiModAuthorityV1 {
    archive_path: PathBuf,
    archive_sha256: String,
    project_id: Uuid,
    dataset: Dataset,
    model: SemModelV4,
    staged_recipe: AnalysisRecipeV4,
    artifact: CompiledMultiModRecipeV1,
    runtime_support: MultiModRuntimeSupportV1,
    mga_group_eligibility: Option<NativeMgaGroupEligibilityV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CachePayloadFileV1 {
    leaf_name: String,
    sha256: String,
    bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiModArchiveReadyCacheManifestV1 {
    schema_version: u32,
    cache_id: Uuid,
    source_archive_path: String,
    source_archive_sha256: String,
    embedded_authority_sha256: String,
    staged_recipe: AnalysisRecipeV4,
    attachment: MultiModResultAttachmentV1,
    canonical_document: CanonicalResultDocumentV2,
    payload_files: Vec<CachePayloadFileV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    superseded_execution_cache: Option<MultiModExternalCacheReceiptV1>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiModMgaExecutionCacheManifestV1 {
    schema_version: u32,
    cache_id: Uuid,
    source_archive_path: String,
    source_archive_sha256: String,
    embedded_authority_sha256: String,
    recipe_id: Uuid,
    staged_recipe_sha256: String,
    result_id: String,
    target: MultiModCompilerTargetV1,
    plan: MgaExecutionPlanV1,
    cache: MgaExecutionCacheV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    finalized_cache_sha256: Option<String>,
    created_at: String,
    updated_at: String,
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn failure(
    stage: MultiModFailureStageV1,
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> MultiModJobFailureV1 {
    MultiModJobFailureV1 {
        schema_version: MULTIMOD_JOB_SCHEMA_VERSION_V1,
        stage,
        code: code.into(),
        message: message.into(),
        corrective_action: corrective_action.into(),
    }
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn current_cache_authority_sha256_v1() -> Result<String, MultiModJobFailureV1> {
    embedded_multimod_cache_authority_sha256_v1().map_err(|error| {
        failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.embedded_authority_invalid",
            error,
            "Do not resume, publish, or export MultiMod results from this executable.",
        )
    })
}

fn metadata_is_reparse_v1(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn sha256_file(path: &Path) -> Result<String, MultiModJobFailureV1> {
    let mut file = File::open(path).map_err(|_| {
        failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.archive_unavailable",
            "The exact local .qpls archive is unavailable.",
            "Reopen the strict schema-6 project and retry.",
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| {
            failure(
                MultiModFailureStageV1::ArchiveAuthority,
                "multimod.job.archive_read_failed",
                "The project archive could not be read completely.",
                "Wait for other local file operations to finish and retry.",
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn validate_multimod_runtime_surface_access_using_v1(
    surface: &str,
    experimental_labs_enabled: bool,
    standard_authorized: impl FnOnce() -> Result<bool, String>,
) -> Result<bool, MultiModJobFailureV1> {
    match surface {
        MULTIMOD_INTERNAL_LABS_SURFACE_V1 if experimental_labs_enabled => Ok(false),
        MULTIMOD_INTERNAL_LABS_SURFACE_V1 => Err(failure(
            MultiModFailureStageV1::Access,
            "multimod.job.internal_labs_required",
            "MultiMod Labs execution requires explicit Experimental Labs opt-in.",
            "Enable Experimental Labs and rerun exact native preflight.",
        )),
        MULTIMOD_STANDARD_SURFACE_V1 if experimental_labs_enabled => Err(failure(
            MultiModFailureStageV1::Access,
            "multimod.job.surface_invalid",
            "The Standard MultiMod surface requires Experimental Labs to be disabled.",
            "Disable Experimental Labs and retry through standard_multimod_v1.",
        )),
        MULTIMOD_STANDARD_SURFACE_V1 => match standard_authorized() {
            Ok(true) => Ok(true),
            Ok(false) => Err(failure(
                MultiModFailureStageV1::Access,
                "multimod.job.standard_authority_required",
                "Standard MultiMod execution requires a release-qualified immutable authority embedded in this executable.",
                "Use the exact release-qualified candidate package, or run this request through explicit Experimental Labs.",
            )),
            Err(error) => Err(failure(
                MultiModFailureStageV1::Integrity,
                "multimod.job.embedded_authority_invalid",
                error,
                "Do not execute, resume, publish, or export MultiMod results from this executable.",
            )),
        },
        _ => Err(failure(
            MultiModFailureStageV1::Access,
            "multimod.job.surface_invalid",
            "The requested MultiMod runtime surface is unsupported.",
            "Use standard_multimod_v1 for a release-qualified candidate or internal_labs_multimod_v1 with explicit Labs opt-in.",
        )),
    }
}

fn validate_multimod_runtime_surface_access_v1(
    surface: &str,
    experimental_labs_enabled: bool,
) -> Result<bool, MultiModJobFailureV1> {
    validate_multimod_runtime_surface_access_using_v1(
        surface,
        experimental_labs_enabled,
        multimod_standard_surface_authorized_v1,
    )
}

fn validate_request_access(request: &MultiModJobRequestV1) -> Result<(), MultiModJobFailureV1> {
    validate_multimod_runtime_surface_access_v1(
        &request.surface,
        request.experimental_labs_enabled,
    )?;
    let path = Path::new(&request.archive_path);
    if request.archive_path.trim() != request.archive_path
        || !path.is_absolute()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("qpls"))
    {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.absolute_qpls_path_required",
            "MultiMod requires an exact absolute local .qpls archive path.",
            "Select the active schema-6 calculation project and retry.",
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.archive_unavailable",
            "The requested archive is unavailable.",
            "Reopen the exact schema-6 project and retry.",
        )
    })?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x0000_0400 != 0
    };
    #[cfg(not(windows))]
    let reparse = metadata.file_type().is_symlink();
    if !metadata.file_type().is_file() || reparse {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.regular_archive_required",
            "The archive must be a regular local file, not a link or reparse point.",
            "Select the exact regular .qpls project file.",
        ));
    }
    for digest in [
        &request.expected_archive_sha256,
        &request.dataset_fingerprint,
        &request.model_scientific_sha256,
        &request.source_recipe_document_sha256,
    ] {
        if !lowercase_sha256(digest) {
            return Err(failure(
                MultiModFailureStageV1::ArchiveAuthority,
                "multimod.job.invalid_digest",
                "A requested archive authority digest is not a lowercase SHA-256 value.",
                "Rebuild the request from the strict native archive snapshot.",
            ));
        }
    }
    Ok(())
}

fn parse_uuid(value: &str, subject: &str) -> Result<Uuid, MultiModJobFailureV1> {
    Uuid::parse_str(value)
        .ok()
        .filter(|value| !value.is_nil())
        .ok_or_else(|| {
            failure(
                MultiModFailureStageV1::ArchiveAuthority,
                "multimod.job.invalid_uuid",
                format!("{subject} must be a non-nil UUID."),
                "Rebuild the request from the strict native archive snapshot.",
            )
        })
}

fn stage_recipe_v1(
    source: &AnalysisRecipeV4,
    request: &MultiModJobRequestV1,
) -> Result<AnalysisRecipeV4, MultiModJobFailureV1> {
    let staged_id = parse_uuid(&request.staged_recipe_id, "stagedRecipeId")?;
    let created_at = DateTime::parse_from_rfc3339(&request.staged_created_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| {
            failure(
                MultiModFailureStageV1::Compilation,
                "multimod.job.invalid_staged_timestamp",
                "The staged Recipe V4 timestamp is not RFC 3339.",
                "Create a new staged request with an exact UTC timestamp.",
            )
        })?;
    let mut recipe = source.clone();
    recipe.id = staged_id;
    recipe.created_at = created_at;
    recipe.method_config = None;
    recipe.mga_multigroup = None;
    recipe.pls_heterogeneity = None;
    recipe.general_sem_conditional_process = None;
    recipe.interventional_causal_mediation = None;
    match &request.config {
        MultiModJobConfigV1::MgaMultigroupV1(config) => {
            recipe.settings.method = AnalysisMethod::Mga;
            recipe.settings.permutation_samples = config.permutation_samples;
            recipe.settings.bootstrap_samples = config.bootstrap_samples;
            recipe.mga_multigroup = Some(config.clone());
        }
        MultiModJobConfigV1::PlsHeterogeneityV2(config) => {
            recipe.settings.method = AnalysisMethod::Predict;
            recipe.settings.permutation_samples = 0;
            recipe.settings.bootstrap_samples =
                config.bootstrap.as_ref().map_or(0, |value| value.resamples);
            recipe.pls_heterogeneity = Some(config.clone());
        }
        MultiModJobConfigV1::GeneralSemConditionalProcessV2(config) => {
            recipe.settings.method = AnalysisMethod::ModeratedMediation;
            recipe.settings.permutation_samples = 0;
            recipe.settings.bootstrap_samples = config.inference.outer_resamples;
            recipe.general_sem_conditional_process = Some(config.clone());
        }
        MultiModJobConfigV1::InterventionalCausalMediationV1(config) => {
            recipe.settings.method = AnalysisMethod::Regression;
            recipe.settings.permutation_samples = 0;
            recipe.settings.bootstrap_samples = config.bootstrap_resamples;
            recipe.interventional_causal_mediation = Some(config.clone());
        }
    }
    persist_multimod_execution_surface_v1(&mut recipe.metadata, request.surface.clone());
    recipe
        .metadata
        .insert("multimod_generation".into(), "multimod_v1".into());
    recipe.metadata.insert(
        "multimod_source_recipe_id".into(),
        request.source_recipe_id.clone(),
    );
    recipe.ensure_valid().map_err(|error| {
        failure(
            MultiModFailureStageV1::Compilation,
            "multimod.job.staged_recipe_invalid",
            format!("The staged Recipe V4 is invalid: {error}"),
            "Correct the exact Labs configuration without simplifying it and stage a new recipe.",
        )
    })?;
    Ok(recipe)
}

fn persist_multimod_execution_surface_v1(
    metadata: &mut BTreeMap<String, String>,
    request_surface: String,
) {
    metadata.insert("execution_surface".into(), request_surface);
}

fn validate_heterogeneity_discovery_lock_source_v2(
    document: &ProjectArchiveDocumentV6,
    staged_recipe: &AnalysisRecipeV4,
) -> Result<(), MultiModJobFailureV1> {
    let Some(config) = staged_recipe.pls_heterogeneity.as_ref() else {
        return Ok(());
    };
    let qpls_core::HeterogeneityPhaseV2::Inference { lock } = &config.phase else {
        return Ok(());
    };
    let source = document.multimod_results.iter().find_map(|attachment| {
        let MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(result) = &attachment.result
        else {
            return None;
        };
        (result.discovery_result_identity_sha256 == lock.discovery_result_identity_sha256
            && result.profile == config.profile
            && result.inference_lock.is_none()
            && result.locked_algorithm.is_none()
            && result.locked_k.is_none()
            && result.bootstrap_ledger.is_none())
        .then_some(result)
    });
    let Some(source) = source else {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.heterogeneity_discovery_lock_source_absent",
            "The inference lock does not bind an unlocked completed discovery result in this strict archive.",
            "Complete and strictly reopen discovery, then select an algorithm and K from that exact candidate table.",
        ));
    };
    let actual_inventory = source
        .candidates
        .iter()
        .filter_map(|candidate| match candidate.method {
            qpls_core::HeterogeneityCandidateMethodV2::Segmentation { algorithm } => {
                Some((algorithm, candidate.k))
            }
            qpls_core::HeterogeneityCandidateMethodV2::PooledBaselineV1 => None,
        })
        .collect::<BTreeSet<_>>();
    let expected_inventory = lock
        .discovery_algorithms
        .iter()
        .flat_map(|algorithm| {
            lock.discovery_candidate_k
                .iter()
                .map(move |k| (*algorithm, *k))
        })
        .collect::<BTreeSet<_>>();
    let selected_is_stable = source.candidates.iter().any(|candidate| {
        candidate.method
            == (qpls_core::HeterogeneityCandidateMethodV2::Segmentation {
                algorithm: lock.selected_algorithm,
            })
            && candidate.k == lock.selected_k
            && candidate.state == qpls_core::HeterogeneityCandidateStateV2::ConvergedStable
    });
    if actual_inventory != expected_inventory || !selected_is_stable {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.heterogeneity_discovery_lock_inventory_mismatch",
            "The persisted discovery candidate table differs from the lock receipt or the selected candidate is not converged stable.",
            "Return to the completed discovery result and lock only one stable row from its exact algorithm-by-K inventory.",
        ));
    }
    Ok(())
}

fn resolve_authority_v1(
    request: &MultiModJobRequestV1,
) -> Result<ResolvedMultiModAuthorityV1, MultiModJobFailureV1> {
    validate_request_access(request)?;
    let archive_path = PathBuf::from(&request.archive_path);
    let observed_sha256 = sha256_file(&archive_path)?;
    if observed_sha256 != request.expected_archive_sha256 {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.archive_changed",
            "The archive digest changed before MultiMod preflight.",
            "Strictly reopen the current archive and build a fresh request.",
        ));
    }
    let loaded = load_project_archive_v6(&archive_path).map_err(|error| {
        failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.strict_archive_reopen_failed",
            format!("The schema-6 archive failed strict reopen: {error}"),
            "Restore the archive from a trusted source; do not execute or display its results.",
        )
    })?;
    if sha256_file(&archive_path)? != observed_sha256 {
        return Err(failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.job.archive_changed_during_read",
            "The archive changed while its MultiMod authority was being resolved.",
            "Wait for every writer to finish and strictly reopen it.",
        ));
    }
    let project_id = parse_uuid(&request.project_id, "projectId")?;
    let dataset_id = parse_uuid(&request.dataset_id, "datasetId")?;
    let source_recipe_id = parse_uuid(&request.source_recipe_id, "sourceRecipeId")?;
    let staged_recipe_id = parse_uuid(&request.staged_recipe_id, "stagedRecipeId")?;
    let embedded_authority_sha256 = current_cache_authority_sha256_v1()?;
    if let Some(receipt) = request.resume_cache.as_ref()
        && (receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
            || receipt.cache_id.is_nil()
            || receipt.recipe_id != staged_recipe_id
            || receipt.target != request.config.target()
            || receipt.source_archive_sha256 != request.expected_archive_sha256
            || receipt.embedded_authority_sha256 != embedded_authority_sha256
            || receipt.result_id.trim().is_empty()
            || !lowercase_sha256(&receipt.manifest_sha256)
            || DateTime::parse_from_rfc3339(&receipt.created_at).is_err()
            || (receipt.stage == MultiModExternalCacheStageV1::MgaExecution
                && receipt.target != MultiModCompilerTargetV1::MgaMultigroupV1))
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.resume_cache_authority_mismatch",
            "The external cache receipt differs from the exact staged request authority.",
            "Discard the receipt and resume only from the matching versioned MultiMod family.",
        ));
    }
    if !loaded.document.supports_general_sem_v1() || loaded.document.project_id != project_id {
        return Err(authority_mismatch(
            "The project is not the exact requested general_sem_v1 archive.",
        ));
    }
    let dataset = loaded
        .datasets
        .into_iter()
        .find(|value| value.id == dataset_id)
        .ok_or_else(|| authority_mismatch("The requested raw dataset is not resident."))?;
    if dataset.schema.kind != DataKind::Raw
        || dataset.fingerprint.0 != request.dataset_fingerprint
        || !loaded.document.datasets.iter().any(|value| {
            value.id == dataset_id && value.fingerprint.0 == request.dataset_fingerprint
        })
    {
        return Err(authority_mismatch(
            "The requested dataset identity, fingerprint, or raw-data kind differs from the archive.",
        ));
    }
    let model_record = loaded
        .document
        .models
        .iter()
        .find(|value| value.model_id == request.model_id)
        .ok_or_else(|| authority_mismatch("The requested promoted model is not resident."))?;
    let ProjectModelPayloadV6::SemModelV4 {
        model,
        scientific_sha256,
    } = &model_record.payload
    else {
        return Err(authority_mismatch(
            "MultiMod execution requires a promoted SemModelV4 authority.",
        ));
    };
    if scientific_sha256 != &request.model_scientific_sha256
        || model.scientific_sha256().ok().as_deref()
            != Some(request.model_scientific_sha256.as_str())
    {
        return Err(authority_mismatch(
            "The requested model scientific identity differs from the archive.",
        ));
    }
    let source_recipe = loaded
        .document
        .recipes
        .iter()
        .find(|value| value.id == source_recipe_id)
        .ok_or_else(|| authority_mismatch("The requested source Recipe V4 is not resident."))?;
    if sha256_serialized(source_recipe) != request.source_recipe_document_sha256
        || source_recipe.dataset_fingerprint != request.dataset_fingerprint
    {
        return Err(authority_mismatch(
            "The requested source Recipe V4 identity differs from the archive.",
        ));
    }
    match &source_recipe.model_binding {
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id,
            scientific_sha256,
        } if model_id == &request.model_id
            && scientific_sha256 == &request.model_scientific_sha256 => {}
        _ => {
            return Err(authority_mismatch(
                "The source Recipe V4 does not reference the exact promoted model.",
            ));
        }
    }
    if loaded
        .document
        .recipes
        .iter()
        .any(|value| value.id == staged_recipe_id)
        || loaded
            .document
            .historical_recipes
            .iter()
            .any(|value| value.recipe_id() == staged_recipe_id)
    {
        return Err(authority_mismatch(
            "The staged Recipe V4 identifier is already resident.",
        ));
    }
    let staged_recipe = stage_recipe_v1(source_recipe, request)?;
    validate_heterogeneity_discovery_lock_source_v2(&loaded.document, &staged_recipe)?;
    let target = request.config.target();
    let artifact = compile_multimod_recipe_v1(&staged_recipe, model, target).map_err(|error| {
        failure(
            MultiModFailureStageV1::Compilation,
            "multimod.job.compilation_failed",
            format!("MultiMod compilation failed: {error}"),
            "Resolve the typed compiler blocker and stage a new exact recipe.",
        )
    })?;
    let runtime_support = multimod_runtime_support_v1(&staged_recipe, target);
    let mut mga_group_eligibility = None;
    if target == MultiModCompilerTargetV1::MgaMultigroupV1
        && runtime_support.readiness == MultiModRuntimeReadinessV1::BuiltInFromDataset
    {
        let config = staged_recipe.mga_multigroup.as_ref().ok_or_else(|| {
            failure(
                MultiModFailureStageV1::Compilation,
                "multimod.job.mga_config_missing",
                "The compiled MGA recipe lost its versioned configuration.",
                "Keep the request unpublished and rebuild it from the strict model authority.",
            )
        })?;
        let (design, _) = prepare_mga_design_v1(&dataset, model, config)?;
        mga_group_eligibility = Some(native_mga_group_eligibility_v1(&dataset, config, &design)?);
        multimod_mga_publishable_parameter_identities_v1(
            &dataset,
            &staged_recipe,
            model,
            &artifact,
            &design,
        )
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::RuntimeSupport,
                "multimod.job.mga_parameter_preflight_failed",
                format!(
                    "The selected MGA parameter inventory is not publishable by the exact refitter: {error}"
                ),
                "Select only identities listed by the MGA path/loading/weight or interaction target inventory.",
            )
        })?;
    }
    Ok(ResolvedMultiModAuthorityV1 {
        archive_path,
        archive_sha256: observed_sha256,
        project_id,
        dataset,
        model: model.clone(),
        staged_recipe,
        artifact,
        runtime_support,
        mga_group_eligibility,
    })
}

fn authority_mismatch(message: impl Into<String>) -> MultiModJobFailureV1 {
    failure(
        MultiModFailureStageV1::ArchiveAuthority,
        "multimod.job.archive_authority_mismatch",
        message,
        "Strictly reopen the archive and rebuild every identity-bound request field.",
    )
}

fn preflight_from_resolved(resolved: &ResolvedMultiModAuthorityV1) -> MultiModPreflightV1 {
    MultiModPreflightV1 {
        schema_version: MULTIMOD_JOB_SCHEMA_VERSION_V1,
        target: resolved.runtime_support.target,
        capability_cell_id: resolved.artifact.receipt().capability_cell.cell_id.clone(),
        readiness: resolved.runtime_support.readiness,
        stable_reason_codes: resolved.runtime_support.stable_reason_codes.clone(),
        staged_recipe_id: resolved.staged_recipe.id,
        staged_recipe_document_sha256: sha256_serialized(&resolved.staged_recipe),
        compilation_identity_sha256: resolved
            .artifact
            .receipt()
            .analytical_identity_sha256
            .clone(),
        mga_group_eligibility: resolved.mga_group_eligibility.clone(),
    }
}

#[tauri::command]
pub(crate) fn preflight_internal_labs_multimod_v1(
    request: MultiModJobRequestV1,
) -> Result<MultiModPreflightV1, MultiModJobFailureV1> {
    resolve_authority_v1(&request).map(|resolved| preflight_from_resolved(&resolved))
}

#[tauri::command]
pub(crate) fn prepare_internal_labs_multimod_raw_probe_metrics_v2(
    request: MultiModRawProbeMetricRequestV2,
) -> Result<Vec<ConditionalRawProbeFitMetricReceiptV2>, MultiModJobFailureV1> {
    if !matches!(request.orientation_sign, -1 | 1) {
        return Err(failure(
            MultiModFailureStageV1::Compilation,
            "multimod.conditional.raw_probe_orientation_invalid",
            "Raw-probe orientation must be exactly -1 or +1.",
            "Choose the fitted score orientation and prepare the metric again.",
        ));
    }
    let resolved = resolve_authority_v1(&request.staged)?;
    if resolved.runtime_support.target != MultiModCompilerTargetV1::GeneralSemConditionalProcessV2
        || resolved.runtime_support.readiness != MultiModRuntimeReadinessV1::BuiltInFromDataset
    {
        return Err(failure(
            MultiModFailureStageV1::RuntimeSupport,
            "multimod.conditional.raw_probe_runtime_unavailable",
            "Raw-probe metrics can be prepared only for a built-in conditional-process V2 profile.",
            "Resolve the exact runtime blocker before authoring raw-unit probes.",
        ));
    }
    let config = resolved
        .staged_recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| {
            failure(
                MultiModFailureStageV1::Compilation,
                "multimod.conditional.raw_probe_config_missing",
                "The staged recipe has no conditional-process V2 configuration.",
                "Restage the exact conditional-process request.",
            )
        })?;
    let frame =
        prepare_conditional_process_analysis_frame_v2(&resolved.dataset, &resolved.model, config)
            .map_err(map_runner_error)?;
    prepare_conditional_raw_probe_fit_metric_receipts_v2(
        &resolved.dataset,
        &resolved.model,
        &frame,
        config,
        &request.moderator_id,
        request.orientation_sign,
    )
    .map_err(map_runner_error)
}

fn grouping_value_at(
    array: &dyn Array,
    row: usize,
) -> Option<Result<CoreTypedGroupValueV1, MultiModJobFailureV1>> {
    if array.is_null(row) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        let value = values.value(row);
        return if value.is_empty() {
            None
        } else {
            Some(Ok(CoreTypedGroupValueV1::Text {
                value: value.to_owned(),
            }))
        };
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(Ok(CoreTypedGroupValueV1::Boolean {
            value: values.value(row),
        }));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(Ok(CoreTypedGroupValueV1::Integer {
            value: values.value(row),
        }));
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        let value = values.value(row);
        return Some(if value.is_finite() {
            Ok(CoreTypedGroupValueV1::Number {
                value: if value == 0.0 { 0.0 } else { value },
            })
        } else {
            Err(failure(
                MultiModFailureStageV1::ArchiveAuthority,
                "multimod.grouping_profile.nonfinite_value",
                "A candidate grouping column contains a nonfinite numeric value.",
                "Recode nonfinite grouping values to explicit missing values before profiling groups.",
            ))
        });
    }
    Some(Err(failure(
        MultiModFailureStageV1::ArchiveAuthority,
        "multimod.grouping_profile.unsupported_arrow_type",
        "A candidate grouping column uses an unsupported Arrow type.",
        "Use text, Boolean, integer, or finite numeric grouping values.",
    )))
}

fn grouping_value_label(value: &CoreTypedGroupValueV1) -> String {
    match value {
        CoreTypedGroupValueV1::Text { value } => value.clone(),
        CoreTypedGroupValueV1::Integer { value } => value.to_string(),
        CoreTypedGroupValueV1::Number { value } => value.to_string(),
        CoreTypedGroupValueV1::Boolean { value } => value.to_string(),
    }
}

#[tauri::command]
pub(crate) fn profile_internal_labs_multimod_grouping_v1(
    request: MultiModGroupingProfileRequestV1,
) -> Result<MultiModGroupingProfileV1, MultiModJobFailureV1> {
    validate_multimod_runtime_surface_access_v1(
        &request.surface,
        request.experimental_labs_enabled,
    )?;
    for digest in [
        &request.expected_archive_sha256,
        &request.dataset_fingerprint,
        &request.model_scientific_sha256,
        &request.source_recipe_document_sha256,
    ] {
        if !lowercase_sha256(digest) {
            return Err(failure(
                MultiModFailureStageV1::ArchiveAuthority,
                "multimod.grouping_profile.invalid_digest",
                "A grouping-profile authority digest is not a lowercase SHA-256 value.",
                "Rebuild the request from a strict native Archive V6 snapshot.",
            ));
        }
    }
    let path = Path::new(&request.archive_path);
    if request.archive_path.trim() != request.archive_path
        || !path.is_absolute()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("qpls"))
    {
        return Err(authority_mismatch(
            "Typed grouping discovery requires an absolute local .qpls archive path.",
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        authority_mismatch("The requested grouping-profile archive is unavailable.")
    })?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x0000_0400 != 0
    };
    #[cfg(not(windows))]
    let reparse = metadata.file_type().is_symlink();
    if !metadata.file_type().is_file() || reparse {
        return Err(authority_mismatch(
            "Typed grouping discovery requires a regular local archive, not a link or reparse point.",
        ));
    }
    let before_sha256 = sha256_file(path)?;
    if before_sha256 != request.expected_archive_sha256 {
        return Err(authority_mismatch(
            "The archive changed before typed grouping discovery.",
        ));
    }
    let loaded = load_project_archive_v6(path).map_err(|error| {
        failure(
            MultiModFailureStageV1::ArchiveAuthority,
            "multimod.grouping_profile.strict_reopen_failed",
            error.to_string(),
            "Restore a trusted Archive V6 file before discovering groups.",
        )
    })?;
    let project_id = parse_uuid(&request.project_id, "projectId")?;
    let dataset_id = parse_uuid(&request.dataset_id, "datasetId")?;
    let source_recipe_id = parse_uuid(&request.source_recipe_id, "sourceRecipeId")?;
    if loaded.document.project_id != project_id || !loaded.document.supports_general_sem_v1() {
        return Err(authority_mismatch(
            "The grouping profile project identity or general_sem_v1 marker differs.",
        ));
    }
    let dataset = loaded
        .datasets
        .iter()
        .find(|candidate| candidate.id == dataset_id)
        .ok_or_else(|| authority_mismatch("The requested grouping-profile dataset is absent."))?;
    if dataset.schema.kind != DataKind::Raw || dataset.fingerprint.0 != request.dataset_fingerprint
    {
        return Err(authority_mismatch(
            "The requested grouping-profile dataset identity differs.",
        ));
    }
    let model_record = loaded
        .document
        .models
        .iter()
        .find(|candidate| candidate.model_id == request.model_id)
        .ok_or_else(|| authority_mismatch("The requested grouping-profile model is absent."))?;
    let ProjectModelPayloadV6::SemModelV4 {
        model,
        scientific_sha256,
    } = &model_record.payload
    else {
        return Err(authority_mismatch(
            "Typed grouping discovery requires the promoted SemModelV4 authority.",
        ));
    };
    if scientific_sha256 != &request.model_scientific_sha256
        || model.scientific_sha256().ok().as_deref()
            != Some(request.model_scientific_sha256.as_str())
    {
        return Err(authority_mismatch(
            "The grouping-profile model scientific identity differs.",
        ));
    }
    let source_recipe = loaded
        .document
        .recipes
        .iter()
        .find(|candidate| candidate.id == source_recipe_id)
        .ok_or_else(|| authority_mismatch("The grouping-profile source recipe is absent."))?;
    if sha256_serialized(source_recipe) != request.source_recipe_document_sha256
        || source_recipe.dataset_fingerprint != request.dataset_fingerprint
    {
        return Err(authority_mismatch(
            "The grouping-profile source Recipe V4 identity differs.",
        ));
    }
    match &source_recipe.model_binding {
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id,
            scientific_sha256,
        } if model_id == &request.model_id
            && scientific_sha256 == &request.model_scientific_sha256 => {}
        _ => {
            return Err(authority_mismatch(
                "The grouping-profile source recipe does not reference the exact model.",
            ));
        }
    }

    let observed_source_columns = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed { source_column, .. } => Some(source_column.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let model_positions = observed_source_columns
        .iter()
        .filter_map(|column| dataset.batch.schema().index_of(column).ok())
        .collect::<Vec<_>>();
    let mut columns = Vec::new();
    let mut omitted_high_cardinality_columns = Vec::new();
    for column_metadata in &dataset.schema.columns {
        if column_metadata.scale_type == ScaleType::Identifier {
            continue;
        }
        let Ok(position) = dataset.batch.schema().index_of(&column_metadata.name) else {
            return Err(authority_mismatch(format!(
                "Dataset metadata column {} is absent from its Arrow batch.",
                column_metadata.name
            )));
        };
        let array = dataset.batch.column(position);
        let mut counts = BTreeMap::<String, (CoreTypedGroupValueV1, usize, usize)>::new();
        for row in 0..dataset.batch.num_rows() {
            let Some(value) = grouping_value_at(array.as_ref(), row) else {
                continue;
            };
            let value = value?;
            let key = value.canonical_key();
            let entry = counts.entry(key).or_insert((value, 0, 0));
            entry.1 += 1;
            if model_row_complete(dataset, &model_positions, row).unwrap_or(false) {
                entry.2 += 1;
            }
        }
        if counts.len() < 2 {
            continue;
        }
        if counts.len() > 250 {
            omitted_high_cardinality_columns.push(column_metadata.name.clone());
            continue;
        }
        let groups = counts
            .into_iter()
            .map(|(key, (value, selected_rows, complete_cases))| {
                let raw_label = grouping_value_label(&value);
                let label = column_metadata
                    .value_labels
                    .get(&raw_label)
                    .cloned()
                    .unwrap_or(raw_label);
                let group_digest =
                    sha256_bytes(format!("{}\0{key}", column_metadata.name).as_bytes());
                MultiModGroupingProfileGroupV1 {
                    group_id: format!("group:{}", &group_digest[..20]),
                    label,
                    value,
                    selected_rows,
                    complete_cases,
                }
            })
            .collect::<Vec<_>>();
        columns.push(MultiModGroupingProfileColumnV1 {
            column: column_metadata.name.clone(),
            label: column_metadata
                .label
                .clone()
                .unwrap_or_else(|| column_metadata.name.clone()),
            used_as_indicator: observed_source_columns.contains(&column_metadata.name),
            groups,
        });
    }
    if sha256_file(path)? != before_sha256 {
        return Err(authority_mismatch(
            "The archive changed during typed grouping discovery.",
        ));
    }
    Ok(MultiModGroupingProfileV1 {
        schema_version: 1,
        archive_sha256: before_sha256,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        columns,
        omitted_high_cardinality_columns,
        source_rechecked_unchanged: true,
    })
}

fn core_group_value_to_estimation(
    value: &CoreTypedGroupValueV1,
) -> Result<EstimationTypedGroupValueV1, MultiModJobFailureV1> {
    match value {
        CoreTypedGroupValueV1::Text { value } => Ok(EstimationTypedGroupValueV1::Text {
            value: value.clone(),
        }),
        CoreTypedGroupValueV1::Integer { value } => {
            Ok(EstimationTypedGroupValueV1::Integer { value: *value })
        }
        CoreTypedGroupValueV1::Number { value } => {
            EstimationTypedGroupValueV1::finite_number(*value).map_err(|error| {
                failure(
                    MultiModFailureStageV1::Compilation,
                    "multimod.job.invalid_group_value",
                    error.to_string(),
                    "Choose a finite typed group value.",
                )
            })
        }
        CoreTypedGroupValueV1::Boolean { value } => {
            Ok(EstimationTypedGroupValueV1::Boolean { value: *value })
        }
    }
}

fn row_token(dataset: &Dataset, row: usize) -> String {
    format!("qpls.row.v1:{}:{row}", dataset.id)
}

fn group_key_at(array: &dyn Array, row: usize) -> Option<Result<String, ()>> {
    if array.is_null(row) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        let value = values.value(row);
        return if value.is_empty() {
            None
        } else {
            Some(Ok(format!("text:{value}")))
        };
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(Ok(format!("boolean:{}", values.value(row))));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(Ok(format!("integer:{}", values.value(row))));
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        let value = values.value(row);
        return if value.is_finite() {
            let normalized = if value == 0.0 { 0.0 } else { value };
            Some(Ok(format!("number:{:016x}", normalized.to_bits())))
        } else {
            Some(Err(()))
        };
    }
    Some(Err(()))
}

fn model_row_complete(dataset: &Dataset, positions: &[usize], row: usize) -> Result<bool, ()> {
    for position in positions {
        let array = dataset.batch.column(*position);
        if array.is_null(row) {
            return Ok(false);
        }
        if let Some(values) = array.as_any().downcast_ref::<Float64Array>()
            && !values.value(row).is_finite()
        {
            return Err(());
        }
        if let Some(values) = array.as_any().downcast_ref::<StringArray>()
            && values.value(row).is_empty()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn prepare_mga_design_v1(
    dataset: &Dataset,
    model: &SemModelV4,
    config: &MgaMultigroupV1,
) -> Result<(MultigroupDesignV1, Vec<qpls_core::ExcludedRowReceiptV1>), MultiModJobFailureV1> {
    let schema = dataset.batch.schema();
    let grouping_position = schema.index_of(&config.grouping_column).map_err(|_| {
        authority_mismatch(format!(
            "Grouping column {} is absent from the resident dataset.",
            config.grouping_column
        ))
    })?;
    let mut model_positions = Vec::new();
    for source_column in model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed { source_column, .. }
                if source_column != &config.grouping_column =>
            {
                Some(source_column)
            }
            _ => None,
        })
    {
        let position = schema.index_of(source_column).map_err(|_| {
            authority_mismatch(format!(
                "Model source column {source_column} is absent from the resident dataset."
            ))
        })?;
        if !model_positions.contains(&position) {
            model_positions.push(position);
        }
    }
    let groups = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            Ok(GroupIdentityV1 {
                index: GroupIndexV1::new(index).map_err(|error| {
                    failure(
                        MultiModFailureStageV1::Compilation,
                        "multimod.job.invalid_group_index",
                        error.to_string(),
                        "Choose between 2 and 20 groups.",
                    )
                })?,
                value: core_group_value_to_estimation(&group.value)?,
                display_label: group.label.clone(),
            })
        })
        .collect::<Result<Vec<_>, MultiModJobFailureV1>>()?;
    let selected = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            (
                group.value.canonical_key(),
                GroupIndexV1::new(index).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let group_array = dataset.batch.column(grouping_position);
    let mut rows = Vec::new();
    let mut excluded = Vec::new();
    for row in 0..dataset.batch.num_rows() {
        let stable_row_token = row_token(dataset, row);
        let group_key = match group_key_at(group_array.as_ref(), row) {
            None => {
                excluded.push(qpls_core::ExcludedRowReceiptV1 {
                    stable_row_token,
                    typed_group_value: "missing".into(),
                    reason: qpls_core::ExcludedRowReasonV1::MissingGroupValue,
                });
                continue;
            }
            Some(Err(())) => {
                excluded.push(qpls_core::ExcludedRowReceiptV1 {
                    stable_row_token,
                    typed_group_value: "nonfinite_or_unsupported".into(),
                    reason: qpls_core::ExcludedRowReasonV1::NonfiniteValue,
                });
                continue;
            }
            Some(Ok(value)) => value,
        };
        let Some(group) = selected.get(&group_key).copied() else {
            excluded.push(qpls_core::ExcludedRowReceiptV1 {
                stable_row_token,
                typed_group_value: group_key,
                reason: qpls_core::ExcludedRowReasonV1::UnselectedGroupValue,
            });
            continue;
        };
        match model_row_complete(dataset, &model_positions, row) {
            Ok(true) => rows.push(SelectedGroupRowV1 {
                source_row: row as u64,
                stable_row_token: row as u64,
                group,
            }),
            Ok(false) => excluded.push(qpls_core::ExcludedRowReceiptV1 {
                stable_row_token,
                typed_group_value: group_key,
                reason: qpls_core::ExcludedRowReasonV1::MissingModelValue,
            }),
            Err(()) => excluded.push(qpls_core::ExcludedRowReceiptV1 {
                stable_row_token,
                typed_group_value: group_key,
                reason: qpls_core::ExcludedRowReasonV1::NonfiniteValue,
            }),
        }
    }
    Ok((MultigroupDesignV1 { groups, rows }, excluded))
}

fn serialized_enum_code_v1<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown_eligibility_code".into())
}

fn native_mga_group_eligibility_v1(
    dataset: &Dataset,
    config: &MgaMultigroupV1,
    design: &MultigroupDesignV1,
) -> Result<NativeMgaGroupEligibilityV1, MultiModJobFailureV1> {
    let physical_counts = design
        .groups
        .iter()
        .map(|group| {
            design
                .rows
                .iter()
                .filter(|row| row.group == group.index)
                .count()
        })
        .collect::<Vec<_>>();
    let (count_basis, eligibility) = if config.profile == MgaModelProfileV1::FrequencyWeightedPls {
        let Some(qpls_core::AnalysisWeightBindingV1::Frequency { column }) = &config.weight else {
            return Err(failure(
                MultiModFailureStageV1::Compilation,
                "multimod.job.mga_frequency_weight_missing",
                "Frequency-weighted MGA requires one explicit positive-integer frequency column.",
                "Choose the exact frequency column and stage the request again.",
            ));
        };
        let source_rows = design
            .rows
            .iter()
            .map(|row| row.source_row)
            .collect::<Vec<_>>();
        let counts = multimod_frequency_counts_for_source_rows_v1(dataset, column, &source_rows)
            .map_err(|error| {
                failure(
                    MultiModFailureStageV1::RuntimeSupport,
                    "multimod.job.mga_frequency_weight_invalid",
                    format!("The selected frequency column failed positive-integer count-space validation: {error}"),
                    "Correct missing, zero, negative, fractional, nonfinite, or overflowing frequencies and stage a new request.",
                )
            })?;
        let frequency_design = FrequencyMultigroupDesignV1 {
            groups: design.groups.clone(),
            rows: design
                .rows
                .iter()
                .zip(counts)
                .map(|(row, frequency)| FrequencySelectedGroupRowV1 {
                    source_row: row.source_row,
                    stable_row_token: row.stable_row_token,
                    group: row.group,
                    frequency,
                })
                .collect(),
        };
        (
            NativeMgaCountBasisV1::FrequencyExpandedCases,
            assess_frequency_multigroup_design_v1(&frequency_design),
        )
    } else {
        (
            NativeMgaCountBasisV1::PhysicalCompleteRows,
            assess_multigroup_design_v1(design),
        )
    };
    let groups = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| NativeMgaGroupCountV1 {
            group_id: group.group_id.clone(),
            label: group.label.clone(),
            physical_complete_rows: physical_counts.get(index).copied().unwrap_or(0),
            effective_complete_cases: eligibility
                .group_counts
                .iter()
                .find(|summary| summary.group.get() == index)
                .map(|summary| summary.complete_cases)
                .unwrap_or(0),
        })
        .collect();
    Ok(NativeMgaGroupEligibilityV1 {
        count_basis,
        groups,
        maximum_imbalance_ratio: eligibility.maximum_imbalance_ratio,
        eligible: eligibility.eligible,
        warning_codes: eligibility
            .warnings
            .iter()
            .map(|warning| serialized_enum_code_v1(&warning.code))
            .collect(),
        blocker_codes: eligibility
            .blockers
            .iter()
            .map(|blocker| serialized_enum_code_v1(&blocker.code))
            .collect(),
    })
}

fn execute_builtin_v1(
    resolved: &ResolvedMultiModAuthorityV1,
    cancellation: &AtomicBool,
    progress: impl Fn(MultiModRunnerProgressV1) + Sync,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1> {
    match resolved.runtime_support.readiness {
        MultiModRuntimeReadinessV1::BuiltInFromDataset => {}
        MultiModRuntimeReadinessV1::PreparedAdapterRequired => {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "native job refuses prepared-only profile: {}",
                resolved.runtime_support.stable_reason_codes.join(",")
            )));
        }
        MultiModRuntimeReadinessV1::Blocked => {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "native runtime blocker: {}",
                resolved.runtime_support.stable_reason_codes.join(",")
            )));
        }
    }
    let cancelled = || cancellation.load(Ordering::Acquire);
    match resolved.runtime_support.target {
        MultiModCompilerTargetV1::MgaMultigroupV1 => {
            Err(MultiModRunnerErrorV1::UnsupportedProfile(
                "multimod.job.mga_resumable_dispatch_required: native MGA must execute through the external-cache dispatcher"
                    .into(),
            ))
        }
        MultiModCompilerTargetV1::PlsHeterogeneityV2 => run_compiled_raw_pls_heterogeneity_v2(
            &resolved.dataset,
            &resolved.staged_recipe,
            &resolved.model,
            &resolved.artifact,
            cancelled,
            progress,
        ),
        MultiModCompilerTargetV1::InterventionalCausalMediationV1 => {
            run_compiled_interventional_causal_mediation_raw_v1(
                &resolved.dataset,
                &resolved.staged_recipe,
                &resolved.model,
                &resolved.artifact,
                cancelled,
                progress,
            )
        }
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2 => {
            run_compiled_general_sem_conditional_process_raw_output_v2(
                &resolved.dataset,
                &resolved.staged_recipe,
                &resolved.model,
                &resolved.artifact,
                ConditionalProcessRawAuthorityV2::BuiltIn,
                cancelled,
                progress,
            )
        }
    }
}

fn set_result_sidecars_v1(
    result: &mut MultiModAnalysisResultV1,
    sidecars: Vec<qpls_core::MultimodResultSidecarDescriptorV1>,
) {
    match result {
        MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(value) => value.sidecars = sidecars,
        MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value) => value.sidecars = sidecars,
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value) => {
            value.sidecars = sidecars
        }
        MultiModAnalysisResultV1::InterventionalMediationResultV1(value) => {
            value.sidecars = sidecars
        }
    }
}

fn archive_ready_output_v1(
    result_id: &str,
    recipe_id: Uuid,
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledMultiModRecipeV1,
    mut output: MultiModRunOutputV1,
    standard_surface: bool,
) -> Result<
    (
        MultiModResultAttachmentV1,
        Vec<MultiModSidecarPayloadV1>,
        bool,
    ),
    MultiModJobFailureV1,
> {
    promote_completed_multimod_result_v1(recipe, artifact, &mut output.result).map_err(|error| {
        failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.candidate_promotion_failed",
            error,
            "Keep the result unpublished; use a Labs preview or rebuild the candidate with exact profile-cell authority.",
        )
    })?;
    if standard_surface
        && output.result.provenance().qualification
            != MultimodQualificationStateV1::ReleaseQualifiedCandidate
    {
        return Err(failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.standard_result_not_qualified",
            "The embedded candidate authority does not cover every exact profile cell required by this Standard result.",
            "Keep the result unpublished and use only a candidate package whose immutable authority covers the exact requested profile.",
        ));
    }
    if output.evidence.is_empty() {
        return Err(failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.evidence_missing",
            "The built-in runner returned no auditable evidence payload.",
            "Keep the result unpublished and repair the exact runner evidence adapter.",
        ));
    }
    if result_id.trim().is_empty() || result_id != result_id.trim() {
        return Err(failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.result_id_invalid",
            "The frozen result identity is empty or contains surrounding whitespace.",
            "Keep the result unpublished and rebuild the exact execution-cache authority.",
        ));
    }
    let identity = multimod_result_identity_sha256_v1(&output.result).map_err(|error| {
        failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.result_identity_failed",
            error.to_string(),
            "Keep the result unpublished and inspect its target identities.",
        )
    })?;
    let encoded = crate::multimod_evidence_sidecars_v1::encode_multimod_runner_evidence_v1(
        result_id,
        &identity,
        &dataset.id.to_string(),
        &output.evidence,
    )
    .map_err(|error| {
        failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.evidence_sidecar_failed",
            error,
            "Keep the result unpublished and repair the typed Arrow evidence adapter.",
        )
    })?;
    let payloads = encoded.payloads;
    let total_bytes = encoded.total_uncompressed_bytes;
    if total_bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        return Err(failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.sidecar_run_cap_exceeded",
            "The completed evidence exceeds the 512 MiB per-run archive cap.",
            "Reduce groups, targets, probes, classes, or resamples and start a new run.",
        ));
    }
    let descriptors = payloads
        .iter()
        .map(|payload| payload.descriptor.clone())
        .collect::<Vec<_>>();
    set_result_sidecars_v1(&mut output.result, descriptors.clone());
    output.result.ensure_valid().map_err(|error| {
        failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.result_contract_invalid",
            error.to_string(),
            "Keep the result unpublished and repair the scientific result contract.",
        )
    })?;
    let attachment = MultiModResultAttachmentV1::new(
        result_id.to_owned(),
        recipe_id,
        output.result,
        descriptors,
    )
    .map_err(|error| {
        failure(
            MultiModFailureStageV1::Evidence,
            "multimod.job.attachment_invalid",
            error.to_string(),
            "Keep the result unpublished and repair its exact attachment identity.",
        )
    })?;
    Ok((
        attachment,
        payloads,
        total_bytes > MULTIMOD_SIDECAR_WARNING_BYTES_V1,
    ))
}

fn cache_root_v1() -> Result<PathBuf, MultiModJobFailureV1> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let application = base.join("QuickPLS");
    let root = application.join("multimod-cache-v1");
    for directory in [&base, &application, &root] {
        if !directory.exists() {
            fs::create_dir(directory).map_err(|error| {
                failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.cache_root_failed",
                    error.to_string(),
                    "Keep the result unpublished and verify local application-data write access.",
                )
            })?;
        }
        let metadata = fs::symlink_metadata(directory).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_root_unavailable",
                error.to_string(),
                "Keep the result unpublished and restore the local cache directory.",
            )
        })?;
        if !metadata.is_dir() || metadata_is_reparse_v1(&metadata) {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_root_not_regular",
                format!(
                    "The MultiMod cache ancestor {} is not a regular local directory.",
                    directory.display()
                ),
                "Remove the unsafe cache redirection and retry.",
            ));
        }
    }
    Ok(root)
}

fn cache_directory_for_receipt_v1(
    receipt: &MultiModExternalCacheReceiptV1,
    expected_stage: MultiModExternalCacheStageV1,
) -> Result<PathBuf, MultiModJobFailureV1> {
    if receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || receipt.stage != expected_stage
        || !lowercase_sha256(&receipt.manifest_sha256)
        || receipt.embedded_authority_sha256 != current_cache_authority_sha256_v1()?
        || receipt.result_id.trim().is_empty()
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_receipt_invalid",
            "The external cache receipt does not match its exact versioned stage.",
            "Discard the receipt and start a new exact run.",
        ));
    }
    let directory = cache_root_v1()?.join(receipt.cache_id.to_string());
    if directory.to_string_lossy() != receipt.cache_directory {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_path_mismatch",
            "The cache receipt path differs from the app-owned cache identity.",
            "Discard the receipt and start a new exact run.",
        ));
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_directory_unavailable",
            error.to_string(),
            "Preserve the receipt for diagnosis or start a new exact run.",
        )
    })?;
    if !metadata.is_dir() || metadata_is_reparse_v1(&metadata) {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_directory_not_regular",
            "The app-owned cache directory is missing, redirected, or not a regular directory.",
            "Do not follow the redirected path; discard the receipt.",
        ));
    }
    Ok(directory)
}

fn read_regular_cache_manifest_v1(directory: &Path) -> Result<Vec<u8>, MultiModJobFailureV1> {
    let path = directory.join("manifest.json");
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_unavailable",
            error.to_string(),
            "Discard the incomplete cache and start a new exact run.",
        )
    })?;
    if !metadata.is_file() || metadata_is_reparse_v1(&metadata) {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_not_regular",
            "The cache manifest is not a regular app-owned file.",
            "Discard the cache; do not resume or publish from it.",
        ));
    }
    fs::read(path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_unavailable",
            error.to_string(),
            "Discard the incomplete cache and start a new exact run.",
        )
    })
}

fn validate_mga_execution_manifest_v1(
    manifest: &MultiModMgaExecutionCacheManifestV1,
) -> Result<(), MultiModJobFailureV1> {
    if manifest.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || manifest.target != MultiModCompilerTargetV1::MgaMultigroupV1
        || manifest.result_id.trim().is_empty()
        || !lowercase_sha256(&manifest.source_archive_sha256)
        || !lowercase_sha256(&manifest.embedded_authority_sha256)
        || manifest.embedded_authority_sha256 != current_cache_authority_sha256_v1()?
        || !lowercase_sha256(&manifest.staged_recipe_sha256)
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_manifest_invalid",
            "The MGA execution-cache manifest has an invalid versioned identity.",
            "Discard the cache; do not resume or publish from it.",
        ));
    }
    manifest.plan.ensure_valid().map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_plan_invalid",
            error.to_string(),
            "Discard the cache; rebuild the execution plan from strict authority.",
        )
    })?;
    manifest
        .cache
        .ensure_valid(&manifest.plan)
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_cache_invalid",
                error.to_string(),
                "Discard the cache; no partial scientific result may be published.",
            )
        })?;
    if let Some(expected) = manifest.finalized_cache_sha256.as_deref() {
        let actual = manifest
            .cache
            .finalized_identity_sha256(&manifest.plan)
            .map_err(|error| {
                failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.mga_execution_finalization_invalid",
                    error.to_string(),
                    "Discard the cache; no partial scientific result may be published.",
                )
            })?;
        if expected != actual {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_finalization_mismatch",
                "The finalized MGA cache identity differs from its completed shard inventory.",
                "Discard the cache; no scientific result may be published from it.",
            ));
        }
    }
    Ok(())
}

fn serialize_mga_execution_manifest_v1(
    manifest: &MultiModMgaExecutionCacheManifestV1,
) -> Result<Vec<u8>, MultiModJobFailureV1> {
    validate_mga_execution_manifest_v1(manifest)?;
    serde_json::to_vec_pretty(manifest).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_manifest_encode_failed",
            error.to_string(),
            "Keep the result unpublished and retry from strict authority.",
        )
    })
}

fn atomic_replace_cache_manifest_v1(path: &Path, bytes: &[u8]) -> Result<(), MultiModJobFailureV1> {
    let parent = path.parent().ok_or_else(|| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_parent_missing",
            "The cache manifest has no parent directory.",
            "Discard the cache and start a new exact run.",
        )
    })?;
    let temporary = parent.join(format!(".manifest-{}.tmp", Uuid::new_v4()));
    write_new_file(&temporary, bytes)?;
    let staged_bytes = fs::read(&temporary).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_staging_readback_failed",
            error.to_string(),
            "Preserve the prior cache receipt and retry without publishing a result.",
        )
    })?;
    if staged_bytes != bytes {
        let _ = fs::remove_file(&temporary);
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_staging_readback_mismatch",
            "The synchronized staged cache manifest differs before atomic replacement.",
            "Preserve the prior cache receipt and retry without publishing a result.",
        ));
    }
    let replacement = (|| -> Result<(), MultiModJobFailureV1> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
            let destination = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            let replacement = temporary
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            // SAFETY: both path buffers are NUL terminated and remain alive.
            if unsafe {
                ReplaceFileW(
                    destination.as_ptr(),
                    replacement.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.cache_manifest_atomic_replace_failed",
                    std::io::Error::last_os_error().to_string(),
                    "Preserve the prior cache receipt and retry without publishing a result.",
                ));
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temporary, path).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_manifest_atomic_replace_failed",
                error.to_string(),
                "Preserve the prior cache receipt and retry without publishing a result.",
            )
        })?;
        Ok(())
    })();
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    replacement
}

fn create_mga_execution_cache_v1(
    source: &ResolvedMultiModAuthorityV1,
    result_id: String,
    plan: MgaExecutionPlanV1,
    cache: MgaExecutionCacheV1,
) -> Result<MultiModExternalCacheReceiptV1, MultiModJobFailureV1> {
    let cache_id = Uuid::new_v4();
    let directory = cache_root_v1()?.join(cache_id.to_string());
    fs::create_dir(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_directory_failed",
            error.to_string(),
            "Keep the result unpublished and retry with a new cache identity.",
        )
    })?;
    let mut guard = CacheCreationGuardV1::new(directory.clone());
    let created_at = now_utc();
    let embedded_authority_sha256 = current_cache_authority_sha256_v1()?;
    let manifest = MultiModMgaExecutionCacheManifestV1 {
        schema_version: MULTIMOD_CACHE_SCHEMA_VERSION_V1,
        cache_id,
        source_archive_path: source.archive_path.to_string_lossy().into_owned(),
        source_archive_sha256: source.archive_sha256.clone(),
        embedded_authority_sha256: embedded_authority_sha256.clone(),
        recipe_id: source.staged_recipe.id,
        staged_recipe_sha256: sha256_serialized(&source.staged_recipe),
        result_id: result_id.clone(),
        target: MultiModCompilerTargetV1::MgaMultigroupV1,
        plan,
        cache,
        finalized_cache_sha256: None,
        created_at: created_at.clone(),
        updated_at: created_at.clone(),
    };
    let bytes = serialize_mga_execution_manifest_v1(&manifest)?;
    let path = directory.join("manifest.json");
    write_new_file(&path, &bytes)?;
    guard.retained(path);
    let receipt = MultiModExternalCacheReceiptV1 {
        schema_version: MULTIMOD_CACHE_SCHEMA_VERSION_V1,
        cache_id,
        cache_directory: directory.to_string_lossy().into_owned(),
        manifest_sha256: sha256_bytes(&bytes),
        embedded_authority_sha256,
        source_archive_sha256: source.archive_sha256.clone(),
        result_id,
        recipe_id: source.staged_recipe.id,
        target: MultiModCompilerTargetV1::MgaMultigroupV1,
        stage: MultiModExternalCacheStageV1::MgaExecution,
        created_at,
    };
    guard.disarm();
    Ok(receipt)
}

fn load_mga_execution_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
    source: &ResolvedMultiModAuthorityV1,
    rebuilt_plan: &MgaExecutionPlanV1,
) -> Result<(MgaExecutionCacheV1, String), MultiModJobFailureV1> {
    let directory =
        cache_directory_for_receipt_v1(receipt, MultiModExternalCacheStageV1::MgaExecution)?;
    let bytes = read_regular_cache_manifest_v1(&directory)?;
    if sha256_bytes(&bytes) != receipt.manifest_sha256 {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_manifest_tampered",
            "The MGA execution-cache manifest digest changed.",
            "Discard the cache; do not resume or publish from it.",
        ));
    }
    let manifest: MultiModMgaExecutionCacheManifestV1 =
        serde_json::from_slice(&bytes).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_manifest_decode_failed",
                error.to_string(),
                "Discard the cache; do not resume or publish from it.",
            )
        })?;
    validate_mga_execution_manifest_v1(&manifest)?;
    if manifest.cache_id != receipt.cache_id
        || manifest.source_archive_path != source.archive_path.to_string_lossy()
        || manifest.source_archive_sha256 != source.archive_sha256
        || manifest.embedded_authority_sha256 != receipt.embedded_authority_sha256
        || manifest.recipe_id != source.staged_recipe.id
        || manifest.staged_recipe_sha256 != sha256_serialized(&source.staged_recipe)
        || manifest.result_id != receipt.result_id
        || manifest.target != receipt.target
        || manifest.plan != *rebuilt_plan
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_identity_mismatch",
            "The MGA execution cache differs from the rebuilt strict plan or staged authority.",
            "Discard the cache and start a new exact run.",
        ));
    }
    manifest.cache.ensure_valid(rebuilt_plan).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cache_invalid",
            error.to_string(),
            "Discard the cache; no partial result may be published.",
        )
    })?;
    Ok((manifest.cache, manifest.created_at))
}

fn update_mga_execution_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
    source: &ResolvedMultiModAuthorityV1,
    plan: &MgaExecutionPlanV1,
    cache: &MgaExecutionCacheV1,
    finalized_cache_sha256: Option<String>,
) -> Result<MultiModExternalCacheReceiptV1, MultiModJobFailureV1> {
    let (prior_cache, created_at) = load_mga_execution_cache_v1(receipt, source, plan)?;
    if prior_cache
        .entries
        .iter()
        .any(|entry| !cache.entries.contains(entry))
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cache_not_append_only",
            "A completed MGA shard disappeared or changed before cache persistence.",
            "Do not resume or publish from this cache.",
        ));
    }
    let directory =
        cache_directory_for_receipt_v1(receipt, MultiModExternalCacheStageV1::MgaExecution)?;
    let manifest = MultiModMgaExecutionCacheManifestV1 {
        schema_version: MULTIMOD_CACHE_SCHEMA_VERSION_V1,
        cache_id: receipt.cache_id,
        source_archive_path: source.archive_path.to_string_lossy().into_owned(),
        source_archive_sha256: source.archive_sha256.clone(),
        embedded_authority_sha256: receipt.embedded_authority_sha256.clone(),
        recipe_id: source.staged_recipe.id,
        staged_recipe_sha256: sha256_serialized(&source.staged_recipe),
        result_id: receipt.result_id.clone(),
        target: MultiModCompilerTargetV1::MgaMultigroupV1,
        plan: plan.clone(),
        cache: cache.clone(),
        finalized_cache_sha256,
        created_at: created_at.clone(),
        updated_at: now_utc(),
    };
    let bytes = serialize_mga_execution_manifest_v1(&manifest)?;
    atomic_replace_cache_manifest_v1(&directory.join("manifest.json"), &bytes)?;
    Ok(MultiModExternalCacheReceiptV1 {
        manifest_sha256: sha256_bytes(&bytes),
        created_at,
        ..receipt.clone()
    })
}

fn remove_mga_execution_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
) -> Result<(), MultiModJobFailureV1> {
    if receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || receipt.stage != MultiModExternalCacheStageV1::MgaExecution
        || receipt.target != MultiModCompilerTargetV1::MgaMultigroupV1
        || !lowercase_sha256(&receipt.manifest_sha256)
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_receipt_invalid",
            "The retained receipt is not an exact MGA execution-cache V1 identity.",
            "Preserve the cache and remove it only through verified QuickPLS cleanup.",
        ));
    }
    let expected_directory = cache_root_v1()?.join(receipt.cache_id.to_string());
    if expected_directory.to_string_lossy() != receipt.cache_directory {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_path_mismatch",
            "The MGA execution-cache path differs from its app-owned identity.",
            "Preserve the cache and inspect its receipt before cleanup.",
        ));
    }
    if !expected_directory.exists() {
        return Ok(());
    }
    let directory =
        cache_directory_for_receipt_v1(receipt, MultiModExternalCacheStageV1::MgaExecution)?;
    let bytes = read_regular_cache_manifest_v1(&directory)?;
    if sha256_bytes(&bytes) != receipt.manifest_sha256 {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_manifest_tampered",
            "The retained MGA execution-cache manifest differs from its receipt.",
            "Do not delete the cache automatically; preserve it for diagnosis.",
        ));
    }
    let manifest: MultiModMgaExecutionCacheManifestV1 =
        serde_json::from_slice(&bytes).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_cleanup_manifest_invalid",
                error.to_string(),
                "Do not delete the cache automatically; preserve it for diagnosis.",
            )
        })?;
    validate_mga_execution_manifest_v1(&manifest)?;
    if manifest.cache_id != receipt.cache_id
        || manifest.result_id != receipt.result_id
        || manifest.recipe_id != receipt.recipe_id
        || manifest.source_archive_sha256 != receipt.source_archive_sha256
        || manifest.embedded_authority_sha256 != receipt.embedded_authority_sha256
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_identity_mismatch",
            "The retained MGA execution cache differs from its cleanup receipt.",
            "Do not delete the cache automatically; preserve it for diagnosis.",
        ));
    }
    let entries = fs::read_dir(&directory)
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_cleanup_inventory_unavailable",
                error.to_string(),
                "Preserve the cache and retry cleanup later.",
            )
        })?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    if entries.len() != 1
        || entries[0].file_name() != std::ffi::OsStr::new("manifest.json")
        || metadata_is_reparse_v1(&fs::symlink_metadata(entries[0].path()).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_cleanup_inventory_unavailable",
                error.to_string(),
                "Preserve the cache and retry cleanup later.",
            )
        })?)
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_inventory_mismatch",
            "The MGA execution cache contains an unexpected or redirected entry.",
            "Do not delete it automatically; preserve it for diagnosis.",
        ));
    }
    fs::remove_file(directory.join("manifest.json")).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_failed",
            error.to_string(),
            "Preserve the receipt and retry cleanup later.",
        )
    })?;
    fs::remove_dir(directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.mga_execution_cleanup_failed",
            error.to_string(),
            "Preserve the receipt and retry cleanup later.",
        )
    })
}

fn validate_superseded_execution_cache_binding_v1(
    source: &ResolvedMultiModAuthorityV1,
    result_id: &str,
    receipt: &MultiModExternalCacheReceiptV1,
) -> Result<(), MultiModJobFailureV1> {
    if receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || receipt.stage != MultiModExternalCacheStageV1::MgaExecution
        || receipt.target != MultiModCompilerTargetV1::MgaMultigroupV1
        || source.runtime_support.target != MultiModCompilerTargetV1::MgaMultigroupV1
        || receipt.source_archive_sha256 != source.archive_sha256
        || receipt.recipe_id != source.staged_recipe.id
        || receipt.result_id != result_id
        || !lowercase_sha256(&receipt.manifest_sha256)
        || receipt.embedded_authority_sha256 != current_cache_authority_sha256_v1()?
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.superseded_execution_cache_mismatch",
            "The superseded MGA execution cache differs from the archive-ready result authority.",
            "Keep both caches unpublished and resume only from an exact staged authority.",
        ));
    }
    Ok(())
}

fn validate_finalized_execution_cache_v1(
    source: &ResolvedMultiModAuthorityV1,
    result_id: &str,
    receipt: &MultiModExternalCacheReceiptV1,
) -> Result<(), MultiModJobFailureV1> {
    validate_superseded_execution_cache_binding_v1(source, result_id, receipt)?;
    let directory =
        cache_directory_for_receipt_v1(receipt, MultiModExternalCacheStageV1::MgaExecution)?;
    let bytes = read_regular_cache_manifest_v1(&directory)?;
    if sha256_bytes(&bytes) != receipt.manifest_sha256 {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.superseded_execution_cache_tampered",
            "The finalized MGA execution-cache manifest digest changed.",
            "Do not create an archive-ready result from this cache.",
        ));
    }
    let manifest: MultiModMgaExecutionCacheManifestV1 =
        serde_json::from_slice(&bytes).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.superseded_execution_cache_invalid",
                error.to_string(),
                "Do not create an archive-ready result from this cache.",
            )
        })?;
    validate_mga_execution_manifest_v1(&manifest)?;
    if manifest.cache_id != receipt.cache_id
        || manifest.result_id != result_id
        || manifest.recipe_id != source.staged_recipe.id
        || manifest.source_archive_sha256 != source.archive_sha256
        || manifest.embedded_authority_sha256 != receipt.embedded_authority_sha256
        || manifest.finalized_cache_sha256.is_none()
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.superseded_execution_cache_not_finalized",
            "The MGA execution cache is not the exact finalized shard inventory for this result.",
            "Do not create or publish an archive-ready scientific result.",
        ));
    }
    Ok(())
}

fn remove_archive_ready_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
) -> Result<(), MultiModJobFailureV1> {
    if receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || receipt.stage != MultiModExternalCacheStageV1::ArchiveReady
        || !lowercase_sha256(&receipt.manifest_sha256)
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_receipt_invalid",
            "The retained cache receipt is not the exact archive-ready V1 contract.",
            "Preserve the cache and remove it only through a verified QuickPLS cleanup.",
        ));
    }
    let directory = cache_root_v1()?.join(receipt.cache_id.to_string());
    if directory.to_string_lossy() != receipt.cache_directory {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_path_mismatch",
            "The retained cache path differs from its app-owned identity.",
            "Preserve the cache and inspect its receipt before cleanup.",
        ));
    }
    if !directory.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_unavailable",
            error.to_string(),
            "Preserve the job receipt and retry cleanup from the completed result.",
        )
    })?;
    if !metadata.is_dir() || metadata_is_reparse_v1(&metadata) {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_not_regular",
            "The retained cache directory is not a regular app-owned directory.",
            "Do not follow or delete the redirected path; inspect it manually.",
        ));
    }
    let manifest_path = directory.join("manifest.json");
    let manifest_metadata = fs::symlink_metadata(&manifest_path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_manifest_unavailable",
            error.to_string(),
            "Preserve the cache and retry cleanup from the completed result.",
        )
    })?;
    if !manifest_metadata.is_file() || metadata_is_reparse_v1(&manifest_metadata) {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_manifest_invalid",
            "The retained cache manifest is not a regular file.",
            "Preserve the cache and inspect it before cleanup.",
        ));
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_manifest_unavailable",
            error.to_string(),
            "Preserve the cache and retry cleanup from the completed result.",
        )
    })?;
    if sha256_bytes(&manifest_bytes) != receipt.manifest_sha256 {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_manifest_tampered",
            "The retained cache manifest differs from its receipt.",
            "Do not delete the cache automatically; preserve it for diagnosis.",
        ));
    }
    let manifest: MultiModArchiveReadyCacheManifestV1 = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_manifest_invalid",
                error.to_string(),
                "Do not delete the cache automatically; preserve it for diagnosis.",
            )
        })?;
    if manifest.cache_id != receipt.cache_id
        || manifest.attachment.result_id != receipt.result_id
        || manifest.staged_recipe.id != receipt.recipe_id
        || manifest.source_archive_sha256 != receipt.source_archive_sha256
        || manifest.embedded_authority_sha256 != receipt.embedded_authority_sha256
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_identity_mismatch",
            "The retained cache manifest identities differ from the cleanup receipt.",
            "Do not delete the cache automatically; preserve it for diagnosis.",
        ));
    }
    let mut expected = BTreeMap::new();
    expected.insert(
        "manifest.json".to_owned(),
        (receipt.manifest_sha256.clone(), manifest_bytes.len() as u64),
    );
    for payload in &manifest.payload_files {
        if payload.leaf_name.is_empty()
            || payload.leaf_name.contains('/')
            || payload.leaf_name.contains('\\')
            || payload.leaf_name.contains("..")
            || expected
                .insert(
                    payload.leaf_name.clone(),
                    (payload.sha256.clone(), payload.bytes),
                )
                .is_some()
        {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_inventory_invalid",
                "The retained cache manifest contains an unsafe or duplicate file identity.",
                "Do not delete the cache automatically; preserve it for diagnosis.",
            ));
        }
    }
    let mut observed = BTreeSet::new();
    for entry in fs::read_dir(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_inventory_unavailable",
            error.to_string(),
            "Preserve the cache and retry cleanup from the completed result.",
        )
    })? {
        let entry = entry.map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_inventory_unavailable",
                error.to_string(),
                "Preserve the cache and retry cleanup from the completed result.",
            )
        })?;
        let leaf = entry.file_name().to_string_lossy().into_owned();
        let Some((expected_sha256, expected_bytes)) = expected.get(&leaf) else {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_unexpected_file",
                format!("The retained cache contains unexpected entry {leaf}."),
                "Do not delete the cache automatically; preserve it for diagnosis.",
            ));
        };
        let entry_metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_inventory_unavailable",
                error.to_string(),
                "Preserve the cache and retry cleanup from the completed result.",
            )
        })?;
        if !entry_metadata.is_file()
            || metadata_is_reparse_v1(&entry_metadata)
            || entry_metadata.len() != *expected_bytes
            || sha256_file_for_cache_v1(&entry.path())? != *expected_sha256
        {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_file_mismatch",
                format!("The retained cache entry {leaf} differs from its manifest."),
                "Do not delete the cache automatically; preserve it for diagnosis.",
            ));
        }
        observed.insert(leaf);
    }
    if observed.len() != expected.len() || expected.keys().any(|leaf| !observed.contains(leaf)) {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_inventory_mismatch",
            "The retained cache file inventory differs from its exact manifest.",
            "Do not delete the cache automatically; preserve it for diagnosis.",
        ));
    }
    if let Some(execution_cache) = manifest.superseded_execution_cache.as_ref() {
        if execution_cache.result_id != receipt.result_id
            || execution_cache.recipe_id != receipt.recipe_id
            || execution_cache.source_archive_sha256 != receipt.source_archive_sha256
            || execution_cache.embedded_authority_sha256 != receipt.embedded_authority_sha256
            || execution_cache.target != MultiModCompilerTargetV1::MgaMultigroupV1
            || execution_cache.stage != MultiModExternalCacheStageV1::MgaExecution
        {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_superseded_identity_mismatch",
                "The archive-ready cache binds an invalid superseded MGA execution cache.",
                "Do not delete either cache automatically; preserve them for diagnosis.",
            ));
        }
        remove_mga_execution_cache_v1(execution_cache)?;
    }
    for payload in &manifest.payload_files {
        fs::remove_file(directory.join(&payload.leaf_name)).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_failed",
                error.to_string(),
                "Retry cleanup from the retained terminal job.",
            )
        })?;
    }
    fs::remove_file(&manifest_path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_failed",
            error.to_string(),
            "Retry cleanup from the retained terminal job.",
        )
    })?;
    fs::remove_dir(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_failed",
            error.to_string(),
            "Retry cleanup from the retained terminal job.",
        )
    })
}

fn remove_external_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
) -> Result<(), MultiModJobFailureV1> {
    match receipt.stage {
        MultiModExternalCacheStageV1::MgaExecution => remove_mga_execution_cache_v1(receipt),
        MultiModExternalCacheStageV1::ArchiveReady => remove_archive_ready_cache_v1(receipt),
    }
}

fn sha256_file_for_cache_v1(path: &Path) -> Result<String, MultiModJobFailureV1> {
    let mut file = File::open(path).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_cleanup_file_unavailable",
            error.to_string(),
            "Preserve the cache and retry cleanup from the retained terminal job.",
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_cleanup_file_unavailable",
                error.to_string(),
                "Preserve the cache and retry cleanup from the retained terminal job.",
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), MultiModJobFailureV1> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_create_failed",
                error.to_string(),
                "Keep the result unpublished and retry with a new cache identity.",
            )
        })?;
    file.write_all(bytes).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_write_failed",
            error.to_string(),
            "Keep the result unpublished and verify available local storage.",
        )
    })?;
    file.sync_all().map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_sync_failed",
            error.to_string(),
            "Keep the result unpublished and verify the local cache volume.",
        )
    })
}

struct CacheCreationGuardV1 {
    directory: PathBuf,
    files: Vec<PathBuf>,
    armed: bool,
}

impl CacheCreationGuardV1 {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            files: Vec::new(),
            armed: true,
        }
    }

    fn retained(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CacheCreationGuardV1 {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for path in self.files.iter().rev() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn persist_archive_ready_cache_v1(
    source: &ResolvedMultiModAuthorityV1,
    attachment: &MultiModResultAttachmentV1,
    payloads: &[MultiModSidecarPayloadV1],
    canonical_document: &CanonicalResultDocumentV2,
    superseded_execution_cache: Option<&MultiModExternalCacheReceiptV1>,
) -> Result<MultiModExternalCacheReceiptV1, MultiModJobFailureV1> {
    if let Some(receipt) = superseded_execution_cache {
        validate_finalized_execution_cache_v1(source, &attachment.result_id, receipt)?;
    }
    let cache_id = Uuid::new_v4();
    let directory = cache_root_v1()?.join(cache_id.to_string());
    fs::create_dir(&directory).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_directory_failed",
            error.to_string(),
            "Keep the result unpublished and retry with a new cache identity.",
        )
    })?;
    let mut creation_guard = CacheCreationGuardV1::new(directory.clone());
    let mut payload_files = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let leaf_name = payload
            .descriptor
            .entry_name
            .rsplit('/')
            .next()
            .filter(|value| {
                !value.is_empty()
                    && !value
                        .chars()
                        .any(|character| matches!(character, '\\' | '/'))
            })
            .ok_or_else(|| {
                failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.cache_leaf_invalid",
                    "A sidecar did not have a safe cache leaf name.",
                    "Keep the result unpublished and repair its descriptor.",
                )
            })?
            .to_owned();
        let payload_path = directory.join(&leaf_name);
        write_new_file(&payload_path, &payload.bytes)?;
        creation_guard.retained(payload_path);
        payload_files.push(CachePayloadFileV1 {
            leaf_name,
            sha256: payload.descriptor.sha256.clone(),
            bytes: payload.bytes.len() as u64,
        });
    }
    let created_at = now_utc();
    let embedded_authority_sha256 = current_cache_authority_sha256_v1()?;
    let manifest = MultiModArchiveReadyCacheManifestV1 {
        schema_version: MULTIMOD_CACHE_SCHEMA_VERSION_V1,
        cache_id,
        source_archive_path: source.archive_path.to_string_lossy().into_owned(),
        source_archive_sha256: source.archive_sha256.clone(),
        embedded_authority_sha256: embedded_authority_sha256.clone(),
        staged_recipe: source.staged_recipe.clone(),
        attachment: attachment.clone(),
        canonical_document: canonical_document.clone(),
        payload_files,
        superseded_execution_cache: superseded_execution_cache.cloned(),
        created_at: created_at.clone(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_failed",
            error.to_string(),
            "Keep the result unpublished and repair its archive-ready cache manifest.",
        )
    })?;
    let manifest_path = directory.join("manifest.json");
    write_new_file(&manifest_path, &manifest_bytes)?;
    creation_guard.retained(manifest_path);
    let receipt = MultiModExternalCacheReceiptV1 {
        schema_version: MULTIMOD_CACHE_SCHEMA_VERSION_V1,
        cache_id,
        cache_directory: directory.to_string_lossy().into_owned(),
        manifest_sha256: sha256_bytes(&manifest_bytes),
        embedded_authority_sha256,
        source_archive_sha256: source.archive_sha256.clone(),
        result_id: attachment.result_id.clone(),
        recipe_id: source.staged_recipe.id,
        target: source.runtime_support.target,
        stage: MultiModExternalCacheStageV1::ArchiveReady,
        created_at,
    };
    creation_guard.disarm();
    Ok(receipt)
}

fn load_archive_ready_cache_v1(
    receipt: &MultiModExternalCacheReceiptV1,
    source: &ResolvedMultiModAuthorityV1,
) -> Result<
    (
        MultiModResultAttachmentV1,
        Vec<MultiModSidecarPayloadV1>,
        CanonicalResultDocumentV2,
    ),
    MultiModJobFailureV1,
> {
    if receipt.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || receipt.stage != MultiModExternalCacheStageV1::ArchiveReady
        || receipt.source_archive_sha256 != source.archive_sha256
        || receipt.recipe_id != source.staged_recipe.id
        || receipt.target != source.runtime_support.target
        || !lowercase_sha256(&receipt.manifest_sha256)
        || receipt.embedded_authority_sha256 != current_cache_authority_sha256_v1()?
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_receipt_mismatch",
            "The external cache receipt differs from the exact staged authority.",
            "Discard the receipt and start a new exact run.",
        ));
    }
    let expected_directory =
        cache_directory_for_receipt_v1(receipt, MultiModExternalCacheStageV1::ArchiveReady)?;
    let manifest_bytes = read_regular_cache_manifest_v1(&expected_directory)?;
    if sha256_bytes(&manifest_bytes) != receipt.manifest_sha256 {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_manifest_tampered",
            "The external cache manifest digest changed.",
            "Discard the cache; do not publish or display its result.",
        ));
    }
    let manifest: MultiModArchiveReadyCacheManifestV1 = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_manifest_invalid",
                error.to_string(),
                "Discard the cache; do not publish or display its result.",
            )
        })?;
    if manifest.schema_version != MULTIMOD_CACHE_SCHEMA_VERSION_V1
        || manifest.cache_id != receipt.cache_id
        || manifest.source_archive_path != source.archive_path.to_string_lossy()
        || manifest.source_archive_sha256 != source.archive_sha256
        || manifest.embedded_authority_sha256 != receipt.embedded_authority_sha256
        || sha256_serialized(&manifest.staged_recipe) != sha256_serialized(&source.staged_recipe)
        || manifest.attachment.result_id != receipt.result_id
        || manifest.attachment.recipe_id != receipt.recipe_id
        || manifest.payload_files.len() != manifest.attachment.sidecars.len()
        || manifest.canonical_document.provenance.run_id != manifest.attachment.result_id
        || manifest.canonical_document.provenance.recipe_id
            != manifest.attachment.recipe_id.to_string()
        || manifest.canonical_document.provenance.project_id != source.project_id.to_string()
    {
        return Err(failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_identity_mismatch",
            "The external cache manifest differs from its receipt or staged Recipe V4.",
            "Discard the cache; do not publish or display its result.",
        ));
    }
    if let Some(execution_cache) = manifest.superseded_execution_cache.as_ref() {
        validate_superseded_execution_cache_binding_v1(
            source,
            &manifest.attachment.result_id,
            execution_cache,
        )?;
    }
    manifest.attachment.ensure_valid().map_err(|error| {
        failure(
            MultiModFailureStageV1::Cache,
            "multimod.job.cache_attachment_invalid",
            error.to_string(),
            "Discard the cache; do not publish or display its result.",
        )
    })?;
    manifest
        .canonical_document
        .ensure_valid()
        .map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_canonical_invalid",
                error.to_string(),
                "Discard the cache; do not publish or display its canonical result.",
            )
        })?;
    let mut payloads = Vec::with_capacity(manifest.payload_files.len());
    for file in &manifest.payload_files {
        if file
            .leaf_name
            .chars()
            .any(|character| matches!(character, '\\' | '/'))
            || file.leaf_name == "manifest.json"
        {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_payload_path_invalid",
                "A cached sidecar filename is unsafe.",
                "Discard the cache; do not publish or display its result.",
            ));
        }
        let payload_path = expected_directory.join(&file.leaf_name);
        let payload_metadata = fs::symlink_metadata(&payload_path).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_payload_unavailable",
                error.to_string(),
                "Discard the incomplete cache and start a new run.",
            )
        })?;
        if !payload_metadata.is_file() || metadata_is_reparse_v1(&payload_metadata) {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_payload_not_regular",
                "A cached sidecar is missing, redirected, or not a regular file.",
                "Discard the cache; do not publish or display its result.",
            ));
        }
        let bytes = fs::read(payload_path).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_payload_unavailable",
                error.to_string(),
                "Discard the incomplete cache and start a new run.",
            )
        })?;
        let descriptor = manifest
            .attachment
            .sidecars
            .iter()
            .find(|value| value.entry_name.ends_with(&format!("/{}", file.leaf_name)))
            .cloned()
            .ok_or_else(|| {
                failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.cache_descriptor_missing",
                    "A cached sidecar is not present in the result descriptor inventory.",
                    "Discard the cache; do not publish or display its result.",
                )
            })?;
        if bytes.len() as u64 != file.bytes || sha256_bytes(&bytes) != file.sha256 {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.cache_payload_tampered",
                "A cached sidecar digest or byte count changed.",
                "Discard the cache; do not publish or display its result.",
            ));
        }
        let payload = MultiModSidecarPayloadV1 { descriptor, bytes };
        validate_multimod_sidecar_payload_v1(&manifest.attachment.result_id, &payload).map_err(
            |error| {
                failure(
                    MultiModFailureStageV1::Cache,
                    "multimod.job.cache_payload_invalid",
                    error.to_string(),
                    "Discard the cache; do not publish or display its result.",
                )
            },
        )?;
        payloads.push(payload);
    }
    Ok((manifest.attachment, payloads, manifest.canonical_document))
}

fn set_progress(
    jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>,
    job_id: Uuid,
    progress: MultiModRunnerProgressV1,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == MultiModJobStateV1::Queued {
            job.snapshot.state = MultiModJobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        if matches!(
            job.snapshot.state,
            MultiModJobStateV1::Running | MultiModJobStateV1::Cancelling
        ) {
            job.snapshot.phase = progress.phase.stable_id().into();
            job.snapshot.shard_id = progress.shard_id;
            job.snapshot.completed_units = progress.completed_units.min(progress.total_units);
            job.snapshot.total_units = progress.total_units.max(1);
        }
    }
}

fn set_publishing(
    jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>,
    job_id: Uuid,
    cache: MultiModExternalCacheReceiptV1,
    warn_large: bool,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.snapshot.resume_cache = Some(cache);
        if warn_large {
            job.snapshot
                .warning_codes
                .push("multimod.job.sidecar_above_128_mib".into());
        }
        job.snapshot.state = MultiModJobStateV1::Publishing;
        job.snapshot.phase = "multimod_archive_publication".into();
        job.snapshot.shard_id = "multimod:archive_publication".into();
        job.snapshot.completed_units = 0;
        job.snapshot.total_units = 1;
    }
}

fn set_mga_execution_cache_v1(
    jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>,
    job_id: Uuid,
    receipt: MultiModExternalCacheReceiptV1,
    cache: &MgaExecutionCacheV1,
    plan: &MgaExecutionPlanV1,
    message: &str,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == MultiModJobStateV1::Queued {
            job.snapshot.state = MultiModJobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        job.snapshot.phase = "multimod_mga_execution".into();
        job.snapshot.shard_id = "mga:execution_cache".into();
        job.snapshot.completed_units = u64::try_from(cache.entries.len()).unwrap_or(u64::MAX);
        job.snapshot.total_units = u64::try_from(plan.shards.len()).unwrap_or(u64::MAX).max(1);
        job.snapshot.message = Some(message.into());
        job.snapshot.resume_cache = Some(receipt);
    }
}

fn set_nonresumable_execution_notice_v1(jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>, job_id: Uuid) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.snapshot.message = Some(
            "This profile does not yet support intra-estimation resume; cancellation before archive-ready publication restarts the exact execution."
                .into(),
        );
    }
}

enum NativeMgaExecutionOutcomeV1 {
    Completed {
        output: MultiModRunOutputV1,
        cache_receipt: MultiModExternalCacheReceiptV1,
    },
    Cancelled,
}

fn execute_resumable_mga_v1(
    job_id: Uuid,
    resume_receipt: Option<&MultiModExternalCacheReceiptV1>,
    resolved: &ResolvedMultiModAuthorityV1,
    cancellation: &AtomicBool,
    jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>,
) -> Result<NativeMgaExecutionOutcomeV1, MultiModJobFailureV1> {
    if resolved.runtime_support.target != MultiModCompilerTargetV1::MgaMultigroupV1 {
        return Err(failure(
            MultiModFailureStageV1::RuntimeSupport,
            "multimod.job.mga_execution_target_mismatch",
            "An MGA execution cache was supplied to a different MultiMod family.",
            "Discard the mismatched receipt and stage the exact requested family again.",
        ));
    }
    let config = resolved
        .staged_recipe
        .mga_multigroup
        .as_ref()
        .ok_or_else(|| {
            failure(
                MultiModFailureStageV1::Compilation,
                "multimod.job.mga_config_missing",
                "The compiled MGA recipe lost its versioned configuration.",
                "Keep the request unpublished and rebuild it from strict authority.",
            )
        })?;
    let (design, excluded_rows) =
        prepare_mga_design_v1(&resolved.dataset, &resolved.model, config)?;
    let plan = prepare_compiled_raw_mga_execution_plan_v1(
        &resolved.dataset,
        &resolved.staged_recipe,
        &resolved.model,
        &resolved.artifact,
        &design,
    )
    .map_err(map_runner_error)?;
    let result_id = resume_receipt
        .map(|receipt| receipt.result_id.clone())
        .unwrap_or_else(|| format!("qpls-multimod-{job_id}"));
    let (mut cache, mut cache_receipt) = if let Some(receipt) = resume_receipt {
        if receipt.stage != MultiModExternalCacheStageV1::MgaExecution {
            return Err(failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_resume_stage_mismatch",
                "The requested resume receipt is not an MGA execution cache.",
                "Resume archive publication through its archive-ready stage, or supply the exact MGA execution receipt.",
            ));
        }
        let (cache, _) = load_mga_execution_cache_v1(receipt, resolved, &plan)?;
        (cache, receipt.clone())
    } else {
        let cache = MgaExecutionCacheV1::empty(&plan).map_err(|error| {
            failure(
                MultiModFailureStageV1::Cache,
                "multimod.job.mga_execution_cache_initialization_failed",
                error.to_string(),
                "Keep the result unpublished and rebuild the exact MGA execution plan.",
            )
        })?;
        let receipt =
            create_mga_execution_cache_v1(resolved, result_id, plan.clone(), cache.clone())?;
        (cache, receipt)
    };
    set_mga_execution_cache_v1(
        jobs,
        job_id,
        cache_receipt.clone(),
        &cache,
        &plan,
        "The validated MGA execution cache can resume completed immutable shards after cancellation.",
    );
    if cancellation.load(Ordering::Acquire) {
        cache_receipt =
            update_mga_execution_cache_v1(&cache_receipt, resolved, &plan, &cache, None)?;
        set_mga_execution_cache_v1(
            jobs,
            job_id,
            cache_receipt,
            &cache,
            &plan,
            "Cancellation preserved the validated completed MGA shard inventory.",
        );
        return Ok(NativeMgaExecutionOutcomeV1::Cancelled);
    }
    let mut checkpoint_receipt = cache_receipt.clone();
    let execution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_compiled_raw_mga_resumable_with_checkpoint_v1(
            &resolved.dataset,
            &resolved.staged_recipe,
            &resolved.model,
            &resolved.artifact,
            &design,
            &excluded_rows,
            &mut cache,
            || cancellation.load(Ordering::Acquire),
            |progress| set_progress(jobs, job_id, progress),
            |checkpoint_plan, checkpoint_cache| {
                if checkpoint_plan != &plan {
                    return Err("native checkpoint received a different MGA execution plan".into());
                }
                let updated = update_mga_execution_cache_v1(
                    &checkpoint_receipt,
                    resolved,
                    checkpoint_plan,
                    checkpoint_cache,
                    None,
                )
                .map_err(|error| format!("{}: {}", error.code, error.message))?;
                checkpoint_receipt = updated.clone();
                set_mga_execution_cache_v1(
                    jobs,
                    job_id,
                    updated,
                    checkpoint_cache,
                    checkpoint_plan,
                    "A newly completed immutable MGA shard was validated and durably checkpointed.",
                );
                Ok(())
            },
        )
    }));
    cache_receipt = checkpoint_receipt;
    match execution {
        Ok(Ok(completed)) => {
            if completed.execution_plan != plan {
                cache_receipt =
                    update_mga_execution_cache_v1(&cache_receipt, resolved, &plan, &cache, None)?;
                set_mga_execution_cache_v1(
                    jobs,
                    job_id,
                    cache_receipt,
                    &cache,
                    &plan,
                    "The runner plan identity changed; only the prior validated shard cache remains resumable.",
                );
                return Err(failure(
                    MultiModFailureStageV1::Integrity,
                    "multimod.job.mga_execution_plan_changed",
                    "The runner returned a different MGA execution-plan identity.",
                    "Do not publish a result; rebuild the plan from strict archive authority.",
                ));
            }
            cache_receipt = update_mga_execution_cache_v1(
                &cache_receipt,
                resolved,
                &plan,
                &cache,
                Some(completed.finalized_cache_sha256),
            )?;
            set_mga_execution_cache_v1(
                jobs,
                job_id,
                cache_receipt.clone(),
                &cache,
                &plan,
                "Every planned MGA shard is finalized; the scientific result is being validated before publication.",
            );
            Ok(NativeMgaExecutionOutcomeV1::Completed {
                output: completed.output,
                cache_receipt,
            })
        }
        Ok(Err(MultiModRunnerErrorV1::Cancelled)) => {
            // The runner mutates this cache only after a complete immutable
            // shard. Persist that exact prefix atomically before returning the
            // terminal cancellation state.
            cache_receipt =
                update_mga_execution_cache_v1(&cache_receipt, resolved, &plan, &cache, None)?;
            set_mga_execution_cache_v1(
                jobs,
                job_id,
                cache_receipt,
                &cache,
                &plan,
                "Cancellation preserved the validated completed MGA shard inventory.",
            );
            Ok(NativeMgaExecutionOutcomeV1::Cancelled)
        }
        Ok(Err(error)) => {
            cache_receipt =
                update_mga_execution_cache_v1(&cache_receipt, resolved, &plan, &cache, None)?;
            set_mga_execution_cache_v1(
                jobs,
                job_id,
                cache_receipt,
                &cache,
                &plan,
                "Execution failed without publication; validated completed MGA shards remain resumable.",
            );
            Err(map_runner_error(error))
        }
        Err(_) => {
            cache_receipt =
                update_mga_execution_cache_v1(&cache_receipt, resolved, &plan, &cache, None)?;
            set_mga_execution_cache_v1(
                jobs,
                job_id,
                cache_receipt,
                &cache,
                &plan,
                "The worker stopped unexpectedly; only validated completed MGA shards remain resumable.",
            );
            Err(failure(
                MultiModFailureStageV1::Integrity,
                "multimod.job.worker_panicked",
                "The MultiMod MGA worker terminated unexpectedly.",
                "Restart QuickPLS and resume only from the validated execution-cache receipt.",
            ))
        }
    }
}

fn finish_failed(
    jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>,
    job_id: Uuid,
    terminal: MultiModJobFailureV1,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = MultiModJobStateV1::Failed;
        job.snapshot.phase = "multimod_failed".into();
        job.snapshot.shard_id = "multimod:failed".into();
        job.snapshot.message = Some(terminal.message.clone());
        job.snapshot.failure = Some(terminal);
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn finish_cancelled(jobs: &Mutex<HashMap<Uuid, MultiModJobV1>>, job_id: Uuid) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = MultiModJobStateV1::Cancelled;
        job.snapshot.phase = "multimod_cancelled".into();
        job.snapshot.shard_id = "multimod:cancelled".into();
        job.snapshot.message = job.snapshot.resume_cache.as_ref().map(|receipt| {
            if receipt.stage == MultiModExternalCacheStageV1::MgaExecution {
                "Cancelled without publication; validated completed MGA shards may be resumed."
                    .into()
            } else {
                "Cancelled without publication; the archive-ready external cache may be resumed."
                    .into()
            }
        });
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn map_runner_error(error: MultiModRunnerErrorV1) -> MultiModJobFailureV1 {
    failure(
        if matches!(error, MultiModRunnerErrorV1::UnsupportedProfile(_)) {
            MultiModFailureStageV1::RuntimeSupport
        } else {
            MultiModFailureStageV1::Estimation
        },
        error.stable_code(),
        error.to_string(),
        "Keep the archive unchanged and resolve the exact typed runner blocker.",
    )
}

fn run_worker_v1(
    job_id: Uuid,
    request: MultiModJobRequestV1,
    resolved: ResolvedMultiModAuthorityV1,
    cancellation: Arc<AtomicBool>,
    jobs: Arc<Mutex<HashMap<Uuid, MultiModJobV1>>>,
    _admission: PlsModelComparisonAdmissionReservationV1,
) {
    let archive_ready = match request.resume_cache.as_ref().map(|receipt| receipt.stage) {
        Some(MultiModExternalCacheStageV1::ArchiveReady) => {
            let receipt = request
                .resume_cache
                .as_ref()
                .expect("matched receipt stage");
            match load_archive_ready_cache_v1(receipt, &resolved) {
                Ok((attachment, payloads, canonical_document)) => (
                    attachment,
                    payloads,
                    canonical_document,
                    false,
                    receipt.clone(),
                ),
                Err(error) => {
                    finish_failed(&jobs, job_id, error);
                    return;
                }
            }
        }
        None | Some(MultiModExternalCacheStageV1::MgaExecution) => {
            let (output, result_id, execution_cache_receipt) = if resolved.runtime_support.target
                == MultiModCompilerTargetV1::MgaMultigroupV1
            {
                match execute_resumable_mga_v1(
                    job_id,
                    request.resume_cache.as_ref(),
                    &resolved,
                    &cancellation,
                    &jobs,
                ) {
                    Ok(NativeMgaExecutionOutcomeV1::Completed {
                        output,
                        cache_receipt,
                    }) => (output, cache_receipt.result_id.clone(), Some(cache_receipt)),
                    Ok(NativeMgaExecutionOutcomeV1::Cancelled) => {
                        finish_cancelled(&jobs, job_id);
                        return;
                    }
                    Err(error) => {
                        finish_failed(&jobs, job_id, error);
                        return;
                    }
                }
            } else {
                if request.resume_cache.is_some() {
                    finish_failed(
                        &jobs,
                        job_id,
                        failure(
                            MultiModFailureStageV1::RuntimeSupport,
                            "multimod.job.execution_resume_mga_only",
                            "Intra-estimation resume is currently implemented only for MGA.",
                            "Discard the mismatched execution receipt and restart this exact non-MGA profile.",
                        ),
                    );
                    return;
                }
                set_nonresumable_execution_notice_v1(&jobs, job_id);
                let execution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute_builtin_v1(&resolved, &cancellation, |progress| {
                        set_progress(&jobs, job_id, progress)
                    })
                }));
                let output = match execution {
                    Ok(Ok(output)) => output,
                    Ok(Err(MultiModRunnerErrorV1::Cancelled)) => {
                        finish_cancelled(&jobs, job_id);
                        return;
                    }
                    Ok(Err(error)) => {
                        finish_failed(&jobs, job_id, map_runner_error(error));
                        return;
                    }
                    Err(_) => {
                        finish_failed(
                            &jobs,
                            job_id,
                            failure(
                                MultiModFailureStageV1::Integrity,
                                "multimod.job.worker_panicked",
                                "The MultiMod worker terminated unexpectedly.",
                                "Restart QuickPLS and retry from the unchanged strict archive.",
                            ),
                        );
                        return;
                    }
                };
                (output, format!("qpls-multimod-{job_id}"), None)
            };
            if cancellation.load(Ordering::Acquire) {
                finish_cancelled(&jobs, job_id);
                return;
            }
            let (attachment, payloads, warn_large) = match archive_ready_output_v1(
                &result_id,
                resolved.staged_recipe.id,
                &resolved.dataset,
                &resolved.staged_recipe,
                &resolved.artifact,
                output,
                request.surface == MULTIMOD_STANDARD_SURFACE_V1,
            ) {
                Ok(value) => value,
                Err(error) => {
                    finish_failed(&jobs, job_id, error);
                    return;
                }
            };
            let canonical_completed_at = now_utc();
            let canonical_started_at = jobs
                .lock()
                .ok()
                .and_then(|jobs| {
                    jobs.get(&job_id)
                        .and_then(|job| job.snapshot.started_at.clone())
                })
                .unwrap_or_else(|| request.staged_created_at.clone());
            let Ok(canonical_workers) =
                u32::try_from(resolved.staged_recipe.settings.workers.max(1))
            else {
                finish_failed(
                    &jobs,
                    job_id,
                    failure(
                        MultiModFailureStageV1::Evidence,
                        "multimod.job.worker_count_unrepresentable",
                        "The staged worker count cannot be represented by CanonicalResultDocumentV2.",
                        "Keep the archive unchanged and select a supported worker count.",
                    ),
                );
                return;
            };
            let canonical_core = match build_multimod_canonical_result_v2(
                &MultiModCanonicalRunContextV1 {
                    run_id: attachment.result_id.clone(),
                    project_id: resolved.project_id.to_string(),
                    recipe_id: resolved.artifact.receipt().recipe_id.clone(),
                    recipe_analytical_sha256: resolved
                        .artifact
                        .receipt()
                        .recipe_analytical_sha256
                        .clone(),
                    model_id: request.model_id.clone(),
                    model_scientific_sha256: request.model_scientific_sha256.clone(),
                    dataset_id: resolved.dataset.id.to_string(),
                    dataset_fingerprint: request.dataset_fingerprint.clone(),
                    engine_version: qpls_runner::MULTIMOD_RUNNER_METHOD_VERSION_V1.into(),
                    workers: canonical_workers,
                    started_at: canonical_started_at,
                    completed_at: canonical_completed_at,
                },
                &attachment.result,
            ) {
                Ok(document) => document,
                Err(error) => {
                    finish_failed(
                        &jobs,
                        job_id,
                        failure(
                            MultiModFailureStageV1::Evidence,
                            "multimod.job.canonical_projection_failed",
                            format!(
                                "The completed result could not be projected before archive publication: {error}"
                            ),
                            "Keep the archive unchanged and repair the canonical projection contract.",
                        ),
                    );
                    return;
                }
            };
            let canonical_document: CanonicalResultDocumentV2 = match serde_json::to_value(
                canonical_core,
            )
            .and_then(serde_json::from_value)
            {
                Ok(document) => document,
                Err(error) => {
                    finish_failed(
                        &jobs,
                        job_id,
                        failure(
                            MultiModFailureStageV1::Evidence,
                            "multimod.job.canonical_wire_conversion_failed",
                            format!(
                                "The validated canonical result could not be converted to the Archive V6 wire contract: {error}"
                            ),
                            "Keep the archive unchanged and repair the canonical wire adapter.",
                        ),
                    );
                    return;
                }
            };
            let cache = match persist_archive_ready_cache_v1(
                &resolved,
                &attachment,
                &payloads,
                &canonical_document,
                execution_cache_receipt.as_ref(),
            ) {
                Ok(value) => value,
                Err(error) => {
                    finish_failed(&jobs, job_id, error);
                    return;
                }
            };
            (attachment, payloads, canonical_document, warn_large, cache)
        }
    };
    let (attachment, payloads, canonical_document, warn_large, cache) = archive_ready;
    if request.surface == MULTIMOD_STANDARD_SURFACE_V1
        && attachment.result.provenance().qualification
            != MultimodQualificationStateV1::ReleaseQualifiedCandidate
    {
        finish_failed(
            &jobs,
            job_id,
            failure(
                MultiModFailureStageV1::Integrity,
                "multimod.job.standard_result_not_qualified",
                "The archive-ready result is not release-qualified by this executable's immutable candidate authority.",
                "Do not publish the cached result; restart only with an exact covered candidate profile.",
            ),
        );
        return;
    }
    set_publishing(&jobs, job_id, cache.clone(), warn_large);
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    let receipt = match append_multimod_recipe_result_and_canonical_to_archive_v6(
        &resolved.archive_path,
        &resolved.archive_sha256,
        resolved.staged_recipe,
        attachment.clone(),
        payloads,
        canonical_document.clone(),
        || cancellation.load(Ordering::Acquire),
    ) {
        Ok(receipt) => receipt,
        Err(qpls_project::MultiModArchiveErrorV1::Cancelled) => {
            finish_cancelled(&jobs, job_id);
            return;
        }
        Err(error) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    MultiModFailureStageV1::Publication,
                    "multimod.job.archive_publication_failed",
                    error.to_string(),
                    "Inspect the typed publication or rollback error, preserve the archive and any private rollback copy, then resume only from the verified archive-ready cache.",
                ),
            );
            return;
        }
    };
    if cancellation.load(Ordering::Acquire) {
        // The append boundary itself checks cancellation before commit. Once
        // committed, the result is complete and must not be relabelled partial.
    }
    let reopened = match load_project_archive_v6(&resolved.archive_path) {
        Ok(value) => value,
        Err(error) => {
            finish_failed(
                &jobs,
                job_id,
                failure(
                    MultiModFailureStageV1::Reopen,
                    "multimod.job.post_write_reopen_failed",
                    error.to_string(),
                    "Do not display the result. Restore or strictly reopen the written archive.",
                ),
            );
            return;
        }
    };
    let Some(persisted) = reopened
        .document
        .multimod_results
        .iter()
        .find(|value| value.result_id == attachment.result_id)
    else {
        finish_failed(
            &jobs,
            job_id,
            failure(
                MultiModFailureStageV1::Reopen,
                "multimod.job.result_absent_after_reopen",
                "The appended result was absent after strict reopen.",
                "Do not display the result; preserve the archive for diagnosis.",
            ),
        );
        return;
    };
    let persisted_canonical = reopened
        .document
        .canonical_result_documents
        .iter()
        .find(|value| value.run_id() == attachment.result_id);
    if persisted != &attachment
        || persisted_canonical.map(|value| value.canonical_document()) != Some(&canonical_document)
        || !receipt.post_write_validated
    {
        finish_failed(
            &jobs,
            job_id,
            failure(
                MultiModFailureStageV1::Reopen,
                "multimod.job.result_changed_after_reopen",
                "The freshly reopened MultiMod attachment or archive digest changed.",
                "Do not display or export the result; preserve the archive for diagnosis.",
            ),
        );
        return;
    }
    if !receipt.rollback_removed
        && let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.snapshot
            .warning_codes
            .push("multimod.job.rollback_cleanup_pending".into());
    }
    let cache_removed_after_commit = match remove_archive_ready_cache_v1(&cache) {
        Ok(()) => true,
        Err(_) => {
            if let Ok(mut jobs) = jobs.lock()
                && let Some(job) = jobs.get_mut(&job_id)
            {
                job.snapshot
                    .warning_codes
                    .push("multimod.job.cache_cleanup_pending".into());
            }
            false
        }
    };
    let completed_at = canonical_document.provenance.completed_at.clone();
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = Some(MultiModCompletedResultV1 {
            schema_version: MULTIMOD_JOB_SCHEMA_VERSION_V1,
            job_id,
            archive_path: resolved.archive_path.to_string_lossy().into_owned(),
            archive_sha256: receipt.updated_archive_sha256.clone(),
            project_id: resolved.project_id,
            dataset_id: resolved.dataset.id,
            model_id: request.model_id.clone(),
            attachment,
            canonical_document,
            append_receipt: receipt,
            cache_receipt: cache,
            cache_removed_after_commit,
        });
        job.snapshot.state = MultiModJobStateV1::Completed;
        job.snapshot.phase = "multimod_completed".into();
        job.snapshot.shard_id = "multimod:completed".into();
        job.snapshot.completed_units = 1;
        job.snapshot.total_units = 1;
        job.snapshot.message = None;
        job.snapshot.failure = None;
        if cache_removed_after_commit {
            job.snapshot.resume_cache = None;
        }
        job.snapshot.completed_at = Some(completed_at);
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, MultiModJobV1>) {
    if jobs.len() < MAXIMUM_RETAINED_MULTIMOD_JOBS_V1 {
        return;
    }
    let remove = jobs
        .iter()
        .filter(|(_, job)| {
            job.snapshot.state.is_terminal()
                && job.snapshot.resume_cache.is_none()
                && job
                    .result
                    .as_ref()
                    .is_none_or(|result| result.cache_removed_after_commit)
        })
        .min_by_key(|(_, job)| job.snapshot.completed_at.as_deref().unwrap_or(""))
        .map(|(job_id, _)| *job_id);
    if let Some(job_id) = remove {
        jobs.remove(&job_id);
    }
}

#[tauri::command]
pub(crate) fn start_internal_labs_multimod_job_v1(
    request: MultiModJobRequestV1,
    standard_jobs: State<'_, DesktopJobs>,
    shared_jobs: State<'_, DesktopRecipeV4Jobs>,
    jobs: State<'_, DesktopMultiModJobsV1>,
) -> Result<MultiModJobSnapshotV1, MultiModJobFailureV1> {
    let resolved = resolve_authority_v1(&request)?;
    if resolved.runtime_support.readiness != MultiModRuntimeReadinessV1::BuiltInFromDataset {
        return Err(failure(
            MultiModFailureStageV1::RuntimeSupport,
            "multimod.job.runtime_not_built_in",
            format!(
                "The exact profile is not executable by the native dataset adapter: {}",
                resolved.runtime_support.stable_reason_codes.join(",")
            ),
            "Keep the configuration staged in Labs and wait for its complete refit adapter.",
        ));
    }
    if let Some(eligibility) = resolved.mga_group_eligibility.as_ref()
        && !eligibility.eligible
    {
        return Err(failure(
            MultiModFailureStageV1::RuntimeSupport,
            "multimod.job.mga_group_eligibility_blocked",
            format!(
                "The exact MGA grouping design is ineligible: {}",
                eligibility.blocker_codes.join(",")
            ),
            "Resolve the native group-size, imbalance, or frequency-count blocker and stage a new exact request.",
        ));
    }
    let job_id = Uuid::new_v4();
    let admission = reserve_multimod_admission_v1(
        job_id,
        resolved.staged_recipe.settings.workers.max(1),
        standard_jobs.0.clone(),
        shared_jobs.inner().clone(),
    )
    .map_err(|error| {
        failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.admission_blocked",
            error.message,
            "Wait for another analysis to finish or reduce the configured worker count.",
        )
    })?;
    let snapshot = MultiModJobSnapshotV1::queued(job_id, resolved.runtime_support.target);
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut state = jobs.0.lock().map_err(|_| job_state_failure())?;
        prune_terminal_jobs(&mut state);
        state.insert(
            job_id,
            MultiModJobV1 {
                snapshot: snapshot.clone(),
                cancellation: cancellation.clone(),
                result: None,
            },
        );
    }
    let worker_jobs = jobs.0.clone();
    let cleanup_jobs = jobs.0.clone();
    std::thread::Builder::new()
        .name(format!("qpls-multimod-v1-{job_id}"))
        .spawn(move || {
            run_worker_v1(
                job_id,
                request,
                resolved,
                cancellation,
                worker_jobs,
                admission,
            )
        })
        .map_err(|error| {
            if let Ok(mut state) = cleanup_jobs.lock() {
                state.remove(&job_id);
            }
            failure(
                MultiModFailureStageV1::Integrity,
                "multimod.job.worker_spawn_failed",
                error.to_string(),
                "Wait for other local analyses to finish and retry.",
            )
        })?;
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn status_internal_labs_multimod_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopMultiModJobsV1>,
) -> Result<MultiModJobSnapshotV1, MultiModJobFailureV1> {
    jobs.0
        .lock()
        .map_err(|_| job_state_failure())?
        .get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| unknown_job(job_id))
}

#[tauri::command]
pub(crate) fn cancel_internal_labs_multimod_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopMultiModJobsV1>,
) -> Result<MultiModJobSnapshotV1, MultiModJobFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| unknown_job(job_id))?;
    if job.snapshot.state.is_active() {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = MultiModJobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn dismiss_internal_labs_multimod_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopMultiModJobsV1>,
) -> Result<(), MultiModJobFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let job = jobs.get(&job_id).ok_or_else(|| unknown_job(job_id))?;
    let terminal = job.snapshot.state.is_terminal();
    if !terminal {
        return Err(failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.active_job_cannot_be_dismissed",
            "An active MultiMod job cannot be dismissed.",
            "Cancel it or wait for a terminal state first.",
        ));
    }
    let retained_cache = job.snapshot.resume_cache.clone().or_else(|| {
        job.result
            .as_ref()
            .filter(|result| !result.cache_removed_after_commit)
            .map(|result| result.cache_receipt.clone())
    });
    if let Some(cache) = retained_cache.as_ref() {
        remove_external_cache_v1(cache)?;
    }
    jobs.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn result_internal_labs_multimod_job_v1(
    job_id: Uuid,
    jobs: State<'_, DesktopMultiModJobsV1>,
) -> Result<MultiModCompletedResultV1, MultiModJobFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| job_state_failure())?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| unknown_job(job_id))?;
    if job.snapshot.state != MultiModJobStateV1::Completed {
        return Err(failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.result_not_complete",
            "A scientific result is available only after successful archive publication.",
            "Wait for completion or inspect the terminal failure.",
        ));
    }
    let completed = job.result.as_ref().ok_or_else(|| {
        failure(
            MultiModFailureStageV1::Integrity,
            "multimod.job.completed_result_missing",
            "The completed job did not retain its strict result receipt.",
            "Discard the job and rerun from the unchanged archive.",
        )
    })?;
    if sha256_file(Path::new(&completed.archive_path))? != completed.archive_sha256 {
        job.result = None;
        job.snapshot.state = MultiModJobStateV1::Failed;
        job.snapshot.failure = Some(failure(
            MultiModFailureStageV1::Reopen,
            "multimod.job.completed_archive_changed",
            "The completed result archive changed before result retrieval.",
            "Strictly reopen the archive; do not display the cached result.",
        ));
        return Err(job.snapshot.failure.clone().unwrap());
    }
    let reopened =
        load_project_archive_v6(Path::new(&completed.archive_path)).map_err(|error| {
            failure(
                MultiModFailureStageV1::Reopen,
                "multimod.job.completed_reopen_failed",
                error.to_string(),
                "Do not display the result; restore or strictly reopen the archive.",
            )
        })?;
    let exact = reopened
        .document
        .multimod_results
        .iter()
        .find(|value| value.result_id == completed.attachment.result_id)
        == Some(&completed.attachment);
    if !exact {
        return Err(failure(
            MultiModFailureStageV1::Reopen,
            "multimod.job.completed_attachment_changed",
            "The result attachment changed or disappeared on strict reopen.",
            "Do not display or export the result; preserve the archive for diagnosis.",
        ));
    }
    Ok(completed.clone())
}

fn job_state_failure() -> MultiModJobFailureV1 {
    failure(
        MultiModFailureStageV1::Integrity,
        "multimod.job.state_unavailable",
        "The MultiMod job state is temporarily unavailable.",
        "Retry after current local analyses finish.",
    )
}

fn unknown_job(job_id: Uuid) -> MultiModJobFailureV1 {
    failure(
        MultiModFailureStageV1::Integrity,
        "multimod.job.unknown_job",
        format!("Unknown MultiMod job {job_id}."),
        "Start a new exact archive-bound job.",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MULTIMOD_INTERNAL_LABS_SURFACE_V1, MULTIMOD_STANDARD_SURFACE_V1,
        MultiModExternalCacheReceiptV1, MultiModExternalCacheStageV1, NativeMgaEngineV1,
        native_mga_engine_v1, persist_multimod_execution_surface_v1,
        validate_multimod_runtime_surface_access_using_v1,
        validate_multimod_runtime_surface_access_v1,
    };
    use crate::multimod_candidate_authority_v1::with_typed_qualification_test_authority_v1;
    use qpls_core::{MgaModelProfileV1, MultiModCompilerTargetV1};
    use uuid::Uuid;

    #[test]
    fn every_advertised_mga_profile_dispatches_to_its_exact_refitter_family() {
        for profile in [
            MgaModelProfileV1::GeneralSemPls,
            MgaModelProfileV1::CaseWeightedPls,
            MgaModelProfileV1::ReflectivePlsc,
        ] {
            assert_eq!(native_mga_engine_v1(profile), NativeMgaEngineV1::Ordinary);
        }
        assert_eq!(
            native_mga_engine_v1(MgaModelProfileV1::FrequencyWeightedPls),
            NativeMgaEngineV1::FrequencyWeighted
        );
        for profile in [
            MgaModelProfileV1::MultipleTwoWayModeration,
            MgaModelProfileV1::BoundedThreeWayModeration,
            MgaModelProfileV1::BoundedTwoWayModeratedMediation,
        ] {
            assert_eq!(
                native_mga_engine_v1(profile),
                NativeMgaEngineV1::Interaction
            );
        }
        assert_eq!(
            native_mga_engine_v1(MgaModelProfileV1::MultipleNonnestedHoc),
            NativeMgaEngineV1::MultipleHoc
        );
    }

    #[test]
    fn execution_cache_stage_is_typed_and_unknown_stages_fail_closed() {
        let receipt = MultiModExternalCacheReceiptV1 {
            schema_version: 1,
            cache_id: Uuid::from_u128(1),
            cache_directory: r"C:\QuickPLS\multimod-cache-v1\cache".into(),
            manifest_sha256: "a".repeat(64),
            embedded_authority_sha256: "c".repeat(64),
            source_archive_sha256: "b".repeat(64),
            result_id: "qpls-multimod-result".into(),
            recipe_id: Uuid::from_u128(2),
            target: MultiModCompilerTargetV1::MgaMultigroupV1,
            stage: MultiModExternalCacheStageV1::MgaExecution,
            created_at: "2026-08-24T10:00:00.000Z".into(),
        };
        let mut value = serde_json::to_value(&receipt).expect("serialize typed cache receipt");
        assert_eq!(value["stage"], "mga_execution");
        value["stage"] = serde_json::Value::String("unknown".into());
        assert!(serde_json::from_value::<MultiModExternalCacheReceiptV1>(value).is_err());
    }

    #[test]
    fn standard_surface_requires_release_qualified_embedded_authority() {
        let mixed_surface = validate_multimod_runtime_surface_access_using_v1(
            MULTIMOD_STANDARD_SURFACE_V1,
            true,
            || -> Result<bool, String> {
                panic!("an invalid Standard/Labs pair must not consult authority")
            },
        )
        .unwrap_err();
        assert_eq!(mixed_surface.code, "multimod.job.surface_invalid");

        let labs_disabled = validate_multimod_runtime_surface_access_using_v1(
            MULTIMOD_INTERNAL_LABS_SURFACE_V1,
            false,
            || -> Result<bool, String> { panic!("Labs denial must not consult authority") },
        )
        .unwrap_err();
        assert_eq!(labs_disabled.code, "multimod.job.internal_labs_required");

        let denied = validate_multimod_runtime_surface_access_using_v1(
            MULTIMOD_STANDARD_SURFACE_V1,
            false,
            || Ok(false),
        )
        .unwrap_err();
        assert_eq!(denied.code, "multimod.job.standard_authority_required");

        with_typed_qualification_test_authority_v1(
            &["conditional.multi_two_way_percentile.v2::explicit_path_target_math"],
            |_| {
                assert!(
                    validate_multimod_runtime_surface_access_v1(
                        MULTIMOD_STANDARD_SURFACE_V1,
                        false,
                    )
                    .unwrap()
                );
            },
        )
        .unwrap();
    }

    #[test]
    fn labs_surface_still_requires_explicit_opt_in_and_never_reads_standard_authority() {
        let denied = validate_multimod_runtime_surface_access_using_v1(
            MULTIMOD_INTERNAL_LABS_SURFACE_V1,
            false,
            || -> Result<bool, String> { panic!("Labs denial must not consult authority") },
        )
        .unwrap_err();
        assert_eq!(denied.code, "multimod.job.internal_labs_required");

        assert!(
            !validate_multimod_runtime_surface_access_using_v1(
                MULTIMOD_INTERNAL_LABS_SURFACE_V1,
                true,
                || -> Result<bool, String> { panic!("Labs access must not consult authority") },
            )
            .unwrap()
        );
        let unsupported =
            validate_multimod_runtime_surface_access_using_v1("standard", true, || Ok(true))
                .unwrap_err();
        assert_eq!(unsupported.code, "multimod.job.surface_invalid");
    }

    #[test]
    fn staged_recipe_provenance_persists_the_validated_request_surface() {
        let mut metadata = std::collections::BTreeMap::new();
        persist_multimod_execution_surface_v1(&mut metadata, MULTIMOD_STANDARD_SURFACE_V1.into());
        assert_eq!(
            metadata.get("execution_surface").map(String::as_str),
            Some(MULTIMOD_STANDARD_SURFACE_V1)
        );
    }
}
