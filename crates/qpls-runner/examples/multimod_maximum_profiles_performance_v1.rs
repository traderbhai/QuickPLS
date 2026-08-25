//! Real maximum-target, archive-boundary, and resumable-execution performance cells.
//!
//! The qualification wrapper owns wall-clock and process-tree memory metrics.
//! This producer executes the same public compiler and raw runner boundaries
//! used by native jobs; it contains no alternate estimator or mock result.

#[path = "support_multimod_qualification/mod.rs"]
mod support;

use qpls_core::*;
use qpls_estimation::{
    GroupIdentityV1, GroupIndexV1, MultigroupDesignV1, SelectedGroupRowV1,
    TypedGroupValueV1 as EstimationGroupValueV1,
};
use qpls_project::{
    encode_multimod_arrow_sidecar_v1, multimod_micom_null_statistics_batch_v1,
    validate_multimod_sidecar_total_bytes_v1,
};
use qpls_runner::*;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use support::*;

struct Arguments {
    output: PathBuf,
    seed: u64,
}

fn arguments() -> Result<Arguments, DynError> {
    let mut output = None;
    let mut seed = 42_u64;
    let mut values = env::args().skip(1);
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--output" => output = values.next().map(PathBuf::from),
            "--seed" => {
                seed = values
                    .next()
                    .ok_or_else(|| invalid("--seed requires a value"))?
                    .parse()?;
            }
            _ => return Err(invalid(format!("unknown argument {argument}"))),
        }
    }
    Ok(Arguments {
        output: output.ok_or_else(|| invalid("--output is required"))?,
        seed,
    })
}

fn construct_id(id: &str) -> String {
    format!("construct:{id}")
}

