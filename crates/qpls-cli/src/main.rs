use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, AssessmentResult, FitCriterionValue, HTMT_ORIGINAL_METHOD_VERSION,
    HTMT_PLUS_METHOD_VERSION, HtmtAssessment, HtmtStatus, PLS_MODEL_FIT_METHOD_VERSION,
    PlsModelFit, RHO_A_METHOD_VERSION, pls_model_fit_matches_v2_contract,
};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisPayload, AnalysisRecipe,
    AnalysisResult, AnalysisSettings, CapabilityOptionCellV2, CapabilityRegistryV2, Construct,
    GateStatus, METHOD_CAPABILITIES, MeasurementMode, MethodConfig, ModelSpec,
    PlsBootstrapTestTail, ProductSurfaceV2, RunStatus, Severity, SliceStatus, StructuralPath,
    ValidatedExecutionRecipe, development_slice_registry, validate_recipe, validate_slice_registry,
};
use qpls_data::{DataKind, ImportOptions, import_path};
use qpls_project::{Project, load_project_with_autosave, save_project};
use qpls_resampling::{
    HTMT_BOOTSTRAP_CRITICAL_VALUE, HTMT_BOOTSTRAP_DECISION_RULE,
    HTMT_BOOTSTRAP_EQUIVALENT_TWO_SIDED_CONFIDENCE_LEVEL, HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION,
    HTMT_BOOTSTRAP_INTERVAL_METHOD, HTMT_BOOTSTRAP_REPLICATE_INDEX_DIGEST_METHOD,
    HTMT_BOOTSTRAP_SIGNIFICANCE_LEVEL, HTMT_BOOTSTRAP_TEST_TYPE,
    HTMT_ORIGINAL_BOOTSTRAP_METHOD_VERSION, HTMT_PLUS_BOOTSTRAP_METHOD_VERSION,
    HtmtBootstrapInference, HtmtBootstrapInferenceBundle, HtmtBootstrapInferenceStatus,
    PERMUTATION_METHOD_VERSION, PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION,
    PLS_MODEL_FIT_EXACT_METHOD_VERSION, PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR,
    PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID, PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY,
    PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD, PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2,
    PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD, PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
    PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2, PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION,
    PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION_V2, PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN,
    PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN_V2, PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION,
    PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION, PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION,
    PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION, PlsBootstrapResult,
    PlsBootstrapTestTailInference, PlsModelFitExactCriterion, PlsModelFitExactInference,
    PlsModelFitExactReplicateStatus, PlsModelFitExactStatus, PlsModelFitExactVariantInference,
    PlsPowerGridDecisionV1, PlsResamplingParameterFamily, PlsResamplingParameterIdentity,
    PlsSampleSizePowerResultV1, PlscConsistentBootstrapResult, PlscConsistentPermutationResult,
    RESAMPLING_METHOD_VERSION, ResamplingPhase, STUDENTIZED_METHOD_VERSION, bootstrap_pls,
    permutation_pls, validate_pls_bootstrap_test_tail_contract,
    validate_pls_model_fit_exact_inference_for_settings, validate_plsc_consistent_bootstrap_result,
    validate_plsc_consistent_permutation_result_for_settings,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

#[derive(Parser)]
#[command(name = "qpls", version, about = "QuickPLS reproducible analysis CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum InputKind {
    Raw,
    Covariance,
    Correlation,
}

#[derive(Clone, Copy, ValueEnum)]
enum QualificationTarget {
    V04Inference,
}

#[derive(Clone, Copy, ValueEnum)]
enum EvidenceTarget {
    V03Pls,
    V04Assessment,
    V05ExtendedPls,
    V07Cbsem,
    V08ExtendedMethods,
    PublicationReady,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    Csv,
    Html,
    Xlsx,
}

