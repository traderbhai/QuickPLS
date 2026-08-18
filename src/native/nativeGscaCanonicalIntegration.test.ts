import { describe, expect, it } from "vitest";
import { tablesToCsv } from "../domain/resultTables";
import type { AnalysisResultEnvelope, NativeCanonicalAnalysisRecipe } from "../types";
import { nativeRunFromCanonicalResult } from "./nativeCanonicalProject";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { completedGscaRun } from "./nativeGsca.testFixture";
import { buildNativeResultNavigation, nativeGscaResultProjection } from "./nativeResults";

describe("native canonical GSCA ALS v2 release gate", () => {
  it("rehydrates the exact model-bound result with labels, tables, and provenance intact", () => {
    const source = completedGscaRun();
    const engineScopeWarning = "GSCA ALS v2 is bounded to standardized raw data, listwise deletion, disjoint reflective/formative blocks, and recursive single-group structural models; inference and broader GSCA variants are not included.";
    expect(source.result?.warnings).toEqual([engineScopeWarning]);
    expect(source.result?.gsca?.warnings).toEqual([engineScopeWarning]);
    const recipe: NativeCanonicalAnalysisRecipe = {
      schema_version: 2,
      id: source.provenance!.recipe_id,
      created_at: source.provenance!.started_at,
      dataset_fingerprint: source.provenance!.dataset_fingerprint,
      model: {
        id: source.modelId!,
        name: "GSCA mixed-block model",
        constructs: [
          { id: "g", name: "Formative Capability", short_name: "G", mode: "formative", indicators: ["g1", "g2", "g3"] },
          { id: "h", name: "Reflective Outcome", short_name: "H", mode: "reflective", indicators: ["h1", "h2"] },
        ],
        paths: [{ source: "g", target: "h" }],
        controls: [],
        interactions: [],
        higher_order_constructs: [],
      },
      settings: source.provenance!.settings,
      metadata: { status: "validated_gsca_als_v2_bounded_scope" },
    };
    const envelope: AnalysisResultEnvelope = {
      schema_version: 1,
      id: source.id,
      status: "completed",
      provenance: source.provenance!,
      diagnostics: source.warnings.map((message) => ({ level: "warning", code: "gsca.scope", message })),
      payload: {
        kind: "pls_pm_v1",
        estimation: source.result!,
        assessment: source.assessment!,
      },
    };

    const reopened = nativeRunFromCanonicalResult(envelope, recipe);
    expect(reopened).not.toBeNull();
    if (!reopened) throw new Error("Expected canonical GSCA result to reopen.");
    expect(reopened).toMatchObject({
      id: source.id,
      modelId: source.modelId,
      method: "GSCA",
      name: "GSCA run",
      provenance: { method: "gsca", method_version: "gsca_als_v2" },
    });
    expect(reopened.modelSnapshot?.nodes.map((node) => [node.id, node.data.label, node.data.mode])).toEqual([
      ["g", "Formative Capability", "formative"],
      ["h", "Reflective Outcome", "reflective"],
    ]);
    expect(nativeGscaResultProjection(reopened)).not.toBeNull();

    const navigation = buildNativeResultNavigation(reopened);
    expect(navigation.defaultItemId).toBe("gsca_fit");
    expect(navigation.tables.find((table) => table.id === "gsca_paths")?.rows).toEqual([
      ["Reflective Outcome ← Formative Capability", "0.770947"],
    ]);
    const exportTables = [...navigation.tables, nativeRunProvenanceTable(reopened)];
    expect(exportTables.map((table) => table.id)).toEqual([
      "gsca_fit",
      "gsca_paths",
      "gsca_r_squared",
      "gsca_loadings",
      "gsca_weights",
      "gsca_scope",
      "run_provenance",
    ]);
    const csv = tablesToCsv(exportTables);
    expect(csv).toContain("gsca_als_v2");
    expect(csv).toContain("alternating_least_squares_v1");
    expect(csv).toContain("Formative Capability");
    expect(csv).not.toMatch(/\bN\/?A\b|bootstrap interval|Confidence level|Workers/i);
  });
});
