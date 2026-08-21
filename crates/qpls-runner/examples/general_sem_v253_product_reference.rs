//! Compact QuickPLS 2.53 production-path reference producer.
//!
//! The fixture arithmetic only creates deterministic observed columns. Every
//! estimate is produced through the real General SEM compiler and runner. The
//! compiler receives the exact internal qualification admission because these
//! source cells intentionally remain fail-closed until this evidence passes.

use chrono::{TimeZone, Utc};
use qpls_core::{
    ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe, AnalysisRecipeModelBindingV4,
    AnalysisSettings, CapabilityCellReferenceV2, Construct, GeneralSemBootstrapIntervalV1,
    GeneralSemConfigV1, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
    InteractionHierarchyPolicyV2, InteractionMethodV4, LegacyBasicModelInterpretationV4,
    MeasurementMode, MethodConfig, ModelSpec, ObservedScaleV4, PlsBootstrapTestTail,
    SemDataBindingV4, SemDerivedTermV4, SemParameterTargetV4, SemParameterV4, SemRelationV4,
    SemVariableV4, StructuralPath, StructuralRelationRoleV4,
    compile_general_sem_pls_recipe_with_internal_capability_admission_v1,
    confirm_legacy_recipe_estimand_v4, migrate_analysis_recipe_to_v4_pending,
};
use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
use qpls_resampling::{
    GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_OPERATION_V1, bootstrap_indices,
};
use qpls_runner::{GeneralSemRequestedEffectEstimateV1, run_compiled_general_sem_pls_recipe_v1};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 1;
const SUITE_ID: &str = "quickpls_v253_general_sem_product_reference_v1";
const RESAMPLES: u32 = 79;
const SEED: u64 = 20_260_821;
const CONFIDENCE_LEVEL: f64 = 0.95;

#[derive(Clone, Copy)]
enum ThreeWayFixtureKind {
    ContinuousContinuous,
    ContinuousBinary,
}

impl ThreeWayFixtureKind {
    fn scenario_id(self) -> &'static str {
        match self {
            Self::ContinuousContinuous => "continuous_continuous",
            Self::ContinuousBinary => "continuous_binary",
        }
    }

    fn binary_second_moderator(self) -> bool {
        matches!(self, Self::ContinuousBinary)
    }
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}

fn main() -> Result<(), Box<dyn Error>> {
    let (output, workers) = arguments()?;
    let mut point = Map::new();
    let mut bootstrap = Map::new();
    for fixture in [
        ThreeWayFixtureKind::ContinuousContinuous,
        ThreeWayFixtureKind::ContinuousBinary,
    ] {
        let (point_row, bootstrap_row) = run_three_way_fixture(fixture, workers)?;
        point.insert(fixture.scenario_id().into(), point_row);
        bootstrap.insert(fixture.scenario_id().into(), bootstrap_row);
    }
    let mediation = run_single_mediation_fixture(workers)?;
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "suite_id": SUITE_ID,
        "passed": true,
        "identities": {
            "single_mediation_bootstrap": identity(
                qpls_core::pls_general_single_mediation_bootstrap_capability_cell_v1(),
                GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1,
                Some(GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_OPERATION_V1),
            ),
            "three_way_point": identity(
                qpls_core::pls_general_three_way_moderation_point_capability_cell_v1(),
                qpls_core::GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
                None,
            ),
            "three_way_bootstrap": identity(
                qpls_core::pls_general_three_way_moderation_bootstrap_capability_cell_v1(),
                qpls_core::GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
                Some(qpls_core::GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1),
            ),
        },
        "three_way_point": point,
        "three_way_bootstrap": bootstrap,
        "single_mediation_bootstrap": mediation,
    });
    let encoded = serde_json::to_string_pretty(&report)? + "\n";
    if let Some(path) = output {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &encoded)?;
    }
    print!("{encoded}");
    Ok(())
}

