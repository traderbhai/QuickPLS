import { describe, expect, it } from "vitest";
import type { AnalysisRun } from "../types";
import {
  NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID,
  NATIVE_PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY,
  NATIVE_PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD,
  NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
  NativePlsSampleSizePowerBuildError,
  buildNativePlsSampleSizePowerRecipe,
  nativePlsPowerWilsonInterval,
  nativePlsSampleSizePowerExportTables,
  nativePlsSampleSizePowerPresentation,
  validateNativePlsSampleSizePowerResult,
  type NativePlsSampleSizePowerDraft,
  type NativePlsSampleSizePowerRecipeV2,
  type NativePlsSampleSizePowerResultV2,
} from "./nativePlsSampleSizePower";
import {
  buildNativeResultNavigation,
  completedResultRuns,
  nativePlsSampleSizePowerPlot,
  nativePlsSampleSizePowerResultProjection,
  nativeResultTables,
} from "./nativeResults";
import { nativeRunProvenanceTable, nativeRunSettingApplicability } from "./nativeExportTables";
import { nativeExportScope } from "./NativeExportDialog";

const draft: NativePlsSampleSizePowerDraft = {
  scenarioIdentity: "two_construct_signal",
  predictorConstruct: "x",
  outcomeConstruct: "y",
  predictorIndicatorLoadings: "0.80, 0.80, 0.80",
  outcomeIndicatorLoadings: "0.80, 0.80, 0.80",
  populationPath: "0.30",
  exogenousDistribution: "standard_normal",
  structuralDisturbanceDistribution: "standard_normal",
  indicatorErrorDistribution: "standard_normal",
  missingData: "none",
  weightingScheme: "path",
  preprocessing: "standardized",
  tolerance: "1e-7",
  maxIterations: "3000",
  inference: "case_bootstrap_null_centered_two_sided_plus_one",
  sampleSizeGrid: "60, 120",
  alpha: "0.05",
  targetPower: "0.80",
  confidenceLevel: "0.95",
  monteCarloReplicates: "100",
  bootstrapReplicates: "99",
  masterSeed: "20260813",
  workers: "4",
};

function result(recipe: NativePlsSampleSizePowerRecipeV2): NativePlsSampleSizePowerResultV2 {
  const outcomes = recipe.sample_size_grid.flatMap((sampleSize) => (
    Array.from({ length: recipe.monte_carlo_replicates }, (_, replicateIndex) => {
      const rejected = sampleSize === 120 && replicateIndex < 95;
      return {
        sample_size: sampleSize,
        replicate_index: replicateIndex,
        stream_identity: replicateIndex.toString(16).padStart(64, "0"),
        attempted: true,
        successful: true,
        converged: true,
        target_estimate: 0.30,
        p_value_two_sided: rejected ? 0.01 : 0.20,
        bootstrap_requested_replicates: 99,
        bootstrap_usable_replicates: 99,
        bootstrap_failed_replicates: 0,
        bootstrap_two_sided_exceedances: rejected ? 0 : 19,
        rejected,
        failure_code: null,
        failure_message: null,
      };
    })
  ));
  const rows = recipe.sample_size_grid.map((sampleSize, index) => {
    const selected = outcomes.slice(index * 100, index * 100 + 100);
    const rejections = selected.filter((outcome) => outcome.rejected).length;
    const [lower, upper] = nativePlsPowerWilsonInterval(rejections, 100, 0.95);
    return {
      sample_size: sampleSize,
      requested_replicates: 100,
      attempted_replicates: 100,
      successful_replicates: 100,
      failed_replicates: 0,
      rejections,
      achieved_power: rejections / 100,
      confidence_lower: lower,
      confidence_upper: upper,
      qualifies: lower >= 0.80,
    };
  });
  return {
    schema_version: 2,
    capability_id: NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID,
    method_version: NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
    recipe_digest: "a".repeat(64),
    stream_domain: "quickpls/pls_sample_size_power_v2/monte_carlo",
    failure_policy: NATIVE_PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY,
    interval_method: NATIVE_PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD,
    inference_method: "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2",
    pls_method_version: "pls_pm_v1",
    resampling_method_version: "indexed_resampling_v4",
    workload: {
      grid_points: 2,
      planned_datasets: 200,
      estimated_pls_fits: 20_000,
      estimated_pls_case_fits: 1_800_000,
    },
    rows,
    outcomes,
    outcome_digest: "b".repeat(64),
    decision: { status: "reached", sample_size: 120 },
    monotonicity_violations: 0,
    warnings: ["Conditional on assumptions."],
    exclusions: ["No heuristic rules."],
  };
}

