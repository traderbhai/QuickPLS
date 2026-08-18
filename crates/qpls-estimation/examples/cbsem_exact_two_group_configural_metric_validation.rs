use qpls_core::{
    Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, ObservedRoleV4,
    ObservedScaleV4, SemDataBindingV4, SemEndpointV4, SemGroupLevelV4, SemGroupV4, SemModelV4,
    SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, compile_cbsem_plan_v2,
    convert_legacy_basic_model_v4,
};
use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
use qpls_estimation::estimate_cbsem_ml_exact_two_group_configural_metric_v1;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};

const KIND: &str = "cbsem_exact_two_group_configural_metric_engine_reference_v2";
const STATUS: &str = "engine_reference_generated_product_qualification_blocked";
const PRIMARY_DATASET_UUID_U128: u128 = 0xCB5E_2A01_0000_0000_0000_0000_0000_0001;
const LABEL_SWAP_DATASET_UUID_U128: u128 = 0xCB5E_2A01_0000_0000_0000_0000_0000_0002;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn load_dataset(
    path: &Path,
    source_name: &str,
    dataset_uuid_u128: u128,
) -> Result<(Dataset, String), String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let digest = sha256(&bytes);
    let mut dataset = import_delimited_bytes(&bytes, source_name, b',', &ImportOptions::default())
        .map_err(|error| error.to_string())?;
    // Import intentionally creates a fresh identity. Replace it immediately
    // with a distinct frozen authority before any model or plan is compiled.
    dataset.id = uuid::Uuid::from_u128(dataset_uuid_u128);
    Ok((dataset, digest))
}

