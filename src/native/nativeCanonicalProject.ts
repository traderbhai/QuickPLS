import type { Edge, Node, XYPosition } from "@xyflow/react";
import { defaultDiagramLayout } from "../domain/diagramGraph";
import { layoutModel } from "../domain/modelLayout";
import type {
  AnalysisModelSnapshot,
  AnalysisResultEnvelope,
  AnalysisRun,
  ConstructData,
  DiagramLayoutState,
  NativeExplorerSelection,
  NativeCanonicalAnalysisRecipe,
  NativeCanonicalModelSpec,
  NativeModelPresentation,
  NativeProjectSnapshot,
  NativeSavedReport,
  NativeWorkspaceRunPresentation,
} from "../types";
import { NATIVE_ANALYSIS_RECIPE_DESCRIPTORS } from "./nativeAnalysisRecipe";
import {
  CURRENT_PLS_PREDICT_METHOD_VERSION,
  LEGACY_PLS_PREDICT_METHOD_VERSION,
  NATIVE_LEGACY_PREDICTION_METHOD_LABEL,
  NATIVE_PREDICTION_METHOD_LABEL,
} from "./nativeCalculationMode";
import { isStandaloneNativeAnalysis } from "./nativeStandaloneAnalysis";

export interface NativeCanonicalProjectState {
  activeModelId: string | null;
  projectModels: NativeCanonicalModelSpec[];
  modelPresentations: Record<string, NativeModelPresentation>;
  savedReports: NativeSavedReport[];
  explorerSelection: NativeExplorerSelection;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  diagramLayout?: DiagramLayoutState;
  runs: AnalysisRun[];
  modelSource: "canonical" | "workspace" | "empty";
  resultSource: "canonical" | "workspace" | "empty";
}

interface NativeWorkspacePresentation {
  nodes?: unknown[];
  edges?: unknown[];
  runs?: Array<AnalysisRun | NativeWorkspaceRunPresentation>;
  diagramLayout?: Partial<DiagramLayoutState>;
  activeModelId?: string;
}

/**
 * Reconciles the validated project records with the opaque workspace layout.
 * Canonical models/results always supply scientific content. Workspace data is
 * used only for presentation (positions, routing, and legacy-only archives).
 */
export function reconcileNativeCanonicalProject(
  project: Pick<NativeProjectSnapshot, "models" | "recipes" | "results" | "activeModelId" | "modelPresentations" | "savedReports" | "workspace">,
): NativeCanonicalProjectState {
  const workspace = (project.workspace ?? null) as NativeWorkspacePresentation | null;
  const models = project.models ?? [];
  const recipes = project.recipes ?? [];
  const results = project.results ?? [];
  const modelPresentations = normalizedModelPresentations(models, project.modelPresentations);
  const savedReports = [...(project.savedReports ?? [])];
  const activeModel = resolveActiveCanonicalModel(
    models,
    recipes,
    results,
    project.activeModelId ?? workspace?.activeModelId ?? null,
  );

  const workspaceNodes = readableWorkspaceNodes(workspace?.nodes);
  const workspaceEdges = readableWorkspaceEdges(workspace?.edges);
  const workspaceRuns = Array.isArray(workspace?.runs) ? workspace.runs : [];
  const activePresentation = activeModel ? modelPresentations[activeModel.id] : undefined;
  const modelSnapshot = activeModel
    ? nativeModelSnapshotFromCanonical(activeModel, {
        nodes: Array.isArray(activePresentation?.nodes)
          ? readableModelPresentationNodes(activePresentation.nodes)
          : workspaceNodes,
        edges: Array.isArray(activePresentation?.edges)
          ? readableWorkspaceEdges(activePresentation.edges)
          : workspaceEdges,
        diagramLayout: activePresentation?.diagramLayout ?? workspace?.diagramLayout,
      })
    : null;
  const canonicalRuns = results.length
    ? nativeRunsFromCanonicalResults(results, recipes, workspaceRuns)
    : [];

  if (activeModel) {
    return {
      activeModelId: activeModel.id,
      projectModels: [...models],
      modelPresentations,
      savedReports,
      explorerSelection: { kind: "model", modelId: activeModel.id },
      nodes: modelSnapshot!.nodes,
      edges: modelSnapshot!.edges,
      diagramLayout: modelSnapshot!.diagramLayout,
      runs: results.length ? canonicalRuns : workspaceRuns,
      modelSource: "canonical",
      resultSource: results.length ? "canonical" : workspaceRuns.length ? "workspace" : "empty",
    };
  }

  if (workspaceNodes.length || workspaceEdges.length) {
    return {
      activeModelId: null,
      projectModels: [...models],
      modelPresentations,
      savedReports,
      explorerSelection: { kind: "models" },
      nodes: workspaceNodes,
      edges: workspaceEdges,
      diagramLayout: workspace?.diagramLayout as DiagramLayoutState | undefined,
      runs: results.length ? canonicalRuns : workspaceRuns,
      modelSource: "workspace",
      resultSource: results.length ? "canonical" : workspaceRuns.length ? "workspace" : "empty",
    };
  }

  return {
    activeModelId: null,
    projectModels: [...models],
    modelPresentations,
    savedReports,
    explorerSelection: models.length ? { kind: "models" } : { kind: "data" },
    nodes: [],
    edges: [],
    diagramLayout: undefined,
    runs: results.length ? canonicalRuns : workspaceRuns,
    modelSource: "empty",
    resultSource: results.length ? "canonical" : workspaceRuns.length ? "workspace" : "empty",
  };
}

