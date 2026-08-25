//! Deterministic, validation-only probe for the additive MultiMod kernels.
//!
//! This executable does not decide qualification.  It emits raw SUT facts for
//! the independent Python comparator in `validation/multimod` and deliberately
//! contains no production implementation code.

use qpls_core::CompiledPlsPlanV2;
use qpls_estimation::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const PROBE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    All,
    Mga,
    Fimix,
    Pos,
    Conditional,
    Causal,
}

impl Gate {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "all" => Ok(Self::All),
            "mga" => Ok(Self::Mga),
            "fimix" => Ok(Self::Fimix),
            "pos" => Ok(Self::Pos),
            "conditional" => Ok(Self::Conditional),
            "causal" => Ok(Self::Causal),
            other => Err(format!("unknown gate: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Mga => "mga",
            Self::Fimix => "fimix",
            Self::Pos => "pos",
            Self::Conditional => "conditional",
            Self::Causal => "causal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scale {
    Development,
    Qualification,
}

impl Scale {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "development" => Ok(Self::Development),
            "qualification" => Ok(Self::Qualification),
            other => Err(format!("unknown scale: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Qualification => "qualification",
        }
    }
}

struct Arguments {
    gate: Gate,
    scale: Scale,
    output: Option<PathBuf>,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut gate = Gate::All;
    let mut scale = Scale::Development;
    let mut output = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--gate" => {
                gate = Gate::parse(
                    &arguments
                        .next()
                        .ok_or_else(|| "--gate requires a value".to_string())?,
                )?;
            }
            "--scale" => {
                scale = Scale::parse(
                    &arguments
                        .next()
                        .ok_or_else(|| "--scale requires a value".to_string())?,
                )?;
            }
            "--output" => {
                output = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--output requires a path".to_string())?,
                ));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Arguments {
        gate,
        scale,
        output,
    })
}

fn enum_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serializable public enum")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("MMQ.SUT.PROBE: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let mut sections = serde_json::Map::new();
    if matches!(arguments.gate, Gate::All | Gate::Mga) {
        sections.insert("mga".into(), mga_probe()?);
    }
    if matches!(arguments.gate, Gate::All | Gate::Fimix) {
        sections.insert("fimix".into(), fimix_probe(arguments.scale)?);
    }
    if matches!(arguments.gate, Gate::All | Gate::Pos) {
        sections.insert("pos".into(), pos_probe()?);
    }
    if matches!(arguments.gate, Gate::All | Gate::Conditional) {
        sections.insert("conditional".into(), conditional_probe()?);
    }
    if matches!(arguments.gate, Gate::All | Gate::Causal) {
        sections.insert("causal".into(), causal_probe()?);
    }
    let document = json!({
        "schema_version": PROBE_SCHEMA_VERSION,
        "probe_id": "qpls.multimod.scientific_sut_probe.v1",
        "gate": arguments.gate.as_str(),
        "scale": arguments.scale.as_str(),
        "seed": 42,
        "sections": sections,
    });
    let bytes = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    if let Some(path) = arguments.output {
        fs::write(path, bytes).map_err(|error| error.to_string())?;
    } else {
        println!(
            "{}",
            String::from_utf8(bytes).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn group_design(group_count: usize, observations_per_group: usize) -> MultigroupDesignV1 {
    let groups = (0..group_count)
        .map(|index| GroupIdentityV1 {
            index: GroupIndexV1::new(index).expect("fixture group index"),
            value: TypedGroupValueV1::Text {
                value: format!("G{}", index + 1),
            },
            display_label: format!("Group {}", index + 1),
        })
        .collect::<Vec<_>>();
    let rows = (0..group_count)
        .flat_map(|group| {
            (0..observations_per_group).map(move |within| SelectedGroupRowV1 {
                source_row: (group * observations_per_group + within) as u64,
                stable_row_token: (group * observations_per_group + within) as u64,
                group: GroupIndexV1::new(group).expect("fixture group index"),
            })
        })
        .collect();
    MultigroupDesignV1 { groups, rows }
}

fn slope(values_x: &[f64], values_y: &[f64], rows: &[u64]) -> Result<f64, RefitFailureV1> {
    if rows.len() < 2 {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "slope fixture needs two rows",
        ));
    }
    let count = rows.len() as f64;
    let mean_x = rows.iter().map(|row| values_x[*row as usize]).sum::<f64>() / count;
    let mean_y = rows.iter().map(|row| values_y[*row as usize]).sum::<f64>() / count;
    let denominator = rows
        .iter()
        .map(|row| (values_x[*row as usize] - mean_x).powi(2))
        .sum::<f64>();
    if denominator <= f64::EPSILON {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::SingularModel,
            "slope fixture is singular",
        ));
    }
    Ok(rows
        .iter()
        .map(|row| (values_x[*row as usize] - mean_x) * (values_y[*row as usize] - mean_y))
        .sum::<f64>()
        / denominator)
}

