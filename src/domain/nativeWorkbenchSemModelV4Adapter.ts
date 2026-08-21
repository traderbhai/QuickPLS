import type { Edge, Node } from "@xyflow/react";
import type {
  ConstructData,
  DiagramLayoutState,
  HigherOrderConstructData,
  InteractionV2Data,
} from "../types";
import {
  inspectNativeConstructAuthoringV4,
  inspectNativeCovarianceAuthoringV4,
} from "./semModelV4Authoring";
import { applyNativeSemModelParameterAuthoringV4 } from "./semModelV4ParameterAuthoring";
import { standardSemGeneralSemModerationV3GeneratedAnnotationIdV1 } from "./standardSemModelV4Authority";
import {
  canonicalizeSemModelV4,
  SEM_MODEL_V4_SCHEMA_VERSION,
  validateSemModelV4,
  type FactorIdentificationV4,
  type ObservedRoleV4,
  type ObservedScaleV4,
  type ObservedTransformationStepV4,
  type SemAnnotationV4,
  type SemDataBindingV4,
  type SemDerivedTermV4,
  type SemEndpointV4,
  type SemGroupV4,
  type SemModelV4,
  type SemParameterTargetV4,
  type SemParameterV4,
  type SemPresentationV4,
  type SemRelationV4,
  type SemVariableV4,
} from "./semModelV4";

export const NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION = 2 as const;

export type NativeWorkbenchConstructEstimandV4 =
  | { kind: "composite" }
  | { kind: "common_factor"; marker_indicator?: string | null }
  | { kind: "legacy_estimand_unspecified" };

export type NativeWorkbenchCovarianceSemanticsV4 =
  | {
    kind: "scientific";
    /** Omit both endpoints to use the two construct variables drawn by the edge. */
    left?: SemEndpointV4 | null;
    right?: SemEndpointV4 | null;
  }
  | { kind: "presentation_only" };

export interface NativeWorkbenchObservedSemanticsV4 {
  label?: string;
  scale?: ObservedScaleV4;
  role?: ObservedRoleV4;
  categories?: string[];
  value_labels?: Record<string, string>;
  missing_markers?: string[];
  transformation_lineage?: ObservedTransformationStepV4[];
}

/** Explicit requests only; no flag is ever inferred from the diagram. */
export interface NativeWorkbenchSpecialAssumptionsV4 {
  imply_exogenous_latent_correlations: boolean;
  imply_causal_indicator_correlations: boolean;
  fix_causal_indicator_variances_to_one: boolean;
}

export interface NativeWorkbenchToSemModelV4Input {
  model_id: string;
  model_name: string;
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  diagram_layout?: Partial<DiagramLayoutState> | null;
  data_binding: SemDataBindingV4;
  group?: SemGroupV4;
  /** Every construct must have an explicit factor-versus-composite decision. */
  construct_estimands: Readonly<Record<string, NativeWorkbenchConstructEstimandV4>>;
  /** Every current `role=covariance` edge must have one exact entry. */
  covariance_semantics: Readonly<Record<string, NativeWorkbenchCovarianceSemanticsV4>>;
  /** Keyed by source-column name. Indicator columns are added automatically. */
  observed_semantics?: Readonly<Record<string, NativeWorkbenchObservedSemanticsV4>>;
  /** Reserved parity settings. Any enabled flag fails closed until plan materialization is implemented. */
  special_assumptions?: NativeWorkbenchSpecialAssumptionsV4;
}

export type AuthoredNativeWorkbenchToSemModelV4Input = Omit<
  NativeWorkbenchToSemModelV4Input,
  "construct_estimands" | "covariance_semantics"
>;

export type NativeWorkbenchSemObjectTraceV4 =
  | { kind: "scientific_relation"; sem_id: string; parameter_id: string }
  | { kind: "presentation_annotation"; sem_id: string }
  | { kind: "derived_measurement_relation"; sem_id: string; parameter_id: string };

export interface NativeWorkbenchSemModelV4Trace {
  construct_variables: Record<string, string>;
  indicator_variables: Record<string, string>;
  edge_objects: Record<string, NativeWorkbenchSemObjectTraceV4>;
}

export type NativeWorkbenchSemModelV4DiagnosticStage = "schema" | "semantics" | "data_binding" | "model";

export interface NativeWorkbenchSemModelV4Diagnostic {
  code: string;
  stage: NativeWorkbenchSemModelV4DiagnosticStage;
  subject: string | null;
  message: string;
  corrective_action: string;
}

export type NativeWorkbenchSemModelV4AdapterResult =
  | {
    ok: true;
    adapter_version: typeof NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION;
    model: SemModelV4;
    trace: NativeWorkbenchSemModelV4Trace;
  }
  | {
    ok: false;
    adapter_version: typeof NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION;
    diagnostics: NativeWorkbenchSemModelV4Diagnostic[];
  };

export class NativeWorkbenchSemModelV4AdapterError extends Error {
  constructor(public readonly diagnostics: NativeWorkbenchSemModelV4Diagnostic[]) {
    super(diagnostics[0]?.message ?? "The native workbench model cannot be converted to SemModelV4.");
    this.name = "NativeWorkbenchSemModelV4AdapterError";
  }
}

export const nativeWorkbenchConstructVariableIdV4 = (constructId: string) => `construct:${constructId}`;
export const nativeWorkbenchDerivedVariableIdV4 = (nodeId: string) => `derived:${nodeId}`;
export const nativeWorkbenchObservedVariableIdV4 = (sourceColumn: string) => `observed:${sourceColumn}`;

type ExplicitInteractionV2Node = Node<ConstructData & {
  semantic: "interaction";
  interaction: InteractionV2Data;
}>;

type ExplicitHigherOrderNode = Node<ConstructData & {
  semantic: "higher_order";
  higherOrder: HigherOrderConstructData;
}>;

const isExplicitInteractionV2Node = (node: Node<ConstructData>): node is ExplicitInteractionV2Node =>
  node.data?.semantic === "interaction" && node.data.interaction?.kind === "interaction_v2";

const isExplicitHigherOrderNode = (node: Node<ConstructData>): node is ExplicitHigherOrderNode =>
  node.data?.semantic === "higher_order"
  && node.data.higherOrder !== null
  && typeof node.data.higherOrder === "object";

/**
 * Converts the live editable graph without passing through NativeRecipeModel, whose
 * legacy contract intentionally omits covariance edges. This function is dormant:
 * callers must opt in and supply explicit estimand and covariance classifications.
 */
