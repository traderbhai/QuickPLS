import { save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  CircleStop,
  Download,
  FlaskConical,
  FolderOpen,
  Play,
  RotateCcw,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  appendGeneralSemResultV1,
  bindGeneralSemPlsModelToDatasetV1,
  buildGeneralSemRecipeV1,
  defaultGeneralSemPlsEngineOptionsV1,
  generalSemConfigFromEngineV1,
  generalSemFailureV1,
  generalSemJobRequestFromReceiptV1,
  monitorGeneralSemPlsJobV1,
  preflightGeneralSemWorkspaceV1,
  rehydrateGeneralSemExecutionAuthorityV1,
  reopenGeneralSemResultV1,
  selectGeneralSemPlsExecutionCapabilityV1,
  validateGeneralSemPlsCompletedExecutionV1,
  type GeneralSemPlsCompletedResultV1,
  type GeneralSemPlsEngineOptionsV1,
  type GeneralSemPlsJobFailureV1,
  type GeneralSemPlsJobSnapshotV1,
  type GeneralSemPlsMonitorOutcomeV1,
  type GeneralSemProjectBootstrapReceiptV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { SemCapabilityDecisionV1 } from "../domain/semCapabilityDecisionV1";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import type { InternalProjectSchema6CanonicalResultEntryV1 } from "../domain/internalProjectSchema6ResultRead";
import type { InternalProjectSchema6ResultAppendOutcomeV1 } from "../domain/internalProjectSchema6ResultAppend";
import { scientificSemModelV4HashInput } from "../domain/semModelV4";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
} from "../domain/nativeWorkbenchSemModelV4Adapter";
import { useInternalProjectArchiveV6Session } from "../internalProjectArchiveV6SessionStore";
import { inspectInternalProjectArchiveV6At } from "../services/internalProjectArchiveV6ReadService";
import {
  appendInternalProjectSchema6CanonicalResultV2,
  bootstrapInternalGeneralSemProjectArchiveV6,
  cancelInternalLabsGeneralSemPlsJobV1,
  dismissInternalLabsGeneralSemPlsJobV1,
  exportNativeXlsxTables,
  getInternalLabsGeneralSemPlsJobResultV1,
  getInternalLabsGeneralSemPlsJobV1,
  getInternalSemModelV4ScientificSha256,
  invalidateNativeGeneralSemFreshDraftAuthorityV1,
  preflightInternalGeneralSemEstimatorsV1,
  readInternalProjectSchema6CanonicalResultsV2,
  startInternalLabsGeneralSemPlsJobV1,
} from "../services/projectService";
import { useWorkspace } from "../store";
import { GeneralSemEstimatorCompatibilityPanel } from "./GeneralSemEstimatorCompatibilityPanel";
import { observedSemanticsForParameterTable } from "./NativeSemParameterTable";
import {
  CanonicalResultDocumentV2View,
  canonicalResultDocumentV2ExportTables,
} from "./NativeRecipeV4CbsemWorkspace";

export interface NativeRecipeV4GeneralSemWorkspaceServices {
  scientificDigest: typeof getInternalSemModelV4ScientificSha256;
  bootstrapArchive: typeof bootstrapInternalGeneralSemProjectArchiveV6;
  inspectArchive: typeof inspectInternalProjectArchiveV6At;
  nativePreflight: typeof preflightInternalGeneralSemEstimatorsV1;
  start: typeof startInternalLabsGeneralSemPlsJobV1;
  status: typeof getInternalLabsGeneralSemPlsJobV1;
  cancel: typeof cancelInternalLabsGeneralSemPlsJobV1;
  dismiss: typeof dismissInternalLabsGeneralSemPlsJobV1;
  result: typeof getInternalLabsGeneralSemPlsJobResultV1;
  append: typeof appendInternalProjectSchema6CanonicalResultV2;
  read: typeof readInternalProjectSchema6CanonicalResultsV2;
  invalidateDraft: typeof invalidateNativeGeneralSemFreshDraftAuthorityV1;
  exportXlsx: typeof exportNativeXlsxTables;
  selectDestination: (suggestedName: string) => Promise<string | null>;
}

const defaultServices: NativeRecipeV4GeneralSemWorkspaceServices = {
  scientificDigest: getInternalSemModelV4ScientificSha256,
  bootstrapArchive: bootstrapInternalGeneralSemProjectArchiveV6,
  inspectArchive: inspectInternalProjectArchiveV6At,
  nativePreflight: preflightInternalGeneralSemEstimatorsV1,
  start: startInternalLabsGeneralSemPlsJobV1,
  status: getInternalLabsGeneralSemPlsJobV1,
  cancel: cancelInternalLabsGeneralSemPlsJobV1,
  dismiss: dismissInternalLabsGeneralSemPlsJobV1,
  result: getInternalLabsGeneralSemPlsJobResultV1,
  append: appendInternalProjectSchema6CanonicalResultV2,
  read: readInternalProjectSchema6CanonicalResultsV2,
  invalidateDraft: invalidateNativeGeneralSemFreshDraftAuthorityV1,
  exportXlsx: exportNativeXlsxTables,
  selectDestination: async (suggestedName) => {
    const selected = await save({
      defaultPath: suggestedName,
      filters: [{ name: "QuickPLS General SEM project", extensions: ["qpls"] }],
    });
    return typeof selected === "string" ? selected : null;
  },
};

export interface NativeRecipeV4GeneralSemWorkspaceProps {
  modelName: string;
  experimentalLabsEnabled: boolean;
  /**
   * Fail-closed feature seam. The primary desktop opts in only when the fresh
   * draft -> bootstrap -> strict schema-6 activation bridge is installed.
   */
  projectActivationConnected?: boolean;
  services?: NativeRecipeV4GeneralSemWorkspaceServices;
}

const ACTIVE_STATES = new Set<GeneralSemPlsJobSnapshotV1["state"]>(["queued", "running", "cancelling"]);

export function generalSemCompletionMatchesLatestAuthorityV1(
  capturedAuthorityKey: string | null,
  latestAuthorityKey: string,
): boolean {
  return capturedAuthorityKey !== null && capturedAuthorityKey === latestAuthorityKey;
}

export function selectGeneralSemDisplayedDocumentV1<T>(
  reopenedDocument: T | null,
  completedDocument: T | null,
  resultIntegrityInvalid: boolean,
  authorityCurrent = true,
): T | null {
  if (resultIntegrityInvalid || !authorityCurrent) return null;
  return reopenedDocument ?? completedDocument;
}

export interface GeneralSemNativePlsPreflightAuthorityV1 {
  readonly authorityKey: string;
  readonly decision: SemCapabilityDecisionV1;
}

export function selectCurrentGeneralSemNativePlsDecisionV1(
  preflight: GeneralSemNativePlsPreflightAuthorityV1 | null,
  authorityKey: string,
): SemCapabilityDecisionV1 | null {
  return preflight?.authorityKey === authorityKey ? preflight.decision : null;
}

export interface GeneralSemCanonicalModerationInventoryV1 {
  readonly interactionEffectCount: number;
  readonly conditionalSlopeCount: number;
  readonly interactionPlotCount: number;
  readonly interactionPlotPointCount: number;
}

export function generalSemCanonicalModerationInventoryV1(
  document: CanonicalResultDocumentV2 | null,
): GeneralSemCanonicalModerationInventoryV1 | null {
  const results = document?.general_sem_results;
  const interactionEffectCount = results?.interaction_effects?.length ?? 0;
  if (interactionEffectCount === 0) return null;
  return {
    interactionEffectCount,
    conditionalSlopeCount: results?.conditional_effects?.length ?? 0,
    interactionPlotCount: results?.interaction_plots?.length ?? 0,
    interactionPlotPointCount: results?.interaction_plots?.reduce((plotTotal, plot) => (
      plotTotal + plot.series.reduce((seriesTotal, series) => seriesTotal + series.points.length, 0)
    ), 0) ?? 0,
  };
}

