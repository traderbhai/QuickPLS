import {
  canonicalizeSemModelV4,
  parseSemModelV4AuthoringDraft,
  validateSemModelV4,
  type CompositeWeightingV4,
  type FactorIdentificationV4,
  type HigherOrderConstructionApproachV4,
  type HigherOrderMeasurementTypeV4,
  type InteractionMethodV4,
  type ObservedRoleV4,
  type ProductIndicatorSpecificationV4,
  type SemConstraintV4,
  type SemDataBindingV4,
  type SemEndpointV4,
  type SemGroupV4,
  type SemModelV4,
  type SemModelV4Issue,
  type SemParameterTargetV4,
  type SemParameterV4,
  type SemRelationV4,
  type SemVariableV4,
} from "./semModelV4";

export const STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION = 1 as const;
export const GENERAL_SEM_INTERACTION_V2_EDITOR_INTENT_VERSION_V1 = 1 as const;
export const GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3 = 3 as const;

const LOWER_SHA256 = /^[0-9a-f]{64}$/;

export interface StandardSemModelV4AuthorityRecordV1 {
  readonly schema_version: typeof STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION;
  readonly model_document_sha256: string;
  readonly model: SemModelV4;
}

export interface StandardSemModelV4AuthorityCandidateV1 {
  readonly schema_version: typeof STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION;
  readonly expected_model_document_sha256: string;
  readonly model: SemModelV4;
  readonly readiness: "ready" | "draft";
  readonly readiness_issues: readonly SemModelV4Issue[];
}

export type StandardSemConstructRepresentationV1 =
  | { kind: "composite"; weighting: CompositeWeightingV4 }
  | { kind: "common_factor"; identification: FactorIdentificationV4 };

export type StandardSemRelationshipDefinitionV1 =
  | { kind: "structural"; source: string; target: string; label: string }
  | { kind: "covariance"; left: SemEndpointV4; right: SemEndpointV4; label: string }
  | { kind: "presentation_only_covariance"; left: string; right: string; label: string }
  | { kind: "control"; source: string; target: string; label: string };

export type StandardSemParameterSpecificationV1 =
  | {
    kind: "free";
    start: number | null;
    lower: number | null;
    upper: number | null;
    equality_label: string | null;
  }
  | { kind: "fixed"; value: number };

/**
 * General-SEM-only two-way moderation authoring contract.
 *
 * The locked method and hierarchy fields are deliberately carried on the
 * wire. A future authority revision must reject unsupported bytes rather than
 * silently reinterpret this intent as the legacy interaction shape.
 */
export interface AddGeneralSemInteractionV2EditorIntentV1 {
  readonly kind: "add_general_sem_interaction_v2";
  readonly intent_version: typeof GENERAL_SEM_INTERACTION_V2_EDITOR_INTENT_VERSION_V1;
  readonly sem_generation: "general_sem_v1";
  readonly label: string;
  readonly operands: readonly [predictor: string, moderator: string];
  readonly focal_relation: string;
  readonly outcome: string;
  readonly method: "two_stage";
  readonly hierarchy_policy: "strong";
}

export type ModeratingEffectTargetV1 =
  | { readonly kind: "focal_relation"; readonly relationId: string }
  | { readonly kind: "parent_interaction"; readonly interactionTermId: string };

export type ModeratingEffectOperandsV3 =
  | readonly [predictor: string, moderator: string]
  | readonly [predictor: string, firstModerator: string, secondModerator: string];

/**
 * Diagram-native General SEM moderation intent. The target is a relationship
 * or an existing two-way interaction; it is never an edge-to-edge SEM
 * relationship. Newly created V3 terms use canonical-safe deterministic IDs;
 * historical identities remain valid for replace/remove compatibility.
 */
export interface AddModeratingEffectIntentV3 {
  readonly kind: "add_moderating_effect_v3";
  readonly intent_version: typeof GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3;
  readonly sem_generation: "general_sem_v1";
  readonly label: string;
  readonly operands: ModeratingEffectOperandsV3;
  readonly target: ModeratingEffectTargetV1;
  readonly outcome: string;
  readonly method: "two_stage";
  readonly hierarchy_policy: "strong";
}

export interface ReplaceModeratingEffectIntentV1 {
  readonly kind: "replace_moderating_effect";
  readonly intent_version: typeof GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3;
  readonly sem_generation: "general_sem_v1";
  readonly term_id: string;
  readonly output_id: string;
  readonly label: string;
  readonly operands: ModeratingEffectOperandsV3;
  readonly target: ModeratingEffectTargetV1;
  readonly outcome: string;
  readonly method: "two_stage";
  readonly hierarchy_policy: "strong";
}

export interface RemoveModeratingEffectIntentV1 {
  readonly kind: "remove_moderating_effect";
  readonly intent_version: typeof GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3;
  readonly sem_generation: "general_sem_v1";
  readonly term_id: string;
  readonly output_id: string;
}

export interface AddGeneralSemHigherOrderEditorIntentV1 {
  readonly kind: "add_higher_order";
  readonly term_id: string;
  readonly output_id: string;
  readonly label: string;
  readonly components: string[];
  readonly approach: HigherOrderConstructionApproachV4;
  readonly measurement_type: HigherOrderMeasurementTypeV4;
  readonly initial_path?: {
    readonly relation_id: string;
    readonly source: string;
    readonly target: string;
    readonly label: string;
  };
}

export interface ReplaceGeneralSemHigherOrderEditorIntentV1 {
  readonly kind: "replace_higher_order";
  readonly term_id: string;
  readonly output_id: string;
  readonly label: string;
  readonly components: string[];
  readonly approach: HigherOrderConstructionApproachV4;
  readonly measurement_type: HigherOrderMeasurementTypeV4;
}

export interface RemoveGeneralSemHigherOrderEditorIntentV1 {
  readonly kind: "remove_higher_order";
  readonly term_id: string;
  readonly output_id: string;
}

export type StandardSemModelV4EditorIntentV1 =
  | { kind: "set_model_name"; name: string }
  | { kind: "replace_complete_model"; model: SemModelV4 }
  | {
    kind: "add_construct";
    variable_id: string;
    label: string;
    representation: StandardSemConstructRepresentationV1;
    indicators: Array<Extract<SemVariableV4, { kind: "observed" }>>;
  }
  | { kind: "add_observed_variable"; variable: Extract<SemVariableV4, { kind: "observed" }> }
  | { kind: "set_observed_role"; variable_id: string; role: ObservedRoleV4 }
  | { kind: "delete_observed_variable"; variable_id: string }
  | { kind: "rename_construct"; variable_id: string; label: string }
  | { kind: "set_construct_representation"; variable_id: string; representation: StandardSemConstructRepresentationV1 }
  | { kind: "delete_construct"; variable_id: string }
  | {
    kind: "assign_indicators";
    construct_id: string;
    indicators: Array<Extract<SemVariableV4, { kind: "observed" }>>;
    replacement_marker?: string | null;
  }
  | { kind: "remove_indicator"; construct_id: string; observed_id: string; replacement_marker?: string | null }
  | { kind: "add_cross_loading"; construct_id: string; observed_id: string }
  | { kind: "replace_observed_variable"; variable_id: string; replacement: Extract<SemVariableV4, { kind: "observed" }> }
  | { kind: "add_relationship"; relationship_id: string; definition: StandardSemRelationshipDefinitionV1 }
  | { kind: "replace_relationship"; relationship_id: string; definition: StandardSemRelationshipDefinitionV1 }
  | { kind: "delete_relationship"; relationship_id: string }
  | { kind: "set_parameter_specification"; parameter_id: string; specification: StandardSemParameterSpecificationV1; label?: string }
  | { kind: "restore_parameter"; parameter_id: string }
  | { kind: "set_factor_identification"; variable_id: string; identification: FactorIdentificationV4 }
  | { kind: "set_latent_mean"; variable_id: string; estimated: boolean }
  | { kind: "set_observed_intercept"; variable_id: string; estimated: boolean }
  | { kind: "set_ordinal_thresholds"; variable_id: string; estimated: boolean }
  | {
    kind: "add_interaction";
    term_id: string;
    output_id: string;
    label: string;
    predictor: string;
    moderator: string;
    focal_relation: string;
    outcome: string;
    method: InteractionMethodV4;
    product_indicator?: ProductIndicatorSpecificationV4 | null;
  }
  | AddGeneralSemInteractionV2EditorIntentV1
  | AddModeratingEffectIntentV3
  | ReplaceModeratingEffectIntentV1
  | RemoveModeratingEffectIntentV1
  | { kind: "add_polynomial"; term_id: string; output_id: string; label: string; source: string; degree: number }
  | { kind: "replace_polynomial"; term_id: string; source: string; degree: number }
  | AddGeneralSemHigherOrderEditorIntentV1
  | ReplaceGeneralSemHigherOrderEditorIntentV1
  | RemoveGeneralSemHigherOrderEditorIntentV1
  | { kind: "set_group"; group: SemGroupV4 }
  | { kind: "set_data_binding"; data_binding: SemDataBindingV4 };

export class StandardSemModelV4AuthorityError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
    public readonly corrective_action: string,
  ) {
    super(message);
    this.name = "StandardSemModelV4AuthorityError";
  }
}

type MutableModel = SemModelV4;

function fail(code: string, subject: string, message: string, correctiveAction: string): never {
  throw new StandardSemModelV4AuthorityError(code, subject, message, correctiveAction);
}

function record(value: unknown, subject: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail("standard_sem_authority.object_required", subject, `${subject} must be an object.`, "Refresh the canonical model authority and retry.");
  }
  return value as Record<string, unknown>;
}

function exactRecord(value: unknown, keys: readonly string[], subject: string) {
  const parsed = record(value, subject);
  const actual = Object.keys(parsed);
  const missing = keys.find((key) => !Object.prototype.hasOwnProperty.call(parsed, key));
  const unknown = actual.find((key) => !keys.includes(key));
  if (missing || unknown) {
    fail(
      missing ? "standard_sem_authority.field_missing" : "standard_sem_authority.field_unknown",
      `${subject}.${missing ?? unknown}`,
      missing ? `${subject}.${missing} is required.` : `${subject}.${unknown} is not part of the authority record.`,
      "Use the exact versioned Standard SemModelV4 authority record.",
    );
  }
  return parsed;
}

function requiredText(value: unknown, subject: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail("standard_sem_authority.text_required", subject, `${subject} must be a nonempty string.`, "Provide a stable nonempty identifier or label.");
  }
  return value.trim();
}

function stableId(value: unknown, subject: string): string {
  if (typeof value !== "string" || !value || value !== value.trim()) {
    fail("standard_sem_authority.stable_id_invalid", subject, `${subject} must be an exact nonempty stable ID without surrounding whitespace.`, "Use the exact canonical identifier bytes.");
  }
  return value;
}

function finiteOrNull(value: number | null, subject: string) {
  if (value !== null && (!Number.isFinite(value) || Object.is(value, -0))) {
    fail("standard_sem_authority.number_invalid", subject, `${subject} must be finite or null.`, "Enter a finite number or clear the value.");
  }
  return value;
}

export function parseStandardSemModelV4AuthorityRecordV1(input: unknown): StandardSemModelV4AuthorityRecordV1 {
  const authority = exactRecord(input, ["schema_version", "model_document_sha256", "model"], "authority");
  if (authority.schema_version !== STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION) {
    fail("standard_sem_authority.version_unsupported", "authority.schema_version", "The Standard authority schema version is unsupported.", "Open the model with a compatible QuickPLS build.");
  }
  const rawDigest = authority.model_document_sha256;
  if (typeof rawDigest !== "string" || rawDigest !== rawDigest.trim() || !LOWER_SHA256.test(rawDigest)) {
    fail("standard_sem_authority.digest_invalid", "authority.model_document_sha256", "The authority revision must be a lowercase SHA-256 digest.", "Refresh the model authority from the project service.");
  }
  const digest = rawDigest;
  const model = canonicalStrictDraft(authority.model);
  assertStableModelIds(model, "authority.model");
  assertControlRelationSources(model);
  return deepFreeze({ schema_version: STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION, model_document_sha256: digest, model });
}

