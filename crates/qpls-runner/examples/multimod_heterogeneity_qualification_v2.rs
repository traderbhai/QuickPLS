//! Production raw-data FIMIX-PLS/PLS-POS V2 qualification producer.
//!
//! The executable is validation-only. Every score, interaction product,
//! segment refit, common-metric refit, and fixed-K bootstrap is delegated to
//! the public Recipe V4 raw runner. Synthetic truth is retained only for the
//! independent oracle; it is never supplied to an estimator.

#[path = "support_multimod_metamorphic/mod.rs"]
mod metamorphic;
#[path = "support_multimod_qualification/mod.rs"]
mod support;

use qpls_core::*;
use qpls_estimation::{
    FIMIX_PLS_METHOD_VERSION_V2, HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
    HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2,
    PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2, PLS_POS_PUBLISHED_METHOD_VERSION_V2,
    align_labels_exhaustive_v2,
};
use qpls_resampling::MultiModShardSpecV1;
use qpls_runner::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use support::*;

const SCHEMA_VERSION: u32 = 1;
const SUITE_ID: &str = "qpls.multimod.heterogeneity.production-qualification.v2";
const OBSERVATIONS: usize = 400;
const MULTICLASS_POINT_OBSERVATIONS: usize = 120;
const MULTICLASS_POINT_FIXTURE_PLAN_ID: &str =
    "qpls.multimod.heterogeneity.pos-published-p0-k3-k5-point-discovery.v1";
const BOOTSTRAP_PREPARED_SUITE_ID: &str =
    "qpls.multimod.heterogeneity.bootstrap-prepared-execution.v1";
const BOOTSTRAP_PREPARED_RECEIPT_SUITE_ID: &str =
    "qpls.multimod.heterogeneity.bootstrap-prepared-execution-receipt.v1";
const BOOTSTRAP_CACHE_SUITE_ID: &str = "qpls.multimod.heterogeneity.bootstrap-shard-cache.v1";
const BOOTSTRAP_CACHE_RECEIPT_SUITE_ID: &str =
    "qpls.multimod.heterogeneity.bootstrap-shard-cache-receipt.v1";
const BOOTSTRAP_CURRENT_POINTER_SUITE_ID: &str =
    "qpls.multimod.heterogeneity.bootstrap-current-generation.v1";
const BOOTSTRAP_CACHE_INVENTORY_SUITE_ID: &str =
    "qpls.multimod.heterogeneity.bootstrap-cache-inventory.v1";
const QUALIFICATION_BOOTSTRAP_DRAWS: u32 = 500;
const DEFAULT_BOOTSTRAP_CHUNK_COUNT: u32 = 100;
const DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS: u64 = 1_500;

fn fixture_observations() -> usize {
    if metamorphic::compact_matrix_v1() {
        80
    } else {
        OBSERVATIONS
    }
}

