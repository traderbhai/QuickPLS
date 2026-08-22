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

  it("restores exact schema-6 CB-SEM results after an ordinary project open", () => {
    const ordinaryOpen = controller.slice(
      controller.indexOf("const loaded = loadNativeSnapshot(await openNativeProjectAt(selectedPath));"),
      controller.indexOf("const openDemoProject = async"),
    );

    expect(ordinaryOpen).toContain("if (!loaded) return;");
    expect(ordinaryOpen).toContain("clearPresentedCanonicalResult();");
    expect(ordinaryOpen).toContain('if (inspected.status !== "ok") return;');
    expect(ordinaryOpen).toContain("readStoredInternalLabsRecipeV4CbsemResultsV1({");
    expect(ordinaryOpen).toContain("archivePath: inspected.value.archivePath");
    expect(ordinaryOpen).toContain("sourceSha256: inspected.value.archiveSha256");
    expect(ordinaryOpen).toContain("projectId: inspected.value.project.project_id");
    expect(ordinaryOpen).toContain("selectLatestStoredExactCaseBootstrapEntryV1(stored.entries)");
    expect(ordinaryOpen).toContain('new CustomEvent("quickpls:general-sem-canonical-result"');
    expect(ordinaryOpen.indexOf("loadNativeSnapshot")).toBeLessThan(ordinaryOpen.indexOf("readStoredInternalLabsRecipeV4CbsemResultsV1"));
  });

  it("clears a prior canonical result before any accepted replacement can restore another", () => {
    expect(controller).toContain('detail: { document: null, navigate: false }');
    expect(app).toContain("if (detail?.document === null)");
    expect(app).toMatch(/document === null[\s\S]*setGeneralSemCanonicalResult\(null\);[\s\S]*setGeneralSemResultSelected\(false\);/);

    const ordinaryOpen = controller.slice(
      controller.indexOf("const loaded = loadNativeSnapshot(await openNativeProjectAt(selectedPath));"),
      controller.indexOf("const openDemoProject = async"),
    );
    expect(ordinaryOpen.indexOf("clearPresentedCanonicalResult();"))
      .toBeLessThan(ordinaryOpen.indexOf("readStoredInternalLabsRecipeV4CbsemResultsV1"));
  });

  it("blocks strict Standard authority before every legacy persistence or calculation boundary", () => {
    const saveStart = controller.indexOf("const saveProject = async");
    const saveGate = controller.indexOf('nativeLegacyProjectOperationBlocker(currentState, "schema5_save")', saveStart);
    const saveNative = controller.indexOf("saveNativeProject(destination", saveStart);
    const calculationStart = controller.indexOf("const runAnalysis = async");
    const calculationGate = controller.indexOf('nativeLegacyProjectOperationBlocker(useWorkspace.getState(), "calculation")', calculationStart);
    const calculationNative = controller.indexOf("startNativePlsJob(recipe)", calculationStart);
    const autosaveStart = controller.indexOf("const scheduledSignature = projectSignature");
    const autosaveGate = controller.indexOf('nativeLegacyProjectOperationBlocker(state, "schema5_autosave")', autosaveStart);
    const autosaveNative = controller.indexOf("autosaveNativeProject(projectPath", autosaveStart);
    const explorerStart = controller.indexOf("const mutateProjectExplorer = async");
    const explorerGate = controller.indexOf('standardSemModelV4OperationBlocker("legacy_graph_serialization")', explorerStart);
    const explorerNative = controller.indexOf("mutateNativeProjectExplorer({", explorerStart);

    expect(saveGate).toBeGreaterThan(saveStart);
    expect(saveGate).toBeLessThan(saveNative);
    expect(calculationGate).toBeGreaterThan(calculationStart);
    expect(calculationGate).toBeLessThan(calculationNative);
    expect(autosaveGate).toBeGreaterThan(autosaveStart);
    expect(autosaveGate).toBeLessThan(autosaveNative);
    expect(explorerGate).toBeGreaterThan(explorerStart);
    expect(explorerGate).toBeLessThan(explorerNative);
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
    expect(controller).toContain("hasUnsavedNativeProjectChanges(true, cleanSignature, nextSignature, {");
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
    expect(app).toContain('case "project.import-data":');
    expect(app).toContain('if (!rejectLockedDataMutation("Import data")) openDialog("import-data")');
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

  it("fails current structural path randomization closed before adding the completed run", () => {
    const plsCompletionStart = controller.indexOf("const { estimation: result, assessment } = envelope.payload;");
    const completedRunAppend = controller.indexOf("addRun(completedRun);", plsCompletionStart);
    const completion = controller.slice(
      plsCompletionStart,
      controller.indexOf("transitionRunMonitor({", completedRunAppend),
    );

    expect(plsCompletionStart).toBeGreaterThan(-1);
    expect(completedRunAppend).toBeGreaterThan(plsCompletionStart);
    expect(completion).toContain("const completedRun: AnalysisRun = {");
    expect(completion).toContain("isStructuralPathRandomizationIdentityPresent(recipe, envelope)");
    expect(completion).toContain("nativeStructuralPathRandomizationProjection(completedRun)");
    expect(completion).toContain("nativeStructuralPathRandomizationRecipeMatches(recipe, envelope, projection)");
    expect(completion).toContain("failed its current scientific contract");
    expect(completion.indexOf("nativeStructuralPathRandomizationProjection(completedRun)"))
      .toBeLessThan(completion.indexOf("addRun(completedRun);"));
  });

  it("uses the consumed native result id for both the saved run and completion selection", () => {
    const powerCompletionStart = controller.indexOf('envelope.payload.kind === "pls_sample_size_power_v1"');
    const plsCompletionStart = controller.indexOf("const { estimation: result, assessment } = envelope.payload;", powerCompletionStart);
    const completionEnd = controller.indexOf("const executeRun = async", plsCompletionStart);
    const powerCompletion = controller.slice(powerCompletionStart, plsCompletionStart);
    const plsCompletion = controller.slice(plsCompletionStart, completionEnd);

    expect(powerCompletionStart).toBeGreaterThan(-1);
    expect(powerCompletion).toContain('envelope.payload.kind === "pls_sample_size_power_v2"');
    expect(plsCompletionStart).toBeGreaterThan(powerCompletionStart);
    expect(completionEnd).toBeGreaterThan(plsCompletionStart);
    expect(powerCompletion).toContain("id: envelope.id,");
    expect(powerCompletion).toContain("modelId,");
    expect(powerCompletion).toContain("lastRunId: envelope.id,");
    expect(powerCompletion.match(/addRun\(completedRun\);/g)).toHaveLength(1);
    expect(plsCompletion).toContain("id: envelope.id,");
    expect(plsCompletion).toContain("modelId: standalone ? null : modelId,");
    expect(plsCompletion).toContain("lastRunId: envelope.id,");
    expect(plsCompletion.match(/addRun\(completedRun\);/g)).toHaveLength(1);
    expect(app).toContain("resolveSelectedCompletedRun(completedRuns, selectedResultRunId)");
    expect(app).toContain("setGeneralSemResultSelected(false);");
    expect(app).toContain("setSelectedResultRun(id);");
    expect(app).not.toContain("useState(selectedResultRunId");
  });

  it("persists and exposes only result-backed completed jobs", () => {
    const runStart = controller.indexOf("const runAnalysis = async");
    const resultFetch = controller.indexOf("const envelope = await getNativePlsJobResult(job.id);", runStart);
    const compatibilityGate = controller.indexOf(
      'if (!envelope || envelope.payload.kind === "legacy") throw new Error("The completed job did not return a compatible result.");',
      resultFetch,
    );
    const powerCompletionStart = controller.indexOf('envelope.payload.kind === "pls_sample_size_power_v1"', compatibilityGate);
    const plsCompletionStart = controller.indexOf("const { estimation: result, assessment } = envelope.payload;", powerCompletionStart);
    const runEnd = controller.indexOf("const executeRun = async", plsCompletionStart);
    const powerCompletion = controller.slice(powerCompletionStart, plsCompletionStart);
    const plsCompletion = controller.slice(plsCompletionStart, runEnd);

    expect(runStart).toBeGreaterThan(-1);
    expect(resultFetch).toBeGreaterThan(runStart);
    expect(compatibilityGate).toBeGreaterThan(resultFetch);
    expect(powerCompletionStart).toBeGreaterThan(compatibilityGate);
    expect(powerCompletion).toContain('envelope.payload.kind === "pls_sample_size_power_v2"');
    expect(plsCompletionStart).toBeGreaterThan(powerCompletionStart);
    expect(controller.slice(runStart, resultFetch)).not.toContain("addRun(completedRun);");
    expect(powerCompletion).toContain('status: "completed"');
    expect(powerCompletion.match(/addRun\(completedRun\);/g)).toHaveLength(1);
    expect(plsCompletion).toContain('status: "completed"');
    expect(plsCompletion.match(/addRun\(completedRun\);/g)).toHaveLength(1);
    expect(app).toContain("const completedRuns = useMemo(() => completedResultRuns(runs), [runs]);");
  });

  it("stores completed-run logs and exposes them on demand", () => {
    expect(controller).toContain("...useWorkspace.getState().runMonitor.logs");
    expect(app).toContain("Calculation log ({run.logs.length})");
  });

  it("latches cancellation before the native job id is available and forwards it once accepted", () => {
    const runStart = controller.indexOf("const runAnalysis = async");
    const nativeStart = controller.indexOf("let job = await startNativePlsJob(recipe);", runStart);
    const latchedForward = controller.indexOf("if (calculationCancellationRequestedRef.current)", nativeStart);
    const nativeCancel = controller.indexOf("job = await cancelNativePlsJob(job.id);", latchedForward);
    const cancelStart = controller.indexOf("const cancelAnalysis = async");
    const latch = controller.indexOf("calculationCancellationRequestedRef.current = true;", cancelStart);
    const storeFallback = controller.indexOf("const jobId = activeJob?.id ?? monitor.activeJobId;", cancelStart);
    const pendingReturn = controller.indexOf("if (!jobId) return;", storeFallback);

    expect(controller).toContain("const calculationCancellationRequestedRef = useRef(false);");
    expect(latchedForward).toBeGreaterThan(nativeStart);
    expect(nativeCancel).toBeGreaterThan(latchedForward);
    expect(latch).toBeGreaterThan(cancelStart);
    expect(storeFallback).toBeGreaterThan(latch);
    expect(pendingReturn).toBeGreaterThan(storeFallback);
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
      ["permutation", "pls_pm", [0, 0, 999], "candidate_freedman_lane_path_randomization_scope"],
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
    expect(app).toContain("createNativeCalculationRequest(calculationKind, calculationSettings, dataProfile)");
    expect(controller).toContain("parseNativeCalculationRequest");
    expect(controller).toContain("nativeAnalysisRecipeDescriptor(request.kind).label");
    expect(calculationDialog).toContain('role="listbox"');
    expect(analysisCatalog).toContain("NATIVE_PREDICTION_SCOPE_DESCRIPTION");
    expect(controller).toContain('const permutation = envelope.payload.kind === "pls_pm_v3"');
    expect(controller).toMatch(/const completedRun: AnalysisRun = \{[\s\S]*permutation,[\s\S]*\};[\s\S]*addRun\(completedRun\);/);
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
    const plsCompletionStart = controller.indexOf("const { estimation: result, assessment } = envelope.payload;");
    const completion = controller.slice(plsCompletionStart, controller.indexOf("const executeRun = async", plsCompletionStart));

    expect(controller).toContain("const standalone = isStandaloneNativeAnalysis(request.kind)");
    expect(construction).toContain("const transientModelId = crypto.randomUUID()");
    expect(construction).toContain("const modelId = standalone ? transientModelId : (currentState.activeModelId ?? transientModelId)");
    expect(construction).not.toContain("standalone-${recipeId}");
    expect(construction).toContain("if (!standalone)");
    expect(construction).toContain("nodes: standalone ? [] : nodes");
    expect(construction).toContain("edges: standalone ? [] : edges");
    expect(plsCompletionStart).toBeGreaterThan(-1);
    expect(completion).toContain("modelId: standalone ? null : modelId");
    expect(completion).toContain("...(!standalone && modelSnapshot ? { modelSnapshot } : {})");
  });

  it("keeps calculation edits local until Start and lets Close discard the draft", () => {
    expect(app).toContain("const [calculationDraft, setCalculationDraft]");
    expect(app).toContain("setCalculationDraft(nativeAnalysisSettingsForWorkbenchKind(analysisSettings, preferredKind))");
    expect(app).toContain("setSettings={(patch) => setCalculationDraft");
    expect(app).toMatch(/const startCalculation = \(dataProfile\?: NativeLogisticProfile \| NativeProcessProfile\) => \{[\s\S]*setAnalysisSettings\(calculationSettings\);[\s\S]*createNativeCalculationRequest\(calculationKind, calculationSettings, dataProfile\)/);
    expect(calculationDialog).toContain('<button type="button" onClick={close}>Close</button>');
  });

});
