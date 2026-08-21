import type { Edge, Node } from "@xyflow/react";
import type {
  AnalysisRun,
  AnalysisUiSettings,
  ConstructData,
  Dataset,
  DatasetVersionRecord,
  DiagramLayoutState,
  DiagramMode,
  DiagramOverlaySettings,
  NativeCanonicalModelSpec,
  NativeModelPresentation,
  NativeSavedReport,
  PublicationDiagramSettings,
} from "../types";
import { buildNativeRecipeModel } from "./nativeAnalysisRecipe";

export type NativeLegacyProjectOperation =
  | "schema5_save"
  | "schema5_autosave"
  | "calculation"
  | "legacy_graph_serialization";

export interface NativeLegacyProjectOperationGate {
  standardSemModelV4OperationBlocker: (operation: NativeLegacyProjectOperation) => string | null;
}

/**
 * Central fail-closed gate for workflows that still derive scientific content
 * from the legacy canvas graph. The operation-specific check explains the user
 * action while the serialization check prevents an accidental graph fallback.
 */
export function nativeLegacyProjectOperationBlocker(
  gate: NativeLegacyProjectOperationGate,
  operation: Exclude<NativeLegacyProjectOperation, "legacy_graph_serialization">,
) {
  const operationBlocker = gate.standardSemModelV4OperationBlocker(operation);
  const serializationBlocker = gate.standardSemModelV4OperationBlocker("legacy_graph_serialization");
  return operationBlocker ?? serializationBlocker;
}

export interface NativeProjectSignatureInput {
  dataset: Dataset;
  datasetCatalog: Dataset[];
  datasetVersions: DatasetVersionRecord[];
  projectModels: NativeCanonicalModelSpec[];
  activeModelId: string | null;
  modelPresentations: Record<string, NativeModelPresentation>;
  savedReports: NativeSavedReport[];
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  runs: AnalysisRun[];
  analysisSettings: AnalysisUiSettings;
  diagramMode: DiagramMode;
  diagramOverlaySettings: DiagramOverlaySettings;
  publicationDiagramSettings: PublicationDiagramSettings;
  diagramLayout: DiagramLayoutState;
}

export interface NativeStandardAuthorityDirtyState {
  /** A strict SemModelV4 authority, rather than the projected graph, owns science. */
  active: boolean;
  /** Compares canonical document/layout state with its validated persistence anchor. */
  dirty: boolean;
  /** Native activation or validated save-copy work must not be discarded mid-flight. */
  operationPending: boolean;
}

export function nativeSchema6BoundWorkspaceReplacementBlocker(
  sourceBound: boolean,
  dirty: boolean,
): string | null {
  if (!sourceBound) return null;
  return dirty
    ? "Save a validated new copy, then close the calculation-ready project"
    : "Close the calculation-ready project to release the schema-6 source binding";
}

function persistedDatasetSignature(dataset: Dataset) {
  return {
    id: dataset.id,
    name: dataset.name,
    fingerprint: dataset.fingerprint ?? null,
    kind: dataset.kind ?? "raw",
    sampleSize: dataset.sampleSize ?? null,
    rowCount: dataset.rowCount ?? dataset.rows.length,
    missing: dataset.missing,
    missingByColumn: dataset.missingByColumn ?? {},
    columns: dataset.columns,
    columnMetadata: dataset.columnMetadata ?? [],
  };
}

function persistedNodeSignature(node: Node<ConstructData>) {
  return {
    id: node.id,
    type: node.type ?? null,
    position: node.position,
    data: node.data,
    parentId: node.parentId ?? null,
    extent: node.extent ?? null,
    hidden: node.hidden ?? false,
    style: node.style ?? null,
  };
}

function persistedEdgeSignature(edge: Edge) {
  return {
    id: edge.id,
    source: edge.source,
    target: edge.target,
    sourceHandle: edge.sourceHandle ?? null,
    targetHandle: edge.targetHandle ?? null,
    type: edge.type ?? null,
    label: edge.label ?? null,
    data: edge.data ?? null,
    markerStart: edge.markerStart ?? null,
    markerEnd: edge.markerEnd ?? null,
    style: edge.style ?? null,
    hidden: edge.hidden ?? false,
  };
}

function persistedPresentationNodeSignature(node: Node<ConstructData>) {
  const { data: _scientificContent, ...persisted } = persistedNodeSignature(node);
  return { ...persisted, semModelV4: node.data?.semModelV4 ?? null };
}

function persistedPresentationEdgeSignature(edge: Edge) {
  const { data: _scientificContent, ...persisted } = persistedEdgeSignature(edge);
  const data = edge.data && typeof edge.data === "object" && !Array.isArray(edge.data)
    ? edge.data as Record<string, unknown>
    : null;
  return { ...persisted, semModelV4: data?.semModelV4 ?? null };
}