fn maximum_conditional(seed: u64) -> Result<serde_json::Value, DynError> {
    const ROWS: usize = 80;
    const PATHS: usize = 8;
    let moderator_names = ["z0", "z1", "z2", "z3"];
    let mut headers = Vec::new();
    let mut columns = Vec::new();
    let mut construct_specs = Vec::<(String, Vec<String>)>::new();
    let mut structural_paths = Vec::<(String, String)>::new();

    for moderator in moderator_names {
        let values = (0..ROWS).map(|row| {
            let t = row as f64 + 1.0;
            (t * (0.071 + moderator.as_bytes()[1] as f64 * 0.0007)).sin() + 0.31 * (t * 0.137).cos()
        });
        let indicator = format!("{moderator}_i");
        headers.push(indicator.clone());
        columns.push(numeric(values));
        construct_specs.push((moderator.into(), vec![indicator]));
    }
    for path_index in 0..PATHS {
        let x = format!("x{path_index}");
        let m = format!("m{path_index}");
        let y = format!("y{path_index}");
        let z = moderator_names[path_index % moderator_names.len()];
        let mut x_values = Vec::with_capacity(ROWS);
        let mut m_values = Vec::with_capacity(ROWS);
        let mut y_values = Vec::with_capacity(ROWS);
        for row in 0..ROWS {
            let t = row as f64 + 1.0;
            let xv = (t * (0.101 + path_index as f64 * 0.003)).sin() + 0.19 * (t * 0.047).cos();
            let zv =
                (t * (0.071 + z.as_bytes()[1] as f64 * 0.0007)).sin() + 0.31 * (t * 0.137).cos();
            let noise = 0.025 * (t * (0.331 + path_index as f64 * 0.002)).sin();
            let mv = 0.56 * xv + 0.14 * zv + (0.18 + 0.01 * path_index as f64) * xv * zv + noise;
            let yv = 0.63 * mv + 0.11 * xv - noise;
            x_values.push(xv);
            m_values.push(mv);
            y_values.push(yv);
        }
        for (id, values) in [(&x, x_values), (&m, m_values), (&y, y_values)] {
            let indicator = format!("{id}_i");
            headers.push(indicator.clone());
            columns.push(numeric(values));
            construct_specs.push((id.clone(), vec![indicator]));
        }
        structural_paths.push((x.clone(), m.clone()));
        structural_paths.push((z.into(), m.clone()));
        structural_paths.push((m, y));
    }
    let dataset = dataset_from_columns("conditional-maximum-targets-v1.csv", &headers, &columns)?;
    let construct_buffers = construct_specs
        .iter()
        .map(|(id, indicators)| {
            (
                id.as_str(),
                indicators.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let constructs = construct_buffers
        .iter()
        .map(|(id, indicators)| (*id, indicators.as_slice()))
        .collect::<Vec<_>>();
    let paths = structural_paths
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_str()))
        .collect::<Vec<_>>();
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0x5045_5246_434f_4e44_0000_0000_0000_0001,
        "conditional maximum target performance",
        &constructs,
        &paths,
        seed,
    )?;
    let mut selected_paths = Vec::new();
    let mut interactions = Vec::new();
    for path_index in 0..PATHS {
        let x = construct_id(&format!("x{path_index}"));
        let m = construct_id(&format!("m{path_index}"));
        let y = construct_id(&format!("y{path_index}"));
        let z = construct_id(moderator_names[path_index % moderator_names.len()]);
        let interaction = format!("interaction:maximum:{path_index}");
        add_interaction(&mut model, &interaction, &[&x, &z], &x, &m)?;
        interactions.push(interaction);
        selected_paths.push(ConditionalProcessPathV2 {
            path_id: format!("maximum_path_{path_index}"),
            ordered_relation_ids: vec![relation_id(&model, &x, &m)?, relation_id(&model, &m, &y)?],
        });
    }
    let probe_values = [
        vec![-1.5, -0.5, 0.5, 1.5],
        vec![-1.5, -0.5, 0.5, 1.5],
        vec![-1.0, 1.0],
        vec![-1.0, 1.0],
    ];
    let moderator_ids = moderator_names
        .iter()
        .map(|id| construct_id(id))
        .collect::<Vec<_>>();
    let probes = moderator_ids
        .iter()
        .zip(probe_values)
        .map(|(moderator_id, values)| ConditionalModeratorProbeV2 {
            probe_id: format!("probe:{moderator_id}"),
            moderator_id: moderator_id.clone(),
            scale: ConditionalProbeScaleV2::StandardizedScore,
            values,
            raw_transformation_receipt: None,
            raw_fit_metric_receipts: Vec::new(),
        })
        .collect();
    recipe.settings.method = AnalysisMethod::ModeratedMediation;
    recipe.method_config = None;
    recipe.general_sem_conditional_process = Some(GeneralSemConditionalProcessConfigV2 {
        schema_version: GENERAL_SEM_CONDITIONAL_PROCESS_V2_SCHEMA_VERSION,
        profile: ConditionalProcessProfileV2::MultiTwoWayPercentile,
        paths: selected_paths,
        declared_interaction_ids: interactions,
        three_way_interaction_id: None,
        hoc_ids: Vec::new(),
        moderator_ids,
        probes,
        explicit_joint_tuples: Vec::new(),
        probe_contrasts: Vec::new(),
        grouping_column: None,
        groups: Vec::new(),
        group_contrasts: Vec::new(),
        weight: None,
        estimands: ConditionalProcessEstimandsV2 {
            conditional_specific_indirect: true,
            conditional_total_indirect: true,
            conditional_total_effect: false,
            scalar_index_when_affine: false,
            local_first_derivatives: false,
            local_second_and_cross_derivatives: false,
            finite_probe_contrasts: false,
        },
        inference: ConditionalProcessInferenceV2 {
            interval: ConditionalProcessIntervalV2::Percentile,
            alternative: InferenceAlternativeV1::TwoSided,
            outer_resamples: 500,
            inner_resamples: 0,
            seed,
            confidence_level: 0.95,
        },
    });
    finalize_recipe(&mut recipe, &model)?;
    let artifact = prepare_multimod_recipe_v1(
        &dataset,
        &recipe,
        &model,
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2,
    )?;
    let run = run_compiled_general_sem_conditional_process_raw_builtin_v2(
        &dataset,
        &recipe,
        &model,
        &artifact,
        || false,
        |_| {},
    )?;
    let MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(analysis) =
        &run.output.result
    else {
        return Err(invalid(
            "maximum conditional runner returned the wrong family",
        ));
    };
    if analysis.targets.len() != 1_024 || analysis.replicate_ledger.requested != 500 {
        return Err(invalid(format!(
            "maximum conditional inventory is {} targets / {} draws, expected 1024 / 500",
            analysis.targets.len(),
            analysis.replicate_ledger.requested
        )));
    }
    Ok(json!({
        "workload_id": "qpls.v256.multimod.performance.conditional-maximum-target-grid.v1",
        "production_compiler_target": artifact.receipt().target,
        "dataset_fingerprint": dataset.fingerprint.0,
        "model_scientific_sha256": model.scientific_sha256()?,
        "recipe_analytical_sha256": artifact.receipt().recipe_analytical_sha256,
        "path_count": 8,
        "moderator_count": 4,
        "cartesian_probe_tuple_count": 64,
        "conditional_cell_count": 512,
        "inferential_target_count": analysis.targets.len(),
        "requested_resamples": analysis.replicate_ledger.requested,
        "usable_resamples": analysis.replicate_ledger.usable,
        "result_identity_sha256": sha256_serialized(&run.output.result),
        "production_raw_runner_completed": true,
    }))
}

