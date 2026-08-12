import { BookOpen, ChevronDown, Download, FileSpreadsheet, FlaskConical, FolderOpen, Keyboard, Menu, Play, Plus, RotateCcw, Save, Settings, ShieldCheck, Square, Table2, Upload, X } from "lucide-react";
import Papa from "papaparse";
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { DESKTOP_MENU_ORDER } from "../domain/desktopCommands";
import { evaluateMethodApplicability, topBarMethods } from "../domain/methodApplicability";
import { effectiveMethodStatus, isSelectableAnalysisMethod, methodStatusDescription, methodStatusLabel } from "../domain/methodStatus";
import { useWorkspace } from "../store";
import type { AnalysisMethodId, Dataset, DesktopDialogId, DesktopMenuId, JobSnapshot, WorkspaceView } from "../types";
import { cancelNativePlsJob, createNativeProject, dismissNativePlsJob, getNativePlsJob, getNativePlsJobResult, importNativeDataset, isNativeDesktop, openNativeDemoProject, openNativeProject, saveNativeProject, startNativePlsJob } from "../services/projectService";

export function TopBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const projectInputRef = useRef<HTMLInputElement>(null);
  const [activeJob, setActiveJob] = useState<JobSnapshot | null>(null);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const setDataset = useWorkspace((state) => state.setDataset);
  const resetProject = useWorkspace((state) => state.resetProject);
  const addRun = useWorkspace((state) => state.addRun);
  const runs = useWorkspace((state) => state.runs);
  const loadProject = useWorkspace((state) => state.loadProject);
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const setProjectMeta = useWorkspace((state) => state.setProjectMeta);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const setAnalysisSettings = useWorkspace((state) => state.setAnalysisSettings);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const setCommandPaletteOpen = useWorkspace((state) => state.setCommandPaletteOpen);
  const setShortcutOverlayOpen = useWorkspace((state) => state.setShortcutOverlayOpen);
  const activeDesktopMenu = useWorkspace((state) => state.activeDesktopMenu);
  const activeDesktopDialog = useWorkspace((state) => state.activeDesktopDialog);
  const setActiveDesktopMenu = useWorkspace((state) => state.setActiveDesktopMenu);
  const setActiveDesktopDialog = useWorkspace((state) => state.setActiveDesktopDialog);
  const setDesktopCommandStatus = useWorkspace((state) => state.setDesktopCommandStatus);
  const setRunMonitor = useWorkspace((state) => state.setRunMonitor);
  const pushToast = useWorkspace((state) => state.pushToast);
  const applicability = evaluateMethodApplicability({ dataset, nodes, edges, settings: analysisSettings, nativeDesktop: isNativeDesktop() });
  const topBarApplicability = topBarMethods(applicability, analysisSettings.method);
  const selectedApplicability = applicability.find((item) => item.method.id === analysisSettings.method);
  const selectedMethod = (methods.filter(isSelectableAnalysisMethod).find((candidate) => candidate.id === analysisSettings.method) ?? methods.find((candidate) => candidate.id === "pls_pm"))!;

  const download = (name: string, contents: string, type: string) => {
    const url = URL.createObjectURL(new Blob([contents], { type }));
    const anchor = document.createElement("a");
    anchor.href = url; anchor.download = name; anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };

  const saveProject = async (saveAs = false) => {
    if (!isNativeDesktop()) { download("corporate-reputation.qpls.json", JSON.stringify({ schemaVersion: 1, nodes, edges, dataset, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout }, null, 2), "application/json"); return; }
    const saved = await saveNativeProject(saveAs ? null : projectPath, { nodes, edges, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout, activeDatasetId: dataset.id });
    if (saved) {
      setProjectMeta(saved.name, saved.path);
      pushToast({ tone: "success", title: "Project saved", detail: saved.path ?? saved.name });
    }
  };
  const openProject = async (file?: File) => {
    if (!file) return;
    const project = JSON.parse(await file.text()) as { schemaVersion: number; nodes: typeof nodes; edges: typeof edges; dataset: typeof dataset; runs?: typeof runs; analysisSettings?: typeof analysisSettings; diagramMode?: typeof diagramMode; diagramOverlaySettings?: typeof diagramOverlaySettings; publicationDiagramSettings?: typeof publicationDiagramSettings; diagramLayout?: typeof diagramLayout };
    if (project.schemaVersion !== 1 || !Array.isArray(project.nodes) || !Array.isArray(project.edges)) throw new Error("Unsupported QuickPLS project");
    loadProject(project);
  };
  const loadNativeProjectSnapshot = (project: Awaited<ReturnType<typeof openNativeProject>>) => {
    if (!project) return;
    const workspace = project.workspace as { nodes: typeof nodes; edges: typeof edges; runs?: typeof runs; analysisSettings?: typeof analysisSettings; diagramMode?: typeof diagramMode; diagramOverlaySettings?: typeof diagramOverlaySettings; publicationDiagramSettings?: typeof publicationDiagramSettings; diagramLayout?: typeof diagramLayout; activeDatasetId?: string } | null | undefined;
    const activeDataset = project.datasets.find((candidate) => candidate.id === workspace?.activeDatasetId) ?? project.datasets[0] ?? dataset;
    if (workspace?.nodes && workspace?.edges) loadProject({ ...workspace, dataset: activeDataset });
    else if (project.datasets[0]) setDataset(project.datasets[0]);
    setProjectMeta(project.name, project.path);
    pushToast({ tone: project.recovered ? "warning" : "success", title: project.recovered ? "Project recovered" : "Project opened", detail: project.name });
  };
  const openProjectCommand = async () => {
    if (!isNativeDesktop()) { projectInputRef.current?.click(); return; }
    const project = await openNativeProject();
    loadNativeProjectSnapshot(project);
    if (project?.recovered) window.alert(project.recoverySource === "autosave" ? "QuickPLS recovered newer autosaved work." : "The primary project was damaged. QuickPLS opened the previous valid backup.");
  };
  const openDemoProjectCommand = async () => {
    if (!isNativeDesktop()) { window.alert("The demo evidence project opens in the native QuickPLS desktop application."); return; }
    loadNativeProjectSnapshot(await openNativeDemoProject());
  };
  const newProjectCommand = async () => { resetProject(); if (isNativeDesktop()) await createNativeProject(); pushToast({ tone: "info", title: "New project ready", detail: "Build a model or import a dataset." }); };
  const importDataCommand = async () => {
    if (!isNativeDesktop()) { inputRef.current?.click(); return; }
    const value = await importNativeDataset();
    if (value) {
      setDataset(value);
      pushToast({ tone: "success", title: "Dataset imported", detail: `${value.name}: ${value.rowCount ?? value.rows.length} rows, ${value.columns.length} variables` });
    }
  };
  const exportSummary = () => {
    download("quickpls-foundation-summary.html", `<!doctype html><meta charset="utf-8"><title>QuickPLS foundation summary</title><h1>QuickPLS foundation summary</h1><p>Dataset: ${dataset.name}</p><p>Rows: ${dataset.rows.length}; constructs: ${nodes.length}; paths: ${edges.length}</p><p><strong>Scope:</strong> supported analyses are validated only inside the documented QuickPLS method scope after a saved run is selected.</p>`, "text/html");
    pushToast({ tone: "success", title: "Summary exported", detail: "HTML summary download started." });
  };

  const importCsv = (file?: File) => {
    if (!file) return;
    Papa.parse<Record<string, string | number | null>>(file, {
      header: true,
      dynamicTyping: true,
      skipEmptyLines: true,
      complete: ({ data, meta }) => {
        const missing = data.reduce((count, row) => count + Object.values(row).filter((value) => value === null || value === "").length, 0);
        setDataset({ id: crypto.randomUUID(), name: file.name, columns: meta.fields ?? [], rows: data, missing });
        pushToast({ tone: "success", title: "CSV imported", detail: `${file.name}: ${data.length} rows, ${(meta.fields ?? []).length} variables` });
      },
    });
  };

  const readiness = analysisReadiness({ dataset, nodes, edges, settings: analysisSettings, nativeDesktop: isNativeDesktop() });
  const topBlocker = readiness.blockers[0];
  const canRun = readiness.canRun;
  const runState = activeJob ? "running" : canRun ? "ready" : "blocked";
  const runBlockerTarget = (topBlocker?.actionView ?? "analyses") as WorkspaceView;
  const runBlockerAction = topBlocker?.actionLabel ?? (topBlocker?.actionView ? `Open ${topBlocker.actionView}` : "Review setup");
  const runDisabledLabel = topBlocker ? `Run disabled: ${topBlocker.detail}` : `Run disabled: ${readiness.summary}`;
  const runBlockerSummary = topBlocker
    ? `${topBlocker.label}: ${runBlockerAction}`
    : readiness.summary;
  const openRunBlockerTarget = () => {
    if (!topBlocker) return;
    setView(runBlockerTarget, {
      from: view,
      to: runBlockerTarget,
      actionLabel: `Run blocker: ${topBlocker.label}`,
      coachId: "top-command-bar",
    });
  };
  const runAnalysis = async () => {
    const startedAt = new Date().toISOString();
    setRunMonitor({
      status: "queued",
      phase: "Queued",
      message: `${selectedMethod.name} is waiting for validation.`,
      completedUnits: 0,
      totalUnits: 5,
      startedAt,
      completedAt: null,
      activeJobId: null,
      lastRunId: null,
      error: null,
    }, { phase: "Queued", message: `${selectedMethod.name} run requested from the desktop command surface.`, tone: "info" });
    if (!dataset.fingerprint) {
      setRunMonitor({
        status: "failed",
        phase: "Validation failed",
        message: "Import and save a dataset before running an analysis.",
        completedUnits: 1,
        totalUnits: 5,
        completedAt: new Date().toISOString(),
        error: "Dataset fingerprint missing",
      }, { phase: "Validation failed", message: "Dataset fingerprint is missing; run was not launched.", tone: "error" });
      throw new Error("Import and save a dataset before running an analysis.");
    }
    setRunMonitor({
      status: "validating",
      phase: "Validating",
      message: "Checking dataset fingerprint, SEM recipe, method scope, and run settings.",
      completedUnits: 1,
      totalUnits: 5,
    }, { phase: "Validating", message: "Dataset, model, and method setup passed the frontend readiness checks.", tone: "success" });
    const createdAt = new Date().toISOString();
    const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance");
    const controls = edges
      .filter((edge) => edge.data?.role === "control")
      .map((edge) => ({
        source: edge.source,
        target: edge.target,
        label: typeof edge.data?.controlLabel === "string" && edge.data.controlLabel.trim() ? edge.data.controlLabel.trim() : null,
      }));
    const metadata = {
      status: analysisSettings.method === "pls_pm" ? "validated_v1_0_supported_pls_scope" : analysisSettings.method === "cbsem" ? "validated_v1_0_supported_cbsem_scope" : ["pca", "gsca", "regression", "nca"].includes(analysisSettings.method) ? "validated_v1_0_supported_extended_methods_scope" : "validated_v1_0_supported_prediction_groups_scope",
      ...(analysisSettings.groupColumn ? { mga_group_column: analysisSettings.groupColumn } : {}),
      ...(analysisSettings.ipmaTargets ? { ipma_targets: analysisSettings.ipmaTargets } : {}),
      ...(analysisSettings.method === "mga" && analysisSettings.groupMethods ? { group_methods: analysisSettings.groupMethods } : {}),
      ...(analysisSettings.method === "mga" ? { group_permutation_samples: String(analysisSettings.groupPermutationSamples ?? 999) } : {}),
      ...(analysisSettings.method === "predict" ? {
        group_methods: analysisSettings.groupMethods?.includes("fimix") ? "fimix" : "pls_pos",
        segment_count: String(analysisSettings.segmentCount ?? 2),
        segment_starts: String(analysisSettings.segmentStarts ?? 10),
        minimum_segment_share: String(analysisSettings.minimumSegmentShare ?? 0.10),
      } : {}),
      ...(analysisSettings.method === "cbsem" ? {
        cbsem_model_type: analysisSettings.cbsemModelType ?? "sem",
        cbsem_estimator: "ml",
        cbsem_input: "raw",
        cbsem_mean_structure: String(Boolean(analysisSettings.cbsemMeanStructure)),
        cbsem_standardization: analysisSettings.cbsemStandardization ?? "std_all",
        ...(analysisSettings.cbsemGroupColumn ? { cbsem_group_column: analysisSettings.cbsemGroupColumn } : {}),
        cbsem_invariance_steps: analysisSettings.cbsemInvarianceSteps ?? "configural,metric,scalar",
        ...(analysisSettings.cbsemBootstrapSamples && analysisSettings.cbsemBootstrapSamples > 0 ? { cbsem_bootstrap_samples: String(analysisSettings.cbsemBootstrapSamples) } : {}),
      } : {}),
      ...(analysisSettings.method === "pca" ? {
        ...(analysisSettings.pcaVariables ? { pca_variables: analysisSettings.pcaVariables } : {}),
        pca_component_rule: analysisSettings.pcaComponentRule ?? "kaiser",
        pca_components: String(analysisSettings.pcaComponents ?? 2),
      } : {}),
      ...(analysisSettings.method === "regression" ? {
        regression_type: analysisSettings.regressionType ?? "ols",
        ...(analysisSettings.regressionOutcome ? { regression_outcome: analysisSettings.regressionOutcome } : {}),
        ...(analysisSettings.regressionPredictors ? { regression_predictors: analysisSettings.regressionPredictors } : {}),
        ...(analysisSettings.regressionControls ? { regression_controls: analysisSettings.regressionControls } : {}),
        robust_se: analysisSettings.robustSe ?? "hc3",
        process_model: analysisSettings.processModel ?? "mediation",
        ...(analysisSettings.processX ? { process_x: analysisSettings.processX } : {}),
        ...(analysisSettings.processM ? { process_m: analysisSettings.processM } : {}),
        ...(analysisSettings.processW ? { process_w: analysisSettings.processW } : {}),
      } : {}),
      ...(analysisSettings.method === "nca" ? {
        ...(analysisSettings.ncaX ? { nca_x: analysisSettings.ncaX } : {}),
        ...(analysisSettings.ncaY ? { nca_y: analysisSettings.ncaY } : {}),
        nca_ceiling: analysisSettings.ncaCeiling ?? "both",
        nca_permutation_samples: String(analysisSettings.ncaPermutationSamples ?? 999),
      } : {}),
    };
    const recipe = {
      schema_version: 2, id: crypto.randomUUID(), created_at: createdAt, dataset_fingerprint: dataset.fingerprint,
      model: {
        id: crypto.randomUUID(),
        name: projectName,
        constructs: nodes.map((node) => ({ id: node.id, name: node.data.label, short_name: node.data.shortName, mode: node.data.mode, indicators: node.data.indicators })),
        paths: structuralEdges.map((edge) => ({ source: edge.source, target: edge.target })),
        controls,
        higher_order_constructs: nodes.filter((node) => node.data.semantic === "higher_order" && node.data.higherOrder).map((node) => ({
          id: node.id,
          components: node.data.higherOrder!.components,
          method: node.data.higherOrder!.method,
          stage_one_recipe: node.data.higherOrder!.stage_one_recipe ?? null,
        })),
        interactions: nodes.filter((node) => node.data.semantic === "interaction" && node.data.interaction).map((node) => ({
          id: node.id,
          predictor: node.data.interaction!.predictor,
          moderator: node.data.interaction!.moderator,
          product_construct: node.id,
          outcome: node.data.interaction!.outcome,
          method: node.data.interaction!.method,
        })),
      },
      settings: { method: analysisSettings.method, weighting_scheme: "path", tolerance: 1e-7, max_iterations: 3000, bootstrap_samples: analysisSettings.bootstrapSamples, studentized_inner_samples: analysisSettings.studentizedInnerSamples, permutation_samples: analysisSettings.permutationSamples, seed: analysisSettings.seed, workers: analysisSettings.workers, confidence_level: analysisSettings.confidenceLevel, preprocessing: "standardized", missing_data: "listwise_deletion", ...(analysisSettings.caseWeightColumn ? { case_weight_column: analysisSettings.caseWeightColumn } : {}) }, metadata,
    };
    let job = await startNativePlsJob(recipe);
    setActiveJob(job);
    setRunMonitor({
      status: job.state === "queued" ? "queued" : "running",
      phase: job.phase || "Engine",
      message: job.message ?? "Native QuickPLS engine accepted the calculation job.",
      completedUnits: job.completed_units,
      totalUnits: job.total_units,
      activeJobId: job.id,
    }, { phase: job.phase || "Engine", message: "Native calculation job started.", tone: "info" });
    while (!["completed", "failed", "cancelled"].includes(job.state)) {
      await new Promise((resolve) => window.setTimeout(resolve, 150));
      job = await getNativePlsJob(job.id);
      setActiveJob(job);
      setRunMonitor({
        status: job.state === "cancelling" ? "cancelling" : job.state === "queued" ? "queued" : "running",
        phase: job.phase || "Engine",
        message: job.message ?? "QuickPLS engine is processing the calculation.",
        completedUnits: job.completed_units,
        totalUnits: job.total_units,
        activeJobId: job.id,
      });
    }
    if (job.state === "cancelled") {
      await dismissNativePlsJob(job.id);
      setActiveJob(null);
      setRunMonitor({
        status: "cancelled",
        phase: "Cancelled",
        message: job.message ?? "Calculation was cancelled before completion.",
        completedUnits: job.completed_units,
        totalUnits: job.total_units,
        completedAt: new Date().toISOString(),
        activeJobId: null,
      }, { phase: "Cancelled", message: "The active calculation job was cancelled.", tone: "warning" });
      return;
    }
    if (job.state === "failed") {
      const message = job.message ?? "PLS analysis failed";
      await dismissNativePlsJob(job.id);
      setActiveJob(null);
      setRunMonitor({
        status: "failed",
        phase: "Failed",
        message,
        completedUnits: job.completed_units,
        totalUnits: job.total_units,
        completedAt: new Date().toISOString(),
        activeJobId: null,
        error: message,
      }, { phase: "Failed", message, tone: "error" });
      throw new Error(message);
    }
    const envelope = await getNativePlsJobResult(job.id);
    setActiveJob(null);
    if (!envelope) throw new Error("Completed PLS job did not return a result");
    if (envelope.payload.kind === "legacy") throw new Error("The completed job returned an incompatible result payload");
    const { estimation: result, assessment } = envelope.payload;
    const bootstrap = envelope.payload.kind === "pls_pm_v2" ? envelope.payload.bootstrap : envelope.payload.kind === "pls_pm_v3" ? envelope.payload.bootstrap ?? undefined : undefined;
    const permutation = envelope.payload.kind === "pls_pm_v3" ? envelope.payload.permutation ?? undefined : undefined;
    addRun({ id: envelope.id, name: `${selectedMethod.name} run`, method: selectedMethod.name, createdAt: envelope.provenance.completed_at, seed: envelope.provenance.seed, status: "completed", warnings: ["Validated for the documented QuickPLS supported scope; unsupported shapes remain blocked or explicitly marked.", ...envelope.diagnostics.filter((item) => item.level === "warning").map((item) => item.message)], fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12), result, assessment, bootstrap, permutation });
    setRunMonitor({
      status: "completed",
      phase: "Completed",
      message: `${selectedMethod.name} completed with ${result.iterations} iterations.`,
      completedUnits: job.total_units,
      totalUnits: job.total_units,
      completedAt: envelope.provenance.completed_at,
      activeJobId: null,
      lastRunId: envelope.id,
      error: null,
    }, { phase: "Completed", message: `Run saved with fingerprint ${envelope.provenance.dataset_fingerprint.slice(0, 12)}.`, tone: "success" });
    pushToast({ tone: "success", title: "Run completed", detail: `${selectedMethod.name} finished with ${result.iterations} iterations.` });
  };
  const cancelAnalysis = async () => {
    if (!activeJob) return;
    setRunMonitor({
      status: "cancelling",
      phase: "Cancelling",
      message: "Cancellation requested. Waiting for the engine to stop safely.",
      activeJobId: activeJob.id,
    }, { phase: "Cancelling", message: "User requested cancellation from the command bar.", tone: "warning" });
    setActiveJob(await cancelNativePlsJob(activeJob.id));
  };
  const recordCommand = (id: string, label: string, detail = "Command completed.") => {
    setDesktopCommandStatus({ id, label, detail, tone: "info" });
  };
  const openDialog = (dialog: Exclude<DesktopDialogId, null>) => {
    setActiveDesktopDialog(dialog);
    recordCommand(`dialog.${dialog}`, `Open ${dialog.replaceAll("_", " ")}`, "Desktop task dialog opened.");
  };
  const runIfReady = () => {
    if (!canRun) {
      setRunMonitor({
        status: "blocked",
        phase: "Blocked",
        message: topBlocker?.detail ?? readiness.summary,
        completedUnits: 0,
        totalUnits: 0,
        startedAt: null,
        completedAt: null,
        activeJobId: null,
        error: topBlocker?.label ?? readiness.summary,
      }, { phase: "Blocked", message: topBlocker?.detail ?? readiness.summary, tone: "warning" });
      openRunBlockerTarget();
      return;
    }
    void runAnalysis().catch((error) => { setActiveJob(null); window.alert(error); });
  };
  const goTo = (target: WorkspaceView, actionLabel: string) => {
    setView(target, { from: view, to: target, actionLabel, coachId: "desktop-menu" });
    recordCommand(`window.${target}`, actionLabel, "Workspace activated from the desktop menu.");
  };
  type MenuConfig = { id: DesktopMenuId; label: string; items: Array<{ label: string; hint?: string; action: () => void; disabled?: boolean }> };
  const menus: MenuConfig[] = ([
    { id: "file", label: "File", items: [
      { label: "New Project...", hint: "Create a blank QuickPLS project", action: () => openDialog("new_project") },
      { label: "Open Project...", hint: "Select a .qpls project", action: () => { void openProjectCommand().catch((error) => window.alert(error)); } },
      { label: "Open Demo Project", hint: "Load bundled sample data", action: () => { void openDemoProjectCommand().catch((error) => window.alert(error)); } },
      { label: "Save Project", hint: projectPath ?? "Choose a save location", action: () => { void saveProject().catch((error) => window.alert(error)); } },
      { label: "Import Data...", hint: "CSV, XLSX, SAV, covariance or correlation", action: () => openDialog("import_data") },
      { label: "Export Options...", hint: "Report, tables, and diagram exports", action: () => openDialog("export_options") },
    ] },
    { id: "edit", label: "Edit", items: [
      { label: "Undo", hint: "Undo last diagram edit", action: undo },
      { label: "Redo", hint: "Redo last diagram edit", action: redo },
      { label: "Reset Project", hint: "Clear the current workspace", action: resetProject },
      { label: "Command Palette", hint: "Find app commands", action: () => setCommandPaletteOpen(true) },
    ] },
    { id: "data", label: "Data", items: [
      { label: "Open Data Workspace", action: () => goTo("data", "Open Data") },
      { label: "Import Data...", action: () => openDialog("import_data") },
      { label: "Load Demo Dataset", action: () => { void openDemoProjectCommand().catch((error) => window.alert(error)); } },
    ] },
    { id: "model", label: "Model", items: [
      { label: "Open SEM Designer", action: () => goTo("models", "Open Model") },
      { label: "Add Latent Construct", action: () => addConstruct() },
      { label: "Arrange Like SmartPLS", action: () => autoLayout("smartpls") },
      { label: uiPreferences.focusDiagramMode ? "Exit Focus Diagram" : "Focus Diagram", action: () => setUiPreferences({ focusDiagramMode: !uiPreferences.focusDiagramMode }) },
    ] },
    { id: "calculate", label: "Calculate", items: [
      { label: "Calculation Setup...", action: () => openDialog("calculation_setup") },
      { label: "Open Setup", action: () => goTo("analyses", "Open Setup") },
      { label: "Run Selected Method", hint: canRun ? selectedMethod.name : readiness.summary, action: runIfReady, disabled: activeJob !== null },
      { label: "Cancel Running Job", action: () => { void cancelAnalysis(); }, disabled: activeJob === null },
    ] },
    { id: "results", label: "Results", items: [
      { label: "Open Results", action: () => goTo("runs", "Open Results") },
      { label: "Method Confidence...", action: () => openDialog("method_scope") },
      { label: "Copy Run List", action: () => { void navigator.clipboard?.writeText(runs.map((run) => `${run.name}\t${run.createdAt}\t${run.method}`).join("\n")); } },
    ] },
    { id: "report", label: "Report", items: [
      { label: "Open Publication Report", action: () => goTo("reports", "Open Report") },
      { label: "Export Summary HTML", action: exportSummary },
      { label: "Export Options...", action: () => openDialog("export_options") },
    ] },
    { id: "view", label: "View", items: [
      { label: "Home", action: () => goTo("welcome", "Open Home") },
      { label: "Data", action: () => goTo("data", "Open Data") },
      { label: "Model", action: () => goTo("models", "Open Model") },
      { label: "Setup", action: () => goTo("analyses", "Open Setup") },
      { label: "Trust Center", action: () => goTo("trust", "Open Trust Center") },
      { label: "Settings...", action: () => openDialog("settings") },
    ] },
    { id: "tools", label: "Tools", items: [
      { label: "Method Scope / Trust Evidence...", hint: "Validated scopes, references, known limitations", action: () => openDialog("method_scope") },
      { label: "Open Trust Center", hint: "Compatibility matrix and validation evidence", action: () => goTo("trust", "Open Trust Center") },
      { label: "Preferences...", hint: "Desktop settings and defaults", action: () => openDialog("settings") },
      { label: "Command Palette", hint: "Find actions without leaving the keyboard", action: () => setCommandPaletteOpen(true) },
    ] },
    { id: "window", label: "Window", items: [
      { label: "Home", hint: "Project start center", action: () => goTo("welcome", "Open Home") },
      { label: "Data Workbench", hint: "Data view and variable view", action: () => goTo("data", "Open Data") },
      { label: "Model Workbench", hint: "SEM diagram designer", action: () => goTo("models", "Open Model") },
      { label: "Setup Center", hint: "Calculation settings and applicability", action: () => goTo("analyses", "Open Setup") },
      { label: "Results Workbook", hint: "Saved runs, interpretation, comparison", action: () => goTo("runs", "Open Results") },
      { label: "Report Wizard", hint: "Figure and table exports", action: () => goTo("reports", "Open Report") },
      { label: uiPreferences.focusDiagramMode ? "Exit Focus Diagram" : "Focus Diagram", hint: "Collapse side panes around the SEM canvas", action: () => setUiPreferences({ focusDiagramMode: !uiPreferences.focusDiagramMode }) },
    ] },
    { id: "help", label: "Help", items: [
      { label: "Shortcuts...", action: () => openDialog("help_shortcuts") },
      { label: "Show Shortcut Overlay", action: () => setShortcutOverlayOpen(true) },
      { label: "Method Scope / Trust Evidence...", action: () => openDialog("method_scope") },
    ] },
  ] as MenuConfig[]).sort((left, right) => DESKTOP_MENU_ORDER.findIndex((menu) => menu.id === left.id) - DESKTOP_MENU_ORDER.findIndex((menu) => menu.id === right.id));
  const runOutputPreview = [
    "Path coefficients and total effects",
    "Latent scores, loadings, and weights",
    "R2 and model diagnostics",
    analysisSettings.bootstrapSamples > 0 ? "Bootstrap inference" : "Bootstrap inference unavailable until enabled",
    analysisSettings.permutationSamples > 0 ? "Permutation output" : "Permutation output unavailable until enabled",
  ];
  useEffect(() => {
    const handleRunRequest = () => {
      if (activeJob) return;
      if (!canRun) {
        setRunMonitor({
          status: "blocked",
          phase: "Blocked",
          message: topBlocker?.detail ?? readiness.summary,
          completedUnits: 0,
          totalUnits: 0,
          startedAt: null,
          completedAt: null,
          activeJobId: null,
          error: topBlocker?.label ?? readiness.summary,
        }, { phase: "Blocked", message: topBlocker?.detail ?? readiness.summary, tone: "warning" });
        return;
      }
      void runAnalysis().catch((error) => { setActiveJob(null); window.alert(error); });
    };
    const handleCancelRunRequest = () => {
      if (!activeJob) return;
      void cancelAnalysis().catch((error) => window.alert(error));
    };
    const handleOpenProject = () => { void openProjectCommand().catch((error) => window.alert(error)); };
    const handleOpenDemo = () => { void openDemoProjectCommand().catch((error) => window.alert(error)); };
    const handleSaveProject = () => { void saveProject().catch((error) => window.alert(error)); };
    const handleSaveProjectAs = () => { void saveProject(true).catch((error) => window.alert(error)); };
    const handleImportData = () => { void importDataCommand().catch((error) => window.alert(error)); };
    window.addEventListener("quickpls:run-analysis", handleRunRequest);
    window.addEventListener("quickpls:cancel-analysis", handleCancelRunRequest);
    window.addEventListener("quickpls:open-project", handleOpenProject);
    window.addEventListener("quickpls:open-demo-project", handleOpenDemo);
    window.addEventListener("quickpls:save-project", handleSaveProject);
    window.addEventListener("quickpls:save-project-as", handleSaveProjectAs);
    window.addEventListener("quickpls:import-data", handleImportData);
    return () => {
      window.removeEventListener("quickpls:run-analysis", handleRunRequest);
      window.removeEventListener("quickpls:cancel-analysis", handleCancelRunRequest);
      window.removeEventListener("quickpls:open-project", handleOpenProject);
      window.removeEventListener("quickpls:open-demo-project", handleOpenDemo);
      window.removeEventListener("quickpls:save-project", handleSaveProject);
      window.removeEventListener("quickpls:save-project-as", handleSaveProjectAs);
      window.removeEventListener("quickpls:import-data", handleImportData);
    };
  }, [activeJob, canRun, runAnalysis, cancelAnalysis]);
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setActiveDesktopMenu(null);
        if (activeDesktopDialog) setActiveDesktopDialog(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeDesktopDialog, setActiveDesktopDialog, setActiveDesktopMenu]);

  return <>
    <header className="title-bar q2-desktop-title" data-v216-desktop-shell="title-strip">
      <Menu size={16} /><strong>QuickPLS 2.0</strong><span className="project-title">{projectName}.qpls</span>
        <span className="alpha-mark">v2.43.0 full native wiring</span>
    </header>
    <nav className="desktop-menu-bar" aria-label="Desktop menu" data-v216-desktop-shell="menu-bar" data-v221-native-shell="desktop-menu" data-v222-command-registry="enabled">
      {menus.map((menu) => (
        <div className="desktop-menu" key={menu.id}>
          <button
            type="button"
            className={activeDesktopMenu === menu.id ? "active" : ""}
            aria-expanded={activeDesktopMenu === menu.id}
            aria-haspopup="menu"
            onClick={() => setActiveDesktopMenu(activeDesktopMenu === menu.id ? null : menu.id)}
          >
            {menu.label}<ChevronDown size={12} />
          </button>
          {activeDesktopMenu === menu.id ? (
            <div className="desktop-menu-popover" role="menu">
              {menu.items.map((item) => (
                <button
                  key={item.label}
                  type="button"
                  role="menuitem"
                  disabled={Boolean(item.disabled)}
                  onClick={() => {
                    if (item.disabled) return;
                    setActiveDesktopMenu(null);
                    item.action();
                    recordCommand(`menu.${menu.id}.${item.label.toLowerCase().replace(/[^a-z0-9]+/g, "_")}`, item.label, item.hint ?? "Menu command executed.");
                  }}
                >
                  <span>{item.label}</span>
                  {item.hint ? <small>{item.hint}</small> : null}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      ))}
    </nav>
    <div className="command-bar q2-desktop-command-strip" data-v216-desktop-shell="command-strip" data-v221-native-shell="command-strip" data-v222-command-feedback="status-bar">
      <button className="icon-command" aria-label="New project" title="New project" onClick={() => { void newProjectCommand().catch((error) => window.alert(error)); }}><Plus size={17} /><span>New</span></button>
      <button className="icon-command" aria-label="Open project" title="Open project" onClick={() => { void openProjectCommand().catch((error) => window.alert(error)); }}><FolderOpen size={17} /><span>Open</span></button>
      <button className="icon-command" aria-label="Open demo evidence project" title="Open demo evidence project" onClick={() => { void openDemoProjectCommand().catch((error) => window.alert(error)); }}><FlaskConical size={17} /><span>Demo</span></button>
      <button className="icon-command" aria-label="Save project" title="Save project" onClick={() => { void saveProject().catch((error) => window.alert(error)); }}><Save size={17} /><span>Save</span></button>
      <span className="command-separator" />
      <button className="icon-command" aria-label="Import data" title="Import data" onClick={() => { void importDataCommand().catch((error) => window.alert(error)); }}><Upload size={17} /><span>Import</span></button>
      <button className="icon-command" aria-label="Export report summary" title="Export report" onClick={exportSummary}><Download size={17} /><span>Export</span></button>
      <input ref={inputRef} className="file-input" type="file" accept=".csv,.tsv,text/csv" onChange={(event) => importCsv(event.target.files?.[0])} />
      <input ref={projectInputRef} className="file-input" type="file" accept=".json,.qpls" onChange={(event) => { void openProject(event.target.files?.[0]); }} />
      <span className="command-separator" />
      <button className="icon-command" aria-label="Reset project" title="Reset project" onClick={resetProject}><RotateCcw size={17} /><span>Reset</span></button>
      <div className="command-spacer" />
      <div
        className="command-run-cluster"
        data-command-bar-state={runState}
        data-run-blocker-id={topBlocker?.id ?? ""}
        data-run-blocker-view={!canRun ? runBlockerTarget : ""}
        data-run-method={analysisSettings.method}
        data-method-applicability-status={selectedApplicability?.status ?? ""}
        data-v216-desktop-shell="method-run-cluster"
        data-topbar-guidance-count={topBarApplicability.length}
      >
        <div className="method-picker" title={selectedApplicability ? `${selectedApplicability.reason} Next: ${selectedApplicability.nextActionLabel}` : (selectedMethod ? methodStatusDescription(selectedMethod, analysisSettings) : undefined)}>
          <select className="method-select" aria-label="Analysis method" value={analysisSettings.method} onChange={(event) => setAnalysisSettings({ method: event.target.value as AnalysisMethodId })}>
            {topBarApplicability.map((item) => <option key={item.method.id} value={item.method.id}>{item.method.name}</option>)}
            <option disabled>More methods in Setup</option>
          </select>
          {selectedMethod ? <span className={`status-text ${effectiveMethodStatus(selectedMethod, analysisSettings)}`}>{methodStatusLabel(effectiveMethodStatus(selectedMethod, analysisSettings))}</span> : null}
        </div>
        <button
          className="run-button"
          aria-label={activeJob ? "Cancel active analysis" : `Run ${selectedMethod.name}`}
          aria-describedby={!activeJob && !canRun ? "run-disabled-reason" : undefined}
          disabled={!activeJob && !canRun}
          title={activeJob ? "Cancel the active analysis" : canRun ? `Run ${selectedMethod.name}` : topBlocker?.detail ?? readiness.summary}
          data-run-state={runState}
          data-run-method={analysisSettings.method}
          data-run-disabled-reason={!activeJob && !canRun ? topBlocker?.detail ?? readiness.summary : ""}
          data-run-blocker-id={!activeJob && !canRun ? topBlocker?.id ?? "" : ""}
          data-run-blocker-action={!activeJob && !canRun ? runBlockerAction : ""}
          onClick={() => { void (activeJob ? cancelAnalysis() : runAnalysis()).catch((error) => { setActiveJob(null); window.alert(error); }); }}
        >
          {activeJob ? <Square size={14} fill="currentColor" /> : <Play size={16} fill="currentColor" />}
          <span className="run-button-label">{activeJob ? `${activeJob.phase} ${activeJob.completed_units}/${activeJob.total_units}` : `Run ${selectedMethod.name}`}</span>
        </button>
        {!activeJob && !canRun ? (
          <button
            id="run-disabled-reason"
            type="button"
            className="command-blocker-chip"
            title={topBlocker?.detail ?? readiness.summary}
            aria-label={runDisabledLabel}
            data-run-blocker-id={topBlocker?.id ?? ""}
            data-run-blocker-view={runBlockerTarget}
            data-run-blocker-action={runBlockerAction}
            onClick={openRunBlockerTarget}
          >
            <strong>Run disabled</strong>
            <small>{runBlockerSummary}</small>
          </button>
        ) : null}
      </div>
    </div>
    {activeDesktopDialog ? (
      <div className="desktop-dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setActiveDesktopDialog(null); }}>
        <section className="desktop-dialog" role="dialog" aria-modal="true" aria-labelledby="desktop-dialog-title" data-dialog={activeDesktopDialog}>
          <header className="desktop-dialog-titlebar">
            <strong id="desktop-dialog-title">
              {activeDesktopDialog === "new_project" ? "New Project" :
                activeDesktopDialog === "open_project" ? "Open Project" :
                activeDesktopDialog === "import_data" ? "Import Data" :
                activeDesktopDialog === "export_options" ? "Export Options" :
                activeDesktopDialog === "calculation_setup" ? "Calculation Setup" :
                activeDesktopDialog === "method_scope" ? "Method Scope and Trust Evidence" :
                activeDesktopDialog === "settings" ? "Settings" : "Help and Shortcuts"}
            </strong>
            <button type="button" aria-label="Close dialog" onClick={() => setActiveDesktopDialog(null)}><X size={15} /></button>
          </header>
          <div className="desktop-dialog-body">
            {activeDesktopDialog === "new_project" ? (
              <div className="desktop-dialog-grid">
                <button type="button" className="desktop-dialog-card primary" onClick={() => { setActiveDesktopDialog(null); void newProjectCommand().catch((error) => window.alert(error)); }}>
                  <Plus size={20} /><span>Blank PLS-SEM project</span><small>Create an empty project and start with Data or Model.</small>
                </button>
                <button type="button" className="desktop-dialog-card" onClick={() => { setActiveDesktopDialog(null); void openDemoProjectCommand().catch((error) => window.alert(error)); }}>
                  <FlaskConical size={20} /><span>Demo project</span><small>Open the bundled corporate reputation workflow.</small>
                </button>
                <button type="button" className="desktop-dialog-card" onClick={() => { setActiveDesktopDialog(null); void importDataCommand().catch((error) => window.alert(error)); }}>
                  <FileSpreadsheet size={20} /><span>Start from data</span><small>Import a dataset and let QuickPLS guide method setup.</small>
                </button>
              </div>
            ) : null}
            {activeDesktopDialog === "open_project" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<FolderOpen size={18} />} title="Open a QuickPLS project" detail="Native desktop builds use the Windows file picker for .qpls projects. Browser preview can open compatible JSON snapshots." />
                <button type="button" className="secondary-button bordered" onClick={() => { setActiveDesktopDialog(null); void openProjectCommand().catch((error) => window.alert(error)); }}>Choose project...</button>
              </div>
            ) : null}
            {activeDesktopDialog === "import_data" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<Upload size={18} />} title="Import source" detail="Use raw data for validated PLS workflows. Covariance/correlation imports require sample size and have narrower method availability." />
                <div className="desktop-dialog-form-grid">
                  <label><span>Missing value markers</span><input value="NA, N/A, ." readOnly /></label>
                  <label><span>Current dataset</span><input value={`${dataset.name}: ${dataset.rowCount ?? dataset.rows.length} rows, ${dataset.columns.length} variables`} readOnly /></label>
                </div>
              </div>
            ) : null}
            {activeDesktopDialog === "export_options" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<Download size={18} />} title="Export from Report" detail="Use Report for WYSIWYG SVG, CSV/HTML/XLSX tables, provenance, and reviewer-pack style exports." />
                <div className="desktop-dialog-grid compact">
                  <button type="button" className="desktop-dialog-card" onClick={exportSummary}><Table2 size={18} /><span>HTML summary</span><small>Quick browser-readable export.</small></button>
                  <button type="button" className="desktop-dialog-card" onClick={() => { setActiveDesktopDialog(null); goTo("reports", "Open Report"); }}><Download size={18} /><span>Report workspace</span><small>Choose figure, table, and provenance settings.</small></button>
                </div>
              </div>
            ) : null}
            {activeDesktopDialog === "calculation_setup" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<Play size={18} />} title={selectedMethod.name} detail={canRun ? "Ready to run with the current data, model, and method settings." : readiness.summary} tone={canRun ? "success" : "warning"} />
                <div className="desktop-dialog-columns">
                  <div><h4>Outputs produced</h4><ul>{runOutputPreview.map((item) => <li key={item}>{item}</li>)}</ul></div>
                  <div><h4>Run settings</h4><dl><dt>Seed</dt><dd>{analysisSettings.seed}</dd><dt>Workers</dt><dd>{analysisSettings.workers}</dd><dt>Bootstrap</dt><dd>{analysisSettings.bootstrapSamples || "off"}</dd></dl></div>
                </div>
              </div>
            ) : null}
            {activeDesktopDialog === "method_scope" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<ShieldCheck size={18} />} title="Validated scope, not blanket equivalence" detail={`${selectedMethod.name} is shown as ${methodStatusLabel(effectiveMethodStatus(selectedMethod, analysisSettings)).toLowerCase()} for documented QuickPLS scopes only. R/Rscript and reference engines are validation tools, not runtime dependencies.`} tone="success" />
                <div className="desktop-dialog-columns">
                  <div><h4>Run confidence fields</h4><ul><li>Method version and status</li><li>Data fingerprint</li><li>Recipe fingerprint</li><li>Seed and worker count</li><li>Warnings and known limitations</li></ul></div>
                  <div><h4>Evidence</h4><p>Open Trust Center for method compatibility, validation artifacts, and known differences.</p></div>
                </div>
              </div>
            ) : null}
            {activeDesktopDialog === "settings" ? (
              <div className="desktop-dialog-stack">
                <div className="desktop-dialog-form-grid">
                  <label><span>Density</span><select value={uiPreferences.density} onChange={(event) => setUiPreferences({ density: event.target.value as typeof uiPreferences.density })}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></label>
                  <label><span>Default precision</span><input value={uiPreferences.defaultPrecision} readOnly /></label>
                  <label className="desktop-checkbox"><input type="checkbox" checked={uiPreferences.showThresholdColors} onChange={(event) => setUiPreferences({ showThresholdColors: event.target.checked })} /> Show threshold colors</label>
                  <label className="desktop-checkbox"><input type="checkbox" checked={uiPreferences.focusDiagramMode} onChange={(event) => setUiPreferences({ focusDiagramMode: event.target.checked })} /> Focus diagram mode</label>
                </div>
              </div>
            ) : null}
            {activeDesktopDialog === "help_shortcuts" ? (
              <div className="desktop-dialog-stack">
                <DesktopDialogNotice icon={<Keyboard size={18} />} title="Desktop shortcuts" detail="QuickPLS keeps primary modeling shortcuts close to SmartPLS-style desktop workflows." />
                <div className="shortcut-grid"><span>Ctrl+Z</span><b>Undo</b><span>Ctrl+Y</span><b>Redo</b><span>P</span><b>Path tool</b><span>C</span><b>Covariance tool</b><span>F</span><b>Fit diagram</b><span>Esc</span><b>Cancel tool or close dialog</b></div>
              </div>
            ) : null}
          </div>
          <footer className="desktop-dialog-footer">
            {activeDesktopDialog === "calculation_setup" ? <button type="button" className="run-button" disabled={!canRun || Boolean(activeJob)} onClick={() => { setActiveDesktopDialog(null); runIfReady(); }}><Play size={14} fill="currentColor" /> Run now</button> : null}
            {activeDesktopDialog === "import_data" ? <button type="button" className="run-button" onClick={() => { setActiveDesktopDialog(null); void importDataCommand().catch((error) => window.alert(error)); }}><Upload size={14} /> Import data</button> : null}
            {activeDesktopDialog === "settings" ? <button type="button" className="secondary-button bordered" onClick={() => { setActiveDesktopDialog(null); goTo("settings", "Open Settings"); }}><Settings size={14} /> Full settings</button> : null}
            {activeDesktopDialog === "method_scope" ? <button type="button" className="secondary-button bordered" onClick={() => { setActiveDesktopDialog(null); goTo("trust", "Open Trust Center"); }}><BookOpen size={14} /> Trust Center</button> : null}
            <button type="button" className="secondary-button bordered" onClick={() => setActiveDesktopDialog(null)}>Close</button>
          </footer>
        </section>
      </div>
    ) : null}
  </>;
}

function DesktopDialogNotice({ icon, title, detail, tone = "info" }: { icon: ReactNode; title: string; detail: string; tone?: "info" | "success" | "warning" }) {
  return <div className={`desktop-dialog-notice ${tone}`}>
    {icon}
    <div><strong>{title}</strong><p>{detail}</p></div>
  </div>;
}
