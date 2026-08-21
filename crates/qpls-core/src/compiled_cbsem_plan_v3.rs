use crate::{
    CompiledCbsemParameterRowV2, CompiledCbsemParameterStatusV2, CompiledCbsemPlanV2,
    CompiledCbsemPlanV2Error, CompiledSemStronglyConnectedComponentV1, CompiledSemTopologyV1,
    CompiledSemTopologyV1Error, FactorIdentificationV4, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemSpecificPathLimitBehaviorV1, SemConstraintV4,
    SemEndpointV4, SemParameterTargetV4, SemVariableV4, compile_cbsem_plan_v2,
    compile_sem_topology_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPILED_CBSEM_PLAN_V3_SCHEMA_VERSION: u32 = 3;
pub const COMPILED_CBSEM_IDENTIFICATION_EVIDENCE_V3_SCHEMA_VERSION: u32 = 1;

/// The structural form represented by the compiled topology.
///
/// `Feedback` is a representation fact only. It does not imply that any
/// estimator is qualified to execute the model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemStructuralFormV3 {
    Recursive,
    Feedback,
}

/// Deliberately conservative identification state.
///
/// V3 records inspectable restrictions but does not yet perform a Jacobian-rank
/// proof or other global identification theorem. Recursive plans are therefore
/// provisional. Feedback plans remain experimental or blocked.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemIdentificationStatusV3 {
    Provisional,
    ExperimentalOrBlocked,
}

/// Execution readiness is intentionally separate from model representation and
/// identification evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemExecutionDispositionV3 {
    RequiresSeparateCapabilityValidation,
    FeedbackExecutionBlocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemParameterTableSourceV3 {
    EmbeddedCompiledCbsemPlanV2,
}

/// Tamper-evident descriptor for the complete parameter table embedded in the
/// unchanged v2 base plan. V3 never projects a role-specific subset as the
/// scientific authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemParameterTableAuthorityV3 {
    source: CompiledCbsemParameterTableSourceV3,
    row_count: u64,
    ordered_parameter_ids: Vec<String>,
    parameter_table_sha256: String,
}

impl CompiledCbsemParameterTableAuthorityV3 {
    pub fn source(&self) -> CompiledCbsemParameterTableSourceV3 {
        self.source
    }

    pub fn row_count(&self) -> u64 {
        self.row_count
    }

    pub fn ordered_parameter_ids(&self) -> &[String] {
        &self.ordered_parameter_ids
    }

    pub fn parameter_table_sha256(&self) -> &str {
        &self.parameter_table_sha256
    }
}

/// Stable SCC evidence copied from the general SEM topology with an identity
/// derived only from its canonical node and internal-relation ids.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemSccIdentificationEvidenceV3 {
    identity: String,
    node_ids: Vec<String>,
    relation_ids: Vec<String>,
    has_feedback: bool,
}

impl CompiledCbsemSccIdentificationEvidenceV3 {
    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn relation_ids(&self) -> &[String] {
        &self.relation_ids
    }

    pub fn has_feedback(&self) -> bool {
        self.has_feedback
    }
}

/// Local factor-scale restrictions proven by `SemModelV4` validation and
/// retained as diagnostic evidence. These are not a global rank proof.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledCbsemFactorScaleRestrictionV3 {
    MarkerLoading {
        factor_id: String,
        indicator_id: String,
        parameter_id: String,
    },
    FixedVariance {
        factor_id: String,
        parameter_id: String,
    },
    EffectsCoding {
        factor_id: String,
        loading_parameter_ids: Vec<String>,
        supporting_constraint_ids: Vec<String>,
    },
}

