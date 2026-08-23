import { Position, getBezierPath, getSmoothStepPath, type Edge, type Node, type XYPosition } from "@xyflow/react";

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

export interface SemRect {
  x: number;
  y: number;
  width: number;
  height: number;
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
  if (node.type === "moderationAnchor") {
    return { x: node.position.x, y: node.position.y, width: 22, height: 22, kind: "compact", ellipse: true };
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
    if (!Number.isFinite(rx) || !Number.isFinite(ry) || rx <= 0 || ry <= 0) return center;
    const denominator = Math.sqrt((dx * dx) / (rx * rx) + (dy * dy) / (ry * ry));
    if (!Number.isFinite(denominator) || denominator < 1e-9) return center;
    const scale = 1 / denominator;
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

function usableAim(primary: SemPoint | undefined, fallback: SemPoint, center: SemPoint, fallbackDirection: -1 | 1): SemPoint {
  for (const candidate of [primary, fallback]) {
    if (candidate
      && Number.isFinite(candidate.x)
      && Number.isFinite(candidate.y)
      && (Math.abs(candidate.x - center.x) >= 1e-9 || Math.abs(candidate.y - center.y) >= 1e-9)) {
      return candidate;
    }
  }
  return { x: center.x + fallbackDirection, y: center.y };
}

/**
 * Resolves exact visual endpoints on the source and target perimeters.
 * Straight, curved, and orthogonal routes aim toward the opposite center. A
 * polyline passes its bends so each endpoint instead aims toward its adjacent
 * segment. Coincident centers and center-aligned bends use a deterministic
 * horizontal fallback, keeping every returned coordinate finite.
 */
export function continuousBoundaryRoute(
  sourceBox: SemBox,
  targetBox: SemBox,
  bends: readonly SemPoint[] = [],
): SemRoute {
  const sourceCenter = boxCenter(sourceBox);
  const targetCenter = boxCenter(targetBox);
  const finiteBends = bends.filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y));
  const sourceAim = usableAim(finiteBends[0], targetCenter, sourceCenter, 1);
  const targetAim = usableAim(finiteBends.at(-1), sourceCenter, targetCenter, -1);
  const start = boundaryPoint(sourceBox, sourceAim);
  const end = boundaryPoint(targetBox, targetAim);
  return {
    source: sideForBoundaryPoint(sourceBox, start),
    target: sideForBoundaryPoint(targetBox, end),
    start,
    end,
    length: distance(start, end),
  };
}

function sideFromHandleId(handleId: string | null | undefined): SemSide | undefined {
  const side = handleId?.split("-").at(-1);
  return side === "left" || side === "right" || side === "top" || side === "bottom"
    ? side
    : undefined;
}

export function pointOnBoxSide(box: SemBox, side: SemSide): SemPoint {
  const center = boxCenter(box);
  const halfWidth = box.ellipse ? (box.ellipseWidth ?? box.width) / 2 : box.width / 2;
  const halfHeight = box.ellipse ? (box.ellipseHeight ?? box.height) / 2 : box.height / 2;
  if (side === "left") return { x: center.x - halfWidth, y: center.y };
  if (side === "right") return { x: center.x + halfWidth, y: center.y };
  if (side === "top") return { x: center.x, y: center.y - halfHeight };
  return { x: center.x, y: center.y + halfHeight };
}

