import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { nativeIpmaPredecessorIds, nativeIpmaTargetOptions } from "./nativeIpma";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
  { id: "m", position: { x: 200, y: 0 }, data: { label: "Experience", shortName: "EXP", mode: "reflective", indicators: ["m1"] } },
  { id: "y", position: { x: 400, y: 0 }, data: { label: "Retention", shortName: "RET", mode: "reflective", indicators: ["y1"] } },
  { id: "u", position: { x: 0, y: 200 }, data: { label: "Unrelated", shortName: "UNR", mode: "reflective", indicators: ["u1"] } },
  { id: "v", position: { x: 200, y: 200 }, data: { label: "Retention", shortName: "ALT", mode: "reflective", indicators: ["v1"] } },
];

const edges: Edge[] = [
  { id: "x-m", source: "x", target: "m" },
  { id: "m-y", source: "m", target: "y" },
  { id: "u-v", source: "u", target: "v" },
  { id: "control-x-y", source: "x", target: "y", data: { role: "control" } },
  { id: "measurement::y::y1", source: "y", target: "y1" },
];

describe("native IPMA model identities", () => {
  it("offers only endogenous immutable IDs with visible model labels", () => {
    expect(nativeIpmaTargetOptions(nodes, edges)).toEqual([
      { id: "m", label: "Experience", optionLabel: "Experience [m]" },
      { id: "y", label: "Retention", optionLabel: "Retention [y]" },
      { id: "v", label: "Retention", optionLabel: "Retention [v]" },
    ]);
  });

  it("returns direct and indirect structural predecessors while excluding controls, self, and unrelated branches", () => {
    expect([...nativeIpmaPredecessorIds(edges, "y")]).toEqual(["m", "x"]);
    expect(nativeIpmaPredecessorIds(edges, "y")).not.toContain("y");
    expect(nativeIpmaPredecessorIds(edges, "y")).not.toContain("u");
    expect(nativeIpmaPredecessorIds(edges, "y")).not.toContain("v");
  });
});
