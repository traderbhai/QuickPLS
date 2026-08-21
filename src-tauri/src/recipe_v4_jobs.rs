use crate::recipe_v4_canonical_result::build_recipe_v4_pls_canonical_result;
use crate::recipe_v4_cbsem_canonical_result::build_recipe_v4_cbsem_canonical_result;
use crate::recipe_v4_cbsem_execution::{
    InternalRecipeV4CbsemExecutionRequestV1, execute_internal_recipe_v4_cbsem_with_control,
    resolve_internal_recipe_v4_cbsem_dataset, validate_internal_recipe_v4_cbsem_access,
};
use crate::{
    DesktopJob, DesktopJobs, DesktopProject, InternalRecipeV4ExecutionFailureV1,
    InternalRecipeV4ExecutionStageV1, InternalRecipeV4PlsExecutionRequestV1,
    execute_internal_recipe_v4_pls_with_control, internal_recipe_v4_failure,
    resolve_internal_recipe_v4_dataset, validate_internal_recipe_v4_pls_access,
};
use chrono::{SecondsFormat, Utc};
use qpls_core::CanonicalResultDocumentV2;
#[cfg(test)]
use qpls_core::validate_canonical_result_document_v2;
use qpls_project::Project;
use qpls_runner::RecipeV4CbsemExecutionResultV1;
use qpls_runner::RecipeV4PlsExecutionResultV1;
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

