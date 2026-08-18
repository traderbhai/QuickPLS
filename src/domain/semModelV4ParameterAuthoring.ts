import type { Edge, Node } from "@xyflow/react";
import type {
  ConstructData,
  PathEdgeData,
  SemModelV4CovarianceAuthoringState,
  SemModelV4FactorIdentificationAuthoring,
  SemModelV4ParameterAuthoringEntry,
  SemModelV4ParameterAuthoringSpecification,
  SemModelV4ParameterAuthoringTarget,
} from "../types";
import type {
  FactorIdentificationV4,
  SemConstraintV4,
  SemModelV4,
  SemParameterTargetV4,
  SemParameterV4,
  SemRelationV4,
  SemVariableV4,
} from "./semModelV4";
import { hasStructuralFeedbackV4 } from "./semModelV4";
import { SEM_MODEL_V4_AUTHORING_VERSION } from "./semModelV4Authoring";

export const SEM_MODEL_V4_PARAMETER_AUTHORING_VERSION = 1 as const;

export interface NativeSemParameterAuthoringDiagnosticV4 {
  code: string;
  subject: string;
  message: string;
  corrective_action: string;
}

export type NativeSemFactorIdentificationInspectionV4 =
  | { state: "marker_loading"; persisted: boolean; specification: Extract<SemModelV4FactorIdentificationAuthoring, { kind: "marker_loading" }> }
  | { state: "fixed_variance"; persisted: true; specification: Extract<SemModelV4FactorIdentificationAuthoring, { kind: "fixed_variance" }> }
  | { state: "effects_coding"; persisted: true; specification: Extract<SemModelV4FactorIdentificationAuthoring, { kind: "effects_coding" }> }
  | { state: "not_applicable"; persisted: false; specification: null }
  | { state: "invalid"; persisted: true; specification: null };

export interface NativeSemModelParameterAuthoringInputV4 {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  variables: readonly SemVariableV4[];
  relations: readonly SemRelationV4[];
  parameters: readonly SemParameterV4[];
  constraints: readonly SemConstraintV4[];
}

export type NativeSemModelParameterAuthoringResultV4 =
  | {
    ok: true;
    variables: SemVariableV4[];
    parameters: SemParameterV4[];
    constraints: SemConstraintV4[];
  }
  | { ok: false; diagnostics: readonly NativeSemParameterAuthoringDiagnosticV4[] };

export const nativeSemObservedInterceptParameterIdV4 = (indicator: string) => `native-sem-v4:intercept:${encodeURIComponent(indicator)}`;
export const nativeSemLatentMeanParameterIdV4 = (constructId: string) => `native-sem-v4:mean:${encodeURIComponent(constructId)}`;
export const nativeSemOrdinalThresholdParameterIdV4 = (indicator: string, index: number) => `native-sem-v4:threshold:${encodeURIComponent(indicator)}:${index}`;
export const nativeSemFixedFactorVarianceParameterIdV4 = (constructId: string) => `native-sem-v4:factor-scale-variance:${encodeURIComponent(constructId)}`;
export const nativeSemEffectsCodingConstraintIdV4 = (constructId: string) => `native-sem-v4:effects-coding:${encodeURIComponent(constructId)}`;

export function inspectNativeSemFactorIdentificationV4(node: Node<ConstructData>): NativeSemFactorIdentificationInspectionV4 {
  const state = node.data.semModelV4;
  if (state?.construct.kind !== "common_factor") return { state: "not_applicable", persisted: false, specification: null };
  const raw = state.identification;
  if (raw === undefined) {
    const marker = state.construct.marker_indicator ?? [...node.data.indicators].sort()[0];
    return marker
      ? { state: "marker_loading", persisted: false, specification: { kind: "marker_loading", indicator: marker } }
      : { state: "invalid", persisted: true, specification: null };
  }
  if (!isRecord(raw)) return { state: "invalid", persisted: true, specification: null };
  if (raw.kind === "marker_loading" && exactKeys(raw, ["kind", "indicator"]) && requiredText(raw.indicator)) return {
    state: "marker_loading",
    persisted: true,
    specification: { kind: "marker_loading", indicator: String(raw.indicator) },
  };
  if (raw.kind === "fixed_variance" && exactKeys(raw, ["kind"])) return {
    state: "fixed_variance",
    persisted: true,
    specification: { kind: "fixed_variance" },
  };
  if (raw.kind === "effects_coding" && exactKeys(raw, ["kind"])) return {
    state: "effects_coding",
    persisted: true,
    specification: { kind: "effects_coding" },
  };
  return { state: "invalid", persisted: true, specification: null };
}

