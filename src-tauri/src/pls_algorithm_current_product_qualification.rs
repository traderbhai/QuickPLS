//! Internal, ignored current-product PLS work-evidence harness.
//!
//! This module is compiled only into the `quickpls-desktop` test executable.
//! It deliberately has no Tauri command, customer CLI entry point, Standard
//! surface, Registry mutation, receipt writer, or promotion authority.

use super::{
    InternalRecipeV4ExecutionSurfaceV1, InternalRecipeV4PlsExecutionRequestV1,
    InternalRecipeV4ResidentDataV1, execute_internal_recipe_v4_pls,
    resolve_internal_recipe_v4_dataset,
};
use crate::recipe_v4_canonical_result::build_recipe_v4_pls_canonical_result;
use chrono::{TimeZone, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4,
    AnalysisSettings, CapabilityCellReferenceV2, Construct, LegacyBasicModelInterpretationV4,
    MeasurementMode, MethodConfig, ModelSpec, RecipeV4CompilerTarget, SemDataBindingV4,
    StructuralPath, WeightingScheme, compile_analysis_recipe_v4, confirm_legacy_recipe_estimand_v4,
    migrate_analysis_recipe_to_v4_pending, sha256_serialized,
};
use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
use qpls_estimation::PLS_METHOD_VERSION;
use qpls_project::{
    CanonicalResultDocumentV2, Project, ProjectArchiveCanonicalAppendReceiptV6,
    ProjectArchiveUpgradeRequestV6, ProjectArchiveWriteReceiptV6, ProjectModelPayloadV6,
    ProjectModelRecordV6, append_canonical_result_document_v2_file_v6, plan_project_upgrade_to_v6,
    read_project_document_v6, write_project_document_v6_new,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const ACTIVATION_ENV: &str = "QPLS_INTERNAL_PLS_QUALIFICATION";
const OUTPUT_ENV: &str = "QPLS_PLS_QUALIFICATION_OUTPUT_DIR";
const SOURCE_SET_ENV: &str = "QPLS_PLS_QUALIFICATION_SOURCE_SET_SHA256";
const SCENARIO_SET_ENV: &str = "QPLS_PLS_QUALIFICATION_SCENARIO_SET_SHA256";
const COMPILED_SOURCE_SET_SHA256: Option<&str> =
    option_env!("QPLS_PLS_QUALIFICATION_SOURCE_SET_SHA256");
const COMPILED_SCENARIO_SET_SHA256: Option<&str> =
    option_env!("QPLS_PLS_QUALIFICATION_SCENARIO_SET_SHA256");

#[derive(Clone)]
struct VariantDefinition {
    id: &'static str,
    weighting_scheme: WeightingScheme,
    measurement_mode: MeasurementMode,
}

const VARIANTS: [VariantDefinition; 4] = [
    VariantDefinition {
        id: "path_mode_a",
        weighting_scheme: WeightingScheme::Path,
        measurement_mode: MeasurementMode::Reflective,
    },
    VariantDefinition {
        id: "path_mode_b",
        weighting_scheme: WeightingScheme::Path,
        measurement_mode: MeasurementMode::Formative,
    },
    VariantDefinition {
        id: "factor_mode_a",
        weighting_scheme: WeightingScheme::Factor,
        measurement_mode: MeasurementMode::Reflective,
    },
    VariantDefinition {
        id: "pca_mode_a",
        weighting_scheme: WeightingScheme::Pca,
        measurement_mode: MeasurementMode::Reflective,
    },
];

struct VariantCase {
    definition: VariantDefinition,
    request: InternalRecipeV4PlsExecutionRequestV1,
    compiled: qpls_core::CompiledAnalysisRecipeV4,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactDescriptor {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutableIdentity {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct CompiledIdentity {
    source_set_sha256: String,
    scenario_set_sha256: String,
    executable: ExecutableIdentity,
}

#[derive(Debug, Serialize)]
struct VariantReport {
    variant: String,
    weighting_scheme: String,
    measurement_mode: String,
    recipe_v4_compiled: bool,
    real_runner_executed: bool,
    product_canonical_builder_executed: bool,
    schema6_append_reopened: bool,
    method_version: String,
    compilation_receipt_sha256: String,
    request: ArtifactDescriptor,
    compiled_artifact: ArtifactDescriptor,
    execution: ArtifactDescriptor,
    canonical_document: ArtifactDescriptor,
    append_receipt: ProjectArchiveCanonicalAppendReceiptV6,
    archive_after_append: ArtifactDescriptor,
}

#[derive(Debug, Serialize)]
struct ArchiveReport {
    schema_version: u32,
    canonical_result_document_count: usize,
    resident_dataset_verified: bool,
    models_and_recipes_verified: bool,
    atomic_append_verified: bool,
    final_reopen_verified: bool,
    source_archive_preserved_as_absent_fixture: bool,
    initial_write: ProjectArchiveWriteReceiptV6,
    initial_document: ArtifactDescriptor,
    document: ArtifactDescriptor,
}

#[derive(Debug, Serialize)]
struct HarnessReport {
    schema_version: u32,
    report_kind: &'static str,
    work_evidence_only: bool,
    qualification_ready: bool,
    promotion_authority: bool,
    customer_cli_invoked: bool,
    customer_registry_admission_invoked: bool,
    candidate_receipt_descriptors: Vec<ArtifactDescriptor>,
    compiled_identity: CompiledIdentity,
    capability_cell: CapabilityCellReferenceV2,
    variants: Vec<VariantReport>,
    archive: ArchiveReport,
}

#[test]
#[ignore = "internal work-evidence harness; requires exact compile/runtime identity environment"]
fn current_product_pls_qualification_work_harness() {
    run_harness().expect("current-product PLS work-evidence harness must pass fail-closed");
}

fn run_harness() -> Result<(), String> {
    require_exact_activation()?;
    let source_set_sha256 =
        require_compiled_runtime_identity(SOURCE_SET_ENV, COMPILED_SOURCE_SET_SHA256)?;
    let scenario_set_sha256 =
        require_compiled_runtime_identity(SCENARIO_SET_ENV, COMPILED_SCENARIO_SET_SHA256)?;
    let output_root = create_new_output_root()?;
    let executable = current_executable_identity()?;

    let dataset = import_delimited_bytes(
        include_bytes!("../../validation/fixtures/simple_reflective.csv"),
        "simple_reflective.csv",
        b',',
        &ImportOptions::default(),
    )
    .map_err(|error| format!("fixture import failed: {error}"))?;
    let cases = VARIANTS
        .iter()
        .enumerate()
        .map(|(index, definition)| build_variant_case(&dataset, index, definition.clone()))
        .collect::<Result<Vec<_>, _>>()?;

    let mut live_project = Project::new("PLS current-product work evidence");
    live_project.datasets.push(dataset.clone());
    let source_archive = output_root.join("unchanged-source-v5.qpls");
    let archive_path = output_root.join("current-product.schema6.json");
    let upgrade = ProjectArchiveUpgradeRequestV6 {
        source_archive_sha256: "a".repeat(64),
        source_archive_path: source_archive.to_string_lossy().into_owned(),
        destination_archive_path: archive_path.to_string_lossy().into_owned(),
        upgraded_at: Utc
            .timestamp_opt(1_786_752_000, 0)
            .single()
            .ok_or_else(|| "fixed upgrade timestamp is invalid".to_string())?,
        legacy_display_covariances: BTreeMap::new(),
    };
    let mut plan = plan_project_upgrade_to_v6(&live_project, &upgrade)
        .map_err(|error| format!("schema-6 plan failed: {error}"))?;
    for case in &cases {
        let scientific_sha256 = case
            .request
            .model
            .scientific_sha256()
            .map_err(|error| format!("model scientific digest failed: {error}"))?;
        plan.document.models.push(ProjectModelRecordV6 {
            model_id: case.request.model.id.clone(),
            payload: ProjectModelPayloadV6::SemModelV4 {
                model: case.request.model.clone(),
                scientific_sha256,
            },
        });
        plan.document.recipes.push(case.request.recipe.clone());
    }
    plan.ensure_valid()
        .map_err(|error| format!("schema-6 resident model/recipe plan failed: {error}"))?;
    let initial_write = write_project_document_v6_new(&archive_path, &plan.document)
        .map_err(|error| format!("schema-6 initial write failed: {error}"))?;
    let initial_document = copy_artifact_create_new(
        &output_root,
        &archive_path,
        "archive_steps/00-initial.schema6.json",
    )?;
    if initial_document.sha256 != initial_write.document_sha256 {
        return Err("schema-6 initial archive copy digest mismatch".into());
    }

    let mut expected_source_sha256 = initial_write.document_sha256.clone();
    let project_id = plan.document.project_id;
    let mut variants = Vec::with_capacity(cases.len());
    let mut append_receipts = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let resident = resolve_internal_recipe_v4_dataset(&live_project, &case.request)
            .map_err(|error| format!("resident dataset resolution failed: {error:?}"))?;
        let execution = execute_internal_recipe_v4_pls(&resident, &case.request)
            .map_err(|error| format!("Recipe-v4 product execution failed: {error:?}"))?;
        if execution.estimation().method_version != PLS_METHOD_VERSION
            || execution.estimation().score_execution.is_some()
        {
            return Err(format!(
                "{} did not execute the exact legacy-standard pls_pm_v1 point-estimate path",
                case.definition.id
            ));
        }
        if execution.provenance().compilation_receipt() != case.compiled.receipt() {
            return Err(format!(
                "{} execution receipt differs from deterministic precompilation",
                case.definition.id
            ));
        }

        let job_id = Uuid::from_u128(0x3000 + index as u128);
        let started_second = index * 2;
        let started_at = format!("2026-08-15T00:00:{started_second:02}Z");
        let completed_at = format!("2026-08-15T00:00:{:02}Z", started_second + 1);
        let canonical = build_recipe_v4_pls_canonical_result(
            job_id,
            project_id,
            &started_at,
            &completed_at,
            &case.request,
            &execution,
        )
        .map_err(|errors| {
            format!(
                "{} product canonical projection failed: {}",
                case.definition.id,
                errors.join("; ")
            )
        })?;
        let archive_canonical: CanonicalResultDocumentV2 = serde_json::from_value(
            serde_json::to_value(&canonical)
                .map_err(|error| format!("canonical serialization failed: {error}"))?,
        )
        .map_err(|error| format!("archive canonical conversion failed: {error}"))?;
        archive_canonical
            .ensure_valid()
            .map_err(|error| format!("archive canonical validation failed: {error}"))?;

        let request_descriptor = write_json_artifact(
            &output_root,
            &format!("requests/{}.request.json", case.definition.id),
            &case.request,
        )?;
        let compiled_descriptor = write_json_artifact(
            &output_root,
            &format!("compiled/{}.compiled.json", case.definition.id),
            &case.compiled,
        )?;
        let execution_descriptor = write_json_artifact(
            &output_root,
            &format!("executions/{}.execution.json", case.definition.id),
            &execution,
        )?;
        let canonical_descriptor = write_json_artifact(
            &output_root,
            &format!("canonical/{}.canonical.json", case.definition.id),
            &canonical,
        )?;

        let append = append_canonical_result_document_v2_file_v6(
            &archive_path,
            &expected_source_sha256,
            archive_canonical.clone(),
        )
        .map_err(|error| format!("schema-6 atomic append failed: {error}"))?;
        if !append.source_verified_at_commit
            || !append.post_write_validated
            || !append.rollback_copy_removed
        {
            return Err(format!(
                "{} schema-6 append receipt is incomplete",
                case.definition.id
            ));
        }
        expected_source_sha256 = append.updated_document_sha256.clone();
        let reopened = read_project_document_v6(&archive_path)
            .map_err(|error| format!("schema-6 reopen failed: {error}"))?;
        if reopened.schema_version != 6
            || reopened.canonical_result_documents.len() != index + 1
            || !reopened
                .canonical_result_documents
                .iter()
                .any(|attachment| {
                    attachment.document_id() == archive_canonical.document_id.as_str()
                        && attachment.canonical_document() == &archive_canonical
                })
        {
            return Err(format!(
                "{} canonical result did not reopen exactly",
                case.definition.id
            ));
        }
        let archive_after_append = copy_artifact_create_new(
            &output_root,
            &archive_path,
            &format!(
                "archive_steps/{:02}-{}.schema6.json",
                index + 1,
                case.definition.id
            ),
        )?;
        if archive_after_append.sha256 != append.updated_document_sha256 {
            return Err(format!(
                "{} retained schema-6 append copy digest mismatch",
                case.definition.id
            ));
        }

        let compilation_receipt_sha256 =
            sha256_serialized(execution.provenance().compilation_receipt());
        append_receipts.push(append.clone());
        variants.push(VariantReport {
            variant: case.definition.id.into(),
            weighting_scheme: weighting_name(&case.definition.weighting_scheme).into(),
            measurement_mode: mode_name(&case.definition.measurement_mode).into(),
            recipe_v4_compiled: true,
            real_runner_executed: true,
            product_canonical_builder_executed: true,
            schema6_append_reopened: true,
            method_version: execution.estimation().method_version.clone(),
            compilation_receipt_sha256,
            request: request_descriptor,
            compiled_artifact: compiled_descriptor,
            execution: execution_descriptor,
            canonical_document: canonical_descriptor,
            append_receipt: append,
            archive_after_append,
        });
    }

    let final_document = read_project_document_v6(&archive_path)
        .map_err(|error| format!("final schema-6 reopen failed: {error}"))?;
    let resident_dataset_verified = final_document
        .datasets
        .iter()
        .any(|resident| resident.id == dataset.id && resident.fingerprint == dataset.fingerprint);
    let models_and_recipes_verified = cases.iter().all(|case| {
        final_document.models.iter().any(|record| {
            record.model_id == case.request.model.id
                && matches!(
                    &record.payload,
                    ProjectModelPayloadV6::SemModelV4 { model, scientific_sha256 }
                        if model == &case.request.model
                            && model.scientific_sha256().ok().as_ref() == Some(scientific_sha256)
                )
        }) && final_document
            .recipes
            .iter()
            .any(|recipe| recipe == &case.request.recipe)
    });
    let atomic_append_verified = append_receipts.iter().all(|receipt| {
        receipt.source_verified_at_commit
            && receipt.post_write_validated
            && receipt.rollback_copy_removed
    });
    if final_document.schema_version != 6
        || final_document.canonical_result_documents.len() != VARIANTS.len()
        || !resident_dataset_verified
        || !models_and_recipes_verified
        || !atomic_append_verified
        || source_archive.exists()
    {
        return Err("final schema-6 product archive contract did not pass".into());
    }

    let archive_descriptor = descriptor(&output_root, &archive_path)?;
    let report = HarnessReport {
        schema_version: 1,
        report_kind: "pls_algorithm_v1_current_product_harness_v1",
        work_evidence_only: true,
        qualification_ready: false,
        promotion_authority: false,
        customer_cli_invoked: false,
        customer_registry_admission_invoked: false,
        candidate_receipt_descriptors: Vec::new(),
        compiled_identity: CompiledIdentity {
            source_set_sha256,
            scenario_set_sha256,
            executable,
        },
        capability_cell: RecipeV4CompilerTarget::PlsPlanV2.capability_cell(),
        variants,
        archive: ArchiveReport {
            schema_version: final_document.schema_version,
            canonical_result_document_count: final_document.canonical_result_documents.len(),
            resident_dataset_verified,
            models_and_recipes_verified,
            atomic_append_verified,
            final_reopen_verified: true,
            source_archive_preserved_as_absent_fixture: !source_archive.exists(),
            initial_write,
            initial_document,
            document: archive_descriptor,
        },
    };
    write_report_atomically(&output_root, &report)?;
    Ok(())
}

fn build_variant_case(
    dataset: &Dataset,
    index: usize,
    definition: VariantDefinition,
) -> Result<VariantCase, String> {
    let construct = |id: &str, label: &str, indicators: &[&str]| Construct {
        id: id.into(),
        name: label.into(),
        short_name: label.into(),
        mode: definition.measurement_mode.clone(),
        indicators: indicators.iter().map(|value| (*value).into()).collect(),
    };
    let source_model = ModelSpec {
        id: Uuid::from_u128(0x1000 + index as u128),
        name: format!("PLS current-product {}", definition.id),
        constructs: vec![
            construct("x", "X", &["x1", "x2"]),
            construct("y", "Y", &["y1", "y2"]),
        ],
        paths: vec![StructuralPath {
            source: "x".into(),
            target: "y".into(),
        }],
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let source_recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: Uuid::from_u128(0x2000 + index as u128),
        created_at: Utc
            .timestamp_opt(1_786_752_000 + index as i64, 0)
            .single()
            .ok_or_else(|| "fixed recipe timestamp is invalid".to_string())?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: source_model.clone(),
        settings: AnalysisSettings {
            method: AnalysisMethod::PlsPm,
            weighting_scheme: definition.weighting_scheme.clone(),
            workers: 1,
            seed: 20_260_815,
            ..AnalysisSettings::default()
        },
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata: BTreeMap::from([(
            "qualification_use".into(),
            "current_product_work_evidence_only".into(),
        )]),
    };
    let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe)
        .map_err(|error| format!("Recipe-v4 migration failed: {error}"))?;
    let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
        &pending,
        &source_model,
        &[],
        LegacyBasicModelInterpretationV4::PlsComposite,
    )
    .map_err(|error| format!("composite estimand confirmation failed: {error}"))?;
    let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
        return Err("confirmed PLS model did not retain raw-data binding".into());
    };
    *dataset_id = dataset.id.to_string();
    recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: model.id.clone(),
        scientific_sha256: model
            .scientific_sha256()
            .map_err(|error| format!("model scientific digest failed: {error}"))?,
    };
    recipe
        .ensure_valid()
        .map_err(|error| format!("Recipe-v4 validation failed: {error}"))?;
    let compiler_target = RecipeV4CompilerTarget::PlsPlanV2;
    let capability_cell = compiler_target.capability_cell();
    let compiled = compile_analysis_recipe_v4(
        &recipe,
        Some(&model),
        compiler_target,
        capability_cell.clone(),
    )
    .map_err(|error| format!("Recipe-v4 compilation failed: {error}"))?;
    Ok(VariantCase {
        definition,
        request: InternalRecipeV4PlsExecutionRequestV1 {
            surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
            experimental_labs_enabled: true,
            resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe,
            model,
            compiler_target,
            capability_cell,
            posthoc_technical_minimum_sample_size: None,
        },
        compiled,
    })
}

