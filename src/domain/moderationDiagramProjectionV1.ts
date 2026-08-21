import type { Edge, Node, XYPosition } from "@xyflow/react";
import type { ConstructData, InteractionData } from "../types";

export type ModeratingEffectTargetV1 =
  | { kind: "focal_relation"; relationId: string }
  | { kind: "parent_interaction"; interactionTermId: string };

export interface AddModeratingEffectIntentV3 {
  kind: "add_moderating_effect_v3";
  intentVersion: 3;
  target: ModeratingEffectTargetV1;
  moderatorId: string;
}

export interface ReplaceModeratingEffectIntentV1 {
  kind: "replace_moderating_effect_v1";
  intentVersion: 1;
  interactionTermId: string;
  target: ModeratingEffectTargetV1;
  moderatorId: string;
}

export interface RemoveModeratingEffectIntentV1 {
  kind: "remove_moderating_effect_v1";
  intentVersion: 1;
  interactionTermId: string;
}

export interface ResultOverlaySelectionV1 {
  kind: "mediation" | "moderation" | "three_way_moderation" | "moderated_mediation" | "generic";
  nodeIds: readonly string[];
  relationIds: readonly string[];
  interactionTermIds: readonly string[];
  label: string;
}

export interface ModerationAnchorProjectionV1 extends Record<string, unknown> {
  visualOnly: true;
  relationshipKind: "moderation_anchor";
  interactionNodeId: string;
  interactionTermId: string;
  focalRelationId: string;
  parentInteractionTermId: string | null;
  predictorId: string;
  moderatorIds: string[];
  outcomeId: string;
  order: 2 | 3;
  slotIndex: number;
  slotCount: number;
  fraction: number;
  label: string;
  editable?: boolean;
}

export interface ModerationConnectorProjectionV1 extends Record<string, unknown> {
  visualOnly: true;
  relationshipKind: "moderation_connector";
  interactionTermId: string;
  focalRelationId: string;
  moderatorId: string;
  order: 2 | 3;
  edgeClassName: string;
  routing: "straight" | "polyline";
  bendPoints?: XYPosition[];
}

export type ModerationCanvasRequestV1 =
  | {
    action: "create";
    target: ModeratingEffectTargetV1;
    moderatorId?: string;
    origin: "drag" | "keyboard" | "menu";
  }
  | {
    action: "edit" | "remove";
    interactionTermId: string;
    origin: "anchor" | "keyboard" | "menu";
  };

export const MODERATION_CANVAS_REQUEST_EVENT = "quickpls:moderation-request";
export const MODERATION_FOCUS_EVENT = "quickpls:moderation-focus";
export const MODERATION_ANCHOR_PREFIX = "moderation-anchor::";
export const MODERATION_CONNECTOR_PREFIX = "moderation-connector::";

export const moderationAnchorNodeId = (termId: string) => `${MODERATION_ANCHOR_PREFIX}${encodeURIComponent(termId)}`;
export const moderationConnectorEdgeId = (termId: string, moderatorId: string) =>
  `${MODERATION_CONNECTOR_PREFIX}${encodeURIComponent(termId)}::${encodeURIComponent(moderatorId)}`;

export const isModerationAnchorNodeId = (id: string) => id.startsWith(MODERATION_ANCHOR_PREFIX);
export const isModerationConnectorEdgeId = (id: string) => id.startsWith(MODERATION_CONNECTOR_PREFIX);

export function isModerationAnchorData(value: unknown): value is ModerationAnchorProjectionV1 {
  if (!value || typeof value !== "object") return false;
  const data = value as Partial<ModerationAnchorProjectionV1>;
  return data.visualOnly === true
    && data.relationshipKind === "moderation_anchor"
    && typeof data.interactionTermId === "string";
}

export function isModerationConnectorData(value: unknown): value is ModerationConnectorProjectionV1 {
  if (!value || typeof value !== "object") return false;
  const data = value as Partial<ModerationConnectorProjectionV1>;
  return data.visualOnly === true
    && data.relationshipKind === "moderation_connector"
    && typeof data.interactionTermId === "string";
}

export function interactionOperands(interaction: InteractionData): string[] {
  return interaction.kind === "interaction_v2"
    ? [...interaction.operands]
    : [interaction.predictor, interaction.moderator];
}

export function interactionTermId(node: Node<ConstructData>): string {
  return node.data.interaction?.termId?.trim() || node.id;
}

export function hiddenInteractionNodeIds(
  nodes: readonly Node<ConstructData>[],
  modelEdges?: readonly Edge[],
): Set<string> {
  return new Set(nodes
    .filter((node) => node.data.semantic === "interaction" && node.data.interaction)
    .filter((node) => {
      if (!modelEdges) return true;
      const operands = interactionOperands(node.data.interaction!);
      return operands.length >= 2 && operands.length <= 3 && Boolean(relationForInteraction(node, modelEdges));
    })
    .map((node) => node.id));
}

interface RawInteractionProjection {
  node: Node<ConstructData>;
  termId: string;
  operands: string[];
  outcomeId: string;
  focalRelationId: string;
  parentInteractionTermId: string | null;
}