impl CompiledCbsemFactorScaleRestrictionV3 {
    pub fn factor_id(&self) -> &str {
        match self {
            Self::MarkerLoading { factor_id, .. }
            | Self::FixedVariance { factor_id, .. }
            | Self::EffectsCoding { factor_id, .. } => factor_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemEqualityLabelRestrictionV3 {
    label: String,
    parameter_ids: Vec<String>,
}

impl CompiledCbsemEqualityLabelRestrictionV3 {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn parameter_ids(&self) -> &[String] {
        &self.parameter_ids
    }
}

/// Inspectable identification diagnostics. The exact constraints and all fixed
/// or equality-labelled parameter identities remain explicit. Status remains
/// conservative because local restrictions alone do not prove global
/// identification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemIdentificationEvidenceV3 {
    schema_version: u32,
    status: CompiledCbsemIdentificationStatusV3,
    structural_form: CompiledCbsemStructuralFormV3,
    execution_disposition: CompiledCbsemExecutionDispositionV3,
    strongly_connected_components: Vec<CompiledCbsemSccIdentificationEvidenceV3>,
    factor_scale_restrictions: Vec<CompiledCbsemFactorScaleRestrictionV3>,
    fixed_parameter_ids: Vec<String>,
    equality_label_restrictions: Vec<CompiledCbsemEqualityLabelRestrictionV3>,
    explicit_constraints: Vec<SemConstraintV4>,
}

impl CompiledCbsemIdentificationEvidenceV3 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn status(&self) -> CompiledCbsemIdentificationStatusV3 {
        self.status
    }

    pub fn structural_form(&self) -> CompiledCbsemStructuralFormV3 {
        self.structural_form
    }

    pub fn execution_disposition(&self) -> CompiledCbsemExecutionDispositionV3 {
        self.execution_disposition
    }

    pub fn strongly_connected_components(&self) -> &[CompiledCbsemSccIdentificationEvidenceV3] {
        &self.strongly_connected_components
    }

    pub fn factor_scale_restrictions(&self) -> &[CompiledCbsemFactorScaleRestrictionV3] {
        &self.factor_scale_restrictions
    }

    pub fn fixed_parameter_ids(&self) -> &[String] {
        &self.fixed_parameter_ids
    }

    pub fn equality_label_restrictions(&self) -> &[CompiledCbsemEqualityLabelRestrictionV3] {
        &self.equality_label_restrictions
    }

    pub fn explicit_constraints(&self) -> &[SemConstraintV4] {
        &self.explicit_constraints
    }
}

/// CB-SEM v3 compile-time representation and diagnostic contract.
///
/// The proven v2 plan is embedded unchanged and remains the sole complete
/// parameter-table authority. V3 adds general topology and conservative,
/// inspectable identification evidence. Estimator capability remains a
/// separate decision; in particular, feedback execution is explicitly blocked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledCbsemPlanV3 {
    schema_version: u32,
    model_id: String,
    scientific_sha256: String,
    general_sem_config_sha256: String,
    base_plan_sha256: String,
    base_plan: CompiledCbsemPlanV2,
    topology: CompiledSemTopologyV1,
    parameter_table_authority: CompiledCbsemParameterTableAuthorityV3,
    identification_evidence: CompiledCbsemIdentificationEvidenceV3,
}

