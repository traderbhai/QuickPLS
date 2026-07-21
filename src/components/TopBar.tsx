import { Download, FlaskConical, FolderOpen, Menu, Play, Plus, RotateCcw, Save, Square, Upload } from "lucide-react";
import Papa from "papaparse";
import { useEffect, useRef, useState } from "react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { effectiveMethodStatus, isSelectableAnalysisMethod, methodStatusDescription, methodStatusLabel } from "../domain/methodStatus";
import { useWorkspace } from "../store";
import type { AnalysisMethodId, Dataset, JobSnapshot } from "../types";
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
  const setProjectMeta = useWorkspace((state) => state.setProjectMeta);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const setAnalysisSettings = useWorkspace((state) => state.setAnalysisSettings);
  const runnableMethods = methods.filter(isSelectableAnalysisMethod);
  const selectedMethod = runnableMethods.find((candidate) => candidate.id === analysisSettings.method) ?? runnableMethods[0];

  const download = (name: string, contents: string, type: string) => {
    const url = URL.createObjectURL(new Blob([contents], { type }));
    const anchor = document.createElement("a");
    anchor.href = url; anchor.download = name; anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };

  const saveProject = async () => {
    if (!isNativeDesktop()) { download("corporate-reputation.qpls.json", JSON.stringify({ schemaVersion: 1, nodes, edges, dataset, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout }, null, 2), "application/json"); return; }
    const saved = await saveNativeProject(projectPath, { nodes, edges, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout, activeDatasetId: dataset.id });
    if (saved) setProjectMeta(saved.name, saved.path);
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
  const newProjectCommand = async () => { resetProject(); if (isNativeDesktop()) await createNativeProject(); };
  const importDataCommand = async () => { if (!isNativeDesktop()) { inputRef.current?.click(); return; } const value = await importNativeDataset(); if (value) setDataset(value); };
  const exportSummary = () => download("quickpls-foundation-summary.html", `<!doctype html><meta charset="utf-8"><title>QuickPLS foundation summary</title><h1>QuickPLS foundation summary</h1><p>Dataset: ${dataset.name}</p><p>Rows: ${dataset.rows.length}; constructs: ${nodes.length}; paths: ${edges.length}</p><p><strong>Stable scope:</strong> supported analyses are validated for the documented v1.0.0 scope after a saved run is selected.</p>`, "text/html");

  const importCsv = (file?: File) => {
    if (!file) return;
    Papa.parse<Record<string, string | number | null>>(file, {
      header: true,
      dynamicTyping: true,
      skipEmptyLines: true,
      complete: ({ data, meta }) => {
        const missing = data.reduce((count, row) => count + Object.values(row).filter((value) => value === null || value === "").length, 0);
        setDataset({ id: crypto.randomUUID(), name: file.name, columns: meta.fields ?? [], rows: data, missing });
      },
    });
  };

  const readiness = analysisReadiness({ dataset, nodes, edges, settings: analysisSettings, nativeDesktop: isNativeDesktop() });
  const canRun = readiness.canRun;
  const runAnalysis = async () => {
    if (!dataset.fingerprint) throw new Error("Import and save a dataset before running an analysis.");
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
    while (!["completed", "failed", "cancelled"].includes(job.state)) {
      await new Promise((resolve) => window.setTimeout(resolve, 150));
      job = await getNativePlsJob(job.id);
      setActiveJob(job);
    }
    if (job.state === "cancelled") { await dismissNativePlsJob(job.id); setActiveJob(null); return; }
    if (job.state === "failed") { const message = job.message ?? "PLS analysis failed"; await dismissNativePlsJob(job.id); setActiveJob(null); throw new Error(message); }
    const envelope = await getNativePlsJobResult(job.id);
    setActiveJob(null);
    if (!envelope) throw new Error("Completed PLS job did not return a result");
    if (envelope.payload.kind === "legacy") throw new Error("The completed job returned an incompatible result payload");
    const { estimation: result, assessment } = envelope.payload;
    const bootstrap = envelope.payload.kind === "pls_pm_v2" ? envelope.payload.bootstrap : envelope.payload.kind === "pls_pm_v3" ? envelope.payload.bootstrap ?? undefined : undefined;
    const permutation = envelope.payload.kind === "pls_pm_v3" ? envelope.payload.permutation ?? undefined : undefined;
    addRun({ id: envelope.id, name: `${selectedMethod.name} run`, method: selectedMethod.name, createdAt: envelope.provenance.completed_at, seed: envelope.provenance.seed, status: "completed", warnings: ["Validated for the documented QuickPLS v1.0.0 supported scope; unsupported shapes remain blocked or explicitly marked.", ...envelope.diagnostics.filter((item) => item.level === "warning").map((item) => item.message)], fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12), result, assessment, bootstrap, permutation });
  };
  const cancelAnalysis = async () => {
    if (!activeJob) return;
    setActiveJob(await cancelNativePlsJob(activeJob.id));
  };
  useEffect(() => {
    const handleRunRequest = () => {
      if (activeJob || !canRun) return;
      void runAnalysis().catch((error) => { setActiveJob(null); window.alert(error); });
    };
    window.addEventListener("quickpls:run-analysis", handleRunRequest);
    return () => window.removeEventListener("quickpls:run-analysis", handleRunRequest);
  }, [activeJob, canRun, runAnalysis]);

  return <>
    <header className="title-bar">
      <Menu size={20} /><strong>QuickPLS</strong><span className="project-title">{projectName}.qpls</span>
      <span className="alpha-mark">v1.0.0</span>
    </header>
    <div className="command-bar">
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
      <div className="method-picker" title={selectedMethod ? methodStatusDescription(selectedMethod, analysisSettings) : undefined}>
        <select className="method-select" aria-label="Analysis method" value={analysisSettings.method} onChange={(event) => setAnalysisSettings({ method: event.target.value as AnalysisMethodId })}>
          {runnableMethods.map((candidate) => <option key={candidate.id} value={candidate.id}>{candidate.name}</option>)}
          <option disabled>GSCA (planned)</option>
        </select>
        {selectedMethod ? <span className={`status-text ${effectiveMethodStatus(selectedMethod, analysisSettings)}`}>{methodStatusLabel(effectiveMethodStatus(selectedMethod, analysisSettings))}</span> : null}
      </div>
      <button className="run-button" aria-label={activeJob ? "Cancel active analysis" : `Run ${selectedMethod.name}`} aria-describedby={!activeJob && !canRun ? "run-disabled-reason" : undefined} disabled={!activeJob && !canRun} title={activeJob ? "Cancel the active analysis" : canRun ? `Run ${selectedMethod.name}` : readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => { void (activeJob ? cancelAnalysis() : runAnalysis()).catch((error) => { setActiveJob(null); window.alert(error); }); }}>
        {activeJob ? <Square size={14} fill="currentColor" /> : <Play size={16} fill="currentColor" />}
        <span className="run-button-label">{activeJob ? `${activeJob.phase} ${activeJob.completed_units}/${activeJob.total_units}` : `Run ${selectedMethod.name}`}</span>
      </button>
      {!activeJob && !canRun ? <span id="run-disabled-reason" className="command-disabled-reason">{readiness.blockers[0]?.detail ?? readiness.summary}</span> : null}
    </div>
  </>;
}
