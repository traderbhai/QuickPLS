//! End-to-end qualification producer for observed interventional mediation V1.
//!
//! Every positive fixture is compiled from Recipe V4 and executed by the raw
//! runner.  The report includes the exact prepared observed equations so an
//! independent Python implementation can reproduce the g-computation targets.

#[path = "support_multimod_metamorphic/mod.rs"]
mod metamorphic;
#[path = "support_multimod_qualification/mod.rs"]
mod support;

use chrono::{TimeZone, Utc};
use qpls_core::*;
use qpls_estimation::{
    InterventionalMediationBlockerV1, PreparedInterventionalDatasetPathV1,
    estimate_interventional_mediation_v1, prepare_interventional_causal_inputs_from_dataset_v1,
};
use qpls_resampling::{MultiModCaseBootstrapDrawV1, MultiModFinalLedgerV1, MultiModRefitOutcomeV1};
use qpls_runner::{MultiModRunnerEvidenceV1, run_compiled_interventional_causal_mediation_raw_v1};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use support::*;
use uuid::Uuid;

const PRODUCER_ID: &str = "qpls.multimod.interventional.raw-qualification.v1";
const FIXTURE_SEED: u64 = 42;

#[derive(Clone, Copy)]
enum Scale {
    Development,
    Qualification,
}

impl Scale {
    fn parse(value: &str) -> Result<Self, DynError> {
        match value {
            "development" => Ok(Self::Development),
            "qualification" => Ok(Self::Qualification),
            _ => Err(invalid(format!("unknown scale {value}"))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Qualification => "qualification",
        }
    }

    fn bootstrap_draws(self) -> u32 {
        match self {
            Self::Development => 500,
            Self::Qualification => 5_000,
        }
    }
}

struct Arguments {
    scale: Scale,
    output: PathBuf,
    mode: ExecutionMode,
    dependencies: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExecutionMode {
    Monolithic,
    Plan,
    Shard(String),
}

fn arguments() -> Result<Arguments, DynError> {
    let mut scale = Scale::Qualification;
    let mut output = None;
    let mut mode = ExecutionMode::Monolithic;
    let mut dependencies = Vec::new();
    let mut values = env::args().skip(1);
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--scale" => {
                scale = Scale::parse(
                    &values
                        .next()
                        .ok_or_else(|| invalid("--scale requires a value"))?,
                )?;
            }
            "--output" => {
                output = Some(PathBuf::from(
                    values
                        .next()
                        .ok_or_else(|| invalid("--output requires a path"))?,
                ));
            }
            "--plan" => {
                if mode != ExecutionMode::Monolithic {
                    return Err(invalid("--plan and --shard are mutually exclusive"));
                }
                mode = ExecutionMode::Plan;
            }
            "--shard" => {
                if mode != ExecutionMode::Monolithic {
                    return Err(invalid("--plan and --shard are mutually exclusive"));
                }
                mode = ExecutionMode::Shard(
                    values
                        .next()
                        .ok_or_else(|| invalid("--shard requires an exact shard id"))?,
                );
            }
            "--dependency" => dependencies.push(PathBuf::from(
                values
                    .next()
                    .ok_or_else(|| invalid("--dependency requires a shard result path"))?,
            )),
            _ => return Err(invalid(format!("unknown argument {argument}"))),
        }
    }
    if !dependencies.is_empty() && !matches!(&mode, ExecutionMode::Shard(_)) {
        return Err(invalid("--dependency is valid only with --shard"));
    }
    Ok(Arguments {
        scale,
        output: output.ok_or_else(|| invalid("--output is required"))?,
        mode,
        dependencies,
    })
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("MMQ.CAUSAL.PRODUCER: {error}");
        std::process::exit(2);
    }
}

fn execute() -> Result<(), DynError> {
    let arguments = arguments()?;
    match &arguments.mode {
        ExecutionMode::Monolithic => execute_monolithic(&arguments),
        ExecutionMode::Plan => write_causal_shard_plan(&arguments),
        ExecutionMode::Shard(shard_id) => run_causal_shard(&arguments, shard_id),
    }
}