fn slope_refitter<'a>(
    x: &'a [f64],
    y: &'a [f64],
    parameter: ParameterIdentityV1,
) -> impl MultigroupRefitterV1 + 'a {
    move |request: &MultigroupFitRequestV1| {
        Ok(ParameterVectorV1 {
            parameters: vec![ParameterEstimateV1 {
                parameter: parameter.clone(),
                estimate: slope(x, y, &request.source_rows)?,
            }],
        })
    }
}

fn mga_probe() -> Result<Value, String> {
    let eligibility = [2usize, 3, 5, 20]
        .into_iter()
        .map(|groups| {
            let result = assess_multigroup_design_v1(&group_design(groups, 10));
            json!({
                "groups": groups,
                "eligible": result.eligible,
                "group_counts": result.group_counts,
                "blockers": result.blockers,
                "warnings": result.warnings,
                "pairwise_comparisons": groups * (groups - 1) / 2,
            })
        })
        .collect::<Vec<_>>();

    let insufficient = assess_multigroup_design_v1(&group_design(2, 9));
    let mut imbalance_design = group_design(2, 10);
    for row in 20..111 {
        imbalance_design.rows.push(SelectedGroupRowV1 {
            source_row: row,
            stable_row_token: row,
            group: GroupIndexV1::new(1).expect("fixture group index"),
        });
    }
    let imbalance = assess_multigroup_design_v1(&imbalance_design);

    let design = group_design(2, 10);
    let x = (0..20)
        .map(|row| (row % 10) as f64 - 4.5)
        .collect::<Vec<_>>();
    let y = (0..20)
        .map(|row| {
            let local_x = x[row];
            let group_slope = if row < 10 { 0.5 } else { 1.5 };
            group_slope * local_x + ((row * 7 % 5) as f64 - 2.0) * 0.03
        })
        .collect::<Vec<_>>();
    let parameter = ParameterIdentityV1 {
        stable_id: "path:x->y".into(),
        family: ParameterFamilyV1::StructuralPath,
    };
    let config = MultigroupResamplingConfigV1::official_defaults();
    let forward_pair = OrderedGroupPairV1::new(
        GroupIndexV1::new(0).expect("fixture group index"),
        GroupIndexV1::new(1).expect("fixture group index"),
    )
    .map_err(|error| error.to_string())?;
    let reverse_pair = OrderedGroupPairV1::new(forward_pair.group_b, forward_pair.group_a)
        .map_err(|error| error.to_string())?;
    let forward = run_pairwise_permutation_v1(
        &design,
        forward_pair,
        std::slice::from_ref(&parameter),
        config,
        &mut slope_refitter(&x, &y, parameter.clone()),
    )
    .map_err(|error| error.to_string())?;
    let reverse = run_pairwise_permutation_v1(
        &design,
        reverse_pair,
        std::slice::from_ref(&parameter),
        config,
        &mut slope_refitter(&x, &y, parameter.clone()),
    )
    .map_err(|error| error.to_string())?;

    let hypotheses = [("h1", 0.01), ("h2", 0.04), ("h3", 0.03), ("h4", 0.20)]
        .into_iter()
        .map(|(id, probability)| HypothesisProbabilityV1 {
            hypothesis_id: id.into(),
            raw_probability: probability,
        })
        .collect::<Vec<_>>();
    let multiplicity = [
        MultiplicityMethodV1::None,
        MultiplicityMethodV1::Holm,
        MultiplicityMethodV1::Bonferroni,
        MultiplicityMethodV1::Sidak,
        MultiplicityMethodV1::BenjaminiHochberg,
    ]
    .into_iter()
    .map(|method| {
        Ok(json!({
            "method": enum_value(&method),
            "probabilities": adjust_probabilities_v1(&hypotheses, method)
                .map_err(|error| error.to_string())?,
        }))
    })
    .collect::<Result<Vec<_>, String>>()?;

    Ok(json!({
        "eligibility": eligibility,
        "boundary_failures": {
            "insufficient_complete_cases": insufficient,
            "imbalance_above_ten_to_one": imbalance,
        },
        "label_reversal": {
            "x": x,
            "y": y,
            "group_a_rows": (0_u64..10).collect::<Vec<_>>(),
            "group_b_rows": (10_u64..20).collect::<Vec<_>>(),
            "forward": {
                "point": forward.point_estimates,
                "inference": forward.parameters,
                "usable": forward.usable,
                "failed": forward.failed,
                "plan_sha256": forward.plan_sha256,
            },
            "reverse": {
                "point": reverse.point_estimates,
                "inference": reverse.parameters,
                "usable": reverse.usable,
                "failed": reverse.failed,
                "plan_sha256": reverse.plan_sha256,
            },
        },
        "multiplicity_input": hypotheses,
        "multiplicity": multiplicity,
    }))
}

