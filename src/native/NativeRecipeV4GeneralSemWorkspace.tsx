import { save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  CircleStop,
  FlaskConical,
  FolderOpen,
  Play,
  RotateCcw,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  appendGeneralSemResultV1,
  bindGeneralSemPlsModelToDatasetV1,
  buildGeneralSemEstimatorRecipeV1,
  defaultGeneralSemPlsEngineOptionsV1,
  generalSemConfigFromEngineV1,
  generalSemFailureV1,
  generalSemJobRequestForEstimatorFromReceiptV1,
  generalSemRequestedCapabilityCellV1,
  monitorGeneralSemCbsemJobV1,
  monitorGeneralSemPlsJobV1,
  preflightGeneralSemWorkspaceV1,
  rehydrateGeneralSemExecutionAuthorityV1,
  reopenGeneralSemResultV1,
  selectGeneralSemCbsemExecutionCapabilityV1,
  selectGeneralSemExecutionAccessV1,
  selectGeneralSemPlsExecutionCapabilityV1,
  validateGeneralSemCbsemCompletedExecutionV1,
  validateGeneralSemPlsCompletedExecutionV1,
  type GeneralSemCbsemMonitorOutcomeV1,
  type GeneralSemCompletedResultV1,
  type GeneralSemEstimatorIdV1,
  type GeneralSemPlsEngineOptionsV1,
  type GeneralSemPlsJobFailureV1,
  type GeneralSemPlsJobSnapshotV1,
  type GeneralSemPlsMonitorOutcomeV1,
  type GeneralSemProjectBootstrapReceiptV1,
  type GeneralSemEstimatorParameterTableAuthorityV2,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
} from "../domain/generalSemCapabilityPreflightV1";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { UnifiedSemCalculationPlanV1 } from "../domain/unifiedSemCalculationV1";
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
  GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1,
  GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_REQUIRES_LABS_V1,
} from "../services/internalGeneralSemModeratedMediationRevisionV2Service";
import {
  appendInternalProjectSchema6CanonicalResultV2,
  bootstrapInternalGeneralSemProjectArchiveV6,
  cancelInternalLabsGeneralSemCbsemJobV1,
  cancelInternalLabsGeneralSemPlsJobV1,
  dismissInternalLabsGeneralSemCbsemJobV1,
  dismissInternalLabsGeneralSemPlsJobV1,
  getInternalLabsGeneralSemCbsemJobResultV1,
  getInternalLabsGeneralSemCbsemJobV1,
  getInternalLabsGeneralSemPlsJobResultV1,
  getInternalLabsGeneralSemPlsJobV1,
  getInternalSemModelV4ScientificSha256,
  invalidateNativeGeneralSemFreshDraftAuthorityV1,
  openNativeProjectAt,
  preflightInternalGeneralSemEstimatorsV1,
  readInternalProjectSchema6CanonicalResultsV2,
  startInternalLabsGeneralSemCbsemJobV1,
  startInternalLabsGeneralSemPlsJobV1,
} from "../services/projectService";
import { useWorkspace } from "../store";
import { GeneralSemEstimatorCompatibilityPanel } from "./GeneralSemEstimatorCompatibilityPanel";
import { NativeGeneralSemModeratedMediationPanel } from "./NativeGeneralSemModeratedMediationPanel";
import { CanonicalResultExportPanelV2 } from "./CanonicalResultExportPanelV2";
import { observedSemanticsForParameterTable } from "./NativeSemParameterTable";
import {
  CanonicalResultDocumentV2View,
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
  startCbsem: typeof startInternalLabsGeneralSemCbsemJobV1;
  statusCbsem: typeof getInternalLabsGeneralSemCbsemJobV1;
  cancelCbsem: typeof cancelInternalLabsGeneralSemCbsemJobV1;
  dismissCbsem: typeof dismissInternalLabsGeneralSemCbsemJobV1;
  resultCbsem: typeof getInternalLabsGeneralSemCbsemJobResultV1;
  append: typeof appendInternalProjectSchema6CanonicalResultV2;
  read: typeof readInternalProjectSchema6CanonicalResultsV2;
  invalidateDraft: typeof invalidateNativeGeneralSemFreshDraftAuthorityV1;
  /** Keeps the native DesktopProject synchronized after a new schema-6 file is activated. */
  adoptActiveProject?: typeof openNativeProjectAt;
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
  startCbsem: startInternalLabsGeneralSemCbsemJobV1,
  statusCbsem: getInternalLabsGeneralSemCbsemJobV1,
  cancelCbsem: cancelInternalLabsGeneralSemCbsemJobV1,
  dismissCbsem: dismissInternalLabsGeneralSemCbsemJobV1,
  resultCbsem: getInternalLabsGeneralSemCbsemJobResultV1,
  append: appendInternalProjectSchema6CanonicalResultV2,
  read: readInternalProjectSchema6CanonicalResultsV2,
  invalidateDraft: invalidateNativeGeneralSemFreshDraftAuthorityV1,
  selectDestination: async (suggestedName) => {
    const selected = await save({
      defaultPath: suggestedName,
      filters: [{ name: "QuickPLS calculation-ready project", extensions: ["qpls"] }],
    });
    return typeof selected === "string" ? selected : null;
  },
};