fn execute_monolithic(arguments: &Arguments) -> Result<(), DynError> {
    let binary_two = binary_two_fixture(arguments.scale, false)?;
    let binary_four = binary_four_fixture(arguments.scale)?;
    let continuous = continuous_fixture(arguments.scale)?;
    let binary_two_case = run_case(
        &[
            "interventional.observed_gcomp.v1::observed_equation_point_fit",
            "interventional.observed_gcomp.v1::parametric_g_computation",
            "interventional.observed_gcomp.v1::known_target_simulation",
            "interventional.observed_gcomp.v1::causal_wording_guard",
        ],
        "binary_two_edge_path",
        binary_two,
    )?;
    let binary_four_case = run_case(
        &[
            "interventional.observed_gcomp.v1::observed_equation_point_fit",
            "interventional.observed_gcomp.v1::parametric_g_computation",
            "interventional.observed_gcomp.v1::known_target_simulation",
            "interventional.observed_gcomp.v1::causal_wording_guard",
        ],
        "binary_four_edge_path",
        binary_four,
    )?;
    let continuous_case = run_case(
        &[
            "interventional.observed_gcomp.v1::observed_equation_point_fit",
            "interventional.observed_gcomp.v1::parametric_g_computation",
            "interventional.observed_gcomp.v1::known_target_simulation",
            "interventional.observed_gcomp.v1::causal_wording_guard",
        ],
        "continuous_three_edge_path",
        continuous,
    )?;
    let blockers = assumption_and_scope_blockers(arguments.scale)?;
    let report = json!({
        "schema_version": 1,
        "producer_id": PRODUCER_ID,
        "family": "interventional_causal_mediation_v1",
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "qualification_claim": "none",
        "execution_contract": "public_recipe_v4_compiler_plus_raw_observed_g_computation_runner",
        "cases": [binary_two_case, binary_four_case, continuous_case],
        "assumption_and_scope_blockers": blockers,
        "required_cell_ids": required_cells(),
        "api_boundaries": {
            "recipe_v4_natural_or_cross_world_request": "not_representable_by_design; estimator guard exercised explicitly",
            "recipe_v4_recanting_witness_request": "represented only by required negative identification declaration; estimator guard also exercised explicitly",
            "recipe_v4_exposure_induced_confounding_request": "represented only by required negative identification declaration; estimator guard also exercised explicitly"
        }
    });
    if let Some(parent) = arguments.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&arguments.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

const SHARD_SCHEMA_VERSION: u32 = 1;
const SHARD_SUITE_ID: &str = "qpls.multimod.raw-qualification.shard.v1";
const SHARD_PLAN_SUITE_ID: &str = "qpls.multimod.raw-qualification.shard-plan.v1";

fn sign_columns_identity() -> Result<Option<String>, DynError> {
    if metamorphic::metamorphism_v1() != "sign_reverse" {
        return Ok(None);
    }
    let value = env::var(metamorphic::SIGN_COLUMNS_ENV_V1)
        .map_err(|_| invalid("sign_reverse requires QPLS_MULTIMOD_SIGN_COLUMNS_V1"))?;
    if value.trim().is_empty() {
        return Err(invalid(
            "sign_reverse requires at least one exact sign-column identity",
        ));
    }
    Ok(Some(value))
}

fn causal_shard_specs() -> Vec<Value> {
    let mut rows = vec![
        json!({
            "shard_id": "sentinel",
            "payload_kind": "sentinel",
            "dependencies": [],
            "resource_class": "sentinel",
            "parallel_safe_after_build": true,
            "scientific_identity": {
                "identity_kind": "sentinel",
                "identity_id": "fast-root-development",
            },
        }),
        json!({
            "shard_id": "assumption-scope-guards",
            "payload_kind": "evidence",
            "dependencies": ["sentinel"],
            "resource_class": "light",
            "parallel_safe_after_build": true,
            "scientific_identity": {
                "identity_kind": "evidence",
                "identity_id": "causal-assumption-scope-guards-v1",
            },
        }),
    ];
    for (shard_id, case_id) in [
        ("binary-two-edge", "binary_two_edge_path"),
        ("binary-four-edge", "binary_four_edge_path"),
        ("continuous-three-edge", "continuous_three_edge_path"),
    ] {
        rows.push(json!({
            "shard_id": shard_id,
            "payload_kind": "case",
            "dependencies": ["sentinel"],
            "resource_class": "heavy",
            "parallel_safe_after_build": true,
            "scientific_identity": {
                "identity_kind": "case",
                "case_id": case_id,
            },
        }));
    }
    rows
}

fn causal_shard_plan(arguments: &Arguments) -> Result<Value, DynError> {
    if metamorphic::compact_matrix_v1() {
        return Err(invalid(
            "the resumable qualification plan is unavailable for compact metamorphic fixtures; use monolithic metamorphic execution",
        ));
    }
    Ok(json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_PLAN_SUITE_ID,
        "family": "causal",
        "producer_id": PRODUCER_ID,
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "execution_contract": "one_cargo_build_then_exact_resumable_case_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": causal_shard_specs(),
    }))
}

fn causal_header(arguments: &Arguments) -> Result<Value, DynError> {
    Ok(json!({
        "schema_version": 1,
        "producer_id": PRODUCER_ID,
        "family": "interventional_causal_mediation_v1",
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "qualification_claim": "none",
        "execution_contract": "public_recipe_v4_compiler_plus_raw_observed_g_computation_runner",
        "required_cell_ids": required_cells(),
        "api_boundaries": {
            "recipe_v4_natural_or_cross_world_request": "not_representable_by_design; estimator guard exercised explicitly",
            "recipe_v4_recanting_witness_request": "represented only by required negative identification declaration; estimator guard also exercised explicitly",
            "recipe_v4_exposure_induced_confounding_request": "represented only by required negative identification declaration; estimator guard also exercised explicitly"
        },
    }))
}

fn write_causal_shard_plan(arguments: &Arguments) -> Result<(), DynError> {
    if let Some(parent) = arguments.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &arguments.output,
        serde_json::to_vec_pretty(&causal_shard_plan(arguments)?)?,
    )?;
    Ok(())
}

fn validate_causal_dependencies(
    arguments: &Arguments,
    shard_id: &str,
) -> Result<Vec<String>, DynError> {
    let expected = if shard_id == "sentinel" {
        Vec::new()
    } else if causal_shard_specs()
        .iter()
        .any(|row| row["shard_id"] == shard_id)
    {
        vec!["sentinel".to_owned()]
    } else {
        return Err(invalid(format!("unknown causal shard {shard_id}")));
    };
    let mut actual = Vec::new();
    let expected_sign_columns = serde_json::to_value(sign_columns_identity()?)?;
    let expected_workers = metamorphic::configured_workers_v1(1).map_err(invalid)? as u64;
    for path in &arguments.dependencies {
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if value["schema_version"].as_u64() != Some(u64::from(SHARD_SCHEMA_VERSION))
            || value["suite_id"] != SHARD_SUITE_ID
            || value["family"] != "causal"
            || value["producer_id"] != PRODUCER_ID
            || value["scale"] != arguments.scale.as_str()
            || value["seed"].as_u64() != Some(FIXTURE_SEED)
            || value["metamorphism"] != metamorphic::metamorphism_v1()
            || value.get("sign_columns") != Some(&expected_sign_columns)
            || value["workers"].as_u64() != Some(expected_workers)
        {
            return Err(invalid(format!(
                "causal dependency {} has the wrong identity",
                path.display()
            )));
        }
        actual.push(
            value["shard_id"]
                .as_str()
                .ok_or_else(|| invalid("causal dependency shard id is absent"))?
                .to_owned(),
        );
    }
    actual.sort();
    if actual != expected {
        return Err(invalid(format!(
            "causal shard {shard_id} requires dependencies {expected:?}, received {actual:?}"
        )));
    }
    Ok(actual)
}