export function withNativeSemFactorIdentificationV4(
  node: Node<ConstructData>,
  identification: SemModelV4FactorIdentificationAuthoring,
): Node<ConstructData> {
  if (node.data.semModelV4?.construct.kind !== "common_factor") throw new Error("Common-factor identification requires an explicitly confirmed common factor.");
  const normalized = normalizeIdentification(identification, node);
  return {
    ...node,
    data: {
      ...node.data,
      semModelV4: {
        ...node.data.semModelV4,
        version: SEM_MODEL_V4_AUTHORING_VERSION,
        identification: normalized,
        construct: normalized.kind === "marker_loading"
          ? { kind: "common_factor", marker_indicator: normalized.indicator }
          : node.data.semModelV4.construct,
      },
    },
  };
}

export function withNativeSemParameterEntryOnConstructV4(
  node: Node<ConstructData>,
  entry: SemModelV4ParameterAuthoringEntry | null,
  parameterId = entry?.parameter_id ?? "",
): Node<ConstructData> {
  if (!node.data.semModelV4) throw new Error("Confirm this construct's scientific representation before editing parameters.");
  const parameters = replaceEntry(node.data.semModelV4.parameters ?? [], entry, parameterId);
  return {
    ...node,
    data: {
      ...node.data,
      semModelV4: {
        ...node.data.semModelV4,
        ...(parameters.length ? { parameters } : { parameters: undefined }),
      },
    },
  };
}

/** Applies a set of construct-owned edits as one immutable presentation update. */
export function withNativeSemParameterEntriesOnConstructV4(
  node: Node<ConstructData>,
  entries: readonly SemModelV4ParameterAuthoringEntry[],
  removeParameterIds: readonly string[] = [],
): Node<ConstructData> {
  if (!node.data.semModelV4) throw new Error("Confirm this construct's scientific representation before editing parameters.");
  const removals = new Set(removeParameterIds);
  let parameters = (node.data.semModelV4.parameters ?? []).filter((entry) => !removals.has(entry.parameter_id));
  for (const entry of entries) parameters = replaceEntry(parameters, entry, entry.parameter_id);
  return {
    ...node,
    data: {
      ...node.data,
      semModelV4: {
        ...node.data.semModelV4,
        ...(parameters.length ? { parameters } : { parameters: undefined }),
      },
    },
  };
}

export function withNativeSemParameterEntryOnEdgeV4(
  edge: Edge,
  entry: SemModelV4ParameterAuthoringEntry | null,
  parameterId = entry?.parameter_id ?? "",
): Edge {
  const state = edge.data?.semModelV4;
  if (!state || !isRecord(state) || state.version !== SEM_MODEL_V4_AUTHORING_VERSION || !isRecord(state.covariance)) {
    if (edge.data?.role === "covariance") throw new Error("Classify this covariance before editing its parameter.");
  }
  const current = isRecord(state) && Array.isArray(state.parameters)
    ? state.parameters as unknown as SemModelV4ParameterAuthoringEntry[]
    : [];
  const parameters = replaceEntry(current, entry, parameterId);
  const data = { ...(edge.data ?? {}) };
  if (edge.data?.role === "covariance") {
    const covarianceState = state as unknown as SemModelV4CovarianceAuthoringState;
    data.semModelV4 = {
      ...covarianceState,
      ...(parameters.length ? { parameters } : { parameters: undefined }),
    };
  } else {
    (data as PathEdgeData).semModelV4ParameterAuthoring = {
      version: SEM_MODEL_V4_PARAMETER_AUTHORING_VERSION,
      parameters,
    };
  }
  return { ...edge, data };
}

export function parameterEntryFromSemParameterV4(parameter: SemParameterV4): SemModelV4ParameterAuthoringEntry {
  if (parameter.kind === "derived") throw new Error("Derived parameters are read-only.");
  return {
    parameter_id: parameter.id,
    target: cloneTarget(parameter.target),
    specification: parameter.kind === "fixed"
      ? { kind: "fixed", value: parameter.value }
      : {
        kind: "free",
        start: parameter.start ?? null,
        lower: parameter.lower ?? null,
        upper: parameter.upper ?? null,
        equality_label: parameter.equality_label?.trim() || null,
      },
  };
}

export function nativeSemObservedInterceptEntryV4(indicator: string): SemModelV4ParameterAuthoringEntry {
  return {
    parameter_id: nativeSemObservedInterceptParameterIdV4(indicator),
    target: { kind: "intercept", variable: `observed:${indicator}` },
    specification: { kind: "free", start: 0, lower: null, upper: null, equality_label: null },
  };
}

export function nativeSemLatentMeanEntryV4(constructId: string): SemModelV4ParameterAuthoringEntry {
  return {
    parameter_id: nativeSemLatentMeanParameterIdV4(constructId),
    target: { kind: "mean", variable: `construct:${constructId}` },
    specification: { kind: "free", start: 0, lower: null, upper: null, equality_label: null },
  };
}

