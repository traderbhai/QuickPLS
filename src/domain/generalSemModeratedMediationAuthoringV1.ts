import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import { parseGeneralSemConfigV1, type GeneralSemConfigV1 } from "./generalSemConfigV1";
import { specificDirectedPathIdentityV1 } from "./generalSemCapabilityPreflightV1";
import {
  compareUtf8StringsV1,
  parseSemModelV4,
  type SemDerivedTermV4,
  type SemModelV4,
  type SemRelationV4,
} from "./semModelV4";

type StructuralRelationV1 = Extract<SemRelationV4, { kind: "structural" }>;
type TwoWayInteractionV1 = Extract<SemDerivedTermV4, { kind: "interaction_v2" }>;

export const GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_PROBES_V1 = [-1, 0, 1] as const;

export const GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1:
CapabilityCellReferenceV2 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
  capability_version: "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
});

export const GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1:
readonly CapabilityCellReferenceV2[] = Object.freeze([
  Object.freeze({
    registry_schema_version: 2,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  }),
  Object.freeze({
    registry_schema_version: 2,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
  }),
]);

export type GeneralSemModeratedMediationStageV1 = "first_stage" | "second_stage";

export interface GeneralSemModeratedMediationPathCandidateV1 {
  readonly pathId: string;
  readonly estimandId: string;
  readonly orderedRelationIds: readonly [string, string];
  readonly xId: string;
  readonly xLabel: string;
  readonly mediatorId: string;
  readonly mediatorLabel: string;
  readonly yId: string;
  readonly yLabel: string;
  readonly moderatorId: string;
  readonly moderatorLabel: string;
  readonly interactionId: string;
  readonly moderatedStage: GeneralSemModeratedMediationStageV1;
  readonly moderatedRelationId: string;
  readonly otherStageRelationId: string;
}

export type GeneralSemModeratedMediationTargetInventoryItemV1 =
  | { readonly id: "scientific_gamma"; readonly kind: "scientific_gamma"; readonly label: "Scientific gamma inference" }
  | {
    readonly id: `conditional_indirect:${-1 | 0 | 1}`;
    readonly kind: "conditional_indirect";
    readonly label: string;
    readonly moderatorValue: -1 | 0 | 1;
  }
  | {
    readonly id: "index_of_moderated_mediation";
    readonly kind: "index_of_moderated_mediation";
    readonly label: "Index of moderated mediation";
  };

export const GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_TARGET_INVENTORY_V1:
readonly GeneralSemModeratedMediationTargetInventoryItemV1[] = Object.freeze([
  Object.freeze({ id: "scientific_gamma", kind: "scientific_gamma", label: "Scientific gamma inference" }),
  ...GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_PROBES_V1.map((moderatorValue) => Object.freeze({
    id: `conditional_indirect:${moderatorValue}` as const,
    kind: "conditional_indirect" as const,
    label: `Conditional indirect effect at W = ${moderatorValue > 0 ? "+1" : moderatorValue}`,
    moderatorValue,
  })),
  Object.freeze({
    id: "index_of_moderated_mediation",
    kind: "index_of_moderated_mediation",
    label: "Index of moderated mediation",
  }),
]);

export interface GeneralSemModeratedMediationAuthoringIssueV1 {
  readonly code: string;
  readonly subject: string;
  readonly message: string;
  readonly correctiveAction: string;
}

interface GeneralSemModeratedMediationSelectionBaseV1 {
  readonly candidates: readonly GeneralSemModeratedMediationPathCandidateV1[];
  readonly selectedPathId: string | null;
  readonly selectedPath: GeneralSemModeratedMediationPathCandidateV1 | null;
  readonly autoSelected: boolean;
  readonly targetInventory: typeof GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_TARGET_INVENTORY_V1;
  readonly supplementalCapabilityCell: CapabilityCellReferenceV2;
  readonly capabilityDependencies: readonly CapabilityCellReferenceV2[];
}

