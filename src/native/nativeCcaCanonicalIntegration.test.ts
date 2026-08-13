import { describe, expect, it } from "vitest";
import { tablesToCsv } from "../domain/resultTables";
import type {
  AnalysisResultEnvelope,
  AssessmentResult,
  NativeCanonicalAnalysisRecipe,
  NativeCanonicalModelSpec,
  PlsResult,
} from "../types";
import { nativeRunFromCanonicalResult } from "./nativeCanonicalProject";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { buildNativeResultNavigation } from "./nativeResults";

const settings = {
  method: "cca",
  weighting_scheme: "path",
  tolerance: 1e-7,
  max_iterations: 3_000,
  bootstrap_samples: 0,
  studentized_inner_samples: 0,
  permutation_samples: 0,
  seed: 20_260_719,
  workers: 1,
  confidence_level: 0.95,
  preprocessing: "standardized",
  missing_data: "listwise_deletion",
  case_weight_column: null,
} as const;

const model: NativeCanonicalModelSpec = {
  id: "00000000-0000-4000-8000-000000000117",
  name: "CCA retention model",
  constructs: [
    { id: "x", name: "Market Capability", short_name: "MC", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "z", name: "Relationship Quality", short_name: "RQ", mode: "reflective", indicators: ["z1", "z2"] },
    { id: "y", name: "Retention Intent", short_name: "RI", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "z" }, { source: "z", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

const recipe: NativeCanonicalAnalysisRecipe = {
  schema_version: 2,
  id: "00000000-0000-4000-8000-000000000017",
  created_at: "2026-08-11T04:06:11.700Z",
  dataset_fingerprint: "v2:da6d5f2557259c19c2728f2d115bb50475ec1f6f9345b48a0250dfa7e6e98dec",
  model,
  settings,
  metadata: { status: "validated_v1_2_3_cca_bounded_scope" },
};

const estimation: PlsResult = {
  method_version: "cca_composite_residual_v1",
  converged: true,
  iterations: 6,
  used_observations: 132,
  omitted_observations: 0,
  outer_estimates: [
    { construct: "x", indicator: "x1", weight: 0.505, loading: 0.993 },
    { construct: "x", indicator: "x2", weight: 0.503, loading: 0.992 },
    { construct: "z", indicator: "z1", weight: 0.507, loading: 0.988 },
    { construct: "z", indicator: "z2", weight: 0.506, loading: 0.988 },
    { construct: "y", indicator: "y1", weight: 0.507, loading: 0.987 },
    { construct: "y", indicator: "y2", weight: 0.506, loading: 0.986 },
  ],
  paths: [
    { source: "x", target: "z", coefficient: 0.5191914418388025 },
    { source: "z", target: "y", coefficient: 0.763190814722803 },
  ],
  effects: [
    { source: "x", target: "z", direct: 0.5191914418388025, indirect: 0, total: 0.5191914418388025 },
    { source: "x", target: "y", direct: 0, indirect: 0.3962421394940626, total: 0.3962421394940626 },
    { source: "z", target: "y", direct: 0.763190814722803, indirect: 0, total: 0.763190814722803 },
  ],
  r_squared: { z: 0.2695597532786549, y: 0.582460219677255 },
  cca: {
    method_version: "cca_composite_residual_v1",
    model: "recursive_standardized_composite_path_model_v1",
    correlations: [
      { left: "x", right: "z", observed: 0.5191914418388025, reproduced: 0.5191914418388025, residual: 0, absolute_residual: 0 },
      { left: "x", right: "y", observed: 0.6725159957813696, reproduced: 0.3962421394940626, residual: 0.27627385628730694, absolute_residual: 0.27627385628730694 },
      { left: "z", right: "y", observed: 0.7631908147228031, reproduced: 0.763190814722803, residual: 1.1102230246251565e-16, absolute_residual: 1.1102230246251565e-16 },
    ],
    max_absolute_residual: 0.27627385628730694,
    warnings: ["CCA inference and decision thresholds are outside this descriptive scope."],
  },
  warnings: ["CCA inference and decision thresholds are outside this descriptive scope."],
};

const assessment: AssessmentResult = {
  method_version: "pls_assessment_v7",
  rho_a_method_version: "dijkstra_henseler_rho_a_v1",
  construct_quality: [
    { construct: "x", cronbach_alpha: 0.984926, rho_a: 0.984934, rho_c: 0.99252, ave: 0.98515 },
    { construct: "z", cronbach_alpha: 0.975489, rho_a: 0.975668, rho_c: 0.987892, ave: 0.976074 },
    { construct: "y", cronbach_alpha: 0.971854, rho_a: 0.972111, rho_c: 0.985865, ave: 0.972258 },
  ],
  cross_loadings: [
    { indicator: "x1", assigned_construct: "x", construct: "x", loading: 0.992612 },
    { indicator: "x2", assigned_construct: "x", construct: "x", loading: 0.992482 },
    { indicator: "z1", assigned_construct: "z", construct: "z", loading: 0.987774 },
    { indicator: "z2", assigned_construct: "z", construct: "z", loading: 0.988154 },
    { indicator: "y1", assigned_construct: "y", construct: "y", loading: 0.986575 },
    { indicator: "y2", assigned_construct: "y", construct: "y", loading: 0.985678 },
  ],
  fornell_larcker: {
    constructs: ["x", "z", "y"],
    values: [
      [0.992547, 0.519191, 0.672516],
      [0.519191, 0.987964, 0.763191],
      [0.672516, 0.763191, 0.986126],
    ],
  },
  r_squared: { z: 0.2695597532786549, y: 0.582460219677255 },
  structural_quality: [
    { construct: "z", predictor_count: 1, r_squared: 0.2695597532786549, adjusted_r_squared: 0.26394098215002915 },
    { construct: "y", predictor_count: 1, r_squared: 0.582460219677255, adjusted_r_squared: 0.5792483752132339 },
  ],
  structural_vif: [
    { predictor_construct: "x", target_construct: "z", vif: 1 },
    { predictor_construct: "z", target_construct: "y", vif: 1 },
  ],
  formative_indicator_vif: [],
  f_squared: [
    { source_construct: "x", target_construct: "z", included_r_squared: 0.2695597532786549, excluded_r_squared: 0, f_squared: 0.3690373777849743 },
    { source_construct: "z", target_construct: "y", included_r_squared: 0.582460219677255, excluded_r_squared: 0, f_squared: 1.394981381718963 },
  ],
  model_fit: {
    saturated: { srmr: 0.010603098401307331, d_uls: 0.0023609396098639274 },
    estimated: { srmr: 0.11847393053696653, d_uls: 0.2947575165544374 },
  },
  warnings: [],
};

const envelope: AnalysisResultEnvelope = {
  schema_version: 1,
  id: "f73f229f-7aef-4efd-83c4-f252ec3f2052",
  status: "completed",
  provenance: {
    recipe_id: recipe.id,
    dataset_fingerprint: recipe.dataset_fingerprint,
    method: "cca",
    method_version: "pls_pm_v1+cca_composite_residual_v1+pls_mediation_v1+pls_assessment_v7",
    engine_version: "2.45.0",
    seed: settings.seed,
    settings,
    started_at: "2026-08-11T04:06:11.722386100Z",
    completed_at: "2026-08-11T04:06:11.875286100Z",
  },
  diagnostics: [],
  payload: { kind: "pls_pm_v1", estimation, assessment },
};

describe("native canonical CCA release gate", () => {
  it("carries a runner-shaped CCA result through navigation, labels, and truthful export composition", () => {
    const run = nativeRunFromCanonicalResult(envelope, recipe);
    expect(run).not.toBeNull();
    if (!run) throw new Error("Expected the canonical completed CCA envelope to create a run.");

    expect(run).toMatchObject({
      id: envelope.id,
      modelId: model.id,
      method: "CCA composite residual diagnostics",
      name: "CCA composite residual diagnostics run",
      assessment: { method_version: "pls_assessment_v7" },
      provenance: { method: "cca", method_version: envelope.provenance.method_version },
    });
    expect(run.modelSnapshot?.nodes.map((node) => node.data.label)).toEqual([
      "Market Capability",
      "Relationship Quality",
      "Retention Intent",
    ]);

    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);
    expect(navigation.defaultItemId).toBe("cca_residual_summary");
    expect(navigation.groups.find((group) => group.id === "assessment")?.items.map((item) => item.id)).toEqual([
      "cca_residual_summary",
      "cca_composite_residuals",
    ]);
    expect(table("cca_residual_summary")).toMatchObject({
      title: "Residual summary",
      rows: expect.arrayContaining([
        ["Correlation pairs", "3"],
        ["Maximum absolute residual", "0.276274"],
      ]),
    });
    expect(table("cca_composite_residuals")?.rows).toEqual([
      ["Market Capability ↔ Relationship Quality", "0.519191", "0.519191", "0.000000", "0.000000"],
      ["Market Capability ↔ Retention Intent", "0.672516", "0.396242", "0.276274", "0.276274"],
      ["Relationship Quality ↔ Retention Intent", "0.763191", "0.763191", "0.000000", "0.000000"],
    ]);
    expect(table("construct_reliability")?.rows.map((row) => row[0])).toEqual([
      "Market Capability",
      "Relationship Quality",
      "Retention Intent",
    ]);

    const exportTables = [
      ...navigation.tables.filter((candidate) => candidate.id !== "run_provenance"),
      nativeRunProvenanceTable(run),
    ];
    const provenance = exportTables.at(-1)!;
    expect(exportTables.map((candidate) => candidate.id)).toEqual(expect.arrayContaining([
      "cca_residual_summary",
      "cca_composite_residuals",
      "construct_reliability",
      "run_provenance",
    ]));
    expect(provenance).toMatchObject({ title: "Run provenance", columns: ["Field", "Value"] });
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["Method", "CCA composite residual diagnostics"],
      ["Method version", envelope.provenance.method_version],
      ["Weighting scheme", "path"],
      ["Preprocessing", "standardized"],
    ]));
    expect(provenance.rows.map(([field]) => field)).not.toEqual(expect.arrayContaining([
      "Seed",
      "Confidence level",
      "Workers",
      "Bootstrap samples",
      "Studentized inner samples",
      "Permutation samples",
      "Case-weight variable",
    ]));
    expect(navigation.groups.some((group) => group.id === "inference")).toBe(false);

    const csv = tablesToCsv(exportTables);
    expect(csv).toContain("Residual summary");
    expect(csv).toContain("Composite residuals");
    expect(csv).toContain("Run provenance");
    expect(csv).not.toMatch(/\bN\/?A\b|Seed|Confidence level|Workers|Bootstrap samples|Studentized inner samples|Permutation samples|Case-weight variable/i);
    expect(exportTables.every((candidate) => candidate.rows.length > 0)).toBe(true);
  });
});