fn grouped_two_factor_model(dataset: &Dataset) -> Result<SemModelV4, String> {
    let legacy = ModelSpec {
        id: uuid::Uuid::from_u128(0xCB5E_2002),
        name: "Two-group correlated two-factor CFA reference".into(),
        constructs: vec![
            Construct {
                id: "f1".into(),
                name: "Factor 1".into(),
                short_name: "F1".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            },
            Construct {
                id: "f2".into(),
                name: "Factor 2".into(),
                short_name: "F2".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["y1".into(), "y2".into(), "y3".into()],
            },
        ],
        paths: Vec::new(),
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    let mut model = convert_legacy_basic_model_v4(
        &legacy,
        LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        &[],
    )
    .map_err(|error| error.to_string())?;
    let left = SemEndpointV4::Variable("construct:f1".into());
    let right = SemEndpointV4::Variable("construct:f2".into());
    let covariance_parameter = "reference_factor_covariance_f1_f2".to_string();
    model.relations.push(SemRelationV4::Covariance {
        id: "reference_covariance_f1_f2".into(),
        left: left.clone(),
        right: right.clone(),
        parameter: covariance_parameter.clone(),
    });
    model.parameters.push(SemParameterV4::Free {
        id: covariance_parameter,
        label: "Cov(F1,F2)".into(),
        target: SemParameterTargetV4::Covariance { left, right },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    model.variables.push(SemVariableV4::Observed {
        id: "observed:group".into(),
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
        grouping_variable: "observed:group".into(),
        levels: vec![
            SemGroupLevelV4 {
                id: "a".into(),
                value: "A".into(),
                label: "Group A".into(),
            },
            SemGroupLevelV4 {
                id: "b".into(),
                value: "B".into(),
                label: "Group B".into(),
            },
        ],
    };
    model.data_binding = SemDataBindingV4::Raw {
        dataset_id: dataset.id.to_string(),
        missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
        weight: None,
        cluster_variable: None,
        strata_variable: None,
    };
    model.ensure_valid().map_err(|error| error.to_string())?;
    Ok(model)
}

fn fit(dataset: &Dataset) -> Result<qpls_estimation::CbsemExactTwoGroupInvarianceResultV1, String> {
    let model = grouped_two_factor_model(dataset)?;
    let plan = compile_cbsem_plan_v2(&model).map_err(|error| error.to_string())?;
    estimate_cbsem_ml_exact_two_group_configural_metric_v1(dataset, &plan, &model)
        .map_err(|error| error.to_string())
}

fn main() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let primary_path = args
        .next()
        .ok_or("usage: example <primary.csv> <label-swap.csv>")?;
    let swap_path = args
        .next()
        .ok_or("usage: example <primary.csv> <label-swap.csv>")?;
    if args.next().is_some() {
        return Err("usage: example <primary.csv> <label-swap.csv>".into());
    }
    let primary_path = Path::new(&primary_path);
    let swap_path = Path::new(&swap_path);
    let (primary_dataset, primary_sha256) = load_dataset(
        primary_path,
        "cbsem_exact_two_group_configural_metric_v1.csv",
        PRIMARY_DATASET_UUID_U128,
    )?;
    let (swap_dataset, swap_sha256) = load_dataset(
        swap_path,
        "cbsem_exact_two_group_configural_metric_v1_label_swap.csv",
        LABEL_SWAP_DATASET_UUID_U128,
    )?;
    if primary_dataset.id == swap_dataset.id {
        return Err("primary and label-swap dataset UUID authorities must differ".into());
    }
    let primary = fit(&primary_dataset)?;
    let label_swap = fit(&swap_dataset)?;
    let configural_difference =
        (primary.configural.chi_square - label_swap.configural.chi_square).abs();
    let metric_difference = (primary.metric.chi_square - label_swap.metric.chi_square).abs();
    let lrt_difference = (primary.nesting.likelihood_ratio_statistic
        - label_swap.nesting.likelihood_ratio_statistic)
        .abs();
    let tolerance = 1.0e-7;
    let dimensions_match = primary.configural.free_dimensions == 26
        && primary.metric.free_dimensions == 22
        && primary.configural.degrees_of_freedom == 16
        && primary.metric.degrees_of_freedom == 20
        && primary.nesting.delta_degrees_of_freedom == 4
        && label_swap.configural.free_dimensions == 26
        && label_swap.metric.free_dimensions == 22
        && label_swap.configural.degrees_of_freedom == 16
        && label_swap.metric.degrees_of_freedom == 20
        && label_swap.nesting.delta_degrees_of_freedom == 4;
    let label_swap_accepted = configural_difference <= tolerance
        && metric_difference <= tolerance
        && lrt_difference <= tolerance;
    if !dimensions_match || !label_swap_accepted {
        return Err(format!(
            "engine reference contract failed: dimensions_match={dimensions_match}, label_swap_accepted={label_swap_accepted}, observed primary D/df={}/{}, {}/{}, delta={}",
            primary.configural.free_dimensions,
            primary.configural.degrees_of_freedom,
            primary.metric.free_dimensions,
            primary.metric.degrees_of_freedom,
            primary.nesting.delta_degrees_of_freedom,
        ));
    }
    let output = json!({
        "schema_version": 2,
        "kind": KIND,
        "status": STATUS,
        "qualification_boundary": {
            "status": "blocked_not_product_qualification",
            "permitted_use": "engine_reference_only",
            "forbidden_claims": ["product_support", "canonical_export_support", "release_qualification"]
        },
        "source_binding": {
            "primary_path": primary_path.to_string_lossy(),
            "primary_sha256": primary_sha256,
            "primary_dataset_uuid": primary_dataset.id.to_string(),
            "label_swap_path": swap_path.to_string_lossy(),
            "label_swap_sha256": swap_sha256,
            "label_swap_dataset_uuid": swap_dataset.id.to_string()
        },
        "expected_dimensions": {
            "configural_free_dimensions": 26,
            "configural_degrees_of_freedom": 16,
            "metric_free_dimensions": 22,
            "metric_degrees_of_freedom": 20,
            "delta_degrees_of_freedom": 4
        },
        "primary": primary,
        "label_swap": label_swap,
        "label_swap_check": {
            "status": "accepted",
            "absolute_tolerance": tolerance,
            "configural_chi_square_absolute_difference": configural_difference,
            "metric_chi_square_absolute_difference": metric_difference,
            "lrt_absolute_difference": lrt_difference
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}
