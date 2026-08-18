import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
  type NativeWorkbenchSemModelV4Diagnostic,
  type NativeWorkbenchSemModelV4Trace,
} from "./nativeWorkbenchSemModelV4Adapter";
import type {
  SemConstraintV4,
  SemDerivedTermV4,
  SemEndpointV4,
  SemModelV4,
  SemParameterTargetV4,
  SemParameterV4,
  SemRelationV4,
  SemVariableV4,
} from "./semModelV4";
import { unsupportedNativeSemParameterAuthoringDiagnosticsV4 } from "./semModelV4ParameterAuthoring";

export const SEM_PARAMETER_TABLE_V4_PROJECTION_VERSION = 1 as const;

export type SemParameterTableSectionV4 =
  | "variable"
  | "relation"
  | "parameter"
  | "constraint"
  | "derived_term"
  | "group"
  | "annotation"
  | "presentation"
  | "diagnostic";

export type SemParameterTableClassificationV4 = "scientific" | "presentation" | "unresolved";
export type SemParameterTableSourceKindV4 = "construct" | "edge" | "indicator" | "group" | "model";

export interface SemParameterTableSourceV4 {
  kind: SemParameterTableSourceKindV4;
  id: string;
}

export interface SemParameterTableRowV4 {
  id: string;
  section: SemParameterTableSectionV4;
  classification: SemParameterTableClassificationV4;
  object_kind: string;
  label: string;
  specification: string;
  sem_id: string | null;
  parameter_id: string | null;
  source: SemParameterTableSourceV4;
}

export interface SemParameterTableDiagnosticV4 extends NativeWorkbenchSemModelV4Diagnostic {
  source: SemParameterTableSourceV4;
}

export interface SemParameterTableProjectionV4 {
  projection_version: typeof SEM_PARAMETER_TABLE_V4_PROJECTION_VERSION;
  status: "ready" | "needs_attention";
  model_id: string;
  rows: readonly SemParameterTableRowV4[];
  diagnostics: readonly SemParameterTableDiagnosticV4[];
  counts: Readonly<Record<SemParameterTableClassificationV4, number>>;
}

const SECTION_ORDER: Readonly<Record<SemParameterTableSectionV4, number>> = {
  diagnostic: 0,
  variable: 1,
  relation: 2,
  parameter: 3,
  constraint: 4,
  derived_term: 5,
  group: 6,
  annotation: 7,
  presentation: 8,
};

/**
 * Projects the live graph through the authored SemModelV4 adapter. If authoring
 * intent is unresolved, the table fails closed and shows only typed diagnostic
 * rows; it never invents factor/composite or covariance semantics.
 */
export function projectNativeWorkbenchSemParameterTableV4(
  input: AuthoredNativeWorkbenchToSemModelV4Input,
): SemParameterTableProjectionV4 {
  const adapted = adaptAuthoredNativeWorkbenchToSemModelV4(input);
  if (adapted.ok) {
    const unsupported = unsupportedNativeSemParameterAuthoringDiagnosticsV4(adapted.model);
    if (!unsupported.length) return projectSemModelV4ParameterTable(adapted.model, adapted.trace);
    return diagnosticProjection(input, unsupported.map((diagnostic) => ({
      ...diagnostic,
      stage: "semantics" as const,
      subject: diagnostic.subject || null,
    })));
  }

  return diagnosticProjection(input, adapted.diagnostics);
}

function diagnosticProjection(
  input: AuthoredNativeWorkbenchToSemModelV4Input,
  values: readonly NativeWorkbenchSemModelV4Diagnostic[],
): SemParameterTableProjectionV4 {
  const diagnostics = values.map((diagnostic) => ({
    ...diagnostic,
    source: diagnosticSource(input, diagnostic),
  }));
  const rows = diagnostics.map((diagnostic, index): SemParameterTableRowV4 => ({
    id: `diagnostic:${diagnostic.code}:${diagnostic.subject ?? "model"}:${index}`,
    section: "diagnostic",
    classification: "unresolved",
    object_kind: diagnostic.stage,
    label: diagnostic.message,
    specification: diagnostic.corrective_action,
    sem_id: null,
    parameter_id: null,
    source: diagnostic.source,
  }));
  return freezeProjection({
    projection_version: SEM_PARAMETER_TABLE_V4_PROJECTION_VERSION,
    status: "needs_attention",
    model_id: input.model_id,
    rows: sortRows(rows),
    diagnostics,
    counts: classificationCounts(rows),
  });
}