fn causal_case_cell_ids() -> [&'static str; 4] {
    [
        "interventional.observed_gcomp.v1::observed_equation_point_fit",
        "interventional.observed_gcomp.v1::parametric_g_computation",
        "interventional.observed_gcomp.v1::known_target_simulation",
        "interventional.observed_gcomp.v1::causal_wording_guard",
    ]
}

fn causal_sentinel(arguments: &Arguments) -> Result<Value, DynError> {
    let diagnostic = run_case(
        &[],
        "causal_fast_root_sentinel",
        binary_two_fixture(Scale::Development, false)?,
    )?;
    Ok(json!({
        "sentinel_id": "fast-root-development",
        "header": causal_header(arguments)?,
        "diagnostic_production_case": diagnostic,
        "claim": "development-sized fail-fast production sentinel; excluded from qualification cases",
    }))
}

fn causal_evidence_shard(arguments: &Arguments, shard_id: &str) -> Result<Value, DynError> {
    match shard_id {
        "assumption-scope-guards" => Ok(json!({
            "evidence_id": "causal-assumption-scope-guards-v1",
            "assumption_and_scope_blockers": assumption_and_scope_blockers(arguments.scale)?,
        })),
        _ => Err(invalid(format!("unknown causal evidence shard {shard_id}"))),
    }
}

fn causal_case_shard(arguments: &Arguments, shard_id: &str) -> Result<Value, DynError> {
    let cell_ids = causal_case_cell_ids();
    match shard_id {
        "binary-two-edge" => run_case(
            &cell_ids,
            "binary_two_edge_path",
            binary_two_fixture(arguments.scale, false)?,
        ),
        "binary-four-edge" => run_case(
            &cell_ids,
            "binary_four_edge_path",
            binary_four_fixture(arguments.scale)?,
        ),
        "continuous-three-edge" => run_case(
            &cell_ids,
            "continuous_three_edge_path",
            continuous_fixture(arguments.scale)?,
        ),
        _ => Err(invalid(format!("unknown causal case shard {shard_id}"))),
    }
}

fn run_causal_shard(arguments: &Arguments, shard_id: &str) -> Result<(), DynError> {
    let dependency_shard_ids = validate_causal_dependencies(arguments, shard_id)?;
    let scientific_identity = causal_shard_specs()
        .into_iter()
        .find(|row| row["shard_id"] == shard_id)
        .ok_or_else(|| invalid(format!("unknown causal shard {shard_id}")))?["scientific_identity"]
        .clone();
    let payload = if shard_id == "sentinel" {
        json!({"kind": "sentinel", "value": causal_sentinel(arguments)?})
    } else if shard_id == "assumption-scope-guards" {
        json!({"kind": "evidence", "value": causal_evidence_shard(arguments, shard_id)?})
    } else {
        json!({"kind": "case", "value": causal_case_shard(arguments, shard_id)?})
    };
    let report = json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_SUITE_ID,
        "family": "causal",
        "producer_id": PRODUCER_ID,
        "shard_id": shard_id,
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "dependency_shard_ids": dependency_shard_ids,
        "scientific_identity": scientific_identity,
        "payload": payload,
    });
    if let Some(parent) = arguments.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&arguments.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

struct CausalFixture {
    dataset: qpls_data::Dataset,
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    dgp: Value,
}

fn metamorphic_dataset_from_columns(
    source_name: &str,
    headers: &[String],
    columns: &[Vec<Option<String>>],
) -> Result<qpls_data::Dataset, DynError> {
    let (headers, columns) =
        metamorphic::transformed_columns_v1(headers, columns).map_err(invalid)?;
    support::dataset_from_columns(source_name, &headers, &columns)
}

fn observed(id: &str, scale: ObservedScaleV4) -> SemVariableV4 {
    SemVariableV4::Observed {
        id: id.into(),
        label: id.to_uppercase(),
        source_column: id.into(),
        scale,
        role: ObservedRoleV4::Structural,
        categories: if scale == ObservedScaleV4::Binary {
            vec!["0".into(), "1".into()]
        } else {
            Vec::new()
        },
        value_labels: BTreeMap::new(),
        missing_markers: Vec::new(),
        transformation_lineage: Vec::new(),
    }
}