fn multiclass_point_fixture_plan() -> Value {
    json!({
        "schema_version": 1,
        "plan_id": MULTICLASS_POINT_FIXTURE_PLAN_ID,
        "purpose": "published_p0_pos_candidate_point_discovery_only",
        "selected_k": [3, 4, 5],
        "observations_per_fixture": MULTICLASS_POINT_OBSERVATIONS,
        "allocation": "row_mod_k_exactly_balanced",
        "bootstrap_evidence": "not_requested",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

struct Arguments {
    output: PathBuf,
    seed: u64,
    scale: Scale,
    mode: ExecutionMode,
    dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExecutionMode {
    Monolithic,
    Plan,
    Shard(String),
    BootstrapPrepare {
        shard_id: String,
        budget_seconds: u64,
    },
    BootstrapChunk {
        shard_id: String,
        prepared_execution: PathBuf,
        resume_cache: Option<PathBuf>,
        chunk_index: u32,
        chunk_count: u32,
        budget_seconds: u64,
    },
    BootstrapFinalize {
        shard_id: String,
        prepared_execution: PathBuf,
        cache_inventory: PathBuf,
    },
}

fn arguments() -> Result<Arguments, DynError> {
    let mut output = None;
    let mut seed = 42_u64;
    let mut scale = Scale::Qualification;
    let mut mode = ExecutionMode::Monolithic;
    let mut dependencies = Vec::new();
    let mut prepared_execution = None;
    let mut resume_cache = None;
    let mut cache_inventory = None;
    let mut chunk_index = None;
    let mut chunk_count = DEFAULT_BOOTSTRAP_CHUNK_COUNT;
    let mut budget_seconds = DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS;
    let mut values = env::args().skip(1);
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--output" => output = values.next().map(PathBuf::from),
            "--seed" => {
                seed = values
                    .next()
                    .ok_or_else(|| invalid("--seed requires a value"))?
                    .parse()?
            }
            "--scale" => {
                scale = Scale::parse(
                    &values
                        .next()
                        .ok_or_else(|| invalid("--scale requires a value"))?,
                )?
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
            "--bootstrap-prepare" => {
                if mode != ExecutionMode::Monolithic {
                    return Err(invalid("execution modes are mutually exclusive"));
                }
                mode = ExecutionMode::BootstrapPrepare {
                    shard_id: values
                        .next()
                        .ok_or_else(|| invalid("--bootstrap-prepare requires a shard id"))?,
                    budget_seconds: DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS,
                };
            }
            "--bootstrap-chunk" => {
                if mode != ExecutionMode::Monolithic {
                    return Err(invalid("execution modes are mutually exclusive"));
                }
                mode = ExecutionMode::BootstrapChunk {
                    shard_id: values
                        .next()
                        .ok_or_else(|| invalid("--bootstrap-chunk requires a shard id"))?,
                    prepared_execution: PathBuf::new(),
                    resume_cache: None,
                    chunk_index: 0,
                    chunk_count: DEFAULT_BOOTSTRAP_CHUNK_COUNT,
                    budget_seconds: DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS,
                };
            }
            "--bootstrap-finalize" => {
                if mode != ExecutionMode::Monolithic {
                    return Err(invalid("execution modes are mutually exclusive"));
                }
                mode = ExecutionMode::BootstrapFinalize {
                    shard_id: values
                        .next()
                        .ok_or_else(|| invalid("--bootstrap-finalize requires a shard id"))?,
                    prepared_execution: PathBuf::new(),
                    cache_inventory: PathBuf::new(),
                };
            }
            "--prepared-execution" => {
                prepared_execution =
                    Some(PathBuf::from(values.next().ok_or_else(|| {
                        invalid("--prepared-execution requires a path")
                    })?));
            }
            "--resume-cache" => {
                resume_cache = Some(PathBuf::from(
                    values
                        .next()
                        .ok_or_else(|| invalid("--resume-cache requires a path"))?,
                ));
            }
            "--cache-inventory" => {
                if cache_inventory.is_some() {
                    return Err(invalid("--cache-inventory may be supplied only once"));
                }
                cache_inventory =
                    Some(PathBuf::from(values.next().ok_or_else(|| {
                        invalid("--cache-inventory requires a path")
                    })?));
            }
            "--chunk-index" => {
                chunk_index = Some(
                    values
                        .next()
                        .ok_or_else(|| invalid("--chunk-index requires a value"))?
                        .parse()?,
                )
            }
            "--chunk-count" => {
                chunk_count = values
                    .next()
                    .ok_or_else(|| invalid("--chunk-count requires a value"))?
                    .parse()?
            }
            "--budget-seconds" => {
                budget_seconds = values
                    .next()
                    .ok_or_else(|| invalid("--budget-seconds requires a value"))?
                    .parse()?
            }
            "--dependency" => dependencies.push(PathBuf::from(
                values
                    .next()
                    .ok_or_else(|| invalid("--dependency requires a shard result path"))?,
            )),
            _ => return Err(invalid(format!("unknown argument {argument}"))),
        }
    }
    mode = match mode {
        ExecutionMode::BootstrapChunk { shard_id, .. } => {
            if !(1..=DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS).contains(&budget_seconds)
                || chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
                || chunk_index.is_none_or(|index| index >= chunk_count)
                || prepared_execution.is_none()
                || cache_inventory.is_some()
            {
                return Err(invalid(
                    "bootstrap chunk requires one prepared execution, an index in the frozen 100-chunk plan, no final cache inventory, and a 1 through 1500 second budget",
                ));
            }
            ExecutionMode::BootstrapChunk {
                shard_id,
                prepared_execution: prepared_execution.expect("validated prepared path"),
                resume_cache,
                chunk_index: chunk_index.expect("validated chunk index"),
                chunk_count,
                budget_seconds,
            }
        }
        ExecutionMode::BootstrapFinalize { shard_id, .. } => {
            if prepared_execution.is_none()
                || resume_cache.is_some()
                || chunk_index.is_some()
                || chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
                || budget_seconds != DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS
                || cache_inventory.is_none()
            {
                return Err(invalid(
                    "bootstrap finalization requires one prepared execution and one --cache-inventory manifest",
                ));
            }
            ExecutionMode::BootstrapFinalize {
                shard_id,
                prepared_execution: prepared_execution.expect("validated prepared path"),
                cache_inventory: cache_inventory.expect("validated cache inventory"),
            }
        }
        ExecutionMode::BootstrapPrepare { shard_id, .. } => {
            if prepared_execution.is_some()
                || resume_cache.is_some()
                || cache_inventory.is_some()
                || chunk_index.is_some()
                || chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
                || !(1..=DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS).contains(&budget_seconds)
            {
                return Err(invalid(
                    "bootstrap preparation does not accept cache or prepared-execution arguments",
                ));
            }
            ExecutionMode::BootstrapPrepare {
                shard_id,
                budget_seconds,
            }
        }
        other => {
            if prepared_execution.is_some()
                || resume_cache.is_some()
                || cache_inventory.is_some()
                || chunk_index.is_some()
                || chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
                || budget_seconds != DEFAULT_BOOTSTRAP_PROCESS_BUDGET_SECONDS
            {
                return Err(invalid(
                    "bootstrap execution arguments require a bootstrap execution mode",
                ));
            }
            other
        }
    };
    if !dependencies.is_empty()
        && !matches!(
            &mode,
            ExecutionMode::Shard(_)
                | ExecutionMode::BootstrapPrepare { .. }
                | ExecutionMode::BootstrapChunk { .. }
                | ExecutionMode::BootstrapFinalize { .. }
        )
    {
        return Err(invalid(
            "--dependency is valid only with a scientific or bootstrap shard mode",
        ));
    }
    Ok(Arguments {
        output: output.ok_or_else(|| invalid("--output is required"))?,
        seed,
        scale,
        mode,
        dependencies,
    })
}

fn scale_id(scale: Scale) -> &'static str {
    match scale {
        Scale::Development => "development",
        Scale::Qualification => "qualification",
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    StrongSeparation,
    PowerModerate,
    Overlap,
    Imbalanced,
    NonNormal,
    HomogeneousNull,
    CommonMetricFailure,
    RankDeficient,
    VarianceCollapse,
    RareClass,
}

struct SplitMixNormal {
    state: u64,
    spare: Option<f64>,
}

impl SplitMixNormal {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare: None,
        }
    }

    fn uniform(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        ((value >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64)
    }

    fn normal(&mut self) -> f64 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform();
        self.spare = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

struct HeterogeneityFixture {
    fixture_id: String,
    dataset: qpls_data::Dataset,
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    profile: HeterogeneityInteractionProfileV2,
    true_classes: Vec<usize>,
    scenario: Scenario,
}

fn profile_constructs(
    profile: HeterogeneityInteractionProfileV2,
    rank_deficient: bool,
) -> (Vec<&'static str>, Vec<(&'static str, &'static str)>) {
    match (profile, rank_deficient) {
        (HeterogeneityInteractionProfileV2::P0Structural, false) => {
            (vec!["x", "y"], vec![("x", "y")])
        }
        (HeterogeneityInteractionProfileV2::P0Structural, true) => {
            (vec!["x", "z", "y"], vec![("x", "y"), ("z", "y")])
        }
        (
            HeterogeneityInteractionProfileV2::P2MultiTwoWay
            | HeterogeneityInteractionProfileV2::P23AllCurrent,
            _,
        ) => (
            vec!["x", "z", "w", "y"],
            vec![("x", "y"), ("z", "y"), ("w", "y")],
        ),
    }
}

fn make_fixture(
    fixture_id: &str,
    profile: HeterogeneityInteractionProfileV2,
    scenario: Scenario,
    data_seed: u64,
) -> Result<HeterogeneityFixture, DynError> {
    make_fixture_with_classes(fixture_id, profile, scenario, data_seed, 2)
}

fn make_fixture_with_classes(
    fixture_id: &str,
    profile: HeterogeneityInteractionProfileV2,
    scenario: Scenario,
    data_seed: u64,
    classes: usize,
) -> Result<HeterogeneityFixture, DynError> {
    make_fixture_with_classes_and_observations(
        fixture_id,
        profile,
        scenario,
        data_seed,
        classes,
        fixture_observations(),
    )
}

fn validate_multiclass_point_balance(
    fixture_id: &str,
    classes: usize,
    true_classes: &[usize],
) -> Result<(), DynError> {
    if !(3..=5).contains(&classes)
        || true_classes.len() != MULTICLASS_POINT_OBSERVATIONS
        || MULTICLASS_POINT_OBSERVATIONS % classes != 0
    {
        return Err(invalid(format!(
            "{fixture_id} does not match the typed K=3 through K=5 n=120 point-fixture plan"
        )));
    }
    let expected = MULTICLASS_POINT_OBSERVATIONS / classes;
    let counts = (0..classes)
        .map(|class| true_classes.iter().filter(|value| **value == class).count())
        .collect::<Vec<_>>();
    if counts.iter().any(|count| *count != expected)
        || true_classes.iter().any(|class| *class >= classes)
    {
        return Err(invalid(format!(
            "{fixture_id} must contain exactly {expected} rows in each of {classes} classes; observed {counts:?}"
        )));
    }
    Ok(())
}

fn make_multiclass_point_fixture(
    fixture_id: &str,
    data_seed: u64,
    selected_k: u8,
) -> Result<HeterogeneityFixture, DynError> {
    let classes = usize::from(selected_k);
    let fixture = make_fixture_with_classes_and_observations(
        fixture_id,
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::StrongSeparation,
        data_seed,
        classes,
        MULTICLASS_POINT_OBSERVATIONS,
    )?;
    validate_multiclass_point_balance(fixture_id, classes, &fixture.true_classes)?;
    Ok(fixture)
}

fn strong_multiclass_p0_equation(class: usize, classes: usize) -> ([f64; 3], f64) {
    debug_assert!(classes > 2 && class < classes);
    const COEFFICIENTS: [[f64; 3]; 5] = [
        [2.2, 0.2, 0.2],
        [-2.0, 0.3, 0.3],
        [0.3, 2.2, 0.3],
        [0.3, -2.0, 0.3],
        [0.3, 0.3, 2.2],
    ];
    let centered_class = class as f64 - (classes - 1) as f64 / 2.0;
    (COEFFICIENTS[class], 0.35 * centered_class)
}

fn make_fixture_with_classes_and_observations(
    fixture_id: &str,
    profile: HeterogeneityInteractionProfileV2,
    scenario: Scenario,
    data_seed: u64,
    classes: usize,
    observations: usize,
) -> Result<HeterogeneityFixture, DynError> {
    if !(2..=5).contains(&classes)
        || (classes != 2
            && !matches!(
                scenario,
                Scenario::StrongSeparation | Scenario::PowerModerate | Scenario::Overlap
            ))
    {
        return Err(invalid(
            "heterogeneity qualification fixtures require 2 through 5 balanced classes; boundary scenarios remain two-class",
        ));
    }
    let mut random = SplitMixNormal::new(data_seed);
    let mut truth = Vec::with_capacity(observations);
    let mut latent_x = Vec::with_capacity(observations);
    let mut latent_z = Vec::with_capacity(observations);
    let mut latent_w = Vec::with_capacity(observations);
    let mut latent_y = Vec::with_capacity(observations);
    for row in 0..observations {
        let class = match scenario {
            Scenario::RareClass => usize::from(row >= observations - 10),
            Scenario::Imbalanced => usize::from(row >= 3 * observations / 4),
            _ => row % classes,
        };
        let raw_x = random.normal();
        let base_x = if scenario == Scenario::NonNormal {
            raw_x.signum() * raw_x.abs().powf(1.65)
        } else {
            raw_x
        };
        let x = base_x
            + if scenario == Scenario::CommonMetricFailure {
                if class == 0 { -1.8 } else { 1.8 }
            } else {
                0.0
            };
        let z = if scenario == Scenario::RankDeficient {
            x
        } else {
            0.25 * x + 0.968_245_836_551_854_3 * random.normal()
        };
        let w = -0.15 * x + 0.1 * z + 0.983_615_778_645_300_1 * random.normal();
        let noise = match scenario {
            Scenario::VarianceCollapse => 0.0,
            Scenario::HomogeneousNull => 0.55 * random.normal(),
            Scenario::PowerModerate => 0.42 * random.normal(),
            Scenario::Overlap => 0.72 * random.normal(),
            Scenario::Imbalanced => 0.14 * random.normal(),
            Scenario::NonNormal => {
                let value = random.normal();
                0.12 * value.signum() * value.abs().powf(1.5)
            }
            _ => 0.075 * random.normal(),
        };
        let class_sign = if classes == 2 {
            if class == 0 { -1.0 } else { 1.0 }
        } else {
            2.0 * class as f64 / (classes - 1) as f64 - 1.0
        };
        let y = match profile {
            HeterogeneityInteractionProfileV2::P0Structural => match scenario {
                Scenario::HomogeneousNull | Scenario::RankDeficient => {
                    0.8 * x
                        + if scenario == Scenario::RankDeficient {
                            0.2 * z
                        } else {
                            0.0
                        }
                        + noise
                }
                Scenario::VarianceCollapse => 1.25 * x,
                Scenario::PowerModerate => class_sign * (0.95 * x + 0.28) + noise,
                Scenario::Overlap => class_sign * (0.62 * x + 0.16) + noise,
                Scenario::Imbalanced => class_sign * (1.55 * x + 0.55) + noise,
                Scenario::NonNormal => class_sign * (1.45 * x + 0.45) + noise,
                Scenario::StrongSeparation if classes > 2 => {
                    // A one-path standardized model collapses coefficient magnitude
                    // when residual noise is small.  Use three ordinary structural
                    // paths so every K=3..5 segment has a distinct high-signal
                    // coefficient vector without adding any interaction term.
                    let (coefficients, intercept) = strong_multiclass_p0_equation(class, classes);
                    coefficients[0] * x
                        + coefficients[1] * z
                        + coefficients[2] * w
                        + intercept
                        + noise
                }
                _ => class_sign * (1.8 * x + 0.65) + noise,
            },
            HeterogeneityInteractionProfileV2::P2MultiTwoWay => {
                class_sign * (1.2 * x + 0.35 * z - 0.25 * w + 1.05 * x * z - 0.7 * x * w) + noise
            }
            HeterogeneityInteractionProfileV2::P23AllCurrent => {
                class_sign
                    * (0.9 * x + 0.25 * z - 0.2 * w + 0.7 * x * z - 0.45 * x * w
                        + 0.35 * z * w
                        + 0.85 * x * z * w)
                    + noise
            }
        };
        truth.push(class);
        latent_x.push(x);
        latent_z.push(z);
        latent_w.push(w);
        latent_y.push(y);
    }

    let mut headers = Vec::new();
    let mut columns = Vec::new();
    for (id, values) in [
        ("x", &latent_x),
        ("z", &latent_z),
        ("w", &latent_w),
        ("y", &latent_y),
    ] {
        for indicator in 1..=3 {
            headers.push(format!("{id}{indicator}"));
            let observed = values
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    let class = truth[row];
                    let metric_sign = if scenario == Scenario::CommonMetricFailure
                        && id == "x"
                        && class == 1
                        && indicator == 3
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    let perturbation = if scenario == Scenario::VarianceCollapse {
                        0.0
                    } else {
                        0.018 * (((row + 1) * (indicator + 5)) as f64 * 0.317).sin()
                    };
                    Some(format!(
                        "{:.17}",
                        metric_sign * value * (0.84 + indicator as f64 * 0.08) + perturbation
                    ))
                })
                .collect::<Vec<_>>();
            columns.push(observed);
        }
    }
    let (headers, columns) =
        metamorphic::transformed_columns_v1(&headers, &columns).map_err(invalid)?;
    metamorphic::transform_row_aligned_values_v1(&mut truth);
    let dataset = dataset_from_columns(&format!("{fixture_id}.csv"), &headers, &columns)?;

    let (construct_ids, paths) = if profile == HeterogeneityInteractionProfileV2::P0Structural
        && scenario == Scenario::StrongSeparation
        && classes > 2
    {
        (
            vec!["x", "z", "w", "y"],
            vec![("x", "y"), ("z", "y"), ("w", "y")],
        )
    } else {
        profile_constructs(profile, scenario == Scenario::RankDeficient)
    };
    let owned = construct_ids
        .iter()
        .map(|id| {
            (
                *id,
                (1..=3)
                    .map(|indicator| format!("{id}{indicator}"))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let borrowed = owned
        .iter()
        .map(|(id, indicators)| {
            (
                *id,
                indicators.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let construct_slices = borrowed
        .iter()
        .map(|(id, indicators)| (*id, indicators.as_slice()))
        .collect::<Vec<_>>();
    let fixture_hash = fixture_id.as_bytes().iter().fold(0_u128, |hash, byte| {
        hash.wrapping_mul(257).wrapping_add(*byte as u128)
    });
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0x4845_5445_524f_0000_0000_0000_0000_0000_u128 ^ fixture_hash,
        fixture_id,
        &construct_slices,
        &paths,
        data_seed,
    )?;
    match profile {
        HeterogeneityInteractionProfileV2::P0Structural => {}
        HeterogeneityInteractionProfileV2::P2MultiTwoWay => {
            add_interaction(
                &mut model,
                "interaction:x_by_z",
                &["construct:x", "construct:z"],
                "construct:x",
                "construct:y",
            )?;
            add_interaction(
                &mut model,
                "interaction:x_by_w",
                &["construct:x", "construct:w"],
                "construct:x",
                "construct:y",
            )?;
        }
        HeterogeneityInteractionProfileV2::P23AllCurrent => {
            for (id, operands, focal) in [
                (
                    "interaction:x_by_z",
                    vec!["construct:x", "construct:z"],
                    "construct:x",
                ),
                (
                    "interaction:x_by_w",
                    vec!["construct:x", "construct:w"],
                    "construct:x",
                ),
                (
                    "interaction:z_by_w",
                    vec!["construct:z", "construct:w"],
                    "construct:z",
                ),
                (
                    "interaction:x_by_z_by_w",
                    vec!["construct:x", "construct:z", "construct:w"],
                    "construct:x",
                ),
            ] {
                add_interaction(&mut model, id, &operands, focal, "construct:y")?;
            }
        }
    }
    finalize_recipe(&mut recipe, &model)?;
    Ok(HeterogeneityFixture {
        fixture_id: fixture_id.into(),
        dataset,
        recipe,
        model,
        profile,
        true_classes: truth,
        scenario,
    })
}

fn complete_micom_checklist() -> MicomConfiguralChecklistV1 {
    MicomConfiguralChecklistV1 {
        identical_indicators_and_coding: true,
        identical_data_treatment: true,
        identical_algorithm_settings: true,
        identical_model_specification: true,
        deterministic_sign_orientation_reviewed: true,
        analyst_review_confirmed: true,
    }
}

fn discovery_config(
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
) -> PlsUnobservedHeterogeneityConfigV2 {
    PlsUnobservedHeterogeneityConfigV2 {
        schema_version: PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
        profile: fixture.profile,
        phase: HeterogeneityPhaseV2::Discovery {
            candidate_k,
            algorithms,
        },
        seed,
        fimix: FimixSettingsV2::default(),
        pls_pos: PlsPosSettingsV2::default(),
        pos_common_metric: None,
        bootstrap: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn inference_config(
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
    discovery_result_identity_sha256: String,
    selected_algorithm: HeterogeneityAlgorithmV2,
    selected_k: u8,
    request_common_metric: bool,
) -> PlsUnobservedHeterogeneityConfigV2 {
    let tandem_fimix_same_k_start_required = selected_algorithm
        != HeterogeneityAlgorithmV2::FimixPlsV2
        && algorithms.contains(&HeterogeneityAlgorithmV2::FimixPlsV2);
    PlsUnobservedHeterogeneityConfigV2 {
        schema_version: PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
        profile: fixture.profile,
        phase: HeterogeneityPhaseV2::Inference {
            lock: HeterogeneityInferenceLockReceiptV2 {
                schema_version: HETEROGENEITY_INFERENCE_LOCK_V2_SCHEMA_VERSION,
                discovery_result_identity_sha256,
                discovery_candidate_k: candidate_k,
                discovery_algorithms: algorithms,
                selected_algorithm,
                selected_k,
                analyst_lock_confirmed: true,
                tandem_fimix_same_k_start_required,
            },
        },
        seed,
        fimix: FimixSettingsV2::default(),
        pls_pos: PlsPosSettingsV2::default(),
        pos_common_metric: request_common_metric.then(|| PosCommonMetricComparabilityV1 {
            schema_version: 1,
            request_segment_contrasts: true,
            permutation_samples: 5_000,
            configural_checklist: complete_micom_checklist(),
            require_partial_compositional_invariance: true,
        }),
        bootstrap: Some(SegmentationBootstrapV2 {
            resamples: 500,
            seed: seed ^ 0x4253_5452_4150,
            confidence_level: 0.95,
        }),
    }
}

fn summarize_evidence(evidence: &[MultiModRunnerEvidenceV1]) -> Value {
    let mut fimix = Vec::new();
    let mut pos = Vec::new();
    let mut pooled = Vec::new();
    let mut raw = Vec::new();
    let mut common_metric = Vec::new();
    let mut bootstrap = Vec::new();
    for row in evidence {
        match row {
            MultiModRunnerEvidenceV1::FimixCandidate { k, result } => fimix.push(json!({
                "k": k,
                "result": result,
            })),
            MultiModRunnerEvidenceV1::PlsPosCandidate { k, result } => pos.push(json!({
                "k": k,
                "result": result,
            })),
            MultiModRunnerEvidenceV1::HeterogeneityPooledBaseline(value) => {
                pooled.push(serde_json::to_value(value).expect("serializable pooled baseline"))
            }
            MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(value) => {
                raw.push(serde_json::to_value(value).expect("serializable raw preparation"))
            }
            MultiModRunnerEvidenceV1::HeterogeneityPosCommonMetric(value) => common_metric
                .push(serde_json::to_value(value).expect("serializable common-metric evidence")),
            MultiModRunnerEvidenceV1::HeterogeneityBootstrap(value) => bootstrap
                .push(serde_json::to_value(value).expect("serializable heterogeneity bootstrap")),
            _ => {}
        }
    }
    json!({
        "fimix": fimix,
        "pos": pos,
        "pooled_baseline": pooled,
        "raw_preparation": raw,
        "common_metric": common_metric,
        "bootstrap": bootstrap,
    })
}

fn partitions_from_overlap(overlap: &[Vec<usize>]) -> Result<(Vec<usize>, Vec<usize>), DynError> {
    let classes = overlap.len();
    if !(3..=5).contains(&classes) || overlap.iter().any(|row| row.len() != classes) {
        return Err(invalid(
            "label-alignment decision fixture requires a square K=3 through K=5 overlap",
        ));
    }
    let mut reference = Vec::new();
    let mut candidate = Vec::new();
    for (reference_label, row) in overlap.iter().enumerate() {
        for (candidate_label, count) in row.iter().copied().enumerate() {
            for _ in 0..count {
                reference.push(reference_label);
                candidate.push(candidate_label);
            }
        }
    }
    Ok((reference, candidate))
}

fn label_alignment_probe(
    case_id: &str,
    classes: usize,
    reference: Vec<usize>,
    candidate: Vec<usize>,
    expected_ambiguous: bool,
    expected_mutual_majority: bool,
) -> Result<Value, DynError> {
    let alignment = align_labels_exhaustive_v2(&reference, &candidate, classes)?;
    Ok(json!({
        "case_id": case_id,
        "k": classes,
        "reference": reference,
        "candidate": candidate,
        "expected_ambiguous": expected_ambiguous,
        "expected_mutual_majority": expected_mutual_majority,
        "sut_alignment": alignment,
    }))
}

fn label_alignment_decision_matrix() -> Result<Vec<Value>, DynError> {
    let mut probes = Vec::new();
    for classes in 3..=5 {
        let reference = (0..classes)
            .flat_map(|label| std::iter::repeat(label).take(6))
            .collect::<Vec<_>>();
        let candidate = reference
            .iter()
            .map(|label| (*label + 1) % classes)
            .collect::<Vec<_>>();
        probes.push(label_alignment_probe(
            &format!("k{classes}-nonidentity-mutual-majority"),
            classes,
            reference,
            candidate,
            false,
            true,
        )?);

        let mut ambiguous_overlap = vec![vec![0usize; classes]; classes];
        ambiguous_overlap[0][0] = 1;
        ambiguous_overlap[0][1] = 1;
        ambiguous_overlap[1][0] = 1;
        ambiguous_overlap[1][1] = 1;
        for class in 2..classes {
            ambiguous_overlap[class][class] = 2;
        }
        let (reference, candidate) = partitions_from_overlap(&ambiguous_overlap)?;
        probes.push(label_alignment_probe(
            &format!("k{classes}-ambiguous"),
            classes,
            reference,
            candidate,
            true,
            false,
        )?);

        let mut nonmajority_overlap = vec![vec![0usize; classes]; classes];
        nonmajority_overlap[0][0] = 2;
        nonmajority_overlap[0][1] = 1;
        nonmajority_overlap[1][0] = 1;
        nonmajority_overlap[1][1] = 2;
        nonmajority_overlap[2][0] = 1;
        nonmajority_overlap[2][2] = 3;
        for class in 3..classes {
            nonmajority_overlap[class][class] = 3;
        }
        let (reference, candidate) = partitions_from_overlap(&nonmajority_overlap)?;
        probes.push(label_alignment_probe(
            &format!("k{classes}-unique-nonmajority"),
            classes,
            reference,
            candidate,
            false,
            false,
        )?);
    }
    Ok(probes)
}

fn run_config(
    cell_id: &str,
    fixture: &HeterogeneityFixture,
    config: PlsUnobservedHeterogeneityConfigV2,
) -> Result<(String, Value), DynError> {
    let mut recipe = fixture.recipe.clone();
    let mut model = fixture.model.clone();
    recipe.settings.workers =
        metamorphic::configured_workers_v1(recipe.settings.workers).map_err(invalid)?;
    metamorphic::transform_model_declaration_order_v1(&mut model);
    stage_additive_multimod_recipe(&mut recipe, AnalysisMethod::Predict);
    recipe.pls_heterogeneity = Some(config.clone());
    finalize_recipe(&mut recipe, &model)?;
    let artifact = prepare_multimod_recipe_v1(
        &fixture.dataset,
        &recipe,
        &model,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let output = run_compiled_raw_pls_heterogeneity_v2(
        &fixture.dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )?;
    let MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(analysis) = &output.result else {
        return Err(invalid(
            "raw heterogeneity runner returned the wrong result family",
        ));
    };
    let identity = analysis.discovery_result_identity_sha256.clone();
    Ok((
        identity,
        json!({
            "cell_id": cell_id,
            "fixture_id": fixture.fixture_id,
            "scenario": format!("{:?}", fixture.scenario).to_lowercase(),
            "profile": fixture.profile,
            "dataset_rows": fixture.dataset.batch.num_rows(),
            "dataset_fingerprint": fixture.dataset.fingerprint.0.clone(),
            "config": config,
            "compiler_receipt": output.compilation_receipt,
            "compiled_plan": artifact.plan(),
            "sem_model_authority": model,
            "analysis": analysis,
            "evidence": summarize_evidence(&output.evidence),
            "true_classes": fixture.true_classes,
        }),
    ))
}

fn run_discovery(
    cell_id: &str,
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
) -> Result<(String, Value), DynError> {
    run_config(
        cell_id,
        fixture,
        discovery_config(fixture, seed, candidate_k, algorithms),
    )
}

fn run_required_discovery(
    cell_id: &str,
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
    required_candidates: &[(HeterogeneityAlgorithmV2, u8)],
) -> Result<(String, Value), DynError> {
    if required_candidates.is_empty() {
        return Err(invalid(
            "required discovery must name at least one exact algorithm/K candidate",
        ));
    }
    let (_, value) = run_discovery(cell_id, fixture, seed, candidate_k, algorithms)?;
    let identities = required_candidates
        .iter()
        .map(|(algorithm, k)| stable_discovery_identity(&value, cell_id, *algorithm, *k))
        .collect::<Result<Vec<_>, _>>()?;
    if identities.iter().any(|identity| identity != &identities[0]) {
        return Err(invalid(format!(
            "discovery {cell_id} produced inconsistent candidate-bound identities"
        )));
    }
    Ok((identities[0].clone(), value))
}

#[allow(clippy::too_many_arguments)]
fn run_inference(
    cell_id: &str,
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
    discovery_identity: String,
    selected_algorithm: HeterogeneityAlgorithmV2,
    request_common_metric: bool,
) -> Result<Value, DynError> {
    run_inference_at_k(
        cell_id,
        fixture,
        seed,
        candidate_k,
        algorithms,
        discovery_identity,
        selected_algorithm,
        2,
        request_common_metric,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_inference_at_k(
    cell_id: &str,
    fixture: &HeterogeneityFixture,
    seed: u64,
    candidate_k: Vec<u8>,
    algorithms: Vec<HeterogeneityAlgorithmV2>,
    discovery_identity: String,
    selected_algorithm: HeterogeneityAlgorithmV2,
    selected_k: u8,
    request_common_metric: bool,
) -> Result<Value, DynError> {
    let (_, value) = run_config(
        cell_id,
        fixture,
        inference_config(
            fixture,
            seed,
            candidate_k,
            algorithms,
            discovery_identity,
            selected_algorithm,
            selected_k,
            request_common_metric,
        ),
    )?;
    Ok(value)
}

fn attempted_boundary(cell_id: &str, fixture: &HeterogeneityFixture, seed: u64) -> Value {
    let mut truth_counts = vec![0usize; 2];
    for class in &fixture.true_classes {
        if let Some(count) = truth_counts.get_mut(*class) {
            *count += 1;
        }
    }
    match run_discovery(
        cell_id,
        fixture,
        seed,
        vec![2],
        vec![HeterogeneityAlgorithmV2::FimixPlsV2],
    ) {
        Ok((_, value)) => json!({
            "status": "completed",
            "fixture_id": fixture.fixture_id,
            "scenario": format!("{:?}", fixture.scenario).to_lowercase(),
            "true_class_counts": truth_counts,
            "run": value,
        }),
        Err(error) => json!({
            "status": "failed_closed",
            "fixture_id": fixture.fixture_id,
            "scenario": format!("{:?}", fixture.scenario).to_lowercase(),
            "true_class_counts": truth_counts,
            "error": error.to_string(),
            "error_type": "production_raw_runner_error",
        }),
    }
}

fn fimix_p0_simulation_series(
    cell_prefix: &str,
    scenario: Scenario,
    seeds: &[u64],
    data_seed_domain: u64,
) -> Result<Vec<Value>, DynError> {
    seeds
        .iter()
        .map(|seed| {
            let fixture = make_fixture(
                &format!("{cell_prefix}-data-seed-{seed}"),
                HeterogeneityInteractionProfileV2::P0Structural,
                scenario,
                data_seed_domain.wrapping_add(*seed),
            )?;
            run_discovery(
                &format!("{cell_prefix}-seed-{seed}"),
                &fixture,
                *seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            )
            .map(|(_, value)| value)
        })
        .collect()
}

const SHARD_SCHEMA_VERSION: u32 = 1;
const SHARD_SUITE_ID: &str = "qpls.multimod.heterogeneity.qualification-shard.v1";
const SHARD_PLAN_SUITE_ID: &str = "qpls.multimod.heterogeneity.qualification-shard-plan.v1";

fn shard_spec(
    shard_id: &str,
    payload_kind: &str,
    index: Option<usize>,
    dependencies: &[&str],
    resource_class: &str,
) -> Value {
    json!({
        "shard_id": shard_id,
        "payload_kind": payload_kind,
        "index": index,
        "dependencies": dependencies,
        "resource_class": resource_class,
        "parallel_safe_after_build": true,
    })
}

fn qualification_shard_specs(scale: Scale) -> Vec<Value> {
    let mut shards = vec![shard_spec("sentinel", "sentinel", None, &[], "sentinel")];
    let recovery_count = match scale {
        Scale::Development => 2,
        Scale::Qualification => 5,
    };
    let scenario_count = match scale {
        Scale::Development => 2,
        Scale::Qualification => 5,
    };
    let power_count = match scale {
        Scale::Development => 2,
        Scale::Qualification => 10,
    };
    for (prefix, payload_kind, count) in [
        ("fimix-recovery", "fimix_recovery", recovery_count),
        ("fimix-power", "fimix_power", power_count),
        ("fimix-overlap", "fimix_overlap", scenario_count),
        ("fimix-imbalance", "fimix_imbalance", scenario_count),
        ("fimix-nonnormal", "fimix_nonnormal", scenario_count),
    ] {
        for index in 0..count {
            shards.push(shard_spec(
                &format!("{prefix}-{index:02}"),
                payload_kind,
                Some(index),
                &["sentinel"],
                "point",
            ));
        }
    }
    for (shard_id, payload_kind) in [
        ("fimix-candidate-k", "fimix_candidate_k"),
        ("fimix-homogeneous-null", "fimix_homogeneous_null"),
        ("pos-published-p0-discovery", "pos_published_p0_discovery"),
        (
            "pos-destination-p2-discovery",
            "pos_destination_p2_discovery",
        ),
        (
            "pos-destination-p23-discovery",
            "pos_destination_p23_discovery",
        ),
        (
            "pos-common-metric-failure-discovery",
            "pos_common_metric_failure_discovery",
        ),
        (
            "pos-homogeneous-null-discovery",
            "pos_homogeneous_null_discovery",
        ),
        ("pos-overlap-discovery", "pos_overlap_discovery"),
        ("boundary-rank", "boundary_rank"),
        ("boundary-variance", "boundary_variance"),
        ("boundary-rare", "boundary_rare"),
    ] {
        shards.push(shard_spec(
            shard_id,
            payload_kind,
            None,
            &["sentinel"],
            "point",
        ));
    }
    if scale == Scale::Qualification {
        for selected_k in 3..=5 {
            let discovery_id = format!("pos-published-k{selected_k}-discovery");
            shards.push(shard_spec(
                &discovery_id,
                "pos_published_k_discovery",
                Some(selected_k),
                &["sentinel"],
                "point",
            ));
        }
        for (shard_id, dependency) in [
            ("bootstrap-fimix-p0", "fimix-recovery-00"),
            ("bootstrap-pos-published-p0", "pos-published-p0-discovery"),
            ("bootstrap-fimix-p2", "pos-destination-p2-discovery"),
            (
                "bootstrap-pos-destination-p2",
                "pos-destination-p2-discovery",
            ),
            ("bootstrap-fimix-p23", "pos-destination-p23-discovery"),
            (
                "bootstrap-pos-destination-p23",
                "pos-destination-p23-discovery",
            ),
            (
                "bootstrap-pos-common-metric-failure",
                "pos-common-metric-failure-discovery",
            ),
        ] {
            shards.push(shard_spec(
                shard_id,
                "bootstrap",
                None,
                &["sentinel", dependency],
                "bootstrap",
            ));
        }
    }
    shards
}

fn shard_plan(args: &Arguments) -> Result<Value, DynError> {
    Ok(json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_PLAN_SUITE_ID,
        "producer_suite_id": SUITE_ID,
        "scale": scale_id(args.scale),
        "campaign_seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
        "multiclass_point_fixture_plan": multiclass_point_fixture_plan(),
        "execution_contract": "one_cargo_build_then_dependency_aware_non_cargo_shards",
        "sentinel_shard_id": "sentinel",
        "aggregation_order": "plan_order",
        "shards": qualification_shard_specs(args.scale),
    }))
}

fn multistart_reproducibility_contract() -> Value {
    json!({
        "schema_version": HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
        "partition_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2),
        "coefficient_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2),
        "posterior_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2),
        "parameter_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2),
        "fit_statistic_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2),
        "partition_encoding": "u64_length_then_u64_labels_little_endian",
        "matrix_encoding": "u64_row_count_then_per_row_u64_length_and_f64_bits_little_endian",
        "completed_start_cardinality": "exactly_one_receipt_per_completed_start",
        "verification": "independent_exhaustive_alignment_and_tolerance_replay"
    })
}

fn shard_report_header(args: &Arguments) -> Result<Value, DynError> {
    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "suite_id": SUITE_ID,
        "scale": scale_id(args.scale),
        "campaign_seed": args.seed,
        "seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
        "multiclass_point_fixture_plan": multiclass_point_fixture_plan(),
        "execution_contract": "public_recipe_v4_compiler_plus_raw_fimix_pos_runner",
        "qualification_claim": "raw_sut_facts_for_independent_comparison_only",
        "multistart_reproducibility_contract": multistart_reproducibility_contract(),
        "label_alignment_decision_contract": "qpls_estimation::align_labels_exhaustive_v2",
        "required_profile_ids": [
            "fimix.p0_structural.v2",
            "fimix.p2_multi_two_way.v2",
            "fimix.p23_all_current.v2",
            "pos.published.p0_structural.v2",
            "pos.destination_scored.p2_multi_two_way.v2",
            "pos.destination_scored.p23_all_current.v2",
            "pos.common_metric.p2_multi_two_way.v1",
            "pos.common_metric.p23_all_current.v1"
        ],
    }))
}

fn dependency_envelopes(args: &Arguments, shard_id: &str) -> Result<Vec<Value>, DynError> {
    let plan = shard_plan(args)?;
    let spec = plan["shards"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["shard_id"] == shard_id))
        .ok_or_else(|| {
            invalid(format!(
                "unknown heterogeneity qualification shard {shard_id}"
            ))
        })?;
    let mut expected = spec["dependencies"]
        .as_array()
        .ok_or_else(|| invalid("shard plan dependencies are malformed"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("shard dependency id is not a string"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    expected.sort();
    let mut envelopes = Vec::new();
    let expected_metamorphism = metamorphic::metamorphism_v1();
    let expected_sign_columns = serde_json::to_value(sign_columns_identity()?)?;
    let expected_workers = metamorphic::configured_workers_v1(1).map_err(invalid)? as u64;
    let expected_multiclass_point_fixture_plan = multiclass_point_fixture_plan();
    for path in &args.dependencies {
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if value["schema_version"].as_u64() != Some(u64::from(SHARD_SCHEMA_VERSION))
            || value["suite_id"] != SHARD_SUITE_ID
            || value["scale"] != scale_id(args.scale)
            || value["campaign_seed"].as_u64() != Some(args.seed)
            || value["metamorphism"] != expected_metamorphism
            || value.get("sign_columns") != Some(&expected_sign_columns)
            || value["workers"].as_u64() != Some(expected_workers)
            || value["fixture_observations"].as_u64() != Some(fixture_observations() as u64)
            || value.get("multiclass_point_fixture_plan")
                != Some(&expected_multiclass_point_fixture_plan)
        {
            return Err(invalid(format!(
                "dependency {} has the wrong shard identity",
                path.display()
            )));
        }
        envelopes.push(value);
    }
    envelopes.sort_by(|left, right| left["shard_id"].as_str().cmp(&right["shard_id"].as_str()));
    let actual = envelopes
        .iter()
        .map(|value| {
            value["shard_id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("dependency shard id is absent"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if actual != expected {
        return Err(invalid(format!(
            "shard {shard_id} requires exact dependencies {expected:?}, received {actual:?}"
        )));
    }
    Ok(envelopes)
}

fn dependency_value<'a>(dependencies: &'a [Value], shard_id: &str) -> Result<&'a Value, DynError> {
    dependencies
        .iter()
        .find(|value| value["shard_id"] == shard_id)
        .and_then(|value| {
            value["payload"]["value"]
                .as_object()
                .map(|_| &value["payload"]["value"])
        })
        .ok_or_else(|| invalid(format!("dependency {shard_id} has no cell value")))
}

fn expected_heterogeneity_method_version(algorithm: HeterogeneityAlgorithmV2) -> &'static str {
    match algorithm {
        HeterogeneityAlgorithmV2::FimixPlsV2 => FIMIX_PLS_METHOD_VERSION_V2,
        HeterogeneityAlgorithmV2::PlsPosPublishedV2 => PLS_POS_PUBLISHED_METHOD_VERSION_V2,
        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
            PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2
        }
    }
}

fn stable_discovery_identity(
    value: &Value,
    source_id: &str,
    algorithm: HeterogeneityAlgorithmV2,
    k: u8,
) -> Result<String, DynError> {
    let algorithm_value = serde_json::to_value(algorithm)?;
    let candidates = value["analysis"]["candidates"]
        .as_array()
        .ok_or_else(|| invalid(format!("discovery {source_id} has no candidate table")))?;
    let matching = candidates
        .iter()
        .filter(|candidate| {
            candidate["k"].as_u64() == Some(u64::from(k))
                && candidate["method"]["kind"] == "segmentation"
                && candidate["method"]["algorithm"] == algorithm_value
        })
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(invalid(format!(
            "multimod.qualification.heterogeneity.discovery_candidate_identity_invalid: discovery {source_id} requires exactly one {algorithm:?} K={k} candidate"
        )));
    }
    let candidate = matching[0];
    let blockers_empty = candidate["blockers"].as_array().is_some_and(Vec::is_empty);
    let stable_starts = candidate["stable_starts"].as_u64().unwrap_or(0);
    let completed_starts = candidate["converged_starts"].as_u64().unwrap_or(0);
    if candidate["state"] != "converged_stable"
        || stable_starts < 2
        || completed_starts < stable_starts
        || !blockers_empty
    {
        return Err(invalid(format!(
            "multimod.qualification.heterogeneity.discovery_candidate_not_stable: discovery {source_id} {algorithm:?} K={k} has state={}, completed_starts={completed_starts}, stable_starts={stable_starts}, blockers={} ",
            candidate["state"], candidate["blockers"]
        )));
    }

    let evidence_key = if algorithm == HeterogeneityAlgorithmV2::FimixPlsV2 {
        "fimix"
    } else {
        "pos"
    };
    let method_version = expected_heterogeneity_method_version(algorithm);
    let evidence = value["evidence"][evidence_key].as_array().ok_or_else(|| {
        invalid(format!(
            "discovery {source_id} has no {evidence_key} evidence"
        ))
    })?;
    let matching_evidence = evidence
        .iter()
        .filter(|row| {
            row["k"].as_u64() == Some(u64::from(k))
                && row["result"]["method_version"] == method_version
        })
        .count();
    if matching_evidence != 1 {
        return Err(invalid(format!(
            "multimod.qualification.heterogeneity.discovery_point_evidence_invalid: discovery {source_id} requires exactly one {method_version} K={k} point result"
        )));
    }

    let identity = value["analysis"]["discovery_result_identity_sha256"]
        .as_str()
        .filter(|identity| {
            identity.len() == 64 && identity.bytes().all(|value| value.is_ascii_hexdigit())
        })
        .ok_or_else(|| {
            invalid(format!(
                "discovery {source_id} has no valid discovery identity"
            ))
        })?;
    Ok(identity.to_owned())
}

fn dependency_discovery_identity(
    dependencies: &[Value],
    shard_id: &str,
    algorithm: HeterogeneityAlgorithmV2,
    k: u8,
) -> Result<String, DynError> {
    stable_discovery_identity(
        dependency_value(dependencies, shard_id)?,
        shard_id,
        algorithm,
        k,
    )
}

struct CompiledBootstrapCell {
    cell_id: String,
    fixture: HeterogeneityFixture,
    config: PlsUnobservedHeterogeneityConfigV2,
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    artifact: CompiledMultiModRecipeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapPreparedEnvelopeV1 {
    schema_version: u32,
    suite_id: String,
    producer_suite_id: String,
    scientific_shard_id: String,
    scale: String,
    campaign_seed: u64,
    metamorphism: String,
    sign_columns: Option<String>,
    workers: usize,
    fixture_observations: usize,
    multiclass_point_fixture_plan: Value,
    dependency_shard_ids: Vec<String>,
    cell_id: String,
    chunk_count: u32,
    requested_replicates: u32,
    execution: PreparedRawHeterogeneityBootstrapExecutionV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapCacheEnvelopeV1 {
    schema_version: u32,
    suite_id: String,
    producer_suite_id: String,
    scientific_shard_id: String,
    scale: String,
    campaign_seed: u64,
    metamorphism: String,
    sign_columns: Option<String>,
    workers: usize,
    fixture_observations: usize,
    multiclass_point_fixture_plan: Value,
    dependency_shard_ids: Vec<String>,
    cell_id: String,
    prepared_execution_identity_sha256: String,
    chunk_index: u32,
    chunk_count: u32,
    requested_replicates: u32,
    prior_record_count: usize,
    record_count: usize,
    expected_record_count: usize,
    completed: bool,
    cache: RawHeterogeneityBootstrapShardCacheV2,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapGenerationPointerV1 {
    schema_version: u32,
    suite_id: String,
    scientific_shard_id: String,
    kind: String,
    chunk_index: Option<u32>,
    generation_id: String,
    payload_file: String,
    payload_sha256: String,
    receipt_file: String,
    receipt_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapDependencyReceiptInventoryV1 {
    shard_id: String,
    receipt_sha256: String,
    result_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapCacheInventoryEntryV1 {
    chunk_index: u32,
    record_count: usize,
    cache_file: String,
    cache_sha256: String,
    receipt_file: String,
    receipt_sha256: String,
    pointer_file: String,
    pointer_sha256: String,
    cache_shard_identity_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapCacheInventoryV1 {
    schema_version: u32,
    suite_id: String,
    producer_suite_id: String,
    scientific_shard_id: String,
    scale: String,
    campaign_seed: u64,
    metamorphism: String,
    sign_columns: Option<String>,
    workers: usize,
    fixture_observations: usize,
    multiclass_point_fixture_plan: Value,
    dependency_shard_ids: Vec<String>,
    dependency_receipts: Vec<BootstrapDependencyReceiptInventoryV1>,
    cell_id: String,
    plan_sha256: String,
    producer_executable_sha256: String,
    source_commit: String,
    chunk_count: u32,
    requested_replicates: u32,
    prepared_execution_file: String,
    prepared_execution_sha256: String,
    prepared_receipt_file: String,
    prepared_receipt_sha256: String,
    prepared_pointer_file: String,
    prepared_pointer_sha256: String,
    execution_identity_sha256: String,
    caches: Vec<BootstrapCacheInventoryEntryV1>,
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn sha256_file(path: &Path) -> Result<String, DynError> {
    Ok(sha256_bytes(&fs::read(path)?))
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn resolve_inventory_file(directory: &Path, name: &str, label: &str) -> Result<PathBuf, DynError> {
    let relative = Path::new(name);
    let mut components = relative.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(invalid(format!(
            "bootstrap cache inventory {label} must be one relative file name"
        )));
    }
    Ok(directory.join(relative))
}

fn inventory_file_with_digest(
    directory: &Path,
    name: &str,
    digest: &str,
    label: &str,
) -> Result<PathBuf, DynError> {
    if !is_lower_hex(digest, 64) {
        return Err(invalid(format!(
            "bootstrap cache inventory {label} digest is invalid"
        )));
    }
    let path = resolve_inventory_file(directory, name, label)?;
    if sha256_file(&path)? != digest {
        return Err(invalid(format!(
            "bootstrap cache inventory {label} is missing or altered"
        )));
    }
    Ok(path)
}

#[allow(clippy::too_many_arguments)]
fn validate_generation_pointer(
    directory: &Path,
    pointer_name: &str,
    pointer_sha256: &str,
    shard_id: &str,
    kind: &str,
    chunk_index: Option<u32>,
    payload_name: &str,
    payload_sha256: &str,
    receipt_name: &str,
    receipt_sha256: &str,
) -> Result<BootstrapGenerationPointerV1, DynError> {
    let pointer_path = inventory_file_with_digest(
        directory,
        pointer_name,
        pointer_sha256,
        "generation pointer",
    )?;
    let pointer: BootstrapGenerationPointerV1 = serde_json::from_slice(&fs::read(pointer_path)?)?;
    if pointer.schema_version != SCHEMA_VERSION
        || pointer.suite_id != BOOTSTRAP_CURRENT_POINTER_SUITE_ID
        || pointer.scientific_shard_id != shard_id
        || pointer.kind != kind
        || pointer.chunk_index != chunk_index
        || !is_lower_hex(&pointer.generation_id, 32)
        || pointer.payload_file != payload_name
        || pointer.payload_sha256 != payload_sha256
        || pointer.receipt_file != receipt_name
        || pointer.receipt_sha256 != receipt_sha256
    {
        return Err(invalid(format!(
            "bootstrap cache inventory {kind} generation pointer identity is invalid"
        )));
    }
    Ok(pointer)
}

fn validate_prepared_transport_receipt(
    receipt_path: &Path,
    pointer: &BootstrapGenerationPointerV1,
    inventory: &BootstrapCacheInventoryV1,
) -> Result<(), DynError> {
    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path)?)?;
    let dependency_receipts = serde_json::to_value(&inventory.dependency_receipts)?;
    if receipt["schema_version"].as_u64() != Some(u64::from(SCHEMA_VERSION))
        || receipt["suite_id"].as_str() != Some(BOOTSTRAP_PREPARED_RECEIPT_SUITE_ID)
        || receipt["status"].as_str() != Some("passed")
        || receipt["generation_id"].as_str() != Some(pointer.generation_id.as_str())
        || receipt["scientific_shard_id"].as_str() != Some(inventory.scientific_shard_id.as_str())
        || receipt["scale"].as_str() != Some(inventory.scale.as_str())
        || receipt["campaign_seed"].as_u64() != Some(inventory.campaign_seed)
        || receipt["plan_sha256"].as_str() != Some(inventory.plan_sha256.as_str())
        || receipt["producer_executable_sha256"].as_str()
            != Some(inventory.producer_executable_sha256.as_str())
        || receipt["source_commit"].as_str() != Some(inventory.source_commit.as_str())
        || receipt["prepared_execution_sha256"].as_str()
            != Some(inventory.prepared_execution_sha256.as_str())
        || receipt["execution_identity_sha256"].as_str()
            != Some(inventory.execution_identity_sha256.as_str())
        || receipt.get("dependency_receipts") != Some(&dependency_receipts)
    {
        return Err(invalid(
            "bootstrap prepared transport receipt is stale or mixed",
        ));
    }
    Ok(())
}

fn validate_cache_transport_receipt(
    receipt_path: &Path,
    pointer: &BootstrapGenerationPointerV1,
    inventory: &BootstrapCacheInventoryV1,
    entry: &BootstrapCacheInventoryEntryV1,
) -> Result<(), DynError> {
    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path)?)?;
    let dependency_receipts = serde_json::to_value(&inventory.dependency_receipts)?;
    let expected_records =
        expected_owned_bootstrap_records(entry.chunk_index, inventory.chunk_count);
    if receipt["schema_version"].as_u64() != Some(u64::from(SCHEMA_VERSION))
        || receipt["suite_id"].as_str() != Some(BOOTSTRAP_CACHE_RECEIPT_SUITE_ID)
        || receipt["status"].as_str() != Some("complete")
        || receipt["generation_id"].as_str() != Some(pointer.generation_id.as_str())
        || receipt["scientific_shard_id"].as_str() != Some(inventory.scientific_shard_id.as_str())
        || receipt["chunk_index"].as_u64() != Some(u64::from(entry.chunk_index))
        || receipt["chunk_count"].as_u64() != Some(u64::from(inventory.chunk_count))
        || receipt["scale"].as_str() != Some(inventory.scale.as_str())
        || receipt["campaign_seed"].as_u64() != Some(inventory.campaign_seed)
        || receipt["plan_sha256"].as_str() != Some(inventory.plan_sha256.as_str())
        || receipt["producer_executable_sha256"].as_str()
            != Some(inventory.producer_executable_sha256.as_str())
        || receipt["source_commit"].as_str() != Some(inventory.source_commit.as_str())
        || receipt["prepared_execution_sha256"].as_str()
            != Some(inventory.prepared_execution_sha256.as_str())
        || receipt["prepared_receipt_sha256"].as_str()
            != Some(inventory.prepared_receipt_sha256.as_str())
        || receipt["execution_identity_sha256"].as_str()
            != Some(inventory.execution_identity_sha256.as_str())
        || receipt["cache_sha256"].as_str() != Some(entry.cache_sha256.as_str())
        || receipt["cache_shard_identity_sha256"].as_str()
            != Some(entry.cache_shard_identity_sha256.as_str())
        || receipt["record_count"].as_u64() != Some(entry.record_count as u64)
        || receipt["expected_record_count"].as_u64() != Some(expected_records as u64)
        || receipt.get("dependency_receipts") != Some(&dependency_receipts)
    {
        return Err(invalid(format!(
            "bootstrap cache transport receipt is stale or mixed for chunk {}",
            entry.chunk_index
        )));
    }
    Ok(())
}

fn bootstrap_dependency_ids(dependencies: &[Value]) -> Result<Vec<String>, DynError> {
    dependencies
        .iter()
        .map(|value| {
            value["shard_id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("bootstrap dependency shard identity is absent"))
        })
        .collect()
}

fn bootstrap_cell_definition(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
) -> Result<
    (
        String,
        HeterogeneityFixture,
        PlsUnobservedHeterogeneityConfigV2,
    ),
    DynError,
> {
    let tandem_p0 = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosPublishedV2,
    ];
    let tandem_interactions = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
    ];
    let (cell_id, fixture, algorithms, dependency_id, selected_algorithm, common_metric) =
        match shard_id {
            "bootstrap-fimix-p0" => (
                "fimix-p0-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p0-strong",
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::StrongSeparation,
                    0x5000_u64.wrapping_add(args.seed),
                )?,
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
                "fimix-recovery-00",
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            ),
            "bootstrap-pos-published-p0" => (
                "pos-published-p0-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p0-strong",
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::StrongSeparation,
                    0x5000_u64.wrapping_add(args.seed),
                )?,
                tandem_p0,
                "pos-published-p0-discovery",
                HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                true,
            ),
            "bootstrap-fimix-p2" => (
                "fimix-p2-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p2-strong",
                    HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                    Scenario::StrongSeparation,
                    0x5200_u64.wrapping_add(args.seed),
                )?,
                tandem_interactions,
                "pos-destination-p2-discovery",
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            ),
            "bootstrap-pos-destination-p2" => (
                "pos-destination-p2-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p2-strong",
                    HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                    Scenario::StrongSeparation,
                    0x5200_u64.wrapping_add(args.seed),
                )?,
                tandem_interactions,
                "pos-destination-p2-discovery",
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            ),
            "bootstrap-fimix-p23" => (
                "fimix-p23-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p23-strong",
                    HeterogeneityInteractionProfileV2::P23AllCurrent,
                    Scenario::StrongSeparation,
                    0x5230_u64.wrapping_add(args.seed),
                )?,
                tandem_interactions,
                "pos-destination-p23-discovery",
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            ),
            "bootstrap-pos-destination-p23" => (
                "pos-destination-p23-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p23-strong",
                    HeterogeneityInteractionProfileV2::P23AllCurrent,
                    Scenario::StrongSeparation,
                    0x5230_u64.wrapping_add(args.seed),
                )?,
                tandem_interactions,
                "pos-destination-p23-discovery",
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            ),
            "bootstrap-pos-common-metric-failure" => (
                "pos-p2-common-metric-failure-fixed-k-bootstrap",
                make_fixture(
                    "heterogeneity-p2-common-metric-failure",
                    HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                    Scenario::CommonMetricFailure,
                    0x4641_494c_u64.wrapping_add(args.seed),
                )?,
                tandem_interactions,
                "pos-common-metric-failure-discovery",
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            ),
            _ => {
                return Err(invalid(format!(
                    "{shard_id} is not one of the seven retained bootstrap scientific cells"
                )));
            }
        };
    let discovery_identity =
        dependency_discovery_identity(dependencies, dependency_id, selected_algorithm, 2)?;
    let config = inference_config(
        &fixture,
        args.seed,
        vec![2],
        algorithms,
        discovery_identity,
        selected_algorithm,
        2,
        common_metric,
    );
    Ok((cell_id.into(), fixture, config))
}

fn compile_bootstrap_cell(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
) -> Result<CompiledBootstrapCell, DynError> {
    let (cell_id, fixture, config) = bootstrap_cell_definition(args, shard_id, dependencies)?;
    let mut recipe = fixture.recipe.clone();
    let mut model = fixture.model.clone();
    recipe.settings.workers =
        metamorphic::configured_workers_v1(recipe.settings.workers).map_err(invalid)?;
    metamorphic::transform_model_declaration_order_v1(&mut model);
    stage_additive_multimod_recipe(&mut recipe, AnalysisMethod::Predict);
    recipe.pls_heterogeneity = Some(config.clone());
    finalize_recipe(&mut recipe, &model)?;
    let artifact = prepare_multimod_recipe_v1(
        &fixture.dataset,
        &recipe,
        &model,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    Ok(CompiledBootstrapCell {
        cell_id,
        fixture,
        config,
        recipe,
        model,
        artifact,
    })
}

fn validate_prepared_envelope(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
    cell: &CompiledBootstrapCell,
    envelope: &BootstrapPreparedEnvelopeV1,
) -> Result<(), DynError> {
    if envelope.schema_version != 1
        || envelope.suite_id != BOOTSTRAP_PREPARED_SUITE_ID
        || envelope.producer_suite_id != SUITE_ID
        || envelope.scientific_shard_id != shard_id
        || envelope.scale != scale_id(args.scale)
        || envelope.campaign_seed != args.seed
        || envelope.metamorphism != metamorphic::metamorphism_v1()
        || envelope.sign_columns != sign_columns_identity()?
        || envelope.workers != metamorphic::configured_workers_v1(1).map_err(invalid)?
        || envelope.fixture_observations != fixture_observations()
        || envelope.multiclass_point_fixture_plan != multiclass_point_fixture_plan()
        || envelope.dependency_shard_ids != bootstrap_dependency_ids(dependencies)?
        || envelope.cell_id != cell.cell_id
        || envelope.chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
        || envelope.requested_replicates != QUALIFICATION_BOOTSTRAP_DRAWS
        || envelope
            .execution
            .reference
            .orchestrator_plan
            .requested_replicates
            != QUALIFICATION_BOOTSTRAP_DRAWS
    {
        return Err(invalid(format!(
            "prepared bootstrap execution identity is invalid for {shard_id}"
        )));
    }
    Ok(())
}

fn expected_owned_bootstrap_records(chunk_index: u32, chunk_count: u32) -> usize {
    (0..QUALIFICATION_BOOTSTRAP_DRAWS)
        .filter(|index| index % chunk_count == chunk_index)
        .count()
}

fn validate_cache_envelope(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
    cell: &CompiledBootstrapCell,
    prepared: &BootstrapPreparedEnvelopeV1,
    envelope: &BootstrapCacheEnvelopeV1,
) -> Result<(), DynError> {
    let expected_records =
        expected_owned_bootstrap_records(envelope.chunk_index, envelope.chunk_count);
    let indices_are_owned = envelope.cache.records.iter().all(|record| {
        record.index < QUALIFICATION_BOOTSTRAP_DRAWS
            && record.index % envelope.chunk_count == envelope.chunk_index
    });
    if envelope.schema_version != 1
        || envelope.suite_id != BOOTSTRAP_CACHE_SUITE_ID
        || envelope.producer_suite_id != SUITE_ID
        || envelope.scientific_shard_id != shard_id
        || envelope.scale != scale_id(args.scale)
        || envelope.campaign_seed != args.seed
        || envelope.metamorphism != metamorphic::metamorphism_v1()
        || envelope.sign_columns != sign_columns_identity()?
        || envelope.workers != metamorphic::configured_workers_v1(1).map_err(invalid)?
        || envelope.fixture_observations != fixture_observations()
        || envelope.multiclass_point_fixture_plan != multiclass_point_fixture_plan()
        || envelope.dependency_shard_ids != bootstrap_dependency_ids(dependencies)?
        || envelope.cell_id != cell.cell_id
        || envelope.prepared_execution_identity_sha256
            != prepared.execution.execution_identity_sha256
        || envelope.chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
        || envelope.chunk_index >= envelope.chunk_count
        || envelope.requested_replicates != QUALIFICATION_BOOTSTRAP_DRAWS
        || envelope.expected_record_count != expected_records
        || envelope.record_count != envelope.cache.records.len()
        || envelope.record_count > expected_records
        || envelope.cache.shard.shard_index != envelope.chunk_index
        || envelope.cache.shard.shard_count != envelope.chunk_count
        || !indices_are_owned
        || envelope.completed
            != (!envelope.cache.cancelled && envelope.record_count == expected_records)
        || (!envelope.cache.cancelled && envelope.record_count != expected_records)
    {
        return Err(invalid(format!(
            "bootstrap cache envelope is invalid for {shard_id} chunk {}",
            envelope.chunk_index
        )));
    }
    Ok(())
}

fn indexed_shard(shard_id: &str, prefix: &str, count: usize) -> Option<usize> {
    shard_id
        .strip_prefix(prefix)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|index| *index < count)
}

fn run_sentinel(args: &Arguments) -> Result<Value, DynError> {
    let fixture = make_fixture_with_classes_and_observations(
        "heterogeneity-fast-root-sentinel",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::StrongSeparation,
        0x5345_4e54_494e_454c_u64.wrapping_add(args.seed),
        2,
        80,
    )?;
    let mut config = discovery_config(
        &fixture,
        args.seed,
        vec![2],
        vec![HeterogeneityAlgorithmV2::FimixPlsV2],
    );
    config.fimix.max_iterations = 500;
    let (_, engine_probe) = run_config("heterogeneity-fast-root-sentinel", &fixture, config)?;
    Ok(json!({
        "header": shard_report_header(args)?,
        "label_alignment_decision_matrix": label_alignment_decision_matrix()?,
        "engine_probe": engine_probe,
        "claim": "fail_fast_pipeline_sentinel_not_scientific_acceptance_evidence",
    }))
}

fn shard_payload(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
) -> Result<Value, DynError> {
    let recovery_count = match args.scale {
        Scale::Development => 2,
        Scale::Qualification => 5,
    };
    let scenario_count = match args.scale {
        Scale::Development => 2,
        Scale::Qualification => 5,
    };
    let power_count = match args.scale {
        Scale::Development => 2,
        Scale::Qualification => 10,
    };
    if shard_id == "sentinel" {
        return Ok(json!({ "kind": "sentinel", "value": run_sentinel(args)? }));
    }
    if let Some(index) = indexed_shard(shard_id, "fimix-recovery-", recovery_count) {
        let seed = args.seed.wrapping_add(index as u64);
        let fixture = make_fixture(
            &if index == 0 {
                "heterogeneity-p0-strong".to_owned()
            } else {
                format!("heterogeneity-p0-strong-data-seed-{seed}")
            },
            HeterogeneityInteractionProfileV2::P0Structural,
            Scenario::StrongSeparation,
            0x5000_u64.wrapping_add(seed),
        )?;
        let (_, value) = run_required_discovery(
            &format!("fimix-p0-strong-seed-{seed}"),
            &fixture,
            seed,
            vec![2],
            vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            &[(HeterogeneityAlgorithmV2::FimixPlsV2, 2)],
        )?;
        return Ok(json!({ "kind": "fimix_recovery", "index": index, "value": value }));
    }
    for (prefix, cell_prefix, kind, scenario, count, data_seed_domain) in [
        (
            "fimix-power-",
            "fimix-p0-power-moderate",
            "fimix_power",
            Scenario::PowerModerate,
            power_count,
            0x504f_5745_5200_u64,
        ),
        (
            "fimix-overlap-",
            "fimix-p0-overlap",
            "fimix_overlap",
            Scenario::Overlap,
            scenario_count,
            0x4f56_4552_0000_u64,
        ),
        (
            "fimix-imbalance-",
            "fimix-p0-imbalanced",
            "fimix_imbalance",
            Scenario::Imbalanced,
            scenario_count,
            0x494d_4241_0000_u64,
        ),
        (
            "fimix-nonnormal-",
            "fimix-p0-nonnormal",
            "fimix_nonnormal",
            Scenario::NonNormal,
            scenario_count,
            0x4e4f_4e4e_0000_u64,
        ),
    ] {
        if let Some(index) = indexed_shard(shard_id, prefix, count) {
            let seed = args.seed.wrapping_add(index as u64);
            let fixture = make_fixture(
                &format!("{cell_prefix}-data-seed-{seed}"),
                HeterogeneityInteractionProfileV2::P0Structural,
                scenario,
                data_seed_domain.wrapping_add(seed),
            )?;
            let (_, value) = run_discovery(
                &format!("{cell_prefix}-seed-{seed}"),
                &fixture,
                seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            )?;
            return Ok(json!({ "kind": kind, "index": index, "value": value }));
        }
    }
    let tandem_p0 = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosPublishedV2,
    ];
    let tandem_interactions = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
    ];
    match shard_id {
        "fimix-candidate-k" => {
            let fixture = make_fixture(
                "heterogeneity-p0-strong",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::StrongSeparation,
                0x5000_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_discovery(
                "fimix-p0-candidate-k-2-through-5",
                &fixture,
                args.seed,
                vec![2, 3, 4, 5],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            )?;
            Ok(json!({ "kind": "fimix_candidate_k", "value": value }))
        }
        "fimix-homogeneous-null" => {
            let fixture = make_fixture(
                "heterogeneity-p0-homogeneous-null",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::HomogeneousNull,
                0x4e55_4c4c_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_discovery(
                "fimix-p0-homogeneous-null",
                &fixture,
                args.seed,
                vec![2, 3],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
            )?;
            Ok(json!({ "kind": "fimix_homogeneous_null", "value": value }))
        }
        "pos-published-p0-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p0-strong",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::StrongSeparation,
                0x5000_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_required_discovery(
                "pos-published-p0-discovery",
                &fixture,
                args.seed,
                vec![2],
                tandem_p0,
                &[
                    (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
                    (HeterogeneityAlgorithmV2::PlsPosPublishedV2, 2),
                ],
            )?;
            Ok(json!({ "kind": "pos_published_p0_discovery", "value": value }))
        }
        "pos-destination-p2-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p2-strong",
                HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                Scenario::StrongSeparation,
                0x5200_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_required_discovery(
                "heterogeneity-p2-tandem-discovery",
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                &[
                    (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
                    (
                        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                        2,
                    ),
                ],
            )?;
            Ok(json!({ "kind": "pos_destination_p2_discovery", "value": value }))
        }
        "pos-destination-p23-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p23-strong",
                HeterogeneityInteractionProfileV2::P23AllCurrent,
                Scenario::StrongSeparation,
                0x5230_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_required_discovery(
                "heterogeneity-p23-tandem-discovery",
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                &[
                    (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
                    (
                        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                        2,
                    ),
                ],
            )?;
            Ok(json!({ "kind": "pos_destination_p23_discovery", "value": value }))
        }
        "pos-common-metric-failure-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p2-common-metric-failure",
                HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                Scenario::CommonMetricFailure,
                0x4641_494c_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_required_discovery(
                "pos-p2-common-metric-failure-discovery",
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                &[
                    (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
                    (
                        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                        2,
                    ),
                ],
            )?;
            Ok(json!({ "kind": "pos_common_metric_failure_discovery", "value": value }))
        }
        "pos-homogeneous-null-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p0-homogeneous-null",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::HomogeneousNull,
                0x4e55_4c4c_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_discovery(
                "pos-published-p0-homogeneous-null",
                &fixture,
                args.seed,
                vec![2],
                tandem_p0,
            )?;
            Ok(json!({ "kind": "pos_homogeneous_null_discovery", "value": value }))
        }
        "pos-overlap-discovery" => {
            let fixture = make_fixture(
                "heterogeneity-p0-overlap-pos",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::Overlap,
                0x4f56_4552_u64.wrapping_add(args.seed),
            )?;
            let (_, value) = run_discovery(
                "pos-published-p0-overlap",
                &fixture,
                args.seed,
                vec![2],
                tandem_p0,
            )?;
            Ok(json!({ "kind": "pos_overlap_discovery", "value": value }))
        }
        "boundary-rank" | "boundary-variance" | "boundary-rare" => {
            let (kind, fixture_id, profile, scenario, data_seed, cell_id) = match shard_id {
                "boundary-rank" => (
                    "boundary_rank",
                    "heterogeneity-rank-deficient",
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::RankDeficient,
                    0x5241_4e4b_u64,
                    "fimix-rank-deficient",
                ),
                "boundary-variance" => (
                    "boundary_variance",
                    "heterogeneity-variance-collapse",
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::VarianceCollapse,
                    0x5641_5249_u64,
                    "fimix-variance-collapse",
                ),
                _ => (
                    "boundary_rare",
                    "heterogeneity-rare-class",
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::RareClass,
                    0x5241_5245_u64,
                    "fimix-rare-class",
                ),
            };
            let fixture = make_fixture(
                fixture_id,
                profile,
                scenario,
                data_seed.wrapping_add(args.seed),
            )?;
            Ok(json!({
                "kind": kind,
                "value": attempted_boundary(cell_id, &fixture, args.seed),
            }))
        }
        _ if shard_id.starts_with("pos-published-k") && shard_id.ends_with("-discovery") => {
            let selected_k = shard_id
                .strip_prefix("pos-published-k")
                .and_then(|value| value.strip_suffix("-discovery"))
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|value| (3..=5).contains(value))
                .ok_or_else(|| invalid(format!("invalid POS K discovery shard {shard_id}")))?;
            let fixture = make_multiclass_point_fixture(
                &format!("heterogeneity-p0-strong-k{selected_k}"),
                0x4b00_0000_u64
                    .wrapping_add(u64::from(selected_k) << 16)
                    .wrapping_add(args.seed),
                selected_k,
            )?;
            let (_, value) = run_required_discovery(
                &format!("pos-published-p0-k{selected_k}-discovery"),
                &fixture,
                args.seed,
                vec![selected_k],
                vec![HeterogeneityAlgorithmV2::PlsPosPublishedV2],
                &[(HeterogeneityAlgorithmV2::PlsPosPublishedV2, selected_k)],
            )?;
            Ok(json!({
                "kind": "pos_published_k_discovery",
                "index": selected_k,
                "value": value,
            }))
        }
        "bootstrap-fimix-p0" => {
            let fixture = make_fixture(
                "heterogeneity-p0-strong",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::StrongSeparation,
                0x5000_u64.wrapping_add(args.seed),
            )?;
            let value = run_inference(
                "fimix-p0-fixed-k-bootstrap",
                &fixture,
                args.seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
                dependency_discovery_identity(
                    dependencies,
                    "fimix-recovery-00",
                    HeterogeneityAlgorithmV2::FimixPlsV2,
                    2,
                )?,
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            )?;
            Ok(json!({ "kind": "bootstrap", "value": value }))
        }
        "bootstrap-pos-published-p0" => {
            let fixture = make_fixture(
                "heterogeneity-p0-strong",
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::StrongSeparation,
                0x5000_u64.wrapping_add(args.seed),
            )?;
            let value = run_inference(
                "pos-published-p0-fixed-k-bootstrap",
                &fixture,
                args.seed,
                vec![2],
                tandem_p0,
                dependency_discovery_identity(
                    dependencies,
                    "pos-published-p0-discovery",
                    HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                    2,
                )?,
                HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                true,
            )?;
            Ok(json!({ "kind": "bootstrap", "value": value }))
        }
        "bootstrap-fimix-p2" | "bootstrap-pos-destination-p2" => {
            let fixture = make_fixture(
                "heterogeneity-p2-strong",
                HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                Scenario::StrongSeparation,
                0x5200_u64.wrapping_add(args.seed),
            )?;
            let (cell_id, algorithm, common_metric) = if shard_id == "bootstrap-fimix-p2" {
                (
                    "fimix-p2-fixed-k-bootstrap",
                    HeterogeneityAlgorithmV2::FimixPlsV2,
                    false,
                )
            } else {
                (
                    "pos-destination-p2-fixed-k-bootstrap",
                    HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                    true,
                )
            };
            let value = run_inference(
                cell_id,
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                dependency_discovery_identity(
                    dependencies,
                    "pos-destination-p2-discovery",
                    algorithm,
                    2,
                )?,
                algorithm,
                common_metric,
            )?;
            Ok(json!({ "kind": "bootstrap", "value": value }))
        }
        "bootstrap-fimix-p23" | "bootstrap-pos-destination-p23" => {
            let fixture = make_fixture(
                "heterogeneity-p23-strong",
                HeterogeneityInteractionProfileV2::P23AllCurrent,
                Scenario::StrongSeparation,
                0x5230_u64.wrapping_add(args.seed),
            )?;
            let (cell_id, algorithm, common_metric) = if shard_id == "bootstrap-fimix-p23" {
                (
                    "fimix-p23-fixed-k-bootstrap",
                    HeterogeneityAlgorithmV2::FimixPlsV2,
                    false,
                )
            } else {
                (
                    "pos-destination-p23-fixed-k-bootstrap",
                    HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                    true,
                )
            };
            let value = run_inference(
                cell_id,
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                dependency_discovery_identity(
                    dependencies,
                    "pos-destination-p23-discovery",
                    algorithm,
                    2,
                )?,
                algorithm,
                common_metric,
            )?;
            Ok(json!({ "kind": "bootstrap", "value": value }))
        }
        "bootstrap-pos-common-metric-failure" => {
            let fixture = make_fixture(
                "heterogeneity-p2-common-metric-failure",
                HeterogeneityInteractionProfileV2::P2MultiTwoWay,
                Scenario::CommonMetricFailure,
                0x4641_494c_u64.wrapping_add(args.seed),
            )?;
            let value = run_inference(
                "pos-p2-common-metric-failure-fixed-k-bootstrap",
                &fixture,
                args.seed,
                vec![2],
                tandem_interactions,
                dependency_discovery_identity(
                    dependencies,
                    "pos-common-metric-failure-discovery",
                    HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                    2,
                )?,
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?;
            Ok(json!({ "kind": "bootstrap", "value": value }))
        }
        _ => Err(invalid(format!(
            "unknown heterogeneity qualification shard {shard_id}"
        ))),
    }
}