/** Projects a complete SemModelV4 without mutating or compiling it. */
export function projectSemModelV4ParameterTable(
  model: SemModelV4,
  trace: NativeWorkbenchSemModelV4Trace,
): SemParameterTableProjectionV4 {
  const unsupported = unsupportedNativeSemParameterAuthoringDiagnosticsV4(model);
  if (unsupported.length) return unsupportedModelProjection(model, trace, unsupported);
  const labels = new Map(model.variables.map((variable) => [variable.id, variable.label]));
  const sources = sourceIndex(trace);
  const rows: SemParameterTableRowV4[] = [];

  for (const variable of model.variables) rows.push(variableRow(variable, labels, sources));
  for (const relation of model.relations) rows.push(relationRow(relation, labels, sources));
  for (const parameter of model.parameters) rows.push(parameterRow(parameter, labels, sources));
  for (const constraint of model.constraints) rows.push(constraintRow(constraint, model.parameters, sources));
  for (const term of model.derived_terms) rows.push(derivedTermRow(term, labels, sources));
  rows.push(groupRow(model));

  for (const annotation of model.annotations) {
    const source = sources.annotations.get(annotation.id) ?? { kind: "model", id: model.id };
    rows.push({
      id: `annotation:${annotation.id}`,
      section: "annotation",
      classification: "presentation",
      object_kind: annotation.kind,
      label: annotation.kind === "display_only_covariance"
        ? annotation.label?.trim() || `${labelOf(labels, annotation.left)} with ${labelOf(labels, annotation.right)}`
        : annotation.kind === "caption" ? annotation.text : annotation.text,
      specification: annotation.kind === "display_only_covariance"
        ? `${labelOf(labels, annotation.left)} ↔ ${labelOf(labels, annotation.right)}; does not affect calculations`
        : annotation.kind === "caption" ? "Canvas caption" : `Note for ${annotation.subject}`,
      sem_id: annotation.id,
      parameter_id: null,
      source,
    });
  }

  if (model.presentation.kind === "canvas") {
    for (const shape of model.presentation.shapes) rows.push(presentationRow(model.id, "shape", shape.id, shape.label?.trim() || shape.shape));
    for (const image of model.presentation.images) rows.push(presentationRow(model.id, "image", image.id, image.alt_text));
    for (const line of model.presentation.lines) rows.push(presentationRow(model.id, "line", line.id, line.label?.trim() || "Presentation line"));
  }

  return freezeProjection({
    projection_version: SEM_PARAMETER_TABLE_V4_PROJECTION_VERSION,
    status: "ready",
    model_id: model.id,
    rows: sortRows(rows),
    diagnostics: [],
    counts: classificationCounts(rows),
  });
}

interface ProjectionSourceIndex {
  variables: Map<string, SemParameterTableSourceV4>;
  relations: Map<string, SemParameterTableSourceV4>;
  parameters: Map<string, SemParameterTableSourceV4>;
  annotations: Map<string, SemParameterTableSourceV4>;
}

function unsupportedModelProjection(
  model: SemModelV4,
  trace: NativeWorkbenchSemModelV4Trace,
  values: ReturnType<typeof unsupportedNativeSemParameterAuthoringDiagnosticsV4>,
): SemParameterTableProjectionV4 {
  const sources = sourceIndex(trace);
  const parameters = new Map(model.parameters.map((parameter) => [parameter.id, parameter]));
  const diagnostics: SemParameterTableDiagnosticV4[] = values.map((value) => {
    const parameter = parameters.get(value.subject);
    return {
      ...value,
      stage: "semantics",
      subject: value.subject || null,
      source: sources.parameters.get(value.subject)
        ?? (parameter ? targetSource(parameter.target, sources) : { kind: "model", id: model.id }),
    };
  });
  const rows = diagnostics.map((diagnostic, index): SemParameterTableRowV4 => ({
    id: `diagnostic:${diagnostic.code}:${diagnostic.subject ?? "model"}:${index}`,
    section: "diagnostic",
    classification: "unresolved",
    object_kind: diagnostic.stage,
    label: diagnostic.message,
    specification: diagnostic.corrective_action,
    sem_id: null,
    parameter_id: null,
    source: diagnostic.source,
  }));
  return freezeProjection({
    projection_version: SEM_PARAMETER_TABLE_V4_PROJECTION_VERSION,
    status: "needs_attention",
    model_id: model.id,
    rows: sortRows(rows),
    diagnostics,
    counts: classificationCounts(rows),
  });
}

