import type {
  AnalysisUiSettings,
  Dataset,
  DatasetRowsPage,
  NativeProcessGraphRelationshipConfig,
  NativeProcessModerationConfig,
  NativeProcessModeratorConfig,
  NativeProcessPathConfig,
} from "../types";
import { getNativeDatasetRows } from "../services/projectService";
import { nativeOlsCsvValues, nativeOlsNumericColumns } from "./nativeOls";

export const NATIVE_PROCESS_METHOD_VERSION = "regression_process_v2" as const;
export const NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION = "regression_process_bootstrap_v1" as const;
export const NATIVE_PROCESS_MAX_MEDIATORS = 4;
export const NATIVE_PROCESS_MAX_MODERATORS = 2;
export const NATIVE_PROCESS_MAX_PREDICTORS = 8;
export const NATIVE_PROCESS_MAX_CONTROLS = 1;
export const NATIVE_PROCESS_MAX_PATHS = 16;
export const NATIVE_PROCESS_MAX_MODERATIONS = 4;
export const NATIVE_PROCESS_MAX_EQUATION_TERMS = 50;
export const NATIVE_PROCESS_PROFILE_PAGE_SIZE = 500;
export const NATIVE_PROCESS_CENTERING_POLICY = "equation_complete_case_mean_v1" as const;
export const NATIVE_PROCESS_SCOPE_NOTE =
  "Graph-defined observed-variable path analysis with raw listwise-complete OLS equations, HC3 covariance, fixed two-sided 95% Student-t inference, parallel and serial mediation, continuous or exact 0/1 moderation, mixed two-moderator interactions, first- or second-stage moderated mediation, simple slopes, and Johnson-Neyman regions where applicable. This release supports up to 8 selected predictors in graph-role order and one control entered in every equation; the 50-term ceiling is an equation-design safety bound. Continuous product participants are centered within each equation sample. Numbered macros, binary outcomes, weights, clusters, custom alpha or tails, studentized intervals, multiple moderated stages on one indirect path, and three-way interactions on mediated paths are excluded.";
export const NATIVE_PROCESS_RESULT_WARNING =
  "PROCESS v2 is an independently implemented graph-defined observed-variable path-analysis workflow; it does not execute copied numbered templates.";
export const NATIVE_PROCESS_INFERENCE_WARNING =
  "PROCESS v2 uses raw listwise-complete OLS equations with HC3 covariance and fixed two-sided 95% Student-t inference; unsupported shapes are rejected.";

export interface NativeProcessGraphAssessment {
  canRun: boolean;
  blockers: string[];
  outcome: string;
  focalPredictor: string;
  mediators: string[];
  moderators: NativeProcessModeratorConfig[];
  controls: string[];
  predictors: string[];
  graph: NativeProcessGraphRelationshipConfig | null;
  equationTermCounts: Array<{ outcome: string; terms: number }>;
  detail: string;
}

export interface NativeProcessProfile {
  datasetId: string;
  datasetFingerprint: string;
  selectionToken: string;
  variables: string[];
  binaryModerators: string[];
  expectedRows: number;
  scannedRows: number;
  completeCases: number;
  omittedRows: number;
  invalidBinaryRows: Record<string, number>;
  binaryEquationOutcomes: string[];
  constantVariables: string[];
}

export interface NativeProcessReadinessAssessment extends NativeProcessGraphAssessment {
  profileRequired: boolean;
  profile: NativeProcessProfile | null;
  completeCases: number | null;
}

type DatasetRow = Dataset["rows"][number];

interface ProcessProfileAccumulator {
  scannedRows: number;
  completeCases: number;
  invalidBinaryRows: Map<string, number>;
  equationOutcomeLevels: Map<string, { zero: number; one: number; other: number }>;
  bounds: Map<string, { minimum: number; maximum: number }>;
}

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function finiteNumber(value: string | number | null | undefined): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function pathKey(path: Pick<NativeProcessPathConfig, "from" | "to">): string {
  return `${path.from}\u0000${path.to}`;
}

function moderationKey(moderation: Pick<NativeProcessModerationConfig, "from" | "to">): string {
  return `${moderation.from}\u0000${moderation.to}`;
}

function sameStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function uniqueMessages(messages: Array<string | null>): string[] {
  return [...new Set(messages.filter((message): message is string => Boolean(message)))];
}

export function parseNativeProcessGraph(value: unknown): NativeProcessGraphRelationshipConfig | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Partial<NativeProcessGraphRelationshipConfig>;
  if (candidate.model !== "graph"
    || !hasText(candidate.focal_predictor)
    || candidate.continuous_product_centering !== NATIVE_PROCESS_CENTERING_POLICY
    || !Array.isArray(candidate.paths)
    || !Array.isArray(candidate.moderators)
    || !Array.isArray(candidate.moderations)) return null;
  const paths = candidate.paths.flatMap((path) => path && typeof path === "object"
    && hasText(path.from) && hasText(path.to)
    ? [{ from: path.from.trim(), to: path.to.trim() }]
    : []);
  const moderators = candidate.moderators.flatMap((moderator) => moderator && typeof moderator === "object"
    && hasText(moderator.variable)
    && (moderator.scale === "continuous" || moderator.scale === "binary_0_1")
    ? [{ variable: moderator.variable.trim(), scale: moderator.scale }]
    : []);
  const moderations = candidate.moderations.flatMap((moderation) => moderation && typeof moderation === "object"
    && hasText(moderation.from) && hasText(moderation.to) && hasText(moderation.moderator)
    && (moderation.conditioning_moderator === undefined || hasText(moderation.conditioning_moderator))
    ? [{
        from: moderation.from.trim(),
        to: moderation.to.trim(),
        moderator: moderation.moderator.trim(),
        ...(hasText(moderation.conditioning_moderator)
          ? { conditioning_moderator: moderation.conditioning_moderator.trim() }
          : {}),
      }]
    : []);
  if (paths.length !== candidate.paths.length
    || moderators.length !== candidate.moderators.length
    || moderations.length !== candidate.moderations.length) return null;
  return {
    model: "graph",
    focal_predictor: candidate.focal_predictor.trim(),
    paths,
    moderators,
    moderations,
    continuous_product_centering: NATIVE_PROCESS_CENTERING_POLICY,
  };
}

function rustStringOrder(left: string, right: string): number {
  const leftPoints = Array.from(left, (character) => character.codePointAt(0)!);
  const rightPoints = Array.from(right, (character) => character.codePointAt(0)!);
  const length = Math.min(leftPoints.length, rightPoints.length);
  for (let index = 0; index < length; index += 1) {
    if (leftPoints[index] !== rightPoints[index]) return leftPoints[index] < rightPoints[index] ? -1 : 1;
  }
  return leftPoints.length === rightPoints.length ? 0 : leftPoints.length < rightPoints.length ? -1 : 1;
}

function topologicalOrder(nodes: readonly string[], paths: readonly NativeProcessPathConfig[]): string[] | null {
  const indegree = new Map(nodes.map((node) => [node, 0]));
  const outgoing = new Map(nodes.map((node) => [node, [] as string[]]));
  for (const path of paths) {
    if (!indegree.has(path.from) || !indegree.has(path.to)) return null;
    indegree.set(path.to, (indegree.get(path.to) ?? 0) + 1);
    outgoing.get(path.from)!.push(path.to);
  }
  for (const targets of outgoing.values()) targets.sort(rustStringOrder);
  const pending = nodes.filter((node) => indegree.get(node) === 0).sort(rustStringOrder);
  const ordered: string[] = [];
  while (pending.length) {
    const node = pending.shift()!;
    ordered.push(node);
    for (const target of outgoing.get(node) ?? []) {
      const next = (indegree.get(target) ?? 0) - 1;
      indegree.set(target, next);
      if (next === 0) {
        pending.push(target);
        pending.sort(rustStringOrder);
      }
    }
  }
  return ordered.length === nodes.length ? ordered : null;
}

function reachableFrom(source: string, paths: readonly NativeProcessPathConfig[]): Set<string> {
  const outgoing = new Map<string, string[]>();
  for (const path of paths) outgoing.set(path.from, [...(outgoing.get(path.from) ?? []), path.to]);
  const reached = new Set([source]);
  const pending = [source];
  while (pending.length) {
    const node = pending.shift()!;
    for (const target of outgoing.get(node) ?? []) {
      if (!reached.has(target)) {
        reached.add(target);
        pending.push(target);
      }
    }
  }
  return reached;
}

