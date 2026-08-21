import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  preflightGeneralSemCbsemV1,
  preflightGeneralSemPlsV1,
} from "./generalSemCapabilityPreflightV1";
import {
  buildGeneralSemModeratedMediationSelectionV1,
  type GeneralSemModeratedMediationPathCandidateV1,
} from "./generalSemModeratedMediationAuthoringV1";
import {
  parseGeneralSemConfigV1,
  type GeneralSemConfigV1,
} from "./generalSemConfigV1";
import {
  INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL,
  INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL,
} from "./internalRecipeV4CbsemExecution";
import type {
  SemCapabilityCellIdV1,
  SemCapabilityDecisionV1,
} from "./semCapabilityDecisionV1";
import type { SemModelV4 } from "./semModelV4";

export type UnifiedSemCalculationMethodV1 = "pls_algorithm" | "pls_bootstrap" | "cbsem";
export type UnifiedSemInferenceChoiceV1 = "point" | "case_bootstrap";

export interface UnifiedSemBootstrapOptionsV1 {
  readonly resamples: number;
  readonly seed: number;
  readonly confidenceLevel: number;
}

export interface UnifiedSemCalculationPreflightV1 {
  /** Decisions are reusable only for this exact model/config authority. */
  readonly authorityKey: string;
  readonly pls?: SemCapabilityDecisionV1 | null;
  readonly cbsem?: SemCapabilityDecisionV1 | null;
}

export interface UnifiedSemCalculationContextV1 {
  readonly authorityKey: string;
  readonly model: SemModelV4;
  readonly config: GeneralSemConfigV1;
  readonly preflight?: UnifiedSemCalculationPreflightV1 | null;
  readonly canonicalDocument?: CanonicalResultDocumentV2 | null;
}

export interface UnifiedSemCanonicalFamilyInventoryV1 {
  readonly mediationRows: number;
  readonly moderationRows: number;
  readonly higherOrderStages: number;
  readonly moderatedMediationRows: number;
  readonly cbsemParameterRows: number;
  readonly cbsemFitRows: number;
  readonly bootstrapInferenceRows: number;
}

export interface UnifiedSemFeatureInventoryV1 {
  /** Researcher-authored, non-control structural paths used for effect discovery. */
  readonly structuralRelationCount: number;
  /** All structural regressions, including controls; used for CFA/SEM topology. */
  readonly structuralRegressionCount: number;
  readonly indirectPathCount: number;
  readonly indirectPathCountCapped: boolean;
  readonly twoWayInteractionCount: number;
  readonly higherOrderConstructCount: number;
  readonly commonFactorCount: number;
  readonly requestedSpecificPathCount: number;
  readonly moderatedMediationCandidateCount: number;
  readonly moderatedMediationSelectedPath: GeneralSemModeratedMediationPathCandidateV1 | null;
  readonly resultFamilies: UnifiedSemCanonicalFamilyInventoryV1;
}

export type UnifiedSemCalculationRouteV1 =
  | "legacy"
  | "general_sem_pls"
  | "general_sem_cbsem"
  | "exact_cbsem_compatibility";

export interface UnifiedSemModeratedMediationSetupV1 {
  readonly availableOnlyInBootstrap: true;
  readonly candidateCount: number;
  readonly selectedPath: GeneralSemModeratedMediationPathCandidateV1 | null;
  readonly autoSelected: boolean;
  readonly configurationRequired: boolean;
  readonly fixedTargetSummary: "1 scientific gamma, 3 conditional indirect effects at W = -1, 0, +1, and 1 index of moderated mediation";
}

