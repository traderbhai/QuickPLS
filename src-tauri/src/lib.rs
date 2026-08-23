mod canonical_result_export_publication_v2;
mod capability_registry_command;
mod general_sem_registry_access_v1;
#[cfg(test)]
mod pls_algorithm_current_product_qualification;
mod pls_model_comparison_jobs;
mod project_archive_v6_general_sem_bootstrap;
mod project_archive_v6_general_sem_preflight;
mod project_archive_v6_general_sem_revision;
mod project_archive_v6_model_mutation;
mod project_archive_v6_native_adoption;
mod project_archive_v6_new_general_sem;
mod project_archive_v6_read;
mod project_archive_v6_save_copy;
mod project_schema6_result_append;
mod project_schema6_result_read;
mod project_upgrade_assistant;
mod recipe_v4_canonical_result;
mod recipe_v4_cbsem_canonical_result;
mod recipe_v4_cbsem_execution;
mod recipe_v4_general_sem_canonical_result;
#[allow(dead_code)]
mod recipe_v4_general_sem_cbsem_canonical_result;
mod recipe_v4_general_sem_cbsem_jobs;
mod recipe_v4_general_sem_pls_jobs;
mod recipe_v4_jobs;
mod sample_projects;
mod sem_model_v4_scientific_digest;
mod standard_sem_model_v4_authority;
#[cfg(test)]
mod wave1_diagram_cbsem_roundtrip;

use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use canonical_result_export_publication_v2::publish_canonical_result_export_v2;
use capability_registry_command::capability_registry_v2;
use chrono::{SecondsFormat, Utc};
use pls_model_comparison_jobs::{
    DesktopPlsModelComparisonJobsV1, cancel_internal_labs_pls_model_comparison_job,
    dismiss_internal_labs_pls_model_comparison_job, internal_labs_pls_model_comparison_job_result,
    internal_labs_pls_model_comparison_job_status, start_internal_labs_pls_model_comparison_job,
};
use project_archive_v6_general_sem_bootstrap::{
    DesktopGeneralSemFreshDraftAuthorityV1, GeneralSemNewProjectModeV1,
    authorize_general_sem_revision_draft_v1, bootstrap_internal_general_sem_project_archive_v6,
    invalidate_general_sem_fresh_draft_authority_v1,
};
use project_archive_v6_general_sem_preflight::preflight_internal_general_sem_estimators_v1;
use project_archive_v6_general_sem_revision::{
    revise_internal_general_sem_execution_authority_v1,
    revise_internal_general_sem_execution_authority_v2,
};
use project_archive_v6_model_mutation::mutate_internal_project_archive_v6_model;
use project_archive_v6_native_adoption::{
    DesktopSchema6NativeAdoptionAuthorityV1,
    adopt_internal_project_archive_v6_native_revision_source_v1,
    clear_internal_project_archive_v6_native_revision_source_v1,
};
use project_archive_v6_new_general_sem::create_internal_general_sem_project_archive_v6;
use project_archive_v6_read::{
    inspect_internal_project_archive_v6_zip, read_internal_project_archive_v6_dataset_rows,
};
use project_archive_v6_save_copy::save_internal_project_archive_v6_copy;
use project_schema6_result_append::append_internal_project_schema6_canonical_result_v2;
use project_schema6_result_read::read_internal_project_schema6_canonical_results_v2;
use project_upgrade_assistant::{
    DesktopProjectUpgradePlans, cancel_internal_project_upgrade_v6,
    execute_internal_project_upgrade_v6, inspect_internal_project_upgrade_v6,
    plan_internal_project_upgrade_v6,
};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeV4,
    AnalysisResult, AnalysisSettings, ApplyDatasetTransformationOptionsV2,
    CapabilityCellReferenceV2, Construct, DatasetTransformationErrorV2,
    DatasetTransformationIssueV2, DatasetTransformationPreviewV2, DatasetTransformationSpecV2,
    JobSnapshot, JobState, METHOD_CAPABILITIES, MeasurementMode, MethodCapability, MethodConfig,
    ModelSpec, PlsPosthocTechnicalMinimumSampleSizeConfigV2, RecipeV4CompilationError,
    RecipeV4CompilerTarget, RunStatus, SemModelV4, Severity, StructuralPath, ValidationIssue,
    apply_dataset_transformation_v2, compile_analysis_recipe_v4,
    preview_dataset_transformation_v2 as preview_dataset_transformation_kernel_v2, sha256_hex,
    validate_recipe,
};
use qpls_data::{
    ColumnMetadata, DataKind, Dataset, ImportOptions, RecodeColumnSpec, import_delimited_bytes,
    import_path, preview, preview_page, recode_column, update_column_metadata,
};
use qpls_project::{
    Project, ProjectDataLineageV1 as DatasetLineageLayout,
    ProjectDatasetVersionOperationV1 as DatasetVersionOperation,
    ProjectDatasetVersionRecordV1 as DatasetVersionRecord, RecoverySource, discard_autosave,
    load_project_with_autosave, save_autosave, save_project, validate_data_lineage_resident_v1,
    validate_project_data_lineage_resident_v1, write_project_data_lineage_v1,
};
use qpls_runner::{
    RecipeV4PlsExecutionError, RecipeV4PlsExecutionResultV1, RunnerError,
    run_compiled_pls_recipe_v4, run_pls_analysis,
};
use recipe_v4_cbsem_execution::run_internal_labs_recipe_v4_cbsem_execution;
use recipe_v4_general_sem_cbsem_jobs::{
    DesktopCbsemGeneralSemJobsV1, cancel_internal_labs_general_sem_cbsem_job_v1,
    dismiss_internal_labs_general_sem_cbsem_job_v1, result_internal_labs_general_sem_cbsem_job_v1,
    start_internal_labs_general_sem_cbsem_job_v1, status_internal_labs_general_sem_cbsem_job_v1,
};
use recipe_v4_general_sem_pls_jobs::{
    DesktopGeneralSemPlsJobsV1, cancel_internal_labs_general_sem_pls_job_v1,
    dismiss_internal_labs_general_sem_pls_job_v1, result_internal_labs_general_sem_pls_job_v1,
    start_internal_labs_general_sem_pls_job_v1, status_internal_labs_general_sem_pls_job_v1,
};
use recipe_v4_jobs::{
    DesktopRecipeV4Jobs, cancel_internal_labs_recipe_v4_cbsem_job,
    cancel_internal_labs_recipe_v4_pls_job, dismiss_internal_labs_recipe_v4_cbsem_job,
    dismiss_internal_labs_recipe_v4_pls_job, internal_labs_recipe_v4_cbsem_job_result,
    internal_labs_recipe_v4_cbsem_job_status, internal_labs_recipe_v4_pls_job_result,
    internal_labs_recipe_v4_pls_job_status, start_internal_labs_recipe_v4_cbsem_job,
    start_internal_labs_recipe_v4_pls_job,
};
use regex::{Captures, Regex};
use sample_projects::{BundledSampleProject, build_bundled_sample_project};
use sem_model_v4_scientific_digest::internal_sem_model_v4_scientific_sha256;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use standard_sem_model_v4_authority::{
    compare_and_swap_standard_sem_model_v4_authority, resolve_standard_sem_model_v4_authority,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    fs,
    io::{self, Cursor, Read, Write},
    net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::State;
use uuid::Uuid;
use zip::{ZipWriter, write::SimpleFileOptions};

struct DesktopProject(Arc<Mutex<Project>>);

struct DesktopJob {
    snapshot: JobSnapshot,
    cancellation: Arc<AtomicBool>,
    result: Option<AnalysisResult>,
    worker_demand: usize,
}

struct DesktopJobs(Arc<Mutex<HashMap<Uuid, DesktopJob>>>);

#[derive(Clone)]
struct DesktopDiagnostics(Arc<Mutex<DiagnosticRuntimeState>>);

#[derive(Debug)]
struct DiagnosticRuntimeState {
    next_sequence: u64,
    events: VecDeque<DiagnosticEvent>,
    pending_previews: HashMap<String, PendingDiagnosticPreview>,
}

#[derive(Debug, Clone)]
struct PendingDiagnosticPreview {
    staging: DiagnosticStaging,
    order: u64,
    expires_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticEvent {
    timestamp: String,
    sequence: u64,
    severity: String,
    code: String,
}

#[derive(Debug, Clone)]
struct DiagnosticStaging {
    created_at: String,
    event_count: usize,
    redaction_counts: DiagnosticRedactionCounts,
    entries: Vec<(&'static str, Vec<u8>)>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticRedactionCounts {
    windows_paths: usize,
    email_addresses: usize,
    url_queries_or_fragments: usize,
    bearer_tokens: usize,
}

impl DiagnosticRedactionCounts {
    fn total(self) -> usize {
        self.windows_paths
            + self.email_addresses
            + self.url_queries_or_fragments
            + self.bearer_tokens
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticBundlePreview {
    preview_id: String,
    created_at: String,
    included_categories: Vec<&'static str>,
    excluded_categories: Vec<&'static str>,
    redaction_counts: DiagnosticRedactionCounts,
    entry_count: usize,
    event_count: usize,
    estimated_uncompressed_bytes: usize,
    local_only: bool,
    network_activity: &'static str,
    staged_contents: DiagnosticStagedContents,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticBundleSaveResult {
    bytes: u64,
    archive_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticSystemMetadata {
    schema_version: u32,
    quickpls_version: String,
    release_channel: String,
    source_revision: String,
    os_family: String,
    architecture: String,
    desktop_runtime: String,
    locale: String,
    webview2_version: String,
    user_data_included: bool,
    network_accessed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticEntryDescriptor {
    name: String,
    sha256: String,
    bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticArchiveLimits {
    maximum_entries: usize,
    maximum_entry_bytes: usize,
    maximum_uncompressed_bytes: usize,
    maximum_archive_bytes: usize,
    compression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticManifestContents {
    schema_version: u32,
    policy_version: String,
    created_at: String,
    quickpls_version: String,
    entries: Vec<DiagnosticEntryDescriptor>,
    redaction_counts: DiagnosticRedactionCounts,
    redaction_total: usize,
    archive_limits: DiagnosticArchiveLimits,
    local_only: bool,
    network_accessed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiagnosticStagedContents {
    system: DiagnosticSystemMetadata,
    events: Vec<DiagnosticEvent>,
    manifest: DiagnosticManifestContents,
}

const MAX_DATASET_ROW_PAGE_SIZE: usize = 500;
const MAX_GROUP_PROFILE_VALUES: usize = 1_000;
const MAX_TEXT_EXPORT_BYTES: usize = 128 * 1024 * 1024;
const DIAGNOSTIC_POLICY_VERSION: &str = "quickpls-diagnostics-v1";
const MAX_DIAGNOSTIC_EVENTS: usize = 128;
const MAX_PENDING_DIAGNOSTIC_PREVIEWS: usize = 4;
const DIAGNOSTIC_PREVIEW_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_DIAGNOSTIC_ARCHIVE_ENTRIES: usize = 3;
const MAX_DIAGNOSTIC_ENTRY_BYTES: usize = 256 * 1024;
const MAX_DIAGNOSTIC_UNCOMPRESSED_BYTES: usize = 512 * 1024;
const MAX_DIAGNOSTIC_ARCHIVE_BYTES: usize = 520 * 1024;
const DIAGNOSTIC_SYSTEM_ENTRY: &str = "metadata/system.json";
const DIAGNOSTIC_EVENTS_ENTRY: &str = "logs/events.jsonl";
const DIAGNOSTIC_MANIFEST_ENTRY: &str = "manifest.json";
const WORKSPACE_EXPLORER_LAYOUT_KEY: &str = "workspace_explorer";
const WORKSPACE_EXPLORER_SCHEMA_VERSION: u32 = 1;
const SAMPLE_RUN_DISPLAY_NAME: &str = "PLS-SEM Bootstrapping run";
const SAMPLE_RUN_METHOD_NAME: &str = "PLS-SEM Bootstrapping";
const WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS: &str = "127.0.0.1:17846";
const WEBVIEW2_OFFLINE_PROXY_RESPONSE: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\nX-QuickPLS-Network-Policy: offline\r\n\r\n";
const MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES: usize = 8 * 1024;

fn bind_webview2_offline_rejection_proxy(bind_address: SocketAddr) -> io::Result<TcpListener> {
    if bind_address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the WebView2 offline proxy must bind only to 127.0.0.1",
        ));
    }
    TcpListener::bind(bind_address)
}

fn reject_webview2_proxy_stream(mut stream: TcpStream) -> io::Result<()> {
    let write_result = (|| {
        stream.set_read_timeout(Some(Duration::from_millis(250)))?;
        let mut request_prefix = [0_u8; MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES];
        let mut received = 0;
        while received < request_prefix.len() {
            match stream.read(&mut request_prefix[received..]) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    received += bytes_read;
                    if request_prefix[..received]
                        .windows(4)
                        .any(|window| window == b"\r\n\r\n")
                    {
                        break;
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream.write_all(WEBVIEW2_OFFLINE_PROXY_RESPONSE)?;
        stream.flush()
    })();
    let _ = stream.shutdown(Shutdown::Both);
    write_result
}

fn serve_webview2_offline_rejection_proxy(listener: TcpListener) -> ! {
    loop {
        match listener.accept() {
            Ok((stream, _peer_address)) => {
                let _ = reject_webview2_proxy_stream(stream);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn start_webview2_offline_rejection_proxy() -> io::Result<std::thread::JoinHandle<()>> {
    let bind_address = WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS
        .parse::<SocketAddr>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let listener = bind_webview2_offline_rejection_proxy(bind_address)?;
    std::thread::Builder::new()
        .name("quickpls-webview2-offline-proxy".to_string())
        .spawn(move || serve_webview2_offline_rejection_proxy(listener))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetSnapshot {
    id: String,
    name: String,
    columns: Vec<String>,
    rows: Vec<std::collections::BTreeMap<String, Option<String>>>,
    row_count: usize,
    missing: usize,
    missing_by_column: std::collections::BTreeMap<String, usize>,
    fingerprint: String,
    kind: DataKind,
    sample_size: Option<usize>,
    column_metadata: Vec<ColumnMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetRowsPage {
    dataset_id: String,
    offset: usize,
    limit: usize,
    row_count: usize,
    rows: Vec<std::collections::BTreeMap<String, Option<String>>>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DatasetGroupProfileValue {
    value: String,
    label: Option<String>,
    observations: usize,
    complete_cases: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DatasetGroupProfile {
    dataset_id: String,
    column_name: String,
    row_count: usize,
    missing_count: usize,
    unsupported_count: usize,
    truncated: bool,
    groups: Vec<DatasetGroupProfileValue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetVersionMutation {
    dataset: DatasetSnapshot,
    version: DatasetVersionRecord,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSnapshot {
    project_id: Uuid,
    name: String,
    path: Option<String>,
    read_only: bool,
    source_archive_version: u32,
    migration_pending: bool,
    compatibility_notices: Vec<ProjectCompatibilityNoticeSnapshot>,
    future_unsupported: ProjectFutureUnsupportedSnapshot,
    save_warning: Option<String>,
    recovered: bool,
    recovery_source: Option<String>,
    datasets: Vec<DatasetSnapshot>,
    dataset_versions: Vec<DatasetVersionRecord>,
    workspace: Option<Value>,
    models: Vec<ModelSpec>,
    recipes: Vec<AnalysisRecipe>,
    results: Vec<AnalysisResult>,
    active_model_id: Option<String>,
    model_presentations: BTreeMap<String, Value>,
    saved_reports: Vec<SavedReportNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFutureUnsupportedSnapshot {
    models: usize,
    recipes: usize,
    results: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectCompatibilityNoticeSnapshot {
    result_id: Uuid,
    code: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SavedReportNode {
    result_id: Uuid,
    name: String,
    saved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct WorkspaceExplorerLayout {
    schema_version: u32,
    #[serde(default)]
    model_presentations: BTreeMap<String, Value>,
    #[serde(default)]
    saved_reports: Vec<SavedReportNode>,
}

impl Default for WorkspaceExplorerLayout {
    fn default() -> Self {
        Self {
            schema_version: WORKSPACE_EXPLORER_SCHEMA_VERSION,
            model_presentations: BTreeMap::new(),
            saved_reports: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum ProjectExplorerMutation {
    CreateModel { name: String },
    ActivateModel { model_id: Uuid },
    RenameModel { model_id: Uuid, name: String },
    DeleteModel { model_id: Uuid },
    SaveReport { result_id: Uuid, name: String },
    RenameReport { result_id: Uuid, name: String },
    RemoveReport { result_id: Uuid },
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ProjectExplorerMutationRequest {
    mutation: ProjectExplorerMutation,
    current_model: Option<ModelSpec>,
    current_presentation: Option<Value>,
    path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChecksumVerification {
    checksum_file: Option<String>,
    checked: usize,
    verified: usize,
    failures: Vec<String>,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportTable {
    title: String,
    status: String,
    warning: Option<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl DesktopDiagnostics {
    fn new() -> Self {
        let mut events = VecDeque::new();
        events.push_back(DiagnosticEvent {
            timestamp: diagnostic_timestamp(),
            sequence: 1,
            severity: "info".to_string(),
            code: "desktop.session.started".to_string(),
        });
        Self(Arc::new(Mutex::new(DiagnosticRuntimeState {
            next_sequence: 2,
            events,
            pending_previews: HashMap::new(),
        })))
    }

    fn record_event(&self, code: &'static str) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_STATE_UNAVAILABLE",
                "Diagnostic state is unavailable.",
            )
        })?;
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);
        state.events.push_back(DiagnosticEvent {
            timestamp: diagnostic_timestamp(),
            sequence,
            severity: "info".to_string(),
            code: code.to_string(),
        });
        while state.events.len() > MAX_DIAGNOSTIC_EVENTS {
            state.events.pop_front();
        }
        Ok(())
    }

    fn create_preview(
        &self,
        replaces_preview_id: Option<&str>,
    ) -> Result<DiagnosticBundlePreview, String> {
        let mut state = self.0.lock().map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_STATE_UNAVAILABLE",
                "Diagnostic state is unavailable.",
            )
        })?;
        let now = Instant::now();
        prune_expired_diagnostic_previews(&mut state, now);
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);
        state.events.push_back(DiagnosticEvent {
            timestamp: diagnostic_timestamp(),
            sequence,
            severity: "info".to_string(),
            code: "diagnostic.preview.requested".to_string(),
        });
        while state.events.len() > MAX_DIAGNOSTIC_EVENTS {
            state.events.pop_front();
        }
        let staging = build_diagnostic_staging(state.events.make_contiguous())?;
        let staged_contents = inspect_diagnostic_staging(&staging.entries)?;
        let preview_id = Uuid::new_v4().to_string();
        let preview = DiagnosticBundlePreview {
            preview_id: preview_id.clone(),
            created_at: staging.created_at.clone(),
            included_categories: vec![
                "QuickPLS build and release identity",
                "Operating-system family and architecture",
                "Bounded session diagnostic event codes",
                "Manifest hashes, sizes, limits, and redaction counts",
            ],
            excluded_categories: vec![
                "Dataset rows, values, and variable names",
                "Project contents, model labels, and project titles",
                "Results, reports, and exports",
                "Credentials, environment values, and command lines",
                "Arbitrary files, registry data, and memory dumps",
            ],
            redaction_counts: staging.redaction_counts,
            entry_count: staging.entries.len(),
            event_count: staging.event_count,
            estimated_uncompressed_bytes: staging
                .entries
                .iter()
                .map(|(_, bytes)| bytes.len())
                .sum(),
            local_only: true,
            network_activity: "none",
            staged_contents,
        };
        if let Some(replaced) = replaces_preview_id {
            state.pending_previews.remove(replaced);
        }
        if state.pending_previews.len() >= MAX_PENDING_DIAGNOSTIC_PREVIEWS {
            let oldest_preview_id = state
                .pending_previews
                .iter()
                .min_by(|(left_id, left), (right_id, right)| {
                    left.order
                        .cmp(&right.order)
                        .then_with(|| left_id.cmp(right_id))
                })
                .map(|(id, _)| id.clone());
            if let Some(oldest_preview_id) = oldest_preview_id {
                state.pending_previews.remove(&oldest_preview_id);
            }
        }
        match state.pending_previews.entry(preview_id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(PendingDiagnosticPreview {
                    staging,
                    order: sequence,
                    expires_at: now + DIAGNOSTIC_PREVIEW_TTL,
                });
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(diagnostic_error(
                    "DIAGNOSTIC_PREVIEW_ID_COLLISION",
                    "QuickPLS could not allocate a unique diagnostic preview ID.",
                ));
            }
        }
        Ok(preview)
    }

    fn consume_preview(&self, id: &str) -> Result<DiagnosticStaging, String> {
        let mut state = self.0.lock().map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_STATE_UNAVAILABLE",
                "Diagnostic state is unavailable.",
            )
        })?;
        prune_expired_diagnostic_previews(&mut state, Instant::now());
        state
            .pending_previews
            .remove(id)
            .map(|pending| pending.staging)
            .ok_or_else(|| {
                diagnostic_error(
                    "DIAGNOSTIC_PREVIEW_REQUIRED",
                    "This diagnostic preview is absent, expired, or already consumed. Create a new preview.",
                )
            })
    }

    fn cancel_preview(&self, id: &str) -> Result<(), String> {
        self.consume_preview(id).map(|_| ())
    }
}

fn prune_expired_diagnostic_previews(state: &mut DiagnosticRuntimeState, now: Instant) {
    state
        .pending_previews
        .retain(|_, preview| preview.expires_at > now);
}

fn diagnostic_error(code: &str, message: &str) -> String {
    format!("{code}: {message}")
}

fn diagnostic_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn diagnostic_release_channel() -> &'static str {
    match option_env!("QUICKPLS_RELEASE_CHANNEL") {
        Some(channel) if matches!(channel, "internal" | "unsigned-preview" | "beta" | "stable") => {
            channel
        }
        _ => "internal",
    }
}

fn diagnostic_source_revision() -> &'static str {
    option_env!("QUICKPLS_SOURCE_REVISION")
        .filter(|value| {
            (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .unwrap_or("not_provided")
}

static BEARER_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer[ \t]+[A-Za-z0-9._~+/=-]+").expect("valid bearer-token regex")
});
static URL_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(https?://[^\s\"'<>?#]+)[?#][^\s\"'<>]*"#).expect("valid URL-suffix regex")
});
static EMAIL_ADDRESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").expect("valid email regex")
});
static WINDOWS_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(^|[^A-Z0-9])([A-Z]:[\\/][^\"'\r\n,;<>|?*]+)"#)
        .expect("valid Windows-path regex")
});

fn sanitize_diagnostic_basename(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .take(64)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.trim_matches(['.', '_', '-']).is_empty() {
        sanitized = "file".to_string();
    }
    sanitized
}

fn redact_diagnostic_text(input: &str, counts: &mut DiagnosticRedactionCounts) -> String {
    let redacted = BEARER_TOKEN
        .replace_all(input, |_: &Captures<'_>| {
            counts.bearer_tokens += 1;
            "Bearer <redacted-token>"
        })
        .into_owned();
    let redacted = URL_SUFFIX
        .replace_all(&redacted, |captures: &Captures<'_>| {
            counts.url_queries_or_fragments += 1;
            captures[1].to_string()
        })
        .into_owned();
    let redacted = EMAIL_ADDRESS
        .replace_all(&redacted, |_: &Captures<'_>| {
            counts.email_addresses += 1;
            "<redacted-email>"
        })
        .into_owned();
    WINDOWS_PATH
        .replace_all(&redacted, |captures: &Captures<'_>| {
            counts.windows_paths += 1;
            let path = captures[2].trim_end_matches(|character: char| {
                character.is_whitespace() || matches!(character, '.' | ')' | ']' | '}')
            });
            let basename = path.rsplit(['\\', '/']).next().unwrap_or("file");
            format!(
                "{}<redacted-path>/{}",
                &captures[1],
                sanitize_diagnostic_basename(basename)
            )
        })
        .into_owned()
}

fn checked_diagnostic_entry(
    name: &'static str,
    bytes: Vec<u8>,
) -> Result<(&'static str, Vec<u8>), String> {
    if !matches!(
        name,
        DIAGNOSTIC_SYSTEM_ENTRY | DIAGNOSTIC_EVENTS_ENTRY | DIAGNOSTIC_MANIFEST_ENTRY
    ) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ENTRY_NOT_ALLOWED",
            "A diagnostic archive entry is not on the fixed allowlist.",
        ));
    }
    if bytes.len() > MAX_DIAGNOSTIC_ENTRY_BYTES {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ENTRY_TOO_LARGE",
            "A diagnostic archive entry exceeds the size limit.",
        ));
    }
    Ok((name, bytes))
}

fn build_diagnostic_staging(events: &[DiagnosticEvent]) -> Result<DiagnosticStaging, String> {
    let created_at = diagnostic_timestamp();
    let metadata = DiagnosticSystemMetadata {
        schema_version: 1,
        quickpls_version: env!("CARGO_PKG_VERSION").to_string(),
        release_channel: diagnostic_release_channel().to_string(),
        source_revision: diagnostic_source_revision().to_string(),
        os_family: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        desktop_runtime: "Tauri 2".to_string(),
        locale: "not_collected".to_string(),
        webview2_version: "not_collected".to_string(),
        user_data_included: false,
        network_accessed: false,
    };
    let mut redaction_counts = DiagnosticRedactionCounts::default();
    let metadata_json = serde_json::to_string_pretty(&metadata).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_SERIALIZATION_FAILED",
            "Diagnostic metadata could not be serialized.",
        )
    })?;
    let metadata_bytes = redact_diagnostic_text(&metadata_json, &mut redaction_counts).into_bytes();

    let mut event_lines = String::new();
    for event in events.iter().take(MAX_DIAGNOSTIC_EVENTS) {
        let line = serde_json::to_string(event).map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_SERIALIZATION_FAILED",
                "Diagnostic events could not be serialized.",
            )
        })?;
        event_lines.push_str(&redact_diagnostic_text(&line, &mut redaction_counts));
        event_lines.push('\n');
    }

    let mut entries = vec![
        checked_diagnostic_entry(DIAGNOSTIC_SYSTEM_ENTRY, metadata_bytes)?,
        checked_diagnostic_entry(DIAGNOSTIC_EVENTS_ENTRY, event_lines.into_bytes())?,
    ];
    let descriptors = entries
        .iter()
        .map(|(name, bytes)| DiagnosticEntryDescriptor {
            name: (*name).to_string(),
            sha256: sha256_hex(bytes),
            bytes: bytes.len(),
        })
        .collect::<Vec<_>>();
    let manifest = DiagnosticManifestContents {
        schema_version: 1,
        policy_version: DIAGNOSTIC_POLICY_VERSION.to_string(),
        created_at: created_at.clone(),
        quickpls_version: env!("CARGO_PKG_VERSION").to_string(),
        entries: descriptors,
        redaction_counts,
        redaction_total: redaction_counts.total(),
        archive_limits: DiagnosticArchiveLimits {
            maximum_entries: MAX_DIAGNOSTIC_ARCHIVE_ENTRIES,
            maximum_entry_bytes: MAX_DIAGNOSTIC_ENTRY_BYTES,
            maximum_uncompressed_bytes: MAX_DIAGNOSTIC_UNCOMPRESSED_BYTES,
            maximum_archive_bytes: MAX_DIAGNOSTIC_ARCHIVE_BYTES,
            compression: "stored".to_string(),
        },
        local_only: true,
        network_accessed: false,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_SERIALIZATION_FAILED",
            "The diagnostic manifest could not be serialized.",
        )
    })?;
    entries.push(checked_diagnostic_entry(
        DIAGNOSTIC_MANIFEST_ENTRY,
        manifest_bytes,
    )?);
    validate_diagnostic_staging(&entries)?;
    Ok(DiagnosticStaging {
        created_at,
        event_count: events.len().min(MAX_DIAGNOSTIC_EVENTS),
        redaction_counts,
        entries,
    })
}