function enumerateDirectedPaths(
  source: string,
  target: string,
  paths: readonly NativeProcessPathConfig[],
): string[][] {
  const outgoing = new Map<string, string[]>();
  for (const path of paths) outgoing.set(path.from, [...(outgoing.get(path.from) ?? []), path.to]);
  const found: string[][] = [];
  const visit = (node: string, route: string[]) => {
    if (node === target) {
      found.push(route);
      return;
    }
    for (const next of outgoing.get(node) ?? []) {
      if (!route.includes(next)) visit(next, [...route, next]);
    }
  };
  visit(source, [source]);
  return found;
}

function equationTermCounts(
  paths: readonly NativeProcessPathConfig[],
  moderations: readonly NativeProcessModerationConfig[],
  controls: readonly string[],
): Array<{ outcome: string; terms: number }> {
  const interactionKey = (variables: readonly string[]) => JSON.stringify(
    [...variables].sort(rustStringOrder),
  );
  const outcomes = [...new Set(paths.map((path) => path.to))].sort(rustStringOrder);
  return outcomes.map((outcome) => {
    const incoming = new Set(paths.filter((path) => path.to === outcome).map((path) => path.from));
    const equationModerations = moderations.filter((moderation) => moderation.to === outcome);
    const moderatorMains = new Set<string>();
    const interactions = new Set<string>();
    for (const moderation of equationModerations) {
      moderatorMains.add(moderation.moderator);
      interactions.add(interactionKey([moderation.from, moderation.moderator]));
      if (moderation.conditioning_moderator) {
        moderatorMains.add(moderation.conditioning_moderator);
        interactions.add(interactionKey([moderation.from, moderation.conditioning_moderator]));
        interactions.add(interactionKey([moderation.moderator, moderation.conditioning_moderator]));
        interactions.add(interactionKey([moderation.from, moderation.moderator, moderation.conditioning_moderator]));
      }
    }
    return {
      outcome,
      terms: incoming.size + moderatorMains.size + interactions.size + controls.length,
    };
  });
}