export function selectLatestGeneralSemReopenedEntryV1(
  entries: readonly InternalProjectSchema6CanonicalResultEntryV1[],
  receipt: GeneralSemProjectBootstrapReceiptV1,
): InternalProjectSchema6CanonicalResultEntryV1 | null {
  const eligible = entries.filter((entry) => {
    const provenance = entry.canonicalDocument.provenance;
    return Boolean(entry.canonicalDocument.general_sem_results)
      && provenance.project_id === receipt.projectId
      && provenance.dataset_id === receipt.residentDatasetId
      && provenance.dataset_fingerprint === receipt.residentDatasetFingerprint
      && provenance.model_id === receipt.residentModelId
      && provenance.model_digest === receipt.residentModelScientificSha256
      && provenance.recipe_id === receipt.residentRecipeId;
  });
  return eligible.reduce<InternalProjectSchema6CanonicalResultEntryV1 | null>((latest, entry) => {
    if (!latest) return entry;
    const entryEpoch = Date.parse(entry.canonicalDocument.provenance.completed_at);
    const latestEpoch = Date.parse(latest.canonicalDocument.provenance.completed_at);
    if (entryEpoch !== latestEpoch) return entryEpoch > latestEpoch ? entry : latest;
    const entryKey = `${entry.canonicalDocument.provenance.completed_at}\0${entry.documentId}`;
    const latestKey = `${latest.canonicalDocument.provenance.completed_at}\0${latest.documentId}`;
    return entryKey > latestKey ? entry : latest;
  }, null);
}

export function generalSemPersistenceNextActionV1(
  appendSucceeded: boolean,
  resultIntegrityInvalid: boolean,
): "append" | "verify_reanchor" | "strict_readback" {
  if (!appendSucceeded) return "append";
  return resultIntegrityInvalid ? "verify_reanchor" : "strict_readback";
}

export function generalSemResultCanAppendV1(input: {
  completed: boolean;
  authorityCurrent: boolean;
  sessionDirty: boolean;
  operationPending: boolean;
  appendSucceeded: boolean;
  resultIntegrityInvalid: boolean;
}): boolean {
  return input.completed
    && input.authorityCurrent
    && !input.sessionDirty
    && !input.operationPending
    && generalSemPersistenceNextActionV1(input.appendSucceeded, input.resultIntegrityInvalid) === "append";
}

export function generalSemTemporaryResultBlocksCloseV1(input: {
  completed: boolean;
  appendSucceeded: boolean;
  reopened: boolean;
  resultIntegrityInvalid: boolean;
}): boolean {
  return input.completed
    && (!input.appendSucceeded || !input.reopened || input.resultIntegrityInvalid);
}

export function generalSemStartedJobRetentionV1(input: {
  started: boolean;
  terminalKnown: boolean;
  activeJobId: string | null;
}): "retain" | "release" {
  return input.started && !input.terminalKnown && Boolean(input.activeJobId)
    ? "retain"
    : "release";
}

export interface GeneralSemProjectCloseBridgeV1 {
  close: () => "closed" | "blocked" | "inactive";
  readFailure: () => ReturnType<typeof useInternalProjectArchiveV6Session.getState>["standardActivationFailure"];
}

export function closeGeneralSemProjectV1(bridge: GeneralSemProjectCloseBridgeV1):
  | { status: "closed" }
  | { status: "blocked"; failure: GeneralSemPlsJobFailureV1 } {
  const result = bridge.close();
  if (result === "closed") return { status: "closed" };
  const diagnostic = bridge.readFailure();
  return {
    status: "blocked",
    failure: {
      schemaVersion: 1,
      stage: "archive_authority",
      subject: "project",
      code: diagnostic?.code ?? "general_sem.project_close.inactive",
      message: diagnostic?.message ?? "No active General SEM project could be closed.",
      correctiveAction: diagnostic?.correctiveAction ?? "Keep the current workspace unchanged and reopen the marked project before retrying.",
      issues: [],
    },
  };
}

export interface GeneralSemProjectActivationBridgeV1 {
  openSnapshot: (snapshot: InternalProjectArchiveV6ReadSnapshotV1) => Promise<string>;
  activateStandardAuthorities: () => Promise<string>;
  rollbackActivation: () => void;
  readSession: () => ReturnType<typeof useInternalProjectArchiveV6Session.getState>["session"];
  readWorkspace: () => Pick<ReturnType<typeof useWorkspace.getState>,
    "activeModelId" | "standardSemModelV4Authorities" | "standardSemModelV4Persistence"
    | "setProjectMeta" | "clearGeneralSemProjectDraftMode">;
}

/** Opens and activates only the exact marked archive created by bootstrap. */
export async function activateGeneralSemProjectArchiveV1(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  receipt: GeneralSemProjectBootstrapReceiptV1,
  bridge: GeneralSemProjectActivationBridgeV1,
): Promise<void> {
  let openedHere = false;
  try {
    const opened = await bridge.openSnapshot(snapshot);
    if (opened !== "activated") throw new Error(`The saved General SEM archive could not enter the schema-6 session (${opened}).`);
    openedHere = true;
    const activated = await bridge.activateStandardAuthorities();
    if (activated !== "activated") throw new Error(`The saved General SEM authority could not become the active QuickPLS canvas authority (${activated}).`);

    const session = bridge.readSession();
    const workspace = bridge.readWorkspace();
    const modelId = receipt.residentModelId;
    if (!session?.standardActivation
      || !supportsGeneralSemV1(session.project)
      || session.project.project_id !== receipt.projectId
      || session.snapshot.archiveSha256 !== receipt.destinationArchiveSha256
      || !session.standardActivation.modelIds.includes(modelId)
      || workspace.activeModelId !== modelId
      || !workspace.standardSemModelV4Authorities[modelId]
      || workspace.standardSemModelV4Persistence[modelId]?.scientificSha256 !== receipt.residentModelScientificSha256) {
      throw new Error("The newly saved General SEM archive did not activate as the exact marked model and digest authority.");
    }

    workspace.setProjectMeta(receipt.name, receipt.destinationArchivePath, receipt.projectId);
    workspace.clearGeneralSemProjectDraftMode();
  } catch (error) {
    if (openedHere) bridge.rollbackActivation();
    throw error;
  }
}

function generalSemAuthorityKeyV1(input: {
  sourceProjectId: string | null;
  datasetId: string;
  datasetFingerprint: string | undefined;
  modelScientificInput: string;
  config: unknown;
  engine: GeneralSemPlsEngineOptionsV1;
}): string {
  return JSON.stringify(input);
}

