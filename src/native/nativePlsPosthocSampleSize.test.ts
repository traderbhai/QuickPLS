import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import {
  nativePlsPosthocMinimumSampleSizeProjection,
  nativeResultTables,
} from "./nativeResults";
import { canonicalResultDocumentFromAnalysisRunV2 } from "./nativeCanonicalResultDocumentV2";
import { previewNativeCanonicalSemanticExportV2 } from "./nativeCanonicalSemanticExportV2";

function completedPosthocRun(): AnalysisRun {
  const run = completedSamplePlsRun();
  return {
    ...run,
    result: {
      ...run.result!,
      posthoc_minimum_sample_size: {
        method_version: "inverse_square_root_posthoc_v1",
        alpha: 0.05,
        power: 0.80,
        test: "directional",
        inverse_square_root_constant: 2.486,
        driver_source: "competence",
        driver_target: "loyalty",
        minimum_absolute_path_coefficient: 0.116,
        technically_required_sample_size: Math.ceil((2.486 / 0.116) ** 2),
        analytical_sample_size: 5,
        meets_technical_requirement: false,
        status: "available",
        caution: "This retrospective technical-power diagnostic does not establish population representativeness, causal validity, or an a-priori recruitment target.",
      },
    },
  };
}

function completedInferenceAwarePosthocRun(): AnalysisRun {
  const run = completedSamplePlsRun();
  run.provenance = {
    recipe_id: "recipe-posthoc-v2",
    dataset_fingerprint: `sha256:${"a".repeat(64)}`,
    method: "pls_pm",
    method_version: run.result!.method_version,
    engine_version: "qpls-estimation-test",
    seed: run.seed,
    settings: {
      method: "pls_pm",
      weighting_scheme: "path",
      tolerance: 1e-7,
      max_iterations: 3_000,
      bootstrap_samples: 999,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      seed: run.seed,
      workers: 4,
      confidence_level: 0.95,
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      case_weight_column: null,
    },
    started_at: "2026-08-14T00:00:00.000Z",
    completed_at: "2026-08-14T00:00:01.000Z",
  };
  run.bootstrap!.percentile.parameters = run.result!.paths.map((path) => ({
    parameter: JSON.stringify(["path", [path.source, path.target]]),
    original: path.coefficient,
    bootstrap_mean: path.coefficient,
    bias: 0,
    standard_error: 0.05,
    lower: path.coefficient - 0.1,
    upper: path.coefficient + 0.1,
    usable_replicates: 999,
    t_statistic: path.coefficient / 0.05,
    p_value_two_sided: path.source === "competence" && path.target === "loyalty" ? 0.20 : 0.01,
  }));
  const driver = run.result!.paths.find((path) => (
    path.source === "likeability" && path.target === "loyalty"
  ))!;
  run.result!.posthoc_minimum_sample_size = {
    method_version: "inverse_square_root_posthoc_v2",
    alpha: 0.05,
    power: 0.80,
    test: "directional",
    inverse_square_root_constant: 2.486,
    selection_rule: "smallest_absolute_statistically_significant_structural_path",
    significance_source: "pls_bootstrap_normal_reference_two_sided",
    significance_alpha: 0.05,
    eligible_path_count: run.result!.paths.length,
    significant_path_count: run.result!.paths.length - 1,
    driver_source: driver.source,
    driver_target: driver.target,
    driver_p_value_two_sided: 0.01,
    minimum_absolute_path_coefficient: Math.abs(driver.coefficient),
    technically_required_sample_size: Math.ceil((2.486 / Math.abs(driver.coefficient)) ** 2),
    analytical_sample_size: run.result!.used_observations,
    meets_technical_requirement: false,
    status: "available",
    caution: "Retrospective technical diagnostic with complete bootstrap significance selection.",
  };
  return run;
}