impl CompiledCbsemPlanV3 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn scientific_sha256(&self) -> &str {
        &self.scientific_sha256
    }

    /// Compatibility alias matching the embedded v2 naming.
    pub fn scientific_hash(&self) -> &str {
        &self.scientific_sha256
    }

    pub fn general_sem_config_sha256(&self) -> &str {
        &self.general_sem_config_sha256
    }

    pub fn base_plan_sha256(&self) -> &str {
        &self.base_plan_sha256
    }

    pub fn base_plan(&self) -> &CompiledCbsemPlanV2 {
        &self.base_plan
    }

    pub fn topology(&self) -> &CompiledSemTopologyV1 {
        &self.topology
    }

    pub fn parameter_table_authority(&self) -> &CompiledCbsemParameterTableAuthorityV3 {
        &self.parameter_table_authority
    }

    /// Returns every authoritative row without projection or role filtering.
    pub fn parameters(&self) -> &[CompiledCbsemParameterRowV2] {
        self.base_plan.parameters()
    }

    pub fn identification_evidence(&self) -> &CompiledCbsemIdentificationEvidenceV3 {
        &self.identification_evidence
    }

    pub fn deterministic_sha256(&self) -> String {
        sha256_serialized(self)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum CompiledCbsemPlanV3Error {
    #[error(transparent)]
    InvalidGeneralSemConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error(transparent)]
    Topology(#[from] CompiledSemTopologyV1Error),
    #[error(transparent)]
    BasePlan(#[from] CompiledCbsemPlanV2Error),
    #[error("compiled CB-SEM v3 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotImplemented,
    #[error(
        "compiled CB-SEM v2 and topology scientific identities differ: base={base_sha256}, topology={topology_sha256}"
    )]
    ScientificIdentityMismatch {
        base_sha256: String,
        topology_sha256: String,
    },
    #[error("validated CB-SEM identification evidence is incomplete ({code}): {subject}")]
    IdentificationEvidenceInvariant { code: &'static str, subject: String },
}

/// Compiles a deterministic CB-SEM v3 representation without claiming
/// estimator support. Feedback is retained with an explicit blocked execution
/// disposition rather than rejected or silently removed.
pub fn compile_cbsem_plan_v3(
    model: &crate::SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<CompiledCbsemPlanV3, CompiledCbsemPlanV3Error> {
    config.ensure_valid()?;
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(CompiledCbsemPlanV3Error::LazySpecificPathMaterializationNotImplemented);
    }
    let topology = compile_sem_topology_v1(
        model,
        config.output_policy.max_materialized_specific_paths as usize,
    )?;
    let base_plan = compile_cbsem_plan_v2(model)?;

    if base_plan.scientific_hash() != topology.model_scientific_sha256() {
        return Err(CompiledCbsemPlanV3Error::ScientificIdentityMismatch {
            base_sha256: base_plan.scientific_hash().to_string(),
            topology_sha256: topology.model_scientific_sha256().to_string(),
        });
    }
    let parameter_table_authority = compile_parameter_table_authority(&base_plan);
    let identification_evidence = compile_identification_evidence(&base_plan, &topology)?;
    let scientific_sha256 = base_plan.scientific_hash().to_string();
    let base_plan_sha256 = base_plan.deterministic_sha256();

    Ok(CompiledCbsemPlanV3 {
        schema_version: COMPILED_CBSEM_PLAN_V3_SCHEMA_VERSION,
        model_id: base_plan.model_id().to_string(),
        scientific_sha256,
        general_sem_config_sha256: sha256_serialized(config),
        base_plan_sha256,
        base_plan,
        topology,
        parameter_table_authority,
        identification_evidence,
    })
}

fn compile_parameter_table_authority(
    base_plan: &CompiledCbsemPlanV2,
) -> CompiledCbsemParameterTableAuthorityV3 {
    CompiledCbsemParameterTableAuthorityV3 {
        source: CompiledCbsemParameterTableSourceV3::EmbeddedCompiledCbsemPlanV2,
        row_count: base_plan.parameters().len() as u64,
        ordered_parameter_ids: base_plan
            .parameters()
            .iter()
            .map(|parameter| parameter.id().to_string())
            .collect(),
        parameter_table_sha256: sha256_serialized(base_plan.parameters()),
    }
}

fn compile_identification_evidence(
    base_plan: &CompiledCbsemPlanV2,
    topology: &CompiledSemTopologyV1,
) -> Result<CompiledCbsemIdentificationEvidenceV3, CompiledCbsemPlanV3Error> {
    let structural_form = if topology.has_feedback() {
        CompiledCbsemStructuralFormV3::Feedback
    } else {
        CompiledCbsemStructuralFormV3::Recursive
    };
    let (status, execution_disposition) = match structural_form {
        CompiledCbsemStructuralFormV3::Recursive => (
            CompiledCbsemIdentificationStatusV3::Provisional,
            CompiledCbsemExecutionDispositionV3::RequiresSeparateCapabilityValidation,
        ),
        CompiledCbsemStructuralFormV3::Feedback => (
            CompiledCbsemIdentificationStatusV3::ExperimentalOrBlocked,
            CompiledCbsemExecutionDispositionV3::FeedbackExecutionBlocked,
        ),
    };

    let strongly_connected_components = topology
        .strongly_connected_components()
        .iter()
        .map(compile_scc_evidence)
        .collect();
    let factor_scale_restrictions = compile_factor_scale_restrictions(base_plan)?;
    let fixed_parameter_ids = base_plan
        .parameters()
        .iter()
        .filter_map(|parameter| {
            matches!(
                parameter.specification(),
                CompiledCbsemParameterStatusV2::Fixed { .. }
            )
            .then(|| parameter.id().to_string())
        })
        .collect();

    let mut equality_labels = BTreeMap::<String, Vec<String>>::new();
    for parameter in base_plan.parameters() {
        let CompiledCbsemParameterStatusV2::Free {
            equality_label: Some(label),
            ..
        } = parameter.specification()
        else {
            continue;
        };
        equality_labels
            .entry(label.clone())
            .or_default()
            .push(parameter.id().to_string());
    }
    let equality_label_restrictions = equality_labels
        .into_iter()
        .map(
            |(label, parameter_ids)| CompiledCbsemEqualityLabelRestrictionV3 {
                label,
                parameter_ids,
            },
        )
        .collect();

    Ok(CompiledCbsemIdentificationEvidenceV3 {
        schema_version: COMPILED_CBSEM_IDENTIFICATION_EVIDENCE_V3_SCHEMA_VERSION,
        status,
        structural_form,
        execution_disposition,
        strongly_connected_components,
        factor_scale_restrictions,
        fixed_parameter_ids,
        equality_label_restrictions,
        explicit_constraints: base_plan.constraints().to_vec(),
    })
}

fn compile_scc_evidence(
    component: &CompiledSemStronglyConnectedComponentV1,
) -> CompiledCbsemSccIdentificationEvidenceV3 {
    CompiledCbsemSccIdentificationEvidenceV3 {
        identity: stable_scc_identity(component.node_ids(), component.relation_ids()),
        node_ids: component.node_ids().to_vec(),
        relation_ids: component.relation_ids().to_vec(),
        has_feedback: component.has_feedback(),
    }
}

fn compile_factor_scale_restrictions(
    base_plan: &CompiledCbsemPlanV2,
) -> Result<Vec<CompiledCbsemFactorScaleRestrictionV3>, CompiledCbsemPlanV3Error> {
    let mut restrictions = Vec::new();
    for variable in base_plan.variables() {
        let SemVariableV4::CommonFactor {
            id, identification, ..
        } = variable
        else {
            continue;
        };
        match identification {
            FactorIdentificationV4::MarkerLoading { indicator } => {
                let loading = base_plan
                    .loadings()
                    .iter()
                    .find(|loading| loading.construct() == id && loading.indicator() == indicator)
                    .ok_or_else(|| {
                        identification_invariant("marker_loading_missing", id.clone())
                    })?;
                restrictions.push(CompiledCbsemFactorScaleRestrictionV3::MarkerLoading {
                    factor_id: id.clone(),
                    indicator_id: indicator.clone(),
                    parameter_id: loading.parameter_id().to_string(),
                });
            }
            FactorIdentificationV4::FixedVariance => {
                let parameter = base_plan
                    .parameters()
                    .iter()
                    .find(|parameter| {
                        matches!(
                            parameter.target(),
                            SemParameterTargetV4::Variance {
                                endpoint: SemEndpointV4::Variable(variable_id),
                            } if variable_id == id
                        ) && matches!(
                            parameter.specification(),
                            CompiledCbsemParameterStatusV2::Fixed { value } if *value == 1.0
                        )
                    })
                    .ok_or_else(|| {
                        identification_invariant("fixed_variance_missing", id.clone())
                    })?;
                restrictions.push(CompiledCbsemFactorScaleRestrictionV3::FixedVariance {
                    factor_id: id.clone(),
                    parameter_id: parameter.id().to_string(),
                });
            }
            FactorIdentificationV4::EffectsCoding => {
                let loading_parameter_ids = base_plan
                    .loadings()
                    .iter()
                    .filter(|loading| loading.construct() == id)
                    .map(|loading| loading.parameter_id().to_string())
                    .collect::<Vec<_>>();
                let loading_parameter_set = loading_parameter_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                let supporting_constraint_ids = base_plan
                    .constraints()
                    .iter()
                    .filter_map(|constraint| match constraint {
                        SemConstraintV4::Linear {
                            id: constraint_id,
                            terms,
                            ..
                        } if terms.len() == loading_parameter_set.len()
                            && terms.iter().all(|term| {
                                loading_parameter_set.contains(term.parameter.as_str())
                            }) =>
                        {
                            Some(constraint_id.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if loading_parameter_ids.is_empty() || supporting_constraint_ids.is_empty() {
                    return Err(identification_invariant(
                        "effects_coding_constraint_missing",
                        id.clone(),
                    ));
                }
                restrictions.push(CompiledCbsemFactorScaleRestrictionV3::EffectsCoding {
                    factor_id: id.clone(),
                    loading_parameter_ids,
                    supporting_constraint_ids,
                });
            }
        }
    }
    Ok(restrictions)
}

fn stable_scc_identity(node_ids: &[String], relation_ids: &[String]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-cbsem-plan-v3.scc-identification-evidence\0");
    digest.update((node_ids.len() as u64).to_be_bytes());
    for node_id in node_ids {
        update_digest_value(&mut digest, node_id);
    }
    digest.update((relation_ids.len() as u64).to_be_bytes());
    for relation_id in relation_ids {
        update_digest_value(&mut digest, relation_id);
    }
    format!("cbsem_scc_v1_{:x}", digest.finalize())
}

fn update_digest_value(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn sha256_serialized<T: Serialize + ?Sized>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("compiled CB-SEM contract must serialize"))
    )
}

fn identification_invariant(code: &'static str, subject: String) -> CompiledCbsemPlanV3Error {
    CompiledCbsemPlanV3Error::IdentificationEvidenceInvariant { code, subject }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, SemModelV4,
        SemParameterV4, SemRelationV4, StructuralPath, StructuralRelationRoleV4,
        convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn model_with_paths(paths: &[(&str, &str)]) -> SemModelV4 {
        let node_ids = paths
            .iter()
            .flat_map(|(source, target)| [*source, *target])
            .collect::<BTreeSet<_>>();
        let constructs = node_ids
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = paths
            .iter()
            .map(|(source, target)| StructuralPath {
                source: (*source).into(),
                target: (*target).into(),
            })
            .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0xcb53_0003),
                name: "CB-SEM v3 fixture".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    fn recursive_model() -> SemModelV4 {
        model_with_paths(&[("x", "y")])
    }

    fn feedback_model() -> SemModelV4 {
        let mut model = recursive_model();
        model.relations.push(SemRelationV4::Structural {
            id: "feedback:y-to-x".into(),
            source: "construct:y".into(),
            target: "construct:x".into(),
            parameter: "parameter:feedback:y-to-x".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:feedback:y-to-x".into(),
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

        let variance_parameter_id = match model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:x")
            .unwrap()
        {
            SemVariableV4::CommonFactor {
                disturbance_policy, ..
            } => match disturbance_policy {
                crate::FactorDisturbancePolicyV4::ExogenousVariance { parameter } => {
                    let parameter_id = parameter.clone();
                    *disturbance_policy = crate::FactorDisturbancePolicyV4::EndogenousDisturbance {
                        parameter: parameter_id.clone(),
                    };
                    parameter_id
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        match model
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id() == variance_parameter_id)
            .unwrap()
        {
            SemParameterV4::Free { target, .. } => {
                *target = SemParameterTargetV4::Variance {
                    endpoint: SemEndpointV4::DisturbanceOf("construct:x".into()),
                };
            }
            _ => unreachable!(),
        }
        model
    }

    #[test]
    fn recursive_plan_wraps_v2_and_keeps_the_complete_parameter_authority() {
        let model = recursive_model();
        let config = GeneralSemConfigV1::default();
        let expected_v2 = compile_cbsem_plan_v2(&model).unwrap();
        let plan = compile_cbsem_plan_v3(&model, &config).unwrap();

        assert_eq!(plan.schema_version(), COMPILED_CBSEM_PLAN_V3_SCHEMA_VERSION);
        assert_eq!(plan.base_plan(), &expected_v2);
        assert_eq!(plan.scientific_hash(), expected_v2.scientific_hash());
        assert_eq!(plan.base_plan_sha256(), expected_v2.deterministic_sha256());
        assert!(!plan.topology().has_feedback());
        assert_eq!(
            plan.identification_evidence().structural_form(),
            CompiledCbsemStructuralFormV3::Recursive
        );
        assert_eq!(
            plan.identification_evidence().status(),
            CompiledCbsemIdentificationStatusV3::Provisional
        );
        assert_eq!(
            plan.identification_evidence().execution_disposition(),
            CompiledCbsemExecutionDispositionV3::RequiresSeparateCapabilityValidation
        );

        let authority = plan.parameter_table_authority();
        assert_eq!(
            authority.source(),
            CompiledCbsemParameterTableSourceV3::EmbeddedCompiledCbsemPlanV2
        );
        assert_eq!(authority.row_count(), expected_v2.parameters().len() as u64);
        assert_eq!(plan.parameters(), expected_v2.parameters());
        assert_eq!(
            authority.ordered_parameter_ids(),
            expected_v2
                .parameters()
                .iter()
                .map(|parameter| parameter.id().to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            authority.parameter_table_sha256(),
            sha256_serialized(expected_v2.parameters())
        );
    }

    #[test]
    fn feedback_is_preserved_with_stable_scc_evidence_and_blocked_execution() {
        let model = feedback_model();
        let plan = compile_cbsem_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();

        assert!(plan.base_plan().has_feedback());
        assert!(plan.topology().has_feedback());
        let evidence = plan.identification_evidence();
        assert_eq!(
            evidence.structural_form(),
            CompiledCbsemStructuralFormV3::Feedback
        );
        assert_eq!(
            evidence.status(),
            CompiledCbsemIdentificationStatusV3::ExperimentalOrBlocked
        );
        assert_eq!(
            evidence.execution_disposition(),
            CompiledCbsemExecutionDispositionV3::FeedbackExecutionBlocked
        );
        let feedback_sccs = evidence
            .strongly_connected_components()
            .iter()
            .filter(|component| component.has_feedback())
            .collect::<Vec<_>>();
        assert_eq!(feedback_sccs.len(), 1);
        assert_eq!(
            feedback_sccs[0].node_ids(),
            &["construct:x".to_string(), "construct:y".to_string()]
        );
        assert_eq!(feedback_sccs[0].relation_ids().len(), 2);
        assert!(feedback_sccs[0].identity().starts_with("cbsem_scc_v1_"));

        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        assert_eq!(
            compile_cbsem_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap(),
            plan
        );
    }

    #[test]
    fn factor_fixed_equality_label_and_explicit_constraint_evidence_is_complete() {
        let mut model = recursive_model();
        let mut equality_parameter_ids = Vec::new();
        for parameter in &mut model.parameters {
            let SemParameterV4::Free {
                id,
                target: SemParameterTargetV4::Loading { indicator, .. },
                equality_label,
                ..
            } = parameter
            else {
                continue;
            };
            if indicator.ends_with("x2") || indicator.ends_with("y2") {
                *equality_label = Some("metric-equality".into());
                equality_parameter_ids.push(id.clone());
            }
        }
        equality_parameter_ids.sort();
        assert_eq!(equality_parameter_ids.len(), 2);

        let regression_parameter_id = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural { parameter, .. } => Some(parameter.clone()),
                _ => None,
            })
            .unwrap();
        model.constraints.push(SemConstraintV4::Bound {
            id: "constraint:regression-bound".into(),
            parameter: regression_parameter_id,
            lower: Some(-1.0),
            upper: Some(1.0),
        });

        let plan = compile_cbsem_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let evidence = plan.identification_evidence();
        assert_eq!(evidence.factor_scale_restrictions().len(), 2);
        assert!(
            evidence
                .factor_scale_restrictions()
                .iter()
                .all(|restriction| {
                    matches!(
                        restriction,
                        CompiledCbsemFactorScaleRestrictionV3::MarkerLoading { .. }
                    )
                })
        );
        assert_eq!(
            evidence
                .equality_label_restrictions()
                .iter()
                .find(|restriction| restriction.label() == "metric-equality")
                .unwrap()
                .parameter_ids(),
            equality_parameter_ids
        );
        assert_eq!(
            evidence.explicit_constraints(),
            plan.base_plan().constraints()
        );

        let expected_fixed_ids = plan
            .base_plan()
            .parameters()
            .iter()
            .filter_map(|parameter| {
                matches!(
                    parameter.specification(),
                    CompiledCbsemParameterStatusV2::Fixed { .. }
                )
                .then(|| parameter.id().to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(evidence.fixed_parameter_ids(), expected_fixed_ids);
    }

    #[test]
    fn declaration_order_and_wire_roundtrip_are_exact_while_config_digest_is_sensitive() {
        let model = model_with_paths(&[("x", "m"), ("m", "y"), ("x", "y")]);
        let config = GeneralSemConfigV1::default();
        let expected = compile_cbsem_plan_v3(&model, &config).unwrap();
        let mut reordered = model.clone();
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        reordered.constraints.reverse();
        assert_eq!(
            compile_cbsem_plan_v3(&reordered, &config).unwrap(),
            expected
        );

        let encoded = serde_json::to_vec(&expected).unwrap();
        assert_eq!(
            serde_json::from_slice::<CompiledCbsemPlanV3>(&encoded).unwrap(),
            expected
        );
        let mut unknown = serde_json::to_value(&expected).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CompiledCbsemPlanV3>(unknown).is_err());

        let mut changed_config = config;
        changed_config.output_policy.max_materialized_specific_paths += 1;
        let changed = compile_cbsem_plan_v3(&model, &changed_config).unwrap();
        assert_eq!(changed.scientific_hash(), expected.scientific_hash());
        assert_eq!(changed.base_plan(), expected.base_plan());
        assert_eq!(changed.topology(), expected.topology());
        assert_ne!(
            changed.general_sem_config_sha256(),
            expected.general_sem_config_sha256()
        );
        assert_ne!(
            changed.deterministic_sha256(),
            expected.deterministic_sha256()
        );
    }

    #[test]
    fn invalid_config_and_path_resource_limits_remain_typed() {
        let model = recursive_model();
        let mut invalid_config = GeneralSemConfigV1::default();
        invalid_config.output_policy.max_materialized_specific_paths = 0;
        assert_eq!(
            compile_cbsem_plan_v3(&model, &invalid_config),
            Err(CompiledCbsemPlanV3Error::InvalidGeneralSemConfig(
                GeneralSemConfigV1ValidationError::ZeroMaxMaterializedSpecificPaths
            ))
        );

        let many_paths = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        let mut bounded = GeneralSemConfigV1::default();
        bounded.output_policy.max_materialized_specific_paths = 1;
        assert_eq!(
            compile_cbsem_plan_v3(&many_paths, &bounded),
            Err(CompiledCbsemPlanV3Error::Topology(
                CompiledSemTopologyV1Error::SpecificPathResourceLimitExceeded { max_paths: 1 }
            ))
        );
    }

    #[test]
    fn direct_cbsem_v3_compilation_rejects_lazy_path_materialization() {
        let model = recursive_model();
        let mut config = GeneralSemConfigV1::default();
        config.output_policy.lazy_specific_path_materialization = true;
        config.output_policy.when_specific_path_limit_exceeded =
            GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy;

        assert_eq!(
            compile_cbsem_plan_v3(&model, &config),
            Err(CompiledCbsemPlanV3Error::LazySpecificPathMaterializationNotImplemented)
        );
    }
}
