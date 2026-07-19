import type { Edge, Node, XYPosition } from "@xyflow/react";
import type { ConstructData } from "../types";

const COLUMN_GAP = 240;
const ROW_GAP = 190;
const ORIGIN: XYPosition = { x: 80, y: 85 };

export function layoutModel(nodes: Array<Node<ConstructData>>, edges: Edge[], direction: "horizontal" | "vertical" = "horizontal"): Array<Node<ConstructData>> {
  if (nodes.length === 0) return [];

  const nodeIds = new Set(nodes.map((node) => node.id));
  const incoming = new Map(nodes.map((node) => [node.id, 0]));
  const outgoing = new Map(nodes.map((node) => [node.id, [] as string[]]));
  for (const edge of edges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target) || edge.source === edge.target) continue;
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
    outgoing.get(edge.source)?.push(edge.target);
  }

  const level = new Map<string, number>();
  const queue = nodes.filter((node) => incoming.get(node.id) === 0).map((node) => node.id);
  for (const id of queue) level.set(id, 0);

  for (let index = 0; index < queue.length; index += 1) {
    const source = queue[index];
    for (const target of outgoing.get(source) ?? []) {
      level.set(target, Math.max(level.get(target) ?? 0, (level.get(source) ?? 0) + 1));
      incoming.set(target, (incoming.get(target) ?? 1) - 1);
      if (incoming.get(target) === 0) queue.push(target);
    }
  }

  const fallbackLevel = Math.max(0, ...level.values()) + 1;
  for (const node of nodes) if (!level.has(node.id)) level.set(node.id, fallbackLevel);

  const columns = new Map<number, Array<Node<ConstructData>>>();
  for (const node of nodes) {
    const column = level.get(node.id) ?? 0;
    columns.set(column, [...(columns.get(column) ?? []), node]);
  }

  const positions = new Map<string, XYPosition>();
  for (const [column, columnNodes] of [...columns.entries()].sort(([a], [b]) => a - b)) {
    columnNodes.forEach((node, row) => positions.set(node.id, direction === "horizontal" ? {
      x: ORIGIN.x + column * COLUMN_GAP,
      y: ORIGIN.y + row * ROW_GAP,
    } : {
      x: ORIGIN.x + row * COLUMN_GAP,
      y: ORIGIN.y + column * ROW_GAP,
    }));
  }

  return nodes.map((node) => ({ ...node, position: positions.get(node.id) ?? node.position }));
}
