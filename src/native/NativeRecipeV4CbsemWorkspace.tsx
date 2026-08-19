import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  CircleStop,
  Database,
  Download,
  FileSearch,
  FlaskConical,
  Play,
  RotateCcw,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CanonicalChartPoint,
  CanonicalResultCell,
  CanonicalResultChart,
  CanonicalResultDocumentV2,
} from "../domain/canonicalResultDocumentV2";
import {
  appendInternalLabsRecipeV4CbsemResultV1,
  bindInternalRecipeV4CbsemDatasetV1,
  buildInternalLabsRecipeV4CbsemRequestV1,
  internalRecipeV4CbsemFailureV1,
  monitorInternalLabsRecipeV4CbsemJobV1,
  preflightInternalRecipeV4CbsemWorkspaceV1,
  readStoredInternalLabsRecipeV4CbsemResultsV1,
  reopenInternalLabsRecipeV4CbsemResultV1,
  schema6ArchiveIdentityFromInspectionV1,
  storedExactCaseBootstrapEntriesV1,
  type InternalRecipeV4CbsemEngineOptionsV1,
  type InternalSchema6ArchiveIdentityV1,
} from "../domain/internalRecipeV4CbsemWorkspace";
import type {
  InternalRecipeV4CbsemCompletedResultV1,
  InternalLabsRecipeV4CbsemExecutionRequestV1,
  InternalRecipeV4CbsemJobSnapshotV1,
} from "../domain/internalRecipeV4CbsemExecution";
import type {
  AnalysisRecipeV4MissingDataPolicy,
  InternalRecipeV4ExecutionFailureV1,
} from "../domain/internalRecipeV4PlsExecution";
import type { InternalProjectSchema6ResultAppendOutcomeV1 } from "../domain/internalProjectSchema6ResultAppend";
import type { InternalProjectSchema6CanonicalResultEntryV1 } from "../domain/internalProjectSchema6ResultRead";
import type { ResultTable } from "../domain/resultTables";
import type { ProjectUpgradeInspectionV1, ProjectUpgradeOutcomeV1 } from "../domain/internalProjectUpgradeV6";
import { scientificSemModelV4HashInput } from "../domain/semModelV4";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
} from "../domain/nativeWorkbenchSemModelV4Adapter";
import {
  appendInternalProjectSchema6CanonicalResultV2,
  cancelInternalLabsRecipeV4CbsemJob,
  dismissInternalLabsRecipeV4CbsemJob,
  getInternalLabsRecipeV4CbsemJob,
  getInternalLabsRecipeV4CbsemJobResult,
  getInternalSemModelV4ScientificSha256,
  inspectInternalProjectUpgradeV6,
  readInternalProjectSchema6CanonicalResultsV2,
  exportNativeXlsxTables,
  startInternalLabsRecipeV4CbsemJob,
} from "../services/projectService";
import { useWorkspace } from "../store";
import type { Dataset } from "../types";
import { observedSemanticsForParameterTable } from "./NativeSemParameterTable";

export interface NativeRecipeV4CbsemWorkspaceServices {
  scientificDigest: typeof getInternalSemModelV4ScientificSha256;
  start: typeof startInternalLabsRecipeV4CbsemJob;
  status: typeof getInternalLabsRecipeV4CbsemJob;
  cancel: typeof cancelInternalLabsRecipeV4CbsemJob;
  dismiss: typeof dismissInternalLabsRecipeV4CbsemJob;
  result: typeof getInternalLabsRecipeV4CbsemJobResult;
  append: typeof appendInternalProjectSchema6CanonicalResultV2;
  read: typeof readInternalProjectSchema6CanonicalResultsV2;
  exportXlsx: typeof exportNativeXlsxTables;
  inspect: typeof inspectInternalProjectUpgradeV6;
  selectArchive: () => Promise<string | null>;
}

const defaultServices: NativeRecipeV4CbsemWorkspaceServices = {
  scientificDigest: getInternalSemModelV4ScientificSha256,
  start: startInternalLabsRecipeV4CbsemJob,
  status: getInternalLabsRecipeV4CbsemJob,
  cancel: cancelInternalLabsRecipeV4CbsemJob,
  dismiss: dismissInternalLabsRecipeV4CbsemJob,
  result: getInternalLabsRecipeV4CbsemJobResult,
  append: appendInternalProjectSchema6CanonicalResultV2,
  read: readInternalProjectSchema6CanonicalResultsV2,
  exportXlsx: exportNativeXlsxTables,
  inspect: inspectInternalProjectUpgradeV6,
  selectArchive: async () => {
    const selection = await open({ multiple: false, filters: [{ name: "QuickPLS schema-6 project", extensions: ["qpls", "json"] }] });
    return typeof selection === "string" ? selection : null;
  },
};

export interface NativeRecipeV4CbsemWorkspaceProps {
  modelName: string;
  experimentalLabsEnabled: boolean;
  services?: NativeRecipeV4CbsemWorkspaceServices;
}

interface CapturedJobIdentity {
  projectPath: string;
  datasetId: string;
  datasetFingerprint: string;
  modelScientificInput: string;
}

const ACTIVE_JOB_STATES = new Set<InternalRecipeV4CbsemJobSnapshotV1["state"]>(["queued", "running", "cancelling"]);
const SHA256 = /^[a-f0-9]{64}$/;