fn sample_standardize(values: &[f64]) -> Vec<f64> {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let standard_deviation = variance.sqrt();
    values
        .iter()
        .map(|value| (value - mean) / standard_deviation)
        .collect()
}

fn heterogeneity_fixture() -> (StandardizedFimixInputV2, Vec<usize>) {
    let mut x = Vec::with_capacity(80);
    let mut y = Vec::with_capacity(80);
    let mut truth = Vec::with_capacity(80);
    let noise = [-0.17, 0.08, 0.12, -0.05, 0.03, -0.11, 0.15, -0.02];
    for class in 0..2 {
        for row in 0..40 {
            let predictor = (row as f64 - 19.5) / 7.0;
            let residual = noise[(row + class * 3) % noise.len()];
            let outcome = if class == 0 {
                -2.2 + 0.85 * predictor + residual
            } else {
                2.2 - 0.85 * predictor + residual
            };
            x.push(predictor);
            y.push(outcome);
            truth.push(class);
        }
    }
    let x = sample_standardize(&x);
    let y = sample_standardize(&y);
    (
        StandardizedFimixInputV2 {
            interaction_profile: HeterogeneityInteractionProfileV2::P0Structural,
            metric: PooledStandardizedMetricReceiptV2 {
                metric_id: "validation.pooled.metric.v1".into(),
                source_sha256: "sha256:multimod-heterogeneity-fixture-v1".into(),
                observation_count: x.len(),
                scores_standardized_once_on_pooled_rows: true,
                products_standardized_once_on_pooled_rows: false,
            },
            equations: vec![StandardizedStructuralEquationV2 {
                equation_id: "eq:y".into(),
                outcome_id: "y".into(),
                predictor_ids: vec!["x".into()],
                design: x.into_iter().map(|value| vec![value]).collect(),
                outcome: y,
                include_intercept: true,
            }],
        },
        truth,
    )
}

