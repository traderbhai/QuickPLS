import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings } from "../types";
import {
  buildNativeAnalysisRecipe,
  nativeAnalysisRecipeKindForCalculationMode,
} from "./nativeAnalysisRecipe";
import type { NativeCalculationMode } from "./nativeCalculationMode";
import { nativeAnalysisSettingsForWorkbenchKind } from "./nativeAnalysisCatalog";

const controller = readFileSync("src/native/NativeDesktopController.tsx", "utf8");
const app = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
const calculationDialog = readFileSync("src/native/NativeCalculationDialog.tsx", "utf8");
const analysisCatalog = readFileSync("src/native/nativeAnalysisCatalog.ts", "utf8");

describe("native controller release contracts", () => {
  it("protects dirty work before every project replacement", () => {
    expect(controller).toContain('confirmWorkspaceReplacement("creating a new project")');
    expect(controller).toContain('confirmWorkspaceReplacement(path ? "opening another project" : "opening a project")');
    expect(controller).toContain('confirmWorkspaceReplacement("opening the sample project")');
    expect(controller).toContain('buttons: { yes: "Save", no: "Don\'t Save", cancel: "Cancel" }');
    expect(app).not.toContain('loadProject({ nodes: [], edges: [], dataset: { id: crypto.randomUUID()');
  });

  it("navigates only after a project or dataset operation succeeds", () => {
    expect(controller).toContain('new CustomEvent("quickpls:navigate-surface"');
    expect(app).toContain('window.addEventListener("quickpls:navigate-surface", onNavigate)');
  });

  it("hydrates and saves through canonical project records instead of trusting workspace result copies", () => {
    expect(controller).toContain('reconcileNativeCanonicalProject(project)');
    expect(controller).toContain('nodes: canonical.nodes');
    expect(controller).toContain('edges: canonical.edges');
    expect(controller).toContain('runs: canonical.runs');
    expect(controller).toContain('const artifacts = modelId ? currentModelArtifacts(currentState, modelId) : null;');
    expect(controller).toContain('saveNativeProject(destination, workspace, model, modelPresentation)');
    expect(controller).toContain('autosaveNativeProject(projectPath, currentWorkspaceSnapshot(modelId, true), model, modelPresentation)');
    expect(controller).toContain('compactNativeWorkspaceRuns(state.runs)');
    expect(controller).not.toContain('runs: workspace?.runs');
  });

  it("hydrates and mutates the typed project explorer without dropping the outgoing model presentation", () => {
    expect(controller).toContain("projectModels: canonical.projectModels");
    expect(controller).toContain("modelPresentations: canonical.modelPresentations");
    expect(controller).toContain("savedReports: canonical.savedReports");
    expect(controller).toContain("mutateNativeProjectExplorer({");
    expect(controller).toContain("currentModel: artifacts?.model ?? null");
    expect(controller).toContain("currentPresentation: artifacts?.presentation ?? null");
    expect(controller).toContain("path: state.projectPath");
    expect(controller).toContain("establishBaseline: false");
    expect(controller).toContain("hasUnsavedNativeProjectChanges(true, cleanSignature, nextSignature)");
    expect(controller).toContain('"quickpls:mutate-project-explorer"');
    expect(controller).toMatch(/mutateProjectExplorer\(detail\.mutation\)[\s\S]*detail\.resolve\(\)[\s\S]*detail\.reject/);
  });

  it("keeps model deletion in Project Explorer and selects a deterministic remaining target", () => {
    const deletion = controller.slice(
      controller.indexOf('mutation.kind === "delete_model"'),
      controller.indexOf('mutation.kind === "rename_model"'),
    );

    expect(deletion).toContain('setExplorerSelection({ kind: "model", modelId: nextState.activeModelId })');
    expect(deletion).toContain('setExplorerSelection({ kind: "models" })');
    expect(deletion).not.toContain('quickpls:navigate-surface');
  });

  it("forwards validated data-kind, sample-size, and missing-marker import options", () => {
    expect(app).toContain('case "project.import-data": openDialog("import-data")');
    expect(app).toContain('<NativeDataImportDialog close={closeDialog} importData={beginDataImport} />');
    expect(controller).toContain('normalizeNativeDataImportRequest((event as CustomEvent<unknown>).detail)');
    expect(controller).toContain('importNativeDataset(request.dataKind, request.sampleSize, request.missingMarkers)');
    expect(controller).not.toContain("const imported = await importNativeDataset();");
  });

  it("attempts to dismiss failed native jobs before exposing retry", () => {
    expect(controller).toMatch(/catch \(error\)[\s\S]*runMonitor\.activeJobId[\s\S]*dismissNativePlsJob\(jobId\)/);
  });

  it("does not dismiss a completed job after its result has been consumed", () => {
    const resultConsumption = controller.slice(
      controller.indexOf("const envelope = await getNativePlsJobResult(job.id);"),
      controller.indexOf("const { estimation: result, assessment } = envelope.payload;"),
    );

    expect(resultConsumption).not.toContain("dismissNativePlsJob(job.id)");
    expect(resultConsumption).toMatch(/getNativePlsJobResult\(job\.id\);[\s\S]*setActiveJob\(null\);[\s\S]*transitionRunMonitor\(\{[\s\S]*activeJobId: null,[\s\S]*\}\);/);
  });

  it("uses the consumed native result id for both the saved run and completion selection", () => {
    const completion = controller.slice(
      controller.indexOf("const envelope = await getNativePlsJobResult(job.id);"),
      controller.indexOf('pushToast({ tone: "success", title: "Calculation completed"'),
    );

    expect(completion).toMatch(/addRun\(\{[\s\S]*id: envelope\.id,[\s\S]*\}\);/);
    expect(completion).toMatch(/addRun\(\{[\s\S]*modelId,[\s\S]*\}\);/);
    expect(completion).toMatch(/transitionRunMonitor\(\{[\s\S]*status: "completed",[\s\S]*lastRunId: envelope\.id,/);
    expect(app).toContain("resolveSelectedCompletedRun(completedRuns, selectedResultRunId)");
    expect(app).toContain("setSelectedRunId={setSelectedResultRun}");
    expect(app).not.toContain("useState(selectedResultRunId");
  });

  it("persists and exposes only result-backed completed jobs", () => {
    const runStart = controller.indexOf("const runAnalysis = async");
    const resultFetch = controller.indexOf("const envelope = await getNativePlsJobResult(job.id);", runStart);
    const compatibilityGate = controller.indexOf(
      'if (!envelope || envelope.payload.kind === "legacy") throw new Error("The completed job did not return a compatible result.");',
      resultFetch,
    );
    const completedRunAppend = controller.indexOf("addRun({", compatibilityGate);
    const runEnd = controller.indexOf("const executeRun = async", completedRunAppend);

    expect(runStart).toBeGreaterThan(-1);
    expect(resultFetch).toBeGreaterThan(runStart);
    expect(compatibilityGate).toBeGreaterThan(resultFetch);
    expect(completedRunAppend).toBeGreaterThan(compatibilityGate);
    expect(controller.slice(runStart, resultFetch)).not.toContain("addRun({");
    expect(controller.slice(completedRunAppend, runEnd)).toContain('status: "completed"');
    expect(controller.slice(completedRunAppend, runEnd).match(/addRun\(\{/g)).toHaveLength(1);
    expect(app).toContain("const completedRuns = useMemo(() => completedResultRuns(runs), [runs]);");
  });

  it("stores completed-run logs and exposes them on demand", () => {
    expect(controller).toContain("...useWorkspace.getState().runMonitor.logs");
    expect(app).toContain("Calculation log ({run.logs.length})");
  });

  it("uses the typed builder as the sole native recipe construction path", () => {
    const construction = controller.slice(
      controller.indexOf("const recipeId = crypto.randomUUID();"),
      controller.indexOf("let job = await startNativePlsJob(recipe);"),
    );

    expect(controller).toContain('from "./nativeAnalysisRecipe"');
    expect(construction).toContain("buildNativeAnalysisRecipe({");
    expect(construction).toContain("kind: request.kind");
    expect(construction).toContain("settings: submittedSettings");
    expect(construction).toContain("projectName: modelName");
    expect(construction).toContain("createdAt: startedAt");
    expect(construction.match(/crypto\.randomUUID\(\)/g)).toHaveLength(2);
    expect(construction).not.toContain("new Date");
    expect(construction).not.toContain("schema_version");
    expect(construction).not.toContain("bootstrap_samples");
    expect(construction).not.toContain("permutation_samples");
  });

  it("builds exact Algorithm, Bootstrapping, Permutation, and Prediction job payloads", () => {
    const settings: AnalysisUiSettings = {
      method: "pls_pm",
      weightingScheme: "path",
      tolerance: 1e-7,
      maxIterations: 3_000,
      preprocessing: "standardized",
      bootstrapSamples: 5_000,
      studentizedInnerSamples: 0,
      permutationSamples: 999,
      seed: 20_260_718,
      workers: 4,
      confidenceLevel: 0.95,
    };
    const expected: Array<[NativeCalculationMode, "pls_pm" | "predict", [number, number, number], string]> = [
      ["pls", "pls_pm", [0, 0, 0], "validated_v1_0_supported_pls_scope"],
      ["bootstrap", "pls_pm", [5_000, 0, 0], "validated_v1_0_supported_pls_scope"],
      ["permutation", "pls_pm", [0, 0, 999], "validated_v1_0_freedman_lane_path_randomization_scope"],
      ["predict", "predict", [0, 0, 0], "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope"],
    ];

    for (const [mode, method, samples, status] of expected) {
      const recipe = buildNativeAnalysisRecipe({
        kind: nativeAnalysisRecipeKindForCalculationMode(mode),
        recipeId: "11111111-1111-4111-8111-111111111111",
        modelId: "22222222-2222-4222-8222-222222222222",
        createdAt: "2026-08-10T08:00:00.000Z",
        datasetFingerprint: "sha256-fixture",
        projectName: "Fixture",
        nodes: [
          { id: "x", position: { x: 0, y: 0 }, data: { label: "X", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
          { id: "y", position: { x: 300, y: 0 }, data: { label: "Y", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
        ],
        edges: [{ id: "x-y", source: "x", target: "y" }],
        settings,
      });
      expect(recipe.settings.method, mode).toBe(method);
      expect([
        recipe.settings.bootstrap_samples,
        recipe.settings.studentized_inner_samples,
        recipe.settings.permutation_samples,
      ], mode).toEqual(samples);
      expect(recipe.metadata.status, mode).toBe(status);
      expect(recipe.schema_version, mode).toBe(3);
      expect(recipe.method_config.kind, mode).toBe(mode === "pls" ? "pls_algorithm" : mode === "bootstrap" ? "pls_bootstrap" : mode === "permutation" ? "pls_permutation" : "predict");
    }
  });

  it("submits an immutable catalog request and stores native inference output", () => {
    expect(app).toContain("createNativeCalculationRequest(calculationKind, calculationSettings, logisticProfile)");
    expect(controller).toContain("parseNativeCalculationRequest");
    expect(controller).toContain("nativeAnalysisRecipeDescriptor(request.kind).label");
    expect(calculationDialog).toContain('role="listbox"');
    expect(analysisCatalog).toContain("NATIVE_PREDICTION_SCOPE_DESCRIPTION");
    expect(controller).toContain('const permutation = envelope.payload.kind === "pls_pm_v3"');
    expect(controller).toMatch(/addRun\(\{[\s\S]*permutation,[\s\S]*\}\);/);
    expect(controller).toContain("provenance: envelope.provenance");
  });

  it("routes a normalized single-target IPMA request through the typed controller recipe path", () => {
    const settings = nativeAnalysisSettingsForWorkbenchKind({
      method: "pls_pm",
      weightingScheme: "factor",
      preprocessing: "mean_centered",
      bootstrapSamples: 999,
      studentizedInnerSamples: 99,
      permutationSamples: 999,
      seed: 20_260_718,
      workers: 4,
      confidenceLevel: 0.95,
      caseWeightColumn: "ignored",
      ipmaTargets: "y",
    }, "ipma");
    const recipe = buildNativeAnalysisRecipe({
      kind: "ipma",
      recipeId: "11111111-1111-4111-8111-111111111111",
      modelId: "22222222-2222-4222-8222-222222222222",
      createdAt: "2026-08-11T08:00:00.000Z",
      datasetFingerprint: "sha256-ipma-fixture",
      projectName: "IPMA fixture",
      nodes: [
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1"] } },
        { id: "y", position: { x: 300, y: 0 }, data: { label: "Retention", shortName: "RET", mode: "reflective", indicators: ["y1"] } },
      ],
      edges: [{ id: "x-y", source: "x", target: "y" }],
      settings,
    });

    expect(controller).toContain("const submittedSettings = request.settings");
    expect(controller).toContain("nativePlsReadiness({ dataset, nodes, edges, settings: submittedSettings");
    expect(recipe.settings).toMatchObject({
      method: "ipma",
      weighting_scheme: "path",
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      bootstrap_samples: 0,
      studentized_inner_samples: 0,
      permutation_samples: 0,
      workers: 1,
      case_weight_column: null,
    });
    expect(recipe.metadata).toEqual({ status: "validated_v1_2_1_ipma_bounded_scope" });
    expect(recipe.method_config).toEqual({ kind: "ipma", targets: ["y"] });
  });

  it("runs standalone NCA with an empty wire model without creating an editable project model", () => {
    const construction = controller.slice(
      controller.indexOf("const recipeId = crypto.randomUUID();"),
      controller.indexOf("let job = await startNativePlsJob(recipe);"),
    );
    const completion = controller.slice(
      controller.indexOf("const envelope = await getNativePlsJobResult(job.id);"),
      controller.indexOf('pushToast({ tone: "success", title: "Calculation completed"'),
    );

    expect(controller).toContain("const standalone = isStandaloneNativeAnalysis(request.kind)");
    expect(construction).toContain("const transientModelId = crypto.randomUUID()");
    expect(construction).toContain("const modelId = standalone ? transientModelId : (currentState.activeModelId ?? transientModelId)");
    expect(construction).not.toContain("standalone-${recipeId}");
    expect(construction).toContain("if (!standalone)");
    expect(construction).toContain("nodes: standalone ? [] : nodes");
    expect(construction).toContain("edges: standalone ? [] : edges");
    expect(completion).toContain("modelId: standalone ? null : modelId");
    expect(completion).toContain("...(modelSnapshot ? { modelSnapshot } : {})");
  });

  it("keeps calculation edits local until Start and lets Close discard the draft", () => {
    expect(app).toContain("const [calculationDraft, setCalculationDraft]");
    expect(app).toContain("setCalculationDraft(nativeAnalysisSettingsForWorkbenchKind(analysisSettings, preferredKind))");
    expect(app).toContain("setSettings={(patch) => setCalculationDraft");
    expect(app).toMatch(/const startCalculation = \(logisticProfile\?: NativeLogisticProfile\) => \{[\s\S]*setAnalysisSettings\(calculationSettings\);[\s\S]*createNativeCalculationRequest\(calculationKind, calculationSettings, logisticProfile\)/);
    expect(calculationDialog).toContain('<button type="button" onClick={close}>Close</button>');
  });

  it("carries a full-data logistic proof through dispatch and revalidates it before job creation", () => {
    expect(calculationDialog).toContain("start(verifiedLogisticProfile)");
    expect(app).toContain("createNativeCalculationRequest(calculationKind, calculationSettings, logisticProfile)");
    expect(controller).toContain("nativeLogisticReadiness(dataset, submittedSettings, request.logisticProfile ?? null)");
    expect(controller).toContain("!request.logisticProfile");
    expect(controller.indexOf("const logisticDispatchError")).toBeLessThan(controller.indexOf("const recipeId = crypto.randomUUID();"));
  });
});
