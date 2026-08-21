import type { Edge, Node } from "@xyflow/react";
import { defaultDiagramLayout, layoutSmartplsModel } from "./diagramGraph";
import { layoutModel } from "./modelLayout";
import {
  compareUtf8StringsV1,
  type SemDerivedTermV4,
  type SemRelationV4,
  type SemVariableV4,
} from "./semModelV4";
import {
  GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
  standardSemGeneralSemModerationV3IdentityV1,
  type ModeratingEffectTargetV1,
  type StandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4EditorIntentV1,
} from "./standardSemModelV4Authority";
import type {
  ConstructData,
  Dataset,
  DiagramLayoutState,
  ModelEditAffectedIdentitiesV1,
  ModelEditCommandV1,
  ModelEditTransactionClassV1,
} from "../types";

type SemMeasurementRelationV4 = Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }>;
type SemStructuralRelationV4 = Extract<SemRelationV4, { kind: "structural" }>;
type SemInteractionV2TermV4 = Extract<SemDerivedTermV4, { kind: "interaction_v2" }>;

export type StrictModelEditIntentPlanV1 =
  | {
      status: "ready";
      intent: StandardSemModelV4EditorIntentV1;
      affected: ModelEditAffectedIdentitiesV1;
    }
  | {
      status: "blocked";
      code: string;
      message: string;
      correctiveAction: string;
    };

type ScientificModelEditCommandV1 = Extract<
  ModelEditCommandV1,
  {
    kind:
      | "add_construct"
      | "rename_construct"
      | "invert_measurement_model"
      | "assign_indicators"
      | "unassign_indicator"
      | "add_path"
      | "reverse_path"
      | "remove_path"
      | "create_higher_order"
      | "edit_higher_order"
      | "remove_higher_order"
      | "create_moderating_effect"
      | "edit_moderating_effect"
      | "remove_moderating_effect";
  }
>;

export const modelEditTransactionClassV1 = (
  command: ModelEditCommandV1,
): ModelEditTransactionClassV1 => {
  switch (command.kind) {
    case "add_construct":
    case "rename_construct":
    case "invert_measurement_model":
    case "assign_indicators":
    case "unassign_indicator":
    case "add_path":
    case "reverse_path":
    case "remove_path":
    case "create_higher_order":
    case "edit_higher_order":
    case "remove_higher_order":
    case "create_moderating_effect":
    case "edit_moderating_effect":
    case "remove_moderating_effect":
      return "scientific";
    default:
      return "presentation";
  }
};

export const emptyModelEditAffectedIdentitiesV1 = (): ModelEditAffectedIdentitiesV1 => ({
  constructIds: [],
  indicatorIds: [],
  relationshipIds: [],
});

export function observedVariableForModelEditColumnV1(
  authority: StandardSemModelV4AuthorityRecordV1,
  dataset: Dataset,
  column: string,
): Extract<SemVariableV4, { kind: "observed" }> {
  const existing = authority.model.variables.find((variable): variable is Extract<SemVariableV4, { kind: "observed" }> =>
    variable.kind === "observed" && variable.source_column === column);
  if (existing) return structuredClone(existing);
  const metadata = dataset.columnMetadata?.find((candidate) => candidate.name === column);
  return {
    kind: "observed",
    id: `observed:${column}`,
    label: metadata?.label?.trim() || column,
    source_column: column,
    scale: metadata?.scale_type ?? "continuous",
    role: "indicator",
    categories: Object.keys(metadata?.value_labels ?? {}).sort(compareUtf8StringsV1),
    value_labels: { ...(metadata?.value_labels ?? {}) },
    missing_markers: [...new Set((metadata?.missing_markers ?? [])
      .map((value) => value.trim())
      .filter(Boolean))].sort(compareUtf8StringsV1),
    transformation_lineage: [],
  };
}