fn fimix_probe(scale: Scale) -> Result<Value, String> {
    let (input, truth) = heterogeneity_fixture();
    let mut config = FimixPlsV2Config::for_classes(2);
    if scale == Scale::Development {
        config.starts = 10;
        config.max_iterations = 1_000;
    }
    let fit = match fit_fimix_pls_v2(&input, &config) {
        Ok(result) => json!({
            "status": "fit",
            "result": result,
        }),
        Err(error) => json!({
            "status": "error",
            "error": format!("{error:?}"),
        }),
    };

    let mut collapse_config = config.clone();
    collapse_config.residual_variance_floor = 2.0;
    let collapse = match fit_fimix_pls_v2(&input, &collapse_config) {
        Ok(result) => json!({
            "status": "unexpected_fit",
            "log_likelihood": result.log_likelihood,
        }),
        Err(HeterogeneityV2Error::NoConvergedFimixStart { diagnostics }) => json!({
            "status": "blocked",
            "error_kind": "no_converged_fimix_start",
            "failure_codes": diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic.failure_code)
                .map(|code| enum_value(&code))
                .collect::<Vec<_>>(),
            "start_count": diagnostics.len(),
        }),
        Err(error) => json!({
            "status": "blocked",
            "error_kind": "other",
            "error": format!("{error:?}"),
        }),
    };

    Ok(json!({
        "input": input,
        "true_assignments": truth,
        "config": config,
        "fit": fit,
        "collapse_boundary": collapse,
        "development_acceptance": {
            "minimum_ari": 0.80,
            "purpose": "deterministic strong-separation recovery pilot",
        },
        "qualification_acceptance": {
            "minimum_median_ari": 0.80,
            "requires_multiple_predeclared_seeds": true,
        },
    }))
}

struct OlsPosRefitter {
    x: Vec<f64>,
    y: Vec<f64>,
}

fn ols_fit(x: &[f64], y: &[f64], rows: &[usize]) -> Result<(f64, f64, f64), String> {
    if rows.len() < 3 {
        return Err("OLS segment has fewer than three rows".into());
    }
    let count = rows.len() as f64;
    let mean_x = rows.iter().map(|row| x[*row]).sum::<f64>() / count;
    let mean_y = rows.iter().map(|row| y[*row]).sum::<f64>() / count;
    let ss_x = rows
        .iter()
        .map(|row| (x[*row] - mean_x).powi(2))
        .sum::<f64>();
    if ss_x <= f64::EPSILON {
        return Err("OLS segment predictor is singular".into());
    }
    let slope = rows
        .iter()
        .map(|row| (x[*row] - mean_x) * (y[*row] - mean_y))
        .sum::<f64>()
        / ss_x;
    let intercept = mean_y - slope * mean_x;
    let total = rows
        .iter()
        .map(|row| (y[*row] - mean_y).powi(2))
        .sum::<f64>();
    let residual = rows
        .iter()
        .map(|row| (y[*row] - intercept - slope * x[*row]).powi(2))
        .sum::<f64>();
    if total <= f64::EPSILON {
        return Err("OLS segment outcome is singular".into());
    }
    Ok((intercept, slope, (1.0 - residual / total).clamp(0.0, 1.0)))
}

impl PlsPosFullRefitterV2 for OlsPosRefitter {
    fn refit_segment(
        &mut self,
        _segment_index: usize,
        row_indices: &[usize],
        scoring: PosScoringContractV2,
    ) -> Result<PosSegmentFullFitV2, String> {
        if scoring != PosScoringContractV2::PublishedP0FullSegmentPls {
            return Err("validation refitter supports published P0 only".into());
        }
        let (intercept, slope, r_squared) = ols_fit(&self.x, &self.y, row_indices)?;
        Ok(PosSegmentFullFitV2 {
            r_squared: vec![PosOutcomeR2V2 {
                outcome_id: "y".into(),
                r_squared,
            }],
            outcome_fit_audits: Vec::new(),
            parameter_signature: vec![intercept, slope],
            receipt: PosFullRefitReceiptV2 {
                method_version: PLS_POS_PUBLISHED_METHOD_VERSION_V2.into(),
                full_segment_pls_refit: true,
                measurement_scores_reestimated: true,
                score_orientation_reapplied: true,
                interaction_stage_one_refit: false,
                interaction_operands_restandardized_within_destination: false,
                interaction_products_rebuilt_within_destination: false,
                joint_structural_equations_refit: true,
            },
        })
    }
}

