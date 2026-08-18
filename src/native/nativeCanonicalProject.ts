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
import { isNativeRegressionBootstrapValidationWitness } from "./nativeRegressionBootstrapWitness";
import { nativePlsSampleSizePowerRecipeFromCanonical } from "./nativePlsSampleSizePower";
import {
  isNativePlscConsistentPermutationIdentityPresent,
  nativePlscConsistentPermutationProjection,
  nativePlscConsistentPermutationRecipeMatches,
} from "./nativeConsistentPermutation";
import {
  NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION,
  NATIVE_PROCESS_METHOD_VERSION,
  nativeProcessGraphAssessment,
  parseNativeProcessGraph,
} from "./nativeProcess";
import { nativeProcessResultProjection } from "./nativeProcessResults";
import {
  isStructuralPathRandomizationIdentityPresent,
  nativeStructuralPathRandomizationProjection,
  nativeStructuralPathRandomizationRecipeMatches,
} from "./nativeStructuralPathRandomization";
import { nativePlsBootstrapTestTailContractValid } from "./nativeResults";

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
 * Canonical v3 models/results supply the currently executable scientific content.
 * Workspace data supplies presentation plus dormant, versioned SemModelV4
 * authoring intent, which is restored but never executed by this reconciler.
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
    const authoredSemantics = workspaceNodes.get(construct.id)?.data?.semModelV4;
    return {
      id: construct.id,
      type: "construct",
      position: { x: 0, y: 0 },
      data: {
        label: construct.name,
        shortName: construct.short_name,
        mode: construct.mode,
        indicators: [...construct.indicators],
        ...(authoredSemantics ? { semModelV4: authoredSemantics } : {}),
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
      && (edge.data as { role?: unknown } | undefined)?.role !== "covariance"
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

  const constructIds = new Set(model.constructs.map((construct) => construct.id));
  const covariancePairs = new Set<string>();
  const covarianceEdges = workspaceEdges.filter((edge) => {
    if ((edge.data as { role?: unknown } | undefined)?.role !== "covariance"
      || edge.source === edge.target
      || !constructIds.has(edge.source)
      || !constructIds.has(edge.target)) return false;
    const pair = [edge.source, edge.target].sort().join("\0");
    if (covariancePairs.has(pair)) return false;
    covariancePairs.add(pair);
    return true;
  }).map((edge) => ({
    ...edge,
    data: { ...((edge.data && typeof edge.data === "object" && !Array.isArray(edge.data) ? edge.data : {})), role: "covariance" as const },
  }));

  const allEdges = [...edges, ...covarianceEdges];
  const laidOut = layoutModel(baseNodes, edges);
  const nodes = laidOut.map((node) => {
    const position = validPosition(workspaceNodes.get(node.id)?.position);
    return position ? { ...node, position } : node;
  });
  return {
    nodes,
    edges: allEdges,
    diagramLayout: defaultDiagramLayout(nodes, allEdges, presentation.diagramLayout),
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
  if (
    envelope.payload.kind === "pls_sample_size_power_v1"
    || envelope.payload.kind === "pls_sample_size_power_v2"
  ) {
    const isV2 = envelope.payload.kind === "pls_sample_size_power_v2";
    const expectedVersion = isV2 ? "pls_sample_size_power_v2" : "pls_sample_size_power_v1";
    const expectedInference = isV2
      ? "case_bootstrap_null_centered_two_sided_plus_one"
      : "case_bootstrap_normal_reference_two_sided";
    if (
      recipe.schema_version !== 3
      || recipe.settings.method !== "pls_sample_size_power"
      || recipe.method_config?.kind !== "pls_sample_size_power"
      || envelope.provenance.method !== "pls_sample_size_power"
      || recipe.method_config.inference !== expectedInference
      || envelope.provenance.method_version !== expectedVersion
      || envelope.provenance.recipe_id !== recipe.id
      || envelope.provenance.dataset_fingerprint !== recipe.dataset_fingerprint
      || envelope.payload.analysis.method_version !== expectedVersion
      || envelope.payload.analysis.capability_id !== "qpls3.pls.sample_size_power"
    ) return null;
    const method = "PLS-SEM Sample Size and Power Analysis";
    return {
      id: envelope.id,
      modelId: recipe.model.id,
      name: `${method} run`,
      method,
      createdAt: envelope.provenance.completed_at,
      seed: envelope.provenance.seed,
      status: "completed",
      warnings: envelope.diagnostics
        .filter((diagnostic) => diagnostic.level === "warning")
        .map((diagnostic) => diagnostic.message),
      logs: workspaceRun?.logs ?? [{
        id: `run-${envelope.id}-completed`,
        timestamp: envelope.provenance.completed_at,
        phase: "Completed",
        message: `${method} completed successfully.`,
        tone: "success",
      }],
      fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12),
      modelSnapshot: nativeModelSnapshotFromCanonical(recipe.model, {
        nodes: workspaceRun?.modelSnapshot?.nodes,
        edges: workspaceRun?.modelSnapshot?.edges,
        diagramLayout: workspaceRun?.modelSnapshot?.diagramLayout,
      }),
      plsSampleSizePower: envelope.payload.analysis,
      plsSampleSizePowerRecipe: nativePlsSampleSizePowerRecipeFromCanonical(recipe.method_config, recipe.settings),
      provenance: envelope.provenance,
    };
  }
  const canonicalRegression = envelope.payload.estimation.regression;
  const processV2 = envelope.provenance.method_version === NATIVE_PROCESS_METHOD_VERSION
    || envelope.provenance.method_version
      === `${NATIVE_PROCESS_METHOD_VERSION}+${NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION}`;
  if (processV2 && !nativeProcessRecipeMatchesEnvelope(recipe, envelope)) return null;
  const regressionBootstrapPayload = canonicalRegression?.bootstrap;
  const regressionBootstrap = envelope.provenance.method_version.endsWith("+regression_bootstrap_v1");
  if (!regressionBootstrap && regressionBootstrapPayload) return null;
  if (regressionBootstrap) {
    const config = recipe.method_config;
    const estimation = envelope.payload.estimation;
    const regression = canonicalRegression;
    const nestedBootstrap = regression?.bootstrap;
    if (config?.kind !== "regression"
      || typeof config.outcome !== "string" || !config.outcome.trim()
      || !Array.isArray(config.predictors)
      || config.predictors.length < 1
      || config.predictors.some((term) => typeof term !== "string" || !term.trim())
      || (config.controls !== undefined
        && (!Array.isArray(config.controls)
          || config.controls.some((term) => typeof term !== "string" || !term.trim())))
      || !config.model
      || (config.model.type !== "ols" && config.model.type !== "logistic")
      || !config.bootstrap
      || config.bootstrap.algorithm !== "case_resampling"
      || !Array.isArray(config.bootstrap.intervals)
      || config.bootstrap.intervals.length !== 2
      || config.bootstrap.intervals[0] !== "percentile"
      || config.bootstrap.intervals[1] !== "bca") return null;
    const controls = config.controls ?? [];
    const selectedTerms = [...config.predictors, ...controls];
    const expectedTerms = ["intercept", ...selectedTerms];
    const logistic = config.model.type === "logistic";
    const baseMethodVersion = logistic ? "regression_logistic_v2" : "regression_ols_v1";
    if (recipe.schema_version !== 3
      || recipe.settings.method !== "regression"
      || envelope.provenance.method !== "regression"
      || envelope.provenance.method_version !== `${baseMethodVersion}+regression_bootstrap_v1`
      || estimation.method_version !== baseMethodVersion
      || !Number.isInteger(recipe.settings.bootstrap_samples)
      || recipe.settings.bootstrap_samples < 99
      || recipe.settings.bootstrap_samples > 10_000
      || recipe.settings.studentized_inner_samples !== 0
      || recipe.settings.permutation_samples !== 0
      || recipe.settings.confidence_level !== 0.95
      || !Number.isInteger(recipe.settings.workers)
      || recipe.settings.workers < 1
      || recipe.settings.workers > 64
      || recipe.settings.seed !== envelope.provenance.seed
      || envelope.provenance.settings.method !== "regression"
      || envelope.provenance.settings.preprocessing !== "unstandardized"
      || envelope.provenance.settings.bootstrap_samples !== recipe.settings.bootstrap_samples
      || envelope.provenance.settings.studentized_inner_samples !== 0
      || envelope.provenance.settings.permutation_samples !== 0
      || envelope.provenance.settings.confidence_level !== 0.95
      || envelope.provenance.settings.seed !== recipe.settings.seed
      || envelope.provenance.settings.workers !== recipe.settings.workers
      || expectedTerms.length > 51
      || new Set([config.outcome, ...selectedTerms]).size !== selectedTerms.length + 1
      || !regression
      || regression.method_version !== baseMethodVersion
      || regression.regression_type !== (logistic ? "logistic" : "ols")
      || regression.outcome !== config.outcome
      || !Array.isArray(regression.predictors)
      || regression.predictors.length !== config.predictors.length
      || regression.predictors.some((term, index) => term !== config.predictors[index])
      || !Array.isArray(regression.controls)
      || regression.controls.length !== controls.length
      || regression.controls.some((term, index) => term !== controls[index])
      || !nestedBootstrap
      || nestedBootstrap.method_version !== "regression_bootstrap_v1"
      || nestedBootstrap.algorithm !== "indexed_case_resampling_v1"
      || nestedBootstrap.alternative !== "two_sided"
      || nestedBootstrap.interval_policy !== "percentile_primary_bca_conditional_v1"
      || nestedBootstrap.test_reference !== "standard_normal_bootstrap_ratio_v1"
      || nestedBootstrap.test_tolerance_policy !== "64eps_max_1_original_replicates_v1"
      || nestedBootstrap.stream_token !== "quickpls_indexed_resampling_v1"
      || nestedBootstrap.requested_replicates !== recipe.settings.bootstrap_samples
      || !Number.isInteger(nestedBootstrap.usable_replicates)
      || nestedBootstrap.usable_replicates < Math.ceil(0.9 * nestedBootstrap.requested_replicates)
      || nestedBootstrap.usable_replicates > nestedBootstrap.requested_replicates
      || nestedBootstrap.minimum_usable_fraction !== 0.9
      || !Array.isArray(nestedBootstrap.failed_replicates)
      || nestedBootstrap.failed_replicates.length
        !== nestedBootstrap.requested_replicates - nestedBootstrap.usable_replicates
      || nestedBootstrap.confidence_level !== 0.95
      || nestedBootstrap.seed !== recipe.settings.seed
      || nestedBootstrap.workers !== recipe.settings.workers
      || !Number.isInteger(regression.observations)
      || regression.observations < 3
      || nestedBootstrap.jackknife_cases !== regression.observations
      || !Number.isInteger(nestedBootstrap.usable_jackknife_cases)
      || nestedBootstrap.usable_jackknife_cases < 0
      || nestedBootstrap.usable_jackknife_cases > nestedBootstrap.jackknife_cases
      || !isNativeRegressionBootstrapValidationWitness(
        nestedBootstrap.validation_witness,
        expectedTerms,
        nestedBootstrap,
        logistic,
      )
      || envelope.payload.kind !== "pls_pm_v1") return null;
  }
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
  const recipeBootstrapTestTail = recipe.settings.bootstrap_test_tail ?? "two_sided";
  const resultBootstrapTestTail = envelope.provenance.settings.bootstrap_test_tail ?? "two_sided";
  if (recipeBootstrapTestTail !== resultBootstrapTestTail
    || (recipeBootstrapTestTail !== "two_sided" && (
      recipe.schema_version !== 3
      || recipe.settings.method !== "pls_pm"
      || recipe.settings.bootstrap_samples < 1
      || recipe.method_config?.kind !== "pls_bootstrap"
      || !bootstrap
    ))) return null;
  const method = canonicalMethodLabel(envelope, recipe, Boolean(bootstrap), Boolean(permutation));
  const warnings = envelope.diagnostics
    .filter((diagnostic) => diagnostic.level === "warning")
    .map((diagnostic) => diagnostic.message);

  const run: AnalysisRun = {
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
  if (!nativePlsBootstrapTestTailContractValid(run)) return null;
  const hasPlscConsistentPermutationIdentity = isNativePlscConsistentPermutationIdentityPresent(
    recipe,
    envelope,
  );
  if (!hasPlscConsistentPermutationIdentity
    && isStructuralPathRandomizationIdentityPresent(recipe, envelope)) {
    const projection = nativeStructuralPathRandomizationProjection(run);
    if (!projection || !nativeStructuralPathRandomizationRecipeMatches(recipe, envelope, projection)) return null;
  }
  if (hasPlscConsistentPermutationIdentity) {
    const projection = nativePlscConsistentPermutationProjection(run);
    if (!projection || !nativePlscConsistentPermutationRecipeMatches(recipe, envelope, projection)) return null;
  }
  if (processV2 && !nativeProcessResultProjection(run)) return null;
  return run;
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
      || envelope.payload.kind === "pls_sample_size_power_v1"
      || envelope.payload.kind === "pls_sample_size_power_v2"
      ? null
      : envelope.payload.estimation.predict?.method_version ?? null;
    if (version === CURRENT_PLS_PREDICT_METHOD_VERSION) return NATIVE_PREDICTION_METHOD_LABEL;
    if (version === LEGACY_PLS_PREDICT_METHOD_VERSION) return NATIVE_LEGACY_PREDICTION_METHOD_LABEL;
    return version ? `Prediction result (${version})` : "Prediction result";
  }
  if (recipe.settings.method === "regression") {
    const version = envelope.provenance.method_version;
    if (version === `${NATIVE_PROCESS_METHOD_VERSION}+${NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION}`) return "Graph-Defined Path Analysis with Bootstrap";
    if (version === NATIVE_PROCESS_METHOD_VERSION) return "Graph-Defined Path Analysis";
    if (version === "regression_process_v1") return "Historical PROCESS-style regression (v1)";
    if (version === "regression_ols_v1+regression_bootstrap_v1") return "Ordinary Least Squares Regression with Bootstrap";
    if (version === "regression_logistic_v2+regression_bootstrap_v1") return "Binary Logistic Regression with Bootstrap";
    if (version === "regression_logistic_v2") return "Binary Logistic Regression";
    if (version === "regression_logistic_v1") return "Legacy binary logistic regression (v1)";
  }
  if (recipe.settings.method === "pls_pm") {
    if (hasBootstrap || recipe.settings.bootstrap_samples > 0) return "PLS-SEM Bootstrapping";
    if (hasPermutation || recipe.settings.permutation_samples > 0) return "Structural Path Randomization";
    return "PLS-SEM Algorithm";
  }
  if (recipe.settings.method === "plsc" && hasBootstrap
    && envelope.provenance.method_version.split("+").includes("plsc_bootstrap_v1")) {
    return "PLSc Consistent Bootstrapping";
  }
  if (recipe.settings.method === "plsc" && hasPermutation
    && envelope.provenance.method_version.split("+").includes("plsc_permutation_v1")
    && envelope.provenance.method_version.split("+").includes("indexed_group_label_permutation_v1")) {
    return "PLSc Consistent Permutation";
  }
  const descriptor = NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.find((candidate) => (
    candidate.engineMethod === recipe.settings.method
  ));
  return descriptor?.label ?? envelope.provenance.method.replaceAll("_", " ");
}

function nativeProcessRecipeMatchesEnvelope(
  recipe: NativeCanonicalAnalysisRecipe,
  envelope: AnalysisResultEnvelope,
): boolean {
  if (recipe.schema_version !== 3
    || recipe.settings.method !== "regression"
    || recipe.settings.weighting_scheme !== "path"
    || recipe.settings.preprocessing !== "unstandardized"
    || recipe.settings.missing_data !== "listwise_deletion"
    || recipe.settings.case_weight_column !== null
    || recipe.settings.studentized_inner_samples !== 0
    || recipe.settings.permutation_samples !== 0
    || recipe.settings.confidence_level !== 0.95
    || recipe.settings.seed !== envelope.provenance.seed
    || envelope.provenance.method !== "regression"
    || envelope.provenance.settings.method !== "regression"
    || envelope.provenance.settings.seed !== recipe.settings.seed
    || envelope.provenance.settings.preprocessing !== "unstandardized"
    || envelope.provenance.settings.missing_data !== "listwise_deletion"
    || envelope.provenance.settings.case_weight_column !== null
    || envelope.provenance.settings.studentized_inner_samples !== 0
    || envelope.provenance.settings.permutation_samples !== 0
    || envelope.provenance.settings.confidence_level !== 0.95
    || envelope.payload.kind !== "pls_pm_v1") return false;
  const config = recipe.method_config;
  if (config?.kind !== "regression"
    || config.model?.type !== "process"
    || !Array.isArray(config.predictors)
    || config.predictors.length < 1
    || config.predictors.some((value) => typeof value !== "string" || !value.trim())
    || (config.controls !== undefined
      && (!Array.isArray(config.controls)
        || config.controls.some((value) => typeof value !== "string" || !value.trim())))) return false;
  const graph = parseNativeProcessGraph(config.model.relationship);
  if (!graph) return false;
  const controls = config.controls ?? [];
  const assessment = nativeProcessGraphAssessment({
    regressionOutcome: config.outcome,
    regressionPredictors: config.predictors.join(","),
    regressionControls: controls.join(",") || null,
    processGraph: graph,
  } as Parameters<typeof nativeProcessGraphAssessment>[0]);
  const bootstrap = envelope.provenance.method_version
    === `${NATIVE_PROCESS_METHOD_VERSION}+${NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION}`;
  const regression = envelope.payload.estimation.regression;
  const nestedGraph = regression?.process?.graph_v2;
  const recipePathKeys = new Set(graph.paths.map((path) => `${path.from}\u0000${path.to}`));
  const resultPathKeys = new Set(nestedGraph?.paths.map((path) => `${path.from}\u0000${path.to}`) ?? []);
  const moderationKey = (row: { from: string; to: string; moderator: string; conditioning_moderator?: string }) => (
    `${row.from}\u0000${row.to}\u0000${row.moderator}\u0000${row.conditioning_moderator ?? ""}`
  );
  const recipeModerationKeys = new Set(graph.moderations.map(moderationKey));
  const resultModerationKeys = new Set(nestedGraph?.moderations.map(moderationKey) ?? []);
  return assessment.canRun
    && Boolean(assessment.graph)
    && assessment.outcome === config.outcome
    && assessment.predictors.length === config.predictors.length
    && assessment.predictors.every((value, index) => value === config.predictors[index])
    && assessment.controls.length === controls.length
    && assessment.controls.every((value, index) => value === controls[index])
    && regression?.outcome === config.outcome
    && regression.predictors.length === config.predictors.length
    && regression.predictors.every((value, index) => value === config.predictors[index])
    && regression.controls.length === controls.length
    && regression.controls.every((value, index) => value === controls[index])
    && recipePathKeys.size === graph.paths.length
    && resultPathKeys.size === recipePathKeys.size
    && [...recipePathKeys].every((key) => resultPathKeys.has(key))
    && recipeModerationKeys.size === graph.moderations.length
    && resultModerationKeys.size === recipeModerationKeys.size
    && [...recipeModerationKeys].every((key) => resultModerationKeys.has(key))
    && graph.moderators.every((moderator) => nestedGraph?.variable_profiles.some((profile) => (
      profile.variable === moderator.variable
      && profile.role === "moderator"
      && profile.scale === moderator.scale
    )))
    && recipe.settings.bootstrap_samples === (bootstrap ? nestedGraph?.bootstrap?.requested_replicates : 0)
    && recipe.settings.workers === (bootstrap ? nestedGraph?.bootstrap?.workers : 1)
    && envelope.provenance.settings.bootstrap_samples === recipe.settings.bootstrap_samples
    && envelope.provenance.settings.workers === recipe.settings.workers
    && (bootstrap
      ? config.bootstrap?.algorithm === "case_resampling"
        && Array.isArray(config.bootstrap.intervals)
        && config.bootstrap.intervals.length === 2
        && config.bootstrap.intervals[0] === "percentile"
        && config.bootstrap.intervals[1] === "bca"
      : config.bootstrap === undefined)
    && envelope.payload.estimation.method_version === NATIVE_PROCESS_METHOD_VERSION
    && envelope.payload.estimation.regression?.method_version === NATIVE_PROCESS_METHOD_VERSION
    && envelope.payload.estimation.regression?.regression_type === "process"
    && envelope.payload.estimation.regression?.bootstrap == null
    && nestedGraph != null;
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
