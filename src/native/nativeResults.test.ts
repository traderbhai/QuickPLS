import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import { tablesToCsv } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import { NATIVE_NCA_ENGINE_SCOPE_WARNING, NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import { NATIVE_PCA_ENGINE_SCOPE_WARNING } from "./nativePca";
import { NATIVE_OLS_ENGINE_SCOPE_WARNING } from "./nativeOls";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { completedGscaRun } from "./nativeGsca.testFixture";
import {
  buildNativeResultNavigation,
  completedResultRuns,
  nativeModerationPlot,
  nativeCbsemDiagramRun,
  nativeCbsemResultProjection,
  nativeGscaResultProjection,
  nativeNcaPlot,
  nativeNcaResultProjection,
  nativePcaResultProjection,
  nativeOlsResultProjection,
  nativeResultTables,
  resolveSelectedCompletedRun,
  resultTableForItem,
} from "./nativeResults";

const DISPLAY_LABELS: Record<string, string> = {
  competence: "Technical Capability",
  likeability: "Brand Appeal",
  satisfaction: "Customer Fulfilment",
  loyalty: "Customer Retention",
};

function labelledModelSnapshot(
  labels: Record<string, string> = DISPLAY_LABELS,
): NonNullable<AnalysisRun["modelSnapshot"]> {
  return {
    nodes: Object.entries(labels).map(([id, label], index) => ({
      id,
      type: "construct",
      position: { x: index * 100, y: index * 40 },
      data: {
        label,
        shortName: `C${index + 1}`,
        mode: "reflective",
        indicators: [],
      },
    })),
    edges: [],
  };
}

function completedMgaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    id: "two-group-permutation-mga-run",
    name: "Two-Group Permutation MGA run",
    method: "Two-Group Permutation MGA",
    modelSnapshot: labelledModelSnapshot(),
    assessment: undefined,
    bootstrap: undefined,
    permutation: undefined,
    result: {
      ...base.result!,
      mga: {
        method_version: "pls_mga_two_group_v2",
        group_column: "market",
        groups: [
          {
            group: "north",
            observations: 54,
            paths: [{ source: "competence", target: "satisfaction", coefficient: 0.46 }],
            r_squared: { satisfaction: 0.37 },
            outer_estimates: [
              { construct: "competence", indicator: "COMP1", loading: 0.82, weight: 0.52 },
              { construct: "satisfaction", indicator: "SAT1", loading: 0.88, weight: 0.61 },
            ],
            transforms: [
              { indicator: "COMP1", mean: 3.1, scale: 0.8 },
              { indicator: "SAT1", mean: 3.4, scale: 0.9 },
            ],
          },
          {
            group: "south",
            observations: 49,
            paths: [{ source: "competence", target: "satisfaction", coefficient: 0.28 }],
            r_squared: { satisfaction: 0.21 },
            outer_estimates: [
              { construct: "competence", indicator: "COMP1", loading: 0.76, weight: 0.47 },
              { construct: "satisfaction", indicator: "SAT1", loading: 0.84, weight: 0.56 },
            ],
            transforms: [
              { indicator: "COMP1", mean: 3, scale: 0.85 },
              { indicator: "SAT1", mean: 3.2, scale: 0.95 },
            ],
          },
        ],
        comparisons: [{
          source: "competence",
          target: "satisfaction",
          group_a: "north",
          group_b: "south",
          coefficient_a: 0.46,
          coefficient_b: 0.28,
          difference: 0.18,
          standard_error: 0.09,
          t_statistic: 2,
          p_value_two_sided: 0.049,
          warning: "Approximate normal inference is not part of the native MGA report.",
        }],
        measurement_comparisons: [
          { parameter: "outer_loading", construct: "competence", indicator: "COMP1", group_a: "north", group_b: "south", estimate_a: 0.82, estimate_b: 0.76, difference: 0.06 },
          { parameter: "outer_weight", construct: "competence", indicator: "COMP1", group_a: "north", group_b: "south", estimate_a: 0.52, estimate_b: 0.47, difference: 0.05 },
          { parameter: "outer_loading", construct: "satisfaction", indicator: "SAT1", group_a: "north", group_b: "south", estimate_a: 0.88, estimate_b: 0.84, difference: 0.04 },
          { parameter: "outer_weight", construct: "satisfaction", indicator: "SAT1", group_a: "north", group_b: "south", estimate_a: 0.61, estimate_b: 0.56, difference: 0.05 },
        ],
        warnings: ["Two unselected group values were excluded from this analysis."],
      },
      mga_permutation: {
        method_version: "pls_mga_permutation_v2",
        group_column: "market",
        permutation_samples: 5_000,
        usable_permutations: 5_000,
        attempted_permutations: 5_003,
        failed_permutations: 3,
        comparisons: [
          {
            source: "competence",
            target: "satisfaction",
            original_difference: 0.18,
            empirical_p_value_two_sided: 0.032,
            percentile_rank: 0.982,
          },
          {
            source: "competence-display",
            target: "satisfaction",
            original_difference: 0.99,
            empirical_p_value_two_sided: 0.001,
            percentile_rank: 1,
          },
        ],
        measurement_comparisons: [
          { parameter: "outer_loading", construct: "competence", indicator: "COMP1", original_difference: 0.06, empirical_p_value_two_sided: 0.044, percentile_rank: 0.978 },
          { parameter: "outer_weight", construct: "competence", indicator: "COMP1", original_difference: 0.05, empirical_p_value_two_sided: 0.12, percentile_rank: 0.94 },
          { parameter: "outer_loading", construct: "satisfaction", indicator: "SAT1", original_difference: 0.04, empirical_p_value_two_sided: 0.2, percentile_rank: 0.9 },
          { parameter: "outer_weight", construct: "satisfaction", indicator: "SAT1", original_difference: 0.05, empirical_p_value_two_sided: 0.18, percentile_rank: 0.91 },
        ],
        warnings: [
          "Three permutations were unusable.",
        ],
      },
      micom: {
        method_version: "micom_v2",
        group_column: "market",
        permutation_samples: 5_000,
        usable_permutations: 5_000,
        attempted_permutations: 5_003,
        failed_permutations: 3,
        confidence_level: 0.95,
        groups: [{ group: "north", observations: 54 }, { group: "south", observations: 49 }],
        constructs: [
          {
            construct: "competence",
            configural_invariance: true,
            compositional_correlation: 0.995,
            compositional_p_value: 0.42,
            compositional_correlation_lower: 0.98,
            mean_a: 0.4,
            mean_b: 0.3,
            mean_difference: 0.1,
            mean_p_value: 0.31,
            mean_difference_lower: -0.15,
            mean_difference_upper: 0.18,
            variance_a: 1.2214027581601699,
            variance_b: 1,
            variance_difference: 0.2,
            variance_p_value: 0.08,
            variance_difference_lower: -0.1,
            variance_difference_upper: 0.25,
            equal_means: true,
            equal_variances: true,
            partial_invariance: true,
            full_invariance: true,
          },
          {
            construct: "satisfaction",
            configural_invariance: true,
            compositional_correlation: 0.97,
            compositional_p_value: 0.03,
            compositional_correlation_lower: 0.98,
            mean_a: 0.12,
            mean_b: -0.1,
            mean_difference: 0.22,
            mean_p_value: 0.04,
            mean_difference_lower: -0.1,
            mean_difference_upper: 0.2,
            variance_a: 0.8607079764250578,
            variance_b: 1,
            variance_difference: -0.15,
            variance_p_value: 0.22,
            variance_difference_lower: -0.3,
            variance_difference_upper: 0.2,
            equal_means: false,
            equal_variances: true,
            partial_invariance: false,
            full_invariance: false,
          },
        ],
        warnings: [],
      },
    },
  };
}

