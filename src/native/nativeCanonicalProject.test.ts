import { describe, expect, it } from "vitest";
import type {
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
  NativeCanonicalModelSpec,
  NativeProjectSnapshot,
} from "../types";
import {
  compactNativeWorkspaceRuns,
  nativeModelSnapshotFromCanonical,
  nativeRunFromCanonicalResult,
  reconcileNativeCanonicalProject,
  resolveActiveCanonicalModel,
} from "./nativeCanonicalProject";
import { newNativeScientificCovarianceEdgeV4, withNativeConstructEstimandV4 } from "../domain/semModelV4Authoring";

function model(id = "model-1", name = "Canonical model"): NativeCanonicalModelSpec {
  return {
    id,
    name,
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [{ source: "x", target: "y" }],
    controls: [{ source: "x", target: "y", label: "Baseline control" }],
    higher_order_constructs: [],
    interactions: [],
  };
}

function recipe(overrides: Partial<NativeCanonicalAnalysisRecipe> = {}): NativeCanonicalAnalysisRecipe {
  return {
    schema_version: 2,
    id: "recipe-1",
    created_at: "2026-08-11T00:00:00.000Z",
    dataset_fingerprint: "fingerprint-abcdefghijklmnopqrstuvwxyz",
    model: model(),
    settings: {
      method: "pls_pm",
      weighting_scheme: "path",
      tolerance: 1e-7,
      max_iterations: 3000,
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      seed: 42,
      workers: 1,
      confidence_level: 0.95,
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      case_weight_column: null,
    },
    metadata: {},
    ...overrides,
  };
}

function envelope(overrides: Partial<AnalysisResultEnvelope> = {}): AnalysisResultEnvelope {
  return {
    schema_version: 1,
    id: "result-1",
    status: "completed",
    provenance: {
      recipe_id: "recipe-1",
      dataset_fingerprint: "fingerprint-abcdefghijklmnopqrstuvwxyz",
      method: "pls_pm",
      method_version: "pls_pm_v1",
      engine_version: "test",
      seed: 42,
      settings: recipe().settings,
      started_at: "2026-08-11T00:00:00.000Z",
      completed_at: "2026-08-11T00:00:01.000Z",
    },
    diagnostics: [{ code: "warning", level: "warning", message: "A genuine warning" }],
    payload: {
      kind: "pls_pm_v1",
      estimation: {
        method_version: "pls_pm_v1",
        converged: true,
        iterations: 4,
        used_observations: 100,
        omitted_observations: 0,
        outer_estimates: [],
        paths: [{ source: "x", target: "y", coefficient: 0.5 }],
        effects: [{ source: "x", target: "y", direct: 0.5, indirect: 0, total: 0.5 }],
        r_squared: { y: 0.25 },
        warnings: [],
      },
      assessment: {
        method_version: "pls_assessment_v1",
        construct_quality: [],
        cross_loadings: [],
        fornell_larcker: { constructs: [], values: [] },
        r_squared: { y: 0.25 },
        structural_quality: [],
        structural_vif: [],
        formative_indicator_vif: [],
        f_squared: [],
        warnings: [],
      },
    },
    ...overrides,
  };
}