#[derive(Subcommand)]
enum DemoCommand {
    Create {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        expected: Option<PathBuf>,
    },
    Validate {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        expected: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

impl From<InputKind> for DataKind {
    fn from(value: InputKind) -> Self {
        match value {
            InputKind::Raw => DataKind::Raw,
            InputKind::Covariance => DataKind::Covariance,
            InputKind::Correlation => DataKind::Correlation,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    Validate {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Import {
        data: PathBuf,
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum, default_value = "raw")]
        kind: InputKind,
        #[arg(long)]
        sample_size: Option<usize>,
        #[arg(long)]
        sheet: Option<String>,
        #[arg(long)]
        delimiter: Option<char>,
    },
    Inspect {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Create a distinct schema-v3 recipe copy while preserving the historical source.
    MigrateRecipe {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// List exact Capability Registry V2 option cells and customer availability.
    Methods {
        #[arg(long)]
        json: bool,
    },
    Roadmap {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        release: Option<String>,
    },
    Gate {
        slice_id: String,
        #[arg(long)]
        json: bool,
    },
    Qualify {
        #[arg(value_enum, default_value = "v04-inference")]
        target: QualificationTarget,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        refresh_quick_monte_carlo: bool,
        #[arg(long)]
        refresh_pilot_monte_carlo: bool,
    },
    Evidence {
        #[arg(value_enum, default_value = "v04-assessment")]
        target: EvidenceTarget,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Demo {
        #[command(subcommand)]
        command: DemoCommand,
    },
    Run {
        input: PathBuf,
        #[arg(long)]
        data: Option<PathBuf>,
        #[arg(long)]
        recipe_id: Option<String>,
        #[arg(long)]
        output: PathBuf,
        // Hidden compatibility switch used only for capability cells that the
        // embedded registry currently exposes through Experimental Labs.
        #[arg(long, hide = true)]
        allow_experimental: bool,
        // Validation-only escape hatch for rebuilding source-tier evidence for
        // the generated established-method contracts. Release builds reject
        // this flag and it never changes the embedded product Registry.
        #[arg(long, hide = true)]
        allow_internal_qualification: bool,
        #[arg(long)]
        bootstrap_samples: Option<u32>,
        #[arg(long)]
        studentized_inner_samples: Option<u32>,
        #[arg(long)]
        permutation_samples: Option<u32>,
        #[arg(long)]
        workers: Option<usize>,
    },
    Export {
        result: PathBuf,
        #[arg(long, value_enum)]
        format: ExportFormat,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        include_experimental: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Validate { input, json } => validate_input(&input, json),
        Command::Import {
            data,
            output,
            name,
            kind,
            sample_size,
            sheet,
            delimiter,
        } => {
            let delimiter = delimiter
                .map(|value| {
                    if !value.is_ascii() {
                        bail!("delimiter must be a single ASCII character");
                    }
                    Ok(value as u8)
                })
                .transpose()?;
            let options = ImportOptions {
                delimiter,
                sheet_name: sheet,
                data_kind: kind.into(),
                sample_size,
                ..ImportOptions::default()
            };
            let dataset = import_path(&data, &options)
                .with_context(|| format!("cannot import {}", data.display()))?;
            let project_name = name.unwrap_or_else(|| {
                data.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("QuickPLS project")
                    .to_owned()
            });
            let mut project = Project::new(project_name);
            project.datasets.push(dataset);
            save_project(&output, &project)
                .with_context(|| format!("cannot save {}", output.display()))?;
            println!("created {}", output.display());
            Ok(())
        }
        Command::Inspect { project, json } => inspect_project(&project, json),
        Command::MigrateRecipe {
            input,
            output,
            json,
        } => migrate_recipe(&input, &output, json),
        Command::Methods { json } => list_capability_registry(json),
        Command::Roadmap { json, release } => roadmap(json, release.as_deref()),
        Command::Gate { slice_id, json } => gate(&slice_id, json),
        Command::Qualify {
            target,
            output,
            refresh_quick_monte_carlo,
            refresh_pilot_monte_carlo,
        } => qualify(
            target,
            output.as_deref(),
            refresh_quick_monte_carlo,
            refresh_pilot_monte_carlo,
        ),
        Command::Evidence { target, output } => evidence(target, output.as_deref()),
        Command::Demo { command } => match command {
            DemoCommand::Create { project, expected } => {
                create_demo_project(project.as_deref(), expected.as_deref())
            }
            DemoCommand::Validate {
                project,
                expected,
                output,
            } => validate_demo_project(project.as_deref(), expected.as_deref(), output.as_deref()),
        },
        Command::Run {
            input,
            data,
            recipe_id,
            output,
            allow_experimental,
            allow_internal_qualification,
            bootstrap_samples,
            studentized_inner_samples,
            permutation_samples,
            workers,
        } => run_analysis(
            &input,
            data.as_deref(),
            recipe_id.as_deref(),
            &output,
            allow_experimental,
            allow_internal_qualification,
            false,
            bootstrap_samples,
            studentized_inner_samples,
            permutation_samples,
            workers,
        ),
        Command::Export {
            result,
            format,
            output,
            include_experimental,
        } => export_result(&result, format, output.as_deref(), include_experimental),
    }
}

fn customer_availability(cell: &CapabilityOptionCellV2) -> &'static str {
    if cell.standard_available() {
        "Standard"
    } else if cell.labs_available() {
        "Experimental Labs"
    } else if cell.surface == ProductSurfaceV2::Legacy {
        "Legacy reopen only"
    } else {
        "Unavailable"
    }
}

fn capability_registry_cli_json(registry: &CapabilityRegistryV2) -> serde_json::Value {
    json!({
        "schema_version": 2,
        "projection": "cli_option_cell_availability_v2",
        "registry_id": registry.registry_id,
        "registry_version": registry.registry_version,
        "source_sha256": registry.source_sha256,
        "catalogue_snapshot": registry.catalogue_snapshot,
        "capabilities": registry.capabilities.iter().map(|row| json!({
            "catalogue_position": row.catalogue_position,
            "capability_id": row.capability_id,
            "official_family": row.official_family,
            "official_method": row.official_method,
            "official_lifecycle": row.official_lifecycle,
            "official_url": row.official_url,
            "option_cells": row.option_cells.iter().map(|cell| json!({
                "registry_schema_version": 2,
                "capability_id": cell.capability_id,
                "cell_id": cell.cell_id,
                "capability_version": cell.capability_version,
                "coverage_state": cell.coverage_state,
                "evidence_state": cell.evidence_state,
                "surface": cell.surface,
                "customer_availability": customer_availability(cell),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn capability_registry_cli_text(registry: &CapabilityRegistryV2) -> String {
    let mut lines = vec![format!(
        "QuickPLS capability registry {} | source sha256:{}",
        registry.registry_version, registry.source_sha256
    )];
    for row in &registry.capabilities {
        for cell in &row.option_cells {
            lines.push(format!(
                "{:>2} | {} | {} | {} | coverage={} evidence={} surface={} | {}",
                row.catalogue_position,
                row.official_method,
                cell.cell_id,
                cell.capability_version,
                cell.coverage_state,
                cell.evidence_state,
                cell.surface,
                customer_availability(cell),
            ));
        }
    }
    lines.join("\n")
}

fn list_capability_registry(as_json: bool) -> Result<()> {
    let registry = CapabilityRegistryV2::embedded().context(
        "the embedded Capability Registry V2 is invalid; method availability cannot be reported",
    )?;
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&capability_registry_cli_json(&registry))?
        );
    } else {
        println!("{}", capability_registry_cli_text(&registry));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExportRow {
    section: String,
    construct: String,
    indicator: String,
    source: String,
    target: String,
    metric: String,
    value: String,
}

fn export_result(
    result_path: &Path,
    format: ExportFormat,
    output: Option<&Path>,
    include_experimental: bool,
) -> Result<()> {
    let result: AnalysisResult = serde_json::from_slice(
        &fs::read(result_path).with_context(|| format!("cannot read {}", result_path.display()))?,
    )
    .context("invalid analysis result JSON")?;
    let rows = match &result.payload {
        AnalysisPayload::PlsSampleSizePowerV1 { analysis }
        | AnalysisPayload::PlsSampleSizePowerV2 { analysis } => {
            pls_sample_size_power_export_rows(&result, analysis)?
        }
        _ if include_experimental => experimental_pls_export_rows(&result)?,
        _ => v03_estimator_export_rows(&result)?,
    };
    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| result_path.with_extension(default_export_extension(format)));
    match format {
        ExportFormat::Csv => fs::write(&output_path, render_estimator_csv(&rows))
            .with_context(|| format!("cannot write {}", output_path.display()))?,
        ExportFormat::Html => fs::write(&output_path, render_estimator_html(&result, &rows))
            .with_context(|| format!("cannot write {}", output_path.display()))?,
        ExportFormat::Xlsx => write_estimator_xlsx(&output_path, &rows)
            .with_context(|| format!("cannot write {}", output_path.display()))?,
    }
    println!(
        "wrote {} export {}",
        if matches!(
            &result.payload,
            AnalysisPayload::PlsSampleSizePowerV1 { .. }
                | AnalysisPayload::PlsSampleSizePowerV2 { .. }
        ) {
            "typed PLS sample-size/power"
        } else if include_experimental {
            "watermarked experimental"
        } else {
            "v0.3 estimator-only"
        },
        output_path.display()
    );
    Ok(())
}

fn default_export_extension(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "estimator.csv",
        ExportFormat::Html => "estimator.html",
        ExportFormat::Xlsx => "estimator.xlsx",
    }
}

fn pls_sample_size_power_export_rows(
    result: &AnalysisResult,
    analysis: &serde_json::Value,
) -> Result<Vec<ExportRow>> {
    if result.status != RunStatus::Completed {
        bail!("only completed analysis results can be exported");
    }
    if result.provenance.method != AnalysisMethod::PlsSampleSizePower {
        bail!("typed PLS sample-size/power payload has incompatible method provenance");
    }
    let power: PlsSampleSizePowerResultV1 = serde_json::from_value(analysis.clone())
        .context("invalid typed PLS sample-size/power result payload")?;
    validate_standalone_power_result_for_export(result, &power)?;

    let mut rows = Vec::new();
    push_metadata_rows(result, &mut rows);
    rows.push(row(
        "metadata",
        "",
        "",
        "",
        "",
        "export_scope",
        "typed PLS sample-size/power result tables and complete ordered replicate ledger".into(),
    ));
    rows.push(row(
        "metadata",
        "",
        "",
        "",
        "",
        "standalone_integrity_scope",
        "Typed shape, frozen identities, accounting, row summaries, ledger ordering, and grid decision were checked. The standalone result does not contain the scientific recipe, so recipe_digest, outcome_digest, and indexed stream identities are exported as stored provenance and cannot be independently recomputed by this command.".into(),
    ));
    for (metric, value) in [
        ("schema_version", power.schema_version.to_string()),
        ("capability_id", power.capability_id.clone()),
        ("method_version", power.method_version.clone()),
        ("recipe_digest", power.recipe_digest.clone()),
        ("outcome_digest", power.outcome_digest.clone()),
        ("stream_domain", power.stream_domain.clone()),
        ("failure_policy", power.failure_policy.clone()),
        ("interval_method", power.interval_method.clone()),
        ("inference_method", power.inference_method.clone()),
        ("pls_method_version", power.pls_method_version.clone()),
        (
            "resampling_method_version",
            power.resampling_method_version.clone(),
        ),
        (
            "monotonicity_violations",
            power.monotonicity_violations.to_string(),
        ),
    ] {
        rows.push(row("pls_power_provenance", "", "", "", "", metric, value));
    }
    let decision = match power.decision {
        PlsPowerGridDecisionV1::Reached { sample_size } => sample_size.to_string(),
        PlsPowerGridDecisionV1::NotReached => "not_reached_on_evaluated_grid".into(),
    };
    rows.push(row(
        "pls_power_decision",
        "",
        "",
        "",
        "",
        "minimum_qualified_sample_size",
        decision,
    ));
    for (metric, value) in [
        ("grid_points", power.workload.grid_points),
        ("planned_datasets", power.workload.planned_datasets),
        ("estimated_pls_fits", power.workload.estimated_pls_fits),
        (
            "estimated_pls_case_fits",
            power.workload.estimated_pls_case_fits,
        ),
    ] {
        rows.push(row(
            "pls_power_workload",
            "",
            "",
            "",
            "",
            metric,
            value.to_string(),
        ));
    }
    for summary in &power.rows {
        let sample_size = summary.sample_size.to_string();
        for (metric, value) in [
            (
                "requested_replicates",
                summary.requested_replicates.to_string(),
            ),
            (
                "attempted_replicates",
                summary.attempted_replicates.to_string(),
            ),
            (
                "successful_replicates",
                summary.successful_replicates.to_string(),
            ),
            ("failed_replicates", summary.failed_replicates.to_string()),
            ("rejections", summary.rejections.to_string()),
            ("achieved_power", summary.achieved_power.to_string()),
            ("confidence_lower", summary.confidence_lower.to_string()),
            ("confidence_upper", summary.confidence_upper.to_string()),
            ("qualifies", summary.qualifies.to_string()),
        ] {
            rows.push(row(
                "pls_power_by_sample_size",
                &sample_size,
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for outcome in &power.outcomes {
        let sample_size = outcome.sample_size.to_string();
        let replicate = outcome.replicate_index.to_string();
        for (metric, value) in [
            ("stream_identity", outcome.stream_identity.clone()),
            ("attempted", outcome.attempted.to_string()),
            ("successful", outcome.successful.to_string()),
            ("converged", outcome.converged.to_string()),
            (
                "target_estimate",
                outcome
                    .target_estimate
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "p_value_two_sided",
                outcome
                    .p_value_two_sided
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "bootstrap_requested_replicates",
                outcome
                    .bootstrap_requested_replicates
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "bootstrap_usable_replicates",
                outcome
                    .bootstrap_usable_replicates
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "bootstrap_failed_replicates",
                outcome
                    .bootstrap_failed_replicates
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "bootstrap_two_sided_exceedances",
                outcome
                    .bootstrap_two_sided_exceedances
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            ("rejected", outcome.rejected.to_string()),
            (
                "failure_code",
                outcome.failure_code.clone().unwrap_or_default(),
            ),
            (
                "failure_message",
                outcome.failure_message.clone().unwrap_or_default(),
            ),
        ] {
            rows.push(row(
                "pls_power_replicate_ledger",
                &sample_size,
                &replicate,
                "",
                "",
                metric,
                value,
            ));
        }
        if !outcome.successful {
            for (metric, value) in [
                ("stream_identity", outcome.stream_identity.clone()),
                (
                    "failure_code",
                    outcome.failure_code.clone().unwrap_or_default(),
                ),
                (
                    "failure_message",
                    outcome.failure_message.clone().unwrap_or_default(),
                ),
            ] {
                rows.push(row(
                    "pls_power_failure",
                    &sample_size,
                    &replicate,
                    "",
                    "",
                    metric,
                    value,
                ));
            }
        }
    }
    for warning in &power.warnings {
        rows.push(row(
            "pls_power_warning",
            "",
            "",
            "",
            "",
            "warning",
            warning.clone(),
        ));
    }
    for exclusion in &power.exclusions {
        rows.push(row(
            "pls_power_exclusion",
            "",
            "",
            "",
            "",
            "exclusion",
            exclusion.clone(),
        ));
    }
    push_result_diagnostics(result, &mut rows);
    Ok(rows)
}

fn validate_standalone_power_result_for_export(
    analysis_result: &AnalysisResult,
    power: &PlsSampleSizePowerResultV1,
) -> Result<()> {
    let (expected_schema, expected_method, expected_stream, expected_inference, is_v2) =
        match &analysis_result.payload {
            AnalysisPayload::PlsSampleSizePowerV1 { .. } => (
                PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION,
                PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
                PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN,
                PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD,
                false,
            ),
            AnalysisPayload::PlsSampleSizePowerV2 { .. } => (
                PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION_V2,
                PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
                PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN_V2,
                PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2,
                true,
            ),
            _ => bail!("typed PLS sample-size/power validator received another payload family"),
        };
    if analysis_result.provenance.method_version != power.method_version
        || power.schema_version != expected_schema
        || power.capability_id != PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID
        || power.method_version != expected_method
        || power.stream_domain != expected_stream
        || power.failure_policy != PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY
        || power.interval_method != PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD
        || power.inference_method != expected_inference
        || power.pls_method_version != qpls_estimation::PLS_METHOD_VERSION
        || power.resampling_method_version != RESAMPLING_METHOD_VERSION
    {
        bail!("typed PLS sample-size/power payload has an incompatible frozen identity");
    }
    if !is_sha256_hex(&power.recipe_digest) || !is_sha256_hex(&power.outcome_digest) {
        bail!("typed PLS sample-size/power payload has an invalid stored digest");
    }
    if power.rows.len() as u64 != power.workload.grid_points
        || power.outcomes.len() as u64 != power.workload.planned_datasets
        || power.workload.estimated_pls_fits < power.workload.planned_datasets
        || power.workload.estimated_pls_case_fits < power.workload.planned_datasets
    {
        bail!("typed PLS sample-size/power workload does not match its stored tables");
    }

    let mut prior_sample_size = None;
    let mut planned = 0_u64;
    for summary in &power.rows {
        if !(100..=10_000).contains(&summary.requested_replicates)
            || !(30..=5_000).contains(&summary.sample_size)
            || summary.attempted_replicates != summary.requested_replicates
            || summary
                .successful_replicates
                .checked_add(summary.failed_replicates)
                != Some(summary.requested_replicates)
            || summary.rejections > summary.successful_replicates
            || !summary.achieved_power.is_finite()
            || !summary.confidence_lower.is_finite()
            || !summary.confidence_upper.is_finite()
            || !(0.0..=1.0).contains(&summary.achieved_power)
            || !(0.0..=1.0).contains(&summary.confidence_lower)
            || !(0.0..=1.0).contains(&summary.confidence_upper)
            || summary.confidence_lower > summary.confidence_upper
            || !float_matches(
                summary.achieved_power,
                summary.rejections as f64 / summary.requested_replicates as f64,
            )
            || prior_sample_size.is_some_and(|prior| summary.sample_size <= prior)
        {
            bail!(
                "typed PLS sample-size/power summary for sample size {} is inconsistent",
                summary.sample_size
            );
        }
        prior_sample_size = Some(summary.sample_size);
        planned += u64::from(summary.requested_replicates);
    }
    if planned != power.workload.planned_datasets {
        bail!("typed PLS sample-size/power row counts do not match planned datasets");
    }

    let ledger_order_matches = power
        .rows
        .iter()
        .flat_map(|summary| {
            (0..summary.requested_replicates)
                .map(move |replicate_index| (summary.sample_size, replicate_index))
        })
        .zip(&power.outcomes)
        .all(|((sample_size, replicate_index), outcome)| {
            outcome.sample_size == sample_size && outcome.replicate_index == replicate_index
        });
    if !ledger_order_matches {
        bail!("typed PLS sample-size/power ledger is not in strict grid and replicate order");
    }
    let mut outcomes_by_sample = std::collections::BTreeMap::new();
    for outcome in &power.outcomes {
        let rows = outcomes_by_sample
            .entry(outcome.sample_size)
            .or_insert_with(Vec::new);
        if outcome.replicate_index != rows.len() as u32
            || outcome.stream_identity.trim().is_empty()
            || !outcome.attempted
            || outcome.successful != outcome.failure_code.is_none()
            || outcome.successful != outcome.failure_message.is_none()
            || (outcome.successful
                && (!outcome.converged
                    || outcome.target_estimate.is_none()
                    || outcome.p_value_two_sided.is_none()))
            || (!outcome.successful
                && (outcome.converged
                    || outcome.target_estimate.is_some()
                    || outcome.p_value_two_sided.is_some()
                    || outcome.rejected
                    || outcome
                        .failure_code
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    || outcome
                        .failure_message
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())))
            || outcome
                .target_estimate
                .is_some_and(|value| !value.is_finite())
            || outcome
                .p_value_two_sided
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            || (outcome.rejected && !outcome.successful)
        {
            bail!(
                "typed PLS sample-size/power replicate {} at sample size {} is inconsistent",
                outcome.replicate_index,
                outcome.sample_size
            );
        }
        let tail_fields = (
            outcome.bootstrap_requested_replicates,
            outcome.bootstrap_usable_replicates,
            outcome.bootstrap_failed_replicates,
            outcome.bootstrap_two_sided_exceedances,
        );
        if is_v2 && outcome.successful {
            let (Some(requested), Some(usable), Some(failed), Some(exceedances)) = tail_fields else {
                bail!(
                    "typed PLS sample-size/power v2 replicate {} omits exact tail accounting",
                    outcome.replicate_index
                );
            };
            let expected_probability =
                (f64::from(exceedances) + 1.0) / (f64::from(usable) + 1.0);
            if usable.saturating_add(failed) != requested
                || exceedances > usable
                || outcome
                    .p_value_two_sided
                    .is_none_or(|value| value.to_bits() != expected_probability.to_bits())
            {
                bail!(
                    "typed PLS sample-size/power v2 replicate {} has inconsistent exact tail accounting",
                    outcome.replicate_index
                );
            }
        } else if tail_fields != (None, None, None, None) {
            bail!(
                "typed PLS sample-size/power replicate {} has tail accounting incompatible with its identity or status",
                outcome.replicate_index
            );
        }
        rows.push(outcome);
    }
    for summary in &power.rows {
        let outcomes = outcomes_by_sample
            .get(&summary.sample_size)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "typed PLS sample-size/power ledger omits sample size {}",
                    summary.sample_size
                )
            })?;
        let successful = outcomes.iter().filter(|outcome| outcome.successful).count() as u32;
        let rejected = outcomes.iter().filter(|outcome| outcome.rejected).count() as u32;
        if outcomes.len() as u32 != summary.requested_replicates
            || successful != summary.successful_replicates
            || summary.requested_replicates - successful != summary.failed_replicates
            || rejected != summary.rejections
        {
            bail!(
                "typed PLS sample-size/power ledger does not reproduce sample size {} accounting",
                summary.sample_size
            );
        }
    }
    if outcomes_by_sample.len() != power.rows.len() {
        bail!("typed PLS sample-size/power ledger contains an undeclared sample size");
    }
    let first_qualified = power.rows.iter().find(|row| row.qualifies);
    match (&power.decision, first_qualified) {
        (PlsPowerGridDecisionV1::Reached { sample_size }, Some(summary))
            if *sample_size == summary.sample_size => {}
        (PlsPowerGridDecisionV1::NotReached, None) => {}
        _ => bail!("typed PLS sample-size/power grid decision does not match stored rows"),
    }
    let violations = power
        .rows
        .windows(2)
        .filter(|pair| pair[1].achieved_power + 1e-12 < pair[0].achieved_power)
        .count() as u32;
    if violations != power.monotonicity_violations {
        bail!("typed PLS sample-size/power monotonicity count does not match stored rows");
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn float_matches(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-12 * left.abs().max(right.abs()).max(1.0)
}

fn v03_estimator_export_rows(result: &AnalysisResult) -> Result<Vec<ExportRow>> {
    if result.status != RunStatus::Completed {
        bail!("only completed analysis results can be exported");
    }
    if result.provenance.method != AnalysisMethod::PlsPm {
        bail!("v0.3 estimator export supports only PLS-SEM results");
    }
    if !result.provenance.method_version.contains("pls_pm_v1") {
        bail!(
            "v0.3 estimator export requires a pls_pm_v1 estimation payload, found {}",
            result.provenance.method_version
        );
    }
    let estimation = match &result.payload {
        AnalysisPayload::PlsPmV1 { estimation, .. }
        | AnalysisPayload::PlsPmV2 { estimation, .. }
        | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
        AnalysisPayload::PlsSampleSizePowerV1 { .. }
        | AnalysisPayload::PlsSampleSizePowerV2 { .. } => {
            bail!("PLS sample-size/power results require the typed power export path")
        }
        AnalysisPayload::Legacy { .. } => bail!("legacy result payloads cannot be exported"),
    };
    let mut rows = Vec::new();
    push_metadata_rows(result, &mut rows);
    rows.push(row(
        "metadata",
        "",
        "",
        "",
        "",
        "export_scope",
        "v0.3 estimator only; assessment and resampling are excluded".into(),
    ));
    push_scalar_estimate(estimation, "summary", "converged", "converged", &mut rows);
    push_scalar_estimate(estimation, "summary", "iterations", "iterations", &mut rows);
    push_scalar_estimate(
        estimation,
        "summary",
        "used_observations",
        "used_observations",
        &mut rows,
    );
    push_scalar_estimate(
        estimation,
        "summary",
        "omitted_observations",
        "omitted_observations",
        &mut rows,
    );
    push_outer_estimates(estimation, &mut rows);
    push_path_coefficients(estimation, &mut rows);
    push_posthoc_minimum_sample_size(result, estimation, &mut rows)?;
    push_effects(estimation, &mut rows);
    push_r_squared(estimation, &mut rows);
    push_result_diagnostics(result, &mut rows);
    Ok(rows)
}

fn experimental_pls_export_rows(result: &AnalysisResult) -> Result<Vec<ExportRow>> {
    if result.status != RunStatus::Completed {
        bail!("only completed analysis results can be exported");
    }
    let estimation = match &result.payload {
        AnalysisPayload::PlsPmV1 { estimation, .. }
        | AnalysisPayload::PlsPmV2 { estimation, .. }
        | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
        AnalysisPayload::PlsSampleSizePowerV1 { .. }
        | AnalysisPayload::PlsSampleSizePowerV2 { .. } => {
            bail!("PLS sample-size/power results require the typed power export path")
        }
        AnalysisPayload::Legacy { .. } => bail!("legacy result payloads cannot be exported"),
    };
    let mut rows = Vec::new();
    push_metadata_rows(result, &mut rows);
    rows.push(row(
        "metadata",
        "",
        "",
        "",
        "",
        "export_scope",
        "supplemental method export; see Method Details for model and data requirements".into(),
    ));
    rows.push(row(
        "scope_warning",
        "",
        "",
        "",
        "",
        "publication_status",
        "Supported result fields are included; incompatible or unavailable fields are omitted."
            .into(),
    ));
    push_scalar_estimate(estimation, "summary", "converged", "converged", &mut rows);
    push_scalar_estimate(estimation, "summary", "iterations", "iterations", &mut rows);
    push_scalar_estimate(
        estimation,
        "summary",
        "used_observations",
        "used_observations",
        &mut rows,
    );
    push_scalar_estimate(
        estimation,
        "summary",
        "omitted_observations",
        "omitted_observations",
        &mut rows,
    );
    push_outer_estimates(estimation, &mut rows);
    push_path_coefficients(estimation, &mut rows);
    push_posthoc_minimum_sample_size(result, estimation, &mut rows)?;
    push_effects(estimation, &mut rows);
    push_r_squared(estimation, &mut rows);
    push_pls_bootstrap_test_tail(result, &mut rows)?;
    push_plsc_consistent_bootstrap(result, estimation, &mut rows)?;
    push_plsc_consistent_permutation(result, estimation, &mut rows)?;
    push_pls_model_fit_and_exact(result, estimation, &mut rows)?;
    push_htmt_assessment_and_inference(result, &mut rows)?;
    push_cbsem_bootstrap_v2(result, estimation, &mut rows)?;
    push_experimental_method_payloads(estimation, &mut rows);
    push_result_diagnostics(result, &mut rows);
    Ok(rows)
}

fn push_pls_bootstrap_test_tail(result: &AnalysisResult, rows: &mut Vec<ExportRow>) -> Result<()> {
    let marker = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION);
    let bootstrap = match &result.payload {
        AnalysisPayload::PlsPmV2 { bootstrap, .. } => Some(bootstrap),
        AnalysisPayload::PlsPmV3 { bootstrap, .. } => bootstrap.as_ref(),
        _ => None,
    };
    let declares_receipt =
        bootstrap.is_some_and(|value| value.get("test_tail_inference").is_some());
    if result.provenance.method != AnalysisMethod::PlsPm {
        if marker
            || declares_receipt
            || result.provenance.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided
        {
            bail!("PLS bootstrap test-tail payload and method attribution disagree");
        }
        return Ok(());
    }
    let Some(bootstrap_value) = bootstrap else {
        if marker
            || result.provenance.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided
        {
            bail!("PLS bootstrap test-tail attribution requires a bootstrap payload");
        }
        return Ok(());
    };
    let receipt = bootstrap_value
        .get("test_tail_inference")
        .map(|value| serde_json::from_value::<PlsBootstrapTestTailInference>(value.clone()))
        .transpose()
        .context("malformed PLS bootstrap test-tail receipt")?;
    let bootstrap: PlsBootstrapResult = serde_json::from_value(bootstrap_value.clone())
        .context("invalid PLS bootstrap payload for test-tail export")?;
    validate_pls_bootstrap_test_tail_contract(
        &bootstrap,
        receipt.as_ref(),
        result.provenance.settings.bootstrap_test_tail,
        marker,
    )
    .map_err(anyhow::Error::new)
    .context("PLS bootstrap test-tail export contract failed")?;

    let Some(receipt) = receipt else {
        return Ok(());
    };
    rows.push(row(
        "pls_bootstrap_test_tail_contract",
        "",
        "",
        "",
        "",
        "method_version",
        receipt.method_version.clone(),
    ));
    rows.push(row(
        "pls_bootstrap_test_tail_contract",
        "",
        "",
        "",
        "",
        "selected_test_tail",
        serde_json::to_value(receipt.selected_test_tail)?
            .as_str()
            .expect("typed tail serializes as a string")
            .into(),
    ));
    for parameter in &receipt.parameters {
        let (construct, indicator, source, target) =
            pls_bootstrap_parameter_dimensions(&parameter.parameter);
        let (selected_count, selected_probability) = match receipt.selected_test_tail {
            PlsBootstrapTestTail::TwoSided => {
                (parameter.two_sided_exceedances, parameter.p_value_two_sided)
            }
            PlsBootstrapTestTail::OneSidedGreater => (
                parameter.greater_or_equal_exceedances,
                parameter.p_value_greater,
            ),
            PlsBootstrapTestTail::OneSidedLess => {
                (parameter.less_or_equal_exceedances, parameter.p_value_less)
            }
        };
        for (metric, value) in [
            ("usable_replicates", parameter.usable_replicates.to_string()),
            ("selected_exceedances", selected_count.to_string()),
            ("selected_p_value", selected_probability.to_string()),
        ] {
            rows.push(row(
                "pls_bootstrap_test_tail_parameter",
                &construct,
                &indicator,
                &source,
                &target,
                metric,
                value,
            ));
        }
    }
    Ok(())
}

fn pls_bootstrap_parameter_dimensions(parameter: &str) -> (String, String, String, String) {
    let identity = PlsResamplingParameterIdentity::decode(parameter)
        .expect("the strict test-tail validator checked canonical parameter identities");
    let components = identity.components();
    match identity.family() {
        PlsResamplingParameterFamily::OuterLoading | PlsResamplingParameterFamily::OuterWeight => (
            components[0].clone(),
            components[1].clone(),
            String::new(),
            String::new(),
        ),
        PlsResamplingParameterFamily::RSquared => (
            components[0].clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        PlsResamplingParameterFamily::Path
        | PlsResamplingParameterFamily::DirectEffect
        | PlsResamplingParameterFamily::IndirectEffect
        | PlsResamplingParameterFamily::TotalEffect => (
            String::new(),
            String::new(),
            components[0].clone(),
            components[1].clone(),
        ),
    }
}

fn push_plsc_consistent_permutation(
    result: &AnalysisResult,
    estimation: &serde_json::Value,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    let has_method_marker = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION);
    let has_scheduler_marker = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION);
    let selected_tail_marker_count = result
        .provenance
        .method_version
        .split('+')
        .filter(|version| *version == PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION)
        .count();
    let has_selected_tail_marker = selected_tail_marker_count != 0;
    let permutation = match &result.payload {
        AnalysisPayload::PlsPmV3 { permutation, .. } => permutation.as_ref(),
        _ => None,
    };
    let artifact_declares_method = permutation
        .and_then(|value| value.get("method_version"))
        .and_then(serde_json::Value::as_str)
        == Some(PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION);
    let artifact_declares_selected_tail =
        permutation.is_some_and(|value| value.get("selected_tail_inference").is_some());
    let expected = result.provenance.method == AnalysisMethod::Plsc
        && result.provenance.settings.permutation_samples > 0;
    if !expected {
        if has_method_marker
            || has_scheduler_marker
            || has_selected_tail_marker
            || artifact_declares_method
            || artifact_declares_selected_tail
        {
            bail!("PLSc consistent-permutation payload and recipe attribution disagree");
        }
        return Ok(());
    }
    if !has_method_marker || !has_scheduler_marker || !artifact_declares_method {
        bail!("PLSc permutation result is missing exact consistent-permutation attribution");
    }
    if has_selected_tail_marker != artifact_declares_selected_tail || selected_tail_marker_count > 1
    {
        bail!("PLSc selected-tail receipt and runner method attribution disagree");
    }

    let point: qpls_estimation::PlsResult = serde_json::from_value(estimation.clone())
        .context("invalid PLSc point payload for consistent-permutation export")?;
    let permutation: PlscConsistentPermutationResult = serde_json::from_value(
        permutation
            .expect("expected PLSc consistent-permutation payload was checked")
            .clone(),
    )
    .context("invalid PLSc consistent-permutation payload for export")?;
    validate_plsc_consistent_permutation_result_for_settings(
        &permutation,
        &point,
        &result.provenance.settings,
    )
    .map_err(anyhow::Error::msg)
    .context("PLSc consistent-permutation export contract failed")?;

    if let Some(selected) = &permutation.selected_tail_inference {
        for (metric, value) in [
            ("method_version", selected.method_version.clone()),
            ("orientation", selected.orientation.clone()),
            (
                "selected_test_tail",
                serde_json::to_value(selected.selected_test_tail)?
                    .as_str()
                    .expect("typed PLSc selected tail serializes as text")
                    .to_owned(),
            ),
        ] {
            rows.push(row(
                "plsc_permutation_selected_tail",
                "",
                "",
                "",
                "",
                metric,
                value,
            ));
        }
        for parameter in &selected.parameters {
            let dimensions = plsc_bootstrap_parameter_dimensions(&parameter.parameter);
            for (metric, value) in [
                (
                    "selected_exceedances",
                    parameter.selected_exceedances.to_string(),
                ),
                ("selected_p_value", parameter.selected_p_value.to_string()),
                ("usable_permutations", parameter.permutations.to_string()),
            ] {
                rows.push(row(
                    "plsc_permutation_selected_tail_parameter",
                    &dimensions.0,
                    &dimensions.1,
                    &dimensions.2,
                    &dimensions.3,
                    metric,
                    value,
                ));
            }
        }
    }

    for (metric, value) in [
        ("method_version", permutation.method_version.clone()),
        (
            "estimator_method_version",
            permutation.estimator_method_version.clone(),
        ),
        (
            "scheduler_method_version",
            permutation.scheduler_method_version.clone(),
        ),
        ("operation", permutation.plan.operation.clone()),
        ("test_method", permutation.test_method.clone()),
        (
            "significance_level",
            permutation.significance_level.to_string(),
        ),
        (
            "requested_permutations",
            permutation.plan.permutations.to_string(),
        ),
        ("master_seed", permutation.plan.master_seed.to_string()),
        (
            "minimum_usable_fraction",
            permutation.minimum_usable_fraction.to_string(),
        ),
        ("retry_policy", permutation.retry_policy.clone()),
        ("group_column", permutation.group_column.clone()),
        (
            "pooled_parameter_values_sha256",
            permutation.pooled_parameter_values_sha256.clone(),
        ),
        (
            "usable_permutations",
            permutation.usable_permutations.to_string(),
        ),
        (
            "failed_permutations",
            permutation.failed_permutations.len().to_string(),
        ),
    ] {
        rows.push(row(
            "plsc_permutation_accounting",
            "",
            "",
            "",
            "",
            metric,
            value,
        ));
    }
    if let Some(directional) = &permutation.directional_inference {
        for (metric, value) in [
            (
                "directional_method_version",
                directional.method_version.clone(),
            ),
            ("directional_test_method", directional.test_method.clone()),
        ] {
            rows.push(row(
                "plsc_permutation_accounting",
                "",
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for (role, group) in [
        ("group_a", &permutation.group_a),
        ("group_b", &permutation.group_b),
    ] {
        for (metric, value) in [
            ("group", group.group.clone()),
            ("observations", group.observations.to_string()),
            (
                "parameter_values_sha256",
                group.parameter_values_sha256.clone(),
            ),
        ] {
            rows.push(row(
                "plsc_permutation_group",
                role,
                &group.group,
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for (parameter_index, parameter) in permutation.parameters.iter().enumerate() {
        let dimensions = plsc_bootstrap_parameter_dimensions(&parameter.parameter);
        let family = serde_json::to_value(parameter.family)?
            .as_str()
            .expect("PLSc permutation parameter family serializes as a string")
            .to_owned();
        for (metric, value) in [
            ("family", family),
            ("estimate_a", parameter.estimate_a.to_string()),
            ("estimate_b", parameter.estimate_b.to_string()),
            ("difference_a_minus_b", parameter.original.to_string()),
            ("exceedances", parameter.exceedances.to_string()),
            ("p_value_two_sided", parameter.p_value_two_sided.to_string()),
            ("usable_permutations", parameter.permutations.to_string()),
        ] {
            rows.push(row(
                "plsc_permutation_parameter",
                &dimensions.0,
                &dimensions.1,
                &dimensions.2,
                &dimensions.3,
                metric,
                value,
            ));
        }
        if let Some(directional) = permutation
            .directional_inference
            .as_ref()
            .map(|inference| &inference.parameters[parameter_index])
        {
            for (metric, value) in [
                ("greater_or_equal", directional.greater_or_equal.to_string()),
                ("less_or_equal", directional.less_or_equal.to_string()),
                ("p_value_greater", directional.p_value_greater.to_string()),
                ("p_value_less", directional.p_value_less.to_string()),
            ] {
                rows.push(row(
                    "plsc_permutation_parameter",
                    &dimensions.0,
                    &dimensions.1,
                    &dimensions.2,
                    &dimensions.3,
                    metric,
                    value,
                ));
            }
        }
    }
    for entry in &permutation.permutation_ledger {
        let status = serde_json::to_value(entry.status)?
            .as_str()
            .expect("PLSc permutation status serializes as a string")
            .to_owned();
        for (metric, value) in [
            ("status", status),
            (
                "label_assignment_sha256",
                entry.label_assignment_sha256.clone(),
            ),
            (
                "parameter_values_sha256",
                entry.parameter_values_sha256.clone().unwrap_or_default(),
            ),
            ("reason_code", entry.reason_code.clone().unwrap_or_default()),
            ("message", entry.message.clone().unwrap_or_default()),
        ] {
            rows.push(row(
                "plsc_permutation_ledger",
                &entry.permutation_index.to_string(),
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for warning in &permutation.warnings {
        rows.push(row(
            "plsc_permutation_warning",
            "",
            "",
            "",
            "",
            "warning",
            warning.clone(),
        ));
    }
    Ok(())
}

fn push_plsc_consistent_bootstrap(
    result: &AnalysisResult,
    estimation: &serde_json::Value,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    let marker = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION);
    let bootstrap = match &result.payload {
        AnalysisPayload::PlsPmV2 { bootstrap, .. } => Some(bootstrap),
        _ => None,
    };
    let artifact_declares_method = bootstrap
        .and_then(|value| value.get("method_version"))
        .and_then(|value| value.as_str())
        == Some(PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION);
    let expected = result.provenance.method == AnalysisMethod::Plsc
        && result.provenance.settings.bootstrap_samples > 0;
    if !expected {
        if marker || artifact_declares_method {
            bail!("PLSc consistent-bootstrap payload and recipe attribution disagree");
        }
        return Ok(());
    }
    if !marker || !artifact_declares_method {
        bail!("PLSc bootstrap result is missing exact consistent-bootstrap attribution");
    }

    let point: qpls_estimation::PlsResult = serde_json::from_value(estimation.clone())
        .context("invalid PLSc point payload for consistent-bootstrap export")?;
    let bootstrap: PlscConsistentBootstrapResult = serde_json::from_value(
        bootstrap
            .expect("expected PLSc bootstrap payload was checked")
            .clone(),
    )
    .context("invalid PLSc consistent-bootstrap payload for export")?;
    validate_plsc_consistent_bootstrap_result(&bootstrap, &point, &result.provenance.settings)
        .map_err(anyhow::Error::msg)
        .context("PLSc consistent-bootstrap export contract failed")?;

    for (metric, value) in [
        ("method_version", bootstrap.method_version.clone()),
        (
            "estimator_method_version",
            bootstrap.estimator_method_version.clone(),
        ),
        (
            "resampling_method_version",
            bootstrap.resampling_method_version.clone(),
        ),
        ("operation", bootstrap.plan.operation.clone()),
        (
            "requested_replicates",
            bootstrap.plan.replicates.to_string(),
        ),
        (
            "attempted_replicates",
            bootstrap.plan.replicates.to_string(),
        ),
        ("master_seed", bootstrap.plan.master_seed.to_string()),
        (
            "minimum_usable_fraction",
            bootstrap.minimum_usable_fraction.to_string(),
        ),
        ("retry_policy", bootstrap.retry_policy.clone()),
        (
            "original_parameter_values_sha256",
            bootstrap.original_parameter_values_sha256.clone(),
        ),
        ("usable_replicates", bootstrap.usable_replicates.to_string()),
        (
            "successful_replicate_witnesses",
            bootstrap.successful_replicates.len().to_string(),
        ),
        (
            "failed_replicates",
            bootstrap.failed_replicates.len().to_string(),
        ),
        (
            "jackknife_case_count",
            bootstrap
                .bca
                .as_ref()
                .map(|artifact| artifact.jackknife_case_count.to_string())
                .unwrap_or_default(),
        ),
        (
            "failed_jackknife_cases",
            bootstrap.failed_jackknife_cases.len().to_string(),
        ),
        (
            "successful_jackknife_witnesses",
            bootstrap.successful_jackknife_cases.len().to_string(),
        ),
    ] {
        rows.push(row(
            "plsc_bootstrap_accounting",
            "",
            "",
            "",
            "",
            metric,
            value,
        ));
    }

    for parameter in &bootstrap.percentile.parameters {
        let dimensions = plsc_bootstrap_parameter_dimensions(&parameter.parameter);
        for (metric, value) in [
            ("original", parameter.original.to_string()),
            ("bootstrap_mean", parameter.bootstrap_mean.to_string()),
            ("bias", parameter.bias.to_string()),
            ("standard_error", parameter.standard_error.to_string()),
            ("lower", parameter.lower.to_string()),
            ("upper", parameter.upper.to_string()),
            ("usable_replicates", parameter.usable_replicates.to_string()),
            (
                "t_statistic",
                parameter
                    .t_statistic
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "p_value_two_sided",
                parameter
                    .p_value_two_sided
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
        ] {
            rows.push(row(
                "plsc_bootstrap_percentile",
                &dimensions.0,
                &dimensions.1,
                &dimensions.2,
                &dimensions.3,
                metric,
                value,
            ));
        }
    }

    if let Some(bca) = &bootstrap.bca {
        for parameter in &bca.parameters {
            let dimensions = plsc_bootstrap_parameter_dimensions(&parameter.parameter);
            for (metric, value) in [
                (
                    "bias_correction",
                    parameter
                        .bias_correction
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "acceleration",
                    parameter
                        .acceleration
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "lower",
                    parameter
                        .lower
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "upper",
                    parameter
                        .upper
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "unavailable_reason",
                    parameter.unavailable_reason.clone().unwrap_or_default(),
                ),
            ] {
                rows.push(row(
                    "plsc_bootstrap_bca",
                    &dimensions.0,
                    &dimensions.1,
                    &dimensions.2,
                    &dimensions.3,
                    metric,
                    value,
                ));
            }
        }
    }

    for failure in &bootstrap.failed_replicates {
        for (metric, value) in [
            ("reason_code", failure.reason_code.clone()),
            ("message", failure.message.clone()),
            (
                "sample_indices_sha256",
                failure.sample_indices_sha256.clone(),
            ),
        ] {
            rows.push(row(
                "plsc_bootstrap_failure",
                &failure.replicate_index.to_string(),
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for failure in &bootstrap.failed_jackknife_cases {
        for (metric, value) in [
            ("reason_code", failure.reason_code.clone()),
            ("message", failure.message.clone()),
        ] {
            rows.push(row(
                "plsc_bootstrap_jackknife_failure",
                &failure.omitted_case.to_string(),
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for warning in &bootstrap.warnings {
        rows.push(row(
            "plsc_bootstrap_warning",
            "",
            "",
            "",
            "",
            "warning",
            warning.clone(),
        ));
    }
    Ok(())
}

fn plsc_bootstrap_parameter_dimensions(parameter: &str) -> (String, String, String, String) {
    let Ok((kind, parts)) = serde_json::from_str::<(String, Vec<String>)>(parameter) else {
        return (
            parameter.into(),
            String::new(),
            String::new(),
            String::new(),
        );
    };
    match (kind.as_str(), parts.as_slice()) {
        ("plsc_outer_loading" | "plsc_outer_weight", [construct, indicator]) => (
            construct.clone(),
            indicator.clone(),
            String::new(),
            String::new(),
        ),
        ("plsc_rho_a" | "plsc_r_squared", [construct]) => (
            construct.clone(),
            String::new(),
            String::new(),
            String::new(),
        ),
        (
            "plsc_construct_correlation"
            | "plsc_path"
            | "plsc_direct_effect"
            | "plsc_indirect_effect"
            | "plsc_total_effect",
            [source, target],
        ) => (String::new(), String::new(), source.clone(), target.clone()),
        _ => (
            parameter.into(),
            String::new(),
            String::new(),
            String::new(),
        ),
    }
}

fn push_pls_model_fit_and_exact(
    result: &AnalysisResult,
    estimation: &serde_json::Value,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    let (assessment, bootstrap) = match &result.payload {
        AnalysisPayload::PlsPmV1 { assessment, .. } => (assessment, None),
        AnalysisPayload::PlsPmV2 {
            assessment,
            bootstrap,
            ..
        } => (assessment, Some(bootstrap)),
        AnalysisPayload::PlsPmV3 {
            assessment,
            bootstrap,
            ..
        } => (assessment, bootstrap.as_ref()),
        _ => return Ok(()),
    };
    let assessment: AssessmentResult = serde_json::from_value(assessment.clone())
        .context("invalid PLS assessment payload for model-fit export")?;
    let raw_exact = bootstrap
        .and_then(|value| value.get("model_fit_exact_inference"))
        .filter(|value| !value.is_null());
    let has_exact_marker = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == PLS_MODEL_FIT_EXACT_METHOD_VERSION);
    let Some(point_fit) = assessment.model_fit.as_ref() else {
        if raw_exact.is_some() || has_exact_marker {
            bail!("PLS model-fit exact payload has no linked point model-fit result");
        }
        return Ok(());
    };

    // Historical assessment payloads predate the matrix-backed v2 contract.
    // Keep their descriptive SRMR/d_ULS values exportable without attributing
    // v2 or exact-fit semantics to them.
    if point_fit.method_version.is_empty() {
        if raw_exact.is_some() || has_exact_marker {
            bail!("PLS model-fit exact payload cannot be linked to a historical point result");
        }
        push_pls_model_fit_legacy_rows(point_fit, rows);
        return Ok(());
    }
    if point_fit.method_version != PLS_MODEL_FIT_METHOD_VERSION {
        bail!("PLS model-fit point payload has an unsupported method identity");
    }

    let point: qpls_estimation::PlsResult = serde_json::from_value(estimation.clone())
        .context("invalid PLS point payload for model-fit export")?;
    if !pls_model_fit_matches_v2_contract(point_fit, point.used_observations) {
        bail!("PLS model-fit v2 payload failed matrix-backed semantic validation");
    }

    let exact = raw_exact
        .map(|value| {
            serde_json::from_value::<PlsModelFitExactInference>(value.clone())
                .context("invalid PLS model-fit exact inference payload")
        })
        .transpose()?;
    match (exact.as_ref(), has_exact_marker) {
        (Some(exact), true) => {
            let expected_bootstrap_method = match result.provenance.settings.method {
                AnalysisMethod::PlsPm => RESAMPLING_METHOD_VERSION,
                AnalysisMethod::Plsc => PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION,
                _ => bail!("PLS model-fit exact inference has incompatible method provenance"),
            };
            if bootstrap
                .and_then(|value| value.get("method_version"))
                .and_then(serde_json::Value::as_str)
                != Some(expected_bootstrap_method)
            {
                bail!("PLS model-fit exact inference has incompatible outer bootstrap identity");
            }
            validate_pls_model_fit_exact_inference_for_settings(
                exact,
                point_fit,
                &point,
                &result.provenance.settings,
            )
            .map_err(anyhow::Error::msg)
            .context("PLS model-fit exact export contract failed")?;
        }
        (None, false) => {}
        _ => bail!("PLS model-fit exact payload and provenance identity disagree"),
    }

    push_pls_model_fit_v2_rows(point_fit, exact.as_ref(), rows);
    if let Some(exact) = exact.as_ref() {
        push_pls_model_fit_exact_rows(exact, rows);
    }
    Ok(())
}

fn push_pls_model_fit_legacy_rows(fit: &PlsModelFit, rows: &mut Vec<ExportRow>) {
    for (variant, measures) in [("saturated", &fit.saturated), ("estimated", &fit.estimated)] {
        for (metric, value) in [("srmr", measures.srmr), ("d_uls", measures.d_uls)] {
            rows.push(row(
                "pls_model_fit_legacy",
                variant,
                "",
                "",
                "",
                metric,
                value.to_string(),
            ));
        }
    }
}

fn push_pls_model_fit_v2_rows(
    fit: &PlsModelFit,
    exact: Option<&PlsModelFitExactInference>,
    rows: &mut Vec<ExportRow>,
) {
    for (metric, value) in [
        ("method_version", fit.method_version.clone()),
        (
            "analytical_sample_size",
            fit.analytical_sample_size.to_string(),
        ),
        ("indicator_count", fit.indicator_order.len().to_string()),
        ("indicator_order", fit.indicator_order.join("|")),
        ("matrix_convention", fit.matrix_convention.clone()),
        ("geodesic_logarithm", fit.geodesic_logarithm.clone()),
        (
            "exact_fit_inference",
            if exact.is_some() {
                "available_in_experimental_run".into()
            } else {
                "unavailable_for_run".into()
            },
        ),
    ] {
        rows.push(row("pls_model_fit_detail", "", "", "", "", metric, value));
    }
    push_pls_fit_criterion_value("null", "chi_square", &fit.null_model_chi_square, rows);
    for (variant, measures) in [("saturated", &fit.saturated), ("estimated", &fit.estimated)] {
        for (metric, value) in [("srmr", measures.srmr), ("d_uls", measures.d_uls)] {
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                metric,
                value.to_string(),
            ));
        }
        for (metric, value) in [
            ("d_g", &measures.d_g),
            ("chi_square", &measures.chi_square),
            ("degrees_of_freedom", &measures.degrees_of_freedom),
            ("nfi", &measures.nfi),
        ] {
            push_pls_fit_criterion_value(variant, metric, value, rows);
        }
    }
}

fn push_pls_fit_criterion_value(
    variant: &str,
    metric: &str,
    value: &FitCriterionValue,
    rows: &mut Vec<ExportRow>,
) {
    match value {
        FitCriterionValue::Available { value } => {
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                metric,
                value.to_string(),
            ));
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                &format!("{metric}_status"),
                "available".into(),
            ));
        }
        FitCriterionValue::Unavailable { reason_code } => {
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                metric,
                String::new(),
            ));
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                &format!("{metric}_status"),
                "unavailable".into(),
            ));
            rows.push(row(
                "pls_model_fit",
                variant,
                "",
                "",
                "",
                &format!("{metric}_reason_code"),
                reason_code.clone(),
            ));
        }
    }
}

fn push_pls_model_fit_exact_rows(exact: &PlsModelFitExactInference, rows: &mut Vec<ExportRow>) {
    for (metric, value) in [
        ("method_version", exact.method_version.clone()),
        (
            "point_fit_method_version",
            exact.point_fit_method_version.clone(),
        ),
        (
            "estimator_method_version",
            exact.estimator_method_version.clone(),
        ),
        (
            "resampling_method_version",
            exact.resampling_method_version.clone(),
        ),
        ("procedure", exact.procedure.clone()),
        ("transformation", exact.transformation.clone()),
        ("matrix_power", exact.matrix_power.clone()),
        ("quantile_method", exact.quantile_method.clone()),
        ("decision_rule", exact.decision_rule.clone()),
        ("retry_policy", exact.retry_policy.clone()),
        ("sample_digest_method", exact.sample_digest_method.clone()),
        (
            "usable_index_digest_method",
            exact.usable_index_digest_method.clone(),
        ),
        ("matrix_digest_method", exact.matrix_digest_method.clone()),
        ("status", pls_model_fit_exact_status(exact.status).into()),
        (
            "analytical_sample_size",
            exact.analytical_sample_size.to_string(),
        ),
        ("master_seed", exact.master_seed.to_string()),
        (
            "requested_replicates_per_model",
            exact.requested_replicates.to_string(),
        ),
        (
            "minimum_usable_fraction",
            exact.minimum_usable_fraction.to_string(),
        ),
        (
            "observed_correlation_sha256",
            exact.observed_correlation_sha256.clone(),
        ),
    ] {
        rows.push(row(
            "pls_model_fit_exact_detail",
            "",
            "",
            "",
            "",
            metric,
            value,
        ));
    }
    push_pls_model_fit_exact_variant_rows(&exact.saturated, rows);
    push_pls_model_fit_exact_variant_rows(&exact.estimated, rows);
}

fn push_pls_model_fit_exact_variant_rows(
    variant: &PlsModelFitExactVariantInference,
    rows: &mut Vec<ExportRow>,
) {
    for criterion in &variant.criteria {
        let criterion_label = pls_model_fit_exact_criterion(criterion.criterion);
        for (metric, value) in [
            (
                "status",
                pls_model_fit_exact_status(criterion.status).into(),
            ),
            ("original", criterion.original.to_string()),
            (
                "requested_replicates",
                criterion.requested_replicates.to_string(),
            ),
            (
                "minimum_usable_replicates",
                criterion.minimum_usable_replicates.to_string(),
            ),
            ("usable_replicates", criterion.usable_replicates.to_string()),
            ("failed_replicates", criterion.failed_replicates.to_string()),
            (
                "usable_replicate_indices_sha256",
                criterion.usable_replicate_indices_sha256.clone(),
            ),
            (
                "replicate_min",
                criterion
                    .replicate_min
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "replicate_max",
                criterion
                    .replicate_max
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "upper_95",
                criterion
                    .upper_95
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "upper_99",
                criterion
                    .upper_99
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "decision_5_percent",
                pls_model_fit_exact_decision(criterion.not_rejected_95).into(),
            ),
            (
                "decision_1_percent",
                pls_model_fit_exact_decision(criterion.not_rejected_99).into(),
            ),
            (
                "exceed_or_equal_count",
                criterion.exceed_or_equal_count.to_string(),
            ),
            (
                "empirical_upper_tail_probability",
                criterion
                    .empirical_upper_tail_probability
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "unavailable_reason_code",
                criterion
                    .unavailable_reason_code
                    .clone()
                    .unwrap_or_default(),
            ),
        ] {
            rows.push(row(
                "pls_model_fit_exact",
                &variant.variant,
                criterion_label,
                "",
                "",
                metric,
                value,
            ));
        }
    }

    for entry in variant
        .ledger
        .iter()
        .filter(|entry| entry.status != PlsModelFitExactReplicateStatus::Success)
    {
        let replicate = entry.replicate_index.to_string();
        for (metric, value) in [
            (
                "status",
                pls_model_fit_exact_replicate_status(entry.status).into(),
            ),
            ("sample_indices_sha256", entry.sample_indices_sha256.clone()),
            (
                "failure_reason_code",
                entry.failure_reason_code.clone().unwrap_or_default(),
            ),
            (
                "failure_message",
                entry.failure_message.clone().unwrap_or_default(),
            ),
        ] {
            rows.push(row(
                "pls_model_fit_exact_failure",
                &variant.variant,
                &replicate,
                "",
                "",
                metric,
                value,
            ));
        }
        for failure in &entry.criterion_failures {
            rows.push(row(
                "pls_model_fit_exact_failure",
                &variant.variant,
                &replicate,
                pls_model_fit_exact_criterion(failure.criterion),
                "",
                "criterion_failure_reason_code",
                failure.reason_code.clone(),
            ));
        }
    }
}

fn pls_model_fit_exact_criterion(criterion: PlsModelFitExactCriterion) -> &'static str {
    match criterion {
        PlsModelFitExactCriterion::Srmr => "srmr",
        PlsModelFitExactCriterion::DULS => "d_uls",
        PlsModelFitExactCriterion::DG => "d_g",
    }
}

fn pls_model_fit_exact_status(status: PlsModelFitExactStatus) -> &'static str {
    match status {
        PlsModelFitExactStatus::Available => "available",
        PlsModelFitExactStatus::Partial => "partial",
        PlsModelFitExactStatus::Unavailable => "unavailable",
    }
}

fn pls_model_fit_exact_replicate_status(status: PlsModelFitExactReplicateStatus) -> &'static str {
    match status {
        PlsModelFitExactReplicateStatus::Success => "success",
        PlsModelFitExactReplicateStatus::Partial => "partial",
        PlsModelFitExactReplicateStatus::Failed => "failed",
    }
}

fn pls_model_fit_exact_decision(not_rejected: Option<bool>) -> &'static str {
    match not_rejected {
        Some(true) => "not_rejected",
        Some(false) => "rejected",
        None => "unavailable",
    }
}

fn push_htmt_assessment_and_inference(
    result: &AnalysisResult,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    let (assessment, bootstrap) = match &result.payload {
        AnalysisPayload::PlsPmV1 { assessment, .. } => (assessment, None),
        AnalysisPayload::PlsPmV2 {
            assessment,
            bootstrap,
            ..
        } => (assessment, Some(bootstrap)),
        AnalysisPayload::PlsPmV3 {
            assessment,
            bootstrap,
            ..
        } => (assessment, bootstrap.as_ref()),
        _ => return Ok(()),
    };
    let assessment: AssessmentResult = serde_json::from_value(assessment.clone())
        .context("invalid PLS assessment payload for HTMT export")?;
    let (Some(plus), Some(original)) = (
        assessment.htmt_plus.as_ref(),
        assessment.htmt_original.as_ref(),
    ) else {
        if assessment.htmt_plus.is_none()
            && assessment.htmt_original.is_none()
            && assessment.htmt_plus_method_version.is_none()
            && assessment.htmt_original_method_version.is_none()
        {
            return Ok(());
        }
        bail!("PLS assessment has an incomplete explicit HTMT payload");
    };
    if assessment.htmt_plus_method_version.as_deref() != Some(HTMT_PLUS_METHOD_VERSION)
        || assessment.htmt_original_method_version.as_deref() != Some(HTMT_ORIGINAL_METHOD_VERSION)
        || !validate_htmt_export_matrix(plus, true)
        || !validate_htmt_export_matrix(original, false)
        || plus.constructs != original.constructs
    {
        bail!("PLS assessment has an inconsistent HTMT/HTMT+ identity");
    }
    push_htmt_point_rows("htmt_plus", HTMT_PLUS_METHOD_VERSION, plus, rows);
    push_htmt_point_rows(
        "htmt_original",
        HTMT_ORIGINAL_METHOD_VERSION,
        original,
        rows,
    );

    let has_inference_version = result
        .provenance
        .method_version
        .split('+')
        .any(|version| version == HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION);
    let parsed_bootstrap = bootstrap
        .map(|value| {
            serde_json::from_value::<PlsBootstrapResult>(value.clone())
                .context("invalid PLS bootstrap payload for HTMT export")
        })
        .transpose()?;
    match (
        parsed_bootstrap
            .as_ref()
            .and_then(|bootstrap| bootstrap.htmt_inference.as_ref()),
        has_inference_version,
    ) {
        (Some(bundle), true) => {
            if !validate_htmt_export_bundle(
                bundle,
                plus,
                original,
                parsed_bootstrap
                    .as_ref()
                    .expect("HTMT bundle came from parsed bootstrap"),
            ) {
                bail!("PLS bootstrap has an inconsistent complete HTMT inference payload");
            }
            push_htmt_inference_rows("htmt_plus_bootstrap", &bundle.htmt_plus, rows);
            push_htmt_inference_rows("htmt_original_bootstrap", &bundle.htmt_original, rows);
        }
        (None, false) => {}
        _ => bail!("HTMT bootstrap inference payload and provenance identity disagree"),
    }
    Ok(())
}

fn validate_htmt_export_matrix(artifact: &HtmtAssessment, absolute: bool) -> bool {
    let dimension = artifact.constructs.len();
    artifact.correlation_type == "pearson"
        && artifact.absolute_correlations == absolute
        && artifact.cells.len() == dimension
        && artifact.cells.iter().all(|row| row.len() == dimension)
        && artifact.cells.iter().enumerate().all(|(row, cells)| {
            cells.iter().enumerate().all(|(column, cell)| {
                let mirror = &artifact.cells[column][row];
                cell.status == mirror.status
                    && cell.reason == mirror.reason
                    && match (cell.value, mirror.value) {
                        (Some(left), Some(right)) => {
                            left.is_finite()
                                && right.is_finite()
                                && (!absolute || (left >= 0.0 && right >= 0.0))
                                && float_matches(left, right)
                                && (row != column || left == 1.0)
                        }
                        (None, None) => cell.status != HtmtStatus::Available,
                        _ => false,
                    }
            })
        })
}

fn push_htmt_point_rows(
    section: &str,
    method_version: &str,
    artifact: &HtmtAssessment,
    rows: &mut Vec<ExportRow>,
) {
    rows.push(row(
        section,
        "",
        "",
        "",
        "",
        "method_version",
        method_version.into(),
    ));
    rows.push(row(
        section,
        "",
        "",
        "",
        "",
        "correlation_policy",
        if artifact.absolute_correlations {
            "absolute Pearson indicator correlations"
        } else {
            "signed Pearson indicator correlations"
        }
        .into(),
    ));
    for row_index in 1..artifact.constructs.len() {
        for column_index in 0..row_index {
            let cell = &artifact.cells[row_index][column_index];
            let source = &artifact.constructs[column_index];
            let target = &artifact.constructs[row_index];
            for (metric, value) in [
                ("status", htmt_status_label(cell.status).into()),
                (
                    "value",
                    cell.value
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                ("reason", cell.reason.clone().unwrap_or_default()),
            ] {
                rows.push(row(section, "", "", source, target, metric, value));
            }
        }
    }
}

fn validate_htmt_export_bundle(
    bundle: &HtmtBootstrapInferenceBundle,
    plus: &HtmtAssessment,
    original: &HtmtAssessment,
    bootstrap: &PlsBootstrapResult,
) -> bool {
    let globally_failed = bootstrap
        .failed_replicates
        .iter()
        .map(|failure| failure.replicate_index)
        .collect::<std::collections::HashSet<_>>();
    globally_failed.len() == bootstrap.failed_replicates.len()
        && bundle.method_version == HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION
        && validate_htmt_export_inference(
            &bundle.htmt_plus,
            plus,
            HTMT_PLUS_BOOTSTRAP_METHOD_VERSION,
            HTMT_PLUS_METHOD_VERSION,
            true,
            &globally_failed,
        )
        && validate_htmt_export_inference(
            &bundle.htmt_original,
            original,
            HTMT_ORIGINAL_BOOTSTRAP_METHOD_VERSION,
            HTMT_ORIGINAL_METHOD_VERSION,
            false,
            &globally_failed,
        )
}

fn validate_htmt_export_inference(
    inference: &HtmtBootstrapInference,
    point: &HtmtAssessment,
    method_version: &str,
    point_method_version: &str,
    absolute: bool,
    globally_failed: &std::collections::HashSet<u32>,
) -> bool {
    let dimension = point.constructs.len();
    inference.method_version == method_version
        && inference.point_method_version == point_method_version
        && inference.constructs == point.constructs
        && inference.correlation_type == "pearson"
        && inference.absolute_correlations == absolute
        && inference.interval_method == HTMT_BOOTSTRAP_INTERVAL_METHOD
        && inference.test_type == HTMT_BOOTSTRAP_TEST_TYPE
        && inference.significance_level == HTMT_BOOTSTRAP_SIGNIFICANCE_LEVEL
        && inference.equivalent_two_sided_confidence_level
            == HTMT_BOOTSTRAP_EQUIVALENT_TWO_SIDED_CONFIDENCE_LEVEL
        && inference.critical_value == HTMT_BOOTSTRAP_CRITICAL_VALUE
        && inference.decision_rule == HTMT_BOOTSTRAP_DECISION_RULE
        && inference.replicate_index_digest_method == HTMT_BOOTSTRAP_REPLICATE_INDEX_DIGEST_METHOD
        && inference.requested_replicates >= 2
        && inference.minimum_usable_replicates >= 2
        && inference.cells.len() == dimension
        && inference.cells.iter().all(|row| row.len() == dimension)
        && inference.cells.iter().enumerate().all(|(row, cells)| {
            cells.iter().enumerate().all(|(column, cell)| {
                if cell != &inference.cells[column][row]
                    || cell.usable_replicates + cell.failed_replicates
                        > inference.requested_replicates
                {
                    return false;
                }
                let mut pair_unavailable = std::collections::HashSet::new();
                if cell.pair_unavailable_replicates.iter().any(|entry| {
                    entry.replicate_index >= inference.requested_replicates
                        || globally_failed.contains(&entry.replicate_index)
                        || entry.reason_code.trim().is_empty()
                        || !pair_unavailable.insert(entry.replicate_index)
                }) {
                    return false;
                }
                let point_available =
                    point.cells[row][column].status == HtmtStatus::Available && row != column;
                if point_available
                    && (cell.usable_replicates as usize
                        + globally_failed.len()
                        + pair_unavailable.len()
                        != inference.requested_replicates as usize
                        || !cell
                            .usable_replicate_indices_sha256
                            .as_ref()
                            .is_some_and(|digest| {
                                digest.len() == 64
                                    && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                            }))
                {
                    return false;
                }
                match cell.status {
                    HtmtBootstrapInferenceStatus::Available => {
                        let values = [
                            cell.original,
                            cell.bootstrap_mean,
                            cell.bias,
                            cell.standard_error,
                            cell.bias_correction,
                            cell.lower,
                            cell.upper,
                            cell.replicate_min,
                            cell.replicate_max,
                        ];
                        values.iter().all(|value| value.is_some_and(f64::is_finite))
                            && cell.reason.is_none()
                            && cell.usable_replicates >= inference.minimum_usable_replicates
                            && cell.usable_replicates + cell.failed_replicates
                                == inference.requested_replicates
                            && cell.upper_bound_below_critical_value
                                == cell
                                    .upper
                                    .map(|upper| upper < HTMT_BOOTSTRAP_CRITICAL_VALUE)
                            && float_matches(
                                cell.bias.unwrap(),
                                cell.bootstrap_mean.unwrap() - cell.original.unwrap(),
                            )
                            && cell.lower.unwrap() <= cell.upper.unwrap()
                            && (!absolute
                                || [cell.original, cell.lower, cell.upper]
                                    .iter()
                                    .all(|value| value.unwrap() >= 0.0))
                    }
                    HtmtBootstrapInferenceStatus::NotApplicable
                    | HtmtBootstrapInferenceStatus::Unavailable => {
                        cell.reason
                            .as_ref()
                            .is_some_and(|reason| !reason.trim().is_empty())
                            && cell.bootstrap_mean.is_none()
                            && cell.bias.is_none()
                            && cell.standard_error.is_none()
                            && cell.bias_correction.is_none()
                            && cell.lower.is_none()
                            && cell.upper.is_none()
                            && cell.upper_bound_below_critical_value.is_none()
                    }
                }
            })
        })
}

fn push_htmt_inference_rows(
    section: &str,
    inference: &HtmtBootstrapInference,
    rows: &mut Vec<ExportRow>,
) {
    for (metric, value) in [
        ("method_version", inference.method_version.clone()),
        (
            "point_method_version",
            inference.point_method_version.clone(),
        ),
        ("interval_method", inference.interval_method.clone()),
        ("test_type", inference.test_type.clone()),
        (
            "significance_level",
            inference.significance_level.to_string(),
        ),
        (
            "equivalent_two_sided_confidence_level",
            inference.equivalent_two_sided_confidence_level.to_string(),
        ),
        ("critical_value", inference.critical_value.to_string()),
        ("decision_rule", inference.decision_rule.clone()),
        (
            "replicate_index_digest_method",
            inference.replicate_index_digest_method.clone(),
        ),
        (
            "requested_replicates",
            inference.requested_replicates.to_string(),
        ),
        (
            "minimum_usable_replicates",
            inference.minimum_usable_replicates.to_string(),
        ),
        ("retry_policy", inference.retry_policy.clone()),
    ] {
        rows.push(row(section, "", "", "", "", metric, value));
    }
    for row_index in 1..inference.constructs.len() {
        for column_index in 0..row_index {
            let cell = &inference.cells[row_index][column_index];
            let source = &inference.constructs[column_index];
            let target = &inference.constructs[row_index];
            let metrics = [
                ("status", htmt_inference_status_label(cell.status).into()),
                ("reason", cell.reason.clone().unwrap_or_default()),
                ("original", optional_f64(cell.original)),
                ("bootstrap_mean", optional_f64(cell.bootstrap_mean)),
                ("bias", optional_f64(cell.bias)),
                ("standard_error", optional_f64(cell.standard_error)),
                ("bias_correction", optional_f64(cell.bias_correction)),
                ("bias_corrected_90_lower", optional_f64(cell.lower)),
                ("bias_corrected_90_upper", optional_f64(cell.upper)),
                (
                    "upper_bound_below_critical_value",
                    cell.upper_bound_below_critical_value
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                ("usable_replicates", cell.usable_replicates.to_string()),
                ("failed_replicates", cell.failed_replicates.to_string()),
                (
                    "usable_replicate_indices_sha256",
                    cell.usable_replicate_indices_sha256
                        .clone()
                        .unwrap_or_default(),
                ),
                (
                    "pair_unavailable_replicates",
                    cell.pair_unavailable_replicates.len().to_string(),
                ),
            ];
            for (metric, value) in metrics {
                rows.push(row(section, "", "", source, target, metric, value));
            }
            for unavailable in &cell.pair_unavailable_replicates {
                rows.push(row(
                    section,
                    "",
                    "",
                    source,
                    target,
                    &format!(
                        "pair_unavailable_replicate_{}_reason",
                        unavailable.replicate_index
                    ),
                    unavailable.reason_code.clone(),
                ));
            }
        }
    }
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn htmt_status_label(status: HtmtStatus) -> &'static str {
    match status {
        HtmtStatus::Available => "available",
        HtmtStatus::NotApplicable => "not_applicable",
        HtmtStatus::Unavailable => "unavailable",
    }
}

fn htmt_inference_status_label(status: HtmtBootstrapInferenceStatus) -> &'static str {
    match status {
        HtmtBootstrapInferenceStatus::Available => "available",
        HtmtBootstrapInferenceStatus::NotApplicable => "not_applicable",
        HtmtBootstrapInferenceStatus::Unavailable => "unavailable",
    }
}

fn push_cbsem_bootstrap_v2(
    result: &AnalysisResult,
    estimation: &serde_json::Value,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    let Some(value) = estimation
        .get("cbsem")
        .and_then(|value| value.get("bootstrap_v2"))
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    let bootstrap: qpls_estimation::CbsemBootstrapAnalysisV2 =
        serde_json::from_value(value.clone())
            .context("invalid typed CB-SEM bootstrap_v2 result payload")?;
    validate_cbsem_bootstrap_v2_for_export(result, &bootstrap)?;

    rows.push(row(
        "cbsem_bootstrap_v2_setting",
        "",
        "",
        "",
        "",
        "standalone_integrity_scope",
        "Typed shape, frozen identities, accounting, inference status, and complete replicate-index coverage were checked. A standalone result does not contain the scientific recipe or raw data, so recipe, base-result, and sample-index witness hashes are exported as stored provenance rather than recomputed by this command.".into(),
    ));

    for (metric, value) in [
        ("method_version", bootstrap.method_version.clone()),
        ("algorithm", bootstrap.algorithm.clone()),
        ("interval_method", bootstrap.interval_method.clone()),
        ("retry_policy", bootstrap.retry_policy.clone()),
        ("confidence_level", bootstrap.confidence_level.to_string()),
        (
            "requested_replicates",
            bootstrap.requested_replicates.to_string(),
        ),
        ("attempted_fits", bootstrap.attempted_fits.to_string()),
        ("usable_replicates", bootstrap.usable_replicates.to_string()),
        ("failed_replicates", bootstrap.failed_replicates.to_string()),
        (
            "minimum_usable_fraction",
            bootstrap.minimum_usable_fraction.to_string(),
        ),
        (
            "minimum_usable_replicates",
            bootstrap.minimum_usable_replicates.to_string(),
        ),
        (
            "max_attempts_per_replicate",
            bootstrap.max_attempts_per_replicate.to_string(),
        ),
        (
            "complete_case_sample_size",
            bootstrap.complete_case_sample_size.to_string(),
        ),
        ("seed", bootstrap.seed.to_string()),
        ("stream_token", bootstrap.stream_token.clone()),
        (
            "outer_workers",
            result.provenance.settings.workers.to_string(),
        ),
    ] {
        rows.push(row(
            "cbsem_bootstrap_v2_setting",
            "",
            "",
            "",
            "",
            metric,
            value,
        ));
    }
    match &bootstrap.inference {
        qpls_estimation::CbsemBootstrapInferenceV2::Available => rows.push(row(
            "cbsem_bootstrap_v2_inference",
            "",
            "",
            "",
            "",
            "status",
            "available".into(),
        )),
        qpls_estimation::CbsemBootstrapInferenceV2::Unavailable {
            reason_code,
            message,
        } => {
            rows.push(row(
                "cbsem_bootstrap_v2_inference",
                "",
                "",
                "",
                "",
                "status",
                "unavailable".into(),
            ));
            rows.push(row(
                "cbsem_bootstrap_v2_inference",
                "",
                "",
                "",
                "",
                "reason_code",
                reason_code.clone(),
            ));
            rows.push(row(
                "cbsem_bootstrap_v2_inference",
                "",
                "",
                "",
                "",
                "message",
                message.clone(),
            ));
        }
    }
    for interval in &bootstrap.intervals {
        for (metric, value) in [
            ("original", interval.original),
            ("bootstrap_mean", interval.bootstrap_mean),
            ("bias", interval.bias),
            ("standard_error", interval.standard_error),
            ("percentile_lower", interval.percentile_lower),
            ("percentile_upper", interval.percentile_upper),
        ] {
            rows.push(row(
                "cbsem_bootstrap_v2_interval",
                &interval.parameter,
                "",
                "",
                "",
                metric,
                value.to_string(),
            ));
        }
        rows.push(row(
            "cbsem_bootstrap_v2_interval",
            &interval.parameter,
            "",
            "",
            "",
            "usable_replicates",
            interval.usable_replicates.to_string(),
        ));
    }
    for failure in &bootstrap.failures {
        let replicate = failure.replicate_index.to_string();
        for (metric, value) in [
            (
                "sample_indices_sha256",
                failure.sample_indices_sha256.clone(),
            ),
            ("reason_code", failure.reason_code.clone()),
            ("message", failure.message.clone()),
        ] {
            rows.push(row(
                "cbsem_bootstrap_v2_failure",
                &replicate,
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for (metric, value) in [
        (
            "method_version",
            bootstrap.validation_witness.method_version.clone(),
        ),
        (
            "dataset_fingerprint",
            bootstrap.validation_witness.dataset_fingerprint.clone(),
        ),
        (
            "recipe_sha256",
            bootstrap.validation_witness.recipe_sha256.clone(),
        ),
        (
            "base_result_sha256",
            bootstrap.validation_witness.base_result_sha256.clone(),
        ),
        (
            "parameter_names",
            serde_json::to_string(&bootstrap.validation_witness.parameter_names)?,
        ),
    ] {
        rows.push(row(
            "cbsem_bootstrap_v2_validation_witness",
            "",
            "",
            "",
            "",
            metric,
            value,
        ));
    }
    for witness in &bootstrap.validation_witness.successful_replicates {
        let replicate = witness.replicate_index.to_string();
        for (metric, value) in [
            (
                "sample_indices_sha256",
                witness.sample_indices_sha256.clone(),
            ),
            ("iterations", witness.iterations.to_string()),
            ("objective", witness.objective.to_string()),
            (
                "parameter_estimates",
                serde_json::to_string(&witness.parameter_estimates)?,
            ),
        ] {
            rows.push(row(
                "cbsem_bootstrap_v2_success_witness",
                &replicate,
                "",
                "",
                "",
                metric,
                value,
            ));
        }
    }
    for warning in &bootstrap.warnings {
        rows.push(row(
            "cbsem_bootstrap_v2_warning",
            "",
            "",
            "",
            "",
            "warning",
            warning.clone(),
        ));
    }
    Ok(())
}

fn validate_cbsem_bootstrap_v2_for_export(
    result: &AnalysisResult,
    bootstrap: &qpls_estimation::CbsemBootstrapAnalysisV2,
) -> Result<()> {
    use qpls_estimation::{
        CBSEM_BOOTSTRAP_ALGORITHM_V2, CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2,
        CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2, CBSEM_BOOTSTRAP_METHOD_VERSION_V2,
        CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2, CBSEM_BOOTSTRAP_RETRY_POLICY_V2,
        CBSEM_BOOTSTRAP_STREAM_TOKEN_V2, CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2,
    };

    if result.provenance.method != AnalysisMethod::Cbsem
        || !result
            .provenance
            .method_version
            .split('+')
            .any(|version| version == CBSEM_BOOTSTRAP_METHOD_VERSION_V2)
        || bootstrap.method_version != CBSEM_BOOTSTRAP_METHOD_VERSION_V2
        || bootstrap.algorithm != CBSEM_BOOTSTRAP_ALGORITHM_V2
        || bootstrap.interval_method != CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2
        || bootstrap.retry_policy != CBSEM_BOOTSTRAP_RETRY_POLICY_V2
        || bootstrap.stream_token != CBSEM_BOOTSTRAP_STREAM_TOKEN_V2
        || bootstrap.max_attempts_per_replicate != CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2
        || !float_matches(
            bootstrap.minimum_usable_fraction,
            CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2,
        )
        || bootstrap.validation_witness.method_version != CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2
    {
        bail!("typed CB-SEM bootstrap_v2 payload has an incompatible frozen identity");
    }
    let required =
        qpls_resampling::cbsem_bootstrap_required_usable_replicates(bootstrap.requested_replicates)
            as u32;
    if !(1_000..=10_000).contains(&bootstrap.requested_replicates)
        || bootstrap.attempted_fits != bootstrap.requested_replicates
        || bootstrap
            .usable_replicates
            .checked_add(bootstrap.failed_replicates)
            != Some(bootstrap.requested_replicates)
        || bootstrap.minimum_usable_replicates != required
        || bootstrap.complete_case_sample_size < 2
        || bootstrap.seed != result.provenance.seed
        || !bootstrap.confidence_level.is_finite()
        || bootstrap.confidence_level.to_bits() != 0.95_f64.to_bits()
        || bootstrap.confidence_level.to_bits()
            != result.provenance.settings.confidence_level.to_bits()
        || bootstrap.failures.len() as u32 != bootstrap.failed_replicates
        || bootstrap.validation_witness.successful_replicates.len() as u32
            != bootstrap.usable_replicates
        || bootstrap.validation_witness.dataset_fingerprint != result.provenance.dataset_fingerprint
        || !is_sha256_hex(&bootstrap.validation_witness.recipe_sha256)
        || !is_sha256_hex(&bootstrap.validation_witness.base_result_sha256)
        || bootstrap.validation_witness.parameter_names.is_empty()
    {
        bail!("typed CB-SEM bootstrap_v2 settings or accounting are inconsistent");
    }
    let parameter_names = bootstrap
        .validation_witness
        .parameter_names
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if parameter_names.len() != bootstrap.validation_witness.parameter_names.len()
        || parameter_names.iter().any(|name| name.trim().is_empty())
    {
        bail!("typed CB-SEM bootstrap_v2 witness parameter names are invalid");
    }

    let mut replicate_indices = std::collections::BTreeSet::new();
    let valid_failure_code = |code: &str| {
        matches!(
            code,
            "insufficient_complete_cases"
                | "constant_indicator"
                | "rank_deficient"
                | "singular_covariance"
                | "ml_nonconvergence"
                | "numerical_failure"
                | "inadmissible_or_unsupported_refit"
                | "invalid_indicator"
                | "ml_refit_error"
                | "missing_cbsem_payload"
                | "sample_size_mismatch"
                | "parameter_identity_mismatch"
                | "nonfinite_ml_fit"
        )
    };
    if bootstrap
        .failures
        .windows(2)
        .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
        || bootstrap
            .validation_witness
            .successful_replicates
            .windows(2)
            .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
    {
        bail!("typed CB-SEM bootstrap_v2 ledgers are not in replicate-index order");
    }
    for failure in &bootstrap.failures {
        if failure.replicate_index >= bootstrap.requested_replicates
            || !replicate_indices.insert(failure.replicate_index)
            || !is_sha256_hex(&failure.sample_indices_sha256)
            || !valid_failure_code(&failure.reason_code)
            || failure.message.trim().is_empty()
        {
            bail!("typed CB-SEM bootstrap_v2 failure ledger is inconsistent");
        }
    }
    for witness in &bootstrap.validation_witness.successful_replicates {
        if witness.replicate_index >= bootstrap.requested_replicates
            || !replicate_indices.insert(witness.replicate_index)
            || !is_sha256_hex(&witness.sample_indices_sha256)
            || witness.parameter_estimates.len()
                != bootstrap.validation_witness.parameter_names.len()
            || witness.iterations == 0
            || witness.iterations > result.provenance.settings.max_iterations
            || !witness.objective.is_finite()
            || witness.objective < 0.0
            || witness
                .parameter_estimates
                .iter()
                .any(|estimate| !estimate.is_finite())
        {
            bail!("typed CB-SEM bootstrap_v2 success witness is inconsistent");
        }
    }
    if replicate_indices.len() != bootstrap.requested_replicates as usize
        || replicate_indices
            .iter()
            .copied()
            .ne(0..bootstrap.requested_replicates)
    {
        bail!("typed CB-SEM bootstrap_v2 ledger does not cover every preplanned replicate");
    }

    match &bootstrap.inference {
        qpls_estimation::CbsemBootstrapInferenceV2::Available => {
            if bootstrap.usable_replicates < required
                || bootstrap.intervals.len() != bootstrap.validation_witness.parameter_names.len()
            {
                bail!("available CB-SEM bootstrap_v2 inference has incomplete intervals");
            }
            for (interval, parameter) in bootstrap
                .intervals
                .iter()
                .zip(&bootstrap.validation_witness.parameter_names)
            {
                if interval.parameter != *parameter
                    || interval.usable_replicates != bootstrap.usable_replicates
                    || [
                        interval.original,
                        interval.bootstrap_mean,
                        interval.bias,
                        interval.standard_error,
                        interval.percentile_lower,
                        interval.percentile_upper,
                    ]
                    .iter()
                    .any(|value| !value.is_finite())
                    || interval.standard_error < 0.0
                    || interval.percentile_lower > interval.percentile_upper
                    || !float_matches(interval.bias, interval.bootstrap_mean - interval.original)
                {
                    bail!("typed CB-SEM bootstrap_v2 interval table is inconsistent");
                }
            }
        }
        qpls_estimation::CbsemBootstrapInferenceV2::Unavailable {
            reason_code,
            message,
        } => {
            if bootstrap.usable_replicates >= required
                || !bootstrap.intervals.is_empty()
                || reason_code != "insufficient_usable_replicates"
                || message.trim().is_empty()
            {
                bail!("unavailable CB-SEM bootstrap_v2 inference status is inconsistent");
            }
        }
    }
    Ok(())
}

fn push_experimental_method_payloads(estimation: &serde_json::Value, rows: &mut Vec<ExportRow>) {
    if let Some(cbsem) = estimation.get("cbsem").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "model_type",
            "estimator",
            "input",
            "mean_structure",
            "converged",
            "iterations",
            "objective",
            "gradient_norm",
            "sample_size",
        ] {
            if let Some(value) = cbsem.get(metric) {
                rows.push(row("cbsem", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(fit) = cbsem.get("fit").and_then(|value| value.as_object()) {
            for metric in [
                "method_version",
                "chi_square",
                "degrees_of_freedom",
                "p_value",
                "cfi",
                "tli",
                "rmsea",
                "rmsea_ci_lower",
                "rmsea_ci_upper",
                "srmr",
                "aic",
                "bic",
                "baseline_chi_square",
                "baseline_degrees_of_freedom",
            ] {
                if let Some(value) = fit.get(metric) {
                    rows.push(row("cbsem_fit", "", "", "", "", metric, json_value(value)));
                }
            }
        }
        if let Some(parameters) = cbsem.get("parameters").and_then(|value| value.as_array()) {
            for parameter in parameters {
                for metric in [
                    "kind",
                    "estimate",
                    "standard_error",
                    "z_statistic",
                    "p_value_two_sided",
                    "fixed",
                    "warning",
                ] {
                    if let Some(value) = parameter.get(metric) {
                        rows.push(row(
                            "cbsem_parameter",
                            "",
                            "",
                            &json_str(parameter, "lhs"),
                            &json_str(parameter, "rhs"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(standardized) = cbsem.get("standardized").and_then(|value| value.as_array()) {
            for parameter in standardized {
                for metric in ["kind", "std_lv", "std_all"] {
                    if let Some(value) = parameter.get(metric) {
                        rows.push(row(
                            "cbsem_standardized",
                            "",
                            "",
                            &json_str(parameter, "lhs"),
                            &json_str(parameter, "rhs"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        for matrix_field in [
            "implied_covariance",
            "residual_covariance",
            "residual_correlation",
        ] {
            if let Some(cells) = cbsem.get(matrix_field).and_then(|value| value.as_array()) {
                for cell in cells {
                    rows.push(row(
                        matrix_field,
                        "",
                        "",
                        &json_str(cell, "row"),
                        &json_str(cell, "column"),
                        "value",
                        json_value(cell.get("value").unwrap_or(&serde_json::Value::Null)),
                    ));
                }
            }
        }
        if let Some(items) = cbsem
            .get("modification_indices")
            .and_then(|value| value.as_array())
        {
            for item in items {
                for metric in ["kind", "modification_index", "expected_parameter_change"] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "cbsem_modification_index",
                            "",
                            "",
                            &json_str(item, "lhs"),
                            &json_str(item, "rhs"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(bootstrap) = cbsem.get("bootstrap").and_then(|value| value.as_object()) {
            for metric in ["method_version", "samples", "usable_samples"] {
                if let Some(value) = bootstrap.get(metric) {
                    rows.push(row(
                        "cbsem_bootstrap",
                        "",
                        "",
                        "",
                        "",
                        metric,
                        json_value(value),
                    ));
                }
            }
            if let Some(intervals) = bootstrap
                .get("intervals")
                .and_then(|value| value.as_array())
            {
                for interval in intervals {
                    for metric in ["original", "lower_percentile", "upper_percentile"] {
                        if let Some(value) = interval.get(metric) {
                            rows.push(row(
                                "cbsem_bootstrap_interval",
                                &json_str(interval, "parameter"),
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            push_json_warnings("cbsem_bootstrap", bootstrap.get("warnings"), rows);
        }
        if let Some(multigroup) = cbsem.get("multigroup").and_then(|value| value.as_object()) {
            for metric in ["method_version", "group_column"] {
                if let Some(value) = multigroup.get(metric) {
                    rows.push(row(
                        "cbsem_multigroup",
                        "",
                        "",
                        "",
                        "",
                        metric,
                        json_value(value),
                    ));
                }
            }
            if let Some(groups) = multigroup.get("groups").and_then(|value| value.as_array()) {
                for group in groups {
                    for metric in [
                        "observations",
                        "chi_square",
                        "degrees_of_freedom",
                        "cfi",
                        "rmsea",
                    ] {
                        if let Some(value) = group.get(metric) {
                            rows.push(row(
                                "cbsem_multigroup_group",
                                &json_str(group, "group"),
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            if let Some(steps) = multigroup
                .get("invariance")
                .and_then(|value| value.as_array())
            {
                for step in steps {
                    for metric in [
                        "chi_square",
                        "degrees_of_freedom",
                        "delta_chi_square",
                        "delta_degrees_of_freedom",
                        "delta_cfi",
                        "delta_rmsea",
                        "warning",
                    ] {
                        if let Some(value) = step.get(metric) {
                            rows.push(row(
                                "cbsem_invariance",
                                &json_str(step, "step"),
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            push_json_warnings("cbsem_multigroup", multigroup.get("warnings"), rows);
        }
        push_json_warnings("cbsem", cbsem.get("warnings"), rows);
        if let Some(diagnostics) = cbsem.get("diagnostics").and_then(|value| value.as_array()) {
            for diagnostic in diagnostics {
                rows.push(row(
                    "cbsem_diagnostic",
                    "",
                    "",
                    "",
                    "",
                    "diagnostic",
                    json_value(diagnostic),
                ));
            }
        }
    }
    if let Some(wpls) = estimation.get("wpls").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "case_weight_column",
            "weight_sum",
            "effective_sample_size",
            "covariance",
        ] {
            if let Some(value) = wpls.get(metric) {
                rows.push(row("wpls", "", "", "", "", metric, json_value(value)));
            }
        }
        push_json_warnings("wpls", wpls.get("warnings"), rows);
    }
    if let Some(cca) = estimation.get("cca").and_then(|value| value.as_object()) {
        if let Some(value) = cca.get("max_absolute_residual") {
            rows.push(row(
                "cca_summary",
                "",
                "",
                "",
                "",
                "max_absolute_residual",
                json_value(value),
            ));
        }
        if let Some(correlations) = cca.get("correlations").and_then(|value| value.as_array()) {
            for item in correlations {
                for metric in ["observed", "reproduced", "residual", "absolute_residual"] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "cca_residual",
                            "",
                            "",
                            &json_str(item, "left"),
                            &json_str(item, "right"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("cca", cca.get("warnings"), rows);
    }
    if let Some(cta) = estimation
        .get("cta_pls")
        .and_then(|value| value.as_object())
    {
        if let Some(summary) = cta
            .get("max_absolute_tetrad_by_construct")
            .and_then(|value| value.as_object())
        {
            for (construct, value) in summary {
                rows.push(row(
                    "cta_pls_summary",
                    construct,
                    "",
                    "",
                    "",
                    "max_absolute_tetrad",
                    json_value(value),
                ));
            }
        }
        if let Some(estimates) = cta.get("estimates").and_then(|value| value.as_array()) {
            for item in estimates {
                let indicator = [
                    json_str(item, "indicator_a"),
                    json_str(item, "indicator_b"),
                    json_str(item, "indicator_c"),
                    json_str(item, "indicator_d"),
                ]
                .join("|");
                for metric in ["pairing", "tetrad", "absolute_tetrad"] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "cta_pls_tetrad",
                            &json_str(item, "construct"),
                            &indicator,
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("cta_pls", cta.get("warnings"), rows);
    }
    if let Some(predict) = estimation
        .get("predict")
        .and_then(|value| value.as_object())
    {
        for metric in [
            "method_version",
            "split",
            "training_observations",
            "test_observations",
            "benchmark",
        ] {
            if let Some(value) = predict.get(metric) {
                rows.push(row("plspredict", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(targets) = predict.get("targets").and_then(|value| value.as_array()) {
            for item in targets {
                for metric in [
                    "predictor_count",
                    "rmse_pls",
                    "mae_pls",
                    "rmse_benchmark",
                    "mae_benchmark",
                    "q_squared_predict",
                    "rmse_lm",
                    "mae_lm",
                    "q_squared_predict_lm",
                ] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "plspredict_target",
                            &json_str(item, "construct"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(kfold) = predict
            .get("repeated_kfold")
            .and_then(|value| value.as_object())
        {
            for metric in [
                "method_version",
                "folds",
                "repeats",
                "assignment",
                "total_test_observations",
            ] {
                if let Some(value) = kfold.get(metric) {
                    rows.push(row(
                        "plspredict_kfold",
                        "",
                        "",
                        "",
                        "",
                        metric,
                        json_value(value),
                    ));
                }
            }
            if let Some(targets) = kfold.get("targets").and_then(|value| value.as_array()) {
                for item in targets {
                    for metric in [
                        "predictor_count",
                        "rmse_pls",
                        "mae_pls",
                        "rmse_benchmark",
                        "mae_benchmark",
                        "q_squared_predict",
                        "rmse_lm",
                        "mae_lm",
                        "q_squared_predict_lm",
                    ] {
                        if let Some(value) = item.get(metric) {
                            rows.push(row(
                                "plspredict_kfold_target",
                                &json_str(item, "construct"),
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            if let Some(comparisons) = kfold.get("cvpat").and_then(|value| value.as_array()) {
                for item in comparisons {
                    for metric in [
                        "loss",
                        "mean_loss_difference",
                        "standard_error",
                        "t_statistic",
                        "p_value_two_sided",
                        "observations",
                        "preferred_model",
                        "warning",
                    ] {
                        if let Some(value) = item.get(metric) {
                            rows.push(row(
                                "cvpat",
                                &json_str(item, "target"),
                                "",
                                &json_str(item, "comparison"),
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            push_json_warnings("plspredict_kfold", kfold.get("warnings"), rows);
        }
        push_json_warnings("plspredict", predict.get("warnings"), rows);
    }
    if let Some(segmentation) = estimation
        .get("segmentation")
        .and_then(|value| value.as_object())
    {
        for metric in [
            "method_version",
            "algorithm",
            "requested_segments",
            "selected_segments",
            "assignment",
            "observations",
            "objective",
            "pooled_objective",
            "objective_improvement",
            "min_segment_share",
            "segment_size_imbalance",
            "max_path_separation",
        ] {
            if let Some(value) = segmentation.get(metric) {
                rows.push(row(
                    "segmentation",
                    "",
                    "",
                    "",
                    "",
                    metric,
                    json_value(value),
                ));
            }
        }
        if let Some(segments) = segmentation
            .get("segments")
            .and_then(|value| value.as_array())
        {
            for segment in segments {
                for metric in ["observations", "share", "r_squared"] {
                    if let Some(value) = segment.get(metric) {
                        rows.push(row(
                            "segmentation_segment",
                            &json_str(segment, "segment"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
                if let Some(paths) = segment.get("paths").and_then(|value| value.as_array()) {
                    for path in paths {
                        if let Some(value) = path.get("coefficient") {
                            rows.push(row(
                                "segmentation_path",
                                &json_str(segment, "segment"),
                                "",
                                &json_str(path, "source"),
                                &json_str(path, "target"),
                                "coefficient",
                                json_value(value),
                            ));
                        }
                    }
                }
            }
        }
        if let Some(memberships) = segmentation
            .get("memberships")
            .and_then(|value| value.as_array())
        {
            for membership in memberships {
                rows.push(row(
                    "segmentation_membership",
                    &json_str(membership, "segment"),
                    "",
                    "",
                    "",
                    "observation",
                    json_value(
                        membership
                            .get("observation")
                            .unwrap_or(&serde_json::Value::Null),
                    ),
                ));
            }
        }
        push_json_warnings("segmentation", segmentation.get("warnings"), rows);
    }
    if let Some(mga) = estimation.get("mga").and_then(|value| value.as_object()) {
        for metric in ["method_version", "group_column"] {
            if let Some(value) = mga.get(metric) {
                rows.push(row("mga", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(groups) = mga.get("groups").and_then(|value| value.as_array()) {
            for group in groups {
                for metric in ["observations", "r_squared"] {
                    if let Some(value) = group.get(metric) {
                        rows.push(row(
                            "mga_group",
                            &json_str(group, "group"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
                if let Some(paths) = group.get("paths").and_then(|value| value.as_array()) {
                    for path in paths {
                        if let Some(value) = path.get("coefficient") {
                            rows.push(row(
                                "mga_group_path",
                                &json_str(group, "group"),
                                "",
                                &json_str(path, "source"),
                                &json_str(path, "target"),
                                "coefficient",
                                json_value(value),
                            ));
                        }
                    }
                }
                if let Some(estimates) = group
                    .get("outer_estimates")
                    .and_then(|value| value.as_array())
                {
                    for estimate in estimates {
                        for metric in ["weight", "loading"] {
                            if let Some(value) = estimate.get(metric) {
                                rows.push(row(
                                    "mga_group_measurement",
                                    &json_str(group, "group"),
                                    &json_str(estimate, "indicator"),
                                    &json_str(estimate, "construct"),
                                    "",
                                    metric,
                                    json_value(value),
                                ));
                            }
                        }
                    }
                }
                if let Some(transforms) = group.get("transforms").and_then(|value| value.as_array())
                {
                    for transform in transforms {
                        for metric in ["mean", "scale"] {
                            if let Some(value) = transform.get(metric) {
                                rows.push(row(
                                    "mga_group_transform",
                                    &json_str(group, "group"),
                                    &json_str(transform, "indicator"),
                                    "",
                                    "",
                                    metric,
                                    json_value(value),
                                ));
                            }
                        }
                    }
                }
            }
        }
        if let Some(comparisons) = mga.get("comparisons").and_then(|value| value.as_array()) {
            for comparison in comparisons {
                for metric in [
                    "group_a",
                    "group_b",
                    "coefficient_a",
                    "coefficient_b",
                    "difference",
                    "standard_error",
                    "t_statistic",
                    "p_value_two_sided",
                    "warning",
                ] {
                    if let Some(value) = comparison.get(metric) {
                        rows.push(row(
                            "mga_comparison",
                            "",
                            "",
                            &json_str(comparison, "source"),
                            &json_str(comparison, "target"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(comparisons) = mga
            .get("measurement_comparisons")
            .and_then(|value| value.as_array())
        {
            for comparison in comparisons {
                for metric in [
                    "group_a",
                    "group_b",
                    "estimate_a",
                    "estimate_b",
                    "difference",
                ] {
                    if let Some(value) = comparison.get(metric) {
                        rows.push(row(
                            "mga_measurement_comparison",
                            &json_str(comparison, "construct"),
                            &json_str(comparison, "indicator"),
                            "",
                            "",
                            &format!("{}_{}", json_str(comparison, "parameter"), metric),
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("mga", mga.get("warnings"), rows);
    }
    if let Some(micom) = estimation.get("micom").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "group_column",
            "permutation_samples",
            "usable_permutations",
            "attempted_permutations",
            "failed_permutations",
            "confidence_level",
            "retry_policy",
            "step1_status",
            "step1_computed",
            "step2_usable_permutations",
            "step2_failed_permutations",
            "step3_usable_permutations",
            "step3_failed_permutations",
            "permutation_plan_sha256",
        ] {
            if let Some(value) = micom.get(metric) {
                rows.push(row("micom", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(groups) = micom.get("groups").and_then(|value| value.as_array()) {
            for group in groups {
                if let Some(value) = group.get("observations") {
                    rows.push(row(
                        "micom_group",
                        &json_str(group, "group"),
                        "",
                        "",
                        "",
                        "observations",
                        json_value(value),
                    ));
                }
            }
        }
        if let Some(constructs) = micom.get("constructs").and_then(|value| value.as_array()) {
            for construct in constructs {
                for metric in [
                    "configural_invariance",
                    "compositional_correlation",
                    "compositional_p_value",
                    "compositional_correlation_lower",
                    "mean_a",
                    "mean_b",
                    "mean_difference",
                    "mean_p_value",
                    "mean_difference_lower",
                    "mean_difference_upper",
                    "variance_a",
                    "variance_b",
                    "variance_difference",
                    "variance_p_value",
                    "variance_difference_lower",
                    "variance_difference_upper",
                    "equal_means",
                    "equal_variances",
                    "partial_invariance",
                    "full_invariance",
                ] {
                    if let Some(value) = construct.get(metric) {
                        rows.push(row(
                            "micom_construct",
                            &json_str(construct, "construct"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(ledger) = micom
            .get("permutation_ledger")
            .and_then(|value| value.as_array())
        {
            for entry in ledger {
                let replicate = entry.get("replicate").map(json_value).unwrap_or_default();
                for metric in [
                    "partition_sha256",
                    "group_a_rows",
                    "group_b_rows",
                    "step2_status",
                    "step2_failure_code",
                    "step3_status",
                    "step3_failure_code",
                ] {
                    if let Some(value) = entry.get(metric) {
                        rows.push(row(
                            "micom_permutation_ledger",
                            &replicate,
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("micom", micom.get("warnings"), rows);
    }
    if let Some(mga_permutation) = estimation
        .get("mga_permutation")
        .and_then(|value| value.as_object())
    {
        for metric in [
            "method_version",
            "group_column",
            "permutation_samples",
            "usable_permutations",
            "attempted_permutations",
            "failed_permutations",
        ] {
            if let Some(value) = mga_permutation.get(metric) {
                rows.push(row(
                    "mga_permutation",
                    "",
                    "",
                    "",
                    "",
                    metric,
                    json_value(value),
                ));
            }
        }
        if let Some(comparisons) = mga_permutation
            .get("measurement_comparisons")
            .and_then(|value| value.as_array())
        {
            for comparison in comparisons {
                for metric in [
                    "original_difference",
                    "empirical_p_value_two_sided",
                    "percentile_rank",
                ] {
                    if let Some(value) = comparison.get(metric) {
                        rows.push(row(
                            "mga_permutation_measurement_comparison",
                            &json_str(comparison, "construct"),
                            &json_str(comparison, "indicator"),
                            "",
                            "",
                            &format!("{}_{}", json_str(comparison, "parameter"), metric),
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(comparisons) = mga_permutation
            .get("comparisons")
            .and_then(|value| value.as_array())
        {
            for comparison in comparisons {
                for metric in [
                    "original_difference",
                    "empirical_p_value_two_sided",
                    "percentile_rank",
                ] {
                    if let Some(value) = comparison.get(metric) {
                        rows.push(row(
                            "mga_permutation_comparison",
                            "",
                            "",
                            &json_str(comparison, "source"),
                            &json_str(comparison, "target"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("mga_permutation", mga_permutation.get("warnings"), rows);
    }
    if let Some(fimix) = estimation.get("fimix").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "classes",
            "starts",
            "iterations",
            "log_likelihood",
            "aic",
            "bic",
            "caic",
            "entropy",
        ] {
            if let Some(value) = fimix.get(metric) {
                rows.push(row("fimix", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(classes) = fimix
            .get("classes_summary")
            .and_then(|value| value.as_array())
        {
            for class in classes {
                for metric in ["observations", "share", "r_squared"] {
                    if let Some(value) = class.get(metric) {
                        rows.push(row(
                            "fimix_class",
                            &json_str(class, "class"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
                if let Some(paths) = class.get("paths").and_then(|value| value.as_array()) {
                    for path in paths {
                        if let Some(value) = path.get("coefficient") {
                            rows.push(row(
                                "fimix_path",
                                &json_str(class, "class"),
                                "",
                                &json_str(path, "source"),
                                &json_str(path, "target"),
                                "coefficient",
                                json_value(value),
                            ));
                        }
                    }
                }
            }
        }
        push_json_warnings("fimix", fimix.get("warnings"), rows);
    }
    if let Some(ipma) = estimation.get("ipma").and_then(|value| value.as_object()) {
        for metric in ["method_version", "performance_scale", "targets"] {
            if let Some(value) = ipma.get(metric) {
                rows.push(row("ipma", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(constructs) = ipma.get("constructs").and_then(|value| value.as_array()) {
            for item in constructs {
                for metric in ["importance", "performance", "score_mean"] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "ipma_construct",
                            &json_str(item, "construct"),
                            "",
                            "",
                            &json_str(item, "target"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(indicators) = ipma.get("indicators").and_then(|value| value.as_array()) {
            for item in indicators {
                for metric in [
                    "construct_importance",
                    "loading",
                    "performance",
                    "score_mean",
                ] {
                    if let Some(value) = item.get(metric) {
                        rows.push(row(
                            "ipma_indicator",
                            &json_str(item, "construct"),
                            &json_str(item, "indicator"),
                            "",
                            &json_str(item, "target"),
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("ipma", ipma.get("warnings"), rows);
    }
    if let Some(pca) = estimation.get("pca").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "component_rule",
            "retained_components",
            "observations",
            "variables",
        ] {
            if let Some(value) = pca.get(metric) {
                rows.push(row("pca", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(components) = pca.get("components").and_then(|value| value.as_array()) {
            for component in components {
                for metric in ["eigenvalue", "explained_variance", "cumulative_variance"] {
                    if let Some(value) = component.get(metric) {
                        rows.push(row(
                            "pca_component",
                            &json_str(component, "component"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(loadings) = pca.get("loadings").and_then(|value| value.as_array()) {
            for loading in loadings {
                for metric in ["loading", "weight"] {
                    if let Some(value) = loading.get(metric) {
                        rows.push(row(
                            "pca_loading",
                            &json_str(loading, "component"),
                            &json_str(loading, "variable"),
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("pca", pca.get("warnings"), rows);
    }
    if let Some(regression) = estimation
        .get("regression")
        .and_then(|value| value.as_object())
    {
        for metric in [
            "method_version",
            "regression_type",
            "outcome",
            "predictors",
            "controls",
            "observations",
        ] {
            if let Some(value) = regression.get(metric) {
                rows.push(row("regression", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(fit) = regression.get("fit").and_then(|value| value.as_object()) {
            for metric in [
                "r_squared",
                "adjusted_r_squared",
                "f_statistic",
                "log_likelihood",
                "pseudo_r_squared",
                "aic",
                "bic",
                "rmse",
            ] {
                if let Some(value) = fit.get(metric) {
                    rows.push(row(
                        "regression_fit",
                        "",
                        "",
                        "",
                        "",
                        metric,
                        json_value(value),
                    ));
                }
            }
        }
        if let Some(coefficients) = regression
            .get("coefficients")
            .and_then(|value| value.as_array())
        {
            for coefficient in coefficients {
                for metric in [
                    "estimate",
                    "standard_error",
                    "statistic",
                    "p_value_two_sided",
                    "confidence_interval_lower",
                    "confidence_interval_upper",
                    "odds_ratio",
                ] {
                    if let Some(value) = coefficient.get(metric) {
                        rows.push(row(
                            "regression_coefficient",
                            &json_str(coefficient, "term"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(process) = regression
            .get("process")
            .and_then(|value| value.as_object())
        {
            if let Some(effects) = process.get("effects").and_then(|value| value.as_array()) {
                for effect in effects {
                    for metric in ["estimate", "lower_percentile", "upper_percentile"] {
                        if let Some(value) = effect.get(metric) {
                            rows.push(row(
                                "process_effect",
                                &json_str(effect, "effect"),
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            if let Some(slopes) = process
                .get("simple_slopes")
                .and_then(|value| value.as_array())
            {
                for slope in slopes {
                    for metric in ["moderator_value", "slope"] {
                        if let Some(value) = slope.get(metric) {
                            rows.push(row(
                                "process_simple_slope",
                                "",
                                "",
                                "",
                                "",
                                metric,
                                json_value(value),
                            ));
                        }
                    }
                }
            }
            push_json_warnings("process", process.get("warnings"), rows);
        }
        push_json_warnings("regression", regression.get("warnings"), rows);
    }
    if let Some(nca) = estimation.get("nca").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "ceiling",
            "permutation_samples",
            "usable_permutations",
            "x",
            "y",
            "observations",
        ] {
            if let Some(value) = nca.get(metric) {
                rows.push(row("nca", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(ceilings) = nca.get("ceilings").and_then(|value| value.as_array()) {
            for ceiling in ceilings {
                for metric in ["effect_size", "permutation_p_value"] {
                    if let Some(value) = ceiling.get(metric) {
                        rows.push(row(
                            "nca_ceiling",
                            &json_str(ceiling, "ceiling"),
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        if let Some(bottlenecks) = nca.get("bottlenecks").and_then(|value| value.as_array()) {
            for bottleneck in bottlenecks {
                for metric in ["outcome_percent", "required_x_percent"] {
                    if let Some(value) = bottleneck.get(metric) {
                        rows.push(row(
                            "nca_bottleneck",
                            "",
                            "",
                            "",
                            "",
                            metric,
                            json_value(value),
                        ));
                    }
                }
            }
        }
        push_json_warnings("nca", nca.get("warnings"), rows);
    }
    if let Some(gsca) = estimation.get("gsca").and_then(|value| value.as_object()) {
        for metric in ["method_version", "iterations", "fit", "adjusted_fit", "gfi"] {
            if let Some(value) = gsca.get(metric) {
                rows.push(row("gsca", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(paths) = gsca.get("paths").and_then(|value| value.as_array()) {
            for path in paths {
                if let Some(value) = path.get("coefficient") {
                    rows.push(row(
                        "gsca_path",
                        "",
                        "",
                        &json_str(path, "source"),
                        &json_str(path, "target"),
                        "coefficient",
                        json_value(value),
                    ));
                }
            }
        }
        push_json_warnings("gsca", gsca.get("warnings"), rows);
    }
    push_optional_estimate_table(
        estimation,
        "endogeneity",
        "endogeneity",
        &[
            "path_coefficient",
            "copula_coefficient",
            "standard_error",
            "t_statistic",
            "p_value_two_sided",
            "predictor_skewness",
            "applicable",
            "warning",
        ],
        rows,
    );
    push_optional_estimate_table(
        estimation,
        "nonlinear_effects",
        "nonlinear_effect",
        &[
            "linear_coefficient",
            "quadratic_coefficient",
            "standard_error",
            "t_statistic",
            "p_value_two_sided",
            "linear_r_squared",
            "augmented_r_squared",
            "delta_r_squared",
            "warning",
        ],
        rows,
    );
    push_optional_estimate_table(
        estimation,
        "moderated_mediation",
        "moderated_mediation",
        &[
            "moderated_stage",
            "index_of_moderated_mediation",
            "conditional_indirect_effects",
            "warning",
        ],
        rows,
    );
}

fn push_optional_estimate_table(
    estimation: &serde_json::Value,
    field: &str,
    section: &str,
    metrics: &[&str],
    rows: &mut Vec<ExportRow>,
) {
    let Some(payload) = estimation.get(field).and_then(|value| value.as_object()) else {
        return;
    };
    if let Some(estimates) = payload.get("estimates").and_then(|value| value.as_array()) {
        for item in estimates {
            for metric in metrics {
                if let Some(value) = item.get(*metric) {
                    rows.push(row(
                        section,
                        "",
                        "",
                        &json_str(item, "source").if_empty_then(|| json_str(item, "predictor")),
                        &json_str(item, "target"),
                        metric,
                        json_value(value),
                    ));
                }
            }
        }
    }
    push_json_warnings(section, payload.get("warnings"), rows);
}

trait EmptyStringFallback {
    fn if_empty_then(self, fallback: impl FnOnce() -> String) -> String;
}

impl EmptyStringFallback for String {
    fn if_empty_then(self, fallback: impl FnOnce() -> String) -> String {
        if self.is_empty() { fallback() } else { self }
    }
}

fn push_json_warnings(section: &str, value: Option<&serde_json::Value>, rows: &mut Vec<ExportRow>) {
    if let Some(warnings) = value.and_then(|value| value.as_array()) {
        for warning in warnings {
            rows.push(row(section, "", "", "", "", "warning", json_value(warning)));
        }
    }
}

fn push_metadata_rows(result: &AnalysisResult, rows: &mut Vec<ExportRow>) {
    let values = [
        ("result_id", result.id.to_string()),
        ("recipe_id", result.provenance.recipe_id.to_string()),
        (
            "dataset_fingerprint",
            result.provenance.dataset_fingerprint.clone(),
        ),
        ("method", format!("{:?}", result.provenance.method)),
        ("method_version", result.provenance.method_version.clone()),
        ("engine_version", result.provenance.engine_version.clone()),
        ("seed", result.provenance.seed.to_string()),
        ("started_at", result.provenance.started_at.to_rfc3339()),
        ("completed_at", result.provenance.completed_at.to_rfc3339()),
    ];
    for (metric, value) in values {
        rows.push(row("metadata", "", "", "", "", metric, value));
    }
}

fn push_scalar_estimate(
    estimation: &serde_json::Value,
    section: &str,
    field: &str,
    metric: &str,
    rows: &mut Vec<ExportRow>,
) {
    if let Some(value) = estimation.get(field) {
        rows.push(row(section, "", "", "", "", metric, json_value(value)));
    }
}

fn push_outer_estimates(estimation: &serde_json::Value, rows: &mut Vec<ExportRow>) {
    let Some(outer_estimates) = estimation
        .get("outer_estimates")
        .and_then(|value| value.as_array())
    else {
        return;
    };
    for outer in outer_estimates {
        let construct = json_str(outer, "construct");
        let indicator = json_str(outer, "indicator");
        for metric in ["weight", "loading"] {
            if let Some(value) = outer.get(metric) {
                rows.push(row(
                    "outer_estimate",
                    &construct,
                    &indicator,
                    "",
                    "",
                    metric,
                    json_value(value),
                ));
            }
        }
    }
}

fn push_path_coefficients(estimation: &serde_json::Value, rows: &mut Vec<ExportRow>) {
    let Some(paths) = estimation.get("paths").and_then(|value| value.as_array()) else {
        return;
    };
    for path in paths {
        rows.push(row(
            "path_coefficient",
            "",
            "",
            &json_str(path, "source"),
            &json_str(path, "target"),
            "path_coefficient",
            json_value(path.get("coefficient").unwrap_or(&serde_json::Value::Null)),
        ));
    }
}

fn push_posthoc_minimum_sample_size(
    result: &AnalysisResult,
    estimation: &serde_json::Value,
    rows: &mut Vec<ExportRow>,
) -> Result<()> {
    if estimation
        .get("posthoc_minimum_sample_size")
        .is_none_or(serde_json::Value::is_null)
    {
        return Ok(());
    }
    let typed: qpls_estimation::PlsResult = serde_json::from_value(estimation.clone())
        .context("invalid PLS result containing a post-hoc minimum sample-size result")?;
    let Some(stored) = typed.posthoc_minimum_sample_size.as_ref() else {
        bail!("post-hoc minimum sample-size payload is missing after typed conversion");
    };
    let expected = match stored.method_version.as_str() {
        qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION_V1 => {
            qpls_estimation::pls_posthoc_minimum_sample_size(&typed.paths, typed.used_observations)
        }
        qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION => {
            let significance = pls_posthoc_bootstrap_significance(result, &typed.paths)?;
            Some(qpls_estimation::pls_posthoc_minimum_sample_size_v2(
                &typed.paths,
                typed.used_observations,
                significance.as_deref(),
            ))
        }
        other => bail!("unsupported post-hoc minimum sample-size method version {other}"),
    };
    if expected.as_ref() != Some(stored) {
        bail!(
            "post-hoc minimum sample-size payload does not reproduce from its path coefficients and linked inference"
        );
    }
    let value = serde_json::to_value(stored)?;
    let mut metrics = vec![
        "method_version",
        "alpha",
        "power",
        "test",
        "inverse_square_root_constant",
        "minimum_absolute_path_coefficient",
        "technically_required_sample_size",
        "analytical_sample_size",
        "meets_technical_requirement",
        "status",
        "caution",
    ];
    if stored.method_version == qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION {
        metrics.splice(
            5..5,
            [
                "selection_rule",
                "significance_source",
                "significance_alpha",
                "eligible_path_count",
                "significant_path_count",
                "driver_p_value_two_sided",
            ],
        );
    }
    for metric in metrics {
        if let Some(metric_value) = value.get(metric) {
            rows.push(row(
                "posthoc_minimum_sample_size",
                "",
                "",
                stored.driver_source.as_deref().unwrap_or(""),
                stored.driver_target.as_deref().unwrap_or(""),
                metric,
                json_value(metric_value),
            ));
        }
    }
    if stored.method_version == qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION {
        let registry = CapabilityRegistryV2::embedded().context(
            "the embedded Capability Registry V2 is invalid; post-hoc availability cannot be exported",
        )?;
        let cell = registry
            .option_cells()
            .find(|cell| {
                cell.capability_id == "smartpls.pls_power_analysis"
                    && cell.cell_id == "qpls3.pls.posthoc_technical_minimum_sample_size"
                    && cell.capability_version == "pls_posthoc_technical_minimum_sample_size_v2"
            })
            .context("the exact post-hoc technical minimum sample-size v2 cell is absent")?;
        rows.push(row(
            "posthoc_minimum_sample_size",
            "",
            "",
            stored.driver_source.as_deref().unwrap_or(""),
            stored.driver_target.as_deref().unwrap_or(""),
            "availability",
            customer_availability(cell).to_string(),
        ));
    }
    Ok(())
}

fn pls_posthoc_bootstrap_significance(
    result: &AnalysisResult,
    paths: &[qpls_estimation::PathEstimate],
) -> Result<Option<Vec<qpls_estimation::PlsPathSignificance>>> {
    let value = match &result.payload {
        AnalysisPayload::PlsPmV2 { bootstrap, .. } => Some(bootstrap),
        AnalysisPayload::PlsPmV3 { bootstrap, .. } => bootstrap.as_ref(),
        AnalysisPayload::PlsPmV1 { .. } => None,
        AnalysisPayload::PlsSampleSizePowerV1 { .. }
        | AnalysisPayload::PlsSampleSizePowerV2 { .. }
        | AnalysisPayload::Legacy { .. } => None,
    };
    let Some(value) = value else {
        return Ok(None);
    };
    let bootstrap: qpls_resampling::PlsBootstrapResult = serde_json::from_value(value.clone())
        .context("invalid linked PLS bootstrap payload for post-hoc sample-size export")?;
    let expected_paths = paths
        .iter()
        .map(|path| ((path.source.clone(), path.target.clone()), path.coefficient))
        .collect::<std::collections::BTreeMap<_, _>>();
    if expected_paths.len() != paths.len() {
        bail!("duplicate PLS path identities in post-hoc sample-size export source");
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut significance = Vec::with_capacity(paths.len());
    for parameter in &bootstrap.percentile.parameters {
        let (kind, parts) = serde_json::from_str::<(String, Vec<String>)>(&parameter.parameter)
            .context("malformed bootstrap parameter identity in post-hoc sample-size export")?;
        if kind != "path" {
            continue;
        }
        if parts.len() != 2 {
            bail!("malformed linked PLS path identity in post-hoc sample-size export");
        }
        let identity = (parts[0].clone(), parts[1].clone());
        let Some(expected_original) = expected_paths.get(&identity) else {
            bail!("foreign linked PLS path identity in post-hoc sample-size export");
        };
        if !seen.insert(identity) || parameter.original.to_bits() != expected_original.to_bits() {
            bail!(
                "duplicate or coefficient-mismatched linked PLS path inference in post-hoc sample-size export"
            );
        }
        significance.push(qpls_estimation::PlsPathSignificance {
            source: parts[0].clone(),
            target: parts[1].clone(),
            p_value_two_sided: parameter.p_value_two_sided,
        });
    }
    if seen.len() != expected_paths.len() {
        bail!("missing linked PLS path inference in post-hoc sample-size export");
    }
    Ok(Some(significance))
}

fn push_effects(estimation: &serde_json::Value, rows: &mut Vec<ExportRow>) {
    let Some(effects) = estimation.get("effects").and_then(|value| value.as_array()) else {
        return;
    };
    for effect in effects {
        for metric in ["direct", "indirect", "total"] {
            if let Some(value) = effect.get(metric) {
                rows.push(row(
                    "effect",
                    "",
                    "",
                    &json_str(effect, "source"),
                    &json_str(effect, "target"),
                    metric,
                    json_value(value),
                ));
            }
        }
    }
}

fn push_r_squared(estimation: &serde_json::Value, rows: &mut Vec<ExportRow>) {
    let Some(values) = estimation
        .get("r_squared")
        .and_then(|value| value.as_object())
    else {
        return;
    };
    for (construct, value) in values {
        rows.push(row(
            "r_squared",
            construct,
            "",
            "",
            "",
            "r_squared",
            json_value(value),
        ));
    }
}

fn push_result_diagnostics(result: &AnalysisResult, rows: &mut Vec<ExportRow>) {
    for diagnostic in &result.diagnostics {
        rows.push(row(
            "diagnostic",
            "",
            "",
            "",
            "",
            &format!("{:?}.{}", diagnostic.level, diagnostic.code),
            diagnostic.message.clone(),
        ));
    }
}

fn row(
    section: &str,
    construct: &str,
    indicator: &str,
    source: &str,
    target: &str,
    metric: &str,
    value: String,
) -> ExportRow {
    ExportRow {
        section: section.into(),
        construct: construct.into(),
        indicator: indicator.into(),
        source: source.into(),
        target: target.into(),
        metric: metric.into(),
        value,
    }
}

fn json_str(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .into()
}

fn json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn render_estimator_csv(rows: &[ExportRow]) -> String {
    let mut output = String::from("section,construct,indicator,source,target,metric,value\n");
    for row in rows {
        output.push_str(
            &[
                row.section.as_str(),
                row.construct.as_str(),
                row.indicator.as_str(),
                row.source.as_str(),
                row.target.as_str(),
                row.metric.as_str(),
                row.value.as_str(),
            ]
            .into_iter()
            .map(csv_field)
            .collect::<Vec<_>>()
            .join(","),
        );
        output.push('\n');
    }
    output
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

fn render_estimator_html(result: &AnalysisResult, rows: &[ExportRow]) -> String {
    let (title, notice) = if rows.iter().any(|row| row.section == "pls_power_provenance") {
        (
            "QuickPLS PLS sample-size/power export",
            "Typed prospective sample-size/power tables and the complete ordered replicate ledger are included. Standalone export checks stored accounting but cannot recompute recipe-bound digests without the scientific recipe.",
        )
    } else if rows.iter().any(|row| row.section == "scope_warning") {
        (
            "QuickPLS supplemental method export",
            "Supplemental method tables are included with their stored scope warnings, inference status, and provenance.",
        )
    } else {
        (
            "QuickPLS v0.3 estimator export",
            "Estimator-only export: validated v0.3 PLS core values are included. Assessment and resampling artifacts are excluded until their publication export gates pass.",
        )
    };
    let table_rows = rows
        .iter()
        .map(|row| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&row.section),
                html_escape(&row.construct),
                html_escape(&row.indicator),
                html_escape(&row.source),
                html_escape(&row.target),
                html_escape(&row.metric),
                html_escape(&row.value)
            )
        })
        .collect::<String>();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font-family:Arial,sans-serif;margin:32px;color:#111827}}.notice{{border:1px solid #f59e0b;background:#fffbeb;padding:12px;margin:16px 0}}table{{border-collapse:collapse;width:100%;font-size:13px}}th,td{{border:1px solid #d1d5db;padding:6px;text-align:left}}th{{background:#f3f4f6}}</style></head><body><h1>{}</h1><p>Result {}</p><div class=\"notice\">{}</div><table><thead><tr><th>section</th><th>construct</th><th>indicator</th><th>source</th><th>target</th><th>metric</th><th>value</th></tr></thead><tbody>{}</tbody></table></body></html>",
        html_escape(title),
        html_escape(title),
        html_escape(&result.id.to_string()),
        html_escape(notice),
        table_rows
    )
}

fn write_estimator_xlsx(path: &Path, rows: &[ExportRow]) -> Result<()> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("QuickPLS export")?;
    let headers = [
        "section",
        "construct",
        "indicator",
        "source",
        "target",
        "metric",
        "value",
    ];
    for (column, header) in headers.iter().enumerate() {
        worksheet.write_string(0, column as u16, *header)?;
    }
    for (row_index, row) in rows.iter().enumerate() {
        let values = [
            row.section.as_str(),
            row.construct.as_str(),
            row.indicator.as_str(),
            row.source.as_str(),
            row.target.as_str(),
            row.metric.as_str(),
            row.value.as_str(),
        ];
        for (column, value) in values.iter().enumerate() {
            worksheet.write_string((row_index + 1) as u32, column as u16, *value)?;
        }
    }
    worksheet.autofit();
    workbook.save(path)?;
    Ok(())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn evidence(target: EvidenceTarget, output: Option<&Path>) -> Result<()> {
    match target {
        EvidenceTarget::V03Pls => write_v03_pls_evidence(output),
        EvidenceTarget::V04Assessment => write_v04_assessment_evidence(output),
        EvidenceTarget::V05ExtendedPls => write_v05_extended_pls_evidence(output),
        EvidenceTarget::V07Cbsem => write_v07_cbsem_evidence(output),
        EvidenceTarget::V08ExtendedMethods => write_v08_extended_methods_evidence(output),
        EvidenceTarget::PublicationReady => write_publication_ready_evidence(output),
    }
}

fn write_v03_pls_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_3_pls_core")
        .context("development registry is missing v0_3_pls_core")?;
    let artifacts = [
        "docs/methods/PLS_PM_V1.md",
        "validation/fixtures/simple_reflective.csv",
        "validation/fixtures/simple_reflective.recipe.json",
        "validation/fixtures/simple_reflective.mode_b.recipe.json",
        "validation/fixtures/simple_reflective.factor.recipe.json",
        "validation/fixtures/simple_reflective.pca.recipe.json",
        "validation/fixtures/csem_threecommonfactors.csv",
        "validation/fixtures/csem_threecommonfactors.recipe.json",
        "validation/results/pls_csem_0_6_1.csv",
        "validation/results/pls_quickpls_path_mode_a.json",
        "validation/results/pls_quickpls_mode_b.json",
        "validation/results/pls_quickpls_factor.json",
        "validation/results/pls_quickpls_pca.json",
        "validation/results/pls_csem_comparison.json",
        "validation/results/pls_plspm_0_5_7.json",
        "validation/results/pls_plspm_comparison.json",
        "validation/results/pls_pca_numpy_reference.json",
        "validation/results/pls_pca_numpy_comparison.json",
        "validation/results/pls_csem_threecommonfactors_0_6_1.csv",
        "validation/results/pls_quickpls_csem_threecommonfactors.json",
        "validation/results/pls_csem_threecommonfactors_comparison.json",
    ];
    let artifact_status = artifacts
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            json!({
                "path": relative,
                "present": path.exists(),
                "bytes": fs::metadata(&path).ok().map(|metadata| metadata.len())
            })
        })
        .collect::<Vec<_>>();
    let comparison_path = root.join("validation/results/pls_csem_comparison.json");
    let comparison: serde_json::Value = serde_json::from_slice(
        &fs::read(&comparison_path)
            .with_context(|| format!("cannot read {}", comparison_path.display()))?,
    )
    .context("invalid PLS cSEM comparison JSON")?;
    let plspm_comparison_path = root.join("validation/results/pls_plspm_comparison.json");
    let plspm_comparison: serde_json::Value = serde_json::from_slice(
        &fs::read(&plspm_comparison_path)
            .with_context(|| format!("cannot read {}", plspm_comparison_path.display()))?,
    )
    .context("invalid PLS plspm comparison JSON")?;
    let csem_variants = comparison["variants"]
        .as_array()
        .context("PLS cSEM comparison is missing variants")?
        .iter()
        .map(|variant| {
            json!({
                "variant": variant["variant"],
                "status": variant["status"],
                "max_abs_diff": variant["max_abs_diff"],
                "reference": "cSEM 0.6.1"
            })
        })
        .collect::<Vec<_>>();
    let plspm_variants = plspm_comparison["variants"]
        .as_array()
        .context("PLS plspm comparison is missing variants")?
        .iter()
        .map(|variant| {
            json!({
                "variant": variant["variant"],
                "status": variant["status"],
                "max_abs_diff": variant["max_abs_diff"],
                "reference": "python-plspm 0.5.7",
                "compared_quantities": plspm_comparison["compared_quantities"]
            })
        })
        .collect::<Vec<_>>();
    let pca_comparison_path = root.join("validation/results/pls_pca_numpy_comparison.json");
    let pca_comparison: serde_json::Value = serde_json::from_slice(
        &fs::read(&pca_comparison_path)
            .with_context(|| format!("cannot read {}", pca_comparison_path.display()))?,
    )
    .context("invalid PLS PCA NumPy comparison JSON")?;
    let published_comparison_path =
        root.join("validation/results/pls_csem_threecommonfactors_comparison.json");
    let published_comparison: serde_json::Value = serde_json::from_slice(
        &fs::read(&published_comparison_path)
            .with_context(|| format!("cannot read {}", published_comparison_path.display()))?,
    )
    .context("invalid published PLS cSEM comparison JSON")?;
    let all_artifacts_present = artifact_status
        .iter()
        .all(|artifact| artifact["present"].as_bool() == Some(true));
    let comparison_status = if comparison["status"] == "passed"
        && plspm_comparison["status"] == "passed"
        && pca_comparison["status"] == "passed"
        && published_comparison["status"] == "passed"
    {
        "passed"
    } else {
        "failed"
    };
    let open_registry_gates = slice
        .gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Open)
        .map(|gate| {
            json!({
                "track": gate.track,
                "name": gate.name,
                "status": gate.status,
                "evidence": gate.evidence
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": 1,
        "target": "v03-pls",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "stable_release_allowed": slice.status == SliceStatus::Validated && slice.stable_output,
        "generated_at": Utc::now(),
        "all_listed_artifacts_present": all_artifacts_present,
        "artifacts": artifact_status,
        "comparison_status": comparison_status,
        "deterministic_tolerance": "1e-6",
        "references": {
            "csem_0_6_1": {
                "status": comparison["status"],
                "variants": csem_variants
            },
            "python_plspm_0_5_7": {
                "status": plspm_comparison["status"],
                "variants": plspm_variants,
                "excluded_quantities": plspm_comparison["excluded_quantities"]
            },
            "numpy_pca_eigh": {
                "status": pca_comparison["status"],
                "variant": "PCA",
                "max_abs_diff": pca_comparison["max_abs_diff"],
                "compared_quantities": pca_comparison["compared_quantities"]
            },
            "published_csem_threecommonfactors": {
                "status": published_comparison["status"],
                "dataset": "cSEM::threecommonfactors",
                "reference": "cSEM 0.6.1",
                "max_abs_diff": published_comparison["max_abs_diff"],
                "compared_quantities": published_comparison["compared_quantities"],
                "population_path_values": published_comparison["source"]["population_path_values"]
            }
        },
        "boundary_parity": {
            "status": "passed",
            "evidence": [
                "qpls-runner::deterministic_payload_is_stable_across_runner_invocations",
                "quickpls-desktop::desktop_runner_payload_matches_cli_serialized_artifact",
                "qpls-cli::cli_analysis_payload_is_exactly_worker_invariant"
            ],
            "numeric_tolerance": "1e-12",
            "ignored_fields": ["result id", "started_at", "completed_at"]
        },
        "open_blockers": open_registry_gates
            .iter()
            .map(|gate| gate["name"].clone())
            .collect::<Vec<_>>(),
        "open_registry_gates": open_registry_gates.clone(),
        "note": if slice.status == SliceStatus::Validated {
            "Evidence traceability report. PLS-PM v0.3 core is validated for the current documented scope; later PLS extensions remain separately gated."
        } else {
            "Evidence traceability report only. PLS-PM remains experimental until all open blockers are resolved."
        }
    });
    let target = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("validation/results/v03_pls_evidence.json"));
    fs::write(&target, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", target.display()))?;
    println!(
        "wrote {} | comparison_status={}",
        target.display(),
        comparison["status"].as_str().unwrap_or("unknown")
    );
    Ok(())
}

fn write_v05_extended_pls_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_5_extended_pls")
        .context("development registry is missing v0_5_extended_pls")?;
    let reports = [
        (
            "mediation_reference_report.json",
            "mediation independent equation fixture",
        ),
        (
            "mediation_r_reference_report.json",
            "mediation R base-lm second source",
        ),
        (
            "mediation_published_example_report.json",
            "mediation published/example decomposition",
        ),
        (
            "mediation_metamorphic_report.json",
            "mediation metamorphic checks",
        ),
        (
            "mediation_randomization_report.json",
            "mediation randomization screen",
        ),
        (
            "moderation_reference_report.json",
            "moderation independent reference",
        ),
        (
            "moderation_r_reference_report.json",
            "moderation R base-lm second source",
        ),
        (
            "moderation_published_formula_report.json",
            "moderation published-formula fixture",
        ),
        (
            "moderation_published_empirical_report.json",
            "moderation empirical-data fixture",
        ),
        (
            "moderation_simulation_report.json",
            "moderation bounded simulation",
        ),
        (
            "moderation_inference_report.json",
            "moderation inference integration",
        ),
        (
            "moderation_inference_qualification_report.json",
            "moderation inference qualification",
        ),
        (
            "moderation_coverage_qualification_report.json",
            "moderation release-oriented coverage",
        ),
        (
            "higher_order_reference_report.json",
            "repeated-indicator HOC reference",
        ),
        (
            "higher_order_metamorphic_report.json",
            "repeated-indicator HOC metamorphic checks",
        ),
        (
            "higher_order_two_stage_reference_report.json",
            "two-stage HOC reference",
        ),
        (
            "higher_order_hybrid_reference_report.json",
            "hybrid HOC reference",
        ),
        (
            "higher_order_hybrid_guard_report.json",
            "hybrid HOC invalid-split guard",
        ),
        ("plsc_reference_report.json", "PLSc independent reference"),
        (
            "plsc_unsupported_guard_report.json",
            "PLSc unsupported guard",
        ),
        (
            "endogeneity_reference_report.json",
            "Gaussian-copula endogeneity reference",
        ),
        (
            "nonlinear_effects_reference_report.json",
            "nonlinear effects reference",
        ),
        (
            "moderated_mediation_reference_report.json",
            "moderated mediation reference",
        ),
        ("cta_pls_reference_report.json", "CTA-PLS reference"),
        ("wpls_reference_report.json", "WPLS reference"),
        ("cca_reference_report.json", "CCA reference"),
        (
            "extended_pls_unsupported_guard_report.json",
            "extended PLS unsupported guard",
        ),
    ];
    let mut all_present = true;
    let mut all_passed = true;
    let artifacts = reports
        .iter()
        .map(|(file, description)| {
            let path = root.join("validation/results").join(file);
            let present = path.exists();
            all_present &= present;
            let report = if present {
                serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?)
                    .with_context(|| format!("invalid JSON {}", path.display()))?
            } else {
                serde_json::Value::Null
            };
            let passed = evidence_report_passed(&report);
            all_passed &= passed;
            Ok(json!({
                "file": format!("validation/results/{file}"),
                "description": description,
                "present": present,
                "passed": passed,
                "kind": report.get("kind").cloned().unwrap_or(serde_json::Value::Null),
                "status": report.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "max_delta": report.get("max_delta").cloned().unwrap_or(serde_json::Value::Null),
                "note": report.get("note").cloned().unwrap_or(serde_json::Value::Null)
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let open_registry_gates = slice
        .gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Open)
        .map(|gate| {
            json!({
                "track": gate.track,
                "name": gate.name,
                "status": gate.status,
                "evidence": gate.evidence
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": 1,
        "target": "v05-extended-pls",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "generated_at": Utc::now(),
        "all_listed_artifacts_present": all_present,
        "all_listed_artifacts_passed": all_passed,
        "artifact_count": artifacts.len(),
        "artifacts": artifacts,
        "open_registry_gates": open_registry_gates.clone(),
        "promotion_ready": all_present && all_passed && open_registry_gates.is_empty(),
        "note": "Traceability report for v0.5 extended PLS evidence. Passing artifacts support experimental preview promotion only; validated/publication-ready status still requires the registry gate to be clear and method-specific limitations to remain documented."
    });
    let target = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("validation/results/v05_extended_pls_evidence.json"));
    fs::write(&target, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", target.display()))?;
    println!(
        "wrote {} | artifacts_present={} artifacts_passed={} open_gates={}",
        target.display(),
        all_present,
        all_passed,
        open_registry_gates.len()
    );
    Ok(())
}

fn write_v07_cbsem_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_7_cbsem")
        .context("development registry is missing v0_7_cbsem")?;
    let artifacts = [
        "docs/methods/CBSEM_ML_V1.md",
        "docs/methods/CFA_ML_V1.md",
        "docs/methods/CBSEM_FIT_V1.md",
        "docs/methods/CBSEM_MODIFICATION_INDICES_V1.md",
        "docs/methods/CBSEM_MULTIGROUP_INVARIANCE_V1.md",
        "validation/results/cbsem_v07_reference_report.json",
    ];
    let artifact_status = artifacts
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            json!({
                "path": relative,
                "present": path.exists(),
                "bytes": fs::metadata(&path).ok().map(|metadata| metadata.len())
            })
        })
        .collect::<Vec<_>>();
    let all_present = artifact_status
        .iter()
        .all(|artifact| artifact["present"].as_bool() == Some(true));
    let validation_path = root.join("validation/results/cbsem_v07_reference_report.json");
    let validation_report = if validation_path.exists() {
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&validation_path)
                .with_context(|| format!("cannot read {}", validation_path.display()))?,
        )
        .context("invalid CB-SEM v0.7 validation report JSON")?
    } else {
        serde_json::Value::Null
    };
    let open_registry_gates = slice
        .gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Open)
        .map(|gate| {
            json!({
                "track": gate.track,
                "name": gate.name,
                "status": gate.status,
                "evidence": gate.evidence
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": 1,
        "target": "v07-cbsem",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "stable_release_allowed": false,
        "generated_at": Utc::now(),
        "all_listed_artifacts_present": all_present,
        "validation_status": validation_report.get("status").cloned().unwrap_or(serde_json::Value::String("missing".into())),
        "artifacts": artifact_status,
        "open_registry_gates": open_registry_gates.clone(),
        "promotion_ready": all_present && evidence_report_passed(&validation_report) && open_registry_gates.is_empty(),
        "note": "v0.7 CB-SEM/CFA is an experimental beta. The current engine is a bounded ML-discrepancy and fit-diagnostics preview seeded by deterministic QuickPLS measurement/structural estimates; full-information SEM optimization and two-reference numerical validation remain later promotion requirements."
    });
    let target = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("validation/results/v07_cbsem_evidence.json"));
    fs::write(&target, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", target.display()))?;
    println!(
        "wrote {} | artifacts_present={} validation_status={} open_gates={}",
        target.display(),
        all_present,
        report["validation_status"].as_str().unwrap_or("unknown"),
        open_registry_gates.len()
    );
    Ok(())
}

fn write_v08_extended_methods_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_8_extended_methods")
        .context("development registry is missing v0_8_extended_methods")?;
    let artifacts = [
        "docs/methods/PCA_V1.md",
        "docs/methods/REGRESSION_OLS_V1.md",
        "docs/methods/REGRESSION_LOGISTIC_V2.md",
        "docs/methods/REGRESSION_LOGISTIC_V1.md",
        "docs/methods/PROCESS_V1.md",
        "docs/methods/NCA_V2.md",
        "docs/methods/NCA_V1.md",
        "docs/methods/GSCA_ALS_V2.md",
        "validation/results/v08_extended_methods_reference_report.json",
    ];
    let artifact_status = artifacts
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            json!({
                "path": relative,
                "present": path.exists(),
                "bytes": fs::metadata(&path).ok().map(|metadata| metadata.len())
            })
        })
        .collect::<Vec<_>>();
    let all_present = artifact_status
        .iter()
        .all(|artifact| artifact["present"].as_bool() == Some(true));
    let validation_path =
        root.join("validation/results/v08_extended_methods_reference_report.json");
    let validation_report = if validation_path.exists() {
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&validation_path)
                .with_context(|| format!("cannot read {}", validation_path.display()))?,
        )
        .context("invalid v0.8 validation report JSON")?
    } else {
        serde_json::Value::Null
    };
    let open_registry_gates = slice
        .gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Open)
        .map(|gate| {
            json!({
                "track": gate.track,
                "name": gate.name,
                "status": gate.status,
                "evidence": gate.evidence
            })
        })
        .collect::<Vec<_>>();
    let validation_status = validation_report
        .get("status")
        .cloned()
        .or_else(|| {
            (validation_report
                .get("passed")
                .and_then(|value| value.as_bool())
                == Some(true))
            .then(|| serde_json::Value::String("passed".into()))
        })
        .unwrap_or(serde_json::Value::String("missing".into()));
    let report = json!({
        "schema_version": 1,
        "target": "v08-extended-methods",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "stable_release_allowed": false,
        "generated_at": Utc::now(),
        "all_listed_artifacts_present": all_present,
        "validation_status": validation_status,
        "artifacts": artifact_status,
        "open_registry_gates": open_registry_gates.clone(),
        "promotion_ready": all_present && evidence_report_passed(&validation_report) && open_registry_gates.is_empty(),
        "note": "v0.8 extended methods are validated only for the documented QuickPLS v1.0.0 supported scope. Unsupported and unaudited shapes remain outside the release scope."
    });
    let target = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("validation/results/v08_extended_methods_evidence.json"));
    fs::write(&target, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", target.display()))?;
    println!(
        "wrote {} | artifacts_present={} validation_status={} open_gates={}",
        target.display(),
        all_present,
        report["validation_status"].as_str().unwrap_or("unknown"),
        open_registry_gates.len()
    );
    Ok(())
}

fn write_publication_ready_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "publication_ready_v0_1_to_v0_8")
        .context("development registry is missing publication_ready_v0_1_to_v0_8")?;
    let method_rows = METHOD_CAPABILITIES
        .iter()
        .map(|method| {
            json!({
                "id": method.id,
                "family": method.family,
                "name": method.name,
                "status": method.status,
            })
        })
        .collect::<Vec<_>>();
    let slice_statuses = registry
        .slices
        .iter()
        .filter(|slice| slice.id.starts_with("v0_") || slice.id == "publication_ready_v0_1_to_v0_8")
        .map(|slice| {
            let summary = slice.gate_summary();
            json!({
                "id": slice.id,
                "release": slice.release,
                "family": slice.family,
                "status": slice.status,
                "stable_output": slice.stable_output,
                "passed": summary.passed,
                "open": summary.open,
                "blocked": summary.blocked,
                "not_applicable": summary.not_applicable,
            })
        })
        .collect::<Vec<_>>();
    let blockers = slice
        .open_gates()
        .map(|gate| {
            json!({
                "track": gate.track,
                "name": gate.name,
                "status": gate.status,
                "required_evidence": gate.evidence,
            })
        })
        .collect::<Vec<_>>();
    let required_artifacts = [
        "docs/PUBLICATION_READY_AUDIT.md",
        "docs/METHOD_COMPATIBILITY.md",
        "validation/development_slices.json",
        "validation/results/publication_promotion_matrix.json",
        "validation/results/r_validation_runtime_audit.json",
        "validation/results/foundation_publication_audit.json",
        "validation/results/data_project_publication_audit.json",
        "validation/results/pls_publication_audit.json",
        "validation/results/pls_publication_bounded_benchmark.json",
        "validation/results/assessment_publication_metric_matrix.json",
        "validation/results/assessment_publication_audit.json",
        "validation/results/inference_publication_matrix.json",
        "validation/results/inference_publication_audit.json",
        "validation/results/extended_pls_publication_audit.json",
        "validation/results/prediction_heterogeneity_publication_audit.json",
        "validation/results/cbsem_publication_audit.json",
        "validation/results/extended_methods_publication_audit.json",
        "validation/results/gui_diagram_publication_audit.json",
        "validation/results/stable_export_publication_audit.json",
        "validation/results/documentation_publication_audit.json",
        "validation/results/performance_release_publication_audit.json",
        "validation/results/v09_smoke_check.json",
        "validation/results/v09_release_candidate_audit.json",
        "docs/RELEASE_NOTES_V0_9_RC1.md",
        "docs/SUPPORTED_SCOPE_V0_9_RC1.md",
        "docs/DEPENDENCY_NOTICES.md",
        "docs/KNOWN_DIFFERENCES.md",
    ];
    let artifacts = required_artifacts
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            json!({
                "path": relative,
                "present": path.exists(),
                "bytes": fs::metadata(&path).ok().map(|metadata| metadata.len())
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": 1,
        "target": "publication_ready_v0_1_to_v0_8",
        "generated_at": Utc::now(),
        "passed": blockers.is_empty(),
        "stable_release_allowed": blockers.is_empty(),
        "blocker_count": blockers.len(),
        "blockers": blockers,
        "slice_statuses": slice_statuses,
        "method_capabilities": method_rows,
        "artifacts": artifacts,
        "required_rscript": r"C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\x64\Rscript.exe",
        "note": "This audit is intentionally conservative. v0.1-v0.8 cannot be marked publication-ready until every blocker is closed with reproducible evidence."
    });
    let target = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("validation/results/publication_ready_audit.json"));
    fs::write(&target, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", target.display()))?;
    println!(
        "wrote {} | passed={} blockers={}",
        target.display(),
        report["passed"].as_bool().unwrap_or(false),
        report["blocker_count"].as_u64().unwrap_or(0)
    );
    Ok(())
}

fn evidence_report_passed(report: &serde_json::Value) -> bool {
    if report.is_null() {
        return false;
    }
    if report.get("passed").and_then(|value| value.as_bool()) == Some(true) {
        return true;
    }
    if report.get("status").and_then(|value| value.as_str()) == Some("passed") {
        return true;
    }
    if let Some(checks) = report.get("checks").and_then(|value| value.as_object()) {
        return checks
            .values()
            .filter(|value| value.is_boolean())
            .all(|value| value.as_bool() == Some(true));
    }
    false
}

fn roadmap(json_output: bool, release: Option<&str>) -> Result<()> {
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let errors = validate_slice_registry(&registry);
    if !errors.is_empty() {
        bail!("development registry is invalid: {}", errors.join("; "));
    }
    let slices = registry
        .slices
        .iter()
        .filter(|slice| release.is_none_or(|release| slice.release == release))
        .collect::<Vec<_>>();
    if json_output {
        println!("{}", serde_json::to_string_pretty(&slices)?);
        return Ok(());
    }
    println!(
        "{} | current stage: {}",
        registry.program, registry.current_stage
    );
    println!("goal: {}", registry.active_goal);
    for slice in slices {
        let gates = slice.gate_summary();
        println!(
            "{:<36} {:<5} {:<13} gates passed/open/blocked: {}/{}/{}",
            slice.id,
            slice.release,
            format!("{:?}", slice.status).to_lowercase(),
            gates.passed,
            gates.open,
            gates.blocked
        );
        if let Some(next) = slice.next_actions.first() {
            println!("  next: {next}");
        }
    }
    Ok(())
}

fn gate(slice_id: &str, json_output: bool) -> Result<()> {
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let errors = validate_slice_registry(&registry);
    if !errors.is_empty() {
        bail!("development registry is invalid: {}", errors.join("; "));
    }
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .with_context(|| format!("unknown development slice {slice_id}"))?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(slice)?);
        return Ok(());
    }
    let gates = slice.gate_summary();
    println!(
        "{} ({}) | {:?} | gates passed/open/blocked: {}/{}/{}",
        slice.name, slice.release, slice.status, gates.passed, gates.open, gates.blocked
    );
    println!("{}", slice.summary);
    let open_gates = slice.open_gates().collect::<Vec<_>>();
    if open_gates.is_empty() {
        println!("promotion gate: clear");
    } else {
        println!("promotion blockers:");
        for gate in open_gates {
            println!("  - [{:?}] {}: {}", gate.status, gate.name, gate.evidence);
        }
    }
    if !slice.next_actions.is_empty() {
        println!("next actions:");
        for action in &slice.next_actions {
            println!("  - {action}");
        }
    }
    Ok(())
}

fn write_v04_assessment_evidence(output: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_4_assessment_reliability")
        .context("development registry is missing v0_4_assessment_reliability")?;
    let artifacts = [
        "validation/fixtures/simple_reflective.csv",
        "validation/fixtures/simple_reflective.recipe.json",
        "validation/fixtures/corporate_reputation.csv",
        "validation/fixtures/rho_a_reference.csv",
        "validation/fixtures/rho_a_reference.recipe.json",
        "validation/results/rho_a_reference.json",
        "validation/results/rho_a_primary_dijkstra_henseler_2015.json",
        "validation/results/rho_a_csem_0_6_1.csv",
        "validation/results/rho_a_quickpls_reference.json",
        "validation/results/rho_a_csem_comparison.json",
        "validation/results/htmt_reference.json",
        "validation/results/htmt_csem_0_6_1.csv",
        "validation/results/htmt_quickpls_reference.json",
        "validation/results/htmt_csem_comparison.json",
        "validation/results/htmt_seminr_2_5_0.csv",
        "validation/results/htmt_seminr_comparison.json",
        "validation/results/htmt_published_ringle_2023.json",
        "validation/results/assessment_csem_0_6_1.csv",
        "validation/results/assessment_quickpls_reference.json",
        "validation/results/assessment_csem_comparison.json",
        "validation/results/blindfolding_quickpls_reference.json",
        "validation/results/blindfolding_python_reference.json",
        "validation/results/blindfolding_python_comparison.json",
        "validation/results/assessment_simulation.csv",
        "validation/results/assessment_simulation_broken.csv",
        "validation/results/assessment_simulation.recipe.json",
        "validation/results/assessment_simulation_broken.recipe.json",
        "validation/results/assessment_simulation_quickpls.json",
        "validation/results/assessment_simulation_broken_quickpls.json",
        "validation/results/assessment_simulation_report.json",
        "validation/results/assessment_published_satisfaction.csv",
        "validation/results/assessment_published_satisfaction_csem_0_6_1.csv",
        "validation/results/assessment_published_satisfaction.recipe.json",
        "validation/results/assessment_published_satisfaction_quickpls.json",
        "validation/results/assessment_published_satisfaction_comparison.json",
        "validation/results/external_reference_probe.json",
        "validation/demo/quickpls_v04_demo.validation.json",
    ];
    let artifact_status = artifacts
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            json!({
                "path": relative,
                "present": path.exists(),
                "bytes": fs::metadata(&path).ok().map(|metadata| metadata.len())
            })
        })
        .collect::<Vec<_>>();
    let all_artifacts_present = artifact_status
        .iter()
        .all(|artifact| artifact["present"].as_bool() == Some(true));
    let metrics = vec![
        evidence_metric(
            "cronbach_alpha",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
                "qpls-core::statistics::alpha_matches_hand_calculated_fixture",
            ],
            &[],
        ),
        evidence_metric(
            "rho_c",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
            ],
            &[],
        ),
        evidence_metric(
            "ave",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
            ],
            &[],
        ),
        evidence_metric(
            "cross_loadings",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
            ],
            &[],
        ),
        evidence_metric(
            "fornell_larcker",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
            ],
            &[],
        ),
        evidence_metric(
            "r_squared",
            "validated_reference_fixture",
            "1e-6",
            &[
                "docs/methods/PLS_ASSESSMENT_V1.md",
                "validation/fixtures/simple_reflective.csv",
                "qpls-assessment::reference_fixture_matches_csem_assessment",
            ],
            &[],
        ),
        evidence_metric(
            "rho_a",
            "fixture_covered_external_reference_open",
            "1e-12 for committed Decimal fixtures; 1e-6 required for external engines",
            &[
                "docs/methods/PLS_RHO_A_V1.md",
                "validation/fixtures/rho_a_reference.csv",
                "validation/fixtures/rho_a_reference.recipe.json",
                "validation/results/rho_a_reference.json",
                "validation/results/rho_a_primary_dijkstra_henseler_2015.json",
                "validation/results/rho_a_csem_0_6_1.csv",
                "validation/results/rho_a_quickpls_reference.json",
                "validation/results/rho_a_csem_comparison.json",
                "validation/results/external_reference_probe.json",
                "qpls-assessment::rho_a_matches_dijkstra_henseler_2015_equation_3_fixture",
                "qpls-assessment::rho_a_matches_independent_decimal_reference_and_metamorphics",
                "qpls-assessment::rho_a_matches_three_and_two_indicator_hand_fixtures",
            ],
            &[],
        ),
        evidence_metric(
            "htmt_original",
            "fixture_covered_external_reference_open",
            "1e-12 for independent formula fixture; 5e-4 for rounded published appendix matrices; 1e-6 for external engines",
            &[
                "docs/methods/PLS_HTMT_V1.md",
                "validation/htmt_reference.py",
                "validation/results/htmt_reference.json",
                "validation/results/htmt_csem_0_6_1.csv",
                "validation/results/htmt_quickpls_reference.json",
                "validation/results/htmt_csem_comparison.json",
                "validation/results/htmt_published_ringle_2023.json",
                "validation/results/external_reference_probe.json",
                "qpls-assessment::htmt_plus_matches_ringle_2023_rounded_formula_examples",
                "qpls-assessment::htmt_matches_independent_corporate_reputation_reference",
            ],
            &[],
        ),
        evidence_metric(
            "htmt_plus",
            "fixture_covered_external_reference_open",
            "1e-12 for independent formula fixture; 5e-4 for rounded published appendix matrices; 1e-6 for external engines",
            &[
                "docs/methods/PLS_HTMT_V1.md",
                "validation/htmt_reference.py",
                "validation/results/htmt_reference.json",
                "validation/results/htmt_csem_comparison.json documents that cSEM .absolute=TRUE is not equivalent to Ringle et al. HTMT+ for mixed-sign cross-block correlations",
                "validation/results/htmt_seminr_2_5_0.csv",
                "validation/results/htmt_seminr_comparison.json",
                "validation/results/htmt_published_ringle_2023.json",
                "validation/results/external_reference_probe.json",
                "qpls-assessment::htmt_plus_matches_ringle_2023_rounded_formula_examples",
                "qpls-assessment::htmt_matches_independent_corporate_reputation_reference",
            ],
            &[],
        ),
        evidence_metric(
            "vif_adjusted_r2_f2_q2_srmr_duls",
            "partially_covered_external_reference_gap",
            "1e-6 for cSEM-equivalent R2, adjusted R2, structural VIF, fixed-score f2, SRMR, and d_ULS",
            &[
                "docs/methods/PLS_ASSESSMENT_V4.md",
                "validation/results/assessment_csem_0_6_1.csv",
                "validation/results/assessment_quickpls_reference.json",
                "validation/results/assessment_csem_comparison.json",
                "validation/results/blindfolding_python_reference.json",
                "validation/results/blindfolding_python_comparison.json",
                "validation/results/assessment_simulation_report.json",
                "validation/results/assessment_published_satisfaction_comparison.json",
                "qpls-assessment focused unit and metamorphic tests",
            ],
            &[],
        ),
    ];
    let open_metric_blockers = metrics
        .iter()
        .filter(|metric| {
            metric["missing_evidence"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
        .count();
    let report = json!({
        "schema_version": 1,
        "target": "v04-assessment",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "assessment_method_version": ASSESSMENT_METHOD_VERSION,
        "rho_a_method_version": RHO_A_METHOD_VERSION,
        "htmt_plus_method_version": HTMT_PLUS_METHOD_VERSION,
        "htmt_original_method_version": HTMT_ORIGINAL_METHOD_VERSION,
        "generated_at": Utc::now(),
        "stable_release_allowed": false,
        "all_listed_artifacts_present": all_artifacts_present,
        "open_metric_blockers": open_metric_blockers,
        "note": "Evidence traceability report only. Metrics with missing_evidence remain experimental and must not be presented as publication-validated.",
        "artifacts": artifact_status,
        "metrics": metrics,
        "open_registry_gates": slice.open_gates().collect::<Vec<_>>()
    });
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/results/v04_assessment_evidence.json"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", output.display()))?;
    println!(
        "wrote {} | open_metric_blockers={}",
        output.display(),
        open_metric_blockers
    );
    Ok(())
}

fn evidence_metric(
    id: &str,
    status: &str,
    tolerance: &str,
    evidence: &[&str],
    missing_evidence: &[&str],
) -> serde_json::Value {
    json!({
        "id": id,
        "status": status,
        "tolerance": tolerance,
        "evidence": evidence,
        "missing_evidence": missing_evidence
    })
}

fn qualify(
    target: QualificationTarget,
    output: Option<&Path>,
    refresh_quick_monte_carlo: bool,
    refresh_pilot_monte_carlo: bool,
) -> Result<()> {
    match target {
        QualificationTarget::V04Inference => {
            qualify_v04_inference(output, refresh_quick_monte_carlo, refresh_pilot_monte_carlo)
        }
    }
}

fn qualify_v04_inference(
    output: Option<&Path>,
    refresh_quick_monte_carlo: bool,
    refresh_pilot_monte_carlo: bool,
) -> Result<()> {
    let root = repository_root()?;
    let registry = development_slice_registry().context("invalid bundled development registry")?;
    let slice = registry
        .slices
        .iter()
        .find(|slice| slice.id == "v0_4_inference_resampling")
        .context("development registry is missing v0_4_inference_resampling")?;
    let worker_matrix = run_cli_worker_matrix(&root)?;
    let cancellation_latency = run_bootstrap_cancellation_latency(&root)?;
    let studentized_cancellation_latency = run_studentized_cancellation_latency(&root)?;
    let quick_monte_carlo = run_or_read_quick_monte_carlo(&root, refresh_quick_monte_carlo)?;
    let pilot_monte_carlo = run_or_read_pilot_monte_carlo(&root, refresh_pilot_monte_carlo)?;
    let sensitivity_monte_carlo =
        run_or_read_sensitivity_monte_carlo(&root, refresh_pilot_monte_carlo)?;
    let studentized_monte_carlo = read_studentized_monte_carlo(&root)?;
    let studentized_sensitivity = read_studentized_sensitivity(&root)?;
    let full_studentized_monte_carlo = read_full_studentized_monte_carlo_qualification(&root)?;
    let studentized_reference = read_studentized_supplied_reference(&root)?;
    let studentized_minimum = read_studentized_minimum_execution(&root)?;
    let studentized_worker_matrix = read_studentized_worker_matrix(&root)?;
    let studentized_performance = read_studentized_performance(&root)?;
    let studentized_release_stress = read_studentized_release_stress(&root)?;
    let pls_bootstrap_external_reference = read_pls_bootstrap_external_reference(&root)?;
    let pls_bootstrap_corporate_csem_reference =
        read_pls_bootstrap_corporate_csem_reference(&root)?;
    let pls_bootstrap_plspm_external_reference =
        read_pls_bootstrap_plspm_external_reference(&root)?;
    let full_monte_carlo = read_full_monte_carlo_qualification(&root)?;
    let checks = vec![
        json!({
            "id": "cli_worker_matrix_1_2_4",
            "status": if worker_matrix["passed"].as_bool() == Some(true) { "passed" } else { "failed" },
            "evidence": worker_matrix
        }),
        json!({
            "id": "bootstrap_cancellation_latency",
            "status": if cancellation_latency["passed"].as_bool() == Some(true) { "passed" } else { "failed" },
            "evidence": cancellation_latency
        }),
        json!({
            "id": "studentized_cancellation_latency_999x99",
            "status": if studentized_cancellation_latency["passed"].as_bool() == Some(true) { "passed" } else { "failed" },
            "evidence": studentized_cancellation_latency
        }),
        json!({
            "id": "quick_monte_carlo_harness",
            "status": if quick_monte_carlo["usable"].as_bool() == Some(true) { "passed" } else { "failed" },
            "evidence": quick_monte_carlo
        }),
        json!({
            "id": "pilot_monte_carlo_harness",
            "status": if pilot_monte_carlo["usable"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": pilot_monte_carlo
        }),
        json!({
            "id": "sensitivity_monte_carlo_harness",
            "status": if sensitivity_monte_carlo["usable"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": sensitivity_monte_carlo
        }),
        json!({
            "id": "studentized_monte_carlo_harness",
            "status": if studentized_monte_carlo["usable"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_monte_carlo
        }),
        json!({
            "id": "studentized_sensitivity_harness",
            "status": if studentized_sensitivity["usable"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_sensitivity
        }),
        json!({
            "id": "full_studentized_monte_carlo_qualification",
            "status": if full_studentized_monte_carlo["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": full_studentized_monte_carlo
        }),
        json!({
            "id": "studentized_supplied_reference",
            "status": if studentized_reference["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_reference
        }),
        json!({
            "id": "studentized_minimum_999x99_execution",
            "status": if studentized_minimum["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_minimum
        }),
        json!({
            "id": "studentized_worker_matrix_999x99",
            "status": if studentized_worker_matrix["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_worker_matrix
        }),
        json!({
            "id": "studentized_performance_benchmark",
            "status": if studentized_performance["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_performance
        }),
        json!({
            "id": "studentized_release_stress_benchmark",
            "status": if studentized_release_stress["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": studentized_release_stress
        }),
        json!({
            "id": "pls_bootstrap_external_reference",
            "status": if pls_bootstrap_external_reference["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": pls_bootstrap_external_reference
        }),
        json!({
            "id": "pls_bootstrap_corporate_csem_reference",
            "status": if pls_bootstrap_corporate_csem_reference["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": pls_bootstrap_corporate_csem_reference
        }),
        json!({
            "id": "pls_bootstrap_plspm_external_reference",
            "status": if pls_bootstrap_plspm_external_reference["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": pls_bootstrap_plspm_external_reference
        }),
        json!({
            "id": "full_monte_carlo_qualification",
            "status": if full_monte_carlo["passed"].as_bool() == Some(true) { "passed" } else { "open" },
            "evidence": full_monte_carlo
        }),
    ];
    let qualification_passed = checks
        .iter()
        .all(|check| check["status"].as_str() == Some("passed"));
    let report = json!({
        "schema_version": 1,
        "target": "v04-inference",
        "slice_id": slice.id,
        "slice_status": slice.status,
        "generated_at": Utc::now(),
        "qualification_passed": qualification_passed,
        "stable_release_allowed": false,
        "note": if qualification_passed {
            "Automated checks passed. Promotion still requires human review of method scope, known differences, and remaining registry gates."
        } else {
            "This accelerator report is not publication evidence. Open checks mean v0.4 inference remains experimental."
        },
        "checks": checks,
        "open_registry_gates": slice.open_gates().collect::<Vec<_>>()
    });
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/results/v04_inference_qualification_quick.json"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write {}", output.display()))?;
    println!(
        "wrote {} | qualification_passed={}",
        output.display(),
        qualification_passed
    );
    Ok(())
}

fn repository_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("cannot resolve repository root")
}

fn run_cli_worker_matrix(root: &Path) -> Result<serde_json::Value> {
    let historical_recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
    let data = root.join("validation/fixtures/simple_reflective.csv");
    let directory = root.join("target/qualification/v04-inference");
    fs::create_dir_all(&directory)?;
    let recipe = directory.join("simple_reflective.v3.recipe.json");
    let historical: AnalysisRecipe = serde_json::from_slice(
        &fs::read(&historical_recipe)
            .with_context(|| format!("cannot read {}", historical_recipe.display()))?,
    )
    .context("invalid historical worker-matrix recipe JSON")?;
    fs::write(
        &recipe,
        serde_json::to_vec_pretty(
            &historical
                .migrated_v3()
                .context("cannot migrate worker-matrix recipe to schema v3")?,
        )?,
    )?;
    let mut payloads = Vec::new();
    let mut diagnostics = Vec::new();
    for workers in [1, 2, 4] {
        let output = directory.join(format!("worker-{workers}.json"));
        run_analysis(
            &recipe,
            Some(&data),
            None,
            &output,
            true,
            false,
            true,
            Some(24),
            None,
            Some(99),
            Some(workers),
        )
        .with_context(|| format!("worker-matrix run failed for workers={workers}"))?;
        let envelope: AnalysisResult = serde_json::from_slice(
            &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
        )?;
        payloads.push(serde_json::to_value(&envelope.payload)?);
        diagnostics.push(serde_json::to_value(&envelope.diagnostics)?);
    }
    let payload_equal = payloads.windows(2).all(|pair| pair[0] == pair[1]);
    let diagnostics_equal = diagnostics.windows(2).all(|pair| pair[0] == pair[1]);
    Ok(json!({
        "passed": payload_equal && diagnostics_equal,
        "workers": [1, 2, 4],
        "bootstrap_samples": 24,
        "permutation_samples": 99,
        "payload_equal": payload_equal,
        "diagnostics_equal": diagnostics_equal,
        "artifact_directory": directory
    }))
}

fn run_bootstrap_cancellation_latency(root: &Path) -> Result<serde_json::Value> {
    let recipe_path = root.join("validation/fixtures/simple_reflective.recipe.json");
    let data_path = root.join("validation/fixtures/simple_reflective.csv");
    let dataset = import_path(&data_path, &ImportOptions::default())
        .with_context(|| format!("cannot import {}", data_path.display()))?;
    let historical: AnalysisRecipe = serde_json::from_slice(
        &fs::read(&recipe_path)
            .with_context(|| format!("cannot read {}", recipe_path.display()))?,
    )
    .context("invalid cancellation benchmark recipe JSON")?;
    let mut recipe = historical
        .migrated_v3()
        .context("cannot migrate cancellation benchmark recipe to schema v3")?;
    recipe.settings.bootstrap_samples = 10_000;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.permutation_samples = 0;
    recipe.settings.workers = 4;
    recipe.method_config = Some(MethodConfig::PlsBootstrap);
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    base_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    let original = qpls_estimation::estimate_pls(&dataset, &base_recipe)
        .context("base PLS estimate failed")?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let completed_before_request = Arc::new(AtomicU64::new(0));
    let request_started = Arc::new(Mutex::new(None::<Instant>));
    let cancelled_for_check = cancelled.clone();
    let cancelled_for_progress = cancelled.clone();
    let completed_for_progress = completed_before_request.clone();
    let request_started_for_progress = request_started.clone();
    let result = bootstrap_pls(
        &dataset,
        &recipe,
        &original,
        recipe.settings.workers,
        move || cancelled_for_check.load(Ordering::Relaxed),
        move |progress| {
            if progress.phase == qpls_resampling::ResamplingPhase::Bootstrap
                && progress.completed_replicates >= 1
                && !cancelled_for_progress.swap(true, Ordering::Relaxed)
            {
                completed_for_progress
                    .store(progress.completed_replicates as u64, Ordering::Relaxed);
                *request_started_for_progress
                    .lock()
                    .expect("cancellation latency mutex poisoned") = Some(Instant::now());
            }
        },
    );
    let elapsed = request_started
        .lock()
        .expect("cancellation latency mutex poisoned")
        .map(|started| started.elapsed().as_secs_f64());
    let cancelled_result = result.is_err()
        && result
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("cancel"));
    let threshold_seconds = 1.0;
    Ok(json!({
        "passed": cancelled_result && elapsed.is_some_and(|value| value <= threshold_seconds),
        "cancelled_result": cancelled_result,
        "requested_replicates": recipe.settings.bootstrap_samples,
        "workers": recipe.settings.workers,
        "cancel_requested_after_completed_replicates": completed_before_request.load(Ordering::Relaxed),
        "elapsed_seconds_after_cancel_request": elapsed,
        "threshold_seconds": threshold_seconds,
        "error": result.err().map(|error| error.to_string())
    }))
}

fn run_studentized_cancellation_latency(root: &Path) -> Result<serde_json::Value> {
    let recipe_path = root.join("validation/fixtures/simple_reflective.recipe.json");
    let data_path = root.join("validation/fixtures/simple_reflective.csv");
    let dataset = import_path(&data_path, &ImportOptions::default())
        .with_context(|| format!("cannot import {}", data_path.display()))?;
    let historical: AnalysisRecipe = serde_json::from_slice(
        &fs::read(&recipe_path)
            .with_context(|| format!("cannot read {}", recipe_path.display()))?,
    )
    .context("invalid studentized cancellation benchmark recipe JSON")?;
    let mut recipe = historical
        .migrated_v3()
        .context("cannot migrate studentized cancellation benchmark recipe to schema v3")?;
    recipe.settings.bootstrap_samples = 999;
    recipe.settings.studentized_inner_samples = 99;
    recipe.settings.permutation_samples = 0;
    recipe.settings.workers = 4;
    recipe.method_config = Some(MethodConfig::PlsBootstrap);
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    base_recipe.settings.studentized_inner_samples = 0;
    base_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    let original = qpls_estimation::estimate_pls(&dataset, &base_recipe)
        .context("base PLS estimate failed")?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let completed_before_request = Arc::new(AtomicU64::new(0));
    let request_started = Arc::new(Mutex::new(None::<Instant>));
    let cancelled_for_check = cancelled.clone();
    let cancelled_for_progress = cancelled.clone();
    let completed_for_progress = completed_before_request.clone();
    let request_started_for_progress = request_started.clone();
    let result = bootstrap_pls(
        &dataset,
        &recipe,
        &original,
        recipe.settings.workers,
        move || cancelled_for_check.load(Ordering::Relaxed),
        move |progress| {
            if progress.phase == ResamplingPhase::StudentizedInner
                && progress.completed_replicates >= 1
                && !cancelled_for_progress.swap(true, Ordering::Relaxed)
            {
                completed_for_progress
                    .store(progress.completed_replicates as u64, Ordering::Relaxed);
                *request_started_for_progress
                    .lock()
                    .expect("studentized cancellation latency mutex poisoned") =
                    Some(Instant::now());
            }
        },
    );
    let elapsed = request_started
        .lock()
        .expect("studentized cancellation latency mutex poisoned")
        .map(|started| started.elapsed().as_secs_f64());
    let cancelled_result = result.is_err()
        && result
            .as_ref()
            .err()
            .is_some_and(|error| error.to_string().contains("cancel"));
    let threshold_seconds = 1.0;
    Ok(json!({
        "passed": cancelled_result && elapsed.is_some_and(|value| value <= threshold_seconds),
        "cancelled_result": cancelled_result,
        "requested_primary_replicates": recipe.settings.bootstrap_samples,
        "requested_studentized_inner_replicates": recipe.settings.studentized_inner_samples,
        "requested_studentized_inner_fits": recipe.settings.bootstrap_samples.saturating_mul(recipe.settings.studentized_inner_samples),
        "workers": recipe.settings.workers,
        "phase_trigger": ResamplingPhase::StudentizedInner.as_str(),
        "cancel_requested_after_completed_inner_replicates": completed_before_request.load(Ordering::Relaxed),
        "elapsed_seconds_after_cancel_request": elapsed,
        "threshold_seconds": threshold_seconds,
        "error": result.err().map(|error| error.to_string()),
        "note": "Cancellation is requested only after nested studentized-inner progress is observed, proving the 999x99 path discards partial output from inside the nested phase."
    }))
}

fn run_or_read_quick_monte_carlo(root: &Path, refresh: bool) -> Result<serde_json::Value> {
    run_or_read_monte_carlo_harness(
        root,
        refresh,
        "quick",
        "validation/results/monte_carlo_quick.json",
        8,
        79,
        "Quick Monte Carlo is an integration and determinism check only; it is not coverage qualification evidence.",
    )
}

fn run_or_read_pilot_monte_carlo(root: &Path, refresh: bool) -> Result<serde_json::Value> {
    run_or_read_monte_carlo_harness(
        root,
        refresh,
        "pilot",
        "validation/results/monte_carlo_pilot.json",
        32,
        199,
        "Pilot Monte Carlo is an early-warning coverage/type-I screen only; it is not release qualification evidence.",
    )
}

fn run_or_read_sensitivity_monte_carlo(root: &Path, refresh: bool) -> Result<serde_json::Value> {
    run_or_read_monte_carlo_harness(
        root,
        refresh,
        "sensitivity",
        "validation/results/monte_carlo_sensitivity.json",
        96,
        399,
        "Sensitivity Monte Carlo is a stronger deterministic drift screen than pilot mode; it is still not release qualification evidence.",
    )
}

fn read_studentized_monte_carlo(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/monte_carlo_studentized.json");
    if !output.exists() {
        return Ok(json!({
            "usable": false,
            "path": output,
            "reason": "missing_studentized_monte_carlo_report",
            "required": "Run npm run qpls:studentized:monte-carlo to generate the bounded studentized 999x99 Monte Carlo pilot report.",
            "note": "This is an early-warning 999x99 studentized pilot only; it is not full coverage qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized Monte Carlo JSON")?;
    let configuration = &report["configuration"];
    let scenarios = report
        .get("scenarios")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let enough_requested = configuration["simulations_per_scenario"].as_u64() >= Some(4)
        && configuration["bootstrap_replicates"].as_u64() >= Some(999)
        && configuration["studentized_inner_replicates"].as_u64() >= Some(99);
    let scenarios_ok = scenarios.len() == 2
        && scenarios.iter().all(|scenario| {
            let completed = scenario["completed_simulations"].as_u64().unwrap_or(0);
            scenario["failed_simulations"].as_u64() == Some(0)
                && completed >= 4
                && scenario
                    .pointer("/studentized/available")
                    .and_then(serde_json::Value::as_u64)
                    == Some(completed)
                && scenario
                    .pointer("/studentized/coverage_rate")
                    .and_then(serde_json::Value::as_f64)
                    .is_some()
                && scenario
                    .pointer("/studentized/exclusion_of_zero_rate")
                    .and_then(serde_json::Value::as_f64)
                    .is_some()
        });
    let evaluated = report
        .pointer("/qualification/evaluated")
        .and_then(serde_json::Value::as_bool);
    let usable = report["mode"].as_str() == Some("studentized")
        && evaluated == Some(false)
        && enough_requested
        && scenarios_ok;
    Ok(json!({
        "usable": usable,
        "path": output,
        "mode": report["mode"],
        "harness_version": report["harness_version"],
        "engine_versions": report["engine_versions"],
        "configuration": report["configuration"],
        "studentized_scenarios": scenarios.iter().map(|scenario| json!({
            "name": scenario["name"],
            "completed_simulations": scenario["completed_simulations"],
            "failed_simulations": scenario["failed_simulations"],
            "studentized": scenario["studentized"],
            "bias": scenario["bias"]
        })).collect::<Vec<_>>(),
        "qualification_evaluated": evaluated,
        "elapsed_seconds": report["elapsed_seconds"],
        "note": "Bounded 999x99 studentized Monte Carlo pilot. It proves availability and early coverage/type-I plumbing only; full preregistered studentized qualification remains open."
    }))
}

fn read_studentized_sensitivity(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/monte_carlo_studentized_sensitivity.json");
    if !output.exists() {
        return Ok(json!({
            "usable": false,
            "path": output,
            "reason": "missing_studentized_sensitivity_report",
            "required": "Run npm run qpls:studentized:sensitivity to generate the bounded normal/heavy-tail 999x99 studentized sensitivity report.",
            "note": "This is scenario-sensitivity plumbing evidence only; it is not preregistered large-simulation qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized sensitivity Monte Carlo JSON")?;
    let configuration = &report["configuration"];
    let scenarios = report
        .get("scenarios")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_names = [
        "coverage_beta_0_35",
        "null_beta_0",
        "heavy_tail_coverage_beta_0_35",
        "heavy_tail_null_beta_0",
    ];
    let enough_requested = configuration["simulations_per_scenario"].as_u64() >= Some(4)
        && configuration["bootstrap_replicates"].as_u64() >= Some(999)
        && configuration["studentized_inner_replicates"].as_u64() >= Some(99);
    let scenarios_ok = expected_names.iter().all(|name| {
        scenarios.iter().any(|scenario| {
            let completed = scenario["completed_simulations"].as_u64().unwrap_or(0);
            scenario["name"].as_str() == Some(*name)
                && scenario["failed_simulations"].as_u64() == Some(0)
                && completed >= 4
                && scenario
                    .pointer("/studentized/available")
                    .and_then(serde_json::Value::as_u64)
                    == Some(completed)
                && scenario
                    .pointer("/studentized/coverage_rate")
                    .and_then(serde_json::Value::as_f64)
                    .is_some()
                && scenario
                    .pointer("/studentized/exclusion_of_zero_rate")
                    .and_then(serde_json::Value::as_f64)
                    .is_some()
        })
    });
    let evaluated = report
        .pointer("/qualification/evaluated")
        .and_then(serde_json::Value::as_bool);
    let usable = report["mode"].as_str() == Some("studentized-sensitivity")
        && evaluated == Some(false)
        && enough_requested
        && scenarios_ok;
    Ok(json!({
        "usable": usable,
        "path": output,
        "mode": report["mode"],
        "harness_version": report["harness_version"],
        "engine_versions": report["engine_versions"],
        "configuration": report["configuration"],
        "studentized_scenarios": scenarios.iter().map(|scenario| json!({
            "name": scenario["name"],
            "error_distribution": scenario["error_distribution"],
            "completed_simulations": scenario["completed_simulations"],
            "failed_simulations": scenario["failed_simulations"],
            "studentized": scenario["studentized"],
            "bias": scenario["bias"]
        })).collect::<Vec<_>>(),
        "qualification_evaluated": evaluated,
        "elapsed_seconds": report["elapsed_seconds"],
        "note": "Bounded normal/heavy-tail 999x99 studentized scenario-sensitivity pilot. Full preregistered studentized coverage remains open."
    }))
}

fn read_full_studentized_monte_carlo_qualification(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/monte_carlo_studentized_qualification.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_full_studentized_qualification_report",
            "required": "Run npm run qpls:studentized:qualification on documented hardware and commit a report with qualification.evaluated=true and qualification.passed=true.",
            "note": "This is intentionally expensive: 1,000 simulations per normal/heavy-tail scenario with 999 outer and 99 inner bootstrap replicates."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid full studentized Monte Carlo qualification JSON")?;
    let configuration = &report["configuration"];
    let scenarios = report
        .get("scenarios")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_names = [
        "coverage_beta_0_35",
        "null_beta_0",
        "heavy_tail_coverage_beta_0_35",
        "heavy_tail_null_beta_0",
    ];
    let scenarios_ok = expected_names.iter().all(|name| {
        scenarios.iter().any(|scenario| {
            let completed = scenario["completed_simulations"].as_u64().unwrap_or(0);
            scenario["name"].as_str() == Some(*name)
                && completed >= 1_000
                && scenario["failed_simulations"].as_u64() == Some(0)
                && scenario
                    .pointer("/studentized/available")
                    .and_then(serde_json::Value::as_u64)
                    == Some(completed)
        })
    });
    let required_metrics = [
        "studentized_coverage",
        "studentized_type_i",
        "alternative_studentized_availability",
        "null_studentized_availability",
        "heavy_tail_studentized_coverage",
        "heavy_tail_studentized_type_i",
        "heavy_tail_alternative_studentized_availability",
        "heavy_tail_null_studentized_availability",
    ];
    let checks = report
        .pointer("/qualification/checks")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required_checks_pass = required_metrics.iter().all(|metric| {
        checks.iter().any(|check| {
            check["metric"].as_str() == Some(*metric) && check["passed"].as_bool() == Some(true)
        })
    });
    let evaluated = report
        .pointer("/qualification/evaluated")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let qualification_passed = report
        .pointer("/qualification/passed")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let passed = report["mode"].as_str() == Some("studentized-qualification")
        && evaluated
        && qualification_passed
        && configuration["simulations_per_scenario"].as_u64() >= Some(1_000)
        && configuration["bootstrap_replicates"].as_u64() >= Some(999)
        && configuration["studentized_inner_replicates"].as_u64() >= Some(99)
        && scenarios_ok
        && required_checks_pass;
    Ok(json!({
        "passed": passed,
        "path": output,
        "mode": report["mode"],
        "harness_version": report["harness_version"],
        "engine_versions": report["engine_versions"],
        "configuration": report["configuration"],
        "qualification_evaluated": evaluated,
        "qualification_passed": qualification_passed,
        "scenario_count": scenarios.len(),
        "scenarios": scenarios.iter().map(|scenario| json!({
            "name": scenario["name"],
            "error_distribution": scenario["error_distribution"],
            "completed_simulations": scenario["completed_simulations"],
            "failed_simulations": scenario["failed_simulations"],
            "studentized": scenario["studentized"],
            "bias": scenario["bias"]
        })).collect::<Vec<_>>(),
        "required_studentized_checks_pass": required_checks_pass,
        "note": "Full preregistered studentized Monte Carlo qualification evidence. This is only accepted when normal and heavy-tail scenarios both meet the configured thresholds."
    }))
}

fn read_studentized_supplied_reference(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/studentized_supplied_reference.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_studentized_supplied_reference",
            "required": "Run npm run qpls:studentized:reference to compare the supplied bootstrap-t fixture against independent Python and R Type-7 references.",
            "note": "This is formula/reference evidence for supplied values; it is not full PLS simulation qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized supplied-reference JSON")?;
    let passed = report["kind"].as_str() == Some("studentized_supplied_reference_v1")
        && report["passed"].as_bool() == Some(true)
        && report["r_type7_max_abs_difference"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-12)
        && report["r_type7"].is_object()
        && report["independent_python"].is_object();
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "tolerance": report["tolerance"],
        "r_type7_max_abs_difference": report["r_type7_max_abs_difference"],
        "independent_python": report["independent_python"],
        "r_type7": report["r_type7"],
        "r_boot_ci_stud": report["r_boot_ci_stud"],
        "r_boot_ci_difference_from_type7": report["r_boot_ci_difference_from_type7"],
        "note": report["r_boot_ci_difference_note"]
    }))
}

fn read_studentized_minimum_execution(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/studentized_minimum_quickpls.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_studentized_minimum_execution",
            "required": "Run qpls with --bootstrap-samples 999 --studentized-inner-samples 99 on the bounded validation fixture and commit the result artifact.",
            "note": "Minimum execution evidence proves the nested 999x99 path can complete; it is not coverage, performance, or publication qualification."
        }));
    }
    let envelope: AnalysisResult = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized minimum result JSON")?;
    let value = serde_json::to_value(&envelope.payload)?;
    let bootstrap = &value["bootstrap"];
    let studentized = &bootstrap["studentized"];
    let settings = &envelope.provenance.settings;
    let parameters = studentized
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let unavailable_parameters = studentized
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| !row["unavailable_reason"].is_null())
                .count()
        })
        .unwrap_or(0);
    let available_parameters = parameters.saturating_sub(unavailable_parameters);
    let passed = envelope.status == RunStatus::Completed
        && settings.bootstrap_samples >= 999
        && settings.studentized_inner_samples >= 99
        && settings.studentized_inner_samples % 2 == 1
        && bootstrap["usable_replicates"]
            .as_u64()
            .is_some_and(|value| value >= 900)
        && studentized["method_version"].as_str() == Some(STUDENTIZED_METHOD_VERSION)
        && studentized["inner_replicates"].as_u64()
            == Some(settings.studentized_inner_samples as u64)
        && studentized["failure"].is_null()
        && available_parameters > 0;
    Ok(json!({
        "passed": passed,
        "path": output,
        "bootstrap_samples": settings.bootstrap_samples,
        "studentized_inner_samples": settings.studentized_inner_samples,
        "usable_replicates": bootstrap["usable_replicates"],
        "studentized_method_version": studentized["method_version"],
        "studentized_failure": studentized["failure"],
        "parameter_count": parameters,
        "available_parameter_count": available_parameters,
        "unavailable_parameter_count": unavailable_parameters,
        "note": "Minimum 999x99 execution evidence on the bounded fixture. This does not replace preregistered coverage, sensitivity, worker-matrix, or performance qualification."
    }))
}

fn read_studentized_worker_matrix(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/studentized_worker_matrix.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_studentized_worker_matrix",
            "required": "Run npm run qpls:studentized:workers to prove the bounded 999x99 nested studentized bootstrap is invariant across workers 1, 2, and 4.",
            "note": "Worker-matrix evidence proves deterministic parallel execution and records bounded timing; it is not a full stress or coverage qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized worker-matrix JSON")?;
    let fixture = &report["fixture"];
    let runs = report
        .get("runs")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let comparisons = report
        .get("comparisons")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let workers: Vec<u64> = runs
        .iter()
        .filter_map(|run| run["workers"].as_u64())
        .collect();
    let detected_max_workers = fixture["detected_max_workers"].as_u64();
    let has_required_workers = [1_u64, 2, 4]
        .iter()
        .all(|required| workers.contains(required))
        && detected_max_workers.is_some_and(|maximum| workers.contains(&maximum));
    let complete_runs = runs
        .iter()
        .all(|run| run["status"].as_str() == Some("completed"));
    let settings_match = runs.iter().all(|run| {
        run["settings_workers"].as_u64() == run["workers"].as_u64()
            && run["usable_replicates"]
                .as_u64()
                .is_some_and(|value| value >= 900)
            && run["studentized_inner_replicates"].as_u64()
                == fixture["studentized_inner_samples"].as_u64()
            && run["studentized_failure"].is_null()
            && run["studentized_available_parameter_count"]
                .as_u64()
                .is_some_and(|value| value > 0)
    });
    let comparisons_match = comparisons.iter().all(|comparison| {
        comparison["payload_equal"].as_bool() == Some(true)
            && comparison["diagnostics_equal"].as_bool() == Some(true)
            && comparison["max_payload_abs_difference"].as_f64() == Some(0.0)
    });
    let passed = report["kind"].as_str() == Some("studentized_worker_matrix_v1")
        && report["passed"].as_bool() == Some(true)
        && fixture["bootstrap_samples"].as_u64() == Some(999)
        && fixture["studentized_inner_samples"].as_u64() == Some(99)
        && has_required_workers
        && complete_runs
        && settings_match
        && comparisons_match;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "workers": workers,
        "detected_max_workers": detected_max_workers,
        "bootstrap_samples": fixture["bootstrap_samples"],
        "studentized_inner_samples": fixture["studentized_inner_samples"],
        "comparisons": comparisons,
        "performance": report["performance"],
        "artifact_directory": report["artifact_directory"],
        "note": "Bounded 999x99 worker-matrix evidence with exact payload and diagnostics equality across workers 1, 2, 4, and the detected maximum worker count."
    }))
}

fn read_studentized_performance(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/studentized_performance.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_studentized_performance_benchmark",
            "required": "Run npm run qpls:studentized:performance to benchmark bounded minimum/default/outer-stress/maximum-inner and broader-model nested studentized plans.",
            "note": "Bounded benchmark evidence records runtime, throughput, peak working set, and compact persistence size; full stress qualification remains broader."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized performance benchmark JSON")?;
    let plans = report
        .get("plans")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required = [
        ("minimum_999x99", 999_u64, 99_u64),
        ("default_inner_999x199", 999_u64, 199_u64),
        ("outer_stress_1999x99", 1999_u64, 99_u64),
        ("maximum_inner_999x999", 999_u64, 999_u64),
        ("broader_corporate_999x99", 999_u64, 99_u64),
    ];
    let required_present = required.iter().all(|(name, bootstrap, inner)| {
        plans.iter().any(|plan| {
            plan["name"].as_str() == Some(*name)
                && plan["bootstrap_samples"].as_u64() == Some(*bootstrap)
                && plan["studentized_inner_samples"].as_u64() == Some(*inner)
                && plan["passed"].as_bool() == Some(true)
                && plan["elapsed_seconds"]
                    .as_f64()
                    .is_some_and(|value| value > 0.0)
                && plan["inner_fits_per_second"]
                    .as_f64()
                    .is_some_and(|value| value > 0.0)
                && plan["peak_working_set_bytes"]
                    .as_u64()
                    .is_some_and(|value| value > 0)
                && plan["studentized_available_parameter_count"]
                    .as_u64()
                    .is_some_and(|value| value > 0)
        })
    });
    let passed = report["kind"].as_str() == Some("studentized_performance_benchmark_v1")
        && report["passed"].as_bool() == Some(true)
        && required_present;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "fixture": report["fixture"],
        "plans": plans,
        "artifact_directory": report["artifact_directory"],
        "note": "Bounded studentized performance benchmark consumed by v0.4 qualifier, including compact and broader corporate model-shape smoke plans. Full release stress still requires documented hardware and full-scale broader model shapes."
    }))
}

fn read_studentized_release_stress(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/studentized_release_stress.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_studentized_release_stress_benchmark",
            "required": "Run npm run qpls:studentized:release-stress to benchmark maximum outer-plus-inner and broader corporate nested studentized plans.",
            "note": "Release-stress evidence records runtime, throughput, peak working set, and available studentized parameters for the maximum outer-plus-inner and broader corporate model-shape stress plans."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid studentized release-stress benchmark JSON")?;
    let plans = report
        .get("plans")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required = [
        ("maximum_outer_inner_1999x999", 1999_u64, 999_u64),
        ("broader_corporate_999x199", 999_u64, 199_u64),
    ];
    let required_present = required.iter().all(|(name, bootstrap, inner)| {
        plans.iter().any(|plan| {
            plan["name"].as_str() == Some(*name)
                && plan["bootstrap_samples"].as_u64() == Some(*bootstrap)
                && plan["studentized_inner_samples"].as_u64() == Some(*inner)
                && plan["passed"].as_bool() == Some(true)
                && plan["elapsed_seconds"]
                    .as_f64()
                    .is_some_and(|value| value > 0.0)
                && plan["inner_fits_per_second"]
                    .as_f64()
                    .is_some_and(|value| value > 0.0)
                && plan["peak_working_set_bytes"]
                    .as_u64()
                    .is_some_and(|value| value > 0)
                && plan["studentized_available_parameter_count"]
                    .as_u64()
                    .is_some_and(|value| value > 0)
        })
    });
    let passed = report["kind"].as_str() == Some("studentized_performance_benchmark_v1")
        && report["profile"].as_str() == Some("release-stress")
        && report["passed"].as_bool() == Some(true)
        && required_present;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "profile": report["profile"],
        "fixture": report["fixture"],
        "plans": plans,
        "artifact_directory": report["artifact_directory"],
        "note": "Release-stress studentized benchmark consumed by v0.4 qualifier, including maximum outer-plus-inner and broader corporate model-shape stress plans."
    }))
}

fn read_pls_bootstrap_external_reference(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/pls_bootstrap_external_reference.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_pls_bootstrap_external_reference",
            "required": "Run npm run qpls:bootstrap:external to compare fixed-resample QuickPLS bootstrap estimates and aggregate summaries against cSEM.",
            "note": "This is a PLS-integrated external-reference fixture on matched resamples; it is not stochastic coverage qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid PLS bootstrap external-reference JSON")?;
    let accepted = report
        .get("accepted_replicates")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let summary_comparisons = report
        .get("summary_comparisons")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let variants = report
        .pointer("/fixture/variants")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let parameters = report
        .pointer("/fixture/parameters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required_variants = ["PATH_MODE_A", "MODE_B", "FACTOR", "PCA"];
    let has_required_variants = required_variants.iter().all(|variant| {
        variants
            .iter()
            .any(|actual| actual.as_str() == Some(*variant))
    });
    let summary_shape_complete = has_required_variants
        && parameters.len() >= 9
        && summary_comparisons.len() >= required_variants.len() * 9;
    let all_summaries_pass = summary_comparisons
        .iter()
        .all(|row| row["passed"].as_bool() == Some(true));
    let passed = report["kind"].as_str() == Some("pls_bootstrap_external_reference_v1")
        && report["passed"].as_bool() == Some(true)
        && accepted >= 12
        && report["max_replicate_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && report["max_summary_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && summary_shape_complete
        && all_summaries_pass;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "reference": report["reference"],
        "fixture": report["fixture"],
        "accepted_replicates": accepted,
        "variants": variants,
        "parameter_count_per_variant": parameters.len(),
        "summary_comparison_count": summary_comparisons.len(),
        "skipped_candidate_count": report
            .get("skipped_candidates")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "max_replicate_abs_diff": report["max_replicate_abs_diff"],
        "max_summary_abs_diff": report["max_summary_abs_diff"],
        "summary_comparisons": summary_comparisons,
        "note": report["note"]
    }))
}

fn read_pls_bootstrap_corporate_csem_reference(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/pls_bootstrap_corporate_csem_reference.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_pls_bootstrap_corporate_csem_reference",
            "required": "Run python validation/pls_bootstrap_corporate_csem_reference.py to compare fixed-resample QuickPLS bootstrap estimates and aggregate summaries against cSEM on the corporate-reputation model.",
            "note": "This is a broader PLS-integrated external-reference fixture on matched resamples; it is not stochastic coverage qualification."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid corporate PLS bootstrap cSEM external-reference JSON")?;
    let accepted = report
        .get("accepted_replicates")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let summary_comparisons = report
        .get("summary_comparisons")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let parameters = report
        .pointer("/fixture/parameters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let model_shape = &report["fixture"]["model_shape"];
    let expected_shape = model_shape["constructs"].as_u64() == Some(4)
        && model_shape["indicators"].as_u64() == Some(9)
        && model_shape["paths"].as_u64() == Some(3);
    let all_summaries_pass = summary_comparisons
        .iter()
        .all(|row| row["passed"].as_bool() == Some(true));
    let passed = report["kind"].as_str() == Some("pls_bootstrap_corporate_csem_reference_v1")
        && report["passed"].as_bool() == Some(true)
        && accepted >= 8
        && report["fixture"]["variant"].as_str() == Some("CORPORATE_PATH_MODE_A")
        && parameters.len() >= 21
        && summary_comparisons.len() >= 21
        && expected_shape
        && report["max_replicate_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && report["max_summary_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && all_summaries_pass;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "reference": report["reference"],
        "fixture": report["fixture"],
        "accepted_replicates": accepted,
        "parameter_count": parameters.len(),
        "summary_comparison_count": summary_comparisons.len(),
        "skipped_candidate_count": report
            .get("skipped_candidates")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "max_replicate_abs_diff": report["max_replicate_abs_diff"],
        "max_summary_abs_diff": report["max_summary_abs_diff"],
        "summary_comparisons": summary_comparisons,
        "note": report["note"]
    }))
}

fn read_pls_bootstrap_plspm_external_reference(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/pls_bootstrap_plspm_external_reference.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_pls_bootstrap_plspm_external_reference",
            "required": "Run npm run qpls:bootstrap:plspm to compare fixed-resample QuickPLS bootstrap estimates and aggregate summaries against python-plspm.",
            "note": "This is a second PLS-integrated external-reference family on matched resamples; python-plspm weights are excluded because its normalization convention differs."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid PLS bootstrap python-plspm external-reference JSON")?;
    let accepted = report
        .get("accepted_replicates")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let summary_comparisons = report
        .get("summary_comparisons")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let variants = report
        .pointer("/fixture/variants")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let parameters = report
        .pointer("/fixture/parameters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let required_variants = ["PATH_MODE_A", "MODE_B", "FACTOR"];
    let has_required_variants = required_variants.iter().all(|variant| {
        variants
            .iter()
            .any(|actual| actual.as_str() == Some(*variant))
    });
    let summary_shape_complete = has_required_variants
        && parameters.len() >= 5
        && summary_comparisons.len() >= required_variants.len() * 5;
    let all_summaries_pass = summary_comparisons
        .iter()
        .all(|row| row["passed"].as_bool() == Some(true));
    let passed = report["kind"].as_str() == Some("pls_bootstrap_plspm_external_reference_v1")
        && report["passed"].as_bool() == Some(true)
        && accepted >= 12
        && report["max_replicate_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && report["max_summary_abs_diff"]
            .as_f64()
            .is_some_and(|value| value <= 1.0e-6)
        && summary_shape_complete
        && all_summaries_pass;
    Ok(json!({
        "passed": passed,
        "path": output,
        "kind": report["kind"],
        "reference": report["reference"],
        "fixture": report["fixture"],
        "accepted_replicates": accepted,
        "variants": variants,
        "parameter_count_per_variant": parameters.len(),
        "summary_comparison_count": summary_comparisons.len(),
        "skipped_candidate_count": report
            .get("skipped_candidates")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "max_replicate_abs_diff": report["max_replicate_abs_diff"],
        "max_summary_abs_diff": report["max_summary_abs_diff"],
        "summary_comparisons": summary_comparisons,
        "note": report["note"]
    }))
}

fn run_or_read_monte_carlo_harness(
    root: &Path,
    refresh: bool,
    mode: &str,
    relative_output: &str,
    minimum_simulations: u64,
    minimum_bootstrap_replicates: u64,
    note: &str,
) -> Result<serde_json::Value> {
    let output = root.join(relative_output);
    if refresh {
        let output_argument = output
            .to_str()
            .with_context(|| format!("{mode} Monte Carlo output path is not UTF-8"))?;
        let status = ProcessCommand::new("cargo")
            .current_dir(root)
            .args([
                "run",
                "--release",
                "--manifest-path",
                "validation/monte_carlo/Cargo.toml",
                "--",
                "--mode",
                mode,
                "--output",
                output_argument,
            ])
            .status()
            .context("failed to start Monte Carlo harness")?;
        if !status.success() {
            bail!("Monte Carlo harness failed with status {status}");
        }
    }
    if !output.exists() {
        return Ok(json!({
            "usable": false,
            "refreshed": refresh,
            "path": output,
            "reason": format!("missing_{mode}_monte_carlo_report"),
            "required": format!("Run the {mode} Monte Carlo harness and commit its JSON report."),
            "note": note
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .with_context(|| format!("invalid {mode} Monte Carlo JSON"))?;
    let evaluated = report
        .pointer("/qualification/evaluated")
        .and_then(serde_json::Value::as_bool);
    let configuration = &report["configuration"];
    let enough_requested = configuration["simulations_per_scenario"].as_u64()
        >= Some(minimum_simulations)
        && configuration["bootstrap_replicates"].as_u64() >= Some(minimum_bootstrap_replicates);
    let expected_scenarios: &[&str] = if mode == "sensitivity" {
        &[
            "coverage_beta_0_35",
            "null_beta_0",
            "heavy_tail_coverage_beta_0_35",
            "heavy_tail_null_beta_0",
        ]
    } else {
        &["coverage_beta_0_35", "null_beta_0"]
    };
    let scenarios = report
        .get("scenarios")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scenarios_ok = expected_scenarios.iter().all(|name| {
        scenarios.iter().any(|scenario| {
            scenario["name"].as_str() == Some(*name)
                && scenario["failed_simulations"].as_u64() == Some(0)
        })
    });
    let usable = report["mode"].as_str() == Some(mode)
        && evaluated == Some(false)
        && enough_requested
        && scenarios_ok;
    Ok(json!({
        "usable": usable,
        "refreshed": refresh,
        "path": output,
        "mode": report["mode"],
        "harness_version": report["harness_version"],
        "engine_versions": report["engine_versions"],
        "configuration": report["configuration"],
        "scenario_count": scenarios.len(),
        "scenarios": scenarios.iter().map(|scenario| json!({
            "name": scenario["name"],
            "error_distribution": scenario["error_distribution"],
            "completed_simulations": scenario["completed_simulations"],
            "failed_simulations": scenario["failed_simulations"],
            "percentile": scenario["percentile"],
            "bca": scenario["bca"],
            "bias": scenario["bias"]
        })).collect::<Vec<_>>(),
        "qualification_evaluated": evaluated,
        "note": note
    }))
}

fn read_full_monte_carlo_qualification(root: &Path) -> Result<serde_json::Value> {
    let output = root.join("validation/results/monte_carlo_qualification.json");
    if !output.exists() {
        return Ok(json!({
            "passed": false,
            "path": output,
            "reason": "missing_full_qualification_report",
            "required": "Run the preregistered qualification mode and commit a report with qualification.evaluated=true and qualification.passed=true."
        }));
    }
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(&output).with_context(|| format!("cannot read {}", output.display()))?,
    )
    .context("invalid full Monte Carlo qualification JSON")?;
    let evaluated = report
        .pointer("/qualification/evaluated")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let passed = report
        .pointer("/qualification/passed")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    Ok(json!({
        "passed": evaluated && passed,
        "path": output,
        "qualification_evaluated": evaluated,
        "qualification_passed": passed,
        "mode": report["mode"],
        "harness_version": report["harness_version"],
        "engine_versions": report["engine_versions"]
    }))
}

fn create_demo_project(project_path: Option<&Path>, expected_path: Option<&Path>) -> Result<()> {
    let root = repository_root()?;
    let project_path = project_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/demo/quickpls_v04_demo.qpls"));
    let expected_path = expected_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/demo/quickpls_v04_demo.expected.json"));
    let (project, expected) = build_demo_project(&root)?;
    save_project(&project_path, &project)
        .with_context(|| format!("cannot save demo project {}", project_path.display()))?;
    if let Some(parent) = expected_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&expected_path, serde_json::to_vec_pretty(&expected)?)
        .with_context(|| format!("cannot write expected result {}", expected_path.display()))?;
    println!(
        "wrote demo project {} and expected result {}",
        project_path.display(),
        expected_path.display()
    );
    Ok(())
}

fn validate_demo_project(
    project_path: Option<&Path>,
    expected_path: Option<&Path>,
    output_path: Option<&Path>,
) -> Result<()> {
    let root = repository_root()?;
    let project_path = project_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/demo/quickpls_v04_demo.qpls"));
    let expected_path = expected_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/demo/quickpls_v04_demo.expected.json"));
    let output_path = output_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("validation/demo/quickpls_v04_demo.validation.json"));
    let (project, recovery) = load_project_with_autosave(&project_path)
        .with_context(|| format!("cannot load demo project {}", project_path.display()))?;
    if recovery.is_some() {
        bail!("demo validation refuses autosave recovery input");
    }
    let expected: serde_json::Value = serde_json::from_slice(
        &fs::read(&expected_path)
            .with_context(|| format!("cannot read expected result {}", expected_path.display()))?,
    )
    .context("invalid expected demo JSON")?;
    if project.datasets.len() != 1 || project.recipes.len() != 1 {
        bail!("demo project must contain exactly one dataset and one recipe");
    }
    let actual = run_demo_recipe(&project.datasets[0], &project.recipes[0])?;
    let actual_canonical = canonical_demo_result(&actual)?;
    let expected_canonical = expected
        .get("canonical_result")
        .context("expected demo JSON is missing canonical_result")?;
    let comparison = compare_json_with_tolerance(expected_canonical, &actual_canonical, 1e-12);
    let matches_expected = comparison.matches;
    let report = json!({
        "schema_version": 1,
        "demo_id": "quickpls_v04_demo",
        "project": project_path,
        "expected": expected_path,
        "validated_at": Utc::now(),
        "matches_expected": matches_expected,
        "comparison": {
            "numeric_tolerance": comparison.numeric_tolerance,
            "max_abs_numeric_difference": comparison.max_abs_numeric_difference,
            "first_difference": comparison.first_difference
        },
        "actual": actual_canonical,
        "expected_engine_versions": expected.get("engine_versions"),
        "actual_engine_versions": demo_engine_versions(),
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("cannot write demo validation {}", output_path.display()))?;
    println!(
        "wrote demo validation {} | matches_expected={}",
        output_path.display(),
        matches_expected
    );
    if !matches_expected {
        bail!("demo project output differs from expected result");
    }
    Ok(())
}

fn build_demo_project(root: &Path) -> Result<(Project, serde_json::Value)> {
    let dataset_path = root.join("validation/fixtures/corporate_reputation.csv");
    let dataset = import_path(&dataset_path, &ImportOptions::default())
        .with_context(|| format!("cannot import {}", dataset_path.display()))?;
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
            .unwrap()
            .with_timezone(&Utc),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: model.clone(),
        settings,
        // QuickPLS supports the historical combined output contract: bootstrap
        // is the primary typed workflow and permutation is an added result.
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
    let mut project = Project::new("Corporate Reputation Sample");
    project.datasets.push(dataset);
    project.models.push(model);
    project.recipes.push(recipe);
    project.results.push(result.clone());
    project.layouts.insert(
        "quickpls_v04_demo_layout".into(),
        json!({
            "constructs": {
                "comp": {"x": 120, "y": 120},
                "like": {"x": 410, "y": 120},
                "satisfaction": {"x": 700, "y": 120},
                "loyalty": {"x": 990, "y": 120}
            },
            "purpose": "deterministic validation demo layout"
        }),
    );
    let expected = json!({
        "schema_version": 1,
        "demo_id": "quickpls_v04_demo",
        "dataset": "validation/fixtures/corporate_reputation.csv",
        "engine_versions": demo_engine_versions(),
        "canonical_result": canonical_demo_result(&result)?,
        "note": "This expected result is a regression artifact for the current experimental v0.4 implementation, not publication validation evidence."
    });
    Ok((project, expected))
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

fn run_demo_recipe(
    dataset: &qpls_data::Dataset,
    recipe: &AnalysisRecipe,
) -> Result<AnalysisResult> {
    let started_at = Utc::now();
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .context("demo recipe validation failed")?;
    let base_recipe = execution
        .without_outer_resampling()
        .context("demo base recipe derivation failed")?;
    let estimation =
        qpls_estimation::estimate_pls_validated_with_control(dataset, &base_recipe, |_| true)
            .context("demo PLS estimation failed")?;
    let assessment = qpls_assessment::assess_pls_validated_with_control(
        dataset,
        &base_recipe,
        &estimation,
        |_| true,
    )
    .context("demo PLS assessment failed")?;
    let bootstrap = bootstrap_pls(
        dataset,
        recipe,
        &estimation,
        recipe.settings.workers,
        || false,
        |_| {},
    )
    .context("demo bootstrap failed")?;
    let permutation = permutation_pls(
        dataset,
        recipe,
        &estimation,
        recipe.settings.workers,
        || false,
        |_| {},
    )
    .context("demo permutation failed")?;
    Ok(AnalysisResult::completed_pls_inference(
        recipe,
        format!(
            "{}+{}+{}+{}+{}+{}",
            qpls_estimation::PLS_METHOD_VERSION,
            qpls_estimation::PLS_MEDIATION_METHOD_VERSION,
            ASSESSMENT_METHOD_VERSION,
            RESAMPLING_METHOD_VERSION,
            HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION,
            PERMUTATION_METHOD_VERSION
        ),
        started_at,
        serde_json::to_value(estimation)?,
        serde_json::to_value(assessment)?,
        Some(serde_json::to_value(bootstrap)?),
        Some(serde_json::to_value(permutation)?),
        Vec::new(),
    ))
}

fn canonical_demo_result(result: &AnalysisResult) -> Result<serde_json::Value> {
    Ok(json!({
        "method": result.provenance.method,
        "method_version": result.provenance.method_version,
        "seed": result.provenance.seed,
        "settings": result.provenance.settings,
        "diagnostics": result.diagnostics,
        "payload": result.payload
    }))
}

fn demo_engine_versions() -> serde_json::Value {
    json!({
        "pls": qpls_estimation::PLS_METHOD_VERSION,
        "pls_mediation": qpls_estimation::PLS_MEDIATION_METHOD_VERSION,
        "assessment": ASSESSMENT_METHOD_VERSION,
        "resampling": RESAMPLING_METHOD_VERSION,
        "htmt_bootstrap_inference": HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION,
        "permutation": PERMUTATION_METHOD_VERSION
    })
}

struct JsonComparison {
    matches: bool,
    numeric_tolerance: f64,
    max_abs_numeric_difference: f64,
    first_difference: Option<String>,
}

fn compare_json_with_tolerance(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    numeric_tolerance: f64,
) -> JsonComparison {
    let mut comparison = JsonComparison {
        matches: true,
        numeric_tolerance,
        max_abs_numeric_difference: 0.0,
        first_difference: None,
    };
    compare_json_at(expected, actual, "$", &mut comparison);
    comparison
}

fn compare_json_at(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
    comparison: &mut JsonComparison,
) {
    if comparison.first_difference.is_some() {
        return;
    }
    match (expected, actual) {
        (serde_json::Value::Number(left), serde_json::Value::Number(right)) => {
            let Some(left) = left.as_f64() else {
                comparison.first_difference = Some(format!("{path}: expected non-f64 number"));
                comparison.matches = false;
                return;
            };
            let Some(right) = right.as_f64() else {
                comparison.first_difference = Some(format!("{path}: actual non-f64 number"));
                comparison.matches = false;
                return;
            };
            let difference = (left - right).abs();
            comparison.max_abs_numeric_difference =
                comparison.max_abs_numeric_difference.max(difference);
            if difference > comparison.numeric_tolerance {
                comparison.first_difference = Some(format!(
                    "{path}: expected {left}, actual {right}, diff {difference}"
                ));
                comparison.matches = false;
            }
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if left.len() != right.len() {
                comparison.first_difference = Some(format!(
                    "{path}: expected array length {}, actual {}",
                    left.len(),
                    right.len()
                ));
                comparison.matches = false;
                return;
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                compare_json_at(left, right, &format!("{path}[{index}]"), comparison);
            }
        }
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            if left.len() != right.len() {
                comparison.first_difference = Some(format!(
                    "{path}: expected object field count {}, actual {}",
                    left.len(),
                    right.len()
                ));
                comparison.matches = false;
                return;
            }
            for (key, left) in left {
                let Some(right) = right.get(key) else {
                    comparison.first_difference =
                        Some(format!("{path}.{key}: missing actual field"));
                    comparison.matches = false;
                    return;
                };
                compare_json_at(left, right, &format!("{path}.{key}"), comparison);
            }
        }
        _ if expected == actual => {}
        _ => {
            comparison.first_difference =
                Some(format!("{path}: expected {expected}, actual {actual}"));
            comparison.matches = false;
        }
    }
}

fn run_analysis(
    input: &Path,
    data_path: Option<&Path>,
    recipe_id: Option<&str>,
    output: &Path,
    allow_experimental: bool,
    allow_internal_qualification: bool,
    allow_v04_inference_qualification: bool,
    bootstrap_samples: Option<u32>,
    studentized_inner_samples: Option<u32>,
    permutation_samples: Option<u32>,
    workers: Option<usize>,
) -> Result<()> {
    let (dataset, mut recipe) = if input
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("qpls"))
    {
        let (project, _) = load_project_with_autosave(input)
            .with_context(|| format!("invalid project {}", input.display()))?;
        require_executable_project(&project)?;
        let recipe = if let Some(recipe_id) = recipe_id {
            project
                .recipes
                .iter()
                .find(|recipe| recipe.id.to_string() == recipe_id)
                .cloned()
                .with_context(|| format!("project has no recipe {recipe_id}"))?
        } else {
            if project.recipes.len() != 1 {
                bail!(
                    "project contains {} recipes; select one with --recipe-id",
                    project.recipes.len()
                );
            }
            project.recipes[0].clone()
        };
        let dataset = project
            .datasets
            .into_iter()
            .find(|dataset| dataset.fingerprint.0 == recipe.dataset_fingerprint)
            .context("project does not contain the dataset referenced by the selected recipe")?;
        (dataset, recipe)
    } else {
        let recipe: AnalysisRecipe = serde_json::from_slice(
            &fs::read(input).with_context(|| format!("cannot read {}", input.display()))?,
        )
        .context("invalid analysis recipe JSON")?;
        let data_path = data_path.context("--data is required when running a recipe JSON file")?;
        let dataset = import_path(data_path, &ImportOptions::default())
            .with_context(|| format!("cannot import {}", data_path.display()))?;
        (dataset, recipe)
    };
    if let Some(bootstrap_samples) = bootstrap_samples {
        recipe.settings.bootstrap_samples = bootstrap_samples;
    }
    if let Some(studentized_inner_samples) = studentized_inner_samples {
        recipe.settings.studentized_inner_samples = studentized_inner_samples;
    }
    if let Some(permutation_samples) = permutation_samples {
        recipe.settings.permutation_samples = permutation_samples;
    }
    if let Some(workers) = workers {
        recipe.settings.workers = workers;
    }
    if (bootstrap_samples.is_some()
        || studentized_inner_samples.is_some()
        || permutation_samples.is_some())
        && recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION
        && recipe.settings.method == AnalysisMethod::PlsPm
    {
        if let Some(MethodConfig::PlsPosthocTechnicalMinimumSampleSize(config)) =
            recipe.method_config.as_mut()
        {
            if recipe.settings.bootstrap_samples > 0 {
                config.base_analysis =
                    qpls_core::PlsPosthocTechnicalMinimumSampleSizeBaseAnalysisV2::PlsBootstrap;
                config.inference = qpls_core::PlsPosthocTechnicalMinimumSampleSizeInferenceV2::CaseBootstrapNormalReferenceTwoSided;
            } else {
                config.base_analysis =
                    qpls_core::PlsPosthocTechnicalMinimumSampleSizeBaseAnalysisV2::PlsAlgorithm;
                config.inference =
                    qpls_core::PlsPosthocTechnicalMinimumSampleSizeInferenceV2::PointEstimateOnly;
            }
        } else {
            recipe.method_config = Some(MethodConfig::default_for_settings(&recipe.settings));
        }
    }
    let issues = validate_recipe(&recipe);
    if let Some(issue) = issues
        .iter()
        .find(|issue| issue.severity == Severity::Error)
    {
        bail!("{}: {}", issue.code, issue.message);
    }
    if recipe.dataset_fingerprint != dataset.fingerprint.0 {
        bail!("recipe dataset fingerprint does not match the imported dataset");
    }
    if allow_v04_inference_qualification {
        require_v04_inference_qualification_scope(&recipe)?;
    } else {
        require_cli_capability_availability(
            &recipe,
            allow_experimental,
            allow_internal_qualification,
        )?;
    }
    let envelope = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {})
        .map_err(anyhow::Error::from)?;
    fs::write(output, serde_json::to_vec_pretty(&envelope)?)
        .with_context(|| format!("cannot write {}", output.display()))?;
    println!("wrote analysis result {}", output.display());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequiredCliCapabilityCellV2 {
    capability_id: String,
    cell_id: String,
    capability_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliCapabilityMappingFailure {
    MissingMethodConfig,
    MethodConfigMismatch,
    UnmappedMethodConfig,
    EmptyMapping,
}

impl std::fmt::Display for CliCapabilityMappingFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MissingMethodConfig => "method_config is missing",
            Self::MethodConfigMismatch => "method and method_config are incompatible",
            Self::UnmappedMethodConfig => "method/config semantics have no exact registry cell",
            Self::EmptyMapping => "the exact registry mapping is empty",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnmappedCliCapabilityError {
    method: AnalysisMethod,
    config_kind: &'static str,
    failure: CliCapabilityMappingFailure,
}

impl std::fmt::Display for UnmappedCliCapabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "analysis method {} with method_config kind `{}` has no exact Capability Registry V2 execution mapping: {}",
            self.method, self.config_kind, self.failure,
        )
    }
}

impl std::error::Error for UnmappedCliCapabilityError {}

fn unmapped_cli_capability_error(
    method: AnalysisMethod,
    config_kind: &'static str,
    failure: CliCapabilityMappingFailure,
) -> anyhow::Error {
    UnmappedCliCapabilityError {
        method,
        config_kind,
        failure,
    }
    .into()
}

fn push_required_cli_capability_cell(
    required_cells: &mut Vec<RequiredCliCapabilityCellV2>,
    capability_id: &str,
    cell_id: &str,
    capability_version: &str,
) {
    if required_cells.iter().any(|required| {
        required.capability_id == capability_id
            && required.cell_id == cell_id
            && required.capability_version == capability_version
    }) {
        return;
    }
    required_cells.push(RequiredCliCapabilityCellV2 {
        capability_id: capability_id.into(),
        cell_id: cell_id.into(),
        capability_version: capability_version.into(),
    });
}

fn push_pls_recipe_add_on_cells(
    recipe: &AnalysisRecipe,
    required_cells: &mut Vec<RequiredCliCapabilityCellV2>,
) {
    if recipe
        .metadata
        .get(PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR)
        .is_some_and(|value| value == "true")
    {
        push_required_cli_capability_cell(
            required_cells,
            "smartpls.model_fit",
            "qpls3.assessment.model_fit",
            PLS_MODEL_FIT_METHOD_VERSION,
        );
    }
    if !recipe.model.interactions.is_empty() {
        push_required_cli_capability_cell(
            required_cells,
            "smartpls.moderation",
            "qpls3.pls.moderation",
            "pls_two_stage_moderation_v1",
        );
    }
    if !recipe.model.higher_order_constructs.is_empty() {
        push_required_cli_capability_cell(
            required_cells,
            "smartpls.higher_order_models",
            "qpls3.pls.higher_order_two_stage",
            "pls_pm_v1",
        );
    }
}

fn push_pls_algorithm_base_cell(required_cells: &mut Vec<RequiredCliCapabilityCellV2>) {
    push_required_cli_capability_cell(
        required_cells,
        "smartpls.pls_algorithm",
        "qpls3.pls.algorithm",
        "pls_pm_v1",
    );
}

const ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1: [&str; 2] = ["primary", "base"];

fn push_generated_established_cli_capability_cells(
    required_cells: &mut Vec<RequiredCliCapabilityCellV2>,
    method: AnalysisMethod,
    config_kind: &'static str,
) -> Result<()> {
    let contract =
        qpls_core::generated::established_method_contract_v1(method.as_str(), config_kind)
            .ok_or_else(|| {
                unmapped_cli_capability_error(
                    method,
                    config_kind,
                    CliCapabilityMappingFailure::UnmappedMethodConfig,
                )
            })?;
    // Preserve the CLI's established first-failure and output-byte behavior:
    // method-specific primary cells are checked before their mandatory base.
    for role in ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1 {
        for requirement in contract
            .capability_requirements
            .iter()
            .filter(|requirement| requirement.role == role)
        {
            push_required_cli_capability_cell(
                required_cells,
                requirement.capability_id,
                requirement.cell_id,
                requirement.capability_version,
            );
        }
    }
    Ok(())
}

fn required_cli_capability_cells(
    recipe: &AnalysisRecipe,
) -> Result<Vec<RequiredCliCapabilityCellV2>> {
    let method = recipe.settings.method;
    let Some(config) = recipe.method_config.as_ref() else {
        return Err(unmapped_cli_capability_error(
            method,
            "<missing>",
            CliCapabilityMappingFailure::MissingMethodConfig,
        ));
    };
    if !config.supports_method(method) {
        return Err(unmapped_cli_capability_error(
            method,
            config.kind(),
            CliCapabilityMappingFailure::MethodConfigMismatch,
        ));
    }

    let mut required_cells = Vec::new();
    match (method, config) {
        (AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm) => {
            push_pls_recipe_add_on_cells(recipe, &mut required_cells);
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::PlsPm, MethodConfig::PlsBootstrap) => {
            push_pls_recipe_add_on_cells(recipe, &mut required_cells);
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.pls_bootstrapping",
                "qpls3.inference.bootstrap",
                "indexed_resampling_v4",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::PlsPm, MethodConfig::PlsPermutation) => {
            push_pls_recipe_add_on_cells(recipe, &mut required_cells);
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.permutation",
                "qpls3.inference.structural_path_randomization",
                "freedman_lane_permutation_v1",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::PlsPm, MethodConfig::PlsPosthocTechnicalMinimumSampleSize(posthoc)) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                &posthoc.capability_cell.capability_id,
                &posthoc.capability_cell.cell_id,
                &posthoc.capability_cell.capability_version,
            );
            push_pls_recipe_add_on_cells(recipe, &mut required_cells);
            if posthoc.base_analysis
                == qpls_core::PlsPosthocTechnicalMinimumSampleSizeBaseAnalysisV2::PlsBootstrap
            {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.pls_bootstrapping",
                    "qpls3.inference.bootstrap",
                    "indexed_resampling_v4",
                );
            }
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::PlsSampleSizePower, MethodConfig::PlsSampleSizePower(_)) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.pls_power_analysis",
                "qpls3.pls.sample_size_power",
                PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
            );
        }
        (AnalysisMethod::Plsc, MethodConfig::Plsc | MethodConfig::PlscPermutation { .. }) => {
            if recipe.settings.bootstrap_samples > 0 {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.consistent_bootstrapping",
                    "qpls3.inference.consistent_bootstrap",
                    PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION,
                );
            }
            if recipe.settings.permutation_samples > 0
                || matches!(config, MethodConfig::PlscPermutation { .. })
            {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.consistent_permutation",
                    "qpls3.inference.consistent_permutation",
                    PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION,
                );
            }
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.plsc",
                "qpls3.pls.consistent",
                "plsc_v2",
            );
        }
        (AnalysisMethod::Wpls, MethodConfig::Wpls) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.wpls",
                "qpls3.pls.weighted",
                "wpls_case_weighted_v1",
            );
        }
        (AnalysisMethod::Cca, MethodConfig::Cca) => {
            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config.kind(),
            )?;
        }
        (AnalysisMethod::CtaPls, MethodConfig::CtaPls) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.cta_pls",
                "qpls3.assessment.cta_pls",
                "cta_pls_tetrad_v1",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::Endogeneity, MethodConfig::Endogeneity) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.endogeneity_gaussian_copulas",
                "qpls3.pls.gaussian_copula_endogeneity",
                "gaussian_copula_endogeneity_v1",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::NonlinearEffects, MethodConfig::NonlinearEffects) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.nonlinear_relationships",
                "qpls3.pls.nonlinear_quadratic",
                "pls_quadratic_nonlinear_effects_v1",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::ModeratedMediation, MethodConfig::ModeratedMediation) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.moderation",
                "qpls3.pls.moderation",
                "pls_two_stage_moderation_v1",
            );
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.mediation",
                "qpls3.pls.mediation",
                "pls_mediation_v1",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::Predict, MethodConfig::Predict { pls_pos, fimix }) => {
            if pls_pos.is_some() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.pls_pos",
                    "qpls3.segmentation.pls_pos",
                    "pls_pos_v1",
                );
            }
            if fimix.is_some() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.fimix_pls",
                    "qpls3.segmentation.fimix_pls",
                    "fimix_pls_v1",
                );
            }
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.plspredict",
                "qpls3.prediction.plspredict_cvpat",
                "plspredict_indicator_v2",
            );
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.cvpat",
                "qpls3.prediction.plspredict_cvpat",
                "plspredict_indicator_v2",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::Mga, MethodConfig::Mga { methods, .. }) => {
            if methods.is_empty() {
                return Err(unmapped_cli_capability_error(
                    method,
                    config.kind(),
                    CliCapabilityMappingFailure::UnmappedMethodConfig,
                ));
            }
            for group_method in methods {
                match group_method {
                    qpls_core::GroupAnalysisMethod::Micom => {
                        push_required_cli_capability_cell(
                            &mut required_cells,
                            "smartpls.micom",
                            "qpls3.groups.micom_permutation_mga",
                            "pls_mga_permutation_v4",
                        );
                    }
                    qpls_core::GroupAnalysisMethod::MgaPermutation => {
                        push_required_cli_capability_cell(
                            &mut required_cells,
                            "smartpls.mga",
                            "qpls3.groups.micom_permutation_mga",
                            "pls_mga_permutation_v4",
                        );
                    }
                }
            }
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::Mga, MethodConfig::Micom { .. }) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.micom",
                "qpls3.groups.micom_permutation_mga",
                "pls_mga_permutation_v4",
            );
            push_pls_algorithm_base_cell(&mut required_cells);
        }
        (AnalysisMethod::Ipma, MethodConfig::Ipma { .. }) => {
            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config.kind(),
            )?;
        }
        (
            AnalysisMethod::Cbsem,
            MethodConfig::Cbsem {
                model_type,
                estimator,
                input,
                mean_structure,
                bootstrap_samples,
                bootstrap_v2,
                group_column,
                invariance_steps,
            },
        ) => {
            if *estimator != qpls_core::CbsemEstimator::Ml
                || *input != qpls_core::CbsemInput::Raw
                || *mean_structure
                || (*bootstrap_samples > 0 && bootstrap_v2.is_none())
            {
                return Err(unmapped_cli_capability_error(
                    method,
                    config.kind(),
                    CliCapabilityMappingFailure::UnmappedMethodConfig,
                ));
            }
            if bootstrap_v2.is_some() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.cbsem_bootstrapping",
                    "qpls3.cbsem.bootstrap",
                    "cbsem_bootstrap_v2",
                );
            }
            if !recipe.model.interactions.is_empty() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.cbsem_moderator",
                    "qpls3.cbsem.moderator",
                    "cbsem_moderator_v1",
                );
            }
            if !invariance_steps.is_empty() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.cbsem_measurement_invariance",
                    "qpls3.cbsem.measurement_invariance",
                    "cbsem_invariance_v2",
                );
            }
            if group_column.is_some() {
                push_required_cli_capability_cell(
                    &mut required_cells,
                    "smartpls.cbsem_mga",
                    "qpls3.cbsem.multigroup",
                    "cbsem_multigroup_v2",
                );
            }
            let capability_id = match model_type {
                qpls_core::CbsemModelType::Cfa => "smartpls.cfa",
                qpls_core::CbsemModelType::Sem => "smartpls.cbsem",
            };
            push_required_cli_capability_cell(
                &mut required_cells,
                capability_id,
                "qpls3.cbsem.ml",
                "cbsem_ml_v1",
            );
        }
        (AnalysisMethod::Pca, MethodConfig::Pca { .. }) => {
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.pca_core",
                "qpls3.standalone.pca",
                "pca_v1",
            );
            push_required_cli_capability_cell(
                &mut required_cells,
                "smartpls.pca_cbsem",
                "qpls3.standalone.pca",
                "pca_v1",
            );
        }
        (AnalysisMethod::Gsca, MethodConfig::Gsca) => {
            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config.kind(),
            )?;
        }
        (
            AnalysisMethod::Regression,
            MethodConfig::Regression {
                model, bootstrap, ..
            },
        ) => {
            if bootstrap.is_some() {
                match model {
                    qpls_core::RegressionModelConfig::Process { .. } => {
                        push_required_cli_capability_cell(
                            &mut required_cells,
                            "smartpls.process_bootstrapping",
                            "qpls3.standalone.process",
                            "regression_process_v2",
                        );
                    }
                    qpls_core::RegressionModelConfig::Ols { .. }
                    | qpls_core::RegressionModelConfig::Logistic => {
                        push_required_cli_capability_cell(
                            &mut required_cells,
                            "smartpls.regression_bootstrapping",
                            "qpls3.standalone.regression_bootstrap",
                            "regression_bootstrap_v1",
                        );
                    }
                }
            }
            let (capability_id, cell_id, capability_version) = match model {
                qpls_core::RegressionModelConfig::Ols { .. } => (
                    "smartpls.regression",
                    "qpls3.standalone.ols",
                    "regression_ols_v1",
                ),
                qpls_core::RegressionModelConfig::Logistic => (
                    "smartpls.logistic_regression",
                    "qpls3.standalone.logistic",
                    "regression_logistic_v2",
                ),
                qpls_core::RegressionModelConfig::Process { .. } => (
                    "smartpls.process",
                    "qpls3.standalone.process",
                    "regression_process_v2",
                ),
            };
            push_required_cli_capability_cell(
                &mut required_cells,
                capability_id,
                cell_id,
                capability_version,
            );
        }
        (AnalysisMethod::Nca, MethodConfig::Nca { .. }) => {
            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config.kind(),
            )?;
        }
        (AnalysisMethod::Legacy, MethodConfig::Legacy) => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
        }
        _ => {
            return Err(unmapped_cli_capability_error(
                method,
                config.kind(),
                CliCapabilityMappingFailure::UnmappedMethodConfig,
            ));
        }
    }

    if required_cells.is_empty() {
        return Err(unmapped_cli_capability_error(
            method,
            config.kind(),
            CliCapabilityMappingFailure::EmptyMapping,
        ));
    }
    Ok(required_cells)
}

fn require_cli_capability_availability(
    recipe: &AnalysisRecipe,
    allow_experimental: bool,
    allow_internal_qualification: bool,
) -> Result<()> {
    require_internal_qualification_build(
        allow_internal_qualification,
        cfg!(debug_assertions),
    )?;
    require_cli_capability_availability_after_build_guard(
        recipe,
        allow_experimental,
        allow_internal_qualification,
    )
}

fn require_internal_qualification_build(
    allow_internal_qualification: bool,
    debug_assertions_enabled: bool,
) -> Result<()> {
    if allow_internal_qualification && !debug_assertions_enabled {
        bail!("--allow-internal-qualification is available only in debug validation builds");
    }
    Ok(())
}

fn require_cli_capability_availability_after_build_guard(
    recipe: &AnalysisRecipe,
    allow_experimental: bool,
    allow_internal_qualification: bool,
) -> Result<()> {
    if !recipe.model.higher_order_constructs.is_empty()
        && !pls_higher_order_recipe_is_standard_scope(recipe)
        && !allow_experimental
    {
        bail!(
            "This higher-order recipe is outside the Standard disjoint two-stage point-estimate scope and requires --allow-experimental; Standard supports exactly one reflective indicator-free HOC, at least two reflective measured measurement-only components, one HOC-to-measured-outcome path, path weighting, standardized listwise data, and no resampling or case weights"
        );
    }
    let required_cells = required_cli_capability_cells(recipe)?;
    let registry = CapabilityRegistryV2::embedded().context(
        "the embedded Capability Registry V2 is invalid; analysis availability cannot be checked",
    )?;
    for required in required_cells {
        let cell = registry
            .option_cells()
            .find(|cell| {
                cell.capability_id == required.capability_id
                    && cell.cell_id == required.cell_id
                    && cell.capability_version == required.capability_version
            })
            .with_context(|| {
                format!(
                    "Capability Registry V2 does not contain exact option cell {}::{}@{}",
                    required.capability_id, required.cell_id, required.capability_version,
                )
            })?;
        if cell.standard_available()
            || (allow_experimental && cell.labs_available())
            || (allow_internal_qualification && internal_qualification_allows_cell(recipe, cell))
        {
            continue;
        }
        if cell.labs_available() {
            bail!(
                "{} is Experimental and requires --allow-experimental for CLI calculation",
                cell.cell_id,
            );
        }
        bail!(
            "{} is unavailable for CLI calculation (coverage={}, evidence={}, surface={}); the source implementation remains restricted to internal qualification until its exact registry cell becomes executable",
            cell.cell_id,
            cell.coverage_state,
            cell.evidence_state,
            cell.surface,
        );
    }
    Ok(())
}

fn pls_higher_order_recipe_is_standard_scope(recipe: &AnalysisRecipe) -> bool {
    let declarations = &recipe.model.higher_order_constructs;
    if declarations.len() != 1
        || recipe.settings.method != AnalysisMethod::PlsPm
        || !matches!(
            recipe.method_config.as_ref(),
            Some(MethodConfig::PlsAlgorithm)
        )
        || recipe.settings.weighting_scheme != qpls_core::WeightingScheme::Path
        || recipe.settings.preprocessing != qpls_core::Preprocessing::Standardized
        || recipe.settings.missing_data != qpls_core::MissingDataPolicy::ListwiseDeletion
        || recipe.settings.bootstrap_samples != 0
        || recipe.settings.studentized_inner_samples != 0
        || recipe.settings.permutation_samples != 0
        || recipe.settings.case_weight_column.is_some()
        || !recipe.model.controls.is_empty()
        || !recipe.model.interactions.is_empty()
        || recipe.model.paths.len() != 1
    {
        return false;
    }

    let declaration = &declarations[0];
    if declaration.method != qpls_core::HigherOrderMethod::TwoStage
        || declaration.stage_one_recipe.is_some()
        || declaration.components.len() < 2
    {
        return false;
    }
    let component_ids = declaration
        .components
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if component_ids.len() != declaration.components.len() {
        return false;
    }
    let Some(hoc) = recipe
        .model
        .constructs
        .iter()
        .find(|construct| construct.id == declaration.id.as_str())
    else {
        return false;
    };
    if hoc.mode != MeasurementMode::Reflective || !hoc.indicators.is_empty() {
        return false;
    }
    if !declaration.components.iter().all(|component_id| {
        recipe.model.constructs.iter().any(|construct| {
            construct.id == component_id.as_str()
                && construct.mode == MeasurementMode::Reflective
                && !construct.indicators.is_empty()
        }) && recipe.model.paths.iter().all(|path| {
            path.source != component_id.as_str() && path.target != component_id.as_str()
        })
    }) {
        return false;
    }
    let path = &recipe.model.paths[0];
    path.source == declaration.id.as_str()
        && !component_ids.contains(path.target.as_str())
        && recipe.model.constructs.iter().any(|construct| {
            construct.id == path.target.as_str()
                && construct.id != declaration.id.as_str()
                && !construct.indicators.is_empty()
        })
}

fn require_v04_inference_qualification_scope(recipe: &AnalysisRecipe) -> Result<()> {
    let required = required_cli_capability_cells(recipe)?;
    let expected = vec![
        RequiredCliCapabilityCellV2 {
            capability_id: "smartpls.pls_bootstrapping".into(),
            cell_id: "qpls3.inference.bootstrap".into(),
            capability_version: "indexed_resampling_v4".into(),
        },
        RequiredCliCapabilityCellV2 {
            capability_id: "smartpls.pls_algorithm".into(),
            cell_id: "qpls3.pls.algorithm".into(),
            capability_version: "pls_pm_v1".into(),
        },
    ];
    if required != expected {
        bail!(
            "v0.4 inference qualification requires exactly the bounded PLS bootstrap and PLS algorithm cells"
        );
    }
    Ok(())
}

fn internal_qualification_allows_cell(
    recipe: &AnalysisRecipe,
    cell: &CapabilityOptionCellV2,
) -> bool {
    internal_qualification_allows_cell_for_build(recipe, cell, cfg!(debug_assertions))
}

fn internal_qualification_allows_cell_for_build(
    recipe: &AnalysisRecipe,
    cell: &CapabilityOptionCellV2,
    debug_assertions_enabled: bool,
) -> bool {
    if !debug_assertions_enabled
        || cell.coverage_state != qpls_core::CoverageStateV2::Partial
        || cell.surface != ProductSurfaceV2::Labs
    {
        return false;
    }
    let Some(config) = recipe.method_config.as_ref() else {
        return false;
    };
    let explicitly_allowlisted = match (recipe.settings.method, config) {
        (AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm) => {
            cell.capability_id == "smartpls.pls_algorithm"
                && cell.cell_id == "qpls3.pls.algorithm"
                && cell.capability_version == "pls_pm_v1"
        }
        (AnalysisMethod::Wpls, MethodConfig::Wpls) => {
            cell.capability_id == "smartpls.wpls"
                && cell.cell_id == "qpls3.pls.weighted"
                && cell.capability_version == "wpls_case_weighted_v1"
        }
        _ => false,
    };
    if explicitly_allowlisted {
        return true;
    }
    let Some(contract) = qpls_core::generated::established_method_contract_v1(
        recipe.settings.method.as_str(),
        config.kind(),
    ) else {
        return false;
    };
    contract.capability_requirements.iter().any(|required| {
        cell.capability_id == required.capability_id
            && cell.cell_id == required.cell_id
            && cell.capability_version == required.capability_version
    })
}

fn require_executable_project(project: &Project) -> Result<()> {
    if project.read_only {
        bail!(
            "project archive schema {} is newer than this QuickPLS build; it is available for read-only inspection/export but cannot execute new analyses",
            project.manifest.schema_version
        );
    }
    Ok(())
}

fn validate_input(input: &Path, json_output: bool) -> Result<()> {
    if input
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("qpls"))
    {
        let (project, recovery_source) = load_project_with_autosave(input)
            .with_context(|| format!("invalid project {}", input.display()))?;
        let recovered = recovery_source.is_some();
        let issues = project
            .recipes
            .iter()
            .flat_map(validate_recipe)
            .collect::<Vec<_>>();
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"valid": !issues.iter().any(|issue| issue.severity == Severity::Error), "recovered": recovered, "issues": issues})
                )?
            );
        } else if issues.is_empty() {
            println!(
                "valid project archive{}",
                if recovered {
                    " (recovered from backup)"
                } else {
                    ""
                }
            );
        } else {
            print_issues(&issues);
        }
        if issues.iter().any(|issue| issue.severity == Severity::Error) {
            bail!("project validation failed");
        }
        return Ok(());
    }
    let recipe: AnalysisRecipe = serde_json::from_slice(
        &fs::read(input).with_context(|| format!("cannot read {}", input.display()))?,
    )
    .context("invalid analysis recipe JSON")?;
    let issues = validate_recipe(&recipe);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&issues)?);
    } else if issues.is_empty() {
        println!("valid");
    } else {
        print_issues(&issues);
    }
    if issues.iter().any(|issue| issue.severity == Severity::Error) {
        bail!("recipe validation failed");
    }
    Ok(())
}

fn migrate_recipe(input: &Path, output: &Path, json_output: bool) -> Result<()> {
    if input == output {
        bail!("migration output must differ from the historical source recipe");
    }
    if output.exists() {
        bail!("migration output already exists: {}", output.display());
    }
    let source: AnalysisRecipe = serde_json::from_slice(
        &fs::read(input).with_context(|| format!("cannot read {}", input.display()))?,
    )
    .context("invalid historical analysis recipe JSON")?;
    let migrated = source
        .migrated_v3_with_fresh_id()
        .context("historical recipe cannot be migrated automatically")?;
    let issues = validate_recipe(&migrated);
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        bail!(
            "migrated recipe failed schema-v3 validation: {}",
            errors
                .iter()
                .map(|issue| format!("{}: {}", issue.code, issue.message))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(output, serde_json::to_vec_pretty(&migrated)?)
        .with_context(|| format!("cannot write {}", output.display()))?;
    let report = json!({
        "source": input,
        "output": output,
        "source_schema_version": source.schema_version,
        "target_schema_version": migrated.schema_version,
        "source_recipe_id": source.id,
        "target_recipe_id": migrated.id,
        "source_method": source.settings.method,
        "target_method": migrated.settings.method,
        "method_config_kind": migrated.method_config.as_ref().map(MethodConfig::kind),
        "preserved_source": true,
        "issues": issues,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "migrated schema v{} recipe {} to schema v{} recipe {} at {}; source preserved",
            source.schema_version,
            source.id,
            migrated.schema_version,
            migrated.id,
            output.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_pls_recipe() -> AnalysisRecipe {
        let historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        historical.migrated_v3().unwrap()
    }

    fn disjoint_two_stage_hoc_recipe() -> AnalysisRecipe {
        let historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/higher_order_two_stage_base.recipe.json"
        ))
        .unwrap();
        historical.migrated_v3().unwrap()
    }

    fn required_cell(
        capability_id: &str,
        cell_id: &str,
        capability_version: &str,
    ) -> RequiredCliCapabilityCellV2 {
        RequiredCliCapabilityCellV2 {
            capability_id: capability_id.into(),
            cell_id: cell_id.into(),
            capability_version: capability_version.into(),
        }
    }

    fn write_runner_result(recipe_path: &Path, data_path: &Path, output: &Path) {
        let recipe: AnalysisRecipe =
            serde_json::from_slice(&fs::read(recipe_path).unwrap()).unwrap();
        let dataset =
            qpls_data::import_path(data_path, &qpls_data::ImportOptions::default()).unwrap();
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }

    fn plsc_consistent_bootstrap_recipe() -> AnalysisRecipe {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let historical: AnalysisRecipe = serde_json::from_slice(
            &fs::read(root.join("validation/results/plsc_reference.recipe.json")).unwrap(),
        )
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.settings.bootstrap_samples = 1_000;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.method_config = Some(MethodConfig::Plsc);
        recipe
    }

    fn plsc_consistent_permutation_fixture() -> (qpls_data::Dataset, AnalysisRecipe) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = qpls_data::import_path(
            &root.join("validation/fixtures/plsc_consistent_permutation_two_group.csv"),
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let historical: AnalysisRecipe = serde_json::from_slice(
            &fs::read(root.join("validation/results/micom_v2_reference.recipe.json")).unwrap(),
        )
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::Plsc;
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 99;
        recipe.settings.workers = 2;
        recipe.settings.confidence_level = 0.95;
        recipe.settings.case_weight_column = None;
        recipe.method_config = Some(MethodConfig::PlscPermutation {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            test_tail: qpls_core::PlscPermutationTestTail::TwoSided,
        });
        recipe.metadata.clear();
        (dataset, recipe)
    }

    #[test]
    fn cli_generated_established_method_cells_match_frozen_contracts() {
        assert_eq!(
            ESTABLISHED_CLI_REQUIREMENT_ROLE_ORDER_V1,
            ["primary", "base"]
        );
        let cases = [
            (
                AnalysisMethod::Cca,
                MethodConfig::Cca,
                "cca",
                vec![
                    (
                        "base",
                        required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
                    ),
                    (
                        "primary",
                        required_cell(
                            "smartpls.cca",
                            "qpls3.assessment.cca_residuals",
                            "cca_composite_residual_v1",
                        ),
                    ),
                ],
            ),
            (
                AnalysisMethod::Gsca,
                MethodConfig::Gsca,
                "gsca",
                vec![(
                    "primary",
                    required_cell("smartpls.gsca", "qpls3.gsca.als", "gsca_als_v2"),
                )],
            ),
            (
                AnalysisMethod::Ipma,
                MethodConfig::Ipma {
                    targets: vec!["y".into()],
                },
                "ipma",
                vec![
                    (
                        "base",
                        required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
                    ),
                    (
                        "primary",
                        required_cell("smartpls.ipma", "qpls3.assessment.ipma", "ipma_v1"),
                    ),
                ],
            ),
            (
                AnalysisMethod::Nca,
                MethodConfig::Nca {
                    condition: "x".into(),
                    outcome: "y".into(),
                    ceiling: qpls_core::NcaCeiling::Both,
                    permutation_samples: 0,
                },
                "nca",
                vec![(
                    "primary",
                    required_cell("smartpls.nca", "qpls3.standalone.nca", "nca_v2"),
                )],
            ),
        ];

        for (method, config, config_kind, expected) in cases {
            assert_eq!(config.kind(), config_kind);
            let contract =
                qpls_core::generated::established_method_contract_v1(method.as_str(), config_kind)
                    .unwrap();
            let generated = contract
                .capability_requirements
                .iter()
                .map(|requirement| {
                    (
                        requirement.role,
                        required_cell(
                            requirement.capability_id,
                            requirement.cell_id,
                            requirement.capability_version,
                        ),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(generated, expected, "{method}");
            let expected_cli = expected
                .iter()
                .filter(|(role, _)| *role == "primary")
                .chain(expected.iter().filter(|(role, _)| *role == "base"))
                .map(|(_, required)| required.clone())
                .collect::<Vec<_>>();

            let mut required_cells = Vec::new();
            push_generated_established_cli_capability_cells(
                &mut required_cells,
                method,
                config_kind,
            )
            .unwrap();
            assert_eq!(required_cells, expected_cli, "{method}");

            let mut recipe = simple_pls_recipe();
            recipe.settings.method = method;
            recipe.method_config = Some(config);
            assert_eq!(
                required_cli_capability_cells(&recipe).unwrap(),
                expected_cli,
                "{method}",
            );
        }
    }

    #[test]
    fn cli_generated_established_method_lookup_preserves_unknown_and_dynamic_fallbacks() {
        assert!(qpls_core::generated::established_method_contract_v1("legacy", "legacy").is_none());
        let mut unknown_cells = Vec::new();
        let error = push_generated_established_cli_capability_cells(
            &mut unknown_cells,
            AnalysisMethod::Legacy,
            "legacy",
        )
        .unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.method, AnalysisMethod::Legacy);
        assert_eq!(typed.config_kind, "legacy");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::UnmappedMethodConfig
        );
        assert!(unknown_cells.is_empty());

        assert!(
            qpls_core::generated::established_method_contract_v1("regression", "regression")
                .is_none()
        );
        let mut regression = simple_pls_recipe();
        regression.settings.method = AnalysisMethod::Regression;
        regression.method_config = Some(MethodConfig::Regression {
            outcome: "y".into(),
            predictors: vec!["x".into()],
            controls: Vec::new(),
            model: qpls_core::RegressionModelConfig::Logistic,
            bootstrap: None,
        });
        assert_eq!(
            required_cli_capability_cells(&regression).unwrap(),
            vec![required_cell(
                "smartpls.logistic_regression",
                "qpls3.standalone.logistic",
                "regression_logistic_v2",
            )],
        );
    }

    #[test]
    fn cli_plain_pls_point_plsc_and_consistent_bootstrap_are_standard() {
        let pls = simple_pls_recipe();
        assert_eq!(
            required_cli_capability_cells(&pls).unwrap(),
            vec![required_cell(
                "smartpls.pls_algorithm",
                "qpls3.pls.algorithm",
                "pls_pm_v1",
            )],
        );
        for allow_experimental in [false, true] {
            require_cli_capability_availability(&pls, allow_experimental, false).unwrap();
        }

        let recipe = plsc_consistent_bootstrap_recipe();
        for allow_experimental in [false, true] {
            require_cli_capability_availability(&recipe, allow_experimental, false).unwrap();
        }

        let mut point_estimate = recipe;
        point_estimate.settings.bootstrap_samples = 0;
        assert_eq!(
            required_cli_capability_cells(&point_estimate).unwrap(),
            vec![required_cell(
                "smartpls.plsc",
                "qpls3.pls.consistent",
                "plsc_v2",
            )],
        );
        for allow_experimental in [false, true] {
            require_cli_capability_availability(&point_estimate, allow_experimental, false)
                .unwrap();
        }
    }

    #[test]
    fn cli_cta_pls_uses_the_exact_scoped_standard_cell_and_pls_base() {
        let mut recipe = simple_pls_recipe();
        recipe.settings.method = AnalysisMethod::CtaPls;
        recipe.method_config = Some(MethodConfig::CtaPls);

        assert_eq!(
            required_cli_capability_cells(&recipe).unwrap(),
            vec![
                required_cell(
                    "smartpls.cta_pls",
                    "qpls3.assessment.cta_pls",
                    "cta_pls_tetrad_v1",
                ),
                required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
            ],
        );
        for allow_experimental in [false, true] {
            require_cli_capability_availability(&recipe, allow_experimental, false).unwrap();
        }
    }

    #[test]
    fn cli_structural_path_randomization_uses_the_exact_scoped_standard_cell_and_pls_base() {
        let mut recipe = simple_pls_recipe();
        recipe.settings.permutation_samples = 999;
        recipe.method_config = Some(MethodConfig::PlsPermutation);

        assert_eq!(
            required_cli_capability_cells(&recipe).unwrap(),
            vec![
                required_cell(
                    "smartpls.permutation",
                    "qpls3.inference.structural_path_randomization",
                    "freedman_lane_permutation_v1",
                ),
                required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
            ],
        );
        for allow_experimental in [false, true] {
            require_cli_capability_availability(&recipe, allow_experimental, false).unwrap();
        }
    }

    #[test]
    fn cli_higher_order_standard_cell_is_exactly_disjoint_two_stage_point_only() {
        let exact = disjoint_two_stage_hoc_recipe();
        assert!(pls_higher_order_recipe_is_standard_scope(&exact));
        assert_eq!(
            required_cli_capability_cells(&exact).unwrap(),
            vec![
                required_cell(
                    "smartpls.higher_order_models",
                    "qpls3.pls.higher_order_two_stage",
                    "pls_pm_v1",
                ),
                required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
            ],
        );
        require_cli_capability_availability(&exact, false, false).unwrap();

        let mut outside = Vec::new();
        let mut repeated = exact.clone();
        repeated.model.higher_order_constructs[0].method =
            qpls_core::HigherOrderMethod::RepeatedIndicators;
        outside.push(repeated);
        let mut hybrid = exact.clone();
        hybrid.model.higher_order_constructs[0].method = qpls_core::HigherOrderMethod::Hybrid;
        outside.push(hybrid);
        let mut extra_path = exact.clone();
        extra_path.model.paths.push(StructuralPath {
            source: "x".into(),
            target: "y".into(),
        });
        outside.push(extra_path);
        let mut resampled = exact.clone();
        resampled.settings.bootstrap_samples = 999;
        resampled.method_config = Some(MethodConfig::PlsBootstrap);
        outside.push(resampled);

        for recipe in outside {
            assert!(!pls_higher_order_recipe_is_standard_scope(&recipe));
            let error = require_cli_capability_availability(&recipe, false, false)
                .unwrap_err()
                .to_string();
            assert!(error.contains("outside the Standard disjoint two-stage point-estimate scope"));
            require_cli_capability_availability(&recipe, true, false).unwrap();
        }
    }

    #[test]
    fn cli_internal_qualification_is_debug_only_and_exactly_scoped() {
        let mut cca = simple_pls_recipe();
        cca.settings.method = AnalysisMethod::Cca;
        cca.method_config = Some(MethodConfig::Cca);

        let ordinary = require_cli_capability_availability(&cca, false, false)
            .unwrap_err()
            .to_string();
        assert!(ordinary.contains("qpls3.assessment.cca_residuals is Experimental"));
        require_cli_capability_availability(&cca, true, false).unwrap();

        let registry = CapabilityRegistryV2::embedded().unwrap();
        let mut absent_evidence_cell = registry
            .option_cells()
            .find(|cell| cell.cell_id == "qpls3.assessment.cca_residuals")
            .unwrap()
            .clone();
        absent_evidence_cell.evidence_state = qpls_core::EvidenceStateV2::Absent;
        if cfg!(debug_assertions) {
            assert!(internal_qualification_allows_cell(
                &cca,
                &absent_evidence_cell
            ));
            require_cli_capability_availability(&cca, false, true).unwrap();
        }

        let plain_pls = simple_pls_recipe();
        let mut pls_cell = registry
            .option_cells()
            .find(|cell| cell.cell_id == "qpls3.pls.algorithm")
            .unwrap()
            .clone();
        pls_cell.evidence_state = qpls_core::EvidenceStateV2::Absent;
        pls_cell.surface = ProductSurfaceV2::Labs;
        assert!(internal_qualification_allows_cell_for_build(
            &plain_pls,
            &pls_cell,
            true,
        ));
        assert!(!internal_qualification_allows_cell_for_build(
            &plain_pls,
            &pls_cell,
            false,
        ));

        let mut wpls = simple_pls_recipe();
        wpls.settings.method = AnalysisMethod::Wpls;
        wpls.method_config = Some(MethodConfig::Wpls);
        let mut wpls_cell = registry
            .option_cells()
            .find(|cell| cell.cell_id == "qpls3.pls.weighted")
            .unwrap()
            .clone();
        wpls_cell.evidence_state = qpls_core::EvidenceStateV2::Absent;
        wpls_cell.surface = ProductSurfaceV2::Labs;
        assert!(internal_qualification_allows_cell_for_build(
            &wpls,
            &wpls_cell,
            true,
        ));
        assert!(!internal_qualification_allows_cell_for_build(
            &wpls,
            &wpls_cell,
            false,
        ));
        if cfg!(debug_assertions) {
            require_cli_capability_availability(&wpls, false, true).unwrap();
        }

        let mut wrong_version = pls_cell;
        wrong_version.capability_version = "pls_pm_v999".into();
        assert!(!internal_qualification_allows_cell_for_build(
            &plain_pls,
            &wrong_version,
            true,
        ));

        require_internal_qualification_build(true, true).unwrap();
        let release_error = require_internal_qualification_build(true, false)
            .unwrap_err()
            .to_string();
        assert!(release_error.contains("only in debug validation builds"));
    }

    #[test]
    fn cli_required_cells_keep_derived_dependencies_before_their_base_cells() {
        let mut bootstrap = simple_pls_recipe();
        bootstrap.settings.bootstrap_samples = 1_000;
        bootstrap.method_config = Some(MethodConfig::PlsBootstrap);
        assert_eq!(
            required_cli_capability_cells(&bootstrap).unwrap(),
            vec![
                required_cell(
                    "smartpls.pls_bootstrapping",
                    "qpls3.inference.bootstrap",
                    "indexed_resampling_v4",
                ),
                required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1",),
            ],
        );

        let mut prospective_power = simple_pls_recipe();
        prospective_power.settings.method = AnalysisMethod::PlsSampleSizePower;
        prospective_power.method_config = Some(MethodConfig::PlsSampleSizePower(
            qpls_core::PlsSampleSizePowerConfig {
                scenario_identity: "cli-v2-power".into(),
                predictor_construct: "Capability".into(),
                outcome_construct: "Retention".into(),
                predictor_indicator_loadings: vec![0.8, 0.8, 0.8],
                outcome_indicator_loadings: vec![0.8, 0.8, 0.8],
                population_path: 0.3,
                exogenous_distribution: qpls_core::PlsPowerDistribution::StandardNormal,
                structural_disturbance_distribution:
                    qpls_core::PlsPowerDistribution::StandardNormal,
                indicator_error_distribution: qpls_core::PlsPowerDistribution::StandardNormal,
                missing_data: qpls_core::PlsPowerMissingData::None,
                inference:
                    qpls_core::PlsPowerInference::CaseBootstrapNullCenteredTwoSidedPlusOne,
                sample_size_grid: vec![50, 100],
                alpha: 0.05,
                target_power: 0.8,
                interval_confidence_level: 0.95,
                monte_carlo_replicates: 100,
                bootstrap_replicates: 99,
            },
        ));
        assert_eq!(
            required_cli_capability_cells(&prospective_power).unwrap(),
            vec![required_cell(
                "smartpls.pls_power_analysis",
                "qpls3.pls.sample_size_power",
                PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
            )],
        );

        let mut posthoc = simple_pls_recipe();
        posthoc.settings.bootstrap_samples = 1_000;
        posthoc.method_config = Some(MethodConfig::PlsPosthocTechnicalMinimumSampleSize(
            qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::bootstrap_v2(),
        ));
        assert_eq!(
            required_cli_capability_cells(&posthoc).unwrap(),
            vec![
                required_cell(
                    "smartpls.pls_power_analysis",
                    "qpls3.pls.posthoc_technical_minimum_sample_size",
                    "pls_posthoc_technical_minimum_sample_size_v2",
                ),
                required_cell(
                    "smartpls.pls_bootstrapping",
                    "qpls3.inference.bootstrap",
                    "indexed_resampling_v4",
                ),
                required_cell("smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1",),
            ],
        );

        let plsc = plsc_consistent_bootstrap_recipe();
        assert_eq!(
            required_cli_capability_cells(&plsc).unwrap(),
            vec![
                required_cell(
                    "smartpls.consistent_bootstrapping",
                    "qpls3.inference.consistent_bootstrap",
                    "plsc_bootstrap_v1",
                ),
                required_cell("smartpls.plsc", "qpls3.pls.consistent", "plsc_v2",),
            ],
        );
    }

    #[test]
    fn cli_required_cells_preserve_standalone_cbsem_and_shared_cell_identities() {
        let mut pca = simple_pls_recipe();
        pca.settings.method = AnalysisMethod::Pca;
        pca.method_config = Some(MethodConfig::Pca {
            variables: vec!["x1".into(), "x2".into()],
            retention: qpls_core::PcaRetentionConfig::Kaiser,
        });
        assert_eq!(
            required_cli_capability_cells(&pca).unwrap(),
            vec![
                required_cell("smartpls.pca_core", "qpls3.standalone.pca", "pca_v1",),
                required_cell("smartpls.pca_cbsem", "qpls3.standalone.pca", "pca_v1",),
            ],
        );

        let mut cbsem = simple_pls_recipe();
        cbsem.settings.method = AnalysisMethod::Cbsem;
        cbsem.method_config = Some(MethodConfig::Cbsem {
            model_type: qpls_core::CbsemModelType::Cfa,
            estimator: qpls_core::CbsemEstimator::Ml,
            input: qpls_core::CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            bootstrap_v2: None,
            group_column: None,
            invariance_steps: Vec::new(),
        });
        assert_eq!(
            required_cli_capability_cells(&cbsem).unwrap(),
            vec![required_cell(
                "smartpls.cfa",
                "qpls3.cbsem.ml",
                "cbsem_ml_v1",
            )],
        );
        require_cli_capability_availability(&cbsem, false, false).unwrap();

        cbsem.method_config = Some(MethodConfig::Cbsem {
            model_type: qpls_core::CbsemModelType::Sem,
            estimator: qpls_core::CbsemEstimator::Ml,
            input: qpls_core::CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            bootstrap_v2: None,
            group_column: None,
            invariance_steps: Vec::new(),
        });
        require_cli_capability_availability(&cbsem, false, false).unwrap();

        cbsem.method_config = Some(MethodConfig::Cbsem {
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
        assert_eq!(
            required_cli_capability_cells(&cbsem).unwrap(),
            vec![
                required_cell(
                    "smartpls.cbsem_bootstrapping",
                    "qpls3.cbsem.bootstrap",
                    "cbsem_bootstrap_v2",
                ),
                required_cell("smartpls.cbsem", "qpls3.cbsem.ml", "cbsem_ml_v1",),
            ],
        );

        let mut regression = simple_pls_recipe();
        regression.settings.method = AnalysisMethod::Regression;
        regression.method_config = Some(MethodConfig::Regression {
            outcome: "y".into(),
            predictors: vec!["x".into()],
            controls: Vec::new(),
            model: qpls_core::RegressionModelConfig::Logistic,
            bootstrap: None,
        });
        assert_eq!(
            required_cli_capability_cells(&regression).unwrap(),
            vec![required_cell(
                "smartpls.logistic_regression",
                "qpls3.standalone.logistic",
                "regression_logistic_v2",
            )],
        );
    }

    #[test]
    fn cli_required_cells_fail_closed_for_missing_mismatched_legacy_and_unmapped_configs() {
        let mut missing = simple_pls_recipe();
        missing.method_config = None;
        let error = required_cli_capability_cells(&missing).unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.method, AnalysisMethod::PlsPm);
        assert_eq!(typed.config_kind, "<missing>");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::MissingMethodConfig
        );

        let mut mismatched = simple_pls_recipe();
        mismatched.method_config = Some(MethodConfig::Plsc);
        let error = required_cli_capability_cells(&mismatched).unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.config_kind, "plsc");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::MethodConfigMismatch
        );

        let mut legacy = simple_pls_recipe();
        legacy.settings.method = AnalysisMethod::Legacy;
        legacy.method_config = Some(MethodConfig::Legacy);
        let error = required_cli_capability_cells(&legacy).unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.method, AnalysisMethod::Legacy);
        assert_eq!(typed.config_kind, "legacy");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::UnmappedMethodConfig
        );

        let mut empty_group_options = simple_pls_recipe();
        empty_group_options.settings.method = AnalysisMethod::Mga;
        empty_group_options.method_config = Some(MethodConfig::Mga {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            methods: Vec::new(),
            permutation_samples: 99,
            configural_invariance_confirmed: true,
        });
        let error = required_cli_capability_cells(&empty_group_options).unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.method, AnalysisMethod::Mga);
        assert_eq!(typed.config_kind, "mga");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::UnmappedMethodConfig
        );

        let mut unmapped_cbsem = simple_pls_recipe();
        unmapped_cbsem.settings.method = AnalysisMethod::Cbsem;
        unmapped_cbsem.method_config = Some(MethodConfig::Cbsem {
            model_type: qpls_core::CbsemModelType::Sem,
            estimator: qpls_core::CbsemEstimator::RobustMl,
            input: qpls_core::CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            bootstrap_v2: None,
            group_column: None,
            invariance_steps: Vec::new(),
        });
        let error = required_cli_capability_cells(&unmapped_cbsem).unwrap_err();
        let typed = error.downcast_ref::<UnmappedCliCapabilityError>().unwrap();
        assert_eq!(typed.method, AnalysisMethod::Cbsem);
        assert_eq!(typed.config_kind, "cbsem");
        assert_eq!(
            typed.failure,
            CliCapabilityMappingFailure::UnmappedMethodConfig
        );
    }

    #[test]
    fn cli_posthoc_standard_scope_exports_exact_runner_attribution_without_labs_override() {
        let dataset = qpls_data::import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.method_config = Some(MethodConfig::PlsPosthocTechnicalMinimumSampleSize(
            qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2(),
        ));

        require_cli_capability_availability(&recipe, false, false).unwrap();

        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert!(result.provenance.method_version.split('+').any(|version| {
            version == qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION
        }));
        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "posthoc_minimum_sample_size"
                && row.metric == "method_version"
                && row.value == qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION
        }));
        assert!(rows.iter().any(|row| {
            row.section == "posthoc_minimum_sample_size"
                && row.metric == "availability"
                && row.value == "Standard"
        }));
    }

    #[test]
    fn pls_bootstrap_one_sided_export_is_semantic_and_fail_closed() {
        let dataset = qpls_data::import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = simple_pls_recipe();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 8;
        recipe.settings.workers = 1;
        recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::OneSidedGreater;
        recipe.method_config = Some(MethodConfig::PlsBootstrap);
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();

        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "pls_bootstrap_test_tail_contract"
                && row.metric == "method_version"
                && row.value == PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION
        }));
        assert!(rows.iter().any(|row| {
            row.section == "pls_bootstrap_test_tail_contract"
                && row.metric == "selected_test_tail"
                && row.value == "one_sided_greater"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "pls_bootstrap_test_tail_parameter"
                && row.metric == "selected_exceedances"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "pls_bootstrap_test_tail_parameter" && row.metric == "selected_p_value"
        }));

        let mut missing = result.clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut missing.payload {
            bootstrap
                .as_object_mut()
                .unwrap()
                .remove("test_tail_inference");
        }
        assert!(experimental_pls_export_rows(&missing).is_err());

        let mut injected_default = result.clone();
        injected_default.provenance.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
        injected_default.provenance.method_version = injected_default
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        assert!(experimental_pls_export_rows(&injected_default).is_err());

        let mut wrong = result.clone();
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut wrong.payload {
            bootstrap["test_tail_inference"]["parameters"][0]["p_value_greater"] =
                serde_json::json!(0.5);
        }
        assert!(experimental_pls_export_rows(&wrong).is_err());

        let mut malformed = result;
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut malformed.payload {
            bootstrap["test_tail_inference"] = serde_json::json!("not-a-receipt");
        }
        assert!(experimental_pls_export_rows(&malformed).is_err());
    }

    #[test]
    fn plsc_consistent_bootstrap_export_is_typed_complete_and_fail_closed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = qpls_data::import_path(
            &root.join("validation/results/plsc_reference.csv"),
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = plsc_consistent_bootstrap_recipe();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.workers = 2;
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let (successful_replicate_witnesses, successful_jackknife_witnesses) =
            match &result.payload {
                AnalysisPayload::PlsPmV2 { bootstrap, .. } => (
                    bootstrap["successful_replicates"].as_array().unwrap().len(),
                    bootstrap["successful_jackknife_cases"]
                        .as_array()
                        .unwrap()
                        .len(),
                ),
                other => panic!("PLSc consistent bootstrap must use a linked v2 payload: {other:?}"),
            };

        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_accounting"
                && row.metric == "requested_replicates"
                && row.value == "1000"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_accounting"
                && row.metric == "attempted_replicates"
                && row.value == "1000"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_accounting"
                && row.metric == "successful_replicate_witnesses"
                && row.value == successful_replicate_witnesses.to_string()
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_accounting"
                && row.metric == "successful_jackknife_witnesses"
                && row.value == successful_jackknife_witnesses.to_string()
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_percentile" && row.metric == "standard_error"
        }));
        assert!(rows.iter().any(|row| row.section == "plsc_bootstrap_bca"));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_bootstrap_warning"
                && row.value.contains("fully re-estimated plsc_v2")
        }));

        let mut tampered = result;
        if let AnalysisPayload::PlsPmV2 { bootstrap, .. } = &mut tampered.payload {
            bootstrap["warnings"] = serde_json::json!([]);
        } else {
            panic!("PLSc consistent bootstrap must use a linked v2 payload");
        }
        assert!(experimental_pls_export_rows(&tampered).is_err());
    }

    #[test]
    fn plsc_consistent_permutation_is_hidden_and_semantic_export_is_fail_closed() {
        let (dataset, recipe) = plsc_consistent_permutation_fixture();
        for allow_experimental in [false, true] {
            let error = require_cli_capability_availability(&recipe, allow_experimental, false)
                .unwrap_err()
                .to_string();
            assert!(error.contains("qpls3.inference.consistent_permutation is unavailable"));
            assert!(error.contains("coverage=absent"));
            assert!(error.contains("evidence=absent"));
            assert!(error.contains("surface=labs"));
        }

        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_accounting"
                && row.metric == "requested_permutations"
                && row.value == "99"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_group"
                && row.construct == "group_a"
                && row.metric == "group"
                && row.value == "A"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_accounting"
                && row.metric == "test_method"
                && row.value == qpls_resampling::PLSC_CONSISTENT_PERMUTATION_TEST
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_accounting"
                && row.metric == "directional_test_method"
                && row.value == qpls_resampling::PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_parameter" && row.metric == "p_value_two_sided"
        }));
        for metric in [
            "greater_or_equal",
            "less_or_equal",
            "p_value_greater",
            "p_value_less",
        ] {
            assert!(rows.iter().any(|row| {
                row.section == "plsc_permutation_parameter" && row.metric == metric
            }));
        }
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_ledger" && row.metric == "label_assignment_sha256"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "plsc_permutation_warning"
                && row.value == qpls_resampling::PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING
        }));

        let mut selected = result.clone();
        selected.provenance.method_version.push('+');
        selected
            .provenance
            .method_version
            .push_str(PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION);
        let selected_permutation = match &mut selected.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(value),
                ..
            } => value,
            _ => unreachable!(),
        };
        let parameters = selected_permutation["directional_inference"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|parameter| {
                serde_json::json!({
                    "parameter": parameter["parameter"],
                    "selected_exceedances": parameter["greater_or_equal"],
                    "selected_p_value": parameter["p_value_greater"],
                    "permutations": parameter["permutations"]
                })
            })
            .collect::<Vec<_>>();
        selected_permutation["selected_tail_inference"] = serde_json::json!({
            "method_version": PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
            "orientation": "group_a_minus_group_b",
            "selected_test_tail": "group_a_greater",
            "parameters": parameters
        });
        let selected_rows = experimental_pls_export_rows(&selected).unwrap();
        for (metric, value) in [
            (
                "method_version",
                PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
            ),
            ("orientation", "group_a_minus_group_b"),
            ("selected_test_tail", "group_a_greater"),
        ] {
            assert!(selected_rows.iter().any(|row| {
                row.section == "plsc_permutation_selected_tail"
                    && row.metric == metric
                    && row.value == value
            }));
        }
        assert!(selected_rows.iter().any(|row| {
            row.section == "plsc_permutation_selected_tail_parameter"
                && row.metric == "selected_exceedances"
        }));
        assert!(selected_rows.iter().any(|row| {
            row.section == "plsc_permutation_selected_tail_parameter"
                && row.metric == "selected_p_value"
        }));
        let mut missing_marker = selected.clone();
        missing_marker.provenance.method_version = missing_marker
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        assert!(experimental_pls_export_rows(&missing_marker).is_err());
        let mut tampered_selected = selected;
        if let AnalysisPayload::PlsPmV3 {
            permutation: Some(value),
            ..
        } = &mut tampered_selected.payload
        {
            value["selected_tail_inference"]["parameters"][0]["selected_p_value"] =
                serde_json::json!(0.5);
        }
        assert!(experimental_pls_export_rows(&tampered_selected).is_err());

        let mut wrong_scheduler = result.clone();
        let permutation = match &mut wrong_scheduler.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => permutation,
            other => panic!("expected PLSc consistent-permutation payload, got {other:?}"),
        };
        permutation["scheduler_method_version"] = serde_json::json!("forged_scheduler");
        assert!(experimental_pls_export_rows(&wrong_scheduler).is_err());

        let mut wrong_ledger = result.clone();
        let permutation = match &mut wrong_ledger.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => permutation,
            other => panic!("expected PLSc consistent-permutation payload, got {other:?}"),
        };
        permutation["permutation_ledger"][0]["label_assignment_sha256"] =
            serde_json::json!("0".repeat(64));
        assert!(experimental_pls_export_rows(&wrong_ledger).is_err());

        let mut wrong_directional_count = result.clone();
        let permutation = match &mut wrong_directional_count.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => permutation,
            other => panic!("expected PLSc consistent-permutation payload, got {other:?}"),
        };
        let count = permutation["directional_inference"]["parameters"][0]["greater_or_equal"]
            .as_u64()
            .unwrap();
        permutation["directional_inference"]["parameters"][0]["greater_or_equal"] =
            serde_json::json!(if count < 99 { count + 1 } else { count - 1 });
        assert!(experimental_pls_export_rows(&wrong_directional_count).is_err());

        let mut wrong_probability = result;
        let permutation = match &mut wrong_probability.payload {
            AnalysisPayload::PlsPmV3 {
                permutation: Some(permutation),
                ..
            } => permutation,
            other => panic!("expected PLSc consistent-permutation payload, got {other:?}"),
        };
        let probability = permutation["parameters"][0]["p_value_two_sided"]
            .as_f64()
            .unwrap();
        permutation["parameters"][0]["p_value_two_sided"] =
            serde_json::json!(if probability < 0.5 {
                probability + 0.25
            } else {
                probability - 0.25
            });
        assert!(experimental_pls_export_rows(&wrong_probability).is_err());
    }

    #[test]
    fn pls_model_fit_export_is_matrix_validated_and_exact_attribution_is_fail_closed() {
        let dataset = qpls_data::import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::PlsPm;
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.method_config = Some(MethodConfig::PlsAlgorithm);

        let mut exact_recipe = recipe.clone();
        exact_recipe.settings.bootstrap_samples = 999;
        exact_recipe.method_config = Some(MethodConfig::PlsBootstrap);
        exact_recipe
            .metadata
            .insert(PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR.into(), "true".into());
        for allow_experimental in [false, true] {
            let error =
                require_cli_capability_availability(&exact_recipe, allow_experimental, false)
                    .unwrap_err()
                    .to_string();
            assert!(error.contains("qpls3.assessment.model_fit is unavailable"));
            assert!(error.contains("coverage=partial"));
            assert!(error.contains("evidence=absent"));
            assert!(error.contains("surface=labs"));
        }

        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "pls_model_fit_detail"
                && row.metric == "method_version"
                && row.value == PLS_MODEL_FIT_METHOD_VERSION
        }));
        assert!(rows.iter().any(|row| {
            row.section == "pls_model_fit" && row.construct == "saturated" && row.metric == "srmr"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "pls_model_fit" && row.construct == "estimated" && row.metric == "nfi"
        }));
        assert!(!rows.iter().any(|row| row.section == "pls_model_fit_exact"));

        let mut tampered = result.clone();
        let assessment = match &mut tampered.payload {
            AnalysisPayload::PlsPmV1 { assessment, .. }
            | AnalysisPayload::PlsPmV2 { assessment, .. }
            | AnalysisPayload::PlsPmV3 { assessment, .. } => assessment,
            other => panic!("expected PLS payload, received {other:?}"),
        };
        assessment["model_fit"]["estimated"]["srmr"] = serde_json::json!(999.0);
        assert!(
            experimental_pls_export_rows(&tampered)
                .unwrap_err()
                .to_string()
                .contains("matrix-backed semantic validation")
        );

        let mut falsely_attributed = result;
        falsely_attributed.provenance.method_version.push('+');
        falsely_attributed
            .provenance
            .method_version
            .push_str(PLS_MODEL_FIT_EXACT_METHOD_VERSION);
        assert!(
            experimental_pls_export_rows(&falsely_attributed)
                .unwrap_err()
                .to_string()
                .contains("payload and provenance identity disagree")
        );
    }

    #[test]
    fn methods_command_uses_option_cell_registry_instead_of_legacy_validated_labels() {
        let registry = CapabilityRegistryV2::embedded().unwrap();
        let text = capability_registry_cli_text(&registry);
        assert!(text.contains(
            "qpls3.pls.algorithm | pls_pm_v1 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.pls.sample_size_power | pls_sample_size_power_v2 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains("Experimental Labs"));
        assert!(text.contains(
            "qpls3.gsca.als | gsca_als_v2 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.assessment.ipma | ipma_v1 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.standalone.nca | nca_v2 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.standalone.logistic | regression_logistic_v2 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.standalone.ols | regression_ols_v1 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.standalone.regression_bootstrap | regression_bootstrap_v1 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(text.contains(
            "qpls3.cbsem.ml | cbsem_ml_v1 | coverage=partial evidence=release_qualified surface=standard | Standard"
        ));
        assert!(!text.contains("Validated"));

        let json = capability_registry_cli_json(&registry);
        assert_eq!(json["schema_version"], 2);
        assert_eq!(json["projection"], "cli_option_cell_availability_v2");
        assert_eq!(json["source_sha256"], registry.source_sha256);
        assert!(
            json["capabilities"]
                .as_array()
                .is_some_and(|rows| rows.len() == 45)
        );
        assert_eq!(
            json["capabilities"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|row| row["option_cells"].as_array())
                .map(Vec::len)
                .sum::<usize>(),
            48,
        );
    }

    fn write_migrated_v3_recipe(source: &Path, destination: &Path) {
        let historical: AnalysisRecipe =
            serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
        fs::write(
            destination,
            serde_json::to_vec_pretty(&historical.migrated_v3().unwrap()).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn migrate_recipe_writes_a_fresh_valid_copy_and_preserves_the_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = root.join("validation/fixtures/simple_reflective.recipe.json");
        let original = fs::read(&source).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("migrated.recipe.json");

        migrate_recipe(&source, &output, true).unwrap();
        let historical: AnalysisRecipe = serde_json::from_slice(&original).unwrap();
        let migrated: AnalysisRecipe = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(migrated.schema_version, ANALYSIS_RECIPE_SCHEMA_VERSION);
        assert_ne!(migrated.id, historical.id);
        assert!(migrated.method_config.is_some());
        assert!(
            validate_recipe(&migrated)
                .iter()
                .all(|issue| issue.severity != Severity::Error)
        );
        assert!(migrate_recipe(&source, &output, false).is_err());
        assert!(migrate_recipe(&source, &source, false).is_err());
    }

    #[test]
    fn cli_run_executes_release_qualified_scoped_pls_without_opt_in() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let historical_recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe = directory.path().join("simple_reflective.v3.recipe.json");
        write_migrated_v3_recipe(&historical_recipe, &recipe);
        for allow_experimental in [false, true] {
            let output = directory
                .path()
                .join(format!("pls-{allow_experimental}.json"));
            run_analysis(
                &recipe,
                Some(&data),
                None,
                &output,
                allow_experimental,
                false,
                false,
                None,
                None,
                None,
                Some(1),
            )
            .unwrap();
            let result: AnalysisResult =
                serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
            assert_eq!(result.status, RunStatus::Completed);
            assert!(result.provenance.method_version.contains("pls_pm_v1"));
        }
    }

    #[test]
    fn cli_does_not_reinterpret_invalid_v3_config_without_sampling_overrides() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe_path = directory.path().join("invalid-v3.recipe.json");
        let result_path = directory.path().join("result.json");
        let historical: AnalysisRecipe = serde_json::from_slice(
            &fs::read(root.join("validation/fixtures/simple_reflective.recipe.json")).unwrap(),
        )
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.method_config = Some(MethodConfig::PlsBootstrap);
        fs::write(&recipe_path, serde_json::to_vec_pretty(&recipe).unwrap()).unwrap();

        let error = run_analysis(
            &recipe_path,
            Some(&data),
            None,
            &result_path,
            false,
            false,
            false,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("method_config.resampling_mismatch")
        );
        assert!(!result_path.exists());
    }

    #[test]
    fn cli_rejects_execution_from_a_read_only_future_project() {
        let mut project = Project::new("Future read-only project");
        project.manifest.schema_version = qpls_project::PROJECT_ARCHIVE_VERSION + 1;
        project.read_only = true;

        let error = require_executable_project(&project).unwrap_err();
        assert!(error.to_string().contains("read-only inspection/export"));
        assert!(
            error
                .to_string()
                .contains(&project.manifest.schema_version.to_string())
        );
    }

    #[test]
    fn bundled_roadmap_commands_accept_current_registry() {
        roadmap(false, Some("v0.4")).unwrap();
        gate("v0_4_assessment_reliability", false).unwrap();
        assert!(gate("missing_slice", false).is_err());
    }

    #[test]
    fn v04_inference_qualifier_maps_automated_and_registry_gates() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("qualification.json");
        qualify_v04_inference(Some(&output), false, false).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(report["target"], "v04-inference");
        assert_eq!(report["qualification_passed"], true);
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "cli_worker_matrix_1_2_4" && check["status"] == "passed"
                )
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "full_monte_carlo_qualification"
                    && check["status"] == "passed")
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "studentized_cancellation_latency_999x99"
                        && check["status"] == "passed"
                )
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "studentized_performance_benchmark"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "studentized_release_stress_benchmark"
                        && check["status"] == "passed"
                )
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "pls_bootstrap_external_reference"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "pls_bootstrap_corporate_csem_reference"
                        && ["passed", "open"].contains(&check["status"].as_str().unwrap())
                )
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "pls_bootstrap_plspm_external_reference"
                        && ["passed", "open"].contains(&check["status"].as_str().unwrap())
                )
        );
        assert!(
            !report["open_registry_gates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gate| gate["name"] == "Full 999x99 studentized qualification")
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "pilot_monte_carlo_harness"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "sensitivity_monte_carlo_harness"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "studentized_monte_carlo_harness"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "studentized_sensitivity_harness"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |check| check["id"] == "full_studentized_monte_carlo_qualification"
                        && check["status"] == "passed"
                )
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "studentized_supplied_reference"
                    && ["passed", "open"].contains(&check["status"].as_str().unwrap()))
        );
    }

    #[test]
    fn v04_assessment_evidence_report_maps_metric_gaps() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("assessment-evidence.json");
        write_v04_assessment_evidence(Some(&output)).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(report["target"], "v04-assessment");
        assert_eq!(report["slice_id"], "v0_4_assessment_reliability");
        assert_eq!(report["all_listed_artifacts_present"], true);
        assert_eq!(report["open_metric_blockers"].as_u64().unwrap(), 0);
        let metrics = report["metrics"].as_array().unwrap();
        assert!(metrics.iter().any(|metric| metric["id"] == "rho_a"
            && metric["status"] == "fixture_covered_external_reference_open"));
        assert!(metrics.iter().any(|metric| {
            metric["id"] == "htmt_plus"
                && metric["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item == "validation/results/htmt_seminr_comparison.json")
                && metric["missing_evidence"].as_array().unwrap().is_empty()
        }));
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["path"]
                    == "validation/results/external_reference_probe.json"
                    && artifact["present"] == true)
        );
    }

    #[test]
    fn v03_pls_evidence_report_maps_variant_gaps() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("pls-evidence.json");
        write_v03_pls_evidence(Some(&output)).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(report["target"], "v03-pls");
        assert_eq!(report["slice_id"], "v0_3_pls_core");
        assert_eq!(report["all_listed_artifacts_present"], true);
        assert_eq!(report["comparison_status"], "passed");
        let variants = report["references"]["csem_0_6_1"]["variants"]
            .as_array()
            .unwrap();
        assert!(
            variants
                .iter()
                .any(|variant| variant["variant"] == "MODE_B")
        );
        let plspm_variants = report["references"]["python_plspm_0_5_7"]["variants"]
            .as_array()
            .unwrap();
        assert!(
            plspm_variants
                .iter()
                .any(|variant| variant["variant"] == "FACTOR")
        );
        assert_eq!(report["references"]["numpy_pca_eigh"]["status"], "passed");
        assert_eq!(report["references"]["numpy_pca_eigh"]["variant"], "PCA");
        assert_eq!(
            report["references"]["published_csem_threecommonfactors"]["status"],
            "passed"
        );
        assert_eq!(report["open_blockers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn v05_extended_pls_evidence_report_aggregates_method_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("v05.json");
        write_v05_extended_pls_evidence(Some(&output)).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(report["target"], "v05-extended-pls");
        assert_eq!(report["all_listed_artifacts_present"], true);
        assert_eq!(report["all_listed_artifacts_passed"], true);
        assert!(report["artifact_count"].as_u64().unwrap() >= 20);
        assert!(report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["file"] == "validation/results/wpls_reference_report.json"));
    }

    fn typed_power_export_fixture() -> AnalysisResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let historical: AnalysisRecipe = serde_json::from_slice(
            &fs::read(root.join("validation/fixtures/simple_reflective.recipe.json")).unwrap(),
        )
        .unwrap();
        let mut recipe = historical.migrated_v3().unwrap();
        recipe.settings.method = AnalysisMethod::PlsSampleSizePower;
        recipe.settings.seed = 20260814;
        let analysis = PlsSampleSizePowerResultV1 {
            schema_version: PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION,
            capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
            method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION.into(),
            recipe_digest: "a".repeat(64),
            stream_domain: PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN.into(),
            failure_policy: PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY.into(),
            interval_method: PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD.into(),
            inference_method: PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD.into(),
            pls_method_version: qpls_estimation::PLS_METHOD_VERSION.into(),
            resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
            workload: qpls_resampling::PlsPowerWorkload {
                grid_points: 1,
                planned_datasets: 100,
                estimated_pls_fits: 200,
                estimated_pls_case_fits: 10_000,
            },
            rows: vec![qpls_resampling::PlsPowerRowV1 {
                sample_size: 50,
                requested_replicates: 100,
                attempted_replicates: 100,
                successful_replicates: 1,
                failed_replicates: 99,
                rejections: 1,
                achieved_power: 0.01,
                confidence_lower: 0.0005,
                confidence_upper: 0.055,
                qualifies: false,
            }],
            outcomes: std::iter::once(qpls_resampling::PlsPowerReplicateOutcomeV1 {
                sample_size: 50,
                replicate_index: 0,
                stream_identity: "sample-50-replicate-0".into(),
                attempted: true,
                successful: true,
                converged: true,
                target_estimate: Some(0.31),
                p_value_two_sided: Some(0.04),
                bootstrap_requested_replicates: None,
                bootstrap_usable_replicates: None,
                bootstrap_failed_replicates: None,
                bootstrap_two_sided_exceedances: None,
                rejected: true,
                failure_code: None,
                failure_message: None,
            })
            .chain((1..100).map(
                |replicate_index| qpls_resampling::PlsPowerReplicateOutcomeV1 {
                    sample_size: 50,
                    replicate_index,
                    stream_identity: format!("sample-50-replicate-{replicate_index}"),
                    attempted: true,
                    successful: false,
                    converged: false,
                    target_estimate: None,
                    p_value_two_sided: None,
                    bootstrap_requested_replicates: None,
                    bootstrap_usable_replicates: None,
                    bootstrap_failed_replicates: None,
                    bootstrap_two_sided_exceedances: None,
                    rejected: false,
                    failure_code: Some("nonconvergence".into()),
                    failure_message: Some("PLS fit did not converge".into()),
                },
            ))
            .collect(),
            outcome_digest: "b".repeat(64),
            decision: PlsPowerGridDecisionV1::NotReached,
            monotonicity_violations: 0,
            warnings: vec!["Power is conditional on the declared design.".into()],
            exclusions: vec!["Retrospective observed power is excluded.".into()],
        };
        AnalysisResult::completed_pls_sample_size_power(
            &recipe,
            PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
            Utc::now(),
            serde_json::to_value(analysis).unwrap(),
            Vec::<String>::new(),
        )
    }

    fn typed_power_v2_export_fixture() -> AnalysisResult {
        let mut result = typed_power_export_fixture();
        result.provenance.method_version = PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2.into();
        let AnalysisPayload::PlsSampleSizePowerV1 { mut analysis } = result.payload else {
            unreachable!()
        };
        analysis["schema_version"] =
            serde_json::json!(PLS_SAMPLE_SIZE_POWER_RESULT_SCHEMA_VERSION_V2);
        analysis["method_version"] =
            serde_json::json!(PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2);
        analysis["stream_domain"] = serde_json::json!(PLS_SAMPLE_SIZE_POWER_STREAM_DOMAIN_V2);
        analysis["inference_method"] =
            serde_json::json!(PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2);
        analysis["outcomes"][0]["bootstrap_requested_replicates"] = serde_json::json!(99);
        analysis["outcomes"][0]["bootstrap_usable_replicates"] = serde_json::json!(99);
        analysis["outcomes"][0]["bootstrap_failed_replicates"] = serde_json::json!(0);
        analysis["outcomes"][0]["bootstrap_two_sided_exceedances"] = serde_json::json!(3);
        result.payload = AnalysisPayload::PlsSampleSizePowerV2 { analysis };
        result
    }

    fn posthoc_minimum_sample_size_export_fixture() -> AnalysisResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut result: AnalysisResult = serde_json::from_slice(
            &fs::read(root.join("validation/results/pls_publication_benchmark_quickpls.json"))
                .unwrap(),
        )
        .unwrap();
        let estimation = match &mut result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. }
            | AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            other => panic!("expected a PLS estimation payload, received {other:?}"),
        };
        let mut typed: qpls_estimation::PlsResult =
            serde_json::from_value(estimation.clone()).unwrap();
        typed.posthoc_minimum_sample_size =
            qpls_estimation::pls_posthoc_minimum_sample_size(&typed.paths, typed.used_observations);
        *estimation = serde_json::to_value(typed).unwrap();
        result
    }

    #[test]
    fn posthoc_minimum_sample_size_export_is_complete_and_rejects_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("posthoc.json");
        let csv_path = directory.path().join("posthoc.csv");
        let result = posthoc_minimum_sample_size_export_fixture();
        let rows = v03_estimator_export_rows(&result).unwrap();
        let technical_rows = rows
            .iter()
            .filter(|row| row.section == "posthoc_minimum_sample_size")
            .collect::<Vec<_>>();
        assert_eq!(technical_rows.len(), 11);
        assert!(
            technical_rows
                .iter()
                .all(|row| row.source == "x" && row.target == "y")
        );
        assert!(
            technical_rows.iter().any(|row| {
                row.metric == "technically_required_sample_size" && row.value == "7"
            })
        );
        assert!(
            technical_rows
                .iter()
                .any(|row| { row.metric == "analytical_sample_size" && row.value == "6" })
        );

        fs::write(&result_path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        export_result(&result_path, ExportFormat::Csv, Some(&csv_path), false).unwrap();
        let csv = fs::read_to_string(csv_path).unwrap();
        assert!(
            csv.contains("posthoc_minimum_sample_size,,,x,y,technically_required_sample_size,7")
        );
        assert!(csv.contains(
            "posthoc_minimum_sample_size,,,x,y,method_version,inverse_square_root_posthoc_v1"
        ));

        let mut tampered = result;
        let estimation = match &mut tampered.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. }
            | AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            other => panic!("expected a PLS estimation payload, received {other:?}"),
        };
        estimation["posthoc_minimum_sample_size"]["technically_required_sample_size"] =
            serde_json::json!(1);
        let error = v03_estimator_export_rows(&tampered).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not reproduce from its path coefficients and linked inference")
        );
    }

    #[test]
    fn inference_aware_posthoc_export_reproduces_linked_bootstrap_and_rejects_tampering() {
        use calamine::{Reader, open_workbook_auto};

        let dataset = qpls_data::import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: qpls_core::AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        if recipe.schema_version != qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION {
            recipe = recipe.migrated_v3().unwrap();
        }
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 99;
        recipe.settings.workers = 2;
        recipe.method_config = Some(
            qpls_core::MethodConfig::PlsPosthocTechnicalMinimumSampleSize(
                qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::bootstrap_v2(),
            ),
        );
        let result = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let rows = v03_estimator_export_rows(&result).unwrap();
        let technical_rows = rows
            .iter()
            .filter(|row| row.section == "posthoc_minimum_sample_size")
            .collect::<Vec<_>>();
        assert_eq!(technical_rows.len(), 18);
        assert!(technical_rows.iter().any(|row| {
            row.metric == "selection_rule"
                && row.value == "smallest_absolute_statistically_significant_structural_path"
        }));
        assert!(technical_rows.iter().any(|row| {
            row.metric == "significance_source"
                && row.value == "pls_bootstrap_normal_reference_two_sided"
        }));
        assert!(
            technical_rows
                .iter()
                .any(|row| { row.metric == "test" && row.value == "directional" })
        );
        assert!(
            technical_rows
                .iter()
                .any(|row| { row.metric == "availability" && row.value == "Standard" })
        );

        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("posthoc-v2.json");
        let csv_path = directory.path().join("posthoc-v2.csv");
        let html_path = directory.path().join("posthoc-v2.html");
        let xlsx_path = directory.path().join("posthoc-v2.xlsx");
        fs::write(&result_path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        export_result(&result_path, ExportFormat::Csv, Some(&csv_path), false).unwrap();
        export_result(&result_path, ExportFormat::Html, Some(&html_path), false).unwrap();
        export_result(&result_path, ExportFormat::Xlsx, Some(&xlsx_path), false).unwrap();

        let csv = fs::read_to_string(csv_path).unwrap();
        for exact_row in [
            "posthoc_minimum_sample_size,,,x,y,test,directional",
            "posthoc_minimum_sample_size,,,x,y,significance_source,pls_bootstrap_normal_reference_two_sided",
            "posthoc_minimum_sample_size,,,x,y,significance_alpha,0.05",
            "posthoc_minimum_sample_size,,,x,y,availability,Standard",
        ] {
            assert!(csv.contains(exact_row), "CSV omitted {exact_row}");
        }
        let html = fs::read_to_string(html_path).unwrap();
        for value in [
            "posthoc_minimum_sample_size",
            "directional",
            "pls_bootstrap_normal_reference_two_sided",
            "significance_alpha",
            "Standard",
        ] {
            assert!(html.contains(value), "HTML omitted {value}");
        }
        let mut workbook = open_workbook_auto(&xlsx_path).unwrap();
        let range = workbook.worksheet_range("QuickPLS export").unwrap();
        let workbook_rows = range
            .rows()
            .map(|row| row.iter().map(ToString::to_string).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert!(workbook_rows.iter().any(|row| {
            row.get(0)
                .is_some_and(|value| value == "posthoc_minimum_sample_size")
                && row.get(5).is_some_and(|value| value == "test")
                && row.get(6).is_some_and(|value| value == "directional")
        }));
        assert!(workbook_rows.iter().any(|row| {
            row.get(0)
                .is_some_and(|value| value == "posthoc_minimum_sample_size")
                && row
                    .get(5)
                    .is_some_and(|value| value == "significance_source")
                && row
                    .get(6)
                    .is_some_and(|value| value == "pls_bootstrap_normal_reference_two_sided")
        }));
        assert!(workbook_rows.iter().any(|row| {
            row.get(0)
                .is_some_and(|value| value == "posthoc_minimum_sample_size")
                && row.get(5).is_some_and(|value| value == "availability")
                && row.get(6).is_some_and(|value| value == "Standard")
        }));

        let mut tampered = result.clone();
        let estimation = match &mut tampered.payload {
            AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            other => panic!("expected bootstrapped PLS payload, received {other:?}"),
        };
        estimation["posthoc_minimum_sample_size"]["significant_path_count"] =
            serde_json::json!(999);
        assert!(
            v03_estimator_export_rows(&tampered)
                .unwrap_err()
                .to_string()
                .contains("linked inference")
        );

        let mut wrong_formula_test = result.clone();
        let estimation = match &mut wrong_formula_test.payload {
            AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            other => panic!("expected bootstrapped PLS payload, received {other:?}"),
        };
        estimation["posthoc_minimum_sample_size"]["test"] = serde_json::json!("two_sided");
        assert!(
            v03_estimator_export_rows(&wrong_formula_test)
                .unwrap_err()
                .to_string()
                .contains("linked inference")
        );

        let mut mismatched_bootstrap_original = result.clone();
        let bootstrap = match &mut mismatched_bootstrap_original.payload {
            AnalysisPayload::PlsPmV2 { bootstrap, .. } => bootstrap,
            AnalysisPayload::PlsPmV3 {
                bootstrap: Some(bootstrap),
                ..
            } => bootstrap,
            other => panic!("expected bootstrapped PLS payload, received {other:?}"),
        };
        let path_parameter = bootstrap["percentile"]["parameters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|parameter| {
                serde_json::from_str::<(String, Vec<String>)>(
                    parameter["parameter"].as_str().unwrap_or_default(),
                )
                .is_ok_and(|(kind, parts)| kind == "path" && parts.len() == 2)
            })
            .unwrap();
        let original = path_parameter["original"].as_f64().unwrap();
        let statistic = path_parameter["t_statistic"].as_f64().unwrap();
        path_parameter["original"] = serde_json::json!(-original);
        path_parameter["t_statistic"] = serde_json::json!(-statistic);
        assert!(
            v03_estimator_export_rows(&mismatched_bootstrap_original)
                .unwrap_err()
                .to_string()
                .contains("coefficient-mismatched linked PLS path inference")
        );

        let mut wrong_selection_test = result;
        let estimation = match &mut wrong_selection_test.payload {
            AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            other => panic!("expected bootstrapped PLS payload, received {other:?}"),
        };
        estimation["posthoc_minimum_sample_size"]["significance_source"] =
            serde_json::json!("pls_bootstrap_normal_reference_one_sided");
        assert!(
            v03_estimator_export_rows(&wrong_selection_test)
                .unwrap_err()
                .to_string()
                .contains("linked inference")
        );
    }

    #[test]
    fn typed_power_export_writes_exact_csv_html_xlsx_tables_and_rejects_accounting_tamper() {
        use calamine::{Reader, open_workbook_auto};

        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("power.json");
        let csv_path = directory.path().join("power.csv");
        let html_path = directory.path().join("power.html");
        let xlsx_path = directory.path().join("power.xlsx");
        let result = typed_power_export_fixture();
        fs::write(&result_path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();

        export_result(&result_path, ExportFormat::Csv, Some(&csv_path), false).unwrap();
        export_result(&result_path, ExportFormat::Html, Some(&html_path), false).unwrap();
        export_result(&result_path, ExportFormat::Xlsx, Some(&xlsx_path), false).unwrap();

        let csv = fs::read_to_string(csv_path).unwrap();
        assert!(csv.contains("pls_power_by_sample_size,50,,,,achieved_power,0.01"));
        assert!(csv.contains("pls_power_failure,50,1,,,failure_code,nonconvergence"));
        assert!(csv.contains("pls_power_replicate_ledger,50,0,,,p_value_two_sided,0.04"));
        assert!(csv.contains("standalone_integrity_scope"));
        let html = fs::read_to_string(html_path).unwrap();
        assert!(html.contains("QuickPLS PLS sample-size/power export"));
        assert!(html.contains("cannot recompute recipe-bound digests"));
        let mut workbook = open_workbook_auto(&xlsx_path).unwrap();
        let range = workbook.worksheet_range("QuickPLS export").unwrap();
        assert!(range.rows().any(|row| {
            row.iter()
                .any(|cell| cell.to_string().contains("pls_power_replicate_ledger"))
        }));

        let mut tampered = result;
        let tampered_analysis = {
            let AnalysisPayload::PlsSampleSizePowerV1 { analysis } = &mut tampered.payload else {
                panic!("expected typed power payload");
            };
            analysis["rows"][0]["successful_replicates"] = serde_json::json!(2);
            analysis.clone()
        };
        let error = pls_sample_size_power_export_rows(&tampered, &tampered_analysis).unwrap_err();
        assert!(error.to_string().contains("summary for sample size 50"));
    }

    #[test]
    fn typed_power_v2_export_exposes_exact_tail_accounting_and_rejects_tampering() {
        let result = typed_power_v2_export_fixture();
        let AnalysisPayload::PlsSampleSizePowerV2 { analysis } = &result.payload else {
            panic!("expected typed power v2 payload")
        };
        let rows = pls_sample_size_power_export_rows(&result, analysis).unwrap();
        for (metric, value) in [
            ("bootstrap_requested_replicates", "99"),
            ("bootstrap_usable_replicates", "99"),
            ("bootstrap_failed_replicates", "0"),
            ("bootstrap_two_sided_exceedances", "3"),
        ] {
            assert!(rows.iter().any(|row| {
                row.section == "pls_power_replicate_ledger"
                    && row.construct == "50"
                    && row.indicator == "0"
                    && row.metric == metric
                    && row.value == value
            }));
        }
        assert!(rows.iter().any(|row| {
            row.section == "pls_power_provenance"
                && row.metric == "inference_method"
                && row.value == PLS_SAMPLE_SIZE_POWER_INFERENCE_METHOD_V2
        }));

        let mut changed_exceedance = result.clone();
        let AnalysisPayload::PlsSampleSizePowerV2 { analysis } =
            &mut changed_exceedance.payload
        else {
            unreachable!()
        };
        analysis["outcomes"][0]["bootstrap_two_sided_exceedances"] = serde_json::json!(4);
        let changed_analysis = analysis.clone();
        let error =
            pls_sample_size_power_export_rows(&changed_exceedance, &changed_analysis).unwrap_err();
        assert!(error.to_string().contains("tail accounting"));

        let mut relabeled = result;
        relabeled.provenance.method_version = PLS_SAMPLE_SIZE_POWER_METHOD_VERSION.into();
        let AnalysisPayload::PlsSampleSizePowerV2 { analysis } = &relabeled.payload else {
            unreachable!()
        };
        assert!(
            pls_sample_size_power_export_rows(&relabeled, analysis)
                .unwrap_err()
                .to_string()
                .contains("identity")
        );
    }

    fn cbsem_bootstrap_v2_export_fixture() -> AnalysisResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut result: AnalysisResult = serde_json::from_slice(
            &fs::read(root.join("validation/results/lavaan_one_factor_cfa_quickpls.json")).unwrap(),
        )
        .unwrap();
        result.provenance.method_version.push('+');
        result
            .provenance
            .method_version
            .push_str(qpls_estimation::CBSEM_BOOTSTRAP_METHOD_VERSION_V2);
        result.provenance.settings.workers = 4;
        let successful_replicates = (0..1_000)
            .map(
                |replicate_index| qpls_estimation::CbsemBootstrapWitnessReplicateV2 {
                    replicate_index,
                    sample_indices_sha256: "c".repeat(64),
                    iterations: 12,
                    objective: 0.25,
                    parameter_estimates: vec![0.72],
                },
            )
            .collect();
        let bootstrap = qpls_estimation::CbsemBootstrapAnalysisV2 {
            method_version: qpls_estimation::CBSEM_BOOTSTRAP_METHOD_VERSION_V2.into(),
            algorithm: qpls_estimation::CBSEM_BOOTSTRAP_ALGORITHM_V2.into(),
            interval_method: qpls_estimation::CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2.into(),
            retry_policy: qpls_estimation::CBSEM_BOOTSTRAP_RETRY_POLICY_V2.into(),
            confidence_level: 0.95,
            requested_replicates: 1_000,
            attempted_fits: 1_000,
            usable_replicates: 1_000,
            failed_replicates: 0,
            minimum_usable_fraction: qpls_estimation::CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2,
            minimum_usable_replicates: 1_000,
            max_attempts_per_replicate:
                qpls_estimation::CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2,
            complete_case_sample_size: 120,
            seed: result.provenance.seed,
            stream_token: qpls_estimation::CBSEM_BOOTSTRAP_STREAM_TOKEN_V2.into(),
            inference: qpls_estimation::CbsemBootstrapInferenceV2::Available,
            intervals: vec![qpls_estimation::CbsemBootstrapParameterIntervalV2 {
                parameter: "loading:y1".into(),
                original: 0.70,
                bootstrap_mean: 0.72,
                bias: 0.02,
                standard_error: 0.08,
                percentile_lower: 0.55,
                percentile_upper: 0.87,
                usable_replicates: 1_000,
            }],
            failures: Vec::new(),
            validation_witness: qpls_estimation::CbsemBootstrapValidationWitnessV2 {
                method_version: qpls_estimation::CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2.into(),
                dataset_fingerprint: result.provenance.dataset_fingerprint.clone(),
                recipe_sha256: "d".repeat(64),
                base_result_sha256: "e".repeat(64),
                parameter_names: vec!["loading:y1".into()],
                successful_replicates,
            },
            warnings: vec!["Percentile inference is available.".into()],
        };
        let (AnalysisPayload::PlsPmV1 { estimation, .. }
        | AnalysisPayload::PlsPmV2 { estimation, .. }
        | AnalysisPayload::PlsPmV3 { estimation, .. }) = &mut result.payload
        else {
            panic!("expected PLS-shaped CB-SEM payload");
        };
        estimation["cbsem"]["bootstrap"] = serde_json::json!({
            "method_version": "cbsem_bootstrap_v1",
            "samples": 999,
            "usable_samples": 999,
            "intervals": [],
            "warnings": ["Historical analytical preview only."]
        });
        estimation["cbsem"]["bootstrap_v2"] = serde_json::to_value(bootstrap).unwrap();
        result
    }

    #[test]
    fn cbsem_bootstrap_v2_export_includes_typed_contract_and_preserves_legacy_rows() {
        let mut result = cbsem_bootstrap_v2_export_fixture();
        let rows = experimental_pls_export_rows(&result).unwrap();
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap_v2_setting"
                && row.metric == "outer_workers"
                && row.value == "4"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap_v2_inference"
                && row.metric == "status"
                && row.value == "available"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap_v2_interval"
                && row.construct == "loading:y1"
                && row.metric == "percentile_lower"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap_v2_validation_witness" && row.metric == "recipe_sha256"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap_v2_success_witness"
                && row.construct == "999"
                && row.metric == "parameter_estimates"
        }));
        assert!(rows.iter().any(|row| {
            row.section == "cbsem_bootstrap"
                && row.metric == "method_version"
                && row.value == "cbsem_bootstrap_v1"
        }));

        let (AnalysisPayload::PlsPmV1 { estimation, .. }
        | AnalysisPayload::PlsPmV2 { estimation, .. }
        | AnalysisPayload::PlsPmV3 { estimation, .. }) = &mut result.payload
        else {
            panic!("expected PLS-shaped CB-SEM payload");
        };
        estimation["cbsem"]["bootstrap_v2"]["attempted_fits"] = serde_json::json!(999);
        let error = experimental_pls_export_rows(&result).unwrap_err();
        assert!(error.to_string().contains("settings or accounting"));
    }

    #[test]
    fn export_writes_validated_v03_estimator_csv_and_html_only() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe = directory.path().join("simple_reflective.v3.recipe.json");
        write_migrated_v3_recipe(
            &root.join("validation/fixtures/simple_reflective.recipe.json"),
            &recipe,
        );
        let result_path = directory.path().join("result.json");
        let csv_path = directory.path().join("estimator.csv");
        let html_path = directory.path().join("estimator.html");

        write_runner_result(&recipe, &data, &result_path);
        export_result(&result_path, ExportFormat::Csv, Some(&csv_path), false).unwrap();
        export_result(&result_path, ExportFormat::Html, Some(&html_path), false).unwrap();

        let csv = fs::read_to_string(csv_path).unwrap();
        assert!(csv.starts_with("section,construct,indicator,source,target,metric,value\n"));
        assert!(csv.contains("outer_estimate,x,x1,,,weight,"));
        assert!(csv.contains("outer_estimate,y,y1,,,loading,"));
        assert!(csv.contains("path_coefficient,,,x,y,path_coefficient,"));
        assert!(csv.contains("effect,,,x,y,total,"));
        assert!(csv.contains("r_squared,y,,,,r_squared,"));
        assert!(csv.contains("metadata,,,,,export_scope,"));
        assert!(!csv.contains("cronbach_alpha"));
        // Post-hoc provenance legitimately identifies bootstrap as the source of
        // path significance. The estimator-only boundary is about exported
        // tables, so bind this assertion to section identities rather than
        // suppressing scientifically relevant provenance text.
        let exported_sections = csv
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(',').map(|(section, _)| section))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            exported_sections
                .iter()
                .all(|section| !section.contains("bootstrap"))
        );

        let html = fs::read_to_string(html_path).unwrap();
        assert!(html.contains("QuickPLS v0.3 estimator export"));
        assert!(html.contains("Assessment and resampling artifacts are excluded"));
        assert!(!html.contains("cronbach_alpha"));
    }

    #[test]
    fn export_writes_xlsx_workbook_with_estimator_rows() {
        use calamine::{Reader, open_workbook_auto};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe = directory.path().join("simple_reflective.v3.recipe.json");
        write_migrated_v3_recipe(
            &root.join("validation/fixtures/simple_reflective.recipe.json"),
            &recipe,
        );
        let result_path = directory.path().join("result.json");
        let xlsx_path = directory.path().join("estimator.xlsx");

        write_runner_result(&recipe, &data, &result_path);
        export_result(&result_path, ExportFormat::Xlsx, Some(&xlsx_path), false).unwrap();

        let mut workbook = open_workbook_auto(&xlsx_path).unwrap();
        let range = workbook.worksheet_range("QuickPLS export").unwrap();
        assert_eq!(range.get((0, 0)).unwrap().to_string(), "section");
        assert!(range.rows().any(|row| {
            row.iter()
                .any(|cell| cell.to_string().contains("outer_estimate"))
        }));
    }

    #[test]
    fn export_rejects_legacy_result_payloads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe = directory.path().join("simple_reflective.v3.recipe.json");
        write_migrated_v3_recipe(
            &root.join("validation/fixtures/simple_reflective.recipe.json"),
            &recipe,
        );
        let result_path = directory.path().join("result.json");
        let legacy_path = directory.path().join("legacy.json");

        write_runner_result(&recipe, &data, &result_path);
        let mut result: AnalysisResult =
            serde_json::from_slice(&fs::read(&result_path).unwrap()).unwrap();
        result.payload = AnalysisPayload::Legacy {
            value: serde_json::json!({"unvalidated": true}),
        };
        fs::write(&legacy_path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();

        let error = export_result(&legacy_path, ExportFormat::Csv, None, false).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("legacy result payloads cannot be exported")
        );
    }

    #[test]
    fn export_includes_watermarked_experimental_method_tables_when_requested() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("validation/results/wpls_reference.csv");
        let directory = tempfile::tempdir().unwrap();
        let recipe = directory.path().join("wpls_reference.v3.recipe.json");
        write_migrated_v3_recipe(
            &root.join("validation/results/wpls_reference.recipe.json"),
            &recipe,
        );
        let result_path = directory.path().join("wpls.json");
        let csv_path = directory.path().join("wpls.csv");

        write_runner_result(&recipe, &data, &result_path);
        let conservative_error =
            export_result(&result_path, ExportFormat::Csv, None, false).unwrap_err();
        assert!(
            conservative_error
                .to_string()
                .contains("v0.3 estimator export supports only PLS-SEM results")
        );
        let xlsx_path = directory.path().join("wpls.xlsx");
        export_result(&result_path, ExportFormat::Csv, Some(&csv_path), true).unwrap();
        export_result(&result_path, ExportFormat::Xlsx, Some(&xlsx_path), true).unwrap();

        let csv = fs::read_to_string(csv_path).unwrap();
        assert!(csv.contains("scope_warning"));
        assert!(csv.contains(&format!(
            "wpls,,,,,method_version,{}",
            qpls_estimation::WPLS_METHOD_VERSION
        )));
        assert!(csv.contains("wpls,,,,,case_weight_column,case_wt"));
        assert!(csv.contains("wpls,,,,,effective_sample_size,"));
        assert!(xlsx_path.exists());
    }

    #[test]
    fn demo_project_create_and_validate_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("demo.qpls");
        let expected = directory.path().join("demo.expected.json");
        let validation = directory.path().join("demo.validation.json");
        create_demo_project(Some(&project), Some(&expected)).unwrap();
        let (saved_project, recovery) = load_project_with_autosave(&project).unwrap();
        assert!(recovery.is_none());
        assert_eq!(saved_project.manifest.name, "Corporate Reputation Sample");
        let result = &saved_project.results[0];
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION)
        );
        let AnalysisPayload::PlsPmV3 {
            bootstrap: Some(bootstrap),
            ..
        } = &result.payload
        else {
            panic!("demo must retain its typed bootstrap payload");
        };
        assert!(bootstrap.get("htmt_inference").is_some());

        let mut missing_htmt_marker = saved_project.clone();
        missing_htmt_marker.results[0].provenance.method_version = missing_htmt_marker.results[0]
            .provenance
            .method_version
            .split('+')
            .filter(|version| *version != HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION)
            .collect::<Vec<_>>()
            .join("+");
        let error = save_project(
            &directory.path().join("demo.missing-htmt-marker.qpls"),
            &missing_htmt_marker,
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid_htmt_inference"));

        let mut missing_htmt_payload = saved_project.clone();
        if let AnalysisPayload::PlsPmV3 {
            bootstrap: Some(bootstrap),
            ..
        } = &mut missing_htmt_payload.results[0].payload
        {
            bootstrap.as_object_mut().unwrap().remove("htmt_inference");
        }
        let error = save_project(
            &directory.path().join("demo.missing-htmt-payload.qpls"),
            &missing_htmt_payload,
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid_htmt_inference"));

        validate_demo_project(Some(&project), Some(&expected), Some(&validation)).unwrap();
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(validation).unwrap()).unwrap();
        assert_eq!(report["matches_expected"], true);
        assert_eq!(report["demo_id"], "quickpls_v04_demo");
    }
}

fn inspect_project(path: &Path, json_output: bool) -> Result<()> {
    let (project, recovery_source) = load_project_with_autosave(path)
        .with_context(|| format!("invalid project {}", path.display()))?;
    let recovered = recovery_source.is_some();
    let datasets = project.datasets.iter().map(|dataset| json!({"id": dataset.id, "name": dataset.name, "rows": dataset.schema.case_count, "columns": dataset.schema.columns.len(), "kind": dataset.schema.kind, "sampleSize": dataset.schema.sample_size, "fingerprint": dataset.fingerprint.0})).collect::<Vec<_>>();
    let summary = json!({"schemaVersion": project.manifest.schema_version, "sourceArchiveVersion": project.source_archive_version, "migrationPending": project.migration_pending, "compatibilityNoticeCount": project.compatibility_notices.len(), "futureUnsupported": {"models": project.future_unsupported.models, "recipes": project.future_unsupported.recipes, "results": project.future_unsupported.results}, "projectId": project.manifest.project_id, "name": project.manifest.name, "engineVersion": project.manifest.engine_version, "readOnly": project.read_only, "recovered": recovered, "datasets": datasets, "models": project.models.len(), "recipes": project.recipes.len(), "results": project.results.len()});
    if json_output {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "{}\nschema: {} | source schema: {} | datasets: {} | models: {} | recipes: {} | compatibility notices: {}{}{}",
            project.manifest.name,
            project.manifest.schema_version,
            project.source_archive_version,
            project.datasets.len(),
            project.models.len(),
            project.recipes.len(),
            project.compatibility_notices.len(),
            if project.migration_pending {
                " | migration pending"
            } else {
                ""
            },
            if recovered { " | recovered backup" } else { "" }
        );
    }
    Ok(())
}

fn print_issues(issues: &[qpls_core::ValidationIssue]) {
    for issue in issues {
        println!("{:?} {}: {}", issue.severity, issue.code, issue.message);
    }
}
