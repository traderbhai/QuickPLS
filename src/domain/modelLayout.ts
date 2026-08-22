import type { Edge, Node, XYPosition } from "@xyflow/react";
import type { ConstructData, DiagramLayoutState, IndicatorSide } from "../types";
import { SEM_SIZES, smartIndicatorPosition, type SemRect } from "./semGeometry";

const ROW_GAP = 190;
const ORIGIN: XYPosition = { x: 80, y: 85 };

export function layoutModel(
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
  direction: "horizontal" | "vertical" = "horizontal",
  layout?: DiagramLayoutState,
): Array<Node<ConstructData>> {
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

  const envelopes = new Map(nodes.map((node) => [node.id, modelNodeEnvelope(
    node,
    layout,
    (parents.get(node.id)?.length ?? 0) === 0 ? "left" : (children.get(node.id)?.length ?? 0) === 0 ? "right" : "top",
  )]));
  const positions = direction === "horizontal"
    ? horizontalEnvelopeLayout(orderedColumns, envelopes)
    : verticalEnvelopeLayout(orderedColumns, envelopes);

  return nodes.map((node) => ({ ...node, position: positions.get(node.id) ?? node.position }));
}

function modelNodeEnvelope(node: Node<ConstructData>, layout: DiagramLayoutState | undefined, fallbackSide: Exclude<IndicatorSide, "free">): SemRect {
  const positions: XYPosition[] = [];
  const bySide = new Map<Exclude<IndicatorSide, "free">, Array<{ index: number; order: number }>>();
  node.data.indicators.forEach((indicator, index) => {
    const item = layout?.indicatorLayouts[node.id]?.[indicator];
    if (item?.side === "free" && Number.isFinite(item.x) && Number.isFinite(item.y)) {
      // Free indicators are persisted in absolute Canvas coordinates. They do
      // not move with an arranged construct, so they cannot define its
      // construct-relative envelope.
      return;
    }
    const side = item?.side && item.side !== "free" ? item.side : fallbackSide;
    bySide.set(side, [...(bySide.get(side) ?? []), { index, order: item?.order ?? index }]);
  });
  for (const [side, entries] of bySide) {
    positions.push(...smartIndicatorPosition({ x: 0, y: 0 }, entries.length, side));
  }
  const rects: SemRect[] = [
    { x: 0, y: 0, width: SEM_SIZES.smartplsLatent.width, height: SEM_SIZES.smartplsLatent.height },
    ...positions.map((position) => ({ x: position.x, y: position.y, width: SEM_SIZES.smartplsIndicator.width, height: SEM_SIZES.smartplsIndicator.height })),
  ];
  const left = Math.min(...rects.map((rect) => rect.x));
  const top = Math.min(...rects.map((rect) => rect.y));
  const right = Math.max(...rects.map((rect) => rect.x + rect.width));
  const bottom = Math.max(...rects.map((rect) => rect.y + rect.height));
  return { x: left, y: top, width: right - left, height: bottom - top };
}

function horizontalEnvelopeLayout(columns: Map<number, Array<Node<ConstructData>>>, envelopes: Map<string, SemRect>): Map<string, XYPosition> {
  const positions = new Map<string, XYPosition>();
  const metrics = new Map([...columns].map(([level, items]) => {
    const extents = items.map((node) => envelopes.get(node.id)!);
    return [level, {
      left: Math.min(...extents.map((extent) => extent.x)),
      right: Math.max(...extents.map((extent) => extent.x + extent.width)),
      height: extents.reduce((sum, extent) => sum + extent.height, 0) + Math.max(0, extents.length - 1) * 84,
    }] as const;
  }));
  const totalHeight = Math.max(...[...metrics.values()].map((metric) => metric.height), 1);
  let x = ORIGIN.x;
  for (const [level, items] of [...columns].sort(([left], [right]) => left - right)) {
    const metric = metrics.get(level)!;
    const originX = x - metric.left;
    let y = ORIGIN.y + (totalHeight - metric.height) / 2;
    for (const node of items) {
      const envelope = envelopes.get(node.id)!;
      positions.set(node.id, { x: originX, y: y - envelope.y });
      y += envelope.height + 84;
    }
    x += metric.right - metric.left + 130;
  }
  return positions;
}

function verticalEnvelopeLayout(rows: Map<number, Array<Node<ConstructData>>>, envelopes: Map<string, SemRect>): Map<string, XYPosition> {
  const positions = new Map<string, XYPosition>();
  const metrics = new Map([...rows].map(([level, items]) => {
    const extents = items.map((node) => envelopes.get(node.id)!);
    return [level, {
      top: Math.min(...extents.map((extent) => extent.y)),
      bottom: Math.max(...extents.map((extent) => extent.y + extent.height)),
      width: extents.reduce((sum, extent) => sum + extent.width, 0) + Math.max(0, extents.length - 1) * 84,
    }] as const;
  }));
  const totalWidth = Math.max(...[...metrics.values()].map((metric) => metric.width), 1);
  let y = ORIGIN.y;
  for (const [level, items] of [...rows].sort(([left], [right]) => left - right)) {
    const metric = metrics.get(level)!;
    const originY = y - metric.top;
    let x = ORIGIN.x + (totalWidth - metric.width) / 2;
    for (const node of items) {
      const envelope = envelopes.get(node.id)!;
      positions.set(node.id, { x: x - envelope.x, y: originY });
      x += envelope.width + 84;
    }
    y += metric.bottom - metric.top + 130;
  }
  return positions;
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