export function nativeProcessGraphAssessment(
  settings: Readonly<AnalysisUiSettings>,
): NativeProcessGraphAssessment {
  const outcome = settings.regressionOutcome?.trim() ?? "";
  const controls = nativeOlsCsvValues(settings.regressionControls);
  const graph = parseNativeProcessGraph(settings.processGraph);
  if (!graph) {
    const blockers = [
      !outcome ? "Choose one numeric outcome variable" : null,
      "Define a graph-based PROCESS relationship with a focal predictor and at least one directed path",
    ].filter((problem): problem is string => Boolean(problem));
    return {
      canRun: false,
      blockers,
      outcome,
      focalPredictor: "",
      mediators: [],
      moderators: [],
      controls,
      predictors: [],
      graph: null,
      equationTermCounts: [],
      detail: `${blockers.join("; ")}.`,
    };
  }

  const focal = graph.focal_predictor;
  const pathNodes = [...new Set(graph.paths.flatMap((path) => [path.from, path.to]))];
  const moderatorNames = graph.moderators.map((moderator) => moderator.variable);
  const graphNodes = [...new Set([focal, outcome, ...pathNodes])].filter(Boolean);
  const order = topologicalOrder(graphNodes, graph.paths);
  const mediators = order?.filter((node) => node !== focal && node !== outcome) ?? [];
  const predictors = [focal, ...mediators, ...moderatorNames];
  const roleNames = [outcome, ...predictors, ...controls];
  const containsReservedProcessDelimiter = (name: string) => name.includes("->")
    || /[\u0000-\u001F\u007F-\u009F@|*,=]/u.test(name);
  const pathKeys = graph.paths.map(pathKey);
  const declaredModeratorSet = new Set(moderatorNames);
  const reachedFromFocal = reachableFrom(focal, graph.paths);
  const reachesOutcome = new Set(
    graphNodes.filter((node) => reachableFrom(node, graph.paths).has(outcome)),
  );
  const indirectPaths = enumerateDirectedPaths(focal, outcome, graph.paths).filter((path) => path.length > 2);
  const moderationByEdge = new Map<string, NativeProcessModerationConfig[]>();
  for (const moderation of graph.moderations) {
    const key = moderationKey(moderation);
    moderationByEdge.set(key, [...(moderationByEdge.get(key) ?? []), moderation]);
  }
  const shapeProblems: Array<string | null> = [];
  for (const route of indirectPaths) {
    const routeEdges = route.slice(0, -1).map((from, index) => `${from}\u0000${route[index + 1]}`);
    const moderated = routeEdges.flatMap((key) => moderationByEdge.get(key) ?? []);
    const firstEdge = routeEdges[0];
    const lastEdge = routeEdges.at(-1);
    if (moderated.length > 1) {
      shapeProblems.push(`Indirect path ${route.join(" -> ")} can moderate only one stage`);
    }
    for (const moderation of moderated) {
      const edge = moderationKey(moderation);
      if (edge !== firstEdge && edge !== lastEdge) {
        shapeProblems.push(`Indirect path ${route.join(" -> ")} cannot moderate an intermediate stage`);
      }
      if (moderation.conditioning_moderator) {
        shapeProblems.push(`Indirect path ${route.join(" -> ")} cannot use a two-moderator interaction`);
      }
    }
  }
  const counts = equationTermCounts(graph.paths, graph.moderations, controls);
  const selectedPredictors = nativeOlsCsvValues(settings.regressionPredictors);
  const blockers = uniqueMessages([
    !outcome ? "Choose one numeric outcome variable" : null,
    !focal ? "Choose one focal predictor" : null,
    focal === outcome ? "The focal predictor and outcome must be different variables" : null,
    graph.paths.length < 1 ? "Add at least one directed path" : null,
    graph.paths.length > NATIVE_PROCESS_MAX_PATHS ? `Use no more than ${NATIVE_PROCESS_MAX_PATHS} directed paths` : null,
    graph.paths.some((path) => path.from === path.to) ? "Self-directed paths are not supported" : null,
    new Set(pathKeys).size !== pathKeys.length ? "Each directed path must be unique" : null,
    graph.paths.some((path) => !path.from || !path.to) ? "Every directed path needs a source and outcome" : null,
    graph.paths.some((path) => path.to === focal) ? "The focal predictor cannot have an incoming path" : null,
    graph.paths.some((path) => path.from === outcome) ? "The outcome must be terminal and cannot have outgoing paths" : null,
    order === null ? "PROCESS paths must form a directed acyclic graph" : null,
    !reachedFromFocal.has(outcome) ? "The graph must contain a directed path from the focal predictor to the outcome" : null,
    mediators.length > NATIVE_PROCESS_MAX_MEDIATORS ? `Use no more than ${NATIVE_PROCESS_MAX_MEDIATORS} mediators` : null,
    mediators.some((mediator) => !reachedFromFocal.has(mediator) || !reachesOutcome.has(mediator))
      ? "Every mediator must lie on a directed focal-predictor-to-outcome path"
      : null,
    graph.moderators.length > NATIVE_PROCESS_MAX_MODERATORS ? `Declare no more than ${NATIVE_PROCESS_MAX_MODERATORS} moderator variables` : null,
    predictors.length > NATIVE_PROCESS_MAX_PREDICTORS
      ? `Use no more than ${NATIVE_PROCESS_MAX_PREDICTORS} predictors across the focal predictor, mediators, and declared moderators`
      : null,
    new Set(moderatorNames).size !== moderatorNames.length ? "Each moderator variable must be declared once" : null,
    roleNames.some(containsReservedProcessDelimiter)
      ? "PROCESS variable names cannot contain control characters, ->, @, |, *, comma, or equals because these tokens are reserved for stable scientific identities"
      : null,
    moderatorNames.some((moderator) => graphNodes.includes(moderator))
      ? "Moderator variables must be exogenous and cannot also be focal, mediator, or outcome variables"
      : null,
    graph.moderations.length > NATIVE_PROCESS_MAX_MODERATIONS ? `Define no more than ${NATIVE_PROCESS_MAX_MODERATIONS} moderated paths` : null,
    new Set(graph.moderations.map(moderationKey)).size !== graph.moderations.length
      ? "Each path can have only one moderation definition"
      : null,
    graph.moderations.some((moderation) => !pathKeys.includes(moderationKey(moderation)))
      ? "Every moderation must target an existing directed path"
      : null,
    graph.moderations.some((moderation) => !declaredModeratorSet.has(moderation.moderator)
      || (moderation.conditioning_moderator !== undefined
        && !declaredModeratorSet.has(moderation.conditioning_moderator)))
      ? "Every moderation variable must be declared with a scale"
      : null,
    moderatorNames.some((moderator) => !graph.moderations.some((moderation) => (
      moderation.moderator === moderator || moderation.conditioning_moderator === moderator
    ))) ? "Every declared moderator must be used as a primary or conditioning moderator" : null,
    graph.moderations.some((moderation) => moderation.conditioning_moderator === moderation.moderator)
      ? "A conditioning moderator must differ from the solved moderator"
      : null,
    graph.moderations.some((moderation) => moderation.conditioning_moderator
      && (moderation.from !== focal || moderation.to !== outcome))
      ? "Two-moderator interactions are supported only on the direct focal-predictor-to-outcome path"
      : null,
    controls.length > NATIVE_PROCESS_MAX_CONTROLS ? "Use no more than one control variable" : null,
    new Set(controls).size !== controls.length ? "Each control variable must be selected once" : null,
    controls.some((control) => [outcome, ...predictors].includes(control))
      ? "Controls must be distinct from every graph role"
      : null,
    counts.some((equation) => equation.terms > NATIVE_PROCESS_MAX_EQUATION_TERMS)
      ? `Each PROCESS equation supports at most ${NATIVE_PROCESS_MAX_EQUATION_TERMS} non-intercept terms`
      : null,
    selectedPredictors.length > 0 && !sameStrings(selectedPredictors, predictors)
      ? "PROCESS predictor order must be focal predictor, topological mediators, then declared moderators"
      : null,
    ...shapeProblems,
  ]);
  const canRun = blockers.length === 0;
  return {
    canRun,
    blockers,
    outcome,
    focalPredictor: focal,
    mediators,
    moderators: graph.moderators,
    controls,
    predictors,
    graph,
    equationTermCounts: counts,
    detail: canRun
      ? `Graph-defined path analysis is structurally ready with ${mediators.length} mediator${mediators.length === 1 ? "" : "s"}, ${graph.moderators.length} moderator${graph.moderators.length === 1 ? "" : "s"}, ${graph.paths.length} path${graph.paths.length === 1 ? "" : "s"}, and ${counts.length} OLS equation${counts.length === 1 ? "" : "s"}.`
      : `${blockers.join("; ")}.`,
  };
}

