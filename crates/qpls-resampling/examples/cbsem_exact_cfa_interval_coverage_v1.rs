use qpls_core::{
    ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
    AnalysisRecipeV4, AnalysisSettings, CbsemEstimator, CbsemInput, CbsemModelType, Construct,
    LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4, MeasurementMode, MethodConfig,
    MissingDataPolicyV4, ModelSpec, Preprocessing, RecipeV4CompilerTarget, SemDataBindingV4,
    SemEndpointV4, SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
    compile_analysis_recipe_v4, convert_legacy_basic_model_v4,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_estimation::{
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1, CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CbsemCompiledMomentErrorV2, CbsemExactCaseBootstrapFailureKindV1,
    CbsemExactParameterTableErrorV3, cbsem_exact_case_bootstrap_complete_case_universe_digest_v1,
    cbsem_exact_case_bootstrap_index_digest_v1,
    cbsem_exact_case_bootstrap_sampling_positions_digest_v1,
    estimate_cbsem_ml_exact_case_delete_one_v1_with_control,
    estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control,
    prepare_cbsem_ml_exact_case_bootstrap_source_v1,
};
use qpls_resampling::{
    CbsemExactCaseBootstrapAttemptErrorV1, CbsemExactCaseBootstrapScheduleV1,
    CbsemExactCaseBootstrapSchedulerErrorV1, ResamplingError, bootstrap_indices,
    cbsem_exact_case_bootstrap_schedule_positions_digest_v1, run_cbsem_exact_case_bootstrap_bca_v1,
    run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

const KIND: &str = "cbsem_exact_cfa_interval_coverage_engine_case_v1";
const TYPED_JSON_DIGEST_METHOD: &str = "sha256_typed_json_tree_v1";
const DIGEST_VERIFICATION_KIND: &str =
    "cbsem_exact_cfa_interval_coverage_digest_verification_receipt_v1";
const DIGEST_VERIFICATION_METHOD: &str = "bound_rust_exact_schedule_and_source_verifier_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CancelPhase {
    None,
    Bootstrap,
    Jackknife,
}

#[derive(Debug)]
struct Args {
    repo_root: PathBuf,
    data: PathBuf,
    output: PathBuf,
    dgp_id: String,
    dataset_index: u32,
    replicates: u32,
    workers: usize,
    bootstrap_seed: u64,
    cancel_phase: CancelPhase,
    cancel_after: Option<u32>,
}

#[derive(Debug, Clone)]
struct Dgp {
    id: &'static str,
    sample_size: usize,
    columns: &'static [&'static str],
    factor_indicators: &'static [&'static [&'static str]],
    free_loadings: &'static [f64],
    factor_variances: &'static [f64],
    factor_covariance: Option<f64>,
    residual_variances: &'static [f64],
    expected_free_parameters: usize,
}

const DGP_A_COLUMNS: &[&str] = &["a1", "a2", "a3"];
const DGP_A_FACTOR_1: &[&str] = &["a1", "a2", "a3"];
const DGP_A_FACTORS: &[&[&str]] = &[DGP_A_FACTOR_1];
const DGP_A_LOADINGS: &[f64] = &[0.78, 0.64];
const DGP_A_VARIANCES: &[f64] = &[1.35];
const DGP_A_RESIDUALS: &[f64] = &[0.42, 0.58, 0.73];

const DGP_B_COLUMNS: &[&str] = &["b1", "b2", "b3", "b4", "b5", "b6"];
const DGP_B_FACTOR_1: &[&str] = &["b1", "b2", "b3"];
const DGP_B_FACTOR_2: &[&str] = &["b4", "b5", "b6"];
const DGP_B_FACTORS: &[&[&str]] = &[DGP_B_FACTOR_1, DGP_B_FACTOR_2];
const DGP_B_LOADINGS: &[f64] = &[0.78, 0.64, 0.70, 1.15];
const DGP_B_VARIANCES: &[f64] = &[1.35, 2.10];
const DGP_B_RESIDUALS: &[f64] = &[0.42, 0.58, 0.73, 0.50, 0.90, 2.00];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args = env::args().skip(1).collect::<Vec<_>>();
    if raw_args
        .first()
        .is_some_and(|value| value == "--derive-schedule-digests")
    {
        return derive_schedule_digests_command(&raw_args);
    }
    if raw_args
        .first()
        .is_some_and(|value| value == "--verify-document")
    {
        return verify_document_command(&raw_args);
    }
    let args = parse_args(raw_args.into_iter())?;
    let dgp = dgp(&args.dgp_id)?;
    if args.workers == 0 || args.replicates == 0 {
        return Err("workers and replicates must be positive".into());
    }
    if args.cancel_phase != CancelPhase::None && args.cancel_after.is_none() {
        return Err("a cancellation phase requires --cancel-after".into());
    }
    if args.cancel_phase == CancelPhase::None && args.cancel_after.is_some() {
        return Err("--cancel-after is valid only with a cancellation phase".into());
    }

    let data_path = if args.data.is_absolute() {
        args.data.clone()
    } else {
        args.repo_root.join(&args.data)
    };
    let data_bytes = fs::read(&data_path)?;
    let mut dataset = import_delimited_bytes(
        &data_bytes,
        "cbsem-exact-cfa-interval-coverage.csv",
        b',',
        &ImportOptions::default(),
    )?;
    dataset.id = dataset_uuid(dgp.id, args.dataset_index);

    // Use only the normal deterministic compiler/estimator start policy. The
    // population truth is kept in a separate reporting map and must never be
    // injected into an executed recipe or model.
    let model = build_model(&dgp, &dataset.id.to_string())?;
    model.ensure_valid()?;
    let observed_variables = model
        .variables
        .iter()
        .filter(|variable| matches!(variable, SemVariableV4::Observed { .. }))
        .count();

    let mut settings = AnalysisSettings::default();
    settings.method = AnalysisMethod::Cbsem;
    settings.preprocessing = Preprocessing::Unstandardized;
    settings.workers = 1;
    let recipe = AnalysisRecipeV4 {
        schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
        id: recipe_uuid(dgp.id, args.dataset_index),
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0)
            .ok_or("invalid fixed coverage timestamp")?,
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
        mga_multigroup: None,
        pls_heterogeneity: None,
        general_sem_conditional_process: None,
        interventional_causal_mediation: None,
        metadata: BTreeMap::new(),
        legacy_source: None,
    };
    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let artifact =
        compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())?;
    let source =
        prepare_cbsem_ml_exact_case_bootstrap_source_v1(&dataset, &artifact, &recipe, &model)?;
    if source.complete_case_sample_size() != dgp.sample_size
        || observed_variables != dgp.columns.len()
        || source.optimizer_dimension_count() != dgp.expected_free_parameters
    {
        return Err("generated coverage fixture dimensions differ from the frozen DGP".into());
    }
    let sampling_frame = (0..dgp.sample_size).collect::<Vec<_>>();
    let original =
        estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
            &source,
            &sampling_frame,
            || false,
            |_| {},
        )?;
    if original.refit.free_parameters.len() != dgp.expected_free_parameters {
        return Err("exact point result free-parameter count differs from the frozen DGP".into());
    }
    let truth = truth_rows(&original.refit, &dgp)?;
    let base_point_result_sha256 = typed_json_sha256(&serde_json::to_value(&original.refit)?)?;
    let schedule = CbsemExactCaseBootstrapScheduleV1 {
        outer_recipe_analytical_identity_sha256: artifact.receipt().recipe_analytical_sha256(),
        base_point_result_sha256: &base_point_result_sha256,
        requested_replicates: args.replicates,
        seed: args.bootstrap_seed,
        stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
        retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
        max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
        hypothesis_test: None,
    };

    let bootstrap_cancelled = AtomicBool::new(false);
    let bootstrap_cancel_started = Mutex::new(None::<Instant>);
    let started = Instant::now();
    let bootstrap = run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
        &sampling_frame,
        &original,
        schedule,
        args.workers,
        |_replicate_index, positions| {
            estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
                &source,
                positions,
                || bootstrap_cancelled.load(Ordering::Relaxed),
                |_| {},
            )
            .map_err(map_refit_error)
        },
        || bootstrap_cancelled.load(Ordering::Relaxed),
        |progress| {
            if args.cancel_phase == CancelPhase::Bootstrap
                && args
                    .cancel_after
                    .is_some_and(|threshold| progress.completed_replicates >= threshold)
                && !bootstrap_cancelled.swap(true, Ordering::Relaxed)
            {
                *bootstrap_cancel_started
                    .lock()
                    .expect("bootstrap cancellation mutex poisoned") = Some(Instant::now());
            }
        },
    );

    if args.cancel_phase == CancelPhase::Bootstrap {
        let error = bootstrap
            .err()
            .ok_or("bootstrap completed instead of cancelling")?;
        if !matches!(
            error,
            CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(ResamplingError::Cancelled)
        ) {
            return Err(error.into());
        }
        let latency = bootstrap_cancel_started
            .lock()
            .expect("bootstrap cancellation mutex poisoned")
            .as_ref()
            .ok_or("bootstrap cancellation threshold was never reached")?
            .elapsed()
            .as_secs_f64();
        return write_document(
            &args,
            json!({
                "schema_version": 1,
                "kind": KIND,
                "status": "cancelled_as_requested",
                "qualification_status": "validation_only_product_qualification_blocked",
                "case": case_json(&args, &dgp, observed_variables, source.optimizer_dimension_count()),
                "elapsed_seconds": started.elapsed().as_secs_f64(),
                "cancellation": {"phase": "bootstrap", "terminal_latency_seconds": latency},
                "fixture": {"data_sha256": sha256(&data_bytes)},
            }),
        );
    }
    let bootstrap = bootstrap?;

    let jackknife_cancelled = AtomicBool::new(false);
    let jackknife_cancel_started = Mutex::new(None::<Instant>);
    let bca = run_cbsem_exact_case_bootstrap_bca_v1(
        &sampling_frame,
        &original.refit,
        &bootstrap.base,
        args.workers,
        |omitted_position| {
            estimate_cbsem_ml_exact_case_delete_one_v1_with_control(
                &source,
                omitted_position,
                || jackknife_cancelled.load(Ordering::Relaxed),
                |_| {},
            )
            .map_err(map_refit_error)
        },
        || jackknife_cancelled.load(Ordering::Relaxed),
        |progress| {
            if args.cancel_phase == CancelPhase::Jackknife
                && args
                    .cancel_after
                    .is_some_and(|threshold| progress.completed_replicates >= threshold)
                && !jackknife_cancelled.swap(true, Ordering::Relaxed)
            {
                *jackknife_cancel_started
                    .lock()
                    .expect("jackknife cancellation mutex poisoned") = Some(Instant::now());
            }
        },
    );

    if args.cancel_phase == CancelPhase::Jackknife {
        let error = bca
            .err()
            .ok_or("jackknife completed instead of cancelling")?;
        if !matches!(
            error,
            CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(ResamplingError::Cancelled)
        ) {
            return Err(error.into());
        }
        let latency = jackknife_cancel_started
            .lock()
            .expect("jackknife cancellation mutex poisoned")
            .as_ref()
            .ok_or("jackknife cancellation threshold was never reached")?
            .elapsed()
            .as_secs_f64();
        return write_document(
            &args,
            json!({
                "schema_version": 1,
                "kind": KIND,
                "status": "cancelled_as_requested",
                "qualification_status": "validation_only_product_qualification_blocked",
                "case": case_json(&args, &dgp, observed_variables, source.optimizer_dimension_count()),
                "elapsed_seconds": started.elapsed().as_secs_f64(),
                "cancellation": {"phase": "jackknife", "terminal_latency_seconds": latency},
                "fixture": {"data_sha256": sha256(&data_bytes)},
            }),
        );
    }
    let bca = bca?;
    let scientific_result_sha256 = typed_json_sha256(&json!({
        "original": &original,
        "bootstrap": &bootstrap,
        "bca": &bca,
    }))?;
    write_document(
        &args,
        json!({
            "schema_version": 1,
            "kind": KIND,
            "status": "completed",
            "qualification_status": "validation_only_product_qualification_blocked",
            "case": case_json(&args, &dgp, observed_variables, source.optimizer_dimension_count()),
            "elapsed_seconds": started.elapsed().as_secs_f64(),
            "fixture": {"data_sha256": sha256(&data_bytes)},
            "digest_contract": digest_contract_json(),
            "truth": truth,
            "scientific_result_sha256": scientific_result_sha256,
            "original": original,
            "bootstrap": bootstrap,
            "bca": bca,
        }),
    )
}