export function strictModelEditIntentPlanV1(
  command: ModelEditCommandV1,
  authority: StandardSemModelV4AuthorityRecordV1,
  dataset: Dataset,
  reservedGroupColumn?: string | null,
): StrictModelEditIntentPlanV1 {
  if (modelEditTransactionClassV1(command) !== "scientific") {
    return blocked(
      "model_edit.transaction_mismatch",
      "A presentation command cannot be converted into a scientific authority intent.",
      "Apply the command to the versioned diagram layout instead.",
    );
  }
  const scientificCommand = command as ScientificModelEditCommandV1;

  if (scientificCommand.kind === "add_construct") {
    const constructId = exactModelEditId(scientificCommand.constructId);
    const label = scientificCommand.label.trim();
    if (!constructId) return blocked("model_edit.construct_id_invalid", "The new construct needs one exact stable identifier.", "Generate a new stable construct ID and retry.");
    if (!label) return blocked("model_edit.label_required", "A construct name cannot be empty.", "Enter a nonempty construct name.");
    if (scientificCommand.position && (!Number.isFinite(scientificCommand.position.x) || !Number.isFinite(scientificCommand.position.y))) {
      return blocked("model_edit.construct_position_invalid", "The requested Canvas position is not finite.", "Choose a valid point on the Canvas and retry.");
    }
    if (authority.model.variables.some((variable) => variable.id === constructId)) {
      return blocked("model_edit.construct_id_in_use", `Construct identity '${constructId}' is already in use.`, "Generate a new stable construct ID and retry.");
    }
    const columns = validColumns(scientificCommand.columns ?? [], dataset, reservedGroupColumn);
    if ((scientificCommand.columns?.length ?? 0) !== columns.length) {
      return blocked("model_edit.indicators_unavailable", "One or more requested indicators are unavailable in the active dataset.", "Select only unique, non-grouping columns from the active dataset.");
    }
    const indicators = columns.map((column) => observedVariableForModelEditColumnV1(authority, dataset, column));
    return {
      status: "ready",
      intent: {
        kind: "add_construct",
        variable_id: constructId,
        label,
        representation: { kind: "composite", weighting: { kind: "mode_a" } },
        indicators,
      },
      affected: { constructIds: [constructId], indicatorIds: indicators.map((indicator) => indicator.id), relationshipIds: [] },
    };
  }

  if (scientificCommand.kind === "add_path") {
    const relationshipId = exactModelEditId(scientificCommand.relationId);
    const sourceId = exactModelEditId(scientificCommand.sourceId);
    const targetId = exactModelEditId(scientificCommand.targetId);
    if (!relationshipId || !sourceId || !targetId) {
      return blocked("model_edit.path_identity_invalid", "The path needs exact stable relationship and endpoint identifiers.", "Refresh the model and draw the path again.");
    }
    if (sourceId === targetId) return blocked("model_edit.path_self_loop", "A structural path cannot connect a construct to itself.", "Choose two different ordinary constructs.");
    const endpoints = authority.model.variables.filter((variable) => variable.id === sourceId || variable.id === targetId);
    if (endpoints.length !== 2 || endpoints.some((variable) => variable.kind !== "common_factor" && variable.kind !== "composite")) {
      return blocked("model_edit.path_endpoint_unavailable", "Both path endpoints must be ordinary constructs in the active authority.", "Select two ordinary factor or composite constructs.");
    }
    if (authority.model.relations.some((relation) => relation.id === relationshipId)) {
      return blocked("model_edit.relationship_id_in_use", `Relationship '${relationshipId}' already exists.`, "Generate one new stable relationship ID and retry.");
    }
    if (authority.model.relations.some((relation) => relation.kind === "structural" && relation.source === sourceId && relation.target === targetId)) {
      return blocked("model_edit.path_duplicate", "That structural path already exists.", "Select the existing path or choose different endpoints.");
    }
    return {
      status: "ready",
      intent: {
        kind: "add_relationship",
        relationship_id: relationshipId,
        definition: { kind: "structural", source: sourceId, target: targetId, label: scientificCommand.label?.trim() || "Path" },
      },
      affected: { constructIds: [sourceId, targetId], indicatorIds: [], relationshipIds: [relationshipId] },
    };
  }

  if (scientificCommand.kind === "reverse_path" || scientificCommand.kind === "remove_path") {
    const relationshipId = exactModelEditId(scientificCommand.relationId);
    const relation = relationshipId
      ? authority.model.relations.find((candidate) => candidate.id === relationshipId)
      : undefined;
    if (!relationshipId || relation?.kind !== "structural" || relation.role) {
      return blocked("model_edit.path_unavailable", `Relationship '${scientificCommand.relationId}' is not an editable ordinary structural path.`, "Select an ordinary structural path and retry.");
    }
    if (scientificCommand.kind === "remove_path") {
      return {
        status: "ready",
        intent: { kind: "delete_relationship", relationship_id: relationshipId },
        affected: { constructIds: [relation.source, relation.target], indicatorIds: [], relationshipIds: [relationshipId] },
      };
    }
    if (authority.model.relations.some((candidate) => candidate.id !== relationshipId
      && candidate.kind === "structural"
      && candidate.source === relation.target
      && candidate.target === relation.source)) {
      return blocked("model_edit.path_reverse_duplicate", "The reverse structural path already exists.", "Remove the reverse path or keep the current direction.");
    }
    const parameter = authority.model.parameters.find((candidate) => candidate.id === relation.parameter);
    return {
      status: "ready",
      intent: {
        kind: "replace_relationship",
        relationship_id: relationshipId,
        definition: { kind: "structural", source: relation.target, target: relation.source, label: parameter?.label?.trim() || "Path" },
      },
      affected: { constructIds: [relation.source, relation.target], indicatorIds: [], relationshipIds: [relationshipId] },
    };
  }

  if (scientificCommand.kind === "create_higher_order"
    || scientificCommand.kind === "edit_higher_order"
    || scientificCommand.kind === "remove_higher_order") {
    return strictHigherOrderIntentPlanV1(scientificCommand, authority);
  }

  if (scientificCommand.kind === "create_moderating_effect"
    || scientificCommand.kind === "edit_moderating_effect"
    || scientificCommand.kind === "remove_moderating_effect") {
    return strictModeratingEffectIntentPlanV1(scientificCommand, authority);
  }

  const constructId = scientificCommand.constructId;
  if (!constructId || constructId !== constructId.trim()) {
    return blocked(
      "model_edit.construct_id_invalid",
      "The selected construct has no exact stable identifier.",
      "Refresh the model and select the construct again.",
    );
  }
  const construct = authority.model.variables.find((variable) => variable.id === constructId);
  if (!construct || (construct.kind !== "common_factor" && construct.kind !== "composite")) {
    return blocked(
      "model_edit.construct_unavailable",
      `Construct '${constructId}' is not an editable factor or composite in the active authority.`,
      "Select an ordinary factor or composite from the active model.",
    );
  }

  if (scientificCommand.kind === "rename_construct") {
    const label = scientificCommand.label.trim();
    if (!label) return blocked("model_edit.label_required", "A construct name cannot be empty.", "Enter a nonempty construct name.");
    if (construct.label === label) return blocked("model_edit.no_change", "The construct already uses that name.", "Enter a different name or cancel the edit.");
    return {
      status: "ready",
      intent: { kind: "rename_construct", variable_id: constructId, label },
      affected: { constructIds: [constructId], indicatorIds: [], relationshipIds: [] },
    };
  }

  if (scientificCommand.kind === "invert_measurement_model") {
    const representation = construct.kind === "common_factor"
      ? { kind: "composite" as const, weighting: { kind: "mode_b" as const } }
      : construct.weighting.kind === "mode_b"
        ? { kind: "composite" as const, weighting: { kind: "mode_a" as const } }
        : { kind: "composite" as const, weighting: { kind: "mode_b" as const } };
    const relations = authority.model.relations.filter((relation): relation is SemMeasurementRelationV4 =>
      relation.kind === "measurement_effect"
        ? relation.construct === constructId
        : relation.kind === "measurement_causal" && relation.composite === constructId);
    return {
      status: "ready",
      intent: { kind: "set_construct_representation", variable_id: constructId, representation },
      affected: {
        constructIds: [constructId],
        indicatorIds: relations.map((relation) => relation.indicator),
        relationshipIds: relations.map((relation) => relation.id),
      },
    };
  }

  if (scientificCommand.kind === "assign_indicators") {
    const columns = validColumns(scientificCommand.columns, dataset, reservedGroupColumn);
    if (!columns.length) {
      return blocked(
        "model_edit.indicators_unavailable",
        "None of the requested indicators is an eligible column in the active dataset.",
        "Select one or more non-grouping columns from the active dataset.",
      );
    }
    const observed = columns.map((column) => observedVariableForModelEditColumnV1(authority, dataset, column));
    const owners = new Map(authority.model.relations.flatMap((relation) => {
      if (relation.kind === "measurement_effect") return [[relation.indicator, relation.construct] as const];
      if (relation.kind === "measurement_causal") return [[relation.indicator, relation.composite] as const];
      return [];
    }));
    if (observed.every((variable) => owners.get(variable.id) === constructId)) {
      return blocked("model_edit.no_change", "The selected indicators are already assigned to this construct.", "Choose different indicators or cancel the edit.");
    }
    return {
      status: "ready",
      intent: { kind: "assign_indicators", construct_id: constructId, indicators: observed },
      affected: {
        constructIds: [...new Set([constructId, ...observed.map((variable) => owners.get(variable.id)).filter((id): id is string => Boolean(id))])],
        indicatorIds: observed.map((variable) => variable.id),
        relationshipIds: [],
      },
    };
  }

  const matchingRelation = authority.model.relations.find((candidate): candidate is SemMeasurementRelationV4 => {
    if (candidate.kind !== "measurement_effect" && candidate.kind !== "measurement_causal") return false;
    const owner = candidate.kind === "measurement_effect" ? candidate.construct : candidate.composite;
    if (owner !== constructId) return false;
    const variable = authority.model.variables.find((item) => item.kind === "observed" && item.id === candidate.indicator);
    return variable?.kind === "observed" && variable.source_column === scientificCommand.column;
  });
  if (!matchingRelation) {
    return blocked(
      "model_edit.indicator_not_assigned",
      `Column '${scientificCommand.column}' is not assigned to construct '${constructId}'.`,
      "Refresh the model and choose an indicator currently assigned to the construct.",
    );
  }
  const replacementMarker = scientificCommand.replacementMarkerColumn
    ? authority.model.variables.find((variable): variable is Extract<SemVariableV4, { kind: "observed" }> =>
      variable.kind === "observed" && variable.source_column === scientificCommand.replacementMarkerColumn)?.id ?? null
    : null;
  if (scientificCommand.replacementMarkerColumn && !replacementMarker) {
    return blocked(
      "model_edit.replacement_marker_unavailable",
      `Replacement marker '${scientificCommand.replacementMarkerColumn}' is not an observed variable in the active authority.`,
      "Choose one remaining indicator from the same construct as the replacement marker.",
    );
  }
  return {
    status: "ready",
    intent: {
      kind: "remove_indicator",
      construct_id: constructId,
      observed_id: matchingRelation.indicator,
      replacement_marker: replacementMarker,
    },
    affected: {
      constructIds: [constructId],
      indicatorIds: [matchingRelation.indicator],
      relationshipIds: [matchingRelation.id],
    },
  };
}

