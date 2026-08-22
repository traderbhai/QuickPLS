import { describe, expect, it } from "vitest";
import { v255NamedSemEvidenceFixture, type V255NamedSemFixture } from "./v255NamedSemEvidenceFixtures";

const cases: V255NamedSemFixture[] = [
  "single_mediation", "parallel_mediation", "serial_mediation",
  "simultaneous_two_way", "three_way", "moderated_mediation_first",
  "moderated_mediation_second", "binary_moderation",
  "hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff", "cfa", "recursive_sem",
];

describe("QuickPLS 2.55 query-gated named SEM evidence fixtures", () => {
  it.each(cases)("builds deterministic, result-free %s authority input", (fixture) => {
    const first = v255NamedSemEvidenceFixture(fixture);
    const second = v255NamedSemEvidenceFixture(fixture);
    expect(first).toEqual(second);
    expect(first.dataset.rowCount).toBe(360);
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
    expect(v255NamedSemEvidenceFixture("three_way").nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(4);
    expect(v255NamedSemEvidenceFixture("moderated_mediation_first").nodes.find((node) => node.data.semantic === "interaction")?.data.interaction?.focalRelationId).toBe("path:x-m1");
    expect(v255NamedSemEvidenceFixture("moderated_mediation_second").nodes.find((node) => node.data.semantic === "interaction")?.data.interaction?.focalRelationId).toBe("path:m1-y");
    expect(v255NamedSemEvidenceFixture("binary_moderation").dataset.columnMetadata.find((column) => column.name === "b")?.scale_type).toBe("binary");
    expect(["hoc_rr", "hoc_rf", "hoc_fr", "hoc_ff"].map((fixture) => v255NamedSemEvidenceFixture(fixture as V255NamedSemFixture).nodes.find((node) => node.data.semantic === "higher_order")?.data.higherOrder?.measurementType)).toEqual([
      "reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative",
    ]);
    expect(v255NamedSemEvidenceFixture("cfa").edges).toHaveLength(0);
    expect(v255NamedSemEvidenceFixture("recursive_sem").edges).toHaveLength(2);
  });
});
