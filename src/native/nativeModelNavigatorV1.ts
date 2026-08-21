import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";

export type NativeModelNavigatorRelationshipV1 =
  | {
    readonly id: string;
    readonly kind: "relationship";
    readonly relationId: string;
    readonly label: string;
    readonly detail: string;
  }
  | {
    readonly id: string;
    readonly kind: "moderation";
    readonly interactionTermId: string;
    readonly label: string;
    readonly detail: string;
  }
  | {
    readonly id: string;
    readonly kind: "higher_order";
    readonly constructId: string;
    readonly label: string;
    readonly detail: string;
  };

function authoredLabel(nodes: readonly Node<ConstructData>[], id: string): string {
  return nodes.find((node) => node.id === id)?.data.label?.trim() || "Unnamed construct";
}

export function nativeModelNavigatorRelationshipsV1(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): readonly NativeModelNavigatorRelationshipV1[] {
  const visibleConstructIds = new Set(nodes
    .filter((node) => node.data.semantic !== "interaction")
    .map((node) => node.id));
  const relationships: NativeModelNavigatorRelationshipV1[] = edges.flatMap((edge) => {
    if (edge.id.startsWith("measurement::")
      || edge.data?.visualOnly
      || edge.data?.technicalGenerated
      || !visibleConstructIds.has(edge.source)
      || !visibleConstructIds.has(edge.target)) return [];
    const source = authoredLabel(nodes, edge.source);
    const target = authoredLabel(nodes, edge.target);
    const role = edge.data?.role === "covariance"
      ? "Covariance"
      : edge.data?.role === "control"
        ? "Control relationship"
        : "Structural relationship";
    return [{
      id: `relationship:${edge.id}`,
      kind: "relationship",
      relationId: edge.id,
      label: edge.data?.role === "covariance" ? `${source} ↔ ${target}` : `${source} → ${target}`,
      detail: role,
    }];
  });
  const moderation: NativeModelNavigatorRelationshipV1[] = nodes.flatMap((node) => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    if (!interaction) return [];
    const operands = interaction.kind === "interaction_v2"
      ? interaction.operands
      : [interaction.predictor, interaction.moderator];
    const predictor = authoredLabel(nodes, operands[0] ?? "");
    const moderators = operands.slice(1).map((id) => authoredLabel(nodes, id));
    const outcome = authoredLabel(nodes, interaction.outcome);
    const termId = interaction.kind === "interaction_v2" ? interaction.termId : node.id;
    return [{
      id: `moderation:${termId}`,
      kind: "moderation",
      interactionTermId: termId,
      label: moderators.length === 1
        ? `${moderators[0]} moderates ${predictor} → ${outcome}`
        : `${moderators.at(-1)} extends ${predictor} × ${moderators.slice(0, -1).join(" × ")} → ${outcome}`,
      detail: operands.length === 3 ? "Three-way moderation" : "Two-way moderation",
    }];
  });
  const higherOrder: NativeModelNavigatorRelationshipV1[] = nodes.flatMap((node) => {
    if (node.data.semantic !== "higher_order" || !node.data.higherOrder) return [];
    const components = node.data.higherOrder.components.map((id) => authoredLabel(nodes, id));
    return [{
      id: `higher-order:${node.id}`,
      kind: "higher_order",
      constructId: node.id,
      label: `${node.data.label}: ${components.join(", ")}`,
      detail: "Higher-order components",
    }];
  });
  return [...relationships, ...moderation, ...higherOrder]
    .sort((left, right) => left.label.localeCompare(right.label));
}