fn arguments() -> Result<(Option<PathBuf>, usize), Box<dyn Error>> {
    let mut output = None;
    let mut workers = 1usize;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| invalid("--output requires a path"))?,
                ));
            }
            "--workers" => {
                workers = args
                    .next()
                    .ok_or_else(|| invalid("--workers requires an integer"))?
                    .parse()?;
                if !(1..=64).contains(&workers) {
                    return Err(invalid("--workers must be between 1 and 64"));
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: general_sem_v253_product_reference [--output PATH] [--workers 1..64]"
                );
                std::process::exit(0);
            }
            _ => return Err(invalid(format!("unknown argument {argument}"))),
        }
    }
    Ok((output, workers))
}

fn identity(
    cell: CapabilityCellReferenceV2,
    method_version: &str,
    operation_version: Option<&str>,
) -> Value {
    let identity = format!(
        "capability_registry_v2:{}:{}:{}",
        cell.capability_id, cell.cell_id, cell.capability_version,
    );
    json!({
        "capability_id": cell.capability_id,
        "cell_id": cell.cell_id,
        "capability_version": cell.capability_version,
        "identity": identity,
        "method_version": method_version,
        "operation_version": operation_version,
    })
}

fn run_three_way_fixture(
    fixture: ThreeWayFixtureKind,
    workers: usize,
) -> Result<(Value, Value), Box<dyn Error>> {
    let dataset = three_way_dataset(fixture)?;
    let (recipe, model) = three_way_recipe(&dataset, fixture, workers)?;
    let artifact = compile_general_sem_pls_recipe_with_internal_capability_admission_v1(
        &recipe,
        Some(&model),
        qpls_core::pls_general_three_way_moderation_bootstrap_capability_cell_v1(),
    )?;
    let result = run_compiled_general_sem_pls_recipe_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )?;
    let point = result
        .three_way_point_estimation()
        .ok_or_else(|| invalid("production runner omitted the three-way point payload"))?;
    let bootstrap = result
        .three_way_bootstrap_inference()
        .ok_or_else(|| invalid("production runner omitted the three-way bootstrap payload"))?;

    let x = "construct:x";
    let w = "construct:w";
    let z = "construct:z";
    let structural = |source: &str| {
        point
            .structural_coefficients
            .iter()
            .find(|row| row.source_id() == source && row.target_id() == "construct:y")
            .map(|row| row.estimate())
            .ok_or_else(|| invalid(format!("missing joint-stage coefficient for {source}")))
    };
    let pair = |left: &str, right: &str| {
        point
            .lower_order_interaction_coefficients
            .iter()
            .find(|row| {
                (row.focal_predictor_id() == left && row.moderator_id() == right)
                    || (row.focal_predictor_id() == right && row.moderator_id() == left)
            })
            .ok_or_else(|| {
                invalid(format!(
                    "missing pairwise coefficient for {left} by {right}"
                ))
            })
    };
    let xw = pair(x, w)?;
    let xz = pair(x, z)?;
    let wz = pair(w, z)?;
    let coefficients = vec![
        structural(x)?,
        structural(w)?,
        structural(z)?,
        xw.standardized_product_estimate(),
        xz.standardized_product_estimate(),
        wz.standardized_product_estimate(),
        point.three_way_coefficient.standardized_product_estimate,
    ];
    let point_output = json!({
        "method_version": point.method_version.as_str(),
        "probe_kinds": [
            point.first_moderator_probes.first().map(|probe| probe.probe_kind),
            point.second_moderator_probes.first().map(|probe| probe.probe_kind),
        ],
        "coefficients": coefficients,
        "pairwise_gammas": [
            xw.raw_product_estimate(),
            xz.raw_product_estimate(),
            wz.raw_product_estimate(),
        ],
        "three_way_delta": point.three_way_coefficient.scientific_rescaled_delta,
        "first_moderator_probes": point.first_moderator_probes.iter()
            .map(|probe| [probe.reported_value, probe.standardized_value])
            .collect::<Vec<_>>(),
        "second_moderator_probes": point.second_moderator_probes.iter()
            .map(|probe| [probe.reported_value, probe.standardized_value])
            .collect::<Vec<_>>(),
        "conditional_interactions": point.conditional_interaction_effects.iter()
            .map(|row| row.estimate)
            .collect::<Vec<_>>(),
        "simple_slopes": point.simple_slopes.iter()
            .map(|row| row.estimate)
            .collect::<Vec<_>>(),
    });
    let failed_indices = bootstrap
        .failed_replicates
        .iter()
        .map(|failure| failure.replicate_index)
        .collect::<BTreeSet<_>>();
    let usable_indices = (0..bootstrap.resamples_requested)
        .filter(|index| !failed_indices.contains(index))
        .collect::<Vec<_>>();
    let bootstrap_output = json!({
        "method_version": bootstrap.method_version.as_str(),
        "operation_version": bootstrap.resampling_operation_version.as_str(),
        "stream_version": bootstrap.resampling_stream_version.as_str(),
        "resamples": bootstrap.resamples_requested,
        "seed": bootstrap.seed.as_str(),
        "workers": bootstrap.workers,
        "usable_indices": usable_indices,
        "usable_indices_sha256": bootstrap.usable_replicate_indices_sha256.as_str(),
        "failed_replicates": &bootstrap.failed_replicates,
        "replicate_positions": replicate_positions(
            dataset.schema.case_count,
            SEED,
            qpls_core::GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
        ),
        "target_intervals": bootstrap.targets.iter().map(|target| json!({
            "target_id": target.target_id.as_str(),
            "original": target.original,
            "bootstrap_mean": target.bootstrap_mean,
            "standard_error": target.standard_error,
            "lower": target.lower,
            "upper": target.upper,
            "usable_replicates": target.usable_replicates,
        })).collect::<Vec<_>>(),
    });
    Ok((point_output, bootstrap_output))
}