export function reduceStandardSemModelV4AuthorityV1(
  authorityInput: StandardSemModelV4AuthorityRecordV1,
  intent: StandardSemModelV4EditorIntentV1,
): StandardSemModelV4AuthorityCandidateV1 {
  const authority = parseStandardSemModelV4AuthorityRecordV1(authorityInput);
  try {
    const candidate = intent.kind === "replace_complete_model"
      ? replaceCompleteModel(authority.model, intent.model)
      : structuredClone(authority.model) as MutableModel;
    if (intent.kind !== "replace_complete_model") {
      applyIntent(candidate, intent);
      normalizeFactorDisturbances(candidate);
    }
    const model = canonicalStrictDraft(candidate);
    assertControlRelationSources(model);
    const readinessIssues = validateSemModelV4(model);
    return deepFreeze({
      schema_version: STANDARD_SEM_MODEL_V4_AUTHORITY_VERSION,
      expected_model_document_sha256: authority.model_document_sha256,
      model,
      readiness: readinessIssues.length ? "draft" : "ready",
      readiness_issues: readinessIssues,
    });
  } catch (error) {
    if (error instanceof StandardSemModelV4AuthorityError) throw error;
    const detail = error instanceof Error ? error.message : "The strict SemModelV4 decoder rejected the candidate.";
    fail("standard_sem_authority.candidate_invalid", authority.model.id, detail, "Correct the editor intent and retry against the unchanged authority revision.");
  }
}

export const standardSemMeasurementRelationIdV1 = (constructId: string, observedId: string) =>
  `standard:v1:measurement:${encodeURIComponent(constructId)}:${encodeURIComponent(observedId)}`;
export const standardSemMeasurementParameterIdV1 = (constructId: string, observedId: string) =>
  `standard:v1:measurement-parameter:${encodeURIComponent(constructId)}:${encodeURIComponent(observedId)}`;
export const standardSemFactorVarianceParameterIdV1 = (factorId: string) =>
  `standard:v1:factor-variance:${encodeURIComponent(factorId)}`;
export const standardSemResidualVarianceParameterIdV1 = (observedId: string) =>
  `standard:v1:residual-variance:${encodeURIComponent(observedId)}`;
export const standardSemRelationshipParameterIdV1 = (relationshipId: string) =>
  `standard:v1:relationship-parameter:${encodeURIComponent(relationshipId)}`;
export const standardSemEffectsConstraintIdV1 = (factorId: string) =>
  `standard:v1:effects-coding:${encodeURIComponent(factorId)}`;
export const standardSemLatentMeanParameterIdV1 = (factorId: string) =>
  `standard:v1:latent-mean:${encodeURIComponent(factorId)}`;
export const standardSemObservedInterceptParameterIdV1 = (observedId: string) =>
  `standard:v1:observed-intercept:${encodeURIComponent(observedId)}`;
export const standardSemObservedThresholdParameterIdV1 = (observedId: string, index: number) =>
  `standard:v1:observed-threshold:${encodeURIComponent(observedId)}:${index}`;
export const standardSemGeneralSemInteractionV2TermIdV1 = (
  focalRelationId: string,
  predictorId: string,
  moderatorId: string,
) => `general-sem:v1:interaction:${encodeURIComponent(focalRelationId)}:${encodeURIComponent(predictorId)}:${encodeURIComponent(moderatorId)}`;
export const standardSemGeneralSemInteractionV2OutputIdV1 = (termId: string) =>
  `general-sem:v1:interaction-output:${encodeURIComponent(termId)}`;
export const standardSemGeneralSemInteractionV2ModeratorMainRelationIdV1 = (termId: string) =>
  `general-sem:v1:interaction-moderator-main:${encodeURIComponent(termId)}`;
export const standardSemGeneralSemInteractionV2EffectRelationIdV1 = (termId: string) =>
  `general-sem:v1:interaction-effect:${encodeURIComponent(termId)}`;
export const standardSemGeneralSemThreeWayInteractionTermIdV1 = (
  parentInteractionTermId: string,
  secondModeratorId: string,
) => `general-sem:v1:interaction-three-way:${encodeURIComponent(parentInteractionTermId)}:${encodeURIComponent(secondModeratorId)}`;
export const standardSemGeneralSemInteractionDependencyAnnotationIdV1 = (
  ownerTermId: string,
  subjectId: string,
) => `general-sem:v1:interaction-dependency:${encodeURIComponent(ownerTermId)}:${encodeURIComponent(subjectId)}`;
export const standardSemGeneralSemGeneratedHierarchyAnnotationIdV1 = (subjectId: string) =>
  `general-sem:v1:interaction-generated:${encodeURIComponent(subjectId)}`;

function standardSemModerationV3GeneratedIdV1(prefix: string, parts: readonly string[]): string {
  const encoder = new TextEncoder();
  const encoded = parts.map((part) => Array.from(
    encoder.encode(part),
    (byte) => byte.toString(16).padStart(2, "0"),
  ).join("")).join("_");
  return `${prefix}_${encoded}`;
}

export const standardSemGeneralSemModerationV3TwoWayTermIdV1 = (
  focalRelationId: string,
  predictorId: string,
  moderatorId: string,
) => standardSemModerationV3GeneratedIdV1(
  "general_sem_v1_moderation_term",
  [focalRelationId, predictorId, moderatorId],
);

export const standardSemGeneralSemModerationV3ThreeWayTermIdV1 = (
  parentInteractionTermId: string,
  secondModeratorId: string,
) => standardSemModerationV3GeneratedIdV1(
  "general_sem_v1_moderation_term",
  [parentInteractionTermId, secondModeratorId],
);

export const standardSemGeneralSemModerationV3OutputIdV1 = (termId: string) =>
  standardSemModerationV3GeneratedIdV1("general_sem_v1_moderation_output", [termId]);

export const standardSemGeneralSemModerationV3MainRelationIdV1 = (
  ownerTermId: string,
  operandId: string,
) => standardSemModerationV3GeneratedIdV1(
  "general_sem_v1_moderation_main_relation",
  [ownerTermId, operandId],
);

export const standardSemGeneralSemModerationV3EffectRelationIdV1 = (termId: string) =>
  standardSemModerationV3GeneratedIdV1("general_sem_v1_moderation_effect_relation", [termId]);

export const standardSemGeneralSemModerationV3ParameterIdV1 = (relationId: string) =>
  standardSemModerationV3GeneratedIdV1("general_sem_v1_moderation_parameter", [relationId]);

export const standardSemGeneralSemModerationV3DependencyAnnotationIdV1 = (
  ownerTermId: string,
  subjectId: string,
) => standardSemModerationV3GeneratedIdV1(
  "general-sem:v1:interaction-dependency:",
  [ownerTermId, subjectId],
);

export const standardSemGeneralSemModerationV3GeneratedAnnotationIdV1 = (subjectId: string) =>
  standardSemModerationV3GeneratedIdV1("general-sem:v1:interaction-generated:", [subjectId]);

export function standardSemGeneralSemModerationV3IdentityV1(
  target: ModeratingEffectTargetV1,
  operands: ModeratingEffectOperandsV3,
): { termId: string; outputId: string } {
  const termId = target.kind === "parent_interaction"
    ? standardSemGeneralSemModerationV3ThreeWayTermIdV1(target.interactionTermId, operands[2] ?? "")
    : standardSemGeneralSemModerationV3TwoWayTermIdV1(
      target.relationId,
      operands[0],
      operands[1],
    );
  return { termId, outputId: standardSemGeneralSemModerationV3OutputIdV1(termId) };
}

function applyIntent(model: MutableModel, intent: StandardSemModelV4EditorIntentV1) {
  switch (intent.kind) {
    case "replace_complete_model":
      return;
    case "set_model_name":
      model.name = requiredText(intent.name, "intent.name");
      return;
    case "add_construct":
      addConstruct(model, intent);
      return;
    case "add_observed_variable":
      addObservedVariable(model, intent.variable);
      return;
    case "set_observed_role":
      observed(model, intent.variable_id).role = intent.role;
      return;
    case "delete_observed_variable":
      observed(model, intent.variable_id);
      removeVariablesCascade(model, new Set([intent.variable_id]));
      return;
    case "rename_construct": {
      const variable = construct(model, intent.variable_id);
      variable.label = requiredText(intent.label, "intent.label");
      return;
    }
    case "set_construct_representation":
      setConstructRepresentation(model, intent.variable_id, intent.representation);
      return;
    case "delete_construct":
      construct(model, intent.variable_id);
      removeVariablesCascade(model, new Set([intent.variable_id]));
      return;
    case "assign_indicators":
      assignIndicators(model, intent.construct_id, intent.indicators, intent.replacement_marker ?? null);
      return;
    case "remove_indicator":
      removeIndicator(model, intent.construct_id, intent.observed_id, intent.replacement_marker ?? null);
      return;
    case "add_cross_loading":
      addCrossLoading(model, intent.construct_id, intent.observed_id);
      return;
    case "replace_observed_variable":
      replaceObserved(model, intent.variable_id, intent.replacement);
      return;
    case "add_relationship":
      addRelationship(model, intent.relationship_id, intent.definition);
      return;
    case "replace_relationship":
      replaceRelationship(model, intent.relationship_id, intent.definition);
      return;
    case "delete_relationship":
      deleteRelationship(model, intent.relationship_id);
      return;
    case "set_parameter_specification":
      setParameterSpecification(model, intent.parameter_id, intent.specification, intent.label);
      return;
    case "restore_parameter":
      restoreParameter(model, intent.parameter_id);
      return;
    case "set_factor_identification":
      setFactorIdentification(model, intent.variable_id, intent.identification);
      return;
    case "set_latent_mean":
      setLatentMean(model, intent.variable_id, intent.estimated);
      return;
    case "set_observed_intercept":
      setObservedIntercept(model, intent.variable_id, intent.estimated);
      return;
    case "set_ordinal_thresholds":
      setOrdinalThresholds(model, intent.variable_id, intent.estimated);
      return;
    case "add_interaction":
      addInteraction(model, intent);
      return;
    case "add_general_sem_interaction_v2":
      addGeneralSemInteractionV2(model, intent);
      return;
    case "add_moderating_effect_v3":
      addModeratingEffectV3(model, intent);
      return;
    case "replace_moderating_effect":
      replaceModeratingEffectV1(model, intent);
      return;
    case "remove_moderating_effect":
      removeModeratingEffectV1(model, intent);
      return;
    case "add_polynomial":
      addPolynomial(model, intent);
      return;
    case "replace_polynomial":
      replacePolynomial(model, intent);
      return;
    case "add_higher_order":
      addHigherOrder(model, intent);
      return;
    case "replace_higher_order":
      replaceHigherOrder(model, intent);
      return;
    case "remove_higher_order":
      removeHigherOrder(model, intent);
      return;
    case "set_group":
      model.group = structuredClone(intent.group);
      return;
    case "set_data_binding":
      model.data_binding = structuredClone(intent.data_binding);
      return;
  }
}

