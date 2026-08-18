import type { Edge, Node } from "@xyflow/react";
import type {
  ConstructData,
  PathEdgeData,
  SemModelV4AuthoringEndpoint,
  SemModelV4ConstructAuthoring,
  SemModelV4CovarianceAuthoring,
} from "../types";

export const SEM_MODEL_V4_AUTHORING_VERSION = 1 as const;

export type NativeCovarianceAuthoringInspectionV4 =
  | { state: "scientific"; persisted: true; specification: Extract<SemModelV4CovarianceAuthoring, { kind: "scientific" }> }
  | { state: "presentation_only"; persisted: true; specification: Extract<SemModelV4CovarianceAuthoring, { kind: "presentation_only" }> }
  | { state: "legacy_unspecified"; persisted: boolean; specification: Extract<SemModelV4CovarianceAuthoring, { kind: "legacy_unspecified" }> }
  | { state: "invalid"; persisted: true; specification: null };

export type NativeConstructAuthoringInspectionV4 =
  | { state: "composite"; persisted: true; specification: Extract<SemModelV4ConstructAuthoring, { kind: "composite" }> }
  | { state: "common_factor"; persisted: true; specification: Extract<SemModelV4ConstructAuthoring, { kind: "common_factor" }> }
  | { state: "legacy_estimand_unspecified"; persisted: boolean; specification: Extract<SemModelV4ConstructAuthoring, { kind: "legacy_estimand_unspecified" }> }
  | { state: "invalid"; persisted: true; specification: null };

export interface NativeSemModelV4ExecutionBlocker {
  code:
    | "sem_model_v4.execution_path_not_active"
    | "sem_model_v4.construct_execution_path_not_active"
    | "sem_model_v4.covariance_use_required"
    | "sem_model_v4.authoring_metadata_invalid";
  edge_id: string | null;
  construct_id: string | null;
  message: string;
  corrective_action: string;
}

export class SemModelV4AuthoringError extends Error {
  constructor(public readonly code: string, public readonly subject: string, message: string) {
    super(message);
    this.name = "SemModelV4AuthoringError";
  }
}

export function inspectNativeConstructAuthoringV4(node: Pick<Node<ConstructData>, "data">): NativeConstructAuthoringInspectionV4 {
  const raw = node.data.semModelV4;
  if (raw === undefined) return {
    state: "legacy_estimand_unspecified",
    persisted: false,
    specification: { kind: "legacy_estimand_unspecified" },
  };
  if (!isRecord(raw) || raw.version !== SEM_MODEL_V4_AUTHORING_VERSION || !isRecord(raw.construct)) {
    return { state: "invalid", persisted: true, specification: null };
  }
  const construct = raw.construct;
  if (construct.kind === "composite" && exactKeys(construct, ["kind"])) {
    return { state: "composite", persisted: true, specification: { kind: "composite" } };
  }
  if (construct.kind === "common_factor"
    && exactKeys(construct, ["kind", "marker_indicator"])
    && (construct.marker_indicator === null || typeof construct.marker_indicator === "string" && Boolean(construct.marker_indicator.trim()))) {
    return {
      state: "common_factor",
      persisted: true,
      specification: { kind: "common_factor", marker_indicator: construct.marker_indicator as string | null },
    };
  }
  if (construct.kind === "legacy_estimand_unspecified" && exactKeys(construct, ["kind"])) {
    return { state: "legacy_estimand_unspecified", persisted: true, specification: { kind: "legacy_estimand_unspecified" } };
  }
  return { state: "invalid", persisted: true, specification: null };
}

export function withNativeConstructEstimandV4(
  node: Node<ConstructData>,
  specification: SemModelV4ConstructAuthoring,
): Node<ConstructData> {
  const normalized = normalizeConstructSpecification(specification, node.id);
  if (normalized.kind === "common_factor"
    && normalized.marker_indicator !== null
    && !node.data.indicators.includes(normalized.marker_indicator)) {
    throw new SemModelV4AuthoringError("sem_model_v4.marker_indicator_unknown", node.id, "Choose a marker indicator assigned to this construct.");
  }
  const previous = node.data.semModelV4?.version === SEM_MODEL_V4_AUTHORING_VERSION
    ? node.data.semModelV4
    : undefined;
  const identification = normalized.kind === "common_factor"
    ? previous?.identification?.kind === "marker_loading"
      ? { kind: "marker_loading" as const, indicator: normalized.marker_indicator ?? previous.identification.indicator }
      : previous?.identification ?? (normalized.marker_indicator ? { kind: "marker_loading" as const, indicator: normalized.marker_indicator } : undefined)
    : previous?.identification;
  return {
    ...node,
    data: {
      ...node.data,
      semModelV4: {
        ...(previous ?? {}),
        version: SEM_MODEL_V4_AUTHORING_VERSION,
        construct: normalized,
        ...(identification ? { identification } : {}),
      },
    },
  };
}

