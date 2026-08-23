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

    expect(ids).toHaveLength(7);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID);
    for (const preservedId of [
      "corporate_reputation",
      "simple_pls",
      "mediation",
      "organizational_identification",
      "organizational_identification_mediation",
      "organizational_identification_moderated_mediation",
      "organizational_identification_higher_order",
    ]) {
      expect(parseNativeSampleProjectId(preservedId)).toBe(preservedId);
    }
  });

  it("accepts only IDs advertised by the validated catalog", () => {
    expect(parseNativeSampleProjectId(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID))
      .toBe(DEFAULT_BUNDLED_SAMPLE_PROJECT_ID);
    expect(parseNativeSampleProjectId("organizational_identification_moderation")).toBeNull();
    expect(parseNativeSampleProjectId("not_bundled")).toBeNull();
    expect(parseNativeSampleProjectId(null)).toBeNull();
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
  });
});