function completedNcaRun(ceiling: "ce_fdh" | "cr_fdh" | "both" = "both"): AnalysisRun {
  const base = completedSamplePlsRun();
  const selectedCeilings = ceiling === "both" ? ["ce_fdh", "cr_fdh"] as const : [ceiling] as const;
  const bottlenecks = selectedCeilings.flatMap((ceilingLine) => [10, 20, 30, 40, 50, 60, 70, 80, 90].map((outcomePercent) => ({
    ceiling: ceilingLine,
    outcome_percent: outcomePercent,
    required_x_percent: outcomePercent === 90 ? null : outcomePercent / 2,
    status: outcomePercent === 90 ? "not_attainable" as const : "required" as const,
  })));
  const settings = {
    method: "nca" as const,
    weighting_scheme: "path" as const,
    tolerance: 1e-7,
    max_iterations: 3_000,
    bootstrap_samples: 0,
    studentized_inner_samples: 0,
    permutation_samples: 0,
    seed: 20_260_811,
    workers: 1,
    confidence_level: 0.95,
    preprocessing: "unstandardized" as const,
    missing_data: "listwise_deletion" as const,
    case_weight_column: null,
  };
  return {
    ...base,
    id: "nca-result",
    modelId: null,
    modelSnapshot: undefined,
    name: "Necessary Condition Analysis run",
    method: "Necessary Condition Analysis",
    seed: 20_260_811,
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-nca",
      dataset_fingerprint: "sha256:nca-fixture",
      method: "nca",
      method_version: "nca_v2",
      engine_version: "2.45.0",
      seed: 20_260_811,
      settings,
      started_at: "2026-08-11T08:00:00.000Z",
      completed_at: "2026-08-11T08:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "nca_v2",
      iterations: 0,
      used_observations: 8,
      omitted_observations: 0,
      nca: {
        method_version: "nca_v2",
        ceiling,
        permutation_samples: 19,
        usable_permutations: 19,
        x: "condition",
        y: "outcome",
        observations: 8,
        scope: { minimum_x: 1, maximum_x: 8, minimum_y: 1, maximum_y: 9 },
        ce_fdh_peers: [{ x: 1, y: 1 }, { x: 3, y: 2.5 }, { x: 6, y: 7 }, { x: 8, y: 9 }],
        ceilings: selectedCeilings.map((ceilingLine) => ceilingLine === "ce_fdh"
          ? { ceiling: ceilingLine, effect_size: 0.3125, permutation_p_value: 0.05, slope: null, intercept: null }
          : { ceiling: ceilingLine, effect_size: 0.28125, permutation_p_value: 0.1, slope: 1, intercept: 0 }),
        bottlenecks,
        warnings: [NATIVE_NCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

function completedPcaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  const settings = {
    method: "pca" as const,
    weighting_scheme: "path" as const,
    tolerance: 1e-7,
    max_iterations: 3_000,
    bootstrap_samples: 0,
    studentized_inner_samples: 0,
    permutation_samples: 0,
    seed: 20_260_812,
    workers: 1,
    confidence_level: 0.95,
    preprocessing: "standardized" as const,
    missing_data: "listwise_deletion" as const,
    case_weight_column: null,
  };
  const weight = Math.SQRT1_2;
  return {
    ...base,
    id: "pca-result",
    modelId: null,
    modelSnapshot: undefined,
    name: "Principal Component Analysis run",
    method: "Principal Component Analysis",
    seed: 20_260_812,
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-pca",
      dataset_fingerprint: "sha256:pca-fixture",
      method: "pca",
      method_version: "pca_v1",
      engine_version: "2.45.0",
      seed: 20_260_812,
      settings,
      started_at: "2026-08-12T01:00:00.000Z",
      completed_at: "2026-08-12T01:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "pca_v1",
      iterations: 0,
      used_observations: 3,
      omitted_observations: 0,
      pca: {
        method_version: "pca_v1",
        component_rule: "variance_threshold",
        retained_components: 2,
        observations: 3,
        variables: ["a", "b"],
        components: [
          { component: "PC1", eigenvalue: 1.8, explained_variance: 0.9, cumulative_variance: 0.9 },
          { component: "PC2", eigenvalue: 0.2, explained_variance: 0.1, cumulative_variance: 1 },
        ],
        loadings: [
          { variable: "a", component: "PC1", loading: weight * Math.sqrt(1.8), weight },
          { variable: "b", component: "PC1", loading: weight * Math.sqrt(1.8), weight },
          { variable: "a", component: "PC2", loading: weight * Math.sqrt(0.2), weight },
          { variable: "b", component: "PC2", loading: -weight * Math.sqrt(0.2), weight: -weight },
        ],
        scores: [
          { observation: 0, component: "PC1", score: -1.2 },
          { observation: 1, component: "PC1", score: 0.1 },
          { observation: 2, component: "PC1", score: 1.1 },
          { observation: 0, component: "PC2", score: -0.3 },
          { observation: 1, component: "PC2", score: 0.6 },
          { observation: 2, component: "PC2", score: -0.3 },
        ],
        warnings: [NATIVE_PCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

function completedOlsRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    id: "ols-result",
    modelId: null,
    modelSnapshot: undefined,
    name: "Ordinary Least Squares Regression run",
    method: "Ordinary Least Squares Regression",
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-ols",
      dataset_fingerprint: "sha256:ols-fixture",
      method: "regression",
      method_version: "regression_ols_v1",
      engine_version: "2.45.0",
      seed: 7,
      settings: {
        method: "regression",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "unstandardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 7,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-12T01:00:00.000Z",
      completed_at: "2026-08-12T01:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "regression_ols_v1",
      used_observations: 5,
      regression: {
        method_version: "regression_ols_v1",
        regression_type: "ols",
        outcome: "y",
        predictors: ["x"],
        controls: ["m"],
        observations: 5,
        coefficients: [
          { term: "intercept", estimate: 1, standard_error: 0.5, statistic: 2, p_value_two_sided: 0.18, confidence_interval_lower: -1.15, confidence_interval_upper: 3.15, odds_ratio: null },
          { term: "x", estimate: 2, standard_error: 0.25, statistic: 8, p_value_two_sided: 0.015, confidence_interval_lower: 0.92, confidence_interval_upper: 3.08, odds_ratio: null },
          { term: "m", estimate: -0.5, standard_error: 0.2, statistic: -2.5, p_value_two_sided: 0.13, confidence_interval_lower: -1.36, confidence_interval_upper: 0.36, odds_ratio: null },
        ],
        fit: { r_squared: 0.9, adjusted_r_squared: 0.8, f_statistic: 9, log_likelihood: null, pseudo_r_squared: null, aic: 4, bic: 3, rmse: 0.2 },
        predictions: Array.from({ length: 5 }, (_, observation) => ({ observation, fitted: observation * 2 + 1, residual: observation % 2 ? -0.1 : 0.1, probability: null })),
        process: null,
        warnings: [NATIVE_OLS_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

describe("native result navigation", () => {
  it("projects exact nca_v2 output into standalone tables and an accessible ceiling plot", () => {
    const run = completedNcaRun();
    const projection = nativeNcaResultProjection(run);
    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);

    expect(projection).toMatchObject({
      methodVersion: "nca_v2",
      x: "condition",
      y: "outcome",
      observations: 8,
      ceiling: "both",
      permutationSamples: 19,
      usablePermutations: 19,
    });
    expect(nativeNcaPlot(run)).toMatchObject({
      xLabel: "condition",
      yLabel: "outcome",
      ceiling: "both",
      crFdh: { slope: 1, intercept: 0 },
    });
    expect(navigation.defaultItemId).toBe("nca_ceiling_effects");
    expect(navigation.groups.map((group) => group.id)).toEqual(["necessary_conditions"]);
    expect(navigation.groups[0].title).toBe("Necessary conditions");
    expect(navigation.groups[0].items.map((item) => item.id)).toEqual([
      "nca_ceiling_effects",
      "nca_cr_line",
      "nca_bottlenecks",
      "nca_scope",
    ]);
    expect(table("nca_ceiling_effects")).toMatchObject({
      title: "Ceiling effect sizes and permutation inference",
      columns: ["Ceiling line", "Effect size", "Permutation p"],
      rows: [["CE-FDH", "0.3125", "0.0500"], ["CR-FDH", "0.2813", "0.1000"]],
    });
    expect(table("nca_cr_line")?.rows).toEqual([["CR-FDH", "1.000000", "0.000000"]]);
    expect(table("nca_bottlenecks")).toMatchObject({
      title: "Observed-range bottlenecks",
      columns: ["Ceiling line", "Outcome (% observed range)", "Condition requirement"],
    });
    expect(table("nca_bottlenecks")?.rows).toHaveLength(18);
    expect(table("nca_bottlenecks")?.rows.at(-1)).toEqual(["CR-FDH", "90%", "Not attainable"]);
    expect(table("nca_scope")?.rows).toEqual(expect.arrayContaining([
      ["Condition variable (X)", "condition"],
      ["Outcome variable (Y)", "outcome"],
      ["Analyzed observations", "8"],
      ["Ceiling lines", "CE-FDH and CR-FDH"],
      ["Requested permutations", "19"],
      ["Usable permutations", "19"],
      ["Method version", "nca_v2"],
    ]));
    expect(table("nca_scope")?.warning).toBe(NATIVE_NCA_ENGINE_SCOPE_WARNING);
    expect(navigation.tables.flatMap((candidate) => candidate.rows).flat()).not.toContain("N/A");
    expect(navigation.groups.some((group) => group.id === "graphical" || group.id === "quality_criteria")).toBe(false);
    expect(tablesToCsv(navigation.tables)).toContain("Observed-range bottlenecks");
  });

  it("omits unavailable CR coefficients and rejects incomplete or stale NCA contracts", () => {
    const ceOnly = completedNcaRun("ce_fdh");
    const ceNavigation = buildNativeResultNavigation(ceOnly);
    expect(ceNavigation.tables.some((table) => table.id === "nca_cr_line")).toBe(false);
    expect(nativeNcaPlot(ceOnly)?.crFdh).toBeNull();

    const missingBottleneck = completedNcaRun();
    missingBottleneck.result!.nca!.bottlenecks.pop();
    expect(nativeResultTables(missingBottleneck)).toEqual([]);
    expect(buildNativeResultNavigation(missingBottleneck)).toMatchObject({ defaultItemId: null, groups: [], tables: [] });

    const staleAssessment = completedNcaRun();
    staleAssessment.assessment!.warnings = ["PLS assessment is not applicable to standalone v0.8 methods."];
    expect(nativeResultTables(staleAssessment)).toEqual([]);
  });

  it("projects exact pca_v1 output into model-free component tables", () => {
    const run = completedPcaRun();
    const projection = nativePcaResultProjection(run);
    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);

    expect(projection).toMatchObject({
      methodVersion: "pca_v1",
      componentRule: "variance_threshold",
      retainedComponents: 2,
      observations: 3,
      variables: ["a", "b"],
      scoresStored: 6,
    });
    expect(navigation.defaultItemId).toBe("pca_component_summary");
    expect(navigation.groups.map((group) => group.id)).toEqual(["components"]);
    expect(navigation.groups[0].items.map((item) => item.id)).toEqual([
      "pca_component_summary",
      "pca_loadings",
      "pca_scope",
    ]);
    expect(table("pca_component_summary")).toMatchObject({
      title: "Component summary",
      rows: [
        ["PC1", "1.800000", "90.00%", "90.00%"],
        ["PC2", "0.200000", "10.00%", "100.00%"],
      ],
    });
    expect(table("pca_loadings")?.rows).toHaveLength(4);
    expect(table("pca_scope")?.rows).toEqual(expect.arrayContaining([
      ["Selected variables", "a, b"],
      ["Retention rule", "Cumulative variance threshold"],
      ["Stored component scores", "6"],
      ["Method version", "pca_v1"],
    ]));
    expect(navigation.groups.some((group) => group.id === "graphical" || group.id === "quality_criteria")).toBe(false);
    expect(tablesToCsv(navigation.tables)).not.toContain("N/A");

    const tampered = completedPcaRun();
    tampered.result!.pca!.loadings[0].loading = 42;
    expect(nativeResultTables(tampered)).toEqual([]);
  });

  it("projects exact regression_ols_v1 output into model-free OLS tables", () => {
    const run = completedOlsRun();
    expect(nativeOlsResultProjection(run)).toMatchObject({
      methodVersion: "regression_ols_v1",
      outcome: "y",
      predictors: ["x"],
      controls: ["m"],
      observations: 5,
      predictionsStored: 5,
    });
    const navigation = buildNativeResultNavigation(run);
    expect(navigation.defaultItemId).toBe("ols_coefficients");
    expect(navigation.groups.map((group) => group.id)).toEqual(["regression"]);
    expect(navigation.groups[0].items.map((item) => item.id)).toEqual([
      "ols_coefficients",
      "ols_model_fit",
      "ols_scope",
    ]);
    expect(navigation.tables.find((table) => table.id === "ols_coefficients")).toMatchObject({
      title: "Coefficients",
      columns: ["Term", "Estimate", "HC3 SE", "t", "p (two-sided)", "95% CI lower", "95% CI upper"],
    });
    expect(navigation.tables.find((table) => table.id === "ols_scope")?.rows).toEqual(expect.arrayContaining([
      ["Outcome", "y"],
      ["Predictors", "x"],
      ["Controls", "m"],
      ["Method version", "regression_ols_v1"],
    ]));
    expect(tablesToCsv(navigation.tables)).not.toContain("N/A");

    const tampered = completedOlsRun();
    tampered.result!.regression!.coefficients[1].odds_ratio = 2;
    expect(nativeResultTables(tampered)).toEqual([]);
  });

  it("projects exact MICOM v2 and permutation MGA v2 output into truthful group tables", () => {
    const navigation = buildNativeResultNavigation(completedMgaRun());
    const group = navigation.groups.find((candidate) => candidate.id === "groups");
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);

    expect(navigation.defaultItemId).toBe("micom_summary");
    expect(navigation.groups.some((candidate) => candidate.id === "graphical")).toBe(false);
    expect(navigation.groups.map((candidate) => candidate.id)).toEqual(["groups"]);
    expect(navigation.tables.every((candidate) => candidate.id.startsWith("mga_") || candidate.id.startsWith("micom_"))).toBe(true);
    expect(group?.items.map((item) => item.id)).toEqual([
      "mga_group_summary",
      "micom_summary",
      "micom_configural",
      "micom_composition",
      "micom_means",
      "micom_variances",
      "mga_group_paths",
      "mga_group_r_squared",
      "mga_group_loadings",
      "mga_group_weights",
      "mga_path_differences",
      "mga_loading_differences",
      "mga_weight_differences",
      "mga_permutation",
      "mga_permutation_loadings",
      "mga_permutation_weights",
    ]);
    expect(table("mga_group_summary")?.rows).toEqual([
      ["market", "Group A", "north", "54"],
      ["market", "Group B", "south", "49"],
    ]);
    expect(table("mga_group_paths")?.rows).toEqual([
      ["Group A", "north", expect.stringContaining("Technical Capability"), "0.460000"],
      ["Group B", "south", expect.stringContaining("Technical Capability"), "0.280000"],
    ]);
    expect(table("mga_group_r_squared")?.rows).toEqual([
      ["Group A", "north", "Customer Fulfilment", "0.370000"],
      ["Group B", "south", "Customer Fulfilment", "0.210000"],
    ]);
    expect(table("mga_path_differences")?.columns).toEqual([
      "Path", "Group A", "Coefficient A", "Group B", "Coefficient B", "A − B",
    ]);
    expect(table("mga_path_differences")?.columns).not.toEqual(expect.arrayContaining(["SE", "t", "p"]));
    expect(table("mga_path_differences")?.rows).toEqual([[
      expect.stringContaining("Customer Fulfilment"),
      "north",
      "0.460000",
      "south",
      "0.280000",
      "0.180000",
    ]]);
    expect(table("mga_group_loadings")?.rows).toEqual([
      ["Group A", "north", "Technical Capability", "COMP1", "0.820000"],
      ["Group A", "north", "Customer Fulfilment", "SAT1", "0.880000"],
      ["Group B", "south", "Technical Capability", "COMP1", "0.760000"],
      ["Group B", "south", "Customer Fulfilment", "SAT1", "0.840000"],
    ]);
    expect(table("mga_group_weights")?.rows[0]).toEqual([
      "Group A", "north", "Technical Capability", "COMP1", "0.520000",
    ]);
    expect(table("mga_loading_differences")?.rows).toEqual([
      ["Technical Capability", "COMP1", "north", "0.820000", "south", "0.760000", "0.060000"],
      ["Customer Fulfilment", "SAT1", "north", "0.880000", "south", "0.840000", "0.040000"],
    ]);
    expect(table("mga_weight_differences")?.rows).toHaveLength(2);
    expect(table("mga_permutation")?.columns).toEqual([
      "Path", "A − B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations",
    ]);
    expect(table("mga_permutation")?.rows).toEqual([[
      expect.stringContaining("Technical Capability"),
      "0.180000",
      "0.0320",
      "0.9820",
      "5000",
      "5000",
    ]]);
    expect(table("mga_permutation_loadings")?.rows[0]).toEqual([
      "Technical Capability", "COMP1", "0.060000", "0.0440", "0.9780", "5000", "5000",
    ]);
    expect(table("mga_permutation_weights")?.rows).toHaveLength(2);
    expect(table("micom_configural")?.rows).toEqual([
      ["Technical Capability", "Confirmed"],
      ["Customer Fulfilment", "Confirmed"],
    ]);
    expect(table("micom_composition")?.rows).toEqual([
      ["Technical Capability", "0.995000", "0.980000", "0.4200", "Established"],
      ["Customer Fulfilment", "0.970000", "0.980000", "0.0300", "Not established"],
    ]);
    expect(table("micom_means")?.columns.slice(1, 4)).toEqual(["Mean north", "Mean south", "Mean difference (north - south)"]);
    expect(table("micom_means")?.rows[1]).toEqual([
      "Customer Fulfilment", "0.120000", "-0.100000", "0.220000", "-0.100000", "0.200000", "0.0400", "Different",
    ]);
    expect(table("micom_variances")?.columns.slice(1, 4)).toEqual(["Variance north", "Variance south", "Log variance ratio (north/south)"]);
    expect(table("micom_summary")?.rows).toEqual([
      ["Technical Capability", "Confirmed", "Established", "Established", "Equal", "Equal", "Established", "95.0%", "5000"],
      ["Customer Fulfilment", "Confirmed", "Not established", "Not established", "Different", "Equal", "Not established", "95.0%", "5000"],
    ]);
    expect(navigation.tables.flatMap((candidate) => candidate.rows).flat()).not.toContain("N/A");
    expect(navigation.tables.every((candidate) => candidate.rows.length > 0)).toBe(true);
    expect(tablesToCsv(navigation.tables)).toContain("MICOM Step 2 - compositional invariance");
  });

  it("keeps descriptive archived MGA output but gates all current inference on exact v2 contracts", () => {
    const run = completedMgaRun();
    const archived: AnalysisRun = {
      ...run,
      result: run.result ? {
        ...run.result,
        mga: run.result.mga ? { ...run.result.mga, method_version: "pls_mga_two_group_v1" } : null,
        mga_permutation: run.result.mga_permutation
          ? { ...run.result.mga_permutation, method_version: "pls_mga_permutation_v1" }
          : null,
        micom: run.result.micom ? { ...run.result.micom, method_version: "micom_v1" } : null,
      } : undefined,
    };

    const navigation = buildNativeResultNavigation(archived);
    expect(navigation.defaultItemId).toBe("mga_path_differences");
    expect(navigation.tables.some((table) => table.id === "mga_permutation")).toBe(false);
    expect(navigation.tables.some((table) => table.id.startsWith("micom_"))).toBe(false);
    expect(navigation.tables.some((table) => table.id === "mga_group_loadings")).toBe(false);
    expect(navigation.tables.some((table) => table.id === "mga_loading_differences")).toBe(false);
    expect(navigation.tables.some((table) => table.id === "mga_group_paths")).toBe(true);
    expect(navigation.groups.some((group) => group.id === "graphical")).toBe(false);
  });

  it("omits inconsistent MICOM decisions and measurement inference instead of fabricating cells", () => {
    const run = completedMgaRun();
    run.result!.micom!.constructs[0].full_invariance = false;
    run.result!.mga_permutation!.measurement_comparisons![0].original_difference = 99;

    const navigation = buildNativeResultNavigation(run);
    expect(navigation.tables.some((table) => table.id.startsWith("micom_"))).toBe(false);
    expect(navigation.tables.find((table) => table.id === "mga_permutation_loadings")?.rows).toEqual([
      ["Customer Fulfilment", "SAT1", "0.040000", "0.2000", "0.9000", "5000", "5000"],
    ]);
    expect(navigation.tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");
  });

  it("selects only completed runs backed by a result payload", () => {
    const complete = completedSamplePlsRun();
    const failed: AnalysisRun = { ...complete, id: "failed", status: "failed" };
    const missing: AnalysisRun = { ...complete, id: "missing", result: undefined };

    expect(completedResultRuns([failed, missing, complete])).toEqual([complete]);
  });

  it("honors the newly completed run selected by the calculation lifecycle", () => {
    const previous = { ...completedSamplePlsRun(), id: "previous-run", name: "Previous run" };
    const completed = { ...completedSamplePlsRun(), id: "consumed-envelope-run", name: "Structural Path Randomization run" };

    expect(resolveSelectedCompletedRun([completed, previous], completed.id)).toBe(completed);
    expect(resolveSelectedCompletedRun([completed, previous], previous.id)).toBe(previous);
    expect(resolveSelectedCompletedRun([completed, previous], "stale-run-id")).toBe(completed);
  });

  it("normalizes rounded negative zero in IPMA tables and table-backed exports", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      modelSnapshot: {
        nodes: [
          { id: "x", position: { x: 0, y: 0 }, data: { label: "Driver", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
          { id: "y", position: { x: 240, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
        ],
        edges: [{ id: "x-y", source: "x", target: "y" }],
      },
      result: {
        ...base.result!,
        ipma: {
          method_version: "ipma_v1",
          performance_scale: "min_max_0_100_from_standardized_scores_v1",
          targets: ["y"],
          constructs: [{ target: "y", construct: "x", importance: -Number.EPSILON, performance: 50, score_mean: -Number.EPSILON }],
          indicators: [
            { target: "y", construct: "x", indicator: "x1", construct_importance: 0.4, loading: 0.9, performance: 50, score_mean: -Number.EPSILON },
            { target: "y", construct: "x", indicator: "x2", construct_importance: 0.4, loading: 0.8, performance: 40, score_mean: -0.04 },
          ],
          warnings: [],
        },
      },
    };

    const indicatorTable = buildNativeResultNavigation(run).tables
      .find((table) => table.id === "ipma_indicators");
    expect(indicatorTable?.rows.map((row) => row[6])).toEqual(["0.000000", "-0.040000"]);

    const csv = tablesToCsv([indicatorTable!]);
    expect(csv).toContain("0.000000");
    expect(csv).toContain("-0.040000");
    expect(csv).not.toContain("-0.000000");
  });

  it("derives a graphical item and every genuine core, quality, and inference capability", () => {
    const navigation = buildNativeResultNavigation(completedSamplePlsRun());
    const itemIds = navigation.groups.flatMap((group) => group.items.map((item) => item.id));
    const tableIds = navigation.tables.map((table) => table.id);

    expect(navigation.defaultItemId).toBe("model_estimates");
    expect(itemIds).toContain("model_estimates");
    expect(tableIds).toEqual(expect.arrayContaining([
      "direct_effects",
      "specific_indirect_effects",
      "total_indirect_effects",
      "outer_loadings",
      "outer_weights",
      "r_squared",
      "total_effects",
      "mediation_bootstrap",
      "construct_reliability",
      "cross_loadings",
      "fornell_larcker",
      "htmt_plus",
      "structural_quality",
      "structural_vif",
      "f_squared",
      "model_fit",
      "blindfolding",
      "bootstrap_percentile",
      "bootstrap_bca",
      "bootstrap_studentized",
    ]));
    expect(tableIds).not.toContain("formative_indicator_vif");
    expect(itemIds).toEqual(expect.arrayContaining(tableIds));
    expect(navigation.groups.find((group) => group.id === "mediation")?.items.map((item) => item.id)).toEqual([
      "direct_effects",
      "specific_indirect_effects",
      "total_indirect_effects",
      "total_effects",
      "mediation_bootstrap",
    ]);
  });

  it("surfaces genuine direct, specific indirect, total indirect, total, and matched bootstrap mediation output", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      bootstrap: base.bootstrap ? {
        ...base.bootstrap,
        percentile: {
          ...base.bootstrap.percentile,
          parameters: [
            {
              parameter: "[\"direct_effect\",[\"competence\",\"loyalty\"]]",
              original: 0.116,
              bootstrap_mean: 0.12,
              bias: 0.004,
              standard_error: 0.03,
              lower: 0.06,
              upper: 0.18,
              usable_replicates: 999,
              t_statistic: 3.8667,
              p_value_two_sided: 0.002,
            },
            ...base.bootstrap.percentile.parameters,
            {
              parameter: "[\"total_effect\",[\"competence\",\"loyalty\"]]",
              original: 0.335,
              bootstrap_mean: 0.338,
              bias: 0.003,
              standard_error: 0.06,
              lower: 0.21,
              upper: 0.44,
              usable_replicates: 999,
              t_statistic: 5.5833,
              p_value_two_sided: 0.0002,
            },
          ],
        },
      } : undefined,
    };
    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);

    expect(table("direct_effects")?.title).toBe("Direct effects");
    expect(table("specific_indirect_effects")?.rows).toEqual([
      ["competence → satisfaction → loyalty", "0.219232"],
      ["likeability → satisfaction → loyalty", "0.177888"],
    ]);
    expect(table("total_indirect_effects")?.rows).toEqual([
      ["competence → loyalty", "0.219000"],
      ["likeability → loyalty", "0.178000"],
    ]);
    expect(table("total_effects")?.columns).toEqual(["Effect", "Total effect"]);
    expect(table("mediation_bootstrap")?.columns).toEqual([
      "Effect type",
      "Effect",
      "Original sample (O)",
      "Sample mean (M)",
      "Standard deviation (STDEV)",
      "T statistics (|O/STDEV|)",
      "P values",
      "CI lower",
      "CI upper",
    ]);
    expect(table("mediation_bootstrap")?.title).toBe("Aggregate mediation effects bootstrap inference");
    expect(table("mediation_bootstrap")?.rows).toEqual([
      ["Direct effect", "competence → loyalty", "0.116000", "0.120000", "0.030000", "3.867", "0.0020", "0.060000", "0.180000"],
      ["Total indirect effect (aggregate)", "competence → loyalty", "0.219000", "0.218000", "0.057000", "3.842", "0.0040", "0.101000", "0.328000"],
      ["Total effect", "competence → loyalty", "0.335000", "0.338000", "0.060000", "5.583", "0.0002", "0.210000", "0.440000"],
    ]);
    expect(table("bootstrap_percentile")?.rows.map((row) => row[0])).toEqual([
      "Path: competence → satisfaction",
      "Path: satisfaction → loyalty",
    ]);
    expect(table("bootstrap_bca")?.rows[0]?.[0]).toBe("Indirect effect: competence → loyalty");
    expect(table("bootstrap_studentized")?.rows[0]?.[0]).toBe("Indirect effect: competence → loyalty");
    expect(navigation.tables.some((candidate) => candidate.id === "path_coefficients")).toBe(false);

    const csv = tablesToCsv(navigation.tables);
    expect(csv).toContain("Direct effects");
    expect(csv).toContain("Specific indirect effects");
    expect(csv).toContain("Total indirect effects");
    expect(csv).toContain("Aggregate mediation effects bootstrap inference");
    expect(csv).not.toMatch(/\bN\/?A\b/i);
  });

  it("uses immutable run-snapshot labels for mediation effects and inference without changing engine identities", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      modelSnapshot: labelledModelSnapshot(),
    };
    const tables = nativeResultTables(run);
    const table = (id: string) => tables.find((candidate) => candidate.id === id);

    expect(table("direct_effects")?.rows[0]).toEqual([
      "Technical Capability → Customer Fulfilment",
      "0.403000",
    ]);
    expect(table("specific_indirect_effects")?.rows).toEqual([
      ["Technical Capability → Customer Fulfilment → Customer Retention", "0.219232"],
      ["Brand Appeal → Customer Fulfilment → Customer Retention", "0.177888"],
    ]);
    expect(table("total_indirect_effects")?.rows).toEqual([
      ["Technical Capability → Customer Retention", "0.219000"],
      ["Brand Appeal → Customer Retention", "0.178000"],
    ]);
    expect(table("total_effects")?.rows[0]?.[0]).toBe("Technical Capability → Customer Fulfilment");
    expect(table("mediation_bootstrap")?.rows[0]?.slice(0, 2)).toEqual([
      "Total indirect effect (aggregate)",
      "Technical Capability → Customer Retention",
    ]);
    expect(table("bootstrap_percentile")?.rows.map((row) => row[0])).toEqual([
      "Path: Technical Capability → Customer Fulfilment",
      "Path: Customer Fulfilment → Customer Retention",
    ]);
    expect(table("bootstrap_bca")?.rows[0]?.[0]).toBe(
      "Indirect effect: Technical Capability → Customer Retention",
    );
    expect(table("bootstrap_studentized")?.rows[0]?.[0]).toBe(
      "Indirect effect: Technical Capability → Customer Retention",
    );
  });

  it("maps ordinary model, quality, and known parameter identities while retaining indicator and unknown identities", () => {
    const base = completedSamplePlsRun();
    const referenceParameter = base.bootstrap!.percentile.parameters[0]!;
    const run: AnalysisRun = {
      ...base,
      modelSnapshot: labelledModelSnapshot(),
      result: {
        ...base.result!,
        mediation: undefined,
      },
      bootstrap: {
        ...base.bootstrap!,
        percentile: {
          ...base.bootstrap!.percentile,
          parameters: [
            ...base.bootstrap!.percentile.parameters,
            { ...referenceParameter, parameter: "[\"outer_loading\",[\"competence\",\"COMP1\"]]" },
            { ...referenceParameter, parameter: "[\"r_squared\",[\"satisfaction\"]]" },
            { ...referenceParameter, parameter: "[\"custom_metric\",[\"competence\",\"COMP1\"]]" },
          ],
        },
      },
    };
    const tables = nativeResultTables(run);
    const table = (id: string) => tables.find((candidate) => candidate.id === id);

    expect(table("path_coefficients")?.rows[0]).toEqual([
      "Technical Capability → Customer Fulfilment",
      "0.403000",
    ]);
    expect(table("outer_loadings")?.rows[0]).toEqual(["Technical Capability", "COMP1", "0.842000"]);
    expect(table("r_squared")?.rows).toEqual([
      ["Customer Fulfilment", "0.544000"],
      ["Customer Retention", "0.617000"],
    ]);
    expect(table("construct_reliability")?.rows[0]?.[0]).toBe("Technical Capability");
    expect(table("cross_loadings")?.rows[0]?.slice(0, 3)).toEqual([
      "COMP1",
      "Technical Capability",
      "Technical Capability",
    ]);
    expect(table("fornell_larcker")?.rows[0]?.slice(0, 2)).toEqual([
      "Technical Capability",
      "Technical Capability",
    ]);
    expect(table("htmt_plus")?.rows[0]?.slice(0, 2)).toEqual(["Brand Appeal", "Technical Capability"]);
    expect(table("structural_quality")?.rows[0]?.[0]).toBe("Customer Fulfilment");
    expect(table("structural_vif")?.rows[0]?.slice(0, 2)).toEqual([
      "Customer Fulfilment",
      "Technical Capability",
    ]);
    expect(table("f_squared")?.rows[0]?.[0]).toBe("Technical Capability → Customer Fulfilment");
    expect(table("blindfolding")?.rows[0]?.[0]).toBe("Customer Fulfilment");

    const parameterLabels = table("bootstrap_percentile")?.rows.map((row) => row[0]) ?? [];
    expect(parameterLabels).toContain("Outer loading: Technical Capability → COMP1");
    expect(parameterLabels).toContain("R squared: Customer Fulfilment");
    expect(parameterLabels).toContain("Custom metric: competence → COMP1");
  });

  it("disambiguates duplicate snapshot labels while preserving legacy ID fallback", () => {
    const base = completedSamplePlsRun();
    const duplicatedLabels = labelledModelSnapshot({
      ...DISPLAY_LABELS,
      competence: "Shared construct",
      likeability: "Ｓｈａｒｅｄ construct",
    });
    const labelledRun: AnalysisRun = {
      ...base,
      modelSnapshot: duplicatedLabels,
      result: { ...base.result!, mediation: undefined },
    };
    const labelledPaths = nativeResultTables(labelledRun).find((table) => table.id === "path_coefficients")?.rows ?? [];

    expect(labelledPaths[0]?.[0]).toBe("Shared construct [competence] → Customer Fulfilment");
    expect(labelledPaths[1]?.[0]).toBe("Ｓｈａｒｅｄ construct [likeability] → Customer Fulfilment");
    expect(nativeResultTables({ ...labelledRun, modelSnapshot: undefined })
      .find((table) => table.id === "path_coefficients")?.rows[0]?.[0]).toBe("competence → satisfaction");
  });

  it("falls back for conflicting duplicate IDs and disambiguates a label from an unmatched raw ID", () => {
    const base = completedSamplePlsRun();
    const conflictingSnapshot = labelledModelSnapshot();
    const competenceNode = conflictingSnapshot.nodes.find((node) => node.id === "competence")!;
    conflictingSnapshot.nodes.push({
      ...competenceNode,
      data: { ...competenceNode.data, label: "Conflicting capability name" },
    });
    const conflictingRun: AnalysisRun = {
      ...base,
      modelSnapshot: conflictingSnapshot,
      result: { ...base.result!, mediation: undefined },
    };
    expect(nativeResultTables(conflictingRun).find((table) => table.id === "path_coefficients")?.rows[0]?.[0])
      .toBe("competence → Customer Fulfilment");

    const labelVsFallbackRun: AnalysisRun = {
      ...conflictingRun,
      modelSnapshot: labelledModelSnapshot({
        competence: "satisfaction",
        likeability: DISPLAY_LABELS.likeability,
        loyalty: DISPLAY_LABELS.loyalty,
      }),
    };
    expect(nativeResultTables(labelVsFallbackRun).find((table) => table.id === "path_coefficients")?.rows[0]?.[0])
      .toBe("satisfaction [competence] → satisfaction");
  });

  it("enumerates parallel and serial specific indirect paths from genuine structural coefficients", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      bootstrap: undefined,
      assessment: undefined,
      result: base.result ? {
        ...base.result,
        paths: [
          { source: "x", target: "m1", coefficient: 0.2 },
          { source: "m1", target: "m2", coefficient: 0.4 },
          { source: "m1", target: "y", coefficient: 0.5 },
          { source: "x", target: "m2", coefficient: 0.3 },
          { source: "m2", target: "y", coefficient: 0.6 },
          { source: "x", target: "y", coefficient: 0.1 },
        ],
        effects: [{ source: "x", target: "y", direct: 0.1, indirect: 0.328, total: 0.428 }],
        mediation: {
          method_version: "pls_mediation_v1",
          tolerance: 1e-12,
          estimates: [{
            source: "x",
            target: "y",
            direct: 0.1,
            indirect: 0.328,
            total: 0.428,
            variance_accounted_for: 0.766355,
            classification: "complementary_partial",
            warning: null,
          }],
          warnings: [],
        },
      } : undefined,
    };

    expect(nativeResultTables(run).find((table) => table.id === "specific_indirect_effects")?.rows).toEqual([
      ["x → m1 → m2", "0.080000"],
      ["x → m1 → m2 → y", "0.048000"],
      ["x → m1 → y", "0.100000"],
      ["x → m2 → y", "0.180000"],
      ["m1 → m2 → y", "0.240000"],
    ]);
  });

  it("retains a zero aggregate and matched inference when parallel specific paths cancel", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      assessment: undefined,
      result: base.result ? {
        ...base.result,
        paths: [
          { source: "x", target: "m1", coefficient: 0.5 },
          { source: "m1", target: "y", coefficient: 0.5 },
          { source: "x", target: "m2", coefficient: 0.5 },
          { source: "m2", target: "y", coefficient: -0.5 },
        ],
        effects: [
          { source: "x", target: "m1", direct: 0.5, indirect: 0, total: 0.5 },
          { source: "m1", target: "y", direct: 0.5, indirect: 0, total: 0.5 },
          { source: "x", target: "m2", direct: 0.5, indirect: 0, total: 0.5 },
          { source: "m2", target: "y", direct: -0.5, indirect: 0, total: -0.5 },
          { source: "x", target: "y", direct: 0, indirect: 0, total: 0 },
        ],
        mediation: {
          method_version: "pls_mediation_v1",
          tolerance: 1e-12,
          estimates: [{
            source: "x",
            target: "y",
            direct: 0,
            indirect: 0,
            total: 0,
            variance_accounted_for: null,
            classification: "no_effect",
            warning: null,
          }],
          warnings: [],
        },
      } : undefined,
      bootstrap: base.bootstrap ? {
        ...base.bootstrap,
        percentile: {
          ...base.bootstrap.percentile,
          parameters: [{
            parameter: "[\"indirect_effect\",[\"x\",\"y\"]]",
            original: 0,
            bootstrap_mean: 0.01,
            bias: 0.01,
            standard_error: 0.04,
            lower: -0.08,
            upper: 0.08,
            usable_replicates: 999,
            t_statistic: 0,
            p_value_two_sided: 1,
          }],
        },
        bca: null,
        studentized: null,
      } : undefined,
    };
    const tables = nativeResultTables(run);
    const table = (id: string) => tables.find((candidate) => candidate.id === id);

    expect(table("specific_indirect_effects")?.rows).toEqual([
      ["x → m1 → y", "0.250000"],
      ["x → m2 → y", "-0.250000"],
    ]);
    expect(table("total_indirect_effects")?.rows).toEqual([["x → y", "0.000000"]]);
    expect(table("mediation_bootstrap")?.rows).toEqual([[
      "Total indirect effect (aggregate)",
      "x → y",
      "0.000000",
      "0.010000",
      "0.040000",
      "0.000",
      "1.0000",
      "-0.080000",
      "0.080000",
    ]]);
    expect(table("bootstrap_percentile")).toBeUndefined();
  });

  it("bounds dense-DAG specific-path enumeration deterministically and discloses truncation", () => {
    const base = completedSamplePlsRun();
    const constructs = Array.from({ length: 14 }, (_, index) => `c${index}`);
    const paths = constructs.flatMap((source, sourceIndex) => constructs
      .slice(sourceIndex + 1)
      .map((target) => ({ source, target, coefficient: 0.5 })));
    const run: AnalysisRun = {
      ...base,
      bootstrap: undefined,
      assessment: undefined,
      result: base.result ? {
        ...base.result,
        paths,
        effects: [{ source: "c0", target: "c13", direct: 0.5, indirect: 1, total: 1.5 }],
        mediation: {
          method_version: "pls_mediation_v1",
          tolerance: 1e-12,
          estimates: [{
            source: "c0",
            target: "c13",
            direct: 0.5,
            indirect: 1,
            total: 1.5,
            variance_accounted_for: 2 / 3,
            classification: "complementary_partial",
            warning: null,
          }],
          warnings: [],
        },
      } : undefined,
    };

    const first = nativeResultTables(run).find((table) => table.id === "specific_indirect_effects");
    const second = nativeResultTables(run).find((table) => table.id === "specific_indirect_effects");

    expect(first?.rows).toHaveLength(5_000);
    expect(first?.warning).toBe("Showing the first 5,000 specific indirect paths. Additional paths were omitted to keep Results responsive.");
    expect(new Set(first?.rows.map(([path]) => path))).toHaveProperty("size", 5_000);
    expect(second?.rows).toEqual(first?.rows);
    expect(nativeResultTables(run).find((table) => table.id === "total_indirect_effects")?.rows).toEqual([
      ["c0 → c13", "1.000000"],
    ]);
  });

  it("hides mediation and mediation inference sections when their typed outputs are unavailable", () => {
    const base = completedSamplePlsRun();
    const noArtifact: AnalysisRun = {
      ...base,
      result: base.result ? { ...base.result, mediation: undefined } : undefined,
    };
    expect(buildNativeResultNavigation(noArtifact).groups.some((group) => group.id === "mediation")).toBe(false);
    expect(nativeResultTables(noArtifact).some((table) => table.id === "path_coefficients")).toBe(true);

    const noMatchedInference: AnalysisRun = {
      ...base,
      bootstrap: base.bootstrap ? {
        ...base.bootstrap,
        percentile: {
          ...base.bootstrap.percentile,
          parameters: [{
            parameter: "[\"outer_loading\",[\"competence\",\"COMP1\"]]",
            original: 0.8,
            bootstrap_mean: 0.79,
            bias: -0.01,
            standard_error: 0.04,
            lower: 0.7,
            upper: 0.86,
            usable_replicates: 999,
          }],
        },
        bca: null,
        studentized: null,
      } : undefined,
    };
    const tables = nativeResultTables(noMatchedInference);
    expect(tables.some((table) => table.id === "mediation_bootstrap")).toBe(false);
    expect(tables.every((table) => table.rows.length > 0)).toBe(true);
    expect(tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");
  });

  it("never emits an empty table or an N/A placeholder", () => {
    const tables = nativeResultTables(completedSamplePlsRun());

    expect(tables.length).toBeGreaterThan(0);
    expect(tables.every((table) => table.columns.length > 0 && table.rows.length > 0)).toBe(true);
    expect(tables.every((table) => table.rows.every((row) => row.length === table.columns.length))).toBe(true);
    expect(tables.flatMap((table) => table.rows).flat().some((cell) => /^N\/?A$/i.test(cell.trim()))).toBe(false);
  });

  it("uses PLSc corrected loadings and weights and exposes only nonduplicative correction diagnostics", () => {
    const base = completedSamplePlsRun();
    const result = base.result!;
    const correctedOuterLoadings = result.outer_estimates.map((row, index) => ({
      ...row,
      loading: 0.95 + index * 0.005,
      weight: 0.45 + index * 0.01,
    }));
    const run: AnalysisRun = {
      ...base,
      id: "plsc-run",
      method: "Consistent PLS",
      assessment: undefined,
      bootstrap: undefined,
      permutation: undefined,
      result: {
        ...result,
        method_version: "plsc_v2",
        outer_estimates: result.outer_estimates.map((row, index) => index === 0
          ? { ...row, loading: 0.111111, weight: 0.222222 }
          : row),
        paths: [{ source: "competence", target: "satisfaction", coefficient: 0.654321 }],
        effects: [{ source: "competence", target: "satisfaction", direct: 0.654321, indirect: 0, total: 0.654321 }],
        r_squared: { satisfaction: 0.765432 },
        plsc: {
          method_version: "plsc_v2",
          reliability_method_version: "dijkstra_henseler_rho_a_v1",
          tolerance: 1e-12,
          reliabilities: [{ construct: "competence", rho_a: 0.876543 }],
          construct_correlations: [{ left: "competence", right: "satisfaction", original: 0.456789, corrected: 0.567891 }],
          corrected_paths: [{ source: "competence", target: "satisfaction", coefficient: 0.654321 }],
          corrected_outer_loadings: correctedOuterLoadings,
          corrected_r_squared: { satisfaction: 0.765432 },
          warnings: [],
        },
      },
    };

    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);
    expect(navigation.defaultItemId).toBe("model_estimates");
    expect(table("outer_loadings")?.rows[0]).toEqual(["competence", "COMP1", "0.950000"]);
    expect(table("outer_weights")?.rows[0]).toEqual(["competence", "COMP1", "0.450000"]);
    expect(table("path_coefficients")?.rows).toEqual([["competence → satisfaction", "0.654321"]]);
    expect(table("r_squared")?.rows).toEqual([["satisfaction", "0.765432"]]);
    expect(table("total_effects")?.rows).toEqual([["competence → satisfaction", "0.654321", "0.000000", "0.654321"]]);
    expect(table("plsc_reliability")?.rows).toEqual([["competence", "0.876543"]]);
    expect(table("plsc_correlations")?.rows).toEqual([["competence", "satisfaction", "0.456789", "0.567891"]]);
    expect(navigation.tables.some((candidate) => candidate.id === "plsc_paths")).toBe(false);
    expect(navigation.groups.find((group) => group.id === "quality_criteria")?.items.map((item) => item.id)).toEqual([
      "plsc_reliability",
      "plsc_correlations",
    ]);
    const csv = tablesToCsv(navigation.tables);
    expect(csv).toContain("PLSc correction reliability");
    expect(csv).toContain("PLSc construct correlations");
    expect(csv).not.toMatch(/\bN\/?A\b/i);
  });

  it("keeps WPLS weighted common results and adds genuine case-weight diagnostics", () => {
    const base = completedSamplePlsRun();
    const result = base.result!;
    const run: AnalysisRun = {
      ...base,
      id: "wpls-run",
      method: "Weighted PLS",
      assessment: undefined,
      bootstrap: undefined,
      permutation: undefined,
      result: {
        ...result,
        method_version: "wpls_case_weighted_v1",
        outer_estimates: [{ construct: "competence", indicator: "COMP1", loading: 0.812345, weight: 0.412345 }],
        paths: [{ source: "competence", target: "satisfaction", coefficient: 0.712345 }],
        effects: [{ source: "competence", target: "satisfaction", direct: 0.712345, indirect: 0, total: 0.712345 }],
        r_squared: { satisfaction: 0.612345 },
        wpls: {
          method_version: "wpls_case_weighted_v1",
          case_weight_column: "case_wt",
          weight_sum: 211.55,
          effective_sample_size: 113.434007,
          covariance: "positive_case_weighted_unbiased_covariance_v1",
          warnings: [],
        },
      },
    };

    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);
    expect(navigation.defaultItemId).toBe("model_estimates");
    expect(table("outer_loadings")?.rows).toEqual([["competence", "COMP1", "0.812345"]]);
    expect(table("outer_weights")?.rows).toEqual([["competence", "COMP1", "0.412345"]]);
    expect(table("path_coefficients")?.rows).toEqual([["competence → satisfaction", "0.712345"]]);
    expect(table("r_squared")?.rows).toEqual([["satisfaction", "0.612345"]]);
    expect(table("wpls_weights")?.rows).toEqual([
      ["Case-weight column", "case_wt"],
      ["Weight sum", "211.550000"],
      ["Effective sample size", "113.434007"],
      ["Covariance estimator", "Positive case weighted unbiased covariance v1"],
    ]);
    expect(navigation.groups.find((group) => group.id === "quality_criteria")?.items.map((item) => item.id)).toEqual(["wpls_weights"]);
    const csv = tablesToCsv(navigation.tables);
    expect(csv).toContain("WPLS case-weight diagnostics");
    expect(csv).not.toMatch(/\bN\/?A\b/i);
  });

  it("opens finite-only CCA residual diagnostics by default using immutable model labels", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      id: "cca-composite-residual-run",
      name: "CCA composite residual diagnostics run",
      method: "CCA composite residual diagnostics",
      modelSnapshot: labelledModelSnapshot(),
      assessment: undefined,
      bootstrap: undefined,
      permutation: undefined,
      result: {
        ...base.result!,
        method_version: "cca_composite_residual_v1",
        cca: {
          method_version: "cca_composite_residual_v1",
          model: "recursive_standardized_composite_path_model_v1",
          correlations: [
            {
              left: "competence",
              right: "satisfaction",
              observed: 0.52,
              reproduced: 0.47,
              residual: 0.05,
              absolute_residual: 0.05,
            },
            {
              left: "likeability",
              right: "loyalty",
              observed: Number.NaN,
              reproduced: 0.31,
              residual: Number.NaN,
              absolute_residual: Number.NaN,
            },
          ],
          max_absolute_residual: 0.05,
          warnings: ["Do not turn this engine note into a p-value or fit classification."],
        },
      },
    };

    const navigation = buildNativeResultNavigation(run);
    const assessment = navigation.groups.find((group) => group.id === "assessment");
    const summary = navigation.tables.find((table) => table.id === "cca_residual_summary");
    const residuals = navigation.tables.find((table) => table.id === "cca_composite_residuals");

    expect(navigation.defaultItemId).toBe("cca_residual_summary");
    expect(assessment?.items.map((item) => item.id)).toEqual([
      "cca_residual_summary",
      "cca_composite_residuals",
    ]);
    expect(summary).toMatchObject({
      title: "Residual summary",
      warning: null,
      columns: ["Metric", "Value"],
      rows: [
        ["Model", "Recursive standardized composite path model"],
        ["Correlation pairs", "1"],
        ["Maximum absolute residual", "0.050000"],
      ],
    });
    expect(residuals).toMatchObject({
      title: "Composite residuals",
      warning: null,
      columns: ["Composite pair", "Observed correlation", "Reproduced correlation", "Residual", "Absolute residual"],
      rows: [["Technical Capability ↔ Customer Fulfilment", "0.520000", "0.470000", "0.050000", "0.050000"]],
    });
    expect(resultTableForItem(navigation, "cca_residual_summary")).toBe(summary);
    expect(navigation.tables.flatMap((table) => table.columns).join(" ")).not.toMatch(/threshold|p-value|classification/i);
    const csv = tablesToCsv(navigation.tables);
    expect(csv).toContain("Residual summary");
    expect(csv).toContain("Composite residuals");
    expect(csv).not.toMatch(/NaN|Infinity|N\/?A|fit classification|p-value/i);
  });

  it("does not fall back to uncorrected or placeholder method output when PLSc/WPLS payload rows are unavailable", () => {
    const base = completedSamplePlsRun();
    const result = base.result!;
    const run: AnalysisRun = {
      ...base,
      assessment: undefined,
      bootstrap: undefined,
      permutation: undefined,
      result: {
        ...result,
        method_version: "plsc_v2",
        plsc: {
          method_version: "plsc_v2",
          reliability_method_version: "dijkstra_henseler_rho_a_v1",
          tolerance: 1e-12,
          reliabilities: [{ construct: "competence", rho_a: Number.NaN }],
          construct_correlations: [{ left: "competence", right: "satisfaction", original: Number.NaN, corrected: Number.NaN }],
          corrected_paths: [],
          corrected_outer_loadings: [],
          corrected_r_squared: {},
          warnings: [],
        },
        wpls: {
          method_version: "wpls_case_weighted_v1",
          case_weight_column: "",
          weight_sum: Number.NaN,
          effective_sample_size: Number.NaN,
          covariance: "",
          warnings: [],
        },
      },
    };

    const tables = nativeResultTables(run);
    const ids = tables.map((table) => table.id);
    expect(ids).not.toContain("outer_loadings");
    expect(ids).not.toContain("outer_weights");
    expect(ids).not.toContain("plsc_reliability");
    expect(ids).not.toContain("plsc_correlations");
    expect(ids).not.toContain("wpls_weights");
    expect(tables.every((table) => table.rows.length > 0)).toBe(true);
    expect(tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");
  });

  it("keeps v1 prediction archives visibly legacy instead of relabeling them as current CVPAT", () => {
    const base = completedSamplePlsRun();
    const predictionRun: AnalysisRun = {
      ...base,
      id: "prediction-run",
      method: "Legacy construct-score prediction (v1)",
      result: base.result ? {
        ...base.result,
        predict: {
          method_version: "plspredict_holdout_v1",
          split: "deterministic_complete_case_modulo_4_test_rows",
          training_observations: 48,
          test_observations: 16,
          benchmark: "indicator_mean",
          warnings: [],
          targets: [{
            construct: "loyalty",
            predictor_count: 2,
            rmse_pls: 0.41,
            mae_pls: 0.32,
            rmse_benchmark: 0.55,
            mae_benchmark: 0.44,
            q_squared_predict: 0.28,
            rmse_lm: null,
            mae_lm: null,
            q_squared_predict_lm: null,
          }],
          repeated_kfold: {
            method_version: "plspredict_repeated_kfold_v1",
            folds: 5,
            repeats: 3,
            assignment: "deterministic_complete_case_index_multiplier_modulo_5",
            total_test_observations: 192,
            warnings: [],
            targets: [{
              construct: "loyalty",
              predictor_count: 2,
              rmse_pls: 0.43,
              mae_pls: 0.34,
              rmse_benchmark: 0.57,
              mae_benchmark: 0.46,
              q_squared_predict: 0.25,
              rmse_lm: 0.48,
              mae_lm: 0.38,
              q_squared_predict_lm: 0.17,
            }],
            cvpat: [{
              target: "loyalty",
              comparison: "PLS versus indicator mean",
              loss: "squared_error",
              mean_loss_difference: -0.07,
              standard_error: 0.02,
              t_statistic: -3.5,
              p_value_two_sided: 0.0005,
              observations: 192,
              preferred_model: "PLS",
              warning: null,
            }],
          },
        },
      } : undefined,
    };

    const navigation = buildNativeResultNavigation(predictionRun);
    const prediction = navigation.groups.find((group) => group.id === "prediction");
    expect(navigation.defaultItemId).toBe("plspredict_holdout");
    expect(prediction?.items.map((item) => item.id)).toEqual([
      "plspredict_holdout",
      "plspredict_split",
      "plspredict_repeated_kfold",
      "plspredict_repeated_kfold_plan",
      "cvpat",
    ]);
    const holdout = navigation.tables.find((table) => table.id === "plspredict_holdout");
    expect(holdout?.title).toBe("Legacy construct-score holdout metrics (v1)");
    expect(holdout?.warning).toContain("not current indicator-level PLSpredict or CVPAT");
    expect(holdout?.columns).not.toEqual(expect.arrayContaining(["RMSE LM", "MAE LM", "Q² LM"]));
    const repeated = navigation.tables.find((table) => table.id === "plspredict_repeated_kfold");
    expect(repeated?.title).toBe("Legacy construct-score repeated-fold metrics (v1)");
    expect(repeated?.columns).toEqual(expect.arrayContaining(["LM RMSE", "LM MAE", "Q²_predict"]));
    expect(navigation.tables.find((table) => table.id === "cvpat")?.title).toBe("Legacy paired loss diagnostics (v1)");
    expect(navigation.tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");
  });

  it("renders exact v2 indicator prediction, null MAPE, CVPAT benchmark status, and assignment provenance", () => {
    const base = completedSamplePlsRun();
    const metric = (rmse: number, mae: number, mapePercent: number | null, mapeObservations: number) => ({
      observations: 64,
      squared_error_sum: rmse * rmse * 64,
      absolute_error_sum: mae * 64,
      rmse,
      mae,
      absolute_percentage_error_sum: mapeObservations === 0 || mapePercent === null ? null : mapePercent * mapeObservations / 100,
      mape_observations: mapeObservations,
      mape_percent: mapePercent,
    });
    const constructTarget = {
      construct: "loyalty",
      predictor_count: 2,
      rmse_pls: 0.41,
      mae_pls: 0.32,
      rmse_benchmark: 0.55,
      mae_benchmark: 0.44,
      q_squared_predict: 0.28,
      rmse_lm: null,
      mae_lm: null,
      q_squared_predict_lm: null,
    };
    const indicatorTarget = {
      construct: "loyalty",
      indicator: "loy1",
      predictor_scope: "earliest_antecedent_indicators",
      predictor_count: 2,
      pls: metric(0.41, 0.32, null, 0),
      indicator_average: metric(0.55, 0.44, 12.5, 64),
      linear_model: { status: "unavailable" as const, metrics: null, reason: "linear_model_rank_deficient" },
      q_squared_predict: 0.28,
    };
    const digest = `sha256:${"a".repeat(64)}`;
    const predictionRun: AnalysisRun = {
      ...base,
      id: "prediction-v2-run",
      method: "PLSpredict / CVPAT",
      result: {
        ...base.result!,
        used_observations: 64,
        predict: {
          method_version: "plspredict_indicator_v2",
          split: "deterministic_complete_case_modulo_4_test_rows",
          training_observations: 48,
          test_observations: 16,
          benchmark: "indicator_average",
          targets: [constructTarget],
          indicator_targets: [indicatorTarget],
          repeated_kfold: {
            method_version: "plspredict_repeated_kfold_indicator_v2",
            folds: 10,
            repeats: 10,
            assignment: "seeded_chacha20_balanced_folds",
            assignment_digest: digest,
            seed: 20_260_811,
            total_test_observations: 640,
            targets: [constructTarget],
            indicator_targets: [indicatorTarget],
            cvpat_benchmark_assessments: [
              {
                method_version: "cvpat_indicator_benchmarks_v2",
                comparison_kind: "benchmark_assessment",
                target_scope: "all_endogenous_indicators",
                benchmark: "indicator_average",
                loss: "mean_squared_error_across_indicators_per_observation",
                alternative: "pls_loss_less_than_benchmark",
                confidence_level: 0.95,
                mean_loss_pls: 0.1681,
                mean_loss_benchmark: 0.3025,
                mean_loss_difference: -0.1344,
                standard_error: 0.04,
                t_statistic: -3.36,
                p_value_one_sided: 0.001,
                confidence_interval_lower: -0.2,
                confidence_interval_upper: -0.0688,
                observations: 64,
                indicator_count: 1,
                status: "available",
                preferred_model: "pls_sem",
                reason: null,
              },
              {
                method_version: "cvpat_indicator_benchmarks_v2",
                comparison_kind: "benchmark_assessment",
                target_scope: "all_endogenous_indicators",
                benchmark: "linear_model",
                loss: "mean_squared_error_across_indicators_per_observation",
                alternative: "pls_loss_less_than_benchmark",
                confidence_level: 0.95,
                mean_loss_pls: null,
                mean_loss_benchmark: null,
                mean_loss_difference: null,
                standard_error: null,
                t_statistic: null,
                p_value_one_sided: null,
                confidence_interval_lower: null,
                confidence_interval_upper: null,
                observations: 64,
                indicator_count: 1,
                status: "benchmark_unavailable",
                preferred_model: null,
                reason: "linear_model_unavailable_for_one_or_more_indicators",
              },
            ],
            warnings: [],
          },
          warnings: [],
        },
      },
    };

    const navigation = buildNativeResultNavigation(predictionRun);
    const prediction = navigation.groups.find((group) => group.id === "prediction");
    expect(navigation.defaultItemId).toBe("plspredict_indicator_summary");
    expect(prediction?.items.map((item) => item.id)).toEqual([
      "plspredict_indicator_summary",
      "cvpat_benchmark_assessment",
      "plspredict_validation_plan",
      "plspredict_construct_summary",
      "plspredict_holdout_indicator_summary",
      "plspredict_holdout_construct_summary",
      "plspredict_holdout_split",
    ]);
    const indicators = navigation.tables.find((table) => table.id === "plspredict_indicator_summary")!;
    expect(indicators.rows).toHaveLength(1);
    expect(indicators.rows[0][indicators.columns.indexOf("PLS-SEM MAPE (%)")]).toBe("");
    expect(indicators.rows[0][indicators.columns.indexOf("LM benchmark")]).toContain("Unavailable — Linear model rank deficient");
    const cvpat = navigation.tables.find((table) => table.id === "cvpat_benchmark_assessment")!;
    expect(cvpat.rows).toHaveLength(2);
    expect(cvpat.columns).toContain("Mean loss difference (PLS-SEM − benchmark)");
    expect(cvpat.columns).not.toEqual(expect.arrayContaining(["Model A", "Model B"]));
    expect(cvpat.rows[0][cvpat.columns.indexOf("Supported conclusion")]).toBe("PLS-SEM significantly better");
    expect(cvpat.rows[1][cvpat.columns.indexOf("Supported conclusion")]).toBe("");
    expect(cvpat.rows[1][cvpat.columns.indexOf("Status")]).toBe("Benchmark unavailable");
    expect(navigation.tables.find((table) => table.id === "plspredict_validation_plan")?.rows[0]).toContain(digest);
    expect(navigation.tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");

    for (const repeatedPatch of [
      { method_version: "plspredict_repeated_kfold_indicator_v2+future" },
      { assignment_digest: "sha256:BAD" },
    ]) {
      const malformed: AnalysisRun = {
        ...predictionRun,
        result: {
          ...predictionRun.result!,
          predict: {
            ...predictionRun.result!.predict!,
            repeated_kfold: { ...predictionRun.result!.predict!.repeated_kfold!, ...repeatedPatch },
          },
        },
      };
      expect(nativeResultTables(malformed).map((table) => table.id)).not.toContain("plspredict_indicator_summary");
    }
  });

  it("keeps empty and unavailable assessment capabilities out of the tree", () => {
    const base = completedSamplePlsRun();
    const sparse: AnalysisRun = {
      ...base,
      assessment: base.assessment ? {
        ...base.assessment,
        construct_quality: [{
          construct: "A",
          cronbach_alpha: null,
          rho_a: null,
          rho_c: null,
          ave: null,
        }],
        cross_loadings: [],
        fornell_larcker: { constructs: ["A"], values: [[null]] },
        htmt_plus: null,
        htmt_original: null,
        structural_quality: [],
        structural_vif: [{ target_construct: "B", predictor_construct: "A", vif: null }],
        formative_indicator_vif: [],
        f_squared: [{ source_construct: "A", target_construct: "B", included_r_squared: 0.2, excluded_r_squared: null, f_squared: null }],
        model_fit: undefined,
        blindfolding: undefined,
      } : undefined,
      bootstrap: undefined,
      permutation: undefined,
    };

    const tableIds = nativeResultTables(sparse).map((table) => table.id);
    expect(tableIds).not.toEqual(expect.arrayContaining([
      "construct_reliability",
      "cross_loadings",
      "fornell_larcker",
      "htmt_plus",
      "structural_vif",
      "formative_indicator_vif",
      "f_squared",
      "model_fit",
      "blindfolding",
      "bootstrap_percentile",
    ]));
  });

  it("projects disjoint two-stage HOC evidence without leaking generated score indicators", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      id: "two-stage-hoc-run",
      name: "Higher-order PLS-SEM run",
      method: "PLS-SEM Algorithm",
      bootstrap: undefined,
      permutation: undefined,
      modelSnapshot: {
        nodes: [
          { id: "x", type: "construct", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "X", mode: "reflective", indicators: ["x1"] } },
          { id: "z", type: "construct", position: { x: 0, y: 120 }, data: { label: "Resources", shortName: "Z", mode: "reflective", indicators: ["z1"] } },
          { id: "hoc", type: "construct", position: { x: 220, y: 60 }, data: { label: "Organizational strength", shortName: "HOC", mode: "reflective", indicators: [], semantic: "higher_order", higherOrder: { id: "hoc", components: ["x", "z"], method: "two_stage", stage_one_recipe: null } } },
          { id: "y", type: "construct", position: { x: 440, y: 60 }, data: { label: "Performance", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
        ],
        edges: [{ id: "hoc-y", source: "hoc", target: "y" }],
      },
      result: {
        ...base.result!,
        paths: [{ source: "hoc", target: "y", coefficient: 0.74 }],
        effects: [{ source: "hoc", target: "y", direct: 0.74, indirect: 0, total: 0.74 }],
        r_squared: { y: 0.5476 },
        outer_estimates: [
          { construct: "x", indicator: "x1", loading: 0.91, weight: 0.72 },
          { construct: "z", indicator: "z1", loading: 0.89, weight: 0.7 },
          { construct: "hoc", indicator: "__qpls_hoc_hoc_x", loading: 0.93, weight: 0.58 },
          { construct: "hoc", indicator: "__qpls_hoc_hoc_z", loading: 0.9, weight: 0.55 },
          { construct: "y", indicator: "y1", loading: 0.94, weight: 0.76 },
        ],
        mediation: undefined,
      },
      assessment: base.assessment ? {
        ...base.assessment,
        construct_quality: [
          { construct: "x", cronbach_alpha: 0.8, rho_a: 0.81, rho_c: 0.88, ave: 0.7 },
          { construct: "z", cronbach_alpha: 0.79, rho_a: 0.8, rho_c: 0.87, ave: 0.68 },
          { construct: "hoc", cronbach_alpha: 0.84, rho_a: 0.85, rho_c: 0.9, ave: 0.73 },
          { construct: "y", cronbach_alpha: 0.82, rho_a: 0.83, rho_c: 0.89, ave: 0.72 },
        ],
        cross_loadings: [
          { indicator: "x1", assigned_construct: "x", construct: "x", loading: 0.91 },
          { indicator: "__qpls_hoc_hoc_x", assigned_construct: "hoc", construct: "hoc", loading: 0.93 },
          { indicator: "__qpls_hoc_hoc_z", assigned_construct: "hoc", construct: "hoc", loading: 0.9 },
          { indicator: "y1", assigned_construct: "y", construct: "y", loading: 0.94 },
        ],
        fornell_larcker: { constructs: ["x", "z", "hoc", "y"], values: [
          [0.84, 0.3, 0.72, 0.42],
          [0.3, 0.82, 0.69, 0.38],
          [0.72, 0.69, 0.85, 0.74],
          [0.42, 0.38, 0.74, 0.86],
        ] },
        formative_indicator_vif: [{ construct: "hoc", indicator: "__qpls_hoc_hoc_x", vif: 1.4 }],
      } : undefined,
    };

    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);
    expect(navigation.defaultItemId).toBe("hoc_component_relationships");
    expect(navigation.groups.find((group) => group.id === "higher_order")?.items.map((item) => item.id)).toEqual([
      "hoc_component_relationships",
      "hoc_structural_paths",
      "hoc_scope",
    ]);
    expect(table("hoc_component_relationships")?.rows).toEqual([
      ["Organizational strength", "Capability", "Disjoint two-stage", "0.930000", "0.580000"],
      ["Organizational strength", "Resources", "Disjoint two-stage", "0.900000", "0.550000"],
    ]);
    expect(table("hoc_structural_paths")?.rows).toEqual([["Organizational strength → Performance", "0.740000"]]);
    expect(table("outer_loadings")?.rows).toEqual([
      ["Capability", "x1", "0.910000"],
      ["Resources", "z1", "0.890000"],
      ["Performance", "y1", "0.940000"],
    ]);
    expect(table("construct_reliability")?.rows.flat()).not.toContain("Organizational strength");
    const rendered = navigation.tables.flatMap((candidate) => [candidate.title, ...candidate.columns, ...candidate.rows.flat()]).join(" ");
    expect(rendered).not.toContain("__qpls_hoc_");
    expect(rendered).not.toContain("N/A");
    expect(tablesToCsv(navigation.tables)).toContain("Higher-order component relationships");
  });

  it("classifies moderation and control inference once and hides generated product indicators", () => {
    const base = completedSamplePlsRun();
    const productKey = JSON.stringify(["path", ["xm", "y"]]);
    const controlKey = JSON.stringify(["path", ["age", "y"]]);
    const ordinaryKey = JSON.stringify(["path", ["x", "y"]]);
    const percentile = (parameter: string, original: number) => ({ parameter, original, bootstrap_mean: original + 0.01, bias: 0.01, standard_error: 0.04, lower: original - 0.08, upper: original + 0.08, usable_replicates: 999, t_statistic: Math.abs(original / 0.04), p_value_two_sided: 0.01 });
    const bca = (parameter: string) => ({ parameter, bias_correction: 0.01, acceleration: 0.02, lower: 0.1, upper: 0.4, unavailable_reason: null });
    const studentized = (parameter: string, original: number) => ({ parameter, original, outer_standard_error: 0.04, outer_scale: 0.04, usable_primary_replicates: 999, lower_pivot: -1.96, upper_pivot: 1.96, lower: original - 0.08, upper: original + 0.08, unavailable_reason: null });
    const randomization = (parameter: string, original: number) => ({ parameter, original, exceedances: 9, p_value_two_sided: 0.01, permutations: 999 });
    const run: AnalysisRun = {
      ...base,
      modelSnapshot: labelledModelSnapshot({ x: "Predictor", m: "Moderator", y: "Outcome", xm: "X by M", age: "Age" }),
      result: {
        ...base.result!,
        mediation: undefined,
        paths: [
          { source: "x", target: "y", coefficient: 0.31 },
          { source: "m", target: "y", coefficient: 0.22 },
          { source: "xm", target: "y", coefficient: 0.27 },
          { source: "age", target: "y", coefficient: -0.08 },
        ],
        effects: [
          { source: "x", target: "y", direct: 0.31, indirect: 0, total: 0.31 },
          { source: "m", target: "y", direct: 0.22, indirect: 0, total: 0.22 },
          { source: "xm", target: "y", direct: 0.27, indirect: 0, total: 0.27 },
          { source: "age", target: "y", direct: -0.08, indirect: 0, total: -0.08 },
        ],
        outer_estimates: [
          { construct: "x", indicator: "x1", weight: 0.8, loading: 0.9 },
          { construct: "xm", indicator: "__qpls_interaction_xm", weight: 1, loading: 1 },
        ],
        control_estimates: [{ source: "age", target: "y", label: "Age covariate", coefficient: -0.08 }],
        moderation: {
          method_version: "pls_two_stage_moderation_v1",
          moderator_score_levels: [-1, 0, 1],
          estimates: [{
            interaction: "x_by_m_to_y",
            predictor: "x",
            moderator: "m",
            product_construct: "xm",
            outcome: "y",
            predictor_main_effect: 0.31,
            moderator_main_effect: 0.22,
            interaction_effect: 0.27,
            simple_slopes: [
              { moderator_score: -1, effect: 0.04 },
              { moderator_score: 0, effect: 0.31 },
              { moderator_score: 1, effect: 0.58 },
            ],
            warning: null,
          }],
          warnings: [],
        },
      },
      assessment: base.assessment ? {
        ...base.assessment,
        construct_quality: [
          { construct: "x", cronbach_alpha: 0.8, rho_c: 0.85, ave: 0.7, rho_a: 0.82 },
          { construct: "xm", cronbach_alpha: null, rho_c: null, ave: null, rho_a: null },
        ],
        cross_loadings: [
          { indicator: "x1", assigned_construct: "x", construct: "x", loading: 0.9 },
          { indicator: "__qpls_interaction_xm", assigned_construct: "xm", construct: "xm", loading: 1 },
        ],
        fornell_larcker: { constructs: ["x", "xm"], values: [[0.84, 0.2], [0.2, 1]] },
        formative_indicator_vif: [{ construct: "xm", indicator: "__qpls_interaction_xm", vif: 1 }],
      } : undefined,
      bootstrap: base.bootstrap ? {
        ...base.bootstrap,
        percentile: { ...base.bootstrap.percentile, parameters: [percentile(productKey, 0.27), percentile(controlKey, -0.08), percentile(ordinaryKey, 0.31)] },
        bca: base.bootstrap.bca ? { ...base.bootstrap.bca, parameters: [bca(productKey), bca(controlKey), bca(ordinaryKey)] } : null,
        studentized: base.bootstrap.studentized ? { ...base.bootstrap.studentized, parameters: [studentized(productKey, 0.27), studentized(controlKey, -0.08), studentized(ordinaryKey, 0.31)] } : null,
      } : undefined,
      permutation: {
        method_version: "qpls-permutation-v1",
        plan: { permutations: 999, master_seed: 7, operation: "pls_permutation" },
        parameters: [randomization(productKey, 0.27), randomization(controlKey, -0.08), randomization(ordinaryKey, 0.31)],
      },
    };

    const navigation = buildNativeResultNavigation(run);
    const ids = navigation.tables.map((table) => table.id);
    expect(navigation.groups.find((group) => group.id === "moderation")?.items.map((item) => item.id)).toEqual([
      "moderation_effects",
      "moderation_simple_slopes",
      "moderation_bootstrap",
      "moderation_bca",
      "moderation_studentized",
      "moderation_randomization",
    ]);
    expect(ids).toEqual(expect.arrayContaining(["control_effects", "control_bootstrap", "control_bca", "control_studentized", "control_randomization"]));
    for (const genericId of ["bootstrap_percentile", "bootstrap_bca", "bootstrap_studentized", "permutation"]) {
      const generic = navigation.tables.find((table) => table.id === genericId)!;
      expect(generic.rows).toHaveLength(1);
      expect(generic.rows.flat().join(" ")).toContain("Predictor");
      expect(generic.rows.flat().join(" ")).not.toMatch(/X by M|Age covariate/);
    }
    expect(navigation.tables.flatMap((table) => table.rows).flat().join(" ")).not.toContain("__qpls_interaction_");
    expect(navigation.tables.flatMap((table) => table.rows).flat()).not.toContain("N/A");
    expect(nativeModerationPlot(run)).toEqual(expect.objectContaining({
      title: "Predictor × Moderator → Outcome",
      slopes: [
        { moderatorScore: -1, effect: 0.04, label: "Moderator = -1.000000" },
        { moderatorScore: 0, effect: 0.31, label: "Moderator = 0.000000" },
        { moderatorScore: 1, effect: 0.58, label: "Moderator = 1.000000" },
      ],
    }));
  });

  it("returns no navigation for failed or payload-free runs", () => {
    const complete = completedSamplePlsRun();
    expect(buildNativeResultNavigation({ ...complete, status: "failed" })).toEqual({
      runId: null,
      defaultItemId: null,
      groups: [],
      tables: [],
    });
    expect(nativeResultTables({ ...complete, result: undefined })).toEqual([]);
  });

  it("projects only exact GSCA ALS v2 output into dedicated, non-inferential tables", () => {
    const run = completedGscaRun();
    expect(nativeGscaResultProjection(run)).toMatchObject({
      methodVersion: "gsca_als_v2",
      algorithmVersion: "alternating_least_squares_v1",
      usedObservations: 140,
      omittedObservations: 0,
      constructModes: { g: "formative", h: "reflective" },
    });

    const navigation = buildNativeResultNavigation(run);
    expect(navigation.defaultItemId).toBe("gsca_fit");
    expect(navigation.groups.map((group) => group.id)).toEqual(["graphical", "gsca_component_model"]);
    expect(navigation.groups.find((group) => group.id === "gsca_component_model")?.items.map((item) => item.id)).toEqual([
      "gsca_fit",
      "gsca_paths",
      "gsca_r_squared",
      "gsca_loadings",
      "gsca_weights",
      "gsca_scope",
    ]);
    expect(navigation.tables.map((table) => table.id)).toEqual([
      "gsca_fit",
      "gsca_paths",
      "gsca_r_squared",
      "gsca_loadings",
      "gsca_weights",
      "gsca_scope",
    ]);
    expect(navigation.tables.find((table) => table.id === "gsca_fit")?.rows).toEqual(expect.arrayContaining([
      ["Global FIT", "0.369258"],
      ["GFI", "0.647993"],
      ["Converged", "Yes"],
      ["ALS iterations", "4"],
    ]));
    expect(navigation.tables.find((table) => table.id === "gsca_paths")?.rows).toEqual([
      ["Reflective Outcome ← Formative Capability", "0.770947"],
    ]);
    expect(navigation.tables.find((table) => table.id === "gsca_loadings")?.rows).toEqual(expect.arrayContaining([
      ["Formative Capability", "g1", "Formative", "0.997413"],
      ["Reflective Outcome", "h2", "Reflective", "0.997890"],
    ]));
    const csv = tablesToCsv(navigation.tables);
    expect(csv).not.toContain("N/A");
    expect(csv).not.toMatch(/bootstrap interval|percentile|p-value/i);
    expect(csv).toContain("Point estimates only; no bootstrap or permutation inference");
  });

  it("rejects stale, internally inconsistent, or inference-bearing GSCA payloads", () => {
    const stale = completedGscaRun();
    stale.provenance!.method_version = "gsca_v1";
    expect(nativeGscaResultProjection(stale)).toBeNull();
    expect(nativeResultTables(stale)).toEqual([]);

    const alteredFit = completedGscaRun();
    alteredFit.result!.gsca!.fit += 0.01;
    expect(nativeGscaResultProjection(alteredFit)).toBeNull();

    const alteredLoading = completedGscaRun();
    alteredLoading.result!.gsca!.loadings[4].loading -= 0.02;
    expect(nativeGscaResultProjection(alteredLoading)).toBeNull();

    const fabricatedInference = completedGscaRun();
    fabricatedInference.result!.gsca!.bootstrap_intervals.push({ parameter: "path:g:h", original: 0.77, lower_percentile: 0.7, upper_percentile: 0.84 });
    expect(nativeGscaResultProjection(fabricatedInference)).toBeNull();

    const staleAssessment = completedGscaRun();
    staleAssessment.assessment!.warnings = ["PLS assessment available."];
    expect(nativeGscaResultProjection(staleAssessment)).toBeNull();
  });

  it("projects only exact current CB-SEM/CFA payloads into method-specific tables and standardized diagrams", () => {
    const run = completedCbsemRun("sem");
    const projection = nativeCbsemResultProjection(run);
    expect(projection).toMatchObject({ methodVersion: "cbsem_ml_v1", modelType: "sem" });

    const navigation = buildNativeResultNavigation(run);
    expect(navigation.defaultItemId).toBe("cbsem_fit");
    expect(navigation.groups.find((group) => group.id === "covariance_sem")?.items.map((item) => item.id)).toEqual([
      "cbsem_fit",
      "cbsem_standardized_parameters",
      "cbsem_unstandardized_parameters",
      "cbsem_residual_correlations",
      "cbsem_residual_covariances",
      "cbsem_implied_covariances",
      "cbsem_modification_diagnostics",
      "cbsem_scope",
    ]);
    expect(navigation.tables.map((table) => table.id)).toEqual([
      "cbsem_fit",
      "cbsem_standardized_parameters",
      "cbsem_unstandardized_parameters",
      "cbsem_residual_correlations",
      "cbsem_residual_covariances",
      "cbsem_implied_covariances",
      "cbsem_modification_diagnostics",
      "cbsem_scope",
    ]);
    expect(navigation.tables.find((table) => table.id === "cbsem_fit")?.rows).toEqual(expect.arrayContaining([
      ["CFI", "0.982000"],
      ["RMSEA", "0.072000"],
      ["SRMR", "0.031000"],
    ]));
    expect(navigation.tables.find((table) => table.id === "cbsem_scope")?.rows).toEqual(expect.arrayContaining([
      ["Estimator", "Maximum likelihood"],
      ["Analyzed observations", "120"],
      ["Unsupported in this workflow", expect.stringContaining("robust/ordinal/FIML")],
    ]));
    expect(tablesToCsv(navigation.tables)).not.toContain("N/A");
    expect(navigation.tables.map((table) => table.id)).not.toEqual(expect.arrayContaining(["path_coefficients", "outer_loadings", "construct_reliability"]));

    const diagramRun = nativeCbsemDiagramRun(run);
    expect(diagramRun.result?.paths).toEqual([{ source: "x", target: "y", coefficient: 0.56 }]);
    expect(diagramRun.result?.outer_estimates).toEqual(expect.arrayContaining([
      expect.objectContaining({ construct: "x", indicator: "x1", loading: 1 }),
      expect.objectContaining({ construct: "y", indicator: "y2", loading: 0.9 }),
    ]));
    expect(diagramRun.result?.r_squared).toEqual({ y: 0.36 });
    expect(nativeCbsemResultProjection({
      ...run,
      provenance: { ...run.provenance!, method_version: "cbsem_ml_v1" },
    })).toBeNull();
  });

  it("resolves table items without treating the diagram as a table", () => {
    const navigation = buildNativeResultNavigation(completedSamplePlsRun());
    expect(resultTableForItem(navigation, "direct_effects")?.title).toBe("Direct effects");
    expect(resultTableForItem(navigation, "model_estimates")).toBeUndefined();
  });
});
