use anyhow::{Context, Result, bail};
use chrono::{TimeZone, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisPayload, AnalysisRecipe,
    AnalysisSettings, Construct, MeasurementMode, MethodConfig, MissingDataPolicy, ModelSpec,
    Preprocessing, RunStatus, Severity, StructuralPath, WeightingScheme, validate_recipe,
};
use qpls_data::{ColumnType, ImportOptions, import_path};
use qpls_estimation::PlsResult;
use qpls_project::{Project, load_project, save_project};
use qpls_runner::run_pls_analysis;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

const EXPECTED_OBSERVATIONS: usize = 344;
const EXPECTED_PARAMETERS: usize = 48;
const EXPECTED_ITERATIONS: u32 = 8;
const EXPECTED_SOURCE_SHA256: &str =
    "45373b5177b19352d146c0a3b9bc66d58255744b795e453974d1248f816db9cb";
const EXPECTED_SCREENSHOT_SHA256: &str =
    "2d604f98aaeb618469486be3bfea55cc57d1901ad49cab8b5d97dd17504f9ed9";

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn load_json(path: &Path) -> Result<(Value, String)> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let value = serde_json::from_slice(&bytes)
        .with_context(|| format!("cannot parse {}", path.display()))?;
    Ok((value, hash))
}

fn required_object<'a>(value: &'a Value, key: &str) -> Result<&'a Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_object)
        .with_context(|| format!("reference field {key} must be an object"))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("reference field {key} must be text"))
}

fn construct_name(id: &str, short_name: &str) -> String {
    match id {
        "qual" => "Quality".into(),
        "perf" => "Performance".into(),
        "csor" => "Corporate social responsibility".into(),
        "attr" => "Attractiveness".into(),
        "comp" => "Competence".into(),
        "like" => "Likeability".into(),
        "cusa" => "Customer satisfaction".into(),
        "cusl" => "Customer loyalty".into(),
        _ => short_name.into(),
    }
}