type HigherOrderModelEditCommandV1 = Extract<
  ScientificModelEditCommandV1,
  { kind: "create_higher_order" | "edit_higher_order" | "remove_higher_order" }
>;

function strictHigherOrderIntentPlanV1(
  command: HigherOrderModelEditCommandV1,
  authority: StandardSemModelV4AuthorityRecordV1,
): StrictModelEditIntentPlanV1 {
  const termId = exactModelEditId(command.termId);
  const outputId = exactModelEditId(command.outputId);
  if (!termId || !outputId) {
    return blocked("model_edit.higher_order_identity_invalid", "The higher-order construct needs exact stable term and output identifiers.", "Refresh the model and retry the HOC operation.");
  }
  const resident = authority.model.derived_terms.find((term) => term.kind === "higher_order" && term.id === termId);
  if (command.kind === "remove_higher_order") {
    if (resident?.kind !== "higher_order" || resident.output !== outputId) {
      return blocked("model_edit.higher_order_unavailable", "The requested higher-order identity does not match the active authority.", "Refresh the model and select the higher-order construct again.");
    }
    const relationshipIds = authority.model.relations
      .filter((relation) => relation.kind === "structural" && (relation.source === outputId || relation.target === outputId))
      .map((relation) => relation.id);
    return {
      status: "ready",
      intent: { kind: "delete_construct", variable_id: outputId },
      affected: { constructIds: [outputId, ...resident.components], indicatorIds: [], relationshipIds },
    };
  }

  const draft = command.draft;
  const label = draft.name.trim();
  const components = [...new Set(draft.components.map((id) => id.trim()).filter(Boolean))];
  if (!label) return blocked("model_edit.higher_order_name_required", "A higher-order construct name cannot be empty.", "Enter a nonempty HOC name.");
  if (components.length < 2 || components.length !== draft.components.length) {
    return blocked("model_edit.higher_order_components_invalid", "A higher-order construct needs at least two distinct components.", "Select at least two different eligible component constructs.");
  }
  const componentVariables = components.map((id) => authority.model.variables.find((variable) => variable.id === id));
  if (componentVariables.some((variable) => !variable || variable.kind === "observed" || variable.kind === "derived")) {
    return blocked("model_edit.higher_order_component_unavailable", "Every HOC component must be an ordinary factor or composite in the active authority.", "Refresh the model and select eligible ordinary components.");
  }

  if (command.kind === "edit_higher_order") {
    if (resident?.kind !== "higher_order" || resident.output !== outputId) {
      return blocked("model_edit.higher_order_unavailable", "The requested higher-order identity does not match the active authority.", "Refresh the model and select the higher-order construct again.");
    }
    return {
      status: "ready",
      intent: {
        kind: "replace_higher_order",
        term_id: termId,
        output_id: outputId,
        label,
        components,
        approach: draft.approach,
        measurement_type: draft.measurementType,
      },
      affected: { constructIds: [...new Set([outputId, ...resident.components, ...components])], indicatorIds: [], relationshipIds: [] },
    };
  }

  if (resident || authority.model.variables.some((variable) => variable.id === outputId)) {
    return blocked("model_edit.higher_order_identity_in_use", "The requested HOC term or output identity is already in use.", "Generate new stable HOC identities and retry.");
  }
  const initialPath = command.kind === "create_higher_order" ? command.draft.initialPath : undefined;
  const initialRelationshipId = initialPath ? exactModelEditId(initialPath.relationshipId) : null;
  if (initialPath && !initialRelationshipId) {
    return blocked("model_edit.higher_order_path_identity_invalid", "The initial HOC path needs one exact stable relationship identifier.", "Generate a stable relationship ID and retry.");
  }
  return {
    status: "ready",
    intent: {
      kind: "add_higher_order",
      term_id: termId,
      output_id: outputId,
      label,
      components,
      approach: draft.approach,
      measurement_type: draft.measurementType,
      ...(initialPath && initialRelationshipId ? {
        initial_path: {
          relation_id: initialRelationshipId,
          source: initialPath.direction === "hoc_to_construct" ? outputId : initialPath.constructId,
          target: initialPath.direction === "hoc_to_construct" ? initialPath.constructId : outputId,
          label: initialPath.label?.trim() || (initialPath.direction === "hoc_to_construct" ? `${label} effect` : `${label} antecedent`),
        },
      } : {}),
    },
    affected: {
      constructIds: [outputId, ...components, ...(initialPath ? [initialPath.constructId] : [])],
      indicatorIds: [],
      relationshipIds: initialRelationshipId ? [initialRelationshipId] : [],
    },
  };
}

