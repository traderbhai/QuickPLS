import { readFileSync } from "node:fs";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import { useWorkspace } from "../store";
import type { AnalysisRun } from "../types";
import {
  aboutVisibleAnalysisLabelsV2,
  completedRunNavigationTarget,
  Launcher,
  NATIVE_BUNDLED_SAMPLE_PROJECTS,
  openNativeSampleProject,
} from "./NativeDesktopApp";
import {
  NATIVE_ANALYSIS_CATALOG,
  NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1,
} from "./nativeAnalysisCatalog";
import { buildNativeResultNavigation, completedResultRuns, nativeResultTables } from "./nativeResults";
import { completedStructuralPathRandomizationRun } from "./nativeStructuralPathRandomization.testFixture";

describe("native desktop result contracts", () => {
  it("navigates once for each unique completed run even without an observed intermediate status", () => {
    expect(completedRunNavigationTarget("completed", "run-a", null)).toBe("run-a");
    expect(completedRunNavigationTarget("completed", "run-a", "run-a")).toBeNull();
    expect(completedRunNavigationTarget("completed", "run-b", "run-a")).toBe("run-b");
    expect(completedRunNavigationTarget("running", "run-b", "run-a")).toBeNull();
    expect(completedRunNavigationTarget("completed", null, "run-a")).toBeNull();
  });

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
      status: "validated",
      columns: ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
    });
    expect(table?.rows).toHaveLength(5);
    expect(table?.rows[0]).toEqual(["competence -> satisfaction", "0.403000", "9", "999", "0.01"]);
    expect(navigation.groups.find((group) => group.id === "inference")?.items.map((item) => item.id)).toContain("permutation");
  });
});

describe("native desktop multi-model shell contracts", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("mounts exactly the three genuine sample choices in the production launcher", () => {
    const markup = renderToStaticMarkup(createElement(Launcher, {
      projectName: "No project open",
      projectPath: null,
      datasetName: "No dataset",
      runs: [],
      recentProjects: [],
      onNavigate: vi.fn(),
      onOpenRecent: vi.fn(),
      onOpenSample: vi.fn(),
    }));

    expect(NATIVE_BUNDLED_SAMPLE_PROJECTS.map((sample) => sample.id)).toEqual([
      "corporate_reputation",
      "simple_pls",
      "mediation",
    ]);
    expect(markup.match(/data-sample-id=/g)).toHaveLength(3);
    for (const sample of NATIVE_BUNDLED_SAMPLE_PROJECTS) {
      expect(markup).toContain(`data-sample-id="${sample.id}"`);
      expect(markup).toContain(sample.label);
    }
    expect(markup).not.toContain("PLSpredict");
    expect(markup).not.toContain("CB-SEM CFA");
  });

  it("dispatches the exact selected sample identity while the File menu keeps its corporate default", () => {
    const dispatchEvent = vi.fn();
    vi.stubGlobal("window", { dispatchEvent });

    openNativeSampleProject("mediation");

    expect(dispatchEvent).toHaveBeenCalledTimes(1);
    const event = dispatchEvent.mock.calls[0][0] as CustomEvent;
    expect(event.type).toBe("quickpls:open-demo-project");
    expect(event.detail).toEqual({ sampleId: "mediation" });
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('case "project.open-demo": commandEvent("open-demo-project"); return;');
    expect(source).toContain("onOpenSample={openNativeSampleProject}");
  });

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

  it("passes the persisted Labs preference and application-session warning ledger into Calculate", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");

    expect(source).toContain("experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled}");
    expect(source).toContain("experimentalWarningShownSessionKeys={experimentalWarningShownSessionKeys}");
    expect(source).toContain("onExperimentalWarningShown={recordExperimentalWarningShown}");
    expect(source).toContain("setExperimentalWarningShownSessionKeys((current) => {");
  });

  it("verifies the installed Rust registry before enabling native calculations", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");

    expect(source).toContain("getNativeCapabilityRegistryV2()");
    expect(source).toContain('setNativeRegistryVerification("verified")');
    expect(source).toContain('setNativeRegistryVerification("failed")');
    expect(source).toContain("registryUnavailableReason={nativeRegistryVerification");
  });

  it("keeps About synchronized with the established Calculate surface and scoped post-hoc add-on", () => {
    const settings = useWorkspace.getState().analysisSettings;

    const standardKinds = ["pls_algorithm", "plsc", "wpls", "gsca", "cca", "cta_pls", "ipma", "cbsem", "pls_bootstrap", "plsc_bootstrap", "pls_permutation", "pls_posthoc_technical_minimum_sample_size", "pls_sample_size_power", "mga", "predict", "nca", "pca", "regression"];
    expect(aboutVisibleAnalysisLabelsV2(settings, false)).toEqual(standardKinds.map((kind) => (
      NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === kind)!.label
    )));
    const labs = aboutVisibleAnalysisLabelsV2(settings, true);
    const expectedKinds = NATIVE_ANALYSIS_CATALOG
      .map((item) => item.kind)
      .filter((kind) => standardKinds.includes(kind)
        || NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1.includes(kind as typeof NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1[number]));
    const expected = expectedKinds.map((kind) => (
      NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === kind)!.label
    ));
    expect(labs).toEqual(expected);
    expect(labs).toHaveLength(18);
  });

  it("binds Method Details to the selected immutable run only on Results", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");

    expect(source).toContain('run={dialog === "trust" && surface === "results" ? selectedRun : null}');
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
    expect(source).toContain('<span className="nd-model-document-title" title={modelName}');
    expect(source).toContain('surface === "model" ? modelName : "Results"');
  });

  it("keeps open-project facts visible in the Project status bar", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain('surface !== "launcher" || projectOpen');
    expect(source).toContain('surface === "launcher" && projectOpen ? "Project"');
  });

  it("routes strict group, higher-order, moderation, and navigator indicator actions through authority intents", () => {
    const source = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
    expect(source).toContain("commitStandardSemModelV4Intent");
    for (const kind of ["set_group", "add_higher_order", "add_interaction", "assign_indicators", "add_construct"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain('title: `${label} blocked`');
    expect(source).toContain('title: `${label} stale`');
    expect(source).toContain('title: `${label} rejected`');
  });
});