describe("PLS posthoc minimum sample size", () => {
  it("recomputes the weakest path and renders the documented result", () => {
    const run = completedPosthocRun();
    const projection = nativePlsPosthocMinimumSampleSizeProjection(run);
    expect(projection?.technically_required_sample_size).toBe(460);
    expect(projection?.meets_technical_requirement).toBe(false);

    const table = nativeResultTables(run).find((candidate) => candidate.id === "posthoc_minimum_sample_size");
    expect(table?.title).toBe("Post-hoc minimum sample size");
    expect(table?.status).toBe("experimental");
    expect(table?.rows).toEqual(expect.arrayContaining([
      ["Technically required sample size", "460"],
      ["Analytical sample size", "5"],
      ["Technical requirement", "Not met"],
      ["Formula assumptions", "5% significance, 80% power, directional inverse-square-root test"],
    ]));
  });

  it("rejects coordinated display tampering instead of rendering it", () => {
    const run = completedPosthocRun();
    run.result!.posthoc_minimum_sample_size!.technically_required_sample_size = 5;
    run.result!.posthoc_minimum_sample_size!.meets_technical_requirement = true;

    expect(nativePlsPosthocMinimumSampleSizeProjection(run)).toBeNull();
    expect(nativeResultTables(run)).toEqual([]);
  });

  it("keeps historical results without the new optional field readable", () => {
    const run = completedSamplePlsRun();

    expect(nativePlsPosthocMinimumSampleSizeProjection(run)).toBeNull();
    expect(nativeResultTables(run).some((table) => table.id === "direct_effects")).toBe(true);
  });

  it("selects the smallest significant path from complete linked bootstrap inference", () => {
    const run = completedInferenceAwarePosthocRun();

    const projection = nativePlsPosthocMinimumSampleSizeProjection(run);
    expect(projection?.driver_source).toBe("likeability");
    expect(projection?.technically_required_sample_size).toBe(209);
    expect(nativeResultTables(run).find((table) => table.id === "posthoc_minimum_sample_size")?.status)
      .toBe("validated");
    expect(nativeResultTables(run).find((table) => table.id === "posthoc_minimum_sample_size")?.rows)
      .toEqual(expect.arrayContaining([
        ["Driving path", "likeability → loyalty"],
        ["Bootstrap p value (two-sided)", "0.010000"],
        ["Driver selection", "Smallest absolute path with two-sided normal-reference bootstrap p ≤ 0.05"],
      ]));
  });

  it("keeps formula direction separate from the two-sided driver-selection boundary", () => {
    const wrongFormulaDirection = completedInferenceAwarePosthocRun();
    (wrongFormulaDirection.result!.posthoc_minimum_sample_size as unknown as { test: string }).test = "two_sided";
    expect(nativePlsPosthocMinimumSampleSizeProjection(wrongFormulaDirection)).toBeNull();

    const wrongSelectionDirection = completedInferenceAwarePosthocRun();
    (wrongSelectionDirection.result!.posthoc_minimum_sample_size as unknown as { significance_source: string }).significance_source = "pls_bootstrap_normal_reference_one_sided";
    expect(nativePlsPosthocMinimumSampleSizeProjection(wrongSelectionDirection)).toBeNull();
  });

  it("treats the two-sided p = 0.05 selection boundary as inclusive", () => {
    const atBoundary = completedInferenceAwarePosthocRun();
    const path = atBoundary.result!.paths.find((candidate) => (
      candidate.source === "competence" && candidate.target === "loyalty"
    ))!;
    const parameter = atBoundary.bootstrap!.percentile.parameters.find((candidate) => (
      candidate.parameter === JSON.stringify(["path", [path.source, path.target]])
    ))!;
    parameter.p_value_two_sided = 0.05;
    const stored = atBoundary.result!.posthoc_minimum_sample_size!;
    stored.driver_source = path.source;
    stored.driver_target = path.target;
    stored.driver_p_value_two_sided = 0.05;
    stored.minimum_absolute_path_coefficient = Math.abs(path.coefficient);
    stored.technically_required_sample_size = Math.ceil((2.486 / Math.abs(path.coefficient)) ** 2);
    stored.significant_path_count = atBoundary.result!.paths.length;
    stored.meets_technical_requirement = false;

    expect(nativePlsPosthocMinimumSampleSizeProjection(atBoundary)?.driver_source).toBe("competence");

    const justAbove = completedInferenceAwarePosthocRun();
    justAbove.bootstrap!.percentile.parameters.find((candidate) => (
      candidate.parameter === JSON.stringify(["path", [path.source, path.target]])
    ))!.p_value_two_sided = 0.05000000000000001;
    expect(nativePlsPosthocMinimumSampleSizeProjection(justAbove)?.driver_source).toBe("likeability");
  });

  it("renders no-significant, no-path, and zero-path states without fabricating a number", () => {
    const noSignificant = completedInferenceAwarePosthocRun();
    for (const parameter of noSignificant.bootstrap!.percentile.parameters) {
      parameter.p_value_two_sided = 0.051;
    }
    Object.assign(noSignificant.result!.posthoc_minimum_sample_size!, {
      significant_path_count: 0,
      driver_source: null,
      driver_target: null,
      driver_p_value_two_sided: null,
      minimum_absolute_path_coefficient: null,
      technically_required_sample_size: null,
      meets_technical_requirement: null,
      status: "no_statistically_significant_path",
    });
    expect(nativePlsPosthocMinimumSampleSizeProjection(noSignificant)?.status)
      .toBe("no_statistically_significant_path");

    const noPath = completedInferenceAwarePosthocRun();
    noPath.result!.paths = [];
    noPath.bootstrap!.percentile.parameters = [];
    Object.assign(noPath.result!.posthoc_minimum_sample_size!, {
      significance_source: null,
      significance_alpha: null,
      eligible_path_count: 0,
      significant_path_count: null,
      driver_source: null,
      driver_target: null,
      driver_p_value_two_sided: null,
      minimum_absolute_path_coefficient: null,
      technically_required_sample_size: null,
      meets_technical_requirement: null,
      status: "not_applicable_no_structural_path",
    });
    expect(nativePlsPosthocMinimumSampleSizeProjection(noPath)?.status)
      .toBe("not_applicable_no_structural_path");

    const zeroPath = completedInferenceAwarePosthocRun();
    const zero = zeroPath.result!.paths.find((candidate) => (
      candidate.source === "likeability" && candidate.target === "loyalty"
    ))!;
    zero.coefficient = 0;
    zeroPath.bootstrap!.percentile.parameters.find((parameter) => (
      parameter.parameter === JSON.stringify(["path", [zero.source, zero.target]])
    ))!.original = 0;
    Object.assign(zeroPath.result!.posthoc_minimum_sample_size!, {
      minimum_absolute_path_coefficient: 0,
      technically_required_sample_size: null,
      meets_technical_requirement: null,
      status: "undefined_zero_path",
    });
    expect(nativePlsPosthocMinimumSampleSizeProjection(zeroPath)?.status)
      .toBe("undefined_zero_path");
  });

  it("rejects nonfinite path payloads even when unavailable metadata is coordinated", () => {
    const run = completedInferenceAwarePosthocRun();
    const path = run.result!.paths[0];
    path.coefficient = Number.POSITIVE_INFINITY;
    run.bootstrap!.percentile.parameters[0].original = Number.POSITIVE_INFINITY;
    Object.assign(run.result!.posthoc_minimum_sample_size!, {
      significant_path_count: null,
      driver_source: null,
      driver_target: null,
      driver_p_value_two_sided: null,
      minimum_absolute_path_coefficient: null,
      technically_required_sample_size: null,
      meets_technical_requirement: null,
      status: "inference_incomplete",
    });

    expect(nativePlsPosthocMinimumSampleSizeProjection(run)).toBeNull();
    expect(nativeResultTables(run)).toEqual([]);
  });

  it("rejects bootstrap inference whose stored original no longer matches its linked path", () => {
    const run = completedInferenceAwarePosthocRun();
    const linked = run.bootstrap!.percentile.parameters.find((parameter) => (
      parameter.parameter === JSON.stringify(["path", ["likeability", "loyalty"]])
    ))!;
    linked.original += 0.01;

    expect(nativePlsPosthocMinimumSampleSizeProjection(run)).toBeNull();
    expect(nativeResultTables(run)).toEqual([]);
  });

  it("attributes the canonical table and readback export to the exact post-hoc capability cell", async () => {
    const run = completedInferenceAwarePosthocRun();
    const built = await canonicalResultDocumentFromAnalysisRunV2(run, {
      projectId: "posthoc-project",
      datasetId: "posthoc-dataset",
    });
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const table = built.document.tables.find((candidate) => candidate.id === "posthoc_minimum_sample_size");
    expect(table?.capability_cells).toEqual([expect.objectContaining({
      registry_schema_version: 2,
      capability_id: "smartpls.pls_power_analysis",
      cell_id: "qpls3.pls.posthoc_technical_minimum_sample_size",
      capability_version: "pls_posthoc_technical_minimum_sample_size_v2",
    })]);
    expect(table?.rows).toHaveLength(1);
    const requiredIndex = table!.columns.findIndex((column) => column.id === "required_sample_size");
    const formulaTestIndex = table!.columns.findIndex((column) => column.id === "formula_test");
    const selectionSourceIndex = table!.columns.findIndex((column) => column.id === "significance_source");
    expect(table!.rows[0].cells[requiredIndex]).toEqual({ kind: "number", value: 209 });
    expect(table!.rows[0].cells[formulaTestIndex]).toEqual({ kind: "text", value: "directional" });
    expect(table!.rows[0].cells[selectionSourceIndex]).toEqual({
      kind: "text",
      value: "pls_bootstrap_normal_reference_two_sided",
    });

    const exported = await previewNativeCanonicalSemanticExportV2(run, {
      projectId: "posthoc-project",
      datasetId: "posthoc-dataset",
    });
    expect(exported.status).toBe("ready");
    if (exported.status !== "ready") return;
    expect(exported.projection.tables.find((candidate) => candidate.id === "posthoc_minimum_sample_size"))
      .toMatchObject({ capability_cells: table?.capability_cells });
    expect(exported.json).toContain("qpls3.pls.posthoc_technical_minimum_sample_size");
  });

  it("shows a corrective unavailable result when bootstrap inference was not run", () => {
    const run = completedSamplePlsRun();
    delete run.bootstrap;
    run.result!.posthoc_minimum_sample_size = {
      method_version: "inverse_square_root_posthoc_v2",
      alpha: 0.05,
      power: 0.80,
      test: "directional",
      inverse_square_root_constant: 2.486,
      selection_rule: "smallest_absolute_statistically_significant_structural_path",
      significance_source: null,
      significance_alpha: null,
      eligible_path_count: run.result!.paths.length,
      significant_path_count: null,
      driver_source: null,
      driver_target: null,
      driver_p_value_two_sided: null,
      minimum_absolute_path_coefficient: null,
      technically_required_sample_size: null,
      analytical_sample_size: run.result!.used_observations,
      meets_technical_requirement: null,
      status: "inference_unavailable",
      caution: "Retrospective technical diagnostic; bootstrap inference is required.",
    };

    expect(nativePlsPosthocMinimumSampleSizeProjection(run)?.status).toBe("inference_unavailable");
    expect(nativeResultTables(run).find((table) => table.id === "posthoc_minimum_sample_size")?.rows)
      .toEqual(expect.arrayContaining([
        ["Result status", "Unavailable: run PLS bootstrapping to identify statistically significant paths"],
      ]));
  });
});
