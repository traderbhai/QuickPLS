use chrono::{TimeZone, Utc};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, AnalysisSettings, Construct, MeasurementMode, ModelSpec,
    PROJECT_SCHEMA_VERSION, StructuralPath,
};
use qpls_data::{ImportOptions, import_delimited_bytes};
use qpls_estimation::{PLS_METHOD_VERSION, estimate_pls};
use qpls_resampling::{RESAMPLING_METHOD_VERSION, bootstrap_pls};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};
use uuid::Uuid;

const HARNESS_VERSION: &str = "qpls_monte_carlo_v1";
const MASTER_SEED: u64 = 20_260_718_041;
const ALPHA: f64 = 0.05;

#[derive(Clone, Copy)]
struct Configuration {
    mode: &'static str,
    simulations: usize,
    simulation_offset: usize,
    sample_size: usize,
    bootstrap_replicates: u32,
    studentized_inner_replicates: u32,
    workers: usize,
    qualification: bool,
}

#[derive(Clone, Copy)]
enum ErrorDistribution {
    Normal,
    StandardizedT3,
}

impl ErrorDistribution {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::StandardizedT3 => "standardized_t3",
        }
    }
}

#[derive(Clone, Copy)]
struct ScenarioSpec {
    name: &'static str,
    beta: f64,
    error_distribution: ErrorDistribution,
    seed_domain: u64,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    harness_version: &'static str,
    engine_versions: EngineVersions,
    mode: &'static str,
    configuration: ConfigurationReport,
    dgp: DgpReport,
    scenarios: Vec<ScenarioSummary>,
    qualification: QualificationSummary,
    elapsed_seconds: f64,
}

#[derive(Serialize)]
struct EngineVersions {
    pls: &'static str,
    resampling: &'static str,
}

#[derive(Serialize)]
struct ConfigurationReport {
    simulations_per_scenario: usize,
    simulation_offset: usize,
    sample_size: usize,
    bootstrap_replicates: u32,
    studentized_inner_replicates: u32,
    workers: usize,
    confidence_level: f64,
    master_seed: u64,
}

#[derive(Serialize)]
struct DgpReport {
    description: &'static str,
    variables: &'static str,
    evaluated_path: &'static str,
    alternatives: Vec<f64>,
}

#[derive(Serialize)]
struct ScenarioSummary {
    name: String,
    error_distribution: &'static str,
    true_path: f64,
    requested_simulations: usize,
    completed_simulations: usize,
    failed_simulations: usize,
    mean_estimate: Option<f64>,
    bias: Option<f64>,
    mean_usable_bootstrap_rate: Option<f64>,
    percentile: IntervalSummary,
    bca: IntervalSummary,
    studentized: IntervalSummary,
    normal_reference_available: usize,
    normal_reference_type_i_rate: Option<f64>,
    failures: Vec<String>,
}

#[derive(Serialize)]
struct IntervalSummary {
    available: usize,
    coverage_rate: Option<f64>,
    exclusion_of_zero_rate: Option<f64>,
}

#[derive(Serialize)]
struct QualificationSummary {
    evaluated: bool,
    passed: Option<bool>,
    minimum_simulations_per_scenario: usize,
    thresholds: Thresholds,
    checks: Vec<QualificationCheck>,
    note: String,
}

#[derive(Serialize)]
struct Thresholds {
    coverage_lower: f64,
    coverage_upper: f64,
    type_i_lower: f64,
    type_i_upper: f64,
    maximum_absolute_bias: f64,
    minimum_bca_availability: f64,
    minimum_studentized_availability: f64,
    minimum_normal_reference_availability: f64,
    minimum_usable_bootstrap_rate: f64,
}

#[derive(Serialize)]
struct QualificationCheck {
    metric: String,
    observed: Option<f64>,
    passed: Option<bool>,
}