fn pos_probe() -> Result<Value, String> {
    let (input, truth) = heterogeneity_fixture();
    let equation = &input.equations[0];
    let x = equation.design.iter().map(|row| row[0]).collect::<Vec<_>>();
    let y = equation.outcome.clone();
    let features = x
        .iter()
        .zip(&y)
        .map(|(x, y)| vec![*x, *y])
        .collect::<Vec<_>>();
    let starts = build_pls_pos_start_plan_v2(&features, 2, 42, Some(&truth))
        .map_err(|error| error.to_string())?;
    let repeated = build_pls_pos_start_plan_v2(&features, 2, 42, Some(&truth))
        .map_err(|error| error.to_string())?;
    let config = PlsPosV2Config::for_segments(2, x.len());
    let result = fit_pls_pos_published_v2(
        &starts,
        &config,
        &mut OlsPosRefitter {
            x: x.clone(),
            y: y.clone(),
        },
    );
    let fit = match result {
        Ok(result) => json!({"status": "fit", "result": result}),
        Err(error) => json!({"status": "error", "error": format!("{error:?}")}),
    };
    Ok(json!({
        "x": x,
        "y": y,
        "true_assignments": truth,
        "config": config,
        "start_plan": starts,
        "same_seed_start_plan_equal": starts == repeated,
        "same_k_partition_is_tenth_start": starts.get(9) == Some(&truth),
        "fit": fit,
        "refitter_contract": "validation_identity_score_ols_full_segment_refit_v1",
    }))
}

fn linear(moderator_id: &str, estimate: f64) -> ConditionalLinearCoefficientV2 {
    ConditionalLinearCoefficientV2 {
        moderator_id: moderator_id.into(),
        estimate,
    }
}

fn edge(
    relation_id: &str,
    source_id: &str,
    target_id: &str,
    intercept: f64,
    linear_coefficients: Vec<ConditionalLinearCoefficientV2>,
    pairwise_coefficients: Vec<ConditionalPairwiseCoefficientV2>,
) -> ConditionalEdgeFunctionV2 {
    ConditionalEdgeFunctionV2 {
        relation_id: relation_id.into(),
        source_id: source_id.into(),
        target_id: target_id.into(),
        intercept,
        linear_coefficients,
        pairwise_coefficients,
    }
}

fn path_cases() -> Vec<(&'static str, ExplicitConditionalPathV2)> {
    vec![
        (
            "first_stage",
            ExplicitConditionalPathV2 {
                path_id: "first_stage".into(),
                edges: vec![
                    edge("x_m", "x", "m", 0.7, vec![linear("z", 0.4)], vec![]),
                    edge("m_y", "m", "y", 1.2, vec![], vec![]),
                ],
            },
        ),
        (
            "second_stage",
            ExplicitConditionalPathV2 {
                path_id: "second_stage".into(),
                edges: vec![
                    edge("x_m", "x", "m", 0.7, vec![], vec![]),
                    edge("m_y", "m", "y", 1.2, vec![linear("z", -0.3)], vec![]),
                ],
            },
        ),
        (
            "both_stage",
            ExplicitConditionalPathV2 {
                path_id: "both_stage".into(),
                edges: vec![
                    edge("x_m", "x", "m", 0.7, vec![linear("z", 0.4)], vec![]),
                    edge("m_y", "m", "y", 1.2, vec![linear("z", -0.3)], vec![]),
                ],
            },
        ),
        (
            "three_way",
            ExplicitConditionalPathV2 {
                path_id: "three_way".into(),
                edges: vec![
                    edge(
                        "x_m",
                        "x",
                        "m",
                        0.6,
                        vec![linear("w", 0.2), linear("z", 0.4)],
                        vec![ConditionalPairwiseCoefficientV2 {
                            first_moderator_id: "w".into(),
                            second_moderator_id: "z".into(),
                            estimate: 0.15,
                        }],
                    ),
                    edge("m_y", "m", "y", 1.1, vec![], vec![]),
                ],
            },
        ),
        (
            "long_path_six_edges",
            ExplicitConditionalPathV2 {
                path_id: "long_path_six_edges".into(),
                edges: vec![
                    edge("r1", "x", "m1", 0.9, vec![linear("z", 0.2)], vec![]),
                    edge("r2", "m1", "m2", 0.8, vec![], vec![]),
                    edge("r3", "m2", "m3", 0.7, vec![linear("w", -0.1)], vec![]),
                    edge("r4", "m3", "m4", 0.6, vec![], vec![]),
                    edge("r5", "m4", "m5", 0.5, vec![], vec![]),
                    edge("r6", "m5", "y", 0.4, vec![], vec![]),
                ],
            },
        ),
    ]
}

