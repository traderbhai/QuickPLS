//! Shared fixture construction for the MultiMod scientific qualification producers.
//!
//! Only deterministic observed columns are synthesized here. Every scientific
//! estimate is produced by the public Recipe V4 compiler and raw-data runner.

use chrono::{TimeZone, Utc};
use qpls_core::*;
use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
use std::collections::BTreeMap;
use std::error::Error;
use uuid::Uuid;

pub type DynError = Box<dyn Error + Send + Sync>;

pub fn invalid(message: impl Into<String>) -> DynError {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}

pub fn dataset_from_columns(
    source_name: &str,
    headers: &[String],
    columns: &[Vec<Option<String>>],
) -> Result<Dataset, DynError> {
    let rows = columns
        .first()
        .map(Vec::len)
        .ok_or_else(|| invalid("fixture has no columns"))?;
    if headers.len() != columns.len() || columns.iter().any(|column| column.len() != rows) {
        return Err(invalid("fixture columns have inconsistent dimensions"));
    }
    let mut csv = headers.join(",") + "\n";
    for row in 0..rows {
        for (column, values) in columns.iter().enumerate() {
            if column > 0 {
                csv.push(',');
            }
            if let Some(value) = &values[row] {
                if value
                    .chars()
                    .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
                {
                    csv.push('"');
                    csv.push_str(&value.replace('"', "\"\""));
                    csv.push('"');
                } else {
                    csv.push_str(value);
                }
            }
        }
        csv.push('\n');
    }
    Ok(import_delimited_bytes(
        csv.as_bytes(),
        source_name,
        b',',
        &ImportOptions::default(),
    )?)
}

pub fn numeric(values: impl IntoIterator<Item = f64>) -> Vec<Option<String>> {
    values
        .into_iter()
        .map(|value| Some(format!("{value:.17}")))
        .collect()
}

pub fn text(values: impl IntoIterator<Item = String>) -> Vec<Option<String>> {
    values.into_iter().map(Some).collect()
}

