import { describe, expect, it } from "vitest";
import type { DiagramLayoutState, DiagramViewport } from "../types";
import {
  modelCanvasInitialViewportPlan,
  shouldAutoFitModelCanvasAfterNodeGrowth,
} from "./ModelCanvas";

const persistedViewport: DiagramViewport = { x: 258, y: 301, zoom: 1 };

describe("ModelCanvas persisted viewport restoration", () => {
  it("remounts a strict saved Canvas without replacing its presentation authority", () => {
    const savedLayout = {
      diagramVersion: "sem_designer_v1",
      constructLayouts: {},
      indicatorLayouts: {},
      measurementConnectorLayouts: {},
      edgeLayouts: {},
      diagramViewport: persistedViewport,
      diagramTheme: "smartpls_like",
      showGrid: true,
      layoutLocked: false,
    } satisfies DiagramLayoutState;
    const plan = modelCanvasInitialViewportPlan(savedLayout.diagramViewport);

    expect(plan).toEqual({ defaultViewport: persistedViewport, fitOnInit: false });
    expect(shouldAutoFitModelCanvasAfterNodeGrowth({
      strictAuthority: true,
      persistedViewport: savedLayout.diagramViewport,
      preserveViewportForDrop: false,
    })).toBe(false);
    expect({ ...savedLayout, diagramViewport: plan.defaultViewport }).toEqual(savedLayout);
  });

  it("retains automatic fitting when no viewport authority exists", () => {
    expect(modelCanvasInitialViewportPlan(undefined)).toEqual({
      defaultViewport: undefined,
      fitOnInit: true,
    });
    expect(shouldAutoFitModelCanvasAfterNodeGrowth({
      strictAuthority: true,
      persistedViewport: undefined,
      preserveViewportForDrop: false,
    })).toBe(true);
  });
});