export function nativeProcessSelectionToken(
  settings: Readonly<AnalysisUiSettings>,
  assessment = nativeProcessGraphAssessment(settings),
): string {
  return JSON.stringify({
    outcome: assessment.outcome,
    predictors: assessment.predictors,
    controls: assessment.controls,
    graph: assessment.graph,
  });
}

function createProfileAccumulator(
  variables: readonly string[],
  binaryModerators: readonly string[],
  equationOutcomes: readonly string[],
): ProcessProfileAccumulator {
  return {
    scannedRows: 0,
    completeCases: 0,
    invalidBinaryRows: new Map(binaryModerators.map((variable) => [variable, 0])),
    equationOutcomeLevels: new Map(equationOutcomes.map((variable) => [variable, {
      zero: 0,
      one: 0,
      other: 0,
    }])),
    bounds: new Map(variables.map((variable) => [variable, {
      minimum: Number.POSITIVE_INFINITY,
      maximum: Number.NEGATIVE_INFINITY,
    }])),
  };
}

function scanProfileRows(
  accumulator: ProcessProfileAccumulator,
  rows: readonly DatasetRow[],
  variables: readonly string[],
  binaryModerators: readonly string[],
) {
  for (const row of rows) {
    accumulator.scannedRows += 1;
    const values = variables.map((variable) => finiteNumber(row[variable]));
    if (values.some((value) => value === null)) continue;
    accumulator.completeCases += 1;
    variables.forEach((variable, index) => {
      const value = values[index]!;
      const bounds = accumulator.bounds.get(variable)!;
      bounds.minimum = Math.min(bounds.minimum, value);
      bounds.maximum = Math.max(bounds.maximum, value);
    });
    for (const moderator of binaryModerators) {
      const value = values[variables.indexOf(moderator)]!;
      if (value !== 0 && value !== 1) {
        accumulator.invalidBinaryRows.set(
          moderator,
          (accumulator.invalidBinaryRows.get(moderator) ?? 0) + 1,
        );
      }
    }
    for (const [variable, levels] of accumulator.equationOutcomeLevels) {
      const value = values[variables.indexOf(variable)]!;
      if (value === 0) levels.zero += 1;
      else if (value === 1) levels.one += 1;
      else levels.other += 1;
    }
  }
}