fn run_single_mediation_fixture(workers: usize) -> Result<Value, Box<dyn Error>> {
    let dataset = mediation_dataset()?;
    let (recipe, model) = mediation_recipe(&dataset, workers)?;
    let artifact = compile_general_sem_pls_recipe_with_internal_capability_admission_v1(
        &recipe,
        Some(&model),
        qpls_core::pls_general_single_mediation_bootstrap_capability_cell_v1(),
    )?;
    let result = run_compiled_general_sem_pls_recipe_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )?;
    let path = |source: &str, target: &str| {
        result
            .point_estimation()
            .estimation()
            .paths
            .iter()
            .find(|row| row.source == source && row.target == target)
            .map(|row| row.coefficient)
            .ok_or_else(|| invalid(format!("missing production path {source} to {target}")))
    };
    let specific = result
        .requested_effects()
        .iter()
        .find(|effect| {
            matches!(
                effect,
                GeneralSemRequestedEffectEstimateV1::SpecificIndirect {
                    source_id,
                    target_id,
                    ..
                } if source_id == "construct:x" && target_id == "construct:y"
            )
        })
        .ok_or_else(|| invalid("single mediation specific indirect effect is missing"))?;
    let total = result
        .requested_effects()
        .iter()
        .find(|effect| {
            matches!(
                effect,
                GeneralSemRequestedEffectEstimateV1::TotalEffect {
                    source_id,
                    target_id,
                    ..
                } if source_id == "construct:x" && target_id == "construct:y"
            )
        })
        .ok_or_else(|| invalid("single mediation total effect is missing"))?;
    let bootstrap = result
        .bootstrap_inference()
        .ok_or_else(|| invalid("production runner omitted single-mediation bootstrap"))?;
    let selected = bootstrap
        .effects
        .iter()
        .find(|effect| effect.effect_id == specific.canonical_effect_id())
        .ok_or_else(|| invalid("single mediation bootstrap target is missing"))?;
    let failed_indices = bootstrap
        .failed_replicates
        .iter()
        .map(|failure| failure.replicate_index)
        .collect::<BTreeSet<_>>();
    let usable_indices = (0..bootstrap.resamples_requested)
        .filter(|index| !failed_indices.contains(index))
        .collect::<Vec<_>>();
    Ok(json!({
        "point": {
            "path_a": path("construct:x", "construct:m")?,
            "path_b": path("construct:m", "construct:y")?,
            "specific_indirect": specific.coefficient(),
            "total_effect": total.coefficient(),
        },
        "method_version": bootstrap.method_version.as_str(),
        "operation_version": bootstrap.resampling_operation_version.as_str(),
        "stream_version": bootstrap.resampling_stream_version.as_str(),
        "resamples": bootstrap.resamples_requested,
        "seed": bootstrap.seed.as_str(),
        "workers": bootstrap.workers,
        "usable_indices": usable_indices,
        "usable_indices_sha256": bootstrap.usable_replicate_indices_sha256.as_str(),
        "failed_replicates": &bootstrap.failed_replicates,
        "replicate_positions": replicate_positions(
            dataset.schema.case_count,
            SEED,
            GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_OPERATION_V1,
        ),
        "interval": {
            "target_id": selected.effect_id.as_str(),
            "original": selected.original,
            "bootstrap_mean": selected.bootstrap_mean,
            "standard_error": selected.standard_error,
            "lower": selected.lower,
            "upper": selected.upper,
            "usable_replicates": selected.usable_replicates,
        },
    }))
}

