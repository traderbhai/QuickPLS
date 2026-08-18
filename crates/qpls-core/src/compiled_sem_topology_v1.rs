use crate::{
    ObservedRoleV4, SemModelV4, SemModelV4ValidationError, SemRelationV4, SemVariableV4,
    StructuralRelationRoleV4,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPILED_SEM_TOPOLOGY_V1_SCHEMA_VERSION: u32 = 1;

/// A structural relation copied from the validated scientific model.
///
/// Control-role relations remain in this collection because they are part of
/// the structural topology even though the default mediation-path projection
/// excludes them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledSemStructuralRelationV1 {
    relation_id: String,
    source: String,
    target: String,
    parameter_id: String,
    role: StructuralRelationRoleV4,
}

impl CompiledSemStructuralRelationV1 {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledSemStronglyConnectedComponentV1 {
    node_ids: Vec<String>,
    relation_ids: Vec<String>,
}

impl CompiledSemStronglyConnectedComponentV1 {
    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn relation_ids(&self) -> &[String] {
        &self.relation_ids
    }

    pub fn has_feedback(&self) -> bool {
        self.node_ids.len() > 1 || !self.relation_ids.is_empty()
    }
}

/// A simple directed path eligible for mediation-effect decomposition.
///
/// Paths contain at least two non-control structural relations. Their identity
/// is derived only from the ordered relation ids, so declaration order and
/// presentation changes cannot alter it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledSemSpecificDirectedPathV1 {
    identity: String,
    source: String,
    target: String,
    node_ids: Vec<String>,
    relation_ids: Vec<String>,
}

impl CompiledSemSpecificDirectedPathV1 {
    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn relation_ids(&self) -> &[String] {
        &self.relation_ids
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledSemTopologyV1 {
    schema_version: u32,
    model_id: String,
    model_scientific_sha256: String,
    structural_nodes: Vec<String>,
    structural_relations: Vec<CompiledSemStructuralRelationV1>,
    strongly_connected_components: Vec<CompiledSemStronglyConnectedComponentV1>,
    dag_topological_order: Option<Vec<String>>,
    specific_directed_paths: Vec<CompiledSemSpecificDirectedPathV1>,
}

impl CompiledSemTopologyV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_scientific_sha256(&self) -> &str {
        &self.model_scientific_sha256
    }

    pub fn structural_nodes(&self) -> &[String] {
        &self.structural_nodes
    }

    pub fn structural_relations(&self) -> &[CompiledSemStructuralRelationV1] {
        &self.structural_relations
    }

    pub fn strongly_connected_components(&self) -> &[CompiledSemStronglyConnectedComponentV1] {
        &self.strongly_connected_components
    }

    pub fn has_feedback(&self) -> bool {
        self.strongly_connected_components
            .iter()
            .any(CompiledSemStronglyConnectedComponentV1::has_feedback)
    }

    /// Returns a stable lexical Kahn ordering when the full structural graph is
    /// acyclic. Feedback models return `None`; their SCCs remain available.
    pub fn dag_topological_order(&self) -> Option<&[String]> {
        self.dag_topological_order.as_deref()
    }

    /// Returns all simple, non-control directed paths containing at least two
    /// relations. These are the default candidates for specific indirect
    /// effects; direct one-edge paths remain available in `structural_relations`.
    pub fn specific_directed_paths(&self) -> &[CompiledSemSpecificDirectedPathV1] {
        &self.specific_directed_paths
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledSemTopologyV1Error {
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
    #[error("compiled SEM topology requires max_paths to be greater than zero")]
    InvalidMaxPaths,
    #[error(
        "specific directed-path enumeration exceeded the explicit max_paths limit of {max_paths}"
    )]
    SpecificPathResourceLimitExceeded { max_paths: usize },
}

/// Compiles deterministic structural topology from a validated `SemModelV4`.
///
/// `max_paths` is mandatory because the number of simple paths can grow
/// exponentially. The compiler returns a typed resource-limit error instead of
/// returning a truncated collection.
pub fn compile_sem_topology_v1(
    model: &SemModelV4,
    max_paths: usize,
) -> Result<CompiledSemTopologyV1, CompiledSemTopologyV1Error> {
    // Validation deliberately precedes every compiler-specific precondition so
    // malformed model references can never be hidden by a path-limit error.
    model.ensure_valid()?;
    if max_paths == 0 {
        return Err(CompiledSemTopologyV1Error::InvalidMaxPaths);
    }

    let structural_nodes = model
        .variables
        .iter()
        .filter(|variable| is_structural_node(variable))
        .map(|variable| variable.id().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut structural_relations = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source,
                target,
                parameter,
                role,
                ..
            } => Some(CompiledSemStructuralRelationV1 {
                relation_id: id.clone(),
                source: source.clone(),
                target: target.clone(),
                parameter_id: parameter.clone(),
                role: *role,
            }),
            SemRelationV4::MeasurementEffect { .. }
            | SemRelationV4::MeasurementCausal { .. }
            | SemRelationV4::Covariance { .. } => None,
        })
        .collect::<Vec<_>>();
    structural_relations.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));

    let strongly_connected_components =
        strongly_connected_components(&structural_nodes, &structural_relations);
    let dag_topological_order = dag_topological_order(&structural_nodes, &structural_relations);
    let specific_directed_paths =
        enumerate_specific_directed_paths(&structural_nodes, &structural_relations, max_paths)?;

    Ok(CompiledSemTopologyV1 {
        schema_version: COMPILED_SEM_TOPOLOGY_V1_SCHEMA_VERSION,
        model_id: model.id.clone(),
        model_scientific_sha256: model.scientific_sha256()?,
        structural_nodes,
        structural_relations,
        strongly_connected_components,
        dag_topological_order,
        specific_directed_paths,
    })
}