export function nativeSemOrdinalThresholdEntriesV4(indicator: string, categoryCount: number): SemModelV4ParameterAuthoringEntry[] {
  if (!Number.isInteger(categoryCount) || categoryCount < 2) return [];
  return Array.from({ length: categoryCount - 1 }, (_, offset) => {
    const index = offset + 1;
    return {
      parameter_id: nativeSemOrdinalThresholdParameterIdV4(indicator, index),
      target: { kind: "threshold", variable: `observed:${indicator}`, index },
      specification: { kind: "free", start: index, lower: null, upper: null, equality_label: null },
    } satisfies SemModelV4ParameterAuthoringEntry;
  });
}

export function validateNativeSemParameterSpecificationV4(
  parameterId: string,
  specification: SemModelV4ParameterAuthoringSpecification,
): readonly NativeSemParameterAuthoringDiagnosticV4[] {
  const diagnostics: NativeSemParameterAuthoringDiagnosticV4[] = [];
  if (!requiredText(parameterId)) diagnostics.push(diagnostic("sem_model_v4.parameter.id_required", parameterId || "parameter", "Parameter ID is missing.", "Restore the parameter from the Parameter Table."));
  if (specification.kind === "fixed") {
    if (!Number.isFinite(specification.value)) diagnostics.push(diagnostic("sem_model_v4.parameter.fixed_value_invalid", parameterId, "Fixed value must be finite.", "Enter a finite numeric value."));
    return diagnostics;
  }
  for (const [field, value] of [["start", specification.start], ["lower", specification.lower], ["upper", specification.upper]] as const) {
    if (value !== null && !Number.isFinite(value)) diagnostics.push(diagnostic(`sem_model_v4.parameter.${field}_invalid`, parameterId, `${human(field)} value must be finite or blank.`, "Enter a finite number or clear the field."));
  }
  if (specification.lower !== null && specification.upper !== null && specification.lower > specification.upper) diagnostics.push(diagnostic(
    "sem_model_v4.parameter.bounds_invalid",
    parameterId,
    "Lower bound cannot exceed upper bound.",
    "Reduce the lower bound or increase the upper bound.",
  ));
  if (
    specification.start !== null
    && (
      (specification.lower !== null && specification.start < specification.lower)
      || (specification.upper !== null && specification.start > specification.upper)
    )
  ) diagnostics.push(diagnostic(
    "sem_model_v4.parameter.start_outside_bounds",
    parameterId,
    "Start value must lie within the selected bounds.",
    "Move the start value inside the bounds or clear it.",
  ));
  if (specification.equality_label !== null && !/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(specification.equality_label)) diagnostics.push(diagnostic(
    "sem_model_v4.parameter.equality_label_invalid",
    parameterId,
    "Equality label must start with a letter and contain only letters, numbers, dot, underscore, or hyphen.",
    "Enter a label such as loading_a or clear the field.",
  ));
  return diagnostics;
}

