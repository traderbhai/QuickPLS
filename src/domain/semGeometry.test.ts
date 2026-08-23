import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import { boundaryPoint, continuousBoundaryRoute, fractionAlongPolyline, measureDiagramQuality, renderedEdgePolyline, routeBetweenBoxes, routePolylineAroundObstacles, semNodeBox } from "./semGeometry";

describe("SEM geometry", () => {
  it("returns true ellipse boundary points for cardinal directions", () => {
    const box = { x: 10, y: 20, width: 104, height: 94, kind: "latent" as const, ellipse: true, ellipseWidth: 104, ellipseHeight: 68 };
    const center = { x: 62, y: 54 };
    expect(boundaryPoint(box, { x: 200, y: center.y })).toEqual({ x: 114, y: 54 });
    expect(boundaryPoint(box, { x: -40, y: center.y })).toEqual({ x: 10, y: 54 });
    expect(boundaryPoint(box, { x: center.x, y: -40 })).toEqual({ x: 62, y: 20 });
    expect(boundaryPoint(box, { x: center.x, y: 200 })).toEqual({ x: 62, y: 88 });
  });

  it("returns nearest rectangle boundary points for indicators", () => {
    const box = { x: 100, y: 50, width: 88, height: 28, kind: "indicator" as const };
    expect(boundaryPoint(box, { x: 240, y: 64 })).toEqual({ x: 188, y: 64 });
    expect(boundaryPoint(box, { x: 144, y: 10 })).toEqual({ x: 144, y: 50 });
  });

  it("uses the compact visual footprint for moderation anchors", () => {
    expect(semNodeBox({ type: "moderationAnchor", position: { x: 40, y: 50 } } as Node)).toMatchObject({
      x: 40,
      y: 50,
      width: 22,
      height: 22,
      ellipse: true,
    });
  });

  it("routes border to border without entering latent interiors", () => {
    const source = semNodeBox({ type: "latent", position: { x: 100, y: 100 } } as Node);
    const target = semNodeBox({ type: "latent", position: { x: 380, y: 130 } } as Node);
    const route = routeBetweenBoxes(source, target);
    expect(route.source).toBe("right");
    expect(route.target).toBe("left");
    expect(route.start.x).toBeGreaterThan(150);
    expect(route.start.x).toBeLessThan(205);
    expect(route.end.x).toBeGreaterThanOrEqual(380);
    expect(route.end.x).toBeLessThan(430);
  });

  it.each([
    { targetPosition: { x: 340, y: 260 }, xDirection: 1, yDirection: 1 },
    { targetPosition: { x: -140, y: 260 }, xDirection: -1, yDirection: 1 },
    { targetPosition: { x: -140, y: -120 }, xDirection: -1, yDirection: -1 },
    { targetPosition: { x: 340, y: -120 }, xDirection: 1, yDirection: -1 },
  ])("uses exact continuous ellipse intersections in every diagonal quadrant", ({ targetPosition, xDirection, yDirection }) => {
    const source = semNodeBox({ type: "latent", position: { x: 100, y: 100 } } as Node);
    const target = semNodeBox({ type: "latent", position: targetPosition } as Node);
    const route = continuousBoundaryRoute(source, target);
    const sourceCenter = { x: 152, y: 134 };
    const targetCenter = { x: targetPosition.x + 52, y: targetPosition.y + 34 };

    expect(((route.start.x - sourceCenter.x) / 52) ** 2 + ((route.start.y - sourceCenter.y) / 34) ** 2).toBeCloseTo(1, 10);
    expect(((route.end.x - targetCenter.x) / 52) ** 2 + ((route.end.y - targetCenter.y) / 34) ** 2).toBeCloseTo(1, 10);
    expect(Math.sign(route.start.x - sourceCenter.x)).toBe(xDirection);
    expect(Math.sign(route.start.y - sourceCenter.y)).toBe(yDirection);
    expect(Math.sign(route.end.x - targetCenter.x)).toBe(-xDirection);
    expect(Math.sign(route.end.y - targetCenter.y)).toBe(-yDirection);
  });

  it("aims continuous polyline endpoints at their adjacent bends", () => {
    const sourceNode = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const targetNode = { type: "indicator", position: { x: 500, y: 250 } } as Node;
    const firstBend = { x: 210, y: 60 };
    const lastBend = { x: 450, y: 500 };
    const points = renderedEdgePolyline({
      sourceHandle: "source-right",
      targetHandle: "target-left",
      data: { perimeterRouting: "continuous", routing: "polyline", bendPoints: [firstBend, lastBend] },
    }, sourceNode, targetNode);
    const start = points[0]!;
    const end = points.at(-1)!;
    const sourceCenter = { x: 152, y: 134 };
    const targetCenter = { x: 544, y: 264 };

    expect(points).toHaveLength(4);
    expect(((start.x - sourceCenter.x) / 52) ** 2 + ((start.y - sourceCenter.y) / 34) ** 2).toBeCloseTo(1, 10);
    expect((start.x - sourceCenter.x) * (firstBend.y - sourceCenter.y)
      - (start.y - sourceCenter.y) * (firstBend.x - sourceCenter.x)).toBeCloseTo(0, 8);
    expect(end.y).toBeCloseTo(278, 10);
    expect((end.x - targetCenter.x) * (lastBend.y - targetCenter.y)
      - (end.y - targetCenter.y) * (lastBend.x - targetCenter.x)).toBeCloseTo(0, 8);
  });

  it.each(["straight", "default", "smoothstep"])("aims continuous %s routes at the opposite center", (routing) => {
    const sourceNode = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const targetNode = { type: "latent", position: { x: 400, y: 260 } } as Node;
    const expected = continuousBoundaryRoute(semNodeBox(sourceNode), semNodeBox(targetNode));
    const points = renderedEdgePolyline({
      sourceHandle: "source-right",
      targetHandle: "target-left",
      data: { perimeterRouting: "continuous", routing, bendPoints: [{ x: 152, y: 40 }] },
    }, sourceNode, targetNode);

    expect(points[0]!.x).toBeCloseTo(expected.start.x, 8);
    expect(points[0]!.y).toBeCloseTo(expected.start.y, 8);
    expect(points.at(-1)!.x).toBeCloseTo(expected.end.x, 8);
    expect(points.at(-1)!.y).toBeCloseTo(expected.end.y, 8);
  });

  it("keeps coincident and zero-size continuous routes finite", () => {
    const coincident = semNodeBox({ type: "latent", position: { x: 100, y: 100 } } as Node);
    const coincidentRoute = continuousBoundaryRoute(coincident, coincident, [{ x: Number.NaN, y: Number.POSITIVE_INFINITY }]);
    const zeroSize = { x: 20, y: 30, width: 0, height: 0, kind: "indicator" as const };
    const zeroSizeRoute = continuousBoundaryRoute(zeroSize, zeroSize);
    const zeroSizeEllipse = { ...zeroSize, kind: "latent" as const, ellipse: true };
    const zeroSizeEllipseRoute = continuousBoundaryRoute(zeroSizeEllipse, zeroSizeEllipse);

    expect([coincidentRoute.start.x, coincidentRoute.start.y, coincidentRoute.end.x, coincidentRoute.end.y, coincidentRoute.length].every(Number.isFinite)).toBe(true);
    expect(coincidentRoute.start).toEqual({ x: 204, y: 134 });
    expect(coincidentRoute.end).toEqual({ x: 100, y: 134 });
    expect([zeroSizeRoute.start.x, zeroSizeRoute.start.y, zeroSizeRoute.end.x, zeroSizeRoute.end.y, zeroSizeRoute.length].every(Number.isFinite)).toBe(true);
    expect(zeroSizeRoute.start).toEqual({ x: 20, y: 30 });
    expect(zeroSizeRoute.end).toEqual({ x: 20, y: 30 });
    expect([zeroSizeEllipseRoute.start.x, zeroSizeEllipseRoute.start.y, zeroSizeEllipseRoute.end.x, zeroSizeEllipseRoute.end.y, zeroSizeEllipseRoute.length].every(Number.isFinite)).toBe(true);
    expect(zeroSizeEllipseRoute.start).toEqual({ x: 20, y: 30 });
    expect(zeroSizeEllipseRoute.end).toEqual({ x: 20, y: 30 });
  });

  it("uses named React Flow handles as the rendered polyline endpoints", () => {
    const source = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const target = { type: "latent", position: { x: 400, y: 260 } } as Node;
    expect(renderedEdgePolyline({ sourceHandle: "source-right", targetHandle: "target-left", data: { bendPoints: [{ x: 300, y: 120 }] } }, source, target)).toEqual([
      { x: 204, y: 134 },
      { x: 300, y: 120 },
      { x: 400, y: 294 },
    ]);
  });

  it("lets flagged edges ignore cardinal handles for exact diagonal endpoints", () => {
    const source = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const target = { type: "latent", position: { x: 400, y: 260 } } as Node;
    const points = renderedEdgePolyline({
      sourceHandle: "source-right",
      targetHandle: "target-left",
      data: { perimeterRouting: "continuous", routing: "straight" },
    }, source, target);

    expect(points[0]).not.toEqual({ x: 204, y: 134 });
    expect(points.at(-1)).not.toEqual({ x: 400, y: 294 });
    expect(((points[0]!.x - 152) / 52) ** 2 + ((points[0]!.y - 134) / 34) ** 2).toBeCloseTo(1, 10);
    expect(((points.at(-1)!.x - 452) / 52) ** 2 + ((points.at(-1)!.y - 294) / 34) ** 2).toBeCloseTo(1, 10);
  });

  it("samples the same curved and orthogonal route shapes used by SemEdge", () => {
    const source = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const target = { type: "latent", position: { x: 400, y: 260 } } as Node;
    const curved = renderedEdgePolyline({ sourceHandle: "source-right", targetHandle: "target-left", data: { routing: "default" } }, source, target);
    const orthogonal = renderedEdgePolyline({ sourceHandle: "source-right", targetHandle: "target-left", data: { routing: "smoothstep" } }, source, target);

    expect(curved.length).toBe(25);
    expect(curved[0]).toEqual({ x: 204, y: 134 });
    expect(curved.at(-1)).toEqual({ x: 400, y: 294 });
    expect(orthogonal.length).toBeGreaterThan(4);
    expect(orthogonal[0]).toEqual({ x: 204, y: 134 });
    expect(orthogonal.at(-1)).toEqual({ x: 400, y: 294 });
  });

  it("detours around direct blockers without allowing a distant object to expand the route", () => {
    const bends = routePolylineAroundObstacles(
      { x: 0, y: 0 },
      { x: 220, y: 0 },
      [
        { x: 90, y: -18, width: 40, height: 36 },
        { x: 8_000, y: 6_000, width: 500, height: 500 },
      ],
      10,
    );
    expect(bends).toHaveLength(2);
    expect(Math.max(...bends.map((point) => Math.abs(point.x)))).toBeLessThan(500);
    expect(Math.max(...bends.map((point) => Math.abs(point.y)))).toBeLessThan(100);
  });

  it("projects a dragged anchor to the nearest fraction of a stored polyline", () => {
    expect(fractionAlongPolyline(
      [{ x: 0, y: 0 }, { x: 100, y: 0 }, { x: 100, y: 100 }],
      { x: 104, y: 50 },
    )).toBeCloseTo(0.75);
  });

  it("measures overlaps, crossings, and path lengths for diagram quality audits", () => {
    const nodes: Node[] = [
      { id: "a", type: "latent", position: { x: 0, y: 80 }, data: {} },
      { id: "b", type: "latent", position: { x: 0, y: 240 }, data: {} },
      { id: "overlap", type: "latent", position: { x: 0, y: 80 }, data: {} },
      { id: "c", type: "latent", position: { x: 300, y: 0 }, data: {} },
      { id: "d", type: "latent", position: { x: 300, y: 220 }, data: {} },
      { id: "a1", type: "indicator", position: { x: -160, y: 120 }, data: {} },
    ];
    const edges: Edge[] = [
      { id: "a-d", source: "a", target: "d", className: "structural-edge", data: { routing: "straight" } },
      { id: "b-c", source: "b", target: "c", className: "structural-edge", data: { routing: "straight" } },
      { id: "m", source: "a", target: "a1", className: "measurement-edge", data: { routing: "straight" } },
    ];
    const quality = measureDiagramQuality(nodes, edges);
    expect(quality.latentOverlapCount).toBe(1);
    expect(quality.pathCrossingCount).toBe(1);
    expect(quality.averageStructuralPathLength).toBeGreaterThan(100);
    expect(quality.averageMeasurementPathLength).toBeGreaterThan(0);
    expect(quality.unnecessaryBendCount).toBe(0);
  });
});