fn relation(source: &str, target: &str) -> (SemRelationV4, SemParameterV4) {
    let relation_id = format!("relation:{source}:{target}");
    let parameter_id = format!("parameter:{source}:{target}");
    (
        SemRelationV4::Structural {
            id: relation_id,
            source: source.into(),
            target: target.into(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        },
        SemParameterV4::Free {
            id: parameter_id,
            label: format!("{source} -> {target}"),
            target: SemParameterTargetV4::Regression {
                source: source.into(),
                target: target.into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        },
    )
}

fn causal_model(
    dataset: &qpls_data::Dataset,
    model_id: &str,
    variables: &[(&str, ObservedScaleV4)],
    edges: &[(&str, &str)],
) -> SemModelV4 {
    let (relations, parameters): (Vec<_>, Vec<_>) = edges
        .iter()
        .map(|(source, target)| relation(source, target))
        .unzip();
    SemModelV4 {
        schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
        id: model_id.into(),
        name: model_id.into(),
        variables: variables
            .iter()
            .map(|(id, scale)| observed(id, *scale))
            .collect(),
        relations,
        parameters,
        constraints: Vec::new(),
        derived_terms: Vec::new(),
        group: SemGroupV4::SingleGroup,
        data_binding: SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        },
        annotations: Vec::new(),
        presentation: SemPresentationV4::default(),
    }
}

fn reviewed_identification() -> CausalIdentificationChecklistV1 {
    CausalIdentificationChecklistV1 {
        temporal_order_declared: true,
        adjustment_set_justified: true,
        consistency_assumption_acknowledged: true,
        no_unmeasured_treatment_outcome_confounding_acknowledged: true,
        no_unmeasured_treatment_mediator_confounding_acknowledged: true,
        no_unmeasured_mediator_outcome_confounding_acknowledged: true,
        no_exposure_induced_mediator_outcome_confounder_confirmed: true,
        no_recanting_witness_confirmed: true,
        linear_model_specification_reviewed: true,
        positivity_reviewed: true,
    }
}

fn main_term(id: &str) -> CausalLinearTermV1 {
    CausalLinearTermV1 {
        term_id: format!("main:{id}"),
        factor_variable_ids: vec![id.into()],
    }
}

fn equation(id: &str, outcome: &str, factors: &[&str]) -> CausalLinearEquationV1 {
    CausalLinearEquationV1 {
        equation_id: id.into(),
        outcome_variable_id: outcome.into(),
        terms: factors.iter().map(|factor| main_term(factor)).collect(),
    }
}

fn recipe(
    dataset: &qpls_data::Dataset,
    model: &SemModelV4,
    fixture_id: u128,
    config: InterventionalCausalMediationConfigV1,
) -> Result<AnalysisRecipeV4, DynError> {
    let mut settings = AnalysisSettings::default();
    settings.method = AnalysisMethod::Regression;
    settings.seed = FIXTURE_SEED;
    settings.workers = 1;
    settings.bootstrap_samples = 0;
    settings.permutation_samples = 0;
    let mut value = AnalysisRecipeV4 {
        schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
        id: Uuid::from_u128(fixture_id),
        created_at: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .ok_or_else(|| invalid("fixed fixture timestamp is invalid"))?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model_binding: AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model.scientific_sha256()?,
        },
        estimand_confirmation: LegacyEstimandConfirmationV4::NotLegacy,
        settings,
        method_config: None,
        general_sem_config: None,
        mga_multigroup: None,
        pls_heterogeneity: None,
        general_sem_conditional_process: None,
        interventional_causal_mediation: Some(config),
        metadata: BTreeMap::new(),
        legacy_source: None,
    };
    finalize_recipe(&mut value, model)?;
    Ok(value)
}

fn orthogonal_residual(
    predictors: &[&[f64]],
    phase: usize,
    target_root_mean_square: f64,
) -> Result<Vec<f64>, DynError> {
    let count = predictors
        .first()
        .map(|values| values.len())
        .ok_or_else(|| invalid("orthogonal residual requires predictors"))?;
    if count == 0
        || predictors.iter().any(|values| values.len() != count)
        || !target_root_mean_square.is_finite()
        || target_root_mean_square <= 0.0
    {
        return Err(invalid("orthogonal residual inputs are invalid"));
    }
    let mut basis = Vec::<Vec<f64>>::new();
    for source in
        std::iter::once(vec![1.0; count]).chain(predictors.iter().map(|values| values.to_vec()))
    {
        let mut residual = source;
        // Reorthogonalization makes the evidence fixtures insensitive to the
        // modest conditioning differences between Rust and the independent
        // Python modified-Gram-Schmidt fit.
        for _ in 0..2 {
            for vector in &basis {
                let projection = residual
                    .iter()
                    .zip(vector)
                    .map(|(left, right)| left * right)
                    .sum::<f64>();
                for (value, direction) in residual.iter_mut().zip(vector) {
                    *value -= projection * direction;
                }
            }
        }
        let norm = residual
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if norm <= 1.0e-10 {
            return Err(invalid("causal DGP predictor design is rank deficient"));
        }
        basis.push(residual.into_iter().map(|value| value / norm).collect());
    }
    let phase = phase as f64;
    let mut residual = (0..count)
        .map(|row| {
            let value = row as f64 + 1.0;
            (value * (0.317 + phase * 0.019)).sin()
                + 0.37 * (value * (0.173 + phase * 0.011)).cos()
                + 0.11 * (value * (0.071 + phase * 0.007)).sin()
        })
        .collect::<Vec<_>>();
    for _ in 0..2 {
        for vector in &basis {
            let projection = residual
                .iter()
                .zip(vector)
                .map(|(left, right)| left * right)
                .sum::<f64>();
            for (value, direction) in residual.iter_mut().zip(vector) {
                *value -= projection * direction;
            }
        }
    }
    let norm = residual
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if norm <= 1.0e-10 {
        return Err(invalid(
            "causal DGP residual collapsed after orthogonalization",
        ));
    }
    let scale = target_root_mean_square * (count as f64).sqrt() / norm;
    Ok(residual.into_iter().map(|value| value * scale).collect())
}

fn recovery_contract(analytic_targets: Value) -> Value {
    json!({
        "analytic_targets_by_path": analytic_targets,
        "maximum_absolute_recovery_error": 5.0e-7,
        "minimum_absolute_nonzero_target": 0.20,
        "minimum_recovered_nonzero_fraction": 1.0,
        "confidence_intervals_must_exclude_zero": true,
        "design": "deterministic equation-orthogonal strong-signal recovery fixture",
        "power_claim": "none; interval exclusion is a predeclared detection guard, not a Monte Carlo power estimate"
    })
}