export function inspectNativeCovarianceAuthoringV4(edge: Pick<Edge, "data">): NativeCovarianceAuthoringInspectionV4 {
  const data = edge.data;
  const raw = isRecord(data) ? data.semModelV4 : undefined;
  if (raw === undefined) return {
    state: "legacy_unspecified",
    persisted: false,
    specification: { kind: "legacy_unspecified", origin: "legacy_archive" },
  };
  if (!isRecord(raw) || raw.version !== SEM_MODEL_V4_AUTHORING_VERSION || !isRecord(raw.covariance)) {
    return { state: "invalid", persisted: true, specification: null };
  }
  const covariance = raw.covariance;
  if (covariance.kind === "scientific"
    && exactKeys(covariance, ["kind", "origin", "left", "right"])
    && (covariance.origin === "new_authoring" || covariance.origin === "explicit_conversion")
    && validEndpointPair(covariance.left, covariance.right)) {
    return {
      state: "scientific",
      persisted: true,
      specification: {
        kind: "scientific",
        origin: covariance.origin,
        left: cloneEndpoint(covariance.left as SemModelV4AuthoringEndpoint | null),
        right: cloneEndpoint(covariance.right as SemModelV4AuthoringEndpoint | null),
      },
    };
  }
  if (covariance.kind === "presentation_only"
    && exactKeys(covariance, ["kind", "origin"])
    && (covariance.origin === "explicit_conversion" || covariance.origin === "legacy_migration")) {
    return { state: "presentation_only", persisted: true, specification: { kind: "presentation_only", origin: covariance.origin } };
  }
  if (covariance.kind === "legacy_unspecified"
    && exactKeys(covariance, ["kind", "origin"])
    && (covariance.origin === "legacy_archive" || covariance.origin === "role_conversion")) {
    return { state: "legacy_unspecified", persisted: true, specification: { kind: "legacy_unspecified", origin: covariance.origin } };
  }
  return { state: "invalid", persisted: true, specification: null };
}

export function newNativeScientificCovarianceEdgeV4(
  id: string,
  source: string,
  target: string,
  options: Pick<Edge, "type" | "label"> = {},
): Edge {
  requireCovarianceIdentity(id, source, target);
  return {
    id,
    source,
    target,
    type: options.type ?? "default",
    label: options.label ?? "Covariance",
    data: covarianceData({
      kind: "scientific",
      origin: "new_authoring",
      left: null,
      right: null,
    }),
  };
}

export function convertNativeCovarianceToScientificV4(
  edge: Edge,
  endpoints: { left: SemModelV4AuthoringEndpoint | null; right: SemModelV4AuthoringEndpoint | null } = { left: null, right: null },
): Edge {
  requireCovarianceEdge(edge);
  if (!validEndpointPair(endpoints.left, endpoints.right)) {
    throw new SemModelV4AuthoringError("sem_model_v4.covariance_endpoints_invalid", edge.id, "Supply both exact endpoints, or omit both to use the two drawn constructs.");
  }
  return {
    ...edge,
    data: covarianceData({
      kind: "scientific",
      origin: "explicit_conversion",
      left: cloneEndpoint(endpoints.left),
      right: cloneEndpoint(endpoints.right),
    }, edge.data),
  };
}

export function convertNativeCovarianceToPresentationV4(edge: Edge): Edge {
  requireCovarianceEdge(edge);
  return {
    ...edge,
    data: covarianceData({ kind: "presentation_only", origin: "explicit_conversion" }, edge.data),
  };
}

export function markNativeCovarianceRoleConversionV4(edge: Edge): Edge {
  return {
    ...edge,
    data: covarianceData({ kind: "legacy_unspecified", origin: "role_conversion" }, edge.data),
  };
}

export function withoutNativeCovarianceAuthoringV4(data: PathEdgeData & Record<string, unknown>): PathEdgeData & Record<string, unknown> {
  const next = { ...data };
  delete next.semModelV4;
  return next;
}

export function nativeCovariancePairExistsV4(
  edges: readonly Edge[],
  source: string,
  target: string,
  ignoredEdgeId?: string,
) {
  const requested = unorderedPair(source, target);
  return edges.some((edge) => edge.id !== ignoredEdgeId
    && edgeRole(edge) === "covariance"
    && unorderedPair(edge.source, edge.target) === requested);
}

