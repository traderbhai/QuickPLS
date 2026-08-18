use crate::{
    FactorDisturbancePolicyV4, FactorIdentificationV4, FactorMeanPolicyV4, MissingDataPolicyV4,
    ObservedRoleV4, ObservedScaleV4, SemConstraintV4, SemDataBindingV4, SemDerivedTermV4,
    SemEndpointV4, SemGroupV4, SemMatrixSampleMetadataV4, SemModelV4, SemModelV4ValidationError,
    SemParameterGroupOverrideV4, SemParameterTargetV4, SemParameterV4, SemPresentationV4,
    SemRelationV4, SemVariableV4, SemWeightBindingV4, StructuralRelationRoleV4,
    WeightCapabilityIssueV1, WeightCapabilityTargetV1, resolve_weight_declaration_parts_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const COMPILED_CBSEM_PLAN_V2_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemLoadingV2 {
    relation_id: String,
    construct: String,
    indicator: String,
    parameter_id: String,
}

impl CompiledCbsemLoadingV2 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn construct(&self) -> &str {
        &self.construct
    }

    /// Compatibility alias for the original common-factor-only compiler.
    pub fn factor(&self) -> &str {
        &self.construct
    }

    pub fn indicator(&self) -> &str {
        &self.indicator
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemCausalMeasurementV2 {
    relation_id: String,
    indicator: String,
    composite: String,
    parameter_id: String,
}

impl CompiledCbsemCausalMeasurementV2 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn indicator(&self) -> &str {
        &self.indicator
    }

    pub fn composite(&self) -> &str {
        &self.composite
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemRegressionV2 {
    relation_id: String,
    source: String,
    target: String,
    parameter_id: String,
    #[serde(
        default,
        skip_serializing_if = "StructuralRelationRoleV4::is_structural"
    )]
    role: StructuralRelationRoleV4,
    intercept_parameter_id: Option<String>,
}

impl CompiledCbsemRegressionV2 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    pub fn role(&self) -> StructuralRelationRoleV4 {
        self.role
    }

    pub fn intercept_parameter_id(&self) -> Option<&str> {
        self.intercept_parameter_id.as_deref()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompiledCbsemCovarianceKindV2 {
    Variable,
    Residual,
    Disturbance,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemCovarianceV2 {
    relation_id: String,
    left: SemEndpointV4,
    right: SemEndpointV4,
    parameter_id: String,
}

impl CompiledCbsemCovarianceV2 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn left(&self) -> &SemEndpointV4 {
        &self.left
    }

    pub fn right(&self) -> &SemEndpointV4 {
        &self.right
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    pub fn kind(&self) -> CompiledCbsemCovarianceKindV2 {
        covariance_kind(&self.left, &self.right)
    }

    pub fn is_residual_covariance(&self) -> bool {
        self.kind() == CompiledCbsemCovarianceKindV2::Residual
    }

    pub fn is_disturbance_covariance(&self) -> bool {
        self.kind() == CompiledCbsemCovarianceKindV2::Disturbance
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemInputV2 {
    Raw {
        dataset_id: String,
        missing_data: MissingDataPolicyV4,
        #[serde(default)]
        weight: Option<SemWeightBindingV4>,
        #[serde(default)]
        cluster_variable: Option<String>,
        #[serde(default)]
        strata_variable: Option<String>,
    },
    Covariance {
        dataset_id: String,
        variables: Vec<String>,
        #[serde(default)]
        means: Option<BTreeMap<String, f64>>,
        #[serde(default)]
        standard_deviations: Option<BTreeMap<String, f64>>,
        sample: SemMatrixSampleMetadataV4,
    },
    Correlation {
        dataset_id: String,
        variables: Vec<String>,
        #[serde(default)]
        means: Option<BTreeMap<String, f64>>,
        #[serde(default)]
        standard_deviations: Option<BTreeMap<String, f64>>,
        sample: SemMatrixSampleMetadataV4,
    },
}

impl CompiledCbsemInputV2 {
    pub fn dataset_id(&self) -> &str {
        match self {
            Self::Raw { dataset_id, .. }
            | Self::Covariance { dataset_id, .. }
            | Self::Correlation { dataset_id, .. } => dataset_id,
        }
    }

    fn from_binding(binding: &SemDataBindingV4) -> Self {
        match binding {
            SemDataBindingV4::Raw {
                dataset_id,
                missing_data,
                weight,
                cluster_variable,
                strata_variable,
            } => Self::Raw {
                dataset_id: dataset_id.clone(),
                missing_data: missing_data.clone(),
                weight: weight.clone(),
                cluster_variable: cluster_variable.clone(),
                strata_variable: strata_variable.clone(),
            },
            SemDataBindingV4::Covariance {
                dataset_id,
                variables,
                means,
                standard_deviations,
                sample,
            } => Self::Covariance {
                dataset_id: dataset_id.clone(),
                variables: variables.clone(),
                means: means.clone(),
                standard_deviations: standard_deviations.clone(),
                sample: sample.clone(),
            },
            SemDataBindingV4::Correlation {
                dataset_id,
                variables,
                means,
                standard_deviations,
                sample,
            } => Self::Correlation {
                dataset_id: dataset_id.clone(),
                variables: variables.clone(),
                means: means.clone(),
                standard_deviations: standard_deviations.clone(),
                sample: sample.clone(),
            },
        }
    }

    fn to_binding(&self) -> SemDataBindingV4 {
        match self {
            Self::Raw {
                dataset_id,
                missing_data,
                weight,
                cluster_variable,
                strata_variable,
            } => SemDataBindingV4::Raw {
                dataset_id: dataset_id.clone(),
                missing_data: missing_data.clone(),
                weight: weight.clone(),
                cluster_variable: cluster_variable.clone(),
                strata_variable: strata_variable.clone(),
            },
            Self::Covariance {
                dataset_id,
                variables,
                means,
                standard_deviations,
                sample,
            } => SemDataBindingV4::Covariance {
                dataset_id: dataset_id.clone(),
                variables: variables.clone(),
                means: means.clone(),
                standard_deviations: standard_deviations.clone(),
                sample: sample.clone(),
            },
            Self::Correlation {
                dataset_id,
                variables,
                means,
                standard_deviations,
                sample,
            } => SemDataBindingV4::Correlation {
                dataset_id: dataset_id.clone(),
                variables: variables.clone(),
                means: means.clone(),
                standard_deviations: standard_deviations.clone(),
                sample: sample.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompiledCbsemParameterRoleV2 {
    Loading,
    CompositeWeight,
    Regression,
    Variance,
    Covariance,
    Intercept,
    Mean,
    Threshold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemParameterStatusV2 {
    Free {
        #[serde(default)]
        start: Option<f64>,
        #[serde(default)]
        lower: Option<f64>,
        #[serde(default)]
        upper: Option<f64>,
        #[serde(default)]
        equality_label: Option<String>,
    },
    Fixed {
        value: f64,
    },
    Derived {
        expression: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemParameterRowV2 {
    id: String,
    label: String,
    target: SemParameterTargetV4,
    specification: CompiledCbsemParameterStatusV2,
    #[serde(default)]
    group_overrides: Vec<SemParameterGroupOverrideV4>,
}

impl CompiledCbsemParameterRowV2 {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn target(&self) -> &SemParameterTargetV4 {
        &self.target
    }

    pub fn role(&self) -> CompiledCbsemParameterRoleV2 {
        match self.target {
            SemParameterTargetV4::Loading { .. } => CompiledCbsemParameterRoleV2::Loading,
            SemParameterTargetV4::Weight { .. } => CompiledCbsemParameterRoleV2::CompositeWeight,
            SemParameterTargetV4::Regression { .. } => CompiledCbsemParameterRoleV2::Regression,
            SemParameterTargetV4::Variance { .. } => CompiledCbsemParameterRoleV2::Variance,
            SemParameterTargetV4::Covariance { .. } => CompiledCbsemParameterRoleV2::Covariance,
            SemParameterTargetV4::Intercept { .. } => CompiledCbsemParameterRoleV2::Intercept,
            SemParameterTargetV4::Mean { .. } => CompiledCbsemParameterRoleV2::Mean,
            SemParameterTargetV4::Threshold { .. } => CompiledCbsemParameterRoleV2::Threshold,
        }
    }

    pub fn specification(&self) -> &CompiledCbsemParameterStatusV2 {
        &self.specification
    }

    pub fn group_overrides(&self) -> &[SemParameterGroupOverrideV4] {
        &self.group_overrides
    }

    fn from_parameter(parameter: SemParameterV4) -> Self {
        match parameter {
            SemParameterV4::Free {
                id,
                label,
                target,
                start,
                lower,
                upper,
                equality_label,
                group_overrides,
            } => Self {
                id,
                label,
                target,
                specification: CompiledCbsemParameterStatusV2::Free {
                    start,
                    lower,
                    upper,
                    equality_label,
                },
                group_overrides,
            },
            SemParameterV4::Fixed {
                id,
                label,
                target,
                value,
                group_overrides,
            } => Self {
                id,
                label,
                target,
                specification: CompiledCbsemParameterStatusV2::Fixed { value },
                group_overrides,
            },
            SemParameterV4::Derived {
                id,
                label,
                target,
                expression,
                group_overrides,
            } => Self {
                id,
                label,
                target,
                specification: CompiledCbsemParameterStatusV2::Derived { expression },
                group_overrides,
            },
        }
    }

    fn to_parameter(&self) -> SemParameterV4 {
        match &self.specification {
            CompiledCbsemParameterStatusV2::Free {
                start,
                lower,
                upper,
                equality_label,
            } => SemParameterV4::Free {
                id: self.id.clone(),
                label: self.label.clone(),
                target: self.target.clone(),
                start: *start,
                lower: *lower,
                upper: *upper,
                equality_label: equality_label.clone(),
                group_overrides: self.group_overrides.clone(),
            },
            CompiledCbsemParameterStatusV2::Fixed { value } => SemParameterV4::Fixed {
                id: self.id.clone(),
                label: self.label.clone(),
                target: self.target.clone(),
                value: *value,
                group_overrides: self.group_overrides.clone(),
            },
            CompiledCbsemParameterStatusV2::Derived { expression } => SemParameterV4::Derived {
                id: self.id.clone(),
                label: self.label.clone(),
                target: self.target.clone(),
                expression: expression.clone(),
                group_overrides: self.group_overrides.clone(),
            },
        }
    }
}

/// Immutable, deterministic CB-SEM scientific representation.
///
/// Compilation answers only whether a valid `SemModelV4` can be represented.
/// Estimator execution support is a distinct check performed by
/// `validate_cbsem_ml_v1_estimator_capability_v2`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemPlanV2 {
    schema_version: u32,
    sem_model_schema_version: u32,
    model_id: String,
    model_name: String,
    scientific_hash: String,
    input: CompiledCbsemInputV2,
    variables: Vec<SemVariableV4>,
    loadings: Vec<CompiledCbsemLoadingV2>,
    causal_measurements: Vec<CompiledCbsemCausalMeasurementV2>,
    regressions: Vec<CompiledCbsemRegressionV2>,
    covariances: Vec<CompiledCbsemCovarianceV2>,
    parameter_table: Vec<CompiledCbsemParameterRowV2>,
    constraints: Vec<SemConstraintV4>,
    derived_terms: Vec<SemDerivedTermV4>,
    group: SemGroupV4,
    has_feedback: bool,
}

impl CompiledCbsemPlanV2 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn scientific_hash(&self) -> &str {
        &self.scientific_hash
    }

    pub fn deterministic_sha256(&self) -> String {
        let encoded = serde_json::to_vec(self).expect("a validated CB-SEM plan is serializable");
        format!("{:x}", Sha256::digest(encoded))
    }

    pub fn input(&self) -> &CompiledCbsemInputV2 {
        &self.input
    }

    pub fn variables(&self) -> &[SemVariableV4] {
        &self.variables
    }

    pub fn observed_variables(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter_map(|variable| match variable {
                SemVariableV4::Observed { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn factors(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter_map(|variable| match variable {
                SemVariableV4::CommonFactor { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn composites(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter_map(|variable| match variable {
                SemVariableV4::Composite { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn derived_variables(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter_map(|variable| match variable {
                SemVariableV4::Derived { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn loadings(&self) -> &[CompiledCbsemLoadingV2] {
        &self.loadings
    }

    pub fn causal_measurements(&self) -> &[CompiledCbsemCausalMeasurementV2] {
        &self.causal_measurements
    }

    pub fn regressions(&self) -> &[CompiledCbsemRegressionV2] {
        &self.regressions
    }

    pub fn covariances(&self) -> &[CompiledCbsemCovarianceV2] {
        &self.covariances
    }

    pub fn parameters(&self) -> &[CompiledCbsemParameterRowV2] {
        &self.parameter_table
    }

    pub fn parameters_with_role(
        &self,
        role: CompiledCbsemParameterRoleV2,
    ) -> Vec<&CompiledCbsemParameterRowV2> {
        self.parameter_table
            .iter()
            .filter(|parameter| parameter.role() == role)
            .collect()
    }

    pub fn constraints(&self) -> &[SemConstraintV4] {
        &self.constraints
    }

    pub fn derived_terms(&self) -> &[SemDerivedTermV4] {
        &self.derived_terms
    }

    pub fn group(&self) -> &SemGroupV4 {
        &self.group
    }

    pub fn has_feedback(&self) -> bool {
        self.has_feedback
    }

    /// Reconstructs the exact canonical scientific model. Annotations and canvas
    /// presentation are intentionally absent from a scientific plan.
    pub fn to_scientific_sem_model_v4(&self) -> SemModelV4 {
        let mut relations = self
            .loadings
            .iter()
            .map(|loading| SemRelationV4::MeasurementEffect {
                id: loading.relation_id.clone(),
                construct: loading.construct.clone(),
                indicator: loading.indicator.clone(),
                parameter: loading.parameter_id.clone(),
            })
            .chain(self.causal_measurements.iter().map(|measurement| {
                SemRelationV4::MeasurementCausal {
                    id: measurement.relation_id.clone(),
                    indicator: measurement.indicator.clone(),
                    composite: measurement.composite.clone(),
                    parameter: measurement.parameter_id.clone(),
                }
            }))
            .chain(
                self.regressions
                    .iter()
                    .map(|regression| SemRelationV4::Structural {
                        id: regression.relation_id.clone(),
                        source: regression.source.clone(),
                        target: regression.target.clone(),
                        parameter: regression.parameter_id.clone(),
                        role: regression.role,
                        intercept_parameter: regression.intercept_parameter_id.clone(),
                    }),
            )
            .chain(
                self.covariances
                    .iter()
                    .map(|covariance| SemRelationV4::Covariance {
                        id: covariance.relation_id.clone(),
                        left: covariance.left.clone(),
                        right: covariance.right.clone(),
                        parameter: covariance.parameter_id.clone(),
                    }),
            )
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.id().cmp(right.id()));
        SemModelV4 {
            schema_version: self.sem_model_schema_version,
            id: self.model_id.clone(),
            name: self.model_name.clone(),
            variables: self.variables.clone(),
            relations,
            parameters: self
                .parameter_table
                .iter()
                .map(CompiledCbsemParameterRowV2::to_parameter)
                .collect(),
            constraints: self.constraints.clone(),
            derived_terms: self.derived_terms.clone(),
            group: self.group.clone(),
            data_binding: self.input.to_binding(),
            annotations: Vec::new(),
            presentation: SemPresentationV4::None,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledCbsemPlanV2Error {
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
}

/// Compile every valid scientific object into a deterministic CB-SEM plan.
/// This function deliberately does not claim that the current ML estimator can
/// execute the resulting plan.
pub fn compile_cbsem_plan_v2(
    model: &SemModelV4,
) -> Result<CompiledCbsemPlanV2, CompiledCbsemPlanV2Error> {
    model.ensure_valid()?;
    let canonical = model.canonicalized();
    let mut loadings = Vec::new();
    let mut causal_measurements = Vec::new();
    let mut regressions = Vec::new();
    let mut covariances = Vec::new();

    for relation in &canonical.relations {
        match relation {
            SemRelationV4::MeasurementEffect {
                id,
                construct,
                indicator,
                parameter,
            } => loadings.push(CompiledCbsemLoadingV2 {
                relation_id: id.clone(),
                construct: construct.clone(),
                indicator: indicator.clone(),
                parameter_id: parameter.clone(),
            }),
            SemRelationV4::MeasurementCausal {
                id,
                indicator,
                composite,
                parameter,
            } => causal_measurements.push(CompiledCbsemCausalMeasurementV2 {
                relation_id: id.clone(),
                indicator: indicator.clone(),
                composite: composite.clone(),
                parameter_id: parameter.clone(),
            }),
            SemRelationV4::Structural {
                id,
                source,
                target,
                parameter,
                role,
                intercept_parameter,
            } => regressions.push(CompiledCbsemRegressionV2 {
                relation_id: id.clone(),
                source: source.clone(),
                target: target.clone(),
                parameter_id: parameter.clone(),
                role: *role,
                intercept_parameter_id: intercept_parameter.clone(),
            }),
            SemRelationV4::Covariance {
                id,
                left,
                right,
                parameter,
            } => covariances.push(CompiledCbsemCovarianceV2 {
                relation_id: id.clone(),
                left: left.clone(),
                right: right.clone(),
                parameter_id: parameter.clone(),
            }),
        }
    }

    let plan = CompiledCbsemPlanV2 {
        schema_version: COMPILED_CBSEM_PLAN_V2_SCHEMA_VERSION,
        sem_model_schema_version: canonical.schema_version,
        model_id: canonical.id.clone(),
        model_name: canonical.name.clone(),
        scientific_hash: canonical.scientific_sha256()?,
        input: CompiledCbsemInputV2::from_binding(&canonical.data_binding),
        variables: canonical.variables,
        loadings,
        causal_measurements,
        regressions,
        covariances,
        parameter_table: canonical
            .parameters
            .into_iter()
            .map(CompiledCbsemParameterRowV2::from_parameter)
            .collect(),
        constraints: canonical.constraints,
        derived_terms: canonical.derived_terms,
        group: canonical.group,
        has_feedback: model.has_structural_feedback(),
    };
    debug_assert_eq!(
        plan.to_scientific_sem_model_v4().scientific_sha256().ok(),
        Some(plan.scientific_hash.clone())
    );
    Ok(plan)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemEstimatorCapabilityIssueV2 {
    pub code: String,
    pub subject: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_issue: Option<WeightCapabilityIssueV1>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("CB-SEM ML v1 estimator cannot execute {count} represented model feature(s)", count = .issues.len())]
pub struct CbsemEstimatorCapabilityErrorV2 {
    pub issues: Vec<CbsemEstimatorCapabilityIssueV2>,
}

/// Reports the exact boundary of the existing bounded ML implementation. A
/// non-empty report never invalidates or truncates the compiled scientific plan.
pub fn validate_cbsem_ml_v1_estimator_capability_v2(
    plan: &CompiledCbsemPlanV2,
) -> Vec<CbsemEstimatorCapabilityIssueV2> {
    validate_cbsem_ml_v1_estimator_capability_for_target_v2(
        plan,
        WeightCapabilityTargetV1::CbsemMlV1,
    )
}

fn validate_cbsem_ml_v1_estimator_capability_for_target_v2(
    plan: &CompiledCbsemPlanV2,
    weight_target: WeightCapabilityTargetV1,
) -> Vec<CbsemEstimatorCapabilityIssueV2> {
    let mut issues = Vec::new();
    if !matches!(plan.group, SemGroupV4::SingleGroup) {
        push_capability_issue(
            &mut issues,
            "multigroup",
            "group",
            "The bounded ML v1 estimator does not execute multigroup plans.",
        );
    }
    for term in &plan.derived_terms {
        push_capability_issue(
            &mut issues,
            "derived_terms",
            term.id(),
            "The bounded ML v1 estimator does not generate or execute derived terms.",
        );
    }
    for parameter in &plan.parameter_table {
        if !parameter.group_overrides.is_empty() {
            push_capability_issue(
                &mut issues,
                "parameter_group_overrides",
                &parameter.id,
                "The bounded ML v1 estimator does not execute parameter group overrides.",
            );
        }
    }

    match &plan.input {
        CompiledCbsemInputV2::Raw {
            dataset_id,
            missing_data,
            weight,
            cluster_variable,
            strata_variable,
        } => {
            let mut invalid_unrelated_option =
                !matches!(missing_data, MissingDataPolicyV4::ListwiseDeletion)
                    || cluster_variable.is_some()
                    || strata_variable.is_some();
            if let Some(weight) = weight {
                match resolve_weight_declaration_parts_v1(dataset_id, weight, &plan.variables) {
                    Ok(declaration) => push_weight_capability_issue(
                        &mut issues,
                        WeightCapabilityIssueV1::unsupported(weight_target, declaration),
                    ),
                    Err(_) => invalid_unrelated_option = true,
                }
            }
            if invalid_unrelated_option {
                push_capability_issue(
                    &mut issues,
                    "raw_data_options",
                    "data_binding",
                    "The bounded ML v1 estimator requires raw data with listwise deletion and no cluster or strata binding; invalid weight declarations are rejected separately.",
                );
            }
        }
        CompiledCbsemInputV2::Covariance {
            means,
            standard_deviations,
            sample,
            ..
        } => {
            if means.is_some() {
                push_capability_issue(
                    &mut issues,
                    "matrix_means_unsupported",
                    "data_binding",
                    "CB-SEM matrix-input ML does not estimate a mean structure; remove matrix means.",
                );
            }
            if standard_deviations.is_some() {
                push_capability_issue(
                    &mut issues,
                    "covariance_scale_metadata_unsupported",
                    "data_binding",
                    "A covariance matrix already contains scale; separate standard deviations are not accepted.",
                );
            }
            validate_matrix_sample_metadata_capability(sample, &mut issues);
        }
        CompiledCbsemInputV2::Correlation {
            means,
            standard_deviations,
            sample,
            ..
        } => {
            if means.is_some() {
                push_capability_issue(
                    &mut issues,
                    "matrix_means_unsupported",
                    "data_binding",
                    "CB-SEM matrix-input ML does not estimate a mean structure; remove matrix means.",
                );
            }
            if standard_deviations.is_none() {
                push_capability_issue(
                    &mut issues,
                    "correlation_scale_metadata_required",
                    "data_binding",
                    "Correlation input requires one positive standard deviation per observed matrix variable so it can be converted explicitly to covariance scale.",
                );
            }
            validate_matrix_sample_metadata_capability(sample, &mut issues);
        }
    }

    for variable in &plan.variables {
        match variable {
            SemVariableV4::Observed {
                id,
                scale: ObservedScaleV4::Continuous,
                missing_markers,
                transformation_lineage,
                ..
            } => {
                if !missing_markers.is_empty() {
                    push_capability_issue(
                        &mut issues,
                        "observed_missing_markers",
                        id,
                        "The bounded ML v1 listwise estimator does not consume observed-variable missing-marker metadata.",
                    );
                }
                if !transformation_lineage.is_empty() {
                    push_capability_issue(
                        &mut issues,
                        "observed_transformation_lineage",
                        id,
                        "The bounded ML v1 estimator does not execute observed-variable transformation lineage.",
                    );
                }
            }
            SemVariableV4::Observed { id, .. } => push_capability_issue(
                &mut issues,
                "categorical_or_ordinal_variable",
                id,
                "The bounded ML v1 estimator currently accepts continuous observed variables only.",
            ),
            SemVariableV4::CommonFactor {
                id,
                mean_policy,
                disturbance_policy,
                ..
            } => {
                if !matches!(mean_policy, FactorMeanPolicyV4::FixedZero)
                    || matches!(
                        disturbance_policy,
                        FactorDisturbancePolicyV4::FixedZero { .. }
                    )
                {
                    push_capability_issue(
                        &mut issues,
                        "factor_mean_or_disturbance_policy",
                        id,
                        "The bounded ML v1 estimator requires fixed-zero factor means and a non-zero exogenous variance or endogenous disturbance policy.",
                    );
                }
            }
            SemVariableV4::Composite { id, .. } => push_capability_issue(
                &mut issues,
                "composite",
                id,
                "The bounded ML v1 estimator does not execute composite variables.",
            ),
            SemVariableV4::Derived { id, .. } => push_capability_issue(
                &mut issues,
                "derived_variable",
                id,
                "The bounded ML v1 estimator does not execute derived variables.",
            ),
        }
    }

    let variance_endpoints = plan
        .parameter_table
        .iter()
        .filter_map(|parameter| match &parameter.target {
            SemParameterTargetV4::Variance { endpoint } => Some(endpoint.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    for factor in plan.factors() {
        if !variance_endpoints.contains(&SemEndpointV4::Variable(factor.to_string()))
            && !variance_endpoints.contains(&SemEndpointV4::DisturbanceOf(factor.to_string()))
        {
            push_capability_issue(
                &mut issues,
                "variance_missing",
                factor,
                "The bounded ML v1 estimator requires an explicit factor variance or disturbance variance.",
            );
        }
    }
    let measured_indicators = plan
        .loadings
        .iter()
        .map(|loading| loading.indicator.as_str())
        .collect::<HashSet<_>>();
    for indicator in measured_indicators {
        if !variance_endpoints.contains(&SemEndpointV4::ResidualOf(indicator.to_string())) {
            push_capability_issue(
                &mut issues,
                "residual_variance_missing",
                indicator,
                "The bounded ML v1 estimator requires an explicit residual variance for every effect indicator.",
            );
        }
    }

    let variables = plan
        .variables
        .iter()
        .map(|variable| (variable.id(), variable))
        .collect::<HashMap<_, _>>();
    for loading in &plan.loadings {
        if !matches!(
            variables.get(loading.construct.as_str()),
            Some(SemVariableV4::CommonFactor { .. })
        ) {
            push_capability_issue(
                &mut issues,
                "non_factor_loading",
                &loading.relation_id,
                "The bounded ML v1 estimator executes effect loadings for common factors only.",
            );
        }
    }
    for measurement in &plan.causal_measurements {
        push_capability_issue(
            &mut issues,
            "causal_measurement",
            &measurement.relation_id,
            "The bounded ML v1 estimator does not execute causal-indicator measurement relations.",
        );
    }
    for regression in &plan.regressions {
        if regression.intercept_parameter_id.is_some() {
            push_capability_issue(
                &mut issues,
                "structural_intercept",
                &regression.relation_id,
                "The bounded ML v1 estimator does not execute structural intercept parameters.",
            );
        }
    }
    for parameter in &plan.parameter_table {
        match parameter.role() {
            CompiledCbsemParameterRoleV2::CompositeWeight => push_capability_issue(
                &mut issues,
                "composite_weight",
                &parameter.id,
                "The bounded ML v1 estimator does not execute composite weights.",
            ),
            CompiledCbsemParameterRoleV2::Intercept
            | CompiledCbsemParameterRoleV2::Mean
            | CompiledCbsemParameterRoleV2::Threshold => push_capability_issue(
                &mut issues,
                "mean_or_threshold_structure",
                &parameter.id,
                "The bounded ML v1 estimator does not execute intercept, mean, or threshold parameters.",
            ),
            _ => {}
        }
    }

    issues.sort_by(|left, right| {
        (&left.code, &left.subject, &left.message).cmp(&(
            &right.code,
            &right.subject,
            &right.message,
        ))
    });
    issues.dedup();
    issues
}

pub fn ensure_cbsem_ml_v1_estimator_capability_v2(
    plan: &CompiledCbsemPlanV2,
) -> Result<(), CbsemEstimatorCapabilityErrorV2> {
    let issues = validate_cbsem_ml_v1_estimator_capability_v2(plan);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CbsemEstimatorCapabilityErrorV2 { issues })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbsemMlExactCapabilityProfileV2 {
    CovarianceStructure,
    RawCfaMeanStructure,
    RawContinuousMeanReplacement,
}

pub fn validate_cbsem_ml_exact_parameter_table_v4_capability_v2(
    plan: &CompiledCbsemPlanV2,
    profile: CbsemMlExactCapabilityProfileV2,
) -> Vec<CbsemEstimatorCapabilityIssueV2> {
    if profile == CbsemMlExactCapabilityProfileV2::CovarianceStructure {
        return validate_cbsem_ml_v1_estimator_capability_v2(plan);
    }

    let weight_target = if profile == CbsemMlExactCapabilityProfileV2::RawContinuousMeanReplacement
    {
        WeightCapabilityTargetV1::CbsemMlMeanReplacementV1
    } else {
        WeightCapabilityTargetV1::CbsemMlV1
    };
    let mut issues = validate_cbsem_ml_v1_estimator_capability_for_target_v2(plan, weight_target);
    if profile == CbsemMlExactCapabilityProfileV2::RawContinuousMeanReplacement {
        let exact_raw_policy = matches!(
            plan.input(),
            CompiledCbsemInputV2::Raw {
                missing_data: MissingDataPolicyV4::MeanReplacement,
                weight: None,
                cluster_variable: None,
                strata_variable: None,
                ..
            }
        );
        if exact_raw_policy {
            issues.retain(|issue| {
                issue.code != "raw_data_options" && issue.code != "observed_missing_markers"
            });
        } else {
            push_capability_issue(
                &mut issues,
                "mean_replacement_raw_options_required",
                "data_binding",
                "Mean replacement requires raw continuous data with the explicit mean-replacement policy and no weights, cluster, or strata binding.",
            );
        }
        let mut measurement_counts = HashMap::<&str, usize>::new();
        for loading in &plan.loadings {
            *measurement_counts
                .entry(loading.indicator.as_str())
                .or_default() += 1;
        }
        for variable in &plan.variables {
            if let SemVariableV4::Observed { id, role, .. } = variable
                && (*role != ObservedRoleV4::Indicator
                    || measurement_counts.get(id.as_str()) != Some(&1))
            {
                push_capability_issue(
                    &mut issues,
                    "mean_replacement_indicator_scope_required",
                    id,
                    "Mean replacement accepts only continuous observed indicators measured exactly once; controls and unmeasured or cross-loaded observed variables remain unsupported.",
                );
            }
        }
        issues.sort_by(|left, right| {
            (&left.code, &left.subject, &left.message).cmp(&(
                &right.code,
                &right.subject,
                &right.message,
            ))
        });
        issues.dedup();
        return issues;
    }

    issues.retain(|issue| {
        issue.code != "factor_mean_or_disturbance_policy"
            && issue.code != "mean_or_threshold_structure"
    });
    if !matches!(plan.input(), CompiledCbsemInputV2::Raw { .. }) {
        push_capability_issue(
            &mut issues,
            "mean_structure_raw_input_required",
            "data_binding",
            "Observed intercepts and latent means are executable only from raw continuous data; covariance/correlation means remain blocked.",
        );
    }
    if !plan.regressions.is_empty() {
        push_capability_issue(
            &mut issues,
            "mean_structure_cfa_required",
            "structural_model",
            "Raw-data mean structure is limited to CFA; structural regressions and structural intercepts remain blocked.",
        );
    }

    let observed_indicators = plan
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                scale: ObservedScaleV4::Continuous,
                role: ObservedRoleV4::Indicator,
                ..
            } => Some(id.clone()),
            SemVariableV4::Observed { id, .. } => {
                push_capability_issue(
                    &mut issues,
                    "cfa_indicator_scope_unsupported",
                    id,
                    "Raw CFA mean structure accepts continuous observed indicators only.",
                );
                None
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    let mut loading_counts = HashMap::<String, usize>::new();
    let mut loadings_by_factor = HashMap::<String, HashSet<String>>::new();
    for loading in &plan.loadings {
        *loading_counts.entry(loading.indicator.clone()).or_default() += 1;
        loadings_by_factor
            .entry(loading.construct.clone())
            .or_default()
            .insert(loading.indicator.clone());
    }
    for observed in &observed_indicators {
        if loading_counts.get(observed).copied() != Some(1) {
            push_capability_issue(
                &mut issues,
                "measurement_block_invalid",
                observed,
                "Every observed indicator must load on exactly one common factor.",
            );
        }
    }

    let mut intercepts_by_variable = HashMap::<String, Vec<&CompiledCbsemParameterRowV2>>::new();
    let mut means_by_variable = HashMap::<String, Vec<&CompiledCbsemParameterRowV2>>::new();
    for parameter in &plan.parameter_table {
        match &parameter.target {
            SemParameterTargetV4::Intercept { variable } => {
                if observed_indicators.contains(variable) {
                    intercepts_by_variable
                        .entry(variable.clone())
                        .or_default()
                        .push(parameter);
                } else {
                    push_capability_issue(
                        &mut issues,
                        "observed_intercept_target_unsupported",
                        &parameter.id,
                        "Only measured observed indicators may have intercept rows in raw CFA mean structure.",
                    );
                }
            }
            SemParameterTargetV4::Mean { variable } => {
                means_by_variable
                    .entry(variable.clone())
                    .or_default()
                    .push(parameter);
            }
            SemParameterTargetV4::Threshold { .. } => push_capability_issue(
                &mut issues,
                "threshold_structure_unsupported",
                &parameter.id,
                "Ordinal thresholds remain outside continuous raw CFA mean structure.",
            ),
            _ => {}
        }
    }
    for observed in &observed_indicators {
        match intercepts_by_variable.get(observed).map(Vec::len) {
            Some(1) => {}
            None | Some(0) => push_capability_issue(
                &mut issues,
                "observed_intercept_missing",
                observed,
                "Every measured indicator requires exactly one explicit observed-intercept parameter.",
            ),
            Some(_) => push_capability_issue(
                &mut issues,
                "observed_intercept_duplicate",
                observed,
                "A measured indicator cannot have more than one observed-intercept parameter.",
            ),
        }
    }

    let mut bound_mean_parameter_ids = HashSet::new();
    for variable in &plan.variables {
        let SemVariableV4::CommonFactor {
            id,
            identification,
            mean_policy,
            disturbance_policy,
            ..
        } = variable
        else {
            continue;
        };
        if !matches!(
            disturbance_policy,
            FactorDisturbancePolicyV4::ExogenousVariance { .. }
        ) {
            push_capability_issue(
                &mut issues,
                "factor_disturbance_policy_unsupported",
                id,
                "Raw CFA mean structure requires an exogenous factor variance; endogenous or fixed-zero disturbance policies remain blocked.",
            );
        }
        let factor_loadings = loadings_by_factor.get(id);
        if factor_loadings.is_none_or(|loadings| loadings.len() < 2) {
            push_capability_issue(
                &mut issues,
                "measurement_block_invalid",
                id,
                "Every common factor requires at least two measured indicators.",
            );
        }
        match identification {
            FactorIdentificationV4::EffectsCoding => push_capability_issue(
                &mut issues,
                "effects_coding_unsupported",
                id,
                "Effects-coding identification requires linear constraints and remains blocked.",
            ),
            FactorIdentificationV4::MarkerLoading { indicator }
                if factor_loadings.is_none_or(|loadings| !loadings.contains(indicator)) =>
            {
                push_capability_issue(
                    &mut issues,
                    "measurement_block_invalid",
                    id,
                    "Marker identification must reference an indicator in the factor measurement block.",
                );
            }
            _ => {}
        }
        match mean_policy {
            FactorMeanPolicyV4::FixedZero => {
                if means_by_variable
                    .get(id)
                    .is_some_and(|rows| !rows.is_empty())
                {
                    push_capability_issue(
                        &mut issues,
                        "latent_mean_parameter_unbound",
                        id,
                        "A fixed-zero factor mean must not also declare a latent-mean parameter row.",
                    );
                }
            }
            FactorMeanPolicyV4::Estimated { parameter } => {
                bound_mean_parameter_ids.insert(parameter.clone());
                let FactorIdentificationV4::MarkerLoading { indicator } = identification else {
                    push_capability_issue(
                        &mut issues,
                        "latent_mean_marker_identification_required",
                        id,
                        "An estimated latent mean requires marker-loading identification.",
                    );
                    continue;
                };
                let matching = means_by_variable.get(id).cloned().unwrap_or_default();
                if matching.len() != 1 || matching[0].id != *parameter {
                    push_capability_issue(
                        &mut issues,
                        "latent_mean_parameter_invalid",
                        id,
                        "An estimated factor mean requires exactly one matching Mean parameter row.",
                    );
                } else if !matches!(
                    &matching[0].specification,
                    CompiledCbsemParameterStatusV2::Free { .. }
                ) {
                    push_capability_issue(
                        &mut issues,
                        "latent_mean_parameter_must_be_free",
                        parameter,
                        "Estimated latent means must be free; fixed latent zero is represented by the factor mean policy.",
                    );
                }
                let marker_intercepts = intercepts_by_variable
                    .get(indicator)
                    .cloned()
                    .unwrap_or_default();
                if marker_intercepts.len() != 1
                    || !matches!(
                        &marker_intercepts[0].specification,
                        CompiledCbsemParameterStatusV2::Fixed { .. }
                    )
                {
                    push_capability_issue(
                        &mut issues,
                        "latent_mean_marker_intercept_must_be_fixed",
                        indicator,
                        "The marker indicator intercept must be fixed to anchor the estimated factor mean.",
                    );
                }
            }
            FactorMeanPolicyV4::ReferenceGroup { .. } => push_capability_issue(
                &mut issues,
                "latent_mean_reference_group_unsupported",
                id,
                "Reference-group latent means require multigroup estimation and remain blocked.",
            ),
        }
    }
    for (variable, rows) in means_by_variable {
        for row in rows {
            if !bound_mean_parameter_ids.contains(&row.id) {
                push_capability_issue(
                    &mut issues,
                    "latent_mean_parameter_unbound",
                    &row.id,
                    format!(
                        "Latent-mean parameter targets {variable} but is not bound by that factor's mean policy."
                    ),
                );
            }
        }
    }

    let mut equality_families = HashMap::<String, HashSet<CompiledCbsemParameterRoleV2>>::new();
    for parameter in &plan.parameter_table {
        let CompiledCbsemParameterStatusV2::Free {
            equality_label: Some(label),
            ..
        } = &parameter.specification
        else {
            continue;
        };
        equality_families
            .entry(label.trim().to_owned())
            .or_default()
            .insert(parameter.role());
    }
    for (label, families) in equality_families {
        let contains_location = families.contains(&CompiledCbsemParameterRoleV2::Intercept)
            || families.contains(&CompiledCbsemParameterRoleV2::Mean);
        if contains_location
            && (families.len() != 1
                || (families.contains(&CompiledCbsemParameterRoleV2::Intercept)
                    && families.contains(&CompiledCbsemParameterRoleV2::Mean)))
        {
            push_capability_issue(
                &mut issues,
                "location_equality_family_mismatch",
                label,
                "Equality labels must stay within observed intercepts or within latent means; location families cannot be mixed.",
            );
        }
    }

    for parameter in &plan.parameter_table {
        if matches!(
            parameter.role(),
            CompiledCbsemParameterRoleV2::Intercept | CompiledCbsemParameterRoleV2::Mean
        ) && matches!(
            &parameter.specification,
            CompiledCbsemParameterStatusV2::Derived { .. }
        ) {
            push_capability_issue(
                &mut issues,
                "derived_location_parameter_unsupported",
                &parameter.id,
                "Derived intercept or mean expressions remain outside the exact raw CFA mean slice.",
            );
        }
    }

    issues.sort_by(|left, right| {
        (&left.code, &left.subject, &left.message).cmp(&(
            &right.code,
            &right.subject,
            &right.message,
        ))
    });
    issues.dedup();
    issues
}

pub fn ensure_cbsem_ml_exact_parameter_table_v4_capability_v2(
    plan: &CompiledCbsemPlanV2,
    profile: CbsemMlExactCapabilityProfileV2,
) -> Result<(), CbsemEstimatorCapabilityErrorV2> {
    let issues = validate_cbsem_ml_exact_parameter_table_v4_capability_v2(plan, profile);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CbsemEstimatorCapabilityErrorV2 { issues })
    }
}

fn covariance_kind(left: &SemEndpointV4, right: &SemEndpointV4) -> CompiledCbsemCovarianceKindV2 {
    match (left, right) {
        (SemEndpointV4::Variable(_), SemEndpointV4::Variable(_)) => {
            CompiledCbsemCovarianceKindV2::Variable
        }
        (SemEndpointV4::ResidualOf(_), SemEndpointV4::ResidualOf(_)) => {
            CompiledCbsemCovarianceKindV2::Residual
        }
        (SemEndpointV4::DisturbanceOf(_), SemEndpointV4::DisturbanceOf(_)) => {
            CompiledCbsemCovarianceKindV2::Disturbance
        }
        _ => CompiledCbsemCovarianceKindV2::Mixed,
    }
}

fn push_capability_issue(
    issues: &mut Vec<CbsemEstimatorCapabilityIssueV2>,
    code: impl Into<String>,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(CbsemEstimatorCapabilityIssueV2 {
        code: code.into(),
        subject: subject.into(),
        message: message.into(),
        weight_issue: None,
    });
}

fn push_weight_capability_issue(
    issues: &mut Vec<CbsemEstimatorCapabilityIssueV2>,
    weight_issue: WeightCapabilityIssueV1,
) {
    issues.push(CbsemEstimatorCapabilityIssueV2 {
        code: weight_issue.code.as_str().into(),
        subject: weight_issue.subject.clone(),
        message: weight_issue.corrective_action.clone(),
        weight_issue: Some(weight_issue),
    });
}

fn validate_matrix_sample_metadata_capability(
    sample: &SemMatrixSampleMetadataV4,
    issues: &mut Vec<CbsemEstimatorCapabilityIssueV2>,
) {
    if sample.effective_sample_size.is_some() {
        push_capability_issue(
            issues,
            "matrix_effective_sample_size_unsupported",
            "data_binding",
            "Matrix-input ML uses the declared integer sample_size and does not substitute an effective sample size.",
        );
    }
    if sample.degrees_of_freedom.is_some() {
        push_capability_issue(
            issues,
            "matrix_degrees_of_freedom_unsupported",
            "data_binding",
            "Matrix-input ML derives model degrees of freedom and does not consume a dataset-level degrees_of_freedom override.",
        );
    }
    if !sample.group_sample_sizes.is_empty() {
        push_capability_issue(
            issues,
            "matrix_group_sample_sizes_unsupported",
            "data_binding",
            "Single-group matrix-input ML does not consume group sample sizes.",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, LegacyDisplayCovarianceV4, MeasurementMode,
        ModelSpec, ObservedRoleV4, SamplingWeightNormalizationV4, SemGroupLevelV4,
        SemLinearConstraintTermV4, SemParameterGroupOverrideSpecV4, StructuralPath,
        WeightCapabilityCodeV1, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn legacy() -> ModelSpec {
        ModelSpec {
            id: Uuid::nil(),
            name: "CB-SEM".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: vec![],
            higher_order_constructs: vec![],
            interactions: vec![],
        }
    }

    fn base_model() -> SemModelV4 {
        convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[LegacyDisplayCovarianceV4 {
                id: "visual-covariance".into(),
                left_construct: "x".into(),
                right_construct: "y".into(),
                label: None,
            }],
        )
        .unwrap()
    }

    fn weighted_model(weight: SemWeightBindingV4) -> SemModelV4 {
        let mut model = base_model();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:weight".into(),
            label: "Weight".into(),
            source_column: "survey_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let SemDataBindingV4::Raw {
            weight: configured, ..
        } = &mut model.data_binding
        else {
            unreachable!()
        };
        *configured = Some(weight);
        model.ensure_valid().unwrap();
        model
    }

    fn add_covariance(
        model: &mut SemModelV4,
        id: &str,
        left: SemEndpointV4,
        right: SemEndpointV4,
        specification: CompiledCbsemParameterStatusV2,
    ) {
        let target = SemParameterTargetV4::Covariance {
            left: left.clone(),
            right: right.clone(),
        };
        let parameter_id = format!("{id}-parameter");
        model.relations.push(SemRelationV4::Covariance {
            id: id.into(),
            left,
            right,
            parameter: parameter_id.clone(),
        });
        model.parameters.push(match specification {
            CompiledCbsemParameterStatusV2::Free {
                start,
                lower,
                upper,
                equality_label,
            } => SemParameterV4::Free {
                id: parameter_id,
                label: id.into(),
                target,
                start,
                lower,
                upper,
                equality_label,
                group_overrides: Vec::new(),
            },
            CompiledCbsemParameterStatusV2::Fixed { value } => SemParameterV4::Fixed {
                id: parameter_id,
                label: id.into(),
                target,
                value,
                group_overrides: Vec::new(),
            },
            CompiledCbsemParameterStatusV2::Derived { expression } => SemParameterV4::Derived {
                id: parameter_id,
                label: id.into(),
                target,
                expression,
                group_overrides: Vec::new(),
            },
        });
    }

    #[test]
    fn display_only_covariance_is_absent_but_all_scientific_covariance_kinds_compile() {
        let base = base_model();
        assert!(
            compile_cbsem_plan_v2(&base)
                .unwrap()
                .covariances()
                .is_empty()
        );

        let mut scientific = base.clone();
        add_covariance(
            &mut scientific,
            "variable-covariance",
            SemEndpointV4::Variable("construct:x".into()),
            SemEndpointV4::Variable("construct:y".into()),
            CompiledCbsemParameterStatusV2::Free {
                start: Some(0.1),
                lower: Some(-1.0),
                upper: Some(1.0),
                equality_label: Some("shared-covariance".into()),
            },
        );
        add_covariance(
            &mut scientific,
            "residual-covariance",
            SemEndpointV4::ResidualOf("observed:x1".into()),
            SemEndpointV4::ResidualOf("observed:y1".into()),
            CompiledCbsemParameterStatusV2::Fixed { value: 0.2 },
        );
        add_covariance(
            &mut scientific,
            "disturbance-covariance",
            SemEndpointV4::DisturbanceOf("construct:x".into()),
            SemEndpointV4::DisturbanceOf("construct:y".into()),
            CompiledCbsemParameterStatusV2::Derived {
                expression: "variable-covariance-parameter * 0.5".into(),
            },
        );
        add_covariance(
            &mut scientific,
            "mixed-covariance",
            SemEndpointV4::ResidualOf("observed:x2".into()),
            SemEndpointV4::Variable("construct:x".into()),
            CompiledCbsemParameterStatusV2::Free {
                start: Some(0.0),
                lower: None,
                upper: None,
                equality_label: None,
            },
        );
        let plan = compile_cbsem_plan_v2(&scientific).unwrap();
        assert_eq!(
            plan.covariances()
                .iter()
                .map(CompiledCbsemCovarianceV2::kind)
                .collect::<HashSet<_>>(),
            HashSet::from([
                CompiledCbsemCovarianceKindV2::Variable,
                CompiledCbsemCovarianceKindV2::Residual,
                CompiledCbsemCovarianceKindV2::Disturbance,
                CompiledCbsemCovarianceKindV2::Mixed,
            ])
        );
        assert_eq!(
            plan.parameters_with_role(CompiledCbsemParameterRoleV2::Covariance)
                .len(),
            4
        );
        assert_ne!(
            plan.scientific_hash(),
            compile_cbsem_plan_v2(&base).unwrap().scientific_hash()
        );
    }

    #[test]
    fn feedback_and_structural_intercepts_remain_explicit() {
        let mut model = base_model();
        let relation = model
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, SemRelationV4::Structural { .. }))
            .unwrap();
        let target = match relation {
            SemRelationV4::Structural {
                target,
                intercept_parameter,
                ..
            } => {
                *intercept_parameter = Some("y-intercept".into());
                target.clone()
            }
            _ => unreachable!(),
        };
        model.parameters.push(SemParameterV4::Free {
            id: "y-intercept".into(),
            label: "Intercept(Y)".into(),
            target: SemParameterTargetV4::Intercept { variable: target },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "feedback".into(),
            source: "construct:y".into(),
            target: "construct:x".into(),
            parameter: "feedback-p".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "feedback-p".into(),
            label: "Y -> X".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:y".into(),
                target: "construct:x".into(),
            },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let x_variance_parameter = match model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            SemVariableV4::CommonFactor {
                disturbance_policy, ..
            } => match disturbance_policy {
                FactorDisturbancePolicyV4::ExogenousVariance { parameter } => {
                    let parameter = parameter.clone();
                    *disturbance_policy = FactorDisturbancePolicyV4::EndogenousDisturbance {
                        parameter: parameter.clone(),
                    };
                    parameter
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        match model
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == x_variance_parameter)
            .unwrap()
        {
            SemParameterV4::Free { target, .. } => {
                *target = SemParameterTargetV4::Variance {
                    endpoint: SemEndpointV4::DisturbanceOf("construct:x".into()),
                };
            }
            _ => unreachable!(),
        }
        let plan = compile_cbsem_plan_v2(&model).unwrap();
        assert!(plan.has_feedback());
        assert_eq!(plan.regressions().len(), 2);
        assert_eq!(
            plan.regressions()
                .iter()
                .filter(|relation| relation.intercept_parameter_id().is_some())
                .count(),
            1
        );
        assert!(
            validate_cbsem_ml_v1_estimator_capability_v2(&plan)
                .iter()
                .any(|issue| issue.code == "structural_intercept")
        );
    }

    #[test]
    fn groups_means_thresholds_constraints_and_raw_metadata_are_lossless() {
        let mut model = base_model();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:weight".into(),
            label: "Sampling weight".into(),
            source_column: "sampling_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: vec!["-999".into()],
            transformation_lineage: Vec::new(),
        });
        model.variables.push(SemVariableV4::Observed {
            id: "observed:ordinal".into(),
            label: "Ordinal outcome".into(),
            source_column: "ordinal_outcome".into(),
            scale: ObservedScaleV4::Ordinal,
            role: ObservedRoleV4::Structural,
            categories: vec!["1".into(), "2".into(), "3".into()],
            value_labels: BTreeMap::from([
                ("1".into(), "Low".into()),
                ("2".into(), "Middle".into()),
                ("3".into(), "High".into()),
            ]),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:x1".into(),
            levels: vec![
                SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "Group B".into(),
                },
                SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "Group A".into(),
                },
            ],
        };
        model.parameters.push(SemParameterV4::Free {
            id: "factor-x-mean".into(),
            label: "Mean(X)".into(),
            target: SemParameterTargetV4::Mean {
                variable: "construct:x".into(),
            },
            start: Some(0.0),
            lower: Some(-10.0),
            upper: Some(10.0),
            equality_label: Some("latent-means".into()),
            group_overrides: vec![
                SemParameterGroupOverrideV4 {
                    group: "b".into(),
                    specification: SemParameterGroupOverrideSpecV4::Free {
                        start: Some(0.0),
                        lower: None,
                        upper: None,
                    },
                },
                SemParameterGroupOverrideV4 {
                    group: "a".into(),
                    specification: SemParameterGroupOverrideSpecV4::Fixed { value: 0.0 },
                },
            ],
        });
        if let SemVariableV4::CommonFactor { mean_policy, .. } = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            *mean_policy = FactorMeanPolicyV4::ReferenceGroup {
                reference_group: "a".into(),
                parameter: "factor-x-mean".into(),
            };
        }
        model.parameters.push(SemParameterV4::Free {
            id: "ordinal-threshold-1".into(),
            label: "Ordinal threshold 1".into(),
            target: SemParameterTargetV4::Threshold {
                variable: "observed:ordinal".into(),
                index: 0,
            },
            start: Some(-0.5),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let free_loading_ids = model
            .parameters
            .iter()
            .filter_map(|parameter| match parameter {
                SemParameterV4::Free {
                    id,
                    target: SemParameterTargetV4::Loading { .. },
                    ..
                } => Some(id.clone()),
                _ => None,
            })
            .take(2)
            .collect::<Vec<_>>();
        model.constraints.extend([
            SemConstraintV4::Equality {
                id: "equal-loadings".into(),
                parameters: free_loading_ids.clone(),
            },
            SemConstraintV4::Bound {
                id: "positive-loading".into(),
                parameter: free_loading_ids[0].clone(),
                lower: Some(0.0),
                upper: None,
            },
            SemConstraintV4::Linear {
                id: "loading-sum".into(),
                terms: free_loading_ids
                    .iter()
                    .map(|parameter| SemLinearConstraintTermV4 {
                        parameter: parameter.clone(),
                        coefficient: 1.0,
                    })
                    .collect(),
                value: 2.0,
            },
        ]);
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: "raw-groups".into(),
            missing_data: MissingDataPolicyV4::FullInformationMaximumLikelihood,
            weight: Some(SemWeightBindingV4::Sampling {
                variable: "observed:weight".into(),
                normalization: SamplingWeightNormalizationV4::MeanOne,
            }),
            cluster_variable: Some("observed:x1".into()),
            strata_variable: Some("observed:x2".into()),
        };
        assert!(model.validate().is_empty(), "{:?}", model.validate());

        let plan = compile_cbsem_plan_v2(&model).unwrap();
        assert!(
            matches!(plan.group(), SemGroupV4::ObservedGroups { levels, .. } if levels[0].id == "a")
        );
        assert_eq!(plan.constraints().len(), 3);
        let mean = plan
            .parameters_with_role(CompiledCbsemParameterRoleV2::Mean)
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(mean.group_overrides()[0].group, "a");
        assert!(matches!(
            mean.specification(),
            CompiledCbsemParameterStatusV2::Free {
                start: Some(0.0),
                lower: Some(-10.0),
                upper: Some(10.0),
                equality_label: Some(label),
            } if label == "latent-means"
        ));
        assert_eq!(
            plan.parameters_with_role(CompiledCbsemParameterRoleV2::Threshold)
                .len(),
            1
        );
        assert!(matches!(
            plan.input(),
            CompiledCbsemInputV2::Raw {
                missing_data: MissingDataPolicyV4::FullInformationMaximumLikelihood,
                weight: Some(SemWeightBindingV4::Sampling { .. }),
                cluster_variable: Some(cluster),
                strata_variable: Some(strata),
                ..
            } if cluster == "observed:x1" && strata == "observed:x2"
        ));
        let issue_codes = validate_cbsem_ml_v1_estimator_capability_v2(&plan)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<HashSet<_>>();
        assert!(issue_codes.contains("multigroup"));
        assert!(issue_codes.contains("parameter_group_overrides"));
        assert!(issue_codes.contains("mean_or_threshold_structure"));
        assert!(issue_codes.contains("sampling_weight_unsupported"));
        assert!(issue_codes.contains("raw_data_options"));
        assert!(ensure_cbsem_ml_v1_estimator_capability_v2(&plan).is_err());
    }

    #[test]
    fn matrix_moments_and_group_sample_metadata_round_trip_exactly() {
        let mut model = base_model();
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:x1".into(),
            levels: vec![
                SemGroupLevelV4 {
                    id: "a".into(),
                    value: "A".into(),
                    label: "A".into(),
                },
                SemGroupLevelV4 {
                    id: "b".into(),
                    value: "B".into(),
                    label: "B".into(),
                },
            ],
        };
        let variables = vec![
            "observed:x1".into(),
            "observed:x2".into(),
            "observed:y1".into(),
            "observed:y2".into(),
        ];
        model.data_binding = SemDataBindingV4::Correlation {
            dataset_id: "correlation-input".into(),
            variables: variables.clone(),
            means: Some(BTreeMap::from([
                ("observed:x1".into(), 1.0),
                ("observed:x2".into(), 2.0),
                ("observed:y1".into(), 3.0),
                ("observed:y2".into(), 4.0),
            ])),
            standard_deviations: Some(BTreeMap::from([
                ("observed:x1".into(), 1.1),
                ("observed:x2".into(), 1.2),
                ("observed:y1".into(), 1.3),
                ("observed:y2".into(), 1.4),
            ])),
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 200,
                covariance_denominator: crate::SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: Some(184.5),
                degrees_of_freedom: Some(199),
                group_sample_sizes: BTreeMap::from([("a".into(), 90), ("b".into(), 110)]),
            },
        };
        let plan = compile_cbsem_plan_v2(&model).unwrap();
        assert_eq!(
            plan.to_scientific_sem_model_v4()
                .scientific_sha256()
                .unwrap(),
            model.scientific_sha256().unwrap()
        );
        assert!(matches!(
            plan.input(),
            CompiledCbsemInputV2::Correlation {
                variables: compiled_variables,
                means: Some(means),
                standard_deviations: Some(standard_deviations),
                sample,
                ..
            } if compiled_variables == &variables
                && means["observed:y1"] == 3.0
                && standard_deviations["observed:x2"] == 1.2
                && sample.group_sample_sizes["b"] == 110
        ));
    }

    #[test]
    fn composite_and_derived_semantics_compile_before_capability_validation() {
        let mut composite_legacy = legacy();
        composite_legacy.constructs[0].mode = MeasurementMode::Formative;
        let composite = convert_legacy_basic_model_v4(
            &composite_legacy,
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let composite_plan = compile_cbsem_plan_v2(&composite).unwrap();
        assert_eq!(composite_plan.composites().len(), 2);
        assert!(!composite_plan.causal_measurements().is_empty());
        assert!(
            validate_cbsem_ml_v1_estimator_capability_v2(&composite_plan)
                .iter()
                .any(|issue| issue.code == "composite")
        );

        let mut derived = base_model();
        derived.variables.push(SemVariableV4::Derived {
            id: "derived:x-squared".into(),
            label: "X squared".into(),
        });
        derived.derived_terms.push(SemDerivedTermV4::Polynomial {
            id: "x-squared-term".into(),
            output: "derived:x-squared".into(),
            source: "construct:x".into(),
            degree: 2,
        });
        derived.relations.push(SemRelationV4::Structural {
            id: "x-squared-effect".into(),
            source: "derived:x-squared".into(),
            target: "construct:y".into(),
            parameter: "x-squared-effect-parameter".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        derived.parameters.push(SemParameterV4::Free {
            id: "x-squared-effect-parameter".into(),
            label: "X squared -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x-squared".into(),
                target: "construct:y".into(),
            },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let derived_plan = compile_cbsem_plan_v2(&derived).unwrap();
        assert_eq!(derived_plan.derived_variables(), vec!["derived:x-squared"]);
        assert_eq!(derived_plan.derived_terms().len(), 1);
        assert!(
            validate_cbsem_ml_v1_estimator_capability_v2(&derived_plan)
                .iter()
                .any(|issue| issue.code == "derived_terms")
        );
    }

    #[test]
    fn compilation_is_reorder_invariant_and_json_round_trip_is_exact() {
        let model = base_model();
        let plan = compile_cbsem_plan_v2(&model).unwrap();
        let mut reordered = model.clone();
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        reordered.constraints.reverse();
        reordered.derived_terms.reverse();
        let reordered_plan = compile_cbsem_plan_v2(&reordered).unwrap();
        assert_eq!(reordered_plan, plan);
        assert_eq!(
            reordered_plan.deterministic_sha256(),
            plan.deterministic_sha256()
        );

        let encoded = serde_json::to_vec(&plan).unwrap();
        let reopened: CompiledCbsemPlanV2 = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(reopened, plan);
        assert_eq!(reopened.deterministic_sha256(), plan.deterministic_sha256());
        assert_eq!(
            reopened
                .to_scientific_sem_model_v4()
                .scientific_sha256()
                .unwrap(),
            model.scientific_sha256().unwrap()
        );

        let mut tampered = serde_json::to_value(&plan).unwrap();
        tampered["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CompiledCbsemPlanV2>(tampered).is_err());
    }

    #[test]
    fn bounded_ml_reports_each_weight_kind_as_typed_and_preserves_unrelated_raw_options() {
        let cases = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::CaseWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::MeanOne,
                },
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
            ),
        ];
        for (binding, expected_code) in cases {
            let plan = compile_cbsem_plan_v2(&weighted_model(binding)).unwrap();
            let issues = validate_cbsem_ml_v1_estimator_capability_v2(&plan);
            assert_eq!(issues.len(), 1);
            let issue = &issues[0];
            assert_eq!(issue.code, expected_code.as_str());
            assert_eq!(issue.subject, "observed:weight");
            let weight_issue = issue.weight_issue.as_ref().unwrap();
            assert_eq!(weight_issue.code, expected_code);
            assert_eq!(weight_issue.target, WeightCapabilityTargetV1::CbsemMlV1);
            let declaration = weight_issue.declaration.as_ref().unwrap();
            assert_eq!(declaration.dataset_id(), "legacy-unbound");
            assert_eq!(declaration.binding().source_column(), "survey_weight");
        }

        let mut unrelated = weighted_model(SemWeightBindingV4::Case {
            variable: "observed:weight".into(),
        });
        let SemDataBindingV4::Raw {
            weight,
            cluster_variable,
            ..
        } = &mut unrelated.data_binding
        else {
            unreachable!()
        };
        *weight = None;
        *cluster_variable = Some("observed:weight".into());
        let issues = validate_cbsem_ml_v1_estimator_capability_v2(
            &compile_cbsem_plan_v2(&unrelated).unwrap(),
        );
        assert!(
            issues
                .iter()
                .any(|issue| { issue.code == "raw_data_options" && issue.weight_issue.is_none() })
        );
    }

    #[test]
    fn bounded_ml_capability_accepts_the_legacy_microcase_only_after_compilation() {
        let plan = compile_cbsem_plan_v2(&base_model()).unwrap();
        assert!(validate_cbsem_ml_v1_estimator_capability_v2(&plan).is_empty());
        assert!(ensure_cbsem_ml_v1_estimator_capability_v2(&plan).is_ok());
    }

    #[test]
    fn raw_cfa_mean_profile_accepts_explicit_marker_location_and_preserves_v1_rejection() {
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_5001),
            name: "Raw CFA mean capability".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "F".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
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
        .unwrap();
        let factor_id = model
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    id, mean_policy, ..
                } => {
                    *mean_policy = FactorMeanPolicyV4::Estimated {
                        parameter: "factor-mean".into(),
                    };
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();
        model.parameters.extend([
            SemParameterV4::Fixed {
                id: "x1-intercept".into(),
                label: "x1 intercept anchor".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x1".into(),
                },
                value: 0.0,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "x2-intercept".into(),
                label: "x2 intercept".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x2".into(),
                },
                start: Some(0.0),
                lower: Some(-10.0),
                upper: Some(10.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "x3-intercept".into(),
                label: "x3 intercept".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x3".into(),
                },
                start: Some(0.0),
                lower: Some(-10.0),
                upper: Some(10.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "factor-mean".into(),
                label: "F mean".into(),
                target: SemParameterTargetV4::Mean {
                    variable: factor_id,
                },
                start: Some(1.0),
                lower: Some(-10.0),
                upper: Some(10.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
        ]);
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: "raw-cfa-means".into(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        let plan = compile_cbsem_plan_v2(&model).unwrap();
        let v1_codes = validate_cbsem_ml_v1_estimator_capability_v2(&plan)
            .into_iter()
            .map(|issue| issue.code)
            .collect::<HashSet<_>>();
        assert!(v1_codes.contains("factor_mean_or_disturbance_policy"));
        assert!(v1_codes.contains("mean_or_threshold_structure"));
        assert!(
            validate_cbsem_ml_exact_parameter_table_v4_capability_v2(
                &plan,
                CbsemMlExactCapabilityProfileV2::RawCfaMeanStructure,
            )
            .is_empty()
        );

        let marker = model
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == "x1-intercept")
            .unwrap();
        *marker = SemParameterV4::Free {
            id: "x1-intercept".into(),
            label: "x1 intercept".into(),
            target: SemParameterTargetV4::Intercept {
                variable: "observed:x1".into(),
            },
            start: Some(0.0),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        };
        let issues = validate_cbsem_ml_exact_parameter_table_v4_capability_v2(
            &compile_cbsem_plan_v2(&model).unwrap(),
            CbsemMlExactCapabilityProfileV2::RawCfaMeanStructure,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "latent_mean_marker_intercept_must_be_fixed")
        );
    }
}