function finishProfile(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
  assessment: NativeProcessGraphAssessment,
  expectedRows: number,
  accumulator: ProcessProfileAccumulator,
): NativeProcessProfile {
  const variables = [assessment.outcome, ...assessment.predictors, ...assessment.controls];
  const binaryModerators = assessment.moderators
    .filter((moderator) => moderator.scale === "binary_0_1")
    .map((moderator) => moderator.variable);
  return {
    datasetId: dataset.id,
    datasetFingerprint: dataset.fingerprint?.trim() ?? "",
    selectionToken: nativeProcessSelectionToken(settings, assessment),
    variables,
    binaryModerators,
    expectedRows,
    scannedRows: accumulator.scannedRows,
    completeCases: accumulator.completeCases,
    omittedRows: Math.max(0, expectedRows - accumulator.completeCases),
    invalidBinaryRows: Object.fromEntries(accumulator.invalidBinaryRows),
    binaryEquationOutcomes: [...accumulator.equationOutcomeLevels.entries()]
      .filter(([, levels]) => levels.zero > 0 && levels.one > 0 && levels.other === 0)
      .map(([variable]) => variable),
    constantVariables: [...accumulator.bounds.entries()]
      .filter(([, bounds]) => Number.isFinite(bounds.minimum) && bounds.minimum === bounds.maximum)
      .map(([variable]) => variable),
  };
}

export function residentNativeProcessProfile(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
): NativeProcessProfile | null {
  const assessment = nativeProcessGraphAssessment(settings);
  if (!assessment.canRun) return null;
  const expectedRows = dataset.rowCount ?? dataset.rows.length;
  if (dataset.rows.length < expectedRows) return null;
  const variables = [assessment.outcome, ...assessment.predictors, ...assessment.controls];
  const binaryModerators = assessment.moderators
    .filter((moderator) => moderator.scale === "binary_0_1")
    .map((moderator) => moderator.variable);
  const equationOutcomes = [...new Set(assessment.graph!.paths.map((path) => path.to))];
  const accumulator = createProfileAccumulator(variables, binaryModerators, equationOutcomes);
  scanProfileRows(accumulator, dataset.rows.slice(0, expectedRows), variables, binaryModerators);
  return finishProfile(dataset, settings, assessment, expectedRows, accumulator);
}

export async function profileNativeProcessDataset(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
  readPage: (datasetId: string, offset: number, limit: number) => Promise<DatasetRowsPage> = getNativeDatasetRows,
  isCancelled: () => boolean = () => false,
): Promise<NativeProcessProfile> {
  const resident = residentNativeProcessProfile(dataset, settings);
  if (resident) return resident;
  const assessment = nativeProcessGraphAssessment(settings);
  if (!assessment.canRun) throw new Error(assessment.blockers[0] ?? "Complete the graph-defined PROCESS setup before profiling.");
  const expectedRows = dataset.rowCount ?? dataset.rows.length;
  const variables = [assessment.outcome, ...assessment.predictors, ...assessment.controls];
  const binaryModerators = assessment.moderators
    .filter((moderator) => moderator.scale === "binary_0_1")
    .map((moderator) => moderator.variable);
  const equationOutcomes = [...new Set(assessment.graph!.paths.map((path) => path.to))];
  const accumulator = createProfileAccumulator(variables, binaryModerators, equationOutcomes);
  let offset = 0;
  while (offset < expectedRows) {
    if (isCancelled()) throw new Error("PROCESS data profiling was cancelled.");
    const page = await readPage(dataset.id, offset, NATIVE_PROCESS_PROFILE_PAGE_SIZE);
    if (page.datasetId !== dataset.id || page.offset !== offset || page.rowCount !== expectedRows) {
      throw new Error("The dataset changed while its PROCESS variables were being profiled.");
    }
    if (page.rows.length === 0 || page.rows.length > NATIVE_PROCESS_PROFILE_PAGE_SIZE) {
      throw new Error("The desktop row service returned an invalid PROCESS-profile page.");
    }
    scanProfileRows(accumulator, page.rows, variables, binaryModerators);
    offset += page.rows.length;
  }
  if (accumulator.scannedRows !== expectedRows) {
    throw new Error(`Expected ${expectedRows} rows but profiled ${accumulator.scannedRows}.`);
  }
  return finishProfile(dataset, settings, assessment, expectedRows, accumulator);
}

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function textArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(hasText);
}

