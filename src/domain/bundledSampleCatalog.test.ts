import { describe, expect, it } from "vitest";
import {
  BUNDLED_SAMPLE_PROJECTS,
  DEFAULT_BUNDLED_SAMPLE_PROJECT_ID,
  parseBundledSampleCatalog,
  parseNativeSampleProjectId,
} from "./bundledSampleCatalog";

describe("bundled sample project catalog", () => {
  it("publishes a unique, non-empty catalog with a listed default", () => {
    const ids = BUNDLED_SAMPLE_PROJECTS.map((sample) => sample.id);

    expect(ids).toHaveLength(8);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID);
    for (const preservedId of [
      "corporate_reputation",
      "simple_pls",
      "mediation",
      "organizational_identification",
      "organizational_identification_mediation",
      "organizational_identification_moderation",
      "organizational_identification_moderated_mediation",
      "organizational_identification_higher_order",
    ]) {
      expect(parseNativeSampleProjectId(preservedId)).toBe(preservedId);
    }
  });

  it("accepts only IDs advertised by the validated catalog", () => {
    expect(parseNativeSampleProjectId(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID))
      .toBe(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID);
    expect(parseNativeSampleProjectId("organizational_identification_moderation"))
      .toBe("organizational_identification_moderation");
    expect(parseNativeSampleProjectId("not_bundled")).toBeNull();
    expect(parseNativeSampleProjectId(null)).toBeNull();
  });

  it("distinguishes strict General SEM samples from ordinary editable projects", () => {
    expect(BUNDLED_SAMPLE_PROJECTS.find((sample) => sample.id === "organizational_identification_moderation"))
      .toMatchObject({ projectKind: "general_sem_v1" });
    expect(BUNDLED_SAMPLE_PROJECTS.filter((sample) => sample.projectKind === "ordinary_v1"))
      .toHaveLength(7);
  });

  it("rejects malformed, duplicate, and dangling-default catalogs", () => {
    expect(() => parseBundledSampleCatalog(null)).toThrow("must be an object");
    expect(() => parseBundledSampleCatalog({
      schemaVersion: 1,
      defaultSampleId: "sample_a",
      datasets: [],
      samples: [
        { id: "sample_a", label: "Sample A", detail: "First" },
        { id: "sample_a", label: "Sample B", detail: "Second" },
      ],
    })).toThrow("duplicate sample id");
    expect(() => parseBundledSampleCatalog({
      schemaVersion: 1,
      defaultSampleId: "missing",
      datasets: [],
      samples: [{ id: "sample_a", label: "Sample A", detail: "First" }],
    })).toThrow("must identify a listed sample");
    expect(() => parseBundledSampleCatalog({
      schemaVersion: 1,
      defaultSampleId: "sample_a",
      datasets: [],
      samples: [{ id: "sample_a", label: "Sample A", detail: "First", projectKind: "future_v2" }],
    })).toThrow("must be ordinary_v1 or general_sem_v1");
  });
});