export interface GeneralSemModeratedMediationSelectionReadyV1
  extends GeneralSemModeratedMediationSelectionBaseV1 {
  readonly status: "ready";
  readonly selectedPath: GeneralSemModeratedMediationPathCandidateV1;
  readonly revisedConfig: GeneralSemConfigV1;
  readonly issues: readonly [];
}

export interface GeneralSemModeratedMediationSelectionBlockedV1
  extends GeneralSemModeratedMediationSelectionBaseV1 {
  readonly status: "blocked";
  readonly revisedConfig: null;
  readonly issues: readonly GeneralSemModeratedMediationAuthoringIssueV1[];
}

export type GeneralSemModeratedMediationSelectionV1 =
  | GeneralSemModeratedMediationSelectionReadyV1
  | GeneralSemModeratedMediationSelectionBlockedV1;

function issue(
  code: string,
  subject: string,
  message: string,
  correctiveAction: string,
): GeneralSemModeratedMediationAuthoringIssueV1 {
  return { code, subject, message, correctiveAction };
}

function exactInteractionShapeV1(model: SemModelV4): {
  interaction: TwoWayInteractionV1 | null;
  focalRelation: StructuralRelationV1 | null;
  relations: readonly StructuralRelationV1[];
  issues: GeneralSemModeratedMediationAuthoringIssueV1[];
} {
  const issues: GeneralSemModeratedMediationAuthoringIssueV1[] = [];
  const interactions = model.derived_terms.filter(
    (term): term is TwoWayInteractionV1 => term.kind === "interaction_v2",
  );
  if (interactions.length !== 1 || model.derived_terms.length !== 1) {
    issues.push(issue(
      "general_sem.moderated_mediation.interaction_cardinality",
      "model.derived_terms",
      `Exactly one two-way interaction is required; found ${interactions.length}.`,
      "Keep one two-stage, strong-hierarchy interaction and remove HOCs, polynomials, and additional interactions from this revision.",
    ));
    return { interaction: null, focalRelation: null, relations: [], issues };
  }
  const interaction = interactions[0]!;
  if (interaction.operands.length !== 2
    || interaction.method !== "two_stage"
    || interaction.hierarchy_policy !== "strong"
    || interaction.product_indicator != null) {
    issues.push(issue(
      "general_sem.moderated_mediation.interaction_contract",
      interaction.id,
      "The interaction is not the exact two-stage, two-operand, strong-hierarchy contract.",
      "Use one two-stage interaction with exactly one focal predictor, one moderator, strong hierarchy, and no product-indicator settings.",
    ));
    return { interaction, focalRelation: null, relations: [], issues };
  }
  const relations = model.relations.filter((relation): relation is StructuralRelationV1 => (
    relation.kind === "structural" && (relation.role ?? "structural") === "structural"
  ));
  const focalRelations = relations.filter((relation) => (
    relation.id === interaction.focal_relation && relation.source === interaction.operands[0]
  ));
  if (focalRelations.length !== 1) {
    issues.push(issue(
      "general_sem.moderated_mediation.focal_relation",
      interaction.focal_relation,
      "The interaction does not bind to one exact authored focal structural relation.",
      "Retarget the interaction to the intended first- or second-stage path relation.",
    ));
    return { interaction, focalRelation: null, relations, issues };
  }
  const focalRelation = focalRelations[0]!;
  const effectRelations = relations.filter((relation) => (
    relation.source === interaction.output && relation.target === focalRelation.target
  ));
  const moderator = interaction.operands[1]!;
  const mainEffects = relations.filter((relation) => (
    relation.source === moderator && relation.target === focalRelation.target
  ));
  if (effectRelations.length !== 1 || mainEffects.length !== 1) {
    issues.push(issue(
      "general_sem.moderated_mediation.strong_hierarchy",
      interaction.id,
      "The interaction effect or moderator main-effect relation is missing or ambiguous.",
      "Keep exactly one interaction-output path and one moderator main-effect path into the moderated-stage outcome.",
    ));
  }
  return { interaction, focalRelation, relations, issues };
}

