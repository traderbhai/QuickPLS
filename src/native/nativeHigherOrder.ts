import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ConstructData, PathEdgeData } from "../types";

export const NATIVE_HIGHER_ORDER_METHOD = "two_stage" as const;
export const NATIVE_HIGHER_ORDER_SCOPE_LABEL = "Reflective–reflective disjoint two-stage";

export interface NativeHigherOrderComponentOption {
  id: string;
  label: string;
  shortName: string;
  eligible: boolean;
  reason: string | null;
}

export interface NativeHigherOrderDraft {
  name: string;
  shortName: string;
  components: string[];
}

function edgeRole(edge: Edge): PathEdgeData["role"] {
  return (edge.data as PathEdgeData | undefined)?.role;
}

export function isNativeStructuralEdge(edge: Edge): boolean {
  return !edge.id.startsWith("measurement::") && edgeRole(edge) !== "control" && edgeRole(edge) !== "covariance";
}

export function nativeHigherOrderComponentOptions(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeHigherOrderComponentOption[] {
  const structurallyConnected = new Set(edges.filter(isNativeStructuralEdge).flatMap((edge) => [edge.source, edge.target]));
  const claimedComponents = new Set(nodes
    .filter((node) => node.data.semantic === "higher_order")
    .flatMap((node) => node.data.higherOrder?.components ?? []));

  return nodes
    .filter((node) => !node.data.semantic)
    .map((node) => {
      const reason = node.data.mode !== "reflective"
        ? "Only reflective lower-order components are supported."
        : node.data.indicators.length === 0
          ? "Assign at least one indicator first."
          : structurallyConnected.has(node.id)
            ? "Disjoint two-stage components must be measurement-only before HOC creation."
            : claimedComponents.has(node.id)
              ? "This construct already belongs to a higher-order construct."
              : null;
      return {
        id: node.id,
        label: node.data.label.trim() || node.id,
        shortName: node.data.shortName.trim() || node.id,
        eligible: reason === null,
        reason,
      };
    });
}

export function nativeHigherOrderCreationBlocker(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): string | null {
  if (nodes.some((node) => node.data.semantic === "interaction")) {
    return "Remove the moderating-effect interaction before creating a higher-order construct; this workflow does not combine both features.";
  }
  if (nodes.some((node) => node.data.semantic === "higher_order")) {
    return "Create exactly one disjoint two-stage higher-order construct per model.";
  }
  if (edges.some((edge) => edgeRole(edge) === "control")) {
    return "Remove or convert control paths before creating a higher-order construct; this workflow does not accept control paths.";
  }
  const eligible = nativeHigherOrderComponentOptions(nodes, edges).filter((option) => option.eligible);
  return eligible.length < 2
    ? "Create at least two reflective, measured, structurally unconnected lower-order constructs first."
    : null;
}

export function nativeHigherOrderDraftProblems(
  draft: NativeHigherOrderDraft,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): string[] {
  const problems: string[] = [];
  const name = draft.name.trim();
  const shortName = draft.shortName.trim();
  const components = draft.components.map((component) => component.trim()).filter(Boolean);
  const eligibleIds = new Set(nativeHigherOrderComponentOptions(nodes, edges)
    .filter((option) => option.eligible)
    .map((option) => option.id));
  if (!name) problems.push("Enter a higher-order construct name.");
  if (!shortName) problems.push("Enter a short name.");
  if (shortName.length > 12) problems.push("The short name must contain at most 12 characters.");
  if (new Set(components).size !== components.length) problems.push("Choose each lower-order component only once.");
  if (components.length < 2) problems.push("Choose at least two lower-order components.");
  if (components.some((component) => !eligibleIds.has(component))) problems.push("One or more selected components are not eligible for disjoint two-stage estimation.");
  const normalizedName = name.normalize("NFKC").toLowerCase();
  const normalizedShortName = shortName.normalize("NFKC").toLowerCase();
  if (nodes.some((node) => node.data.label.trim().normalize("NFKC").toLowerCase() === normalizedName)) {
    problems.push("Choose a name that is not already used by another construct.");
  }
  if (nodes.some((node) => node.data.shortName.trim().normalize("NFKC").toLowerCase() === normalizedShortName)) {
    problems.push("Choose a short name that is not already used by another construct.");
  }
  return [...new Set(problems)];
}

export function canCreateNativeHigherOrder(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): boolean {
  return nativeHigherOrderCreationBlocker(nodes, edges) === null;
}

export function nativeHigherOrderScopeProblems(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  settings: AnalysisUiSettings,
): string[] {
  const higherOrderNodes = nodes.filter((node) => node.data.semantic === "higher_order");
  if (higherOrderNodes.length === 0) return [];
  const problems: string[] = [];
  if (higherOrderNodes.length !== 1) problems.push("Create exactly one higher-order construct per model");
  if (nodes.some((node) => node.data.semantic === "interaction")) problems.push("Higher-order constructs cannot be combined with a moderating effect in this workflow");
  if (edges.some((edge) => edgeRole(edge) === "control")) problems.push("Higher-order constructs cannot be combined with control paths in this workflow");
  if (settings.method !== "pls_pm" || settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0) {
    problems.push("Run the higher-order workflow with PLS-SEM Algorithm only; HOC resampling inference is outside this point-estimate workflow");
  }
  if ((settings.weightingScheme ?? "path") !== "path") problems.push("The higher-order workflow requires path weighting");
  if ((settings.preprocessing ?? "standardized") !== "standardized") problems.push("The higher-order workflow requires standardized result data");
  if (settings.caseWeightColumn?.trim()) problems.push("The higher-order workflow does not support case weights");

  const byId = new Map(nodes.map((node) => [node.id, node]));
  const structuralEdges = edges.filter(isNativeStructuralEdge);
  for (const node of higherOrderNodes) {
    const declaration = node.data.higherOrder;
    if (!declaration || declaration.id !== node.id || declaration.method !== NATIVE_HIGHER_ORDER_METHOD) {
      problems.push("The native workflow supports only a complete disjoint two-stage higher-order declaration");
      continue;
    }
    if (node.data.mode !== "reflective" || node.data.indicators.length !== 0) {
      problems.push("The higher-order construct must remain reflective and indicator-free because component scores are generated in stage 2");
    }
    const componentIds = new Set(declaration.components);
    if (declaration.components.length < 2 || componentIds.size !== declaration.components.length) {
      problems.push("The higher-order construct requires at least two unique lower-order components");
    }
    const components = declaration.components.map((component) => byId.get(component));
    if (components.some((component) => !component || component.data.semantic || component.data.mode !== "reflective" || component.data.indicators.length === 0)) {
      problems.push("Every lower-order component must be an ordinary measured reflective construct");
    }
    if (structuralEdges.some((edge) => componentIds.has(edge.source) || componentIds.has(edge.target))) {
      problems.push("Lower-order components must remain measurement-only in the disjoint two-stage model");
    }
    if (structuralEdges.some((edge) => edge.target === node.id)) {
      problems.push("Use the higher-order construct as an exogenous predictor, not as a structural outcome");
    }
    const outgoing = structuralEdges.filter((edge) => edge.source === node.id);
    if (outgoing.length === 0) problems.push("Connect the higher-order construct to at least one measured outcome before calculation");
    if (structuralEdges.length !== 1 || outgoing.length !== 1) {
      problems.push("Use exactly one higher-order-construct-to-outcome path and no other structural relationships");
    }
    if (outgoing.some((edge) => componentIds.has(edge.target) || byId.get(edge.target)?.data.semantic || !byId.get(edge.target)?.data.indicators.length)) {
      problems.push("Higher-order structural paths must target ordinary measured constructs outside the component set");
    }
  }
  return [...new Set(problems)];
}
