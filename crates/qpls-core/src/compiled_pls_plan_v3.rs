use crate::{
    CompiledPlsPlanV2, CompiledPlsPlanV2Error, CompiledSemSpecificDirectedPathV1,
    CompiledSemTopologyV1, CompiledSemTopologyV1Error, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemEffectEstimandV1,
    GeneralSemSpecificPathLimitBehaviorV1, SemModelV4, StructuralRelationRoleV4,
    compile_pls_plan_v2, compile_sem_topology_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const COMPILED_PLS_PLAN_V3_SCHEMA_VERSION: u32 = 3;

/// Stable effect identities compiled from ordinary directed structural paths.
/// Mediation remains a topology-derived estimand rather than a special model
/// object. Every contributing relation and path remains inspectable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledPlsEffectEstimandV3 {
    SpecificIndirect {
        estimand_id: String,
        path_identity: String,
        source_id: String,
        target_id: String,
        ordered_relation_ids: Vec<String>,
    },
    TotalIndirect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        contributing_path_identities: Vec<String>,
    },
    TotalEffect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        direct_relation_ids: Vec<String>,
        contributing_indirect_path_identities: Vec<String>,
    },
}

impl CompiledPlsEffectEstimandV3 {
    pub fn estimand_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { estimand_id, .. }
            | Self::TotalIndirect { estimand_id, .. }
            | Self::TotalEffect { estimand_id, .. } => estimand_id,
        }
    }

    pub fn source_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { source_id, .. }
            | Self::TotalIndirect { source_id, .. }
            | Self::TotalEffect { source_id, .. } => source_id,
        }
    }

    pub fn target_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { target_id, .. }
            | Self::TotalIndirect { target_id, .. }
            | Self::TotalEffect { target_id, .. } => target_id,
        }
    }
}

/// PLS v3 foundation for general recursive path models. The proven v2 scoring
/// plan remains embedded unchanged while topology/effect authority is added.
/// Derived-stage interactions and HOCs are intentionally rejected by the v2
/// scoring compiler until their exact v3 executor cells are implemented.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsPlanV3 {
    schema_version: u32,
    model_id: String,
    scientific_hash: String,
    general_sem_config_sha256: String,
    base_plan: CompiledPlsPlanV2,
    topology: CompiledSemTopologyV1,
    effect_estimands: Vec<CompiledPlsEffectEstimandV3>,
    auto_selected_effects: bool,
}

impl CompiledPlsPlanV3 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn scientific_hash(&self) -> &str {
        &self.scientific_hash
    }

    pub fn general_sem_config_sha256(&self) -> &str {
        &self.general_sem_config_sha256
    }

    pub fn base_plan(&self) -> &CompiledPlsPlanV2 {
        &self.base_plan
    }

    pub fn topology(&self) -> &CompiledSemTopologyV1 {
        &self.topology
    }

    pub fn effect_estimands(&self) -> &[CompiledPlsEffectEstimandV3] {
        &self.effect_estimands
    }

    pub fn auto_selected_effects(&self) -> bool {
        self.auto_selected_effects
    }

    pub fn deterministic_sha256(&self) -> String {
        sha256_serialized(self)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum CompiledPlsPlanV3Error {
    #[error(transparent)]
    InvalidGeneralSemConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error(transparent)]
    Topology(#[from] CompiledSemTopologyV1Error),
    #[error(transparent)]
    BasePlan(#[from] CompiledPlsPlanV2Error),
    #[error("compiled PLS v3 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotImplemented,
    #[error("PLS v3 requires an acyclic structural topology")]
    StructuralFeedback,
    #[error("aggregate estimand id {estimand_id} collides with a canonical specific-path identity")]
    AggregateEstimandIdCollidesWithSpecificPathIdentity { estimand_id: String },
    #[error("requested specific indirect estimand {estimand_id} is not an exact compiled path")]
    UnknownSpecificIndirectPath { estimand_id: String },
    #[error(
        "requested {kind} estimand {estimand_id} has no eligible path from {source_id} to {target_id}"
    )]
    UnreachableEffect {
        kind: &'static str,
        estimand_id: String,
        source_id: String,
        target_id: String,
    },
}

