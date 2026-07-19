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
  const parents = new Map(nodes.map((node) => [node.id, [] as string[]]));
  const children = new Map(nodes.map((node) => [node.id, [] as string[]]));
  for (const edge of edges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target) || edge.source === edge.target) continue;
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
    outgoing.get(edge.source)?.push(edge.target);
    parents.get(edge.target)?.push(edge.source);
    children.get(edge.source)?.push(edge.target);
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

  const orderedColumns = orderColumns(columns, parents, children);

  const positions = new Map<string, XYPosition>();
  for (const [column, columnNodes] of [...orderedColumns.entries()].sort(([a], [b]) => a - b)) {
    const columnOffset = Math.max(0, (Math.max(...[...orderedColumns.values()].map((items) => items.length), 1) - columnNodes.length) * ROW_GAP / 2);
    columnNodes.forEach((node, row) => positions.set(node.id, direction === "horizontal" ? {
      x: ORIGIN.x + column * COLUMN_GAP,
      y: ORIGIN.y + columnOffset + row * ROW_GAP,
    } : {
      x: ORIGIN.x + columnOffset + row * COLUMN_GAP,
      y: ORIGIN.y + column * ROW_GAP,
    }));
  }

  return nodes.map((node) => ({ ...node, position: positions.get(node.id) ?? node.position }));
}

function orderColumns(
  columns: Map<number, Array<Node<ConstructData>>>,
  parents: Map<string, string[]>,
  children: Map<string, string[]>,
) {
  const sortedLevels = [...columns.keys()].sort((a, b) => a - b);
  let ordered = new Map(sortedLevels.map((column) => [
    column,
    [...(columns.get(column) ?? [])].sort((left, right) => left.position.y - right.position.y || left.id.localeCompare(right.id)),
  ]));

  for (let sweep = 0; sweep < 4; sweep += 1) {
    ordered = sweepColumns(ordered, sortedLevels, parents, "forward");
    ordered = sweepColumns(ordered, [...sortedLevels].reverse(), children, "backward");
  }

  return ordered;
}

function sweepColumns(
  ordered: Map<number, Array<Node<ConstructData>>>,
  levels: number[],
  neighbors: Map<string, string[]>,
  direction: "forward" | "backward",
) {
  const next = new Map(ordered);
  for (const column of levels) {
    const columnNodes = next.get(column) ?? [];
    const neighborColumn = direction === "forward" ? column - 1 : column + 1;
    const neighborOrder = new Map((next.get(neighborColumn) ?? []).map((node, index) => [node.id, index]));
    if (neighborOrder.size === 0) continue;
    next.set(column, [...columnNodes].sort((left, right) => {
      const leftScore = barycenter(left, neighbors, neighborOrder);
      const rightScore = barycenter(right, neighbors, neighborOrder);
      return leftScore - rightScore || left.position.y - right.position.y || left.id.localeCompare(right.id);
    }));
  }
  return next;
}

function barycenter(node: Node<ConstructData>, neighbors: Map<string, string[]>, neighborOrder: Map<string, number>) {
  const indexes = (neighbors.get(node.id) ?? [])
    .map((id) => neighborOrder.get(id))
    .filter((index): index is number => typeof index === "number");
  if (indexes.length === 0) return node.position.y / ROW_GAP;
  return indexes.reduce((sum, index) => sum + index, 0) / indexes.length;
}