function enumerateCandidatesV1(model: SemModelV4): {
  candidates: GeneralSemModeratedMediationPathCandidateV1[];
  issues: GeneralSemModeratedMediationAuthoringIssueV1[];
} {
  let parsed: SemModelV4;
  try {
    parsed = parseSemModelV4(model);
  } catch (error) {
    return {
      candidates: [],
      issues: [issue(
        "general_sem.moderated_mediation.model_invalid",
        "model",
        error instanceof Error ? error.message : "The SemModelV4 authority is invalid.",
        "Keep the authored diagram unchanged and resolve the strict model diagnostic before selecting a path.",
      )],
    };
  }
  const shape = exactInteractionShapeV1(parsed);
  if (!shape.interaction || !shape.focalRelation || shape.issues.length > 0) {
    return { candidates: [], issues: shape.issues };
  }
  const labels = new Map(parsed.variables.map((variable) => [variable.id, variable.label]));
  const scientificVariableIds = new Set(parsed.variables
    .filter((variable) => variable.kind !== "derived")
    .map((variable) => variable.id));
  const moderatorId = shape.interaction.operands[1]!;
  const candidates = new Map<string, GeneralSemModeratedMediationPathCandidateV1>();

  for (const first of shape.relations) {
    if (!scientificVariableIds.has(first.source) || !scientificVariableIds.has(first.target)) continue;
    for (const second of shape.relations) {
      if (first.id === second.id
        || first.target !== second.source
        || !scientificVariableIds.has(second.target)) continue;
      const [xId, mediatorId, yId] = [first.source, first.target, second.target];
      if (new Set([xId, mediatorId, yId, moderatorId]).size !== 4) continue;
      const firstStage = first.id === shape.focalRelation.id
        && first.source === shape.interaction.operands[0]
        && first.target === shape.focalRelation.target;
      const secondStage = second.id === shape.focalRelation.id
        && second.source === shape.interaction.operands[0]
        && second.target === shape.focalRelation.target;
      if (!firstStage && !secondStage) continue;
      const orderedRelationIds = [first.id, second.id] as const;
      const pathId = specificDirectedPathIdentityV1(orderedRelationIds);
      candidates.set(pathId, {
        pathId,
        estimandId: pathId,
        orderedRelationIds,
        xId,
        xLabel: labels.get(xId) ?? xId,
        mediatorId,
        mediatorLabel: labels.get(mediatorId) ?? mediatorId,
        yId,
        yLabel: labels.get(yId) ?? yId,
        moderatorId,
        moderatorLabel: labels.get(moderatorId) ?? moderatorId,
        interactionId: shape.interaction.id,
        moderatedStage: firstStage ? "first_stage" : "second_stage",
        moderatedRelationId: firstStage ? first.id : second.id,
        otherStageRelationId: firstStage ? second.id : first.id,
      });
    }
  }
  const orderedCandidates = [...candidates.values()].sort((left, right) => (
    compareUtf8StringsV1(left.orderedRelationIds[0], right.orderedRelationIds[0])
    || compareUtf8StringsV1(left.orderedRelationIds[1], right.orderedRelationIds[1])
  ));
  if (orderedCandidates.length === 0) {
    shape.issues.push(issue(
      "general_sem.moderated_mediation.path_missing",
      shape.interaction.id,
      "The interaction does not moderate either stage of an eligible two-relation X → M → Y path.",
      "Keep the diagram unchanged and select an interaction on the first or second relation of one two-relation path.",
    ));
  }
  return { candidates: orderedCandidates, issues: shape.issues };
}

function sameSelectedRequestV1(
  config: GeneralSemConfigV1,
  selected: GeneralSemModeratedMediationPathCandidateV1,
): boolean {
  if (config.requested_effect_estimands.length === 0) return true;
  const [request] = config.requested_effect_estimands;
  return config.requested_effect_estimands.length === 1
    && request?.kind === "specific_path"
    && request.estimand_id === selected.estimandId
    && request.ordered_relation_ids.length === 2
    && request.ordered_relation_ids[0] === selected.orderedRelationIds[0]
    && request.ordered_relation_ids[1] === selected.orderedRelationIds[1];
}