fn replicate_positions(case_count: usize, seed: u64, operation: &str) -> Vec<Value> {
    (0..RESAMPLES)
        .map(|replicate_index| {
            json!({
                "replicate_index": replicate_index,
                "sample_indices": bootstrap_indices(case_count, seed, operation, replicate_index),
            })
        })
        .collect()
}

fn three_way_dataset(fixture: ThreeWayFixtureKind) -> Result<Dataset, Box<dyn Error>> {
    let index = (0..60)
        .map(|position| f64::from(position) - 29.5)
        .collect::<Vec<_>>();
    let x = index
        .iter()
        .map(|row| (0.173 * row).sin() + 0.011 * row + 0.0008 * row * row)
        .collect::<Vec<_>>();
    let w = index
        .iter()
        .map(|row| (0.217 * row).cos() - 0.006 * row + 0.19 * (0.071 * row).sin())
        .collect::<Vec<_>>();
    let z = if fixture.binary_second_moderator() {
        (0..60)
            .map(|position| ((position * 7 + position / 5) % 2) as f64)
            .collect::<Vec<_>>()
    } else {
        index
            .iter()
            .map(|row| (0.307 * row).sin() + 0.23 * (0.127 * row).cos() + 0.004 * row)
            .collect::<Vec<_>>()
    };
    let noise = index
        .iter()
        .map(|row| 0.045 * (0.811 * row).sin() + 0.027 * (0.439 * row).cos())
        .collect::<Vec<_>>();
    let xs = standardized(&x)?;
    let ws = standardized(&w)?;
    let zs = standardized(&z)?;
    let y = xs
        .iter()
        .zip(&ws)
        .zip(&zs)
        .zip(&noise)
        .map(|(((x, w), z), error)| {
            0.31 * x + 0.17 * w - 0.13 * z + 0.24 * x * w - 0.18 * x * z
                + 0.12 * w * z
                + 0.21 * x * w * z
                + error
        })
        .collect::<Vec<_>>();
    dataset_from_columns(fixture.scenario_id(), &["x", "w", "z", "y"], &[x, w, z, y])
}

fn mediation_dataset() -> Result<Dataset, Box<dyn Error>> {
    let index = (0..70)
        .map(|position| f64::from(position) - 34.5)
        .collect::<Vec<_>>();
    let x = index
        .iter()
        .map(|row| (0.149 * row).sin() + 0.012 * row + 0.0005 * row * row)
        .collect::<Vec<_>>();
    let xs = standardized(&x)?;
    let disturbance_m = standardized(
        &index
            .iter()
            .map(|row| (0.337 * row).cos() + 0.09 * (0.71 * row).sin())
            .collect::<Vec<_>>(),
    )?;
    let m = xs
        .iter()
        .zip(&disturbance_m)
        .map(|(x, error)| 0.63 * x + 0.31 * error)
        .collect::<Vec<_>>();
    let ms = standardized(&m)?;
    let disturbance_y = standardized(
        &index
            .iter()
            .map(|row| (0.419 * row).sin() - 0.11 * (0.83 * row).cos())
            .collect::<Vec<_>>(),
    )?;
    let y = xs
        .iter()
        .zip(&ms)
        .zip(&disturbance_y)
        .map(|((x, m), error)| 0.22 * x + 0.57 * m + 0.27 * error)
        .collect::<Vec<_>>();
    dataset_from_columns("single_mediation", &["x", "m", "y"], &[x, m, y])
}

