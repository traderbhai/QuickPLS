use qpls_core::{
    ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
    AnalysisRecipeV4, AnalysisSettings, CbsemEstimator, CbsemInput, CbsemModelType,
    LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4, MethodConfig,
    MissingDataPolicyV4, Preprocessing, RecipeV4CompilerTarget, SemDataBindingV4, SemVariableV4,
    compile_analysis_recipe_v4, convert_legacy_basic_model_v4,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_estimation::{
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1, CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CbsemCompiledMomentErrorV2, CbsemExactCaseBootstrapFailureKindV1,
    CbsemExactParameterTableErrorV3,
    estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control,
    prepare_cbsem_ml_exact_case_bootstrap_source_v1,
};
use qpls_resampling::{
    CbsemExactCaseBootstrapAttemptErrorV1, CbsemExactCaseBootstrapScheduleV1,
    CbsemExactCaseBootstrapSchedulerErrorV1, ResamplingError,
    run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

const KIND: &str = "cbsem_exact_case_bootstrap_studentized_benchmark_run_v1";
const FIXTURE_DATA: &str = "validation/results/v07_cbsem.csv";
const FIXTURE_RECIPE: &str = "validation/results/v07_cfa.recipe.json";

#[derive(Debug)]
struct Args {
    repo_root: PathBuf,
    output: PathBuf,
    rows: usize,
    factors: usize,
    replicates: u32,
    workers: usize,
    seed: u64,
    cancel_after: Option<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let data_path = args.repo_root.join(FIXTURE_DATA);
    let recipe_path = args.repo_root.join(FIXTURE_RECIPE);
    let source_data = fs::read_to_string(&data_path)?;
    let mut lines = source_data.lines();
    let header = lines.next().ok_or("benchmark CSV has no header")?;
    let rows = lines.take(args.rows).collect::<Vec<_>>();
    if rows.len() != args.rows {
        return Err(format!(
            "requested {} rows but the source-bound fixture has only {}",
            args.rows,
            rows.len()
        )
        .into());
    }
    let truncated = format!("{header}\n{}\n", rows.join("\n"));
    let mut dataset = import_delimited_bytes(
        truncated.as_bytes(),
        "cbsem-exact-studentized-benchmark.csv",
        b',',
        &ImportOptions::default(),
    )?;
    dataset.id = uuid::Uuid::from_u128(0xCB5E_B311_0000_0000_0000_0000_0000_0001);

    let fixture: Value = serde_json::from_slice(&fs::read(&recipe_path)?)?;
    let mut legacy_model: qpls_core::ModelSpec = serde_json::from_value(
        fixture
            .get("model")
            .cloned()
            .ok_or("fixture has no model")?,
    )?;
    if !(1..=legacy_model.constructs.len()).contains(&args.factors) {
        return Err(format!(
            "factors must be between 1 and {} for the source-bound fixture",
            legacy_model.constructs.len()
        )
        .into());
    }
    legacy_model.constructs.truncate(args.factors);
    legacy_model.paths.clear();
    let mut model = convert_legacy_basic_model_v4(
        &legacy_model,
        LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        &[],
    )?;
    model.data_binding = SemDataBindingV4::Raw {
        dataset_id: dataset.id.to_string(),
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        weight: None,
        cluster_variable: None,
        strata_variable: None,
    };
    model.ensure_valid()?;
    let observed_variables = model
        .variables
        .iter()
        .filter(|variable| matches!(variable, SemVariableV4::Observed { .. }))
        .count();

    let mut settings = AnalysisSettings::default();
    settings.method = AnalysisMethod::Cbsem;
    settings.preprocessing = Preprocessing::Unstandardized;
    // Keep the compiled point-estimation authority identical while the
    // benchmark varies only the exact-bootstrap scheduler worker count.
    settings.workers = 1;
    let recipe = AnalysisRecipeV4 {
        schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
        id: uuid::Uuid::from_u128(0xCB5E_B311_0000_0000_0000_0000_0000_0002),
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0)
            .ok_or("invalid fixed benchmark timestamp")?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256()?,
            model: model.clone(),
        },
        estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
        settings,
        method_config: Some(MethodConfig::Cbsem {
            model_type: CbsemModelType::Cfa,
            estimator: CbsemEstimator::Ml,
            input: CbsemInput::Raw,
            mean_structure: false,
            bootstrap_samples: 0,
            bootstrap_v2: None,
            group_column: None,
            invariance_steps: Vec::new(),
        }),
        general_sem_config: None,
        metadata: BTreeMap::new(),
        legacy_source: None,
    };
    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let artifact =
        compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())?;
    let source =
        prepare_cbsem_ml_exact_case_bootstrap_source_v1(&dataset, &artifact, &recipe, &model)?;
    if source.complete_case_sample_size() != args.rows {
        return Err("source-bound benchmark fixture unexpectedly lost complete cases".into());
    }
    let sampling_frame = (0..args.rows).collect::<Vec<_>>();
    let identity = sampling_frame.clone();
    let original =
        estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
            &source,
            &identity,
            || false,
            |_| {},
        )?;
    let base_point_result_sha256 = sha256(&serde_json::to_vec(&original.refit)?);
    let schedule = CbsemExactCaseBootstrapScheduleV1 {
        outer_recipe_analytical_identity_sha256: artifact.receipt().recipe_analytical_sha256(),
        base_point_result_sha256: &base_point_result_sha256,
        requested_replicates: args.replicates,
        seed: args.seed,
        stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
        retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
        max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
        hypothesis_test: None,
    };

    let cancelled = AtomicBool::new(false);
    let cancellation_started = Mutex::new(None::<Instant>);
    let started = Instant::now();
    let result = run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
        &sampling_frame,
        &original,
        schedule,
        args.workers,
        |_replicate_index, positions| {
            estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
                &source,
                positions,
                || cancelled.load(Ordering::Relaxed),
                |_| {},
            )
            .map_err(map_refit_error)
        },
        || cancelled.load(Ordering::Relaxed),
        |progress| {
            if args
                .cancel_after
                .is_some_and(|threshold| progress.completed_replicates >= threshold)
                && !cancelled.swap(true, Ordering::Relaxed)
            {
                *cancellation_started
                    .lock()
                    .expect("cancellation mutex poisoned") = Some(Instant::now());
            }
        },
    );
    let elapsed_seconds = started.elapsed().as_secs_f64();

    let document = match (args.cancel_after, result) {
        (
            Some(_),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(ResamplingError::Cancelled)),
        ) => {
            let cancellation_latency_seconds = cancellation_started
                .lock()
                .expect("cancellation mutex poisoned")
                .as_ref()
                .ok_or("cancellation threshold was never reached")?
                .elapsed()
                .as_secs_f64();
            json!({
                "schema_version": 1,
                "kind": KIND,
                "status": "cancelled_as_requested",
                "qualification_status": "measurement_only_no_caps_or_promotion",
                "case": case_json(
                    &args,
                    observed_variables,
                    original.refit.free_parameters.len(),
                    source.optimizer_dimension_count(),
                ),
                "elapsed_seconds": elapsed_seconds,
                "cancellation_latency_seconds": cancellation_latency_seconds,
                "fixture": fixture_json(&data_path, &recipe_path)?,
            })
        }
        (Some(_), Ok(_)) => {
            return Err("cancellation benchmark completed instead of cancelling".into());
        }
        (Some(_), Err(error)) => return Err(error.into()),
        (None, Err(error)) => return Err(error.into()),
        (None, Ok(result)) => {
            let successful = result.base.successful_refits.len();
            let se_unavailable = result
                .studentized
                .refit_standard_errors
                .iter()
                .filter(|row| matches!(
                    &row.outcome,
                    qpls_estimation::CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Unavailable { .. }
                ))
                .count();
            let sidecar_json_bytes = serde_json::to_vec(&result.studentized)?.len();
            let combined_json = serde_json::to_vec(&result)?;
            let scientific_sha256 = sha256(&combined_json);
            json!({
                "schema_version": 1,
                "kind": KIND,
                "status": "completed",
                "qualification_status": "measurement_only_no_caps_or_promotion",
                "case": case_json(
                    &args,
                    observed_variables,
                    original.refit.free_parameters.len(),
                    source.optimizer_dimension_count(),
                ),
                "elapsed_seconds": elapsed_seconds,
                "metrics": {
                    "successful_point_refits": successful,
                    "failed_point_refits": result.base.failed_refits.len(),
                    "analytic_se_unavailable_refits": se_unavailable,
                    "analytic_se_unavailable_rate_among_point_successes": ratio(se_unavailable, successful),
                    "studentized_usable_refits": result.studentized.studentized_usable_replicates,
                    "studentized_usable_rate_among_point_successes": ratio(result.studentized.studentized_usable_replicates as usize, successful),
                    "base_json_bytes": serde_json::to_vec(&result.base)?.len(),
                    "studentized_sidecar_json_bytes": sidecar_json_bytes,
                    "combined_s2_json_bytes": combined_json.len(),
                    "canonical_v11_json_bytes": Value::Null,
                    "text_export_v11_bytes": Value::Null,
                    "unavailable_size_reason": "v11 canonical and export builders are not yet an execution authority; this run does not project guessed bytes"
                },
                "scientific_result_sha256": scientific_sha256,
                "fixture": fixture_json(&data_path, &recipe_path)?,
                "result": result,
            })
        }
    };
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&document)?)?;
    Ok(())
}