fn model_from_reference(reference: &Value) -> Result<ModelSpec> {
    let model = required_object(reference, "model")?;
    let constructs = model
        .get("constructs")
        .and_then(Value::as_array)
        .context("reference model.constructs must be an array")?
        .iter()
        .map(|value| {
            let id = required_str(value, "id")?;
            let short_name = required_str(value, "short_name")?;
            let mode = match required_str(value, "mode")? {
                "formative" => MeasurementMode::Formative,
                "reflective" | "reflective_single_item" => MeasurementMode::Reflective,
                other => bail!("unsupported reference measurement mode {other}"),
            };
            let indicators = value
                .get("indicators")
                .and_then(Value::as_array)
                .context("reference construct indicators must be an array")?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(ToOwned::to_owned)
                        .context("reference indicator must be text")
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Construct {
                id: id.into(),
                name: construct_name(id, short_name),
                short_name: short_name.into(),
                mode,
                indicators,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let paths = model
        .get("paths")
        .and_then(Value::as_array)
        .context("reference model.paths must be an array")?
        .iter()
        .map(|value| {
            let value = value.as_str().context("reference path must be text")?;
            let (source, target) = value
                .split_once("->")
                .with_context(|| format!("reference path {value} must use source->target"))?;
            Ok(StructuralPath {
                source: source.into(),
                target: target.into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if constructs.len() != 8 || paths.len() != 13 {
        bail!(
            "reference model must contain 8 constructs and 13 paths; found {} and {}",
            constructs.len(),
            paths.len()
        );
    }
    Ok(ModelSpec {
        id: "00000000-0000-4000-8000-00000000c001".parse().unwrap(),
        name: "SmartPLS Corporate Reputation benchmark".into(),
        constructs,
        paths,
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    })
}

fn reference_values(reference: &Value) -> Result<BTreeMap<String, f64>> {
    let values = required_object(reference, "values")?
        .iter()
        .map(|(parameter, value)| {
            let value = value
                .as_f64()
                .filter(|value| value.is_finite())
                .with_context(|| format!("reference value {parameter} must be finite"))?;
            Ok((parameter.clone(), value))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    if values.len() != EXPECTED_PARAMETERS {
        bail!(
            "reference must contain {EXPECTED_PARAMETERS} values; found {}",
            values.len()
        );
    }
    let family_counts = ["path:", "r2:", "weight:", "loading:"]
        .map(|prefix| values.keys().filter(|key| key.starts_with(prefix)).count());
    if family_counts != [13, 4, 21, 10] {
        bail!("reference parameter family counts are not 13 paths, 4 R2, 21 weights, 10 loadings");
    }
    Ok(values)
}

fn insert_unique(values: &mut BTreeMap<String, f64>, key: String, value: f64) -> Result<()> {
    if !value.is_finite() {
        bail!("QuickPLS produced a non-finite value for {key}");
    }
    if values.insert(key.clone(), value).is_some() {
        bail!("QuickPLS produced a duplicate parameter {key}");
    }
    Ok(())
}

fn quickpls_values(estimation: &PlsResult, model: &ModelSpec) -> Result<BTreeMap<String, f64>> {
    let mut values = BTreeMap::new();
    for estimate in &estimation.paths {
        insert_unique(
            &mut values,
            format!("path:{}->{}", estimate.source, estimate.target),
            estimate.coefficient,
        )?;
    }
    for (construct, value) in &estimation.r_squared {
        insert_unique(&mut values, format!("r2:{construct}"), *value)?;
    }
    for estimate in &estimation.outer_estimates {
        let construct = model
            .constructs
            .iter()
            .find(|construct| construct.id == estimate.construct)
            .with_context(|| {
                format!(
                    "QuickPLS returned an outer estimate for unknown construct {}",
                    estimate.construct
                )
            })?;
        if construct.mode == MeasurementMode::Formative {
            insert_unique(
                &mut values,
                format!("weight:{}:{}", estimate.construct, estimate.indicator),
                estimate.weight,
            )?;
        } else {
            insert_unique(
                &mut values,
                format!("loading:{}:{}", estimate.construct, estimate.indicator),
                estimate.loading,
            )?;
        }
    }
    Ok(values)
}

fn rounded_three(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn comparison(
    estimation: &PlsResult,
    model: &ModelSpec,
    expected: &BTreeMap<String, f64>,
) -> Result<Value> {
    let actual = quickpls_values(estimation, model)?;
    let actual_keys = actual.keys().cloned().collect::<BTreeSet<_>>();
    let expected_keys = expected.keys().cloned().collect::<BTreeSet<_>>();
    if actual_keys != expected_keys {
        let missing = expected_keys.difference(&actual_keys).collect::<Vec<_>>();
        let unexpected = actual_keys.difference(&expected_keys).collect::<Vec<_>>();
        bail!("selected parameter keys differ; missing={missing:?}, unexpected={unexpected:?}");
    }
    let rows = expected
        .iter()
        .map(|(parameter, expected)| {
            let actual = actual
                .get(parameter)
                .copied()
                .with_context(|| format!("QuickPLS result is missing {parameter}"))?;
            let absolute_difference = (actual - expected).abs();
            Ok(json!({
                "parameter": parameter,
                "smartpls_displayed": expected,
                "quickpls": actual,
                "absolute_difference_from_displayed_center": absolute_difference,
                "distance_outside_display_rounding_interval": (absolute_difference - 0.0005).max(0.0),
                "rounded_to_3_decimals_matches": rounded_three(actual) == *expected,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let matched = rows
        .iter()
        .filter(|row| row["rounded_to_3_decimals_matches"] == Value::Bool(true))
        .count();
    Ok(json!({
        "comparison_basis": "The SmartPLS reference is published to three decimals. A match means the full-precision QuickPLS estimate rounds to the identical displayed value.",
        "rounding_interval_half_width": 0.0005,
        "matched": matched,
        "total": rows.len(),
        "all_match": matched == rows.len(),
        "rows": rows,
    }))
}

fn validate_preprocessing_receipt(receipt: &Value, cleaned_sha256: &str) -> Result<Vec<String>> {
    let source = required_object(receipt, "source")?;
    let selection = required_object(receipt, "selection")?;
    let output = required_object(receipt, "output")?;
    let cleaning = required_object(receipt, "cleaning")?;
    let expected_missing = json!({"cusl_1": 3, "cusl_2": 4, "cusl_3": 3, "cusa": 1});
    if source.get("sha256").and_then(Value::as_str) != Some(EXPECTED_SOURCE_SHA256)
        || source.get("sheet").and_then(Value::as_str) != Some("Sheet2")
        || source.get("rows").and_then(Value::as_u64) != Some(EXPECTED_OBSERVATIONS as u64)
        || source.get("columns").and_then(Value::as_u64) != Some(41)
        || source.get("all_cells_numeric").and_then(Value::as_bool) != Some(true)
        || source.get("duplicate_rows").and_then(Value::as_u64) != Some(0)
        || output.get("sha256").and_then(Value::as_str) != Some(cleaned_sha256)
        || output.get("rows").and_then(Value::as_u64) != Some(EXPECTED_OBSERVATIONS as u64)
        || output.get("columns").and_then(Value::as_u64) != Some(31)
        || output.get("missing_cells").and_then(Value::as_u64) != Some(0)
        || cleaning.get("missing_marker").and_then(Value::as_i64) != Some(-99)
        || cleaning.get("missing_cells").and_then(Value::as_u64) != Some(11)
        || cleaning.get("rows_with_missing").and_then(Value::as_u64) != Some(8)
        || cleaning.get("treatment").and_then(Value::as_str) != Some("indicator_mean_replacement")
        || cleaning.get("missing_by_column") != Some(&expected_missing)
    {
        bail!("preprocessing receipt does not match the frozen Corporate Reputation contract");
    }
    let model_columns = selection
        .get("model_columns")
        .and_then(Value::as_array)
        .context("preprocessing receipt selection.model_columns must be an array")?
        .iter()
        .map(|column| {
            column
                .as_str()
                .map(ToOwned::to_owned)
                .context("preprocessing receipt model column must be text")
        })
        .collect::<Result<Vec<_>>>()?;
    if model_columns.len() != 31
        || model_columns.iter().collect::<BTreeSet<_>>().len() != model_columns.len()
    {
        bail!("preprocessing receipt must identify 31 unique model columns");
    }
    Ok(model_columns)
}

fn validate_reference_contract(reference: &Value) -> Result<()> {
    let settings = required_object(reference, "settings")?;
    if reference.get("schema_version").and_then(Value::as_u64) != Some(1)
        || reference
            .get("reference_precision_decimals")
            .and_then(Value::as_u64)
            != Some(3)
        || reference.get("screenshot_sha256").and_then(Value::as_str)
            != Some(EXPECTED_SCREENSHOT_SHA256)
        || settings.get("method").and_then(Value::as_str) != Some("ordinary_pls_sem")
        || settings.get("weighting_scheme").and_then(Value::as_str) != Some("path")
        || settings.get("preprocessing").and_then(Value::as_str) != Some("standardized")
        || settings
            .get("initial_outer_weights")
            .and_then(Value::as_f64)
            != Some(1.0)
        || settings.get("maximum_iterations").and_then(Value::as_u64) != Some(3_000)
        || settings.get("stop_criterion").and_then(Value::as_f64) != Some(1e-7)
        || settings.get("missing_data").and_then(Value::as_str)
            != Some("indicator_mean_replacement")
        || settings.get("expected_iterations").and_then(Value::as_u64)
            != Some(EXPECTED_ITERATIONS as u64)
        || settings.get("observations").and_then(Value::as_u64)
            != Some(EXPECTED_OBSERVATIONS as u64)
    {
        bail!("SmartPLS reference settings or source identity differ from the frozen contract");
    }
    Ok(())
}

fn validate_clean_csv(path: &Path, expected_columns: &[String]) -> Result<()> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("cannot read cleaned CSV {}", path.display()))?;
    let mut lines = contents.lines();
    let header = lines.next().context("cleaned CSV is empty")?;
    let columns = header
        .trim_end_matches('\r')
        .split(',')
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if columns != expected_columns {
        bail!("cleaned CSV header differs from the preprocessing receipt");
    }
    let mut rows = 0usize;
    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            bail!(
                "cleaned CSV contains a blank row at line {}",
                line_index + 2
            );
        }
        let cells = line.trim_end_matches('\r').split(',').collect::<Vec<_>>();
        if cells.len() != expected_columns.len() {
            bail!(
                "cleaned CSV line {} has {} cells instead of {}",
                line_index + 2,
                cells.len(),
                expected_columns.len()
            );
        }
        for (column, cell) in expected_columns.iter().zip(cells) {
            let value = cell.parse::<f64>().with_context(|| {
                format!(
                    "cleaned CSV line {} column {column} is not numeric",
                    line_index + 2
                )
            })?;
            if !value.is_finite() || !(1.0..=7.0).contains(&value) {
                bail!(
                    "cleaned CSV line {} column {column} is not a finite 1..7 response",
                    line_index + 2
                );
            }
        }
        rows += 1;
    }
    if rows != EXPECTED_OBSERVATIONS {
        bail!("cleaned CSV has {rows} observations instead of {EXPECTED_OBSERVATIONS}");
    }
    Ok(())
}

fn build_recipe(
    dataset_fingerprint: &str,
    model: ModelSpec,
    cleaned_sha256: &str,
    receipt_sha256: &str,
    reference_sha256: &str,
) -> AnalysisRecipe {
    let mut settings = AnalysisSettings::default();
    settings.method = AnalysisMethod::PlsPm;
    settings.weighting_scheme = WeightingScheme::Path;
    settings.tolerance = 1e-7;
    settings.max_iterations = 3_000;
    settings.bootstrap_samples = 0;
    settings.studentized_inner_samples = 0;
    settings.permutation_samples = 0;
    settings.seed = 20_260_823;
    settings.workers = 1;
    settings.confidence_level = 0.95;
    settings.preprocessing = Preprocessing::Standardized;
    settings.missing_data = MissingDataPolicy::ListwiseDeletion;
    settings.case_weight_column = None;
    let metadata = BTreeMap::from([
        ("benchmark".into(), "smartpls_corporate_reputation".into()),
        ("cleaned_csv_sha256".into(), cleaned_sha256.into()),
        ("preprocessing_receipt_sha256".into(), receipt_sha256.into()),
        ("smartpls_reference_sha256".into(), reference_sha256.into()),
        ("source_missing_marker".into(), "-99".into()),
        (
            "source_missing_treatment".into(),
            "external_indicator_mean_replacement_before_quickpls_import".into(),
        ),
        (
            "source_observations".into(),
            EXPECTED_OBSERVATIONS.to_string(),
        ),
        (
            "status".into(),
            "external_display_precision_parity_benchmark".into(),
        ),
    ]);
    AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: "00000000-0000-4000-8000-00000000c002".parse().unwrap(),
        created_at: Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap(),
        dataset_fingerprint: dataset_fingerprint.into(),
        model,
        settings,
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata,
    }
}

fn output_path(path: &Path, extension: &str) -> Result<PathBuf> {
    if path.extension().and_then(|value| value.to_str()) != Some(extension) {
        bail!("output {} must use .{extension}", path.display());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    let file_name = absolute
        .file_name()
        .context("output path must include a file name")?;
    let parent = absolute
        .parent()
        .context("output path must have a parent")?;
    fs::create_dir_all(parent)?;
    Ok(fs::canonicalize(parent)?.join(file_name))
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

fn main() -> Result<()> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 5 {
        bail!(
            "usage: corporate_reputation_benchmark <mean-replaced.csv> <preprocessing-receipt.json> <smartpls-reference.json> <output.qpls> <report.json>"
        );
    }
    let data_path = fs::canonicalize(PathBuf::from(&arguments[0]))?;
    let receipt_path = fs::canonicalize(PathBuf::from(&arguments[1]))?;
    let reference_path = fs::canonicalize(PathBuf::from(&arguments[2]))?;
    let project_path = output_path(&PathBuf::from(&arguments[3]), "qpls")?;
    let report_path = output_path(&PathBuf::from(&arguments[4]), "json")?;
    let mut all_paths = vec![
        data_path.clone(),
        receipt_path.clone(),
        reference_path.clone(),
        project_path.clone(),
        report_path.clone(),
    ];
    all_paths.sort();
    all_paths.dedup();
    if all_paths.len() != 5 {
        bail!("benchmark input and output paths must be distinct");
    }

    let cleaned_sha256 = sha256_file(&data_path)?;
    let (receipt, receipt_sha256) = load_json(&receipt_path)?;
    let receipt_model_columns = validate_preprocessing_receipt(&receipt, &cleaned_sha256)?;
    validate_clean_csv(&data_path, &receipt_model_columns)?;
    let (reference, reference_sha256) = load_json(&reference_path)?;
    validate_reference_contract(&reference)?;
    let expected = reference_values(&reference)?;
    let model = model_from_reference(&reference)?;
    let model_columns = model
        .constructs
        .iter()
        .flat_map(|construct| construct.indicators.iter().cloned())
        .collect::<Vec<_>>();
    if model_columns.iter().collect::<BTreeSet<_>>()
        != receipt_model_columns.iter().collect::<BTreeSet<_>>()
    {
        bail!("reference model indicators differ from the preprocessing receipt");
    }

    let dataset = import_path(&data_path, &ImportOptions::default())
        .with_context(|| format!("cannot import {}", data_path.display()))?;
    let imported_columns = dataset
        .schema
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    if dataset.schema.case_count != EXPECTED_OBSERVATIONS
        || imported_columns != receipt_model_columns
        || dataset
            .schema
            .columns
            .iter()
            .any(|column| column.column_type != ColumnType::Numeric)
        || dataset
            .batch
            .columns()
            .iter()
            .any(|column| column.null_count() != 0)
    {
        bail!("cleaned dataset does not match the 344-row, 31-numeric-column benchmark contract");
    }

    let recipe = build_recipe(
        &dataset.fingerprint.0,
        model.clone(),
        &cleaned_sha256,
        &receipt_sha256,
        &reference_sha256,
    );
    let issues = validate_recipe(&recipe);
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        bail!("benchmark recipe has validation errors: {errors:?}");
    }
    let warning_codes = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Warning)
        .map(|issue| issue.code)
        .collect::<Vec<_>>();
    if warning_codes != ["construct.single_item"] {
        bail!("unexpected recipe warnings: {warning_codes:?}");
    }

    let result = run_pls_analysis(&dataset, &recipe, || false, |_| {})
        .context("QuickPLS product runner failed")?;
    if result.status != RunStatus::Completed {
        bail!("QuickPLS result is not completed");
    }
    let estimation_value = match &result.payload {
        AnalysisPayload::PlsPmV1 { estimation, .. } => estimation.clone(),
        other => bail!("QuickPLS returned an unexpected result payload: {other:?}"),
    };
    let estimation: PlsResult = serde_json::from_value(estimation_value)?;
    let comparison = comparison(&estimation, &model, &expected)?;
    if !estimation.converged
        || estimation.iterations != EXPECTED_ITERATIONS
        || estimation.used_observations != EXPECTED_OBSERVATIONS
        || estimation.omitted_observations != 0
        || !estimation.warnings.is_empty()
        || comparison["matched"].as_u64() != Some(EXPECTED_PARAMETERS as u64)
        || comparison["all_match"] != Value::Bool(true)
    {
        bail!(
            "parity gate failed: converged={} iterations={} used={} omitted={} warnings={} matches={}/{}",
            estimation.converged,
            estimation.iterations,
            estimation.used_observations,
            estimation.omitted_observations,
            estimation.warnings.len(),
            comparison["matched"],
            comparison["total"]
        );
    }

    let dataset_id = dataset.id.to_string();
    let model_id = model.id.to_string();
    let recipe_id = recipe.id.to_string();
    let result_id = result.id.to_string();
    let mut project = Project::new("SmartPLS Corporate Reputation benchmark");
    project.datasets.push(dataset);
    project.models.push(model);
    project
        .append_validated_result(recipe.clone(), result.clone())
        .context("cannot append validated benchmark result")?;
    project.layouts.insert(
        "workspace".into(),
        json!({
            "activeDatasetId": dataset_id,
            "activeModelId": model_id,
            "activeRecipeId": recipe_id,
            "activeRunId": result_id,
            "selectedRunId": result_id,
            "analysisSettings": {
                "method": "pls_pm",
                "weightingScheme": "path",
                "tolerance": 1e-7,
                "maxIterations": 3000,
                "bootstrapSamples": 0,
                "studentizedInnerSamples": 0,
                "permutationSamples": 0,
                "seed": 20260823,
                "workers": 1,
                "confidenceLevel": 0.95,
                "preprocessing": "standardized",
                "missingData": "listwise_deletion"
            },
            "diagramOverlaySettings": {"selectedRunId": result_id}
        }),
    );
    project.layouts.insert(
        "workspace_explorer".into(),
        json!({
            "schemaVersion": 1,
            "modelPresentations": {
                model_id.clone(): {
                    "nodes": [
                        {"id": "perf", "position": {"x": 100, "y": 280}},
                        {"id": "csor", "position": {"x": 100, "y": 620}},
                        {"id": "qual", "position": {"x": 460, "y": 80}},
                        {"id": "attr", "position": {"x": 460, "y": 850}},
                        {"id": "comp", "position": {"x": 900, "y": 300}},
                        {"id": "like", "position": {"x": 900, "y": 650}},
                        {"id": "cusa", "position": {"x": 1260, "y": 450}},
                        {"id": "cusl", "position": {"x": 1580, "y": 450}}
                    ]
                }
            },
            "savedReports": [{
                "resultId": result_id,
                "name": "SmartPLS parity result - 48 of 48",
                "savedAt": result.provenance.completed_at.to_rfc3339()
            }]
        }),
    );
    project.layouts.insert(
        "data_lineage".into(),
        json!({
            "schemaVersion": 1,
            "records": [{
                "datasetId": dataset_id,
                "parentDatasetId": null,
                "operation": "import",
                "createdAt": null,
                "summary": "Corporate Reputation: -99 recognized as missing and replaced by indicator means before QuickPLS import",
                "sourceColumn": null,
                "targetColumn": null
            }]
        }),
    );
    save_project(&project_path, &project)
        .with_context(|| format!("cannot save {}", project_path.display()))?;
    let reopened = load_project(&project_path)
        .with_context(|| format!("cannot reopen {}", project_path.display()))?;
    if reopened.datasets.len() != 1
        || reopened.models.len() != 1
        || reopened.recipes.len() != 1
        || reopened.results.len() != 1
        || reopened.datasets[0].fingerprint.0 != recipe.dataset_fingerprint
        || reopened.models[0].id.to_string() != model_id
        || reopened.recipes[0].id.to_string() != recipe_id
        || reopened.results[0].id.to_string() != result_id
        || reopened.results[0].status != RunStatus::Completed
    {
        bail!("reopened project did not preserve the benchmark authority");
    }

    let report = json!({
        "schema_version": 2,
        "benchmark": "SmartPLS Corporate Reputation",
        "evidence_boundary": "SmartPLS values are published to three decimals; this report proves identical displayed values at that precision, not bitwise identity with unexported SmartPLS internals.",
        "source": {
            "cleaned_data_path": data_path,
            "cleaned_data_sha256": cleaned_sha256,
            "preprocessing_receipt_path": receipt_path,
            "preprocessing_receipt_sha256": receipt_sha256,
            "preprocessing_receipt": receipt,
            "smartpls_reference_path": reference_path,
            "smartpls_reference_sha256": reference_sha256,
            "smartpls_reference": reference,
        },
        "quickpls": {
            "engine_version": result.provenance.engine_version,
            "method_version": result.provenance.method_version,
            "dataset_fingerprint": recipe.dataset_fingerprint,
            "model_id": model_id,
            "recipe_id": recipe_id,
            "result_id": result_id,
            "converged": estimation.converged,
            "iterations": estimation.iterations,
            "used_observations": estimation.used_observations,
            "omitted_observations": estimation.omitted_observations,
            "settings": recipe.settings,
            "initialization": "standard +1 outer weights",
            "stored_missing_data_policy": "listwise_deletion_on_already_mean-replaced_complete_data",
            "recipe_warning_codes": warning_codes,
            "estimation": estimation,
        },
        "project": {
            "path": project_path,
            "schema_version": reopened.manifest.schema_version,
            "reopen_verified": true,
            "datasets": reopened.datasets.len(),
            "models": reopened.models.len(),
            "recipes": reopened.recipes.len(),
            "results": reopened.results.len(),
        },
        "comparison": comparison,
    });
    write_json(&report_path, &report)?;

    println!(
        "converged=true iterations={EXPECTED_ITERATIONS} used={EXPECTED_OBSERVATIONS} omitted=0 matches={EXPECTED_PARAMETERS}/{EXPECTED_PARAMETERS} project={} report={}",
        project_path.display(),
        report_path.display(),
    );
    Ok(())
}