fn standardized(values: &[f64]) -> Result<Vec<f64>, Box<dyn Error>> {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let standard_deviation = variance.sqrt();
    if !standard_deviation.is_finite() || standard_deviation <= f64::EPSILON {
        return Err(invalid("fixture column is constant or non-finite"));
    }
    Ok(values
        .iter()
        .map(|value| (value - mean) / standard_deviation)
        .collect())
}

fn dataset_from_columns(
    name: &str,
    headers: &[&str],
    columns: &[Vec<f64>],
) -> Result<Dataset, Box<dyn Error>> {
    let row_count = columns
        .first()
        .map(Vec::len)
        .ok_or_else(|| invalid("fixture has no columns"))?;
    if columns.len() != headers.len() || columns.iter().any(|column| column.len() != row_count) {
        return Err(invalid("fixture columns have inconsistent dimensions"));
    }
    let mut csv = headers.join(",") + "\n";
    for row in 0..row_count {
        csv.push_str(
            &columns
                .iter()
                .map(|column| column[row].to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok(import_delimited_bytes(
        csv.as_bytes(),
        &format!("v253-{name}.csv"),
        b',',
        &ImportOptions::default(),
    )?)
}

fn three_way_recipe(
    dataset: &Dataset,
    fixture: ThreeWayFixtureKind,
    workers: usize,
) -> Result<(qpls_core::AnalysisRecipeV4, qpls_core::SemModelV4), Box<dyn Error>> {
    let source_model = legacy_model(
        if fixture.binary_second_moderator() {
            0x2530_0000_0000_0000_0000_0000_0000_0002
        } else {
            0x2530_0000_0000_0000_0000_0000_0000_0001
        },
        fixture.scenario_id(),
        &["x", "w", "z", "y"],
        &[("x", "y"), ("w", "y"), ("z", "y")],
    );
    let (mut recipe, mut model) = migrated_recipe(dataset, source_model, workers)?;
    if fixture.binary_second_moderator() {
        let SemVariableV4::Observed {
            scale,
            categories,
            value_labels,
            ..
        } = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "observed:z")
            .ok_or_else(|| invalid("binary Z indicator is missing"))?
        else {
            return Err(invalid("binary Z is not an observed indicator"));
        };
        *scale = ObservedScaleV4::Binary;
        *categories = vec!["0".into(), "1".into()];
        value_labels.clear();
    }
    add_interaction(
        &mut model,
        "interaction:x_by_w",
        &["construct:x", "construct:w"],
        "construct:x",
    )?;
    add_interaction(
        &mut model,
        "interaction:x_by_z",
        &["construct:x", "construct:z"],
        "construct:x",
    )?;
    add_interaction(
        &mut model,
        "interaction:w_by_z",
        &["construct:w", "construct:z"],
        "construct:w",
    )?;
    add_interaction(
        &mut model,
        "interaction:x_by_w_by_z",
        &["construct:x", "construct:w", "construct:z"],
        "construct:x",
    )?;
    finalize_recipe(&mut recipe, &model)?;
    Ok((recipe, model))
}

fn mediation_recipe(
    dataset: &Dataset,
    workers: usize,
) -> Result<(qpls_core::AnalysisRecipeV4, qpls_core::SemModelV4), Box<dyn Error>> {
    let source_model = legacy_model(
        0x2530_0000_0000_0000_0000_0000_0000_0003,
        "single_mediation",
        &["x", "m", "y"],
        &[("x", "m"), ("x", "y"), ("m", "y")],
    );
    let (mut recipe, model) = migrated_recipe(dataset, source_model, workers)?;
    finalize_recipe(&mut recipe, &model)?;
    Ok((recipe, model))
}

fn legacy_model(id: u128, name: &str, constructs: &[&str], paths: &[(&str, &str)]) -> ModelSpec {
    ModelSpec {
        id: Uuid::from_u128(id),
        name: name.into(),
        constructs: constructs
            .iter()
            .map(|id| Construct {
                id: (*id).into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![(*id).into()],
            })
            .collect(),
        paths: paths
            .iter()
            .map(|(source, target)| StructuralPath {
                source: (*source).into(),
                target: (*target).into(),
            })
            .collect(),
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    }
}

fn migrated_recipe(
    dataset: &Dataset,
    source_model: ModelSpec,
    workers: usize,
) -> Result<(qpls_core::AnalysisRecipeV4, qpls_core::SemModelV4), Box<dyn Error>> {
    let source = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: Uuid::from_u128(source_model.id.as_u128() ^ 0x2530_0000_0000_0000),
        created_at: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .ok_or_else(|| invalid("fixed fixture timestamp is invalid"))?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: source_model.clone(),
        settings: AnalysisSettings {
            method: AnalysisMethod::PlsPm,
            bootstrap_samples: RESAMPLES,
            seed: SEED,
            confidence_level: CONFIDENCE_LEVEL,
            bootstrap_test_tail: PlsBootstrapTestTail::TwoSided,
            studentized_inner_samples: 0,
            workers,
            ..AnalysisSettings::default()
        },
        method_config: Some(MethodConfig::PlsAlgorithm),
        metadata: BTreeMap::new(),
    };
    let pending = migrate_analysis_recipe_to_v4_pending(&source)?;
    let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
        &pending,
        &source_model,
        &[],
        LegacyBasicModelInterpretationV4::PlsComposite,
    )?;
    let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
        return Err(invalid("migrated fixture did not retain raw-data binding"));
    };
    *dataset_id = dataset.id.to_string();
    recipe.settings.bootstrap_samples = RESAMPLES;
    recipe.settings.seed = SEED;
    recipe.settings.confidence_level = CONFIDENCE_LEVEL;
    recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.workers = workers;
    recipe.general_sem_config = Some(GeneralSemConfigV1 {
        inference: GeneralSemInferenceV1::CaseBootstrap {
            resamples: RESAMPLES,
            seed: SEED,
            confidence_level: CONFIDENCE_LEVEL,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        },
        ..GeneralSemConfigV1::default()
    });
    Ok((recipe, model))
}