fn is_structural_node(variable: &&SemVariableV4) -> bool {
    !matches!(
        variable,
        SemVariableV4::Observed {
            role: ObservedRoleV4::Indicator,
            ..
        }
    )
}

fn strongly_connected_components(
    nodes: &[String],
    relations: &[CompiledSemStructuralRelationV1],
) -> Vec<CompiledSemStronglyConnectedComponentV1> {
    let mut outgoing = nodes
        .iter()
        .map(|node| (node.clone(), Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = outgoing.clone();
    for relation in relations {
        outgoing
            .get_mut(&relation.source)
            .expect("validated structural source")
            .push(relation.target.clone());
        incoming
            .get_mut(&relation.target)
            .expect("validated structural target")
            .push(relation.source.clone());
    }
    for adjacent in outgoing.values_mut().chain(incoming.values_mut()) {
        adjacent.sort();
        adjacent.dedup();
    }

    let mut visited = BTreeSet::new();
    let mut finish_order = Vec::with_capacity(nodes.len());
    for node in nodes {
        visit_for_finish(node, &outgoing, &mut visited, &mut finish_order);
    }

    visited.clear();
    let mut components = Vec::new();
    for node in finish_order.into_iter().rev() {
        if visited.contains(&node) {
            continue;
        }
        let mut component_nodes = Vec::new();
        collect_component(&node, &incoming, &mut visited, &mut component_nodes);
        component_nodes.sort();
        let component_set = component_nodes.iter().collect::<BTreeSet<_>>();
        let relation_ids = relations
            .iter()
            .filter(|relation| {
                component_set.contains(&relation.source) && component_set.contains(&relation.target)
            })
            .map(|relation| relation.relation_id.clone())
            .collect::<Vec<_>>();
        components.push(CompiledSemStronglyConnectedComponentV1 {
            node_ids: component_nodes,
            relation_ids,
        });
    }
    components.sort_by(|left, right| left.node_ids.cmp(&right.node_ids));
    components
}

fn visit_for_finish(
    node: &str,
    outgoing: &BTreeMap<String, Vec<String>>,
    visited: &mut BTreeSet<String>,
    finish_order: &mut Vec<String>,
) {
    if !visited.insert(node.to_string()) {
        return;
    }
    for target in outgoing.get(node).into_iter().flatten() {
        visit_for_finish(target, outgoing, visited, finish_order);
    }
    finish_order.push(node.to_string());
}

fn collect_component(
    node: &str,
    incoming: &BTreeMap<String, Vec<String>>,
    visited: &mut BTreeSet<String>,
    component: &mut Vec<String>,
) {
    if !visited.insert(node.to_string()) {
        return;
    }
    component.push(node.to_string());
    for source in incoming.get(node).into_iter().flatten() {
        collect_component(source, incoming, visited, component);
    }
}

fn dag_topological_order(
    nodes: &[String],
    relations: &[CompiledSemStructuralRelationV1],
) -> Option<Vec<String>> {
    let mut indegree = nodes
        .iter()
        .map(|node| (node.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = nodes
        .iter()
        .map(|node| (node.clone(), Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    for relation in relations {
        *indegree
            .get_mut(&relation.target)
            .expect("validated structural target") += 1;
        outgoing
            .get_mut(&relation.source)
            .expect("validated structural source")
            .push(relation.target.clone());
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
                .expect("validated structural target");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(target.clone());
            }
        }
    }

    (ordered.len() == nodes.len()).then_some(ordered)
}

fn enumerate_specific_directed_paths(
    nodes: &[String],
    relations: &[CompiledSemStructuralRelationV1],
    max_paths: usize,
) -> Result<Vec<CompiledSemSpecificDirectedPathV1>, CompiledSemTopologyV1Error> {
    let mut outgoing = nodes
        .iter()
        .map(|node| (node.clone(), Vec::<usize>::new()))
        .collect::<BTreeMap<_, _>>();
    for (index, relation) in relations.iter().enumerate() {
        if relation.role == StructuralRelationRoleV4::Structural {
            outgoing
                .get_mut(&relation.source)
                .expect("validated structural source")
                .push(index);
        }
    }
    for relation_indices in outgoing.values_mut() {
        relation_indices.sort_by(|left, right| {
            relations[*left]
                .relation_id
                .cmp(&relations[*right].relation_id)
        });
    }

    let mut paths = Vec::new();
    for source in nodes {
        let mut visited = BTreeSet::from([source.clone()]);
        let mut node_ids = vec![source.clone()];
        let mut relation_ids = Vec::new();
        enumerate_paths_from(
            source,
            source,
            relations,
            &outgoing,
            &mut visited,
            &mut node_ids,
            &mut relation_ids,
            max_paths,
            &mut paths,
        )?;
    }
    paths.sort_by(|left, right| {
        left.relation_ids
            .cmp(&right.relation_ids)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
    });
    Ok(paths)
}

#[allow(clippy::too_many_arguments)]
fn enumerate_paths_from(
    source: &str,
    current: &str,
    relations: &[CompiledSemStructuralRelationV1],
    outgoing: &BTreeMap<String, Vec<usize>>,
    visited: &mut BTreeSet<String>,
    node_ids: &mut Vec<String>,
    relation_ids: &mut Vec<String>,
    max_paths: usize,
    paths: &mut Vec<CompiledSemSpecificDirectedPathV1>,
) -> Result<(), CompiledSemTopologyV1Error> {
    for relation_index in outgoing.get(current).into_iter().flatten() {
        let relation = &relations[*relation_index];
        if visited.contains(&relation.target) {
            continue;
        }

        visited.insert(relation.target.clone());
        node_ids.push(relation.target.clone());
        relation_ids.push(relation.relation_id.clone());

        if relation_ids.len() >= 2 {
            if paths.len() >= max_paths {
                return Err(
                    CompiledSemTopologyV1Error::SpecificPathResourceLimitExceeded { max_paths },
                );
            }
            paths.push(CompiledSemSpecificDirectedPathV1 {
                identity: specific_directed_path_identity_v1(relation_ids),
                source: source.to_string(),
                target: relation.target.clone(),
                node_ids: node_ids.clone(),
                relation_ids: relation_ids.clone(),
            });
        }

        enumerate_paths_from(
            source,
            &relation.target,
            relations,
            outgoing,
            visited,
            node_ids,
            relation_ids,
            max_paths,
            paths,
        )?;

        relation_ids.pop();
        node_ids.pop();
        visited.remove(&relation.target);
    }
    Ok(())
}

/// Derives the canonical v1 identity for an ordered structural-relation path.
///
/// The length-delimited relation-id sequence is the complete identity input;
/// model declaration order and presentation metadata cannot affect the result.
pub fn specific_directed_path_identity_v1(relation_ids: &[String]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-sem-topology-v1.specific-directed-path\0");
    for relation_id in relation_ids {
        digest.update((relation_id.len() as u64).to_be_bytes());
        digest.update(relation_id.as_bytes());
    }
    let digest = digest.finalize();
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sem_specific_path_v1_{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, StructuralPath,
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
                name: "Topology fixture".into(),
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

    #[test]
    fn serial_and_parallel_mediation_paths_have_stable_relation_identities() {
        let model = model_with_paths(&[
            ("x", "m1"),
            ("m1", "y"),
            ("x", "m2"),
            ("m2", "y"),
            ("m1", "m2"),
        ]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();

        let mut actual = topology
            .specific_directed_paths()
            .iter()
            .filter(|path| path.source() == node_id("x") && path.target() == node_id("y"))
            .map(|path| path.relation_ids().to_vec())
            .collect::<Vec<_>>();
        actual.sort();
        let mut expected = vec![
            vec![
                relation_id(&model, "x", "m1"),
                relation_id(&model, "m1", "y"),
            ],
            vec![
                relation_id(&model, "x", "m2"),
                relation_id(&model, "m2", "y"),
            ],
            vec![
                relation_id(&model, "x", "m1"),
                relation_id(&model, "m1", "m2"),
                relation_id(&model, "m2", "y"),
            ],
        ];
        expected.sort();
        assert_eq!(actual, expected);
        assert!(topology.specific_directed_paths().iter().all(|path| {
            path.identity() == specific_directed_path_identity_v1(path.relation_ids())
        }));
    }

    #[test]
    fn model_and_relation_declaration_reordering_is_invariant() {
        let model = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        let expected = compile_sem_topology_v1(&model, 100).unwrap();
        let mut reordered = model.clone();
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();

        assert_eq!(compile_sem_topology_v1(&reordered, 100).unwrap(), expected);
        assert!(
            expected
                .structural_nodes()
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            expected
                .structural_relations()
                .windows(2)
                .all(|pair| pair[0].relation_id() < pair[1].relation_id())
        );
    }

    #[test]
    fn control_relations_are_preserved_but_excluded_from_mediation_paths() {
        let mut model = model_with_paths(&[("x", "m"), ("m", "y"), ("c", "m")]);
        let control_relation_id = relation_id(&model, "c", "m");
        let relation = model
            .relations
            .iter_mut()
            .find(|relation| relation.id() == control_relation_id)
            .unwrap();
        let SemRelationV4::Structural { role, .. } = relation else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Control;

        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        assert!(topology.structural_relations().iter().any(|relation| {
            relation.relation_id() == control_relation_id
                && relation.role() == StructuralRelationRoleV4::Control
        }));
        assert!(
            topology
                .specific_directed_paths()
                .iter()
                .all(|path| { !path.relation_ids().contains(&control_relation_id) })
        );
        assert!(topology.specific_directed_paths().iter().any(|path| {
            path.source() == node_id("x")
                && path.target() == node_id("y")
                && path.relation_ids().len() == 2
        }));
    }

    #[test]
    fn reciprocal_graph_reports_one_feedback_scc_and_no_dag_order() {
        let model = model_with_paths(&[("x", "y"), ("y", "x")]);
        let topology = compile_sem_topology_v1(&model, 100).unwrap();

        assert!(topology.has_feedback());
        assert_eq!(topology.dag_topological_order(), None);
        assert_eq!(topology.strongly_connected_components().len(), 1);
        assert_eq!(
            topology.strongly_connected_components()[0].node_ids(),
            &[node_id("x"), node_id("y")]
        );
        assert_eq!(
            topology.strongly_connected_components()[0]
                .relation_ids()
                .len(),
            2
        );

        // Ready SemModelV4 validation rejects structural self-loops, but the
        // SCC value remains correct and defensive when decoded independently.
        assert!(
            CompiledSemStronglyConnectedComponentV1 {
                node_ids: vec![node_id("x")],
                relation_ids: vec!["self-loop".into()],
            }
            .has_feedback()
        );
    }

    #[test]
    fn unknown_structural_endpoint_fails_existing_model_validation_first() {
        let mut model = model_with_paths(&[("x", "y")]);
        let SemRelationV4::Structural { target, .. } = model
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, SemRelationV4::Structural { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        *target = "construct:missing".into();

        let error = compile_sem_topology_v1(&model, 10).unwrap_err();
        let CompiledSemTopologyV1Error::InvalidModel(error) = error else {
            panic!("expected model validation error")
        };
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.code == "structural.variable.unknown")
        );
    }

    #[test]
    fn zero_and_overflowing_specific_path_limits_fail_typed() {
        let model = model_with_paths(&[("x", "m1"), ("m1", "y"), ("x", "m2"), ("m2", "y")]);
        assert_eq!(
            compile_sem_topology_v1(&model, 0),
            Err(CompiledSemTopologyV1Error::InvalidMaxPaths)
        );
        assert_eq!(
            compile_sem_topology_v1(&model, 1),
            Err(CompiledSemTopologyV1Error::SpecificPathResourceLimitExceeded { max_paths: 1 })
        );
    }

    #[test]
    fn exact_serde_roundtrip_and_unknown_field_rejection_are_strict() {
        let model = model_with_paths(&[("x", "m"), ("m", "y")]);
        let topology = compile_sem_topology_v1(&model, 10).unwrap();
        let wire = serde_json::to_vec(&topology).unwrap();
        let decoded: CompiledSemTopologyV1 = serde_json::from_slice(&wire).unwrap();
        assert_eq!(decoded, topology);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), wire);

        let mut tampered = serde_json::to_value(&topology).unwrap();
        tampered
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::Value::Bool(true));
        let error = serde_json::from_value::<CompiledSemTopologyV1>(tampered).unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }
}
