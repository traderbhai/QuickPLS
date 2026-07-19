import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";

export interface ModelIssue {
  code: "construct.empty_name" | "construct.no_indicators" | "indicator.duplicate" | "path.self" | "path.duplicate" | "path.cycle" | "path.unknown_construct";
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
    if (node.data.indicators.length === 0) issues.push({ code: "construct.no_indicators", subject: node.id });
    for (const indicator of node.data.indicators) {
      const owner = indicatorOwners.get(indicator);
      if (owner && owner !== node.id) issues.push({ code: "indicator.duplicate", subject: indicator });
      indicatorOwners.set(indicator, node.id);
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