export function buildGeneralSemModeratedMediationSelectionV1(input: {
  model: SemModelV4;
  config: GeneralSemConfigV1;
  selectedPathId?: string | null;
}): GeneralSemModeratedMediationSelectionV1 {
  const enumerated = enumerateCandidatesV1(input.model);
  const issues = [...enumerated.issues];
  let config: GeneralSemConfigV1 | null = null;
  try {
    config = parseGeneralSemConfigV1(input.config);
  } catch (error) {
    issues.push(issue(
      "general_sem.moderated_mediation.config_invalid",
      "general_sem_config",
      error instanceof Error ? error.message : "The GeneralSemConfigV1 authority is invalid.",
      "Repair the strict RecipeV4 configuration before creating a versioned path-selection revision.",
    ));
  }
  if (config?.inference.kind !== "case_bootstrap"
    || config.inference.interval !== "percentile"
    || config.inference.tail !== "two_sided") {
    issues.push(issue(
      "general_sem.moderated_mediation.bootstrap_required",
      "general_sem_config.inference",
      "Two-way moderated mediation requires two-sided percentile case-bootstrap inference.",
      "Choose full-model percentile bootstrap and keep the fixed two-sided inference policy.",
    ));
  }
  if ((config?.conditional_effect_probes.length ?? 0) > 0) {
    issues.push(issue(
      "general_sem.moderated_mediation.authored_probes_unsupported",
      "general_sem_config.conditional_effect_probes",
      "Authored probes are outside the bounded moderated-mediation cell.",
      "Remove authored probes; QuickPLS locks this cell to standardized W values -1, 0, and +1.",
    ));
  }

  const autoSelected = input.selectedPathId == null && enumerated.candidates.length === 1;
  const selectedPathId = input.selectedPathId ?? (autoSelected ? enumerated.candidates[0]!.pathId : null);
  const selectedPath = selectedPathId
    ? enumerated.candidates.find((candidate) => candidate.pathId === selectedPathId) ?? null
    : null;
  if (selectedPathId && !selectedPath) {
    issues.push(issue(
      "general_sem.moderated_mediation.path_selection_stale",
      selectedPathId,
      "The selected relation path is no longer eligible in the strict model authority.",
      "Review the current eligible paths and choose one by its stable relation identities.",
    ));
  } else if (!selectedPath && enumerated.candidates.length > 1) {
    issues.push(issue(
      "general_sem.moderated_mediation.path_selection_required",
      "general_sem_config.requested_effect_estimands",
      `${enumerated.candidates.length} eligible two-relation paths exist; QuickPLS will not choose one heuristically.`,
      "Select exactly one X → M → Y path before saving the new model-and-Recipe revision.",
    ));
  }
  if (config && selectedPath && !sameSelectedRequestV1(config, selectedPath)) {
    issues.push(issue(
      "general_sem.moderated_mediation.effect_requests_conflict",
      "general_sem_config.requested_effect_estimands",
      "The resident RecipeV4 contains effect requests other than the selected two-relation SpecificPath.",
      "Create a deliberate Recipe revision containing exactly this selected SpecificPath; do not silently discard existing estimands.",
    ));
  }

  const base = {
    candidates: enumerated.candidates,
    selectedPathId,
    selectedPath,
    autoSelected,
    targetInventory: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_TARGET_INVENTORY_V1,
    supplementalCapabilityCell: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
    capabilityDependencies: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1,
  };
  if (!config || !selectedPath || issues.length > 0) {
    return { ...base, status: "blocked", revisedConfig: null, issues };
  }
  const revisedConfig = parseGeneralSemConfigV1({
    ...config,
    requested_effect_estimands: [{
      kind: "specific_path",
      estimand_id: selectedPath.estimandId,
      ordered_relation_ids: [...selectedPath.orderedRelationIds],
    }],
  });
  return { ...base, status: "ready", selectedPath, revisedConfig, issues: [] };
}