fn run_shard(args: &Arguments, shard_id: &str) -> Result<(), DynError> {
    let dependencies = dependency_envelopes(args, shard_id)?;
    let payload = shard_payload(args, shard_id, &dependencies)?;
    let dependency_ids = dependencies
        .iter()
        .filter_map(|value| value["shard_id"].as_str())
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_SUITE_ID,
        "producer_suite_id": SUITE_ID,
        "shard_id": shard_id,
        "scale": scale_id(args.scale),
        "campaign_seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
        "multiclass_point_fixture_plan": multiclass_point_fixture_plan(),
        "dependency_shard_ids": dependency_ids,
        "payload": payload,
    });
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn bootstrap_prepared_envelope(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
    cell: &CompiledBootstrapCell,
    execution: PreparedRawHeterogeneityBootstrapExecutionV2,
) -> Result<BootstrapPreparedEnvelopeV1, DynError> {
    let envelope = BootstrapPreparedEnvelopeV1 {
        schema_version: 1,
        suite_id: BOOTSTRAP_PREPARED_SUITE_ID.into(),
        producer_suite_id: SUITE_ID.into(),
        scientific_shard_id: shard_id.into(),
        scale: scale_id(args.scale).into(),
        campaign_seed: args.seed,
        metamorphism: metamorphic::metamorphism_v1().into(),
        sign_columns: sign_columns_identity()?,
        workers: metamorphic::configured_workers_v1(1).map_err(invalid)?,
        fixture_observations: fixture_observations(),
        multiclass_point_fixture_plan: multiclass_point_fixture_plan(),
        dependency_shard_ids: bootstrap_dependency_ids(dependencies)?,
        cell_id: cell.cell_id.clone(),
        chunk_count: DEFAULT_BOOTSTRAP_CHUNK_COUNT,
        requested_replicates: QUALIFICATION_BOOTSTRAP_DRAWS,
        execution,
    };
    validate_prepared_envelope(args, shard_id, dependencies, cell, &envelope)?;
    Ok(envelope)
}