fn path_probe_case(label: &str, path: ExplicitConditionalPathV2) -> Result<Value, String> {
    let polynomial =
        compile_explicit_conditional_path_v2(&path).map_err(|error| format!("{label}: {error}"))?;
    let joint_values = polynomial
        .moderator_ids
        .iter()
        .map(|moderator| {
            let value = if moderator == "w" { -0.25 } else { 0.5 };
            (moderator.clone(), value)
        })
        .collect::<BTreeMap<_, _>>();
    let joint = ConditionalProbePointV2 {
        probe_id: format!("{label}:joint"),
        standardized_values: joint_values,
    };
    let left = ConditionalProbePointV2 {
        probe_id: format!("{label}:left"),
        standardized_values: polynomial
            .moderator_ids
            .iter()
            .map(|moderator| (moderator.clone(), 1.0))
            .collect(),
    };
    let right = ConditionalProbePointV2 {
        probe_id: format!("{label}:right"),
        standardized_values: polynomial
            .moderator_ids
            .iter()
            .map(|moderator| (moderator.clone(), -1.0))
            .collect(),
    };
    let scalar_index = if polynomial.moderator_ids.len() == 1 {
        match scalar_index_of_moderated_mediation_v2(&polynomial, &polynomial.moderator_ids[0]) {
            Ok(value) => json!({"status": "available", "value": value}),
            Err(error) => json!({"status": "blocked", "error": format!("{error:?}")}),
        }
    } else {
        json!({"status": "not_requested"})
    };
    Ok(json!({
        "label": label,
        "input": path,
        "polynomial": polynomial,
        "joint_probe": joint,
        "effect": conditional_effect_v2(&polynomial, &joint)
            .map_err(|error| error.to_string())?,
        "derivatives": conditional_derivatives_v2(&polynomial, &joint)
            .map_err(|error| error.to_string())?,
        "left_probe": left,
        "right_probe": right,
        "contrast": conditional_probe_contrast_v2(&polynomial, &left, &right)
            .map_err(|error| error.to_string())?,
        "scalar_index": scalar_index,
    }))
}

fn fixture_compiled_plan() -> Result<CompiledPlsPlanV2, String> {
    serde_json::from_value(json!({
        "model_id": "conditional-frequency-equivalence-v1",
        "scientific_hash": "sha256:conditional-frequency-equivalence-v1",
        "dataset_id": "validation.multimod.frequency.v1",
        "blocks": [
            {"construct_id": "x", "mode": "mode_a", "indicators": []},
            {"construct_id": "y", "mode": "mode_a", "indicators": []},
            {"construct_id": "z", "mode": "mode_a", "indicators": []}
        ],
        "paths": [
            {"relation_id": "rel:x:y", "source": "x", "target": "y", "parameter_id": "beta:x:y"},
            {"relation_id": "rel:z:y", "source": "z", "target": "y", "parameter_id": "beta:z:y"}
        ]
    }))
    .map_err(|error| error.to_string())
}

