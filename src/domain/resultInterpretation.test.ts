import { describe, expect, it } from "vitest";
import { buildResultInterpretation, findingsByGroup, rowSpecificInterpretation } from "./resultInterpretation";
import type { AnalysisRun, AssessmentResult, PlsResult } from "../types";
import { processV2Run } from "../native/nativeProcessTestFixture";

const resultFixture: PlsResult = {
  method_version: "pls_pm_v1",
  converged: true,
  iterations: 8,
  used_observations: 120,
  omitted_observations: 0,
  outer_estimates: [
    { construct: "competence", indicator: "COMP1", loading: 0.91, weight: 0.4 },
    { construct: "competence", indicator: "COMP2", loading: 0.35, weight: 0.3 },
    { construct: "loyalty", indicator: "LOY1", loading: 0.65, weight: 0.5 },
  ],
  paths: [
    { source: "competence", target: "satisfaction", coefficient: 0.82 },
    { source: "satisfaction", target: "loyalty", coefficient: -0.21 },
    { source: "likeability", target: "loyalty", coefficient: 0.01 },
  ],
  effects: [],
  mediation: {
    method_version: "mediation_v1",
    tolerance: 1e-9,
    estimates: [
      { source: "competence", target: "loyalty", direct: -0.21, indirect: 0.61, total: 0.40, variance_accounted_for: 1.5, classification: "competitive_partial", warning: null },
    ],
    warnings: [],
  },
  r_squared: { satisfaction: 0.76, loyalty: 0.18 },
  warnings: [],
};

const assessmentFixture: AssessmentResult = {
  method_version: "assessment_v1",
  construct_quality: [
    { construct: "competence", cronbach_alpha: 0.82, rho_c: 0.88, ave: 0.62, rho_a: 0.83 },
    { construct: "loyalty", cronbach_alpha: 0.66, rho_c: 0.69, ave: 0.48, rho_a: 0.67 },
  ],
  cross_loadings: [
    { indicator: "COMP2", assigned_construct: "competence", construct: "competence", loading: 0.35 },
    { indicator: "COMP2", assigned_construct: "competence", construct: "loyalty", loading: 0.58 },
  ],
  fornell_larcker: { constructs: ["competence", "loyalty"], values: [[0.8, 0.6], [0.6, 0.7]] },
  htmt_plus: {
    constructs: ["competence", "loyalty"],
    correlation_type: "pearson",
    absolute_correlations: true,
    cells: [
      [{ value: null, status: "not_applicable", reason: "diagonal" }, { value: 0.93, status: "available", reason: null }],
      [{ value: 0.93, status: "available", reason: null }, { value: null, status: "not_applicable", reason: "diagonal" }],
    ],
  },
  r_squared: { satisfaction: 0.76, loyalty: 0.18 },
  structural_quality: [
    { construct: "satisfaction", predictor_count: 1, r_squared: 0.76, adjusted_r_squared: 0.75 },
    { construct: "loyalty", predictor_count: 2, r_squared: 0.18, adjusted_r_squared: 0.16 },
  ],
  structural_vif: [
    { target_construct: "loyalty", predictor_construct: "satisfaction", vif: 5.2 },
  ],
  formative_indicator_vif: [],
  f_squared: [
    { source_construct: "competence", target_construct: "satisfaction", included_r_squared: 0.76, excluded_r_squared: 0.31, f_squared: 1.875 },
  ],
  blindfolding: {
    settings: { omission_distance: 7, selection: "validated", missing_value_treatment: "listwise" },
    constructs: [{ construct: "loyalty", q_squared: -0.04, prediction_error_sum_squares: 10, observation_sum_squares: 9 }],
  },
  warnings: [],
};

const runFixture: AnalysisRun = {
  id: "run-1",
  name: "Fixture run",
  method: "PLS path modeling core",
  createdAt: "2026-07-23T00:00:00.000Z",
  seed: 20260723,
  status: "completed",
  warnings: ["Validated for documented QuickPLS scope."],
  fingerprint: "fixture",
  result: resultFixture,
  assessment: assessmentFixture,
};