export interface NativeRecipeV4GeneralSemWorkspaceProps {
  modelName: string;
  experimentalLabsEnabled: boolean;
  presentation?: "workspace" | "calculation";
  initialCalculationKind?: "pls_algorithm" | "pls_bootstrap" | "cbsem";
  /** Resolved by the single Calculate dialog; the executor still recompiles and preflights it. */
  calculationPlan?: UnifiedSemCalculationPlanV1 | null;
  /** Publishes/activates a safe revision, then hands control to a compatibility coordinator. */
  activationOnly?: boolean;
  onAuthorityActivated?: () => void;
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

export interface GeneralSemNativeEstimatorPreflightAuthorityV2 {
  readonly authorityKey: string;
  readonly pls: SemCapabilityDecisionV1;
  readonly cbsem: SemCapabilityDecisionV1;
  readonly authority: GeneralSemEstimatorParameterTableAuthorityV2;
}

export function selectCurrentGeneralSemNativeEstimatorPreflightV2(
  preflight: GeneralSemNativeEstimatorPreflightAuthorityV2 | null,
  authorityKey: string,
): GeneralSemNativeEstimatorPreflightAuthorityV2 | null {
  return preflight?.authorityKey === authorityKey ? preflight : null;
}

export function selectCurrentGeneralSemNativePlsDecisionV1(
  preflight: GeneralSemNativeEstimatorPreflightAuthorityV2 | null,
  authorityKey: string,
): SemCapabilityDecisionV1 | null {
  return selectCurrentGeneralSemNativeEstimatorPreflightV2(preflight, authorityKey)?.pls ?? null;
}

export interface GeneralSemCanonicalModerationInventoryV1 {
  readonly interactionEffectCount: number;
  readonly gammaInferenceCount: number;
  readonly conditionalSlopeCount: number;
  readonly interactionPlotCount: number;
  readonly interactionPlotPointCount: number;
  readonly bootstrapResamplesRequested: number | null;
  readonly bootstrapResamplesUsable: number | null;
  readonly conditionalIndirectCount: number;
  readonly moderatedMediationIndexCount: number;
  readonly combinedModeratedMediation: boolean;
}

export function generalSemCanonicalModerationInventoryV1(
  document: CanonicalResultDocumentV2 | null,
): GeneralSemCanonicalModerationInventoryV1 | null {
  const results = document?.general_sem_results;
  const interactionEffectCount = results?.interaction_effects?.length ?? 0;
  if (interactionEffectCount === 0) return null;
  const inferenceCellId = results?.inference_receipt?.capability_cell.cell_id;
  const moderationBootstrapReceipt = results && (inferenceCellId
    === "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
    || inferenceCellId === "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap")
    ? results.inference_receipt
    : null;
  return {
    interactionEffectCount,
    gammaInferenceCount: results?.interaction_effects?.filter((effect) => (
      effect.scientific_rescaled_gamma?.standard_error != null
    )).length ?? 0,
    conditionalSlopeCount: results?.conditional_effects?.length ?? 0,
    interactionPlotCount: results?.interaction_plots?.length ?? 0,
    interactionPlotPointCount: results?.interaction_plots?.reduce((plotTotal, plot) => (
      plotTotal + plot.series.reduce((seriesTotal, series) => seriesTotal + series.points.length, 0)
    ), 0) ?? 0,
    bootstrapResamplesRequested: moderationBootstrapReceipt?.resamples_requested ?? null,
    bootstrapResamplesUsable: moderationBootstrapReceipt?.resamples_usable ?? null,
    conditionalIndirectCount: results?.conditional_indirect_effects?.length ?? 0,
    moderatedMediationIndexCount: results?.moderated_mediation_indices?.length ?? 0,
    combinedModeratedMediation: inferenceCellId
      === "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
  };
}

export function generalSemCalculationActionLabelV1(
  interactionPlan: boolean,
  bootstrap: boolean,
  estimatorOrHigherOrder: GeneralSemEstimatorIdV1 | boolean = GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  higherOrderPlan = false,
): string {
  const estimatorId = typeof estimatorOrHigherOrder === "boolean"
    ? GENERAL_SEM_PLS_ESTIMATOR_ID_V1
    : estimatorOrHigherOrder;
  const hasHigherOrderPlan = typeof estimatorOrHigherOrder === "boolean"
    ? estimatorOrHigherOrder
    : higherOrderPlan;
  if (estimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1) {
    return bootstrap ? "Calculate CB-SEM recursive bootstrap" : "Calculate CB-SEM ML estimates";
  }
  if (hasHigherOrderPlan) {
    return bootstrap ? "Calculate HOC bootstrap" : "Calculate HOC point estimates";
  }
  if (!interactionPlan) return "Calculate PLS effects";
  return bootstrap ? "Calculate moderation bootstrap" : "Calculate moderation point estimates";
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

export function generalSemAutomaticPersistenceNextActionV1(input: {
  completed: boolean;
  appendSucceeded: boolean;
  persistedArchiveAvailable: boolean;
  reopened: boolean;
  authorityCurrent: boolean;
  sessionDirty: boolean;
  resultIntegrityInvalid: boolean;
  executionReady: boolean;
  appendStarted: boolean;
  reopenStarted: boolean;
}): "append" | "reopen" | null {
  if (!input.completed
    || !input.authorityCurrent
    || input.sessionDirty
    || input.resultIntegrityInvalid
    || !input.executionReady) return null;
  if (!input.appendSucceeded) return input.appendStarted ? null : "append";
  if (!input.persistedArchiveAvailable || input.reopened || input.reopenStarted) return null;
  return "reopen";
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
      message: diagnostic?.message ?? "No active calculation-ready project could be closed.",
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
      throw new Error("The newly saved calculation-ready project did not activate as the exact model and digest authority.");
    }

    workspace.setProjectMeta(receipt.name, receipt.destinationArchivePath, receipt.projectId);
    workspace.clearGeneralSemProjectDraftMode();
  } catch (error) {
    if (openedHere) bridge.rollbackActivation();
    throw error;
  }
}

