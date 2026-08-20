//! Internal-only native job boundary for the unqualified CB-SEM V3 cells.
//!
//! Nothing in this module is a Tauri command and no state is registered with
//! the application shell. It connects the source-only adapters to the shared
//! job admission and schema-6 authorities for qualification fixtures only.

use crate::{
    DesktopJobs,
    recipe_v4_general_sem_cbsem_canonical_result::build_internal_cbsem_general_sem_canonical_result_v1,
    recipe_v4_general_sem_pls_jobs::{
        GeneralSemArchiveIdentityV1, InternalLabsGeneralSemPlsJobRequestV1,
        ResolvedGeneralSemArchiveV1, resolve_archive_authority, verify_archive_identity,
    },
    recipe_v4_jobs::{
        DesktopRecipeV4Jobs, PlsModelComparisonAdmissionReservationV1,
        reserve_general_sem_cbsem_admission,
    },
};
use chrono::{SecondsFormat, Utc};
use qpls_core::{
    CanonicalResultDocumentV2, GeneralSemInferenceV1, cbsem_general_sem_ml_capability_cell_v1,
    cbsem_recursive_sem_bootstrap_capability_cell_v1,
};
use qpls_project::{
    ProjectArchiveCanonicalAppendReceiptV6,
    append_canonical_result_document_v2_file_v6_with_cancel, load_project_archive_v6,
};
use qpls_runner::internal_cbsem_general_sem_execution::{
    InternalCbsemGeneralSemExecutionErrorV1, InternalCbsemGeneralSemExecutionResultV1,
    run_internal_cbsem_general_sem_v3,
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use uuid::Uuid;

const JOB_SCHEMA_VERSION_V1: u32 = 1;
const MAXIMUM_RETAINED_JOBS: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternalCbsemGeneralSemJobStateV1 {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl InternalCbsemGeneralSemJobStateV1 {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalCbsemGeneralSemJobSnapshotV1 {
    pub(crate) schema_version: u32,
    pub(crate) job_id: Uuid,
    pub(crate) state: InternalCbsemGeneralSemJobStateV1,
    pub(crate) phase: String,
    pub(crate) completed_units: u64,
    pub(crate) total_units: u64,
    pub(crate) message: Option<String>,
    pub(crate) queued_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

impl InternalCbsemGeneralSemJobSnapshotV1 {
    fn queued(job_id: Uuid) -> Self {
        Self {
            schema_version: JOB_SCHEMA_VERSION_V1,
            job_id,
            state: InternalCbsemGeneralSemJobStateV1::Queued,
            phase: "queued".into(),
            completed_units: 0,
            total_units: 1,
            message: None,
            queued_at: now_utc(),
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InternalCbsemGeneralSemCompletedResultV1 {
    pub(crate) schema_version: u32,
    pub(crate) archive_identity: GeneralSemArchiveIdentityV1,
    pub(crate) analytical_result: InternalCbsemGeneralSemExecutionResultV1,
    pub(crate) canonical_document: CanonicalResultDocumentV2,
}

struct InternalCbsemGeneralSemJobV1 {
    snapshot: InternalCbsemGeneralSemJobSnapshotV1,
    cancellation: Arc<AtomicBool>,
    archive_identity: GeneralSemArchiveIdentityV1,
    result: Option<InternalCbsemGeneralSemCompletedResultV1>,
}

#[derive(Clone, Default)]
pub(crate) struct DesktopCbsemGeneralSemJobsV1(
    Arc<Mutex<HashMap<Uuid, InternalCbsemGeneralSemJobV1>>>,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerCheckpointV1 {
    AfterExecutionBeforeCanonicalization,
    AfterCanonicalizationBeforePublication,
}

type WorkerCheckpointHookV1 = Arc<dyn Fn(WorkerCheckpointV1) + Send + Sync>;

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn expected_primary_cell(
    resolved: &ResolvedGeneralSemArchiveV1,
) -> Result<qpls_core::CapabilityCellReferenceV2, String> {
    let inference = resolved
        .recipe
        .general_sem_config
        .as_ref()
        .ok_or_else(|| "resident RecipeV4 omits GeneralSemConfigV1".to_owned())?
        .inference;
    Ok(match inference {
        GeneralSemInferenceV1::None => cbsem_general_sem_ml_capability_cell_v1(),
        GeneralSemInferenceV1::CaseBootstrap { .. } => {
            cbsem_recursive_sem_bootstrap_capability_cell_v1()
        }
    })
}

pub(crate) fn start_internal_cbsem_general_sem_job_v1(
    request: InternalLabsGeneralSemPlsJobRequestV1,
    standard_jobs: &DesktopJobs,
    shared_jobs: &DesktopRecipeV4Jobs,
    jobs: &DesktopCbsemGeneralSemJobsV1,
) -> Result<InternalCbsemGeneralSemJobSnapshotV1, String> {
    let resolved = resolve_archive_authority(&request).map_err(|error| error.message)?;
    if request.capability_cell != expected_primary_cell(&resolved)? {
        return Err(
            "requested candidate cell differs from the strict resident CB-SEM inference recipe"
                .into(),
        );
    }
    let job_id = Uuid::new_v4();
    let admission = reserve_general_sem_cbsem_admission(
        job_id,
        resolved.recipe.settings.workers,
        standard_jobs.0.clone(),
        shared_jobs.clone(),
    )
    .map_err(|error| error.message)?;
    let snapshot = InternalCbsemGeneralSemJobSnapshotV1::queued(job_id);
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut state = jobs
            .0
            .lock()
            .map_err(|_| "internal CB-SEM V3 job state is unavailable".to_owned())?;
        prune_terminal_jobs(&mut state);
        state.insert(
            job_id,
            InternalCbsemGeneralSemJobV1 {
                snapshot: snapshot.clone(),
                cancellation: cancellation.clone(),
                archive_identity: resolved.archive_identity.clone(),
                result: None,
            },
        );
    }
    let worker_jobs = jobs.0.clone();
    let cleanup_jobs = jobs.0.clone();
    std::thread::Builder::new()
        .name(format!("qpls-cbsem-general-sem-v3-{job_id}"))
        .spawn(move || {
            run_worker_with_checkpoint_hook(
                job_id,
                resolved,
                cancellation,
                worker_jobs,
                admission,
                None,
            )
        })
        .map_err(|error| {
            if let Ok(mut state) = cleanup_jobs.lock() {
                state.remove(&job_id);
            }
            format!("internal CB-SEM V3 worker could not start: {error}")
        })?;
    Ok(snapshot)
}

pub(crate) fn status_internal_cbsem_general_sem_job_v1(
    job_id: Uuid,
    jobs: &DesktopCbsemGeneralSemJobsV1,
) -> Result<InternalCbsemGeneralSemJobSnapshotV1, String> {
    jobs.0
        .lock()
        .map_err(|_| "internal CB-SEM V3 job state is unavailable".to_owned())?
        .get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| format!("unknown internal CB-SEM V3 job {job_id}"))
}

pub(crate) fn cancel_internal_cbsem_general_sem_job_v1(
    job_id: Uuid,
    jobs: &DesktopCbsemGeneralSemJobsV1,
) -> Result<InternalCbsemGeneralSemJobSnapshotV1, String> {
    let mut state = jobs
        .0
        .lock()
        .map_err(|_| "internal CB-SEM V3 job state is unavailable".to_owned())?;
    let job = state
        .get_mut(&job_id)
        .ok_or_else(|| format!("unknown internal CB-SEM V3 job {job_id}"))?;
    if matches!(
        job.snapshot.state,
        InternalCbsemGeneralSemJobStateV1::Queued | InternalCbsemGeneralSemJobStateV1::Running
    ) {
        job.cancellation.store(true, Ordering::Release);
        job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Cancelling;
        job.snapshot.message = Some("Cancellation requested".into());
    }
    Ok(job.snapshot.clone())
}

pub(crate) fn take_internal_cbsem_general_sem_result_v1(
    job_id: Uuid,
    jobs: &DesktopCbsemGeneralSemJobsV1,
) -> Result<InternalCbsemGeneralSemCompletedResultV1, String> {
    let identity = {
        let state = jobs
            .0
            .lock()
            .map_err(|_| "internal CB-SEM V3 job state is unavailable".to_owned())?;
        let job = state
            .get(&job_id)
            .ok_or_else(|| format!("unknown internal CB-SEM V3 job {job_id}"))?;
        if job.snapshot.state != InternalCbsemGeneralSemJobStateV1::Completed {
            return Err("internal CB-SEM V3 result is not complete".into());
        }
        job.archive_identity.clone()
    };
    verify_archive_identity(&identity).map_err(|error| error.message)?;
    jobs.0
        .lock()
        .map_err(|_| "internal CB-SEM V3 job state is unavailable".to_owned())?
        .remove(&job_id)
        .and_then(|job| job.result)
        .ok_or_else(|| "completed internal CB-SEM V3 result is unavailable".into())
}

fn run_worker_with_checkpoint_hook(
    job_id: Uuid,
    resolved: ResolvedGeneralSemArchiveV1,
    cancellation: Arc<AtomicBool>,
    jobs: Arc<Mutex<HashMap<Uuid, InternalCbsemGeneralSemJobV1>>>,
    _admission: PlsModelComparisonAdmissionReservationV1,
    checkpoint_hook: Option<WorkerCheckpointHookV1>,
) {
    set_running(&jobs, job_id, "compilation", 0, 1);
    let execution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_internal_cbsem_general_sem_v3(
            &resolved.dataset,
            &resolved.recipe,
            &resolved.model,
            || cancellation.load(Ordering::Acquire),
            |progress| {
                set_running(
                    &jobs,
                    job_id,
                    &progress.phase,
                    progress.completed_units,
                    progress.total_units,
                )
            },
        )
    }));
    let analytical_result = match execution {
        Ok(Ok(result)) => result,
        Ok(Err(InternalCbsemGeneralSemExecutionErrorV1::Cancelled)) => {
            finish_cancelled(&jobs, job_id);
            return;
        }
        Ok(Err(error)) => {
            finish_failed(&jobs, job_id, error.to_string());
            return;
        }
        Err(_) => {
            finish_failed(
                &jobs,
                job_id,
                "internal CB-SEM V3 worker terminated unexpectedly".into(),
            );
            return;
        }
    };
    notify_checkpoint(
        &checkpoint_hook,
        WorkerCheckpointV1::AfterExecutionBeforeCanonicalization,
    );
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    set_running(&jobs, job_id, "canonicalization", 0, 1);
    let started_at = jobs
        .lock()
        .ok()
        .and_then(|state| {
            state
                .get(&job_id)
                .and_then(|job| job.snapshot.started_at.clone())
        })
        .unwrap_or_else(now_utc);
    let completed_at = now_utc();
    let canonical_document = match build_internal_cbsem_general_sem_canonical_result_v1(
        job_id,
        resolved.document.project_id,
        &started_at,
        &completed_at,
        &resolved.dataset,
        &resolved.recipe,
        &resolved.model,
        &analytical_result,
    ) {
        Ok(document) => document,
        Err(errors) => {
            finish_failed(&jobs, job_id, errors.join("; "));
            return;
        }
    };
    notify_checkpoint(
        &checkpoint_hook,
        WorkerCheckpointV1::AfterCanonicalizationBeforePublication,
    );
    if cancellation.load(Ordering::Acquire) {
        finish_cancelled(&jobs, job_id);
        return;
    }
    if let Err(error) = verify_archive_identity(&resolved.archive_identity) {
        finish_failed(&jobs, job_id, error.message);
        return;
    }
    if let Ok(mut state) = jobs.lock()
        && let Some(job) = state.get_mut(&job_id)
    {
        if job.cancellation.load(Ordering::Acquire)
            || job.snapshot.state == InternalCbsemGeneralSemJobStateV1::Cancelling
        {
            job.result = None;
            job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Cancelled;
            job.snapshot.phase = "cancelled".into();
            job.snapshot.completed_at = Some(now_utc());
            return;
        }
        job.result = Some(InternalCbsemGeneralSemCompletedResultV1 {
            schema_version: JOB_SCHEMA_VERSION_V1,
            archive_identity: resolved.archive_identity,
            analytical_result,
            canonical_document,
        });
        job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Completed;
        job.snapshot.phase = "completed".into();
        job.snapshot.completed_units = 1;
        job.snapshot.total_units = 1;
        job.snapshot.message = None;
        job.snapshot.completed_at = Some(completed_at);
    }
}

pub(crate) fn append_and_reopen_internal_cbsem_general_sem_result_v1(
    completed: &InternalCbsemGeneralSemCompletedResultV1,
    cancelled: impl Fn() -> bool,
) -> Result<
    (
        ProjectArchiveCanonicalAppendReceiptV6,
        qpls_project::CanonicalResultDocumentV2,
    ),
    String,
> {
    if completed.schema_version != JOB_SCHEMA_VERSION_V1 || cancelled() {
        return Err("internal CB-SEM V3 append was cancelled before publication".into());
    }
    verify_archive_identity(&completed.archive_identity).map_err(|error| error.message)?;
    let project_document = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
        serde_json::to_value(&completed.canonical_document).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let receipt = append_canonical_result_document_v2_file_v6_with_cancel(
        Path::new(&completed.archive_identity.archive_path),
        &completed.archive_identity.archive_sha256,
        project_document.clone(),
        cancelled,
    )
    .map_err(|error| error.to_string())?;
    let reopened = load_project_archive_v6(Path::new(&completed.archive_identity.archive_path))
        .map_err(|error| error.to_string())?;
    let persisted = reopened
        .document
        .canonical_result_documents
        .iter()
        .find(|attachment| attachment.document_id() == project_document.document_id.as_str())
        .map(|attachment| attachment.canonical_document().clone())
        .ok_or_else(|| "appended CB-SEM V3 result was absent after fresh reopen".to_owned())?;
    if persisted != project_document
        || receipt.canonical_document_id != project_document.document_id
        || !receipt.post_write_validated
        || !receipt.rollback_copy_removed
    {
        return Err("fresh reopen differed from the exact appended CB-SEM V3 document".into());
    }
    Ok((receipt, persisted))
}

fn set_running(
    jobs: &Mutex<HashMap<Uuid, InternalCbsemGeneralSemJobV1>>,
    job_id: Uuid,
    phase: &str,
    completed_units: u64,
    total_units: u64,
) {
    if let Ok(mut state) = jobs.lock()
        && let Some(job) = state.get_mut(&job_id)
    {
        if job.snapshot.state == InternalCbsemGeneralSemJobStateV1::Queued {
            job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Running;
            job.snapshot.started_at = Some(now_utc());
        }
        if matches!(
            job.snapshot.state,
            InternalCbsemGeneralSemJobStateV1::Running
                | InternalCbsemGeneralSemJobStateV1::Cancelling
        ) {
            job.snapshot.phase = phase.into();
            job.snapshot.completed_units = completed_units.min(total_units);
            job.snapshot.total_units = total_units.max(1);
        }
    }
}

fn finish_cancelled(jobs: &Mutex<HashMap<Uuid, InternalCbsemGeneralSemJobV1>>, job_id: Uuid) {
    if let Ok(mut state) = jobs.lock()
        && let Some(job) = state.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Cancelled;
        job.snapshot.phase = "cancelled".into();
        job.snapshot.message = None;
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn finish_failed(
    jobs: &Mutex<HashMap<Uuid, InternalCbsemGeneralSemJobV1>>,
    job_id: Uuid,
    message: String,
) {
    if let Ok(mut state) = jobs.lock()
        && let Some(job) = state.get_mut(&job_id)
    {
        job.result = None;
        job.snapshot.state = InternalCbsemGeneralSemJobStateV1::Failed;
        job.snapshot.phase = "failed".into();
        job.snapshot.message = Some(message);
        job.snapshot.completed_at = Some(now_utc());
    }
}

fn notify_checkpoint(hook: &Option<WorkerCheckpointHookV1>, checkpoint: WorkerCheckpointV1) {
    if let Some(hook) = hook {
        hook(checkpoint);
    }
}

fn prune_terminal_jobs(jobs: &mut HashMap<Uuid, InternalCbsemGeneralSemJobV1>) {
    if jobs.len() <= MAXIMUM_RETAINED_JOBS {
        return;
    }
    let removable = jobs
        .iter()
        .filter_map(|(job_id, job)| job.snapshot.state.is_terminal().then_some(*job_id))
        .take(jobs.len() - MAXIMUM_RETAINED_JOBS)
        .collect::<Vec<_>>();
    for job_id in removable {
        jobs.remove(&job_id);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisRecipeV4, AnalysisSettings, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2,
        CbsemBootstrapInterval, CbsemBootstrapTestTail, CbsemEstimator, CbsemInput, CbsemModelType,
        Construct, GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
        LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4, MeasurementMode,
        MethodConfig, MissingDataPolicy, MissingDataPolicyV4, ModelSpec, Preprocessing,
        SemDataBindingV4, StructuralPath, convert_legacy_basic_model_v4, sha256_serialized,
    };
    use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
    use qpls_project::{
        ProjectArchiveV6Error, attach_canonical_result_document_v2_v6,
        create_populated_general_sem_project_archive_v6,
    };
    use std::{collections::BTreeMap, fs};

    struct PublishedFixtureV1 {
        _directory: tempfile::TempDir,
        request: InternalLabsGeneralSemPlsJobRequestV1,
    }

    fn recursive_fixture(bootstrap: bool) -> (Dataset, AnalysisRecipeV4, qpls_core::SemModelV4) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/results/lavaan_latent_regression_sem.csv"),
            "rank3-native-cbsem-v3.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xcb53_4001),
            name: "Rank 3 native recursive SEM fixture".into(),
            constructs: [
                ("x", ["x1", "x2", "x3"]),
                ("m", ["m1", "m2", "m3"]),
                ("y", ["y1", "y2", "y3"]),
            ]
            .into_iter()
            .map(|(id, indicators)| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: indicators.into_iter().map(str::to_owned).collect(),
            })
            .collect(),
            paths: [("x", "m"), ("x", "y"), ("m", "y")]
                .into_iter()
                .map(|(source, target)| StructuralPath {
                    source: source.into(),
                    target: target.into(),
                })
                .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        settings.workers = 1;
        settings.bootstrap_samples = if bootstrap { 500 } else { 0 };
        settings.seed = 777;
        settings.confidence_level = 0.95;
        let bootstrap_v2 = bootstrap.then_some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval: CbsemBootstrapInterval::PercentileType7,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        let general_sem_config = if bootstrap {
            GeneralSemConfigV1 {
                inference: GeneralSemInferenceV1::CaseBootstrap {
                    resamples: 500,
                    seed: 777,
                    confidence_level: 0.95,
                    interval: GeneralSemBootstrapIntervalV1::Percentile,
                    tail: GeneralSemInferenceTailV1::TwoSided,
                },
                ..GeneralSemConfigV1::default()
            }
        } else {
            GeneralSemConfigV1::default()
        };
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(if bootstrap { 0xcb53_4003 } else { 0xcb53_4002 }),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model.id.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: if bootstrap { 500 } else { 0 },
                bootstrap_v2,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: Some(general_sem_config),
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        recipe.ensure_valid().unwrap();
        (dataset, recipe, model)
    }

    fn published_fixture(bootstrap: bool) -> PublishedFixtureV1 {
        let (dataset, recipe, model) = recursive_fixture(bootstrap);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(if bootstrap {
            "cbsem-v3-bootstrap.qpls"
        } else {
            "cbsem-v3-point.qpls"
        });
        let recipe_digest = sha256_serialized(&recipe);
        let receipt = create_populated_general_sem_project_archive_v6(
            &path,
            Uuid::from_u128(if bootstrap { 0xcb53_4011 } else { 0xcb53_4010 }),
            "Rank 3 internal native fixture",
            Utc.timestamp_opt(1_700_000_100, 0).unwrap(),
            &dataset,
            model,
            recipe,
        )
        .unwrap();
        PublishedFixtureV1 {
            _directory: directory,
            request: InternalLabsGeneralSemPlsJobRequestV1 {
                surface: "internal_labs".into(),
                experimental_labs_enabled: true,
                archive_path: receipt.destination_archive_path,
                expected_archive_sha256: receipt.destination_archive_sha256,
                project_id: receipt.project_id.to_string(),
                dataset_id: receipt.resident_dataset_id.to_string(),
                dataset_fingerprint: receipt.resident_dataset_fingerprint,
                model_id: receipt.resident_model_id,
                model_scientific_sha256: receipt.resident_model_scientific_sha256,
                recipe_id: receipt.resident_recipe_id.to_string(),
                recipe_document_sha256: recipe_digest,
                capability_cell: if bootstrap {
                    cbsem_recursive_sem_bootstrap_capability_cell_v1()
                } else {
                    cbsem_general_sem_ml_capability_cell_v1()
                },
            },
        }
    }

    fn state_for(
        job_id: Uuid,
        cancellation: Arc<AtomicBool>,
        identity: GeneralSemArchiveIdentityV1,
    ) -> DesktopCbsemGeneralSemJobsV1 {
        let state = DesktopCbsemGeneralSemJobsV1::default();
        state.0.lock().unwrap().insert(
            job_id,
            InternalCbsemGeneralSemJobV1 {
                snapshot: InternalCbsemGeneralSemJobSnapshotV1::queued(job_id),
                cancellation,
                archive_identity: identity,
                result: None,
            },
        );
        state
    }

    fn admission(job_id: Uuid) -> PlsModelComparisonAdmissionReservationV1 {
        reserve_general_sem_cbsem_admission(
            job_id,
            1,
            Arc::new(Mutex::new(HashMap::new())),
            DesktopRecipeV4Jobs::default(),
        )
        .unwrap()
    }

    fn completed_point() -> (PublishedFixtureV1, InternalCbsemGeneralSemCompletedResultV1) {
        let published = published_fixture(false);
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0xcb53_4100);
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = state_for(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker_with_checkpoint_hook(
            job_id,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
            None,
        );
        assert_eq!(
            state.0.lock().unwrap()[&job_id].snapshot.state,
            InternalCbsemGeneralSemJobStateV1::Completed
        );
        let completed = take_internal_cbsem_general_sem_result_v1(job_id, &state).unwrap();
        (published, completed)
    }

    #[test]
    fn internal_point_job_appends_and_strictly_reopens_through_schema6() {
        let (_published, completed) = completed_point();
        let expected = completed.canonical_document.clone();
        let (receipt, reopened) =
            append_and_reopen_internal_cbsem_general_sem_result_v1(&completed, || false).unwrap();
        let reopened_core = serde_json::from_value::<CanonicalResultDocumentV2>(
            serde_json::to_value(reopened).unwrap(),
        )
        .unwrap();
        assert_eq!(reopened_core, expected);
        assert!(receipt.post_write_validated && receipt.rollback_copy_removed);
        assert_eq!(
            expected
                .general_sem_results
                .as_ref()
                .unwrap()
                .cbsem_fit
                .len(),
            1
        );
    }

    #[test]
    fn point_fit_and_parameter_table_tampering_fail_closed() {
        let (published, completed) = completed_point();
        let loaded = load_project_archive_v6(Path::new(&published.request.archive_path)).unwrap();
        let mut tampered = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&completed.canonical_document).unwrap(),
        )
        .unwrap();
        tampered.general_sem_results.as_mut().unwrap().cbsem_fit[0].chi_square += 1.0;
        assert!(matches!(
            attach_canonical_result_document_v2_v6(&loaded.document, tampered),
            Err(ProjectArchiveV6Error::CanonicalGeneralSemAuthority(message))
                if message.contains("tables")
        ));

        let mut tampered = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&completed.canonical_document).unwrap(),
        )
        .unwrap();
        tampered
            .general_sem_results
            .as_mut()
            .unwrap()
            .cbsem_parameters[0]
            .parameter_id = "parameter:foreign".into();
        assert!(attach_canonical_result_document_v2_v6(&loaded.document, tampered).is_err());
    }

    #[test]
    fn cancellation_after_canonicalization_publishes_nothing_and_keeps_archive_bytes() {
        let published = published_fixture(false);
        let before = fs::read(&published.request.archive_path).unwrap();
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0xcb53_4101);
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = state_for(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        let hook_cancel = cancellation.clone();
        run_worker_with_checkpoint_hook(
            job_id,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
            Some(Arc::new(move |checkpoint| {
                if checkpoint == WorkerCheckpointV1::AfterCanonicalizationBeforePublication {
                    hook_cancel.store(true, Ordering::Release);
                }
            })),
        );
        let state = state.0.lock().unwrap();
        assert_eq!(
            state[&job_id].snapshot.state,
            InternalCbsemGeneralSemJobStateV1::Cancelled
        );
        assert!(state[&job_id].result.is_none());
        assert_eq!(fs::read(&published.request.archive_path).unwrap(), before);
    }

    #[test]
    #[ignore = "qualification-scale fixture: executes 500 full recursive ML refits"]
    fn recursive_bootstrap_job_owns_one_receipt_and_inference_ledger() {
        let published = published_fixture(true);
        let resolved = resolve_archive_authority(&published.request).unwrap();
        let job_id = Uuid::from_u128(0xcb53_4102);
        let cancellation = Arc::new(AtomicBool::new(false));
        let state = state_for(
            job_id,
            cancellation.clone(),
            resolved.archive_identity.clone(),
        );
        run_worker_with_checkpoint_hook(
            job_id,
            resolved,
            cancellation,
            state.0.clone(),
            admission(job_id),
            None,
        );
        let completed = take_internal_cbsem_general_sem_result_v1(job_id, &state).unwrap();
        let mut receipt_tamper = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&completed.canonical_document).unwrap(),
        )
        .unwrap();
        receipt_tamper
            .general_sem_results
            .as_mut()
            .unwrap()
            .cbsem_bootstrap_receipt
            .as_mut()
            .unwrap()
            .compiled_plan_sha256 = "0".repeat(64);
        let loaded = load_project_archive_v6(Path::new(&published.request.archive_path)).unwrap();
        assert!(attach_canonical_result_document_v2_v6(&loaded.document, receipt_tamper).is_err());

        let results = completed
            .canonical_document
            .general_sem_results
            .as_ref()
            .unwrap();
        let receipt = results.cbsem_bootstrap_receipt.as_ref().unwrap();
        assert_eq!(receipt.resamples_requested, 500);
        assert_eq!(
            results.cbsem_bootstrap_inference.len(),
            results
                .cbsem_parameters
                .iter()
                .filter(|row| matches!(
                    row.state,
                    qpls_core::CanonicalCbsemParameterStateV1::Free { .. }
                ))
                .count()
        );
        let (_receipt, reopened) =
            append_and_reopen_internal_cbsem_general_sem_result_v1(&completed, || false).unwrap();
        assert!(
            reopened
                .general_sem_results
                .unwrap()
                .cbsem_bootstrap_receipt
                .is_some()
        );
    }
}
