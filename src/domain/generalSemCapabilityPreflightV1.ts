import {
  parseGeneralSemConfigV1,
  type GeneralSemConfigV1,
} from "./generalSemConfigV1";
import {
  createSemCapabilityDecisionV1,
  type SemCapabilityDecisionV1,
  type SemCapabilityDiagnosticV1,
  type SemCapabilityEvidenceV1,
} from "./semCapabilityDecisionV1";
import {
  canonicalizeSemModelV4,
  compareUtf8StringsV1,
  compileCbsemPlanV2,
  compilePlsPlanV2,
  SemModelV4OperationError,
  validateSemModelV4,
  type SemModelV4,
  type SemRelationV4,
} from "./semModelV4";
import { sha256HexBytesV1 } from "./sha256V1";
import { capabilityRegistryV2 } from "./capabilityRegistryV2";
import { preflightGeneralSemHocContractV1 } from "./generalSemHigherOrderContractV1";

export const GENERAL_SEM_PLS_ESTIMATOR_ID_V1 = "qpls.pls_sem.v3" as const;
export const GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1 = "qpls.cbsem.v3" as const;

const PLS_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.mediation",
  capability_version: "pls_mediation_v1",
} as const;

const PLS_BOOTSTRAP_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
  capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
} as const;

export const GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_single_mediation_bootstrap",
  capability_version: "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
} as const);

export const GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
  capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
} as const);

export const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
  capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
} as const);

export const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
  capability_version: "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
} as const);

export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_point",
  capability_version: "general_sem_pls_three_way_moderation_point_v1",
} as const);

export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_bootstrap",
  capability_version: "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
} as const);

const CBSEM_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem",
  cell_id: "qpls3.cbsem.general_sem_ml",
  capability_version: "cbsem_general_sem_ml_v1",
} as const;

const CBSEM_BOOTSTRAP_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem_bootstrapping",
  cell_id: "qpls3.cbsem.bootstrap.recursive_sem",
  capability_version: "cbsem_exact_recursive_sem_case_bootstrap_v1",
} as const;

function exactRegistryDecisionStatusV1(
  cells: readonly {
    capability_id: string;
    cell_id: string;
    capability_version: string;
  }[],
): "supported" | "experimental" | null {
  let allStandard = true;
  for (const expected of cells) {
    const matches = capabilityRegistryV2.quickPlsCell(expected.cell_id).filter(({ row }) => (
      row.capability_id === expected.capability_id
    ));
    if (matches.length !== 1) return null;
    const actual = matches[0]!.cell;
    if (actual.capability_version !== expected.capability_version) return null;
    if (!capabilityRegistryV2.availability(actual.capability_id, actual.cell_id, true).selectable) return null;
    if (actual.surface !== "standard") allStandard = false;
  }
  return allStandard ? "supported" : "experimental";
}

const PLS_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "capability_registry_v2:smartpls.mediation:qpls3.pls.mediation:pls_mediation_v1",
    description: "Capability Registry V2 exposes the exact bounded mediation option.",
  },
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
    description: "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
  },
];

const PLS_BOOTSTRAP_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_multiple_mediation_bootstrap:general_sem_pls_full_model_case_bootstrap_v1",
  description: "Capability Registry V2 exposes this exact multiple-mediation, full-model percentile case-bootstrap combination.",
};

const PLS_SINGLE_MEDIATION_BOOTSTRAP_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_single_mediation_bootstrap:general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
  description: "Capability Registry V2 exposes the exact single-mediation full-model percentile case-bootstrap combination.",
};

const PLS_BOOTSTRAP_MECHANISM_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "capability_dependency:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
  description: "The exact General SEM cell uses the separately governed indexed case-resampling mechanism without inheriting that mechanism cell's release maturity.",
};

const PLS_BOOTSTRAP_COMPILER_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
  description: "The bootstrap compiler binds exact Recipe V4 inference settings to the General SEM config while retaining the proven point-scoring plan.",
};

const PLS_MULTIPLE_MODERATION_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1",
    description: "The bounded compiler projects one shared stage-one score model and jointly solves every qualified two-way interaction in each stage-two equation.",
  },
  {
    evidence_id: "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_multiple_two_way_moderation_point:general_sem_pls_multiple_two_way_moderation_point_v1",
    description: "Capability Registry V2 exposes the exact simultaneous interaction_v2 point-estimation option.",
  },
];

const PLS_MULTIPLE_MODERATION_BOOTSTRAP_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_bootstrap_v1",
    description: "The supplemental moderation bootstrap compiler binds percentile, two-sided full-model case resampling while preserving the point cell as the compiled artifact's primary authority.",
  },
  {
    evidence_id: "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap:general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
    description: "Capability Registry V2 exposes the exact gamma-only simultaneous interaction_v2 full-model case-bootstrap option.",
  },
];

const PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_two_way_moderated_mediation_bootstrap_v1",
    description: "The bounded compiler binds one exact two-relation path and one first- or second-stage two-way interaction to a shared five-target bootstrap ledger.",
  },
  {
    evidence_id: "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap:general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
    description: "Capability Registry V2 exposes the exact conditional-process full-model case-bootstrap option on its current authorized surface.",
  },
];

const PLS_THREE_WAY_MODERATION_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_three_way_moderation_point_v1",
    description: "The bounded compiler preserves strong hierarchy and jointly binds main, pairwise, and one three-way product to one stage-two solve.",
  },
  {
    evidence_id: "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_three_way_moderation_point:general_sem_pls_three_way_moderation_point_v1",
    description: "Capability Registry V2 exposes the exact bounded three-way moderation point cell.",
  },
];

const PLS_THREE_WAY_MODERATION_BOOTSTRAP_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "compiler:recipe_v4_to_compiled_pls_plan_v3_three_way_moderation_bootstrap_v1",
    description: "The bootstrap compiler binds one indexed full-model traversal and a shared three-way conditional-effects ledger.",
  },
  {
    evidence_id: "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_three_way_moderation_bootstrap:general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
    description: "Capability Registry V2 exposes the exact bounded three-way full-model case-bootstrap cell.",
  },
];

const CBSEM_EVIDENCE: readonly SemCapabilityEvidenceV1[] = [
  {
    evidence_id: "capability_registry_v2:smartpls.cbsem:qpls3.cbsem.general_sem_ml:cbsem_general_sem_ml_v1",
    description: "Capability Registry V2 is the exact authority for the bounded General SEM CB-SEM ML cell.",
  },
  {
    evidence_id: "compiler:compiled_cbsem_plan_v3",
    description: "CB-SEM v3 preserves the complete v2 parameter table and binds recursive topology, identification evidence, data, recipe, and capability identities.",
  },
];

const CBSEM_BOOTSTRAP_EVIDENCE: SemCapabilityEvidenceV1 = {
  evidence_id: "capability_registry_v2:smartpls.cbsem_bootstrapping:qpls3.cbsem.bootstrap.recursive_sem:cbsem_exact_recursive_sem_case_bootstrap_v1",
  description: "Capability Registry V2 is the exact authority for bounded recursive-SEM full-refit percentile case bootstrapping.",
};