fn inspect_diagnostic_staging(
    entries: &[(&'static str, Vec<u8>)],
) -> Result<DiagnosticStagedContents, String> {
    validate_diagnostic_staging(entries)?;
    let bytes = |name| {
        entries
            .iter()
            .find_map(|(entry_name, bytes)| (*entry_name == name).then_some(bytes.as_slice()))
            .ok_or_else(|| {
                diagnostic_error(
                    "DIAGNOSTIC_ENTRY_SET_INVALID",
                    "The diagnostic preview entry set is not valid.",
                )
            })
    };
    let system = serde_json::from_slice(bytes(DIAGNOSTIC_SYSTEM_ENTRY)?).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_PREVIEW_INVALID",
            "Redacted system metadata could not be inspected.",
        )
    })?;
    let events = std::str::from_utf8(bytes(DIAGNOSTIC_EVENTS_ENTRY)?)
        .map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_PREVIEW_INVALID",
                "Redacted diagnostic events could not be inspected.",
            )
        })?
        .lines()
        .map(|line| {
            serde_json::from_str(line).map_err(|_| {
                diagnostic_error(
                    "DIAGNOSTIC_PREVIEW_INVALID",
                    "A redacted diagnostic event could not be inspected.",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = serde_json::from_slice(bytes(DIAGNOSTIC_MANIFEST_ENTRY)?).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_PREVIEW_INVALID",
            "The diagnostic manifest could not be inspected.",
        )
    })?;
    Ok(DiagnosticStagedContents {
        system,
        events,
        manifest,
    })
}

fn validate_diagnostic_staging(entries: &[(&'static str, Vec<u8>)]) -> Result<(), String> {
    if entries.len() != MAX_DIAGNOSTIC_ARCHIVE_ENTRIES {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ENTRY_COUNT_INVALID",
            "The diagnostic archive must contain exactly three allowlisted entries.",
        ));
    }
    let expected = [
        DIAGNOSTIC_SYSTEM_ENTRY,
        DIAGNOSTIC_EVENTS_ENTRY,
        DIAGNOSTIC_MANIFEST_ENTRY,
    ];
    let mut names = BTreeSet::new();
    let mut total = 0usize;
    for ((name, bytes), expected_name) in entries.iter().zip(expected) {
        if *name != expected_name || !names.insert(*name) {
            return Err(diagnostic_error(
                "DIAGNOSTIC_ENTRY_SET_INVALID",
                "The diagnostic archive entry set is not valid.",
            ));
        }
        if bytes.len() > MAX_DIAGNOSTIC_ENTRY_BYTES {
            return Err(diagnostic_error(
                "DIAGNOSTIC_ENTRY_TOO_LARGE",
                "A diagnostic archive entry exceeds the size limit.",
            ));
        }
        total = total.checked_add(bytes.len()).ok_or_else(|| {
            diagnostic_error(
                "DIAGNOSTIC_ARCHIVE_TOO_LARGE",
                "The diagnostic archive exceeds the total size limit.",
            )
        })?;
    }
    if total > MAX_DIAGNOSTIC_UNCOMPRESSED_BYTES {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ARCHIVE_TOO_LARGE",
            "The diagnostic archive exceeds the total size limit.",
        ));
    }
    Ok(())
}

fn validate_new_diagnostic_path(path: &Path) -> Result<(), String> {
    let raw = path.to_str().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_PATH_ENCODING_BLOCKED",
            "The diagnostic destination must be a valid Unicode Windows path.",
        )
    })?;
    if raw.starts_with(r"\\")
        || raw.starts_with(r"//")
        || raw.starts_with(r"\\?")
        || raw.starts_with(r"\\.")
        || raw.starts_with(r"\??\")
    {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED",
            "UNC, verbatim, and device namespace destinations are not supported.",
        ));
    }
    validate_diagnostic_path_root(path, raw)?;
    if diagnostic_path_has_alternate_stream(path, raw) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PATH_ADS_BLOCKED",
            "Diagnostic destination components cannot contain a colon or alternate data stream.",
        ));
    }
    for component in path.components() {
        match component {
            std::path::Component::CurDir | std::path::Component::ParentDir => {
                return Err(diagnostic_error(
                    "DIAGNOSTIC_PATH_COMPONENT_BLOCKED",
                    "Diagnostic destinations cannot contain current-directory or parent-directory components.",
                ));
            }
            std::path::Component::Normal(value) => {
                let value = value.to_str().ok_or_else(|| {
                    diagnostic_error(
                        "DIAGNOSTIC_PATH_ENCODING_BLOCKED",
                        "Diagnostic destination components must use valid Unicode.",
                    )
                })?;
                if value.ends_with(' ') || value.ends_with('.') {
                    return Err(diagnostic_error(
                        "DIAGNOSTIC_PATH_COMPONENT_BLOCKED",
                        "Diagnostic destination components cannot end in a space or period.",
                    ));
                }
                if diagnostic_component_is_reserved(value) {
                    return Err(diagnostic_error(
                        "DIAGNOSTIC_DEVICE_NAME_BLOCKED",
                        "Reserved Windows device names are not valid diagnostic destinations.",
                    ));
                }
            }
            _ => {}
        }
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if extension.as_deref() != Some("zip") {
        return Err(diagnostic_error(
            "DIAGNOSTIC_EXTENSION_INVALID",
            "Diagnostic bundles must use the .zip extension.",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_PARENT_INVALID",
            "The selected destination has no parent directory.",
        )
    })?;
    if !parent.is_dir() {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PARENT_UNAVAILABLE",
            "The selected destination directory does not exist.",
        ));
    }
    for ancestor in parent
        .ancestors()
        .filter(|value| !value.as_os_str().is_empty())
    {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_PARENT_UNAVAILABLE",
                "The selected destination directory is unavailable.",
            )
        })?;
        if diagnostic_metadata_is_reparse(&metadata) {
            return Err(diagnostic_error(
                "DIAGNOSTIC_REPARSE_POINT_BLOCKED",
                "QuickPLS will not write a diagnostic bundle through a symbolic link, junction, or other reparse point.",
            ));
        }
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_EXISTS",
            "Choose a new ZIP filename; diagnostic bundles never overwrite files.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_UNAVAILABLE",
            "The selected destination could not be inspected safely.",
        )),
    }
}

#[cfg(windows)]
fn validate_diagnostic_path_root(path: &Path, raw: &str) -> Result<(), String> {
    let bytes = raw.as_bytes();
    let drive_rooted = bytes.len() >= 4
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    if !path.is_absolute() || !drive_rooted {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE",
            "Choose a drive-rooted local Windows destination such as C:\\Support\\bundle.zip.",
        ));
    }
    if !diagnostic_drive_is_local(bytes[0]) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_DRIVE_NOT_LOCAL",
            "Choose a local fixed drive; removable, RAM, and mapped network drives are not supported.",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn validate_diagnostic_path_root(path: &Path, _raw: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE",
            "Choose an absolute local destination for the diagnostic ZIP.",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn diagnostic_path_has_alternate_stream(_path: &Path, raw: &str) -> bool {
    raw.as_bytes()
        .get(2..)
        .is_some_and(|suffix| suffix.contains(&b':'))
}

#[cfg(not(windows))]
fn diagnostic_path_has_alternate_stream(path: &Path, _raw: &str) -> bool {
    path.components().any(|component| {
        matches!(component, std::path::Component::Normal(value) if value.to_string_lossy().contains(':'))
    })
}

fn diagnostic_component_is_reserved(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2"
                        | "3"
                        | "4"
                        | "5"
                        | "6"
                        | "7"
                        | "8"
                        | "9"
                        | "\u{00B9}"
                        | "\u{00B2}"
                        | "\u{00B3}"
                )
            })
}

fn diagnostic_drive_type_is_local(drive_type: u32) -> bool {
    drive_type == 3
}

#[cfg(windows)]
fn diagnostic_drive_is_local(letter: u8) -> bool {
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    let root = [
        letter.to_ascii_uppercase() as u16,
        b':' as u16,
        b'\\' as u16,
        0,
    ];
    diagnostic_drive_type_is_local(unsafe { GetDriveTypeW(root.as_ptr()) })
}

#[cfg(not(windows))]
fn diagnostic_drive_is_local(_letter: u8) -> bool {
    false
}

#[cfg(windows)]
fn diagnostic_metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn diagnostic_metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

struct OpenDiagnosticDestination {
    file: fs::File,
    #[cfg(windows)]
    _parent_guard: fs::File,
}

fn expected_diagnostic_destination(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_PARENT_INVALID",
            "The selected destination has no parent directory.",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_DESTINATION_UNAVAILABLE",
            "The selected diagnostic filename is unavailable.",
        )
    })?;
    fs::canonicalize(parent)
        .map(|canonical_parent| canonical_parent.join(file_name))
        .map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_PARENT_UNAVAILABLE",
                "The selected destination directory could not be bound safely.",
            )
        })
}

fn verify_open_diagnostic_file(file: &fs::File, expected: &Path) -> Result<(), String> {
    let metadata = file.metadata().map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_DESTINATION_VERIFY_FAILED",
            "The opened diagnostic destination could not be inspected safely.",
        )
    })?;
    if !metadata.file_type().is_file() || diagnostic_metadata_is_reparse(&metadata) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_TYPE_BLOCKED",
            "The opened diagnostic destination must be a regular non-reparse file.",
        ));
    }
    verify_open_diagnostic_file_path(file, expected)
}

#[cfg(windows)]
fn diagnostic_path_from_handle(file: &fs::File) -> Result<PathBuf, String> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW, VOLUME_NAME_DOS,
    };

    let mut buffer = vec![0_u16; 512];
    loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        };
        if length == 0 {
            return Err(diagnostic_error(
                "DIAGNOSTIC_FINAL_PATH_UNAVAILABLE",
                "Windows could not resolve the opened diagnostic destination.",
            ));
        }
        if (length as usize) < buffer.len() {
            buffer.truncate(length as usize);
            return Ok(PathBuf::from(OsString::from_wide(&buffer)));
        }
        let required = (length as usize).checked_add(1).ok_or_else(|| {
            diagnostic_error(
                "DIAGNOSTIC_FINAL_PATH_UNAVAILABLE",
                "The opened diagnostic destination path is too long to inspect safely.",
            )
        })?;
        if required > 32_768 {
            return Err(diagnostic_error(
                "DIAGNOSTIC_FINAL_PATH_UNAVAILABLE",
                "The opened diagnostic destination path exceeds the supported limit.",
            ));
        }
        buffer.resize(required, 0);
    }
}

#[cfg(windows)]
fn normalized_windows_diagnostic_path(path: &Path) -> Result<String, String> {
    let raw = path.to_str().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_PATH_ENCODING_BLOCKED",
            "The opened diagnostic destination must have a valid Unicode path.",
        )
    })?;
    let dos_path = raw.strip_prefix(r"\\?\").unwrap_or(raw);
    if dos_path.starts_with("UNC\\") || dos_path.starts_with(r"\\") {
        return Err(diagnostic_error(
            "DIAGNOSTIC_FINAL_PATH_NOT_LOCAL",
            "The opened diagnostic destination resolved outside a local drive.",
        ));
    }
    let bytes = dos_path.as_bytes();
    let drive_rooted = bytes.len() >= 4
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    if !drive_rooted || !diagnostic_drive_is_local(bytes[0]) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_FINAL_PATH_NOT_LOCAL",
            "The opened diagnostic destination must resolve to a local fixed drive.",
        ));
    }
    Ok(dos_path.replace('/', "\\").to_lowercase())
}

#[cfg(windows)]
fn verify_open_diagnostic_file_path(file: &fs::File, expected: &Path) -> Result<(), String> {
    let resolved = diagnostic_path_from_handle(file)?;
    let expected_identity = normalized_windows_diagnostic_path(expected)?;
    let resolved_identity = normalized_windows_diagnostic_path(&resolved)?;
    if expected_identity != resolved_identity {
        return Err(diagnostic_error(
            "DIAGNOSTIC_FINAL_PATH_MISMATCH",
            "The opened destination did not resolve to the selected local path. No diagnostic bytes were written.",
        ));
    }
    verify_diagnostic_destination_identity(file, expected)?;
    Ok(())
}

#[cfg(windows)]
fn diagnostic_windows_file_identity(file: &fs::File) -> Result<(u32, u64), String> {
    use std::{mem::zeroed, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_VERIFY_FAILED",
            "Windows could not identify the opened diagnostic destination.",
        ));
    }
    Ok((
        information.dwVolumeSerialNumber,
        ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
    ))
}

#[cfg(windows)]
fn verify_diagnostic_destination_identity(file: &fs::File, expected: &Path) -> Result<(), String> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = fs::OpenOptions::new();
    options
        .read(true)
        .access_mode(0)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let probe = options.open(expected).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_DESTINATION_IDENTITY_MISMATCH",
            "The selected path no longer names the newly opened diagnostic destination.",
        )
    })?;
    let metadata = probe.metadata().map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_DESTINATION_VERIFY_FAILED",
            "The selected diagnostic destination identity could not be inspected.",
        )
    })?;
    if !metadata.file_type().is_file() || diagnostic_metadata_is_reparse(&metadata) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_TYPE_BLOCKED",
            "The selected diagnostic destination identity is not a regular non-reparse file.",
        ));
    }
    if diagnostic_windows_file_identity(file)? != diagnostic_windows_file_identity(&probe)? {
        return Err(diagnostic_error(
            "DIAGNOSTIC_DESTINATION_IDENTITY_MISMATCH",
            "The selected path no longer names the newly opened diagnostic destination.",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn verify_open_diagnostic_file_path(_file: &fs::File, expected: &Path) -> Result<(), String> {
    let resolved = fs::canonicalize(expected).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_FINAL_PATH_UNAVAILABLE",
            "The opened diagnostic destination could not be resolved safely.",
        )
    })?;
    if resolved != expected {
        return Err(diagnostic_error(
            "DIAGNOSTIC_FINAL_PATH_MISMATCH",
            "The opened destination did not resolve to the selected local path. No diagnostic bytes were written.",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn verify_diagnostic_parent_guard(guard: &fs::File, parent: &Path) -> Result<(), String> {
    let metadata = guard.metadata().map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_PARENT_GUARD_FAILED",
            "The selected destination directory could not be verified.",
        )
    })?;
    if !metadata.file_type().is_dir() || diagnostic_metadata_is_reparse(&metadata) {
        return Err(diagnostic_error(
            "DIAGNOSTIC_REPARSE_POINT_BLOCKED",
            "The selected destination directory must be a regular non-reparse directory.",
        ));
    }
    let resolved = diagnostic_path_from_handle(&guard)?;
    if normalized_windows_diagnostic_path(&resolved)? != normalized_windows_diagnostic_path(parent)?
    {
        return Err(diagnostic_error(
            "DIAGNOSTIC_PARENT_GUARD_FAILED",
            "The selected destination directory changed while it was being secured.",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn open_diagnostic_parent_guard(parent: &Path) -> Result<fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let mut options = fs::OpenOptions::new();
    options
        .read(true)
        .access_mode(0)
        .share_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let guard = options.open(parent).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_PARENT_GUARD_FAILED",
            "The selected destination directory could not be locked against replacement.",
        )
    })?;
    verify_diagnostic_parent_guard(&guard, parent)?;
    Ok(guard)
}

fn open_new_diagnostic_destination<F>(
    path: &Path,
    before_create: F,
) -> Result<OpenDiagnosticDestination, String>
where
    F: FnOnce(),
{
    validate_new_diagnostic_path(path)?;
    let expected = expected_diagnostic_destination(path)?;
    #[cfg(windows)]
    let parent_guard = open_diagnostic_parent_guard(expected.parent().ok_or_else(|| {
        diagnostic_error(
            "DIAGNOSTIC_PARENT_INVALID",
            "The selected destination has no parent directory.",
        )
    })?)?;

    before_create();
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|_| {
        diagnostic_error(
            "DIAGNOSTIC_DESTINATION_CREATE_FAILED",
            "QuickPLS could not create the selected new ZIP file.",
        )
    })?;
    #[cfg(windows)]
    verify_diagnostic_parent_guard(
        &parent_guard,
        expected.parent().ok_or_else(|| {
            diagnostic_error(
                "DIAGNOSTIC_PARENT_INVALID",
                "The selected destination has no parent directory.",
            )
        })?,
    )?;
    verify_open_diagnostic_file(&file, &expected)?;
    Ok(OpenDiagnosticDestination {
        file,
        #[cfg(windows)]
        _parent_guard: parent_guard,
    })
}

fn build_diagnostic_zip_bytes(staging: &DiagnosticStaging) -> Result<Vec<u8>, String> {
    validate_diagnostic_staging(&staging.entries)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o600);
    for (name, bytes) in &staging.entries {
        writer.start_file(*name, options).map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_ARCHIVE_BUILD_FAILED",
                "QuickPLS could not build the diagnostic archive in memory.",
            )
        })?;
        writer.write_all(bytes).map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_ARCHIVE_BUILD_FAILED",
                "QuickPLS could not build the diagnostic archive in memory.",
            )
        })?;
    }
    let archive = writer
        .finish()
        .map_err(|_| {
            diagnostic_error(
                "DIAGNOSTIC_ARCHIVE_BUILD_FAILED",
                "QuickPLS could not finalize the diagnostic archive in memory.",
            )
        })?
        .into_inner();
    if archive.len() > MAX_DIAGNOSTIC_ARCHIVE_BYTES {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ARCHIVE_TOO_LARGE",
            "The completed diagnostic ZIP exceeds its bounded size limit.",
        ));
    }
    Ok(archive)
}

fn write_diagnostic_archive_bytes(
    path: &Path,
    archive: &[u8],
) -> Result<DiagnosticBundleSaveResult, String> {
    write_diagnostic_archive_bytes_with_hook(path, archive, || {})
}

fn write_diagnostic_archive_bytes_with_hook<F>(
    path: &Path,
    archive: &[u8],
    before_create: F,
) -> Result<DiagnosticBundleSaveResult, String>
where
    F: FnOnce(),
{
    if archive.len() > MAX_DIAGNOSTIC_ARCHIVE_BYTES {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ARCHIVE_TOO_LARGE",
            "The completed diagnostic ZIP exceeds its bounded size limit.",
        ));
    }
    let saved = DiagnosticBundleSaveResult {
        bytes: archive.len() as u64,
        archive_sha256: sha256_hex(archive),
    };
    let mut destination = open_new_diagnostic_destination(path, before_create)?;
    if destination
        .file
        .write_all(archive)
        .and_then(|_| destination.file.sync_all())
        .is_err()
    {
        return Err(diagnostic_error(
            "DIAGNOSTIC_ARCHIVE_WRITE_FAILED",
            "QuickPLS could not save and synchronize the diagnostic archive. A partial new file may remain and will never be overwritten automatically.",
        ));
    }
    Ok(saved)
}

fn write_diagnostic_bundle(
    path: &Path,
    staging: &DiagnosticStaging,
) -> Result<DiagnosticBundleSaveResult, String> {
    let archive = build_diagnostic_zip_bytes(staging)?;
    write_diagnostic_archive_bytes(path, &archive)
}

fn create_diagnostic_preview(
    diagnostics: &DesktopDiagnostics,
    replaces_preview_id: Option<&str>,
) -> Result<DiagnosticBundlePreview, String> {
    diagnostics.create_preview(replaces_preview_id)
}

#[tauri::command]
fn preview_diagnostic_bundle(
    replaces_preview_id: Option<String>,
    diagnostics: State<'_, DesktopDiagnostics>,
) -> Result<DiagnosticBundlePreview, String> {
    create_diagnostic_preview(&diagnostics, replaces_preview_id.as_deref())
}

#[tauri::command]
fn cancel_diagnostic_bundle_preview(
    preview_id: String,
    diagnostics: State<'_, DesktopDiagnostics>,
) -> Result<(), String> {
    diagnostics.cancel_preview(&preview_id)?;
    diagnostics.record_event("diagnostic.preview.cancelled")?;
    Ok(())
}

#[tauri::command]
fn save_diagnostic_bundle(
    path: String,
    preview_id: String,
    diagnostics: State<'_, DesktopDiagnostics>,
) -> Result<DiagnosticBundleSaveResult, String> {
    let staging = diagnostics.consume_preview(&preview_id)?;
    let result = write_diagnostic_bundle(Path::new(&path), &staging)?;
    diagnostics.record_event("diagnostic.bundle.saved")?;
    Ok(result)
}

const INTERNAL_RECIPE_V4_PLS_COMMAND_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InternalRecipeV4ExecutionSurfaceV1 {
    Standard,
    Labs,
    InternalLabs,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InternalRecipeV4ResidentDataV1 {
    ProjectResident,
    Inline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InternalRecipeV4PlsExecutionRequestV1 {
    surface: InternalRecipeV4ExecutionSurfaceV1,
    experimental_labs_enabled: bool,
    resident_data: InternalRecipeV4ResidentDataV1,
    dataset_id: String,
    dataset_fingerprint: String,
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    compiler_target: RecipeV4CompilerTarget,
    capability_cell: CapabilityCellReferenceV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    posthoc_technical_minimum_sample_size: Option<PlsPosthocTechnicalMinimumSampleSizeConfigV2>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InternalRecipeV4ExecutionStageV1 {
    Access,
    Capability,
    DataResolution,
    Compilation,
    Projection,
    Estimation,
    Integrity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InternalRecipeV4ExecutionFailureV1 {
    schema_version: u32,
    stage: InternalRecipeV4ExecutionStageV1,
    subject: String,
    code: String,
    message: String,
    corrective_action: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    issues: Vec<InternalRecipeV4ExecutionIssueV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InternalRecipeV4ExecutionIssueV1 {
    code: String,
    subject: String,
    message: String,
}

fn internal_recipe_v4_failure(
    stage: InternalRecipeV4ExecutionStageV1,
    subject: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> InternalRecipeV4ExecutionFailureV1 {
    InternalRecipeV4ExecutionFailureV1 {
        schema_version: INTERNAL_RECIPE_V4_PLS_COMMAND_SCHEMA_VERSION,
        stage,
        subject: subject.into(),
        code: code.into(),
        message: message.into(),
        corrective_action: corrective_action.into(),
        issues: Vec::new(),
    }
}

fn validate_internal_recipe_v4_pls_access(
    request: &InternalRecipeV4PlsExecutionRequestV1,
) -> Result<(), InternalRecipeV4ExecutionFailureV1> {
    if request.surface != InternalRecipeV4ExecutionSurfaceV1::InternalLabs
        || !request.experimental_labs_enabled
    {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Access,
            "experimentalLabsEnabled",
            "recipe_v4.internal_labs_required",
            "Recipe-v4 execution is available only through the internal Experimental Labs boundary.",
            "Enable Experimental Labs and call the internal recipe-v4 service; do not expose this path in Standard Calculate.",
        ));
    }
    if request.resident_data != InternalRecipeV4ResidentDataV1::ProjectResident {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "residentData",
            "recipe_v4.project_resident_data_required",
            "Recipe-v4 execution requires a dataset resident in the active project.",
            "Import or derive the dataset in the active project, then retry with its current identity.",
        ));
    }
    if request.compiler_target != RecipeV4CompilerTarget::PlsPlanV2 {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "compilerTarget",
            "recipe_v4.pls_target_required",
            "This internal command executes only the bounded core PLS compiler target.",
            "Use compilerTarget pls_plan_v2. CB-SEM recipe-v4 execution is not active.",
        ));
    }
    let expected_capability_cell = request
        .compiler_target
        .capability_cell_for_method(request.recipe.settings.method);
    if request.capability_cell != expected_capability_cell {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "capabilityCell",
            "recipe_v4.capability_cell_mismatch",
            "The capability-cell identity does not match the selected compiler target and analytical method.",
            "Use the exact primary capability-cell reference for the requested Recipe-v4 method.",
        ));
    }
    Ok(())
}

fn resolve_internal_recipe_v4_dataset(
    project: &Project,
    request: &InternalRecipeV4PlsExecutionRequestV1,
) -> Result<Dataset, InternalRecipeV4ExecutionFailureV1> {
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == request.dataset_id)
        .ok_or_else(|| {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "datasetId",
                "recipe_v4.dataset_not_resident",
                format!(
                    "Dataset {} is not resident in the active project.",
                    request.dataset_id
                ),
                "Select a resident project dataset and rebuild the recipe-v4 request from its current identity.",
            )
        })?;
    if dataset.fingerprint.0 != request.dataset_fingerprint {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "datasetFingerprint",
            "recipe_v4.dataset_fingerprint_mismatch",
            "The requested dataset fingerprint does not match the resident dataset.",
            "Refresh the dataset binding and recompile the recipe before execution.",
        ));
    }
    Ok(dataset.clone())
}

fn map_recipe_v4_compilation_failure(
    error: RecipeV4CompilationError,
) -> InternalRecipeV4ExecutionFailureV1 {
    if let RecipeV4CompilationError::WeightCapability(issue) = &error {
        return internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Compilation,
            issue.subject.clone(),
            issue.code.as_str(),
            issue.to_string(),
            issue.corrective_action.clone(),
        );
    }
    let subject = match &error {
        RecipeV4CompilationError::CapabilityCellMismatch => "capabilityCell",
        RecipeV4CompilationError::UnresolvedModelReference
        | RecipeV4CompilationError::EmbeddedModelMismatch
        | RecipeV4CompilationError::ModelIdMismatch { .. }
        | RecipeV4CompilationError::ModelScientificDigestMismatch
        | RecipeV4CompilationError::InvalidSemModel(_)
        | RecipeV4CompilationError::ModelEstimandMismatch(_)
        | RecipeV4CompilationError::PlsCompiler(_)
        | RecipeV4CompilationError::CbsemCompiler(_) => "model",
        _ => "recipe",
    };
    internal_recipe_v4_failure(
        InternalRecipeV4ExecutionStageV1::Compilation,
        subject,
        "recipe_v4.compilation_failed",
        error.to_string(),
        "Correct the identified recipe, model, data binding, or option and compile a new immutable execution artifact.",
    )
}

fn map_recipe_v4_execution_failure(
    error: RecipeV4PlsExecutionError,
) -> InternalRecipeV4ExecutionFailureV1 {
    match error {
        RecipeV4PlsExecutionError::Compilation(error) => {
            let mut failure = map_recipe_v4_compilation_failure(error);
            failure.stage = InternalRecipeV4ExecutionStageV1::Integrity;
            failure.code = "recipe_v4.compiled_artifact_revalidation_failed".into();
            failure.corrective_action =
                "Discard the stale artifact and compile again from the exact recipe and resolved model."
                    .into();
            failure
        }
        RecipeV4PlsExecutionError::CompilerTarget(_) => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "compilerTarget",
            "recipe_v4.pls_target_required",
            error.to_string(),
            "Compile and execute only the pls_plan_v2 target through this internal command.",
        ),
        RecipeV4PlsExecutionError::PosthocOptionIdentityMismatch
        | RecipeV4PlsExecutionError::PosthocCapabilityUnavailable
        | RecipeV4PlsExecutionError::PosthocCapabilityRegistry(_) => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "posthocTechnicalMinimumSampleSize",
            "recipe_v4.posthoc_option_unavailable",
            error.to_string(),
            "Use the exact Experimental Labs post-hoc technical minimum sample-size v2 option, or omit the option from this run.",
        ),
        RecipeV4PlsExecutionError::RawDataRequired
        | RecipeV4PlsExecutionError::DatasetFingerprintMismatch
        | RecipeV4PlsExecutionError::DatasetIdMismatch { .. } => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "dataset",
            "recipe_v4.dataset_binding_failed",
            error.to_string(),
            "Refresh the resident raw-data binding and compile again before execution.",
        ),
        RecipeV4PlsExecutionError::UnknownComposite(_)
        | RecipeV4PlsExecutionError::UnknownPathParameter(_)
        | RecipeV4PlsExecutionError::UnknownInitialWeightIdentity(_, _)
        | RecipeV4PlsExecutionError::InitialWeightTranslationMismatch
        | RecipeV4PlsExecutionError::InvalidModelIdentity
        | RecipeV4PlsExecutionError::Projection(_) => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Projection,
            "model",
            "recipe_v4.projection_failed",
            error.to_string(),
            "Correct the bounded composite model and compile a new execution artifact.",
        ),
        RecipeV4PlsExecutionError::Cancelled => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Estimation,
            "execution",
            "recipe_v4.execution_cancelled",
            error.to_string(),
            "Run the internal analysis again when ready.",
        ),
        RecipeV4PlsExecutionError::Estimation(_)
        | RecipeV4PlsExecutionError::EstimatorVersionMismatch { .. }
        | RecipeV4PlsExecutionError::FixedScoreScaleReceiptMismatch
        | RecipeV4PlsExecutionError::PointEstimateAttributionMismatch
        | RecipeV4PlsExecutionError::AlgorithmConvergenceReceiptMismatch
        | RecipeV4PlsExecutionError::NonlinearEffectsIdentityMismatch => {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Estimation,
                "estimator",
                "recipe_v4.estimation_failed",
                error.to_string(),
                "Review the typed model and data diagnostics, correct the setup, and execute again.",
            )
        }
    }
}

