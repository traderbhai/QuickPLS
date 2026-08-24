//! End-to-end qualification producer for General SEM conditional-process V2.
//!
//! This executable deliberately uses the public Recipe V4 compiler and the
//! built-in raw-data runner for every admitted profile.  It emits compact,
//! deterministic evidence summaries; it does not decide release qualification.

#[path = "support_multimod_metamorphic/mod.rs"]
mod metamorphic;
#[path = "support_multimod_qualification/mod.rs"]
mod support;

use qpls_core::*;
use qpls_resampling::{
    MultiModDeleteOneJackknifeDrawV1, MultiModFinalLedgerV1, MultiModFrequencyBootstrapDrawV1,
    MultiModJackknifePlanV1, MultiModRefitFailureV1, MultiModRefitOutcomeV1, MultiModShardSpecV1,
    MultiModStudentizedFinalLedgerV1, finalize_multimod_delete_one_jackknife_v1,
    run_multimod_delete_one_jackknife_shard_v1,
};
use qpls_runner::{
    ConditionalProcessRawAuthorityV2, GroupConditionalCaseLedgerV2,
    RawConditionalProcessEvidenceV2, run_compiled_general_sem_conditional_process_raw_v2,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use support::*;

const PRODUCER_ID: &str = "qpls.multimod.conditional.raw-qualification.v1";
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

    fn percentile_draws(self) -> u32 {
        // Both scales stay inside the admitted public envelope. Qualification
        // uses the product defaults; development remains a real, not mocked,
        // production run.
        match self {
            Self::Development => 500,
            Self::Qualification => 5_000,
        }
    }

    fn studentized_draws(self) -> (u32, u32) {
        match self {
            Self::Development => (500, 100),
            Self::Qualification => (1_000, 200),
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
        eprintln!("MMQ.CONDITIONAL.PRODUCER: {error}");
        std::process::exit(2);
    }
}

fn execute() -> Result<(), DynError> {
    let arguments = arguments()?;
    match &arguments.mode {
        ExecutionMode::Monolithic => execute_monolithic(&arguments),
        ExecutionMode::Plan => write_conditional_shard_plan(&arguments),
        ExecutionMode::Shard(shard_id) => run_conditional_shard(&arguments, shard_id),
    }
}

fn execute_monolithic(arguments: &Arguments) -> Result<(), DynError> {
    let mut cases = Vec::new();

    let alternatives = if metamorphic::compact_matrix_v1() {
        vec![InferenceAlternativeV1::TwoSided]
    } else {
        all_alternatives().to_vec()
    };

    for alternative in alternatives.iter().copied() {
        cases.push(run_multi_path_case(arguments.scale, alternative)?);
    }
    for alternative in alternatives.iter().copied() {
        cases.push(run_bca_case(arguments.scale, alternative, false)?);
    }
    if !metamorphic::compact_matrix_v1() {
        for alternative in alternatives.iter().copied() {
            cases.push(run_bca_case(arguments.scale, alternative, true)?);
        }
    }
    for alternative in alternatives.iter().copied() {
        cases.push(run_studentized_case(arguments.scale, alternative)?);
    }
    for alternative in alternatives {
        cases.push(run_three_way_case(arguments.scale, alternative)?);
    }
    cases.push(run_hoc_case(arguments.scale)?);
    cases.push(run_grouped_case(arguments.scale)?);
    cases.push(run_weighted_case(arguments.scale, false)?);
    cases.push(run_weighted_case(arguments.scale, true)?);

    let report = json!({
        "schema_version": 1,
        "producer_id": PRODUCER_ID,
        "family": "conditional_process_v2",
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "qualification_claim": "none",
        "execution_contract": "public_recipe_v4_compiler_plus_builtin_raw_runner",
        "cases": cases,
        "qualification_boundary_guards": qualification_boundary_guards()?,
        "unsupported_intersections": unsupported_intersections()?,
        "required_cell_ids": required_cells(),
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

fn conditional_shard_specs() -> Vec<Value> {
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
            "shard_id": "qualification-guards",
            "payload_kind": "evidence",
            "dependencies": ["sentinel"],
            "resource_class": "light",
            "parallel_safe_after_build": true,
            "scientific_identity": {
                "identity_kind": "evidence",
                "identity_id": "conditional-qualification-guards-v1",
            },
        }),
    ];
    for prefix in [
        "multi-path",
        "bca-non-null",
        "bca-null",
        "studentized",
        "three-way",
    ] {
        for alternative in ["two_sided", "less", "greater"] {
            let (case_id, fixture_role) = match prefix {
                "multi-path" => (format!("multi_path_percentile:{alternative}"), "non_null"),
                "bca-non-null" => (format!("bca:non_null:{alternative}"), "non_null"),
                "bca-null" => (format!("bca:null:{alternative}"), "null"),
                "studentized" => (format!("studentized:non_null:{alternative}"), "non_null"),
                "three-way" => (format!("bounded_three_way:{alternative}"), "non_null"),
                _ => unreachable!("conditional shard prefix is frozen"),
            };
            rows.push(json!({
                "shard_id": format!("{prefix}-{alternative}"),
                "payload_kind": "case",
                "dependencies": ["sentinel"],
                "resource_class": "heavy",
                "parallel_safe_after_build": true,
                "scientific_identity": {
                    "identity_kind": "case",
                    "case_id": case_id,
                    "alternative": alternative,
                    "fixture_role": fixture_role,
                },
            }));
        }
    }
    for (shard_id, case_id) in [
        ("hoc", "multiple_hoc:four_disjoint"),
        ("grouped", "grouped:stratified"),
        ("case-weighted", "case_weighted:positive"),
        (
            "frequency-weighted",
            "frequency_weighted:count_space_and_physical_expansion",
        ),
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
                "alternative": "two_sided",
                "fixture_role": "non_null",
            },
        }));
    }
    rows
}

fn conditional_shard_plan(arguments: &Arguments) -> Result<Value, DynError> {
    if metamorphic::compact_matrix_v1() {
        return Err(invalid(
            "the resumable qualification plan is unavailable for compact metamorphic fixtures; use monolithic metamorphic execution",
        ));
    }
    Ok(json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_PLAN_SUITE_ID,
        "family": "conditional",
        "producer_id": PRODUCER_ID,
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "execution_contract": "one_cargo_build_then_exact_resumable_case_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": conditional_shard_specs(),
    }))
}

fn conditional_header(arguments: &Arguments) -> Result<Value, DynError> {
    Ok(json!({
        "schema_version": 1,
        "producer_id": PRODUCER_ID,
        "family": "conditional_process_v2",
        "scale": arguments.scale.as_str(),
        "seed": FIXTURE_SEED,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "qualification_claim": "none",
        "execution_contract": "public_recipe_v4_compiler_plus_builtin_raw_runner",
        "required_cell_ids": required_cells(),
    }))
}

fn write_conditional_shard_plan(arguments: &Arguments) -> Result<(), DynError> {
    if let Some(parent) = arguments.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &arguments.output,
        serde_json::to_vec_pretty(&conditional_shard_plan(arguments)?)?,
    )?;
    Ok(())
}

fn alternative_from_shard(shard_id: &str, prefix: &str) -> Option<InferenceAlternativeV1> {
    let suffix = shard_id.strip_prefix(prefix)?;
    match suffix {
        "two_sided" => Some(InferenceAlternativeV1::TwoSided),
        "less" => Some(InferenceAlternativeV1::Less),
        "greater" => Some(InferenceAlternativeV1::Greater),
        _ => None,
    }
}

