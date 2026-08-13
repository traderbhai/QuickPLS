use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use chrono::Utc;
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisResult,
    AnalysisSettings, Construct, JobSnapshot, JobState, METHOD_CAPABILITIES, MeasurementMode,
    MethodCapability, MethodConfig, ModelSpec, RunStatus, Severity, StructuralPath,
    ValidationIssue, sha256_hex, validate_recipe,
};
use qpls_data::{
    ColumnMetadata, DataKind, Dataset, ImportOptions, RecodeColumnSpec, import_delimited_bytes,
    import_path, preview, preview_page, recode_column, update_column_metadata,
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
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Command,
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

const MAX_DATASET_ROW_PAGE_SIZE: usize = 500;
const MAX_GROUP_PROFILE_VALUES: usize = 1_000;
const MAX_TEXT_EXPORT_BYTES: usize = 128 * 1024 * 1024;
const DATA_LINEAGE_LAYOUT_KEY: &str = "data_lineage";
const WORKSPACE_EXPLORER_LAYOUT_KEY: &str = "workspace_explorer";
const WORKSPACE_EXPLORER_SCHEMA_VERSION: u32 = 1;
const SAMPLE_RUN_DISPLAY_NAME: &str = "PLS-SEM Algorithm run";
const SAMPLE_RUN_METHOD_NAME: &str = "PLS-SEM Algorithm";

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DatasetVersionOperation {
    Import,
    Metadata,
    Recode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DatasetVersionRecord {
    dataset_id: String,
    parent_dataset_id: Option<String>,
    operation: DatasetVersionOperation,
    created_at: Option<String>,
    summary: String,
    source_column: Option<String>,
    target_column: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DatasetLineageLayout {
    schema_version: u32,
    records: Vec<DatasetVersionRecord>,
}

impl Default for DatasetLineageLayout {
    fn default() -> Self {
        Self {
            schema_version: 1,
            records: vec![],
        }
    }
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
    if group_methods.len() != 2
        || !group_methods
            .iter()
            .any(|method| method.eq_ignore_ascii_case("mga_permutation"))
        || !group_methods
            .iter()
            .any(|method| method.eq_ignore_ascii_case("micom"))
    {
        return Err("Native two-group analysis requires both permutation MGA and MICOM v2".into());
    }
    if !recipe
        .metadata
        .get("micom_configural_confirmed")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return Err(
            "MICOM requires explicit confirmation of configural invariance prerequisites".into(),
        );
    }
    if !recipe
        .metadata
        .get("group_permutation_samples")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .is_some_and(|samples| (5_000..=10_000).contains(&samples))
    {
        return Err("MICOM and permutation MGA require 5000 to 10000 permutations".into());
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
                    format!("{role} value '{selected}' is not observed in {group_column}")
                }
            })?;
        if group.complete_cases < 10 {
            return Err(format!(
                "{role} value '{selected}' has {} complete model cases; at least 10 are required",
                group.complete_cases
            ));
        }
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
    };
    Ok(commit_dataset_version(project, dataset, record)?.dataset)
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
    };
    commit_dataset_version(project, version, record)
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
    let mut lineage = read_dataset_lineage(project)?;
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
    let lineage_value = serde_json::to_value(lineage).map_err(|error| error.to_string())?;
    let workspace = workspace_with_active_dataset(project, &record.dataset_id)?;
    let response = DatasetVersionMutation {
        dataset: dataset_snapshot(&dataset),
        version: record,
    };

    project.datasets.push(dataset);
    project
        .layouts
        .insert(DATA_LINEAGE_LAYOUT_KEY.to_owned(), lineage_value);
    project.layouts.insert("workspace".to_owned(), workspace);
    Ok(response)
}