export function resolveActiveCanonicalModel(
  models: readonly NativeCanonicalModelSpec[],
  recipes: readonly NativeCanonicalAnalysisRecipe[],
  results: readonly AnalysisResultEnvelope[],
  requestedModelId: string | null,
): NativeCanonicalModelSpec | null {
  const modelBoundRecipes = recipes.filter((recipe) => !isStandaloneNativeAnalysis(recipe.settings.method));
  const allModels = uniqueModels([...models, ...modelBoundRecipes.map((recipe) => recipe.model)]);
  if (requestedModelId) {
    const requested = allModels.find((model) => model.id === requestedModelId);
    if (requested) return requested;
  }

  const newestResult = [...results]
    .filter((result) => result.status === "completed"
      && modelBoundRecipes.some((recipe) => recipe.id === result.provenance.recipe_id))
    .sort((left, right) => right.provenance.completed_at.localeCompare(left.provenance.completed_at))[0];
  if (newestResult) {
    const recipe = modelBoundRecipes.find((candidate) => candidate.id === newestResult.provenance.recipe_id);
    if (recipe) return recipe.model;
  }
  if (models.length === 1) return models[0];
  if (allModels.length === 1) return allModels[0];
  return null;
}

export function nativeModelSnapshotFromCanonical(
  model: NativeCanonicalModelSpec,
  presentation: {
    nodes?: Array<Node<ConstructData>>;
    edges?: Edge[];
    diagramLayout?: Partial<DiagramLayoutState>;
  } = {},
): AnalysisModelSnapshot {
  const workspaceNodes = new Map((presentation.nodes ?? []).map((node) => [node.id, node]));
  const workspaceEdges = presentation.edges ?? [];
  const interactions = new Map(model.interactions.map((interaction) => [interaction.product_construct, interaction]));
  const higherOrder = new Map(model.higher_order_constructs.map((construct) => [construct.id, construct]));
  const controls = new Map(model.controls.map((control) => [pathKey(control.source, control.target), control]));

  const baseNodes: Array<Node<ConstructData>> = model.constructs.map((construct) => {
    const interaction = interactions.get(construct.id);
    const higher = higherOrder.get(construct.id);
    return {
      id: construct.id,
      type: "construct",
      position: { x: 0, y: 0 },
      data: {
        label: construct.name,
        shortName: construct.short_name,
        mode: construct.mode,
        indicators: [...construct.indicators],
        ...(interaction ? {
          semantic: "interaction" as const,
          interaction: {
            predictor: interaction.predictor,
            moderator: interaction.moderator,
            outcome: interaction.outcome,
            method: interaction.method,
          },
        } : {}),
        ...(higher ? {
          semantic: "higher_order" as const,
          higherOrder: {
            id: higher.id,
            components: [...higher.components],
            method: higher.method,
            stage_one_recipe: higher.stage_one_recipe,
          },
        } : {}),
      },
    };
  });

  const edges: Edge[] = model.paths.map((path, index) => {
    const control = controls.get(pathKey(path.source, path.target));
    const workspaceEdge = workspaceEdges.find((edge) => (
      edge.source === path.source
      && edge.target === path.target
      && !edge.id.startsWith("measurement::")
    ));
    const data: Record<string, unknown> | undefined = control
      ? { role: "control", controlLabel: control.label }
      : undefined;
    return {
      id: workspaceEdge?.id ?? `path::${encodeURIComponent(path.source)}::${encodeURIComponent(path.target)}::${index}`,
      source: path.source,
      target: path.target,
      type: workspaceEdge?.type ?? "smoothstep",
      label: control?.label?.trim() || (control ? "Control" : workspaceEdge?.label ?? "Path"),
      ...(data ? { data } : {}),
    };
  });

  const laidOut = layoutModel(baseNodes, edges);
  const nodes = laidOut.map((node) => {
    const position = validPosition(workspaceNodes.get(node.id)?.position);
    return position ? { ...node, position } : node;
  });
  return {
    nodes,
    edges,
    diagramLayout: defaultDiagramLayout(nodes, edges, presentation.diagramLayout),
  };
}