function replaceCompleteModel(current: SemModelV4, input: unknown): MutableModel {
  const replacement = canonicalStrictDraft(input);
  assertStableModelIds(replacement, "intent.model");
  if (replacement.id !== current.id) {
    fail(
      "standard_sem_authority.model_identity_mismatch",
      "intent.model.id",
      `The complete replacement model ID '${replacement.id}' does not match the active model ID '${current.id}'.`,
      "Keep the active model's exact stable ID and retry the complete-document edit.",
    );
  }
  if (replacement.data_binding.dataset_id !== current.data_binding.dataset_id) {
    fail(
      "standard_sem_authority.dataset_binding_switch_requires_descriptor_transaction",
      "intent.model.data_binding.dataset_id",
      `The complete replacement cannot switch dataset binding from '${current.data_binding.dataset_id}' to '${replacement.data_binding.dataset_id}'.`,
      "Keep the current dataset_id. Switch datasets through a separate descriptor-aware project transaction so model authority, resident data, and provenance change atomically.",
    );
  }
  if (JSON.stringify(replacement.annotations) !== JSON.stringify(current.annotations)) {
    fail(
      "standard_sem_authority.presentation_annotations_owned_by_layout",
      "intent.model.annotations",
      "The complete scientific-document replacement cannot change presentation annotations.",
      "Keep annotations unchanged and use the canvas presentation controls, including the scientific/presentation covariance controls, so the saved presentation layout remains authoritative.",
    );
  }
  if (JSON.stringify(replacement.presentation) !== JSON.stringify(current.presentation)) {
    fail(
      "standard_sem_authority.presentation_owned_by_layout",
      "intent.model.presentation",
      "The complete scientific-document replacement cannot change the embedded presentation lane.",
      "Keep presentation unchanged and use the canvas presentation layer for captions, notes, shapes, images, lines, positions, routing, and viewport settings.",
    );
  }
  return replacement;
}

function assertStableModelIds(model: SemModelV4, subject: string) {
  stableId(model.id, `${subject}.id`);
  for (const [index, variable] of model.variables.entries()) stableId(variable.id, `${subject}.variables[${index}].id`);
  for (const [index, relation] of model.relations.entries()) stableId(relation.id, `${subject}.relations[${index}].id`);
  for (const [index, parameter] of model.parameters.entries()) stableId(parameter.id, `${subject}.parameters[${index}].id`);
  for (const [index, constraint] of model.constraints.entries()) stableId(constraint.id, `${subject}.constraints[${index}].id`);
  for (const [index, term] of model.derived_terms.entries()) stableId(term.id, `${subject}.derived_terms[${index}].id`);
}

function assertControlRelationSources(model: SemModelV4) {
  const variables = new Map(model.variables.map((variable) => [variable.id, variable]));
  for (const relation of model.relations) {
    if (relation.kind !== "structural" || relation.role !== "control") continue;
    const source = variables.get(relation.source);
    if (source?.kind === "observed" && source.role !== "control") {
      fail(
        "standard_sem_authority.control_role_required",
        source.id,
        "An observed control path must originate at an observed variable with role control.",
        "Set the observed role to control in the same authority workflow before adding the path.",
      );
    }
  }
}

function addConstruct(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "add_construct" }>) {
  const id = stableId(intent.variable_id, "intent.variable_id");
  if (model.variables.some((variable) => variable.id === id)) duplicate("variable", id);
  const representation = normalizeRepresentation(intent.representation, intent.indicators, id);
  const variable = constructVariable(id, requiredText(intent.label, "intent.label"), representation, false);
  model.variables.push(variable);
  if (variable.kind === "common_factor") normalizeFactorDisturbances(model);
  assignIndicators(model, id, intent.indicators, null);
  if (variable.kind === "common_factor") {
    setFactorIdentification(model, id, representation.kind === "common_factor" ? representation.identification : variable.identification);
  }
}

function addObservedVariable(model: MutableModel, input: Extract<SemVariableV4, { kind: "observed" }>) {
  const variable = parseObserved(input);
  if (model.variables.some((candidate) => candidate.id === variable.id)) duplicate("variable", variable.id);
  model.variables.push(variable);
}

function addCrossLoading(model: MutableModel, constructId: string, observedId: string) {
  const target = construct(model, constructId);
  observed(model, observedId);
  if (measurementRelation(model, constructId, observedId)) duplicate("measurement relation", `${constructId}/${observedId}`);
  addMeasurement(model, constructId, observedId);
  if (target.kind === "common_factor") setFactorIdentification(model, constructId, target.identification);
}

function setConstructRepresentation(model: MutableModel, variableId: string, input: StandardSemConstructRepresentationV1) {
  const current = construct(model, variableId);
  const indicators = measurementObservedIds(model, variableId).map((id) => observed(model, id));
  const representation = normalizeRepresentation(input, indicators, variableId);
  const label = current.label;
  const meanEstimated = current.kind === "common_factor" && current.mean_policy.kind !== "fixed_zero";
  removeMeasurementBlock(model, variableId, false);
  const index = model.variables.findIndex((variable) => variable.id === variableId);
  model.variables[index] = constructVariable(variableId, label, representation, hasIncomingStructural(model, variableId));
  for (const indicator of indicators) addMeasurement(model, variableId, indicator.id);
  if (representation.kind === "common_factor") {
    normalizeFactorDisturbances(model);
    setFactorIdentification(model, variableId, representation.identification);
    if (meanEstimated) setLatentMean(model, variableId, true);
  }
}

function assignIndicators(
  model: MutableModel,
  constructId: string,
  inputs: Array<Extract<SemVariableV4, { kind: "observed" }>>,
  replacementMarker: string | null,
) {
  const target = construct(model, constructId);
  const seen = new Set<string>();
  for (const raw of inputs) {
    const parsed = parseObserved(raw);
    if (seen.has(parsed.id)) duplicate("indicator assignment", parsed.id);
    seen.add(parsed.id);
    const owner = measurementOwner(model, parsed.id);
    if (owner && owner !== constructId) removeIndicator(model, owner, parsed.id, replacementMarker);
    const existing = model.variables.find((variable) => variable.id === parsed.id);
    if (existing && existing.kind !== "observed") fail("standard_sem_authority.indicator_id_collision", parsed.id, `Indicator ${parsed.id} collides with a non-observed variable.`, "Choose the existing dataset-backed observed variable ID.");
    if (!existing) model.variables.push(parsed);
    else model.variables[model.variables.indexOf(existing)] = { ...parsed, role: existing.role === "structural" || existing.role === "both" ? "both" : "indicator" };
    if (!measurementRelation(model, constructId, parsed.id)) addMeasurement(model, constructId, parsed.id);
  }
  if (target.kind === "common_factor") setFactorIdentification(model, constructId, target.identification);
}

function removeIndicator(model: MutableModel, constructId: string, observedId: string, replacementMarker: string | null) {
  const target = construct(model, constructId);
  observed(model, observedId);
  const relation = measurementRelation(model, constructId, observedId);
  if (!relation) missing("measurement relation", `${constructId}/${observedId}`);
  if (target.kind === "common_factor" && target.identification.kind === "marker_loading" && target.identification.indicator === observedId) {
    const remaining = measurementObservedIds(model, constructId).filter((id) => id !== observedId);
    if (!replacementMarker || !remaining.includes(replacementMarker)) {
      fail("standard_sem_authority.marker_replacement_required", observedId, "Removing the current marker would leave the common factor unidentified.", "Choose one remaining indicator as the replacement marker in the same atomic edit.");
    }
    target.identification = { kind: "marker_loading", indicator: replacementMarker };
  }
  model.relations = model.relations.filter((candidate) => candidate.id !== relation.id);
  removeParameters(model, new Set([relation.parameter, standardSemResidualVarianceParameterIdV1(observedId)]));
  if (target.kind === "common_factor") setFactorIdentification(model, constructId, target.identification);
  const variable = observed(model, observedId);
  if (!isObservedReferenced(model, observedId)) {
    if (variable.role === "both") variable.role = "structural";
    else if (variable.role === "indicator" && !isBindingVariable(model, observedId)) removeVariablesCascade(model, new Set([observedId]));
  }
}

function replaceObserved(model: MutableModel, variableId: string, replacement: Extract<SemVariableV4, { kind: "observed" }>) {
  const current = observed(model, variableId);
  const parsed = parseObserved(replacement);
  if (parsed.id !== current.id) fail("standard_sem_authority.identity_change_forbidden", variableId, "Observed-variable replacement must retain its exact stable ID.", "Create a new observed variable for a new identity.");
  model.variables[model.variables.indexOf(current)] = parsed;
}

function addRelationship(
  model: MutableModel,
  relationshipId: string,
  definition: StandardSemRelationshipDefinitionV1,
  parameterIdentity?: string,
) {
  const id = stableId(relationshipId, "intent.relationship_id");
  if (model.relations.some((relation) => relation.id === id) || model.annotations.some((annotation) => annotation.id === id)) duplicate("relationship", id);
  writeRelationship(model, id, definition, null, parameterIdentity);
}

function replaceRelationship(model: MutableModel, relationshipId: string, definition: StandardSemRelationshipDefinitionV1) {
  const relation = model.relations.find((candidate) => candidate.id === relationshipId) ?? null;
  const annotation = model.annotations.find((candidate) => candidate.id === relationshipId) ?? null;
  if (!relation && !annotation) missing("relationship", relationshipId);
  const preserved = relation ? model.parameters.find((parameter) => parameter.id === relation.parameter) ?? null : null;
  if (relation) {
    model.relations = model.relations.filter((candidate) => candidate.id !== relationshipId);
    removeParameters(model, new Set([relation.parameter, ...(relation.kind === "structural" && relation.intercept_parameter ? [relation.intercept_parameter] : [])]));
  }
  model.annotations = model.annotations.filter((candidate) => candidate.id !== relationshipId);
  writeRelationship(model, relationshipId, definition, preserved);
}

function deleteRelationship(model: MutableModel, relationshipId: string) {
  const relation = model.relations.find((candidate) => candidate.id === relationshipId);
  const annotation = model.annotations.find((candidate) => candidate.id === relationshipId);
  if (!relation && !annotation) missing("relationship", relationshipId);
  model.annotations = model.annotations.filter((candidate) => candidate.id !== relationshipId);
  if (relation) removeRelationCascade(model, relationshipId);
}

function writeRelationship(
  model: MutableModel,
  id: string,
  definition: StandardSemRelationshipDefinitionV1,
  preserved: SemParameterV4 | null,
  parameterIdentity?: string,
) {
  if (definition.kind === "presentation_only_covariance") {
    const left = anyVariable(model, definition.left).id;
    const right = anyVariable(model, definition.right).id;
    if (left === right) fail("standard_sem_authority.covariance_self_relation", id, "A presentation covariance needs two distinct variables.", "Choose two distinct constructs.");
    model.annotations.push({ kind: "display_only_covariance", id, left, right, label: requiredText(definition.label, "definition.label") });
    return;
  }
  const parameterId = preserved?.id
    ?? (parameterIdentity ? stableId(parameterIdentity, "intent.parameter_id") : standardSemRelationshipParameterIdV1(id));
  let relation: SemRelationV4;
  let target: SemParameterTargetV4;
  if (definition.kind === "structural" || definition.kind === "control") {
    const sourceVariable = structuralVariable(model, definition.source, "source");
    const targetVariable = structuralVariable(model, definition.target, "target");
    if (definition.kind === "control" && sourceVariable.kind === "observed" && sourceVariable.role !== "control") {
      fail("standard_sem_authority.control_role_required", sourceVariable.id, "An observed control path must originate at an observed variable with role control.", "Set the observed role to control in the same authority workflow before adding the path.");
    }
    const source = sourceVariable.id;
    const targetId = targetVariable.id;
    if (source === targetId) fail("standard_sem_authority.structural_self_relation", id, "A structural path cannot point to itself.", "Choose two distinct variables.");
    if (model.relations.some((candidate) => candidate.kind === "structural" && candidate.source === source && candidate.target === targetId)) duplicate("directed structural path", `${source}->${targetId}`);
    relation = {
      kind: "structural",
      id,
      source,
      target: targetId,
      parameter: parameterId,
      ...(definition.kind === "control" ? { role: "control" as const } : {}),
      intercept_parameter: null,
    };
    target = { kind: "regression", source, target: targetId };
  } else {
    const [left, right] = canonicalEndpointPair(definition.left, definition.right);
    if (endpointKey(left) === endpointKey(right)) fail("standard_sem_authority.covariance_self_relation", id, "A covariance needs two distinct endpoints.", "Choose two distinct variables or use a variance parameter.");
    endpointVariable(model, left);
    endpointVariable(model, right);
    if (model.relations.some((candidate) => candidate.kind === "covariance" && covarianceKey(candidate.left, candidate.right) === covarianceKey(left, right))) duplicate("covariance", covarianceKey(left, right));
    relation = { kind: "covariance", id, left, right, parameter: parameterId };
    target = { kind: "covariance", left, right };
  }
  model.relations.push(relation);
  model.parameters.push(parameterFromPreserved(preserved, parameterId, requiredText(definition.label, "definition.label"), target));
}

