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
    HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
    HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2,
    HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2, align_labels_exhaustive_v2,
};
use qpls_runner::*;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;
use support::*;

const SCHEMA_VERSION: u32 = 1;
const SUITE_ID: &str = "qpls.multimod.heterogeneity.production-qualification.v2";
const OBSERVATIONS: usize = 400;

fn fixture_observations() -> usize {
    if metamorphic::compact_matrix_v1() {
        80
    } else {
        OBSERVATIONS
    }
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
}

fn arguments() -> Result<Arguments, DynError> {
    let mut output = None;
    let mut seed = 42_u64;
    let mut scale = Scale::Qualification;
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
            _ => return Err(invalid(format!("unknown argument {argument}"))),
        }
    }
    Ok(Arguments {
        output: output.ok_or_else(|| invalid("--output is required"))?,
        seed,
        scale,
    })
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
    let observations = fixture_observations();
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
        let x = if scenario == Scenario::NonNormal {
            raw_x.signum() * raw_x.abs().powf(1.65)
        } else {
            raw_x
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
                        && indicator >= 2
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

    let (construct_ids, paths) = profile_constructs(profile, scenario == Scenario::RankDeficient);
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

fn main() -> Result<(), DynError> {
    let args = arguments()?;
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
            run_discovery(
                &format!("fimix-p0-strong-seed-{seed}"),
                fixture,
                *seed,
                vec![2],
                vec![HeterogeneityAlgorithmV2::FimixPlsV2],
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
    let (p0_tandem_identity, p0_tandem_discovery) = run_discovery(
        "pos-published-p0-discovery",
        &p0,
        args.seed,
        vec![2],
        tandem_p0_algorithms.clone(),
    )?;
    let (p2_identity, p2_discovery) = run_discovery(
        "heterogeneity-p2-tandem-discovery",
        &p2,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
    )?;
    let (p23_identity, p23_discovery) = run_discovery(
        "heterogeneity-p23-tandem-discovery",
        &p23,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
    )?;
    let (metric_failure_identity, metric_failure_discovery) = run_discovery(
        "pos-p2-common-metric-failure-discovery",
        &metric_failure,
        args.seed,
        vec![2],
        tandem_interaction_algorithms.clone(),
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
    let mut pos_published_k3_through_k5_bootstrap = Vec::new();
    if args.scale == Scale::Qualification && !metamorphic::compact_matrix_v1() {
        for selected_k in 3_u8..=5 {
            let fixture = make_fixture_with_classes(
                &format!("heterogeneity-p0-strong-k{selected_k}"),
                HeterogeneityInteractionProfileV2::P0Structural,
                Scenario::StrongSeparation,
                0x4b00_0000_u64
                    .wrapping_add(u64::from(selected_k) << 16)
                    .wrapping_add(args.seed),
                usize::from(selected_k),
            )?;
            let algorithms = vec![HeterogeneityAlgorithmV2::PlsPosPublishedV2];
            let (discovery_identity, discovery) = run_discovery(
                &format!("pos-published-p0-k{selected_k}-discovery"),
                &fixture,
                args.seed,
                vec![selected_k],
                algorithms.clone(),
            )?;
            let bootstrap = run_inference_at_k(
                &format!("pos-published-p0-k{selected_k}-fixed-k-bootstrap"),
                &fixture,
                args.seed,
                vec![selected_k],
                algorithms,
                discovery_identity,
                HeterogeneityAlgorithmV2::PlsPosPublishedV2,
                selected_k,
                false,
            )?;
            pos_published_k3_through_k5_discovery.push(discovery);
            pos_published_k3_through_k5_bootstrap.push(bootstrap);
        }
    }

    let mut bootstrap_cells = if metamorphic::compact_matrix_v1() {
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
    bootstrap_cells.extend(pos_published_k3_through_k5_bootstrap);

    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "suite_id": SUITE_ID,
        "scale": match args.scale { Scale::Development => "development", Scale::Qualification => "qualification" },
        "campaign_seed": args.seed,
        "seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "fixture_observations": fixture_observations(),
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
    fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
