use crate::{
    CompiledCbsemInputV2, CompiledCbsemPlanV2, CompiledCbsemPlanV2Error, FactorDisturbancePolicyV4,
    FactorIdentificationV4, FactorMeanPolicyV4, InteractionMethodV4, MissingDataPolicyV4,
    ObservedRoleV4, ObservedScaleV4, ProductIndicatorSpecificationV4, SemDataBindingV4,
    SemDerivedTermV4, SemEndpointV4, SemGroupV4, SemModelV4, SemParameterTargetV4, SemParameterV4,
    SemRelationV4, SemVariableV4, compile_cbsem_plan_v2,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

pub const COMPILED_CBSEM_PRODUCT_INDICATOR_PLAN_SCHEMA_VERSION_V1: u32 = 1;
pub const CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1: &str =
    "cbsem_unconstrained_product_indicator_moderation_v1";
/// Internal v1 safety envelope. This bounds both Cartesian construction and
/// the quadratic scan used to generate shared-source residual covariances.
pub const CBSEM_PRODUCT_INDICATOR_MAX_PRODUCT_COLUMNS_V1: usize = 81;
pub const CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1: u64 = 10_000_000;
pub const CBSEM_PRODUCT_INDICATOR_RAW_BYTES_PER_CELL_V1: u64 = 8;
/// One live product vector, its Arrow copy, and one conservative work copy.
pub const CBSEM_PRODUCT_INDICATOR_ESTIMATED_PEAK_BYTES_PER_CELL_V1: u64 = 24;
pub const CBSEM_PRODUCT_INDICATOR_PEAK_WORK_MEMORY_CEILING_BYTES_V1: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemProductIndicatorSourceV1 {
    observed_variable_id: String,
    source_column: String,
}

impl CompiledCbsemProductIndicatorSourceV1 {
    pub fn observed_variable_id(&self) -> &str {
        &self.observed_variable_id
    }

    pub fn source_column(&self) -> &str {
        &self.source_column
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemProductIndicatorV1 {
    observed_variable_id: String,
    source_column: String,
    predictor_indicator_id: String,
    predictor_source_column: String,
    moderator_indicator_id: String,
    moderator_source_column: String,
    measurement_relation_id: String,
    loading_parameter_id: String,
    residual_variance_parameter_id: String,
    marker: bool,
}

impl CompiledCbsemProductIndicatorV1 {
    pub fn observed_variable_id(&self) -> &str {
        &self.observed_variable_id
    }

    pub fn source_column(&self) -> &str {
        &self.source_column
    }

    pub fn predictor_indicator_id(&self) -> &str {
        &self.predictor_indicator_id
    }

    pub fn predictor_source_column(&self) -> &str {
        &self.predictor_source_column
    }

    pub fn moderator_indicator_id(&self) -> &str {
        &self.moderator_indicator_id
    }

    pub fn moderator_source_column(&self) -> &str {
        &self.moderator_source_column
    }

    pub fn measurement_relation_id(&self) -> &str {
        &self.measurement_relation_id
    }

    pub fn loading_parameter_id(&self) -> &str {
        &self.loading_parameter_id
    }

    pub fn residual_variance_parameter_id(&self) -> &str {
        &self.residual_variance_parameter_id
    }

    pub fn is_marker(&self) -> bool {
        self.marker
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemGeneratedCovarianceV1 {
    relation_id: String,
    parameter_id: String,
    left: SemEndpointV4,
    right: SemEndpointV4,
}

impl CompiledCbsemGeneratedCovarianceV1 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    pub fn left(&self) -> &SemEndpointV4 {
        &self.left
    }

    pub fn right(&self) -> &SemEndpointV4 {
        &self.right
    }
}

/// Immutable, data-independent materialization of one two-way latent
/// product-indicator moderation relation. Data-dependent means and standard
/// deviations are computed only by the estimator and are recorded in its run
/// provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemProductIndicatorPlanV1 {
    schema_version: u32,
    method_version: String,
    source_plan_sha256: String,
    source_model_scientific_sha256: String,
    interaction_term_id: String,
    predictor_id: String,
    moderator_id: String,
    outcome_id: String,
    interaction_factor_id: String,
    specification: ProductIndicatorSpecificationV4,
    predictor_indicators: Vec<CompiledCbsemProductIndicatorSourceV1>,
    moderator_indicators: Vec<CompiledCbsemProductIndicatorSourceV1>,
    product_indicators: Vec<CompiledCbsemProductIndicatorV1>,
    interaction_variance_parameter_id: String,
    generated_factor_covariances: Vec<CompiledCbsemGeneratedCovarianceV1>,
    generated_residual_covariances: Vec<CompiledCbsemGeneratedCovarianceV1>,
    expanded_plan: CompiledCbsemPlanV2,
}

impl CompiledCbsemProductIndicatorPlanV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn source_plan_sha256(&self) -> &str {
        &self.source_plan_sha256
    }

    pub fn source_model_scientific_sha256(&self) -> &str {
        &self.source_model_scientific_sha256
    }

    pub fn interaction_term_id(&self) -> &str {
        &self.interaction_term_id
    }

    pub fn predictor_id(&self) -> &str {
        &self.predictor_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
    }

    pub fn outcome_id(&self) -> &str {
        &self.outcome_id
    }

    pub fn interaction_factor_id(&self) -> &str {
        &self.interaction_factor_id
    }

    pub fn specification(&self) -> &ProductIndicatorSpecificationV4 {
        &self.specification
    }

    pub fn predictor_indicators(&self) -> &[CompiledCbsemProductIndicatorSourceV1] {
        &self.predictor_indicators
    }

    pub fn moderator_indicators(&self) -> &[CompiledCbsemProductIndicatorSourceV1] {
        &self.moderator_indicators
    }

    pub fn product_indicators(&self) -> &[CompiledCbsemProductIndicatorV1] {
        &self.product_indicators
    }

    pub fn interaction_variance_parameter_id(&self) -> &str {
        &self.interaction_variance_parameter_id
    }

    pub fn generated_factor_covariances(&self) -> &[CompiledCbsemGeneratedCovarianceV1] {
        &self.generated_factor_covariances
    }

    pub fn generated_residual_covariances(&self) -> &[CompiledCbsemGeneratedCovarianceV1] {
        &self.generated_residual_covariances
    }

    pub fn expanded_plan(&self) -> &CompiledCbsemPlanV2 {
        &self.expanded_plan
    }

    pub fn deterministic_sha256(&self) -> String {
        let bytes =
            serde_json::to_vec(self).expect("a validated product-indicator plan is serializable");
        format!("{:x}", Sha256::digest(bytes))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorCapabilityIssueV1 {
    pub code: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledCbsemProductIndicatorErrorV1 {
    #[error("CB-SEM product-indicator moderation cannot execute {count} represented feature(s)", count = .issues.len())]
    Unsupported {
        issues: Vec<CbsemProductIndicatorCapabilityIssueV1>,
    },
    #[error(transparent)]
    ExpandedPlan(#[from] CompiledCbsemPlanV2Error),
}

pub fn validate_cbsem_product_indicator_capability_v1(
    plan: &CompiledCbsemPlanV2,
) -> Vec<CbsemProductIndicatorCapabilityIssueV1> {
    let mut issues = Vec::new();
    match plan.input() {
        CompiledCbsemInputV2::Raw {
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
            ..
        } => {}
        _ => push_issue(
            &mut issues,
            "raw_listwise_input_required",
            "data_binding",
            "Product-indicator moderation v1 requires unweighted raw data with listwise deletion and no cluster or strata binding.",
        ),
    }
    if !matches!(plan.group(), SemGroupV4::SingleGroup) {
        push_issue(
            &mut issues,
            "single_group_required",
            "group",
            "Product-indicator moderation v1 is single-group only.",
        );
    }
    if plan.has_feedback() {
        push_issue(
            &mut issues,
            "recursive_model_required",
            "structural_model",
            "Product-indicator moderation v1 requires a recursive structural model.",
        );
    }

    let interactions = plan
        .derived_terms()
        .iter()
        .filter_map(|term| match term {
            SemDerivedTermV4::Interaction {
                id,
                output,
                predictor,
                moderator,
                focal_relation,
                method,
                product_indicator,
            } => Some((
                id,
                output,
                predictor,
                moderator,
                focal_relation,
                method,
                product_indicator,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    if plan.derived_terms().len() != 1 || interactions.len() != 1 {
        push_issue(
            &mut issues,
            "exactly_one_interaction_required",
            "derived_terms",
            "The v1 estimator executes exactly one two-way interaction and no other derived term.",
        );
    }
    let Some((term_id, output, predictor, moderator, focal_relation, method, specification)) =
        interactions.first().copied()
    else {
        sort_issues(&mut issues);
        return issues;
    };
    if *method != InteractionMethodV4::ProductIndicator {
        push_issue(
            &mut issues,
            "product_indicator_method_required",
            term_id,
            "This estimator executes the documented product-indicator method; two-stage, orthogonalizing, and LMS methods are not substituted.",
        );
    }
    if specification.is_none() {
        push_issue(
            &mut issues,
            "product_indicator_spec_required",
            term_id,
            "Explicit product construction, centering, and standardization settings are required.",
        );
    }

    let variables = plan
        .variables()
        .iter()
        .map(|variable| (variable.id(), variable))
        .collect::<HashMap<_, _>>();
    for (role, id) in [("predictor", predictor), ("moderator", moderator)] {
        match variables.get(id.as_str()) {
            Some(SemVariableV4::CommonFactor {
                identification: FactorIdentificationV4::MarkerLoading { .. },
                ..
            }) => {}
            Some(SemVariableV4::CommonFactor { .. }) => push_issue(
                &mut issues,
                "marker_loading_identification_required",
                id,
                format!(
                    "The interaction {role} must use marker-loading identification so the interaction marker has a stable scientific scale."
                ),
            ),
            _ => push_issue(
                &mut issues,
                "common_factor_input_required",
                id,
                format!("The interaction {role} must be a common factor."),
            ),
        }
        if plan
            .regressions()
            .iter()
            .any(|regression| regression.target() == id.as_str())
        {
            push_issue(
                &mut issues,
                "exogenous_interaction_input_required",
                id,
                format!(
                    "The interaction {role} must be exogenous in product-indicator moderation v1."
                ),
            );
        }
    }
    if !matches!(
        variables.get(output.as_str()),
        Some(SemVariableV4::Derived { .. })
    ) {
        push_issue(
            &mut issues,
            "derived_interaction_output_required",
            output,
            "The scientific interaction output must be an explicit derived variable before materialization.",
        );
    }

    let focal = plan
        .regressions()
        .iter()
        .find(|regression| regression.relation_id() == focal_relation.as_str());
    let outcome = focal.map(|regression| regression.target());
    if let Some(outcome) = outcome {
        if outcome == predictor
            || outcome == moderator
            || !matches!(
                variables.get(outcome),
                Some(SemVariableV4::CommonFactor { .. })
            )
        {
            push_issue(
                &mut issues,
                "common_factor_outcome_required",
                outcome,
                "The bounded v1 interaction outcome must be a distinct common factor.",
            );
        }
        let interaction_effects = plan
            .regressions()
            .iter()
            .filter(|regression| regression.source() == output.as_str())
            .collect::<Vec<_>>();
        if interaction_effects.len() != 1 || interaction_effects[0].target() != outcome {
            push_issue(
                &mut issues,
                "interaction_effect_path_invalid",
                output,
                "The interaction output must have exactly one structural effect, targeting the focal-path outcome.",
            );
        }
        if !plan.regressions().iter().any(|regression| {
            regression.source() == moderator.as_str() && regression.target() == outcome
        }) {
            push_issue(
                &mut issues,
                "moderator_main_effect_missing",
                moderator,
                "Add the moderator main effect to the focal outcome before estimating the interaction.",
            );
        }
    }
    if plan
        .regressions()
        .iter()
        .any(|regression| regression.target() == output.as_str())
    {
        push_issue(
            &mut issues,
            "interaction_factor_must_be_exogenous",
            output,
            "The product-indicator interaction factor must be exogenous in the v1 slice.",
        );
    }
    if plan.regressions().iter().any(|regression| {
        regression.intercept_parameter_id().is_some()
            && (regression.source() == output.as_str()
                || regression.source() == predictor.as_str()
                || regression.source() == moderator.as_str())
    }) {
        push_issue(
            &mut issues,
            "structural_intercept_unsupported",
            term_id,
            "Structural intercepts remain outside the covariance-structure product-indicator v1 slice.",
        );
    }

    let source_model = plan.to_scientific_sem_model_v4();
    let indicator_sources = source_model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                scale: ObservedScaleV4::Continuous,
                role: ObservedRoleV4::Indicator,
                categories,
                value_labels,
                missing_markers,
                transformation_lineage,
                ..
            } if categories.is_empty()
                && value_labels.is_empty()
                && missing_markers.is_empty()
                && transformation_lineage.is_empty() =>
            {
                Some((id.as_str(), source_column.as_str()))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let predictor_indicators = factor_indicator_ids(plan, predictor);
    let moderator_indicators = factor_indicator_ids(plan, moderator);
    for (role, indicators) in [
        ("predictor", &predictor_indicators),
        ("moderator", &moderator_indicators),
    ] {
        if indicators.len() < 2 {
            push_issue(
                &mut issues,
                "multiple_indicators_required",
                if role == "predictor" {
                    predictor
                } else {
                    moderator
                },
                format!("The {role} requires at least two effect indicators."),
            );
        }
        for indicator in indicators {
            if !indicator_sources.contains_key(indicator.as_str()) {
                push_issue(
                    &mut issues,
                    "continuous_untransformed_indicator_required",
                    indicator,
                    format!("The {role} indicator must be an untransformed continuous indicator."),
                );
            }
            let measurement_factors = plan
                .loadings()
                .iter()
                .filter(|loading| loading.indicator() == indicator.as_str())
                .map(|loading| loading.factor())
                .collect::<Vec<_>>();
            let expected_factor = if role == "predictor" {
                predictor.as_str()
            } else {
                moderator.as_str()
            };
            if measurement_factors.len() != 1 || measurement_factors[0] != expected_factor {
                push_issue(
                    &mut issues,
                    "simple_measurement_structure_required",
                    indicator,
                    format!(
                        "The {role} indicator must load exactly once, on {expected_factor}; cross-loadings are outside product-indicator moderation v1."
                    ),
                );
            }
        }
    }
    match predictor_indicators
        .len()
        .checked_mul(moderator_indicators.len())
    {
        Some(product_count) if product_count <= CBSEM_PRODUCT_INDICATOR_MAX_PRODUCT_COLUMNS_V1 => {}
        Some(product_count) => push_issue(
            &mut issues,
            "product_column_limit_exceeded",
            term_id,
            format!(
                "The all-pairs interaction requires {product_count} product columns; Internal v1 permits at most {CBSEM_PRODUCT_INDICATOR_MAX_PRODUCT_COLUMNS_V1}. Reduce either measurement block."
            ),
        ),
        None => push_issue(
            &mut issues,
            "product_column_count_overflow",
            term_id,
            "The all-pairs product-column count overflowed the platform size; reduce both measurement blocks.",
        ),
    }
    let overlap = predictor_indicators
        .iter()
        .filter(|indicator| moderator_indicators.contains(indicator))
        .cloned()
        .collect::<Vec<_>>();
    for indicator in overlap {
        push_issue(
            &mut issues,
            "indicator_overlap_unsupported",
            indicator,
            "Predictor and moderator measurement blocks must not share indicators.",
        );
    }

    let interaction_source_indicators = predictor_indicators
        .iter()
        .chain(&moderator_indicators)
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    for relation in &source_model.relations {
        let SemRelationV4::Covariance {
            id, left, right, ..
        } = relation
        else {
            continue;
        };
        let touches_source_residual = [left, right].into_iter().any(|endpoint| {
            matches!(
                endpoint,
                SemEndpointV4::ResidualOf(indicator)
                    if interaction_source_indicators.contains(indicator.as_str())
            )
        });
        if touches_source_residual {
            push_issue(
                &mut issues,
                "local_independence_required",
                id,
                "Product-indicator moderation v1 requires locally independent predictor and moderator measurement errors; remove residual covariances touching their source indicators.",
            );
        }
    }

    let output_touches = source_model
        .relations
        .iter()
        .filter(|relation| match relation {
            SemRelationV4::Structural { source, .. } => source == output.as_str(),
            SemRelationV4::MeasurementEffect {
                construct,
                indicator,
                ..
            } => construct == output.as_str() || indicator == output.as_str(),
            SemRelationV4::MeasurementCausal {
                indicator,
                composite,
                ..
            } => indicator == output.as_str() || composite == output.as_str(),
            SemRelationV4::Covariance { left, right, .. } => {
                left.variable_id() == output.as_str() || right.variable_id() == output.as_str()
            }
        })
        .count();
    if output_touches != 1 {
        push_issue(
            &mut issues,
            "interaction_output_relations_unsupported",
            output,
            "Before materialization the interaction output may participate only in its one effect path.",
        );
    }

    sort_issues(&mut issues);
    issues
}

pub fn compile_cbsem_product_indicator_plan_v1(
    source_plan: &CompiledCbsemPlanV2,
) -> Result<CompiledCbsemProductIndicatorPlanV1, CompiledCbsemProductIndicatorErrorV1> {
    let issues = validate_cbsem_product_indicator_capability_v1(source_plan);
    if !issues.is_empty() {
        return Err(CompiledCbsemProductIndicatorErrorV1::Unsupported { issues });
    }
    let SemDerivedTermV4::Interaction {
        id: interaction_term_id,
        output,
        predictor,
        moderator,
        focal_relation,
        method: InteractionMethodV4::ProductIndicator,
        product_indicator: Some(specification),
    } = source_plan.derived_terms()[0].clone()
    else {
        unreachable!("capability validation froze one explicit product-indicator interaction")
    };
    let outcome = source_plan
        .regressions()
        .iter()
        .find(|regression| regression.relation_id() == focal_relation)
        .expect("validated focal relation")
        .target()
        .to_owned();
    let source_model = source_plan.to_scientific_sem_model_v4();
    let source_columns = source_model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.clone(), source_column.clone())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let predictor_indicators = compiled_sources(source_plan, &predictor, &source_columns);
    let moderator_indicators = compiled_sources(source_plan, &moderator, &source_columns);
    let term_namespace = stable_fragment(&["term", &interaction_term_id], 20);
    let interaction_variance_parameter_id =
        format!("parameter:pi:{term_namespace}:interaction_variance");
    let mut product_indicators = Vec::new();
    for predictor_source in &predictor_indicators {
        for moderator_source in &moderator_indicators {
            let pair = stable_fragment(
                &[
                    "product",
                    &interaction_term_id,
                    &predictor_source.observed_variable_id,
                    &moderator_source.observed_variable_id,
                ],
                20,
            );
            product_indicators.push(CompiledCbsemProductIndicatorV1 {
                observed_variable_id: format!("observed:pi:{term_namespace}:{pair}"),
                source_column: format!("__qpls_pi_{term_namespace}_{pair}"),
                predictor_indicator_id: predictor_source.observed_variable_id.clone(),
                predictor_source_column: predictor_source.source_column.clone(),
                moderator_indicator_id: moderator_source.observed_variable_id.clone(),
                moderator_source_column: moderator_source.source_column.clone(),
                measurement_relation_id: format!("relation:pi:{term_namespace}:{pair}:loading"),
                loading_parameter_id: format!("parameter:pi:{term_namespace}:{pair}:loading"),
                residual_variance_parameter_id: format!(
                    "parameter:pi:{term_namespace}:{pair}:residual_variance"
                ),
                marker: false,
            });
        }
    }
    product_indicators.sort_by(|left, right| {
        (
            &left.predictor_indicator_id,
            &left.moderator_indicator_id,
            &left.observed_variable_id,
        )
            .cmp(&(
                &right.predictor_indicator_id,
                &right.moderator_indicator_id,
                &right.observed_variable_id,
            ))
    });
    let predictor_marker = factor_marker_indicator_id(&source_model, &predictor)
        .expect("capability validation requires predictor marker identification");
    let moderator_marker = factor_marker_indicator_id(&source_model, &moderator)
        .expect("capability validation requires moderator marker identification");
    product_indicators
        .iter_mut()
        .find(|product| {
            product.predictor_indicator_id == predictor_marker
                && product.moderator_indicator_id == moderator_marker
        })
        .expect("validated source markers belong to their measurement blocks")
        .marker = true;

    let mut expanded_model = source_model;
    let interaction_label = expanded_model
        .variables
        .iter()
        .find(|variable| variable.id() == output)
        .map(|variable| variable.label().to_owned())
        .expect("validated interaction output");
    let marker_indicator = product_indicators
        .iter()
        .find(|product| product.marker)
        .expect("the declared source-marker product was materialized")
        .observed_variable_id
        .clone();
    let output_variable = expanded_model
        .variables
        .iter_mut()
        .find(|variable| variable.id() == output)
        .expect("validated interaction output");
    *output_variable = SemVariableV4::CommonFactor {
        id: output.clone(),
        label: interaction_label,
        identification: FactorIdentificationV4::MarkerLoading {
            indicator: marker_indicator,
        },
        mean_policy: FactorMeanPolicyV4::FixedZero,
        disturbance_policy: FactorDisturbancePolicyV4::ExogenousVariance {
            parameter: interaction_variance_parameter_id.clone(),
        },
    };
    expanded_model
        .derived_terms
        .retain(|term| term.id() != interaction_term_id);
    expanded_model.parameters.push(SemParameterV4::Free {
        id: interaction_variance_parameter_id.clone(),
        label: format!("Variance({output})"),
        target: SemParameterTargetV4::Variance {
            endpoint: SemEndpointV4::Variable(output.clone()),
        },
        start: Some(1.0),
        lower: Some(1e-8),
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    for product in &product_indicators {
        expanded_model.variables.push(SemVariableV4::Observed {
            id: product.observed_variable_id.clone(),
            label: format!(
                "{} × {}",
                product.predictor_source_column, product.moderator_source_column
            ),
            source_column: product.source_column.clone(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Indicator,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            // The executable transformation is frozen in this product plan and
            // the run provenance. The already-materialized estimator dataset is
            // intentionally bound as untransformed input to the exact engine.
            transformation_lineage: Vec::new(),
        });
        expanded_model
            .relations
            .push(SemRelationV4::MeasurementEffect {
                id: product.measurement_relation_id.clone(),
                construct: output.clone(),
                indicator: product.observed_variable_id.clone(),
                parameter: product.loading_parameter_id.clone(),
            });
        expanded_model.parameters.push(if product.marker {
            SemParameterV4::Fixed {
                id: product.loading_parameter_id.clone(),
                label: format!("{} loading", product.source_column),
                target: SemParameterTargetV4::Loading {
                    construct: output.clone(),
                    indicator: product.observed_variable_id.clone(),
                },
                value: 1.0,
                group_overrides: Vec::new(),
            }
        } else {
            SemParameterV4::Free {
                id: product.loading_parameter_id.clone(),
                label: format!("{} loading", product.source_column),
                target: SemParameterTargetV4::Loading {
                    construct: output.clone(),
                    indicator: product.observed_variable_id.clone(),
                },
                start: Some(1.0),
                lower: None,
                upper: None,
                equality_label: None,
                group_overrides: Vec::new(),
            }
        });
        expanded_model.parameters.push(SemParameterV4::Free {
            id: product.residual_variance_parameter_id.clone(),
            label: format!("Residual variance({})", product.source_column),
            target: SemParameterTargetV4::Variance {
                endpoint: SemEndpointV4::ResidualOf(product.observed_variable_id.clone()),
            },
            start: Some(0.5),
            lower: Some(1e-8),
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }

    let mut generated_factor_covariances = Vec::new();
    for main_factor in [&predictor, &moderator] {
        generated_factor_covariances.push(add_generated_covariance(
            &mut expanded_model,
            &interaction_term_id,
            "factor",
            SemEndpointV4::Variable(output.clone()),
            SemEndpointV4::Variable(main_factor.clone()),
        ));
    }
    let mut generated_residual_covariances = Vec::new();
    for left_index in 0..product_indicators.len() {
        for right_index in left_index + 1..product_indicators.len() {
            let left = &product_indicators[left_index];
            let right = &product_indicators[right_index];
            if left.predictor_indicator_id == right.predictor_indicator_id
                || left.moderator_indicator_id == right.moderator_indicator_id
            {
                generated_residual_covariances.push(add_generated_covariance(
                    &mut expanded_model,
                    &interaction_term_id,
                    "shared_source_residual",
                    SemEndpointV4::ResidualOf(left.observed_variable_id.clone()),
                    SemEndpointV4::ResidualOf(right.observed_variable_id.clone()),
                ));
            }
        }
    }
    expanded_model.data_binding = SemDataBindingV4::Raw {
        dataset_id: source_plan.input().dataset_id().to_owned(),
        missing_data: MissingDataPolicyV4::ListwiseDeletion,
        weight: None,
        cluster_variable: None,
        strata_variable: None,
    };
    let expanded_plan = compile_cbsem_plan_v2(&expanded_model)?;
    Ok(CompiledCbsemProductIndicatorPlanV1 {
        schema_version: COMPILED_CBSEM_PRODUCT_INDICATOR_PLAN_SCHEMA_VERSION_V1,
        method_version: CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1.into(),
        source_plan_sha256: source_plan.deterministic_sha256(),
        source_model_scientific_sha256: source_plan.scientific_hash().into(),
        interaction_term_id,
        predictor_id: predictor,
        moderator_id: moderator,
        outcome_id: outcome,
        interaction_factor_id: output,
        specification,
        predictor_indicators,
        moderator_indicators,
        product_indicators,
        interaction_variance_parameter_id,
        generated_factor_covariances,
        generated_residual_covariances,
        expanded_plan,
    })
}

fn compiled_sources(
    plan: &CompiledCbsemPlanV2,
    factor: &str,
    source_columns: &HashMap<String, String>,
) -> Vec<CompiledCbsemProductIndicatorSourceV1> {
    let mut sources = factor_indicator_ids(plan, factor)
        .into_iter()
        .map(
            |observed_variable_id| CompiledCbsemProductIndicatorSourceV1 {
                source_column: source_columns[&observed_variable_id].clone(),
                observed_variable_id,
            },
        )
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        (&left.observed_variable_id, &left.source_column)
            .cmp(&(&right.observed_variable_id, &right.source_column))
    });
    sources
}

fn factor_indicator_ids(plan: &CompiledCbsemPlanV2, factor: &str) -> Vec<String> {
    let mut indicators = plan
        .loadings()
        .iter()
        .filter(|loading| loading.factor() == factor)
        .map(|loading| loading.indicator().to_owned())
        .collect::<Vec<_>>();
    indicators.sort();
    indicators.dedup();
    indicators
}

fn add_generated_covariance(
    model: &mut SemModelV4,
    interaction_term_id: &str,
    role: &str,
    left: SemEndpointV4,
    right: SemEndpointV4,
) -> CompiledCbsemGeneratedCovarianceV1 {
    let (left, right) = canonical_endpoint_pair(left, right);
    let (left_kind, left_id) = stable_endpoint_parts(&left);
    let (right_kind, right_id) = stable_endpoint_parts(&right);
    let fragment = stable_fragment(
        &[
            "covariance",
            interaction_term_id,
            role,
            left_kind,
            left_id,
            right_kind,
            right_id,
        ],
        20,
    );
    let relation_id = format!("relation:pi:{fragment}:covariance");
    let parameter_id = format!("parameter:pi:{fragment}:covariance");
    model.relations.push(SemRelationV4::Covariance {
        id: relation_id.clone(),
        left: left.clone(),
        right: right.clone(),
        parameter: parameter_id.clone(),
    });
    model.parameters.push(SemParameterV4::Free {
        id: parameter_id.clone(),
        label: format!("Cov({left:?}, {right:?})"),
        target: SemParameterTargetV4::Covariance {
            left: left.clone(),
            right: right.clone(),
        },
        start: Some(0.0),
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    });
    CompiledCbsemGeneratedCovarianceV1 {
        relation_id,
        parameter_id,
        left,
        right,
    }
}

fn canonical_endpoint_pair(
    left: SemEndpointV4,
    right: SemEndpointV4,
) -> (SemEndpointV4, SemEndpointV4) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn stable_endpoint_parts(endpoint: &SemEndpointV4) -> (&'static str, &str) {
    match endpoint {
        SemEndpointV4::Variable(id) => ("variable_v1", id),
        SemEndpointV4::ResidualOf(id) => ("residual_v1", id),
        SemEndpointV4::DisturbanceOf(id) => ("disturbance_v1", id),
    }
}

fn factor_marker_indicator_id<'a>(model: &'a SemModelV4, factor: &str) -> Option<&'a str> {
    model.variables.iter().find_map(|variable| match variable {
        SemVariableV4::CommonFactor {
            id,
            identification: FactorIdentificationV4::MarkerLoading { indicator },
            ..
        } if id == factor => Some(indicator.as_str()),
        _ => None,
    })
}

fn stable_fragment(parts: &[&str], length: usize) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    let encoded = format!("{:x}", digest.finalize());
    encoded[..length].to_owned()
}

fn push_issue(
    issues: &mut Vec<CbsemProductIndicatorCapabilityIssueV1>,
    code: impl Into<String>,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(CbsemProductIndicatorCapabilityIssueV1 {
        code: code.into(),
        subject: subject.into(),
        message: message.into(),
    });
}

fn sort_issues(issues: &mut Vec<CbsemProductIndicatorCapabilityIssueV1>) {
    issues.sort_by(|left, right| {
        (&left.code, &left.subject, &left.message).cmp(&(
            &right.code,
            &right.subject,
            &right.message,
        ))
    });
    issues.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec,
        ProductIndicatorCenteringV4, ProductIndicatorPairingV4, ProductIndicatorStandardizationV4,
        StructuralPath, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn source_model() -> SemModelV4 {
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_5010),
            name: "Product indicator fixture".into(),
            constructs: vec![
                construct("x", &["x1", "x2"]),
                construct("m", &["m1", "m2"]),
                construct("y", &["y1", "y2", "y3"]),
            ],
            paths: vec![path("x", "y"), path("m", "y")],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x_by_m".into(),
            label: "X × M".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "relation:interaction_effect".into(),
            source: "derived:x_by_m".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction_effect".into(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction_effect".into(),
            label: "X × M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x_by_m".into(),
                target: "construct:y".into(),
            },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "term:x_by_m".into(),
            output: "derived:x_by_m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator: Some(ProductIndicatorSpecificationV4 {
                centering: ProductIndicatorCenteringV4::DoubleMeanCenter,
                standardization: ProductIndicatorStandardizationV4::None,
                pairing: ProductIndicatorPairingV4::AllPairs,
            }),
        });
        model.ensure_valid().unwrap();
        model
    }

    fn construct(id: &str, indicators: &[&str]) -> Construct {
        Construct {
            id: id.into(),
            name: id.to_uppercase(),
            short_name: id.to_uppercase(),
            mode: MeasurementMode::Reflective,
            indicators: indicators.iter().map(|value| (*value).into()).collect(),
        }
    }

    fn path(source: &str, target: &str) -> StructuralPath {
        StructuralPath {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn all_pairs_materialize_as_an_exact_latent_factor_with_stable_provenance() {
        let source = compile_cbsem_plan_v2(&source_model()).unwrap();
        let compiled = compile_cbsem_product_indicator_plan_v1(&source).unwrap();
        assert_eq!(compiled.product_indicators().len(), 4);
        assert_eq!(
            compiled
                .product_indicators()
                .iter()
                .filter(|indicator| indicator.is_marker())
                .count(),
            1
        );
        assert_eq!(compiled.generated_factor_covariances().len(), 2);
        assert_eq!(compiled.generated_residual_covariances().len(), 4);
        assert_eq!(
            compiled
                .generated_factor_covariances()
                .iter()
                .map(|covariance| covariance.parameter_id())
                .collect::<Vec<_>>(),
            vec![
                "parameter:pi:49e5b689a726118f090f:covariance",
                "parameter:pi:851a59fe13e0cfffc423:covariance",
            ]
        );
        assert_eq!(
            compiled
                .generated_residual_covariances()
                .iter()
                .map(|covariance| covariance.parameter_id())
                .collect::<Vec<_>>(),
            vec![
                "parameter:pi:455d216cdd8d06a78b37:covariance",
                "parameter:pi:51b91456f4bcebe0615c:covariance",
                "parameter:pi:d28cdaf9ed6ea7d6ed29:covariance",
                "parameter:pi:1711248fdd7bdd197fa3:covariance",
            ]
        );
        assert!(compiled.expanded_plan().derived_terms().is_empty());
        assert_eq!(
            compiled.expanded_plan().factors(),
            vec![
                "construct:m",
                "construct:x",
                "construct:y",
                "derived:x_by_m"
            ]
        );
        assert_eq!(
            compile_cbsem_product_indicator_plan_v1(&source)
                .unwrap()
                .deterministic_sha256(),
            compiled.deterministic_sha256()
        );
    }

    #[test]
    fn declaration_reordering_preserves_the_compiled_product_plan() {
        let base = source_model();
        let expected =
            compile_cbsem_product_indicator_plan_v1(&compile_cbsem_plan_v2(&base).unwrap())
                .unwrap();
        let mut reordered = base;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        let actual =
            compile_cbsem_product_indicator_plan_v1(&compile_cbsem_plan_v2(&reordered).unwrap())
                .unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn interaction_marker_uses_the_two_declared_source_markers() {
        let mut model = source_model();
        set_factor_marker(&mut model, "construct:x", "observed:x2");
        set_factor_marker(&mut model, "construct:m", "observed:m2");
        model.ensure_valid().unwrap();
        let compiled =
            compile_cbsem_product_indicator_plan_v1(&compile_cbsem_plan_v2(&model).unwrap())
                .unwrap();
        let marker = compiled
            .product_indicators()
            .iter()
            .find(|indicator| indicator.is_marker())
            .unwrap();
        assert_eq!(marker.predictor_indicator_id(), "observed:x2");
        assert_eq!(marker.moderator_indicator_id(), "observed:m2");
    }

    #[test]
    fn endogenous_interaction_input_fails_with_a_corrective_code() {
        let mut model = source_model();
        make_factor_endogenous(&mut model, "construct:x", "construct:m");
        model.ensure_valid().unwrap();
        let issues =
            validate_cbsem_product_indicator_capability_v1(&compile_cbsem_plan_v2(&model).unwrap());
        assert!(issues.iter().any(|issue| {
            issue.code == "exogenous_interaction_input_required" && issue.subject == "construct:x"
        }));
    }

    #[test]
    fn correlated_source_measurement_errors_fail_with_a_corrective_code() {
        let mut model = source_model();
        let left = SemEndpointV4::ResidualOf("observed:x1".into());
        let right = SemEndpointV4::ResidualOf("observed:m2".into());
        model.relations.push(SemRelationV4::Covariance {
            id: "relation:test:source_residual_covariance".into(),
            left: left.clone(),
            right: right.clone(),
            parameter: "parameter:test:source_residual_covariance".into(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:test:source_residual_covariance".into(),
            label: "Cov(error(x1), error(m2))".into(),
            target: SemParameterTargetV4::Covariance { left, right },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        let issues =
            validate_cbsem_product_indicator_capability_v1(&compile_cbsem_plan_v2(&model).unwrap());
        assert!(issues.iter().any(|issue| {
            issue.code == "local_independence_required"
                && issue.subject == "relation:test:source_residual_covariance"
        }));
    }

    #[test]
    fn cross_loaded_source_indicator_fails_with_a_corrective_code() {
        let mut model = source_model();
        model.relations.push(SemRelationV4::MeasurementEffect {
            id: "relation:test:x1_cross_loading".into(),
            construct: "construct:y".into(),
            indicator: "observed:x1".into(),
            parameter: "parameter:test:x1_cross_loading".into(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:test:x1_cross_loading".into(),
            label: "Y loading on x1".into(),
            target: SemParameterTargetV4::Loading {
                construct: "construct:y".into(),
                indicator: "observed:x1".into(),
            },
            start: Some(0.2),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        let issues =
            validate_cbsem_product_indicator_capability_v1(&compile_cbsem_plan_v2(&model).unwrap());
        assert!(issues.iter().any(|issue| {
            issue.code == "simple_measurement_structure_required" && issue.subject == "observed:x1"
        }));
    }

    #[test]
    fn product_column_envelope_rejects_cartesian_explosion_before_expansion() {
        let mut model = source_model();
        for index in 3..=10 {
            add_effect_indicator(&mut model, "construct:x", &format!("x{index}"));
            add_effect_indicator(&mut model, "construct:m", &format!("m{index}"));
        }
        model.ensure_valid().unwrap();
        let issues =
            validate_cbsem_product_indicator_capability_v1(&compile_cbsem_plan_v2(&model).unwrap());
        assert!(issues.iter().any(|issue| {
            issue.code == "product_column_limit_exceeded"
                && issue.message.contains("100 product columns")
        }));
    }

    #[test]
    fn missing_main_effect_and_non_product_methods_fail_with_corrective_codes() {
        let mut missing_main = source_model();
        let relation_id = missing_main
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:m" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        let parameter_id = missing_main
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural { id, parameter, .. } if id == &relation_id => {
                    Some(parameter.clone())
                }
                _ => None,
            })
            .unwrap();
        missing_main
            .relations
            .retain(|relation| relation.id() != relation_id);
        missing_main
            .parameters
            .retain(|parameter| parameter.id() != parameter_id);
        missing_main.ensure_valid().unwrap();
        let issues = validate_cbsem_product_indicator_capability_v1(
            &compile_cbsem_plan_v2(&missing_main).unwrap(),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "moderator_main_effect_missing")
        );

        let mut wrong_method = source_model();
        let SemDerivedTermV4::Interaction {
            method,
            product_indicator,
            ..
        } = &mut wrong_method.derived_terms[0]
        else {
            unreachable!()
        };
        *method = InteractionMethodV4::TwoStage;
        *product_indicator = None;
        wrong_method.ensure_valid().unwrap();
        let issues = validate_cbsem_product_indicator_capability_v1(
            &compile_cbsem_plan_v2(&wrong_method).unwrap(),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "product_indicator_method_required")
        );
    }

    #[test]
    fn product_indicator_settings_are_explicit_and_fail_closed() {
        let mut model = source_model();
        let SemDerivedTermV4::Interaction {
            product_indicator, ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        *product_indicator = None;
        assert!(
            model.validate().iter().any(|issue| {
                issue.code == "derived.interaction.product_indicator_spec_required"
            })
        );
    }

    fn set_factor_marker(model: &mut SemModelV4, factor: &str, marker: &str) {
        let old_marker = model
            .variables
            .iter()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    id,
                    identification: FactorIdentificationV4::MarkerLoading { indicator },
                    ..
                } if id == factor => Some(indicator.clone()),
                _ => None,
            })
            .unwrap();
        let variable = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == factor)
            .unwrap();
        let SemVariableV4::CommonFactor { identification, .. } = variable else {
            unreachable!()
        };
        *identification = FactorIdentificationV4::MarkerLoading {
            indicator: marker.into(),
        };
        for parameter in &mut model.parameters {
            let replacement = match parameter.clone() {
                SemParameterV4::Fixed {
                    id,
                    label,
                    target:
                        SemParameterTargetV4::Loading {
                            construct,
                            indicator,
                        },
                    group_overrides,
                    ..
                } if construct == factor && indicator == old_marker => Some(SemParameterV4::Free {
                    id,
                    label,
                    target: SemParameterTargetV4::Loading {
                        construct,
                        indicator,
                    },
                    start: Some(0.7),
                    lower: None,
                    upper: None,
                    equality_label: None,
                    group_overrides,
                }),
                SemParameterV4::Free {
                    id,
                    label,
                    target:
                        SemParameterTargetV4::Loading {
                            construct,
                            indicator,
                        },
                    group_overrides,
                    ..
                } if construct == factor && indicator == marker => Some(SemParameterV4::Fixed {
                    id,
                    label,
                    target: SemParameterTargetV4::Loading {
                        construct,
                        indicator,
                    },
                    value: 1.0,
                    group_overrides,
                }),
                _ => None,
            };
            if let Some(replacement) = replacement {
                *parameter = replacement;
            }
        }
    }

    fn make_factor_endogenous(model: &mut SemModelV4, target: &str, source: &str) {
        let variance_parameter = model
            .variables
            .iter()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    id,
                    disturbance_policy: FactorDisturbancePolicyV4::ExogenousVariance { parameter },
                    ..
                } if id == target => Some(parameter.clone()),
                _ => None,
            })
            .unwrap();
        let variable = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == target)
            .unwrap();
        let SemVariableV4::CommonFactor {
            disturbance_policy, ..
        } = variable
        else {
            unreachable!()
        };
        *disturbance_policy = FactorDisturbancePolicyV4::EndogenousDisturbance {
            parameter: variance_parameter.clone(),
        };
        let variance = model
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == variance_parameter)
            .unwrap();
        match variance {
            SemParameterV4::Free {
                target: endpoint, ..
            }
            | SemParameterV4::Fixed {
                target: endpoint, ..
            } => {
                *endpoint = SemParameterTargetV4::Variance {
                    endpoint: SemEndpointV4::DisturbanceOf(target.into()),
                };
            }
            SemParameterV4::Derived { .. } => unreachable!(),
        }
        let relation_id = "relation:test:endogenous_interaction_input".to_string();
        let parameter_id = "parameter:test:endogenous_interaction_input".to_string();
        model.relations.push(SemRelationV4::Structural {
            id: relation_id,
            source: source.into(),
            target: target.into(),
            parameter: parameter_id.clone(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter_id,
            label: format!("{source} -> {target}"),
            target: SemParameterTargetV4::Regression {
                source: source.into(),
                target: target.into(),
            },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }

    fn add_effect_indicator(model: &mut SemModelV4, factor: &str, name: &str) {
        let observed_id = format!("observed:{name}");
        let loading_parameter_id = format!("parameter:test:{factor}:{name}:loading");
        let residual_parameter_id = format!("parameter:test:{factor}:{name}:residual_variance");
        model.variables.push(SemVariableV4::Observed {
            id: observed_id.clone(),
            label: name.to_uppercase(),
            source_column: name.into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Indicator,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        model.relations.push(SemRelationV4::MeasurementEffect {
            id: format!("relation:test:{factor}:{name}:loading"),
            construct: factor.into(),
            indicator: observed_id.clone(),
            parameter: loading_parameter_id.clone(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: loading_parameter_id,
            label: format!("{factor} loading on {name}"),
            target: SemParameterTargetV4::Loading {
                construct: factor.into(),
                indicator: observed_id.clone(),
            },
            start: Some(0.7),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: residual_parameter_id,
            label: format!("Residual variance({name})"),
            target: SemParameterTargetV4::Variance {
                endpoint: SemEndpointV4::ResidualOf(observed_id),
            },
            start: Some(0.5),
            lower: Some(1e-8),
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }
}