fn run_bootstrap_prepare(
    args: &Arguments,
    shard_id: &str,
    budget_seconds: u64,
) -> Result<(), DynError> {
    let started = Instant::now();
    let budget = Duration::from_secs(budget_seconds);
    let dependencies = dependency_envelopes(args, shard_id)?;
    let cell = compile_bootstrap_cell(args, shard_id, &dependencies)?;
    let execution = prepare_compiled_raw_pls_heterogeneity_bootstrap_v2(
        &cell.fixture.dataset,
        &cell.recipe,
        &cell.model,
        &cell.artifact,
        || started.elapsed() >= budget,
        |_| {},
    )?;
    let envelope = bootstrap_prepared_envelope(args, shard_id, &dependencies, &cell, execution)?;
    fs::write(&args.output, serde_json::to_vec_pretty(&envelope)?)?;
    Ok(())
}

fn read_prepared_envelope(
    args: &Arguments,
    shard_id: &str,
    dependencies: &[Value],
    cell: &CompiledBootstrapCell,
    path: &PathBuf,
) -> Result<BootstrapPreparedEnvelopeV1, DynError> {
    let envelope: BootstrapPreparedEnvelopeV1 = serde_json::from_slice(&fs::read(path)?)?;
    validate_prepared_envelope(args, shard_id, dependencies, cell, &envelope)?;
    Ok(envelope)
}

