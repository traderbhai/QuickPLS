import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ConstructData, PathEdgeData } from "../types";
import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
} from "../domain/semModelV4";
import { generalSemHocApproachTypeSupportedV1 } from "../domain/generalSemHigherOrderContractV1";

export const NATIVE_HIGHER_ORDER_METHOD = "two_stage" as const;
export const NATIVE_HIGHER_ORDER_SCOPE_LABEL = "Bounded second-order PLS higher-order construct";

export type NativeHigherOrderEditableApproach = Exclude<HigherOrderConstructionApproachV4, "hybrid">;
export type NativeHigherOrderComponentMode = "reflective" | "formative";
export type NativeHigherOrderConceptualDirection = "hoc_explains_components" | "components_form_hoc";

export const NATIVE_HIGHER_ORDER_EDITABLE_APPROACHES = [
  "repeated_indicators",
  "extended_repeated_indicators",
  "embedded_two_stage",
  "disjoint_two_stage",
] as const satisfies readonly NativeHigherOrderEditableApproach[];

export const NATIVE_HIGHER_ORDER_APPROACH_LABELS: Readonly<Record<NativeHigherOrderEditableApproach, string>> = {
  repeated_indicators: "Repeated indicators",
  extended_repeated_indicators: "Extended repeated indicators",
  embedded_two_stage: "Embedded two-stage",
  disjoint_two_stage: "Disjoint two-stage",
};

export interface NativeHigherOrderComponentOption {
  id: string;
  label: string;
  shortName: string;
  mode: NativeHigherOrderComponentMode;
  eligible: boolean;
  reason: string | null;
}

export interface NativeHigherOrderDraft {
  name: string;
  /** Presentation metadata for legacy canvas projects. Strict projects derive it from the label. */
  shortName: string;
  components: string[];
  approach?: NativeHigherOrderEditableApproach;
  measurementType?: HigherOrderMeasurementTypeV4;
  initialPath?: {
    direction: "hoc_to_construct" | "construct_to_hoc";
    constructId: string;
  };
}

export type NativeHigherOrderDraftIssueField = "name" | "components" | "direction" | "approach" | "initial_path";

export interface NativeHigherOrderDraftIssue {
  field: NativeHigherOrderDraftIssueField;
  message: string;
}

export interface NativeHigherOrderDraftValidationContext {
  /** Output/node identity of the HOC being edited. Its current components remain selectable. */
  editingHigherOrderId?: string;
  /** Immutable calculation-ready revisions require one initial authored structural path. */
  requireInitialPath?: boolean;
  /** Existing HOC topology during edit. Null means the path direction has not been authored yet. */
  hocIsEndogenous?: boolean | null;
}

export interface NativeHigherOrderApproachOption {
  approach: NativeHigherOrderEditableApproach;
  label: string;
  valid: boolean;
  recommended: boolean;
  reason: string | null;
}

export interface NativeHigherOrderApproachOptionsInput {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  components: readonly string[];
  measurementType: HigherOrderMeasurementTypeV4 | null;
  hocIsEndogenous: boolean | null;
  editingHigherOrderId?: string;
}

export const DEFAULT_NATIVE_HIGHER_ORDER_APPROACH = "disjoint_two_stage" as const;
export const DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE = "reflective_reflective" as const;

