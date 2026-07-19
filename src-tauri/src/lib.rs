use chrono::Utc;
use qpls_core::{
    AnalysisRecipe, AnalysisResult, AnalysisSettings, Construct, JobSnapshot, JobState,
    METHOD_CAPABILITIES, MeasurementMode, MethodCapability, ModelSpec, PROJECT_SCHEMA_VERSION,
    Severity, StructuralPath, ValidationIssue, validate_recipe,
};
use qpls_data::{
    ColumnMetadata, DataKind, Dataset, ImportOptions, import_delimited_bytes, import_path, preview,
    update_column_metadata,
};
use qpls_project::{
    Project, RecoverySource, discard_autosave, load_project_with_autosave, save_autosave,
    save_project,
};
use qpls_runner::{RunnerError, run_pls_analysis};
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::State;
use uuid::Uuid;

struct DesktopProject(Arc<Mutex<Project>>);

struct DesktopJob {
    snapshot: JobSnapshot,
    cancellation: Arc<AtomicBool>,
    result: Option<AnalysisResult>,
    worker_demand: usize,
}

struct DesktopJobs(Arc<Mutex<HashMap<Uuid, DesktopJob>>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetSnapshot {
    id: String,
    name: String,
    columns: Vec<String>,
    rows: Vec<std::collections::BTreeMap<String, Option<String>>>,
    row_count: usize,
    missing: usize,
    fingerprint: String,
    kind: DataKind,
    sample_size: Option<usize>,
    column_metadata: Vec<ColumnMetadata>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSnapshot {
    name: String,
    path: Option<String>,
    read_only: bool,
    recovered: bool,
    recovery_source: Option<String>,
    datasets: Vec<DatasetSnapshot>,
    workspace: Option<Value>,
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

fn write_xlsx_tables(
    path: &Path,
    tables: &[ExportTable],
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    for (index, table) in tables.iter().enumerate() {
        let worksheet = workbook.add_worksheet();
        let sheet_name = safe_sheet_name(&table.title, index);
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
    name.chars().take(31).collect()
}

#[tauri::command]
fn new_project(name: String, state: State<'_, DesktopProject>) -> Result<ProjectSnapshot, String> {
    let mut active = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    *active = Project::new(name);
    Ok(snapshot(&active, None, None))
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
    let response = dataset_snapshot(&dataset);
    state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?
        .datasets
        .push(dataset);
    Ok(response)
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
    let response = dataset_snapshot(&dataset);
    state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?
        .datasets
        .push(dataset);
    Ok(response)
}

#[tauri::command]
fn open_demo_project(state: State<'_, DesktopProject>) -> Result<ProjectSnapshot, String> {
    let project = build_demo_project().map_err(|error| error.to_string())?;
    let response = snapshot(&project, None, None);
    *state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())? = project;
    Ok(response)
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
    let dataset = project
        .datasets
        .iter_mut()
        .find(|dataset| dataset.id.to_string() == dataset_id)
        .ok_or_else(|| format!("unknown dataset {dataset_id}"))?;
    update_column_metadata(dataset, &column_name, metadata).map_err(|error| error.to_string())?;
    Ok(dataset_snapshot(dataset))
}

#[tauri::command]
fn open_project(path: String, state: State<'_, DesktopProject>) -> Result<ProjectSnapshot, String> {
    let (project, recovery_source) =
        load_project_with_autosave(Path::new(&path)).map_err(|error| error.to_string())?;
    let response = snapshot(&project, Some(path), recovery_source);
    *state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())? = project;
    Ok(response)
}

#[tauri::command]
fn save_active_project(
    path: String,
    workspace: Value,
    state: State<'_, DesktopProject>,
) -> Result<ProjectSnapshot, String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    project.layouts.insert("workspace".to_owned(), workspace);
    save_project(Path::new(&path), &project).map_err(|error| error.to_string())?;
    discard_autosave(Path::new(&path)).map_err(|error| error.to_string())?;
    Ok(snapshot(&project, Some(path), None))
}

#[tauri::command]
fn autosave_active_project(
    path: String,
    workspace: Value,
    state: State<'_, DesktopProject>,
) -> Result<(), String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
    project.layouts.insert("workspace".to_owned(), workspace);
    save_autosave(Path::new(&path), &project).map_err(|error| error.to_string())
}

#[tauri::command]
fn start_pls_job(
    recipe: AnalysisRecipe,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
) -> Result<JobSnapshot, String> {
    let issues = validate_recipe(&recipe);
    if let Some(issue) = issues
        .iter()
        .find(|issue| issue.severity == Severity::Error)
    {
        return Err(format!("{}: {}", issue.code, issue.message));
    }
    let (dataset, project_id) = {
        let project = project_state
            .0
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        if project.read_only {
            return Err("cannot run or store analyses in a read-only project".into());
        }
        let dataset = project
            .datasets
            .iter()
            .find(|dataset| dataset.fingerprint.0 == recipe.dataset_fingerprint)
            .cloned()
            .ok_or_else(|| "recipe dataset fingerprint is not present in the project".to_owned())?;
        (dataset, project.manifest.project_id)
    };
    let snapshot = JobSnapshot::queued(2);
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut jobs_guard = job_state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?;
    prune_terminal_jobs(&mut jobs_guard, 255);
    let active_count = jobs_guard
        .values()
        .filter(|job| {
            matches!(
                job.snapshot.state,
                JobState::Queued | JobState::Running | JobState::Cancelling | JobState::Committing
            )
        })
        .count();
    if active_count >= 4 {
        return Err("four analyses are already active; wait for one to finish".into());
    }
    let worker_demand =
        if recipe.settings.bootstrap_samples > 0 || recipe.settings.permutation_samples > 0 {
            recipe.settings.workers
        } else {
            1
        };
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
        .sum::<usize>();
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

#[tauri::command]
fn pls_job_status(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
    state
        .0
        .lock()
        .map_err(|_| "job state is unavailable".to_owned())?
        .get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| format!("unknown job {job_id}"))
}

