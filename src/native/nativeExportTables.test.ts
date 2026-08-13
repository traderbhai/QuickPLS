import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import { NATIVE_NCA_ENGINE_SCOPE_WARNING, NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import { NATIVE_PCA_ENGINE_SCOPE_WARNING } from "./nativePca";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { completedGscaRun } from "./nativeGsca.testFixture";
import { nativePcaScoreExportTable, nativeRunProvenanceTable, nativeRunSettingApplicability } from "./nativeExportTables";

function completedNcaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: null,
    modelSnapshot: undefined,
    method: "Necessary Condition Analysis",
    seed: 20_260_811,
    bootstrap: undefined,
    permutation: undefined,
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    provenance: {
      recipe_id: "recipe-nca",
      dataset_fingerprint: "sha256:nca-fixture",
      method: "nca",
      method_version: "nca_v2",
      engine_version: "2.45.0",
      seed: 20_260_811,
      settings: {
        method: "nca",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "unstandardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20_260_811,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-11T08:00:00.000Z",
      completed_at: "2026-08-11T08:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "nca_v2",
      iterations: 0,
      used_observations: 8,
      nca: {
        method_version: "nca_v2",
        ceiling: "both",
        permutation_samples: 19,
        usable_permutations: 19,
        x: "condition",
        y: "outcome",
        observations: 8,
        scope: { minimum_x: 1, maximum_x: 8, minimum_y: 1, maximum_y: 9 },
        ce_fdh_peers: [{ x: 1, y: 1 }, { x: 8, y: 9 }],
        ceilings: [
          { ceiling: "ce_fdh", effect_size: 0.3, permutation_p_value: 0.05, slope: null, intercept: null },
          { ceiling: "cr_fdh", effect_size: 0.2, permutation_p_value: 0.1, slope: 1, intercept: 0 },
        ],
        bottlenecks: ["ce_fdh", "cr_fdh"].flatMap((ceiling) => [10, 20, 30, 40, 50, 60, 70, 80, 90].map((outcome) => ({
          ceiling,
          outcome_percent: outcome,
          required_x_percent: outcome / 2,
          status: "required" as const,
        }))),
        warnings: [NATIVE_NCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

function completedPcaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: null,
    modelSnapshot: undefined,
    method: "Principal Component Analysis",
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
      seed: 7,
      settings: {
        method: "pca",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "standardized",
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
      method_version: "pca_v1",
      used_observations: 3,
      pca: {
        method_version: "pca_v1",
        component_rule: "fixed",
        retained_components: 1,
        observations: 3,
        variables: ["a", "b"],
        components: [{ component: "PC1", eigenvalue: 2, explained_variance: 1, cumulative_variance: 1 }],
        loadings: [
          { variable: "a", component: "PC1", loading: 1, weight: Math.SQRT1_2 },
          { variable: "b", component: "PC1", loading: 1, weight: Math.SQRT1_2 },
        ],
        scores: [
          { observation: 0, component: "PC1", score: -Number.EPSILON },
          { observation: 1, component: "PC1", score: 0.25 },
          { observation: 2, component: "PC1", score: 1.25 },
        ],
        warnings: [NATIVE_PCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

describe("native export provenance", () => {
  it("exports exact GSCA ALS v2 provenance without PLS or inference settings", () => {
    const run = completedGscaRun();
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Method version", "gsca_als_v2"],
      ["Estimator", "Joint global least-squares alternating least squares"],
      ["Algorithm version", "alternating_least_squares_v1"],
      ["Analyzed observations", "140"],
      ["Converged", "Yes"],
      ["ALS iterations", "4"],
      ["Initialization", "Deterministic +1 block weights"],
      ["Inference", "Point estimates only"],
    ]));
    expect(fields).not.toEqual(expect.arrayContaining([
      "Seed",
      "Weighting scheme",
      "Confidence level",
      "Workers",
      "Bootstrap samples",
      "Permutation samples",
    ]));
    expect(table.rows.flat().join(" ")).not.toMatch(/\bN\/?A\b|percentile interval/i);
  });

  it("exports bounded CB-SEM/CFA optimizer, identification, and method-version provenance without resampling fields", () => {
    const run = completedCbsemRun("cfa");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    const table = nativeRunProvenanceTable(run);
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Model type", "Confirmatory factor analysis"],
      ["Estimator", "Maximum likelihood"],
      ["Analyzed observations", "120"],
      ["Identification", "First loading fixed to 1 for each latent factor"],
      ["Estimator method version", "cfa_ml_v1"],
      ["Fit method version", "cbsem_fit_v1"],
      ["Modification-diagnostic version", "cbsem_modification_indices_v1"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toEqual(expect.arrayContaining(["Seed", "Workers", "Confidence level", "Bootstrap samples", "Permutation samples"]));
  });

  it("exports model-free PCA scores and PCA-specific provenance without SEM weighting claims", () => {
    const run = completedPcaRun();
    const scoreTable = nativePcaScoreExportTable(run);
    const provenance = nativeRunProvenanceTable(run);

    expect(scoreTable).toMatchObject({
      id: "pca_scores",
      title: "Component scores",
      columns: ["Complete-case observation", "PC1"],
      rows: [["1", "0.000000"], ["2", "0.250000"], ["3", "1.250000"]],
    });
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["Selected variables", "a, b"],
      ["Retention rule", "Fixed component count"],
      ["Input matrix", "Correlation matrix of standardized variables"],
    ]));
    expect(provenance.rows.some(([field]) => field === "Weighting scheme" || field === "Seed")).toBe(false);
  });

  it("builds one truthful provenance table without legacy N/A placeholders", () => {
    const run = {
      ...completedSamplePlsRun(),
      warnings: ["Small sample warning"],
    };
    const table = nativeRunProvenanceTable(run);

    expect(table.id).toBe("run_provenance");
    expect(table.rows).toContainEqual(["Method version", run.result?.method_version]);
    expect(table.rows).toContainEqual(["Warnings", "Small sample warning"]);
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
  });

  it("omits unavailable optional fields instead of manufacturing a value", () => {
    const run = {
      ...completedSamplePlsRun(),
      result: undefined,
      warnings: [],
    };
    const table = nativeRunProvenanceTable(run, "experimental");

    expect(table.status).toBe("experimental");
    expect(table.rows.some(([field]) => field === "Method version")).toBe(false);
    expect(table.rows.some(([field]) => field === "Warnings")).toBe(false);
  });

  it("exports immutable engine settings when native provenance is available", () => {
    const run = {
      ...completedSamplePlsRun(),
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-1",
        dataset_fingerprint: "sha256:full-fingerprint",
        method: "wpls" as const,
        method_version: "wpls_case_weighted_v1+assessment_v1",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "wpls" as const,
          weighting_scheme: "path" as const,
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized" as const,
          missing_data: "listwise_deletion" as const,
          case_weight_column: "case_weight",
        },
        started_at: "2026-08-10T10:00:00Z",
        completed_at: "2026-08-10T10:00:01Z",
      },
    };
    const table = nativeRunProvenanceTable(run);

    expect(table.rows).toContainEqual(["Dataset fingerprint", "sha256:full-fingerprint"]);
    expect(table.rows).toContainEqual(["Method version", "wpls_case_weighted_v1+assessment_v1"]);
    expect(table.rows).toContainEqual(["Case-weight variable", "case_weight"]);
    expect(table.rows).toContainEqual(["Preprocessing", "standardized"]);
    expect(table.rows.some(([field]) => field === "Seed")).toBe(false);
    expect(table.rows.some(([field]) => field === "Confidence level")).toBe(false);
    expect(table.rows.some(([field]) => field === "Workers")).toBe(false);
  });

  it("omits unused inference settings from descriptive CCA exports", () => {
    const run = {
      ...completedSamplePlsRun(),
      method: "CCA composite residual diagnostics",
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-cca",
        dataset_fingerprint: "sha256:cca",
        method: "cca" as const,
        method_version: "pls_pm_v1+cca_composite_residual_v1+pls_assessment_v7",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "cca" as const,
          weighting_scheme: "path" as const,
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized" as const,
          missing_data: "listwise_deletion" as const,
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:01Z",
      },
    };
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);

    expect(fields).not.toContain("Seed");
    expect(fields).not.toContain("Confidence level");
    expect(fields).not.toContain("Workers");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
  });

  it("exports the genuine MICOM permutation plan without claiming parallel workers", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      method: "MICOM and Two-Group Permutation MGA",
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-mga-v2",
        dataset_fingerprint: "sha256:mga-v2",
        method: "mga",
        method_version: "pls_mga_two_group_v2+pls_mga_permutation_v2+micom_v2+pls_assessment_v7",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "mga",
          weighting_scheme: "path",
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized",
          missing_data: "listwise_deletion",
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:03Z",
      },
      result: {
        ...base.result!,
        mga_permutation: {
          method_version: "pls_mga_permutation_v2",
          group_column: "group",
          permutation_samples: 5_000,
          usable_permutations: 5_000,
          comparisons: [],
          measurement_comparisons: [],
          warnings: [],
        },
        micom: {
          method_version: "micom_v2",
          group_column: "group",
          permutation_samples: 5_000,
          usable_permutations: 5_000,
          confidence_level: 0.95,
          groups: [{ group: "A", observations: 50 }, { group: "B", observations: 50 }],
          constructs: [],
          warnings: [],
        },
      },
    };
    const table = nativeRunProvenanceTable(run);

    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Confidence level", "0.95"],
      ["MICOM Step 1", "Researcher confirmed configural invariance"],
      ["Requested group permutations", "5000"],
      ["Usable group permutations", "5000"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toContain("Workers");
  });

  it("exports exact standalone NCA provenance without irrelevant SEM iteration settings", () => {
    const run = completedNcaRun();
    const applicability = nativeRunSettingApplicability(run);
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);

    expect(applicability).toEqual({ usesSeed: true, usesConfidenceLevel: false, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Seed", "20260811"],
      ["Method version", "nca_v2"],
      ["Condition variable (X)", "condition"],
      ["Outcome variable (Y)", "outcome"],
      ["Analyzed observations", "8"],
      ["Ceiling lines", "CE-FDH and CR-FDH"],
      ["Requested NCA permutations", "19"],
      ["Usable NCA permutations", "19"],
      ["Missing data", "listwise deletion"],
    ]));
    expect(fields).not.toContain("Weighting scheme");
    expect(fields).not.toContain("Maximum iterations");
    expect(fields).not.toContain("Stop criterion");
    expect(fields).not.toContain("Workers");
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
  });

  it("exports exact PLSpredict / CVPAT v2 provenance and the assignment digest", () => {
    const base = completedSamplePlsRun();
    const assessment = (benchmark: "indicator_average" | "linear_model") => ({
      method_version: "cvpat_indicator_benchmarks_v2",
      comparison_kind: "benchmark_assessment" as const,
      target_scope: "all_endogenous_indicators" as const,
      benchmark,
      loss: "mean_squared_error_across_indicators_per_observation" as const,
      alternative: "pls_loss_less_than_benchmark" as const,
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
      indicator_count: 2,
      status: "benchmark_unavailable" as const,
      preferred_model: null,
      reason: "fixture_unavailable",
    });
    const digest = `sha256:${"b".repeat(64)}`;
    const run: AnalysisRun = {
      ...base,
      method: "PLSpredict / CVPAT",
      seed: 20_260_811,
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-prediction-v2",
        dataset_fingerprint: "sha256:prediction-v2",
        method: "predict",
        method_version: "plspredict_indicator_v2",
        engine_version: "2.46.0",
        seed: 20_260_811,
        settings: {
          method: "predict",
          weighting_scheme: "path",
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_811,
          workers: 8,
          confidence_level: 0.9,
          preprocessing: "standardized",
          missing_data: "listwise_deletion",
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:03Z",
      },
      result: {
        ...base.result!,
        used_observations: 64,
        predict: {
          method_version: "plspredict_indicator_v2",
          split: "deterministic_complete_case_modulo_4_test_rows",
          training_observations: 48,
          test_observations: 16,
          benchmark: "indicator_average",
          targets: [],
          indicator_targets: [],
          repeated_kfold: {
            method_version: "plspredict_repeated_kfold_indicator_v2",
            folds: 10,
            repeats: 10,
            assignment: "seeded_chacha20_balanced_folds",
            assignment_digest: digest,
            seed: 20_260_811,
            total_test_observations: 640,
            targets: [],
            indicator_targets: [],
            cvpat_benchmark_assessments: [assessment("indicator_average"), assessment("linear_model")],
            warnings: [],
          },
          warnings: [],
        },
      },
    };

    const table = nativeRunProvenanceTable(run);
    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Seed", "20260811"],
      ["CVPAT confidence level", "0.95"],
      ["Repeated prediction method version", "plspredict_repeated_kfold_indicator_v2"],
      ["CVPAT method version", "cvpat_indicator_benchmarks_v2"],
      ["Primary validation", "10-fold × 10-repeat cross-validation"],
      ["Assignment digest", digest],
      ["Benchmarks", "Indicator average (IA); Linear model (LM, where estimable)"],
      ["CVPAT alternative", "PLS-SEM loss < benchmark (one-sided)"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toContain("Workers");
    expect(table.rows.flat().join(" ")).not.toContain("saved model");
  });
});