pub fn base_recipe_model(
    dataset: &Dataset,
    fixture_id: u128,
    name: &str,
    constructs: &[(&str, &[&str])],
    paths: &[(&str, &str)],
    seed: u64,
) -> Result<(AnalysisRecipeV4, SemModelV4), DynError> {
    let source_model = ModelSpec {
        id: Uuid::from_u128(fixture_id),
        name: name.into(),
        constructs: constructs
            .iter()
            .map(|(id, indicators)| Construct {
                id: (*id).into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: indicators.iter().map(|value| (*value).into()).collect(),
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
    };
    let source = AnalysisRecipe {
        schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
        id: Uuid::from_u128(fixture_id ^ 0x4d55_4c54_494d_4f44),
        created_at: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .ok_or_else(|| invalid("fixed fixture timestamp is invalid"))?,
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        model: source_model.clone(),
        settings: AnalysisSettings {
            method: AnalysisMethod::PlsPm,
            bootstrap_samples: 0,
            permutation_samples: 0,
            seed,
            confidence_level: 0.95,
            bootstrap_test_tail: PlsBootstrapTestTail::TwoSided,
            studentized_inner_samples: 0,
            workers: 1,
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
    let SemDataBindingV4::Raw {
        dataset_id,
        missing_data,
        ..
    } = &mut model.data_binding
    else {
        return Err(invalid("migrated fixture did not retain raw data binding"));
    };
    *dataset_id = dataset.id.to_string();
    *missing_data = MissingDataPolicyV4::ListwiseDeletion;
    recipe.settings.bootstrap_samples = 0;
    recipe.settings.permutation_samples = 0;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.seed = seed;
    recipe.settings.workers = 1;
    recipe.general_sem_config = Some(GeneralSemConfigV1::default());
    Ok((recipe, model))
}

/// Mirrors the production staging boundary before a qualification fixture
/// attaches exactly one additive MultiMod configuration. The legacy recipe is
/// used only to exercise ModelSpec -> SemModelV4 conversion; neither its
/// method identity nor a previously staged MultiMod request may leak forward.
pub fn stage_additive_multimod_recipe(recipe: &mut AnalysisRecipeV4, method: AnalysisMethod) {
    recipe.settings.method = method;
    recipe.method_config = None;
    recipe.mga_multigroup = None;
    recipe.pls_heterogeneity = None;
    recipe.general_sem_conditional_process = None;
    recipe.interventional_causal_mediation = None;
}

pub fn finalize_recipe(recipe: &mut AnalysisRecipeV4, model: &SemModelV4) -> Result<(), DynError> {
    model.ensure_valid()?;
    recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: model.scientific_sha256()?,
        model: model.clone(),
    };
    recipe.ensure_valid()?;
    Ok(())
}

pub fn add_observed_control(
    model: &mut SemModelV4,
    id: &str,
    source_column: &str,
    label: &str,
    scale: ObservedScaleV4,
    categories: Vec<String>,
) {
    model.variables.push(SemVariableV4::Observed {
        id: id.into(),
        label: label.into(),
        source_column: source_column.into(),
        scale,
        role: ObservedRoleV4::Control,
        categories,
        value_labels: BTreeMap::new(),
        missing_markers: Vec::new(),
        transformation_lineage: Vec::new(),
    });
}

pub fn add_control_relation(
    model: &mut SemModelV4,
    observed_id: &str,
    source_column: &str,
    outcome: &str,
) {
    add_observed_control(
        model,
        observed_id,
        source_column,
        source_column,
        ObservedScaleV4::Continuous,
        Vec::new(),
    );
    let relation = format!("relation:{observed_id}:to:{outcome}");
    let parameter = format!("parameter:{observed_id}:to:{outcome}");
    model.relations.push(SemRelationV4::Structural {
        id: relation,
        source: observed_id.into(),
        target: outcome.into(),
        parameter: parameter.clone(),
        role: StructuralRelationRoleV4::Control,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter,
        label: format!("{source_column} -> {outcome}"),
        target: SemParameterTargetV4::Regression {
            source: observed_id.into(),
            target: outcome.into(),
        },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
}

/// Converts one migrated Mode-A composite and all of its measurement
/// authorities to a formative Mode-B block without changing stable IDs.
pub fn make_formative_composite(
    model: &mut SemModelV4,
    construct_id: &str,
) -> Result<(), DynError> {
    let variable = model
        .variables
        .iter_mut()
        .find(|variable| variable.id() == construct_id)
        .ok_or_else(|| invalid(format!("missing composite {construct_id}")))?;
    let SemVariableV4::Composite { weighting, .. } = variable else {
        return Err(invalid(format!("{construct_id} is not a composite")));
    };
    *weighting = CompositeWeightingV4::ModeB;

    for relation in &mut model.relations {
        let replacement = match relation {
            SemRelationV4::MeasurementEffect {
                id,
                construct,
                indicator,
                parameter,
            } if construct == construct_id => Some(SemRelationV4::MeasurementCausal {
                id: id.clone(),
                indicator: indicator.clone(),
                composite: construct.clone(),
                parameter: parameter.clone(),
            }),
            _ => None,
        };
        if let Some(replacement) = replacement {
            *relation = replacement;
        }
    }
    for parameter in &mut model.parameters {
        let replacement = match parameter {
            SemParameterV4::Free {
                id,
                label,
                target:
                    SemParameterTargetV4::Loading {
                        construct,
                        indicator,
                    },
                start,
                lower,
                upper,
                equality_label,
                group_overrides,
            } if construct == construct_id => Some(SemParameterV4::Free {
                id: id.clone(),
                label: label.clone(),
                target: SemParameterTargetV4::Weight {
                    indicator: indicator.clone(),
                    composite: construct.clone(),
                },
                start: *start,
                lower: *lower,
                upper: *upper,
                equality_label: equality_label.clone(),
                group_overrides: group_overrides.clone(),
            }),
            _ => None,
        };
        if let Some(replacement) = replacement {
            *parameter = replacement;
        }
    }
    Ok(())
}

pub fn add_groups(
    model: &mut SemModelV4,
    grouping_column: &str,
    levels: &[(String, String, String)],
) {
    add_observed_control(
        model,
        "observed:multimod_group",
        grouping_column,
        "Qualification group",
        ObservedScaleV4::Nominal,
        levels.iter().map(|(_, value, _)| value.clone()).collect(),
    );
    model.group = SemGroupV4::ObservedGroups {
        grouping_variable: "observed:multimod_group".into(),
        levels: levels
            .iter()
            .map(|(id, value, label)| SemGroupLevelV4 {
                id: id.clone(),
                value: value.clone(),
                label: label.clone(),
            })
            .collect(),
    };
}

pub fn add_weight_binding(
    model: &mut SemModelV4,
    source_column: &str,
    frequency: bool,
) -> Result<(), DynError> {
    add_observed_control(
        model,
        "observed:multimod_weight",
        source_column,
        "Qualification weight",
        ObservedScaleV4::Continuous,
        Vec::new(),
    );
    let SemDataBindingV4::Raw { weight, .. } = &mut model.data_binding else {
        return Err(invalid("weight fixture requires raw data"));
    };
    *weight = Some(if frequency {
        SemWeightBindingV4::Frequency {
            variable: "observed:multimod_weight".into(),
        }
    } else {
        SemWeightBindingV4::Case {
            variable: "observed:multimod_weight".into(),
        }
    });
    Ok(())
}

pub fn relation_id(model: &SemModelV4, source: &str, target: &str) -> Result<String, DynError> {
    model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source: actual_source,
                target: actual_target,
                ..
            } if actual_source == source && actual_target == target => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("missing structural relation {source}->{target}")))
}

pub fn relation_parameter(
    model: &SemModelV4,
    source: &str,
    target: &str,
) -> Result<String, DynError> {
    model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                parameter,
                source: actual_source,
                target: actual_target,
                ..
            } if actual_source == source && actual_target == target => Some(parameter.clone()),
            _ => None,
        })
        .ok_or_else(|| invalid(format!("missing structural parameter {source}->{target}")))
}