type StructuralRelation = Extract<SemRelationV4, { kind: "structural" }>;

interface StructuralScc {
  readonly nodeIds: readonly string[];
  readonly relationIds: readonly string[];
  readonly hasFeedback: boolean;
}

interface SpecificDirectedPath {
  readonly source: string;
  readonly target: string;
  readonly relationIds: readonly string[];
}

interface SpecificPathEnumeration {
  readonly paths: readonly SpecificDirectedPath[];
  readonly limitExceeded: boolean;
}

function errorDiagnostic(
  code: SemCapabilityDiagnosticV1["code"],
  message: string,
  correction: string,
  subject: string | null = null,
): SemCapabilityDiagnosticV1 {
  return { code, severity: "error", subject, message, corrections: [correction] };
}

function structuralNodes(model: SemModelV4): string[] {
  return model.variables
    .filter((variable) => variable.kind !== "observed" || variable.role !== "indicator")
    .map((variable) => variable.id)
    .sort(compareUtf8StringsV1);
}

function structuralRelations(model: SemModelV4): StructuralRelation[] {
  return model.relations
    .filter((relation): relation is StructuralRelation => relation.kind === "structural")
    .slice()
    .sort((left, right) => compareUtf8StringsV1(left.id, right.id));
}

function compareStringArrays(left: readonly string[], right: readonly string[]): number {
  const sharedLength = Math.min(left.length, right.length);
  for (let index = 0; index < sharedLength; index += 1) {
    const compared = compareUtf8StringsV1(left[index]!, right[index]!);
    if (compared !== 0) return compared;
  }
  return left.length - right.length;
}

/** Kosaraju SCC partition with UTF-8 ordering, independent of authored array order. */
function structuralSccs(model: SemModelV4): readonly StructuralScc[] {
  const nodes = structuralNodes(model);
  const nodeSet = new Set(nodes);
  const relations = structuralRelations(model)
    .filter((relation) => nodeSet.has(relation.source) && nodeSet.has(relation.target));
  const outgoing = new Map(nodes.map((node) => [node, [] as string[]]));
  const incoming = new Map(nodes.map((node) => [node, [] as string[]]));

  for (const relation of relations) {
    outgoing.get(relation.source)!.push(relation.target);
    incoming.get(relation.target)!.push(relation.source);
  }
  for (const adjacent of [...outgoing.values(), ...incoming.values()]) {
    adjacent.sort(compareUtf8StringsV1);
    for (let index = adjacent.length - 1; index > 0; index -= 1) {
      if (adjacent[index] === adjacent[index - 1]) adjacent.splice(index, 1);
    }
  }

  const visited = new Set<string>();
  const finishOrder: string[] = [];
  const visitForFinish = (node: string): void => {
    if (visited.has(node)) return;
    visited.add(node);
    for (const target of outgoing.get(node) ?? []) visitForFinish(target);
    finishOrder.push(node);
  };
  for (const node of nodes) visitForFinish(node);

  visited.clear();
  const components: StructuralScc[] = [];
  const collect = (node: string, component: string[]): void => {
    if (visited.has(node)) return;
    visited.add(node);
    component.push(node);
    for (const source of incoming.get(node) ?? []) collect(source, component);
  };
  for (const node of finishOrder.slice().reverse()) {
    if (visited.has(node)) continue;
    const nodeIds: string[] = [];
    collect(node, nodeIds);
    nodeIds.sort(compareUtf8StringsV1);
    const componentNodes = new Set(nodeIds);
    const relationIds = relations
      .filter((relation) => componentNodes.has(relation.source) && componentNodes.has(relation.target))
      .map((relation) => relation.id);
    components.push({
      nodeIds,
      relationIds,
      hasFeedback: nodeIds.length > 1 || relationIds.length > 0,
    });
  }
  return components.sort((left, right) => compareStringArrays(left.nodeIds, right.nodeIds));
}

function enumerateSpecificDirectedPaths(
  model: SemModelV4,
  maxPaths: number,
): SpecificPathEnumeration {
  const nodes = structuralNodes(model);
  const nodeSet = new Set(nodes);
  const relations = structuralRelations(model)
    .filter((relation) => (
      (relation.role ?? "structural") === "structural"
      && nodeSet.has(relation.source)
      && nodeSet.has(relation.target)
    ));
  const outgoing = new Map(nodes.map((node) => [node, [] as StructuralRelation[]]));
  for (const relation of relations) outgoing.get(relation.source)!.push(relation);
  for (const adjacent of outgoing.values()) {
    adjacent.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  }

  const paths: SpecificDirectedPath[] = [];
  let limitExceeded = false;
  const visit = (
    source: string,
    current: string,
    visited: Set<string>,
    relationIds: string[],
  ): void => {
    if (limitExceeded) return;
    for (const relation of outgoing.get(current) ?? []) {
      if (visited.has(relation.target)) continue;
      visited.add(relation.target);
      relationIds.push(relation.id);
      if (relationIds.length >= 2) {
        if (paths.length >= maxPaths) {
          limitExceeded = true;
        } else {
          paths.push({ source, target: relation.target, relationIds: relationIds.slice() });
        }
      }
      if (!limitExceeded) visit(source, relation.target, visited, relationIds);
      relationIds.pop();
      visited.delete(relation.target);
      if (limitExceeded) return;
    }
  };

  for (const source of nodes) {
    visit(source, source, new Set([source]), []);
    if (limitExceeded) break;
  }
  paths.sort((left, right) => (
    compareStringArrays(left.relationIds, right.relationIds)
    || compareUtf8StringsV1(left.source, right.source)
    || compareUtf8StringsV1(left.target, right.target)
  ));
  return { paths, limitExceeded };
}

function sameRelationPath(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((relationId, index) => relationId === right[index]);
}

/** Mirrors qpls-core specific_directed_path_identity_v1 byte-for-byte. */
export function specificDirectedPathIdentityV1(relationIds: readonly string[]): string {
  const encoder = new TextEncoder();
  const domain = encoder.encode("qpls.compiled-sem-topology-v1.specific-directed-path\0");
  const encodedIds = relationIds.map((relationId) => encoder.encode(relationId));
  const totalLength = domain.length + encodedIds.reduce((total, bytes) => total + 8 + bytes.length, 0);
  const identityInput = new Uint8Array(totalLength);
  identityInput.set(domain);
  let offset = domain.length;
  for (const bytes of encodedIds) {
    const length = BigInt(bytes.length);
    const lengthView = new DataView(identityInput.buffer, offset, 8);
    lengthView.setUint32(0, Number(length >> 32n), false);
    lengthView.setUint32(4, Number(length & 0xffff_ffffn), false);
    offset += 8;
    identityInput.set(bytes, offset);
    offset += bytes.length;
  }
  return `sem_specific_path_v1_${sha256HexBytesV1(identityInput)}`;
}