struct SimulationObservation {
    estimate: f64,
    usable_rate: f64,
    percentile: (f64, f64),
    bca: Option<(f64, f64)>,
    studentized: Option<(f64, f64)>,
    normal_p: Option<f64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--self-check") {
        self_check()?;
        return Ok(());
    }
    let mode = value_after(&arguments, "--mode").unwrap_or("quick");
    let mut configuration = match mode {
        "quick" => Configuration {
            mode: "quick",
            simulations: 8,
            simulation_offset: 0,
            sample_size: 60,
            bootstrap_replicates: 79,
            studentized_inner_replicates: 0,
            workers: 1,
            qualification: false,
        },
        "pilot" => Configuration {
            mode: "pilot",
            simulations: 32,
            simulation_offset: 0,
            sample_size: 100,
            bootstrap_replicates: 199,
            studentized_inner_replicates: 0,
            workers: 1,
            qualification: false,
        },
        "sensitivity" => Configuration {
            mode: "sensitivity",
            simulations: 96,
            simulation_offset: 0,
            sample_size: 120,
            bootstrap_replicates: 399,
            studentized_inner_replicates: 0,
            workers: 1,
            qualification: false,
        },
        "studentized" => Configuration {
            mode: "studentized",
            simulations: 4,
            simulation_offset: 0,
            sample_size: 100,
            bootstrap_replicates: 999,
            studentized_inner_replicates: 99,
            workers: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            qualification: false,
        },
        "studentized-sensitivity" => Configuration {
            mode: "studentized-sensitivity",
            simulations: 4,
            simulation_offset: 0,
            sample_size: 100,
            bootstrap_replicates: 999,
            studentized_inner_replicates: 99,
            workers: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            qualification: false,
        },
        "studentized-qualification" => Configuration {
            mode: "studentized-qualification",
            simulations: 1_000,
            simulation_offset: 0,
            sample_size: 100,
            bootstrap_replicates: 999,
            studentized_inner_replicates: 99,
            workers: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            qualification: true,
        },
        "qualification" => Configuration {
            mode: "qualification",
            simulations: 1_000,
            simulation_offset: 0,
            sample_size: 100,
            bootstrap_replicates: 999,
            studentized_inner_replicates: 0,
            workers: 1,
            qualification: true,
        },
        _ => {
            return Err(
                format!("unknown mode '{mode}'; use quick, pilot, sensitivity, studentized, studentized-sensitivity, qualification, or studentized-qualification")
                    .into(),
            );
        }
    };
    if let Some(value) = value_after(&arguments, "--simulations") {
        configuration.simulations = value.parse::<usize>()?;
    }
    if let Some(value) = value_after(&arguments, "--simulation-offset") {
        configuration.simulation_offset = value.parse::<usize>()?;
    }
    let scenario_filter = value_after(&arguments, "--scenario");
    let output = value_after(&arguments, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("validation/results/monte_carlo_quick.json"));
    let started = Instant::now();
    let selected_specs = scenario_specs(configuration)
        .into_iter()
        .filter(|scenario| scenario_filter.is_none_or(|name| scenario.name == name))
        .collect::<Vec<_>>();
    if selected_specs.is_empty() {
        return Err(format!("no scenario matched {:?}", scenario_filter).into());
    }
    let scenarios = selected_specs
        .into_iter()
        .map(|scenario| run_scenario(scenario, configuration))
        .collect::<Result<Vec<_>, _>>()?;
    let qualification = qualify(&scenarios, configuration);
    let report = Report {
        schema_version: 1,
        harness_version: HARNESS_VERSION,
        engine_versions: EngineVersions {
            pls: PLS_METHOD_VERSION,
            resampling: RESAMPLING_METHOD_VERSION,
        },
        mode: configuration.mode,
        configuration: ConfigurationReport {
            simulations_per_scenario: configuration.simulations,
            simulation_offset: configuration.simulation_offset,
            sample_size: configuration.sample_size,
            bootstrap_replicates: configuration.bootstrap_replicates,
            studentized_inner_replicates: configuration.studentized_inner_replicates,
            workers: configuration.workers,
            confidence_level: 1.0 - ALPHA,
            master_seed: MASTER_SEED,
        },
        dgp: DgpReport {
            description: "Bivariate generated as x~N(0,1), y=beta*x+sqrt(1-beta^2)*e with independent unit-variance e. Normal modes use e~N(0,1); sensitivity, studentized-sensitivity, and studentized-qualification also include standardized t(3) errors.",
            variables: "Single-item reflective constructs x={x1}, y={y1}",
            evaluated_path: "x -> y; population PLS path equals beta",
            alternatives: vec![0.35, 0.0],
        },
        scenarios,
        qualification,
        elapsed_seconds: started.elapsed().as_secs_f64(),
    };
    write_report(&output, &report)?;
    println!("wrote {} ({})", output.display(), configuration.mode);
    Ok(())
}

