import type { Edge, Node } from "@xyflow/react";
import type { ConstructData, Dataset, NativeCanonicalModelSpec } from "../types";
import { sha256HexUtf8V1 } from "../domain/sha256V1";

export type V255NamedSemFixture =
  | "single_mediation" | "parallel_mediation" | "serial_mediation"
  | "simultaneous_two_way" | "three_way" | "moderated_mediation_first"
  | "moderated_mediation_second" | "binary_moderation"
  | "hoc_rr" | "hoc_rf" | "hoc_fr" | "hoc_ff"
  | "cfa" | "recursive_sem";

const measured = (
  id: string,
  label: string,
  indicators: string[],
  position: { x: number; y: number },
  mode: ConstructData["mode"] = "reflective",
  factor = false,
): Node<ConstructData> => ({
  id,
  type: "construct",
  position,
  data: {
    label,
    shortName: id.toUpperCase(),
    mode,
    indicators,
    semModelV4: {
      version: 1,
      construct: factor
        ? { kind: "common_factor", marker_indicator: indicators[0] ?? null }
        : { kind: "composite" },
      ...(factor && indicators[0] ? { identification: { kind: "marker_loading" as const, indicator: indicators[0] } } : {}),
    },
  },
});

const interaction = (
  id: string,
  operands: [string, string, ...string[]],
  focalRelationId: string,
  outcome: string,
  y: number,
): Node<ConstructData> => ({
  id,
  type: "construct",
  position: { x: 360, y },
  data: {
    label: operands.map((operand) => operand.toUpperCase()).join(" × "),
    shortName: operands.map((operand) => operand.toUpperCase()).join("X"),
    mode: "formative",
    indicators: [],
    semantic: "interaction",
    interaction: {
      kind: "interaction_v2",
      termId: `term:${id}`,
      operands,
      focalRelationId,
      outcome,
      canonicalMethod: "two_stage",
      hierarchyPolicy: "strong",
      productIndicator: null,
    },
  },
});

const edge = (id: string, source: string, target: string, technicalGenerated = false): Edge => ({
  id, source, target, type: "smoothstep", ...(technicalGenerated ? { data: { technicalGenerated: true } } : {}),
});

const values = (index: number) => {
  const x = Math.sin(index * 0.071) + Math.cos(index * 0.037) * 0.29;
  const w = Math.cos(index * 0.059) - Math.sin(index * 0.023) * 0.31;
  const z = Math.sin(index * 0.043) + Math.cos(index * 0.031) * 0.27;
  const b = index % 2;
  const noise = Math.sin(index * 0.83) * 0.055 + Math.cos(index * 0.47) * 0.031;
  const m1 = 0.58 * x + 0.21 * w + noise;
  const m2 = 0.46 * x + 0.42 * m1 + 0.17 * z - noise * 0.2;
  const c1 = 0.67 * x + 0.25 * w + noise;
  const c2 = 0.61 * x + 0.29 * z - noise;
  const y = 0.24 * x + 0.35 * m1 + 0.31 * m2 + 0.16 * w + 0.13 * z
    + 0.18 * x * w + 0.15 * x * z + 0.11 * w * z + 0.14 * x * w * z + 0.12 * b * x + noise;
  return { x, w, z, b, m1, m2, c1, c2, y, noise };
};

const columns = ["x1", "x2", "x3", "m11", "m12", "m13", "m21", "m22", "w1", "w2", "z1", "z2", "b", "c11", "c12", "c21", "c22", "y1", "y2", "y3"];