fn validate_conditional_dependencies(
    arguments: &Arguments,
    shard_id: &str,
) -> Result<Vec<String>, DynError> {
    let expected = if shard_id == "sentinel" {
        Vec::new()
    } else if conditional_shard_specs()
        .iter()
        .any(|row| row["shard_id"] == shard_id)
    {
        vec!["sentinel".to_owned()]
    } else {
        return Err(invalid(format!("unknown conditional shard {shard_id}")));
    };
    let mut actual = Vec::new();
    let expected_sign_columns = serde_json::to_value(sign_columns_identity()?)?;
    let expected_workers = metamorphic::configured_workers_v1(1).map_err(invalid)? as u64;
    for path in &arguments.dependencies {
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if value["schema_version"].as_u64() != Some(u64::from(SHARD_SCHEMA_VERSION))
            || value["suite_id"] != SHARD_SUITE_ID
            || value["family"] != "conditional"
            || value["producer_id"] != PRODUCER_ID
            || value["scale"] != arguments.scale.as_str()
            || value["seed"].as_u64() != Some(FIXTURE_SEED)
            || value["metamorphism"] != metamorphic::metamorphism_v1()
            || value.get("sign_columns") != Some(&expected_sign_columns)
            || value["workers"].as_u64() != Some(expected_workers)
        {
            return Err(invalid(format!(
                "conditional dependency {} has the wrong identity",
                path.display()
            )));
        }
        actual.push(
            value["shard_id"]
                .as_str()
                .ok_or_else(|| invalid("conditional dependency shard id is absent"))?
                .to_owned(),
        );
    }
    actual.sort();
    if actual != expected {
        return Err(invalid(format!(
            "conditional shard {shard_id} requires dependencies {expected:?}, received {actual:?}"
        )));
    }
    Ok(actual)
}

fn conditional_sentinel(arguments: &Arguments) -> Result<Value, DynError> {
    let (dataset, recipe, model) = compact_two_stage_fixture(
        Scale::Development,
        ConditionalProcessProfileV2::MultiTwoWayPercentile,
        ConditionalProcessIntervalV2::Percentile,
        InferenceAlternativeV1::TwoSided,
        false,
    )?;
    let diagnostic = finish_and_run(
        &[],
        "conditional_fast_root_sentinel",
        "diagnostic_only",
        dataset,
        recipe,
        model,
    )?;
    Ok(json!({
        "sentinel_id": "fast-root-development",
        "header": conditional_header(arguments)?,
        "diagnostic_production_case": diagnostic,
        "claim": "development-sized fail-fast production sentinel; excluded from qualification cases",
    }))
}

fn conditional_evidence_shard(shard_id: &str) -> Result<Value, DynError> {
    match shard_id {
        "qualification-guards" => Ok(json!({
            "evidence_id": "conditional-qualification-guards-v1",
            "qualification_boundary_guards": qualification_boundary_guards()?,
            "unsupported_intersections": unsupported_intersections()?,
        })),
        _ => Err(invalid(format!(
            "unknown conditional evidence shard {shard_id}"
        ))),
    }
}

fn conditional_case_shard(arguments: &Arguments, shard_id: &str) -> Result<Value, DynError> {
    if let Some(alternative) = alternative_from_shard(shard_id, "multi-path-") {
        return run_multi_path_case(arguments.scale, alternative);
    }
    if let Some(alternative) = alternative_from_shard(shard_id, "bca-non-null-") {
        return run_bca_case(arguments.scale, alternative, false);
    }
    if let Some(alternative) = alternative_from_shard(shard_id, "bca-null-") {
        return run_bca_case(arguments.scale, alternative, true);
    }
    if let Some(alternative) = alternative_from_shard(shard_id, "studentized-") {
        return run_studentized_case(arguments.scale, alternative);
    }
    if let Some(alternative) = alternative_from_shard(shard_id, "three-way-") {
        return run_three_way_case(arguments.scale, alternative);
    }
    match shard_id {
        "hoc" => run_hoc_case(arguments.scale),
        "grouped" => run_grouped_case(arguments.scale),
        "case-weighted" => run_weighted_case(arguments.scale, false),
        "frequency-weighted" => run_weighted_case(arguments.scale, true),
        _ => Err(invalid(format!(
            "unknown conditional case shard {shard_id}"
        ))),
    }
}

fn run_conditional_shard(arguments: &Arguments, shard_id: &str) -> Result<(), DynError> {
    let dependency_shard_ids = validate_conditional_dependencies(arguments, shard_id)?;
    let scientific_identity = conditional_shard_specs()
        .into_iter()
        .find(|row| row["shard_id"] == shard_id)
        .ok_or_else(|| invalid(format!("unknown conditional shard {shard_id}")))?["scientific_identity"]
        .clone();
    let payload = if shard_id == "sentinel" {
        json!({"kind": "sentinel", "value": conditional_sentinel(arguments)?})
    } else if shard_id == "qualification-guards" {
        json!({"kind": "evidence", "value": conditional_evidence_shard(shard_id)?})
    } else {
        json!({"kind": "case", "value": conditional_case_shard(arguments, shard_id)?})
    };
    let report = json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_SUITE_ID,
        "family": "conditional",
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

fn all_alternatives() -> [InferenceAlternativeV1; 3] {
    [
        InferenceAlternativeV1::TwoSided,
        InferenceAlternativeV1::Less,
        InferenceAlternativeV1::Greater,
    ]
}

fn alternative_id(value: InferenceAlternativeV1) -> &'static str {
    match value {
        InferenceAlternativeV1::TwoSided => "two_sided",
        InferenceAlternativeV1::Less => "less",
        InferenceAlternativeV1::Greater => "greater",
    }
}

fn standardized_probe(id: &str) -> ConditionalModeratorProbeV2 {
    ConditionalModeratorProbeV2 {
        probe_id: format!("probe:{id}"),
        moderator_id: id.into(),
        scale: ConditionalProbeScaleV2::StandardizedScore,
        values: vec![-1.0, 0.0, 1.0],
        raw_transformation_receipt: None,
        raw_fit_metric_receipts: Vec::new(),
    }
}

fn joint_tuple(id: &str, values: &[(&str, f64)]) -> ConditionalJointProbeTupleV2 {
    ConditionalJointProbeTupleV2 {
        tuple_id: id.into(),
        values_by_moderator: values
            .iter()
            .map(|(moderator, value)| ((*moderator).into(), *value))
            .collect(),
    }
}

fn estimands(all: bool) -> ConditionalProcessEstimandsV2 {
    ConditionalProcessEstimandsV2 {
        conditional_specific_indirect: true,
        conditional_total_indirect: all,
        conditional_total_effect: all,
        scalar_index_when_affine: true,
        local_first_derivatives: true,
        local_second_and_cross_derivatives: true,
        finite_probe_contrasts: true,
    }
}

fn inference(
    scale: Scale,
    interval: ConditionalProcessIntervalV2,
    alternative: InferenceAlternativeV1,
) -> ConditionalProcessInferenceV2 {
    let (outer_resamples, inner_resamples) = match interval {
        ConditionalProcessIntervalV2::Studentized => scale.studentized_draws(),
        _ => (scale.percentile_draws(), 0),
    };
    ConditionalProcessInferenceV2 {
        interval,
        alternative,
        outer_resamples,
        inner_resamples,
        seed: FIXTURE_SEED,
        confidence_level: 0.95,
    }
}

fn construct_id(id: &str) -> String {
    format!("construct:{id}")
}

fn indicator_id(id: &str) -> String {
    format!("{id}_i")
}

fn add_interaction_by_construct(
    model: &mut SemModelV4,
    id: &str,
    operands: &[&str],
    focal: &str,
    outcome: &str,
) -> Result<(), DynError> {
    let operands = operands
        .iter()
        .map(|id| construct_id(id))
        .collect::<Vec<_>>();
    let operand_refs = operands.iter().map(String::as_str).collect::<Vec<_>>();
    add_interaction(
        model,
        id,
        &operand_refs,
        &construct_id(focal),
        &construct_id(outcome),
    )
}