fn scenario_specs(configuration: Configuration) -> Vec<ScenarioSpec> {
    let mut scenarios = vec![
        ScenarioSpec {
            name: "coverage_beta_0_35",
            beta: 0.35,
            error_distribution: ErrorDistribution::Normal,
            seed_domain: 0xC0A3,
        },
        ScenarioSpec {
            name: "null_beta_0",
            beta: 0.0,
            error_distribution: ErrorDistribution::Normal,
            seed_domain: 0xA011,
        },
    ];
    if configuration.mode == "sensitivity"
        || configuration.mode == "studentized-sensitivity"
        || configuration.mode == "studentized-qualification"
    {
        scenarios.extend([
            ScenarioSpec {
                name: "heavy_tail_coverage_beta_0_35",
                beta: 0.35,
                error_distribution: ErrorDistribution::StandardizedT3,
                seed_domain: 0xC0A3_73,
            },
            ScenarioSpec {
                name: "heavy_tail_null_beta_0",
                beta: 0.0,
                error_distribution: ErrorDistribution::StandardizedT3,
                seed_domain: 0xA011_73,
            },
        ]);
    }
    scenarios
}

fn run_scenario(
    scenario: ScenarioSpec,
    configuration: Configuration,
) -> Result<ScenarioSummary, Box<dyn std::error::Error>> {
    let mut observations = Vec::new();
    let mut failures = Vec::new();
    for simulation in 0..configuration.simulations {
        let global_simulation_index = configuration.simulation_offset + simulation;
        let simulation_seed = derived_seed(scenario.seed_domain, global_simulation_index as u64);
        match run_simulation(scenario, configuration, simulation_seed) {
            Ok(observation) => observations.push(observation),
            Err(error) => {
                if failures.len() < 20 {
                    failures.push(format!("simulation {global_simulation_index}: {error}"));
                }
            }
        }
    }
    let count = observations.len();
    let mean_estimate = mean(observations.iter().map(|value| value.estimate), count);
    let percentile_coverage = rate(
        observations
            .iter()
            .filter(|value| contains(value.percentile, scenario.beta))
            .count(),
        count,
    );
    let percentile_exclusion = rate(
        observations
            .iter()
            .filter(|value| excludes_zero(value.percentile))
            .count(),
        count,
    );
    let bca_values = observations
        .iter()
        .filter_map(|value| value.bca)
        .collect::<Vec<_>>();
    let studentized_values = observations
        .iter()
        .filter_map(|value| value.studentized)
        .collect::<Vec<_>>();
    let normal_values = observations
        .iter()
        .filter_map(|value| value.normal_p)
        .collect::<Vec<_>>();
    Ok(ScenarioSummary {
        name: scenario.name.into(),
        error_distribution: scenario.error_distribution.as_str(),
        true_path: scenario.beta,
        requested_simulations: configuration.simulations,
        completed_simulations: count,
        failed_simulations: configuration.simulations - count,
        mean_estimate,
        bias: mean_estimate.map(|value| value - scenario.beta),
        mean_usable_bootstrap_rate: mean(observations.iter().map(|value| value.usable_rate), count),
        percentile: IntervalSummary {
            available: count,
            coverage_rate: percentile_coverage,
            exclusion_of_zero_rate: percentile_exclusion,
        },
        bca: IntervalSummary {
            available: bca_values.len(),
            coverage_rate: rate(
                bca_values
                    .iter()
                    .filter(|interval| contains(**interval, scenario.beta))
                    .count(),
                bca_values.len(),
            ),
            exclusion_of_zero_rate: rate(
                bca_values
                    .iter()
                    .filter(|interval| excludes_zero(**interval))
                    .count(),
                bca_values.len(),
            ),
        },
        studentized: IntervalSummary {
            available: studentized_values.len(),
            coverage_rate: rate(
                studentized_values
                    .iter()
                    .filter(|interval| contains(**interval, scenario.beta))
                    .count(),
                studentized_values.len(),
            ),
            exclusion_of_zero_rate: rate(
                studentized_values
                    .iter()
                    .filter(|interval| excludes_zero(**interval))
                    .count(),
                studentized_values.len(),
            ),
        },
        normal_reference_available: normal_values.len(),
        normal_reference_type_i_rate: if scenario.beta == 0.0 {
            rate(
                normal_values.iter().filter(|value| **value < ALPHA).count(),
                normal_values.len(),
            )
        } else {
            None
        },
        failures,
    })
}

