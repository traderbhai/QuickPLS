import type { Edge, Node } from "@xyflow/react";
import { isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";
import {
  isModerationAnchorData,
  type ModerationCanvasRequestV1,
} from "../domain/moderationDiagramProjectionV1";
import type { ModelEditCommandV1 } from "../types";

export type NativeCanvasSemanticZoomLevelV1 = "far" | "medium" | "near";

/**
 * Stable semantic-zoom boundaries shared by the editable Canvas and focused
 * behavior tests. The boundary values intentionally select the more detailed
 * level so zooming in never appears to skip a state.
 */
export function nativeCanvasSemanticZoomLevelV1(zoom: number): NativeCanvasSemanticZoomLevelV1 {
  if (!Number.isFinite(zoom)) return "near";
  if (zoom < 0.48) return "far";
  if (zoom < 0.78) return "medium";
  return "near";
}

export interface NativeCanvasSemanticProjectionV1<TData extends Record<string, unknown>> {
  readonly nodes: Array<Node<TData>>;
  readonly edges: Edge[];
}

/**
 * Applies presentation-only semantic zoom and focus/isolation. It never adds,
 * removes, or rewrites a scientific node/relationship identity.
 */
export function projectNativeCanvasSemanticZoomV1<TData extends Record<string, unknown>>(
  nodes: readonly Node<TData>[],
  edges: readonly Edge[],
  level: NativeCanvasSemanticZoomLevelV1,
  isolatedNodeIds: ReadonlySet<string> | null = null,
): NativeCanvasSemanticProjectionV1<TData> {
  const hideIndicators = level !== "near";
  const projectedNodes = nodes.map((node): Node<TData> => {
    const indicator = isIndicatorNodeId(node.id);
    const constructId = indicator ? parseIndicatorNodeId(node.id)?.constructId : null;
    const hiddenByIsolation = Boolean(
      isolatedNodeIds
      && !isolatedNodeIds.has(node.id)
      && (!constructId || !isolatedNodeIds.has(constructId)),
    );
    const data = indicator || isModerationAnchorData(node.data)
      ? node.data
      : { ...node.data, semanticZoomLevel: level } as TData;
    return {
      ...node,
      hidden: Boolean(node.hidden) || hiddenByIsolation || (hideIndicators && indicator),
      data,
    };
  });
  const projectedEdges = edges.map((edge): Edge => {
    const measurement = edge.id.startsWith("measurement::");
    const hiddenByIsolation = Boolean(
      isolatedNodeIds
      && (!isolatedNodeIds.has(edge.source) || !isolatedNodeIds.has(edge.target)),
    );
    return {
      ...edge,
      hidden: Boolean(edge.hidden) || hiddenByIsolation || (hideIndicators && measurement),
    };
  });
  return { nodes: projectedNodes, edges: projectedEdges };
}

export type NativeCanvasConnectionTargetV1 =
  | { readonly kind: "construct"; readonly constructId: string }
  | { readonly kind: "focal_relation"; readonly relationId: string }
  | {
    readonly kind: "moderation_anchor";
    /** Presentation identity only; deliberately omitted from every result. */
    readonly visualNodeId: string;
    readonly interactionTermId: string;
    readonly order: 2 | 3;
  };

export interface NativeCanvasConnectionInputV1 {
  readonly sourceConstructId: string;
  readonly target: NativeCanvasConnectionTargetV1;
  readonly relationId?: string;
  readonly structuralPathExists?: boolean;
  readonly origin?: Extract<ModerationCanvasRequestV1, { action: "create" }>["origin"];
}

export type NativeCanvasConnectionPlanV1 =
  | {
    readonly status: "ready";
    readonly operation: "structural_path";
    readonly command: Extract<ModelEditCommandV1, { kind: "add_path" }>;
  }
  | {
    readonly status: "ready";
    readonly operation: "moderating_effect";
    readonly request: Extract<ModerationCanvasRequestV1, { action: "create" }>;
  }
  | {
    readonly status: "blocked";
    readonly code: "invalid_source" | "invalid_target" | "self_path" | "duplicate_path" | "fourth_order";
    readonly message: string;
  };

/**
 * Classifies one Connect gesture without ever treating a visual path/anchor as
 * a scientific relationship endpoint.
 */
export function planNativeCanvasConnectionV1(
  input: NativeCanvasConnectionInputV1,
): NativeCanvasConnectionPlanV1 {
  const sourceConstructId = input.sourceConstructId.trim();
  if (!sourceConstructId) {
    return { status: "blocked", code: "invalid_source", message: "Choose a measured source construct." };
  }
  if (input.target.kind === "focal_relation") {
    const relationId = input.target.relationId.trim();
    if (!relationId) {
      return { status: "blocked", code: "invalid_target", message: "Choose an eligible focal relationship." };
    }
    return {
      status: "ready",
      operation: "moderating_effect",
      request: {
        action: "create",
        target: { kind: "focal_relation", relationId },
        moderatorId: sourceConstructId,
        origin: input.origin ?? "drag",
      },
    };
  }
  if (input.target.kind === "moderation_anchor") {
    if (input.target.order !== 2) {
      return {
        status: "blocked",
        code: "fourth_order",
        message: "A fourth-order interaction is outside the supported model scope.",
      };
    }
    const interactionTermId = input.target.interactionTermId.trim();
    if (!interactionTermId) {
      return { status: "blocked", code: "invalid_target", message: "Choose an eligible two-way moderating effect." };
    }
    return {
      status: "ready",
      operation: "moderating_effect",
      request: {
        action: "create",
        target: { kind: "parent_interaction", interactionTermId },
        moderatorId: sourceConstructId,
        origin: input.origin ?? "drag",
      },
    };
  }
  const targetConstructId = input.target.constructId.trim();
  if (!targetConstructId) {
    return { status: "blocked", code: "invalid_target", message: "Choose a measured target construct." };
  }
  if (targetConstructId === sourceConstructId) {
    return { status: "blocked", code: "self_path", message: "Self-paths are not valid." };
  }
  if (input.structuralPathExists) {
    return { status: "blocked", code: "duplicate_path", message: "That structural path already exists." };
  }
  const relationId = input.relationId?.trim();
  if (!relationId) {
    return { status: "blocked", code: "invalid_target", message: "Generate a stable relationship identity." };
  }
  return {
    status: "ready",
    operation: "structural_path",
    command: {
      kind: "add_path",
      relationId,
      sourceId: sourceConstructId,
      targetId: targetConstructId,
      label: "Path",
    },
  };
}