function completedRun(recipe: NativePlsSampleSizePowerRecipeV2): AnalysisRun {
  return {
    id: "power-run",
    modelId: "power-model",
    name: "Prospective power run",
    method: "PLS-SEM Sample Size and Power Analysis",
    createdAt: "2026-08-13T00:00:01Z",
    seed: recipe.master_seed,
    status: "completed",
    warnings: [],
    fingerprint: "prospective",
    plsSampleSizePower: result(recipe),
    plsSampleSizePowerRecipe: recipe,
    provenance: {
      recipe_id: "power-recipe",
      dataset_fingerprint: "prospective-design",
      method: "pls_sample_size_power",
      method_version: "pls_sample_size_power_v2",
      engine_version: "test",
      seed: recipe.master_seed,
      settings: {
        method: "pls_sample_size_power",
        weighting_scheme: "path",
        tolerance: recipe.estimator.tolerance,
        max_iterations: recipe.estimator.max_iterations,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: recipe.master_seed,
        workers: recipe.workers,
        confidence_level: recipe.confidence_level,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-13T00:00:00Z",
      completed_at: "2026-08-13T00:00:01Z",
    },
  };
}

describe("native PLS sample-size and power v2", () => {
  it("builds an explicit bounded prospective recipe and workload estimate", () => {
    const built = buildNativePlsSampleSizePowerRecipe(draft);
    expect(built.recipe).toMatchObject({
      capability_id: NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID,
      method_version: NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
      schema_version: 2,
      inference: "case_bootstrap_null_centered_two_sided_plus_one",
      design: { population_path: 0.30, missing_data: "none" },
      sample_size_grid: [60, 120],
      monte_carlo_replicates: 100,
      bootstrap_replicates: 99,
    });
    expect(built.workload).toEqual({
      gridPoints: 2,
      plannedDatasets: 200,
      estimatedPlsFits: 20_000,
      estimatedPlsCaseFits: 1_800_000,
    });
  });

  it.each([
    ["predictorIndicatorLoadings", ""],
    ["missingData", ""],
    ["sampleSizeGrid", "120, 60"],
    ["bootstrapReplicates", "100"],
  ] as const)("blocks incomplete or unsupported %s", (field, value) => {
    expect(() => buildNativePlsSampleSizePowerRecipe({ ...draft, [field]: value })).toThrow(
      NativePlsSampleSizePowerBuildError,
    );
  });

  it("matches the known Wilson interval", () => {
    const [lower, upper] = nativePlsPowerWilsonInterval(80, 100, 0.95);
    expect(lower).toBeCloseTo(0.7111708343, 9);
    expect(upper).toBeCloseTo(0.8666330667, 9);
  });

  it("accepts Rust statrs Wilson bounds while rejecting material bound drift", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const payload = result(recipe);
    // Values below are frozen by the Rust/statrs unit contract for the exact
    // 0/100 and 95/100 ledgers in this fixture, not by the TypeScript helper.
    payload.rows[0].confidence_lower = 3.469446951953614e-18;
    payload.rows[0].confidence_upper = 0.03699349820698568;
    payload.rows[1].confidence_lower = 0.8882495307680808;
    payload.rows[1].confidence_upper = 0.9784563208456319;
    expect(() => validateNativePlsSampleSizePowerResult(recipe, payload)).not.toThrow();

    payload.rows[1].confidence_lower += 2e-8;
    expect(() => validateNativePlsSampleSizePowerResult(recipe, payload)).toThrow(/does not reproduce/);
  });

  it("renders an accessible table-equivalent presentation with failures and provenance", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const payload = result(recipe);
    const presentation = nativePlsSampleSizePowerPresentation(recipe, payload);
    expect(presentation.decisionLabel).toContain("n = 120");
    expect(presentation.rows).toHaveLength(2);
    expect(presentation.failureSummary).toContain("0 of 200");
    expect(presentation.assumptions.join(" ")).toContain("Target path: x → y");
    expect(presentation.provenance).toContainEqual(["Outcome digest", "b".repeat(64)]);
  });

  it("exports all five v2 contract tables from the same validated result", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const tables = nativePlsSampleSizePowerExportTables(recipe, result(recipe));
    expect(tables.map((table) => table.name)).toEqual([
      "Power by sample size",
      "Bootstrap tail accounting",
      "Simulation failures",
      "Design assumptions",
      "Run provenance",
    ]);
    expect(tables[0].rows[1]).toContain("true");
    expect(tables[1].rows[0]).toEqual(["60", "100", "9900", "9900", "0", "1900", "99", "99"]);
    expect(tables[4].rows).toContainEqual(["grid_decision", "120"]);
  });

  it("rejects changed denominators, power rows, and recommendation logic", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const changedDenominator = result(recipe);
    changedDenominator.rows[0].requested_replicates = 99;
    expect(() => validateNativePlsSampleSizePowerResult(recipe, changedDenominator)).toThrow(/does not reproduce/);

    const changedOutcome = result(recipe);
    changedOutcome.outcomes[0].rejected = true;
    expect(() => validateNativePlsSampleSizePowerResult(recipe, changedOutcome)).toThrow(/inconsistent/);

    const changedTail = result(recipe);
    changedTail.outcomes[0].bootstrap_two_sided_exceedances = 1;
    expect(() => validateNativePlsSampleSizePowerResult(recipe, changedTail)).toThrow(/inconsistent/);

    const changedDecision = result(recipe);
    changedDecision.decision = { status: "not_reached" };
    expect(() => validateNativePlsSampleSizePowerResult(recipe, changedDecision)).toThrow(/decision/);
  });

  it("binds completed power payloads to native Results, provenance, and tables-only export", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const run = completedRun(recipe);
    expect(nativePlsSampleSizePowerResultProjection(run)?.presentation.decisionLabel).toContain("n = 120");
    expect(nativePlsSampleSizePowerPlot(run)).toEqual({
      targetPower: recipe.target_power,
      confidenceLevel: recipe.confidence_level,
      points: run.plsSampleSizePower!.rows.map((row) => ({
        sampleSize: row.sample_size,
        achievedPower: row.achieved_power,
        confidenceLower: row.confidence_lower,
        confidenceUpper: row.confidence_upper,
        qualifies: row.qualifies,
      })),
    });
    expect(completedResultRuns([run])).toEqual([run]);
    expect(nativeResultTables(run).map((table) => table.id)).toEqual([
      "pls_power_by_sample_size",
      "pls_power_bootstrap_tail_accounting",
      "pls_power_simulation_failures",
      "pls_power_design_assumptions",
      "pls_power_run_provenance",
    ]);
    expect(buildNativeResultNavigation(run).defaultItemId).toBe("pls_power_by_sample_size");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: true,
      usesConfidenceLevel: true,
      usesWorkers: true,
    });
    expect(nativeRunProvenanceTable(run).rows).toContainEqual(["Planned PLS fits", "20000"]);
    expect(nativeExportScope(run).includeModelDiagram).toBe(false);
  });

  it("fails native Results closed when a persisted power row is changed", () => {
    const recipe = buildNativePlsSampleSizePowerRecipe(draft).recipe;
    const run = completedRun(recipe);
    run.plsSampleSizePower!.rows[0].rejections = 1;
    expect(nativePlsSampleSizePowerResultProjection(run)).toBeNull();
    expect(nativePlsSampleSizePowerPlot(run)).toBeNull();
    expect(completedResultRuns([run])).toEqual([]);
    expect(nativeResultTables(run)).toEqual([]);
  });
});