export function NativeRecipeV4CbsemWorkspace({
  modelName,
  experimentalLabsEnabled,
  services = defaultServices,
}: NativeRecipeV4CbsemWorkspaceProps) {
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const activeDataset = useWorkspace((state) => state.dataset);
  const residentDatasets = useWorkspace((state) => state.datasetCatalog);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const datasets = residentDatasets.length ? residentDatasets : [activeDataset];
  const [datasetId, setDatasetId] = useState(activeDataset.id);
  const dataset = datasets.find((candidate) => candidate.id === datasetId) ?? null;
  const [missingDataPolicy, setMissingDataPolicy] = useState<AnalysisRecipeV4MissingDataPolicy>("listwise_deletion");
  const effectiveMissingDataPolicy: AnalysisRecipeV4MissingDataPolicy = dataset?.kind === "covariance" || dataset?.kind === "correlation"
    ? "listwise_deletion"
    : missingDataPolicy;
  const [denominator, setDenominator] = useState<"sample_n_minus_one" | "maximum_likelihood_n">("sample_n_minus_one");
  const [correlationScales, setCorrelationScales] = useState<Record<string, string>>({});
  const [engine, setEngine] = useState<InternalRecipeV4CbsemEngineOptionsV1>(() => ({
    tolerance: analysisSettings.tolerance ?? 1e-7,
    maxIterations: analysisSettings.maxIterations ?? 1_000,
    seed: analysisSettings.seed,
    workers: analysisSettings.workers,
    confidenceLevel: analysisSettings.confidenceLevel,
    bootstrapSamples: analysisSettings.cbsemBootstrapSamples ?? 0,
    bootstrapInterval: analysisSettings.cbsemBootstrapInterval ?? "percentile_type7",
    bootstrapTestTail: analysisSettings.cbsemBootstrapTestTail ?? "two_sided",
  }));
  const [snapshot, setSnapshot] = useState<InternalRecipeV4CbsemJobSnapshotV1 | null>(null);
  const [completed, setCompleted] = useState<InternalRecipeV4CbsemCompletedResultV1 | null>(null);
  const [completedRecipe, setCompletedRecipe] = useState<InternalLabsRecipeV4CbsemExecutionRequestV1["recipe"] | null>(null);
  const [failure, setFailure] = useState<InternalRecipeV4ExecutionFailureV1 | null>(null);
  const [busy, setBusy] = useState(false);
  const [archivePath, setArchivePath] = useState("");
  const [archiveIdentity, setArchiveIdentity] = useState<InternalSchema6ArchiveIdentityV1 | null>(null);
  const [archiveFailure, setArchiveFailure] = useState<{ code: string; message: string; correctiveAction: string } | null>(null);
  const [appendOutcome, setAppendOutcome] = useState<InternalProjectSchema6ResultAppendOutcomeV1 | null>(null);
  const [storedEntries, setStoredEntries] = useState<InternalProjectSchema6CanonicalResultEntryV1[]>([]);
  const [reopenedEntry, setReopenedEntry] = useState<InternalProjectSchema6CanonicalResultEntryV1 | null>(null);
  const [archiveBusy, setArchiveBusy] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const [exportFeedback, setExportFeedback] = useState<string | null>(null);
  const startButtonRef = useRef<HTMLButtonElement>(null);
  const browseButtonRef = useRef<HTMLButtonElement>(null);
  const resultHeadingRef = useRef<HTMLHeadingElement>(null);
  const activeJobIdRef = useRef<string | null>(null);
  const monitorAbortRef = useRef<AbortController | null>(null);
  const capturedIdentityRef = useRef<CapturedJobIdentity | null>(null);
  const identityCancellationRequestedRef = useRef(false);

  useEffect(() => {
    if (!dataset && activeDataset.id) setDatasetId(activeDataset.id);
  }, [activeDataset.id, dataset]);

  const indicatorColumns = useMemo(() => [...new Set(nodes.flatMap((node) => node.data.indicators))].sort(), [nodes]);
  const baseInput = useMemo<AuthoredNativeWorkbenchToSemModelV4Input>(() => ({
    model_id: activeModelId ?? "",
    model_name: modelName,
    nodes,
    edges,
    diagram_layout: diagramLayout,
    data_binding: {
      kind: "raw",
      dataset_id: dataset?.id ?? "",
      missing_data: effectiveMissingDataPolicy,
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
    observed_semantics: dataset ? observedSemanticsForParameterTable(dataset, indicatorColumns) : {},
  }), [activeModelId, dataset, diagramLayout, edges, effectiveMissingDataPolicy, indicatorColumns, modelName, nodes]);
  const baseAdapted = useMemo(() => adaptAuthoredNativeWorkbenchToSemModelV4(baseInput), [baseInput]);
  const observed = baseAdapted.ok ? baseAdapted.model.variables.filter((variable) => variable.kind === "observed") : [];
  const parsedScales = useMemo(() => Object.fromEntries(observed.map((variable) => [variable.id, Number(correlationScales[variable.id] ?? "")])), [correlationScales, observed]);
  const boundModel = useMemo(() => {
    if (!dataset || !baseAdapted.ok) return { model: null, error: null as unknown };
    try {
      return {
        model: bindInternalRecipeV4CbsemDatasetV1(baseAdapted.model, dataset, {
          covarianceDenominator: denominator,
          missingDataPolicy: effectiveMissingDataPolicy,
          correlationStandardDeviations: parsedScales,
        }),
        error: null,
      };
    } catch (error) {
      return { model: null, error };
    }
  }, [baseAdapted, dataset, denominator, effectiveMissingDataPolicy, parsedScales]);
  const modelDiagnostics = useMemo(() => {
    if (!baseAdapted.ok) return baseAdapted.diagnostics.map((diagnostic) => ({
      code: diagnostic.code,
      subject: diagnostic.subject ?? "model",
      message: diagnostic.message,
      correctiveAction: diagnostic.corrective_action,
    }));
    if (boundModel.error) {
      const normalized = internalRecipeV4CbsemFailureV1(boundModel.error, "data_resolution");
      return [{ code: normalized.code, subject: normalized.subject, message: normalized.message, correctiveAction: normalized.correctiveAction }];
    }
    return [];
  }, [baseAdapted, boundModel.error]);
  const preflight = useMemo(() => preflightInternalRecipeV4CbsemWorkspaceV1({
    experimentalLabsEnabled,
    projectName,
    projectPath,
    dataset,
    model: boundModel.model,
    modelDiagnostics,
    missingDataPolicy: effectiveMissingDataPolicy,
    engine,
  }), [boundModel.model, dataset, effectiveMissingDataPolicy, engine, experimentalLabsEnabled, modelDiagnostics, projectName, projectPath]);
  const modelScientificInput = useMemo(() => {
    if (!boundModel.model) return "";
    try { return scientificSemModelV4HashInput(boundModel.model); } catch { return ""; }
  }, [boundModel.model]);
  const running = Boolean(snapshot && ACTIVE_JOB_STATES.has(snapshot.state));
  const bootstrapEnabled = (engine.bootstrapSamples ?? 0) > 0;
  const boundedBootstrapInterval = engine.bootstrapInterval === "analytic_studentized_type7"
    || engine.bootstrapInterval === "bca_type7";
  const displayedDocument = reopenedEntry?.canonicalDocument ?? completed?.canonicalDocument ?? null;

  useEffect(() => {
    const captured = capturedIdentityRef.current;
    if (!captured || !running || identityCancellationRequestedRef.current) return;
    const changed = captured.projectPath !== projectPath
      || captured.datasetId !== dataset?.id
      || captured.datasetFingerprint !== dataset?.fingerprint
      || captured.modelScientificInput !== modelScientificInput;
    if (!changed || !activeJobIdRef.current) return;
    identityCancellationRequestedRef.current = true;
    setFailure({
      schemaVersion: 1,
      stage: "integrity",
      subject: "active_project",
      code: "recipe_v4.cbsem.active_identity_changed",
      message: "The active project, dataset, or scientific model changed while the job was running.",
      correctiveAction: "The job is being cancelled. Restore the intended project and start a new job.",
    });
    void services.cancel(activeJobIdRef.current).then(setSnapshot).catch(() => undefined);
  }, [dataset?.fingerprint, dataset?.id, modelScientificInput, projectPath, running, services]);

  useEffect(() => () => {
    monitorAbortRef.current?.abort();
    const jobId = activeJobIdRef.current;
    if (jobId) void services.cancel(jobId).catch(() => undefined);
  }, [services]);

  const start = async () => {
    if (!preflight.ready || !dataset || !boundModel.model || !projectPath) {
      document.getElementById("nd-cbsem-v4-preflight")?.focus();
      return;
    }
    setBusy(true);
    setFailure(null);
    setCompleted(null);
    setCompletedRecipe(null);
    setReopenedEntry(null);
    setAppendOutcome(null);
    setExportFeedback(null);
    identityCancellationRequestedRef.current = false;
    const controller = new AbortController();
    monitorAbortRef.current?.abort();
    monitorAbortRef.current = controller;
    try {
      if (!globalThis.crypto?.randomUUID) throw new Error("Secure recipe identifiers are unavailable in this runtime.");
      const nativeScientificSha256 = await services.scientificDigest(boundModel.model);
      const request = await buildInternalLabsRecipeV4CbsemRequestV1({
        recipeId: globalThis.crypto.randomUUID(),
        createdAt: new Date().toISOString(),
        dataset,
        model: boundModel.model,
        nativeScientificSha256,
        engine,
      });
      capturedIdentityRef.current = {
        projectPath,
        datasetId: dataset.id,
        datasetFingerprint: dataset.fingerprint!,
        modelScientificInput,
      };
      const initial = await services.start(request);
      activeJobIdRef.current = initial.jobId;
      setSnapshot(initial);
      const outcome = await monitorInternalLabsRecipeV4CbsemJobV1({
        initial,
        getStatus: services.status,
        getResult: services.result,
        onSnapshot: setSnapshot,
        signal: controller.signal,
      });
      if (outcome.status === "completed" && !identityCancellationRequestedRef.current) {
        setCompleted(outcome.completed);
        setCompletedRecipe(request.recipe);
        setFailure(null);
        window.setTimeout(() => resultHeadingRef.current?.focus(), 0);
      } else if (outcome.status === "completed") {
        setCompleted(null);
        window.setTimeout(() => startButtonRef.current?.focus(), 0);
      } else if (outcome.status === "terminal_without_result") {
        if (outcome.snapshot.failure) setFailure(outcome.snapshot.failure);
        window.setTimeout(() => startButtonRef.current?.focus(), 0);
      }
    } catch (error) {
      if (!controller.signal.aborted) {
        setFailure(internalRecipeV4CbsemFailureV1(error));
        window.setTimeout(() => startButtonRef.current?.focus(), 0);
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      activeJobIdRef.current = null;
      capturedIdentityRef.current = null;
    }
  };

  const cancel = async () => {
    const jobId = activeJobIdRef.current;
    if (!jobId) return;
    try {
      setSnapshot(await services.cancel(jobId));
    } catch (error) {
      setFailure(internalRecipeV4CbsemFailureV1(error));
    }
  };

  const clearTerminalJob = async () => {
    const jobId = snapshot?.jobId;
    if (jobId && snapshot && !ACTIVE_JOB_STATES.has(snapshot.state) && snapshot.state !== "completed") {
      try { await services.dismiss(jobId); } catch { /* A consumed or expired terminal job is already clear. */ }
    }
    setSnapshot(null);
    setFailure(null);
    setBusy(false);
    window.setTimeout(() => startButtonRef.current?.focus(), 0);
  };

  const chooseArchive = async () => {
    try {
      const selected = await services.selectArchive();
      if (selected) {
        setArchivePath(selected);
        setArchiveIdentity(null);
        setArchiveFailure(null);
        setAppendOutcome(null);
        setStoredEntries([]);
        setReopenedEntry(null);
      }
    } finally {
      window.setTimeout(() => browseButtonRef.current?.focus(), 0);
    }
  };

  const inspectArchive = async () => {
    setArchiveBusy(true);
    setArchiveFailure(null);
    setArchiveIdentity(null);
    try {
      const outcome: ProjectUpgradeOutcomeV1<ProjectUpgradeInspectionV1> = await services.inspect(archivePath.trim());
      if (outcome.status === "blocked") setArchiveFailure(outcome.diagnostic);
      else {
        const identity = schema6ArchiveIdentityFromInspectionV1(outcome.value);
        setArchiveIdentity(identity);
        const stored = await readStoredInternalLabsRecipeV4CbsemResultsV1(identity, services.read);
        if (stored.outcome.status === "blocked") setArchiveFailure(stored.outcome.diagnostic);
        else {
          setStoredEntries(stored.entries);
          setReopenedEntry(stored.entries.at(-1) ?? null);
          if (stored.entries.length) window.setTimeout(() => resultHeadingRef.current?.focus(), 0);
        }
      }
    } catch (error) {
      const normalized = internalRecipeV4CbsemFailureV1(error);
      setArchiveFailure({ code: normalized.code, message: normalized.message, correctiveAction: normalized.correctiveAction });
    } finally {
      setArchiveBusy(false);
    }
  };

  const appendResult = async () => {
    if (!completed || !completedRecipe || !archiveIdentity) return;
    setArchiveBusy(true);
    setArchiveFailure(null);
    try {
      const outcome = await appendInternalLabsRecipeV4CbsemResultV1(completed, completedRecipe, archiveIdentity, services.append);
      setAppendOutcome(outcome);
      if (outcome.status === "blocked") setArchiveFailure(outcome.diagnostic);
      else setArchiveIdentity({ ...archiveIdentity, sourceSha256: outcome.value.updated_document_sha256 });
    } catch (error) {
      const normalized = internalRecipeV4CbsemFailureV1(error);
      setArchiveFailure({ code: normalized.code, message: normalized.message, correctiveAction: normalized.correctiveAction });
    } finally {
      setArchiveBusy(false);
    }
  };

  const reopenResult = async () => {
    if (!completed || !archiveIdentity) return;
    setArchiveBusy(true);
    setArchiveFailure(null);
    try {
      const reopened = await reopenInternalLabsRecipeV4CbsemResultV1(completed, archiveIdentity, services.read);
      if (reopened.outcome.status === "blocked") setArchiveFailure(reopened.outcome.diagnostic);
      else {
        setReopenedEntry(reopened.entry);
        setStoredEntries(storedExactCaseBootstrapEntriesV1(reopened.outcome.value.documents));
        window.setTimeout(() => resultHeadingRef.current?.focus(), 0);
      }
    } catch (error) {
      const normalized = internalRecipeV4CbsemFailureV1(error);
      setArchiveFailure({ code: normalized.code, message: normalized.message, correctiveAction: normalized.correctiveAction });
    } finally {
      setArchiveBusy(false);
    }
  };

  const exportDisplayedResult = async () => {
    if (!displayedDocument || exportBusy) return;
    setExportBusy(true);
    setExportFeedback(null);
    try {
      const path = await services.exportXlsx(canonicalResultDocumentV2ExportTables(displayedDocument));
      setExportFeedback(path ? `Saved ${path}.` : "Export cancelled. No file was created.");
    } catch (error) {
      const normalized = internalRecipeV4CbsemFailureV1(error);
      setExportFeedback(`Export failed: ${normalized.message}`);
    } finally {
      setExportBusy(false);
    }
  };

  const progressMaximum = Math.max(snapshot?.totalUnits ?? 1, 1);
  const progressValue = Math.min(snapshot?.completedUnits ?? 0, progressMaximum);
  const archiveReady = Boolean(archiveIdentity && SHA256.test(archiveIdentity.sourceSha256));

  return <section id="nd-model-cbsem-labs-panel" className="nd-cbsem-v4-workspace" role="tabpanel" aria-labelledby="nd-model-cbsem-labs-tab">
    <header className="nd-cbsem-v4-header">
      <div><h2>Exact CB-SEM workspace</h2></div>
      <FlaskConical size={24} aria-hidden="true" />
    </header>

    <div className="nd-cbsem-v4-grid">
      <section className="nd-cbsem-v4-card" aria-labelledby="nd-cbsem-v4-input-heading">
        <h3 id="nd-cbsem-v4-input-heading">Authoritative input</h3>
        <label>Scientific model<select value={boundModel.model?.id ?? ""} disabled={running || !boundModel.model} aria-describedby="nd-cbsem-v4-model-help"><option value={boundModel.model?.id ?? ""}>{boundModel.model ? `${modelName} (${boundModel.model.id})` : "Complete model decisions first"}</option></select></label>
        <small id="nd-cbsem-v4-model-help">The complete SemModelV4 is embedded in the recipe. The native compilation receipt is shown after completion.</small>
        <label htmlFor="nd-cbsem-v4-dataset">Resident dataset</label><select id="nd-cbsem-v4-dataset" value={datasetId} disabled={running} onChange={(event) => {
          setDatasetId(event.target.value);
          setCorrelationScales({});
          setCompleted(null);
          setReopenedEntry(null);
        }}>{datasets.map((candidate) => <option key={candidate.id} value={candidate.id}>{candidate.name} — {datasetKindLabel(candidate)}</option>)}</select>
        {dataset && dataset.kind !== "covariance" && dataset.kind !== "correlation" ? <>
          <label htmlFor="nd-cbsem-v4-missing-data">Missing-data treatment</label>
          <select
            id="nd-cbsem-v4-missing-data"
            value={missingDataPolicy}
            disabled={running}
            aria-describedby="nd-cbsem-v4-missing-data-help"
            onChange={(event) => {
              setMissingDataPolicy(event.target.value as AnalysisRecipeV4MissingDataPolicy);
              setCompleted(null);
              setReopenedEntry(null);
              setFailure(null);
            }}
          >
            <option value="listwise_deletion">Listwise deletion</option>
            <option value="mean_replacement">Mean replacement (continuous variables only)</option>
          </select>
          <small id="nd-cbsem-v4-missing-data-help">Mean replacement is confined to this CB-SEM job and does not create or overwrite a dataset. Exact bootstrap requires listwise deletion. Rows missing every modeled value are retained and fillable when every modeled variable has an observed finite value. Variable warnings begin at 5%; high warnings appear above 15% for a variable or case. These warnings do not block point estimation.</small>
        </> : <p className="nd-cbsem-v4-summary"><strong>Missing-data treatment</strong><span>Not applied to matrix input</span></p>}
        {dataset?.kind === "covariance" || dataset?.kind === "correlation" ? <>
          <label>Covariance denominator<select value={denominator} disabled={running} onChange={(event) => setDenominator(event.target.value as typeof denominator)}><option value="sample_n_minus_one">Sample covariance (n − 1)</option><option value="maximum_likelihood_n">Maximum-likelihood covariance (n)</option></select></label>
          <p className="nd-cbsem-v4-summary"><strong>Declared sample size</strong><span>{dataset.sampleSize ?? "Missing"}</span></p>
        </> : null}
        {dataset?.kind === "correlation" ? <fieldset className="nd-cbsem-v4-scales"><legend>Correlation scales</legend><p>Enter the study standard deviation for every modeled variable.</p>{observed.map((variable) => <label key={variable.id}>{variable.label}<small>{variable.source_column}</small><input type="number" min="0" step="any" inputMode="decimal" value={correlationScales[variable.id] ?? ""} disabled={running} onChange={(event) => setCorrelationScales((current) => ({ ...current, [variable.id]: event.target.value }))} /></label>)}</fieldset> : null}
        <fieldset className="nd-cbsem-v4-scales">
          <legend>Exact CFA bootstrap</legend>
          <label className="nd-checkbox-row" htmlFor="nd-cbsem-v4-bootstrap-enabled">
            <input
              id="nd-cbsem-v4-bootstrap-enabled"
              type="checkbox"
              checked={bootstrapEnabled}
              disabled={running}
              onChange={(event) => {
                const enabled = event.target.checked;
                setMissingDataPolicy(enabled ? "listwise_deletion" : missingDataPolicy);
                setEngine((current) => ({
                  ...current,
                  bootstrapSamples: enabled ? Math.max(current.bootstrapSamples ?? 1_000, 1_000) : 0,
                  bootstrapInterval: current.bootstrapInterval ?? "percentile_type7",
                  bootstrapTestTail: enabled ? (current.bootstrapTestTail ?? "two_sided") : "two_sided",
                  confidenceLevel: enabled ? 0.95 : current.confidenceLevel,
                }));
                setCompleted(null);
                setReopenedEntry(null);
              }}
            />
            Full-ML indexed case bootstrap
          </label>
          {bootstrapEnabled ? <>
            <label htmlFor="nd-cbsem-v4-bootstrap-samples">Requested refits
              <input
                id="nd-cbsem-v4-bootstrap-samples"
                type="number"
                min={500}
                max={10_000}
                step={100}
                value={engine.bootstrapSamples ?? 1_000}
                disabled={running}
                onChange={(event) => setEngine((current) => ({ ...current, bootstrapSamples: Number(event.target.value) }))}
              />
            </label>
            <label htmlFor="nd-cbsem-v4-bootstrap-interval">Interval
              <select
                id="nd-cbsem-v4-bootstrap-interval"
                value={engine.bootstrapInterval ?? "percentile_type7"}
                disabled={running}
                onChange={(event) => {
                  const interval = event.target.value as NonNullable<InternalRecipeV4CbsemEngineOptionsV1["bootstrapInterval"]>;
                  setEngine((current) => ({
                    ...current,
                    bootstrapInterval: interval,
                    bootstrapTestTail: interval === "percentile_type7" ? (current.bootstrapTestTail ?? "two_sided") : "two_sided",
                    workers: interval === "percentile_type7" ? current.workers : Math.min(current.workers, 12),
                  }));
                }}
              >
                <option value="percentile_type7">Percentile Type-7</option>
                <option value="analytic_studentized_type7">Analytic studentized Type-7</option>
                <option value="bca_type7">BCa Type-7 (complete delete-one)</option>
              </select>
            </label>
            <label htmlFor="nd-cbsem-v4-bootstrap-tail">Zero-null test
              <select
                id="nd-cbsem-v4-bootstrap-tail"
                value={engine.bootstrapTestTail ?? "two_sided"}
                disabled={running || boundedBootstrapInterval}
                onChange={(event) => setEngine((current) => ({
                  ...current,
                  bootstrapTestTail: event.target.value as NonNullable<InternalRecipeV4CbsemEngineOptionsV1["bootstrapTestTail"]>,
                }))}
              >
                <option value="two_sided">Two-sided</option>
                <option value="one_sided_greater">One-sided greater</option>
                <option value="one_sided_less">One-sided less</option>
              </select>
            </label>
          </> : null}
        </fieldset>
        <details><summary>Engine settings</summary><div className="nd-cbsem-v4-engine-grid">
          <label>Tolerance<input type="number" min="0.000000000001" max="0.01" step="any" value={engine.tolerance} disabled={running} onChange={(event) => setEngine((current) => ({ ...current, tolerance: Number(event.target.value) }))} /></label>
          <label>Maximum iterations<input type="number" min="100" max="100000" value={engine.maxIterations} disabled={running} onChange={(event) => setEngine((current) => ({ ...current, maxIterations: Number(event.target.value) }))} /></label>
          <label>Workers<input type="number" min="1" max="64" value={engine.workers} disabled={running} onChange={(event) => setEngine((current) => ({ ...current, workers: Number(event.target.value) }))} /></label>
          <label>Seed<input type="number" min="0" max="4294967295" value={engine.seed} disabled={running} onChange={(event) => setEngine((current) => ({ ...current, seed: Number(event.target.value) }))} /></label>
        </div></details>
      </section>

      <section id="nd-cbsem-v4-preflight" className="nd-cbsem-v4-card" aria-labelledby="nd-cbsem-v4-preflight-heading" tabIndex={-1}>
        <h3 id="nd-cbsem-v4-preflight-heading">Layered preflight</h3>
        {effectiveMissingDataPolicy === "mean_replacement" ? <div className="nd-inline-warning" role="status" aria-live="polite">
          <AlertTriangle size={16} aria-hidden="true" />
          <div><strong>{preflight.warnings.length ? `${preflight.warnings.length} non-blocking missing-data warning${preflight.warnings.length === 1 ? "" : "s"}` : "No variable-level threshold warning in the resident summary"}</strong>
            <p>Native execution recomputes exact variable and case diagnostics and records them in the treatment receipt.</p>
            {preflight.warnings.length ? <ul>{preflight.warnings.map((warning) => <li key={`${warning.code}:${warning.subject}`}><b>{warning.severity === "high" ? "High warning" : "Warning"}:</b> {warning.message}</li>)}</ul> : null}
          </div>
        </div> : null}
        <ol className="nd-cbsem-v4-preflight-list">{preflight.layers.map((layer) => <li key={layer.stage} className={layer.status}><span>{layer.status === "ready" ? <CheckCircle2 size={15} aria-hidden="true" /> : <AlertTriangle size={15} aria-hidden="true" />}</span><div><strong>{layer.label}</strong><small>{layer.status === "ready" ? layer.warnings.length ? `Ready with ${layer.warnings.length} warning${layer.warnings.length === 1 ? "" : "s"}` : "Ready" : `${layer.issues.length} action${layer.issues.length === 1 ? "" : "s"} needed`}</small>{layer.issues.map((diagnostic) => <p key={`${diagnostic.code}:${diagnostic.subject}`}><b>{diagnostic.message}</b> {diagnostic.correctiveAction}</p>)}</div></li>)}</ol>
        <div className="nd-cbsem-v4-actions"><button ref={startButtonRef} type="button" className="primary" disabled={!preflight.ready || busy || running} onClick={() => void start()}><Play size={14} aria-hidden="true" />Start native job</button>{running ? <button type="button" className="danger" onClick={() => void cancel()}><CircleStop size={14} aria-hidden="true" />Cancel</button> : snapshot && snapshot.state !== "completed" ? <button type="button" onClick={() => void clearTerminalJob()}><RotateCcw size={14} aria-hidden="true" />Clear</button> : null}</div>
      </section>
    </div>

    {snapshot ? <section className="nd-cbsem-v4-card nd-cbsem-v4-monitor" aria-labelledby="nd-cbsem-v4-monitor-heading" aria-live="polite">
      <div><h3 id="nd-cbsem-v4-monitor-heading">Native job</h3><span className={`nd-cbsem-v4-state ${snapshot.state}`}>{humanToken(snapshot.state)}</span></div>
      <progress aria-label="CB-SEM Recipe-v4 job progress" max={progressMaximum} value={progressValue} />
      <p>{snapshot.message ?? humanToken(snapshot.phase)} · {progressValue} of {progressMaximum}</p>
      <small>Job {snapshot.jobId}</small>
    </section> : null}

    {failure ? <section className="nd-cbsem-v4-failure" role="alert"><AlertTriangle size={18} aria-hidden="true" /><div><strong>{failure.message}</strong><p>{failure.correctiveAction}</p><small>{failure.stage} · {failure.code} · {failure.subject}</small></div></section> : null}

    <section className="nd-cbsem-v4-card nd-cbsem-v4-archive" aria-labelledby="nd-cbsem-v4-archive-heading">
      <h3 id="nd-cbsem-v4-archive-heading"><Archive size={17} aria-hidden="true" />Schema-6 attachment</h3>
      <div className="nd-cbsem-v4-archive-path"><label>Schema-6 archive path<input value={archivePath} disabled={archiveBusy} onChange={(event) => { setArchivePath(event.target.value); setArchiveIdentity(null); setAppendOutcome(null); setStoredEntries([]); setReopenedEntry(null); }} /></label><button ref={browseButtonRef} type="button" disabled={archiveBusy} onClick={() => void chooseArchive()}>Browse…</button><button type="button" disabled={archiveBusy || !archivePath.trim()} onClick={() => void inspectArchive()}><FileSearch size={14} aria-hidden="true" />Inspect</button></div>
      {archiveIdentity ? <dl className="nd-cbsem-v4-receipt"><div><dt>Project</dt><dd>{archiveIdentity.projectId}</dd></div><div><dt>Source digest</dt><dd>{archiveIdentity.sourceSha256}</dd></div></dl> : null}
      {archiveIdentity ? <label className="nd-cbsem-v4-stored-results">Stored exact result<select id="nd-cbsem-v4-stored-results" value={reopenedEntry?.documentId ?? ""} disabled={archiveBusy || storedEntries.length === 0} onChange={(event) => {
        const selected = storedEntries.find((entry) => entry.documentId === event.target.value) ?? null;
        setReopenedEntry(selected);
        if (selected) window.setTimeout(() => resultHeadingRef.current?.focus(), 0);
      }}><option value="">{storedEntries.length ? "Select a stored result" : "No stored exact-bootstrap result"}</option>{storedEntries.map((entry) => <option key={entry.documentId} value={entry.documentId}>{entry.documentId} · run {entry.runId}</option>)}</select></label> : null}
      <div className="nd-cbsem-v4-actions"><button type="button" className="primary" disabled={archiveBusy || !archiveReady || !completed || !completedRecipe} onClick={() => void appendResult()}>Append exact native document</button><button type="button" disabled={archiveBusy || !archiveReady || !completed} onClick={() => void reopenResult()}>Reopen and verify completed run</button></div>
      {appendOutcome?.status === "ok" ? <p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Appended document {appendOutcome.value.canonical_document_id}; {appendOutcome.value.canonical_result_document_count} canonical result document{appendOutcome.value.canonical_result_document_count === 1 ? "" : "s"} in the archive.</p> : null}
      {reopenedEntry ? <p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Reopened immutable document {reopenedEntry.documentId} from schema 6.</p> : null}
      {archiveFailure ? <div className="nd-cbsem-v4-failure" role="alert"><AlertTriangle size={16} aria-hidden="true" /><div><strong>{archiveFailure.message}</strong><p>{archiveFailure.correctiveAction}</p><small>{archiveFailure.code}</small></div></div> : null}
    </section>

    {displayedDocument ? <>
      <div className="nd-cbsem-v4-actions nd-cbsem-v4-export-actions">
        <button id="nd-cbsem-v4-export-xlsx" type="button" disabled={exportBusy} onClick={() => void exportDisplayedResult()}><Download size={14} aria-hidden="true" />{exportBusy ? "Exporting…" : "Export selected exact run to XLSX"}</button>
        {exportFeedback ? <p role="status">{exportFeedback}</p> : null}
      </div>
      <CanonicalResultDocumentV2View document={displayedDocument} reopened={Boolean(reopenedEntry)} headingRef={resultHeadingRef} compilationReceipt={completed?.analyticalResult.provenance.compilation_receipt ?? null} />
    </> : null}
  </section>;
}

export function canonicalResultDocumentV2ExportTables(document: CanonicalResultDocumentV2): ResultTable[] {
  const noticesByTable = new Map<string, string[]>();
  for (const notice of document.notices) {
    for (const tableId of notice.table_ids) {
      const messages = noticesByTable.get(tableId) ?? [];
      messages.push(notice.message);
      noticesByTable.set(tableId, messages);
    }
  }
  const precision = document.presentation.precision;
  const missing = document.presentation.missing_value_label;
  const resultTables = document.tables.map<ResultTable>((table) => ({
    id: table.id,
    title: table.title,
    status: "validated",
    warning: noticesByTable.get(table.id)?.join(" ") ?? null,
    columns: table.columns.map((column) => column.label),
    rows: table.rows.map((row) => row.cells.map((cell, index) => canonicalCellText(
      cell,
      table.columns[index]?.default_precision ?? precision,
      missing,
    ))),
  }));
  const provenance = document.provenance;
  resultTables.push({
    id: "canonical_run_provenance",
    title: "Canonical run provenance",
    status: "validated",
    warning: "Exported from the selected immutable CanonicalResultDocumentV2.",
    columns: ["Field", "Value"],
    rows: [
      ["Document ID", document.document_id],
      ["Run ID", provenance.run_id],
      ["Project ID", provenance.project_id],
      ["Model ID", provenance.model_id],
      ["Model digest", provenance.model_digest],
      ["Dataset ID", provenance.dataset_id],
      ["Dataset fingerprint", provenance.dataset_fingerprint],
      ["Recipe ID", provenance.recipe_id],
      ["Recipe digest", provenance.recipe_digest],
      ["Capability ID", provenance.capability_cell.capability_id],
      ["Capability cell", provenance.capability_cell.cell_id],
      ["Capability version", provenance.capability_cell.capability_version],
      ["Method version", provenance.method_version],
      ["Engine version", provenance.engine_version],
      ["Seed", provenance.seed === null ? missing : String(provenance.seed)],
      ["Workers", String(provenance.workers)],
      ["Started at", provenance.started_at],
      ["Completed at", provenance.completed_at],
    ],
  });
  resultTables.push({
    id: "canonical_result_notes",
    title: "Canonical result notes",
    status: "validated",
    warning: null,
    columns: ["Kind", "ID", "Scope", "Text"],
    rows: [
      ...document.tables.flatMap((table) => table.description
        ? [["Table description", table.id, table.id, table.description]]
        : []),
      ...document.notices.map((notice) => [
        `Notice (${notice.severity})`,
        notice.id,
        [...notice.section_ids, ...notice.table_ids].join(", "),
        notice.message,
      ]),
      ...document.exclusions.map((exclusion) => [
        "Exclusion",
        exclusion.id,
        exclusion.capability_cell?.cell_id ?? "method",
        `${exclusion.title}: ${exclusion.reason}`,
      ]),
      ...document.footnotes.map((footnote) => [
        "Footnote",
        footnote.id,
        "document",
        footnote.reference ? `${footnote.text} (${footnote.reference})` : footnote.text,
      ]),
    ],
  });
  return resultTables;
}

export function CanonicalResultDocumentV2View({
  document,
  reopened,
  headingRef,
  compilationReceipt,
}: {
  document: CanonicalResultDocumentV2;
  reopened: boolean;
  headingRef?: React.RefObject<HTMLHeadingElement | null>;
  compilationReceipt?: InternalRecipeV4CbsemCompletedResultV1["analyticalResult"]["provenance"]["compilation_receipt"] | null;
}) {
  const tables = new Map(document.tables.map((table) => [table.id, table]));
  const charts = new Map(document.charts.map((chart) => [chart.id, chart]));
  return <section className="nd-cbsem-v4-results" aria-labelledby="nd-cbsem-v4-results-heading">
    <header><div><h2 id="nd-cbsem-v4-results-heading" ref={headingRef} tabIndex={-1}>{document.title}</h2><p>{reopened ? "Reopened immutable schema-6 document" : "Native CanonicalResultDocumentV2"}</p></div><Database size={22} aria-hidden="true" /></header>
    {document.notices.length ? <div className="nd-cbsem-v4-notices">{document.notices.map((notice) => <p key={notice.id} role={notice.severity === "error" ? "alert" : "note"}><strong>{humanToken(notice.severity)}</strong> {notice.message}</p>)}</div> : null}
    {document.sections.map((section) => <section key={section.id} aria-labelledby={`nd-cbsem-section-${canonicalDomToken(section.id)}`}>
      <h3 id={`nd-cbsem-section-${canonicalDomToken(section.id)}`}>{section.title}</h3>
      {section.description ? <p>{section.description}</p> : null}
      {section.table_ids.map((tableId) => {
        const table = tables.get(tableId);
        if (!table) return null;
        return <div
          className="nd-cbsem-v4-table-wrap"
          data-canonical-table-id={table.id}
          id={`nd-canonical-table-${canonicalDomToken(table.id)}`}
          key={table.id}
        ><table><caption><strong>{table.title}</strong>{table.description ? <span>{table.description}</span> : null}</caption><thead><tr>{table.columns.map((column) => <th key={column.id} scope="col" title={column.description}>{column.label}</th>)}</tr></thead><tbody>{table.rows.map((row) => <tr key={row.id}>{row.cells.map((cell, index) => <td key={`${row.id}:${table.columns[index]?.id ?? index}`}>{canonicalCellText(cell, table.columns[index]?.default_precision ?? document.presentation.precision, document.presentation.missing_value_label)}</td>)}</tr>)}</tbody></table></div>;
      })}
      {section.chart_ids.map((chartId) => {
        const chart = charts.get(chartId);
        if (!chart) return null;
        return <CanonicalResultChartView chart={chart} sourceTableTitle={chart.source_table_id ? tables.get(chart.source_table_id)?.title : undefined} key={chart.id} />;
      })}
    </section>)}
    <details className="nd-cbsem-v4-run-details"><summary>Run and compilation details</summary><dl><div><dt>Run</dt><dd>{document.provenance.run_id}</dd></div><div><dt>Project</dt><dd>{document.provenance.project_id}</dd></div><div><dt>Model</dt><dd>{document.provenance.model_id}</dd></div><div><dt>Dataset</dt><dd>{document.provenance.dataset_id}</dd></div><div><dt>Method</dt><dd>{document.provenance.method_version}</dd></div>{compilationReceipt ? <><div><dt>Compiler</dt><dd>{compilationReceipt.compiler_version}</dd></div><div><dt>Plan digest</dt><dd>{compilationReceipt.plan_sha256}</dd></div><div><dt>Scientific model digest</dt><dd>{compilationReceipt.model_scientific_sha256}</dd></div></> : null}</dl></details>
  </section>;
}

const CANONICAL_CHART_WIDTH = 640;
const CANONICAL_CHART_HEIGHT = 300;
const CANONICAL_CHART_MARGIN = { top: 18, right: 20, bottom: 48, left: 58 } as const;
const CANONICAL_CHART_DASHES = ["", "10 5", "3 4", "12 4 3 4", "2 3 8 3"] as const;
const CANONICAL_CHART_LINE_NAMES = ["solid", "long dashed", "dotted", "dash-dot", "dot-dash"] as const;

function CanonicalResultChartView({
  chart,
  sourceTableTitle,
}: {
  chart: CanonicalResultChart;
  sourceTableTitle?: string;
}) {
  const domId = `nd-canonical-chart-${canonicalDomToken(chart.id)}`;
  const sourceTableHref = chart.source_table_id
    ? `#nd-canonical-table-${canonicalDomToken(chart.source_table_id)}`
    : null;
  const pointCount = chart.series.reduce((total, series) => total + series.points.length, 0);
  const summary = `${humanToken(chart.kind)} chart with ${chart.series.length} series and ${pointCount} persisted ${pointCount === 1 ? "point" : "points"}. Exact values remain available in the canonical source table.`;
  const supported = chart.kind === "line" || chart.kind === "scatter" || chart.kind === "interval";
  const projection = supported ? projectCanonicalChart(chart) : null;

  return <figure
    className="nd-canonical-chart"
    data-canonical-chart-id={chart.id}
    aria-labelledby={`${domId}-title`}
    aria-describedby={`${domId}-description ${domId}-summary`}
  >
    <figcaption>
      <h4 id={`${domId}-title`}>{chart.title}</h4>
      {chart.description ? <p id={`${domId}-description`}>{chart.description}</p> : <span id={`${domId}-description`} className="nd-sr-only">Canonical result chart.</span>}
    </figcaption>
    <p id={`${domId}-summary`} className="nd-canonical-chart__summary">{summary}</p>
    {projection ? <div className="nd-canonical-chart__plot-wrap">
      <svg
        className="nd-canonical-chart__plot"
        viewBox={`0 0 ${CANONICAL_CHART_WIDTH} ${CANONICAL_CHART_HEIGHT}`}
        role="img"
        aria-label={`${chart.title}. ${summary}`}
      >
        {projection.yTicks.map((tick) => <g key={`y:${tick.value}`} aria-hidden="true">
          <line className="nd-canonical-chart__grid" x1={CANONICAL_CHART_MARGIN.left} x2={CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right} y1={tick.coordinate} y2={tick.coordinate} />
          <text className="nd-canonical-chart__tick" x={CANONICAL_CHART_MARGIN.left - 8} y={tick.coordinate + 4} textAnchor="end">{tick.label}</text>
        </g>)}
        <line className="nd-canonical-chart__axis" x1={CANONICAL_CHART_MARGIN.left} x2={CANONICAL_CHART_MARGIN.left} y1={CANONICAL_CHART_MARGIN.top} y2={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom} aria-hidden="true" />
        <line className="nd-canonical-chart__axis" x1={CANONICAL_CHART_MARGIN.left} x2={CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right} y1={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom} y2={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom} aria-hidden="true" />
        {projection.xTicks.map((tick) => <g key={`x:${String(tick.value)}`} aria-hidden="true">
          <line className="nd-canonical-chart__axis-tick" x1={tick.coordinate} x2={tick.coordinate} y1={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom} y2={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom + 5} />
          <text className="nd-canonical-chart__tick" x={tick.coordinate} y={CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom + 19} textAnchor="middle">{tick.label}</text>
        </g>)}
        {chart.display.x_axis_label ? <text className="nd-canonical-chart__axis-label" x={(CANONICAL_CHART_MARGIN.left + CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right) / 2} y={CANONICAL_CHART_HEIGHT - 8} textAnchor="middle">{chart.display.x_axis_label}</text> : null}
        {chart.display.y_axis_label ? <text className="nd-canonical-chart__axis-label" x={15} y={CANONICAL_CHART_HEIGHT / 2} textAnchor="middle" transform={`rotate(-90 15 ${CANONICAL_CHART_HEIGHT / 2})`}>{chart.display.y_axis_label}</text> : null}
        {chart.series.map((series, seriesIndex) => {
          const projectedPoints = series.points.map((point) => ({
            point,
            x: projection.x(point.x),
            y: projection.y(point.y),
          }));
          const dash = CANONICAL_CHART_DASHES[seriesIndex % CANONICAL_CHART_DASHES.length];
          const seriesClass = `nd-canonical-chart__series nd-canonical-chart__series--${seriesIndex % CANONICAL_CHART_DASHES.length}`;
          const path = projectedPoints.map(({ x, y }, index) => `${index === 0 ? "M" : "L"} ${x} ${y}`).join(" ");
          return <g className={seriesClass} key={series.id}>
            {chart.kind !== "scatter" && projectedPoints.length > 1 ? <path className="nd-canonical-chart__line" d={path} strokeDasharray={dash || undefined} aria-hidden="true" /> : null}
            {projectedPoints.map(({ point, x, y }, pointIndex) => {
              const lower = typeof point.lower === "number" ? projection.y(point.lower) : null;
              const upper = typeof point.upper === "number" ? projection.y(point.upper) : null;
              const pointText = canonicalChartPointText(series.label, point);
              return <g key={`${series.id}:${pointIndex}`}>
                {lower !== null || upper !== null ? <line className="nd-canonical-chart__interval" x1={x} x2={x} y1={lower ?? y} y2={upper ?? y} aria-hidden="true" /> : null}
                <circle className="nd-canonical-chart__point" cx={x} cy={y} r={4}><title>{pointText}</title></circle>
                {chart.display.show_values ? <text className="nd-canonical-chart__value" x={x} y={y - 8} textAnchor="middle">{formatCanonicalChartNumber(point.y)}</text> : null}
              </g>;
            })}
          </g>;
        })}
      </svg>
    </div> : <p className="nd-canonical-chart__fallback" role="note">QuickPLS preserves this {humanToken(chart.kind).toLowerCase()} chart and its exact source data, but this chart kind does not yet have a visual renderer. Use the canonical source table in this section.</p>}
    {chart.display.show_legend !== false && chart.series.length ? <ul className="nd-canonical-chart__legend" aria-label={`${chart.title} series key`}>
      {chart.series.map((series, index) => <li key={series.id} className={`nd-canonical-chart__series--${index % CANONICAL_CHART_DASHES.length}`}>
        <svg viewBox="0 0 34 10" width="34" height="10" aria-hidden="true"><line className="nd-canonical-chart__legend-line" x1="1" x2="33" y1="5" y2="5" strokeDasharray={CANONICAL_CHART_DASHES[index % CANONICAL_CHART_DASHES.length] || undefined} /></svg>
        <span>{series.label} ({CANONICAL_CHART_LINE_NAMES[index % CANONICAL_CHART_LINE_NAMES.length]})</span>
      </li>)}
    </ul> : null}
    {sourceTableHref ? <p className="nd-canonical-chart__source"><a href={sourceTableHref}>Exact plot data: {sourceTableTitle ?? chart.source_table_id}</a></p> : null}
  </figure>;
}

function projectCanonicalChart(chart: CanonicalResultChart) {
  const points = chart.series.flatMap((series) => series.points);
  if (!points.length) return null;
  const yValues = points.flatMap((point) => [point.y, point.lower, point.upper]).filter((value): value is number => typeof value === "number" && Number.isFinite(value));
  if (!yValues.length) return null;
  const yExtent = paddedCanonicalChartExtent(Math.min(...yValues), Math.max(...yValues));
  const y = (value: number) => scaleCanonicalChartValue(value, yExtent[0], yExtent[1], CANONICAL_CHART_HEIGHT - CANONICAL_CHART_MARGIN.bottom, CANONICAL_CHART_MARGIN.top);
  const yTicks = Array.from({ length: 5 }, (_, index) => {
    const value = yExtent[0] + ((yExtent[1] - yExtent[0]) * index) / 4;
    return { value, label: formatCanonicalChartNumber(value), coordinate: y(value) };
  });

  const allNumericX = points.every((point) => typeof point.x === "number" && Number.isFinite(point.x));
  if (allNumericX) {
    const xValues = points.map((point) => point.x as number);
    const xExtent = paddedCanonicalChartExtent(Math.min(...xValues), Math.max(...xValues), false);
    const x = (value: number | string) => scaleCanonicalChartValue(Number(value), xExtent[0], xExtent[1], CANONICAL_CHART_MARGIN.left, CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right);
    const uniqueValues = [...new Set(xValues)].sort((left, right) => left - right);
    const tickValues = uniqueValues.length <= 7
      ? uniqueValues
      : Array.from({ length: 5 }, (_, index) => xExtent[0] + ((xExtent[1] - xExtent[0]) * index) / 4);
    return {
      x,
      y,
      xTicks: tickValues.map((value) => ({ value, label: formatCanonicalChartNumber(value), coordinate: x(value) })),
      yTicks,
    };
  }

  const categories: string[] = [];
  for (const point of points) {
    const value = String(point.x);
    if (!categories.includes(value)) categories.push(value);
  }
  const x = (value: number | string) => {
    const index = Math.max(0, categories.indexOf(String(value)));
    if (categories.length === 1) return (CANONICAL_CHART_MARGIN.left + CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right) / 2;
    return scaleCanonicalChartValue(index, 0, categories.length - 1, CANONICAL_CHART_MARGIN.left, CANONICAL_CHART_WIDTH - CANONICAL_CHART_MARGIN.right);
  };
  return {
    x,
    y,
    xTicks: categories.map((value) => ({ value, label: value, coordinate: x(value) })),
    yTicks,
  };
}

function paddedCanonicalChartExtent(minimum: number, maximum: number, addPadding = true): readonly [number, number] {
  if (minimum === maximum) {
    const radius = Math.max(1, Math.abs(minimum) * 0.1);
    return [minimum - radius, maximum + radius];
  }
  if (!addPadding) return [minimum, maximum];
  const padding = (maximum - minimum) * 0.08;
  return [minimum - padding, maximum + padding];
}

function scaleCanonicalChartValue(value: number, sourceMin: number, sourceMax: number, targetMin: number, targetMax: number): number {
  return targetMin + ((value - sourceMin) / (sourceMax - sourceMin)) * (targetMax - targetMin);
}

function canonicalChartPointText(seriesLabel: string, point: CanonicalChartPoint): string {
  const interval = typeof point.lower === "number" || typeof point.upper === "number"
    ? `, interval ${typeof point.lower === "number" ? formatCanonicalChartNumber(point.lower) : "not reported"} to ${typeof point.upper === "number" ? formatCanonicalChartNumber(point.upper) : "not reported"}`
    : "";
  const label = point.label ? `${point.label}, ` : "";
  return `${seriesLabel}: ${label}x ${String(point.x)}, y ${formatCanonicalChartNumber(point.y)}${interval}`;
}

function formatCanonicalChartNumber(value: number): string {
  if (Object.is(value, -0)) return "0";
  if (value === 0) return "0";
  const absolute = Math.abs(value);
  if (absolute >= 1_000_000 || absolute < 0.0001) return value.toExponential(3);
  return value.toFixed(4).replace(/\.?0+$/u, "");
}

function canonicalDomToken(value: string): string {
  return Array.from(new TextEncoder().encode(value), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function canonicalCellText(cell: CanonicalResultCell, precision: number | null, missingLabel: string): string {
  if (cell.kind === "number") return cell.display ?? cell.value.toFixed(Math.max(0, Math.min(12, precision ?? 3)));
  if (cell.kind === "boolean") return cell.value ? "Yes" : "No";
  if (cell.kind === "text") return cell.value;
  return cell.display ?? missingLabel;
}

function datasetKindLabel(dataset: Dataset): string {
  if (dataset.kind === "covariance") return "Covariance matrix";
  if (dataset.kind === "correlation") return "Scaled correlation matrix";
  return "Raw observations";
}

function humanToken(value: string): string {
  const spaced = value.replaceAll("_", " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
