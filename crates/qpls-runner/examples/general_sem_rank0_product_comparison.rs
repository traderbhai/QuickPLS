//! Validation-only General SEM Rank 0 current-product producer.
//!
//! This example is deliberately a thin adapter. It constructs the public
//! production contracts, calls the production compiler and runner, and emits
//! the untouched typed result. Scientific normalization and comparison remain
//! in `validation/general_sem_rank0_qualification_runner.py`.

use chrono::{TimeZone, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4,
    AnalysisSettings, Construct, GeneralSemBootstrapIntervalV1, GeneralSemConfigV1,
    GeneralSemInferenceTailV1, GeneralSemInferenceV1, InteractionHierarchyPolicyV2,
    InteractionMethodV4, LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig,
    ModelSpec, SemDataBindingV4, SemDerivedTermV4, SemParameterTargetV4, SemParameterV4,
    SemRelationV4, SemVariableV4, StructuralPath, StructuralRelationRoleV4,
    compile_general_sem_pls_recipe_v1, confirm_legacy_recipe_estimand_v4,
    migrate_analysis_recipe_to_v4_pending,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_resampling::{
    GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1,
    GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1, bootstrap_indices,
};
use qpls_runner::{RecipeV4GeneralSemPlsExecutionResultV1, run_compiled_general_sem_pls_recipe_v1};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const REQUEST_KIND: &str = "general_sem_rank0_current_product_request_v1";
const BUNDLE_KIND: &str = "general_sem_rank0_current_product_bundle_v2";
const PRODUCER_CONTRACT: &str = "qpls_runner_production_api_adapter_v1";
const EXECUTION_NONCE_ENV: &str = "QPLS_RANK0_PRODUCT_EXECUTION_NONCE";
const MAX_SAFE_SEED: u64 = (1_u64 << 53) - 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    kind: String,
    integrity_scope: String,
    plan_sha256: String,
    shard_id: String,
    scenario_id: String,
    scenario_sha256: String,
    cell_id: String,
    method_version: String,
    analytical_method_version: String,
    operation: String,
    scenario_seed: u64,
    bootstrap_seed: Option<u64>,
    required_worker_axes: Vec<String>,
    product_input: ProductInput,
    required_production_result: RequiredProductionResult,
    cargo_execution: CargoExecution,
    request_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductInput {
    schema_version: u32,
    scenario_id: String,
    columns: BTreeMap<String, Vec<Option<f64>>>,
    blocks: Vec<Block>,
    paths: Vec<DirectedPath>,
    interactions: Vec<Interaction>,
    effect_target: Option<EffectTarget>,
    bootstrap: Option<BootstrapRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    construct_id: String,
    indicator_ids: Vec<String>,
    mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectedPath {
    source_id: String,
    target_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Interaction {
    interaction_id: String,
    focal_id: String,
    moderator_id: String,
    outcome_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectTarget {
    source_id: String,
    target_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapRequest {
    confidence_level: f64,
    requested: u32,
    index_plan_authority: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiredProductionResult {
    raw_typed_result: bool,
    exact_bootstrap_indices: bool,
    point_bindings: Vec<String>,
    bootstrap_bindings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CargoExecution {
    serialized: bool,
    owner: String,
    automatic_parallel_invocation_forbidden: bool,
    package: String,
    example: String,
    bundle_kind: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerRun {
    worker_axis: String,
    workers: usize,
    seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bootstrap_authority_receipt: Option<BootstrapAuthorityReceipt>,
    relation_identities: Vec<ProductionRelationIdentity>,
    production_result: RecipeV4GeneralSemPlsExecutionResultV1,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapAuthorityReceipt {
    compiled_primary_capability_cell: qpls_core::CapabilityCellReferenceV2,
    analytical_primary_capability_cell: qpls_core::CapabilityCellReferenceV2,
    supplemental_inference_capability_cell: qpls_core::CapabilityCellReferenceV2,
    supplemental_inference_method_version: String,
    supplemental_resampling_operation_version: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProductionRelationIdentity {
    relation_id: String,
    source_id: String,
    target_id: String,
}

struct ExecutionOutput {
    relation_identities: Vec<ProductionRelationIdentity>,
    bootstrap_authority_receipt: Option<BootstrapAuthorityReceipt>,
    result: RecipeV4GeneralSemPlsExecutionResultV1,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapIndexPlan {
    authority: String,
    operation: String,
    complete_case_count: usize,
    requested: u32,
    seed: u64,
    replicate_indices: Vec<Vec<usize>>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema_version: u32,
    kind: String,
    producer_contract_version: String,
    request_sha256: String,
    plan_sha256: String,
    shard_id: String,
    scenario_id: String,
    scenario_sha256: String,
    cell_id: String,
    method_version: String,
    analytical_method_version: String,
    scenario_seed: u64,
    bootstrap_seed: Option<u64>,
    cargo_invocation: String,
    cargo_exit_code: i32,
    producer_executable_sha256: String,
    maximum_available_workers: usize,
    execution_nonce: String,
    bootstrap_index_plan: Option<BootstrapIndexPlan>,
    worker_runs: Vec<WorkerRun>,
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}

fn valid_execution_nonce(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_capability_authorities(
    operation: &str,
) -> Result<
    (
        qpls_core::CapabilityCellReferenceV2,
        Option<qpls_core::CapabilityCellReferenceV2>,
    ),
    Box<dyn Error>,
> {
    match operation {
        "mediation_point" => Ok((
            qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            None,
        )),
        "mediation_bootstrap" => Ok((
            qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            Some(qpls_core::pls_general_bootstrap_capability_cell_v1()),
        )),
        "moderation_point" => Ok((
            qpls_core::pls_general_multiple_moderation_point_capability_cell_v1(),
            None,
        )),
        "moderation_bootstrap" => Ok((
            qpls_core::pls_general_multiple_moderation_point_capability_cell_v1(),
            Some(qpls_core::pls_general_multiple_moderation_bootstrap_capability_cell_v1()),
        )),
        _ => Err(invalid("current-product operation is not recognized")),
    }
}

fn expected_bootstrap_method_version(
    operation: &str,
) -> Result<Option<&'static str>, Box<dyn Error>> {
    match operation {
        "mediation_point" | "moderation_point" => Ok(None),
        "mediation_bootstrap" => Ok(Some(
            qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
        )),
        "moderation_bootstrap" => Ok(Some(
            qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
        )),
        _ => Err(invalid("current-product operation is not recognized")),
    }
}

fn expected_analytical_method_version(operation: &str) -> Result<&'static str, Box<dyn Error>> {
    match operation {
        "mediation_point" => Ok("pls_mediation_v1"),
        "mediation_bootstrap" => Ok(qpls_core::GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1),
        "moderation_point" => {
            Ok(qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1)
        }
        "moderation_bootstrap" => {
            Ok(qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1)
        }
        _ => Err(invalid("current-product operation is not recognized")),
    }
}

fn validate_request(request: &Request) -> Result<(), Box<dyn Error>> {
    if request.schema_version != 1
        || request.kind != REQUEST_KIND
        || request.integrity_scope
            != "prequalification_integrity_only_not_source_or_identity_receipt"
        || request.product_input.schema_version != 1
        || request.product_input.scenario_id != request.scenario_id
        || request.request_sha256.len() != 64
        || request.plan_sha256.len() != 64
        || request.scenario_sha256.len() != 64
        || request.required_worker_axes.is_empty()
        || request.scenario_seed > MAX_SAFE_SEED
    {
        return Err(invalid("current-product request identity is invalid"));
    }
    if !request.required_production_result.raw_typed_result
        || !request.required_production_result.exact_bootstrap_indices
        || request.required_production_result.point_bindings.is_empty()
        || !request.cargo_execution.serialized
        || !request
            .cargo_execution
            .automatic_parallel_invocation_forbidden
        || request.cargo_execution.owner != "root_integration_lane"
        || request.cargo_execution.package != "qpls-runner"
        || request.cargo_execution.example != "general_sem_rank0_product_comparison"
        || request.cargo_execution.bundle_kind != BUNDLE_KIND
    {
        return Err(invalid("current-product execution contract is invalid"));
    }
    let is_bootstrap = request.operation.ends_with("_bootstrap");
    let is_moderation = request.operation.starts_with("moderation_");
    let (primary_authority, supplemental_authority) =
        expected_capability_authorities(&request.operation)?;
    let requested_authority = supplemental_authority
        .as_ref()
        .unwrap_or(&primary_authority);
    if is_bootstrap
        != (request.bootstrap_seed.is_some() && request.product_input.bootstrap.is_some())
        || is_bootstrap
            != request
                .required_production_result
                .bootstrap_bindings
                .is_some()
        || is_moderation != !request.product_input.interactions.is_empty()
        || is_moderation != request.product_input.effect_target.is_none()
    {
        return Err(invalid("operation and product input disagree"));
    }
    let expected_worker_axes = if is_bootstrap {
        &["1", "2", "4", "max"][..]
    } else {
        &["not_applicable"][..]
    };
    if request
        .required_worker_axes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != expected_worker_axes
    {
        return Err(invalid("semantic worker-axis inventory differs"));
    }
    if request.cell_id != requested_authority.cell_id
        || request.method_version != requested_authority.capability_version
        || request.analytical_method_version
            != expected_analytical_method_version(&request.operation)?
    {
        return Err(invalid(
            "request authority differs from the exact operation capability",
        ));
    }
    if let (Some(seed), Some(bootstrap)) =
        (request.bootstrap_seed, &request.product_input.bootstrap)
    {
        if seed > MAX_SAFE_SEED
            || !(2..=10_000).contains(&bootstrap.requested)
            || !(0.0..1.0).contains(&bootstrap.confidence_level)
            || bootstrap.index_plan_authority
                != "qpls_resampling_bootstrap_indices_exact_operation_v1"
        {
            return Err(invalid(
                "bootstrap request is outside the production contract",
            ));
        }
    }
    let row_counts = request
        .product_input
        .columns
        .values()
        .map(Vec::len)
        .collect::<BTreeSet<_>>();
    if row_counts.len() != 1 || row_counts.contains(&0) {
        return Err(invalid(
            "indicator columns must share one nonzero row count",
        ));
    }
    let declared = request
        .product_input
        .blocks
        .iter()
        .flat_map(|block| block.indicator_ids.iter())
        .collect::<BTreeSet<_>>();
    let actual = request
        .product_input
        .columns
        .keys()
        .collect::<BTreeSet<_>>();
    if declared != actual {
        return Err(invalid("declared indicators and data columns differ"));
    }
    if let Some(target) = &request.product_input.effect_target {
        let constructs = request
            .product_input
            .blocks
            .iter()
            .map(|block| block.construct_id.as_str())
            .collect::<BTreeSet<_>>();
        if target.source_id == target.target_id
            || !constructs.contains(target.source_id.as_str())
            || !constructs.contains(target.target_id.as_str())
        {
            return Err(invalid("mediation effect target is invalid"));
        }
    }
    Ok(())
}

fn csv_bytes(input: &ProductInput) -> Result<Vec<u8>, Box<dyn Error>> {
    let headers = input.columns.keys().collect::<Vec<_>>();
    let rows = input.columns.values().next().map(Vec::len).unwrap_or(0);
    let mut csv = String::new();
    csv.push_str(
        &headers
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );
    csv.push('\n');
    for row in 0..rows {
        for (index, header) in headers.iter().enumerate() {
            if index != 0 {
                csv.push(',');
            }
            if let Some(value) = input.columns[*header][row] {
                if !value.is_finite() {
                    return Err(invalid("product input contains a non-finite value"));
                }
                csv.push_str(&value.to_string());
            }
        }
        csv.push('\n');
    }
    Ok(csv.into_bytes())
}

fn construct_id(identifier: &str) -> String {
    format!("construct:{identifier}")
}

fn structural_relation_id(
    model: &qpls_core::SemModelV4,
    source: &str,
    target: &str,
) -> Result<String, Box<dyn Error>> {
    let source = construct_id(source);
    let target = construct_id(target);
    model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source: actual_source,
                target: actual_target,
                ..
            } if actual_source == &source && actual_target == &target => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("missing hierarchy path {source}->{target}")))
}

fn add_interaction(
    model: &mut qpls_core::SemModelV4,
    interaction: &Interaction,
) -> Result<(), Box<dyn Error>> {
    let interaction_id = format!("interaction:{}", interaction.interaction_id);
    let focal_relation =
        structural_relation_id(model, &interaction.focal_id, &interaction.outcome_id)?;
    let output = format!("derived:{interaction_id}");
    let effect_relation = format!("relation:{interaction_id}:effect");
    let effect_parameter = format!("parameter:{interaction_id}:effect");
    let outcome = construct_id(&interaction.outcome_id);
    model.variables.push(SemVariableV4::Derived {
        id: output.clone(),
        label: interaction.interaction_id.clone(),
    });
    model.relations.push(SemRelationV4::Structural {
        id: effect_relation,
        source: output.clone(),
        target: outcome.clone(),
        parameter: effect_parameter.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: effect_parameter,
        label: format!(
            "{} -> {}",
            interaction.interaction_id, interaction.outcome_id
        ),
        target: SemParameterTargetV4::Regression {
            source: output.clone(),
            target: outcome,
        },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
        id: interaction_id,
        output,
        operands: vec![
            construct_id(&interaction.focal_id),
            construct_id(&interaction.moderator_id),
        ],
        focal_relation,
        method: InteractionMethodV4::TwoStage,
        hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
        product_indicator: None,
    });
    model.ensure_valid()?;
    Ok(())
}

fn execute_worker(request: &Request, workers: usize) -> Result<ExecutionOutput, Box<dyn Error>> {
    let csv = csv_bytes(&request.product_input)?;
    let dataset = import_delimited_bytes(
        &csv,
        "general-sem-rank0-product-comparison.csv",
        b',',
        &ImportOptions::default(),
    )?;
    let source_model = ModelSpec {
        id: Uuid::from_u128(
            0x5150_4c53_0000_0000_0000_0000_0000_0000 | u128::from(request.scenario_seed),
        ),
        name: request.scenario_id.clone(),
        constructs: request
            .product_input
            .blocks
            .iter()
            .map(|block| {
                let mode = match block.mode.as_str() {
                    "A" => Ok(MeasurementMode::Reflective),
                    "B" => Ok(MeasurementMode::Formative),
                    _ => Err(invalid(format!("unsupported block mode {}", block.mode))),
                }?;
                Ok(Construct {
                    id: block.construct_id.clone(),
                    name: block.construct_id.clone(),
                    short_name: block.construct_id.clone(),
                    mode,
                    indicators: block.indicator_ids.clone(),
                })
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
        paths: request
            .product_input
            .paths
            .iter()
            .map(|path| StructuralPath {
                source: path.source_id.clone(),
                target: path.target_id.clone(),
            })
            .collect(),
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let mut settings = AnalysisSettings {
        method: AnalysisMethod::PlsPm,
        workers,
        ..AnalysisSettings::default()
    };
    if let (Some(seed), Some(bootstrap)) =
        (request.bootstrap_seed, &request.product_input.bootstrap)
    {
        settings.bootstrap_samples = bootstrap.requested;
        settings.seed = seed;
        settings.confidence_level = bootstrap.confidence_level;
        settings.bootstrap_test_tail = qpls_core::PlsBootstrapTestTail::TwoSided;
        settings.studentized_inner_samples = 0;
    }
    let source_recipe = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: Uuid::from_u128(
            0x5150_4c53_1000_0000_0000_0000_0000_0000 | u128::from(request.scenario_seed),
        ),
        created_at: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .ok_or_else(|| invalid("invalid fixed timestamp"))?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: source_model.clone(),
        settings,
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata: BTreeMap::new(),
    };
    let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe)?;
    let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
        &pending,
        &source_model,
        &[],
        LegacyBasicModelInterpretationV4::PlsComposite,
    )?;
    let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
        return Err(invalid("PLS product model did not retain raw data binding"));
    };
    *dataset_id = dataset.id.to_string();
    for interaction in &request.product_input.interactions {
        add_interaction(&mut model, interaction)?;
    }
    recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: model.scientific_sha256()?,
        model: model.clone(),
    };
    let mut config = GeneralSemConfigV1::default();
    if let (Some(seed), Some(bootstrap)) =
        (request.bootstrap_seed, &request.product_input.bootstrap)
    {
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: bootstrap.requested,
            seed,
            confidence_level: bootstrap.confidence_level,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
    }
    recipe.general_sem_config = Some(config);
    recipe.ensure_valid()?;
    let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&model))?;
    let (expected_primary, expected_supplemental) =
        expected_capability_authorities(&request.operation)?;
    if artifact.capability_cell() != &expected_primary {
        return Err(invalid(
            "compiled General SEM artifact does not preserve point capability authority",
        ));
    }
    let relation_identities = artifact
        .plan()
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| ProductionRelationIdentity {
            relation_id: relation.relation_id().into(),
            source_id: relation.source().into(),
            target_id: relation.target().into(),
        })
        .collect();
    let result = run_compiled_general_sem_pls_recipe_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )?;
    if result.capability_cell() != &expected_primary {
        return Err(invalid(
            "analytical General SEM result does not preserve point capability authority",
        ));
    }
    let canonical = result.canonical_general_sem_results_v1()?;
    if let Some(interaction_point) = result.interaction_point_estimation()
        && interaction_point.method_version()
            != qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
    {
        return Err(invalid(
            "analytical moderation point method differs from the production result",
        ));
    }
    let bootstrap_authority_receipt = match expected_supplemental {
        None => {
            if canonical.inference_receipt.is_some() {
                return Err(invalid(
                    "point General SEM result unexpectedly carries supplemental inference authority",
                ));
            }
            None
        }
        Some(expected_supplemental) => {
            let receipt = canonical.inference_receipt.as_ref().ok_or_else(|| {
                invalid("bootstrap General SEM result omits supplemental inference authority")
            })?;
            let raw_bootstrap_method = match request.operation.as_str() {
                "mediation_bootstrap" => result
                    .bootstrap_inference()
                    .map(|bootstrap| bootstrap.method_version.as_str()),
                "moderation_bootstrap" => result
                    .moderation_bootstrap_inference()
                    .map(|bootstrap| bootstrap.method_version.as_str()),
                _ => None,
            }
            .ok_or_else(|| invalid("bootstrap result omits its exact inference method"))?;
            let expected_bootstrap_method = expected_bootstrap_method_version(&request.operation)?
                .ok_or_else(|| invalid("bootstrap operation omits its exact method authority"))?;
            if receipt.capability_cell != expected_supplemental
                || receipt.capability_cell.cell_id != request.cell_id
                || receipt.capability_cell.capability_version != request.method_version
                || receipt.method_version != expected_bootstrap_method
                || receipt.method_version != request.analytical_method_version
                || receipt.method_version != raw_bootstrap_method
            {
                return Err(invalid(
                    "supplemental bootstrap capability or exact algorithm method differs",
                ));
            }
            Some(BootstrapAuthorityReceipt {
                compiled_primary_capability_cell: artifact.capability_cell().clone(),
                analytical_primary_capability_cell: result.capability_cell().clone(),
                supplemental_inference_capability_cell: receipt.capability_cell.clone(),
                supplemental_inference_method_version: receipt.method_version.clone(),
                supplemental_resampling_operation_version: receipt
                    .resampling_operation_version
                    .clone(),
            })
        }
    };
    Ok(ExecutionOutput {
        relation_identities,
        bootstrap_authority_receipt,
        result,
    })
}

fn complete_case_count(input: &ProductInput) -> usize {
    let indicators = input
        .blocks
        .iter()
        .flat_map(|block| block.indicator_ids.iter())
        .collect::<Vec<_>>();
    let rows = input.columns.values().next().map(Vec::len).unwrap_or(0);
    (0..rows)
        .filter(|row| {
            indicators
                .iter()
                .all(|indicator| input.columns[*indicator][*row].is_some())
        })
        .count()
}

fn bootstrap_index_plan(request: &Request) -> Result<Option<BootstrapIndexPlan>, Box<dyn Error>> {
    let (Some(seed), Some(bootstrap)) = (request.bootstrap_seed, &request.product_input.bootstrap)
    else {
        return Ok(None);
    };
    let operation = match request.operation.as_str() {
        "mediation_bootstrap" => GENERAL_SEM_PLS_BOOTSTRAP_OPERATION_V1,
        "moderation_bootstrap" => GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_OPERATION_V1,
        _ => return Err(invalid("bootstrap operation is not recognized")),
    };
    let cases = complete_case_count(&request.product_input);
    if cases < 2 {
        return Err(invalid("bootstrap complete-case frame is too small"));
    }
    Ok(Some(BootstrapIndexPlan {
        authority: "qpls_resampling::bootstrap_indices".into(),
        operation: operation.into(),
        complete_case_count: cases,
        requested: bootstrap.requested,
        seed,
        replicate_indices: (0..bootstrap.requested)
            .map(|index| bootstrap_indices(cases, seed, operation, index))
            .collect(),
    }))
}

fn read_request(path: &Path) -> Result<Request, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    validate_request(&request)?;
    Ok(request)
}