fn case_json(
    args: &Args,
    observed_variables: usize,
    free_parameters: usize,
    optimizer_dimensions: usize,
) -> Value {
    json!({
        "n_complete_cases": args.rows,
        "factor_blocks": args.factors,
        "free_parameters": free_parameters,
        "optimizer_dimensions": optimizer_dimensions,
        "requested_replicates": args.replicates,
        "workers": args.workers,
        "seed": args.seed,
        "dimensions": {
            "n_complete_cases": args.rows,
            "v_observed_variables": observed_variables,
            "p_free_parameter_rows": free_parameters,
            "d_optimizer_dimensions": optimizer_dimensions,
            "d_status": "available_from_prepared_exact_bootstrap_source_v1",
            "d_note": "D is the exact independent free-dimension count from the prepared source; equality-constrained parameter rows share one optimizer dimension."
        }
    })
}

fn fixture_json(data_path: &Path, recipe_path: &Path) -> Result<Value, std::io::Error> {
    Ok(json!({
        "data_path": FIXTURE_DATA,
        "data_sha256": sha256(&fs::read(data_path)?),
        "recipe_path": FIXTURE_RECIPE,
        "recipe_sha256": sha256(&fs::read(recipe_path)?),
        "fixture_scope": "existing_v07_cfa_rows_truncated_and_first_factor_blocks_selected_v1",
    }))
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn map_refit_error(error: CbsemCompiledMomentErrorV2) -> CbsemExactCaseBootstrapAttemptErrorV1 {
    if matches!(&error, CbsemCompiledMomentErrorV2::Cancelled) {
        return CbsemExactCaseBootstrapAttemptErrorV1::Cancelled;
    }
    let message = error.to_string();
    let kind = match error {
        CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite { .. } => {
            CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite
        }
        CbsemCompiledMomentErrorV2::NonConvergence
        | CbsemCompiledMomentErrorV2::ExactParameterTable(
            CbsemExactParameterTableErrorV3::NonConvergence
            | CbsemExactParameterTableErrorV3::OptimizerLineSearchFailed { .. }
            | CbsemExactParameterTableErrorV3::OptimizerObjectiveStagnation { .. }
            | CbsemExactParameterTableErrorV3::OptimizerIterationLimit { .. },
        ) => CbsemExactCaseBootstrapFailureKindV1::NonConvergence,
        CbsemCompiledMomentErrorV2::ExactParameterTable(
            CbsemExactParameterTableErrorV3::Numerical(_),
        ) => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
        CbsemCompiledMomentErrorV2::ExactParameterTable(_) => {
            CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution
        }
        _ => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
    };
    CbsemExactCaseBootstrapAttemptErrorV1::Failed { kind, message }
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut values = env::args().skip(1);
    let mut args = Args {
        repo_root: env::current_dir()?,
        output: PathBuf::from("validation/results/cbsem_exact_case_bootstrap_studentized/raw.json"),
        rows: 180,
        factors: 3,
        replicates: 1_000,
        workers: 1,
        seed: 91,
        cancel_after: None,
    };
    while let Some(flag) = values.next() {
        let value = values
            .next()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--repo-root" => args.repo_root = PathBuf::from(value),
            "--output" => args.output = PathBuf::from(value),
            "--rows" => args.rows = value.parse()?,
            "--factors" => args.factors = value.parse()?,
            "--replicates" => args.replicates = value.parse()?,
            "--workers" => args.workers = value.parse()?,
            "--seed" => args.seed = value.parse()?,
            "--cancel-after" => args.cancel_after = Some(value.parse()?),
            _ => return Err(format!("unsupported argument {flag}").into()),
        }
    }
    Ok(args)
}
