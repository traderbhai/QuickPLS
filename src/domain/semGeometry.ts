import type { Edge, Node, XYPosition } from "@xyflow/react";

export type SemSide = "left" | "right" | "top" | "bottom";
export type SemNodeKind = "latent" | "indicator" | "compact";

export interface SemSize {
  width: number;
  height: number;
}

export interface SemBox extends SemSize {
  x: number;
  y: number;
  kind: SemNodeKind;
  ellipse?: boolean;
  ellipseWidth?: number;
  ellipseHeight?: number;
  ellipseOffsetY?: number;
}

export interface SemPoint {
  x: number;
  y: number;
}

export interface SemRoute {
  source: SemSide;
  target: SemSide;
  start: SemPoint;
  end: SemPoint;
  length: number;
}

export interface SemDiagramQuality {
  latentOverlapCount: number;
  indicatorOverlapCount: number;
  pathCrossingCount: number;
  labelOverlapCount: number;
  averageStructuralPathLength: number;
  averageMeasurementPathLength: number;
  unnecessaryBendCount: number;
}

export const SEM_SIZES = {
  compactLatent: { width: 150, height: 110 },
  compactIndicator: { width: 96, height: 34 },
  smartplsLatent: { width: 104, height: 94 },
  smartplsEllipse: { width: 104, height: 68 },
  smartplsIndicator: { width: 88, height: 28 },
} as const;

export function semNodeBox(node: Pick<Node, "type" | "position">): SemBox {
  if (node.type === "indicator") {
    return { x: node.position.x, y: node.position.y, kind: "indicator", ...SEM_SIZES.smartplsIndicator };
  }
  if (node.type === "latent") {
    return {
      x: node.position.x,
      y: node.position.y,
      kind: "latent",
      ...SEM_SIZES.smartplsLatent,
      ellipse: true,
      ellipseWidth: SEM_SIZES.smartplsEllipse.width,
      ellipseHeight: SEM_SIZES.smartplsEllipse.height,
      ellipseOffsetY: 0,
    };
  }
  return { x: node.position.x, y: node.position.y, kind: "compact", ...SEM_SIZES.compactLatent, ellipse: true };
}

