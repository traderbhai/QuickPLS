use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, HTMT_ORIGINAL_METHOD_VERSION, HTMT_PLUS_METHOD_VERSION,
    RHO_A_METHOD_VERSION, assess_pls,
};
use qpls_core::{
    AnalysisMethod, AnalysisPayload, AnalysisRecipe, AnalysisResult, AnalysisSettings, Construct,
    GateStatus, METHOD_CAPABILITIES, MeasurementMode, ModelSpec, PROJECT_SCHEMA_VERSION, RunStatus,
    Severity, SliceStatus, StructuralPath, development_slice_registry, validate_recipe,
    validate_slice_registry,
};
use qpls_data::{DataKind, ImportOptions, import_path};
use qpls_project::{Project, load_project_with_autosave, save_project};
use qpls_resampling::{
    PERMUTATION_METHOD_VERSION, RESAMPLING_METHOD_VERSION, ResamplingPhase,
    STUDENTIZED_METHOD_VERSION, bootstrap_pls, permutation_pls,
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
        #[arg(long)]
        allow_experimental: bool,
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
        Command::Methods { json } => {
            if json {
                println!("{}", serde_json::to_string_pretty(METHOD_CAPABILITIES)?);
            } else {
                for method in METHOD_CAPABILITIES {
                    println!(
                        "{:<14} {:<18} {:?}",
                        method.id, method.family, method.status
                    );
                }
            }
            Ok(())
        }
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
    let rows = if include_experimental {
        experimental_pls_export_rows(&result)?
    } else {
        v03_estimator_export_rows(&result)?
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
        if include_experimental {
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
        AnalysisPayload::Legacy { .. } => bail!("legacy result payloads cannot be exported"),
    };
    let mut rows = Vec::new();
    push_metadata_rows(result, &mut rows);
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
        "supplemental method export; values are validated only inside the documented QuickPLS v1.0.0 supported scope".into(),
    ));
    rows.push(row(
        "scope_warning",
        "",
        "",
        "",
        "",
        "publication_status",
        "Validated for the documented QuickPLS v1.0.0 supported scope where covered by publication audits; unsupported or unaudited payload fields remain outside the release scope.".into(),
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
    push_effects(estimation, &mut rows);
    push_r_squared(estimation, &mut rows);
    push_experimental_method_payloads(estimation, &mut rows);
    push_result_diagnostics(result, &mut rows);
    Ok(rows)
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
        push_json_warnings("mga", mga.get("warnings"), rows);
    }
    if let Some(micom) = estimation.get("micom").and_then(|value| value.as_object()) {
        for metric in [
            "method_version",
            "group_column",
            "permutation_samples",
            "usable_permutations",
        ] {
            if let Some(value) = micom.get(metric) {
                rows.push(row("micom", "", "", "", "", metric, json_value(value)));
            }
        }
        if let Some(constructs) = micom.get("constructs").and_then(|value| value.as_array()) {
            for construct in constructs {
                for metric in [
                    "configural_invariance",
                    "compositional_correlation",
                    "compositional_p_value",
                    "mean_difference",
                    "mean_p_value",
                    "variance_difference",
                    "variance_p_value",
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
        (
            "export_scope",
            "v0.3 validated estimator only; assessment and resampling are excluded".into(),
        ),
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
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>QuickPLS v0.3 estimator export</title><style>body{{font-family:Arial,sans-serif;margin:32px;color:#111827}}.notice{{border:1px solid #f59e0b;background:#fffbeb;padding:12px;margin:16px 0}}table{{border-collapse:collapse;width:100%;font-size:13px}}th,td{{border:1px solid #d1d5db;padding:6px;text-align:left}}th{{background:#f3f4f6}}</style></head><body><h1>QuickPLS v0.3 estimator export</h1><p>Result {}</p><div class=\"notice\">Estimator-only export: validated v0.3 PLS core values are included. Assessment and resampling artifacts are excluded until their publication export gates pass.</div><table><thead><tr><th>section</th><th>construct</th><th>indicator</th><th>source</th><th>target</th><th>metric</th><th>value</th></tr></thead><tbody>{}</tbody></table></body></html>",
        html_escape(&result.id.to_string()),
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
        "docs/methods/REGRESSION_LOGISTIC_V1.md",
        "docs/methods/PROCESS_V1.md",
        "docs/methods/NCA_V1.md",
        "docs/methods/GSCA_V1.md",
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
    let recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
    let data = root.join("validation/fixtures/simple_reflective.csv");
    let directory = root.join("target/qualification/v04-inference");
    fs::create_dir_all(&directory)?;
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
    let mut recipe: AnalysisRecipe = serde_json::from_slice(
        &fs::read(&recipe_path)
            .with_context(|| format!("cannot read {}", recipe_path.display()))?,
    )
    .context("invalid cancellation benchmark recipe JSON")?;
    recipe.settings.bootstrap_samples = 10_000;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.permutation_samples = 0;
    recipe.settings.workers = 4;
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
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
    let mut recipe: AnalysisRecipe = serde_json::from_slice(
        &fs::read(&recipe_path)
            .with_context(|| format!("cannot read {}", recipe_path.display()))?,
    )
    .context("invalid studentized cancellation benchmark recipe JSON")?;
    recipe.settings.bootstrap_samples = 999;
    recipe.settings.studentized_inner_samples = 99;
    recipe.settings.permutation_samples = 0;
    recipe.settings.workers = 4;
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    base_recipe.settings.studentized_inner_samples = 0;
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
        schema_version: PROJECT_SCHEMA_VERSION,
        id: "00000000-0000-0000-0000-00000000d004"
            .parse()
            .expect("fixed demo recipe UUID is valid"),
        created_at: chrono::DateTime::parse_from_rfc3339("2026-07-18T00:00:00Z")
            .unwrap()
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
    let mut project = Project::new("QuickPLS v0.4 Demo Evidence Project");
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
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    base_recipe.settings.permutation_samples = 0;
    let estimation = qpls_estimation::estimate_pls(dataset, &base_recipe)
        .context("demo PLS estimation failed")?;
    let assessment =
        assess_pls(dataset, &base_recipe, &estimation).context("demo PLS assessment failed")?;
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
            "{}+{}+{}+{}+{}",
            qpls_estimation::PLS_METHOD_VERSION,
            qpls_estimation::PLS_MEDIATION_METHOD_VERSION,
            ASSESSMENT_METHOD_VERSION,
            RESAMPLING_METHOD_VERSION,
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
    bootstrap_samples: Option<u32>,
    studentized_inner_samples: Option<u32>,
    permutation_samples: Option<u32>,
    workers: Option<usize>,
) -> Result<()> {
    let requests_experimental_inference = bootstrap_samples.unwrap_or(0) > 0
        || studentized_inner_samples.unwrap_or(0) > 0
        || permutation_samples.unwrap_or(0) > 0;
    if !allow_experimental && requests_experimental_inference {
        bail!(
            "PLS inference add-ons are experimental; rerun with --allow-experimental after reviewing the validation status"
        );
    }
    let (dataset, mut recipe) = if input
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("qpls"))
    {
        let (project, _) = load_project_with_autosave(input)
            .with_context(|| format!("invalid project {}", input.display()))?;
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
    let envelope = qpls_runner::run_pls_analysis(&dataset, &recipe, || false, |_| {})
        .map_err(anyhow::Error::from)?;
    fs::write(output, serde_json::to_vec_pretty(&envelope)?)
        .with_context(|| format!("cannot write {}", output.display()))?;
    println!("wrote analysis result {}", output.display());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_analysis_payload_is_exactly_worker_invariant() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let serial_path = directory.path().join("serial.json");
        let parallel_path = directory.path().join("parallel.json");
        run_analysis(
            &recipe,
            Some(&data),
            None,
            &serial_path,
            true,
            Some(24),
            None,
            Some(99),
            Some(1),
        )
        .unwrap();
        run_analysis(
            &recipe,
            Some(&data),
            None,
            &parallel_path,
            true,
            Some(24),
            None,
            Some(99),
            Some(4),
        )
        .unwrap();
        let serial: AnalysisResult =
            serde_json::from_slice(&fs::read(serial_path).unwrap()).unwrap();
        let parallel: AnalysisResult =
            serde_json::from_slice(&fs::read(parallel_path).unwrap()).unwrap();
        assert_eq!(serial.payload, parallel.payload);
        assert_eq!(serial.diagnostics, parallel.diagnostics);
        assert_eq!(
            serial.provenance.method_version,
            parallel.provenance.method_version
        );
        assert_eq!(serial.provenance.settings.workers, 1);
        assert_eq!(parallel.provenance.settings.workers, 4);
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
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["file"] == "validation/results/wpls_reference_report.json")
        );
    }

    #[test]
    fn export_writes_validated_v03_estimator_csv_and_html_only() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("result.json");
        let csv_path = directory.path().join("estimator.csv");
        let html_path = directory.path().join("estimator.html");

        run_analysis(
            &recipe,
            Some(&data),
            None,
            &result_path,
            true,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap();
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
        assert!(!csv.contains("bootstrap"));

        let html = fs::read_to_string(html_path).unwrap();
        assert!(html.contains("QuickPLS v0.3 estimator export"));
        assert!(html.contains("Assessment and resampling artifacts are excluded"));
        assert!(!html.contains("cronbach_alpha"));
    }

    #[test]
    fn export_writes_xlsx_workbook_with_estimator_rows() {
        use calamine::{Reader, open_workbook_auto};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("result.json");
        let xlsx_path = directory.path().join("estimator.xlsx");

        run_analysis(
            &recipe,
            Some(&data),
            None,
            &result_path,
            true,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap();
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
        let recipe = root.join("validation/fixtures/simple_reflective.recipe.json");
        let data = root.join("validation/fixtures/simple_reflective.csv");
        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("result.json");
        let legacy_path = directory.path().join("legacy.json");

        run_analysis(
            &recipe,
            Some(&data),
            None,
            &result_path,
            true,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap();
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
        let recipe = root.join("validation/results/wpls_reference.recipe.json");
        let data = root.join("validation/results/wpls_reference.csv");
        let directory = tempfile::tempdir().unwrap();
        let result_path = directory.path().join("wpls.json");
        let csv_path = directory.path().join("wpls.csv");

        run_analysis(
            &recipe,
            Some(&data),
            None,
            &result_path,
            true,
            None,
            None,
            None,
            Some(1),
        )
        .unwrap();
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
        assert!(csv.contains("documented QuickPLS v1.0.0 supported scope"));
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
    let summary = json!({"schemaVersion": project.manifest.schema_version, "projectId": project.manifest.project_id, "name": project.manifest.name, "engineVersion": project.manifest.engine_version, "readOnly": project.read_only, "recovered": recovered, "datasets": datasets, "models": project.models.len(), "recipes": project.recipes.len(), "results": project.results.len()});
    if json_output {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "{}\nschema: {} | datasets: {} | models: {} | recipes: {}{}",
            project.manifest.name,
            project.manifest.schema_version,
            project.datasets.len(),
            project.models.len(),
            project.recipes.len(),
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