function sourceIndex(trace: NativeWorkbenchSemModelV4Trace): ProjectionSourceIndex {
  const variables = new Map<string, SemParameterTableSourceV4>();
  for (const [sourceId, semId] of Object.entries(trace.construct_variables)) variables.set(semId, { kind: "construct", id: sourceId });
  for (const [sourceId, semId] of Object.entries(trace.indicator_variables)) variables.set(semId, { kind: "indicator", id: sourceId });

  const relations = new Map<string, SemParameterTableSourceV4>();
  const parameters = new Map<string, SemParameterTableSourceV4>();
  const annotations = new Map<string, SemParameterTableSourceV4>();
  for (const [sourceId, object] of Object.entries(trace.edge_objects)) {
    const source: SemParameterTableSourceV4 = { kind: "edge", id: sourceId };
    if (object.kind === "presentation_annotation") annotations.set(object.sem_id, source);
    else {
      relations.set(object.sem_id, source);
      parameters.set(object.parameter_id, source);
    }
  }
  return { variables, relations, parameters, annotations };
}

function variableRow(
  variable: SemVariableV4,
  labels: ReadonlyMap<string, string>,
  sources: ProjectionSourceIndex,
): SemParameterTableRowV4 {
  const source = sources.variables.get(variable.id) ?? { kind: "model", id: variable.id };
  let specification: string;
  if (variable.kind === "observed") {
    specification = `Observed; ${humanToken(variable.scale)}; ${humanToken(variable.role)}; column ${variable.source_column}`;
  } else if (variable.kind === "composite") {
    specification = `Composite; ${weightingLabel(variable.weighting.kind)}`;
  } else if (variable.kind === "common_factor") {
    specification = `Common factor; ${identificationLabel(variable.identification, labels)}; ${humanToken(variable.disturbance_policy.kind)}`;
  } else specification = "Derived variable";
  return {
    id: `variable:${variable.id}`,
    section: "variable",
    classification: "scientific",
    object_kind: variable.kind,
    label: variable.label,
    specification,
    sem_id: variable.id,
    parameter_id: null,
    source,
  };
}

function relationRow(
  relation: SemRelationV4,
  labels: ReadonlyMap<string, string>,
  sources: ProjectionSourceIndex,
): SemParameterTableRowV4 {
  const source = sources.relations.get(relation.id) ?? relationSource(relation, sources);
  const specification = relation.kind === "measurement_effect"
    ? `${labelOf(labels, relation.construct)} → ${labelOf(labels, relation.indicator)}`
    : relation.kind === "measurement_causal"
      ? `${labelOf(labels, relation.indicator)} → ${labelOf(labels, relation.composite)}`
      : relation.kind === "structural"
        ? `${labelOf(labels, relation.source)} → ${labelOf(labels, relation.target)}`
        : `${endpointLabel(relation.left, labels)} ↔ ${endpointLabel(relation.right, labels)}`;
  return {
    id: `relation:${relation.id}`,
    section: "relation",
    classification: "scientific",
    object_kind: relation.kind,
    label: humanToken(relation.kind),
    specification,
    sem_id: relation.id,
    parameter_id: relation.parameter,
    source,
  };
}

function parameterRow(
  parameter: SemParameterV4,
  labels: ReadonlyMap<string, string>,
  sources: ProjectionSourceIndex,
): SemParameterTableRowV4 {
  const source = sources.parameters.get(parameter.id) ?? targetSource(parameter.target, sources);
  const details = [parameterKindLabel(parameter), targetLabel(parameter.target, labels)];
  if (parameter.kind === "free") {
    if (parameter.start != null) details.push(`start ${numberLabel(parameter.start)}`);
    if (parameter.lower != null || parameter.upper != null) details.push(`bounds ${boundLabel(parameter.lower)} to ${boundLabel(parameter.upper)}`);
    if (parameter.equality_label?.trim()) details.push(`equality ${parameter.equality_label.trim()}`);
  } else if (parameter.kind === "fixed") details.push(`value ${numberLabel(parameter.value)}`);
  else details.push(`expression ${parameter.expression}`);
  if (parameter.group_overrides?.length) details.push(`${parameter.group_overrides.length} group override${parameter.group_overrides.length === 1 ? "" : "s"}`);
  return {
    id: `parameter:${parameter.id}`,
    section: "parameter",
    classification: "scientific",
    object_kind: parameter.target.kind,
    label: parameter.label,
    specification: details.join("; "),
    sem_id: parameter.id,
    parameter_id: parameter.id,
    source,
  };
}