function generalSemAuthorityKeyV1(input: {
  estimatorId: GeneralSemEstimatorIdV1;
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

function currentGeneralSemDraftPublicationKeyV1(
  fallbackModelName: string,
  estimatorId: GeneralSemEstimatorIdV1,
): string {
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
  if (!adapted.ok) throw new Error("The current Canvas no longer has a valid scientific model authority.");
  const bound = bindGeneralSemPlsModelToDatasetV1(adapted.model, state.dataset);
  return JSON.stringify({
    estimatorId,
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
  presentation = "workspace",
  initialCalculationKind = "pls_algorithm",
  calculationPlan = null,
  activationOnly = false,
  onAuthorityActivated,
  projectActivationConnected = false,
  services = defaultServices,
}: NativeRecipeV4GeneralSemWorkspaceProps) {
  const calculationPresentation = presentation === "calculation";
  const workspaceProjectId = useWorkspace((state) => state.projectId);
  const workspaceProjectPath = useWorkspace((state) => state.projectPath);
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
  const generalSemRevisionPending = useInternalProjectArchiveV6Session((state) => state.revisionForkPending);
  const generalSemRevisionFailure = useInternalProjectArchiveV6Session((state) => state.revisionForkFailure);
  const generalSemRevisionStatusMessage = useInternalProjectArchiveV6Session((state) => state.revisionForkStatusMessage);
  const reviseGeneralSemModeratedMediationAuthority = useInternalProjectArchiveV6Session(
    (state) => state.reviseGeneralSemModeratedMediationAuthority,
  );
  const activatedGeneralSemProjectMode = Boolean(
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
  const markedGeneralSemProjectMode = activatedGeneralSemProjectMode && !freshGeneralSemDraftMode;
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
  const [draftEstimatorId, setDraftEstimatorId] = useState<GeneralSemEstimatorIdV1>(
    (calculationPlan?.method ?? initialCalculationKind) === "cbsem"
      ? GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
      : GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  );
  const residentEstimatorId = rehydratedExecution?.status === "ok"
    ? rehydratedExecution.value.estimatorId
    : null;
  const selectedEstimatorId = markedGeneralSemProjectMode && residentEstimatorId
    ? residentEstimatorId
    : draftEstimatorId;
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
    ...(rehydratedExecution?.status !== "ok" && (calculationPlan?.inference === "case_bootstrap" || initialCalculationKind === "pls_bootstrap")
      ? {
          inference: "percentile_case_bootstrap" as const,
          bootstrapSamples: calculationPlan?.requestedConfig?.inference.kind === "case_bootstrap"
            ? calculationPlan.requestedConfig.inference.resamples
            : Math.max(analysisSettings.bootstrapSamples ?? 500, 2),
          seed: calculationPlan?.requestedConfig?.inference.kind === "case_bootstrap"
            ? calculationPlan.requestedConfig.inference.seed
            : analysisSettings.seed,
          confidenceLevel: calculationPlan?.requestedConfig?.inference.kind === "case_bootstrap"
            ? calculationPlan.requestedConfig.inference.confidence_level
            : analysisSettings.confidenceLevel,
        }
      : {}),
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
  const [nativeEstimatorPreflight, setNativeEstimatorPreflight] = useState<GeneralSemNativeEstimatorPreflightAuthorityV2 | null>(null);
  const [snapshot, setSnapshot] = useState<GeneralSemPlsJobSnapshotV1 | null>(null);
  const [completed, setCompleted] = useState<GeneralSemCompletedResultV1 | null>(null);
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
  const activeJobIdRef = useRef<string | null>(null);
  const monitorAbortRef = useRef<AbortController | null>(null);
  const capturedAuthorityKeyRef = useRef<string | null>(null);
  const latestAuthorityKeyRef = useRef<string | null>(null);
  const resultHeadingRef = useRef<HTMLHeadingElement>(null);
  const createButtonRef = useRef<HTMLButtonElement>(null);
  const automatedJourneyRef = useRef({
    key: "",
    createStarted: false,
    calculationStarted: false,
    appendStarted: false,
    reopenStarted: false,
  });

  useEffect(() => {
    if (!reopenedEntry || resultIntegrityInvalid) return;
    window.dispatchEvent(new CustomEvent("quickpls:general-sem-canonical-result", {
      detail: { document: reopenedEntry.canonicalDocument, navigate: presentation === "calculation" },
    }));
  }, [presentation, reopenedEntry, resultIntegrityInvalid]);

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
    () => freshGeneralSemDraftMode && !strictAuthority ? adaptAuthoredNativeWorkbenchToSemModelV4(draftAuthoringInput) : null,
    [draftAuthoringInput, freshGeneralSemDraftMode, strictAuthority],
  );
  // Adaptation is allowed only inside the identity-bound fresh draft. Every
  // ordinary or previously opened project remains ineligible.
  const model = useMemo(() => {
    if (!projectActivationConnected) return null;
    if (markedGeneralSemProjectMode) return strictAuthority?.model ?? null;
    if (!freshGeneralSemDraftMode) return null;
    const draftModel = strictAuthority?.model ?? (adaptedDraft?.ok ? adaptedDraft.model : null);
    if (!draftModel) return null;
    try { return bindGeneralSemPlsModelToDatasetV1(draftModel, dataset); } catch { return null; }
  }, [adaptedDraft, dataset, freshGeneralSemDraftMode, markedGeneralSemProjectMode, projectActivationConnected, strictAuthority]);
  const config = useMemo(() => {
    if (markedGeneralSemProjectMode) {
      return rehydratedExecution?.status === "ok" ? rehydratedExecution.value.config : null;
    }
    try {
      return generalSemConfigFromEngineV1(
        effectiveEngine,
        calculationPlan?.requestedConfig?.requested_effect_estimands ?? [],
      );
    } catch { return null; }
  }, [calculationPlan?.requestedConfig?.requested_effect_estimands, effectiveEngine, markedGeneralSemProjectMode, rehydratedExecution]);
  const localPreflight = useMemo(() => !projectActivationConnected ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.primary_activation_pending",
      subject: "project",
      message: "Primary calculation-ready project activation is not connected in this build.",
      correctiveAction: "Keep this capability disabled until the complete new-project bootstrap and strict activation bridge is installed.",
    }],
  } : !freshGeneralSemDraftMode && !markedGeneralSemProjectMode ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.required",
      subject: "project",
      message: "The open canvas is neither a new calculation-ready draft nor an activated scientific authority.",
      correctiveAction: "Create a source-preserving calculation-ready revision before using this advanced method.",
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
  } : markedGeneralSemProjectMode
    && rehydratedExecution?.status === "ok"
    && rehydratedExecution.value.legacyLabsRecipeOnStandardCell ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.recipe.historical_labs_read_only",
      subject: rehydratedExecution.value.receipt.residentRecipeId,
      message: "This historical Labs RecipeV4 remains readable under its stored identity, but it is not relabelled as a Standard execution recipe.",
      correctiveAction: "Keep the archive unchanged. Use a newly authored Registry-authorized Standard project or an explicit source-preserving revision for new calculations.",
    }],
  } : markedGeneralSemProjectMode && !strictAuthority ? {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.project_mode.strict_authority_required",
      subject: "model",
      message: "The calculation-ready project has no active strict SemModelV4 authority.",
      correctiveAction: "Activate a ready or draft SemModelV4 authority from the marked schema-6 project before calculating.",
    }],
  } : config ? preflightGeneralSemWorkspaceV1({
    experimentalLabsEnabled,
    sourceProjectId,
    dataset,
    model,
    config,
    engine: effectiveEngine,
    estimatorId: selectedEstimatorId,
  }) : {
    ready: false,
    decision: null,
    issues: [{
      code: "general_sem.config.invalid",
      subject: "config",
      message: "The advanced-method configuration is invalid.",
      correctiveAction: "Correct the bounded inference and output settings.",
    }],
  }, [config, dataset, effectiveEngine, experimentalLabsEnabled, freshGeneralSemDraftMode, markedGeneralSemProjectMode, model, projectActivationConnected, rehydratedExecution, selectedEstimatorId, sourceProjectId, strictAuthority]);
  const modelScientificInput = useMemo(() => {
    if (!model) return "";
    try { return scientificSemModelV4HashInput(model); } catch { return ""; }
  }, [model]);
  const authorityKey = useMemo(() => generalSemAuthorityKeyV1({
    estimatorId: selectedEstimatorId,
    sourceProjectId,
    datasetId: dataset.id,
    datasetFingerprint: dataset.fingerprint,
    modelScientificInput,
    config,
    engine: effectiveEngine,
  }), [config, dataset.fingerprint, dataset.id, effectiveEngine, modelScientificInput, selectedEstimatorId, sourceProjectId]);
  latestAuthorityKeyRef.current = authorityKey;
  const currentNativeEstimatorPreflight = selectCurrentGeneralSemNativeEstimatorPreflightV2(
    nativeEstimatorPreflight,
    authorityKey,
  );
  const nativeEstimatorDecision = selectedEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? currentNativeEstimatorPreflight?.cbsem ?? null
    : currentNativeEstimatorPreflight?.pls ?? null;
  const selectedCapabilityCell = useMemo(() => {
    if (!model || !config) return null;
    return generalSemRequestedCapabilityCellV1(selectedEstimatorId, model, config);
  }, [config, model, selectedEstimatorId]);
  const nativePlsExecution = useMemo(() => {
    if (selectedEstimatorId !== GENERAL_SEM_PLS_ESTIMATOR_ID_V1
      || !nativeEstimatorDecision || !model || !config) return null;
    try {
      return selectGeneralSemPlsExecutionCapabilityV1({
        model,
        config,
        decision: nativeEstimatorDecision,
      });
    } catch {
      return null;
    }
  }, [config, model, nativeEstimatorDecision, selectedEstimatorId]);
  const nativeCbsemExecution = useMemo(() => {
    if (selectedEstimatorId !== GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
      || !nativeEstimatorDecision || !config) return null;
    try {
      return selectGeneralSemCbsemExecutionCapabilityV1({
        config,
        decision: nativeEstimatorDecision,
      });
    } catch {
      return null;
    }
  }, [config, nativeEstimatorDecision, selectedEstimatorId]);
  const nativeExecution = nativeCbsemExecution ?? nativePlsExecution;
  const nativePreflightReady = nativeExecution !== null;
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
  const operationBusy = busy || generalSemPublicationPending || generalSemRevisionPending;
  const startJob = residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? services.startCbsem
    : services.start;
  const statusJob = residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? services.statusCbsem
    : services.status;
  const cancelJob = residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? services.cancelCbsem
    : services.cancel;
  const dismissJob = residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? services.dismissCbsem
    : services.dismiss;

  useEffect(() => {
    if (!markedGeneralSemProjectMode || !generalSemSession) {
      setNativeEstimatorPreflight(null);
      return;
    }
    if (rehydratedExecution?.status !== "ok" || !model || !config || !selectedCapabilityCell) {
      setNativeEstimatorPreflight(null);
      if (rehydratedExecution?.status === "blocked") setFailure(rehydratedExecution.failure);
      return;
    }
    const rehydrated = rehydratedExecution.value;
    setReceipt(rehydrated.receipt);
    setArchiveSnapshot(generalSemSession.snapshot);
    setCurrentArchiveSha256(generalSemSession.snapshot.archiveSha256);
    capturedAuthorityKeyRef.current = authorityKey;
    latestAuthorityKeyRef.current = authorityKey;
    setNativeEstimatorPreflight(null);
    const requestedAuthorityKey = authorityKey;
    let live = true;
    void services.nativePreflight({
      project: generalSemSession.project,
      modelId: model.id,
      config,
      capabilityCell: selectedCapabilityCell,
      experimentalLabsEnabled,
    }).then((outcome) => {
      if (!live || latestAuthorityKeyRef.current !== requestedAuthorityKey) return;
      if (outcome.status === "ok") {
        try {
          if (outcome.value.authority.modelId !== rehydrated.receipt.residentModelId
            || outcome.value.authority.modelScientificSha256 !== rehydrated.receipt.residentModelScientificSha256) {
            throw new Error("Native estimator preflight returned a different resident SemModelV4 parameter-table authority.");
          }
          setNativeEstimatorPreflight({
            authorityKey: requestedAuthorityKey,
            pls: outcome.value.pls,
            cbsem: outcome.value.cbsem,
            authority: outcome.value.authority,
          });
          setFailure(null);
        } catch (error) {
          setNativeEstimatorPreflight(null);
          setFailure(generalSemFailureV1(error));
        }
        return;
      }
      setNativeEstimatorPreflight(null);
      setFailure({ schemaVersion: 1, stage: "capability", subject: "preflight", ...outcome.diagnostic, issues: [] });
    }).catch((error) => {
      if (live && latestAuthorityKeyRef.current === requestedAuthorityKey) {
        setNativeEstimatorPreflight(null);
        setFailure(generalSemFailureV1(error));
      }
    });
    return () => { live = false; };
  }, [authorityKey, config, experimentalLabsEnabled, generalSemSession, markedGeneralSemProjectMode, model, rehydratedExecution, selectedCapabilityCell, services]);

  useEffect(() => {
    if (!markedGeneralSemProjectMode
      || !generalSemSession
      || rehydratedExecution?.status !== "ok"
      || !selectedCapabilityCell) return;
    let live = true;
    const currentReceipt = rehydratedExecution.value.receipt;
    void services.read({
      ...rehydratedExecution.value.readAccess,
      archivePath: generalSemSession.snapshot.archivePath,
      expectedSourceSha256: generalSemSession.snapshot.archiveSha256,
      capabilityCell: selectedCapabilityCell,
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
          message: "The strict result readback differs from the active project authority.",
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
  }, [generalSemSession, markedGeneralSemProjectMode, rehydratedExecution, selectedCapabilityCell, services]);

  useEffect(() => {
    if (generalSemPublicationPending || (!completed && !reopenedEntry) || resultAuthorityCurrent) return;
    setResultIntegrityInvalid(true);
    setReopenedEntry(null);
    setFailure({
      schemaVersion: 1,
      stage: "integrity",
      subject: "active_authority",
      code: "general_sem.completed_authority_stale",
      message: "The displayed result no longer belongs to the active project authority.",
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
      message: "The active project, dataset, model, or calculation configuration changed while estimation was running.",
      correctiveAction: "The job is being cancelled. Save a new calculation project from the intended canvas model and dataset.",
      issues: [],
    });
    void cancelJob(activeJobIdRef.current).then(setSnapshot).catch(() => undefined);
  }, [authorityKey, cancelJob, running]);

  useEffect(() => () => {
    monitorAbortRef.current?.abort();
    const activeJobId = activeJobIdRef.current;
    if (activeJobId) {
      void cancelJob(activeJobId)
        .then((cancelled) => {
          if (!ACTIVE_STATES.has(cancelled.state)) {
            useWorkspace.getState().setGeneralSemTransientWorkBlocker(null);
          }
        })
        .catch(() => undefined);
    }
  }, [cancelJob]);

  const clearResults = () => {
    setCompleted(null);
    setAppendOutcome(null);
    setPersistedArchiveSha256(null);
    setReopenedEntry(null);
    setResultIntegrityInvalid(false);
    setJobRecoveryRequired(false);
    setGeneralSemTransientWorkBlocker(null);
  };

  const createCalculationProject = async () => {
    if (generalSemPublicationPending || !projectActivationConnected || !freshGeneralSemDraftMode || !localPreflight.ready || !model || !config || !sourceProjectId || !dataset.fingerprint) {
      document.getElementById("nd-general-sem-preflight")?.focus();
      return;
    }
    const draftPublicationKey = currentGeneralSemDraftPublicationKeyV1(modelName, selectedEstimatorId);
    const assertDraftPublicationCurrent = () => {
      const current = useWorkspace.getState();
      if (!current.generalSemPublicationPending
        || current.projectId !== sourceProjectId
        || current.projectPath !== workspaceProjectPath
        || current.generalSemProjectDraftMode?.sourceProjectId !== sourceProjectId
         || currentGeneralSemDraftPublicationKeyV1(modelName, selectedEstimatorId) !== draftPublicationKey) {
        throw new Error("The calculation-ready project authority changed while its marked archive was being published.");
      }
    };
    let publishedReceipt: GeneralSemProjectBootstrapReceiptV1 | null = null;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    clearResults();
    try {
      if (!globalThis.crypto?.randomUUID) throw new Error("Secure project and recipe identifiers are unavailable in this runtime.");
      const destination = await services.selectDestination(`${safeFileStem(projectName)}-Calculation.qpls`);
      if (!destination) return;
      assertDraftPublicationCurrent();
      const createdAt = new Date().toISOString();
      const nativeScientificSha256 = await services.scientificDigest(model);
      assertDraftPublicationCurrent();
      const capabilityCell = generalSemRequestedCapabilityCellV1(
        selectedEstimatorId,
        model,
        config,
      );
      const recipe = buildGeneralSemEstimatorRecipeV1(selectedEstimatorId, {
        recipeId: globalThis.crypto.randomUUID(),
        createdAt,
        dataset,
        model,
        nativeScientificSha256,
        config,
        engine: effectiveEngine,
        capabilityCell,
        experimentalLabsEnabled,
      });
      const executionAccess = selectGeneralSemExecutionAccessV1({
        capabilityCell,
        experimentalLabsEnabled,
      });
      const outcome = await services.bootstrapArchive({
        ...executionAccess,
        capabilityCell,
        destinationPath: destination,
        projectId: globalThis.crypto.randomUUID(),
        name: `${projectName} — Calculation`,
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
      const authoritative = await services.nativePreflight({
        project: inspected.value.project,
        modelId: model.id,
        config,
        capabilityCell,
        experimentalLabsEnabled,
      });
      if (authoritative.status === "blocked") {
        throw { schemaVersion: 1, stage: "capability", subject: "preflight", ...authoritative.diagnostic, issues: [] } satisfies GeneralSemPlsJobFailureV1;
      }
      assertDraftPublicationCurrent();
      if (authoritative.value.authority.modelId !== outcome.value.receipt.residentModelId
        || authoritative.value.authority.modelScientificSha256 !== outcome.value.receipt.residentModelScientificSha256) {
        throw new Error("Native estimator preflight did not bind the newly promoted resident SemModelV4 parameter table.");
      }
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
      if (services.adoptActiveProject) {
        try {
          const adopted = await services.adoptActiveProject(createdReceipt.destinationArchivePath);
          if (adopted.projectId !== createdReceipt.projectId) {
            throw new Error("The native project identity differs from the activated calculation-ready revision.");
          }
        } catch (error) {
          // The verified schema-6 session remains usable. Reopening the saved
          // file repairs native-project synchronization before a later revision.
          pushToast({
            tone: "warning",
            title: "Revision activated; reopen before another revision",
            detail: error instanceof Error ? error.message : String(error),
          });
        }
      }
      const activatedExecution = rehydrateGeneralSemExecutionAuthorityV1(inspected.value);
      const activatedModelAuthority = useWorkspace.getState()
        .standardSemModelV4Authorities[createdReceipt.residentModelId];
      if (!activatedModelAuthority) {
        throw new Error("The newly activated scientific model authority is unavailable.");
      }
      const activatedAuthorityKey = generalSemAuthorityKeyV1({
        estimatorId: activatedExecution.estimatorId,
        sourceProjectId: activatedExecution.receipt.projectId,
        datasetId: activatedExecution.receipt.residentDatasetId,
        datasetFingerprint: activatedExecution.receipt.residentDatasetFingerprint,
        modelScientificInput: scientificSemModelV4HashInput(activatedModelAuthority.model),
        config: activatedExecution.config,
        engine: activatedExecution.engine,
      });
      capturedAuthorityKeyRef.current = activatedAuthorityKey;
      latestAuthorityKeyRef.current = activatedAuthorityKey;
      setReceipt(createdReceipt);
      setCurrentArchiveSha256(createdReceipt.destinationArchiveSha256);
      setArchiveSnapshot(inspected.value);
      setNativeEstimatorPreflight({
        authorityKey: activatedAuthorityKey,
        pls: authoritative.value.pls,
        cbsem: authoritative.value.cbsem,
        authority: authoritative.value.authority,
      });
      setSnapshot(null);
      onAuthorityActivated?.();
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
          title: "Calculation-ready project saved but not activated",
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

  const applyMonitorOutcome = (
    outcome: GeneralSemPlsMonitorOutcomeV1 | GeneralSemCbsemMonitorOutcomeV1,
  ): boolean => {
    if (outcome.status === "completed" && generalSemCompletionMatchesLatestAuthorityV1(
      capturedAuthorityKeyRef.current,
      latestAuthorityKeyRef.current ?? "",
    )) {
      const executionAvailable = residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
        ? nativeCbsemExecution !== null
        : nativePlsExecution !== null;
      if (!executionAvailable) {
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
        if (residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1) {
          if (!("adapterVersion" in outcome.completed) || !nativeCbsemExecution) {
            throw new Error("The completed job did not return the exact CB-SEM V3 result contract.");
          }
          validateGeneralSemCbsemCompletedExecutionV1(outcome.completed, nativeCbsemExecution);
        } else {
          if (!("analyticalResult" in outcome.completed) || !nativePlsExecution) {
            throw new Error("The completed job did not return the exact PLS-SEM result contract.");
          }
          validateGeneralSemPlsCompletedExecutionV1(outcome.completed, nativePlsExecution);
        }
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
        message: "The completed result belongs to an earlier Canvas, dataset, or calculation configuration.",
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
  ): Promise<boolean> => residentEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
    ? applyMonitorOutcome(await monitorGeneralSemCbsemJobV1({
      initial,
      getStatus: services.statusCbsem,
      getResult: services.resultCbsem,
      onSnapshot: setSnapshot,
      signal: controller.signal,
    }))
    : applyMonitorOutcome(await monitorGeneralSemPlsJobV1({
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
    if (!receipt || !residentEstimatorId || !currentArchiveSha256 || !model || !config || !nativeEstimatorDecision || !resultAuthorityCurrent || !nativePreflightReady || resultIntegrityInvalid || generalSemSessionDirty || running || generalSemTransientWorkBlocker) return;
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
      const initial = await startJob(generalSemJobRequestForEstimatorFromReceiptV1({
        estimatorId: residentEstimatorId,
        receipt,
        model,
        config,
        decision: nativeEstimatorDecision,
        expectedArchiveSha256,
        experimentalLabsEnabled,
      }));
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
      const current = await statusJob(jobId);
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
      const cancelled = await cancelJob(activeJobIdRef.current);
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
      const cancelled = await cancelJob(jobId);
      if (ACTIVE_STATES.has(cancelled.state)) {
        setSnapshot(cancelled);
        setFailure({
          schemaVersion: 1,
          stage: "estimation",
          subject: jobId,
          code: "general_sem.job_abandon.cancel_pending",
          message: "The advanced calculation is still cancelling.",
          correctiveAction: "Wait for cancellation to become terminal, then retry recovery or abandonment.",
          issues: [],
        });
        setBusy(false);
        return;
      }
      try { await dismissJob(jobId); } catch { /* A consumed or expired terminal job is already gone. */ }
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
      try { await dismissJob(snapshot.jobId); } catch { /* Expired jobs are already clear. */ }
    }
    setSnapshot(null);
    activeJobIdRef.current = null;
    setFailure(null);
    setBusy(false);
    setJobRecoveryRequired(false);
    setGeneralSemTransientWorkBlocker(null);
  };

  const verifyAndReanchorPersistedArchive = async (updatedArchiveSha256: string) => {
    if (!completed) throw new Error("No completed advanced result is available for project verification.");
    if (useInternalProjectArchiveV6Session.getState().dirty) {
      throw new Error("The active Canvas presentation changed after calculation. Restore its saved layout before verifying or appending results.");
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
      throw new Error("The appended project digest differs from its strict reopen identity.");
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
        message: diagnostic?.message ?? "The saved result could not reanchor the active calculation-ready project.",
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
            : "The Canvas presentation differs from the saved project.",
          correctiveAction: !resultAuthorityCurrent
            ? "Reopen the exact marked project or calculate again before saving this result."
            : "Undo the unsaved presentation changes, then save and verify the result.",
          issues: [],
        });
      }
      return;
    }
    if (!completed || !nativeExecution) return;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    try {
      const outcome = await appendGeneralSemResultV1(
        completed,
        nativeExecution,
        experimentalLabsEnabled,
        services.append,
      );
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
    if (!persistedArchiveSha256 || !completed || !nativeExecution || !resultAuthorityCurrent || generalSemPublicationPending) return;
    setBusy(true);
    setGeneralSemPublicationPending(true);
    setFailure(null);
    try {
      await verifyAndReanchorPersistedArchive(persistedArchiveSha256);
      const reopened = await reopenGeneralSemResultV1(
        completed,
        nativeExecution,
        persistedArchiveSha256,
        services.read,
      );
      if (reopened.outcome.status === "blocked" || !reopened.entry) {
        throw reopened.outcome.status === "blocked"
          ? { schemaVersion: 1, stage: "archive_authority", subject: "archive", ...reopened.outcome.diagnostic, issues: [] } satisfies GeneralSemPlsJobFailureV1
          : new Error("The verified project does not contain the appended result.");
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
    if (!completed || !nativeExecution || !persistedArchiveSha256 || !resultAuthorityCurrent) return;
    setBusy(true);
    setFailure(null);
    try {
      const reopened = await reopenGeneralSemResultV1(
        completed,
        nativeExecution,
        persistedArchiveSha256,
        services.read,
      );
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
          message: "The verified QuickPLS project file did not contain the completed result.",
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

  const automaticJourneyKey = calculationPlan
    ? JSON.stringify({
        authorityKey: calculationPlan.authorityKey,
        method: calculationPlan.method,
        inference: calculationPlan.inference,
        config: calculationPlan.requestedConfig,
      })
    : "";
  if (automatedJourneyRef.current.key !== automaticJourneyKey) {
    automatedJourneyRef.current = {
      key: automaticJourneyKey,
      createStarted: false,
      calculationStarted: false,
      appendStarted: false,
      reopenStarted: false,
    };
  }

  // In the unified Calculate workflow the old internal workspace becomes a
  // coordinator: one Start action drives safe revision publication (when
  // required), estimation, append, strict readback, and Results navigation.
  // The visible controls remain available as recovery actions if any stage
  // fails or the native save dialog is cancelled.
  useEffect(() => {
    const journey = automatedJourneyRef.current;
    if (!calculationPresentation || (!calculationPlan && !activationOnly) || failure || operationBusy || running) return;
    if (freshGeneralSemDraftMode && localPreflight.ready && !journey.createStarted) {
      journey.createStarted = true;
      void createCalculationProject();
      return;
    }
    if (!activationOnly && markedGeneralSemProjectMode
      && receipt
      && nativePreflightReady
      && resultAuthorityCurrent
      && !generalSemSessionDirty
      && !completed
      && !snapshot
      && !journey.calculationStarted) {
      journey.calculationStarted = true;
      void start();
      return;
    }
    const persistenceAction = generalSemAutomaticPersistenceNextActionV1({
      completed: Boolean(completed),
      appendSucceeded: appendOutcome?.status === "ok",
      persistedArchiveAvailable: Boolean(persistedArchiveSha256),
      reopened: Boolean(reopenedEntry),
      authorityCurrent: resultAuthorityCurrent,
      sessionDirty: generalSemSessionDirty,
      resultIntegrityInvalid,
      executionReady: nativeExecution !== null,
      appendStarted: journey.appendStarted,
      reopenStarted: journey.reopenStarted,
    });
    if (persistenceAction === "append") {
      journey.appendStarted = true;
      void appendResult();
      return;
    }
    if (persistenceAction === "reopen") {
      journey.reopenStarted = true;
      void reopenResult();
    }
  }, [
    appendOutcome,
    activationOnly,
    calculationPlan,
    calculationPresentation,
    completed,
    failure,
    freshGeneralSemDraftMode,
    generalSemSessionDirty,
    localPreflight.ready,
    markedGeneralSemProjectMode,
    nativePreflightReady,
    nativeExecution,
    operationBusy,
    persistedArchiveSha256,
    receipt,
    reopenedEntry,
    resultAuthorityCurrent,
    resultIntegrityInvalid,
    running,
    snapshot,
  ]);

  const closeGeneralSemProject = async () => {
    if (operationBusy || running) return;
    if (unpersistedCompletedResult) {
      setFailure({
        schemaVersion: 1,
        stage: "archive_authority",
        subject: "result",
        code: "general_sem.project_close.temporary_result_pending",
        message: "A completed result has not passed append and strict readback.",
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
      setNativeEstimatorPreflight(null);
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
  const moderatedMediationPlan = interactionPlan
    && bootstrap
    && (config?.requested_effect_estimands.length ?? 0) === 1;
  const higherOrderPlan = Boolean(model?.derived_terms.some((term) => term.kind === "higher_order"));
  const moderationBootstrapInputDisabled = running
    || operationBusy
    || markedGeneralSemProjectMode;
  const compactCalculationProgress = calculationPresentation
    && Boolean(snapshot || completed || reopenedEntry);

  return <section
    id={calculationPresentation ? "nd-unified-sem-calculation-panel" : "nd-model-general-sem-labs-panel"}
    className={`nd-cbsem-v4-workspace nd-general-sem-workspace${calculationPresentation ? " nd-unified-sem-calculation-workspace" : ""}`}
    role={calculationPresentation ? "region" : "tabpanel"}
    aria-label={calculationPresentation ? "Advanced model calculation setup" : undefined}
    aria-labelledby={calculationPresentation ? undefined : "nd-model-general-sem-labs-tab"}
  >
    <header className="nd-cbsem-v4-header"><div><h2>{calculationPresentation ? "Advanced model calculation" : "General SEM in QuickPLS"}</h2><p>One graphical model · Registry-authorized PLS-SEM and CB-SEM · calculation, progress, Results, export, and reopen in one workflow</p></div><FlaskConical size={24} aria-hidden="true" /></header>
    {!compactCalculationProgress ? <><p className="nd-inline-warning" role="note"><AlertTriangle size={16} aria-hidden="true" /><span>{freshGeneralSemDraftMode
      ? "This new calculation-ready project is not yet activated. Review the detected model and settings, then save and activate its scientific authority before calculating."
      : markedGeneralSemProjectMode
        ? "This canvas is bound to its activated scientific model and calculation recipe."
        : "This older project needs a source-preserving calculation-ready revision before advanced PLS-SEM or CB-SEM methods can run."}</span></p>

    <div className="nd-cbsem-v4-grid">
      <section className="nd-cbsem-v4-card" aria-labelledby="nd-general-sem-input-heading">
        <h3 id="nd-general-sem-input-heading">Scientific input and inference</h3>
        <p className="nd-cbsem-v4-summary"><strong>Project state</strong><span>{freshGeneralSemDraftMode ? "New calculation-ready draft" : markedGeneralSemProjectMode ? "Activated calculation authority" : "Safe revision required"}</span></p>
        <p className="nd-cbsem-v4-summary"><strong>Model</strong><span>{model ? `${modelName} (${model.id})` : freshGeneralSemDraftMode ? "Resolve this new canvas model first" : "Create or activate a calculation-ready revision first"}</span></p>
        <p className="nd-cbsem-v4-summary"><strong>Dataset</strong><span>{dataset.name} · {dataset.rowCount ?? dataset.rows.length} cases</span></p>
        <label htmlFor="nd-general-sem-estimator-recipe">Estimator recipe
          <select
            id="nd-general-sem-estimator-recipe"
            value={selectedEstimatorId}
            disabled={running || operationBusy || markedGeneralSemProjectMode || !freshGeneralSemDraftMode}
            onChange={(event) => {
              if (markedGeneralSemProjectMode) return;
              const next = event.target.value as GeneralSemEstimatorIdV1;
              setDraftEstimatorId(next);
              if (next === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1) {
                setEngine((current) => ({
                  ...current,
                  bootstrapSamples: Math.max(current.bootstrapSamples, 500),
                  confidenceLevel: 0.95,
                }));
              }
              setReceipt(null); setArchiveSnapshot(null); setNativeEstimatorPreflight(null); setFailure(null); clearResults();
            }}
          >
            <option value={GENERAL_SEM_PLS_ESTIMATOR_ID_V1}>PLS-SEM General v3</option>
            <option value={GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1}>CB-SEM General v3 (ML)</option>
          </select>
        </label>
        <p className="nd-inline-warning" role="note">For a fresh draft this chooses which immutable resident RecipeV4 will be published. It does not authorize calculation. Only a matching native <strong>Supported or Experimental</strong> decision with exact Registry evidence enables the action.</p>
        <fieldset className="nd-cbsem-v4-scales"><legend>Inference</legend>
          <label className="nd-checkbox-row" htmlFor="nd-general-sem-bootstrap"><input id="nd-general-sem-bootstrap" type="checkbox" checked={bootstrap} disabled={moderationBootstrapInputDisabled} aria-describedby={interactionPlan && selectedEstimatorId === GENERAL_SEM_PLS_ESTIMATOR_ID_V1 ? "nd-general-sem-moderation-inference-note" : undefined} onChange={(event) => {
            if (markedGeneralSemProjectMode) return;
            setEngine((current) => ({
              ...current,
              inference: event.target.checked ? "percentile_case_bootstrap" : "none",
              bootstrapSamples: selectedEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
                ? Math.max(current.bootstrapSamples, 500)
                : current.bootstrapSamples,
              confidenceLevel: selectedEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
                ? 0.95
                : current.confidenceLevel,
            }));
            setReceipt(null); setArchiveSnapshot(null); setNativeEstimatorPreflight(null); setFailure(null); clearResults();
          }} />Full-model percentile case bootstrap</label>
          {interactionPlan && selectedEstimatorId === GENERAL_SEM_PLS_ESTIMATOR_ID_V1 ? <p id="nd-general-sem-moderation-inference-note" className="nd-inline-warning" role="status">
            {bootstrap
              ? moderatedMediationPlan
                ? "One shared full-model replicate ledger reports scientific gamma, conditional indirect effects at standardized W = -1/0/+1, and the index of moderated mediation. Joint-stage coefficients remain point estimates and no causal interpretation is added."
                : "Full-model case bootstrap reports percentile inference only for each scientific rescaled interaction gamma. Standardized-product coefficients, joint-stage coefficients, fixed -1/0/+1 slopes, and interaction plots remain point estimates; plots do not include confidence bands."
              : "Optional full-model case bootstrap is available for scientific rescaled interaction gamma. Standardized-product coefficients, joint-stage coefficients, fixed -1/0/+1 slopes, and interaction plots remain point-only."}
          </p> : higherOrderPlan ? <p className="nd-inline-warning" role="status">
            {bootstrap
              ? "Full-model case bootstrap reruns every required HOC stage and infers component loadings or weights, authored HOC paths, and extended-repeated total effects."
              : "Point estimation reports the approach-specific HOC stages, component loadings or weights, formative VIF, and authored structural paths."}
          </p> : null}
          {bootstrap ? <>
            <label htmlFor="nd-general-sem-bootstrap-samples">Replicates<input id="nd-general-sem-bootstrap-samples" type="number" min={selectedEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1 ? 500 : 2} max={10_000} step={100} value={effectiveEngine.bootstrapSamples} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, bootstrapSamples: Number(event.target.value) }))} /></label>
            <label htmlFor="nd-general-sem-confidence">Confidence level<input id="nd-general-sem-confidence" type="number" min={0.8} max={0.999} step={0.01} value={effectiveEngine.confidenceLevel} disabled={running || operationBusy || markedGeneralSemProjectMode || selectedEstimatorId === GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1} onChange={(event) => setEngine((current) => ({ ...current, confidenceLevel: Number(event.target.value) }))} /></label>
          </> : null}
          <label htmlFor="nd-general-sem-seed">Seed<input id="nd-general-sem-seed" type="number" min={0} step={1} value={effectiveEngine.seed} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, seed: Number(event.target.value) }))} /></label>
          <label htmlFor="nd-general-sem-workers">Workers<input id="nd-general-sem-workers" type="number" min={1} max={64} step={1} value={effectiveEngine.workers} disabled={running || operationBusy || markedGeneralSemProjectMode} onChange={(event) => setEngine((current) => ({ ...current, workers: Number(event.target.value) }))} /></label>
        </fieldset>
        {markedGeneralSemProjectMode ? <p className="nd-cbsem-v4-summary"><strong>Recipe authority</strong><span>Settings are restored from the resident RecipeV4 and remain immutable for this calculation identity.</span></p> : null}
        {currentNativeEstimatorPreflight ? <GeneralSemEstimatorCompatibilityPanel
          decisions={{ pls: currentNativeEstimatorPreflight.pls, cbsem: currentNativeEstimatorPreflight.cbsem }}
          authority={currentNativeEstimatorPreflight.authority}
          onSelectEstimator={() => undefined}
          selectedEstimatorId={selectedEstimatorId}
          selectionLocked
        /> : <p className="nd-inline-warning" role="status" aria-live="polite">Estimator cards appear only after native preflight binds the promoted resident schema-6 SemModelV4 parameter table. Canvas nodes and edges are not reconstructed for CB-SEM compatibility.</p>}
      </section>

      <section id="nd-general-sem-preflight" className="nd-cbsem-v4-card" tabIndex={-1} aria-labelledby="nd-general-sem-preflight-heading">
        <h3 id="nd-general-sem-preflight-heading">Prepare and verify calculation</h3>
        <ol className="nd-cbsem-v4-preflight-list">
          <li className={localPreflight.ready ? "ready" : "blocked"}><span aria-hidden="true">{localPreflight.ready ? "✓" : "!"}</span><div><strong>Project and model authority</strong><small>{localPreflight.ready ? "Ready for QuickPLS engine verification" : `${localPreflight.issues.length} issue${localPreflight.issues.length === 1 ? "" : "s"}`}</small>{localPreflight.issues.map((item) => <p key={`${item.code}:${item.subject}`}><strong>{item.message}</strong> {item.correctiveAction} <code>{item.code}</code></p>)}</div></li>
          <li className={receipt ? "ready" : "blocked"}><span aria-hidden="true">{receipt ? "✓" : "2"}</span><div><strong>Safe QuickPLS project file</strong><small>{receipt ? `Verified ${receipt.destinationArchivePath}` : "Save the current dataset, canvas model, and analysis settings in one calculation file"}</small>{receipt && !archiveCurrent ? <p>The canvas or settings changed. Keep the saved file unchanged and create a fresh calculation project from the current canvas.</p> : null}</div></li>
          <li className={nativePreflightReady && archiveCurrent ? "ready" : "blocked"}><span aria-hidden="true">{nativePreflightReady && archiveCurrent ? "✓" : "3"}</span><div><strong>QuickPLS engine preflight</strong><small>{nativePreflightReady && archiveCurrent ? nativeCbsemExecution?.kind === "recursive_sem_bootstrap" ? "Exact CB-SEM V3 recursive case-bootstrap cell verified" : nativeCbsemExecution?.kind === "recursive_sem_point" ? "Exact CB-SEM V3 ML point cell verified" : nativePlsExecution?.kind === "two_way_moderated_mediation_bootstrap" ? "Exact Registry-authorized two-way moderated-mediation bootstrap cell verified" : nativePlsExecution?.kind === "higher_order_bootstrap" ? "Exact Registry-authorized HOC bootstrap cell verified" : nativePlsExecution?.kind === "higher_order_point" ? "Exact Registry-authorized HOC point cell verified" : nativePlsExecution?.kind === "multiple_two_way_moderation_bootstrap" ? "Exact simultaneous two-way moderation gamma-bootstrap cell verified" : nativePlsExecution?.kind === "multiple_two_way_moderation_point" ? "Exact simultaneous two-way moderation point cell verified" : "Exact Registry-authorized PLS capability verified" : "The resident RecipeV4 estimator is pending exact Registry-backed engine verification"}</small></div></li>
        </ol>
        <div className="nd-cbsem-v4-actions">
          <button ref={createButtonRef} type="button" className="primary" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || !freshGeneralSemDraftMode || !localPreflight.ready} title={!freshGeneralSemDraftMode ? "Create a source-preserving calculation-ready revision before using this method." : !localPreflight.ready ? "Resolve every compatibility issue first." : "Save and activate this new scientific model and calculation recipe."} onClick={() => void createCalculationProject()}><Archive size={15} aria-hidden="true" />Save and activate project…</button>
          <button type="button" className="primary" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || !receipt || !residentEstimatorId || !resultAuthorityCurrent || !nativePreflightReady || resultIntegrityInvalid || generalSemSessionDirty} title={generalSemSessionDirty ? "Undo unsaved presentation changes before calculating from this fixed archive authority." : !resultAuthorityCurrent ? "Reopen the exact marked project authority before calculating." : !nativePreflightReady ? "The resident RecipeV4 estimator requires its exact Registry-authorized decision." : undefined} onClick={() => void start()}><Play size={15} aria-hidden="true" />{moderatedMediationPlan ? "Calculate moderated-mediation bootstrap" : generalSemCalculationActionLabelV1(interactionPlan, bootstrap, residentEstimatorId ?? selectedEstimatorId, higherOrderPlan)}</button>
          <button type="button" className="danger" disabled={!activeJobIdRef.current || snapshot?.state === "cancelling" || snapshot?.state === "completed"} onClick={() => void cancel()}><CircleStop size={15} aria-hidden="true" />Cancel</button>
          {markedGeneralSemProjectMode ? <button type="button" disabled={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || unpersistedCompletedResult} title={unpersistedCompletedResult ? "Save and strictly reopen the completed result, or dismiss it explicitly, before closing." : undefined} onClick={() => void closeGeneralSemProject()}><FolderOpen size={15} aria-hidden="true" />Close project</button> : null}
        </div>
        {generalSemSessionDirty ? <p className="nd-inline-warning" role="status">The canvas presentation differs from the saved archive. Undo those presentation changes before calculating or appending a result.</p> : null}
      </section>
    </div></> : null}

    {GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1
      && !calculationPresentation
      && (!GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_REQUIRES_LABS_V1 || experimentalLabsEnabled)
      && markedGeneralSemProjectMode
      && model
      && config
      ? <NativeGeneralSemModeratedMediationPanel
          connected={GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1}
          model={model}
          config={config}
          revisionPending={generalSemRevisionPending}
          revisionBlocked={operationBusy || running || Boolean(generalSemTransientWorkBlocker) || generalSemSessionDirty}
          revisionBlockedReason={generalSemSessionDirty
            ? "Restore the exact clean saved archive authority before creating a scientific revision."
            : running
              ? "Wait for the active calculation to reach a terminal state."
              : generalSemTransientWorkBlocker
                ? "Save, strictly reopen, or dismiss the current temporary result before revising its authority."
                : operationBusy
                  ? "Wait for the current project operation to finish."
                  : undefined}
          revisionFailure={generalSemRevisionFailure}
          revisionStatusMessage={generalSemRevisionStatusMessage}
          onSaveAsRevision={async (selection) => {
            const outcome = await reviseGeneralSemModeratedMediationAuthority({ selection });
            if (outcome === "saved") {
              clearResults();
              setFailure(null);
              pushToast({
                tone: "success",
                title: "Moderated-mediation revision activated",
                detail: useInternalProjectArchiveV6Session.getState().revisionForkStatusMessage,
              });
            }
          }}
        />
      : null}

    {snapshot ? <section className="nd-cbsem-v4-card nd-cbsem-v4-monitor" aria-labelledby="nd-general-sem-monitor-heading"><div><h3 id="nd-general-sem-monitor-heading">Calculation progress</h3><span className={`nd-cbsem-v4-state ${snapshot.state}`}>{snapshot.state}</span></div><progress max={progressMaximum} value={progressValue}>{progressValue} of {progressMaximum}</progress><p aria-live="polite" aria-atomic="true">{snapshot.phase}: {snapshot.completedUnits} of {snapshot.totalUnits}</p>{snapshot.state === "failed" || snapshot.state === "cancelled" ? <button type="button" onClick={() => void clearTerminal()}><RotateCcw size={15} aria-hidden="true" />Clear terminal job</button> : null}{jobRecoveryRequired ? <div className="nd-cbsem-v4-actions"><button type="button" className="primary" disabled={busy} onClick={() => void recoverJob()}><RotateCcw size={15} aria-hidden="true" />Retry job recovery</button><button type="button" className="danger" disabled={busy} onClick={() => void abandonJobRecovery()}><CircleStop size={15} aria-hidden="true" />Abandon unrecovered job</button></div> : null}</section> : null}
    {failure ? <div className="nd-cbsem-v4-failure" role="alert"><AlertTriangle size={16} aria-hidden="true" /><div><strong>{failure.message}</strong><p>{failure.correctiveAction}</p><small>{failure.code}</small></div></div> : null}

    {completed ? <section className="nd-cbsem-v4-card nd-cbsem-v4-archive" aria-labelledby="nd-general-sem-persistence-heading">
      <h3 id="nd-general-sem-persistence-heading"><Archive size={16} aria-hidden="true" />Save and verify result</h3>
      <p>The completed result is held temporarily until it is safely written to this QuickPLS calculation project.</p>
      <div className="nd-cbsem-v4-actions">
        <button type="button" className="primary" disabled={operationBusy || appendOutcome?.status === "ok" || !resultAuthorityCurrent || generalSemSessionDirty} onClick={() => void appendResult()}><Archive size={15} aria-hidden="true" />Save result to project</button>
        <button type="button" disabled={operationBusy || !persistedArchiveSha256 || !resultAuthorityCurrent || generalSemSessionDirty} onClick={() => void (resultIntegrityInvalid ? retryPersistedVerification() : reopenResult())}><FolderOpen size={15} aria-hidden="true" />{resultIntegrityInvalid && appendOutcome?.status === "ok" ? "Verify and reanchor saved result" : "Reopen and verify"}</button>
        {unpersistedCompletedResult ? <button type="button" className="danger" disabled={operationBusy} onClick={clearResults}>Dismiss temporary result</button> : null}
      </div>
      {appendOutcome?.status === "ok" ? <p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Saved result {appendOutcome.value.canonical_document_id}.</p> : null}
      {reopenedEntry ? <><p className="nd-cbsem-v4-success" role="status"><CheckCircle2 size={15} aria-hidden="true" />Reopened and verified result {reopenedEntry.documentId}.</p><button type="button" className="primary" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:general-sem-canonical-result", { detail: { document: reopenedEntry.canonicalDocument, navigate: true } }))}>View in Results</button></> : null}
    </section> : null}
    {!completed && reopenedEntry ? <section className="nd-cbsem-v4-card nd-cbsem-v4-archive" aria-labelledby="nd-general-sem-reopened-result-heading"><h3 id="nd-general-sem-reopened-result-heading"><CheckCircle2 size={16} aria-hidden="true" />Verified project result</h3><p>QuickPLS restored the latest matching result from strict project readback.</p><p className="nd-cbsem-v4-success" role="status">Verified result {reopenedEntry.documentId}.</p><button type="button" className="primary" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:general-sem-canonical-result", { detail: { document: reopenedEntry.canonicalDocument, navigate: true } }))}>View in Results</button></section> : null}

    {moderationInventory ? <p className="nd-cbsem-v4-success" role="status" aria-live="polite">
      Verified canonical moderation output: {moderationInventory.interactionEffectCount} interaction effect{moderationInventory.interactionEffectCount === 1 ? "" : "s"}, {moderationInventory.conditionalSlopeCount} conditional slope{moderationInventory.conditionalSlopeCount === 1 ? "" : "s"}, and {moderationInventory.interactionPlotCount} interaction plot{moderationInventory.interactionPlotCount === 1 ? "" : "s"} with {moderationInventory.interactionPlotPointCount} persisted point{moderationInventory.interactionPlotPointCount === 1 ? "" : "s"}. {moderationInventory.combinedModeratedMediation
        ? `The shared ledger also verifies ${moderationInventory.conditionalIndirectCount} fixed-probe conditional indirect effects and ${moderationInventory.moderatedMediationIndexCount} index of moderated mediation using ${moderationInventory.bootstrapResamplesUsable} of ${moderationInventory.bootstrapResamplesRequested} replicates.`
        : moderationInventory.gammaInferenceCount > 0
          ? `Scientific gamma inference is complete for ${moderationInventory.gammaInferenceCount} interaction${moderationInventory.gammaInferenceCount === 1 ? "" : "s"} using ${moderationInventory.bootstrapResamplesUsable} of ${moderationInventory.bootstrapResamplesRequested} bootstrap replicates. Standardized-product coefficients, joint-stage coefficients, slopes, and plots remain point estimates; plots have no confidence bands.`
        : "QuickPLS displays and exports the native canonical point values without adding inference."}
    </p> : null}
    {displayedDocument ? <CanonicalResultDocumentV2View document={displayedDocument} reopened={Boolean(reopenedEntry)} headingRef={resultHeadingRef} compilationReceipt={null} /> : null}
    {displayedDocument ? <CanonicalResultExportPanelV2 document={displayedDocument} /> : null}
    {archiveSnapshot ? <details className="nd-cbsem-v4-run-details"><summary>Calculation project receipt</summary><dl><div><dt>Project</dt><dd>{archiveSnapshot.project.project_id}</dd></div><div><dt>File SHA-256</dt><dd>{currentArchiveSha256 ?? archiveSnapshot.archiveSha256}</dd></div><div><dt>Model</dt><dd>{receipt?.residentModelId}</dd></div><div><dt>Recipe</dt><dd>{receipt?.residentRecipeId}</dd></div></dl></details> : null}
  </section>;
}

export default NativeRecipeV4GeneralSemWorkspace;