export function adaptNativeWorkbenchToSemModelV4(
  input: NativeWorkbenchToSemModelV4Input,
): NativeWorkbenchSemModelV4AdapterResult {
  const built = buildNativeWorkbenchSemModelV4(input);
  if (Array.isArray(built)) {
    return deepFreeze({
      ok: false,
      adapter_version: NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION,
      diagnostics: built,
    });
  }
  return deepFreeze({
    ok: true,
    adapter_version: NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION,
    model: built.model,
    trace: built.trace,
  });
}

/** Reads persisted editor metadata without treating missing legacy metadata as scientific. */
export function adaptAuthoredNativeWorkbenchToSemModelV4(
  input: AuthoredNativeWorkbenchToSemModelV4Input,
): NativeWorkbenchSemModelV4AdapterResult {
  const diagnostics: NativeWorkbenchSemModelV4Diagnostic[] = [];
  const construct_estimands: Record<string, NativeWorkbenchConstructEstimandV4> = {};
  for (const node of input.nodes) {
    if (isExplicitInteractionV2Node(node) || isExplicitHigherOrderNode(node)) continue;
    const inspection = inspectNativeConstructAuthoringV4(node);
    if (inspection.state === "invalid") {
      diagnostics.push(diagnostic(
        "native_workbench.construct_authoring_metadata_invalid",
        "schema",
        node.id,
        `SEM authoring details for construct ${node.id} cannot be read safely.`,
        "Open the construct and choose Composite, Common factor, or Choose later again.",
      ));
    } else if (inspection.state === "common_factor") {
      construct_estimands[node.id] = {
        kind: "common_factor",
        marker_indicator: inspection.specification.marker_indicator,
      };
    } else if (inspection.state === "composite") {
      construct_estimands[node.id] = { kind: "composite" };
    } else construct_estimands[node.id] = { kind: "legacy_estimand_unspecified" };
  }

  const covariance_semantics: Record<string, NativeWorkbenchCovarianceSemanticsV4> = {};
  for (const edge of input.edges) {
    if (edgeRole(edge) !== "covariance") continue;
    const inspection = inspectNativeCovarianceAuthoringV4(edge);
    if (inspection.state === "invalid") {
      diagnostics.push(diagnostic(
        "native_workbench.covariance_authoring_metadata_invalid",
        "schema",
        edge.id,
        `SEM authoring details for covariance ${edge.id} cannot be read safely.`,
        "Open the covariance in Experimental Labs and choose one of the four relationship uses again.",
      ));
    } else if (inspection.state === "scientific") {
      covariance_semantics[edge.id] = inspection.specification.left && inspection.specification.right
        ? { kind: "scientific", left: inspection.specification.left, right: inspection.specification.right }
        : { kind: "scientific" };
    } else if (inspection.state === "presentation_only") {
      covariance_semantics[edge.id] = { kind: "presentation_only" };
    }
    // Missing or explicitly legacy-unspecified metadata intentionally leaves the
    // entry absent so the normal classification-required diagnostic is emitted.
  }
  if (diagnostics.length) return deepFreeze({
    ok: false,
    adapter_version: NATIVE_WORKBENCH_SEM_MODEL_V4_ADAPTER_VERSION,
    diagnostics: sortDiagnostics(diagnostics),
  });
  return adaptNativeWorkbenchToSemModelV4({ ...input, construct_estimands, covariance_semantics });
}

export function requireNativeWorkbenchSemModelV4(input: NativeWorkbenchToSemModelV4Input): SemModelV4 {
  const result = adaptNativeWorkbenchToSemModelV4(input);
  if (!result.ok) throw new NativeWorkbenchSemModelV4AdapterError(result.diagnostics);
  return result.model;
}

interface BuiltNativeWorkbenchSemModelV4 {
  model: SemModelV4;
  trace: NativeWorkbenchSemModelV4Trace;
}

interface MeasurementIdentity {
  constructId: string;
  indicator: string;
  relationId: string;
  parameterId: string;
}

