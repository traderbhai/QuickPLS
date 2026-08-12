import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ConstructData } from "../types";
import { nativeCalculationSettingsForMode } from "./nativeCalculationMode";
import {
  NATIVE_ANALYSIS_RECIPE_BOUNDS,
  NATIVE_ANALYSIS_RECIPE_DESCRIPTORS,
  NATIVE_ANALYSIS_RECIPE_KINDS,
  NativeAnalysisRecipeBuildError,
  buildNativeAnalysisRecipe,
  nativeAnalysisRecipeDescriptor,
  nativeAnalysisRecipeKindForCalculationMode,
  nativeAnalysisRecipeKindForSettings,
  type NativeAnalysisRecipeBuildInput,
  type NativeAnalysisRecipeKind,
} from "./nativeAnalysisRecipe";

const recipeId = "11111111-1111-4111-8111-111111111111";
const modelId = "22222222-2222-4222-8222-222222222222";
const createdAt = "2026-08-10T08:00:00.000Z";

const baseSettings: AnalysisUiSettings = {
  method: "pls_pm",
  weightingScheme: "path",
  tolerance: 1e-7,
  maxIterations: 3_000,
  preprocessing: "standardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_718,
  workers: 1,
  confidenceLevel: 0.95,
  caseWeightColumn: null,
  groupColumn: null,
  groupAValue: null,
  groupBValue: null,
  ipmaTargets: null,
  groupMethods: "micom,mga_permutation",
  groupPermutationSamples: 5_000,
  micomConfiguralConfirmed: true,
  segmentCount: 2,
  segmentStarts: 10,
  minimumSegmentShare: 0.1,
  cbsemModelType: "sem",
  cbsemMeanStructure: false,
  cbsemStandardization: "std_all",
  cbsemGroupColumn: null,
  cbsemInvarianceSteps: "configural,metric,scalar",
  cbsemBootstrapSamples: 0,
  pcaVariables: null,
  pcaComponentRule: "kaiser",
  pcaComponents: 2,
  pcaVarianceThreshold: 0.80,
  regressionType: "ols",
  regressionOutcome: null,
  regressionPredictors: null,
  regressionControls: null,
  robustSe: "hc3",
  processModel: "mediation",
  processX: null,
  processM: null,
  processW: null,
  ncaX: null,
  ncaY: null,
  ncaCeiling: "both",
  ncaPermutationSamples: 999,
};

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
    position: { x: 400, y: 100 },
    data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  },
];

const edges: Edge[] = [{ id: "path-x-y", source: "x", target: "y" }];

function makeInput(
  kind: NativeAnalysisRecipeKind,
  patch: Partial<AnalysisUiSettings> = {},
  model: { nodes?: readonly Node<ConstructData>[]; edges?: readonly Edge[] } = {},
): NativeAnalysisRecipeBuildInput {
  return {
    kind,
    recipeId,
    modelId,
    createdAt,
    datasetFingerprint: "sha256-fixture",
    projectName: "Fixture project",
    nodes: model.nodes ?? nodes,
    edges: model.edges ?? edges,
    settings: { ...baseSettings, ...patch },
  };
}

function expectFieldError(input: NativeAnalysisRecipeBuildInput, field: string) {
  try {
    buildNativeAnalysisRecipe(input);
    throw new Error("Expected recipe construction to fail");
  } catch (error) {
    expect(error).toBeInstanceOf(NativeAnalysisRecipeBuildError);
    expect((error as NativeAnalysisRecipeBuildError).field).toBe(field);
  }
}