fn execute_internal_recipe_v4_pls(
    dataset: &Dataset,
    request: &InternalRecipeV4PlsExecutionRequestV1,
) -> Result<RecipeV4PlsExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    execute_internal_recipe_v4_pls_with_control(dataset, request, || false, |_| {})
}

fn execute_internal_recipe_v4_pls_with_control(
    dataset: &Dataset,
    request: &InternalRecipeV4PlsExecutionRequestV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(qpls_runner::RunnerProgress) + Sync,
) -> Result<RecipeV4PlsExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_pls_access(request)?;
    if dataset.id.to_string() != request.dataset_id
        || dataset.fingerprint.0 != request.dataset_fingerprint
    {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "dataset",
            "recipe_v4.resident_dataset_identity_changed",
            "The resident dataset identity changed before recipe-v4 execution began.",
            "Refresh the dataset binding and compile a new immutable execution artifact.",
        ));
    }
    let artifact = compile_analysis_recipe_v4(
        &request.recipe,
        Some(&request.model),
        request.compiler_target,
        request.capability_cell.clone(),
    )
    .map_err(map_recipe_v4_compilation_failure)?;
    run_compiled_pls_recipe_v4(
        dataset,
        &request.recipe,
        &request.model,
        &artifact,
        request.posthoc_technical_minimum_sample_size.as_ref(),
        should_cancel,
        progress,
    )
    .map_err(map_recipe_v4_execution_failure)
}

/// Internal/Labs-only synchronous bridge. It deliberately returns an
/// ephemeral deterministic result and never appends recipe-v4/schema-v6 data
/// to the active schema-v5 project archive.
#[tauri::command]
fn run_internal_labs_recipe_v4_pls_execution(
    request: InternalRecipeV4PlsExecutionRequestV1,
    state: State<'_, DesktopProject>,
) -> Result<RecipeV4PlsExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_pls_access(&request)?;
    let dataset = {
        let project = state.0.lock().map_err(|_| {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "project",
                "recipe_v4.project_state_unavailable",
                "The active project data is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?;
        resolve_internal_recipe_v4_dataset(&project, &request)?
    };
    execute_internal_recipe_v4_pls(&dataset, &request)
}

#[cfg(test)]
mod internal_recipe_v4_pls_command_tests {
    use super::*;
    use qpls_core::{
        AnalysisRecipeModelBindingV4, LegacyBasicModelInterpretationV4, ObservedRoleV4,
        ObservedScaleV4, SamplingWeightNormalizationV4, SemDataBindingV4, SemParameterV4,
        SemVariableV4, SemWeightBindingV4, WeightCapabilityCodeV1, WeightCapabilityIssueV1,
        WeightCapabilityTargetV1, confirm_legacy_recipe_estimand_v4,
        migrate_analysis_recipe_to_v4_pending, resolve_weight_declaration_v1,
    };
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> (Project, InternalRecipeV4PlsExecutionRequestV1) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        let mut source = legacy.migrated_v3().unwrap();
        source.dataset_fingerprint = dataset.fingerprint.0.clone();
        source.settings.workers = 1;
        source.method_config = Some(MethodConfig::PlsAlgorithm);
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let request = InternalRecipeV4PlsExecutionRequestV1 {
            surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
            experimental_labs_enabled: true,
            resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe,
            model,
            compiler_target: RecipeV4CompilerTarget::PlsPlanV2,
            capability_cell: RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            posthoc_technical_minimum_sample_size: Some(
                qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2(),
            ),
        };
        let mut project = Project::new("Recipe-v4 internal command fixture");
        project.datasets.push(dataset);
        (project, request)
    }

    pub(crate) fn fixed_custom_fixture() -> (Project, InternalRecipeV4PlsExecutionRequestV1) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        let mut source = legacy.migrated_v3().unwrap();
        source.model.constructs[0].mode = qpls_core::MeasurementMode::Formative;
        source.dataset_fingerprint = dataset.fingerprint.0.clone();
        source.settings.workers = 1;
        source.method_config = Some(MethodConfig::PlsAlgorithm);
        let pending = migrate_analysis_recipe_to_v4_pending(&source).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source.model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        let qpls_core::SemVariableV4::Composite { weighting, .. } = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        else {
            unreachable!()
        };
        *weighting = qpls_core::CompositeWeightingV4::Custom {
            weights: std::collections::BTreeMap::from([
                ("observed:x1".into(), 2.0),
                ("observed:x2".into(), -1.0),
            ]),
            normalization: qpls_core::CompositeWeightNormalizationV4::UnitVariance,
        };
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let request = InternalRecipeV4PlsExecutionRequestV1 {
            surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
            experimental_labs_enabled: true,
            resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe,
            model,
            compiler_target: RecipeV4CompilerTarget::PlsPlanV2,
            capability_cell: RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
            posthoc_technical_minimum_sample_size: None,
        };
        let mut project = Project::new("Recipe-v4 fixed custom internal command fixture");
        project.datasets.push(dataset);
        (project, request)
    }

    pub(crate) fn nonlinear_fixture() -> (Project, InternalRecipeV4PlsExecutionRequestV1) {
        let (project, mut request) = fixture();
        request.recipe.settings.method = AnalysisMethod::NonlinearEffects;
        request.recipe.method_config = Some(MethodConfig::NonlinearEffects);
        request.capability_cell = request
            .compiler_target
            .capability_cell_for_method(AnalysisMethod::NonlinearEffects);
        request.posthoc_technical_minimum_sample_size = None;
        (project, request)
    }

    #[test]
    fn internal_labs_command_contract_executes_resident_data_without_persisting() {
        let (project, request) = fixture();
        let datasets_before = project.datasets.len();
        let recipes_before = project.recipes.len();
        let results_before = project.results.len();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();

        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();

        assert!(result.estimation().converged);
        assert_eq!(result.estimation().paths.len(), 1);
        assert_eq!(
            result.provenance().compilation_receipt().capability_cell(),
            &RecipeV4CompilerTarget::PlsPlanV2.capability_cell()
        );
        assert_eq!(
            result.provenance().posthoc_technical_minimum_sample_size(),
            request.posthoc_technical_minimum_sample_size.as_ref()
        );
        assert!(result.estimation().posthoc_minimum_sample_size.is_some());

        let mut unopted = request.clone();
        unopted.posthoc_technical_minimum_sample_size = None;
        let unopted_result = execute_internal_recipe_v4_pls(&dataset, &unopted).unwrap();
        assert!(
            unopted_result
                .provenance()
                .posthoc_technical_minimum_sample_size()
                .is_none()
        );
        assert!(
            unopted_result
                .estimation()
                .posthoc_minimum_sample_size
                .is_none()
        );
        assert_eq!(project.datasets.len(), datasets_before);
        assert_eq!(project.recipes.len(), recipes_before);
        assert_eq!(project.results.len(), results_before);
    }

    #[test]
    fn internal_labs_executes_the_exact_nonlinear_primary_cell_only() {
        let (project, request) = nonlinear_fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        assert_eq!(
            result.provenance().adapter_version(),
            qpls_runner::RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        );
        assert_eq!(
            result.provenance().estimator_method_version(),
            qpls_estimation::NONLINEAR_EFFECTS_METHOD_VERSION
        );
        assert!(result.estimation().nonlinear_effects.is_some());

        let mut standard = request.clone();
        standard.surface = InternalRecipeV4ExecutionSurfaceV1::Standard;
        standard.experimental_labs_enabled = false;
        assert_eq!(
            validate_internal_recipe_v4_pls_access(&standard)
                .unwrap_err()
                .stage,
            InternalRecipeV4ExecutionStageV1::Access
        );

        let mut base_cell = request;
        base_cell.capability_cell = RecipeV4CompilerTarget::PlsPlanV2.capability_cell();
        let failure = validate_internal_recipe_v4_pls_access(&base_cell).unwrap_err();
        assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Capability);
        assert_eq!(failure.subject, "capabilityCell");
    }

    #[test]
    fn internal_labs_access_capability_and_resident_identity_fail_with_corrections() {
        let (project, request) = fixture();

        let mut standard_request = request.clone();
        standard_request.surface = InternalRecipeV4ExecutionSurfaceV1::Standard;
        standard_request.experimental_labs_enabled = false;
        let access = validate_internal_recipe_v4_pls_access(&standard_request).unwrap_err();
        assert_eq!(access.stage, InternalRecipeV4ExecutionStageV1::Access);
        assert_eq!(access.subject, "experimentalLabsEnabled");
        assert!(!access.corrective_action.is_empty());
        let access_wire = serde_json::to_value(&access).unwrap();
        assert_eq!(access_wire["schemaVersion"], 1);
        assert_eq!(access_wire["stage"], "access");
        assert_eq!(access_wire["correctiveAction"], access.corrective_action);

        let mut labs_only_request = request.clone();
        labs_only_request.surface = InternalRecipeV4ExecutionSurfaceV1::Labs;
        assert_eq!(
            validate_internal_recipe_v4_pls_access(&labs_only_request)
                .unwrap_err()
                .stage,
            InternalRecipeV4ExecutionStageV1::Access
        );

        let mut inline_request = request.clone();
        inline_request.resident_data = InternalRecipeV4ResidentDataV1::Inline;
        assert_eq!(
            validate_internal_recipe_v4_pls_access(&inline_request)
                .unwrap_err()
                .stage,
            InternalRecipeV4ExecutionStageV1::DataResolution
        );

        let mut wrong_target = request.clone();
        wrong_target.compiler_target = RecipeV4CompilerTarget::CbsemPlanV2;
        wrong_target.capability_cell = RecipeV4CompilerTarget::CbsemPlanV2.capability_cell();
        let capability = validate_internal_recipe_v4_pls_access(&wrong_target).unwrap_err();
        assert_eq!(
            capability.stage,
            InternalRecipeV4ExecutionStageV1::Capability
        );
        assert_eq!(capability.subject, "compilerTarget");
        assert!(!capability.corrective_action.is_empty());

        let mut stale = request.clone();
        stale.dataset_fingerprint = "stale-fingerprint".into();
        let data = resolve_internal_recipe_v4_dataset(&project, &stale).unwrap_err();
        assert_eq!(data.stage, InternalRecipeV4ExecutionStageV1::DataResolution);
        assert_eq!(data.subject, "datasetFingerprint");
        assert!(!data.corrective_action.is_empty());
    }

    #[test]
    fn ignored_parameter_options_fail_at_the_typed_compilation_stage() {
        let (project, mut request) = fixture();
        let SemParameterV4::Free { start, .. } = &mut request.model.parameters[0] else {
            unreachable!()
        };
        *start = Some(0.25);
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();

        let failure = execute_internal_recipe_v4_pls(&dataset, &request).unwrap_err();

        assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Compilation);
        assert_eq!(failure.subject, "model");
        assert_eq!(failure.code, "recipe_v4.compilation_failed");
        assert!(!failure.corrective_action.is_empty());
    }

    fn resolved_weight_issue(
        binding: SemWeightBindingV4,
        target: WeightCapabilityTargetV1,
    ) -> WeightCapabilityIssueV1 {
        let (_, request) = fixture();
        let mut model = request.model;
        model.variables.push(SemVariableV4::Observed {
            id: "observed:survey_weight".into(),
            label: "Survey weight".into(),
            source_column: "survey_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let SemDataBindingV4::Raw { weight, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *weight = Some(binding);
        let declaration = resolve_weight_declaration_v1(&model).unwrap().unwrap();
        WeightCapabilityIssueV1::unsupported(target, declaration)
    }

    #[test]
    fn native_compilation_mapping_preserves_typed_pls_and_cbsem_weight_diagnostics() {
        let bindings = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:survey_weight".into(),
                },
                WeightCapabilityCodeV1::CaseWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:survey_weight".into(),
                },
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:survey_weight".into(),
                    normalization: SamplingWeightNormalizationV4::MeanOne,
                },
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
            ),
        ];
        for target in [
            WeightCapabilityTargetV1::PlsPlanV2,
            WeightCapabilityTargetV1::CbsemMlV1,
        ] {
            for (binding, expected_code) in &bindings {
                let issue = resolved_weight_issue(binding.clone(), target);
                let expected_message = issue.to_string();
                let expected_action = issue.corrective_action.clone();
                let failure = map_recipe_v4_compilation_failure(
                    RecipeV4CompilationError::WeightCapability(issue),
                );

                assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Compilation);
                assert_eq!(failure.code, expected_code.as_str());
                assert_eq!(failure.subject, "observed:survey_weight");
                assert_eq!(failure.message, expected_message);
                assert_eq!(failure.corrective_action, expected_action);
                assert!(failure.issues.is_empty());
            }
        }
    }

    #[test]
    fn native_compilation_mapping_preserves_exact_legacy_weight_ambiguity() {
        for target in [
            WeightCapabilityTargetV1::PlsPlanV2,
            WeightCapabilityTargetV1::CbsemMlV1,
        ] {
            let issue = WeightCapabilityIssueV1::legacy_case_weight_binding_ambiguous(
                target,
                " case_weight ",
                None,
            )
            .unwrap();
            let expected_message = issue.to_string();
            let expected_action = issue.corrective_action.clone();
            let failure = map_recipe_v4_compilation_failure(
                RecipeV4CompilationError::WeightCapability(issue),
            );

            assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Compilation);
            assert_eq!(failure.code, "legacy_case_weight_binding_ambiguous");
            assert_eq!(failure.subject, " case_weight ");
            assert_eq!(failure.message, expected_message);
            assert_eq!(failure.corrective_action, expected_action);
            assert!(failure.issues.is_empty());
        }
    }

    #[test]
    fn initialization_translation_failures_map_to_the_fail_closed_projection_issue() {
        for error in [
            RecipeV4PlsExecutionError::UnknownInitialWeightIdentity(
                "construct:x".into(),
                "observed:missing".into(),
            ),
            RecipeV4PlsExecutionError::InitialWeightTranslationMismatch,
        ] {
            let failure = map_recipe_v4_execution_failure(error);

            assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Projection);
            assert_eq!(failure.subject, "model");
            assert_eq!(failure.code, "recipe_v4.projection_failed");
            assert!(!failure.corrective_action.is_empty());
        }
    }

    #[test]
    fn request_wire_is_camel_case_strict_and_keeps_scientific_payloads_typed() {
        let (_, request) = fixture();
        let mut value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["surface"], "internal_labs");
        assert_eq!(value["residentData"], "project_resident");
        assert_eq!(value["compilerTarget"], "pls_plan_v2");
        assert_eq!(
            value["posthocTechnicalMinimumSampleSize"]["base_analysis"],
            "pls_algorithm"
        );
        assert_eq!(
            value["posthocTechnicalMinimumSampleSize"]["inference"],
            "point_estimate_only"
        );
        assert_eq!(value["recipe"]["schema_version"], 4);
        assert_eq!(value["model"]["schema_version"], 4);
        value["standardAvailable"] = serde_json::json!(true);
        assert!(serde_json::from_value::<InternalRecipeV4PlsExecutionRequestV1>(value).is_err());
    }

    #[test]
    fn posthoc_option_tampering_fails_closed_at_the_capability_stage() {
        let (project, mut request) = fixture();
        request
            .posthoc_technical_minimum_sample_size
            .as_mut()
            .unwrap()
            .method_version = "inverse_square_root_posthoc_v1".into();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();

        let failure = execute_internal_recipe_v4_pls(&dataset, &request).unwrap_err();
        assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Capability);
        assert_eq!(failure.subject, "posthocTechnicalMinimumSampleSize");
        assert_eq!(failure.code, "recipe_v4.posthoc_option_unavailable");
        assert!(!failure.corrective_action.is_empty());
    }
}

#[tauri::command]
fn validate_analysis_recipe(recipe: AnalysisRecipe) -> Vec<ValidationIssue> {
    validate_recipe(&recipe)
}

#[tauri::command]
fn method_capabilities() -> Vec<MethodCapability> {
    METHOD_CAPABILITIES.to_vec()
}

#[tauri::command]
fn export_xlsx_tables(path: String, tables: Vec<ExportTable>) -> Result<(), String> {
    write_xlsx_tables(Path::new(&path), &tables).map_err(|error| error.to_string())
}
#[tauri::command]
fn export_text_file(path: String, contents: String) -> Result<(), String> {
    write_text_export(Path::new(&path), &contents)
}

#[tauri::command]
fn open_default_export_folder() -> Result<String, String> {
    let folder = default_export_folder();
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    open_folder(&folder)?;
    Ok(folder.to_string_lossy().to_string())
}

#[tauri::command]
fn verify_latest_release_checksums() -> Result<ChecksumVerification, String> {
    let Some(checksum_file) = latest_checksum_file() else {
        return Ok(ChecksumVerification {
            checksum_file: None,
            checked: 0,
            verified: 0,
            failures: vec![],
            message: "No release checksum file was found in target/release/artifacts.".to_string(),
        });
    };
    let content = fs::read_to_string(&checksum_file).map_err(|error| error.to_string())?;
    let mut checked = 0usize;
    let mut verified = 0usize;
    let mut failures = Vec::new();
    for line in content.lines() {
        let Some((expected, file_name)) = parse_checksum_line(line) else {
            continue;
        };
        let artifact = checksum_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(file_name);
        checked += 1;
        match fs::read(&artifact) {
            Ok(bytes) => {
                let actual = sha256_hex(&bytes);
                if actual.eq_ignore_ascii_case(&expected) {
                    verified += 1;
                } else {
                    failures.push(format!(
                        "{} expected {} but found {}",
                        artifact.display(),
                        expected,
                        actual
                    ));
                }
            }
            Err(error) => failures.push(format!(
                "{} could not be read: {}",
                artifact.display(),
                error
            )),
        }
    }
    let message = if checked == 0 {
        "Checksum file was found, but no artifact entries were parsed.".to_string()
    } else if failures.is_empty() {
        format!("Verified {verified}/{checked} release artifact checksum(s).")
    } else {
        format!(
            "Verified {verified}/{checked}; {} failure(s).",
            failures.len()
        )
    };
    Ok(ChecksumVerification {
        checksum_file: Some(checksum_file.to_string_lossy().to_string()),
        checked,
        verified,
        failures,
        message,
    })
}

fn default_export_folder() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| home.join("Documents").join("QuickPLS").join("Exports"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("exports")
        })
}