export function nativeHigherOrderDraftApproach(
  draft: Pick<NativeHigherOrderDraft, "approach">,
): NativeHigherOrderEditableApproach {
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

export function nativeHigherOrderConceptualDirection(
  measurementType: HigherOrderMeasurementTypeV4,
): NativeHigherOrderConceptualDirection {
  return measurementType.endsWith("_reflective")
    ? "hoc_explains_components"
    : "components_form_hoc";
}

export function nativeHigherOrderMeasurementType(
  componentMode: NativeHigherOrderComponentMode,
  direction: NativeHigherOrderConceptualDirection,
): HigherOrderMeasurementTypeV4 {
  if (componentMode === "reflective") {
    return direction === "hoc_explains_components" ? "reflective_reflective" : "reflective_formative";
  }
  return direction === "hoc_explains_components" ? "formative_reflective" : "formative_formative";
}

export function nativeHigherOrderMeasurementCode(measurementType: HigherOrderMeasurementTypeV4): "RR" | "RF" | "FR" | "FF" {
  if (measurementType === "reflective_reflective") return "RR";
  if (measurementType === "reflective_formative") return "RF";
  if (measurementType === "formative_reflective") return "FR";
  return "FF";
}

export function nativeHigherOrderSelectedComponentMode(
  componentIds: readonly string[],
  nodes: readonly Node<ConstructData>[],
): NativeHigherOrderComponentMode | null {
  if (!componentIds.length) return null;
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const modes = new Set(componentIds.map((id) => byId.get(id)?.data.mode).filter(
    (mode): mode is NativeHigherOrderComponentMode => mode === "reflective" || mode === "formative",
  ));
  return modes.size === 1 && componentIds.every((id) => byId.has(id)) ? [...modes][0]! : null;
}

export function nativeHigherOrderSuggestedShortName(
  name: string,
  nodes: readonly Node<ConstructData>[],
  editingHigherOrderId?: string,
): string {
  const words = name.normalize("NFKC").match(/[\p{L}\p{N}]+/gu) ?? [];
  const acronym = words.map((word) => [...word][0]).join("").toUpperCase();
  const compact = name.normalize("NFKD").replace(/[^A-Za-z0-9]+/g, "").toUpperCase();
  const base = (acronym.length >= 2 ? acronym : compact || "HOC").slice(0, 12);
  const used = new Set(nodes
    .filter((node) => node.id !== editingHigherOrderId)
    .map((node) => node.data.shortName.trim().normalize("NFKC").toLowerCase()));
  if (!used.has(base.toLowerCase())) return base;
  for (let suffix = 2; suffix < 10_000; suffix += 1) {
    const text = String(suffix);
    const candidate = `${base.slice(0, Math.max(1, 12 - text.length))}${text}`;
    if (!used.has(candidate.toLowerCase())) return candidate;
  }
  return "HOC";
}

function edgeRole(edge: Edge): PathEdgeData["role"] {
  return (edge.data as PathEdgeData | undefined)?.role;
}

export function isNativeStructuralEdge(edge: Edge): boolean {
  return !edge.id.startsWith("measurement::") && edgeRole(edge) !== "control" && edgeRole(edge) !== "covariance";
}

function claimedHigherOrderComponents(
  nodes: readonly Node<ConstructData>[],
  editingHigherOrderId?: string,
): Set<string> {
  return new Set(nodes
    .filter((node) => node.data.semantic === "higher_order"
      && node.id !== editingHigherOrderId
      && node.data.higherOrder?.id !== editingHigherOrderId)
    .flatMap((node) => node.data.higherOrder?.components ?? []));
}

/** Base measured-component inventory, independent of a construction approach. */
export function nativeHigherOrderComponentCandidates(
  nodes: readonly Node<ConstructData>[],
  editingHigherOrderId?: string,
): NativeHigherOrderComponentOption[] {
  const claimedComponents = claimedHigherOrderComponents(nodes, editingHigherOrderId);
  return nodes
    .filter((node) => !node.data.semantic)
    .map((node) => {
      const reason = node.data.indicators.length === 0
        ? "Assign at least one indicator first."
        : claimedComponents.has(node.id)
          ? "This construct already belongs to another higher-order construct."
          : null;
      return {
        id: node.id,
        label: node.data.label.trim() || node.id,
        shortName: node.data.shortName.trim() || node.id,
        mode: node.data.mode,
        eligible: reason === null,
        reason,
      };
    });
}

export function nativeHigherOrderComponentOptions(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  approach: NativeHigherOrderEditableApproach = DEFAULT_NATIVE_HIGHER_ORDER_APPROACH,
  measurementType: HigherOrderMeasurementTypeV4 = DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE,
  editingHigherOrderId?: string,
): NativeHigherOrderComponentOption[] {
  const structurallyConnected = new Set(edges.filter(isNativeStructuralEdge).flatMap((edge) => [edge.source, edge.target]));
  const locMode = nativeHigherOrderLocMode(measurementType);
  return nativeHigherOrderComponentCandidates(nodes, editingHigherOrderId).map((option) => {
    const reason = option.reason
      ?? (option.mode !== locMode
        ? `Choose ${locMode === "reflective" ? "reflective (Mode A)" : "formative (Mode B)"} lower-order components together.`
        : approach === "disjoint_two_stage" && structurallyConnected.has(option.id)
          ? "Disjoint two-stage requires measurement-only lower-order components."
          : null);
    return { ...option, eligible: reason === null, reason };
  });
}

/**
 * Broad command gate. When approach/type are omitted, admit any model with at
 * least two measured ordinary components sharing one measurement mode. Exact
 * approach validation belongs inside the dialog after the analyst's choices.
 */
export function nativeHigherOrderCreationBlocker(
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  approach?: NativeHigherOrderEditableApproach,
  measurementType?: HigherOrderMeasurementTypeV4,
): string | null {
  if (nodes.some((node) => node.data.semantic === "interaction")) {
    return "Remove the moderating-effect interaction before creating a higher-order construct; this workflow does not combine both features.";
  }
  if (nodes.some((node) => node.data.semantic === "higher_order")) {
    return "Create exactly one higher-order construct per model.";
  }
  if (approach && measurementType) {
    const eligible = nativeHigherOrderComponentOptions(nodes, edges, approach, measurementType)
      .filter((option) => option.eligible);
    return eligible.length < 2
      ? "Choose at least two measured lower-order components valid for this approach."
      : null;
  }
  const candidates = nativeHigherOrderComponentCandidates(nodes).filter((option) => option.eligible);
  const reflective = candidates.filter((option) => option.mode === "reflective").length;
  const formative = candidates.filter((option) => option.mode === "formative").length;
  return Math.max(reflective, formative) < 2
    ? "Create at least two measured lower-order components using the same Mode A/B measurement first."
    : null;
}

function approachTopologyReason(
  approach: NativeHigherOrderEditableApproach,
  measurementType: HigherOrderMeasurementTypeV4,
  hocIsEndogenous: boolean | null,
): string {
  if (approach === "extended_repeated_indicators") {
    return measurementType === "reflective_formative" || measurementType === "formative_formative"
      ? "Extended repeated indicators require an endogenous formative HOC."
      : "Extended repeated indicators support formative HOCs only.";
  }
  if (approach === "repeated_indicators"
    && (measurementType === "reflective_formative" || measurementType === "formative_formative")) {
    return hocIsEndogenous === true
      ? "Repeated indicators support this formative HOC only when it is exogenous."
      : "Repeated indicators require this formative HOC to remain exogenous.";
  }
  return "This approach, HCM type, and structural position are not supported together.";
}

export function nativeHigherOrderApproachOptions(
  input: NativeHigherOrderApproachOptionsInput,
): NativeHigherOrderApproachOption[] {
  const uniqueComponents = [...new Set(input.components)];
  const structuralNodes = new Set(input.edges.filter(isNativeStructuralEdge).flatMap((edge) => [edge.source, edge.target]));
  const hasConnectedComponent = uniqueComponents.some((component) => structuralNodes.has(component));
  const provisional = NATIVE_HIGHER_ORDER_EDITABLE_APPROACHES.map((approach): NativeHigherOrderApproachOption => {
    if (uniqueComponents.length < 2) {
      return { approach, label: NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach], valid: false, recommended: false, reason: "Choose at least two dimensions." };
    }
    if (!input.measurementType) {
      return { approach, label: NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach], valid: false, recommended: false, reason: "Choose dimensions with one common measurement mode." };
    }
    const byId = new Map(nativeHigherOrderComponentOptions(
      input.nodes,
      input.edges,
      approach,
      input.measurementType,
      input.editingHigherOrderId,
    ).map((option) => [option.id, option]));
    const invalidComponent = uniqueComponents.map((id) => byId.get(id)).find((option) => !option?.eligible);
    if (invalidComponent) {
      return {
        approach,
        label: NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach],
        valid: false,
        recommended: false,
        reason: invalidComponent.reason ?? "A selected dimension is unavailable for this approach.",
      };
    }
    const topologySupported = input.hocIsEndogenous === null
      ? generalSemHocApproachTypeSupportedV1(approach, input.measurementType, false)
        || generalSemHocApproachTypeSupportedV1(approach, input.measurementType, true)
      : generalSemHocApproachTypeSupportedV1(approach, input.measurementType, input.hocIsEndogenous);
    return topologySupported
      ? { approach, label: NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach], valid: true, recommended: false, reason: null }
      : {
          approach,
          label: NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach],
          valid: false,
          recommended: false,
          reason: approachTopologyReason(approach, input.measurementType, input.hocIsEndogenous),
        };
  });
  const valid = new Set(provisional.filter((option) => option.valid).map((option) => option.approach));
  const preferred = hasConnectedComponent && valid.has("embedded_two_stage")
    ? "embedded_two_stage"
    : valid.has("disjoint_two_stage")
      ? "disjoint_two_stage"
      : valid.has("embedded_two_stage")
        ? "embedded_two_stage"
        : provisional.find((option) => option.valid)?.approach ?? null;
  return provisional.map((option) => ({ ...option, recommended: option.approach === preferred }));
}