export function nativeRunsFromCanonicalResults(
  results: readonly AnalysisResultEnvelope[],
  recipes: readonly NativeCanonicalAnalysisRecipe[],
  workspaceRuns: readonly (AnalysisRun | NativeWorkspaceRunPresentation)[] = [],
): AnalysisRun[] {
  const recipeById = new Map(recipes.map((recipe) => [recipe.id, recipe]));
  const workspaceRunById = new Map(workspaceRuns.map((run) => [run.id, run]));
  return results.flatMap((envelope) => {
    const recipe = recipeById.get(envelope.provenance.recipe_id);
    if (!recipe) return [];
    const run = nativeRunFromCanonicalResult(envelope, recipe, workspaceRunById.get(envelope.id));
    return run ? [run] : [];
  });
}

export function nativeRunFromCanonicalResult(
  envelope: AnalysisResultEnvelope,
  recipe: NativeCanonicalAnalysisRecipe,
  workspaceRun?: AnalysisRun | NativeWorkspaceRunPresentation,
): AnalysisRun | null {
  if (envelope.status !== "completed" || envelope.payload.kind === "legacy") return null;
  const standalone = isStandaloneNativeAnalysis(recipe.settings.method);
  const modelSnapshot = standalone ? undefined : nativeModelSnapshotFromCanonical(recipe.model, {
    nodes: workspaceRun?.modelSnapshot?.nodes,
    edges: workspaceRun?.modelSnapshot?.edges,
    diagramLayout: workspaceRun?.modelSnapshot?.diagramLayout,
  });
  const bootstrap = envelope.payload.kind === "pls_pm_v2"
    ? envelope.payload.bootstrap
    : envelope.payload.kind === "pls_pm_v3"
      ? envelope.payload.bootstrap ?? undefined
      : undefined;
  const permutation = envelope.payload.kind === "pls_pm_v3"
    ? envelope.payload.permutation ?? undefined
    : undefined;
  const method = canonicalMethodLabel(envelope, recipe, Boolean(bootstrap), Boolean(permutation));
  const warnings = envelope.diagnostics
    .filter((diagnostic) => diagnostic.level === "warning")
    .map((diagnostic) => diagnostic.message);

  return {
    id: envelope.id,
    modelId: standalone ? null : recipe.model.id,
    name: `${method} run`,
    method,
    createdAt: envelope.provenance.completed_at,
    seed: envelope.provenance.seed,
    status: "completed",
    warnings,
    logs: workspaceRun?.logs ?? [{
      id: `run-${envelope.id}-completed`,
      timestamp: envelope.provenance.completed_at,
      phase: "Completed",
      message: `${method} completed successfully.`,
      tone: "success",
    }],
    fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12),
    ...(modelSnapshot ? { modelSnapshot } : {}),
    result: envelope.payload.estimation,
    assessment: envelope.payload.assessment,
    bootstrap,
    permutation,
    provenance: envelope.provenance,
  };
}