fn open_folder(folder: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(folder)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(folder)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn latest_checksum_file() -> Option<PathBuf> {
    let mut dirs = vec![
        std::env::current_dir()
            .ok()?
            .join("target")
            .join("release")
            .join("artifacts"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
            dirs.push(parent.join("artifacts"));
        }
    }
    dirs.into_iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_checksums.txt"))
        })
        .filter_map(|path| {
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn parse_checksum_line(line: &str) -> Option<(String, String)> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let hex_index = parts
        .iter()
        .position(|part| part.len() == 64 && part.chars().all(|ch| ch.is_ascii_hexdigit()))?;
    let file_index = if hex_index == 0 { 1 } else { 0 };
    let file_name = parts
        .get(file_index)?
        .trim_start_matches('*')
        .trim_matches('"');
    if file_name.ends_with("_checksums.txt") {
        return None;
    }
    Some((parts[hex_index].to_string(), file_name.to_string()))
}
fn write_text_export(path: &Path, contents: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(
            "Text exports require an absolute path selected by the Save dialog.".to_string(),
        );
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "Text exports require a .csv, .html, or .svg file extension.".to_string())?;
    if !matches!(extension.as_str(), "csv" | "html" | "svg") {
        return Err("Only .csv, .html, and .svg text exports are supported.".to_string());
    }
    if contents.len() > MAX_TEXT_EXPORT_BYTES {
        return Err(format!(
            "Text export exceeds the {} MiB safety limit.",
            MAX_TEXT_EXPORT_BYTES / 1024 / 1024
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| "The selected export location has no parent directory.".to_string())?;
    if !parent.is_dir() {
        return Err("The selected export directory does not exist.".to_string());
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("QuickPLS will not overwrite a symbolic link.".to_string());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    fs::write(path, contents.as_bytes()).map_err(|error| error.to_string())
}

fn write_xlsx_tables(
    path: &Path,
    tables: &[ExportTable],
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let mut used_sheet_names = BTreeSet::new();
    for (index, table) in tables.iter().enumerate() {
        let worksheet = workbook.add_worksheet();
        let sheet_name = unique_sheet_name(&table.title, index, &mut used_sheet_names);
        worksheet.set_name(&sheet_name)?;
        worksheet.write_string(0, 0, &table.title)?;
        worksheet.write_string(1, 0, "Status")?;
        worksheet.write_string(1, 1, &table.status)?;
        worksheet.write_string(2, 0, "Warning")?;
        worksheet.write_string(2, 1, table.warning.as_deref().unwrap_or(""))?;
        for (column, header) in table.columns.iter().enumerate() {
            worksheet.write_string(4, column as u16, header)?;
        }
        for (row_index, row) in table.rows.iter().enumerate() {
            for (column, value) in row.iter().enumerate() {
                worksheet.write_string((row_index + 5) as u32, column as u16, value)?;
            }
        }
        worksheet.autofit();
    }
    workbook.save(path)
}

fn unique_sheet_name(title: &str, index: usize, used: &mut BTreeSet<String>) -> String {
    let base = safe_sheet_name(title, index);
    let mut ordinal = 1usize;
    loop {
        let candidate = if ordinal == 1 {
            base.clone()
        } else {
            let suffix = format!(" ({ordinal})");
            let prefix_length = 31usize.saturating_sub(suffix.chars().count());
            format!(
                "{}{}",
                base.chars().take(prefix_length).collect::<String>(),
                suffix
            )
        };
        if used.insert(candidate.to_lowercase()) {
            return candidate;
        }
        ordinal += 1;
    }
}

fn safe_sheet_name(title: &str, index: usize) -> String {
    let cleaned = title
        .chars()
        .map(|ch| match ch {
            ':' | '\\' | '/' | '?' | '*' | '[' | ']' => ' ',
            other => other,
        })
        .collect::<String>();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let fallback = format!("Table {}", index + 1);
    let name = if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    };
    name.chars()
        .take(31)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[tauri::command]
fn new_project(
    name: String,
    project_mode: Option<GeneralSemNewProjectModeV1>,
    state: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
    native_adoption_authority: State<'_, DesktopSchema6NativeAdoptionAuthorityV1>,
) -> Result<ProjectSnapshot, String> {
    let project = Project::new(name);
    let response = snapshot(&project, None, None)?;
    native_adoption_authority.replace_from_new_project(
        &state.0,
        fresh_draft_authority.inner(),
        project,
        project_mode,
    )?;
    Ok(response)
}

#[tauri::command]
fn import_dataset(
    path: String,
    data_kind: Option<DataKind>,
    sample_size: Option<usize>,
    missing_markers: Option<Vec<String>>,
    state: State<'_, DesktopProject>,
) -> Result<DatasetSnapshot, String> {
    let options = ImportOptions {
        data_kind: data_kind.unwrap_or(DataKind::Raw),
        sample_size,
        missing_markers: missing_markers
            .unwrap_or_else(|| ImportOptions::default().missing_markers),
        ..ImportOptions::default()
    };
    let dataset = import_path(Path::new(&path), &options).map_err(|error| error.to_string())?;
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    append_dataset(&mut project, dataset)
}

#[tauri::command]
fn dataset_rows(
    dataset_id: String,
    offset: usize,
    limit: usize,
    state: State<'_, DesktopProject>,
) -> Result<DatasetRowsPage, String> {
    let project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    dataset_rows_page(&project, &dataset_id, offset, limit)
}

#[tauri::command]
fn profile_dataset_groups(
    dataset_id: String,
    column_name: String,
    analysis_columns: Vec<String>,
    state: State<'_, DesktopProject>,
) -> Result<DatasetGroupProfile, String> {
    let project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    build_dataset_group_profile(&project, &dataset_id, &column_name, &analysis_columns)
}

fn build_dataset_group_profile(
    project: &Project,
    dataset_id: &str,
    column_name: &str,
    analysis_columns: &[String],
) -> Result<DatasetGroupProfile, String> {
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    build_dataset_group_profile_for_dataset(dataset, column_name, analysis_columns)
}

fn build_dataset_group_profile_for_dataset(
    dataset: &Dataset,
    column_name: &str,
    analysis_columns: &[String],
) -> Result<DatasetGroupProfile, String> {
    if analysis_columns.iter().any(|column| column == column_name) {
        return Err("the grouping column cannot also be a model indicator".into());
    }
    let mut unique_analysis_columns = BTreeSet::new();
    for column in analysis_columns {
        if column.trim().is_empty() || !unique_analysis_columns.insert(column.as_str()) {
            return Err("analysis columns must be non-empty and unique".into());
        }
    }
    if dataset.schema.kind != DataKind::Raw {
        return Err("group profiling requires raw observations".into());
    }
    let schema = dataset.batch.schema();
    let group_position = schema
        .index_of(column_name)
        .map_err(|_| format!("unknown grouping column {column_name}"))?;
    let analysis_positions = analysis_columns
        .iter()
        .map(|column| {
            schema
                .index_of(column)
                .map_err(|_| format!("unknown analysis column {column}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (column, position) in analysis_columns.iter().zip(&analysis_positions) {
        let array = dataset.batch.column(*position);
        if array.as_any().downcast_ref::<Float64Array>().is_none()
            && array.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(format!("analysis column {column} is not numeric"));
        }
    }
    let group_array = dataset.batch.column(group_position);
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    let mut missing_count = 0usize;
    let mut unsupported_count = 0usize;
    for row in 0..dataset.batch.num_rows() {
        let value = match canonical_group_value(group_array.as_ref(), row) {
            GroupCellValue::Value(value) => value,
            GroupCellValue::Missing => {
                missing_count += 1;
                continue;
            }
            GroupCellValue::Unsupported => {
                unsupported_count += 1;
                continue;
            }
        };
        let complete = analysis_positions.iter().all(|position| {
            let array = dataset.batch.column(*position);
            !array.is_null(row)
                && if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
                    values.value(row).is_finite()
                } else {
                    array.as_any().downcast_ref::<Int64Array>().is_some()
                }
        });
        let entry = counts.entry(value).or_default();
        entry.0 += 1;
        if complete {
            entry.1 += 1;
        }
    }
    let column_metadata = dataset
        .schema
        .columns
        .iter()
        .find(|column| column.name == column_name)
        .ok_or_else(|| format!("grouping column metadata is missing for {column_name}"))?;
    let truncated = counts.len() > MAX_GROUP_PROFILE_VALUES;
    let groups = counts
        .into_iter()
        .take(MAX_GROUP_PROFILE_VALUES)
        .map(
            |(value, (observations, complete_cases))| DatasetGroupProfileValue {
                label: column_metadata.value_labels.get(&value).cloned(),
                value,
                observations,
                complete_cases,
            },
        )
        .collect();
    Ok(DatasetGroupProfile {
        dataset_id: dataset.id.to_string(),
        column_name: column_name.to_owned(),
        row_count: dataset.batch.num_rows(),
        missing_count,
        unsupported_count,
        truncated,
        groups,
    })
}

enum GroupCellValue {
    Value(String),
    Missing,
    Unsupported,
}

fn canonical_group_value(array: &dyn Array, row: usize) -> GroupCellValue {
    if array.is_null(row) {
        return GroupCellValue::Missing;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        let value = values.value(row).trim();
        return if value.is_empty() {
            GroupCellValue::Missing
        } else {
            GroupCellValue::Value(value.to_owned())
        };
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return GroupCellValue::Value(values.value(row).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return GroupCellValue::Value(values.value(row).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        let value = values.value(row);
        if !value.is_finite() {
            return GroupCellValue::Unsupported;
        }
        return GroupCellValue::Value(if value.fract().abs() <= f64::EPSILON {
            format!("{value:.0}")
        } else {
            value.to_string()
        });
    }
    GroupCellValue::Unsupported
}

fn validate_mga_dataset_contract(dataset: &Dataset, recipe: &AnalysisRecipe) -> Result<(), String> {
    if recipe.settings.method != AnalysisMethod::Mga {
        return Ok(());
    }
    let micom_only = matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::Micom { .. })
    );
    let group_methods = recipe
        .metadata
        .get("group_methods")
        .map(|methods| {
            methods
                .split(',')
                .map(str::trim)
                .filter(|method| !method.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let exact_micom = group_methods.len() == 1 && group_methods[0].eq_ignore_ascii_case("micom");
    let legacy_combined = group_methods.len() == 2
        && group_methods
            .iter()
            .any(|method| method.eq_ignore_ascii_case("mga_permutation"))
        && group_methods
            .iter()
            .any(|method| method.eq_ignore_ascii_case("micom"));
    if (micom_only && !exact_micom) || (!micom_only && !legacy_combined) {
        return Err(if micom_only {
            "method_config.kind=micom requires MICOM only"
        } else {
            "Historical combined group analysis requires both permutation MGA and MICOM"
        }
        .into());
    }
    if !recipe
        .metadata
        .get("micom_configural_confirmed")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return Err("micom.configural_invariance_not_confirmed: MICOM Step 1 requires explicit researcher confirmation".into());
    }
    if !recipe
        .metadata
        .get("group_permutation_samples")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .is_some_and(|samples| (5_000..=10_000).contains(&samples))
    {
        return Err("The selected two-group workflow requires 5000 to 10000 permutations".into());
    }
    let group_column = recipe
        .metadata
        .get("mga_group_column")
        .or_else(|| recipe.metadata.get("mga.group_column"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "MGA requires a grouping column".to_owned())?;
    let group_a = recipe
        .metadata
        .get("mga_group_a")
        .or_else(|| recipe.metadata.get("mga.group_a"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "MGA requires an explicit Group A value".to_owned())?;
    let group_b = recipe
        .metadata
        .get("mga_group_b")
        .or_else(|| recipe.metadata.get("mga.group_b"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "MGA requires an explicit Group B value".to_owned())?;
    let analysis_columns = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().cloned())
        .collect::<Vec<_>>();
    let profile =
        build_dataset_group_profile_for_dataset(dataset, group_column, &analysis_columns)?;
    let mut selected_counts = Vec::with_capacity(2);
    for (role, selected) in [("Group A", group_a), ("Group B", group_b)] {
        let group = profile
            .groups
            .iter()
            .find(|group| group.value == selected)
            .ok_or_else(|| {
                if profile.truncated {
                    format!(
                        "{role} value '{selected}' is not available in the bounded group profile"
                    )
                } else {
                    format!(
                        "micom.empty_group: {role} value '{selected}' is not observed in {group_column}"
                    )
                }
            })?;
        if group.complete_cases < 10 {
            return Err(format!(
                "micom.group_too_small: {role} value '{selected}' has {} complete model cases; at least 10 are required",
                group.complete_cases
            ));
        }
        selected_counts.push(group.complete_cases);
    }
    if micom_only
        && selected_counts[0].max(selected_counts[1])
            > selected_counts[0]
                .min(selected_counts[1])
                .saturating_mul(10)
    {
        return Err(format!(
            "micom.extreme_group_imbalance: selected group sizes {} and {} exceed the bounded 10:1 ratio",
            selected_counts[0], selected_counts[1]
        ));
    }
    Ok(())
}

#[tauri::command]
fn import_validation_fixture(state: State<'_, DesktopProject>) -> Result<DatasetSnapshot, String> {
    let dataset = import_delimited_bytes(
        include_bytes!("../../validation/fixtures/corporate_reputation.csv"),
        "corporate_reputation.csv",
        b',',
        &ImportOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    append_dataset(&mut project, dataset)
}

fn append_dataset(project: &mut Project, dataset: Dataset) -> Result<DatasetSnapshot, String> {
    require_writable_project(project, "import data")?;
    let record = DatasetVersionRecord {
        dataset_id: dataset.id.to_string(),
        parent_dataset_id: None,
        operation: DatasetVersionOperation::Import,
        created_at: Some(Utc::now().to_rfc3339()),
        summary: format!("Imported {}", dataset.name),
        source_column: None,
        target_column: None,
        transformation: None,
    };
    Ok(commit_dataset_version(project, dataset, record)?.dataset)
}

#[tauri::command]
fn open_demo_project(
    sample_id: Option<String>,
    state: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
    native_adoption_authority: State<'_, DesktopSchema6NativeAdoptionAuthorityV1>,
) -> Result<ProjectSnapshot, String> {
    let project = build_sample_project(sample_id.as_deref().unwrap_or("corporate_reputation"))?;
    let response = snapshot(&project, None, None)?;
    native_adoption_authority.replace_ordinary_project(
        &state.0,
        fresh_draft_authority.inner(),
        project,
    )?;
    Ok(response)
}

fn build_sample_project(sample_id: &str) -> Result<Project, String> {
    build_bundled_sample_project(BundledSampleProject::parse(sample_id)?)
}

#[tauri::command]
fn set_column_metadata(
    dataset_id: String,
    column_name: String,
    metadata: ColumnMetadata,
    state: State<'_, DesktopProject>,
) -> Result<DatasetSnapshot, String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    version_column_metadata(&mut project, &dataset_id, &column_name, metadata)
}

fn version_column_metadata(
    project: &mut Project,
    dataset_id: &str,
    column_name: &str,
    metadata: ColumnMetadata,
) -> Result<DatasetSnapshot, String> {
    require_writable_project(project, "edit variable metadata")?;
    let source = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    if source
        .schema
        .columns
        .iter()
        .find(|column| column.name == column_name)
        .is_some_and(|column| column == &metadata)
    {
        return Ok(dataset_snapshot(source));
    }

    // Metadata is fingerprinted analysis input. Keep the prior dataset version so
    // completed results can still resolve the exact source fingerprint they used.
    let mut version = source.clone();
    version.id = Uuid::new_v4();
    update_column_metadata(&mut version, column_name, metadata)
        .map_err(|error| error.to_string())?;
    let record = DatasetVersionRecord {
        dataset_id: version.id.to_string(),
        parent_dataset_id: Some(source.id.to_string()),
        operation: DatasetVersionOperation::Metadata,
        created_at: Some(Utc::now().to_rfc3339()),
        summary: format!("Updated metadata for {column_name}"),
        source_column: Some(column_name.to_owned()),
        target_column: None,
        transformation: None,
    };
    Ok(commit_dataset_version(project, version, record)?.dataset)
}

#[tauri::command]
fn recode_dataset_column(
    dataset_id: String,
    spec: RecodeColumnSpec,
    state: State<'_, DesktopProject>,
) -> Result<DatasetVersionMutation, String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    version_recode_column(&mut project, &dataset_id, spec)
}

fn version_recode_column(
    project: &mut Project,
    dataset_id: &str,
    spec: RecodeColumnSpec,
) -> Result<DatasetVersionMutation, String> {
    require_writable_project(project, "recode data")?;
    let source = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    let version = recode_column(source, &spec).map_err(|error| error.to_string())?;
    let record = DatasetVersionRecord {
        dataset_id: version.id.to_string(),
        parent_dataset_id: Some(source.id.to_string()),
        operation: DatasetVersionOperation::Recode,
        created_at: Some(Utc::now().to_rfc3339()),
        summary: format!("Recoded {} into {}", spec.source_column, spec.target_column),
        source_column: Some(spec.source_column),
        target_column: Some(spec.target_column),
        transformation: None,
    };
    commit_dataset_version(project, version, record)
}

fn dataset_transformation_command_error(
    code: &str,
    field: &str,
    message: impl Into<String>,
) -> DatasetTransformationErrorV2 {
    DatasetTransformationErrorV2 {
        issues: vec![DatasetTransformationIssueV2 {
            code: code.to_owned(),
            field: field.to_owned(),
            message: message.into(),
            row_index: None,
        }],
    }
}

#[tauri::command]
fn preview_dataset_transformation(
    dataset_id: String,
    spec: DatasetTransformationSpecV2,
    state: State<'_, DesktopProject>,
) -> Result<DatasetTransformationPreviewV2, DatasetTransformationErrorV2> {
    let project = state.0.lock().map_err(|_| {
        dataset_transformation_command_error(
            "project.state_unavailable",
            "project",
            "The project data is temporarily unavailable.",
        )
    })?;
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| {
            dataset_transformation_command_error(
                "dataset.unknown",
                "dataset_id",
                format!("Dataset {dataset_id} is not available in this project."),
            )
        })?;
    Ok(preview_dataset_transformation_kernel_v2(dataset, &spec))
}

#[tauri::command]
fn apply_dataset_transformation(
    dataset_id: String,
    spec: DatasetTransformationSpecV2,
    output_dataset_name: String,
    state: State<'_, DesktopProject>,
) -> Result<DatasetVersionMutation, DatasetTransformationErrorV2> {
    let mut project = state.0.lock().map_err(|_| {
        dataset_transformation_command_error(
            "project.state_unavailable",
            "project",
            "The project data is temporarily unavailable.",
        )
    })?;
    version_dataset_transformation(&mut project, &dataset_id, spec, output_dataset_name)
}

fn version_dataset_transformation(
    project: &mut Project,
    dataset_id: &str,
    spec: DatasetTransformationSpecV2,
    output_dataset_name: String,
) -> Result<DatasetVersionMutation, DatasetTransformationErrorV2> {
    require_writable_project(&project, "derive a variable").map_err(|message| {
        dataset_transformation_command_error("project.read_only", "project", message)
    })?;
    let source = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| {
            dataset_transformation_command_error(
                "dataset.unknown",
                "dataset_id",
                format!("Dataset {dataset_id} is not available in this project."),
            )
        })?;
    let options = ApplyDatasetTransformationOptionsV2 {
        output_dataset_id: Uuid::new_v4().to_string(),
        output_dataset_name,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    };
    let mutation = apply_dataset_transformation_v2(source, &spec, &options)?;
    let input_columns = spec.input_columns();
    let output_columns = spec.target_columns();
    let summary = if input_columns.is_empty() {
        format!("Added {}", output_columns.join(", "))
    } else {
        format!(
            "Derived {} from {}",
            output_columns.join(", "),
            input_columns.join(", ")
        )
    };
    let record = DatasetVersionRecord {
        dataset_id: mutation.dataset.id.to_string(),
        parent_dataset_id: Some(source.id.to_string()),
        operation: DatasetVersionOperation::Transform,
        created_at: Some(mutation.lineage.created_at.clone()),
        summary,
        source_column: input_columns.first().cloned(),
        target_column: output_columns.first().cloned(),
        transformation: Some(mutation.lineage),
    };
    commit_dataset_version(project, mutation.dataset, record).map_err(|message| {
        dataset_transformation_command_error("project.commit_failed", "project", message)
    })
}

#[tauri::command]
fn activate_dataset(
    dataset_id: String,
    state: State<'_, DesktopProject>,
) -> Result<DatasetSnapshot, String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    activate_dataset_version(&mut project, &dataset_id)
}

fn activate_dataset_version(
    project: &mut Project,
    dataset_id: &str,
) -> Result<DatasetSnapshot, String> {
    let response = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .map(dataset_snapshot)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    if !project.read_only {
        let workspace = workspace_with_active_dataset(&project, &dataset_id)?;
        project.layouts.insert("workspace".to_owned(), workspace);
    }
    Ok(response)
}

fn commit_dataset_version(
    project: &mut Project,
    dataset: Dataset,
    record: DatasetVersionRecord,
) -> Result<DatasetVersionMutation, String> {
    let mut lineage = read_dataset_lineage(project)?.unwrap_or_default();
    if lineage
        .records
        .iter()
        .any(|current| current.dataset_id == record.dataset_id)
    {
        return Err(format!(
            "dataset version {} already has a lineage record",
            record.dataset_id
        ));
    }
    lineage.records.push(record.clone());
    let mut candidate_datasets = project.datasets.clone();
    candidate_datasets.push(dataset.clone());
    validate_data_lineage_resident_v1(&candidate_datasets, Some(&lineage))
        .map_err(|error| error.to_string())?;
    let workspace = workspace_with_active_dataset(project, &record.dataset_id)?;
    let response = DatasetVersionMutation {
        dataset: dataset_snapshot(&dataset),
        version: record,
    };

    project.datasets.push(dataset);
    write_project_data_lineage_v1(&mut project.layouts, &lineage)
        .map_err(|error| error.to_string())?;
    project.layouts.insert("workspace".to_owned(), workspace);
    Ok(response)
}

fn read_dataset_lineage(project: &Project) -> Result<Option<DatasetLineageLayout>, String> {
    validate_project_data_lineage_resident_v1(&project.datasets, &project.layouts)
        .map_err(|error| format!("dataset lineage is invalid: {error}"))
}

fn workspace_with_active_dataset(project: &Project, dataset_id: &str) -> Result<Value, String> {
    let mut workspace = project
        .layouts
        .get("workspace")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let object = workspace
        .as_object_mut()
        .ok_or_else(|| "project workspace layout is invalid".to_owned())?;
    object.insert(
        "activeDatasetId".to_owned(),
        Value::String(dataset_id.to_owned()),
    );
    Ok(workspace)
}

/// Central mutation boundary for future-schema and otherwise read-only
/// projects. Every persistent desktop mutation should pass through this guard
/// before cloning or changing project state.
fn require_writable_project(project: &Project, operation: &str) -> Result<(), String> {
    if project.read_only {
        return Err(format!(
            "cannot {operation}: the project is read-only because its archive schema is newer than this application"
        ));
    }
    Ok(())
}

#[tauri::command]
fn open_project(
    path: String,
    state: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
    native_adoption_authority: State<'_, DesktopSchema6NativeAdoptionAuthorityV1>,
) -> Result<ProjectSnapshot, String> {
    let (project, recovery_source) =
        load_project_with_autosave(Path::new(&path)).map_err(|error| error.to_string())?;
    let response = snapshot(&project, Some(path), recovery_source)?;
    native_adoption_authority.replace_ordinary_project(
        &state.0,
        fresh_draft_authority.inner(),
        project,
    )?;
    Ok(response)
}

#[tauri::command]
fn save_active_project(
    path: String,
    workspace: Value,
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
    state: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
) -> Result<ProjectSnapshot, String> {
    save_active_project_with_fresh_draft_authority(
        path,
        workspace,
        model,
        model_presentation,
        &state.0,
        fresh_draft_authority.inner(),
    )
}

pub(crate) fn save_active_project_with_fresh_draft_authority(
    path: String,
    workspace: Value,
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
    active_project: &Arc<Mutex<Project>>,
    fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
) -> Result<ProjectSnapshot, String> {
    let mut project = fresh_draft_authority.lock_project_for_unmarked_save(active_project)?;
    let mut candidate =
        project_with_workspace_model(&project, workspace, model, model_presentation)?;
    let manifest = save_project(Path::new(&path), &candidate).map_err(|error| error.to_string())?;
    candidate
        .adopt_explicit_save(manifest)
        .map_err(|error| error.to_string())?;
    *project = candidate;
    let save_warning = discard_autosave(Path::new(&path)).err().map(|error| {
        format!("the project was saved, but its stale autosave could not be removed: {error}")
    });
    let mut response = snapshot(&project, Some(path), None)?;
    response.save_warning = save_warning;
    Ok(response)
}

#[tauri::command]
fn autosave_active_project(
    path: String,
    workspace: Value,
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
    state: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
) -> Result<(), String> {
    autosave_active_project_with_fresh_draft_authority(
        path,
        workspace,
        model,
        model_presentation,
        &state.0,
        fresh_draft_authority.inner(),
    )
}

pub(crate) fn autosave_active_project_with_fresh_draft_authority(
    path: String,
    workspace: Value,
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
    active_project: &Arc<Mutex<Project>>,
    fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
) -> Result<(), String> {
    let mut project = fresh_draft_authority.lock_project_for_unmarked_save(active_project)?;
    let candidate = project_with_workspace_model(&project, workspace, model, model_presentation)?;
    save_autosave(Path::new(&path), &candidate).map_err(|error| error.to_string())?;
    *project = candidate;
    Ok(())
}

fn project_with_workspace_model(
    project: &Project,
    mut workspace: Value,
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
) -> Result<Project, String> {
    require_writable_project(project, "save the project")?;
    let workspace_object = workspace
        .as_object_mut()
        .ok_or_else(|| "project workspace layout is invalid".to_owned())?;
    let mut candidate = project.clone();
    if model.is_none() && model_presentation.is_some() {
        return Err("a model presentation requires its canonical model".to_owned());
    }
    let mut persisted_model_id = None;
    if let Some(model) = model {
        persisted_model_id = Some(model.id);
        workspace_object.insert(
            "activeModelId".to_owned(),
            Value::String(model.id.to_string()),
        );
        if let Some(existing) = candidate
            .models
            .iter_mut()
            .find(|existing| existing.id == model.id)
        {
            *existing = model;
        } else {
            candidate.models.push(model);
        }
    } else if let Some(active_model_id) = workspace_object
        .get("activeModelId")
        .and_then(Value::as_str)
    {
        let canonical_model_ids = candidate
            .models
            .iter()
            .map(|model| model.id.to_string())
            .chain(
                candidate
                    .recipes
                    .iter()
                    .map(|recipe| recipe.model.id.to_string()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        if !canonical_model_ids.contains(active_model_id) {
            workspace_object.remove("activeModelId");
        }
    }
    candidate.layouts.insert("workspace".to_owned(), workspace);
    if let (Some(model_id), Some(presentation)) = (persisted_model_id, model_presentation) {
        validate_model_presentation(&presentation)?;
        let mut explorer = normalized_workspace_explorer(&candidate);
        explorer
            .model_presentations
            .insert(model_id.to_string(), presentation);
        write_workspace_explorer(&mut candidate, explorer)?;
    }
    Ok(candidate)
}

#[tauri::command]
fn mutate_project_explorer(
    request: ProjectExplorerMutationRequest,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
) -> Result<ProjectSnapshot, String> {
    let mut project = project_state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    let jobs = job_state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    let path = request.path.clone();
    let candidate = guarded_project_after_explorer_mutation(&project, &jobs, request)?;
    drop(jobs);
    *project = candidate;
    snapshot(&project, path, None)
}

fn guarded_project_after_explorer_mutation(
    project: &Project,
    jobs: &HashMap<Uuid, DesktopJob>,
    request: ProjectExplorerMutationRequest,
) -> Result<Project, String> {
    if has_active_jobs(jobs) {
        return Err("project files cannot be changed while a calculation is active".to_owned());
    }
    project_after_explorer_mutation(project, request)
}

fn project_after_explorer_mutation(
    project: &Project,
    request: ProjectExplorerMutationRequest,
) -> Result<Project, String> {
    require_writable_project(project, "edit the project explorer")?;
    let mut candidate = project.clone();
    let mut explorer = normalized_workspace_explorer(&candidate);
    persist_current_explorer_model(
        &mut candidate,
        &mut explorer,
        request.current_model,
        request.current_presentation,
    )?;

    match request.mutation {
        ProjectExplorerMutation::CreateModel { name } => {
            let name = normalize_explorer_name("model", &name)?;
            ensure_unique_model_name(&candidate.models, None, &name)?;
            let model = ModelSpec {
                id: Uuid::new_v4(),
                name,
                constructs: vec![],
                paths: vec![],
                controls: vec![],
                higher_order_constructs: vec![],
                interactions: vec![],
            };
            let model_id = model.id;
            candidate.models.push(model);
            set_workspace_active_model(&mut candidate, Some(model_id))?;
        }
        ProjectExplorerMutation::ActivateModel { model_id } => {
            require_editable_model(&candidate, model_id)?;
            set_workspace_active_model(&mut candidate, Some(model_id))?;
        }
        ProjectExplorerMutation::RenameModel { model_id, name } => {
            require_editable_model(&candidate, model_id)?;
            let name = normalize_explorer_name("model", &name)?;
            ensure_unique_model_name(&candidate.models, Some(model_id), &name)?;
            candidate
                .models
                .iter_mut()
                .find(|model| model.id == model_id)
                .expect("editable model existence was checked")
                .name = name;
        }
        ProjectExplorerMutation::DeleteModel { model_id } => {
            require_editable_model(&candidate, model_id)?;
            let deleting_active_model = active_editable_model_id(&candidate) == Some(model_id);
            candidate.models.retain(|model| model.id != model_id);
            explorer.model_presentations.remove(&model_id.to_string());
            if deleting_active_model {
                let fallback = candidate
                    .models
                    .iter()
                    .min_by(|left, right| {
                        explorer_name_key(&left.name)
                            .cmp(&explorer_name_key(&right.name))
                            .then_with(|| left.id.cmp(&right.id))
                    })
                    .map(|model| model.id);
                set_workspace_active_model(&mut candidate, fallback)?;
            }
        }
        ProjectExplorerMutation::SaveReport { result_id, name } => {
            require_reportable_result(&candidate, result_id)?;
            if explorer
                .saved_reports
                .iter()
                .any(|report| report.result_id == result_id)
            {
                return Err(format!("result {result_id} already has a saved report"));
            }
            let name = normalize_explorer_name("report", &name)?;
            ensure_unique_report_name(&explorer.saved_reports, None, &name)?;
            explorer.saved_reports.push(SavedReportNode {
                result_id,
                name,
                saved_at: Utc::now().to_rfc3339(),
            });
        }
        ProjectExplorerMutation::RenameReport { result_id, name } => {
            if !explorer
                .saved_reports
                .iter()
                .any(|report| report.result_id == result_id)
            {
                return Err(format!("unknown saved report for result {result_id}"));
            }
            let name = normalize_explorer_name("report", &name)?;
            ensure_unique_report_name(&explorer.saved_reports, Some(result_id), &name)?;
            explorer
                .saved_reports
                .iter_mut()
                .find(|report| report.result_id == result_id)
                .expect("saved report existence was checked")
                .name = name;
        }
        ProjectExplorerMutation::RemoveReport { result_id } => {
            let original_len = explorer.saved_reports.len();
            explorer
                .saved_reports
                .retain(|report| report.result_id != result_id);
            if explorer.saved_reports.len() == original_len {
                return Err(format!("unknown saved report for result {result_id}"));
            }
        }
    }

    write_workspace_explorer(&mut candidate, explorer)?;
    Ok(candidate)
}

fn persist_current_explorer_model(
    project: &mut Project,
    explorer: &mut WorkspaceExplorerLayout,
    current_model: Option<ModelSpec>,
    current_presentation: Option<Value>,
) -> Result<(), String> {
    if current_model.is_none() && current_presentation.is_some() {
        return Err("a current model presentation requires its canonical model".to_owned());
    }
    let Some(current_model) = current_model else {
        return Ok(());
    };
    let active_model_id = active_editable_model_id(project)
        .ok_or_else(|| "the project has no active editable model to persist".to_owned())?;
    if current_model.id != active_model_id {
        return Err(format!(
            "current model {} does not match active model {active_model_id}",
            current_model.id
        ));
    }
    let existing = project
        .models
        .iter_mut()
        .find(|model| model.id == current_model.id)
        .ok_or_else(|| format!("unknown editable model {}", current_model.id))?;
    if existing.name != current_model.name {
        return Err("rename the active model through the Explorer rename command".to_owned());
    }
    *existing = current_model;
    if let Some(presentation) = current_presentation {
        validate_model_presentation(&presentation)?;
        explorer
            .model_presentations
            .insert(active_model_id.to_string(), presentation);
    }
    Ok(())
}

fn has_active_jobs(jobs: &HashMap<Uuid, DesktopJob>) -> bool {
    jobs.values().any(|job| {
        matches!(
            job.snapshot.state,
            JobState::Queued | JobState::Running | JobState::Cancelling | JobState::Committing
        )
    })
}

fn active_editable_model_id(project: &Project) -> Option<Uuid> {
    resolve_active_model_id(project)
        .and_then(|model_id| Uuid::parse_str(&model_id).ok())
        .filter(|model_id| project.models.iter().any(|model| model.id == *model_id))
}

fn require_editable_model(project: &Project, model_id: Uuid) -> Result<(), String> {
    project
        .models
        .iter()
        .any(|model| model.id == model_id)
        .then_some(())
        .ok_or_else(|| format!("unknown editable model {model_id}"))
}

fn set_workspace_active_model(project: &mut Project, model_id: Option<Uuid>) -> Result<(), String> {
    let workspace = project
        .layouts
        .entry("workspace".to_owned())
        .or_insert_with(|| serde_json::json!({}));
    let object = workspace
        .as_object_mut()
        .ok_or_else(|| "project workspace layout is invalid".to_owned())?;
    if let Some(model_id) = model_id {
        object.insert(
            "activeModelId".to_owned(),
            Value::String(model_id.to_string()),
        );
    } else {
        object.remove("activeModelId");
    }
    Ok(())
}

fn require_reportable_result(project: &Project, result_id: Uuid) -> Result<(), String> {
    let result = project
        .results
        .iter()
        .find(|result| result.id == result_id)
        .ok_or_else(|| format!("unknown result {result_id}"))?;
    if result.status != RunStatus::Completed {
        return Err(format!("result {result_id} is not completed"));
    }
    if !project
        .recipes
        .iter()
        .any(|recipe| recipe.id == result.provenance.recipe_id)
    {
        return Err(format!(
            "result {result_id} has no matching canonical recipe"
        ));
    }
    Ok(())
}

fn ensure_unique_model_name(
    models: &[ModelSpec],
    excluded_id: Option<Uuid>,
    name: &str,
) -> Result<(), String> {
    let key = explorer_name_key(name);
    if models
        .iter()
        .any(|model| Some(model.id) != excluded_id && explorer_name_key(&model.name) == key)
    {
        return Err(format!("a model named '{name}' already exists"));
    }
    Ok(())
}

fn ensure_unique_report_name(
    reports: &[SavedReportNode],
    excluded_result_id: Option<Uuid>,
    name: &str,
) -> Result<(), String> {
    let key = explorer_name_key(name);
    if reports.iter().any(|report| {
        Some(report.result_id) != excluded_result_id && explorer_name_key(&report.name) == key
    }) {
        return Err(format!("a saved report named '{name}' already exists"));
    }
    Ok(())
}

fn normalize_explorer_name(kind: &str, value: &str) -> Result<String, String> {
    let name = value.trim();
    let length = name.chars().count();
    if length == 0 {
        return Err(format!("{kind} name cannot be empty"));
    }
    if length > 120 {
        return Err(format!("{kind} name cannot exceed 120 characters"));
    }
    if matches!(name, "." | "..")
        || name.ends_with('.')
        || name.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
    {
        return Err(format!(
            "{kind} name contains characters that Windows cannot use"
        ));
    }
    Ok(name.to_owned())
}

fn explorer_name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn validate_model_presentation(presentation: &Value) -> Result<(), String> {
    if !presentation.is_object() {
        return Err("model presentation must be a JSON object".to_owned());
    }
    Ok(())
}

fn normalized_workspace_explorer(project: &Project) -> WorkspaceExplorerLayout {
    let stored = project.layouts.get(WORKSPACE_EXPLORER_LAYOUT_KEY);
    let has_stored_layout = stored.is_some();
    let mut explorer = stored
        .and_then(parse_workspace_explorer_layout)
        .unwrap_or_default();
    explorer.schema_version = WORKSPACE_EXPLORER_SCHEMA_VERSION;

    let editable_model_ids = project
        .models
        .iter()
        .map(|model| model.id.to_string())
        .collect::<BTreeSet<_>>();
    explorer.model_presentations.retain(|model_id, value| {
        editable_model_ids.contains(model_id) && validate_model_presentation(value).is_ok()
    });

    if !has_stored_layout {
        explorer.saved_reports = derive_legacy_saved_reports(project);
    }
    explorer.saved_reports = normalized_saved_reports(project, explorer.saved_reports);
    explorer
}

fn parse_workspace_explorer_layout(value: &Value) -> Option<WorkspaceExplorerLayout> {
    let object = value.as_object()?;
    let schema_version = u32::try_from(
        object
            .get("schemaVersion")
            .or_else(|| object.get("schema_version"))
            .and_then(Value::as_u64)?,
    )
    .ok()?;
    if schema_version != WORKSPACE_EXPLORER_SCHEMA_VERSION {
        return None;
    }
    let model_presentations = object
        .get("modelPresentations")
        .or_else(|| object.get("model_presentations"))
        .and_then(Value::as_object)
        .map(|presentations| {
            presentations
                .iter()
                .map(|(model_id, presentation)| (model_id.clone(), presentation.clone()))
                .collect()
        })
        .unwrap_or_default();
    let saved_reports = object
        .get("savedReports")
        .or_else(|| object.get("saved_reports"))
        .and_then(Value::as_array)
        .map(|reports| {
            reports
                .iter()
                .filter_map(|report| serde_json::from_value::<SavedReportNode>(report.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    Some(WorkspaceExplorerLayout {
        schema_version,
        model_presentations,
        saved_reports,
    })
}

fn normalized_saved_reports(
    project: &Project,
    reports: Vec<SavedReportNode>,
) -> Vec<SavedReportNode> {
    let mut seen_results = BTreeSet::new();
    let mut used_names = BTreeSet::new();
    let mut normalized = Vec::new();
    for mut report in reports {
        let Some(result) = project.results.iter().find(|result| {
            result.id == report.result_id
                && result.status == RunStatus::Completed
                && project
                    .recipes
                    .iter()
                    .any(|recipe| recipe.id == result.provenance.recipe_id)
        }) else {
            continue;
        };
        if !seen_results.insert(report.result_id) {
            continue;
        }
        let base_name = normalize_explorer_name("report", &report.name)
            .unwrap_or_else(|_| default_saved_report_name(project, result));
        report.name = unique_explorer_name(&base_name, &mut used_names);
        if chrono::DateTime::parse_from_rfc3339(report.saved_at.trim()).is_err() {
            report.saved_at = result.provenance.completed_at.to_rfc3339();
        }
        normalized.push(report);
    }
    normalized.sort_by(|left, right| {
        explorer_name_key(&left.name)
            .cmp(&explorer_name_key(&right.name))
            .then_with(|| left.result_id.cmp(&right.result_id))
    });
    normalized
}

fn derive_legacy_saved_reports(project: &Project) -> Vec<SavedReportNode> {
    project
        .results
        .iter()
        .filter(|result| {
            result.status == RunStatus::Completed
                && project
                    .recipes
                    .iter()
                    .any(|recipe| recipe.id == result.provenance.recipe_id)
        })
        .map(|result| SavedReportNode {
            result_id: result.id,
            name: default_saved_report_name(project, result),
            saved_at: result.provenance.completed_at.to_rfc3339(),
        })
        .collect()
}

fn default_saved_report_name(project: &Project, result: &AnalysisResult) -> String {
    let result_id = result.id.to_string();
    let workspace_name = project
        .layouts
        .get("workspace")
        .and_then(|workspace| workspace.get("runs"))
        .and_then(Value::as_array)
        .and_then(|runs| {
            runs.iter().find_map(|run| {
                (run.get("id").and_then(Value::as_str) == Some(result_id.as_str()))
                    .then(|| run.get("name").and_then(Value::as_str))
                    .flatten()
            })
        })
        .and_then(|name| normalize_explorer_name("report", name).ok());
    workspace_name.unwrap_or_else(|| {
        let method = result.provenance.method.as_str().replace('_', " ");
        format!("{method} report")
    })
}

fn unique_explorer_name(base_name: &str, used_names: &mut BTreeSet<String>) -> String {
    let base_name =
        normalize_explorer_name("report", base_name).unwrap_or_else(|_| "Saved report".to_owned());
    if used_names.insert(explorer_name_key(&base_name)) {
        return base_name;
    }
    for suffix in 2.. {
        let suffix = format!(" ({suffix})");
        let maximum_base = 120usize.saturating_sub(suffix.chars().count());
        let truncated = base_name.chars().take(maximum_base).collect::<String>();
        let candidate = format!("{truncated}{suffix}");
        if used_names.insert(explorer_name_key(&candidate)) {
            return candidate;
        }
    }
    unreachable!("an unused report name suffix is always available")
}

fn write_workspace_explorer(
    project: &mut Project,
    mut explorer: WorkspaceExplorerLayout,
) -> Result<(), String> {
    explorer.schema_version = WORKSPACE_EXPLORER_SCHEMA_VERSION;
    project.layouts.insert(
        WORKSPACE_EXPLORER_LAYOUT_KEY.to_owned(),
        serde_json::to_value(explorer).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn validate_executable_recipe(recipe: &AnalysisRecipe) -> Result<(), String> {
    if recipe.schema_version != ANALYSIS_RECIPE_SCHEMA_VERSION {
        return Err(format!(
            "schema.current_required: new desktop analyses require recipe schema v{ANALYSIS_RECIPE_SCHEMA_VERSION}; found v{}",
            recipe.schema_version
        ));
    }
    let issues = validate_recipe(recipe);
    if let Some(issue) = issues
        .iter()
        .find(|issue| issue.severity == Severity::Error)
    {
        return Err(format!("{}: {}", issue.code, issue.message));
    }
    recipe
        .effective_metadata()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn analysis_worker_demand(recipe: &AnalysisRecipe) -> usize {
    let typed_parallel_plan = recipe.settings.method == AnalysisMethod::PlsSampleSizePower
        || matches!(
            recipe.method_config.as_ref(),
            Some(qpls_core::MethodConfig::Cbsem {
                bootstrap_v2: Some(_),
                ..
            })
        );
    if recipe.settings.bootstrap_samples > 0
        || recipe.settings.permutation_samples > 0
        || typed_parallel_plan
    {
        recipe.settings.workers
    } else {
        1
    }
}

#[tauri::command]
fn start_analysis_job(
    recipe: AnalysisRecipe,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
    recipe_v4_job_state: State<'_, DesktopRecipeV4Jobs>,
) -> Result<JobSnapshot, String> {
    validate_executable_recipe(&recipe)?;
    let (dataset, project_id) = {
        let project = project_state
            .0
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        require_writable_project(&project, "run or store analyses")?;
        let dataset = project
            .datasets
            .iter()
            .find(|dataset| dataset.fingerprint.0 == recipe.dataset_fingerprint)
            .cloned()
            .ok_or_else(|| "recipe dataset fingerprint is not present in the project".to_owned())?;
        (dataset, project.manifest.project_id)
    };
    if recipe.settings.method == AnalysisMethod::Mga {
        let execution_recipe = recipe
            .with_effective_metadata()
            .map_err(|error| error.to_string())?;
        validate_mga_dataset_contract(&dataset, &execution_recipe)?;
    }
    {
        let project = project_state
            .0
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        if project.manifest.project_id != project_id {
            return Err("the active project changed during analysis preflight".into());
        }
        require_writable_project(&project, "finish analysis preflight")?;
        if !project
            .datasets
            .iter()
            .any(|candidate| candidate.fingerprint.0 == recipe.dataset_fingerprint)
        {
            return Err("the analysis dataset changed during analysis preflight".into());
        }
    }
    let snapshot = JobSnapshot::queued(2);
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut jobs_guard = job_state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    prune_terminal_jobs(&mut jobs_guard, 255);
    let (recipe_v4_active_count, recipe_v4_worker_demand) = recipe_v4_job_state.active_summary()?;
    let active_count = jobs_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                JobState::Queued | JobState::Running | JobState::Cancelling | JobState::Committing
            )
        })
        .count()
        + recipe_v4_active_count;
    if active_count >= 4 {
        return Err("four analyses are already active; wait for one to finish".into());
    }
    let worker_demand = analysis_worker_demand(&recipe);
    let cpu_budget = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let allocated_workers = jobs_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                JobState::Queued | JobState::Running | JobState::Cancelling | JobState::Committing
            )
        })
        .map(|job| job.worker_demand)
        .sum::<usize>()
        + recipe_v4_worker_demand;
    if worker_demand > cpu_budget || allocated_workers + worker_demand > cpu_budget {
        return Err(format!(
            "analysis requests {worker_demand} workers but only {} of {cpu_budget} are available",
            cpu_budget.saturating_sub(allocated_workers)
        ));
    }
    jobs_guard.insert(
        snapshot.id,
        DesktopJob {
            snapshot: snapshot.clone(),
            cancellation: cancellation.clone(),
            result: None,
            worker_demand,
        },
    );
    drop(jobs_guard);

    let jobs = job_state.0.clone();
    let project = project_state.0.clone();
    let job_id = snapshot.id;
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            set_job_progress(&jobs, job_id, JobState::Running, "estimation", 0, 1, None);
            if cancellation.load(Ordering::Relaxed) {
                set_job_progress(&jobs, job_id, JobState::Cancelled, "cancelled", 0, 1, None);
                return;
            }
            let result = match run_pls_analysis(
                &dataset,
                &recipe,
                || cancellation.load(Ordering::Relaxed),
                |progress| {
                    set_job_progress(
                        &jobs,
                        job_id,
                        JobState::Running,
                        progress.phase.as_str(),
                        progress.completed_units,
                        progress.total_units,
                        None,
                    );
                },
            ) {
                Ok(result) => result,
                Err(RunnerError::Cancelled) => {
                    set_job_progress(&jobs, job_id, JobState::Cancelled, "cancelled", 0, 1, None);
                    return;
                }
                Err(error) => {
                    set_job_progress(
                        &jobs,
                        job_id,
                        JobState::Failed,
                        "analysis",
                        0,
                        1,
                        Some(error.to_string()),
                    );
                    return;
                }
            };
            set_job_progress(
                &jobs,
                job_id,
                JobState::Committing,
                "committing",
                0,
                1,
                None,
            );
            let stored = commit_job_result(&project, &jobs, project_id, job_id, recipe, result);
            if let Err(error) = stored {
                set_job_progress(
                    &jobs,
                    job_id,
                    JobState::Failed,
                    "persisting",
                    0,
                    1,
                    Some(error),
                );
            }
        }));
        if outcome.is_err() {
            set_job_progress(
                &jobs,
                job_id,
                JobState::Failed,
                "internal_error",
                0,
                1,
                Some("analysis worker terminated unexpectedly".into()),
            );
        }
    });
    Ok(snapshot)
}

/// Deprecated compatibility alias for `start_analysis_job`; retain for one major release.
#[tauri::command]
fn start_pls_job(
    recipe: AnalysisRecipe,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
    recipe_v4_job_state: State<'_, DesktopRecipeV4Jobs>,
) -> Result<JobSnapshot, String> {
    start_analysis_job(recipe, project_state, job_state, recipe_v4_job_state)
}

#[tauri::command]
fn analysis_job_status(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
    state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?
        .get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| format!("unknown job {job_id}"))
}

/// Deprecated compatibility alias for `analysis_job_status`; retain for one major release.
#[tauri::command]
fn pls_job_status(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
    analysis_job_status(job_id, state)
}

#[tauri::command]
fn cancel_analysis_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
    let mut jobs = state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    let job = jobs
        .get_mut(&job_id)
        .ok_or_else(|| format!("unknown job {job_id}"))?;
    if matches!(
        job.snapshot.state,
        JobState::Queued | JobState::Running | JobState::Committing
    ) {
        job.cancellation.store(true, Ordering::Relaxed);
        job.snapshot.state = JobState::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

/// Deprecated compatibility alias for `cancel_analysis_job`; retain for one major release.
#[tauri::command]
fn cancel_pls_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
    cancel_analysis_job(job_id, state)
}

#[tauri::command]
fn dismiss_analysis_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<(), String> {
    let mut jobs = state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    let terminal = jobs
        .get(&job_id)
        .map(|job| {
            matches!(
                job.snapshot.state,
                JobState::Completed | JobState::Failed | JobState::Cancelled
            )
        })
        .ok_or_else(|| format!("unknown job {job_id}"))?;
    if !terminal {
        return Err("an active job cannot be dismissed".into());
    }
    jobs.remove(&job_id);
    Ok(())
}

/// Deprecated compatibility alias for `dismiss_analysis_job`; retain for one major release.
#[tauri::command]
fn dismiss_pls_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<(), String> {
    dismiss_analysis_job(job_id, state)
}

#[tauri::command]
fn analysis_job_result(
    job_id: Uuid,
    state: State<'_, DesktopJobs>,
) -> Result<Option<AnalysisResult>, String> {
    take_job_result(&state.0, job_id)
}

/// Deprecated compatibility alias for `analysis_job_result`; retain for one major release.
#[tauri::command]
fn pls_job_result(
    job_id: Uuid,
    state: State<'_, DesktopJobs>,
) -> Result<Option<AnalysisResult>, String> {
    analysis_job_result(job_id, state)
}

fn commit_job_result(
    project: &Mutex<Project>,
    jobs: &Mutex<HashMap<Uuid, DesktopJob>>,
    expected_project_id: Uuid,
    job_id: Uuid,
    recipe: AnalysisRecipe,
    result: AnalysisResult,
) -> Result<(), String> {
    let mut project = project
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    if project.manifest.project_id != expected_project_id {
        return Err("the active project changed while estimation was running".into());
    }
    require_writable_project(&project, "commit the completed analysis")?;
    let mut jobs = jobs
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    let job = jobs
        .get_mut(&job_id)
        .ok_or_else(|| format!("unknown job {job_id}"))?;
    if job.cancellation.load(Ordering::Relaxed) {
        job.snapshot.state = JobState::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        return Ok(());
    }
    if !project
        .datasets
        .iter()
        .any(|candidate| candidate.fingerprint.0 == recipe.dataset_fingerprint)
    {
        return Err("the analysis dataset was removed while estimation was running".into());
    }
    project
        .append_validated_result(recipe, result.clone())
        .map_err(|error| format!("completed result failed project validation: {error}"))?;
    job.result = Some(result);
    job.snapshot.state = JobState::Completed;
    job.snapshot.phase = "completed".into();
    job.snapshot.completed_units = job.snapshot.total_units;
    job.snapshot.message = None;
    Ok(())
}

fn take_job_result(
    jobs: &Mutex<HashMap<Uuid, DesktopJob>>,
    job_id: Uuid,
) -> Result<Option<AnalysisResult>, String> {
    let mut jobs = jobs
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    let job = jobs
        .remove(&job_id)
        .ok_or_else(|| format!("unknown job {job_id}"))?;
    if job.snapshot.state != JobState::Completed {
        jobs.insert(job_id, job);
        return Err("job result is available only after successful completion".into());
    }
    Ok(job.result)
}

fn set_job_progress(
    jobs: &Mutex<HashMap<Uuid, DesktopJob>>,
    job_id: Uuid,
    state: JobState,
    phase: &str,
    completed_units: u64,
    total_units: u64,
    message: Option<String>,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        let cancellation_is_pending = job.snapshot.state == JobState::Cancelling
            && matches!(
                state,
                JobState::Queued | JobState::Running | JobState::Committing
            );
        if cancellation_is_pending {
            job.snapshot.phase = "cancelling".into();
            job.snapshot.message = Some("Cancellation requested".into());
        } else {
            job.snapshot.state = state;
            job.snapshot.phase = phase.into();
            job.snapshot.message = message;
        }
        job.snapshot.completed_units = completed_units;
        job.snapshot.total_units = total_units;
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, DesktopJob>, maximum_retained: usize) {
    if jobs.len() <= maximum_retained {
        return;
    }
    let removable = jobs
        .iter()
        .filter_map(|(id, job)| {
            matches!(
                job.snapshot.state,
                JobState::Completed | JobState::Failed | JobState::Cancelled
            )
            .then_some(*id)
        })
        .take(jobs.len() - maximum_retained)
        .collect::<Vec<_>>();
    for id in removable {
        jobs.remove(&id);
    }
}

fn snapshot(
    project: &Project,
    path: Option<String>,
    recovery_source: Option<RecoverySource>,
) -> Result<ProjectSnapshot, String> {
    let explorer = normalized_workspace_explorer(project);
    let dataset_versions = read_dataset_lineage(project)?
        .map(|lineage| lineage.records)
        .unwrap_or_default();
    Ok(ProjectSnapshot {
        project_id: project.manifest.project_id,
        name: project.manifest.name.clone(),
        path,
        read_only: project.read_only,
        source_archive_version: project.source_archive_version,
        migration_pending: project.migration_pending,
        compatibility_notices: project
            .compatibility_notices
            .iter()
            .map(|notice| ProjectCompatibilityNoticeSnapshot {
                result_id: notice.result_id,
                code: notice.diagnostic.code.clone(),
                message: notice.diagnostic.message.clone(),
            })
            .collect(),
        future_unsupported: ProjectFutureUnsupportedSnapshot {
            models: project.future_unsupported.models,
            recipes: project.future_unsupported.recipes,
            results: project.future_unsupported.results,
        },
        save_warning: None,
        recovered: recovery_source.is_some(),
        recovery_source: recovery_source.map(|source| {
            match source {
                RecoverySource::Autosave => "autosave",
                RecoverySource::Backup => "backup",
            }
            .to_owned()
        }),
        datasets: project.datasets.iter().map(dataset_snapshot).collect(),
        dataset_versions,
        workspace: project.layouts.get("workspace").cloned(),
        models: project.models.clone(),
        recipes: project.recipes.clone(),
        results: project.results.clone(),
        active_model_id: resolve_active_model_id(project),
        model_presentations: explorer.model_presentations,
        saved_reports: explorer.saved_reports,
    })
}

/// Resolves an active canonical model only from identifiers that can be tied
/// back to archive content. The workspace layout is intentionally treated as
/// a navigation hint, never as the authoritative model or result store.
fn resolve_active_model_id(project: &Project) -> Option<String> {
    let workspace = project.layouts.get("workspace");
    let canonical_model_ids = project
        .models
        .iter()
        .map(|model| model.id)
        .chain(
            project
                .recipes
                .iter()
                .filter(|recipe| recipe_uses_editable_model(recipe))
                .map(|recipe| recipe.model.id),
        )
        .collect::<std::collections::BTreeSet<_>>();

    let workspace_string = |keys: &[&str]| {
        workspace.and_then(|workspace| {
            keys.iter()
                .find_map(|key| workspace.get(*key).and_then(Value::as_str))
        })
    };

    if let Some(model_id) = workspace_string(&["activeModelId", "active_model_id"])
        .and_then(|value| Uuid::parse_str(value).ok())
        .filter(|id| canonical_model_ids.contains(id))
    {
        return Some(model_id.to_string());
    }

    if let Some(recipe_id) = workspace_string(&["activeRecipeId", "active_recipe_id"])
        .and_then(|value| Uuid::parse_str(value).ok())
        && let Some(recipe) = project
            .recipes
            .iter()
            .find(|recipe| recipe.id == recipe_id && recipe_uses_editable_model(recipe))
    {
        return Some(recipe.model.id.to_string());
    }

    let selected_run_id = workspace_string(&[
        "selectedRunId",
        "selected_run_id",
        "activeRunId",
        "active_run_id",
    ])
    .or_else(|| {
        workspace
            .and_then(|workspace| workspace.get("diagramOverlaySettings"))
            .and_then(|settings| settings.get("selectedRunId"))
            .and_then(Value::as_str)
    });
    if let Some(result_id) = selected_run_id.and_then(|value| Uuid::parse_str(value).ok())
        && let Some(result) = project.results.iter().find(|result| result.id == result_id)
        && let Some(recipe) = project.recipes.iter().find(|recipe| {
            recipe.id == result.provenance.recipe_id && recipe_uses_editable_model(recipe)
        })
    {
        return Some(recipe.model.id.to_string());
    }

    (canonical_model_ids.len() == 1)
        .then(|| canonical_model_ids.iter().next().map(Uuid::to_string))
        .flatten()
}

/// Standalone raw-data methods carry an empty `recipe.model` for schema
/// compatibility. That embedded placeholder must never be surfaced as an
/// editable canonical model after a project is reopened.
fn recipe_uses_editable_model(recipe: &AnalysisRecipe) -> bool {
    !matches!(
        recipe.settings.method,
        AnalysisMethod::Pca | AnalysisMethod::Regression | AnalysisMethod::Nca
    )
}

fn dataset_rows_page(
    project: &Project,
    dataset_id: &str,
    offset: usize,
    limit: usize,
) -> Result<DatasetRowsPage, String> {
    if limit == 0 {
        return Err("dataset row page limit must be at least 1".to_owned());
    }
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    let bounded_limit = limit.min(MAX_DATASET_ROW_PAGE_SIZE);
    let offset = offset.min(dataset.schema.case_count);
    Ok(DatasetRowsPage {
        dataset_id: dataset.id.to_string(),
        offset,
        limit: bounded_limit,
        row_count: dataset.schema.case_count,
        rows: preview_page(dataset, offset, bounded_limit),
    })
}

fn dataset_snapshot(dataset: &Dataset) -> DatasetSnapshot {
    let missing_by_column = dataset
        .schema
        .columns
        .iter()
        .zip(dataset.batch.columns())
        .map(|(metadata, column)| (metadata.name.clone(), column.null_count()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let missing = missing_by_column.values().sum();
    DatasetSnapshot {
        id: dataset.id.to_string(),
        name: dataset.name.clone(),
        columns: dataset
            .schema
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect(),
        rows: preview(dataset, 100),
        row_count: dataset.schema.case_count,
        missing,
        missing_by_column,
        fingerprint: dataset.fingerprint.0.clone(),
        kind: dataset.schema.kind,
        sample_size: dataset.schema.sample_size,
        column_metadata: dataset.schema.columns.clone(),
    }
}

fn build_demo_project() -> Result<Project, String> {
    let dataset = import_delimited_bytes(
        include_bytes!("../../validation/fixtures/corporate_reputation.csv"),
        "corporate_reputation.csv",
        b',',
        &ImportOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let model = demo_model();
    let mut settings = AnalysisSettings::default();
    settings.bootstrap_samples = 24;
    settings.permutation_samples = 0;
    settings.seed = 20_260_718;
    settings.workers = 1;
    let recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: "00000000-0000-0000-0000-00000000d004"
            .parse()
            .expect("fixed demo recipe UUID is valid"),
        created_at: chrono::DateTime::parse_from_rfc3339("2026-07-18T00:00:00Z")
            .expect("fixed demo date is valid")
            .with_timezone(&Utc),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: model.clone(),
        settings,
        method_config: Some(MethodConfig::PlsBootstrap),
        metadata: std::collections::BTreeMap::from([
            ("demo".into(), "quickpls_v04_demo".into()),
            (
                "fixture".into(),
                "validation/fixtures/corporate_reputation.csv".into(),
            ),
        ]),
    };
    let result = run_demo_recipe(&dataset, &recipe)?;
    let workspace = demo_workspace(&dataset, &result);
    let dataset_id = dataset.id.to_string();
    let mut project = Project::new("Corporate Reputation Sample");
    project.datasets.push(dataset);
    project.models.push(model);
    project.recipes.push(recipe);
    project.results.push(result);
    project.layouts.insert("workspace".into(), workspace);
    write_project_data_lineage_v1(
        &mut project.layouts,
        &DatasetLineageLayout {
            schema_version: 1,
            records: vec![DatasetVersionRecord {
                dataset_id,
                parent_dataset_id: None,
                operation: DatasetVersionOperation::Import,
                created_at: None,
                summary: "Bundled corporate reputation sample".into(),
                source_column: None,
                target_column: None,
                transformation: None,
            }],
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(project)
}

fn demo_model() -> ModelSpec {
    ModelSpec {
        id: "00000000-0000-0000-0000-00000000d003"
            .parse()
            .expect("fixed demo model UUID is valid"),
        name: "Corporate reputation validation demo".into(),
        constructs: vec![
            Construct {
                id: "comp".into(),
                name: "Competence".into(),
                short_name: "COMP".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["COMP1".into(), "COMP2".into(), "COMP3".into()],
            },
            Construct {
                id: "like".into(),
                name: "Likeability".into(),
                short_name: "LIKE".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["LIKE1".into(), "LIKE2".into()],
            },
            Construct {
                id: "satisfaction".into(),
                name: "Customer satisfaction".into(),
                short_name: "CUSA".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["CUSA1".into(), "CUSA2".into()],
            },
            Construct {
                id: "loyalty".into(),
                name: "Customer loyalty".into(),
                short_name: "CUSL".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["CUSL1".into(), "CUSL2".into()],
            },
        ],
        paths: vec![
            StructuralPath {
                source: "comp".into(),
                target: "satisfaction".into(),
            },
            StructuralPath {
                source: "like".into(),
                target: "satisfaction".into(),
            },
            StructuralPath {
                source: "satisfaction".into(),
                target: "loyalty".into(),
            },
        ],
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    }
}

fn run_demo_recipe(dataset: &Dataset, recipe: &AnalysisRecipe) -> Result<AnalysisResult, String> {
    run_pls_analysis(dataset, recipe, || false, |_| {}).map_err(|error| error.to_string())
}

fn demo_workspace(dataset: &Dataset, result: &AnalysisResult) -> Value {
    let (estimation, assessment, bootstrap, permutation) = match &result.payload {
        qpls_core::AnalysisPayload::PlsPmV2 {
            estimation,
            assessment,
            bootstrap,
        } => (estimation, assessment, Some(bootstrap.clone()), None),
        qpls_core::AnalysisPayload::PlsPmV3 {
            estimation,
            assessment,
            bootstrap,
            permutation,
        } => (
            estimation,
            assessment,
            bootstrap.clone(),
            permutation.clone(),
        ),
        _ => unreachable!("demo result is created as bootstrap-only pls_pm_v2"),
    };
    serde_json::json!({
        "activeDatasetId": dataset.id.to_string(),
        "analysisSettings": {
            "bootstrapSamples": 24,
            "studentizedInnerSamples": 0,
            "permutationSamples": 0,
            "seed": 20260718,
            "workers": 1,
            "confidenceLevel": 0.95
        },
        "nodes": [
            {"id": "comp", "type": "construct", "position": {"x": 90, "y": 115}, "data": {"label": "Competence", "shortName": "COMP", "mode": "reflective", "indicators": ["COMP1", "COMP2", "COMP3"]}},
            {"id": "like", "type": "construct", "position": {"x": 90, "y": 350}, "data": {"label": "Likeability", "shortName": "LIKE", "mode": "reflective", "indicators": ["LIKE1", "LIKE2"]}},
            {"id": "satisfaction", "type": "construct", "position": {"x": 465, "y": 115}, "data": {"label": "Customer satisfaction", "shortName": "CUSA", "mode": "reflective", "indicators": ["CUSA1", "CUSA2"]}},
            {"id": "loyalty", "type": "construct", "position": {"x": 835, "y": 235}, "data": {"label": "Customer loyalty", "shortName": "CUSL", "mode": "reflective", "indicators": ["CUSL1", "CUSL2"]}}
        ],
        "edges": [
            {"id": "path-comp-satisfaction", "source": "comp", "target": "satisfaction", "type": "smoothstep", "label": "Path", "markerEnd": {"type": "arrowclosed", "width": 16, "height": 16}},
            {"id": "path-like-satisfaction", "source": "like", "target": "satisfaction", "type": "smoothstep", "label": "Path", "markerEnd": {"type": "arrowclosed", "width": 16, "height": 16}},
            {"id": "path-satisfaction-loyalty", "source": "satisfaction", "target": "loyalty", "type": "smoothstep", "label": "Path", "markerEnd": {"type": "arrowclosed", "width": 16, "height": 16}}
        ],
        "runs": [{
            "id": result.id.to_string(),
            "name": SAMPLE_RUN_DISPLAY_NAME,
            "method": SAMPLE_RUN_METHOD_NAME,
            "createdAt": result.provenance.completed_at,
            "seed": result.provenance.seed,
            "status": "completed",
            "warnings": ["Validated for the documented QuickPLS v1.0.0 supported scope."],
            "fingerprint": result.provenance.dataset_fingerprint.chars().take(12).collect::<String>(),
            "result": estimation,
            "assessment": assessment,
            "bootstrap": bootstrap,
            "permutation": permutation
        }]
    })
}

#[cfg(test)]
mod webview2_offline_proxy_tests {
    use super::*;
    use std::io::Read;

    fn rejection_response(request: &[u8]) -> Vec<u8> {
        let listener = bind_webview2_offline_rejection_proxy(
            "127.0.0.1:0"
                .parse()
                .expect("valid ephemeral loopback address"),
        )
        .expect("bind test rejection proxy");
        let proxy_address = listener.local_addr().expect("read proxy address");
        let worker = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept proxy request");
            reject_webview2_proxy_stream(stream).expect("write rejection response");
        });

        let mut client =
            TcpStream::connect(proxy_address).expect("connect to test rejection proxy");
        client.write_all(request).expect("write proxy request");
        client
            .shutdown(Shutdown::Write)
            .expect("finish proxy request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .expect("read proxy rejection");
        worker.join().expect("join proxy worker");
        response
    }

    #[test]
    fn webview2_offline_proxy_rejects_connect_without_contacting_requested_upstream() {
        let upstream = TcpListener::bind("127.0.0.1:0").expect("bind upstream sentinel");
        upstream
            .set_nonblocking(true)
            .expect("make upstream sentinel nonblocking");
        let upstream_address = upstream.local_addr().expect("read upstream address");
        let request =
            format!("CONNECT {upstream_address} HTTP/1.1\r\nHost: {upstream_address}\r\n\r\n");

        let response = rejection_response(request.as_bytes());

        assert!(response.starts_with(b"HTTP/1.1 403 Forbidden\r\n"));
        assert!(response.ends_with(b"\r\n\r\n"));
        assert!(matches!(
            upstream.accept(),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock
        ));
    }

    #[test]
    fn webview2_offline_proxy_rejects_plain_http_without_using_the_destination() {
        let response = rejection_response(
            b"GET http://telemetry.example.test/ping HTTP/1.1\r\nHost: telemetry.example.test\r\n\r\n",
        );

        assert_eq!(response, WEBVIEW2_OFFLINE_PROXY_RESPONSE);
        assert!(
            response
                .windows(b"X-QuickPLS-Network-Policy: offline".len())
                .any(|window| window == b"X-QuickPLS-Network-Policy: offline")
        );
    }

    #[test]
    fn webview2_offline_proxy_bind_is_ipv4_loopback_only() {
        for forbidden in ["0.0.0.0:0", "[::1]:0", "[::]:0"] {
            let error = bind_webview2_offline_rejection_proxy(
                forbidden.parse().expect("valid forbidden bind address"),
            )
            .expect_err("non-127.0.0.1 bind must be rejected");
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        }

        let listener = bind_webview2_offline_rejection_proxy(
            "127.0.0.1:0".parse().expect("valid allowed bind address"),
        )
        .expect("127.0.0.1 bind must be allowed");
        assert_eq!(listener.local_addr().unwrap().ip(), Ipv4Addr::LOCALHOST);
    }

    #[test]
    fn production_webview2_offline_proxy_address_is_fixed_loopback() {
        let bind_address: SocketAddr = WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS
            .parse()
            .expect("production proxy address must parse");

        assert_eq!(bind_address.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(bind_address.port(), 17846);
    }
}

#[cfg(test)]
mod desktop_job_tests {
    use super::*;
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::{thread, time::Duration};

    fn fixture(
        cancelled: bool,
    ) -> (
        Project,
        HashMap<Uuid, DesktopJob>,
        Uuid,
        AnalysisRecipe,
        AnalysisResult,
    ) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = recipe.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let mut project = Project::new("Job fixture");
        project.datasets.push(dataset);
        let mut snapshot = JobSnapshot::queued(1);
        snapshot.state = JobState::Committing;
        let job_id = snapshot.id;
        let cancellation = Arc::new(AtomicBool::new(cancelled));
        let jobs = HashMap::from([(
            job_id,
            DesktopJob {
                snapshot,
                cancellation,
                result: None,
                worker_demand: 1,
            },
        )]);
        (project, jobs, job_id, recipe, result)
    }

    fn explorer_request(mutation: ProjectExplorerMutation) -> ProjectExplorerMutationRequest {
        ProjectExplorerMutationRequest {
            mutation,
            current_model: None,
            current_presentation: None,
            path: None,
        }
    }

    #[test]
    fn desktop_job_boundary_requires_v3_and_keeps_execution_projection_ephemeral() {
        let legacy: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        assert!(
            validate_executable_recipe(&legacy)
                .unwrap_err()
                .contains("require recipe schema v3")
        );

        let mut legacy_mga: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/results/mga_reference.recipe.json"
        ))
        .unwrap();
        legacy_mga
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        legacy_mga
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        legacy_mga
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        let typed = legacy_mga.migrated_v3().unwrap();
        let persisted_value = serde_json::to_value(&typed).unwrap();
        assert!(typed.executable_legacy_metadata_keys().is_empty());

        validate_executable_recipe(&typed).unwrap();
        let execution = typed.with_effective_metadata().unwrap();

        assert_eq!(serde_json::to_value(&typed).unwrap(), persisted_value);
        assert_eq!(execution.id, typed.id);
        assert_eq!(execution.method_config, typed.method_config);
        assert_eq!(
            execution
                .metadata
                .get("mga_group_column")
                .map(String::as_str),
            Some("group")
        );
        assert_eq!(
            execution
                .metadata
                .get("micom_configural_confirmed")
                .map(String::as_str),
            Some("true")
        );
        assert!(typed.metadata.get("mga_group_column").is_none());
        assert!(typed.metadata.get("micom_configural_confirmed").is_none());
    }

    #[test]
    fn worker_demand_charges_typed_power_and_cbsem_bootstrap_plans() {
        let (_, _, _, recipe, _) = fixture(false);
        let mut point = recipe.clone();
        point.settings.workers = 8;
        assert_eq!(analysis_worker_demand(&point), 1);

        let mut generic = recipe.clone();
        generic.settings.workers = 3;
        generic.settings.bootstrap_samples = 999;
        assert_eq!(analysis_worker_demand(&generic), 3);

        let mut power = recipe.clone();
        power.settings.method = AnalysisMethod::PlsSampleSizePower;
        power.settings.workers = 5;
        power.settings.bootstrap_samples = 0;
        power.settings.permutation_samples = 0;
        assert_eq!(analysis_worker_demand(&power), 5);

        let mut cbsem = recipe;
        cbsem.settings.method = AnalysisMethod::Cbsem;
        cbsem.settings.workers = 4;
        cbsem.settings.bootstrap_samples = 0;
        cbsem.settings.permutation_samples = 0;
        cbsem.method_config = Some(qpls_core::MethodConfig::Cbsem {
            model_type: qpls_core::CbsemModelType::Sem,
            estimator: qpls_core::CbsemEstimator::Ml,
            input: qpls_core::CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 1_000,
            bootstrap_v2: Some(qpls_core::CbsemBootstrapConfigV2 {
                algorithm: qpls_core::CbsemBootstrapAlgorithm::CaseResamplingFullMl,
                interval: qpls_core::CbsemBootstrapInterval::PercentileType7,
                test_tail: qpls_core::CbsemBootstrapTestTail::TwoSided,
            }),
            group_column: None,
            invariance_steps: Vec::new(),
        });
        assert_eq!(analysis_worker_demand(&cbsem), 4);
        if let Some(qpls_core::MethodConfig::Cbsem { bootstrap_v2, .. }) =
            cbsem.method_config.as_mut()
        {
            *bootstrap_v2 = None;
        }
        assert_eq!(analysis_worker_demand(&cbsem), 1);
    }

    #[test]
    fn cancellation_wins_before_commit_and_does_not_persist() {
        let (project, jobs, job_id, recipe, result) = fixture(true);
        let project_id = project.manifest.project_id;
        let project = Mutex::new(project);
        let jobs = Mutex::new(jobs);
        commit_job_result(&project, &jobs, project_id, job_id, recipe, result).unwrap();
        assert!(project.lock().unwrap().results.is_empty());
        assert_eq!(
            jobs.lock().unwrap()[&job_id].snapshot.state,
            JobState::Cancelled
        );
    }

    #[test]
    fn progress_cannot_regress_a_cancelling_job_back_to_active_state() {
        let mut snapshot = JobSnapshot::queued(10);
        snapshot.state = JobState::Cancelling;
        snapshot.phase = "cancelling".into();
        snapshot.message = Some("Cancellation requested".into());
        let job_id = snapshot.id;
        let jobs = Mutex::new(HashMap::from([(
            job_id,
            DesktopJob {
                snapshot,
                cancellation: Arc::new(AtomicBool::new(true)),
                result: None,
                worker_demand: 1,
            },
        )]));

        for state in [JobState::Running, JobState::Committing] {
            set_job_progress(&jobs, job_id, state, "estimation", 4, 10, None);
            let jobs = jobs.lock().unwrap();
            let snapshot = &jobs[&job_id].snapshot;
            assert_eq!(snapshot.state, JobState::Cancelling);
            assert_eq!(snapshot.phase, "cancelling");
            assert_eq!(snapshot.message.as_deref(), Some("Cancellation requested"));
            assert_eq!(snapshot.completed_units, 4);
            assert_eq!(snapshot.total_units, 10);
        }

        set_job_progress(&jobs, job_id, JobState::Cancelled, "cancelled", 4, 10, None);
        let jobs = jobs.lock().unwrap();
        let snapshot = &jobs[&job_id].snapshot;
        assert_eq!(snapshot.state, JobState::Cancelled);
        assert_eq!(snapshot.phase, "cancelled");
        assert_eq!(snapshot.message, None);
    }

    #[test]
    fn commit_checks_project_identity_and_read_only_state() {
        let (mut project, jobs, job_id, recipe, result) = fixture(false);
        let original_id = project.manifest.project_id;
        let jobs = Mutex::new(jobs);
        assert!(
            commit_job_result(
                &Mutex::new(project.clone()),
                &jobs,
                Uuid::new_v4(),
                job_id,
                recipe.clone(),
                result.clone(),
            )
            .unwrap_err()
            .contains("active project changed")
        );
        project.read_only = true;
        assert!(
            commit_job_result(
                &Mutex::new(project),
                &jobs,
                original_id,
                job_id,
                recipe,
                result,
            )
            .unwrap_err()
            .contains("read-only")
        );
    }

    #[test]
    fn commit_rejects_a_result_that_cannot_be_saved_or_reopened() {
        let (project, jobs, job_id, recipe, mut result) = fixture(false);
        let project_id = project.manifest.project_id;
        match &mut result.payload {
            qpls_core::AnalysisPayload::PlsPmV1 { estimation, .. }
            | qpls_core::AnalysisPayload::PlsPmV2 { estimation, .. }
            | qpls_core::AnalysisPayload::PlsPmV3 { estimation, .. } => {
                estimation["method_version"] = serde_json::json!("tampered_method_version");
            }
            qpls_core::AnalysisPayload::Legacy { .. } => {
                panic!("runner returned an unexpected legacy payload")
            }
            qpls_core::AnalysisPayload::PlsSampleSizePowerV1 { .. }
            | qpls_core::AnalysisPayload::PlsSampleSizePowerV2 { .. } => {
                panic!("fixture returned an unexpected prospective power payload")
            }
        }
        let project = Mutex::new(project);
        let jobs = Mutex::new(jobs);

        let error =
            commit_job_result(&project, &jobs, project_id, job_id, recipe, result).unwrap_err();

        assert!(error.contains("completed result failed project validation"));
        let project = project.lock().unwrap();
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());
        assert!(jobs.lock().unwrap()[&job_id].result.is_none());
    }

    #[test]
    fn completed_result_is_consumed_and_terminal_retention_is_bounded() {
        let (project, jobs, job_id, recipe, result) = fixture(false);
        let project_id = project.manifest.project_id;
        let project = Mutex::new(project);
        let jobs = Mutex::new(jobs);
        commit_job_result(&project, &jobs, project_id, job_id, recipe, result).unwrap();
        assert!(take_job_result(&jobs, job_id).unwrap().is_some());
        assert!(!jobs.lock().unwrap().contains_key(&job_id));

        let mut retained = HashMap::new();
        for _ in 0..260 {
            let mut snapshot = JobSnapshot::queued(1);
            snapshot.state = JobState::Failed;
            retained.insert(
                snapshot.id,
                DesktopJob {
                    snapshot,
                    cancellation: Arc::new(AtomicBool::new(false)),
                    result: None,
                    worker_demand: 1,
                },
            );
        }
        prune_terminal_jobs(&mut retained, 255);
        assert_eq!(retained.len(), 255);
    }

    #[test]
    fn waiting_for_project_does_not_hold_the_job_registry() {
        let (project, jobs, job_id, recipe, result) = fixture(false);
        let project_id = project.manifest.project_id;
        let project = Arc::new(Mutex::new(project));
        let jobs = Arc::new(Mutex::new(jobs));
        let project_guard = project.lock().unwrap();
        let worker_project = project.clone();
        let worker_jobs = jobs.clone();
        let worker = thread::spawn(move || {
            commit_job_result(
                &worker_project,
                &worker_jobs,
                project_id,
                job_id,
                recipe,
                result,
            )
        });
        thread::sleep(Duration::from_millis(20));
        assert!(jobs.try_lock().is_ok());
        drop(project_guard);
        worker.join().unwrap().unwrap();
    }

    #[test]
    fn demo_project_contains_workspace_dataset_and_run() {
        let project = build_demo_project().unwrap();
        assert_eq!(project.manifest.name, "Corporate Reputation Sample");
        assert_eq!(project.datasets.len(), 1);
        assert_eq!(project.models.len(), 1);
        assert_eq!(project.recipes.len(), 1);
        assert_eq!(project.results.len(), 1);
        assert_eq!(
            project.recipes[0].method_config,
            Some(MethodConfig::PlsBootstrap)
        );
        assert_eq!(project.recipes[0].settings.bootstrap_samples, 24);
        assert_eq!(project.recipes[0].settings.permutation_samples, 0);
        match &project.results[0].payload {
            qpls_core::AnalysisPayload::PlsPmV2 { bootstrap, .. } => {
                assert_eq!(bootstrap["plan"]["replicates"].as_u64(), Some(24));
            }
            payload => panic!("corporate sample produced unexpected payload {payload:?}"),
        }
        let workspace = project.layouts.get("workspace").unwrap();
        let dataset_id = project.datasets[0].id.to_string();
        assert_eq!(
            workspace["activeDatasetId"].as_str(),
            Some(dataset_id.as_str())
        );
        assert_eq!(workspace["nodes"].as_array().unwrap().len(), 4);
        assert_eq!(workspace["edges"].as_array().unwrap().len(), 3);
        assert_eq!(workspace["runs"].as_array().unwrap().len(), 1);
        assert_eq!(workspace["runs"][0]["name"], SAMPLE_RUN_DISPLAY_NAME);
        assert_eq!(workspace["runs"][0]["method"], SAMPLE_RUN_METHOD_NAME);
        assert_eq!(workspace["analysisSettings"]["bootstrapSamples"], 24);
        assert_eq!(workspace["analysisSettings"]["permutationSamples"], 0);
        assert!(workspace["runs"][0]["bootstrap"].is_object());
        assert!(workspace["runs"][0]["permutation"].is_null());
        let lineage = read_dataset_lineage(&project).unwrap().unwrap();
        assert_eq!(lineage.schema_version, 1);
        assert_eq!(lineage.records.len(), 1);
        assert_eq!(lineage.records[0].dataset_id, dataset_id);
        assert_eq!(
            lineage.records[0].operation,
            DatasetVersionOperation::Import
        );
        assert_eq!(lineage.records[0].created_at, None);
        assert_eq!(
            project.recipes[0].metadata.get("demo").map(String::as_str),
            Some("quickpls_v04_demo")
        );
        assert_eq!(
            project.recipes[0].id.to_string(),
            "00000000-0000-0000-0000-00000000d004"
        );
    }

    #[test]
    fn sample_project_selector_opens_only_the_four_advertised_projects() {
        for (sample_id, expected_name, expected_constructs) in [
            ("corporate_reputation", "Corporate Reputation Sample", 8),
            (
                "organizational_identification",
                "Organizational Identification Model",
                4,
            ),
            ("simple_pls", "Simple Reflective PLS Sample", 2),
            ("mediation", "Mediation Sample", 3),
        ] {
            let project = build_sample_project(sample_id).unwrap();
            assert_eq!(project.manifest.name, expected_name);
            assert_eq!(project.datasets.len(), 1);
            assert_eq!(project.models.len(), 1);
            assert_eq!(project.models[0].constructs.len(), expected_constructs);
            assert_eq!(project.recipes.len(), 1);
            assert_eq!(project.results.len(), 1);
        }

        for invalid in ["", "plspredict", "cbsem_cfa"] {
            assert!(build_sample_project(invalid).is_err());
        }
    }

    #[test]
    fn project_snapshot_exposes_canonical_content_independently_of_workspace_runs() {
        let mut project = build_demo_project().unwrap();
        project.source_archive_version = 4;
        project.migration_pending = true;
        project
            .compatibility_notices
            .push(qpls_project::ProjectCompatibilityNotice {
                result_id: project.results[0].id,
                diagnostic: qpls_core::Diagnostic {
                    code: "archive.legacy_result".into(),
                    level: qpls_core::DiagnosticLevel::Warning,
                    message: "Historical result remains readable under its original label".into(),
                },
            });
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({
                "activeDatasetId": project.datasets[0].id.to_string(),
                "nodes": [],
                "edges": [],
                "runs": []
            }),
        );

        let canonical_model_id = project.models[0].id.to_string();
        let canonical_recipe_id = project.recipes[0].id.to_string();
        let canonical_result_id = project.results[0].id.to_string();
        let response = snapshot(&project, Some("study.qpls".into()), None).unwrap();

        assert_eq!(response.models, project.models);
        assert_eq!(response.recipes, project.recipes);
        assert_eq!(response.results, project.results);
        assert_eq!(
            response.active_model_id.as_deref(),
            Some(canonical_model_id.as_str())
        );

        let wire = serde_json::to_value(response).unwrap();
        assert_eq!(wire["projectId"], project.manifest.project_id.to_string());
        assert_eq!(wire["activeModelId"], canonical_model_id);
        assert_eq!(wire["sourceArchiveVersion"], 4);
        assert_eq!(wire["migrationPending"], true);
        assert_eq!(
            wire["compatibilityNotices"][0]["resultId"],
            canonical_result_id
        );
        assert_eq!(
            wire["compatibilityNotices"][0]["code"],
            "archive.legacy_result"
        );
        assert_eq!(wire["saveWarning"], Value::Null);
        assert_eq!(wire["recipes"][0]["id"], canonical_recipe_id);
        assert_eq!(wire["results"][0]["id"], canonical_result_id);
        assert_eq!(
            wire["results"][0]["provenance"]["recipe_id"],
            canonical_recipe_id
        );
        assert_eq!(wire["workspace"]["runs"], serde_json::json!([]));
    }

    #[test]
    fn active_model_resolution_uses_only_canonical_archive_links() {
        let mut project = build_demo_project().unwrap();
        let first_model_id = project.recipes[0].model.id;
        let first_recipe_id = project.recipes[0].id;
        let first_result_id = project.results[0].id;
        let mut second_model = project.models[0].clone();
        second_model.id = Uuid::new_v4();
        second_model.name = "Second canonical model".into();
        let second_model_id = second_model.id;
        project.models.push(second_model);

        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "activeModelId": second_model_id }),
        );
        assert_eq!(
            resolve_active_model_id(&project),
            Some(second_model_id.to_string())
        );

        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "activeModelId": Uuid::new_v4() }),
        );
        assert_eq!(resolve_active_model_id(&project), None);

        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "activeRecipeId": first_recipe_id }),
        );
        assert_eq!(
            resolve_active_model_id(&project),
            Some(first_model_id.to_string())
        );

        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({
                "diagramOverlaySettings": { "selectedRunId": first_result_id }
            }),
        );
        assert_eq!(
            resolve_active_model_id(&project),
            Some(first_model_id.to_string())
        );
    }

    #[test]
    fn legacy_workspace_without_canonical_collections_has_an_additive_empty_snapshot() {
        let mut project = Project::new("Legacy workspace");
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "nodes": [], "edges": [] }),
        );

        let response = snapshot(&project, None, None).unwrap();
        assert!(response.models.is_empty());
        assert!(response.recipes.is_empty());
        assert!(response.results.is_empty());
        assert_eq!(response.active_model_id, None);

        let wire = serde_json::to_value(response).unwrap();
        assert_eq!(wire["models"], serde_json::json!([]));
        assert_eq!(wire["recipes"], serde_json::json!([]));
        assert_eq!(wire["results"], serde_json::json!([]));
        assert_eq!(wire["activeModelId"], Value::Null);
        assert_eq!(wire["workspace"]["nodes"], serde_json::json!([]));
    }

    #[test]
    fn malformed_reserved_data_lineage_never_becomes_an_empty_snapshot() {
        let mut project = Project::new("Malformed lineage");
        project.layouts.insert(
            qpls_project::PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1.into(),
            serde_json::json!({"schemaVersion": 1, "records": [], "unknown": true}),
        );

        let error = snapshot(&project, None, None).unwrap_err();
        assert!(error.contains("dataset lineage is invalid"));
        assert!(error.contains("malformed"));
    }

    #[test]
    fn recipe_embedded_model_remains_addressable_when_legacy_model_catalog_is_empty() {
        let mut project = build_demo_project().unwrap();
        let expected_model_id = project.recipes[0].model.id.to_string();
        let selected_run_id = project.results[0].id;
        project.models.clear();
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "selectedRunId": selected_run_id }),
        );

        let response = snapshot(&project, None, None).unwrap();

        assert!(response.models.is_empty());
        assert_eq!(response.recipes.len(), 1);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.active_model_id, Some(expected_model_id));
    }

    #[test]
    fn standalone_nca_recipe_never_reopens_as_a_phantom_active_model() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/results/v08_nca.recipe.json"
        ))
        .unwrap();
        recipe = recipe.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let placeholder_model_id = recipe.model.id;
        let recipe_id = recipe.id;
        let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let result_id = result.id;

        let mut project = Project::new("Standalone NCA");
        project.datasets.push(dataset);
        project.append_validated_result(recipe, result).unwrap();
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({
                "activeModelId": placeholder_model_id,
                "activeRecipeId": recipe_id,
                "selectedRunId": result_id
            }),
        );

        assert!(project.models.is_empty());
        assert_eq!(resolve_active_model_id(&project), None);
        assert_eq!(
            snapshot(&project, None, None).unwrap().active_model_id,
            None
        );

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("standalone-nca.qpls");
        save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        let reopened_snapshot =
            snapshot(&reopened, Some(archive.to_string_lossy().into()), None).unwrap();

        assert!(reopened_snapshot.models.is_empty());
        assert_eq!(reopened_snapshot.recipes.len(), 1);
        assert_eq!(reopened_snapshot.results.len(), 1);
        assert_eq!(reopened_snapshot.active_model_id, None);
    }

    #[test]
    fn workspace_save_candidate_atomically_upserts_the_typed_active_model() {
        let project = build_demo_project().unwrap();
        let original_name = project.models[0].name.clone();
        let recipe_count = project.recipes.len();
        let result_count = project.results.len();
        let mut edited_model = project.models[0].clone();
        edited_model.name = "Edited canonical model".into();
        edited_model.constructs[0].name = "Edited construct".into();

        let candidate = project_with_workspace_model(
            &project,
            serde_json::json!({
                "nodes": [],
                "edges": [],
                "activeModelId": Uuid::new_v4()
            }),
            Some(edited_model.clone()),
            Some(serde_json::json!({
                "nodes": [{"id": edited_model.constructs[0].id, "position": {"x": 12, "y": 34}}],
                "edges": []
            })),
        )
        .unwrap();

        assert_eq!(project.models[0].name, original_name);
        assert_eq!(candidate.models.len(), project.models.len());
        assert_eq!(candidate.models[0], edited_model);
        assert_eq!(candidate.recipes.len(), recipe_count);
        assert_eq!(candidate.results.len(), result_count);
        assert_eq!(
            candidate.layouts["workspace"]["activeModelId"],
            edited_model.id.to_string()
        );
        assert_eq!(
            resolve_active_model_id(&candidate),
            Some(edited_model.id.to_string())
        );
        assert_eq!(
            normalized_workspace_explorer(&candidate).model_presentations
                [&edited_model.id.to_string()]["nodes"][0]["position"],
            serde_json::json!({"x": 12, "y": 34})
        );

        let unknown_id = Uuid::new_v4().to_string();
        let without_model = project_with_workspace_model(
            &project,
            serde_json::json!({
                "nodes": [],
                "edges": [],
                "activeModelId": unknown_id
            }),
            None,
            None,
        )
        .unwrap();
        assert!(
            without_model.layouts["workspace"]
                .get("activeModelId")
                .is_none()
        );

        let mut read_only = project.clone();
        read_only.read_only = true;
        assert!(
            project_with_workspace_model(&read_only, serde_json::json!({}), None, None)
                .unwrap_err()
                .contains("read-only")
        );
        assert!(
            project_with_workspace_model(&project, Value::Null, None, None)
                .unwrap_err()
                .contains("workspace layout is invalid")
        );
    }

    #[test]
    fn workspace_explorer_snapshot_normalizes_presentations_and_legacy_reports() {
        let mut project = build_demo_project().unwrap();
        let model_id = project.models[0].id.to_string();
        let result_id = project.results[0].id;

        let legacy = snapshot(&project, Some("study.qpls".into()), None).unwrap();
        assert!(legacy.model_presentations.is_empty());
        assert_eq!(legacy.saved_reports.len(), 1);
        assert_eq!(legacy.saved_reports[0].result_id, result_id);
        assert_eq!(legacy.saved_reports[0].name, SAMPLE_RUN_DISPLAY_NAME);

        project.layouts.insert(
            WORKSPACE_EXPLORER_LAYOUT_KEY.into(),
            serde_json::json!({
                "schemaVersion": 1,
                "modelPresentations": {
                    (model_id.clone()): {"nodes": [], "edges": []},
                    (Uuid::new_v4().to_string()): {"nodes": []},
                    "not-a-uuid": []
                },
                "savedReports": [
                    {"resultId": result_id, "name": "Bad/Name", "savedAt": "not-a-date"},
                    {"resultId": "malformed-entry-without-required-fields"},
                    {"resultId": result_id, "name": "Duplicate", "savedAt": "2026-08-11T00:00:00Z"},
                    {"resultId": Uuid::new_v4(), "name": "Unknown", "savedAt": "2026-08-11T00:00:00Z"}
                ]
            }),
        );

        let normalized = snapshot(&project, None, None).unwrap();
        assert_eq!(
            normalized.model_presentations.keys().collect::<Vec<_>>(),
            vec![&model_id]
        );
        assert_eq!(normalized.saved_reports.len(), 1);
        assert_eq!(normalized.saved_reports[0].result_id, result_id);
        assert_eq!(normalized.saved_reports[0].name, SAMPLE_RUN_DISPLAY_NAME);
        assert_eq!(
            normalized.saved_reports[0].saved_at,
            project.results[0].provenance.completed_at.to_rfc3339()
        );

        write_workspace_explorer(&mut project, WorkspaceExplorerLayout::default()).unwrap();
        let explicitly_empty = snapshot(&project, None, None).unwrap();
        assert!(explicitly_empty.saved_reports.is_empty());

        project.layouts.insert(
            WORKSPACE_EXPLORER_LAYOUT_KEY.into(),
            serde_json::json!({
                "schemaVersion": WORKSPACE_EXPLORER_SCHEMA_VERSION + 1,
                "savedReports": [{
                    "resultId": result_id,
                    "name": "Unsupported layout entry",
                    "savedAt": "2026-08-11T00:00:00Z"
                }]
            }),
        );
        let unsupported = snapshot(&project, None, None).unwrap();
        assert!(unsupported.saved_reports.is_empty());
    }

    #[test]
    fn explorer_model_mutations_are_atomic_and_preserve_historical_science() {
        let project = build_demo_project().unwrap();
        let original_model_id = project.models[0].id;
        let original_recipe_model = project.recipes[0].model.clone();
        let recipe_count = project.recipes.len();
        let result_count = project.results.len();
        let mut current_model = project.models[0].clone();
        current_model.constructs[0].name = "Edited current construct".into();
        let created = project_after_explorer_mutation(
            &project,
            ProjectExplorerMutationRequest {
                mutation: ProjectExplorerMutation::CreateModel {
                    name: "Alternative model".into(),
                },
                current_model: Some(current_model.clone()),
                current_presentation: Some(serde_json::json!({
                    "nodes": [{"id": current_model.constructs[0].id, "position": {"x": 10, "y": 20}}],
                    "edges": []
                })),
                path: None,
            },
        )
        .unwrap();

        assert_eq!(created.models.len(), 2);
        assert_eq!(created.recipes.len(), recipe_count);
        assert_eq!(created.results.len(), result_count);
        assert_eq!(created.models[0], current_model);
        assert_eq!(created.recipes[0].model, original_recipe_model);
        let alternative_id = active_editable_model_id(&created).unwrap();
        assert_ne!(alternative_id, original_model_id);
        assert_eq!(
            created
                .models
                .iter()
                .find(|model| model.id == alternative_id)
                .unwrap()
                .name,
            "Alternative model"
        );
        assert!(
            normalized_workspace_explorer(&created)
                .model_presentations
                .contains_key(&original_model_id.to_string())
        );

        let renamed = project_after_explorer_mutation(
            &created,
            explorer_request(ProjectExplorerMutation::RenameModel {
                model_id: alternative_id,
                name: "Competing model".into(),
            }),
        )
        .unwrap();
        assert_eq!(
            renamed
                .models
                .iter()
                .find(|model| model.id == alternative_id)
                .unwrap()
                .name,
            "Competing model"
        );

        let duplicate_error = project_after_explorer_mutation(
            &renamed,
            explorer_request(ProjectExplorerMutation::RenameModel {
                model_id: alternative_id,
                name: renamed.models[0].name.to_uppercase(),
            }),
        )
        .unwrap_err();
        assert!(duplicate_error.contains("already exists"));
        assert_eq!(
            renamed
                .models
                .iter()
                .find(|model| model.id == alternative_id)
                .unwrap()
                .name,
            "Competing model"
        );

        let activated = project_after_explorer_mutation(
            &renamed,
            explorer_request(ProjectExplorerMutation::ActivateModel {
                model_id: original_model_id,
            }),
        )
        .unwrap();
        let deleted = project_after_explorer_mutation(
            &activated,
            explorer_request(ProjectExplorerMutation::DeleteModel {
                model_id: original_model_id,
            }),
        )
        .unwrap();
        assert_eq!(deleted.models.len(), 1);
        assert_eq!(active_editable_model_id(&deleted), Some(alternative_id));
        assert_eq!(deleted.recipes.len(), recipe_count);
        assert_eq!(deleted.results.len(), result_count);
        assert_eq!(deleted.recipes[0].model, original_recipe_model);
        assert_eq!(
            normalized_workspace_explorer(&deleted).saved_reports.len(),
            1
        );
        assert!(
            !normalized_workspace_explorer(&deleted)
                .model_presentations
                .contains_key(&original_model_id.to_string())
        );
    }

    #[test]
    fn explorer_saved_report_aliases_never_mutate_canonical_results() {
        let mut project = build_demo_project().unwrap();
        write_workspace_explorer(&mut project, WorkspaceExplorerLayout::default()).unwrap();
        let result_id = project.results[0].id;
        let original_result = project.results[0].clone();

        let saved = project_after_explorer_mutation(
            &project,
            explorer_request(ProjectExplorerMutation::SaveReport {
                result_id,
                name: "  Primary PLS report  ".into(),
            }),
        )
        .unwrap();
        let saved_reports = normalized_workspace_explorer(&saved).saved_reports;
        assert_eq!(saved_reports.len(), 1);
        assert_eq!(saved_reports[0].name, "Primary PLS report");
        assert!(chrono::DateTime::parse_from_rfc3339(&saved_reports[0].saved_at).is_ok());
        assert_eq!(saved.results, vec![original_result.clone()]);

        let renamed = project_after_explorer_mutation(
            &saved,
            explorer_request(ProjectExplorerMutation::RenameReport {
                result_id,
                name: "Reviewer report".into(),
            }),
        )
        .unwrap();
        assert_eq!(
            normalized_workspace_explorer(&renamed).saved_reports[0].name,
            "Reviewer report"
        );
        assert_eq!(renamed.results, vec![original_result.clone()]);

        let removed = project_after_explorer_mutation(
            &renamed,
            explorer_request(ProjectExplorerMutation::RemoveReport { result_id }),
        )
        .unwrap();
        assert!(
            normalized_workspace_explorer(&removed)
                .saved_reports
                .is_empty()
        );
        assert_eq!(removed.results, vec![original_result]);
        assert!(removed.recipes.len() == 1);
    }

    #[test]
    fn explorer_mutations_reject_read_only_projects_invalid_names_and_active_jobs() {
        let mut read_only = build_demo_project().unwrap();
        read_only.read_only = true;
        assert!(
            project_after_explorer_mutation(
                &read_only,
                explorer_request(ProjectExplorerMutation::CreateModel {
                    name: "Blocked".into(),
                }),
            )
            .unwrap_err()
            .contains("read-only")
        );

        let project = build_demo_project().unwrap();
        for invalid_name in ["", "Bad/Model", "Trailing."] {
            assert!(
                project_after_explorer_mutation(
                    &project,
                    explorer_request(ProjectExplorerMutation::CreateModel {
                        name: invalid_name.into(),
                    }),
                )
                .is_err()
            );
        }

        let (_, mut jobs, job_id, _, _) = fixture(false);
        assert!(has_active_jobs(&jobs));
        let blocked = guarded_project_after_explorer_mutation(
            &project,
            &jobs,
            explorer_request(ProjectExplorerMutation::CreateModel {
                name: "Blocked while running".into(),
            }),
        )
        .unwrap_err();
        assert!(blocked.contains("calculation is active"));
        assert_eq!(project.models.len(), 1);
        jobs.get_mut(&job_id).unwrap().snapshot.state = JobState::Completed;
        assert!(!has_active_jobs(&jobs));
        assert!(
            guarded_project_after_explorer_mutation(
                &project,
                &jobs,
                explorer_request(ProjectExplorerMutation::CreateModel {
                    name: "Allowed after completion".into(),
                }),
            )
            .is_ok()
        );
    }

    #[test]
    fn explorer_wire_contract_is_camel_case_and_round_trips_through_project_archives() {
        let project = build_demo_project().unwrap();
        let model_id = project.models[0].id;
        let request: ProjectExplorerMutationRequest = serde_json::from_value(serde_json::json!({
            "mutation": {"kind": "activate_model", "modelId": model_id},
            "currentModel": null,
            "currentPresentation": null,
            "path": "study.qpls"
        }))
        .unwrap();
        assert_eq!(
            request.mutation,
            ProjectExplorerMutation::ActivateModel { model_id }
        );

        let candidate = project_with_workspace_model(
            &project,
            project.layouts["workspace"].clone(),
            Some(project.models[0].clone()),
            Some(serde_json::json!({"nodes": [], "edges": []})),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("workspace-explorer.qpls");
        save_project(&archive, &candidate).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        let response = snapshot(&reopened, Some(archive.to_string_lossy().into()), None).unwrap();
        let wire = serde_json::to_value(response).unwrap();
        assert!(wire["modelPresentations"][model_id.to_string()].is_object());
        assert_eq!(
            wire["savedReports"][0]["resultId"],
            project.results[0].id.to_string()
        );
        assert!(wire.get("model_presentations").is_none());
        assert!(wire.get("saved_reports").is_none());
    }

    #[test]
    fn dataset_row_pages_are_id_scoped_and_bounded() {
        let dataset = import_delimited_bytes(
            b"x,y\n1,a\n2,b\n3,NA\n4,d\n5,e\n",
            "paged.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let dataset_id = dataset.id.to_string();
        let snapshot = dataset_snapshot(&dataset);
        let mut project = Project::new("Paged data fixture");
        project.datasets.push(dataset);

        let page = dataset_rows_page(&project, &dataset_id, 2, 2).unwrap();
        assert_eq!(page.dataset_id, dataset_id);
        assert_eq!((page.offset, page.limit, page.row_count), (2, 2, 5));
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0]["x"].as_deref(), Some("3"));
        assert_eq!(page.rows[0]["y"], None);

        let capped = dataset_rows_page(&project, &dataset_id, 3, usize::MAX).unwrap();
        assert_eq!(capped.limit, MAX_DATASET_ROW_PAGE_SIZE);
        assert_eq!(capped.rows.len(), 2);
        assert_eq!(snapshot.missing, 1);
        assert_eq!(snapshot.missing_by_column["y"], 1);
        assert!(dataset_rows_page(&project, &dataset_id, 0, 0).is_err());
        assert!(dataset_rows_page(&project, "not-active", 0, 10).is_err());
    }

    #[test]
    fn group_profile_scans_full_data_and_counts_model_complete_cases() {
        let mut csv = String::from("group,x1,x2\n");
        for row in 0..100 {
            csv.push_str(&format!("A,{},{}\n", row + 1, row + 2));
        }
        for row in 0..20 {
            let x2 = if row == 0 {
                "NA".to_owned()
            } else {
                (row + 3).to_string()
            };
            csv.push_str(&format!("B,{},{}\n", row + 2, x2));
        }
        for row in 0..5 {
            csv.push_str(&format!("NA,{},{}\n", row + 1, row + 2));
        }
        let mut dataset = import_delimited_bytes(
            csv.as_bytes(),
            "groups.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        dataset.schema.columns[0]
            .value_labels
            .insert("A".into(), "Control".into());
        let dataset_id = dataset.id.to_string();
        let mut project = Project::new("Group profile fixture");
        project.datasets.push(dataset);

        let profile = build_dataset_group_profile(
            &project,
            &dataset_id,
            "group",
            &["x1".into(), "x2".into()],
        )
        .unwrap();
        assert_eq!(profile.row_count, 125);
        assert_eq!(profile.missing_count, 5);
        assert_eq!(profile.unsupported_count, 0);
        assert!(!profile.truncated);
        assert_eq!(profile.groups.len(), 2);
        assert_eq!(
            profile.groups[0],
            DatasetGroupProfileValue {
                value: "A".into(),
                label: Some("Control".into()),
                observations: 100,
                complete_cases: 100,
            }
        );
        assert_eq!(profile.groups[1].value, "B");
        assert_eq!(profile.groups[1].observations, 20);
        assert_eq!(profile.groups[1].complete_cases, 19);
        let pre_model_profile =
            build_dataset_group_profile(&project, &dataset_id, "group", &[]).unwrap();
        assert_eq!(pre_model_profile.groups[0].complete_cases, 100);
        assert_eq!(pre_model_profile.groups[1].complete_cases, 20);
        assert!(
            build_dataset_group_profile(
                &project,
                &dataset_id,
                "group",
                &["group".into(), "x1".into()],
            )
            .unwrap_err()
            .contains("cannot also be a model indicator")
        );
    }

    #[test]
    fn native_micom_v31_preflight_requires_review_and_accepts_profiled_groups() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/results/mga_reference.csv"),
            "mga_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/results/mga_reference.recipe.json"
        ))
        .unwrap();
        historical.dataset_fingerprint = dataset.fingerprint.0.clone();
        historical
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        historical
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        historical
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.method_config = Some(MethodConfig::Micom {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            permutation_samples: 5_000,
            configural_invariance_confirmed: false,
        });
        let projected = recipe.with_effective_metadata().unwrap();
        assert!(
            validate_mga_dataset_contract(&dataset, &projected)
                .unwrap_err()
                .contains("micom.configural_invariance_not_confirmed")
        );
        recipe.method_config = Some(MethodConfig::Micom {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            permutation_samples: 5_000,
            configural_invariance_confirmed: true,
        });
        validate_mga_dataset_contract(&dataset, &recipe.with_effective_metadata().unwrap())
            .unwrap();
    }

    #[test]
    fn column_metadata_edits_create_immutable_dataset_versions() {
        let dataset = import_delimited_bytes(
            b"x,y\n1,2\n3,4\n",
            "metadata.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = dataset.id;
        let source_fingerprint = dataset.fingerprint.clone();
        let source_batch = dataset.batch.clone();
        let mut metadata = dataset.schema.columns[0].clone();
        metadata.label = Some("Predictor score".to_owned());
        metadata.theoretical_min = Some(1.0);
        metadata.theoretical_max = Some(7.0);
        let mut project = Project::new("Metadata version fixture");
        append_dataset(&mut project, dataset).unwrap();

        let revised =
            version_column_metadata(&mut project, &source_id.to_string(), "x", metadata.clone())
                .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].id, source_id);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        assert_ne!(revised.id, source_id.to_string());
        assert_ne!(revised.fingerprint, source_fingerprint.0);
        assert_eq!(project.datasets[1].schema.columns[0], metadata);
        assert_eq!(project.datasets[1].batch, source_batch);
        let lineage = read_dataset_lineage(&project).unwrap().unwrap();
        assert_eq!(lineage.records.len(), 2);
        assert_eq!(lineage.records[0].dataset_id, source_id.to_string());
        assert_eq!(
            lineage.records[0].operation,
            DatasetVersionOperation::Import
        );
        assert_eq!(lineage.records[1].dataset_id, revised.id);
        assert_eq!(
            lineage.records[1].parent_dataset_id,
            Some(source_id.to_string())
        );
        assert_eq!(
            lineage.records[1].operation,
            DatasetVersionOperation::Metadata
        );

        let no_op = version_column_metadata(&mut project, &revised.id, "x", metadata).unwrap();
        assert_eq!(no_op.id, revised.id);
        assert_eq!(project.datasets.len(), 2);
        assert_eq!(
            read_dataset_lineage(&project)
                .unwrap()
                .unwrap()
                .records
                .len(),
            2
        );

        project.layouts.insert(
            "workspace".to_owned(),
            serde_json::json!({ "activeDatasetId": revised.id }),
        );
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("metadata-versions.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(reopened.datasets.len(), 2);
        assert_eq!(reopened.layouts["workspace"]["activeDatasetId"], revised.id);
        assert!(
            reopened
                .datasets
                .iter()
                .any(|dataset| dataset.fingerprint == source_fingerprint)
        );

        project.read_only = true;
        let unchanged_metadata = project.datasets[1].schema.columns[0].clone();
        let blocked = version_column_metadata(&mut project, &revised.id, "x", unchanged_metadata)
            .unwrap_err();
        assert!(blocked.contains("read-only"));

        let dataset_count = project.datasets.len();
        let rejected_import = project.datasets[0].clone();
        assert!(
            append_dataset(&mut project, rejected_import)
                .unwrap_err()
                .contains("read-only")
        );
        assert_eq!(project.datasets.len(), dataset_count);
    }

    #[test]
    fn recode_dataset_versions_full_data_persists_lineage_and_can_reactivate_source() {
        let mut csv = String::from("score,group\n");
        for row in 0..525 {
            csv.push_str(&format!(
                "{},{}\n",
                row % 3,
                if row % 2 == 0 { "A" } else { "B" }
            ));
        }
        let source = import_delimited_bytes(
            csv.as_bytes(),
            "large-recode.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = source.id.to_string();
        let source_fingerprint = source.fingerprint.clone();
        let source_batch = source.batch.clone();
        let mut project = Project::new("Recode version fixture");
        project.datasets.push(source);

        let mutation = version_recode_column(
            &mut project,
            &source_id,
            RecodeColumnSpec {
                source_column: "score".into(),
                target_column: "score_code".into(),
                target_label: Some("Recoded score".into()),
                target_type: qpls_data::ColumnType::Numeric,
                target_scale: qpls_data::ScaleType::Ordinal,
                mappings: vec![qpls_data::RecodeValueMapping {
                    source: "1".into(),
                    target: Some("10".into()),
                }],
                unmapped: qpls_data::RecodeUnmappedPolicy::KeepOriginal,
            },
        )
        .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].id.to_string(), source_id);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        assert_ne!(mutation.dataset.id, source_id);
        assert_eq!(
            mutation.version.parent_dataset_id.as_deref(),
            Some(source_id.as_str())
        );
        assert_eq!(mutation.version.operation, DatasetVersionOperation::Recode);
        assert_eq!(
            mutation.version.target_column.as_deref(),
            Some("score_code")
        );
        assert_eq!(
            project.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );

        let late_page = dataset_rows_page(&project, &mutation.dataset.id, 510, 3).unwrap();
        assert_eq!(late_page.rows[0]["score_code"].as_deref(), Some("0"));
        assert_eq!(late_page.rows[1]["score_code"].as_deref(), Some("10"));
        assert_eq!(late_page.rows[2]["score_code"].as_deref(), Some("2"));

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("recode-versions.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let mut reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(reopened.datasets.len(), 2);
        assert_eq!(
            read_dataset_lineage(&reopened).unwrap().unwrap().records,
            vec![mutation.version]
        );
        assert_eq!(
            reopened.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );

        let reactivated = activate_dataset_version(&mut reopened, &source_id).unwrap();
        assert_eq!(reactivated.id, source_id);
        assert_eq!(reopened.layouts["workspace"]["activeDatasetId"], source_id);

        reopened.read_only = true;
        let dataset_count = reopened.datasets.len();
        let blocked = version_recode_column(
            &mut reopened,
            &reactivated.id,
            RecodeColumnSpec {
                source_column: "score".into(),
                target_column: "blocked".into(),
                target_label: None,
                target_type: qpls_data::ColumnType::Numeric,
                target_scale: qpls_data::ScaleType::Continuous,
                mappings: vec![qpls_data::RecodeValueMapping {
                    source: "1".into(),
                    target: Some("2".into()),
                }],
                unmapped: qpls_data::RecodeUnmappedPolicy::KeepOriginal,
            },
        )
        .unwrap_err();
        assert!(blocked.contains("read-only"));
        assert_eq!(reopened.datasets.len(), dataset_count);
    }

    #[test]
    fn generic_dataset_transform_versions_data_and_persists_exact_lineage() {
        let source = import_delimited_bytes(
            b"score,group\n1,A\n3,B\n5,A\n",
            "transform-source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = source.id.to_string();
        let source_fingerprint = source.fingerprint.clone();
        let source_batch = source.batch.clone();
        let mut project = Project::new("Transformation version fixture");
        project.datasets.push(source);
        let spec = DatasetTransformationSpecV2::ReverseScale {
            source_column: "score".into(),
            target_column: "score_reversed".into(),
            scale_min: 1.0,
            scale_max: 5.0,
            target_label: Some("Reversed score".into()),
        };

        let mutation = version_dataset_transformation(
            &mut project,
            &source_id,
            spec.clone(),
            "transform-source - derived".into(),
        )
        .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        assert_eq!(
            mutation.version.operation,
            DatasetVersionOperation::Transform
        );
        assert_eq!(
            mutation.version.parent_dataset_id.as_deref(),
            Some(source_id.as_str())
        );
        assert_eq!(
            mutation.version.target_column.as_deref(),
            Some("score_reversed")
        );
        let lineage = mutation.version.transformation.as_ref().unwrap();
        assert_eq!(lineage.spec, spec);
        assert_eq!(lineage.source_dataset_id, source_id);
        assert_eq!(lineage.output_dataset_id, mutation.dataset.id);
        assert_eq!(
            lineage.output_dataset_fingerprint,
            mutation.dataset.fingerprint
        );
        assert_eq!(lineage.output_columns, vec!["score_reversed"]);
        assert_eq!(lineage.output_missing_count, 0);

        let rows = dataset_rows_page(&project, &mutation.dataset.id, 0, 3).unwrap();
        assert_eq!(rows.rows[0]["score_reversed"].as_deref(), Some("5"));
        assert_eq!(rows.rows[1]["score_reversed"].as_deref(), Some("3"));
        assert_eq!(rows.rows[2]["score_reversed"].as_deref(), Some("1"));

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("transformation-versions.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(
            read_dataset_lineage(&reopened).unwrap().unwrap().records,
            vec![mutation.version]
        );
        assert_eq!(
            reopened.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );
    }

    #[test]
    fn standardize_dataset_transform_versions_once_and_reopens_exact_lineage() {
        let source = import_delimited_bytes(
            b"score\n1\n3\n5\n",
            "standardize-source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = source.id.to_string();
        let source_fingerprint = source.fingerprint.clone();
        let source_batch = source.batch.clone();
        let mut project = Project::new("Standardization version fixture");
        project.datasets.push(source);
        let spec = DatasetTransformationSpecV2::Standardize {
            source_column: "score".into(),
            target_column: "z_score".into(),
            denominator: qpls_core::StandardDeviationDenominatorV2::SampleNMinusOne,
            target_label: Some("Standardized score".into()),
        };

        let mutation = version_dataset_transformation(
            &mut project,
            &source_id,
            spec.clone(),
            "standardize-source - derived".into(),
        )
        .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        assert_eq!(
            mutation.version.operation,
            DatasetVersionOperation::Transform
        );
        assert_eq!(
            mutation.version.parent_dataset_id.as_deref(),
            Some(source_id.as_str())
        );
        assert_eq!(mutation.version.target_column.as_deref(), Some("z_score"));
        let lineage = mutation.version.transformation.as_ref().unwrap();
        assert_eq!(lineage.spec, spec);
        assert_eq!(lineage.input_columns, vec!["score"]);
        assert_eq!(lineage.output_columns, vec!["z_score"]);
        assert_eq!(lineage.spec_sha256.len(), 64);
        assert_eq!(lineage.output_dataset_id, mutation.dataset.id);

        let rows = dataset_rows_page(&project, &mutation.dataset.id, 0, 3).unwrap();
        assert_eq!(rows.rows[0]["z_score"].as_deref(), Some("-1"));
        assert_eq!(rows.rows[1]["z_score"].as_deref(), Some("0"));
        assert_eq!(rows.rows[2]["z_score"].as_deref(), Some("1"));

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("standardize-version.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(
            read_dataset_lineage(&reopened).unwrap().unwrap().records,
            vec![mutation.version]
        );
        assert_eq!(
            reopened.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );
    }

    #[test]
    fn add_column_transform_versions_once_and_reopens_zero_input_lineage() {
        let source = import_delimited_bytes(
            b"score,group\n1,A\n3,B\n5,A\n",
            "add-column-source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = source.id.to_string();
        let source_fingerprint = source.fingerprint.clone();
        let source_batch = source.batch.clone();
        let mut project = Project::new("Add-column version fixture");
        project.datasets.push(source);
        let spec = DatasetTransformationSpecV2::AddColumn {
            target_column: "cohort".into(),
            value: qpls_core::DatasetCellV2::Text("pilot".into()),
            target_type: qpls_data::ColumnType::Text,
            target_scale: qpls_data::ScaleType::Nominal,
            target_label: Some("Cohort".into()),
            value_labels: std::collections::BTreeMap::new(),
        };

        let mutation = version_dataset_transformation(
            &mut project,
            &source_id,
            spec.clone(),
            "add-column-source - derived".into(),
        )
        .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        assert_eq!(mutation.version.summary, "Added cohort");
        assert_eq!(mutation.version.source_column, None);
        assert_eq!(mutation.version.target_column.as_deref(), Some("cohort"));
        let lineage = mutation.version.transformation.as_ref().unwrap();
        assert_eq!(lineage.spec, spec);
        assert!(lineage.input_columns.is_empty());
        assert_eq!(lineage.output_columns, vec!["cohort"]);

        let rows = dataset_rows_page(&project, &mutation.dataset.id, 0, 3).unwrap();
        assert!(
            rows.rows
                .iter()
                .all(|row| row["cohort"].as_deref() == Some("pilot"))
        );

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("add-column-version.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(
            read_dataset_lineage(&reopened).unwrap().unwrap().records,
            vec![mutation.version]
        );
        assert_eq!(
            reopened.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );
    }

    #[test]
    fn multi_missing_marker_transform_commits_one_child_and_reopens_exact_lineage() {
        let source = import_delimited_bytes(
            b"score,group\n1,A\n-99,MISSING_CODE\n3,B\n",
            "missing-marker-source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_id = source.id.to_string();
        let source_fingerprint = source.fingerprint.clone();
        let source_batch = source.batch.clone();
        let mut project = Project::new("Missing-marker version fixture");
        project.datasets.push(source);
        let spec = DatasetTransformationSpecV2::MissingMarkers {
            columns: vec![
                qpls_core::DatasetMissingMarkerColumnV2 {
                    source_column: "score".into(),
                    target_column: "score_clean".into(),
                    markers: vec![qpls_core::NonMissingDatasetCellV2::Number(-99.0)],
                    target_type: qpls_data::ColumnType::Numeric,
                    target_scale: qpls_data::ScaleType::Continuous,
                    target_label: Some("Clean score".into()),
                    value_labels: std::collections::BTreeMap::new(),
                },
                qpls_core::DatasetMissingMarkerColumnV2 {
                    source_column: "group".into(),
                    target_column: "group_clean".into(),
                    markers: vec![qpls_core::NonMissingDatasetCellV2::Text(
                        "MISSING_CODE".into(),
                    )],
                    target_type: qpls_data::ColumnType::Text,
                    target_scale: qpls_data::ScaleType::Nominal,
                    target_label: Some("Clean group".into()),
                    value_labels: std::collections::BTreeMap::new(),
                },
            ],
        };

        let mutation = version_dataset_transformation(
            &mut project,
            &source_id,
            spec.clone(),
            "missing-marker-source - derived".into(),
        )
        .unwrap();

        assert_eq!(project.datasets.len(), 2);
        assert_eq!(project.datasets[0].fingerprint, source_fingerprint);
        assert_eq!(project.datasets[0].batch, source_batch);
        let lineage = mutation.version.transformation.as_ref().unwrap();
        assert_eq!(lineage.spec, spec);
        assert_eq!(lineage.input_columns, vec!["score", "group"]);
        assert_eq!(lineage.output_columns, vec!["score_clean", "group_clean"]);
        assert_eq!(lineage.output_missing_count, 2);
        let records = read_dataset_lineage(&project).unwrap().unwrap();
        assert_eq!(records.records.len(), 1);

        let rows = dataset_rows_page(&project, &mutation.dataset.id, 0, 3).unwrap();
        assert_eq!(rows.rows[0]["score_clean"].as_deref(), Some("1"));
        assert_eq!(rows.rows[1]["score_clean"], None);
        assert_eq!(rows.rows[1]["group_clean"], None);
        assert_eq!(rows.rows[2]["group_clean"].as_deref(), Some("B"));

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("multi-missing-marker-version.qpls");
        qpls_project::save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        assert_eq!(
            read_dataset_lineage(&reopened).unwrap().unwrap().records,
            vec![mutation.version]
        );
        assert_eq!(reopened.datasets.len(), 2);
        assert_eq!(
            reopened.layouts["workspace"]["activeDatasetId"],
            mutation.dataset.id
        );
    }

    #[test]
    fn text_export_writes_only_the_selected_supported_utf8_file() {
        use std::fs;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("results.csv");
        let contents = "metric,value\nbeta,\u{03b2}\n";

        write_text_export(&path, contents).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        let unsupported = directory.path().join("results.txt");
        let error = write_text_export(&unsupported, "not written").unwrap_err();
        assert!(error.contains("Only .csv, .html, and .svg"));
        assert!(!unsupported.exists());
        let relative_error =
            write_text_export(Path::new("relative.csv"), "not written").unwrap_err();
        assert!(relative_error.contains("absolute path"));
    }
    #[test]
    fn xlsx_table_export_writes_readable_workbook() {
        use calamine::{Reader, open_workbook_auto};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tables.xlsx");
        let tables = vec![ExportTable {
            title: "WPLS weights".into(),
            status: "validated_scope".into(),
            warning: Some(
                "Validated for the documented QuickPLS v1.0.0 supported scope; unsupported shapes remain blocked."
                    .into(),
            ),
            columns: vec!["Metric".into(), "Value".into()],
            rows: vec![vec!["case_weight_column".into(), "case_wt".into()]],
        }];

        write_xlsx_tables(&path, &tables).unwrap();

        let mut workbook = open_workbook_auto(&path).unwrap();
        let range = workbook.worksheet_range("WPLS weights").unwrap();
        assert_eq!(range.get((0, 0)).unwrap().to_string(), "WPLS weights");
        assert_eq!(range.get((1, 1)).unwrap().to_string(), "validated_scope");
        assert_eq!(range.get((5, 1)).unwrap().to_string(), "case_wt");
        assert_eq!(
            safe_sheet_name("Model fit and likelihood-ratio test", 0),
            "Model fit and likelihood-ratio"
        );

        let collision_path = directory.path().join("micom-tables.xlsx");
        let collision_tables = vec![
            ExportTable {
                title: "MICOM Step 3 - equality of composite means".into(),
                status: "validated_scope".into(),
                warning: None,
                columns: vec!["Construct".into(), "Difference".into()],
                rows: vec![vec!["A".into(), "0.1".into()]],
            },
            ExportTable {
                title: "MICOM Step 3 - equality of composite variances".into(),
                status: "validated_scope".into(),
                warning: None,
                columns: vec!["Construct".into(), "Difference".into()],
                rows: vec![vec!["A".into(), "0.2".into()]],
            },
        ];
        write_xlsx_tables(&collision_path, &collision_tables).unwrap();
        let mut collision_workbook = open_workbook_auto(&collision_path).unwrap();
        let sheet_names = collision_workbook.sheet_names();
        assert_eq!(sheet_names.len(), 2);
        assert_ne!(sheet_names[0].to_lowercase(), sheet_names[1].to_lowercase());
        assert!(sheet_names.iter().all(|name| name.chars().count() <= 31));
        assert_eq!(
            collision_workbook
                .worksheet_range(&sheet_names[0])
                .unwrap()
                .get((0, 0))
                .unwrap()
                .to_string(),
            collision_tables[0].title
        );
        assert_eq!(
            collision_workbook
                .worksheet_range(&sheet_names[1])
                .unwrap()
                .get((0, 0))
                .unwrap()
                .to_string(),
            collision_tables[1].title
        );
    }

    #[test]
    fn desktop_runner_payload_matches_cli_serialized_artifact() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = recipe.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let desktop_result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let cli_result: AnalysisResult = serde_json::from_slice(include_bytes!(
            "../../validation/results/pls_quickpls_path_mode_a.json"
        ))
        .unwrap();
        let mut desktop_payload = serde_json::to_value(&desktop_result.payload).unwrap();
        assert_eq!(
            desktop_payload["estimation"]["algorithm_convergence_receipt"]["contract_version"],
            "pls_algorithm_convergence_receipt_v1"
        );
        assert_eq!(
            desktop_payload["estimation"]["point_estimate_attribution"]["contract_version"],
            "pls_point_estimate_attribution_v1"
        );
        desktop_payload["estimation"]
            .as_object_mut()
            .unwrap()
            .remove("algorithm_convergence_receipt");
        desktop_payload["estimation"]
            .as_object_mut()
            .unwrap()
            .remove("point_estimate_attribution");
        assert_json_close(
            &desktop_payload,
            &serde_json::to_value(&cli_result.payload).unwrap(),
            1e-12,
        );
        assert_eq!(desktop_result.diagnostics, cli_result.diagnostics);
        assert_eq!(
            desktop_result.provenance.method_version,
            cli_result.provenance.method_version
        );
        assert_eq!(
            desktop_result.provenance.dataset_fingerprint,
            cli_result.provenance.dataset_fingerprint
        );
        assert_eq!(
            desktop_result.provenance.settings,
            cli_result.provenance.settings
        );
    }

    fn assert_json_close(left: &Value, right: &Value, tolerance: f64) {
        assert_json_close_at(left, right, tolerance, "$");
    }

    fn assert_json_close_at(left: &Value, right: &Value, tolerance: f64, path: &str) {
        match (left, right) {
            (Value::Number(left), Value::Number(right)) => {
                let left = left.as_f64().unwrap();
                let right = right.as_f64().unwrap();
                assert!(
                    (left - right).abs() <= tolerance,
                    "{path}: expected {left}, actual {right}"
                );
            }
            (Value::Array(left), Value::Array(right)) => {
                assert_eq!(left.len(), right.len(), "{path}: array length mismatch");
                for (index, (left, right)) in left.iter().zip(right).enumerate() {
                    assert_json_close_at(left, right, tolerance, &format!("{path}[{index}]"));
                }
            }
            (Value::Object(left), Value::Object(right)) => {
                for (key, left) in left {
                    let Some(right) = right.get(key) else {
                        assert!(
                            left.is_null(),
                            "{path}.{key}: missing non-null field in right payload"
                        );
                        continue;
                    };
                    assert_json_close_at(left, right, tolerance, &format!("{path}.{key}"));
                }
                for (key, right) in right {
                    assert!(
                        left.contains_key(key) || right.is_null(),
                        "{path}.{key}: unexpected non-null field in right payload"
                    );
                }
            }
            _ => assert_eq!(left, right, "{path}: value mismatch"),
        }
    }
}

#[cfg(test)]
mod diagnostic_bundle_tests {
    use super::*;
    use std::{io::Read, sync::Barrier, thread};

    #[test]
    fn diagnostic_redaction_removes_paths_emails_url_suffixes_and_bearer_tokens() {
        let raw = concat!(
            "parser at C:\\Users\\Alice Smith\\Documents\\study.qpls; ",
            "contact alice@example.org; ",
            "GET https://support.example.org/report?token=secret#private; ",
            "Authorization: Bearer eyJhbGciOiJub25l.secret"
        );
        let mut counts = DiagnosticRedactionCounts::default();

        let redacted = redact_diagnostic_text(raw, &mut counts);

        assert!(redacted.contains("<redacted-path>/study.qpls"));
        assert!(redacted.contains("<redacted-email>"));
        assert!(redacted.contains("https://support.example.org/report"));
        assert!(redacted.contains("Bearer <redacted-token>"));
        for forbidden in [
            "Alice Smith",
            "alice@example.org",
            "token=secret",
            "#private",
            "eyJhbGciOiJub25l.secret",
        ] {
            assert!(!redacted.contains(forbidden), "found {forbidden}");
        }
        assert_eq!(counts.windows_paths, 1);
        assert_eq!(counts.email_addresses, 1);
        assert_eq!(counts.url_queries_or_fragments, 1);
        assert_eq!(counts.bearer_tokens, 1);
    }

    #[test]
    fn diagnostic_preview_is_bounded_local_only_and_required_for_save() {
        let diagnostics = DesktopDiagnostics::new();

        let preview = create_diagnostic_preview(&diagnostics, None).unwrap();

        assert!(preview.local_only);
        assert_eq!(preview.network_activity, "none");
        assert_eq!(preview.entry_count, 3);
        assert!(preview.event_count <= MAX_DIAGNOSTIC_EVENTS);
        assert!(preview.estimated_uncompressed_bytes <= MAX_DIAGNOSTIC_UNCOMPRESSED_BYTES);
        assert_eq!(preview.staged_contents.system.user_data_included, false);
        assert_eq!(preview.staged_contents.system.network_accessed, false);
        assert_eq!(preview.staged_contents.events.len(), preview.event_count);
        assert_eq!(preview.staged_contents.manifest.entries.len(), 2);
        assert!(
            diagnostics
                .consume_preview("not-the-current-preview")
                .is_err()
        );
        assert!(diagnostics.consume_preview(&preview.preview_id).is_ok());
        assert!(diagnostics.consume_preview(&preview.preview_id).is_err());
        assert!(diagnostics.cancel_preview(&preview.preview_id).is_err());
    }

    #[test]
    fn diagnostic_preview_ids_are_atomic_single_use_under_concurrent_saves() {
        let diagnostics = DesktopDiagnostics::new();
        let preview = diagnostics.create_preview(None).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let attempts = (0..2)
            .map(|_| {
                let diagnostics = diagnostics.clone();
                let preview_id = preview.preview_id.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    diagnostics.consume_preview(&preview_id).is_ok()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let successes = attempts
            .into_iter()
            .map(|attempt| attempt.join().unwrap())
            .filter(|success| *success)
            .count();

        assert_eq!(successes, 1);
        assert!(diagnostics.consume_preview(&preview.preview_id).is_err());
    }

    #[test]
    fn independently_created_preview_ids_never_overwrite_each_others_staging() {
        let diagnostics = DesktopDiagnostics::new();
        let first = diagnostics.create_preview(None).unwrap();
        let second = diagnostics.create_preview(None).unwrap();

        assert_ne!(first.preview_id, second.preview_id);
        assert!(diagnostics.consume_preview(&first.preview_id).is_ok());
        assert!(diagnostics.consume_preview(&second.preview_id).is_ok());
    }

    #[test]
    fn abandoned_diagnostic_previews_evict_the_oldest_without_blocking_recovery() {
        let diagnostics = DesktopDiagnostics::new();
        let previews = (0..(MAX_PENDING_DIAGNOSTIC_PREVIEWS + 3))
            .map(|_| diagnostics.create_preview(None).unwrap())
            .collect::<Vec<_>>();

        let state = diagnostics.0.lock().unwrap();
        assert_eq!(
            state.pending_previews.len(),
            MAX_PENDING_DIAGNOSTIC_PREVIEWS
        );
        drop(state);
        for preview in previews.iter().take(3) {
            assert!(diagnostics.consume_preview(&preview.preview_id).is_err());
        }
        for preview in previews.iter().skip(3) {
            assert!(diagnostics.consume_preview(&preview.preview_id).is_ok());
        }
    }

    #[test]
    fn expired_diagnostic_previews_are_pruned_before_capacity_and_consumption_checks() {
        let diagnostics = DesktopDiagnostics::new();
        let expired = diagnostics.create_preview(None).unwrap();
        {
            let mut state = diagnostics.0.lock().unwrap();
            state
                .pending_previews
                .get_mut(&expired.preview_id)
                .unwrap()
                .expires_at = Instant::now() - Duration::from_secs(1);
        }

        for _ in 0..(MAX_PENDING_DIAGNOSTIC_PREVIEWS + 2) {
            diagnostics.create_preview(None).unwrap();
        }

        assert!(diagnostics.consume_preview(&expired.preview_id).is_err());
        assert_eq!(
            diagnostics.0.lock().unwrap().pending_previews.len(),
            MAX_PENDING_DIAGNOSTIC_PREVIEWS
        );
    }

    #[test]
    fn concurrent_refresh_and_cancel_leave_only_the_new_preview_consumable() {
        let diagnostics = DesktopDiagnostics::new();
        let old = diagnostics.create_preview(None).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let cancel = {
            let diagnostics = diagnostics.clone();
            let preview_id = old.preview_id.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                diagnostics.cancel_preview(&preview_id)
            })
        };
        let refresh = {
            let diagnostics = diagnostics.clone();
            let preview_id = old.preview_id.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                diagnostics.create_preview(Some(&preview_id))
            })
        };
        barrier.wait();
        let _ = cancel.join().unwrap();
        let new_preview = refresh.join().unwrap().unwrap();

        assert_ne!(old.preview_id, new_preview.preview_id);
        assert!(diagnostics.consume_preview(&old.preview_id).is_err());
        assert!(diagnostics.consume_preview(&new_preview.preview_id).is_ok());
        assert!(
            diagnostics
                .consume_preview(&new_preview.preview_id)
                .is_err()
        );
    }

    #[test]
    fn diagnostic_bundle_writes_only_the_fixed_allowlist_and_describes_payload_hashes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("quickpls-diagnostic-bundle.zip");
        let events = vec![DiagnosticEvent {
            timestamp: diagnostic_timestamp(),
            sequence: 1,
            severity: "error".to_string(),
            code: concat!(
                "C:\\Users\\Alice\\study.qpls ",
                "alice@example.org ",
                "https://example.org/help?secret=yes ",
                "Bearer private-token"
            )
            .to_string(),
        }];
        let staging = build_diagnostic_staging(&events).unwrap();
        let inspected = inspect_diagnostic_staging(&staging.entries).unwrap();
        let archive_bytes = build_diagnostic_zip_bytes(&staging).unwrap();
        let expected_hash = sha256_hex(&archive_bytes);

        let saved = write_diagnostic_bundle(&path, &staging).unwrap();

        assert_eq!(saved.bytes as usize, archive_bytes.len());
        assert_eq!(saved.archive_sha256, expected_hash);
        assert_eq!(fs::read(&path).unwrap(), archive_bytes);
        assert!(archive_bytes.len() <= MAX_DIAGNOSTIC_ARCHIVE_BYTES);
        assert_eq!(inspected.events.len(), 1);
        let inspected_event = &inspected.events[0].code;
        assert!(inspected_event.contains("<redacted-path>/study.qpls"));
        assert!(inspected_event.contains("<redacted-email>"));
        assert!(inspected_event.contains("Bearer <redacted-token>"));
        assert!(!inspected_event.contains("secret=yes"));
        let mut archive = zip::ZipArchive::new(Cursor::new(&archive_bytes)).unwrap();
        assert_eq!(archive.len(), MAX_DIAGNOSTIC_ARCHIVE_ENTRIES);
        let mut contents = BTreeMap::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            assert!(entry.is_file());
            assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
            assert!(entry.size() as usize <= MAX_DIAGNOSTIC_ENTRY_BYTES);
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            contents.insert(name, bytes);
        }
        assert_eq!(
            contents.keys().cloned().collect::<Vec<_>>(),
            vec![
                DIAGNOSTIC_EVENTS_ENTRY.to_string(),
                DIAGNOSTIC_MANIFEST_ENTRY.to_string(),
                DIAGNOSTIC_SYSTEM_ENTRY.to_string(),
            ]
        );
        let combined = contents
            .values()
            .flat_map(|bytes| bytes.iter().copied())
            .collect::<Vec<_>>();
        let combined = String::from_utf8(combined).unwrap();
        for forbidden in [
            "C:\\Users\\Alice",
            "alice@example.org",
            "secret=yes",
            "private-token",
        ] {
            assert!(!combined.contains(forbidden), "found {forbidden}");
        }
        let manifest: Value = serde_json::from_slice(&contents[DIAGNOSTIC_MANIFEST_ENTRY]).unwrap();
        assert_eq!(manifest["schemaVersion"], 1);
        assert_eq!(manifest["policyVersion"], DIAGNOSTIC_POLICY_VERSION);
        assert_eq!(manifest["localOnly"], true);
        assert_eq!(manifest["networkAccessed"], false);
        let descriptors = manifest["entries"].as_array().unwrap();
        assert_eq!(descriptors.len(), 2);
        for descriptor in descriptors {
            let name = descriptor["name"].as_str().unwrap();
            let bytes = &contents[name];
            assert_eq!(descriptor["bytes"], bytes.len());
            assert_eq!(descriptor["sha256"], sha256_hex(bytes));
        }
    }

    #[test]
    fn diagnostic_bundle_requires_a_new_local_drive_rooted_zip_destination() {
        let directory = tempfile::tempdir().unwrap();
        let staging = build_diagnostic_staging(&[]).unwrap();
        let existing = directory.path().join("existing.zip");
        fs::write(&existing, b"keep me").unwrap();

        let relative = write_diagnostic_bundle(Path::new("relative.zip"), &staging).unwrap_err();
        assert!(relative.contains("DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE"));
        for blocked in [
            r"\\server\share\bundle.zip",
            r"//server/share/bundle.zip",
            r"\\?\C:\Support\bundle.zip",
            r"\\.\C:\Support\bundle.zip",
            r"\??\C:\Support\bundle.zip",
        ] {
            let error = validate_new_diagnostic_path(Path::new(blocked)).unwrap_err();
            assert!(
                error.contains("DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"),
                "{blocked}: {error}"
            );
        }
        let ads = format!("{}:private", directory.path().join("bundle.zip").display());
        assert!(
            validate_new_diagnostic_path(Path::new(&ads))
                .unwrap_err()
                .contains("DIAGNOSTIC_PATH_ADS_BLOCKED")
        );
        for reserved in [
            "NUL.zip",
            "CON.txt.zip",
            "COM1.zip",
            "LPT9.zip",
            "COM\u{00B9}.zip",
            "COM\u{00B2}.zip",
            "COM\u{00B3}.zip",
            "LPT\u{00B9}.zip",
            "LPT\u{00B2}.zip",
            "LPT\u{00B3}.zip",
        ] {
            let error = validate_new_diagnostic_path(&directory.path().join(reserved)).unwrap_err();
            assert!(error.contains("DIAGNOSTIC_DEVICE_NAME_BLOCKED"));
        }
        let wrong_extension =
            write_diagnostic_bundle(&directory.path().join("bundle.qpls"), &staging).unwrap_err();
        assert!(wrong_extension.contains("DIAGNOSTIC_EXTENSION_INVALID"));
        let overwrite = write_diagnostic_bundle(&existing, &staging).unwrap_err();
        assert!(overwrite.contains("DIAGNOSTIC_DESTINATION_EXISTS"));
        assert_eq!(fs::read(&existing).unwrap(), b"keep me");
        assert!(diagnostic_drive_type_is_local(3));
        assert!(!diagnostic_drive_type_is_local(2));
        assert!(!diagnostic_drive_type_is_local(6));
        assert!(!diagnostic_drive_type_is_local(4));
        assert!(diagnostic_component_is_reserved("COM\u{00B9}.zip"));
        assert!(diagnostic_component_is_reserved("LPT\u{00B3}.txt"));
        assert!(!diagnostic_component_is_reserved("COM\u{2074}.zip"));
    }

    #[test]
    fn diagnostic_destination_rejects_reparse_ancestors_when_available() {
        let directory = tempfile::tempdir().unwrap();
        let real = directory.path().join("real");
        let link = directory.path().join("linked");
        fs::create_dir(&real).unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&real, &link).is_ok();
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&real, &link).is_ok();
        #[cfg(not(any(windows, unix)))]
        let linked = false;
        if linked {
            let error = validate_new_diagnostic_path(&link.join("bundle.zip")).unwrap_err();
            assert!(error.contains("DIAGNOSTIC_REPARSE_POINT_BLOCKED"));
        }
    }

    #[test]
    fn diagnostic_create_new_closes_the_destination_toctou_window() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("raced.zip");
        let archive = build_diagnostic_zip_bytes(&build_diagnostic_staging(&[]).unwrap()).unwrap();

        let error = write_diagnostic_archive_bytes_with_hook(&path, &archive, || {
            fs::write(&path, b"created by another process").unwrap();
        })
        .unwrap_err();

        assert!(error.contains("DIAGNOSTIC_DESTINATION_CREATE_FAILED"));
        assert_eq!(fs::read(&path).unwrap(), b"created by another process");
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_destination_handle_is_exclusive_and_bound_to_the_verified_final_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exclusive.zip");
        let expected = expected_diagnostic_destination(&path).unwrap();
        let destination = open_new_diagnostic_destination(&path, || {}).unwrap();

        let resolved = diagnostic_path_from_handle(&destination.file).unwrap();
        assert_eq!(
            normalized_windows_diagnostic_path(&resolved).unwrap(),
            normalized_windows_diagnostic_path(&expected).unwrap()
        );
        assert!(destination.file.metadata().unwrap().file_type().is_file());
        assert!(!diagnostic_metadata_is_reparse(
            &destination.file.metadata().unwrap()
        ));
        assert!(fs::OpenOptions::new().read(true).open(&path).is_err());
        assert!(fs::rename(&path, directory.path().join("renamed.zip")).is_err());

        drop(destination);
        assert!(fs::OpenOptions::new().read(true).open(&path).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_parent_swap_never_receives_archive_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.path().join("selected");
        let moved = directory.path().join("moved");
        let alternate = directory.path().join("alternate");
        fs::create_dir(&selected).unwrap();
        fs::create_dir(&alternate).unwrap();
        let path = selected.join("guarded.zip");
        let archive = build_diagnostic_zip_bytes(&build_diagnostic_staging(&[]).unwrap()).unwrap();
        let swap_state = Arc::new(Mutex::new((false, false)));
        let observed = swap_state.clone();

        let result = write_diagnostic_archive_bytes_with_hook(&path, &archive, || {
            let renamed = fs::rename(&selected, &moved).is_ok();
            let linked =
                renamed && std::os::windows::fs::symlink_dir(&alternate, &selected).is_ok();
            *observed.lock().unwrap() = (renamed, linked);
        });

        let (renamed, linked) = *swap_state.lock().unwrap();
        if !renamed {
            assert!(result.is_ok());
            assert_eq!(fs::read(&path).unwrap(), archive);
            return;
        }
        let error = result.unwrap_err();
        assert!(
            error.contains("DIAGNOSTIC_DESTINATION_CREATE_FAILED")
                || error.contains("DIAGNOSTIC_FINAL_PATH_MISMATCH")
                || error.contains("DIAGNOSTIC_PARENT_GUARD_FAILED"),
            "{error}"
        );
        assert!(!moved.join("guarded.zip").exists());
        if linked && alternate.join("guarded.zip").exists() {
            assert_eq!(
                fs::metadata(alternate.join("guarded.zip")).unwrap().len(),
                0
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_parent_guard_detects_same_path_directory_replacement_after_open() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.path().join("selected");
        let moved = directory.path().join("moved");
        fs::create_dir(&selected).unwrap();
        let path = selected.join("replaced-parent.zip");
        let archive = build_diagnostic_zip_bytes(&build_diagnostic_staging(&[]).unwrap()).unwrap();

        let result = write_diagnostic_archive_bytes_with_hook(&path, &archive, || {
            fs::rename(&selected, &moved).unwrap();
            fs::create_dir(&selected).unwrap();
        });

        assert!(
            result
                .unwrap_err()
                .contains("DIAGNOSTIC_PARENT_GUARD_FAILED")
        );
        assert!(!moved.join("replaced-parent.zip").exists());
        assert_eq!(
            fs::metadata(selected.join("replaced-parent.zip"))
                .unwrap()
                .len(),
            0
        );
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_handle_verification_rejects_directories_and_resolved_link_targets() {
        let directory = tempfile::tempdir().unwrap();
        let parent = fs::canonicalize(directory.path()).unwrap();
        let parent_guard = open_diagnostic_parent_guard(&parent).unwrap();
        assert!(
            verify_open_diagnostic_file(&parent_guard, &parent)
                .unwrap_err()
                .contains("DIAGNOSTIC_DESTINATION_TYPE_BLOCKED")
        );

        let target = directory.path().join("target.zip");
        let link = directory.path().join("link.zip");
        fs::write(&target, b"target").unwrap();
        if std::os::windows::fs::symlink_file(&target, &link).is_ok() {
            let linked_file = fs::File::open(&link).unwrap();
            let expected_link = fs::canonicalize(directory.path()).unwrap().join("link.zip");
            assert!(
                verify_open_diagnostic_file(&linked_file, &expected_link)
                    .unwrap_err()
                    .contains("DIAGNOSTIC_FINAL_PATH_MISMATCH")
            );
        }
    }
}

#[test]
fn desktop_native_v11_workflow_smoke_import_run_save_reopen_and_export() {
    use qpls_project::{load_project, save_project};
    use std::fs;

    let directory = tempfile::tempdir().unwrap();
    let project_path = directory.path().join("v11-workflow.qpls");
    let xlsx_path = directory.path().join("v11-export.xlsx");

    let dataset = import_delimited_bytes(
        include_bytes!("../../validation/fixtures/simple_reflective.csv"),
        "simple_reflective.csv",
        b',',
        &ImportOptions::default(),
    )
    .unwrap();
    let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
        "../../validation/fixtures/simple_reflective.recipe.json"
    ))
    .unwrap();
    recipe = recipe.migrated_v3().unwrap();
    recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
    recipe.settings.workers = 1;
    recipe.settings.bootstrap_samples = 0;
    recipe.settings.permutation_samples = 0;

    let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
    let workspace = serde_json::json!({
        "activeDatasetId": dataset.id.to_string(),
        "analysisSettings": {"method": "pls_pm", "workers": 1, "bootstrapSamples": 0, "permutationSamples": 0},
        "nodes": [
            {"id": "x", "type": "construct", "position": {"x": 120, "y": 160}, "data": {"label": "X", "shortName": "X", "mode": "reflective", "indicators": ["x1", "x2"]}},
            {"id": "y", "type": "construct", "position": {"x": 420, "y": 160}, "data": {"label": "Y", "shortName": "Y", "mode": "reflective", "indicators": ["y1", "y2"]}}
        ],
        "edges": [
            {"id": "path-x-y", "source": "x", "target": "y", "type": "smoothstep", "label": "Path"}
        ],
        "runs": [{
            "id": result.id.to_string(),
            "name": "v1.1 native workflow run",
            "method": "PLS path modeling core",
            "createdAt": result.provenance.completed_at,
            "seed": result.provenance.seed,
            "status": "completed",
            "warnings": ["Validated for the documented QuickPLS v1.0.0 supported scope."],
            "fingerprint": result.provenance.dataset_fingerprint.chars().take(12).collect::<String>()
        }],
        "diagramLayout": {
            "constructLayouts": {
                "x": {"position": {"x": 120, "y": 160}, "pinned": true},
                "y": {"position": {"x": 420, "y": 160}, "pinned": true}
            }
        }
    });
    let mut project = Project::new("v1.1 native workflow smoke");
    project.datasets.push(dataset.clone());
    project.models.push(recipe.model.clone());
    project.recipes.push(recipe);
    project.results.push(result);
    project.layouts.insert("workspace".into(), workspace);

    save_project(&project_path, &project).unwrap();
    let reopened = load_project(&project_path).unwrap();
    assert_eq!(reopened.datasets.len(), 1);
    assert_eq!(reopened.models.len(), 1);
    assert_eq!(reopened.recipes.len(), 1);
    assert_eq!(reopened.results.len(), 1);
    assert_eq!(
        reopened.layouts["workspace"]["diagramLayout"]["constructLayouts"]["x"]["pinned"],
        true
    );

    let tables = vec![ExportTable {
        title: "v1.1 native workflow".into(),
        status: "validated".into(),
        warning: None,
        columns: vec!["Field".into(), "Value".into()],
        rows: vec![
            vec!["datasets".into(), reopened.datasets.len().to_string()],
            vec!["runs".into(), reopened.results.len().to_string()],
        ],
    }];
    write_xlsx_tables(&xlsx_path, &tables).unwrap();
    assert!(fs::metadata(&xlsx_path).unwrap().len() > 0);
}