const INTERNAL_RECIPE_V4_JOB_SCHEMA_VERSION: u32 = 1;
const MAXIMUM_ACTIVE_INTERNAL_RECIPE_V4_JOBS: usize = 4;
const MAXIMUM_RETAINED_INTERNAL_RECIPE_V4_JOBS: usize = 255;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InternalRecipeV4JobStateV1 {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl InternalRecipeV4JobStateV1 {
    fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Cancelling)
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalRecipeV4JobSnapshotV1 {
    schema_version: u32,
    job_id: Uuid,
    state: InternalRecipeV4JobStateV1,
    phase: String,
    completed_units: u64,
    total_units: u64,
    message: Option<String>,
    failure: Option<InternalRecipeV4ExecutionFailureV1>,
    queued_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

impl InternalRecipeV4JobSnapshotV1 {
    fn queued() -> Self {
        Self {
            schema_version: INTERNAL_RECIPE_V4_JOB_SCHEMA_VERSION,
            job_id: Uuid::new_v4(),
            state: InternalRecipeV4JobStateV1::Queued,
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

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

struct InternalRecipeV4Job {
    snapshot: InternalRecipeV4JobSnapshotV1,
    kind: InternalRecipeV4JobKindV1,
    worker_demand: usize,
    cancellation: Arc<AtomicBool>,
    result: Option<InternalRecipeV4StoredResultV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalRecipeV4JobKindV1 {
    Pls,
    Cbsem,
    /// Admission-only marker owned by the isolated genuine PLS
    /// model-comparison job service. No result is stored in this map.
    PlsModelComparisonReservation,
}

enum InternalRecipeV4StoredResultV1 {
    Pls(InternalRecipeV4CompletedResultV1),
    Cbsem(InternalRecipeV4CbsemCompletedResultV1),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalRecipeV4CompletedResultV1 {
    schema_version: u32,
    analytical_result: RecipeV4PlsExecutionResultV1,
    canonical_document: CanonicalResultDocumentV2,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalRecipeV4CbsemCompletedResultV1 {
    schema_version: u32,
    analytical_result: RecipeV4CbsemExecutionResultV1,
    canonical_document: CanonicalResultDocumentV2,
}

#[derive(Clone, Default)]
pub(crate) struct DesktopRecipeV4Jobs(Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>);

impl DesktopRecipeV4Jobs {
    pub(crate) fn active_summary(&self) -> Result<(usize, usize), String> {
        let jobs = self
            .0
            .lock()
            .map_err(|_| "internal recipe-v4 job state is unavailable".to_owned())?;
        let active_count = jobs
            .values()
            .filter(|job| job.snapshot.state.is_active())
            .count();
        let worker_demand = jobs
            .values()
            .filter(|job| job.snapshot.state.is_active())
            .map(|job| job.worker_demand)
            .sum();
        Ok((active_count, worker_demand))
    }
}

/// RAII reservation in the existing Standard/Recipe-v4 admission pool. The
/// comparison job service moves this guard into its worker; every return,
/// cancellation, caught panic, and spawn failure therefore releases the
/// charged active-job and configured worker demand.
pub(crate) struct PlsModelComparisonAdmissionReservationV1 {
    job_id: Uuid,
    jobs: Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>,
}

impl std::fmt::Debug for PlsModelComparisonAdmissionReservationV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlsModelComparisonAdmissionReservationV1")
            .field("job_id", &self.job_id)
            .finish_non_exhaustive()
    }
}

impl Drop for PlsModelComparisonAdmissionReservationV1 {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.jobs.lock()
            && jobs.get(&self.job_id).is_some_and(|job| {
                job.kind == InternalRecipeV4JobKindV1::PlsModelComparisonReservation
            })
        {
            jobs.remove(&self.job_id);
        }
    }
}

pub(crate) fn reserve_pls_model_comparison_admission(
    job_id: Uuid,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    job_state: DesktopRecipeV4Jobs,
) -> Result<PlsModelComparisonAdmissionReservationV1, InternalRecipeV4ExecutionFailureV1> {
    let cpu_budget = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    reserve_pls_model_comparison_admission_with_cpu_budget(
        job_id,
        standard_jobs,
        job_state,
        cpu_budget,
    )
}

/// Reserves capacity in the shared Standard/Recipe-v4 admission pool for an
/// isolated General SEM job. The returned RAII guard must live for the worker
/// lifetime so cancellation, panic, and every terminal return release the
/// charged worker demand.
pub(crate) fn reserve_general_sem_pls_admission(
    job_id: Uuid,
    worker_demand: usize,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    job_state: DesktopRecipeV4Jobs,
) -> Result<PlsModelComparisonAdmissionReservationV1, InternalRecipeV4ExecutionFailureV1> {
    let cpu_budget = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    reserve_internal_recipe_v4_admission_with_cpu_budget(
        job_id,
        standard_jobs,
        job_state,
        cpu_budget,
        worker_demand,
        "general_sem_pls",
        "General SEM PLS analysis",
        "Wait for another analysis to finish or reduce the General SEM recipe worker count.",
    )
}

fn reserve_pls_model_comparison_admission_with_cpu_budget(
    job_id: Uuid,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    job_state: DesktopRecipeV4Jobs,
    cpu_budget: usize,
) -> Result<PlsModelComparisonAdmissionReservationV1, InternalRecipeV4ExecutionFailureV1> {
    reserve_internal_recipe_v4_admission_with_cpu_budget(
        job_id,
        standard_jobs,
        job_state,
        cpu_budget,
        1,
        "pls_model_comparison",
        "PLS model comparison",
        "Wait for another analysis to finish before starting this comparison.",
    )
}

#[allow(clippy::too_many_arguments)]
fn reserve_internal_recipe_v4_admission_with_cpu_budget(
    job_id: Uuid,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    job_state: DesktopRecipeV4Jobs,
    cpu_budget: usize,
    worker_demand: usize,
    code_prefix: &str,
    analysis_label: &str,
    worker_corrective_action: &str,
) -> Result<PlsModelComparisonAdmissionReservationV1, InternalRecipeV4ExecutionFailureV1> {
    if worker_demand == 0 {
        return Err(job_failure(
            "workers",
            format!("{code_prefix}.worker_demand_invalid"),
            format!("{analysis_label} requires at least one worker."),
            worker_corrective_action,
        ));
    }
    let jobs = job_state.0;
    let standard_guard = standard_jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            format!("{code_prefix}.shared_job_state_unavailable"),
            "The shared analysis job state is temporarily unavailable.",
            "Retry after current analyses finish.",
        )
    })?;
    let mut guard = jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            format!("{code_prefix}.internal_job_state_unavailable"),
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    prune_terminal_jobs(&mut guard);
    if guard.contains_key(&job_id) {
        return Err(job_failure(
            "jobId",
            format!("{code_prefix}.duplicate_job_id"),
            format!("Internal analysis job ID {job_id} is already reserved."),
            "Create a new comparison job request and retry.",
        ));
    }
    let standard_active_count = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .count();
    let internal_active_count = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .count();
    if standard_active_count + internal_active_count >= MAXIMUM_ACTIVE_INTERNAL_RECIPE_V4_JOBS {
        return Err(job_failure(
            "jobs",
            format!("{code_prefix}.active_job_limit_reached"),
            "Four analyses are already active across Standard and internal execution.",
            "Wait for one analysis to finish, or cancel it, before starting another.",
        ));
    }
    let internal_worker_demand = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .map(|job| job.worker_demand)
        .sum::<usize>();
    let allocated_workers = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .map(|job| job.worker_demand)
        .sum::<usize>()
        + internal_worker_demand;
    if allocated_workers.saturating_add(worker_demand) > cpu_budget {
        return Err(job_failure(
            "jobs",
            format!("{code_prefix}.worker_budget_unavailable"),
            format!(
                "{analysis_label} requires {worker_demand} worker(s), but only {} of {cpu_budget} are available.",
                cpu_budget.saturating_sub(allocated_workers)
            ),
            worker_corrective_action,
        ));
    }
    let mut snapshot = InternalRecipeV4JobSnapshotV1::queued();
    snapshot.job_id = job_id;
    guard.insert(
        job_id,
        InternalRecipeV4Job {
            snapshot,
            kind: InternalRecipeV4JobKindV1::PlsModelComparisonReservation,
            worker_demand,
            cancellation: Arc::new(AtomicBool::new(false)),
            result: None,
        },
    );
    drop(guard);
    drop(standard_guard);
    Ok(PlsModelComparisonAdmissionReservationV1 { job_id, jobs })
}

fn job_failure(
    subject: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> InternalRecipeV4ExecutionFailureV1 {
    internal_recipe_v4_failure(
        InternalRecipeV4ExecutionStageV1::Integrity,
        subject,
        code,
        message,
        corrective_action,
    )
}

fn ensure_job_kind(
    job: &InternalRecipeV4Job,
    expected: InternalRecipeV4JobKindV1,
    job_id: Uuid,
) -> Result<(), InternalRecipeV4ExecutionFailureV1> {
    if job.kind == expected {
        Ok(())
    } else {
        Err(job_failure(
            "jobId",
            "recipe_v4.job_kind_mismatch",
            format!("Internal Recipe-v4 job {job_id} belongs to a different estimator."),
            "Use the status, cancellation, dismissal, or result command for the estimator that created this job.",
        ))
    }
}

fn set_running(
    jobs: &Mutex<HashMap<Uuid, InternalRecipeV4Job>>,
    job_id: Uuid,
    phase: &str,
    completed_units: u64,
    total_units: u64,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == InternalRecipeV4JobStateV1::Queued {
            job.snapshot.state = InternalRecipeV4JobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        if matches!(
            job.snapshot.state,
            InternalRecipeV4JobStateV1::Running | InternalRecipeV4JobStateV1::Cancelling
        ) {
            job.snapshot.phase = phase.into();
            job.snapshot.completed_units = completed_units.min(total_units);
            job.snapshot.total_units = total_units.max(1);
        }
    }
}

fn finish_cancelled(jobs: &Mutex<HashMap<Uuid, InternalRecipeV4Job>>, job_id: Uuid) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalRecipeV4JobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn finish_failed(
    jobs: &Mutex<HashMap<Uuid, InternalRecipeV4Job>>,
    job_id: Uuid,
    failure: InternalRecipeV4ExecutionFailureV1,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalRecipeV4JobStateV1::Failed;
        job.snapshot.phase = "failed".into();
        job.snapshot.message = Some(failure.message.clone());
        job.snapshot.failure = Some(failure);
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn publish_completed_result(
    project: &Mutex<Project>,
    jobs: &Mutex<HashMap<Uuid, InternalRecipeV4Job>>,
    expected_project_id: Uuid,
    expected_dataset_id: &str,
    expected_dataset_fingerprint: &str,
    job_id: Uuid,
    request: &InternalRecipeV4PlsExecutionRequestV1,
    result: RecipeV4PlsExecutionResultV1,
) {
    let project = match project.lock() {
        Ok(project) => project,
        Err(_) => {
            finish_failed(
                jobs,
                job_id,
                job_failure(
                    "project",
                    "recipe_v4.project_state_unavailable_at_completion",
                    "The active project could not be rechecked before publishing the completed result.",
                    "Retry after the active project finishes its current operation.",
                ),
            );
            return;
        }
    };
    let project_is_exact = project_contains_exact_dataset(
        &project,
        expected_project_id,
        expected_dataset_id,
        expected_dataset_fingerprint,
    );
    if !project_is_exact {
        drop(project);
        finish_failed(
            jobs,
            job_id,
            job_failure(
                "project",
                "recipe_v4.active_project_changed",
                "The active project or resident dataset changed while the internal analysis was running.",
                "Discard this job, rebuild the request from the current project, and run it again.",
            ),
        );
        return;
    }

    drop(project);

    let (started_at, completed_at) = {
        let guard = match jobs.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let Some(job) = guard.get(&job_id) else {
            return;
        };
        if job.cancellation.load(Ordering::Acquire)
            || job.snapshot.state == InternalRecipeV4JobStateV1::Cancelling
        {
            drop(guard);
            finish_cancelled(jobs, job_id);
            return;
        }
        (
            job.snapshot
                .started_at
                .clone()
                .unwrap_or_else(|| job.snapshot.queued_at.clone()),
            now_utc(),
        )
    };
    let canonical_document = match build_recipe_v4_pls_canonical_result(
        job_id,
        expected_project_id,
        &started_at,
        &completed_at,
        request,
        &result,
    ) {
        Ok(document) => document,
        Err(errors) => {
            finish_failed(
                jobs,
                job_id,
                job_failure(
                    "result",
                    "recipe_v4.canonical_result_invalid",
                    format!(
                        "The completed estimate could not be represented as a canonical result: {}",
                        errors.join("; ")
                    ),
                    "Discard this job and correct the reported canonical result contract error before retrying.",
                ),
            );
            return;
        }
    };

    let mut jobs = match jobs.lock() {
        Ok(jobs) => jobs,
        Err(_) => return,
    };
    let Some(job) = jobs.get_mut(&job_id) else {
        return;
    };
    if job.cancellation.load(Ordering::Acquire)
        || job.snapshot.state == InternalRecipeV4JobStateV1::Cancelling
    {
        job.result = None;
        job.snapshot.state = InternalRecipeV4JobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
        return;
    }
    job.result = Some(InternalRecipeV4StoredResultV1::Pls(
        InternalRecipeV4CompletedResultV1 {
            schema_version: INTERNAL_RECIPE_V4_JOB_SCHEMA_VERSION,
            analytical_result: result,
            canonical_document,
        },
    ));
    job.snapshot.state = InternalRecipeV4JobStateV1::Completed;
    job.snapshot.phase = "completed".into();
    job.snapshot.completed_units = job.snapshot.total_units;
    job.snapshot.message = None;
    job.snapshot.failure = None;
    job.snapshot.completed_at = Some(completed_at);
}

fn project_contains_exact_dataset(
    project: &Project,
    expected_project_id: Uuid,
    expected_dataset_id: &str,
    expected_dataset_fingerprint: &str,
) -> bool {
    project.manifest.project_id == expected_project_id
        && project.datasets.iter().any(|dataset| {
            dataset.id.to_string() == expected_dataset_id
                && dataset.fingerprint.0 == expected_dataset_fingerprint
        })
}

fn run_job_worker(
    project: Arc<Mutex<Project>>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>,
    job_id: Uuid,
    expected_project_id: Uuid,
    dataset: qpls_data::Dataset,
    request: InternalRecipeV4PlsExecutionRequestV1,
) {
    let cancellation = match jobs.lock() {
        Ok(jobs) => match jobs.get(&job_id) {
            Some(job) => job.cancellation.clone(),
            None => return,
        },
        Err(_) => return,
    };
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    set_running(&jobs, job_id, "compilation", 0, 1);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_internal_recipe_v4_pls_with_control(
            &dataset,
            &request,
            || cancellation.load(Ordering::Acquire),
            |progress| {
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

    match outcome {
        Ok(Ok(result)) => publish_completed_result(
            &project,
            &jobs,
            expected_project_id,
            &request.dataset_id,
            &request.dataset_fingerprint,
            job_id,
            &request,
            result,
        ),
        Ok(Err(failure)) if failure.code == "recipe_v4.execution_cancelled" => {
            finish_cancelled(&jobs, job_id)
        }
        Ok(Err(failure)) => finish_failed(&jobs, job_id, failure),
        Err(_) => finish_failed(
            &jobs,
            job_id,
            job_failure(
                "execution",
                "recipe_v4.worker_terminated_unexpectedly",
                "The internal recipe-v4 worker terminated unexpectedly.",
                "Discard this job and retry. If the problem repeats, export a diagnostic bundle.",
            ),
        ),
    }
}

fn publish_completed_cbsem_result(
    project: &Mutex<Project>,
    jobs: &Mutex<HashMap<Uuid, InternalRecipeV4Job>>,
    expected_project_id: Uuid,
    expected_dataset_id: &str,
    expected_dataset_fingerprint: &str,
    job_id: Uuid,
    request: &InternalRecipeV4CbsemExecutionRequestV1,
    result: RecipeV4CbsemExecutionResultV1,
) {
    let project = match project.lock() {
        Ok(project) => project,
        Err(_) => {
            finish_failed(
                jobs,
                job_id,
                job_failure(
                    "project",
                    "recipe_v4.cbsem.project_state_unavailable_at_completion",
                    "The active project could not be rechecked before publishing the completed CB-SEM result.",
                    "Retry after the active project finishes its current operation.",
                ),
            );
            return;
        }
    };
    if !project_contains_exact_dataset(
        &project,
        expected_project_id,
        expected_dataset_id,
        expected_dataset_fingerprint,
    ) {
        drop(project);
        finish_failed(
            jobs,
            job_id,
            job_failure(
                "project",
                "recipe_v4.cbsem.active_project_changed",
                "The active project or resident dataset changed while CB-SEM was running.",
                "Discard this job, rebuild the request from the current project, and run it again.",
            ),
        );
        return;
    }
    drop(project);

    let (started_at, completed_at) = {
        let guard = match jobs.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let Some(job) = guard.get(&job_id) else {
            return;
        };
        if job.kind != InternalRecipeV4JobKindV1::Cbsem {
            drop(guard);
            finish_failed(
                jobs,
                job_id,
                job_failure(
                    "jobId",
                    "recipe_v4.job_kind_mismatch",
                    "The CB-SEM worker found a job allocated to another estimator.",
                    "Discard the job and start the CB-SEM analysis again.",
                ),
            );
            return;
        }
        if job.cancellation.load(Ordering::Acquire)
            || job.snapshot.state == InternalRecipeV4JobStateV1::Cancelling
        {
            drop(guard);
            finish_cancelled(jobs, job_id);
            return;
        }
        (
            job.snapshot
                .started_at
                .clone()
                .unwrap_or_else(|| job.snapshot.queued_at.clone()),
            now_utc(),
        )
    };
    let canonical_document = match build_recipe_v4_cbsem_canonical_result(
        job_id,
        expected_project_id,
        &started_at,
        &completed_at,
        request,
        &result,
    ) {
        Ok(document) => document,
        Err(errors) => {
            finish_failed(
                jobs,
                job_id,
                job_failure(
                    "result",
                    "recipe_v4.cbsem.canonical_result_invalid",
                    format!(
                        "The completed CB-SEM estimate could not be represented as a canonical result: {}",
                        errors.join("; ")
                    ),
                    "Discard this job and correct the reported canonical result contract error before retrying.",
                ),
            );
            return;
        }
    };

    let mut jobs = match jobs.lock() {
        Ok(jobs) => jobs,
        Err(_) => return,
    };
    let Some(job) = jobs.get_mut(&job_id) else {
        return;
    };
    if job.kind != InternalRecipeV4JobKindV1::Cbsem {
        job.result = None;
        job.snapshot.state = InternalRecipeV4JobStateV1::Failed;
        job.snapshot.phase = "failed".into();
        job.snapshot.failure = Some(job_failure(
            "jobId",
            "recipe_v4.job_kind_mismatch",
            "The CB-SEM worker found a job allocated to another estimator.",
            "Discard the job and start the CB-SEM analysis again.",
        ));
        job.snapshot.completed_at = Some(now_utc());
        return;
    }
    if job.cancellation.load(Ordering::Acquire)
        || job.snapshot.state == InternalRecipeV4JobStateV1::Cancelling
    {
        job.result = None;
        job.snapshot.state = InternalRecipeV4JobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
        return;
    }
    job.result = Some(InternalRecipeV4StoredResultV1::Cbsem(
        InternalRecipeV4CbsemCompletedResultV1 {
            schema_version: INTERNAL_RECIPE_V4_JOB_SCHEMA_VERSION,
            analytical_result: result,
            canonical_document,
        },
    ));
    job.snapshot.state = InternalRecipeV4JobStateV1::Completed;
    job.snapshot.phase = "completed".into();
    job.snapshot.completed_units = job.snapshot.total_units;
    job.snapshot.message = None;
    job.snapshot.failure = None;
    job.snapshot.completed_at = Some(completed_at);
}

fn run_cbsem_job_worker(
    project: Arc<Mutex<Project>>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>,
    job_id: Uuid,
    expected_project_id: Uuid,
    dataset: qpls_data::Dataset,
    request: InternalRecipeV4CbsemExecutionRequestV1,
) {
    let cancellation = match jobs.lock() {
        Ok(jobs) => match jobs.get(&job_id) {
            Some(job) if job.kind == InternalRecipeV4JobKindV1::Cbsem => job.cancellation.clone(),
            _ => return,
        },
        Err(_) => return,
    };
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    set_running(&jobs, job_id, "compilation", 0, 1);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_internal_recipe_v4_cbsem_with_control(
            &dataset,
            &request,
            || cancellation.load(Ordering::Acquire),
            |progress| {
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
    match outcome {
        Ok(Ok(result)) => publish_completed_cbsem_result(
            &project,
            &jobs,
            expected_project_id,
            &request.dataset_id,
            &request.dataset_fingerprint,
            job_id,
            &request,
            result,
        ),
        Ok(Err(failure)) if failure.code == "recipe_v4.cbsem.execution_cancelled" => {
            finish_cancelled(&jobs, job_id)
        }
        Ok(Err(failure)) => finish_failed(&jobs, job_id, failure),
        Err(_) => finish_failed(
            &jobs,
            job_id,
            job_failure(
                "execution",
                "recipe_v4.cbsem.worker_terminated_unexpectedly",
                "The internal CB-SEM Recipe-v4 worker terminated unexpectedly.",
                "Discard this job and retry. If the problem repeats, export a diagnostic bundle.",
            ),
        ),
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, InternalRecipeV4Job>) {
    if jobs.len() <= MAXIMUM_RETAINED_INTERNAL_RECIPE_V4_JOBS {
        return;
    }
    let removable = jobs
        .iter()
        .filter_map(|(id, job)| job.snapshot.state.is_terminal().then_some(*id))
        .take(jobs.len() - MAXIMUM_RETAINED_INTERNAL_RECIPE_V4_JOBS)
        .collect::<Vec<_>>();
    for id in removable {
        jobs.remove(&id);
    }
}

fn start_job(
    request: InternalRecipeV4PlsExecutionRequestV1,
    project: Arc<Mutex<Project>>,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_pls_access(&request)?;
    let (dataset, project_id) = {
        let project = project.lock().map_err(|_| {
            job_failure(
                "project",
                "recipe_v4.project_state_unavailable",
                "The active project data is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?;
        (
            resolve_internal_recipe_v4_dataset(&project, &request)?,
            project.manifest.project_id,
        )
    };

    let snapshot = InternalRecipeV4JobSnapshotV1::queued();
    let cancellation = Arc::new(AtomicBool::new(false));
    let standard_guard = standard_jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.shared_job_state_unavailable",
            "The shared analysis job state is temporarily unavailable.",
            "Retry after current analyses finish.",
        )
    })?;
    let mut guard = jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    prune_terminal_jobs(&mut guard);
    let standard_active_count = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .count();
    let internal_active_count = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .count();
    if standard_active_count + internal_active_count >= MAXIMUM_ACTIVE_INTERNAL_RECIPE_V4_JOBS {
        return Err(job_failure(
            "jobs",
            "recipe_v4.active_job_limit_reached",
            "Four analyses are already active across Standard and internal execution.",
            "Wait for one analysis to finish, or cancel it, before starting another.",
        ));
    }
    let internal_worker_demand = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .map(|job| job.worker_demand)
        .sum::<usize>();
    let allocated_workers = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .map(|job| job.worker_demand)
        .sum::<usize>()
        + internal_worker_demand;
    // Preserve the existing Standard/internal Recipe V4 admission behavior:
    // these legacy job types occupy one shared worker slot. General SEM jobs
    // use the explicit reservation helper below to charge their configured
    // worker demand without changing this established path.
    let worker_demand = 1;
    let cpu_budget = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    if allocated_workers.saturating_add(worker_demand) > cpu_budget {
        return Err(job_failure(
            "jobs",
            "recipe_v4.worker_budget_unavailable",
            format!(
                "The internal analysis requires {worker_demand} worker(s), but only {} of {cpu_budget} are available.",
                cpu_budget.saturating_sub(allocated_workers)
            ),
            "Wait for another analysis to finish or reduce its configured worker count.",
        ));
    }
    guard.insert(
        snapshot.job_id,
        InternalRecipeV4Job {
            snapshot: snapshot.clone(),
            kind: InternalRecipeV4JobKindV1::Pls,
            worker_demand,
            cancellation,
            result: None,
        },
    );
    drop(guard);
    drop(standard_guard);

    let job_id = snapshot.job_id;
    std::thread::spawn(move || run_job_worker(project, jobs, job_id, project_id, dataset, request));
    Ok(snapshot)
}

fn start_cbsem_job(
    request: InternalRecipeV4CbsemExecutionRequestV1,
    project: Arc<Mutex<Project>>,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalRecipeV4Job>>>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_cbsem_access(&request)?;
    let (dataset, project_id) = {
        let project = project.lock().map_err(|_| {
            job_failure(
                "project",
                "recipe_v4.cbsem.project_state_unavailable",
                "The active project data is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?;
        (
            resolve_internal_recipe_v4_cbsem_dataset(&project, &request)?,
            project.manifest.project_id,
        )
    };

    let snapshot = InternalRecipeV4JobSnapshotV1::queued();
    let cancellation = Arc::new(AtomicBool::new(false));
    let standard_guard = standard_jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.shared_job_state_unavailable",
            "The shared analysis job state is temporarily unavailable.",
            "Retry after current analyses finish.",
        )
    })?;
    let mut guard = jobs.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    prune_terminal_jobs(&mut guard);
    let standard_active_count = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .count();
    let internal_active_count = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .count();
    if standard_active_count + internal_active_count >= MAXIMUM_ACTIVE_INTERNAL_RECIPE_V4_JOBS {
        return Err(job_failure(
            "jobs",
            "recipe_v4.active_job_limit_reached",
            "Four analyses are already active across Standard and internal execution.",
            "Wait for one analysis to finish, or cancel it, before starting another.",
        ));
    }
    let internal_worker_demand = guard
        .values()
        .filter(|job| job.snapshot.state.is_active())
        .map(|job| job.worker_demand)
        .sum::<usize>();
    let allocated_workers = standard_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                qpls_core::JobState::Queued
                    | qpls_core::JobState::Running
                    | qpls_core::JobState::Cancelling
                    | qpls_core::JobState::Committing
            )
        })
        .map(|job| job.worker_demand)
        .sum::<usize>()
        + internal_worker_demand;
    // Preserve the existing Standard/internal Recipe V4 admission behavior;
    // General SEM uses its dedicated reservation to charge configured demand.
    let worker_demand = 1;
    let cpu_budget = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    if allocated_workers.saturating_add(worker_demand) > cpu_budget {
        return Err(job_failure(
            "jobs",
            "recipe_v4.worker_budget_unavailable",
            format!(
                "The internal CB-SEM analysis requires {worker_demand} worker(s), but only {} of {cpu_budget} are available.",
                cpu_budget.saturating_sub(allocated_workers)
            ),
            "Wait for another analysis to finish or reduce its configured worker count.",
        ));
    }
    guard.insert(
        snapshot.job_id,
        InternalRecipeV4Job {
            snapshot: snapshot.clone(),
            kind: InternalRecipeV4JobKindV1::Cbsem,
            worker_demand,
            cancellation,
            result: None,
        },
    );
    drop(guard);
    drop(standard_guard);

    let job_id = snapshot.job_id;
    std::thread::spawn(move || {
        run_cbsem_job_worker(project, jobs, job_id, project_id, dataset, request)
    });
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn start_internal_labs_recipe_v4_pls_job(
    request: InternalRecipeV4PlsExecutionRequestV1,
    project: State<'_, DesktopProject>,
    standard_jobs: State<'_, DesktopJobs>,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    start_job(
        request,
        project.0.clone(),
        standard_jobs.0.clone(),
        jobs.0.clone(),
    )
}

#[tauri::command]
pub(crate) fn start_internal_labs_recipe_v4_cbsem_job(
    request: InternalRecipeV4CbsemExecutionRequestV1,
    project: State<'_, DesktopProject>,
    standard_jobs: State<'_, DesktopJobs>,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    start_cbsem_job(
        request,
        project.0.clone(),
        standard_jobs.0.clone(),
        jobs.0.clone(),
    )
}

#[tauri::command]
pub(crate) fn internal_labs_recipe_v4_pls_job_status(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    let jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Pls, job_id)?;
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn cancel_internal_labs_recipe_v4_pls_job(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Pls, job_id)?;
    if matches!(
        job.snapshot.state,
        InternalRecipeV4JobStateV1::Queued | InternalRecipeV4JobStateV1::Running
    ) {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = InternalRecipeV4JobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn dismiss_internal_labs_recipe_v4_pls_job(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<(), InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let terminal = jobs
        .get(&job_id)
        .map(|job| -> Result<bool, InternalRecipeV4ExecutionFailureV1> {
            ensure_job_kind(job, InternalRecipeV4JobKindV1::Pls, job_id)?;
            Ok(job.snapshot.state.is_terminal())
        })
        .transpose()?
        .ok_or_else(|| {
            job_failure(
                "jobId",
                "recipe_v4.unknown_job",
                format!("No internal recipe-v4 job exists with ID {job_id}."),
                "Refresh the internal job list and select an existing job.",
            )
        })?;
    if !terminal {
        return Err(job_failure(
            "jobId",
            "recipe_v4.active_job_cannot_be_dismissed",
            "An active internal recipe-v4 job cannot be dismissed.",
            "Wait for completion or cancellation before dismissing this job.",
        ));
    }
    jobs.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn internal_labs_recipe_v4_pls_job_result(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4CompletedResultV1, InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Pls, job_id)?;
    if job.snapshot.state != InternalRecipeV4JobStateV1::Completed {
        return Err(job_failure(
            "jobId",
            "recipe_v4.result_not_available",
            "An internal recipe-v4 result is available only after successful completion.",
            "Wait for successful completion, or inspect the typed terminal failure.",
        ));
    }
    let mut job = jobs.remove(&job_id).expect("job existence was checked");
    let result = job.result.take().ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.completed_result_missing",
            "The completed internal job did not retain its result.",
            "Discard the job and run the analysis again.",
        )
    })?;
    match result {
        InternalRecipeV4StoredResultV1::Pls(result) => Ok(result),
        InternalRecipeV4StoredResultV1::Cbsem(_) => Err(job_failure(
            "jobId",
            "recipe_v4.job_kind_mismatch",
            "The completed job retained a CB-SEM result instead of a PLS result.",
            "Discard the job and use the CB-SEM result command for the originating job.",
        )),
    }
}

#[tauri::command]
pub(crate) fn internal_labs_recipe_v4_cbsem_job_status(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    let jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal Recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Cbsem, job_id)?;
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn cancel_internal_labs_recipe_v4_cbsem_job(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4JobSnapshotV1, InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal Recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Cbsem, job_id)?;
    if matches!(
        job.snapshot.state,
        InternalRecipeV4JobStateV1::Queued | InternalRecipeV4JobStateV1::Running
    ) {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = InternalRecipeV4JobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn dismiss_internal_labs_recipe_v4_cbsem_job(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<(), InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let terminal = jobs
        .get(&job_id)
        .map(|job| -> Result<bool, InternalRecipeV4ExecutionFailureV1> {
            ensure_job_kind(job, InternalRecipeV4JobKindV1::Cbsem, job_id)?;
            Ok(job.snapshot.state.is_terminal())
        })
        .transpose()?
        .ok_or_else(|| {
            job_failure(
                "jobId",
                "recipe_v4.unknown_job",
                format!("No internal Recipe-v4 job exists with ID {job_id}."),
                "Refresh the internal job list and select an existing job.",
            )
        })?;
    if !terminal {
        return Err(job_failure(
            "jobId",
            "recipe_v4.active_job_cannot_be_dismissed",
            "An active internal Recipe-v4 job cannot be dismissed.",
            "Wait for completion or cancellation before dismissing this job.",
        ));
    }
    jobs.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn internal_labs_recipe_v4_cbsem_job_result(
    job_id: Uuid,
    jobs: State<'_, DesktopRecipeV4Jobs>,
) -> Result<InternalRecipeV4CbsemCompletedResultV1, InternalRecipeV4ExecutionFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        job_failure(
            "jobs",
            "recipe_v4.cbsem.job_state_unavailable",
            "The internal analysis job state is temporarily unavailable.",
            "Retry after current internal analyses finish.",
        )
    })?;
    let job = jobs.get(&job_id).ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.unknown_job",
            format!("No internal Recipe-v4 job exists with ID {job_id}."),
            "Refresh the internal job list and select an existing job.",
        )
    })?;
    ensure_job_kind(job, InternalRecipeV4JobKindV1::Cbsem, job_id)?;
    if job.snapshot.state != InternalRecipeV4JobStateV1::Completed {
        return Err(job_failure(
            "jobId",
            "recipe_v4.result_not_available",
            "An internal Recipe-v4 result is available only after successful completion.",
            "Wait for successful completion, or inspect the typed terminal failure.",
        ));
    }
    let mut job = jobs.remove(&job_id).expect("job existence was checked");
    let result = job.result.take().ok_or_else(|| {
        job_failure(
            "jobId",
            "recipe_v4.completed_result_missing",
            "The completed internal CB-SEM job did not retain its result.",
            "Discard the job and run the analysis again.",
        )
    })?;
    match result {
        InternalRecipeV4StoredResultV1::Cbsem(result) => Ok(result),
        InternalRecipeV4StoredResultV1::Pls(_) => Err(job_failure(
            "jobId",
            "recipe_v4.job_kind_mismatch",
            "The completed job retained a PLS result instead of a CB-SEM result.",
            "Discard the job and use the PLS result command for the originating job.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InternalRecipeV4ExecutionSurfaceV1;
    use std::{thread, time::Duration};

    fn queued_job_with_kind(kind: InternalRecipeV4JobKindV1) -> (Uuid, InternalRecipeV4Job) {
        let snapshot = InternalRecipeV4JobSnapshotV1::queued();
        let id = snapshot.job_id;
        (
            id,
            InternalRecipeV4Job {
                snapshot,
                kind,
                worker_demand: 1,
                cancellation: Arc::new(AtomicBool::new(false)),
                result: None,
            },
        )
    }

    fn queued_job() -> (Uuid, InternalRecipeV4Job) {
        queued_job_with_kind(InternalRecipeV4JobKindV1::Pls)
    }

    #[test]
    fn snapshot_wire_contract_is_typed_and_camel_case() {
        let snapshot = InternalRecipeV4JobSnapshotV1::queued();
        let value = serde_json::to_value(snapshot).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["state"], "queued");
        assert_eq!(value["phase"], "queued");
        assert!(value.get("job_id").is_none());
        assert!(value["jobId"].as_str().is_some());
        assert!(chrono::DateTime::parse_from_rfc3339(value["queuedAt"].as_str().unwrap()).is_ok());
        assert!(value["startedAt"].is_null());
        assert!(value["completedAt"].is_null());
    }

    #[test]
    fn authoritative_cancellation_never_retains_a_partial_result() {
        let (job_id, job) = queued_job();
        job.cancellation.store(true, Ordering::Release);
        let jobs = Mutex::new(HashMap::from([(job_id, job)]));

        finish_cancelled(&jobs, job_id);

        let jobs = jobs.lock().unwrap();
        let job = &jobs[&job_id];
        assert_eq!(job.snapshot.state, InternalRecipeV4JobStateV1::Cancelled);
        assert!(job.result.is_none());
        assert!(job.snapshot.failure.is_none());
    }

    #[test]
    fn project_and_dataset_identity_must_both_stay_exact() {
        let project = Project::new("changed");
        assert!(!project_contains_exact_dataset(
            &project,
            Uuid::new_v4(),
            &Uuid::nil().to_string(),
            "dataset",
        ));
    }

    #[test]
    fn internal_labs_surface_has_a_stable_wire_value() {
        assert_eq!(
            serde_json::to_value(InternalRecipeV4ExecutionSurfaceV1::InternalLabs).unwrap(),
            "internal_labs"
        );
    }

    #[test]
    fn job_lifecycle_completes_without_mutating_the_schema_five_project() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let project = Arc::new(Mutex::new(project));
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let standard_jobs = Arc::new(Mutex::new(HashMap::new()));
        let snapshot = start_job(request, project.clone(), standard_jobs, jobs.clone()).unwrap();

        let terminal = (0..200).find_map(|_| {
            let snapshot = jobs
                .lock()
                .unwrap()
                .get(&snapshot.job_id)
                .unwrap()
                .snapshot
                .clone();
            if snapshot.state.is_terminal() {
                Some(snapshot)
            } else {
                thread::sleep(Duration::from_millis(10));
                None
            }
        });

        let terminal = terminal.expect("internal job did not reach a terminal state");
        assert_eq!(terminal.state, InternalRecipeV4JobStateV1::Completed);
        assert!(terminal.failure.is_none());
        assert!(terminal.started_at.is_some());
        assert!(terminal.completed_at.is_some());
        let jobs = jobs.lock().unwrap();
        let InternalRecipeV4StoredResultV1::Pls(result) =
            jobs[&snapshot.job_id].result.as_ref().unwrap()
        else {
            panic!("PLS job retained the wrong result kind");
        };
        assert!(result.analytical_result.estimation().converged);
        assert_eq!(result.analytical_result.estimation().paths.len(), 1);
        assert!(validate_canonical_result_document_v2(&result.canonical_document).passed);
        assert_eq!(
            result.canonical_document.provenance.run_id,
            snapshot.job_id.to_string()
        );
        let wire = serde_json::to_value(result).unwrap();
        assert_eq!(wire["schemaVersion"], 1);
        assert_eq!(wire["canonicalDocument"]["schema_version"], 2);
        assert_eq!(
            wire["analyticalResult"]["provenance"]["compilation_receipt"]["capability_cell"]["cell_id"],
            "qpls3.pls.algorithm"
        );
        assert!(wire.get("canonical_document").is_none());
        drop(jobs);
        let project = project.lock().unwrap();
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());
    }

    #[test]
    fn cancellation_before_worker_execution_is_terminal_and_result_free() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let project_id = project.manifest.project_id;
        let project = Arc::new(Mutex::new(project));
        let (job_id, job) = queued_job();
        job.cancellation.store(true, Ordering::Release);
        let jobs = Arc::new(Mutex::new(HashMap::from([(job_id, job)])));

        run_job_worker(project, jobs.clone(), job_id, project_id, dataset, request);

        let jobs = jobs.lock().unwrap();
        let job = &jobs[&job_id];
        assert_eq!(job.snapshot.state, InternalRecipeV4JobStateV1::Cancelled);
        assert!(job.result.is_none());
        assert!(job.snapshot.failure.is_none());
    }

    #[test]
    fn standard_and_internal_jobs_share_the_four_job_limit() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let project = Arc::new(Mutex::new(project));
        let standard_jobs = (0..4)
            .map(|_| {
                let snapshot = qpls_core::JobSnapshot::queued(2);
                (
                    snapshot.id,
                    DesktopJob {
                        snapshot,
                        cancellation: Arc::new(AtomicBool::new(false)),
                        result: None,
                        worker_demand: 1,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let failure = start_job(
            request,
            project,
            Arc::new(Mutex::new(standard_jobs)),
            Arc::new(Mutex::new(HashMap::new())),
        )
        .unwrap_err();

        assert_eq!(failure.code, "recipe_v4.active_job_limit_reached");
    }

    #[test]
    fn comparison_reservation_is_the_fourth_shared_standard_pls_cbsem_job_and_never_a_fifth() {
        let standard_snapshot = qpls_core::JobSnapshot::queued(2);
        let standard_jobs = Arc::new(Mutex::new(HashMap::from([(
            standard_snapshot.id,
            DesktopJob {
                snapshot: standard_snapshot,
                cancellation: Arc::new(AtomicBool::new(false)),
                result: None,
                worker_demand: 1,
            },
        )])));
        let (pls_id, pls) = queued_job_with_kind(InternalRecipeV4JobKindV1::Pls);
        let (cbsem_id, cbsem) = queued_job_with_kind(InternalRecipeV4JobKindV1::Cbsem);
        let shared = DesktopRecipeV4Jobs(Arc::new(Mutex::new(HashMap::from([
            (pls_id, pls),
            (cbsem_id, cbsem),
        ]))));

        let comparison = reserve_pls_model_comparison_admission_with_cpu_budget(
            Uuid::from_u128(41),
            standard_jobs.clone(),
            shared.clone(),
            8,
        )
        .unwrap();
        assert_eq!(shared.active_summary().unwrap(), (3, 3));

        let fifth = reserve_pls_model_comparison_admission_with_cpu_budget(
            Uuid::from_u128(42),
            standard_jobs,
            shared.clone(),
            8,
        )
        .unwrap_err();
        assert_eq!(fifth.code, "pls_model_comparison.active_job_limit_reached");
        assert_eq!(shared.active_summary().unwrap(), (3, 3));

        drop(comparison);
        assert_eq!(shared.active_summary().unwrap(), (2, 2));
    }

    #[test]
    fn failed_comparison_admission_releases_both_locks_and_leaves_no_reservation() {
        let standard_snapshot = qpls_core::JobSnapshot::queued(2);
        let standard_jobs = Arc::new(Mutex::new(HashMap::from([(
            standard_snapshot.id,
            DesktopJob {
                snapshot: standard_snapshot,
                cancellation: Arc::new(AtomicBool::new(false)),
                result: None,
                worker_demand: 1,
            },
        )])));
        let shared = DesktopRecipeV4Jobs::default();
        let blocked = reserve_pls_model_comparison_admission_with_cpu_budget(
            Uuid::from_u128(51),
            standard_jobs.clone(),
            shared.clone(),
            1,
        )
        .unwrap_err();
        assert_eq!(
            blocked.code,
            "pls_model_comparison.worker_budget_unavailable"
        );
        assert_eq!(shared.active_summary().unwrap(), (0, 0));

        standard_jobs.lock().unwrap().clear();
        let reservation = reserve_pls_model_comparison_admission_with_cpu_budget(
            Uuid::from_u128(52),
            standard_jobs,
            shared.clone(),
            1,
        )
        .unwrap();
        assert_eq!(shared.active_summary().unwrap(), (1, 1));
        drop(reservation);
        assert_eq!(shared.active_summary().unwrap(), (0, 0));
    }

    #[test]
    fn cbsem_job_completes_with_a_native_canonical_result_and_no_schema5_mutation() {
        let (project, request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let project = Arc::new(Mutex::new(project));
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let standard_jobs = Arc::new(Mutex::new(HashMap::new()));
        let snapshot =
            start_cbsem_job(request, project.clone(), standard_jobs, jobs.clone()).unwrap();

        let terminal = (0..300).find_map(|_| {
            let snapshot = jobs
                .lock()
                .unwrap()
                .get(&snapshot.job_id)
                .unwrap()
                .snapshot
                .clone();
            if snapshot.state.is_terminal() {
                Some(snapshot)
            } else {
                thread::sleep(Duration::from_millis(10));
                None
            }
        });
        let terminal = terminal.expect("CB-SEM job did not reach a terminal state");
        assert_eq!(terminal.state, InternalRecipeV4JobStateV1::Completed);
        assert!(terminal.failure.is_none());

        let jobs = jobs.lock().unwrap();
        let InternalRecipeV4StoredResultV1::Cbsem(result) =
            jobs[&snapshot.job_id].result.as_ref().unwrap()
        else {
            panic!("CB-SEM job retained the wrong result kind");
        };
        assert!(result.analytical_result.estimation().analysis.converged);
        assert!(validate_canonical_result_document_v2(&result.canonical_document).passed);
        assert_eq!(
            result.canonical_document.provenance.capability_cell.cell_id,
            "qpls3.cbsem.ml"
        );
        assert_eq!(
            result.canonical_document.provenance.run_id,
            snapshot.job_id.to_string()
        );
        drop(jobs);
        let project = project.lock().unwrap();
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());
    }

    #[test]
    fn cbsem_cancellation_before_worker_execution_is_terminal_and_result_free() {
        let (project, request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let project_id = project.manifest.project_id;
        let project = Arc::new(Mutex::new(project));
        let (job_id, job) = queued_job_with_kind(InternalRecipeV4JobKindV1::Cbsem);
        job.cancellation.store(true, Ordering::Release);
        let jobs = Arc::new(Mutex::new(HashMap::from([(job_id, job)])));

        run_cbsem_job_worker(project, jobs.clone(), job_id, project_id, dataset, request);

        let jobs = jobs.lock().unwrap();
        let job = &jobs[&job_id];
        assert_eq!(job.snapshot.state, InternalRecipeV4JobStateV1::Cancelled);
        assert!(job.result.is_none());
        assert!(job.snapshot.failure.is_none());
    }

    #[test]
    fn one_shared_internal_map_enforces_job_kind_and_cross_estimator_capacity() {
        let (project, request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let project = Arc::new(Mutex::new(project));
        let active = (0..4)
            .map(|index| {
                queued_job_with_kind(if index % 2 == 0 {
                    InternalRecipeV4JobKindV1::Pls
                } else {
                    InternalRecipeV4JobKindV1::Cbsem
                })
            })
            .collect::<HashMap<_, _>>();
        let failure = start_cbsem_job(
            request,
            project,
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(active)),
        )
        .unwrap_err();
        assert_eq!(failure.code, "recipe_v4.active_job_limit_reached");

        let (job_id, job) = queued_job_with_kind(InternalRecipeV4JobKindV1::Cbsem);
        assert_eq!(
            ensure_job_kind(&job, InternalRecipeV4JobKindV1::Pls, job_id)
                .unwrap_err()
                .code,
            "recipe_v4.job_kind_mismatch"
        );
    }
}