fn resumable_mga(seed: u64) -> Result<serde_json::Value, DynError> {
    let rows_per_group = 15usize;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut groups = Vec::new();
    for group in 0..2 {
        for row in 0..rows_per_group {
            let t = (group * rows_per_group + row) as f64 + 1.0;
            let xv = (t * 0.173).sin() + 0.01 * t;
            let yv = (-0.25 + 0.8 * group as f64) * xv + 0.04 * (t * 0.419).cos();
            x.push(xv);
            y.push(yv);
            groups.push(format!("G{}", group + 1));
        }
    }
    let headers = ["x1", "x2", "y1", "y2", "group"]
        .map(str::to_owned)
        .to_vec();
    let dataset = dataset_from_columns(
        "mga-resume-performance-v1.csv",
        &headers,
        &[
            numeric(x.iter().copied()),
            numeric(x.iter().map(|value| 0.93 * value + 0.01)),
            numeric(y.iter().copied()),
            numeric(y.iter().map(|value| 0.94 * value - 0.01)),
            text(groups),
        ],
    )?;
    let (mut recipe, mut model) = base_recipe_model(
        &dataset,
        0x5045_5246_4d47_4100_0000_0000_0000_0002,
        "MGA deterministic resume performance",
        &[("x", &["x1", "x2"]), ("y", &["y1", "y2"])],
        &[("x", "y")],
        seed,
    )?;
    let selected_groups = (0..2)
        .map(|group| SelectedGroupV1 {
            group_id: format!("g{}", group + 1),
            label: format!("Group {}", group + 1),
            value: qpls_core::TypedGroupValueV1::Text {
                value: format!("G{}", group + 1),
            },
        })
        .collect::<Vec<_>>();
    let levels = selected_groups
        .iter()
        .map(|group| {
            let qpls_core::TypedGroupValueV1::Text { value } = &group.value else {
                unreachable!()
            };
            (group.group_id.clone(), value.clone(), group.label.clone())
        })
        .collect::<Vec<_>>();
    add_groups(&mut model, "group", &levels);
    stage_additive_multimod_recipe(&mut recipe, AnalysisMethod::Mga);
    recipe.mga_multigroup = Some(MgaMultigroupV1 {
        schema_version: MGA_MULTIGROUP_V1_SCHEMA_VERSION,
        profile: MgaModelProfileV1::GeneralSemPls,
        grouping_column: "group".into(),
        groups: selected_groups,
        comparison_plan: MgaComparisonPlanV1::AllPairs {
            heavy_run_confirmed: false,
        },
        procedures: vec![MgaProcedureV1::PairwisePermutation],
        permutation_samples: 5_000,
        bootstrap_samples: 5_000,
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
        selected_parameter_ids: Vec::new(),
    });
    finalize_recipe(&mut recipe, &model)?;
    let artifact = prepare_multimod_recipe_v1(
        &dataset,
        &recipe,
        &model,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let design = MultigroupDesignV1 {
        groups: (0..2)
            .map(|group| {
                Ok(GroupIdentityV1 {
                    index: GroupIndexV1::new(group)?,
                    value: EstimationGroupValueV1::Text {
                        value: format!("G{}", group + 1),
                    },
                    display_label: format!("Group {}", group + 1),
                })
            })
            .collect::<Result<Vec<_>, DynError>>()?,
        rows: (0..(2 * rows_per_group))
            .map(|row| SelectedGroupRowV1 {
                source_row: row as u64,
                stable_row_token: row as u64,
                group: GroupIndexV1::new(row / rows_per_group).expect("bounded group"),
            })
            .collect(),
    };
    let plan =
        prepare_compiled_raw_mga_execution_plan_v1(&dataset, &recipe, &model, &artifact, &design)?;
    let mut cancelled_cache = MgaExecutionCacheV1::empty(&plan)?;
    let cancel = AtomicBool::new(false);
    let interrupted = run_compiled_raw_mga_resumable_with_checkpoint_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        &design,
        &[],
        &mut cancelled_cache,
        || cancel.load(Ordering::Acquire),
        |_| {},
        |_, _| {
            cancel.store(true, Ordering::Release);
            Ok(())
        },
    );
    if !matches!(interrupted, Err(MultiModRunnerErrorV1::Cancelled))
        || cancelled_cache.entries.is_empty()
    {
        return Err(invalid(
            "MGA did not cancel after a complete resumable shard",
        ));
    }
    let serialized_cache = serde_json::to_vec(&cancelled_cache)?;
    let mut reopened_cache: MgaExecutionCacheV1 = serde_json::from_slice(&serialized_cache)?;
    reopened_cache.ensure_valid(&plan)?;
    let resumed = run_compiled_raw_mga_resumable_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        &design,
        &[],
        &mut reopened_cache,
        || false,
        |_| {},
    )?;
    let mut fresh_cache = MgaExecutionCacheV1::empty(&plan)?;
    let fresh = run_compiled_raw_mga_resumable_v1(
        &dataset,
        &recipe,
        &model,
        &artifact,
        &design,
        &[],
        &mut fresh_cache,
        || false,
        |_| {},
    )?;
    if resumed.finalized_cache_sha256 != fresh.finalized_cache_sha256
        || sha256_serialized(&resumed.output.result) != sha256_serialized(&fresh.output.result)
    {
        return Err(invalid("resumed MGA differs from uninterrupted execution"));
    }
    Ok(json!({
        "workload_id": "qpls.v256.multimod.performance.mga-cancel-resume.v1",
        "execution_plan_sha256": plan.plan_sha256,
        "execution_shard_count": plan.shards.len(),
        "cancelled_completed_shard_count": cancelled_cache.entries.len(),
        "serialized_cache_bytes": serialized_cache.len(),
        "finalized_cache_sha256": resumed.finalized_cache_sha256,
        "result_identity_sha256": sha256_serialized(&resumed.output.result),
        "cancelled_result_suppressed": true,
        "cache_strict_reopen_validated": true,
        "resumed_equals_uninterrupted": true,
    }))
}