export interface UnifiedSemCalculationPlanV1 {
  readonly schemaVersion: 1;
  readonly authorityKey: string | null;
  readonly method: UnifiedSemCalculationMethodV1;
  readonly route: UnifiedSemCalculationRouteV1;
  readonly inference: UnifiedSemInferenceChoiceV1;
  readonly inventory: UnifiedSemFeatureInventoryV1 | null;
  readonly featureSummaries: readonly string[];
  readonly expectedResultCategories: readonly string[];
  readonly decision: SemCapabilityDecisionV1 | null;
  readonly capabilityCells: readonly SemCapabilityCellIdV1[];
  readonly requestedConfig: GeneralSemConfigV1 | null;
  readonly moderatedMediation: UnifiedSemModeratedMediationSetupV1 | null;
  readonly canStart: boolean;
  readonly blockers: readonly string[];
  /** The compatibility CB controller performs its dataset-aware preflight on start. */
  readonly controllerPreflightRequired: boolean;
  readonly fallbackReason: "strict_authority_unavailable" | "ordinary_model" | null;
}

export type UnifiedSemCalculationActionV1 =
  | {
      readonly kind: "start";
      readonly plan: UnifiedSemCalculationPlanV1;
    }
  | {
      readonly kind: "open_advanced_parameter_table";
      readonly authorityKey: string;
      readonly plan: UnifiedSemCalculationPlanV1;
    }
  | {
      readonly kind: "configure_moderated_mediation";
      readonly authorityKey: string;
      /** Preserve the exact bootstrap setup while the path selector/revision opens. */
      readonly plan: UnifiedSemCalculationPlanV1;
      readonly candidates: readonly GeneralSemModeratedMediationPathCandidateV1[];
      readonly selectedPathId: string | null;
    };

const INDIRECT_PATH_COUNT_LIMIT_V1 = 10_000;
const FIXED_MODERATED_MEDIATION_TARGET_SUMMARY_V1 =
  "1 scientific gamma, 3 conditional indirect effects at W = -1, 0, +1, and 1 index of moderated mediation" as const;

type StructuralRelationV1 = Extract<SemModelV4["relations"][number], { kind: "structural" }>;

function scientificStructuralRelationsV1(model: SemModelV4): StructuralRelationV1[] {
  const scientificVariables = new Set(model.variables
    .filter((variable) => variable.kind !== "observed" || variable.role !== "indicator")
    .filter((variable) => variable.kind !== "derived")
    .map((variable) => variable.id));
  return model.relations.filter((relation): relation is StructuralRelationV1 => (
    relation.kind === "structural"
    && (relation.role ?? "structural") === "structural"
    && scientificVariables.has(relation.source)
    && scientificVariables.has(relation.target)
  ));
}

function scientificStructuralRegressionsV1(model: SemModelV4): StructuralRelationV1[] {
  const scientificVariables = new Set(model.variables
    .filter((variable) => variable.kind !== "observed" || variable.role !== "indicator")
    .filter((variable) => variable.kind !== "derived")
    .map((variable) => variable.id));
  return model.relations.filter((relation): relation is StructuralRelationV1 => (
    relation.kind === "structural"
    && scientificVariables.has(relation.source)
    && scientificVariables.has(relation.target)
  ));
}

function countSpecificIndirectPathsV1(model: SemModelV4): {
  count: number;
  capped: boolean;
} {
  const relations = scientificStructuralRelationsV1(model);
  const outgoing = new Map<string, StructuralRelationV1[]>();
  for (const relation of relations) {
    const bucket = outgoing.get(relation.source) ?? [];
    bucket.push(relation);
    outgoing.set(relation.source, bucket);
  }
  let count = 0;
  let capped = false;
  const visit = (nodeId: string, visited: ReadonlySet<string>, depth: number): void => {
    if (capped) return;
    for (const relation of outgoing.get(nodeId) ?? []) {
      if (visited.has(relation.target)) continue;
      const nextDepth = depth + 1;
      if (nextDepth >= 2) {
        count += 1;
        if (count > INDIRECT_PATH_COUNT_LIMIT_V1) {
          count = INDIRECT_PATH_COUNT_LIMIT_V1;
          capped = true;
          return;
        }
      }
      visit(relation.target, new Set([...visited, relation.target]), nextDepth);
      if (capped) return;
    }
  };
  for (const source of outgoing.keys()) {
    visit(source, new Set([source]), 0);
    if (capped) break;
  }
  return { count, capped };
}

