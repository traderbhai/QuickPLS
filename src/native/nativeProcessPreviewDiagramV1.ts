import type { Edge, Node } from "@xyflow/react";
import { buildDiagramGraph, type DiagramGraph } from "../domain/diagramGraph";
import type { ModerationAnchorProjectionV1 } from "../domain/moderationDiagramProjectionV1";
import type { ConstructData, NativeProcessGraphRelationshipConfig } from "../types";

const pathId = (from: string, to: string) => `process-preview:path:${encodeURIComponent(from)}:${encodeURIComponent(to)}`;

export function nativeProcessPreviewDiagramV1(graph: NativeProcessGraphRelationshipConfig, outcome: string): DiagramGraph {
  const variables = [...new Set([
    graph.focal_predictor,
    ...graph.paths.flatMap((path) => [path.from, path.to]),
    ...graph.moderators.map((moderator) => moderator.variable),
    ...graph.moderations.flatMap((moderation) => [moderation.moderator, moderation.conditioning_moderator ?? ""]),
    outcome,
  ].filter(Boolean))];
  const nodes: Array<Node<ConstructData>> = variables.map((variable, index) => ({
    id: variable,
    type: "construct",
    position: { x: (index % 4) * 240, y: Math.floor(index / 4) * 190 },
    data: { label: variable, shortName: variable, mode: "reflective", indicators: [] },
  }));
  const edges: Edge[] = graph.paths.map((path) => ({
    id: pathId(path.from, path.to),
    source: path.from,
    target: path.to,
    label: "Path",
    type: "straight",
    data: { visualOnly: true, relationshipKind: "process_preview_path" },
  }));
  const byFocal = new Map<string, number>();
  const anchors: ModerationAnchorProjectionV1[] = graph.moderations.map((moderation, index) => {
    const focalRelationId = pathId(moderation.from, moderation.to);
    const slotIndex = byFocal.get(focalRelationId) ?? 0;
    byFocal.set(focalRelationId, slotIndex + 1);
    const moderatorIds = [moderation.moderator, moderation.conditioning_moderator].filter((value): value is string => Boolean(value));
    const order = moderatorIds.length > 1 ? 3 as const : 2 as const;
    return {
      visualOnly: true,
      relationshipKind: "moderation_anchor",
      interactionNodeId: `process-preview:interaction:${index}`,
      interactionTermId: `process-preview:moderation:${index}:${encodeURIComponent(moderatorIds.join("x"))}`,
      focalRelationId,
      parentInteractionTermId: null,
      predictorId: moderation.from,
      moderatorIds,
      outcomeId: moderation.to,
      order,
      slotIndex,
      slotCount: graph.moderations.filter((candidate) => candidate.from === moderation.from && candidate.to === moderation.to).length,
      fraction: 0.5 + (slotIndex % 2 === 0 ? 1 : -1) * Math.ceil(slotIndex / 2) * 0.1,
      label: order === 3
        ? `Three-way moderating effect: ${moderatorIds.join(" and ")} moderate ${moderation.from} to ${moderation.to}`
        : `Moderating effect: ${moderatorIds[0]} moderates ${moderation.from} to ${moderation.to}`,
    };
  });
  return buildDiagramGraph(nodes, edges, "smartpls_result", "model", undefined, {
    layoutSource: "tidy_publication",
    moderationAnchorProjections: anchors,
  });
}