fn descriptor(bytes: u64, suffix: &str) -> MultimodResultSidecarDescriptorV1 {
    MultimodResultSidecarDescriptorV1 {
        schema_version: MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION,
        entry_name: format!("results/performance/{suffix}.arrow"),
        evidence_role: format!("performance:{suffix}"),
        arrow_schema_contract_id: "qpls.performance.boundary.v1".into(),
        arrow_schema_contract_version: 1,
        media_type: "application/vnd.apache.arrow.stream".into(),
        compression: "zip_deflate".into(),
        arrow_schema_sha256: "1".repeat(64),
        row_count: 1,
        column_count: 1,
        uncompressed_bytes: bytes,
        sha256: "2".repeat(64),
        identity_sha256: "3".repeat(64),
        required_for_scientific_reopen: true,
    }
}

fn sidecar_boundaries() -> Result<serde_json::Value, DynError> {
    let warning = MULTIMOD_SIDECAR_WARN_BYTES_V1;
    let maximum = MULTIMOD_SIDECAR_MAX_BYTES_V1;
    let exact_total = validate_multimod_sidecar_total_bytes_v1(&[
        descriptor(maximum / 2, "left"),
        descriptor(maximum - maximum / 2, "right"),
    ])?;
    let over_total = validate_multimod_sidecar_total_bytes_v1(&[
        descriptor(maximum, "maximum"),
        descriptor(1, "overflow"),
    ]);
    if exact_total != maximum || over_total.is_ok() {
        return Err(invalid(
            "archive sidecar aggregate boundary did not fail closed",
        ));
    }
    let micom_rows = 25_000_u32;
    let micom_batch = multimod_micom_null_statistics_batch_v1(
        (0..micom_rows).map(|row| row / 5).collect(),
        (0..micom_rows)
            .map(|row| if row % 5 < 3 { row % 5 } else { 0 })
            .collect(),
        (0..micom_rows)
            .map(|row| if row % 5 < 3 { 0 } else { (row % 5 - 2) as u8 })
            .collect(),
        (0..micom_rows)
            .map(|row| f64::from(row) / 10_000.0)
            .collect(),
    )?;
    let micom_payload = encode_multimod_arrow_sidecar_v1(
        "result:maximum-profile-micom-boundary",
        "mga-micom-pair-null-statistics.arrow",
        &"a".repeat(64),
        "mga-micom-pair:null-statistics",
        &micom_batch,
    )?;
    let micom_predicted = predict_mga_micom_null_statistics_arrow_bytes_v1(u64::from(micom_rows));
    if micom_payload.descriptor.uncompressed_bytes > micom_predicted {
        return Err(invalid(
            "trusted compact MICOM Arrow bytes exceeded the conservative preflight",
        ));
    }
    Ok(json!({
        "workload_id": "qpls.v256.multimod.performance.archive-sidecar-boundaries.v1",
        "warning_bytes": warning,
        "maximum_bytes": maximum,
        "cost_states": {
            "warning_exact": multimod_sidecar_cost_state_v1(warning),
            "warning_plus_one": multimod_sidecar_cost_state_v1(warning + 1),
            "maximum_exact": multimod_sidecar_cost_state_v1(maximum),
            "maximum_plus_one": multimod_sidecar_cost_state_v1(maximum + 1)
        },
        "aggregate_maximum_admitted_bytes": exact_total,
        "aggregate_maximum_plus_one_rejected": true,
        "micom_null_statistics": {
            "representation": "construct_ordinal_plus_statistic_kind_v1",
            "rows": micom_rows,
            "actual_uncompressed_bytes": micom_payload.descriptor.uncompressed_bytes,
            "predicted_uncompressed_bytes": micom_predicted,
            "prediction_bounds_actual": true,
        },
        "production_archive_validator_executed": true,
    }))
}

fn main() -> Result<(), DynError> {
    let args = arguments()?;
    let report = json!({
        "schema_version": 1,
        "suite_id": "qpls.v256.multimod.maximum-profiles-production-performance.v1",
        "seed": args.seed,
        "conditional_maximum": maximum_conditional(args.seed)?,
        "mga_cancellation_resume": resumable_mga(args.seed ^ 0x4d47_4152_4553_554d)?,
        "archive_sidecar_boundaries": sidecar_boundaries()?,
    });
    fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
