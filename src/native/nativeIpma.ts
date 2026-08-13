import type { Edge, Node } from "@xyflow/react";
import type { ConstructData, PathEdgeData } from "../types";

export interface NativeIpmaTargetOption {
  id: string;
  label: string;
  optionLabel: string;
}

function isStructuralPath(edge: Readonly<Edge>): boolean {
  const role = (edge.data as PathEdgeData | undefined)?.role;
  return !edge.id.startsWith("measurement::") && role !== "control" && role !== "covariance";
}

/**
 * IPMA targets are always immutable model IDs. Display labels remain useful to
 * researchers, while the bracketed ID makes duplicate or renamed constructs
 * unambiguous at submission time.
 */
export function nativeIpmaTargetOptions(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeIpmaTargetOption[] {
  const endogenousIds = new Set(edges.filter(isStructuralPath).map((edge) => edge.target));
  return nodes
    .filter((node) => endogenousIds.has(node.id))
    .map((node) => {
      const label = node.data.label.trim() || node.id;
      return { id: node.id, label, optionLabel: `${label} [${node.id}]` };
    });
}

/** Returns all direct and indirect structural predecessors of a target ID. */
export function nativeIpmaPredecessorIds(
  edges: readonly Edge[],
  targetId: string,
): ReadonlySet<string> {
  const sourcesByTarget = new Map<string, Set<string>>();
  for (const edge of edges.filter(isStructuralPath)) {
    const sources = sourcesByTarget.get(edge.target) ?? new Set<string>();
    sources.add(edge.source);
    sourcesByTarget.set(edge.target, sources);
  }

  const predecessors = new Set<string>();
  const pending = [...(sourcesByTarget.get(targetId) ?? [])];
  while (pending.length) {
    const candidate = pending.pop()!;
    if (candidate === targetId || predecessors.has(candidate)) continue;
    predecessors.add(candidate);
    pending.push(...(sourcesByTarget.get(candidate) ?? []));
  }
  return predecessors;
}