function buildNativeWorkbenchSemModelV4(
  input: NativeWorkbenchToSemModelV4Input,
): BuiltNativeWorkbenchSemModelV4 | NativeWorkbenchSemModelV4Diagnostic[] {
  const diagnostics: NativeWorkbenchSemModelV4Diagnostic[] = [];
  diagnostics.push(...specialAssumptionDiagnostics(input.special_assumptions));
  const modelId = requiredText(input.model_id);
  const modelName = requiredText(input.model_name);
  if (!modelId) diagnostics.push(diagnostic(
    "native_workbench.model_id_required",
    "schema",
    "model_id",
    "The live model has no stable id.",
    "Assign a stable model id before creating a SemModelV4 copy.",
  ));
  if (!modelName) diagnostics.push(diagnostic(
    "native_workbench.model_name_required",
    "schema",
    input.model_id || null,
    "The live model has no name.",
    "Name the model before creating a SemModelV4 copy.",
  ));

  const nodesById = new Map<string, Node<ConstructData>>();
  const explicitInteractionNodesById = new Map<string, ExplicitInteractionV2Node>();
  const explicitHigherOrderNodesById = new Map<string, ExplicitHigherOrderNode>();
  const indicatorOwner = new Map<string, string>();
  const nodeIds = new Set(input.nodes.map((node) => node.id));
  for (const node of input.nodes) {
    if (!requiredText(node.id)) {
      diagnostics.push(diagnostic("native_workbench.construct_id_required", "schema", null, "A construct has no stable id.", "Assign a non-empty construct id."));
      continue;
    }
    if (nodesById.has(node.id)) {
      diagnostics.push(diagnostic("native_workbench.construct_id_duplicate", "schema", node.id, `Construct id ${node.id} occurs more than once.`, "Give every construct a unique stable id."));
      continue;
    }
    nodesById.set(node.id, node);
    if (!Number.isFinite(node.position?.x) || !Number.isFinite(node.position?.y)) {
      diagnostics.push(diagnostic("native_workbench.construct_position_invalid", "schema", node.id, `Construct ${node.id} has a non-finite canvas position.`, "Move the construct to a valid finite canvas position."));
    }
    if (!requiredText(node.data?.label) || !requiredText(node.data?.shortName)) {
      diagnostics.push(diagnostic("native_workbench.construct_label_required", "schema", node.id, `Construct ${node.id} needs a name and short name.`, "Enter both construct labels before conversion."));
    }
    const explicitInteractionV2 = isExplicitInteractionV2Node(node);
    const explicitHigherOrder = isExplicitHigherOrderNode(node);
    if ((node.data?.semantic === "interaction" || node.data?.semantic === "higher_order")
      && !explicitInteractionV2
      && !explicitHigherOrder) {
      diagnostics.push(diagnostic(
        "native_workbench.derived_construct_requires_explicit_v4_definition",
        "semantics",
        node.id,
        `Derived construct ${node.id} cannot be inferred from the legacy node shape without changing its scientific meaning.`,
        "Author its interaction or higher-order SemModelV4 term explicitly before conversion.",
      ));
    }
    if (explicitInteractionV2) {
      explicitInteractionNodesById.set(node.id, node);
      if (!Array.isArray(node.data.indicators) || node.data.indicators.length !== 0) {
        diagnostics.push(diagnostic(
          "native_workbench.interaction_v2_indicators_forbidden",
          "semantics",
          node.id,
          `Explicit interaction ${node.id} cannot own measurement indicators.`,
          "Remove indicators from the generated interaction while preserving its operands and focal path.",
        ));
      }
      continue;
    }
    if (explicitHigherOrder) {
      explicitHigherOrderNodesById.set(node.id, node);
      if (!Array.isArray(node.data.indicators) || node.data.indicators.length !== 0) {
        diagnostics.push(diagnostic(
          "native_workbench.higher_order_indicators_forbidden",
          "semantics",
          node.id,
          `Higher-order construct ${node.id} cannot own measurement indicators directly.`,
          "Remove direct indicators from the higher-order construct; its lower-order components supply the measurement information.",
        ));
      }
      continue;
    }
    if (!Array.isArray(node.data?.indicators) || !node.data.indicators.length) {
      diagnostics.push(diagnostic("native_workbench.indicators_required", "semantics", node.id, `Construct ${node.id} has no indicators.`, "Assign at least one indicator; common factors require at least two."));
      continue;
    }
    const local = new Set<string>();
    for (const rawIndicator of node.data.indicators) {
      const indicator = requiredText(rawIndicator);
      if (!indicator) {
        diagnostics.push(diagnostic("native_workbench.indicator_id_required", "schema", node.id, `Construct ${node.id} contains an empty indicator id.`, "Remove or rename the empty indicator."));
        continue;
      }
      if (local.has(indicator)) {
        diagnostics.push(diagnostic("native_workbench.indicator_duplicate", "semantics", indicator, `Indicator ${indicator} occurs twice in construct ${node.id}.`, "Keep each indicator once in its measurement block."));
        continue;
      }
      local.add(indicator);
      const owner = indicatorOwner.get(indicator);
      if (owner && owner !== node.id) {
        diagnostics.push(diagnostic("native_workbench.indicator_owner_ambiguous", "semantics", indicator, `Indicator ${indicator} belongs to both ${owner} and ${node.id}.`, "Choose one measurement owner or author an explicit CB-SEM cross-loading after conversion."));
      } else indicatorOwner.set(indicator, node.id);
    }
  }

  for (const configuredId of Object.keys(input.construct_estimands)) {
    if (!nodeIds.has(configuredId)) diagnostics.push(diagnostic(
      "native_workbench.estimand_construct_unknown",
      "semantics",
      configuredId,
      `An estimand decision references unknown construct ${configuredId}.`,
      "Remove the stale decision or restore the referenced construct.",
    ));
    else if (explicitInteractionNodesById.has(configuredId) || explicitHigherOrderNodesById.has(configuredId)) diagnostics.push(diagnostic(
      "native_workbench.derived_estimand_forbidden",
      "semantics",
      configuredId,
      `Derived construct ${configuredId} cannot have a factor-versus-composite estimand.`,
      "Remove the construct estimand from the generated term.",
    ));
  }
  for (const node of nodesById.values()) {
    if (explicitInteractionNodesById.has(node.id) || explicitHigherOrderNodesById.has(node.id)) continue;
    const estimand = input.construct_estimands[node.id];
    if (!estimand || estimand.kind === "legacy_estimand_unspecified") {
      diagnostics.push(diagnostic(
        "native_workbench.estimand_confirmation_required",
        "semantics",
        node.id,
        `Construct ${node.data.label || node.id} has ambiguous legacy factor-versus-composite semantics.`,
        "Choose Composite for PLS-SEM semantics or Common factor for CB-SEM semantics before conversion.",
      ));
      continue;
    }
    if (estimand.kind !== "composite" && estimand.kind !== "common_factor") {
      diagnostics.push(diagnostic("native_workbench.estimand_kind_unknown", "semantics", node.id, `Construct ${node.id} has an unknown estimand kind.`, "Choose Composite or Common factor."));
      continue;
    }
    if (estimand.kind === "common_factor" && node.data.mode !== "reflective") {
      diagnostics.push(diagnostic(
        "native_workbench.common_factor_formative_unsupported",
        "semantics",
        node.id,
        `Construct ${node.data.label || node.id} is formative in the current graph but was classified as a common factor.`,
        "Use a composite or author a supported causal-indicator factor configuration explicitly.",
      ));
    }
    if (estimand.kind === "common_factor" && node.data.indicators.length < 2) {
      diagnostics.push(diagnostic("native_workbench.factor_indicators_insufficient", "semantics", node.id, `Common factor ${node.data.label || node.id} has fewer than two indicators.`, "Assign at least two effect indicators or use a composite."));
    }
    if (estimand.kind === "common_factor" && estimand.marker_indicator != null && !node.data.indicators.includes(estimand.marker_indicator)) {
      diagnostics.push(diagnostic("native_workbench.marker_indicator_unknown", "semantics", node.id, `Marker indicator ${estimand.marker_indicator} is not assigned to construct ${node.id}.`, "Choose one of this factor's assigned indicators as its marker."));
    }
  }

  const edgesById = new Map<string, Edge>();
  const structuralPairs = new Set<string>();
  const covarianceEdgeIds = new Set<string>();
  for (const edge of input.edges) {
    if (!requiredText(edge.id) || edgesById.has(edge.id)) {
      diagnostics.push(diagnostic("native_workbench.edge_id_duplicate_or_empty", "schema", edge.id || null, "Every live edge needs a unique non-empty id.", "Assign a unique stable id to the edge."));
      continue;
    }
    edgesById.set(edge.id, edge);
    if (isMeasurementEdge(edge)) continue;
    const role = edgeRole(edge);
    if (role !== undefined && role !== "control" && role !== "covariance") {
      diagnostics.push(diagnostic("native_workbench.edge_role_unknown", "semantics", edge.id, `Edge ${edge.id} has unknown role ${String(role)}.`, "Classify it as a structural path, control path, or covariance."));
      continue;
    }
    if (!nodesById.has(edge.source) || !nodesById.has(edge.target)) {
      const unknown = !nodesById.has(edge.source) ? edge.source : edge.target;
      diagnostics.push(diagnostic("native_workbench.edge_endpoint_unknown", "semantics", edge.id, `Edge ${edge.id} references unknown endpoint ${unknown}.`, "Reconnect the edge to existing constructs or delete the stale edge."));
      continue;
    }
    if (edge.source === edge.target) {
      diagnostics.push(diagnostic("native_workbench.edge_self_relation", "semantics", edge.id, `Edge ${edge.id} connects ${edge.source} to itself.`, "Use an explicit variance parameter instead of a self-edge."));
      continue;
    }
    if (role === "covariance") {
      covarianceEdgeIds.add(edge.id);
      const classification = input.covariance_semantics[edge.id];
      if (!classification) diagnostics.push(diagnostic(
        "native_workbench.covariance_classification_required",
        "semantics",
        edge.id,
        `Covariance edge ${edge.id} has not been classified as scientific or presentation-only.`,
        "Choose Model covariance, Residual/error covariance, Disturbance covariance, or Presentation only.",
      ));
      else if (classification.kind !== "scientific" && classification.kind !== "presentation_only") diagnostics.push(diagnostic(
        "native_workbench.covariance_classification_unknown",
        "semantics",
        edge.id,
        `Covariance edge ${edge.id} has an unknown classification.`,
        "Choose one of the four supported relationship uses.",
      ));
      continue;
    }
    const pair = `${edge.source}\0${edge.target}`;
    if (structuralPairs.has(pair)) diagnostics.push(diagnostic("native_workbench.structural_path_duplicate", "semantics", edge.id, `Structural path ${edge.source} -> ${edge.target} is duplicated.`, "Keep one scientific path for this ordered construct pair."));
    structuralPairs.add(pair);
  }
  for (const configuredId of Object.keys(input.covariance_semantics)) {
    if (!covarianceEdgeIds.has(configuredId)) diagnostics.push(diagnostic(
      "native_workbench.covariance_classification_orphaned",
      "semantics",
      configuredId,
      `Covariance classification ${configuredId} does not match a current covariance edge.`,
      "Remove the stale classification or restore the matching covariance edge.",
    ));
  }

  const derivedTermIds = new Set<string>();
  for (const node of explicitInteractionNodesById.values()) {
    const interaction = node.data.interaction!;
    const termId = requiredText(interaction.termId);
    if (!termId || derivedTermIds.has(termId)) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_term_id_invalid",
        "schema",
        node.id,
        `Explicit interaction ${node.id} needs a unique non-empty scientific term id.`,
        "Recreate the interaction so it receives a stable unique term identity.",
      ));
    } else derivedTermIds.add(termId);

    const operands = Array.isArray(interaction.operands) ? interaction.operands : [];
    if (operands.length < 2
      || new Set(operands).size !== operands.length
      || operands.some((operand) => !requiredText(operand) || operand === node.id || !nodesById.has(operand) || explicitInteractionNodesById.has(operand))) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_operands_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} must reference at least two unique ordinary construct operands in authored order.`,
        "Choose existing non-derived constructs as the focal predictor and moderators.",
      ));
    }
    if (!requiredText(interaction.outcome)
      || interaction.outcome === node.id
      || !nodesById.has(interaction.outcome)
      || explicitInteractionNodesById.has(interaction.outcome)) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_outcome_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} must reference one existing ordinary outcome construct.`,
        "Reconnect the interaction to an existing non-derived outcome.",
      ));
    }
    if (!["two_stage", "product_indicator", "orthogonalizing"].includes(interaction.canonicalMethod)) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_method_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} has an unsupported construction method.`,
        "Choose two-stage, product-indicator, or orthogonalizing construction explicitly.",
      ));
    }
    if (!["strong", "weak", "none"].includes(interaction.hierarchyPolicy)) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_hierarchy_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} has an unsupported hierarchy policy.`,
        "Choose strong, weak, or no hierarchy explicitly.",
      ));
    }
    const productIndicator = (interaction as { productIndicator?: unknown }).productIndicator;
    const hasProductIndicator = productIndicator !== undefined && productIndicator !== null;
    if (interaction.canonicalMethod === "product_indicator") {
      if (!hasProductIndicator || !isExactProductIndicatorSpecificationV4(productIndicator)) {
        diagnostics.push(diagnostic(
          "native_workbench.interaction_v2_product_indicator_invalid",
          "semantics",
          node.id,
          `Product-indicator interaction ${node.id} needs one exact construction specification.`,
          "Choose centering, sample standardization, and all-pairs construction explicitly.",
        ));
      }
    } else if (hasProductIndicator) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_product_indicator_forbidden",
        "semantics",
        node.id,
        `Interaction ${node.id} carries product-indicator settings for a different construction method.`,
        "Remove the product-indicator settings or choose the product-indicator method explicitly.",
      ));
    }

    const focalEdge = edgesById.get(interaction.focalRelationId);
    if (!focalEdge
      || isMeasurementEdge(focalEdge)
      || edgeRole(focalEdge) === "control"
      || edgeRole(focalEdge) === "covariance"
      || focalEdge.source !== operands[0]
      || focalEdge.target !== interaction.outcome) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_focal_relation_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} does not reference its exact focal structural path.`,
        "Reconnect the focal predictor to the outcome and recreate or retarget the interaction.",
      ));
    }
    const effectEdges = [...edgesById.values()].filter((edge) =>
      !isMeasurementEdge(edge)
      && edgeRole(edge) !== "control"
      && edgeRole(edge) !== "covariance"
      && edge.source === node.id
      && edge.target === interaction.outcome);
    if (effectEdges.length !== 1) {
      diagnostics.push(diagnostic(
        "native_workbench.interaction_v2_effect_relation_invalid",
        "semantics",
        node.id,
        `Explicit interaction ${node.id} needs exactly one interaction-effect path to its outcome.`,
        "Restore the generated interaction path or remove duplicate effect paths.",
      ));
    }
  }

  const higherOrderApproaches = [
    "repeated_indicators",
    "extended_repeated_indicators",
    "embedded_two_stage",
    "disjoint_two_stage",
    "hybrid",
  ] as const;
  const higherOrderMeasurementTypes = [
    "reflective_reflective",
    "reflective_formative",
    "formative_reflective",
    "formative_formative",
  ] as const;
  for (const node of explicitHigherOrderNodesById.values()) {
    const higherOrder = node.data.higherOrder;
    const termId = requiredText(higherOrder.id);
    if (!termId || derivedTermIds.has(termId)) {
      diagnostics.push(diagnostic(
        "native_workbench.higher_order_term_id_invalid",
        "schema",
        node.id,
        `Higher-order construct ${node.id} needs a unique non-empty scientific term id.`,
        "Recreate the higher-order construct so it receives a stable unique term identity.",
      ));
    } else derivedTermIds.add(termId);

    const approach = higherOrder.canonicalApproach;
    if (!approach || !higherOrderApproaches.includes(approach)) {
      diagnostics.push(diagnostic(
        "native_workbench.higher_order_approach_invalid",
        "semantics",
        node.id,
        `Higher-order construct ${node.id} has no supported canonical construction approach.`,
        "Open Higher-Order Construct and choose one of the available construction approaches.",
      ));
    }
    const measurementType = higherOrder.measurementType;
    if (!measurementType || !higherOrderMeasurementTypes.includes(measurementType)) {
      diagnostics.push(diagnostic(
        "native_workbench.higher_order_measurement_type_invalid",
        "semantics",
        node.id,
        `Higher-order construct ${node.id} has no supported RR, RF, FR, or FF measurement type.`,
        "Open Higher-Order Construct and choose the conceptual direction again.",
      ));
    }

    if (approach && higherOrderApproaches.includes(approach)) {
      const expectedMethod = approach === "repeated_indicators" || approach === "extended_repeated_indicators"
        ? "repeated_indicators"
        : approach === "hybrid" ? "hybrid" : "two_stage";
      if (higherOrder.method !== expectedMethod) diagnostics.push(diagnostic(
        "native_workbench.higher_order_method_mismatch",
        "semantics",
        node.id,
        `Higher-order construct ${node.id} carries a legacy method that conflicts with its canonical approach.`,
        "Open Higher-Order Construct and save the selected approach again.",
      ));
    }

    const components = Array.isArray(higherOrder.components) ? higherOrder.components : [];
    const uniqueComponents = new Set(components);
    if (components.length < 2
      || uniqueComponents.size !== components.length
      || components.some((component) => !requiredText(component) || component === node.id)) {
      diagnostics.push(diagnostic(
        "native_workbench.higher_order_components_invalid",
        "semantics",
        node.id,
        `Higher-order construct ${node.id} needs at least two unique non-self components.`,
        "Choose at least two distinct lower-order constructs.",
      ));
    }
    for (const componentId of uniqueComponents) {
      const component = nodesById.get(componentId);
      if (!component
        || explicitInteractionNodesById.has(componentId)
        || explicitHigherOrderNodesById.has(componentId)
        || component.data.semantic !== undefined) {
        diagnostics.push(diagnostic(
          "native_workbench.higher_order_component_invalid",
          "semantics",
          node.id,
          `Higher-order component ${componentId || "(empty)"} is not an existing ordinary construct.`,
          "Choose existing non-derived lower-order constructs as components.",
        ));
        continue;
      }
      if (!Array.isArray(component.data.indicators) || component.data.indicators.length === 0) {
        diagnostics.push(diagnostic(
          "native_workbench.higher_order_component_unmeasured",
          "semantics",
          node.id,
          `Higher-order component ${componentId} has no measurement indicators.`,
          "Assign indicators to every lower-order component.",
        ));
      }
      if (measurementType && higherOrderMeasurementTypes.includes(measurementType)) {
        const requiredComponentMode = measurementType.startsWith("reflective_") ? "reflective" : "formative";
        if (component.data.mode !== requiredComponentMode) diagnostics.push(diagnostic(
          "native_workbench.higher_order_component_mode_mismatch",
          "semantics",
          node.id,
          `Higher-order component ${componentId} does not match the first part of measurement type ${measurementType}.`,
          `Change every lower-order component to ${requiredComponentMode} or choose the matching HCM type.`,
        ));
      }
    }
    if (measurementType && higherOrderMeasurementTypes.includes(measurementType)) {
      const requiredHigherOrderMode = measurementType.endsWith("_reflective") ? "reflective" : "formative";
      if (node.data.mode !== requiredHigherOrderMode) diagnostics.push(diagnostic(
        "native_workbench.higher_order_mode_mismatch",
        "semantics",
        node.id,
        `Higher-order construct ${node.id} does not match the second part of measurement type ${measurementType}.`,
        `Change the higher-order construct to ${requiredHigherOrderMode} or choose the matching HCM type.`,
      ));
    }
  }

  if (diagnostics.length) return sortDiagnostics(diagnostics);

  const constructVariables = sortedRecord([...nodesById.keys()].map((id) => [
    id,
    explicitInteractionNodesById.has(id) || explicitHigherOrderNodesById.has(id)
      ? nativeWorkbenchDerivedVariableIdV4(id)
      : nativeWorkbenchConstructVariableIdV4(id),
  ]));
  const indicatorVariables = sortedRecord([...indicatorOwner.keys()].map((indicator) => [indicator, nativeWorkbenchObservedVariableIdV4(indicator)]));
  const trace: NativeWorkbenchSemModelV4Trace = {
    construct_variables: constructVariables,
    indicator_variables: indicatorVariables,
    edge_objects: {},
  };

  const structuralEdges = [...edgesById.values()].filter((edge) => !isMeasurementEdge(edge) && edgeRole(edge) !== "covariance");
  const endogenous = new Set(structuralEdges.map((edge) => edge.target));
  const variables: SemVariableV4[] = [];
  const relations: SemRelationV4[] = [];
  const parameters: SemParameterV4[] = [];
  const annotations: SemAnnotationV4[] = [];
  const measurementIdentities = new Map<string, MeasurementIdentity>();

  const observedColumns = new Set([...indicatorOwner.keys(), ...Object.keys(input.observed_semantics ?? {})]);
  for (const sourceColumn of [...observedColumns].sort()) {
    const configured = input.observed_semantics?.[sourceColumn];
    variables.push({
      kind: "observed",
      id: nativeWorkbenchObservedVariableIdV4(sourceColumn),
      label: configured?.label?.trim() || sourceColumn,
      source_column: sourceColumn,
      scale: configured?.scale ?? "continuous",
      role: configured?.role ?? (indicatorOwner.has(sourceColumn) ? "indicator" : "control"),
      categories: [...(configured?.categories ?? [])].sort(),
      value_labels: sortedRecord(Object.entries(configured?.value_labels ?? {})),
      missing_markers: [...(configured?.missing_markers ?? [])].sort(),
      transformation_lineage: serializableClone(configured?.transformation_lineage ?? []),
    });
  }

  for (const node of [...nodesById.values()].sort((left, right) => left.id.localeCompare(right.id))) {
    const constructId = constructVariables[node.id];
    if (explicitInteractionNodesById.has(node.id) || explicitHigherOrderNodesById.has(node.id)) {
      variables.push({ kind: "derived", id: constructId, label: node.data.label });
      continue;
    }
    const estimand = input.construct_estimands[node.id] as Exclude<NativeWorkbenchConstructEstimandV4, { kind: "legacy_estimand_unspecified" }>;
    const varianceParameterId = stableAdapterId("factor_variance", [node.id]);
    if (estimand.kind === "composite") {
      variables.push({
        kind: "composite",
        id: constructId,
        label: node.data.label,
        weighting: { kind: node.data.mode === "reflective" ? "mode_a" : "mode_b" },
      });
    } else {
      const marker = estimand.marker_indicator ?? [...node.data.indicators].sort()[0];
      const identification: FactorIdentificationV4 = { kind: "marker_loading", indicator: indicatorVariables[marker] };
      variables.push({
        kind: "common_factor",
        id: constructId,
        label: node.data.label,
        identification,
        mean_policy: { kind: "fixed_zero" },
        disturbance_policy: endogenous.has(node.id)
          ? { kind: "endogenous_disturbance", parameter: varianceParameterId }
          : { kind: "exogenous_variance", parameter: varianceParameterId },
      });
      const varianceEndpoint: SemEndpointV4 = endogenous.has(node.id)
        ? { kind: "disturbance_of", id: constructId }
        : { kind: "variable", id: constructId };
      parameters.push(freeParameter(
        varianceParameterId,
        `Variance(${node.data.shortName})`,
        { kind: "variance", endpoint: varianceEndpoint },
        1,
        0,
      ));
    }

    for (const indicator of [...node.data.indicators].sort()) {
      const observedId = indicatorVariables[indicator];
      const relationId = stableAdapterId("measurement_relation", [node.id, indicator]);
      const parameterId = stableAdapterId("measurement_parameter", [node.id, indicator]);
      const measurementEdgeId = `measurement::${node.id}::${indicator}`;
      measurementIdentities.set(measurementEdgeId, { constructId: node.id, indicator, relationId, parameterId });
      if (estimand.kind === "composite" && node.data.mode === "formative") {
        relations.push({ kind: "measurement_causal", id: relationId, indicator: observedId, composite: constructId, parameter: parameterId });
        parameters.push(freeParameter(parameterId, `${indicator} -> ${node.data.shortName}`, { kind: "weight", indicator: observedId, composite: constructId }));
      } else {
        relations.push({ kind: "measurement_effect", id: relationId, construct: constructId, indicator: observedId, parameter: parameterId });
        const target: SemParameterTargetV4 = { kind: "loading", construct: constructId, indicator: observedId };
        const marker = estimand.kind === "common_factor" && (estimand.marker_indicator ?? [...node.data.indicators].sort()[0]) === indicator;
        parameters.push(marker
          ? { kind: "fixed", id: parameterId, label: `${node.data.shortName} -> ${indicator}`, target, value: 1, group_overrides: [] }
          : freeParameter(parameterId, `${node.data.shortName} -> ${indicator}`, target, estimand.kind === "common_factor" ? 0.7 : null));
      }
      if (estimand.kind === "common_factor") {
        const residualVarianceId = stableAdapterId("residual_variance", [indicator]);
        parameters.push(freeParameter(
          residualVarianceId,
          `Residual variance(${indicator})`,
          { kind: "variance", endpoint: { kind: "residual_of", id: observedId } },
          0.5,
          0,
        ));
      }
    }
  }

  const presentationEdges: Extract<SemPresentationV4, { kind: "canvas" }>["edges"] = [];
  for (const edge of [...edgesById.values()].sort((left, right) => left.id.localeCompare(right.id))) {
    if (isMeasurementEdge(edge)) {
      const measurement = measurementIdentities.get(edge.id);
      if (!measurement) {
        diagnostics.push(diagnostic("native_workbench.measurement_edge_unknown", "semantics", edge.id, `Measurement edge ${edge.id} does not match the current indicator assignments.`, "Refresh the diagram so measurement edges are regenerated from construct indicators."));
        continue;
      }
      const owner = nodesById.get(measurement.constructId)!;
      const indicatorNode = currentIndicatorNodeId(measurement.constructId, measurement.indicator);
      const expectedSource = owner.data.mode === "reflective" ? owner.id : indicatorNode;
      const expectedTarget = owner.data.mode === "reflective" ? indicatorNode : owner.id;
      if (edge.source !== expectedSource || edge.target !== expectedTarget) {
        diagnostics.push(diagnostic(
          "native_workbench.measurement_edge_endpoint_invalid",
          "semantics",
          edge.id,
          `Measurement edge ${edge.id} does not connect its declared construct and indicator in the expected direction.`,
          "Refresh the diagram so measurement edges are regenerated from the current indicator assignments.",
        ));
        continue;
      }
      trace.edge_objects[edge.id] = { kind: "derived_measurement_relation", sem_id: measurement.relationId, parameter_id: measurement.parameterId };
      presentationEdges.push({ relation: measurement.relationId, routing: edgeRouting(edge, input.diagram_layout) });
      continue;
    }
    if (edgeRole(edge) !== "covariance") {
      const relationId = stableAdapterId("structural_relation", [edge.id]);
      const parameterId = stableAdapterId("structural_parameter", [edge.id]);
      const source = constructVariables[edge.source];
      const target = constructVariables[edge.target];
      relations.push({ kind: "structural", id: relationId, source, target, parameter: parameterId, intercept_parameter: null });
      if ((edge.data as { technicalGenerated?: boolean } | undefined)?.technicalGenerated === true) {
        annotations.push({
          kind: "note",
          id: standardSemGeneralSemModerationV3GeneratedAnnotationIdV1(relationId),
          subject: relationId,
          text: "QuickPLS-generated strong-hierarchy dependency.",
        });
      }
      const controlLabel = edgeRole(edge) === "control" ? edgeDataText(edge, "controlLabel") : null;
      parameters.push(freeParameter(parameterId, controlLabel || edgeTextLabel(edge) || `${edge.source} -> ${edge.target}`, { kind: "regression", source, target }));
      trace.edge_objects[edge.id] = { kind: "scientific_relation", sem_id: relationId, parameter_id: parameterId };
      presentationEdges.push({ relation: relationId, routing: edgeRouting(edge, input.diagram_layout) });
      continue;
    }

    const classification = input.covariance_semantics[edge.id];
    if (classification.kind === "presentation_only") {
      const annotationId = stableAdapterId("presentation_covariance", [edge.id]);
      annotations.push({
        kind: "display_only_covariance",
        id: annotationId,
        left: constructVariables[edge.source],
        right: constructVariables[edge.target],
        label: edgeTextLabel(edge),
      });
      trace.edge_objects[edge.id] = { kind: "presentation_annotation", sem_id: annotationId };
      continue;
    }

    const suppliedLeft = classification.left ?? null;
    const suppliedRight = classification.right ?? null;
    if ((suppliedLeft === null) !== (suppliedRight === null)) {
      diagnostics.push(diagnostic(
        "native_workbench.covariance_endpoint_pair_incomplete",
        "semantics",
        edge.id,
        `Scientific covariance ${edge.id} supplies only one endpoint.`,
        "Supply both exact endpoints, or omit both to use the two drawn construct variables.",
      ));
      continue;
    }
    const left = suppliedLeft ?? { kind: "variable" as const, id: constructVariables[edge.source] };
    const right = suppliedRight ?? { kind: "variable" as const, id: constructVariables[edge.target] };
    const endpointProblem = validateCovarianceEndpoints(edge, left, right, variables, indicatorOwner);
    if (endpointProblem) {
      diagnostics.push(endpointProblem);
      continue;
    }
    const relationId = stableAdapterId("covariance_relation", [edge.id]);
    const parameterId = stableAdapterId("covariance_parameter", [edge.id]);
    relations.push({ kind: "covariance", id: relationId, left, right, parameter: parameterId });
    parameters.push(freeParameter(parameterId, edgeTextLabel(edge) || `Covariance(${edge.source}, ${edge.target})`, { kind: "covariance", left, right }));
    trace.edge_objects[edge.id] = { kind: "scientific_relation", sem_id: relationId, parameter_id: parameterId };
    presentationEdges.push({ relation: relationId, routing: edgeRouting(edge, input.diagram_layout) });
  }

  if (diagnostics.length) return sortDiagnostics(diagnostics);

  const derivedTerms: SemDerivedTermV4[] = [
    ...[...explicitInteractionNodesById.values()].map((node) => {
      const interaction = node.data.interaction!;
      const term: Extract<SemDerivedTermV4, { kind: "interaction_v2" }> = {
        kind: "interaction_v2",
        id: interaction.termId,
        output: constructVariables[node.id],
        operands: interaction.operands.map((operand) => constructVariables[operand]),
        focal_relation: stableAdapterId("structural_relation", [interaction.focalRelationId]),
        method: interaction.canonicalMethod,
        hierarchy_policy: interaction.hierarchyPolicy,
      };
      if (interaction.productIndicator !== undefined && interaction.productIndicator !== null) {
        term.product_indicator = serializableClone(interaction.productIndicator);
      }
      return term;
    }),
    ...[...explicitHigherOrderNodesById.values()].map((node) => ({
      kind: "higher_order" as const,
      id: node.data.higherOrder.id,
      output: constructVariables[node.id],
      components: node.data.higherOrder.components.map((component) => constructVariables[component]),
      approach: node.data.higherOrder.canonicalApproach!,
      measurement_type: node.data.higherOrder.measurementType!,
    })),
  ].sort((left, right) => left.id.localeCompare(right.id));

  const authoredParameters = applyNativeSemModelParameterAuthoringV4({
    nodes: [...nodesById.values()],
    edges: [...edgesById.values()],
    variables,
    relations,
    parameters,
    constraints: [],
  });
  if (!authoredParameters.ok) return sortDiagnostics(authoredParameters.diagnostics.map((item) => diagnostic(
    item.code,
    "semantics",
    item.subject,
    item.message,
    item.corrective_action,
  )));

  const viewport = input.diagram_layout?.diagramViewport;
  const model = canonicalizeSemModelV4({
    schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
    id: modelId!,
    name: modelName!,
    variables: authoredParameters.variables,
    relations,
    parameters: authoredParameters.parameters,
    constraints: authoredParameters.constraints,
    derived_terms: derivedTerms,
    group: serializableClone(input.group ?? { kind: "single_group" }),
    data_binding: serializableClone(input.data_binding),
    annotations,
    presentation: {
      kind: "canvas",
      nodes: [...nodesById.values()].map((node) => ({
        variable: constructVariables[node.id],
        x: node.position.x,
        y: node.position.y,
        style: {},
      })),
      edges: presentationEdges,
      shapes: [],
      images: [],
      lines: [],
      zoom: viewport?.zoom ?? null,
      pan_x: viewport?.x ?? null,
      pan_y: viewport?.y ?? null,
    },
  });
  const modelIssues = validateSemModelV4(model);
  if (modelIssues.length) return sortDiagnostics(modelIssues.map((issue) => diagnostic(
    `native_workbench.sem_model.${issue.code}`,
    issue.code.startsWith("data.") || issue.code.startsWith("group.") ? "data_binding" : "model",
    issue.subject,
    issue.message,
    correctiveActionForModelIssue(issue.code),
  )));

  trace.edge_objects = sortedRecord(Object.entries(trace.edge_objects));
  return { model, trace };
}

function validateCovarianceEndpoints(
  edge: Edge,
  left: SemEndpointV4,
  right: SemEndpointV4,
  variables: readonly SemVariableV4[],
  indicatorOwner: ReadonlyMap<string, string>,
): NativeWorkbenchSemModelV4Diagnostic | null {
  const variablesById = new Map(variables.map((variable) => [variable.id, variable]));
  for (const endpoint of [left, right]) {
    if (!endpoint || !["variable", "residual_of", "disturbance_of"].includes(endpoint.kind) || !requiredText(endpoint.id)) {
      return diagnostic("native_workbench.covariance_endpoint_invalid", "semantics", edge.id, `Scientific covariance ${edge.id} has a malformed endpoint.`, "Choose a variable, observed residual, or latent disturbance endpoint with a stable id.");
    }
    const variable = variablesById.get(endpoint.id);
    if (!variable) return diagnostic(
      "native_workbench.covariance_endpoint_unknown",
      "semantics",
      edge.id,
      `Scientific covariance ${edge.id} references unknown endpoint ${endpoint.id}.`,
      "Select an existing construct or indicator endpoint from this model.",
    );
    if (endpoint.kind === "residual_of" && variable.kind !== "observed") return diagnostic(
      "native_workbench.residual_endpoint_requires_indicator",
      "semantics",
      edge.id,
      `Residual endpoint ${endpoint.id} is not an observed indicator.`,
      "Choose an observed indicator for a residual covariance.",
    );
    if (endpoint.kind === "disturbance_of" && variable.kind === "observed") return diagnostic(
      "native_workbench.disturbance_endpoint_requires_construct",
      "semantics",
      edge.id,
      `Disturbance endpoint ${endpoint.id} is observed rather than latent or composite.`,
      "Choose a construct for a disturbance covariance.",
    );
  }
  if (left.kind === right.kind && left.id === right.id) return diagnostic(
    "native_workbench.covariance_self_relation",
    "semantics",
    edge.id,
    `Scientific covariance ${edge.id} uses the same endpoint twice.`,
    "Use a variance parameter or select two distinct endpoints.",
  );

  const owner = (endpoint: SemEndpointV4) => {
    if (endpoint.id.startsWith("construct:")) return endpoint.id.slice("construct:".length);
    if (!endpoint.id.startsWith("observed:")) return null;
    return indicatorOwner.get(endpoint.id.slice("observed:".length)) ?? null;
  };
  const endpointOwners = [owner(left), owner(right)].sort();
  const drawnOwners = [edge.source, edge.target].sort();
  if (endpointOwners.some((value) => value === null) || endpointOwners[0] !== drawnOwners[0] || endpointOwners[1] !== drawnOwners[1]) return diagnostic(
    "native_workbench.covariance_endpoint_drawing_mismatch",
    "semantics",
    edge.id,
    `Scientific endpoints on ${edge.id} do not belong to the two constructs connected on the canvas.`,
    "Reconnect the edge or choose residual/disturbance endpoints owned by the two drawn constructs.",
  );
  return null;
}

function freeParameter(
  id: string,
  label: string,
  target: SemParameterTargetV4,
  start: number | null = null,
  lower: number | null = null,
): Extract<SemParameterV4, { kind: "free" }> {
  return { kind: "free", id, label, target, start, lower, upper: null, equality_label: null, group_overrides: [] };
}

function correctiveActionForModelIssue(code: string) {
  if (code.startsWith("data.") || code.startsWith("group.")) return "Correct the exact data, weight, matrix, or group binding named by this issue before conversion.";
  if (code.startsWith("endpoint.") || code.startsWith("covariance.")) return "Choose two valid, distinct endpoints and keep only one scientific covariance for that pair.";
  if (code.startsWith("observed.")) return "Correct the observed-variable scale, role, categories, or source-column metadata named by this issue.";
  if (code.startsWith("identification.") || code.startsWith("factor.")) return "Correct the factor/composite identification setting named by this issue.";
  return "Correct the referenced model object before enabling this SemModelV4 copy.";
}

function specialAssumptionDiagnostics(value: NativeWorkbenchSpecialAssumptionsV4 | undefined): NativeWorkbenchSemModelV4Diagnostic[] {
  if (value === undefined) return [];
  const keys: Array<keyof NativeWorkbenchSpecialAssumptionsV4> = [
    "imply_exogenous_latent_correlations",
    "imply_causal_indicator_correlations",
    "fix_causal_indicator_variances_to_one",
  ];
  const record = value as unknown as Record<string, unknown>;
  if (!record || typeof record !== "object" || Array.isArray(record)
    || Object.keys(record).length !== keys.length
    || keys.some((key) => typeof record[key] !== "boolean")) return [diagnostic(
    "native_workbench.special_assumptions_invalid",
    "schema",
    "special_assumptions",
    "Special Assumptions settings cannot be read safely.",
    "Restore all three Special Assumptions switches to explicit on or off values.",
  )];
  const labels: Readonly<Record<keyof NativeWorkbenchSpecialAssumptionsV4, string>> = {
    imply_exogenous_latent_correlations: "Imply all exogenous latent correlations",
    imply_causal_indicator_correlations: "Imply causal-indicator correlations",
    fix_causal_indicator_variances_to_one: "Fix causal-indicator variances to 1.0",
  };
  return keys.flatMap((key) => value[key] ? [diagnostic(
    `native_workbench.special_assumption.${key}_not_available`,
    "semantics",
    key,
    `${labels[key]} is not available in the current SemModelV4 compiler.`,
    "Leave this switch off; use explicit scientific parameters where supported, or wait for compiler and result-provenance support.",
  )] : []);
}

function edgeRole(edge: Edge): unknown {
  const data = edge.data;
  return data && typeof data === "object" && !Array.isArray(data) ? (data as Record<string, unknown>).role : undefined;
}

function edgeDataText(edge: Edge, key: string): string | null {
  const data = edge.data;
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;
  const value = (data as Record<string, unknown>)[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function edgeTextLabel(edge: Edge): string | null {
  if (typeof edge.label === "string" && edge.label.trim()) return edge.label.trim();
  if (typeof edge.label === "number" && Number.isFinite(edge.label)) return String(edge.label);
  return null;
}

function edgeRouting(edge: Edge, layout: Partial<DiagramLayoutState> | null | undefined): string | null {
  const configured = layout?.edgeLayouts?.[edge.id]?.routing;
  if (configured) return configured;
  const explicit = edgeDataText(edge, "routing");
  if (explicit) return explicit;
  const type = edge.type?.toLowerCase();
  if (type === "straight") return "straight";
  if (type === "bezier" || type === "simplebezier") return "curved";
  if (type === "step" || type === "smoothstep") return "orthogonal";
  return null;
}

function isMeasurementEdge(edge: Edge) {
  return edge.id.startsWith("measurement::");
}

function currentIndicatorNodeId(constructId: string, indicator: string) {
  return `indicator::${constructId}::${encodeURIComponent(indicator)}`;
}

function diagnostic(
  code: string,
  stage: NativeWorkbenchSemModelV4DiagnosticStage,
  subject: string | null,
  message: string,
  corrective_action: string,
): NativeWorkbenchSemModelV4Diagnostic {
  return { code, stage, subject, message, corrective_action };
}

function sortDiagnostics(diagnostics: NativeWorkbenchSemModelV4Diagnostic[]) {
  return [...diagnostics].sort((left, right) => `${left.code}\0${left.subject ?? ""}`.localeCompare(`${right.code}\0${right.subject ?? ""}`));
}

function requiredText(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function isExactProductIndicatorSpecificationV4(
  value: unknown,
): value is NonNullable<InteractionV2Data["productIndicator"]> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  return keys.length === 3
    && keys[0] === "centering"
    && keys[1] === "pairing"
    && keys[2] === "standardization"
    && ["none", "mean_center", "double_mean_center"].includes(record.centering as string)
    && ["none", "sample_standard_deviation"].includes(record.standardization as string)
    && record.pairing === "all_pairs";
}

function stableAdapterId(prefix: string, parts: readonly string[]) {
  const encoder = new TextEncoder();
  const encoded = parts.map((part) => [...encoder.encode(part)].map((byte) => byte.toString(16).padStart(2, "0")).join("")).join("_");
  return `${prefix}_${encoded}`;
}

function sortedRecord<T>(entries: readonly (readonly [string, T])[]): Record<string, T> {
  return Object.fromEntries([...entries].sort(([left], [right]) => left.localeCompare(right)));
}

function serializableClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function deepFreeze<T>(value: T): T {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value as Record<string, unknown>).forEach(deepFreeze);
  return value;
}