function constraintRow(
  constraint: SemConstraintV4,
  parameters: readonly SemParameterV4[],
  sources: ProjectionSourceIndex,
): SemParameterTableRowV4 {
  const ids = constraint.kind === "equality" ? constraint.parameters
    : constraint.kind === "bound" ? [constraint.parameter]
      : constraint.terms.map((term) => term.parameter);
  const source = sharedParameterSource(ids, parameters, sources) ?? { kind: "model", id: constraint.id };
  const specification = constraint.kind === "equality"
    ? constraint.parameters.join(" = ")
    : constraint.kind === "bound"
      ? `${constraint.parameter}: ${boundLabel(constraint.lower)} to ${boundLabel(constraint.upper)}`
      : `${constraint.terms.map((term) => `${numberLabel(term.coefficient)}×${term.parameter}`).join(" + ")} = ${numberLabel(constraint.value)}`;
  return {
    id: `constraint:${constraint.id}`,
    section: "constraint",
    classification: "scientific",
    object_kind: constraint.kind,
    label: `${humanToken(constraint.kind)} constraint`,
    specification,
    sem_id: constraint.id,
    parameter_id: null,
    source,
  };
}

function derivedTermRow(
  term: SemDerivedTermV4,
  labels: ReadonlyMap<string, string>,
  sources: ProjectionSourceIndex,
): SemParameterTableRowV4 {
  const source = sources.variables.get(term.output) ?? { kind: "model", id: term.id };
  const specification = term.kind === "interaction"
    ? `${labelOf(labels, term.predictor)} × ${labelOf(labels, term.moderator)}; ${humanToken(term.method)}`
    : term.kind === "higher_order"
      ? `${term.components.map((id) => labelOf(labels, id)).join(", ")}; ${humanToken(term.approach)}; ${humanToken(term.measurement_type)}`
      : `${labelOf(labels, term.source)} to degree ${term.degree}`;
  return {
    id: `derived:${term.id}`,
    section: "derived_term",
    classification: "scientific",
    object_kind: term.kind,
    label: labelOf(labels, term.output),
    specification,
    sem_id: term.id,
    parameter_id: null,
    source,
  };
}

function groupRow(model: SemModelV4): SemParameterTableRowV4 {
  if (model.group.kind === "single_group") return {
    id: "group:single_group",
    section: "group",
    classification: "scientific",
    object_kind: "single_group",
    label: "Single group",
    specification: "All included observations are estimated together",
    sem_id: null,
    parameter_id: null,
    source: { kind: "group", id: "single_group" },
  };
  return {
    id: `group:${model.group.grouping_variable}`,
    section: "group",
    classification: "scientific",
    object_kind: "observed_groups",
    label: model.group.grouping_variable,
    specification: model.group.levels.map((level) => `${level.label} (${level.value})`).join(", "),
    sem_id: null,
    parameter_id: null,
    source: { kind: "group", id: model.group.grouping_variable },
  };
}

function presentationRow(modelId: string, kind: string, id: string, label: string): SemParameterTableRowV4 {
  return {
    id: `presentation:${kind}:${id}`,
    section: "presentation",
    classification: "presentation",
    object_kind: kind,
    label,
    specification: "Canvas-only object; does not affect calculations",
    sem_id: id,
    parameter_id: null,
    source: { kind: "model", id: modelId },
  };
}

function relationSource(relation: SemRelationV4, sources: ProjectionSourceIndex): SemParameterTableSourceV4 {
  if (relation.kind === "measurement_effect") return sources.variables.get(relation.construct) ?? { kind: "model", id: relation.id };
  if (relation.kind === "measurement_causal") return sources.variables.get(relation.composite) ?? { kind: "model", id: relation.id };
  if (relation.kind === "structural") return sources.variables.get(relation.target) ?? { kind: "model", id: relation.id };
  return sourceForEndpoint(relation.left, sources) ?? { kind: "model", id: relation.id };
}

function targetSource(target: SemParameterTargetV4, sources: ProjectionSourceIndex): SemParameterTableSourceV4 {
  if (target.kind === "loading") return sources.variables.get(target.construct) ?? { kind: "model", id: target.construct };
  if (target.kind === "weight") return sources.variables.get(target.composite) ?? { kind: "model", id: target.composite };
  if (target.kind === "regression") return sources.variables.get(target.target) ?? { kind: "model", id: target.target };
  if (target.kind === "variance") return sourceForEndpoint(target.endpoint, sources) ?? { kind: "model", id: target.endpoint.id };
  if (target.kind === "covariance") return sourceForEndpoint(target.left, sources) ?? { kind: "model", id: target.left.id };
  return sources.variables.get(target.variable) ?? { kind: "model", id: target.variable };
}