function persistedModelPresentationSignature(presentation: NativeModelPresentation) {
  return {
    nodes: (presentation.nodes ?? []).map(persistedPresentationNodeSignature),
    edges: (presentation.edges ?? []).map(persistedPresentationEdgeSignature),
    diagramLayout: presentation.diagramLayout ?? null,
  };
}

/**
 * Builds a stable, inexpensive representation of the state persisted with a project.
 * Transient React Flow state such as selection, hover, measured dimensions, and drag
 * flags is intentionally excluded. The live canvas is folded into its canonical model
 * and presentation; versioned SemModelV4 authoring intent remains part of the saved
 * signature. Changing the active Explorer selection alone never marks the
 * project dirty while edits to any model or saved report still do.
 */
export function nativeProjectSignature(input: NativeProjectSignatureInput) {
  const activeModel = input.activeModelId
    ? input.projectModels.find((model) => model.id === input.activeModelId)
    : null;
  const projectModels = activeModel
    ? input.projectModels.map((model) => model.id === activeModel.id
      ? buildNativeRecipeModel(model.id, model.name, input.nodes, input.edges)
      : model)
    : input.projectModels;
  const modelPresentations = activeModel
    ? {
        ...input.modelPresentations,
        [activeModel.id]: {
          nodes: input.nodes,
          edges: input.edges,
          diagramLayout: input.diagramLayout,
        },
      }
    : input.modelPresentations;

  return JSON.stringify({
    activeDatasetId: input.dataset.id,
    dataset: persistedDatasetSignature(input.dataset),
    datasetCatalog: input.datasetCatalog.map(persistedDatasetSignature),
    datasetVersions: input.datasetVersions.map((version) => ({
      datasetId: version.datasetId,
      parentDatasetId: version.parentDatasetId,
      operation: version.operation,
      createdAt: version.createdAt,
      summary: version.summary,
      sourceColumn: version.sourceColumn,
      targetColumn: version.targetColumn,
      transformation: version.transformation ?? null,
    })),
    projectModels: [...projectModels].sort((left, right) => left.id.localeCompare(right.id)),
    modelPresentations: Object.fromEntries(Object.entries(modelPresentations)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([modelId, presentation]) => [modelId, persistedModelPresentationSignature(presentation)])),
    savedReports: [...input.savedReports]
      .sort((left, right) => left.resultId.localeCompare(right.resultId))
      .map((report) => ({ resultId: report.resultId, name: report.name, savedAt: report.savedAt })),
    legacyModel: activeModel ? null : {
      nodes: input.nodes.map(persistedNodeSignature),
      edges: input.edges.map(persistedEdgeSignature),
      diagramLayout: input.diagramLayout,
    },
    runs: input.runs.map((run) => ({
      id: run.id,
      modelId: run.modelId ?? null,
      name: run.name,
      method: run.method,
      createdAt: run.createdAt,
      seed: run.seed,
      status: run.status,
      warnings: run.warnings,
      fingerprint: run.fingerprint,
    })),
    analysisSettings: input.analysisSettings,
    diagramMode: input.diagramMode,
    diagramOverlaySettings: input.diagramOverlaySettings,
    publicationDiagramSettings: input.publicationDiagramSettings,
  });
}

/**
 * Reconciles a workspace captured before Save with the authoritative dataset
 * catalogue returned by the desktop backend. This keeps concurrent model edits
 * dirty while accepting lineage that the backend added during import/metadata
 * operations as part of the just-completed save.
 */
export function nativeSavedProjectSignature(
  input: NativeProjectSignatureInput,
  datasetCatalog: Dataset[],
  datasetVersions: DatasetVersionRecord[],
  activeDatasetId: string,
  explorer?: {
    projectModels: NativeCanonicalModelSpec[];
    activeModelId: string | null;
    modelPresentations: Record<string, NativeModelPresentation>;
    savedReports: NativeSavedReport[];
  },
) {
  return nativeProjectSignature({
    ...input,
    dataset: datasetCatalog.find((dataset) => dataset.id === activeDatasetId) ?? input.dataset,
    datasetCatalog,
    datasetVersions,
    ...(explorer ?? {}),
  });
}

export function hasUnsavedNativeProjectChanges(
  hasOpenProject: boolean,
  cleanSignature: string | null,
  currentSignature: string,
  standardAuthority?: NativeStandardAuthorityDirtyState,
) {
  if (standardAuthority?.operationPending) return true;
  if (standardAuthority?.active) return hasOpenProject && standardAuthority.dirty;
  return hasOpenProject && cleanSignature !== null && cleanSignature !== currentSignature;
}
