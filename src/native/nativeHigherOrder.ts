import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ConstructData, PathEdgeData } from "../types";
import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
} from "../domain/semModelV4";
import { generalSemHocApproachTypeSupportedV1 } from "../domain/generalSemHigherOrderContractV1";

export const NATIVE_HIGHER_ORDER_METHOD = "two_stage" as const;
export const NATIVE_HIGHER_ORDER_SCOPE_LABEL = "Bounded second-order PLS higher-order construct";

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
  approach?: Exclude<HigherOrderConstructionApproachV4, "hybrid">;
  measurementType?: HigherOrderMeasurementTypeV4;
  initialPath?: {
    direction: "hoc_to_construct" | "construct_to_hoc";
    constructId: string;
  };
}

export const DEFAULT_NATIVE_HIGHER_ORDER_APPROACH = "disjoint_two_stage" as const;
export const DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE = "reflective_reflective" as const;

export function nativeHigherOrderDraftApproach(
  draft: Pick<NativeHigherOrderDraft, "approach">,
): Exclude<HigherOrderConstructionApproachV4, "hybrid"> {
  return draft.approach ?? DEFAULT_NATIVE_HIGHER_ORDER_APPROACH;
}

export function nativeHigherOrderDraftMeasurementType(
  draft: Pick<NativeHigherOrderDraft, "measurementType">,
): HigherOrderMeasurementTypeV4 {
  return draft.measurementType ?? DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE;
}

export function nativeHigherOrderLocMode(
  measurementType: HigherOrderMeasurementTypeV4,
): ConstructData["mode"] {
  return measurementType.startsWith("reflective_") ? "reflective" : "formative";
}

