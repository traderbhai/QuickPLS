import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun, AnalysisUiSettings, ConstructData } from "../types";
import {
  buildNativeAnalysisRecipe,
  nativeAnalysisRecipeDescriptor,
} from "./nativeAnalysisRecipe";
import {
  nativeRunProvenanceTable,
  nativeRunSettingApplicability,
} from "./nativeExportTables";
import { buildNativeResultNavigation, nativeResultTables } from "./nativeResults";

const nodes: Node<ConstructData>[] = [
  {
    id: "x",
    type: "construct",
    position: { x: 100, y: 100 },
    data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] },
  },
  {
    id: "y",
    type: "construct",
    position: { x: 420, y: 100 },
    data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  },
];

const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

const settings = {
  method: "pls_pm",
  weightingScheme: "path",
  tolerance: 1e-7,
  maxIterations: 3_000,
  preprocessing: "standardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_813,
  workers: 1,
  confidenceLevel: 0.95,
  caseWeightColumn: null,
} as AnalysisUiSettings;

function completedAlgorithmRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    name: "PLS-SEM Algorithm run",
    method: "PLS-SEM Algorithm",
    assessment: undefined,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "11111111-1111-4111-8111-111111111111",
      dataset_fingerprint: "v2:pls-algorithm-factory",
      method: "pls_pm",
      method_version: "pls_pm_v1+pls_mediation_v1+pls_assessment_v7",
      engine_version: "2.46.0",
      seed: 20_260_813,
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20_260_813,
        workers: 1,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-13T08:00:00.000Z",
      completed_at: "2026-08-13T08:00:01.000Z",
    },
  };
}

describe("PLS Algorithm v1 factory-native contract", () => {
  it("serializes the exact deterministic PLS algorithm recipe without inference state", () => {
    expect(nativeAnalysisRecipeDescriptor("pls_algorithm")).toMatchObject({
      engineMethod: "pls_pm",
      label: "PLS-SEM Algorithm",
      scopeStatus: "validated",
      scopeMetadata: "validated_v1_0_supported_pls_scope",
    });

    const recipe = buildNativeAnalysisRecipe({
      kind: "pls_algorithm",
      recipeId: "11111111-1111-4111-8111-111111111111",
      modelId: "22222222-2222-4222-8222-222222222222",
      createdAt: "2026-08-13T08:00:00.000Z",
      datasetFingerprint: "v2:pls-algorithm-factory",
      projectName: "PLS Algorithm factory",
      nodes,
      edges,
      settings,
    });

    expect(recipe).toMatchObject({
      schema_version: 3,
      dataset_fingerprint: "v2:pls-algorithm-factory",
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        workers: 1,
      },
      method_config: { kind: "pls_algorithm" },
      metadata: { status: "validated_v1_0_supported_pls_scope" },
    });
    expect(recipe.model.constructs).toHaveLength(2);
    expect(recipe.model.paths).toEqual([{ source: "x", target: "y" }]);
    expect(recipe.model.controls).toEqual([]);
  });

  it("projects accessible point-estimate tables and same-run provenance without inference claims", () => {
    const run = completedAlgorithmRun();
    const tables = nativeResultTables(run);
    const titles = tables.map((table) => table.title);
    expect(titles).toEqual(expect.arrayContaining([
      "Direct effects",
      "Outer loadings",
      "Outer weights",
      "R-square",
      "Total effects",
    ]));
    expect(titles.some((title) => /Bootstrapping|Permutation/i.test(title))).toBe(false);
    for (const table of tables) {
      expect(table.columns.length).toBeGreaterThan(0);
      expect(table.rows.length).toBeGreaterThan(0);
      expect(table.status).toBe("validated");
    }

    const navigation = buildNativeResultNavigation(run);
    expect(navigation.runId).toBe(run.id);
    expect(navigation.groups.flatMap((group) => group.items).map((item) => item.title))
      .toEqual(expect.arrayContaining(["Direct effects", "Outer loadings", "R-square"]));

    const provenance = nativeRunProvenanceTable(run);
    const fields = Object.fromEntries(provenance.rows.map(([field, value]) => [field, value]));
    expect(fields).toMatchObject({
      Run: "PLS-SEM Algorithm run",
      Method: "PLS-SEM Algorithm",
      "Dataset fingerprint": "v2:pls-algorithm-factory",
      Recipe: "11111111-1111-4111-8111-111111111111",
      "Method version": "pls_pm_v1+pls_mediation_v1+pls_assessment_v7",
      "Weighting scheme": "path",
      Preprocessing: "standardized",
    });
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    expect(provenance.rows.some(([field]) => /Bootstrap|Permutation|Seed|Workers/i.test(field))).toBe(false);
  });
});