fn run_simulation(
    scenario: ScenarioSpec,
    configuration: Configuration,
    seed: u64,
) -> Result<SimulationObservation, Box<dyn std::error::Error>> {
    let bytes = generate_csv(
        scenario.beta,
        scenario.error_distribution,
        configuration.sample_size,
        seed,
    );
    let dataset =
        import_delimited_bytes(&bytes, "monte-carlo.csv", b',', &ImportOptions::default())?;
    let mut settings = AnalysisSettings::default();
    settings.method = AnalysisMethod::PlsPm;
    settings.bootstrap_samples = configuration.bootstrap_replicates;
    settings.studentized_inner_samples = configuration.studentized_inner_replicates;
    settings.seed = derived_seed(seed, 0xB00757A9);
    settings.workers = configuration.workers;
    settings.confidence_level = 1.0 - ALPHA;
    let recipe = AnalysisRecipe {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::nil(),
        created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: ModelSpec {
            id: Uuid::nil(),
            name: "Monte Carlo single-item path".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        },
        settings,
        metadata: BTreeMap::from([("validation_harness".into(), HARNESS_VERSION.into())]),
    };
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    let original = estimate_pls(&dataset, &base_recipe)?;
    let result = bootstrap_pls(
        &dataset,
        &recipe,
        &original,
        configuration.workers,
        || false,
        |_| {},
    )?;
    let identity = serde_json::to_string(&("path", ["x", "y"]))?;
    let percentile = result
        .percentile
        .parameters
        .iter()
        .find(|parameter| parameter.parameter == identity)
        .ok_or("bootstrap output has no x -> y path")?;
    let bca = result.bca.as_ref().and_then(|summary| {
        summary
            .parameters
            .iter()
            .find(|parameter| parameter.parameter == identity)
            .and_then(|parameter| parameter.lower.zip(parameter.upper))
    });
    let studentized = result.studentized.as_ref().and_then(|summary| {
        summary
            .parameters
            .iter()
            .find(|parameter| parameter.parameter == identity)
            .and_then(|parameter| {
                if parameter.unavailable_reason.is_none() {
                    parameter.lower.zip(parameter.upper)
                } else {
                    None
                }
            })
    });
    Ok(SimulationObservation {
        estimate: percentile.original,
        usable_rate: result.usable_replicates as f64 / result.plan.replicates as f64,
        percentile: (percentile.lower, percentile.upper),
        bca,
        studentized,
        normal_p: percentile.p_value_two_sided,
    })
}

fn generate_csv(
    beta: f64,
    error_distribution: ErrorDistribution,
    sample_size: usize,
    seed: u64,
) -> Vec<u8> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let residual_scale = (1.0 - beta * beta).sqrt();
    let mut output = String::from("x1,y1\n");
    for _ in 0..sample_size {
        let x = standard_normal(&mut rng);
        let y = beta * x + residual_scale * unit_variance_error(error_distribution, &mut rng);
        output.push_str(&format!("{x:.17},{y:.17}\n"));
    }
    output.into_bytes()
}

fn unit_variance_error(distribution: ErrorDistribution, rng: &mut ChaCha20Rng) -> f64 {
    match distribution {
        ErrorDistribution::Normal => standard_normal(rng),
        ErrorDistribution::StandardizedT3 => {
            let numerator = standard_normal(rng);
            let chi_square_3 = (0..3)
                .map(|_| {
                    let value = standard_normal(rng);
                    value * value
                })
                .sum::<f64>();
            let t3 = numerator / (chi_square_3 / 3.0).sqrt();
            t3 / 3.0_f64.sqrt()
        }
    }
}

fn standard_normal(rng: &mut ChaCha20Rng) -> f64 {
    let first = rng.random::<f64>().max(f64::MIN_POSITIVE);
    let second = rng.random::<f64>();
    (-2.0 * first.ln()).sqrt() * (std::f64::consts::TAU * second).cos()
}

