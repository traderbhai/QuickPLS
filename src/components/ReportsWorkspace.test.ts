import { describe, expect, it } from "vitest";
import { runExportTables } from "../domain/resultTables";
import { defaultDiagramLayout } from "../domain/diagramGraph";
import { processV2Run } from "../native/nativeProcessTestFixture";
import { completedStructuralPathRandomizationRun } from "../native/nativeStructuralPathRandomization.testFixture";
import {
  PROCESS_GENERIC_DIAGRAM_UNAVAILABLE,
  reportableRuns,
  reportDiagramSvgForRun,
  reportModelForRun,
  reportMethodId,
  reportScopeStatus,
  reportTableStatusLabel,
} from "./ReportsWorkspace";

describe("ReportsWorkspace run truth", () => {
  it("translates internal table states into customer-facing result labels", () => {
    expect(reportTableStatusLabel("validated")).toBe("Supported result");
    expect(reportTableStatusLabel("experimental")).toBe("Experimental result");
  });

  it("falls back from a newer failed run to a completed result-backed PROCESS run", () => {
    const process = processV2Run(true);
    const failed = {
      ...process,
      id: "newer-failed",
      createdAt: "2026-08-12T13:00:00.000Z",
      status: "failed" as const,
      result: undefined,
    };
    const available = reportableRuns([failed, process]);
    expect(available.map((run) => run.id)).toEqual([process.id]);
    const tables = runExportTables(available[0]);
    expect(reportScopeStatus(tables)).toBe("validated");
    expect(tables.map((table) => table.id)).toContain("process_reference_effects");
  });

  it("binds method scope to archived run provenance rather than current setup", () => {
    const process = processV2Run();
    expect(reportMethodId(process, "pls_pm")).toBe("regression");
    expect(reportMethodId(undefined, "pls_pm")).toBe("pls_pm");
  });

  it("binds a strict Structural Path Randomization run to its scoped Standard identity", () => {
    const randomization = completedStructuralPathRandomizationRun();
    const archivedNode: Parameters<typeof reportDiagramSvgForRun>[1][number] = {
      id: "archived-randomization-construct",
      type: "construct",
      position: { x: 20, y: 30 },
      data: { label: "Archived randomization construct", shortName: "ARC", mode: "reflective", indicators: [] },
    };
    const unrelatedLiveNode: Parameters<typeof reportDiagramSvgForRun>[1][number] = {
      ...archivedNode,
      id: "unrelated-live-construct",
      data: { ...archivedNode.data, label: "Unrelated live construct" },
    };
    randomization.modelSnapshot = { nodes: [archivedNode], edges: [], diagramLayout: defaultDiagramLayout([archivedNode], []) };
    expect(reportMethodId(randomization, "pls_pm")).toBe("permutation");
    expect(reportScopeStatus(runExportTables(randomization))).toBe("validated");
    const svg = reportDiagramSvgForRun(randomization, [unrelatedLiveNode], [], { showValidationWatermark: false }, randomization.modelSnapshot.diagramLayout);
    expect(svg).toContain("Archived randomization construct");
    expect(svg).not.toContain("Unrelated live construct");
    expect(svg).toContain("Supported for the documented bounded scope: single-model Freedman-Lane randomization");
    expect(svg).not.toContain("Validated for documented QuickPLS supported scope");
    expect(reportModelForRun(randomization, [unrelatedLiveNode], [], randomization.modelSnapshot.diagramLayout).nodes)
      .toEqual([archivedNode]);
  });

  it("does not turn an unrelated live PLS canvas into a PROCESS report SVG", () => {
    const process = processV2Run();
    const unrelatedLiveCanvas: Parameters<typeof reportDiagramSvgForRun>[1] = [{
      id: "unrelated-live-construct",
      type: "construct",
      position: { x: 10, y: 20 },
      data: { label: "Unrelated live construct", shortName: "ULC", mode: "reflective", indicators: [] },
    }];
    const svg = reportDiagramSvgForRun(
      process,
      unrelatedLiveCanvas,
      [],
      { showValidationWatermark: true },
      undefined,
    );
    expect(svg).toBe("");
    expect(svg).not.toContain("Validated for documented QuickPLS supported scope");
    expect(PROCESS_GENERIC_DIAGRAM_UNAVAILABLE).toContain("no archived method-specific diagram");

    const pls = structuredClone(process);
    pls.method = "pls_pm";
    pls.provenance = { ...process.provenance!, method: "pls_pm", method_version: "pls_pm_v2" };
    pls.result!.regression = undefined;
    const plsSvg = reportDiagramSvgForRun(pls, [], [], { showValidationWatermark: false }, undefined);
    expect(plsSvg).toContain("<svg");
    expect(plsSvg).toContain("publication diagram");
  });

  it("routes historical PROCESS v1 through read-only tables and suppresses generic SVG", () => {
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
    const tables = runExportTables(legacy);
    expect(tables.map((table) => table.id)).toEqual([
      "run_provenance",
      "legacy_process_effects",
      "legacy_process_scope",
    ]);
    expect(tables.every((table) => table.status === "experimental")).toBe(true);
    expect(tables.find((table) => table.id === "legacy_process_scope")?.rows)
      .toContainEqual(["Status", "Readable historical output only; create a graph-defined PROCESS v2 analysis for a current interpretation."]);
    const unrelatedLegacyCanvas: Parameters<typeof reportDiagramSvgForRun>[1] = [{
      id: "unrelated-legacy-live-construct",
      type: "construct",
      position: { x: 10, y: 20 },
      data: { label: "Unrelated legacy live construct", shortName: "ULLC", mode: "reflective", indicators: [] },
    }];
    expect(reportDiagramSvgForRun(legacy, unrelatedLegacyCanvas, [], {}, undefined)).toBe("");
  });
});