type ModeratingEffectModelEditCommandV1 = Extract<
  ScientificModelEditCommandV1,
  { kind: "create_moderating_effect" | "edit_moderating_effect" | "remove_moderating_effect" }
>;

export function modelEditModeratingEffectIdentityV1(
  target: ModeratingEffectTargetV1,
  operands: readonly [string, string] | readonly [string, string, string],
) {
  return standardSemGeneralSemModerationV3IdentityV1(target, operands);
}

function strictModeratingEffectIntentPlanV1(
  command: ModeratingEffectModelEditCommandV1,
  authority: StandardSemModelV4AuthorityRecordV1,
): StrictModelEditIntentPlanV1 {
  const requestedTermId = command.kind === "create_moderating_effect" ? null : exactModelEditId(command.termId);
  const requestedOutputId = command.kind === "create_moderating_effect" ? null : exactModelEditId(command.outputId);
  const resident = requestedTermId
    ? authority.model.derived_terms.find((term): term is SemInteractionV2TermV4 => term.kind === "interaction_v2" && term.id === requestedTermId)
    : undefined;
  const interactionFocalRelation = (term: SemInteractionV2TermV4 | undefined) => term
    ? authority.model.relations.find((relation): relation is SemStructuralRelationV4 => (
        relation.kind === "structural" && relation.id === term.focal_relation
      ))
    : undefined;
  const residentFocalRelation = interactionFocalRelation(resident);
  if (command.kind === "remove_moderating_effect") {
    if (!requestedTermId || !requestedOutputId || !resident || !residentFocalRelation || resident.output !== requestedOutputId) {
      return blocked("model_edit.moderating_effect_unavailable", "The requested moderating-effect identity does not match the active authority.", "Refresh the model and select the moderating effect again.");
    }
    return {
      status: "ready",
      intent: {
        kind: "remove_moderating_effect",
        intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
        sem_generation: "general_sem_v1",
        term_id: requestedTermId,
        output_id: requestedOutputId,
      },
      affected: { constructIds: [requestedOutputId, ...resident.operands, residentFocalRelation.target], indicatorIds: [], relationshipIds: [resident.focal_relation] },
    };
  }

  const effect = command.effect;
  const target: ModeratingEffectTargetV1 = effect.target.kind === "parent_interaction"
    ? { kind: "parent_interaction", interactionTermId: effect.target.interactionTermId }
    : { kind: "focal_relation", relationId: effect.target.relationId };
  const operands = effect.operands;
  if ((target.kind === "focal_relation" && operands.length !== 2)
    || (target.kind === "parent_interaction" && operands.length !== 3)) {
    return blocked("model_edit.moderating_effect_order_invalid", "The moderation target and operand order do not agree.", "Use two operands for a focal path or three operands for a parent interaction.");
  }
  if (new Set([...operands, effect.outcomeId]).size !== operands.length + 1) {
    return blocked("model_edit.moderating_effect_constructs_not_distinct", "Predictor, moderator operands, and outcome must be distinct constructs.", "Choose distinct ordinary constructs for the moderating effect.");
  }
  const structuralVariables = [...operands, effect.outcomeId].map((id) => authority.model.variables.find((variable) => variable.id === id));
  if (structuralVariables.some((variable) => !variable || variable.kind === "observed" || variable.kind === "derived")) {
    return blocked("model_edit.moderating_effect_construct_unavailable", "Every moderation operand and outcome must be an ordinary factor or composite.", "Refresh the model and select eligible ordinary constructs.");
  }
  const parent = target.kind === "parent_interaction"
    ? authority.model.derived_terms.find((term): term is SemInteractionV2TermV4 => term.kind === "interaction_v2" && term.id === target.interactionTermId)
    : undefined;
  const parentFocalRelation = interactionFocalRelation(parent);
  if (target.kind === "focal_relation") {
    const focal = authority.model.relations.find((relation) => relation.id === target.relationId);
    if (focal?.kind !== "structural" || focal.role || focal.source !== operands[0] || focal.target !== effect.outcomeId) {
      return blocked("model_edit.moderating_effect_focal_path_missing", "The selected focal path does not match the predictor and outcome.", "Select the exact predictor-to-outcome structural path and retry.");
    }
  } else {
    if (!parent || !parentFocalRelation || parent.operands.length !== 2
      || parent.operands[0] !== operands[0] || parent.operands[1] !== operands[1] || parentFocalRelation.target !== effect.outcomeId) {
      return blocked("model_edit.moderating_effect_parent_missing", "The selected parent two-way interaction does not match the requested three-way effect.", "Refresh the model and select one resident two-way moderating effect.");
    }
  }
  const label = effect.label.trim();
  if (!label) return blocked("model_edit.moderating_effect_label_required", "A moderating-effect label cannot be empty.", "Enter a nonempty label or regenerate it from the selected constructs.");
  const identity = command.kind === "create_moderating_effect"
    ? modelEditModeratingEffectIdentityV1(target, operands)
    : { termId: requestedTermId!, outputId: requestedOutputId! };
  if (command.kind === "edit_moderating_effect"
    && (!requestedTermId || !requestedOutputId || resident?.kind !== "interaction_v2" || resident.output !== requestedOutputId)) {
    return blocked("model_edit.moderating_effect_unavailable", "The requested moderating-effect identity does not match the active authority.", "Refresh the model and select the moderating effect again.");
  }
  const targetFocalRelation = target.kind === "focal_relation"
    ? target.relationId
    : parent?.focal_relation;
  const common = {
    intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
    sem_generation: "general_sem_v1" as const,
    label,
    operands,
    target,
    outcome: effect.outcomeId,
    method: "two_stage" as const,
    hierarchy_policy: "strong" as const,
  };
  return {
    status: "ready",
    intent: command.kind === "edit_moderating_effect"
      ? { kind: "replace_moderating_effect", term_id: identity.termId, output_id: identity.outputId, ...common }
      : { kind: "add_moderating_effect_v3", ...common },
    affected: {
      constructIds: [...new Set([
        identity.outputId,
        ...operands,
        effect.outcomeId,
        ...(resident ? [...resident.operands, ...(residentFocalRelation ? [residentFocalRelation.target] : [])] : []),
      ])],
      indicatorIds: [],
      relationshipIds: [...new Set([targetFocalRelation, resident?.focal_relation].filter((id): id is string => Boolean(id)))],
    },
  };
}