function setParameterSpecification(model: MutableModel, parameterId: string, specification: StandardSemParameterSpecificationV1, label?: string) {
  const parameter = model.parameters.find((candidate) => candidate.id === parameterId);
  if (!parameter) missing("parameter", parameterId);
  const normalizedLabel = label === undefined ? parameter.label : requiredText(label, "intent.label");
  if (specification.kind === "fixed") {
    if (!Number.isFinite(specification.value) || Object.is(specification.value, -0)) fail("standard_sem_authority.number_invalid", parameterId, "A fixed parameter value must be finite.", "Enter a finite fixed value.");
    replaceParameter(model, { kind: "fixed", id: parameter.id, label: normalizedLabel, target: parameter.target, value: specification.value, group_overrides: parameter.group_overrides ?? [] });
    return;
  }
  const start = finiteOrNull(specification.start, `${parameterId}.start`);
  const lower = finiteOrNull(specification.lower, `${parameterId}.lower`);
  const upper = finiteOrNull(specification.upper, `${parameterId}.upper`);
  if (lower !== null && upper !== null && lower > upper) fail("standard_sem_authority.bounds_invalid", parameterId, "The lower bound exceeds the upper bound.", "Choose ordered bounds.");
  if (start !== null && (lower !== null && start < lower || upper !== null && start > upper)) fail("standard_sem_authority.start_outside_bounds", parameterId, "The start value lies outside the selected bounds.", "Move the start value inside the bounds.");
  const equality = specification.equality_label?.trim() || null;
  if (equality !== null && !/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(equality)) fail("standard_sem_authority.equality_label_invalid", parameterId, "The equality label is invalid.", "Start with a letter and use letters, numbers, dot, underscore, or hyphen.");
  replaceParameter(model, { kind: "free", id: parameter.id, label: normalizedLabel, target: parameter.target, start, lower, upper, equality_label: equality, group_overrides: parameter.group_overrides ?? [] });
}

function restoreParameter(model: MutableModel, parameterId: string) {
  const parameter = model.parameters.find((candidate) => candidate.id === parameterId);
  if (!parameter) missing("parameter", parameterId);
  replaceParameter(model, defaultParameter(parameter.id, parameter.label, parameter.target));
  const factorId = parameter.target.kind === "loading"
    ? parameter.target.construct
    : parameter.target.kind === "variance" && parameter.target.endpoint.kind !== "residual_of"
      ? parameter.target.endpoint.id
      : null;
  const factor = factorId ? model.variables.find((variable) => variable.id === factorId && variable.kind === "common_factor") : null;
  if (factor?.kind === "common_factor") setFactorIdentification(model, factor.id, factor.identification);
}

function setFactorIdentification(model: MutableModel, variableId: string, identification: FactorIdentificationV4) {
  const factor = construct(model, variableId);
  if (factor.kind !== "common_factor") fail("standard_sem_authority.factor_required", variableId, "Factor identification applies only to a common factor.", "Choose Common factor before editing identification.");
  const previousIdentification = structuredClone(factor.identification);
  const effects = model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" }> => relation.kind === "measurement_effect" && relation.construct === variableId);
  model.constraints = model.constraints.filter((constraint) => constraint.id !== standardSemEffectsConstraintIdV1(variableId));
  if (previousIdentification.kind === "marker_loading") {
    const previousMarker = effects.find((effect) => effect.indicator === previousIdentification.indicator);
    if (previousMarker) {
      const previousMarkerParameter = parameter(model, previousMarker.parameter);
      if (previousMarkerParameter.kind === "fixed" && previousMarkerParameter.value === 1) {
        replaceParameter(model, defaultParameter(previousMarker.parameter, previousMarkerParameter.label, { kind: "loading", construct: variableId, indicator: previousMarker.indicator }, 0.7));
      }
    }
  }
  const varianceId = factor.disturbance_policy.parameter;
  if (previousIdentification.kind === "fixed_variance" && identification.kind !== "fixed_variance") {
    const previousVariance = parameter(model, varianceId);
    if (previousVariance.kind === "fixed" && previousVariance.value === 1) {
      replaceParameter(model, defaultParameter(varianceId, previousVariance.label, previousVariance.target, 1, 0));
    }
  }
  if (identification.kind === "marker_loading") {
    const marker = effects.find((effect) => effect.indicator === identification.indicator);
    if (!marker) fail("standard_sem_authority.marker_unknown", variableId, `Marker ${identification.indicator} is not assigned to the factor.`, "Choose one assigned indicator as the marker.");
    replaceParameter(model, { kind: "fixed", id: marker.parameter, label: parameter(model, marker.parameter).label, target: { kind: "loading", construct: variableId, indicator: marker.indicator }, value: 1, group_overrides: [] });
  } else if (identification.kind === "fixed_variance") {
    const variance = parameter(model, varianceId);
    replaceParameter(model, { kind: "fixed", id: varianceId, label: variance.label, target: variance.target, value: 1, group_overrides: [] });
  } else {
    if (effects.length < 3) fail("standard_sem_authority.effects_coding_indicators", variableId, "Effects coding requires at least three indicators.", "Assign at least three indicators or use another identification method.");
    model.constraints.push({ kind: "linear", id: standardSemEffectsConstraintIdV1(variableId), terms: effects.map((effect) => ({ parameter: effect.parameter, coefficient: 1 })), value: effects.length });
  }
  factor.identification = structuredClone(identification);
}

function setLatentMean(model: MutableModel, variableId: string, estimated: boolean) {
  const factor = construct(model, variableId);
  if (factor.kind !== "common_factor") fail("standard_sem_authority.factor_required", variableId, "Latent means apply only to common factors.", "Choose a common factor.");
  const id = standardSemLatentMeanParameterIdV1(variableId);
  if (!estimated) {
    removeParameters(model, new Set([id]));
    factor.mean_policy = { kind: "fixed_zero" };
    return;
  }
  if (!model.parameters.some((candidate) => candidate.id === id)) model.parameters.push(defaultParameter(id, `Mean(${factor.label})`, { kind: "mean", variable: variableId }, 0));
  factor.mean_policy = { kind: "estimated", parameter: id };
}

function setObservedIntercept(model: MutableModel, variableId: string, estimated: boolean) {
  const variable = observed(model, variableId);
  const id = standardSemObservedInterceptParameterIdV1(variableId);
  if (!estimated) return removeParameters(model, new Set([id]));
  if (["ordinal", "nominal", "identifier"].includes(variable.scale)) fail("standard_sem_authority.intercept_scale_invalid", variableId, `Observed intercepts are unavailable for ${variable.scale} variables.`, "Use thresholds for eligible ordinal indicators or correct the scale.");
  if (!model.parameters.some((candidate) => candidate.id === id)) model.parameters.push(defaultParameter(id, `Intercept(${variable.label})`, { kind: "intercept", variable: variableId }, 0));
}

function setOrdinalThresholds(model: MutableModel, variableId: string, estimated: boolean) {
  const variable = observed(model, variableId);
  const thresholdIds = model.parameters.filter((candidate) => candidate.target.kind === "threshold" && candidate.target.variable === variableId).map((candidate) => candidate.id);
  removeParameters(model, new Set(thresholdIds));
  if (!estimated) return;
  if (variable.scale !== "ordinal" || variable.categories.length < 2) fail("standard_sem_authority.threshold_scale_invalid", variableId, "Thresholds require an ordinal variable with at least two categories.", "Correct the observed metadata before estimating thresholds.");
  for (let index = 1; index < variable.categories.length; index += 1) {
    const id = standardSemObservedThresholdParameterIdV1(variableId, index);
    model.parameters.push(defaultParameter(id, `Threshold ${index}(${variable.label})`, { kind: "threshold", variable: variableId, index }, index - variable.categories.length / 2));
  }
}