export function nativeHigherOrderDraftIssues(
  draft: NativeHigherOrderDraft,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  context: NativeHigherOrderDraftValidationContext = {},
): NativeHigherOrderDraftIssue[] {
  const issues: NativeHigherOrderDraftIssue[] = [];
  const name = draft.name.trim();
  const shortName = draft.shortName.trim();
  const components = draft.components.map((component) => component.trim()).filter(Boolean);
  const uniqueComponents = [...new Set(components)];
  const measurementType = nativeHigherOrderDraftMeasurementType(draft);
  const approach = nativeHigherOrderDraftApproach(draft);
  if (!name) issues.push({ field: "name", message: "Enter a higher-order construct name." });
  if (!shortName) issues.push({ field: "name", message: "A short name could not be derived from this name." });
  if (shortName.length > 12) issues.push({ field: "name", message: "The short name must contain at most 12 characters." });
  if (uniqueComponents.length !== components.length) issues.push({ field: "components", message: "Choose each lower-order component only once." });
  if (uniqueComponents.length < 2) issues.push({ field: "components", message: "Choose at least two lower-order components." });

  const candidates = new Map(nativeHigherOrderComponentCandidates(nodes, context.editingHigherOrderId).map((option) => [option.id, option]));
  const unavailableId = uniqueComponents.find((id) => !candidates.get(id)?.eligible);
  if (unavailableId) {
    const unavailable = candidates.get(unavailableId);
    issues.push({
      field: "components",
      message: unavailable?.reason ?? "One or more selected components are unavailable.",
    });
  }
  const componentMode = nativeHigherOrderSelectedComponentMode(uniqueComponents, nodes);
  if (uniqueComponents.length >= 2 && !componentMode) {
    issues.push({ field: "components", message: "All selected dimensions must use the same Mode A/B measurement." });
  }
  if (componentMode) {
    const derived = nativeHigherOrderMeasurementType(componentMode, nativeHigherOrderConceptualDirection(measurementType));
    if (derived !== measurementType) {
      issues.push({ field: "direction", message: "The HCM type must match the selected dimensions and conceptual direction." });
    }
  }

  const pathRequired = context.requireInitialPath === true;
  if (pathRequired && !draft.initialPath) {
    issues.push({ field: "initial_path", message: "Choose one initial structural path for the calculation-ready revision." });
  }
  if (draft.initialPath) {
    const endpoint = nodes.find((node) => node.id === draft.initialPath!.constructId);
    if (!endpoint || endpoint.data.semantic || endpoint.data.indicators.length === 0 || uniqueComponents.includes(endpoint.id)) {
      issues.push({ field: "initial_path", message: "Choose an ordinary measured construct outside the HOC dimensions." });
    }
  }
  const hocIsEndogenous = draft.initialPath
    ? draft.initialPath.direction === "construct_to_hoc"
    : context.hocIsEndogenous ?? null;
  const selectedApproach = nativeHigherOrderApproachOptions({
    nodes,
    edges,
    components: uniqueComponents,
    measurementType: componentMode ? measurementType : null,
    hocIsEndogenous,
    editingHigherOrderId: context.editingHigherOrderId,
  }).find((option) => option.approach === approach);
  if (selectedApproach && !selectedApproach.valid) {
    issues.push({ field: "approach", message: selectedApproach.reason ?? "Choose a valid construction approach." });
  }

  const normalizedName = name.normalize("NFKC").toLowerCase();
  const normalizedShortName = shortName.normalize("NFKC").toLowerCase();
  if (nodes.some((node) => node.id !== context.editingHigherOrderId
    && node.data.label.trim().normalize("NFKC").toLowerCase() === normalizedName)) {
    issues.push({ field: "name", message: "Choose a name that is not already used by another construct." });
  }
  if (nodes.some((node) => node.id !== context.editingHigherOrderId
    && node.data.shortName.trim().normalize("NFKC").toLowerCase() === normalizedShortName)) {
    issues.push({ field: "name", message: "Choose a short name that is not already used by another construct." });
  }
  const seen = new Set<string>();
  return issues.filter((issue) => {
    const key = `${issue.field}\u0000${issue.message}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export function nativeHigherOrderDraftProblems(
  draft: NativeHigherOrderDraft,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
  context: NativeHigherOrderDraftValidationContext = {},
): string[] {
  return nativeHigherOrderDraftIssues(draft, nodes, edges, context).map((issue) => issue.message);
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
    if (!declaration || !declaration.id.trim()) {
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