describe("result-specific interpretation", () => {
  it("creates value-specific findings for paths, loadings, validity, VIF, f2, and missing inference", () => {
    const interpretation = buildResultInterpretation({ run: runFixture });
    const ids = interpretation.findings.map((finding) => finding.id);
    expect(ids).toContain("path.strongest.competence.satisfaction");
    expect(ids).toContain("path.negative.satisfaction.loyalty");
    expect(ids).toContain("path.near_zero.likeability.loyalty");
    expect(ids).toContain("loading.weak.competence.COMP2");
    expect(ids).toContain("loading.review.loyalty.LOY1");
    expect(ids).toContain("ave.low.loyalty");
    expect(ids).toContain("reliability.low.loyalty");
    expect(ids).toContain("htmt.competence.loyalty");
    expect(ids).toContain("cross_loading.COMP2");
    expect(ids).toContain("vif.satisfaction.loyalty");
    expect(ids).toContain("f2.strongest.competence.satisfaction");
    expect(ids).toContain("inference.missing");
  });

  it("classifies interpretation checklist groups deterministically", () => {
    const grouped = findingsByGroup(buildResultInterpretation({ run: runFixture }).findings);
    expect(grouped.must.length).toBeGreaterThan(0);
    expect(grouped.recommended.length).toBeGreaterThan(0);
    expect(grouped.report.some((item) => item.reportSentence.includes("R2"))).toBe(true);
  });

  it("adds SEM diagram advisor findings from model shape", () => {
    const interpretation = buildResultInterpretation({
      run: runFixture,
      nodes: [
        { id: "competence", data: { label: "Competence", mode: "reflective", indicators: ["COMP1"] } },
        { id: "satisfaction", data: { label: "Satisfaction", mode: "reflective", indicators: ["SAT1"] } },
        { id: "loyalty", data: { label: "Loyalty", mode: "reflective", indicators: ["LOY1"] } },
        { id: "interaction", data: { label: "Interaction", semantic: "interaction", mode: "reflective", indicators: [] } },
      ],
      edges: [
        { source: "competence", target: "satisfaction" },
        { source: "satisfaction", target: "loyalty" },
        { source: "interaction", target: "loyalty" },
      ],
    });
    const ids = interpretation.diagramAdvice.map((finding) => finding.id);
    expect(ids).toContain("advisor.mediation_shape");
    expect(ids).toContain("advisor.moderation_shape");
    expect(ids).toContain("advisor.enable_bootstrap");
  });

  it("interprets bootstrap confidence intervals by whether they include zero", () => {
    const run: AnalysisRun = {
      ...runFixture,
      bootstrap: {
        method_version: "bootstrap_v1",
        plan: { replicates: 999, master_seed: 1, operation: "bootstrap" },
        usable_replicates: 999,
        failed_replicates: [],
        percentile: {
          confidence_level: 0.95,
          parameters: [
            { parameter: "competence -> satisfaction", original: 0.82, bootstrap_mean: 0.81, bias: 0.01, standard_error: 0.1, lower: 0.6, upper: 0.9, usable_replicates: 999 },
            { parameter: "satisfaction -> loyalty", original: -0.21, bootstrap_mean: -0.2, bias: 0.01, standard_error: 0.2, lower: -0.5, upper: 0.1, usable_replicates: 999 },
          ],
        },
      },
    };
    const findings = new Map(buildResultInterpretation({ run }).findings.filter((finding) => finding.id.startsWith("bootstrap.percentile")).map((finding) => [finding.id, finding.severity]));
    expect(findings.get("bootstrap.percentile.competence -> satisfaction")).toBe("good");
    expect(findings.get("bootstrap.percentile.satisfaction -> loyalty")).toBe("caution");
  });

  it("uses dedicated PROCESS point and bootstrap inference without generic PLS reinterpretation", () => {
    const pointRun = processV2Run(false);
    const point = buildResultInterpretation({
      run: pointRun,
      nodes: [{ id: "unrelated", data: { label: "Unrelated PLS construct", mode: "reflective", indicators: ["I1"] } }],
    });
    expect(point.findings.map((finding) => finding.id)).toContain("inference.process_bootstrap_missing");
    expect(point.findings.map((finding) => finding.id)).not.toContain("inference.missing");
    expect(point.reportParagraphs.map((paragraph) => paragraph.section)).toEqual([
      "Model and provenance",
      "Graph-defined path analysis",
      "Inference caveat",
      "Reporting checks",
    ]);
    expect(point.reportParagraphs.map((paragraph) => paragraph.text).join(" ")).toContain("Y R-squared 0.5500");
    expect(point.reportParagraphs.map((paragraph) => paragraph.text).join(" ")).not.toContain("No endogenous R2");
    expect(point.reportParagraphs.map((paragraph) => paragraph.text).join(" ")).not.toContain("outer loading");
    expect(point.diagramAdvice).toEqual([]);

    const bootstrap = buildResultInterpretation({ run: processV2Run(true) });
    expect(bootstrap.findings.map((finding) => finding.id)).toContain("inference.process_bootstrap");
    expect(bootstrap.findings.map((finding) => finding.id)).not.toContain("inference.missing");
    expect(bootstrap.reportParagraphs.find((paragraph) => paragraph.section === "Inference caveat")?.text)
      .toContain("99 usable indexed case-bootstrap replicates");
    expect(bootstrap.reportParagraphs.map((paragraph) => paragraph.text).join(" ")).not.toContain("generic PLS");
  });

  it("keeps historical PROCESS v1 interpretation read-only and isolated from generic PLS rules", () => {
    const legacy = structuredClone(processV2Run());
    legacy.provenance!.method_version = "regression_process_v1";
    legacy.result!.method_version = "regression_process_v1";
    legacy.result!.regression!.method_version = "regression_process_v1";
    legacy.result!.regression!.process = {
      method_version: "regression_process_v1",
      model: "mediation",
      effects: [{ effect: "indirect", estimate: 0.2, lower_percentile: 0.1, upper_percentile: 0.3 }],
      simple_slopes: [],
      warnings: ["Historical PROCESS v1 archive."],
    };
    const interpretation = buildResultInterpretation({
      run: legacy,
      nodes: [{ id: "unrelated", data: { label: "Unrelated PLS construct", mode: "reflective", indicators: ["I1"] } }],
    });
    expect(interpretation.findings.map((finding) => finding.id)).toEqual(["process_v1.historical_read_only"]);
    expect(interpretation.diagramAdvice).toEqual([]);
    expect(interpretation.reportParagraphs).toHaveLength(1);
    expect(interpretation.reportParagraphs[0].section).toBe("Historical archive disclosure");
    expect(interpretation.reportParagraphs[0].text).toContain("not reinterpreted as generic PLS or current PROCESS v2 results");
    expect(interpretation.reportParagraphs[0].text).not.toContain("No endogenous R2");
  });

  it("generates row detail text using exact selected row values", () => {
    expect(rowSpecificInterpretation("Path coefficients", ["Path", "Coefficient"], ["a -> b", "-0.1250"])).toContain("-0.1250");
    expect(rowSpecificInterpretation("Inner VIF", ["Target", "Predictor", "VIF"], ["b", "a", "5.2000"])).toContain("5.2000");
  });
});