function addGeneralSemInteractionV2(
  model: MutableModel,
  intent: AddGeneralSemInteractionV2EditorIntentV1,
) {
  if (intent.sem_generation !== "general_sem_v1") {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_generation_required",
      "intent.sem_generation",
      "The interaction_v2 intent is available only to a general_sem_v1 project authority.",
      "Create or activate a newly marked General SEM project; do not convert or relabel an ordinary project.",
    );
  }
  if (intent.intent_version !== GENERAL_SEM_INTERACTION_V2_EDITOR_INTENT_VERSION_V1) {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_intent_version_unsupported",
      "intent.intent_version",
      "The General SEM interaction authoring intent version is unsupported.",
      "Refresh the editor and submit the exact version-1 General SEM interaction intent.",
    );
  }
  if (intent.method !== "two_stage") {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_method_invalid",
      "intent.method",
      "This General SEM interaction intent supports only the qualified two-stage construction method.",
      "Choose the two-stage method or use a separately versioned and qualified interaction workflow.",
    );
  }
  if (intent.hierarchy_policy !== "strong") {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_hierarchy_invalid",
      "intent.hierarchy_policy",
      "This General SEM interaction intent requires strong hierarchy.",
      "Use strong hierarchy so every required lower-order main-effect path remains explicit.",
    );
  }
  if (!Array.isArray(intent.operands) || intent.operands.length !== 2) {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_operands_invalid",
      "intent.operands",
      "A two-way General SEM interaction requires exactly two operands in focal-predictor, moderator order.",
      "Provide exactly the focal predictor followed by one moderator.",
    );
  }

  const predictor = stableId(intent.operands[0], "intent.operands[0]");
  const moderator = stableId(intent.operands[1], "intent.operands[1]");
  const outcome = stableId(intent.outcome, "intent.outcome");
  const focalRelationId = stableId(intent.focal_relation, "intent.focal_relation");
  structuralVariable(model, predictor, "source");
  structuralVariable(model, moderator, "source");
  structuralVariable(model, outcome, "target");
  if (new Set([predictor, moderator, outcome]).size !== 3) {
    fail(
      "standard_sem_authority.interaction_variables_distinct",
      focalRelationId,
      "Predictor, moderator, and outcome must be distinct.",
      "Choose three distinct variables.",
    );
  }

  const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
    relation.id === focalRelationId && relation.kind === "structural");
  if (!focal || focal.role === "control" || focal.source !== predictor || focal.target !== outcome) {
    fail(
      "standard_sem_authority.focal_relation_invalid",
      focalRelationId,
      "The interaction focal relation must be the predictor-to-outcome structural-effect path.",
      "Select the exact current non-control focal relation.",
    );
  }

  const termId = standardSemGeneralSemInteractionV2TermIdV1(focalRelationId, predictor, moderator);
  const outputId = standardSemGeneralSemInteractionV2OutputIdV1(termId);
  if (model.derived_terms.some((term) => term.id === termId)) duplicate("interaction", termId);
  if (model.variables.some((variable) => variable.id === outputId)) duplicate("interaction output", outputId);
  const semanticDuplicate = model.derived_terms.find((term) => {
    if (term.kind !== "interaction" && term.kind !== "interaction_v2") return false;
    if (term.focal_relation !== focalRelationId) return false;
    if (term.kind === "interaction") return term.predictor === predictor && term.moderator === moderator;
    return term.kind === "interaction_v2"
      && term.operands.length === 2
      && term.operands[0] === predictor
      && term.operands[1] === moderator;
  });
  if (semanticDuplicate) {
    fail(
      "standard_sem_authority.interaction_duplicate",
      termId,
      `Moderating effect ${semanticDuplicate.id} already uses predictor ${predictor}, moderator ${moderator}, and focal relation ${focalRelationId}.`,
      "Choose a different moderator or focal relationship, or remove the existing moderating effect before retrying.",
    );
  }

  const existingModeratorMain = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
    relation.kind === "structural" && relation.source === moderator && relation.target === outcome);
  if (existingModeratorMain?.role === "control") {
    fail(
      "standard_sem_authority.general_sem_interaction_v2_main_effect_conflicts_control",
      existingModeratorMain.id,
      "The moderator-to-outcome relationship is a control path, not the required main-effect path.",
      "Convert the relationship to a structural-effect path before authoring the interaction.",
    );
  }

  model.variables.push({ kind: "derived", id: outputId, label: requiredText(intent.label, "intent.label") });
  model.derived_terms.push({
    kind: "interaction_v2",
    id: termId,
    output: outputId,
    operands: [predictor, moderator],
    focal_relation: focalRelationId,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
  if (!existingModeratorMain) {
    addRelationship(
      model,
      standardSemGeneralSemInteractionV2ModeratorMainRelationIdV1(termId),
      { kind: "structural", source: moderator, target: outcome, label: "Moderator main effect" },
    );
  }
  addRelationship(
    model,
    standardSemGeneralSemInteractionV2EffectRelationIdV1(termId),
    { kind: "structural", source: outputId, target: outcome, label: "Interaction effect" },
  );
}

type EditableModeratingEffectIntent = AddModeratingEffectIntentV3 | ReplaceModeratingEffectIntentV1;

interface ResolvedModeratingEffectV3 {
  operands: [string, string] | [string, string, string];
  focalRelationId: string;
  outcome: string;
  parentInteractionTermId: string | null;
}

function validateModeratingEffectIntentV3(intent: EditableModeratingEffectIntent) {
  if (intent.intent_version !== GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3
    || intent.sem_generation !== "general_sem_v1") {
    fail(
      "standard_sem_authority.moderating_effect_intent_version_invalid",
      "intent.intent_version",
      "Diagram-native moderation requires the exact version-3 General SEM intent.",
      "Refresh the model and retry from the current Moderating Effect dialog.",
    );
  }
  if (intent.method !== "two_stage" || intent.hierarchy_policy !== "strong") {
    fail(
      "standard_sem_authority.moderating_effect_method_invalid",
      "intent.method",
      "QuickPLS 2.53 moderation supports the qualified two-stage method with strong hierarchy.",
      "Use the qualified two-stage method and strong hierarchy.",
    );
  }
  if (!Array.isArray(intent.operands)
    || (intent.operands.length !== 2 && intent.operands.length !== 3)
    || new Set(intent.operands).size !== intent.operands.length) {
    fail(
      "standard_sem_authority.moderating_effect_operands_invalid",
      "intent.operands",
      "A moderating effect requires two or three distinct operands in authored order.",
      "Choose a predictor followed by one or two distinct moderators.",
    );
  }
}

function resolveModeratingEffectV3(
  model: MutableModel,
  intent: EditableModeratingEffectIntent,
): ResolvedModeratingEffectV3 {
  validateModeratingEffectIntentV3(intent);
  const operands = intent.operands.map((operand, index) => stableId(operand, `intent.operands[${index}]`)) as
    [string, string] | [string, string, string];
  const outcome = stableId(intent.outcome, "intent.outcome");
  for (const operand of operands) structuralVariable(model, operand, "source");
  structuralVariable(model, outcome, "target");
  if (new Set([...operands, outcome]).size !== operands.length + 1) {
    fail(
      "standard_sem_authority.moderating_effect_variables_not_distinct",
      "intent.operands",
      "The predictor, moderators, and outcome must be distinct variables.",
      "Choose distinct constructs for every role.",
    );
  }

  if (intent.target.kind === "focal_relation") {
    if (operands.length !== 2) {
      fail(
        "standard_sem_authority.three_way_parent_required",
        "intent.target",
        "A three-way moderation must extend an existing two-way interaction.",
        "Target the existing moderation anchor rather than the focal path.",
      );
    }
    const focalRelationId = stableId(intent.target.relationId, "intent.target.relationId");
    const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
      relation.id === focalRelationId && relation.kind === "structural");
    if (!focal || focal.role === "control" || focal.source !== operands[0] || focal.target !== outcome) {
      fail(
        "standard_sem_authority.moderating_effect_focal_invalid",
        focalRelationId,
        "The moderating effect must target the exact current predictor-to-outcome structural-effect path.",
        "Choose an eligible structural path and retry.",
      );
    }
    return { operands, focalRelationId, outcome, parentInteractionTermId: null };
  }

  if (operands.length !== 3) {
    fail(
      "standard_sem_authority.parent_interaction_requires_three_operands",
      "intent.operands",
      "Extending a moderation anchor requires predictor, first moderator, and second moderator operands.",
      "Choose the existing two-way effect and one additional moderator.",
    );
  }
  const parentInteractionTermId = stableId(
    intent.target.interactionTermId,
    "intent.target.interactionTermId",
  );
  const parent = model.derived_terms.find((term): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> =>
    term.kind === "interaction_v2" && term.id === parentInteractionTermId);
  if (!parent
    || parent.operands.length !== 2
    || parent.operands[0] !== operands[0]
    || parent.operands[1] !== operands[1]
    || parent.method !== "two_stage"
    || parent.hierarchy_policy !== "strong") {
    fail(
      "standard_sem_authority.parent_interaction_invalid",
      parentInteractionTermId,
      "Three-way moderation must extend the exact resident qualified two-way interaction.",
      "Refresh the Canvas and target an existing two-way moderation anchor.",
    );
  }
  const focalRelationId = parent.focal_relation;
  const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
    relation.kind === "structural" && relation.id === focalRelationId);
  if (!focal || focal.role === "control" || focal.source !== operands[0] || focal.target !== outcome) {
    fail(
      "standard_sem_authority.parent_interaction_focal_invalid",
      focalRelationId,
      "The parent interaction no longer resolves to the requested focal path and outcome.",
      "Refresh the model before extending the interaction.",
    );
  }
  return { operands, focalRelationId, outcome, parentInteractionTermId };
}

function hierarchyOrigin(model: MutableModel, subjectId: string) {
  const id = standardSemGeneralSemModerationV3GeneratedAnnotationIdV1(subjectId);
  if (!model.annotations.some((annotation) => annotation.id === id)) {
    model.annotations.push({
      kind: "note",
      id,
      subject: subjectId,
      text: "QuickPLS-generated strong-hierarchy dependency.",
    });
  }
}

function hierarchyReference(model: MutableModel, ownerTermId: string, subjectId: string) {
  const id = standardSemGeneralSemModerationV3DependencyAnnotationIdV1(ownerTermId, subjectId);
  if (!model.annotations.some((annotation) => annotation.id === id)) {
    model.annotations.push({
      kind: "note",
      id,
      subject: subjectId,
      text: `Required by moderating effect ${ownerTermId}.`,
    });
  }
}

function addModerationRelationship(
  model: MutableModel,
  relationshipId: string,
  definition: StandardSemRelationshipDefinitionV1,
) {
  addRelationship(
    model,
    relationshipId,
    definition,
    standardSemGeneralSemModerationV3ParameterIdV1(relationshipId),
  );
}

function ensureModerationMainEffect(
  model: MutableModel,
  operand: string,
  outcome: string,
  ownerTermId: string,
) {
  const conflicting = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
    relation.kind === "structural" && relation.source === operand && relation.target === outcome);
  if (conflicting?.role === "control") {
    fail(
      "standard_sem_authority.moderating_effect_main_conflicts_control",
      conflicting.id,
      "A required moderator main effect is currently authored as a control path.",
      "Convert it to a structural-effect path before adding moderation.",
    );
  }
  if (conflicting) {
    hierarchyReference(model, ownerTermId, conflicting.id);
    return conflicting.id;
  }
  const id = standardSemGeneralSemModerationV3MainRelationIdV1(ownerTermId, operand);
  addModerationRelationship(model, id, {
    kind: "structural",
    source: operand,
    target: outcome,
    label: "Moderator main effect",
  });
  hierarchyOrigin(model, id);
  hierarchyReference(model, ownerTermId, id);
  return id;
}

function findPairInteraction(
  model: MutableModel,
  first: string,
  second: string,
  focalRelationId: string,
  outcome: string,
) {
  return model.derived_terms.find((term): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> => {
    if (term.kind !== "interaction_v2"
      || term.operands.length !== 2
      || term.operands[0] !== first
      || term.operands[1] !== second
      || term.focal_relation !== focalRelationId
      || term.method !== "two_stage"
      || term.hierarchy_policy !== "strong") return false;
    return model.relations.some((relation) => relation.kind === "structural"
      && relation.source === term.output
      && relation.target === outcome
      && relation.role !== "control");
  });
}