function canonicalFamilyInventoryV1(
  document: CanonicalResultDocumentV2 | null | undefined,
): UnifiedSemCanonicalFamilyInventoryV1 {
  const results = document?.general_sem_results;
  return {
    mediationRows: (results?.specific_indirect_effects?.length ?? 0)
      + (results?.aggregate_effects?.length ?? 0),
    moderationRows: (results?.interaction_effects?.length ?? 0)
      + (results?.conditional_effects?.length ?? 0),
    higherOrderStages: results?.higher_order_stages?.length ?? 0,
    moderatedMediationRows: (results?.conditional_indirect_effects?.length ?? 0)
      + (results?.moderated_mediation_indices?.length ?? 0),
    cbsemParameterRows: results?.cbsem_parameters?.length ?? 0,
    cbsemFitRows: results?.cbsem_fit?.length ?? 0,
    bootstrapInferenceRows: results?.cbsem_bootstrap_inference?.length ?? 0,
  };
}

function requestedSpecificPathIdV1(config: GeneralSemConfigV1): string | null {
  const requested = config.requested_effect_estimands.filter((estimand) => estimand.kind === "specific_path");
  return requested.length === 1 ? requested[0]!.estimand_id : null;
}

export function detectUnifiedSemFeatureInventoryV1(
  context: UnifiedSemCalculationContextV1,
): UnifiedSemFeatureInventoryV1 {
  const indirect = countSpecificIndirectPathsV1(context.model);
  let moderatedMediationCandidateCount = 0;
  let moderatedMediationSelectedPath: GeneralSemModeratedMediationPathCandidateV1 | null = null;
  try {
    const selection = buildGeneralSemModeratedMediationSelectionV1({
      model: context.model,
      config: context.config,
      selectedPathId: requestedSpecificPathIdV1(context.config),
    });
    moderatedMediationCandidateCount = selection.candidates.length;
    moderatedMediationSelectedPath = selection.selectedPath;
  } catch {
    // Inventory remains descriptive and fail-closed; preflight owns diagnostics.
  }
  return {
    structuralRelationCount: scientificStructuralRelationsV1(context.model).length,
    structuralRegressionCount: scientificStructuralRegressionsV1(context.model).length,
    indirectPathCount: indirect.count,
    indirectPathCountCapped: indirect.capped,
    twoWayInteractionCount: context.model.derived_terms.filter((term) => (
      term.kind === "interaction_v2" && term.operands.length === 2
    )).length,
    higherOrderConstructCount: context.model.derived_terms.filter((term) => term.kind === "higher_order").length,
    commonFactorCount: context.model.variables.filter((variable) => variable.kind === "common_factor").length,
    requestedSpecificPathCount: context.config.requested_effect_estimands.filter((estimand) => (
      estimand.kind === "specific_path"
    )).length,
    moderatedMediationCandidateCount,
    moderatedMediationSelectedPath,
    resultFamilies: canonicalFamilyInventoryV1(context.canonicalDocument),
  };
}

function configWithInferenceV1(
  config: GeneralSemConfigV1,
  inference: UnifiedSemInferenceChoiceV1,
  bootstrap: UnifiedSemBootstrapOptionsV1,
): GeneralSemConfigV1 {
  return parseGeneralSemConfigV1({
    ...config,
    inference: inference === "point"
      ? { kind: "none" }
      : {
          kind: "case_bootstrap",
          resamples: bootstrap.resamples,
          seed: bootstrap.seed,
          confidence_level: bootstrap.confidenceLevel,
          interval: "percentile",
          tail: "two_sided",
        },
  });
}