function executionScopeDiagnostics(
  config: GeneralSemConfigV1,
  hasInteractions: boolean,
): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (config.conditional_effect_probes.length > 0) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.conditional_probes_not_executable",
      hasInteractions
        ? "Authored probe policies are preserved, but the moderation point cell uses the frozen standardized -1/0/+1 policy and the supplemental bootstrap cell is gamma-only."
        : "Conditional-effect probes are authored but are not executable in the current PLS v3 point-estimation slice.",
      "Remove the probe request for point estimation, or wait for the qualified moderation execution cell.",
    ));
  }
  if (config.inference.kind === "case_bootstrap") {
    if (config.inference.interval !== "percentile") {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.general_bootstrap_bca_not_executable",
        "BCa intervals are represented in the General SEM contract but are not qualified for this execution slice.",
        "Choose percentile intervals with two-sided inference, or set inference to none until the full General SEM delete-one effect ledger is qualified.",
      ));
    }
    if (config.inference.tail !== "two_sided") {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.general_bootstrap_one_sided_not_executable",
        "One-sided General SEM bootstrap intervals are represented but their interval semantics are not yet qualified.",
        "Choose two-sided inference, or set inference to none until the directional interval contract is qualified.",
      ));
    }
  }
  if (config.output_policy.lazy_specific_path_materialization
    || config.output_policy.when_specific_path_limit_exceeded === "return_lazy") {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.lazy_path_materialization_not_executable",
      "Lazy specific-path materialization is requested but is not implemented by the current executor.",
      "Use bounded eager materialization with an explicit path limit, or reduce the model before calculation.",
    ));
  }
  return diagnostics;
}

function plsShapeDiagnostics(
  model: SemModelV4,
  hasInteractions: boolean,
  hasHigherOrder: boolean,
): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (hasInteractions) {
    const threeWayTerms = model.derived_terms.filter((term) => (
      term.kind === "interaction_v2" && term.operands.length === 3
    ));
    if (threeWayTerms.length > 1) diagnostics.push(errorDiagnostic(
      "sem.capability.pls.multiple_three_way_interactions_not_executable",
      `The bounded three-way cell accepts one three-way term; received ${threeWayTerms.length}.`,
      "Keep one qualified three-way moderating effect in this calculation.",
    ));
    for (const term of model.derived_terms) {
      if (term.kind === "interaction_v2"
        && term.operands.length !== 2
        && term.operands.length !== 3) {
        diagnostics.push(errorDiagnostic(
          "sem.capability.pls.interaction_order_not_executable",
          `Interaction ${term.id} requires two or three operands; received ${term.operands.length}.`,
          "Use a qualified two-way or bounded three-way interaction; fourth-order interactions remain unsupported.",
          term.id,
        ));
      }
    }
  }
  if (model.variables.some((variable) => variable.kind === "common_factor")) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.common_factor_not_executable",
      "The current General SEM PLS executor requires structural constructs to be composites; the authored model contains common factors.",
      "Use composite constructs for this PLS request, or retain the factor model for a qualified CB-SEM capability cell.",
    ));
  }
  if (!hasInteractions && !hasHigherOrder && (
    model.variables.some((variable) => variable.kind === "derived")
    || model.derived_terms.length > 0
    || model.parameters.some((parameter) => parameter.kind === "derived")
  )) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.derived_shape_not_executable",
      "The authored model contains derived variables, terms, or parameters that the current PLS v3 executor does not calculate.",
      "Keep the derived semantics on the canvas, but remove them from this calculation or wait for their exact qualified execution cell.",
    ));
  }
  return diagnostics;
}

interface CompiledInteractionProjectionV1 {
  readonly projectedModel: SemModelV4;
  readonly outputIds: ReadonlySet<string>;
}

function compileHocLowerOrderProjectionV1(model: SemModelV4, outputId: string): SemModelV4 {
  const removedRelations = model.relations.filter((relation) => relationReferencesVariable(relation, outputId));
  const removedRelationIds = new Set(removedRelations.map((relation) => relation.id));
  const removedParameterIds = new Set(removedRelations.flatMap((relation) => (
    relation.kind === "structural" && relation.intercept_parameter
      ? [relation.parameter, relation.intercept_parameter]
      : [relation.parameter]
  )));
  return canonicalizeSemModelV4({
    ...structuredClone(model),
    variables: model.variables.filter((variable) => variable.id !== outputId),
    relations: model.relations.filter((relation) => !removedRelationIds.has(relation.id)),
    parameters: model.parameters.filter((parameter) => !removedParameterIds.has(parameter.id)),
    derived_terms: [],
    annotations: [],
    presentation: { kind: "none" },
  });
}

function constraintReferencesParameter(model: SemModelV4, parameterId: string): boolean {
  return model.constraints.some((constraint) => constraint.kind === "equality"
    ? constraint.parameters.includes(parameterId)
    : constraint.kind === "bound"
      ? constraint.parameter === parameterId
      : constraint.terms.some((term) => term.parameter === parameterId));
}

function relationReferencesVariable(relation: SemRelationV4, variableId: string): boolean {
  if (relation.kind === "measurement_effect") {
    return relation.construct === variableId || relation.indicator === variableId;
  }
  if (relation.kind === "measurement_causal") {
    return relation.composite === variableId || relation.indicator === variableId;
  }
  if (relation.kind === "structural") {
    return relation.source === variableId || relation.target === variableId;
  }
  return relation.left.id === variableId || relation.right.id === variableId;
}

export function interactionProductColumnIdentityV1(input: {
  interactionId: string;
  outputId: string;
  operands: readonly [string, string];
  focalRelationId: string;
  effectRelationId: string;
}): string {
  const encoder = new TextEncoder();
  const domain = encoder.encode("qpls.compiled-pls-plan-v3.two-stage-product\0");
  const values = [
    input.interactionId,
    input.outputId,
    input.operands[0],
    input.operands[1],
    input.focalRelationId,
    input.effectRelationId,
  ].map((value) => encoder.encode(value));
  const bytes = new Uint8Array(domain.length + values.reduce((total, value) => total + 8 + value.length, 0));
  bytes.set(domain);
  let offset = domain.length;
  for (const value of values) {
    const length = BigInt(value.length);
    const view = new DataView(bytes.buffer, offset, 8);
    view.setUint32(0, Number(length >> 32n), false);
    view.setUint32(4, Number(length & 0xffff_ffffn), false);
    offset += 8;
    bytes.set(value, offset);
    offset += value.length;
  }
  return `qpls_pls_product_v1_${sha256HexBytesV1(bytes)}`;
}

function threeWayInteractionProductColumnIdentityV1(input: {
  interactionId: string;
  outputId: string;
  operands: readonly [string, string, string];
  focalRelationId: string;
  effectRelationId: string;
}): string {
  const encoded = new TextEncoder().encode([
    "qpls.compiled-pls-plan-v3.three-way-product-v1",
    input.interactionId,
    input.outputId,
    ...input.operands,
    input.focalRelationId,
    input.effectRelationId,
  ].join("\0"));
  return `qpls_pls_three_way_product_v1_${sha256HexBytesV1(encoded)}`;
}

