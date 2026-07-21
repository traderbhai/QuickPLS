import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import { boundaryPoint, measureDiagramQuality, routeBetweenBoxes, semNodeBox } from "./semGeometry";

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