function ensurePairInteraction(
  model: MutableModel,
  first: string,
  second: string,
  focalRelationId: string,
  outcome: string,
  ownerTermId: string,
) {
  const existing = findPairInteraction(model, first, second, focalRelationId, outcome);
  if (existing) {
    hierarchyReference(model, ownerTermId, existing.id);
    return existing;
  }
  const termId = standardSemGeneralSemModerationV3TwoWayTermIdV1(focalRelationId, first, second);
  const outputId = standardSemGeneralSemModerationV3OutputIdV1(termId);
  if (model.derived_terms.some((term) => term.id === termId)
    || model.variables.some((variable) => variable.id === outputId)) {
    fail(
      "standard_sem_authority.lower_order_interaction_identity_conflict",
      termId,
      "A required lower-order interaction identity is occupied by incompatible model content.",
      "Rename or remove the conflicting derived term before creating three-way moderation.",
    );
  }
  model.variables.push({
    kind: "derived",
    id: outputId,
    label: `${anyVariable(model, first).label} × ${anyVariable(model, second).label}`,
  });
  model.derived_terms.push({
    kind: "interaction_v2",
    id: termId,
    output: outputId,
    operands: [first, second],
    focal_relation: focalRelationId,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
  addModerationRelationship(model, standardSemGeneralSemModerationV3EffectRelationIdV1(termId), {
    kind: "structural",
    source: outputId,
    target: outcome,
    label: "Lower-order interaction effect",
  });
  hierarchyOrigin(model, termId);
  hierarchyReference(model, ownerTermId, termId);
  return model.derived_terms.find((term): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> =>
    term.kind === "interaction_v2" && term.id === termId)!;
}

function addModeratingEffectV3(
  model: MutableModel,
  intent: AddModeratingEffectIntentV3,
  preservedIdentity?: { termId: string; outputId: string },
) {
  const resolved = resolveModeratingEffectV3(model, intent);
  if (resolved.operands.length === 3 && model.derived_terms.some((term) => (
    term.kind === "interaction_v2" && term.operands.length === 3
  ))) {
    fail(
      "standard_sem_authority.multiple_three_way_interactions_unsupported",
      "intent.operands",
      "This bounded workflow supports one three-way moderating effect per model.",
      "Edit or remove the existing three-way effect before creating another one.",
    );
  }
  const [predictor, firstModerator, secondModerator] = resolved.operands;
  const identity = standardSemGeneralSemModerationV3IdentityV1(
    resolved.parentInteractionTermId
      ? { kind: "parent_interaction", interactionTermId: resolved.parentInteractionTermId }
      : { kind: "focal_relation", relationId: resolved.focalRelationId },
    resolved.operands,
  );
  const termId = preservedIdentity?.termId ?? identity.termId;
  const outputId = preservedIdentity?.outputId ?? identity.outputId;
  if (model.derived_terms.some((term) => term.id === termId)
    || model.variables.some((variable) => variable.id === outputId)) duplicate("moderating effect", termId);
  const semanticDuplicate = model.derived_terms.find((term) => term.kind === "interaction_v2"
    && term.focal_relation === resolved.focalRelationId
    && term.operands.length === resolved.operands.length
    && term.operands.every((operand, index) => operand === resolved.operands[index]));
  if (semanticDuplicate) {
    fail(
      "standard_sem_authority.moderating_effect_duplicate",
      termId,
      `Moderating effect ${semanticDuplicate.id} already represents the requested operands and focal relationship.`,
      "Edit the existing moderating effect or choose a different target.",
    );
  }

  for (const operand of resolved.operands) {
    ensureModerationMainEffect(model, operand, resolved.outcome, termId);
  }
  if (resolved.parentInteractionTermId && secondModerator) {
    hierarchyReference(model, termId, resolved.parentInteractionTermId);
    const firstModeratorMain = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
      relation.kind === "structural"
      && relation.source === firstModerator
      && relation.target === resolved.outcome
      && relation.role !== "control");
    if (!firstModeratorMain) {
      fail(
        "standard_sem_authority.parent_interaction_main_missing",
        resolved.parentInteractionTermId,
        "The parent interaction has no first-moderator main-effect path.",
        "Repair the parent interaction before extending it.",
      );
    }
    ensurePairInteraction(
      model,
      predictor,
      secondModerator,
      resolved.focalRelationId,
      resolved.outcome,
      termId,
    );
    ensurePairInteraction(
      model,
      firstModerator,
      secondModerator,
      firstModeratorMain.id,
      resolved.outcome,
      termId,
    );
  }

  model.variables.push({
    kind: "derived",
    id: outputId,
    label: requiredText(intent.label, "intent.label"),
  });
  model.derived_terms.push({
    kind: "interaction_v2",
    id: termId,
    output: outputId,
    operands: [...resolved.operands],
    focal_relation: resolved.focalRelationId,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
  addModerationRelationship(model, standardSemGeneralSemModerationV3EffectRelationIdV1(termId), {
    kind: "structural",
    source: outputId,
    target: resolved.outcome,
    label: resolved.operands.length === 3 ? "Three-way interaction effect" : "Interaction effect",
  });
}

function generatedHierarchyReferences(model: MutableModel, ownerTermId: string) {
  return model.annotations.filter((annotation): annotation is Extract<SemModelV4["annotations"][number], { kind: "note" }> =>
    annotation.kind === "note" && (
      annotation.id === standardSemGeneralSemModerationV3DependencyAnnotationIdV1(ownerTermId, annotation.subject)
      || annotation.id === standardSemGeneralSemInteractionDependencyAnnotationIdV1(ownerTermId, annotation.subject)
    ));
}

function removeModeratingEffectCore(
  model: MutableModel,
  termId: string,
  outputId: string,
) {
  const term = model.derived_terms.find((candidate): candidate is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> =>
    candidate.kind === "interaction_v2" && candidate.id === termId);
  if (!term || term.output !== outputId) {
    fail(
      "standard_sem_authority.moderating_effect_identity_mismatch",
      termId,
      "The requested moderating effect identity does not match the resident model.",
      "Refresh the Canvas and retry against the current moderating effect.",
    );
  }
  const requiredBy = model.annotations.filter((annotation) => annotation.kind === "note"
    && annotation.subject === termId
    && annotation.id.startsWith("general-sem:v1:interaction-dependency:"));
  if (requiredBy.length) {
    fail(
      "standard_sem_authority.moderating_effect_still_required",
      termId,
      "This two-way moderating effect is required by a three-way effect.",
      "Remove the dependent three-way effect first.",
    );
  }
  const dependencySubjects = generatedHierarchyReferences(model, termId).map((annotation) => annotation.subject);
  model.annotations = model.annotations.filter((annotation) => annotation.kind !== "note" || (
    annotation.id !== standardSemGeneralSemModerationV3DependencyAnnotationIdV1(termId, annotation.subject)
    && annotation.id !== standardSemGeneralSemInteractionDependencyAnnotationIdV1(termId, annotation.subject)
  ));
  removeVariablesCascade(model, new Set([outputId]));

  const stillReferenced = (subjectId: string) => model.annotations.some((annotation) =>
    annotation.kind === "note"
    && annotation.subject === subjectId
    && annotation.id.startsWith("general-sem:v1:interaction-dependency:"));
  for (const subjectId of dependencySubjects) {
    const originIds = [
      standardSemGeneralSemModerationV3GeneratedAnnotationIdV1(subjectId),
      standardSemGeneralSemGeneratedHierarchyAnnotationIdV1(subjectId),
    ];
    if (!model.annotations.some((annotation) => originIds.includes(annotation.id)) || stillReferenced(subjectId)) continue;
    const dependencyTerm = model.derived_terms.find((candidate) => candidate.id === subjectId);
    if (dependencyTerm) removeVariablesCascade(model, new Set([dependencyTerm.output]));
    else if (model.relations.some((relation) => relation.id === subjectId)) removeRelationCascade(model, subjectId);
    model.annotations = model.annotations.filter((annotation) => !originIds.includes(annotation.id));
  }
}

function replaceModeratingEffectV1(model: MutableModel, intent: ReplaceModeratingEffectIntentV1) {
  const termId = stableId(intent.term_id, "intent.term_id");
  const outputId = stableId(intent.output_id, "intent.output_id");
  removeModeratingEffectCore(model, termId, outputId);
  addModeratingEffectV3(model, {
    kind: "add_moderating_effect_v3",
    intent_version: intent.intent_version,
    sem_generation: intent.sem_generation,
    label: intent.label,
    operands: intent.operands,
    target: intent.target,
    outcome: intent.outcome,
    method: intent.method,
    hierarchy_policy: intent.hierarchy_policy,
  }, { termId, outputId });
}

function removeModeratingEffectV1(model: MutableModel, intent: RemoveModeratingEffectIntentV1) {
  if (intent.intent_version !== GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3
    || intent.sem_generation !== "general_sem_v1") {
    fail(
      "standard_sem_authority.moderating_effect_intent_version_invalid",
      "intent.intent_version",
      "Removing moderation requires the exact version-3 General SEM intent.",
      "Refresh the model and retry.",
    );
  }
  removeModeratingEffectCore(
    model,
    stableId(intent.term_id, "intent.term_id"),
    stableId(intent.output_id, "intent.output_id"),
  );
}

function addInteraction(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "add_interaction" }>) {
  const termId = stableId(intent.term_id, "intent.term_id");
  const outputId = stableId(intent.output_id, "intent.output_id");
  structuralVariable(model, intent.predictor, "source");
  structuralVariable(model, intent.moderator, "source");
  structuralVariable(model, intent.outcome, "target");
  if (new Set([intent.predictor, intent.moderator, intent.outcome]).size !== 3) fail("standard_sem_authority.interaction_variables_distinct", intent.term_id, "Predictor, moderator, and outcome must be distinct.", "Choose three distinct variables.");
  const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> => relation.id === intent.focal_relation && relation.kind === "structural");
  if (!focal || focal.source !== intent.predictor || focal.target !== intent.outcome) fail("standard_sem_authority.focal_relation_invalid", intent.focal_relation, "The interaction focal relation must be the predictor-to-outcome structural path.", "Select the exact current focal relation.");
  if (intent.method === "product_indicator" && !intent.product_indicator) fail("standard_sem_authority.product_indicator_spec_required", termId, "Product-indicator interactions require explicit construction settings.", "Choose centering, standardization, and pairing settings.");
  if (intent.method !== "product_indicator" && intent.product_indicator) fail("standard_sem_authority.product_indicator_spec_forbidden", termId, "Product-indicator settings apply only to the product-indicator method.", "Clear the product-indicator settings or choose that method.");
  if (model.derived_terms.some((term) => term.id === termId) || model.variables.some((variable) => variable.id === outputId)) duplicate("interaction", termId);
  const semanticDuplicate = model.derived_terms.find((term) => {
    if (term.kind !== "interaction" && term.kind !== "interaction_v2") return false;
    if (term.focal_relation !== intent.focal_relation) return false;
    if (term.kind === "interaction") {
      return term.predictor === intent.predictor && term.moderator === intent.moderator;
    }
    return term.kind === "interaction_v2"
      && term.operands.length === 2
      && term.operands[0] === intent.predictor
      && term.operands[1] === intent.moderator;
  });
  if (semanticDuplicate) {
    fail(
      "standard_sem_authority.interaction_duplicate",
      termId,
      `Moderating effect ${semanticDuplicate.id} already uses predictor ${intent.predictor}, moderator ${intent.moderator}, and focal relation ${intent.focal_relation}.`,
      "Choose a different moderator or focal relationship, or remove the existing moderating effect before retrying.",
    );
  }
  model.variables.push({ kind: "derived", id: outputId, label: requiredText(intent.label, "intent.label") });
  model.derived_terms.push({
    kind: "interaction",
    id: termId,
    output: outputId,
    predictor: intent.predictor,
    moderator: intent.moderator,
    focal_relation: intent.focal_relation,
    method: intent.method,
    ...(intent.product_indicator ? { product_indicator: structuredClone(intent.product_indicator) } : {}),
  });
  if (!model.relations.some((relation) => relation.kind === "structural" && relation.source === intent.moderator && relation.target === intent.outcome)) {
    addRelationship(model, `${termId}:moderator-main`, { kind: "structural", source: intent.moderator, target: intent.outcome, label: "Moderator main effect" });
  }
  addRelationship(model, `${termId}:interaction-effect`, { kind: "structural", source: outputId, target: intent.outcome, label: "Interaction effect" });
}

function addPolynomial(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "add_polynomial" }>) {
  const termId = stableId(intent.term_id, "intent.term_id");
  const outputId = stableId(intent.output_id, "intent.output_id");
  structuralVariable(model, intent.source, "source");
  const degree = polynomialDegree(intent.degree, termId);
  if (model.derived_terms.some((term) => term.id === termId) || model.variables.some((variable) => variable.id === outputId)) duplicate("polynomial term", termId);
  model.variables.push({ kind: "derived", id: outputId, label: requiredText(intent.label, "intent.label") });
  model.derived_terms.push({ kind: "polynomial", id: termId, output: outputId, source: intent.source, degree });
}

function replacePolynomial(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "replace_polynomial" }>) {
  const index = model.derived_terms.findIndex((term) => term.kind === "polynomial" && term.id === intent.term_id);
  if (index < 0) missing("polynomial term", intent.term_id);
  structuralVariable(model, intent.source, "source");
  const current = model.derived_terms[index] as Extract<SemModelV4["derived_terms"][number], { kind: "polynomial" }>;
  model.derived_terms[index] = { ...current, source: intent.source, degree: polynomialDegree(intent.degree, intent.term_id) };
}

function polynomialDegree(value: number, subject: string) {
  if (!Number.isInteger(value) || value < 2 || value > 255) fail("standard_sem_authority.polynomial_degree_invalid", subject, "Polynomial degree must be an integer from 2 through 255.", "Choose a supported nonlinear degree.");
  return value;
}

function addHigherOrder(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "add_higher_order" }>) {
  const termId = stableId(intent.term_id, "intent.term_id");
  const outputId = stableId(intent.output_id, "intent.output_id");
  const components = distinctExistingConstructs(model, intent.components, termId);
  if (model.derived_terms.some((term) => term.id === termId) || model.variables.some((variable) => variable.id === outputId)) duplicate("higher-order term", termId);
  model.variables.push({ kind: "derived", id: outputId, label: requiredText(intent.label, "intent.label") });
  model.derived_terms.push({ kind: "higher_order", id: termId, output: outputId, components, approach: intent.approach, measurement_type: intent.measurement_type });
  if (intent.initial_path) {
    const path = intent.initial_path;
    const other = path.source === outputId
      ? path.target
      : path.target === outputId
        ? path.source
        : fail(
          "standard_sem_authority.higher_order_initial_path_invalid",
          path.relation_id,
          "The initial HOC path must use the new HOC output as exactly one endpoint.",
          "Choose one ordinary construct and an incoming or outgoing HOC direction.",
        );
    if (components.includes(other)) fail(
      "standard_sem_authority.higher_order_initial_path_invalid",
      path.relation_id,
      "The initial HOC structural path cannot connect the HOC to one of its lower-order components.",
      "Choose an ordinary construct outside the component set.",
    );
    structuralVariable(model, other, path.source === other ? "source" : "target");
    addRelationship(model, stableId(path.relation_id, "intent.initial_path.relation_id"), {
      kind: "structural",
      source: path.source,
      target: path.target,
      label: requiredText(path.label, "intent.initial_path.label"),
    });
  }
}