describe("native analysis recipe descriptors", () => {
  it("covers every native calculation entry exactly once with an executable Rust method", () => {
    expect(NATIVE_ANALYSIS_RECIPE_KINDS).toHaveLength(18);
    expect(new Set(NATIVE_ANALYSIS_RECIPE_KINDS).size).toBe(18);
    expect(NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.every((item) => item.scopeStatus === "validated")).toBe(true);
    expect(NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.map((item) => item.engineMethod)).not.toContain("bootstrap");
    expect(nativeAnalysisRecipeDescriptor("pls_bootstrap").engineMethod).toBe("pls_pm");
    expect(nativeAnalysisRecipeDescriptor("pls_permutation").engineMethod).toBe("pls_pm");
    expect(nativeAnalysisRecipeDescriptor("predict").label).toBe("PLSpredict / CVPAT");
    expect(nativeAnalysisRecipeDescriptor("cca")).toMatchObject({
      engineMethod: "cca",
      family: "Assessment",
      label: "CCA composite residual diagnostics",
    });
    expect(nativeAnalysisRecipeDescriptor("ipma")).toMatchObject({
      engineMethod: "ipma",
      family: "Assessment",
      label: "Importance-Performance Map Analysis",
    });
    expect(nativeAnalysisRecipeDescriptor("cbsem")).toMatchObject({
      engineMethod: "cbsem",
      family: "CB-SEM",
      label: "CB-SEM / CFA",
    });
    expect(nativeAnalysisRecipeDescriptor("gsca")).toMatchObject({
      engineMethod: "gsca",
      family: "Component Models",
      label: "GSCA",
      scopeMetadata: "validated_gsca_als_v2_bounded_scope",
    });
  });

  it("infers the primary calculation modes before falling through to advanced engine methods", () => {
    expect(nativeAnalysisRecipeKindForCalculationMode("pls")).toBe("pls_algorithm");
    expect(nativeAnalysisRecipeKindForCalculationMode("bootstrap")).toBe("pls_bootstrap");
    expect(nativeAnalysisRecipeKindForCalculationMode("permutation")).toBe("pls_permutation");
    expect(nativeAnalysisRecipeKindForCalculationMode("predict")).toBe("predict");
    expect(nativeAnalysisRecipeKindForSettings(baseSettings)).toBe("pls_algorithm");
    expect(nativeAnalysisRecipeKindForSettings({ ...baseSettings, method: "bootstrap", bootstrapSamples: 0 })).toBe("pls_bootstrap");
    expect(nativeAnalysisRecipeKindForSettings({ ...baseSettings, bootstrapSamples: 5_000 })).toBe("pls_bootstrap");
    expect(nativeAnalysisRecipeKindForSettings({ ...baseSettings, method: "bootstrap", bootstrapSamples: 5_000, permutationSamples: 999 })).toBe("pls_permutation");
    expect(nativeAnalysisRecipeKindForSettings({ ...baseSettings, method: "plsc", bootstrapSamples: 5_000, permutationSamples: 999 })).toBe("plsc");
    expect(nativeAnalysisRecipeKindForSettings({ ...baseSettings, method: "predict", bootstrapSamples: 5_000, permutationSamples: 999 })).toBe("predict");
  });
});

describe("native recipe model payload", () => {
  it("maps the Rust model contract without duplicating controls as structural paths", () => {
    const higherOrder: Node<ConstructData> = {
      id: "hoc",
      type: "construct",
      position: { x: 200, y: 300 },
      data: {
        label: "Higher order",
        shortName: "HOC",
        mode: "reflective",
        indicators: ["h1", "h2"],
        semantic: "higher_order",
        higherOrder: { id: "hoc", components: ["x", "y"], method: "two_stage", stage_one_recipe: " stage-one " },
      },
    };
    const interaction: Node<ConstructData> = {
      id: "x-by-y",
      type: "construct",
      position: { x: 250, y: 400 },
      data: {
        label: "X by Y",
        shortName: "XxY",
        mode: "reflective",
        indicators: ["product"],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "y", outcome: "hoc", method: "two_stage_product_score" },
      },
    };
    const mixedEdges: Edge[] = [
      ...edges,
      { id: "control-x-y", source: "x", target: "y", data: { role: "control", controlLabel: " Age " } },
      { id: "cov-x-y", source: "x", target: "y", data: { role: "covariance" } },
      { id: "measurement::x::x1", source: "x", target: "x1" },
    ];
    const sourceNodes = [...nodes, higherOrder, interaction];
    const recipe = buildNativeAnalysisRecipe(makeInput("pls_algorithm", {}, { nodes: sourceNodes, edges: mixedEdges }));

    expect(recipe).toMatchObject({
      schema_version: 2,
      id: recipeId,
      created_at: createdAt,
      dataset_fingerprint: "sha256-fixture",
      model: { id: modelId, name: "Fixture project" },
    });
    expect(recipe.model.paths).toEqual([{ source: "x", target: "y" }]);
    expect(recipe.model.controls).toEqual([{ source: "x", target: "y", label: "Age" }]);
    expect(recipe.model.higher_order_constructs).toEqual([{ id: "hoc", components: ["x", "y"], method: "two_stage", stage_one_recipe: "stage-one" }]);
    expect(recipe.model.interactions).toEqual([{ id: "x-by-y", predictor: "x", moderator: "y", product_construct: "x-by-y", outcome: "hoc", method: "two_stage_product_score" }]);

    recipe.model.constructs[0].indicators.push("local-only");
    expect(sourceNodes[0].data.indicators).toEqual(["x1", "x2"]);
  });

  it("rejects a two-group MGA recipe when its grouping variable is also an indicator", () => {
    expectFieldError(makeInput("mga", {
      method: "mga",
      groupColumn: "x1",
      groupAValue: "A",
      groupBValue: "B",
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: 5_000,
    }), "groupColumn");
  });

  it("serializes a control-only edge as both a structural coefficient and a control declaration", () => {
    const controlOnly: Edge[] = [{ id: "control-x-y", source: "x", target: "y", data: { role: "control", controlLabel: "Age" } }];
    const recipe = buildNativeAnalysisRecipe(makeInput("pls_algorithm", {}, { edges: controlOnly }));
    expect(recipe.model.paths).toEqual([{ source: "x", target: "y" }]);
    expect(recipe.model.controls).toEqual([{ source: "x", target: "y", label: "Age" }]);
  });
});

