import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
  SemDerivedTermV4,
  SemModelV4,
} from "./semModelV4";
import type {
  SemCapabilityCellIdV1,
  SemCapabilityDiagnosticV1,
  SemCapabilityEvidenceV1,
} from "./semCapabilityDecisionV1";

export const GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.higher_order_models",
  cell_id: "qpls3.pls.general_sem_higher_order_point",
  capability_version: "general_sem_pls_higher_order_point_v1",
} as const satisfies SemCapabilityCellIdV1);

export const GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.higher_order_models",
  cell_id: "qpls3.pls.general_sem_higher_order_full_model_case_bootstrap",
  capability_version: "general_sem_pls_higher_order_full_model_case_bootstrap_v1",
} as const satisfies SemCapabilityCellIdV1);

export function generalSemHocApproachTypeSupportedV1(
  approach: HigherOrderConstructionApproachV4,
  measurementType: HigherOrderMeasurementTypeV4,
  hocIsEndogenous: boolean,
): boolean {
  if (approach === "hybrid") return false;
  if (approach === "embedded_two_stage" || approach === "disjoint_two_stage") return true;
  if (approach === "extended_repeated_indicators") {
    return hocIsEndogenous
      && (measurementType === "reflective_formative" || measurementType === "formative_formative");
  }
  return measurementType === "reflective_reflective"
    || measurementType === "formative_reflective"
    || !hocIsEndogenous;
}

export interface GeneralSemHocContractPreflightV1 {
  readonly present: boolean;
  readonly contractCompiles: boolean;
  readonly outputId: string | null;
  readonly capabilityCells: readonly SemCapabilityCellIdV1[];
  readonly diagnostics: readonly SemCapabilityDiagnosticV1[];
  readonly evidence: readonly SemCapabilityEvidenceV1[];
}

function diagnostic(
  code: SemCapabilityDiagnosticV1["code"],
  subject: string | null,
  message: string,
  correction: string,
): SemCapabilityDiagnosticV1 {
  return { code, severity: "error", subject, message, corrections: [correction] };
}

function derivedKind(term: SemDerivedTermV4): string {
  return term.kind;
}

/**
 * Frontend mirror of the bounded HOC compiler/runtime predicate. Exact cell
 * identities are returned only when the resident model fits the supported
 * approach/type/topology matrix.
 */
