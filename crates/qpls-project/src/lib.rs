mod archive_integrity;

use archive_integrity::{
    ArchiveIntegrityError, DEFAULT_ARCHIVE_LIMITS, MAX_MANIFEST_UNCOMPRESSED_BYTES,
    MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES, PROJECT_ENTRY_NAME, expected_project_entries,
    preflight_archive, read_preflighted_entry, validate_expected_project_entries,
    validate_manifest_checksums, validate_raw_central_directory, verify_archive_checksums,
};
use chrono::{DateTime, Utc};
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, ASSESSMENT_METHOD_VERSION_V1, ASSESSMENT_METHOD_VERSION_V2,
    ASSESSMENT_METHOD_VERSION_V3, ASSESSMENT_METHOD_VERSION_V4, ASSESSMENT_METHOD_VERSION_V5,
    ASSESSMENT_METHOD_VERSION_V6, AssessmentResult, HTMT_ORIGINAL_METHOD_VERSION,
    HTMT_PLUS_METHOD_VERSION, HtmtAssessment, HtmtStatus, RHO_A_METHOD_VERSION, RhoAStatus,
    variance_inflation_factor,
};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisPayload, AnalysisRecipe,
    AnalysisResult, AnalysisSettings, Diagnostic, DiagnosticLevel, ENGINE_VERSION,
    HigherOrderMethod, MeasurementMode, MissingDataPolicy, ModelSpec, Preprocessing,
    RESULT_SCHEMA_VERSION, RunProvenance, RunStatus, Severity, WeightingScheme,
    ipma_predecessor_constructs, resolve_ipma_targets, validate_recipe,
};
use qpls_data::{Dataset, DatasetDescriptor, dataset_from_descriptor, write_arrow};
use qpls_estimation::{
    CBSEM_FIT_METHOD_VERSION, CBSEM_ML_METHOD_VERSION, CBSEM_MODIFICATION_INDICES_METHOD_VERSION,
    CCA_METHOD_VERSION, CFA_ML_METHOD_VERSION, CTA_PLS_METHOD_VERSION,
    CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION, CbsemMatrixCell, GSCA_ALGORITHM_VERSION,
    GSCA_METHOD_VERSION, GSCA_METHOD_VERSION_V1, IPMA_METHOD_VERSION, IPMA_PERFORMANCE_SCALE,
    MICOM_METHOD_VERSION, MICOM_METHOD_VERSION_V1, MediationAnalysis, NCA_METHOD_VERSION,
    NCA_METHOD_VERSION_V1, NcaAnalysis, PCA_METHOD_VERSION, PLS_MEDIATION_METHOD_VERSION,
    PLS_METHOD_VERSION, PLS_MGA_METHOD_VERSION, PLS_MGA_METHOD_VERSION_V1,
    PLS_MGA_PERMUTATION_METHOD_VERSION, PLS_MGA_PERMUTATION_METHOD_VERSION_V1,
    PLS_PREDICT_METHOD_VERSION, PLS_PREDICT_METHOD_VERSION_V1,
    PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION, PLS_TWO_STAGE_MODERATION_METHOD_VERSION,
    PLSC_METHOD_VERSION, PLSC_METHOD_VERSION_V1, PcaAnalysis, PlsPredictAnalysis,
    PlsPredictCvpatBenchmarkAssessment, PlsPredictErrorMetrics, PlsPredictIndicatorTarget,
    PlsResult, ProcessBootstrapAnalysis, ProcessGraphAnalysis, REGRESSION_LOGISTIC_METHOD_VERSION,
    REGRESSION_LOGISTIC_METHOD_VERSION_V1, REGRESSION_OLS_METHOD_VERSION,
    REGRESSION_PROCESS_METHOD_VERSION, REGRESSION_PROCESS_METHOD_VERSION_V1, RegressionAnalysis,
    RegressionBootstrapAnalysis, RegressionBootstrapBcaInterval, RegressionBootstrapCoefficient,
    RegressionBootstrapOddsRatio, RegressionBootstrapTest, WPLS_METHOD_VERSION,
    analyze_mediation_effects_with_tolerance, analyze_moderation, nca_analysis_matches_v2_contract,
    process_bootstrap_estimands,
};
use qpls_resampling::{
    PERMUTATION_METHOD_VERSION, PROCESS_BOOTSTRAP_ALGORITHM, PROCESS_BOOTSTRAP_INTERVAL_POLICY,
    PROCESS_BOOTSTRAP_METHOD_VERSION, PROCESS_BOOTSTRAP_STREAM_TOKEN,
    PROCESS_BOOTSTRAP_TEST_REFERENCE, PROCESS_BOOTSTRAP_VALIDATION_WITNESS_VERSION,
    PlsBootstrapResult, PlsPermutationResult, REGRESSION_BOOTSTRAP_ALGORITHM,
    REGRESSION_BOOTSTRAP_INTERVAL_POLICY, REGRESSION_BOOTSTRAP_METHOD_VERSION,
    REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION, REGRESSION_BOOTSTRAP_STREAM_TOKEN,
    REGRESSION_BOOTSTRAP_TEST_REFERENCE, REGRESSION_BOOTSTRAP_VALIDATION_WITNESS_VERSION,
    RESAMPLING_METHOD_VERSION, RESAMPLING_METHOD_VERSION_V1, RESAMPLING_METHOD_VERSION_V2,
    RESAMPLING_METHOD_VERSION_V3, STUDENTIZED_METHOD_VERSION, normal_reference_test,
    summarize_process_bootstrap_estimands, summarize_regression_bootstrap_coefficients,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal, StudentsT};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const PROJECT_ARCHIVE_VERSION: u32 = 5;
const PROJECT_ARCHIVE_VERSION_V4: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub engine_version: String,
    #[serde(default = "default_checksum_algorithm")]
    pub checksum_algorithm: String,
    pub checksums: BTreeMap<String, String>,
}

fn default_checksum_algorithm() -> String {
    "sha256".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectDocument {
    #[serde(default)]
    datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    models: Vec<ModelSpec>,
    #[serde(default)]
    recipes: Vec<AnalysisRecipe>,
    #[serde(default)]
    layouts: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    results: Vec<AnalysisResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct FutureProjectDocument {
    #[serde(default)]
    datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    models: Vec<serde_json::Value>,
    #[serde(default)]
    recipes: Vec<serde_json::Value>,
    #[serde(default)]
    layouts: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    results: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyProjectDocument {
    #[serde(default)]
    datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    models: Vec<ModelSpec>,
    #[serde(default)]
    recipes: Vec<AnalysisRecipe>,
    #[serde(default)]
    layouts: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    results: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct V3ProjectDocument {
    #[serde(default)]
    datasets: Vec<DatasetDescriptor>,
    #[serde(default)]
    models: Vec<ModelSpec>,
    #[serde(default)]
    recipes: Vec<AnalysisRecipe>,
    #[serde(default)]
    layouts: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    results: Vec<V3AnalysisResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct V3AnalysisResult {
    schema_version: u32,
    id: Uuid,
    status: RunStatus,
    provenance: V3RunProvenance,
    diagnostics: Vec<Diagnostic>,
    payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct V3RunProvenance {
    recipe_id: Uuid,
    dataset_fingerprint: String,
    method: String,
    method_version: String,
    engine_version: String,
    seed: u64,
    settings: AnalysisSettings,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub manifest: ProjectManifest,
    pub datasets: Vec<Dataset>,
    pub models: Vec<ModelSpec>,
    pub recipes: Vec<AnalysisRecipe>,
    pub layouts: BTreeMap<String, serde_json::Value>,
    pub results: Vec<AnalysisResult>,
    pub read_only: bool,
    /// Archive schema observed on load. This is runtime migration metadata and
    /// is never serialized into the archive.
    pub source_archive_version: u32,
    /// Whether an explicit save still needs to establish a current v5 primary
    /// while retaining the loaded legacy primary as its previous generation.
    /// Autosave does not change this runtime state.
    pub migration_pending: bool,
    /// Compatibility information derived from immutable stored results. These
    /// notices are deliberately kept outside `AnalysisResult::diagnostics` so
    /// opening an archive never rewrites its historical scientific record.
    pub compatibility_notices: Vec<ProjectCompatibilityNotice>,
    /// Counts of future-schema items that were checksum-verified but could not
    /// be decoded by this build. These are read-only visibility metadata and
    /// are never written back into an archive.
    pub future_unsupported: FutureUnsupportedCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCompatibilityNotice {
    pub result_id: Uuid,
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FutureUnsupportedCounts {
    pub models: usize,
    pub recipes: usize,
    pub results: usize,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            manifest: ProjectManifest {
                schema_version: PROJECT_ARCHIVE_VERSION,
                project_id: Uuid::new_v4(),
                name: name.into(),
                created_at: now,
                modified_at: now,
                engine_version: ENGINE_VERSION.into(),
                checksum_algorithm: default_checksum_algorithm(),
                checksums: BTreeMap::new(),
            },
            datasets: vec![],
            models: vec![],
            recipes: vec![],
            layouts: BTreeMap::new(),
            results: Vec::new(),
            read_only: false,
            source_archive_version: PROJECT_ARCHIVE_VERSION,
            migration_pending: false,
            compatibility_notices: Vec::new(),
            future_unsupported: FutureUnsupportedCounts::default(),
        }
    }

    /// Adopts the manifest returned by a successful explicit `save_project`
    /// call and completes any pending archive migration. Autosave callers must
    /// not call this method.
    pub fn adopt_explicit_save(&mut self, manifest: ProjectManifest) -> Result<(), ProjectError> {
        if self.read_only {
            return Err(ProjectError::ReadOnly);
        }
        if manifest.project_id != self.manifest.project_id {
            return Err(ProjectError::Invalid(
                "saved manifest project ID does not match the active project".into(),
            ));
        }
        if manifest.schema_version != PROJECT_ARCHIVE_VERSION {
            return Err(ProjectError::Invalid(format!(
                "saved manifest schema {} is not current archive schema {}",
                manifest.schema_version, PROJECT_ARCHIVE_VERSION
            )));
        }
        self.manifest = manifest;
        self.source_archive_version = PROJECT_ARCHIVE_VERSION;
        self.migration_pending = false;
        Ok(())
    }

    /// Appends a recipe and result only when the resulting project satisfies
    /// the same result contracts enforced by save and load.
    pub fn append_validated_result(
        &mut self,
        recipe: AnalysisRecipe,
        result: AnalysisResult,
    ) -> Result<(), ProjectError> {
        if self.read_only {
            return Err(ProjectError::ReadOnly);
        }
        if self.recipes.iter().any(|stored| stored.id == recipe.id) {
            return Err(ProjectError::Invalid(format!(
                "analysis recipe {} already exists; recipe IDs must be unique",
                recipe.id
            )));
        }
        if self.results.iter().any(|stored| stored.id == result.id) {
            return Err(ProjectError::Invalid(format!(
                "analysis result {} already exists; result IDs must be unique",
                result.id
            )));
        }
        if matches!(&result.payload, AnalysisPayload::Legacy { .. }) {
            return Err(ProjectError::Invalid(
                "legacy result payloads are archive-readable only and cannot be appended as new evidence"
                    .into(),
            ));
        }
        if recipe.schema_version != ANALYSIS_RECIPE_SCHEMA_VERSION {
            return Err(ProjectError::Invalid(format!(
                "historical analysis recipe schema {} is archive-readable but cannot be appended as a new result; explicitly migrate it to schema v{} first",
                recipe.schema_version, ANALYSIS_RECIPE_SCHEMA_VERSION
            )));
        }
        let recipe_errors = validate_recipe(&recipe)
            .into_iter()
            .filter(|issue| issue.severity == Severity::Error)
            .map(|issue| format!("{}: {}", issue.code, issue.message))
            .collect::<Vec<_>>();
        if !recipe_errors.is_empty() {
            return Err(ProjectError::Invalid(format!(
                "analysis recipe cannot be appended: {}",
                recipe_errors.join("; ")
            )));
        }
        if result.provenance.method == AnalysisMethod::Mga
            && result.provenance.method_version.split('+').any(|version| {
                matches!(
                    version,
                    PLS_MGA_METHOD_VERSION_V1
                        | PLS_MGA_PERMUTATION_METHOD_VERSION_V1
                        | MICOM_METHOD_VERSION_V1
                )
            })
        {
            return Err(ProjectError::Invalid(
                "historical MGA/MICOM v1 results are archive-readable but cannot be appended as new scientific evidence"
                    .into(),
            ));
        }
        if result.provenance.method == AnalysisMethod::Predict
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_PREDICT_METHOD_VERSION_V1)
        {
            return Err(ProjectError::Invalid(
                "historical PLSpredict holdout v1 results are archive-readable but cannot be appended as new scientific evidence"
                    .into(),
            ));
        }
        if result.provenance.method == AnalysisMethod::Gsca
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == GSCA_METHOD_VERSION_V1)
        {
            return Err(ProjectError::Invalid(
                "historical gsca_v1 preview results are archive-readable but cannot be appended as new GSCA evidence"
                    .into(),
            ));
        }
        if result.provenance.method == AnalysisMethod::Regression
            && result.provenance.method_version == REGRESSION_LOGISTIC_METHOD_VERSION_V1
        {
            return Err(ProjectError::Invalid(
                "historical regression_logistic_v1 results are archive-readable but cannot be appended as new scientific evidence"
                    .into(),
            ));
        }
        if result.provenance.method == AnalysisMethod::Regression
            && result.provenance.method_version == REGRESSION_PROCESS_METHOD_VERSION_V1
        {
            return Err(ProjectError::Invalid(
                "historical regression_process_v1 results are archive-readable but cannot be appended as new scientific evidence"
                    .into(),
            ));
        }
        self.recipes.push(recipe);
        self.results.push(result);
        if let Err(error) = validate_result_contracts_with_recipes(&self.results, &self.recipes) {
            self.results.pop();
            self.recipes.pop();
            return Err(error);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project is read-only because its schema is newer than this application")]
    ReadOnly,
    #[error("project archive is missing {0}")]
    MissingEntry(String),
    #[error("checksum mismatch for {0}")]
    ChecksumMismatch(String),
    #[error("project archive is invalid: {0}")]
    Invalid(String),
    #[error("project recovery failed: {0}")]
    RecoveryFailed(String),
    #[error(
        "save promotion failed ({promotion}) and restoring the original project also failed ({rollback})"
    )]
    RollbackFailed { promotion: String, rollback: String },
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("data failed: {0}")]
    Data(#[from] qpls_data::DataError),
    #[error("JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP failed: {0}")]
    Zip(#[from] zip::result::ZipError),
}

fn map_archive_integrity_error(error: ArchiveIntegrityError) -> ProjectError {
    match error {
        ArchiveIntegrityError::MissingRequiredEntry(name) => ProjectError::MissingEntry(name),
        ArchiveIntegrityError::ChecksumMismatch(name) => ProjectError::ChecksumMismatch(name),
        other => ProjectError::Invalid(other.to_string()),
    }
}

pub fn save_project(path: &Path, project: &Project) -> Result<ProjectManifest, ProjectError> {
    if project.read_only {
        return Err(ProjectError::ReadOnly);
    }
    if transaction_journal_path(path).exists() {
        recover_incomplete_save(path)?;
        if transaction_journal_path(path).exists() {
            return Err(ProjectError::RecoveryFailed(
                "a prior save is committed but its recovery identity is not yet durable; retry after the filesystem permits recovery metadata writes"
                    .into(),
            ));
        }
    }
    validate_result_contracts_with_recipes(&project.results, &project.recipes)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let document = ProjectDocument {
        datasets: project
            .datasets
            .iter()
            .map(DatasetDescriptor::from)
            .collect(),
        models: project.models.clone(),
        recipes: project.recipes.clone(),
        layouts: project.layouts.clone(),
        results: project.results.clone(),
    };
    let mut manifest = project.manifest.clone();
    manifest.schema_version = PROJECT_ARCHIVE_VERSION;
    manifest.modified_at = Utc::now();
    manifest.engine_version = ENGINE_VERSION.into();
    manifest.checksum_algorithm = default_checksum_algorithm();
    manifest.checksums.clear();

    let temporary = temporary_path(path);
    let mut temporary_guard = TemporaryArchiveGuard::new(temporary.clone());
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let project_bytes = serde_json::to_vec_pretty(&document)?;
    manifest
        .checksums
        .insert(PROJECT_ENTRY_NAME.to_owned(), sha256(&project_bytes));
    zip.start_file(PROJECT_ENTRY_NAME, options)?;
    zip.write_all(&project_bytes)?;

    // Arrow buffers are serialized, hashed, written, and dropped one dataset
    // at a time. A save therefore never retains every dataset version in an
    // additional in-memory archive map.
    for dataset in &project.datasets {
        let name = format!("data/{}.arrow", dataset.id);
        let bytes = write_arrow(&dataset.batch)?;
        manifest.checksums.insert(name.clone(), sha256(&bytes));
        zip.start_file(name, options)?;
        zip.write_all(&bytes)?;
    }

    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.finish()?.sync_all()?;

    // Validate the exact bytes that will be promoted, rather than assuming
    // successful ZIP finalization implies a readable scientific archive.
    let persisted = load_project(&temporary)?;
    if persisted.read_only
        || persisted.manifest.project_id != manifest.project_id
        || persisted.manifest.schema_version != PROJECT_ARCHIVE_VERSION
        || persisted.manifest.checksums != manifest.checksums
    {
        return Err(ProjectError::Invalid(
            "temporary archive validation did not reproduce the persisted manifest".into(),
        ));
    }

    promote_validated_archive(path, &temporary, &manifest)?;
    temporary_guard.disarm();
    Ok(manifest)
}

pub fn load_project(path: &Path) -> Result<Project, ProjectError> {
    let mut raw_archive = File::open(path)?;
    validate_raw_central_directory(&mut raw_archive, DEFAULT_ARCHIVE_LIMITS)
        .map_err(map_archive_integrity_error)?;
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS)
        .map_err(map_archive_integrity_error)?;
    let manifest_bytes = read_preflighted_entry(
        &mut archive,
        &preflight,
        archive_integrity::MANIFEST_ENTRY_NAME,
        MAX_MANIFEST_UNCOMPRESSED_BYTES,
    )
    .map_err(map_archive_integrity_error)?;
    let manifest_value: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
    let mut manifest: ProjectManifest = serde_json::from_value(manifest_value.clone())?;
    let source_archive_version = manifest.schema_version;
    if source_archive_version >= PROJECT_ARCHIVE_VERSION
        && manifest_value.get("checksum_algorithm").is_none()
    {
        return Err(ProjectError::Invalid(
            "archive schema v5 and newer must declare checksum_algorithm".into(),
        ));
    }
    if !manifest.checksum_algorithm.eq_ignore_ascii_case("sha256") {
        return Err(ProjectError::Invalid(format!(
            "unsupported archive checksum algorithm {}",
            manifest.checksum_algorithm
        )));
    }
    manifest.checksum_algorithm = default_checksum_algorithm();
    let checksums = validate_manifest_checksums(&preflight, &manifest.checksums)
        .map_err(map_archive_integrity_error)?;
    verify_archive_checksums(&mut archive, &preflight, &checksums)
        .map_err(map_archive_integrity_error)?;
    let project_bytes = read_preflighted_entry(
        &mut archive,
        &preflight,
        PROJECT_ENTRY_NAME,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES,
    )
    .map_err(map_archive_integrity_error)?;
    let (document, future, future_unsupported) = match source_archive_version {
        0 => {
            return Err(ProjectError::Invalid(
                "archive schema version 0 is unsupported".into(),
            ));
        }
        1 | 2 | 3 | PROJECT_ARCHIVE_VERSION_V4 | PROJECT_ARCHIVE_VERSION => (
            migrate_document(source_archive_version, &project_bytes)?,
            false,
            FutureUnsupportedCounts::default(),
        ),
        _ => {
            let future = read_future_document(&project_bytes)?;
            (future.document, true, future.unsupported)
        }
    };
    let expected_entries = expected_project_entries(document.datasets.iter().map(|item| item.id))
        .map_err(map_archive_integrity_error)?;
    if future {
        let available = checksums.entry_names();
        let missing = expected_entries
            .difference(&available)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(ProjectError::Invalid(format!(
                "future archive is missing compatible project entries: {}",
                missing.join(", ")
            )));
        }
    } else {
        validate_expected_project_entries(&checksums, &expected_entries)
            .map_err(map_archive_integrity_error)?;
    }
    let compatibility_notices = compatibility_notices(&document.results);
    if !future {
        manifest.schema_version = PROJECT_ARCHIVE_VERSION;
    }
    let mut datasets = Vec::with_capacity(document.datasets.len());
    for descriptor in document.datasets {
        let name = format!("data/{}.arrow", descriptor.id);
        let bytes = read_preflighted_entry(
            &mut archive,
            &preflight,
            &name,
            DEFAULT_ARCHIVE_LIMITS.max_entry_uncompressed_bytes,
        )
        .map_err(map_archive_integrity_error)?;
        datasets.push(dataset_from_descriptor(descriptor, &bytes)?);
    }
    Ok(Project {
        manifest,
        datasets,
        models: document.models,
        recipes: document.recipes,
        layouts: document.layouts,
        results: document.results,
        read_only: future,
        source_archive_version,
        migration_pending: !future && source_archive_version < PROJECT_ARCHIVE_VERSION,
        compatibility_notices,
        future_unsupported,
    })
}

fn migrate_document(schema_version: u32, bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    match schema_version {
        0 => Err(ProjectError::Invalid(
            "archive schema version 0 is unsupported".into(),
        )),
        1 | 2 => migrate_legacy_document(bytes),
        3 => migrate_v3_document(bytes),
        PROJECT_ARCHIVE_VERSION_V4 => migrate_v4_document(bytes),
        PROJECT_ARCHIVE_VERSION => read_current_document(bytes),
        version => Err(ProjectError::Invalid(format!(
            "archive schema version {version} requires the future-schema read-only loader"
        ))),
    }
}

fn read_current_document(bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    let document: ProjectDocument = serde_json::from_slice(bytes)?;
    validate_result_contracts_with_recipes(&document.results, &document.recipes)?;
    Ok(document)
}

fn migrate_v4_document(bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    // V4 and v5 deliberately share the collection wire shape. Migration is an
    // identity parse: no IDs, timestamps, ordering, diagnostics, payloads, or
    // other scientific values are rewritten.
    read_current_document(bytes)
}

struct FutureDocumentRead {
    document: ProjectDocument,
    unsupported: FutureUnsupportedCounts,
}

fn read_future_document(bytes: &[u8]) -> Result<FutureDocumentRead, ProjectError> {
    // Future archives are decoded collection-by-collection. Compatible items
    // remain viewable/exportable, while unknown model/recipe/result variants
    // are omitted rather than making the whole verified archive unreadable.
    // The returned project remains read-only and is never resaved.
    let future: FutureProjectDocument = serde_json::from_slice(bytes)?;
    let model_count = future.models.len();
    let models = future
        .models
        .into_iter()
        .filter_map(|item| serde_json::from_value::<ModelSpec>(item).ok())
        .collect::<Vec<_>>();
    let recipe_count = future.recipes.len();
    let recipes = future
        .recipes
        .into_iter()
        .filter_map(|item| serde_json::from_value::<AnalysisRecipe>(item).ok())
        .collect::<Vec<_>>();
    let result_count = future.results.len();
    let results = future
        .results
        .into_iter()
        .filter_map(|item| serde_json::from_value::<AnalysisResult>(item).ok())
        .collect::<Vec<_>>();
    let document = ProjectDocument {
        datasets: future.datasets,
        models,
        recipes,
        layouts: future.layouts,
        results,
    };
    validate_unique_analysis_ids(&document.results, &document.recipes)?;
    Ok(FutureDocumentRead {
        unsupported: FutureUnsupportedCounts {
            models: model_count - document.models.len(),
            recipes: recipe_count - document.recipes.len(),
            results: result_count - document.results.len(),
        },
        document,
    })
}

fn migrate_v3_document(bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    let legacy: V3ProjectDocument = serde_json::from_slice(bytes)?;
    let results = legacy
        .results
        .into_iter()
        .map(|result| {
            let method = migrate_method(&result.provenance.method);
            let payload = if method == AnalysisMethod::PlsPm {
                match (
                    result.payload.get("estimation").cloned(),
                    result.payload.get("assessment").cloned(),
                ) {
                    (Some(estimation), Some(assessment)) => AnalysisPayload::PlsPmV1 {
                        estimation,
                        assessment,
                    },
                    _ => AnalysisPayload::Legacy {
                        value: result.payload,
                    },
                }
            } else {
                AnalysisPayload::Legacy {
                    value: result.payload,
                }
            };
            AnalysisResult {
                schema_version: result.schema_version,
                id: result.id,
                status: result.status,
                provenance: RunProvenance {
                    recipe_id: result.provenance.recipe_id,
                    dataset_fingerprint: result.provenance.dataset_fingerprint,
                    method,
                    method_version: result.provenance.method_version,
                    engine_version: result.provenance.engine_version,
                    seed: result.provenance.seed,
                    settings: result.provenance.settings,
                    started_at: result.provenance.started_at,
                    completed_at: result.provenance.completed_at,
                },
                diagnostics: result.diagnostics,
                payload,
            }
        })
        .collect();
    let document = ProjectDocument {
        datasets: legacy.datasets,
        models: legacy.models,
        recipes: legacy.recipes,
        layouts: legacy.layouts,
        results,
    };
    validate_unique_analysis_ids(&document.results, &document.recipes)?;
    Ok(document)
}

fn migrate_legacy_document(bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    let legacy: LegacyProjectDocument = serde_json::from_slice(bytes)?;
    let results = legacy
        .results
        .into_iter()
        .enumerate()
        .map(|(index, payload)| {
            let recipe = legacy.recipes.get(index);
            let method_version = payload
                .get("method_version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("legacy_unknown")
                .to_owned();
            AnalysisResult {
                schema_version: RESULT_SCHEMA_VERSION,
                id: Uuid::new_v4(),
                status: RunStatus::Completed,
                provenance: RunProvenance {
                    recipe_id: recipe.map(|recipe| recipe.id).unwrap_or_else(Uuid::nil),
                    dataset_fingerprint: recipe
                        .map(|recipe| recipe.dataset_fingerprint.clone())
                        .unwrap_or_default(),
                    method: recipe
                        .map(|recipe| recipe.settings.method)
                        .unwrap_or(AnalysisMethod::Legacy),
                    method_version,
                    engine_version: "legacy_unknown".into(),
                    seed: recipe.map(|recipe| recipe.settings.seed).unwrap_or_default(),
                    settings: recipe
                        .map(|recipe| recipe.settings.clone())
                        .unwrap_or_else(AnalysisSettings::default),
                    started_at: recipe
                        .map(|recipe| recipe.created_at)
                        .unwrap_or_else(Utc::now),
                    completed_at: Utc::now(),
                },
                diagnostics: vec![Diagnostic {
                    code: "migration.legacy_result".into(),
                    level: DiagnosticLevel::Warning,
                    message: "Result was migrated from an untyped legacy project; provenance may be incomplete".into(),
                }],
                payload: AnalysisPayload::Legacy { value: payload },
            }
        })
        .collect();
    let document = ProjectDocument {
        datasets: legacy.datasets,
        models: legacy.models,
        recipes: legacy.recipes,
        layouts: legacy.layouts,
        results,
    };
    validate_unique_analysis_ids(&document.results, &document.recipes)?;
    Ok(document)
}

fn migrate_method(method: &str) -> AnalysisMethod {
    match method {
        "pls_pm" => AnalysisMethod::PlsPm,
        "bootstrap" => AnalysisMethod::Bootstrap,
        "plsc" => AnalysisMethod::Plsc,
        "wpls" => AnalysisMethod::Wpls,
        "cca" => AnalysisMethod::Cca,
        "cta_pls" => AnalysisMethod::CtaPls,
        "endogeneity" => AnalysisMethod::Endogeneity,
        "nonlinear_effects" => AnalysisMethod::NonlinearEffects,
        "moderated_mediation" => AnalysisMethod::ModeratedMediation,
        "predict" => AnalysisMethod::Predict,
        "mga" => AnalysisMethod::Mga,
        "ipma" => AnalysisMethod::Ipma,
        "cbsem" => AnalysisMethod::Cbsem,
        "pca" => AnalysisMethod::Pca,
        "gsca" => AnalysisMethod::Gsca,
        "regression" => AnalysisMethod::Regression,
        "nca" => AnalysisMethod::Nca,
        _ => AnalysisMethod::Legacy,
    }
}

#[cfg(test)]
fn validate_result_contracts(results: &[AnalysisResult]) -> Result<(), ProjectError> {
    validate_result_contracts_internal(results, &[], false)
}

fn validate_result_contracts_with_recipes(
    results: &[AnalysisResult],
    recipes: &[AnalysisRecipe],
) -> Result<(), ProjectError> {
    validate_result_contracts_internal(results, recipes, true)
}

fn process_v2_pls_root_keys_are_declared(estimation: &serde_json::Value) -> bool {
    if estimation
        .pointer("/regression/process/method_version")
        .and_then(serde_json::Value::as_str)
        != Some(REGRESSION_PROCESS_METHOD_VERSION)
        || estimation
            .pointer("/regression/process/graph_v2")
            .map_or(true, serde_json::Value::is_null)
    {
        return true;
    }
    let Some(root) = estimation.as_object() else {
        return false;
    };
    if root.contains_key("mediation") || root.contains_key("moderation") {
        return false;
    }
    root.keys().all(|key| {
        matches!(
            key.as_str(),
            "method_version"
                | "converged"
                | "iterations"
                | "used_observations"
                | "omitted_observations"
                | "transforms"
                | "construct_scores"
                | "outer_estimates"
                | "paths"
                | "control_estimates"
                | "effects"
                | "plsc"
                | "endogeneity"
                | "nonlinear_effects"
                | "moderated_mediation"
                | "cta_pls"
                | "wpls"
                | "cca"
                | "predict"
                | "segmentation"
                | "mga"
                | "micom"
                | "mga_permutation"
                | "fimix"
                | "ipma"
                | "cbsem"
                | "pca"
                | "regression"
                | "nca"
                | "gsca"
                | "r_squared"
                | "warnings"
        )
    })
}

fn validate_unique_analysis_ids(
    results: &[AnalysisResult],
    recipes: &[AnalysisRecipe],
) -> Result<(), ProjectError> {
    let mut recipe_ids = BTreeSet::new();
    for recipe in recipes {
        if !recipe_ids.insert(recipe.id) {
            return Err(ProjectError::Invalid(format!(
                "analysis recipe {} is duplicated; recipe IDs must be unique",
                recipe.id
            )));
        }
    }
    let mut result_ids = BTreeSet::new();
    for result in results {
        if !result_ids.insert(result.id) {
            return Err(ProjectError::Invalid(format!(
                "analysis result {} is duplicated; result IDs must be unique",
                result.id
            )));
        }
    }
    Ok(())
}

fn executable_pls_payload_method_version(method: AnalysisMethod) -> Option<&'static str> {
    match method {
        AnalysisMethod::PlsPm => Some(PLS_METHOD_VERSION),
        AnalysisMethod::Plsc => Some(PLSC_METHOD_VERSION),
        AnalysisMethod::Wpls => Some(WPLS_METHOD_VERSION),
        AnalysisMethod::Cca => Some(CCA_METHOD_VERSION),
        AnalysisMethod::CtaPls => Some(CTA_PLS_METHOD_VERSION),
        AnalysisMethod::Predict => Some(PLS_PREDICT_METHOD_VERSION),
        AnalysisMethod::Mga => Some(PLS_MGA_METHOD_VERSION),
        AnalysisMethod::Ipma => Some(IPMA_METHOD_VERSION),
        AnalysisMethod::Cbsem => Some(CBSEM_ML_METHOD_VERSION),
        AnalysisMethod::Nca => Some(NCA_METHOD_VERSION),
        AnalysisMethod::Pca => Some(PCA_METHOD_VERSION),
        AnalysisMethod::Regression => Some(REGRESSION_OLS_METHOD_VERSION),
        AnalysisMethod::Gsca => Some(GSCA_METHOD_VERSION),
        _ => None,
    }
}

fn is_supported_plsc_method_version(version: &str) -> bool {
    matches!(version, PLSC_METHOD_VERSION | PLSC_METHOD_VERSION_V1)
}

fn metadata_value<'a>(
    recipe: &'a AnalysisRecipe,
    primary: &str,
    alternate: &str,
) -> Option<&'a str> {
    recipe
        .metadata
        .get(primary)
        .or_else(|| recipe.metadata.get(alternate))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn close_enough(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 1e-10 * left.abs().max(right.abs()).max(1.0)
}

fn validate_cca_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    if recipe.settings.method != AnalysisMethod::Cca
        || !matches!(
            &recipe.settings.weighting_scheme,
            WeightingScheme::Path | WeightingScheme::Factor
        )
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || recipe.model.constructs.len() < 2
        || recipe.model.paths.is_empty()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || recipe.model.constructs.iter().any(|construct| {
            construct.id.trim().is_empty()
                || construct.mode != MeasurementMode::Reflective
                || construct.indicators.is_empty()
                || construct
                    .indicators
                    .iter()
                    .any(|indicator| indicator.trim().is_empty())
        })
    {
        return false;
    }

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    let indicator_ids = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let indicator_count = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len())
        .sum::<usize>();
    if construct_ids.len() != recipe.model.constructs.len()
        || indicator_ids.len() != indicator_count
    {
        return false;
    }

    let mut recipe_paths = BTreeSet::new();
    for path in &recipe.model.paths {
        if path.source == path.target
            || !construct_ids.contains(path.source.as_str())
            || !construct_ids.contains(path.target.as_str())
            || !recipe_paths.insert((path.source.as_str(), path.target.as_str()))
        {
            return false;
        }
    }
    let mut estimation_paths = BTreeSet::new();
    for path in &estimation.paths {
        if !path.coefficient.is_finite()
            || !estimation_paths.insert((path.source.as_str(), path.target.as_str()))
        {
            return false;
        }
    }
    if estimation_paths != recipe_paths {
        return false;
    }

    let expected_provenance_version = format!(
        "{PLS_METHOD_VERSION}+{CCA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{assessment_method_version}"
    );
    if result.provenance.method_version != expected_provenance_version
        || estimation.method_version != CCA_METHOD_VERSION
        || !estimation.control_estimates.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
        || estimation.mediation.method_version != PLS_MEDIATION_METHOD_VERSION
        || estimation.moderation.method_version != PLS_TWO_STAGE_MODERATION_METHOD_VERSION
        || !estimation.moderation.estimates.is_empty()
        || !estimation.moderation.warnings.is_empty()
    {
        return false;
    }

    let Some(cca) = estimation.cca.as_ref() else {
        return false;
    };
    if cca.method_version != CCA_METHOD_VERSION
        || cca.method_version != estimation.method_version
        || cca.model != "recursive_standardized_composite_path_model_v1"
        || !cca.max_absolute_residual.is_finite()
        || cca.max_absolute_residual < 0.0
    {
        return false;
    }

    let construct_order = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<Vec<_>>();
    let mut expected_pairs = BTreeSet::new();
    for right in 1..construct_order.len() {
        for left in 0..right {
            let first = construct_order[left];
            let second = construct_order[right];
            expected_pairs.insert(if first <= second {
                (first.to_string(), second.to_string())
            } else {
                (second.to_string(), first.to_string())
            });
        }
    }

    let mut actual_pairs = BTreeSet::new();
    let mut computed_max = 0.0_f64;
    for row in &cca.correlations {
        if row.left == row.right
            || !construct_ids.contains(row.left.as_str())
            || !construct_ids.contains(row.right.as_str())
            || !row.observed.is_finite()
            || !row.reproduced.is_finite()
            || !row.residual.is_finite()
            || !row.absolute_residual.is_finite()
            || row.absolute_residual < 0.0
            || !close_enough(row.residual, row.observed - row.reproduced)
            || !close_enough(row.absolute_residual, row.residual.abs())
        {
            return false;
        }
        let pair = if row.left <= row.right {
            (row.left.clone(), row.right.clone())
        } else {
            (row.right.clone(), row.left.clone())
        };
        if !actual_pairs.insert(pair) {
            return false;
        }
        computed_max = computed_max.max(row.absolute_residual);
    }

    actual_pairs == expected_pairs && close_enough(cca.max_absolute_residual, computed_max)
}

fn validate_cta_pls_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    const COVARIANCE_VERSION: &str = "sample_covariance_of_preprocessed_indicators_v1";
    const RESULT_WARNING: &str = "CTA-PLS tetrad bootstrap/permutation inference is outside the validated QuickPLS v1.2.3 descriptive scope.";
    const ESTIMATION_WARNING: &str = "CTA-PLS tetrad diagnostics are validated for the documented QuickPLS v1.2.3 descriptive tetrad scope; bootstrap/permutation tetrad decision rules remain unsupported.";
    const PAIRINGS: [&str; 3] = [
        "ab_cd_minus_ac_bd",
        "ac_bd_minus_ad_bc",
        "ad_bc_minus_ab_cd",
    ];

    let Some(recipe) = recipe else {
        return false;
    };
    let eligible_constructs = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| construct.indicators.len() >= 4)
        .collect::<Vec<_>>();
    if recipe.settings.method != AnalysisMethod::CtaPls
        || !matches!(
            &recipe.settings.weighting_scheme,
            WeightingScheme::Path | WeightingScheme::Factor
        )
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || recipe.settings.workers != 1
        || eligible_constructs.is_empty()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
    {
        return false;
    }

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    let indicator_ids = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let indicator_count = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len())
        .sum::<usize>();
    if construct_ids.len() != recipe.model.constructs.len()
        || indicator_ids.len() != indicator_count
        || recipe.model.constructs.iter().any(|construct| {
            construct.id.trim().is_empty()
                || construct.indicators.is_empty()
                || construct
                    .indicators
                    .iter()
                    .any(|indicator| indicator.trim().is_empty())
        })
    {
        return false;
    }

    let recipe_paths = recipe
        .model
        .paths
        .iter()
        .map(|path| (path.source.as_str(), path.target.as_str()))
        .collect::<BTreeSet<_>>();
    let estimation_paths = estimation
        .paths
        .iter()
        .filter(|path| path.coefficient.is_finite())
        .map(|path| (path.source.as_str(), path.target.as_str()))
        .collect::<BTreeSet<_>>();
    if recipe_paths.len() != recipe.model.paths.len()
        || estimation_paths.len() != estimation.paths.len()
        || recipe_paths != estimation_paths
    {
        return false;
    }

    let expected_provenance_version = format!(
        "{PLS_METHOD_VERSION}+{CTA_PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{assessment_method_version}"
    );
    if result.provenance.method_version != expected_provenance_version
        || estimation.method_version != CTA_PLS_METHOD_VERSION
        || !estimation.control_estimates.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
        || estimation.mediation.method_version != PLS_MEDIATION_METHOD_VERSION
        || estimation.moderation.method_version != PLS_TWO_STAGE_MODERATION_METHOD_VERSION
        || !estimation.moderation.estimates.is_empty()
        || !estimation.moderation.warnings.is_empty()
        || !estimation
            .warnings
            .iter()
            .any(|warning| warning == ESTIMATION_WARNING)
    {
        return false;
    }

    let Some(cta) = estimation.cta_pls.as_ref() else {
        return false;
    };
    if cta.method_version != CTA_PLS_METHOD_VERSION
        || cta.covariance != COVARIANCE_VERSION
        || cta.warnings.len() != 1
        || cta.warnings[0] != RESULT_WARNING
    {
        return false;
    }

    let mut expected = BTreeSet::new();
    for construct in &eligible_constructs {
        let indicators = &construct.indicators;
        for a in 0..indicators.len() - 3 {
            for b in a + 1..indicators.len() - 2 {
                for c in b + 1..indicators.len() - 1 {
                    for d in c + 1..indicators.len() {
                        for pairing in PAIRINGS {
                            expected.insert((
                                construct.id.clone(),
                                indicators[a].clone(),
                                indicators[b].clone(),
                                indicators[c].clone(),
                                indicators[d].clone(),
                                pairing.to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut actual = BTreeSet::new();
    let mut values_by_quadruple =
        BTreeMap::<(String, String, String, String, String), BTreeMap<String, f64>>::new();
    let mut maxima = BTreeMap::<String, f64>::new();
    for row in &cta.estimates {
        if !row.tetrad.is_finite()
            || !row.absolute_tetrad.is_finite()
            || row.absolute_tetrad < 0.0
            || !close_enough(row.absolute_tetrad, row.tetrad.abs())
            || !PAIRINGS.contains(&row.pairing.as_str())
        {
            return false;
        }
        let identity = (
            row.construct.clone(),
            row.indicator_a.clone(),
            row.indicator_b.clone(),
            row.indicator_c.clone(),
            row.indicator_d.clone(),
            row.pairing.clone(),
        );
        if !actual.insert(identity) {
            return false;
        }
        let quadruple = (
            row.construct.clone(),
            row.indicator_a.clone(),
            row.indicator_b.clone(),
            row.indicator_c.clone(),
            row.indicator_d.clone(),
        );
        if values_by_quadruple
            .entry(quadruple)
            .or_default()
            .insert(row.pairing.clone(), row.tetrad)
            .is_some()
        {
            return false;
        }
        maxima
            .entry(row.construct.clone())
            .and_modify(|maximum| *maximum = maximum.max(row.absolute_tetrad))
            .or_insert(row.absolute_tetrad);
    }
    if actual != expected {
        return false;
    }
    for values in values_by_quadruple.values() {
        if values.len() != PAIRINGS.len()
            || !close_enough(values.values().copied().sum::<f64>(), 0.0)
        {
            return false;
        }
    }
    if cta.max_absolute_tetrad_by_construct.len() != eligible_constructs.len() {
        return false;
    }
    eligible_constructs.iter().all(|construct| {
        let actual = cta.max_absolute_tetrad_by_construct.get(&construct.id);
        let expected = maxima.get(&construct.id);
        actual
            .zip(expected)
            .is_some_and(|(actual, expected)| close_enough(*actual, *expected))
    })
}

fn cbsem_matrix_from_cells(
    cells: &[CbsemMatrixCell],
    indicator_names: &[&str],
) -> Option<Vec<Vec<f64>>> {
    let size = indicator_names.len();
    if size == 0 || cells.len() != size * size {
        return None;
    }
    let mut matrix = vec![vec![0.0; size]; size];
    for (index, cell) in cells.iter().enumerate() {
        let row = index / size;
        let column = index % size;
        if cell.row != indicator_names[row]
            || cell.column != indicator_names[column]
            || !cell.value.is_finite()
        {
            return None;
        }
        matrix[row][column] = cell.value;
    }
    Some(matrix)
}

fn validate_cbsem_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    const SCOPE_WARNING: &str = "CB-SEM/CFA ML v1 is validated for the documented QuickPLS v1.2.4 raw-data single-group reflective ML scope; bootstrap, unrestricted multigroup/invariance, robust, ordinal, and FIML estimators remain experimental or unsupported.";

    let Some(recipe) = recipe else {
        return false;
    };
    let (model_type, configured_scope_valid) =
        if recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION {
            let Some(qpls_core::MethodConfig::Cbsem {
                model_type,
                estimator,
                input,
                mean_structure,
                bootstrap_samples,
                group_column,
                invariance_steps,
            }) = recipe.method_config.as_ref()
            else {
                return false;
            };
            (
                match model_type {
                    qpls_core::CbsemModelType::Cfa => "cfa".to_string(),
                    qpls_core::CbsemModelType::Sem => "sem".to_string(),
                },
                *estimator == qpls_core::CbsemEstimator::Ml
                    && *input == qpls_core::CbsemInput::Raw
                    && !*mean_structure
                    && *bootstrap_samples == 0
                    && group_column.is_none()
                    && invariance_steps.is_empty(),
            )
        } else {
            let Some(model_type) = metadata_value(recipe, "cbsem_model_type", "cbsem.model_type")
            else {
                return false;
            };
            let metadata_is_absent_or = |key: &str, accepted: &str| {
                recipe
                    .metadata
                    .get(key)
                    .is_none_or(|value| value.trim().eq_ignore_ascii_case(accepted))
            };
            let no_cbsem_bootstrap = recipe
                .metadata
                .get("cbsem_bootstrap_samples")
                .is_none_or(|value| value.trim().parse::<usize>().ok() == Some(0));
            (
                model_type.to_string(),
                metadata_value(recipe, "cbsem_input", "cbsem.input") == Some("raw")
                    && metadata_is_absent_or("cbsem_estimator", "ml")
                    && metadata_is_absent_or("cbsem_mean_structure", "false")
                    && no_cbsem_bootstrap
                    && !recipe.metadata.contains_key("cbsem_group_column")
                    && !recipe.metadata.contains_key("cbsem_invariance_steps"),
            )
        };
    let expected_method_version = match model_type.as_str() {
        "cfa" => CFA_ML_METHOD_VERSION,
        "sem" => CBSEM_ML_METHOD_VERSION,
        _ => return false,
    };
    if recipe.settings.method != AnalysisMethod::Cbsem
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || recipe.settings.workers != 1
        || !configured_scope_valid
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || recipe.model.constructs.is_empty()
        || (model_type == "cfa" && !recipe.model.paths.is_empty())
        || (model_type == "sem" && recipe.model.paths.is_empty())
    {
        return false;
    }

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    let indicator_names = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let unique_indicators = indicator_names.iter().copied().collect::<BTreeSet<_>>();
    if construct_ids.len() != recipe.model.constructs.len()
        || unique_indicators.len() != indicator_names.len()
        || recipe.model.constructs.iter().any(|construct| {
            construct.id.trim().is_empty()
                || construct.mode != MeasurementMode::Reflective
                || construct.indicators.len() < 2
                || construct
                    .indicators
                    .iter()
                    .any(|indicator| indicator.trim().is_empty())
        })
    {
        return false;
    }
    let mut unique_paths = BTreeSet::new();
    for path in &recipe.model.paths {
        if path.source == path.target
            || !construct_ids.contains(path.source.as_str())
            || !construct_ids.contains(path.target.as_str())
            || !unique_paths.insert((path.source.as_str(), path.target.as_str()))
        {
            return false;
        }
    }

    let expected_provenance_version = format!(
        "{PLS_METHOD_VERSION}+{expected_method_version}+{CBSEM_FIT_METHOD_VERSION}+{CBSEM_MODIFICATION_INDICES_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{assessment_method_version}"
    );
    let Some(cbsem) = estimation.cbsem.as_ref() else {
        return false;
    };
    if result.provenance.method_version != expected_provenance_version
        || estimation.method_version != expected_method_version
        || cbsem.method_version != expected_method_version
        || cbsem.model_type != model_type
        || cbsem.estimator != "ml"
        || cbsem.input != "raw"
        || cbsem.mean_structure
        || !cbsem.converged
        || cbsem.iterations == 0
        || cbsem.iterations > recipe.settings.max_iterations
        || !cbsem.objective.is_finite()
        || cbsem.objective < 0.0
        || !cbsem.gradient_norm.is_finite()
        || cbsem.gradient_norm < 0.0
        || cbsem.sample_size != estimation.used_observations
        || cbsem.sample_size < 10
        || cbsem.bootstrap.is_some()
        || cbsem.multigroup.is_some()
        || cbsem.warnings.len() != cbsem.diagnostics.len() + 1
        || cbsem.warnings.first().map(String::as_str) != Some(SCOPE_WARNING)
        || cbsem.warnings[1..] != cbsem.diagnostics
        || !estimation.control_estimates.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
    {
        return false;
    }
    if cbsem.diagnostics.iter().any(|diagnostic| {
        diagnostic != "sample covariance is not positive definite"
            && diagnostic != "implied covariance is not positive definite"
            && !diagnostic.starts_with("nonpositive variance estimate for ")
    }) {
        return false;
    }

    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<BTreeSet<_>>();
    let mut expected_parameters = Vec::new();
    for construct in &recipe.model.constructs {
        for (index, indicator) in construct.indicators.iter().enumerate() {
            expected_parameters.push((
                format!("{}=~{indicator}", construct.id),
                "loading".to_string(),
                construct.id.clone(),
                indicator.clone(),
                index == 0,
            ));
        }
        expected_parameters.push((
            format!("{}~~{}", construct.id, construct.id),
            "latent_variance".to_string(),
            construct.id.clone(),
            construct.id.clone(),
            false,
        ));
        for indicator in &construct.indicators {
            expected_parameters.push((
                format!("{indicator}~~{indicator}"),
                "residual_variance".to_string(),
                indicator.clone(),
                indicator.clone(),
                false,
            ));
        }
    }
    for left in 0..recipe.model.constructs.len() {
        for right in left + 1..recipe.model.constructs.len() {
            let left_id = &recipe.model.constructs[left].id;
            let right_id = &recipe.model.constructs[right].id;
            if !endogenous.contains(left_id.as_str()) && !endogenous.contains(right_id.as_str()) {
                expected_parameters.push((
                    format!("{left_id}~~{right_id}"),
                    "latent_covariance".to_string(),
                    left_id.clone(),
                    right_id.clone(),
                    false,
                ));
            }
        }
    }
    for path in &recipe.model.paths {
        expected_parameters.push((
            format!("{}~{}", path.target, path.source),
            "structural_path".to_string(),
            path.target.clone(),
            path.source.clone(),
            false,
        ));
    }
    if cbsem.parameters.len() != expected_parameters.len()
        || cbsem.standardized.len() != expected_parameters.len()
    {
        return false;
    }
    for ((parameter, standardized), expected) in cbsem
        .parameters
        .iter()
        .zip(&cbsem.standardized)
        .zip(&expected_parameters)
    {
        let (name, kind, lhs, rhs, fixed) = expected;
        if parameter.name != *name
            || parameter.kind != *kind
            || parameter.lhs != *lhs
            || parameter.rhs != *rhs
            || parameter.fixed != *fixed
            || !parameter.estimate.is_finite()
            || parameter.warning.is_some()
            || standardized.name != *name
            || standardized.kind != *kind
            || standardized.lhs != *lhs
            || standardized.rhs != *rhs
            || !standardized.std_lv.is_finite()
            || !standardized.std_all.is_finite()
        {
            return false;
        }
        if *fixed {
            if !close_enough(parameter.estimate, 1.0)
                || parameter.standard_error.is_some()
                || parameter.z_statistic.is_some()
                || parameter.p_value_two_sided.is_some()
            {
                return false;
            }
        } else {
            let Some(standard_error) = parameter.standard_error else {
                return false;
            };
            if !standard_error.is_finite() || standard_error <= 0.0 {
                return false;
            }
            let (expected_z, expected_p) =
                normal_reference_test(parameter.estimate, standard_error);
            if parameter
                .z_statistic
                .zip(expected_z)
                .is_none_or(|(actual, expected)| !close_enough(actual, expected))
                || parameter
                    .p_value_two_sided
                    .zip(expected_p)
                    .is_none_or(|(actual, expected)| !close_enough(actual, expected))
            {
                return false;
            }
        }
    }

    let Some(implied) = cbsem_matrix_from_cells(&cbsem.implied_covariance, &indicator_names) else {
        return false;
    };
    let Some(residual) = cbsem_matrix_from_cells(&cbsem.residual_covariance, &indicator_names)
    else {
        return false;
    };
    let Some(residual_correlation) =
        cbsem_matrix_from_cells(&cbsem.residual_correlation, &indicator_names)
    else {
        return false;
    };
    let size = indicator_names.len();
    let mut sample = vec![vec![0.0; size]; size];
    let mut srmr_sum = 0.0;
    let mut srmr_count = 0usize;
    for row in 0..size {
        for column in 0..size {
            if !close_enough(implied[row][column], implied[column][row])
                || !close_enough(residual[row][column], residual[column][row])
                || !close_enough(
                    residual_correlation[row][column],
                    residual_correlation[column][row],
                )
            {
                return false;
            }
            sample[row][column] = implied[row][column] + residual[row][column];
        }
        if implied[row][row] <= 0.0 || sample[row][row] <= 0.0 {
            return false;
        }
    }
    for row in 0..size {
        for column in 0..size {
            let denominator = (sample[row][row].abs() * sample[column][column].abs()).sqrt();
            let expected = if denominator > f64::EPSILON {
                residual[row][column] / denominator
            } else {
                0.0
            };
            if !close_enough(residual_correlation[row][column], expected) {
                return false;
            }
            if column <= row {
                srmr_sum += expected.powi(2);
                srmr_count += 1;
            }
        }
    }

    let fit = &cbsem.fit;
    let free_parameters = expected_parameters
        .iter()
        .filter(|parameter| !parameter.4)
        .count();
    let expected_df = (size * (size + 1) / 2) as i64 - free_parameters as i64;
    let expected_chi_square = (cbsem.sample_size as f64 * cbsem.objective).max(0.0);
    let expected_p = if expected_df > 0 {
        let Ok(distribution) = ChiSquared::new(expected_df as f64) else {
            return false;
        };
        Some((1.0 - distribution.cdf(expected_chi_square)).clamp(0.0, 1.0))
    } else {
        None
    };
    let model_noncentrality = (expected_chi_square - expected_df as f64).max(0.0);
    let baseline_noncentrality =
        (fit.baseline_chi_square - fit.baseline_degrees_of_freedom as f64).max(f64::EPSILON);
    let expected_cfi = Some((1.0 - model_noncentrality / baseline_noncentrality).clamp(0.0, 1.0));
    let expected_tli = if expected_df > 0 && fit.baseline_degrees_of_freedom > 0 {
        let model_ratio = expected_chi_square / expected_df as f64;
        let baseline_ratio = fit.baseline_chi_square / fit.baseline_degrees_of_freedom as f64;
        Some((baseline_ratio - model_ratio) / (baseline_ratio - 1.0))
    } else {
        None
    };
    let expected_rmsea = if expected_df > 0 && cbsem.sample_size > 1 {
        Some((model_noncentrality / (expected_df as f64 * cbsem.sample_size as f64)).sqrt())
    } else {
        None
    };
    let option_matches = |actual: Option<f64>, expected: Option<f64>| match (actual, expected) {
        (Some(actual), Some(expected)) => close_enough(actual, expected),
        (None, None) => true,
        _ => false,
    };
    let expected_srmr = (srmr_sum / srmr_count.max(1) as f64).sqrt();
    if fit.method_version != CBSEM_FIT_METHOD_VERSION
        || fit.degrees_of_freedom != expected_df
        || fit.baseline_degrees_of_freedom != (size * size.saturating_sub(1) / 2) as i64
        || !fit.baseline_chi_square.is_finite()
        || fit.baseline_chi_square < 0.0
        || !close_enough(fit.chi_square, expected_chi_square)
        || !option_matches(fit.p_value, expected_p)
        || !option_matches(fit.cfi, expected_cfi)
        || !option_matches(fit.tli, expected_tli)
        || !option_matches(fit.rmsea, expected_rmsea)
        || !option_matches(
            fit.rmsea_ci_lower,
            expected_rmsea.map(|value| (value * 0.80).max(0.0)),
        )
        || !option_matches(
            fit.rmsea_ci_upper,
            expected_rmsea.map(|value| value * 1.20 + 1e-12),
        )
        || !close_enough(fit.srmr, expected_srmr)
        || !close_enough(
            fit.aic,
            cbsem.sample_size as f64 * cbsem.objective + 2.0 * free_parameters as f64,
        )
        || !close_enough(
            fit.bic,
            cbsem.sample_size as f64 * cbsem.objective
                + (cbsem.sample_size as f64).ln() * free_parameters as f64,
        )
    {
        return false;
    }

    let assigned = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct.id.as_str(), indicator.as_str()))
        })
        .collect::<BTreeSet<_>>();
    let mut expected_modification_indices = Vec::new();
    for row in 0..size {
        for column in row + 1..size {
            let correlation = residual_correlation[row][column];
            expected_modification_indices.push((
                "residual_covariance",
                indicator_names[row],
                indicator_names[column],
                correlation * correlation * cbsem.sample_size as f64,
                correlation,
            ));
        }
    }
    for construct in &recipe.model.constructs {
        for indicator in &indicator_names {
            if !assigned.contains(&(construct.id.as_str(), *indicator)) {
                expected_modification_indices.push((
                    "cross_loading",
                    construct.id.as_str(),
                    *indicator,
                    0.0,
                    0.0,
                ));
            }
        }
    }
    expected_modification_indices.sort_by(|left, right| {
        right
            .3
            .total_cmp(&left.3)
            .then(left.1.cmp(right.1))
            .then(left.2.cmp(right.2))
    });
    expected_modification_indices.truncate(50);
    if cbsem.modification_indices.len() != expected_modification_indices.len() {
        return false;
    }
    for (actual, expected) in cbsem
        .modification_indices
        .iter()
        .zip(expected_modification_indices)
    {
        if actual.method_version != CBSEM_MODIFICATION_INDICES_METHOD_VERSION
            || actual.kind != expected.0
            || actual.lhs != expected.1
            || actual.rhs != expected.2
            || !close_enough(actual.modification_index, expected.3)
            || actual
                .expected_parameter_change
                .is_none_or(|value| !close_enough(value, expected.4))
        {
            return false;
        }
    }
    true
}

fn validate_ipma_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    if recipe.settings.method != AnalysisMethod::Ipma
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || recipe.settings.workers != 1
        || (recipe.settings.confidence_level - 0.95).abs() > 1e-12
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
    {
        return false;
    }
    let Ok(expected_targets) = resolve_ipma_targets(recipe) else {
        return false;
    };
    let expected_provenance_version = format!(
        "{PLS_METHOD_VERSION}+{IPMA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{assessment_method_version}"
    );
    if result.provenance.method_version != expected_provenance_version
        || estimation.method_version != IPMA_METHOD_VERSION
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
    {
        return false;
    }
    let Some(ipma) = estimation.ipma.as_ref() else {
        return false;
    };
    if ipma.method_version != IPMA_METHOD_VERSION
        || ipma.method_version != estimation.method_version
        || ipma.performance_scale != IPMA_PERFORMANCE_SCALE
        || ipma.targets != expected_targets
        || ipma.warnings.is_empty()
        || ipma
            .warnings
            .iter()
            .any(|warning| !estimation.warnings.contains(warning))
    {
        return false;
    }

    let effect_index = estimation
        .effects
        .iter()
        .map(|effect| {
            (
                (effect.source.as_str(), effect.target.as_str()),
                effect.total,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected_construct_rows = expected_targets
        .iter()
        .flat_map(|target| {
            let predecessors = ipma_predecessor_constructs(recipe, target)
                .into_iter()
                .collect::<BTreeSet<_>>();
            recipe
                .model
                .constructs
                .iter()
                .filter(move |construct| predecessors.contains(&construct.id))
                .map(move |construct| (target.as_str(), construct.id.as_str()))
        })
        .collect::<Vec<_>>();
    if ipma.constructs.len() != expected_construct_rows.len() {
        return false;
    }
    let mut construct_performance = BTreeMap::<&str, f64>::new();
    for (row, (target, construct)) in ipma.constructs.iter().zip(expected_construct_rows) {
        let Some(scores) = estimation.construct_scores.get(construct) else {
            return false;
        };
        let expected_importance = effect_index
            .get(&(construct, target))
            .copied()
            .unwrap_or(0.0);
        let score_mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let minimum = scores.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let expected_performance = if !minimum.is_finite()
            || !maximum.is_finite()
            || (maximum - minimum).abs() <= f64::EPSILON
        {
            50.0
        } else {
            scores
                .iter()
                .map(|value| 100.0 * (value - minimum) / (maximum - minimum))
                .sum::<f64>()
                / scores.len() as f64
        };
        if row.target != target
            || row.construct != construct
            || !row.importance.is_finite()
            || !row.performance.is_finite()
            || !(0.0..=100.0).contains(&row.performance)
            || !row.score_mean.is_finite()
            || !close_enough(row.importance, expected_importance)
            || !close_enough(row.performance, expected_performance)
            || !close_enough(row.score_mean, score_mean)
        {
            return false;
        }
        if let Some(previous) = construct_performance.insert(construct, row.performance)
            && !close_enough(previous, row.performance)
        {
            return false;
        }
    }

    let loading_index = estimation
        .outer_estimates
        .iter()
        .map(|row| {
            (
                (row.construct.as_str(), row.indicator.as_str()),
                row.loading,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected_indicator_rows = expected_targets
        .iter()
        .flat_map(|target| {
            let predecessors = ipma_predecessor_constructs(recipe, target)
                .into_iter()
                .collect::<BTreeSet<_>>();
            recipe
                .model
                .constructs
                .iter()
                .filter(move |construct| predecessors.contains(&construct.id))
                .flat_map(move |construct| {
                    construct.indicators.iter().map(move |indicator| {
                        (target.as_str(), construct.id.as_str(), indicator.as_str())
                    })
                })
        })
        .collect::<Vec<_>>();
    if ipma.indicators.len() != expected_indicator_rows.len() {
        return false;
    }
    let mut indicator_values = BTreeMap::<&str, (f64, f64)>::new();
    for (row, (target, construct, indicator)) in ipma.indicators.iter().zip(expected_indicator_rows)
    {
        let expected_importance = effect_index
            .get(&(construct, target))
            .copied()
            .unwrap_or(0.0);
        let Some(expected_loading) = loading_index.get(&(construct, indicator)).copied() else {
            return false;
        };
        if row.target != target
            || row.construct != construct
            || row.indicator != indicator
            || !row.construct_importance.is_finite()
            || !row.loading.is_finite()
            || !row.performance.is_finite()
            || !(0.0..=100.0).contains(&row.performance)
            || !row.score_mean.is_finite()
            || row.score_mean.abs() > 1e-10
            || !close_enough(row.construct_importance, expected_importance)
            || !close_enough(row.loading, expected_loading)
        {
            return false;
        }
        if let Some((previous_performance, previous_mean)) =
            indicator_values.insert(indicator, (row.performance, row.score_mean))
            && (!close_enough(previous_performance, row.performance)
                || !close_enough(previous_mean, row.score_mean))
        {
            return false;
        }
    }
    true
}

const GSCA_NOT_APPLICABLE_ASSESSMENT_VERSION: &str = "assessment_not_applicable_v1";
const GSCA_NOT_APPLICABLE_ASSESSMENT_WARNING: &str =
    "PLS assessment is not applicable to GSCA ALS component-model estimation.";
const NCA_NOT_APPLICABLE_ASSESSMENT_VERSION: &str = "assessment_not_applicable_v1";
const NCA_NOT_APPLICABLE_ASSESSMENT_WARNING: &str =
    "PLS assessment is not applicable to standalone raw-data analyses.";
const PCA_NOT_APPLICABLE_ASSESSMENT_VERSION: &str = "assessment_not_applicable_v1";
const PCA_NOT_APPLICABLE_ASSESSMENT_WARNING: &str =
    "PLS assessment is not applicable to standalone raw-data analyses.";
const REGRESSION_NOT_APPLICABLE_ASSESSMENT_VERSION: &str = "assessment_not_applicable_v1";
const REGRESSION_NOT_APPLICABLE_ASSESSMENT_WARNING: &str =
    "PLS assessment is not applicable to standalone raw-data analyses.";
const REGRESSION_NOT_APPLICABLE_ASSESSMENT_WARNING_V08: &str =
    "PLS assessment is not applicable to standalone v0.8 methods.";

fn validate_gsca_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let Some(gsca) = estimation.gsca.as_ref() else {
        return false;
    };
    if recipe.settings.method != AnalysisMethod::Gsca
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || recipe.settings.workers != 1
        || recipe.settings.max_iterations != 3_000
        || (recipe.settings.tolerance - 1e-7).abs() > f64::EPSILON
        || recipe.model.constructs.len() < 2
        || recipe.model.paths.is_empty()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || result.provenance.method_version != GSCA_METHOD_VERSION
        || estimation.method_version != GSCA_METHOD_VERSION
        || gsca.method_version != GSCA_METHOD_VERSION
        || gsca.algorithm != GSCA_ALGORITHM_VERSION
        || assessment_method_version != GSCA_NOT_APPLICABLE_ASSESSMENT_VERSION
        || !estimation.converged
        || !gsca.converged
        || estimation.iterations != gsca.iterations
        || estimation.used_observations != gsca.observations
        || estimation.used_observations < 3
        || gsca.iterations == 0
        || gsca.iterations > recipe.settings.max_iterations
        || !close_enough(gsca.stop_criterion, recipe.settings.tolerance)
        || !gsca.final_change.is_finite()
        || gsca.final_change < 0.0
        || gsca.final_change > recipe.settings.tolerance + 1e-12
        || !gsca.objective.is_finite()
        || gsca.objective < 0.0
        || !gsca.fit.is_finite()
        || gsca.fit > 1.0 + 1e-10
        || !gsca.measurement_fit.is_finite()
        || gsca.measurement_fit > 1.0 + 1e-10
        || !gsca.structural_fit.is_finite()
        || gsca.structural_fit > 1.0 + 1e-10
        || !gsca.adjusted_fit.is_finite()
        || !gsca.gfi.is_finite()
        || gsca.gfi > 1.0 + 1e-10
        || !gsca.srmr.is_finite()
        || gsca.srmr < 0.0
        || !gsca.covariance_discrepancy.is_finite()
        || gsca.covariance_discrepancy < 0.0
        || !gsca.covariance_sample_total.is_finite()
        || gsca.covariance_sample_total <= 0.0
        || !gsca.standardized_residual_sum.is_finite()
        || gsca.standardized_residual_sum < 0.0
        || !gsca.bootstrap_intervals.is_empty()
        || gsca.warnings.len() != 1
        || !gsca.warnings[0].contains("GSCA ALS v2 is bounded")
        || !estimation.warnings.contains(&gsca.warnings[0])
        || !estimation.control_estimates.is_empty()
        || !estimation.effects.is_empty()
        || !estimation.mediation.estimates.is_empty()
        || !estimation.moderation.estimates.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
    {
        return false;
    }

    let connected = recipe
        .model
        .paths
        .iter()
        .flat_map(|path| [path.source.as_str(), path.target.as_str()])
        .collect::<BTreeSet<_>>();
    if recipe
        .model
        .constructs
        .iter()
        .any(|construct| !connected.contains(construct.id.as_str()))
    {
        return false;
    }
    let indicator_rows = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct.id.as_str(), indicator.as_str(), &construct.mode))
        })
        .collect::<Vec<_>>();
    if indicator_rows.is_empty()
        || estimation.transforms.len() != indicator_rows.len()
        || estimation.outer_estimates.len() != indicator_rows.len()
        || gsca.weights != estimation.outer_estimates
        || gsca.loadings != estimation.outer_estimates
        || gsca.weights.len() != indicator_rows.len()
        || gsca.loadings.len() != indicator_rows.len()
    {
        return false;
    }
    let mut measurement_residual = 0.0;
    let mut weight_sums = BTreeMap::<&str, f64>::new();
    for (index, (expected_construct, expected_indicator, mode)) in indicator_rows.iter().enumerate()
    {
        let transform = &estimation.transforms[index];
        let outer = &estimation.outer_estimates[index];
        if transform.indicator != *expected_indicator
            || !transform.mean.is_finite()
            || !transform.scale.is_finite()
            || transform.scale <= 0.0
            || outer.construct != *expected_construct
            || outer.indicator != *expected_indicator
            || !outer.weight.is_finite()
            || !outer.loading.is_finite()
            || outer.loading.abs() > 1.0 + 1e-10
        {
            return false;
        }
        *weight_sums.entry(expected_construct).or_default() += outer.weight;
        measurement_residual += match mode {
            MeasurementMode::Reflective => 1.0 - outer.loading * outer.loading,
            MeasurementMode::Formative => 1.0,
        };
    }
    if weight_sums.values().any(|sum| *sum < -1e-12) {
        return false;
    }

    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.id.as_str())
        .collect::<BTreeSet<_>>();
    if estimation.construct_scores.len() != construct_ids.len()
        || estimation
            .construct_scores
            .keys()
            .any(|construct| !construct_ids.contains(construct.as_str()))
    {
        return false;
    }
    for scores in estimation.construct_scores.values() {
        if scores.len() != estimation.used_observations
            || scores.iter().any(|score| !score.is_finite())
        {
            return false;
        }
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores
            .iter()
            .map(|score| (score - mean).powi(2))
            .sum::<f64>()
            / (scores.len() - 1) as f64;
        if mean.abs() > 1e-10 || !close_enough(variance, 1.0) {
            return false;
        }
    }

    if estimation.paths != gsca.paths
        || estimation.paths.len() != recipe.model.paths.len()
        || estimation
            .paths
            .iter()
            .zip(&recipe.model.paths)
            .any(|(actual, expected)| {
                actual.source != expected.source
                    || actual.target != expected.target
                    || !actual.coefficient.is_finite()
            })
        || estimation.r_squared != gsca.r_squared
    {
        return false;
    }
    let endogenous = recipe
        .model
        .paths
        .iter()
        .map(|path| path.target.as_str())
        .collect::<BTreeSet<_>>();
    if estimation.r_squared.len() != endogenous.len()
        || estimation.r_squared.iter().any(|(construct, value)| {
            !endogenous.contains(construct.as_str()) || !value.is_finite() || *value > 1.0 + 1e-10
        })
    {
        return false;
    }
    let structural_residual = recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            estimation
                .r_squared
                .get(&construct.id)
                .map_or(1.0, |r_squared| 1.0 - r_squared)
        })
        .sum::<f64>();
    let observed = indicator_rows.len() as f64;
    let constructs = recipe.model.constructs.len() as f64;
    let expected_objective = measurement_residual + structural_residual;
    let expected_fit = 1.0 - expected_objective / (observed + constructs);
    let expected_measurement_fit = 1.0 - measurement_residual / observed;
    let expected_structural_fit = 1.0 - structural_residual / constructs;
    let expected_free_parameters = recipe
        .model
        .constructs
        .iter()
        .map(|construct| construct.indicators.len().saturating_sub(1))
        .sum::<usize>()
        + recipe
            .model
            .constructs
            .iter()
            .filter(|construct| construct.mode == MeasurementMode::Reflective)
            .map(|construct| construct.indicators.len())
            .sum::<usize>()
        + recipe.model.paths.len();
    let null_degrees = estimation.used_observations * indicator_rows.len();
    if null_degrees <= expected_free_parameters {
        return false;
    }
    let expected_adjusted_fit = 1.0
        - (1.0 - expected_fit) * null_degrees as f64
            / (null_degrees - expected_free_parameters) as f64;
    let expected_gfi = 1.0 - gsca.covariance_discrepancy / gsca.covariance_sample_total;
    let expected_srmr = (2.0 * gsca.standardized_residual_sum
        / (indicator_rows.len() * (indicator_rows.len() + 1)) as f64)
        .sqrt();
    close_enough(gsca.objective, expected_objective)
        && close_enough(gsca.fit, expected_fit)
        && close_enough(gsca.measurement_fit, expected_measurement_fit)
        && close_enough(gsca.structural_fit, expected_structural_fit)
        && gsca.free_parameters == expected_free_parameters
        && close_enough(gsca.adjusted_fit, expected_adjusted_fit)
        && close_enough(gsca.gfi, expected_gfi)
        && close_enough(gsca.srmr, expected_srmr)
}

fn validate_legacy_gsca_v1_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let Some(gsca) = estimation.gsca.as_ref() else {
        return false;
    };
    if recipe.settings.method != AnalysisMethod::Gsca
        || estimation.method_version != GSCA_METHOD_VERSION_V1
        || gsca.method_version != GSCA_METHOD_VERSION_V1
        || !result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == GSCA_METHOD_VERSION_V1)
        || assessment_method_version != GSCA_NOT_APPLICABLE_ASSESSMENT_VERSION
        || gsca
            .weights
            .iter()
            .any(|row| !row.weight.is_finite() || !row.loading.is_finite())
        || gsca
            .loadings
            .iter()
            .any(|row| !row.weight.is_finite() || !row.loading.is_finite())
        || gsca.paths.iter().any(|path| !path.coefficient.is_finite())
        || gsca.r_squared.values().any(|value| !value.is_finite())
        || !gsca.fit.is_finite()
        || !gsca.adjusted_fit.is_finite()
        || !gsca.gfi.is_finite()
    {
        return false;
    }
    gsca.bootstrap_intervals.len() == gsca.paths.len()
        && gsca
            .bootstrap_intervals
            .iter()
            .zip(&gsca.paths)
            .all(|(interval, path)| {
                interval.parameter == format!("{}->{}", path.source, path.target)
                    && close_enough(interval.original, path.coefficient)
                    && close_enough(interval.lower_percentile, path.coefficient - 0.05)
                    && close_enough(interval.upper_percentile, path.coefficient + 0.05)
            })
}

fn validate_pca_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let variables = metadata_value(recipe, "pca_variables", "pca.variables")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let unique_variables = variables.iter().collect::<BTreeSet<_>>();
    let component_rule = recipe
        .metadata
        .get("pca_component_rule")
        .map(|value| value.trim())
        .unwrap_or("kaiser");
    let fixed_components = recipe
        .metadata
        .get("pca_components")
        .and_then(|value| value.trim().parse::<usize>().ok());
    let variance_threshold = recipe
        .metadata
        .get("pca_variance_threshold")
        .and_then(|value| value.trim().parse::<f64>().ok());
    if recipe.settings.method != AnalysisMethod::Pca
        || result.provenance.method_version != PCA_METHOD_VERSION
        || result.provenance.settings != recipe.settings
        || result.provenance.dataset_fingerprint != recipe.dataset_fingerprint
        || assessment_method_version != PCA_NOT_APPLICABLE_ASSESSMENT_VERSION
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || !recipe.model.constructs.is_empty()
        || !recipe.model.paths.is_empty()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || variables.len() < 2
        || variables.len() > 50
        || unique_variables.len() != variables.len()
        || !matches!(component_rule, "kaiser" | "fixed" | "variance_threshold")
        || component_rule == "fixed"
            && fixed_components
                .is_none_or(|components| components == 0 || components > variables.len().min(50))
        || component_rule == "variance_threshold"
            && variance_threshold.is_none_or(|threshold| {
                !threshold.is_finite() || !(0.01..=0.999).contains(&threshold)
            })
        || estimation.method_version != PCA_METHOD_VERSION
        || estimation.used_observations < 3
        || !estimation.transforms.is_empty()
        || !estimation.construct_scores.is_empty()
        || !estimation.outer_estimates.is_empty()
        || !estimation.paths.is_empty()
        || !estimation.control_estimates.is_empty()
        || !estimation.effects.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
        || !estimation.r_squared.is_empty()
    {
        return false;
    }
    let Some(pca) = estimation.pca.as_ref() else {
        return false;
    };
    validate_pca_analysis_contract(
        pca,
        &variables,
        component_rule,
        fixed_components,
        variance_threshold,
        estimation.used_observations,
    ) && pca.warnings == estimation.warnings
}

fn validate_pca_analysis_contract(
    pca: &PcaAnalysis,
    variables: &[String],
    component_rule: &str,
    fixed_components: Option<usize>,
    variance_threshold: Option<f64>,
    used_observations: usize,
) -> bool {
    if pca.method_version != PCA_METHOD_VERSION
        || pca.component_rule != component_rule
        || pca.variables != variables
        || pca.observations != used_observations
        || pca.retained_components == 0
        || pca.retained_components != pca.components.len()
        || pca.retained_components > variables.len().min(used_observations.saturating_sub(1))
        || pca.loadings.len() != variables.len() * pca.retained_components
        || pca.scores.len() != used_observations * pca.retained_components
        || pca.warnings.is_empty()
    {
        return false;
    }
    if component_rule == "fixed"
        && fixed_components.is_none_or(|requested| pca.retained_components > requested)
    {
        return false;
    }
    if component_rule == "variance_threshold"
        && variance_threshold.is_some_and(|threshold| {
            pca.components
                .last()
                .is_some_and(|component| component.cumulative_variance + 1e-10 < threshold)
                && pca.retained_components
                    < variables.len().min(used_observations.saturating_sub(1))
        })
    {
        return false;
    }

    let mut cumulative = 0.0;
    for (index, component) in pca.components.iter().enumerate() {
        cumulative += component.explained_variance;
        if component.component != format!("PC{}", index + 1)
            || !component.eigenvalue.is_finite()
            || component.eigenvalue <= 0.0
            || !component.explained_variance.is_finite()
            || component.explained_variance <= 0.0
            || !component.cumulative_variance.is_finite()
            || !close_enough(
                component.explained_variance,
                component.eigenvalue / variables.len() as f64,
            )
            || !close_enough(component.cumulative_variance, cumulative)
            || index > 0 && component.eigenvalue > pca.components[index - 1].eigenvalue + 1e-10
            || component_rule == "kaiser" && component.eigenvalue < 1.0 - 1e-10
        {
            return false;
        }
    }

    for (component_index, component) in pca.components.iter().enumerate() {
        let start = component_index * variables.len();
        let rows = &pca.loadings[start..start + variables.len()];
        let mut weight_norm = 0.0;
        let mut orientation_index = 0usize;
        let mut orientation_magnitude = -1.0f64;
        for (variable_index, (row, variable)) in rows.iter().zip(variables).enumerate() {
            if row.variable != *variable
                || row.component != component.component
                || !row.loading.is_finite()
                || !row.weight.is_finite()
                || !close_enough(row.loading, row.weight * component.eigenvalue.sqrt())
            {
                return false;
            }
            weight_norm += row.weight * row.weight;
            if row.weight.abs() > orientation_magnitude {
                orientation_magnitude = row.weight.abs();
                orientation_index = variable_index;
            }
        }
        if !close_enough(weight_norm, 1.0) || rows[orientation_index].weight < -1e-12 {
            return false;
        }
        let score_start = component_index * used_observations;
        for (observation, score) in pca.scores[score_start..score_start + used_observations]
            .iter()
            .enumerate()
        {
            if score.observation != observation
                || score.component != component.component
                || !score.score.is_finite()
            {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessPersistenceContract {
    Mediation {
        x: String,
        mediator: String,
    },
    Moderation {
        x: String,
        moderator: String,
    },
    ModeratedMediation {
        x: String,
        mediator: String,
        moderator: String,
    },
    Graph {
        focal_predictor: String,
        paths: Vec<qpls_core::ProcessPathConfig>,
        moderators: Vec<qpls_core::ProcessModeratorConfig>,
        moderations: Vec<qpls_core::ProcessModerationConfig>,
    },
}

impl ProcessPersistenceContract {
    fn model(&self) -> &'static str {
        match self {
            Self::Mediation { .. } => "mediation",
            Self::Moderation { .. } => "moderation",
            Self::ModeratedMediation { .. } => "moderated_mediation",
            Self::Graph { .. } => "graph",
        }
    }

    fn variables_are_bound(&self, predictors: &[String], outcome: &str) -> bool {
        let contains = |value: &str| predictors.iter().any(|predictor| predictor == value);
        match self {
            Self::Mediation { x, mediator } => {
                x != mediator
                    && x != outcome
                    && mediator != outcome
                    && contains(x)
                    && contains(mediator)
            }
            Self::Moderation { x, moderator } => {
                x != moderator
                    && x != outcome
                    && moderator != outcome
                    && contains(x)
                    && contains(moderator)
            }
            Self::ModeratedMediation {
                x,
                mediator,
                moderator,
            } => {
                let unique = [x.as_str(), mediator.as_str(), moderator.as_str(), outcome]
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                unique.len() == 4 && contains(x) && contains(mediator) && contains(moderator)
            }
            Self::Graph {
                focal_predictor,
                paths,
                moderators,
                moderations,
            } => {
                contains(focal_predictor)
                    && paths.iter().all(|path| {
                        path.from != path.to
                            && (contains(&path.from) || path.from == outcome)
                            && (contains(&path.to) || path.to == outcome)
                    })
                    && moderators
                        .iter()
                        .all(|moderator| contains(&moderator.variable))
                    && moderations.iter().all(|moderation| {
                        paths
                            .iter()
                            .any(|path| path.from == moderation.from && path.to == moderation.to)
                    })
            }
        }
    }

    fn method_version(&self) -> &'static str {
        match self {
            Self::Graph { .. } => REGRESSION_PROCESS_METHOD_VERSION,
            _ => REGRESSION_PROCESS_METHOD_VERSION_V1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RegressionPersistenceKind {
    Ols,
    Logistic,
    Process(ProcessPersistenceContract),
}

impl RegressionPersistenceKind {
    fn method_version(&self) -> &'static str {
        match self {
            Self::Ols => REGRESSION_OLS_METHOD_VERSION,
            Self::Logistic => REGRESSION_LOGISTIC_METHOD_VERSION,
            Self::Process(process) => process.method_version(),
        }
    }

    fn scope_warning(&self, method_version: &str) -> &'static str {
        match self {
            Self::Ols => {
                "OLS regression v1 is validated for the documented QuickPLS v1.2 OLS scope; unsupported shapes remain blocked."
            }
            Self::Logistic if method_version == REGRESSION_LOGISTIC_METHOD_VERSION_V1 => {
                "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, and Firth-corrected models remain unsupported."
            }
            Self::Logistic => {
                "Logistic regression v2 is validated for the documented QuickPLS binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, categorical auto-encoding, and Firth-corrected models remain unsupported."
            }
            Self::Process(ProcessPersistenceContract::Graph { .. }) => {
                "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates."
            }
            Self::Process(_) => {
                "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope; moderated mediation and the full Hayes model catalogue remain experimental."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegressionRecipeContract {
    outcome: String,
    predictors: Vec<String>,
    controls: Vec<String>,
    kind: RegressionPersistenceKind,
    bootstrap: bool,
    current_typed: bool,
}

fn regression_recipe_contract(recipe: &AnalysisRecipe) -> Option<RegressionRecipeContract> {
    if recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION {
        let qpls_core::MethodConfig::Regression {
            outcome,
            predictors,
            controls,
            model,
            bootstrap,
        } = recipe.method_config.as_ref()?
        else {
            return None;
        };
        let kind = match model {
            qpls_core::RegressionModelConfig::Ols { robust_se } => {
                if *robust_se != qpls_core::RobustStandardError::Hc3 {
                    return None;
                }
                RegressionPersistenceKind::Ols
            }
            qpls_core::RegressionModelConfig::Logistic => RegressionPersistenceKind::Logistic,
            qpls_core::RegressionModelConfig::Process { relationship } => {
                let process = match relationship {
                    qpls_core::ProcessRelationshipConfig::Mediation { x, mediator } => {
                        ProcessPersistenceContract::Mediation {
                            x: x.clone(),
                            mediator: mediator.clone(),
                        }
                    }
                    qpls_core::ProcessRelationshipConfig::Moderation { x, moderator } => {
                        ProcessPersistenceContract::Moderation {
                            x: x.clone(),
                            moderator: moderator.clone(),
                        }
                    }
                    qpls_core::ProcessRelationshipConfig::ModeratedMediation {
                        x,
                        mediator,
                        moderator,
                    } => ProcessPersistenceContract::ModeratedMediation {
                        x: x.clone(),
                        mediator: mediator.clone(),
                        moderator: moderator.clone(),
                    },
                    qpls_core::ProcessRelationshipConfig::Graph {
                        focal_predictor,
                        paths,
                        moderators,
                        moderations,
                        ..
                    } => ProcessPersistenceContract::Graph {
                        focal_predictor: focal_predictor.clone(),
                        paths: paths.clone(),
                        moderators: moderators.clone(),
                        moderations: moderations.clone(),
                    },
                };
                RegressionPersistenceKind::Process(process)
            }
        };
        return Some(RegressionRecipeContract {
            outcome: outcome.trim().to_string(),
            predictors: predictors
                .iter()
                .map(|value| value.trim().to_string())
                .collect(),
            controls: controls
                .iter()
                .map(|value| value.trim().to_string())
                .collect(),
            kind,
            bootstrap: bootstrap.is_some(),
            current_typed: true,
        });
    }

    let outcome = recipe
        .metadata
        .get("regression_outcome")?
        .trim()
        .to_string();
    let predictors = metadata_value(recipe, "regression_predictors", "regression.predictors")
        .map(csv_values)
        .unwrap_or_default();
    let controls = metadata_value(recipe, "regression_controls", "regression.controls")
        .map(csv_values)
        .unwrap_or_default();
    let regression_type = recipe
        .metadata
        .get("regression_type")
        .map(|value| value.trim())
        .unwrap_or("ols");
    let kind = match regression_type {
        "ols" => {
            if recipe.metadata.get("robust_se").map(|value| value.trim()) != Some("hc3") {
                return None;
            }
            RegressionPersistenceKind::Ols
        }
        "logistic" => RegressionPersistenceKind::Logistic,
        "process" => {
            let x = recipe
                .metadata
                .get("process_x")
                .map(|value| value.trim().to_string())
                .or_else(|| predictors.first().cloned())?;
            let process = match recipe
                .metadata
                .get("process_model")
                .map(|value| value.trim())
                .unwrap_or("mediation")
            {
                "mediation" => ProcessPersistenceContract::Mediation {
                    x,
                    mediator: recipe.metadata.get("process_m")?.trim().to_string(),
                },
                "moderation" => ProcessPersistenceContract::Moderation {
                    x,
                    moderator: recipe.metadata.get("process_w")?.trim().to_string(),
                },
                "moderated_mediation" => ProcessPersistenceContract::ModeratedMediation {
                    x,
                    mediator: recipe.metadata.get("process_m")?.trim().to_string(),
                    moderator: recipe.metadata.get("process_w")?.trim().to_string(),
                },
                _ => return None,
            };
            RegressionPersistenceKind::Process(process)
        }
        _ => return None,
    };
    Some(RegressionRecipeContract {
        outcome,
        predictors,
        controls,
        kind,
        bootstrap: false,
        current_typed: false,
    })
}

fn validate_regression_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let Some(contract) = regression_recipe_contract(recipe) else {
        return false;
    };
    let mut variables = vec![contract.outcome.as_str()];
    variables.extend(contract.predictors.iter().map(String::as_str));
    variables.extend(contract.controls.iter().map(String::as_str));
    let unique_variables = variables.iter().copied().collect::<BTreeSet<_>>();
    let preprocessing_valid =
        if contract.current_typed || matches!(contract.kind, RegressionPersistenceKind::Ols) {
            recipe.settings.preprocessing == Preprocessing::Unstandardized
        } else {
            matches!(
                recipe.settings.preprocessing,
                Preprocessing::Standardized | Preprocessing::Unstandardized
            )
        };
    let process_variables_valid = match &contract.kind {
        RegressionPersistenceKind::Process(process) => {
            process.variables_are_bound(&contract.predictors, &contract.outcome)
        }
        _ => true,
    };
    let base_method_version = match &contract.kind {
        RegressionPersistenceKind::Logistic if contract.current_typed => {
            REGRESSION_LOGISTIC_METHOD_VERSION
        }
        RegressionPersistenceKind::Logistic
            if !contract.current_typed
                && result.provenance.method_version == REGRESSION_LOGISTIC_METHOD_VERSION_V1 =>
        {
            REGRESSION_LOGISTIC_METHOD_VERSION_V1
        }
        RegressionPersistenceKind::Logistic => return false,
        _ => contract.kind.method_version(),
    };
    let process_graph = matches!(
        contract.kind,
        RegressionPersistenceKind::Process(ProcessPersistenceContract::Graph { .. })
    );
    let expected_method_version = if contract.bootstrap && process_graph {
        format!("{base_method_version}+{PROCESS_BOOTSTRAP_METHOD_VERSION}")
    } else if contract.bootstrap {
        format!("{base_method_version}+{REGRESSION_BOOTSTRAP_METHOD_VERSION}")
    } else {
        base_method_version.to_string()
    };
    let bootstrap_settings_valid = if contract.bootstrap {
        contract.current_typed
            && matches!(
                &contract.kind,
                RegressionPersistenceKind::Ols
                    | RegressionPersistenceKind::Logistic
                    | RegressionPersistenceKind::Process(ProcessPersistenceContract::Graph { .. })
            )
            && (99..=10_000).contains(&recipe.settings.bootstrap_samples)
            && (1..=64).contains(&recipe.settings.workers)
    } else {
        recipe.settings.bootstrap_samples == 0
            && (!contract.current_typed || recipe.settings.workers == 1)
    };
    if recipe.settings.method != AnalysisMethod::Regression
        || contract.outcome.is_empty()
        || contract.predictors.is_empty()
        || variables.iter().any(|value| value.is_empty())
        || unique_variables.len() != variables.len()
        || !process_variables_valid
        || result.provenance.method_version != expected_method_version
        || result.provenance.settings != recipe.settings
        || result.provenance.dataset_fingerprint != recipe.dataset_fingerprint
        || assessment_method_version != REGRESSION_NOT_APPLICABLE_ASSESSMENT_VERSION
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || !preprocessing_valid
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || !bootstrap_settings_valid
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || (recipe.settings.confidence_level - 0.95).abs() > 1e-12
        || !recipe.model.constructs.is_empty()
        || !recipe.model.paths.is_empty()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || estimation.method_version != base_method_version
        || !estimation.converged
        || estimation.iterations != 0
        || estimation.used_observations <= contract.predictors.len() + contract.controls.len() + 1
        || !estimation.transforms.is_empty()
        || !estimation.construct_scores.is_empty()
        || !estimation.outer_estimates.is_empty()
        || !estimation.paths.is_empty()
        || !estimation.control_estimates.is_empty()
        || !estimation.effects.is_empty()
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
        || !estimation.r_squared.is_empty()
        || estimation.warnings.first().map(String::as_str)
            != Some(contract.kind.scope_warning(base_method_version))
        || (process_graph
            && (estimation.warnings.len() != 2
                || estimation.warnings[1]
                    != "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected."))
        || (!process_graph && estimation.warnings.len() != 1)
    {
        return false;
    }
    let Some(regression) = estimation.regression.as_ref() else {
        return false;
    };
    let analysis_valid = match &contract.kind {
        RegressionPersistenceKind::Ols => validate_linear_regression_analysis_contract(
            regression,
            base_method_version,
            "ols",
            &contract.outcome,
            &contract.predictors,
            &contract.controls,
            estimation.used_observations,
            recipe.settings.confidence_level,
            false,
        ),
        RegressionPersistenceKind::Logistic => validate_logistic_analysis_contract(
            regression,
            base_method_version,
            &contract.outcome,
            &contract.predictors,
            &contract.controls,
            estimation.used_observations,
            estimation.omitted_observations,
            recipe.settings.confidence_level,
        ),
        RegressionPersistenceKind::Process(process) => {
            if process_graph {
                regression.method_version == base_method_version
                    && regression.regression_type == "process"
                    && regression.outcome == contract.outcome
                    && regression.predictors == contract.predictors
                    && regression.controls == contract.controls
                    && regression.observations == estimation.used_observations
                    && regression.coefficients.is_empty()
                    && regression.fit.is_none()
                    && regression.predictions.is_empty()
                    && regression.logistic.is_none()
                    && validate_process_analysis_contract(
                        regression,
                        process,
                        estimation.omitted_observations,
                    )
            } else {
                validate_linear_regression_analysis_contract(
                    regression,
                    base_method_version,
                    "process",
                    &contract.outcome,
                    &contract.predictors,
                    &contract.controls,
                    estimation.used_observations,
                    recipe.settings.confidence_level,
                    true,
                ) && validate_process_analysis_contract(
                    regression,
                    process,
                    estimation.omitted_observations,
                )
            }
        }
    };
    let bootstrap_valid =
        if let RegressionPersistenceKind::Process(ProcessPersistenceContract::Graph { .. }) =
            &contract.kind
        {
            regression.bootstrap.is_none()
                && regression
                    .process
                    .as_ref()
                    .and_then(|process| process.graph_v2.as_ref())
                    .is_some_and(
                        |graph| match (contract.bootstrap, graph.bootstrap.as_ref()) {
                            (true, Some(bootstrap)) => validate_process_bootstrap_contract(
                                bootstrap,
                                graph,
                                &recipe.settings,
                            ),
                            (false, None) => true,
                            _ => false,
                        },
                    )
        } else {
            match (contract.bootstrap, regression.bootstrap.as_ref()) {
                (true, Some(bootstrap)) => validate_regression_bootstrap_contract(
                    bootstrap,
                    regression,
                    &contract.kind,
                    &recipe.settings,
                ),
                (false, None) => true,
                _ => false,
            }
        };
    analysis_valid && bootstrap_valid && regression.warnings == estimation.warnings
}

fn validate_regression_bootstrap_contract(
    bootstrap: &RegressionBootstrapAnalysis,
    regression: &RegressionAnalysis,
    kind: &RegressionPersistenceKind,
    settings: &AnalysisSettings,
) -> bool {
    if bootstrap.method_version != REGRESSION_BOOTSTRAP_METHOD_VERSION
        || bootstrap.algorithm != REGRESSION_BOOTSTRAP_ALGORITHM
        || bootstrap.confidence_level.to_bits() != 0.95_f64.to_bits()
        || bootstrap.alternative != "two_sided"
        || bootstrap.interval_policy != REGRESSION_BOOTSTRAP_INTERVAL_POLICY
        || bootstrap.test_reference != REGRESSION_BOOTSTRAP_TEST_REFERENCE
        || bootstrap.test_tolerance_policy
            != qpls_resampling::REGRESSION_BOOTSTRAP_TEST_TOLERANCE_POLICY
        || bootstrap.requested_replicates != settings.bootstrap_samples
        || bootstrap.usable_replicates as usize + bootstrap.failed_replicates.len()
            != bootstrap.requested_replicates as usize
        || bootstrap.minimum_usable_fraction.to_bits()
            != REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION.to_bits()
        || bootstrap.usable_replicates
            < ((bootstrap.requested_replicates as f64
                * REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION)
                .ceil() as u32)
                .max(2)
        || bootstrap.jackknife_cases < 3
        || bootstrap.usable_jackknife_cases > bootstrap.jackknife_cases
        || bootstrap.seed != settings.seed
        || bootstrap.workers != settings.workers
        || bootstrap.stream_token != REGRESSION_BOOTSTRAP_STREAM_TOKEN
        || bootstrap.coefficients.len() != regression.coefficients.len()
    {
        return false;
    }
    let failed_indices = bootstrap
        .failed_replicates
        .iter()
        .map(|failure| failure.replicate_index)
        .collect::<BTreeSet<_>>();
    if failed_indices.len() != bootstrap.failed_replicates.len()
        || bootstrap.failed_replicates.iter().any(|failure| {
            failure.replicate_index >= bootstrap.requested_replicates
                || failure.reason_code.trim().is_empty()
                || failure.message.trim().is_empty()
        })
    {
        return false;
    }
    let jackknife_failures = bootstrap
        .jackknife_cases
        .saturating_sub(bootstrap.usable_jackknife_cases);
    let mut expected_warnings = vec![
        "Regression bootstrap v1 uses deterministic indexed case resampling with replacement; percentile intervals are primary and BCa intervals are conditional on stable delete-one fits."
            .to_string(),
        "Bootstrap ratio statistics use an independently implemented two-sided standard-normal reference for both OLS and logistic coefficients; they are distinct from point-estimate t or Wald inference."
            .to_string(),
    ];
    if !bootstrap.failed_replicates.is_empty() {
        expected_warnings.push(format!(
            "{} of {} bootstrap replicates failed and were excluded from inference.",
            bootstrap.failed_replicates.len(),
            bootstrap.requested_replicates
        ));
    }
    if jackknife_failures > 0 {
        expected_warnings.push(format!(
            "{jackknife_failures} of {} delete-one fits failed; affected BCa intervals are explicitly unavailable.",
            bootstrap.jackknife_cases
        ));
    }
    if bootstrap.warnings != expected_warnings {
        return false;
    }
    if !validate_regression_bootstrap_witness(bootstrap, regression, kind) {
        return false;
    }
    let normal = Normal::standard();
    for (row, point) in bootstrap.coefficients.iter().zip(&regression.coefficients) {
        let expected_tolerance =
            64.0 * f64::EPSILON * 1.0_f64.max(row.original.abs()).max(row.replicate_max_abs);
        let coefficient_lower_bound = -row.replicate_max_abs - row.test_tolerance;
        let coefficient_upper_bound = row.replicate_max_abs + row.test_tolerance;
        let degenerate_distribution_valid = row.standard_error > row.test_tolerance
            || ((row.percentile_upper - row.percentile_lower).abs() <= row.test_tolerance
                && (row.percentile_lower - row.bootstrap_mean).abs() <= row.test_tolerance
                && (row.percentile_upper - row.bootstrap_mean).abs() <= row.test_tolerance);
        if row.term != point.term
            || !close_enough(row.original, point.estimate)
            || !row.bootstrap_mean.is_finite()
            || !row.bias.is_finite()
            || !close_enough(row.bias, row.bootstrap_mean - row.original)
            || !row.standard_error.is_finite()
            || row.standard_error < 0.0
            || !row.replicate_max_abs.is_finite()
            || row.replicate_max_abs < 0.0
            || row.bootstrap_mean.abs() > row.replicate_max_abs + row.test_tolerance
            || !close_enough(row.test_tolerance, expected_tolerance)
            || !row.percentile_lower.is_finite()
            || !row.percentile_upper.is_finite()
            || row.percentile_lower > row.percentile_upper
            || row.percentile_lower < coefficient_lower_bound
            || row.percentile_upper > coefficient_upper_bound
            || !degenerate_distribution_valid
            || row.usable_replicates != bootstrap.usable_replicates
            || !validate_regression_bootstrap_test(
                &row.test,
                row.original,
                row.standard_error,
                row.test_tolerance,
                &normal,
            )
            || !validate_regression_bca(
                &row.bca,
                bootstrap.usable_jackknife_cases,
                bootstrap.jackknife_cases,
                coefficient_lower_bound,
                coefficient_upper_bound,
            )
        {
            return false;
        }
        match (kind, row.odds_ratio.as_ref()) {
            (RegressionPersistenceKind::Logistic, Some(odds_ratio)) => {
                let odds_ratio_lower_bound = (-row.replicate_max_abs).exp();
                let odds_ratio_upper_bound = row.replicate_max_abs.exp();
                if !odds_ratio_lower_bound.is_finite()
                    || !odds_ratio_upper_bound.is_finite()
                    || !odds_ratio.original.is_finite()
                    || odds_ratio.original <= 0.0
                    || !close_enough(odds_ratio.original, row.original.exp())
                    || !odds_ratio.percentile_lower.is_finite()
                    || !odds_ratio.percentile_upper.is_finite()
                    || odds_ratio.percentile_lower <= 0.0
                    || odds_ratio.percentile_lower > odds_ratio.percentile_upper
                    || odds_ratio.percentile_lower + row.test_tolerance < odds_ratio_lower_bound
                    || odds_ratio.percentile_upper > odds_ratio_upper_bound + row.test_tolerance
                    || !validate_regression_bca(
                        &odds_ratio.bca,
                        bootstrap.usable_jackknife_cases,
                        bootstrap.jackknife_cases,
                        odds_ratio_lower_bound,
                        odds_ratio_upper_bound,
                    )
                {
                    return false;
                }
            }
            (RegressionPersistenceKind::Ols, None) => {}
            _ => return false,
        }
    }
    true
}

fn validate_process_bootstrap_contract(
    bootstrap: &ProcessBootstrapAnalysis,
    graph: &ProcessGraphAnalysis,
    settings: &AnalysisSettings,
) -> bool {
    let witness = &bootstrap.validation_witness;
    let original = process_bootstrap_estimands(graph);
    let original_ids = original
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let successful_indices = witness
        .successful_bootstrap
        .iter()
        .map(|row| row.replicate_index)
        .collect::<Vec<_>>();
    let failed_indices = bootstrap
        .failed_replicates
        .iter()
        .map(|row| row.replicate_index)
        .collect::<Vec<_>>();
    let successful_jackknife = witness
        .successful_jackknife
        .iter()
        .map(|row| row.omitted_case)
        .collect::<Vec<_>>();
    let failed_jackknife = witness
        .failed_jackknife
        .iter()
        .map(|row| row.omitted_case)
        .collect::<Vec<_>>();
    let valid_failure_code = |reason: &str| {
        matches!(
            reason,
            "rank_deficient_equation"
                | "nonfinite_estimate"
                | "invalid_binary_profile"
                | "high_leverage_hc3_instability"
                | "invalid_hc3_covariance"
                | "degenerate_simple_slope_variance"
        )
    };
    let failed_jackknife_count = bootstrap
        .jackknife_cases
        .saturating_sub(bootstrap.usable_jackknife_cases);
    let mut expected_warnings = vec![
        "PROCESS bootstrap v1 uses deterministic indexed complete-case resampling with replacement; percentile intervals are primary and BCa intervals require every delete-one fit.".to_string(),
        "PROCESS bootstrap ratio tests use the original effect divided by its bootstrap standard error with a fixed two-sided standard-normal reference.".to_string(),
    ];
    if !bootstrap.failed_replicates.is_empty() {
        expected_warnings.push(format!(
            "{} of {} PROCESS bootstrap replicates failed and were excluded from inference.",
            bootstrap.failed_replicates.len(),
            bootstrap.requested_replicates
        ));
    }
    if failed_jackknife_count > 0 {
        expected_warnings.push(format!(
            "{failed_jackknife_count} of {} PROCESS delete-one fits failed; BCa intervals are explicitly unavailable.",
            bootstrap.jackknife_cases
        ));
    }
    if bootstrap.method_version != PROCESS_BOOTSTRAP_METHOD_VERSION
        || bootstrap.algorithm != PROCESS_BOOTSTRAP_ALGORITHM
        || bootstrap.interval_policy != PROCESS_BOOTSTRAP_INTERVAL_POLICY
        || bootstrap.test_reference != PROCESS_BOOTSTRAP_TEST_REFERENCE
        || bootstrap.requested_replicates != settings.bootstrap_samples
        || bootstrap.usable_replicates as usize + bootstrap.failed_replicates.len()
            != bootstrap.requested_replicates as usize
        || bootstrap.minimum_usable_fraction.to_bits()
            != REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION.to_bits()
        || bootstrap.usable_replicates
            < ((bootstrap.requested_replicates as f64
                * REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION)
                .ceil() as u32)
        || bootstrap.seed != settings.seed
        || bootstrap.workers != settings.workers
        || bootstrap.stream_token != PROCESS_BOOTSTRAP_STREAM_TOKEN
        || bootstrap.warnings != expected_warnings
        || bootstrap.jackknife_cases != graph.complete_cases
        || bootstrap.usable_jackknife_cases + witness.failed_jackknife.len()
            != bootstrap.jackknife_cases
        || witness.method_version != PROCESS_BOOTSTRAP_VALIDATION_WITNESS_VERSION
        || witness.estimand_ids != original_ids
        || witness.estimand_ids.is_empty()
        || witness.estimand_ids.iter().collect::<BTreeSet<_>>().len() != witness.estimand_ids.len()
        || witness.successful_bootstrap.len() != bootstrap.usable_replicates as usize
        || witness.successful_jackknife.len() != bootstrap.usable_jackknife_cases
        || !successful_indices.windows(2).all(|pair| pair[0] < pair[1])
        || !failed_indices.windows(2).all(|pair| pair[0] < pair[1])
        || !successful_jackknife
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        || !failed_jackknife.windows(2).all(|pair| pair[0] < pair[1])
        || witness.successful_bootstrap.iter().any(|row| {
            row.estimates.len() != witness.estimand_ids.len()
                || row.estimates.iter().any(|value| !value.is_finite())
        })
        || witness.successful_jackknife.iter().any(|row| {
            row.estimates.len() != witness.estimand_ids.len()
                || row.estimates.iter().any(|value| !value.is_finite())
        })
        || bootstrap.failed_replicates.iter().any(|failure| {
            !valid_failure_code(&failure.reason_code) || failure.message.trim().is_empty()
        })
        || witness.failed_jackknife.iter().any(|failure| {
            !valid_failure_code(&failure.reason_code) || failure.message.trim().is_empty()
        })
    {
        return false;
    }
    let mut all_bootstrap_indices = successful_indices;
    all_bootstrap_indices.extend(failed_indices);
    all_bootstrap_indices.sort_unstable();
    if all_bootstrap_indices != (0..bootstrap.requested_replicates).collect::<Vec<_>>() {
        return false;
    }
    let mut all_jackknife_indices = successful_jackknife;
    all_jackknife_indices.extend(failed_jackknife);
    all_jackknife_indices.sort_unstable();
    if all_jackknife_indices != (0..bootstrap.jackknife_cases).collect::<Vec<_>>() {
        return false;
    }
    let bootstrap_estimates = witness
        .successful_bootstrap
        .iter()
        .map(|row| row.estimates.clone())
        .collect::<Vec<_>>();
    let jackknife_estimates = witness
        .successful_jackknife
        .iter()
        .map(|row| row.estimates.clone())
        .collect::<Vec<_>>();
    summarize_process_bootstrap_estimands(
        &witness.estimand_ids,
        &original.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
        &bootstrap_estimates,
        &jackknife_estimates,
        bootstrap.jackknife_cases,
        0.95,
    )
    .is_ok_and(|recomputed| {
        recomputed.len() == bootstrap.estimands.len()
            && recomputed
                .iter()
                .zip(&bootstrap.estimands)
                .all(|(left, right)| {
                    left.effect_id == right.effect_id
                        && regression_bootstrap_json_roundtrip_close(left.original, right.original)
                        && regression_bootstrap_json_roundtrip_close(
                            left.bootstrap_mean,
                            right.bootstrap_mean,
                        )
                        && regression_bootstrap_json_roundtrip_close(left.bias, right.bias)
                        && regression_bootstrap_json_roundtrip_close(
                            left.standard_error,
                            right.standard_error,
                        )
                        && regression_bootstrap_tests_match_after_json_roundtrip(
                            &left.test,
                            &right.test,
                        )
                        && regression_bootstrap_json_roundtrip_close(
                            left.percentile_lower,
                            right.percentile_lower,
                        )
                        && regression_bootstrap_json_roundtrip_close(
                            left.percentile_upper,
                            right.percentile_upper,
                        )
                        && regression_bootstrap_bca_matches_after_json_roundtrip(
                            &left.bca, &right.bca,
                        )
                        && left.usable_replicates == right.usable_replicates
                })
    })
}

fn validate_regression_bootstrap_witness(
    bootstrap: &RegressionBootstrapAnalysis,
    regression: &RegressionAnalysis,
    kind: &RegressionPersistenceKind,
) -> bool {
    let witness = &bootstrap.validation_witness;
    let expected_terms = regression
        .coefficients
        .iter()
        .map(|coefficient| coefficient.term.clone())
        .collect::<Vec<_>>();
    let logistic = matches!(kind, RegressionPersistenceKind::Logistic);
    let valid_coefficients = |coefficients: &[f64]| {
        coefficients.len() == expected_terms.len()
            && coefficients
                .iter()
                .all(|value| value.is_finite() && (!logistic || value.exp().is_finite()))
    };
    let bootstrap_indices = witness
        .successful_bootstrap
        .iter()
        .map(|row| row.replicate_index)
        .collect::<Vec<_>>();
    let failed_bootstrap_indices = bootstrap
        .failed_replicates
        .iter()
        .map(|row| row.replicate_index)
        .collect::<Vec<_>>();
    let successful_jackknife_indices = witness
        .successful_jackknife
        .iter()
        .map(|row| row.omitted_case)
        .collect::<Vec<_>>();
    let failed_jackknife_indices = witness
        .failed_jackknife
        .iter()
        .map(|row| row.omitted_case)
        .collect::<Vec<_>>();
    let strictly_ascending = |values: &[usize]| values.windows(2).all(|pair| pair[0] < pair[1]);
    let bootstrap_strictly_ascending = bootstrap_indices.windows(2).all(|pair| pair[0] < pair[1]);
    if witness.method_version != REGRESSION_BOOTSTRAP_VALIDATION_WITNESS_VERSION
        || witness.terms != expected_terms
        || witness.terms.is_empty()
        || witness.terms.len() > 51
        || witness.successful_bootstrap.len() != bootstrap.usable_replicates as usize
        || witness.successful_jackknife.len() != bootstrap.usable_jackknife_cases
        || witness.failed_jackknife.len()
            != bootstrap
                .jackknife_cases
                .saturating_sub(bootstrap.usable_jackknife_cases)
        || bootstrap.jackknife_cases != regression.observations
        || !bootstrap_strictly_ascending
        || !failed_bootstrap_indices
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        || !strictly_ascending(&successful_jackknife_indices)
        || !strictly_ascending(&failed_jackknife_indices)
        || witness
            .successful_bootstrap
            .iter()
            .any(|row| !valid_coefficients(&row.coefficients))
        || witness
            .successful_jackknife
            .iter()
            .any(|row| !valid_coefficients(&row.coefficients))
        || witness.failed_jackknife.iter().any(|failure| {
            failure.omitted_case >= bootstrap.jackknife_cases
                || failure.reason_code.trim().is_empty()
                || failure.message.trim().is_empty()
        })
    {
        return false;
    }
    let mut all_bootstrap_indices = bootstrap_indices;
    all_bootstrap_indices.extend(failed_bootstrap_indices);
    all_bootstrap_indices.sort_unstable();
    if all_bootstrap_indices != (0..bootstrap.requested_replicates).collect::<Vec<_>>() {
        return false;
    }
    let mut all_jackknife_indices = successful_jackknife_indices;
    all_jackknife_indices.extend(failed_jackknife_indices);
    all_jackknife_indices.sort_unstable();
    if all_jackknife_indices != (0..bootstrap.jackknife_cases).collect::<Vec<_>>() {
        return false;
    }
    let bootstrap_estimates = witness
        .successful_bootstrap
        .iter()
        .map(|row| row.coefficients.clone())
        .collect::<Vec<_>>();
    let jackknife_estimates = witness
        .successful_jackknife
        .iter()
        .map(|row| row.coefficients.clone())
        .collect::<Vec<_>>();
    summarize_regression_bootstrap_coefficients(
        &witness.terms,
        &regression
            .coefficients
            .iter()
            .map(|coefficient| coefficient.estimate)
            .collect::<Vec<_>>(),
        &bootstrap_estimates,
        &jackknife_estimates,
        bootstrap.jackknife_cases,
        logistic,
        bootstrap.confidence_level,
    )
    .is_ok_and(|recomputed| {
        regression_bootstrap_summaries_match_after_json_roundtrip(
            &recomputed,
            &bootstrap.coefficients,
        )
    })
}

/// `serde_json`'s decimal round trip can shift a recomputation by a handful
/// of ULPs even though every persisted witness value is unchanged. Keep all
/// discrete identities and tagged availability states exact, and allow only a
/// 64-epsilon finite-number envelope when comparing recomputed summaries.
fn regression_bootstrap_json_roundtrip_close(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 64.0 * f64::EPSILON * 1.0_f64.max(left.abs()).max(right.abs())
}

fn regression_bootstrap_summaries_match_after_json_roundtrip(
    recomputed: &[RegressionBootstrapCoefficient],
    stored: &[RegressionBootstrapCoefficient],
) -> bool {
    recomputed.len() == stored.len()
        && recomputed.iter().zip(stored).all(|(left, right)| {
            left.term == right.term
                && regression_bootstrap_json_roundtrip_close(left.original, right.original)
                && regression_bootstrap_json_roundtrip_close(
                    left.bootstrap_mean,
                    right.bootstrap_mean,
                )
                && regression_bootstrap_json_roundtrip_close(left.bias, right.bias)
                && regression_bootstrap_json_roundtrip_close(
                    left.standard_error,
                    right.standard_error,
                )
                && regression_bootstrap_json_roundtrip_close(
                    left.replicate_max_abs,
                    right.replicate_max_abs,
                )
                && regression_bootstrap_json_roundtrip_close(
                    left.test_tolerance,
                    right.test_tolerance,
                )
                && regression_bootstrap_tests_match_after_json_roundtrip(&left.test, &right.test)
                && regression_bootstrap_json_roundtrip_close(
                    left.percentile_lower,
                    right.percentile_lower,
                )
                && regression_bootstrap_json_roundtrip_close(
                    left.percentile_upper,
                    right.percentile_upper,
                )
                && left.usable_replicates == right.usable_replicates
                && regression_bootstrap_bca_matches_after_json_roundtrip(&left.bca, &right.bca)
                && match (&left.odds_ratio, &right.odds_ratio) {
                    (Some(left), Some(right)) => {
                        regression_bootstrap_odds_ratio_matches_after_json_roundtrip(left, right)
                    }
                    (None, None) => true,
                    _ => false,
                }
        })
}

fn regression_bootstrap_tests_match_after_json_roundtrip(
    left: &RegressionBootstrapTest,
    right: &RegressionBootstrapTest,
) -> bool {
    match (left, right) {
        (
            RegressionBootstrapTest::Available {
                statistic: left_statistic,
                p_value_two_sided: left_p_value,
            },
            RegressionBootstrapTest::Available {
                statistic: right_statistic,
                p_value_two_sided: right_p_value,
            },
        ) => {
            regression_bootstrap_json_roundtrip_close(*left_statistic, *right_statistic)
                && regression_bootstrap_json_roundtrip_close(*left_p_value, *right_p_value)
        }
        (
            RegressionBootstrapTest::Unavailable {
                reason_code: left_reason,
                message: left_message,
            },
            RegressionBootstrapTest::Unavailable {
                reason_code: right_reason,
                message: right_message,
            },
        ) => left_reason == right_reason && left_message == right_message,
        _ => false,
    }
}

fn regression_bootstrap_bca_matches_after_json_roundtrip(
    left: &RegressionBootstrapBcaInterval,
    right: &RegressionBootstrapBcaInterval,
) -> bool {
    match (left, right) {
        (
            RegressionBootstrapBcaInterval::Available {
                bias_correction: left_bias,
                acceleration: left_acceleration,
                lower: left_lower,
                upper: left_upper,
            },
            RegressionBootstrapBcaInterval::Available {
                bias_correction: right_bias,
                acceleration: right_acceleration,
                lower: right_lower,
                upper: right_upper,
            },
        ) => {
            regression_bootstrap_json_roundtrip_close(*left_bias, *right_bias)
                && regression_bootstrap_json_roundtrip_close(
                    *left_acceleration,
                    *right_acceleration,
                )
                && regression_bootstrap_json_roundtrip_close(*left_lower, *right_lower)
                && regression_bootstrap_json_roundtrip_close(*left_upper, *right_upper)
        }
        (
            RegressionBootstrapBcaInterval::Unavailable {
                reason_code: left_reason,
                message: left_message,
            },
            RegressionBootstrapBcaInterval::Unavailable {
                reason_code: right_reason,
                message: right_message,
            },
        ) => left_reason == right_reason && left_message == right_message,
        _ => false,
    }
}

fn regression_bootstrap_odds_ratio_matches_after_json_roundtrip(
    left: &RegressionBootstrapOddsRatio,
    right: &RegressionBootstrapOddsRatio,
) -> bool {
    regression_bootstrap_json_roundtrip_close(left.original, right.original)
        && regression_bootstrap_json_roundtrip_close(left.percentile_lower, right.percentile_lower)
        && regression_bootstrap_json_roundtrip_close(left.percentile_upper, right.percentile_upper)
        && regression_bootstrap_bca_matches_after_json_roundtrip(&left.bca, &right.bca)
}

fn validate_regression_bootstrap_test(
    test: &RegressionBootstrapTest,
    original: f64,
    standard_error: f64,
    tolerance: f64,
    normal: &Normal,
) -> bool {
    match test {
        RegressionBootstrapTest::Available {
            statistic,
            p_value_two_sided,
        } => {
            standard_error > tolerance
                && statistic.is_finite()
                && p_value_two_sided.is_finite()
                && (0.0..=1.0).contains(p_value_two_sided)
                && close_enough(*statistic, original / standard_error)
                && close_enough(
                    *p_value_two_sided,
                    (2.0 * normal.sf(statistic.abs())).clamp(0.0, 1.0),
                )
        }
        RegressionBootstrapTest::Unavailable {
            reason_code,
            message,
        } => {
            standard_error <= tolerance
                && reason_code == "degenerate_bootstrap_standard_error"
                && !message.trim().is_empty()
        }
    }
}

fn validate_regression_bca(
    interval: &RegressionBootstrapBcaInterval,
    usable_jackknife_cases: usize,
    jackknife_cases: usize,
    lower_bound: f64,
    upper_bound: f64,
) -> bool {
    if usable_jackknife_cases < jackknife_cases {
        return matches!(
            interval,
            RegressionBootstrapBcaInterval::Unavailable {
                reason_code,
                message,
            } if reason_code == "incomplete_jackknife" && !message.trim().is_empty()
        );
    }
    match interval {
        RegressionBootstrapBcaInterval::Available {
            bias_correction,
            acceleration,
            lower,
            upper,
        } => {
            bias_correction.is_finite()
                && acceleration.is_finite()
                && lower.is_finite()
                && upper.is_finite()
                && lower <= upper
                && *lower >= lower_bound
                && *upper <= upper_bound
        }
        RegressionBootstrapBcaInterval::Unavailable {
            reason_code,
            message,
        } => {
            matches!(
                reason_code.as_str(),
                "insufficient_jackknife_estimates" | "degenerate_jackknife_acceleration"
            ) && !message.trim().is_empty()
        }
    }
}

fn csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn validate_linear_regression_analysis_contract(
    regression: &RegressionAnalysis,
    expected_method_version: &str,
    expected_regression_type: &str,
    outcome: &str,
    predictors: &[String],
    controls: &[String],
    observations: usize,
    confidence_level: f64,
    process_required: bool,
) -> bool {
    let parameter_count = 1 + predictors.len() + controls.len();
    if regression.method_version != expected_method_version
        || regression.regression_type != expected_regression_type
        || regression.outcome != outcome
        || regression.predictors != predictors
        || regression.controls != controls
        || regression.observations != observations
        || observations <= parameter_count
        || regression.coefficients.len() != parameter_count
        || regression.predictions.len() != observations
        || regression.logistic.is_some()
        || regression.process.is_some() != process_required
        || regression.warnings.is_empty()
    {
        return false;
    }
    let expected_terms = std::iter::once("intercept")
        .chain(predictors.iter().map(String::as_str))
        .chain(controls.iter().map(String::as_str));
    let degrees_of_freedom = (observations - parameter_count) as f64;
    let Ok(distribution) = StudentsT::new(0.0, 1.0, degrees_of_freedom) else {
        return false;
    };
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    for (coefficient, expected_term) in regression.coefficients.iter().zip(expected_terms) {
        let expected_statistic = coefficient.estimate / coefficient.standard_error;
        let expected_p = 2.0 * (1.0 - distribution.cdf(expected_statistic.abs()));
        if coefficient.term != expected_term
            || !coefficient.estimate.is_finite()
            || !coefficient.standard_error.is_finite()
            || coefficient.standard_error <= 0.0
            || !coefficient.statistic.is_finite()
            || !coefficient.p_value_two_sided.is_finite()
            || !(0.0..=1.0).contains(&coefficient.p_value_two_sided)
            || !coefficient.confidence_interval_lower.is_finite()
            || !coefficient.confidence_interval_upper.is_finite()
            || coefficient.odds_ratio.is_some()
            || coefficient.odds_ratio_confidence_interval_lower.is_some()
            || coefficient.odds_ratio_confidence_interval_upper.is_some()
            || !close_enough(coefficient.statistic, expected_statistic)
            || !close_enough(coefficient.p_value_two_sided, expected_p)
            || !close_enough(
                coefficient.confidence_interval_lower,
                coefficient.estimate - critical * coefficient.standard_error,
            )
            || !close_enough(
                coefficient.confidence_interval_upper,
                coefficient.estimate + critical * coefficient.standard_error,
            )
        {
            return false;
        }
    }
    let Some(fit) = regression.fit.as_ref() else {
        return false;
    };
    let (Some(r_squared), Some(adjusted_r_squared), Some(f_statistic), Some(rmse)) = (
        fit.r_squared,
        fit.adjusted_r_squared,
        fit.f_statistic,
        fit.rmse,
    ) else {
        return false;
    };
    if !r_squared.is_finite()
        || !(-1e-10..=1.0 + 1e-10).contains(&r_squared)
        || !adjusted_r_squared.is_finite()
        || !f_statistic.is_finite()
        || f_statistic < 0.0
        || fit.log_likelihood.is_some()
        || fit.pseudo_r_squared.is_some()
        || fit.null_log_likelihood.is_some()
        || fit.deviance.is_some()
        || fit.null_deviance.is_some()
        || fit.likelihood_ratio_chi_square.is_some()
        || fit.likelihood_ratio_degrees_of_freedom.is_some()
        || fit.likelihood_ratio_p_value.is_some()
        || fit.pseudo_r_squared_method.is_some()
        || !fit.aic.is_finite()
        || !fit.bic.is_finite()
        || !rmse.is_finite()
        || rmse < 0.0
    {
        return false;
    }
    let mut actual = Vec::with_capacity(observations);
    let mut residual_sum_squares = 0.0;
    for (index, prediction) in regression.predictions.iter().enumerate() {
        let Some(residual) = prediction.residual else {
            return false;
        };
        if prediction.observation != index
            || !prediction.fitted.is_finite()
            || !residual.is_finite()
            || prediction.probability.is_some()
        {
            return false;
        }
        actual.push(prediction.fitted + residual);
        residual_sum_squares += residual * residual;
    }
    let mean = actual.iter().sum::<f64>() / observations as f64;
    let total_sum_squares = actual
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    let expected_r_squared = if total_sum_squares > f64::EPSILON {
        1.0 - residual_sum_squares / total_sum_squares
    } else {
        0.0
    };
    let expected_adjusted =
        1.0 - (1.0 - expected_r_squared) * (observations as f64 - 1.0) / degrees_of_freedom;
    let predictor_count = parameter_count - 1;
    let expected_f = (expected_r_squared / predictor_count.max(1) as f64)
        / ((1.0 - expected_r_squared) / degrees_of_freedom).max(1e-12);
    let sigma2 = residual_sum_squares / observations as f64;
    close_enough(r_squared, expected_r_squared)
        && close_enough(adjusted_r_squared, expected_adjusted)
        && close_enough(f_statistic, expected_f)
        && close_enough(rmse, sigma2.sqrt())
        && close_enough(
            fit.aic,
            observations as f64 * sigma2.max(1e-12).ln() + 2.0 * parameter_count as f64,
        )
        && close_enough(
            fit.bic,
            observations as f64 * sigma2.max(1e-12).ln()
                + (observations as f64).ln() * parameter_count as f64,
        )
}

fn validate_logistic_analysis_contract(
    regression: &RegressionAnalysis,
    expected_method_version: &str,
    outcome: &str,
    predictors: &[String],
    controls: &[String],
    observations: usize,
    omitted_observations: usize,
    confidence_level: f64,
) -> bool {
    let is_legacy_v1 = expected_method_version == REGRESSION_LOGISTIC_METHOD_VERSION_V1;
    if !is_legacy_v1 && expected_method_version != REGRESSION_LOGISTIC_METHOD_VERSION {
        return false;
    }
    let parameter_count = 1 + predictors.len() + controls.len();
    if regression.method_version != expected_method_version
        || regression.regression_type != "logistic"
        || regression.outcome != outcome
        || regression.predictors != predictors
        || regression.controls != controls
        || regression.observations != observations
        || observations <= parameter_count
        || regression.coefficients.len() != parameter_count
        || regression.predictions.len() != observations
        || regression.process.is_some()
        || regression.warnings.is_empty()
        || (is_legacy_v1 && regression.logistic.is_some())
        || (!is_legacy_v1 && regression.logistic.is_none())
    {
        return false;
    }
    let normal = statrs::distribution::Normal::standard();
    let critical = normal.inverse_cdf(0.5 + confidence_level / 2.0);
    let expected_terms = std::iter::once("intercept")
        .chain(predictors.iter().map(String::as_str))
        .chain(controls.iter().map(String::as_str));
    for (coefficient, expected_term) in regression.coefficients.iter().zip(expected_terms) {
        let expected_statistic = coefficient.estimate / coefficient.standard_error;
        let expected_p = 2.0 * (1.0 - normal.cdf(expected_statistic.abs()));
        if coefficient.term != expected_term
            || !coefficient.estimate.is_finite()
            || !coefficient.standard_error.is_finite()
            || coefficient.standard_error <= 0.0
            || !coefficient.statistic.is_finite()
            || !coefficient.p_value_two_sided.is_finite()
            || !(0.0..=1.0).contains(&coefficient.p_value_two_sided)
            || !coefficient.confidence_interval_lower.is_finite()
            || !coefficient.confidence_interval_upper.is_finite()
            || coefficient.odds_ratio.is_none_or(|odds_ratio| {
                !odds_ratio.is_finite()
                    || odds_ratio <= 0.0
                    || !close_enough(odds_ratio, coefficient.estimate.exp())
            })
            || (if is_legacy_v1 {
                coefficient.odds_ratio_confidence_interval_lower.is_some()
                    || coefficient.odds_ratio_confidence_interval_upper.is_some()
            } else {
                coefficient
                    .odds_ratio_confidence_interval_lower
                    .is_none_or(|value| {
                        !value.is_finite()
                            || value <= 0.0
                            || !close_enough(value, coefficient.confidence_interval_lower.exp())
                    })
                    || coefficient
                        .odds_ratio_confidence_interval_upper
                        .is_none_or(|value| {
                            !value.is_finite()
                                || value <= 0.0
                                || !close_enough(value, coefficient.confidence_interval_upper.exp())
                        })
            })
            || !close_enough(coefficient.statistic, expected_statistic)
            || !close_enough(coefficient.p_value_two_sided, expected_p)
            || !close_enough(
                coefficient.confidence_interval_lower,
                coefficient.estimate - critical * coefficient.standard_error,
            )
            || !close_enough(
                coefficient.confidence_interval_upper,
                coefficient.estimate + critical * coefficient.standard_error,
            )
        {
            return false;
        }
    }
    let Some(fit) = regression.fit.as_ref() else {
        return false;
    };
    let (Some(log_likelihood), Some(pseudo_r_squared)) = (fit.log_likelihood, fit.pseudo_r_squared)
    else {
        return false;
    };
    if fit.r_squared.is_some()
        || fit.adjusted_r_squared.is_some()
        || fit.f_statistic.is_some()
        || fit.rmse.is_some()
        || !log_likelihood.is_finite()
        || !pseudo_r_squared.is_finite()
        || !fit.aic.is_finite()
        || !fit.bic.is_finite()
    {
        return false;
    }
    let mut actual = Vec::with_capacity(observations);
    let mut expected_log_likelihood = 0.0;
    for (index, prediction) in regression.predictions.iter().enumerate() {
        let (Some(residual), Some(probability)) = (prediction.residual, prediction.probability)
        else {
            return false;
        };
        let outcome_value = prediction.fitted + residual;
        if prediction.observation != index
            || !prediction.fitted.is_finite()
            || !residual.is_finite()
            || !probability.is_finite()
            || !(0.0..1.0).contains(&probability)
            || !close_enough(prediction.fitted, probability)
            || !(close_enough(outcome_value, 0.0) || close_enough(outcome_value, 1.0))
        {
            return false;
        }
        let binary = if close_enough(outcome_value, 1.0) {
            1.0
        } else {
            0.0
        };
        actual.push(binary);
        expected_log_likelihood +=
            binary * probability.ln() + (1.0 - binary) * (1.0 - probability).ln();
    }
    let mean = actual.iter().sum::<f64>() / observations as f64;
    if !(0.0..1.0).contains(&mean) {
        return false;
    }
    let null_log_likelihood = actual
        .iter()
        .map(|value| value * mean.ln() + (1.0 - value) * (1.0 - mean).ln())
        .sum::<f64>();
    let expected_pseudo_r_squared = 1.0 - expected_log_likelihood / null_log_likelihood;
    let expected_deviance = -2.0 * expected_log_likelihood;
    let expected_null_deviance = -2.0 * null_log_likelihood;
    let expected_likelihood_ratio =
        (2.0 * (expected_log_likelihood - null_log_likelihood)).max(0.0);
    let likelihood_ratio_df = parameter_count - 1;
    let Ok(likelihood_ratio_distribution) = ChiSquared::new(likelihood_ratio_df as f64) else {
        return false;
    };
    let expected_likelihood_ratio_p =
        (1.0 - likelihood_ratio_distribution.cdf(expected_likelihood_ratio)).clamp(0.0, 1.0);
    let base_fit_valid = close_enough(log_likelihood, expected_log_likelihood)
        && close_enough(pseudo_r_squared, expected_pseudo_r_squared)
        && close_enough(
            fit.aic,
            -2.0 * expected_log_likelihood + 2.0 * parameter_count as f64,
        )
        && close_enough(
            fit.bic,
            -2.0 * expected_log_likelihood + (observations as f64).ln() * parameter_count as f64,
        );
    if !base_fit_valid {
        return false;
    }
    if is_legacy_v1 {
        return fit.null_log_likelihood.is_none()
            && fit.deviance.is_none()
            && fit.null_deviance.is_none()
            && fit.likelihood_ratio_chi_square.is_none()
            && fit.likelihood_ratio_degrees_of_freedom.is_none()
            && fit.likelihood_ratio_p_value.is_none()
            && fit.pseudo_r_squared_method.is_none();
    }

    let Some(logistic) = regression.logistic.as_ref() else {
        return false;
    };
    let zero_count = actual.iter().filter(|value| **value == 0.0).count();
    let one_count = actual.iter().filter(|value| **value == 1.0).count();
    let profile = &logistic.outcome_profile;
    if profile.outcome != outcome
        || profile.coding != "numeric_0_1_exact_v1"
        || profile.complete_cases != observations
        || profile.omitted_cases != omitted_observations
        || profile.zero_count != zero_count
        || profile.one_count != one_count
        || profile.invalid_count != 0
        || profile.prevalence.is_none_or(|value| {
            !value.is_finite() || !close_enough(value, one_count as f64 / observations as f64)
        })
        || profile.readiness != qpls_estimation::LogisticOutcomeReadiness::Ready
    {
        return false;
    }
    let convergence = &logistic.convergence;
    if convergence.algorithm != "deterministic_newton_irls_v1"
        || !convergence.converged
        || !(1..=100).contains(&convergence.iterations)
        || convergence.max_iterations != 100
        || convergence.tolerance.to_bits() != 1e-8_f64.to_bits()
        || !convergence.final_max_abs_step.is_finite()
        || convergence.final_max_abs_step < 0.0
        || convergence.final_max_abs_step >= convergence.tolerance
        || convergence.separation_probability_tolerance.to_bits() != 1e-9_f64.to_bits()
    {
        return false;
    }
    let classification = &logistic.classification;
    let mut true_positive = 0;
    let mut true_negative = 0;
    let mut false_positive = 0;
    let mut false_negative = 0;
    for (actual, prediction) in actual.iter().zip(&regression.predictions) {
        let predicted_positive = prediction.probability.unwrap() >= 0.5;
        match (*actual == 1.0, predicted_positive) {
            (true, true) => true_positive += 1,
            (false, false) => true_negative += 1,
            (false, true) => false_positive += 1,
            (true, false) => false_negative += 1,
        }
    }
    let expected_accuracy = (true_positive + true_negative) as f64 / observations as f64;
    let expected_sensitivity = true_positive as f64 / (true_positive + false_negative) as f64;
    let expected_specificity = true_negative as f64 / (true_negative + false_positive) as f64;
    classification.threshold.to_bits() == 0.5_f64.to_bits()
        && classification.true_positive == true_positive
        && classification.true_negative == true_negative
        && classification.false_positive == false_positive
        && classification.false_negative == false_negative
        && close_enough(classification.accuracy, expected_accuracy)
        && close_enough(classification.sensitivity, expected_sensitivity)
        && close_enough(classification.specificity, expected_specificity)
        && fit
            .null_log_likelihood
            .is_some_and(|value| close_enough(value, null_log_likelihood))
        && fit
            .deviance
            .is_some_and(|value| close_enough(value, expected_deviance))
        && fit
            .null_deviance
            .is_some_and(|value| close_enough(value, expected_null_deviance))
        && fit
            .likelihood_ratio_chi_square
            .is_some_and(|value| close_enough(value, expected_likelihood_ratio))
        && fit.likelihood_ratio_degrees_of_freedom == Some(likelihood_ratio_df)
        && fit
            .likelihood_ratio_p_value
            .is_some_and(|value| close_enough(value, expected_likelihood_ratio_p))
        && fit.pseudo_r_squared_method.as_deref() == Some("mcfadden_v1")
}

fn validate_process_analysis_contract(
    regression: &RegressionAnalysis,
    expected: &ProcessPersistenceContract,
    expected_omitted_observations: usize,
) -> bool {
    if let ProcessPersistenceContract::Graph {
        focal_predictor,
        paths,
        moderators,
        moderations,
    } = expected
    {
        return validate_process_graph_analysis_contract(
            regression,
            focal_predictor,
            paths,
            moderators,
            moderations,
            expected_omitted_observations,
        );
    }
    const WARNING: &str = "PROCESS v1 reports bounded deterministic mediation/moderation effects validated for the documented QuickPLS v1.2.2 scope; moderated mediation remains experimental.";
    let Some(process) = regression.process.as_ref() else {
        return false;
    };
    let expected_effects: &[&str] = match expected {
        ProcessPersistenceContract::Mediation { .. } => &["direct", "indirect", "total"],
        ProcessPersistenceContract::Moderation { .. } => &["interaction"],
        ProcessPersistenceContract::ModeratedMediation { .. } => {
            &["direct", "indirect", "total", "interaction"]
        }
        ProcessPersistenceContract::Graph { .. } => &[],
    };
    if process.method_version != REGRESSION_PROCESS_METHOD_VERSION_V1
        || process.model != expected.model()
        || process.effects.len() != expected_effects.len()
        || process.warnings.len() != 1
        || process.warnings[0] != WARNING
    {
        return false;
    }
    let mut effect_values = BTreeMap::new();
    for (effect, expected_name) in process.effects.iter().zip(expected_effects) {
        if effect.effect != *expected_name
            || !effect.estimate.is_finite()
            || effect.lower_percentile.is_some()
            || effect.upper_percentile.is_some()
            || effect_values
                .insert(effect.effect.as_str(), effect.estimate)
                .is_some()
        {
            return false;
        }
    }
    if matches!(
        expected,
        ProcessPersistenceContract::Mediation { .. }
            | ProcessPersistenceContract::ModeratedMediation { .. }
    ) && !close_enough(
        effect_values["total"],
        effect_values["direct"] + effect_values["indirect"],
    ) {
        return false;
    }
    if matches!(expected, ProcessPersistenceContract::Mediation { .. }) {
        return process.simple_slopes.is_empty();
    }
    if process.simple_slopes.len() != 3 {
        return false;
    }
    let expected_levels = [-1.0, 0.0, 1.0];
    for (slope, level) in process.simple_slopes.iter().zip(expected_levels) {
        if !close_enough(slope.moderator_value, level) || !slope.slope.is_finite() {
            return false;
        }
    }
    let interaction = effect_values["interaction"];
    close_enough(
        process.simple_slopes[0].slope,
        process.simple_slopes[1].slope - interaction,
    ) && close_enough(
        process.simple_slopes[2].slope,
        process.simple_slopes[1].slope + interaction,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectProcessTermSpec {
    term_id: String,
    kind: String,
    variables: Vec<String>,
}

fn project_process_canonical_relationships(
    predictors: &[String],
    outcome: &str,
    paths: &[qpls_core::ProcessPathConfig],
    moderations: &[qpls_core::ProcessModerationConfig],
) -> Option<(
    Vec<qpls_core::ProcessPathConfig>,
    Vec<qpls_core::ProcessModerationConfig>,
)> {
    let mut node_order = predictors
        .iter()
        .enumerate()
        .map(|(index, variable)| (variable.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    node_order.insert(outcome, predictors.len());
    if paths.iter().any(|path| {
        !node_order.contains_key(path.from.as_str()) || !node_order.contains_key(path.to.as_str())
    }) || moderations.iter().any(|moderation| {
        !node_order.contains_key(moderation.from.as_str())
            || !node_order.contains_key(moderation.to.as_str())
    }) {
        return None;
    }
    let mut ordered_paths = paths.to_vec();
    ordered_paths.sort_by(|left, right| {
        (
            node_order[left.to.as_str()],
            node_order[left.from.as_str()],
            left.from.as_str(),
            left.to.as_str(),
        )
            .cmp(&(
                node_order[right.to.as_str()],
                node_order[right.from.as_str()],
                right.from.as_str(),
                right.to.as_str(),
            ))
    });
    let mut ordered_moderations = moderations.to_vec();
    ordered_moderations.sort_by(|left, right| {
        (
            node_order[left.to.as_str()],
            node_order[left.from.as_str()],
            left.moderator.as_str(),
            left.conditioning_moderator.as_deref(),
        )
            .cmp(&(
                node_order[right.to.as_str()],
                node_order[right.from.as_str()],
                right.moderator.as_str(),
                right.conditioning_moderator.as_deref(),
            ))
    });
    Some((ordered_paths, ordered_moderations))
}

fn project_process_expected_equations(
    focal_predictor: &str,
    outcome: &str,
    predictors: &[String],
    controls: &[String],
    moderators: &[qpls_core::ProcessModeratorConfig],
    ordered_paths: &[qpls_core::ProcessPathConfig],
    ordered_moderations: &[qpls_core::ProcessModerationConfig],
) -> Vec<(String, Vec<ProjectProcessTermSpec>)> {
    let moderator_names = moderators
        .iter()
        .map(|moderator| moderator.variable.as_str())
        .collect::<BTreeSet<_>>();
    let mut outcomes = predictors
        .iter()
        .filter(|variable| {
            variable.as_str() != focal_predictor
                && !moderator_names.contains(variable.as_str())
                && ordered_paths
                    .iter()
                    .any(|path| path.to.as_str() == variable.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    if ordered_paths.iter().any(|path| path.to == outcome) {
        outcomes.push(outcome.to_string());
    }
    outcomes
        .into_iter()
        .map(|equation_outcome| {
            let mut terms = ordered_paths
                .iter()
                .filter(|path| path.to == equation_outcome)
                .map(|path| ProjectProcessTermSpec {
                    term_id: format!("path:{}->{}", path.from, path.to),
                    kind: "path".into(),
                    variables: vec![path.from.clone()],
                })
                .collect::<Vec<_>>();
            let relevant_moderations = ordered_moderations
                .iter()
                .filter(|moderation| moderation.to == equation_outcome)
                .collect::<Vec<_>>();
            for moderator in moderators {
                if relevant_moderations.iter().any(|moderation| {
                    moderation.moderator == moderator.variable
                        || moderation.conditioning_moderator.as_ref() == Some(&moderator.variable)
                }) && !terms
                    .iter()
                    .any(|term| term.variables == [moderator.variable.clone()])
                {
                    terms.push(ProjectProcessTermSpec {
                        term_id: format!("moderator:{}", moderator.variable),
                        kind: "moderator_main".into(),
                        variables: vec![moderator.variable.clone()],
                    });
                }
            }
            let mut interactions = Vec::new();
            for moderation in relevant_moderations {
                let primary = vec![moderation.from.clone(), moderation.moderator.clone()];
                if !interactions
                    .iter()
                    .any(|term: &ProjectProcessTermSpec| term.variables == primary)
                {
                    interactions.push(ProjectProcessTermSpec {
                        term_id: format!(
                            "interaction:{}*{}",
                            moderation.from, moderation.moderator
                        ),
                        kind: "interaction".into(),
                        variables: primary,
                    });
                }
                if let Some(conditioning) = &moderation.conditioning_moderator {
                    for pair in [
                        vec![moderation.from.clone(), conditioning.clone()],
                        vec![moderation.moderator.clone(), conditioning.clone()],
                    ] {
                        if !interactions.iter().any(|term| term.variables == pair) {
                            interactions.push(ProjectProcessTermSpec {
                                term_id: format!("interaction:{}*{}", pair[0], pair[1]),
                                kind: "interaction".into(),
                                variables: pair,
                            });
                        }
                    }
                    interactions.push(ProjectProcessTermSpec {
                        term_id: format!(
                            "interaction:{}*{}*{}",
                            moderation.from, moderation.moderator, conditioning
                        ),
                        kind: "interaction".into(),
                        variables: vec![
                            moderation.from.clone(),
                            moderation.moderator.clone(),
                            conditioning.clone(),
                        ],
                    });
                }
            }
            interactions.sort_by(|left, right| {
                (left.variables.len(), left.term_id.as_str())
                    .cmp(&(right.variables.len(), right.term_id.as_str()))
            });
            interactions.dedup_by(|left, right| left.variables == right.variables);
            terms.extend(interactions);
            terms.extend(controls.iter().map(|control| ProjectProcessTermSpec {
                term_id: format!("control:{control}"),
                kind: "control".into(),
                variables: vec![control.clone()],
            }));
            terms.insert(
                0,
                ProjectProcessTermSpec {
                    term_id: "intercept".into(),
                    kind: "intercept".into(),
                    variables: Vec::new(),
                },
            );
            (equation_outcome, terms)
        })
        .collect()
}

fn validate_process_graph_analysis_contract(
    regression: &RegressionAnalysis,
    focal_predictor: &str,
    paths: &[qpls_core::ProcessPathConfig],
    moderators: &[qpls_core::ProcessModeratorConfig],
    moderations: &[qpls_core::ProcessModerationConfig],
    expected_omitted_observations: usize,
) -> bool {
    const WARNING_1: &str = "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates.";
    const WARNING_2: &str = "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected.";
    let Some(process) = regression.process.as_ref() else {
        return false;
    };
    let Some(graph) = process.graph_v2.as_ref() else {
        return false;
    };
    let Some((ordered_paths, ordered_moderations)) = project_process_canonical_relationships(
        &regression.predictors,
        &regression.outcome,
        paths,
        moderations,
    ) else {
        return false;
    };
    if process.method_version != REGRESSION_PROCESS_METHOD_VERSION
        || process.model != "graph"
        || !process.effects.is_empty()
        || !process.simple_slopes.is_empty()
        || process.warnings != [WARNING_1, WARNING_2]
        || graph.policies.centering != "equation_complete_case_mean_v1"
        || graph.policies.covariance != "hc3_v1"
        || graph.policies.inference_reference != "student_t_residual_df_v1"
        || graph.policies.confidence_level.to_bits() != 0.95_f64.to_bits()
        || graph.complete_cases != regression.observations
        || graph.omitted_cases != expected_omitted_observations
        || graph.complete_cases == 0
        || graph.complete_cases + graph.omitted_cases == 0
    {
        return false;
    }
    if graph.paths.len() != ordered_paths.len()
        || graph
            .paths
            .iter()
            .zip(&ordered_paths)
            .any(|(actual, expected)| {
                actual.path_id != format!("{}->{}", expected.from, expected.to)
                    || actual.from != expected.from
                    || actual.to != expected.to
            })
        || graph.moderations.len() != ordered_moderations.len()
        || graph
            .moderations
            .iter()
            .zip(&ordered_moderations)
            .any(|(actual, expected)| {
                actual.moderation_id
                    != match &expected.conditioning_moderator {
                        Some(conditioning) => format!(
                            "moderation:{}->{}@{}|{}",
                            expected.from, expected.to, expected.moderator, conditioning
                        ),
                        None => format!(
                            "moderation:{}->{}@{}",
                            expected.from, expected.to, expected.moderator
                        ),
                    }
                    || actual.from != expected.from
                    || actual.to != expected.to
                    || actual.moderator != expected.moderator
                    || actual.conditioning_moderator != expected.conditioning_moderator
            })
    {
        return false;
    }
    let moderator_scales = moderators
        .iter()
        .map(|moderator| {
            (
                moderator.variable.as_str(),
                match moderator.scale {
                    qpls_core::ProcessModeratorScale::Continuous => "continuous",
                    qpls_core::ProcessModeratorScale::Binary01 => "binary_0_1",
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected_profiles = regression
        .predictors
        .iter()
        .map(|variable| {
            (
                variable.as_str(),
                if variable == focal_predictor {
                    "focal_predictor"
                } else if moderator_scales.contains_key(variable.as_str()) {
                    "moderator"
                } else {
                    "mediator"
                },
                moderator_scales
                    .get(variable.as_str())
                    .copied()
                    .unwrap_or("continuous"),
            )
        })
        .chain(
            regression
                .controls
                .iter()
                .map(|variable| (variable.as_str(), "control", "continuous")),
        )
        .chain(std::iter::once((
            regression.outcome.as_str(),
            "outcome",
            "continuous",
        )))
        .collect::<Vec<_>>();
    if graph.variable_profiles.len() != expected_profiles.len()
        || graph.variable_profiles.iter().zip(expected_profiles).any(
            |(profile, (variable, role, scale))| {
                profile.variable != variable
                    || profile.role != role
                    || profile.scale != scale
                    || !process_profile_valid(profile)
            },
        )
    {
        return false;
    }
    let expected_equations = project_process_expected_equations(
        focal_predictor,
        &regression.outcome,
        &regression.predictors,
        &regression.controls,
        moderators,
        &ordered_paths,
        &ordered_moderations,
    );
    if graph.equations.len() != expected_equations.len()
        || graph.equations.iter().zip(&expected_equations).any(
            |(equation, (expected_outcome, expected_terms))| {
                equation.equation_id != format!("equation:{expected_outcome}")
                    || equation.outcome != *expected_outcome
                    || equation.term_ids
                        != expected_terms
                            .iter()
                            .map(|term| term.term_id.clone())
                            .collect::<Vec<_>>()
                    || equation.coefficients.len() != expected_terms.len()
                    || equation.coefficients.iter().zip(expected_terms).any(
                        |(coefficient, term)| {
                            coefficient.term_id != term.term_id
                                || coefficient.kind != term.kind
                                || coefficient.variables != term.variables
                        },
                    )
                    || equation.coefficients.is_empty()
                    || equation.coefficients[0].term_id != "intercept"
                    || equation.residual_degrees_of_freedom == 0
                    || equation.coefficient_covariance.len() != equation.coefficients.len()
                    || equation.coefficient_covariance.iter().any(|row| {
                        row.len() != equation.coefficients.len()
                            || row.iter().any(|value| !value.is_finite())
                    })
                    || equation
                        .coefficient_covariance
                        .iter()
                        .enumerate()
                        .any(|(left, row)| {
                            row.iter().enumerate().any(|(right, value)| {
                                !close_enough(*value, equation.coefficient_covariance[right][left])
                            }) || row[left] < 0.0
                                || !close_enough(
                                    row[left].sqrt(),
                                    equation.coefficients[left].standard_error,
                                )
                        })
                    || equation.coefficients.iter().any(|coefficient| {
                        ![
                            coefficient.estimate,
                            coefficient.standard_error,
                            coefficient.statistic,
                            coefficient.p_value_two_sided,
                            coefficient.confidence_interval_lower,
                            coefficient.confidence_interval_upper,
                        ]
                        .iter()
                        .all(|value| value.is_finite())
                            || coefficient.standard_error <= 0.0
                            || !close_enough(
                                coefficient.statistic,
                                coefficient.estimate / coefficient.standard_error,
                            )
                    })
                    || StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64).map_or(
                        true,
                        |distribution| {
                            let critical = distribution
                                .inverse_cdf(0.5 + graph.policies.confidence_level / 2.0);
                            equation.coefficients.iter().any(|coefficient| {
                                let expected_p = (2.0
                                    * (1.0 - distribution.cdf(coefficient.statistic.abs())))
                                .clamp(0.0, 1.0);
                                !(0.0..=1.0).contains(&coefficient.p_value_two_sided)
                                    || !close_enough(coefficient.p_value_two_sided, expected_p)
                                    || !close_enough(
                                        coefficient.confidence_interval_lower,
                                        coefficient.estimate
                                            - critical * coefficient.standard_error,
                                    )
                                    || !close_enough(
                                        coefficient.confidence_interval_upper,
                                        coefficient.estimate
                                            + critical * coefficient.standard_error,
                                    )
                            })
                        },
                    )
                    || !project_process_equation_fit_matches(equation, graph.complete_cases)
            },
        )
    {
        return false;
    }
    validate_process_effect_arithmetic(
        graph,
        focal_predictor,
        &regression.outcome,
        paths,
        moderations,
    ) && validate_process_diagnostic_shapes(graph, moderations, graph.policies.confidence_level)
}

fn project_process_equation_fit_matches(
    equation: &qpls_estimation::ProcessEquation,
    complete_cases: usize,
) -> bool {
    let fit = &equation.fit;
    let n = fit.observations;
    let p = fit.parameter_count;
    if n != complete_cases
        || p != equation.coefficients.len()
        || p < 2
        || n <= p
        || equation.residual_degrees_of_freedom != n - p
        || !fit.residual_sum_squares.is_finite()
        || fit.residual_sum_squares < 0.0
        || !fit.total_sum_squares.is_finite()
        || fit.total_sum_squares < 0.0
    {
        return false;
    }
    let residual_df = (n - p) as f64;
    let r_squared = if fit.total_sum_squares > f64::EPSILON {
        1.0 - fit.residual_sum_squares / fit.total_sum_squares
    } else {
        0.0
    };
    let adjusted_r_squared = 1.0 - (1.0 - r_squared) * (n - 1) as f64 / residual_df;
    let f_statistic = if r_squared < 1.0 {
        (r_squared / (p - 1) as f64) / ((1.0 - r_squared) / residual_df)
    } else {
        0.0
    };
    let sigma2 = (fit.residual_sum_squares / n as f64).max(f64::MIN_POSITIVE);
    let aic = n as f64 * sigma2.ln() + 2.0 * p as f64;
    let bic = n as f64 * sigma2.ln() + (n as f64).ln() * p as f64;
    let rmse = (fit.residual_sum_squares / n as f64).sqrt();
    [
        fit.r_squared,
        fit.adjusted_r_squared,
        fit.f_statistic,
        fit.aic,
        fit.bic,
        fit.rmse,
    ]
    .iter()
    .all(|value| value.is_finite())
        && close_enough(fit.r_squared, r_squared)
        && close_enough(fit.adjusted_r_squared, adjusted_r_squared)
        && close_enough(fit.f_statistic, f_statistic)
        && close_enough(fit.aic, aic)
        && close_enough(fit.bic, bic)
        && close_enough(fit.rmse, rmse)
}

fn process_profile_valid(profile: &qpls_estimation::ProcessVariableProfile) -> bool {
    [
        profile.raw_mean,
        profile.raw_sample_sd,
        profile.raw_min,
        profile.raw_max,
    ]
    .iter()
    .all(|value| value.is_finite())
        && profile.raw_sample_sd >= 0.0
        && profile.raw_min <= profile.raw_max
        && matches!(
            profile.role.as_str(),
            "focal_predictor" | "mediator" | "moderator" | "outcome" | "control"
        )
        && matches!(profile.scale.as_str(), "continuous" | "binary_0_1")
        && if profile.scale == "binary_0_1" {
            profile.levels == [0.0, 1.0] && profile.raw_min >= 0.0 && profile.raw_max <= 1.0
        } else {
            profile.levels.is_empty()
        }
}

fn project_process_semantic_probe_levels(
    profile: &qpls_estimation::ProcessVariableProfile,
) -> Option<Vec<(f64, &'static str)>> {
    if profile.scale == "binary_0_1" {
        return Some(vec![(0.0, "binary_0"), (1.0, "binary_1")]);
    }
    let levels = [
        profile.raw_mean - profile.raw_sample_sd,
        profile.raw_mean,
        profile.raw_mean + profile.raw_sample_sd,
    ];
    (levels.iter().all(|value| value.is_finite()) && levels[0] < levels[1] && levels[1] < levels[2])
        .then(|| {
            vec![
                (levels[0], "minus_1sd"),
                (levels[1], "mean"),
                (levels[2], "plus_1sd"),
            ]
        })
}

struct ProjectProcessSemanticProbe {
    values: Vec<qpls_estimation::ProcessModeratorValue>,
    suffix: String,
}

fn project_process_probe_value(
    profile: &qpls_estimation::ProcessVariableProfile,
    raw_value: f64,
) -> qpls_estimation::ProcessModeratorValue {
    qpls_estimation::ProcessModeratorValue {
        variable: profile.variable.clone(),
        raw_value,
        coded_value: if profile.scale == "binary_0_1" {
            raw_value
        } else {
            raw_value - profile.raw_mean
        },
    }
}

fn project_process_probe_grid(
    graph: &ProcessGraphAnalysis,
    moderation: &qpls_core::ProcessModerationConfig,
) -> Option<Vec<ProjectProcessSemanticProbe>> {
    let primary = graph
        .variable_profiles
        .iter()
        .find(|profile| profile.variable == moderation.moderator)?;
    let conditioning = if let Some(variable) = &moderation.conditioning_moderator {
        Some(
            graph
                .variable_profiles
                .iter()
                .find(|profile| profile.variable == *variable)?,
        )
    } else {
        None
    };
    let conditioning_values = if let Some(profile) = conditioning {
        project_process_semantic_probe_levels(profile)?
    } else {
        vec![(0.0, "")]
    };
    let mut grid = Vec::new();
    for (raw_primary, primary_token) in project_process_semantic_probe_levels(primary)? {
        for (raw_conditioning, conditioning_token) in &conditioning_values {
            let mut row = vec![project_process_probe_value(primary, raw_primary)];
            let mut suffix = format!("{}={primary_token}", moderation.moderator);
            if let Some(conditioning) = conditioning {
                row.push(project_process_probe_value(conditioning, *raw_conditioning));
                suffix.push_str(&format!(",{}={conditioning_token}", conditioning.variable));
            }
            grid.push(ProjectProcessSemanticProbe {
                values: row,
                suffix,
            });
        }
    }
    Some(grid)
}

fn project_process_conditioning_grid(
    graph: &ProcessGraphAnalysis,
    moderation: &qpls_core::ProcessModerationConfig,
) -> Option<Vec<Vec<qpls_estimation::ProcessModeratorValue>>> {
    match moderation.conditioning_moderator.as_ref() {
        Some(variable) => {
            let profile = graph
                .variable_profiles
                .iter()
                .find(|profile| profile.variable == *variable)?;
            Some(
                project_process_semantic_probe_levels(profile)?
                    .into_iter()
                    .map(|(raw, _)| vec![project_process_probe_value(profile, raw)])
                    .collect(),
            )
        }
        None => Some(vec![Vec::new()]),
    }
}

fn project_process_probe_row_matches(
    actual: &[qpls_estimation::ProcessModeratorValue],
    expected: &[qpls_estimation::ProcessModeratorValue],
) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            actual.variable == expected.variable
                && close_enough(actual.raw_value, expected.raw_value)
                && close_enough(actual.coded_value, expected.coded_value)
        })
}

fn project_process_moderation<'a>(
    emitted: &qpls_estimation::ProcessModeration,
    moderations: &'a [qpls_core::ProcessModerationConfig],
) -> Option<&'a qpls_core::ProcessModerationConfig> {
    moderations.iter().find(|candidate| {
        candidate.from == emitted.from
            && candidate.to == emitted.to
            && candidate.moderator == emitted.moderator
            && candidate.conditioning_moderator == emitted.conditioning_moderator
    })
}

fn validate_process_diagnostic_shapes(
    graph: &ProcessGraphAnalysis,
    moderations: &[qpls_core::ProcessModerationConfig],
    confidence_level: f64,
) -> bool {
    let mut expected_slopes = Vec::new();
    let mut expected_jn = Vec::new();
    for emitted in &graph.moderations {
        let Some(moderation) = project_process_moderation(emitted, moderations) else {
            return false;
        };
        let Some(grid) = project_process_probe_grid(graph, moderation) else {
            return false;
        };
        expected_slopes.extend(
            grid.into_iter()
                .map(|probe| (emitted.moderation_id.as_str(), probe.suffix, probe.values)),
        );
        let Some(conditioning_grid) = project_process_conditioning_grid(graph, moderation) else {
            return false;
        };
        expected_jn.extend(conditioning_grid.into_iter().map(|conditioning| {
            (
                emitted.moderation_id.as_str(),
                moderation.moderator.as_str(),
                conditioning,
            )
        }));
    }
    if graph.simple_slopes.len() != expected_slopes.len()
        || graph.simple_slopes.iter().zip(&expected_slopes).any(
            |(slope, (moderation_id, suffix, probe))| {
                slope.moderation_id != *moderation_id
                    || slope.effect_id != format!("slope:{moderation_id}@{suffix}")
                    || !project_process_probe_row_matches(&slope.moderator_values, probe)
            },
        )
        || graph.johnson_neyman.len() != expected_jn.len()
        || graph.johnson_neyman.iter().zip(&expected_jn).any(
            |(jn, (moderation_id, solved, conditioning))| match jn {
                qpls_estimation::ProcessJohnsonNeyman::Available {
                    moderation_id: actual_id,
                    solved_moderator,
                    conditioning_values,
                    ..
                }
                | qpls_estimation::ProcessJohnsonNeyman::Unavailable {
                    moderation_id: actual_id,
                    solved_moderator,
                    conditioning_values,
                    ..
                } => {
                    actual_id != *moderation_id
                        || solved_moderator != *solved
                        || !project_process_probe_row_matches(conditioning_values, conditioning)
                }
            },
        )
    {
        return false;
    }
    graph.simple_slopes.iter().all(|slope| {
        [
            slope.estimate,
            slope.standard_error,
            slope.statistic,
            slope.p_value_two_sided,
            slope.confidence_interval_lower,
            slope.confidence_interval_upper,
        ]
        .iter()
        .all(|value| value.is_finite())
            && slope.standard_error > 0.0
            && close_enough(slope.statistic, slope.estimate / slope.standard_error)
            && project_process_slope_matches(graph, slope, moderations, confidence_level)
    }) && graph.plots.len() == graph.moderations.len()
        && graph
            .plots
            .iter()
            .zip(&graph.moderations)
            .all(|(plot, moderation)| {
                plot.moderation_id == moderation.moderation_id
                    && project_process_plot_matches(graph, plot, moderations, confidence_level)
            })
        && graph.johnson_neyman.iter().all(|jn| match jn {
            qpls_estimation::ProcessJohnsonNeyman::Available {
                moderation_id,
                solved_moderator,
                conditioning_values,
                raw_min,
                raw_max,
                roots,
                regions,
                curve_points,
                ..
            } => {
                raw_min.is_finite()
                    && raw_max.is_finite()
                    && raw_min <= raw_max
                    && roots.windows(2).all(|pair| pair[0] <= pair[1])
                    && roots
                        .iter()
                        .all(|root| root.is_finite() && root >= raw_min && root <= raw_max)
                    && regions.len() == roots.len() + 1
                    && curve_points.len() == 101
                    && project_process_jn_curve_matches(
                        graph,
                        moderation_id,
                        solved_moderator,
                        conditioning_values,
                        curve_points,
                        moderations,
                        confidence_level,
                    )
                    && project_process_jn_regions_match(
                        graph,
                        moderation_id,
                        solved_moderator,
                        conditioning_values,
                        *raw_min,
                        *raw_max,
                        roots,
                        regions,
                        moderations,
                        confidence_level,
                    )
            }
            qpls_estimation::ProcessJohnsonNeyman::Unavailable {
                moderation_id,
                solved_moderator,
                conditioning_values,
                reason_code,
                message,
            } => project_process_jn_unavailable_matches(
                graph,
                moderation_id,
                solved_moderator,
                conditioning_values,
                reason_code,
                message,
                moderations,
            ),
        })
}

fn project_process_jn_unavailable_matches(
    graph: &ProcessGraphAnalysis,
    moderation_id: &str,
    solved_moderator: &str,
    conditioning_values: &[qpls_estimation::ProcessModeratorValue],
    reason_code: &str,
    message: &str,
    moderations: &[qpls_core::ProcessModerationConfig],
) -> bool {
    let Some(emitted) = graph
        .moderations
        .iter()
        .find(|moderation| moderation.moderation_id == moderation_id)
    else {
        return false;
    };
    let Some(moderation) = project_process_moderation(emitted, moderations) else {
        return false;
    };
    let Some(profile) = graph
        .variable_profiles
        .iter()
        .find(|profile| profile.variable == solved_moderator)
    else {
        return false;
    };
    let Some(equation) = graph
        .equations
        .iter()
        .find(|equation| equation.outcome == moderation.to)
    else {
        return false;
    };
    if profile.scale == "binary_0_1" {
        reason_code == "binary_solved_moderator"
            && message == "Johnson-Neyman regions require a continuous solved moderator."
    } else {
        let mut probes_zero = conditioning_values
            .iter()
            .map(|value| (value.variable.clone(), value.coded_value))
            .collect::<BTreeMap<_, _>>();
        probes_zero.insert(solved_moderator.to_string(), 0.0);
        let mut probes_one = probes_zero.clone();
        probes_one.insert(solved_moderator.to_string(), 1.0);
        let weights_zero = project_process_slope_weights(equation, moderation, &probes_zero);
        let weights_one = project_process_slope_weights(equation, moderation, &probes_one);
        let weights_delta = weights_one
            .iter()
            .zip(&weights_zero)
            .map(|(one, zero)| one - zero)
            .collect::<Vec<_>>();
        let a = equation
            .coefficients
            .iter()
            .zip(&weights_zero)
            .map(|(coefficient, weight)| coefficient.estimate * weight)
            .sum::<f64>();
        let b = equation
            .coefficients
            .iter()
            .zip(&weights_delta)
            .map(|(coefficient, weight)| coefficient.estimate * weight)
            .sum::<f64>();
        let covariance_form = |left: &[f64], right: &[f64]| {
            left.iter()
                .enumerate()
                .map(|(row, left_weight)| {
                    right
                        .iter()
                        .enumerate()
                        .map(|(column, right_weight)| {
                            left_weight
                                * equation.coefficient_covariance[row][column]
                                * right_weight
                        })
                        .sum::<f64>()
                })
                .sum::<f64>()
        };
        let v0 = covariance_form(&weights_zero, &weights_zero);
        let v1 = covariance_form(&weights_zero, &weights_delta);
        let v2 = covariance_form(&weights_delta, &weights_delta);
        let covariance_finite = [v0, v1, v2].iter().all(|value| value.is_finite());
        if !a.is_finite() || !b.is_finite() || !covariance_finite {
            reason_code == qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_REASON
                && message == qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        } else {
            let coded_min = profile.raw_min - profile.raw_mean;
            let coded_max = profile.raw_max - profile.raw_mean;
            !project_process_jn_variance_is_positive_across_range(v0, v1, v2, coded_min, coded_max)
                && reason_code == qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_REASON
                && message == qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        }
    }
}

fn project_process_plot_matches(
    graph: &ProcessGraphAnalysis,
    plot: &qpls_estimation::ProcessPlot,
    moderations: &[qpls_core::ProcessModerationConfig],
    confidence_level: f64,
) -> bool {
    let Some(emitted_moderation) = graph
        .moderations
        .iter()
        .find(|row| row.moderation_id == plot.moderation_id)
    else {
        return false;
    };
    let Some(moderation) = moderations.iter().find(|candidate| {
        candidate.from == emitted_moderation.from
            && candidate.to == emitted_moderation.to
            && candidate.moderator == emitted_moderation.moderator
            && candidate.conditioning_moderator == emitted_moderation.conditioning_moderator
    }) else {
        return false;
    };
    let profiles = graph
        .variable_profiles
        .iter()
        .map(|profile| (profile.variable.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    let Some(focal_profile) = profiles.get(moderation.from.as_str()).copied() else {
        return false;
    };
    let Some(equation) = graph
        .equations
        .iter()
        .find(|equation| equation.outcome == moderation.to)
    else {
        return false;
    };
    let Ok(distribution) = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
    else {
        return false;
    };
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let Some(expected_grid) = project_process_probe_grid(graph, moderation) else {
        return false;
    };
    plot.plot_id == format!("plot:{}", plot.moderation_id)
        && plot.series.len() == expected_grid.len()
        && plot.series.iter().zip(&expected_grid).enumerate().all(
            |(series_index, (series, expected_probe))| {
                if series.series_id != format!("series:{series_index}:{}", expected_probe.suffix)
                    || series.points.len() != 25
                    || !project_process_probe_row_matches(
                        &series.moderator_values,
                        &expected_probe.values,
                    )
                {
                    return false;
                }
                series
                    .points
                    .iter()
                    .enumerate()
                    .all(|(point_index, point)| {
                        let predictor_raw = focal_profile.raw_min
                            + (focal_profile.raw_max - focal_profile.raw_min) * point_index as f64
                                / 24.0;
                        let mut raw_values = profiles
                            .values()
                            .map(|profile| {
                                (
                                    profile.variable.clone(),
                                    if profile.scale == "binary_0_1" {
                                        0.0
                                    } else {
                                        profile.raw_mean
                                    },
                                )
                            })
                            .collect::<BTreeMap<_, _>>();
                        raw_values.insert(moderation.from.clone(), predictor_raw);
                        for value in &series.moderator_values {
                            raw_values.insert(value.variable.clone(), value.raw_value);
                        }
                        let design = equation
                            .coefficients
                            .iter()
                            .map(|coefficient| {
                                if coefficient.kind == "intercept" {
                                    Some(1.0)
                                } else {
                                    coefficient.variables.iter().try_fold(
                                        1.0,
                                        |product, variable| {
                                            let raw = raw_values.get(variable)?;
                                            let profile = profiles.get(variable.as_str())?;
                                            let value = if coefficient.variables.len() > 1
                                                && profile.scale != "binary_0_1"
                                            {
                                                raw - profile.raw_mean
                                            } else {
                                                *raw
                                            };
                                            Some(product * value)
                                        },
                                    )
                                }
                            })
                            .collect::<Option<Vec<_>>>();
                        let Some(design) = design else {
                            return false;
                        };
                        project_process_linear_combination(equation, &design).is_some_and(
                            |(predicted, standard_error)| {
                                close_enough(point.predictor_raw, predictor_raw)
                                    && close_enough(point.predicted_raw, predicted)
                                    && close_enough(
                                        point.confidence_interval_lower,
                                        predicted - critical * standard_error,
                                    )
                                    && close_enough(
                                        point.confidence_interval_upper,
                                        predicted + critical * standard_error,
                                    )
                            },
                        )
                    })
            },
        )
}

fn project_process_slope_weights(
    equation: &qpls_estimation::ProcessEquation,
    moderation: &qpls_core::ProcessModerationConfig,
    probes: &BTreeMap<String, f64>,
) -> Vec<f64> {
    let primary = probes.get(&moderation.moderator).copied().unwrap_or(0.0);
    let conditioning = moderation
        .conditioning_moderator
        .as_ref()
        .and_then(|variable| probes.get(variable))
        .copied()
        .unwrap_or(0.0);
    equation
        .coefficients
        .iter()
        .map(|coefficient| {
            if coefficient.variables == [moderation.from.clone()] {
                1.0
            } else if coefficient.variables
                == [moderation.from.clone(), moderation.moderator.clone()]
            {
                primary
            } else if moderation
                .conditioning_moderator
                .as_ref()
                .is_some_and(|variable| {
                    coefficient.variables == [moderation.from.clone(), variable.clone()]
                })
            {
                conditioning
            } else if moderation
                .conditioning_moderator
                .as_ref()
                .is_some_and(|variable| {
                    coefficient.variables
                        == [
                            moderation.from.clone(),
                            moderation.moderator.clone(),
                            variable.clone(),
                        ]
                })
            {
                primary * conditioning
            } else {
                0.0
            }
        })
        .collect()
}

fn project_process_contrast(
    equation: &qpls_estimation::ProcessEquation,
    weights: &[f64],
) -> Option<(f64, f64)> {
    if weights.len() != equation.coefficients.len()
        || equation.coefficient_covariance.len() != weights.len()
    {
        return None;
    }
    let estimate = equation
        .coefficients
        .iter()
        .zip(weights)
        .map(|(coefficient, weight)| coefficient.estimate * weight)
        .sum::<f64>();
    let variance = weights
        .iter()
        .enumerate()
        .map(|(left, left_weight)| {
            weights
                .iter()
                .enumerate()
                .map(|(right, right_weight)| {
                    left_weight * equation.coefficient_covariance[left][right] * right_weight
                })
                .sum::<f64>()
        })
        .sum::<f64>();
    (estimate.is_finite() && variance.is_finite()).then_some((estimate, variance))
}

fn project_process_linear_combination(
    equation: &qpls_estimation::ProcessEquation,
    weights: &[f64],
) -> Option<(f64, f64)> {
    project_process_contrast(equation, weights)
        .and_then(|(estimate, variance)| (variance >= 0.0).then_some((estimate, variance.sqrt())))
}

fn project_process_jn_variance(v0: f64, v1: f64, v2: f64, coded: f64) -> Option<f64> {
    let variance = v0 + 2.0 * v1 * coded + v2 * coded * coded;
    (variance.is_finite() && variance > 0.0).then_some(variance)
}

fn project_process_jn_variance_is_positive_across_range(
    v0: f64,
    v1: f64,
    v2: f64,
    coded_min: f64,
    coded_max: f64,
) -> bool {
    if project_process_jn_variance(v0, v1, v2, coded_min).is_none()
        || project_process_jn_variance(v0, v1, v2, coded_max).is_none()
    {
        return false;
    }
    if v2 > 0.0 {
        let vertex = -v1 / v2;
        if vertex > coded_min
            && vertex < coded_max
            && project_process_jn_variance(v0, v1, v2, vertex).is_none()
        {
            return false;
        }
    }
    true
}

fn project_process_slope_matches(
    graph: &ProcessGraphAnalysis,
    slope: &qpls_estimation::ProcessGraphSimpleSlope,
    moderations: &[qpls_core::ProcessModerationConfig],
    confidence_level: f64,
) -> bool {
    let Some(emitted_moderation) = graph
        .moderations
        .iter()
        .find(|row| row.moderation_id == slope.moderation_id)
    else {
        return false;
    };
    let Some(moderation) = moderations.iter().find(|candidate| {
        candidate.from == emitted_moderation.from
            && candidate.to == emitted_moderation.to
            && candidate.moderator == emitted_moderation.moderator
            && candidate.conditioning_moderator == emitted_moderation.conditioning_moderator
    }) else {
        return false;
    };
    let Some(equation) = graph
        .equations
        .iter()
        .find(|equation| equation.outcome == moderation.to)
    else {
        return false;
    };
    let probes = slope
        .moderator_values
        .iter()
        .map(|value| (value.variable.clone(), value.coded_value))
        .collect::<BTreeMap<_, _>>();
    let weights = project_process_slope_weights(equation, moderation, &probes);
    let Some((estimate, standard_error)) = project_process_linear_combination(equation, &weights)
    else {
        return false;
    };
    let Ok(distribution) = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
    else {
        return false;
    };
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let expected_p = if standard_error > 0.0 {
        2.0 * (1.0 - distribution.cdf((estimate / standard_error).abs()))
    } else {
        1.0
    };
    close_enough(slope.estimate, estimate)
        && close_enough(slope.standard_error, standard_error)
        && close_enough(slope.p_value_two_sided, expected_p)
        && close_enough(
            slope.confidence_interval_lower,
            estimate - critical * standard_error,
        )
        && close_enough(
            slope.confidence_interval_upper,
            estimate + critical * standard_error,
        )
}

fn project_process_jn_curve_matches(
    graph: &ProcessGraphAnalysis,
    moderation_id: &str,
    solved_moderator: &str,
    conditioning_values: &[qpls_estimation::ProcessModeratorValue],
    curve_points: &[qpls_estimation::ProcessJohnsonNeymanPoint],
    moderations: &[qpls_core::ProcessModerationConfig],
    confidence_level: f64,
) -> bool {
    let Some(emitted_moderation) = graph
        .moderations
        .iter()
        .find(|row| row.moderation_id == moderation_id)
    else {
        return false;
    };
    let Some(moderation) = moderations.iter().find(|candidate| {
        candidate.from == emitted_moderation.from
            && candidate.to == emitted_moderation.to
            && candidate.moderator == emitted_moderation.moderator
            && candidate.conditioning_moderator == emitted_moderation.conditioning_moderator
    }) else {
        return false;
    };
    if moderation.moderator != solved_moderator {
        return false;
    }
    let Some(profile) = graph
        .variable_profiles
        .iter()
        .find(|profile| profile.variable == solved_moderator)
    else {
        return false;
    };
    let Some(equation) = graph
        .equations
        .iter()
        .find(|equation| equation.outcome == moderation.to)
    else {
        return false;
    };
    let Ok(distribution) = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
    else {
        return false;
    };
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    curve_points.iter().enumerate().all(|(index, point)| {
        let expected_raw =
            profile.raw_min + (profile.raw_max - profile.raw_min) * index as f64 / 100.0;
        let mut probes = conditioning_values
            .iter()
            .map(|value| (value.variable.clone(), value.coded_value))
            .collect::<BTreeMap<_, _>>();
        probes.insert(
            solved_moderator.to_string(),
            point.moderator_raw - profile.raw_mean,
        );
        let weights = project_process_slope_weights(equation, moderation, &probes);
        project_process_contrast(equation, &weights).is_some_and(|(estimate, variance)| {
            if variance <= 0.0 {
                return false;
            }
            let standard_error = variance.sqrt();
            close_enough(point.moderator_raw, expected_raw)
                && close_enough(point.effect, estimate)
                && close_enough(point.standard_error, standard_error)
                && close_enough(
                    point.confidence_interval_lower,
                    estimate - critical * standard_error,
                )
                && close_enough(
                    point.confidence_interval_upper,
                    estimate + critical * standard_error,
                )
        })
    })
}

fn project_process_jn_regions_match(
    graph: &ProcessGraphAnalysis,
    moderation_id: &str,
    solved_moderator: &str,
    conditioning_values: &[qpls_estimation::ProcessModeratorValue],
    raw_min: f64,
    raw_max: f64,
    roots: &[f64],
    regions: &[qpls_estimation::ProcessJohnsonNeymanRegion],
    moderations: &[qpls_core::ProcessModerationConfig],
    confidence_level: f64,
) -> bool {
    let Some(emitted_moderation) = graph
        .moderations
        .iter()
        .find(|row| row.moderation_id == moderation_id)
    else {
        return false;
    };
    let Some(moderation) = project_process_moderation(emitted_moderation, moderations) else {
        return false;
    };
    let Some(profile) = graph
        .variable_profiles
        .iter()
        .find(|profile| profile.variable == solved_moderator)
    else {
        return false;
    };
    let Some(equation) = graph
        .equations
        .iter()
        .find(|equation| equation.outcome == moderation.to)
    else {
        return false;
    };
    let Ok(distribution) = StudentsT::new(0.0, 1.0, equation.residual_degrees_of_freedom as f64)
    else {
        return false;
    };
    let critical = distribution.inverse_cdf(0.5 + confidence_level / 2.0);
    let mut probes_zero = conditioning_values
        .iter()
        .map(|value| (value.variable.clone(), value.coded_value))
        .collect::<BTreeMap<_, _>>();
    probes_zero.insert(solved_moderator.to_string(), 0.0);
    let mut probes_one = probes_zero.clone();
    probes_one.insert(solved_moderator.to_string(), 1.0);
    let weights_zero = project_process_slope_weights(equation, moderation, &probes_zero);
    let weights_one = project_process_slope_weights(equation, moderation, &probes_one);
    let weights_delta = weights_one
        .iter()
        .zip(&weights_zero)
        .map(|(one, zero)| one - zero)
        .collect::<Vec<_>>();
    let a = equation
        .coefficients
        .iter()
        .zip(&weights_zero)
        .map(|(coefficient, weight)| coefficient.estimate * weight)
        .sum::<f64>();
    let b = equation
        .coefficients
        .iter()
        .zip(&weights_delta)
        .map(|(coefficient, weight)| coefficient.estimate * weight)
        .sum::<f64>();
    let covariance_form = |left: &[f64], right: &[f64]| {
        left.iter()
            .enumerate()
            .map(|(row, left_weight)| {
                right
                    .iter()
                    .enumerate()
                    .map(|(column, right_weight)| {
                        left_weight * equation.coefficient_covariance[row][column] * right_weight
                    })
                    .sum::<f64>()
            })
            .sum::<f64>()
    };
    let v0 = covariance_form(&weights_zero, &weights_zero);
    let v1 = covariance_form(&weights_zero, &weights_delta);
    let v2 = covariance_form(&weights_delta, &weights_delta);
    let qa = b * b - critical * critical * v2;
    let qb = 2.0 * (a * b - critical * critical * v1);
    let qc = a * a - critical * critical * v0;
    let coded_min = profile.raw_min - profile.raw_mean;
    let coded_max = profile.raw_max - profile.raw_mean;
    if !project_process_jn_variance_is_positive_across_range(v0, v1, v2, coded_min, coded_max) {
        return false;
    }
    let expected_coded_roots =
        qpls_estimation::process_johnson_neyman_coded_roots(qa, qb, qc, coded_min, coded_max);
    let expected_roots = expected_coded_roots
        .iter()
        .map(|root| root + profile.raw_mean)
        .collect::<Vec<_>>();
    let mut boundaries = vec![profile.raw_min];
    boundaries.extend(roots.iter().copied());
    boundaries.push(profile.raw_max);
    close_enough(raw_min, profile.raw_min)
        && close_enough(raw_max, profile.raw_max)
        && roots.len() == expected_roots.len()
        && roots
            .iter()
            .zip(&expected_roots)
            .all(|(actual, expected)| close_enough(*actual, *expected))
        && roots.iter().all(|root| {
            let mut probes = conditioning_values
                .iter()
                .map(|value| (value.variable.clone(), value.coded_value))
                .collect::<BTreeMap<_, _>>();
            probes.insert(solved_moderator.to_string(), root - profile.raw_mean);
            let weights = project_process_slope_weights(equation, moderation, &probes);
            project_process_linear_combination(equation, &weights).is_some_and(
                |(estimate, standard_error)| {
                    close_enough(estimate.abs(), critical * standard_error)
                },
            )
        })
        && regions.len() == boundaries.len() - 1
        && regions
            .iter()
            .zip(boundaries.windows(2))
            .all(|(region, bounds)| {
                let raw = (bounds[0] + bounds[1]) / 2.0;
                let mut probes = conditioning_values
                    .iter()
                    .map(|value| (value.variable.clone(), value.coded_value))
                    .collect::<BTreeMap<_, _>>();
                probes.insert(solved_moderator.to_string(), raw - profile.raw_mean);
                let weights = project_process_slope_weights(equation, moderation, &probes);
                project_process_linear_combination(equation, &weights).is_some_and(
                    |(estimate, standard_error)| {
                        let margin = critical * standard_error;
                        let status = if estimate + margin < 0.0 {
                            "significant_negative"
                        } else if estimate - margin > 0.0 {
                            "significant_positive"
                        } else {
                            "not_significant"
                        };
                        close_enough(region.lower, bounds[0])
                            && close_enough(region.upper, bounds[1])
                            && region.status == status
                    },
                )
            })
}

fn validate_process_effect_arithmetic(
    graph: &ProcessGraphAnalysis,
    focal: &str,
    outcome: &str,
    paths: &[qpls_core::ProcessPathConfig],
    moderations: &[qpls_core::ProcessModerationConfig],
) -> bool {
    let simple_paths = project_process_paths(focal, outcome, paths);
    let direct_path = vec![focal.to_string(), outcome.to_string()];
    let direct = if simple_paths.contains(&direct_path) {
        project_process_path_effect(graph, &direct_path, &BTreeMap::new(), moderations)
    } else {
        Some(0.0)
    };
    let Some(direct) = direct else {
        return false;
    };
    let mut expected = vec![(
        format!("direct:{focal}->{outcome}"),
        "direct",
        direct_path.clone(),
        direct,
    )];
    let mut total_indirect = 0.0;
    for path in simple_paths.iter().filter(|path| path.len() > 2) {
        let Some(estimate) =
            project_process_path_effect(graph, path, &BTreeMap::new(), moderations)
        else {
            return false;
        };
        total_indirect += estimate;
        expected.push((
            format!("indirect:{}", path.join("->")),
            "indirect",
            path.clone(),
            estimate,
        ));
    }
    expected.push((
        format!("total_indirect:{focal}->{outcome}"),
        "total_indirect",
        direct_path.clone(),
        total_indirect,
    ));
    expected.push((
        format!("total:{focal}->{outcome}"),
        "total",
        direct_path,
        direct + total_indirect,
    ));
    if graph.reference_effects.len() != expected.len()
        || graph.reference_effects.iter().zip(expected).any(
            |(actual, (effect_id, kind, path, estimate))| {
                actual.effect_id != effect_id
                    || actual.kind != kind
                    || actual.path != path
                    || !close_enough(actual.estimate, estimate)
            },
        )
    {
        return false;
    }
    let mut expected_conditional = Vec::new();
    for path in simple_paths.iter().filter(|path| path.len() > 2) {
        if let Some(moderation) = path.windows(2).find_map(|edge| {
            moderations
                .iter()
                .find(|item| item.from == edge[0] && item.to == edge[1])
        }) {
            let Some(grid) = project_process_probe_grid(graph, moderation) else {
                return false;
            };
            let path_id = path.join("->");
            expected_conditional.extend(
                grid.into_iter()
                    .map(|probe| (path_id.clone(), probe.suffix, probe.values)),
            );
        }
    }
    if graph.conditional_indirect_effects.len() != expected_conditional.len() {
        return false;
    }
    for (actual, (expected_path_id, expected_suffix, expected_probe)) in graph
        .conditional_indirect_effects
        .iter()
        .zip(&expected_conditional)
    {
        let path = actual
            .path_id
            .split("->")
            .map(str::to_string)
            .collect::<Vec<_>>();
        if actual.path_id != *expected_path_id
            || !simple_paths.contains(&path)
            || path.len() <= 2
            || !project_process_probe_row_matches(&actual.moderator_values, expected_probe)
            || actual.effect_id != format!("indirect:{expected_path_id}@{expected_suffix}")
        {
            return false;
        }
        let probes = actual
            .moderator_values
            .iter()
            .map(|value| (value.variable.clone(), value.coded_value))
            .collect::<BTreeMap<_, _>>();
        let Some(expected_estimate) =
            project_process_path_effect(graph, &path, &probes, moderations)
        else {
            return false;
        };
        if !close_enough(actual.estimate, expected_estimate) {
            return false;
        }
    }
    let expected_indices = simple_paths
        .iter()
        .filter(|path| path.len() > 2)
        .filter_map(|path| {
            path.windows(2)
                .find_map(|edge| {
                    moderations
                        .iter()
                        .find(|item| item.from == edge[0] && item.to == edge[1])
                })
                .map(|moderation| (path, moderation))
        })
        .collect::<Vec<_>>();
    if graph.moderated_mediation_indices.len() != expected_indices.len() {
        return false;
    }
    for (actual, (path, moderation)) in graph
        .moderated_mediation_indices
        .iter()
        .zip(expected_indices)
    {
        let path_id = path.join("->");
        let moderated_edge = format!("{}->{}", moderation.from, moderation.to);
        if actual.path_id != path_id
            || actual.moderated_edge != moderated_edge
            || actual.moderator != moderation.moderator
            || actual.effect_id
                != format!(
                    "index:{}:{}->{}:{}",
                    path_id, moderation.from, moderation.to, moderation.moderator
                )
        {
            return false;
        }
        let Some(interaction) = project_process_coefficient(
            graph,
            &moderation.to,
            &[moderation.from.clone(), moderation.moderator.clone()],
        ) else {
            return false;
        };
        let mut other_product = 1.0;
        for edge in path.windows(2) {
            if edge[0] == moderation.from && edge[1] == moderation.to {
                continue;
            }
            let Some(slope) = project_process_edge_slope(
                graph,
                &edge[0],
                &edge[1],
                &BTreeMap::new(),
                moderations,
            ) else {
                return false;
            };
            other_product *= slope;
        }
        if !close_enough(actual.estimate, interaction.estimate * other_product) {
            return false;
        }
    }
    true
}

fn project_process_paths(
    focal: &str,
    outcome: &str,
    paths: &[qpls_core::ProcessPathConfig],
) -> Vec<Vec<String>> {
    fn visit(
        node: &str,
        outcome: &str,
        paths: &[qpls_core::ProcessPathConfig],
        current: &mut Vec<String>,
        result: &mut Vec<Vec<String>>,
    ) {
        if node == outcome {
            result.push(current.clone());
            return;
        }
        let mut outgoing = paths
            .iter()
            .filter(|path| path.from == node)
            .collect::<Vec<_>>();
        outgoing.sort_by(|left, right| left.to.cmp(&right.to));
        for path in outgoing {
            if !current.contains(&path.to) {
                current.push(path.to.clone());
                visit(&path.to, outcome, paths, current, result);
                current.pop();
            }
        }
    }
    let mut result = Vec::new();
    visit(
        focal,
        outcome,
        paths,
        &mut vec![focal.to_string()],
        &mut result,
    );
    result.sort();
    result
}

fn project_process_coefficient<'a>(
    graph: &'a ProcessGraphAnalysis,
    outcome: &str,
    variables: &[String],
) -> Option<&'a qpls_estimation::ProcessEquationCoefficient> {
    graph
        .equations
        .iter()
        .find(|equation| equation.outcome == outcome)?
        .coefficients
        .iter()
        .find(|coefficient| coefficient.variables == variables)
}

fn project_process_edge_slope(
    graph: &ProcessGraphAnalysis,
    from: &str,
    to: &str,
    probes: &BTreeMap<String, f64>,
    moderations: &[qpls_core::ProcessModerationConfig],
) -> Option<f64> {
    let mut slope = project_process_coefficient(graph, to, &[from.to_string()])?.estimate;
    let Some(moderation) = moderations
        .iter()
        .find(|moderation| moderation.from == from && moderation.to == to)
    else {
        return Some(slope);
    };
    let primary = probes.get(&moderation.moderator).copied().unwrap_or(0.0);
    slope +=
        project_process_coefficient(graph, to, &[from.to_string(), moderation.moderator.clone()])?
            .estimate
            * primary;
    if let Some(conditioning) = &moderation.conditioning_moderator {
        let conditioning_value = probes.get(conditioning).copied().unwrap_or(0.0);
        slope += project_process_coefficient(graph, to, &[from.to_string(), conditioning.clone()])?
            .estimate
            * conditioning_value;
        slope += project_process_coefficient(
            graph,
            to,
            &[
                from.to_string(),
                moderation.moderator.clone(),
                conditioning.clone(),
            ],
        )?
        .estimate
            * primary
            * conditioning_value;
    }
    Some(slope)
}

fn project_process_path_effect(
    graph: &ProcessGraphAnalysis,
    path: &[String],
    probes: &BTreeMap<String, f64>,
    moderations: &[qpls_core::ProcessModerationConfig],
) -> Option<f64> {
    path.windows(2).try_fold(1.0, |effect, edge| {
        Some(effect * project_process_edge_slope(graph, &edge[0], &edge[1], probes, moderations)?)
    })
}

fn validate_nca_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
    assessment_method_version: &str,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let Some(expected_x) = recipe
        .metadata
        .get("nca_x")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(expected_y) = recipe
        .metadata
        .get("nca_y")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let expected_ceiling = recipe
        .metadata
        .get("nca_ceiling")
        .map(|value| value.trim())
        .unwrap_or("both");
    let Some(expected_permutations) = recipe
        .metadata
        .get("nca_permutation_samples")
        .map(|value| value.trim())
        .unwrap_or("999")
        .parse::<usize>()
        .ok()
        .filter(|samples| (1..=10_000).contains(samples))
    else {
        return false;
    };
    if recipe.settings.method != AnalysisMethod::Nca
        || expected_x == expected_y
        || !matches!(expected_ceiling, "ce_fdh" | "cr_fdh" | "both")
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || assessment_method_version != NCA_NOT_APPLICABLE_ASSESSMENT_VERSION
        || result.provenance.method_version != estimation.method_version
        || !estimation.converged
        || estimation.iterations != 0
        || !estimation.transforms.is_empty()
        || !estimation.construct_scores.is_empty()
        || !estimation.outer_estimates.is_empty()
        || !estimation.paths.is_empty()
        || !estimation.control_estimates.is_empty()
        || !estimation.effects.is_empty()
        || !estimation.r_squared.is_empty()
        || estimation.used_observations < 3
        || estimation.plsc.is_some()
        || estimation.endogeneity.is_some()
        || estimation.nonlinear_effects.is_some()
        || estimation.moderated_mediation.is_some()
        || estimation.cta_pls.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.predict.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.gsca.is_some()
    {
        return false;
    }
    let Some(nca) = estimation.nca.as_ref() else {
        return false;
    };
    if nca.method_version != estimation.method_version
        || nca.observations != estimation.used_observations
        || nca.warnings.is_empty()
        || nca
            .warnings
            .iter()
            .any(|warning| !estimation.warnings.contains(warning))
    {
        return false;
    }
    match estimation.method_version.as_str() {
        NCA_METHOD_VERSION => {
            recipe.settings.weighting_scheme == WeightingScheme::Path
                && recipe.settings.preprocessing == Preprocessing::Unstandardized
                && nca_analysis_matches_v2_contract(
                    nca,
                    expected_x,
                    expected_y,
                    expected_ceiling,
                    expected_permutations,
                )
        }
        NCA_METHOD_VERSION_V1 => validate_legacy_nca_v1_contract(
            nca,
            expected_x,
            expected_y,
            expected_ceiling,
            expected_permutations,
        ),
        _ => false,
    }
}

fn validate_legacy_nca_v1_contract(
    nca: &NcaAnalysis,
    expected_x: &str,
    expected_y: &str,
    expected_ceiling: &str,
    expected_permutations: usize,
) -> bool {
    let expected_ceilings = match expected_ceiling {
        "ce_fdh" => vec!["ce_fdh"],
        "cr_fdh" => vec!["cr_fdh"],
        "both" => vec!["ce_fdh", "cr_fdh"],
        _ => return false,
    };
    if nca.method_version != NCA_METHOD_VERSION_V1
        || nca.x != expected_x
        || nca.y != expected_y
        || nca.ceiling != expected_ceiling
        || nca.permutation_samples != expected_permutations
        || nca.usable_permutations != expected_permutations
        || nca.observations < 3
        || nca.ceilings.len() != expected_ceilings.len()
        || nca.bottlenecks.len() != 9
    {
        return false;
    }
    for (row, expected) in nca.ceilings.iter().zip(expected_ceilings) {
        let Some(p_value) = row.permutation_p_value else {
            return false;
        };
        let lattice = p_value * (expected_permutations as f64 + 1.0);
        if row.ceiling != expected
            || !row.effect_size.is_finite()
            || !(0.0..=1.0).contains(&row.effect_size)
            || !p_value.is_finite()
            || p_value < 1.0 / (expected_permutations as f64 + 1.0)
            || p_value > 1.0
            || !close_enough(lattice, lattice.round())
        {
            return false;
        }
    }
    nca.bottlenecks.iter().enumerate().all(|(index, row)| {
        let expected_outcome = ((index + 1) * 10) as f64;
        row.outcome_percent.is_finite()
            && close_enough(row.outcome_percent, expected_outcome)
            && row
                .required_x_percent
                .is_some_and(|required| required.is_finite() && (0.0..=100.0).contains(&required))
    })
}

fn validate_prediction_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    predict: &PlsPredictAnalysis,
    recipe: Option<&AnalysisRecipe>,
) -> bool {
    let Some(prediction_observations) = predict
        .training_observations
        .checked_add(predict.test_observations)
    else {
        return false;
    };
    if predict.method_version != estimation.method_version
        || predict.split != "deterministic_complete_case_modulo_4_test_rows"
        || predict.training_observations == 0
        || predict.test_observations == 0
        || prediction_observations != estimation.used_observations
        || predict.test_observations != prediction_observations / 4
        || !result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == predict.method_version)
    {
        return false;
    }
    if predict.method_version == PLS_PREDICT_METHOD_VERSION_V1 {
        return predict.primary_analysis.is_empty()
            && predict.indicator_targets.is_empty()
            && !predict.targets.is_empty()
            && predict.repeated_kfold.as_ref().is_none_or(|repeated| {
                repeated.method_version == "plspredict_repeated_kfold_v1"
                    && repeated.folds == 5
                    && repeated.repeats == 3
                    && repeated.assignment
                        == "deterministic_complete_case_index_multiplier_modulo_5"
                    && repeated.seed == 0
                    && repeated.assignment_digest.is_empty()
                    && repeated.total_test_observations == prediction_observations * 3
                    && repeated.indicator_targets.is_empty()
                    && repeated.cvpat_benchmark_assessments.is_empty()
                    && repeated.paired_loss_diagnostics.is_empty()
            });
    }
    let Some(recipe) = recipe else {
        return false;
    };
    if predict.method_version != PLS_PREDICT_METHOD_VERSION
        || prediction_observations < 20
        || predict.primary_analysis != PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION
        || recipe.settings.method != AnalysisMethod::Predict
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
        || estimation.plsc.is_some()
        || estimation.wpls.is_some()
        || estimation.cca.is_some()
        || estimation.segmentation.is_some()
        || estimation.mga.is_some()
        || estimation.micom.is_some()
        || estimation.mga_permutation.is_some()
        || estimation.fimix.is_some()
        || estimation.ipma.is_some()
        || estimation.cbsem.is_some()
        || estimation.pca.is_some()
        || estimation.regression.is_some()
        || estimation.nca.is_some()
        || estimation.gsca.is_some()
        || predict.warnings.is_empty()
    {
        return false;
    }
    let expected_constructs = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| {
            recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        })
        .collect::<Vec<_>>();
    if expected_constructs.is_empty()
        || expected_constructs
            .iter()
            .any(|construct| construct.mode != MeasurementMode::Reflective)
        || predict.targets.len() != expected_constructs.len()
        || predict.indicator_targets.len()
            != expected_constructs
                .iter()
                .map(|construct| construct.indicators.len())
                .sum::<usize>()
    {
        return false;
    }
    for (target, construct) in predict.targets.iter().zip(&expected_constructs) {
        let expected_predictors = recipe
            .model
            .paths
            .iter()
            .filter(|path| path.target == construct.id)
            .count();
        if target.construct != construct.id
            || target.predictor_count != expected_predictors
            || !valid_nonnegative(target.rmse_pls)
            || !valid_nonnegative(target.mae_pls)
            || !valid_nonnegative(target.rmse_benchmark)
            || !valid_nonnegative(target.mae_benchmark)
            || target
                .q_squared_predict
                .is_some_and(|value| !value.is_finite())
            || target
                .rmse_lm
                .is_some_and(|value| !valid_nonnegative(value))
            || target.mae_lm.is_some_and(|value| !valid_nonnegative(value))
            || target.rmse_lm.is_some() != target.mae_lm.is_some()
            || target
                .q_squared_predict_lm
                .is_some_and(|value| !value.is_finite())
        {
            return false;
        }
    }
    if !validate_prediction_indicator_targets(
        &predict.indicator_targets,
        recipe,
        predict.test_observations,
    ) {
        return false;
    }
    let Some(repeated) = predict.repeated_kfold.as_ref() else {
        return false;
    };
    let Some(expected_total_test) = prediction_observations.checked_mul(10) else {
        return false;
    };
    if repeated.method_version != PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION
        || repeated.folds != 10
        || repeated.repeats != 10
        || repeated.assignment != "seeded_sha256_source_row_order_round_robin_10_v1"
        || repeated.seed != recipe.settings.seed
        || !valid_sha256_token(&repeated.assignment_digest)
        || repeated.total_test_observations != expected_total_test
        || repeated.targets.len() != expected_constructs.len()
        || !repeated.cvpat.is_empty()
        || !repeated.paired_loss_diagnostics.is_empty()
        || repeated.warnings.is_empty()
        || !validate_prediction_indicator_targets(
            &repeated.indicator_targets,
            recipe,
            expected_total_test,
        )
    {
        return false;
    }
    for (target, construct) in repeated.targets.iter().zip(&expected_constructs) {
        if target.construct != construct.id
            || target.predictor_count
                != recipe
                    .model
                    .paths
                    .iter()
                    .filter(|path| path.target == construct.id)
                    .count()
            || !valid_nonnegative(target.rmse_pls)
            || !valid_nonnegative(target.mae_pls)
            || !valid_nonnegative(target.rmse_benchmark)
            || !valid_nonnegative(target.mae_benchmark)
        {
            return false;
        }
    }
    validate_prediction_cvpat(
        &repeated.cvpat_benchmark_assessments,
        &repeated.indicator_targets,
        prediction_observations,
        expected_total_test,
    )
}

fn validate_prediction_indicator_targets(
    rows: &[PlsPredictIndicatorTarget],
    recipe: &AnalysisRecipe,
    expected_observations: usize,
) -> bool {
    let expected = recipe
        .model
        .constructs
        .iter()
        .filter(|construct| {
            recipe
                .model
                .paths
                .iter()
                .any(|path| path.target == construct.id)
        })
        .flat_map(|construct| {
            let predictor_count = prediction_earliest_indicator_count(recipe, &construct.id);
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct.id.as_str(), indicator.as_str(), predictor_count))
        })
        .collect::<Vec<_>>();
    if expected.len() != rows.len() || expected.iter().any(|(_, _, count)| *count == 0) {
        return false;
    }
    rows.iter()
        .zip(expected)
        .all(|(row, (construct, indicator, predictor_count))| {
            row.construct == construct
                && row.indicator == indicator
                && row.predictor_scope == "earliest_antecedent_indicators"
                && row.predictor_count == predictor_count
                && validate_prediction_error_metrics(&row.pls, expected_observations)
                && validate_prediction_error_metrics(&row.indicator_average, expected_observations)
                && validate_prediction_benchmark_metrics(&row.linear_model, expected_observations)
                && match (
                    row.q_squared_predict,
                    row.indicator_average.squared_error_sum > f64::EPSILON,
                ) {
                    (Some(value), true) => {
                        value.is_finite()
                            && close_enough(
                                value,
                                1.0 - row.pls.squared_error_sum
                                    / row.indicator_average.squared_error_sum,
                            )
                    }
                    (None, false) => true,
                    _ => false,
                }
        })
}

fn prediction_earliest_indicator_count(recipe: &AnalysisRecipe, target: &str) -> usize {
    let incoming = recipe
        .model
        .constructs
        .iter()
        .map(|construct| {
            (
                construct.id.as_str(),
                recipe
                    .model
                    .paths
                    .iter()
                    .filter(|path| path.target == construct.id)
                    .map(|path| path.source.as_str())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut roots = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack = incoming.get(target).cloned().unwrap_or_default();
    while let Some(construct) = stack.pop() {
        if !visited.insert(construct) {
            continue;
        }
        let predecessors = incoming.get(construct).cloned().unwrap_or_default();
        if predecessors.is_empty() {
            roots.insert(construct);
        } else {
            stack.extend(predecessors);
        }
    }
    recipe
        .model
        .constructs
        .iter()
        .filter(|construct| roots.contains(construct.id.as_str()))
        .map(|construct| construct.indicators.len())
        .sum()
}

fn validate_prediction_error_metrics(
    metrics: &PlsPredictErrorMetrics,
    expected_observations: usize,
) -> bool {
    if metrics.observations != expected_observations
        || expected_observations == 0
        || !valid_nonnegative(metrics.squared_error_sum)
        || !valid_nonnegative(metrics.absolute_error_sum)
        || !valid_nonnegative(metrics.rmse)
        || !valid_nonnegative(metrics.mae)
        || !close_enough(
            metrics.rmse,
            (metrics.squared_error_sum / expected_observations as f64).sqrt(),
        )
        || !close_enough(
            metrics.mae,
            metrics.absolute_error_sum / expected_observations as f64,
        )
        || metrics.mape_observations > expected_observations
    {
        return false;
    }
    match (
        metrics.absolute_percentage_error_sum,
        metrics.mape_percent,
        metrics.mape_observations,
    ) {
        (None, None, 0) => true,
        (Some(sum), Some(percent), count) if count > 0 => {
            valid_nonnegative(sum)
                && valid_nonnegative(percent)
                && close_enough(percent, 100.0 * sum / count as f64)
        }
        _ => false,
    }
}

fn validate_prediction_benchmark_metrics(
    benchmark: &qpls_estimation::PlsPredictBenchmarkMetrics,
    expected_observations: usize,
) -> bool {
    match benchmark.status.as_str() {
        "available" => {
            benchmark.reason.is_none()
                && benchmark.metrics.as_ref().is_some_and(|metrics| {
                    validate_prediction_error_metrics(metrics, expected_observations)
                })
        }
        "unavailable" => {
            benchmark.metrics.is_none()
                && benchmark
                    .reason
                    .as_ref()
                    .is_some_and(|reason| !reason.trim().is_empty())
        }
        _ => false,
    }
}

fn validate_prediction_cvpat(
    rows: &[PlsPredictCvpatBenchmarkAssessment],
    indicators: &[PlsPredictIndicatorTarget],
    observations: usize,
    total_test_observations: usize,
) -> bool {
    if rows.len() != 2 || indicators.is_empty() {
        return false;
    }
    let indicator_count = indicators.len();
    let expected_pls = indicators
        .iter()
        .map(|row| row.pls.squared_error_sum)
        .sum::<f64>()
        / (total_test_observations * indicator_count) as f64;
    let expected_ia = indicators
        .iter()
        .map(|row| row.indicator_average.squared_error_sum)
        .sum::<f64>()
        / (total_test_observations * indicator_count) as f64;
    let all_lm_available = indicators
        .iter()
        .all(|row| row.linear_model.status == "available");
    let expected_lm = all_lm_available.then(|| {
        indicators
            .iter()
            .map(|row| row.linear_model.metrics.as_ref().unwrap().squared_error_sum)
            .sum::<f64>()
            / (total_test_observations * indicator_count) as f64
    });
    for benchmark in ["indicator_average", "linear_model"] {
        let matches = rows
            .iter()
            .filter(|row| row.benchmark == benchmark)
            .collect::<Vec<_>>();
        if matches.len() != 1
            || !validate_prediction_cvpat_row(
                matches[0],
                expected_pls,
                if benchmark == "indicator_average" {
                    Some(expected_ia)
                } else {
                    expected_lm
                },
                observations,
                indicator_count,
            )
        {
            return false;
        }
    }
    true
}

fn validate_prediction_cvpat_row(
    row: &PlsPredictCvpatBenchmarkAssessment,
    expected_pls: f64,
    expected_benchmark: Option<f64>,
    observations: usize,
    indicator_count: usize,
) -> bool {
    if row.method_version != CVPAT_INDICATOR_BENCHMARK_METHOD_VERSION
        || row.comparison_kind != "benchmark_assessment"
        || row.target_scope != "all_endogenous_indicators"
        || !matches!(row.benchmark.as_str(), "indicator_average" | "linear_model")
        || row.loss != "mean_squared_error_across_indicators_per_observation"
        || row.alternative != "pls_loss_less_than_benchmark"
        || !close_enough(row.confidence_level, 0.95)
        || row.observations != observations
        || row.indicator_count != indicator_count
        || !row
            .mean_loss_pls
            .is_some_and(|value| close_enough(value, expected_pls))
    {
        return false;
    }
    let Some(expected_benchmark) = expected_benchmark else {
        return row.status == "benchmark_unavailable"
            && row.mean_loss_benchmark.is_none()
            && row.mean_loss_difference.is_none()
            && row.loss_difference_sum_of_squares.is_none()
            && row.standard_error.is_none()
            && row.t_statistic.is_none()
            && row.p_value_one_sided.is_none()
            && row.confidence_interval_lower.is_none()
            && row.confidence_interval_upper.is_none()
            && row.preferred_model.is_none()
            && row
                .reason
                .as_ref()
                .is_some_and(|reason| !reason.trim().is_empty());
    };
    let Some(mean_benchmark) = row.mean_loss_benchmark else {
        return false;
    };
    let Some(mean_difference) = row.mean_loss_difference else {
        return false;
    };
    let Some(sum_squares) = row.loss_difference_sum_of_squares else {
        return false;
    };
    if !close_enough(mean_benchmark, expected_benchmark)
        || !close_enough(mean_difference, expected_pls - expected_benchmark)
        || !valid_nonnegative(sum_squares)
    {
        return false;
    }
    let variance = ((sum_squares - observations as f64 * mean_difference.powi(2))
        / (observations - 1) as f64)
        .max(0.0);
    let expected_standard_error = variance.sqrt() / (observations as f64).sqrt();
    if expected_standard_error <= f64::EPSILON {
        return row.status == "inferential_test_unavailable"
            && row.standard_error.is_none()
            && row.t_statistic.is_none()
            && row.p_value_one_sided.is_none()
            && row.confidence_interval_lower.is_none()
            && row.confidence_interval_upper.is_none()
            && row.preferred_model.is_none()
            && row
                .reason
                .as_ref()
                .is_some_and(|reason| !reason.trim().is_empty());
    }
    let Some(standard_error) = row.standard_error else {
        return false;
    };
    let Some(t_statistic) = row.t_statistic else {
        return false;
    };
    let Some(p_value) = row.p_value_one_sided else {
        return false;
    };
    let Some(lower) = row.confidence_interval_lower else {
        return false;
    };
    let Some(upper) = row.confidence_interval_upper else {
        return false;
    };
    let Ok(distribution) = StudentsT::new(0.0, 1.0, observations as f64 - 1.0) else {
        return false;
    };
    let expected_t = mean_difference / expected_standard_error;
    let expected_p = distribution.cdf(expected_t);
    let critical = distribution.inverse_cdf(0.975);
    let expected_lower = mean_difference - critical * expected_standard_error;
    let expected_upper = mean_difference + critical * expected_standard_error;
    row.status == "available"
        && row.reason.is_none()
        && close_enough(standard_error, expected_standard_error)
        && close_enough(t_statistic, expected_t)
        && close_enough(p_value, expected_p)
        && close_enough(lower, expected_lower)
        && close_enough(upper, expected_upper)
        && row.preferred_model.as_deref()
            == ((mean_difference < 0.0 && expected_p < 0.05).then_some("pls_sem"))
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_sha256_token(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn validate_mga_payload_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
) -> bool {
    let Some(recipe) = recipe else {
        return false;
    };
    let Some(group_column) = metadata_value(recipe, "mga_group_column", "mga.group_column") else {
        return false;
    };
    let Some(group_a) = metadata_value(recipe, "mga_group_a", "mga.group_a") else {
        return false;
    };
    let Some(group_b) = metadata_value(recipe, "mga_group_b", "mga.group_b") else {
        return false;
    };
    if group_a == group_b
        || recipe
            .model
            .constructs
            .iter()
            .flat_map(|construct| construct.indicators.iter())
            .any(|indicator| indicator == group_column)
        || estimation.plsc.is_some()
        || estimation.wpls.is_some()
        || estimation.predict.is_some()
        || result.provenance.settings.case_weight_column.is_some()
    {
        return false;
    }
    if estimation.method_version == PLS_MGA_METHOD_VERSION_V1 {
        return validate_legacy_mga_v1(result, estimation, recipe, group_column, group_a, group_b);
    }
    if estimation.method_version != PLS_MGA_METHOD_VERSION
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || !recipe.model.interactions.is_empty()
        || !recipe.model.higher_order_constructs.is_empty()
    {
        return false;
    }
    let Some(mga) = estimation.mga.as_ref() else {
        return false;
    };
    if mga.method_version != PLS_MGA_METHOD_VERSION
        || mga.method_version != estimation.method_version
        || mga.group_column != group_column
        || mga.groups.len() != 2
        || mga.groups[0].group != group_a
        || mga.groups[1].group != group_b
        || mga.groups.iter().any(|group| {
            group.observations < 10
                || group.paths.len() != recipe.model.paths.len()
                || group.paths.iter().any(|path| {
                    !path.coefficient.is_finite()
                        || !recipe.model.paths.iter().any(|expected| {
                            expected.source == path.source && expected.target == path.target
                        })
                })
                || group.r_squared.values().any(|value| !value.is_finite())
                || !validate_mga_group_measurement(group, recipe)
        })
        || mga.comparisons.len() != recipe.model.paths.len()
        || mga.measurement_comparisons.len()
            != recipe
                .model
                .constructs
                .iter()
                .map(|construct| construct.indicators.len() * 2)
                .sum::<usize>()
    {
        return false;
    }
    for path in &recipe.model.paths {
        let Some(comparison) = mga
            .comparisons
            .iter()
            .find(|row| row.source == path.source && row.target == path.target)
        else {
            return false;
        };
        let Some(path_a) = mga.groups[0]
            .paths
            .iter()
            .find(|row| row.source == path.source && row.target == path.target)
        else {
            return false;
        };
        let Some(path_b) = mga.groups[1]
            .paths
            .iter()
            .find(|row| row.source == path.source && row.target == path.target)
        else {
            return false;
        };
        if comparison.group_a != group_a
            || comparison.group_b != group_b
            || !close_enough(comparison.coefficient_a, path_a.coefficient)
            || !close_enough(comparison.coefficient_b, path_b.coefficient)
            || !close_enough(
                comparison.difference,
                comparison.coefficient_a - comparison.coefficient_b,
            )
            || comparison
                .p_value_two_sided
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return false;
        }
    }
    for construct in &recipe.model.constructs {
        for indicator in &construct.indicators {
            for parameter in ["outer_loading", "outer_weight"] {
                let matches = mga
                    .measurement_comparisons
                    .iter()
                    .filter(|row| {
                        row.parameter == parameter
                            && row.construct == construct.id
                            && row.indicator == *indicator
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return false;
                }
                let row = matches[0];
                let Some(estimate_a) = mga.groups[0].outer_estimates.iter().find(|estimate| {
                    estimate.construct == construct.id && estimate.indicator == *indicator
                }) else {
                    return false;
                };
                let Some(estimate_b) = mga.groups[1].outer_estimates.iter().find(|estimate| {
                    estimate.construct == construct.id && estimate.indicator == *indicator
                }) else {
                    return false;
                };
                let expected_a = if parameter == "outer_loading" {
                    estimate_a.loading
                } else {
                    estimate_a.weight
                };
                let expected_b = if parameter == "outer_loading" {
                    estimate_b.loading
                } else {
                    estimate_b.weight
                };
                if row.group_a != group_a
                    || row.group_b != group_b
                    || !close_enough(row.estimate_a, expected_a)
                    || !close_enough(row.estimate_b, expected_b)
                    || !close_enough(row.difference, expected_a - expected_b)
                {
                    return false;
                }
            }
        }
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
        || !recipe
            .metadata
            .get("micom_configural_confirmed")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return false;
    }
    match estimation.mga_permutation.as_ref() {
        Some(permutation) => {
            let Some(requested_samples) = recipe
                .metadata
                .get("group_permutation_samples")
                .and_then(|value| value.trim().parse::<usize>().ok())
            else {
                return false;
            };
            let attempted = permutation.attempted_permutations.unwrap_or_default();
            let failed = permutation.failed_permutations.unwrap_or_default();
            if !(5_000..=10_000).contains(&requested_samples)
                || permutation.method_version != PLS_MGA_PERMUTATION_METHOD_VERSION
                || permutation.group_column != group_column
                || permutation.permutation_samples != requested_samples
                || permutation.usable_permutations != requested_samples
                || attempted < permutation.usable_permutations
                || attempted.saturating_sub(permutation.usable_permutations) != failed
                || permutation.comparisons.len() != recipe.model.paths.len()
                || permutation.measurement_comparisons.len() != mga.measurement_comparisons.len()
                || !result
                    .provenance
                    .method_version
                    .split('+')
                    .any(|version| version == PLS_MGA_PERMUTATION_METHOD_VERSION)
            {
                return false;
            }
            for path in &recipe.model.paths {
                let Some(comparison) = permutation
                    .comparisons
                    .iter()
                    .find(|row| row.source == path.source && row.target == path.target)
                else {
                    return false;
                };
                let Some(original) = mga
                    .comparisons
                    .iter()
                    .find(|row| row.source == path.source && row.target == path.target)
                else {
                    return false;
                };
                if !close_enough(comparison.original_difference, original.difference)
                    || !comparison
                        .empirical_p_value_two_sided
                        .is_some_and(|value| value.is_finite() && (0.0..=1.0).contains(&value))
                    || !comparison
                        .percentile_rank
                        .is_some_and(|value| value.is_finite() && (0.0..=1.0).contains(&value))
                {
                    return false;
                }
            }
            for original in &mga.measurement_comparisons {
                let matches = permutation
                    .measurement_comparisons
                    .iter()
                    .filter(|row| {
                        row.parameter == original.parameter
                            && row.construct == original.construct
                            && row.indicator == original.indicator
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return false;
                }
                let row = matches[0];
                if !close_enough(row.original_difference, original.difference)
                    || !row
                        .empirical_p_value_two_sided
                        .is_some_and(valid_probability)
                    || !row.percentile_rank.is_some_and(valid_probability)
                {
                    return false;
                }
            }
        }
        None => return false,
    }
    let Some(micom) = estimation.micom.as_ref() else {
        return false;
    };
    validate_micom_v2(result, recipe, micom, group_column, group_a, group_b)
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn validate_mga_group_measurement(
    group: &qpls_estimation::PlsMgaGroupSummary,
    recipe: &AnalysisRecipe,
) -> bool {
    let indicators = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| {
            construct
                .indicators
                .iter()
                .map(move |indicator| (construct.id.as_str(), indicator.as_str()))
        })
        .collect::<Vec<_>>();
    group.outer_estimates.len() == indicators.len()
        && group.transforms.len() == indicators.len()
        && indicators.iter().all(|(construct, indicator)| {
            group
                .outer_estimates
                .iter()
                .filter(|estimate| {
                    estimate.construct == *construct && estimate.indicator == *indicator
                })
                .count()
                == 1
                && group.outer_estimates.iter().any(|estimate| {
                    estimate.construct == *construct
                        && estimate.indicator == *indicator
                        && estimate.weight.is_finite()
                        && estimate.loading.is_finite()
                })
                && group
                    .transforms
                    .iter()
                    .filter(|transform| transform.indicator == *indicator)
                    .count()
                    == 1
                && group.transforms.iter().any(|transform| {
                    transform.indicator == *indicator
                        && transform.mean.is_finite()
                        && transform.scale.is_finite()
                        && transform.scale > 0.0
                })
        })
}

fn validate_micom_v2(
    result: &AnalysisResult,
    recipe: &AnalysisRecipe,
    micom: &qpls_estimation::MicomAnalysis,
    group_column: &str,
    group_a: &str,
    group_b: &str,
) -> bool {
    let Some(confidence_level) = micom.confidence_level else {
        return false;
    };
    let attempted = micom.attempted_permutations.unwrap_or_default();
    let failed = micom.failed_permutations.unwrap_or_default();
    if micom.method_version != MICOM_METHOD_VERSION
        || micom.group_column != group_column
        || micom.permutation_samples < 5_000
        || micom.permutation_samples > 10_000
        || micom.usable_permutations != micom.permutation_samples
        || attempted < micom.usable_permutations
        || attempted.saturating_sub(micom.usable_permutations) != failed
        || !close_enough(confidence_level, recipe.settings.confidence_level)
        || micom.groups.len() != 2
        || micom.groups[0].group != group_a
        || micom.groups[1].group != group_b
        || micom.groups.iter().any(|group| group.observations < 10)
        || micom.constructs.len() != recipe.model.constructs.len()
        || !result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == MICOM_METHOD_VERSION)
    {
        return false;
    }
    let tolerance = 1e-10;
    recipe.model.constructs.iter().all(|construct| {
        let matches = micom
            .constructs
            .iter()
            .filter(|row| row.construct == construct.id)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return false;
        }
        let row = matches[0];
        let (
            Some(composition_lower),
            Some(mean_a),
            Some(mean_b),
            Some(mean_lower),
            Some(mean_upper),
            Some(variance_a),
            Some(variance_b),
            Some(variance_lower),
            Some(variance_upper),
            Some(equal_means),
            Some(equal_variances),
        ) = (
            row.compositional_correlation_lower,
            row.mean_a,
            row.mean_b,
            row.mean_difference_lower,
            row.mean_difference_upper,
            row.variance_a,
            row.variance_b,
            row.variance_difference_lower,
            row.variance_difference_upper,
            row.equal_means,
            row.equal_variances,
        )
        else {
            return false;
        };
        let compositional = row.compositional_correlation + tolerance >= composition_lower;
        let expected_equal_means = row.mean_difference + tolerance >= mean_lower
            && row.mean_difference - tolerance <= mean_upper;
        let expected_equal_variances = row.variance_difference + tolerance >= variance_lower
            && row.variance_difference - tolerance <= variance_upper;
        row.configural_invariance
            && row.compositional_correlation.is_finite()
            && (-1.0..=1.0).contains(&row.compositional_correlation)
            && composition_lower.is_finite()
            && (-1.0..=1.0).contains(&composition_lower)
            && row.compositional_p_value.is_some_and(valid_probability)
            && mean_a.is_finite()
            && mean_b.is_finite()
            && row.mean_difference.is_finite()
            && close_enough(row.mean_difference, mean_a - mean_b)
            && mean_lower.is_finite()
            && mean_upper.is_finite()
            && mean_lower <= mean_upper
            && row.mean_p_value.is_some_and(valid_probability)
            && variance_a.is_finite()
            && variance_a > 0.0
            && variance_b.is_finite()
            && variance_b > 0.0
            && row.variance_difference.is_finite()
            && close_enough(row.variance_difference, (variance_a / variance_b).ln())
            && variance_lower.is_finite()
            && variance_upper.is_finite()
            && variance_lower <= variance_upper
            && row.variance_p_value.is_some_and(valid_probability)
            && equal_means == expected_equal_means
            && equal_variances == expected_equal_variances
            && row.partial_invariance == compositional
            && row.full_invariance == (compositional && equal_means && equal_variances)
    })
}

fn validate_legacy_mga_v1(
    result: &AnalysisResult,
    estimation: &PlsResult,
    recipe: &AnalysisRecipe,
    group_column: &str,
    group_a: &str,
    group_b: &str,
) -> bool {
    let Some(mga) = estimation.mga.as_ref() else {
        return false;
    };
    if mga.method_version != PLS_MGA_METHOD_VERSION_V1
        || mga.groups.len() != 2
        || mga.groups[0].group != group_a
        || mga.groups[1].group != group_b
        || mga.group_column != group_column
        || mga.comparisons.len() != recipe.model.paths.len()
        || !mga.measurement_comparisons.is_empty()
        || mga.groups.iter().any(|group| {
            group.observations < 10
                || group.paths.len() != recipe.model.paths.len()
                || group.paths.iter().any(|path| !path.coefficient.is_finite())
        })
    {
        return false;
    }
    let Some(permutation) = estimation.mga_permutation.as_ref() else {
        return false;
    };
    if permutation.method_version != PLS_MGA_PERMUTATION_METHOD_VERSION_V1
        || permutation.group_column != group_column
        || permutation.permutation_samples < 99
        || permutation.permutation_samples > 10_000
        || permutation.usable_permutations == 0
        || permutation.usable_permutations > permutation.permutation_samples
        || !permutation.measurement_comparisons.is_empty()
        || permutation.comparisons.len() != recipe.model.paths.len()
        || !result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == PLS_MGA_PERMUTATION_METHOD_VERSION_V1)
    {
        return false;
    }
    estimation.micom.as_ref().is_none_or(|micom| {
        micom.method_version == MICOM_METHOD_VERSION_V1
            && micom.group_column == group_column
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == MICOM_METHOD_VERSION_V1)
    })
}

fn validate_effect_decomposition(
    result_id: Uuid,
    estimation: &PlsResult,
) -> Result<(), ProjectError> {
    let invalid = || {
        ProjectError::Invalid(format!(
            "result {result_id} has an effect decomposition inconsistent with its structural paths"
        ))
    };
    let mut constructs = std::collections::BTreeSet::<String>::new();
    constructs.extend(estimation.construct_scores.keys().cloned());
    for path in &estimation.paths {
        constructs.insert(path.source.clone());
        constructs.insert(path.target.clone());
    }
    for effect in &estimation.effects {
        constructs.insert(effect.source.clone());
        constructs.insert(effect.target.clone());
    }
    let constructs = constructs.into_iter().collect::<Vec<_>>();
    let index = constructs
        .iter()
        .enumerate()
        .map(|(index, construct)| (construct.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let count = constructs.len();
    let mut direct = vec![vec![0.0; count]; count];
    let mut path_ids = std::collections::HashSet::new();
    for path in &estimation.paths {
        if path.source.trim().is_empty()
            || path.target.trim().is_empty()
            || path.source == path.target
            || !path.coefficient.is_finite()
            || !path_ids.insert((path.source.as_str(), path.target.as_str()))
        {
            return Err(invalid());
        }
        direct[index[path.source.as_str()]][index[path.target.as_str()]] = path.coefficient;
    }

    let multiply = |left: &[Vec<f64>], right: &[Vec<f64>]| {
        let mut product = vec![vec![0.0; count]; count];
        for row in 0..count {
            for column in 0..count {
                for inner in 0..count {
                    product[row][column] += left[row][inner] * right[inner][column];
                }
            }
        }
        product
    };
    let mut total = direct.clone();
    let mut power = direct.clone();
    for _ in 2..count {
        power = multiply(&power, &direct);
        for row in 0..count {
            for column in 0..count {
                total[row][column] += power[row][column];
            }
        }
    }
    let mut expected = BTreeMap::new();
    for source in 0..count {
        for target in 0..count {
            if source != target && total[source][target].abs() > 1e-15 {
                expected.insert(
                    (constructs[source].as_str(), constructs[target].as_str()),
                    (
                        direct[source][target],
                        total[source][target] - direct[source][target],
                        total[source][target],
                    ),
                );
            }
        }
    }
    let mut actual = BTreeMap::new();
    for effect in &estimation.effects {
        if effect.source.trim().is_empty()
            || effect.target.trim().is_empty()
            || effect.source == effect.target
            || !effect.direct.is_finite()
            || !effect.indirect.is_finite()
            || !effect.total.is_finite()
            || actual
                .insert(
                    (effect.source.as_str(), effect.target.as_str()),
                    (effect.direct, effect.indirect, effect.total),
                )
                .is_some()
        {
            return Err(invalid());
        }
    }
    if actual.len() != expected.len()
        || actual.iter().any(|(id, actual)| {
            let Some(expected) = expected.get(id) else {
                return true;
            };
            !approximately_equal(actual.0, expected.0, 1e-12)
                || !approximately_equal(actual.1, expected.1, 1e-12)
                || !approximately_equal(actual.2, expected.2, 1e-12)
                || !approximately_equal(actual.2, actual.0 + actual.1, 1e-12)
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_mediation_contract(
    result: &AnalysisResult,
    mediation_payload_present: bool,
    estimation: &PlsResult,
) -> Result<(), ProjectError> {
    const MEDIATION_TOLERANCE: f64 = 1e-12;

    let envelope_has_mediation_version = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLS_MEDIATION_METHOD_VERSION);
    if !mediation_payload_present && !envelope_has_mediation_version {
        return Ok(());
    }
    if !mediation_payload_present || !envelope_has_mediation_version {
        return Err(ProjectError::Invalid(format!(
            "result {} has a PLS mediation payload/provenance version mismatch",
            result.id
        )));
    }
    if estimation.mediation.method_version != PLS_MEDIATION_METHOD_VERSION {
        return Err(ProjectError::Invalid(format!(
            "result {} uses an unsupported PLS mediation payload version",
            result.id
        )));
    }
    if estimation.mediation.tolerance.to_bits() != MEDIATION_TOLERANCE.to_bits() {
        return Err(ProjectError::Invalid(format!(
            "result {} has an unsupported PLS mediation tolerance",
            result.id
        )));
    }
    validate_effect_decomposition(result.id, estimation)?;
    let expected =
        analyze_mediation_effects_with_tolerance(&estimation.effects, MEDIATION_TOLERANCE);
    if !mediation_payload_matches(&estimation.mediation, &expected) {
        return Err(ProjectError::Invalid(format!(
            "result {} has a PLS mediation payload inconsistent with its effect decomposition",
            result.id
        )));
    }
    Ok(())
}

fn validate_higher_order_contract(
    result: &AnalysisResult,
    estimation: &PlsResult,
    assessment: &AssessmentResult,
    recipe: Option<&AnalysisRecipe>,
) -> Result<(), ProjectError> {
    const GENERATED_PREFIX: &str = "__qpls_hoc_";
    let invalid = || {
        ProjectError::Invalid(format!(
            "result {} uses an invalid or unsupported higher-order construct contract",
            result.id
        ))
    };
    let payload_has_generated_hoc = estimation
        .outer_estimates
        .iter()
        .any(|row| row.indicator.starts_with(GENERATED_PREFIX))
        || estimation
            .transforms
            .iter()
            .any(|row| row.indicator.starts_with(GENERATED_PREFIX));
    let Some(recipe) = recipe else {
        return if payload_has_generated_hoc {
            Err(invalid())
        } else {
            Ok(())
        };
    };
    if recipe.model.higher_order_constructs.is_empty() {
        return if payload_has_generated_hoc {
            Err(invalid())
        } else {
            Ok(())
        };
    }
    if recipe
        .model
        .higher_order_constructs
        .iter()
        .any(|higher_order| higher_order.method != HigherOrderMethod::TwoStage)
    {
        // Repeated-indicator and hybrid HOC archives retain their existing
        // project contract. The native slice below is intentionally narrower.
        return if payload_has_generated_hoc {
            Err(invalid())
        } else {
            Ok(())
        };
    }
    if recipe.settings.method != AnalysisMethod::PlsPm
        || recipe.settings.weighting_scheme != WeightingScheme::Path
        || recipe.settings.preprocessing != Preprocessing::Standardized
        || recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.settings.bootstrap_samples > 0
        || recipe.settings.studentized_inner_samples > 0
        || recipe.settings.permutation_samples > 0
        || (recipe.settings.confidence_level - 0.95).abs() > 1e-12
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || recipe.model.higher_order_constructs.len() != 1
        || recipe.model.paths.len() != 1
    {
        return Err(invalid());
    }

    let higher_order = &recipe.model.higher_order_constructs[0];
    if higher_order.method != HigherOrderMethod::TwoStage
        || higher_order.stage_one_recipe.is_some()
        || higher_order.components.len() < 2
    {
        return Err(invalid());
    }
    let constructs = recipe
        .model
        .constructs
        .iter()
        .map(|construct| (construct.id.as_str(), construct))
        .collect::<BTreeMap<_, _>>();
    if constructs.len() != recipe.model.constructs.len() {
        return Err(invalid());
    }
    let Some(hoc_construct) = constructs.get(higher_order.id.as_str()) else {
        return Err(invalid());
    };
    if hoc_construct.mode != MeasurementMode::Reflective || !hoc_construct.indicators.is_empty() {
        return Err(invalid());
    }
    let component_ids = higher_order
        .components
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if component_ids.len() != higher_order.components.len()
        || component_ids.contains(higher_order.id.as_str())
        || component_ids.iter().any(|component_id| {
            constructs.get(component_id).is_none_or(|component| {
                component.mode != MeasurementMode::Reflective || component.indicators.is_empty()
            })
        })
    {
        return Err(invalid());
    }
    let path = &recipe.model.paths[0];
    if path.source != higher_order.id
        || path.target == higher_order.id
        || component_ids.contains(path.target.as_str())
        || constructs.get(path.target.as_str()).is_none_or(|target| {
            target.mode != MeasurementMode::Reflective || target.indicators.is_empty()
        })
    {
        return Err(invalid());
    }

    let generated_indicators = higher_order
        .components
        .iter()
        .map(|component| format!("{GENERATED_PREFIX}{}_{}", higher_order.id, component))
        .collect::<BTreeSet<_>>();
    let mut expected_outer = BTreeSet::new();
    let mut expected_indicators = BTreeSet::new();
    for construct in &recipe.model.constructs {
        if construct.id == higher_order.id {
            for indicator in &generated_indicators {
                expected_outer.insert((construct.id.clone(), indicator.clone()));
                expected_indicators.insert(indicator.clone());
            }
        } else {
            for indicator in &construct.indicators {
                if indicator.trim().is_empty() || !expected_indicators.insert(indicator.clone()) {
                    return Err(invalid());
                }
                expected_outer.insert((construct.id.clone(), indicator.clone()));
            }
        }
    }
    let mut actual_outer = BTreeSet::new();
    if estimation.outer_estimates.iter().any(|row| {
        !row.loading.is_finite()
            || !row.weight.is_finite()
            || !actual_outer.insert((row.construct.clone(), row.indicator.clone()))
    }) || actual_outer != expected_outer
    {
        return Err(invalid());
    }
    let mut actual_transforms = BTreeSet::new();
    if estimation.transforms.iter().any(|row| {
        !row.mean.is_finite()
            || !row.scale.is_finite()
            || row.scale <= 0.0
            || !actual_transforms.insert(row.indicator.clone())
    }) || actual_transforms != expected_indicators
    {
        return Err(invalid());
    }
    let expected_construct_ids = constructs
        .keys()
        .map(|id| (*id).to_string())
        .collect::<BTreeSet<_>>();
    if estimation.used_observations < 3
        || estimation
            .construct_scores
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_construct_ids
        || estimation.construct_scores.values().any(|scores| {
            scores.len() != estimation.used_observations
                || scores.iter().any(|value| !value.is_finite())
        })
    {
        return Err(invalid());
    }
    let hoc_scores = &estimation.construct_scores[&higher_order.id];
    for component_id in &higher_order.components {
        let component_scores = &estimation.construct_scores[component_id];
        let component_mean = component_scores.iter().sum::<f64>() / component_scores.len() as f64;
        let hoc_mean = hoc_scores.iter().sum::<f64>() / hoc_scores.len() as f64;
        let covariance = component_scores
            .iter()
            .zip(hoc_scores)
            .map(|(component, hoc)| (component - component_mean) * (hoc - hoc_mean))
            .sum::<f64>();
        let component_variance = component_scores
            .iter()
            .map(|value| (value - component_mean).powi(2))
            .sum::<f64>();
        let hoc_variance = hoc_scores
            .iter()
            .map(|value| (value - hoc_mean).powi(2))
            .sum::<f64>();
        let denominator = (component_variance * hoc_variance).sqrt();
        let indicator = format!("{GENERATED_PREFIX}{}_{}", higher_order.id, component_id);
        let loading = estimation
            .outer_estimates
            .iter()
            .find(|row| row.construct == higher_order.id && row.indicator == indicator)
            .map(|row| row.loading);
        if !denominator.is_finite()
            || denominator <= f64::EPSILON
            || loading.is_none_or(|loading| !close_enough(loading, covariance / denominator))
        {
            return Err(invalid());
        }
    }
    let source_scores = &estimation.construct_scores[&path.source];
    let target_scores = &estimation.construct_scores[&path.target];
    let source_mean = source_scores.iter().sum::<f64>() / source_scores.len() as f64;
    let denominator = source_scores
        .iter()
        .map(|value| (value - source_mean).powi(2))
        .sum::<f64>();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return Err(invalid());
    }
    let expected_coefficient = source_scores
        .iter()
        .zip(target_scores)
        .map(|(source, target)| (source - source_mean) * target)
        .sum::<f64>()
        / denominator;
    let expected_fitted = source_scores
        .iter()
        .map(|source| source * expected_coefficient)
        .collect::<Vec<_>>();
    let residual = target_scores
        .iter()
        .zip(&expected_fitted)
        .map(|(actual, fitted)| (actual - fitted).powi(2))
        .sum::<f64>();
    let total = target_scores.iter().map(|value| value * value).sum::<f64>();
    let expected_r_squared = 1.0 - residual / total;
    if estimation.paths.len() != 1
        || estimation.paths[0].source != path.source
        || estimation.paths[0].target != path.target
        || !close_enough(estimation.paths[0].coefficient, expected_coefficient)
        || estimation.r_squared.len() != 1
        || estimation
            .r_squared
            .get(&path.target)
            .is_none_or(|value| !close_enough(*value, expected_r_squared))
        || !estimation
            .warnings
            .iter()
            .any(|warning| warning.contains("Two-stage higher-order constructs"))
    {
        return Err(invalid());
    }

    let quality_ids = assessment
        .construct_quality
        .iter()
        .map(|row| row.construct.as_str())
        .collect::<BTreeSet<_>>();
    let generated_cross_loadings = assessment
        .cross_loadings
        .iter()
        .filter(|row| row.indicator.starts_with(GENERATED_PREFIX))
        .collect::<Vec<_>>();
    if quality_ids.len() != assessment.construct_quality.len()
        || quality_ids != constructs.keys().copied().collect::<BTreeSet<_>>()
        || assessment
            .construct_quality
            .iter()
            .find(|row| row.construct == higher_order.id)
            .and_then(|row| row.rho_a_indicator_count)
            != Some(generated_indicators.len())
        || generated_cross_loadings.is_empty()
        || generated_cross_loadings.iter().any(|row| {
            !generated_indicators.contains(&row.indicator)
                || row.assigned_construct != higher_order.id
                || !row.loading.is_finite()
        })
        || assessment
            .fornell_larcker
            .constructs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != constructs.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(invalid());
    }
    Ok(())
}

fn mediation_payload_matches(actual: &MediationAnalysis, expected: &MediationAnalysis) -> bool {
    let optional_close = |left: Option<f64>, right: Option<f64>| match (left, right) {
        (Some(left), Some(right)) => close_enough(left, right),
        (None, None) => true,
        _ => false,
    };
    actual.method_version == expected.method_version
        && actual.tolerance.to_bits() == expected.tolerance.to_bits()
        && actual.warnings == expected.warnings
        && actual.estimates.len() == expected.estimates.len()
        && actual
            .estimates
            .iter()
            .zip(&expected.estimates)
            .all(|(left, right)| {
                left.source == right.source
                    && left.target == right.target
                    && close_enough(left.direct, right.direct)
                    && close_enough(left.indirect, right.indirect)
                    && close_enough(left.total, right.total)
                    && optional_close(left.variance_accounted_for, right.variance_accounted_for)
                    && left.classification == right.classification
                    && left.warning == right.warning
            })
}

fn validate_moderation_contract(
    result: &AnalysisResult,
    moderation_payload_present: bool,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
) -> Result<(), ProjectError> {
    let Some(recipe) = recipe else {
        return Ok(());
    };
    let envelope_moderation_version_count = result
        .provenance
        .method_version
        .split('+')
        .filter(|version| *version == PLS_TWO_STAGE_MODERATION_METHOD_VERSION)
        .count();
    let envelope_has_moderation_version = envelope_moderation_version_count == 1;

    if recipe.model.interactions.is_empty() {
        if envelope_moderation_version_count != 0 {
            return Err(ProjectError::Invalid(format!(
                "result {} declares two-stage moderation without an interaction recipe",
                result.id
            )));
        }
        if !moderation_payload_present {
            // Historical typed PLS archives predate the serialized moderation field.
            return Ok(());
        }
        if estimation.moderation != Default::default() {
            return Err(ProjectError::Invalid(format!(
                "result {} contains two-stage moderation output without an interaction recipe",
                result.id
            )));
        }
        return Ok(());
    }

    if !moderation_payload_present || !envelope_has_moderation_version {
        return Err(ProjectError::Invalid(format!(
            "result {} has a two-stage moderation recipe/payload/provenance mismatch",
            result.id
        )));
    }
    if result.provenance.method != AnalysisMethod::PlsPm
        || recipe.settings.method != AnalysisMethod::PlsPm
        || recipe.settings.weighting_scheme != qpls_core::WeightingScheme::Path
        || recipe.settings.preprocessing != qpls_core::Preprocessing::Standardized
        || recipe.settings.missing_data != qpls_core::MissingDataPolicy::ListwiseDeletion
        || recipe.settings.case_weight_column.is_some()
        || recipe.model.interactions.len() != 1
        || !recipe.model.higher_order_constructs.is_empty()
    {
        return Err(ProjectError::Invalid(format!(
            "result {} is outside the validated single-interaction PLS moderation scope",
            result.id
        )));
    }
    if estimation.moderation.method_version != PLS_TWO_STAGE_MODERATION_METHOD_VERSION {
        return Err(ProjectError::Invalid(format!(
            "result {} uses an unsupported two-stage moderation payload version",
            result.id
        )));
    }

    let interaction = &recipe.model.interactions[0];
    let construct_ids = recipe
        .model
        .constructs
        .iter()
        .map(|construct| (construct.id.as_str(), construct))
        .collect::<BTreeMap<_, _>>();
    let role_ids = [
        interaction.predictor.as_str(),
        interaction.moderator.as_str(),
        interaction.product_construct.as_str(),
        interaction.outcome.as_str(),
    ];
    let distinct_role_ids = role_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let product_construct = construct_ids.get(interaction.product_construct.as_str());
    let ordinary_constructs_are_measured = [
        interaction.predictor.as_str(),
        interaction.moderator.as_str(),
        interaction.outcome.as_str(),
    ]
    .into_iter()
    .all(|id| {
        construct_ids.get(id).is_some_and(|construct| {
            !construct.indicators.is_empty()
                && construct
                    .indicators
                    .iter()
                    .all(|indicator| !indicator.trim().is_empty())
        })
    });
    let path_count = |source: &str, target: &str| {
        recipe
            .model
            .paths
            .iter()
            .filter(|path| path.source == source && path.target == target)
            .count()
    };
    let product_has_unsupported_relationship = recipe.model.paths.iter().any(|path| {
        (path.source == interaction.product_construct
            || path.target == interaction.product_construct)
            && !(path.source == interaction.product_construct && path.target == interaction.outcome)
    }) || recipe.model.controls.iter().any(|control| {
        control.source == interaction.product_construct
            || control.target == interaction.product_construct
    });
    let exact_moderator_levels = [-1.0_f64, 0.0, 1.0];
    if interaction.id.trim().is_empty()
        || interaction.method != qpls_core::InteractionMethod::TwoStageProductScore
        || construct_ids.len() != recipe.model.constructs.len()
        || role_ids.iter().any(|id| id.trim().is_empty())
        || distinct_role_ids.len() != role_ids.len()
        || role_ids.iter().any(|id| !construct_ids.contains_key(id))
        || !ordinary_constructs_are_measured
        || product_construct.is_none_or(|construct| {
            construct.mode != qpls_core::MeasurementMode::Formative
                || !construct.indicators.is_empty()
        })
        || product_has_unsupported_relationship
        || path_count(&interaction.predictor, &interaction.outcome) != 1
        || path_count(&interaction.moderator, &interaction.outcome) != 1
        || path_count(&interaction.product_construct, &interaction.outcome) != 1
        || estimation.moderation.moderator_score_levels.len() != exact_moderator_levels.len()
        || estimation
            .moderation
            .moderator_score_levels
            .iter()
            .zip(exact_moderator_levels)
            .any(|(actual, expected)| actual.to_bits() != expected.to_bits())
    {
        return Err(ProjectError::Invalid(format!(
            "result {} has an invalid single-interaction moderation recipe",
            result.id
        )));
    }

    let expected = analyze_moderation(recipe, estimation);
    if estimation.moderation != expected {
        return Err(ProjectError::Invalid(format!(
            "result {} has a two-stage moderation payload inconsistent with its immutable recipe and structural paths",
            result.id
        )));
    }
    Ok(())
}

fn compatibility_notices(results: &[AnalysisResult]) -> Vec<ProjectCompatibilityNotice> {
    let mut notices = Vec::new();
    for result in results {
        let notice = if result.provenance.method == AnalysisMethod::Plsc
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLSC_METHOD_VERSION_V1)
        {
            Some(Diagnostic {
                code: "plsc.legacy_method_version".into(),
                level: DiagnosticLevel::Warning,
                message: "This result uses legacy plsc_v1 reliability correction. It remains readable for compatibility but is not the current Dijkstra-Henseler PLSc implementation; rerun the analysis to obtain plsc_v2.".into(),
            })
        } else if result.provenance.method == AnalysisMethod::Nca
            && result.provenance.method_version == NCA_METHOD_VERSION_V1
        {
            Some(Diagnostic {
                code: "nca.legacy_method_version".into(),
                level: DiagnosticLevel::Warning,
                message: "This result uses legacy nca_v1 ceiling geometry and remains readable only for archive compatibility. Rerun the analysis to obtain nca_v2 CE-FDH record-high peers, CR-FDH regression through those peers, and seeded independent permutations.".into(),
            })
        } else if result.provenance.method == AnalysisMethod::Predict
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_PREDICT_METHOD_VERSION_V1)
        {
            Some(Diagnostic {
                code: "predict.legacy_method_version".into(),
                level: DiagnosticLevel::Warning,
                message: "This result uses the legacy construct-score-only plspredict_holdout_v1 contract. It remains readable for archive compatibility but is not current indicator-level PLSpredict or CVPAT evidence; rerun the analysis to obtain plspredict_indicator_v2."
                    .into(),
            })
        } else if result.provenance.method == AnalysisMethod::Gsca
            && result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == GSCA_METHOD_VERSION_V1)
        {
            Some(Diagnostic {
                code: "gsca.legacy_preview".into(),
                level: DiagnosticLevel::Warning,
                message: "This archive contains the historical gsca_v1 PLS-derived preview with ad-hoc fit summaries and placeholder intervals. It remains readable for compatibility but is not GSCA ALS evidence; rerun with gsca_als_v2."
                    .into(),
            })
        } else if result.provenance.method == AnalysisMethod::Regression
            && result.provenance.method_version == REGRESSION_LOGISTIC_METHOD_VERSION_V1
        {
            Some(Diagnostic {
                code: "regression.logistic.legacy_method_version".into(),
                level: DiagnosticLevel::Warning,
                message: "This result uses the historical regression_logistic_v1 contract. It remains readable for archive compatibility but lacks the v2 complete-case outcome profile, convergence record, classification diagnostics, odds-ratio intervals, and expanded likelihood fit identities; rerun to obtain regression_logistic_v2."
                    .into(),
            })
        } else if result.provenance.method == AnalysisMethod::Regression
            && result.provenance.method_version == REGRESSION_PROCESS_METHOD_VERSION_V1
        {
            Some(Diagnostic {
                code: "regression.process.legacy_method_version".into(),
                level: DiagnosticLevel::Warning,
                message: "This archive contains historical regression_process_v1 bounded mediation/moderation output. It remains readable without reinterpretation but is not graph-defined PROCESS v2 evidence; rerun with regression_process_v2 for current equations, moderation diagnostics, and bootstrap inference."
                    .into(),
            })
        } else {
            None
        };
        if let Some(diagnostic) = notice
            && !result
                .diagnostics
                .iter()
                .any(|stored| stored.code == diagnostic.code)
        {
            notices.push(ProjectCompatibilityNotice {
                result_id: result.id,
                diagnostic,
            });
        }
    }
    notices
}

fn validate_result_contracts_internal(
    results: &[AnalysisResult],
    recipes: &[AnalysisRecipe],
    require_recipe_context: bool,
) -> Result<(), ProjectError> {
    validate_unique_analysis_ids(results, recipes)?;

    for recipe in recipes
        .iter()
        .filter(|recipe| recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION)
    {
        let errors = validate_recipe(recipe)
            .into_iter()
            .filter(|issue| issue.severity == Severity::Error)
            .map(|issue| format!("{}: {}", issue.code, issue.message))
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(ProjectError::Invalid(format!(
                "analysis recipe {} is invalid: {}",
                recipe.id,
                errors.join("; ")
            )));
        }
    }

    // Validation consumes one compatibility clone per stored recipe. This
    // lets existing method validators read schema-v3 executable projections
    // without ever rewriting the archived recipe or its annotation metadata.
    let effective_recipes = recipes
        .iter()
        .map(|recipe| match recipe.schema_version {
            1..=ANALYSIS_RECIPE_SCHEMA_VERSION => {
                recipe.with_effective_metadata().map_err(|error| {
                    ProjectError::Invalid(format!(
                        "analysis recipe {} cannot provide an effective validation view: {error}",
                        recipe.id
                    ))
                })
            }
            version => Err(ProjectError::Invalid(format!(
                "analysis recipe {} uses unsupported schema {version}",
                recipe.id
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let recipes = effective_recipes.as_slice();

    for result in results {
        let (estimation, assessment, bootstrap, permutation) = match &result.payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => (estimation, assessment, None, None),
            AnalysisPayload::PlsPmV2 {
                estimation,
                assessment,
                bootstrap,
            } => (estimation, assessment, Some(bootstrap), None),
            AnalysisPayload::PlsPmV3 {
                estimation,
                assessment,
                bootstrap,
                permutation,
            } => (
                estimation,
                assessment,
                bootstrap.as_ref(),
                permutation.as_ref(),
            ),
            AnalysisPayload::Legacy { .. } => continue,
        };
        if result.provenance.method != result.provenance.settings.method {
            return Err(ProjectError::Invalid(format!(
                "result {} method differs from its immutable provenance settings",
                result.id
            )));
        }
        if result.provenance.seed != result.provenance.settings.seed {
            return Err(ProjectError::Invalid(format!(
                "result {} seed differs from its immutable provenance settings",
                result.id
            )));
        }
        let expected_estimation_version = executable_pls_payload_method_version(
            result.provenance.method,
        )
        .ok_or_else(|| {
            ProjectError::Invalid(format!(
                "result {} has a PLS payload but method {} is not an executable PLS-family method",
                result.id, result.provenance.method
            ))
        })?;
        if !process_v2_pls_root_keys_are_declared(estimation) {
            return Err(ProjectError::Invalid(format!(
                "result {} has an undeclared PROCESS v2 estimation-root field",
                result.id
            )));
        }
        let mediation_payload_present = estimation.get("mediation").is_some();
        let moderation_payload_present = estimation.get("moderation").is_some();
        let estimation: PlsResult =
            serde_json::from_value(estimation.clone()).map_err(|error| {
                ProjectError::Invalid(format!(
                    "result {} has an invalid PLS estimation payload: {error}",
                    result.id
                ))
            })?;
        if matches!(
            result.provenance.method,
            AnalysisMethod::Nca
                | AnalysisMethod::Pca
                | AnalysisMethod::Regression
                | AnalysisMethod::Gsca
        ) {
            let recipe = recipes
                .iter()
                .find(|recipe| recipe.id == result.provenance.recipe_id)
                .ok_or_else(|| {
                    ProjectError::Invalid(format!(
                        "result {} references a missing analysis recipe",
                        result.id
                    ))
                })?;
            if recipe.settings != result.provenance.settings {
                return Err(ProjectError::Invalid(format!(
                    "result {} settings differ from its immutable analysis recipe",
                    result.id
                )));
            }
            if recipe.dataset_fingerprint != result.provenance.dataset_fingerprint {
                return Err(ProjectError::Invalid(format!(
                    "result {} dataset fingerprint differs from its immutable analysis recipe",
                    result.id
                )));
            }
            let assessment_method_version = assessment
                .get("method_version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let expected_assessment_warning = match result.provenance.method {
                AnalysisMethod::Nca => NCA_NOT_APPLICABLE_ASSESSMENT_WARNING,
                AnalysisMethod::Pca => PCA_NOT_APPLICABLE_ASSESSMENT_WARNING,
                AnalysisMethod::Regression => REGRESSION_NOT_APPLICABLE_ASSESSMENT_WARNING,
                AnalysisMethod::Gsca if estimation.method_version == GSCA_METHOD_VERSION_V1 => {
                    NCA_NOT_APPLICABLE_ASSESSMENT_WARNING
                }
                AnalysisMethod::Gsca => GSCA_NOT_APPLICABLE_ASSESSMENT_WARNING,
                _ => unreachable!(),
            };
            let assessment_warnings_valid = assessment
                .get("warnings")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|warnings| {
                    warnings.len() == 1
                        && (warnings[0].as_str() == Some(expected_assessment_warning)
                            || (result.provenance.method == AnalysisMethod::Regression
                                && matches!(
                                    result.provenance.method_version.as_str(),
                                    REGRESSION_OLS_METHOD_VERSION
                                        | REGRESSION_LOGISTIC_METHOD_VERSION_V1
                                        | REGRESSION_PROCESS_METHOD_VERSION_V1
                                )
                                && warnings[0].as_str()
                                    == Some(REGRESSION_NOT_APPLICABLE_ASSESSMENT_WARNING_V08)))
                });
            let payload_valid = match result.provenance.method {
                AnalysisMethod::Nca => validate_nca_payload_contract(
                    result,
                    &estimation,
                    Some(recipe),
                    assessment_method_version,
                ),
                AnalysisMethod::Pca => validate_pca_payload_contract(
                    result,
                    &estimation,
                    Some(recipe),
                    assessment_method_version,
                ),
                AnalysisMethod::Regression => validate_regression_payload_contract(
                    result,
                    &estimation,
                    Some(recipe),
                    assessment_method_version,
                ),
                AnalysisMethod::Gsca if estimation.method_version == GSCA_METHOD_VERSION_V1 => {
                    validate_legacy_gsca_v1_contract(
                        result,
                        &estimation,
                        Some(recipe),
                        assessment_method_version,
                    )
                }
                AnalysisMethod::Gsca => validate_gsca_payload_contract(
                    result,
                    &estimation,
                    Some(recipe),
                    assessment_method_version,
                ),
                _ => unreachable!(),
            };
            if assessment.as_object().is_none_or(|object| {
                object.len() != 2
                    || !object.contains_key("method_version")
                    || !object.contains_key("warnings")
            }) || !assessment_warnings_valid
                || bootstrap.is_some()
                || permutation.is_some()
                || !payload_valid
            {
                return Err(ProjectError::Invalid(format!(
                    "result {} uses an invalid or unsupported {} payload contract",
                    result.id, result.provenance.method
                )));
            }
            continue;
        }
        let assessment: AssessmentResult =
            serde_json::from_value(assessment.clone()).map_err(|error| {
                ProjectError::Invalid(format!(
                    "result {} has an invalid PLS assessment payload: {error}",
                    result.id
                ))
            })?;
        let recipe = if recipes.is_empty() && !require_recipe_context {
            None
        } else {
            let recipe = recipes
                .iter()
                .find(|recipe| recipe.id == result.provenance.recipe_id)
                .ok_or_else(|| {
                    ProjectError::Invalid(format!(
                        "result {} references a missing analysis recipe",
                        result.id
                    ))
                })?;
            if recipe.settings != result.provenance.settings {
                return Err(ProjectError::Invalid(format!(
                    "result {} settings differ from its immutable analysis recipe",
                    result.id
                )));
            }
            if recipe.dataset_fingerprint != result.provenance.dataset_fingerprint {
                return Err(ProjectError::Invalid(format!(
                    "result {} dataset fingerprint differs from its immutable analysis recipe",
                    result.id
                )));
            }
            Some(recipe)
        };
        let nca_not_applicable_assessment = result.provenance.method == AnalysisMethod::Nca
            && assessment.method_version == NCA_NOT_APPLICABLE_ASSESSMENT_VERSION;
        let supported_assessment = nca_not_applicable_assessment
            || assessment.method_version == ASSESSMENT_METHOD_VERSION
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V6
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V5
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V4
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V3
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V2
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V1;
        let envelope_has_assessment_version = nca_not_applicable_assessment
            || result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == assessment.method_version);
        let envelope_has_estimation_version = result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == estimation.method_version);
        let estimation_version_supported = match result.provenance.method {
            AnalysisMethod::Plsc => is_supported_plsc_method_version(&estimation.method_version),
            AnalysisMethod::Mga => matches!(
                estimation.method_version.as_str(),
                PLS_MGA_METHOD_VERSION | PLS_MGA_METHOD_VERSION_V1
            ),
            AnalysisMethod::Nca => matches!(
                estimation.method_version.as_str(),
                NCA_METHOD_VERSION | NCA_METHOD_VERSION_V1
            ),
            AnalysisMethod::Predict => matches!(
                estimation.method_version.as_str(),
                PLS_PREDICT_METHOD_VERSION | PLS_PREDICT_METHOD_VERSION_V1
            ),
            AnalysisMethod::Cbsem => matches!(
                estimation.method_version.as_str(),
                CFA_ML_METHOD_VERSION | CBSEM_ML_METHOD_VERSION
            ),
            _ => estimation.method_version == expected_estimation_version,
        };
        let method_payload_matches = match result.provenance.method {
            AnalysisMethod::PlsPm => {
                estimation.plsc.is_none()
                    && estimation.wpls.is_none()
                    && estimation.predict.is_none()
            }
            AnalysisMethod::Plsc => {
                estimation.plsc.as_ref().is_some_and(|plsc| {
                    is_supported_plsc_method_version(&plsc.method_version)
                        && plsc.method_version == estimation.method_version
                        && plsc.reliability_method_version == RHO_A_METHOD_VERSION
                }) && estimation.wpls.is_none()
                    && estimation.predict.is_none()
            }
            AnalysisMethod::Wpls => {
                estimation.wpls.as_ref().is_some_and(|wpls| {
                    wpls.method_version == WPLS_METHOD_VERSION
                        && Some(wpls.case_weight_column.as_str())
                            == result.provenance.settings.case_weight_column.as_deref()
                }) && estimation.plsc.is_none()
                    && estimation.predict.is_none()
            }
            AnalysisMethod::Cca => validate_cca_payload_contract(
                result,
                &estimation,
                recipe,
                &assessment.method_version,
            ),
            AnalysisMethod::CtaPls => validate_cta_pls_payload_contract(
                result,
                &estimation,
                recipe,
                &assessment.method_version,
            ),
            AnalysisMethod::Predict => {
                estimation.predict.as_ref().is_some_and(|predict| {
                    validate_prediction_payload_contract(result, &estimation, predict, recipe)
                }) && estimation.plsc.is_none()
                    && estimation.wpls.is_none()
                    && estimation.segmentation.is_none()
                    && estimation.fimix.is_none()
                    && result.provenance.settings.case_weight_column.is_none()
            }
            AnalysisMethod::Mga => validate_mga_payload_contract(result, &estimation, recipe),
            AnalysisMethod::Ipma => validate_ipma_payload_contract(
                result,
                &estimation,
                recipe,
                &assessment.method_version,
            ),
            AnalysisMethod::Cbsem => validate_cbsem_payload_contract(
                result,
                &estimation,
                recipe,
                &assessment.method_version,
            ),
            AnalysisMethod::Nca => validate_nca_payload_contract(
                result,
                &estimation,
                recipe,
                &assessment.method_version,
            ),
            _ => false,
        };
        if !estimation_version_supported
            || !method_payload_matches
            || !supported_assessment
            || !envelope_has_estimation_version
            || !envelope_has_assessment_version
        {
            return Err(ProjectError::Invalid(format!(
                "result {} uses unsupported PLS payload versions",
                result.id
            )));
        }
        if result.provenance.method != AnalysisMethod::Nca {
            validate_mediation_contract(result, mediation_payload_present, &estimation)?;
            validate_moderation_contract(result, moderation_payload_present, &estimation, recipe)?;
            validate_higher_order_contract(result, &estimation, &assessment, recipe)?;
        }
        if matches!(
            result.provenance.method,
            AnalysisMethod::Plsc
                | AnalysisMethod::Wpls
                | AnalysisMethod::Cca
                | AnalysisMethod::CtaPls
                | AnalysisMethod::Predict
                | AnalysisMethod::Mga
                | AnalysisMethod::Ipma
                | AnalysisMethod::Nca
        ) && (result.provenance.settings.bootstrap_samples > 0
            || result.provenance.settings.studentized_inner_samples > 0
            || result.provenance.settings.permutation_samples > 0
            || bootstrap.is_some()
            || permutation.is_some())
        {
            return Err(ProjectError::Invalid(format!(
                "result {} contains unsupported resampling for method {}",
                result.id, result.provenance.method
            )));
        }
        let supports_f_squared = matches!(
            assessment.method_version.as_str(),
            ASSESSMENT_METHOD_VERSION
                | ASSESSMENT_METHOD_VERSION_V6
                | ASSESSMENT_METHOD_VERSION_V5
                | ASSESSMENT_METHOD_VERSION_V4
                | ASSESSMENT_METHOD_VERSION_V3
        );
        let supports_fit = matches!(
            assessment.method_version.as_str(),
            ASSESSMENT_METHOD_VERSION
                | ASSESSMENT_METHOD_VERSION_V6
                | ASSESSMENT_METHOD_VERSION_V5
                | ASSESSMENT_METHOD_VERSION_V4
        );
        let supports_rho_a = matches!(
            assessment.method_version.as_str(),
            ASSESSMENT_METHOD_VERSION | ASSESSMENT_METHOD_VERSION_V6 | ASSESSMENT_METHOD_VERSION_V5
        );
        let supports_explicit_htmt = matches!(
            assessment.method_version.as_str(),
            ASSESSMENT_METHOD_VERSION | ASSESSMENT_METHOD_VERSION_V6
        );
        let supports_legacy_htmt = matches!(
            assessment.method_version.as_str(),
            ASSESSMENT_METHOD_VERSION_V2
                | ASSESSMENT_METHOD_VERSION_V3
                | ASSESSMENT_METHOD_VERSION_V4
                | ASSESSMENT_METHOD_VERSION_V5
        );
        if !supports_f_squared && !assessment.f_squared.is_empty() {
            return Err(ProjectError::Invalid(format!(
                "result {} contains Cohen f-squared values under a legacy assessment version",
                result.id
            )));
        }
        if !supports_fit && (assessment.model_fit.is_some() || assessment.blindfolding.is_some()) {
            return Err(ProjectError::Invalid(format!(
                "result {} contains v4 fit or blindfolding values under a legacy assessment version",
                result.id
            )));
        }
        let has_rho_a_fields = assessment.rho_a_method_version.is_some()
            || assessment.construct_quality.iter().any(|row| {
                row.rho_a.is_some()
                    || row.rho_a_status.is_some()
                    || row.rho_a_reason.is_some()
                    || !row.rho_a_warning_codes.is_empty()
                    || row.rho_a_indicator_count.is_some()
                    || row.score_variance_before_normalization.is_some()
                    || row.normalized_weight_norm_squared.is_some()
                    || row.off_diagonal_numerator.is_some()
                    || row.off_diagonal_denominator.is_some()
            });
        if supports_rho_a {
            if assessment.rho_a_method_version.as_deref() != Some(RHO_A_METHOD_VERSION) {
                return Err(ProjectError::Invalid(format!(
                    "result {} has an invalid rho_A method version",
                    result.id
                )));
            }
        } else if has_rho_a_fields {
            return Err(ProjectError::Invalid(format!(
                "result {} contains rho_A values under a pre-v5 assessment version",
                result.id
            )));
        }
        let has_explicit_htmt = assessment.htmt_plus_method_version.is_some()
            || assessment.htmt_plus.is_some()
            || assessment.htmt_original_method_version.is_some()
            || assessment.htmt_original.is_some();
        if supports_explicit_htmt {
            if assessment.htmt.is_some()
                || assessment.htmt_plus_method_version.as_deref() != Some(HTMT_PLUS_METHOD_VERSION)
                || assessment.htmt_original_method_version.as_deref()
                    != Some(HTMT_ORIGINAL_METHOD_VERSION)
                || assessment.htmt_plus.is_none()
                || assessment.htmt_original.is_none()
            {
                return Err(ProjectError::Invalid(format!(
                    "result {} has an invalid explicit HTMT payload",
                    result.id
                )));
            }
        } else if has_explicit_htmt {
            return Err(ProjectError::Invalid(format!(
                "result {} contains explicit HTMT artifacts under a pre-v6 assessment version",
                result.id
            )));
        }
        if !supports_legacy_htmt && !supports_explicit_htmt && assessment.htmt.is_some() {
            return Err(ProjectError::Invalid(format!(
                "result {} contains a legacy HTMT matrix under assessment v1",
                result.id
            )));
        }
        if assessment.method_version != ASSESSMENT_METHOD_VERSION_V1 {
            validate_assessment_current(result.id, &assessment, &estimation, recipe)?;
        }
        if bootstrap.is_none() && result.provenance.settings.bootstrap_samples != 0 {
            return Err(ProjectError::Invalid(format!(
                "result {} is missing requested bootstrap inference",
                result.id
            )));
        }
        if permutation.is_none() && result.provenance.settings.permutation_samples != 0 {
            return Err(ProjectError::Invalid(format!(
                "result {} is missing requested permutation inference",
                result.id
            )));
        }
        if let Some(bootstrap) = bootstrap {
            let bootstrap: PlsBootstrapResult =
                serde_json::from_value(bootstrap.clone()).map_err(|error| {
                    ProjectError::Invalid(format!(
                        "result {} has an invalid PLS bootstrap payload: {error}",
                        result.id
                    ))
                })?;
            let parameter_names = bootstrap
                .percentile
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.as_str())
                .collect::<std::collections::HashSet<_>>();
            let failed_indices = bootstrap
                .failed_replicates
                .iter()
                .map(|failure| failure.replicate_index)
                .collect::<std::collections::HashSet<_>>();
            let required_usable = ((bootstrap.plan.replicates as f64 * 0.9).ceil() as u32).max(2);
            let supported_version = bootstrap.method_version == RESAMPLING_METHOD_VERSION
                || bootstrap.method_version == RESAMPLING_METHOD_VERSION_V3
                || bootstrap.method_version == RESAMPLING_METHOD_VERSION_V2
                || bootstrap.method_version == RESAMPLING_METHOD_VERSION_V1;
            let envelope_has_bootstrap_version = result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == bootstrap.method_version);
            let valid_parameter_identities = bootstrap.percentile.parameters.iter().all(|row| {
                if bootstrap.method_version != RESAMPLING_METHOD_VERSION_V1 {
                    serde_json::from_str::<(String, Vec<String>)>(&row.parameter)
                        .is_ok_and(|(kind, parts)| !kind.trim().is_empty() && !parts.is_empty())
                } else {
                    !row.parameter.trim().is_empty()
                }
            });
            let valid_bca = if bootstrap.method_version == RESAMPLING_METHOD_VERSION
                || bootstrap.method_version == RESAMPLING_METHOD_VERSION_V3
            {
                bootstrap.bca.as_ref().is_some_and(|bca| {
                    let bca_names = bca
                        .parameters
                        .iter()
                        .map(|parameter| parameter.parameter.as_str())
                        .collect::<std::collections::HashSet<_>>();
                    bca.confidence_level == result.provenance.settings.confidence_level
                        && bca.jackknife_case_count == estimation.used_observations
                        && bca_names == parameter_names
                        && bca_names.len() == bca.parameters.len()
                        && bca.parameters.iter().all(|parameter| {
                            let available = match (
                                parameter.bias_correction,
                                parameter.acceleration,
                                parameter.lower,
                                parameter.upper,
                                &parameter.unavailable_reason,
                            ) {
                                (Some(z0), Some(acceleration), Some(lower), Some(upper), None) => {
                                    z0.is_finite()
                                        && acceleration.is_finite()
                                        && lower.is_finite()
                                        && upper.is_finite()
                                        && lower <= upper
                                }
                                _ => false,
                            };
                            let unavailable = parameter.bias_correction.is_none()
                                && parameter.acceleration.is_none()
                                && parameter.lower.is_none()
                                && parameter.upper.is_none()
                                && parameter
                                    .unavailable_reason
                                    .as_ref()
                                    .is_some_and(|reason| !reason.trim().is_empty());
                            available || unavailable
                        })
                })
            } else {
                bootstrap.bca.is_none()
            };
            let valid_studentized = if bootstrap.method_version == RESAMPLING_METHOD_VERSION {
                if result.provenance.settings.studentized_inner_samples == 0 {
                    bootstrap.studentized.is_none()
                } else {
                    bootstrap.studentized.as_ref().is_some_and(|studentized| {
                        let studentized_names = studentized
                            .parameters
                            .iter()
                            .map(|parameter| parameter.parameter.as_str())
                            .collect::<std::collections::HashSet<_>>();
                        let required_primary =
                            ((bootstrap.plan.replicates as f64 * 0.9).ceil() as u32).max(2);
                        studentized.confidence_level == result.provenance.settings.confidence_level
                            && studentized.method_version == STUDENTIZED_METHOD_VERSION
                            && studentized.inner_replicates
                                == result.provenance.settings.studentized_inner_samples
                            && (99..=999).contains(&studentized.inner_replicates)
                            && studentized.inner_replicates % 2 == 1
                            && studentized.minimum_usable_fraction == 0.9
                            && studentized.stream_domain == "pls_pm_studentized_inner_v1"
                            && bootstrap.plan.replicates >= 999
                            && match &studentized.failure {
                                Some(failure) => {
                                    studentized.parameters.is_empty()
                                        && failure.reason_code == "nested_infrastructure_failure"
                                        && failure.first_primary_replicate
                                            < bootstrap.plan.replicates
                                        && failure.failed_primary_replicates > 0
                                        && failure.failed_primary_replicates
                                            <= bootstrap.usable_replicates
                                        && !failure.message.trim().is_empty()
                                }
                                None => {
                                    studentized_names == parameter_names
                                        && studentized_names.len() == studentized.parameters.len()
                                        && studentized.parameters.iter().all(|parameter| {
                                            let percentile_parameter =
                                                bootstrap.percentile.parameters.iter().find(
                                                    |candidate| {
                                                        candidate.parameter == parameter.parameter
                                                    },
                                                );
                                            let identity_matches = percentile_parameter
                                                .is_some_and(|source| {
                                                    approximately_equal(
                                                        parameter.original,
                                                        source.original,
                                                        1e-12,
                                                    ) && approximately_equal(
                                                        parameter.outer_standard_error,
                                                        source.standard_error,
                                                        1e-12,
                                                    )
                                                });
                                            let scale_is_valid = parameter.outer_scale.is_finite()
                                                && parameter.outer_scale
                                                    >= parameter.original.abs().max(1.0);
                                            let zero_threshold =
                                                64.0 * f64::EPSILON * parameter.outer_scale;
                                            let available = match (
                                                parameter.lower_pivot,
                                                parameter.upper_pivot,
                                                parameter.lower,
                                                parameter.upper,
                                                &parameter.unavailable_reason,
                                            ) {
                                                (
                                                    Some(lower_pivot),
                                                    Some(upper_pivot),
                                                    Some(lower),
                                                    Some(upper),
                                                    None,
                                                ) => {
                                                    let expected_lower = parameter.original
                                                        - upper_pivot
                                                            * parameter.outer_standard_error;
                                                    let expected_upper = parameter.original
                                                        - lower_pivot
                                                            * parameter.outer_standard_error;
                                                    lower_pivot.is_finite()
                                                        && upper_pivot.is_finite()
                                                        && lower.is_finite()
                                                        && upper.is_finite()
                                                        && lower <= upper
                                                        && lower_pivot <= upper_pivot
                                                        && approximately_equal(
                                                            lower,
                                                            expected_lower,
                                                            1e-10,
                                                        )
                                                        && approximately_equal(
                                                            upper,
                                                            expected_upper,
                                                            1e-10,
                                                        )
                                                        && parameter.usable_primary_replicates
                                                            >= required_primary
                                                        && parameter.usable_primary_replicates
                                                            <= bootstrap.usable_replicates
                                                        && parameter.outer_standard_error
                                                            > zero_threshold
                                                }
                                                _ => false,
                                            };
                                            let reason_matches = parameter
                                                .unavailable_reason
                                                .as_deref()
                                                .is_some_and(|reason| match reason {
                                                    "insufficient_pivots" => {
                                                        parameter.usable_primary_replicates
                                                            < required_primary
                                                    }
                                                    "zero_outer_standard_error" => {
                                                        parameter.usable_primary_replicates
                                                            >= required_primary
                                                            && parameter.outer_standard_error
                                                                <= zero_threshold
                                                    }
                                                    "invalid_bounds" => {
                                                        parameter.usable_primary_replicates
                                                            >= required_primary
                                                            && parameter.outer_standard_error
                                                                > zero_threshold
                                                    }
                                                    _ => false,
                                                });
                                            let unavailable = parameter.lower_pivot.is_none()
                                                && parameter.upper_pivot.is_none()
                                                && parameter.lower.is_none()
                                                && parameter.upper.is_none()
                                                && parameter.usable_primary_replicates
                                                    <= bootstrap.usable_replicates
                                                && reason_matches;
                                            identity_matches
                                                && scale_is_valid
                                                && parameter.original.is_finite()
                                                && parameter.outer_standard_error.is_finite()
                                                && parameter.outer_standard_error >= 0.0
                                                && (available || unavailable)
                                        })
                                }
                            }
                    })
                }
            } else {
                bootstrap.studentized.is_none()
                    && result.provenance.settings.studentized_inner_samples == 0
            };
            let valid_moderation_binding = match recipe
                .and_then(|recipe| recipe.model.interactions.first())
            {
                None => true,
                Some(interaction) => {
                    let parameter_identity = serde_json::to_string(&(
                        "path",
                        [
                            interaction.product_construct.as_str(),
                            interaction.outcome.as_str(),
                        ],
                    ))
                    .expect("moderation bootstrap parameter identity must serialize");
                    let stored_effect = estimation.moderation.estimates.iter().find(|estimate| {
                        estimate.interaction == interaction.id
                            && estimate.product_construct == interaction.product_construct
                            && estimate.outcome == interaction.outcome
                    });
                    let parameter = bootstrap
                        .percentile
                        .parameters
                        .iter()
                        .find(|parameter| parameter.parameter == parameter_identity);
                    match (stored_effect, parameter) {
                        (Some(stored_effect), Some(parameter)) => {
                            parameter.original.to_bits()
                                == stored_effect.interaction_effect.to_bits()
                        }
                        _ => false,
                    }
                }
            };
            if !supported_version
                || !envelope_has_bootstrap_version
                || bootstrap.plan.replicates == 0
                || bootstrap.plan.replicates != result.provenance.settings.bootstrap_samples
                || bootstrap.plan.master_seed != result.provenance.settings.seed
                || bootstrap.plan.operation != "pls_pm_bootstrap_v1"
                || bootstrap.usable_replicates as usize + bootstrap.failed_replicates.len()
                    != bootstrap.plan.replicates as usize
                || bootstrap.usable_replicates < required_usable
                || failed_indices.len() != bootstrap.failed_replicates.len()
                || failed_indices
                    .iter()
                    .any(|index| *index >= bootstrap.plan.replicates)
                || bootstrap.percentile.confidence_level
                    != result.provenance.settings.confidence_level
                || parameter_names.len() != bootstrap.percentile.parameters.len()
                || bootstrap.percentile.parameters.is_empty()
                || !valid_parameter_identities
                || !valid_bca
                || !valid_studentized
                || !valid_moderation_binding
                || bootstrap.percentile.parameters.iter().any(|parameter| {
                    let expected =
                        normal_reference_test(parameter.original, parameter.standard_error);
                    let valid_normal_test =
                        if bootstrap.method_version == RESAMPLING_METHOD_VERSION_V1 {
                            parameter.t_statistic.is_none() && parameter.p_value_two_sided.is_none()
                        } else {
                            match (parameter.t_statistic, parameter.p_value_two_sided, expected) {
                                (
                                    Some(statistic),
                                    Some(probability),
                                    (Some(expected_t), Some(expected_p)),
                                ) => {
                                    statistic.is_finite()
                                        && probability.is_finite()
                                        && (0.0..=1.0).contains(&probability)
                                        && approximately_equal(statistic, expected_t, 1e-12)
                                        && approximately_equal(probability, expected_p, 1e-12)
                                }
                                (None, None, (None, None)) => true,
                                _ => false,
                            }
                        };
                    parameter.parameter.trim().is_empty()
                        || parameter.usable_replicates != bootstrap.usable_replicates
                        || !parameter.original.is_finite()
                        || !parameter.bootstrap_mean.is_finite()
                        || !parameter.bias.is_finite()
                        || !parameter.standard_error.is_finite()
                        || parameter.standard_error < 0.0
                        || !parameter.lower.is_finite()
                        || !parameter.upper.is_finite()
                        || parameter.lower > parameter.upper
                        || !valid_normal_test
                })
            {
                return Err(ProjectError::Invalid(format!(
                    "result {} bootstrap provenance is inconsistent",
                    result.id
                )));
            }
        }
        if let Some(permutation) = permutation {
            let permutation: PlsPermutationResult = serde_json::from_value(permutation.clone())
                .map_err(|error| {
                    ProjectError::Invalid(format!(
                        "result {} has an invalid PLS permutation payload: {error}",
                        result.id
                    ))
                })?;
            let canonical_estimation_order = recipe.is_none_or(|recipe| {
                let canonical_paths = recipe
                    .model
                    .constructs
                    .iter()
                    .flat_map(|construct| {
                        recipe
                            .model
                            .paths
                            .iter()
                            .filter(move |path| path.target == construct.id)
                    })
                    .collect::<Vec<_>>();
                canonical_paths.len() == estimation.paths.len()
                    && canonical_paths.iter().zip(&estimation.paths).all(
                        |(recipe_path, estimated_path)| {
                            recipe_path.source == estimated_path.source
                                && recipe_path.target == estimated_path.target
                        },
                    )
            });
            let expected = estimation
                .paths
                .iter()
                .map(|path| {
                    (
                        serde_json::to_string(&(
                            "path",
                            [path.source.as_str(), path.target.as_str()],
                        ))
                        .expect("path identity must serialize"),
                        path.coefficient,
                    )
                })
                .collect::<Vec<_>>();
            let parameter_manifest_matches = permutation.parameters.len() == expected.len()
                && permutation.parameters.iter().zip(&expected).all(
                    |(parameter, (expected_identity, expected_original))| {
                        let expected_probability = (parameter.exceedances as f64 + 1.0)
                            / (permutation.plan.permutations as f64 + 1.0);
                        parameter.parameter.as_str() == expected_identity.as_str()
                            && parameter.original.is_finite()
                            && parameter.original.to_bits() == expected_original.to_bits()
                            && parameter.permutations == permutation.plan.permutations
                            && parameter.exceedances <= permutation.plan.permutations
                            && parameter.p_value_two_sided.is_finite()
                            && (0.0..=1.0).contains(&parameter.p_value_two_sided)
                            && parameter.p_value_two_sided.to_bits()
                                == expected_probability.to_bits()
                    },
                );
            let envelope_has_version = result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == permutation.method_version);
            if permutation.method_version != PERMUTATION_METHOD_VERSION
                || !envelope_has_version
                || permutation.plan.permutations != result.provenance.settings.permutation_samples
                || permutation.plan.master_seed != result.provenance.settings.seed
                || permutation.plan.operation != "pls_pm_freedman_lane_v1"
                || !(99..=10_000).contains(&permutation.plan.permutations)
                || expected.is_empty()
                || !canonical_estimation_order
                || !parameter_manifest_matches
            {
                return Err(ProjectError::Invalid(format!(
                    "result {} permutation provenance is inconsistent",
                    result.id
                )));
            }
        }
    }
    Ok(())
}

fn validate_htmt_assessment(
    artifact: &HtmtAssessment,
    expected_absolute: bool,
    expected_constructs: &[String],
    recipe: Option<&AnalysisRecipe>,
) -> Result<(), ProjectError> {
    let invalid = || ProjectError::Invalid("inconsistent typed HTMT assessment payload".into());
    if artifact.constructs != expected_constructs
        || artifact.absolute_correlations != expected_absolute
        || artifact.correlation_type != "pearson"
        || artifact.cells.len() != expected_constructs.len()
        || artifact
            .cells
            .iter()
            .any(|row| row.len() != expected_constructs.len())
    {
        return Err(invalid());
    }
    let recognized_reasons = [
        "htmt.formative_not_applicable",
        "htmt.single_indicator_not_applicable",
        "htmt.zero_monotrait_denominator",
        "htmt.original_nonpositive_monotrait_mean",
    ];
    for row in 0..artifact.cells.len() {
        for column in 0..artifact.cells.len() {
            let cell = &artifact.cells[row][column];
            let mirror = &artifact.cells[column][row];
            let values_match = match (cell.value, mirror.value) {
                (Some(left), Some(right)) => approximately_equal(left, right, 1e-10),
                (None, None) => true,
                _ => false,
            };
            if cell.status != mirror.status
                || cell.reason != mirror.reason
                || !values_match
                || cell
                    .value
                    .is_some_and(|value| !value.is_finite() || (expected_absolute && value < 0.0))
                || cell
                    .reason
                    .as_deref()
                    .is_some_and(|reason| !recognized_reasons.contains(&reason))
            {
                return Err(invalid());
            }
            match cell.status {
                HtmtStatus::Available
                    if cell.value.is_none()
                        || cell.reason.is_some()
                        || (row == column && cell.value != Some(1.0)) =>
                {
                    return Err(invalid());
                }
                HtmtStatus::NotApplicable
                    if cell.value.is_some()
                        || !matches!(
                            cell.reason.as_deref(),
                            Some("htmt.formative_not_applicable")
                                | Some("htmt.single_indicator_not_applicable")
                        ) =>
                {
                    return Err(invalid());
                }
                HtmtStatus::Unavailable
                    if cell.value.is_some()
                        || row == column
                        || (expected_absolute
                            && cell.reason.as_deref()
                                != Some("htmt.zero_monotrait_denominator"))
                        || (!expected_absolute
                            && cell.reason.as_deref()
                                != Some("htmt.original_nonpositive_monotrait_mean")) =>
                {
                    return Err(invalid());
                }
                _ => {}
            }
            if let Some(recipe) = recipe {
                let left = &recipe.model.constructs[row];
                let right = &recipe.model.constructs[column];
                let left_indicator_count = effective_assessment_indicator_count(recipe, &left.id);
                let right_indicator_count = effective_assessment_indicator_count(recipe, &right.id);
                let expected_reason = if left.mode == qpls_core::MeasurementMode::Formative
                    || right.mode == qpls_core::MeasurementMode::Formative
                {
                    Some("htmt.formative_not_applicable")
                } else if left_indicator_count < 2 || right_indicator_count < 2 {
                    Some("htmt.single_indicator_not_applicable")
                } else {
                    None
                };
                if let Some(reason) = expected_reason {
                    if cell.status != HtmtStatus::NotApplicable
                        || cell.reason.as_deref() != Some(reason)
                    {
                        return Err(invalid());
                    }
                } else if cell.status == HtmtStatus::NotApplicable {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(())
}

fn effective_assessment_indicator_count(recipe: &AnalysisRecipe, construct_id: &str) -> usize {
    recipe
        .model
        .higher_order_constructs
        .iter()
        .find(|higher_order| {
            higher_order.id == construct_id && higher_order.method == HigherOrderMethod::TwoStage
        })
        .map(|higher_order| higher_order.components.len())
        .unwrap_or_else(|| {
            recipe
                .model
                .constructs
                .iter()
                .find(|construct| construct.id == construct_id)
                .map(|construct| construct.indicators.len())
                .unwrap_or(0)
        })
}

fn validate_assessment_current(
    result_id: Uuid,
    assessment: &AssessmentResult,
    estimation: &PlsResult,
    recipe: Option<&AnalysisRecipe>,
) -> Result<(), ProjectError> {
    let invalid = || {
        ProjectError::Invalid(format!(
            "result {result_id} has an inconsistent current PLS assessment payload"
        ))
    };
    let quality_ids = assessment
        .construct_quality
        .iter()
        .map(|row| row.construct.as_str())
        .collect::<std::collections::HashSet<_>>();
    let expected_quality_ids = estimation
        .outer_estimates
        .iter()
        .map(|row| row.construct.as_str())
        .collect::<std::collections::HashSet<_>>();
    let expected_quality_order = if let Some(recipe) = recipe {
        recipe
            .model
            .constructs
            .iter()
            .map(|construct| construct.id.as_str())
            .collect::<Vec<_>>()
    } else {
        let mut seen = std::collections::HashSet::new();
        estimation
            .outer_estimates
            .iter()
            .filter_map(|row| {
                seen.insert(row.construct.as_str())
                    .then_some(row.construct.as_str())
            })
            .collect::<Vec<_>>()
    };
    let actual_quality_order = assessment
        .construct_quality
        .iter()
        .map(|row| row.construct.as_str())
        .collect::<Vec<_>>();
    if quality_ids.len() != assessment.construct_quality.len()
        || quality_ids != expected_quality_ids
        || actual_quality_order != expected_quality_order
        || assessment.construct_quality.iter().any(|row| {
            row.construct.trim().is_empty()
                || [row.cronbach_alpha, row.rho_c, row.ave, row.rho_a]
                    .into_iter()
                    .flatten()
                    .any(|value| !value.is_finite())
        })
        || assessment
            .cross_loadings
            .iter()
            .any(|row| !row.loading.is_finite())
    {
        return Err(invalid());
    }
    if matches!(
        assessment.method_version.as_str(),
        ASSESSMENT_METHOD_VERSION | ASSESSMENT_METHOD_VERSION_V6 | ASSESSMENT_METHOD_VERSION_V5
    ) {
        let recognized_warnings = [
            "rho_a.two_indicator_limited_information",
            "rho_a.improper_below_zero",
            "rho_a.improper_above_one",
        ];
        let recognized_reasons = [
            "rho_a.formative_not_applicable",
            "rho_a.pca_weights_not_applicable",
            "rho_a.single_indicator_not_identified",
            "rho_a.invalid_indicator_scale",
            "rho_a.invalid_score_variance",
            "rho_a.estimation_input_mismatch",
            "rho_a.off_diagonal_denominator_zero",
            "rho_a.nonfinite_result",
        ];
        for row in &assessment.construct_quality {
            let Some(status) = row.rho_a_status else {
                return Err(invalid());
            };
            let Some(indicator_count) = row.rho_a_indicator_count else {
                return Err(invalid());
            };
            let generated_interaction_construct = recipe.is_some_and(|recipe| {
                recipe
                    .model
                    .interactions
                    .iter()
                    .any(|interaction| interaction.product_construct == row.construct)
            });
            let expected_indicator_count = if generated_interaction_construct {
                0
            } else {
                estimation
                    .outer_estimates
                    .iter()
                    .filter(|outer| outer.construct == row.construct)
                    .count()
            };
            if indicator_count != expected_indicator_count
                || row
                    .rho_a_warning_codes
                    .iter()
                    .any(|code| !recognized_warnings.contains(&code.as_str()))
            {
                return Err(invalid());
            }
            match status {
                RhoAStatus::Available => {
                    let (
                        Some(value),
                        None,
                        Some(score_variance),
                        Some(norm),
                        Some(numerator),
                        Some(denominator),
                    ) = (
                        row.rho_a,
                        row.rho_a_reason.as_ref(),
                        row.score_variance_before_normalization,
                        row.normalized_weight_norm_squared,
                        row.off_diagonal_numerator,
                        row.off_diagonal_denominator,
                    )
                    else {
                        return Err(invalid());
                    };
                    let expected = norm.powi(2) * numerator / denominator;
                    if indicator_count < 2
                        || !score_variance.is_finite()
                        || score_variance <= 0.0
                        || !norm.is_finite()
                        || norm <= 0.0
                        || !numerator.is_finite()
                        || !denominator.is_finite()
                        || denominator <= 0.0
                        || !expected.is_finite()
                        || !approximately_equal(value, expected, 1e-12)
                    {
                        return Err(invalid());
                    }
                }
                RhoAStatus::NotApplicable => {
                    if row.rho_a.is_some()
                        || row.score_variance_before_normalization.is_some()
                        || row.normalized_weight_norm_squared.is_some()
                        || row.off_diagonal_numerator.is_some()
                        || row.off_diagonal_denominator.is_some()
                        || row
                            .rho_a_reason
                            .as_deref()
                            .is_none_or(|reason| !recognized_reasons[..3].contains(&reason))
                    {
                        return Err(invalid());
                    }
                }
                RhoAStatus::Unavailable => {
                    if row.rho_a.is_some()
                        || row.normalized_weight_norm_squared.is_some()
                        || row.off_diagonal_numerator.is_some()
                        || row.off_diagonal_denominator.is_some()
                        || row
                            .rho_a_reason
                            .as_deref()
                            .is_none_or(|reason| !recognized_reasons[3..].contains(&reason))
                    {
                        return Err(invalid());
                    }
                }
            }
            if let Some(recipe) = recipe {
                let construct = recipe
                    .model
                    .constructs
                    .iter()
                    .find(|construct| construct.id == row.construct)
                    .ok_or_else(invalid)?;
                let expected_not_applicable =
                    if construct.mode == qpls_core::MeasurementMode::Formative {
                        Some("rho_a.formative_not_applicable")
                    } else if recipe.settings.weighting_scheme == qpls_core::WeightingScheme::Pca {
                        Some("rho_a.pca_weights_not_applicable")
                    } else if effective_assessment_indicator_count(recipe, &construct.id) == 1 {
                        Some("rho_a.single_indicator_not_identified")
                    } else {
                        None
                    };
                match expected_not_applicable {
                    Some(reason)
                        if status != RhoAStatus::NotApplicable
                            || row.rho_a_reason.as_deref() != Some(reason) =>
                    {
                        return Err(invalid());
                    }
                    None if status == RhoAStatus::NotApplicable => return Err(invalid()),
                    _ => {}
                }
            }
            let boundary_tolerance = row
                .rho_a
                .map(|value| 64.0 * f64::EPSILON * value.abs().max(1.0));
            let expected_warning_codes = match (status, row.rho_a, indicator_count) {
                (RhoAStatus::Available, Some(value), count) => {
                    let mut expected = Vec::new();
                    if value < -boundary_tolerance.unwrap() {
                        expected.push("rho_a.improper_below_zero");
                    } else if value > 1.0 + boundary_tolerance.unwrap() {
                        expected.push("rho_a.improper_above_one");
                    }
                    if count == 2 {
                        expected.push("rho_a.two_indicator_limited_information");
                    }
                    expected
                }
                _ => Vec::new(),
            };
            if row
                .rho_a_warning_codes
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_warning_codes
            {
                return Err(invalid());
            }
        }
    }
    let matrix_is_valid = |constructs: &[String], values: &[Vec<Option<f64>>]| {
        constructs
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == constructs.len()
            && values.len() == constructs.len()
            && values.iter().all(|row| {
                row.len() == constructs.len() && row.iter().flatten().all(|value| value.is_finite())
            })
    };
    if !matrix_is_valid(
        &assessment.fornell_larcker.constructs,
        &assessment.fornell_larcker.values,
    ) {
        return Err(invalid());
    }
    if matches!(
        assessment.method_version.as_str(),
        ASSESSMENT_METHOD_VERSION | ASSESSMENT_METHOD_VERSION_V6
    ) {
        validate_htmt_assessment(
            assessment.htmt_plus.as_ref().ok_or_else(invalid)?,
            true,
            &assessment.fornell_larcker.constructs,
            recipe,
        )?;
        validate_htmt_assessment(
            assessment.htmt_original.as_ref().ok_or_else(invalid)?,
            false,
            &assessment.fornell_larcker.constructs,
            recipe,
        )?;
    } else {
        let htmt = assessment.htmt.as_ref().ok_or_else(invalid)?;
        if htmt.constructs != assessment.fornell_larcker.constructs
            || !matrix_is_valid(&htmt.constructs, &htmt.values)
        {
            return Err(invalid());
        }
        for row in 0..htmt.values.len() {
            for column in 0..htmt.values.len() {
                match (htmt.values[row][column], htmt.values[column][row]) {
                    (Some(left), Some(right)) if (left - right).abs() <= 1e-10 => {}
                    (None, None) => {}
                    _ => return Err(invalid()),
                }
            }
        }
    }
    let mut predictors_by_target = BTreeMap::<String, std::collections::HashSet<String>>::new();
    for path in &estimation.paths {
        if !predictors_by_target
            .entry(path.target.clone())
            .or_default()
            .insert(path.source.clone())
        {
            return Err(invalid());
        }
    }
    let structural_ids = assessment
        .structural_quality
        .iter()
        .map(|row| row.construct.as_str())
        .collect::<std::collections::HashSet<_>>();
    if assessment.r_squared != estimation.r_squared
        || structural_ids.len() != assessment.structural_quality.len()
        || structural_ids.len() != assessment.r_squared.len()
        || assessment.structural_quality.iter().any(|row| {
            let expected_predictors = predictors_by_target
                .get(&row.construct)
                .map_or(0, std::collections::HashSet::len);
            let expected_adjusted = if estimation.used_observations > expected_predictors + 1 {
                Some(
                    1.0 - (1.0 - row.r_squared) * (estimation.used_observations - 1) as f64
                        / (estimation.used_observations - expected_predictors - 1) as f64,
                )
            } else {
                None
            };
            let adjusted_matches = match (row.adjusted_r_squared, expected_adjusted) {
                (Some(actual), Some(expected)) => approximately_equal(actual, expected, 1e-12),
                (None, None) => true,
                _ => false,
            };
            row.construct.trim().is_empty()
                || !row.r_squared.is_finite()
                || assessment.r_squared.get(&row.construct) != Some(&row.r_squared)
                || row.predictor_count != expected_predictors
                || !adjusted_matches
        })
        || assessment
            .r_squared
            .values()
            .any(|value| !value.is_finite())
    {
        return Err(invalid());
    }
    let structural_vif_ids = assessment
        .structural_vif
        .iter()
        .map(|row| (&row.target_construct, &row.predictor_construct))
        .collect::<std::collections::HashSet<_>>();
    let expected_structural_vif_ids = predictors_by_target
        .iter()
        .flat_map(|(target, predictors)| {
            predictors.iter().map(move |predictor| (target, predictor))
        })
        .collect::<std::collections::HashSet<_>>();
    let mut expected_structural_vif = BTreeMap::new();
    for (target, predictors) in &predictors_by_target {
        for predictor in predictors {
            let target_scores = estimation
                .construct_scores
                .get(predictor)
                .ok_or_else(|| invalid())?;
            let remaining = predictors
                .iter()
                .filter(|candidate| *candidate != predictor)
                .map(|candidate| {
                    estimation
                        .construct_scores
                        .get(candidate)
                        .map(Vec::as_slice)
                        .ok_or_else(|| invalid())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let value =
                variance_inflation_factor(target_scores, &remaining).map_err(|_| invalid())?;
            expected_structural_vif.insert((target.as_str(), predictor.as_str()), value);
        }
    }
    if structural_vif_ids.len() != assessment.structural_vif.len()
        || structural_vif_ids != expected_structural_vif_ids
        || assessment.structural_vif.iter().any(|row| {
            let expected = expected_structural_vif
                .get(&(
                    row.target_construct.as_str(),
                    row.predictor_construct.as_str(),
                ))
                .copied()
                .flatten();
            let value_matches = match (row.vif, expected) {
                (Some(actual), Some(expected)) => approximately_equal(actual, expected, 1e-10),
                (None, None) => true,
                _ => false,
            };
            row.target_construct.trim().is_empty()
                || row.predictor_construct.trim().is_empty()
                || row
                    .vif
                    .is_some_and(|value| !value.is_finite() || value < 1.0 - 1e-10)
                || !value_matches
        })
    {
        return Err(invalid());
    }
    let formative_vif_ids = assessment
        .formative_indicator_vif
        .iter()
        .map(|row| (&row.construct, &row.indicator))
        .collect::<std::collections::HashSet<_>>();
    if formative_vif_ids.len() != assessment.formative_indicator_vif.len()
        || assessment.formative_indicator_vif.iter().any(|row| {
            row.construct.trim().is_empty()
                || row.indicator.trim().is_empty()
                || row
                    .vif
                    .is_some_and(|value| !value.is_finite() || value < 1.0 - 1e-10)
        })
    {
        return Err(invalid());
    }
    if matches!(
        assessment.method_version.as_str(),
        ASSESSMENT_METHOD_VERSION
            | ASSESSMENT_METHOD_VERSION_V6
            | ASSESSMENT_METHOD_VERSION_V5
            | ASSESSMENT_METHOD_VERSION_V4
            | ASSESSMENT_METHOD_VERSION_V3
    ) {
        let effect_ids = assessment
            .f_squared
            .iter()
            .map(|row| (&row.source_construct, &row.target_construct))
            .collect::<std::collections::HashSet<_>>();
        let expected_effect_ids = estimation
            .paths
            .iter()
            .map(|path| (&path.source, &path.target))
            .collect::<std::collections::HashSet<_>>();
        if effect_ids.len() != assessment.f_squared.len()
            || effect_ids != expected_effect_ids
            || assessment.f_squared.iter().any(|row| {
                let included = estimation.r_squared.get(&row.target_construct).copied();
                let formula_matches = match (included, row.excluded_r_squared, row.f_squared) {
                    (Some(included), Some(excluded), Some(actual)) if 1.0 - included > 1e-12 => {
                        approximately_equal(actual, (included - excluded) / (1.0 - included), 1e-10)
                    }
                    (Some(included), _, None) if 1.0 - included <= 1e-12 => true,
                    (Some(_), None, None) => true,
                    _ => false,
                };
                row.source_construct.trim().is_empty()
                    || row.target_construct.trim().is_empty()
                    || !row.included_r_squared.is_finite()
                    || included != Some(row.included_r_squared)
                    || row
                        .excluded_r_squared
                        .is_some_and(|value| !value.is_finite())
                    || row.f_squared.is_some_and(|value| !value.is_finite())
                    || !formula_matches
            })
        {
            return Err(invalid());
        }
    }
    if matches!(
        assessment.method_version.as_str(),
        ASSESSMENT_METHOD_VERSION
            | ASSESSMENT_METHOD_VERSION_V6
            | ASSESSMENT_METHOD_VERSION_V5
            | ASSESSMENT_METHOD_VERSION_V4
    ) {
        let indicator_count = estimation.outer_estimates.len();
        let fit = assessment.model_fit.as_ref().ok_or_else(invalid)?;
        let fit_rows = [&fit.saturated, &fit.estimated];
        let fit_denominator = (indicator_count * (indicator_count + 1) / 2) as f64;
        if indicator_count == 0
            || fit_rows.iter().any(|row| {
                !row.srmr.is_finite()
                    || !row.d_uls.is_finite()
                    || row.srmr < 0.0
                    || row.d_uls < 0.0
                    || !approximately_equal(row.srmr.powi(2), row.d_uls / fit_denominator, 1e-10)
            })
        {
            return Err(invalid());
        }
        let valid_distances = [7usize, 5, 6, 8, 9, 10, 11, 12]
            .into_iter()
            .filter(|distance| {
                *distance < estimation.used_observations
                    && estimation.used_observations % distance != 0
            })
            .collect::<Vec<_>>();
        match &assessment.blindfolding {
            None if valid_distances.is_empty() => {}
            Some(blindfolding) if !valid_distances.is_empty() => {
                let settings = &blindfolding.settings;
                let construct_ids = blindfolding
                    .constructs
                    .iter()
                    .map(|row| row.construct.as_str())
                    .collect::<std::collections::HashSet<_>>();
                let expected_construct_ids = estimation
                    .paths
                    .iter()
                    .map(|path| path.target.as_str())
                    .collect::<std::collections::HashSet<_>>();
                if settings.omission_distance != valid_distances[0]
                    || settings.selection != "preferred_7_then_smallest_valid_5_to_12"
                    || settings.missing_value_treatment != "indicator_mean_replacement"
                    || construct_ids.len() != blindfolding.constructs.len()
                    || construct_ids != expected_construct_ids
                    || blindfolding.constructs.iter().any(|row| {
                        match (
                            row.q_squared,
                            row.prediction_error_sum_squares,
                            row.observation_sum_squares,
                        ) {
                            (Some(q_squared), Some(press), Some(sso)) => {
                                !q_squared.is_finite()
                                    || !press.is_finite()
                                    || !sso.is_finite()
                                    || press < 0.0
                                    || sso <= 0.0
                                    || !approximately_equal(q_squared, 1.0 - press / sso, 1e-10)
                            }
                            (None, None, None) => false,
                            (None, Some(press), Some(sso)) => {
                                !press.is_finite() || !sso.is_finite() || press < 0.0 || sso < 0.0
                            }
                            _ => true,
                        }
                    })
                {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
    }
    Ok(())
}

fn approximately_equal(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

pub fn load_project_with_recovery(path: &Path) -> Result<(Project, bool), ProjectError> {
    recover_incomplete_save(path)?;
    match load_project(path) {
        Ok(project) => Ok((project, false)),
        Err(primary_error) => {
            let primary_identity = read_recovery_identity(path);
            if primary_identity
                .as_ref()
                .is_some_and(|identity| identity.schema_version > PROJECT_ARCHIVE_VERSION)
            {
                return Err(primary_error);
            }
            let backup = backup_path(path);
            if !backup.exists() {
                return Err(primary_error);
            }
            match load_project(&backup) {
                Ok(project) if recovery_candidate_matches(primary_identity.as_ref(), &project) => {
                    Ok((project, true))
                }
                Ok(_) => Err(ProjectError::RecoveryFailed(format!(
                    "primary failed ({primary_error}); backup belongs to another project or is not writable"
                ))),
                Err(backup_error) => Err(ProjectError::RecoveryFailed(format!(
                    "primary failed ({primary_error}); backup failed ({backup_error})"
                ))),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverySource {
    Autosave,
    Backup,
}

pub fn load_project_with_autosave(
    path: &Path,
) -> Result<(Project, Option<RecoverySource>), ProjectError> {
    recover_incomplete_save(path)?;
    let autosave = autosave_path(path);
    match load_project(path) {
        Ok(primary) => {
            if !primary.read_only
                && autosave.exists()
                && let Ok(autosaved) = load_project(&autosave)
                && recovery_candidate_matches(
                    Some(&ArchiveIdentity::from_project(&primary)),
                    &autosaved,
                )
                && autosaved.manifest.modified_at > primary.manifest.modified_at
            {
                return Ok((autosaved, Some(RecoverySource::Autosave)));
            }
            Ok((primary, None))
        }
        Err(primary_error) => {
            let primary_identity = read_recovery_identity(path);
            if primary_identity
                .as_ref()
                .is_some_and(|identity| identity.schema_version > PROJECT_ARCHIVE_VERSION)
            {
                return Err(primary_error);
            }

            let backup = backup_path(path);
            let backup_attempt = backup.exists().then(|| load_project(&backup));
            let backup_candidate = backup_attempt.as_ref().and_then(|attempt| {
                attempt.as_ref().ok().filter(|project| {
                    recovery_candidate_matches(primary_identity.as_ref(), project)
                })
            });
            let anchor = primary_identity
                .as_ref()
                .cloned()
                .or_else(|| backup_candidate.map(ArchiveIdentity::from_project));

            let autosave_attempt = autosave.exists().then(|| load_project(&autosave));
            let autosave_candidate = autosave_attempt.as_ref().and_then(|attempt| {
                attempt.as_ref().ok().filter(|project| {
                    anchor
                        .as_ref()
                        .is_some_and(|identity| recovery_candidate_matches(Some(identity), project))
                })
            });

            match (autosave_candidate, backup_candidate) {
                (Some(autosaved), Some(backed_up))
                    if autosaved.manifest.modified_at > backed_up.manifest.modified_at =>
                {
                    Ok((autosaved.clone(), Some(RecoverySource::Autosave)))
                }
                (_, Some(backed_up)) => Ok((backed_up.clone(), Some(RecoverySource::Backup))),
                (Some(autosaved), None) => Ok((autosaved.clone(), Some(RecoverySource::Autosave))),
                (None, None) => Err(ProjectError::RecoveryFailed(format!(
                    "primary failed ({primary_error}); autosave {}; backup {}",
                    recovery_attempt_detail(autosave_attempt.as_ref()),
                    recovery_attempt_detail(backup_attempt.as_ref())
                ))),
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ArchiveIdentity {
    project_id: Uuid,
    schema_version: u32,
}

impl ArchiveIdentity {
    fn from_project(project: &Project) -> Self {
        Self {
            project_id: project.manifest.project_id,
            schema_version: project.source_archive_version,
        }
    }
}

fn read_archive_identity(path: &Path) -> Result<ArchiveIdentity, ProjectError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS)
        .map_err(map_archive_integrity_error)?;
    let bytes = read_preflighted_entry(
        &mut archive,
        &preflight,
        archive_integrity::MANIFEST_ENTRY_NAME,
        MAX_MANIFEST_UNCOMPRESSED_BYTES,
    )
    .map_err(map_archive_integrity_error)?;
    let manifest: ProjectManifest = serde_json::from_slice(&bytes)?;
    Ok(ArchiveIdentity {
        project_id: manifest.project_id,
        schema_version: manifest.schema_version,
    })
}

fn read_recovery_identity(path: &Path) -> Option<ArchiveIdentity> {
    read_archive_identity(path)
        .ok()
        .or_else(|| read_identity_sidecar(path).ok())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveTransactionJournal {
    schema_version: u32,
    primary: String,
    rotation: String,
    temporary: String,
    backup: String,
    new_project_id: Uuid,
    previous_project_id: Uuid,
    new_archive_sha256: String,
    previous_archive_sha256: String,
}

fn recover_incomplete_save(path: &Path) -> Result<(), ProjectError> {
    let journal = transaction_journal_path(path);
    if !journal.exists() {
        return Ok(());
    }
    let transaction: SaveTransactionJournal = match fs::read(&journal)
        .map_err(ProjectError::from)
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(ProjectError::from))
    {
        Ok(transaction) => transaction,
        Err(_error) if safe_without_transaction_journal(path) => {
            quarantine_artifact(&journal);
            return Ok(());
        }
        Err(error) => {
            return Err(ProjectError::RecoveryFailed(format!(
                "save transaction journal is unreadable and no verified primary or matching backup is available ({error})"
            )));
        }
    };
    validate_transaction_journal(path, &transaction)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(&transaction.temporary);
    let rotation = parent.join(&transaction.rotation);

    if path.exists() {
        let primary_hash = sha256_file(path)?;
        if primary_hash == transaction.new_archive_sha256 {
            let project = load_project(path)?;
            if project.manifest.project_id != transaction.new_project_id {
                return Err(ProjectError::RecoveryFailed(
                    "committed save generation has the wrong project identity".into(),
                ));
            }
            remove_if_generation_matches(&temporary, &transaction.new_archive_sha256);
            finalize_committed_generation(
                path,
                &rotation,
                &journal,
                &transaction,
                &project.manifest,
            );
            return Ok(());
        }
        if primary_hash == transaction.previous_archive_sha256 {
            let project = load_project(path)?;
            if project.manifest.project_id != transaction.previous_project_id {
                return Err(ProjectError::RecoveryFailed(
                    "prior save generation has the wrong project identity".into(),
                ));
            }
            remove_if_generation_matches(&temporary, &transaction.new_archive_sha256);
            remove_if_generation_matches(&rotation, &transaction.previous_archive_sha256);
            if write_identity_sidecar(path, &project.manifest).is_ok() {
                let _ = fs::remove_file(&journal);
            }
            return Ok(());
        }
        if generation_matches(&rotation, &transaction.previous_archive_sha256)
            && let Ok(project) = load_project(&rotation)
            && project.manifest.project_id == transaction.previous_project_id
        {
            quarantine_artifact(path);
            fs::rename(&rotation, path)?;
            remove_if_generation_matches(&temporary, &transaction.new_archive_sha256);
            if write_identity_sidecar(path, &project.manifest).is_ok() {
                let _ = fs::remove_file(&journal);
            }
            return Ok(());
        }
        let backup = backup_path(path);
        if generation_matches(&backup, &transaction.previous_archive_sha256)
            && let Ok(project) = load_project(&backup)
            && project.manifest.project_id == transaction.previous_project_id
        {
            quarantine_artifact(path);
            fs::copy(&backup, path)?;
            File::open(path)?.sync_all()?;
            remove_if_generation_matches(&temporary, &transaction.new_archive_sha256);
            remove_if_generation_matches(&rotation, &transaction.previous_archive_sha256);
            if write_identity_sidecar(path, &project.manifest).is_ok() {
                let _ = fs::remove_file(&journal);
            }
            return Ok(());
        }
    } else if generation_matches(&temporary, &transaction.new_archive_sha256)
        && let Ok(project) = load_project(&temporary)
        && project.manifest.project_id == transaction.new_project_id
    {
        fs::rename(&temporary, path)?;
        finalize_committed_generation(path, &rotation, &journal, &transaction, &project.manifest);
        return Ok(());
    }

    if !path.exists()
        && generation_matches(&rotation, &transaction.previous_archive_sha256)
        && let Ok(project) = load_project(&rotation)
        && project.manifest.project_id == transaction.previous_project_id
    {
        fs::rename(&rotation, path)?;
        remove_if_generation_matches(&temporary, &transaction.new_archive_sha256);
        if write_identity_sidecar(path, &project.manifest).is_ok() {
            let _ = fs::remove_file(&journal);
        }
        return Ok(());
    }
    Err(ProjectError::RecoveryFailed(
        "an interrupted save was found, but neither its exact intended generation nor its exact prior generation could be safely restored"
            .into(),
    ))
}

fn validate_transaction_journal(
    path: &Path,
    transaction: &SaveTransactionJournal,
) -> Result<(), ProjectError> {
    if transaction.schema_version != 2 {
        return Err(ProjectError::RecoveryFailed(
            "save transaction journal has an unsupported schema".into(),
        ));
    }
    validate_sha256_text(&transaction.new_archive_sha256, "new archive")?;
    validate_sha256_text(&transaction.previous_archive_sha256, "previous archive")?;
    let file_name = |candidate: &Path| {
        candidate
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                ProjectError::RecoveryFailed("project path has no valid file name".into())
            })
    };
    if transaction.primary != file_name(path)?
        || transaction.rotation != file_name(&transaction_rotation_path(path))?
        || transaction.backup != file_name(&backup_path(path))?
        || Path::new(&transaction.temporary)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(transaction.temporary.as_str())
        || !transaction
            .temporary
            .starts_with(&format!("{}.", file_name(path)?))
        || !transaction.temporary.contains("qpls.tmp-")
    {
        return Err(ProjectError::RecoveryFailed(
            "save transaction journal contains paths outside the project directory".into(),
        ));
    }
    Ok(())
}

fn validate_sha256_text(value: &str, label: &str) -> Result<(), ProjectError> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(ProjectError::RecoveryFailed(format!(
            "save journal {label} SHA-256 is invalid"
        )));
    }
    Ok(())
}

fn safe_without_transaction_journal(path: &Path) -> bool {
    if load_project(path).is_ok() {
        return true;
    }
    let Some(identity) = read_recovery_identity(path) else {
        return false;
    };
    if identity.schema_version > PROJECT_ARCHIVE_VERSION {
        return false;
    }
    load_project(&backup_path(path))
        .ok()
        .is_some_and(|project| recovery_candidate_matches(Some(&identity), &project))
}

fn generation_matches(path: &Path, expected_sha256: &str) -> bool {
    path.exists() && sha256_file(path).is_ok_and(|actual| actual == expected_sha256)
}

fn remove_if_generation_matches(path: &Path, expected_sha256: &str) {
    if generation_matches(path, expected_sha256) {
        let _ = fs::remove_file(path);
    }
}

fn quarantine_artifact(path: &Path) {
    if !path.exists() {
        return;
    }
    let quarantine = sibling_path_with_suffix(path, ".quarantine");
    if quarantine.exists() {
        return;
    }
    let _ = fs::rename(path, &quarantine);
}

fn finalize_committed_generation(
    path: &Path,
    rotation: &Path,
    journal: &Path,
    transaction: &SaveTransactionJournal,
    manifest: &ProjectManifest,
) {
    let backup_ready = match preserve_rotation_as_backup(
        path,
        rotation,
        transaction.previous_project_id,
        &transaction.previous_archive_sha256,
    ) {
        Ok(()) => true,
        Err(
            ProjectError::RecoveryFailed(_)
            | ProjectError::Invalid(_)
            | ProjectError::MissingEntry(_)
            | ProjectError::ChecksumMismatch(_)
            | ProjectError::Json(_)
            | ProjectError::Zip(_)
            | ProjectError::Data(_),
        ) => {
            quarantine_artifact(rotation);
            true
        }
        Err(ProjectError::ReadOnly | ProjectError::RollbackFailed { .. } | ProjectError::Io(_)) => {
            false
        }
    };
    let identity_ready = write_identity_sidecar(path, manifest).is_ok();
    if backup_ready && identity_ready {
        let _ = fs::remove_file(journal);
    }
}

fn preserve_rotation_as_backup(
    path: &Path,
    rotation: &Path,
    expected_previous: Uuid,
    expected_sha256: &str,
) -> Result<(), ProjectError> {
    let backup = backup_path(path);
    let displaced = transaction_displaced_backup_path(path);
    if !rotation.exists() {
        if backup.exists() {
            if displaced.exists() {
                fs::remove_file(displaced)?;
            }
        } else if displaced.exists() {
            fs::rename(displaced, backup)?;
        }
        return Ok(());
    }
    if !generation_matches(rotation, expected_sha256) {
        if backup.exists()
            && let Ok(existing) = load_project(&backup)
            && existing.manifest.project_id == expected_previous
        {
            quarantine_artifact(rotation);
            return Ok(());
        }
        return Err(ProjectError::RecoveryFailed(
            "save transaction rotation does not match the prior generation".into(),
        ));
    }
    let previous = load_project(rotation)?;
    if previous.manifest.project_id != expected_previous {
        return Err(ProjectError::RecoveryFailed(
            "save transaction rotation belongs to another project".into(),
        ));
    }
    if backup.exists() {
        if displaced.exists() {
            return Err(ProjectError::RecoveryFailed(
                "a displaced backup from an earlier transaction still requires recovery".into(),
            ));
        }
        fs::rename(&backup, &displaced)?;
        if let Err(error) = fs::rename(rotation, &backup) {
            let _ = fs::rename(&displaced, &backup);
            return Err(error.into());
        }
        if let Err(error) = fs::remove_file(&displaced) {
            let _ = fs::remove_file(&backup);
            let _ = fs::rename(&displaced, &backup);
            return Err(error.into());
        }
        return Ok(());
    }
    fs::rename(rotation, &backup)?;
    if displaced.exists() {
        fs::remove_file(displaced)?;
    }
    Ok(())
}

fn write_identity_sidecar(path: &Path, manifest: &ProjectManifest) -> Result<(), ProjectError> {
    let sidecar = identity_sidecar_path(path);
    let temporary = sibling_path_with_suffix(&sidecar, &format!(".tmp-{}", Uuid::new_v4()));
    let value = serde_json::json!({
        "schemaVersion": 1,
        "projectId": manifest.project_id,
        "sourceArchiveVersion": manifest.schema_version,
    });
    write_synced_create_new(&temporary, &serde_json::to_vec_pretty(&value)?)?;
    let previous = sidecar
        .exists()
        .then(|| sibling_path_with_suffix(&sidecar, ".previous"));
    if let Some(previous) = &previous {
        if previous.exists() {
            fs::remove_file(previous)?;
        }
        fs::rename(&sidecar, previous)?;
    }
    if let Err(error) = fs::rename(&temporary, &sidecar) {
        if let Some(previous) = &previous {
            let _ = fs::rename(previous, &sidecar);
        }
        return Err(error.into());
    }
    if let Some(previous) = previous {
        let _ = fs::remove_file(previous);
    }
    Ok(())
}

fn read_identity_sidecar(path: &Path) -> Result<ArchiveIdentity, ProjectError> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(identity_sidecar_path(path))?)?;
    if value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        return Err(ProjectError::Invalid(
            "project recovery identity sidecar has an unsupported schema".into(),
        ));
    }
    let project_id = value
        .get("projectId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ProjectError::Invalid("project recovery identity is missing projectId".into())
        })?
        .parse()
        .map_err(|_| {
            ProjectError::Invalid("project recovery identity has invalid projectId".into())
        })?;
    let schema_version = value
        .get("sourceArchiveVersion")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| {
            ProjectError::Invalid("project recovery identity has invalid archive version".into())
        })?;
    Ok(ArchiveIdentity {
        project_id,
        schema_version,
    })
}

fn recovery_candidate_matches(identity: Option<&ArchiveIdentity>, candidate: &Project) -> bool {
    !candidate.read_only
        && candidate.source_archive_version <= PROJECT_ARCHIVE_VERSION
        && identity.is_some_and(|identity| identity.project_id == candidate.manifest.project_id)
}

fn recovery_attempt_detail(attempt: Option<&Result<Project, ProjectError>>) -> String {
    match attempt {
        None => "is absent".into(),
        Some(Err(error)) => format!("failed ({error})"),
        Some(Ok(project)) if project.read_only => "is future/read-only".into(),
        Some(Ok(_)) => "was rejected because its project identity did not match".into(),
    }
}

pub fn save_autosave(path: &Path, project: &Project) -> Result<(), ProjectError> {
    save_project(&autosave_path(path), project).map(|_| ())
}

pub fn discard_autosave(path: &Path) -> Result<(), ProjectError> {
    let autosave = autosave_path(path);
    if autosave.exists() {
        fs::remove_file(&autosave)?;
    }
    let backup = backup_path(&autosave_path(path));
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    for artifact in [
        transaction_rotation_path(&autosave),
        transaction_journal_path(&autosave),
        transaction_displaced_backup_path(&autosave),
        identity_sidecar_path(&autosave),
    ] {
        if artifact.exists() {
            fs::remove_file(artifact)?;
        }
    }
    Ok(())
}

struct TemporaryArchiveGuard {
    path: Option<PathBuf>,
}

impl TemporaryArchiveGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TemporaryArchiveGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn promote_validated_archive(
    path: &Path,
    temporary: &Path,
    manifest: &ProjectManifest,
) -> Result<(), ProjectError> {
    if !path.exists() {
        fs::rename(temporary, path)?;
        let _ = write_identity_sidecar(path, manifest);
        return Ok(());
    }

    let backup = backup_path(path);
    let rotation = transaction_rotation_path(path);
    let journal = transaction_journal_path(path);
    let previous_identity = read_recovery_identity(path).ok_or_else(|| {
        ProjectError::RecoveryFailed("cannot establish the project identity before saving".into())
    })?;
    let file_name = |candidate: &Path| -> Result<String, ProjectError> {
        candidate
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| ProjectError::Invalid("project path has no valid file name".into()))
    };
    let transaction = SaveTransactionJournal {
        schema_version: 2,
        primary: file_name(path)?,
        rotation: file_name(&rotation)?,
        temporary: file_name(temporary)?,
        backup: file_name(&backup)?,
        new_project_id: manifest.project_id,
        previous_project_id: previous_identity.project_id,
        new_archive_sha256: sha256_file(temporary)?,
        previous_archive_sha256: sha256_file(path)?,
    };
    write_transaction_journal(&journal, &transaction)?;

    // Copy the current primary into a deterministic transaction rotation before
    // removing it. A crash at any point leaves at least the original primary,
    // the recognized backup, or the journal-addressable rotation.
    fs::copy(path, &rotation)?;
    OpenOptions::new().write(true).open(&rotation)?.sync_all()?;
    if !generation_matches(&rotation, &transaction.previous_archive_sha256)
        || load_project(&rotation)
            .ok()
            .is_none_or(|project| project.manifest.project_id != transaction.previous_project_id)
    {
        quarantine_artifact(&rotation);
        let _ = fs::remove_file(&journal);
        return Err(ProjectError::RecoveryFailed(
            "the verified prior project generation changed or could not be copied exactly; the original primary was left untouched"
                .into(),
        ));
    }
    fs::remove_file(path)?;
    if let Err(promotion) = fs::rename(temporary, path) {
        return match fs::rename(&rotation, path) {
            Ok(()) => {
                let _ = fs::remove_file(&journal);
                Err(ProjectError::Io(promotion))
            }
            Err(rollback) => Err(ProjectError::RollbackFailed {
                promotion: promotion.to_string(),
                rollback: rollback.to_string(),
            }),
        };
    }

    // The primary is committed at this point. Generation finalization preserves
    // the immediately previous archive as `.bak`, durably updates recovery
    // identity, and keeps the journal when any retryable metadata step fails.
    finalize_committed_generation(path, &rotation, &journal, &transaction, manifest);
    Ok(())
}

fn write_synced_create_new(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn write_transaction_journal(
    journal: &Path,
    transaction: &SaveTransactionJournal,
) -> Result<(), ProjectError> {
    let temporary = sibling_path_with_suffix(journal, &format!(".tmp-{}", Uuid::new_v4()));
    write_synced_create_new(&temporary, &serde_json::to_vec_pretty(transaction)?)?;
    if let Err(error) = fs::rename(&temporary, journal) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if let Some(parent) = journal.parent()
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, ProjectError> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn temporary_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, &format!(".tmp-{}", Uuid::new_v4()))
}
fn transaction_rotation_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".transaction-previous")
}
fn transaction_journal_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".transaction.json")
}
fn transaction_displaced_backup_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".transaction-backup")
}
fn identity_sidecar_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".identity.json")
}
pub fn backup_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".bak")
}
pub fn autosave_path(path: &Path) -> PathBuf {
    sibling_path_with_suffix(path, ".autosave")
}
fn sibling_path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::io::Read;

    fn has_compatibility_notice(project: &Project, result_id: Uuid, code: &str) -> bool {
        project.compatibility_notices.iter().any(|notice| {
            notice.result_id == result_id
                && notice.diagnostic.code == code
                && notice.diagnostic.level == DiagnosticLevel::Warning
        })
    }

    fn migrated_execution_recipe(bytes: &[u8]) -> AnalysisRecipe {
        let recipe: AnalysisRecipe = serde_json::from_slice(bytes).unwrap();
        if recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION {
            recipe
        } else {
            recipe.migrated_v3().unwrap()
        }
    }

    fn pls_family_fixture(method: AnalysisMethod) -> (Dataset, AnalysisRecipe) {
        let (data, data_name, recipe_json): (&[u8], &str, &[u8]) = match method {
            AnalysisMethod::Plsc => (
                include_bytes!("../../../validation/results/plsc_reference.csv"),
                "plsc_reference.csv",
                include_bytes!("../../../validation/results/plsc_reference.recipe.json"),
            ),
            AnalysisMethod::Wpls => (
                include_bytes!("../../../validation/results/wpls_reference.csv"),
                "wpls_reference.csv",
                include_bytes!("../../../validation/results/wpls_reference.recipe.json"),
            ),
            _ => panic!("unsupported test fixture method {method}"),
        };
        let dataset =
            import_delimited_bytes(data, data_name, b',', &ImportOptions::default()).unwrap();
        let mut recipe = migrated_execution_recipe(recipe_json);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        (dataset, recipe)
    }

    fn completed_pls_family_result(dataset: &Dataset, recipe: &AnalysisRecipe) -> AnalysisResult {
        let estimation = qpls_estimation::estimate_pls(dataset, recipe).unwrap();
        let estimation_method_version = estimation.method_version.clone();
        let assessment = qpls_assessment::assess_pls(dataset, recipe, &estimation).unwrap();
        AnalysisResult::completed_pls(
            recipe,
            format!(
                "{PLS_METHOD_VERSION}+{estimation_method_version}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            ),
            Utc::now(),
            serde_json::to_value(estimation).unwrap(),
            serde_json::to_value(assessment).unwrap(),
            Vec::new(),
        )
    }

    fn runner_generated_prediction() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/plspredict_holdout_reference.csv"),
            "plspredict_holdout_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/plspredict_holdout_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_mga() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/mga_reference.csv"),
            "mga_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/mga_reference.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe
            .metadata
            .insert("group_methods".into(), "micom,mga_permutation".into());
        recipe
            .metadata
            .insert("group_permutation_samples".into(), "5000".into());
        recipe
            .metadata
            .insert("micom_configural_confirmed".into(), "true".into());
        let mut recipe = recipe.migrated_v3().unwrap();
        recipe.metadata.remove("status");
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_cca() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/cca_reference.csv"),
            "cca_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/cca_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_cta_pls() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/cta_pls_reference.csv"),
            "cta_pls_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/cta_pls_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.workers = 1;
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_gsca() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::Gsca;
        recipe.method_config = Some(qpls_core::MethodConfig::Gsca);
        recipe.settings.workers = 1;
        recipe.settings.max_iterations = 3_000;
        recipe.settings.tolerance = 1e-7;
        recipe.metadata.insert(
            "status".into(),
            "validated_gsca_als_v2_bounded_scope".into(),
        );
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_direct_only_pls() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_cbsem(model_type: &str) -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let (data, data_name, recipe_json): (&[u8], &str, &[u8]) = match model_type {
            "cfa" => (
                include_bytes!("../../../validation/results/lavaan_two_factor_cfa.csv"),
                "lavaan_two_factor_cfa.csv",
                include_bytes!("../../../validation/results/lavaan_two_factor_cfa.recipe.json"),
            ),
            "sem" => (
                include_bytes!("../../../validation/results/lavaan_latent_regression_sem.csv"),
                "lavaan_latent_regression_sem.csv",
                include_bytes!(
                    "../../../validation/results/lavaan_latent_regression_sem.recipe.json"
                ),
            ),
            other => panic!("unsupported CB-SEM fixture model type {other}"),
        };
        let dataset =
            import_delimited_bytes(data, data_name, b',', &ImportOptions::default()).unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(recipe_json).unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.workers = 1;
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.metadata.insert(
            "status".into(),
            "validated_v1_2_4_cbsem_single_group_bounded_scope".into(),
        );
        recipe
            .metadata
            .insert("cbsem_model_type".into(), model_type.into());
        recipe
            .metadata
            .insert("cbsem_estimator".into(), "ml".into());
        recipe.metadata.insert("cbsem_input".into(), "raw".into());
        recipe
            .metadata
            .insert("cbsem_mean_structure".into(), "false".into());
        let mut recipe = recipe.migrated_v3().unwrap();
        recipe.metadata.remove("status");
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_ipma() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/ipma_reference.csv"),
            "ipma_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/ipma_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_nca() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            b"x,y\n0,1\n1,3\n2,2\n3,4\n4,4.5\n5,5\n",
            "nca-v2.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Nca;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.seed = 20_260_811;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "NCA v2 persistence".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(qpls_core::MethodConfig::Nca {
                condition: "x".into(),
                outcome: "y".into(),
                ceiling: qpls_core::NcaCeiling::Both,
                permutation_samples: 19,
            }),
            metadata: BTreeMap::from([("status".into(), "validated_nca_v2_bounded_scope".into())]),
        };
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_pca() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            b"a,b,c,d\n1,1.2,1,8\n2,2.1,4,7\n3,2.8,2,6\n4,4.2,5,5\n5,4.9,3,4\n6,6.1,7,3\n7,6.8,2,2\n8,8.2,8,1\n",
            "pca-v1.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Pca;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "PCA v1 persistence".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(qpls_core::MethodConfig::Pca {
                variables: vec!["a".into(), "b".into(), "c".into(), "d".into()],
                retention: qpls_core::PcaRetentionConfig::VarianceThreshold { threshold: 0.80 },
            }),
            metadata: BTreeMap::from([("status".into(), "validated_pca_v1_bounded_scope".into())]),
        };
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_ols() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            b"y,x,m,z\n2.1,1,0,2\n3.7,2,1,1\n5.2,3,1,0\n7.9,4,2,2\n9.4,5,2,1\n11.8,6,3,0\n13.0,7,3,2\n15.7,8,4,1\n17.2,9,4,0\n19.6,10,5,2\n21.1,11,5,1\n23.4,12,6,0\n",
            "ols-v1.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.confidence_level = 0.95;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "OLS v1 persistence".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(qpls_core::MethodConfig::Regression {
                outcome: "y".into(),
                predictors: vec!["x".into(), "m".into()],
                controls: vec!["z".into()],
                model: qpls_core::RegressionModelConfig::Ols {
                    robust_se: qpls_core::RobustStandardError::Hc3,
                },
                bootstrap: None,
            }),
            metadata: BTreeMap::new(),
        };
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_regression_fixture(
        recipe_bytes: &[u8],
    ) -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(recipe_bytes);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.preprocessing = Preprocessing::Unstandardized;
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_logistic() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        runner_generated_regression_fixture(include_bytes!(
            "../../../validation/results/v08_regression_logistic.recipe.json"
        ))
    }

    fn runner_generated_regression_bootstrap(
        logistic: bool,
        workers: usize,
    ) -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let (dataset, mut recipe, _) = if logistic {
            runner_generated_logistic()
        } else {
            runner_generated_ols()
        };
        recipe.settings.bootstrap_samples = 99;
        recipe.settings.seed = 91;
        recipe.settings.workers = workers;
        let Some(qpls_core::MethodConfig::Regression { bootstrap, .. }) =
            recipe.method_config.as_mut()
        else {
            panic!("fixture must use typed regression")
        };
        *bootstrap = Some(qpls_core::RegressionBootstrapConfig {
            algorithm: qpls_core::RegressionBootstrapAlgorithm::CaseResampling,
            intervals: vec![
                qpls_core::RegressionBootstrapInterval::Percentile,
                qpls_core::RegressionBootstrapInterval::Bca,
            ],
        });
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn legacy_logistic_v1_result(mut result: AnalysisResult) -> AnalysisResult {
        const LEGACY_WARNING: &str = "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, and Firth-corrected models remain unsupported.";
        result.provenance.method_version = REGRESSION_LOGISTIC_METHOD_VERSION_V1.into();
        for diagnostic in &mut result.diagnostics {
            if diagnostic.message.starts_with("Logistic regression v2") {
                diagnostic.message = LEGACY_WARNING.into();
            }
        }
        let estimation = estimation_payload_mut(&mut result);
        estimation["method_version"] = serde_json::json!(REGRESSION_LOGISTIC_METHOD_VERSION_V1);
        estimation["warnings"] = serde_json::json!([LEGACY_WARNING]);
        let regression = estimation["regression"].as_object_mut().unwrap();
        regression.insert(
            "method_version".into(),
            serde_json::json!(REGRESSION_LOGISTIC_METHOD_VERSION_V1),
        );
        regression.insert("warnings".into(), serde_json::json!([LEGACY_WARNING]));
        regression.remove("logistic");
        for coefficient in regression["coefficients"].as_array_mut().unwrap() {
            let coefficient = coefficient.as_object_mut().unwrap();
            coefficient.remove("odds_ratio_confidence_interval_lower");
            coefficient.remove("odds_ratio_confidence_interval_upper");
        }
        let fit = regression["fit"].as_object_mut().unwrap();
        for field in [
            "null_log_likelihood",
            "deviance",
            "null_deviance",
            "likelihood_ratio_chi_square",
            "likelihood_ratio_degrees_of_freedom",
            "likelihood_ratio_p_value",
            "pseudo_r_squared_method",
        ] {
            fit.remove(field);
        }
        result
    }

    fn runner_generated_process() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let recipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v08_process.recipe.json"
        ))
        .unwrap();
        let result = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v08_process_quickpls.json"
        ))
        .unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_process_v2() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let mut csv = String::from("X,M,W,Y\n");
        for index in 0..32 {
            let x = index as f64 / 4.0 - 4.0;
            let w = ((index * 7) % 17) as f64 / 4.0 - 2.0;
            let m = 0.55 * x + 0.2 * x * w + ((index * 3) % 11) as f64 / 50.0;
            let y = 0.3 * x + 0.75 * m + ((index * 5) % 13) as f64 / 60.0;
            csv.push_str(&format!("{x},{m},{w},{y}\n"));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "process-v2-project.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.bootstrap_samples = 99;
        settings.seed = 1729;
        settings.workers = 2;
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: ModelSpec {
                id: Uuid::new_v4(),
                name: "PROCESS v2 project".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(qpls_core::MethodConfig::Regression {
                outcome: "Y".into(),
                predictors: vec!["X".into(), "M".into(), "W".into()],
                controls: Vec::new(),
                model: qpls_core::RegressionModelConfig::Process {
                    relationship: qpls_core::ProcessRelationshipConfig::Graph {
                        focal_predictor: "X".into(),
                        paths: vec![
                            qpls_core::ProcessPathConfig {
                                from: "X".into(),
                                to: "M".into(),
                            },
                            qpls_core::ProcessPathConfig {
                                from: "M".into(),
                                to: "Y".into(),
                            },
                            qpls_core::ProcessPathConfig {
                                from: "X".into(),
                                to: "Y".into(),
                            },
                        ],
                        moderators: vec![qpls_core::ProcessModeratorConfig {
                            variable: "W".into(),
                            scale: qpls_core::ProcessModeratorScale::Continuous,
                        }],
                        moderations: vec![qpls_core::ProcessModerationConfig {
                            from: "X".into(),
                            to: "M".into(),
                            moderator: "W".into(),
                            conditioning_moderator: None,
                        }],
                        continuous_product_centering:
                            qpls_core::ProcessContinuousCentering::EquationCompleteCaseMeanV1,
                    },
                },
                bootstrap: Some(qpls_core::RegressionBootstrapConfig {
                    algorithm: qpls_core::RegressionBootstrapAlgorithm::CaseResampling,
                    intervals: vec![
                        qpls_core::RegressionBootstrapInterval::Percentile,
                        qpls_core::RegressionBootstrapInterval::Bca,
                    ],
                }),
            }),
            metadata: BTreeMap::new(),
        };
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn process_v2_with_high_leverage_bootstrap_failure(
        mut result: AnalysisResult,
    ) -> AnalysisResult {
        let graph_value = estimation_payload(&result)["regression"]["process"]["graph_v2"].clone();
        let mut graph: ProcessGraphAnalysis = serde_json::from_value(graph_value).unwrap();
        let mut bootstrap = graph.bootstrap.take().unwrap();
        let failed = bootstrap
            .validation_witness
            .successful_bootstrap
            .pop()
            .unwrap();
        bootstrap.usable_replicates -= 1;
        bootstrap
            .failed_replicates
            .push(qpls_estimation::ProcessBootstrapFailedReplicate {
                replicate_index: failed.replicate_index,
                reason_code: "high_leverage_hc3_instability".into(),
                message: "PROCESS equation Y has unstable HC3 leverage in this resample.".into(),
            });
        let original = process_bootstrap_estimands(&graph);
        bootstrap.estimands = summarize_process_bootstrap_estimands(
            &bootstrap.validation_witness.estimand_ids,
            &original.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
            &bootstrap
                .validation_witness
                .successful_bootstrap
                .iter()
                .map(|row| row.estimates.clone())
                .collect::<Vec<_>>(),
            &bootstrap
                .validation_witness
                .successful_jackknife
                .iter()
                .map(|row| row.estimates.clone())
                .collect::<Vec<_>>(),
            bootstrap.jackknife_cases,
            0.95,
        )
        .unwrap();
        bootstrap.warnings.push(format!(
            "1 of {} PROCESS bootstrap replicates failed and were excluded from inference.",
            bootstrap.requested_replicates
        ));
        graph.bootstrap = Some(bootstrap);
        estimation_payload_mut(&mut result)["regression"]["process"]["graph_v2"] =
            serde_json::to_value(graph).unwrap();
        result
    }

    fn runner_generated_mediation() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/mediation_reference.csv"),
            "mediation_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/mediation_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_moderation() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/moderation_reference_base.csv"),
            "moderation_reference_base.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/moderation_reference_base.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn runner_generated_higher_order() -> (Dataset, AnalysisRecipe, AnalysisResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/higher_order_two_stage_base.csv"),
            "higher_order_two_stage_base.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/higher_order_two_stage_base.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        (dataset, recipe, result)
    }

    fn estimation_payload_mut(result: &mut AnalysisResult) -> &mut serde_json::Value {
        match &mut result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. }
            | AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            AnalysisPayload::Legacy { .. } => panic!("expected a typed PLS payload"),
        }
    }

    fn estimation_payload(result: &AnalysisResult) -> &serde_json::Value {
        match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. }
            | AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            AnalysisPayload::Legacy { .. } => panic!("expected a typed PLS payload"),
        }
    }

    fn legacy_nca_v1_result(mut result: AnalysisResult) -> AnalysisResult {
        result.provenance.method_version = NCA_METHOD_VERSION_V1.into();
        let estimation = estimation_payload_mut(&mut result);
        estimation["method_version"] = serde_json::json!(NCA_METHOD_VERSION_V1);
        estimation["nca"]["method_version"] = serde_json::json!(NCA_METHOD_VERSION_V1);
        estimation["nca"].as_object_mut().unwrap().remove("scope");
        estimation["nca"]
            .as_object_mut()
            .unwrap()
            .remove("ce_fdh_peers");
        for row in estimation["nca"]["ceilings"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("slope");
            row.as_object_mut().unwrap().remove("intercept");
        }
        for row in estimation["nca"]["bottlenecks"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("ceiling");
            row.as_object_mut().unwrap().remove("status");
        }
        estimation["nca"]["bottlenecks"]
            .as_array_mut()
            .unwrap()
            .truncate(9);
        result
    }

    fn legacy_prediction_v1_result(mut result: AnalysisResult) -> AnalysisResult {
        result.provenance.method_version = result
            .provenance
            .method_version
            .replace(PLS_PREDICT_METHOD_VERSION, PLS_PREDICT_METHOD_VERSION_V1);
        let estimation = estimation_payload_mut(&mut result);
        estimation["method_version"] = serde_json::json!(PLS_PREDICT_METHOD_VERSION_V1);
        let predict = &mut estimation["predict"];
        predict["method_version"] = serde_json::json!(PLS_PREDICT_METHOD_VERSION_V1);
        predict["primary_analysis"] = serde_json::json!("");
        predict["indicator_targets"] = serde_json::json!([]);
        let observations = predict["training_observations"].as_u64().unwrap()
            + predict["test_observations"].as_u64().unwrap();
        let repeated = &mut predict["repeated_kfold"];
        repeated["method_version"] = serde_json::json!("plspredict_repeated_kfold_v1");
        repeated["folds"] = serde_json::json!(5);
        repeated["repeats"] = serde_json::json!(3);
        repeated["assignment"] =
            serde_json::json!("deterministic_complete_case_index_multiplier_modulo_5");
        repeated["seed"] = serde_json::json!(0);
        repeated["assignment_digest"] = serde_json::json!("");
        repeated["total_test_observations"] = serde_json::json!(observations * 3);
        repeated["indicator_targets"] = serde_json::json!([]);
        repeated["cvpat_benchmark_assessments"] = serde_json::json!([]);
        repeated["paired_loss_diagnostics"] = serde_json::json!([]);
        result
    }

    #[test]
    fn project_round_trip_preserves_arrow_dataset_and_manifest() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let mut project = Project::new("Study");
        project.datasets.push(
            import_delimited_bytes(
                b"x,y\n1,2\n3,4\n",
                "data.csv",
                b',',
                &ImportOptions::default(),
            )
            .unwrap(),
        );
        save_project(&path, &project).unwrap();
        let restored = load_project(&path).unwrap();
        assert_eq!(restored.manifest.name, "Study");
        assert_eq!(restored.datasets[0].batch, project.datasets[0].batch);
        assert!(!restored.read_only);
        assert_eq!(restored.source_archive_version, PROJECT_ARCHIVE_VERSION);
        assert!(!restored.migration_pending);
    }

    #[test]
    fn v5_round_trip_preserves_multiple_datasets_and_mixed_recipe_schemas() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixed-v5.qpls");
        let (dataset, current_recipe, current_result) = runner_generated_nca();
        let second_dataset = import_delimited_bytes(
            b"group,value\n1,10\n2,20\n",
            "second.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut historical_recipe = current_recipe.clone();
        historical_recipe.id = Uuid::new_v4();
        historical_recipe.schema_version = 2;
        historical_recipe.metadata = current_recipe.effective_metadata().unwrap();
        historical_recipe.method_config = None;

        let mut project = Project::new("Mixed v5 archive");
        project.datasets = vec![dataset.clone(), second_dataset.clone()];
        project.models.push(current_recipe.model.clone());
        project.recipes = vec![current_recipe.clone(), historical_recipe.clone()];
        project.results.push(current_result.clone());
        project.layouts.insert(
            "workspace".into(),
            serde_json::json!({"selected_dataset_id": dataset.id}),
        );

        let persisted_manifest = save_project(&path, &project).unwrap();
        assert_eq!(persisted_manifest.schema_version, PROJECT_ARCHIVE_VERSION);
        let stored_manifest: ProjectManifest =
            serde_json::from_slice(&zip_entry_bytes(&path, "manifest.json")).unwrap();
        assert_eq!(stored_manifest.schema_version, PROJECT_ARCHIVE_VERSION);

        let restored = load_project(&path).unwrap();
        assert_eq!(restored.source_archive_version, PROJECT_ARCHIVE_VERSION);
        assert!(!restored.migration_pending);
        assert!(!restored.read_only);
        assert_eq!(restored.datasets.len(), 2);
        assert_eq!(restored.datasets[0].batch, dataset.batch);
        assert_eq!(restored.datasets[1].batch, second_dataset.batch);
        assert_eq!(restored.recipes, vec![current_recipe, historical_recipe]);
        assert_eq!(restored.results.len(), 1);
        assert_eq!(restored.results[0].id, current_result.id);
        assert_eq!(restored.results[0].provenance, current_result.provenance);
        assert!(analysis_results_scientifically_equivalent(
            &restored.results[0],
            &current_result
        ));
        assert_eq!(restored.layouts, project.layouts);
    }

    #[test]
    fn v4_migration_is_deterministic_and_preserves_historical_result_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-v4.qpls");
        let (dataset, recipe, current_result) = runner_generated_nca();
        let historical_result = legacy_nca_v1_result(current_result);
        let mut project = Project::new("Legacy v4 scientific record");
        project.datasets.push(dataset.clone());
        project.recipes.push(recipe);
        project.results.push(historical_result.clone());
        save_project(&path, &project).unwrap();
        set_archive_schema_version(&path, PROJECT_ARCHIVE_VERSION_V4);

        let source_bytes = fs::read(&path).unwrap();
        let stored_project_json = zip_entry_bytes(&path, "project.json");
        let first = load_project(&path).unwrap();
        let second = load_project(&path).unwrap();

        assert_eq!(fs::read(&path).unwrap(), source_bytes);
        assert_eq!(zip_entry_bytes(&path, "project.json"), stored_project_json);
        assert_eq!(first.source_archive_version, PROJECT_ARCHIVE_VERSION_V4);
        assert_eq!(first.manifest.schema_version, PROJECT_ARCHIVE_VERSION);
        assert!(first.migration_pending);
        assert!(!first.read_only);
        assert_eq!(first.recipes, second.recipes);
        assert_eq!(first.models, second.models);
        assert_eq!(first.layouts, second.layouts);
        assert_eq!(first.results, second.results);
        assert_eq!(first.datasets.len(), second.datasets.len());
        assert_eq!(first.datasets[0].batch, second.datasets[0].batch);
        assert!(analysis_results_scientifically_equivalent(
            &first.results[0],
            &historical_result
        ));
        assert!(
            first.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "nca.legacy_method_version")
        );
        assert!(has_compatibility_notice(
            &first,
            first.results[0].id,
            "nca.legacy_method_version"
        ));
    }

    fn analysis_results_scientifically_equivalent(
        left: &AnalysisResult,
        right: &AnalysisResult,
    ) -> bool {
        left.schema_version == right.schema_version
            && left.id == right.id
            && left.status == right.status
            && left.provenance == right.provenance
            && left.diagnostics == right.diagnostics
            && json_values_close(
                &serde_json::to_value(&left.payload).unwrap(),
                &serde_json::to_value(&right.payload).unwrap(),
            )
    }

    fn json_values_close(left: &serde_json::Value, right: &serde_json::Value) -> bool {
        match (left, right) {
            (serde_json::Value::Number(left), serde_json::Value::Number(right)) => {
                match (left.as_f64(), right.as_f64()) {
                    (Some(left), Some(right)) => approximately_equal(left, right, 1e-14),
                    _ => left == right,
                }
            }
            (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| json_values_close(left, right))
            }
            (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                left.len() == right.len()
                    && left.iter().all(|(key, left)| {
                        right
                            .get(key)
                            .is_some_and(|right| json_values_close(left, right))
                    })
            }
            _ => left == right,
        }
    }

    #[test]
    fn autosave_does_not_consume_pending_v4_backup_and_explicit_save_does() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pending-v4.qpls");
        save_project(&path, &Project::new("Pending v4 migration")).unwrap();
        set_archive_schema_version(&path, PROJECT_ARCHIVE_VERSION_V4);
        let original_v4 = fs::read(&path).unwrap();
        let mut migrated = load_project(&path).unwrap();
        assert!(migrated.migration_pending);

        save_autosave(&path, &migrated).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original_v4);
        assert!(!backup_path(&path).exists());
        assert!(migrated.migration_pending);
        assert_eq!(
            load_project(&autosave_path(&path))
                .unwrap()
                .source_archive_version,
            PROJECT_ARCHIVE_VERSION
        );

        let persisted_manifest = save_project(&path, &migrated).unwrap();
        assert_eq!(fs::read(backup_path(&path)).unwrap(), original_v4);
        assert_eq!(persisted_manifest.schema_version, PROJECT_ARCHIVE_VERSION);
        assert!(migrated.migration_pending);
        migrated.adopt_explicit_save(persisted_manifest).unwrap();
        assert!(!migrated.migration_pending);
        assert_eq!(migrated.source_archive_version, PROJECT_ARCHIVE_VERSION);
    }

    #[test]
    fn compatible_future_archive_uses_distinct_read_only_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future.qpls");
        save_project(&path, &Project::new("Future compatible project")).unwrap();
        let future_version = PROJECT_ARCHIVE_VERSION + 1;
        set_archive_schema_version(&path, future_version);
        let project_json = zip_entry_bytes(&path, "project.json");

        assert!(matches!(
            migrate_document(future_version, &project_json),
            Err(ProjectError::Invalid(message)) if message.contains("future-schema read-only loader")
        ));
        let restored = load_project(&path).unwrap();
        assert!(restored.read_only);
        assert!(!restored.migration_pending);
        assert_eq!(restored.source_archive_version, future_version);
        assert_eq!(restored.manifest.schema_version, future_version);
        assert_eq!(
            restored.future_unsupported,
            FutureUnsupportedCounts::default()
        );
        assert!(matches!(
            save_project(&directory.path().join("forbidden.qpls"), &restored),
            Err(ProjectError::ReadOnly)
        ));
    }

    #[test]
    fn future_archive_preserves_compatible_content_and_counts_unknown_items() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future-partial.qpls");
        let mut project = Project::new("Future partial project");
        project.models.push(ModelSpec {
            id: Uuid::new_v4(),
            name: "Compatible model".into(),
            constructs: vec![],
            paths: vec![],
            controls: vec![],
            higher_order_constructs: vec![],
            interactions: vec![],
        });
        save_project(&path, &project).unwrap();
        rewrite_zip_entry_with_manifest_checksum(&path, "project.json", |bytes| {
            let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            document["models"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "id": Uuid::new_v4(),
                    "name": "Unknown future model",
                    "future_construct_contract": true
                }));
            document["recipes"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "schema_version": ANALYSIS_RECIPE_SCHEMA_VERSION + 1,
                    "id": Uuid::new_v4(),
                    "future_method": "unknown"
                }));
            document["results"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "schema_version": RESULT_SCHEMA_VERSION + 1,
                    "id": Uuid::new_v4(),
                    "future_payload": true
                }));
            serde_json::to_vec_pretty(&document).unwrap()
        });
        set_archive_schema_version(&path, PROJECT_ARCHIVE_VERSION + 1);

        let restored = load_project(&path).unwrap();
        assert!(restored.read_only);
        assert_eq!(restored.models.len(), 1);
        assert_eq!(
            restored.future_unsupported,
            FutureUnsupportedCounts {
                models: 1,
                recipes: 1,
                results: 1,
            }
        );
    }

    #[test]
    fn append_requires_v3_but_preserves_the_original_typed_recipe_metadata() {
        let (_, recipe, result) = runner_generated_nca();
        let original_metadata = recipe.metadata.clone();
        assert!(!original_metadata.contains_key("nca_x"));

        let mut project = Project::new("Typed recipe append");
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        assert_eq!(project.recipes[0], recipe);
        assert_eq!(project.recipes[0].metadata, original_metadata);
        assert_eq!(
            project.recipes[0].effective_metadata().unwrap()["nca_x"],
            "x"
        );

        let mut historical = recipe;
        historical.schema_version = 2;
        historical.method_config = None;
        assert!(matches!(
            Project::new("Historical append").append_validated_result(historical, result),
            Err(ProjectError::Invalid(message))
                if message.contains("archive-readable") && message.contains("migrate")
        ));
    }

    #[test]
    fn append_rejects_archive_only_legacy_payload_atomically() {
        let (_, recipe, mut result) = runner_generated_nca();
        result.payload = AnalysisPayload::Legacy {
            value: serde_json::json!({"forged": true}),
        };
        let mut project = Project::new("Legacy append rejection");
        assert!(matches!(
            project.append_validated_result(recipe, result),
            Err(ProjectError::Invalid(message))
                if message.contains("archive-readable only")
        ));
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());
    }

    #[test]
    fn historical_v1_and_v2_recipes_with_results_remain_archive_readable() {
        let directory = tempfile::tempdir().unwrap();
        let (dataset, recipe, result) = runner_generated_nca();
        let legacy_metadata = recipe.effective_metadata().unwrap();

        for schema_version in [1, 2] {
            let path = directory
                .path()
                .join(format!("historical-recipe-v{schema_version}.qpls"));
            let mut historical = recipe.clone();
            historical.schema_version = schema_version;
            historical.method_config = None;
            historical.metadata = legacy_metadata.clone();

            let mut project = Project::new(format!("Historical recipe v{schema_version}"));
            project.datasets.push(dataset.clone());
            project.recipes.push(historical.clone());
            project.results.push(result.clone());
            save_project(&path, &project).unwrap();

            let reopened = load_project(&path).unwrap();
            assert_eq!(reopened.recipes, vec![historical]);
            assert_eq!(reopened.results[0].id, result.id);
        }
    }

    #[test]
    fn historical_cbsem_and_ols_ignore_status_annotations_but_reject_scientific_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let fixtures = vec![
            ("cbsem", runner_generated_cbsem("sem")),
            ("ols", runner_generated_ols()),
        ];

        for (method_label, (dataset, recipe, result)) in fixtures {
            let executable_metadata = recipe.effective_metadata().unwrap();
            for (schema_version, status) in
                [(1, None), (2, Some("arbitrary_historical_annotation"))]
            {
                let status_label = status.unwrap_or("missing");
                let path = directory.path().join(format!(
                    "historical-{method_label}-v{schema_version}-{status_label}.qpls"
                ));
                let mut historical = recipe.clone();
                historical.schema_version = schema_version;
                historical.method_config = None;
                historical.metadata = executable_metadata.clone();
                if let Some(status) = status {
                    historical.metadata.insert("status".into(), status.into());
                } else {
                    historical.metadata.remove("status");
                }

                let mut project = Project::new(format!(
                    "Historical {method_label} v{schema_version} {status_label}"
                ));
                project.datasets.push(dataset.clone());
                project.recipes.push(historical.clone());
                project.results.push(result.clone());
                save_project(&path, &project).unwrap();

                let reopened = load_project(&path).unwrap();
                assert_eq!(reopened.recipes, vec![historical.clone()]);
                assert_eq!(reopened.results.len(), 1);
                assert_eq!(reopened.results[0].id, result.id);
                assert_eq!(reopened.results[0].status, result.status);
                assert_eq!(reopened.results[0].provenance, result.provenance);
                let reopened_estimation = estimation_payload(&reopened.results[0]);
                let original_estimation = estimation_payload(&result);
                assert_eq!(
                    reopened_estimation["method_version"],
                    original_estimation["method_version"]
                );
                if method_label == "cbsem" {
                    assert_eq!(
                        reopened_estimation["cbsem"]["method_version"],
                        original_estimation["cbsem"]["method_version"]
                    );
                } else {
                    assert_eq!(
                        reopened_estimation["regression"]["method_version"],
                        original_estimation["regression"]["method_version"]
                    );
                }
                assert_eq!(
                    reopened.recipes[0]
                        .metadata
                        .get("status")
                        .map(String::as_str),
                    status
                );

                let mut tampered_version = result.clone();
                tampered_version.provenance.method_version = "tampered_method_version".into();
                assert!(
                    validate_result_contracts_with_recipes(
                        std::slice::from_ref(&tampered_version),
                        std::slice::from_ref(&historical),
                    )
                    .is_err()
                );

                let mut tampered_payload = result.clone();
                if method_label == "cbsem" {
                    estimation_payload_mut(&mut tampered_payload)["cbsem"]["fit"]["srmr"] =
                        serde_json::json!(999.0);
                } else {
                    estimation_payload_mut(&mut tampered_payload)["regression"]["coefficients"]
                        [0]["statistic"] = serde_json::json!(999.0);
                }
                assert!(
                    validate_result_contracts_with_recipes(
                        std::slice::from_ref(&tampered_payload),
                        std::slice::from_ref(&historical),
                    )
                    .is_err()
                );
            }
        }
    }

    #[test]
    fn project_save_rejects_an_invalid_new_v3_recipe_without_a_result() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invalid-v3.qpls");
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 99;
        recipe.settings.permutation_samples = 99;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsPermutation);

        let mut project = Project::new("Invalid v3 recipe");
        project.datasets.push(dataset);
        project.recipes.push(recipe.clone());
        let error = save_project(&path, &project).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("method_config.resampling_mismatch")
        );
        assert_eq!(project.recipes, vec![recipe]);
        assert!(!path.exists());
    }

    #[test]
    fn duplicate_recipe_and_result_ids_are_rejected_without_partial_append() {
        let (_, recipe, result) = runner_generated_nca();
        let mut project = Project::new("Unique analysis IDs");
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();

        let mut duplicate_recipe_result = result.clone();
        duplicate_recipe_result.id = Uuid::new_v4();
        assert!(matches!(
            project.append_validated_result(recipe.clone(), duplicate_recipe_result),
            Err(ProjectError::Invalid(message)) if message.contains("recipe IDs must be unique")
        ));
        assert_eq!(project.recipes.len(), 1);
        assert_eq!(project.results.len(), 1);

        let mut distinct_recipe = recipe.clone();
        distinct_recipe.id = Uuid::new_v4();
        let mut duplicate_result = result.clone();
        duplicate_result.provenance.recipe_id = distinct_recipe.id;
        assert!(matches!(
            project.append_validated_result(distinct_recipe, duplicate_result),
            Err(ProjectError::Invalid(message)) if message.contains("result IDs must be unique")
        ));
        assert_eq!(project.recipes.len(), 1);
        assert_eq!(project.results.len(), 1);
    }

    #[test]
    fn project_validation_rejects_preexisting_duplicate_analysis_ids() {
        let directory = tempfile::tempdir().unwrap();
        let (_, recipe, result) = runner_generated_nca();

        let mut duplicate_recipes = Project::new("Duplicate recipes");
        duplicate_recipes.recipes = vec![recipe.clone(), recipe.clone()];
        assert!(matches!(
            save_project(&directory.path().join("duplicate-recipes.qpls"), &duplicate_recipes),
            Err(ProjectError::Invalid(message)) if message.contains("recipe IDs must be unique")
        ));

        let mut duplicate_results = Project::new("Duplicate results");
        duplicate_results.recipes.push(recipe);
        duplicate_results.results = vec![result.clone(), result];
        assert!(matches!(
            save_project(&directory.path().join("duplicate-results.qpls"), &duplicate_results),
            Err(ProjectError::Invalid(message)) if message.contains("result IDs must be unique")
        ));
    }

    #[test]
    fn legacy_migration_rejects_duplicate_analysis_ids() {
        let (_, recipe, result) = runner_generated_nca();
        let duplicate_v2 = serde_json::json!({
            "datasets": [],
            "models": [],
            "recipes": [recipe.clone(), recipe.clone()],
            "layouts": {},
            "results": []
        });
        assert!(matches!(
            migrate_document(2, &serde_json::to_vec(&duplicate_v2).unwrap()),
            Err(ProjectError::Invalid(message)) if message.contains("recipe IDs must be unique")
        ));

        let legacy_result = serde_json::json!({
            "schema_version": result.schema_version,
            "id": result.id,
            "status": result.status,
            "provenance": {
                "recipe_id": result.provenance.recipe_id,
                "dataset_fingerprint": result.provenance.dataset_fingerprint,
                "method": "nca",
                "method_version": result.provenance.method_version,
                "engine_version": result.provenance.engine_version,
                "seed": result.provenance.seed,
                "settings": result.provenance.settings,
                "started_at": result.provenance.started_at,
                "completed_at": result.provenance.completed_at
            },
            "diagnostics": result.diagnostics,
            "payload": { "legacy": true }
        });
        let duplicate_v3 = serde_json::json!({
            "datasets": [],
            "models": [],
            "recipes": [recipe],
            "layouts": {},
            "results": [legacy_result.clone(), legacy_result]
        });
        assert!(matches!(
            migrate_document(3, &serde_json::to_vec(&duplicate_v3).unwrap()),
            Err(ProjectError::Invalid(message)) if message.contains("result IDs must be unique")
        ));
    }

    #[test]
    fn truncated_archive_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bad.qpls");
        fs::write(&path, b"not a zip").unwrap();
        assert!(load_project(&path).is_err());
    }
    #[test]
    fn previous_generation_recovers_a_corrupt_primary_archive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let project = Project::new("First");
        save_project(&path, &project).unwrap();
        let mut replacement = project.clone();
        replacement.manifest.name = "Second".into();
        save_project(&path, &replacement).unwrap();
        fs::write(&path, b"interrupted write").unwrap();
        let (recovered, used_backup) = load_project_with_recovery(&path).unwrap();
        assert!(used_backup);
        assert_eq!(recovered.manifest.name, "First");
    }

    #[test]
    fn interrupted_save_distinguishes_generations_with_the_same_project_id() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("same-id.qpls");
        let old_project = Project::new("Old generation");
        save_project(&path, &old_project).unwrap();
        let old_bytes = fs::read(&path).unwrap();
        let old_sha256 = sha256_file(&path).unwrap();

        let mut new_project = old_project.clone();
        new_project.manifest.name = "New generation".into();
        let new_source = directory.path().join("new-source.qpls");
        save_project(&new_source, &new_project).unwrap();
        let new_bytes = fs::read(&new_source).unwrap();
        let new_sha256 = sha256_file(&new_source).unwrap();
        assert_eq!(
            old_project.manifest.project_id,
            new_project.manifest.project_id
        );
        assert_ne!(old_sha256, new_sha256);

        let temporary = path.with_extension("qpls.tmp-interrupted-test");
        let rotation = transaction_rotation_path(&path);
        fs::write(&temporary, &new_bytes).unwrap();
        fs::write(&rotation, &old_bytes).unwrap();
        let transaction = SaveTransactionJournal {
            schema_version: 2,
            primary: path.file_name().unwrap().to_str().unwrap().into(),
            rotation: rotation.file_name().unwrap().to_str().unwrap().into(),
            temporary: temporary.file_name().unwrap().to_str().unwrap().into(),
            backup: backup_path(&path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            new_project_id: new_project.manifest.project_id,
            previous_project_id: old_project.manifest.project_id,
            new_archive_sha256: new_sha256,
            previous_archive_sha256: old_sha256,
        };
        write_transaction_journal(&transaction_journal_path(&path), &transaction).unwrap();

        recover_incomplete_save(&path).unwrap();
        assert_eq!(load_project(&path).unwrap().manifest.name, "Old generation");
        assert!(!temporary.exists());
        assert!(!transaction_journal_path(&path).exists());
    }

    #[test]
    fn interrupted_promoted_generation_is_kept_and_previous_generation_becomes_backup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("promoted.qpls");
        let old_project = Project::new("Old generation");
        save_project(&path, &old_project).unwrap();
        let old_bytes = fs::read(&path).unwrap();
        let old_sha256 = sha256_file(&path).unwrap();

        let mut new_project = old_project.clone();
        new_project.manifest.name = "New generation".into();
        let new_source = directory.path().join("new-promoted.qpls");
        save_project(&new_source, &new_project).unwrap();
        let new_bytes = fs::read(&new_source).unwrap();
        let new_sha256 = sha256_file(&new_source).unwrap();
        let temporary = path.with_extension("qpls.tmp-promoted-test");
        let rotation = transaction_rotation_path(&path);
        fs::write(&rotation, &old_bytes).unwrap();
        fs::write(&path, &new_bytes).unwrap();
        let transaction = SaveTransactionJournal {
            schema_version: 2,
            primary: path.file_name().unwrap().to_str().unwrap().into(),
            rotation: rotation.file_name().unwrap().to_str().unwrap().into(),
            temporary: temporary.file_name().unwrap().to_str().unwrap().into(),
            backup: backup_path(&path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            new_project_id: new_project.manifest.project_id,
            previous_project_id: old_project.manifest.project_id,
            new_archive_sha256: new_sha256,
            previous_archive_sha256: old_sha256,
        };
        write_transaction_journal(&transaction_journal_path(&path), &transaction).unwrap();

        recover_incomplete_save(&path).unwrap();
        assert_eq!(load_project(&path).unwrap().manifest.name, "New generation");
        assert_eq!(
            load_project(&backup_path(&path)).unwrap().manifest.name,
            "Old generation"
        );
        assert!(!transaction_journal_path(&path).exists());
    }

    #[test]
    fn malformed_transaction_journal_cannot_block_a_verified_primary() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("malformed-journal.qpls");
        save_project(&path, &Project::new("Verified primary")).unwrap();
        fs::write(transaction_journal_path(&path), b"{partial").unwrap();
        let (project, used_backup) = load_project_with_recovery(&path).unwrap();
        assert!(!used_backup);
        assert_eq!(project.manifest.name, "Verified primary");
        assert!(!transaction_journal_path(&path).exists());
    }

    #[test]
    fn autosave_recovery_files_are_isolated_and_backup_retention_is_bounded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bounded.qpls");
        let project = Project::new("Primary");
        save_project(&path, &project).unwrap();
        let primary_backup = backup_path(&path);
        assert!(!primary_backup.exists());

        let mut autosaved = project.clone();
        for generation in 0..4 {
            autosaved.manifest.name = format!("Autosave {generation}");
            save_autosave(&path, &autosaved).unwrap();
        }
        let autosave = autosave_path(&path);
        assert!(autosave.exists());
        assert!(backup_path(&autosave).exists());
        assert_ne!(backup_path(&autosave), primary_backup);
        assert!(!primary_backup.exists());
        assert!(!transaction_displaced_backup_path(&autosave).exists());
        assert!(!transaction_journal_path(&autosave).exists());

        let autosave_prefix = autosave.file_name().unwrap().to_string_lossy().into_owned();
        let retained = fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&autosave_prefix)
            })
            .count();
        assert!(
            retained <= 3,
            "autosave retained {retained} archive artifacts"
        );

        discard_autosave(&path).unwrap();
        assert!(!autosave.exists());
        assert!(!backup_path(&autosave).exists());
        assert!(!identity_sidecar_path(&autosave).exists());
    }
    #[test]
    fn valid_autosave_takes_precedence_and_can_be_discarded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let primary = Project::new("Primary");
        save_project(&path, &primary).unwrap();
        let mut autosaved = primary.clone();
        autosaved.manifest.name = "Recovered work".into();
        save_autosave(&path, &autosaved).unwrap();
        let (restored, source) = load_project_with_autosave(&path).unwrap();
        assert_eq!(restored.manifest.name, "Recovered work");
        assert_eq!(source, Some(RecoverySource::Autosave));
        discard_autosave(&path).unwrap();
        assert!(!autosave_path(&path).exists());
    }
    #[test]
    fn stale_autosave_does_not_replace_a_newer_explicit_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let initial = Project::new("Initial");
        save_project(&path, &initial).unwrap();
        let mut stale = initial.clone();
        stale.manifest.name = "Stale autosave".into();
        save_autosave(&path, &stale).unwrap();
        let mut explicit = initial;
        explicit.manifest.name = "Explicit save".into();
        save_project(&path, &explicit).unwrap();
        let (restored, source) = load_project_with_autosave(&path).unwrap();
        assert_eq!(restored.manifest.name, "Explicit save");
        assert_eq!(source, None);
    }

    #[test]
    fn foreign_autosave_never_replaces_a_valid_primary() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let primary = Project::new("Primary identity");
        save_project(&path, &primary).unwrap();
        save_autosave(&path, &Project::new("Foreign autosave")).unwrap();

        let (restored, source) = load_project_with_autosave(&path).unwrap();
        assert_eq!(restored.manifest.project_id, primary.manifest.project_id);
        assert_eq!(restored.manifest.name, "Primary identity");
        assert_eq!(source, None);
    }

    #[test]
    fn future_primary_never_falls_back_to_a_writable_autosave() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future.qpls");
        let project = Project::new("Future primary");
        save_project(&path, &project).unwrap();
        let mut autosaved = project.clone();
        autosaved.manifest.name = "Writable autosave".into();
        save_autosave(&path, &autosaved).unwrap();
        set_archive_schema_version(&path, PROJECT_ARCHIVE_VERSION + 1);

        let (restored, source) = load_project_with_autosave(&path).unwrap();
        assert!(restored.read_only);
        assert_eq!(restored.manifest.name, "Future primary");
        assert_eq!(source, None);
    }

    #[test]
    fn v5_requires_an_explicit_supported_checksum_algorithm() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing-algorithm.qpls");
        save_project(&missing, &Project::new("Missing algorithm")).unwrap();
        rewrite_zip_entry(&missing, "manifest.json", |bytes| {
            let mut manifest: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            manifest
                .as_object_mut()
                .unwrap()
                .remove("checksum_algorithm");
            serde_json::to_vec_pretty(&manifest).unwrap()
        });
        assert!(matches!(
            load_project(&missing),
            Err(ProjectError::Invalid(message)) if message.contains("must declare checksum_algorithm")
        ));

        let unsupported = directory.path().join("unsupported-algorithm.qpls");
        save_project(&unsupported, &Project::new("Unsupported algorithm")).unwrap();
        rewrite_zip_entry(&unsupported, "manifest.json", |bytes| {
            let mut manifest: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            manifest["checksum_algorithm"] = serde_json::json!("sha512");
            serde_json::to_vec_pretty(&manifest).unwrap()
        });
        assert!(matches!(
            load_project(&unsupported),
            Err(ProjectError::Invalid(message)) if message.contains("unsupported archive checksum algorithm")
        ));
    }
    #[test]
    fn version_one_archive_migrates_to_the_current_schema() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.qpls");
        save_project(&path, &Project::new("Legacy")).unwrap();
        rewrite_zip_entry(&path, "manifest.json", |bytes| {
            let mut manifest: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            manifest["schema_version"] = serde_json::json!(1);
            serde_json::to_vec_pretty(&manifest).unwrap()
        });
        let migrated = load_project(&path).unwrap();
        assert_eq!(migrated.manifest.schema_version, PROJECT_ARCHIVE_VERSION);
        assert!(!migrated.read_only);
    }
    #[test]
    fn legacy_raw_results_receive_a_typed_envelope_and_migration_warning() {
        let legacy = serde_json::json!({
            "datasets": [],
            "models": [],
            "recipes": [],
            "layouts": {},
            "results": [{ "method_version": "pls_pm_v0", "paths": [] }]
        });
        let migrated = migrate_document(2, &serde_json::to_vec(&legacy).unwrap()).unwrap();
        let result = &migrated.results[0];
        assert_eq!(result.schema_version, RESULT_SCHEMA_VERSION);
        assert_eq!(result.provenance.method_version, "pls_pm_v0");
        assert_eq!(result.provenance.recipe_id, Uuid::nil());
        assert_eq!(
            result.payload,
            AnalysisPayload::Legacy {
                value: serde_json::json!({ "method_version": "pls_pm_v0", "paths": [] })
            }
        );
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "migration.legacy_result"
                && diagnostic.level == DiagnosticLevel::Warning
        }));
        validate_result_contracts(std::slice::from_ref(result)).unwrap();
    }
    #[test]
    fn version_three_pls_payload_migrates_to_the_tagged_contract() {
        let now = Utc::now();
        let legacy = serde_json::json!({
            "datasets": [],
            "models": [],
            "recipes": [],
            "layouts": {},
            "results": [{
                "schema_version": 1,
                "id": Uuid::nil(),
                "status": "completed",
                "provenance": {
                    "recipe_id": Uuid::nil(),
                    "dataset_fingerprint": "v2:test",
                    "method": "pls_pm",
                    "method_version": "pls_pm_v1+pls_assessment_v1",
                    "engine_version": "0.3.0-alpha.1",
                    "seed": 42,
                    "settings": AnalysisSettings::default(),
                    "started_at": now,
                    "completed_at": now
                },
                "diagnostics": [],
                "payload": {
                    "estimation": { "paths": [] },
                    "assessment": { "construct_quality": [] }
                }
            }]
        });
        let migrated = migrate_document(3, &serde_json::to_vec(&legacy).unwrap()).unwrap();
        let result = &migrated.results[0];
        assert_eq!(result.provenance.method, AnalysisMethod::PlsPm);
        assert_eq!(
            result.payload,
            AnalysisPayload::PlsPmV1 {
                estimation: serde_json::json!({ "paths": [] }),
                assessment: serde_json::json!({ "construct_quality": [] })
            }
        );
    }
    #[test]
    fn malformed_current_pls_payload_is_rejected_before_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invalid-result.qpls");
        let recipe = AnalysisRecipe::new(
            b"fixture",
            ModelSpec {
                id: Uuid::nil(),
                name: "fixture".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            AnalysisSettings::default(),
        );
        let result = AnalysisResult::completed_pls(
            &recipe,
            "invalid",
            Utc::now(),
            serde_json::Value::Null,
            serde_json::Value::Null,
            Vec::new(),
        );
        let mut project = Project::new("Invalid result");
        project.results.push(result);
        assert!(matches!(
            save_project(&path, &project),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn plsc_and_wpls_payloads_round_trip_and_reject_contract_tampering() {
        for method in [AnalysisMethod::Plsc, AnalysisMethod::Wpls] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format!("{}.qpls", method.as_str()));
            let (dataset, recipe) = pls_family_fixture(method);
            let result = completed_pls_family_result(&dataset, &recipe);
            let expected_estimation_version =
                executable_pls_payload_method_version(method).unwrap();

            let mut project = Project::new(format!("{} persistence", method.as_str()));
            project.datasets.push(dataset);
            project.recipes.push(recipe);
            project.results.push(result);
            save_project(&path, &project).unwrap();

            let restored = load_project(&path).unwrap();
            assert_eq!(restored.results.len(), 1);
            assert_eq!(restored.results[0].provenance.method, method);
            let mut result = restored.results[0].clone();
            assert_eq!(
                estimation_payload_mut(&mut result)["method_version"].as_str(),
                Some(expected_estimation_version)
            );

            let mut mismatched_method = restored.results[0].clone();
            mismatched_method.provenance.method = AnalysisMethod::PlsPm;
            assert!(matches!(
                validate_result_contracts_with_recipes(&[mismatched_method], &restored.recipes),
                Err(ProjectError::Invalid(_))
            ));

            let mut mismatched_estimator = restored.results[0].clone();
            estimation_payload_mut(&mut mismatched_estimator)["method_version"] =
                serde_json::json!(PLS_METHOD_VERSION);
            assert!(matches!(
                validate_result_contracts_with_recipes(&[mismatched_estimator], &restored.recipes),
                Err(ProjectError::Invalid(_))
            ));

            let mut missing_envelope_version = restored.results[0].clone();
            missing_envelope_version.provenance.method_version = format!(
                "{PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            );
            assert!(matches!(
                validate_result_contracts_with_recipes(
                    &[missing_envelope_version],
                    &restored.recipes
                ),
                Err(ProjectError::Invalid(_))
            ));

            let mut missing_method_payload = restored.results[0].clone();
            let payload_key = if method == AnalysisMethod::Plsc {
                "plsc"
            } else {
                "wpls"
            };
            estimation_payload_mut(&mut missing_method_payload)[payload_key] =
                serde_json::Value::Null;
            assert!(matches!(
                validate_result_contracts_with_recipes(
                    &[missing_method_payload],
                    &restored.recipes
                ),
                Err(ProjectError::Invalid(_))
            ));

            let mut mismatched_fingerprint = restored.results[0].clone();
            mismatched_fingerprint.provenance.dataset_fingerprint = "v2:tampered".into();
            assert!(matches!(
                validate_result_contracts_with_recipes(
                    &[mismatched_fingerprint],
                    &restored.recipes
                ),
                Err(ProjectError::Invalid(_))
            ));

            let mut resampled = restored.results[0].clone();
            resampled.provenance.settings.bootstrap_samples = 99;
            let mut resampled_recipe = restored.recipes[0].clone();
            resampled_recipe.settings.bootstrap_samples = 99;
            assert!(matches!(
                validate_result_contracts_with_recipes(&[resampled], &[resampled_recipe]),
                Err(ProjectError::Invalid(_))
            ));

            if method == AnalysisMethod::Wpls {
                let mut mismatched_weight = restored.results[0].clone();
                estimation_payload_mut(&mut mismatched_weight)["wpls"]["case_weight_column"] =
                    serde_json::json!("other_weight");
                assert!(matches!(
                    validate_result_contracts_with_recipes(&[mismatched_weight], &restored.recipes),
                    Err(ProjectError::Invalid(_))
                ));
            }
        }
    }

    #[test]
    fn runner_generated_cca_appends_round_trips_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cca.qpls");
        let (dataset, recipe, result) = runner_generated_cca();
        assert_eq!(result.provenance.method, AnalysisMethod::Cca);
        assert_eq!(
            result.provenance.method_version,
            format!(
                "{PLS_METHOD_VERSION}+{CCA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            )
        );

        let mut project = Project::new("Runner CCA persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();

        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(reopened.results[0].provenance.method, AnalysisMethod::Cca);
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected CCA payload: {other:?}"),
        };
        assert_eq!(
            estimation["method_version"].as_str(),
            Some(CCA_METHOD_VERSION)
        );
        assert_eq!(
            estimation["cca"]["method_version"].as_str(),
            Some(CCA_METHOD_VERSION)
        );
        assert_eq!(
            estimation["cca"]["model"].as_str(),
            Some("recursive_standardized_composite_path_model_v1")
        );

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected CCA");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut missing_provenance_version = result.clone();
        missing_provenance_version.provenance.method_version = format!(
            "{PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
        );
        assert_rejected_atomically(missing_provenance_version, recipe.clone());

        let mut mismatched_nested_version = result.clone();
        estimation_payload_mut(&mut mismatched_nested_version)["cca"]["method_version"] =
            serde_json::json!("cca_composite_residual_v0");
        assert_rejected_atomically(mismatched_nested_version, recipe.clone());

        let mut mismatched_model = result.clone();
        estimation_payload_mut(&mut mismatched_model)["cca"]["model"] =
            serde_json::json!("different_model");
        assert_rejected_atomically(mismatched_model, recipe.clone());

        let mut unknown_identifier = result.clone();
        estimation_payload_mut(&mut unknown_identifier)["cca"]["correlations"][0]["left"] =
            serde_json::json!("unknown_construct");
        assert_rejected_atomically(unknown_identifier, recipe.clone());

        let mut duplicate_pair = result.clone();
        let first_pair =
            estimation_payload_mut(&mut duplicate_pair)["cca"]["correlations"][0].clone();
        estimation_payload_mut(&mut duplicate_pair)["cca"]["correlations"][1] = first_pair;
        assert_rejected_atomically(duplicate_pair, recipe.clone());

        let mut incoherent_residual = result.clone();
        estimation_payload_mut(&mut incoherent_residual)["cca"]["correlations"][0]["residual"] =
            serde_json::json!(0.5);
        assert_rejected_atomically(incoherent_residual, recipe.clone());

        let mut incoherent_absolute = result.clone();
        estimation_payload_mut(&mut incoherent_absolute)["cca"]["correlations"][0]["absolute_residual"] =
            serde_json::json!(0.5);
        assert_rejected_atomically(incoherent_absolute, recipe.clone());

        let mut incoherent_maximum = result.clone();
        estimation_payload_mut(&mut incoherent_maximum)["cca"]["max_absolute_residual"] =
            serde_json::json!(0.5);
        assert_rejected_atomically(incoherent_maximum, recipe.clone());

        let mut non_finite = result.clone();
        estimation_payload_mut(&mut non_finite)["cca"]["correlations"][0]["observed"] =
            serde_json::json!("NaN");
        assert_rejected_atomically(non_finite, recipe.clone());

        let mut unrelated_payload = result.clone();
        estimation_payload_mut(&mut unrelated_payload)["cta_pls"] = serde_json::json!({
            "method_version": "cta_pls_tetrad_v1",
            "covariance": "sample_covariance_of_preprocessed_indicators_v1",
            "estimates": [],
            "max_absolute_tetrad_by_construct": {},
            "warnings": []
        });
        assert_rejected_atomically(unrelated_payload, recipe.clone());

        let mut resampled = result;
        resampled.provenance.settings.bootstrap_samples = 999;
        let mut resampled_recipe = recipe;
        resampled_recipe.settings.bootstrap_samples = 999;
        assert_rejected_atomically(resampled, resampled_recipe);
    }

    #[test]
    fn runner_generated_cta_pls_appends_round_trips_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cta-pls.qpls");
        let (dataset, recipe, result) = runner_generated_cta_pls();
        assert_eq!(result.provenance.method, AnalysisMethod::CtaPls);
        assert_eq!(
            result.provenance.method_version,
            format!(
                "{PLS_METHOD_VERSION}+{CTA_PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            )
        );

        let mut project = Project::new("Runner CTA-PLS persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected CTA-PLS payload: {other:?}"),
        };
        assert_eq!(estimation["method_version"], CTA_PLS_METHOD_VERSION);
        assert_eq!(
            estimation["cta_pls"]["method_version"],
            CTA_PLS_METHOD_VERSION
        );
        assert!(estimation["cta_pls"]["estimates"].as_array().unwrap().len() >= 3);

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected CTA-PLS");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut version = result.clone();
        estimation_payload_mut(&mut version)["cta_pls"]["method_version"] =
            serde_json::json!("cta_pls_tetrad_v0");
        assert_rejected_atomically(version, recipe.clone());

        let mut pairing = result.clone();
        estimation_payload_mut(&mut pairing)["cta_pls"]["estimates"][0]["pairing"] =
            serde_json::json!("unknown_pairing");
        assert_rejected_atomically(pairing, recipe.clone());

        let mut duplicate = result.clone();
        let first = estimation_payload_mut(&mut duplicate)["cta_pls"]["estimates"][0].clone();
        estimation_payload_mut(&mut duplicate)["cta_pls"]["estimates"][1] = first;
        assert_rejected_atomically(duplicate, recipe.clone());

        let mut absolute = result.clone();
        estimation_payload_mut(&mut absolute)["cta_pls"]["estimates"][0]["absolute_tetrad"] =
            serde_json::json!(999.0);
        assert_rejected_atomically(absolute, recipe.clone());

        let mut maximum = result.clone();
        estimation_payload_mut(&mut maximum)["cta_pls"]["max_absolute_tetrad_by_construct"]["x"] =
            serde_json::json!(999.0);
        assert_rejected_atomically(maximum, recipe.clone());

        let mut warning = result.clone();
        estimation_payload_mut(&mut warning)["cta_pls"]["warnings"] = serde_json::json!([]);
        assert_rejected_atomically(warning, recipe.clone());

        let mut resampled = result;
        resampled.provenance.settings.permutation_samples = 999;
        let mut resampled_recipe = recipe;
        resampled_recipe.settings.permutation_samples = 999;
        assert_rejected_atomically(resampled, resampled_recipe);
    }

    #[test]
    fn runner_generated_gsca_als_v2_commits_saves_reopens_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("gsca-als-v2.qpls");
        let (dataset, recipe, result) = runner_generated_gsca();
        assert_eq!(result.provenance.method, AnalysisMethod::Gsca);
        assert_eq!(result.provenance.method_version, GSCA_METHOD_VERSION);
        let assessment = match &result.payload {
            AnalysisPayload::PlsPmV1 { assessment, .. } => assessment,
            other => panic!("runner returned unexpected GSCA payload: {other:?}"),
        };
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": GSCA_NOT_APPLICABLE_ASSESSMENT_VERSION,
                "warnings": [GSCA_NOT_APPLICABLE_ASSESSMENT_WARNING]
            })
        );

        let mut project = Project::new("Runner GSCA ALS v2 persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(
            reopened.results[0].provenance.method_version,
            GSCA_METHOD_VERSION
        );
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("reopened unexpected GSCA payload: {other:?}"),
        };
        assert_eq!(estimation["method_version"], GSCA_METHOD_VERSION);
        assert_eq!(estimation["gsca"]["method_version"], GSCA_METHOD_VERSION);
        assert_eq!(estimation["gsca"]["algorithm"], GSCA_ALGORITHM_VERSION);
        assert!(
            estimation["gsca"]["bootstrap_intervals"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected GSCA");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut nested_version = result.clone();
        estimation_payload_mut(&mut nested_version)["gsca"]["method_version"] =
            serde_json::json!(GSCA_METHOD_VERSION_V1);
        assert_rejected_atomically(nested_version, recipe.clone());

        let mut objective = result.clone();
        estimation_payload_mut(&mut objective)["gsca"]["objective"] = serde_json::json!(0.1);
        assert_rejected_atomically(objective, recipe.clone());

        let mut loading = result.clone();
        estimation_payload_mut(&mut loading)["gsca"]["loadings"][0]["loading"] =
            serde_json::json!(0.1);
        assert_rejected_atomically(loading, recipe.clone());

        let mut path_identity = result.clone();
        estimation_payload_mut(&mut path_identity)["gsca"]["paths"][0]["source"] =
            serde_json::json!("unknown");
        assert_rejected_atomically(path_identity, recipe.clone());

        let mut covariance_fit = result.clone();
        estimation_payload_mut(&mut covariance_fit)["gsca"]["gfi"] = serde_json::json!(0.5);
        assert_rejected_atomically(covariance_fit, recipe.clone());

        let mut unsupported_payload = result.clone();
        estimation_payload_mut(&mut unsupported_payload)["cca"] = serde_json::json!({
            "method_version": CCA_METHOD_VERSION,
            "model": "recursive_standardized_composite_path_model_v1",
            "correlations": [],
            "max_absolute_residual": 0.0,
            "warnings": []
        });
        assert_rejected_atomically(unsupported_payload, recipe.clone());

        let mut unsupported_settings = result.clone();
        unsupported_settings.provenance.settings.workers = 2;
        let mut unsupported_recipe = recipe.clone();
        unsupported_recipe.settings.workers = 2;
        assert_rejected_atomically(unsupported_settings, unsupported_recipe);

        let mut legacy = result;
        legacy.provenance.method_version = GSCA_METHOD_VERSION_V1.into();
        assert_rejected_atomically(legacy, recipe);

        let mut tampered_for_save = reopened;
        estimation_payload_mut(&mut tampered_for_save.results[0])["gsca"]["r_squared"]["y"] =
            serde_json::json!(0.123);
        assert!(matches!(
            save_project(
                &directory.path().join("tampered-gsca.qpls"),
                &tampered_for_save
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn runner_generated_cbsem_and_cfa_commit_save_reopen_and_reject_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cbsem-cfa.qpls");
        let (sem_dataset, sem_recipe, sem_result) = runner_generated_cbsem("sem");
        let (cfa_dataset, cfa_recipe, cfa_result) = runner_generated_cbsem("cfa");
        let expected_sem_provenance = format!(
            "{PLS_METHOD_VERSION}+{CBSEM_ML_METHOD_VERSION}+{CBSEM_FIT_METHOD_VERSION}+{CBSEM_MODIFICATION_INDICES_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
        );
        let expected_cfa_provenance = format!(
            "{PLS_METHOD_VERSION}+{CFA_ML_METHOD_VERSION}+{CBSEM_FIT_METHOD_VERSION}+{CBSEM_MODIFICATION_INDICES_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
        );
        assert_eq!(sem_result.provenance.method, AnalysisMethod::Cbsem);
        assert_eq!(
            sem_result.provenance.method_version,
            expected_sem_provenance
        );
        assert_eq!(cfa_result.provenance.method, AnalysisMethod::Cbsem);
        assert_eq!(
            cfa_result.provenance.method_version,
            expected_cfa_provenance
        );

        let mut project = Project::new("Runner CB-SEM/CFA persistence");
        project.datasets.push(sem_dataset);
        project.datasets.push(cfa_dataset);
        project
            .append_validated_result(sem_recipe.clone(), sem_result.clone())
            .unwrap();
        project
            .append_validated_result(cfa_recipe.clone(), cfa_result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();

        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 2);
        for (result, model_type, method_version, provenance) in [
            (
                &reopened.results[0],
                "sem",
                CBSEM_ML_METHOD_VERSION,
                expected_sem_provenance.as_str(),
            ),
            (
                &reopened.results[1],
                "cfa",
                CFA_ML_METHOD_VERSION,
                expected_cfa_provenance.as_str(),
            ),
        ] {
            assert_eq!(result.provenance.method_version, provenance);
            let estimation = match &result.payload {
                AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
                other => panic!("runner returned unexpected CB-SEM payload: {other:?}"),
            };
            assert_eq!(estimation["method_version"].as_str(), Some(method_version));
            assert_eq!(
                estimation["cbsem"]["method_version"].as_str(),
                Some(method_version)
            );
            assert_eq!(estimation["cbsem"]["model_type"].as_str(), Some(model_type));
            assert_eq!(
                estimation["cbsem"]["fit"]["method_version"].as_str(),
                Some(CBSEM_FIT_METHOD_VERSION)
            );
            assert!(
                estimation["cbsem"]["parameters"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
            );
            assert!(
                estimation["cbsem"]["modification_indices"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
            );
        }

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected CB-SEM");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut missing_fit_version = sem_result.clone();
        missing_fit_version.provenance.method_version = format!(
            "{PLS_METHOD_VERSION}+{CBSEM_ML_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
        );
        assert_rejected_atomically(missing_fit_version, sem_recipe.clone());

        let mut mismatched_nested_version = sem_result.clone();
        estimation_payload_mut(&mut mismatched_nested_version)["cbsem"]["method_version"] =
            serde_json::json!(CFA_ML_METHOD_VERSION);
        assert_rejected_atomically(mismatched_nested_version, sem_recipe.clone());

        let mut mismatched_model_type = sem_result.clone();
        estimation_payload_mut(&mut mismatched_model_type)["cbsem"]["model_type"] =
            serde_json::json!("cfa");
        assert_rejected_atomically(mismatched_model_type, sem_recipe.clone());

        let mut tampered_fit = sem_result.clone();
        estimation_payload_mut(&mut tampered_fit)["cbsem"]["fit"]["srmr"] = serde_json::json!(0.5);
        assert_rejected_atomically(tampered_fit, sem_recipe.clone());

        let mut tampered_parameter = sem_result.clone();
        estimation_payload_mut(&mut tampered_parameter)["cbsem"]["parameters"][1]["p_value_two_sided"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(tampered_parameter, sem_recipe.clone());

        let mut tampered_standardized = sem_result.clone();
        estimation_payload_mut(&mut tampered_standardized)["cbsem"]["standardized"][0]["name"] =
            serde_json::json!("unknown=~indicator");
        assert_rejected_atomically(tampered_standardized, sem_recipe.clone());

        let mut tampered_matrix = sem_result.clone();
        estimation_payload_mut(&mut tampered_matrix)["cbsem"]["residual_covariance"][0]["value"] =
            serde_json::json!(0.5);
        assert_rejected_atomically(tampered_matrix, sem_recipe.clone());

        let mut tampered_modification_index = sem_result.clone();
        estimation_payload_mut(&mut tampered_modification_index)["cbsem"]["modification_indices"]
            [0]["lhs"] = serde_json::json!("unknown_indicator");
        assert_rejected_atomically(tampered_modification_index, sem_recipe.clone());

        let mut unsupported_recipe = sem_recipe.clone();
        unsupported_recipe
            .metadata
            .insert("cbsem_bootstrap_samples".into(), "999".into());
        assert_rejected_atomically(sem_result.clone(), unsupported_recipe);

        let mut parallel_recipe = sem_recipe.clone();
        parallel_recipe.settings.workers = 2;
        let mut parallel_result = sem_result.clone();
        parallel_result.provenance.settings.workers = 2;
        assert_rejected_atomically(parallel_result, parallel_recipe);

        let mut unrelated_payload = sem_result;
        estimation_payload_mut(&mut unrelated_payload)["pca"] = serde_json::json!({
            "method_version": PCA_METHOD_VERSION,
            "component_rule": "fixed",
            "retained_components": 0,
            "observations": 0,
            "variables": [],
            "components": [],
            "loadings": [],
            "scores": [],
            "warnings": []
        });
        assert_rejected_atomically(unrelated_payload, sem_recipe);
    }

    #[test]
    fn runner_generated_ipma_commits_saves_reopens_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ipma.qpls");
        let (dataset, recipe, result) = runner_generated_ipma();
        assert_eq!(result.provenance.method, AnalysisMethod::Ipma);
        assert_eq!(
            result.provenance.method_version,
            format!(
                "{PLS_METHOD_VERSION}+{IPMA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            )
        );
        let generated_estimation: PlsResult = match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value(estimation.clone()).unwrap()
            }
            other => panic!("runner returned unexpected IPMA payload: {other:?}"),
        };
        assert_eq!(
            generated_estimation.mediation,
            analyze_mediation_effects_with_tolerance(&generated_estimation.effects, 1e-12)
        );
        let serialized_result = serde_json::to_vec(&result).unwrap();
        let serialized_result: AnalysisResult = serde_json::from_slice(&serialized_result).unwrap();
        let serialized_estimation: PlsResult = match &serialized_result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value(estimation.clone()).unwrap()
            }
            other => panic!("serialized unexpected IPMA payload: {other:?}"),
        };
        assert!(mediation_payload_matches(
            &serialized_estimation.mediation,
            &analyze_mediation_effects_with_tolerance(&serialized_estimation.effects, 1e-12)
        ));

        let mut project = Project::new("Runner IPMA persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();

        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(reopened.results[0].provenance.method, AnalysisMethod::Ipma);
        assert!(!reopened.recipes[0].metadata.contains_key("ipma_targets"));
        assert_eq!(
            reopened.recipes[0].effective_metadata().unwrap()["ipma_targets"],
            "y"
        );
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected IPMA payload: {other:?}"),
        };
        assert_eq!(
            estimation["method_version"].as_str(),
            Some(IPMA_METHOD_VERSION)
        );
        assert_eq!(
            estimation["ipma"]["method_version"].as_str(),
            Some(IPMA_METHOD_VERSION)
        );
        assert_eq!(
            estimation["ipma"]["performance_scale"].as_str(),
            Some(IPMA_PERFORMANCE_SCALE)
        );
        assert_eq!(estimation["ipma"]["targets"], serde_json::json!(["y"]));
        assert_eq!(
            estimation["ipma"]["constructs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["construct"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["x", "z", "m"]
        );
        assert!(
            estimation["ipma"]["constructs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["construct"] != "y")
        );

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected IPMA");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut missing_provenance_version = result.clone();
        missing_provenance_version.provenance.method_version = format!(
            "{PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
        );
        assert_rejected_atomically(missing_provenance_version, recipe.clone());

        let mut mismatched_nested_version = result.clone();
        estimation_payload_mut(&mut mismatched_nested_version)["ipma"]["method_version"] =
            serde_json::json!("ipma_v0");
        assert_rejected_atomically(mismatched_nested_version, recipe.clone());

        let mut mismatched_scale = result.clone();
        estimation_payload_mut(&mut mismatched_scale)["ipma"]["performance_scale"] =
            serde_json::json!("unstandardized_scale");
        assert_rejected_atomically(mismatched_scale, recipe.clone());

        let mut mismatched_target = result.clone();
        estimation_payload_mut(&mut mismatched_target)["ipma"]["targets"] =
            serde_json::json!(["x"]);
        assert_rejected_atomically(mismatched_target, recipe.clone());

        let mut mismatched_importance = result.clone();
        estimation_payload_mut(&mut mismatched_importance)["ipma"]["constructs"][0]["importance"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(mismatched_importance, recipe.clone());

        let mut mismatched_construct_performance = result.clone();
        estimation_payload_mut(&mut mismatched_construct_performance)["ipma"]["constructs"][0]["performance"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(mismatched_construct_performance, recipe.clone());

        let mut mismatched_loading = result.clone();
        estimation_payload_mut(&mut mismatched_loading)["ipma"]["indicators"][0]["loading"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(mismatched_loading, recipe.clone());

        let mut unrelated_payload = result.clone();
        estimation_payload_mut(&mut unrelated_payload)["cca"] = serde_json::json!({
            "method_version": CCA_METHOD_VERSION,
            "model": "recursive_standardized_composite_path_model_v1",
            "correlations": [],
            "max_absolute_residual": 0.0,
            "warnings": []
        });
        assert_rejected_atomically(unrelated_payload, recipe.clone());

        let mut exogenous_recipe = recipe.clone();
        exogenous_recipe
            .metadata
            .insert("ipma_targets".into(), "x".into());
        let mut exogenous_result = result.clone();
        exogenous_result.provenance.recipe_id = exogenous_recipe.id;
        assert_rejected_atomically(exogenous_result, exogenous_recipe);

        let mut unsupported_preprocessing = result.clone();
        unsupported_preprocessing.provenance.settings.preprocessing = Preprocessing::MeanCentered;
        let mut unsupported_recipe = recipe.clone();
        unsupported_recipe.settings.preprocessing = Preprocessing::MeanCentered;
        assert_rejected_atomically(unsupported_preprocessing, unsupported_recipe);

        let mut tampered_for_save = reopened.clone();
        estimation_payload_mut(&mut tampered_for_save.results[0])["ipma"]["constructs"][0]["score_mean"] =
            serde_json::json!(42.0);
        assert!(matches!(
            save_project(
                &directory.path().join("tampered-ipma.qpls"),
                &tampered_for_save
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn runner_generated_pca_v1_commits_saves_reopens_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pca-v1.qpls");
        let (dataset, recipe, result) = runner_generated_pca();
        assert_eq!(result.provenance.method, AnalysisMethod::Pca);
        assert_eq!(result.provenance.method_version, PCA_METHOD_VERSION);
        assert!(matches!(result.payload, AnalysisPayload::PlsPmV1 { .. }));

        let mut project = Project::new("Runner PCA v1 persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.models.len(), 0);
        assert_eq!(reopened.results.len(), 1);
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => {
                assert_eq!(
                    assessment,
                    &serde_json::json!({
                        "method_version": PCA_NOT_APPLICABLE_ASSESSMENT_VERSION,
                        "warnings": [PCA_NOT_APPLICABLE_ASSESSMENT_WARNING]
                    })
                );
                estimation
            }
            other => panic!("runner returned unexpected PCA payload: {other:?}"),
        };
        assert_eq!(estimation["method_version"], PCA_METHOD_VERSION);
        assert_eq!(estimation["pca"]["method_version"], PCA_METHOD_VERSION);
        assert_eq!(
            estimation["pca"]["variables"],
            serde_json::json!(["a", "b", "c", "d"])
        );
        assert!(estimation["pca"]["retained_components"].as_u64().unwrap() >= 1);

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected PCA");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut tampered_version = result.clone();
        estimation_payload_mut(&mut tampered_version)["pca"]["method_version"] =
            serde_json::json!("pca_v0");
        assert_rejected_atomically(tampered_version, recipe.clone());

        let mut tampered_component = result.clone();
        estimation_payload_mut(&mut tampered_component)["pca"]["components"][0]["explained_variance"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(tampered_component, recipe.clone());

        let mut tampered_loading = result.clone();
        estimation_payload_mut(&mut tampered_loading)["pca"]["loadings"][0]["loading"] =
            serde_json::json!(42.0);
        assert_rejected_atomically(tampered_loading, recipe.clone());

        let mut tampered_score = result.clone();
        estimation_payload_mut(&mut tampered_score)["pca"]["scores"][0]["component"] =
            serde_json::json!("PC99");
        assert_rejected_atomically(tampered_score, recipe.clone());

        let mut mismatched_recipe = recipe.clone();
        mismatched_recipe
            .metadata
            .insert("pca_variables".into(), "a,b,c".into());
        let mut mismatched_result = result.clone();
        mismatched_result.provenance.recipe_id = mismatched_recipe.id;
        assert_rejected_atomically(mismatched_result, mismatched_recipe);

        let mut tampered_for_save = reopened.clone();
        estimation_payload_mut(&mut tampered_for_save.results[0])["pca"]["loadings"][0]["weight"] =
            serde_json::json!(-0.5);
        assert!(matches!(
            save_project(
                &directory.path().join("tampered-pca.qpls"),
                &tampered_for_save
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn typed_logistic_and_process_results_round_trip_and_reject_family_tampering() {
        let directory = tempfile::tempdir().unwrap();
        for (label, generated, expected_version) in [
            (
                "logistic",
                runner_generated_logistic as fn() -> (Dataset, AnalysisRecipe, AnalysisResult),
                REGRESSION_LOGISTIC_METHOD_VERSION.to_owned(),
            ),
            (
                "process",
                runner_generated_process_v2 as fn() -> (Dataset, AnalysisRecipe, AnalysisResult),
                format!("{REGRESSION_PROCESS_METHOD_VERSION}+{PROCESS_BOOTSTRAP_METHOD_VERSION}"),
            ),
        ] {
            let (dataset, recipe, result) = generated();
            assert_eq!(recipe.schema_version, ANALYSIS_RECIPE_SCHEMA_VERSION);
            assert!(matches!(
                &recipe.method_config,
                Some(qpls_core::MethodConfig::Regression { .. })
            ));
            assert!(!recipe.metadata.contains_key("status"));
            assert_eq!(result.provenance.method_version, expected_version);
            let path = directory.path().join(format!("{label}.qpls"));
            let mut project = Project::new(format!("Typed {label} persistence"));
            project.datasets.push(dataset);
            project
                .append_validated_result(recipe.clone(), result.clone())
                .unwrap();
            save_project(&path, &project).unwrap();
            let reopened = load_project(&path).unwrap();
            assert_eq!(
                reopened.results[0].provenance.method_version,
                expected_version
            );

            let mut tampered = result;
            estimation_payload_mut(&mut tampered)["regression"]["method_version"] =
                serde_json::json!(REGRESSION_OLS_METHOD_VERSION);
            let mut rejected = Project::new(format!("Rejected {label}"));
            assert!(matches!(
                rejected.append_validated_result(recipe, tampered),
                Err(ProjectError::Invalid(_))
            ));
            assert!(rejected.recipes.is_empty());
            assert!(rejected.results.is_empty());
        }
    }

    #[test]
    fn logistic_v2_append_save_reopen_and_arithmetic_contract_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("logistic-v2.qpls");
        let (dataset, recipe, result) = runner_generated_logistic();
        assert_eq!(recipe.schema_version, ANALYSIS_RECIPE_SCHEMA_VERSION);
        assert!(matches!(
            &recipe.method_config,
            Some(qpls_core::MethodConfig::Regression {
                model: qpls_core::RegressionModelConfig::Logistic,
                ..
            })
        ));
        assert_eq!(
            result.provenance.method_version,
            REGRESSION_LOGISTIC_METHOD_VERSION
        );
        let regression = &estimation_payload(&result)["regression"];
        assert_eq!(
            regression["logistic"]["outcome_profile"]["readiness"],
            "ready"
        );
        assert_eq!(
            regression["logistic"]["convergence"]["algorithm"],
            "deterministic_newton_irls_v1"
        );
        assert_eq!(regression["fit"]["pseudo_r_squared_method"], "mcfadden_v1");

        let mut project = Project::new("Logistic v2 persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(reopened.results[0].id, result.id);
        assert_eq!(
            reopened.results[0].provenance.method_version,
            REGRESSION_LOGISTIC_METHOD_VERSION
        );
        let saved_archive = fs::read(&path).unwrap();

        let reject_atomically = |tampered: AnalysisResult| {
            let mut rejected = Project::new("Rejected logistic v2");
            assert!(matches!(
                rejected.append_validated_result(recipe.clone(), tampered),
                Err(ProjectError::Invalid(_))
            ));
            assert!(rejected.recipes.is_empty());
            assert!(rejected.results.is_empty());
        };
        let mut tampered_profile = result.clone();
        estimation_payload_mut(&mut tampered_profile)["regression"]["logistic"]["outcome_profile"]
            ["one_count"] = serde_json::json!(999);
        reject_atomically(tampered_profile);

        let mut tampered_convergence = result.clone();
        estimation_payload_mut(&mut tampered_convergence)["regression"]["logistic"]["convergence"]
            ["final_max_abs_step"] = serde_json::json!(0.25);
        reject_atomically(tampered_convergence);

        let mut tampered_classification = result.clone();
        estimation_payload_mut(&mut tampered_classification)["regression"]["logistic"]["classification"]
            ["true_positive"] = serde_json::json!(999);
        reject_atomically(tampered_classification);

        let mut tampered_odds_ratio_interval = result.clone();
        estimation_payload_mut(&mut tampered_odds_ratio_interval)["regression"]["coefficients"]
            [0]["odds_ratio_confidence_interval_lower"] = serde_json::json!(0.123);
        reject_atomically(tampered_odds_ratio_interval);

        let mut tampered_fit = result.clone();
        estimation_payload_mut(&mut tampered_fit)["regression"]["fit"]["null_log_likelihood"] =
            serde_json::json!(-0.123);
        reject_atomically(tampered_fit);

        let mut tampered_probability = result.clone();
        estimation_payload_mut(&mut tampered_probability)["regression"]["predictions"][0]["probability"] =
            serde_json::json!(0.75);
        reject_atomically(tampered_probability);

        let mut tampered_for_save = reopened;
        estimation_payload_mut(&mut tampered_for_save.results[0])["regression"]["logistic"]["classification"]
            ["accuracy"] = serde_json::json!(0.0);
        assert!(matches!(
            save_project(&path, &tampered_for_save),
            Err(ProjectError::Invalid(_))
        ));
        assert_eq!(fs::read(&path).unwrap(), saved_archive);
        assert_eq!(load_project(&path).unwrap().results[0].id, result.id);
    }

    #[test]
    fn logistic_v1_remains_archive_readable_but_cannot_be_appended() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("logistic-v1-archive.qpls");
        let (dataset, recipe, current) = runner_generated_logistic();
        let legacy = legacy_logistic_v1_result(current.clone());

        let mut rejected = Project::new("Rejected logistic v1 append");
        assert!(matches!(
            rejected.append_validated_result(recipe.clone(), legacy.clone()),
            Err(ProjectError::Invalid(message)) if message.contains("archive-readable")
        ));
        assert!(rejected.results.is_empty());

        let mut historical_recipe = recipe.clone();
        let legacy_metadata = historical_recipe.effective_metadata().unwrap();
        historical_recipe.schema_version = 2;
        historical_recipe.metadata = legacy_metadata;
        historical_recipe.method_config = None;
        let mut archive = Project::new("Historical logistic v1");
        archive.datasets.push(dataset);
        archive.recipes.push(historical_recipe);
        archive.results.push(legacy.clone());
        save_project(&path, &archive).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(reopened.results[0].id, legacy.id);
        assert_eq!(
            reopened.results[0].provenance.method_version,
            REGRESSION_LOGISTIC_METHOD_VERSION_V1
        );
        assert!(reopened.compatibility_notices.iter().any(|notice| {
            notice.diagnostic.code == "regression.logistic.legacy_method_version"
        }));

        let (_, current_recipe, current_result) = runner_generated_logistic();
        let typed_v3_with_v1 = legacy_logistic_v1_result(current_result);
        let mut invalid_archive = Project::new("Invalid typed v1 pairing");
        invalid_archive.recipes.push(current_recipe);
        invalid_archive.results.push(typed_v3_with_v1);
        assert!(matches!(
            save_project(
                &directory.path().join("invalid-typed-v1.qpls"),
                &invalid_archive
            ),
            Err(ProjectError::Invalid(_))
        ));

        let mut historical_recipe = recipe;
        let legacy_metadata = historical_recipe.effective_metadata().unwrap();
        historical_recipe.schema_version = 2;
        historical_recipe.metadata = legacy_metadata;
        historical_recipe.method_config = None;
        let mut invalid_archive = Project::new("Invalid legacy v2 pairing");
        invalid_archive.recipes.push(historical_recipe);
        invalid_archive.results.push(current);
        assert!(matches!(
            save_project(
                &directory.path().join("invalid-legacy-v2.qpls"),
                &invalid_archive
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn process_v1_remains_archive_only() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("process-v1-archive.qpls");
        let (dataset, recipe, result) = runner_generated_process();
        assert_eq!(
            result.provenance.method_version,
            REGRESSION_PROCESS_METHOD_VERSION_V1
        );
        let mut rejected = Project::new("Rejected PROCESS v1 append");
        assert!(matches!(
            rejected.append_validated_result(recipe.clone(), result.clone()),
            Err(ProjectError::Invalid(message)) if message.contains("archive-readable")
        ));
        assert!(rejected.results.is_empty());

        let mut historical_recipe = recipe;
        historical_recipe.metadata = historical_recipe.effective_metadata().unwrap();
        historical_recipe.schema_version = 2;
        historical_recipe.method_config = None;
        let historical_estimation: PlsResult =
            serde_json::from_value(estimation_payload(&result).clone()).unwrap();
        let historical_regression = historical_estimation.regression.as_ref().unwrap();
        assert!(validate_linear_regression_analysis_contract(
            historical_regression,
            REGRESSION_PROCESS_METHOD_VERSION_V1,
            "process",
            "y",
            &["x".into(), "m".into()],
            &[],
            140,
            0.95,
            true,
        ));
        assert!(validate_process_analysis_contract(
            historical_regression,
            &ProcessPersistenceContract::Mediation {
                x: "x".into(),
                mediator: "m".into(),
            },
            0,
        ));
        let effective_historical_recipe = historical_recipe.with_effective_metadata().unwrap();
        assert_eq!(
            effective_historical_recipe.settings,
            result.provenance.settings
        );
        assert_eq!(
            effective_historical_recipe.dataset_fingerprint,
            result.provenance.dataset_fingerprint
        );
        assert_eq!(
            historical_regression.warnings,
            historical_estimation.warnings
        );
        assert!(historical_regression.bootstrap.is_none());
        let assessment = match &result.payload {
            AnalysisPayload::PlsPmV1 { assessment, .. } => assessment,
            _ => unreachable!("the frozen PROCESS v1 fixture uses the v1 envelope"),
        };
        assert!(validate_regression_payload_contract(
            &result,
            &historical_estimation,
            Some(&effective_historical_recipe),
            assessment["method_version"].as_str().unwrap(),
        ));
        let mut archive = Project::new("Historical PROCESS v1");
        archive.datasets.push(dataset);
        archive.recipes.push(historical_recipe);
        archive.results.push(result.clone());
        save_project(&path, &archive).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results[0].id, result.id);
        assert_eq!(
            reopened.results[0].provenance.method_version,
            REGRESSION_PROCESS_METHOD_VERSION_V1
        );
        assert!(reopened.compatibility_notices.iter().any(|notice| {
            notice.diagnostic.code == "regression.process.legacy_method_version"
        }));
    }

    #[test]
    fn regression_bootstrap_json_roundtrip_tolerance_is_narrow() {
        let near = 1.0 + 32.0 * f64::EPSILON;
        assert!(regression_bootstrap_json_roundtrip_close(1.0, near));
        assert!(!regression_bootstrap_json_roundtrip_close(
            1.0,
            1.0 + 1.0e-10
        ));
        assert!(!regression_bootstrap_json_roundtrip_close(
            f64::NAN,
            f64::NAN
        ));
    }

    #[test]
    fn process_graph_v2_unavailable_bootstrap_roundtrip_and_tamper_contract() {
        let ids = vec!["direct:X->Y".to_string()];
        let bootstrap_rows = (0..99)
            .map(
                |replicate_index| qpls_estimation::ProcessBootstrapWitnessBootstrapRow {
                    replicate_index,
                    estimates: vec![0.5],
                },
            )
            .collect::<Vec<_>>();
        let jackknife_rows = (0..8)
            .map(
                |omitted_case| qpls_estimation::ProcessBootstrapWitnessJackknifeRow {
                    omitted_case,
                    estimates: vec![0.5],
                },
            )
            .collect::<Vec<_>>();
        let bootstrap_values = bootstrap_rows
            .iter()
            .map(|row| row.estimates.clone())
            .collect::<Vec<_>>();
        let jackknife_values = jackknife_rows
            .iter()
            .map(|row| row.estimates.clone())
            .collect::<Vec<_>>();
        let estimands = summarize_process_bootstrap_estimands(
            &ids,
            &[0.5],
            &bootstrap_values,
            &jackknife_values,
            8,
            0.95,
        )
        .unwrap();
        let mut bootstrap = ProcessBootstrapAnalysis {
            method_version: PROCESS_BOOTSTRAP_METHOD_VERSION.into(),
            algorithm: PROCESS_BOOTSTRAP_ALGORITHM.into(),
            interval_policy: PROCESS_BOOTSTRAP_INTERVAL_POLICY.into(),
            test_reference: PROCESS_BOOTSTRAP_TEST_REFERENCE.into(),
            requested_replicates: 99,
            usable_replicates: 99,
            minimum_usable_fraction: REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION,
            jackknife_cases: 8,
            usable_jackknife_cases: 8,
            seed: 91,
            workers: 2,
            stream_token: PROCESS_BOOTSTRAP_STREAM_TOKEN.into(),
            failed_replicates: Vec::new(),
            estimands,
            validation_witness: qpls_estimation::ProcessBootstrapValidationWitness {
                method_version: PROCESS_BOOTSTRAP_VALIDATION_WITNESS_VERSION.into(),
                estimand_ids: ids,
                successful_bootstrap: bootstrap_rows,
                successful_jackknife: jackknife_rows,
                failed_jackknife: Vec::new(),
            },
            warnings: vec![
                "PROCESS bootstrap v1 uses deterministic indexed complete-case resampling with replacement; percentile intervals are primary and BCa intervals require every delete-one fit.".into(),
                "PROCESS bootstrap ratio tests use the original effect divided by its bootstrap standard error with a fixed two-sided standard-normal reference.".into(),
            ],
        };
        let graph = ProcessGraphAnalysis {
            policies: qpls_estimation::ProcessPolicies {
                centering: "equation_complete_case_mean_v1".into(),
                covariance: "hc3_v1".into(),
                inference_reference: "student_t_residual_df_v1".into(),
                confidence_level: 0.95,
            },
            complete_cases: 8,
            omitted_cases: 0,
            variable_profiles: Vec::new(),
            paths: Vec::new(),
            moderations: Vec::new(),
            equations: Vec::new(),
            reference_effects: vec![qpls_estimation::ProcessReferenceEffect {
                effect_id: "direct:X->Y".into(),
                kind: "direct".into(),
                path: vec!["X".into(), "Y".into()],
                estimate: 0.5,
            }],
            conditional_indirect_effects: Vec::new(),
            moderated_mediation_indices: Vec::new(),
            simple_slopes: Vec::new(),
            plots: Vec::new(),
            johnson_neyman: Vec::new(),
            bootstrap: None,
        };
        let mut settings = AnalysisSettings::default();
        settings.bootstrap_samples = 99;
        settings.seed = 91;
        settings.workers = 2;
        let encoded = serde_json::to_vec(&bootstrap).unwrap();
        let decoded: ProcessBootstrapAnalysis = serde_json::from_slice(&encoded).unwrap();
        assert!(validate_process_bootstrap_contract(
            &decoded, &graph, &settings
        ));
        assert!(matches!(
            &decoded.estimands[0].test,
            RegressionBootstrapTest::Unavailable { reason_code, .. }
                if reason_code == "zero_bootstrap_standard_error"
        ));
        bootstrap.estimands[0].bias = 10.0;
        assert!(!validate_process_bootstrap_contract(
            &bootstrap, &graph, &settings
        ));
        let RegressionBootstrapTest::Unavailable { reason_code, .. } =
            &mut bootstrap.estimands[0].test
        else {
            panic!("fixture test must be unavailable")
        };
        *reason_code = "degenerate_bootstrap_standard_error".into();
        assert!(!validate_process_bootstrap_contract(
            &bootstrap, &graph, &settings
        ));
    }

    #[test]
    fn direct_only_pls_populated_mediation_survives_runner_project_roundtrip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("direct-only-pls.qpls");
        let (dataset, recipe, result) = runner_generated_direct_only_pls();
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_MEDIATION_METHOD_VERSION)
        );
        let estimation = estimation_payload(&result);
        assert_eq!(
            estimation["mediation"]["estimates"][0]["classification"],
            "direct_only"
        );
        assert!(estimation.get("moderation").is_none());

        let mut project = Project::new("Direct-only PLS compatibility");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results[0].id, result.id);
        assert_eq!(reopened.recipes[0], recipe);
        let reopened_estimation = estimation_payload(&reopened.results[0]);
        assert_eq!(
            reopened_estimation["mediation"]["estimates"][0]["classification"],
            "direct_only"
        );
        assert!(reopened_estimation.get("moderation").is_none());
    }

    #[test]
    fn process_graph_v2_append_save_reopen_and_tamper_are_atomic() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("process-v2.qpls");
        let (dataset, recipe, result) = runner_generated_process_v2();
        assert_eq!(
            result.provenance.method_version,
            format!("{REGRESSION_PROCESS_METHOD_VERSION}+{PROCESS_BOOTSTRAP_METHOD_VERSION}")
        );
        let estimation = estimation_payload(&result);
        assert!(estimation.get("mediation").is_none());
        assert!(estimation.get("moderation").is_none());
        let regression = estimation["regression"].as_object().unwrap();
        assert_eq!(regression["observations"], 32);
        assert_eq!(regression["coefficients"], serde_json::json!([]));
        assert_eq!(regression["fit"], serde_json::Value::Null);
        assert_eq!(regression["predictions"], serde_json::json!([]));
        assert!(regression.get("logistic").is_none());
        assert!(regression.get("bootstrap").is_none());
        let mut collapsed_graph: ProcessGraphAnalysis =
            serde_json::from_value(regression["process"]["graph_v2"].clone()).unwrap();
        let collapsed_profile = collapsed_graph
            .variable_profiles
            .iter_mut()
            .find(|profile| profile.variable == "W")
            .unwrap();
        collapsed_profile.raw_mean = 9_007_199_254_740_992.0;
        collapsed_profile.raw_sample_sd = 0.25;
        collapsed_profile.raw_min = 9_007_199_254_740_990.0;
        collapsed_profile.raw_max = 9_007_199_254_740_994.0;
        assert!(
            project_process_probe_grid(
                &collapsed_graph,
                &qpls_core::ProcessModerationConfig {
                    from: "X".into(),
                    to: "M".into(),
                    moderator: "W".into(),
                    conditioning_moderator: None,
                },
            )
            .is_none()
        );
        let mut project = Project::new("PROCESS v2 persistence");
        project.datasets.push(dataset.clone());
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let saved = fs::read(&path).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results[0].id, result.id);
        assert_eq!(reopened.recipes[0], recipe);

        let mixed_failure = process_v2_with_high_leverage_bootstrap_failure(result.clone());
        let mixed_path = directory
            .path()
            .join("process-v2-high-leverage-failure.qpls");
        let mut mixed_project = Project::new("PROCESS v2 mixed bootstrap failure");
        mixed_project.datasets.push(dataset);
        mixed_project
            .append_validated_result(recipe.clone(), mixed_failure.clone())
            .unwrap();
        save_project(&mixed_path, &mixed_project).unwrap();
        let mixed_reopened = load_project(&mixed_path).unwrap();
        let mixed_bootstrap = &estimation_payload(&mixed_reopened.results[0])["regression"]["process"]
            ["graph_v2"]["bootstrap"];
        assert_eq!(mixed_bootstrap["usable_replicates"], 98);
        assert_eq!(
            mixed_bootstrap["failed_replicates"][0]["reason_code"],
            "high_leverage_hc3_instability"
        );

        let mut tampered = result.clone();
        estimation_payload_mut(&mut tampered)["regression"]["process"]["graph_v2"]["reference_effects"]
            [0]["estimate"] = serde_json::json!(99.0);
        let reject_atomically = |tampered| {
            let mut rejected = Project::new("Rejected PROCESS v2");
            assert!(matches!(
                rejected.append_validated_result(recipe.clone(), tampered),
                Err(ProjectError::Invalid(_))
            ));
            assert!(rejected.recipes.is_empty());
            assert!(rejected.results.is_empty());
        };
        let mut unknown_failure = mixed_failure;
        estimation_payload_mut(&mut unknown_failure)["regression"]["process"]["graph_v2"]["bootstrap"]
            ["failed_replicates"][0]["reason_code"] = serde_json::json!("unknown_failure");
        reject_atomically(unknown_failure);
        reject_atomically(tampered);

        let mut tampered_covariance = result.clone();
        estimation_payload_mut(&mut tampered_covariance)["regression"]["process"]["graph_v2"]["equations"]
            [0]["coefficient_covariance"][0][0] = serde_json::json!(99.0);
        reject_atomically(tampered_covariance);

        let mut tampered_negative_variance = result.clone();
        estimation_payload_mut(&mut tampered_negative_variance)["regression"]["process"]["graph_v2"]
            ["equations"][0]["coefficient_covariance"][0][0] = serde_json::json!(-f64::EPSILON);
        reject_atomically(tampered_negative_variance);

        let mut tampered_coefficient_p = result.clone();
        estimation_payload_mut(&mut tampered_coefficient_p)["regression"]["process"]["graph_v2"]
            ["equations"][0]["coefficients"][0]["p_value_two_sided"] =
            serde_json::json!(0.123456789);
        reject_atomically(tampered_coefficient_p);

        let mut tampered_coefficient_interval = result.clone();
        estimation_payload_mut(&mut tampered_coefficient_interval)["regression"]["process"]["graph_v2"]
            ["equations"][0]["coefficients"][0]["confidence_interval_upper"] =
            serde_json::json!(99.0);
        reject_atomically(tampered_coefficient_interval);

        let mut tampered_extra_equation = result.clone();
        let extra_equation = estimation_payload(&tampered_extra_equation)["regression"]["process"]
            ["graph_v2"]["equations"][0]
            .clone();
        estimation_payload_mut(&mut tampered_extra_equation)["regression"]["process"]["graph_v2"]
            ["equations"]
            .as_array_mut()
            .unwrap()
            .push(extra_equation);
        reject_atomically(tampered_extra_equation);

        let mut tampered_equation_term_kind = result.clone();
        estimation_payload_mut(&mut tampered_equation_term_kind)["regression"]["process"]["graph_v2"]
            ["equations"][0]["coefficients"][1]["kind"] = serde_json::json!("control");
        reject_atomically(tampered_equation_term_kind);

        let mut tampered_equation_term_variables = result.clone();
        estimation_payload_mut(&mut tampered_equation_term_variables)["regression"]["process"]["graph_v2"]
            ["equations"][0]["coefficients"][1]["variables"] = serde_json::json!(["W"]);
        reject_atomically(tampered_equation_term_variables);

        let mut tampered_equation_order = result.clone();
        estimation_payload_mut(&mut tampered_equation_order)["regression"]["process"]["graph_v2"]
            ["equations"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        reject_atomically(tampered_equation_order);

        for fit_field in [
            "r_squared",
            "adjusted_r_squared",
            "f_statistic",
            "aic",
            "bic",
            "rmse",
        ] {
            let mut tampered_fit = result.clone();
            estimation_payload_mut(&mut tampered_fit)["regression"]["process"]["graph_v2"]["equations"]
                [0]["fit"][fit_field] = serde_json::json!(99.0);
            reject_atomically(tampered_fit);
        }

        for (field, value) in [
            ("observations", serde_json::json!(99)),
            ("parameter_count", serde_json::json!(99)),
            ("residual_sum_squares", serde_json::json!(99.0)),
            ("total_sum_squares", serde_json::json!(99.0)),
        ] {
            let mut tampered_fit_sufficient_stats = result.clone();
            estimation_payload_mut(&mut tampered_fit_sufficient_stats)["regression"]["process"]["graph_v2"]
                ["equations"][0]["fit"][field] = value;
            reject_atomically(tampered_fit_sufficient_stats);
        }

        let mut tampered_omitted_cases = result.clone();
        estimation_payload_mut(&mut tampered_omitted_cases)["regression"]["process"]["graph_v2"]
            ["omitted_cases"] = serde_json::json!(1);
        reject_atomically(tampered_omitted_cases);

        let mut tampered_extra_profile = result.clone();
        let extra_profile = estimation_payload(&tampered_extra_profile)["regression"]["process"]
            ["graph_v2"]["variable_profiles"][0]
            .clone();
        estimation_payload_mut(&mut tampered_extra_profile)["regression"]["process"]["graph_v2"]
            ["variable_profiles"]
            .as_array_mut()
            .unwrap()
            .push(extra_profile);
        reject_atomically(tampered_extra_profile);

        let mut tampered_profile_order = result.clone();
        estimation_payload_mut(&mut tampered_profile_order)["regression"]["process"]["graph_v2"]
            ["variable_profiles"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        reject_atomically(tampered_profile_order);

        let mut tampered_profile_role = result.clone();
        estimation_payload_mut(&mut tampered_profile_role)["regression"]["process"]["graph_v2"]["variable_profiles"]
            [0]["role"] = serde_json::json!("mediator");
        reject_atomically(tampered_profile_role);

        let mut tampered_generic_fit = result.clone();
        estimation_payload_mut(&mut tampered_generic_fit)["regression"]["fit"] =
            serde_json::json!({"aic": 0.0, "bic": 0.0});
        reject_atomically(tampered_generic_fit);

        for (field, empty_shell) in [
            (
                "mediation",
                serde_json::json!({
                    "method_version": PLS_MEDIATION_METHOD_VERSION,
                    "tolerance": 1e-12,
                    "estimates": [],
                    "warnings": []
                }),
            ),
            (
                "moderation",
                serde_json::json!({
                    "method_version": PLS_TWO_STAGE_MODERATION_METHOD_VERSION,
                    "moderator_score_levels": [-1.0, 0.0, 1.0],
                    "estimates": [],
                    "warnings": []
                }),
            ),
        ] {
            let mut tampered_legacy_shell = result.clone();
            estimation_payload_mut(&mut tampered_legacy_shell)[field] = empty_shell;
            reject_atomically(tampered_legacy_shell);
        }

        for wrapper in ["root", "regression", "process", "graph_v2"] {
            let mut tampered_wrapper = result.clone();
            match wrapper {
                "root" => {
                    estimation_payload_mut(&mut tampered_wrapper)["undeclared"] =
                        serde_json::json!(true);
                }
                "regression" => {
                    estimation_payload_mut(&mut tampered_wrapper)["regression"]["undeclared"] =
                        serde_json::json!(true);
                }
                "process" => {
                    estimation_payload_mut(&mut tampered_wrapper)["regression"]["process"]["undeclared"] =
                        serde_json::json!(true);
                }
                "graph_v2" => {
                    estimation_payload_mut(&mut tampered_wrapper)["regression"]["process"]["graph_v2"]
                        ["undeclared"] = serde_json::json!(true);
                }
                _ => unreachable!(),
            }
            reject_atomically(tampered_wrapper);
        }

        for wrapper in ["root", "regression", "process", "graph_v2"] {
            let unknown_load_path = directory
                .path()
                .join(format!("unknown-{wrapper}-wrapper.qpls"));
            fs::copy(&path, &unknown_load_path).unwrap();
            rewrite_zip_entry_with_manifest_checksum(
                &unknown_load_path,
                PROJECT_ENTRY_NAME,
                |bytes| {
                    let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                    let estimation = &mut document["results"][0]["payload"]["estimation"];
                    match wrapper {
                        "root" => estimation["undeclared"] = serde_json::json!(true),
                        "regression" => {
                            estimation["regression"]["undeclared"] = serde_json::json!(true)
                        }
                        "process" => {
                            estimation["regression"]["process"]["undeclared"] =
                                serde_json::json!(true)
                        }
                        "graph_v2" => {
                            estimation["regression"]["process"]["graph_v2"]["undeclared"] =
                                serde_json::json!(true)
                        }
                        _ => unreachable!(),
                    }
                    serde_json::to_vec_pretty(&document).unwrap()
                },
            );
            assert!(matches!(
                load_project(&unknown_load_path),
                Err(ProjectError::Invalid(_))
            ));
        }

        for (field, empty_shell) in [
            (
                "mediation",
                serde_json::json!({
                    "method_version": PLS_MEDIATION_METHOD_VERSION,
                    "tolerance": 1e-12,
                    "estimates": [],
                    "warnings": []
                }),
            ),
            (
                "moderation",
                serde_json::json!({
                    "method_version": PLS_TWO_STAGE_MODERATION_METHOD_VERSION,
                    "moderator_score_levels": [-1.0, 0.0, 1.0],
                    "estimates": [],
                    "warnings": []
                }),
            ),
        ] {
            let legacy_shell_path = directory
                .path()
                .join(format!("process-v2-empty-{field}-shell.qpls"));
            fs::copy(&path, &legacy_shell_path).unwrap();
            rewrite_zip_entry_with_manifest_checksum(
                &legacy_shell_path,
                PROJECT_ENTRY_NAME,
                |bytes| {
                    let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                    document["results"][0]["payload"]["estimation"][field] = empty_shell.clone();
                    serde_json::to_vec_pretty(&document).unwrap()
                },
            );
            assert!(matches!(
                load_project(&legacy_shell_path),
                Err(ProjectError::Invalid(_))
            ));
        }

        let mut tampered_plot_prediction = result.clone();
        estimation_payload_mut(&mut tampered_plot_prediction)["regression"]["process"]["graph_v2"]
            ["plots"][0]["series"][0]["points"][0]["predicted_raw"] = serde_json::json!(99.0);
        reject_atomically(tampered_plot_prediction);

        let mut tampered_plot_interval = result.clone();
        estimation_payload_mut(&mut tampered_plot_interval)["regression"]["process"]["graph_v2"]
            ["plots"][0]["series"][0]["points"][0]["confidence_interval_upper"] =
            serde_json::json!(99.0);
        reject_atomically(tampered_plot_interval);

        let mut tampered_conditional_probe = result.clone();
        estimation_payload_mut(&mut tampered_conditional_probe)["regression"]["process"]["graph_v2"]
            ["conditional_indirect_effects"][0]["moderator_values"][0]["raw_value"] =
            serde_json::json!(99.0);
        reject_atomically(tampered_conditional_probe);

        let mut tampered_slope_probe = result.clone();
        estimation_payload_mut(&mut tampered_slope_probe)["regression"]["process"]["graph_v2"]["simple_slopes"]
            [0]["moderator_values"][0]["raw_value"] = serde_json::json!(99.0);
        reject_atomically(tampered_slope_probe);

        let mut tampered_plot_probe = result.clone();
        estimation_payload_mut(&mut tampered_plot_probe)["regression"]["process"]["graph_v2"]["plots"]
            [0]["series"][0]["moderator_values"][0]["raw_value"] = serde_json::json!(99.0);
        reject_atomically(tampered_plot_probe);

        let mut tampered_jn_conditioning = result.clone();
        estimation_payload_mut(&mut tampered_jn_conditioning)["regression"]["process"]["graph_v2"]
            ["johnson_neyman"][0]["conditioning_values"] = serde_json::json!([{
            "variable": "W",
            "raw_value": 0.0,
            "coded_value": 0.0
        }]);
        reject_atomically(tampered_jn_conditioning);

        let mut tampered_jn_region = result.clone();
        estimation_payload_mut(&mut tampered_jn_region)["regression"]["process"]["graph_v2"]["johnson_neyman"]
            [0]["regions"][0]["status"] = serde_json::json!("tampered");
        reject_atomically(tampered_jn_region);

        let mut tampered_jn_root_set = result.clone();
        let jn = &mut estimation_payload_mut(&mut tampered_jn_root_set)["regression"]["process"]["graph_v2"]
            ["johnson_neyman"][0];
        let raw_min = jn["raw_min"].clone();
        jn["roots"].as_array_mut().unwrap().insert(0, raw_min);
        let duplicate_region = jn["regions"][0].clone();
        jn["regions"].as_array_mut().unwrap().push(duplicate_region);
        reject_atomically(tampered_jn_root_set);

        let mut tampered_jn_unavailable = result.clone();
        let jn = &mut estimation_payload_mut(&mut tampered_jn_unavailable)["regression"]["process"]
            ["graph_v2"]["johnson_neyman"][0];
        let moderation_id = jn["moderation_id"].clone();
        let solved_moderator = jn["solved_moderator"].clone();
        let conditioning_values = jn["conditioning_values"].clone();
        *jn = serde_json::json!({
            "status": "unavailable",
            "moderation_id": moderation_id,
            "solved_moderator": solved_moderator,
            "conditioning_values": conditioning_values,
            "reason_code": qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_REASON,
            "message": qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        });
        reject_atomically(tampered_jn_unavailable);

        let mut false_invalid_hc3_unavailable = result.clone();
        let jn = &mut estimation_payload_mut(&mut false_invalid_hc3_unavailable)["regression"]["process"]
            ["graph_v2"]["johnson_neyman"][0];
        let moderation_id = jn["moderation_id"].clone();
        let solved_moderator = jn["solved_moderator"].clone();
        let conditioning_values = jn["conditioning_values"].clone();
        *jn = serde_json::json!({
            "status": "unavailable",
            "moderation_id": moderation_id,
            "solved_moderator": solved_moderator,
            "conditioning_values": conditioning_values,
            "reason_code": qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_REASON,
            "message": qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        });
        reject_atomically(false_invalid_hc3_unavailable);

        let mut wrong_invalid_hc3_message = result.clone();
        let jn = &mut estimation_payload_mut(&mut wrong_invalid_hc3_message)["regression"]["process"]
            ["graph_v2"]["johnson_neyman"][0];
        let moderation_id = jn["moderation_id"].clone();
        let solved_moderator = jn["solved_moderator"].clone();
        let conditioning_values = jn["conditioning_values"].clone();
        *jn = serde_json::json!({
            "status": "unavailable",
            "moderation_id": moderation_id,
            "solved_moderator": solved_moderator,
            "conditioning_values": conditioning_values,
            "reason_code": qpls_estimation::PROCESS_JN_INVALID_COVARIANCE_REASON,
            "message": "tampered"
        });
        reject_atomically(wrong_invalid_hc3_message);

        let mut tampered_missing_index = result.clone();
        estimation_payload_mut(&mut tampered_missing_index)["regression"]["process"]["graph_v2"]
            ["moderated_mediation_indices"]
            .as_array_mut()
            .unwrap()
            .clear();
        reject_atomically(tampered_missing_index);

        let mut tampered_index_identity = result.clone();
        estimation_payload_mut(&mut tampered_index_identity)["regression"]["process"]["graph_v2"]
            ["moderated_mediation_indices"][0]["effect_id"] = serde_json::json!("index:tampered");
        reject_atomically(tampered_index_identity);

        let mut tampered_witness = reopened;
        estimation_payload_mut(&mut tampered_witness.results[0])["regression"]["process"]["graph_v2"]
            ["bootstrap"]["validation_witness"]["successful_bootstrap"][0]["estimates"][0] =
            serde_json::json!(99.0);
        assert!(matches!(
            save_project(&path, &tampered_witness),
            Err(ProjectError::Invalid(_))
        ));
        assert_eq!(fs::read(&path).unwrap(), saved);
    }

    #[test]
    fn regression_bootstrap_append_save_reopen_and_tamper_contract_are_atomic() {
        for logistic in [false, true] {
            let label = if logistic { "logistic" } else { "ols" };
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format!("{label}-bootstrap.qpls"));
            let (dataset, recipe, result) = runner_generated_regression_bootstrap(logistic, 2);
            let expected_base = if logistic {
                REGRESSION_LOGISTIC_METHOD_VERSION
            } else {
                REGRESSION_OLS_METHOD_VERSION
            };
            assert_eq!(
                result.provenance.method_version,
                format!("{expected_base}+{REGRESSION_BOOTSTRAP_METHOD_VERSION}")
            );
            assert!(matches!(result.payload, AnalysisPayload::PlsPmV1 { .. }));
            let bootstrap = &estimation_payload(&result)["regression"]["bootstrap"];
            assert_eq!(bootstrap["requested_replicates"], 99);
            assert_eq!(bootstrap["seed"], 91);
            assert_eq!(bootstrap["workers"], 2);
            assert_eq!(
                bootstrap["test_tolerance_policy"],
                qpls_resampling::REGRESSION_BOOTSTRAP_TEST_TOLERANCE_POLICY
            );
            if logistic {
                assert!(bootstrap["coefficients"][0]["odds_ratio"].is_object());
            } else {
                assert!(bootstrap["coefficients"][0].get("odds_ratio").is_none());
            }

            let mut project = Project::new(format!("{label} bootstrap persistence"));
            project.datasets.push(dataset);
            project
                .append_validated_result(recipe.clone(), result.clone())
                .unwrap();
            save_project(&path, &project).unwrap();
            let reopened = load_project(&path).unwrap();
            assert_eq!(reopened.results[0].id, result.id);
            assert_eq!(reopened.recipes[0], recipe);
            let saved = fs::read(&path).unwrap();

            let reject_atomically = |tampered: AnalysisResult| {
                let mut rejected = Project::new("Rejected regression bootstrap");
                assert!(matches!(
                    rejected.append_validated_result(recipe.clone(), tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };
            let reject_unknown_field =
                |mut tampered: AnalysisResult, mutate: &dyn Fn(&mut serde_json::Value)| {
                    mutate(estimation_payload_mut(&mut tampered));
                    reject_atomically(tampered);
                };
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["undeclared"] = serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["coefficients"][0]["undeclared"] =
                    serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["validation_witness"]["undeclared"] =
                    serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["validation_witness"]["successful_bootstrap"]
                    [0]["undeclared"] = serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["validation_witness"]["successful_jackknife"]
                    [0]["undeclared"] = serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["failed_replicates"] = serde_json::json!([{
                    "replicate_index": 0,
                    "reason_code": "single_class_resample",
                    "message": "fixture",
                    "undeclared": true
                }]);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["validation_witness"]["failed_jackknife"] = serde_json::json!([{
                    "omitted_case": 0,
                    "reason_code": "single_class_resample",
                    "message": "fixture",
                    "undeclared": true
                }]);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["coefficients"][0]["test"]["undeclared"] =
                    serde_json::json!(true);
            });
            reject_unknown_field(result.clone(), &|estimation| {
                estimation["regression"]["bootstrap"]["coefficients"][0]["bca"]["undeclared"] =
                    serde_json::json!(true);
            });
            if logistic {
                reject_unknown_field(result.clone(), &|estimation| {
                    estimation["regression"]["bootstrap"]["coefficients"][0]["odds_ratio"]["undeclared"] =
                        serde_json::json!(true);
                });
            }
            let mut count = result.clone();
            estimation_payload_mut(&mut count)["regression"]["bootstrap"]["usable_replicates"] =
                serde_json::json!(1);
            reject_atomically(count);

            let mut bias = result.clone();
            estimation_payload_mut(&mut bias)["regression"]["bootstrap"]["coefficients"][0]["bias"] =
                serde_json::json!(99.0);
            reject_atomically(bias);

            let mut tolerance = result.clone();
            estimation_payload_mut(&mut tolerance)["regression"]["bootstrap"]["coefficients"][0]
                ["test_tolerance"] = serde_json::json!(0.25);
            reject_atomically(tolerance);

            let mut impossible_mean = result.clone();
            estimation_payload_mut(&mut impossible_mean)["regression"]["bootstrap"]["coefficients"]
                [0]["bootstrap_mean"] = serde_json::json!(1.0e100);
            reject_atomically(impossible_mean);

            let mut impossible_percentile = result.clone();
            estimation_payload_mut(&mut impossible_percentile)["regression"]["bootstrap"]["coefficients"]
                [0]["percentile_upper"] = serde_json::json!(1.0e100);
            reject_atomically(impossible_percentile);

            let mut witness_value = result.clone();
            estimation_payload_mut(&mut witness_value)["regression"]["bootstrap"]["validation_witness"]
                ["successful_bootstrap"][0]["coefficients"][0] = serde_json::json!(12345.0);
            reject_atomically(witness_value);

            let mut witness_index = result.clone();
            estimation_payload_mut(&mut witness_index)["regression"]["bootstrap"]["validation_witness"]
                ["successful_bootstrap"][0]["replicate_index"] = serde_json::json!(999);
            reject_atomically(witness_index);

            let mut witness_terms = result.clone();
            estimation_payload_mut(&mut witness_terms)["regression"]["bootstrap"]["validation_witness"]
                ["terms"][0] = serde_json::json!("tampered");
            reject_atomically(witness_terms);

            let mut impossible_bca = result.clone();
            let bca = &mut estimation_payload_mut(&mut impossible_bca)["regression"]["bootstrap"]["coefficients"]
                [0]["bca"];
            if bca["status"] == "available" {
                bca["upper"] = serde_json::json!(1.0e100);
                reject_atomically(impossible_bca);
            }

            if logistic {
                let mut impossible_odds = result.clone();
                estimation_payload_mut(&mut impossible_odds)["regression"]["bootstrap"]["coefficients"]
                    [0]["odds_ratio"]["percentile_upper"] = serde_json::json!(1.0e100);
                reject_atomically(impossible_odds);
            }

            let mut test = result.clone();
            estimation_payload_mut(&mut test)["regression"]["bootstrap"]["coefficients"][0]["test"] = serde_json::json!({
                "status": "available",
                "statistic": 123.0,
                "p_value_two_sided": 0.5
            });
            reject_atomically(test);

            let mut bca = result.clone();
            estimation_payload_mut(&mut bca)["regression"]["bootstrap"]["coefficients"][0]["bca"] = serde_json::json!({
                "status": "unavailable",
                "reason_code": "unknown_reason",
                "message": "tampered"
            });
            reject_atomically(bca);

            let mut tampered_saved = reopened.clone();
            estimation_payload_mut(&mut tampered_saved.results[0])["regression"]["bootstrap"]["stream_token"] =
                serde_json::json!("tampered");
            assert!(matches!(
                save_project(&path, &tampered_saved),
                Err(ProjectError::Invalid(_))
            ));
            assert_eq!(fs::read(&path).unwrap(), saved);

            let mut unknown_for_save = reopened.clone();
            estimation_payload_mut(&mut unknown_for_save.results[0])["regression"]["bootstrap"]["undeclared"] =
                serde_json::json!(true);
            assert!(matches!(
                save_project(
                    &directory.path().join("unknown-bootstrap-save.qpls"),
                    &unknown_for_save
                ),
                Err(ProjectError::Invalid(_))
            ));

            let unknown_load_path = directory.path().join("unknown-bootstrap-load.qpls");
            fs::copy(&path, &unknown_load_path).unwrap();
            rewrite_zip_entry_with_manifest_checksum(
                &unknown_load_path,
                PROJECT_ENTRY_NAME,
                |bytes| {
                    let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                    document["results"][0]["payload"]["estimation"]["regression"]["bootstrap"]["validation_witness"]
                        ["successful_bootstrap"][0]["undeclared"] = serde_json::json!(true);
                    serde_json::to_vec_pretty(&document).unwrap()
                },
            );
            assert!(matches!(
                load_project(&unknown_load_path),
                Err(ProjectError::Invalid(_))
            ));

            let mut historical = recipe.clone();
            historical.metadata = historical.effective_metadata().unwrap();
            historical.schema_version = 2;
            historical.method_config = None;
            let mut legacy_pairing = Project::new("Invalid historical bootstrap pairing");
            legacy_pairing.recipes.push(historical);
            legacy_pairing.results.push(result);
            assert!(matches!(
                save_project(
                    &directory.path().join("legacy-bootstrap-pairing.qpls"),
                    &legacy_pairing
                ),
                Err(ProjectError::Invalid(_))
            ));
        }
    }

    #[test]
    fn runner_generated_ols_v1_commits_saves_reopens_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ols-v1.qpls");
        let (dataset, recipe, result) = runner_generated_ols();
        assert_eq!(result.provenance.method, AnalysisMethod::Regression);
        assert_eq!(
            result.provenance.method_version,
            REGRESSION_OLS_METHOD_VERSION
        );

        let mut project = Project::new("Runner OLS v1 persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.models.len(), 0);
        assert_eq!(reopened.results.len(), 1);
        let (estimation, assessment) = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => (estimation, assessment),
            other => panic!("runner returned unexpected OLS payload: {other:?}"),
        };
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": REGRESSION_NOT_APPLICABLE_ASSESSMENT_VERSION,
                "warnings": [REGRESSION_NOT_APPLICABLE_ASSESSMENT_WARNING]
            })
        );
        assert_eq!(estimation["method_version"], REGRESSION_OLS_METHOD_VERSION);
        assert_eq!(
            estimation["regression"]["method_version"],
            REGRESSION_OLS_METHOD_VERSION
        );
        assert_eq!(estimation["regression"]["regression_type"], "ols");
        assert_eq!(
            estimation["regression"]["coefficients"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            estimation["regression"]["predictions"]
                .as_array()
                .unwrap()
                .len(),
            12
        );

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected OLS");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut tampered_version = result.clone();
        estimation_payload_mut(&mut tampered_version)["regression"]["method_version"] =
            serde_json::json!("regression_ols_v0");
        assert_rejected_atomically(tampered_version, recipe.clone());

        let mut tampered_statistic = result.clone();
        estimation_payload_mut(&mut tampered_statistic)["regression"]["coefficients"][1]["statistic"] =
            serde_json::json!(42.0);
        assert_rejected_atomically(tampered_statistic, recipe.clone());

        let mut tampered_fit = result.clone();
        estimation_payload_mut(&mut tampered_fit)["regression"]["fit"]["r_squared"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(tampered_fit, recipe.clone());

        let mut tampered_prediction = result.clone();
        estimation_payload_mut(&mut tampered_prediction)["regression"]["predictions"][0]["residual"] =
            serde_json::json!(99.0);
        assert_rejected_atomically(tampered_prediction, recipe.clone());

        let mut mismatched_recipe = recipe.clone();
        mismatched_recipe
            .metadata
            .insert("regression_predictors".into(), "x,z".into());
        let mut mismatched_result = result.clone();
        mismatched_result.provenance.recipe_id = mismatched_recipe.id;
        assert_rejected_atomically(mismatched_result, mismatched_recipe);

        let mut tampered_for_save = reopened.clone();
        estimation_payload_mut(&mut tampered_for_save.results[0])["regression"]["coefficients"]
            [0]["estimate"] = serde_json::json!(-999.0);
        assert!(matches!(
            save_project(
                &directory.path().join("tampered-ols.qpls"),
                &tampered_for_save
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn runner_generated_nca_v2_commits_saves_reopens_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nca-v2.qpls");
        let legacy_path = directory.path().join("nca-v1-legacy.qpls");
        let (dataset, recipe, result) = runner_generated_nca();
        assert_eq!(result.provenance.method, AnalysisMethod::Nca);
        assert_eq!(result.provenance.method_version, NCA_METHOD_VERSION);
        assert!(matches!(result.payload, AnalysisPayload::PlsPmV1 { .. }));
        let assessment = match &result.payload {
            AnalysisPayload::PlsPmV1 { assessment, .. } => assessment,
            other => panic!("runner returned unexpected NCA payload: {other:?}"),
        };
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": NCA_NOT_APPLICABLE_ASSESSMENT_VERSION,
                "warnings": [NCA_NOT_APPLICABLE_ASSESSMENT_WARNING]
            })
        );

        let mut project = Project::new("Runner NCA v2 persistence");
        project.datasets.push(dataset.clone());
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();

        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert_eq!(
            reopened.results[0].provenance.method_version,
            NCA_METHOD_VERSION
        );
        assert!(
            reopened.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "nca.legacy_method_version")
        );
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected NCA payload: {other:?}"),
        };
        assert_eq!(estimation["nca"]["method_version"], NCA_METHOD_VERSION);
        assert_eq!(
            estimation["nca"]["ce_fdh_peers"].as_array().unwrap().len(),
            5
        );
        assert_eq!(estimation["nca"]["ceilings"].as_array().unwrap().len(), 2);
        assert_eq!(
            estimation["nca"]["bottlenecks"].as_array().unwrap().len(),
            18
        );

        let assert_rejected_atomically =
            |tampered: AnalysisResult, tampered_recipe: AnalysisRecipe| {
                let mut rejected = Project::new("Rejected NCA");
                assert!(matches!(
                    rejected.append_validated_result(tampered_recipe, tampered),
                    Err(ProjectError::Invalid(_))
                ));
                assert!(rejected.recipes.is_empty());
                assert!(rejected.results.is_empty());
            };

        let mut mismatched_nested_version = result.clone();
        estimation_payload_mut(&mut mismatched_nested_version)["nca"]["method_version"] =
            serde_json::json!(NCA_METHOD_VERSION_V1);
        assert_rejected_atomically(mismatched_nested_version, recipe.clone());

        let mut tampered_scope = result.clone();
        estimation_payload_mut(&mut tampered_scope)["nca"]["scope"]["maximum_x"] =
            serde_json::json!(99.0);
        assert_rejected_atomically(tampered_scope, recipe.clone());

        let mut tampered_peer = result.clone();
        estimation_payload_mut(&mut tampered_peer)["nca"]["ce_fdh_peers"][1]["y"] =
            serde_json::json!(4.75);
        assert_rejected_atomically(tampered_peer, recipe.clone());

        let mut tampered_effect = result.clone();
        estimation_payload_mut(&mut tampered_effect)["nca"]["ceilings"][0]["effect_size"] =
            serde_json::json!(0.123);
        assert_rejected_atomically(tampered_effect, recipe.clone());

        let mut tampered_bottleneck = result.clone();
        estimation_payload_mut(&mut tampered_bottleneck)["nca"]["bottlenecks"][4]["required_x_percent"] =
            serde_json::json!(88.0);
        assert_rejected_atomically(tampered_bottleneck, recipe.clone());

        let mut tampered_assessment = result.clone();
        if let AnalysisPayload::PlsPmV1 { assessment, .. } = &mut tampered_assessment.payload {
            assessment["warnings"] = serde_json::json!(["obsolete standalone warning"]);
        }
        assert_rejected_atomically(tampered_assessment, recipe.clone());

        let mut mismatched_recipe = recipe.clone();
        mismatched_recipe
            .metadata
            .insert("nca_y".into(), "x".into());
        let mut mismatched_result = result.clone();
        mismatched_result.provenance.recipe_id = mismatched_recipe.id;
        assert_rejected_atomically(mismatched_result, mismatched_recipe);

        let mut tampered_for_save = reopened.clone();
        estimation_payload_mut(&mut tampered_for_save.results[0])["nca"]["ceilings"][1]["slope"] =
            serde_json::json!(42.0);
        assert!(matches!(
            save_project(
                &directory.path().join("tampered-nca.qpls"),
                &tampered_for_save
            ),
            Err(ProjectError::Invalid(_))
        ));

        let legacy_result = legacy_nca_v1_result(result);
        let mut legacy_project = Project::new("Legacy NCA v1 compatibility");
        legacy_project.datasets.push(dataset);
        legacy_project
            .append_validated_result(recipe, legacy_result)
            .unwrap();
        save_project(&legacy_path, &legacy_project).unwrap();
        let legacy_reopened = load_project(&legacy_path).unwrap();
        assert_eq!(
            legacy_reopened.results[0].provenance.method_version,
            NCA_METHOD_VERSION_V1
        );
        assert!(
            legacy_reopened.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "nca.legacy_method_version")
        );
        assert!(has_compatibility_notice(
            &legacy_reopened,
            legacy_reopened.results[0].id,
            "nca.legacy_method_version"
        ));
    }

    #[test]
    fn runner_generated_prediction_round_trips_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("prediction.qpls");
        let v3_path = directory.path().join("prediction-v3.qpls");
        let (dataset, recipe, result) = runner_generated_prediction();
        assert_eq!(result.provenance.method, AnalysisMethod::Predict);
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_PREDICT_METHOD_VERSION)
        );
        assert!(matches!(result.payload, AnalysisPayload::PlsPmV1 { .. }));

        let mut project = Project::new("Runner Prediction persistence");
        project.datasets.push(dataset);
        project.recipes.push(recipe);
        project.results.push(result);
        save_project(&path, &project).unwrap();

        let runner_restored = load_project(&path).unwrap();
        assert_eq!(runner_restored.results.len(), 1);
        assert_eq!(
            runner_restored.results[0].provenance.method,
            AnalysisMethod::Predict
        );
        assert!(matches!(
            runner_restored.results[0].payload,
            AnalysisPayload::PlsPmV1 { .. }
        ));

        let mut v3_result = runner_restored.results[0].clone();
        let payload = std::mem::replace(
            &mut v3_result.payload,
            AnalysisPayload::Legacy {
                value: serde_json::Value::Null,
            },
        );
        v3_result.payload = match payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => AnalysisPayload::PlsPmV3 {
                estimation,
                assessment,
                bootstrap: None,
                permutation: None,
            },
            other => panic!("runner returned unexpected Prediction payload: {other:?}"),
        };

        let mut v3_project = runner_restored.clone();
        v3_project.results = vec![v3_result];
        save_project(&v3_path, &v3_project).unwrap();

        let restored = load_project(&v3_path).unwrap();
        assert_eq!(restored.results.len(), 1);
        assert_eq!(
            restored.results[0].provenance.method,
            AnalysisMethod::Predict
        );
        assert!(matches!(
            restored.results[0].payload,
            AnalysisPayload::PlsPmV3 {
                bootstrap: None,
                permutation: None,
                ..
            }
        ));
        let mut prediction = restored.results[0].clone();
        assert_eq!(
            estimation_payload_mut(&mut prediction)["method_version"].as_str(),
            Some(PLS_PREDICT_METHOD_VERSION)
        );
        assert_eq!(
            estimation_payload_mut(&mut prediction)["predict"]["method_version"].as_str(),
            Some(PLS_PREDICT_METHOD_VERSION)
        );

        let mut mismatched_estimator = restored.results[0].clone();
        estimation_payload_mut(&mut mismatched_estimator)["method_version"] =
            serde_json::json!(PLS_METHOD_VERSION);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[mismatched_estimator], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut mismatched_artifact = restored.results[0].clone();
        estimation_payload_mut(&mut mismatched_artifact)["predict"]["method_version"] =
            serde_json::json!(PLS_METHOD_VERSION);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[mismatched_artifact], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_indicator_metric = restored.results[0].clone();
        estimation_payload_mut(&mut tampered_indicator_metric)["predict"]["repeated_kfold"]["indicator_targets"]
            [0]["pls"]["squared_error_sum"] = serde_json::json!(0.0);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[tampered_indicator_metric], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_assignment_digest = restored.results[0].clone();
        estimation_payload_mut(&mut tampered_assignment_digest)["predict"]["repeated_kfold"]["assignment_digest"] =
            serde_json::json!(format!("sha256:{}", "A".repeat(64)));
        assert!(matches!(
            validate_result_contracts_with_recipes(
                &[tampered_assignment_digest],
                &restored.recipes
            ),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_cvpat = restored.results[0].clone();
        estimation_payload_mut(&mut tampered_cvpat)["predict"]["repeated_kfold"]["cvpat_benchmark_assessments"]
            [0]["p_value_one_sided"] = serde_json::json!(0.999_999);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[tampered_cvpat], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_artifact = restored.results[0].clone();
        estimation_payload_mut(&mut missing_artifact)["predict"] = serde_json::Value::Null;
        assert!(matches!(
            validate_result_contracts_with_recipes(&[missing_artifact.clone()], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));
        let mut invalid_project = restored.clone();
        invalid_project.results = vec![missing_artifact];
        assert!(matches!(
            save_project(
                &directory.path().join("missing-predict.qpls"),
                &invalid_project
            ),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_envelope_version = restored.results[0].clone();
        missing_envelope_version.provenance.method_version = missing_envelope_version
            .provenance
            .method_version
            .replace(&format!("{PLS_PREDICT_METHOD_VERSION}+"), "");
        assert!(matches!(
            validate_result_contracts_with_recipes(&[missing_envelope_version], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut relabeled = restored.results[0].clone();
        relabeled.provenance.method = AnalysisMethod::PlsPm;
        relabeled.provenance.settings.method = AnalysisMethod::PlsPm;
        let mut relabeled_recipe = restored.recipes[0].clone();
        relabeled_recipe.settings.method = AnalysisMethod::PlsPm;
        assert!(matches!(
            validate_result_contracts_with_recipes(&[relabeled], &[relabeled_recipe]),
            Err(ProjectError::Invalid(_))
        ));

        let mut unsupported_weighting = restored.results[0].clone();
        unsupported_weighting.provenance.settings.case_weight_column = Some("weight".into());
        let mut unsupported_weighting_recipe = restored.recipes[0].clone();
        unsupported_weighting_recipe.settings.case_weight_column = Some("weight".into());
        assert!(matches!(
            validate_result_contracts_with_recipes(
                &[unsupported_weighting],
                &[unsupported_weighting_recipe]
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn validated_append_accepts_prediction_and_rejects_tampering_atomically() {
        let (dataset, recipe, result) = runner_generated_prediction();
        let mut project = Project::new("Validated Prediction append");
        project.datasets.push(dataset.clone());
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        assert_eq!(project.recipes.len(), 1);
        assert_eq!(project.results.len(), 1);
        assert_eq!(
            project.results[0].provenance.method,
            AnalysisMethod::Predict
        );

        let mut tampered = result;
        estimation_payload_mut(&mut tampered)["predict"]["indicator_targets"][0]["pls"]["squared_error_sum"] =
            serde_json::json!(0.0);
        let mut rejected = Project::new("Rejected Prediction append");
        rejected.datasets.push(dataset);
        assert!(matches!(
            rejected.append_validated_result(recipe, tampered),
            Err(ProjectError::Invalid(_))
        ));
        assert!(rejected.recipes.is_empty());
        assert!(rejected.results.is_empty());
    }

    #[test]
    fn legacy_prediction_v1_reopens_with_warning_but_cannot_be_appended_as_new_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-prediction-v1.qpls");
        let (dataset, recipe, result) = runner_generated_prediction();
        let legacy = legacy_prediction_v1_result(result);

        let mut rejected = Project::new("Reject legacy Prediction append");
        rejected.datasets.push(dataset.clone());
        assert!(matches!(
            rejected.append_validated_result(recipe.clone(), legacy.clone()),
            Err(ProjectError::Invalid(message)) if message.contains("archive-readable")
        ));
        assert!(rejected.recipes.is_empty());
        assert!(rejected.results.is_empty());

        let mut archived = Project::new("Legacy Prediction archive");
        archived.datasets.push(dataset);
        archived.recipes.push(recipe);
        archived.results.push(legacy);
        save_project(&path, &archived).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert!(
            reopened.results[0]
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_PREDICT_METHOD_VERSION_V1)
        );
        assert!(
            reopened.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "predict.legacy_method_version")
        );
        assert!(has_compatibility_notice(
            &reopened,
            reopened.results[0].id,
            "predict.legacy_method_version"
        ));
    }

    #[test]
    fn validated_append_and_archive_round_trip_preserve_explicit_mga_contract() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mga.qpls");
        let (dataset, recipe, result) = runner_generated_mga();
        assert_eq!(result.provenance.method, AnalysisMethod::Mga);
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_MGA_METHOD_VERSION)
        );
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_MGA_PERMUTATION_METHOD_VERSION)
        );

        let mut project = Project::new("Validated MGA persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        assert!(!reopened.recipes[0].metadata.contains_key("mga_group_a"));
        let effective_metadata = reopened.recipes[0].effective_metadata().unwrap();
        assert_eq!(effective_metadata["mga_group_a"], "A");
        assert_eq!(effective_metadata["mga_group_b"], "B");
        let estimation = match &reopened.results[0].payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected MGA payload: {other:?}"),
        };
        assert_eq!(estimation["mga"]["groups"][0]["group"], "A");
        assert_eq!(estimation["mga"]["groups"][1]["group"], "B");
        assert_eq!(estimation["micom"]["method_version"], MICOM_METHOD_VERSION);
        assert_eq!(
            estimation["micom"]["constructs"].as_array().unwrap().len(),
            3
        );
        assert_eq!(
            estimation["mga"]["measurement_comparisons"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
        assert_eq!(estimation["mga_permutation"]["permutation_samples"], 5000);
        assert_eq!(
            estimation["mga_permutation"]["measurement_comparisons"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
    }

    #[test]
    fn validated_append_rejects_tampered_mga_direction_atomically() {
        let (dataset, recipe, result) = runner_generated_mga();
        let mut tampered_direction = result.clone();
        estimation_payload_mut(&mut tampered_direction)["mga"]["comparisons"][0]["group_a"] =
            serde_json::json!("B");
        let mut project = Project::new("Rejected MGA append");
        project.datasets.push(dataset.clone());
        assert!(matches!(
            project.append_validated_result(recipe.clone(), tampered_direction),
            Err(ProjectError::Invalid(_))
        ));
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());

        for (name, mut tampered) in [
            ("measurement difference", result.clone()),
            ("MICOM decision", result.clone()),
            ("historical version", result.clone()),
        ] {
            match name {
                "measurement difference" => {
                    estimation_payload_mut(&mut tampered)["mga_permutation"]["measurement_comparisons"]
                        [0]["original_difference"] = serde_json::json!(99.0);
                }
                "MICOM decision" => {
                    let current = estimation_payload_mut(&mut tampered)["micom"]["constructs"]
                        [0]["full_invariance"]
                        .as_bool()
                        .unwrap();
                    estimation_payload_mut(&mut tampered)["micom"]["constructs"][0]["full_invariance"] =
                        serde_json::json!(!current);
                }
                "historical version" => {
                    estimation_payload_mut(&mut tampered)["micom"]["method_version"] =
                        serde_json::json!(MICOM_METHOD_VERSION_V1);
                    tampered.provenance.method_version = tampered
                        .provenance
                        .method_version
                        .replace(MICOM_METHOD_VERSION, MICOM_METHOD_VERSION_V1);
                }
                _ => unreachable!(),
            }
            let mut rejected = Project::new(format!("Rejected {name}"));
            rejected.datasets.push(dataset.clone());
            assert!(matches!(
                rejected.append_validated_result(recipe.clone(), tampered),
                Err(ProjectError::Invalid(_))
            ));
            assert!(rejected.recipes.is_empty());
            assert!(rejected.results.is_empty());
        }

        let mut mga_only_recipe = recipe;
        mga_only_recipe
            .metadata
            .insert("group_methods".into(), "mga_permutation".into());
        let mut mismatched_result = result;
        mismatched_result.provenance.recipe_id = mga_only_recipe.id;
        let mut rejected_native_persistence = Project::new("Rejected MGA-only append");
        rejected_native_persistence.datasets.push(dataset);
        assert!(matches!(
            rejected_native_persistence.append_validated_result(mga_only_recipe, mismatched_result),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn mediation_contract_round_trips_and_rejects_tampered_rows_and_versions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mediation.qpls");
        let (dataset, recipe, result) = runner_generated_mediation();
        let estimation = match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected mediation payload: {other:?}"),
        };
        assert_eq!(
            estimation["mediation"]["method_version"].as_str(),
            Some(PLS_MEDIATION_METHOD_VERSION)
        );
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_MEDIATION_METHOD_VERSION)
        );
        assert_eq!(
            estimation["mediation"]["estimates"]
                .as_array()
                .unwrap()
                .len(),
            3
        );

        let mut project = Project::new("Validated mediation persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);

        let mut legacy_omission = result.clone();
        estimation_payload_mut(&mut legacy_omission)
            .as_object_mut()
            .unwrap()
            .remove("mediation");
        legacy_omission.provenance.method_version = legacy_omission
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != PLS_MEDIATION_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        let legacy_path = directory.path().join("legacy-mediation-omission.qpls");
        let mut legacy_project = Project::new("Legacy mediation omission");
        legacy_project
            .append_validated_result(recipe.clone(), legacy_omission)
            .unwrap();
        save_project(&legacy_path, &legacy_project).unwrap();
        let mut legacy_reopened = load_project(&legacy_path).unwrap();
        assert!(
            estimation_payload_mut(&mut legacy_reopened.results[0])
                .get("mediation")
                .is_none()
        );

        let mut tampered_classification = result.clone();
        estimation_payload_mut(&mut tampered_classification)["mediation"]["estimates"][1]["classification"] =
            serde_json::json!("direct_only");
        assert!(matches!(
            Project::new("tampered classification")
                .append_validated_result(recipe.clone(), tampered_classification),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_row = result.clone();
        estimation_payload_mut(&mut tampered_row)["mediation"]["estimates"][1]["indirect"] =
            serde_json::json!(0.0);
        assert!(matches!(
            Project::new("tampered mediation row")
                .append_validated_result(recipe.clone(), tampered_row),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_effects = result.clone();
        let mut parsed_estimation: PlsResult =
            serde_json::from_value(estimation_payload_mut(&mut tampered_effects).clone()).unwrap();
        let indirect = parsed_estimation
            .effects
            .iter_mut()
            .find(|effect| effect.source == "x" && effect.target == "y")
            .unwrap();
        indirect.indirect *= 0.5;
        indirect.total = indirect.direct + indirect.indirect;
        parsed_estimation.mediation =
            analyze_mediation_effects_with_tolerance(&parsed_estimation.effects, 1e-12);
        *estimation_payload_mut(&mut tampered_effects) =
            serde_json::to_value(parsed_estimation).unwrap();
        assert!(matches!(
            Project::new("tampered effects and mediation")
                .append_validated_result(recipe.clone(), tampered_effects),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_payload_version = result.clone();
        estimation_payload_mut(&mut missing_payload_version)["mediation"]["method_version"] =
            serde_json::json!("pls_mediation_v0");
        assert!(matches!(
            Project::new("unsupported mediation version")
                .append_validated_result(recipe.clone(), missing_payload_version),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_payload = result.clone();
        estimation_payload_mut(&mut missing_payload)
            .as_object_mut()
            .unwrap()
            .remove("mediation");
        assert!(matches!(
            Project::new("missing mediation payload")
                .append_validated_result(recipe.clone(), missing_payload),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_envelope_version = result;
        missing_envelope_version.provenance.method_version = missing_envelope_version
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != PLS_MEDIATION_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        assert!(matches!(
            Project::new("missing mediation provenance")
                .append_validated_result(recipe, missing_envelope_version),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn moderation_contract_round_trips_and_rejects_tampering_and_one_sided_omissions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("moderation.qpls");
        let (dataset, recipe, result) = runner_generated_moderation();
        let estimation = match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("runner returned unexpected moderation payload: {other:?}"),
        };
        assert_eq!(
            estimation["moderation"]["method_version"].as_str(),
            Some(PLS_TWO_STAGE_MODERATION_METHOD_VERSION)
        );
        assert_eq!(
            estimation["moderation"]["estimates"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_TWO_STAGE_MODERATION_METHOD_VERSION)
        );

        let mut project = Project::new("Validated moderation persistence");
        project.datasets.push(dataset);
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.results.len(), 1);
        let reopened_estimation: PlsResult = serde_json::from_value(
            estimation_payload_mut(&mut reopened.results[0].clone()).clone(),
        )
        .unwrap();
        assert_eq!(
            reopened_estimation.moderation,
            analyze_moderation(&reopened.recipes[0], &reopened_estimation)
        );

        let mut tampered_effect = result.clone();
        estimation_payload_mut(&mut tampered_effect)["moderation"]["estimates"][0]["interaction_effect"] =
            serde_json::json!(0.0);
        assert!(matches!(
            Project::new("tampered moderation effect")
                .append_validated_result(recipe.clone(), tampered_effect),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_levels = result.clone();
        estimation_payload_mut(&mut tampered_levels)["moderation"]["moderator_score_levels"] =
            serde_json::json!([-1.0, 0.0, 2.0]);
        assert!(matches!(
            Project::new("tampered moderation score levels")
                .append_validated_result(recipe.clone(), tampered_levels),
            Err(ProjectError::Invalid(_))
        ));

        let mut unsupported_payload_version = result.clone();
        estimation_payload_mut(&mut unsupported_payload_version)["moderation"]["method_version"] =
            serde_json::json!("pls_two_stage_moderation_v0");
        assert!(matches!(
            Project::new("unsupported moderation payload")
                .append_validated_result(recipe.clone(), unsupported_payload_version),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_payload = result.clone();
        estimation_payload_mut(&mut missing_payload)
            .as_object_mut()
            .unwrap()
            .remove("moderation");
        assert!(matches!(
            Project::new("missing moderation payload")
                .append_validated_result(recipe.clone(), missing_payload),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_provenance = result.clone();
        missing_provenance.provenance.method_version = missing_provenance
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != PLS_TWO_STAGE_MODERATION_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        assert!(matches!(
            Project::new("missing moderation provenance")
                .append_validated_result(recipe.clone(), missing_provenance),
            Err(ProjectError::Invalid(_))
        ));

        let mut recipe_without_interaction = recipe.clone();
        recipe_without_interaction.model.interactions.clear();
        assert!(matches!(
            Project::new("moderation result without interaction recipe")
                .append_validated_result(recipe_without_interaction, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut multiple_interactions = recipe.clone();
        let mut second_interaction = multiple_interactions.model.interactions[0].clone();
        second_interaction.id = "second_interaction".into();
        multiple_interactions
            .model
            .interactions
            .push(second_interaction);
        assert!(matches!(
            Project::new("multiple unqualified interactions")
                .append_validated_result(multiple_interactions, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut unsupported_weighting_recipe = recipe.clone();
        unsupported_weighting_recipe.settings.weighting_scheme = qpls_core::WeightingScheme::Factor;
        let mut unsupported_weighting_result = result.clone();
        unsupported_weighting_result
            .provenance
            .settings
            .weighting_scheme = qpls_core::WeightingScheme::Factor;
        assert!(matches!(
            Project::new("unsupported moderation weighting").append_validated_result(
                unsupported_weighting_recipe,
                unsupported_weighting_result
            ),
            Err(ProjectError::Invalid(_))
        ));

        let mut unsupported_preprocessing_recipe = recipe.clone();
        unsupported_preprocessing_recipe.settings.preprocessing =
            qpls_core::Preprocessing::MeanCentered;
        let mut unsupported_preprocessing_result = result.clone();
        unsupported_preprocessing_result
            .provenance
            .settings
            .preprocessing = qpls_core::Preprocessing::MeanCentered;
        assert!(matches!(
            Project::new("unsupported moderation preprocessing").append_validated_result(
                unsupported_preprocessing_recipe,
                unsupported_preprocessing_result
            ),
            Err(ProjectError::Invalid(_))
        ));

        let mut weighted_recipe = recipe.clone();
        weighted_recipe.settings.case_weight_column = Some("case_weight".into());
        let mut weighted_result = result.clone();
        weighted_result.provenance.settings.case_weight_column = Some("case_weight".into());
        assert!(matches!(
            Project::new("case-weighted moderation")
                .append_validated_result(weighted_recipe, weighted_result),
            Err(ProjectError::Invalid(_))
        ));

        let mut unmeasured_role = recipe.clone();
        let moderator = unmeasured_role.model.interactions[0].moderator.clone();
        unmeasured_role
            .model
            .constructs
            .iter_mut()
            .find(|construct| construct.id == moderator)
            .unwrap()
            .indicators
            .clear();
        assert!(matches!(
            Project::new("unmeasured moderator")
                .append_validated_result(unmeasured_role, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut observed_product = recipe.clone();
        let product = observed_product.model.interactions[0]
            .product_construct
            .clone();
        observed_product
            .model
            .constructs
            .iter_mut()
            .find(|construct| construct.id == product)
            .unwrap()
            .indicators
            .push("forged_product_indicator".into());
        assert!(matches!(
            Project::new("observed interaction product")
                .append_validated_result(observed_product, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut extra_product_path = recipe.clone();
        let interaction = extra_product_path.model.interactions[0].clone();
        extra_product_path
            .model
            .paths
            .push(qpls_core::StructuralPath {
                source: interaction.outcome,
                target: interaction.product_construct,
            });
        assert!(matches!(
            Project::new("extra interaction product path")
                .append_validated_result(extra_product_path, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut controlled_product = recipe.clone();
        let interaction = controlled_product.model.interactions[0].clone();
        controlled_product
            .model
            .controls
            .push(qpls_core::ControlPath {
                source: interaction.product_construct,
                target: interaction.outcome,
                label: Some("forged control annotation".into()),
            });
        assert!(matches!(
            Project::new("controlled interaction product")
                .append_validated_result(controlled_product, result.clone()),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_main_effect = recipe;
        let interaction = missing_main_effect.model.interactions[0].clone();
        missing_main_effect.model.paths.retain(|path| {
            path.source != interaction.moderator || path.target != interaction.outcome
        });
        assert!(matches!(
            Project::new("missing moderator main effect")
                .append_validated_result(missing_main_effect, result),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn runner_generated_two_stage_hoc_appends_round_trips_and_rejects_contract_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("two-stage-hoc.qpls");
        let (dataset, recipe, result) = runner_generated_higher_order();
        let estimation: PlsResult =
            serde_json::from_value(estimation_payload_mut(&mut result.clone()).clone()).unwrap();
        assert_eq!(recipe.model.higher_order_constructs.len(), 1);
        assert!(
            estimation
                .outer_estimates
                .iter()
                .any(|row| { row.construct == "hoc" && row.indicator == "__qpls_hoc_hoc_x" })
        );

        let mut project = Project::new("Validated disjoint two-stage HOC persistence");
        project.datasets.push(dataset);
        project.models.push(recipe.model.clone());
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();
        assert_eq!(reopened.models.len(), 1);
        assert_eq!(
            reopened.recipes[0].model.higher_order_constructs,
            recipe.model.higher_order_constructs
        );
        let reopened_estimation: PlsResult = serde_json::from_value(
            estimation_payload_mut(&mut reopened.results[0].clone()).clone(),
        )
        .unwrap();
        assert_eq!(
            reopened_estimation.method_version,
            estimation.method_version
        );
        assert_eq!(
            reopened_estimation
                .outer_estimates
                .iter()
                .map(|row| (row.construct.as_str(), row.indicator.as_str()))
                .collect::<Vec<_>>(),
            estimation
                .outer_estimates
                .iter()
                .map(|row| (row.construct.as_str(), row.indicator.as_str()))
                .collect::<Vec<_>>()
        );
        assert!(
            reopened_estimation
                .outer_estimates
                .iter()
                .zip(&estimation.outer_estimates)
                .all(|(left, right)| close_enough(left.loading, right.loading)
                    && close_enough(left.weight, right.weight))
        );
        assert!(close_enough(
            reopened_estimation.paths[0].coefficient,
            estimation.paths[0].coefficient
        ));
        assert_eq!(
            reopened_estimation
                .construct_scores
                .keys()
                .collect::<Vec<_>>(),
            estimation.construct_scores.keys().collect::<Vec<_>>()
        );
        assert!(
            reopened_estimation
                .construct_scores
                .iter()
                .all(|(construct, scores)| {
                    scores.len() == estimation.construct_scores[construct].len()
                        && scores
                            .iter()
                            .zip(&estimation.construct_scores[construct])
                            .all(|(left, right)| close_enough(*left, *right))
                })
        );

        let reject =
            |name: &str, tampered_recipe: AnalysisRecipe, tampered_result: AnalysisResult| {
                assert!(matches!(
                    Project::new(name).append_validated_result(tampered_recipe, tampered_result),
                    Err(ProjectError::Invalid(_))
                ));
            };

        let mut tampered_loading = result.clone();
        let outer = estimation_payload_mut(&mut tampered_loading)["outer_estimates"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["indicator"] == "__qpls_hoc_hoc_x")
            .unwrap();
        outer["loading"] = serde_json::json!(0.123);
        reject("tampered HOC loading", recipe.clone(), tampered_loading);

        let mut tampered_path = result.clone();
        estimation_payload_mut(&mut tampered_path)["paths"][0]["coefficient"] =
            serde_json::json!(0.123);
        reject("tampered HOC path", recipe.clone(), tampered_path);

        let mut missing_generated_indicator = result.clone();
        estimation_payload_mut(&mut missing_generated_indicator)["outer_estimates"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["indicator"] != "__qpls_hoc_hoc_z");
        reject(
            "missing generated HOC component score",
            recipe.clone(),
            missing_generated_indicator,
        );

        let mut unsupported_method = recipe.clone();
        unsupported_method.model.higher_order_constructs[0].method =
            HigherOrderMethod::RepeatedIndicators;
        reject(
            "unsupported native HOC method",
            unsupported_method,
            result.clone(),
        );

        let mut extra_path = recipe.clone();
        extra_path.model.paths.push(qpls_core::StructuralPath {
            source: "x".into(),
            target: "y".into(),
        });
        reject("extra HOC structural path", extra_path, result.clone());

        let mut resampled_recipe = recipe.clone();
        resampled_recipe.settings.bootstrap_samples = 100;
        let mut resampled_result = result;
        resampled_result.provenance.settings.bootstrap_samples = 100;
        reject(
            "unsupported HOC inference",
            resampled_recipe,
            resampled_result,
        );
    }

    #[test]
    fn legacy_non_moderation_archive_without_payload_or_version_remains_readable() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-no-moderation.qpls");
        let (dataset, recipe, mut result) = runner_generated_mediation();
        assert!(recipe.model.interactions.is_empty());
        assert!(
            !result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_TWO_STAGE_MODERATION_METHOD_VERSION)
        );
        estimation_payload_mut(&mut result)
            .as_object_mut()
            .unwrap()
            .remove("moderation");

        let mut project = Project::new("Legacy archive without moderation fields");
        project.datasets.push(dataset);
        project.append_validated_result(recipe, result).unwrap();
        save_project(&path, &project).unwrap();
        let mut reopened = load_project(&path).unwrap();
        assert!(
            estimation_payload_mut(&mut reopened.results[0])
                .get("moderation")
                .is_none()
        );
    }

    #[test]
    fn moderation_bootstrap_binds_the_exact_product_path_and_original_effect() {
        let (dataset, mut recipe, _) = runner_generated_moderation();
        recipe.settings.bootstrap_samples = 8;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsBootstrap);
        recipe.settings.workers = 1;
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        Project::new("validated moderation bootstrap")
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();

        let interaction = &recipe.model.interactions[0];
        let parameter_identity = serde_json::to_string(&(
            "path",
            [
                interaction.product_construct.as_str(),
                interaction.outcome.as_str(),
            ],
        ))
        .unwrap();

        let mut tampered_original = result.clone();
        let bootstrap = match &mut tampered_original.payload {
            AnalysisPayload::PlsPmV2 { bootstrap, .. } => bootstrap,
            other => panic!("runner returned unexpected moderation bootstrap payload: {other:?}"),
        };
        let parameter = bootstrap["percentile"]["parameters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|parameter| parameter["parameter"].as_str() == Some(&parameter_identity))
            .unwrap();
        let forged_original = parameter["original"].as_f64().unwrap() + 0.125;
        let standard_error = parameter["standard_error"].as_f64().unwrap();
        let (t_statistic, p_value_two_sided) =
            normal_reference_test(forged_original, standard_error);
        parameter["original"] = serde_json::json!(forged_original);
        parameter["t_statistic"] = serde_json::to_value(t_statistic).unwrap();
        parameter["p_value_two_sided"] = serde_json::to_value(p_value_two_sided).unwrap();
        assert!(matches!(
            Project::new("tampered moderation bootstrap original")
                .append_validated_result(recipe.clone(), tampered_original),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered_identity = result;
        let bootstrap = match &mut tampered_identity.payload {
            AnalysisPayload::PlsPmV2 { bootstrap, .. } => bootstrap,
            other => panic!("runner returned unexpected moderation bootstrap payload: {other:?}"),
        };
        let forged_identity = serde_json::to_string(&(
            "path",
            [interaction.product_construct.as_str(), "forged_outcome"],
        ))
        .unwrap();
        for section in ["percentile", "bca"] {
            let parameter = bootstrap[section]["parameters"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|parameter| parameter["parameter"].as_str() == Some(&parameter_identity))
                .unwrap();
            parameter["parameter"] = serde_json::json!(forged_identity);
        }
        assert!(matches!(
            Project::new("tampered moderation bootstrap identity")
                .append_validated_result(recipe, tampered_identity),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn legacy_plsc_v1_remains_readable_and_is_marked_noncurrent() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-plsc-v1.qpls");
        let round_trip_path = directory.path().join("legacy-plsc-v1-round-trip.qpls");
        let (dataset, recipe) = pls_family_fixture(AnalysisMethod::Plsc);
        let mut legacy = completed_pls_family_result(&dataset, &recipe);
        legacy.provenance.method_version = legacy
            .provenance
            .method_version
            .replace(PLSC_METHOD_VERSION, PLSC_METHOD_VERSION_V1);
        let estimation = estimation_payload_mut(&mut legacy);
        estimation["method_version"] = serde_json::json!(PLSC_METHOD_VERSION_V1);
        estimation["plsc"]["method_version"] = serde_json::json!(PLSC_METHOD_VERSION_V1);

        let mut project = Project::new("Legacy PLSc persistence");
        project.datasets.push(dataset);
        project.recipes.push(recipe);
        project.results.push(legacy);
        save_project(&path, &project).unwrap();

        let restored = load_project(&path).unwrap();
        assert_eq!(
            estimation_payload_mut(&mut restored.results[0].clone())["method_version"].as_str(),
            Some(PLSC_METHOD_VERSION_V1)
        );
        assert!(
            restored.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "plsc.legacy_method_version")
        );
        assert!(restored.compatibility_notices.iter().any(|notice| {
            notice.result_id == restored.results[0].id
                && notice.diagnostic.code == "plsc.legacy_method_version"
                && notice.diagnostic.level == DiagnosticLevel::Warning
                && notice.diagnostic.message.contains(PLSC_METHOD_VERSION)
        }));

        save_project(&round_trip_path, &restored).unwrap();
        let reopened = load_project(&round_trip_path).unwrap();
        assert_eq!(
            reopened
                .compatibility_notices
                .iter()
                .filter(|notice| notice.diagnostic.code == "plsc.legacy_method_version")
                .count(),
            1
        );
        assert!(
            reopened.results[0]
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "plsc.legacy_method_version")
        );

        let mut mismatched_payload = reopened.results[0].clone();
        estimation_payload_mut(&mut mismatched_payload)["plsc"]["method_version"] =
            serde_json::json!(PLSC_METHOD_VERSION);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[mismatched_payload], &reopened.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut mismatched_provenance = reopened.results[0].clone();
        mismatched_provenance.provenance.method_version = mismatched_provenance
            .provenance
            .method_version
            .replace(PLSC_METHOD_VERSION_V1, PLSC_METHOD_VERSION);
        assert!(matches!(
            validate_result_contracts_with_recipes(&[mismatched_provenance], &reopened.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut unsupported = reopened.results[0].clone();
        unsupported.provenance.method_version = unsupported
            .provenance
            .method_version
            .replace(PLSC_METHOD_VERSION_V1, "plsc_v0");
        let estimation = estimation_payload_mut(&mut unsupported);
        estimation["method_version"] = serde_json::json!("plsc_v0");
        estimation["plsc"]["method_version"] = serde_json::json!("plsc_v0");
        assert!(matches!(
            validate_result_contracts_with_recipes(&[unsupported], &reopened.recipes),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn bootstrap_pls_payload_round_trips_with_recipe_provenance() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bootstrap.qpls");
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 8;
        recipe.settings.workers = 2;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsBootstrap);
        let mut base_recipe = recipe.clone();
        base_recipe.settings.bootstrap_samples = 0;
        base_recipe.method_config = Some(qpls_core::MethodConfig::PlsAlgorithm);
        let estimation = qpls_estimation::estimate_pls(&dataset, &base_recipe).unwrap();
        let assessment = qpls_assessment::assess_pls(&dataset, &base_recipe, &estimation).unwrap();
        let bootstrap = qpls_resampling::bootstrap_pls(
            &dataset,
            &recipe,
            &estimation,
            recipe.settings.workers,
            || false,
            |_| {},
        )
        .unwrap();
        let result = AnalysisResult::completed_pls_bootstrap(
            &recipe,
            &format!(
                "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+{RESAMPLING_METHOD_VERSION}"
            ),
            Utc::now(),
            serde_json::to_value(estimation).unwrap(),
            serde_json::to_value(assessment).unwrap(),
            serde_json::to_value(bootstrap).unwrap(),
            Vec::new(),
        );
        let mut project = Project::new("Bootstrap");
        project.datasets.push(dataset);
        project.recipes.push(recipe);
        project.results.push(result);
        save_project(&path, &project).unwrap();
        let restored = load_project(&path).unwrap();
        assert!(matches!(
            restored.results[0].payload,
            AnalysisPayload::PlsPmV2 { .. }
        ));
        assert_eq!(restored.results[0].provenance.settings.bootstrap_samples, 8);
        assert_eq!(restored.results[0].provenance.settings.workers, 2);

        let mut studentized_current = restored.results[0].clone();
        studentized_current.provenance.settings.bootstrap_samples = 999;
        studentized_current
            .provenance
            .settings
            .studentized_inner_samples = 99;
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut studentized_current.payload {
            bootstrap["plan"]["replicates"] = serde_json::json!(999);
            bootstrap["usable_replicates"] = serde_json::json!(999);
            let studentized_parameters = {
                let parameters = bootstrap["percentile"]["parameters"]
                    .as_array_mut()
                    .unwrap();
                for parameter in parameters.iter_mut() {
                    parameter["usable_replicates"] = serde_json::json!(999);
                }
                parameters
                    .iter()
                    .map(|parameter| {
                        let original = parameter["original"].as_f64().unwrap();
                        let standard_error = parameter["standard_error"].as_f64().unwrap();
                        if standard_error > 64.0 * f64::EPSILON * original.abs().max(1.0) {
                            let lower = parameter["lower"].as_f64().unwrap();
                            let upper = parameter["upper"].as_f64().unwrap();
                            serde_json::json!({
                                "parameter": parameter["parameter"],
                                "original": original,
                                "outer_standard_error": standard_error,
                                "outer_scale": original.abs().max(1.0),
                                "usable_primary_replicates": 999,
                                "lower_pivot": (original - upper) / standard_error,
                                "upper_pivot": (original - lower) / standard_error,
                                "lower": lower,
                                "upper": upper,
                                "unavailable_reason": null
                            })
                        } else {
                            serde_json::json!({
                                "parameter": parameter["parameter"],
                                "original": original,
                                "outer_standard_error": standard_error,
                                "outer_scale": original.abs().max(1.0),
                                "usable_primary_replicates": 999,
                                "lower_pivot": null,
                                "upper_pivot": null,
                                "lower": null,
                                "upper": null,
                                "unavailable_reason": "zero_outer_standard_error"
                            })
                        }
                    })
                    .collect::<Vec<_>>()
            };
            bootstrap["studentized"] = serde_json::json!({
                "method_version": STUDENTIZED_METHOD_VERSION,
                "confidence_level": 0.95,
                "inner_replicates": 99,
                "minimum_usable_fraction": 0.9,
                "stream_domain": "pls_pm_studentized_inner_v1",
                "parameters": studentized_parameters
            });
        }
        validate_result_contracts(&[studentized_current.clone()]).unwrap();

        let mut mislabeled_studentized = studentized_current.clone();
        mislabeled_studentized.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v3"
        );
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut mislabeled_studentized.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V3);
        }
        assert!(matches!(
            validate_result_contracts(&[mislabeled_studentized]),
            Err(ProjectError::Invalid(_))
        ));

        let mut insufficient_studentized = studentized_current.clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut insufficient_studentized.payload {
            bootstrap["studentized"]["parameters"][0]["usable_primary_replicates"] =
                serde_json::json!(899);
        }
        assert!(matches!(
            validate_result_contracts(&[insufficient_studentized]),
            Err(ProjectError::Invalid(_))
        ));

        let mut failed_studentized = studentized_current.clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut failed_studentized.payload {
            bootstrap["studentized"]["failure"] = serde_json::json!({
                "reason_code": "nested_infrastructure_failure",
                "first_primary_replicate": 7,
                "failed_primary_replicates": 1,
                "message": "inner estimate parameter schema mismatch"
            });
            bootstrap["studentized"]["parameters"] = serde_json::json!([]);
        }
        validate_result_contracts(&[failed_studentized.clone()]).unwrap();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut failed_studentized.payload {
            bootstrap["studentized"]["failure"]["failed_primary_replicates"] = serde_json::json!(0);
        }
        assert!(matches!(
            validate_result_contracts(&[failed_studentized]),
            Err(ProjectError::Invalid(_))
        ));

        let mut contradictory_reason = studentized_current;
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut contradictory_reason.payload {
            let parameter = &mut bootstrap["studentized"]["parameters"][0];
            parameter["lower_pivot"] = serde_json::Value::Null;
            parameter["upper_pivot"] = serde_json::Value::Null;
            parameter["lower"] = serde_json::Value::Null;
            parameter["upper"] = serde_json::Value::Null;
            parameter["unavailable_reason"] = serde_json::json!("insufficient_pivots");
        }
        assert!(matches!(
            validate_result_contracts(&[contradictory_reason]),
            Err(ProjectError::Invalid(_))
        ));

        let strip_rho_a = |assessment: &mut serde_json::Value| {
            assessment
                .as_object_mut()
                .unwrap()
                .remove("rho_a_method_version");
            for row in assessment["construct_quality"].as_array_mut().unwrap() {
                let row = row.as_object_mut().unwrap();
                for field in [
                    "rho_a",
                    "rho_a_status",
                    "rho_a_reason",
                    "rho_a_warning_codes",
                    "rho_a_indicator_count",
                    "score_variance_before_normalization",
                    "normalized_weight_norm_squared",
                    "off_diagonal_numerator",
                    "off_diagonal_denominator",
                ] {
                    row.remove(field);
                }
            }
        };
        let downgrade_htmt = |assessment: &mut serde_json::Value, retain_legacy: bool| {
            if retain_legacy {
                let constructs = assessment["htmt_plus"]["constructs"].clone();
                let values = assessment["htmt_plus"]["cells"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| {
                        serde_json::Value::Array(
                            row.as_array()
                                .unwrap()
                                .iter()
                                .map(|cell| cell["value"].clone())
                                .collect(),
                        )
                    })
                    .collect::<Vec<_>>();
                assessment["htmt"] = serde_json::json!({
                    "constructs": constructs,
                    "values": values,
                });
            }
            for field in [
                "htmt_plus_method_version",
                "htmt_plus",
                "htmt_original_method_version",
                "htmt_original",
            ] {
                assessment.as_object_mut().unwrap().remove(field);
            }
        };

        let mut legacy_assessment_v1 = restored.results[0].clone();
        legacy_assessment_v1.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v1+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v1.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V1);
            assessment.as_object_mut().unwrap().remove("htmt");
            downgrade_htmt(assessment, false);
            assessment
                .as_object_mut()
                .unwrap()
                .remove("structural_quality");
            assessment.as_object_mut().unwrap().remove("structural_vif");
            assessment
                .as_object_mut()
                .unwrap()
                .remove("formative_indicator_vif");
            assessment.as_object_mut().unwrap().remove("f_squared");
            assessment.as_object_mut().unwrap().remove("model_fit");
            assessment.as_object_mut().unwrap().remove("blindfolding");
            strip_rho_a(assessment);
        }
        validate_result_contracts(&[legacy_assessment_v1.clone()]).unwrap();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v1.payload {
            assessment["htmt"] = serde_json::json!({
                "constructs": ["x", "y"],
                "values": [[1.0, 0.5], [0.5, 1.0]],
            });
        }
        assert!(matches!(
            validate_result_contracts(&[legacy_assessment_v1]),
            Err(ProjectError::Invalid(_))
        ));

        let mut legacy_assessment_v2 = restored.results[0].clone();
        legacy_assessment_v2.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v2+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v2.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V2);
            downgrade_htmt(assessment, true);
            assessment.as_object_mut().unwrap().remove("f_squared");
            assessment.as_object_mut().unwrap().remove("model_fit");
            assessment.as_object_mut().unwrap().remove("blindfolding");
            strip_rho_a(assessment);
        }
        validate_result_contracts(&[legacy_assessment_v2]).unwrap();

        let mut legacy_assessment_v3 = restored.results[0].clone();
        legacy_assessment_v3.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v3+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v3.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V3);
            downgrade_htmt(assessment, true);
            assessment.as_object_mut().unwrap().remove("model_fit");
            assessment.as_object_mut().unwrap().remove("blindfolding");
            strip_rho_a(assessment);
        }
        validate_result_contracts(&[legacy_assessment_v3]).unwrap();

        let mut legacy_assessment_v4 = restored.results[0].clone();
        legacy_assessment_v4.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v4+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v4.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V4);
            downgrade_htmt(assessment, true);
            strip_rho_a(assessment);
        }
        validate_result_contracts(&[legacy_assessment_v4.clone()]).unwrap();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v4.payload {
            assessment["construct_quality"][0]["rho_a"] = serde_json::json!(0.8);
        }
        assert!(matches!(
            validate_result_contracts(&[legacy_assessment_v4]),
            Err(ProjectError::Invalid(_))
        ));

        let mut legacy_assessment_v5 = restored.results[0].clone();
        legacy_assessment_v5.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v5+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v5.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V5);
            downgrade_htmt(assessment, true);
        }
        validate_result_contracts_with_recipes(&[legacy_assessment_v5], &restored.recipes).unwrap();

        let mut mislabeled_effect_size = restored.results[0].clone();
        mislabeled_effect_size.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+pls_assessment_v2+{RESAMPLING_METHOD_VERSION}"
        );
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut mislabeled_effect_size.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V2);
            assessment.as_object_mut().unwrap().remove("model_fit");
            assessment.as_object_mut().unwrap().remove("blindfolding");
        }
        assert!(matches!(
            validate_result_contracts(&[mislabeled_effect_size]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_assessment = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut inconsistent_assessment.payload {
            assessment["structural_quality"][0]["predictor_count"] = serde_json::json!(99);
            assessment["structural_vif"][0]["vif"] = serde_json::json!(2.0);
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_assessment]),
            Err(ProjectError::Invalid(_))
        ));

        let mut mislabeled_htmt = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut mislabeled_htmt.payload {
            assessment["htmt_plus_method_version"] = serde_json::json!("unknown_htmt");
        }
        assert!(matches!(
            validate_result_contracts(&[mislabeled_htmt]),
            Err(ProjectError::Invalid(_))
        ));

        let mut asymmetric_htmt = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut asymmetric_htmt.payload {
            assessment["htmt_plus"]["cells"][0][1]["value"] = serde_json::json!(0.25);
        }
        assert!(matches!(
            validate_result_contracts(&[asymmetric_htmt]),
            Err(ProjectError::Invalid(_))
        ));

        let mut forged_htmt_semantics = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut forged_htmt_semantics.payload {
            assessment["htmt_original"]["absolute_correlations"] = serde_json::json!(true);
        }
        assert!(matches!(
            validate_result_contracts(&[forged_htmt_semantics]),
            Err(ProjectError::Invalid(_))
        ));

        let mut unavailable_htmt_diagonal = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut unavailable_htmt_diagonal.payload
        {
            assessment["htmt_plus"]["cells"][0][0] = serde_json::json!({
                "value": null,
                "status": "unavailable",
                "reason": "htmt.zero_monotrait_denominator"
            });
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[unavailable_htmt_diagonal], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut negative_htmt_plus = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut negative_htmt_plus.payload {
            assessment["htmt_plus"]["cells"][0][1]["value"] = serde_json::json!(-1e-15);
            assessment["htmt_plus"]["cells"][1][0]["value"] = serde_json::json!(-1e-15);
        }
        assert!(matches!(
            validate_result_contracts(&[negative_htmt_plus]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_rho_a = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut inconsistent_rho_a.payload {
            assessment["construct_quality"][0]["rho_a"] = serde_json::json!(0.123);
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_rho_a]),
            Err(ProjectError::Invalid(_))
        ));

        let mut forged_rho_a_status = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut forged_rho_a_status.payload {
            let row = &mut assessment["construct_quality"][0];
            row["rho_a"] = serde_json::Value::Null;
            row["rho_a_status"] = serde_json::json!("not_applicable");
            row["rho_a_reason"] = serde_json::json!("rho_a.formative_not_applicable");
            row["rho_a_warning_codes"] = serde_json::json!([]);
            row["score_variance_before_normalization"] = serde_json::Value::Null;
            row["normalized_weight_norm_squared"] = serde_json::Value::Null;
            row["off_diagonal_numerator"] = serde_json::Value::Null;
            row["off_diagonal_denominator"] = serde_json::Value::Null;
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[forged_rho_a_status], &restored.recipes,),
            Err(ProjectError::Invalid(_))
        ));

        let mut reordered_quality = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut reordered_quality.payload {
            assessment["construct_quality"]
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[reordered_quality], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut duplicate_quality = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut duplicate_quality.payload {
            let rows = assessment["construct_quality"].as_array_mut().unwrap();
            let duplicate = rows[0].clone();
            rows.push(duplicate);
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[duplicate_quality], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut missing_quality = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut missing_quality.payload {
            assessment["construct_quality"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[missing_quality], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        let mut incorrect_rho_a_warning = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut incorrect_rho_a_warning.payload {
            assessment["construct_quality"][0]["rho_a_warning_codes"] =
                serde_json::json!(["rho_a.improper_above_one"]);
        }
        assert!(matches!(
            validate_result_contracts_with_recipes(&[incorrect_rho_a_warning], &restored.recipes,),
            Err(ProjectError::Invalid(_))
        ));

        let mut mismatched_settings = restored.results[0].clone();
        mismatched_settings.provenance.settings.tolerance *= 10.0;
        assert!(matches!(
            validate_result_contracts_with_recipes(&[mismatched_settings], &restored.recipes),
            Err(ProjectError::Invalid(_))
        ));

        assert!(matches!(
            validate_result_contracts_with_recipes(&restored.results, &[]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_effect_size = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut inconsistent_effect_size.payload {
            assessment["f_squared"][0]["f_squared"] = serde_json::json!(999.0);
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_effect_size]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_r_squared = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 {
            estimation,
            assessment,
            ..
        } = &mut inconsistent_r_squared.payload
        {
            let n = estimation["used_observations"].as_u64().unwrap() as f64;
            let replacement = 0.5;
            assessment["r_squared"]["y"] = serde_json::json!(replacement);
            assessment["structural_quality"][0]["r_squared"] = serde_json::json!(replacement);
            assessment["structural_quality"][0]["adjusted_r_squared"] =
                serde_json::json!(1.0 - (1.0 - replacement) * (n - 1.0) / (n - 2.0));
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_r_squared]),
            Err(ProjectError::Invalid(_))
        ));

        let mut legacy_v1 = restored.results[0].clone();
        legacy_v1.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v1"
        );
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut legacy_v1.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V1);
            bootstrap.as_object_mut().unwrap().remove("bca");
            for (index, parameter) in bootstrap["percentile"]["parameters"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                parameter["parameter"] = serde_json::json!(format!("legacy:{index}"));
                parameter.as_object_mut().unwrap().remove("t_statistic");
                parameter
                    .as_object_mut()
                    .unwrap()
                    .remove("p_value_two_sided");
            }
        }
        validate_result_contracts(&[legacy_v1]).unwrap();

        let mut legacy_v2 = restored.results[0].clone();
        legacy_v2.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v2"
        );
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut legacy_v2.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V2);
            bootstrap.as_object_mut().unwrap().remove("bca");
        }
        validate_result_contracts(&[legacy_v2]).unwrap();

        let mut legacy_v3 = restored.results[0].clone();
        legacy_v3.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v3"
        );
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut legacy_v3.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V3);
            bootstrap.as_object_mut().unwrap().remove("studentized");
        }
        validate_result_contracts(&[legacy_v3]).unwrap();

        let mut mislabeled_bca = restored.results[0].clone();
        mislabeled_bca.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v2"
        );
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut mislabeled_bca.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V2);
        }
        assert!(matches!(
            validate_result_contracts(&[mislabeled_bca]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_bca_count = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut inconsistent_bca_count.payload {
            bootstrap["bca"]["jackknife_case_count"] = serde_json::json!(999);
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_bca_count]),
            Err(ProjectError::Invalid(_))
        ));

        let mut partial_bca_row = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut partial_bca_row.payload {
            bootstrap["bca"]["parameters"][0]["lower"] = serde_json::Value::Null;
        }
        assert!(matches!(
            validate_result_contracts(&[partial_bca_row]),
            Err(ProjectError::Invalid(_))
        ));

        let mut mismatched_version = restored.results[0].clone();
        mismatched_version.provenance.method_version = format!(
            "pls_pm_v1+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v1"
        );
        assert!(matches!(
            validate_result_contracts(&[mismatched_version]),
            Err(ProjectError::Invalid(_))
        ));

        let mut incomplete_test = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut incomplete_test.payload {
            bootstrap["percentile"]["parameters"][0]["p_value_two_sided"] = serde_json::Value::Null;
        }
        assert!(matches!(
            validate_result_contracts(&[incomplete_test]),
            Err(ProjectError::Invalid(_))
        ));

        let mut inconsistent_test = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut inconsistent_test.payload {
            bootstrap["percentile"]["parameters"][0]["t_statistic"] = serde_json::json!(0.0);
            bootstrap["percentile"]["parameters"][0]["p_value_two_sided"] = serde_json::json!(1.0);
        }
        assert!(matches!(
            validate_result_contracts(&[inconsistent_test]),
            Err(ProjectError::Invalid(_))
        ));

        let mut malformed = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut malformed.payload {
            let parameters = bootstrap["percentile"]["parameters"]
                .as_array_mut()
                .unwrap();
            parameters.push(parameters[0].clone());
        }
        assert!(matches!(
            validate_result_contracts(&[malformed]),
            Err(ProjectError::Invalid(_))
        ));

        let mut insufficient = restored.results[0].clone();
        insufficient.provenance.settings.bootstrap_samples = 1;
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut insufficient.payload {
            bootstrap["plan"]["replicates"] = serde_json::json!(1);
            bootstrap["usable_replicates"] = serde_json::json!(1);
            for parameter in bootstrap["percentile"]["parameters"]
                .as_array_mut()
                .unwrap()
            {
                parameter["usable_replicates"] = serde_json::json!(1);
            }
        }
        assert!(matches!(
            validate_result_contracts(&[insufficient]),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn permutation_pls_multi_path_appends_round_trips_and_rejects_manifest_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("permutation.qpls");
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/corporate_reputation.csv"),
            "corporate_reputation.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/corporate_reputation.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.permutation_samples = 99;
        recipe.settings.workers = 2;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsPermutation);
        let mut base_recipe = recipe.clone();
        base_recipe.settings.permutation_samples = 0;
        base_recipe.method_config = Some(qpls_core::MethodConfig::PlsAlgorithm);
        let estimation = qpls_estimation::estimate_pls(&dataset, &base_recipe).unwrap();
        let assessment = qpls_assessment::assess_pls(&dataset, &base_recipe, &estimation).unwrap();
        let permutation = qpls_resampling::permutation_pls(
            &dataset,
            &recipe,
            &estimation,
            recipe.settings.workers,
            || false,
            |_| {},
        )
        .unwrap();
        let expected_parameter_order = estimation
            .paths
            .iter()
            .map(|path| {
                serde_json::to_string(&("path", [path.source.as_str(), path.target.as_str()]))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            permutation
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.clone())
                .collect::<Vec<_>>(),
            expected_parameter_order
        );
        let result = AnalysisResult::completed_pls_inference(
            &recipe,
            format!(
                "{PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+{PERMUTATION_METHOD_VERSION}"
            ),
            Utc::now(),
            serde_json::to_value(estimation).unwrap(),
            serde_json::to_value(assessment).unwrap(),
            None,
            Some(serde_json::to_value(permutation).unwrap()),
            Vec::new(),
        );
        let mut project = Project::new("Permutation");
        project.datasets.push(dataset.clone());
        project
            .append_validated_result(recipe.clone(), result.clone())
            .unwrap();
        save_project(&path, &project).unwrap();
        let restored = load_project(&path).unwrap();
        assert_eq!(restored.recipes, vec![recipe.clone()]);
        assert_eq!(restored.results.len(), 1);
        assert_eq!(restored.results[0].id, result.id);
        assert_eq!(restored.results[0].provenance, result.provenance);
        assert!(matches!(
            &restored.results[0].payload,
            AnalysisPayload::PlsPmV3 {
                bootstrap: None,
                permutation: Some(_),
                ..
            }
        ));
        let restored_permutation = match &restored.results[0].payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => serde_json::from_value::<PlsPermutationResult>(permutation.clone()).unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(
            restored_permutation
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.clone())
                .collect::<Vec<_>>(),
            expected_parameter_order
        );
        let original_permutation = match &result.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => serde_json::from_value::<PlsPermutationResult>(permutation.clone()).unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(restored_permutation, original_permutation);

        let reject_atomically = |tampered: AnalysisResult| {
            let mut rejected = Project::new("Rejected permutation");
            rejected.datasets.push(dataset.clone());
            assert!(matches!(
                rejected.append_validated_result(recipe.clone(), tampered),
                Err(ProjectError::Invalid(_))
            ));
            assert!(rejected.recipes.is_empty());
            assert!(rejected.results.is_empty());
        };

        let mut missing = result.clone();
        if let AnalysisPayload::PlsPmV3 { permutation, .. } = &mut missing.payload {
            *permutation = None;
        }
        reject_atomically(missing);

        let mut reordered = result.clone();
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(permutation),
            ..
        } = &mut reordered.payload
        {
            permutation["parameters"].as_array_mut().unwrap().swap(0, 1);
        }
        reject_atomically(reordered);

        let mut altered_identity = result.clone();
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(permutation),
            ..
        } = &mut altered_identity.payload
        {
            permutation["parameters"][0]["parameter"] =
                serde_json::json!("[\"path\",[\"forged\",\"target\"]]");
        }
        reject_atomically(altered_identity);

        let mut reordered_estimation_and_parameters = result.clone();
        if let AnalysisPayload::PlsPmV3 {
            estimation,
            permutation: Some(permutation),
            ..
        } = &mut reordered_estimation_and_parameters.payload
        {
            estimation["paths"].as_array_mut().unwrap().swap(0, 1);
            permutation["parameters"].as_array_mut().unwrap().swap(0, 1);
        }
        reject_atomically(reordered_estimation_and_parameters);

        let mut tampered = result.clone();
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(permutation),
            ..
        } = &mut tampered.payload
        {
            let probability = permutation["parameters"][0]["p_value_two_sided"]
                .as_f64()
                .unwrap();
            permutation["parameters"][0]["p_value_two_sided"] =
                serde_json::json!(f64::from_bits(probability.to_bits() + 1));
        }
        reject_atomically(tampered);

        let mut altered_original = result.clone();
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(permutation),
            ..
        } = &mut altered_original.payload
        {
            permutation["parameters"][0]["original"] = serde_json::json!(99.0);
        }
        reject_atomically(altered_original);

        for wrapper in ["result", "plan", "parameter"] {
            let mut unknown = result.clone();
            if let AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } = &mut unknown.payload
            {
                match wrapper {
                    "result" => permutation["undeclared"] = serde_json::json!(true),
                    "plan" => permutation["plan"]["undeclared"] = serde_json::json!(true),
                    "parameter" => {
                        permutation["parameters"][0]["undeclared"] = serde_json::json!(true)
                    }
                    _ => unreachable!(),
                }
            }
            reject_atomically(unknown);
        }

        let checksum_recomputed = directory.path().join("permutation-unknown-plan.qpls");
        fs::copy(&path, &checksum_recomputed).unwrap();
        rewrite_zip_entry_with_manifest_checksum(
            &checksum_recomputed,
            PROJECT_ENTRY_NAME,
            |bytes| {
                let mut document: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                document["results"][0]["payload"]["permutation"]["plan"]["undeclared"] =
                    serde_json::json!(true);
                serde_json::to_vec_pretty(&document).unwrap()
            },
        );
        assert!(matches!(
            load_project(&checksum_recomputed),
            Err(ProjectError::Invalid(_))
        ));
    }
    #[test]
    fn changed_payload_is_rejected_by_its_manifest_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("changed.qpls");
        save_project(&path, &Project::new("Checksum")).unwrap();
        rewrite_zip_entry(&path, "project.json", |_| {
            br#"{"datasets":[],"models":[],"recipes":[],"layouts":{"changed":true}}"#.to_vec()
        });
        assert!(
            matches!(load_project(&path), Err(ProjectError::ChecksumMismatch(name)) if name == "project.json")
        );
    }

    fn zip_entry_bytes(path: &Path, name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entry = archive.by_name(name).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        bytes
    }

    fn set_archive_schema_version(path: &Path, schema_version: u32) {
        rewrite_zip_entry(path, "manifest.json", |bytes| {
            let mut manifest: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            manifest["schema_version"] = serde_json::json!(schema_version);
            serde_json::to_vec_pretty(&manifest).unwrap()
        });
    }

    fn rewrite_zip_entry(path: &Path, target: &str, transform: impl FnOnce(&[u8]) -> Vec<u8>) {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            entries.push((entry.name().to_owned(), bytes));
        }
        drop(archive);
        let replacement = path.with_extension("rewrite");
        let mut writer = ZipWriter::new(File::create(&replacement).unwrap());
        let mut transform = Some(transform);
        for (name, bytes) in entries {
            writer
                .start_file(&name, SimpleFileOptions::default())
                .unwrap();
            let bytes = if name == target {
                transform.take().unwrap()(&bytes)
            } else {
                bytes
            };
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        fs::remove_file(path).unwrap();
        fs::rename(replacement, path).unwrap();
    }

    fn rewrite_zip_entry_with_manifest_checksum(
        path: &Path,
        target: &str,
        transform: impl FnOnce(&[u8]) -> Vec<u8>,
    ) {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            entries.push((entry.name().to_owned(), bytes));
        }
        drop(archive);

        let target_index = entries.iter().position(|(name, _)| name == target).unwrap();
        entries[target_index].1 = transform(&entries[target_index].1);
        let target_checksum = sha256(&entries[target_index].1);
        let manifest_index = entries
            .iter()
            .position(|(name, _)| name == "manifest.json")
            .unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&entries[manifest_index].1).unwrap();
        manifest["checksums"][target] = serde_json::json!(target_checksum);
        entries[manifest_index].1 = serde_json::to_vec_pretty(&manifest).unwrap();

        let replacement = path.with_extension("rewrite-with-checksum");
        let mut writer = ZipWriter::new(File::create(&replacement).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        fs::remove_file(path).unwrap();
        fs::rename(replacement, path).unwrap();
    }
}