fn read_dataset_lineage(project: &Project) -> Result<DatasetLineageLayout, String> {
    project
        .layouts
        .get(DATA_LINEAGE_LAYOUT_KEY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("dataset lineage is invalid: {error}"))
        .map(Option::unwrap_or_default)
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
    model: Option<ModelSpec>,
    model_presentation: Option<Value>,
    state: State<'_, DesktopProject>,
) -> Result<ProjectSnapshot, String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
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
    let mut response = snapshot(&project, Some(path), None);
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
) -> Result<(), String> {
    let mut project = state
        .0
        .lock()
        .map_err(|_| "project state is unavailable".to_owned())?;
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
    Ok(snapshot(&project, path, None))
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

#[tauri::command]
fn start_analysis_job(
    recipe: AnalysisRecipe,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
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

/// Deprecated compatibility alias for `start_analysis_job`; retain for one major release.
#[tauri::command]
fn start_pls_job(
    recipe: AnalysisRecipe,
    project_state: State<'_, DesktopProject>,
    job_state: State<'_, DesktopJobs>,
) -> Result<JobSnapshot, String> {
    start_analysis_job(recipe, project_state, job_state)
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
    let explorer = normalized_workspace_explorer(project);
    ProjectSnapshot {
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
        dataset_versions: read_dataset_lineage(project)
            .map(|lineage| lineage.records)
            .unwrap_or_default(),
        workspace: project.layouts.get("workspace").cloned(),
        models: project.models.clone(),
        recipes: project.recipes.clone(),
        results: project.results.clone(),
        active_model_id: resolve_active_model_id(project),
        model_presentations: explorer.model_presentations,
        saved_reports: explorer.saved_reports,
    }
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
    settings.permutation_samples = 99;
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
    project.layouts.insert(
        DATA_LINEAGE_LAYOUT_KEY.into(),
        serde_json::to_value(DatasetLineageLayout {
            schema_version: 1,
            records: vec![DatasetVersionRecord {
                dataset_id,
                parent_dataset_id: None,
                operation: DatasetVersionOperation::Import,
                created_at: None,
                summary: "Bundled corporate reputation sample".into(),
                source_column: None,
                target_column: None,
            }],
        })
        .map_err(|error| error.to_string())?,
    );
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
        let lineage = read_dataset_lineage(&project).unwrap();
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
        let response = snapshot(&project, Some("study.qpls".into()), None);

        assert_eq!(response.models, project.models);
        assert_eq!(response.recipes, project.recipes);
        assert_eq!(response.results, project.results);
        assert_eq!(
            response.active_model_id.as_deref(),
            Some(canonical_model_id.as_str())
        );

        let wire = serde_json::to_value(response).unwrap();
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

        let response = snapshot(&project, None, None);
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
    fn recipe_embedded_model_remains_addressable_when_legacy_model_catalog_is_empty() {
        let mut project = build_demo_project().unwrap();
        let expected_model_id = project.recipes[0].model.id.to_string();
        let selected_run_id = project.results[0].id;
        project.models.clear();
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({ "selectedRunId": selected_run_id }),
        );

        let response = snapshot(&project, None, None);

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
        assert_eq!(snapshot(&project, None, None).active_model_id, None);

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("standalone-nca.qpls");
        save_project(&archive, &project).unwrap();
        let reopened = qpls_project::load_project(&archive).unwrap();
        let reopened_snapshot = snapshot(&reopened, Some(archive.to_string_lossy().into()), None);

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

        let legacy = snapshot(&project, Some("study.qpls".into()), None);
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

        let normalized = snapshot(&project, None, None);
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
        let explicitly_empty = snapshot(&project, None, None);
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
        let unsupported = snapshot(&project, None, None);
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
        let response = snapshot(&reopened, Some(archive.to_string_lossy().into()), None);
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
    fn native_mga_preflight_requires_micom_confirmation_and_accepts_profiled_groups() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../validation/results/mga_reference.csv"),
            "mga_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../validation/results/mga_reference.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        assert!(
            validate_mga_dataset_contract(&dataset, &recipe)
                .unwrap_err()
                .contains("requires both permutation MGA and MICOM v2")
        );
        recipe
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        assert!(
            validate_mga_dataset_contract(&dataset, &recipe)
                .unwrap_err()
                .contains("configural invariance")
        );
        recipe
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        validate_mga_dataset_contract(&dataset, &recipe).unwrap();
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
        let lineage = read_dataset_lineage(&project).unwrap();
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
        assert_eq!(read_dataset_lineage(&project).unwrap().records.len(), 2);

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
            read_dataset_lineage(&reopened).unwrap().records,
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
            dataset_rows,
            profile_dataset_groups,
            import_validation_fixture,
            open_demo_project,
            set_column_metadata,
            recode_dataset_column,
            activate_dataset,
            export_xlsx_tables,
            export_text_file,
            open_default_export_folder,
            verify_latest_release_checksums,
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
