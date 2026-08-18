import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import { methodResultTables, runExportTables, tablesToCsv, tablesToHtml } from "../domain/resultTables";
import type { AnalysisRun, AnalysisUiSettings, ConstructData } from "../types";
import {
  NativeAnalysisRecipeBuildError,
  buildNativeAnalysisRecipe,
  type NativeAnalysisRecipeBuildInput,
} from "./nativeAnalysisRecipe";
import { nativeResultTables } from "./nativeResults";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 320, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const edges: Edge[] = [{ id: "path-x-y", source: "x", target: "y" }];
const settings: AnalysisUiSettings = {
  method: "endogeneity",
  weightingScheme: "path",
  tolerance: 1e-7,
  maxIterations: 3_000,
  preprocessing: "standardized",
  bootstrapSamples: 5_000,
  studentizedInnerSamples: 99,
  permutationSamples: 999,
  seed: 20_260_814,
  workers: 4,
  confidenceLevel: 0.95,
};

function recipeInput(overrides: Partial<Pick<NativeAnalysisRecipeBuildInput, "nodes" | "edges">> = {}, settingPatch: Partial<AnalysisUiSettings> = {}): NativeAnalysisRecipeBuildInput {
  return {
    kind: "endogeneity",
    recipeId: "11111111-1111-4111-8111-111111111111",
    modelId: "22222222-2222-4222-8222-222222222222",
    createdAt: "2026-08-14T00:00:00Z",
    datasetFingerprint: "sha256:endogeneity-fixture",
    projectName: "Gaussian-copula fixture",
    nodes: overrides.nodes ?? nodes,
    edges: overrides.edges ?? edges,
    settings: { ...settings, ...settingPatch },
  };
}

function expectModelError(input: NativeAnalysisRecipeBuildInput) {
  expect(() => buildNativeAnalysisRecipe(input)).toThrowError(NativeAnalysisRecipeBuildError);
  try {
    buildNativeAnalysisRecipe(input);
  } catch (error) {
    expect((error as NativeAnalysisRecipeBuildError).field).toBe("model");
  }
}

function completedEndogeneityRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  const scopeWarning = "Gaussian-copula endogeneity diagnostics are validated for the documented QuickPLS v1.2.3 diagnostic scope and assume nonnormal predictor scores; use as a diagnostic, not proof of causality.";
  const weakWarning = "Predictor score skewness is below the experimental applicability threshold; Gaussian-copula evidence is weak for near-normal predictors.";
  return {
    ...base,
    id: "endogeneity-run",
    name: "Gaussian-copula endogeneity",
    method: "Gaussian-Copula Endogeneity",
    fingerprint: "sha256:endogeneity-fixture",
    provenance: {
      recipe_id: "recipe-endogeneity",
      dataset_fingerprint: "sha256:endogeneity-fixture",
      method: "endogeneity",
      method_version: "pls_pm_v1+gaussian_copula_endogeneity_v1+pls_mediation_v1+pls_assessment_v7",
      engine_version: "2.46.0",
      seed: 20_260_814,
      settings: {
        method: "endogeneity",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20_260_814,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-14T00:00:00Z",
      completed_at: "2026-08-14T00:00:01Z",
    },
    result: {
      ...base.result!,
      method_version: "gaussian_copula_endogeneity_v1",
      endogeneity: {
        method_version: "gaussian_copula_endogeneity_v1",
        transform: "rankit_inverse_normal_v1",
        estimates: [
          {
            source: "competence",
            target: "satisfaction",
            path_coefficient: 0.431245,
            copula_coefficient: -0.118765,
            standard_error: 0.051,
            t_statistic: -2.3287,
            p_value_two_sided: 0.0199,
            predictor_skewness: 0.82,
            applicable: true,
            warning: null,
          },
          {
            source: "likeability",
            target: "satisfaction",
            path_coefficient: 0.276543,
            copula_coefficient: 0.011234,
            standard_error: 0.064,
            t_statistic: 0.1755,
            p_value_two_sided: 0.8607,
            predictor_skewness: 0.18,
            applicable: false,
            warning: weakWarning,
          },
        ],
        warnings: [
          "likeability -> satisfaction has near-symmetric predictor scores; interpret Gaussian-copula diagnostics cautiously",
          scopeWarning,
        ],
      },
    },
  };
}

describe("bounded native Gaussian-copula workflow", () => {
  it("serializes one typed deterministic recipe and clears unrelated resampling", () => {
    const recipe = buildNativeAnalysisRecipe(recipeInput());
    expect(recipe.method_config).toEqual({ kind: "endogeneity" });
    expect(recipe.settings).toMatchObject({
      method: "endogeneity",
      weighting_scheme: "path",
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      seed: 20_260_814,
      workers: 4,
    });
    expect(recipe.metadata).toEqual({ status: "validated_v1_2_3_endogeneity_bounded_scope" });
  });

  it("fails closed on PCA weighting, missing paths, controls, interactions, and higher-order constructs", () => {
    expect(() => buildNativeAnalysisRecipe(recipeInput({}, { weightingScheme: "pca" }))).toThrowError(/path or factor weighting/i);
    expectModelError(recipeInput({ edges: [] }));
    expectModelError(recipeInput({ edges: [{ id: "control-x-y", source: "x", target: "y", data: { role: "control" } }] }));
    expectModelError(recipeInput({
      nodes: nodes.map((node) => node.id === "x" ? {
        ...node,
        data: { ...node.data, semantic: "higher_order" as const, higherOrder: { id: "x", components: ["y"], method: "two_stage" as const } },
      } : node),
    }));
    expectModelError(recipeInput({
      nodes: [...nodes, {
        id: "x-by-x",
        position: { x: 160, y: 180 },
        data: {
          label: "Interaction",
          shortName: "XX",
          mode: "reflective" as const,
          indicators: [],
          semantic: "interaction" as const,
          interaction: { predictor: "x", moderator: "x", outcome: "y", method: "two_stage_product_score" as const },
        },
      }],
    }));
  });

  it("uses one completed run for accessible results and CSV/HTML/XLSX input tables", () => {
    const run = completedEndogeneityRun();
    const domainTable = methodResultTables(run.result!).find((table) => table.id === "endogeneity_copula");
    const nativeTable = nativeResultTables(run).find((table) => table.id === "endogeneity_copula");
    const exportTables = runExportTables(run);
    const exportTable = exportTables.find((table) => table.id === "endogeneity_copula");

    expect(domainTable).toEqual(nativeTable);
    expect(exportTable).toEqual(domainTable);
    expect(exportTable).toMatchObject({
      status: "validated",
      columns: ["Source", "Target", "Path coefficient", "Copula coefficient", "t statistic", "p value", "Predictor skewness", "Applicability", "Warning"],
    });
    expect(exportTable?.rows).toHaveLength(2);
    expect(exportTable?.rows[0]).toContain("screenable");
    expect(exportTable?.rows[1]).toContain("weak");
    expect(tablesToCsv(exportTables)).toContain("Gaussian-copula endogeneity diagnostics");
    expect(tablesToCsv(exportTables)).toContain("sha256:endogeneity-fixture");
    expect(tablesToHtml(exportTables)).toContain("not proof of causality");

    // NativeExportDialog passes this exact table array to the XLSX command;
    // keeping the method table identical across surfaces prevents recomputation.
    expect(exportTables.find((table) => table.id === "run_provenance")?.rows).toContainEqual([
      "Method version",
      "gaussian_copula_endogeneity_v1",
    ]);
  });
});
