//! Full raw-data MGA qualification producer for MultiMod V1.
//!
//! This executable is validation-only. It synthesizes deterministic observed
//! columns, compiles real Recipe V4 authorities, and calls the public raw MGA
//! runner. It never substitutes identity scores or a validation refitter.

#[path = "support_multimod_metamorphic/mod.rs"]
mod metamorphic;
#[path = "support_multimod_qualification/mod.rs"]
mod support;

use qpls_core::*;
use qpls_estimation::*;
use qpls_runner::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use support::*;

const SCHEMA_VERSION: u32 = 1;
const SUITE_ID: &str = "qpls.multimod.mga.production-qualification.v1";
const PERMUTATIONS: u32 = 5_000;
const BOOTSTRAPS: u32 = 5_000;

fn sha256_f64_series(values: &[f64]) -> String {
    let mut digest = Sha256::new();
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        digest.update(value.to_bits().to_le_bytes());
    }
    let digest = digest.finalize();
    format!("{digest:x}")
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroupEncoding {
    Text,
    Integer,
    Number,
    Boolean,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileFixture {
    General,
    FrequencyExpansionUnweighted,
    MultipleTwoWay,
    ThreeWay,
    ModeratedMediation,
    MultipleHoc,
    CaseWeighted,
    FrequencyWeighted,
    ReflectivePlsc,
    GeneralParametric,
}

impl ProfileFixture {
    fn id(self) -> &'static str {
        match self {
            Self::General => "general_sem_pls",
            Self::FrequencyExpansionUnweighted => "frequency_expansion_unweighted_pls",
            Self::MultipleTwoWay => "multiple_two_way",
            Self::ThreeWay => "bounded_three_way",
            Self::ModeratedMediation => "bounded_two_way_moderated_mediation",
            Self::MultipleHoc => "multiple_nonnested_hoc",
            Self::CaseWeighted => "case_weighted_pls",
            Self::FrequencyWeighted => "frequency_weighted_pls",
            Self::ReflectivePlsc => "reflective_plsc",
            Self::GeneralParametric => "general_sem_pls_parametric_sensitivity",
        }
    }

    fn profile(self) -> MgaModelProfileV1 {
        match self {
            Self::General | Self::GeneralParametric | Self::FrequencyExpansionUnweighted => {
                MgaModelProfileV1::GeneralSemPls
            }
            Self::MultipleTwoWay => MgaModelProfileV1::MultipleTwoWayModeration,
            Self::ThreeWay => MgaModelProfileV1::BoundedThreeWayModeration,
            Self::ModeratedMediation => MgaModelProfileV1::BoundedTwoWayModeratedMediation,
            Self::MultipleHoc => MgaModelProfileV1::MultipleNonnestedHoc,
            Self::CaseWeighted => MgaModelProfileV1::CaseWeightedPls,
            Self::FrequencyWeighted => MgaModelProfileV1::FrequencyWeightedPls,
            Self::ReflectivePlsc => MgaModelProfileV1::ReflectivePlsc,
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

struct MgaFixture {
    dataset: qpls_data::Dataset,
    groups: Vec<SelectedGroupV1>,
    design: MultigroupDesignV1,
    exclusions: Vec<ExcludedRowReceiptV1>,
    x1: Vec<f64>,
    y1: Vec<f64>,
    weights: Vec<f64>,
    source_weights: Vec<f64>,
}

fn group_value_text(encoding: GroupEncoding, group: usize) -> String {
    match encoding {
        GroupEncoding::Text => format!("G{:02}", group + 1),
        GroupEncoding::Integer => (group as i64 + 1).to_string(),
        GroupEncoding::Number => format!("{:.1}", group as f64 + 0.5),
        GroupEncoding::Boolean => (group == 0).to_string(),
    }
}

fn core_group_value(encoding: GroupEncoding, group: usize) -> qpls_core::TypedGroupValueV1 {
    match encoding {
        GroupEncoding::Text => qpls_core::TypedGroupValueV1::Text {
            value: group_value_text(encoding, group),
        },
        GroupEncoding::Integer => qpls_core::TypedGroupValueV1::Integer {
            value: group as i64 + 1,
        },
        GroupEncoding::Number => qpls_core::TypedGroupValueV1::Number {
            value: group as f64 + 0.5,
        },
        GroupEncoding::Boolean => qpls_core::TypedGroupValueV1::Boolean { value: group == 0 },
    }
}

fn estimation_group_value(
    encoding: GroupEncoding,
    group: usize,
) -> Result<qpls_estimation::TypedGroupValueV1, DynError> {
    Ok(match encoding {
        GroupEncoding::Text => qpls_estimation::TypedGroupValueV1::Text {
            value: group_value_text(encoding, group),
        },
        GroupEncoding::Integer => qpls_estimation::TypedGroupValueV1::Integer {
            value: group as i64 + 1,
        },
        GroupEncoding::Number => {
            qpls_estimation::TypedGroupValueV1::finite_number(group as f64 + 0.5)?
        }
        GroupEncoding::Boolean => qpls_estimation::TypedGroupValueV1::Boolean { value: group == 0 },
    })
}

fn make_mga_fixture(
    cell_id: &str,
    group_count: usize,
    rows_per_group: usize,
    encoding: GroupEncoding,
    frequency_weights: bool,
) -> Result<MgaFixture, DynError> {
    if encoding == GroupEncoding::Boolean && group_count != 2 {
        return Err(invalid("boolean fixture supports exactly two groups"));
    }
    let selected = group_count * rows_per_group;
    let mut group_column = Vec::<Option<String>>::new();
    let mut latent = BTreeMap::<&str, Vec<f64>>::from([
        ("x", Vec::new()),
        ("z", Vec::new()),
        ("w", Vec::new()),
        ("m", Vec::new()),
        ("a", Vec::new()),
        ("b", Vec::new()),
        ("c", Vec::new()),
        ("d", Vec::new()),
        ("y", Vec::new()),
    ]);
    let mut weights = Vec::new();
    for group in 0..group_count {
        for row in 0..rows_per_group {
            let position = (group * rows_per_group + row) as f64 + 1.0;
            let x = (0.173 * position).sin() + 0.009 * position;
            let z = (0.211 * position).cos() - 0.004 * position;
            let w = (0.137 * position).sin() + 0.17 * (0.071 * position).cos();
            let noise = 0.035 * (0.617 * position).sin() + 0.019 * (0.293 * position).cos();
            let m = 0.58 * x + 0.18 * z + (0.05 + group as f64 * 0.006) * x * z + noise;
            let a = 0.75 * x + 0.20 * z + 0.5 * noise;
            let b = 0.62 * x - 0.23 * w - 0.4 * noise;
            let c = 0.57 * z + 0.24 * w + 0.3 * noise;
            let d = -0.46 * x + 0.51 * w - 0.2 * noise;
            let slope = -0.35 + 0.07 * group as f64;
            let y =
                slope * x + 0.21 * z - 0.13 * w + 0.16 * m + (0.08 + 0.01 * group as f64) * x * z
                    - 0.06 * x * w
                    + 0.05 * z * w
                    + 0.04 * x * z * w
                    + 0.17 * a
                    + 0.14 * b
                    - 0.11 * c
                    + 0.09 * d
                    + noise;
            group_column.push(Some(group_value_text(encoding, group)));
            for (id, value) in [
                ("x", x),
                ("z", z),
                ("w", w),
                ("m", m),
                ("a", a),
                ("b", b),
                ("c", c),
                ("d", d),
                ("y", y),
            ] {
                latent.get_mut(id).expect("known latent").push(value);
            }
            weights.push(if frequency_weights {
                (1 + row % 3) as f64
            } else {
                0.55 + (1 + row % 9) as f64 / 5.0
            });
        }
    }

    // Three source rows are intentionally unusable and remain outside the
    // scientific design: an unselected level, a missing group, and a selected
    // group with one missing model value.
    let extra_start = group_column.len();
    group_column.extend([
        Some(match encoding {
            GroupEncoding::Text | GroupEncoding::Boolean => "UNSELECTED".into(),
            GroupEncoding::Integer | GroupEncoding::Number => "9999".into(),
        }),
        None,
        Some(group_value_text(encoding, 0)),
    ]);
    for values in latent.values_mut() {
        let tail = *values.last().expect("selected rows exist");
        values.extend([tail + 0.1, tail + 0.2, tail + 0.3]);
    }
    weights.extend([1.0, 1.0, 1.0]);

    let mut headers = vec!["group".to_string()];
    let mut columns = vec![group_column];
    for id in ["x", "z", "w", "m", "a", "b", "c", "d", "y"] {
        let values = &latent[id];
        for indicator in 1..=3 {
            let mut observed = values
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    let perturbation =
                        0.013 * ((row * (indicator + 3) + indicator) as f64 * 0.37).sin();
                    Some(format!(
                        "{:.17}",
                        value * (0.82 + 0.09 * indicator as f64) + perturbation
                    ))
                })
                .collect::<Vec<_>>();
            if id == "x" && indicator == 1 {
                observed[extra_start + 2] = None;
            }
            headers.push(format!("{id}{indicator}"));
            columns.push(observed);
        }
    }
    headers.push("weight".into());
    columns.push(numeric(weights.clone()));
    let total_rows = columns
        .first()
        .map(Vec::len)
        .ok_or_else(|| invalid("MGA fixture omitted every source column"))?;
    let (headers, columns) =
        metamorphic::transformed_columns_v1(&headers, &columns).map_err(invalid)?;
    let dataset = dataset_from_columns(&format!("{cell_id}.csv"), &headers, &columns)?;
    let groups = (0..group_count)
        .map(|group| SelectedGroupV1 {
            group_id: format!("group_{:02}", group + 1),
            label: format!("Group {}", group + 1),
            value: core_group_value(encoding, group),
        })
        .collect::<Vec<_>>();
    let identities = (0..group_count)
        .map(|group| {
            Ok(GroupIdentityV1 {
                index: GroupIndexV1::new(group)?,
                value: estimation_group_value(encoding, group)?,
                display_label: format!("Group {}", group + 1),
            })
        })
        .collect::<Result<Vec<_>, DynError>>()?;
    let rows = (0..selected)
        .map(|row| {
            Ok(SelectedGroupRowV1 {
                source_row: metamorphic::mapped_source_row_v1(row, total_rows).map_err(invalid)?
                    as u64,
                group: GroupIndexV1::new(row / rows_per_group).expect("bounded group index"),
            })
        })
        .collect::<Result<Vec<_>, DynError>>()?;
    let design = MultigroupDesignV1 {
        groups: identities,
        rows,
    };
    let exclusions = vec![
        ExcludedRowReceiptV1 {
            stable_row_token: format!(
                "source_row:{}",
                metamorphic::mapped_source_row_v1(extra_start, total_rows).map_err(invalid)?
            ),
            typed_group_value: "unselected".into(),
            reason: ExcludedRowReasonV1::UnselectedGroupValue,
        },
        ExcludedRowReceiptV1 {
            stable_row_token: format!(
                "source_row:{}",
                metamorphic::mapped_source_row_v1(extra_start + 1, total_rows).map_err(invalid)?
            ),
            typed_group_value: "missing".into(),
            reason: ExcludedRowReasonV1::MissingGroupValue,
        },
        ExcludedRowReceiptV1 {
            stable_row_token: format!(
                "source_row:{}",
                metamorphic::mapped_source_row_v1(extra_start + 2, total_rows).map_err(invalid)?
            ),
            typed_group_value: groups[0].value.canonical_key(),
            reason: ExcludedRowReasonV1::MissingModelValue,
        },
    ];
    let mut source_weights = weights.clone();
    metamorphic::transform_row_aligned_values_v1(&mut source_weights);
    Ok(MgaFixture {
        dataset,
        groups,
        design,
        exclusions,
        x1: latent["x"][..selected].to_vec(),
        y1: latent["y"][..selected].to_vec(),
        weights: weights[..selected].to_vec(),
        source_weights,
    })
}

fn indicators<'a>(id: &'a str, count: usize) -> Vec<String> {
    (1..=count).map(|index| format!("{id}{index}")).collect()
}