#[allow(clippy::too_many_arguments)]
fn run_bootstrap_chunk(
    args: &Arguments,
    shard_id: &str,
    prepared_execution: &PathBuf,
    resume_cache: Option<&PathBuf>,
    chunk_index: u32,
    chunk_count: u32,
    budget_seconds: u64,
) -> Result<(), DynError> {
    let started = Instant::now();
    let budget = Duration::from_secs(budget_seconds);
    let dependencies = dependency_envelopes(args, shard_id)?;
    let cell = compile_bootstrap_cell(args, shard_id, &dependencies)?;
    let prepared =
        read_prepared_envelope(args, shard_id, &dependencies, &cell, prepared_execution)?;
    let resume = resume_cache
        .map(|path| {
            let envelope: BootstrapCacheEnvelopeV1 = serde_json::from_slice(&fs::read(path)?)?;
            validate_cache_envelope(args, shard_id, &dependencies, &cell, &prepared, &envelope)?;
            if envelope.chunk_index != chunk_index || envelope.chunk_count != chunk_count {
                return Err(invalid(
                    "resume cache belongs to a different deterministic modulo chunk",
                ));
            }
            Ok(envelope.cache)
        })
        .transpose()?;
    let prior_record_count = resume.as_ref().map_or(0, |cache| cache.records.len());
    let cache = run_prepared_raw_pls_heterogeneity_bootstrap_shard_v2(
        &cell.fixture.dataset,
        &cell.recipe,
        &cell.model,
        &cell.artifact,
        &prepared.execution,
        MultiModShardSpecV1 {
            shard_index: chunk_index,
            shard_count: chunk_count,
        },
        resume,
        || started.elapsed() >= budget,
        |_| {},
    )?;
    let record_count = cache.records.len();
    let expected_record_count = expected_owned_bootstrap_records(chunk_index, chunk_count);
    if record_count <= prior_record_count {
        return Err(invalid(format!(
            "bootstrap chunk {chunk_index}/{chunk_count} made zero verified record progress"
        )));
    }
    let completed = !cache.cancelled && record_count == expected_record_count;
    let envelope = BootstrapCacheEnvelopeV1 {
        schema_version: 1,
        suite_id: BOOTSTRAP_CACHE_SUITE_ID.into(),
        producer_suite_id: SUITE_ID.into(),
        scientific_shard_id: shard_id.into(),
        scale: scale_id(args.scale).into(),
        campaign_seed: args.seed,
        metamorphism: metamorphic::metamorphism_v1().into(),
        sign_columns: sign_columns_identity()?,
        workers: metamorphic::configured_workers_v1(1).map_err(invalid)?,
        fixture_observations: fixture_observations(),
        multiclass_point_fixture_plan: multiclass_point_fixture_plan(),
        dependency_shard_ids: bootstrap_dependency_ids(&dependencies)?,
        cell_id: cell.cell_id.clone(),
        prepared_execution_identity_sha256: prepared.execution.execution_identity_sha256.clone(),
        chunk_index,
        chunk_count,
        requested_replicates: QUALIFICATION_BOOTSTRAP_DRAWS,
        prior_record_count,
        record_count,
        expected_record_count,
        completed,
        cache,
    };
    validate_cache_envelope(args, shard_id, &dependencies, &cell, &prepared, &envelope)?;
    fs::write(&args.output, serde_json::to_vec_pretty(&envelope)?)?;
    Ok(())
}

