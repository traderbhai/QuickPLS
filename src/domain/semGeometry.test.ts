import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import { boundaryPoint, fractionAlongPolyline, measureDiagramQuality, renderedEdgePolyline, routeBetweenBoxes, routePolylineAroundObstacles, semNodeBox } from "./semGeometry";

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

  it("uses named React Flow handles as the rendered polyline endpoints", () => {
    const source = { type: "latent", position: { x: 100, y: 100 } } as Node;
    const target = { type: "latent", position: { x: 400, y: 260 } } as Node;
    expect(renderedEdgePolyline({ sourceHandle: "source-right", targetHandle: "target-left", data: { bendPoints: [{ x: 300, y: 120 }] } }, source, target)).toEqual([
      { x: 204, y: 134 },
      { x: 300, y: 120 },
      { x: 400, y: 294 },
    ]);
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
