import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";

export interface ModelIssue {
  code: "construct.empty_name" | "construct.no_indicators" | "indicator.duplicate" | "path.self" | "path.duplicate" | "path.cycle" | "path.unknown_construct" | "interaction.invalid" | "interaction.duplicate" | "higher_order.invalid" | "higher_order.components" | "higher_order.self_component" | "higher_order.unknown_component" | "higher_order.duplicate_component" | "higher_order.hybrid_component_indicators";
  subject: string;
}

export function validateModel(nodes: Array<Node<ConstructData>>, edges: Edge[]): ModelIssue[] {
  const issues: ModelIssue[] = [];
  const nodeIds = new Set(nodes.map((node) => node.id));
  const indicatorOwners = new Map<string, string>();
  const structuralPaths = new Set<string>();
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance");

  for (const node of nodes) {
    if (!node.data.label.trim()) issues.push({ code: "construct.empty_name", subject: node.id });
    if (node.data.semantic !== "interaction" && node.data.semantic !== "higher_order" && node.data.indicators.length === 0) issues.push({ code: "construct.no_indicators", subject: node.id });
    for (const indicator of node.data.indicators) {
      const owner = indicatorOwners.get(indicator);
      if (owner && owner !== node.id) issues.push({ code: "indicator.duplicate", subject: indicator });
      indicatorOwners.set(indicator, node.id);
    }
  }

  for (const node of nodes.filter((candidate) => candidate.data.semantic === "higher_order")) {
    const higherOrder = node.data.higherOrder;
    if (!higherOrder || higherOrder.id !== node.id) {
      issues.push({ code: "higher_order.invalid", subject: node.id });
      continue;
    }
    if (higherOrder.components.length < 2) {
      issues.push({ code: "higher_order.components", subject: node.id });
    }
    const componentIds = new Set<string>();
    for (const componentId of higherOrder.components) {
      if (componentId === node.id) issues.push({ code: "higher_order.self_component", subject: node.id });
      const component = nodes.find((candidate) => candidate.id === componentId);
      if (!component) issues.push({ code: "higher_order.unknown_component", subject: `${node.id}:${componentId}` });
      if (componentIds.has(componentId)) issues.push({ code: "higher_order.duplicate_component", subject: `${node.id}:${componentId}` });
      componentIds.add(componentId);
      if (higherOrder.method === "hybrid" && component && component.data.indicators.length < 2) {
        issues.push({ code: "higher_order.hybrid_component_indicators", subject: `${node.id}:${componentId}` });
      }
    }
  }

  const interactionNodes = nodes.filter((node) => node.data.semantic === "interaction");
  const interactionIdentities = new Set<string>();
  for (const node of interactionNodes) {
    const interaction = node.data.interaction;
    const operands = interaction
      ? interaction.kind === "interaction_v2"
        ? interaction.operands
        : [interaction.predictor, interaction.moderator]
      : [];
    if (interaction) {
      const identity = JSON.stringify([operands, interaction.outcome]);
      if (interactionIdentities.has(identity)) issues.push({ code: "interaction.duplicate", subject: node.id });
      interactionIdentities.add(identity);
    }
    const roles = interaction ? [...operands, node.id, interaction.outcome] : [];
    const hasPath = (source: string, target: string, allowControl: boolean) => edges.some((edge) =>
      !edge.id.startsWith("measurement::")
      && edge.source === source
      && edge.target === target
      && edge.data?.role !== "covariance"
      && (allowControl || edge.data?.role !== "control"),
    );
    const hasExactV2FocalPath = interaction?.kind !== "interaction_v2" || edges.some((edge) =>
      edge.id === interaction.focalRelationId
      && edge.source === operands[0]
      && edge.target === interaction.outcome
      && edge.data?.role !== "covariance"
      && edge.data?.role !== "control",
    );
    const hasRequiredMainEffects = !interaction
      ? false
      : interaction.kind === "interaction_v2"
        ? interaction.hierarchyPolicy === "none" || operands.every((operand) => hasPath(operand, interaction.outcome, false))
        : hasPath(interaction.moderator, interaction.outcome, true);
    const hasRequiredLowerOrderInteractions = interaction?.kind !== "interaction_v2"
      || interaction.hierarchyPolicy !== "strong"
      || operands.length <= 2
      || operands.every((_, omitted) => {
        const required = new Set(operands.filter((__, index) => index !== omitted));
        return interactionNodes.some((candidate) => {
          if (candidate.id === node.id || candidate.data.interaction?.outcome !== interaction.outcome) return false;
          const candidateInteraction = candidate.data.interaction;
          const candidateOperands = candidateInteraction.kind === "interaction_v2"
            ? candidateInteraction.operands
            : [candidateInteraction.predictor, candidateInteraction.moderator];
          return candidateOperands.length === required.size
            && candidateOperands.every((operand) => required.has(operand))
            && (candidateOperands.length <= 2
              || candidateInteraction.kind === "interaction_v2" && candidateInteraction.hierarchyPolicy === "strong")
            && hasPath(candidate.id, interaction.outcome, false);
        });
      });
    const hasCoherentMethod = interaction?.kind !== "interaction_v2"
      || (interaction.canonicalMethod === "product_indicator") === Boolean(interaction.productIndicator);
    if (!interaction
      || operands.length < 2
      || new Set(roles).size !== operands.length + 2
      || roles.some((id) => !nodeIds.has(id))
      || !hasPath(operands[0]!, interaction.outcome, false)
      || !hasExactV2FocalPath
      || !hasRequiredMainEffects
      || !hasRequiredLowerOrderInteractions
      || !hasCoherentMethod
      || !hasPath(node.id, interaction.outcome, false)) {
      issues.push({ code: "interaction.invalid", subject: node.id });
    }
  }

  for (const edge of structuralEdges) {
    if (edge.source === edge.target) issues.push({ code: "path.self", subject: edge.id });
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) issues.push({ code: "path.unknown_construct", subject: edge.id });
    const identity = JSON.stringify([edge.source, edge.target]);
    if (structuralPaths.has(identity)) issues.push({ code: "path.duplicate", subject: edge.id });
    structuralPaths.add(identity);
  }
  if (containsDirectedCycle(nodeIds, structuralEdges)) issues.push({ code: "path.cycle", subject: "model" });
  return issues;
}

function containsDirectedCycle(nodeIds: Set<string>, edges: Edge[]) {
  const indegree = new Map([...nodeIds].map((id) => [id, 0]));
  const adjacency = new Map<string, string[]>();
  for (const edge of edges) {
    if (edge.source === edge.target || !nodeIds.has(edge.source) || !nodeIds.has(edge.target)) continue;
    adjacency.set(edge.source, [...(adjacency.get(edge.source) ?? []), edge.target]);
    indegree.set(edge.target, (indegree.get(edge.target) ?? 0) + 1);
  }
  const ready = [...indegree].filter(([, degree]) => degree === 0).map(([id]) => id);
  let visited = 0;
  while (ready.length > 0) {
    const source = ready.pop()!;
    visited += 1;
    for (const target of adjacency.get(source) ?? []) {
      const degree = (indegree.get(target) ?? 0) - 1;
      indegree.set(target, degree);
      if (degree === 0) ready.push(target);
    }
  }
  return visited !== nodeIds.size;
}