pub fn compile_pls_plan_v3(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<CompiledPlsPlanV3, CompiledPlsPlanV3Error> {
    config.ensure_valid()?;
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(CompiledPlsPlanV3Error::LazySpecificPathMaterializationNotImplemented);
    }
    let topology = compile_sem_topology_v1(
        model,
        config.output_policy.max_materialized_specific_paths as usize,
    )?;
    if topology.has_feedback() {
        return Err(CompiledPlsPlanV3Error::StructuralFeedback);
    }
    let base_plan = compile_pls_plan_v2(model)?;
    let auto_selected_effects = config.requested_effect_estimands.is_empty();
    let effect_estimands = if auto_selected_effects {
        compile_all_effect_estimands(&topology)
    } else {
        compile_requested_effect_estimands(&topology, config)?
    };
    let scientific_hash = base_plan.scientific_hash().to_string();
    debug_assert_eq!(base_plan.scientific_hash(), scientific_hash.as_str());
    debug_assert_eq!(topology.model_scientific_sha256(), scientific_hash);
    Ok(CompiledPlsPlanV3 {
        schema_version: COMPILED_PLS_PLAN_V3_SCHEMA_VERSION,
        model_id: model.id.clone(),
        scientific_hash,
        general_sem_config_sha256: sha256_serialized(config),
        base_plan,
        topology,
        effect_estimands,
        auto_selected_effects,
    })
}

fn compile_requested_effect_estimands(
    topology: &CompiledSemTopologyV1,
    config: &GeneralSemConfigV1,
) -> Result<Vec<CompiledPlsEffectEstimandV3>, CompiledPlsPlanV3Error> {
    let reserved_specific_path_identities = topology
        .specific_directed_paths()
        .iter()
        .map(|path| path.identity().to_string())
        .collect::<BTreeSet<_>>();
    let mut compiled = Vec::with_capacity(config.requested_effect_estimands.len());
    for request in &config.requested_effect_estimands {
        match request {
            GeneralSemEffectEstimandV1::SpecificPath {
                estimand_id,
                ordered_relation_ids,
            } => {
                let path = topology
                    .specific_directed_paths()
                    .iter()
                    .find(|path| path.relation_ids() == ordered_relation_ids)
                    .ok_or_else(|| CompiledPlsPlanV3Error::UnknownSpecificIndirectPath {
                        estimand_id: estimand_id.clone(),
                    })?;
                compiled.push(specific_estimand(estimand_id.clone(), path));
            }
            GeneralSemEffectEstimandV1::TotalIndirect {
                estimand_id,
                source_id,
                target_id,
            } => {
                reject_aggregate_specific_path_identity_collision(
                    &reserved_specific_path_identities,
                    estimand_id,
                )?;
                let path_ids = indirect_path_ids(topology, source_id, target_id);
                if path_ids.is_empty() {
                    return Err(CompiledPlsPlanV3Error::UnreachableEffect {
                        kind: "total_indirect",
                        estimand_id: estimand_id.clone(),
                        source_id: source_id.clone(),
                        target_id: target_id.clone(),
                    });
                }
                compiled.push(CompiledPlsEffectEstimandV3::TotalIndirect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    contributing_path_identities: path_ids,
                });
            }
            GeneralSemEffectEstimandV1::TotalEffect {
                estimand_id,
                source_id,
                target_id,
            } => {
                reject_aggregate_specific_path_identity_collision(
                    &reserved_specific_path_identities,
                    estimand_id,
                )?;
                let direct_relation_ids = direct_relation_ids(topology, source_id, target_id);
                let path_ids = indirect_path_ids(topology, source_id, target_id);
                if direct_relation_ids.is_empty() && path_ids.is_empty() {
                    return Err(CompiledPlsPlanV3Error::UnreachableEffect {
                        kind: "total_effect",
                        estimand_id: estimand_id.clone(),
                        source_id: source_id.clone(),
                        target_id: target_id.clone(),
                    });
                }
                compiled.push(CompiledPlsEffectEstimandV3::TotalEffect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    direct_relation_ids,
                    contributing_indirect_path_identities: path_ids,
                });
            }
        }
    }
    Ok(compiled)
}