function featureSummariesV1(
  method: UnifiedSemCalculationMethodV1,
  inventory: UnifiedSemFeatureInventoryV1,
  moderatedMediation: UnifiedSemModeratedMediationSetupV1 | null,
): string[] {
  const summaries: string[] = [];
  if (method === "cbsem") {
    if (inventory.commonFactorCount > 0) summaries.push(`${inventory.commonFactorCount} common-factor construct${inventory.commonFactorCount === 1 ? "" : "s"}`);
    if (inventory.structuralRegressionCount > 0) summaries.push(`${inventory.structuralRegressionCount} structural relation${inventory.structuralRegressionCount === 1 ? "" : "s"}`);
    return summaries;
  }
  if (inventory.indirectPathCount > 0) summaries.push(`${inventory.indirectPathCount}${inventory.indirectPathCountCapped ? "+" : ""} indirect path${inventory.indirectPathCount === 1 ? "" : "s"}`);
  if (inventory.twoWayInteractionCount > 0) summaries.push(`${inventory.twoWayInteractionCount} two-way interaction${inventory.twoWayInteractionCount === 1 ? "" : "s"}`);
  if (inventory.higherOrderConstructCount > 0) summaries.push(`${inventory.higherOrderConstructCount} higher-order construct${inventory.higherOrderConstructCount === 1 ? "" : "s"}`);
  if (method === "pls_bootstrap" && moderatedMediation?.selectedPath) {
    summaries.push(`Moderated mediation (${moderatedMediation.selectedPath.moderatedStage === "first_stage" ? "first stage" : "second stage"})`);
  }
  return summaries;
}

function resultCategoriesV1(
  route: UnifiedSemCalculationRouteV1,
  inventory: UnifiedSemFeatureInventoryV1 | null,
  inference: UnifiedSemInferenceChoiceV1,
  moderatedMediation: UnifiedSemModeratedMediationSetupV1 | null,
): string[] {
  if (route === "legacy" || !inventory) return [];
  if (route === "general_sem_cbsem" || route === "exact_cbsem_compatibility") return [
    "Overview",
    "Measurement Model",
    ...(inventory.structuralRegressionCount > 0 ? ["Structural Model"] : []),
    "CB-SEM Parameters",
    "Model Fit and Identification",
    ...(inference === "case_bootstrap" ? ["Bootstrap Inference"] : []),
    "Diagnostics and Run Details",
  ];
  return [
    "Overview",
    "Measurement Model",
    "Structural Model",
    ...(inventory.indirectPathCount > 0 ? ["Direct, Indirect, and Total Effects"] : []),
    ...(inventory.twoWayInteractionCount > 0 ? ["Moderation and Conditional Effects"] : []),
    ...(inventory.higherOrderConstructCount > 0 ? ["Higher-Order Constructs"] : []),
    ...(moderatedMediation?.selectedPath ? ["Moderated Mediation"] : []),
    ...(inference === "case_bootstrap" ? ["Bootstrap Inference"] : []),
    "Diagnostics and Run Details",
  ];
}

function legacyPlanV1(
  method: UnifiedSemCalculationMethodV1,
  inference: UnifiedSemInferenceChoiceV1,
  inventory: UnifiedSemFeatureInventoryV1 | null,
  authorityKey: string | null,
  fallbackReason: UnifiedSemCalculationPlanV1["fallbackReason"],
): UnifiedSemCalculationPlanV1 {
  return {
    schemaVersion: 1,
    authorityKey,
    method,
    route: "legacy",
    inference,
    inventory,
    featureSummaries: inventory ? featureSummariesV1(method, inventory, null) : [],
    expectedResultCategories: [],
    decision: null,
    capabilityCells: [],
    requestedConfig: null,
    moderatedMediation: null,
    canStart: true,
    blockers: [],
    controllerPreflightRequired: false,
    fallbackReason,
  };
}