export function boxCenter(box: SemBox): SemPoint {
  if (box.ellipse) {
    return {
      x: box.x + (box.ellipseWidth ?? box.width) / 2,
      y: box.y + (box.ellipseOffsetY ?? 0) + (box.ellipseHeight ?? box.height) / 2,
    };
  }
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

export function boundaryPoint(box: SemBox, toward: SemPoint): SemPoint {
  const center = boxCenter(box);
  const dx = toward.x - center.x;
  const dy = toward.y - center.y;
  if (Math.abs(dx) < 1e-9 && Math.abs(dy) < 1e-9) return center;

  if (box.ellipse) {
    const rx = (box.ellipseWidth ?? box.width) / 2;
    const ry = (box.ellipseHeight ?? box.height) / 2;
    const scale = 1 / Math.sqrt((dx * dx) / (rx * rx) + (dy * dy) / (ry * ry));
    return { x: center.x + dx * scale, y: center.y + dy * scale };
  }

  const halfWidth = box.width / 2;
  const halfHeight = box.height / 2;
  const scaleX = Math.abs(dx) < 1e-9 ? Number.POSITIVE_INFINITY : halfWidth / Math.abs(dx);
  const scaleY = Math.abs(dy) < 1e-9 ? Number.POSITIVE_INFINITY : halfHeight / Math.abs(dy);
  const scale = Math.min(scaleX, scaleY);
  return { x: center.x + dx * scale, y: center.y + dy * scale };
}

export function sideForBoundaryPoint(box: SemBox, point: SemPoint): SemSide {
  const center = boxCenter(box);
  const horizontalRadius = box.ellipse ? (box.ellipseWidth ?? box.width) / 2 : box.width / 2;
  const verticalRadius = box.ellipse ? (box.ellipseHeight ?? box.height) / 2 : box.height / 2;
  const nx = (point.x - center.x) / Math.max(1, horizontalRadius);
  const ny = (point.y - center.y) / Math.max(1, verticalRadius);
  if (Math.abs(nx) >= Math.abs(ny)) return nx >= 0 ? "right" : "left";
  return ny >= 0 ? "bottom" : "top";
}

export function routeBetweenBoxes(sourceBox: SemBox, targetBox: SemBox): SemRoute {
  const targetCenter = boxCenter(targetBox);
  const sourceCenter = boxCenter(sourceBox);
  const start = boundaryPoint(sourceBox, targetCenter);
  const end = boundaryPoint(targetBox, sourceCenter);
  return {
    source: sideForBoundaryPoint(sourceBox, start),
    target: sideForBoundaryPoint(targetBox, end),
    start,
    end,
    length: distance(start, end),
  };
}

export function distance(left: SemPoint, right: SemPoint): number {
  return Math.hypot(right.x - left.x, right.y - left.y);
}

export function translatePoint(point: SemPoint, delta: SemPoint): SemPoint {
  return { x: point.x + delta.x, y: point.y + delta.y };
}

export function measureDiagramQuality(nodes: Array<Node>, edges: Edge[]): SemDiagramQuality {
  const boxes = new Map(nodes.map((node) => [node.id, semNodeBox(node)]));
  const latents = nodes.filter((node) => node.type === "latent").map((node) => boxes.get(node.id)!);
  const indicators = nodes.filter((node) => node.type === "indicator").map((node) => boxes.get(node.id)!);
  const structuralSegments: Array<[SemPoint, SemPoint]> = [];
  const measurementSegments: Array<[SemPoint, SemPoint]> = [];
  let structuralLength = 0;
  let measurementLength = 0;
  let structuralCount = 0;
  let measurementCount = 0;
  let unnecessaryBendCount = 0;

  for (const edge of edges) {
    const source = boxes.get(edge.source);
    const target = boxes.get(edge.target);
    if (!source || !target) continue;
    const route = routeBetweenBoxes(source, target);
    const segment: [SemPoint, SemPoint] = [route.start, route.end];
    const className = String(edge.className ?? edge.data?.edgeClassName ?? "");
    const routing = String(edge.data?.routing ?? edge.type ?? "straight");
    if (routing !== "straight" && !className.includes("covariance")) unnecessaryBendCount += 1;
    if (className.includes("measurement-edge") || edge.id.startsWith("measurement::")) {
      measurementSegments.push(segment);
      measurementLength += route.length;
      measurementCount += 1;
    } else if (!className.includes("covariance")) {
      structuralSegments.push(segment);
      structuralLength += route.length;
      structuralCount += 1;
    }
  }

  let pathCrossingCount = 0;
  for (let i = 0; i < structuralSegments.length; i += 1) {
    for (let j = i + 1; j < structuralSegments.length; j += 1) {
      if (segmentsIntersect(structuralSegments[i][0], structuralSegments[i][1], structuralSegments[j][0], structuralSegments[j][1])) {
        pathCrossingCount += 1;
      }
    }
  }

  return {
    latentOverlapCount: overlapCount(latents),
    indicatorOverlapCount: overlapCount(indicators),
    pathCrossingCount,
    labelOverlapCount: 0,
    averageStructuralPathLength: structuralCount ? structuralLength / structuralCount : 0,
    averageMeasurementPathLength: measurementCount ? measurementLength / measurementCount : 0,
    unnecessaryBendCount,
  };
}

function overlapCount(boxes: SemBox[]): number {
  let count = 0;
  for (let i = 0; i < boxes.length; i += 1) {
    for (let j = i + 1; j < boxes.length; j += 1) {
      if (rectanglesOverlap(boxes[i], boxes[j])) count += 1;
    }
  }
  return count;
}

function rectanglesOverlap(left: SemBox, right: SemBox): boolean {
  return left.x < right.x + right.width
    && left.x + left.width > right.x
    && left.y < right.y + right.height
    && left.y + left.height > right.y;
}

function segmentsIntersect(a: SemPoint, b: SemPoint, c: SemPoint, d: SemPoint): boolean {
  if (sharesEndpoint(a, b, c, d)) return false;
  const o1 = orientation(a, b, c);
  const o2 = orientation(a, b, d);
  const o3 = orientation(c, d, a);
  const o4 = orientation(c, d, b);
  return o1 * o2 < 0 && o3 * o4 < 0;
}

function sharesEndpoint(a: SemPoint, b: SemPoint, c: SemPoint, d: SemPoint): boolean {
  return samePoint(a, c) || samePoint(a, d) || samePoint(b, c) || samePoint(b, d);
}

function samePoint(left: SemPoint, right: SemPoint): boolean {
  return Math.abs(left.x - right.x) < 1e-6 && Math.abs(left.y - right.y) < 1e-6;
}

function orientation(a: SemPoint, b: SemPoint, c: SemPoint): number {
  return (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
}

export function smartIndicatorPosition(base: XYPosition, count: number, side: SemSide): XYPosition[] {
  if (count === 0) return [];
  const gap = 42;
  const horizontalGap = 94;
  if (side === "top" || side === "bottom") {
    const stackWidth = Math.max(0, count - 1) * (SEM_SIZES.smartplsIndicator.width + 10);
    const y = side === "top"
      ? base.y - horizontalGap
      : base.y + SEM_SIZES.smartplsEllipse.height + horizontalGap;
    return Array.from({ length: count }, (_, index) => ({
      x: base.x + SEM_SIZES.smartplsEllipse.width / 2 - SEM_SIZES.smartplsIndicator.width / 2 - stackWidth / 2 + index * (SEM_SIZES.smartplsIndicator.width + 10),
      y,
    }));
  }
  const stackHeight = Math.max(0, count - 1) * gap;
  const x = side === "left"
    ? base.x - horizontalGap - SEM_SIZES.smartplsIndicator.width
    : base.x + SEM_SIZES.smartplsEllipse.width + horizontalGap;
  const centerY = base.y + SEM_SIZES.smartplsEllipse.height / 2 - SEM_SIZES.smartplsIndicator.height / 2;
  return Array.from({ length: count }, (_, index) => ({ x, y: centerY - stackHeight / 2 + index * gap }));
}