fn dgp(id: &str) -> Result<Dgp, Box<dyn std::error::Error>> {
    match id {
        "dgp_a_one_factor_n150" => Ok(Dgp {
            id: "dgp_a_one_factor_n150",
            sample_size: 150,
            columns: DGP_A_COLUMNS,
            factor_indicators: DGP_A_FACTORS,
            free_loadings: DGP_A_LOADINGS,
            factor_variances: DGP_A_VARIANCES,
            factor_covariance: None,
            residual_variances: DGP_A_RESIDUALS,
            expected_free_parameters: 6,
        }),
        "dgp_b_two_factor_n300" => Ok(Dgp {
            id: "dgp_b_two_factor_n300",
            sample_size: 300,
            columns: DGP_B_COLUMNS,
            factor_indicators: DGP_B_FACTORS,
            free_loadings: DGP_B_LOADINGS,
            factor_variances: DGP_B_VARIANCES,
            factor_covariance: Some(0.45),
            residual_variances: DGP_B_RESIDUALS,
            expected_free_parameters: 13,
        }),
        _ => Err(format!("unknown frozen DGP {id}").into()),
    }
}

fn build_model(
    dgp: &Dgp,
    dataset_id: &str,
) -> Result<qpls_core::SemModelV4, Box<dyn std::error::Error>> {
    let constructs = dgp
        .factor_indicators
        .iter()
        .enumerate()
        .map(|(index, indicators)| Construct {
            id: format!("f{}", index + 1),
            name: format!("Factor {}", index + 1),
            short_name: format!("F{}", index + 1),
            mode: MeasurementMode::Reflective,
            indicators: indicators.iter().map(|value| (*value).into()).collect(),
        })
        .collect();
    let legacy = ModelSpec {
        id: uuid::Uuid::from_u128(0xCB5E_C0A0_0000_0000_0000_0000_0000_0001),
        name: format!("Exact CFA interval coverage {}", dgp.id),
        constructs,
        paths: Vec::new(),
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let mut model = convert_legacy_basic_model_v4(
        &legacy,
        LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        &[],
    )?;
    if dgp.factor_covariance.is_some() {
        let left = SemEndpointV4::Variable("construct:f1".into());
        let right = SemEndpointV4::Variable("construct:f2".into());
        let parameter = "coverage_factor_covariance_f1_f2".to_string();
        model.relations.push(SemRelationV4::Covariance {
            id: "coverage_covariance_f1_f2".into(),
            left: left.clone(),
            right: right.clone(),
            parameter: parameter.clone(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter,
            label: "Cov(F1,F2)".into(),
            target: SemParameterTargetV4::Covariance { left, right },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }
    model.data_binding = SemDataBindingV4::Raw {
        dataset_id: dataset_id.into(),
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        weight: None,
        cluster_variable: None,
        strata_variable: None,
    };
    Ok(model.canonicalized())
}

fn truth_rows(
    original: &qpls_estimation::CbsemExactCaseBootstrapRefitV1,
    dgp: &Dgp,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let model_truth = truth_by_target(dgp);
    let mut by_id = BTreeMap::new();
    // Exact result parameter ids are stable conversion ids. Reconstruct the
    // same model solely to bind ids to target truth without using estimates.
    let model = build_model(dgp, "truth-binding")?;
    for parameter in &model.parameters {
        if let SemParameterV4::Free { id, target, .. } = parameter {
            let key = target_key(target)?;
            by_id.insert(id.clone(), (key.clone(), model_truth[&key]));
        }
    }
    original
        .free_parameters
        .iter()
        .map(|row| {
            let (target, value) = by_id
                .get(&row.parameter_id)
                .ok_or_else(|| format!("point parameter {} has no truth", row.parameter_id))?;
            Ok(json!({
                "parameter_id": row.parameter_id,
                "target": target,
                "true_value": value,
            }))
        })
        .collect()
}

fn truth_by_target(dgp: &Dgp) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::new();
    let mut free_loading_index = 0;
    for (factor_index, indicators) in dgp.factor_indicators.iter().enumerate() {
        for indicator in indicators.iter().skip(1) {
            values.insert(
                format!(
                    "loading:construct:f{}:observed:{indicator}",
                    factor_index + 1
                ),
                dgp.free_loadings[free_loading_index],
            );
            free_loading_index += 1;
        }
        values.insert(
            format!("variance:variable:construct:f{}", factor_index + 1),
            dgp.factor_variances[factor_index],
        );
    }
    for (indicator, value) in dgp.columns.iter().zip(dgp.residual_variances) {
        values.insert(format!("variance:residual:observed:{indicator}"), *value);
    }
    if let Some(value) = dgp.factor_covariance {
        values.insert(
            "covariance:variable:construct:f1:variable:construct:f2".into(),
            value,
        );
    }
    values
}

fn target_key(target: &SemParameterTargetV4) -> Result<String, Box<dyn std::error::Error>> {
    fn endpoint(endpoint: &SemEndpointV4) -> String {
        match endpoint {
            SemEndpointV4::Variable(value) => format!("variable:{value}"),
            SemEndpointV4::ResidualOf(value) => format!("residual:{value}"),
            SemEndpointV4::DisturbanceOf(value) => format!("disturbance:{value}"),
        }
    }
    match target {
        SemParameterTargetV4::Loading {
            construct,
            indicator,
        } => Ok(format!("loading:{construct}:{indicator}")),
        SemParameterTargetV4::Variance { endpoint: value } => {
            Ok(format!("variance:{}", endpoint(value)))
        }
        SemParameterTargetV4::Covariance { left, right } => {
            Ok(format!("covariance:{}:{}", endpoint(left), endpoint(right)))
        }
        other => Err(format!("unsupported frozen DGP target {other:?}").into()),
    }
}

fn case_json(args: &Args, dgp: &Dgp, variables: usize, dimensions: usize) -> Value {
    json!({
        "dgp_id": dgp.id,
        "dataset_index": args.dataset_index,
        "n_complete_cases": dgp.sample_size,
        "v_observed_variables": variables,
        "p_free_parameter_rows": dgp.expected_free_parameters,
        "d_optimizer_dimensions": dimensions,
        "requested_replicates": args.replicates,
        "workers": args.workers,
        "bootstrap_seed": args.bootstrap_seed,
        "cancel_phase": match args.cancel_phase {
            CancelPhase::None => Value::Null,
            CancelPhase::Bootstrap => json!("bootstrap"),
            CancelPhase::Jackknife => json!("jackknife"),
        },
        "cancel_after": args.cancel_after,
    })
}

fn dataset_uuid(dgp_id: &str, dataset_index: u32) -> uuid::Uuid {
    let tag = if dgp_id == "dgp_a_one_factor_n150" {
        0xA_u128
    } else {
        0xB_u128
    };
    uuid::Uuid::from_u128(
        0xCB5E_C0A0_0000_0000_0000_0000_0000_0000 | (tag << 32) | dataset_index as u128,
    )
}

fn recipe_uuid(dgp_id: &str, dataset_index: u32) -> uuid::Uuid {
    let tag = if dgp_id == "dgp_a_one_factor_n150" {
        0x1A_u128
    } else {
        0x1B_u128
    };
    uuid::Uuid::from_u128(
        0xCB5E_C0A1_0000_0000_0000_0000_0000_0000 | (tag << 32) | dataset_index as u128,
    )
}

fn write_document(args: &Args, document: Value) -> Result<(), Box<dyn std::error::Error>> {
    if args.output.exists() {
        return Err(format!(
            "append-only output already exists: {}",
            args.output.display()
        )
        .into());
    }
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&document)?)?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn digest_contract_json() -> Value {
    json!({
        "scientific_result_digest_method": TYPED_JSON_DIGEST_METHOD,
        "base_point_result_digest_method": TYPED_JSON_DIGEST_METHOD,
        "schedule_authentication_method": DIGEST_VERIFICATION_METHOD,
        "compact_evidence_digest_method": TYPED_JSON_DIGEST_METHOD,
        "compact_evidence_fields": [
            "scientific_result_sha256",
            "data_sha256",
            "source_dataset_fingerprint",
            "ledger",
            "methods",
        ],
        "compact_method_order": ["percentile", "studentized", "bca"],
        "compact_parameter_order": "frozen_dgp_parameter_truth_order_v1",
        "coverage_authentication": "excluded_from_compact_evidence_and_rederived_by_aggregator_v1",
    })
}

fn typed_json_sha256(value: &Value) -> Result<String, Box<dyn std::error::Error>> {
    fn update(digest: &mut Sha256, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
        match value {
            Value::Null => digest.update(b"N"),
            Value::Bool(true) => digest.update(b"T"),
            Value::Bool(false) => digest.update(b"F"),
            Value::Number(number) => {
                if let Some(integer) = number.as_i64() {
                    let token = integer.to_string();
                    digest.update(b"I");
                    digest.update((token.len() as u64).to_le_bytes());
                    digest.update(token.as_bytes());
                } else if let Some(integer) = number.as_u64() {
                    let token = integer.to_string();
                    digest.update(b"I");
                    digest.update((token.len() as u64).to_le_bytes());
                    digest.update(token.as_bytes());
                } else {
                    let number = number
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or("typed JSON digest rejects a nonfinite number")?;
                    digest.update(b"D");
                    digest.update(number.to_le_bytes());
                }
            }
            Value::String(value) => {
                digest.update(b"S");
                digest.update((value.len() as u64).to_le_bytes());
                digest.update(value.as_bytes());
            }
            Value::Array(values) => {
                digest.update(b"A");
                digest.update((values.len() as u64).to_le_bytes());
                for value in values {
                    update(digest, value)?;
                }
            }
            Value::Object(values) => {
                let mut keys = values.keys().collect::<Vec<_>>();
                keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
                digest.update(b"O");
                digest.update((keys.len() as u64).to_le_bytes());
                for key in keys {
                    digest.update((key.len() as u64).to_le_bytes());
                    digest.update(key.as_bytes());
                    update(digest, &values[key])?;
                }
            }
        }
        Ok(())
    }

    let mut digest = Sha256::new();
    update(&mut digest, value)?;
    Ok(format!("{:x}", digest.finalize()))
}

fn categorical_counts(rows: &Value, field: &str, allowed: &[&str]) -> BTreeMap<String, u64> {
    let mut counts = allowed
        .iter()
        .map(|value| ((*value).to_owned(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    for row in rows.as_array().into_iter().flatten() {
        if let Some(value) = row[field].as_str()
            && let Some(count) = counts.get_mut(value)
        {
            *count += 1;
        }
    }
    counts
}

fn compact_ledger(document: &Value) -> Value {
    const POINT_FAILURE_KINDS: &[&str] = &[
        "moment_matrix_not_positive_definite",
        "non_convergence",
        "inadmissible_solution",
        "numerical_failure",
    ];
    const SE_UNAVAILABLE_REASONS: &[&str] = &[
        "singular_information",
        "information_not_positive_definite",
        "invalid_information_variance_or_standard_error",
        "derivative_unavailable",
        "numerical_information_failure",
    ];
    let base = &document["bootstrap"]["base"];
    let studentized = &document["bootstrap"]["studentized"];
    let bca = &document["bca"];
    let point_successes = base["successful_refits"].as_array().map_or(0, Vec::len);
    let point_failures = base["failed_refits"].as_array().map_or(0, Vec::len);
    let mut studentized_usable = 0_usize;
    let mut standard_error_unavailable = 0_usize;
    let mut unavailable_outcomes = Vec::new();
    for row in studentized["refit_standard_errors"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let outcome = &row["outcome"];
        match outcome["status"].as_str() {
            Some("available") => studentized_usable += 1,
            Some("unavailable") => {
                standard_error_unavailable += 1;
                unavailable_outcomes.push(outcome.clone());
            }
            _ => {}
        }
    }
    let delete_one_successes = bca["successful_delete_one_refits"]
        .as_array()
        .map_or(0, Vec::len);
    let delete_one_failures = bca["failed_delete_one_refits"]
        .as_array()
        .map_or(0, Vec::len);
    json!({
        "requested_replicates": base["requested_replicates"].clone(),
        "point_successes": point_successes,
        "point_failures": point_failures,
        "point_failure_counts_by_kind": categorical_counts(
            &base["failed_refits"], "kind", POINT_FAILURE_KINDS
        ),
        "studentized_usable": studentized_usable,
        "standard_error_unavailable": standard_error_unavailable,
        "standard_error_unavailable_counts_by_reason": categorical_counts(
            &Value::Array(unavailable_outcomes), "reason", SE_UNAVAILABLE_REASONS
        ),
        "delete_one_successes": delete_one_successes,
        "delete_one_failures": delete_one_failures,
        "delete_one_failure_counts_by_kind": categorical_counts(
            &bca["failed_delete_one_refits"], "kind", POINT_FAILURE_KINDS
        ),
        "failure_rate_inference_unit": "dataset_not_bootstrap_replicate",
    })
}

fn compact_interval(document: &Value, method: &str, parameter_id: &str) -> Option<(Value, Value)> {
    let rows = match method {
        "percentile" => &document["bootstrap"]["base"]["intervals"],
        "studentized" => &document["bootstrap"]["studentized"]["intervals"],
        "bca" => &document["bca"]["intervals"],
        _ => return None,
    };
    let row = rows
        .as_array()?
        .iter()
        .find(|row| row["parameter_id"].as_str() == Some(parameter_id))?;
    if method == "percentile" {
        Some((
            row["percentile_lower"].clone(),
            row["percentile_upper"].clone(),
        ))
    } else {
        let outcome = &row["outcome"];
        (outcome["status"].as_str() == Some("available")).then(|| {
            (
                outcome["interval_lower"].clone(),
                outcome["interval_upper"].clone(),
            )
        })
    }
}

fn compact_method_rows(document: &Value) -> Value {
    let parameter_ids = document["original"]["refit"]["free_parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["parameter_id"].as_str().unwrap_or("").to_owned())
        .collect::<Vec<_>>();
    let methods = ["percentile", "studentized", "bca"]
        .into_iter()
        .map(|method| {
            let parameters = parameter_ids
                .iter()
                .map(|parameter_id| {
                    let interval = compact_interval(document, method, parameter_id);
                    json!({
                        "parameter_id": parameter_id,
                        "available": interval.is_some(),
                        "interval_lower": interval.as_ref().map(|value| value.0.clone()),
                        "interval_upper": interval.as_ref().map(|value| value.1.clone()),
                    })
                })
                .collect::<Vec<_>>();
            let available = parameters
                .iter()
                .all(|row| row["available"].as_bool() == Some(true));
            json!({
                "method": method,
                "available": available,
                "parameters": parameters,
            })
        })
        .collect::<Vec<_>>();
    Value::Array(methods)
}

fn compact_evidence(
    document: &Value,
    scientific_result_sha256: &str,
    data_sha256: &str,
    source_dataset_fingerprint: &str,
) -> Value {
    json!({
        "scientific_result_sha256": scientific_result_sha256,
        "data_sha256": data_sha256,
        "source_dataset_fingerprint": source_dataset_fingerprint,
        "ledger": compact_ledger(document),
        "methods": compact_method_rows(document),
    })
}

fn verify_document_command(raw_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut document_path = None;
    let mut data_path = None;
    let mut values = raw_args.iter();
    while let Some(flag) = values.next() {
        let value = values
            .next()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--verify-document" => document_path = Some(PathBuf::from(value)),
            "--data" => data_path = Some(PathBuf::from(value)),
            _ => return Err(format!("unsupported verification argument {flag}").into()),
        }
    }
    let document_path = document_path.ok_or("--verify-document is required")?;
    let data_path = data_path.ok_or("--data is required")?;
    let document_bytes = fs::read(&document_path)?;
    let data_bytes = fs::read(&data_path)?;
    let document: Value = serde_json::from_slice(&document_bytes)?;
    let mut reasons = Vec::<String>::new();

    let scientific_payload = json!({
        "original": document["original"].clone(),
        "bootstrap": document["bootstrap"].clone(),
        "bca": document["bca"].clone(),
    });
    let scientific_sha = typed_json_sha256(&scientific_payload)?;
    if document["scientific_result_sha256"].as_str() != Some(scientific_sha.as_str()) {
        reasons.push("scientific_result_digest_differs".into());
    }
    let point = &document["original"]["refit"];
    let base = &document["bootstrap"]["base"];
    let bca = &document["bca"];
    let base_point_sha = typed_json_sha256(point)?;
    if base["base_point_result_sha256"].as_str() != Some(base_point_sha.as_str())
        || bca["base_point_result_sha256"].as_str() != Some(base_point_sha.as_str())
    {
        reasons.push("base_point_result_digest_differs".into());
    }
    if document["digest_contract"] != digest_contract_json() {
        reasons.push("digest_contract_differs".into());
    }
    let data_sha256 = sha256(&data_bytes);
    if document["fixture"]["data_sha256"].as_str() != Some(data_sha256.as_str()) {
        reasons.push("fixture_data_digest_differs".into());
    }

    let dgp_id = document["case"]["dgp_id"].as_str().unwrap_or("");
    let dataset_index = document["case"]["dataset_index"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok());
    let mut derived_source_fingerprint = String::new();
    if let (Ok(dgp), Some(dataset_index)) = (dgp(dgp_id), dataset_index) {
        let identity = (|| -> Result<_, Box<dyn std::error::Error>> {
            let mut dataset = import_delimited_bytes(
                &data_bytes,
                "cbsem-exact-cfa-interval-coverage.csv",
                b',',
                &ImportOptions::default(),
            )?;
            dataset.id = dataset_uuid(dgp.id, dataset_index);
            let model = build_model(&dgp, &dataset.id.to_string())?;
            model.ensure_valid()?;
            let mut settings = AnalysisSettings::default();
            settings.method = AnalysisMethod::Cbsem;
            settings.preprocessing = Preprocessing::Unstandardized;
            settings.workers = 1;
            let recipe = AnalysisRecipeV4 {
                schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
                id: recipe_uuid(dgp.id, dataset_index),
                created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0)
                    .ok_or("invalid fixed coverage timestamp")?,
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
                mga_multigroup: None,
                pls_heterogeneity: None,
                general_sem_conditional_process: None,
                interventional_causal_mediation: None,
                metadata: BTreeMap::new(),
                legacy_source: None,
            };
            let target = RecipeV4CompilerTarget::CbsemPlanV2;
            let artifact = compile_analysis_recipe_v4(
                &recipe,
                Some(&model),
                target,
                target.capability_cell(),
            )?;
            let source_fingerprint = dataset
                .fingerprint
                .0
                .strip_prefix("v2:")
                .unwrap_or(&dataset.fingerprint.0)
                .to_owned();
            Ok((
                source_fingerprint,
                artifact.receipt().analytical_identity_sha256().to_owned(),
                artifact.receipt().plan_sha256().to_owned(),
                artifact.receipt().model_scientific_sha256().to_owned(),
                artifact.receipt().recipe_analytical_sha256().to_owned(),
            ))
        })();
        match identity {
            Ok((source_fingerprint, compiler_sha, plan_sha, model_sha, recipe_sha)) => {
                derived_source_fingerprint = source_fingerprint;
                for (label, observed, expected) in [
                    (
                        "source_dataset_fingerprint",
                        point["source_dataset_fingerprint"].as_str(),
                        derived_source_fingerprint.as_str(),
                    ),
                    (
                        "compiler_analytical_identity",
                        point["compiler_analytical_identity_sha256"].as_str(),
                        compiler_sha.as_str(),
                    ),
                    ("plan", point["plan_sha256"].as_str(), plan_sha.as_str()),
                    (
                        "model_scientific_identity",
                        point["model_scientific_sha256"].as_str(),
                        model_sha.as_str(),
                    ),
                    (
                        "outer_recipe_analytical_identity",
                        base["outer_recipe_analytical_identity_sha256"].as_str(),
                        recipe_sha.as_str(),
                    ),
                ] {
                    if observed != Some(expected) {
                        reasons.push(format!("{label}_differs"));
                    }
                }
            }
            Err(error) => reasons.push(format!("identity_rebuild_failed:{error}")),
        }
    } else {
        reasons.push("frozen_dgp_or_dataset_index_is_invalid".into());
    }

    let complete_case_count = point["complete_case_sample_size"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0);
    let source_row_count = point["source_row_count"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0);
    let complete_rows = (0..complete_case_count).collect::<Vec<_>>();
    let universe = cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
        &derived_source_fingerprint,
        source_row_count,
        &complete_rows,
    );
    let original_positions = cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
        complete_case_count,
        &complete_rows,
    );
    let original_rows = cbsem_exact_case_bootstrap_index_digest_v1(
        &derived_source_fingerprint,
        source_row_count,
        &complete_rows,
    );
    for (label, observed, expected) in [
        (
            "complete_case_universe",
            point["complete_case_universe_sha256"].as_str(),
            universe.as_str(),
        ),
        (
            "original_sampling_positions",
            point["sampling_positions_sha256"].as_str(),
            original_positions.as_str(),
        ),
        (
            "original_source_rows",
            point["sample_indices_sha256"].as_str(),
            original_rows.as_str(),
        ),
    ] {
        if observed != Some(expected) {
            reasons.push(format!("{label}_digest_differs"));
        }
    }

    let requested = base["requested_replicates"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let seed = base["seed"].as_u64().unwrap_or(0);
    let stream_token = base["stream_token"].as_str().unwrap_or("");
    let primary_operation = format!("{stream_token}:primary");
    let mut replicate_indices = BTreeSet::new();
    for ledger_name in ["successful_refits", "failed_refits"] {
        for row in base[ledger_name].as_array().into_iter().flatten() {
            let Some(replicate_index) = row["replicate_index"]
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
            else {
                reasons.push("bootstrap_replicate_index_is_invalid".into());
                continue;
            };
            if !replicate_indices.insert(replicate_index) {
                reasons.push("bootstrap_replicate_index_is_duplicated".into());
            }
            let positions = bootstrap_indices(
                complete_case_count,
                seed,
                &primary_operation,
                replicate_index,
            );
            let expected_schedule = cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                stream_token,
                seed,
                replicate_index,
                complete_case_count,
                &positions,
            );
            let source_rows = positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let expected_rows = cbsem_exact_case_bootstrap_index_digest_v1(
                &derived_source_fingerprint,
                source_row_count,
                &source_rows,
            );
            if row["sampling_positions_sha256"].as_str() != Some(expected_schedule.as_str()) {
                reasons.push(format!(
                    "bootstrap_schedule_positions_digest_differs:{replicate_index}"
                ));
            }
            if row["sample_indices_sha256"].as_str() != Some(expected_rows.as_str()) {
                reasons.push(format!(
                    "bootstrap_source_rows_digest_differs:{replicate_index}"
                ));
            }
        }
    }
    if replicate_indices != (0..requested).collect::<BTreeSet<_>>() {
        reasons.push("bootstrap_digest_ledger_is_not_the_exact_schedule".into());
    }

    let mut delete_positions = BTreeSet::new();
    for ledger_name in ["successful_delete_one_refits", "failed_delete_one_refits"] {
        for row in bca[ledger_name].as_array().into_iter().flatten() {
            let Some(omitted) = row["omitted_complete_case_position"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
            else {
                reasons.push("delete_one_position_is_invalid".into());
                continue;
            };
            if !delete_positions.insert(omitted) || omitted >= complete_case_count {
                reasons.push("delete_one_position_is_duplicated_or_out_of_range".into());
                continue;
            }
            let retained = complete_rows
                .iter()
                .copied()
                .filter(|position| *position != omitted)
                .collect::<Vec<_>>();
            let expected_positions = cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                complete_case_count,
                &retained,
            );
            let expected_rows = cbsem_exact_case_bootstrap_index_digest_v1(
                &derived_source_fingerprint,
                source_row_count,
                &retained,
            );
            if row["omitted_source_row_index"].as_u64() != Some(omitted as u64)
                || row["retained_sampling_positions_sha256"].as_str()
                    != Some(expected_positions.as_str())
            {
                reasons.push(format!(
                    "delete_one_sampling_positions_digest_differs:{omitted}"
                ));
            }
            if row["retained_sample_indices_sha256"].as_str() != Some(expected_rows.as_str()) {
                reasons.push(format!("delete_one_source_rows_digest_differs:{omitted}"));
            }
        }
    }
    if delete_positions != (0..complete_case_count).collect::<BTreeSet<_>>() {
        reasons.push("delete_one_digest_ledger_is_not_the_exact_schedule".into());
    }

    let checks = json!({
        "scientific_result": 1,
        "base_point_result": 1,
        "source_dataset_fingerprint": 1,
        "compiler_analytical_identity": 1,
        "plan": 1,
        "model_scientific_identity": 1,
        "outer_recipe_analytical_identity": 1,
        "complete_case_universe": 1,
        "original_sampling_positions": 1,
        "original_source_rows": 1,
        "bootstrap_schedule_positions": requested,
        "bootstrap_source_rows": requested,
        "delete_one_sampling_positions": complete_case_count,
        "delete_one_source_rows": complete_case_count,
    });
    let compact_evidence = compact_evidence(
        &document,
        &scientific_sha,
        &data_sha256,
        &derived_source_fingerprint,
    );
    let receipt = json!({
        "schema_version": 1,
        "kind": DIGEST_VERIFICATION_KIND,
        "status": if reasons.is_empty() { "accepted" } else { "rejected" },
        "method": DIGEST_VERIFICATION_METHOD,
        "document_typed_sha256": typed_json_sha256(&document)?,
        "scientific_result_sha256": scientific_sha,
        "compact_evidence_typed_sha256": typed_json_sha256(&compact_evidence)?,
        "data_sha256": data_sha256,
        "source_dataset_fingerprint": derived_source_fingerprint,
        "checks": checks,
        "reasons": reasons,
    });
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}

fn derive_schedule_digests_command(raw_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut complete_case_count = None;
    let mut source_fingerprint = None;
    let mut seed = None;
    let mut replicates = None;
    let mut stream_token = None;
    let mut values = raw_args.iter();
    while let Some(flag) = values.next() {
        let value = values
            .next()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--derive-schedule-digests" => complete_case_count = Some(value.parse::<usize>()?),
            "--source-fingerprint" => source_fingerprint = Some(value.to_owned()),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--replicates" => replicates = Some(value.parse::<u32>()?),
            "--stream-token" => stream_token = Some(value.to_owned()),
            _ => return Err(format!("unsupported schedule argument {flag}").into()),
        }
    }
    let complete_case_count = complete_case_count.ok_or("complete-case count is required")?;
    let source_fingerprint = source_fingerprint.ok_or("--source-fingerprint is required")?;
    let seed = seed.ok_or("--seed is required")?;
    let replicates = replicates.ok_or("--replicates is required")?;
    let stream_token = stream_token.ok_or("--stream-token is required")?;
    if source_fingerprint.len() != 64
        || !source_fingerprint
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
        || complete_case_count == 0
        || replicates == 0
    {
        return Err("schedule digest derivation preimage is invalid".into());
    }
    let operation = format!("{stream_token}:primary");
    let rows = (0..replicates)
        .map(|replicate_index| {
            let positions =
                bootstrap_indices(complete_case_count, seed, &operation, replicate_index);
            let schedule = cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                &stream_token,
                seed,
                replicate_index,
                complete_case_count,
                &positions,
            );
            let source_rows = cbsem_exact_case_bootstrap_index_digest_v1(
                &source_fingerprint,
                complete_case_count,
                &positions,
            );
            json!({
                "replicate_index": replicate_index,
                "sampling_positions_sha256": schedule,
                "sample_indices_sha256": source_rows,
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string(&json!({
            "schema_version": 1,
            "kind": "cbsem_exact_cfa_interval_coverage_schedule_digest_plan_v1",
            "complete_case_count": complete_case_count,
            "source_fingerprint": source_fingerprint,
            "seed": seed,
            "replicates": replicates,
            "stream_token": stream_token,
            "rows": rows,
        }))?
    );
    Ok(())
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

fn parse_args(
    mut values: impl Iterator<Item = String>,
) -> Result<Args, Box<dyn std::error::Error>> {
    let mut repo_root = env::current_dir()?;
    let mut data = None;
    let mut output = None;
    let mut dgp_id = None;
    let mut dataset_index = None;
    let mut replicates = None;
    let mut workers = None;
    let mut bootstrap_seed = None;
    let mut cancel_phase = CancelPhase::None;
    let mut cancel_after = None;
    while let Some(flag) = values.next() {
        let value = values
            .next()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--repo-root" => repo_root = PathBuf::from(value),
            "--data" => data = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--dgp-id" => dgp_id = Some(value),
            "--dataset-index" => dataset_index = Some(value.parse()?),
            "--replicates" => replicates = Some(value.parse()?),
            "--workers" => workers = Some(value.parse()?),
            "--bootstrap-seed" => bootstrap_seed = Some(value.parse()?),
            "--cancel-phase" => {
                cancel_phase = match value.as_str() {
                    "none" => CancelPhase::None,
                    "bootstrap" => CancelPhase::Bootstrap,
                    "jackknife" => CancelPhase::Jackknife,
                    _ => return Err(format!("unsupported cancellation phase {value}").into()),
                }
            }
            "--cancel-after" => cancel_after = Some(value.parse()?),
            _ => return Err(format!("unsupported argument {flag}").into()),
        }
    }
    Ok(Args {
        repo_root,
        data: data.ok_or("--data is required")?,
        output: output.ok_or("--output is required")?,
        dgp_id: dgp_id.ok_or("--dgp-id is required")?,
        dataset_index: dataset_index.ok_or("--dataset-index is required")?,
        replicates: replicates.ok_or("--replicates is required")?,
        workers: workers.ok_or("--workers is required")?,
        bootstrap_seed: bootstrap_seed.ok_or("--bootstrap-seed is required")?,
        cancel_phase,
        cancel_after,
    })
}