function replaceHigherOrder(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "replace_higher_order" }>) {
  const index = model.derived_terms.findIndex((term) => term.id === intent.term_id && term.kind === "higher_order");
  if (index < 0) missing("higher-order term", intent.term_id);
  const current = model.derived_terms[index] as Extract<SemModelV4["derived_terms"][number], { kind: "higher_order" }>;
  const outputId = stableId(intent.output_id, "intent.output_id");
  if (current.output !== outputId) {
    fail(
      "standard_sem_authority.higher_order_output_mismatch",
      intent.term_id,
      "The replacement output identity differs from the resident higher-order construct.",
      "Refresh the model and edit the existing higher-order construct.",
    );
  }
  const outputIndex = model.variables.findIndex((variable) => variable.id === outputId && variable.kind === "derived");
  if (outputIndex < 0) missing("higher-order output", outputId);
  const output = model.variables[outputIndex] as Extract<SemVariableV4, { kind: "derived" }>;
  model.variables[outputIndex] = { ...output, label: requiredText(intent.label, "intent.label") };
  model.derived_terms[index] = { ...current, components: distinctExistingConstructs(model, intent.components, intent.term_id), approach: intent.approach, measurement_type: intent.measurement_type };
}

function removeHigherOrder(model: MutableModel, intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "remove_higher_order" }>) {
  const termId = stableId(intent.term_id, "intent.term_id");
  const outputId = stableId(intent.output_id, "intent.output_id");
  const term = model.derived_terms.find((candidate) => candidate.id === termId && candidate.kind === "higher_order");
  if (!term || term.output !== outputId) {
    fail(
      "standard_sem_authority.higher_order_identity_mismatch",
      termId,
      "The resident higher-order term/output identity differs from the removal request.",
      "Refresh the authority and remove the exact current higher-order construct.",
    );
  }
  if (!model.variables.some((variable) => variable.kind === "derived" && variable.id === outputId)) {
    missing("higher-order output", outputId);
  }
  const outputIds = new Set([outputId]);
  const incidentRelationIds = new Set(model.relations
    .filter((relation) => relationReferencesAny(relation, outputIds))
    .map((relation) => relation.id));
  const referencedBy = model.derived_terms.find((candidate) => (
    candidate.id !== termId && (
      derivedReferencesAny(candidate, outputIds)
      || isInteractionTerm(candidate) && incidentRelationIds.has(candidate.focal_relation)
    )
  ));
  if (referencedBy) {
    fail(
      "standard_sem_authority.higher_order_still_referenced",
      termId,
      `Higher-order construct ${termId} is required by derived term ${referencedBy.id}.`,
      "Remove the dependent derived term before removing this higher-order construct.",
    );
  }
  removeVariablesCascade(model, outputIds);
}

function normalizeRepresentation(
  input: StandardSemConstructRepresentationV1,
  indicators: readonly Extract<SemVariableV4, { kind: "observed" }>[],
  subject: string,
): StandardSemConstructRepresentationV1 {
  if (input?.kind === "composite") return { kind: "composite", weighting: structuredClone(input.weighting) };
  if (input?.kind !== "common_factor") fail("standard_sem_authority.construct_representation_required", subject, "A construct needs an explicit Composite or Common factor representation.", "Choose the scientific representation before committing the construct.");
  if (indicators.length < 2) fail("standard_sem_authority.factor_indicators_insufficient", subject, "A common factor needs at least two indicators.", "Assign at least two indicators or choose Composite.");
  if (input.identification.kind === "marker_loading") {
    const marker = input.identification.indicator;
    if (!indicators.some((indicator) => indicator.id === marker)) fail("standard_sem_authority.marker_unknown", subject, "The selected marker is not one of the factor indicators.", "Choose an assigned observed variable ID as the marker.");
  }
  if (input.identification.kind === "effects_coding" && indicators.length < 3) fail("standard_sem_authority.effects_coding_indicators", subject, "Effects coding requires at least three indicators.", "Assign at least three indicators or choose another identification method.");
  return { kind: "common_factor", identification: structuredClone(input.identification) };
}

function constructVariable(id: string, label: string, representation: StandardSemConstructRepresentationV1, endogenous: boolean): Exclude<SemVariableV4, { kind: "observed" | "derived" }> {
  if (representation.kind === "composite") return { kind: "composite", id, label, weighting: structuredClone(representation.weighting) };
  return {
    kind: "common_factor",
    id,
    label,
    identification: structuredClone(representation.identification),
    mean_policy: { kind: "fixed_zero" },
    disturbance_policy: endogenous
      ? { kind: "endogenous_disturbance", parameter: standardSemFactorVarianceParameterIdV1(id) }
      : { kind: "exogenous_variance", parameter: standardSemFactorVarianceParameterIdV1(id) },
  };
}

function addMeasurement(model: MutableModel, constructId: string, observedId: string) {
  const target = construct(model, constructId);
  const indicator = observed(model, observedId);
  const relationId = standardSemMeasurementRelationIdV1(constructId, observedId);
  const parameterId = standardSemMeasurementParameterIdV1(constructId, observedId);
  if (model.relations.some((relation) => relation.id === relationId)) duplicate("measurement relation", relationId);
  const effect = target.kind === "common_factor" || target.kind === "composite" && target.weighting.kind === "mode_a";
  const relation: SemRelationV4 = effect
    ? { kind: "measurement_effect", id: relationId, construct: constructId, indicator: observedId, parameter: parameterId }
    : { kind: "measurement_causal", id: relationId, indicator: observedId, composite: constructId, parameter: parameterId };
  const parameterTarget: SemParameterTargetV4 = effect
    ? { kind: "loading", construct: constructId, indicator: observedId }
    : { kind: "weight", indicator: observedId, composite: constructId };
  const marker = target.kind === "common_factor" && target.identification.kind === "marker_loading" && target.identification.indicator === observedId;
  model.relations.push(relation);
  model.parameters.push(marker
    ? { kind: "fixed", id: parameterId, label: `${target.label} -> ${indicator.label}`, target: parameterTarget, value: 1, group_overrides: [] }
    : defaultParameter(parameterId, `${target.label} -> ${indicator.label}`, parameterTarget, target.kind === "common_factor" ? 0.7 : null));
  if (target.kind === "common_factor") {
    const residualId = standardSemResidualVarianceParameterIdV1(observedId);
    if (!model.parameters.some((parameter) => parameter.id === residualId)) model.parameters.push(defaultParameter(residualId, `Residual variance(${indicator.label})`, { kind: "variance", endpoint: { kind: "residual_of", id: observedId } }, 0.5, 0));
  }
}

function removeMeasurementBlock(model: MutableModel, constructId: string, removeObservedVariables: boolean) {
  const current = construct(model, constructId);
  const disturbanceParameterId = current.kind === "common_factor"
    ? current.disturbance_policy.parameter
    : null;
  const factorVarianceParameterIds = model.parameters.flatMap((parameter) => parameter.target.kind === "variance"
    && parameter.target.endpoint.id === constructId
    && (parameter.target.endpoint.kind === "variable" || parameter.target.endpoint.kind === "disturbance_of")
    ? [parameter.id]
    : []);
  const relations = model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> => isMeasurementFor(relation, constructId));
  const observedIds = relations.map(measurementObservedId);
  const relationIds = new Set(relations.map((relation) => relation.id));
  model.relations = model.relations.filter((relation) => !relationIds.has(relation.id));
  removeParameters(model, new Set([
    ...relations.map((relation) => relation.parameter),
    ...factorVarianceParameterIds,
    ...(disturbanceParameterId ? [disturbanceParameterId] : []),
    standardSemLatentMeanParameterIdV1(constructId),
    ...observedIds.map(standardSemResidualVarianceParameterIdV1),
  ]));
  model.constraints = model.constraints.filter((constraint) => constraint.id !== standardSemEffectsConstraintIdV1(constructId));
  if (removeObservedVariables) for (const id of observedIds) if (!isObservedReferenced(model, id) && !isBindingVariable(model, id)) removeVariablesCascade(model, new Set([id]));
}

function normalizeFactorDisturbances(model: MutableModel) {
  for (const variable of model.variables) {
    if (variable.kind !== "common_factor") continue;
    const currentId = variable.disturbance_policy.parameter;
    const current = model.parameters.find((parameter) => parameter.id === currentId);
    const currentEndpoint = current?.target.kind === "variance" ? current.target.endpoint : null;
    const currentIsFactorVariance = currentEndpoint !== null
      && currentEndpoint.id === variable.id
      && (currentEndpoint.kind === "variable" || currentEndpoint.kind === "disturbance_of");
    const preserveFixedZero = currentIsFactorVariance
      && variable.disturbance_policy.kind === "fixed_zero"
      && current?.kind === "fixed"
      && Math.abs(current.value) <= 1e-12;
    const fallbackId = standardSemFactorVarianceParameterIdV1(variable.id);
    if (!currentIsFactorVariance && model.parameters.some((parameter) => parameter.id === fallbackId)) {
      duplicate("parameter", fallbackId);
    }
    const id = currentIsFactorVariance ? currentId : fallbackId;
    const endogenous = hasIncomingStructural(model, variable.id);
    variable.disturbance_policy = preserveFixedZero
      ? { kind: "fixed_zero", parameter: id }
      : endogenous
        ? { kind: "endogenous_disturbance", parameter: id }
        : { kind: "exogenous_variance", parameter: id };
    const target: SemParameterTargetV4 = { kind: "variance", endpoint: { kind: endogenous ? "disturbance_of" : "variable", id: variable.id } };
    const existing = model.parameters.find((parameter) => parameter.id === id);
    if (!existing) model.parameters.push(defaultParameter(id, `${endogenous ? "Disturbance variance" : "Variance"}(${variable.label})`, target, 1, 0));
    else replaceParameter(model, preserveFixedZero
      ? { ...existing, target } as SemParameterV4
      : { ...existing, target, label: `${endogenous ? "Disturbance variance" : "Variance"}(${variable.label})` } as SemParameterV4);
  }
}

function removeRelationCascade(model: MutableModel, relationId: string) {
  const relation = model.relations.find((candidate) => candidate.id === relationId);
  if (!relation) missing("relation", relationId);
  const outputIds = model.derived_terms.filter((term) => isInteractionTerm(term) && term.focal_relation === relationId).map((term) => term.output);
  model.relations = model.relations.filter((candidate) => candidate.id !== relationId);
  removeParameters(model, new Set([relation.parameter, ...(relation.kind === "structural" && relation.intercept_parameter ? [relation.intercept_parameter] : [])]));
  if (outputIds.length) removeVariablesCascade(model, new Set(outputIds));
  if (model.presentation.kind === "canvas") model.presentation.edges = model.presentation.edges.filter((edge) => edge.relation !== relationId);
}