function invalidOptionsPlanV1(input: {
  method: UnifiedSemCalculationMethodV1;
  route: Exclude<UnifiedSemCalculationRouteV1, "legacy">;
  inference: UnifiedSemInferenceChoiceV1;
  context: UnifiedSemCalculationContextV1;
  inventory: UnifiedSemFeatureInventoryV1;
  message: string;
}): UnifiedSemCalculationPlanV1 {
  return {
    schemaVersion: 1,
    authorityKey: input.context.authorityKey,
    method: input.method,
    route: input.route,
    inference: input.inference,
    inventory: input.inventory,
    featureSummaries: featureSummariesV1(input.method, input.inventory, null),
    expectedResultCategories: resultCategoriesV1(input.route, input.inventory, input.inference, null),
    decision: null,
    capabilityCells: [],
    requestedConfig: null,
    moderatedMediation: null,
    canStart: false,
    blockers: [input.message],
    controllerPreflightRequired: false,
    fallbackReason: null,
  };
}

export function resolveUnifiedSemCalculationV1(input: {
  readonly method: UnifiedSemCalculationMethodV1;
  readonly context?: UnifiedSemCalculationContextV1 | null;
  readonly cbsemInference?: UnifiedSemInferenceChoiceV1;
  /** Undefined preserves a saved selection; null explicitly requests none. */
  readonly moderatedMediationPathId?: string | null;
  readonly bootstrap: UnifiedSemBootstrapOptionsV1;
}): UnifiedSemCalculationPlanV1 {
  const inference: UnifiedSemInferenceChoiceV1 = input.method === "pls_bootstrap"
    ? "case_bootstrap"
    : input.method === "cbsem"
      ? input.cbsemInference ?? (input.context?.config.inference.kind === "case_bootstrap" ? "case_bootstrap" : "point")
      : "point";
  if (!input.context) return legacyPlanV1(input.method, inference, null, null, "strict_authority_unavailable");

  const context = input.context;
  const inventory = detectUnifiedSemFeatureInventoryV1(context);
  const strictOnlyPlsSemantics = inventory.commonFactorCount > 0
    || context.model.derived_terms.length > 0
    || context.model.variables.some((variable) => variable.kind === "derived")
    || context.model.parameters.some((parameter) => parameter.kind === "derived");
  const advancedPls = input.method === "pls_algorithm"
    ? inventory.indirectPathCount > 0
      || inventory.twoWayInteractionCount > 0
      || inventory.higherOrderConstructCount > 0
      || strictOnlyPlsSemantics
    : inventory.indirectPathCount > 0
      || inventory.twoWayInteractionCount > 0
      || inventory.higherOrderConstructCount > 0
      || strictOnlyPlsSemantics;
  if (input.method !== "cbsem" && !advancedPls) {
    return legacyPlanV1(input.method, inference, inventory, context.authorityKey, "ordinary_model");
  }

  const anticipatedCbsemCompatibility = input.method === "cbsem" && (
    context.model.data_binding.kind !== "raw"
    || (inference === "case_bootstrap"
      && inventory.commonFactorCount > 0
      && inventory.structuralRegressionCount === 0)
  );
  const anticipatedRoute: Exclude<UnifiedSemCalculationRouteV1, "legacy"> = input.method === "cbsem"
    ? anticipatedCbsemCompatibility ? "exact_cbsem_compatibility" : "general_sem_cbsem"
    : "general_sem_pls";
  let requestedConfig: GeneralSemConfigV1;
  try {
    requestedConfig = configWithInferenceV1(context.config, inference, input.bootstrap);
  } catch {
    return invalidOptionsPlanV1({
      method: input.method,
      route: anticipatedRoute,
      inference,
      context,
      inventory,
      message: "Review the bootstrap samples, confidence level, and seed. The current values do not satisfy this method's bounded inference contract.",
    });
  }
  let moderatedMediation: UnifiedSemModeratedMediationSetupV1 | null = null;
  if (input.method === "pls_bootstrap" && inventory.twoWayInteractionCount === 1) {
    const selectedPathId = input.moderatedMediationPathId === undefined
      ? requestedSpecificPathIdV1(context.config)
      : input.moderatedMediationPathId;
    const selection = buildGeneralSemModeratedMediationSelectionV1({
      model: context.model,
      config: requestedConfig,
      selectedPathId,
    });
    // Do not silently turn an ordinary moderation bootstrap into moderated
    // mediation merely because there is only one eligible indirect path. The
    // fixed five-target estimand is installed only after the researcher has
    // saved an explicit path selection.
    const explicitlySelectedPath = selectedPathId ? selection.selectedPath : null;
    if (selection.status === "ready" && explicitlySelectedPath) requestedConfig = selection.revisedConfig;
    moderatedMediation = {
      availableOnlyInBootstrap: true,
      candidateCount: selection.candidates.length,
      selectedPath: explicitlySelectedPath,
      autoSelected: false,
      configurationRequired: selection.candidates.length > 0 && explicitlySelectedPath === null,
      fixedTargetSummary: FIXED_MODERATED_MEDIATION_TARGET_SUMMARY_V1,
    };
  }

  const exactCbsemCompatibility = anticipatedCbsemCompatibility;
  if (exactCbsemCompatibility) {
    const route: UnifiedSemCalculationRouteV1 = "exact_cbsem_compatibility";
    return {
      schemaVersion: 1,
      authorityKey: context.authorityKey,
      method: input.method,
      route,
      inference,
      inventory,
      featureSummaries: featureSummariesV1(input.method, inventory, null),
      expectedResultCategories: resultCategoriesV1(route, inventory, inference, null),
      decision: null,
      capabilityCells: [
        INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL,
        ...(inference === "case_bootstrap" ? [INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL] : []),
      ],
      requestedConfig,
      moderatedMediation: null,
      canStart: true,
      blockers: [],
      controllerPreflightRequired: true,
      fallbackReason: null,
    };
  }

  // Preflight is intentionally recomputed from the resident model and the
  // exact settings chosen in this dialog. A previously rendered decision can
  // share an authority key while its model revision has already changed.
  const decision = input.method === "cbsem"
    ? preflightGeneralSemCbsemV1(context.model, requestedConfig)
    : preflightGeneralSemPlsV1(context.model, requestedConfig);
  const route: UnifiedSemCalculationRouteV1 = input.method === "cbsem"
    ? "general_sem_cbsem"
    : "general_sem_pls";
  const blockers = decision.diagnostics
    .filter((diagnostic) => diagnostic.severity === "error")
    .map((diagnostic) => `${diagnostic.message}${diagnostic.corrections[0] ? ` ${diagnostic.corrections[0]}` : ""}`);

  return {
    schemaVersion: 1,
    authorityKey: context.authorityKey,
    method: input.method,
    route,
    inference,
    inventory,
    featureSummaries: featureSummariesV1(input.method, inventory, moderatedMediation),
    expectedResultCategories: resultCategoriesV1(route, inventory, inference, moderatedMediation),
    decision,
    capabilityCells: decision.capability_cells,
    requestedConfig,
    moderatedMediation,
    canStart: decision.status !== "blocked",
    blockers,
    controllerPreflightRequired: false,
    fallbackReason: null,
  };
}

export function unifiedSemModeratedMediationCandidatesV1(
  context: UnifiedSemCalculationContextV1,
  bootstrap: UnifiedSemBootstrapOptionsV1,
): readonly GeneralSemModeratedMediationPathCandidateV1[] {
  const config = configWithInferenceV1(context.config, "case_bootstrap", bootstrap);
  return buildGeneralSemModeratedMediationSelectionV1({
    model: context.model,
    config,
    selectedPathId: requestedSpecificPathIdV1(context.config),
  }).candidates;
}