function exactBinaryZeroOneCategoriesV1(categories: readonly string[]): boolean {
  if (categories.length !== 2) return false;
  const values = categories.map((category) => {
    const normalized = category.trim();
    if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(normalized)) return null;
    const value = Number(normalized);
    return Number.isFinite(value) ? value : null;
  });
  if (values.some((value) => value == null)) return false;
  const sorted = (values as number[]).sort((left, right) => left - right);
  return Object.is(sorted[0], 0) && Object.is(sorted[1], 1);
}

function qualifiedThreeWayModeratorObservedIdsV1(model: SemModelV4): ReadonlySet<string> {
  const terms = model.derived_terms.filter((term): term is Extract<typeof term, { kind: "interaction_v2" }> => (
    term.kind === "interaction_v2" && term.operands.length === 3
  ));
  if (terms.length !== 1) return new Set();
  const term = terms[0]!;
  if (term.method !== "two_stage"
    || term.hierarchy_policy !== "strong"
    || term.product_indicator != null) return new Set();
  const focal = model.relations.find((relation): relation is StructuralRelation => (
    relation.kind === "structural"
    && (relation.role ?? "structural") === "structural"
    && relation.id === term.focal_relation
    && relation.source === term.operands[0]
  ));
  if (!focal) return new Set();
  if (!model.relations.some((relation) => relation.kind === "structural"
    && (relation.role ?? "structural") === "structural"
    && relation.source === term.output
    && relation.target === focal.target)) return new Set();
  if (!term.operands.every((operand) => model.relations.some((relation) => (
    relation.kind === "structural"
    && (relation.role ?? "structural") === "structural"
    && relation.source === operand
    && relation.target === focal.target
  )))) return new Set();
  const pairs = [[0, 1], [0, 2], [1, 2]] as const;
  if (!pairs.every(([left, right]) => model.derived_terms.some((candidate) => (
    candidate.kind === "interaction_v2"
    && candidate.operands.length === 2
    && candidate.method === "two_stage"
    && candidate.hierarchy_policy === "strong"
    && candidate.product_indicator == null
    && candidate.operands.includes(term.operands[left]!)
    && candidate.operands.includes(term.operands[right]!)
  )))) return new Set();

  return new Set(term.operands.slice(1).flatMap((moderatorId) => {
    const moderator = model.variables.find((variable) => variable.id === moderatorId);
    if (moderator?.kind === "observed") return [moderator.id];
    const indicators = model.relations.flatMap((relation) => {
      if (relation.kind === "measurement_effect" && relation.construct === moderatorId) return [relation.indicator];
      if (relation.kind === "measurement_causal" && relation.composite === moderatorId) return [relation.indicator];
      return [];
    });
    return indicators.length === 1 ? [indicators[0]!] : [];
  }));
}

