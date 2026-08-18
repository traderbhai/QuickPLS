//! Isolated Internal/Labs job service for genuine two-model PLS comparison.
//!
//! Result storage is separate from Standard and Recipe-v4 results. Admission,
//! however, is reserved atomically in the existing shared job pool so the
//! desktop-wide four-job and CPU budgets cannot be bypassed.

use crate::recipe_v4_jobs::{
    DesktopRecipeV4Jobs, PlsModelComparisonAdmissionReservationV1,
    reserve_pls_model_comparison_admission,
};
use crate::{DesktopJob, DesktopJobs, DesktopProject, InternalRecipeV4ExecutionFailureV1};
use chrono::{SecondsFormat, Utc};
use qpls_core::CanonicalResultDocumentV2;
use qpls_project::Project;
use qpls_runner::{
    InternalLabsPlsModelComparisonRequestV1, PlsModelComparisonExecutionErrorV1,
    PlsModelComparisonExecutionResultV1, PlsModelComparisonRunContextV1,
    PlsModelComparisonRunnerProgressV1, build_pls_model_comparison_canonical_result_v2,
    run_internal_labs_pls_model_comparison_v1,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

const PLS_MODEL_COMPARISON_JOB_SCHEMA_VERSION_V1: u32 = 1;
const MAXIMUM_RETAINED_PLS_MODEL_COMPARISON_JOBS: usize = 255;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlsModelComparisonJobStateV1 {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl PlsModelComparisonJobStateV1 {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlsModelComparisonFailureStageV1 {
    Access,
    DataResolution,
    Admission,
    Execution,
    Integrity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlsModelComparisonFailureV1 {
    schema_version: u32,
    stage: PlsModelComparisonFailureStageV1,
    subject: String,
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlsModelComparisonJobSnapshotV1 {
    schema_version: u32,
    job_id: Uuid,
    state: PlsModelComparisonJobStateV1,
    phase: String,
    completed_units: u64,
    total_units: u64,
    repeat: Option<usize>,
    fold: Option<usize>,
    model: Option<String>,
    message: Option<String>,
    failure: Option<PlsModelComparisonFailureV1>,
    queued_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

impl PlsModelComparisonJobSnapshotV1 {
    fn queued() -> Self {
        Self {
            schema_version: PLS_MODEL_COMPARISON_JOB_SCHEMA_VERSION_V1,
            job_id: Uuid::new_v4(),
            state: PlsModelComparisonJobStateV1::Queued,
            phase: "queued".into(),
            completed_units: 0,
            total_units: 1,
            repeat: None,
            fold: None,
            model: None,
            message: None,
            failure: None,
            queued_at: now_utc(),
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlsModelComparisonCompletedResultV1 {
    schema_version: u32,
    analytical_result: PlsModelComparisonExecutionResultV1,
    canonical_document: CanonicalResultDocumentV2,
}

struct PlsModelComparisonJobV1 {
    snapshot: PlsModelComparisonJobSnapshotV1,
    cancellation: Arc<AtomicBool>,
    result: Option<PlsModelComparisonCompletedResultV1>,
}

#[derive(Clone, Default)]
pub(crate) struct DesktopPlsModelComparisonJobsV1(
    Arc<Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>>,
);

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn failure(
    stage: PlsModelComparisonFailureStageV1,
    subject: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> PlsModelComparisonFailureV1 {
    PlsModelComparisonFailureV1 {
        schema_version: PLS_MODEL_COMPARISON_JOB_SCHEMA_VERSION_V1,
        stage,
        subject: subject.into(),
        code: code.into(),
        message: message.into(),
        corrective_action: corrective_action.into(),
    }
}

fn admission_failure(source: InternalRecipeV4ExecutionFailureV1) -> PlsModelComparisonFailureV1 {
    failure(
        PlsModelComparisonFailureStageV1::Admission,
        source.subject,
        source.code,
        source.message,
        source.corrective_action,
    )
}

fn execution_failure(error: PlsModelComparisonExecutionErrorV1) -> PlsModelComparisonFailureV1 {
    failure(
        if matches!(
            &error,
            PlsModelComparisonExecutionErrorV1::InternalLabsRequired
                | PlsModelComparisonExecutionErrorV1::RequestSchema
                | PlsModelComparisonExecutionErrorV1::CapabilityIdentity
                | PlsModelComparisonExecutionErrorV1::MethodIdentity
                | PlsModelComparisonExecutionErrorV1::QualificationBoundary
        ) {
            PlsModelComparisonFailureStageV1::Access
        } else {
            PlsModelComparisonFailureStageV1::Execution
        },
        "request",
        error.code(),
        error.to_string(),
        "Correct the exact model, dataset, fold, or Internal/Labs requirement and start a new comparison job.",
    )
}

fn resolve_dataset(
    project: &Project,
    request: &InternalLabsPlsModelComparisonRequestV1,
) -> Result<(qpls_data::Dataset, Uuid), PlsModelComparisonFailureV1> {
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == request.dataset_id)
        .ok_or_else(|| {
            failure(
                PlsModelComparisonFailureStageV1::DataResolution,
                "datasetId",
                "pls_model_comparison.dataset_not_resident",
                "The exact comparison dataset is not resident in the active project.",
                "Rebuild the request from the active project and its current dataset identity.",
            )
        })?;
    if dataset.fingerprint.0 != request.dataset_fingerprint {
        return Err(failure(
            PlsModelComparisonFailureStageV1::DataResolution,
            "datasetFingerprint",
            "pls_model_comparison.dataset_fingerprint_changed",
            "The resident dataset fingerprint differs from the exact comparison request.",
            "Rebuild both model recipes and their shared-fold request from the current dataset.",
        ));
    }
    Ok((dataset.clone(), project.manifest.project_id))
}

fn project_still_matches(
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

fn set_running(
    jobs: &Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>,
    job_id: Uuid,
    update: Option<PlsModelComparisonRunnerProgressV1>,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        if job.snapshot.state == PlsModelComparisonJobStateV1::Queued {
            job.snapshot.state = PlsModelComparisonJobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        if let Some(update) = update
            && matches!(
                job.snapshot.state,
                PlsModelComparisonJobStateV1::Running | PlsModelComparisonJobStateV1::Cancelling
            )
        {
            job.snapshot.phase = update.phase;
            job.snapshot.completed_units = update.completed_units.min(update.total_units);
            job.snapshot.total_units = update.total_units.max(1);
            job.snapshot.repeat = update.repeat;
            job.snapshot.fold = update.fold;
            job.snapshot.model = update.model;
        }
    }
}

fn finish_cancelled(jobs: &Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>, job_id: Uuid) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = PlsModelComparisonJobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn finish_failed(
    jobs: &Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>,
    job_id: Uuid,
    problem: PlsModelComparisonFailureV1,
) {
    if let Ok(mut jobs) = jobs.lock()
        && let Some(job) = jobs.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = PlsModelComparisonJobStateV1::Failed;
        job.snapshot.phase = "failed".into();
        job.snapshot.message = Some(problem.message.clone());
        job.snapshot.failure = Some(problem);
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn publish_completed(
    project: &Mutex<Project>,
    jobs: &Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>,
    job_id: Uuid,
    expected_project_id: Uuid,
    request: &InternalLabsPlsModelComparisonRequestV1,
    analytical_result: PlsModelComparisonExecutionResultV1,
) {
    let project_matches = match project.lock() {
        Ok(project) => project_still_matches(
            &project,
            expected_project_id,
            &request.dataset_id,
            &request.dataset_fingerprint,
        ),
        Err(_) => {
            finish_failed(
                jobs,
                job_id,
                failure(
                    PlsModelComparisonFailureStageV1::Integrity,
                    "project",
                    "pls_model_comparison.project_state_unavailable_at_completion",
                    "The active project could not be rechecked before publishing the comparison.",
                    "Discard this job and retry after the active project finishes its current operation.",
                ),
            );
            return;
        }
    };
    if !project_matches {
        finish_failed(
            jobs,
            job_id,
            failure(
                PlsModelComparisonFailureStageV1::Integrity,
                "project",
                "pls_model_comparison.active_project_changed",
                "The active project or resident dataset changed while model comparison was running.",
                "Discard this job and rebuild both models from the current project and dataset.",
            ),
        );
        return;
    }

    let (started_at, completed_at, cancelled) = match jobs.lock() {
        Ok(jobs) => match jobs.get(&job_id) {
            Some(job) => (
                job.snapshot
                    .started_at
                    .clone()
                    .unwrap_or_else(|| job.snapshot.queued_at.clone()),
                now_utc(),
                job.cancellation.load(Ordering::Acquire)
                    || job.snapshot.state == PlsModelComparisonJobStateV1::Cancelling,
            ),
            None => return,
        },
        Err(_) => return,
    };
    if cancelled {
        finish_cancelled(jobs, job_id);
        return;
    }

    let canonical_document = match build_pls_model_comparison_canonical_result_v2(
        &PlsModelComparisonRunContextV1 {
            run_id: job_id,
            project_id: expected_project_id,
            started_at,
            completed_at: completed_at.clone(),
        },
        &analytical_result,
    ) {
        Ok(document) => document,
        Err(error) => {
            finish_failed(jobs, job_id, execution_failure(error));
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
        || job.snapshot.state == PlsModelComparisonJobStateV1::Cancelling
    {
        job.result = None;
        job.snapshot.state = PlsModelComparisonJobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.failure = None;
        job.snapshot.completed_at = Some(now_utc());
        return;
    }
    job.result = Some(PlsModelComparisonCompletedResultV1 {
        schema_version: PLS_MODEL_COMPARISON_JOB_SCHEMA_VERSION_V1,
        analytical_result,
        canonical_document,
    });
    job.snapshot.state = PlsModelComparisonJobStateV1::Completed;
    job.snapshot.phase = "completed".into();
    job.snapshot.completed_units = job.snapshot.total_units;
    job.snapshot.message = None;
    job.snapshot.failure = None;
    job.snapshot.completed_at = Some(completed_at);
}

#[allow(clippy::too_many_arguments)]
fn run_job_worker(
    project: Arc<Mutex<Project>>,
    jobs: Arc<Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>>,
    job_id: Uuid,
    project_id: Uuid,
    dataset: qpls_data::Dataset,
    request: InternalLabsPlsModelComparisonRequestV1,
    _admission: PlsModelComparisonAdmissionReservationV1,
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
    set_running(&jobs, job_id, None);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_internal_labs_pls_model_comparison_v1(
            &dataset,
            &request,
            || cancellation.load(Ordering::Acquire),
            |update| set_running(&jobs, job_id, Some(update)),
        )
    }));
    match outcome {
        Ok(Ok(result)) => publish_completed(&project, &jobs, job_id, project_id, &request, result),
        Ok(Err(error)) if error.is_cancelled() => finish_cancelled(&jobs, job_id),
        Ok(Err(error)) => finish_failed(&jobs, job_id, execution_failure(error)),
        Err(_) => finish_failed(
            &jobs,
            job_id,
            failure(
                PlsModelComparisonFailureStageV1::Integrity,
                "execution",
                "pls_model_comparison.worker_terminated_unexpectedly",
                "The PLS model-comparison worker terminated unexpectedly.",
                "Discard this job and retry. If it repeats, export a diagnostic bundle.",
            ),
        ),
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, PlsModelComparisonJobV1>) {
    if jobs.len() <= MAXIMUM_RETAINED_PLS_MODEL_COMPARISON_JOBS {
        return;
    }
    let removable = jobs
        .iter()
        .filter_map(|(id, job)| job.snapshot.state.is_terminal().then_some(*id))
        .take(jobs.len() - MAXIMUM_RETAINED_PLS_MODEL_COMPARISON_JOBS)
        .collect::<Vec<_>>();
    for id in removable {
        jobs.remove(&id);
    }
}

type ComparisonWorker = Box<dyn FnOnce() + Send + 'static>;

fn start_job_with_spawner<Spawn>(
    request: InternalLabsPlsModelComparisonRequestV1,
    project: Arc<Mutex<Project>>,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    shared_internal_jobs: DesktopRecipeV4Jobs,
    jobs: Arc<Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>>,
    spawn: Spawn,
) -> Result<PlsModelComparisonJobSnapshotV1, PlsModelComparisonFailureV1>
where
    Spawn: FnOnce(ComparisonWorker) -> io::Result<()>,
{
    if request.surface != "internal_labs" || !request.experimental_labs_enabled {
        return Err(failure(
            PlsModelComparisonFailureStageV1::Access,
            "experimentalLabsEnabled",
            "pls_model_comparison.internal_labs_required",
            "PLS model comparison is available only through Experimental Labs.",
            "Enable Experimental Labs and use the internal PLS model-comparison service.",
        ));
    }
    let (dataset, project_id) = {
        let project = project.lock().map_err(|_| {
            failure(
                PlsModelComparisonFailureStageV1::DataResolution,
                "project",
                "pls_model_comparison.project_state_unavailable",
                "The active project data is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?;
        resolve_dataset(&project, &request)?
    };
    // Validate the complete source/hash/method contract before charging a job
    // slot. Scientific topology validation remains inside the runner.
    qpls_runner::validate_internal_labs_pls_model_comparison_request_v1(&dataset, &request)
        .map_err(execution_failure)?;

    let snapshot = PlsModelComparisonJobSnapshotV1::queued();
    let job_id = snapshot.job_id;
    let admission =
        reserve_pls_model_comparison_admission(job_id, standard_jobs, shared_internal_jobs)
            .map_err(admission_failure)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut jobs_guard = jobs.lock().map_err(|_| {
            failure(
                PlsModelComparisonFailureStageV1::Admission,
                "jobs",
                "pls_model_comparison.job_state_unavailable",
                "The comparison job state is temporarily unavailable.",
                "Retry after current comparison jobs finish.",
            )
        })?;
        prune_terminal_jobs(&mut jobs_guard);
        if jobs_guard.contains_key(&job_id) {
            return Err(failure(
                PlsModelComparisonFailureStageV1::Admission,
                "jobId",
                "pls_model_comparison.duplicate_job_id",
                "The generated comparison job identity is already in use.",
                "Retry to generate a new job identity.",
            ));
        }
        jobs_guard.insert(
            job_id,
            PlsModelComparisonJobV1 {
                snapshot: snapshot.clone(),
                cancellation,
                result: None,
            },
        );
    }

    let worker_project = project;
    let worker_jobs = jobs.clone();
    let worker: ComparisonWorker = Box::new(move || {
        run_job_worker(
            worker_project,
            worker_jobs,
            job_id,
            project_id,
            dataset,
            request,
            admission,
        )
    });
    if let Err(error) = spawn(worker) {
        if let Ok(mut jobs) = jobs.lock() {
            jobs.remove(&job_id);
        }
        return Err(failure(
            PlsModelComparisonFailureStageV1::Admission,
            "worker",
            "pls_model_comparison.worker_spawn_failed",
            format!("The comparison worker could not be started: {error}"),
            "Retry after system resources become available.",
        ));
    }
    Ok(snapshot)
}

fn start_job(
    request: InternalLabsPlsModelComparisonRequestV1,
    project: Arc<Mutex<Project>>,
    standard_jobs: Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
    shared_internal_jobs: DesktopRecipeV4Jobs,
    jobs: Arc<Mutex<HashMap<Uuid, PlsModelComparisonJobV1>>>,
) -> Result<PlsModelComparisonJobSnapshotV1, PlsModelComparisonFailureV1> {
    start_job_with_spawner(
        request,
        project,
        standard_jobs,
        shared_internal_jobs,
        jobs,
        |worker| {
            std::thread::Builder::new()
                .name("quickpls-pls-model-comparison".into())
                .spawn(worker)
                .map(|_| ())
        },
    )
}

#[tauri::command]
pub(crate) fn start_internal_labs_pls_model_comparison_job(
    request: InternalLabsPlsModelComparisonRequestV1,
    project: State<'_, DesktopProject>,
    standard_jobs: State<'_, DesktopJobs>,
    shared_internal_jobs: State<'_, DesktopRecipeV4Jobs>,
    jobs: State<'_, DesktopPlsModelComparisonJobsV1>,
) -> Result<PlsModelComparisonJobSnapshotV1, PlsModelComparisonFailureV1> {
    start_job(
        request,
        project.0.clone(),
        standard_jobs.0.clone(),
        shared_internal_jobs.inner().clone(),
        jobs.0.clone(),
    )
}

#[tauri::command]
pub(crate) fn internal_labs_pls_model_comparison_job_status(
    job_id: Uuid,
    jobs: State<'_, DesktopPlsModelComparisonJobsV1>,
) -> Result<PlsModelComparisonJobSnapshotV1, PlsModelComparisonFailureV1> {
    let jobs = jobs.0.lock().map_err(|_| {
        failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobs",
            "pls_model_comparison.job_state_unavailable",
            "The comparison job state is temporarily unavailable.",
            "Retry after the current job operation finishes.",
        )
    })?;
    jobs.get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| unknown_job(job_id))
}

#[tauri::command]
pub(crate) fn cancel_internal_labs_pls_model_comparison_job(
    job_id: Uuid,
    jobs: State<'_, DesktopPlsModelComparisonJobsV1>,
) -> Result<PlsModelComparisonJobSnapshotV1, PlsModelComparisonFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobs",
            "pls_model_comparison.job_state_unavailable",
            "The comparison job state is temporarily unavailable.",
            "Retry after the current job operation finishes.",
        )
    })?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| unknown_job(job_id))?;
    if matches!(
        job.snapshot.state,
        PlsModelComparisonJobStateV1::Queued | PlsModelComparisonJobStateV1::Running
    ) {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = PlsModelComparisonJobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
pub(crate) fn dismiss_internal_labs_pls_model_comparison_job(
    job_id: Uuid,
    jobs: State<'_, DesktopPlsModelComparisonJobsV1>,
) -> Result<(), PlsModelComparisonFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobs",
            "pls_model_comparison.job_state_unavailable",
            "The comparison job state is temporarily unavailable.",
            "Retry after the current job operation finishes.",
        )
    })?;
    let terminal = jobs
        .get(&job_id)
        .map(|job| job.snapshot.state.is_terminal())
        .ok_or_else(|| unknown_job(job_id))?;
    if !terminal {
        return Err(failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobId",
            "pls_model_comparison.active_job_cannot_be_dismissed",
            "An active PLS model-comparison job cannot be dismissed.",
            "Wait for completion or cancellation before dismissing the job.",
        ));
    }
    jobs.remove(&job_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn internal_labs_pls_model_comparison_job_result(
    job_id: Uuid,
    jobs: State<'_, DesktopPlsModelComparisonJobsV1>,
) -> Result<PlsModelComparisonCompletedResultV1, PlsModelComparisonFailureV1> {
    let mut jobs = jobs.0.lock().map_err(|_| {
        failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobs",
            "pls_model_comparison.job_state_unavailable",
            "The comparison job state is temporarily unavailable.",
            "Retry after the current job operation finishes.",
        )
    })?;
    let job = jobs.get(&job_id).ok_or_else(|| unknown_job(job_id))?;
    if job.snapshot.state != PlsModelComparisonJobStateV1::Completed {
        return Err(failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobId",
            "pls_model_comparison.result_not_available",
            "A model-comparison result is available only after successful completion.",
            "Wait for completion or inspect the typed terminal failure.",
        ));
    }
    let mut job = jobs.remove(&job_id).expect("job existence was checked");
    job.result.take().ok_or_else(|| {
        failure(
            PlsModelComparisonFailureStageV1::Integrity,
            "jobId",
            "pls_model_comparison.completed_result_missing",
            "The completed comparison job did not retain its result.",
            "Discard the job and run both models again.",
        )
    })
}

fn unknown_job(job_id: Uuid) -> PlsModelComparisonFailureV1 {
    failure(
        PlsModelComparisonFailureStageV1::Integrity,
        "jobId",
        "pls_model_comparison.unknown_job",
        format!("No PLS model-comparison job exists with ID {job_id}."),
        "Refresh the internal job list and select an existing comparison job.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode, ModelSpec, StructuralPath,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_estimation::PlsModelComparisonConfigV1;

    fn fixture() -> (
        Arc<Mutex<Project>>,
        Arc<Mutex<HashMap<Uuid, DesktopJob>>>,
        DesktopRecipeV4Jobs,
        DesktopPlsModelComparisonJobsV1,
        InternalLabsPlsModelComparisonRequestV1,
    ) {
        let mut csv = String::from("x1,x2,z1,z2,y1,y2\n");
        for row in 0..40 {
            let t = row as f64 / 6.0;
            let x = t.sin() + row as f64 * 0.01;
            let z = (t * 0.8).cos() - row as f64 * 0.007;
            let noise = ((row * 11 % 9) as f64 - 4.0) * 0.015;
            let y = 0.64 * x + 0.52 * z + noise;
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                x + noise * 0.1,
                x * 0.94 - noise * 0.1,
                z - noise * 0.1,
                z * 1.03 + noise * 0.1,
                y + noise * 0.2,
                y * 0.97 - noise * 0.2
            ));
        }
        let bytes = csv.into_bytes();
        let dataset = import_delimited_bytes(
            &bytes,
            "tauri-model-comparison.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let constructs = vec![
            Construct {
                id: "x".into(),
                name: "X".into(),
                short_name: "X".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into()],
            },
            Construct {
                id: "z".into(),
                name: "Z".into(),
                short_name: "Z".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["z1".into(), "z2".into()],
            },
            Construct {
                id: "y".into(),
                name: "Y".into(),
                short_name: "Y".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["y1".into(), "y2".into()],
            },
        ];
        let model = |name: &str, include_z: bool| ModelSpec {
            id: Uuid::new_v4(),
            name: name.into(),
            constructs: if include_z {
                constructs.clone()
            } else {
                constructs
                    .iter()
                    .filter(|construct| construct.id != "z")
                    .cloned()
                    .collect()
            },
            paths: if include_z {
                vec![
                    StructuralPath {
                        source: "x".into(),
                        target: "y".into(),
                    },
                    StructuralPath {
                        source: "z".into(),
                        target: "y".into(),
                    },
                ]
            } else {
                vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }]
            },
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let settings = AnalysisSettings {
            workers: 1,
            ..AnalysisSettings::default()
        };
        let mut established =
            AnalysisRecipe::new(&bytes, model("Established", false), settings.clone());
        let mut alternative = AnalysisRecipe::new(&bytes, model("Alternative", true), settings);
        established.dataset_fingerprint = dataset.fingerprint.0.clone();
        alternative.dataset_fingerprint = dataset.fingerprint.0.clone();
        let request = InternalLabsPlsModelComparisonRequestV1::exact_internal_labs(
            &dataset,
            established,
            alternative,
            PlsModelComparisonConfigV1 {
                folds: 4,
                repeats: 1,
                seed: 551,
                confidence_level: 0.95,
            },
        );
        let mut project = Project::new("PLS comparison job fixture");
        project.datasets.push(dataset);
        (
            Arc::new(Mutex::new(project)),
            Arc::new(Mutex::new(HashMap::new())),
            DesktopRecipeV4Jobs::default(),
            DesktopPlsModelComparisonJobsV1::default(),
            request,
        )
    }

    #[test]
    fn successful_inline_worker_publishes_only_complete_result_and_releases_admission() {
        let (project, standard, shared, jobs, request) = fixture();
        let shared_for_assert = shared.clone();
        let snapshot = start_job_with_spawner(
            request,
            project,
            standard,
            shared.clone(),
            jobs.0.clone(),
            move |worker| {
                assert_eq!(shared_for_assert.active_summary().unwrap(), (1, 1));
                worker();
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(shared.active_summary().unwrap(), (0, 0));
        let guard = jobs.0.lock().unwrap();
        let job = guard.get(&snapshot.job_id).unwrap();
        assert_eq!(job.snapshot.state, PlsModelComparisonJobStateV1::Completed);
        let result = job.result.as_ref().unwrap();
        assert_eq!(
            result.canonical_document.provenance.run_id,
            snapshot.job_id.to_string()
        );
    }

    #[test]
    fn cancellation_before_execution_is_terminal_result_free_and_releases_admission() {
        let (project, standard, shared, jobs, request) = fixture();
        let jobs_for_cancel = jobs.0.clone();
        let snapshot = start_job_with_spawner(
            request,
            project,
            standard,
            shared.clone(),
            jobs.0.clone(),
            move |worker| {
                let cancellation = jobs_for_cancel
                    .lock()
                    .unwrap()
                    .values()
                    .next()
                    .unwrap()
                    .cancellation
                    .clone();
                cancellation.store(true, Ordering::Release);
                worker();
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(shared.active_summary().unwrap(), (0, 0));
        let guard = jobs.0.lock().unwrap();
        let job = guard.get(&snapshot.job_id).unwrap();
        assert_eq!(job.snapshot.state, PlsModelComparisonJobStateV1::Cancelled);
        assert!(job.result.is_none());
    }

    #[test]
    fn spawn_failure_removes_job_and_raii_reservation_without_leak() {
        let (project, standard, shared, jobs, request) = fixture();
        let error = start_job_with_spawner(
            request,
            project,
            standard,
            shared.clone(),
            jobs.0.clone(),
            |_worker| Err(io::Error::other("injected spawn failure")),
        )
        .unwrap_err();
        assert_eq!(error.code, "pls_model_comparison.worker_spawn_failed");
        assert_eq!(shared.active_summary().unwrap(), (0, 0));
        assert!(jobs.0.lock().unwrap().is_empty());
    }
}