const dataset = (): Dataset => {
  const rows = Array.from({ length: 360 }, (_, offset) => {
    const row = values(offset + 1);
    return {
      x1: row.x, x2: row.x * 0.91 + row.noise * 0.11, x3: row.x * 0.86 - row.noise * 0.09,
      m11: row.m1, m12: row.m1 * 0.92 - row.noise * 0.08, m13: row.m1 * 0.87 + row.noise * 0.12,
      m21: row.m2, m22: row.m2 * 0.90 + row.noise * 0.10,
      w1: row.w, w2: row.w * 0.89 - row.noise * 0.09,
      z1: row.z, z2: row.z * 0.90 + row.noise * 0.08,
      b: row.b,
      c11: row.c1, c12: row.c1 * 0.91 + row.noise * 0.09,
      c21: row.c2, c22: row.c2 * 0.92 - row.noise * 0.08,
      y1: row.y, y2: row.y * 0.93 + row.noise * 0.07, y3: row.y * 0.88 - row.noise * 0.10,
    };
  });
  return {
    id: "v255-named-sem-evidence",
    name: "QuickPLS 2.55 named SEM evidence fixture",
    columns,
    rows,
    rowCount: rows.length,
    missing: 0,
    fingerprint: `v2:${sha256HexUtf8V1(JSON.stringify({ columns, rows }))}`,
    kind: "raw",
    columnMetadata: columns.map((name) => ({
      name, label: null, column_type: "numeric" as const,
      scale_type: name === "b" ? "binary" as const : "continuous" as const,
      missing_markers: [], theoretical_min: null, theoretical_max: null,
      value_labels: name === "b" ? { "0": "Group 0", "1": "Group 1" } : {},
    })),
  };
};

const ordinaryNodes = (factor = false) => ({
  x: measured("x", "Predictor", factor ? ["x1", "x2", "x3"] : ["x1", "x2"], { x: 40, y: 220 }, "reflective", factor),
  m1: measured("m1", "Mediator 1", factor ? ["m11", "m12", "m13"] : ["m11", "m12"], { x: 310, y: 100 }, "reflective", factor),
  m2: measured("m2", "Mediator 2", ["m21", "m22"], { x: 310, y: 340 }, "reflective", factor),
  w: measured("w", "Moderator W", ["w1", "w2"], { x: 40, y: 430 }),
  z: measured("z", "Moderator Z", ["z1", "z2"], { x: 40, y: 590 }),
  b: measured("b", "Binary moderator", ["b"], { x: 40, y: 590 }),
  y: measured("y", "Outcome", factor ? ["y1", "y2", "y3"] : ["y1", "y2"], { x: 650, y: 220 }, "reflective", factor),
});

const canonicalModel = (id: string, nodes: Array<Node<ConstructData>>, edges: Edge[]): NativeCanonicalModelSpec => ({
  id,
  name: `Named evidence ${id}`,
  constructs: nodes.filter((node) => !node.data.semantic).map((node) => ({
    id: node.id, name: node.data.label, short_name: node.data.shortName,
    mode: node.data.mode, indicators: node.data.indicators,
  })),
  paths: edges.filter((candidate) => !candidate.data?.technicalGenerated).map((candidate) => ({ source: candidate.source, target: candidate.target })),
  controls: [],
  higher_order_constructs: nodes.flatMap((node) => node.data.semantic === "higher_order" && node.data.higherOrder ? [{
    id: node.id,
    components: [...node.data.higherOrder.components],
    method: "two_stage" as const,
    stage_one_recipe: node.data.higherOrder.stage_one_recipe ?? null,
  }] : []),
  interactions: nodes.flatMap((node) => {
    const value = node.data.interaction;
    if (node.data.semantic !== "interaction" || !value || value.kind !== "interaction_v2" || value.operands.length !== 2) return [];
    return [{
      id: value.termId,
      predictor: value.operands[0],
      moderator: value.operands[1],
      product_construct: node.id,
      outcome: value.outcome,
      method: "two_stage_product_score" as const,
    }];
  }),
});