export function parseNativeProcessProfile(value: unknown): NativeProcessProfile | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Partial<NativeProcessProfile>;
  if (!hasText(candidate.datasetId)
    || !hasText(candidate.datasetFingerprint)
    || !hasText(candidate.selectionToken)
    || !textArray(candidate.variables)
    || !textArray(candidate.binaryModerators)
    || !nonNegativeInteger(candidate.expectedRows)
    || !nonNegativeInteger(candidate.scannedRows)
    || !nonNegativeInteger(candidate.completeCases)
    || !nonNegativeInteger(candidate.omittedRows)
    || !Array.isArray(candidate.constantVariables)
    || !candidate.constantVariables.every(hasText)
    || !Array.isArray(candidate.binaryEquationOutcomes)
    || !candidate.binaryEquationOutcomes.every(hasText)
    || !candidate.invalidBinaryRows
    || typeof candidate.invalidBinaryRows !== "object"
    || Array.isArray(candidate.invalidBinaryRows)) return null;
  const invalidEntries = Object.entries(candidate.invalidBinaryRows);
  const variables = candidate.variables;
  const binaryModerators = candidate.binaryModerators;
  if (candidate.scannedRows !== candidate.expectedRows
    || candidate.completeCases + candidate.omittedRows !== candidate.expectedRows
    || new Set(variables).size !== variables.length
    || new Set(binaryModerators).size !== binaryModerators.length
    || binaryModerators.some((variable) => !variables.includes(variable))
    || new Set(candidate.constantVariables).size !== candidate.constantVariables.length
    || candidate.constantVariables.some((variable) => !variables.includes(variable))
    || new Set(candidate.binaryEquationOutcomes).size !== candidate.binaryEquationOutcomes.length
    || candidate.binaryEquationOutcomes.some((variable) => !variables.includes(variable))
    || invalidEntries.length !== binaryModerators.length
    || invalidEntries.some(([variable, count]) => !binaryModerators.includes(variable)
      || !nonNegativeInteger(count))) return null;
  return {
    datasetId: candidate.datasetId,
    datasetFingerprint: candidate.datasetFingerprint,
    selectionToken: candidate.selectionToken,
    variables: [...variables],
    binaryModerators: [...binaryModerators],
    expectedRows: candidate.expectedRows,
    scannedRows: candidate.scannedRows,
    completeCases: candidate.completeCases,
    omittedRows: candidate.omittedRows,
    invalidBinaryRows: Object.fromEntries(invalidEntries) as Record<string, number>,
    binaryEquationOutcomes: [...candidate.binaryEquationOutcomes],
    constantVariables: [...candidate.constantVariables],
  };
}