fn require_exact_activation() -> Result<(), String> {
    match env::var(ACTIVATION_ENV) {
        Ok(value) if value == "1" => Ok(()),
        _ => Err(format!(
            "ignored harness requires exact {ACTIVATION_ENV}=1 activation"
        )),
    }
}

fn require_compiled_runtime_identity(
    name: &str,
    compiled: Option<&'static str>,
) -> Result<String, String> {
    let compiled = compiled.ok_or_else(|| {
        format!("test executable was not compiled with the required {name} identity")
    })?;
    let runtime = env::var(name).map_err(|_| format!("runtime {name} identity is missing"))?;
    if runtime != compiled || !is_sha256(&runtime) {
        return Err(format!("compile/runtime {name} identity mismatch"));
    }
    Ok(runtime)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn create_new_output_root() -> Result<PathBuf, String> {
    let requested =
        PathBuf::from(env::var(OUTPUT_ENV).map_err(|_| format!("{OUTPUT_ENV} is required"))?);
    if !requested.is_absolute() || requested.exists() {
        return Err(format!(
            "{OUTPUT_ENV} must be an absolute path that does not already exist"
        ));
    }
    let parent = requested
        .parent()
        .ok_or_else(|| format!("{OUTPUT_ENV} has no parent directory"))?
        .canonicalize()
        .map_err(|error| format!("{OUTPUT_ENV} parent is unavailable: {error}"))?;
    let file_name = requested
        .file_name()
        .ok_or_else(|| format!("{OUTPUT_ENV} has no final component"))?;
    let root = parent.join(file_name);
    fs::create_dir(&root).map_err(|error| format!("output root creation failed: {error}"))?;
    let metadata = fs::symlink_metadata(&root)
        .map_err(|error| format!("output root inspection failed: {error}"))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err("output root must be a real directory".into());
    }
    Ok(root)
}

