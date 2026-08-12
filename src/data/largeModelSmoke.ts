import type { Edge, Node } from "@xyflow/react";
import type { ConstructData, Dataset } from "../types";

/** Deterministic interaction fixture; loaded only by the visual acceptance query. */
export function largeModelSmokeProject(): {
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
} {
  const constructCount = 20;
  const indicatorsPerConstruct = 4;
  const columns = Array.from({ length: constructCount * indicatorsPerConstruct }, (_, index) => {
    const construct = Math.floor(index / indicatorsPerConstruct) + 1;
    const indicator = index % indicatorsPerConstruct + 1;
    return `C${String(construct).padStart(2, "0")}_${indicator}`;
  });
  const nodes: Array<Node<ConstructData>> = Array.from({ length: constructCount }, (_, index) => {
    const number = index + 1;
    return {
      id: `construct-${number}`,
      type: "construct",
      position: { x: (index % 5) * 260 + 80, y: Math.floor(index / 5) * 190 + 70 },
      data: {
        label: `Construct ${number}`,
        shortName: `C${String(number).padStart(2, "0")}`,
        mode: "reflective",
        indicators: columns.slice(index * indicatorsPerConstruct, (index + 1) * indicatorsPerConstruct),
      },
    };
  });
  const edges: Edge[] = Array.from({ length: constructCount - 1 }, (_, index) => ({
    id: `path-${index + 1}-${index + 2}`,
    source: `construct-${index + 1}`,
    target: `construct-${index + 2}`,
    type: "smoothstep",
    label: "Path",
  }));
  const rows = Array.from({ length: 200 }, (_, rowIndex) => Object.fromEntries(
    columns.map((column, columnIndex) => [column, ((rowIndex * 7 + columnIndex * 3) % 97) / 10 + 1]),
  ));
  return {
    dataset: { id: "large-model-smoke", name: "Large model acceptance fixture", kind: "raw", columns, rows, missing: 0, rowCount: rows.length, sampleSize: rows.length, fingerprint: "quickpls-large-model-smoke-v1" },
    nodes,
    edges,
  };
}