export function nativeProcessReadiness(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
  suppliedProfile: NativeProcessProfile | null = null,
): NativeProcessReadinessAssessment {
  const graph = nativeProcessGraphAssessment(settings);
  const numeric = new Set(nativeOlsNumericColumns(dataset));
  const variables = [graph.outcome, ...graph.predictors, ...graph.controls].filter(Boolean);
  const blockers = [
    ...graph.blockers,
    ...variables.map((variable) => !dataset.columns.includes(variable)
      ? `The selected variable ${variable} is absent from the active dataset`
      : !numeric.has(variable)
        ? `The selected variable ${variable} is not numeric`
        : null),
    !dataset.fingerprint?.trim() ? "Import or reopen a fingerprinted dataset before graph-defined path analysis" : null,
    settings.preprocessing !== "unstandardized" ? "PROCESS v2 requires raw unstandardized variables" : null,
    settings.confidenceLevel !== 0.95 ? "PROCESS v2 uses a fixed 95% confidence level" : null,
    settings.studentizedInnerSamples !== 0 ? "PROCESS v2 does not support studentized intervals" : null,
    settings.permutationSamples !== 0 ? "PROCESS v2 cannot be combined with permutation inference" : null,
  ].filter((problem): problem is string => Boolean(problem));

  const parsedProfile = suppliedProfile ? parseNativeProcessProfile(suppliedProfile) : null;
  const profile = parsedProfile ?? (suppliedProfile || blockers.length
    ? null
    : residentNativeProcessProfile(dataset, settings));
  if (profile) {
    const expectedRows = dataset.rowCount ?? dataset.rows.length;
    const binaryModerators = graph.moderators
      .filter((moderator) => moderator.scale === "binary_0_1")
      .map((moderator) => moderator.variable);
    if (profile.datasetId !== dataset.id
      || profile.datasetFingerprint !== dataset.fingerprint
      || profile.selectionToken !== nativeProcessSelectionToken(settings, graph)
      || !sameStrings(profile.variables, variables)
      || !sameStrings(profile.binaryModerators, binaryModerators)
      || profile.expectedRows !== expectedRows
      || profile.scannedRows !== expectedRows) {
      blockers.push("Reload the complete PROCESS profile for the current dataset and graph");
    } else {
      const invalid = Object.entries(profile.invalidBinaryRows).filter(([, count]) => count > 0);
      if (invalid.length) {
        blockers.push(invalid.map(([variable, count]) => `${variable} has ${count} listwise-complete value${count === 1 ? "" : "s"} outside exact 0/1 coding`).join("; "));
      }
      if (profile.constantVariables.length) {
        blockers.push(`${profile.constantVariables.join(", ")} ${profile.constantVariables.length === 1 ? "is constant" : "are constant"} after global listwise deletion`);
      }
      if (profile.binaryEquationOutcomes.length) {
        blockers.push(`PROCESS v2 requires continuous endogenous equation outcomes; ${profile.binaryEquationOutcomes.join(", ")} ${profile.binaryEquationOutcomes.length === 1 ? "is" : "are"} exactly coded 0/1 in the original complete sample`);
      }
      const largestEquation = Math.max(0, ...graph.equationTermCounts.map((equation) => equation.terms));
      const minimumCases = largestEquation + 2;
      if (profile.completeCases < minimumCases) {
        blockers.push(`PROCESS v2 requires at least ${minimumCases} complete finite rows for the largest ${largestEquation}-term equation`);
      }
    }
  } else if (suppliedProfile) {
    blockers.push("Reload the complete PROCESS profile because its dispatch proof is invalid");
  }

  const uniqueBlockers = [...new Set(blockers)];
  const profileRequired = uniqueBlockers.length === 0 && profile === null;
  const completeCases = profile?.completeCases ?? null;
  const detail = uniqueBlockers.length
    ? `${uniqueBlockers.join("; ")}.`
    : profileRequired
      ? "The graph-defined PROCESS setup is structurally ready. Profile every dataset row before starting the calculation."
      : `Graph-defined PROCESS v2 is ready with ${completeCases} global listwise-complete case${completeCases === 1 ? "" : "s"}, ${graph.equationTermCounts.length} OLS equation${graph.equationTermCounts.length === 1 ? "" : "s"}, HC3 covariance, and fixed two-sided 95% Student-t inference.`;
  return {
    ...graph,
    canRun: uniqueBlockers.length === 0,
    blockers: uniqueBlockers,
    profileRequired,
    profile,
    completeCases,
    detail,
  };
}

export function nativeProcessPredictors(settings: Readonly<AnalysisUiSettings>): string[] {
  return nativeProcessGraphAssessment(settings).predictors;
}