describe("canonical native project reconciliation", () => {
  it("keeps legacy schema-v1/v2 recipes readable without synthesizing method_config", () => {
    for (const schema_version of [1, 2]) {
      const legacy = recipe({ schema_version });
      expect(legacy.schema_version).toBe(schema_version);
      expect(legacy.method_config).toBeUndefined();
    }
  });

  it("uses canonical model content while retaining covariance presentation without reinterpreting it as a path", () => {
    const snapshot = nativeModelSnapshotFromCanonical(model(), {
      nodes: [
        { id: "x", type: "construct", position: { x: 222, y: 333 }, data: { label: "Stale X", shortName: "OLD", mode: "formative", indicators: ["wrong"] } },
        { id: "y", type: "construct", position: { x: 555, y: 333 }, data: { label: "Stale Y", shortName: "OLD", mode: "formative", indicators: ["wrong"] } },
      ],
      edges: [{ id: "stale-edge-id", source: "x", target: "y", type: "straight", data: { role: "covariance" } }],
    });

    expect(snapshot.nodes[0]).toMatchObject({
      id: "x",
      position: { x: 222, y: 333 },
      data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] },
    });
    expect(snapshot.edges[0]).toMatchObject({
      source: "x",
      target: "y",
      label: "Baseline control",
      data: { role: "control", controlLabel: "Baseline control" },
    });
    expect(snapshot.edges[1]).toMatchObject({
      id: "stale-edge-id",
      source: "x",
      target: "y",
      type: "straight",
      data: { role: "covariance" },
    });
    expect(snapshot.diagramLayout?.constructLayouts.x).toMatchObject({ x: 222, y: 333 });
  });

  it("preserves versioned construct and covariance authoring metadata through canonical reopen", () => {
    const x = withNativeConstructEstimandV4({
      id: "x",
      type: "construct",
      position: { x: 10, y: 20 },
      data: { label: "X", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] },
    }, { kind: "common_factor", marker_indicator: "x1" });
    const covariance = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    const snapshot = nativeModelSnapshotFromCanonical(model(), {
      nodes: [x, { id: "y", type: "construct", position: { x: 200, y: 20 }, data: { label: "Y", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } }],
      edges: [covariance],
    });
    expect(snapshot.nodes.find((node) => node.id === "x")?.data.semModelV4).toEqual(x.data.semModelV4);
    expect(snapshot.edges.find((edge) => edge.id === covariance.id)?.data).toEqual(covariance.data);
  });

  it("rebuilds completed runs from canonical results instead of stale workspace payloads", () => {
    const canonicalPayload = envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const stale: AnalysisRun = {
      id: "result-1",
      name: "Fabricated run",
      method: "Wrong method",
      createdAt: "1999-01-01T00:00:00.000Z",
      seed: 1,
      status: "completed",
      warnings: ["wrong"],
      fingerprint: "wrong",
      result: { ...canonicalPayload.estimation, paths: [{ source: "x", target: "y", coefficient: 999 }] },
    };
    const project = {
      models: [model()],
      recipes: [recipe()],
      results: [envelope()],
      activeModelId: "model-1",
      workspace: { nodes: [], edges: [], runs: [stale] },
    } satisfies Pick<NativeProjectSnapshot, "models" | "recipes" | "results" | "activeModelId" | "workspace">;

    const reconciled = reconcileNativeCanonicalProject(project);

    expect(reconciled.modelSource).toBe("canonical");
    expect(reconciled.resultSource).toBe("canonical");
    expect(reconciled.runs).toHaveLength(1);
    expect(reconciled.runs[0]).toMatchObject({
      id: "result-1",
      modelId: "model-1",
      name: "PLS-SEM Algorithm run",
      method: "PLS-SEM Algorithm",
      createdAt: "2026-08-11T00:00:01.000Z",
      seed: 42,
      warnings: ["A genuine warning"],
      fingerprint: "fingerprint-",
    });
    expect(reconciled.runs[0].result?.paths[0].coefficient).toBe(0.5);
    expect(reconciled.runs[0].modelSnapshot?.nodes[0].data.label).toBe("Predictor");
  });

  it("hydrates the active canonical model from its own presentation without leaking another model layout", () => {
    const first = model("model-1", "First model");
    const second = model("model-2", "Second model");
    const reconciled = reconcileNativeCanonicalProject({
      models: [first, second],
      recipes: [],
      results: [],
      activeModelId: second.id,
      modelPresentations: {
        [first.id]: {
          nodes: first.constructs.map((construct) => ({
            id: construct.id,
            position: { x: 10, y: 20 },
            data: { label: construct.name, shortName: construct.short_name, mode: construct.mode, indicators: construct.indicators },
          })),
          edges: [],
        },
        [second.id]: {
          nodes: second.constructs.map((construct) => ({
            id: construct.id,
            position: { x: 700, y: 300 },
            data: { label: construct.name, shortName: construct.short_name, mode: construct.mode, indicators: construct.indicators },
          })),
          edges: [],
        },
      },
      savedReports: [{ resultId: "result-1", name: "Reviewer view", savedAt: "2026-08-11T12:00:00Z" }],
      workspace: {
        nodes: first.constructs.map((construct) => ({
          id: construct.id,
          position: { x: 99, y: 99 },
          data: { label: construct.name, shortName: construct.short_name, mode: construct.mode, indicators: construct.indicators },
        })),
        edges: [],
      },
    });

    expect(reconciled.activeModelId).toBe(second.id);
    expect(reconciled.projectModels.map((candidate) => candidate.id)).toEqual([first.id, second.id]);
    expect(reconciled.nodes.every((node) => node.position.x === 700)).toBe(true);
    expect(reconciled.savedReports).toEqual([{ resultId: "result-1", name: "Reviewer view", savedAt: "2026-08-11T12:00:00Z" }]);
    expect(reconciled.explorerSelection).toEqual({ kind: "model", modelId: second.id });
  });

  it("derives the method identity from canonical settings and real resampling artifacts", () => {
    const bootstrapRecipe = recipe({
      settings: { ...recipe().settings, bootstrap_samples: 100 },
    });
    const bootstrapEnvelope = envelope({
      payload: {
        kind: "pls_pm_v2",
        estimation: (envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>).estimation,
        assessment: (envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>).assessment,
        bootstrap: {
          method_version: "pls_bootstrap_v1",
          plan: { replicates: 100, master_seed: 42, operation: "bootstrap" },
          usable_replicates: 100,
          failed_replicates: [],
          percentile: { confidence_level: 0.95, parameters: [] },
        },
      },
    });

    expect(nativeRunFromCanonicalResult(bootstrapEnvelope, bootstrapRecipe)?.method).toBe("PLS-SEM Bootstrapping");
  });

  it("binds explicit one-sided bootstrap receipts to recipe, marker, order, and plus-one arithmetic", () => {
    const baseEnvelope = envelope();
    const basePayload = baseEnvelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const bootstrapRecipe = recipe({
      schema_version: 3,
      settings: {
        ...recipe().settings,
        bootstrap_samples: 4,
        bootstrap_test_tail: "one_sided_greater",
      },
      method_config: { kind: "pls_bootstrap" },
    });
    const parameters = ["[\"path\",[\"x\",\"y\"]]", "[\"total_effect\",[\"x\",\"y\"]]"];
    const configured = envelope({
      provenance: {
        ...baseEnvelope.provenance,
        method_version: "pls_pm_v1+indexed_resampling_v4+pls_bootstrap_null_centered_test_tail_v1",
        settings: bootstrapRecipe.settings,
      },
      payload: {
        kind: "pls_pm_v2",
        estimation: basePayload.estimation,
        assessment: basePayload.assessment,
        bootstrap: {
          method_version: "indexed_resampling_v4",
          plan: { replicates: 4, master_seed: 42, operation: "pls_pm_bootstrap_v1" },
          usable_replicates: 4,
          failed_replicates: [],
          percentile: {
            confidence_level: 0.95,
            parameters: parameters.map((parameter) => ({ parameter, original: 0.5, bootstrap_mean: 0.5, bias: 0, standard_error: 0.1, lower: 0.3, upper: 0.7, usable_replicates: 4 })),
          },
          test_tail_inference: {
            method_version: "pls_bootstrap_null_centered_test_tail_v1",
            selected_test_tail: "one_sided_greater",
            parameters: parameters.map((parameter, index) => ({
              parameter,
              usable_replicates: 4,
              two_sided_exceedances: index + 1,
              greater_or_equal_exceedances: index + 2,
              less_or_equal_exceedances: index,
              p_value_two_sided: (index + 2) / 5,
              p_value_greater: (index + 3) / 5,
              p_value_less: (index + 1) / 5,
            })),
          },
        },
      },
    });
    expect(nativeRunFromCanonicalResult(configured, bootstrapRecipe)).not.toBeNull();

    const mutations: Array<(candidate: any) => void> = [
      (candidate) => { delete candidate.payload.bootstrap.test_tail_inference; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.unexpected = true; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.selected_test_tail = "one_sided_less"; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.method_version = "wrong"; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.parameters.reverse(); },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.parameters[0].greater_or_equal_exceedances = 5; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.parameters[0].usable_replicates = 3; },
      (candidate) => { candidate.payload.bootstrap.test_tail_inference.parameters[0].p_value_greater = 0.61; },
      (candidate) => { candidate.provenance.method_version = "pls_pm_v1+indexed_resampling_v4"; },
    ];
    for (const mutate of mutations) {
      const candidate = JSON.parse(JSON.stringify(configured)) as AnalysisResultEnvelope;
      mutate(candidate);
      expect(nativeRunFromCanonicalResult(candidate, bootstrapRecipe)).toBeNull();
    }
  });

  it("hydrates a bootstrap-only corporate-shaped result and rejects a mixed bootstrap-permutation payload", () => {
    const baseEnvelope = envelope();
    const basePayload = baseEnvelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const bootstrapRecipe = recipe({
      schema_version: 3,
      settings: {
        ...recipe().settings,
        bootstrap_samples: 24,
        permutation_samples: 0,
      },
      method_config: { kind: "pls_bootstrap" },
      metadata: { demo: "quickpls_v04_demo" },
    });
    const bootstrap = {
      method_version: "pls_bootstrap_v1",
      plan: { replicates: 24, master_seed: 42, operation: "bootstrap" },
      usable_replicates: 24,
      failed_replicates: [],
      percentile: { confidence_level: 0.95, parameters: [] },
    };
    const bootstrapEnvelope = envelope({
      provenance: {
        ...baseEnvelope.provenance,
        settings: bootstrapRecipe.settings,
      },
      payload: {
        kind: "pls_pm_v2",
        estimation: basePayload.estimation,
        assessment: basePayload.assessment,
        bootstrap,
      },
    });

    const hydrated = reconcileNativeCanonicalProject({
      models: [model()],
      recipes: [bootstrapRecipe],
      results: [bootstrapEnvelope],
      activeModelId: "model-1",
      workspace: { nodes: [], edges: [], runs: [] },
    });

    expect(hydrated.runs).toHaveLength(1);
    expect(hydrated.runs[0]).toMatchObject({
      name: "PLS-SEM Bootstrapping run",
      method: "PLS-SEM Bootstrapping",
      bootstrap,
    });

    const mixedRecipe: NativeCanonicalAnalysisRecipe = {
      ...bootstrapRecipe,
      settings: { ...bootstrapRecipe.settings, permutation_samples: 99 },
    };
    const mixedEnvelope: AnalysisResultEnvelope = {
      ...bootstrapEnvelope,
      provenance: {
        ...bootstrapEnvelope.provenance,
        settings: mixedRecipe.settings,
      },
      payload: {
        kind: "pls_pm_v3",
        estimation: basePayload.estimation,
        assessment: basePayload.assessment,
        bootstrap,
        permutation: {
          method_version: "freedman_lane_permutation_v1",
          plan: { permutations: 99, master_seed: 42, operation: "pls_pm_freedman_lane_v1" },
          parameters: [],
        },
      },
    };

    expect(nativeRunFromCanonicalResult(mixedEnvelope, mixedRecipe)).toBeNull();
  });

  it("reopens current and legacy logistic results as model-free versioned runs", () => {
    const baseEnvelope = envelope();
    const basePayload = baseEnvelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const logisticRecipe = recipe({
      schema_version: 3,
      settings: {
        ...recipe().settings,
        method: "regression",
        preprocessing: "unstandardized",
      },
      method_config: {
        kind: "regression",
        outcome: "converted",
        predictors: ["score"],
        controls: [],
        model: { type: "logistic" },
      },
      metadata: { status: "validated_regression_logistic_v2_bounded_scope" },
    });
    const legacyLogisticRecipe = recipe({
      schema_version: 2,
      id: "recipe-logistic-v1",
      settings: {
        ...recipe().settings,
        method: "regression",
        preprocessing: "standardized",
      },
      metadata: {
        regression_type: "logistic",
        regression_outcome: "converted",
        regression_predictors: "score",
      },
    });
    const logisticEnvelope = (
      version: "regression_logistic_v2" | "regression_logistic_v1",
      canonicalRecipe: NativeCanonicalAnalysisRecipe,
    ): AnalysisResultEnvelope => ({
      ...baseEnvelope,
      id: `result-${version}`,
      provenance: {
        ...baseEnvelope.provenance,
        recipe_id: canonicalRecipe.id,
        method: "regression",
        method_version: version,
        settings: canonicalRecipe.settings,
      },
      payload: {
        ...basePayload,
        estimation: {
          ...basePayload.estimation,
          method_version: version,
          used_observations: 6,
          omitted_observations: 0,
          regression: {
            method_version: version,
            regression_type: "logistic",
            outcome: "converted",
            predictors: ["score"],
            controls: [],
            observations: 6,
            coefficients: [],
            fit: { aic: 10, bic: 9.5 },
            predictions: [],
            process: null,
            warnings: [],
          },
        },
      },
    });

    const current = nativeRunFromCanonicalResult(logisticEnvelope("regression_logistic_v2", logisticRecipe), logisticRecipe);
    expect(current).toMatchObject({
      modelId: null,
      method: "Binary Logistic Regression",
      name: "Binary Logistic Regression run",
    });
    expect(current?.modelSnapshot).toBeUndefined();

    expect(legacyLogisticRecipe).toMatchObject({
      schema_version: 2,
      settings: { method: "regression", preprocessing: "standardized" },
      metadata: { regression_type: "logistic", regression_outcome: "converted", regression_predictors: "score" },
    });
    expect(legacyLogisticRecipe.method_config).toBeUndefined();
    const legacy = nativeRunFromCanonicalResult(logisticEnvelope("regression_logistic_v1", legacyLogisticRecipe), legacyLogisticRecipe);
    expect(legacy).toMatchObject({
      modelId: null,
      method: "Legacy binary logistic regression (v1)",
      name: "Legacy binary logistic regression (v1) run",
    });
    expect(legacy?.modelSnapshot).toBeUndefined();
  });

  it("hydrates regression bootstrap only from the exact typed schema-v3 recipe", () => {
    const baseEnvelope = envelope();
    const basePayload = baseEnvelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const bootstrapRecipe = recipe({
      schema_version: 3,
      settings: {
        ...recipe().settings,
        method: "regression",
        preprocessing: "unstandardized",
        bootstrap_samples: 99,
        workers: 2,
      },
      method_config: {
        kind: "regression",
        outcome: "y",
        predictors: ["x"],
        model: { type: "ols", robust_se: "hc3" },
        bootstrap: { algorithm: "case_resampling", intervals: ["percentile", "bca"] },
      },
    });
    const bootstrapEnvelope: AnalysisResultEnvelope = {
      ...baseEnvelope,
      id: "result-regression-bootstrap",
      provenance: {
        ...baseEnvelope.provenance,
        recipe_id: bootstrapRecipe.id,
        method: "regression",
        method_version: "regression_ols_v1+regression_bootstrap_v1",
        settings: bootstrapRecipe.settings,
      },
      payload: {
        ...basePayload,
        estimation: {
          ...basePayload.estimation,
          method_version: "regression_ols_v1",
          regression: {
            method_version: "regression_ols_v1",
            regression_type: "ols",
            outcome: "y",
            predictors: ["x"],
            controls: [],
            observations: 6,
            coefficients: [],
            fit: { aic: 1, bic: 1 },
            predictions: [],
            bootstrap: {
              method_version: "regression_bootstrap_v1",
              algorithm: "indexed_case_resampling_v1",
              alternative: "two_sided",
              interval_policy: "percentile_primary_bca_conditional_v1",
              test_reference: "standard_normal_bootstrap_ratio_v1",
              test_tolerance_policy: "64eps_max_1_original_replicates_v1",
              confidence_level: 0.95,
              requested_replicates: 99,
              usable_replicates: 99,
              minimum_usable_fraction: 0.9,
              seed: 42,
              workers: 2,
              stream_token: "quickpls_indexed_resampling_v1",
              failed_replicates: [],
              jackknife_cases: 6,
              usable_jackknife_cases: 6,
              validation_witness: {
                method_version: "regression_bootstrap_validation_witness_v1",
                terms: ["intercept", "x"],
                successful_bootstrap: Array.from({ length: 99 }, (_, replicate_index) => ({
                  replicate_index,
                  coefficients: [1, 0.5],
                })),
                successful_jackknife: Array.from({ length: 6 }, (_, omitted_case) => ({
                  omitted_case,
                  coefficients: [1, 0.5],
                })),
                failed_jackknife: [],
              },
              coefficients: [],
              warnings: [],
            },
            process: null,
            warnings: [],
          },
        },
      },
    };

    expect(nativeRunFromCanonicalResult(bootstrapEnvelope, bootstrapRecipe)).toMatchObject({
      modelId: null,
      method: "Ordinary Least Squares Regression with Bootstrap",
      name: "Ordinary Least Squares Regression with Bootstrap run",
    });
    const missingTypedConfig = { ...bootstrapRecipe, method_config: undefined };
    expect(nativeRunFromCanonicalResult(bootstrapEnvelope, missingTypedConfig)).toBeNull();
    const wrongIntervals = {
      ...bootstrapRecipe,
      method_config: {
        ...bootstrapRecipe.method_config!,
        bootstrap: { algorithm: "case_resampling" as const, intervals: ["bca", "percentile"] as ["bca", "percentile"] },
      },
    } as unknown as NativeCanonicalAnalysisRecipe;
    expect(nativeRunFromCanonicalResult(bootstrapEnvelope, wrongIntervals)).toBeNull();

    const wrongWitnessTerms = JSON.parse(JSON.stringify(bootstrapEnvelope)) as AnalysisResultEnvelope;
    const wrongTermsBootstrap = (wrongWitnessTerms.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>)
      .estimation.regression!.bootstrap!;
    wrongTermsBootstrap.validation_witness.terms = ["x", "intercept"];
    expect(nativeRunFromCanonicalResult(wrongWitnessTerms, bootstrapRecipe)).toBeNull();

    const incompleteWitness = JSON.parse(JSON.stringify(bootstrapEnvelope)) as AnalysisResultEnvelope;
    const incompleteBootstrap = (incompleteWitness.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>)
      .estimation.regression!.bootstrap!;
    incompleteBootstrap.validation_witness.successful_bootstrap.pop();
    expect(nativeRunFromCanonicalResult(incompleteWitness, bootstrapRecipe)).toBeNull();
  });

  it("hydrates current and legacy prediction archives without relabeling v1 as current CVPAT", () => {
    const basePayload = envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const predictionRecipe = recipe({ settings: { ...recipe().settings, method: "predict" } });
    const prediction = {
      split: "deterministic_complete_case_modulo_4_test_rows",
      training_observations: 75,
      test_observations: 25,
      benchmark: "indicator_average",
      targets: [],
      repeated_kfold: null,
      warnings: [],
    };
    const current = envelope({
      payload: {
        ...basePayload,
        estimation: { ...basePayload.estimation, predict: { ...prediction, method_version: "plspredict_indicator_v2" } },
      },
    });
    const legacy = envelope({
      payload: {
        ...basePayload,
        estimation: { ...basePayload.estimation, predict: { ...prediction, method_version: "plspredict_holdout_v1" } },
      },
    });

    expect(nativeRunFromCanonicalResult(current, predictionRecipe)?.method).toBe("PLSpredict / CVPAT");
    expect(nativeRunFromCanonicalResult(legacy, predictionRecipe)?.method).toBe("Legacy construct-score prediction (v1)");
  });

  it("falls back to legacy workspace content only when canonical records are absent", () => {
    const legacyRun: AnalysisRun = {
      id: "legacy-run",
      name: "Legacy run",
      method: "Legacy",
      createdAt: "2020-01-01T00:00:00.000Z",
      seed: 1,
      status: "completed",
      warnings: [],
      fingerprint: "legacy",
    };
    const reconciled = reconcileNativeCanonicalProject({
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      workspace: {
        nodes: [{ id: "legacy", type: "construct", position: { x: 10, y: 20 }, data: { label: "Legacy", shortName: "L", mode: "reflective", indicators: ["l1"] } }],
        edges: [],
        runs: [legacyRun],
      },
    });

    expect(reconciled.modelSource).toBe("workspace");
    expect(reconciled.resultSource).toBe("workspace");
    expect(reconciled.nodes[0].data.label).toBe("Legacy");
    expect(reconciled.runs).toEqual([legacyRun]);
  });

  it("never trusts an unknown active model id and resolves the latest result recipe", () => {
    const older = recipe({ id: "recipe-old", model: model("model-old", "Old") });
    const newer = recipe({ id: "recipe-new", model: model("model-new", "New") });
    const oldResult = envelope({
      id: "old-result",
      provenance: { ...envelope().provenance, recipe_id: "recipe-old", completed_at: "2026-01-01T00:00:00.000Z" },
    });
    const newResult = envelope({
      id: "new-result",
      provenance: { ...envelope().provenance, recipe_id: "recipe-new", completed_at: "2026-08-01T00:00:00.000Z" },
    });

    expect(resolveActiveCanonicalModel([], [older, newer], [oldResult, newResult], "unknown")?.id).toBe("model-new");
  });

  it("keeps standalone NCA recipe placeholders out of editable model resolution", () => {
    const emptyWireModel: NativeCanonicalModelSpec = {
      id: "standalone-recipe-model",
      name: "Necessary Condition Analysis (standalone)",
      constructs: [],
      paths: [],
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    };
    const ncaRecipe = recipe({
      id: "recipe-nca",
      model: emptyWireModel,
      settings: {
        ...recipe().settings,
        method: "nca",
        preprocessing: "unstandardized",
      },
      metadata: {
        status: "validated_nca_v2_bounded_scope",
        nca_x: "x1",
        nca_y: "y1",
        nca_ceiling: "both",
        nca_permutation_samples: "999",
      },
    });
    const ncaEnvelope = envelope({
      id: "result-nca",
      provenance: {
        ...envelope().provenance,
        recipe_id: "recipe-nca",
        method: "nca",
        method_version: "nca_v2",
        settings: ncaRecipe.settings,
      },
      payload: {
        kind: "pls_pm_v1",
        estimation: {
          ...(envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>).estimation,
          method_version: "nca_v2",
          iterations: 0,
          used_observations: 8,
          paths: [],
          effects: [],
          r_squared: {},
          nca: {
            method_version: "nca_v2",
            ceiling: "both",
            permutation_samples: 999,
            usable_permutations: 999,
            x: "x1",
            y: "y1",
            observations: 8,
            scope: { minimum_x: 1, maximum_x: 8, minimum_y: 1, maximum_y: 9 },
            ce_fdh_peers: [{ x: 1, y: 1 }, { x: 8, y: 9 }],
            ceilings: [
              { ceiling: "ce_fdh", effect_size: 0.3, permutation_p_value: 0.02, slope: null, intercept: null },
              { ceiling: "cr_fdh", effect_size: 0.2, permutation_p_value: 0.04, slope: 1, intercept: 0 },
            ],
            bottlenecks: [{ ceiling: "ce_fdh", outcome_percent: 10, required_x_percent: 5, status: "required" }],
            warnings: [],
          },
        },
        assessment: (envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>).assessment,
      },
    });

    expect(resolveActiveCanonicalModel([], [ncaRecipe], [ncaEnvelope], null)).toBeNull();
    const run = nativeRunFromCanonicalResult(ncaEnvelope, ncaRecipe)!;
    expect(run).toMatchObject({ modelId: null, method: "Necessary Condition Analysis" });
    expect(run.modelSnapshot).toBeUndefined();

    const reconciled = reconcileNativeCanonicalProject({
      models: [],
      recipes: [ncaRecipe],
      results: [ncaEnvelope],
      activeModelId: null,
      workspace: { nodes: [], edges: [], runs: [] },
    });
    expect(reconciled).toMatchObject({ activeModelId: null, projectModels: [], nodes: [], edges: [], modelSource: "empty" });
    expect(reconciled.runs).toHaveLength(1);

    const intentionalEmptyModel = { ...emptyWireModel, id: "user-empty-model", name: "User model" };
    expect(resolveActiveCanonicalModel([intentionalEmptyModel], [ncaRecipe], [ncaEnvelope], intentionalEmptyModel.id))
      .toEqual(intentionalEmptyModel);
  });

  it("does not surface a canonical result that has no matching canonical recipe", () => {
    const reconciled = reconcileNativeCanonicalProject({
      models: [model()],
      recipes: [],
      results: [envelope()],
      activeModelId: "model-1",
      workspace: { nodes: [], edges: [], runs: [] },
    });

    expect(reconciled.resultSource).toBe("canonical");
    expect(reconciled.runs).toEqual([]);
  });

  it("persists only presentation metadata for canonical native workspace runs", () => {
    const payload = envelope().payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v1" }>;
    const run = nativeRunFromCanonicalResult(envelope(), recipe())!;
    run.bootstrap = {
      method_version: "pls_bootstrap_v1",
      plan: { replicates: 100, master_seed: 42, operation: "bootstrap" },
      usable_replicates: 100,
      failed_replicates: [],
      percentile: { confidence_level: 0.95, parameters: [] },
    };
    run.permutation = {
      method_version: "pls_permutation_v1",
      plan: { permutations: 99, master_seed: 42, operation: "permutation" },
      parameters: [],
    };
    run.result = payload.estimation;
    run.assessment = payload.assessment;

    const [presentation] = compactNativeWorkspaceRuns([run]);

    expect(presentation).toMatchObject({
      id: run.id,
      modelId: "model-1",
      method: run.method,
      logs: run.logs,
      modelSnapshot: run.modelSnapshot,
    });
    expect(presentation).not.toHaveProperty("result");
    expect(presentation).not.toHaveProperty("assessment");
    expect(presentation).not.toHaveProperty("bootstrap");
    expect(presentation).not.toHaveProperty("permutation");
    expect(presentation).not.toHaveProperty("provenance");
  });
});