/** Current recipe-v3 execution must call this before any covariance filtering. */
export function semModelV4ExecutionBlockers(
  edges: readonly Edge[],
  nodes: readonly Node<ConstructData>[] = [],
): NativeSemModelV4ExecutionBlocker[] {
  const blockers: NativeSemModelV4ExecutionBlocker[] = [];
  for (const node of nodes) {
    const inspection = inspectNativeConstructAuthoringV4(node);
    if (inspection.state === "composite" || inspection.state === "common_factor") blockers.push({
      code: "sem_model_v4.construct_execution_path_not_active",
      edge_id: null,
      construct_id: node.id,
      message: `The ${inspection.state === "composite" ? "composite" : "common-factor"} representation for ${node.data.label || node.id} is saved for the new SEM execution path, which is not active in this calculation yet.`,
      corrective_action: "In Experimental Labs, clear the representation decision or wait for the compatible execution path before running.",
    });
    else if (inspection.state === "invalid") blockers.push({
      code: "sem_model_v4.authoring_metadata_invalid",
      edge_id: null,
      construct_id: node.id,
      message: `SEM authoring details for construct ${node.id} cannot be read safely.`,
      corrective_action: "Open the construct and choose Composite, Common factor, or Choose later again.",
    });
  }
  for (const edge of edges) {
    const inspection = inspectNativeCovarianceAuthoringV4(edge);
    if (inspection.state === "scientific") blockers.push({
      code: "sem_model_v4.execution_path_not_active",
      edge_id: edge.id,
      construct_id: null,
      message: `Scientific covariance ${edge.id} is saved for the new SEM execution path, which is not active in this calculation yet.`,
      corrective_action: "Change it to Presentation only or remove it before running this method.",
    });
    else if (inspection.state === "legacy_unspecified" && inspection.persisted && inspection.specification.origin === "role_conversion") blockers.push({
      code: "sem_model_v4.covariance_use_required",
      edge_id: edge.id,
      construct_id: null,
      message: `Covariance ${edge.id} still needs a use choice.`,
      corrective_action: "In Experimental Labs, choose Model covariance, Residual/error covariance, Disturbance covariance, or Presentation only before running.",
    });
    else if (inspection.state === "invalid") blockers.push({
      code: "sem_model_v4.authoring_metadata_invalid",
      edge_id: edge.id,
      construct_id: null,
      message: `Covariance details for ${edge.id} cannot be read safely.`,
      corrective_action: "In Experimental Labs, open the covariance and choose one of the four relationship uses again.",
    });
  }
  return blockers;
}

export function covarianceAuthoringLabelV4(inspection: NativeCovarianceAuthoringInspectionV4) {
  if (inspection.state === "scientific") return "Model covariance";
  if (inspection.state === "presentation_only") return "Presentation only";
  if (inspection.state === "invalid") return "Needs correction";
  return "Choose use";
}

function normalizeConstructSpecification(specification: SemModelV4ConstructAuthoring, subject: string): SemModelV4ConstructAuthoring {
  if (specification.kind === "composite" || specification.kind === "legacy_estimand_unspecified") return { kind: specification.kind };
  if (specification.kind === "common_factor"
    && (specification.marker_indicator === null || typeof specification.marker_indicator === "string" && Boolean(specification.marker_indicator.trim()))) {
    return { kind: "common_factor", marker_indicator: specification.marker_indicator };
  }
  throw new SemModelV4AuthoringError("sem_model_v4.construct_estimand_invalid", subject, "Choose Composite, Common factor, or Choose later.");
}

function covarianceData(
  covariance: SemModelV4CovarianceAuthoring,
  current: Edge["data"] = undefined,
): PathEdgeData & Record<string, unknown> {
  const data = { ...((isRecord(current) ? current : {}) as PathEdgeData & Record<string, unknown>) };
  delete data.controlLabel;
  const previous = data.semModelV4?.version === SEM_MODEL_V4_AUTHORING_VERSION
    ? data.semModelV4
    : undefined;
  return {
    ...data,
    role: "covariance",
    semModelV4: {
      ...(previous ?? {}),
      version: SEM_MODEL_V4_AUTHORING_VERSION,
      covariance,
    },
  };
}

function requireCovarianceIdentity(id: string, source: string, target: string) {
  if (![id, source, target].every((value) => typeof value === "string" && Boolean(value.trim())) || source === target) {
    throw new SemModelV4AuthoringError("sem_model_v4.covariance_identity_invalid", id, "A covariance needs a stable id and two distinct construct endpoints.");
  }
}

function requireCovarianceEdge(edge: Edge) {
  requireCovarianceIdentity(edge.id, edge.source, edge.target);
  if (edgeRole(edge) !== "covariance") throw new SemModelV4AuthoringError("sem_model_v4.covariance_role_required", edge.id, "Choose the Covariance relationship type before setting how it is used.");
}

function validEndpointPair(left: unknown, right: unknown) {
  if (left === null && right === null) return true;
  return validEndpoint(left) && validEndpoint(right);
}

function validEndpoint(value: unknown): value is SemModelV4AuthoringEndpoint {
  return isRecord(value)
    && exactKeys(value, ["kind", "id"])
    && ["variable", "residual_of", "disturbance_of"].includes(String(value.kind))
    && typeof value.id === "string"
    && Boolean(value.id.trim());
}

function cloneEndpoint(value: SemModelV4AuthoringEndpoint | null): SemModelV4AuthoringEndpoint | null {
  return value ? { kind: value.kind, id: value.id } : null;
}

function edgeRole(edge: Pick<Edge, "data">): unknown {
  return isRecord(edge.data) ? edge.data.role : undefined;
}

function unorderedPair(left: string, right: string) {
  return [left, right].sort().join("\0");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, allowed: readonly string[]) {
  const keys = Object.keys(value);
  return keys.length === allowed.length && keys.every((key) => allowed.includes(key));
}