fn bootstrap_scientific_value(
    cell: &CompiledBootstrapCell,
    output: &MultiModRunOutputV1,
) -> Result<Value, DynError> {
    let MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(analysis) = &output.result else {
        return Err(invalid(
            "resumable heterogeneity finalizer returned the wrong result family",
        ));
    };
    Ok(json!({
        "cell_id": cell.cell_id,
        "fixture_id": cell.fixture.fixture_id,
        "scenario": format!("{:?}", cell.fixture.scenario).to_lowercase(),
        "profile": cell.fixture.profile,
        "dataset_rows": cell.fixture.dataset.batch.num_rows(),
        "dataset_fingerprint": cell.fixture.dataset.fingerprint.0,
        "config": cell.config,
        "compiler_receipt": output.compilation_receipt,
        "compiled_plan": cell.artifact.plan(),
        "sem_model_authority": cell.model,
        "analysis": analysis,
        "evidence": summarize_evidence(&output.evidence),
        "true_classes": cell.fixture.true_classes,
    }))
}

fn run_bootstrap_finalize(
    args: &Arguments,
    shard_id: &str,
    prepared_execution: &PathBuf,
    cache_inventory: &PathBuf,
) -> Result<(), DynError> {
    let dependencies = dependency_envelopes(args, shard_id)?;
    let cell = compile_bootstrap_cell(args, shard_id, &dependencies)?;
    let prepared =
        read_prepared_envelope(args, shard_id, &dependencies, &cell, prepared_execution)?;
    let inventory: BootstrapCacheInventoryV1 = serde_json::from_slice(&fs::read(cache_inventory)?)?;
    let inventory_directory = cache_inventory
        .parent()
        .ok_or_else(|| invalid("bootstrap cache inventory has no parent directory"))?;
    let dependency_ids = bootstrap_dependency_ids(&dependencies)?;
    if inventory.schema_version != SCHEMA_VERSION
        || inventory.suite_id != BOOTSTRAP_CACHE_INVENTORY_SUITE_ID
        || inventory.producer_suite_id != SUITE_ID
        || inventory.scientific_shard_id != shard_id
        || inventory.scale != scale_id(args.scale)
        || inventory.campaign_seed != args.seed
        || inventory.metamorphism != metamorphic::metamorphism_v1()
        || inventory.sign_columns != sign_columns_identity()?
        || inventory.workers != metamorphic::configured_workers_v1(1).map_err(invalid)?
        || inventory.fixture_observations != fixture_observations()
        || inventory.multiclass_point_fixture_plan != multiclass_point_fixture_plan()
        || inventory.dependency_shard_ids != dependency_ids
        || inventory.cell_id != cell.cell_id
        || !is_lower_hex(&inventory.plan_sha256, 64)
        || !is_lower_hex(&inventory.producer_executable_sha256, 64)
        || !is_lower_hex(&inventory.source_commit, 40)
        || inventory.chunk_count != DEFAULT_BOOTSTRAP_CHUNK_COUNT
        || inventory.requested_replicates != QUALIFICATION_BOOTSTRAP_DRAWS
        || inventory.execution_identity_sha256 != prepared.execution.execution_identity_sha256
    {
        return Err(invalid(
            "bootstrap cache inventory scientific identity is invalid",
        ));
    }
    if sha256_file(&env::current_exe()?)? != inventory.producer_executable_sha256 {
        return Err(invalid(
            "bootstrap cache inventory executable identity is stale or mixed",
        ));
    }

    let inventory_prepared = inventory_file_with_digest(
        inventory_directory,
        &inventory.prepared_execution_file,
        &inventory.prepared_execution_sha256,
        "prepared execution",
    )?;
    if fs::canonicalize(&inventory_prepared)? != fs::canonicalize(prepared_execution)? {
        return Err(invalid(
            "bootstrap cache inventory names a different prepared execution",
        ));
    }
    let prepared_receipt_path = inventory_file_with_digest(
        inventory_directory,
        &inventory.prepared_receipt_file,
        &inventory.prepared_receipt_sha256,
        "prepared receipt",
    )?;
    let prepared_pointer = validate_generation_pointer(
        inventory_directory,
        &inventory.prepared_pointer_file,
        &inventory.prepared_pointer_sha256,
        shard_id,
        "prepared",
        None,
        &inventory.prepared_execution_file,
        &inventory.prepared_execution_sha256,
        &inventory.prepared_receipt_file,
        &inventory.prepared_receipt_sha256,
    )?;
    validate_prepared_transport_receipt(&prepared_receipt_path, &prepared_pointer, &inventory)?;

    let mut dependency_paths = args
        .dependencies
        .iter()
        .map(|path| {
            let value: Value = serde_json::from_slice(&fs::read(path)?)?;
            let dependency_id = value["shard_id"]
                .as_str()
                .ok_or_else(|| invalid("bootstrap dependency result has no shard id"))?;
            Ok((dependency_id.to_owned(), path))
        })
        .collect::<Result<Vec<_>, DynError>>()?;
    dependency_paths.sort_by(|left, right| left.0.cmp(&right.0));
    let mut dependency_receipts = inventory.dependency_receipts.iter().collect::<Vec<_>>();
    dependency_receipts.sort_by(|left, right| left.shard_id.cmp(&right.shard_id));
    if dependency_receipts.len() != dependency_paths.len() {
        return Err(invalid(
            "bootstrap cache inventory dependency receipt inventory is incomplete",
        ));
    }
    for (receipt, (dependency_id, dependency_path)) in
        dependency_receipts.into_iter().zip(dependency_paths)
    {
        let receipt_path = dependency_path
            .parent()
            .ok_or_else(|| invalid("bootstrap dependency result has no parent directory"))?
            .join(format!("{dependency_id}.receipt.json"));
        if receipt.shard_id != dependency_id
            || !is_lower_hex(&receipt.receipt_sha256, 64)
            || !is_lower_hex(&receipt.result_sha256, 64)
            || sha256_file(dependency_path)? != receipt.result_sha256
            || sha256_file(&receipt_path)? != receipt.receipt_sha256
        {
            return Err(invalid(
                "bootstrap cache inventory dependency receipt identity is stale or mixed",
            ));
        }
    }

    if inventory.caches.len() != DEFAULT_BOOTSTRAP_CHUNK_COUNT as usize {
        return Err(invalid(
            "bootstrap cache inventory is missing, duplicate, or mixed",
        ));
    }
    let mut caches = Vec::with_capacity(inventory.caches.len());
    for (expected_chunk, entry) in inventory.caches.iter().enumerate() {
        if entry.chunk_index as usize != expected_chunk {
            return Err(invalid(
                "bootstrap cache inventory is missing, duplicate, or mixed",
            ));
        }
        let cache_path = inventory_file_with_digest(
            inventory_directory,
            &entry.cache_file,
            &entry.cache_sha256,
            "cache payload",
        )?;
        let cache_receipt_path = inventory_file_with_digest(
            inventory_directory,
            &entry.receipt_file,
            &entry.receipt_sha256,
            "cache receipt",
        )?;
        let cache_pointer = validate_generation_pointer(
            inventory_directory,
            &entry.pointer_file,
            &entry.pointer_sha256,
            shard_id,
            "cache",
            Some(entry.chunk_index),
            &entry.cache_file,
            &entry.cache_sha256,
            &entry.receipt_file,
            &entry.receipt_sha256,
        )?;
        validate_cache_transport_receipt(&cache_receipt_path, &cache_pointer, &inventory, entry)?;
        let envelope: BootstrapCacheEnvelopeV1 = serde_json::from_slice(&fs::read(cache_path)?)?;
        validate_cache_envelope(args, shard_id, &dependencies, &cell, &prepared, &envelope)?;
        if !envelope.completed
            || envelope.chunk_index != entry.chunk_index
            || envelope.record_count != entry.record_count
            || envelope.cache.shard.shard_identity_sha256 != entry.cache_shard_identity_sha256
        {
            return Err(invalid(format!(
                "bootstrap cache inventory chunk {} is incomplete or mixed",
                entry.chunk_index
            )));
        }
        caches.push(envelope.cache);
    }
    let output = finalize_prepared_raw_pls_heterogeneity_bootstrap_v2(
        &cell.fixture.dataset,
        &cell.recipe,
        &cell.model,
        &cell.artifact,
        &prepared.execution,
        caches,
        |_| {},
    )?;
    let payload = json!({
        "kind": "bootstrap",
        "value": bootstrap_scientific_value(&cell, &output)?,
    });
    let report = json!({
        "schema_version": SHARD_SCHEMA_VERSION,
        "suite_id": SHARD_SUITE_ID,
        "producer_suite_id": SUITE_ID,
        "shard_id": shard_id,
        "scale": scale_id(args.scale),
        "campaign_seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "sign_columns": sign_columns_identity()?,
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
        "multiclass_point_fixture_plan": multiclass_point_fixture_plan(),
        "dependency_shard_ids": bootstrap_dependency_ids(&dependencies)?,
        "payload": payload,
    });
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn run_monolithic(args: &Arguments) -> Result<(), DynError> {
    let p0 = make_fixture(
        "heterogeneity-p0-strong",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::StrongSeparation,
        0x5000 + args.seed,
    )?;
    let p2 = make_fixture(
        "heterogeneity-p2-strong",
        HeterogeneityInteractionProfileV2::P2MultiTwoWay,
        Scenario::StrongSeparation,
        0x5200 + args.seed,
    )?;
    let p23 = make_fixture(
        "heterogeneity-p23-strong",
        HeterogeneityInteractionProfileV2::P23AllCurrent,
        Scenario::StrongSeparation,
        0x5230 + args.seed,
    )?;
    let null = make_fixture(
        "heterogeneity-p0-homogeneous-null",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::HomogeneousNull,
        0x4e55_4c4c + args.seed,
    )?;
    let metric_failure = make_fixture(
        "heterogeneity-p2-common-metric-failure",
        HeterogeneityInteractionProfileV2::P2MultiTwoWay,
        Scenario::CommonMetricFailure,
        0x4641_494c + args.seed,
    )?;
    let rank = make_fixture(
        "heterogeneity-rank-deficient",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::RankDeficient,
        0x5241_4e4b + args.seed,
    )?;
    let variance = make_fixture(
        "heterogeneity-variance-collapse",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::VarianceCollapse,
        0x5641_5249 + args.seed,
    )?;
    let rare = make_fixture(
        "heterogeneity-rare-class",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::RareClass,
        0x5241_5245 + args.seed,
    )?;
    let pos_overlap = make_fixture(
        "heterogeneity-p0-overlap-pos",
        HeterogeneityInteractionProfileV2::P0Structural,
        Scenario::Overlap,
        0x4f56_4552 + args.seed,
    )?;

    let recovery_seeds = if metamorphic::compact_matrix_v1() {
        vec![args.seed]
    } else {
        match args.scale {
            Scale::Development => vec![args.seed, args.seed + 1],
            Scale::Qualification => vec![
                args.seed,
                args.seed + 1,
                args.seed + 2,
                args.seed + 3,
                args.seed + 4,
            ],
        }
    };
    let fimix_recovery = recovery_seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            let generated_fixture = if index == 0 {
                None
            } else {
                Some(make_fixture(
                    &format!("heterogeneity-p0-strong-data-seed-{seed}"),
                    HeterogeneityInteractionProfileV2::P0Structural,
                    Scenario::StrongSeparation,
                    0x5000 + *seed,
                )?)
            };
            let fixture = generated_fixture.as_ref().unwrap_or(&p0);
            run_required_discovery(
                &format!("fimix-p0-strong-seed-{seed}"),
                fixture,
                *seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
                &[(HeterogeneityAlgorithmV2::FimixPlsV2, 2)],
            )
            .map(|(_, value)| value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let scenario_seeds = if metamorphic::compact_matrix_v1() {
        vec![args.seed]
    } else {
        match args.scale {
            Scale::Development => vec![args.seed, args.seed + 1],
            Scale::Qualification => (args.seed..args.seed + 5).collect(),
        }
    };
    let power_seeds = if metamorphic::compact_matrix_v1() {
        scenario_seeds.clone()
    } else {
        match args.scale {
            Scale::Development => scenario_seeds.clone(),
            Scale::Qualification => (args.seed..args.seed + 10).collect(),
        }
    };
    let power_recovery = fimix_p0_simulation_series(
        "fimix-p0-power-moderate",
        Scenario::PowerModerate,
        &power_seeds,
        0x504f_5745_5200,
    )?;
    let overlap_recovery = fimix_p0_simulation_series(
        "fimix-p0-overlap",
        Scenario::Overlap,
        &scenario_seeds,
        0x4f56_4552_0000,
    )?;
    let imbalance_recovery = fimix_p0_simulation_series(
        "fimix-p0-imbalanced",
        Scenario::Imbalanced,
        &scenario_seeds,
        0x494d_4241_0000,
    )?;
    let nonnormal_recovery = fimix_p0_simulation_series(
        "fimix-p0-nonnormal",
        Scenario::NonNormal,
        &scenario_seeds,
        0x4e4f_4e4e_0000,
    )?;
    let (_, candidate_k_table) = run_discovery(
        "fimix-p0-candidate-k-2-through-5",
        &p0,
        args.seed,
        vec![2, 3, 4, 5],
        vec![HeterogeneityAlgorithmV2::FimixPlsV2],
    )?;
    let (_, homogeneous_null) = run_discovery(
        "fimix-p0-homogeneous-null",
        &null,
        args.seed,
        vec![2, 3],
        vec![HeterogeneityAlgorithmV2::FimixPlsV2],
    )?;

    let tandem_p0_algorithms = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosPublishedV2,
    ];
    let tandem_interaction_algorithms = vec![
        HeterogeneityAlgorithmV2::FimixPlsV2,
        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
    ];
    let (p0_tandem_identity, p0_tandem_discovery) = run_required_discovery(
        "pos-published-p0-discovery",
        &p0,
        args.seed,
        vec![2],
        tandem_p0_algorithms.clone(),
        &[
            (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
            (HeterogeneityAlgorithmV2::PlsPosPublishedV2, 2),
        ],
    )?;
    let (p2_identity, p2_discovery) = run_required_discovery(
        "heterogeneity-p2-tandem-discovery",
        &p2,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
        &[
            (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
            (
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                2,
            ),
        ],
    )?;
    let (p23_identity, p23_discovery) = run_required_discovery(
        "heterogeneity-p23-tandem-discovery",
        &p23,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
        &[
            (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
            (
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                2,
            ),
        ],
    )?;
    let (metric_failure_identity, metric_failure_discovery) = run_required_discovery(
        "pos-p2-common-metric-failure-discovery",
        &metric_failure,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
        &[
            (HeterogeneityAlgorithmV2::FimixPlsV2, 2),
            (
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                2,
            ),
        ],
    )?;
    let (_, pos_null_discovery) = run_discovery(
        "pos-published-p0-homogeneous-null",
        &null,
        args.seed,
        vec![2],
        tandem_p0_algorithms.clone(),
    )?;
    let (_, pos_overlap_discovery) = run_discovery(
        "pos-published-p0-overlap",
        &pos_overlap,
        args.seed,
        vec![2],
        tandem_p0_algorithms.clone(),
    )?;

    let mut pos_published_k3_through_k5_discovery = Vec::new();
    if args.scale == Scale::Qualification && !metamorphic::compact_matrix_v1() {
        for selected_k in 3_u8..=5 {
            let fixture = make_multiclass_point_fixture(
                &format!("heterogeneity-p0-strong-k{selected_k}"),
                0x4b00_0000_u64
                    .wrapping_add(u64::from(selected_k) << 16)
                    .wrapping_add(args.seed),
                selected_k,
            )?;
            let (_, discovery) = run_required_discovery(
                &format!("pos-published-p0-k{selected_k}-discovery"),
                &fixture,
                args.seed,
                vec![selected_k],
                vec![HeterogeneityAlgorithmV2::PlsPosPublishedV2],
                &[(HeterogeneityAlgorithmV2::PlsPosPublishedV2, selected_k)],
            )?;
            pos_published_k3_through_k5_discovery.push(discovery);
        }
    }

    let bootstrap_cells = if metamorphic::compact_matrix_v1() {
        vec![
            run_inference(
                "fimix-p0-fixed-k-bootstrap",
                &p0,
                args.seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
                fimix_recovery[0]["analysis"]["discovery_result_identity_sha256"]
                    .as_str()
                    .ok_or_else(|| invalid("FIMIX discovery identity is absent"))?
                    .into(),
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            )?,
            run_inference(
                "pos-destination-p2-fixed-k-bootstrap",
                &p2,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p2_identity.clone(),
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?,
            run_inference(
                "pos-destination-p23-fixed-k-bootstrap",
                &p23,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p23_identity.clone(),
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?,
        ]
    } else if args.scale == Scale::Qualification {
        vec![
            run_inference(
                "fimix-p0-fixed-k-bootstrap",
                &p0,
                args.seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
                fimix_recovery[0]["analysis"]["discovery_result_identity_sha256"]
                    .as_str()
                    .ok_or_else(|| invalid("FIMIX discovery identity is absent"))?
                    .into(),
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            )?,
            run_inference(
                "pos-published-p0-fixed-k-bootstrap",
                &p0,
                args.seed,
                vec![2],
                tandem_p0_algorithms.clone(),
                p0_tandem_identity,
                HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                true,
            )?,
            run_inference(
                "fimix-p2-fixed-k-bootstrap",
                &p2,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p2_identity.clone(),
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            )?,
            run_inference(
                "pos-destination-p2-fixed-k-bootstrap",
                &p2,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p2_identity,
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?,
            run_inference(
                "fimix-p23-fixed-k-bootstrap",
                &p23,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p23_identity.clone(),
                HeterogeneityAlgorithmV2::FimixPlsV2,
                false,
            )?,
            run_inference(
                "pos-destination-p23-fixed-k-bootstrap",
                &p23,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                p23_identity,
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?,
            run_inference(
                "pos-p2-common-metric-failure-fixed-k-bootstrap",
                &metric_failure,
                args.seed,
                vec![2],
                tandem_interaction_algorithms.clone(),
                metric_failure_identity,
                HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                true,
            )?,
        ]
    } else {
        Vec::new()
    };
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "suite_id": SUITE_ID,
        "scale": match args.scale { Scale::Development => "development", Scale::Qualification => "qualification" },
        "campaign_seed": args.seed,
        "seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
        "multiclass_point_fixture_plan": multiclass_point_fixture_plan(),
        "execution_contract": "public_recipe_v4_compiler_plus_raw_fimix_pos_runner",
        "qualification_claim": "raw_sut_facts_for_independent_comparison_only",
        "multistart_reproducibility_contract": {
            "schema_version": HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
            "partition_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2),
            "coefficient_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2),
            "posterior_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2),
            "parameter_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2),
            "fit_statistic_digest_domain": String::from_utf8_lossy(HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2),
            "partition_encoding": "u64_length_then_u64_labels_little_endian",
            "matrix_encoding": "u64_row_count_then_per_row_u64_length_and_f64_bits_little_endian",
            "completed_start_cardinality": "exactly_one_receipt_per_completed_start",
            "verification": "independent_exhaustive_alignment_and_tolerance_replay"
        },
        "label_alignment_decision_contract": "qpls_estimation::align_labels_exhaustive_v2",
        "label_alignment_decision_matrix": label_alignment_decision_matrix()?,
        "required_profile_ids": [
            "fimix.p0_structural.v2",
            "fimix.p2_multi_two_way.v2",
            "fimix.p23_all_current.v2",
            "pos.published.p0_structural.v2",
            "pos.destination_scored.p2_multi_two_way.v2",
            "pos.destination_scored.p23_all_current.v2",
            "pos.common_metric.p2_multi_two_way.v1",
            "pos.common_metric.p23_all_current.v1"
        ],
        "fimix": {
            "strong_separation_recovery": fimix_recovery,
            "power_recovery": power_recovery,
            "overlap_recovery": overlap_recovery,
            "imbalance_recovery": imbalance_recovery,
            "nonnormal_recovery": nonnormal_recovery,
            "simulation_acceptance": {
                "strong_median_ari_minimum": 0.80,
                "power_success_ari_minimum": 0.60,
                "power_success_rate_minimum": 0.80,
                "overlap_median_ari_minimum": 0.35,
                "imbalance_median_ari_minimum": 0.70,
                "nonnormal_median_ari_minimum": 0.70
            },
            "candidate_k_table": candidate_k_table,
            "homogeneous_null": homogeneous_null,
            "boundaries": {
                "rank_deficient": attempted_boundary("fimix-rank-deficient", &rank, args.seed),
                "variance_collapse": attempted_boundary("fimix-variance-collapse", &variance, args.seed),
                "rare_class": attempted_boundary("fimix-rare-class", &rare, args.seed),
                "separately_bound_kernel_test_filter": "fimix_failure_boundary_",
            },
        },
        "pos": {
            "published_p0_discovery": p0_tandem_discovery,
            "destination_p2_discovery": p2_discovery,
            "destination_p23_discovery": p23_discovery,
            "common_metric_failure_discovery": metric_failure_discovery,
            "homogeneous_null_discovery": pos_null_discovery,
            "overlap_discovery": pos_overlap_discovery,
            "published_p0_k3_through_k5_discovery": pos_published_k3_through_k5_discovery,
            "recovery_acceptance": {
                "published_p0_ari_minimum": 0.80,
                "destination_p2_ari_minimum": 0.75,
                "destination_p23_ari_minimum": 0.70,
                "overlap_ari_minimum": 0.35,
                "homogeneous_null_ari_maximum": 0.25
            }
        },
        "fixed_k_bootstrap": bootstrap_cells,
    });
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn main() -> Result<(), DynError> {
    let args = arguments()?;
    match &args.mode {
        ExecutionMode::Monolithic => run_monolithic(&args),
        ExecutionMode::Plan => {
            fs::write(
                &args.output,
                serde_json::to_vec_pretty(&shard_plan(&args)?)?,
            )?;
            Ok(())
        }
        ExecutionMode::Shard(shard_id) => run_shard(&args, shard_id),
        ExecutionMode::BootstrapPrepare {
            shard_id,
            budget_seconds,
        } => run_bootstrap_prepare(&args, shard_id, *budget_seconds),
        ExecutionMode::BootstrapChunk {
            shard_id,
            prepared_execution,
            resume_cache,
            chunk_index,
            chunk_count,
            budget_seconds,
        } => run_bootstrap_chunk(
            &args,
            shard_id,
            prepared_execution,
            resume_cache.as_ref(),
            *chunk_index,
            *chunk_count,
            *budget_seconds,
        ),
        ExecutionMode::BootstrapFinalize {
            shard_id,
            prepared_execution,
            cache_inventory,
        } => run_bootstrap_finalize(&args, shard_id, prepared_execution, cache_inventory),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn stable_discovery_fixture(algorithm: HeterogeneityAlgorithmV2, k: u8) -> Value {
        let algorithm_value = serde_json::to_value(algorithm).unwrap();
        let evidence_key = if algorithm == HeterogeneityAlgorithmV2::FimixPlsV2 {
            "fimix"
        } else {
            "pos"
        };
        let mut evidence = json!({
            "fimix": [],
            "pos": [],
        });
        evidence[evidence_key] = json!([{
            "k": k,
            "result": {
                "method_version": expected_heterogeneity_method_version(algorithm),
            },
        }]);
        json!({
            "analysis": {
                "candidates": [{
                    "method": {
                        "kind": "segmentation",
                        "algorithm": algorithm_value,
                    },
                    "k": k,
                    "state": "converged_stable",
                    "converged_starts": 10,
                    "stable_starts": 2,
                    "blockers": [],
                }],
                "discovery_result_identity_sha256": "a".repeat(64),
            },
            "evidence": evidence,
        })
    }

    #[test]
    fn discovery_dependency_requires_exact_stable_candidate_and_point_evidence() {
        let algorithm = HeterogeneityAlgorithmV2::PlsPosPublishedV2;
        let fixture = stable_discovery_fixture(algorithm, 3);
        assert_eq!(
            stable_discovery_identity(&fixture, "fixture", algorithm, 3).unwrap(),
            "a".repeat(64)
        );

        for mutation in ["state", "starts", "blocker", "evidence", "duplicate"] {
            let mut altered = fixture.clone();
            match mutation {
                "state" => altered["analysis"]["candidates"][0]["state"] = json!("unstable"),
                "starts" => altered["analysis"]["candidates"][0]["stable_starts"] = json!(1),
                "blocker" => altered["analysis"]["candidates"][0]["blockers"] = json!(["unstable"]),
                "evidence" => altered["evidence"]["pos"] = json!([]),
                "duplicate" => {
                    let duplicate = altered["analysis"]["candidates"][0].clone();
                    altered["analysis"]["candidates"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                }
                _ => unreachable!(),
            }
            assert!(
                stable_discovery_identity(&altered, "fixture", algorithm, 3).is_err(),
                "mutation {mutation} must fail closed"
            );
        }
    }

    #[test]
    fn multiclass_strong_fixture_has_distinct_multidimensional_structural_equations() {
        for classes in 3..=5 {
            let equations = (0..classes)
                .map(|class| strong_multiclass_p0_equation(class, classes))
                .collect::<Vec<_>>();
            assert!(equations.iter().all(|(coefficients, intercept)| {
                coefficients.iter().all(|value| value.is_finite())
                    && intercept.is_finite()
                    && coefficients.iter().any(|value| value.abs() >= 2.0)
            }));
            assert_eq!(
                equations
                    .iter()
                    .map(|(coefficients, intercept)| (
                        coefficients.map(f64::to_bits),
                        intercept.to_bits()
                    ))
                    .collect::<BTreeSet<_>>()
                    .len(),
                classes
            );
            assert!(
                equations
                    .iter()
                    .any(|(coefficients, _)| { coefficients.iter().any(|value| *value < 0.0) })
            );
            assert!(
                equations
                    .iter()
                    .any(|(coefficients, _)| { coefficients.iter().any(|value| *value > 0.0) })
            );
        }
    }

    #[test]
    fn lean_multiclass_point_plan_keeps_discovery_and_seven_bootstrap_profiles_distinct() {
        let plan = multiclass_point_fixture_plan();
        assert_eq!(plan["schema_version"], 1);
        assert_eq!(plan["plan_id"], MULTICLASS_POINT_FIXTURE_PLAN_ID);
        assert_eq!(plan["selected_k"], json!([3, 4, 5]));
        assert_eq!(plan["observations_per_fixture"], 120);
        assert_eq!(plan["bootstrap_evidence"], "not_requested");

        let specs = qualification_shard_specs(Scale::Qualification);
        let shard_ids = specs
            .iter()
            .filter_map(|row| row["shard_id"].as_str())
            .collect::<BTreeSet<_>>();
        for selected_k in 3..=5 {
            assert!(shard_ids.contains(format!("pos-published-k{selected_k}-discovery").as_str()));
            assert!(!shard_ids.contains(format!("pos-published-k{selected_k}-bootstrap").as_str()));
        }
        assert_eq!(
            specs
                .iter()
                .filter(|row| row["resource_class"] == "bootstrap")
                .count(),
            7
        );
    }

    #[test]
    fn multiclass_point_balance_requires_exact_n120_equal_allocation() {
        for classes in 3..=5 {
            let balanced = (0..MULTICLASS_POINT_OBSERVATIONS)
                .map(|row| row % classes)
                .collect::<Vec<_>>();
            validate_multiclass_point_balance("balanced", classes, &balanced).unwrap();

            let mut altered = balanced;
            altered[0] = 1;
            assert!(validate_multiclass_point_balance("altered", classes, &altered).is_err());
        }
        assert!(validate_multiclass_point_balance("short", 3, &[0, 1, 2]).is_err());
    }
}