function sampledSvgPath(path: string): SemPoint[] {
  const tokens = path.match(/[MLCQ]|[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?/gi) ?? [];
  const points: SemPoint[] = [];
  let index = 0;
  let current: SemPoint = { x: 0, y: 0 };
  const number = () => Number(tokens[index++]);
  while (index < tokens.length) {
    const command = tokens[index++]?.toUpperCase();
    if (command === "M" || command === "L") {
      current = { x: number(), y: number() };
      points.push(current);
      continue;
    }
    if (command === "C") {
      const start = current;
      const control1 = { x: number(), y: number() };
      const control2 = { x: number(), y: number() };
      const end = { x: number(), y: number() };
      for (let step = 1; step <= 24; step += 1) {
        const t = step / 24;
        const inverse = 1 - t;
        points.push({
          x: inverse ** 3 * start.x + 3 * inverse ** 2 * t * control1.x + 3 * inverse * t ** 2 * control2.x + t ** 3 * end.x,
          y: inverse ** 3 * start.y + 3 * inverse ** 2 * t * control1.y + 3 * inverse * t ** 2 * control2.y + t ** 3 * end.y,
        });
      }
      current = end;
      continue;
    }
    if (command === "Q") {
      const start = current;
      const control = { x: number(), y: number() };
      const end = { x: number(), y: number() };
      for (let step = 1; step <= 8; step += 1) {
        const t = step / 8;
        const inverse = 1 - t;
        points.push({
          x: inverse ** 2 * start.x + 2 * inverse * t * control.x + t ** 2 * end.x,
          y: inverse ** 2 * start.y + 2 * inverse * t * control.y + t ** 2 * end.y,
        });
      }
      current = end;
      continue;
    }
    return [];
  }
  return points;
}

/**
 * Mirrors the path SemEdge renders. Named cardinal handles remain the legacy
 * behavior; edges that opt into continuous perimeter routing use exact ellipse
 * or rectangle intersections without changing persisted handles or bends.
 */
export function renderedEdgePolyline(
  edge: Pick<Edge, "sourceHandle" | "targetHandle" | "data">,
  sourceNode: Pick<Node, "type" | "position">,
  targetNode: Pick<Node, "type" | "position">,
): SemPoint[] {
  const sourceBox = semNodeBox(sourceNode);
  const targetBox = semNodeBox(targetNode);
  const fallback = routeBetweenBoxes(sourceBox, targetBox);
  const sourceSide = sideFromHandleId(edge.sourceHandle);
  const targetSide = sideFromHandleId(edge.targetHandle);
  const rawBends = (edge.data as { bendPoints?: unknown } | undefined)?.bendPoints;
  const routing = String((edge.data as { routing?: unknown } | undefined)?.routing ?? "straight");
  const continuousPerimeter = (edge.data as { perimeterRouting?: unknown } | undefined)?.perimeterRouting === "continuous";
  const bends = Array.isArray(rawBends)
    ? rawBends.flatMap((point): SemPoint[] => {
        if (!point || typeof point !== "object") return [];
        const x = Number((point as { x?: unknown }).x);
        const y = Number((point as { y?: unknown }).y);
        return Number.isFinite(x) && Number.isFinite(y) ? [{ x, y }] : [];
      })
    : [];
  const continuousRoute = continuousPerimeter
    ? continuousBoundaryRoute(sourceBox, targetBox, routing === "polyline" ? bends : [])
    : fallback;
  const start = continuousPerimeter
    ? continuousRoute.start
    : sourceSide ? pointOnBoxSide(sourceBox, sourceSide) : fallback.start;
  const end = continuousPerimeter
    ? continuousRoute.end
    : targetSide ? pointOnBoxSide(targetBox, targetSide) : fallback.end;
  const sourcePosition = (continuousPerimeter ? continuousRoute.source : sourceSide ?? fallback.source) as Position;
  const targetPosition = (continuousPerimeter ? continuousRoute.target : targetSide ?? fallback.target) as Position;
  if (routing === "default") {
    const sampled = sampledSvgPath(getBezierPath({
      sourceX: start.x,
      sourceY: start.y,
      targetX: end.x,
      targetY: end.y,
      sourcePosition,
      targetPosition,
    })[0]);
    return sampled.length >= 2 ? sampled : [start, end];
  }
  if (routing === "smoothstep") {
    const sampled = sampledSvgPath(getSmoothStepPath({
      sourceX: start.x,
      sourceY: start.y,
      targetX: end.x,
      targetY: end.y,
      sourcePosition,
      targetPosition,
      borderRadius: 8,
    })[0]);
    return sampled.length >= 2 ? sampled : [start, end];
  }
  return [start, ...bends, end];
}

export function distance(left: SemPoint, right: SemPoint): number {
  return Math.hypot(right.x - left.x, right.y - left.y);
}

export function translatePoint(point: SemPoint, delta: SemPoint): SemPoint {
  return { x: point.x + delta.x, y: point.y + delta.y };
}

export function inflateSemRect(rect: SemRect, margin: number): SemRect {
  return {
    x: rect.x - margin,
    y: rect.y - margin,
    width: rect.width + margin * 2,
    height: rect.height + margin * 2,
  };
}

export function polylineMidpoint(points: readonly SemPoint[]): SemPoint {
  if (!points.length) return { x: 0, y: 0 };
  if (points.length === 1) return { ...points[0]! };
  const lengths = points.slice(1).map((point, index) => distance(points[index]!, point));
  const halfway = lengths.reduce((sum, length) => sum + length, 0) / 2;
  let consumed = 0;
  for (let index = 0; index < lengths.length; index += 1) {
    const length = lengths[index]!;
    if (consumed + length >= halfway && length > 0) {
      const start = points[index]!;
      const end = points[index + 1]!;
      const fraction = (halfway - consumed) / length;
      return {
        x: start.x + (end.x - start.x) * fraction,
        y: start.y + (end.y - start.y) * fraction,
      };
    }
    consumed += length;
  }
  return { ...points[points.length - 1]! };
}

export function pointAtPolylineFraction(points: readonly SemPoint[], requestedFraction: number): SemPoint {
  if (!points.length) return { x: 0, y: 0 };
  if (points.length === 1) return { ...points[0]! };
  const fraction = Math.min(1, Math.max(0, requestedFraction));
  const lengths = points.slice(1).map((point, index) => distance(points[index]!, point));
  const target = lengths.reduce((sum, length) => sum + length, 0) * fraction;
  let consumed = 0;
  for (let index = 0; index < lengths.length; index += 1) {
    const length = lengths[index]!;
    if (consumed + length >= target && length > 0) {
      const start = points[index]!;
      const end = points[index + 1]!;
      const local = (target - consumed) / length;
      return { x: start.x + (end.x - start.x) * local, y: start.y + (end.y - start.y) * local };
    }
    consumed += length;
  }
  return { ...points[points.length - 1]! };
}

export function fractionAlongSegment(start: SemPoint, end: SemPoint, point: SemPoint): number {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared < 1e-9) return 0.5;
  return Math.min(1, Math.max(0, ((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared));
}

export function fractionAlongPolyline(points: readonly SemPoint[], point: SemPoint): number {
  if (points.length < 2) return 0.5;
  const segmentLengths = points.slice(1).map((candidate, index) => distance(points[index]!, candidate));
  const total = segmentLengths.reduce((sum, length) => sum + length, 0);
  if (total < 1e-9) return 0.5;
  let bestDistance = Number.POSITIVE_INFINITY;
  let bestAlong = total / 2;
  let consumed = 0;
  for (let index = 0; index < segmentLengths.length; index += 1) {
    const start = points[index]!;
    const end = points[index + 1]!;
    const fraction = fractionAlongSegment(start, end, point);
    const projected = { x: start.x + (end.x - start.x) * fraction, y: start.y + (end.y - start.y) * fraction };
    const projectedDistance = distance(projected, point);
    if (projectedDistance < bestDistance) {
      bestDistance = projectedDistance;
      bestAlong = consumed + segmentLengths[index]! * fraction;
    }
    consumed += segmentLengths[index]!;
  }
  return Math.min(1, Math.max(0, bestAlong / total));
}

/**
 * Returns a deterministic two-bend detour only when the direct segment crosses
 * an obstacle. Callers exclude the source and target objects themselves.
 */
export function routePolylineAroundObstacles(
  start: SemPoint,
  end: SemPoint,
  obstacles: readonly SemRect[],
  clearance = 12,
): SemPoint[] {
  const inflated = obstacles.map((rect) => inflateSemRect(rect, clearance));
  const directBlockers = inflated.filter((rect) => segmentIntersectsRect(start, end, rect));
  if (!directBlockers.length) return [];
  const bounds = directBlockers.reduce((current, rect) => ({
    left: Math.min(current.left, rect.x),
    right: Math.max(current.right, rect.x + rect.width),
    top: Math.min(current.top, rect.y),
    bottom: Math.max(current.bottom, rect.y + rect.height),
  }), { left: Number.POSITIVE_INFINITY, right: Number.NEGATIVE_INFINITY, top: Number.POSITIVE_INFINITY, bottom: Number.NEGATIVE_INFINITY });
  const candidates: SemPoint[][] = [
    [{ x: start.x, y: bounds.top - clearance }, { x: end.x, y: bounds.top - clearance }],
    [{ x: start.x, y: bounds.bottom + clearance }, { x: end.x, y: bounds.bottom + clearance }],
    [{ x: bounds.left - clearance, y: start.y }, { x: bounds.left - clearance, y: end.y }],
    [{ x: bounds.right + clearance, y: start.y }, { x: bounds.right + clearance, y: end.y }],
  ];
  const score = (bends: readonly SemPoint[]) => {
    const points = [start, ...bends, end];
    const crossings = points.slice(1).reduce((sum, point, index) => sum + inflated.filter((rect) => segmentIntersectsRect(points[index]!, point, rect)).length, 0);
    const length = points.slice(1).reduce((sum, point, index) => sum + distance(points[index]!, point), 0);
    return crossings * 1_000_000 + length;
  };
  return candidates.sort((left, right) => score(left) - score(right))[0] ?? [];
}

export function semRectsOverlap(left: SemRect, right: SemRect): boolean {
  return left.x < right.x + right.width
    && left.x + left.width > right.x
    && left.y < right.y + right.height
    && left.y + left.height > right.y;
}

function segmentIntersectsRect(start: SemPoint, end: SemPoint, rect: SemRect): boolean {
  if (pointInsideRect(start, rect) || pointInsideRect(end, rect)) return true;
  const topLeft = { x: rect.x, y: rect.y };
  const topRight = { x: rect.x + rect.width, y: rect.y };
  const bottomRight = { x: rect.x + rect.width, y: rect.y + rect.height };
  const bottomLeft = { x: rect.x, y: rect.y + rect.height };
  return segmentsIntersect(start, end, topLeft, topRight)
    || segmentsIntersect(start, end, topRight, bottomRight)
    || segmentsIntersect(start, end, bottomRight, bottomLeft)
    || segmentsIntersect(start, end, bottomLeft, topLeft);
}

function pointInsideRect(point: SemPoint, rect: SemRect): boolean {
  return point.x > rect.x && point.x < rect.x + rect.width
    && point.y > rect.y && point.y < rect.y + rect.height;
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