export function applyNativeSemModelParameterAuthoringV4(
  input: NativeSemModelParameterAuthoringInputV4,
): NativeSemModelParameterAuthoringResultV4 {
  const diagnostics: NativeSemParameterAuthoringDiagnosticV4[] = [];
  const variables = structuredClone(input.variables) as SemVariableV4[];
  let parameters = structuredClone(input.parameters) as SemParameterV4[];
  const constraints = structuredClone(input.constraints) as SemConstraintV4[];
  const variablesById = new Map(variables.map((variable) => [variable.id, variable]));
  const nodeById = new Map(input.nodes.map((node) => [node.id, node]));
  const indicatorOwner = new Map(input.nodes.flatMap((node) => node.data.indicators.map((indicator) => [indicator, node.id] as const)));
  const collected = collectEntries(input.nodes, input.edges, diagnostics);
  const authoredById = new Map<string, SemModelV4ParameterAuthoringEntry>();
  for (const item of collected) {
    if (authoredById.has(item.entry.parameter_id)) {
      diagnostics.push(diagnostic("sem_model_v4.parameter.id_duplicate", item.entry.parameter_id, `Parameter ${item.entry.parameter_id} is authored on more than one object.`, "Keep the parameter on its owning construct or relationship only."));
      continue;
    }
    authoredById.set(item.entry.parameter_id, item.entry);
    diagnostics.push(...validateNativeSemParameterSpecificationV4(item.entry.parameter_id, item.entry.specification));
    diagnostics.push(...validateEntryOwnership(item.entry, item.source, nodeById, indicatorOwner, input.edges));
  }

  const existingById = new Map(parameters.map((parameter) => [parameter.id, parameter]));
  for (const [id, entry] of authoredById) {
    const existing = existingById.get(id);
    if (existing) {
      if (!targetEquals(existing.target, entry.target)) {
        diagnostics.push(diagnostic("sem_model_v4.parameter.target_mismatch", id, `Stored target for ${id} no longer matches the generated model object.`, "Remove the stale edit and edit the current Parameter Table row again."));
        continue;
      }
      parameters = parameters.map((parameter) => parameter.id === id ? parameterFromEntry(entry, parameter.label, parameter.target) : parameter);
      continue;
    }
    const locationProblem = validateNewLocationEntry(entry, variablesById, indicatorOwner);
    if (locationProblem) {
      diagnostics.push(locationProblem);
      continue;
    }
    parameters.push(parameterFromEntry(entry, locationLabel(entry.target, variablesById), cloneTarget(entry.target)));
  }

  for (const node of input.nodes) {
    const variable = variablesById.get(`construct:${node.id}`);
    if (variable?.kind !== "common_factor") continue;
    const identification = inspectNativeSemFactorIdentificationV4(node);
    if (identification.state === "invalid") {
      diagnostics.push(diagnostic("sem_model_v4.identification.metadata_invalid", node.id, `Identification for ${node.data.label || node.id} cannot be read safely.`, "Choose Marker loading, Fixed variance, or Effects coding again."));
      continue;
    }
    if (identification.state === "not_applicable") continue;
    const effects = input.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" }> => relation.kind === "measurement_effect" && relation.construct === variable.id);
    const result = applyIdentification(node, variable, identification.specification, effects, parameters, constraints, authoredById);
    parameters = result.parameters;
    constraints.splice(0, constraints.length, ...result.constraints);
    diagnostics.push(...result.diagnostics);
  }

  for (const parameter of parameters) if (parameter.target.kind === "mean") {
    const variable = variablesById.get(parameter.target.variable);
    if (variable?.kind !== "common_factor") continue;
    if (parameter.kind !== "free") diagnostics.push(diagnostic("sem_model_v4.mean.fixed_unsupported", parameter.id, "The current single-group factor mean policy supports fixed zero or an estimated free mean.", "Remove this parameter to fix the mean at zero, or change it to Free."));
    else variable.mean_policy = { kind: "estimated", parameter: parameter.id };
  }

  if (diagnostics.length) return { ok: false, diagnostics: freeze(sortDiagnostics(diagnostics)) };
  return {
    ok: true,
    variables: variables.sort((left, right) => left.id.localeCompare(right.id)),
    parameters: parameters.sort((left, right) => left.id.localeCompare(right.id)),
    constraints: constraints.sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function unsupportedNativeSemParameterAuthoringDiagnosticsV4(model: SemModelV4): readonly NativeSemParameterAuthoringDiagnosticV4[] {
  const diagnostics: NativeSemParameterAuthoringDiagnosticV4[] = [];
  const groupOverride = model.parameters.find((parameter) => parameter.group_overrides?.length);
  if (groupOverride) diagnostics.push(diagnostic(
    "sem_model_v4.parameter.group_overrides_not_available",
    groupOverride.id,
    "Group-specific parameter overrides are not available in this editor yet.",
    "Remove the group override and use the shared single-group parameter specification.",
  ));
  if (hasStructuralFeedbackV4(model)) diagnostics.push(diagnostic(
    "sem_model_v4.parameter.feedback_not_available",
    model.id,
    "Parameter authoring for feedback or reciprocal structural systems is not available yet.",
    "Remove one reciprocal path or keep this model read-only until feedback identification is implemented.",
  ));
  return freeze(sortDiagnostics(diagnostics));
}

function collectEntries(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  diagnostics: NativeSemParameterAuthoringDiagnosticV4[],
): Array<{ entry: SemModelV4ParameterAuthoringEntry; source: { kind: "node" | "edge"; id: string } }> {
  const output: Array<{ entry: SemModelV4ParameterAuthoringEntry; source: { kind: "node" | "edge"; id: string } }> = [];
  for (const node of nodes) {
    const raw = node.data.semModelV4?.parameters;
    if (raw === undefined) continue;
    if (!Array.isArray(raw)) {
      diagnostics.push(diagnostic("sem_model_v4.parameter.collection_invalid", node.id, "Construct parameter edits cannot be read safely.", "Open the construct and restore its parameter rows."));
      continue;
    }
    for (const value of raw) {
      const parsed = parseEntry(value, node.id);
      if (parsed.ok) output.push({ entry: parsed.entry, source: { kind: "node", id: node.id } });
      else diagnostics.push(parsed.diagnostic);
    }
  }
  for (const edge of edges) {
    const sem = edge.data?.role === "covariance"
      ? edge.data?.semModelV4
      : (edge.data as Record<string, unknown> | undefined)?.semModelV4ParameterAuthoring;
    if (!isRecord(sem) || sem.parameters === undefined) continue;
    if (sem.version !== SEM_MODEL_V4_PARAMETER_AUTHORING_VERSION || !Array.isArray(sem.parameters)) {
      diagnostics.push(diagnostic("sem_model_v4.parameter.collection_invalid", edge.id, "Relationship parameter edits cannot be read safely.", "Open the relationship and restore its parameter row."));
      continue;
    }
    for (const value of sem.parameters) {
      const parsed = parseEntry(value, edge.id);
      if (parsed.ok) output.push({ entry: parsed.entry, source: { kind: "edge", id: edge.id } });
      else diagnostics.push(parsed.diagnostic);
    }
  }
  return output.sort((left, right) => left.entry.parameter_id.localeCompare(right.entry.parameter_id));
}

function applyIdentification(
  node: Node<ConstructData>,
  variable: Extract<SemVariableV4, { kind: "common_factor" }>,
  identification: SemModelV4FactorIdentificationAuthoring,
  effects: readonly Extract<SemRelationV4, { kind: "measurement_effect" }>[],
  currentParameters: readonly SemParameterV4[],
  currentConstraints: readonly SemConstraintV4[],
  authoredById: ReadonlyMap<string, SemModelV4ParameterAuthoringEntry>,
): { parameters: SemParameterV4[]; constraints: SemConstraintV4[]; diagnostics: NativeSemParameterAuthoringDiagnosticV4[] } {
  let parameters = [...currentParameters];
  let constraints = currentConstraints.filter((constraint) => constraint.id !== nativeSemEffectsCodingConstraintIdV4(node.id));
  const diagnostics: NativeSemParameterAuthoringDiagnosticV4[] = [];
  if (identification.kind === "marker_loading") {
    if (!node.data.indicators.includes(identification.indicator)) diagnostics.push(diagnostic("sem_model_v4.identification.marker_unknown", node.id, `${identification.indicator} is not assigned to ${node.data.label || node.id}.`, "Choose an assigned indicator as the marker."));
    const relation = effects.find((effect) => effect.indicator === `observed:${identification.indicator}`);
    if (!relation) diagnostics.push(diagnostic("sem_model_v4.identification.marker_relation_missing", node.id, "The selected marker has no loading relation.", "Restore the indicator assignment or choose another marker."));
    else {
      const authored = authoredById.get(relation.parameter);
      if (authored && (authored.specification.kind !== "fixed" || authored.specification.value !== 1)) diagnostics.push(diagnostic("sem_model_v4.identification.marker_parameter_conflict", relation.parameter, "Marker identification requires its loading to be fixed at one.", "Set the marker loading to Fixed with value 1, or choose a different identification method."));
      parameters = parameters.map((parameter) => {
        if (parameter.id === relation.parameter) return { kind: "fixed", id: parameter.id, label: parameter.label, target: parameter.target, value: 1, group_overrides: [] };
        if (effects.some((effect) => effect.parameter === parameter.id) && parameter.kind === "fixed" && !authoredById.has(parameter.id)) return defaultFreeLoading(parameter);
        return parameter;
      });
      variable.identification = { kind: "marker_loading", indicator: relation.indicator };
    }
  } else if (identification.kind === "fixed_variance") {
    parameters = parameters.map((parameter) => effects.some((effect) => effect.parameter === parameter.id) && parameter.kind === "fixed" && !authoredById.has(parameter.id)
      ? defaultFreeLoading(parameter)
      : parameter);
    const id = variable.disturbance_policy.kind === "exogenous_variance"
      ? variable.disturbance_policy.parameter
      : nativeSemFixedFactorVarianceParameterIdV4(node.id);
    const authored = authoredById.get(id);
    if (authored && (authored.specification.kind !== "fixed" || authored.specification.value !== 1)) diagnostics.push(diagnostic("sem_model_v4.identification.fixed_variance_parameter_conflict", id, "Fixed-variance identification requires the factor variance to equal one.", "Set this variance to Fixed with value 1, or choose another identification method."));
    const existing = parameters.find((parameter) => parameter.id === id);
    if (existing) parameters = parameters.map((parameter) => parameter.id === id ? { kind: "fixed", id: parameter.id, label: parameter.label, target: { kind: "variance", endpoint: { kind: "variable", id: variable.id } }, value: 1, group_overrides: [] } : parameter);
    else parameters.push({
      kind: "fixed",
      id,
      label: `Variance(${variable.label})`,
      target: { kind: "variance", endpoint: { kind: "variable", id: variable.id } },
      value: 1,
      group_overrides: [],
    });
    variable.identification = { kind: "fixed_variance" };
  } else {
    if (effects.length < 3) diagnostics.push(diagnostic("sem_model_v4.identification.effects_coding_indicators", node.id, "Effects coding requires at least three reflective indicators.", "Assign at least three indicators or choose Marker loading or Fixed variance."));
    for (const effect of effects) {
      const authored = authoredById.get(effect.parameter);
      if (authored?.specification.kind === "fixed") diagnostics.push(diagnostic("sem_model_v4.identification.effects_coding_parameter_conflict", effect.parameter, "Effects coding requires every loading in the sum constraint to remain free.", "Change this loading to Free or choose another identification method."));
      parameters = parameters.map((parameter) => parameter.id === effect.parameter && parameter.kind === "fixed"
        ? { kind: "free", id: parameter.id, label: parameter.label, target: parameter.target, start: 0.7, lower: null, upper: null, equality_label: null, group_overrides: [] }
        : parameter);
    }
    constraints.push({
      kind: "linear",
      id: nativeSemEffectsCodingConstraintIdV4(node.id),
      terms: [...effects].sort((left, right) => left.parameter.localeCompare(right.parameter)).map((effect) => ({ parameter: effect.parameter, coefficient: 1 })),
      value: effects.length,
    });
    variable.identification = { kind: "effects_coding" };
  }
  return { parameters, constraints, diagnostics };
}

function validateNewLocationEntry(
  entry: SemModelV4ParameterAuthoringEntry,
  variables: ReadonlyMap<string, SemVariableV4>,
  indicatorOwner: ReadonlyMap<string, string>,
): NativeSemParameterAuthoringDiagnosticV4 | null {
  const target = entry.target;
  if (target.kind === "variance" && target.endpoint.kind === "variable") {
    const variable = variables.get(target.endpoint.id);
    const constructId = target.endpoint.id.startsWith("construct:") ? target.endpoint.id.slice("construct:".length) : "";
    if (variable?.kind === "common_factor" && entry.parameter_id === nativeSemFixedFactorVarianceParameterIdV4(constructId)) return null;
  }
  if (target.kind !== "intercept" && target.kind !== "mean" && target.kind !== "threshold") return diagnostic("sem_model_v4.parameter.id_orphaned", entry.parameter_id, `Parameter ${entry.parameter_id} does not match a generated model parameter.`, "Remove the stale edit and edit a current Parameter Table row.");
  const variable = variables.get(target.variable);
  if (target.kind === "mean") {
    const constructId = target.variable.startsWith("construct:") ? target.variable.slice("construct:".length) : "";
    if (variable?.kind !== "common_factor" || entry.parameter_id !== nativeSemLatentMeanParameterIdV4(constructId)) return diagnostic("sem_model_v4.mean.target_invalid", entry.parameter_id, "Latent mean must target its owning common factor with the stable generated ID.", "Add the mean from that factor's Variable row.");
    if (entry.specification.kind !== "free") return diagnostic("sem_model_v4.mean.fixed_unsupported", entry.parameter_id, "The current factor mean policy supports fixed zero or an estimated free mean.", "Remove the mean to fix it at zero, or use a Free parameter.");
    return null;
  }
  const indicator = target.variable.startsWith("observed:") ? target.variable.slice("observed:".length) : "";
  if (variable?.kind !== "observed" || !indicatorOwner.has(indicator)) return diagnostic("sem_model_v4.location.target_invalid", entry.parameter_id, "Observed location parameter must target an indicator in this model.", "Add the parameter from that indicator's Variable row.");
  if (target.kind === "intercept") {
    if (entry.parameter_id !== nativeSemObservedInterceptParameterIdV4(indicator)) return diagnostic("sem_model_v4.intercept.id_invalid", entry.parameter_id, "Observed intercept ID is not stable for its indicator.", "Remove and add the intercept again from the Variable row.");
    if (variable.scale === "ordinal" || variable.scale === "nominal" || variable.scale === "identifier") return diagnostic("sem_model_v4.intercept.scale_invalid", entry.parameter_id, `Observed intercept is not available for ${variable.scale} indicator ${indicator}.`, "Use ordinal thresholds where applicable, or correct the variable scale.");
    return null;
  }
  const categories = variable.categories.length;
  if (variable.scale !== "ordinal" || target.index < 1 || target.index >= categories || entry.parameter_id !== nativeSemOrdinalThresholdParameterIdV4(indicator, target.index)) return diagnostic("sem_model_v4.threshold.target_invalid", entry.parameter_id, "Ordinal threshold index, scale, categories, or stable ID is invalid.", "Confirm ordinal metadata with at least two categories, then add thresholds again from the Variable row.");
  return null;
}

function validateEntryOwnership(
  entry: SemModelV4ParameterAuthoringEntry,
  source: { kind: "node" | "edge"; id: string },
  nodeById: ReadonlyMap<string, Node<ConstructData>>,
  indicatorOwner: ReadonlyMap<string, string>,
  edges: readonly Edge[],
): NativeSemParameterAuthoringDiagnosticV4[] {
  const target = entry.target;
  if (source.kind === "node") {
    const node = nodeById.get(source.id);
    if (!node) return [diagnostic("sem_model_v4.parameter.owner_missing", entry.parameter_id, `Owning construct ${source.id} is missing.`, "Remove the orphaned parameter edit.")];
    const ownsVariable = (id: string) => id === `construct:${node.id}` || id.startsWith("observed:") && indicatorOwner.get(id.slice("observed:".length)) === node.id;
    const owned = target.kind === "loading" ? target.construct === `construct:${node.id}` && ownsVariable(target.indicator)
      : target.kind === "weight" ? target.composite === `construct:${node.id}` && ownsVariable(target.indicator)
        : target.kind === "variance" ? ownsVariable(target.endpoint.id)
          : target.kind === "intercept" || target.kind === "mean" || target.kind === "threshold" ? ownsVariable(target.variable)
            : false;
    return owned ? [] : [diagnostic("sem_model_v4.parameter.owner_mismatch", entry.parameter_id, `Parameter ${entry.parameter_id} is not owned by construct ${node.id}.`, "Remove the stale edit and edit the row through its current source link.")];
  }
  const edge = edges.find((candidate) => candidate.id === source.id);
  if (!edge) return [diagnostic("sem_model_v4.parameter.owner_missing", entry.parameter_id, `Owning relationship ${source.id} is missing.`, "Remove the orphaned parameter edit.")];
  const ownsEndpoint = (id: string) => id === `construct:${edge.source}` || id === `construct:${edge.target}`
    || id.startsWith("observed:") && [edge.source, edge.target].includes(indicatorOwner.get(id.slice("observed:".length)) ?? "");
  const owned = target.kind === "regression" && target.source === `construct:${edge.source}` && target.target === `construct:${edge.target}`
    || target.kind === "covariance" && ownsEndpoint(target.left.id) && ownsEndpoint(target.right.id);
  return owned ? [] : [diagnostic("sem_model_v4.parameter.owner_mismatch", entry.parameter_id, `Parameter ${entry.parameter_id} is not owned by relationship ${edge.id}.`, "Remove the stale edit and edit the row through its current source link.")];
}

function parseEntry(value: unknown, subject: string): { ok: true; entry: SemModelV4ParameterAuthoringEntry } | { ok: false; diagnostic: NativeSemParameterAuthoringDiagnosticV4 } {
  if (!isRecord(value) || !exactKeys(value, ["parameter_id", "target", "specification"]) || !requiredText(value.parameter_id)) return invalidEntry(subject);
  const target = parseTarget(value.target);
  const specification = parseSpecification(value.specification);
  if (!target || !specification) return invalidEntry(String(value.parameter_id));
  return { ok: true, entry: { parameter_id: String(value.parameter_id), target, specification } };
}

function invalidEntry(subject: string) {
  return { ok: false as const, diagnostic: diagnostic("sem_model_v4.parameter.entry_invalid", subject, "Parameter authoring entry cannot be read safely.", "Restore this parameter from the current Parameter Table row; group overrides are not available here.") };
}

function parseSpecification(value: unknown): SemModelV4ParameterAuthoringSpecification | null {
  if (!isRecord(value)) return null;
  if (value.kind === "fixed" && exactKeys(value, ["kind", "value"]) && typeof value.value === "number") return { kind: "fixed", value: value.value };
  if (value.kind === "free" && exactKeys(value, ["kind", "start", "lower", "upper", "equality_label"])
    && [value.start, value.lower, value.upper].every((item) => item === null || typeof item === "number")
    && (value.equality_label === null || typeof value.equality_label === "string")) return {
    kind: "free",
    start: value.start as number | null,
    lower: value.lower as number | null,
    upper: value.upper as number | null,
    equality_label: value.equality_label as string | null,
  };
  return null;
}

function parseTarget(value: unknown): SemModelV4ParameterAuthoringTarget | null {
  if (!isRecord(value) || !requiredText(value.kind)) return null;
  if (value.kind === "loading" && exactKeys(value, ["kind", "construct", "indicator"]) && requiredText(value.construct) && requiredText(value.indicator)) return { kind: "loading", construct: String(value.construct), indicator: String(value.indicator) };
  if (value.kind === "weight" && exactKeys(value, ["kind", "indicator", "composite"]) && requiredText(value.indicator) && requiredText(value.composite)) return { kind: "weight", indicator: String(value.indicator), composite: String(value.composite) };
  if (value.kind === "regression" && exactKeys(value, ["kind", "source", "target"]) && requiredText(value.source) && requiredText(value.target)) return { kind: "regression", source: String(value.source), target: String(value.target) };
  if (value.kind === "variance" && exactKeys(value, ["kind", "endpoint"]) && parseEndpoint(value.endpoint)) return { kind: "variance", endpoint: parseEndpoint(value.endpoint)! };
  if (value.kind === "covariance" && exactKeys(value, ["kind", "left", "right"]) && parseEndpoint(value.left) && parseEndpoint(value.right)) return { kind: "covariance", left: parseEndpoint(value.left)!, right: parseEndpoint(value.right)! };
  if ((value.kind === "intercept" || value.kind === "mean") && exactKeys(value, ["kind", "variable"]) && requiredText(value.variable)) return { kind: value.kind, variable: String(value.variable) };
  if (value.kind === "threshold" && exactKeys(value, ["kind", "variable", "index"]) && requiredText(value.variable) && Number.isInteger(value.index)) return { kind: "threshold", variable: String(value.variable), index: Number(value.index) };
  return null;
}

function parseEndpoint(value: unknown): Extract<SemModelV4ParameterAuthoringTarget, { kind: "variance" }> ["endpoint"] | null {
  if (!isRecord(value) || !exactKeys(value, ["kind", "id"]) || !["variable", "residual_of", "disturbance_of"].includes(String(value.kind)) || !requiredText(value.id)) return null;
  return { kind: value.kind as "variable" | "residual_of" | "disturbance_of", id: String(value.id) };
}

function parameterFromEntry(entry: SemModelV4ParameterAuthoringEntry, label: string, target: SemParameterTargetV4): SemParameterV4 {
  return entry.specification.kind === "fixed"
    ? { kind: "fixed", id: entry.parameter_id, label, target, value: entry.specification.value, group_overrides: [] }
    : {
      kind: "free",
      id: entry.parameter_id,
      label,
      target,
      start: entry.specification.start,
      lower: entry.specification.lower,
      upper: entry.specification.upper,
      equality_label: entry.specification.equality_label,
      group_overrides: [],
    };
}

function defaultFreeLoading(parameter: SemParameterV4): Extract<SemParameterV4, { kind: "free" }> {
  return {
    kind: "free",
    id: parameter.id,
    label: parameter.label,
    target: parameter.target,
    start: 0.7,
    lower: null,
    upper: null,
    equality_label: null,
    group_overrides: [],
  };
}

function locationLabel(target: SemModelV4ParameterAuthoringTarget, variables: ReadonlyMap<string, SemVariableV4>): string {
  const label = "variable" in target ? variables.get(target.variable)?.label ?? target.variable : "Parameter";
  if (target.kind === "intercept") return `Intercept(${label})`;
  if (target.kind === "mean") return `Mean(${label})`;
  if (target.kind === "threshold") return `Threshold ${target.index}(${label})`;
  return label;
}

function normalizeIdentification(value: SemModelV4FactorIdentificationAuthoring, node: Node<ConstructData>): SemModelV4FactorIdentificationAuthoring {
  if (value.kind === "marker_loading") {
    if (!node.data.indicators.includes(value.indicator)) throw new Error("Choose an indicator assigned to this factor as the marker.");
    return { kind: "marker_loading", indicator: value.indicator };
  }
  if (value.kind === "effects_coding" && node.data.indicators.length < 3) throw new Error("Effects coding requires at least three indicators.");
  return { kind: value.kind };
}

function replaceEntry(
  current: readonly SemModelV4ParameterAuthoringEntry[],
  entry: SemModelV4ParameterAuthoringEntry | null,
  parameterId: string,
): SemModelV4ParameterAuthoringEntry[] {
  const next = current.filter((candidate) => candidate.parameter_id !== parameterId);
  if (entry) next.push(structuredClone(entry));
  return next.sort((left, right) => left.parameter_id.localeCompare(right.parameter_id));
}

function cloneTarget(target: SemParameterTargetV4 | SemModelV4ParameterAuthoringTarget): SemModelV4ParameterAuthoringTarget {
  return structuredClone(target) as SemModelV4ParameterAuthoringTarget;
}

function targetEquals(left: SemParameterTargetV4, right: SemModelV4ParameterAuthoringTarget): boolean {
  return JSON.stringify(canonicalTarget(left)) === JSON.stringify(canonicalTarget(right));
}

function canonicalTarget(target: SemParameterTargetV4 | SemModelV4ParameterAuthoringTarget): SemParameterTargetV4 | SemModelV4ParameterAuthoringTarget {
  if (target.kind !== "covariance") return target;
  const pair = [target.left, target.right].sort((left, right) => `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`));
  return { kind: "covariance", left: pair[0], right: pair[1] };
}

function diagnostic(code: string, subject: string, message: string, correctiveAction: string): NativeSemParameterAuthoringDiagnosticV4 {
  return { code, subject, message, corrective_action: correctiveAction };
}

function sortDiagnostics(values: readonly NativeSemParameterAuthoringDiagnosticV4[]): NativeSemParameterAuthoringDiagnosticV4[] {
  return [...values].sort((left, right) => left.code.localeCompare(right.code) || left.subject.localeCompare(right.subject));
}

function requiredText(value: unknown): boolean {
  return typeof value === "string" && Boolean(value.trim());
}

function exactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const current = Object.keys(value);
  return current.length === keys.length && current.every((key) => keys.includes(key));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function human(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function freeze<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) freeze(child);
  }
  return value;
}
