import type { Edge, Node } from "@xyflow/react";
import type { AddTwoStageInteractionBlockReason } from "../store";
import type { ConstructData, PathEdgeData } from "../types";

export interface NativeModerationRelationship {
  edgeId: string;
  predictor: string;
  predictorLabel: string;
  outcome: string;
  outcomeLabel: string;
  label: string;
}

export interface NativeModeratorCandidate {
  id: string;
  label: string;
}

function edgeRole(edge: Edge): PathEdgeData["role"] {
  return (edge.data as PathEdgeData | undefined)?.role;
}

export function isNativeModerationRelationshipEdge(
  edge: Edge,
  nodes: readonly Node<ConstructData>[],
): boolean {
  if (edge.id.startsWith("measurement::") || edgeRole(edge) === "control" || edgeRole(edge) === "covariance") return false;
  if (edge.source === edge.target) return false;
  const source = nodes.find((node) => node.id === edge.source);
  const target = nodes.find((node) => node.id === edge.target);
  return Boolean(
    source
    && target
    && !source.data.semantic
    && !target.data.semantic
    && source.data.indicators.length > 0
    && target.data.indicators.length > 0,
  );
}

export function nativeModerationRelationships(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeModerationRelationship[] {
  return edges
    .filter((edge) => isNativeModerationRelationshipEdge(edge, nodes))
    .map((edge) => {
      const predictorLabel = nodes.find((node) => node.id === edge.source)?.data.label.trim() || edge.source;
      const outcomeLabel = nodes.find((node) => node.id === edge.target)?.data.label.trim() || edge.target;
      return {
        edgeId: edge.id,
        predictor: edge.source,
        predictorLabel,
        outcome: edge.target,
        outcomeLabel,
        label: `${predictorLabel} → ${outcomeLabel}`,
      };
    });
}

export function nativeModeratorCandidates(
  nodes: readonly Node<ConstructData>[],
  relationship: NativeModerationRelationship | undefined,
): NativeModeratorCandidate[] {
  if (!relationship) return [];
  return nodes
    .filter((node) => node.id !== relationship.predictor && node.id !== relationship.outcome)
    .filter((node) => !node.data.semantic && node.data.indicators.length > 0)
    .map((node) => ({ id: node.id, label: node.data.label.trim() || node.id }));
}

export function canAddNativeModeration(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  selectedEdgeId?: string | null,
): boolean {
  if (nodes.some((node) => node.data.semantic === "interaction")) return false;
  if (edges.some((edge) => edgeRole(edge) === "control")) return false;
  const relationships = nativeModerationRelationships(nodes, edges);
  const candidates = selectedEdgeId
    ? relationships.filter((relationship) => relationship.edgeId === selectedEdgeId)
    : relationships;
  return candidates
    .some((relationship) => nativeModeratorCandidates(nodes, relationship).length > 0);
}

export function nativeModerationCreationError(reason: AddTwoStageInteractionBlockReason): string {
  switch (reason) {
    case "constructs_not_distinct": return "Choose three different constructs for predictor, moderator, and outcome.";
    case "interaction_exists": return "Create only one two-way moderating effect per model.";
    case "construct_missing": return "One of the selected constructs is no longer available.";
    case "unsupported_construct": return "Predictor, moderator, and outcome must be ordinary measured constructs.";
    case "focal_path_missing": return "The selected predictor-to-outcome relationship no longer exists.";
    case "control_paths_unsupported": return "Remove or convert control paths before creating a moderating effect; this workflow does not accept control paths.";
  }
}
