import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import {
  canonicalComparisonDisplayRowsV2,
  canonicalRunComparisonFromAnalysisRunsV2,
} from "./nativeCanonicalRunComparisonV2";

function run(id: string, coefficient: number): AnalysisRun {
  const source = completedSamplePlsRun();
  const result = structuredClone(source.result!);
  result.paths[0].coefficient = coefficient;
  return {
    ...source,
    id,
    name: id,
    result,
    modelId: "corporate-reputation-model",
    modelSnapshot: {
      nodes: [
        { id: "competence", position: { x: 0, y: 0 }, data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2"] } },
        { id: "satisfaction", position: { x: 200, y: 0 }, data: { label: "Satisfaction", shortName: "CUSA", mode: "reflective", indicators: ["CUSA1", "CUSA2"] } },
      ],
      edges: [{ id: "competence-satisfaction", source: "competence", target: "satisfaction" }],
    },
    provenance: {
      recipe_id: `recipe-${id}`,
      dataset_fingerprint: `sha256:${"a".repeat(64)}`,
      method: "pls_pm",
      method_version: result.method_version,
      engine_version: "qpls-estimation-test",
      seed: source.seed,
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: source.seed,
        workers: 1,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-14T09:00:00.000Z",
      completed_at: "2026-08-14T09:00:01.000Z",
    },
  };
}

describe("native canonical run comparison v2", () => {
  it("builds a typed cell-by-cell comparison for compatible saved runs", async () => {
    const built = await canonicalRunComparisonFromAnalysisRunsV2(run("first", 0.4), run("second", 0.5));
    expect(built.status).toBe("ready");
    if (built.status !== "ready") return;
    expect(built.comparison.summary.table_count).toBeGreaterThan(0);
    expect(built.comparison.summary.changed_cell_count).toBeGreaterThan(0);
    expect(canonicalComparisonDisplayRowsV2(built.comparison)).toContainEqual(expect.objectContaining({
      tableId: "direct_effects",
      tableTitle: "Direct effects",
      rowLabel: "Effect: Competence → Satisfaction",
      first: "0.4",
      second: "0.5",
      change: "0.1",
      changed: true,
    }));
  });

  it("blocks comparison when the dataset identity differs", async () => {
    const second = run("second", 0.5);
    second.provenance!.dataset_fingerprint = `sha256:${"b".repeat(64)}`;
    const built = await canonicalRunComparisonFromAnalysisRunsV2(run("first", 0.4), second);
    expect(built).toMatchObject({
      status: "blocked",
      issues: [expect.objectContaining({ code: "dataset_mismatch", title: "Data differs" })],
    });
  });

  it("does not compare historical text-only runs as typed values", async () => {
    const historical = run("historical", 0.4);
    historical.provenance = undefined;
    const built = await canonicalRunComparisonFromAnalysisRunsV2(historical, run("current", 0.4));
    expect(built).toMatchObject({
      status: "blocked",
      issues: [expect.objectContaining({ code: "first_result_historical_text" })],
    });
  });
});