fn expand_by_counts(values: &[f64], counts: &[u64]) -> Vec<f64> {
    values
        .iter()
        .zip(counts)
        .flat_map(|(value, count)| std::iter::repeat(*value).take(*count as usize))
        .collect()
}

fn frequency_equivalence_probe() -> Result<Value, String> {
    let plan = fixture_compiled_plan()?;
    let interaction = MultimodConditionalTwoWayInteractionV2 {
        interaction_id: "int:x:z".into(),
        output_id: "product:x:z".into(),
        focal_relation_id: "rel:x:y".into(),
        interaction_effect_relation_id: "rel:xz:y".into(),
        interaction_effect_parameter_id: "gamma:x:z:y".into(),
        focal_predictor_id: "x".into(),
        moderator_id: "z".into(),
        outcome_id: "y".into(),
    };
    let counts = vec![2_u64, 1, 3, 2, 1, 4, 2, 3];
    let x = vec![-2.0, -1.2, -0.5, 0.2, 0.8, 1.3, 1.7, 2.2];
    let z = vec![-1.4, 0.6, -0.8, 1.1, -0.2, 1.7, -1.1, 0.4];
    let y = x
        .iter()
        .zip(&z)
        .enumerate()
        .map(|(row, (x, z))| 0.7 * x + 0.4 * z + 0.9 * x * z + ((row * 5 % 7) as f64 - 3.0) * 0.02)
        .collect::<Vec<_>>();
    let compact_scores = BTreeMap::from([
        ("x".into(), x.clone()),
        ("y".into(), y.clone()),
        ("z".into(), z.clone()),
    ]);
    let compact = estimate_multimod_conditional_interactions_v2_with_control(
        &plan,
        std::slice::from_ref(&interaction),
        &compact_scores,
        MultimodConditionalRowMassV2::PositiveIntegerFrequency(&counts),
        || true,
    )
    .map_err(|error| error.to_string())?;
    let expanded_x = expand_by_counts(&x, &counts);
    let expanded_y = expand_by_counts(&y, &counts);
    let expanded_z = expand_by_counts(&z, &counts);
    let expanded_scores = BTreeMap::from([
        ("x".into(), expanded_x.clone()),
        ("y".into(), expanded_y.clone()),
        ("z".into(), expanded_z.clone()),
    ]);
    let expanded = estimate_multimod_conditional_interactions_v2_with_control(
        &plan,
        std::slice::from_ref(&interaction),
        &expanded_scores,
        MultimodConditionalRowMassV2::Unweighted,
        || true,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "counts": counts,
        "compact_scores": compact_scores,
        "expanded_scores": expanded_scores,
        "compact_result": compact,
        "expanded_result": expanded,
    }))
}

fn conditional_probe() -> Result<Value, String> {
    let cases = path_cases()
        .into_iter()
        .map(|(label, path)| path_probe_case(label, path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "path_cases": cases,
        "frequency_expanded_equivalence": frequency_equivalence_probe()?,
    }))
}

fn main_term(id: &str) -> ObservedLinearTermV1 {
    ObservedLinearTermV1 {
        term_id: id.into(),
        factor_variable_ids: vec![id.into()],
    }
}

fn reviewed_checklist() -> InterventionalIdentificationChecklistV1 {
    InterventionalIdentificationChecklistV1 {
        temporal_order_reviewed: true,
        consistency_reviewed: true,
        treatment_outcome_exchangeability_reviewed: true,
        treatment_mediator_exchangeability_reviewed: true,
        mediator_outcome_exchangeability_reviewed: true,
        no_exposure_induced_mediator_outcome_confounder_reviewed: true,
        no_recanting_witness_reviewed: true,
        linear_model_specification_reviewed: true,
    }
}