fn selected_path(
    model: &SemModelV4,
    path_id: &str,
    nodes: &[&str],
) -> Result<ConditionalProcessPathV2, DynError> {
    Ok(ConditionalProcessPathV2 {
        path_id: path_id.into(),
        ordered_relation_ids: nodes
            .windows(2)
            .map(|edge| relation_id(model, &construct_id(edge[0]), &construct_id(edge[1])))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn finish_and_run(
    cell_ids: &[&str],
    case_id: &str,
    fixture_role: &str,
    dataset: qpls_data::Dataset,
    mut recipe: AnalysisRecipeV4,
    mut model: SemModelV4,
) -> Result<Value, DynError> {
    recipe.settings.workers =
        metamorphic::configured_workers_v1(recipe.settings.workers).map_err(invalid)?;
    metamorphic::transform_model_declaration_order_v1(&mut model);
    finalize_recipe(&mut recipe, &model)?;
    let artifact = compile_multimod_recipe_v1(
        &recipe,
        &model,
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2,
    )?;
    let compiled_again = compile_multimod_recipe_v1(
        &recipe,
        &model,
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2,
    )?;
    let run = run_compiled_general_sem_conditional_process_raw_v2(
        &dataset,
        &recipe,
        &model,
        &artifact,
        ConditionalProcessRawAuthorityV2::BuiltIn,
        || false,
        |_| {},
    )?;
    let result = match &run.output.result {
        MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(result) => result,
        _ => return Err(invalid("conditional runner returned another result family")),
    };
    let conditional_config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| invalid("conditional fixture lost its configuration"))?;
    let evidence = evidence_summary(&run.raw_evidence, result.targets.len())?;
    let target_kinds = result
        .targets
        .iter()
        .map(|target| serde_json::to_string(&target.kind))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let path_ids = result
        .targets
        .iter()
        .map(|target| target.path_id.clone())
        .collect::<BTreeSet<_>>();
    let usable_counts = result
        .targets
        .iter()
        .map(|target| target.usable_replicates)
        .collect::<BTreeSet<_>>();
    Ok(json!({
        "case_id": case_id,
        "cell_ids": cell_ids,
        "fixture_role": fixture_role,
        "dataset_fingerprint": dataset.fingerprint.0,
        "dataset_rows": dataset.batch.num_rows(),
        "recipe_id": recipe.id,
        "model_id": model.id,
        "compiler_receipt": artifact.receipt(),
        "compiled_plan": artifact.plan(),
        "conditional_config": conditional_config,
        "deterministic_recompile_equal": artifact == compiled_again,
        "result": result,
        "analysis_frame": run.preparation,
        "original_sample_point_fits": run.point_fits,
        "evidence": evidence,
        "target_kind_inventory": target_kinds,
        "path_id_inventory": path_ids,
        "target_usable_replicate_counts": usable_counts,
        "one_result_ledger_for_all_targets": usable_counts.len() == 1,
    }))
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

fn compact_case_ledger<Draw: Serialize>(
    ledger: &MultiModFinalLedgerV1<Draw, Vec<f64>>,
    target_count: usize,
) -> Result<Value, DynError> {
    let mut first_target_draws = Vec::new();
    let mut widths = BTreeSet::new();
    let mut record_identities = Vec::with_capacity(ledger.records.len());
    let mut successful_target_vectors = Vec::with_capacity(ledger.usable as usize);
    let mut draws_with_case_weights = 0usize;
    let mut draws_without_case_weights = 0usize;
    let mut case_weight_identity_sha256s = BTreeSet::new();
    for record in &ledger.records {
        record_identities.push(record.record_identity_sha256.clone());
        let serialized_draw = serde_json::to_value(&record.draw)?;
        if let Some(object) = serialized_draw.as_object()
            && object.contains_key("case_weights")
        {
            match object.get("case_weights") {
                Some(Value::Array(weights)) if !weights.is_empty() => {
                    draws_with_case_weights += 1;
                    if let Some(identity) =
                        object.get("case_weights_sha256").and_then(Value::as_str)
                    {
                        case_weight_identity_sha256s.insert(identity.to_owned());
                    }
                }
                Some(Value::Null) => draws_without_case_weights += 1,
                _ => {}
            }
        }
        if let MultiModRefitOutcomeV1::Success { value, .. } = &record.outcome {
            widths.insert(value.len());
            if let Some(first) = value.first() {
                first_target_draws.push(*first);
            }
            successful_target_vectors.push(json!({
                "replicate_index": record.index,
                "target_values": value,
            }));
        }
    }
    Ok(json!({
        "method_version": ledger.method_version,
        "execution_identity_sha256": ledger.execution_identity_sha256,
        "requested": ledger.requested,
        "usable": ledger.usable,
        "minimum_required": ledger.minimum_required,
        "usable_fraction": ledger.usable_fraction,
        "complete": ledger.complete,
        "ledger_sha256": ledger.ledger_sha256,
        "record_count": ledger.records.len(),
        "unique_record_identity_count": record_identities.iter().collect::<BTreeSet<_>>().len(),
        "record_identity_sha256s": record_identities,
        "draws_with_case_weights": draws_with_case_weights,
        "draws_without_case_weights": draws_without_case_weights,
        "unique_case_weight_identity_count": case_weight_identity_sha256s.len(),
        "case_weight_identity_sha256s": case_weight_identity_sha256s,
        "successful_target_vector_widths": widths,
        "expected_target_vector_width": target_count,
        "first_target_draws": first_target_draws,
        "successful_target_vectors": successful_target_vectors,
    }))
}

fn sample_standard_error(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let standard_error = variance.sqrt();
    standard_error.is_finite().then_some(standard_error)
}

fn compact_studentized_ledger(
    ledger: &MultiModStudentizedFinalLedgerV1<Vec<f64>, Vec<f64>>,
    target_count: usize,
) -> Value {
    let usable_outer_indices = ledger
        .usable_outer_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let outer_summaries = ledger
        .records
        .iter()
        .filter_map(|record| {
            if !usable_outer_indices.contains(&record.outer.index) {
                return None;
            }
            let MultiModRefitOutcomeV1::Success { value: outer, .. } = &record.outer.outcome else {
                return None;
            };
            let inner_vectors = record
                .inner_records
                .iter()
                .filter_map(|inner| match &inner.outcome {
                    MultiModRefitOutcomeV1::Success { value, .. }
                        if value.len() == target_count =>
                    {
                        Some(value)
                    }
                    MultiModRefitOutcomeV1::Failed { .. } => None,
                    MultiModRefitOutcomeV1::Success { .. } => None,
                })
                .collect::<Vec<_>>();
            let inner_standard_errors = (0..target_count)
                .map(|target_index| {
                    sample_standard_error(
                        &inner_vectors
                            .iter()
                            .map(|values| values[target_index])
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            if inner_standard_errors.iter().any(|value| match value {
                Some(value) => !value.is_finite() || *value <= 0.0,
                None => true,
            }) {
                return None;
            }
            Some(json!({
                "replicate_index": record.outer.index,
                "outer_first_target": outer.first(),
                "outer_target_values": outer,
                "outer_target_width": outer.len(),
                "inner_usable": inner_vectors.len(),
                "inner_first_target_standard_error": inner_standard_errors.first(),
                "inner_target_standard_errors": inner_standard_errors,
                "inner_ledger_identity_sha256": record.inner_ledger_identity_sha256,
            }))
        })
        .collect::<Vec<_>>();
    let prepared_usable_outer = outer_summaries.len();
    json!({
        "method_version": ledger.method_version,
        "execution_identity_sha256": ledger.execution_identity_sha256,
        "requested_outer": ledger.requested_outer,
        "usable_outer": prepared_usable_outer,
        "orchestrator_usable_outer": ledger.usable_outer,
        "minimum_outer_required": ledger.minimum_outer_required,
        "requested_inner_per_outer": ledger.requested_inner_per_outer,
        "minimum_inner_required": ledger.minimum_inner_required,
        "complete": ledger.complete,
        "ledger_sha256": ledger.ledger_sha256,
        "record_count": ledger.records.len(),
        "expected_target_vector_width": target_count,
        "outer_first_target_summaries": outer_summaries,
    })
}

fn compact_frequency_ledger(
    ledger: &MultiModFinalLedgerV1<MultiModFrequencyBootstrapDrawV1, Vec<f64>>,
    target_count: usize,
) -> Result<Value, DynError> {
    let mut value = compact_case_ledger(ledger, target_count)?;
    let draws = ledger
        .records
        .iter()
        .map(|record| {
            json!({
                "replicate_index": record.index,
                "counts": record.draw.counts,
                "total_count": record.draw.total_count,
                "counts_sha256": record.draw.counts_sha256,
                "draw_identity_sha256": record.draw.draw_identity_sha256,
            })
        })
        .collect::<Vec<_>>();
    value["count_space_draws"] = json!(draws);
    Ok(value)
}

fn evidence_summary(
    evidence: &RawConditionalProcessEvidenceV2,
    target_count: usize,
) -> Result<Value, DynError> {
    match evidence {
        RawConditionalProcessEvidenceV2::PercentileCase { bootstrap } => Ok(json!({
            "kind": "percentile_case",
            "bootstrap": compact_case_ledger(bootstrap, target_count)?,
        })),
        RawConditionalProcessEvidenceV2::BcaCase {
            bootstrap,
            delete_one,
        } => Ok(json!({
            "kind": "bca_case",
            "bootstrap": compact_case_ledger(bootstrap, target_count)?,
            "delete_one": compact_case_ledger(delete_one, target_count)?,
            "complete_delete_one": delete_one.complete && delete_one.usable == delete_one.requested,
        })),
        RawConditionalProcessEvidenceV2::StudentizedCase {
            nested,
            observed_inner,
        } => Ok(json!({
            "kind": "studentized_case",
            "nested": compact_studentized_ledger(nested, target_count),
            "observed_inner": compact_case_ledger(observed_inner, target_count)?,
            "no_percentile_fallback": true,
        })),
        RawConditionalProcessEvidenceV2::GroupedStratified { groups } => Ok(json!({
            "kind": "grouped_stratified",
            "groups": compact_group_ledgers(groups, target_count)?,
        })),
        RawConditionalProcessEvidenceV2::FrequencyCountSpace { bootstrap } => Ok(json!({
            "kind": "frequency_count_space",
            "bootstrap": compact_frequency_ledger(bootstrap, target_count)?,
            "physical_expansion_used": false,
        })),
    }
}

fn compact_group_ledgers(
    groups: &[GroupConditionalCaseLedgerV2],
    target_count: usize,
) -> Result<Vec<Value>, DynError> {
    groups
        .iter()
        .map(|group| {
            let stratum_target_count = group
                .ledger
                .records
                .iter()
                .find_map(|record| match &record.outcome {
                    MultiModRefitOutcomeV1::Success { value, .. } => Some(value.len()),
                    MultiModRefitOutcomeV1::Failed { .. } => None,
                })
                .unwrap_or(target_count);
            Ok(json!({
                "group_id": group.group_id,
                "ledger": compact_case_ledger(&group.ledger, stratum_target_count)?,
            }))
        })
        .collect()
}

fn multi_path_data(null_interactions: bool) -> (Vec<String>, Vec<Vec<Option<String>>>) {
    let ids = [
        "xf", "mf", "yf", "xs", "ms", "ys", "xb", "mb", "yb", "xl", "l1", "l2", "l3", "l4", "l5",
        "yl", "z", "w",
    ];
    let mut values = ids
        .iter()
        .map(|id| ((*id).to_owned(), Vec::<f64>::new()))
        .collect::<BTreeMap<_, _>>();
    for row in 0..48 {
        let z = (row % 7) as f64 / 2.0 - 1.5;
        let w = ((row * 5 + 3) % 11) as f64 / 3.0 - 1.7;
        let xf = ((row * 7 + 1) % 19) as f64 / 4.0 - 2.0;
        let xs = ((row * 11 + 2) % 23) as f64 / 5.0 - 2.2;
        let xb = ((row * 13 + 4) % 29) as f64 / 6.0 - 2.3;
        let xl = ((row * 17 + 5) % 31) as f64 / 7.0 - 2.0;
        let noise = ((row * 19 + 7) % 17) as f64 / 100.0 - 0.08;
        let interaction_scale = if null_interactions { 0.0 } else { 1.0 };
        let mf = 0.62 * xf + 0.22 * z + interaction_scale * 0.38 * xf * z + noise;
        let yf = 0.84 * mf + 0.17 * xf - 0.4 * noise;
        let ms = 0.71 * xs + 0.19 * z - 0.2 * noise;
        let ys = 0.77 * ms + 0.18 * z + interaction_scale * -0.31 * ms * z + 0.3 * noise;
        let mb = 0.58 * xb + 0.16 * z + interaction_scale * 0.29 * xb * z + 0.4 * noise;
        let yb = 0.69 * mb + 0.15 * w + interaction_scale * 0.27 * mb * w + 0.12 * xb - noise;
        let l1 = 0.66 * xl + 0.14 * z + interaction_scale * 0.23 * xl * z + noise;
        let l2 = 0.73 * l1 + 0.7 * noise;
        let l3 = 0.68 * l2 - 0.5 * noise;
        let l4 = 0.64 * l3 + 0.3 * noise;
        let l5 = 0.61 * l4 - 0.2 * noise;
        let yl = 0.79 * l5 + 0.11 * xl + 0.4 * noise;
        for (id, value) in [
            ("xf", xf),
            ("mf", mf),
            ("yf", yf),
            ("xs", xs),
            ("ms", ms),
            ("ys", ys),
            ("xb", xb),
            ("mb", mb),
            ("yb", yb),
            ("xl", xl),
            ("l1", l1),
            ("l2", l2),
            ("l3", l3),
            ("l4", l4),
            ("l5", l5),
            ("yl", yl),
            ("z", z),
            ("w", w),
        ] {
            values.get_mut(id).expect("fixture variable").push(value);
        }
    }
    let headers = ids.iter().map(|id| indicator_id(id)).collect::<Vec<_>>();
    let columns = ids
        .iter()
        .map(|id| numeric(values.remove(*id).expect("fixture column")))
        .collect();
    (headers, columns)
}

fn multi_path_fixture(
    scale: Scale,
    profile: ConditionalProcessProfileV2,
    interval: ConditionalProcessIntervalV2,
    alternative: InferenceAlternativeV1,
    null_interactions: bool,
) -> Result<(qpls_data::Dataset, AnalysisRecipeV4, SemModelV4), DynError> {
    let (headers, columns) = multi_path_data(null_interactions);
    let dataset =
        metamorphic_dataset_from_columns("conditional-multipath-v1.csv", &headers, &columns)?;
    let ids = [
        "xf", "mf", "yf", "xs", "ms", "ys", "xb", "mb", "yb", "xl", "l1", "l2", "l3", "l4", "l5",
        "yl", "z", "w",
    ];
    let constructs = ids
        .iter()
        .map(|id| (*id, vec![indicator_id(id)]))
        .collect::<Vec<_>>();
    let construct_refs = constructs
        .iter()
        .map(|(id, indicators)| {
            (
                *id,
                indicators.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let construct_slices = construct_refs
        .iter()
        .map(|(id, indicators)| (*id, indicators.as_slice()))
        .collect::<Vec<_>>();
    let paths = [
        ("xf", "mf"),
        ("z", "mf"),
        ("mf", "yf"),
        ("xf", "yf"),
        ("xs", "ms"),
        ("ms", "ys"),
        ("z", "ys"),
        ("xs", "ys"),
        ("xb", "mb"),
        ("z", "mb"),
        ("mb", "yb"),
        ("w", "yb"),
        ("xb", "yb"),
        ("xl", "l1"),
        ("z", "l1"),
        ("l1", "l2"),
        ("l2", "l3"),
        ("l3", "l4"),
        ("l4", "l5"),
        ("l5", "yl"),
        ("xl", "yl"),
    ];
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0001,
        "conditional multi-path qualification",
        &construct_slices,
        &paths,
        FIXTURE_SEED,
    )?;
    for (id, operands, focal, outcome) in [
        ("int:xf:z:mf", vec!["xf", "z"], "xf", "mf"),
        ("int:ms:z:ys", vec!["ms", "z"], "ms", "ys"),
        ("int:xb:z:mb", vec!["xb", "z"], "xb", "mb"),
        ("int:mb:w:yb", vec!["mb", "w"], "mb", "yb"),
        ("int:xl:z:l1", vec!["xl", "z"], "xl", "l1"),
    ] {
        add_interaction_by_construct(&mut model, id, &operands, focal, outcome)?;
    }
    let paths = vec![
        selected_path(&model, "first_stage_2_edges", &["xf", "mf", "yf"])?,
        selected_path(&model, "second_stage_2_edges", &["xs", "ms", "ys"])?,
        selected_path(&model, "both_stage_2_edges", &["xb", "mb", "yb"])?,
        selected_path(
            &model,
            "long_path_6_edges",
            &["xl", "l1", "l2", "l3", "l4", "l5", "yl"],
        )?,
    ];
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile,
        paths,
        declared_interaction_ids: vec![
            "int:xf:z:mf".into(),
            "int:ms:z:ys".into(),
            "int:xb:z:mb".into(),
            "int:mb:w:yb".into(),
            "int:xl:z:l1".into(),
        ],
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z"), construct_id("w")],
        probes: vec![
            standardized_probe(&construct_id("z")),
            standardized_probe(&construct_id("w")),
        ],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0), ("construct:w", -1.0)]),
            joint_tuple("center", &[("construct:z", 0.0), ("construct:w", 0.0)]),
            joint_tuple("high", &[("construct:z", 1.0), ("construct:w", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: estimands(true),
        inference: inference(scale, interval, alternative),
    });
    Ok((dataset, recipe, model))
}

fn run_multi_path_case(
    scale: Scale,
    alternative: InferenceAlternativeV1,
) -> Result<Value, DynError> {
    let (dataset, recipe, model) = multi_path_fixture(
        scale,
        ConditionalProcessProfileV2::MultiTwoWayPercentile,
        ConditionalProcessIntervalV2::Percentile,
        alternative,
        false,
    )?;
    finish_and_run(
        &[
            "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
            "conditional.multi_two_way_percentile.v2::shared_ledger_percentile_type7",
            "conditional.multi_two_way_percentile.v2::both_stage_multiple_long_path",
            "conditional.multi_two_way_percentile.v2::all_predeclared_alternatives",
        ],
        &format!("multi_path_percentile:{}", alternative_id(alternative)),
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn compact_two_stage_fixture(
    scale: Scale,
    profile: ConditionalProcessProfileV2,
    interval: ConditionalProcessIntervalV2,
    alternative: InferenceAlternativeV1,
    null_interactions: bool,
) -> Result<(qpls_data::Dataset, AnalysisRecipeV4, SemModelV4), DynError> {
    let mut x = Vec::new();
    let mut z = Vec::new();
    let mut m = Vec::new();
    let mut y = Vec::new();
    for row in 0..36 {
        let local_x = ((row * 7 + 1) % 23) as f64 / 5.0 - 2.2;
        let local_z = ((row * 11 + 2) % 19) as f64 / 4.0 - 2.0;
        let noise = ((row * 13 + 3) % 17) as f64 / 120.0 - 0.06;
        let gamma = if null_interactions { 0.0 } else { 0.34 };
        let first = 0.68 * local_x + 0.21 * local_z + gamma * local_x * local_z + noise;
        let second =
            0.76 * first + 0.18 * local_z - gamma * 0.7 * first * local_z + 0.12 * local_x - noise;
        x.push(local_x);
        z.push(local_z);
        m.push(first);
        y.push(second);
    }
    let headers = ["x_i", "z_i", "m_i", "y_i"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let columns = vec![numeric(x), numeric(z), numeric(m), numeric(y)];
    let dataset =
        metamorphic_dataset_from_columns("conditional-two-stage-v1.csv", &headers, &columns)?;
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0002,
        "conditional two-stage qualification",
        &[
            ("x", &["x_i"]),
            ("z", &["z_i"]),
            ("m", &["m_i"]),
            ("y", &["y_i"]),
        ],
        &[("x", "m"), ("z", "m"), ("m", "y"), ("z", "y"), ("x", "y")],
        FIXTURE_SEED,
    )?;
    add_interaction_by_construct(&mut model, "int:x:z:m", &["x", "z"], "x", "m")?;
    add_interaction_by_construct(&mut model, "int:m:z:y", &["m", "z"], "m", "y")?;
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile,
        paths: vec![selected_path(&model, "both_stage", &["x", "m", "y"])?],
        declared_interaction_ids: vec!["int:x:z:m".into(), "int:m:z:y".into()],
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z")],
        probes: vec![standardized_probe(&construct_id("z"))],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0)]),
            joint_tuple("center", &[("construct:z", 0.0)]),
            joint_tuple("high", &[("construct:z", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: estimands(false),
        inference: inference(scale, interval, alternative),
    });
    Ok((dataset, recipe, model))
}

fn run_bca_case(
    scale: Scale,
    alternative: InferenceAlternativeV1,
    null_interactions: bool,
) -> Result<Value, DynError> {
    let (dataset, recipe, model) = compact_two_stage_fixture(
        scale,
        ConditionalProcessProfileV2::MultiTwoWayBca,
        ConditionalProcessIntervalV2::Bca,
        alternative,
        null_interactions,
    )?;
    finish_and_run(
        &[
            "conditional.multi_two_way_bca.v2::explicit_path_target_math",
            "conditional.multi_two_way_bca.v2::complete_delete_one_bca",
            "conditional.multi_two_way_bca.v2::all_predeclared_alternatives",
        ],
        &format!(
            "bca:{}:{}",
            if null_interactions {
                "null"
            } else {
                "non_null"
            },
            alternative_id(alternative)
        ),
        if null_interactions {
            "null"
        } else {
            "non_null"
        },
        dataset,
        recipe,
        model,
    )
}

fn run_studentized_case(
    scale: Scale,
    alternative: InferenceAlternativeV1,
) -> Result<Value, DynError> {
    let (dataset, recipe, model) = compact_two_stage_fixture(
        scale,
        ConditionalProcessProfileV2::MultiTwoWayStudentized,
        ConditionalProcessIntervalV2::Studentized,
        alternative,
        false,
    )?;
    finish_and_run(
        &[
            "conditional.studentized.v2::nested_studentized",
            "conditional.studentized.v2::no_percentile_fallback",
            "conditional.studentized.v2::all_predeclared_alternatives",
        ],
        &format!("studentized:non_null:{}", alternative_id(alternative)),
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn run_three_way_case(
    scale: Scale,
    alternative: InferenceAlternativeV1,
) -> Result<Value, DynError> {
    let mut x = Vec::new();
    let mut z = Vec::new();
    let mut w = Vec::new();
    let mut m = Vec::new();
    let mut y = Vec::new();
    for row in 0..54 {
        let local_x = ((row * 5 + 1) % 17) as f64 / 4.0 - 1.8;
        let local_z = ((row * 7 + 2) % 19) as f64 / 4.5 - 1.8;
        let local_w = ((row * 11 + 3) % 23) as f64 / 5.0 - 2.0;
        let noise = ((row * 13 + 5) % 29) as f64 / 180.0 - 0.08;
        let mediator = 0.51 * local_x + 0.29 * local_z + 0.23 * local_w + 0.31 * local_x * local_z
            - 0.22 * local_x * local_w
            + 0.17 * local_z * local_w
            + 0.28 * local_x * local_z * local_w
            + noise;
        x.push(local_x);
        z.push(local_z);
        w.push(local_w);
        m.push(mediator);
        y.push(0.79 * mediator + 0.13 * local_x - 0.4 * noise);
    }
    let headers = ["x_i", "z_i", "w_i", "m_i", "y_i"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns(
        "conditional-three-way-v1.csv",
        &headers,
        &[numeric(x), numeric(z), numeric(w), numeric(m), numeric(y)],
    )?;
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0003,
        "conditional three-way qualification",
        &[
            ("x", &["x_i"]),
            ("z", &["z_i"]),
            ("w", &["w_i"]),
            ("m", &["m_i"]),
            ("y", &["y_i"]),
        ],
        &[("x", "m"), ("z", "m"), ("w", "m"), ("m", "y"), ("x", "y")],
        FIXTURE_SEED,
    )?;
    for (id, operands, focal) in [
        ("int:x:z:m", vec!["x", "z"], "x"),
        ("int:x:w:m", vec!["x", "w"], "x"),
        ("int:z:w:m", vec!["z", "w"], "z"),
        ("int:x:z:w:m", vec!["x", "z", "w"], "x"),
    ] {
        add_interaction_by_construct(&mut model, id, &operands, focal, "m")?;
    }
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile: ConditionalProcessProfileV2::BoundedThreeWayPercentile,
        paths: vec![
            selected_path(&model, "three_way_x_path", &["x", "m", "y"])?,
            selected_path(&model, "lower_order_z_path", &["z", "m", "y"])?,
        ],
        declared_interaction_ids: vec![
            "int:x:z:m".into(),
            "int:x:w:m".into(),
            "int:z:w:m".into(),
            "int:x:z:w:m".into(),
        ],
        three_way_interaction_id: Some("int:x:z:w:m".into()),
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z"), construct_id("w")],
        probes: vec![
            standardized_probe(&construct_id("z")),
            standardized_probe(&construct_id("w")),
        ],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0), ("construct:w", -1.0)]),
            joint_tuple("center", &[("construct:z", 0.0), ("construct:w", 0.0)]),
            joint_tuple("mixed", &[("construct:z", 1.0), ("construct:w", -0.5)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "mixed-minus-low".into(),
            left_tuple_id: "mixed".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: estimands(false),
        inference: inference(scale, ConditionalProcessIntervalV2::Percentile, alternative),
    });
    finish_and_run(
        &[
            "conditional.bounded_three_way_percentile.v2::complete_lower_order_closure",
            "conditional.bounded_three_way_percentile.v2::derivatives_and_cross_derivatives",
            "conditional.bounded_three_way_percentile.v2::shared_ledger_percentile_type7",
            "conditional.bounded_three_way_percentile.v2::all_predeclared_alternatives",
        ],
        &format!("bounded_three_way:{}", alternative_id(alternative)),
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn run_hoc_case(scale: Scale) -> Result<Value, DynError> {
    let mut headers = Vec::new();
    let mut columns = Vec::new();
    let mut construct_storage = Vec::<(String, Vec<String>)>::new();
    let mut values = BTreeMap::<String, Vec<f64>>::new();
    let mut all_ids = vec!["z".to_owned()];
    for index in 1..=4 {
        all_ids.extend([
            format!("h{index}a"),
            format!("h{index}b"),
            format!("m{index}"),
            format!("y{index}"),
        ]);
    }
    for id in &all_ids {
        values.insert(id.clone(), Vec::new());
    }
    for row in 0..64 {
        let z = ((row * 7 + 1) % 19) as f64 / 4.0 - 2.0;
        values.get_mut("z").unwrap().push(z);
        for index in 1..=4 {
            let a = ((row * (index * 3 + 5) + index) % (23 + index)) as f64 / 5.0 - 2.0;
            let b = ((row * (index * 5 + 7) + 2 * index) % (29 + index)) as f64 / 6.0 - 2.1;
            let noise = ((row * (index * 7 + 3) + 1) % 17) as f64 / 140.0 - 0.06;
            let hoc = (a + b) / 2.0;
            let gamma = if index <= 2 { 0.22 } else { 0.0 };
            let mediator = 0.63 * hoc + 0.18 * z + gamma * hoc * z + noise;
            let outcome = 0.78 * mediator - 0.3 * noise;
            values.get_mut(&format!("h{index}a")).unwrap().push(a);
            values.get_mut(&format!("h{index}b")).unwrap().push(b);
            values.get_mut(&format!("m{index}")).unwrap().push(mediator);
            values.get_mut(&format!("y{index}")).unwrap().push(outcome);
        }
    }
    for id in &all_ids {
        let indicator = indicator_id(id);
        headers.push(indicator.clone());
        columns.push(numeric(values.remove(id).unwrap()));
        construct_storage.push((id.clone(), vec![indicator]));
    }
    let construct_refs = construct_storage
        .iter()
        .map(|(id, indicators)| {
            (
                id.as_str(),
                indicators.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let construct_slices = construct_refs
        .iter()
        .map(|(id, indicators)| (*id, indicators.as_slice()))
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns("conditional-hoc-v1.csv", &headers, &columns)?;
    let mut path_storage = Vec::<(String, String)>::new();
    for index in 1..=4 {
        path_storage.push(("z".into(), format!("m{index}")));
        path_storage.push((format!("m{index}"), format!("y{index}")));
    }
    let path_refs = path_storage
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_str()))
        .collect::<Vec<_>>();
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0004,
        "conditional four-HOC qualification",
        &construct_slices,
        &path_refs,
        FIXTURE_SEED,
    )?;
    for index in 1..=4 {
        let components = [
            construct_id(&format!("h{index}a")),
            construct_id(&format!("h{index}b")),
        ];
        let component_refs = components.iter().map(String::as_str).collect::<Vec<_>>();
        add_disjoint_hoc(
            &mut model,
            &format!("hoc{index}"),
            &component_refs,
            &construct_id(&format!("m{index}")),
        );
    }
    for index in 1..=2 {
        let hoc = format!("derived:hoc{index}");
        let moderator = construct_id("z");
        let outcome = construct_id(&format!("m{index}"));
        add_interaction(
            &mut model,
            &format!("int:hoc{index}:z:m{index}"),
            &[hoc.as_str(), moderator.as_str()],
            &hoc,
            &outcome,
        )?;
    }
    let mut selected = Vec::new();
    for index in 1..=4 {
        selected.push(ConditionalProcessPathV2 {
            path_id: format!("hoc{index}_path"),
            ordered_relation_ids: vec![
                relation_id(
                    &model,
                    &format!("derived:hoc{index}"),
                    &construct_id(&format!("m{index}")),
                )?,
                relation_id(
                    &model,
                    &construct_id(&format!("m{index}")),
                    &construct_id(&format!("y{index}")),
                )?,
            ],
        });
    }
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile: ConditionalProcessProfileV2::MultipleHocPercentile,
        paths: selected,
        declared_interaction_ids: vec!["int:hoc1:z:m1".into(), "int:hoc2:z:m2".into()],
        three_way_interaction_id: None,
        hoc_ids: (1..=4).map(|index| format!("term:hoc{index}")).collect(),
        moderator_ids: vec![construct_id("z")],
        probes: vec![standardized_probe(&construct_id("z"))],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0)]),
            joint_tuple("center", &[("construct:z", 0.0)]),
            joint_tuple("high", &[("construct:z", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: estimands(false),
        inference: inference(
            scale,
            ConditionalProcessIntervalV2::Percentile,
            InferenceAlternativeV1::TwoSided,
        ),
    });
    finish_and_run(
        &[
            "conditional.multiple_hoc_percentile.v2::hoc_dependency_before_products",
            "conditional.multiple_hoc_percentile.v2::disjoint_nonnested_single_approach",
            "conditional.multiple_hoc_percentile.v2::shared_ledger_percentile_type7_two_sided",
        ],
        "multiple_hoc:four_disjoint",
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn grouped_or_weighted_base(
    source_name: &str,
    include_group: bool,
    weight_kind: Option<bool>,
) -> Result<(qpls_data::Dataset, AnalysisRecipeV4, SemModelV4), DynError> {
    let mut x = Vec::new();
    let mut z = Vec::new();
    let mut m = Vec::new();
    let mut y = Vec::new();
    let mut group = Vec::new();
    let mut weight = Vec::new();
    for row in 0..72 {
        let local_group = if row < 36 { "A" } else { "B" };
        let local_x = ((row * 7 + 1) % 29) as f64 / 6.0 - 2.2;
        let local_z = ((row * 11 + 2) % 23) as f64 / 5.0 - 2.0;
        let noise = ((row * 13 + 3) % 19) as f64 / 150.0 - 0.06;
        let gamma = if local_group == "A" { 0.27 } else { 0.42 };
        let mediator = 0.64 * local_x + 0.2 * local_z + gamma * local_x * local_z + noise;
        let outcome = 0.81 * mediator + 0.14 * local_x - 0.3 * noise;
        x.push(local_x);
        z.push(local_z);
        m.push(mediator);
        y.push(outcome);
        group.push(local_group.to_owned());
        weight.push(if weight_kind == Some(true) {
            (row % 4 + 1) as f64
        } else {
            0.5 + (row % 9) as f64 / 5.0
        });
    }
    let mut headers = vec!["x_i".into(), "z_i".into(), "m_i".into(), "y_i".into()];
    let mut columns = vec![numeric(x), numeric(z), numeric(m), numeric(y)];
    if include_group {
        headers.push("group".into());
        columns.push(text(group));
    }
    if weight_kind.is_some() {
        headers.push("analysis_weight".into());
        columns.push(numeric(weight));
    }
    let dataset = metamorphic_dataset_from_columns(source_name, &headers, &columns)?;
    let (recipe, model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0005
            + u128::from(include_group)
            + u128::from(weight_kind == Some(true)) * 2,
        source_name,
        &[
            ("x", &["x_i"]),
            ("z", &["z_i"]),
            ("m", &["m_i"]),
            ("y", &["y_i"]),
        ],
        &[("x", "m"), ("z", "m"), ("m", "y"), ("x", "y")],
        FIXTURE_SEED,
    )?;
    Ok((dataset, recipe, model))
}

fn run_grouped_case(scale: Scale) -> Result<Value, DynError> {
    let (dataset, mut recipe, mut model) =
        grouped_or_weighted_base("conditional-grouped-v1.csv", true, None)?;
    add_groups(
        &mut model,
        "group",
        &[
            ("group-a".into(), "A".into(), "Group A".into()),
            ("group-b".into(), "B".into(), "Group B".into()),
        ],
    );
    add_interaction_by_construct(&mut model, "int:x:z:m", &["x", "z"], "x", "m")?;
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile: ConditionalProcessProfileV2::GroupedPercentile,
        paths: vec![selected_path(&model, "grouped_path", &["x", "m", "y"])?],
        declared_interaction_ids: vec!["int:x:z:m".into()],
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z")],
        probes: vec![standardized_probe(&construct_id("z"))],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0)]),
            joint_tuple("high", &[("construct:z", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: Some("group".into()),
        groups: vec![
            SelectedGroupV1 {
                group_id: "group-a".into(),
                label: "Group A".into(),
                value: TypedGroupValueV1::Text { value: "A".into() },
            },
            SelectedGroupV1 {
                group_id: "group-b".into(),
                label: "Group B".into(),
                value: TypedGroupValueV1::Text { value: "B".into() },
            },
        ],
        group_contrasts: vec![ConditionalGroupContrastV2 {
            contrast_id: "a-minus-b".into(),
            left_group_id: "group-a".into(),
            right_group_id: "group-b".into(),
        }],
        weight: None,
        estimands: estimands(false),
        inference: inference(
            scale,
            ConditionalProcessIntervalV2::Percentile,
            InferenceAlternativeV1::TwoSided,
        ),
    });
    finish_and_run(
        &[
            "conditional.grouped_percentile.v2::group_stratified_shared_ledger",
            "conditional.grouped_percentile.v2::percentile_type7_two_sided",
        ],
        "grouped:stratified",
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn run_weighted_case(scale: Scale, frequency: bool) -> Result<Value, DynError> {
    let (dataset, mut recipe, mut model) = grouped_or_weighted_base(
        if frequency {
            "conditional-frequency-v1.csv"
        } else {
            "conditional-case-weight-v1.csv"
        },
        false,
        Some(frequency),
    )?;
    add_weight_binding(&mut model, "analysis_weight", frequency)?;
    add_interaction_by_construct(&mut model, "int:x:z:m", &["x", "z"], "x", "m")?;
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    let profile = if frequency {
        ConditionalProcessProfileV2::FrequencyWeightedPercentile
    } else {
        ConditionalProcessProfileV2::CaseWeightedPercentile
    };
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile,
        paths: vec![selected_path(&model, "weighted_path", &["x", "m", "y"])?],
        declared_interaction_ids: vec!["int:x:z:m".into()],
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z")],
        probes: vec![standardized_probe(&construct_id("z"))],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0)]),
            joint_tuple("high", &[("construct:z", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: Some(if frequency {
            AnalysisWeightBindingV1::Frequency {
                column: "analysis_weight".into(),
            }
        } else {
            AnalysisWeightBindingV1::Case {
                column: "analysis_weight".into(),
            }
        }),
        estimands: estimands(false),
        inference: inference(
            scale,
            ConditionalProcessIntervalV2::Percentile,
            InferenceAlternativeV1::TwoSided,
        ),
    });
    let cell_ids: &[&str] = if frequency {
        &[
            "conditional.frequency_weighted_percentile.v2::count_space_point_equivalence",
            "conditional.frequency_weighted_percentile.v2::multinomial_count_bootstrap_equivalence",
            "conditional.frequency_weighted_percentile.v2::percentile_type7_two_sided",
        ]
    } else {
        &[
            "conditional.case_weighted_percentile.v2::positive_normalized_case_weights",
            "conditional.case_weighted_percentile.v2::row_weight_resampling",
            "conditional.case_weighted_percentile.v2::percentile_type7_two_sided",
        ]
    };
    let compact = finish_and_run(
        cell_ids,
        if frequency {
            "frequency_weighted:count_space"
        } else {
            "case_weighted:positive"
        },
        "non_null",
        dataset,
        recipe,
        model,
    )?;
    if !frequency {
        return Ok(compact);
    }
    let expanded = run_physically_expanded_frequency_reference(scale)?;
    Ok(json!({
        "case_id": "frequency_weighted:count_space_and_physical_expansion",
        "cell_ids": ["conditional.frequency_weighted_percentile.v2::count_space_point_equivalence"],
        "fixture_role": "non_null",
        "compact_count_space": compact,
        "physical_expansion_reference": expanded,
        "comparison_contract": "same single-indicator scientific model; compact frequency point targets must equal physical row expansion, and every compact bootstrap refit is runner-gated by the exact expansion receipt",
    }))
}

fn run_physically_expanded_frequency_reference(scale: Scale) -> Result<Value, DynError> {
    let mut x = Vec::new();
    let mut z = Vec::new();
    let mut m = Vec::new();
    let mut y = Vec::new();
    for row in 0..72 {
        let local_x = ((row * 7 + 1) % 29) as f64 / 6.0 - 2.2;
        let local_z = ((row * 11 + 2) % 23) as f64 / 5.0 - 2.0;
        let noise = ((row * 13 + 3) % 19) as f64 / 150.0 - 0.06;
        let mediator = 0.64 * local_x + 0.2 * local_z + 0.27 * local_x * local_z + noise;
        let outcome = 0.81 * mediator + 0.14 * local_x - 0.3 * noise;
        for _ in 0..(row % 4 + 1) {
            x.push(local_x);
            z.push(local_z);
            m.push(mediator);
            y.push(outcome);
        }
    }
    let headers = ["x_i", "z_i", "m_i", "y_i"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let dataset = metamorphic_dataset_from_columns(
        "conditional-frequency-expanded-v1.csv",
        &headers,
        &[numeric(x), numeric(z), numeric(m), numeric(y)],
    )?;
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0xc011_d111_0000_0000_0000_0000_0000_0008,
        "conditional physical frequency expansion qualification",
        &[
            ("x", &["x_i"]),
            ("z", &["z_i"]),
            ("m", &["m_i"]),
            ("y", &["y_i"]),
        ],
        &[("x", "m"), ("z", "m"), ("m", "y"), ("x", "y")],
        FIXTURE_SEED,
    )?;
    add_interaction_by_construct(&mut model, "int:x:z:m", &["x", "z"], "x", "m")?;
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: 2,
        profile: ConditionalProcessProfileV2::MultiTwoWayPercentile,
        paths: vec![selected_path(&model, "weighted_path", &["x", "m", "y"])?],
        declared_interaction_ids: vec!["int:x:z:m".into()],
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids: vec![construct_id("z")],
        probes: vec![standardized_probe(&construct_id("z"))],
        explicit_joint_tuples: vec![
            joint_tuple("low", &[("construct:z", -1.0)]),
            joint_tuple("high", &[("construct:z", 1.0)]),
        ],
        probe_contrasts: vec![ConditionalProbeContrastV2 {
            contrast_id: "high-minus-low".into(),
            left_tuple_id: "high".into(),
            right_tuple_id: "low".into(),
        }],
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: estimands(false),
        inference: inference(
            scale,
            ConditionalProcessIntervalV2::Percentile,
            InferenceAlternativeV1::TwoSided,
        ),
    });
    finish_and_run(
        &["conditional.frequency_weighted_percentile.v2::count_space_point_equivalence"],
        "frequency_weighted:physical_expansion_reference",
        "non_null",
        dataset,
        recipe,
        model,
    )
}

fn config_error(
    mut config: GeneralSemConditionalProcessConfigV2,
    mutate: impl FnOnce(&mut GeneralSemConditionalProcessConfigV2),
) -> Value {
    mutate(&mut config);
    match config.ensure_valid() {
        Ok(()) => json!({"status": "unexpectedly_admitted"}),
        Err(error) => {
            json!({"status": "blocked", "code": error.code, "path": error.path, "message": error.message})
        }
    }
}

fn unsupported_intersections() -> Result<Value, DynError> {
    let (_, recipe, _) = compact_two_stage_fixture(
        Scale::Development,
        ConditionalProcessProfileV2::MultiTwoWayPercentile,
        ConditionalProcessIntervalV2::Percentile,
        InferenceAlternativeV1::TwoSided,
        false,
    )?;
    let base = recipe.general_sem_conditional_process.unwrap();
    Ok(json!({
        "group_plus_weight": config_error(base.clone(), |config| {
            config.profile = ConditionalProcessProfileV2::GroupedPercentile;
            config.grouping_column = Some("group".into());
            config.groups = vec![
                SelectedGroupV1 { group_id: "a".into(), label: "A".into(), value: TypedGroupValueV1::Text { value: "A".into() } },
                SelectedGroupV1 { group_id: "b".into(), label: "B".into(), value: TypedGroupValueV1::Text { value: "B".into() } },
            ];
            config.weight = Some(AnalysisWeightBindingV1::Case { column: "weight".into() });
        }),
        "hoc_plus_group": config_error(base.clone(), |config| {
            config.profile = ConditionalProcessProfileV2::MultipleHocPercentile;
            config.hoc_ids = vec!["term:hoc".into()];
            config.grouping_column = Some("group".into());
        }),
        "three_way_plus_hoc": config_error(base.clone(), |config| {
            config.profile = ConditionalProcessProfileV2::BoundedThreeWayPercentile;
            config.three_way_interaction_id = Some("int:three".into());
            config.hoc_ids = vec!["term:hoc".into()];
        }),
        "studentized_outside_profile": config_error(base, |config| {
            config.inference.interval = ConditionalProcessIntervalV2::Studentized;
            config.inference.inner_resamples = 200;
        }),
    }))
}

fn qualification_boundary_guards() -> Result<Value, DynError> {
    let (_, grouped_recipe, _) = compact_two_stage_fixture(
        Scale::Development,
        ConditionalProcessProfileV2::MultiTwoWayPercentile,
        ConditionalProcessIntervalV2::Percentile,
        InferenceAlternativeV1::TwoSided,
        false,
    )?;
    let grouped_base = grouped_recipe
        .general_sem_conditional_process
        .ok_or_else(|| invalid("group-boundary fixture lost its config"))?;
    let group = |index: usize| SelectedGroupV1 {
        group_id: format!("group-{index}"),
        label: format!("Group {index}"),
        value: TypedGroupValueV1::Text {
            value: format!("G{index}"),
        },
    };
    let one_group = config_error(grouped_base.clone(), |config| {
        config.profile = ConditionalProcessProfileV2::GroupedPercentile;
        config.grouping_column = Some("group".into());
        config.groups = vec![group(1)];
    });
    let twenty_one_groups = config_error(grouped_base, |config| {
        config.profile = ConditionalProcessProfileV2::GroupedPercentile;
        config.grouping_column = Some("group".into());
        config.groups = (1..=21).map(group).collect();
    });

    let (_, studentized_recipe, _) = compact_two_stage_fixture(
        Scale::Development,
        ConditionalProcessProfileV2::MultiTwoWayStudentized,
        ConditionalProcessIntervalV2::Studentized,
        InferenceAlternativeV1::TwoSided,
        false,
    )?;
    let studentized = studentized_recipe
        .general_sem_conditional_process
        .ok_or_else(|| invalid("studentized-boundary fixture lost its config"))?;
    let studentized_budget = config_error(studentized, |config| {
        config.inference.outer_resamples = 5_000;
        config.inference.inner_resamples = 201;
    });

    let plan = MultiModJackknifePlanV1 {
        schema_version: 1,
        scientific_refit_identity_sha256: "1".repeat(64),
    };
    let mut callback = |draw: &MultiModDeleteOneJackknifeDrawV1| {
        if draw.omitted_row == 2 {
            Err(MultiModRefitFailureV1 {
                code: "fixture.singular_design".into(),
                message: "qualification fixture intentionally failed one delete-one refit".into(),
            })
        } else {
            Ok(vec![f64::from(draw.omitted_row)])
        }
    };
    let cache = run_multimod_delete_one_jackknife_shard_v1(
        &plan,
        4,
        None,
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut callback,
        || false,
    )?;
    let incomplete_jackknife =
        match finalize_multimod_delete_one_jackknife_v1(&plan, 4, None, vec![cache]) {
            Ok(_) => json!({"status": "unexpectedly_admitted"}),
            Err(error) => json!({"status": "blocked", "error": error.to_string()}),
        };

    let weight_headers = vec!["w".to_owned()];
    let case_ratio_dataset = metamorphic_dataset_from_columns(
        "conditional-invalid-case-ratio-v1.csv",
        &weight_headers,
        &[numeric([1.0, 1_000_001.0])],
    )?;
    let case_weight_ratio =
        match qpls_estimation::prepare_multimod_case_weight_dataset_v1(&case_ratio_dataset, "w") {
            Ok(_) => json!({"status": "unexpectedly_admitted"}),
            Err(error) => json!({"status": "blocked", "error": error.to_string()}),
        };
    let noninteger_dataset = metamorphic_dataset_from_columns(
        "conditional-invalid-frequency-noninteger-v1.csv",
        &weight_headers,
        &[numeric([1.5, 2.0])],
    )?;
    let noninteger_frequency = match qpls_estimation::prepare_multimod_frequency_weight_dataset_v1(
        &noninteger_dataset,
        "w",
    ) {
        Ok(_) => json!({"status": "unexpectedly_admitted"}),
        Err(error) => json!({"status": "blocked", "error": error.to_string()}),
    };
    let excessive_total_dataset = metamorphic_dataset_from_columns(
        "conditional-invalid-frequency-total-v1.csv",
        &weight_headers,
        &[numeric([9_007_199_254_740_991.0, 1.0])],
    )?;
    let excessive_frequency_total =
        match qpls_estimation::prepare_multimod_frequency_weight_dataset_v1(
            &excessive_total_dataset,
            "w",
        ) {
            Ok(_) => json!({"status": "unexpectedly_admitted"}),
            Err(error) => json!({"status": "blocked", "error": error.to_string()}),
        };

    Ok(json!({
        "cell_ids": [
            "conditional.multi_two_way_bca.v2::incomplete_jackknife_fail_closed",
            "conditional.studentized.v2::outer_inner_budget_limits",
            "conditional.grouped_percentile.v2::two_to_twenty_group_bounds",
            "conditional.case_weighted_percentile.v2::kish_ess_and_ratio_guards",
            "conditional.frequency_weighted_percentile.v2::exact_integer_total_guard"
        ],
        "incomplete_bca_jackknife": incomplete_jackknife,
        "studentized_outer_inner_budget": studentized_budget,
        "group_count_below_minimum": one_group,
        "group_count_above_maximum": twenty_one_groups,
        "case_weight_ratio": case_weight_ratio,
        "noninteger_frequency": noninteger_frequency,
        "excessive_frequency_total": excessive_frequency_total,
    }))
}

fn required_cells() -> Vec<&'static str> {
    vec![
        "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
        "conditional.multi_two_way_percentile.v2::shared_ledger_percentile_type7",
        "conditional.multi_two_way_percentile.v2::both_stage_multiple_long_path",
        "conditional.multi_two_way_percentile.v2::all_predeclared_alternatives",
        "conditional.multi_two_way_bca.v2::explicit_path_target_math",
        "conditional.multi_two_way_bca.v2::complete_delete_one_bca",
        "conditional.multi_two_way_bca.v2::all_predeclared_alternatives",
        "conditional.multi_two_way_bca.v2::incomplete_jackknife_fail_closed",
        "conditional.studentized.v2::nested_studentized",
        "conditional.studentized.v2::no_percentile_fallback",
        "conditional.studentized.v2::all_predeclared_alternatives",
        "conditional.studentized.v2::outer_inner_budget_limits",
        "conditional.bounded_three_way_percentile.v2::complete_lower_order_closure",
        "conditional.bounded_three_way_percentile.v2::derivatives_and_cross_derivatives",
        "conditional.bounded_three_way_percentile.v2::shared_ledger_percentile_type7",
        "conditional.bounded_three_way_percentile.v2::all_predeclared_alternatives",
        "conditional.multiple_hoc_percentile.v2::hoc_dependency_before_products",
        "conditional.multiple_hoc_percentile.v2::disjoint_nonnested_single_approach",
        "conditional.multiple_hoc_percentile.v2::shared_ledger_percentile_type7_two_sided",
        "conditional.grouped_percentile.v2::group_stratified_shared_ledger",
        "conditional.grouped_percentile.v2::percentile_type7_two_sided",
        "conditional.grouped_percentile.v2::two_to_twenty_group_bounds",
        "conditional.case_weighted_percentile.v2::positive_normalized_case_weights",
        "conditional.case_weighted_percentile.v2::row_weight_resampling",
        "conditional.case_weighted_percentile.v2::percentile_type7_two_sided",
        "conditional.case_weighted_percentile.v2::kish_ess_and_ratio_guards",
        "conditional.frequency_weighted_percentile.v2::count_space_point_equivalence",
        "conditional.frequency_weighted_percentile.v2::multinomial_count_bootstrap_equivalence",
        "conditional.frequency_weighted_percentile.v2::percentile_type7_two_sided",
        "conditional.frequency_weighted_percentile.v2::exact_integer_total_guard",
    ]
}