fn reject_aggregate_specific_path_identity_collision(
    reserved_specific_path_identities: &BTreeSet<String>,
    estimand_id: &str,
) -> Result<(), CompiledPlsPlanV3Error> {
    if reserved_specific_path_identities.contains(estimand_id) {
        Err(
            CompiledPlsPlanV3Error::AggregateEstimandIdCollidesWithSpecificPathIdentity {
                estimand_id: estimand_id.to_string(),
            },
        )
    } else {
        Ok(())
    }
}

fn compile_all_effect_estimands(
    topology: &CompiledSemTopologyV1,
) -> Vec<CompiledPlsEffectEstimandV3> {
    let mut compiled = topology
        .specific_directed_paths()
        .iter()
        .map(|path| specific_estimand(path.identity().to_string(), path))
        .collect::<Vec<_>>();
    let mut pairs = BTreeSet::new();
    for relation in topology.structural_relations() {
        if relation.role() == StructuralRelationRoleV4::Structural {
            pairs.insert((relation.source().to_string(), relation.target().to_string()));
        }
    }
    for path in topology.specific_directed_paths() {
        pairs.insert((path.source().to_string(), path.target().to_string()));
    }
    for (source_id, target_id) in pairs {
        let path_ids = indirect_path_ids(topology, &source_id, &target_id);
        if !path_ids.is_empty() {
            compiled.push(CompiledPlsEffectEstimandV3::TotalIndirect {
                estimand_id: auto_effect_identity("total_indirect", &source_id, &target_id),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                contributing_path_identities: path_ids.clone(),
            });
        }
        compiled.push(CompiledPlsEffectEstimandV3::TotalEffect {
            estimand_id: auto_effect_identity("total_effect", &source_id, &target_id),
            direct_relation_ids: direct_relation_ids(topology, &source_id, &target_id),
            source_id,
            target_id,
            contributing_indirect_path_identities: path_ids,
        });
    }
    compiled.sort_by(|left, right| left.estimand_id().cmp(right.estimand_id()));
    compiled
}

fn specific_estimand(
    estimand_id: String,
    path: &CompiledSemSpecificDirectedPathV1,
) -> CompiledPlsEffectEstimandV3 {
    CompiledPlsEffectEstimandV3::SpecificIndirect {
        estimand_id,
        path_identity: path.identity().to_string(),
        source_id: path.source().to_string(),
        target_id: path.target().to_string(),
        ordered_relation_ids: path.relation_ids().to_vec(),
    }
}

fn indirect_path_ids(topology: &CompiledSemTopologyV1, source: &str, target: &str) -> Vec<String> {
    topology
        .specific_directed_paths()
        .iter()
        .filter(|path| path.source() == source && path.target() == target)
        .map(|path| path.identity().to_string())
        .collect()
}

fn direct_relation_ids(
    topology: &CompiledSemTopologyV1,
    source: &str,
    target: &str,
) -> Vec<String> {
    topology
        .structural_relations()
        .iter()
        .filter(|relation| {
            relation.role() == StructuralRelationRoleV4::Structural
                && relation.source() == source
                && relation.target() == target
        })
        .map(|relation| relation.relation_id().to_string())
        .collect()
}

fn auto_effect_identity(kind: &str, source: &str, target: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-pls-plan-v3.effect\0");
    for value in [kind, source, target] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    format!("sem_{kind}_v1_{:x}", digest.finalize())
}

