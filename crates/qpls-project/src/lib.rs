use chrono::{DateTime, Utc};
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, ASSESSMENT_METHOD_VERSION_V1, ASSESSMENT_METHOD_VERSION_V2,
    ASSESSMENT_METHOD_VERSION_V3, ASSESSMENT_METHOD_VERSION_V4, ASSESSMENT_METHOD_VERSION_V5,
    ASSESSMENT_METHOD_VERSION_V6, AssessmentResult, HTMT_ORIGINAL_METHOD_VERSION,
    HTMT_PLUS_METHOD_VERSION, HtmtAssessment, HtmtStatus, RHO_A_METHOD_VERSION, RhoAStatus,
    variance_inflation_factor,
};
use qpls_core::{
    AnalysisMethod, AnalysisPayload, AnalysisRecipe, AnalysisResult, AnalysisSettings, Diagnostic,
    DiagnosticLevel, ENGINE_VERSION, ModelSpec, RESULT_SCHEMA_VERSION, RunProvenance, RunStatus,
};
use qpls_data::{Dataset, DatasetDescriptor, dataset_from_descriptor, write_arrow};
use qpls_estimation::{
    CCA_METHOD_VERSION, CTA_PLS_METHOD_VERSION, GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION,
    MODERATED_MEDIATION_METHOD_VERSION, NONLINEAR_EFFECTS_METHOD_VERSION, PLS_METHOD_VERSION,
    PLSC_METHOD_VERSION, PlsResult, WPLS_METHOD_VERSION,
};
use qpls_resampling::{
    PERMUTATION_METHOD_VERSION, PlsBootstrapResult, PlsPermutationResult,
    RESAMPLING_METHOD_VERSION, RESAMPLING_METHOD_VERSION_V1, RESAMPLING_METHOD_VERSION_V2,
    RESAMPLING_METHOD_VERSION_V3, STUDENTIZED_METHOD_VERSION, normal_reference_test,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const PROJECT_ARCHIVE_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub engine_version: String,
    pub checksums: BTreeMap<String, String>,
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
                checksums: BTreeMap::new(),
            },
            datasets: vec![],
            models: vec![],
            recipes: vec![],
            layouts: BTreeMap::new(),
            results: Vec::new(),
            read_only: false,
        }
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
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("data failed: {0}")]
    Data(#[from] qpls_data::DataError),
    #[error("JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP failed: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub fn save_project(path: &Path, project: &Project) -> Result<(), ProjectError> {
    if project.read_only {
        return Err(ProjectError::ReadOnly);
    }
    validate_result_contracts_with_recipes(&project.results, &project.recipes)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut entries = BTreeMap::<String, Vec<u8>>::new();
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
    entries.insert("project.json".into(), serde_json::to_vec_pretty(&document)?);
    for dataset in &project.datasets {
        entries.insert(
            format!("data/{}.arrow", dataset.id),
            write_arrow(&dataset.batch)?,
        );
    }
    let mut manifest = project.manifest.clone();
    manifest.schema_version = PROJECT_ARCHIVE_VERSION;
    manifest.modified_at = Utc::now();
    manifest.engine_version = ENGINE_VERSION.into();
    manifest.checksums = entries
        .iter()
        .map(|(name, bytes)| (name.clone(), sha256(bytes)))
        .collect();
    let temporary = temporary_path(path);
    let file = File::create(&temporary)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    for (name, bytes) in entries {
        zip.start_file(name, options)?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?.sync_all()?;
    if path.exists() {
        let backup = backup_path(path);
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::rename(&backup, path);
            return Err(error.into());
        }
    } else {
        fs::rename(temporary, path)?;
    }
    Ok(())
}

pub fn load_project(path: &Path) -> Result<Project, ProjectError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut manifest: ProjectManifest =
        serde_json::from_slice(&read_entry(&mut archive, "manifest.json")?)?;
    let future = manifest.schema_version > PROJECT_ARCHIVE_VERSION;
    let project_bytes = verified_entry(&mut archive, &manifest, "project.json")?;
    let document = migrate_document(manifest.schema_version, &project_bytes)?;
    if !future {
        manifest.schema_version = PROJECT_ARCHIVE_VERSION;
    }
    let mut datasets = Vec::with_capacity(document.datasets.len());
    for descriptor in document.datasets {
        let name = format!("data/{}.arrow", descriptor.id);
        let bytes = verified_entry(&mut archive, &manifest, &name)?;
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
    })
}