#[tauri::command]
fn cancel_pls_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<JobSnapshot, String> {
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

#[tauri::command]
fn dismiss_pls_job(job_id: Uuid, state: State<'_, DesktopJobs>) -> Result<(), String> {
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

#[tauri::command]
fn pls_job_result(
    job_id: Uuid,
    state: State<'_, DesktopJobs>,
) -> Result<Option<AnalysisResult>, String> {
    take_job_result(&state.0, job_id)
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
    if project.read_only {
        return Err("the project became read-only while estimation was running".into());
    }
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
    project.recipes.push(recipe);
    project.results.push(result.clone());
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
        job.snapshot.state = state;
        job.snapshot.phase = phase.into();
        job.snapshot.completed_units = completed_units;
        job.snapshot.total_units = total_units;
        job.snapshot.message = message;
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
) -> ProjectSnapshot {
    ProjectSnapshot {
        name: project.manifest.name.clone(),
        path,
        read_only: project.read_only,
        recovered: recovery_source.is_some(),
        recovery_source: recovery_source.map(|source| {
            match source {
                RecoverySource::Autosave => "autosave",
                RecoverySource::Backup => "backup",
            }
            .to_owned()
        }),
        datasets: project.datasets.iter().map(dataset_snapshot).collect(),
        workspace: project.layouts.get("workspace").cloned(),
    }
}

fn dataset_snapshot(dataset: &Dataset) -> DatasetSnapshot {
    let missing = dataset
        .batch
        .columns()
        .iter()
        .map(|column| column.null_count())
        .sum();
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
    settings.permutation_samples = 99;
    settings.seed = 20_260_718;
    settings.workers = 1;
    let recipe = AnalysisRecipe {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: "00000000-0000-0000-0000-00000000d004"
            .parse()
            .expect("fixed demo recipe UUID is valid"),
        created_at: chrono::DateTime::parse_from_rfc3339("2026-07-18T00:00:00Z")
            .expect("fixed demo date is valid")
            .with_timezone(&Utc),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: model.clone(),
        settings,
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
    let mut project = Project::new("QuickPLS v0.4 Demo Evidence Project");
    project.datasets.push(dataset);
    project.models.push(model);
    project.recipes.push(recipe);
    project.results.push(result);
    project.layouts.insert("workspace".into(), workspace);
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
        _ => unreachable!("demo result is created as pls_pm_v3"),
    };
    serde_json::json!({
        "activeDatasetId": dataset.id.to_string(),
        "analysisSettings": {
            "bootstrapSamples": 24,
            "studentizedInnerSamples": 0,
            "permutationSamples": 99,
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
            "name": "v0.4 demo evidence run",
            "method": "PLS-PM",
            "createdAt": result.provenance.completed_at,
            "seed": result.provenance.seed,
            "status": "completed",
            "warnings": ["Validated for the documented QuickPLS v0.9.0-rc.1 supported scope."],
            "fingerprint": result.provenance.dataset_fingerprint.chars().take(12).collect::<String>(),
            "result": estimation,
            "assessment": assessment,
            "bootstrap": bootstrap,
            "permutation": permutation
        }]
    })
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
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = AnalysisResult::completed_pls(
            &recipe,
            "test",
            Utc::now(),
            serde_json::Value::Null,
            serde_json::Value::Null,
            Vec::new(),
        );
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
        assert_eq!(project.datasets.len(), 1);
        assert_eq!(project.models.len(), 1);
        assert_eq!(project.recipes.len(), 1);
        assert_eq!(project.results.len(), 1);
        let workspace = project.layouts.get("workspace").unwrap();
        let dataset_id = project.datasets[0].id.to_string();
        assert_eq!(
            workspace["activeDatasetId"].as_str(),
            Some(dataset_id.as_str())
        );
        assert_eq!(workspace["nodes"].as_array().unwrap().len(), 4);
        assert_eq!(workspace["edges"].as_array().unwrap().len(), 3);
        assert_eq!(workspace["runs"].as_array().unwrap().len(), 1);
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
                "Validated for the documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked."
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
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let desktop_result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let cli_result: AnalysisResult = serde_json::from_slice(include_bytes!(
            "../../validation/results/pls_quickpls_path_mode_a.json"
        ))
        .unwrap();
        assert_json_close(
            &serde_json::to_value(&desktop_result.payload).unwrap(),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopProject(Arc::new(Mutex::new(Project::new(
            "Untitled project",
        )))))
        .manage(DesktopJobs(Arc::new(Mutex::new(HashMap::new()))))
        .invoke_handler(tauri::generate_handler![
            validate_analysis_recipe,
            method_capabilities,
            new_project,
            import_dataset,
            import_validation_fixture,
            open_demo_project,
            set_column_metadata,
            export_xlsx_tables,
            open_project,
            save_active_project,
            autosave_active_project,
            start_pls_job,
            pls_job_status,
            cancel_pls_job,
            dismiss_pls_job,
            pls_job_result
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuickPLS");
}