fn qualify(scenarios: &[ScenarioSummary], configuration: Configuration) -> QualificationSummary {
    let thresholds = Thresholds {
        coverage_lower: 0.925,
        coverage_upper: 0.975,
        type_i_lower: 0.025,
        type_i_upper: 0.075,
        maximum_absolute_bias: 0.03,
        minimum_bca_availability: 1.0,
        minimum_studentized_availability: 0.99,
        minimum_normal_reference_availability: 1.0,
        minimum_usable_bootstrap_rate: 0.99,
    };
    let enough = configuration.qualification
        && scenarios
            .iter()
            .all(|scenario| scenario.completed_simulations >= 1_000);
    let Some(alternative) = scenarios.iter().find(|scenario| scenario.name == "coverage_beta_0_35")
    else {
        return qualification_not_evaluated(configuration, thresholds, "missing alternative scenario");
    };
    let Some(null) = scenarios.iter().find(|scenario| scenario.name == "null_beta_0") else {
        return qualification_not_evaluated(configuration, thresholds, "missing null scenario");
    };
    let mut candidates: Vec<(&str, Option<f64>, fn(f64) -> bool)> = vec![
        (
            "percentile_coverage",
            alternative.percentile.coverage_rate,
            passes_coverage_threshold as fn(f64) -> bool,
        ),
        (
            "bca_coverage",
            alternative.bca.coverage_rate,
            passes_coverage_threshold as fn(f64) -> bool,
        ),
        (
            "percentile_type_i",
            null.percentile.exclusion_of_zero_rate,
            passes_type_i_threshold as fn(f64) -> bool,
        ),
        (
            "bca_type_i",
            null.bca.exclusion_of_zero_rate,
            passes_type_i_threshold as fn(f64) -> bool,
        ),
        (
            "normal_reference_type_i",
            null.normal_reference_type_i_rate,
            passes_type_i_threshold as fn(f64) -> bool,
        ),
        (
            "absolute_bias",
            alternative.bias.map(f64::abs),
            passes_bias_threshold as fn(f64) -> bool,
        ),
        (
            "alternative_bca_availability",
            Some(
                alternative.bca.available as f64 / alternative.completed_simulations.max(1) as f64,
            ),
            passes_required_availability_threshold as fn(f64) -> bool,
        ),
        (
            "null_bca_availability",
            Some(null.bca.available as f64 / null.completed_simulations.max(1) as f64),
            passes_required_availability_threshold as fn(f64) -> bool,
        ),
        (
            "null_normal_reference_availability",
            Some(null.normal_reference_available as f64 / null.completed_simulations.max(1) as f64),
            passes_required_availability_threshold as fn(f64) -> bool,
        ),
        (
            "alternative_usable_bootstrap_rate",
            alternative.mean_usable_bootstrap_rate,
            passes_minimum_availability_threshold as fn(f64) -> bool,
        ),
        (
            "null_usable_bootstrap_rate",
            null.mean_usable_bootstrap_rate,
            passes_minimum_availability_threshold as fn(f64) -> bool,
        ),
    ];
    if configuration.studentized_inner_replicates > 0 {
        candidates.extend([
            (
                "studentized_coverage",
                alternative.studentized.coverage_rate,
                passes_coverage_threshold as fn(f64) -> bool,
            ),
            (
                "studentized_type_i",
                null.studentized.exclusion_of_zero_rate,
                passes_type_i_threshold as fn(f64) -> bool,
            ),
            (
                "alternative_studentized_availability",
                Some(
                    alternative.studentized.available as f64
                        / alternative.completed_simulations.max(1) as f64,
                ),
                passes_minimum_availability_threshold as fn(f64) -> bool,
            ),
            (
                "null_studentized_availability",
                Some(
                    null.studentized.available as f64 / null.completed_simulations.max(1) as f64,
                ),
                passes_minimum_availability_threshold as fn(f64) -> bool,
            ),
        ]);
    }
    if let (Some(heavy_alternative), Some(heavy_null)) = (
        scenarios
            .iter()
            .find(|scenario| scenario.name == "heavy_tail_coverage_beta_0_35"),
        scenarios
            .iter()
            .find(|scenario| scenario.name == "heavy_tail_null_beta_0"),
    ) {
        candidates.extend([
            (
                "heavy_tail_percentile_coverage",
                heavy_alternative.percentile.coverage_rate,
                passes_coverage_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_bca_coverage",
                heavy_alternative.bca.coverage_rate,
                passes_coverage_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_percentile_type_i",
                heavy_null.percentile.exclusion_of_zero_rate,
                passes_type_i_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_bca_type_i",
                heavy_null.bca.exclusion_of_zero_rate,
                passes_type_i_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_absolute_bias",
                heavy_alternative.bias.map(f64::abs),
                passes_bias_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_alternative_usable_bootstrap_rate",
                heavy_alternative.mean_usable_bootstrap_rate,
                passes_minimum_availability_threshold as fn(f64) -> bool,
            ),
            (
                "heavy_tail_null_usable_bootstrap_rate",
                heavy_null.mean_usable_bootstrap_rate,
                passes_minimum_availability_threshold as fn(f64) -> bool,
            ),
        ]);
        if configuration.studentized_inner_replicates > 0 {
            candidates.extend([
                (
                    "heavy_tail_studentized_coverage",
                    heavy_alternative.studentized.coverage_rate,
                    passes_coverage_threshold as fn(f64) -> bool,
                ),
                (
                    "heavy_tail_studentized_type_i",
                    heavy_null.studentized.exclusion_of_zero_rate,
                    passes_type_i_threshold as fn(f64) -> bool,
                ),
                (
                    "heavy_tail_alternative_studentized_availability",
                    Some(
                        heavy_alternative.studentized.available as f64
                            / heavy_alternative.completed_simulations.max(1) as f64,
                    ),
                    passes_minimum_availability_threshold as fn(f64) -> bool,
                ),
                (
                    "heavy_tail_null_studentized_availability",
                    Some(
                        heavy_null.studentized.available as f64
                            / heavy_null.completed_simulations.max(1) as f64,
                    ),
                    passes_minimum_availability_threshold as fn(f64) -> bool,
                ),
            ]);
        }
    }
    let checks = candidates
        .into_iter()
        .map(|(metric, observed, predicate)| QualificationCheck {
            metric: metric.into(),
            observed,
            passed: if enough {
                observed.map(predicate)
            } else {
                None
            },
        })
        .collect::<Vec<_>>();
    let passed = if enough {
        Some(checks.iter().all(|check| check.passed == Some(true)))
    } else {
        None
    };
    QualificationSummary {
        evaluated: enough,
        passed,
        minimum_simulations_per_scenario: 1_000,
        thresholds,
        checks,
        note: if enough {
            "Qualification thresholds were evaluated; inspect every check and the documented DGP scope."
                .into()
        } else {
            "This mode is a deterministic harness check only. Its simulation count is insufficient for coverage or type-I qualification."
                .into()
        },
    }
}