fn migrate_document(schema_version: u32, bytes: &[u8]) -> Result<ProjectDocument, ProjectError> {
    if schema_version == 0 {
        return Err(ProjectError::Invalid(
            "archive schema version 0 is unsupported".into(),
        ));
    }
    if schema_version >= 4 {
        let document: ProjectDocument = serde_json::from_slice(bytes)?;
        validate_result_contracts_with_recipes(&document.results, &document.recipes)?;
        return Ok(document);
    }
    if schema_version == 3 {
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
        return Ok(ProjectDocument {
            datasets: legacy.datasets,
            models: legacy.models,
            recipes: legacy.recipes,
            layouts: legacy.layouts,
            results,
        });
    }
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
    Ok(ProjectDocument {
        datasets: legacy.datasets,
        models: legacy.models,
        recipes: legacy.recipes,
        layouts: legacy.layouts,
        results,
    })
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

fn validate_result_contracts_internal(
    results: &[AnalysisResult],
    recipes: &[AnalysisRecipe],
    require_recipe_context: bool,
) -> Result<(), ProjectError> {
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
        if result.provenance.method != AnalysisMethod::PlsPm {
            return Err(ProjectError::Invalid(format!(
                "result {} has a PLS payload but method {}",
                result.id, result.provenance.method
            )));
        }
        let estimation: PlsResult =
            serde_json::from_value(estimation.clone()).map_err(|error| {
                ProjectError::Invalid(format!(
                    "result {} has an invalid PLS estimation payload: {error}",
                    result.id
                ))
            })?;
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
            Some(recipe)
        };
        let supported_assessment = assessment.method_version == ASSESSMENT_METHOD_VERSION
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V6
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V5
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V4
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V3
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V2
            || assessment.method_version == ASSESSMENT_METHOD_VERSION_V1;
        let envelope_has_assessment_version = result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == assessment.method_version);
        let supported_estimation = estimation.method_version == PLS_METHOD_VERSION
            || estimation.method_version == PLSC_METHOD_VERSION
            || estimation.method_version == GAUSSIAN_COPULA_ENDOGENEITY_METHOD_VERSION
            || estimation.method_version == NONLINEAR_EFFECTS_METHOD_VERSION
            || estimation.method_version == MODERATED_MEDIATION_METHOD_VERSION
            || estimation.method_version == CTA_PLS_METHOD_VERSION
            || estimation.method_version == WPLS_METHOD_VERSION
            || estimation.method_version == CCA_METHOD_VERSION;
        if !supported_estimation || !supported_assessment || !envelope_has_assessment_version {
            return Err(ProjectError::Invalid(format!(
                "result {} uses unsupported PLS payload versions",
                result.id
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
            let parameter_names = permutation
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.as_str())
                .collect::<std::collections::HashSet<_>>();
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
                .collect::<std::collections::HashMap<_, _>>();
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
                || parameter_names.len() != permutation.parameters.len()
                || parameter_names.len() != expected.len()
                || permutation.parameters.iter().any(|parameter| {
                    let expected_original = expected.get(&parameter.parameter).copied();
                    let expected_probability = (parameter.exceedances as f64 + 1.0)
                        / (permutation.plan.permutations as f64 + 1.0);
                    expected_original.is_none()
                        || !parameter.original.is_finite()
                        || !approximately_equal(
                            parameter.original,
                            expected_original.unwrap_or_default(),
                            1e-12,
                        )
                        || parameter.permutations != permutation.plan.permutations
                        || parameter.exceedances > permutation.plan.permutations
                        || !parameter.p_value_two_sided.is_finite()
                        || !(0.0..=1.0).contains(&parameter.p_value_two_sided)
                        || !approximately_equal(
                            parameter.p_value_two_sided,
                            expected_probability,
                            1e-12,
                        )
                })
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
                let expected_reason = if left.mode == qpls_core::MeasurementMode::Formative
                    || right.mode == qpls_core::MeasurementMode::Formative
                {
                    Some("htmt.formative_not_applicable")
                } else if left.indicators.len() < 2 || right.indicators.len() < 2 {
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
            let expected_indicator_count = estimation
                .outer_estimates
                .iter()
                .filter(|outer| outer.construct == row.construct)
                .count();
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
                    } else if construct.indicators.len() == 1 {
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
    match load_project(path) {
        Ok(project) => Ok((project, false)),
        Err(primary_error) => {
            let backup = backup_path(path);
            if !backup.exists() {
                return Err(primary_error);
            }
            load_project(&backup)
                .map(|project| (project, true))
                .map_err(|_| primary_error)
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
    let autosave = autosave_path(path);
    match load_project(path) {
        Ok(primary) => {
            if autosave.exists()
                && let Ok(autosaved) = load_project(&autosave)
                && autosaved.manifest.modified_at > primary.manifest.modified_at
            {
                return Ok((autosaved, Some(RecoverySource::Autosave)));
            }
            Ok((primary, None))
        }
        Err(primary_error) => {
            if autosave.exists()
                && let Ok(autosaved) = load_project(&autosave)
            {
                return Ok((autosaved, Some(RecoverySource::Autosave)));
            }
            let backup = backup_path(path);
            if backup.exists() {
                return load_project(&backup)
                    .map(|project| (project, Some(RecoverySource::Backup)))
                    .map_err(|_| primary_error);
            }
            Err(primary_error)
        }
    }
}

pub fn save_autosave(path: &Path, project: &Project) -> Result<(), ProjectError> {
    save_project(&autosave_path(path), project)
}

pub fn discard_autosave(path: &Path) -> Result<(), ProjectError> {
    let autosave = autosave_path(path);
    if autosave.exists() {
        fs::remove_file(autosave)?;
    }
    let backup = backup_path(&autosave_path(path));
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}

fn verified_entry(
    archive: &mut ZipArchive<File>,
    manifest: &ProjectManifest,
    name: &str,
) -> Result<Vec<u8>, ProjectError> {
    let bytes = read_entry(archive, name)?;
    let expected = manifest
        .checksums
        .get(name)
        .ok_or_else(|| ProjectError::MissingEntry(format!("checksum for {name}")))?;
    if sha256(&bytes) != *expected {
        return Err(ProjectError::ChecksumMismatch(name.into()));
    }
    Ok(bytes)
}
fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, ProjectError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| ProjectError::MissingEntry(name.into()))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}
fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("qpls.tmp-{}", Uuid::new_v4()))
}
pub fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("qpls.bak")
}
pub fn autosave_path(path: &Path) -> PathBuf {
    path.with_extension("qpls.autosave")
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_data::{ImportOptions, import_delimited_bytes};
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
        let replacement = Project::new("Second");
        save_project(&path, &replacement).unwrap();
        fs::write(&path, b"interrupted write").unwrap();
        let (recovered, used_backup) = load_project_with_recovery(&path).unwrap();
        assert!(used_backup);
        assert_eq!(recovered.manifest.name, "First");
    }
    #[test]
    fn valid_autosave_takes_precedence_and_can_be_discarded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("study.qpls");
        let primary = Project::new("Primary");
        save_project(&path, &primary).unwrap();
        let autosaved = Project::new("Recovered work");
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
        save_project(&path, &Project::new("Initial")).unwrap();
        save_autosave(&path, &Project::new("Stale autosave")).unwrap();
        save_project(&path, &Project::new("Explicit save")).unwrap();
        let (restored, source) = load_project_with_autosave(&path).unwrap();
        assert_eq!(restored.manifest.name, "Explicit save");
        assert_eq!(source, None);
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
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 8;
        recipe.settings.workers = 2;
        let mut base_recipe = recipe.clone();
        base_recipe.settings.bootstrap_samples = 0;
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
            &format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+{RESAMPLING_METHOD_VERSION}"),
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
        mislabeled_studentized.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v3");
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
        legacy_assessment_v1.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v1+{RESAMPLING_METHOD_VERSION}");
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
        legacy_assessment_v2.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v2+{RESAMPLING_METHOD_VERSION}");
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
        legacy_assessment_v3.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v3+{RESAMPLING_METHOD_VERSION}");
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v3.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V3);
            downgrade_htmt(assessment, true);
            assessment.as_object_mut().unwrap().remove("model_fit");
            assessment.as_object_mut().unwrap().remove("blindfolding");
            strip_rho_a(assessment);
        }
        validate_result_contracts(&[legacy_assessment_v3]).unwrap();

        let mut legacy_assessment_v4 = restored.results[0].clone();
        legacy_assessment_v4.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v4+{RESAMPLING_METHOD_VERSION}");
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
        legacy_assessment_v5.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v5+{RESAMPLING_METHOD_VERSION}");
        if let AnalysisPayload::PlsPmV2 { assessment, .. } = &mut legacy_assessment_v5.payload {
            assessment["method_version"] = serde_json::json!(ASSESSMENT_METHOD_VERSION_V5);
            downgrade_htmt(assessment, true);
        }
        validate_result_contracts_with_recipes(&[legacy_assessment_v5], &restored.recipes).unwrap();

        let mut mislabeled_effect_size = restored.results[0].clone();
        mislabeled_effect_size.provenance.method_version =
            format!("pls_pm_v1+pls_assessment_v2+{RESAMPLING_METHOD_VERSION}");
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
        legacy_v1.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v1");
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
        legacy_v2.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v2");
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut legacy_v2.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V2);
            bootstrap.as_object_mut().unwrap().remove("bca");
        }
        validate_result_contracts(&[legacy_v2]).unwrap();

        let mut legacy_v3 = restored.results[0].clone();
        legacy_v3.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v3");
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut legacy_v3.payload {
            bootstrap["method_version"] = serde_json::json!(RESAMPLING_METHOD_VERSION_V3);
            bootstrap.as_object_mut().unwrap().remove("studentized");
        }
        validate_result_contracts(&[legacy_v3]).unwrap();

        let mut mislabeled_bca = restored.results[0].clone();
        mislabeled_bca.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v2");
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
        mismatched_version.provenance.method_version =
            format!("pls_pm_v1+{ASSESSMENT_METHOD_VERSION}+indexed_resampling_v1");
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
    fn permutation_pls_payload_round_trips_and_rejects_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("permutation.qpls");
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.permutation_samples = 99;
        recipe.settings.workers = 2;
        let mut base_recipe = recipe.clone();
        base_recipe.settings.permutation_samples = 0;
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
        let result = AnalysisResult::completed_pls_inference(
            &recipe,
            format!(
                "{PLS_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}+{PERMUTATION_METHOD_VERSION}"
            ),
            Utc::now(),
            serde_json::to_value(estimation).unwrap(),
            serde_json::to_value(assessment).unwrap(),
            None,
            Some(serde_json::to_value(permutation).unwrap()),
            Vec::new(),
        );
        let mut project = Project::new("Permutation");
        project.datasets.push(dataset);
        project.recipes.push(recipe);
        project.results.push(result);
        save_project(&path, &project).unwrap();
        let restored = load_project(&path).unwrap();
        assert!(matches!(
            &restored.results[0].payload,
            AnalysisPayload::PlsPmV3 {
                bootstrap: None,
                permutation: Some(_),
                ..
            }
        ));

        let mut missing = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV3 { permutation, .. } = &mut missing.payload {
            *permutation = None;
        }
        assert!(matches!(
            validate_result_contracts(&[missing]),
            Err(ProjectError::Invalid(_))
        ));

        let mut tampered = restored.results[0].clone();
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(permutation),
            ..
        } = &mut tampered.payload
        {
            permutation["parameters"][0]["p_value_two_sided"] = serde_json::json!(0.75);
        }
        assert!(matches!(
            validate_result_contracts(&[tampered]),
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
}