describe("primary native PLS calculation payloads", () => {
  it("builds the exact mutually exclusive algorithm, bootstrap, and permutation settings", () => {
    const algorithm = buildNativeAnalysisRecipe(makeInput("pls_algorithm", { bootstrapSamples: 5_000, studentizedInnerSamples: 99, permutationSamples: 999 }));
    const bootstrap = buildNativeAnalysisRecipe(makeInput("pls_bootstrap", { bootstrapSamples: 5_000, studentizedInnerSamples: 99, permutationSamples: 999 }));
    const permutation = buildNativeAnalysisRecipe(makeInput("pls_permutation", { bootstrapSamples: 5_000, studentizedInnerSamples: 99, permutationSamples: 999 }));

    expect(algorithm.settings).toMatchObject({ method: "pls_pm", bootstrap_samples: 0, studentized_inner_samples: 0, permutation_samples: 0 });
    expect(bootstrap.settings).toMatchObject({ method: "pls_pm", bootstrap_samples: 5_000, studentized_inner_samples: 99, permutation_samples: 0 });
    expect(permutation.settings).toMatchObject({ method: "pls_pm", bootstrap_samples: 0, studentized_inner_samples: 0, permutation_samples: 999 });
    expect(algorithm.metadata).toEqual({ status: "validated_v1_0_supported_pls_scope" });
    expect(bootstrap.metadata).toEqual({ status: "validated_v1_0_supported_pls_scope" });
    expect(permutation.metadata).toEqual({
      status: "validated_v1_0_freedman_lane_path_randomization_scope",
    });
  });

  it("uses the current native mode defaults and clamps primary sample counts to their dialog bounds", () => {
    expect(buildNativeAnalysisRecipe(makeInput("pls_bootstrap", { bootstrapSamples: 0 })).settings.bootstrap_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.default);
    expect(buildNativeAnalysisRecipe(makeInput("pls_bootstrap", { bootstrapSamples: 1 })).settings.bootstrap_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.minimum);
    expect(buildNativeAnalysisRecipe(makeInput("pls_bootstrap", { bootstrapSamples: 99_999 })).settings.bootstrap_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.maximum);
    expect(buildNativeAnalysisRecipe(makeInput("pls_permutation", { permutationSamples: 0 })).settings.permutation_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.permutationSamples.default);
    expect(buildNativeAnalysisRecipe(makeInput("pls_permutation", { permutationSamples: 1 })).settings.permutation_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.permutationSamples.minimum);
    expect(buildNativeAnalysisRecipe(makeInput("pls_permutation", { permutationSamples: 99_999 })).settings.permutation_samples).toBe(NATIVE_ANALYSIS_RECIPE_BOUNDS.permutationSamples.maximum);
  });

  it("enforces the Rust-qualified studentized bootstrap plan", () => {
    expectFieldError(makeInput("pls_bootstrap", { bootstrapSamples: 998, studentizedInnerSamples: 99 }), "bootstrapSamples");
    expectFieldError(makeInput("pls_bootstrap", { bootstrapSamples: 999, studentizedInnerSamples: 100 }), "studentizedInnerSamples");
    expectFieldError(makeInput("pls_bootstrap", { bootstrapSamples: 999, studentizedInnerSamples: 1_001 }), "studentizedInnerSamples");
    expect(buildNativeAnalysisRecipe(makeInput("pls_bootstrap", { bootstrapSamples: 999, studentizedInnerSamples: 999 })).settings.studentized_inner_samples).toBe(999);
  });
});