fn qualification_not_evaluated(
    configuration: Configuration,
    thresholds: Thresholds,
    reason: &str,
) -> QualificationSummary {
    QualificationSummary {
        evaluated: false,
        passed: None,
        minimum_simulations_per_scenario: 1_000,
        thresholds,
        checks: Vec::new(),
        note: format!(
            "Qualification was not evaluated for mode '{}': {reason}.",
            configuration.mode
        ),
    }
}

fn passes_coverage_threshold(value: f64) -> bool {
    (0.925..=0.975).contains(&value)
}

fn passes_type_i_threshold(value: f64) -> bool {
    (0.025..=0.075).contains(&value)
}

fn passes_bias_threshold(value: f64) -> bool {
    value <= 0.03
}

fn passes_required_availability_threshold(value: f64) -> bool {
    value >= 1.0
}

fn passes_minimum_availability_threshold(value: f64) -> bool {
    value >= 0.99
}

fn self_check() -> Result<(), Box<dyn std::error::Error>> {
    let first = generate_csv(0.35, ErrorDistribution::Normal, 20, 1234);
    let heavy = generate_csv(0.35, ErrorDistribution::StandardizedT3, 20, 1234);
    if first != generate_csv(0.35, ErrorDistribution::Normal, 20, 1234)
        || first == generate_csv(0.35, ErrorDistribution::Normal, 20, 1235)
        || first == heavy
        || !contains((-0.1, 0.4), 0.35)
        || !excludes_zero((0.01, 0.4))
        || excludes_zero((-0.01, 0.4))
    {
        return Err("Monte Carlo harness self-check failed".into());
    }
    println!("{HARNESS_VERSION} self-check passed");
    Ok(())
}

fn write_report(path: &Path, report: &Report) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn value_after<'a>(arguments: &'a [String], flag: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == flag)
        .and_then(|index| arguments.get(index + 1))
        .map(String::as_str)
}

fn derived_seed(domain: u64, index: u64) -> u64 {
    MASTER_SEED ^ domain.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn contains(interval: (f64, f64), value: f64) -> bool {
    interval.0 <= value && value <= interval.1
}

fn excludes_zero(interval: (f64, f64)) -> bool {
    interval.0 > 0.0 || interval.1 < 0.0
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn mean(values: impl Iterator<Item = f64>, count: usize) -> Option<f64> {
    (count > 0).then(|| values.sum::<f64>() / count as f64)
}