function removeVariablesCascade(model: MutableModel, initial: Set<string>) {
  const variableIds = new Set(initial);
  let changed = true;
  while (changed) {
    changed = false;
    for (const term of model.derived_terms) if (!variableIds.has(term.output) && (
      derivedReferencesAny(term, variableIds)
      || isInteractionTerm(term) && model.relations.some((relation) => relation.id === term.focal_relation && relationReferencesAny(relation, variableIds))
    )) {
      variableIds.add(term.output);
      changed = true;
    }
  }
  const relationIds = new Set(model.relations.filter((relation) => relationReferencesAny(relation, variableIds)).map((relation) => relation.id));
  const parameterIds = new Set(model.relations.filter((relation) => relationIds.has(relation.id)).flatMap((relation) => [relation.parameter, ...(relation.kind === "structural" && relation.intercept_parameter ? [relation.intercept_parameter] : [])]));
  for (const parameter of model.parameters) if (targetReferencesAny(parameter.target, variableIds)) parameterIds.add(parameter.id);
  model.variables = model.variables.filter((variable) => !variableIds.has(variable.id));
  model.relations = model.relations.filter((relation) => !relationIds.has(relation.id));
  model.derived_terms = model.derived_terms.filter((term) => !variableIds.has(term.output) && !derivedReferencesAny(term, variableIds) && !(isInteractionTerm(term) && relationIds.has(term.focal_relation)));
  removeParameters(model, parameterIds);
  model.annotations = model.annotations.filter((annotation) => annotation.kind !== "display_only_covariance" || !variableIds.has(annotation.left) && !variableIds.has(annotation.right));
  if (model.presentation.kind === "canvas") {
    model.presentation.nodes = model.presentation.nodes.filter((node) => !variableIds.has(node.variable));
    model.presentation.edges = model.presentation.edges.filter((edge) => !relationIds.has(edge.relation));
  }
  if (model.group.kind === "observed_groups" && variableIds.has(model.group.grouping_variable)) model.group = { kind: "single_group" };
}

function removeParameters(model: MutableModel, ids: Set<string>) {
  if (!ids.size) return;
  model.parameters = model.parameters.filter((parameter) => !ids.has(parameter.id));
  model.constraints = model.constraints.filter((constraint) => !constraintReferencesAny(constraint, ids));
  for (const variable of model.variables) if (variable.kind === "common_factor" && variable.mean_policy.kind !== "fixed_zero" && ids.has(variable.mean_policy.parameter)) variable.mean_policy = { kind: "fixed_zero" };
}

function parameterFromPreserved(preserved: SemParameterV4 | null, id: string, label: string, target: SemParameterTargetV4): SemParameterV4 {
  if (!preserved) return defaultParameter(id, label, target);
  if (preserved.kind === "fixed") return { ...preserved, id, label, target };
  if (preserved.kind === "free") return { ...preserved, id, label, target };
  return defaultParameter(id, label, target);
}

function defaultParameter(id: string, label: string, target: SemParameterTargetV4, start: number | null = null, lower: number | null = null): Extract<SemParameterV4, { kind: "free" }> {
  return { kind: "free", id, label, target: structuredClone(target), start, lower, upper: null, equality_label: null, group_overrides: [] };
}

function replaceParameter(model: MutableModel, replacement: SemParameterV4) {
  const index = model.parameters.findIndex((candidate) => candidate.id === replacement.id);
  if (index < 0) missing("parameter", replacement.id);
  model.parameters[index] = structuredClone(replacement);
}

function parameter(model: MutableModel, id: string) {
  const value = model.parameters.find((candidate) => candidate.id === id);
  if (!value) missing("parameter", id);
  return value;
}

function anyVariable(model: MutableModel, id: string): SemVariableV4 {
  const variable = model.variables.find((candidate) => candidate.id === id);
  if (!variable) missing("variable", id);
  return variable;
}

function structuralVariable(model: MutableModel, id: string, position: "source" | "target") {
  const variable = anyVariable(model, id);
  if (variable.kind !== "observed") return variable;
  const allowed = position === "source"
    ? variable.role === "structural" || variable.role === "both" || variable.role === "control"
    : variable.role === "structural" || variable.role === "both";
  if (!allowed) {
    fail("standard_sem_authority.observed_role_invalid", id, `Observed variable ${id} cannot be a structural ${position} while its role is ${variable.role}.`, `Set an explicit ${position === "source" ? "structural, both, or control" : "structural or both"} role first.`);
  }
  if (variable.scale === "identifier") fail("standard_sem_authority.identifier_model_use_invalid", id, "Identifier variables cannot participate in structural relations.", "Use a modeled observed variable with a compatible scale.");
  return variable;
}

function construct(model: MutableModel, id: string): Exclude<SemVariableV4, { kind: "observed" }> {
  const variable = model.variables.find((candidate) => candidate.id === id);
  if (!variable || variable.kind === "observed") missing("construct variable", id);
  return variable;
}

function observed(model: MutableModel, id: string): Extract<SemVariableV4, { kind: "observed" }> {
  const variable = model.variables.find((candidate) => candidate.id === id);
  if (variable?.kind !== "observed") missing("observed variable", id);
  return variable;
}

function parseObserved(value: Extract<SemVariableV4, { kind: "observed" }>) {
  stableId(value.id, "intent.indicator.id");
  stableId(value.source_column, "intent.indicator.source_column");
  const model: SemModelV4 = {
    schema_version: 4,
    id: "standard:observed-validation",
    name: "Observed validation",
    variables: [structuredClone(value)],
    relations: [], parameters: [], constraints: [], derived_terms: [],
    group: { kind: "single_group" },
    data_binding: { kind: "raw", dataset_id: "validation", missing_data: "listwise_deletion", weight: null, cluster_variable: null, strata_variable: null },
    annotations: [], presentation: { kind: "none" },
  };
  return parseSemModelV4AuthoringDraft(model).variables[0] as Extract<SemVariableV4, { kind: "observed" }>;
}

function measurementRelation(model: MutableModel, constructId: string, observedId: string) {
  return model.relations.find((relation) => isMeasurementFor(relation, constructId) && measurementObservedId(relation) === observedId);
}

function measurementOwner(model: MutableModel, observedId: string) {
  const relation = model.relations.find((candidate): candidate is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> => isMeasurementRelation(candidate) && measurementObservedId(candidate) === observedId);
  return relation ? measurementConstructId(relation) : null;
}

function measurementObservedIds(model: MutableModel, constructId: string) {
  return model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> => isMeasurementFor(relation, constructId)).map(measurementObservedId);
}

function isMeasurementRelation(relation: SemRelationV4): relation is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> {
  return relation.kind === "measurement_effect" || relation.kind === "measurement_causal";
}

function measurementObservedId(relation: Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }>) {
  return relation.indicator;
}

function measurementConstructId(relation: Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }>) {
  return relation.kind === "measurement_effect" ? relation.construct : relation.composite;
}

function isMeasurementFor(relation: SemRelationV4, constructId: string): relation is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> {
  return relation.kind === "measurement_effect" ? relation.construct === constructId : relation.kind === "measurement_causal" && relation.composite === constructId;
}

function isObservedReferenced(model: MutableModel, observedId: string) {
  return model.relations.some((relation) => relationReferencesAny(relation, new Set([observedId])))
    || model.parameters.some((parameter) => targetReferencesAny(parameter.target, new Set([observedId])));
}

function isBindingVariable(model: MutableModel, id: string) {
  return model.group.kind === "observed_groups" && model.group.grouping_variable === id
    || model.data_binding.kind === "raw" && (model.data_binding.weight?.variable === id || model.data_binding.cluster_variable === id || model.data_binding.strata_variable === id);
}

function hasIncomingStructural(model: MutableModel, id: string) {
  return model.relations.some((relation) => relation.kind === "structural" && relation.target === id);
}

function endpointVariable(model: MutableModel, endpoint: SemEndpointV4) {
  const variable = model.variables.find((candidate) => candidate.id === endpoint.id);
  if (!variable) missing("covariance endpoint", endpoint.id);
  if (endpoint.kind === "residual_of" && variable.kind !== "observed") fail("standard_sem_authority.residual_endpoint_invalid", endpoint.id, "Residual covariance endpoints must be observed variables.", "Choose an observed indicator residual.");
  if (endpoint.kind === "disturbance_of" && variable.kind === "observed") fail("standard_sem_authority.disturbance_endpoint_invalid", endpoint.id, "Disturbance covariance endpoints must be construct variables.", "Choose an endogenous construct disturbance.");
  if (endpoint.kind === "disturbance_of" && !hasIncomingStructural(model, endpoint.id)) fail("standard_sem_authority.disturbance_endpoint_exogenous", endpoint.id, "An exogenous construct has no structural disturbance.", "Add an incoming structural path or choose model covariance.");
  return variable;
}

function canonicalEndpointPair(left: SemEndpointV4, right: SemEndpointV4): [SemEndpointV4, SemEndpointV4] {
  return endpointKey(left) <= endpointKey(right) ? [structuredClone(left), structuredClone(right)] : [structuredClone(right), structuredClone(left)];
}

function endpointKey(endpoint: SemEndpointV4) { return `${endpoint.kind}\0${endpoint.id}`; }
function covarianceKey(left: SemEndpointV4, right: SemEndpointV4) { const pair = canonicalEndpointPair(left, right); return `${endpointKey(pair[0])}\0${endpointKey(pair[1])}`; }

function relationReferencesAny(relation: SemRelationV4, ids: ReadonlySet<string>) {
  if (relation.kind === "measurement_effect") return ids.has(relation.construct) || ids.has(relation.indicator);
  if (relation.kind === "measurement_causal") return ids.has(relation.composite) || ids.has(relation.indicator);
  if (relation.kind === "structural") return ids.has(relation.source) || ids.has(relation.target);
  return ids.has(relation.left.id) || ids.has(relation.right.id);
}

function targetReferencesAny(target: SemParameterTargetV4, ids: ReadonlySet<string>) {
  if (target.kind === "loading") return ids.has(target.construct) || ids.has(target.indicator);
  if (target.kind === "weight") return ids.has(target.composite) || ids.has(target.indicator);
  if (target.kind === "regression") return ids.has(target.source) || ids.has(target.target);
  if (target.kind === "variance") return ids.has(target.endpoint.id);
  if (target.kind === "covariance") return ids.has(target.left.id) || ids.has(target.right.id);
  return ids.has(target.variable);
}

function isInteractionTerm(
  term: SemModelV4["derived_terms"][number],
): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction" | "interaction_v2" }> {
  return term.kind === "interaction" || term.kind === "interaction_v2";
}

function derivedReferencesAny(term: SemModelV4["derived_terms"][number], ids: ReadonlySet<string>) {
  if (term.kind === "interaction") return ids.has(term.output) || ids.has(term.predictor) || ids.has(term.moderator);
  if (term.kind === "interaction_v2") return ids.has(term.output) || term.operands.some((id) => ids.has(id));
  if (term.kind === "higher_order") return ids.has(term.output) || term.components.some((id) => ids.has(id));
  return ids.has(term.output) || ids.has(term.source);
}

function constraintReferencesAny(constraint: SemConstraintV4, ids: ReadonlySet<string>) {
  if (constraint.kind === "equality") return constraint.parameters.some((id) => ids.has(id));
  if (constraint.kind === "bound") return ids.has(constraint.parameter);
  return constraint.terms.some((term) => ids.has(term.parameter));
}

function distinctExistingConstructs(model: MutableModel, values: readonly string[], subject: string) {
  const ids = values.map((value, index) => stableId(value, `${subject}.components[${index}]`));
  if (ids.length < 2 || new Set(ids).size !== ids.length) fail("standard_sem_authority.higher_order_components_invalid", subject, "A higher-order construct needs at least two distinct components.", "Choose at least two distinct existing constructs.");
  ids.forEach((id) => construct(model, id));
  return ids;
}

function duplicate(kind: string, id: string): never {
  return fail("standard_sem_authority.identity_duplicate", id, `${kind} ${id} already exists.`, "Refresh the authority and use a new stable ID.");
}

function missing(kind: string, id: string): never {
  return fail("standard_sem_authority.target_missing", id, `${kind} ${id} does not exist in the current authority.`, "Refresh the authority and select an existing exact ID.");
}

function canonicalStrictDraft(input: unknown) {
  const strict = parseSemModelV4AuthoringDraft(input);
  return parseSemModelV4AuthoringDraft(canonicalizeSemModelV4(strict));
}

function deepFreeze<T>(value: T): T {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value as Record<string, unknown>).forEach(deepFreeze);
  return value;
}