fn current_executable_identity() -> Result<ExecutableIdentity, String> {
    let path = env::current_exe()
        .map_err(|error| format!("current test executable resolution failed: {error}"))?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("current test executable inspection failed: {error}"))?;
    if !metadata.is_file() {
        return Err("current test executable is not a regular file".into());
    }
    Ok(ExecutableIdentity {
        path: path.to_string_lossy().into_owned(),
        size_bytes: metadata.len(),
        sha256: sha256_file(&path)?,
    })
}

fn write_json_artifact<T: Serialize>(
    root: &Path,
    relative: &str,
    value: &T,
) -> Result<ArtifactDescriptor, String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("artifact directory creation failed: {error}"))?;
    }
    let mut payload = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("artifact serialization failed: {error}"))?;
    payload.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("artifact create-new failed: {error}"))?;
    file.write_all(&payload)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("artifact write/sync failed: {error}"))?;
    descriptor(root, &path)
}

fn copy_artifact_create_new(
    root: &Path,
    source: &Path,
    relative: &str,
) -> Result<ArtifactDescriptor, String> {
    let destination = root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("archive copy directory creation failed: {error}"))?;
    }
    let mut input =
        File::open(source).map_err(|error| format!("archive copy source open failed: {error}"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("archive copy create-new failed: {error}"))?;
    std::io::copy(&mut input, &mut output)
        .and_then(|_| output.sync_all())
        .map_err(|error| format!("archive copy write/sync failed: {error}"))?;
    descriptor(root, &destination)
}

