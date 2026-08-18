use crate::{
    COMPILED_SEM_TOPOLOGY_V1_SCHEMA_VERSION, CompiledSemSpecificDirectedPathV1,
    CompiledSemStructuralRelationV1, CompiledSemTopologyV1, StructuralRelationRoleV4,
    specific_directed_path_identity_v1,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const GENERAL_SEM_EFFECTS_V1_SCHEMA_VERSION: u32 = 1;
pub const GENERAL_SEM_EFFECTS_V1_METHOD_VERSION: &str = "general_sem_effects_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemSpecificIndirectEffectV1 {
    specific_path_identity: String,
    ordered_relation_ids: Vec<String>,
    source_id: String,
    target_id: String,
    coefficient: f64,
}

impl GeneralSemSpecificIndirectEffectV1 {
    pub fn specific_path_identity(&self) -> &str {
        &self.specific_path_identity
    }

    pub fn ordered_relation_ids(&self) -> &[String] {
        &self.ordered_relation_ids
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn coefficient(&self) -> f64 {
        self.coefficient
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPairEffectsV1 {
    source_id: String,
    target_id: String,
    direct_effect: f64,
    total_indirect_effect: f64,
    total_effect: f64,
}

impl GeneralSemPairEffectsV1 {
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn direct_effect(&self) -> f64 {
        self.direct_effect
    }

    pub fn total_indirect_effect(&self) -> f64 {
        self.total_indirect_effect
    }

    pub fn total_effect(&self) -> f64 {
        self.total_effect
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemEffectsV1 {
    schema_version: u32,
    method_version: String,
    topology_schema_version: u32,
    model_id: String,
    model_scientific_sha256: String,
    specific_indirect_effects: Vec<GeneralSemSpecificIndirectEffectV1>,
    pair_effects: Vec<GeneralSemPairEffectsV1>,
}

impl GeneralSemEffectsV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn topology_schema_version(&self) -> u32 {
        self.topology_schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn specific_indirect_effects(&self) -> &[GeneralSemSpecificIndirectEffectV1] {
        &self.specific_indirect_effects
    }

    pub fn pair_effects(&self) -> &[GeneralSemPairEffectsV1] {
        &self.pair_effects
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemEffectsV1Error {
    #[error("general SEM effects v1 requires compiled topology schema {expected}, found {found}")]
    UnsupportedTopologyVersion { expected: u32, found: u32 },
    #[error("general SEM effect decomposition requires an acyclic structural topology")]
    StructuralFeedback,
    #[error("compiled SEM topology is incomplete ({code}): {detail}")]
    IncompleteTopology { code: String, detail: String },
    #[error(
        "relation coefficient ids do not exactly match the compiled topology; missing={missing_relation_ids:?}, unknown={unknown_relation_ids:?}"
    )]
    CoefficientDomainMismatch {
        missing_relation_ids: Vec<String>,
        unknown_relation_ids: Vec<String>,
    },
    #[error("relation coefficient must be finite: {relation_id}")]
    NonFiniteCoefficient { relation_id: String },
    #[error("effect calculation produced a non-finite value: {effect_identity}")]
    NonFiniteEffect { effect_identity: String },
}

#[derive(Default)]
struct PairAccumulator {
    direct: Vec<f64>,
    indirect: Vec<f64>,
}

/// Decomposes all non-control effects represented by an exact compiled DAG.
///
/// The coefficient map must contain every structural relation id, including
/// control-role relations, and no other ids. Controls are validated but never
/// enter direct, indirect, or total causal effects.
pub fn decompose_general_sem_effects_v1(
    topology: &CompiledSemTopologyV1,
    relation_coefficients: &BTreeMap<String, f64>,
) -> Result<GeneralSemEffectsV1, GeneralSemEffectsV1Error> {
    validate_topology(topology)?;
    validate_coefficient_domain(topology, relation_coefficients)?;

    let mut pairs = BTreeMap::<(String, String), PairAccumulator>::new();
    for relation in topology.structural_relations() {
        if relation.role() == StructuralRelationRoleV4::Control {
            continue;
        }
        pairs
            .entry((relation.source().to_string(), relation.target().to_string()))
            .or_default()
            .direct
            .push(relation_coefficients[relation.relation_id()]);
    }

    let mut specific_indirect_effects =
        Vec::with_capacity(topology.specific_directed_paths().len());
    for path in topology.specific_directed_paths() {
        let coefficient = path_product(path, relation_coefficients)?;
        pairs
            .entry((path.source().to_string(), path.target().to_string()))
            .or_default()
            .indirect
            .push(coefficient);
        specific_indirect_effects.push(GeneralSemSpecificIndirectEffectV1 {
            specific_path_identity: path.identity().to_string(),
            ordered_relation_ids: path.relation_ids().to_vec(),
            source_id: path.source().to_string(),
            target_id: path.target().to_string(),
            coefficient,
        });
    }

    let mut pair_effects = Vec::with_capacity(pairs.len());
    for ((source_id, target_id), values) in pairs {
        let pair_identity = format!("{source_id}->{target_id}");
        let direct_effect = stable_sum(&values.direct, format!("pair:{pair_identity}:direct"))?;
        let total_indirect_effect = stable_sum(
            &values.indirect,
            format!("pair:{pair_identity}:total_indirect"),
        )?;
        let total_effect = stable_sum(
            &[direct_effect, total_indirect_effect],
            format!("pair:{pair_identity}:total"),
        )?;
        pair_effects.push(GeneralSemPairEffectsV1 {
            source_id,
            target_id,
            direct_effect,
            total_indirect_effect,
            total_effect,
        });
    }

    Ok(GeneralSemEffectsV1 {
        schema_version: GENERAL_SEM_EFFECTS_V1_SCHEMA_VERSION,
        method_version: GENERAL_SEM_EFFECTS_V1_METHOD_VERSION.into(),
        topology_schema_version: topology.schema_version(),
        model_id: topology.model_id().to_string(),
        model_scientific_sha256: topology.model_scientific_sha256().to_string(),
        specific_indirect_effects,
        pair_effects,
    })
}

fn validate_topology(topology: &CompiledSemTopologyV1) -> Result<(), GeneralSemEffectsV1Error> {
    if topology.schema_version() != COMPILED_SEM_TOPOLOGY_V1_SCHEMA_VERSION {
        return Err(GeneralSemEffectsV1Error::UnsupportedTopologyVersion {
            expected: COMPILED_SEM_TOPOLOGY_V1_SCHEMA_VERSION,
            found: topology.schema_version(),
        });
    }
    if topology.model_id().trim().is_empty() {
        return Err(incomplete("model_id_empty", "model_id must be non-empty"));
    }
    if topology.model_scientific_sha256().len() != 64
        || !topology
            .model_scientific_sha256()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(incomplete(
            "scientific_hash_invalid",
            "model_scientific_sha256 must be 64 lowercase hexadecimal characters",
        ));
    }

    let nodes = topology.structural_nodes();
    if nodes.iter().any(|node| node.trim().is_empty())
        || nodes.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(incomplete(
            "structural_nodes_noncanonical",
            "structural nodes must be non-empty, unique, and strictly sorted",
        ));
    }
    let node_set = nodes.iter().map(String::as_str).collect::<BTreeSet<_>>();

    let relations = topology.structural_relations();
    if relations
        .windows(2)
        .any(|pair| pair[0].relation_id() >= pair[1].relation_id())
    {
        return Err(incomplete(
            "structural_relations_noncanonical",
            "structural relations must have unique, strictly sorted ids",
        ));
    }
    for relation in relations {
        if relation.relation_id().trim().is_empty()
            || relation.parameter_id().trim().is_empty()
            || !node_set.contains(relation.source())
            || !node_set.contains(relation.target())
        {
            return Err(incomplete(
                "structural_relation_invalid",
                format!(
                    "relation {} must have non-empty identities and known endpoints",
                    relation.relation_id()
                ),
            ));
        }
    }

    let expected_order = canonical_dag_order(nodes, relations)?;
    let Some(expected_order) = expected_order else {
        return Err(GeneralSemEffectsV1Error::StructuralFeedback);
    };
    if topology.has_feedback() {
        return Err(GeneralSemEffectsV1Error::StructuralFeedback);
    }
    if topology.dag_topological_order() != Some(expected_order.as_slice()) {
        return Err(incomplete(
            "dag_order_incomplete",
            "stored DAG order does not equal the deterministic order implied by the relations",
        ));
    }

    let components = topology.strongly_connected_components();
    if components.len() != nodes.len()
        || components.iter().zip(nodes).any(|(component, node)| {
            component.node_ids() != std::slice::from_ref(node)
                || !component.relation_ids().is_empty()
        })
    {
        return Err(incomplete(
            "scc_partition_incomplete",
            "an acyclic topology must contain one relation-free SCC per structural node",
        ));
    }

    validate_specific_paths(topology, &node_set)?;
    let expected_path_count = count_specific_paths(&expected_order, relations)?;
    if topology.specific_directed_paths().len() != expected_path_count {
        return Err(incomplete(
            "specific_path_count_mismatch",
            format!(
                "stored {} specific paths but the DAG implies {expected_path_count}",
                topology.specific_directed_paths().len()
            ),
        ));
    }
    Ok(())
}

fn validate_specific_paths(
    topology: &CompiledSemTopologyV1,
    node_set: &BTreeSet<&str>,
) -> Result<(), GeneralSemEffectsV1Error> {
    let relations = topology
        .structural_relations()
        .iter()
        .map(|relation| (relation.relation_id(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut identities = BTreeSet::new();
    let mut relation_sequences = BTreeSet::new();
    let paths = topology.specific_directed_paths();
    if paths
        .windows(2)
        .any(|pair| compare_paths(&pair[0], &pair[1]) != Ordering::Less)
    {
        return Err(incomplete(
            "specific_paths_noncanonical",
            "specific paths must be unique and in deterministic relation-sequence order",
        ));
    }

    for path in paths {
        if path.identity().trim().is_empty()
            || !identities.insert(path.identity())
            || path.relation_ids().len() < 2
            || path.node_ids().len() != path.relation_ids().len() + 1
            || path.node_ids().first().map(String::as_str) != Some(path.source())
            || path.node_ids().last().map(String::as_str) != Some(path.target())
            || path
                .node_ids()
                .iter()
                .any(|node| !node_set.contains(node.as_str()))
            || path.node_ids().iter().collect::<BTreeSet<_>>().len() != path.node_ids().len()
            || !relation_sequences.insert(path.relation_ids().to_vec())
        {
            return Err(incomplete(
                "specific_path_invalid",
                format!("invalid specific path identity {}", path.identity()),
            ));
        }

        let expected_identity = specific_directed_path_identity_v1(path.relation_ids());
        if path.identity() != expected_identity.as_str() {
            return Err(incomplete(
                "specific_path_identity_mismatch",
                format!(
                    "path identity {} does not match canonical identity {expected_identity}",
                    path.identity()
                ),
            ));
        }

        for (index, relation_id) in path.relation_ids().iter().enumerate() {
            let Some(relation) = relations.get(relation_id.as_str()) else {
                return Err(incomplete(
                    "specific_path_relation_unknown",
                    format!("path {} references {relation_id}", path.identity()),
                ));
            };
            if relation.role() != StructuralRelationRoleV4::Structural
                || relation.source() != path.node_ids()[index]
                || relation.target() != path.node_ids()[index + 1]
            {
                return Err(incomplete(
                    "specific_path_discontinuous",
                    format!("path {} is not a continuous causal path", path.identity()),
                ));
            }
        }
    }
    Ok(())
}

fn compare_paths(
    left: &CompiledSemSpecificDirectedPathV1,
    right: &CompiledSemSpecificDirectedPathV1,
) -> Ordering {
    left.relation_ids()
        .cmp(right.relation_ids())
        .then_with(|| left.source().cmp(right.source()))
        .then_with(|| left.target().cmp(right.target()))
}

fn canonical_dag_order(
    nodes: &[String],
    relations: &[CompiledSemStructuralRelationV1],
) -> Result<Option<Vec<String>>, GeneralSemEffectsV1Error> {
    let mut indegree = nodes
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = nodes
        .iter()
        .map(|node| (node.clone(), Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    for relation in relations {
        let degree = indegree
            .get_mut(relation.target())
            .expect("topology endpoints validated");
        *degree = degree.checked_add(1).ok_or_else(|| {
            incomplete(
                "indegree_overflow",
                format!("indegree overflow at {}", relation.target()),
            )
        })?;
        outgoing
            .get_mut(relation.source())
            .expect("topology endpoints validated")
            .push(relation.target().to_string());
    }
    for targets in outgoing.values_mut() {
        targets.sort();
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(nodes.len());
    while let Some(node) = ready.iter().next().cloned() {
        ready.remove(&node);
        ordered.push(node.clone());
        for target in outgoing.get(&node).into_iter().flatten() {
            let degree = indegree
                .get_mut(target)
                .expect("topology endpoints validated");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(target.clone());
            }
        }
    }
    Ok((ordered.len() == nodes.len()).then_some(ordered))
}

fn count_specific_paths(
    dag_order: &[String],
    relations: &[CompiledSemStructuralRelationV1],
) -> Result<usize, GeneralSemEffectsV1Error> {
    let mut outgoing = dag_order
        .iter()
        .map(|node| (node.clone(), Vec::<&CompiledSemStructuralRelationV1>::new()))
        .collect::<BTreeMap<_, _>>();
    for relation in relations {
        if relation.role() == StructuralRelationRoleV4::Structural {
            outgoing
                .get_mut(relation.source())
                .expect("topology endpoints validated")
                .push(relation);
        }
    }
    let mut nonempty_paths_ending_at = dag_order
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut specific_path_count = 0usize;
    for source in dag_order {
        let prefixes = nonempty_paths_ending_at[source];
        for relation in outgoing.get(source).into_iter().flatten() {
            specific_path_count = specific_path_count.checked_add(prefixes).ok_or_else(|| {
                incomplete(
                    "specific_path_count_overflow",
                    "DAG-specific path count exceeds usize",
                )
            })?;
            let extensions = prefixes.checked_add(1).ok_or_else(|| {
                incomplete(
                    "specific_path_count_overflow",
                    "DAG-specific path prefix count exceeds usize",
                )
            })?;
            let target_count = nonempty_paths_ending_at
                .get_mut(relation.target())
                .expect("topology endpoints validated");
            *target_count = target_count.checked_add(extensions).ok_or_else(|| {
                incomplete(
                    "specific_path_count_overflow",
                    "DAG-specific path target count exceeds usize",
                )
            })?;
        }
    }
    Ok(specific_path_count)
}

fn validate_coefficient_domain(
    topology: &CompiledSemTopologyV1,
    relation_coefficients: &BTreeMap<String, f64>,
) -> Result<(), GeneralSemEffectsV1Error> {
    let expected = topology
        .structural_relations()
        .iter()
        .map(|relation| relation.relation_id().to_string())
        .collect::<BTreeSet<_>>();
    let provided = relation_coefficients
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing_relation_ids = expected.difference(&provided).cloned().collect::<Vec<_>>();
    let unknown_relation_ids = provided.difference(&expected).cloned().collect::<Vec<_>>();
    if !missing_relation_ids.is_empty() || !unknown_relation_ids.is_empty() {
        return Err(GeneralSemEffectsV1Error::CoefficientDomainMismatch {
            missing_relation_ids,
            unknown_relation_ids,
        });
    }
    for relation_id in expected {
        if !relation_coefficients[&relation_id].is_finite() {
            return Err(GeneralSemEffectsV1Error::NonFiniteCoefficient { relation_id });
        }
    }
    Ok(())
}

fn path_product(
    path: &CompiledSemSpecificDirectedPathV1,
    relation_coefficients: &BTreeMap<String, f64>,
) -> Result<f64, GeneralSemEffectsV1Error> {
    if path
        .relation_ids()
        .iter()
        .any(|relation_id| relation_coefficients[relation_id] == 0.0)
    {
        return Ok(0.0);
    }
    let mut product = 1.0;
    for relation_id in path.relation_ids() {
        product *= relation_coefficients[relation_id];
        if !product.is_finite() {
            return Err(GeneralSemEffectsV1Error::NonFiniteEffect {
                effect_identity: path.identity().to_string(),
            });
        }
    }
    Ok(canonical_zero(product))
}

/// Neumaier compensated summation in an already-canonical input order.
fn stable_sum(values: &[f64], effect_identity: String) -> Result<f64, GeneralSemEffectsV1Error> {
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for value in values {
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            compensation += (sum - updated) + value;
        } else {
            compensation += (value - updated) + sum;
        }
        sum = updated;
        if !sum.is_finite() || !compensation.is_finite() {
            return Err(GeneralSemEffectsV1Error::NonFiniteEffect { effect_identity });
        }
    }
    let total = sum + compensation;
    if !total.is_finite() {
        return Err(GeneralSemEffectsV1Error::NonFiniteEffect { effect_identity });
    }
    Ok(canonical_zero(total))
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn incomplete(code: &str, detail: impl Into<String>) -> GeneralSemEffectsV1Error {
    GeneralSemEffectsV1Error::IncompleteTopology {
        code: code.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CompiledSemTopologyV1Error, Construct, LegacyBasicModelInterpretationV4, MeasurementMode,
        ModelSpec, SemModelV4, SemRelationV4, StructuralPath, compile_sem_topology_v1,
        convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn model_with_paths(paths: &[(&str, &str)]) -> SemModelV4 {
        let node_names = paths
            .iter()
            .flat_map(|(source, target)| [*source, *target])
            .collect::<BTreeSet<_>>();
        let constructs = node_names
            .into_iter()
            .map(|node| Construct {
                id: node.into(),
                name: node.to_uppercase(),
                short_name: node.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{node}_1"), format!("{node}_2")],
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
                id: Uuid::nil(),
                name: "General SEM effects fixture".into(),
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

    fn node_id(node: &str) -> String {
        format!("construct:{node}")
    }

    fn relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        let source = node_id(source);
        let target = node_id(target);
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source: candidate_source,
                    target: candidate_target,
                    ..
                } if candidate_source == &source && candidate_target == &target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn exact_coefficients(topology: &CompiledSemTopologyV1) -> BTreeMap<String, f64> {
        topology
            .structural_relations()
            .iter()
            .map(|relation| (relation.relation_id().to_string(), 1.0))
            .collect()
    }

    fn set_coefficient(
        coefficients: &mut BTreeMap<String, f64>,
        model: &SemModelV4,
        source: &str,
        target: &str,
        value: f64,
    ) {
        *coefficients
            .get_mut(&relation_id(model, source, target))
            .unwrap() = value;
    }

    fn pair<'a>(
        result: &'a GeneralSemEffectsV1,
        source: &str,
        target: &str,
    ) -> &'a GeneralSemPairEffectsV1 {
        let source = node_id(source);
        let target = node_id(target);
        result
            .pair_effects()
            .iter()
            .find(|effect| effect.source_id() == source && effect.target_id() == target)
            .unwrap()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= 1e-12, "{actual} != {expected}");
    }

    #[test]
    fn serial_and_parallel_products_reconcile_to_pair_aggregates() {
        let model = model_with_paths(&[
            ("x", "y"),
            ("x", "m1"),
            ("m1", "y"),
            ("x", "m2"),
            ("m2", "y"),
            ("m1", "m2"),
        ]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let mut coefficients = exact_coefficients(&topology);
        set_coefficient(&mut coefficients, &model, "x", "y", 0.2);
        set_coefficient(&mut coefficients, &model, "x", "m1", 0.5);
        set_coefficient(&mut coefficients, &model, "m1", "y", 0.4);
        set_coefficient(&mut coefficients, &model, "x", "m2", -0.3);
        set_coefficient(&mut coefficients, &model, "m2", "y", 0.2);
        set_coefficient(&mut coefficients, &model, "m1", "m2", 0.1);

        let result = decompose_general_sem_effects_v1(&topology, &coefficients).unwrap();
        let xy_specific = result
            .specific_indirect_effects()
            .iter()
            .filter(|effect| {
                effect.source_id() == node_id("x") && effect.target_id() == node_id("y")
            })
            .collect::<Vec<_>>();
        assert_eq!(xy_specific.len(), 3);
        let specific_sum = xy_specific
            .iter()
            .map(|effect| effect.coefficient())
            .sum::<f64>();
        let xy = pair(&result, "x", "y");
        assert_close(specific_sum, 0.15);
        assert_close(xy.direct_effect(), 0.2);
        assert_close(xy.total_indirect_effect(), specific_sum);
        assert_close(xy.total_effect(), 0.35);
        assert!(xy_specific.iter().all(|effect| {
            !effect.specific_path_identity().is_empty() && effect.ordered_relation_ids().len() >= 2
        }));
    }

    #[test]
    fn control_coefficients_are_required_but_excluded_from_all_effects() {
        let mut model = model_with_paths(&[("x", "m"), ("m", "y"), ("c", "m")]);
        let control_id = relation_id(&model, "c", "m");
        let SemRelationV4::Structural { role, .. } = model
            .relations
            .iter_mut()
            .find(|relation| relation.id() == control_id)
            .unwrap()
        else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Control;
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let mut coefficients = exact_coefficients(&topology);
        coefficients.insert(control_id.clone(), 99.0);
        set_coefficient(&mut coefficients, &model, "x", "m", 0.5);
        set_coefficient(&mut coefficients, &model, "m", "y", 0.4);

        let result = decompose_general_sem_effects_v1(&topology, &coefficients).unwrap();
        assert_close(pair(&result, "x", "y").total_indirect_effect(), 0.2);
        assert!(result.pair_effects().iter().all(|effect| {
            effect.source_id() != node_id("c") && effect.target_id() != node_id("c")
        }));
        assert!(
            result
                .specific_indirect_effects()
                .iter()
                .all(|effect| { !effect.ordered_relation_ids().contains(&control_id) })
        );
    }

    #[test]
    fn declaration_and_map_insertion_order_are_exactly_metamorphic() {
        let model = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let coefficients = topology
            .structural_relations()
            .iter()
            .enumerate()
            .map(|(index, relation)| (relation.relation_id().to_string(), index as f64 / 10.0))
            .collect::<BTreeMap<_, _>>();
        let expected = decompose_general_sem_effects_v1(&topology, &coefficients).unwrap();

        let mut reordered_model = model.clone();
        reordered_model.variables.reverse();
        reordered_model.relations.reverse();
        reordered_model.parameters.reverse();
        let reordered_topology = compile_sem_topology_v1(&reordered_model, 100).unwrap();
        let reordered_coefficients = coefficients
            .iter()
            .rev()
            .map(|(id, value)| (id.clone(), *value))
            .collect::<BTreeMap<_, _>>();
        let actual =
            decompose_general_sem_effects_v1(&reordered_topology, &reordered_coefficients).unwrap();
        assert_eq!(actual, expected);
        let wire = serde_json::to_vec(&actual).unwrap();
        assert_eq!(
            serde_json::to_vec(&serde_json::from_slice::<GeneralSemEffectsV1>(&wire).unwrap())
                .unwrap(),
            wire
        );
    }

    #[test]
    fn signed_and_zero_specific_paths_remain_explicit() {
        let model = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let mut coefficients = exact_coefficients(&topology);
        set_coefficient(&mut coefficients, &model, "x", "m1", -0.5);
        set_coefficient(&mut coefficients, &model, "m1", "y", 0.4);
        set_coefficient(&mut coefficients, &model, "x", "m2", 0.0);
        set_coefficient(&mut coefficients, &model, "m2", "y", -0.7);

        let result = decompose_general_sem_effects_v1(&topology, &coefficients).unwrap();
        let xy = result
            .specific_indirect_effects()
            .iter()
            .filter(|effect| {
                effect.source_id() == node_id("x") && effect.target_id() == node_id("y")
            })
            .collect::<Vec<_>>();
        assert_eq!(xy.len(), 2);
        assert!(xy.iter().any(|effect| effect.coefficient() == 0.0));
        assert!(xy.iter().any(|effect| effect.coefficient() < 0.0));
        assert_close(pair(&result, "x", "y").total_indirect_effect(), -0.2);
    }

    #[test]
    fn coefficient_domain_and_finiteness_fail_closed() {
        let model = model_with_paths(&[("x", "y")]);
        let topology = compile_sem_topology_v1(&model, 10).unwrap();
        let relation_id = topology.structural_relations()[0].relation_id().to_string();

        assert_eq!(
            decompose_general_sem_effects_v1(&topology, &BTreeMap::new()),
            Err(GeneralSemEffectsV1Error::CoefficientDomainMismatch {
                missing_relation_ids: vec![relation_id.clone()],
                unknown_relation_ids: Vec::new(),
            })
        );

        let mut extra = exact_coefficients(&topology);
        extra.insert("unknown-relation".into(), 1.0);
        assert_eq!(
            decompose_general_sem_effects_v1(&topology, &extra),
            Err(GeneralSemEffectsV1Error::CoefficientDomainMismatch {
                missing_relation_ids: Vec::new(),
                unknown_relation_ids: vec!["unknown-relation".into()],
            })
        );

        let mut nonfinite = exact_coefficients(&topology);
        nonfinite.insert(relation_id.clone(), f64::NAN);
        assert_eq!(
            decompose_general_sem_effects_v1(&topology, &nonfinite),
            Err(GeneralSemEffectsV1Error::NonFiniteCoefficient { relation_id })
        );
    }

    #[test]
    fn feedback_topology_is_rejected_before_effect_calculation() {
        let model = model_with_paths(&[("x", "y"), ("y", "x")]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let coefficients = exact_coefficients(&topology);
        assert_eq!(
            decompose_general_sem_effects_v1(&topology, &coefficients),
            Err(GeneralSemEffectsV1Error::StructuralFeedback)
        );
    }

    #[test]
    fn missing_compiled_path_is_rejected_as_incomplete_not_truncated() {
        let model = model_with_paths(&[("x", "m"), ("m", "y")]);
        let topology = compile_sem_topology_v1(&model, 10).unwrap();
        let coefficients = exact_coefficients(&topology);
        let mut wire = serde_json::to_value(&topology).unwrap();
        wire["specific_directed_paths"]
            .as_array_mut()
            .unwrap()
            .clear();
        let incomplete: CompiledSemTopologyV1 = serde_json::from_value(wire).unwrap();

        assert!(matches!(
            decompose_general_sem_effects_v1(&incomplete, &coefficients),
            Err(GeneralSemEffectsV1Error::IncompleteTopology { code, .. })
                if code == "specific_path_count_mismatch"
        ));
    }

    #[test]
    fn mismatched_specific_path_identity_is_rejected_as_incomplete() {
        let model = model_with_paths(&[("x", "m"), ("m", "y")]);
        let topology = compile_sem_topology_v1(&model, 10).unwrap();
        let coefficients = exact_coefficients(&topology);
        let mut wire = serde_json::to_value(&topology).unwrap();
        wire["specific_directed_paths"][0]["identity"] =
            serde_json::json!("sem_specific_path_v1_tampered");
        let tampered: CompiledSemTopologyV1 = serde_json::from_value(wire).unwrap();

        assert!(matches!(
            decompose_general_sem_effects_v1(&tampered, &coefficients),
            Err(GeneralSemEffectsV1Error::IncompleteTopology { code, .. })
                if code == "specific_path_identity_mismatch"
        ));
    }

    #[test]
    fn topology_resource_limit_remains_a_typed_upstream_failure() {
        let model = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        assert_eq!(
            compile_sem_topology_v1(&model, 1),
            Err(CompiledSemTopologyV1Error::SpecificPathResourceLimitExceeded { max_paths: 1 })
        );
    }
}