/**
 * Rebinds a graph to its prior versioned presentation without changing any
 * surviving object's stable layout metadata. Entries for removed scientific
 * objects are pruned; newly created objects receive deterministic defaults.
 */
export function reconcileModelEditDiagramLayoutV1(
  previous: DiagramLayoutState,
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
): DiagramLayoutState {
  const next = defaultDiagramLayout(nodes, edges, previous);
  for (const node of nodes) {
    const priorConstruct = previous.constructLayouts[node.id];
    next.constructLayouts[node.id] = {
      ...(priorConstruct ?? next.constructLayouts[node.id]),
      x: node.position.x,
      y: node.position.y,
    };
    const previousIndicators = previous.indicatorLayouts[node.id] ?? {};
    const nextIndicators = next.indicatorLayouts[node.id] ?? {};
    for (const column of node.data.indicators) {
      if (previousIndicators[column]) nextIndicators[column] = { ...previousIndicators[column] };
    }
    next.indicatorLayouts[node.id] = nextIndicators;
  }
  for (const edge of edges) {
    if (previous.edgeLayouts[edge.id]) next.edgeLayouts[edge.id] = cloneEdgeLayout(previous.edgeLayouts[edge.id]);
  }
  return next;
}

export function arrangeModelPreservingLayoutV1(
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
  layout: DiagramLayoutState,
  direction: "horizontal" | "vertical" | "smartpls",
) {
  const proposed = direction === "smartpls"
    ? layoutSmartplsModel(nodes, edges)
    : layoutModel(nodes, edges, direction);
  const nextNodes = proposed.map((node) => {
    const current = nodes.find((candidate) => candidate.id === node.id) ?? node;
    return layout.constructLayouts[node.id]?.pinned
      ? { ...node, position: { ...current.position } }
      : node;
  });
  return {
    nodes: nextNodes,
    diagramLayout: reconcileModelEditDiagramLayoutV1(layout, nextNodes, edges),
    movedConstructIds: nextNodes
      .filter((node) => {
        const before = nodes.find((candidate) => candidate.id === node.id);
        return before && (before.position.x !== node.position.x || before.position.y !== node.position.y);
      })
      .map((node) => node.id),
  };
}