function compileInteractionProjectionV1(
  model: SemModelV4,
): { value: CompiledInteractionProjectionV1 | null; diagnostics: SemCapabilityDiagnosticV1[] } {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  const interactionTerms = model.derived_terms
    .filter((term): term is Extract<typeof term, { kind: "interaction_v2" }> => term.kind === "interaction_v2")
    .slice()
    .sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  const unsupported = model.derived_terms.find((term) => term.kind !== "interaction_v2");
  if (unsupported) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.interaction_shape_not_executable",
      `PLS v3 interaction point estimation does not support derived term ${unsupported.id} (${unsupported.kind}).`,
      "Keep only qualified interaction_v2 terms in this exact moderation point request; other derived semantics remain saved for future cells.",
      unsupported.id,
    ));
    return { value: null, diagnostics };
  }

  const outputIds = new Set(interactionTerms.map((term) => term.output));
  const extraDerivedVariable = model.variables.find((variable) => (
    variable.kind === "derived" && !outputIds.has(variable.id)
  ));
  const derivedParameter = model.parameters.find((parameter) => parameter.kind === "derived");
  if (extraDerivedVariable || derivedParameter) {
    const subject = extraDerivedVariable?.id ?? derivedParameter!.id;
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.interaction_shape_not_executable",
      "The moderation point graph contains derived scientific objects outside its compiled interaction_v2 outputs.",
      "Keep only the interaction_v2 outputs and their exact free effect paths in this point request.",
      subject,
    ));
    return { value: null, diagnostics };
  }

  const effectRelationIds = new Set<string>();
  const effectParameterIds = new Set<string>();
  const productDesigns = new Map<string, string>();
  const generatedColumns = new Set<string>();
  const sourceColumns = new Set(model.variables.flatMap((variable) => (
    variable.kind === "observed" ? [variable.source_column] : []
  )));

  for (const term of interactionTerms) {
    if (term.operands.length !== 2 && term.operands.length !== 3) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_order_not_executable",
        `Interaction ${term.id} requires two or three operands; received ${term.operands.length}.`,
        "Use a qualified two-way or bounded three-way interaction; fourth-order interactions remain unsupported.",
        term.id,
      ));
      continue;
    }
    if (term.method !== "two_stage") {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_method_not_executable",
        `Interaction ${term.id} requires the two-stage construction method.`,
        "Choose the two-stage interaction construction method for this bounded point cell.",
        term.id,
      ));
      continue;
    }
    if (term.hierarchy_policy !== "strong") {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_hierarchy_not_executable",
        `Interaction ${term.id} requires strong hierarchy.`,
        "Use strong hierarchy and retain every required lower-order path.",
        term.id,
      ));
      continue;
    }
    if (term.product_indicator != null) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Interaction ${term.id} carries product-indicator settings outside the fixed two-stage point cell.`,
        "Remove product-indicator settings and use the fixed two-stage construction policy.",
        term.id,
      ));
      continue;
    }
    const focal = model.relations.find((relation): relation is StructuralRelation => (
      relation.kind === "structural"
      && (relation.role ?? "structural") === "structural"
      && relation.id === term.focal_relation
    ));
    if (!focal || focal.source !== term.operands[0]) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Interaction ${term.id} does not bind operands[0] to an exact focal structural path.`,
        "Retarget the interaction to the exact authored focal path and preserve focal-predictor order.",
        term.id,
      ));
      continue;
    }
    const effectRelations = model.relations.filter((relation): relation is StructuralRelation => (
      relation.kind === "structural"
      && (relation.role ?? "structural") === "structural"
      && relation.source === term.output
      && relation.target === focal.target
    ));
    if (effectRelations.length !== 1) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Interaction ${term.id} must have exactly one structural-effect path from ${term.output} to ${focal.target}; received ${effectRelations.length}.`,
        "Keep one exact interaction-effect path to the focal outcome.",
        term.id,
      ));
      continue;
    }
    const effectRelation = effectRelations[0]!;
    const unsupportedOutputRelation = model.relations.find((relation) => (
      relation.id !== effectRelation.id && relationReferencesVariable(relation, term.output)
    ));
    if (unsupportedOutputRelation) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Interaction output ${term.output} participates in unsupported relation ${unsupportedOutputRelation.id}.`,
        "Use the interaction output only as the source of its single effect path.",
        term.output,
      ));
      continue;
    }
    const parameter = model.parameters.find((candidate) => candidate.id === effectRelation.parameter);
    const parameterIsExact = parameter?.kind === "free"
      && parameter.target.kind === "regression"
      && parameter.target.source === term.output
      && parameter.target.target === focal.target
      && parameter.start == null
      && parameter.lower == null
      && parameter.upper == null
      && parameter.equality_label == null
      && (parameter.group_overrides?.length ?? 0) === 0
      && effectRelation.intercept_parameter == null;
    if (!parameterIsExact || constraintReferencesParameter(model, effectRelation.parameter)) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Interaction ${term.id} effect parameter ${effectRelation.parameter} must be an unconstrained free regression parameter.`,
        "Remove starts, bounds, equality labels, group overrides, intercepts, and constraints from the interaction-effect parameter.",
        effectRelation.parameter,
      ));
      continue;
    }

    const sortedOperands = [...term.operands].sort(compareUtf8StringsV1);
    const productDesign = `${sortedOperands.join("\0")}\0${focal.target}`;
    const duplicate = productDesigns.get(productDesign);
    if (duplicate) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.duplicate_interaction_product_design",
        `Interactions ${duplicate} and ${term.id} compile to the same fixed product design for outcome ${focal.target}.`,
        "Keep one product per operand set and outcome; operand order still defines focal and moderator roles.",
        term.id,
      ));
      continue;
    }
    productDesigns.set(productDesign, term.id);
    const generatedColumn = term.operands.length === 3
      ? threeWayInteractionProductColumnIdentityV1({
        interactionId: term.id,
        outputId: term.output,
        operands: [term.operands[0]!, term.operands[1]!, term.operands[2]!],
        focalRelationId: term.focal_relation,
        effectRelationId: effectRelation.id,
      })
      : interactionProductColumnIdentityV1({
        interactionId: term.id,
        outputId: term.output,
        operands: [term.operands[0]!, term.operands[1]!],
        focalRelationId: term.focal_relation,
        effectRelationId: effectRelation.id,
      });
    if (sourceColumns.has(generatedColumn) || generatedColumns.has(generatedColumn)) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.interaction_shape_not_executable",
        `Generated interaction product column id collides: ${generatedColumn}.`,
        "Rename the colliding source column or interaction identity before calculation.",
        term.id,
      ));
      continue;
    }
    generatedColumns.add(generatedColumn);
    effectRelationIds.add(effectRelation.id);
    effectParameterIds.add(effectRelation.parameter);
  }

  if (diagnostics.length > 0) return { value: null, diagnostics };
  const qualifiedThreeWayModeratorObservedIds = qualifiedThreeWayModeratorObservedIdsV1(model);
  const projectedModel = canonicalizeSemModelV4({
    ...structuredClone(model),
    variables: model.variables
      .filter((variable) => !outputIds.has(variable.id))
      .map((variable) => variable.kind === "observed"
        && variable.scale === "binary"
        && qualifiedThreeWayModeratorObservedIds.has(variable.id)
        && exactBinaryZeroOneCategoriesV1(variable.categories)
        ? { ...variable, scale: "continuous" as const, categories: [], value_labels: {} }
        : variable),
    relations: model.relations.filter((relation) => !effectRelationIds.has(relation.id)),
    parameters: model.parameters.filter((parameter) => !effectParameterIds.has(parameter.id)),
    derived_terms: [],
    annotations: [],
    presentation: { kind: "none" },
  });
  return { value: { projectedModel, outputIds }, diagnostics };
}

function exactTwoWayModeratedMediationRequestV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
  paths: readonly SpecificDirectedPath[],
): boolean {
  if (config.inference.kind !== "case_bootstrap"
    || config.inference.interval !== "percentile"
    || config.inference.tail !== "two_sided"
    || config.conditional_effect_probes.length !== 0
    || config.requested_effect_estimands.length !== 1
    || model.derived_terms.length !== 1) return false;
  const request = config.requested_effect_estimands[0];
  const interaction = model.derived_terms[0];
  if (request?.kind !== "specific_path"
    || request.ordered_relation_ids.length !== 2
    || interaction?.kind !== "interaction_v2"
    || interaction.operands.length !== 2
    || interaction.method !== "two_stage"
    || interaction.hierarchy_policy !== "strong"
    || interaction.product_indicator != null) return false;
  const path = paths.find((candidate) => sameRelationPath(
    candidate.relationIds,
    request.ordered_relation_ids,
  ));
  if (!path || request.estimand_id !== specificDirectedPathIdentityV1(path.relationIds)
    || !path.relationIds.includes(interaction.focal_relation)) return false;
  const relations = structuralRelations(model);
  const first = relations.find((relation) => relation.id === path.relationIds[0]);
  const second = relations.find((relation) => relation.id === path.relationIds[1]);
  if (!first || !second || first.target !== second.source) return false;
  const moderatorId = interaction.operands[1];
  if (new Set([first.source, first.target, second.target, moderatorId]).size !== 4) return false;
  const registryMatches = capabilityRegistryV2.quickPlsCell(
    GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.cell_id,
  ).filter(({ row }) => (
    row.capability_id === GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.capability_id
  ));
  if (registryMatches.length !== 1) return false;
  const registryCell = registryMatches[0]!.cell;
  const availability = capabilityRegistryV2.availability(
    registryCell.capability_id,
    registryCell.cell_id,
    true,
  );
  return registryCell.capability_version
    === GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.capability_version
    && (registryCell.surface === "standard" || registryCell.surface === "labs")
    && availability.selectable;
}

function interactionScopeDiagnostics(
  model: SemModelV4,
  config: GeneralSemConfigV1,
  paths: readonly SpecificDirectedPath[],
): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  if (exactTwoWayModeratedMediationRequestV1(model, config, paths)) return diagnostics;
  if (config.requested_effect_estimands.length > 0) diagnostics.push(errorDiagnostic(
    "sem.capability.pls.multiple_moderation_effect_requests_not_executable",
    "Mediation-effect requests cannot be combined with the simultaneous interaction_v2 point or gamma-only bootstrap cells.",
    "Clear requested indirect/total effects and calculate moderation point estimates only, or retain the model until the combined estimand cell is qualified.",
  ));
  if (paths.length > 0) diagnostics.push(errorDiagnostic(
    "sem.capability.pls.moderated_mediation_not_executable",
    "A directed chain is present, so this graph may imply moderated mediation outside the bounded direct-only moderation cells.",
    "Use a direct-only structural graph for these cells, or retain the authored chain until moderated-mediation execution is qualified.",
  ));
  return diagnostics;
}

function requestedEffectDiagnostics(
  model: SemModelV4,
  config: GeneralSemConfigV1,
  paths: readonly SpecificDirectedPath[],
): SemCapabilityDiagnosticV1[] {
  const relations = structuralRelations(model)
    .filter((relation) => (relation.role ?? "structural") === "structural");
  const reservedSpecificPathIdentities = new Set(
    paths.map((path) => specificDirectedPathIdentityV1(path.relationIds)),
  );
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  for (const estimand of config.requested_effect_estimands) {
    if (estimand.kind === "specific_path") {
      if (!paths.some((path) => sameRelationPath(path.relationIds, estimand.ordered_relation_ids))) {
        diagnostics.push(errorDiagnostic(
          "sem.capability.pls.requested_path_missing",
          "The requested specific indirect effect is not an exact directed relation path in the current model.",
          "Re-open the mediation inspector and select an exact directed relation path that exists in the current model.",
          estimand.estimand_id,
        ));
      }
      continue;
    }
    if (reservedSpecificPathIdentities.has(estimand.estimand_id)) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.effect_identity_collision",
        "The aggregate effect request reuses a reserved canonical specific-path identity.",
        "Choose an aggregate estimand id that does not use an existing sem_specific_path_v1 identity.",
        estimand.estimand_id,
      ));
      continue;
    }
    const indirectReachable = paths.some((path) => (
      path.source === estimand.source_id && path.target === estimand.target_id
    ));
    const directReachable = estimand.kind === "total_effect" && relations.some((relation) => (
      relation.source === estimand.source_id && relation.target === estimand.target_id
    ));
    if (!indirectReachable && !directReachable) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.requested_effect_unreachable",
        `The requested ${estimand.kind === "total_effect" ? "total" : "total indirect"} effect has no eligible directed path between its endpoints.`,
        "Choose endpoints connected by a supported directed path, or add the intended structural relation on the canvas.",
        estimand.estimand_id,
      ));
    }
  }
  return diagnostics;
}

function plsDataScopeDiagnostics(model: SemModelV4): SemCapabilityDiagnosticV1[] {
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  const threeWayModeratorObservedIds = qualifiedThreeWayModeratorObservedIdsV1(model);
  if (model.data_binding.kind !== "raw") {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.raw_data_required",
      "The exact General SEM PLS cell requires raw case-level data.",
      "Choose a raw resident dataset, or use a qualified matrix-input CB-SEM cell.",
      model.id,
    ));
  } else {
    if (model.data_binding.missing_data !== "listwise_deletion") diagnostics.push(errorDiagnostic(
      "sem.capability.pls.listwise_deletion_required",
      "The exact General SEM PLS cell requires listwise deletion; the authored missing-data policy is preserved but unsupported.",
      "Select listwise deletion explicitly for this PLS request.",
      model.id,
    ));
    if (model.data_binding.weight !== null
      || model.data_binding.cluster_variable !== null
      || model.data_binding.strata_variable !== null) diagnostics.push(errorDiagnostic(
      "sem.capability.pls.complex_sampling_not_executable",
      "Weights, cluster variables, and strata variables are not executable in this exact General SEM PLS cell.",
      "Use an unweighted single-level request, or retain these semantics for a future qualified cell.",
      model.id,
    ));
  }
  if (model.group.kind !== "single_group") diagnostics.push(errorDiagnostic(
    "sem.capability.pls.single_group_required",
    "The exact General SEM PLS cell currently executes single-group models only.",
    "Select the single-group definition, or retain the group semantics for a future qualified multi-group cell.",
    model.id,
  ));
  for (const variable of model.variables) {
    if (variable.kind !== "observed") continue;
    const boundedBinaryModerator = variable.scale === "binary"
      && threeWayModeratorObservedIds.has(variable.id)
      && exactBinaryZeroOneCategoriesV1(variable.categories);
    if ((variable.scale !== "continuous" && !boundedBinaryModerator)
      || variable.missing_markers.length > 0
      || variable.transformation_lineage.length > 0) diagnostics.push(errorDiagnostic(
      "sem.capability.pls.observed_semantics_not_executable",
      "This observed variable carries scale, missing-marker, or transformation semantics outside the exact General SEM PLS cell. Binary 0/1 coding is admitted only for a bounded three-way moderator.",
      "Keep the authored semantics unchanged and use an explicit, lineage-recorded dataset transformation or a future qualified cell.",
      variable.id,
    ));
  }
  return diagnostics;
}

/**
 * Exact preflight for the General SEM PLS point-estimation and bounded
 * percentile case-bootstrap compiler slices. The inputs are never rewritten.
 */
export function preflightGeneralSemPlsV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
): SemCapabilityDecisionV1 {
  const validatedConfig = parseGeneralSemConfigV1(config);
  const interactionTerms = model.derived_terms.filter((term) => term.kind === "interaction_v2");
  const hasInteractions = interactionTerms.length > 0;
  const hasThreeWayInteraction = interactionTerms.some((term) => term.operands.length === 3);
  const hasHigherOrder = model.derived_terms.some((term) => term.kind === "higher_order");
  const bootstrapRequested = validatedConfig.inference.kind === "case_bootstrap";
  const initialIndirectPathCount = !hasInteractions && !hasHigherOrder
    ? enumerateSpecificDirectedPaths(
      model,
      validatedConfig.output_policy.max_materialized_specific_paths,
    ).paths.length
    : 0;
  const requestsModeratedMediation = hasInteractions
    && bootstrapRequested
    && validatedConfig.requested_effect_estimands.length > 0;
  const hocContract = preflightGeneralSemHocContractV1(model, bootstrapRequested);
  const capabilityCells = hasHigherOrder
    ? hocContract.capabilityCells
    : [
      hasInteractions
        ? hasThreeWayInteraction
          ? GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CELL_V1
          : GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1
        : PLS_CELL,
      ...(bootstrapRequested
        ? [hasInteractions
          ? hasThreeWayInteraction
            ? GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CELL_V1
            : requestsModeratedMediation
              ? GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1
              : GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CELL_V1
          : initialIndirectPathCount === 1
            ? GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CELL_V1
            : PLS_BOOTSTRAP_CELL]
        : []),
    ];
  const evidence = [
    PLS_EVIDENCE.find((item) => item.evidence_id === "compiler:recipe_v4_to_compiled_pls_plan_v3_v1")!,
    ...(hasHigherOrder ? hocContract.evidence : hasInteractions
      ? hasThreeWayInteraction
        ? PLS_THREE_WAY_MODERATION_EVIDENCE
        : PLS_MULTIPLE_MODERATION_EVIDENCE
      : PLS_EVIDENCE.filter((item) => (
      item.evidence_id !== "compiler:recipe_v4_to_compiled_pls_plan_v3_v1"
    ))),
    ...(bootstrapRequested ? [
      ...(hasHigherOrder
        ? []
        : hasInteractions
          ? hasThreeWayInteraction
            ? PLS_THREE_WAY_MODERATION_BOOTSTRAP_EVIDENCE
            : requestsModeratedMediation
              ? PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_EVIDENCE
              : PLS_MULTIPLE_MODERATION_BOOTSTRAP_EVIDENCE
        : [
          PLS_BOOTSTRAP_COMPILER_EVIDENCE,
          initialIndirectPathCount === 1
            ? PLS_SINGLE_MEDIATION_BOOTSTRAP_EVIDENCE
            : PLS_BOOTSTRAP_EVIDENCE,
        ]),
      PLS_BOOTSTRAP_MECHANISM_EVIDENCE,
    ] : []),
  ];
  const diagnostics = executionScopeDiagnostics(validatedConfig, hasInteractions);
  diagnostics.push(...plsShapeDiagnostics(model, hasInteractions, hasHigherOrder));
  if (hasHigherOrder) diagnostics.push(...hocContract.diagnostics);
  diagnostics.push(...plsDataScopeDiagnostics(model));

  let modelIsValid = false;
  try {
    const modelIssues = validateSemModelV4(model);
    modelIsValid = modelIssues.length === 0;
  } catch {
    modelIsValid = false;
  }
  if (!modelIsValid) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.model_invalid",
      "The authored SEM model does not satisfy the SemModelV4 scientific integrity contract.",
      "Resolve the model-integrity diagnostics before selecting an estimator capability.",
    ));
  }

  const hasFeedback = structuralSccs(model).some((component) => component.hasFeedback);
  if (hasFeedback) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.pls.feedback_blocked",
      "The structural graph contains a reciprocal block, while the current PLS v3 executor requires a recursive DAG.",
      "Remove the reciprocal path for PLS-SEM, or use a future qualified nonrecursive CB-SEM cell.",
    ));
  }

  const registryStatus = exactRegistryDecisionStatusV1(capabilityCells);
  if (!registryStatus) diagnostics.push(errorDiagnostic(
    "sem.capability.pls.registry_unavailable",
    "The exact General SEM PLS capability set is not currently selectable in Capability Registry V2.",
    "Keep the model unchanged and restore the exact Registry cells before calculation.",
    "capability_registry_v2",
  ));

  let basePlanCompiles = false;
  let interactionOutputIds: ReadonlySet<string> = new Set();
  if (modelIsValid && !hasFeedback && diagnostics.every((item) => (
    item.code !== "sem.capability.pls.common_factor_not_executable"
    && item.code !== "sem.capability.pls.derived_shape_not_executable"
  ))) {
    try {
      if (hasHigherOrder) {
        if (hocContract.contractCompiles && hocContract.outputId) {
          compilePlsPlanV2(compileHocLowerOrderProjectionV1(model, hocContract.outputId));
          basePlanCompiles = true;
        }
      } else if (hasInteractions) {
        const compiled = compileInteractionProjectionV1(model);
        diagnostics.push(...compiled.diagnostics);
        if (compiled.value) {
          compilePlsPlanV2(compiled.value.projectedModel);
          interactionOutputIds = compiled.value.outputIds;
          basePlanCompiles = true;
        }
      } else {
        compilePlsPlanV2(model);
        basePlanCompiles = true;
      }
    } catch (error) {
      const reason = error instanceof SemModelV4OperationError ? error.code : "unknown_compile_failure";
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.model_shape_not_executable",
        `The current PLS v3 base executor cannot compile this authored model shape (${reason}).`,
        "Review construct types, measurement directions, groups, constraints, data binding, and generated terms in the estimator compatibility inspector.",
      ));
    }
  }

  if (basePlanCompiles) {
    const enumeration = enumerateSpecificDirectedPaths(
      model,
      validatedConfig.output_policy.max_materialized_specific_paths,
    );
    if (enumeration.limitExceeded) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.pls.specific_path_limit_exceeded",
        "Specific directed-path enumeration exceeds the explicit eager materialization limit; no truncated result will be returned.",
        "Increase the explicit path limit within available resources, or simplify the structural graph before calculation.",
      ));
    } else {
      const eligiblePaths = enumeration.paths.filter((path) => !interactionOutputIds.has(path.source));
      if (hasInteractions) {
        diagnostics.push(...interactionScopeDiagnostics(model, validatedConfig, eligiblePaths));
        diagnostics.push(...requestedEffectDiagnostics(model, validatedConfig, eligiblePaths));
      } else if (!hasHigherOrder && !bootstrapRequested && eligiblePaths.length === 0) {
        diagnostics.push(errorDiagnostic(
          "sem.capability.pls.mediation_requires_indirect_path",
          "The PLS mediation point cell requires at least one compiled specific indirect path; this graph has none.",
          "Add a supported mediator path, or use the existing ordinary PLS workflow for a direct-only recursive model.",
        ));
      } else if (!hasHigherOrder && bootstrapRequested && eligiblePaths.length === 0) {
        diagnostics.push(errorDiagnostic(
          "sem.capability.pls.mediation_bootstrap_requires_indirect_path",
          "The mediation bootstrap cells require at least one compiled specific indirect path; this graph has none.",
          "Add a supported mediator path, or use the ordinary PLS workflow for a direct-only recursive model.",
        ));
      }
      if (!hasInteractions) diagnostics.push(...requestedEffectDiagnostics(model, validatedConfig, eligiblePaths));
    }
  }

  if (diagnostics.length > 0) {
    return createSemCapabilityDecisionV1({
      status: "blocked",
      estimator_id: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
      capability_cells: capabilityCells,
      diagnostics,
      evidence,
      summary: "PLS-SEM cannot calculate this exact General SEM request yet.",
      explanation: "The authored model remains intact. Apply one of the listed corrections or select an estimator whose exact capability cell supports the graph.",
    });
  }

  const status = registryStatus ?? "experimental";
  const standard = status === "supported";
  return createSemCapabilityDecisionV1({
    status,
    estimator_id: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
    capability_cells: capabilityCells,
    diagnostics: [{
      code: standard ? "sem.capability.pls.standard" : "sem.capability.pls.experimental_labs",
      severity: "info",
      subject: null,
      message: hasHigherOrder
        ? bootstrapRequested
          ? "General SEM higher-order full-model percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
          : "General SEM higher-order point estimation passes the bounded exact-cell compiler preflight."
        : hasInteractions
        ? bootstrapRequested
          ? hasThreeWayInteraction
            ? "General SEM three-way full-model percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
            : requestsModeratedMediation
              ? "General SEM two-way moderated-mediation five-target percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
              : "General SEM simultaneous two-way moderation gamma-only percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
          : hasThreeWayInteraction
            ? "General SEM bounded three-way moderation point estimation passes the exact-cell compiler preflight."
            : "General SEM simultaneous two-way moderation point estimation passes the bounded exact-cell compiler preflight."
        : bootstrapRequested
          ? initialIndirectPathCount === 1
            ? "General single-mediation percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
            : "General multiple-mediation percentile case-bootstrap inference passes the bounded exact-cell compiler preflight."
          : "General recursive PLS point estimation and path-specific effects pass the bounded exact-cell compiler preflight.",
      corrections: [],
    }],
    evidence,
    summary: standard
      ? "PLS-SEM can compile this exact Standard Registry request."
      : "PLS-SEM can compile this exact Registry-governed request.",
    explanation: hasHigherOrder
      ? bootstrapRequested
        ? "The exact HOC point cell remains the primary artifact authority and the supplemental HOC cell authorizes indexed raw-case resampling with every compiled stage refitted before bounded target inference is published."
        : "The compiler binds the authored HOC to its approach-specific stages, generated identities, component loading-or-weight interpretation, authored paths, and canonical stage receipts."
      : hasInteractions
      ? bootstrapRequested
        ? hasThreeWayInteraction
          ? "The bounded three-way point authority and supplemental bootstrap cell jointly bind strong hierarchy, fixed moderator probes, complete-model refits, and one shared usable/failure ledger."
          : requestsModeratedMediation
            ? "The point moderation cell remains the primary artifact authority and the exact Registry-authorized supplemental cell adds scientific gamma, fixed -1/0/+1 conditional indirect effects, and the index of moderated mediation from one shared full-model replicate ledger. Runtime validation remains authoritative before publication."
            : "The point moderation cell remains the primary artifact authority and the supplemental exact cell authorizes percentile, two-sided full-model case-bootstrap inference for scientific rescaled gamma only. A runtime must retain indexed-resampling and complete-model re-estimation receipts before publication."
        : hasThreeWayInteraction
          ? "The compiler binds one three-way product, its strong-hierarchy lower-order terms, and the fixed two-dimensional simple-slope probe grid to one joint stage-two solve."
          : "The compiler binds the source model to one stage-one projection, a joint stage-two solve, explicit product-scale receipts, and fixed -1/0/+1 conditional-slope provenance. Runtime validation remains authoritative before publication."
      : bootstrapRequested
      ? `The compiler binds percentile, two-sided case resampling to the exact ${initialIndirectPathCount === 1 ? "single" : "multiple"}-mediation bootstrap cell and records the indexed-resampling mechanism as a dependency. Runtime inference must carry a matching complete-model re-estimation receipt before publication.`
      : "The compiler binds the proven PLS scoring plan to stable relation-path identities. Runtime validation remains authoritative before a result can be published.",
  });
}

/** Exact Registry-governed preview for the connected bounded CB-SEM v3 adapter. */
export function preflightGeneralSemCbsemV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
): SemCapabilityDecisionV1 {
  const validatedConfig = parseGeneralSemConfigV1(config);
  const diagnostics: SemCapabilityDiagnosticV1[] = [];
  const bootstrapRequested = validatedConfig.inference.kind === "case_bootstrap";
  const capabilityCells = [CBSEM_CELL, ...(bootstrapRequested ? [CBSEM_BOOTSTRAP_CELL] : [])];
  const evidence = [...CBSEM_EVIDENCE, ...(bootstrapRequested ? [CBSEM_BOOTSTRAP_EVIDENCE] : [])];
  const registryStatus = exactRegistryDecisionStatusV1(capabilityCells);
  if (registryStatus === null) diagnostics.push(errorDiagnostic(
    "sem.capability.cbsem.registry_unavailable",
    "Capability Registry V2 cannot authorize the exact General SEM CB-SEM request.",
    "Restore the exact point or recursive-bootstrap option cell before calculating.",
  ));
  let modelIsValid = false;
  try {
    modelIsValid = validateSemModelV4(model).length === 0;
  } catch {
    modelIsValid = false;
  }
  if (!modelIsValid) {
    diagnostics.push(errorDiagnostic(
      "sem.capability.cbsem.compile_blocked",
      "CB-SEM cannot compile this graph because the authored model does not satisfy the SemModelV4 scientific integrity contract.",
      "Open the estimator compatibility inspector and resolve the reported model or identification issue.",
    ));
  }

  let cbsemPlanCompiles = false;
  if (modelIsValid) {
    const enumeration = enumerateSpecificDirectedPaths(
      model,
      validatedConfig.output_policy.max_materialized_specific_paths,
    );
    if (enumeration.limitExceeded) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.cbsem.compile_blocked",
        "CB-SEM cannot compile this graph because specific directed-path enumeration exceeds the explicit resource limit.",
        "Increase the explicit path limit within available resources, or simplify the structural graph before calculation.",
      ));
    } else {
      try {
        compileCbsemPlanV2(model);
        cbsemPlanCompiles = true;
      } catch (error) {
        const reason = error instanceof SemModelV4OperationError ? error.code : "unknown_compile_failure";
        diagnostics.push(errorDiagnostic(
          "sem.capability.cbsem.compile_blocked",
          `CB-SEM cannot compile this exact graph (${reason}).`,
          "Open the estimator compatibility inspector and resolve the reported model or identification issue.",
        ));
      }
    }
  }

  if (cbsemPlanCompiles) {
    if (bootstrapRequested && !model.relations.some((relation) => relation.kind === "structural")) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.cbsem.recursive_sem_requires_regression",
        "General SEM CB-SEM recursive bootstrap requires at least one structural regression; point CFA remains supported by the point cell.",
        "Use point CB-SEM for CFA, route CFA bootstrap through the exact compatibility cell, or add the scientifically intended recursive structural relation.",
      ));
    }
    if (structuralSccs(model).some((component) => component.hasFeedback)) {
      diagnostics.push(errorDiagnostic(
        "sem.capability.cbsem.feedback_execution_blocked",
        "The reciprocal block is preserved, but the current CB-SEM executor is not qualified to estimate feedback systems.",
        "Remove the reciprocal path to create a recursive model, or retain the model until the identified feedback capability is qualified.",
      ));
    }
  }

  if (diagnostics.length === 0) {
    const status = registryStatus ?? "experimental";
    const standard = status === "supported";
    return createSemCapabilityDecisionV1({
      status,
      estimator_id: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      capability_cells: capabilityCells,
      diagnostics: [{
        code: standard ? "sem.capability.cbsem.standard" : "sem.capability.cbsem.experimental_labs",
        severity: "info",
        subject: null,
        message: bootstrapRequested
          ? "General SEM recursive CB-SEM full-refit percentile case bootstrapping passes the bounded exact-cell preview."
          : "General SEM recursive CB-SEM ML estimation passes the bounded exact-cell preview.",
        corrections: [],
      }],
      evidence,
      summary: standard
        ? "CB-SEM can compile this exact Standard Registry request."
        : "CB-SEM can compile this exact Registry-governed request.",
      explanation: bootstrapRequested
        ? "The point cell remains the model authority and the recursive-bootstrap cell owns one indexed full-ML refit ledger for eligible free parameters. Native preflight remains authoritative before execution."
        : "The connected v3 adapter binds the resident SemModelV4 parameter table to the proven ML kernel and canonical parameter, fit, and identification results.",
    });
  }

  return createSemCapabilityDecisionV1({
    status: "blocked",
    estimator_id: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
    capability_cells: capabilityCells,
    diagnostics,
    evidence,
    summary: "CB-SEM cannot calculate this exact General SEM request yet.",
    explanation: "The authored model remains unchanged. Resolve the listed predicate or Registry issue and run preflight again.",
  });
}
