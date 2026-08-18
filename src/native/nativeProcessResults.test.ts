import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { tablesToCsv, tablesToHtml } from "../domain/resultTables";
import { nativeRunFromCanonicalResult } from "./nativeCanonicalProject";
import { nativeRunProvenanceTable, nativeRunSettingApplicability } from "./nativeExportTables";
import {
  buildNativeResultNavigation,
  nativeProcessResultProjection,
  nativeResultTables,
} from "./nativeResults";
import { nativeProcessSemanticProbeSuffix } from "./nativeProcessResults";
import {
  processV2Envelope,
  processV2Recipe,
  processV2Run,
  processV2RunWithHighLeverageBootstrapFailure,
} from "./nativeProcessTestFixture";

describe("native PROCESS v2 result projection", () => {
  it("keeps PROCESS production copy and method documentation free of mojibake", () => {
    const sources = [
      new URL("./nativeProcess.ts", import.meta.url),
      new URL("./nativeProcessResults.ts", import.meta.url),
      new URL("./nativeExportTables.ts", import.meta.url),
      new URL("./NativeResultsSurface.tsx", import.meta.url),
      new URL("./NativeProcessSetup.tsx", import.meta.url),
      new URL("./NativeCalculationDialog.tsx", import.meta.url),
      new URL("../../docs/methods/PROCESS_V2.md", import.meta.url),
    ].map((path) => readFileSync(path, "utf8"));
    const forbidden = /[\u00c2\u00c3\ufffd]|\u00e2[\u2020\u20ac]/u;
    expect(sources.every((source) => !forbidden.test(source))).toBe(true);
  });

  it("resolves semantic probes by exact or uniquely nearest round-trip identity", () => {
    const graph = processV2Run().result!.regression!.process!.graph_v2!;
    const profile = graph.variable_profiles.find((candidate) => candidate.variable === "W")!;
    profile.raw_mean = 10_000_000_000_000_000;
    profile.raw_sample_sd = 4;
    profile.raw_min = profile.raw_mean - 8;
    profile.raw_max = profile.raw_mean + 8;
    const plus = profile.raw_mean + profile.raw_sample_sd;
    expect(nativeProcessSemanticProbeSuffix(graph, [{
      variable: "W",
      raw_value: plus + 2,
      coded_value: 4,
    }])).toBe("W=plus_1sd");

    profile.raw_sample_sd = 0.25;
    expect(nativeProcessSemanticProbeSuffix(graph, [{
      variable: "W",
      raw_value: profile.raw_mean,
      coded_value: 0,
    }])).toBeNull();
  });

  it("projects point and bootstrap runs into an explicit graph-defined result tree", () => {
    const point = processV2Run(false);
    const bootstrap = processV2Run(true);
    expect(nativeProcessResultProjection(point)).toMatchObject({
      methodVersion: "regression_process_v2",
      outcome: "Y",
      observations: 8,
      bootstrap: null,
    });
    expect(nativeProcessResultProjection(bootstrap)?.bootstrap).toMatchObject({
      method_version: "regression_process_bootstrap_v1",
      requested_replicates: 99,
      usable_replicates: 99,
    });
    const pointNavigation = buildNativeResultNavigation(point);
    const bootstrapNavigation = buildNativeResultNavigation(bootstrap);
    expect(pointNavigation.defaultItemId).toBe("process_model_summary");
    expect(bootstrapNavigation.defaultItemId).toBe("process_model_summary");
    expect(pointNavigation.groups).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: "process", title: "Graph-defined path analysis" }),
    ]));
    expect(bootstrapNavigation.groups).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: "process", title: "Graph-defined path analysis with bootstrap" }),
    ]));
  });

  it("creates table-backed plots and exportable bootstrap reasons without exposing the validation witness", () => {
    const run = processV2Run(true);
    const tables = nativeResultTables(run);
    expect(tables.map((table) => table.id)).toEqual([
      "process_model_summary",
      "process_paths",
      "process_equation_coefficients",
      "process_equation_fit",
      "process_reference_effects",
      "process_simple_slopes",
      "process_conditional_plot_points",
      "process_johnson_neyman",
      "process_johnson_neyman_curve_points",
      "process_bootstrap_summary",
      "process_bootstrap_failures",
      "process_bootstrap_inference",
      "process_bootstrap_bca",
      "process_scope",
    ]);
    expect(tables.every((table) => table.status === "validated")).toBe(true);
    expect(tables.every((table) => !table.warning?.includes("final reporting"))).toBe(true);
    expect(tables.find((table) => table.id === "process_simple_slopes")?.rows).toHaveLength(3);
    const referenceEffects = tables.find((table) => table.id === "process_reference_effects");
    expect(referenceEffects?.columns).toEqual([
      "Effect ID", "Kind", "Path", "Estimate", "Reference condition",
    ]);
    expect(referenceEffects?.rows.every((row) => row[4]
      === "Continuous moderators are evaluated at their original complete-sample raw means (coded 0); binary moderators are evaluated at 0.")).toBe(true);
    const plotPoints = tables.find((table) => table.id === "process_conditional_plot_points");
    expect(plotPoints?.rows).toHaveLength(75);
    expect(plotPoints?.rows[0]).toEqual([
      "plot:moderation:X->Y@W",
      "moderation:X->Y@W",
      "series:0:W=minus_1sd",
      "W = -1.0000 (coded -1.0000)",
      "1",
      "-2.000000",
      "-0.950000",
      "-1.150000",
      "-0.750000",
    ]);
    expect(tables.find((table) => table.id === "process_johnson_neyman")?.rows[0]).toContain("Significant Positive");
    expect(tables.find((table) => table.id === "process_scope")?.title).toBe("Scope and provenance");
    const johnsonNeymanPoints = tables.find((table) => table.id === "process_johnson_neyman_curve_points");
    expect(johnsonNeymanPoints?.rows).toHaveLength(101);
    expect(johnsonNeymanPoints?.rows[0]).toEqual([
      "moderation:X->Y@W",
      "W",
      "",
      "1",
      "-2.000000",
      "0.400000",
      "0.100000",
      "0.200000",
      "0.600000",
    ]);
    expect(tables.find((table) => table.id === "process_bootstrap_failures")?.rows[0]).toContain("No failed replicates");
    expect(JSON.stringify(tables)).not.toContain("validation_witness");
    expect(JSON.stringify(tables)).not.toContain("successful_bootstrap");
    expect(JSON.stringify(tables)).not.toContain("successful_jackknife");
    const provenance = nativeRunProvenanceTable(run);
    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: true });
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["Moderator probes", "Original-sample raw mean - SD, mean, mean + SD for continuous moderators; raw 0/1 for binary moderators"],
      ["PROCESS bootstrap stream", "process_indexed_case_stream_v1"],
    ]));
    expect(JSON.stringify(provenance)).not.toContain("validation_witness");
    const csv = tablesToCsv([...tables, provenance]);
    const html = tablesToHtml([...tables, provenance]);
    expect(csv).toContain("Graph-Defined Path Analysis with Bootstrap");
    expect(html).toContain("PROCESS bootstrap stream");
    expect(csv).toContain("Conditional outcome plot data");
    expect(html).toContain("series:0:W=minus_1sd");
    expect(csv).toContain("Reference condition");
    expect(html).toContain("Continuous moderators are evaluated at their original complete-sample raw means (coded 0); binary moderators are evaluated at 0.");
    expect(`${csv}${html}`).not.toContain("successful_bootstrap");
    expect(`${csv}${html}`).not.toContain("successful_jackknife");
  });

  it("reopens, renders, and exports a usable PROCESS bootstrap with an exact HC3 failure token", () => {
    const run = processV2RunWithHighLeverageBootstrapFailure();
    const projection = nativeProcessResultProjection(run);
    expect(projection?.bootstrap).toMatchObject({ requested_replicates: 99, usable_replicates: 98 });
    const tables = nativeResultTables(run);
    const failures = tables.find((table) => table.id === "process_bootstrap_failures");
    expect(failures?.rows).toEqual([[
      "49",
      "high_leverage_hc3_instability",
      "PROCESS equation Y has unstable HC3 leverage in this resample.",
    ]]);
    expect(tables.find((table) => table.id === "process_bootstrap_summary")?.rows)
      .toContainEqual(["Usable replicates", "98"]);
    const csv = tablesToCsv(tables);
    const html = tablesToHtml(tables);
    expect(csv).toContain("high_leverage_hc3_instability");
    expect(html).toContain("high_leverage_hc3_instability");

    const unknown = structuredClone(run);
    Object.assign(
      unknown.result!.regression!.process!.graph_v2!.bootstrap!.failed_replicates[0],
      { reason_code: "unknown_failure" },
    );
    expect(nativeProcessResultProjection(unknown)).toBeNull();
  });

  it("fails closed on tampered plot, diagnostic, tagged bootstrap, and witness structures", () => {
    const wrongPlot = structuredClone(processV2Run());
    wrongPlot.result!.regression!.process!.graph_v2!.plots[0].series[0].points.pop();
    expect(nativeProcessResultProjection(wrongPlot)).toBeNull();

    const wrongJunction = structuredClone(processV2Run());
    const available = wrongJunction.result!.regression!.process!.graph_v2!.johnson_neyman[0];
    if (available.status === "available") available.curve_points[50].moderator_raw = available.curve_points[49].moderator_raw;
    expect(nativeProcessResultProjection(wrongJunction)).toBeNull();

    const negativeVariance = structuredClone(processV2Run());
    negativeVariance.result!.regression!.process!.graph_v2!.equations[0]
      .coefficient_covariance[0][0] = -Number.EPSILON;
    expect(nativeProcessResultProjection(negativeVariance)).toBeNull();

    const wrongTest = structuredClone(processV2Run(true));
    const firstEstimand = wrongTest.result!.regression!.process!.graph_v2!.bootstrap!.estimands[0];
    firstEstimand.test = { status: "unavailable", reason_code: "zero_bootstrap_standard_error", message: "tampered" };
    expect(nativeProcessResultProjection(wrongTest)).toBeNull();

    const wrongWitness = structuredClone(processV2Run(true));
    wrongWitness.result!.regression!.process!.graph_v2!.bootstrap!.validation_witness.successful_bootstrap[0].replicate_index = 1;
    expect(nativeProcessResultProjection(wrongWitness)).toBeNull();
  });

  it("accepts only the exact scientifically derived invalid-HC3 Johnson-Neyman state", () => {
    const invalid = processV2Run();
    const graph = invalid.result!.regression!.process!.graph_v2!;
    graph.equations[0].coefficient_covariance[1][3] = 0.006;
    graph.equations[0].coefficient_covariance[3][1] = 0.006;
    graph.johnson_neyman[0] = {
      status: "unavailable",
      moderation_id: "moderation:X->Y@W",
      solved_moderator: "W",
      conditioning_values: [],
      reason_code: "invalid_hc3_covariance",
      message: "Johnson-Neyman conditional-effect variance must be finite and strictly positive across the tested moderator range.",
    };
    expect(nativeProcessResultProjection(invalid)).not.toBeNull();

    const wrongMessage = structuredClone(invalid);
    const unavailable = wrongMessage.result!.regression!.process!.graph_v2!.johnson_neyman[0];
    if (unavailable.status === "unavailable") unavailable.message = "tampered";
    expect(nativeProcessResultProjection(wrongMessage)).toBeNull();

    const falseUnavailable = processV2Run();
    falseUnavailable.result!.regression!.process!.graph_v2!.johnson_neyman[0] = {
      status: "unavailable",
      moderation_id: "moderation:X->Y@W",
      solved_moderator: "W",
      conditioning_values: [],
      reason_code: "invalid_hc3_covariance",
      message: "Johnson-Neyman conditional-effect variance must be finite and strictly positive across the tested moderator range.",
    };
    expect(nativeProcessResultProjection(falseUnavailable)).toBeNull();
  });

  it("requires the exact empty generic regression shell for PROCESS v2", () => {
    const coefficientShell = structuredClone(processV2Run());
    coefficientShell.result!.regression!.coefficients.push({
      term: "intercept",
      estimate: 0,
      standard_error: 1,
      statistic: 0,
      p_value_two_sided: 1,
      confidence_interval_lower: -1,
      confidence_interval_upper: 1,
    });
    expect(nativeProcessResultProjection(coefficientShell)).toBeNull();

    const fitShell = structuredClone(processV2Run());
    fitShell.result!.regression!.fit = { aic: 1, bic: 1 };
    expect(nativeProcessResultProjection(fitShell)).toBeNull();

    const predictionShell = structuredClone(processV2Run());
    predictionShell.result!.regression!.predictions.push({ observation: 0, fitted: 0 });
    expect(nativeProcessResultProjection(predictionShell)).toBeNull();
  });

  it("accepts the exact unavailable Johnson-Neyman arm without invented raw bounds", () => {
    const run = structuredClone(processV2Run());
    const graph = run.result!.regression!.process!.graph_v2!;
    graph.johnson_neyman[0] = {
      status: "unavailable",
      moderation_id: graph.moderations[0].moderation_id,
      solved_moderator: "W",
      conditioning_values: [],
      reason_code: "invalid_hc3_covariance",
      message: "Johnson-Neyman inference requires positive residual degrees of freedom.",
    };
    expect(nativeProcessResultProjection(run)).toBeNull();
  });
});