export function preflightGeneralSemHocContractV1(
  model: SemModelV4,
  bootstrapRequested: boolean,
): GeneralSemHocContractPreflightV1 {
  const hocTerms = model.derived_terms.filter(
    (term): term is Extract<SemDerivedTermV4, { kind: "higher_order" }> => term.kind === "higher_order",
  );
  if (hocTerms.length === 0) {
    return {
      present: false,
      contractCompiles: false,
      outputId: null,
      capabilityCells: [],
      diagnostics: [],
      evidence: [],
    };
  }

  const capabilityCells = [
    GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1,
    ...(bootstrapRequested ? [GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1] : []),
  ];
  const evidence: SemCapabilityEvidenceV1[] = [{
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_higher_order_point_v1",
    description: "The bounded compiler binds one SemModelV4 HOC to explicit Mode A/B semantics, stable generated identities, and ordered approach-specific stages.",
  }, {
    evidence_id: "capability_contract:smartpls.higher_order_models:qpls3.pls.general_sem_higher_order_point:general_sem_pls_higher_order_point_v1",
    description: "The exact HOC point identity owns approach-specific staged PLS execution and canonical result authority.",
  }];
  if (bootstrapRequested) evidence.push({
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_higher_order_full_model_case_bootstrap_v1",
    description: "The supplemental HOC bootstrap compiler binds indexed raw-case resampling to complete approach-specific stage refitting.",
  }, {
    evidence_id: "capability_contract:smartpls.higher_order_models:qpls3.pls.general_sem_higher_order_full_model_case_bootstrap:general_sem_pls_higher_order_full_model_case_bootstrap_v1",
    description: "The exact HOC bootstrap identity owns indexed full-model case refitting across every compiled HOC stage.",
  });

  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (hocTerms.length !== 1) {
    diagnostics.push(diagnostic(
      "sem.capability.pls.higher_order_cardinality_not_executable",
      null,
      `General SEM HOC v1 supports exactly one higher-order term; found ${hocTerms.length}.`,
      "Keep exactly one non-nested second-order HOC in this bounded request.",
    ));
  }
  const hoc = hocTerms[0]!;
  const otherTerm = model.derived_terms.find((term) => term.id !== hoc.id);
  if (otherTerm) diagnostics.push(diagnostic(
    "sem.capability.pls.higher_order_derived_combination_not_executable",
    otherTerm.id,
    `Higher-order term ${hoc.id} cannot be combined with ${otherTerm.id} (${derivedKind(otherTerm)}).`,
    "Remove interaction, polynomial, nested, or additional HOC terms from this exact calculation.",
  ));

  const variables = new Map(model.variables.map((variable) => [variable.id, variable]));
  const expectedLocMode = hoc.measurement_type.startsWith("reflective_") ? "mode_a" : "mode_b";
  const componentIds = new Set(hoc.components);
  for (const componentId of [...hoc.components].sort()) {
    const component = variables.get(componentId);
    if (component?.kind !== "composite") {
      diagnostics.push(diagnostic(
        "sem.capability.pls.higher_order_component_not_executable",
        componentId,
        `Higher-order component ${componentId} must be an ordinary non-nested composite.`,
        "Select at least two ordinary composite lower-order components.",
      ));
      continue;
    }
    if (component.weighting.kind === "unit" || component.weighting.kind === "custom") {
      diagnostics.push(diagnostic(
        "sem.capability.pls.higher_order_measurement_mode_not_executable",
        componentId,
        `Higher-order component ${componentId} uses fixed/custom scoring outside HOC v1.`,
        "Use Mode A or Mode B and match it to the first term of the declared HCM type.",
      ));
      continue;
    }
    if (component.weighting.kind !== expectedLocMode) diagnostics.push(diagnostic(
      "sem.capability.pls.higher_order_measurement_mode_not_executable",
      componentId,
      `Higher-order component ${componentId} must use ${expectedLocMode === "mode_a" ? "Mode A" : "Mode B"}.`,
      "Match every LOC mode to the first term of the declared HCM type.",
    ));
  }
  const fixedScoring = model.variables.find((variable) => variable.kind === "composite"
    && !componentIds.has(variable.id)
    && (variable.weighting.kind === "unit" || variable.weighting.kind === "custom"));
  if (fixedScoring) diagnostics.push(diagnostic(
    "sem.capability.pls.higher_order_measurement_mode_not_executable",
    fixedScoring.id,
    `Composite ${fixedScoring.id} uses fixed/custom scoring outside HOC v1.`,
    "Use Mode A or Mode B for every composite in this exact request.",
  ));

  const authoredComponentPath = model.relations.find((relation) => relation.kind === "structural"
    && ((relation.source === hoc.output && componentIds.has(relation.target))
      || (relation.target === hoc.output && componentIds.has(relation.source))));
  if (authoredComponentPath) diagnostics.push(diagnostic(
    "sem.capability.pls.higher_order_shape_not_executable",
    authoredComponentPath.id,
    "The HOC-to-LOC relationship is compiler-generated and cannot also be an authored structural hypothesis.",
    "Remove the duplicate structural path and keep the relationship in the HOC declaration.",
  ));
  const hocIsEndogenous = model.relations.some((relation) => relation.kind === "structural"
    && relation.target === hoc.output);
  if (!generalSemHocApproachTypeSupportedV1(hoc.approach, hoc.measurement_type, hocIsEndogenous)) {
    diagnostics.push(diagnostic(
      hoc.approach === "hybrid"
        ? "sem.capability.pls.higher_order_hybrid_compatibility_only"
        : "sem.capability.pls.higher_order_approach_type_topology_not_executable",
      hoc.id,
      `The ${hoc.approach}/${hoc.measurement_type} combination is outside the bounded topology predicate.`,
      "Use repeated RR/FR or exogenous RF/FF; endogenous extended RF/FF; or embedded/disjoint with any HCM type.",
    ));
  }

  const contractCompiles = diagnostics.length === 0;
  return {
    present: true,
    contractCompiles,
    outputId: hoc.output,
    capabilityCells,
    diagnostics,
    evidence,
  };
}