export function v255NamedSemEvidenceFixture(fixture: V255NamedSemFixture) {
  const n = ordinaryNodes(fixture === "cfa" || fixture === "recursive_sem");
  let nodes: Array<Node<ConstructData>> = [];
  let edges: Edge[] = [];
  if (fixture === "single_mediation") {
    nodes = [n.x, n.m1, n.y]; edges = [edge("path:x-m1", "x", "m1"), edge("path:m1-y", "m1", "y"), edge("path:x-y", "x", "y")];
  } else if (fixture === "parallel_mediation") {
    nodes = [n.x, n.m1, n.m2, n.y]; edges = [edge("path:x-m1", "x", "m1"), edge("path:x-m2", "x", "m2"), edge("path:m1-y", "m1", "y"), edge("path:m2-y", "m2", "y"), edge("path:x-y", "x", "y")];
  } else if (fixture === "serial_mediation") {
    nodes = [n.x, n.m1, n.m2, n.y]; edges = [edge("path:x-m1", "x", "m1"), edge("path:m1-m2", "m1", "m2"), edge("path:m2-y", "m2", "y"), edge("path:x-y", "x", "y")];
  } else if (["simultaneous_two_way", "binary_moderation"].includes(fixture)) {
    const moderators = fixture === "binary_moderation" ? [n.b] : [n.w, n.z];
    const ids = fixture === "binary_moderation" ? ["b"] : ["w", "z"];
    const terms = ids.map((id, index) => interaction(`x-${id}-y`, ["x", id], "path:x-y", "y", 80 + index * 150));
    nodes = [n.x, ...moderators, n.y, ...terms];
    edges = [edge("path:x-y", "x", "y"), ...ids.map((id) => edge(`path:${id}-y`, id, "y")), ...terms.map((term) => edge(`path:${term.id}-y`, term.id, "y", true))];
  } else if (fixture === "three_way") {
    const terms = [
      interaction("x-w-y", ["x", "w"], "path:x-y", "y", 40), interaction("x-z-y", ["x", "z"], "path:x-y", "y", 150),
      interaction("w-z-y", ["w", "z"], "path:x-y", "y", 260), interaction("x-w-z-y", ["x", "w", "z"], "path:x-y", "y", 370),
    ];
    nodes = [n.x, n.w, n.z, n.y, ...terms];
    edges = [edge("path:x-y", "x", "y"), edge("path:w-y", "w", "y"), edge("path:z-y", "z", "y"), ...terms.map((term) => edge(`path:${term.id}-y`, term.id, "y", true))];
  } else if (fixture === "moderated_mediation_first" || fixture === "moderated_mediation_second") {
    const first = fixture.endsWith("first");
    const focal = first ? "path:x-m1" : "path:m1-y";
    const predictor = first ? "x" : "m1";
    const term = interaction(`${predictor}-w-${first ? "m1" : "y"}`, [predictor, "w"], focal, first ? "m1" : "y", 440);
    nodes = [n.x, n.m1, n.w, n.y, term];
    edges = [edge("path:x-m1", "x", "m1"), edge("path:m1-y", "m1", "y"), edge("path:x-y", "x", "y"), edge(`path:w-${first ? "m1" : "y"}`, "w", first ? "m1" : "y"), edge(`path:${term.id}-${first ? "m1" : "y"}`, term.id, first ? "m1" : "y", true)];
  } else if (fixture.startsWith("hoc_")) {
    const type = fixture.slice(4).toUpperCase();
    const componentMode: ConstructData["mode"] = type.charAt(1) === "F" ? "formative" : "reflective";
    const hocMode: ConstructData["mode"] = type.charAt(0) === "F" ? "formative" : "reflective";
    const c1 = measured("c1", "Component 1", ["c11", "c12"], { x: 40, y: 120 }, componentMode);
    const c2 = measured("c2", "Component 2", ["c21", "c22"], { x: 40, y: 380 }, componentMode);
    const hoc: Node<ConstructData> = { id: "hoc", type: "construct", position: { x: 350, y: 250 }, data: { label: `Service Quality ${type}`, shortName: `SQ${type}`, mode: hocMode, indicators: [], semantic: "higher_order", higherOrder: { id: `hoc:${type.toLowerCase()}`, components: ["c1", "c2"], method: "two_stage", canonicalApproach: "disjoint_two_stage", measurementType: ({ RR: "reflective_reflective", RF: "reflective_formative", FR: "formative_reflective", FF: "formative_formative" } as const)[type as "RR" | "RF" | "FR" | "FF"], stage_one_recipe: null } } };
    nodes = [c1, c2, hoc, n.y]; edges = [edge("path:hoc-y", "hoc", "y")];
  } else if (fixture === "cfa") {
    nodes = [n.x, n.m1, n.y]; edges = [];
  } else {
    nodes = [n.x, n.m1, n.y]; edges = [edge("path:x-m1", "x", "m1"), edge("path:m1-y", "m1", "y")];
  }
  const modelId = `v255-${fixture}`;
  const model = canonicalModel(modelId, nodes, edges);
  return { nodes, edges, dataset: dataset(), projectModels: [model], activeModelId: modelId, fixture, modelId };
}