fn write_report_atomically(root: &Path, report: &HarnessReport) -> Result<(), String> {
    let temporary = root.join("harness_report.json.tmp");
    let final_path = root.join("harness_report.json");
    let mut payload = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("harness report serialization failed: {error}"))?;
    payload.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("harness report create-new failed: {error}"))?;
    file.write_all(&payload)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("harness report write/sync failed: {error}"))?;
    drop(file);
    fs::rename(&temporary, &final_path)
        .map_err(|error| format!("harness report atomic publish failed: {error}"))?;
    let parsed: HarnessReportReadback = serde_json::from_slice(
        &fs::read(&final_path)
            .map_err(|error| format!("harness report readback failed: {error}"))?,
    )
    .map_err(|error| format!("harness report strict readback failed: {error}"))?;
    if parsed.schema_version != 1
        || parsed.report_kind != "pls_algorithm_v1_current_product_harness_v1"
        || !parsed.work_evidence_only
        || parsed.qualification_ready
        || parsed.promotion_authority
        || parsed.customer_cli_invoked
        || parsed.customer_registry_admission_invoked
        || !parsed.candidate_receipt_descriptors.is_empty()
        || parsed.compiled_identity.is_null()
        || parsed.capability_cell.is_null()
        || parsed.variants.len() != VARIANTS.len()
        || parsed.archive.is_null()
    {
        return Err("harness report readback became promotional or drifted".into());
    }
    if parsed
        .candidate_receipt_descriptors
        .iter()
        .any(|descriptor| {
            descriptor.path.is_empty()
                || descriptor.size_bytes == 0
                || !is_sha256(&descriptor.sha256)
        })
    {
        return Err("harness report contains an invalid candidate receipt descriptor".into());
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessReportReadback {
    schema_version: u32,
    report_kind: String,
    work_evidence_only: bool,
    qualification_ready: bool,
    promotion_authority: bool,
    customer_cli_invoked: bool,
    customer_registry_admission_invoked: bool,
    candidate_receipt_descriptors: Vec<ArtifactDescriptorReadback>,
    compiled_identity: serde_json::Value,
    capability_cell: serde_json::Value,
    variants: Vec<serde_json::Value>,
    archive: serde_json::Value,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactDescriptorReadback {
    path: String,
    size_bytes: u64,
    sha256: String,
}

fn descriptor(root: &Path, path: &Path) -> Result<ArtifactDescriptor, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("artifact root canonicalization failed: {error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("artifact canonicalization failed: {error}"))?;
    let relative = canonical_path
        .strip_prefix(&canonical_root)
        .map_err(|_| "artifact escapes the output root".to_string())?;
    let metadata = fs::metadata(&canonical_path)
        .map_err(|error| format!("artifact inspection failed: {error}"))?;
    if !metadata.is_file() {
        return Err("artifact is not a regular file".into());
    }
    Ok(ArtifactDescriptor {
        path: relative.to_string_lossy().replace('\\', "/"),
        size_bytes: metadata.len(),
        sha256: sha256_file(&canonical_path)?,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("SHA-256 open failed: {error}"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("SHA-256 read failed: {error}"))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn weighting_name(value: &WeightingScheme) -> &'static str {
    match value {
        WeightingScheme::Path => "path",
        WeightingScheme::Factor => "factor",
        WeightingScheme::Pca => "pca",
    }
}

fn mode_name(value: &MeasurementMode) -> &'static str {
    match value {
        MeasurementMode::Reflective => "mode_a",
        MeasurementMode::Formative => "mode_b",
    }
}