fn binary_two_fixture(scale: Scale, fail_positivity: bool) -> Result<CausalFixture, DynError> {
    let mut x = Vec::new();
    let mut c = Vec::new();
    let mut z = Vec::new();
    for row in 0..96 {
        let treatment = if fail_positivity {
            if row < 5 { 0.0 } else { 1.0 }
        } else {
            (row % 2) as f64
        };
        let moderator = ((row / 2) % 2) as f64;
        let covariate = ((row * 7 + 3) % 31) as f64 / 10.0 - 1.5;
        x.push(treatment);
        c.push(covariate);
        z.push(moderator);
    }
    let mediator_residual = orthogonal_residual(&[&x, &c, &z], 1, 0.035)?;
    let m = x
        .iter()
        .zip(&c)
        .zip(&z)
        .zip(&mediator_residual)
        .map(|(((treatment, covariate), moderator), residual)| {
            0.4 + 0.82 * treatment + 0.27 * covariate + 0.18 * moderator + residual
        })
        .collect::<Vec<_>>();
    let outcome_residual = orthogonal_residual(&[&x, &m, &c, &z], 2, 0.04)?;
    let y = x
        .iter()
        .zip(&m)
        .zip(&c)
        .zip(&z)
        .zip(&outcome_residual)
        .map(
            |((((treatment, mediator), covariate), moderator), residual)| {
                0.5 + 0.31 * treatment
                    + 0.58 * mediator
                    + 0.21 * covariate
                    + 0.12 * moderator
                    + residual
            },
        )
        .collect::<Vec<_>>();
    let headers = ["x", "c", "z", "m", "y"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns(
        "causal-binary-two-edge-v1.csv",
        &headers,
        &[numeric(x), numeric(c), numeric(z), numeric(m), numeric(y)],
    )?;
    let model = causal_model(
        &dataset,
        "causal-binary-two-edge-model-v1",
        &[
            ("x", ObservedScaleV4::Binary),
            ("c", ObservedScaleV4::Continuous),
            ("z", ObservedScaleV4::Binary),
            ("m", ObservedScaleV4::Continuous),
            ("y", ObservedScaleV4::Continuous),
        ],
        &[("x", "m"), ("m", "y"), ("x", "y")],
    );
    let config = InterventionalCausalMediationConfigV1 {
        schema_version: 1,
        treatment: "x".into(),
        treatment_contrast: ObservedTreatmentContrastV1::Binary {
            control: 0.0,
            treated: 1.0,
        },
        outcome: "y".into(),
        mediators: vec!["m".into()],
        baseline_moderators: vec!["z".into()],
        adjustment_covariates: vec!["c".into()],
        paths: vec![ObservedCausalPathV1 {
            path_id: "binary_two_edge".into(),
            ordered_variable_ids: vec!["x".into(), "m".into(), "y".into()],
            equations: vec![
                equation("binary_two:m", "m", &["x", "c", "z"]),
                equation("binary_two:y", "y", &["x", "m", "c", "z"]),
            ],
        }],
        positivity_policy: CausalPositivityPolicyV1 {
            minimum_binary_arm_count: 10,
            maximum_binary_arm_ratio: 10.0,
            positivity_strata_variable_ids: vec!["z".into()],
            minimum_count_per_binary_stratum_arm: 5,
            ..CausalPositivityPolicyV1::default()
        },
        identification: reviewed_identification(),
        bootstrap_resamples: scale.bootstrap_draws(),
        seed: FIXTURE_SEED,
        confidence_level: 0.95,
    };
    let recipe = recipe(
        &dataset,
        &model,
        0xca05_a100_0000_0000_0000_0000_0000_0001,
        config,
    )?;
    Ok(CausalFixture {
        dataset,
        recipe,
        model,
        dgp: json!({
            "treatment_kind": "binary",
            "selected_path_edges": [2],
            "known_target_basis": "predeclared analytic linear DGP truth plus independent OLS and g-computation over the emitted complete-case frame",
            "baseline_moderator": "z",
            "adjustment_set": ["c"],
            "recovery_contract": recovery_contract(json!({
                "binary_two_edge": {
                    "interventional_direct_effect": 0.31,
                    "joint_interventional_indirect_effect": 0.82 * 0.58,
                    "total_interventional_contrast": 0.31 + 0.82 * 0.58
                }
            }))
        }),
    })
}

fn binary_four_fixture(scale: Scale) -> Result<CausalFixture, DynError> {
    let x = (0..100).map(|row| (row % 2) as f64).collect::<Vec<_>>();
    let z = (0..100)
        .map(|row| ((row / 2) % 2) as f64)
        .collect::<Vec<_>>();
    let c = (0..100)
        .map(|row| ((row * 7 + 3) % 31) as f64 / 10.0 - 1.5)
        .collect::<Vec<_>>();
    let e1 = orthogonal_residual(&[&x, &c, &z], 3, 0.035)?;
    let m1 = x
        .iter()
        .zip(&c)
        .zip(&z)
        .zip(&e1)
        .map(|(((treatment, covariate), moderator), residual)| {
            -0.2 + 0.61 * treatment + 0.22 * covariate + 0.14 * moderator + residual
        })
        .collect::<Vec<_>>();
    let e2 = orthogonal_residual(&[&m1, &c, &z], 4, 0.035)?;
    let m2 = m1
        .iter()
        .zip(&c)
        .zip(&z)
        .zip(&e2)
        .map(|(((mediator, covariate), moderator), residual)| {
            0.1 + 0.73 * mediator + 0.19 * covariate + 0.09 * moderator + residual
        })
        .collect::<Vec<_>>();
    let e3 = orthogonal_residual(&[&m2, &c, &z], 5, 0.035)?;
    let m3 = m2
        .iter()
        .zip(&c)
        .zip(&z)
        .zip(&e3)
        .map(|(((mediator, covariate), moderator), residual)| {
            -0.1 + 0.67 * mediator + 0.17 * covariate + 0.08 * moderator + residual
        })
        .collect::<Vec<_>>();
    let ey = orthogonal_residual(&[&x, &m3, &c, &z], 6, 0.04)?;
    let y = x
        .iter()
        .zip(&m3)
        .zip(&c)
        .zip(&z)
        .zip(&ey)
        .map(
            |((((treatment, mediator), covariate), moderator), residual)| {
                0.5 + 0.31 * treatment
                    + 0.76 * mediator
                    + 0.21 * covariate
                    + 0.12 * moderator
                    + residual
            },
        )
        .collect::<Vec<_>>();
    let headers = ["x", "c", "z", "m1", "m2", "m3", "y"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns(
        "causal-binary-four-edge-v1.csv",
        &headers,
        &[
            numeric(x),
            numeric(c),
            numeric(z),
            numeric(m1),
            numeric(m2),
            numeric(m3),
            numeric(y),
        ],
    )?;
    let model = causal_model(
        &dataset,
        "causal-binary-four-edge-model-v1",
        &[
            ("x", ObservedScaleV4::Binary),
            ("c", ObservedScaleV4::Continuous),
            ("z", ObservedScaleV4::Binary),
            ("m1", ObservedScaleV4::Continuous),
            ("m2", ObservedScaleV4::Continuous),
            ("m3", ObservedScaleV4::Continuous),
            ("y", ObservedScaleV4::Continuous),
        ],
        &[
            ("x", "m1"),
            ("m1", "m2"),
            ("m2", "m3"),
            ("m3", "y"),
            ("x", "y"),
        ],
    );
    let config = InterventionalCausalMediationConfigV1 {
        schema_version: 1,
        treatment: "x".into(),
        treatment_contrast: ObservedTreatmentContrastV1::Binary {
            control: 0.0,
            treated: 1.0,
        },
        outcome: "y".into(),
        mediators: vec!["m1".into(), "m2".into(), "m3".into()],
        baseline_moderators: vec!["z".into()],
        adjustment_covariates: vec!["c".into()],
        paths: vec![ObservedCausalPathV1 {
            path_id: "binary_four_edge".into(),
            ordered_variable_ids: vec![
                "x".into(),
                "m1".into(),
                "m2".into(),
                "m3".into(),
                "y".into(),
            ],
            equations: vec![
                equation("binary_four:m1", "m1", &["x", "c", "z"]),
                equation("binary_four:m2", "m2", &["m1", "c", "z"]),
                equation("binary_four:m3", "m3", &["m2", "c", "z"]),
                equation("binary_four:y", "y", &["x", "m3", "c", "z"]),
            ],
        }],
        positivity_policy: CausalPositivityPolicyV1 {
            minimum_binary_arm_count: 10,
            maximum_binary_arm_ratio: 10.0,
            positivity_strata_variable_ids: vec!["z".into()],
            minimum_count_per_binary_stratum_arm: 5,
            ..CausalPositivityPolicyV1::default()
        },
        identification: reviewed_identification(),
        bootstrap_resamples: scale.bootstrap_draws(),
        seed: FIXTURE_SEED,
        confidence_level: 0.95,
    };
    let recipe = recipe(
        &dataset,
        &model,
        0xca05_a100_0000_0000_0000_0000_0000_0003,
        config,
    )?;
    Ok(CausalFixture {
        dataset,
        recipe,
        model,
        dgp: json!({
            "treatment_kind": "binary",
            "selected_path_edges": [4],
            "known_target_basis": "predeclared analytic linear DGP truth plus independent OLS and g-computation over the emitted complete-case frame",
            "baseline_moderator": "z",
            "adjustment_set": ["c"],
            "recovery_contract": recovery_contract(json!({
                "binary_four_edge": {
                    "interventional_direct_effect": 0.31,
                    "joint_interventional_indirect_effect": 0.61 * 0.73 * 0.67 * 0.76,
                    "total_interventional_contrast": 0.31 + 0.61 * 0.73 * 0.67 * 0.76
                }
            }))
        }),
    })
}

fn continuous_fixture(scale: Scale) -> Result<CausalFixture, DynError> {
    let mut x = Vec::new();
    let mut c = Vec::new();
    for row in 0..98 {
        let treatment = (row % 49) as f64 / 12.0 - 2.0;
        let covariate = ((row * 7 + 4) % 37) as f64 / 11.0 - 1.6;
        x.push(treatment);
        c.push(covariate);
    }
    let e1 = orthogonal_residual(&[&x, &c], 7, 0.035)?;
    let m1 = x
        .iter()
        .zip(&c)
        .zip(&e1)
        .map(|((treatment, covariate), residual)| {
            0.2 + 0.74 * treatment + 0.28 * covariate + residual
        })
        .collect::<Vec<_>>();
    let e2 = orthogonal_residual(&[&m1, &c], 8, 0.035)?;
    let m2 = m1
        .iter()
        .zip(&c)
        .zip(&e2)
        .map(|((mediator, covariate), residual)| {
            -0.1 + 0.69 * mediator + 0.24 * covariate + residual
        })
        .collect::<Vec<_>>();
    let ey = orthogonal_residual(&[&x, &m2, &c], 9, 0.04)?;
    let y = x
        .iter()
        .zip(&m2)
        .zip(&c)
        .zip(&ey)
        .map(|(((treatment, mediator), covariate), residual)| {
            0.4 + 0.36 * treatment + 0.83 * mediator + 0.19 * covariate + residual
        })
        .collect::<Vec<_>>();
    let headers = ["x", "c", "m1", "m2", "y"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns(
        "causal-continuous-v1.csv",
        &headers,
        &[numeric(x), numeric(c), numeric(m1), numeric(m2), numeric(y)],
    )?;
    let model = causal_model(
        &dataset,
        "causal-continuous-model-v1",
        &[
            ("x", ObservedScaleV4::Continuous),
            ("c", ObservedScaleV4::Continuous),
            ("m1", ObservedScaleV4::Continuous),
            ("m2", ObservedScaleV4::Continuous),
            ("y", ObservedScaleV4::Continuous),
        ],
        &[("x", "m1"), ("m1", "m2"), ("m2", "y"), ("x", "y")],
    );
    let config = InterventionalCausalMediationConfigV1 {
        schema_version: 1,
        treatment: "x".into(),
        treatment_contrast: ObservedTreatmentContrastV1::Continuous { x0: -0.5, x1: 0.75 },
        outcome: "y".into(),
        mediators: vec!["m1".into(), "m2".into()],
        baseline_moderators: Vec::new(),
        adjustment_covariates: vec!["c".into()],
        paths: vec![ObservedCausalPathV1 {
            path_id: "continuous_three_edge".into(),
            ordered_variable_ids: vec!["x".into(), "m1".into(), "m2".into(), "y".into()],
            equations: vec![
                equation("continuous:m1", "m1", &["x", "c"]),
                equation("continuous:m2", "m2", &["m1", "c"]),
                equation("continuous:y", "y", &["x", "m2", "c"]),
            ],
        }],
        positivity_policy: CausalPositivityPolicyV1 {
            continuous_neighborhood_fraction_of_range: 0.10,
            minimum_continuous_neighborhood_count: 5,
            ..CausalPositivityPolicyV1::default()
        },
        identification: reviewed_identification(),
        bootstrap_resamples: scale.bootstrap_draws(),
        seed: FIXTURE_SEED,
        confidence_level: 0.95,
    };
    let recipe = recipe(
        &dataset,
        &model,
        0xca05_a100_0000_0000_0000_0000_0000_0002,
        config,
    )?;
    Ok(CausalFixture {
        dataset,
        recipe,
        model,
        dgp: json!({
            "treatment_kind": "continuous",
            "contrast": {"x0": -0.5, "x1": 0.75},
            "selected_path_edges": [3],
            "known_target_basis": "predeclared analytic linear DGP truth plus independent OLS and g-computation over the emitted complete-case frame",
            "adjustment_set": ["c"],
            "recovery_contract": recovery_contract(json!({
                "continuous_three_edge": {
                    "interventional_direct_effect": 0.36 * 1.25,
                    "joint_interventional_indirect_effect": 0.74 * 0.69 * 0.83 * 1.25,
                    "total_interventional_contrast": (0.36 + 0.74 * 0.69 * 0.83) * 1.25
                }
            }))
        }),
    })
}

fn compact_ledger(
    ledger: &MultiModFinalLedgerV1<MultiModCaseBootstrapDrawV1, Vec<f64>>,
    target_count: usize,
) -> Value {
    let mut widths = BTreeSet::new();
    let successful_target_vectors = ledger
        .records
        .iter()
        .filter_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => Some(json!({
                "replicate_index": record.index,
                "target_values": value,
            })),
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let first_target_draws = ledger
        .records
        .iter()
        .filter_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => {
                widths.insert(value.len());
                value.first().copied()
            }
            MultiModRefitOutcomeV1::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    json!({
        "method_version": ledger.method_version,
        "execution_identity_sha256": ledger.execution_identity_sha256,
        "requested": ledger.requested,
        "usable": ledger.usable,
        "minimum_required": ledger.minimum_required,
        "usable_fraction": ledger.usable_fraction,
        "complete": ledger.complete,
        "ledger_sha256": ledger.ledger_sha256,
        "record_count": ledger.records.len(),
        "expected_target_vector_width": target_count,
        "successful_target_vector_widths": widths,
        "first_target_draws": first_target_draws,
        "successful_target_vectors": successful_target_vectors,
        "unique_record_identity_count": ledger.records.iter().map(|record| &record.record_identity_sha256).collect::<BTreeSet<_>>().len(),
    })
}

fn run_case(cell_ids: &[&str], case_id: &str, fixture: CausalFixture) -> Result<Value, DynError> {
    let mut fixture = fixture;
    fixture.recipe.settings.workers =
        metamorphic::configured_workers_v1(fixture.recipe.settings.workers).map_err(invalid)?;
    metamorphic::transform_model_declaration_order_v1(&mut fixture.model);
    finalize_recipe(&mut fixture.recipe, &fixture.model)?;
    let artifact = compile_multimod_recipe_v1(
        &fixture.recipe,
        &fixture.model,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    )?;
    let compiled_again = compile_multimod_recipe_v1(
        &fixture.recipe,
        &fixture.model,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    )?;
    let config = fixture
        .recipe
        .interventional_causal_mediation
        .as_ref()
        .ok_or_else(|| invalid("causal fixture lost its configuration"))?;
    let prepared = prepare_interventional_causal_inputs_from_dataset_v1(
        &fixture.dataset,
        &fixture.model,
        config,
        None,
    )?;
    let output = run_compiled_interventional_causal_mediation_raw_v1(
        &fixture.dataset,
        &fixture.recipe,
        &fixture.model,
        &artifact,
        || false,
        |_| {},
    )?;
    let result = match &output.result {
        MultiModAnalysisResultV1::InterventionalMediationResultV1(result) => result,
        _ => return Err(invalid("causal runner returned another result family")),
    };
    let ledger = output
        .evidence
        .iter()
        .find_map(|evidence| match evidence {
            MultiModRunnerEvidenceV1::InterventionalFullRefitLedger(ledger) => Some(ledger),
            _ => None,
        })
        .ok_or_else(|| invalid("causal runner omitted its full-refit ledger"))?;
    let target_ids = result
        .effects
        .iter()
        .map(|effect| effect.target_id.clone())
        .collect::<BTreeSet<_>>();
    let path_ids = result
        .effects
        .iter()
        .map(|effect| effect.path_id.clone())
        .collect::<BTreeSet<_>>();
    Ok(json!({
        "case_id": case_id,
        "cell_ids": cell_ids,
        "dataset_fingerprint": fixture.dataset.fingerprint.0,
        "dataset_rows": fixture.dataset.batch.num_rows(),
        "dgp": fixture.dgp,
        "recipe_id": fixture.recipe.id,
        "model_id": fixture.model.id,
        "compiler_receipt": artifact.receipt(),
        "compiled_plan": artifact.plan(),
        "deterministic_recompile_equal": artifact == compiled_again,
        "prepared_paths": prepared,
        "result": result,
        "bootstrap_evidence": compact_ledger(ledger, result.effects.len()),
        "target_ids_are_unique": target_ids.len() == result.effects.len(),
        "path_id_inventory": path_ids,
        "interpretation_contains_assumption_dependent_interventional_estimate": result.interpretation_label.to_lowercase().contains("assumption-dependent interventional estimate"),
        "interpretation_avoids_causality_established": !result.interpretation_label.to_lowercase().contains("causality established"),
    }))
}

fn config_blocker(
    mut config: InterventionalCausalMediationConfigV1,
    mutate: impl FnOnce(&mut InterventionalCausalMediationConfigV1),
) -> Value {
    mutate(&mut config);
    match config.ensure_valid() {
        Ok(()) => json!({"status": "unexpectedly_admitted"}),
        Err(error) => json!({
            "status": "blocked",
            "code": error.code,
            "path": error.path,
            "message": error.message,
        }),
    }
}

fn estimator_scope_blocker(
    prepared: &PreparedInterventionalDatasetPathV1,
    mutate: impl FnOnce(&mut qpls_estimation::InterventionalCausalMediationInputV1),
) -> Value {
    let mut input = prepared.input.clone();
    mutate(&mut input);
    match estimate_interventional_mediation_v1(&input) {
        Ok(result) => json!({"status": "unexpectedly_admitted", "target_id": result.target_id}),
        Err(blockers) => blocker_inventory(&blockers),
    }
}

fn blocker_inventory(blockers: &[InterventionalMediationBlockerV1]) -> Value {
    json!({
        "status": "blocked",
        "codes": blockers.iter().map(|blocker| format!("{:?}", blocker.code)).collect::<BTreeSet<_>>(),
        "blockers": blockers,
    })
}

fn compiler_scope_blocker(
    mut fixture: CausalFixture,
    mutate: impl FnOnce(&mut SemModelV4),
) -> Value {
    mutate(&mut fixture.model);
    fixture.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: fixture.model.scientific_sha256().unwrap_or_default(),
        model: fixture.model.clone(),
    };
    match compile_multimod_recipe_v1(
        &fixture.recipe,
        &fixture.model,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    ) {
        Ok(_) => json!({"status": "unexpectedly_admitted"}),
        Err(error) => json!({"status": "blocked", "error": error.to_string()}),
    }
}

fn assumption_and_scope_blockers(scale: Scale) -> Result<Value, DynError> {
    let valid = binary_two_fixture(scale, false)?;
    let config = valid
        .recipe
        .interventional_causal_mediation
        .clone()
        .ok_or_else(|| invalid("binary causal fixture lacks config"))?;
    let prepared = prepare_interventional_causal_inputs_from_dataset_v1(
        &valid.dataset,
        &valid.model,
        &config,
        None,
    )?;
    let prepared_path = prepared
        .first()
        .ok_or_else(|| invalid("binary causal fixture produced no path"))?;

    let positivity_fixture = binary_two_fixture(scale, true)?;
    let positivity_artifact = compile_multimod_recipe_v1(
        &positivity_fixture.recipe,
        &positivity_fixture.model,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    )?;
    let positivity = match run_compiled_interventional_causal_mediation_raw_v1(
        &positivity_fixture.dataset,
        &positivity_fixture.recipe,
        &positivity_fixture.model,
        &positivity_artifact,
        || false,
        |_| {},
    ) {
        Ok(_) => json!({"status": "unexpectedly_admitted"}),
        Err(error) => json!({"status": "blocked", "error": error.to_string()}),
    };

    let groups = compiler_scope_blocker(binary_two_fixture(scale, false)?, |model| {
        model.variables.push(SemVariableV4::Observed {
            id: "group".into(),
            label: "Group".into(),
            source_column: "group".into(),
            scale: ObservedScaleV4::Nominal,
            role: ObservedRoleV4::Control,
            categories: vec!["A".into(), "B".into()],
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "group".into(),
            levels: vec![
                SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "A".into(),
                },
                SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "B".into(),
                },
            ],
        };
    });
    let weights = compiler_scope_blocker(binary_two_fixture(scale, false)?, |model| {
        model.variables.push(SemVariableV4::Observed {
            id: "weight".into(),
            label: "Weight".into(),
            source_column: "weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        if let SemDataBindingV4::Raw { weight, .. } = &mut model.data_binding {
            *weight = Some(SemWeightBindingV4::Case {
                variable: "weight".into(),
            });
        }
    });

    Ok(json!({
        "cell_ids": [
            "interventional.observed_gcomp.v1::positivity_diagnostics",
            "interventional.observed_gcomp.v1::identification_failure_guards"
        ],
        "missing_adjustment_set": config_blocker(config.clone(), |value| value.adjustment_covariates.clear()),
        "temporal_order_unreviewed": config_blocker(config.clone(), |value| value.identification.temporal_order_declared = false),
        "recanting_witness_not_excluded": config_blocker(config.clone(), |value| value.identification.no_recanting_witness_confirmed = false),
        "exposure_induced_confounding_not_excluded": config_blocker(config.clone(), |value| value.identification.no_exposure_induced_mediator_outcome_confounder_confirmed = false),
        "positivity_failure_from_raw_runner": positivity,
        "natural_or_cross_world_effect_request": estimator_scope_blocker(prepared_path, |input| input.unsupported_features.natural_or_cross_world_effects = true),
        "recanting_witness_present": estimator_scope_blocker(prepared_path, |input| input.unsupported_features.recanting_witness_present = true),
        "exposure_induced_confounder_present": estimator_scope_blocker(prepared_path, |input| input.unsupported_features.exposure_induced_mediator_outcome_confounder_present = true),
        "latent_composite_or_hoc_role_request": estimator_scope_blocker(prepared_path, |input| input.unsupported_features.latent_composite_or_hoc_roles = true),
        "group_request_compile": groups,
        "weight_request_compile": weights,
    }))
}

fn required_cells() -> Vec<&'static str> {
    vec![
        "interventional.observed_gcomp.v1::observed_equation_point_fit",
        "interventional.observed_gcomp.v1::parametric_g_computation",
        "interventional.observed_gcomp.v1::known_target_simulation",
        "interventional.observed_gcomp.v1::causal_wording_guard",
        "interventional.observed_gcomp.v1::positivity_diagnostics",
        "interventional.observed_gcomp.v1::identification_failure_guards",
    ]
}