describe("common native recipe settings", () => {
  it("serializes every Rust AnalysisSettings field with no browser-only aliases", () => {
    const recipe = buildNativeAnalysisRecipe(makeInput("pls_algorithm", {
      weightingScheme: "factor",
      tolerance: 1e-8,
      maxIterations: 5_000,
      preprocessing: "mean_centered",
      seed: 42,
      workers: 4,
      confidenceLevel: 0.975,
      caseWeightColumn: "ignored-for-unweighted-method",
    }));
    expect(recipe.settings).toEqual({
      method: "pls_pm",
      weighting_scheme: "factor",
      tolerance: 1e-8,
      max_iterations: 5_000,
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      seed: 42,
      workers: 4,
      confidence_level: 0.975,
      preprocessing: "mean_centered",
      missing_data: "listwise_deletion",
      case_weight_column: null,
    });
  });

  it.each([
    ["tolerance", { tolerance: 0 }],
    ["tolerance", { tolerance: 0.1 }],
    ["maxIterations", { maxIterations: 99 }],
    ["maxIterations", { maxIterations: 100_001 }],
    ["maxIterations", { maxIterations: 1_000.5 }],
    ["workers", { workers: 0 }],
    ["workers", { workers: 65 }],
    ["seed", { seed: -1 }],
    ["seed", { seed: 4_294_967_296 }],
    ["confidenceLevel", { confidenceLevel: 0.799 }],
    ["confidenceLevel", { confidenceLevel: 1 }],
  ] as const)("rejects an out-of-contract %s value", (field, patch) => {
    expectFieldError(makeInput("pls_algorithm", patch), field);
  });
});