fn causal_fixture() -> InterventionalCausalMediationInputV1 {
    let mut x = Vec::new();
    let mut c = Vec::new();
    let mut m = Vec::new();
    let mut y = Vec::new();
    for row in 0..40 {
        let treatment = (row % 2) as f64;
        let covariate = ((row * 7) % 13) as f64 / 6.0 - 1.0;
        let mediator = 1.0 + 2.0 * treatment + 0.5 * covariate;
        let outcome = 3.0 + treatment + 4.0 * mediator + 0.25 * covariate;
        x.push(treatment);
        c.push(covariate);
        m.push(mediator);
        y.push(outcome);
    }
    InterventionalCausalMediationInputV1 {
        analysis_id: "known_linear_dgp".into(),
        columns: vec![
            ObservedNumericColumnV1 {
                variable_id: "x".into(),
                values: x,
            },
            ObservedNumericColumnV1 {
                variable_id: "c".into(),
                values: c,
            },
            ObservedNumericColumnV1 {
                variable_id: "m".into(),
                values: m,
            },
            ObservedNumericColumnV1 {
                variable_id: "y".into(),
                values: y,
            },
        ],
        treatment_variable_id: "x".into(),
        ordered_mediator_variable_ids: vec!["m".into()],
        outcome_variable_id: "y".into(),
        adjustment_covariate_variable_ids: vec!["c".into()],
        baseline_moderator_variable_ids: Vec::new(),
        baseline_moderator_intervention_values: BTreeMap::new(),
        equations: vec![
            ObservedLinearEquationV1 {
                equation_id: "m_model".into(),
                outcome_variable_id: "m".into(),
                terms: vec![main_term("x"), main_term("c")],
            },
            ObservedLinearEquationV1 {
                equation_id: "y_model".into(),
                outcome_variable_id: "y".into(),
                terms: vec![main_term("x"), main_term("m"), main_term("c")],
            },
        ],
        treatment_contrast: ObservedTreatmentContrastV1 {
            kind: ObservedTreatmentKindV1::Binary,
            x0: 0.0,
            x1: 1.0,
        },
        identification_checklist: reviewed_checklist(),
        positivity_policy: InterventionalPositivityPolicyV1::default(),
        unsupported_features: UnsupportedInterventionalFeatureRequestV1::default(),
    }
}

fn blocker_codes(input: &InterventionalCausalMediationInputV1) -> Value {
    match estimate_interventional_mediation_v1(input) {
        Ok(result) => json!({
            "status": "unexpected_fit",
            "target_id": result.target_id,
        }),
        Err(blockers) => json!({
            "status": "blocked",
            "codes": blockers
                .iter()
                .map(|blocker| enum_value(&blocker.code))
                .collect::<Vec<_>>(),
            "blockers": blockers,
        }),
    }
}

fn causal_probe() -> Result<Value, String> {
    let input = causal_fixture();
    let result = estimate_interventional_mediation_v1(&input).map_err(|blockers| {
        blockers
            .into_iter()
            .map(|blocker| format!("{:?}: {}", blocker.code, blocker.detail))
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let mut checklist_failure = input.clone();
    checklist_failure
        .identification_checklist
        .consistency_reviewed = false;
    let mut positivity_failure = input.clone();
    let treatment = positivity_failure
        .columns
        .iter_mut()
        .find(|column| column.variable_id == "x")
        .expect("fixture treatment column");
    for value in treatment.values.iter_mut().skip(5) {
        *value = 1.0;
    }
    let mut unsupported_failure = input.clone();
    unsupported_failure
        .unsupported_features
        .natural_or_cross_world_effects = true;

    Ok(json!({
        "input": input,
        "known_target": result,
        "assumption_failures": {
            "incomplete_checklist": blocker_codes(&checklist_failure),
            "positivity": blocker_codes(&positivity_failure),
            "unsupported_natural_effect": blocker_codes(&unsupported_failure),
        },
    }))
}