fn write_exclusive(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_vec_pretty(value)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(&payload)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(())
}

fn run(input_path: PathBuf, output_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let execution_nonce = std::env::var(EXECUTION_NONCE_ENV)
        .map_err(|_| invalid("fresh-run execution nonce is absent"))?;
    if !valid_execution_nonce(&execution_nonce) {
        return Err(invalid("fresh-run execution nonce is invalid"));
    }
    let request = read_request(&input_path)?;
    let maximum_available_workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let mut worker_runs = Vec::with_capacity(request.required_worker_axes.len());
    for worker_axis in &request.required_worker_axes {
        let workers = match worker_axis.as_str() {
            "not_applicable" | "1" => 1,
            "2" => 2,
            "4" => 4,
            "max" => maximum_available_workers,
            _ => return Err(invalid("semantic worker axis is unsupported")),
        };
        let execution = execute_worker(&request, workers)?;
        worker_runs.push(WorkerRun {
            worker_axis: worker_axis.clone(),
            workers,
            seed: request.bootstrap_seed,
            bootstrap_authority_receipt: execution.bootstrap_authority_receipt,
            relation_identities: execution.relation_identities,
            production_result: execution.result,
        });
    }
    let invocation = format!(
        "cargo run --locked -p qpls-runner --example general_sem_rank0_product_comparison -- {} {}",
        input_path.display(),
        output_path.display()
    );
    let index_plan = bootstrap_index_plan(&request)?;
    let executable = std::env::current_exe()?;
    let producer_executable_sha256 = qpls_core::sha256_hex(&fs::read(&executable)?);
    let bundle = Bundle {
        schema_version: 2,
        kind: BUNDLE_KIND.into(),
        producer_contract_version: PRODUCER_CONTRACT.into(),
        request_sha256: request.request_sha256,
        plan_sha256: request.plan_sha256,
        shard_id: request.shard_id,
        scenario_id: request.scenario_id,
        scenario_sha256: request.scenario_sha256,
        cell_id: request.cell_id,
        method_version: request.method_version,
        analytical_method_version: request.analytical_method_version,
        scenario_seed: request.scenario_seed,
        bootstrap_seed: request.bootstrap_seed,
        cargo_invocation: invocation,
        cargo_exit_code: 0,
        producer_executable_sha256,
        maximum_available_workers,
        execution_nonce,
        bootstrap_index_plan: index_plan,
        worker_runs,
    };
    write_exclusive(&output_path, &bundle)
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let input = arguments.next().map(PathBuf::from);
    let output = arguments.next().map(PathBuf::from);
    if input.is_none() || output.is_none() || arguments.next().is_some() {
        eprintln!("usage: general_sem_rank0_product_comparison <request.json> <new-output.json>");
        std::process::exit(2);
    }
    if let Err(error) = run(input.unwrap(), output.unwrap()) {
        let detail = Value::String(error.to_string());
        eprintln!(
            "{}",
            serde_json::to_string(&detail).unwrap_or_else(|_| "\"producer failure\"".into())
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        expected_analytical_method_version, expected_bootstrap_method_version,
        expected_capability_authorities, valid_execution_nonce,
    };

    #[test]
    fn fresh_run_nonce_is_exact_lowercase_hex() {
        assert!(valid_execution_nonce("0123456789abcdef0123456789abcdef"));
        assert!(!valid_execution_nonce("0123456789ABCDEF0123456789ABCDEF"));
        assert!(!valid_execution_nonce("0123456789abcdef"));
    }

    #[test]
    fn mediation_bootstrap_maps_point_primary_and_bootstrap_supplemental_authority() {
        let (primary, supplemental) =
            expected_capability_authorities("mediation_bootstrap").unwrap();
        let supplemental = supplemental.unwrap();
        assert_eq!(primary.cell_id, "qpls3.pls.mediation");
        assert_eq!(primary.capability_version, "pls_mediation_v1");
        assert_eq!(
            supplemental.cell_id,
            "qpls3.pls.general_sem_multiple_mediation_bootstrap"
        );
        assert_eq!(
            supplemental.capability_version,
            "general_sem_pls_full_model_case_bootstrap_v1"
        );
        assert_eq!(
            expected_bootstrap_method_version("mediation_bootstrap").unwrap(),
            Some("general_sem_pls_full_model_case_bootstrap_v1")
        );
        assert_eq!(
            expected_analytical_method_version("mediation_bootstrap").unwrap(),
            "general_sem_pls_full_model_case_bootstrap_v1"
        );
    }

    #[test]
    fn moderation_bootstrap_maps_point_primary_and_bootstrap_supplemental_authority() {
        let (primary, supplemental) =
            expected_capability_authorities("moderation_bootstrap").unwrap();
        let supplemental = supplemental.unwrap();
        assert_eq!(
            primary.cell_id,
            "qpls3.pls.general_sem_multiple_two_way_moderation_point"
        );
        assert_eq!(
            primary.capability_version,
            "general_sem_pls_multiple_two_way_moderation_point_v1"
        );
        assert_eq!(
            supplemental.cell_id,
            "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
        );
        assert_eq!(
            supplemental.capability_version,
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
        );
        let method = expected_bootstrap_method_version("moderation_bootstrap")
            .unwrap()
            .unwrap();
        assert_eq!(
            method,
            "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1"
        );
        assert_ne!(method, supplemental.capability_version);
        assert_eq!(
            expected_analytical_method_version("moderation_bootstrap").unwrap(),
            method
        );
    }

    #[test]
    fn point_operations_bind_their_exact_analytical_methods() {
        assert_eq!(
            expected_analytical_method_version("mediation_point").unwrap(),
            "pls_mediation_v1"
        );
        assert_eq!(
            expected_analytical_method_version("moderation_point").unwrap(),
            "qpls.general-sem-pls.multiple-two-way.point.v1"
        );
    }
}
