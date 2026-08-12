import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun, PlsMgaAnalysis } from "../types";
import NativeExportDialog, { nativeExportScope, nativeReviewerPackHtml } from "./NativeExportDialog";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { completedGscaRun } from "./nativeGsca.testFixture";

function completedMgaRun(): AnalysisRun {
  const run = completedSamplePlsRun();
  return {
    ...run,
    result: {
      ...run.result!,
      mga: {
        method_version: "pls_mga_two_group_v1",
        group_column: "market",
        groups: [],
        comparisons: [],
        warnings: [],
      } satisfies PlsMgaAnalysis,
    },
  };
}

function completedNcaRun(): AnalysisRun {
  const run = completedSamplePlsRun();
  return {
    ...run,
    modelId: null,
    modelSnapshot: undefined,
    method: "Necessary Condition Analysis",
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
      ...run.result!,
      method_version: "nca_v2",
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
        ceilings: [],
        bottlenecks: [],
        warnings: [],
      },
    },
  };
}

describe("NativeExportDialog export scope", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("keeps ordinary completed-run diagram exports and reviewer diagrams", () => {
    const scope = nativeExportScope(completedSamplePlsRun());
    const html = "<html><body><table><tr><td>estimate</td></tr></table></body></html>";

    expect(scope).toMatchObject({
      includeModelDiagram: true,
      reviewerPackDetail: "Diagram, results, and run provenance",
    });
    expect(nativeReviewerPackHtml(html, scope.includeModelDiagram ? "<svg>model</svg>" : null))
      .toContain("<h2>Model estimates</h2><svg>model</svg>");
  });

  it("keeps a standardized model diagram in CB-SEM/CFA reviewer and SVG exports", () => {
    const run = completedCbsemRun("sem");
    expect(nativeExportScope(run)).toMatchObject({
      includeModelDiagram: true,
      reviewerPackDetail: "Diagram, results, and run provenance",
    });
  });

  it("keeps the immutable GSCA model diagram alongside its dedicated result tables", () => {
    const run = completedGscaRun();
    expect(nativeExportScope(run)).toMatchObject({
      includeModelDiagram: true,
      reviewerPackDetail: "Diagram, results, and run provenance",
    });
  });

  it("makes completed MGA exports table-only and never injects a pooled model diagram", () => {
    const scope = nativeExportScope(completedMgaRun());
    const html = "<html><body><table><tr><td>A - B</td></tr></table></body></html>";
    const reviewerPack = nativeReviewerPackHtml(
      html,
      scope.includeModelDiagram ? "<svg>pooled model</svg>" : null,
    );

    expect(scope).toEqual({
      includeModelDiagram: false,
      reviewerPackDetail: "Results tables and run provenance",
      printDetail: "Print the selected MGA results table",
    });
    expect(reviewerPack).toBe(html);
    expect(reviewerPack).not.toContain("svg");
    expect(reviewerPack).not.toContain("pooled model");
  });

  it("omits the SVG action from the completed MGA export dialog", () => {
    vi.stubGlobal("window", {});
    const markup = renderToStaticMarkup(createElement(NativeExportDialog, {
      run: completedMgaRun(),
      tables: [],
      close: () => undefined,
    }));

    expect(markup).toContain("Results tables and run provenance");
    expect(markup).toContain("Print the selected MGA results table");
    expect(markup).not.toContain("Model diagram");
    expect(markup).not.toContain("quickpls-model.svg");
  });

  it("exports standalone NCA as five table-backed formats without a fabricated model diagram", () => {
    vi.stubGlobal("window", {});
    const run = completedNcaRun();
    expect(nativeExportScope(run)).toEqual({
      includeModelDiagram: false,
      reviewerPackDetail: "Results tables and run provenance",
      printDetail: "Print the selected standalone-analysis results table",
    });

    const markup = renderToStaticMarkup(createElement(NativeExportDialog, {
      run,
      tables: [],
      close: () => undefined,
    }));
    expect(markup).toContain("CSV tables");
    expect(markup).toContain("HTML report");
    expect(markup).toContain("Reviewer pack");
    expect(markup).toContain("XLSX workbook");
    expect(markup).toContain("Print / PDF");
    expect(markup).toContain("Print the selected standalone-analysis results table");
    expect(markup).not.toContain("Model diagram");
    expect(markup).not.toContain("quickpls-model.svg");
  });

  it("keeps standalone PCA exports table-only", () => {
    const run = completedNcaRun();
    const pcaRun: AnalysisRun = {
      ...run,
      provenance: { ...run.provenance!, method: "pca", settings: { ...run.provenance!.settings, method: "pca" } },
    };
    expect(nativeExportScope(pcaRun)).toMatchObject({
      includeModelDiagram: false,
      reviewerPackDetail: "Results tables and run provenance",
    });
    const olsRun: AnalysisRun = {
      ...run,
      provenance: { ...run.provenance!, method: "regression", settings: { ...run.provenance!.settings, method: "regression" } },
    };
    expect(nativeExportScope(olsRun)).toMatchObject({
      includeModelDiagram: false,
      reviewerPackDetail: "Results tables and run provenance",
    });
  });
});