describe("advanced validated backend family mappings", () => {
  it("maps every direct advanced method to its Rust method and clears unrelated PLS resampling state", () => {
    const configurations: Array<[NativeAnalysisRecipeKind, Partial<AnalysisUiSettings>]> = [
      ["plsc", {}],
      ["wpls", { caseWeightColumn: "weight" }],
      ["cca", {}],
      ["cta_pls", {}],
      ["endogeneity", {}],
      ["nonlinear_effects", {}],
      ["moderated_mediation", {}],
      ["predict", { groupMethods: null }],
      ["mga", { groupColumn: "group", groupAValue: "A", groupBValue: "B", groupMethods: "micom,mga_permutation", groupPermutationSamples: 5_000, micomConfiguralConfirmed: true }],
      ["ipma", { ipmaTargets: "y" }],
      ["cbsem", {}],
      ["pca", { pcaVariables: "x1,x2" }],
      ["gsca", {}],
      ["regression", { regressionOutcome: "y1", regressionPredictors: "x1,x2" }],
      ["nca", { ncaX: "x1", ncaY: "y1" }],
    ];
    for (const [kind, patch] of configurations) {
      const recipe = buildNativeAnalysisRecipe(makeInput(kind, { ...patch, bootstrapSamples: 5_000, studentizedInnerSamples: 99, permutationSamples: 999 }));
      expect(recipe.settings.method).toBe(nativeAnalysisRecipeDescriptor(kind).engineMethod);
      expect(recipe.settings.bootstrap_samples, kind).toBe(0);
      expect(recipe.settings.studentized_inner_samples, kind).toBe(0);
      expect(recipe.settings.permutation_samples, kind).toBe(0);
      expect(recipe.metadata.status, kind).toContain("validated_");
    }
  });

  it("maps WPLS through the Rust settings field and rejects ignored/unsupported settings", () => {
    const recipe = buildNativeAnalysisRecipe(makeInput("wpls", { caseWeightColumn: " weight ", weightingScheme: "factor" }));
    expect(recipe.settings.case_weight_column).toBe("weight");
    expect(recipe.metadata).toEqual({ status: "validated_v1_2_1_wpls_bounded_scope" });
    expectFieldError(makeInput("wpls", { caseWeightColumn: null }), "caseWeightColumn");
    expectFieldError(makeInput("wpls", { caseWeightColumn: "weight", preprocessing: "mean_centered" }), "preprocessing");
    expectFieldError(makeInput("wpls", { caseWeightColumn: "weight", weightingScheme: "pca" }), "weightingScheme");
  });

  it("maps only the bounded standardized CCA composite residual scope", () => {
    const singleIndicatorNodes = nodes.map((node) => ({
      ...node,
      data: { ...node.data, indicators: [node.data.indicators[0]] },
    }));
    const recipe = buildNativeAnalysisRecipe(makeInput("cca", {
      method: "cca",
      weightingScheme: "factor",
      preprocessing: "standardized",
      caseWeightColumn: "ignored",
      bootstrapSamples: 5_000,
      studentizedInnerSamples: 99,
      permutationSamples: 999,
    }, { nodes: singleIndicatorNodes }));

    expect(recipe.settings).toMatchObject({
      method: "cca",
      weighting_scheme: "factor",
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      case_weight_column: null,
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
    });
    expect(recipe.metadata).toEqual({ status: "validated_v1_2_3_cca_bounded_scope" });
    expectFieldError(makeInput("cca", { preprocessing: "mean_centered" }), "preprocessing");
    expectFieldError(makeInput("cca", {}, { nodes: [nodes[0]], edges: [] }), "model");
    expectFieldError(makeInput("cca", {}, { edges: [] }), "model");
    expectFieldError(makeInput("cca", {}, {
      nodes: nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, mode: "formative" as const } } : node),
    }), "model");
    expectFieldError(makeInput("cca", {}, {
      edges: [{ id: "control-x-y", source: "x", target: "y", data: { role: "control" } }],
    }), "model");
    expectFieldError(makeInput("cca", {}, {
      nodes: nodes.map((node) => node.id === "x" ? {
        ...node,
        data: {
          ...node.data,
          semantic: "higher_order" as const,
          higherOrder: { id: "x", components: ["y"], method: "repeated_indicators" as const },
        },
      } : node),
    }), "model");
  });

  it.each(["plsc", "cca", "ipma", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"] as const)("blocks PCA weighting for %s", (kind) => {
    expectFieldError(makeInput(kind, { weightingScheme: "pca", ...(kind === "ipma" ? { ipmaTargets: "y" } : {}) }), "weightingScheme");
  });

  it("keeps current prediction separate from explicitly requested legacy PLS-POS and FIMIX workflows", () => {
    const base = buildNativeAnalysisRecipe(makeInput("predict", { groupMethods: "mga_permutation" }));
    expect(base.metadata).toEqual({ status: "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope" });

    const pos = buildNativeAnalysisRecipe(makeInput("predict", { groupMethods: "pls_pos", segmentCount: 5, segmentStarts: 50, minimumSegmentShare: 0.4 }));
    expect(pos.metadata).toEqual({
      status: "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope",
      segment_count: "5",
      segment_starts: "50",
      minimum_segment_share: "0.4",
    });

    const fimix = buildNativeAnalysisRecipe(makeInput("predict", { groupMethods: "fimix", segmentCount: 3 }));
    expect(fimix.metadata).toMatchObject({ group_methods: "fimix", fimix_classes: "3" });
    expect(fimix.metadata).not.toHaveProperty("segment_count");
    expectFieldError(makeInput("predict", { groupMethods: "fimix", segmentCount: 4 }), "segmentCount");
    expectFieldError(makeInput("predict", { groupMethods: "pls_pos", segmentStarts: 0 }), "segmentStarts");
    expectFieldError(makeInput("predict", { groupMethods: "pls_pos", minimumSegmentShare: 0.41 }), "minimumSegmentShare");
    expectFieldError(makeInput("predict", { groupMethods: "unknown-segmentation" }), "groupMethods");
  });

  it("does not carry a hidden legacy segmentation plan into the compact Prediction dialog workflow", () => {
    const settings = nativeCalculationSettingsForMode(
      { ...baseSettings, groupMethods: "pls_pos,fimix", segmentCount: 3 },
      "predict",
    );
    const recipe = buildNativeAnalysisRecipe(makeInput("predict", settings));

    expect(recipe.metadata).not.toHaveProperty("segment_count");
    expect(recipe.metadata).not.toHaveProperty("fimix_classes");
    expect(recipe.metadata).not.toHaveProperty("group_methods");
  });

  it("maps the explicit bounded joint MICOM and permutation-MGA contract", () => {
    const recipe = buildNativeAnalysisRecipe(makeInput("mga", {
      groupColumn: " Group ",
      groupAValue: " Treatment ",
      groupBValue: " Control ",
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: 10_000,
      micomConfiguralConfirmed: true,
    }));
    expect(recipe.metadata).toEqual({
      status: "validated_micom_v2_and_permutation_mga_v2_bounded_scope",
      mga_group_column: "Group",
      mga_group_a: "Treatment",
      mga_group_b: "Control",
      group_methods: "micom,mga_permutation",
      group_permutation_samples: "10000",
      micom_configural_confirmed: "true",
    });
    expectFieldError(makeInput("mga", { groupColumn: null }), "groupColumn");
    expectFieldError(makeInput("mga", { groupColumn: "group", groupAValue: null, groupBValue: "B" }), "groupAValue");
    expectFieldError(makeInput("mga", { groupColumn: "group", groupAValue: "A", groupBValue: "A" }), "groupBValue");
    expectFieldError(makeInput("mga", { groupColumn: "group", groupAValue: "A", groupBValue: "B", groupPermutationSamples: 4_999 }), "groupPermutationSamples");
    expectFieldError(makeInput("mga", { groupColumn: "group", groupAValue: "A", groupBValue: "B", groupMethods: "micom" }), "groupMethods");
    expectFieldError(makeInput("mga", { groupColumn: "group", groupAValue: "A", groupBValue: "B", micomConfiguralConfirmed: false }), "micomConfiguralConfirmed");
  });

  it("maps one ID-backed endogenous IPMA target into the fixed native recipe scope", () => {
    const recipe = buildNativeAnalysisRecipe(makeInput("ipma", {
      ipmaTargets: " y ",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 5_000,
      studentizedInnerSamples: 99,
      permutationSamples: 999,
      caseWeightColumn: "ignored",
    }));
    expect(recipe.metadata).toEqual({
      status: "validated_v1_2_1_ipma_bounded_scope",
      ipma_targets: "y",
    });
    expect(recipe.settings).toMatchObject({
      method: "ipma",
      weighting_scheme: "path",
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      case_weight_column: null,
    });
    expectFieldError(makeInput("ipma", { ipmaTargets: null }), "ipmaTargets");
    expectFieldError(makeInput("ipma", { ipmaTargets: "x" }), "ipmaTargets");
    expectFieldError(makeInput("ipma", { ipmaTargets: "y,z" }), "ipmaTargets");
    expectFieldError(makeInput("ipma", { ipmaTargets: "missing" }), "ipmaTargets");
    expectFieldError(makeInput("ipma", { ipmaTargets: "y", weightingScheme: "factor" }), "weightingScheme");
    expectFieldError(makeInput("ipma", { ipmaTargets: "y", preprocessing: "mean_centered" }), "preprocessing");
  });

  it("keeps CB-SEM inside the validated raw single-group ML scope", () => {
    const cfa = buildNativeAnalysisRecipe(makeInput(
      "cbsem",
      { cbsemModelType: "cfa", cbsemMeanStructure: false },
      { edges: [] },
    ));
    expect(cfa.metadata).toEqual({
      status: "validated_v1_2_4_cbsem_single_group_bounded_scope",
      cbsem_model_type: "cfa",
      cbsem_estimator: "ml",
      cbsem_input: "raw",
      cbsem_mean_structure: "false",
    });
    expect(cfa.settings).toMatchObject({
      method: "cbsem",
      weighting_scheme: "path",
      preprocessing: "standardized",
      workers: 1,
      bootstrap_samples: 0,
      permutation_samples: 0,
      case_weight_column: null,
    });
    expect(cfa.metadata).not.toHaveProperty("cbsem_standardization");

    const sem = buildNativeAnalysisRecipe(makeInput("cbsem", { cbsemModelType: "sem" }));
    expect(sem.metadata.cbsem_model_type).toBe("sem");
    expect(sem.model.paths).toEqual([{ source: "x", target: "y" }]);
    expectFieldError(makeInput("cbsem", { cbsemModelType: "cfa" }), "cbsemModelType");
    expectFieldError(makeInput("cbsem", { cbsemModelType: "sem" }, { edges: [] }), "cbsemModelType");
    expectFieldError(makeInput("cbsem", { cbsemMeanStructure: true }), "cbsemMeanStructure");
    expectFieldError(makeInput("cbsem", { cbsemStandardization: "std_lv" }), "cbsemStandardization");
    expectFieldError(makeInput("cbsem", { cbsemGroupColumn: "group" }), "cbsemGroupColumn");
    expectFieldError(makeInput("cbsem", { cbsemBootstrapSamples: 999 }), "cbsemBootstrapSamples");
  });

  it("builds only the fixed recursive GSCA ALS v2 scope", () => {
    const mixedNodes = [
      { ...nodes[0], data: { ...nodes[0].data, mode: "formative" as const } },
      nodes[1],
    ];
    const recipe = buildNativeAnalysisRecipe(makeInput("gsca", {
      method: "gsca",
      weightingScheme: "factor",
      preprocessing: "mean_centered",
      tolerance: 0.01,
      maxIterations: 100,
      workers: 8,
      bootstrapSamples: 999,
      permutationSamples: 999,
      caseWeightColumn: "weight",
    }, { nodes: mixedNodes }));
    expect(recipe.settings).toMatchObject({
      method: "gsca",
      weighting_scheme: "path",
      preprocessing: "standardized",
      tolerance: 1e-7,
      max_iterations: 3_000,
      workers: 1,
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      case_weight_column: null,
    });
    expect(recipe.metadata).toEqual({ status: "validated_gsca_als_v2_bounded_scope" });
    expect(recipe.model.constructs.map((construct) => construct.mode)).toEqual(["formative", "reflective"]);

    expectFieldError(makeInput("gsca", {}, { nodes: [nodes[0]], edges: [] }), "model");
    expectFieldError(makeInput("gsca", {}, { edges: [] }), "model");
    expectFieldError(makeInput("gsca", {}, { nodes: [...nodes, {
      id: "z",
      position: { x: 700, y: 100 },
      data: { label: "Isolated", shortName: "Z", mode: "reflective", indicators: ["z1"] },
    }] }), "model");
    expectFieldError(makeInput("gsca", {}, { edges: [...edges, { id: "y-x", source: "y", target: "x" }] }), "model");
    expectFieldError(makeInput("gsca", {}, { edges: [{ ...edges[0], data: { role: "control" } }] }), "model");
    expectFieldError(makeInput("gsca", {}, { edges: [...edges, { id: "cov", source: "x", target: "y", data: { role: "covariance" } }] }), "model");
  });

  it("emits only PCA metadata that changes the Rust calculation", () => {
    const kaiser = buildNativeAnalysisRecipe(makeInput("pca", { pcaVariables: "x1, x2", pcaComponentRule: "kaiser", pcaComponents: 9 }));
    expect(kaiser.metadata).toEqual({ status: "validated_pca_v1_bounded_scope", pca_variables: "x1,x2", pca_component_rule: "kaiser" });
    expect(kaiser.settings).toMatchObject({ method: "pca", weighting_scheme: "path", preprocessing: "standardized", bootstrap_samples: 0, permutation_samples: 0, case_weight_column: null });
    const fixed = buildNativeAnalysisRecipe(makeInput("pca", { pcaVariables: "x1,x2,x3,x4", pcaComponentRule: "fixed", pcaComponents: 4 }));
    expect(fixed.metadata.pca_components).toBe("4");
    const threshold = buildNativeAnalysisRecipe(makeInput("pca", { pcaVariables: "x1,x2", pcaComponentRule: "variance_threshold", pcaVarianceThreshold: 0.85 }));
    expect(threshold.metadata.pca_variance_threshold).toBe("0.85");
    expectFieldError(makeInput("pca", { pcaVariables: "x1" }), "pcaVariables");
    expectFieldError(makeInput("pca", { pcaVariables: "x1,x1" }), "pcaVariables");
    expectFieldError(makeInput("pca", { pcaVariables: "x1,x2", pcaComponentRule: "fixed", pcaComponents: 3 }), "pcaComponents");
    expectFieldError(makeInput("pca", { pcaVariables: "x1,x2", pcaComponentRule: "fixed", pcaComponents: 51 }), "pcaComponents");
    expectFieldError(makeInput("pca", { pcaVariables: "x1,x2", pcaComponentRule: "variance_threshold", pcaVarianceThreshold: 1 }), "pcaVarianceThreshold");
  });

  it("maps OLS with its explicit HC3 contract and preserves bounded hidden regression variants", () => {
    const common = { regressionOutcome: "y", regressionPredictors: "x, z, x", regressionControls: "age" } satisfies Partial<AnalysisUiSettings>;
    const ols = buildNativeAnalysisRecipe(makeInput("regression", common));
    expect(ols.metadata).toEqual({
      status: "validated_regression_ols_v1_bounded_scope",
      regression_type: "ols",
      regression_outcome: "y",
      regression_predictors: "x,z",
      regression_controls: "age",
      robust_se: "hc3",
    });
    expect(ols.settings).toMatchObject({ weighting_scheme: "path", preprocessing: "unstandardized", confidence_level: 0.95 });

    const logistic = buildNativeAnalysisRecipe(makeInput("regression", { ...common, regressionType: "logistic" }));
    expect(logistic.metadata.regression_type).toBe("logistic");

    const mediation = buildNativeAnalysisRecipe(makeInput("regression", {
      ...common,
      regressionType: "process",
      processModel: "mediation",
      processX: "x",
      processM: "m",
    }));
    expect(mediation.metadata).toMatchObject({
      status: "validated_v1_2_2_process_mediation_bounded_scope",
      process_model: "mediation",
      process_x: "x",
      process_m: "m",
    });
    expect(mediation.metadata).not.toHaveProperty("process_w");

    const moderation = buildNativeAnalysisRecipe(makeInput("regression", {
      ...common,
      regressionType: "process",
      processModel: "moderation",
      processX: "x",
      processW: "w",
    }));
    expect(moderation.metadata).toMatchObject({ process_model: "moderation", process_x: "x", process_w: "w" });
    expect(moderation.metadata).not.toHaveProperty("process_m");

    expectFieldError(makeInput("regression", { regressionOutcome: null, regressionPredictors: "x" }), "regressionOutcome");
    expectFieldError(makeInput("regression", { regressionOutcome: "y", regressionPredictors: null }), "regressionPredictors");
    expectFieldError(makeInput("regression", { ...common, robustSe: "hc4" }), "robustSe");
    expectFieldError(makeInput("regression", { ...common, regressionType: "process", processModel: "moderated_mediation", processX: "x", processM: "m", processW: "w" }), "processModel");
  });

  it("maps NCA variable, ceiling, and permutation metadata with its independent bounds", () => {
    const recipe = buildNativeAnalysisRecipe(makeInput("nca", { ncaX: " x ", ncaY: " y ", ncaCeiling: "cr_fdh", ncaPermutationSamples: 10_000 }));
    expect(recipe.metadata).toEqual({
      status: "validated_nca_v2_bounded_scope",
      nca_x: "x",
      nca_y: "y",
      nca_ceiling: "cr_fdh",
      nca_permutation_samples: "10000",
    });
    expect(recipe.settings).toMatchObject({ weighting_scheme: "path", preprocessing: "unstandardized" });
    expectFieldError(makeInput("nca", { ncaX: null, ncaY: "y" }), "ncaX");
    expectFieldError(makeInput("nca", { ncaX: "x", ncaY: null }), "ncaY");
    expectFieldError(makeInput("nca", { ncaX: "x", ncaY: "y", ncaPermutationSamples: 0 }), "ncaPermutationSamples");
    expectFieldError(makeInput("nca", { ncaX: "x", ncaY: "x" }), "ncaY");
  });
});

describe("native recipe identity contract", () => {
  it.each([
    ["recipeId", { recipeId: "not-a-uuid" }],
    ["modelId", { modelId: "not-a-uuid" }],
    ["createdAt", { createdAt: "not-a-date" }],
    ["createdAt", { createdAt: "08/10/2026" }],
    ["datasetFingerprint", { datasetFingerprint: "  " }],
    ["projectName", { projectName: "  " }],
  ] as const)("rejects invalid %s before producing a wire payload", (field, patch) => {
    expectFieldError({ ...makeInput("pls_algorithm"), ...patch }, field);
  });
});
