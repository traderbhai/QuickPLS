import { applyNodeChanges, type Edge, type Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { focusedConstructSelectionChanges, measurementConnectorSelectionForEdge } from "./ModelCanvas";

const construct = (id: string, selected: boolean): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  selected,
  data: {
    label: id.toUpperCase(),
    shortName: id.toUpperCase(),
    mode: "reflective",
    indicators: [`${id}_1`],
  },
});

describe("ModelCanvas construct focus selection", () => {
  it("replaces a stale Canvas selection before React Flow Ctrl-selection extends it", () => {
    const nodes = [construct("x", false), construct("y", false), construct("z", true)];
    const changes = focusedConstructSelectionChanges(nodes, "x");

    expect(changes).toEqual([
      { type: "select", id: "x", selected: true },
      { type: "select", id: "z", selected: false },
    ]);
    expect(applyNodeChanges(changes, nodes).filter((node) => node.selected).map((node) => node.id)).toEqual(["x"]);
  });

  it("resolves reflective and formative measurement edges to the same connector selection", () => {
    const reflective: Edge = {
      id: "measurement::quality::qual 1",
      source: "quality",
      target: "indicator::quality::qual%201",
    };
    const formative: Edge = {
      id: "measurement::quality::qual 1",
      source: "indicator::quality::qual%201",
      target: "quality",
    };

    expect(measurementConnectorSelectionForEdge(reflective)).toEqual({ constructId: "quality", indicator: "qual 1" });
    expect(measurementConnectorSelectionForEdge(formative)).toEqual({ constructId: "quality", indicator: "qual 1" });
    expect(measurementConnectorSelectionForEdge({ id: "path:quality-loyalty", source: "quality", target: "loyalty" })).toBeNull();
  });

  it("prefers structured connector identity when construct and indicator ids contain delimiters", () => {
    const structured: Edge = {
      id: "measurement::stale::identity",
      source: "construct::quality",
      target: "indicator::construct%3A%3Aquality::qual%3A%3A1",
      data: {
        visualOnly: true,
        relationshipKind: "measurement_connector",
        constructId: "construct::quality",
        indicator: "qual::1",
        routeEditable: true,
        perimeterRouting: "continuous",
        routing: "straight",
        edgeClassName: "measurement-edge reflective",
      },
    };
    const encodedFallback: Edge = {
      id: "measurement::construct%3A%3Aquality::qual%3A%3A1",
      source: "construct::quality",
      target: "indicator::construct%3A%3Aquality::qual%3A%3A1",
    };

    expect(measurementConnectorSelectionForEdge(structured)).toEqual({ constructId: "construct::quality", indicator: "qual::1" });
    expect(measurementConnectorSelectionForEdge(encodedFallback)).toEqual({ constructId: "construct::quality", indicator: "qual::1" });
    expect(measurementConnectorSelectionForEdge({
      id: "measurement::ambiguous::construct::qual::1",
      source: "ambiguous::construct",
      target: "not-an-indicator-node",
    })).toBeNull();
  });
});