describe("native PROCESS v2 canonical hydration", () => {
  it("reopens point and bootstrap archives only with the exact typed recipe", () => {
    const point = nativeRunFromCanonicalResult(processV2Envelope(false), processV2Recipe(false));
    const bootstrap = nativeRunFromCanonicalResult(processV2Envelope(true), processV2Recipe(true));
    expect(point?.method).toBe("Graph-Defined Path Analysis");
    expect(bootstrap?.method).toBe("Graph-Defined Path Analysis with Bootstrap");
    expect(buildNativeResultNavigation(bootstrap).defaultItemId).toBe("process_model_summary");
  });

  it("rejects mismatched graph, predictor order, bootstrap config, and internal witness", () => {
    const envelope = processV2Envelope(true);
    const wrongGraph = structuredClone(processV2Recipe(true));
    if (wrongGraph.method_config?.kind === "regression"
      && wrongGraph.method_config.model.type === "process"
      && wrongGraph.method_config.model.relationship.model === "graph") {
      wrongGraph.method_config.model.relationship = {
        ...wrongGraph.method_config.model.relationship,
        focal_predictor: "W",
      };
    }
    expect(nativeRunFromCanonicalResult(envelope, wrongGraph)).toBeNull();

    const wrongOrder = structuredClone(processV2Recipe(true));
    if (wrongOrder.method_config?.kind === "regression") wrongOrder.method_config.predictors.reverse();
    expect(nativeRunFromCanonicalResult(envelope, wrongOrder)).toBeNull();

    const missingBootstrap = structuredClone(processV2Recipe(true));
    if (missingBootstrap.method_config?.kind === "regression") delete missingBootstrap.method_config.bootstrap;
    expect(nativeRunFromCanonicalResult(envelope, missingBootstrap)).toBeNull();

    const wrongWitness = structuredClone(envelope);
    if (wrongWitness.payload.kind !== "pls_pm_v1") throw new Error("PROCESS v2 fixture must use the PLS result envelope.");
    wrongWitness.payload.estimation.regression!.process!.graph_v2!.bootstrap!.validation_witness.estimand_ids.reverse();
    expect(nativeRunFromCanonicalResult(wrongWitness, processV2Recipe(true))).toBeNull();
  });
});