pub fn add_interaction(
    model: &mut SemModelV4,
    interaction_id: &str,
    operands: &[&str],
    focal_predictor: &str,
    outcome: &str,
) -> Result<(), DynError> {
    let focal_relation = relation_id(model, focal_predictor, outcome)?;
    let output = format!("derived:{interaction_id}");
    let relation = format!("relation:{interaction_id}:effect");
    let parameter = format!("parameter:{interaction_id}:effect");
    model.variables.push(SemVariableV4::Derived {
        id: output.clone(),
        label: interaction_id.into(),
    });
    model.relations.push(SemRelationV4::Structural {
        id: relation,
        source: output.clone(),
        target: outcome.into(),
        parameter: parameter.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter,
        label: format!("{interaction_id} -> {outcome}"),
        target: SemParameterTargetV4::Regression {
            source: output.clone(),
            target: outcome.into(),
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

pub fn add_disjoint_hoc(model: &mut SemModelV4, hoc_id: &str, components: &[&str], outcome: &str) {
    let output = format!("derived:{hoc_id}");
    let relation = format!("relation:{hoc_id}:to:{outcome}");
    let parameter = format!("parameter:{hoc_id}:to:{outcome}");
    model.variables.push(SemVariableV4::Derived {
        id: output.clone(),
        label: hoc_id.into(),
    });
    model.relations.push(SemRelationV4::Structural {
        id: relation,
        source: output.clone(),
        target: outcome.into(),
        parameter: parameter.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: None,
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter,
        label: format!("{hoc_id} -> {outcome}"),
        target: SemParameterTargetV4::Regression {
            source: output.clone(),
            target: outcome.into(),
        },
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    model.derived_terms.push(SemDerivedTermV4::HigherOrder {
        id: format!("term:{hoc_id}"),
        output,
        components: components.iter().map(|value| (*value).into()).collect(),
        approach: HigherOrderConstructionApproachV4::DisjointTwoStage,
        measurement_type: HigherOrderMeasurementTypeV4::ReflectiveReflective,
    });
}

pub fn standardize(values: &[f64]) -> Result<Vec<f64>, DynError> {
    if values.len() < 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(invalid(
            "standardization requires at least three finite values",
        ));
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let sd = variance.sqrt();
    if !sd.is_finite() || sd <= f64::EPSILON {
        return Err(invalid("standardization received a constant vector"));
    }
    Ok(values.iter().map(|value| (value - mean) / sd).collect())
}