fn sha256_serialized<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("compiled SEM contract serializes"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, GeneralSemEffectEstimandV1, LegacyBasicModelInterpretationV4, MeasurementMode,
        ModelSpec, StructuralPath, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn recursive_model() -> SemModelV4 {
        let constructs = ["x", "m1", "m2", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [
            ("x", "m1"),
            ("x", "m2"),
            ("x", "y"),
            ("m1", "m2"),
            ("m1", "y"),
            ("m2", "y"),
        ]
        .into_iter()
        .map(|(source, target)| StructuralPath {
            source: source.into(),
            target: target.into(),
        })
        .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_5303),
                name: "Parallel and serial mediation".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                crate::SemRelationV4::Structural {
                    id,
                    source: relation_source,
                    target: relation_target,
                    ..
                } if relation_source == source && relation_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn default_config_infers_all_specific_and_aggregate_effects_deterministically() {
        let model = recursive_model();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(plan.schema_version(), 3);
        assert!(plan.auto_selected_effects());
        assert!(plan.effect_estimands().iter().any(|estimand| matches!(
            estimand,
            CompiledPlsEffectEstimandV3::SpecificIndirect {
                source_id,
                target_id,
                ordered_relation_ids,
                ..
            } if source_id == "construct:x" && target_id == "construct:y"
                && ordered_relation_ids.len() == 3
        )));
        assert!(plan.effect_estimands().iter().any(|estimand| matches!(
            estimand,
            CompiledPlsEffectEstimandV3::TotalIndirect {
                source_id,
                target_id,
                contributing_path_identities,
                ..
            } if source_id == "construct:x" && target_id == "construct:y"
                && contributing_path_identities.len() >= 3
        )));
        let encoded = serde_json::to_vec(&plan).unwrap();
        assert_eq!(
            serde_json::from_slice::<CompiledPlsPlanV3>(&encoded).unwrap(),
            plan
        );
        assert_eq!(
            compile_pls_plan_v3(&model, &GeneralSemConfigV1::default())
                .unwrap()
                .deterministic_sha256(),
            plan.deterministic_sha256()
        );
    }

    #[test]
    fn explicit_requests_bind_exact_relation_paths_and_reject_unreachable_effects() {
        let model = recursive_model();
        let relation_x_m1 = relation_id(&model, "construct:x", "construct:m1");
        let relation_m1_y = relation_id(&model, "construct:m1", "construct:y");
        let mut config = GeneralSemConfigV1::default();
        config.requested_effect_estimands = vec![
            GeneralSemEffectEstimandV1::SpecificPath {
                estimand_id: "effect:01:specific".into(),
                ordered_relation_ids: vec![relation_x_m1.clone(), relation_m1_y.clone()],
            },
            GeneralSemEffectEstimandV1::TotalIndirect {
                estimand_id: "effect:02:indirect".into(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
            },
            GeneralSemEffectEstimandV1::TotalEffect {
                estimand_id: "effect:03:total".into(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
            },
        ];
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        assert!(!plan.auto_selected_effects());
        assert_eq!(plan.effect_estimands().len(), 3);

        {
            let GeneralSemEffectEstimandV1::SpecificPath {
                ordered_relation_ids,
                ..
            } = &mut config.requested_effect_estimands[0]
            else {
                unreachable!()
            };
            ordered_relation_ids.reverse();
        }
        assert!(matches!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::UnknownSpecificIndirectPath { .. })
        ));

        let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut config.requested_effect_estimands[0]
        else {
            unreachable!()
        };
        *ordered_relation_ids = vec![relation_x_m1, relation_m1_y];
        let GeneralSemEffectEstimandV1::TotalEffect { target_id, .. } =
            &mut config.requested_effect_estimands[2]
        else {
            unreachable!()
        };
        *target_id = "construct:missing".into();
        assert!(matches!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::UnreachableEffect {
                kind: "total_effect",
                ..
            })
        ));
    }

    #[test]
    fn declaration_order_does_not_change_v3_plan_or_effect_identities() {
        let model = recursive_model();
        let expected = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        let actual = compile_pls_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn aggregate_estimand_cannot_reuse_a_canonical_specific_path_identity() {
        let model = recursive_model();
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let path = topology
            .specific_directed_paths()
            .iter()
            .find(|path| path.source() == "construct:x" && path.target() == "construct:y")
            .unwrap();
        let collision = path.identity().to_string();
        let mut config = GeneralSemConfigV1::default();
        config.requested_effect_estimands = vec![GeneralSemEffectEstimandV1::TotalEffect {
            estimand_id: collision.clone(),
            source_id: "construct:x".into(),
            target_id: "construct:y".into(),
        }];

        assert_eq!(
            compile_pls_plan_v3(&model, &config),
            Err(
                CompiledPlsPlanV3Error::AggregateEstimandIdCollidesWithSpecificPathIdentity {
                    estimand_id: collision,
                }
            )
        );
    }

    #[test]
    fn direct_pls_v3_compilation_rejects_lazy_path_materialization() {
        let model = recursive_model();
        let mut config = GeneralSemConfigV1::default();
        config.output_policy.lazy_specific_path_materialization = true;
        config.output_policy.when_specific_path_limit_exceeded =
            GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy;

        assert_eq!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::LazySpecificPathMaterializationNotImplemented)
        );
    }
}
