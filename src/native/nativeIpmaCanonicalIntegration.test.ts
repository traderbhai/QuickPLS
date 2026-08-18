import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { tablesToCsv } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import { nativeRunProvenanceTable, nativeRunSettingApplicability } from "./nativeExportTables";
import NativeResultsSurface from "./NativeResultsSurface";
import { buildNativeResultNavigation, nativeIpmaPlot, resultTableForItem } from "./nativeResults";

function canonicalIpmaRun(): AnalysisRun {
  return {
    id: "ipma-run",
    modelId: "model-1",
    name: "Importance-Performance Map Analysis run",
    method: "Importance-Performance Map Analysis",
    createdAt: "2026-08-11T08:00:00.000Z",
    seed: 20_260_718,
    status: "completed",
    warnings: [],
    fingerprint: "v2:ipma-fixture",
    modelSnapshot: {
      nodes: [
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Market Capability", shortName: "MC", mode: "reflective", indicators: ["x1"] } },
        { id: "m", position: { x: 220, y: 0 }, data: { label: "Relationship Quality", shortName: "RQ", mode: "reflective", indicators: ["m1"] } },
        { id: "y", position: { x: 440, y: 0 }, data: { label: "Retention Intent", shortName: "RI", mode: "reflective", indicators: ["y1"] } },
        { id: "u", position: { x: 0, y: 200 }, data: { label: "Unrelated Driver", shortName: "UD", mode: "reflective", indicators: ["u1"] } },
        { id: "v", position: { x: 220, y: 200 }, data: { label: "Unrelated Outcome", shortName: "UO", mode: "reflective", indicators: ["v1"] } },
      ],
      edges: [
        { id: "x-m", source: "x", target: "m" },
        { id: "m-y", source: "m", target: "y" },
        { id: "u-v", source: "u", target: "v" },
      ],
    },
    result: {
      method_version: "ipma_v1",
      converged: true,
      iterations: 7,
      used_observations: 132,
      omitted_observations: 0,
      outer_estimates: [
        { construct: "x", indicator: "x1", weight: 1, loading: 0.91 },
        { construct: "m", indicator: "m1", weight: 1, loading: 0.88 },
        { construct: "y", indicator: "y1", weight: 1, loading: 0.93 },
      ],
      paths: [
        { source: "x", target: "m", coefficient: 0.42 },
        { source: "m", target: "y", coefficient: 0.61 },
      ],
      effects: [
        { source: "x", target: "y", direct: 0, indirect: 0.2562, total: 0.2562 },
        { source: "m", target: "y", direct: 0.61, indirect: 0, total: 0.61 },
      ],
      r_squared: { m: 0.1764, y: 0.3721 },
      ipma: {
        method_version: "ipma_v1",
        performance_scale: "min_max_0_100_from_standardized_scores_v1",
        targets: ["y"],
        constructs: [
          { target: "y", construct: "x", importance: 0.2562, performance: 61.25, score_mean: 0.02 },
          { target: "y", construct: "m", importance: 0.61, performance: 72.5, score_mean: 0.16 },
        ],
        indicators: [
          { target: "y", construct: "x", indicator: "x1", construct_importance: 0.2562, loading: 0.91, performance: 58.5, score_mean: -0.04 },
          { target: "y", construct: "m", indicator: "m1", construct_importance: 0.61, loading: 0.88, performance: 70.25, score_mean: 0.11 },
        ],
        warnings: [],
      },
      warnings: [],
    },
    provenance: {
      recipe_id: "recipe-1",
      dataset_fingerprint: "v2:ipma-fixture",
      method: "ipma",
      method_version: "pls_pm_v1+ipma_v1+pls_mediation_v1+pls_assessment_v7",
      engine_version: "2.45.0",
      seed: 20_260_718,
      settings: {
        method: "ipma",
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
      started_at: "2026-08-11T07:59:59.000Z",
      completed_at: "2026-08-11T08:00:00.000Z",
    },
  };
}

describe("native canonical IPMA frontend slice", () => {
  it("opens predecessor-only labeled tables and keeps deterministic settings out of UI provenance", () => {
    const run = canonicalIpmaRun();
    const navigation = buildNativeResultNavigation(run);
    const table = (id: string) => navigation.tables.find((candidate) => candidate.id === id);

    expect(navigation.defaultItemId).toBe("ipma_constructs");
    expect(navigation.groups.find((group) => group.id === "importance_performance")?.items.map((item) => item.id)).toEqual([
      "ipma_constructs",
      "ipma_indicators",
      "ipma_scope",
    ]);
    expect(table("ipma_constructs")?.rows).toEqual([
      ["Retention Intent", "Market Capability", "0.256200", "61.2500"],
      ["Retention Intent", "Relationship Quality", "0.610000", "72.5000"],
    ]);
    expect(table("ipma_indicators")?.rows).toEqual([
      ["Retention Intent", "Market Capability", "x1", "0.256200", "0.910000", "58.5000", "-0.040000"],
      ["Retention Intent", "Relationship Quality", "m1", "0.610000", "0.880000", "70.2500", "0.110000"],
    ]);
    expect(table("ipma_scope")?.title).toBe("Run details");
    expect(table("ipma_scope")?.rows).toEqual(expect.arrayContaining([
      ["Target", "Retention Intent"],
      ["Method version", "ipma_v1"],
      ["Theoretical-range correction", "Not applied"],
    ]));
    expect(navigation.tables.flatMap((candidate) => candidate.rows).flat()).not.toContain("N/A");

    const plot = nativeIpmaPlot(run);
    expect(plot).toMatchObject({
      targetId: "y",
      targetLabel: "Retention Intent",
      points: [
        { constructId: "x", constructLabel: "Market Capability", importance: 0.2562, performance: 61.25 },
        { constructId: "m", constructLabel: "Relationship Quality", importance: 0.61, performance: 72.5 },
      ],
    });

    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: false, usesConfidenceLevel: false, usesWorkers: false });
    const provenance = nativeRunProvenanceTable(run);
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["Method", "Importance-Performance Map Analysis"],
      ["Method version", "pls_pm_v1+ipma_v1+pls_mediation_v1+pls_assessment_v7"],
      ["Weighting scheme", "path"],
      ["Preprocessing", "standardized"],
    ]));
    expect(provenance.rows.map(([field]) => field)).not.toEqual(expect.arrayContaining(["Seed", "Confidence level", "Workers"]));

    const csv = tablesToCsv([...navigation.tables, provenance]);
    expect(csv).toContain("Construct importance and performance");
    expect(csv).toContain("Indicator performance");
    expect(csv).toContain("Run details");
    expect(csv).not.toMatch(/\bN\/?A\b|Confidence level|Workers/i);
  });

  it("renders the compact accessible plot from the same selected table values", () => {
    const run = canonicalIpmaRun();
    const navigation = buildNativeResultNavigation(run);
    const selectedItem = navigation.groups
      .flatMap((group) => group.items)
      .find((item) => item.id === "ipma_constructs");
    const selectedTable = resultTableForItem(navigation, "ipma_constructs");
    const markup = renderToStaticMarkup(createElement(NativeResultsSurface, {
      runs: [run],
      selectedRun: run,
      selectedRunId: run.id,
      setSelectedRunId: () => undefined,
      navigation,
      selectedItem,
      selectedTable,
      setSelectedTableId: () => undefined,
      propertiesOpen: true,
    }));

    expect(markup).toContain("Importance-performance map");
    expect(markup).toContain("Target: Retention Intent");
    expect(markup).toContain("Importance-performance map for Retention Intent");
    expect(markup).toContain("Performance (0-100)");
    expect(markup).toContain("No theoretical-range correction is applied");
    expect(markup).not.toContain("Recorded seed");
  });

  it("omits IPMA navigation instead of rendering empty or contract-drift tables", () => {
    const run = canonicalIpmaRun();
    run.result!.ipma = {
      ...run.result!.ipma!,
      performance_scale: "unknown_scale",
    };
    const navigation = buildNativeResultNavigation(run);

    expect(navigation.groups.some((group) => group.id === "importance_performance")).toBe(false);
    expect(navigation.tables.some((table) => table.id.startsWith("ipma_"))).toBe(false);
    expect(nativeIpmaPlot(run)).toBeNull();
  });
});