function sourceForEndpoint(endpoint: SemEndpointV4, sources: ProjectionSourceIndex): SemParameterTableSourceV4 | undefined {
  return sources.variables.get(endpoint.id);
}

function sharedParameterSource(
  ids: readonly string[],
  parameters: readonly SemParameterV4[],
  sources: ProjectionSourceIndex,
): SemParameterTableSourceV4 | null {
  const byId = new Map(parameters.map((parameter) => [parameter.id, parameter]));
  const resolved = ids.map((id) => {
    const parameter = byId.get(id);
    return sources.parameters.get(id) ?? (parameter ? targetSource(parameter.target, sources) : null);
  });
  if (!resolved.length || resolved.some((source) => !source)) return null;
  const first = resolved[0]!;
  return resolved.every((source) => source?.kind === first.kind && source.id === first.id) ? first : null;
}

function diagnosticSource(
  input: AuthoredNativeWorkbenchToSemModelV4Input,
  diagnostic: NativeWorkbenchSemModelV4Diagnostic,
): SemParameterTableSourceV4 {
  const subject = diagnostic.subject?.trim();
  if (subject && input.nodes.some((node) => node.id === subject)) return { kind: "construct", id: subject };
  if (subject && input.edges.some((edge) => edge.id === subject)) return { kind: "edge", id: subject };
  if (subject && input.nodes.some((node) => node.data.indicators.includes(subject))) return { kind: "indicator", id: subject };
  return { kind: "model", id: subject || input.model_id || "model" };
}

function targetLabel(target: SemParameterTargetV4, labels: ReadonlyMap<string, string>): string {
  if (target.kind === "loading") return `${labelOf(labels, target.construct)} → ${labelOf(labels, target.indicator)}`;
  if (target.kind === "weight") return `${labelOf(labels, target.indicator)} → ${labelOf(labels, target.composite)}`;
  if (target.kind === "regression") return `${labelOf(labels, target.source)} → ${labelOf(labels, target.target)}`;
  if (target.kind === "variance") return `Variance of ${endpointLabel(target.endpoint, labels)}`;
  if (target.kind === "covariance") return `${endpointLabel(target.left, labels)} ↔ ${endpointLabel(target.right, labels)}`;
  if (target.kind === "intercept") return `Intercept of ${labelOf(labels, target.variable)}`;
  if (target.kind === "mean") return `Mean of ${labelOf(labels, target.variable)}`;
  return `Threshold ${target.index} of ${labelOf(labels, target.variable)}`;
}

function endpointLabel(endpoint: SemEndpointV4, labels: ReadonlyMap<string, string>): string {
  const label = labelOf(labels, endpoint.id);
  if (endpoint.kind === "residual_of") return `Residual(${label})`;
  if (endpoint.kind === "disturbance_of") return `Disturbance(${label})`;
  return label;
}

function identificationLabel(
  identification: Extract<SemVariableV4, { kind: "common_factor" }>["identification"],
  labels: ReadonlyMap<string, string>,
): string {
  if (identification.kind === "marker_loading") return `marker ${labelOf(labels, identification.indicator)}`;
  return humanToken(identification.kind);
}

function parameterKindLabel(parameter: SemParameterV4): string {
  if (parameter.kind === "free") return "Free";
  if (parameter.kind === "fixed") return "Fixed";
  return "Derived";
}

function weightingLabel(kind: Extract<SemVariableV4, { kind: "composite" }>["weighting"]["kind"]): string {
  if (kind === "mode_a") return "Mode A";
  if (kind === "mode_b") return "Mode B";
  return humanToken(kind);
}

function labelOf(labels: ReadonlyMap<string, string>, id: string): string {
  return labels.get(id) ?? id;
}

function humanToken(value: string): string {
  const spaced = value.replaceAll("_", " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function numberLabel(value: number): string {
  return Number.isFinite(value) ? String(value) : "not finite";
}

function boundLabel(value: number | null | undefined): string {
  return value == null ? "unbounded" : numberLabel(value);
}

function sortRows(rows: readonly SemParameterTableRowV4[]): SemParameterTableRowV4[] {
  return [...rows].sort((left, right) => SECTION_ORDER[left.section] - SECTION_ORDER[right.section]
    || left.id.localeCompare(right.id));
}

function classificationCounts(rows: readonly SemParameterTableRowV4[]): Record<SemParameterTableClassificationV4, number> {
  const counts = { scientific: 0, presentation: 0, unresolved: 0 };
  for (const row of rows) counts[row.classification] += 1;
  return counts;
}

function freezeProjection<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) freezeProjection(child);
  }
  return value;
}