/**
 * Tidies only the requested local subgraph. Non-requested constructs never
 * move, requested pinned constructs stay fixed, and all existing manual
 * indicator/edge/anchor metadata survives the projection.
 */
export function tidyConstructsPreservingLayoutV1(
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
  layout: DiagramLayoutState,
  constructIds: readonly string[],
) {
  const requested = new Set(constructIds);
  const selected = nodes.filter((node) => requested.has(node.id));
  if (selected.length < 2) {
    return { nodes, diagramLayout: layout, movedConstructIds: [] as string[] };
  }
  const selectedIds = new Set(selected.map((node) => node.id));
  const selectedEdges = edges.filter((edge) => selectedIds.has(edge.source) && selectedIds.has(edge.target));
  const proposal = layoutSmartplsModel(selected, selectedEdges);
  const proposalMinX = Math.min(...proposal.map((node) => node.position.x));
  const proposalMinY = Math.min(...proposal.map((node) => node.position.y));
  const currentMinX = Math.min(...selected.map((node) => node.position.x));
  const currentMinY = Math.min(...selected.map((node) => node.position.y));
  const proposedPositions = new Map(proposal.map((node) => [node.id, {
    x: node.position.x - proposalMinX + currentMinX,
    y: node.position.y - proposalMinY + currentMinY,
  }]));
  const nextNodes = nodes.map((node) => {
    const proposed = proposedPositions.get(node.id);
    if (!proposed || layout.constructLayouts[node.id]?.pinned) return node;
    return { ...node, position: proposed };
  });
  const movedConstructIds = nextNodes.filter((node) => {
    const before = nodes.find((candidate) => candidate.id === node.id);
    return before && (before.position.x !== node.position.x || before.position.y !== node.position.y);
  }).map((node) => node.id);
  return {
    nodes: nextNodes,
    diagramLayout: movedConstructIds.length
      ? reconcileModelEditDiagramLayoutV1(layout, nextNodes, edges)
      : layout,
    movedConstructIds,
  };
}

function validColumns(columns: readonly string[], dataset: Dataset, reservedGroupColumn?: string | null) {
  const reserved = reservedGroupColumn?.trim() ?? "";
  return [...new Set(columns)].filter((column): column is string =>
    typeof column === "string" && dataset.columns.includes(column) && column !== reserved);
}

function exactModelEditId(value: string | null | undefined) {
  return typeof value === "string" && value.length > 0 && value === value.trim() ? value : null;
}

function blocked(code: string, message: string, correctiveAction: string): StrictModelEditIntentPlanV1 {
  return { status: "blocked", code, message, correctiveAction };
}

function cloneEdgeLayout(layout: DiagramLayoutState["edgeLayouts"][string]) {
  return {
    ...layout,
    ...(layout.bendPoints ? { bendPoints: layout.bendPoints.map((point) => ({ ...point })) } : {}),
    ...(layout.labelOffset ? { labelOffset: { ...layout.labelOffset } } : {}),
  };
}