function safeFileStem(value: string): string {
  const stem = value.trim().replace(/[<>:"/\\|?*\u0000-\u001f]+/gu, "-").replace(/[ .]+$/u, "");
  return stem || "QuickPLS-General-SEM";
}

function currentGeneralSemDraftPublicationKeyV1(fallbackModelName: string): string {
  const state = useWorkspace.getState();
  const activeName = state.projectModels.find((candidate) => candidate.id === state.activeModelId)?.name
    ?? fallbackModelName;
  const indicators = [...new Set(state.nodes.flatMap((node) => node.data.indicators))].sort();
  const adapted = adaptAuthoredNativeWorkbenchToSemModelV4({
    model_id: state.activeModelId ?? "",
    model_name: activeName,
    nodes: state.nodes,
    edges: state.edges,
    diagram_layout: state.diagramLayout,
    data_binding: {
      kind: "raw",
      dataset_id: state.dataset.id,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
    observed_semantics: observedSemanticsForParameterTable(state.dataset, indicators),
  });
  if (!adapted.ok) throw new Error("The fresh General SEM canvas no longer has a valid scientific model authority.");
  const bound = bindGeneralSemPlsModelToDatasetV1(adapted.model, state.dataset);
  return JSON.stringify({
    projectId: state.projectId,
    projectPath: state.projectPath,
    draft: state.generalSemProjectDraftMode,
    datasetId: state.dataset.id,
    datasetFingerprint: state.dataset.fingerprint,
    activeModelId: state.activeModelId,
    modelScientificInput: scientificSemModelV4HashInput(bound),
    diagramLayout: state.diagramLayout,
  });
}

export function NativeRecipeV4GeneralSemWorkspace({
  modelName,
  experimentalLabsEnabled,
  projectActivationConnected = false,
  services = defaultServices,
}: NativeRecipeV4GeneralSemWorkspaceProps) {
  const workspaceProjectId = useWorkspace((state) => state.projectId);
  const projectName = useWorkspace((state) => state.projectName);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const generalSemProjectDraftMode = useWorkspace((state) => state.generalSemProjectDraftMode);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const generalSemTransientWorkBlocker = useWorkspace((state) => state.generalSemTransientWorkBlocker);
  const setGeneralSemPublicationPending = useWorkspace((state) => state.setGeneralSemPublicationPending);
  const setGeneralSemTransientWorkBlocker = useWorkspace((state) => state.setGeneralSemTransientWorkBlocker);
  const clearGeneralSemProjectDraftMode = useWorkspace((state) => state.clearGeneralSemProjectDraftMode);
  const pushToast = useWorkspace((state) => state.pushToast);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const dataset = useWorkspace((state) => state.dataset);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const generalSemSession = useInternalProjectArchiveV6Session((state) => state.session);
  const generalSemSessionDirty = useInternalProjectArchiveV6Session((state) => state.dirty);
  const markedGeneralSemProjectMode = Boolean(
    generalSemSession?.standardActivation
    && supportsGeneralSemV1(generalSemSession.project)
    && activeModelId
    && generalSemSession.standardActivation.modelIds.includes(activeModelId),
  );
  const freshGeneralSemDraftMode = Boolean(
    projectActivationConnected
    && generalSemProjectDraftMode
    && workspaceProjectId === generalSemProjectDraftMode.sourceProjectId,
  );
  const sourceProjectId = markedGeneralSemProjectMode
    ? generalSemSession?.project.project_id ?? null
    : workspaceProjectId;
  const rehydratedExecution = useMemo(() => {
    if (!markedGeneralSemProjectMode || !generalSemSession) return null;
    try {
      return { status: "ok" as const, value: rehydrateGeneralSemExecutionAuthorityV1(generalSemSession.snapshot) };
    } catch (error) {
      return { status: "blocked" as const, failure: generalSemFailureV1(error) };
    }
  }, [generalSemSession, markedGeneralSemProjectMode]);
  const [engine, setEngine] = useState<GeneralSemPlsEngineOptionsV1>(() => ({
    ...(rehydratedExecution?.status === "ok" ? rehydratedExecution.value.engine : {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      tolerance: analysisSettings.tolerance ?? 1e-7,
      maxIterations: analysisSettings.maxIterations ?? 1_000,
      seed: analysisSettings.seed,
      workers: analysisSettings.workers,
      confidenceLevel: analysisSettings.confidenceLevel,
      bootstrapSamples: Math.max(analysisSettings.bootstrapSamples ?? 500, 2),
    }),
  }));
  const effectiveEngine = rehydratedExecution?.status === "ok"
    ? rehydratedExecution.value.engine
    : engine;
  const [receipt, setReceipt] = useState<GeneralSemProjectBootstrapReceiptV1 | null>(
    rehydratedExecution?.status === "ok" ? rehydratedExecution.value.receipt : null,
  );
  const [archiveSnapshot, setArchiveSnapshot] = useState<InternalProjectArchiveV6ReadSnapshotV1 | null>(
    markedGeneralSemProjectMode ? generalSemSession?.snapshot ?? null : null,
  );
  const [nativePlsPreflight, setNativePlsPreflight] = useState<GeneralSemNativePlsPreflightAuthorityV1 | null>(null);
  const [snapshot, setSnapshot] = useState<GeneralSemPlsJobSnapshotV1 | null>(null);
  const [completed, setCompleted] = useState<GeneralSemPlsCompletedResultV1 | null>(null);
  const [failure, setFailure] = useState<GeneralSemPlsJobFailureV1 | null>(null);
  const [appendOutcome, setAppendOutcome] = useState<InternalProjectSchema6ResultAppendOutcomeV1 | null>(null);
  const [currentArchiveSha256, setCurrentArchiveSha256] = useState<string | null>(
    markedGeneralSemProjectMode ? generalSemSession?.snapshot?.archiveSha256 ?? null : null,
  );
  const [persistedArchiveSha256, setPersistedArchiveSha256] = useState<string | null>(null);
  const [reopenedEntry, setReopenedEntry] = useState<InternalProjectSchema6CanonicalResultEntryV1 | null>(null);
  const [resultIntegrityInvalid, setResultIntegrityInvalid] = useState(false);
  const [jobRecoveryRequired, setJobRecoveryRequired] = useState(false);
  const [busy, setBusy] = useState(false);
  const [exportFeedback, setExportFeedback] = useState<string | null>(null);
  const activeJobIdRef = useRef<string | null>(null);
  const monitorAbortRef = useRef<AbortController | null>(null);
  const capturedAuthorityKeyRef = useRef<string | null>(null);
  const latestAuthorityKeyRef = useRef<string | null>(null);
  const resultHeadingRef = useRef<HTMLHeadingElement>(null);
  const createButtonRef = useRef<HTMLButtonElement>(null);

  const indicatorColumns = useMemo(
    () => [...new Set(nodes.flatMap((node) => node.data.indicators))].sort(),
    [nodes],
  );
  const draftAuthoringInput = useMemo<AuthoredNativeWorkbenchToSemModelV4Input>(() => ({
    model_id: activeModelId ?? "",
    model_name: modelName,
    nodes,
    edges,
    diagram_layout: diagramLayout,
    data_binding: {
      kind: "raw",
      dataset_id: dataset.id,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
    observed_semantics: observedSemanticsForParameterTable(dataset, indicatorColumns),
  }), [activeModelId, dataset, diagramLayout, edges, indicatorColumns, modelName, nodes]);
  const adaptedDraft = useMemo(
    () => freshGeneralSemDraftMode ? adaptAuthoredNativeWorkbenchToSemModelV4(draftAuthoringInput) : null,
    [draftAuthoringInput, freshGeneralSemDraftMode],
  );
  // Adaptation is allowed only inside the identity-bound fresh draft. Every
  // ordinary or previously opened project remains ineligible.
  const model = useMemo(() => {
    if (!projectActivationConnected) return null;
    if (markedGeneralSemProjectMode) return strictAuthority?.model ?? null;
    if (!freshGeneralSemDraftMode || !adaptedDraft?.ok) return null;
    try { return bindGeneralSemPlsModelToDatasetV1(adaptedDraft.model, dataset); } catch { return null; }
  }, [adaptedDraft, dataset, freshGeneralSemDraftMode, markedGeneralSemProjectMode, projectActivationConnected, strictAuthority]);
  const config = useMemo(() => {
    if (markedGeneralSemProjectMode) {
      return rehydratedExecution?.status === "ok" ? rehydratedExecution.value.config : null;
    }
    try { return generalSemConfigFromEngineV1(effectiveEngine); } catch { return null; }
  }, [effectiveEngine, markedGeneralSemProjectMode, rehydratedExecution]);
  const localPreflight = useMemo(() => !projectActivationConnected ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.primary_activation_pending",
      subject: "project",
      message: "Primary General SEM project activation is not connected in this build.",
      correctiveAction: "Keep this capability disabled until the complete new-project bootstrap and strict activation bridge is installed.",
    }],
  } : !freshGeneralSemDraftMode && !markedGeneralSemProjectMode ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.required",
      subject: "project",
      message: "The open canvas is neither a fresh General SEM draft nor an activated general_sem_v1 authority.",
      correctiveAction: "Create a new General SEM project from QuickPLS New Project. Existing projects are never copied, adapted, or relabelled.",
    }],
  } : markedGeneralSemProjectMode && rehydratedExecution?.status === "blocked" ? {
    ready: false,
    decision: null,
    issues: [{
      code: rehydratedExecution.failure.code,
      subject: rehydratedExecution.failure.subject,
      message: rehydratedExecution.failure.message,
      correctiveAction: rehydratedExecution.failure.correctiveAction,
    }],
  } : markedGeneralSemProjectMode && !strictAuthority ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.strict_authority_required",
      subject: "model",
      message: "The General SEM project has no active strict SemModelV4 authority.",
      correctiveAction: "Activate a ready or draft SemModelV4 authority from the marked schema-6 project before calculating.",
    }],
  } : config ? preflightGeneralSemWorkspaceV1({
    experimentalLabsEnabled,
    sourceProjectId,
    dataset,
    model,
    config,
    engine: effectiveEngine,
  }) : {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.config.invalid",
      subject: "config",
      message: "The General SEM configuration is invalid.",
      correctiveAction: "Correct the bounded inference and output settings.",
    }],
  }, [config, dataset, effectiveEngine, experimentalLabsEnabled, freshGeneralSemDraftMode, markedGeneralSemProjectMode, model, projectActivationConnected, rehydratedExecution, sourceProjectId, strictAuthority]);
  const modelScientificInput = useMemo(() => {
    if (!model) return "";
    try { return scientificSemModelV4HashInput(model); } catch { return ""; }
  }, [model]);
  const authorityKey = useMemo(() => generalSemAuthorityKeyV1({
    sourceProjectId,
    datasetId: dataset.id,
    datasetFingerprint: dataset.fingerprint,
    modelScientificInput,
    config,
    engine: effectiveEngine,
  }), [config, dataset.fingerprint, dataset.id, effectiveEngine, modelScientificInput, sourceProjectId]);
  latestAuthorityKeyRef.current = authorityKey;
  const nativePlsDecision = selectCurrentGeneralSemNativePlsDecisionV1(
    nativePlsPreflight,
    authorityKey,
  );
  const nativePlsExecution = useMemo(() => {
    if (!nativePlsDecision || !model || !config) return null;
    try {
      return selectGeneralSemPlsExecutionCapabilityV1({
        model,
        config,
        decision: nativePlsDecision,
      });
    } catch {
      return null;
    }
  }, [config, model, nativePlsDecision]);
  const nativePreflightReady = nativePlsExecution !== null;
  const archiveCurrent = Boolean(receipt && currentArchiveSha256 && capturedAuthorityKeyRef.current === authorityKey);
  const resultAuthorityCurrent = Boolean(
    archiveCurrent
    && markedGeneralSemProjectMode
    && generalSemSession
    && receipt
    && currentArchiveSha256 === generalSemSession.snapshot.archiveSha256
    && receipt.destinationArchivePath === generalSemSession.snapshot.archivePath
    && receipt.projectId === generalSemSession.project.project_id,
  );
  const running = Boolean(snapshot && ACTIVE_STATES.has(snapshot.state));
  const displayedDocument = selectGeneralSemDisplayedDocumentV1(
    reopenedEntry?.canonicalDocument ?? null,
    completed?.canonicalDocument ?? null,
    resultIntegrityInvalid,
    !completed && !reopenedEntry ? true : resultAuthorityCurrent,
  );
  const moderationInventory = generalSemCanonicalModerationInventoryV1(displayedDocument);
  const unpersistedCompletedResult = generalSemTemporaryResultBlocksCloseV1({
    completed: Boolean(completed),
    appendSucceeded: appendOutcome?.status === "ok",
    reopened: Boolean(reopenedEntry),
    resultIntegrityInvalid,
  });
  const operationBusy = busy || generalSemPublicationPending;

  useEffect(() => {
    if (!markedGeneralSemProjectMode || !generalSemSession) {
      setNativePlsPreflight(null);
      return;
    }
    if (rehydratedExecution?.status !== "ok" || !model || !config) {
      setNativePlsPreflight(null);
      if (rehydratedExecution?.status === "blocked") setFailure(rehydratedExecution.failure);
      return;
    }
    const rehydrated = rehydratedExecution.value;
    setReceipt(rehydrated.receipt);
    setArchiveSnapshot(generalSemSession.snapshot);
    setCurrentArchiveSha256(generalSemSession.snapshot.archiveSha256);
    capturedAuthorityKeyRef.current = authorityKey;
    latestAuthorityKeyRef.current = authorityKey;
    setNativePlsPreflight(null);
    const requestedAuthorityKey = authorityKey;
    let live = true;
    void services.nativePreflight({
      project: generalSemSession.project,
      model,
      config,
    }).then((outcome) => {
      if (!live || latestAuthorityKeyRef.current !== requestedAuthorityKey) return;
      if (outcome.status === "ok") {
        try {
          selectGeneralSemPlsExecutionCapabilityV1({
            model,
            config,
            decision: outcome.value.pls,
          });
          setNativePlsPreflight({
            authorityKey: requestedAuthorityKey,
            decision: outcome.value.pls,
          });
        } catch (error) {
          setNativePlsPreflight(null);
          setFailure(generalSemFailureV1(error));
        }
        return;
      }
      setNativePlsPreflight(null);
      setFailure({ schemaVersion: 1, stage: "capability", subject: "preflight", ...outcome.diagnostic, issues: [] });
    }).catch((error) => {
      if (live && latestAuthorityKeyRef.current === requestedAuthorityKey) {
        setNativePlsPreflight(null);
        setFailure(generalSemFailureV1(error));
      }
    });
    return () => { live = false; };
  }, [authorityKey, config, generalSemSession, markedGeneralSemProjectMode, model, rehydratedExecution, services]);

  useEffect(() => {
    if (!markedGeneralSemProjectMode
      || !generalSemSession
      || rehydratedExecution?.status !== "ok") return;
    let live = true;
    const currentReceipt = rehydratedExecution.value.receipt;
    void services.read({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      archivePath: generalSemSession.snapshot.archivePath,
      expectedSourceSha256: generalSemSession.snapshot.archiveSha256,
    }).then((outcome) => {
      if (!live) return;
      if (outcome.status === "blocked") {
        setResultIntegrityInvalid(true);
        setReopenedEntry(null);
        setFailure({ schemaVersion: 1, stage: "archive_authority", subject: "archive", ...outcome.diagnostic, issues: [] });
        return;
      }
      if (outcome.value.projectId !== currentReceipt.projectId
        || outcome.value.sourceDocumentSha256 !== generalSemSession.snapshot.archiveSha256) {
        setResultIntegrityInvalid(true);
        setReopenedEntry(null);
        setFailure({
          schemaVersion: 1,
          stage: "integrity",
          subject: "archive",
          code: "general_sem.rehydrate.result_archive_mismatch",
          message: "The strict result readback differs from the active General SEM archive authority.",
          correctiveAction: "Preserve the project unchanged and do not display or export its results.",
          issues: [],
        });
        return;
      }
      const latest = selectLatestGeneralSemReopenedEntryV1(outcome.value.documents, currentReceipt);
      setReopenedEntry(latest);
      setPersistedArchiveSha256(generalSemSession.snapshot.archiveSha256);
      setResultIntegrityInvalid(false);
    }).catch((error) => {
      if (!live) return;
      setResultIntegrityInvalid(true);
      setReopenedEntry(null);
      setFailure(generalSemFailureV1(error));
    });
    return () => { live = false; };
  }, [generalSemSession, markedGeneralSemProjectMode, rehydratedExecution, services]);

  useEffect(() => {
    if (generalSemPublicationPending || (!completed && !reopenedEntry) || resultAuthorityCurrent) return;
    setResultIntegrityInvalid(true);
    setReopenedEntry(null);
    setFailure({
      schemaVersion: 1,
      stage: "integrity",
      subject: "active_authority",
      code: "general_sem.completed_authority_stale",
      message: "The displayed result no longer belongs to the active General SEM archive authority.",
      correctiveAction: "The stale result was hidden. Reopen the exact marked project or calculate again from its current verified authority.",
      issues: [],
    });
  }, [completed, generalSemPublicationPending, reopenedEntry, resultAuthorityCurrent]);

  useEffect(() => {
    if (!running || capturedAuthorityKeyRef.current === authorityKey || !activeJobIdRef.current) return;
    setFailure({
      schemaVersion: 1,
      stage: "integrity",
      subject: "active_authority",
      code: "general_sem.active_authority_changed",
      message: "The active project, dataset, model, or General SEM configuration changed while estimation was running.",
      correctiveAction: "The job is being cancelled. Save a new calculation project from the intended canvas model and dataset.",
      issues: [],
    });
    void services.cancel(activeJobIdRef.current).then(setSnapshot).catch(() => undefined);
  }, [authorityKey, running, services]);

  useEffect(() => () => {
    monitorAbortRef.current?.abort();
    const activeJobId = activeJobIdRef.current;
    if (activeJobId) {
      void services.cancel(activeJobId)
        .then((cancelled) => {
          if (!ACTIVE_STATES.has(cancelled.state)) {
            useWorkspace.getState().setGeneralSemTransientWorkBlocker(null);
          }
        })
        .catch(() => undefined);
    }
  }, [services]);

  const clearResults = () => {
    setCompleted(null);
    setAppendOutcome(null);
    setPersistedArchiveSha256(null);
    setReopenedEntry(null);
    setResultIntegrityInvalid(false);
    setExportFeedback(null);
    setJobRecoveryRequired(false);
    setGeneralSemTransientWorkBlocker(null);
  };

  const createCalculationProject = async () => {
    if (generalSemPublicationPending || !projectActivationConnected || !freshGeneralSemDraftMode || !localPreflight.ready || !model || !config || !sourceProjectId || !dataset.fingerprint) {
      document.getElementById("nd-general-sem-preflight")?.focus();
      return;
    }
    const draftPublicationKey = currentGeneralSemDraftPublicationKeyV1(modelName);
    const assertDraftPublicationCurrent = () => {
      const current = useWorkspace.getState();
      if (!current.generalSemPublicationPending
        || current.projectId !== sourceProjectId
        || current.projectPath !== null
        || current.generalSemProjectDraftMode?.sourceProjectId !== sourceProjectId
        || currentGeneralSemDraftPublicationKeyV1(modelName) !== draftPublicationKey) {
        throw new Error("The fresh General SEM project authority changed while its marked archive was being published.");
      }
    };
    let publishedReceipt: GeneralSemProjectBootstrapReceiptV1 | null = null;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    clearResults();
    try {
      if (!globalThis.crypto?.randomUUID) throw new Error("Secure project and recipe identifiers are unavailable in this runtime.");
      const destination = await services.selectDestination(`${safeFileStem(projectName)}-General-SEM.qpls`);
      if (!destination) return;
      assertDraftPublicationCurrent();
      const createdAt = new Date().toISOString();
      const nativeScientificSha256 = await services.scientificDigest(model);
      assertDraftPublicationCurrent();
      const recipe = buildGeneralSemRecipeV1({
        recipeId: globalThis.crypto.randomUUID(),
        createdAt,
        dataset,
        model,
        nativeScientificSha256,
        config,
        engine: effectiveEngine,
      });
      const outcome = await services.bootstrapArchive({
        surface: "internal_labs",
        experimentalLabsEnabled: true,
        destinationPath: destination,
        projectId: globalThis.crypto.randomUUID(),
        name: `${projectName} — General SEM`,
        createdAt,
        sourceProjectId,
        sourceDatasetId: dataset.id,
        sourceDatasetFingerprint: dataset.fingerprint,
        model,
        recipe,
      });
      if (outcome.status === "blocked") {
        setFailure({ schemaVersion: 1, stage: "archive_authority", subject: "archive", ...outcome.diagnostic, issues: [] });
        return;
      }
      publishedReceipt = outcome.value.receipt;
      assertDraftPublicationCurrent();
      const inspected = await services.inspectArchive(outcome.value.receipt.destinationArchivePath);
      if (inspected.status === "blocked") {
        throw { schemaVersion: 1, stage: "archive_authority", subject: "archive", ...inspected.diagnostic, issues: [] } satisfies GeneralSemPlsJobFailureV1;
      }
      assertDraftPublicationCurrent();
      if (inspected.value.archiveSha256 !== outcome.value.receipt.destinationArchiveSha256
        || inspected.value.project.sem_generation !== "general_sem_v1") {
        throw new Error("The saved QuickPLS project file could not be verified against its creation receipt.");
      }
      const authoritative = await services.nativePreflight({ project: inspected.value.project, model, config });
      if (authoritative.status === "blocked") {
        throw { schemaVersion: 1, stage: "capability", subject: "preflight", ...authoritative.diagnostic, issues: [] } satisfies GeneralSemPlsJobFailureV1;
      }
      assertDraftPublicationCurrent();
      selectGeneralSemPlsExecutionCapabilityV1({
        model,
        config,
        decision: authoritative.value.pls,
      });
      const createdReceipt = outcome.value.receipt;
      await activateGeneralSemProjectArchiveV1(inspected.value, createdReceipt, {
        openSnapshot: (snapshot) => useInternalProjectArchiveV6Session.getState().open(async () => ({ status: "ok", value: snapshot })),
        activateStandardAuthorities: () => useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(),
        rollbackActivation: () => {
          const current = useInternalProjectArchiveV6Session.getState();
          if (current.session?.standardActivation) current.closeStandardProject();
          else current.deactivate();
        },
        readSession: () => useInternalProjectArchiveV6Session.getState().session,
        readWorkspace: () => useWorkspace.getState(),
      });
      const activatedAuthorityKey = generalSemAuthorityKeyV1({
        sourceProjectId: createdReceipt.projectId,
        datasetId: dataset.id,
        datasetFingerprint: dataset.fingerprint,
        modelScientificInput: scientificSemModelV4HashInput(model),
        config,
        engine: effectiveEngine,
      });
      capturedAuthorityKeyRef.current = activatedAuthorityKey;
      latestAuthorityKeyRef.current = activatedAuthorityKey;
      setReceipt(createdReceipt);
      setCurrentArchiveSha256(createdReceipt.destinationArchiveSha256);
      setArchiveSnapshot(inspected.value);
      setNativePlsPreflight({
        authorityKey: activatedAuthorityKey,
        decision: authoritative.value.pls,
      });
      setSnapshot(null);
    } catch (error) {
      const failure = generalSemFailureV1(error);
      if (publishedReceipt) {
        const current = useWorkspace.getState();
        if (current.projectId === sourceProjectId
          && current.generalSemProjectDraftMode?.sourceProjectId === sourceProjectId) {
          clearGeneralSemProjectDraftMode();
        }
        setFailure({
          ...failure,
          correctiveAction: `A validated marked project was already published at ${publishedReceipt.destinationArchivePath}. Use File > Open to activate that exact file; do not publish this draft again.`,
        });
        pushToast({
          tone: "warning",
          title: "General SEM project saved but not activated",
          detail: `Open ${publishedReceipt.destinationArchivePath} to continue from its validated authority.`,
        });
      } else {
        setFailure(failure);
      }
    } finally {
      setGeneralSemPublicationPending(false);
      setBusy(false);
      window.setTimeout(() => createButtonRef.current?.focus(), 0);
    }
  };

  const applyMonitorOutcome = (outcome: GeneralSemPlsMonitorOutcomeV1): boolean => {
    if (outcome.status === "completed" && generalSemCompletionMatchesLatestAuthorityV1(
      capturedAuthorityKeyRef.current,
      latestAuthorityKeyRef.current ?? "",
    )) {
      if (!nativePlsExecution) {
        setGeneralSemTransientWorkBlocker(null);
        setResultIntegrityInvalid(true);
        setFailure({
          schemaVersion: 1,
          stage: "integrity",
          subject: "native_preflight",
          code: "general_sem.completed_execution_authority_missing",
          message: "The exact native execution capability is no longer available for the completed result.",
          correctiveAction: "The result was not displayed or persisted. Rerun estimator preflight from the unchanged marked project and calculate again.",
          issues: [],
        });
        return true;
      }
      try {
        validateGeneralSemPlsCompletedExecutionV1(outcome.completed, nativePlsExecution);
      } catch (error) {
        setGeneralSemTransientWorkBlocker(null);
        setResultIntegrityInvalid(true);
        setCompleted(null);
        setFailure(generalSemFailureV1(error));
        return true;
      }
      setCompleted(outcome.completed);
      setGeneralSemTransientWorkBlocker("temporary_result_pending");
      setResultIntegrityInvalid(false);
      setJobRecoveryRequired(false);
      setFailure(null);
      window.setTimeout(() => resultHeadingRef.current?.focus(), 0);
      return true;
    }
    if (outcome.status === "completed") {
      setGeneralSemTransientWorkBlocker(null);
      setJobRecoveryRequired(false);
      setFailure({
        schemaVersion: 1,
        stage: "integrity",
        subject: "active_authority",
        code: "general_sem.completed_authority_stale",
        message: "The completed result belongs to an earlier canvas, dataset, or General SEM configuration.",
        correctiveAction: "The stale result was not displayed. Save a fresh calculation project from the intended current canvas and run it again.",
        issues: [],
      });
      return true;
    }
    if (outcome.status === "terminal_without_result") {
      setGeneralSemTransientWorkBlocker(null);
      setJobRecoveryRequired(false);
      if (outcome.snapshot.failure) setFailure(outcome.snapshot.failure);
      return true;
    }
    return false;
  };

  const monitorStartedJob = async (
    initial: GeneralSemPlsJobSnapshotV1,
    controller: AbortController,
  ): Promise<boolean> => applyMonitorOutcome(await monitorGeneralSemPlsJobV1({
    initial,
    getStatus: services.status,
    getResult: services.result,
    onSnapshot: setSnapshot,
    signal: controller.signal,
  }));

  const retainRecoverableJobFailure = (error: unknown) => {
    const diagnostic = generalSemFailureV1(error);
    setGeneralSemTransientWorkBlocker("job_active");
    setJobRecoveryRequired(true);
    setFailure({
      ...diagnostic,
      correctiveAction: `${diagnostic.correctiveAction} Retry job recovery; if the native job can no longer be recovered, explicitly abandon it before leaving this workspace.`,
    });
  };

  const start = async () => {
    if (!receipt || !currentArchiveSha256 || !model || !config || !nativePlsDecision || !resultAuthorityCurrent || !nativePreflightReady || resultIntegrityInvalid || generalSemSessionDirty || running || generalSemTransientWorkBlocker) return;
    const expectedArchiveSha256 = currentArchiveSha256;
    let started = false;
    let terminalKnown = false;
    setBusy(true);
    setFailure(null);
    clearResults();
    setGeneralSemTransientWorkBlocker("job_active");
    const controller = new AbortController();
    monitorAbortRef.current?.abort();
    monitorAbortRef.current = controller;
    try {
      const initial = await services.start(generalSemJobRequestFromReceiptV1(
        receipt,
        model,
        config,
        nativePlsDecision,
        expectedArchiveSha256,
      ));
      started = true;
      activeJobIdRef.current = initial.jobId;
      setSnapshot(initial);
      terminalKnown = await monitorStartedJob(initial, controller);
    } catch (error) {
      if (!controller.signal.aborted) {
        if (generalSemStartedJobRetentionV1({ started, terminalKnown, activeJobId: activeJobIdRef.current }) === "retain") retainRecoverableJobFailure(error);
        else {
          setGeneralSemTransientWorkBlocker(null);
          setFailure(generalSemFailureV1(error));
        }
      }
    } finally {
      if (generalSemStartedJobRetentionV1({ started, terminalKnown, activeJobId: activeJobIdRef.current }) === "release") activeJobIdRef.current = null;
      if (!controller.signal.aborted) setBusy(false);
    }
  };

  const recoverJob = async () => {
    const jobId = activeJobIdRef.current;
    if (!jobId || !jobRecoveryRequired) return;
    let terminalKnown = false;
    setBusy(true);
    setFailure(null);
    setJobRecoveryRequired(false);
    const controller = new AbortController();
    monitorAbortRef.current?.abort();
    monitorAbortRef.current = controller;
    try {
      const current = await services.status(jobId);
      setSnapshot(current);
      terminalKnown = await monitorStartedJob(current, controller);
    } catch (error) {
      if (!controller.signal.aborted) retainRecoverableJobFailure(error);
    } finally {
      if (generalSemStartedJobRetentionV1({ started: true, terminalKnown, activeJobId: activeJobIdRef.current }) === "release") activeJobIdRef.current = null;
      if (!controller.signal.aborted) setBusy(false);
    }
  };

  const cancel = async () => {
    if (!activeJobIdRef.current) return;
    try {
      const cancelled = await services.cancel(activeJobIdRef.current);
      setSnapshot(cancelled);
      if (!ACTIVE_STATES.has(cancelled.state) && cancelled.state !== "completed") {
        activeJobIdRef.current = null;
        setJobRecoveryRequired(false);
        setGeneralSemTransientWorkBlocker(null);
      } else if (cancelled.state === "completed") {
        setJobRecoveryRequired(true);
      }
    }
    catch (error) { setFailure(generalSemFailureV1(error)); }
  };

  const abandonJobRecovery = async () => {
    const jobId = activeJobIdRef.current;
    if (!jobId || !jobRecoveryRequired) return;
    setBusy(true);
    try {
      const cancelled = await services.cancel(jobId);
      if (ACTIVE_STATES.has(cancelled.state)) {
        setSnapshot(cancelled);
        setFailure({
          schemaVersion: 1,
          stage: "estimation",
          subject: jobId,
          code: "general_sem.job_abandon.cancel_pending",
          message: "The General SEM job is still cancelling.",
          correctiveAction: "Wait for cancellation to become terminal, then retry recovery or abandonment.",
          issues: [],
        });
        setBusy(false);
        return;
      }
      try { await services.dismiss(jobId); } catch { /* A consumed or expired terminal job is already gone. */ }
    } catch {
      // Explicit abandonment is also the recovery path when result parsing
      // consumed the one-shot native job before rejecting its payload.
    }
    activeJobIdRef.current = null;
    setSnapshot(null);
    setFailure(null);
    setJobRecoveryRequired(false);
    setGeneralSemTransientWorkBlocker(null);
    setBusy(false);
  };

  const clearTerminal = async () => {
    if (snapshot && !ACTIVE_STATES.has(snapshot.state) && snapshot.state !== "completed") {
      try { await services.dismiss(snapshot.jobId); } catch { /* Expired jobs are already clear. */ }
    }
    setSnapshot(null);
    activeJobIdRef.current = null;
    setFailure(null);
    setBusy(false);
    setJobRecoveryRequired(false);
    setGeneralSemTransientWorkBlocker(null);
  };

  const verifyAndReanchorPersistedArchive = async (updatedArchiveSha256: string) => {
    if (!completed) throw new Error("No completed General SEM result is available for archive verification.");
    if (useInternalProjectArchiveV6Session.getState().dirty) {
      throw new Error("The active General SEM presentation changed after calculation. Restore its saved layout before verifying or appending results.");
    }
    const inspected = await services.inspectArchive(completed.archiveIdentity.archivePath);
    if (inspected.status === "blocked") {
      throw {
        schemaVersion: 1,
        stage: "archive_authority",
        subject: "archive",
        ...inspected.diagnostic,
        issues: [],
      } satisfies GeneralSemPlsJobFailureV1;
    }
    if (inspected.value.archiveSha256 !== updatedArchiveSha256) {
      throw new Error("The appended General SEM archive digest differs from its strict reopen identity.");
    }
    const reanchored = useInternalProjectArchiveV6Session.getState()
      .reanchorGeneralSemSnapshot(inspected.value);
    if (reanchored !== "reanchored") {
      const diagnostic = useInternalProjectArchiveV6Session.getState().standardActivationFailure;
      throw {
        schemaVersion: 1,
        stage: "archive_authority",
        subject: "archive",
        code: diagnostic?.code ?? "general_sem.persisted_reanchor_failed",
        message: diagnostic?.message ?? "The saved result could not reanchor the active General SEM project.",
        correctiveAction: diagnostic?.correctiveAction ?? "Preserve the archive unchanged and reopen it before continuing.",
        issues: [],
      } satisfies GeneralSemPlsJobFailureV1;
    }
    setArchiveSnapshot(inspected.value);
    setCurrentArchiveSha256(updatedArchiveSha256);
    setPersistedArchiveSha256(updatedArchiveSha256);
    setResultIntegrityInvalid(false);
  };

  const appendResult = async () => {
    if (!generalSemResultCanAppendV1({
      completed: Boolean(completed),
      authorityCurrent: resultAuthorityCurrent,
      sessionDirty: generalSemSessionDirty,
      operationPending: generalSemPublicationPending,
      appendSucceeded: appendOutcome?.status === "ok",
      resultIntegrityInvalid,
    })) {
      if (completed && (!resultAuthorityCurrent || generalSemSessionDirty)) {
        setFailure({
          schemaVersion: 1,
          stage: "integrity",
          subject: "active_authority",
          code: !resultAuthorityCurrent ? "general_sem.completed_authority_stale" : "general_sem.presentation_not_persisted",
          message: !resultAuthorityCurrent
            ? "The completed result no longer matches the current archive authority."
            : "The General SEM canvas presentation differs from the saved archive.",
          correctiveAction: !resultAuthorityCurrent
            ? "Reopen the exact marked project or calculate again before saving this result."
            : "Undo the unsaved presentation changes, then save and verify the result.",
          issues: [],
        });
      }
      return;
    }
    if (!completed) return;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    try {
      const outcome = await appendGeneralSemResultV1(completed, services.append);
      if (outcome.status === "ok") {
        // Record the successful append before verification. A later inspect or
        // reanchor failure must retry verification, never append a duplicate.
        setAppendOutcome(outcome);
        setPersistedArchiveSha256(outcome.value.updated_document_sha256);
        setResultIntegrityInvalid(true);
        await verifyAndReanchorPersistedArchive(outcome.value.updated_document_sha256);
      }
      else {
        setAppendOutcome(outcome);
        setResultIntegrityInvalid(true);
        setFailure({ schemaVersion: 1, stage: "archive_authority", subject: "archive", ...outcome.diagnostic, issues: [] });
      }
    } catch (error) {
      setResultIntegrityInvalid(true);
      setFailure(generalSemFailureV1(error));
    } finally {
      setGeneralSemPublicationPending(false);
      setBusy(false);
    }
  };

  const retryPersistedVerification = async () => {
    if (!persistedArchiveSha256 || !completed || !resultAuthorityCurrent || generalSemPublicationPending) return;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    try {
      await verifyAndReanchorPersistedArchive(persistedArchiveSha256);
      const reopened = await reopenGeneralSemResultV1(completed, persistedArchiveSha256, services.read);
      if (reopened.outcome.status === "blocked" || !reopened.entry) {
        throw reopened.outcome.status === "blocked"
          ? { schemaVersion: 1, stage: "archive_authority", subject: "archive", ...reopened.outcome.diagnostic, issues: [] } satisfies GeneralSemPlsJobFailureV1
          : new Error("The verified archive does not contain the appended General SEM result.");
      }
      setReopenedEntry(reopened.entry);
      setResultIntegrityInvalid(false);
      setGeneralSemTransientWorkBlocker(null);
    } catch (error) {
      setResultIntegrityInvalid(true);
      setReopenedEntry(null);
      setFailure(generalSemFailureV1(error));
    } finally {
      setGeneralSemPublicationPending(false);
      setBusy(false);
    }
  };

  const reopenResult = async () => {
    if (!completed || !persistedArchiveSha256 || !resultAuthorityCurrent) return;
    setBusy(true);
    setFailure(null);
    try {
      const reopened = await reopenGeneralSemResultV1(completed, persistedArchiveSha256, services.read);
      if (reopened.outcome.status === "blocked") {
        setResultIntegrityInvalid(true);
        setReopenedEntry(null);
        setFailure({ schemaVersion: 1, stage: "archive_authority", subject: "archive", ...reopened.outcome.diagnostic, issues: [] });
      } else if (!reopened.entry) {
        setResultIntegrityInvalid(true);
        setReopenedEntry(null);
        setFailure({
          schemaVersion: 1,
          stage: "integrity",
          subject: completed.canonicalDocument.document_id,
          code: "general_sem.persisted_result_not_found",
          message: "The verified QuickPLS project file did not contain the completed General SEM result.",
          correctiveAction: "Keep the project file unchanged and report this integrity failure.",
          issues: [],
        });
      } else {
        setReopenedEntry(reopened.entry);
        setResultIntegrityInvalid(false);
        setGeneralSemTransientWorkBlocker(null);
      }
    } catch (error) {
      setResultIntegrityInvalid(true);
      setReopenedEntry(null);
      setFailure(generalSemFailureV1(error));
    }
    finally { setBusy(false); }
  };

  const exportDisplayed = async () => {
    if (!displayedDocument) return;
    setExportFeedback(null);
    try {
      const path = await services.exportXlsx(canonicalResultDocumentV2ExportTables(displayedDocument));
      setExportFeedback(path ? `Saved ${path}.` : "Export cancelled. No file was created.");
    } catch (error) { setExportFeedback(`Export failed: ${generalSemFailureV1(error).message}`); }
  };

  const closeGeneralSemProject = async () => {
    if (operationBusy || running) return;
    if (unpersistedCompletedResult) {
      setFailure({
        schemaVersion: 1,
        stage: "archive_authority",
        subject: "result",
        code: "general_sem.project_close.temporary_result_pending",
        message: "A completed General SEM result has not passed append and strict readback.",
        correctiveAction: "Save and reopen the result first, or explicitly dismiss the temporary result before closing the project.",
        issues: [],
      });
      return;
    }
    setBusy(true);
    try {
      await services.invalidateDraft();
      const outcome = closeGeneralSemProjectV1({
        close: () => useInternalProjectArchiveV6Session.getState().closeStandardProject(),
        readFailure: () => useInternalProjectArchiveV6Session.getState().standardActivationFailure,
      });
      if (outcome.status === "blocked") {
        setFailure(outcome.failure);
        return;
      }
      monitorAbortRef.current?.abort();
      activeJobIdRef.current = null;
      setReceipt(null);
      setArchiveSnapshot(null);
      setCurrentArchiveSha256(null);
      setNativePlsPreflight(null);
      setSnapshot(null);
      setFailure(null);
      clearResults();
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } }));
    } catch (error) {
      setFailure(generalSemFailureV1(error));
    } finally {
      setBusy(false);
    }
  };

  const progressMaximum = Math.max(snapshot?.totalUnits ?? 1, 1);
  const progressValue = Math.min(snapshot?.completedUnits ?? 0, progressMaximum);
  const bootstrap = effectiveEngine.inference === "percentile_case_bootstrap";
  const interactionPlan = Boolean(model?.derived_terms.some((term) => term.kind === "interaction_v2"));
  const moderationBootstrapTurnOffRequired = interactionPlan && bootstrap && !markedGeneralSemProjectMode;
  const moderationBootstrapInputDisabled = running
    || operationBusy
    || markedGeneralSemProjectMode
    || (interactionPlan && !bootstrap);

  return <section id="nd-model-general-sem-labs-panel" className="nd-cbsem-v4-workspace nd-general-sem-workspace" role="tabpanel" aria-labelledby="nd-model-general-sem-labs-tab">
    <header className="nd-cbsem-v4-header"><div><h2>General SEM in QuickPLS</h2><p>One QuickPLS canvas · explicit General SEM project authority · PLS-first Experimental Labs</p></div><FlaskConical size={24} aria-hidden="true" /></header>
    <p className="nd-inline-warning" role="note"><AlertTriangle size={16} aria-hidden="true" /><span>{freshGeneralSemDraftMode
      ? "This is a fresh General SEM draft inside QuickPLS. Only this newly created canvas may be adapted; ordinary projects are never converted. Standard Save is blocked until Save and activate creates the marked schema-6 authority."
      : markedGeneralSemProjectMode
        ? "General SEM is a project mode inside QuickPLS, not a separate application. This canvas is bound to the activated, newly created general_sem_v1 authority."
        : "This ordinary QuickPLS project is intentionally isolated from General SEM project mode. QuickPLS will not copy, adapt, or relabel its canvas as general_sem_v1."}</span></p>

    <div className="nd-cbsem-v4-grid">
      <section className="nd-cbsem-v4-card" aria-labelledby="nd-general-sem-input-heading">
        <h3 id="nd-general-sem-input-heading">Scientific input and inference</h3>
        <p className="nd-cbsem-v4-summary"><strong>Project mode</strong><span>{freshGeneralSemDraftMode ? "Fresh General SEM draft" : markedGeneralSemProjectMode ? "General SEM (general_sem_v1)" : "Ordinary QuickPLS project — calculation blocked"}</span></p>
        <p className="nd-cbsem-v4-summary"><strong>Model</strong><span>{model ? `${modelName} (${model.id})` : freshGeneralSemDraftMode ? "Resolve this new canvas model first" : "Create or activate a General SEM project first"}</span></p>
        <p className="nd-cbsem-v4-summary"><strong>Dataset</strong><span>{dataset.name} · {dataset.rowCount ?? dataset.rows.length} cases</span></p>
        <fieldset className="nd-cbsem-v4-scales"><legend>Inference</legend>
          <label className="nd-checkbox-row" htmlFor="nd-general-sem-bootstrap"><input id="nd-general-sem-bootstrap" type="checkbox" checked={bootstrap} disabled={moderationBootstrapInputDisabled} aria-describedby={interactionPlan ? "nd-general-sem-moderation-inference-note" : undefined} onChange={(event) => {
            if (markedGeneralSemProjectMode) return;
            if (interactionPlan && event.target.checked) return;
            setEngine((current) => ({ ...current, inference: event.target.checked ? "percentile_case_bootstrap" : "none" }));
            setReceipt(null); setArchiveSnapshot(null); setNativePlsPreflight(null); clearResults();
          }} />Full-model percentile case bootstrap</label>
          {interactionPlan ? <p id="nd-general-sem-moderation-inference-note" className="nd-inline-warning" role="status">
            Simultaneous two-way moderation is point-estimation only. Bootstrap inference is not qualified for this exact capability cell.{moderationBootstrapTurnOffRequired ? " Turn off Full-model percentile case bootstrap to continue." : " The bootstrap option remains unavailable for this model."}
          </p> : null}
          {bootstrap ? <>
            <label htmlFor="nd-general-sem-bootstrap-samples">Replicates<input id="nd-general-sem-bootstrap-samples" type="number" min={2} max={10_000} step={100} value={effectiveEngine.bootstrapSamples} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, bootstrapSamples: Number(event.target.value) }))} /></label>
            <label htmlFor="nd-general-sem-confidence">Confidence level<input id="nd-general-sem-confidence" type="number" min={0.8} max={0.999} step={0.01} value={effectiveEngine.confidenceLevel} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, confidenceLevel: Number(event.target.value) }))} /></label>
          </> : null}
          <label htmlFor="nd-general-sem-seed">Seed<input id="nd-general-sem-seed" type="number" min={0} step={1} value={effectiveEngine.seed} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, seed: Number(event.target.value) }))} /></label>
          <label htmlFor="nd-general-sem-workers">Workers<input id="nd-general-sem-workers" type="number" min={1} max={64} step={1} value={effectiveEngine.workers} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, workers: Number(event.target.value) }))} /></label>
        </fieldset>
        {markedGeneralSemProjectMode ? <p className="nd-cbsem-v4-summary"><strong>Recipe authority</strong><span>Settings are restored from the resident RecipeV4 and remain immutable for this calculation identity.</span></p> : null}
        {model && config ? <GeneralSemEstimatorCompatibilityPanel model={model} config={config} onSelectEstimator={() => undefined} selectedEstimatorId="qpls.pls_sem.v3" /> : null}
      </section>

      <section id="nd-general-sem-preflight" className="nd-cbsem-v4-card" tabIndex={-1} aria-labelledby="nd-general-sem-preflight-heading">
        <h3 id="nd-general-sem-preflight-heading">Prepare and verify calculation</h3>
        <ol className="nd-cbsem-v4-preflight-list">
          <li className={localPreflight.ready ? "ready" : "blocked"}><span aria-hidden="true">{localPreflight.ready ? "✓" : "!"}</span><div><strong>General SEM project and model authority</strong><small>{localPreflight.ready ? "Ready for QuickPLS engine verification" : `${localPreflight.issues.length} issue${localPreflight.issues.length === 1 ? "" : "s"}`}</small>{localPreflight.issues.map((item) => <p key={`${item.code}:${item.subject}`}><strong>{item.message}</strong> {item.correctiveAction} <code>{item.code}</code></p>)}</div></li>
          <li className={receipt ? "ready" : "blocked"}><span aria-hidden="true">{receipt ? "✓" : "2"}</span><div><strong>Safe QuickPLS project file</strong><small>{receipt ? `Verified ${receipt.destinationArchivePath}` : "Save the current dataset, canvas model, and analysis settings in one calculation file"}</small>{receipt && !archiveCurrent ? <p>The canvas or settings changed. Keep the saved file unchanged and create a fresh calculation project from the current canvas.</p> : null}</div></li>
          <li className={nativePreflightReady && archiveCurrent ? "ready" : "blocked"}><span aria-hidden="true">{nativePreflightReady && archiveCurrent ? "✓" : "3"}</span><div><strong>QuickPLS engine preflight</strong><small>{nativePreflightReady && archiveCurrent ? nativePlsExecution?.kind === "multiple_two_way_moderation_point" ? "Exact simultaneous two-way moderation point cell verified" : "Experimental PLS support verified" : "Pending final engine verification"}</small></div></li>
        </ol>
        <div className="nd-cbsem-v4-actions">
          <button ref={createButtonRef} type="button" className="primary" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || !freshGeneralSemDraftMode || !localPreflight.ready} title={!freshGeneralSemDraftMode ? "Start a new General SEM project to create its marked authority; existing projects cannot enter this path." : !localPreflight.ready ? "Resolve every compatibility issue first." : "Save and activate this new General SEM project as the current QuickPLS canvas authority."} onClick={() => void createCalculationProject()}><Archive size={15} aria-hidden="true" />Save and activate project…</button>
          <button type="button" className="primary" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || !receipt || !resultAuthorityCurrent || !nativePreflightReady || resultIntegrityInvalid || generalSemSessionDirty} title={generalSemSessionDirty ? "Undo unsaved presentation changes before calculating from this fixed archive authority." : !resultAuthorityCurrent ? "Reopen the exact marked project authority before calculating." : undefined} onClick={() => void start()}><Play size={15} aria-hidden="true" />{interactionPlan ? "Calculate moderation point estimates" : "Calculate PLS effects"}</button>
          <button type="button" className="danger" disabled={!activeJobIdRef.current || snapshot?.state === "cancelling" || snapshot?.state === "completed"} onClick={() => void cancel()}><CircleStop size={15} aria-hidden="true" />Cancel</button>
          {markedGeneralSemProjectMode ? <button type="button" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || unpersistedCompletedResult} title={unpersistedCompletedResult ? "Save and strictly reopen the completed result, or dismiss it explicitly, before closing." : undefined} onClick={() => void closeGeneralSemProject()}><FolderOpen size={15} aria-hidden="true" />Close General SEM project</button> : null}
        </div>
        {generalSemSessionDirty ? <p className="nd-inline-warning" role="status">The canvas presentation differs from the saved archive. Undo those presentation changes before calculating or appending a result.</p> : null}
      </section>
    </div>

    {snapshot ? <section className="nd-cbsem-v4-card nd-cbsem-v4-monitor" aria-labelledby="nd-general-sem-monitor-heading"><div><h3 id="nd-general-sem-monitor-heading">Calculation progress</h3><span className={`nd-cbsem-v4-state ${snapshot.state}`}>{snapshot.state}</span></div><progress max={progressMaximum} value={progressValue}>{progressValue} of {progressMaximum}</progress><p aria-live="polite" aria-atomic="true">{snapshot.phase}: {snapshot.completedUnits} of {snapshot.totalUnits}</p>{snapshot.state === "failed" || snapshot.state === "cancelled" ? <button type="button" onClick={() => void clearTerminal()}><RotateCcw size={15} aria-hidden="true" />Clear terminal job</button> : null}{jobRecoveryRequired ? <div className="nd-cbsem-v4-actions"><button type="button" className="primary" disabled={busy} onClick={() => void recoverJob()}><RotateCcw size={15} aria-hidden="true" />Retry job recovery</button><button type="button" className="danger" disabled={busy} onClick={() => void abandonJobRecovery()}><CircleStop size={15} aria-hidden="true" />Abandon unrecovered job</button></div> : null}</section> : null}
    {failure ? <div className="nd-cbsem-v4-failure" role="alert"><AlertTriangle size={16} aria-hidden="true" /><div><strong>{failure.message}</strong><p>{failure.correctiveAction}</p><small>{failure.code}</small></div></div> : null}

    {completed ? <section className="nd-cbsem-v4-card nd-cbsem-v4-archive" aria-labelledby="nd-general-sem-persistence-heading">
      <h3 id="nd-general-sem-persistence-heading"><Archive size={16} aria-hidden="true" />Save and verify result</h3>
      <p>The completed result is held temporarily until it is safely written to this QuickPLS calculation project.</p>
      <div className="nd-cbsem-v4-actions">
        <button type="button" className="primary" disabled={operationBusy || appendOutcome?.status === "ok" || !resultAuthorityCurrent || generalSemSessionDirty} onClick={() => void appendResult()}><Archive size={15} aria-hidden="true" />Save result to project</button>
        <button type="button" disabled={operationBusy || !persistedArchiveSha256 || !resultAuthorityCurrent || generalSemSessionDirty} onClick={() => void (resultIntegrityInvalid ? retryPersistedVerification() : reopenResult())}><FolderOpen size={15} aria-hidden="true" />{resultIntegrityInvalid && appendOutcome?.status === "ok" ? "Verify and reanchor saved result" : "Reopen and verify"}</button>
        <button type="button" disabled={!displayedDocument} onClick={() => void exportDisplayed()}><Download size={15} aria-hidden="true" />Export XLSX</button>
        {unpersistedCompletedResult ? <button type="button" className="danger" disabled={operationBusy} onClick={clearResults}>Dismiss temporary result</button> : null}
      </div>
      {appendOutcome?.status === "ok" ? <p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Saved result {appendOutcome.value.canonical_document_id}.</p> : null}
      {reopenedEntry ? <p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Reopened and verified result {reopenedEntry.documentId}.</p> : null}
      {exportFeedback ? <p role="status" aria-live="polite">{exportFeedback}</p> : null}
    </section> : null}
    {!completed && reopenedEntry ? <section className="nd-cbsem-v4-card nd-cbsem-v4-archive" aria-labelledby="nd-general-sem-reopened-result-heading"><h3 id="nd-general-sem-reopened-result-heading"><CheckCircle2 size={16} aria-hidden="true" />Verified project result</h3><p>QuickPLS restored the latest matching General SEM result from strict archive readback.</p><div className="nd-cbsem-v4-actions"><button type="button" disabled={!displayedDocument} onClick={() => void exportDisplayed()}><Download size={15} aria-hidden="true" />Export XLSX</button></div><p className="nd-cbsem-v4-success" role="status">Verified result {reopenedEntry.documentId}.</p>{exportFeedback ? <p role="status" aria-live="polite">{exportFeedback}</p> : null}</section> : null}

    {moderationInventory ? <p className="nd-cbsem-v4-success" role="status" aria-live="polite">
      Verified canonical moderation output: {moderationInventory.interactionEffectCount} interaction effect{moderationInventory.interactionEffectCount === 1 ? "" : "s"}, {moderationInventory.conditionalSlopeCount} conditional slope{moderationInventory.conditionalSlopeCount === 1 ? "" : "s"}, and {moderationInventory.interactionPlotCount} interaction plot{moderationInventory.interactionPlotCount === 1 ? "" : "s"} with {moderationInventory.interactionPlotPointCount} persisted point{moderationInventory.interactionPlotPointCount === 1 ? "" : "s"}. QuickPLS displays and exports the native canonical values without adding inference.
    </p> : null}
    {displayedDocument ? <CanonicalResultDocumentV2View document={displayedDocument} reopened={Boolean(reopenedEntry)} headingRef={resultHeadingRef} compilationReceipt={null} /> : null}
    {archiveSnapshot ? <details className="nd-cbsem-v4-run-details"><summary>Calculation project receipt</summary><dl><div><dt>Project</dt><dd>{archiveSnapshot.project.project_id}</dd></div><div><dt>File SHA-256</dt><dd>{currentArchiveSha256 ?? archiveSnapshot.archiveSha256}</dd></div><div><dt>Model</dt><dd>{receipt?.residentModelId}</dd></div><div><dt>Recipe</dt><dd>{receipt?.residentRecipeId}</dd></div></dl></details> : null}
  </section>;
}

export default NativeRecipeV4GeneralSemWorkspace;