function relationForInteraction(
  node: Node<ConstructData>,
  modelEdges: readonly Edge[],
): Edge | undefined {
  const interaction = node.data.interaction;
  if (!interaction) return undefined;
  const predictor = interactionOperands(interaction)[0];
  if (!predictor) return undefined;
  return modelEdges.find((edge) => edge.id === interaction.focalRelationId)
    ?? modelEdges.find((edge) => edge.source === predictor
      && edge.target === interaction.outcome
      && edge.data?.role !== "control"
      && edge.data?.role !== "covariance");
}

function isOperandSubset(candidate: RawInteractionProjection, parent: RawInteractionProjection): boolean {
  if (candidate.outcomeId !== parent.outcomeId || candidate.operands.length >= parent.operands.length) return false;
  const parentOperands = new Set(parent.operands);
  return candidate.operands.every((operand) => parentOperands.has(operand));
}

function recommendedParentTerm(
  current: RawInteractionProjection,
  all: readonly RawInteractionProjection[],
): string | null {
  if (current.operands.length < 3) return null;
  return all
    .filter((candidate) => candidate.outcomeId === current.outcomeId
      && candidate.focalRelationId === current.focalRelationId
      && candidate.operands.length === 2
      && candidate.operands[0] === current.operands[0]
      && candidate.operands[1] === current.operands[1])
    .sort((left, right) => left.termId.localeCompare(right.termId))[0]?.termId ?? null;
}

function moderationFraction(index: number, count: number): number {
  if (count <= 1) return 0.5;
  const spacing = Math.min(0.14, 0.46 / Math.max(1, count - 1));
  return Number(Math.max(0.27, Math.min(0.73, 0.5 + (index - (count - 1) / 2) * spacing)).toFixed(4));
}

/**
 * Derive the visible moderation vocabulary from scientific interaction nodes.
 * Generated lower-order closure terms are intentionally folded into their
 * maximal three-way parent so the ordinary canvas never becomes a technical
 * interaction-term graph.
 */
export function deriveModerationAnchorProjections(
  modelNodes: readonly Node<ConstructData>[],
  modelEdges: readonly Edge[],
  anchorFractions: Readonly<Record<string, number>> = {},
): ModerationAnchorProjectionV1[] {
  const labels = new Map(modelNodes.map((node) => [node.id, node.data.label.trim() || node.id]));
  const raw = modelNodes.flatMap((node): RawInteractionProjection[] => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    if (!interaction) return [];
    const operands = interactionOperands(interaction);
    const relation = relationForInteraction(node, modelEdges);
    if (!relation || operands.length < 2 || operands.length > 3) return [];
    return [{
      node,
      termId: interactionTermId(node),
      operands,
      outcomeId: interaction.outcome,
      focalRelationId: relation.id,
      parentInteractionTermId: null,
    }];
  });
  const maximal = raw
    .filter((candidate) => !raw.some((parent) => isOperandSubset(candidate, parent)))
    .map((candidate) => ({ ...candidate, parentInteractionTermId: recommendedParentTerm(candidate, raw) }))
    .sort((left, right) => left.focalRelationId.localeCompare(right.focalRelationId)
      || left.termId.localeCompare(right.termId));
  const byRelation = new Map<string, RawInteractionProjection[]>();
  for (const candidate of maximal) {
    byRelation.set(candidate.focalRelationId, [...(byRelation.get(candidate.focalRelationId) ?? []), candidate]);
  }
  return maximal.map((candidate) => {
    const siblings = byRelation.get(candidate.focalRelationId) ?? [candidate];
    const slotIndex = siblings.findIndex((sibling) => sibling.termId === candidate.termId);
    const order = candidate.operands.length === 3 ? 3 : 2;
    const predictorId = candidate.operands[0]!;
    const moderatorIds = candidate.operands.slice(1);
    const predictor = labels.get(predictorId) ?? predictorId;
    const moderators = moderatorIds.map((id) => labels.get(id) ?? id);
    const outcome = labels.get(candidate.outcomeId) ?? candidate.outcomeId;
    const storedFraction = Number(anchorFractions[candidate.termId]);
    const fraction = Number.isFinite(storedFraction)
      ? Math.max(0.2, Math.min(0.8, storedFraction))
      : moderationFraction(slotIndex, siblings.length);
    return {
      visualOnly: true,
      relationshipKind: "moderation_anchor",
      interactionNodeId: candidate.node.id,
      interactionTermId: candidate.termId,
      focalRelationId: candidate.focalRelationId,
      parentInteractionTermId: candidate.parentInteractionTermId,
      predictorId,
      moderatorIds,
      outcomeId: candidate.outcomeId,
      order,
      slotIndex,
      slotCount: siblings.length,
      fraction,
      label: order === 3
        ? `Three-way moderating effect: ${moderators.join(" and ")} moderate ${predictor} to ${outcome}`
        : `Moderating effect: ${moderators[0]} moderates ${predictor} to ${outcome}`,
    };
  });
}

export function moderationAnchorPosition(
  source: XYPosition,
  target: XYPosition,
  fraction: number,
  anchorSize = 22,
): XYPosition {
  return {
    x: source.x + (target.x - source.x) * fraction - anchorSize / 2,
    y: source.y + (target.y - source.y) * fraction - anchorSize / 2,
  };
}

export function dispatchModerationCanvasRequest(detail: ModerationCanvasRequestV1): void {
  window.dispatchEvent(new CustomEvent<ModerationCanvasRequestV1>(MODERATION_CANVAS_REQUEST_EVENT, { detail }));
}
