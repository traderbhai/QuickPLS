import { describe, expect, it } from "vitest";
import { v255NamedSemEvidenceFixture, type V255NamedSemFixture } from "./v255NamedSemEvidenceFixtures";

const cases: V255NamedSemFixture[] = [
  "single_mediation", "parallel_mediation", "serial_mediation",
  "simultaneous_two_way", "three_way", "moderated_mediation_first",
  "moderated_mediation_second", "binary_moderation",
  "hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff", "cfa", "recursive_sem",
];

const minimumCovarianceCholeskyPivot = (values: number[][]) => {
  const means = values[0].map((_, column) => values.reduce((sum, row) => sum + row[column], 0) / values.length);
  const covariance = means.map((_, row) => means.map((__, column) => values.reduce(
    (sum, valuesRow) => sum + (valuesRow[row] - means[row]) * (valuesRow[column] - means[column]),
    0,
  ) / values.length));
  const lower = covariance.map((row) => row.map(() => 0));
  let minimumPivot = Number.POSITIVE_INFINITY;
  for (let row = 0; row < covariance.length; row += 1) {
    for (let column = 0; column <= row; column += 1) {
      const adjusted = covariance[row][column] - Array.from({ length: column }, (_, index) => lower[row][index] * lower[column][index])
        .reduce((sum, value) => sum + value, 0);
      if (row === column) {
        minimumPivot = Math.min(minimumPivot, adjusted);
        lower[row][column] = Math.sqrt(Math.max(0, adjusted));
      } else {
        lower[row][column] = adjusted / lower[column][column];
      }
    }
  }
  return minimumPivot;
};

describe("QuickPLS 2.55 query-gated named SEM evidence fixtures", () => {
  it.each(cases)("builds deterministic, result-free %s authority input", (fixture) => {
    const first = v255NamedSemEvidenceFixture(fixture);
    const second = v255NamedSemEvidenceFixture(fixture);
    expect(first).toEqual(second);
    expect(first.dataset.rowCount).toBe(360);
    expect(first.dataset.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u);
    expect(first.projectModels).toHaveLength(1);
    expect(first.activeModelId).toBe(first.modelId);
  });

  it("distinguishes mediation topology, moderation order, HOC type, and CB topology", () => {
    expect(v255NamedSemEvidenceFixture("single_mediation").edges.map((edge) => edge.id)).toEqual([
      "path:x-m1", "path:m1-y", "path:x-y",
    ]);
    expect(v255NamedSemEvidenceFixture("parallel_mediation").edges).toHaveLength(5);
    expect(v255NamedSemEvidenceFixture("serial_mediation").edges.map((edge) => edge.id)).toContain("path:m1-m2");
    expect(v255NamedSemEvidenceFixture("simultaneous_two_way").nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(2);
    const threeWayInteractions = v255NamedSemEvidenceFixture("three_way").nodes.filter((node) => node.data.semantic === "interaction");
    expect(threeWayInteractions).toHaveLength(4);
    expect(threeWayInteractions.find((node) => node.id === "w-z-y")?.data.interaction?.focalRelationId).toBe("path:w-y");
    expect(v255NamedSemEvidenceFixture("moderated_mediation_first").nodes.find((node) => node.data.semantic === "interaction")?.data.interaction?.focalRelationId).toBe("path:x-m1");
    expect(v255NamedSemEvidenceFixture("moderated_mediation_second").nodes.find((node) => node.data.semantic === "interaction")?.data.interaction?.focalRelationId).toBe("path:m1-y");
    const binary = v255NamedSemEvidenceFixture("binary_moderation");
    const binaryInteractions = binary.nodes.filter((node) => node.data.semantic === "interaction");
    expect(binaryInteractions.map((node) => node.data.interaction?.operands.length).sort()).toEqual([2, 2, 2, 3]);
    expect(binaryInteractions.find((node) => node.id === "w-b-y")?.data.interaction?.focalRelationId).toBe("path:w-y");
    expect(binary.edges.filter((edge) => !edge.data?.technicalGenerated).map((edge) => edge.id)).toEqual([
      "path:x-y", "path:w-y", "path:b-y",
    ]);
    expect(binary.dataset.columnMetadata.find((column) => column.name === "b")).toEqual({
      name: "b",
      label: null,
      column_type: "numeric",
      scale_type: "binary",
      missing_markers: [],
      theoretical_min: 0,
      theoretical_max: 1,
      value_labels: { "0": "Group 0", "1": "Group 1" },
    });
    expect(["hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff"].map((fixture) => v255NamedSemEvidenceFixture(fixture as V255NamedSemFixture).nodes.find((node) => node.data.semantic === "higher_order")?.data.higherOrder?.measurementType)).toEqual([
      "reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative",
    ]);
    expect(["hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff"].map((fixture) => {
      const nodes = v255NamedSemEvidenceFixture(fixture as V255NamedSemFixture).nodes;
      const hoc = nodes.find((node) => node.data.semantic === "higher_order");
      return [nodes.find((node) => node.id === "c1")?.data.mode, hoc?.data.mode];
    })).toEqual([
      ["reflective", "reflective"], ["reflective", "formative"],
      ["formative", "reflective"], ["formative", "formative"],
    ]);
    expect(v255NamedSemEvidenceFixture("cfa").edges).toHaveLength(0);
    expect(v255NamedSemEvidenceFixture("recursive_sem").edges).toHaveLength(2);
  });

  it("provides a strictly positive-definite manifest covariance for CB-SEM evidence", () => {
    const columns = ["x1", "x2", "x3", "m11", "m12", "m13", "y1", "y2", "y3"];
    const rows = v255NamedSemEvidenceFixture("cfa").dataset.rows.map((row) => columns.map((column) => Number(row[column])));
    expect(minimumCovarianceCholeskyPivot(rows)).toBeGreaterThan(1e-8);
  });
});