#[tauri::command]
fn exit_desktop_application(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _webview2_offline_proxy = start_webview2_offline_rejection_proxy().unwrap_or_else(|error| {
        panic!(
            "QuickPLS refused to start because the fail-closed WebView2 offline proxy could not bind to {WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS}: {error}"
        )
    });
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopProject(Arc::new(Mutex::new(Project::new(
            "Untitled project",
        )))))
        .manage(DesktopGeneralSemFreshDraftAuthorityV1::default())
        .manage(DesktopSchema6NativeAdoptionAuthorityV1::default())
        .manage(DesktopJobs(Arc::new(Mutex::new(HashMap::new()))))
        .manage(DesktopDiagnostics::new())
        .manage(DesktopProjectUpgradePlans::default())
        .manage(DesktopRecipeV4Jobs::default())
        .manage(DesktopGeneralSemPlsJobsV1::default())
        .manage(DesktopCbsemGeneralSemJobsV1::default())
        .manage(DesktopPlsModelComparisonJobsV1::default())
        .invoke_handler(tauri::generate_handler![
            capability_registry_v2,
            start_internal_labs_pls_model_comparison_job,
            internal_labs_pls_model_comparison_job_status,
            cancel_internal_labs_pls_model_comparison_job,
            dismiss_internal_labs_pls_model_comparison_job,
            internal_labs_pls_model_comparison_job_result,
            run_internal_labs_recipe_v4_cbsem_execution,
            start_internal_labs_recipe_v4_cbsem_job,
            internal_labs_recipe_v4_cbsem_job_status,
            cancel_internal_labs_recipe_v4_cbsem_job,
            dismiss_internal_labs_recipe_v4_cbsem_job,
            internal_labs_recipe_v4_cbsem_job_result,
            internal_sem_model_v4_scientific_sha256,
            compare_and_swap_standard_sem_model_v4_authority,
            resolve_standard_sem_model_v4_authority,
            run_internal_labs_recipe_v4_pls_execution,
            start_internal_labs_recipe_v4_pls_job,
            internal_labs_recipe_v4_pls_job_status,
            cancel_internal_labs_recipe_v4_pls_job,
            dismiss_internal_labs_recipe_v4_pls_job,
            internal_labs_recipe_v4_pls_job_result,
            create_internal_general_sem_project_archive_v6,
            bootstrap_internal_general_sem_project_archive_v6,
            authorize_general_sem_revision_draft_v1,
            revise_internal_general_sem_execution_authority_v1,
            revise_internal_general_sem_execution_authority_v2,
            invalidate_general_sem_fresh_draft_authority_v1,
            preflight_internal_general_sem_estimators_v1,
            start_internal_labs_general_sem_pls_job_v1,
            status_internal_labs_general_sem_pls_job_v1,
            cancel_internal_labs_general_sem_pls_job_v1,
            dismiss_internal_labs_general_sem_pls_job_v1,
            result_internal_labs_general_sem_pls_job_v1,
            start_internal_labs_general_sem_cbsem_job_v1,
            status_internal_labs_general_sem_cbsem_job_v1,
            cancel_internal_labs_general_sem_cbsem_job_v1,
            dismiss_internal_labs_general_sem_cbsem_job_v1,
            result_internal_labs_general_sem_cbsem_job_v1,
            mutate_internal_project_archive_v6_model,
            adopt_internal_project_archive_v6_native_revision_source_v1,
            clear_internal_project_archive_v6_native_revision_source_v1,
            inspect_internal_project_archive_v6_zip,
            read_internal_project_archive_v6_dataset_rows,
            save_internal_project_archive_v6_copy,
            append_internal_project_schema6_canonical_result_v2,
            read_internal_project_schema6_canonical_results_v2,
            inspect_internal_project_upgrade_v6,
            plan_internal_project_upgrade_v6,
            execute_internal_project_upgrade_v6,
            cancel_internal_project_upgrade_v6,
            validate_analysis_recipe,
            method_capabilities,
            new_project,
            import_dataset,
            dataset_rows,
            profile_dataset_groups,
            import_validation_fixture,
            open_demo_project,
            set_column_metadata,
            recode_dataset_column,
            preview_dataset_transformation,
            apply_dataset_transformation,
            activate_dataset,
            publish_canonical_result_export_v2,
            export_xlsx_tables,
            export_text_file,
            open_default_export_folder,
            verify_latest_release_checksums,
            preview_diagnostic_bundle,
            cancel_diagnostic_bundle_preview,
            save_diagnostic_bundle,
            exit_desktop_application,
            open_project,
            save_active_project,
            autosave_active_project,
            mutate_project_explorer,
            start_analysis_job,
            analysis_job_status,
            cancel_analysis_job,
            dismiss_analysis_job,
            analysis_job_result,
            start_pls_job,
            pls_job_status,
            cancel_pls_job,
            dismiss_pls_job,
            pls_job_result
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuickPLS");
}