fn build_profile_recipe(
    fixture: &MgaFixture,
    profile: ProfileFixture,
    seed: u64,
) -> Result<(AnalysisRecipeV4, SemModelV4, Vec<String>), DynError> {
    let owned = ["x", "z", "w", "m", "a", "b", "c", "d", "y"]
        .into_iter()
        .map(|id| {
            let count = match profile {
                ProfileFixture::General | ProfileFixture::GeneralParametric if id != "x" => 3,
                ProfileFixture::ReflectivePlsc | ProfileFixture::MultipleHoc => 3,
                _ => 1,
            };
            (id, indicators(id, count))
        })
        .collect::<BTreeMap<_, _>>();
    let construct =
        |id: &'static str| (id, owned[id].iter().map(String::as_str).collect::<Vec<_>>());
    let (construct_ids, paths): (Vec<&str>, Vec<(&str, &str)>) = match profile {
        ProfileFixture::General | ProfileFixture::GeneralParametric => {
            (vec!["x", "z", "y"], vec![("x", "y"), ("z", "y")])
        }
        ProfileFixture::FrequencyExpansionUnweighted
        | ProfileFixture::CaseWeighted
        | ProfileFixture::FrequencyWeighted
        | ProfileFixture::ReflectivePlsc => (vec!["x", "y"], vec![("x", "y")]),
        ProfileFixture::MultipleTwoWay | ProfileFixture::ThreeWay => (
            vec!["x", "z", "w", "y"],
            vec![("x", "y"), ("z", "y"), ("w", "y")],
        ),
        ProfileFixture::ModeratedMediation => (
            vec!["x", "z", "m", "y"],
            vec![("x", "m"), ("z", "m"), ("x", "y"), ("m", "y")],
        ),
        ProfileFixture::MultipleHoc => (
            vec!["x", "z", "w", "m", "a", "b", "c", "d", "y"],
            Vec::new(),
        ),
    };
    let buffers = construct_ids
        .iter()
        .map(|id| construct(id))
        .collect::<Vec<_>>();
    let borrowed = buffers
        .iter()
        .map(|(id, values)| (*id, values.as_slice()))
        .collect::<Vec<_>>();
    let (mut recipe, mut model) = base_recipe_model(
        &fixture.dataset,
        0x4d47_4100_0000_0000_u128 + profile as u128,
        profile.id(),
        &borrowed,
        &paths,
        seed,
    )?;
    let levels = fixture
        .groups
        .iter()
        .map(|group| {
            let value = match &group.value {
                qpls_core::TypedGroupValueV1::Text { value } => value.clone(),
                qpls_core::TypedGroupValueV1::Integer { value } => value.to_string(),
                qpls_core::TypedGroupValueV1::Number { value } => value.to_string(),
                qpls_core::TypedGroupValueV1::Boolean { value } => value.to_string(),
            };
            (group.group_id.clone(), value, group.label.clone())
        })
        .collect::<Vec<_>>();
    add_groups(&mut model, "group", &levels);
    match profile {
        ProfileFixture::General | ProfileFixture::GeneralParametric => {
            make_formative_composite(&mut model, "construct:z")?;
            add_control_relation(
                &mut model,
                "observed:qualification_control",
                "w1",
                "construct:y",
            );
        }
        ProfileFixture::MultipleTwoWay => {
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
        ProfileFixture::ThreeWay => {
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
        ProfileFixture::ModeratedMediation => add_interaction(
            &mut model,
            "interaction:x_by_z_to_m",
            &["construct:x", "construct:z"],
            "construct:x",
            "construct:m",
        )?,
        ProfileFixture::MultipleHoc => {
            add_disjoint_hoc(
                &mut model,
                "hoc_ab",
                &["construct:a", "construct:b"],
                "construct:y",
            );
            add_disjoint_hoc(
                &mut model,
                "hoc_cd",
                &["construct:c", "construct:d"],
                "construct:y",
            );
            add_disjoint_hoc(
                &mut model,
                "hoc_xz",
                &["construct:x", "construct:z"],
                "construct:y",
            );
            add_disjoint_hoc(
                &mut model,
                "hoc_wm",
                &["construct:w", "construct:m"],
                "construct:y",
            );
        }
        ProfileFixture::CaseWeighted => add_weight_binding(&mut model, "weight", false)?,
        ProfileFixture::FrequencyWeighted => add_weight_binding(&mut model, "weight", true)?,
        ProfileFixture::FrequencyExpansionUnweighted | ProfileFixture::ReflectivePlsc => {}
    }
    let selected = match profile {
        ProfileFixture::General => Vec::new(),
        ProfileFixture::GeneralParametric
        | ProfileFixture::FrequencyExpansionUnweighted
        | ProfileFixture::CaseWeighted
        | ProfileFixture::FrequencyWeighted
        | ProfileFixture::ReflectivePlsc => {
            vec![relation_parameter(&model, "construct:x", "construct:y")?]
        }
        _ => Vec::new(),
    };
    finalize_recipe(&mut recipe, &model)?;
    Ok((recipe, model, selected))
}

fn procedures(group_count: usize, profile: ProfileFixture) -> Vec<MgaProcedureV1> {
    let mut values = vec![
        MgaProcedureV1::MicomPairwise,
        MgaProcedureV1::PairwisePermutation,
        MgaProcedureV1::HenselerPlsMga,
        MgaProcedureV1::BootstrapDifferenceBc,
    ];
    if group_count >= 3 {
        values.push(MgaProcedureV1::OmnibusMaxSpreadPermutation);
    }
    if profile == ProfileFixture::GeneralParametric {
        values.extend([
            MgaProcedureV1::ParametricPooledVariance,
            MgaProcedureV1::ParametricWelchSatterthwaite,
        ]);
        if group_count >= 3 {
            values.push(MgaProcedureV1::ParametricWaldOmnibus);
        }
    }
    values
}

fn summarize_mga_evidence(evidence: &[MultiModRunnerEvidenceV1], retain_bootstrap: bool) -> Value {
    let mut pairwise = Vec::new();
    let mut omnibus = Vec::new();
    let mut bootstrap = Vec::new();
    let mut micom = Vec::new();
    let mut parametric = Vec::new();
    let mut parametric_group_se = Vec::new();
    let mut wald = Vec::new();
    for row in evidence {
        match row {
            MultiModRunnerEvidenceV1::MgaPairwisePermutation(value) => {
                let canonical_first_pair = {
                    let left = value.pair.group_a.get();
                    let right = value.pair.group_b.get();
                    left.min(right) == 0 && left.max(right) == 1
                };
                let audit_null = canonical_first_pair
                    .then(|| value.parameters.first())
                    .flatten()
                    .map(|parameter| json!({
                        "parameter": parameter.parameter,
                        "observed_difference_a_minus_b": parameter.difference_a_minus_b,
                        "null_differences": parameter.null_differences,
                        "null_differences_sha256": sha256_f64_series(&parameter.null_differences),
                    }));
                let parameters = value
                    .parameters
                    .iter()
                    .map(|parameter| json!({
                        "parameter": parameter.parameter,
                        "estimate_a": parameter.estimate_a,
                        "estimate_b": parameter.estimate_b,
                        "difference_a_minus_b": parameter.difference_a_minus_b,
                        "p_value_two_sided": parameter.p_value_two_sided,
                        "p_value_greater": parameter.p_value_greater,
                        "p_value_less": parameter.p_value_less,
                        "selected_alternative": parameter.selected_alternative,
                        "selected_probability": parameter.selected_probability,
                        "null_differences_sha256": sha256_f64_series(&parameter.null_differences),
                    }))
                    .collect::<Vec<_>>();
                pairwise.push(json!({
                    "method_version": value.method_version,
                    "pair": value.pair,
                    "seed": value.seed,
                    "requested": value.requested,
                    "attempted": value.attempted,
                    "usable": value.usable,
                    "failed": value.failed,
                    "minimum_usable": value.minimum_usable,
                    "retry_policy": value.retry_policy,
                    "plan_sha256": value.plan_sha256,
                    "availability": value.availability,
                    "point_estimates": value.point_estimates,
                    "parameters": parameters,
                    "audit_null_difference": audit_null,
                    "ledger_partition_list_sha256": sha256_serialized(
                        &value.ledger.iter().map(|item| &item.partition_sha256).collect::<Vec<_>>()
                    ),
                }));
            }
            MultiModRunnerEvidenceV1::MgaOmnibusPermutation(value) => omnibus.push(json!({
                "method_version": value.method_version,
                "seed": value.seed,
                "requested": value.requested,
                "attempted": value.attempted,
                "usable": value.usable,
                "failed": value.failed,
                "minimum_usable": value.minimum_usable,
                "retry_policy": value.retry_policy,
                "plan_sha256": value.plan_sha256,
                "availability": value.availability,
                "group_point_estimates": value.group_point_estimates,
                "parameters": value.parameters,
            })),
            MultiModRunnerEvidenceV1::MgaBootstrapBanks(value) => bootstrap.push(json!({
                "method_version": value.method_version,
                "seed": value.seed,
                "requested": value.requested,
                "attempted": value.attempted,
                "minimum_usable": value.minimum_usable,
                "retry_policy": value.retry_policy,
                "plan_sha256": value.plan_sha256,
                "availability": value.availability,
                "parameters": value.parameters,
                "groups": value.groups.iter().map(|group| json!({
                    "group": group.group,
                    "point_estimates": group.point_estimates,
                    "usable": group.usable,
                    "failed": group.failed,
                    "replicate_estimates": retain_bootstrap.then_some(&group.replicate_estimates),
                })).collect::<Vec<_>>(),
            })),
            MultiModRunnerEvidenceV1::MgaMicomPair(value) => {
                let canonical_first_pair = {
                    let left = value.pair.group_a.get();
                    let right = value.pair.group_b.get();
                    left.min(right) == 0 && left.max(right) == 1
                };
                let audit_step2 = canonical_first_pair
                    .then(|| value.constructs.first())
                    .flatten()
                    .map(|construct| json!({
                        "construct_id": construct.construct_id,
                        "observed_compositional_correlation": construct.observed_compositional_correlation,
                        "permutation_compositional_correlations": construct.permutation_compositional_correlations,
                        "observed_mean_difference_a_minus_b": construct.observed_mean_difference_a_minus_b,
                        "permutation_mean_differences": construct.permutation_mean_differences,
                        "observed_log_variance_ratio_a_minus_b": construct.observed_log_variance_ratio_a_minus_b,
                        "permutation_log_variance_ratios": construct.permutation_log_variance_ratios,
                        "permutation_values_sha256": sha256_f64_series(
                            &construct.permutation_compositional_correlations
                        ),
                        "permutation_mean_differences_sha256": sha256_f64_series(
                            &construct.permutation_mean_differences
                        ),
                        "permutation_log_variance_ratios_sha256": sha256_f64_series(
                            &construct.permutation_log_variance_ratios
                        ),
                    }));
                let constructs = value
                    .constructs
                    .iter()
                    .map(|construct| json!({
                        "construct_id": construct.construct_id,
                        "observed_compositional_correlation": construct.observed_compositional_correlation,
                        "compositional_lower_quantile": construct.compositional_lower_quantile,
                        "compositional_invariance_probability": construct.compositional_invariance_probability,
                        "compositional_invariance": construct.compositional_invariance,
                        "observed_mean_difference_a_minus_b": construct.observed_mean_difference_a_minus_b,
                        "mean_difference_two_sided_probability": construct.mean_difference_two_sided_probability,
                        "equal_means": construct.equal_means,
                        "observed_log_variance_ratio_a_minus_b": construct.observed_log_variance_ratio_a_minus_b,
                        "variance_difference_two_sided_probability": construct.variance_difference_two_sided_probability,
                        "equal_variances": construct.equal_variances,
                        "partial_measurement_invariance": construct.partial_measurement_invariance,
                        "full_measurement_invariance": construct.full_measurement_invariance,
                        "permutation_compositional_correlations_sha256": sha256_f64_series(
                            &construct.permutation_compositional_correlations
                        ),
                        "permutation_mean_differences_sha256": sha256_f64_series(
                            &construct.permutation_mean_differences
                        ),
                        "permutation_log_variance_ratios_sha256": sha256_f64_series(
                            &construct.permutation_log_variance_ratios
                        ),
                    }))
                    .collect::<Vec<_>>();
                micom.push(json!({
                    "method_version": value.method_version,
                    "pair": value.pair,
                    "configural_receipt": value.configural_receipt,
                    "requested_permutations": value.requested_permutations,
                    "usable_permutations": value.usable_permutations,
                    "minimum_usable_permutations": value.minimum_usable_permutations,
                    "partition_plan_sha256": value.partition_plan_sha256,
                    "ledger_sha256": value.ledger_sha256,
                    "constructs": constructs,
                    "audit_step2": audit_step2,
                    "complete": value.complete,
                }));
            }
            MultiModRunnerEvidenceV1::MgaPairwiseParametric(value) => parametric
                .push(serde_json::to_value(value).expect("serializable parametric evidence")),
            MultiModRunnerEvidenceV1::MgaOrdinaryPlsPathStandardError {
                parameter,
                group,
                receipt,
            } => parametric_group_se.push(json!({
                "parameter": parameter,
                "group": group,
                "receipt": receipt,
            })),
            MultiModRunnerEvidenceV1::MgaParametricWald(value) => {
                wald.push(serde_json::to_value(value).expect("serializable Wald evidence"))
            }
            _ => {}
        }
    }
    json!({
        "pairwise_permutation": pairwise,
        "omnibus_permutation": omnibus,
        "bootstrap_banks": bootstrap,
        "micom": micom,
        "parametric_group_se": parametric_group_se,
        "parametric": parametric,
        "wald": wald,
    })
}

fn multiplicity_replays(analysis: &PlsMultigroupAnalysisV1) -> Result<Value, DynError> {
    let hypotheses = analysis
        .pairwise
        .iter()
        .filter_map(|row| {
            row.raw_p_value
                .map(|raw_probability| HypothesisProbabilityV1 {
                    hypothesis_id: format!(
                        "{}:{}:{}:{}",
                        row.procedure, row.left_group_id, row.right_group_id, row.target_id
                    ),
                    raw_probability,
                })
        })
        .collect::<Vec<_>>();
    let methods = [
        MultiplicityMethodV1::Holm,
        MultiplicityMethodV1::Bonferroni,
        MultiplicityMethodV1::Sidak,
        MultiplicityMethodV1::BenjaminiHochberg,
        MultiplicityMethodV1::None,
    ];
    let rows = methods
        .into_iter()
        .map(|method| {
            Ok(json!({
                "method": method,
                "probabilities": adjust_probabilities_v1(&hypotheses, method)?,
            }))
        })
        .collect::<Result<Vec<_>, MultigroupKernelErrorV1>>()?;
    Ok(json!({"hypotheses": hypotheses, "methods": rows}))
}

fn run_cell(
    cell_id: &str,
    profile: ProfileFixture,
    group_count: usize,
    rows_per_group: usize,
    encoding: GroupEncoding,
    seed: u64,
    comparison_plan: MgaComparisonPlanV1,
    retain_bootstrap: bool,
) -> Result<Value, DynError> {
    let fixture = make_mga_fixture(
        cell_id,
        group_count,
        rows_per_group,
        encoding,
        profile == ProfileFixture::FrequencyWeighted,
    )?;
    run_fixture_cell(
        cell_id,
        profile,
        group_count,
        rows_per_group,
        encoding,
        seed,
        comparison_plan,
        retain_bootstrap,
        &fixture,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_fixture_cell(
    cell_id: &str,
    profile: ProfileFixture,
    group_count: usize,
    rows_per_group: usize,
    encoding: GroupEncoding,
    seed: u64,
    comparison_plan: MgaComparisonPlanV1,
    retain_bootstrap: bool,
    fixture: &MgaFixture,
) -> Result<Value, DynError> {
    let (mut recipe, mut model, selected_parameter_ids) =
        build_profile_recipe(fixture, profile, seed)?;
    recipe.settings.workers =
        metamorphic::configured_workers_v1(recipe.settings.workers).map_err(invalid)?;
    metamorphic::transform_model_declaration_order_v1(&mut model);
    stage_additive_multimod_recipe(&mut recipe, AnalysisMethod::Mga);
    recipe.mga_multigroup = Some(MgaMultigroupV1 {
        schema_version: MGA_MULTIGROUP_V1_SCHEMA_VERSION,
        profile: profile.profile(),
        grouping_column: "group".into(),
        groups: fixture.groups.clone(),
        comparison_plan,
        procedures: procedures(group_count, profile),
        permutation_samples: PERMUTATIONS,
        bootstrap_samples: BOOTSTRAPS,
        seed,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: InferenceAlternativeV1::TwoSided,
        multiplicity: MultiplicityAdjustmentV1::Holm,
        configural_checklist: MicomConfiguralChecklistV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_sign_orientation_reviewed: true,
            analyst_review_confirmed: true,
        },
        weight: match profile {
            ProfileFixture::CaseWeighted => Some(AnalysisWeightBindingV1::Case {
                column: "weight".into(),
            }),
            ProfileFixture::FrequencyWeighted => Some(AnalysisWeightBindingV1::Frequency {
                column: "weight".into(),
            }),
            _ => None,
        },
        selected_parameter_ids,
    });
    finalize_recipe(&mut recipe, &model)?;
    let artifact = prepare_multimod_recipe_v1(
        &fixture.dataset,
        &recipe,
        &model,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let execution_plan = prepare_compiled_raw_mga_execution_plan_v1(
        &fixture.dataset,
        &recipe,
        &model,
        &artifact,
        &fixture.design,
    )?;
    let mut cache = MgaExecutionCacheV1::empty(&execution_plan)?;
    let run = run_compiled_raw_mga_resumable_v1(
        &fixture.dataset,
        &recipe,
        &model,
        &artifact,
        &fixture.design,
        &fixture.exclusions,
        &mut cache,
        || false,
        |_| {},
    )?;
    let MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis) = &run.output.result else {
        return Err(invalid("raw MGA runner returned the wrong result family"));
    };
    let cancel_resume = if metamorphic::compact_matrix_v1()
        && metamorphic::metamorphism_v1() == "baseline"
        && cell_id == "mga-general-2-groups"
    {
        Some(cancel_resume_receipt(
            &fixture.dataset,
            &recipe,
            &model,
            &artifact,
            &fixture.design,
            &fixture.exclusions,
            &run,
        )?)
    } else {
        None
    };
    let expected_pairs = group_count * (group_count - 1) / 2;
    Ok(json!({
        "cell_id": cell_id,
        "profile": profile.profile(),
        "profile_fixture": profile.id(),
        "group_count": group_count,
        "rows_per_group": rows_per_group,
        "dataset_rows": fixture.dataset.batch.num_rows(),
        "dataset_fingerprint": fixture.dataset.fingerprint.0.clone(),
        "typed_group_encoding": format!("{encoding:?}").to_lowercase(),
        "expected_pair_count": expected_pairs,
        "heavy_run_confirmed": group_count == 20,
        "compiler_receipt": &run.output.compilation_receipt,
        "compiled_plan": artifact.plan(),
        "sem_model_authority": &model,
        "execution_plan": &run.execution_plan,
        "finalized_cache_sha256": &run.finalized_cache_sha256,
        "cancel_resume": cancel_resume,
        "analysis": analysis,
        "evidence": summarize_mga_evidence(&run.output.evidence, retain_bootstrap),
        "multiplicity_replays": multiplicity_replays(analysis)?,
        "raw_reference": {
            "x": &fixture.x1,
            "y": &fixture.y1,
            "weight": &fixture.weights,
            "rows_by_group": (0..group_count).map(|group| {
                (group * rows_per_group..(group + 1) * rows_per_group).collect::<Vec<_>>()
            }).collect::<Vec<_>>(),
        },
    }))
}

#[allow(clippy::too_many_arguments)]
fn cancel_resume_receipt(
    dataset: &qpls_data::Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    exclusions: &[ExcludedRowReceiptV1],
    uninterrupted: &ResumableMgaRunV1,
) -> Result<Value, DynError> {
    let plan =
        prepare_compiled_raw_mga_execution_plan_v1(dataset, recipe, model, artifact, design)?;
    let mut cache = MgaExecutionCacheV1::empty(&plan)?;
    let cancel = AtomicBool::new(false);
    let interrupted_checkpoints = AtomicUsize::new(0);
    let interrupted = run_compiled_raw_mga_resumable_with_checkpoint_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        exclusions,
        &mut cache,
        || cancel.load(Ordering::SeqCst),
        |_| {},
        |checkpoint_plan, checkpoint_cache| {
            checkpoint_cache
                .ensure_valid(checkpoint_plan)
                .map_err(|error| error.to_string())?;
            interrupted_checkpoints.fetch_add(1, Ordering::SeqCst);
            cancel.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    let cancelled_without_result = matches!(interrupted, Err(MultiModRunnerErrorV1::Cancelled));
    let completed_before_resume = cache.entries.len();
    let partial_result_unpublishable = cache.finalized_identity_sha256(&plan).is_err();
    cancel.store(false, Ordering::SeqCst);
    let resumed_checkpoints = AtomicUsize::new(0);
    let resumed = run_compiled_raw_mga_resumable_with_checkpoint_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        exclusions,
        &mut cache,
        || false,
        |_| {},
        |checkpoint_plan, checkpoint_cache| {
            checkpoint_cache
                .ensure_valid(checkpoint_plan)
                .map_err(|error| error.to_string())?;
            resumed_checkpoints.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )?;
    let expected_resume_checkpoints = plan.shards.len().saturating_sub(completed_before_resume);
    Ok(json!({
        "receipt_id": "qpls.multimod.mga-production-cancel-resume.v1",
        "planned_shards": plan.shards.len(),
        "completed_before_resume": completed_before_resume,
        "interrupted_checkpoints": interrupted_checkpoints.load(Ordering::SeqCst),
        "resume_checkpoints": resumed_checkpoints.load(Ordering::SeqCst),
        "cancelled_without_result": cancelled_without_result,
        "partial_result_unpublishable": partial_result_unpublishable,
        "completed_cache_exact": cache.entries.len() == plan.shards.len(),
        "completed_shards_reused_without_retry": resumed_checkpoints.load(Ordering::SeqCst)
            == expected_resume_checkpoints,
        "finalized_cache_matches_uninterrupted": resumed.finalized_cache_sha256
            == uninterrupted.finalized_cache_sha256,
        "complete_result_matches_uninterrupted": resumed.output.result
            == uninterrupted.output.result,
        "complete_evidence_matches_uninterrupted": resumed.output.evidence
            == uninterrupted.output.evidence,
        "compilation_receipt_matches_uninterrupted": resumed.output.compilation_receipt
            == uninterrupted.output.compilation_receipt,
    }))
}

fn boundary_receipts(seed: u64) -> Result<Value, DynError> {
    let minimum = make_mga_fixture("mga-boundary-minimum", 2, 10, GroupEncoding::Text, false)?;
    let mut too_small = minimum.design.clone();
    too_small
        .rows
        .retain(|row| !(row.group.get() == 1 && row.source_row == 19));
    let warning = make_mga_fixture("mga-boundary-warning", 2, 10, GroupEncoding::Text, false)?;
    let mut imbalance = warning.design.clone();
    let group_zero = imbalance
        .rows
        .iter()
        .filter(|row| row.group.get() == 0)
        .cloned()
        .collect::<Vec<_>>();
    for repeat in 0..10 {
        imbalance
            .rows
            .extend(group_zero.iter().cloned().map(|mut row| {
                row.source_row += ((repeat + 1) * 100) as u64;
                row
            }));
    }
    let heavy = MgaMultigroupV1 {
        schema_version: MGA_MULTIGROUP_V1_SCHEMA_VERSION,
        profile: MgaModelProfileV1::GeneralSemPls,
        grouping_column: "group".into(),
        groups: (0..20)
            .map(|group| SelectedGroupV1 {
                group_id: format!("g{group}"),
                label: format!("G{group}"),
                value: qpls_core::TypedGroupValueV1::Text {
                    value: format!("G{group}"),
                },
            })
            .collect(),
        comparison_plan: MgaComparisonPlanV1::AllPairs {
            heavy_run_confirmed: false,
        },
        procedures: vec![
            MgaProcedureV1::OmnibusMaxSpreadPermutation,
            MgaProcedureV1::PairwisePermutation,
        ],
        permutation_samples: PERMUTATIONS,
        bootstrap_samples: BOOTSTRAPS,
        seed,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: InferenceAlternativeV1::TwoSided,
        multiplicity: MultiplicityAdjustmentV1::Holm,
        configural_checklist: MicomConfiguralChecklistV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_sign_orientation_reviewed: true,
            analyst_review_confirmed: true,
        },
        weight: None,
        selected_parameter_ids: vec!["parameter:x:y".into()],
    };
    let directional = [
        InferenceAlternativeV1::Less,
        InferenceAlternativeV1::Greater,
    ]
    .into_iter()
    .map(|alternative| {
        let mut candidate = heavy.clone();
        candidate.groups.truncate(2);
        candidate.comparison_plan = MgaComparisonPlanV1::AllPairs {
            heavy_run_confirmed: false,
        };
        candidate.procedures = vec![MgaProcedureV1::PairwisePermutation];
        candidate.alternative = alternative;
        json!({"alternative": alternative, "validation": candidate.ensure_valid().err()})
    })
    .collect::<Vec<_>>();
    Ok(json!({
        "minimum_complete_cases": assess_multigroup_design_v1(&too_small),
        "ten_case_boundary": assess_multigroup_design_v1(&minimum.design),
        "below_thirty_warning": assess_multigroup_design_v1(&warning.design),
        "above_ten_to_one": assess_multigroup_design_v1(&imbalance),
        "heavy_run_without_confirmation": heavy.ensure_valid().err(),
        "directional_predeclarations": directional,
    }))
}

fn label_reversal(seed: u64) -> Result<Value, DynError> {
    let forward = run_cell(
        "mga-label-forward",
        ProfileFixture::General,
        2,
        30,
        GroupEncoding::Text,
        seed,
        MgaComparisonPlanV1::AllPairs {
            heavy_run_confirmed: false,
        },
        true,
    )?;
    // Reversal is a scientific relabeling: the raw rows are identical while
    // the ordered group identities and design indices swap.
    let actual_reverse = run_cell(
        "mga-label-reverse",
        ProfileFixture::General,
        2,
        30,
        GroupEncoding::Text,
        seed,
        MgaComparisonPlanV1::SelectedPairs {
            pairs: vec![GroupPairV1 {
                left_group_id: "group_02".into(),
                right_group_id: "group_01".into(),
            }],
        },
        true,
    )?;
    Ok(json!({"forward": forward, "reverse": actual_reverse}))
}

fn frequency_expansion(seed: u64) -> Result<Value, DynError> {
    let compact_fixture =
        make_mga_fixture("mga-frequency-compact", 2, 15, GroupEncoding::Text, true)?;
    let comparison = MgaComparisonPlanV1::AllPairs {
        heavy_run_confirmed: false,
    };
    let compact = run_fixture_cell(
        "mga-frequency-compact",
        ProfileFixture::FrequencyWeighted,
        2,
        15,
        GroupEncoding::Text,
        seed,
        comparison.clone(),
        true,
        &compact_fixture,
    )?;

    let source_rows = qpls_data::preview_page(
        &compact_fixture.dataset,
        0,
        compact_fixture.dataset.batch.num_rows(),
    );
    let headers = compact_fixture
        .dataset
        .schema
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    let mut expanded_columns = headers
        .iter()
        .map(|_| Vec::<Option<String>>::new())
        .collect::<Vec<_>>();
    let mut expanded_design_rows = Vec::new();
    let mut expanded_x = Vec::new();
    let mut expanded_y = Vec::new();
    let mut next_row = 0_u64;
    for selected in &compact_fixture.design.rows {
        let source = selected.source_row as usize;
        let count = compact_fixture.source_weights[source] as usize;
        for _ in 0..count {
            for (column, header) in headers.iter().enumerate() {
                let value = if header == "weight" {
                    Some("1".into())
                } else {
                    source_rows[source].get(header).cloned().flatten()
                };
                expanded_columns[column].push(value);
            }
            expanded_design_rows.push(SelectedGroupRowV1 {
                source_row: next_row,
                group: selected.group,
            });
            expanded_x.push(compact_fixture.x1[source]);
            expanded_y.push(compact_fixture.y1[source]);
            next_row += 1;
        }
    }
    let expanded_dataset = dataset_from_columns(
        "mga-frequency-physically-expanded.csv",
        &headers,
        &expanded_columns,
    )?;
    let expanded_fixture = MgaFixture {
        dataset: expanded_dataset,
        groups: compact_fixture.groups.clone(),
        design: MultigroupDesignV1 {
            groups: compact_fixture.design.groups.clone(),
            rows: expanded_design_rows,
        },
        exclusions: Vec::new(),
        x1: expanded_x,
        y1: expanded_y,
        weights: vec![1.0; next_row as usize],
        source_weights: vec![1.0; next_row as usize],
    };
    let expanded = run_fixture_cell(
        "mga-frequency-physically-expanded",
        ProfileFixture::FrequencyExpansionUnweighted,
        2,
        30,
        GroupEncoding::Text,
        seed,
        comparison,
        true,
        &expanded_fixture,
    )?;
    Ok(json!({
        "compact_frequency_run": compact,
        "physically_expanded_unweighted_run": expanded,
        "compact_source_rows": 30,
        "represented_rows": next_row,
        "equivalence_oracle": "production_count_space_vs_physical_expansion_v1",
    }))
}

fn main() -> Result<(), DynError> {
    let args = arguments()?;
    let compact = metamorphic::compact_matrix_v1();
    let retain_full_bootstrap = args.scale == Scale::Qualification && !compact;
    let mut group_matrix = Vec::new();
    let group_cases = if compact {
        vec![(2, GroupEncoding::Text)]
    } else {
        match args.scale {
            Scale::Development => vec![(2, GroupEncoding::Text), (3, GroupEncoding::Integer)],
            Scale::Qualification => vec![
                (2, GroupEncoding::Text),
                (3, GroupEncoding::Integer),
                (5, GroupEncoding::Number),
                (20, GroupEncoding::Text),
            ],
        }
    };
    for (group_count, encoding) in group_cases {
        group_matrix.push(run_cell(
            &format!("mga-general-{group_count}-groups"),
            ProfileFixture::General,
            group_count,
            if group_count == 20 { 10 } else { 30 },
            encoding,
            args.seed,
            MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: group_count == 20,
            },
            group_count == 2 || retain_full_bootstrap,
        )?);
    }
    let profile_cases = if compact || args.scale == Scale::Qualification {
        vec![
            ProfileFixture::MultipleTwoWay,
            ProfileFixture::ThreeWay,
            ProfileFixture::ModeratedMediation,
            ProfileFixture::MultipleHoc,
            ProfileFixture::CaseWeighted,
            ProfileFixture::ReflectivePlsc,
        ]
    } else {
        vec![
            ProfileFixture::MultipleTwoWay,
            ProfileFixture::ReflectivePlsc,
        ]
    };
    let profile_matrix = profile_cases
        .into_iter()
        .map(|profile| {
            run_cell(
                &format!("mga-profile-{}", profile.id()),
                profile,
                2,
                30,
                GroupEncoding::Text,
                args.seed,
                MgaComparisonPlanV1::AllPairs {
                    heavy_run_confirmed: false,
                },
                retain_full_bootstrap,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parametric_sensitivity = if compact {
        json!({"status": "outside_compact_metamorphic_matrix"})
    } else {
        run_cell(
            "mga-general-parametric-3-groups",
            ProfileFixture::GeneralParametric,
            3,
            30,
            GroupEncoding::Integer,
            args.seed,
            MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            retain_full_bootstrap,
        )?
    };
    let label_reversal = if !compact || metamorphic::metamorphism_v1() == "baseline" {
        label_reversal(args.seed)?
    } else {
        json!({"status": "verified_once_on_baseline_production_fixture"})
    };
    let report = json!({
        "schema_version": SCHEMA_VERSION,
        "suite_id": SUITE_ID,
        "scale": match args.scale { Scale::Development => "development", Scale::Qualification => "qualification" },
        "seed": args.seed,
        "metamorphism": metamorphic::metamorphism_v1(),
        "workers": metamorphic::configured_workers_v1(1).map_err(invalid)?,
        "execution_contract": "public_recipe_v4_compiler_plus_raw_resumable_mga_runner",
        "qualification_claim": "raw_sut_facts_for_independent_comparison_only",
        "required_profile_ids": [
            "mga.general_sem_pls.v1",
            "mga.multiple_two_way_moderation.v1",
            "mga.bounded_three_way_moderation.v1",
            "mga.bounded_two_way_moderated_mediation.v1",
            "mga.multiple_nonnested_hoc.v1",
            "mga.case_weighted_pls.v1",
            "mga.frequency_weighted_pls.v1",
            "mga.reflective_plsc.v1"
        ],
        "group_matrix": group_matrix,
        "parametric_sensitivity": parametric_sensitivity,
        "profile_matrix": profile_matrix,
        "frequency_expansion": frequency_expansion(args.seed)?,
        "label_reversal": label_reversal,
        "boundaries": boundary_receipts(args.seed)?,
    });
    fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