export function nativeHigherOrderHocMode(
  measurementType: HigherOrderMeasurementTypeV4,
): ConstructData["mode"] {
  return measurementType.endsWith("_reflective") ? "reflective" : "formative";
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
  approach: Exclude<HigherOrderConstructionApproachV4, "hybrid"> = DEFAULT_NATIVE_HIGHER_ORDER_APPROACH,
  measurementType: HigherOrderMeasurementTypeV4 = DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE,
): NativeHigherOrderComponentOption[] {
  const structurallyConnected = new Set(edges.filter(isNativeStructuralEdge).flatMap((edge) => [edge.source, edge.target]));
  const claimedComponents = new Set(nodes
    .filter((node) => node.data.semantic === "higher_order")
    .flatMap((node) => node.data.higherOrder?.components ?? []));

  return nodes
    .filter((node) => !node.data.semantic)
    .map((node) => {
      const locMode = nativeHigherOrderLocMode(measurementType);
      const reason = node.data.mode !== locMode
        ? `This HCM type requires ${locMode === "reflective" ? "reflective (Mode A)" : "formative (Mode B)"} lower-order components.`
        : node.data.indicators.length === 0
          ? "Assign at least one indicator first."
          : approach === "disjoint_two_stage" && structurallyConnected.has(node.id)
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
  approach: Exclude<HigherOrderConstructionApproachV4, "hybrid"> = DEFAULT_NATIVE_HIGHER_ORDER_APPROACH,
  measurementType: HigherOrderMeasurementTypeV4 = DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE,
): string | null {
  if (nodes.some((node) => node.data.semantic === "interaction")) {
    return "Remove the moderating-effect interaction before creating a higher-order construct; this workflow does not combine both features.";
  }
  if (nodes.some((node) => node.data.semantic === "higher_order")) {
    return "Create exactly one higher-order construct per model.";
  }
  const eligible = nativeHigherOrderComponentOptions(nodes, edges, approach, measurementType)
    .filter((option) => option.eligible);
  return eligible.length < 2
    ? "Create at least two measured lower-order components with the required Mode A/B measurement first."
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
  const approach = nativeHigherOrderDraftApproach(draft);
  const measurementType = nativeHigherOrderDraftMeasurementType(draft);
  const eligibleIds = new Set(nativeHigherOrderComponentOptions(nodes, edges, approach, measurementType)
    .filter((option) => option.eligible)
    .map((option) => option.id));
  if (!name) problems.push("Enter a higher-order construct name.");
  if (!shortName) problems.push("Enter a short name.");
  if (shortName.length > 12) problems.push("The short name must contain at most 12 characters.");
  if (new Set(components).size !== components.length) problems.push("Choose each lower-order component only once.");
  if (components.length < 2) problems.push("Choose at least two lower-order components.");
  if (components.some((component) => !eligibleIds.has(component))) problems.push("One or more selected components are not eligible for the chosen approach and HCM type.");
  if (draft.initialPath) {
    const endpoint = nodes.find((node) => node.id === draft.initialPath!.constructId);
    if (!endpoint || endpoint.data.semantic || endpoint.data.indicators.length === 0 || components.includes(endpoint.id)) {
      problems.push("Choose an ordinary measured construct outside the HOC component set for the initial structural path.");
    }
    const hocIsEndogenous = draft.initialPath.direction === "construct_to_hoc";
    if (!generalSemHocApproachTypeSupportedV1(approach, measurementType, hocIsEndogenous)) {
      problems.push("The chosen approach/HCM type does not support the selected HOC path direction.");
    }
  }
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
  if ((settings.method !== "pls_pm" && settings.method !== "bootstrap") || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0) {
    problems.push("Run the higher-order workflow with PLS-SEM Algorithm and optional percentile case bootstrap only");
  }
  if ((settings.weightingScheme ?? "path") !== "path") problems.push("The higher-order workflow requires path weighting");
  if ((settings.preprocessing ?? "standardized") !== "standardized") problems.push("The higher-order workflow requires standardized result data");
  if (settings.caseWeightColumn?.trim()) problems.push("The higher-order workflow does not support case weights");
  if (settings.groupColumn?.trim()) problems.push("The higher-order workflow is single-group only");

  const byId = new Map(nodes.map((node) => [node.id, node]));
  const structuralEdges = edges.filter(isNativeStructuralEdge);
  for (const node of higherOrderNodes) {
    const declaration = node.data.higherOrder;
    if (!declaration || declaration.id !== node.id) {
      problems.push("The native workflow requires a complete higher-order declaration");
      continue;
    }
    const approach = declaration.canonicalApproach ?? DEFAULT_NATIVE_HIGHER_ORDER_APPROACH;
    const measurementType = declaration.measurementType ?? DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE;
    if (approach === "hybrid") {
      problems.push("Hybrid higher-order models remain compatibility-only");
      continue;
    }
    if (node.data.mode !== nativeHigherOrderHocMode(measurementType) || node.data.indicators.length !== 0) {
      problems.push("The higher-order construct measurement mode must match the second term of its HCM type and remain indicator-free in the authored model");
    }
    const componentIds = new Set(declaration.components);
    if (declaration.components.length < 2 || componentIds.size !== declaration.components.length) {
      problems.push("The higher-order construct requires at least two unique lower-order components");
    }
    const components = declaration.components.map((component) => byId.get(component));
    const requiredLocMode = nativeHigherOrderLocMode(measurementType);
    if (components.some((component) => !component || component.data.semantic || component.data.mode !== requiredLocMode || component.data.indicators.length === 0)) {
      problems.push("Every lower-order component must be ordinary, measured, and match the first term of the HCM type");
    }
    if (approach === "disjoint_two_stage" && structuralEdges.some((edge) => componentIds.has(edge.source) || componentIds.has(edge.target))) {
      problems.push("Lower-order components must remain measurement-only in the disjoint two-stage model");
    }
    const hocIsEndogenous = structuralEdges.some((edge) => edge.target === node.id);
    if (!generalSemHocApproachTypeSupportedV1(approach, measurementType, hocIsEndogenous)) {
      problems.push("The chosen approach/HCM type does not support the higher-order construct's current exogenous/endogenous position");
    }
    const outgoing = structuralEdges.filter((edge) => edge.source === node.id);
    const incoming = structuralEdges.filter((edge) => edge.target === node.id);
    if (outgoing.length + incoming.length === 0) problems.push("Connect the higher-order construct to at least one ordinary construct before calculation");
    if (outgoing.some((edge) => componentIds.has(edge.target) || byId.get(edge.target)?.data.semantic || !byId.get(edge.target)?.data.indicators.length)) {
      problems.push("Higher-order structural paths must target ordinary measured constructs outside the component set");
    }
    if (incoming.some((edge) => componentIds.has(edge.source) || byId.get(edge.source)?.data.semantic || !byId.get(edge.source)?.data.indicators.length)) {
      problems.push("Higher-order structural antecedents must be ordinary measured constructs outside the component set");
    }
  }
  return [...new Set(problems)];
}
