import type { Edge, Node } from "@xyflow/react";
import type { AddTwoStageInteractionBlockReason } from "../store";
import type { ConstructData, PathEdgeData } from "../types";
import { interactionOperands, interactionTermId } from "../domain/moderationDiagramProjectionV1";

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

export interface NativeModeratingEffect {
  interactionNodeId: string;
  interactionTermId: string;
  focalRelationId: string;
  parentInteractionTermId: string | null;
  predictor: string;
  moderatorIds: string[];
  outcome: string;
  order: 2 | 3;
  method: "two_stage" | "product_indicator" | "orthogonalizing";
}

export interface NativeModerationPoint {
  x: number;
  y: number;
}

export interface NativeModerationDropTarget {
  relationship: NativeModerationRelationship;
  distance: number;
}

function edgeRole(edge: Edge): PathEdgeData["role"] {
  return (edge.data as PathEdgeData | undefined)?.role;
}

function isVisualOnlyModerationEdge(edge: Edge): boolean {
  const data = edge.data as (PathEdgeData & {
    visualOnly?: boolean;
    standardSemV4Authority?: { presentationOnly?: boolean };
  }) | undefined;
  return data?.visualOnly === true
    || data?.standardSemV4Authority?.presentationOnly === true;
}

export function isNativeModerationRelationshipEdge(
  edge: Edge,
  nodes: readonly Node<ConstructData>[],
): boolean {
  if (edge.id.startsWith("measurement::")
    || edgeRole(edge) === "control"
    || edgeRole(edge) === "covariance"
    || (edge.data as PathEdgeData | undefined)?.technicalGenerated === true
    || isVisualOnlyModerationEdge(edge)) return false;
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
  excludingInteractionTermId?: string | null,
  allowExistingTwoWayModerator = false,
): NativeModeratorCandidate[] {
  if (!relationship) return [];
  const duplicateModerators = new Set(nodes.flatMap((node) => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    const operands = interaction?.kind === "interaction_v2"
      ? interaction.operands
      : interaction
        ? [interaction.predictor, interaction.moderator]
        : [];
    return interaction
      && interactionTermId(node) !== excludingInteractionTermId
      && operands.length === 2
      && operands[0] === relationship.predictor
      && interaction.outcome === relationship.outcome
      ? [operands[1]!]
      : [];
  }));
  return nodes
    .filter((node) => node.id !== relationship.predictor && node.id !== relationship.outcome)
    .filter((node) => !node.data.semantic && node.data.indicators.length > 0)
    .filter((node) => allowExistingTwoWayModerator || !duplicateModerators.has(node.id))
    .map((node) => ({ id: node.id, label: node.data.label.trim() || node.id }));
}

export function nativeModeratingEffects(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeModeratingEffect[] {
  const effects = nodes.flatMap((node): NativeModeratingEffect[] => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    if (!interaction) return [];
    const operands = interactionOperands(interaction);
    if (operands.length < 2 || operands.length > 3) return [];
    const focal = edges.find((edge) => edge.id === interaction.focalRelationId)
      ?? edges.find((edge) => edge.source === operands[0]
        && edge.target === interaction.outcome
        && edgeRole(edge) !== "control"
        && edgeRole(edge) !== "covariance");
    if (!focal) return [];
    return [{
      interactionNodeId: node.id,
      interactionTermId: interactionTermId(node),
      focalRelationId: focal.id,
      parentInteractionTermId: null,
      predictor: operands[0]!,
      moderatorIds: operands.slice(1),
      outcome: interaction.outcome,
      order: operands.length === 3 ? 3 : 2,
      method: interaction.canonicalMethod ?? "two_stage",
    }];
  });
  return effects.map((effect) => {
    if (effect.order !== 3) return effect;
    const parent = effects
      .filter((candidate) => candidate.order === 2
        && candidate.outcome === effect.outcome
        && candidate.focalRelationId === effect.focalRelationId)
      .find((candidate) => candidate.predictor === effect.predictor
        && candidate.moderatorIds[0] === effect.moderatorIds[0]);
    return { ...effect, parentInteractionTermId: parent?.interactionTermId ?? null };
  });
}

export function nativeModeratingEffect(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  termId: string,
): NativeModeratingEffect | undefined {
  return nativeModeratingEffects(nodes, edges)
    .find((effect) => effect.interactionTermId === termId || effect.interactionNodeId === termId);
}

function distanceToSegment(point: NativeModerationPoint, start: NativeModerationPoint, end: NativeModerationPoint): number {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  if (Math.abs(dx) + Math.abs(dy) < 1e-9) return Math.hypot(point.x - start.x, point.y - start.y);
  const amount = Math.max(0, Math.min(1, ((point.x - start.x) * dx + (point.y - start.y) * dy) / (dx * dx + dy * dy)));
  return Math.hypot(point.x - (start.x + amount * dx), point.y - (start.y + amount * dy));
}

/**
 * Geometry-only path targeting used while a construct is dragged. The caller
 * supplies current visual centers so this remains independent of React Flow
 * zoom, node style, and persisted scientific topology.
 */
export function nearestNativeModerationDropTarget(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  moderatorId: string,
  point: NativeModerationPoint,
  centerForNode: (nodeId: string) => NativeModerationPoint | undefined,
  maximumDistance = 34,
): NativeModerationDropTarget | null {
  if (edges.some((edge) => edgeRole(edge) === "control")) return null;
  const candidates = nativeModerationRelationships(nodes, edges)
    .filter((relationship) => nativeModeratorCandidates(nodes, relationship)
      .some((candidate) => candidate.id === moderatorId))
    .flatMap((relationship): NativeModerationDropTarget[] => {
      const start = centerForNode(relationship.predictor);
      const end = centerForNode(relationship.outcome);
      if (!start || !end) return [];
      return [{ relationship, distance: distanceToSegment(point, start, end) }];
    })
    .filter((candidate) => candidate.distance <= maximumDistance)
    .sort((left, right) => left.distance - right.distance
      || left.relationship.edgeId.localeCompare(right.relationship.edgeId));
  return candidates[0] ?? null;
}

export function canAddNativeModeration(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  selectedEdgeId?: string | null,
): boolean {
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
    case "duplicate_interaction": return "This predictor, moderator, and focal outcome already define a moderating effect. Choose a different moderator or relationship.";
    case "construct_missing": return "One of the selected constructs is no longer available.";
    case "unsupported_construct": return "Predictor, moderator, and outcome must be ordinary measured constructs.";
    case "focal_path_missing": return "The selected predictor-to-outcome relationship no longer exists.";
    case "control_paths_unsupported": return "Remove or convert control paths before creating a moderating effect; this workflow does not accept control paths.";
  }
}