export function compactNativeWorkspaceRuns(
  runs: readonly AnalysisRun[],
): NativeWorkspaceRunPresentation[] {
  return runs.map((run) => ({
    id: run.id,
    ...(run.modelId !== undefined ? { modelId: run.modelId } : {}),
    name: run.name,
    method: run.method,
    createdAt: run.createdAt,
    seed: run.seed,
    status: run.status,
    warnings: [...run.warnings],
    ...(run.logs ? { logs: run.logs } : {}),
    fingerprint: run.fingerprint,
    ...(run.modelSnapshot ? { modelSnapshot: run.modelSnapshot } : {}),
  }));
}

export function currentNativeModelPresentation(
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
  diagramLayout: DiagramLayoutState,
): NativeModelPresentation {
  return { nodes, edges, diagramLayout };
}

function canonicalMethodLabel(
  envelope: AnalysisResultEnvelope,
  recipe: NativeCanonicalAnalysisRecipe,
  hasBootstrap: boolean,
  hasPermutation: boolean,
) {
  if (recipe.settings.method === "predict") {
    const version = envelope.payload.kind === "legacy"
      ? null
      : envelope.payload.estimation.predict?.method_version ?? null;
    if (version === CURRENT_PLS_PREDICT_METHOD_VERSION) return NATIVE_PREDICTION_METHOD_LABEL;
    if (version === LEGACY_PLS_PREDICT_METHOD_VERSION) return NATIVE_LEGACY_PREDICTION_METHOD_LABEL;
    return version ? `Prediction result (${version})` : "Prediction result";
  }
  if (recipe.settings.method === "pls_pm") {
    if (hasBootstrap || recipe.settings.bootstrap_samples > 0) return "PLS-SEM Bootstrapping";
    if (hasPermutation || recipe.settings.permutation_samples > 0) return "Structural Path Randomization";
    return "PLS-SEM Algorithm";
  }
  const descriptor = NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.find((candidate) => (
    candidate.engineMethod === recipe.settings.method
  ));
  return descriptor?.label ?? envelope.provenance.method.replaceAll("_", " ");
}

function uniqueModels(models: readonly NativeCanonicalModelSpec[]) {
  const byId = new Map<string, NativeCanonicalModelSpec>();
  for (const model of models) if (!byId.has(model.id)) byId.set(model.id, model);
  return [...byId.values()];
}

function normalizedModelPresentations(
  models: readonly NativeCanonicalModelSpec[],
  presentations: Record<string, NativeModelPresentation> | undefined,
) {
  const knownModelIds = new Set(models.map((model) => model.id));
  return Object.fromEntries(Object.entries(presentations ?? {}).filter(([modelId, presentation]) => (
    knownModelIds.has(modelId)
    && Boolean(presentation)
    && typeof presentation === "object"
    && !Array.isArray(presentation)
  )));
}

function readableWorkspaceNodes(value: unknown): Array<Node<ConstructData>> {
  if (!Array.isArray(value)) return [];
  return value.filter((candidate): candidate is Node<ConstructData> => {
    if (!candidate || typeof candidate !== "object") return false;
    const node = candidate as Partial<Node<ConstructData>>;
    return typeof node.id === "string"
      && Boolean(node.data)
      && typeof node.data?.label === "string"
      && Array.isArray(node.data?.indicators)
      && Boolean(validPosition(node.position));
  });
}

function readableModelPresentationNodes(value: unknown): Array<Node<ConstructData>> {
  if (!Array.isArray(value)) return [];
  return value.filter((candidate): candidate is Node<ConstructData> => {
    if (!candidate || typeof candidate !== "object") return false;
    const node = candidate as Partial<Node<ConstructData>>;
    return typeof node.id === "string" && Boolean(validPosition(node.position));
  });
}

function readableWorkspaceEdges(value: unknown): Edge[] {
  if (!Array.isArray(value)) return [];
  return value.filter((candidate): candidate is Edge => {
    if (!candidate || typeof candidate !== "object") return false;
    const edge = candidate as Partial<Edge>;
    return typeof edge.id === "string"
      && typeof edge.source === "string"
      && typeof edge.target === "string";
  });
}

function validPosition(value: unknown): XYPosition | null {
  if (!value || typeof value !== "object") return null;
  const position = value as Partial<XYPosition>;
  return Number.isFinite(position.x) && Number.isFinite(position.y)
    ? { x: Number(position.x), y: Number(position.y) }
    : null;
}

function pathKey(source: string, target: string) {
  return `${source}\u0000${target}`;
}