fn finalize_recipe(
    recipe: &mut qpls_core::AnalysisRecipeV4,
    model: &qpls_core::SemModelV4,
) -> Result<(), Box<dyn Error>> {
    model.ensure_valid()?;
    recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: model.scientific_sha256()?,
        model: model.clone(),
    };
    recipe.ensure_valid()?;
    Ok(())
}

fn add_interaction(
    model: &mut qpls_core::SemModelV4,
    interaction_id: &str,
    operands: &[&str],
    focal_predictor: &str,
) -> Result<(), Box<dyn Error>> {
    let focal_relation = model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source,
                target,
                role: StructuralRelationRoleV4::Structural,
                ..
            } if source == focal_predictor && target == "construct:y" => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("missing focal relation for {interaction_id}")))?;
    let output = format!("derived:{interaction_id}");
    let relation_id = format!("relation:{interaction_id}:effect");
    let parameter_id = format!("parameter:{interaction_id}:effect");
    model.variables.push(SemVariableV4::Derived {
        id: output.clone(),
        label: interaction_id.into(),
    });
    model.relations.push(SemRelationV4::Structural {
        id: relation_id,
        source: output.clone(),
        target: "construct:y".into(),
        parameter: parameter_id.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter_id,
        label: format!("{interaction_id} -> Y"),
        target: SemParameterTargetV4::Regression {
            source: output.clone(),
            target: "construct:y".into(),
        },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
        id: interaction_id.into(),
        output,
        operands: operands.iter().map(|operand| (*operand).into()).collect(),
        focal_relation,
        method: InteractionMethodV4::TwoStage,
        hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
        product_indicator: None,
    });
    Ok(())
}
