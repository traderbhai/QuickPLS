import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import { buildNativeResultNavigation, completedResultRuns, nativeResultTables } from "./nativeResults";
import { completedStructuralPathRandomizationRun } from "./nativeStructuralPathRandomization.testFixture";

describe("native desktop result contracts", () => {
  it("shows only completed runs with a real result payload", () => {
    const complete = completedSamplePlsRun();
    const failed: AnalysisRun = { ...complete, id: "failed", status: "failed" };
    const empty: AnalysisRun = { ...complete, id: "empty", result: undefined };

    expect(completedResultRuns([failed, empty, complete])).toEqual([complete]);
  });

  it("derives PLS result navigation only from available outputs", () => {
    const tables = buildNativeResultNavigation(completedSamplePlsRun()).tables;

    expect(tables.map((table) => table.id)).toContain("direct_effects");
    expect(tables.map((table) => table.id)).not.toContain("path_coefficients");
    expect(tables.map((table) => table.id)).toContain("outer_loadings");
    expect(tables.every((table) => table.rows.length > 0)).toBe(true);
    expect(tables.flatMap((table) => table.rows).flat()).not.toContain("No completed run");
  });

  it("returns no placeholder tables when a run has no result", () => {
    const run = { ...completedSamplePlsRun(), result: undefined };
    expect(nativeResultTables(run)).toEqual([]);
  });

  it("adds bootstrap navigation only when bootstrap output exists", () => {
    const base = completedSamplePlsRun();
    expect(nativeResultTables(base).some((table) => table.id === "bootstrap_percentile")).toBe(Boolean(base.bootstrap?.percentile.parameters.length));
  });

  it("opens truthful permutation output in the inference result group", () => {
    const run = completedStructuralPathRandomizationRun();
    const navigation = buildNativeResultNavigation(run);
    const table = navigation.tables.find((item) => item.id === "permutation");

    expect(table).toMatchObject({
      title: "Structural path randomization",
      status: "experimental",
      columns: ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
    });
    expect(table?.rows).toHaveLength(5);
    expect(table?.rows[0]).toEqual(["competence -> satisfaction", "0.403000", "9", "999", "0.01"]);
    expect(navigation.groups.find((group) => group.id === "inference")?.items.map((item) => item.id)).toContain("permutation");
  });
});

describe("native desktop multi-model shell contracts", () => {
  it("reapplies an asynchronously hydrated result default without requiring a run-id change", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('setSelectedTableId(resultNavigation.defaultItemId ?? "")');
    expect(source).toContain("[resultNavigation.defaultItemId, resultNavigation.runId]");
  });

  it("routes Data Analyze through the shared catalog with standalone NCA selected", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('const preferredKind = surface === "data" ? "nca"');
    expect(source).toContain("setCalculationDraft(nativeAnalysisSettingsForWorkbenchKind(analysisSettings, preferredKind))");
    expect(source).toContain("setCalculationKind(preferredKind)");
    expect(source).toContain("loadNcaFixture");
    expect(source).toContain("loadPcaFixture");
    expect(source).toContain("nativePcaResultProjection(run)");
    expect(source).toContain("nativePcaComponentRuleLabel(pca.componentRule)");
    expect(source).toContain("Correlation matrix of standardized variables");
    expect(source).toContain("loadHocFixture");
    expect(source).toContain('projectModels: []');
    expect(source).toContain('activeModelId: null');
    expect(source).toContain('navigate("data")');
  });

  it("exposes a query-gated resident PROCESS v2 setup fixture without runs or models", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    const start = source.lastIndexOf("loadProcessV2Fixture: () => {");
    const end = source.indexOf("loadHocFixture: () => {", start);
    const fixtureLoader = source.slice(start, end);

    expect(start).toBeGreaterThan(0);
    expect(end).toBeGreaterThan(start);
    expect(fixtureLoader).toContain('const columns = ["X", "M1", "M2", "M3", "M4", "W", "B", "C", "Y"]');
    expect(fixtureLoader).toContain('id: "native-process-v2-smoke"');
    expect(fixtureLoader).toContain('fingerprint: "sha256:native-process-v2-smoke-v1"');
    expect(fixtureLoader).toContain("projectModels: []");
    expect(fixtureLoader).toContain("runs: []");
    expect(fixtureLoader).toContain('navigate("data")');
    expect(fixtureLoader).toContain("return { variables: 9, models: 0 }");
  });

  it("routes Data grouping through the typed command and focus-trapped dialog host", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('case "data.configure-groups"');
    expect(source).toContain('openDialog("group-setup")');
    expect(source).toContain('dialog === "group-setup"');
    expect(source).toContain("analysisColumns={calculationAnalysisColumns}");
    expect(source).toContain("setAnalysisSettings(patch)");
    expect(source).toContain('role="dialog" aria-modal="true"');
  });

  it("opens the selected result's surviving source model instead of the arbitrary active model", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('const modelId = surface === "results" ? selectedRun?.modelId : activeModelId');
    expect(source).toContain("projectModels.some((model) => model.id === modelId)");
    expect(source).toContain("const resultModelId = selectedRun?.modelId");
    expect(source).toContain('commandEvent("open-explorer-model", { modelId: resultModelId })');
  });

  it("shows the active editable model name in the command context and document tab", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain("modelName={activeEditableModelName}");
    expect(source).toContain('<span title={modelName}>{modelName}</span>');
    expect(source).toContain('surface === "model" ? modelName : "Results"');
  });

  it("keeps open-project facts visible in the Project status bar", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('surface !== "launcher" || projectOpen');
    expect(source).toContain('surface === "launcher" && projectOpen ? "Project"');
  });
});
